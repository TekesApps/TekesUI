//! `Renderer`：持有 wgpu 设备与（可选的）surface，按调用驱动绘制。

use crate::common::COLOR_FORMAT;
use crate::error::RenderError;
use crate::frame::{FrameCounts, Pipelines};
use crate::timing::{FrameTiming, SlotTicket, Timing};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tekes_ui_scene::{GlyphAtlas, ImageId, Scene};
use tekes_ui_tokens::Rgba;

/// Triple buffering leaves a drawable available while the compositor owns the
/// previous frame. One render per display callback still limits producer pace;
/// this does not add a free-running render-ahead loop.
const FRAME_LATENCY: u32 = 2;

// Diagnostic A/B only. Normal applications retain the measured FIFO default.
// wgpu Metal uses desired latency + 1 drawables; never disable display sync.
fn frame_latency() -> u32 {
    if std::env::var_os("TEKES_PERF").is_some() {
        match std::env::var("TEKES_PERF_FRAME_LATENCY").as_deref() {
            Ok("1") => 1,
            Ok("2") => 2,
            _ => FRAME_LATENCY,
        }
    } else {
        FRAME_LATENCY
    }
}

/// 渲染器编号。atlas 靠它区分是哪个渲染器取走了脏区（见 `GlyphAtlas::take_dirty`）。
static NEXT_RENDERER_ID: AtomicU64 = AtomicU64::new(1);

struct SurfaceState {
    surface: wgpu::Surface<'static>,
    /// 建 surface 用的 CAMetalLayer 指针。`new_for_metal_layer` 的调用方保证它在
    /// Renderer 存活期间有效，surface 丢失时用它重建。
    layer: NonNull<c_void>,
    size_px: (u32, u32),
    scale: f32,
    /// surface 当前是否已按 `size_px` 配置。尺寸为 0 时不能配置。
    configured: bool,
}

/// 渲染器。
///
/// 只在 [`render`](Renderer::render) 或 [`render_to_rgba8`](Renderer::render_to_rgba8)
/// 被调用时工作：没有内部循环、定时器或线程，空闲时 CPU 占用为 0。
///
/// 线程：创建、`resize`、`render` 都应在主线程调用。wgpu 的 Metal 后端在取 drawable
/// 时会读窗口的 occlusionState，AppKit 对象只能在主线程访问；
/// 并且 `presentsWithTransaction` 的同步上屏也要求在主线程。
pub struct Renderer {
    id: u64,
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    gpu: Pipelines,
    retired_images: std::sync::Arc<std::sync::Mutex<Vec<ImageId>>>,
    /// 最近一帧各类图元的实例数，测试用。
    last_frame: FrameCounts,
    surface: Option<SurfaceState>,
    /// 设备带 `TIMESTAMP_QUERY`，可以量 GPU 时间。
    gpu_timestamps: bool,
    /// 逐帧计时，`None` 表示关着（默认）。
    timing: Option<Timing>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("has_surface", &self.surface.is_some())
            .field("instance_capacity", &self.gpu.quad.instances.capacity())
            .finish_non_exhaustive()
    }
}

fn new_instance() -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::METAL;
    wgpu::Instance::new(desc)
}

/// 返回设备、队列，以及设备是否带 `TIMESTAMP_QUERY`（适配器支持就请求，供 `set_frame_timing` 使用）。
fn request_device(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Device, wgpu::Queue, bool), RenderError> {
    pollster::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| RenderError::NoAdapter)?;
        let timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let required_features = if timestamps {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("tekes device"),
                required_features,
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::Device(e.to_string()))?;
        Ok((device, queue, timestamps))
    })
}

fn check_size(size_px: (u32, u32), scale: f32) -> Result<(), RenderError> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(RenderError::InvalidArgument("scale 必须是正的有限数"));
    }
    if size_px.0 == 0 || size_px.1 == 0 {
        return Err(RenderError::InvalidArgument("尺寸不能为 0"));
    }
    Ok(())
}

impl Renderer {
    /// 在一个 `CAMetalLayer` 上建 surface 并初始化设备。阻塞，通常几十毫秒。
    ///
    /// `drawable_size` 是 drawable 的像素尺寸，`scale` 是 layer 的 `contentsScale`
    /// （逻辑点到像素的比例）。尺寸为 0 是允许的（窗口刚创建、还没布局），此时
    /// surface 暂不配置，`render` 什么也不做，直到 `resize` 给出非零尺寸。
    ///
    /// # Safety
    ///
    /// - `layer` 必须指向一个有效的 `CAMetalLayer`，并且在返回的 `Renderer` 被丢弃前
    ///   一直有效（调用方持有它）。
    /// - 在主线程调用。
    ///
    /// # 对 layer 的修改
    ///
    /// 每次配置 surface（创建、`resize`、surface 失效后重配），本 crate 会设置 layer 的
    /// `device`、`pixelFormat`（`Bgra8Unorm`）、`framebufferOnly`、`opaque`、
    /// `drawableSize`、`maximumDrawableCount`、`displaySyncEnabled`、
    /// `allowsNextDrawableTimeout`（wgpu 做的），并把 `colorspace` 设为 sRGB
    /// （本 crate 在 wgpu 之后做的，见 crate 文档"颜色管线"）。
    /// 调用方负责 layer 的 `bounds`、`contentsScale` 和 `presentsWithTransaction`，
    /// 不要自己设 `drawableSize` 和 `colorspace`：前者由 `resize` 管，后者会被重置。
    #[cfg(target_os = "macos")]
    pub unsafe fn new_for_metal_layer(
        layer: NonNull<c_void>,
        drawable_size: (u32, u32),
        scale: f32,
    ) -> Result<Renderer, RenderError> {
        if !(scale.is_finite() && scale > 0.0) {
            return Err(RenderError::InvalidArgument("scale 必须是正的有限数"));
        }
        let instance = new_instance();
        // SAFETY: 由调用方保证 layer 有效。
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                layer.as_ptr(),
            ))
        }
        .map_err(|e| RenderError::Surface(e.to_string()))?;
        let (device, queue, gpu_timestamps) = request_device(&instance, Some(&surface))?;
        let id = NEXT_RENDERER_ID.fetch_add(1, Ordering::Relaxed);
        let gpu = Pipelines::new(&device, id);
        let mut r = Renderer {
            id,
            instance,
            device,
            queue,
            gpu,
            retired_images: Default::default(),
            last_frame: FrameCounts::default(),
            surface: Some(SurfaceState {
                surface,
                layer,
                size_px: drawable_size,
                scale,
                configured: false,
            }),
            gpu_timestamps,
            timing: None,
        };
        r.configure_surface()?;
        Ok(r)
    }

    /// 不带 surface 的渲染器，只能用 [`render_to_rgba8`](Renderer::render_to_rgba8)。
    /// 用同一套管线和着色器，供测试和截图使用。
    pub fn new_headless() -> Result<Renderer, RenderError> {
        let instance = new_instance();
        let (device, queue, gpu_timestamps) = request_device(&instance, None)?;
        let id = NEXT_RENDERER_ID.fetch_add(1, Ordering::Relaxed);
        let gpu = Pipelines::new(&device, id);
        Ok(Renderer {
            id,
            instance,
            device,
            queue,
            gpu,
            retired_images: Default::default(),
            last_frame: FrameCounts::default(),
            surface: None,
            gpu_timestamps,
            timing: None,
        })
    }

    /// drawable 的像素尺寸或 scale 变了（窗口缩放、换屏）。会重新配置 surface。
    ///
    /// 尺寸为 0（窗口最小化等）时暂停配置，`render` 变成空操作，直到再次给出非零尺寸。
    /// 没有 surface 的渲染器上调用无效果。
    pub fn resize(&mut self, drawable_size: (u32, u32), scale: f32) {
        let Some(s) = &mut self.surface else {
            return;
        };
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        if s.size_px == drawable_size && s.scale == scale && s.configured {
            return;
        }
        s.size_px = drawable_size;
        s.scale = scale;
        s.configured = false;
        // 配置失败不在这里报告：下一次 render 会重试并返回错误。
        let _ = self.configure_surface();
    }

    fn configure_surface(&mut self) -> Result<(), RenderError> {
        let Some(s) = &mut self.surface else {
            return Ok(());
        };
        if s.size_px.0 == 0 || s.size_px.1 == 0 {
            s.configured = false;
            return Ok(());
        }
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: COLOR_FORMAT,
            // sRGB：Metal 后端对它的处理是把 layer 的 colorspace 重置为 nil，
            // 下面我们自己再设成显式的 sRGB。
            color_space: wgpu::SurfaceColorSpace::Srgb,
            width: s.size_px.0,
            height: s.size_px.1,
            // Fifo：与显示器刷新同步，不撕裂。Metal 上对应 displaySyncEnabled = true。
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: frame_latency(),
            // Opaque：窗口背景由我们自己画，不需要和窗口后面的内容合成。
            // Metal 上对应 CAMetalLayer.opaque = true，合成器可以跳过混合。
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        s.surface.configure(&self.device, &config);
        // wgpu 的配置会把 colorspace 重置为 nil，必须在它之后设置。
        // SAFETY: layer 的有效性由 new_for_metal_layer 的调用方保证。
        #[cfg(target_os = "macos")]
        unsafe {
            set_srgb_colorspace(s.layer);
        }
        s.configured = true;
        Ok(())
    }

    /// 画一帧：取 drawable、清屏、画 scene、上屏。
    ///
    /// `clear` 是整个窗口的背景色。drawable 过期（`Outdated`）会重新配置后重试，
    /// surface 丢失（`Lost`）会重建 surface 后重试。
    /// 窗口被遮挡、或尺寸为 0 时跳过这一帧并返回 `Ok`；窗口重新可见时平台层应触发一次重绘。
    ///
    /// `atlas` 是 scene 里字形所引用的 atlas：画之前先把它的脏区上传到 GPU。
    /// scene 必须是在该 atlas 当前这一帧（最近一次 `begin_frame` 之后）编出来的。
    ///
    /// 没有 surface 的渲染器返回 `RenderError::Surface`。
    pub fn render(
        &mut self,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        clear: Rgba,
    ) -> Result<(), RenderError> {
        self.render_with_gpu(scene, atlas, clear, |_| Ok(()))
    }
    /// Encode GPU component passes into the acquired UI frame before composition.
    /// The closure runs once after successful acquisition, never on timeout or
    /// occlusion; it must use this renderer's GpuContext and must not submit.
    /// All component and UI commands are submitted together in queue order.
    /// CPU encode timing includes the closure; GPU timestamps cover the UI pass.
    pub fn render_with_gpu(
        &mut self,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        clear: Rgba,
        before_ui: impl FnOnce(&mut wgpu::CommandEncoder) -> Result<(), RenderError>,
    ) -> Result<(), RenderError> {
        self.collect_retired_images();
        let Some(s) = &self.surface else {
            return Err(RenderError::Surface("这个渲染器没有 surface".into()));
        };
        if s.size_px.0 == 0 || s.size_px.1 == 0 {
            return Ok(());
        }
        if !s.configured {
            self.configure_surface()?;
        }

        // 计时关着时 `t_start` 是 `None`，整条路径只多这几次 `Option` 判断。
        let t_start = self.timing.is_some().then(Instant::now);
        for _attempt in 0..3 {
            let s = self.surface.as_ref().expect("surface 存在");
            let (size_px, scale) = (s.size_px, s.scale);
            let t_acquire = t_start.map(|_| Instant::now());
            let (frame, suboptimal) = match s.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) => (t, false),
                wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, true),
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.configure_surface()?;
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.recreate_surface()?;
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(RenderError::Surface("取 drawable 时校验失败".into()));
                }
            };
            let t_acquired = t_acquire.map(|_| Instant::now());
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("tekes frame"),
                });
            before_ui(&mut encoder)?;
            let ticket =
                self.encode_scene(&mut encoder, &view, size_px, scale, scene, atlas, clear);
            let t_encoded = t_start.map(|_| Instant::now());
            self.queue.submit([encoder.finish()]);
            self.queue.present(frame);
            if let (Some(t), Some(t0), Some(ta), Some(tb), Some(tc)) =
                (&mut self.timing, t_start, t_acquire, t_acquired, t_encoded)
            {
                let end = Instant::now();
                t.finish_frame(
                    &self.queue,
                    FrameTiming {
                        frame: 0,
                        acquire: tb - ta,
                        encode: tc - tb,
                        submit: end - tc,
                        total: end - t0,
                        gpu: None,
                    },
                    ticket,
                );
                // 非阻塞地推进回调，把已完成的 GPU 结果收进来。
                let _ = self.device.poll(wgpu::PollType::Poll);
                t.collect(&self.device, &self.queue);
            }
            if suboptimal {
                self.configure_surface()?;
            }
            return Ok(());
        }
        Err(RenderError::Surface("多次重试后仍取不到 drawable".into()))
    }

    /// 把 scene 编进 `encoder`。计时开着且有空槽时，给 render pass 挂上时间戳并解析到回读缓冲，
    /// 返回占用的槽；否则返回 `None`。上屏和离屏两条路径共用。
    #[allow(clippy::too_many_arguments)]
    fn encode_scene(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size_px: (u32, u32),
        scale: f32,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        clear: Rgba,
    ) -> Option<SlotTicket> {
        if let Some(t) = &mut self.timing
            && let Some((ticket, writes)) = t.begin_gpu()
        {
            self.last_frame = self.gpu.encode(
                &self.device,
                &self.queue,
                encoder,
                view,
                size_px,
                scale,
                scene,
                atlas,
                clear,
                Some(writes),
            );
            return Some(ticket);
        }
        self.last_frame = self.gpu.encode(
            &self.device,
            &self.queue,
            encoder,
            view,
            size_px,
            scale,
            scene,
            atlas,
            clear,
            None,
        );
        None
    }

    fn recreate_surface(&mut self) -> Result<(), RenderError> {
        #[cfg(target_os = "macos")]
        {
            let Some(s) = &mut self.surface else {
                return Ok(());
            };
            // SAFETY: layer 的有效性由 new_for_metal_layer 的调用方保证。
            let surface = unsafe {
                self.instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                        s.layer.as_ptr(),
                    ))
            }
            .map_err(|e| RenderError::Surface(e.to_string()))?;
            s.surface = surface;
            s.configured = false;
            self.configure_surface()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(RenderError::Surface("当前平台不支持重建 surface".into()))
        }
    }

    /// 离屏渲染并读回像素，返回 `size_px.0 * size_px.1 * 4` 字节，RGBA 顺序，
    /// 行紧密排列，左上角在前，**sRGB 编码、预乘 alpha**（与上屏的像素一致）。
    ///
    /// `size_px` 是像素尺寸，`scale` 是逻辑点到像素的比例：scene 里 10pt 的矩形在
    /// scale 2 下占 20 个像素。用的管线和着色器与 [`render`](Renderer::render) 完全相同。
    /// Render into a caller-owned BGRA8 target on this renderer's device,
    /// without CPU readback. Caller owns external synchronization and lifetime.
    pub fn render_to_texture(
        &mut self,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        texture: &wgpu::Texture,
        scale: f32,
        clear: Rgba,
    ) -> Result<wgpu::SubmissionIndex, RenderError> {
        self.collect_retired_images();
        let size = (texture.width(), texture.height());
        check_size(size, scale)?;
        if texture.format() != COLOR_FORMAT
            || texture.sample_count() != 1
            || texture.dimension() != wgpu::TextureDimension::D2
            || texture.depth_or_array_layers() != 1
            || !texture
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(RenderError::InvalidArgument(
                "invalid external render target",
            ));
        }
        // Timing is disabled for this external-target API until its ownership
        // and completion reporting are integrated with the external producer.
        if self.timing.is_some() {
            return Err(RenderError::InvalidArgument(
                "external target timing is not supported",
            ));
        }
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode_scene(&mut encoder, &view, size, scale, scene, atlas, clear);
        Ok(self.queue.submit([encoder.finish()]))
    }

    pub fn render_to_rgba8(
        &mut self,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        size_px: (u32, u32),
        scale: f32,
        clear: Rgba,
    ) -> Result<Vec<u8>, RenderError> {
        self.render_to_rgba8_with_gpu(scene, atlas, size_px, scale, clear, |_| Ok(()))
    }
    /// Offscreen counterpart of render_with_gpu, for snapshots and GPU image
    /// regression tests. Only the final composed image is read back.
    #[allow(clippy::too_many_arguments)]
    pub fn render_to_rgba8_with_gpu(
        &mut self,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        size_px: (u32, u32),
        scale: f32,
        clear: Rgba,
        before_ui: impl FnOnce(&mut wgpu::CommandEncoder) -> Result<(), RenderError>,
    ) -> Result<Vec<u8>, RenderError> {
        self.collect_retired_images();
        check_size(size_px, scale)?;
        let (w, h) = size_px;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tekes offscreen"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 行字节数要对齐到 COPY_BYTES_PER_ROW_ALIGNMENT（256）。
        let unpadded = w * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tekes readback"),
            size: padded as u64 * h as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tekes offscreen"),
            });
        let t_start = self.timing.is_some().then(Instant::now);
        before_ui(&mut encoder)?;
        let ticket = self.encode_scene(&mut encoder, &view, size_px, scale, scene, atlas, clear);
        let t_encoded = t_start.map(|_| Instant::now());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        if let (Some(t), Some(t0), Some(tc)) = (&mut self.timing, t_start, t_encoded) {
            // 离屏没有 drawable，acquire 为 0；total 到提交为止（不含读回等待）。
            let end = Instant::now();
            t.finish_frame(
                &self.queue,
                FrameTiming {
                    frame: 0,
                    acquire: Duration::ZERO,
                    encode: tc - t0,
                    submit: end - tc,
                    total: end - t0,
                    gpu: None,
                },
                ticket,
            );
        }

        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| RenderError::Readback(e.to_string()))?;
        rx.recv()
            .map_err(|e| RenderError::Readback(e.to_string()))?
            .map_err(|e| RenderError::Readback(e.to_string()))?;

        let mapped = readback
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Readback(e.to_string()))?;
        let mut out = Vec::with_capacity((unpadded * h) as usize);
        for row in 0..h as usize {
            let start = row * padded as usize;
            let line = &mapped[start..start + unpadded as usize];
            // 纹理是 BGRA，对外交付 RGBA。
            for px in line.chunks_exact(4) {
                out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
        }
        drop(mapped);
        readback.unmap();
        Ok(out)
    }

    /// 设备是否支持 GPU 时间戳（Metal 的 stage boundary 计数器采样）。
    pub fn gpu_timing_supported(&self) -> bool {
        self.gpu_timestamps
    }

    /// 打开或关闭逐帧计时，见 [`FrameTiming`]。关闭时丢弃尚未取走的结果。
    ///
    /// 打开后每次 [`render`](Renderer::render) 量取 drawable、编码、提交三段 CPU 时间，
    /// 设备支持时再量 render pass 的 GPU 时间；结果用
    /// [`drain_frame_timings`](Renderer::drain_frame_timings) 取走。关着时每帧没有额外开销。
    /// 上屏（`render`）和离屏（`render_to_rgba8`）两条路径都计时；离屏没有 drawable，`acquire` 为 0。
    pub fn set_frame_timing(&mut self, enabled: bool) {
        match (enabled, self.timing.is_some()) {
            (true, false) => {
                self.timing = Some(Timing::new(&self.device, &self.queue, self.gpu_timestamps));
            }
            (false, true) => self.timing = None,
            _ => {}
        }
    }

    /// Record CPU stages without GPU counter resolves/readbacks. This allows
    /// presentation pacing to be measured without the extra profiling submits.
    /// Resets previous timing samples, just like starting a new measurement.
    pub fn set_frame_timing_cpu_only(&mut self) {
        self.timing = Some(Timing::new(&self.device, &self.queue, false));
    }

    /// 最近一次真正画出（没有因遮挡等跳过）的帧的编号，对应 [`FrameTiming::frame`]。
    /// 计时关着或还没画过时为 `None`。宿主靠它把自己量的数字和渲染器的结果配对。
    pub fn last_timed_frame(&self) -> Option<u64> {
        self.timing.as_ref().and_then(Timing::last_frame)
    }

    /// 取走已经完成的帧计时（追加到 `out`，按帧顺序）。GPU 结果通常晚一两帧到；
    /// `wait` 为 true 时先阻塞到所有已提交的工作完成，用于收尾。
    pub fn drain_frame_timings(&mut self, out: &mut Vec<FrameTiming>, wait: bool) {
        let Some(t) = &mut self.timing else { return };
        // 等待时最多循环几轮：第一轮等帧做完并提交解析，第二轮等映射完成。
        for _ in 0..if wait { 4 } else { 1 } {
            let _ = self.device.poll(if wait {
                wgpu::PollType::wait_indefinitely()
            } else {
                wgpu::PollType::Poll
            });
            t.collect(&self.device, &self.queue);
            if t.pending() == 0 {
                break;
            }
        }
        t.take_done(out);
    }

    /// 还有多少帧在等 GPU 结果（计时关着时为 0）。
    pub fn pending_frame_timings(&self) -> usize {
        self.timing.as_ref().map_or(0, Timing::pending)
    }

    /// 实例缓冲当前能放多少个 quad。
    pub fn instance_capacity(&self) -> usize {
        self.gpu.quad.instances.capacity()
    }

    /// 实例缓冲到目前为止分配过几次（含创建时的第一次）。
    /// 容量够用时不会增加：这是"跨帧复用缓冲"的可观测证据。
    pub fn instance_buffer_allocations(&self) -> usize {
        self.gpu.quad.instances.allocations()
    }

    /// 登记一张图像，供 [`tekes_ui_scene::Primitive::Image`] 引用。
    ///
    /// `rgba8_premul` 是 RGBA 顺序、**预乘 alpha**、行紧密排列的 8 位像素，长度 `width * height * 4`，
    /// 数值原样当作 sRGB 编码值（不做颜色转换）。图像画进矩形时线性采样、缩放到矩形大小。
    /// 登记后图像常驻 GPU，直到 [`unregister_image`](Renderer::unregister_image)。
    pub fn register_image(
        &mut self,
        rgba8_premul: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageId, RenderError> {
        self.gpu
            .register_image(&self.device, &self.queue, rgba8_premul, width, height)
    }

    /// Update pixels in an existing image of the same dimensions, preserving its
    /// ID, texture and bind group. Bytes are premultiplied RGBA8, as at registration.
    pub fn update_image(&self, id: ImageId, rgba8_premul: &[u8]) -> Result<(), RenderError> {
        self.gpu.image.update(&self.queue, id, rgba8_premul)
    }

    /// 注销图像并释放它的 GPU 内存。返回 `false` 表示这个 id 没有登记（或已注销）。
    /// 之后 scene 里仍引用该 id 的 `Image` 图元被跳过，不画。
    pub fn unregister_image(&mut self, id: ImageId) -> bool {
        self.gpu.image.unregister(id)
    }

    /// 当前登记着多少张图像。
    pub fn image_count(&self) -> usize {
        self.gpu.image.len()
    }

    /// Opt into wgpu interop on this renderer's device and submission queue.
    pub fn gpu_context(&self) -> crate::gpu::GpuContext {
        crate::gpu::GpuContext {
            owner: self.id,
            device: self.device.clone(),
            queue: self.queue.clone(),
        }
    }

    /// Register GPU storage directly. No pixel copy, readback, mapping or wait.
    /// Only images created from this renderer's context are accepted.
    pub fn register_gpu_image(
        &mut self,
        image: &crate::gpu::GpuImage,
    ) -> Result<ImageId, RenderError> {
        self.check_gpu_image(image)?;
        Ok(self.gpu.image.register_gpu(&self.device, image))
    }

    /// Register storage with a component-owned lifetime. No borrowed Renderer
    /// is needed on component unmount/Drop; render entry points collect it.
    pub fn register_owned_gpu_image(
        &mut self,
        image: &crate::gpu::GpuImage,
    ) -> Result<crate::gpu::GpuImageRegistration, RenderError> {
        self.collect_retired_images();
        let id = self.register_gpu_image(image)?;
        Ok(crate::gpu::GpuImageRegistration {
            id,
            owner: self.id,
            retired: std::sync::Arc::downgrade(&self.retired_images),
        })
    }
    pub fn replace_owned_gpu_image(
        &mut self,
        registration: &crate::gpu::GpuImageRegistration,
        image: &crate::gpu::GpuImage,
    ) -> Result<(), RenderError> {
        if registration.owner != self.id {
            return Err(RenderError::InvalidArgument(
                "GPU registration belongs to a different renderer",
            ));
        }
        self.replace_gpu_image(registration.id, image)
    }
    /// Collect component registrations retired since the last render. Also safe
    /// to call when a host removes content while it does not submit a frame.
    pub fn collect_retired_images(&mut self) -> usize {
        let retired = std::mem::take(&mut *self.retired_images.lock().unwrap());
        retired
            .into_iter()
            .filter(|id| self.gpu.image.unregister(*id))
            .count()
    }

    /// Replace a GPU image's backing storage (e.g. after resize), retaining ImageId.
    /// Ordinary CPU-uploaded images cannot be replaced through this method.
    pub fn replace_gpu_image(
        &mut self,
        id: ImageId,
        image: &crate::gpu::GpuImage,
    ) -> Result<(), RenderError> {
        self.check_gpu_image(image)?;
        self.gpu.image.replace_gpu(&self.device, id, image)
    }

    fn check_gpu_image(&self, image: &crate::gpu::GpuImage) -> Result<(), RenderError> {
        if image.owner != self.id {
            return Err(RenderError::InvalidArgument(
                "GPU image belongs to a different renderer",
            ));
        }
        Ok(())
    }

    /// atlas 的 GPU 数组纹理到目前为止建过几次（首次、换 atlas、页数超过层数时重建）。
    pub fn atlas_texture_creations(&self) -> usize {
        self.gpu.glyph.texture_creations()
    }

    /// 最近一帧的绘制统计：（quad 数，字形数，图像数，draw 数）。
    /// 用来验证批处理：交错的图元种类各自成批，同类相邻的并成一次 draw。
    pub fn last_frame_counts(&self) -> (usize, usize, usize, usize) {
        let c = self.last_frame;
        (c.quads, c.glyphs, c.images, c.draws)
    }
}

/// 把 layer 的 colorspace 设为 sRGB。
///
/// # Safety
///
/// `layer` 必须指向有效的 `CAMetalLayer`。
#[cfg(target_os = "macos")]
unsafe fn set_srgb_colorspace(layer: NonNull<c_void>) {
    use objc2_core_graphics::{CGColorSpace, kCGColorSpaceSRGB};
    use objc2_quartz_core::CAMetalLayer;
    // SAFETY: 由调用方保证。
    let layer: &CAMetalLayer = unsafe { layer.cast::<CAMetalLayer>().as_ref() };
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    layer.setColorspace(space.as_deref());
}
