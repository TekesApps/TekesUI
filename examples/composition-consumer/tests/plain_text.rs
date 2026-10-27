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

use std::rc::Rc;
use tekes_ui_composition_consumer::plain_text::*;
use tekes_ui_controls::{ScrollPolicy, TextSurfacePresentation};
use tekes_ui_scene::Point;
fn render(s: &mut Scenario, width: f32) {
    let frame = Rect::new(0., 0., width, 480.);
    s.host
        .render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
}
#[test]
fn ordinary_surface_buttons_footnote_and_plain_document_share_existing_public_controls() {
    let mut s = Scenario::new().unwrap();
    render(&mut s, 480.);
    let body = s.host.text_surface(BODY).unwrap();
    assert_eq!(body.inner, body.outer);
    assert_eq!(
        body.outer.min_y(),
        s.host.get(BODY_STACK).unwrap().frame().min_y()
    );
    assert_eq!(
        body.scroll.scroll_policy(),
        (ScrollPolicy::Disabled, ScrollPolicy::Disabled)
    );
    assert_eq!(
        s.host.text_surface(FORM).unwrap().presentation,
        TextSurfacePresentation::Field
    );
    assert!(s.host.semantics().iter().any(|n| n.id == FOOTNOTE));
    assert!(
        s.host
            .accessibility_action(BODY, None, AccessibilityAction::SetSelection(8..12))
            .consumed
    );
    assert_eq!(
        s.host.text_surface(BODY).unwrap().input.selected_text(),
        "中文😀"
    );
    assert!(
        s.host
            .accessibility_action(APPEND, None, AccessibilityAction::Press)
            .consumed
    );
    s.host.focus(Some(BODY), true);
    let focused = s.host.focused();
    assert!(s.process_actions());
    assert_eq!(s.host.focused(), focused);
    assert_eq!(s.value.version, 2);
    assert_eq!(
        s.host.text_surface(BODY).unwrap().input.selected_text(),
        "中文😀"
    );
    assert!(!s.process_actions());
    render(&mut s, 480.);
    assert_eq!(s.host.text_surface(BODY).unwrap().input.selection(), 8..12);
    assert_eq!(
        s.host.text_surface(BODY).unwrap().input.text,
        s.value.payload
    );
}
#[test]
fn parent_scroll_does_not_change_body_wrap_and_narrow_resize_remeasures_whole_surface() {
    let mut s = Scenario::new().unwrap();
    render(&mut s, 480.);
    let body = s.host.text_surface(BODY).unwrap();
    let paragraph = body.paragraph.clone().unwrap();
    let height = body.outer.size.height;
    let p = body.visible_frame().unwrap();
    assert!(
        s.host
            .event(&ContentEvent::Scroll {
                position: Point::new(p.min_x() + 2., p.min_y() + 2.),
                delta: Point::new(0., 100.),
                precise: true,
                begin: true,
                end: false,
                momentum: false,
                time: 1.
            })
            .consumed
    );
    render(&mut s, 480.);
    assert!(
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
    assert_eq!(
        s.host.text_surface(BODY).unwrap().scroll.offset,
        Point::new(0., 0.)
    );
    assert!(Rc::ptr_eq(
        &paragraph,
        s.host
            .text_surface(BODY)
            .unwrap()
            .paragraph
            .as_ref()
            .unwrap()
    ));
    render(&mut s, 220.);
    assert!(s.host.text_surface(BODY).unwrap().outer.size.height > height);
    let footer = s.host.get(FOOTNOTE).unwrap().frame();
    let card = s.host.get(PANEL).unwrap().frame();
    assert!(footer.max_y() <= card.max_y());
}

#[cfg(feature = "token-audit")]
#[test]
fn plain_content_token_dependencies_exclude_form_chrome_and_metadata() {
    use tekes_ui_tokens::Palette;
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for state in ["rest", "focus", "selection", "disabled"] {
                let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                    let mut body = TextFieldContent::read_only_content(
                        "Accessible name",
                        "中文😀 e\u{301} text",
                        tekes_ui_tokens::proposal::TextRole::UiBody,
                    );
                    body.surface.input.disabled = state == "disabled";
                    let h = body.measure(&mut Text, Size::new(300., 1000.)).size.height;
                    let f = Rect::new(0., 0., 300., h);
                    body.arrange(&mut Text, f, f);
                    body.set_focus(state == "focus" || state == "selection", true);
                    if state == "selection" {
                        body.surface.input.select_range(0..4);
                    }
                    body.paint(
                        &mut Scene::new(),
                        &mut Text,
                        f,
                        f,
                        Palette::new(theme, brand).expect("registered Gallery brand").into(),
                    );
                });
                for (family, id) in &reads {
                    assert!(
                        ![
                            "transcript-input",
                            "surface-1",
                            "radius-input",
                            "space-tight",
                            "size-button-md"
                        ]
                        .contains(&id.as_str()),
                        "unexpected form dependency {family}/{id} in {state}"
                    );
                }
                cases.push(serde_json::json!({"component":"tui.text","presentation":"content","state":state,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 48);
    if let Ok(path) = std::env::var("TEKESUI_PLAIN_TEXT_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"48 bounded public TextFieldContent Content-presentation cases; no component migration or native claim","cases":cases,"excluded":["compiled TextRole accessors","alias closure","all state combinations","GPU/native/VoiceOver","performance"]})).unwrap()).unwrap();
    }
}

#[test]
fn focusing_and_navigating_document_reveals_caret_through_parent_not_entire_block() {
    let mut s = Scenario::new().unwrap();
    render(&mut s, 480.);
    assert!(
        s.host
            .accessibility_action(BODY, None, AccessibilityAction::SetSelection(8..12))
            .consumed
    );
    assert_eq!(
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y,
        0.,
        "selecting first-line text must not jump to the end of a long block"
    );
    let response = s
        .host
        .event_with_text(
            &ContentEvent::TextCommand("moveToEndOfDocument:".into()),
            &mut Text,
        )
        .unwrap();
    assert!(response.consumed);
    assert!(
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.,
        "keyboard caret reveal belongs to the parent document scroller"
    );
    render(&mut s, 480.);
    let surface = s.host.text_surface(BODY).unwrap();
    let viewport = s.host.get(SCROLLER).unwrap().frame();
    assert!(surface.caret().min_y() >= viewport.min_y());
    assert!(surface.caret().max_y() <= viewport.max_y());
    assert_eq!(surface.scroll.offset, Point::new(0., 0.));
    assert!(
        s.host
            .event_with_text(
                &ContentEvent::TextCommand("moveToBeginningOfDocument:".into()),
                &mut Text
            )
            .unwrap()
            .consumed
    );
    render(&mut s, 480.);
    let surface = s.host.text_surface(BODY).unwrap();
    assert!(surface.caret().min_y() >= viewport.min_y());
    assert_eq!(surface.input.caret, 0);
    let visible = surface.visible_frame().unwrap();
    let before = s
        .host
        .get(SCROLLER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y;
    assert!(
        s.host
            .event(&ContentEvent::Scroll {
                position: Point::new(visible.min_x() + 2., visible.min_y() + 2.),
                delta: Point::new(0., 80.),
                precise: true,
                begin: true,
                end: false,
                momentum: false,
                time: 2.
            })
            .consumed
    );
    render(&mut s, 480.);
    assert!(
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > before,
        "manual document scroll must not reveal the old caret again"
    );
    assert_eq!(s.host.text_surface(BODY).unwrap().input.caret, 0);
}
