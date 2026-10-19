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
    let mut host = ContentHost::new(Content::new(id, ButtonContent::new("Send"))).unwrap();
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
            .component::<ButtonContent>()
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
            .component::<ButtonContent>()
            .unwrap()
            .activations,
        2
    );
}
