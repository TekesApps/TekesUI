use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, proposal};
#[derive(Debug, Default)]
struct Recorder {
    seen: Vec<PointerEvent>,
}
impl Component for Recorder {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
    }
    fn interactive(&self) -> bool {
        true
    }
    fn pointer_event(&mut self, e: &PointerEvent) -> ContentResponse {
        self.seen.push(*e);
        ContentResponse {
            consumed: true,
            changed: true,
            capture_pointer: e.phase == PointerPhase::Down,
            activated: false,
            text_key: None,
        }
    }
}
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(40., 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: tekes_ui_controls::ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
fn render(host: &mut ContentHost) {
    let rect = Rect::new(0., 0., 200., 80.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        rect,
        rect,
        tekes_ui_tokens::Theme::LIGHT.into(),
    )
    .unwrap();
}
fn pointer(phase: PointerPhase, button: PointerButton) -> PointerEvent {
    PointerEvent {
        position: Point::new(20., 20.),
        phase,
        button: Some(button),
        modifiers: PointerModifiers {
            shift: true,
            control: true,
            alt: true,
            command: true,
            caps_lock: true,
            function: true,
        },
        click_count: 3,
    }
}
#[test]
fn metadata_is_lossless_and_secondary_release_does_not_end_primary_capture() {
    let id = ComponentId(9901);
    let mut host = ContentHost::new(Content::new(id, Recorder::default())).unwrap();
    render(&mut host);
    let down = pointer(PointerPhase::Down, PointerButton::Primary);
    host.event(&ContentEvent::Pointer(down));
    let secondary = pointer(PointerPhase::Up, PointerButton::Secondary);
    host.event(&ContentEvent::Pointer(secondary));
    let mut drag = pointer(PointerPhase::Drag, PointerButton::Primary);
    drag.position = Point::new(500., 500.);
    host.event(&ContentEvent::Pointer(drag));
    let seen = &host.get(id).unwrap().component::<Recorder>().unwrap().seen;
    assert_eq!(seen, &vec![down, secondary, drag]);
    host.event(&ContentEvent::Cancel);
    host.event(&ContentEvent::Pointer(drag));
    // Hover broadcast is allowed after cancel; it must not restore capture.
    let up = PointerEvent {
        phase: PointerPhase::Up,
        ..drag
    };
    host.event(&ContentEvent::Pointer(up));
    assert!(
        !host
            .get(id)
            .unwrap()
            .component::<Recorder>()
            .unwrap()
            .seen
            .contains(&up)
    );
}
#[test]
fn ordinary_button_accepts_primary_metadata_and_legacy_but_ignores_secondary() {
    let id = ComponentId(9902);
    let mut host = ContentHost::new(Content::new(id, Button::new("Send"))).unwrap();
    render(&mut host);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        host.event(&ContentEvent::Pointer(pointer(
            phase,
            PointerButton::Secondary,
        )));
    }
    assert_eq!(
        host.get(id)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        0
    );
    assert_eq!(host.focused(), None);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        host.event(&ContentEvent::Pointer(pointer(
            phase,
            PointerButton::Primary,
        )));
    }
    host.event(&ContentEvent::PointerDown(Point::new(20., 20.)));
    host.event(&ContentEvent::PointerUp(Point::new(20., 20.)));
    assert_eq!(
        host.get(id)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        2
    );
}

#[derive(Debug)]
struct BubbleProbe {
    children: Vec<Content>,
    handled: bool,
    changed: bool,
    seen: Vec<PointerEvent>,
}
impl Component for BubbleProbe {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            child.arrange(text, frame, clip);
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        self.seen.push(*pointer);
        ContentResponse {
            consumed: self.handled,
            changed: self.changed,
            ..Default::default()
        }
    }
}
fn probe(id: u64, children: Vec<Content>, handled: bool, changed: bool, bubble: bool) -> Content {
    Content::new(
        ComponentId(id),
        BubbleProbe {
            children,
            handled,
            changed,
            seen: vec![],
        },
    )
    .bubbling_handled_events(bubble)
}
fn seen(host: &ContentHost, id: u64) -> &[PointerEvent] {
    &host
        .get(ComponentId(id))
        .unwrap()
        .component::<BubbleProbe>()
        .unwrap()
        .seen
}
#[test]
fn handled_stops_by_default_but_explicit_bubbling_keeps_handled_result() {
    for bubble in [false, true] {
        let mut host = ContentHost::new(probe(
            1,
            vec![probe(2, vec![], true, true, bubble)],
            false,
            false,
            false,
        ))
        .unwrap();
        render(&mut host);
        let event = pointer(PointerPhase::Down, PointerButton::Secondary);
        let result = host.event(&ContentEvent::Pointer(event));
        assert!(result.consumed && result.changed);
        assert_eq!(seen(&host, 2), &[event]);
        assert_eq!(seen(&host, 1).len(), usize::from(bubble));
        if bubble {
            assert_eq!(seen(&host, 1), &[event]);
        }
        assert_eq!(host.event_bubbled(), bubble);
    }
}
#[test]
fn unhandled_target_bubbles_only_to_ancestors_not_siblings_behind_it() {
    let mut host = ContentHost::new(probe(
        1,
        vec![
            probe(2, vec![], true, false, false),
            probe(3, vec![], false, true, false),
        ],
        false,
        false,
        false,
    ))
    .unwrap();
    render(&mut host);
    let event = pointer(PointerPhase::Down, PointerButton::Primary);
    let result = host.event(&ContentEvent::Pointer(event));
    assert!(!result.consumed && result.changed);
    assert!(host.event_bubbled());
    assert!(seen(&host, 2).is_empty());
    assert_eq!(seen(&host, 3), &[event]);
    assert_eq!(seen(&host, 1), &[event]);
}
#[test]
fn ancestor_can_stop_a_handled_bubbling_event_without_replaying_child() {
    let mut host = ContentHost::new(probe(
        1,
        vec![probe(2, vec![], true, true, true)],
        true,
        false,
        false,
    ))
    .unwrap();
    render(&mut host);
    let event = pointer(PointerPhase::Down, PointerButton::Secondary);
    assert!(host.event(&ContentEvent::Pointer(event)).consumed);
    assert!(!host.event_bubbled());
    assert_eq!(seen(&host, 2), &[event]);
    assert_eq!(seen(&host, 1), &[event]);
}
#[test]
fn hover_redraw_does_not_consume_unhandled_event() {
    let mut host = ContentHost::new(probe(1, vec![], false, true, false)).unwrap();
    render(&mut host);
    let result = host.event(&ContentEvent::Pointer(pointer(
        PointerPhase::Move,
        PointerButton::Primary,
    )));
    assert!(result.changed);
    assert!(!result.consumed);
    assert!(host.event_bubbled());
}
#[test]
fn bubbling_button_preserves_capture_and_records_activation_once() {
    let button =
        Content::new(ComponentId(2), Button::new("Send")).bubbling_handled_events(true);
    let mut host = ContentHost::new(probe(1, vec![button], false, false, false)).unwrap();
    host.set_action_events(true);
    render(&mut host);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        assert!(
            host.event(&ContentEvent::Pointer(pointer(
                phase,
                PointerButton::Primary
            )))
            .consumed
        );
        assert!(host.event_bubbled());
    }
    assert_eq!(host.drain_actions().len(), 1);
    assert!(host.drain_actions().is_empty());
    assert_eq!(seen(&host, 1).len(), 2);
}

#[derive(Debug, Default)]
struct DragBubbles(Recorder);
impl Component for DragBubbles {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.0.measure(text, available)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        self.0.pointer_event(pointer)
    }
    fn bubble_handled_event(&self, event: &ContentEvent) -> bool {
        matches!(event, ContentEvent::Pointer(pointer) if pointer.phase == PointerPhase::Drag)
    }
}
#[test]
fn per_event_policy_bubbles_captured_drag_but_does_not_surrender_capture() {
    let child = Content::new(ComponentId(2), DragBubbles::default());
    let mut host = ContentHost::new(probe(1, vec![child], false, false, false)).unwrap();
    render(&mut host);
    host.event(&ContentEvent::Pointer(pointer(
        PointerPhase::Down,
        PointerButton::Primary,
    )));
    assert!(!host.event_bubbled());
    let mut drag = pointer(PointerPhase::Drag, PointerButton::Primary);
    drag.position = Point::new(900., 900.);
    assert!(host.event(&ContentEvent::Pointer(drag)).consumed);
    assert!(host.event_bubbled());
    assert_eq!(seen(&host, 1), &[drag]);
    let up = PointerEvent {
        phase: PointerPhase::Up,
        ..drag
    };
    assert!(host.event(&ContentEvent::Pointer(up)).consumed);
    assert!(!host.event_bubbled());
    assert_eq!(
        host.get(ComponentId(2))
            .unwrap()
            .component::<DragBubbles>()
            .unwrap()
            .0
            .seen
            .len(),
        3
    );
}
#[test]
fn opt_in_cannot_bubble_through_modal_boundary() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        DialogContent::new(
            "Dialog",
            Content::new(
                ComponentId(2),
                TextContent {
                    text: "Title".into(),
                    role: proposal::TextRole::UiHeading,
                },
            ),
            Content::new(ComponentId(3), Button::new("Action"))
                .bubbling_handled_events(true),
            Content::new(ComponentId(4), Button::new("Close")),
        ),
    ))
    .unwrap();
    host.set_action_events(true);
    host.show_dialog(ComponentId(1)).unwrap();
    render(&mut host);
    host.focus(Some(ComponentId(3)), true);
    assert!(
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Enter,
            down: true
        })
        .consumed
    );
    assert!(!host.event_bubbled());
    assert_eq!(host.drain_actions().len(), 1);
}

#[test]
fn explicit_false_overrides_component_opt_in_for_handled_events_only() {
    let child = Content::new(ComponentId(2), DragBubbles::default()).bubbling_handled_events(false);
    let mut host = ContentHost::new(probe(1, vec![child], false, false, false)).unwrap();
    render(&mut host);
    host.event(&ContentEvent::Pointer(pointer(
        PointerPhase::Down,
        PointerButton::Primary,
    )));
    host.event(&ContentEvent::Pointer(pointer(
        PointerPhase::Drag,
        PointerButton::Primary,
    )));
    assert!(!host.event_bubbled());
    assert!(seen(&host, 1).is_empty());
}
