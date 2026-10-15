//! `Renderer`：持有 wgpu 设备与（可选的）surface，按调用驱动绘制。

use crate::error::RenderError;
use crate::quad_pass::{COLOR_FORMAT, QuadPass};
use std::ffi::c_void;
use std::ptr::NonNull;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Rgba;

/// 同时在途的帧数，用于 `desired_maximum_frame_latency`。
///
/// 取 1：Metal 上对应 `maximumDrawableCount = 2`。
/// 我们每帧 GPU 工作量很小（几百个 quad），瓶颈不在 GPU 吞吐，而在输入到上屏的延迟，
/// 所以用最短的队列，鼠标拖动和打字的反馈少一帧。代价是 CPU 编码和 GPU 执行不能
/// 重叠，帧很重时可能掉帧；F1 以后若文字很多、实测 120Hz 吃紧，再改成 2。
const FRAME_LATENCY: u32 = 1;

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
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pass: QuadPass,
    surface: Option<SurfaceState>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("has_surface", &self.surface.is_some())
            .field("instance_capacity", &self.pass.instance_capacity())
            .finish_non_exhaustive()
    }
}

fn new_instance() -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::METAL;
    wgpu::Instance::new(desc)
}

fn request_device(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Device, wgpu::Queue), RenderError> {
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
        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("tekes device"),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::Device(e.to_string()))
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
        let (device, queue) = request_device(&instance, Some(&surface))?;
        let pass = QuadPass::new(&device);
        let mut r = Renderer {
            instance,
            device,
            queue,
            pass,
            surface: Some(SurfaceState {
                surface,
                layer,
                size_px: drawable_size,
                scale,
                configured: false,
            }),
        };
        r.configure_surface()?;
        Ok(r)
    }

    /// 不带 surface 的渲染器，只能用 [`render_to_rgba8`](Renderer::render_to_rgba8)。
    /// 用同一套管线和着色器，供测试和截图使用。
    pub fn new_headless() -> Result<Renderer, RenderError> {
        let instance = new_instance();
        let (device, queue) = request_device(&instance, None)?;
        let pass = QuadPass::new(&device);
        Ok(Renderer {
            instance,
            device,
            queue,
            pass,
            surface: None,
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
            desired_maximum_frame_latency: FRAME_LATENCY,
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
    /// 没有 surface 的渲染器返回 `RenderError::Surface`。
    pub fn render(&mut self, scene: &Scene, clear: Rgba) -> Result<(), RenderError> {
        let Some(s) = &self.surface else {
            return Err(RenderError::Surface("这个渲染器没有 surface".into()));
        };
        if s.size_px.0 == 0 || s.size_px.1 == 0 {
            return Ok(());
        }
        if !s.configured {
            self.configure_surface()?;
        }

        for _attempt in 0..3 {
            let s = self.surface.as_ref().expect("surface 存在");
            let (size_px, scale) = (s.size_px, s.scale);
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
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("tekes frame"),
                });
            self.pass.encode(
                &self.device,
                &self.queue,
                &mut encoder,
                &view,
                size_px,
                scale,
                scene,
                clear,
            );
            self.queue.submit([encoder.finish()]);
            self.queue.present(frame);
            if suboptimal {
                self.configure_surface()?;
            }
            return Ok(());
        }
        Err(RenderError::Surface("多次重试后仍取不到 drawable".into()))
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
    pub fn render_to_rgba8(
        &mut self,
        scene: &Scene,
        size_px: (u32, u32),
        scale: f32,
        clear: Rgba,
    ) -> Result<Vec<u8>, RenderError> {
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
        self.pass.encode(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            size_px,
            scale,
            scene,
            clear,
        );
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

    /// 实例缓冲当前能放多少个 quad。
    pub fn instance_capacity(&self) -> usize {
        self.pass.instance_capacity()
    }

    /// 实例缓冲到目前为止分配过几次（含创建时的第一次）。
    /// 容量够用时不会增加：这是"跨帧复用缓冲"的可观测证据。
    pub fn instance_buffer_allocations(&self) -> usize {
        self.pass.buffer_allocations()
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
