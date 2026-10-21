//! Opt-in wgpu interop for GPU-rendered components. Ordinary Scene/ImageId APIs
//! remain backend-neutral. A context is issued by its UI Renderer, never assembled
//! from unrelated device/queue handles. Prefer Renderer::render_with_gpu to encode
//! component passes before UI sampling in one submission; standalone producers
//! may also submit before UI rendering.
use crate::RenderError;

/// Shared UI device/queue and registration identity. Cloning retains GPU handles;
/// it does not create a second device or queue. Follow the host render-thread policy.
#[derive(Clone, Debug)]
pub struct GpuContext {
    pub(crate) owner: u64,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}
impl GpuContext {
    /// Adopt an externally imported, single-sample 2D texture on this context's
    /// device. Platform import and cross-process synchronization belong to the
    /// caller. No pixels are copied or read back. wgpu validates device identity.
    pub fn import_image(&self, texture: wgpu::Texture) -> Result<GpuImage, RenderError> {
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.sample_count() != 1
            || texture.depth_or_array_layers() != 1
            || !texture
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            || !matches!(
                texture.format(),
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        {
            return Err(RenderError::InvalidArgument(
                "unsupported imported GPU image",
            ));
        }
        let render_view = texture.create_view(&Default::default());
        let sample_view = texture.create_view(&Default::default());
        Ok(GpuImage {
            owner: self.owner,
            texture,
            render_view,
            sample_view,
        })
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// A single-sample RGBA8 sRGB render target, sampled as encoded RGBA8 by
    /// the UI's gamma-space compositor. There is no staging buffer or COPY_SRC.
    /// Sampled values must be premultiplied in encoded sRGB space; clear
    /// transparent pixels to zero. OPAQUE/MASK producers satisfy this directly.
    /// Fractional-alpha producers must account for the sRGB write conversion:
    /// premultiplying linear color before that conversion is insufficient.
    pub fn create_image(&self, size: (u32, u32)) -> Result<GpuImage, RenderError> {
        let max = self.device.limits().max_texture_dimension_2d;
        if size.0 == 0 || size.1 == 0 || size.0 > max || size.1 > max {
            return Err(RenderError::InvalidArgument(
                "GPU image dimensions exceed device limits or are zero",
            ));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tekes shared GPU image"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });
        let render_view = texture.create_view(&Default::default());
        // UI stores sRGB encoded channel values in an unorm target. Sampling
        // with an sRGB view here would decode them and darken the final image.
        let sample_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        Ok(GpuImage {
            owner: self.owner,
            texture,
            render_view,
            sample_view,
        })
    }
}

/// GPU-only image storage. Renderer registration retains the texture even if
/// this handle is dropped. Replacement/unregistration releases only its own
/// references; already-submitted GPU work remains valid through wgpu ownership.
#[derive(Clone, Debug)]
pub struct GpuImage {
    pub(crate) owner: u64,
    pub(crate) texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    pub(crate) sample_view: wgpu::TextureView,
}
impl GpuImage {
    pub fn size(&self) -> (u32, u32) {
        (self.texture.width(), self.texture.height())
    }
    pub fn render_format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }
    pub fn render_view(&self) -> &wgpu::TextureView {
        &self.render_view
    }
}

/// Own a Renderer registration without borrowing the Renderer across component
/// callbacks. Drop queues unregistration on that Renderer; its next render or
/// explicit collection releases storage. Submitted GPU work retains its normal
/// wgpu resource lifetime. This handle is not cloneable and cannot be rebound.
#[derive(Debug)]
pub struct GpuImageRegistration {
    pub(crate) id: tekes_ui_scene::ImageId,
    pub(crate) owner: u64,
    pub(crate) retired: std::sync::Weak<std::sync::Mutex<Vec<tekes_ui_scene::ImageId>>>,
}
impl GpuImageRegistration {
    pub fn id(&self) -> tekes_ui_scene::ImageId {
        self.id
    }
}
impl Drop for GpuImageRegistration {
    fn drop(&mut self) {
        if let Some(retired) = self.retired.upgrade() {
            retired.lock().unwrap().push(self.id);
        }
    }
}
