use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * 8.).min(w),
            r.style().line_height,
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
        _: Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_composition_consumer::context_menu::*;
fn render(sidebar: &mut Sidebar) {
    sidebar
        .host
        .render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(0., 0., 360., 280.),
            Rect::new(0., 0., 360., 280.),
            Theme::LIGHT.into(),
        )
        .unwrap();
}
#[test]
fn context_menu_second_consumer_original_b_action_preserves_a_draft_selection_and_focus() {
    let mut sidebar = Sidebar::new();
    render(&mut sidebar);
    sidebar.host.focus(Some(DRAFT), true);
    let selection = sidebar
        .host
        .get(DRAFT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .selection();
    let frame = sidebar.host.get(B).unwrap().frame();
    let response = sidebar.host.event(&ContentEvent::Pointer(PointerEvent {
        position: Point::new(frame.min_x() + 8., frame.min_y() + 8.),
        phase: PointerPhase::Down,
        button: Some(PointerButton::Secondary),
        modifiers: Default::default(),
        click_count: 1,
    }));
    assert!(response.consumed);
    sidebar.collect();
    assert_eq!(sidebar.primary_selection, "session-a");
    assert!(sidebar.commands.is_empty());
    render(&mut sidebar);
    sidebar.host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    sidebar.collect();
    assert_eq!(sidebar.commands, [("session-b", vec!["pin".into()])]);
    assert_eq!(sidebar.selections.len(), 1);
    let request = sidebar.selections[0].context_request().unwrap();
    assert_eq!(request.owner(), B);
    assert_eq!(request.source(), ContentActionSource::Pointer);
    assert_eq!(sidebar.selections[0].value_path().unwrap(), ["pin"]);

    assert_eq!(sidebar.host.focused(), Some(DRAFT));
    let input = &sidebar
        .host
        .get(DRAFT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    assert_eq!(input.text, "Draft belongs to A");
    assert_eq!(input.selection(), selection);
}
#[test]
fn context_menu_second_consumer_marked_policy_and_immutable_a_b_actions() {
    let mut sidebar = Sidebar::new();
    render(&mut sidebar);
    sidebar.host.focus(Some(DRAFT), true);
    sidebar.host.event(&ContentEvent::Preedit {
        text: "中文".into(),
        selection: 1..1,
        clauses: Vec::new(),
        replacement: None,
    });
    let before = sidebar
        .host
        .get(DRAFT)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .marked
        .clone();
    assert_eq!(
        sidebar
            .host
            .request_context_menu(sidebar.b, ContentActionSource::Keyboard),
        Err(ContextMenuError::Composing)
    );
    assert_eq!(
        sidebar
            .host
            .get(DRAFT)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .marked,
        before
    );
    sidebar.host.event(&ContentEvent::Commit {
        text: "中文".into(),
        replacement: None,
    });
    for binding in [sidebar.a, sidebar.b] {
        sidebar
            .host
            .request_context_menu(binding, ContentActionSource::Keyboard)
            .unwrap();
        render(&mut sidebar);
        sidebar.host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
    }
    sidebar.collect();
    assert_eq!(
        sidebar.commands,
        [
            ("session-a", vec!["pin".into()]),
            ("session-b", vec!["pin".into()])
        ]
    );
    assert_eq!(sidebar.primary_selection, "session-a");
}
