//! Original Scene3D model and GPU output mounted in the one AppHost ContentHost.
use super::super::scene3d_preview::Preview3D;
use super::Entry;
use anyhow::Result;
use std::{cell::RefCell, rc::Rc, time::Duration};
use tekes_ui_app_host::ContainerRenderContext;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_platform_macos as native;
use tekes_ui_scene::{Rect, Scene, Size};

pub(crate) const SPECIMEN: ComponentId = ComponentId(95020);
const ROOT: ComponentId = ComponentId(95000);
const ROWS: ComponentId = ComponentId(95001);
const BODY: ComponentId = ComponentId(95002);
const COMMAND: u64 = 95100;

#[derive(Debug)]
struct State {
    preview: Preview3D,
    frame: Rect,
    clip: Rect,
    active: bool,
    runnable: bool,
    last_time: Option<Duration>,
    focused: bool,
    focus_visible: bool,
    focus_space: RequiredFocusSpace,
    backing_scale: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            preview: Preview3D::default(),
            frame: Rect::default(),
            clip: Rect::default(),
            active: true,
            runnable: false,
            last_time: None,
            focused: false,
            focus_visible: false,
            focus_space: RequiredFocusSpace::Unmeasured,
            backing_scale: 1.,
        }
    }
}
impl State {
    // Keyboard focus belongs to the actual visible image viewport, not the
    // taller document block containing its caption. Keep the same boundary
    // for Host reveal admission and the focus indicator we paint.
    fn refresh_focus_space(&mut self) {
        self.focus_space = RequiredFocusSpace::Insufficient;
        if !self.active {
            return;
        }
        let image = super::super::scene3d_preview::image_frame(self.frame);
        let metrics = tekes_ui_controls::extended::FocusRingMetrics::new(self.backing_scale);
        // If the image is offscreen, retain a real image-region of viewport
        // size for keyboard reveal. Runnable governs animation, not whether
        // keyboard traversal may scroll this already laid-out owner into view.
        let fits = |r: Rect| {
            r.size.width > 4. * metrics.reserved_band()
                && r.size.height > 4. * metrics.reserved_band()
        };
        let visible = image.intersection(&self.clip).filter(|r| fits(*r));
        let boundary = visible.unwrap_or_else(|| {
            Rect::new(
                image.min_x(),
                image.min_y(),
                image.size.width.min(self.clip.size.width),
                image.size.height.min(self.clip.size.height),
            )
        });
        if !fits(boundary) {
            return;
        }
        self.focus_space = RequiredFocusSpace::Sufficient {
            witness: boundary.inset(metrics.antialias_guard),
            evidence: RequiredFocusEvidence::new(),
        };
    }
    fn stop(&mut self) {
        self.last_time = None;
        self.preview.blur();
        self.focus_space = RequiredFocusSpace::Insufficient;
    }
    fn event(&mut self, event: native::Event) -> ContentResponse {
        if !self.active || !self.runnable {
            return ContentResponse::default();
        }
        let handled = self.preview.handle(&event, self.frame, self.clip);
        ContentResponse {
            capture_pointer: handled
                && matches!(
                    event,
                    native::Event::Mouse(native::MouseEvent {
                        kind: native::MouseEventKind::Down { .. },
                        ..
                    })
                ),
            ..if handled {
                ContentResponse::consumed(true)
            } else {
                ContentResponse::default()
            }
        }
    }
}
#[derive(Debug)]
struct PreviewContent(Rc<RefCell<State>>);
fn modifiers(m: InputModifiers) -> native::Modifiers {
    native::Modifiers {
        shift: m.shift,
        control: m.control,
        alt: m.alt,
        command: m.command,
        caps_lock: m.caps_lock,
        function: m.function,
    }
}
impl Component for PreviewContent {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        true
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut state = self.0.borrow_mut();
        state.preview.set_focused(focused);
        state.focused = focused;
        state.focus_visible = focused && visible;
    }
    fn requires_focus_reveal(&self) -> bool {
        true
    }
    fn required_focus_space(&self) -> RequiredFocusSpace {
        self.0.borrow().focus_space.clone()
    }
    fn required_focus_result(&self) -> Option<RequiredFocusResult> {
        let state = self.0.borrow();
        Some(match &state.focus_space {
            RequiredFocusSpace::Sufficient { witness, .. }
                if witness.intersection(&state.clip) == Some(*witness) =>
            {
                RequiredFocusResult::Visible { witness: *witness }
            }
            RequiredFocusSpace::Sufficient { witness, .. } => {
                RequiredFocusResult::Clipped { witness: *witness }
            }
            RequiredFocusSpace::Insufficient => RequiredFocusResult::Insufficient,
            _ => RequiredFocusResult::Unmeasured,
        })
    }
    fn focus_boundary_visible(&self) -> bool {
        let state = self.0.borrow();
        state.focused
            && state.focus_visible
            && matches!(
                self.required_focus_result(),
                Some(RequiredFocusResult::Visible { .. })
            )
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.0.borrow_mut().focus_space = RequiredFocusSpace::Unmeasured;
        let width = available.width.min(720.);
        MeasuredChild::new(Size::new(width, (width + 48.).min(620.)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let mut state = self.0.borrow_mut();
        state.frame = frame;
        state.clip = clip;
        state.preview.record_layout(frame);
        state.backing_scale = text.backing_scale();
        state.refresh_focus_space();
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let state = self.0.borrow();
        scene.push_clip(clip);
        state
            .preview
            .paint_content(scene, frame, text, theme.palette);
        if state.focused
            && state.focus_visible
            && let RequiredFocusSpace::Sufficient { witness, .. } = state.focus_space
            && witness.intersection(&state.clip) == Some(witness)
        {
            let metrics = tekes_ui_controls::extended::FocusRingMetrics::new(state.backing_scale);
            tekes_ui_controls::paint::focus_ring(
                scene,
                witness.inset(metrics.reserved_band()),
                theme.palette,
                0.,
            );
        }
        scene.pop_clip();
        if state.active
            && state.runnable
            && text.animation_frames_allowed()
            && text
                .motion_context()
                .is_some_and(|clock| state.preview.needs_frame(clock.reduced_motion))
        {
            text.request_animation_frame();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        let mut state = self.0.borrow_mut();
        state.active = active;
        if !active {
            state.stop();
        }
        state.refresh_focus_space();
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        let mut state = self.0.borrow_mut();
        state.runnable = runnable;
        if !runnable {
            state.stop();
        }
        state.refresh_focus_space();
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        let mut state = self.0.borrow_mut();
        state.runnable = false;
        state.stop();
        state.focus_space = RequiredFocusSpace::Unmeasured;
        state.focused = false;
        state.focus_visible = false;
        state.frame = Rect::default();
        state.clip = Rect::default();
        // New Content nodes start active; Host attachment then projects inactive
        // ancestry if needed. A retained GPU model must follow the new mount.
        if event == Lifecycle::Mounted {
            state.active = true;
        }
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        let button = match pointer.button {
            Some(PointerButton::Primary) => native::MouseButton::Left,
            Some(PointerButton::Secondary) => native::MouseButton::Right,
            Some(PointerButton::Middle) => native::MouseButton::Middle,
            Some(PointerButton::Other(n)) => native::MouseButton::Other(n),
            None => return ContentResponse::default(),
        };
        let kind = match pointer.phase {
            PointerPhase::Down => native::MouseEventKind::Down {
                button,
                click_count: pointer.click_count,
            },
            PointerPhase::Drag => native::MouseEventKind::Dragged { button },
            PointerPhase::Up => native::MouseEventKind::Up {
                button,
                click_count: pointer.click_count,
            },
            _ => return ContentResponse::default(),
        };
        self.0
            .borrow_mut()
            .event(native::Event::Mouse(native::MouseEvent {
                kind,
                position: native::Point {
                    x: pointer.position.x as f64,
                    y: pointer.position.y as f64,
                },
                modifiers: modifiers(pointer.modifiers),
            }))
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        self.0.borrow_mut().preview.cancel_drag();
        ContentResponse::default()
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        // The Host is the keyboard focus authority; the retained camera reducer
        // must remain in sync after an unhandled key or pointer-only cancellation.
        self.0.borrow_mut().preview.set_focused(true);
        self.0
            .borrow_mut()
            .event(native::Event::Key(native::KeyEvent {
                kind: if key.down {
                    native::KeyEventKind::Down
                } else {
                    native::KeyEventKind::Up
                },
                key_code: key.native_code,
                characters: key.characters.clone(),
                characters_ignoring_modifiers: key.characters_ignoring_modifiers.clone(),
                modifiers: modifiers(key.modifiers),
                is_repeat: key.repeat,
            }))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Cancel => {
                self.0.borrow_mut().stop();
                ContentResponse::default()
            }
            ContentEvent::Scroll {
                position,
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
            } => {
                let phase = if *begin {
                    native::ScrollPhase::Began
                } else if *end {
                    native::ScrollPhase::Ended
                } else {
                    native::ScrollPhase::Changed
                };
                // Content's positive-later convention is the inverse of native
                // deltas used by the retained Preview3D camera reducer.
                self.0
                    .borrow_mut()
                    .event(native::Event::Scroll(native::ScrollEvent {
                        timestamp: *time,
                        delta: native::Vec2 {
                            x: -delta.x as f64,
                            y: -delta.y as f64,
                        },
                        precise: *precise,
                        phase: if *momentum {
                            native::ScrollPhase::None
                        } else {
                            phase
                        },
                        momentum_phase: if *momentum {
                            phase
                        } else {
                            native::ScrollPhase::None
                        },
                        inverted: false,
                        position: native::Point {
                            x: position.x as f64,
                            y: position.y as f64,
                        },
                        modifiers: native::Modifiers::NONE,
                    }))
            }
            _ => ContentResponse::default(),
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Interactive Scene3D preview")
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let state = self.0.borrow();
        Some(vec![
            InspectionProperty::new("component", "Scene3D"),
            InspectionProperty::new("render", state.preview.diagnostics().to_string()),
            InspectionProperty::new("active", state.active.to_string()),
            InspectionProperty::new("runnable", state.runnable.to_string()),
        ])
    }
}

pub(crate) struct AppHostScene3D {
    pub(crate) host: ContentHost,
    state: Rc<RefCell<State>>,
    entry: &'static Entry,
    playground: bool,
}
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
impl AppHostScene3D {
    pub(crate) fn new(entry: &'static Entry) -> Result<Self> {
        let state = Rc::new(RefCell::new(State::default()));
        let commands = Content::new(
            ComponentId(95006),
            StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: 8.,
                },
                children: [(1, "Docs"), (2, "Playground"), (0, "Reset")]
                    .into_iter()
                    .map(|(n, label)| Content::new(ComponentId(COMMAND + n), Button::new(label)))
                    .collect(),
                flexible_child: None,
            },
        );
        let mut host = ContentHost::new(Content::new(
            ROOT,
            ListContent::scrollable(
                "Scene3D stories",
                ROWS,
                vec![
                    super::story_heading(95004, entry.title()),
                    text(95005, super::story_docs(entry)),
                    commands,
                    Self::body(entry, true, state.clone()),
                ],
            )
            .content_padding(16.),
        ))
        .map_err(|e| anyhow::anyhow!("Scene3D host: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            state,
            entry,
            playground: true,
        })
    }
    fn body(entry: &Entry, playground: bool, state: Rc<RefCell<State>>) -> Content {
        if playground {
            Content::new(
                BODY,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 12.,
                    },
                    children: vec![
                        text(95007, "Playground"),
                        Content::new(SPECIMEN, PreviewContent(state)),
                    ],
                    flexible_child: None,
                },
            )
        } else {
            Content::new(
                BODY,
                TextContent::new(format!(
                    "{}\n{}\n{}",
                    entry.title(),
                    super::story_docs(entry),
                    entry.get("usage")
                )),
            )
        }
    }
    pub(crate) fn after_host_visit(&mut self) {
        for record in self.host.drain_action_records() {
            if record.action.kind != ContentActionKind::Activate {
                continue;
            }
            match record.action.target.0.checked_sub(COMMAND) {
                Some(0) => {
                    let mut state = self.state.borrow_mut();
                    state.preview.reset();
                    state.last_time = None;
                    let _ = self.host.get_mut(SPECIMEN);
                }
                Some(n @ 1..=2) if self.playground != (n == 2) => {
                    self.playground = n == 2;
                    self.host.remove(BODY).expect("original Scene3D body");
                    self.host
                        .insert(
                            ROWS,
                            Self::body(self.entry, self.playground, self.state.clone()),
                        )
                        .expect("unique Scene3D body");
                }
                _ => {}
            }
        }
    }
    pub(crate) fn prepare_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> Result<()> {
        if self.playground {
            self.state.borrow_mut().preview.prepare_resources(
                context.renderer,
                context.motion.is_some_and(|m| m.reduced_motion),
                context.scale,
            );
        }
        Ok(())
    }
    pub(crate) fn finish_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> Result<()> {
        if !self.playground {
            return Ok(());
        }
        let mut state = self.state.borrow_mut();
        if state.active && state.runnable {
            if let Some(clock) = context.motion {
                let dt = state
                    .last_time
                    .map_or(0., |old| clock.now.saturating_sub(old).as_secs_f32());
                state.last_time = Some(clock.now);
                state.preview.advance(dt, clock.reduced_motion);
            }
        } else {
            state.last_time = None;
        }
        // arrange has recorded this frame's actual nested preview size; update
        // the same registration before the window submits its scene.
        state.preview.prepare(
            context.renderer,
            context.motion.is_some_and(|m| m.reduced_motion),
            context.scale,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::text::TextEngine;
    use super::*;
    use tekes_ui_controls::ControlMotionContext;
    use tekes_ui_render::Renderer;
    use tekes_ui_tokens::{Palette, Theme};

    fn page() -> AppHostScene3D {
        AppHostScene3D::new(
            super::super::entries()
                .iter()
                .find(|e| e.renderer() == "scene3d")
                .unwrap(),
        )
        .unwrap()
    }
    fn draw(
        page: &mut AppHostScene3D,
        renderer: &mut Renderer,
        text: &mut TextEngine,
        width: f32,
        now: f64,
        reduced: bool,
    ) -> (Scene, Vec<u8>) {
        text.begin_frame();
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::from_secs_f64(now), reduced);
        let palette = Palette::from(Theme::default());
        let frame = Rect::new(0., 0., width, 900.);
        let mut context = ContainerRenderContext {
            renderer,
            frame,
            clip: frame,
            scale: 1.,
            motion: Some(ControlMotionContext {
                now: Duration::from_secs_f64(now),
                reduced_motion: reduced,
            }),
            palette,
        };
        page.prepare_resources(&mut context).unwrap();
        let mut scene = Scene::new();
        page.host
            .render(&mut scene, text, frame, frame, palette)
            .unwrap();
        page.finish_resources(&mut context).unwrap();
        let pixels = context
            .renderer
            .render_to_rgba8(
                &scene,
                text.atlas_mut(),
                (width as u32, 900),
                1.,
                crate::gallery::Gallery::background(Theme::default()),
            )
            .unwrap();
        (scene, pixels)
    }
    #[test]
    fn apphost_scene3d_activity_retires_clock_and_gestures_without_losing_page() {
        let mut page = page();
        page.state.borrow_mut().last_time = Some(Duration::from_secs(10));
        page.host.set_window_active(false);
        assert!(!page.state.borrow().active);
        assert_eq!(page.state.borrow().last_time, None);
        page.host.set_window_active(true);
        assert!(page.state.borrow().active);
        assert_eq!(page.state.borrow().last_time, None);
        assert!(page.host.get(SPECIMEN).is_some());
    }
    #[test]
    fn apphost_scene3d_visible_viewport_focus_does_not_scroll_on_first_drag() {
        let mut page = page();
        let mut text = TextEngine::new();
        let frame = Rect::new(0., 0., 640., 480.);
        let render = |page: &mut AppHostScene3D, text: &mut TextEngine| {
            let mut scene = Scene::new();
            page.host
                .render(&mut scene, text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            scene
        };
        render(&mut page, &mut text);
        let geometry = |page: &AppHostScene3D| {
            let state = page.state.borrow();
            (
                page.host
                    .get(ROOT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .offset,
                state.frame,
                state.clip,
            )
        };
        let original = geometry(&page);
        assert!(
            original.1.max_y() > original.2.max_y(),
            "fixture must clip the tall preview"
        );
        // This is the same focus admission invoked by the first pointer Down,
        // without needing a GPU image or substituting camera focus for the Host.
        assert!(page.host.focus(Some(SPECIMEN), false));
        render(&mut page, &mut text);
        assert_eq!(
            geometry(&page),
            original,
            "pointer focus moved the preview beneath the drag"
        );
        assert!(
            !page
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<PreviewContent>()
                .unwrap()
                .focus_boundary_visible()
        );
        page.host.focus(None, false);
        assert!(page.host.focus(Some(SPECIMEN), true));
        let scene = render(&mut page, &mut text);
        assert_eq!(geometry(&page), original);
        let node = page
            .host
            .get(SPECIMEN)
            .unwrap()
            .component::<PreviewContent>()
            .unwrap();
        assert!(node.focus_boundary_visible());
        let RequiredFocusSpace::Sufficient { witness, .. } = node.required_focus_space() else {
            panic!("visible image boundary")
        };
        assert_eq!(witness.intersection(&original.2), Some(witness));
        assert!(scene.primitives().iter().any(|p| matches!(p, tekes_ui_scene::Primitive::Quad(q) if q.border_width > 0. && q.rect.intersection(&witness) == Some(q.rect))), "keyboard focus must actually paint its boundary");
        // A fully offscreen target must still be reachable by ordinary
        // keyboard focus; only animation is suspended by presentation clipping.
        let small = Rect::new(0., 0., 640., 120.);
        page.host.focus(None, false);
        page.host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .offset = Default::default();
        page.host
            .render(
                &mut Scene::new(),
                &mut text,
                small,
                small,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(!page.state.borrow().runnable);
        assert!(matches!(
            page.host
                .get(SPECIMEN)
                .unwrap()
                .component::<PreviewContent>()
                .unwrap()
                .required_focus_result(),
            Some(RequiredFocusResult::Clipped { .. })
        ));
        assert!(
            page.host.focus(Some(SPECIMEN), true),
            "offscreen keyboard target must reveal"
        );
        page.host
            .render(
                &mut Scene::new(),
                &mut text,
                small,
                small,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(page.state.borrow().runnable);
        assert!(
            page.host
                .get(SPECIMEN)
                .unwrap()
                .component::<PreviewContent>()
                .unwrap()
                .focus_boundary_visible()
        );
        let offset = page
            .host
            .get(ROOT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        page.host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .offset = Default::default();
        page.host
            .render(
                &mut Scene::new(),
                &mut text,
                small,
                small,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(!page.state.borrow().runnable);
        page.host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .offset = offset;
        page.host
            .render(
                &mut Scene::new(),
                &mut text,
                small,
                small,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(
            page.host
                .get(SPECIMEN)
                .unwrap()
                .component::<PreviewContent>()
                .unwrap()
                .focus_boundary_visible(),
            "scroll clipping must not clear Host logical focus"
        );
        page.host.set_window_active(false);
        assert_eq!(
            page.host
                .get(SPECIMEN)
                .unwrap()
                .component::<PreviewContent>()
                .unwrap()
                .required_focus_space(),
            RequiredFocusSpace::Insufficient
        );
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn apphost_scene3d_first_frame_resize_input_activity_and_renderer_replacement() {
        let mut page = page();
        let mut renderer = Renderer::new_headless().unwrap();
        let mut text = TextEngine::new();
        let (scene, pixels) = draw(&mut page, &mut renderer, &mut text, 360., 1., true);
        let image_id = scene
            .primitives()
            .iter()
            .find_map(|p| {
                if let tekes_ui_scene::Primitive::Image(q) = p {
                    Some(q.image)
                } else {
                    None
                }
            })
            .expect("first frame must reference the prepared GPU image");
        // Caption/controls are neutral; the actual Rubik cube contributes color.
        assert!(
            pixels
                .chunks_exact(4)
                .filter(|p| p[0].abs_diff(p[1]) > 20 || p[1].abs_diff(p[2]) > 20)
                .count()
                > 100
        );
        let diagnostic = page.state.borrow().preview.diagnostics();
        assert_ne!(diagnostic, serde_json::Value::Null);
        let frame = page.state.borrow().frame;
        assert_eq!(
            diagnostic["target_px"][0].as_u64().unwrap(),
            frame
                .size
                .width
                .min((frame.size.height - 48.).max(0.))
                .round() as u64
        );
        let (resized, _) = draw(&mut page, &mut renderer, &mut text, 520., 1., true);
        assert!(
            resized
                .primitives()
                .iter()
                .any(|p| matches!(p,tekes_ui_scene::Primitive::Image(q) if q.image==image_id))
        );
        let resized_diagnostic = page.state.borrow().preview.diagnostics();
        assert_ne!(diagnostic["target_px"], resized_diagnostic["target_px"]);
        assert_eq!(
            diagnostic["vertex_buffer_allocations"],
            resized_diagnostic["vertex_buffer_allocations"]
        );
        let frame = page.state.borrow().frame;
        let point = tekes_ui_scene::Point::new(frame.center().x, frame.min_y() + 80.);
        let pointer = |phase, position| {
            ContentEvent::Pointer(PointerEvent {
                phase,
                position,
                button: Some(PointerButton::Secondary),
                modifiers: InputModifiers::default(),
                click_count: 1,
            })
        };
        assert!(
            page.host
                .event(&pointer(PointerPhase::Down, point))
                .consumed
        );
        assert!(
            page.host
                .event(&pointer(
                    PointerPhase::Drag,
                    tekes_ui_scene::Point::new(point.x + 25., point.y + 15.)
                ))
                .consumed
        );
        page.host.event(&pointer(PointerPhase::Up, point));
        assert_ne!(
            page.state.borrow().preview.diagnostics()["model_rotation"],
            resized_diagnostic["model_rotation"]
        );
        let zoom = page.state.borrow().preview.diagnostics()["camera_zoom"].clone();
        assert!(
            page.host
                .event(&ContentEvent::Scroll {
                    position: point,
                    delta: tekes_ui_scene::Point::new(0., 12.),
                    precise: true,
                    begin: true,
                    end: false,
                    momentum: false,
                    time: 1.2
                })
                .consumed
        );
        assert_ne!(
            page.state.borrow().preview.diagnostics()["camera_zoom"],
            zoom
        );
        draw(&mut page, &mut renderer, &mut text, 520., 2., false);
        let paused = page.state.borrow().preview.diagnostics()["clip_time"].clone();
        page.host.set_window_active(false);
        draw(&mut page, &mut renderer, &mut text, 520., 100., false);
        assert_eq!(
            page.state.borrow().preview.diagnostics()["clip_time"],
            paused
        );
        page.host.set_window_active(true);
        draw(&mut page, &mut renderer, &mut text, 520., 200., false);
        assert_eq!(
            page.state.borrow().preview.diagnostics()["clip_time"],
            paused,
            "resume must not integrate hidden time"
        );
        draw(&mut page, &mut renderer, &mut text, 520., 200.05, false);
        assert_ne!(
            page.state.borrow().preview.diagnostics()["clip_time"],
            paused
        );
        let before = page.state.borrow().preview.diagnostics();
        let mut replacement = Renderer::new_headless().unwrap();
        draw(&mut page, &mut replacement, &mut text, 520., 200.05, true);
        let after = page.state.borrow().preview.diagnostics();
        assert_eq!(before["camera_zoom"], after["camera_zoom"]);
        assert_eq!(before["model_rotation"], after["model_rotation"]);
        assert_eq!(before["clip_time"], after["clip_time"]);
        assert_eq!(
            renderer.collect_retired_images(),
            1,
            "original Renderer reclaims its own registration"
        );
        assert_eq!(replacement.image_count(), 1);
        let mut docs_text = TextEngine::new();
        let frame = Rect::new(0., 0., 520., 900.);
        page.host
            .render(
                &mut Scene::new(),
                &mut docs_text,
                frame,
                frame,
                Palette::from(Theme::default()),
            )
            .unwrap();
        page.host
            .accessibility_action(ComponentId(COMMAND + 1), None, AccessibilityAction::Press);
        page.after_host_visit();
        assert!(!page.playground);
        assert!(page.host.get(SPECIMEN).is_none());
        assert_eq!(page.state.borrow().last_time, None);
    }
}
