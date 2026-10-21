//! Gallery adapter only. Asset parsing, pose, playback and 3D rendering live in
//! tekes-ui-scene3d. Normal workbench chrome supplies theme and reduced motion.
use super::text::TextEngine;
use tekes_ui_platform_macos::{Event, KeyEventKind, MouseButton, MouseEventKind};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Point, Rect, Scene};
use tekes_ui_scene3d::{Asset3D, Camera3D, Error, Renderer3D, Scene3D};
use tekes_ui_tokens::{ColorRole, Theme, proposal};

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
fn image_frame(bounds: Rect) -> Rect {
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
    pub fn prepare(&mut self, renderer: &mut Renderer, reduced: bool, scale: f32) {
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
                Asset3D::from_slice(include_bytes!(
                    "../../../tekes-ui-3d/assets/rubiks-cube.glb"
                ))?
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
            let side = self.requested_side.unwrap_or(state.target_side);
            if side != state.target_side {
                let result = state.gpu.resize((side, side)).and_then(|()| {
                    if let Some(image) = state.image {
                        renderer
                            .replace_gpu_image(image, state.gpu.output())
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
                let result = state
                    .gpu
                    .render_camera(&state.view, &state.camera)
                    .and_then(|()| {
                        if state.image.is_none() {
                            state.image = Some(
                                renderer
                                    .register_gpu_image(state.gpu.output())
                                    .map_err(|e| Error(e.to_string()))?,
                            );
                        }
                        Ok(())
                    });
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
    pub(super) fn start_rotation(&mut self, animate_clip: bool) {
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
    pub(super) fn frame_work(&self) -> (u64, usize) {
        match &self.loaded {
            Some(Ok(state)) => (
                state.gpu.last_frame_pose_upload_bytes(),
                state.gpu.last_frame_deformed_meshes(),
            ),
            _ => (0, 0),
        }
    }
    pub(super) fn diagnostics(&self) -> serde_json::Value {
        match &self.loaded {
            Some(Ok(state)) => {
                serde_json::json!({"target_px":[state.target_side,state.target_side],"requested_side_px":self.requested_side,"backing_scale":self.scale,"target_limit_px":self.target_limit,"target_rebuilds":state.target_rebuilds,"pose_upload_bytes":state.gpu.last_frame_pose_upload_bytes(),"deformed_meshes":state.gpu.last_frame_deformed_meshes(),"vertex_buffer_allocations":state.gpu.vertex_buffer_allocations(),"geometry_resident_bytes":state.gpu.geometry_resident_bytes(),"model_rotation":state.camera.model_rotation(),"clip_time":state.view.time(),"spin":state.spin,"playing_clip":state.playing})
            }
            _ => serde_json::Value::Null,
        }
    }
    pub fn paint(&mut self, scene: &mut Scene, bounds: Rect, text: &mut TextEngine, theme: Theme) {
        let frame = image_frame(bounds);
        self.requested_side = physical_side(frame.size.width, self.scale, self.target_limit);
        scene.push_clip(bounds);
        match &self.loaded {
            Some(Ok(state)) => {
                if let Some(image) = state.image {
                    let frame = image_frame(bounds);
                    state.view.paint(scene, frame, image);
                    text.proposal_wrapped_in(scene,theme,Rect::new(bounds.min_x(),bounds.max_y()-44.,bounds.size.width,44.),"Drag: orbit · Right / Shift-drag: rotate · Scroll: zoom. Click preview: arrows, +/−, R reset, Space play/pause, A auto-rotate.",proposal::TextRole::UiCaption,ColorRole::SecondaryText.resolve(theme));
                }
            }
            Some(Err(e)) => {
                text.proposal_wrapped_in(
                    scene,
                    theme,
                    bounds,
                    &format!("Scene3D: {e}"),
                    proposal::TextRole::UiBody,
                    ColorRole::PrimaryText.resolve(theme),
                );
            }
            None => {}
        }
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
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn scene3d_adapts_resolution_while_paused_without_geometry_reuploads() {
        let mut gallery = crate::gallery::Gallery::new();
        let mut ui = Renderer::new_headless().unwrap();
        gallery.open_route("component-scene3d/playground");
        gallery.set_reduced_motion(true);
        let mut image = None;
        let mut count = 0;
        let mut previous_side = 512;
        let mut rebuilds = 0;
        for (width, height, scale) in [
            (1280., 900., 1.),
            (1280., 900., 2.),
            (720., 640., 2.),
            (1280., 900., 1.25),
            (1280., 900., 1.25),
            (1280., 900., 1.),
        ] {
            gallery.set_view_size(width, height);
            // A layout/backing change schedules one settle frame even when all
            // animation is disabled. It then returns to idle.
            for _ in 0..2 {
                gallery.prepare(&mut ui, scale);
                gallery.build_scene(&mut Scene::new());
                if gallery.scene3d.resize_pending() {
                    assert!(gallery.scene3d.needs_frame(true));
                    assert!(gallery.animate(1. / 60.));
                }
            }
            let crate::gallery::Page::Workbench(i) = gallery.page else {
                panic!()
            };
            let state = gallery.scene3d.loaded.as_ref().unwrap().as_ref().unwrap();
            let expected = physical_side(
                image_frame(gallery.workbench.preview_rect(i)).size.width,
                scale,
                4096,
            )
            .unwrap();
            assert_eq!(state.target_side, expected);
            if expected != previous_side {
                rebuilds += 1;
                previous_side = expected;
            }
            assert_eq!(state.target_rebuilds, rebuilds);
            assert!(!gallery.scene3d.needs_frame(true));
            assert!(!gallery.scene3d.resize_pending());
            assert_eq!(state.gpu.last_frame_pose_upload_bytes(), 0);
            assert_eq!(state.gpu.last_frame_deformed_meshes(), 0);
            if image.is_none() {
                image = state.image;
                count = ui.image_count();
            } else {
                assert_eq!(image, state.image);
                assert_eq!(count, ui.image_count());
            }
        }
        gallery.start_scene3d_rotation(false);
        gallery.set_reduced_motion(false);
        let stable = gallery.scene3d.diagnostics();
        for _ in 0..120 {
            gallery.animate(1. / 120.);
            gallery.prepare(&mut ui, 1.);
            gallery.build_scene(&mut Scene::new());
            assert_eq!(gallery.scene3d_frame_work(), (0, 0));
        }
        let end = gallery.scene3d.diagnostics();
        assert_eq!(stable["target_rebuilds"], end["target_rebuilds"]);
        assert_eq!(
            stable["vertex_buffer_allocations"],
            end["vertex_buffer_allocations"]
        );
        assert_ne!(stable["model_rotation"], end["model_rotation"]);
    }

    #[test]
    fn scene3d_has_a_stable_gallery_route() {
        let mut gallery = crate::gallery::Gallery::new();
        assert!(gallery.open_route("component-scene3d/playground"));
        assert!(gallery.scene3d_visible());
        let section = super::super::navigation::sections()
            .iter()
            .find(|s| s.title == "3D")
            .unwrap();
        assert!(section.items.iter().any(|(page, _)| *page == gallery.page));
        assert!(gallery.open_route("component-scene3d/docs"));
        assert!(!gallery.scene3d_visible());
    }

    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn scene3d_gallery_routes_clipped_input_and_reuses_gpu_geometry() {
        use tekes_ui_platform_macos::{Modifiers, MouseEvent, ScrollEvent, ScrollPhase, Vec2};
        let mut gallery = crate::gallery::Gallery::new();
        let mut ui = Renderer::new_headless().unwrap();
        gallery.open_route("component-scene3d/playground");
        gallery.set_view_size(1280., 900.);
        gallery.prepare(&mut ui, 1.);
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        let crate::gallery::Page::Workbench(i) = gallery.page else {
            panic!()
        };
        let frame = image_frame(gallery.workbench.preview_rect(i));
        let point = frame
            .intersection(&gallery.viewport.bounds())
            .unwrap()
            .center();
        let mouse = |kind, p: Point| {
            Event::Mouse(MouseEvent {
                kind,
                position: tekes_ui_platform_macos::Point {
                    x: p.x as f64,
                    y: p.y as f64,
                },
                modifiers: Modifiers::NONE,
            })
        };
        let camera = gallery
            .scene3d
            .loaded
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .camera
            .clone();
        gallery.handle_input(&mouse(
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
            point,
        ));
        gallery.handle_input(&mouse(
            MouseEventKind::Dragged {
                button: MouseButton::Left,
            },
            Point::new(point.x + 20., point.y + 12.),
        ));
        gallery.prepare(&mut ui, 1.);
        let loaded = gallery.scene3d.loaded.as_ref().unwrap().as_ref().unwrap();
        assert_ne!(camera, loaded.camera);
        assert_eq!(loaded.gpu.last_frame_deformed_meshes(), 0);
        assert_eq!(loaded.gpu.last_frame_pose_upload_bytes(), 0);
        assert_eq!(
            loaded.view.asset().clips().next().unwrap().name,
            "Top layer quarter turn"
        );
        let old_zoom = loaded.camera.zoom();
        let page_scroll = gallery.viewport.offset;
        gallery.handle_input(&Event::Scroll(ScrollEvent {
            timestamp: 1.,
            delta: Vec2 { x: 0., y: 10. },
            precise: true,
            phase: ScrollPhase::Began,
            momentum_phase: ScrollPhase::None,
            inverted: false,
            position: tekes_ui_platform_macos::Point {
                x: point.x as f64,
                y: point.y as f64,
            },
            modifiers: Modifiers::NONE,
        }));
        assert_ne!(
            old_zoom,
            gallery
                .scene3d
                .loaded
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .camera
                .zoom()
        );
        assert_eq!(page_scroll, gallery.viewport.offset);
        gallery.open_route("introduction");
        assert!(!gallery.scene3d.focused);
        assert!(gallery.scene3d.drag.is_none());
    }

    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn scene3d_gallery_composes_gpu_image_and_stops_off_page() {
        let mut gallery = crate::gallery::Gallery::new();
        let mut ui = Renderer::new_headless().unwrap();
        assert!(gallery.open_route("component-scene3d/playground"));
        gallery.set_view_size(1280., 900.);
        gallery.prepare(&mut ui, 1.);
        assert!(matches!(gallery.scene3d.loaded, Some(Ok(_))));
        let mut scene = Scene::new();
        gallery.build_scene(&mut scene);
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, tekes_ui_scene::Primitive::Image(_)))
        );
        gallery.prepare(&mut ui, 1.);
        scene.clear();
        gallery.build_scene(&mut scene);
        assert!(gallery.scene3d.needs_frame(false));
        assert!(!gallery.scene3d.needs_frame(true));
        let pixels = ui
            .render_to_rgba8(
                &scene,
                gallery.atlas_mut(),
                (1280, 900),
                1.,
                crate::gallery::Gallery::background(Theme::default()),
            )
            .unwrap();
        let image_count = ui.image_count();
        let image_id = gallery
            .scene3d
            .loaded
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .image;
        gallery.animate(0.9);
        gallery.prepare(&mut ui, 1.);
        scene.clear();
        gallery.build_scene(&mut scene);
        let animated = ui
            .render_to_rgba8(
                &scene,
                gallery.atlas_mut(),
                (1280, 900),
                1.,
                crate::gallery::Gallery::background(Theme::default()),
            )
            .unwrap();
        assert_ne!(pixels, animated);
        assert_eq!(image_count, ui.image_count());
        assert_eq!(
            image_id,
            gallery
                .scene3d
                .loaded
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .image
        );
        if let Some(path) = std::env::var_os("TEKESUI_SCENE3D_CAPTURE") {
            let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 1280, 900);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&pixels)
                .unwrap();
        }
        assert!(gallery.open_route("introduction"));
        assert!(!gallery.scene3d_visible());
    }
}
