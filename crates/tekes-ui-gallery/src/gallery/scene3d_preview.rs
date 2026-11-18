//! Gallery adapter only. Asset parsing, pose, playback and 3D rendering live in
//! tekes-ui-scene3d. Normal workbench chrome supplies theme and reduced motion.
use tekes_ui_platform_macos::{Event, KeyEventKind, MouseButton, MouseEventKind};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Point, Rect, Scene};
use tekes_ui_scene3d::{Asset3D, Camera3D, Error, Renderer3D, Scene3D};
use tekes_ui_tokens::proposal;

#[derive(Debug, Default)]
pub(super) struct Preview3D {
    loaded: Option<Result<Loaded, Error>>,
    drag: Option<(MouseButton, Point, bool)>,
    focused: bool,
    requested_side: Option<u32>,
    scale: f32,
    target_limit: u32,
}

#[derive(Debug)]
struct Loaded {
    view: Scene3D,
    gpu: Renderer3D,
    image: Option<ImageId>,
    registration: Option<tekes_ui_render::gpu::GpuImageRegistration>,
    renderer_identity: u64,
    dirty: bool,
    camera: Camera3D,
    playing: bool,
    spin: bool,
    target_side: u32,
    target_rebuilds: usize,
}
fn home_camera() -> Camera3D {
    let mut camera = Camera3D::default();
    camera.orbit(0.65, 0.16).unwrap();
    camera.zoom_by(0.85).unwrap();
    camera
}
pub(super) fn image_frame(bounds: Rect) -> Rect {
    let side = bounds
        .size
        .width
        .min((bounds.size.height - 48.).max(0.))
        .max(0.);
    Rect::new(
        bounds.min_x() + (bounds.size.width - side) * 0.5,
        bounds.min_y(),
        side,
        side,
    )
}
// Layout records the desired target; the next prepare updates GPU storage.
// The one-frame settle also runs while paused / Reduced Motion, then becomes idle.
fn physical_side(side: f32, scale: f32, limit: u32) -> Option<u32> {
    if !side.is_finite() || side <= 0. || !scale.is_finite() || scale <= 0. || limit == 0 {
        return None;
    }
    Some((side as f64 * scale as f64).round().clamp(1., limit as f64) as u32)
}
impl Preview3D {
    pub fn blur(&mut self) {
        self.drag = None;
        self.focused = false;
    }
    pub fn handle(&mut self, event: &Event, bounds: Rect, clip: Rect) -> bool {
        let frame = image_frame(bounds);
        let Some(Ok(state)) = &mut self.loaded else {
            return false;
        };
        let hit = |p: Point| frame.contains(p) && clip.contains(p);
        let result = match event {
            Event::Mouse(e) => {
                let p = Point::new(e.position.x as f32, e.position.y as f32);
                match e.kind {
                    MouseEventKind::Down { button, .. } => {
                        self.focused = hit(p);
                        self.drag = None;
                        if !self.focused
                            || !matches!(button, MouseButton::Left | MouseButton::Right)
                        {
                            return false;
                        }
                        self.drag = Some((button, p, e.modifiers.shift));
                        Ok(())
                    }
                    MouseEventKind::Dragged { button } => {
                        let Some((held, previous, rotate)) = self.drag else {
                            return false;
                        };
                        if button != held {
                            return false;
                        }
                        let dx = (p.x - previous.x) * 0.008;
                        let dy = (p.y - previous.y) * 0.008;
                        self.drag = Some((held, p, rotate));
                        if rotate || held == MouseButton::Right {
                            state.camera.rotate_model(dx, dy, 0.)
                        } else {
                            state.camera.orbit(-dx, dy)
                        }
                    }
                    MouseEventKind::Up { .. } => {
                        if self.drag.take().is_none() {
                            return false;
                        }
                        Ok(())
                    }
                    _ => return false,
                }
            }
            Event::Scroll(e) if hit(Point::new(e.position.x as f32, e.position.y as f32)) => {
                state.camera.zoom_by(
                    (e.delta.y as f32 * if e.precise { 0.01 } else { 0.08 })
                        .clamp(-1., 1.)
                        .exp(),
                )
            }
            Event::Key(e)
                if self.focused && e.kind == KeyEventKind::Down && !e.modifiers.command =>
            {
                let delta = match e.key_code {
                    123 => Some((-0.15, 0.)),
                    124 => Some((0.15, 0.)),
                    125 => Some((0., -0.15)),
                    126 => Some((0., 0.15)),
                    _ => None,
                };
                if let Some((x, y)) = delta {
                    if e.modifiers.shift {
                        state.camera.rotate_model(x, y, 0.)
                    } else {
                        state.camera.orbit(x, y)
                    }
                } else {
                    match e.characters.to_lowercase().as_str() {
                        "+" | "=" => state.camera.zoom_by(0.85),
                        "-" => state.camera.zoom_by(1. / 0.85),
                        "r" => {
                            state.camera = home_camera();
                            state.view.seek(0.)
                        }
                        " " => {
                            state.playing = !state.playing;
                            if state.playing {
                                state.view.play();
                            } else {
                                state.view.pause();
                            }
                            Ok(())
                        }
                        "a" => {
                            state.spin = !state.spin;
                            Ok(())
                        }
                        _ => {
                            self.focused = false;
                            return false;
                        }
                    }
                }
            }
            _ => return false,
        };
        state.dirty |= result.is_ok();
        result.is_ok()
    }
    pub fn reset(&mut self) {
        if let Some(Ok(state)) = &mut self.loaded {
            let _ = state.view.seek(0.);
            state.view.play();
            state.camera = home_camera();
            state.playing = true;
            state.spin = false;
            state.dirty = true;
        }
    }
    /// Allocate the current renderer's resources before ContentHost paints an
    /// image reference. Rendering the pixels is deferred until layout is known.
    pub fn prepare_resources(&mut self, renderer: &mut Renderer, reduced: bool, scale: f32) {
        self.scale = scale;
        // Bound the color + depth targets to 128 MiB at most, and respect the
        // adapter limit. This cap changes resolution only, never mesh geometry.
        self.target_limit = renderer
            .gpu_context()
            .device()
            .limits()
            .max_texture_dimension_2d
            .min(4096);
        let state = self.loaded.get_or_insert_with(|| {
            let asset = if let Some(path) = std::env::var_os("TEKESUI_SCENE3D_ASSET") {
                Asset3D::load(path)?
            } else {
                Asset3D::from_slice(include_bytes!("../../assets/rubiks-cube.glb"))?
            };
            let mut view = Scene3D::new(asset);
            if view.asset().clips().len() > 0 {
                view.select_clip(0)?;
                view.play();
            }
            view.reduced_motion = reduced;
            let gpu = Renderer3D::new(&renderer.gpu_context(), &view, (512, 512))?;
            Ok(Loaded {
                view,
                gpu,
                image: None,
                registration: None,
                renderer_identity: renderer.resource_identity(),
                dirty: true,
                camera: home_camera(),
                playing: true,
                spin: false,
                target_side: 512,
                target_rebuilds: 0,
            })
        });
        if let Ok(state) = state {
            state.view.reduced_motion = reduced;
            let result = (|| {
                if state.renderer_identity != renderer.resource_identity() {
                    state.gpu = Renderer3D::new(
                        &renderer.gpu_context(),
                        &state.view,
                        (state.target_side, state.target_side),
                    )?;
                    state.renderer_identity = renderer.resource_identity();
                    state.registration = None;
                    state.image = None;
                    state.dirty = true;
                }
                if state.registration.is_none() {
                    let registration = renderer
                        .register_owned_gpu_image(state.gpu.output())
                        .map_err(|e| Error(e.to_string()))?;
                    state.image = Some(registration.id());
                    state.registration = Some(registration);
                }
                Ok::<(), Error>(())
            })();
            if let Err(error) = result {
                self.loaded = Some(Err(error));
            }
        }
    }
    pub fn prepare(&mut self, renderer: &mut Renderer, reduced: bool, scale: f32) {
        self.prepare_resources(renderer, reduced, scale);
        if let Some(Ok(state)) = &mut self.loaded {
            let side = self.requested_side.unwrap_or(state.target_side);
            if side != state.target_side {
                let result = state.gpu.resize((side, side)).and_then(|()| {
                    if let Some(registration) = &state.registration {
                        renderer
                            .replace_owned_gpu_image(registration, state.gpu.output())
                            .map_err(|e| Error(e.to_string()))?;
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    self.loaded = Some(Err(error));
                    return;
                }
                state.target_side = side;
                state.target_rebuilds += 1;
                state.dirty = true;
            }
            if state.dirty {
                let result = state.gpu.render_camera(&state.view, &state.camera);
                if let Err(e) = result {
                    self.loaded = Some(Err(e));
                } else {
                    state.dirty = false;
                }
            }
        }
    }
    fn resize_pending(&self) -> bool {
        self.loaded.as_ref().is_some_and(|s| {
            s.as_ref().is_ok_and(|s| {
                self.requested_side
                    .is_some_and(|side| side != s.target_side)
            })
        })
    }
    pub fn needs_frame(&self, reduced: bool) -> bool {
        self.resize_pending()
            || (!reduced
                && self
                    .loaded
                    .as_ref()
                    .is_some_and(|s| s.as_ref().is_ok_and(|s| s.view.needs_frame() || s.spin)))
    }
    pub fn advance(&mut self, dt: f32, reduced: bool) -> bool {
        let resize_pending = self.resize_pending();
        if let Some(Ok(state)) = &mut self.loaded {
            state.view.reduced_motion = reduced;
            let mut changed = state.view.advance(dt).unwrap_or(false);
            if state.spin && !reduced && dt.is_finite() && dt > 0. {
                changed |= state
                    .camera
                    .rotate_model(dt.min(0.1) * 0.45, 0., 0.)
                    .is_ok();
            }
            state.dirty |= changed;
            changed || resize_pending
        } else {
            false
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn start_rotation(&mut self, animate_clip: bool) {
        if let Some(Ok(state)) = &mut self.loaded {
            state.spin = true;
            state.playing = animate_clip;
            if animate_clip {
                state.view.play();
            } else {
                state.view.pause();
            }
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn frame_work(&self) -> (u64, usize) {
        match &self.loaded {
            Some(Ok(state)) => (
                state.gpu.last_frame_pose_upload_bytes(),
                state.gpu.last_frame_deformed_meshes(),
            ),
            _ => (0, 0),
        }
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        match &self.loaded {
            Some(Ok(state)) => {
                serde_json::json!({"target_px":[state.target_side,state.target_side],"requested_side_px":self.requested_side,"backing_scale":self.scale,"target_limit_px":self.target_limit,"target_rebuilds":state.target_rebuilds,"pose_upload_bytes":state.gpu.last_frame_pose_upload_bytes(),"deformed_meshes":state.gpu.last_frame_deformed_meshes(),"vertex_buffer_allocations":state.gpu.vertex_buffer_allocations(),"geometry_resident_bytes":state.gpu.geometry_resident_bytes(),"model_rotation":state.camera.model_rotation(),"clip_time":state.view.time(),"spin":state.spin,"playing_clip":state.playing,"camera_zoom":state.camera.zoom()})
            }
            _ => serde_json::Value::Null,
        }
    }
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.drag = None;
        }
    }
    pub fn record_layout(&mut self, bounds: Rect) {
        self.requested_side = physical_side(
            image_frame(bounds).size.width,
            self.scale,
            self.target_limit,
        );
    }
    pub fn paint_content(
        &self,
        scene: &mut Scene,
        bounds: Rect,
        text: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        palette: tekes_ui_tokens::Palette,
    ) {
        scene.push_clip(bounds);
        let caption = match &self.loaded {
            Some(Ok(state)) => {
                if let Some(image) = state.image {
                    state.view.paint(scene, image_frame(bounds), image);
                }
                "Drag: orbit · Right / Shift-drag: rotate · Scroll: zoom. Click preview: arrows, +/−, R reset, Space play/pause, A auto-rotate.".to_owned()
            }
            Some(Err(error)) => format!("Scene3D: {error}"),
            None => "Preparing Scene3D…".to_owned(),
        };
        text.draw(
            scene,
            &caption,
            proposal::TextRole::UiCaption,
            Rect::new(bounds.min_x(), bounds.max_y() - 44., bounds.size.width, 44.),
            tekes_ui_controls::ButtonTextAlign::default(),
            palette
                .color("secondary-text")
                .expect("standard text token"),
        );
        scene.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene3d_target_sizes_follow_backing_pixels_and_limits() {
        assert_eq!(physical_side(401.25, 2., 4096), Some(803));
        assert_eq!(physical_side(401.25, 1.25, 4096), Some(502));
        assert_eq!(physical_side(20000., 3., 2048), Some(2048));
        for (side, scale) in [(0., 2.), (f32::NAN, 2.), (200., 0.), (200., f32::INFINITY)] {
            assert_eq!(physical_side(side, scale, 4096), None);
        }
    }
}
