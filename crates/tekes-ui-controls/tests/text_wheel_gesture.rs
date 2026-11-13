use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use ContentWheelPhase as Phase;
use tekes_ui_controls::extended::{Axis, Stack};
use tekes_ui_controls::{ScrollArea, ScrollPolicy, TextSurface, layout::MeasuredChild};
use tekes_ui_scene::Point;
const ROOT: ComponentId = ComponentId(1);
const BODY: ComponentId = ComponentId(10);
const AREA: ComponentId = ComponentId(11);
const SECOND: ComponentId = ComponentId(12);
const BUTTON: ComponentId = ComponentId(14);

#[derive(Debug)]
struct Tail;
impl Component for Tail {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(available.width, 500.))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "tail")
    }
}
fn stack(children: Vec<Content>) -> StackContent {
    StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 0.,
        },
        children,
        flexible_child: None,
    }
}
fn field() -> TextFieldContent {
    let value = "A line of editable text.\n".repeat(80);
    let mut field = TextFieldContent::new("Area", &value);
    field.show_metadata = false;
    field.viewport = TextViewport::Fixed(90.);
    field.surface = TextSurface::new(&value, true, false);
    field.surface.scroll.set_reduced_motion(true);
    field
}
fn fixture(bubble: bool) -> ContentHost {
    let children = vec![
        Content::new(AREA, field()).bubbling_handled_events(bubble),
        Content::new(BUTTON, Button::new("Sibling")),
        Content::new(SECOND, stack(vec![Content::new(ComponentId(13), Tail)])),
    ];
    let mut scroll = ScrollArea::default();
    scroll.set_reduced_motion(true);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        ScrollContent::new(scroll, Content::new(BODY, stack(children))),
    ))
    .unwrap();
    paint(&mut host, Theme::LIGHT, Rect::new(0., 0., 300., 220.));
    assert!(host.focus(Some(AREA), true));
    host
}
fn paint(host: &mut ContentHost, theme: Theme, frame: Rect) {
    host.render(&mut Scene::new(), &mut Text, frame, frame, theme.into())
        .unwrap();
}
fn surface(host: &ContentHost) -> &TextSurface {
    host.text_surface(AREA).unwrap()
}
fn offset(host: &ContentHost) -> f32 {
    surface(host).scroll.offset.y
}
fn parent_offset(host: &ContentHost) -> f32 {
    host.get(ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y
}
fn position(host: &ContentHost) -> Point {
    surface(host).inner.center()
}
fn outside(host: &ContentHost) -> Point {
    let frame = surface(host).outer;
    Point::new(frame.center().x, frame.max_y() + 10.)
}
fn send(
    host: &mut ContentHost,
    position: Point,
    delta: f32,
    phase: Phase,
    momentum: Phase,
    precise: bool,
) -> ContentResponse {
    host.wheel_event_with_text(
        &ContentWheelEvent {
            position,
            delta: Point::new(0., delta),
            precise,
            phase,
            momentum_phase: momentum,
            time: 1.,
        },
        &mut Text,
    )
    .unwrap()
}
fn begin(host: &mut ContentHost) {
    let point = position(host);
    assert!(send(host, point, 0., Phase::Began, Phase::None, true).consumed);
}

#[test]
fn text_wheel_zero_begin_continues_outside_through_finger_end_and_momentum_end() {
    let mut host = fixture(false);
    begin(&mut host);
    assert_eq!(offset(&host), 0.);
    let point = outside(&host);
    assert!(send(&mut host, point, 20., Phase::Changed, Phase::None, true).consumed);
    let moved = offset(&host);
    assert!(moved > 0.);
    assert_eq!(parent_offset(&host), 0.);
    send(&mut host, point, 0., Phase::Ended, Phase::None, true);
    send(&mut host, point, 15., Phase::None, Phase::Changed, true);
    assert!(
        offset(&host) > moved,
        "finger Ended does not drop native momentum"
    );
    send(&mut host, point, 0., Phase::None, Phase::Ended, true);
    let ended = offset(&host);
    send(&mut host, point, 20., Phase::None, Phase::Changed, true);
    assert_eq!(
        offset(&host),
        ended,
        "late momentum cannot revive a retired lease"
    );
    assert_eq!(parent_offset(&host), 0.);
    assert!(!host.event_bubbled());
    send(&mut host, point, 20., Phase::None, Phase::None, true);
    assert_eq!(offset(&host), ended);
    assert!(
        parent_offset(&host) > 0.,
        "unphased precise input uses ordinary hit routing"
    );
}

#[test]
fn text_wheel_cancelled_tail_is_rejected_and_new_begin_can_acquire_again() {
    let mut host = fixture(false);
    begin(&mut host);
    let point = outside(&host);
    send(&mut host, point, 20., Phase::Changed, Phase::None, true);
    send(&mut host, point, 0., Phase::Cancelled, Phase::None, true);
    let before = offset(&host);
    let inside = position(&host);
    send(&mut host, inside, 20., Phase::Changed, Phase::None, true);
    assert_eq!(
        offset(&host),
        before,
        "cancelled tail cannot re-hit the same editor"
    );
    begin(&mut host);
    send(&mut host, point, 20., Phase::Changed, Phase::None, true);
    assert!(offset(&host) > before);
}

#[test]
fn text_wheel_nonprecise_edge_disabled_axes_and_button_keep_parent_bubbling() {
    let mut host = fixture(false);
    let field = host
        .get_mut(AREA)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface.scroll.scroll_by(0., f32::MAX);
    let inside = position(&host);
    send(&mut host, inside, 1., Phase::None, Phase::None, false);
    assert!(
        parent_offset(&host) > 0.,
        "ordinary wheel at child bottom reaches parent"
    );

    let mut host = fixture(false);
    host.get_mut(AREA)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .scroll
        .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Disabled);
    let inside = position(&host);
    send(&mut host, inside, 20., Phase::Began, Phase::None, true);
    assert!(
        parent_offset(&host) > 0.,
        "non-scrolling Field does not acquire a lease"
    );
    assert_eq!(offset(&host), 0.);

    let mut host = fixture(false);
    let button = host.get(BUTTON).unwrap().frame().center();
    send(&mut host, button, 4., Phase::Began, Phase::None, true);
    let parent = parent_offset(&host);
    assert!(parent > 0.);
    let inside = position(&host);
    send(&mut host, inside, 20., Phase::Changed, Phase::None, true);
    assert!(
        offset(&host) > 0.,
        "ScrollContent did not capture the button's gesture"
    );
    assert_eq!(parent_offset(&host), parent);
}

#[test]
fn text_wheel_explicit_handled_bubbling_keeps_the_original_ancestor_path() {
    let mut host = fixture(true);
    begin(&mut host);
    let point = outside(&host);
    send(&mut host, point, 2., Phase::Changed, Phase::None, true);
    assert!(offset(&host) > 0.);
    assert!(
        parent_offset(&host) > 0.,
        "captured delivery still honors handled-event bubbling"
    );
}

#[test]
fn text_wheel_mount_blur_disabled_inactive_cancel_and_reparent_revoke_tail() {
    for change in [
        "remount", "blur", "disabled", "inactive", "cancel", "subtree", "reparent",
    ] {
        let mut host = fixture(false);
        begin(&mut host);
        match change {
            "remount" => {
                let removed = host.remove(AREA).unwrap();
                host.insert(BODY, removed).unwrap();
                host.reorder(BODY, &[AREA, BUTTON, SECOND]).unwrap();
            }
            "blur" => {
                host.focus(None, false);
            }
            "disabled" => {
                host.get_mut(AREA)
                    .unwrap()
                    .component_mut::<TextFieldContent>()
                    .unwrap()
                    .surface
                    .input
                    .disabled = true;
            }
            "inactive" => {
                host.set_window_active(false);
                host.set_window_active(true);
            }
            "cancel" => {
                host.cancel();
            }
            "subtree" => {
                host.cancel_subtree(AREA);
            }
            "reparent" => {
                host.reparent(AREA, SECOND).unwrap();
            }
            _ => unreachable!(),
        }
        paint(&mut host, Theme::LIGHT, Rect::new(0., 0., 300., 220.));
        let before = offset(&host);
        let parent = parent_offset(&host);
        let inside = position(&host);
        send(&mut host, inside, 20., Phase::Changed, Phase::None, true);
        assert_eq!(
            offset(&host),
            before,
            "{change} must retire the old wheel authority"
        );
        assert_eq!(
            parent_offset(&host),
            parent,
            "{change} tail must not retarget ancestor"
        );
    }
}

#[test]
fn text_wheel_theme_and_normal_relayout_keep_the_original_mount_lease() {
    let mut host = fixture(false);
    begin(&mut host);
    let mount = host.get(AREA).unwrap().mount_identity();
    paint(&mut host, Theme::DARK, Rect::new(20., 30., 320., 240.));
    assert_eq!(host.get(AREA).unwrap().mount_identity(), mount);
    let point = outside(&host);
    send(&mut host, point, 20., Phase::Changed, Phase::None, true);
    assert!(offset(&host) > 0.);
    assert_eq!(parent_offset(&host), 0.);
}

#[test]
fn text_wheel_end_does_not_cancel_pointer_capture_focus_or_marked_text() {
    let mut host = fixture(false);
    let point = position(&host);
    host.event_with_text(&ContentEvent::PointerDown(point), &mut Text)
        .unwrap();
    let capture = host
        .pointer_capture()
        .expect("original pointer capture lease");
    assert_eq!(capture.0, AREA);
    host.event_with_text(
        &ContentEvent::Preedit {
            text: "拼".into(),
            selection: 0..1,
            clauses: vec![],
            replacement: None,
        },
        &mut Text,
    )
    .unwrap();
    let text = surface(&host).input.text.clone();
    let marked = surface(&host).input.marked.clone();
    let selection = surface(&host).input.selection();
    assert!(marked.is_some());
    begin(&mut host);
    let point = outside(&host);
    send(&mut host, point, 0., Phase::Ended, Phase::None, true);
    send(&mut host, point, 0., Phase::None, Phase::Ended, true);
    assert_eq!(
        host.pointer_capture(),
        Some(capture),
        "wheel end preserves the exact pointer owner and capture epoch"
    );
    assert_eq!(host.focused(), Some(AREA));
    assert_eq!(surface(&host).input.text, text);
    assert_eq!(surface(&host).input.marked, marked);
    assert_eq!(surface(&host).input.selection(), selection);
    host.event_with_text(&ContentEvent::PointerUp(point), &mut Text)
        .unwrap();
    assert_eq!(host.pointer_capture(), None);
}
