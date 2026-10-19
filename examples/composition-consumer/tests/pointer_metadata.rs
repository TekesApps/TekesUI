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

use tekes_ui_scene::Point;
const ID: ComponentId = ComponentId(9910);
fn host() -> ContentHost {
    let mut host = ContentHost::new(Content::new(
        ID,
        TextFieldContent::new("Input", "Hello 中文 😀 e\u{301} second word"),
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 500., 100.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    host
}
fn point(h: &ContentHost, index: usize) -> Point {
    let s = h.text_surface(ID).unwrap();
    let caret = s.caret();
    let p = s.caret_doc(index);
    Point::new(s.origin().x + p.min_x() + 0.1, caret.center().y)
}
fn send(
    h: &mut ContentHost,
    phase: PointerPhase,
    p: Point,
    shift: bool,
    count: u32,
    button: PointerButton,
) {
    h.event(&ContentEvent::Pointer(PointerEvent {
        position: p,
        phase,
        button: Some(button),
        modifiers: PointerModifiers {
            shift,
            ..Default::default()
        },
        click_count: count,
    }));
}
#[test]
fn actual_field_receives_shift_click_count_and_grapheme_legal_selection() {
    let mut h = host();
    let a = point(&h, 2);
    let b = point(&h, 9);
    send(
        &mut h,
        PointerPhase::Down,
        a,
        false,
        1,
        PointerButton::Primary,
    );
    send(
        &mut h,
        PointerPhase::Up,
        a,
        false,
        1,
        PointerButton::Primary,
    );
    let anchor = h.text_surface(ID).unwrap().input.anchor;
    send(
        &mut h,
        PointerPhase::Down,
        b,
        true,
        1,
        PointerButton::Primary,
    );
    let s = h.text_surface(ID).unwrap();
    assert_eq!(s.input.anchor, anchor);
    assert!(s.input.caret > anchor);
    send(&mut h, PointerPhase::Up, b, true, 1, PointerButton::Primary);
    for index in [7, 10, 13] {
        let p = point(&h, index);
        send(
            &mut h,
            PointerPhase::Down,
            p,
            false,
            2,
            PointerButton::Primary,
        );
        let s = h.text_surface(ID).unwrap();
        let selection = s.input.selection();
        assert!(!selection.is_empty());
        assert!(s.input.boundaries().contains(&selection.start));
        assert!(s.input.boundaries().contains(&selection.end));
        send(
            &mut h,
            PointerPhase::Up,
            p,
            false,
            2,
            PointerButton::Primary,
        );
    }
    let p = point(&h, 3);
    send(
        &mut h,
        PointerPhase::Down,
        p,
        false,
        3,
        PointerButton::Primary,
    );
    let s = h.text_surface(ID).unwrap();
    assert_eq!(s.input.selection(), 0..s.input.len()); // One paragraph: its selection covers the complete input.
}
#[test]
fn secondary_is_inert_and_cancel_ends_real_field_drag_capture() {
    let mut h = host();
    let p = point(&h, 3);
    send(
        &mut h,
        PointerPhase::Down,
        p,
        false,
        1,
        PointerButton::Primary,
    );
    send(
        &mut h,
        PointerPhase::Up,
        p,
        false,
        1,
        PointerButton::Primary,
    );
    let before = h.text_surface(ID).unwrap().input.selection();
    let focus = h.focused();
    let later = point(&h, 15);
    send(
        &mut h,
        PointerPhase::Down,
        later,
        false,
        1,
        PointerButton::Secondary,
    );
    send(
        &mut h,
        PointerPhase::Up,
        later,
        false,
        1,
        PointerButton::Secondary,
    );
    assert_eq!(h.text_surface(ID).unwrap().input.selection(), before);
    assert_eq!(h.focused(), focus);
    send(
        &mut h,
        PointerPhase::Down,
        p,
        false,
        1,
        PointerButton::Primary,
    );
    let outside = Point::new(1000., 300.);
    send(
        &mut h,
        PointerPhase::Drag,
        outside,
        false,
        0,
        PointerButton::Primary,
    );
    let selected = h.text_surface(ID).unwrap().input.selection();
    assert!(selected.end > selected.start);
    h.event(&ContentEvent::Cancel);
    send(
        &mut h,
        PointerPhase::Drag,
        Point::new(-300., -300.),
        false,
        0,
        PointerButton::Primary,
    );
    assert_eq!(h.text_surface(ID).unwrap().input.selection(), selected);
    assert!(!h.text_surface(ID).unwrap().pointer_state(None).pressed);
}

#[test]
fn double_click_targets_the_visual_grapheme_on_both_sides_of_caret_boundary() {
    let mut h = host();
    for word in ["😀", "e\u{301}", "second"] {
        let s = h.text_surface(ID).unwrap();
        let byte = s.input.text.find(word).unwrap();
        let start = s.input.text[..byte].encode_utf16().count();
        let end = start + word.encode_utf16().count();
        let first_end = s
            .input
            .boundaries()
            .into_iter()
            .find(|i| *i > start)
            .unwrap();
        let rect = s.selection_rects(start..first_end)[0];
        for fraction in [0.25, 0.75] {
            let p = Point::new(rect.min_x() + rect.size.width * fraction, rect.center().y);
            send(
                &mut h,
                PointerPhase::Down,
                p,
                false,
                2,
                PointerButton::Primary,
            );
            assert_eq!(
                h.text_surface(ID).unwrap().input.selection(),
                start..end,
                "{word} at {fraction}"
            );
            send(
                &mut h,
                PointerPhase::Up,
                p,
                false,
                2,
                PointerButton::Primary,
            );
        }
    }
}

#[test]
fn triple_click_selects_the_logical_paragraph_including_its_line_break() {
    let body = "First para 中文😀\r\nSecond e\u{301} paragraph\nTail";
    let mut field = TextFieldContent::new("History", "");
    field.surface = tekes_ui_controls::TextSurface::new(body, true, true);
    let mut h = ContentHost::new(Content::new(ID, field)).unwrap();
    let frame = Rect::new(0., 0., 500., 100.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let p = point(&h, 3);
    send(
        &mut h,
        PointerPhase::Down,
        p,
        false,
        3,
        PointerButton::Primary,
    );
    assert_eq!(
        h.text_surface(ID).unwrap().input.selection(),
        0.."First para 中文😀\r\n".encode_utf16().count()
    );
}

#[test]
fn paragraph_selection_preserves_crlf_and_the_trailing_empty_paragraph() {
    for body in ["A\r\nB\r\n", "A\nB\n", "A\u{2028}B\u{2029}"] {
        let second = body.find('B').unwrap();
        let start = body[..second].encode_utf16().count();
        let len = body.encode_utf16().count();
        let mut surface = tekes_ui_controls::TextSurface::new(body, true, true);
        let frame = Rect::new(0., 0., 500., 200.);
        surface.layout_with(frame, None, 1., 1., |s, w| {
            tekes_ui_controls::TextSurfaceHost::shape_surface(
                &mut Text,
                s,
                proposal::TextRole::UiBody,
                w,
            )
        });
        for (index, expected) in [(start, start..len), (len, len..len)] {
            let caret = surface.caret_doc(index);
            let p = Point::new(
                surface.origin().x + caret.min_x() + 0.1,
                surface.origin().y + caret.center().y,
            );
            assert!(surface.pointer_down(p, false, 3, 24.).is_some());
            assert_eq!(surface.input.selection(), expected, "{body:?} at {index}");
            surface.pointer_up();
        }
    }
}

#[test]
fn long_read_only_document_drag_reuses_scroll_reveal_and_cancel() {
    let body: String = (0..100)
        .map(|i| format!("Row {i} 中文 😀 e\u{301}\n"))
        .collect();
    let mut surface = tekes_ui_controls::TextSurface::new(&body, true, true);
    let frame = Rect::new(0., 0., 220., 80.);
    surface.layout_with(frame, None, 1., 1., |s, w| {
        tekes_ui_controls::TextSurfaceHost::shape_surface(
            &mut Text,
            s,
            proposal::TextRole::UiBody,
            w,
        )
    });
    let p = surface.caret().center();
    assert!(surface.pointer_down(p, false, 1, 24.).is_some());
    assert!(surface.pointer_drag(Point::new(500., 1800.), 24.));
    assert!(surface.scroll.offset.y > 0.);
    let selection = surface.input.selection();
    assert!(surface.input.boundaries().contains(&selection.start));
    assert!(surface.input.boundaries().contains(&selection.end));
    assert!(surface.pointer_state(None).pressed);
    surface.cancel_interaction();
    assert!(!surface.pointer_state(None).pressed);
    assert!(!surface.pointer_drag(Point::new(-500., -500.), 24.));
    assert_eq!(surface.input.selection(), selection);
}
