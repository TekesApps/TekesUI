//! Original rectangular Scroll input admission, not native/GPU acceptance.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use std::rc::Rc;
use tekes_ui_controls::{
    ButtonTextRenderer, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
    layout_view::IconRenderer,
};
use tekes_ui_scene::{ImageId, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme};
const ROOT: ComponentId = ComponentId(1);
const TRIGGER: ComponentId = ComponentId(2);
const BODY: ComponentId = ComponentId(4);
const EDITOR: ComponentId = ComponentId(5);
const OUTSIDE: ComponentId = ComponentId(7);
struct Icons;
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("must use the prepared icon service")
    }
}
fn disclosure(preserve_focus: bool) -> DisclosureContent {
    let mut control = DisclosureContent::new(
        TRIGGER,
        Content::new(ComponentId(3), TextContent::new("folder")),
        Content::new(BODY, TextFieldContent::new("Editor", "draft")),
        true,
    );
    if preserve_focus {
        let children = control.children_mut().unwrap();
        let trigger = children.remove(0).preserving_focus();
        children.insert(0, trigger);
    }
    control
}
fn host(preserve_focus: bool) -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ComponentId(10),
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(ROOT, disclosure(preserve_focus)),
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn text() -> coretext::Text {
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, clip: Rect) {
    t.atlas.begin_frame();
    h.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(t, &mut Icons),
        Rect::new(0., 0., 400., 300.),
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn viewport() -> Rect {
    Rect::new(0., 0., 400., 300.)
}
fn control(h: &ContentHost) -> &DisclosureContent {
    h.get(ROOT).unwrap().component().unwrap()
}
fn control_mut(h: &mut ContentHost) -> &mut DisclosureContent {
    h.get_mut(ROOT).unwrap().component_mut().unwrap()
}
fn take_intent(h: &mut ContentHost) -> DisclosureIntent {
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, TRIGGER);
    assert_eq!(records[0].mount, h.get(TRIGGER).unwrap().mount_identity());
    let Some(ContentActionPayload::DisclosureExpansion(intent)) = &records[0].payload else {
        panic!("the original trigger must emit its owned intent");
    };
    assert!(
        h.drain_actions().is_empty(),
        "one once-only rich/legacy queue"
    );
    assert!(
        !intent.expanded(),
        "the request snapshots the confirmed open state"
    );
    intent.clone()
}
#[test]
fn scroll_ax_press_emits_one_owned_intent_and_accept_reject_keep_original_mounts() {
    for preserve in [false, true] {
        let mut h = host(preserve);
        let mut t = text();
        render(&mut h, &mut t, viewport());
        let mounts = (
            h.get(TRIGGER).unwrap().mount_identity(),
            h.get(BODY).unwrap().mount_identity(),
        );
        let response = h.accessibility_action(TRIGGER, None, AccessibilityAction::Press);
        assert!(response.consumed && response.activated, "{response:?}");
        let intent = take_intent(&mut h);
        assert!(control(&h).expanded(), "requests never eagerly toggle");
        assert!(control_mut(&mut h).confirm(&intent, false));
        assert!(!control_mut(&mut h).confirm(&intent, true));
        render(&mut h, &mut t, viewport());
        assert!(
            h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
                .activated
        );
        let intent = take_intent(&mut h);
        assert!(control_mut(&mut h).confirm(&intent, true));
        assert!(!control(&h).expanded());
        assert!(!h.get(BODY).unwrap().active());
        assert_eq!(
            (
                h.get(TRIGGER).unwrap().mount_identity(),
                h.get(BODY).unwrap().mount_identity()
            ),
            mounts
        );
        assert!(h.drain_action_records().is_empty());
    }
}
#[test]
fn scroll_pointer_uses_original_capture_and_releases_one_intent_without_eager_toggle() {
    for preserve in [false, true] {
        let mut h = host(preserve);
        let mut t = text();
        render(&mut h, &mut t, viewport());
        let point = h.get(TRIGGER).unwrap().frame().center();
        assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
        assert!(h.drain_action_records().is_empty());
        let response = h.event(&ContentEvent::PointerUp(point));
        assert!(response.consumed && response.activated, "{response:?}");
        let intent = take_intent(&mut h);
        assert!(control(&h).expanded());
        assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
        assert!(h.drain_action_records().is_empty());
        assert!(control_mut(&mut h).confirm(&intent, true));
        assert!(!control(&h).expanded());
    }
}
#[test]
fn unarranged_clipped_and_inactive_scroll_headers_keep_input_refusal() {
    let mut h = host(true);
    let mut t = text();
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
    render(&mut h, &mut t, Rect::new(0., 60., 400., 240.));
    let point = h.get(TRIGGER).unwrap().frame().center();
    assert!(h.get(TRIGGER).unwrap().visible().is_none());
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(!h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(h.drain_action_records().is_empty());
    render(&mut h, &mut t, viewport());
    control_mut(&mut h).set_window_active(false);
    control_mut(&mut h).set_window_active(true);
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(!h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(h.drain_action_records().is_empty());
    render(&mut h, &mut t, viewport());
    assert!(
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let intent = take_intent(&mut h);
    assert!(control_mut(&mut h).confirm(&intent, false));
    h.set_window_active(false);
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(!h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(h.drain_action_records().is_empty());
    assert!(control(&h).expanded());
}
#[test]
fn disabled_scroll_header_and_disable_interval_cannot_resurrect_pointer_or_intent() {
    let mut h = host(true);
    let mut t = text();
    control_mut(&mut h).set_disabled(true);
    render(&mut h, &mut t, viewport());
    let point = h.get(TRIGGER).unwrap().frame().center();
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(!h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(h.drain_action_records().is_empty());
    control_mut(&mut h).set_disabled(false);
    render(&mut h, &mut t, viewport());
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    control_mut(&mut h).set_disabled(true);
    control_mut(&mut h).set_disabled(false);
    render(&mut h, &mut t, viewport());
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(h.drain_action_records().is_empty());
    assert!(control(&h).expanded());
}
#[test]
fn preserving_scroll_header_keeps_marked_editor_and_sibling_while_confirmation_refuses_hide() {
    let mut control = DisclosureContent::new(
        TRIGGER,
        Content::new(ComponentId(3), TextContent::new("folder")),
        Content::new(
            BODY,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![Content::new(
                    EDITOR,
                    TextFieldContent::new("Editor", "draft"),
                )],
            },
        ),
        true,
    );
    let trigger = control.children_mut().unwrap().remove(0).preserving_focus();
    control.children_mut().unwrap().insert(0, trigger);
    let mut h = ContentHost::new(Content::new(
        ComponentId(10),
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                ComponentId(20),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![
                        Content::new(ROOT, control),
                        Content::new(OUTSIDE, TextFieldContent::new("Outside", "outside draft")),
                    ],
                },
            ),
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = text();
    render(&mut h, &mut t, viewport());
    assert!(h.focus(Some(EDITOR), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut h, &mut t, viewport());
    let before = h.text_input_snapshot();
    assert!(before.as_ref().unwrap().marked.is_some());
    let outside = {
        let input = &h.text_surface(OUTSIDE).unwrap().input;
        (
            input.text.clone(),
            input.anchor,
            input.caret,
            input.marked.clone(),
        )
    };
    let response = h.accessibility_action(TRIGGER, None, AccessibilityAction::Press);
    assert!(response.consumed && response.activated, "{response:?}");
    let intent = take_intent(&mut h);
    assert!(!control_mut(&mut h).confirm(&intent, true));
    assert!(control(&h).expanded());
    assert_eq!(h.focused(), Some(EDITOR));
    assert_eq!(h.text_input_snapshot(), before);
    let input = &h.text_surface(OUTSIDE).unwrap().input;
    assert_eq!(
        (
            input.text.clone(),
            input.anchor,
            input.caret,
            input.marked.clone()
        ),
        outside
    );
    assert!(control_mut(&mut h).confirm(&intent, false));
    assert!(!control_mut(&mut h).confirm(&intent, true));
    assert!(h.drain_action_records().is_empty());
}
#[derive(Debug)]
struct Unknown;
impl Component for Unknown {
    fn interactive(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(available.width, 32.))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Button, "unknown")
    }
    fn accessibility_action(
        &mut self,
        _: Option<usize>,
        _: &AccessibilityAction,
    ) -> ContentResponse {
        ContentResponse {
            activated: true,
            consumed: true,
            ..Default::default()
        }
    }
}
#[test]
fn rectangular_trigger_does_not_grant_curved_or_unknown_owner_admission() {
    let mut d = disclosure(false);
    let mut trigger = d.children_mut().unwrap().remove(0);
    let mut t = text();
    let frame = Rect::new(0., 0., 400., 32.);
    trigger.measure(&mut t, frame.size);
    trigger.arrange(&mut t, frame, viewport());
    t.atlas.begin_frame();
    trigger.paint(
        &mut Scene::new(),
        &mut WithIconRenderer::new(&mut t, &mut Icons),
        Theme::LIGHT.into(),
    );
    assert!(matches!(
        PanelContent::new(trigger),
        Err(CompositionError::UnsupportedCurveChild(TRIGGER))
    ));
    let id = ComponentId(77);
    let mut h = ContentHost::new(Content::new(
        ComponentId(10),
        ScrollContent::new(ScrollArea::default(), Content::new(id, Unknown)),
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, &mut t, viewport());
    assert!(
        !h.accessibility_action(id, None, AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
}
