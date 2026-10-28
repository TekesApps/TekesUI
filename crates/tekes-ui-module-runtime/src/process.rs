use anyhow::{Result, ensure};
use std::io::BufRead;
use tekes_ui_app_protocol::{Frame, Request, VERSION};
use tekes_ui_render::{Renderer, shared_surface_macos as gpu};
use tekes_ui_scene::{GlyphAtlas, Scene};
use tekes_ui_tokens::Rgba;

/// The sole immutable GPU producer for the legacy loop and negotiated View actor.
/// Construct/run on its owning GPU thread, never the UI or a service dispatcher.
pub(crate) struct FrameRenderer {
    renderer: Renderer,
    registry: u64,
}
impl FrameRenderer {
    pub(crate) fn new() -> Result<Self> {
        let renderer = Renderer::new_headless()?;
        let registry = gpu::registry(&renderer.gpu_context())?;
        Ok(Self { renderer, registry })
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
        // New immutable surface per frame. Consumer GPU acknowledgements, not
        // another request, retire prior frames; storage is never overwritten.
        let (texture, surface) = gpu::create(&context, pixel_size)?;
        self.renderer
            .render_to_texture(scene, atlas, &texture, r.scale, clear)?;
        context.device().poll(wgpu::PollType::wait_indefinitely())?;
        let frame = Frame {
            version: VERSION,
            seq: r.seq,
            viewport: r.viewport,
            pid: std::process::id(),
            registry: self.registry,
            width: r.width,
            height: r.height,
            pixel_width: pixel_size.0,
            pixel_height: pixel_size.1,
            port: 0,
            revision: state.revision,
            toolbar: state.toolbar,
        };
        crate::transport::send(endpoint, surface.create_mach_port())?;
        Ok(frame)
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
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line == "quit" {
            break;
        }
        let request: Request = serde_json::from_str(&line)?;
        validate_view_request(&request)?;
        atlas.begin_frame();
        let mut scene = Scene::new();
        let state = module.update(&request, &mut scene, &mut atlas)?;
        let frame = renderer.render(&request, &scene, &mut atlas, state, endpoint)?;
        crate::pipes::write_packet(&mut stdout, &serde_json::to_vec(&frame)?)?;
    }
    Ok(())
}
