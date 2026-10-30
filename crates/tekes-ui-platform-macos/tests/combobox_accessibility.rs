#![cfg(all(target_os = "macos", feature = "content-accessibility"))]

use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::{ComboBoxContent, ComponentId, Content, ContentEvent, ContentHost},
};
use tekes_ui_platform_macos::{AccessibilityAction, AccessibilityRole, content_accessibility::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

const COMBO: ComponentId = ComponentId(9400);

struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (value.chars().count() as f32 * role.style().size).min(width),
            role.style().line_height,
        )
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
impl TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
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

fn setup() -> ContentHost {
    let combo = ComboBoxContent::new("Language", vec!["A".into(), "B".into()]);
    ContentHost::new(Content::new(COMBO, combo)).unwrap()
}

fn render(host: &mut ContentHost) {
    let frame = Rect::new(30., 40., 320., 240.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}

#[test]
fn original_combo_owner_publishes_and_routes_its_single_editable_text_state() {
    let mut host = setup();
    render(&mut host);
    assert!(host.focus(Some(COMBO), true));
    let key = content_key(&host, COMBO, None).unwrap();
    assert!(content_action(&mut host, key, AccessibilityAction::SetValue("A😀B".into()),).changed);
    assert!(content_action(&mut host, key, AccessibilityAction::SetSelection(1..3),).changed);
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "漢".into(),
            selection: 0..1,
            clauses: vec![0..1],
            replacement: Some(1..3),
        })
        .changed
    );
    render(&mut host);

    let snapshot = ContentAccessibilitySnapshot::from_host(&host).unwrap();
    let editor = snapshot
        .nodes()
        .iter()
        .find(|node| node.key == key)
        .unwrap();
    assert_eq!(editor.role, AccessibilityRole::ComboBox);
    assert_eq!(editor.value.as_deref(), Some("A漢B"));
    assert_eq!(editor.text_selection, Some(1..2));
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.key.component == key.component && node.value.is_some())
            .count(),
        1,
        "the original editor has one semantic text owner"
    );
    assert_eq!(snapshot.text_layouts().len(), 1);
    let layout = snapshot.text_layouts().get(&key).unwrap();
    assert_eq!(layout.state.text, "A漢B");
    assert_eq!(layout.state.selection, 1..2);
    assert_eq!(layout.state.marked, Some(1..2));
    assert!(layout.is_valid());
}

#[path = "../../tekes-ui-module-runtime/src/combo_bridge_fixture.rs"]
mod combo_fixture;

#[test]
fn multiline_original_editor_is_rejected_without_relabeling_the_combo_role() {
    use tekes_ui_controls::composition::TextFieldContent;
    let mut field = TextFieldContent::new("Combo", "A😀B");
    field.surface.input.multiline = true;
    let host =
        ContentHost::new(Content::new(COMBO, combo_fixture::ComboOwner::new(field))).unwrap();
    assert!(content_nodes(&host).is_empty());
}
