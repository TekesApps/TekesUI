//! Public API/CoreText contract tests. IconImage is an API lease fixture only;
//! no GPU, canonical SVG appearance, native IME or VoiceOver proof is claimed.
#[path = "support/coretext.rs"]
mod coretext;
use std::{cell::RefCell, rc::Rc, time::Duration};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, surface_palette::resolve_surface};
const ROOT: ComponentId = ComponentId(1);
const TRIGGER: ComponentId = ComponentId(2);
const TITLE: ComponentId = ComponentId(3);
const BODY: ComponentId = ComponentId(4);
#[derive(Default)]
struct Icons {
    calls: Vec<(String, Rect, Rgba)>,
}
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        name: &str,
        size: f32,
        _backing: f32,
        tint: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.calls
            .push((name.into(), Rect::new(0., 0., size, size), tint));
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("must use original checked resource seam")
    }
}
fn host(body: Content, expanded: bool) -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        DisclosureContent::new(
            TRIGGER,
            Content::new(
                TITLE,
                TextContent::new("长期项目分组：包含摘要、正在进行的任务与中文说明"),
            ),
            body,
            expanded,
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn text() -> coretext::Text {
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t.reduced_motion = false;
    t
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, icons: &mut Icons, width: f32) -> Scene {
    let frame = Rect::new(8., 8., width, 240.);
    let mut scene = Scene::new();
    t.atlas.begin_frame();
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(t, icons),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}
fn d(h: &ContentHost) -> &DisclosureContent {
    h.get(ROOT).unwrap().component().unwrap()
}
fn dm(h: &mut ContentHost) -> &mut DisclosureContent {
    h.get_mut(ROOT).unwrap().component_mut().unwrap()
}
fn intent(h: &mut ContentHost) -> (DisclosureIntent, ContentActionRecord) {
    let mut records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    let r = records.remove(0);
    let Some(ContentActionPayload::DisclosureExpansion(i)) = &r.payload else {
        panic!("owned intent absent")
    };
    (i.clone(), r)
}
fn press(h: &mut ContentHost) -> (DisclosureIntent, ContentActionRecord) {
    assert!(
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    intent(h)
}
#[test]
fn controlled_accept_reject_out_of_order_and_same_value_external_update_keep_owner() {
    let mut h = host(Content::new(BODY, Button::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let mount = h.semantic_mount(TRIGGER);
    assert!(h.focus(Some(TRIGGER), true));
    let (a, record) = press(&mut h);
    let (b, _) = press(&mut h);
    assert_ne!(a.generation(), b.generation());
    assert!(a.expanded() && b.expanded());
    assert!(!d(&h).expanded());
    assert!(!dm(&mut h).confirm(&a, true));
    assert!(dm(&mut h).confirm(&b, true));
    assert!(d(&h).expanded());
    assert!(!dm(&mut h).confirm(&b, true));
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(h.semantic_mount(TRIGGER), mount);
    assert_eq!(h.focused(), Some(TRIGGER));
    assert_eq!(record.action.source, ContentActionSource::Accessibility);
    assert_eq!(record.action.target, TRIGGER);
    assert_eq!(Some(record.mount), mount);
    let (c, _) = press(&mut h);
    assert!(!c.expanded());
    assert!(dm(&mut h).confirm(&c, false));
    assert!(d(&h).expanded());
    assert!(!dm(&mut h).confirm(&c, true));
    let (e, _) = press(&mut h);
    assert!(dm(&mut h).set_expanded(true));
    assert!(!dm(&mut h).confirm(&e, true));
    assert!(d(&h).expanded());
    let (f, _) = press(&mut h);
    dm(&mut h).set_disabled(true);
    dm(&mut h).set_disabled(false);
    assert!(!dm(&mut h).confirm(&f, true));
}
#[test]
fn pointer_enter_space_and_ax_have_one_action_and_no_eager_toggle() {
    let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let p = h.get(TRIGGER).unwrap().frame().center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    assert_eq!(h.pointer_capture().unwrap().0, TRIGGER);
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    let (_, r) = intent(&mut h);
    assert_eq!(r.action.source, ContentActionSource::Pointer);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_actions().is_empty());
    assert_eq!(h.focused(), Some(TRIGGER));
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true
        })
        .activated
    );
    let (_, r) = intent(&mut h);
    assert_eq!(r.action.source, ContentActionSource::Keyboard);
    assert!(
        !h.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Enter),
            down: true,
            repeat: true,
            modifiers: InputModifiers::default(),
            native_code: 36,
            characters: "\r".into(),
            characters_ignoring_modifiers: "\r".into()
        }))
        .activated
    );
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: true
        })
        .activated
    );
    assert!(h.drain_actions().is_empty());
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    let _ = intent(&mut h);
    let _ = press(&mut h);
    assert!(!d(&h).expanded());
    assert!(h.drain_actions().is_empty());
}
#[test]
fn disabled_interval_cancel_blur_and_old_drained_mount_do_not_resurrect_intents() {
    let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    h.focus(Some(TRIGGER), true);
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    dm(&mut h).set_disabled(true);
    dm(&mut h).set_disabled(false);
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    h.event(&ContentEvent::Cancel);
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    let (old, _) = press(&mut h);
    dm(&mut h).set_window_active(false);
    dm(&mut h).set_window_active(true);
    assert!(!dm(&mut h).confirm(&old, true));
    let (old, _) = press(&mut h);
    let mut replacement = DisclosureContent::new(
        TRIGGER,
        Content::new(TITLE, TextContent::new("New")),
        Content::new(BODY, TextContent::new("Body")),
        false,
    );
    assert!(!replacement.confirm(&old, true));
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    h.focus(None, false);
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
}
#[derive(Debug)]
struct Probe {
    paints: Rc<RefCell<Vec<ContentTheme>>>,
}
impl Component for Probe {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, 90.))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Button, "Body probe")
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.paints.borrow_mut().push(theme);
    }
    fn event(&mut self, _: &ContentEvent) -> ContentResponse {
        ContentResponse {
            consumed: true,
            activated: true,
            ..Default::default()
        }
    }
    fn accessibility_action(
        &mut self,
        _: Option<usize>,
        _: &AccessibilityAction,
    ) -> ContentResponse {
        ContentResponse {
            consumed: true,
            activated: true,
            ..Default::default()
        }
    }
}
#[test]
fn closing_body_paints_clipped_but_is_inert_and_original_mount_returns() {
    let paints = Rc::new(RefCell::new(vec![]));
    let mut h = host(
        Content::new(
            BODY,
            Probe {
                paints: paints.clone(),
            },
        ),
        true,
    );
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let body_mount = h.semantic_mount(BODY);
    let trigger_mount = h.semantic_mount(TRIGGER);
    assert!(body_mount.is_some());
    assert!(!paints.borrow().is_empty());
    assert!(dm(&mut h).set_expanded(false));
    t.now = Duration::from_millis(10);
    render(&mut h, &mut t, &mut icons, 260.);
    paints.borrow_mut().clear();
    t.now = Duration::from_millis(40);
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(
        !paints.borrow().is_empty(),
        "closing original body must still paint"
    );
    let layout = d(&h).body_layout();
    assert!(!layout.interactive && layout.progress > 0. && layout.progress < 1.);
    assert!(!h.focus_targets().iter().any(|(id, _)| *id == BODY));
    assert!(!h.semantics().iter().any(|n| n.id == BODY));
    assert!(
        !h.accessibility_action(BODY, None, AccessibilityAction::Press)
            .activated
    );
    let p = h.get(BODY).unwrap().frame().center();
    assert!(!h.event(&ContentEvent::PointerDown(p)).activated);
    assert!(dm(&mut h).set_expanded(true));
    t.now = Duration::from_millis(60);
    render(&mut h, &mut t, &mut icons, 260.);
    t.now = Duration::from_millis(400);
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(h.semantic_mount(BODY), body_mount);
    assert_eq!(h.semantic_mount(TRIGGER), trigger_mount);
    assert!(h.focus_targets().iter().any(|(id, _)| *id == BODY));
    assert!(dm(&mut h).set_expanded(false));
    t.reduced_motion = true;
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(d(&h).body_layout().progress, 0.);
}
#[test]
fn marked_text_in_nested_body_rejects_collapse_without_mutating_original_editor() {
    let editor = ComponentId(6);
    let body = Content::new(
        BODY,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.,
            },
            flexible_child: None,
            children: vec![Content::new(
                editor,
                TextFieldContent::new("Editor", "draft"),
            )],
        },
    );
    let mut h = host(body, true);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(h.focus(Some(editor), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let before = h.text_input_snapshot();
    assert!(before.as_ref().unwrap().marked.is_some());
    assert!(!dm(&mut h).set_expanded(false));
    assert!(d(&h).expanded());
    assert_eq!(h.focused(), Some(editor));
    assert_eq!(h.text_input_snapshot(), before);
}
#[test]
fn passive_title_is_validated_and_has_no_second_action_or_ax_owner() {
    let title = Content::new(TITLE, Button::new("bad"));
    assert!(
        ContentHost::new(Content::new(
            ROOT,
            DisclosureContent::new(
                TRIGGER,
                title,
                Content::new(BODY, TextContent::new("Body")),
                false
            )
        ))
        .is_err()
    );
    let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(
        h.focus_targets()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![TRIGGER]
    );
    assert!(!h.semantics().iter().any(|n| n.id == TITLE));
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.semantics.expanded == Some(false))
            .count(),
        1
    );
}
#[test]
fn rtl_narrow_chinese_indicator_and_quiet_header_keep_body_theme_separate() {
    let paints = Rc::new(RefCell::new(vec![]));
    let mut h = host(
        Content::new(
            BODY,
            Probe {
                paints: paints.clone(),
            },
        ),
        true,
    );
    h.set_inherited_surface("sidebar").unwrap();
    let mut t = text();
    let mut icons = Icons::default();
    for rtl in [false, true] {
        dm(&mut h).set_rtl(rtl);
        render(&mut h, &mut t, &mut icons, 120.);
        let r = icons.calls.last().unwrap();
        assert_eq!(r.0, "tekes-expand");
        let theme = *paints.borrow().last().unwrap();
        assert_eq!(theme.foreground, None);
        assert_eq!(theme.icon_foreground, None);
        assert_eq!(theme.surface(), "sidebar");
    }
    dm(&mut h).set_expanded(false);
    t.reduced_motion = true;
    render(&mut h, &mut t, &mut icons, 120.);
    assert_eq!(icons.calls.last().unwrap().0, "tekes-back");
    let expected = resolve_surface(Theme::LIGHT.into(), "sidebar", "action-quiet", "rest").unwrap();
    assert_eq!(icons.calls.last().unwrap().2, expected.icon);
    dm(&mut h).set_rtl(false);
    dm(&mut h).set_disabled(true);
    let scene = render(&mut h, &mut t, &mut icons, 120.);
    assert_eq!(icons.calls.last().unwrap().0, "tekes-forward");
    let expected =
        resolve_surface(Theme::LIGHT.into(), "sidebar", "action-quiet", "disabled").unwrap();
    assert_eq!(icons.calls.last().unwrap().2, expected.icon);
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Glyphs(r)if r.color==expected.text))
    );
}
#[test]
fn disabled_authoritative_updates_and_host_cancellation_retire_delivered_requests() {
    let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    dm(&mut h).set_disabled(true);
    assert!(dm(&mut h).set_expanded(true));
    assert!(d(&h).expanded());
    assert!(d(&h).disabled());
    dm(&mut h).set_disabled(false);
    let (a, _) = press(&mut h);
    h.set_window_active(false);
    h.set_window_active(true);
    assert!(!dm(&mut h).confirm(&a, true));
    assert!(d(&h).expanded());
    let (a, _) = press(&mut h);
    h.cancel();
    assert!(!dm(&mut h).confirm(&a, true));
}

#[test]
fn original_retained_content_remount_rejects_old_drained_intent() {
    let disclosure = Content::new(
        ROOT,
        DisclosureContent::new(
            TRIGGER,
            Content::new(TITLE, TextContent::new("Group")),
            Content::new(BODY, TextContent::new("Body")),
            false,
        ),
    );
    let outer = ComponentId(100);
    let mut h = ContentHost::new(Content::new(
        outer,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![disclosure],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let (old, record) = press(&mut h);
    let original = h.remove(ROOT).unwrap();
    h.insert(outer, original).unwrap();
    render(&mut h, &mut t, &mut icons, 260.);
    assert_ne!(h.semantic_mount(TRIGGER), Some(record.mount));
    assert!(!dm(&mut h).confirm(&old, true));
    let (fresh, _) = press(&mut h);
    assert_ne!(old.generation(), fresh.generation());
    assert!(dm(&mut h).confirm(&fresh, true));
}
#[test]
fn body_preedit_after_collapse_request_rejects_confirmation_and_keeps_identity() {
    let editor = ComponentId(6);
    let mut h = host(
        Content::new(
            BODY,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![Content::new(
                    editor,
                    TextFieldContent::new("Editor", "draft"),
                )],
            },
        ),
        true,
    );
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let mount = h.semantic_mount(editor);
    let (collapse, _) = press(&mut h);
    assert!(!collapse.expanded());
    assert!(h.focus(Some(editor), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let state = h.text_input_snapshot();
    assert!(state.as_ref().unwrap().marked.is_some());
    assert!(!dm(&mut h).confirm(&collapse, true));
    assert!(d(&h).expanded());
    assert_eq!(h.text_input_snapshot(), state);
    assert_eq!(h.semantic_mount(editor), mount);
    assert_eq!(h.focused(), Some(editor));
}
#[test]
fn disabled_pointer_interval_and_subtree_cancel_do_not_rearm_late_release_or_confirmation() {
    let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let p = h.get(TRIGGER).unwrap().frame().center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    dm(&mut h).set_disabled(true);
    dm(&mut h).set_disabled(false);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_actions().is_empty());
    let (old, _) = press(&mut h);
    assert!(h.cancel_subtree(ROOT));
    assert!(!dm(&mut h).confirm(&old, true));
}
#[test]
fn collapsing_original_body_retires_nested_popup_and_never_paints_its_overlay_again() {
    let popup = ComponentId(7);
    let action = ComponentId(8);
    let body = Content::new(
        BODY,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 2.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(6), Button::new("Anchor")),
                Content::new(
                    popup,
                    PopoverContent::new(
                        "Popup",
                        Content::new(action, Button::new("Nested popup action")),
                    ),
                ),
            ],
        },
    );
    let mut h = host(body, true);
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    let _session = h.show_popover(popup, ComponentId(6)).unwrap();
    t.now = Duration::from_millis(300);
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(
        h.get(popup)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .is_open()
    );
    assert!(dm(&mut h).set_expanded(false));
    t.now = Duration::from_millis(310);
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(!h.semantics().iter().any(|n| n.id == action));
    assert!(
        !h.accessibility_action(action, None, AccessibilityAction::Press)
            .activated
    );
    assert!(
        !h.get(popup)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .is_open()
    );
    assert!(dm(&mut h).set_expanded(true));
    t.now = Duration::from_millis(330);
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(
        !h.get(popup)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .is_open()
    );
}

#[test]
fn header_complete_quiet_packet_uses_original_surface_states_and_rtl_geometry() {
    for theme in Theme::ALL {
        for surface in ["sidebar", "code"] {
            for state in ["rest", "hover", "pressed", "disabled"] {
                let mut h = host(Content::new(BODY, TextContent::new("Body")), false);
                h.set_inherited_surface("sidebar").unwrap();
                h.get_mut(TRIGGER)
                    .unwrap()
                    .set_surface(Some(surface))
                    .unwrap();
                let mut t = text();
                let mut icons = Icons::default();
                render(&mut h, &mut t, &mut icons, 120.);
                let p = h.get(TRIGGER).unwrap().frame().center();
                match state {
                    "hover" => {
                        h.event(&ContentEvent::PointerMove(p));
                    }
                    "pressed" => {
                        h.event(&ContentEvent::PointerDown(p));
                    }
                    "disabled" => dm(&mut h).set_disabled(true),
                    _ => {}
                }
                let frame = Rect::new(8., 8., 120., 240.);
                for now in [10, 500] {
                    t.now = Duration::from_millis(now);
                    t.atlas.begin_frame();
                    let mut scene = Scene::new();
                    h.render(
                        &mut scene,
                        &mut WithIconRenderer::new(&mut t, &mut icons),
                        frame,
                        frame,
                        theme.into(),
                    )
                    .unwrap();
                    if now == 500 {
                        let expected =
                            resolve_surface(theme.into(), surface, "action-quiet", state).unwrap();
                        assert_eq!(
                            icons.calls.last().unwrap().2,
                            expected.icon,
                            "{theme:?}/{surface}/{state}"
                        );
                        assert!(
                            scene.primitives().iter().any(
                                |p| matches!(p,Primitive::Glyphs(run)if run.color==expected.text)
                            ),
                            "title packet {surface}/{state}"
                        );
                    }
                }
                dm(&mut h).set_rtl(true);
                let scene = render(&mut h, &mut t, &mut icons, 120.);
                let icon = scene
                    .primitives()
                    .iter()
                    .find_map(|p| {
                        if let Primitive::Image(i) = p {
                            Some(i.rect)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let title = h.get(TITLE).unwrap().frame();
                let trigger = h.get(TRIGGER).unwrap().frame();
                assert!(icon.min_x() >= trigger.min_x() && icon.max_x() <= trigger.max_x());
                assert!(icon.center().x >= title.max_x());
            }
        }
    }
}

#[test]
fn host_window_loss_snaps_original_closing_reveal_and_requests_no_frame() {
    let mut h = host(
        Content::new(
            BODY,
            Probe {
                paints: Rc::new(RefCell::new(vec![])),
            },
        ),
        true,
    );
    let mut t = text();
    let mut icons = Icons::default();
    render(&mut h, &mut t, &mut icons, 260.);
    dm(&mut h).set_expanded(false);
    render(&mut h, &mut t, &mut icons, 260.);
    t.now = Duration::from_millis(30);
    render(&mut h, &mut t, &mut icons, 260.);
    assert!(d(&h).body_layout().progress > 0.);
    h.set_window_active(false);
    t.now = Duration::from_millis(40);
    t.requested = false;
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(d(&h).body_layout().progress, 0.);
    assert_eq!(d(&h).body_layout().occupied_height, 0.);
    assert!(!h.animation_frame_requested());
    assert!(!t.requested);
    // An authoritative update while inactive must snap too, after the first
    // cancellation frame has consumed reveal_suspended.
    dm(&mut h).set_expanded(true);
    t.now = Duration::from_millis(50);
    t.requested = false;
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(d(&h).body_layout().progress, 1.);
    assert!(!t.requested);
    h.set_window_active(true);
    t.now = Duration::from_millis(60);
    render(&mut h, &mut t, &mut icons, 260.);
    assert_eq!(d(&h).body_layout().progress, 1.);
}

#[test]
fn body_reveal_uses_ancestor_horizontal_paint_room_without_widening_hit_geometry() {
    let mut h = host(Content::new(BODY, Button::new("Original body")), true);
    let mut t = text();
    let mut icons = Icons::default();
    let frame = Rect::new(8., 8., 344., 240.);
    for viewport in [
        Rect::new(0., 0., 360., 260.),
        Rect::new(10., 0., 330., 260.),
    ] {
        let mut scene = Scene::new();
        t.atlas.begin_frame();
        h.render(
            &mut scene,
            &mut WithIconRenderer::new(&mut t, &mut icons),
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let body = h.get(BODY).unwrap().frame();
        let reveal = d(&h).body_layout().visible_clip.unwrap();
        assert_eq!(reveal.min_x(), viewport.min_x());
        assert_eq!(reveal.max_x(), viewport.max_x());
        assert_eq!(body.min_x(), frame.min_x());
        assert_eq!(body.size.width, frame.size.width);
        assert!(reveal.max_y() <= frame.max_y());
        let mount = h.get(BODY).unwrap().mount_identity();
        let semantic_frame = h.semantics().iter().find(|n| n.id == BODY).unwrap().frame;
        assert_eq!(semantic_frame, body);
        let outside = tekes_ui_scene::Point::new(frame.min_x() - 1., body.center().y);
        assert!(!h.event(&ContentEvent::PointerDown(outside)).capture_pointer);
        assert!(!h.event(&ContentEvent::PointerUp(outside)).activated);
        assert!(h.drain_action_records().is_empty());
        assert_eq!(h.get(BODY).unwrap().mount_identity(), mount);
    }
}
