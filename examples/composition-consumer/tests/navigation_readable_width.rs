//! Real CoreText, unchanged public fixtures: bounded navigation reading regressions.
use std::rc::Rc;
use tekes_ui_composition_consumer::navigation_content::{self, BREADCRUMB, FIRST_PANEL, TABS};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Draw {
    text: String,
    frame: Rect,
    lines: Vec<String>,
}
struct Text {
    shaper: CoreTextShaper,
    draws: Vec<Draw>,
}
impl Text {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            draws: vec![],
        }
    }
}
impl ButtonTextRenderer for Text {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        role.style().size * 1.5
    }
    fn backing_scale(&self) -> f32 {
        2.
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.shape_surface(text, role, Some(width));
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        let p = self.shape_surface(text, role, Some(frame.size.width));
        self.draws.push(Draw {
            text: text.into(),
            frame,
            lines: p
                .lines
                .iter()
                .map(|line| text[line.bytes.clone()].to_owned())
                .collect(),
        });
    }
}
impl TextSurfaceHost for Text {
    fn type_scale(&self) -> f32 {
        1.5
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            text,
            &TextStyle::system(role.style().size * 1.5).with_weight(role.style().weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn render(host: &mut ContentHost, width: f32) -> Text {
    let mut text = Text::new();
    host.render(
        &mut Scene::new(),
        &mut text,
        Rect::new(12., 12., width, 396.),
        Rect::new(0., 0., width + 24., 420.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    text
}
fn whole(text: &Text, value: &str) {
    let draw = text.draws.iter().rfind(|draw| draw.text == value).unwrap();
    assert_eq!(
        draw.lines,
        vec![value],
        "ordinary word {value:?} fragmented inside {:?}",
        draw.frame
    );
}
#[test]
fn tabs_readable_words_share_existing_240pt() {
    let mut host = navigation_content::tabs().unwrap();
    let text = render(&mut host, 240.);
    for word in ["Overview", "Welcome", "Settings"] {
        whole(&text, word);
    }
    let tabs = host
        .semantics()
        .into_iter()
        .find(|node| node.id == TABS)
        .unwrap();
    assert!(tabs.parts[0].frame.size.width > tabs.parts[1].frame.size.width);
    assert!(tabs.parts[0].frame.max_x() < tabs.parts[1].frame.min_x());
    assert!(tabs.parts[1].frame.max_x() <= 252.);
    let overview = text
        .draws
        .iter()
        .find(|draw| draw.text == "Overview")
        .unwrap();
    let welcome = text
        .draws
        .iter()
        .find(|draw| draw.text == "Welcome")
        .unwrap();
    assert_eq!(
        welcome.frame.min_y() - overview.frame.max_y(),
        2.,
        "original label Stack gap changed"
    );
    assert!(welcome.frame.max_y() <= host.get(FIRST_PANEL).unwrap().frame().min_y());
    let hit = tabs.parts[1].frame.center();
    assert!(host.event(&ContentEvent::PointerDown(hit)).capture_pointer);
    assert!(host.event(&ContentEvent::PointerUp(hit)).activated);
    let action = host.drain_action_records();
    assert_eq!(action.len(), 1);
    assert!(matches!(
        action[0].payload,
        Some(ContentActionPayload::TabSelection { index: 1, .. })
    ));
}
#[test]
fn breadcrumb_240_wraps_at_path_boundaries_before_breaking_ordinary_words() {
    let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"]).unwrap();
    let text = render(&mut host, 240.);
    for word in ["Home", "Library", "Current"] {
        whole(&text, word);
    }
    let node = host
        .semantics()
        .into_iter()
        .find(|node| node.id == BREADCRUMB)
        .unwrap();
    assert_eq!(node.parts[0].frame.min_y(), node.parts[1].frame.min_y());
    assert!(node.parts[2].frame.min_y() > node.parts[1].frame.max_y());
    let slashes: Vec<_> = text.draws.iter().filter(|draw| draw.text == "/").collect();
    assert_eq!(slashes.len(), 2);
    for slash in slashes {
        assert!(
            node.parts
                .iter()
                .all(|part| slash.frame.intersection(&part.frame).is_none())
        );
    }
    let hit = node.parts[1].frame.center();
    assert!(host.event(&ContentEvent::PointerDown(hit)).capture_pointer);
    assert!(host.event(&ContentEvent::PointerUp(hit)).activated);
    let actions = host.drain_action_records();
    assert!(matches!(
        &actions[0].payload,
        Some(ContentActionPayload::BreadcrumbNavigation { index: 1, .. })
    ));
}

#[test]
fn tabs_preserve_equal_cap_and_infeasible_wrap() {
    for width in [320., 400., 160.] {
        let mut host = navigation_content::tabs().unwrap();
        let text = render(&mut host, width);
        let node = host
            .semantics()
            .into_iter()
            .find(|node| node.id == TABS)
            .unwrap();
        assert_eq!(
            node.parts[0].frame.size.width,
            node.parts[1].frame.size.width
        );
        assert!(node.parts.iter().all(|part| part.frame.size.width <= 160.));
        if width >= 320. {
            for word in ["Overview", "Welcome", "Settings"] {
                whole(&text, word);
            }
        } else {
            assert!(
                text.draws
                    .iter()
                    .any(|draw| draw.text == "Overview" && draw.lines.len() > 1)
            );
        }
    }
}
#[test]
fn breadcrumb_wide_row_and_genuinely_oversized_word_keep_original_fallback() {
    let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"]).unwrap();
    let text = render(&mut host, 400.);
    let node = host
        .semantics()
        .into_iter()
        .find(|node| node.id == BREADCRUMB)
        .unwrap();
    assert!(
        node.parts
            .iter()
            .all(|part| part.frame.min_y() == node.parts[0].frame.min_y())
    );
    assert_eq!(node.parts.last().unwrap().frame.max_x(), 412.);
    for word in ["Home", "Library", "Current"] {
        whole(&text, word);
    }
    let long = "Supercalifragilisticexpialidocious";
    let mut host = navigation_content::breadcrumb(&[long]).unwrap();
    let text = render(&mut host, 240.);
    let draw = text.draws.iter().find(|draw| draw.text == long).unwrap();
    assert!(draw.lines.len() > 1);
    assert!(draw.frame.size.width <= 240.);
}
