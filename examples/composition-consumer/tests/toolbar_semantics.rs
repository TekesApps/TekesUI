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

use tekes_ui_composition_consumer::toolbar_semantics::*;
use tekes_ui_controls::{ControlKey, ToolbarControl};
fn paint(host: &mut ContentHost, width: f32) {
    let frame = Rect::new(0., 0., width, 400.);
    host.get_mut(TOOLBAR)
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_viewport(frame);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn parts(host: &ContentHost) -> Vec<SemanticPart> {
    host.semantics()
        .into_iter()
        .find(|n| n.id == TOOLBAR)
        .unwrap()
        .parts
}
#[test]
fn nested_toolbar_measures_available_width_and_preserves_original_editor() {
    for width in [360., 704., 1400.] {
        let mut host = build().unwrap();
        paint(&mut host, width);
        assert!(host.get(TOOLBAR).unwrap().frame().size.width <= width);
        let before = host
            .get(EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text
            .clone();
        if width < 1000. {
            let more = parts(&host)
                .into_iter()
                .find(|p| p.semantics.role == ContentRole::MenuButton)
                .expect("nested finite allocation must produce overflow");
            let r =
                host.accessibility_action(TOOLBAR, Some(more.index), AccessibilityAction::Press);
            assert!(r.consumed && !r.activated);
            assert!(host.drain_action_records().is_empty());
            paint(&mut host, width);
            let target = parts(&host)
                .into_iter()
                .find(|p| p.semantics.name == "Last action")
                .unwrap();
            assert_eq!(target.semantics.role, ContentRole::MenuItem);
            assert!(
                host.accessibility_action(TOOLBAR, Some(target.index), AccessibilityAction::Focus)
                    .consumed
            );
            assert_eq!(host.focused(), Some(TOOLBAR));
            assert!(
                host.accessibility_action(TOOLBAR, Some(target.index), AccessibilityAction::Press)
                    .activated
            );
            assert_eq!(
                host.get_mut(TOOLBAR)
                    .unwrap()
                    .component_mut::<ToolbarControl>()
                    .unwrap()
                    .take_action(),
                Some(60)
            );
            assert_eq!(host.drain_action_records().len(), 1);
        } else {
            assert!(
                !parts(&host)
                    .iter()
                    .any(|p| p.semantics.role == ContentRole::MenuButton)
            );
            assert_eq!(parts(&host).len(), 6);
        }
        assert_eq!(
            host.get(EDITOR)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text,
            before
        );
        host.focus(Some(EDITOR), true);
        host.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse: true },
            down: true,
        });
        assert_eq!(host.focused(), Some(TOOLBAR));
    }
}
