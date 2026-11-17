use anyhow::{Context, Result, ensure};
use std::sync::Arc;
use tekes_ui_app_protocol::{Frame, Request, VERSION};
use tekes_ui_render::{Renderer, shared_surface_macos as gpu};
use tekes_ui_scene::{GlyphAtlas, Scene};
use tekes_ui_tokens::Rgba;

/// The sole immutable GPU producer for the legacy loop and negotiated View actor.
/// Construct/run on its owning GPU thread, never the UI or a service dispatcher.
pub(crate) struct FrameRenderer {
    renderer: Renderer,
    registry: u64,
    pool: gpu::SharedSurfacePool,
}
impl FrameRenderer {
    pub(crate) fn new() -> Result<Self> {
        let renderer = Renderer::new_headless()?;
        let registry = gpu::registry(&renderer.gpu_context())?;
        Ok(Self {
            renderer,
            registry,
            pool: gpu::SharedSurfacePool::new(),
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn finish_pooled_frame(
        context: &tekes_ui_render::gpu::GpuContext,
        registry: u64,
        submission: wgpu::SubmissionIndex,
        surface: &objc2_io_surface::IOSurfaceRef,
        r: &Request,
        pixel_size: (u32, u32),
        state: crate::ModuleState,
        endpoint: &str,
    ) -> Result<Frame> {
        context.device().poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        let frame = Frame {
            version: VERSION,
            seq: r.seq,
            viewport: r.viewport,
            pid: std::process::id(),
            registry,
            width: r.width,
            height: r.height,
            pixel_width: pixel_size.0,
            pixel_height: pixel_size.1,
            port: 0,
            revision: state.revision,
            toolbar: state.toolbar,
        };
        // SAFETY: `IOSurfaceRef::create_mach_port()` returns one freshly retained Mach send right owned here.
        unsafe { crate::transport::send(endpoint, surface.create_mach_port())? };
        Ok(frame)
    }
    pub(crate) fn render(
        &mut self,
        r: &Request,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        state: crate::ModuleState,
        endpoint: &str,
    ) -> Result<Frame> {
        self.render_with_clear(r, scene, atlas, state, endpoint, Rgba::gray(1.))
    }
    pub(crate) fn render_with_clear(
        &mut self,
        r: &Request,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        state: crate::ModuleState,
        endpoint: &str,
        clear: Rgba,
    ) -> Result<Frame> {
        validate_view_request(r)?;
        let context = self.renderer.gpu_context();
        let pixel_size = (
            (r.width as f32 * r.scale).round() as u32,
            (r.height as f32 * r.scale).round() as u32,
        );
        let (texture, surface) = self.pool.acquire(&context, pixel_size)?;
        let submission = self
            .renderer
            .render_to_texture(scene, atlas, texture, r.scale, clear)?;
        Self::finish_pooled_frame(
            &context,
            self.registry,
            submission,
            surface,
            r,
            pixel_size,
            state,
            endpoint,
        )
    }

    /// Produce an explicit independent layer directly on GPU into a pooled
    /// `BGRA8Unorm` `IOSurface` target. The original logical layout is
    /// translated only by its raster phase; the parent applies its own mask.
    #[cfg(target_os = "macos")]
    pub(crate) fn render_layer(
        &mut self,
        r: &Request,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        state: crate::ModuleState,
        endpoint: &str,
        layout: &crate::layer_raster::LayerRasterLayout,
    ) -> Result<Frame> {
        validate_view_request(r)?;
        ensure!(
            layout.logical_size() == [r.width, r.height] && layout.scale() == r.scale,
            "layer raster does not match original View layout"
        );
        let mut raster_scene = Scene::new();
        raster_scene
            .try_append_fragment(
                scene,
                tekes_ui_scene::FragmentBudget::default(),
                tekes_ui_scene::FragmentTransform {
                    translation: layout.phase_logical(),
                    ..Default::default()
                },
            )
            .map_err(|error| anyhow::anyhow!("layer raster phase transform: {error:?}"))?;
        let context = self.renderer.gpu_context();
        let pixel_size = layout.pixel_size();
        let (texture, surface) = self.pool.acquire(&context, pixel_size)?;
        let submission =
            self.renderer
                .render_layer_to_texture(&raster_scene, atlas, texture, r.scale)?;
        Self::finish_pooled_frame(
            &context,
            self.registry,
            submission,
            surface,
            r,
            pixel_size,
            state,
            endpoint,
        )
    }
    /// Render directly into a pooled `BGRA8Unorm` `IOSurface` texture using the
    /// original renderer's finite parent-boundary proof, avoiding GPU->CPU pixel
    /// readback and CPU RGBA->BGRA swizzle/upload copies.
    #[cfg(target_os = "macos")]
    pub(crate) fn render_with_parent_boundary_proof(
        &mut self,
        r: &Request,
        scene: &Scene,
        atlas: &mut GlyphAtlas,
        state: crate::ModuleState,
        endpoint: &str,
        parent_shapes: Arc<[tekes_ui_scene::ClipShape]>,
    ) -> Result<Frame> {
        validate_view_request(r)?;
        let pixel_size = (
            (r.width as f32 * r.scale).round() as u32,
            (r.height as f32 * r.scale).round() as u32,
        );
        let context = self.renderer.gpu_context();
        let (texture, surface) = self.pool.acquire(&context, pixel_size)?;
        let verified = self
            .renderer
            .render_to_texture_with_parent_boundary_proof(
                scene,
                atlas,
                texture,
                r.scale,
                Rgba::new(0., 0., 0., 0.),
                parent_shapes.clone(),
                tekes_ui_render::BoundaryProofLimits::default(),
            )
            .with_context(|| {
                let parents = parent_shapes
                    .iter()
                    .map(|shape| {
                        format!(
                            "rect={:?} radii={:?} inner_witness={:?}",
                            shape.rect(),
                            shape.radii(),
                            shape.full_coverage_inner_rect_px(r.scale, pixel_size)
                        )
                    })
                    .collect::<Vec<_>>();
                let primitives = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .map(|(index, primitive)| {
                        use tekes_ui_scene::Primitive;
                        match primitive {
                            Primitive::Glyphs(run) => {
                                format!("{index}: Glyphs count={} color={:?}", run.glyphs.len(), run.color)
                            }
                            Primitive::PushShapeClip(key) => {
                                format!("{index}: {primitive:?} shape={:?}", scene.clip_shape(key))
                            }
                            Primitive::KeyedQuad { outer, .. } => {
                                format!("{index}: {primitive:?} shape={:?}", scene.clip_shape(outer))
                            }
                            _ => format!("{index}: {primitive:?}"),
                        }
                    })
                    .collect::<Vec<_>>();
                format!(
                    "Curve producer proof failed pid={} request={r:?} target={pixel_size:?} parents={parents:?} primitives={primitives:?}",
                    std::process::id()
                )
            })?;
        ensure!(
            verified.dimensions_px() == pixel_size,
            "verified frame dimensions changed"
        );
        Self::finish_pooled_frame(
            &context,
            self.registry,
            verified.into_submission_index(),
            surface,
            r,
            pixel_size,
            state,
            endpoint,
        )
    }
}
pub(crate) fn validate_view_request(r: &Request) -> Result<()> {
    ensure!(
        r.version == VERSION,
        "unsupported inner View request profile"
    );
    ensure!(
        (1..=4096).contains(&r.width) && (1..=4096).contains(&r.height),
        "invalid viewport"
    );
    ensure!(
        r.scale.is_finite() && (0.5..=4.).contains(&r.scale) && r.x.is_finite() && r.y.is_finite(),
        "invalid scale/point"
    );
    ensure!(
        (r.width as f32 * r.scale).round() <= 8192. && (r.height as f32 * r.scale).round() <= 8192.,
        "pixel viewport limit"
    );
    Ok(())
}
pub fn run(mut module: Box<dyn crate::PanelModule>, endpoint: &str) -> Result<()> {
    let mut renderer = FrameRenderer::new()?;
    let mut atlas = GlyphAtlas::default();
    let (mut input, mut output) = crate::pipes::worker_ipc()?;
    while let Some(bytes) = crate::pipes::read_packet(&mut input)? {
        if bytes == b"quit" {
            break;
        }
        let request: Request = serde_json::from_slice(&bytes)?;
        validate_view_request(&request)?;
        atlas.begin_frame();
        let mut scene = Scene::new();
        let state = module.update(&request, &mut scene, &mut atlas)?;
        let frame = renderer.render(&request, &scene, &mut atlas, state, endpoint)?;
        crate::pipes::write_packet(&mut output, &serde_json::to_vec(&frame)?)?;
    }
    Ok(())
}
