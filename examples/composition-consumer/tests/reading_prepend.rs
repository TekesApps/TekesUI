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
use tekes_ui_composition_consumer::reading_prepend::{EARLIER, EDITOR, ReadingFixture};
fn paint(f: &mut ReadingFixture) {
    f.paint(
        &mut Text,
        &mut Scene::new(),
        Rect::new(20., 20., 540., 600.),
        Rect::new(0., 0., 600., 660.),
        Theme::LIGHT.into(),
    );
}
#[test]
fn independent_composed_editor_focus_draft_and_undo_survive_history_prepend() {
    let mut f = ReadingFixture::new();
    paint(&mut f);
    f.controls.focus(Some(EDITOR), true);
    f.controls.event(&ContentEvent::Commit {
        text: "编辑".into(),
        replacement: None,
    });
    let before = f.controls.text_surface(EDITOR).unwrap().input.text.clone();
    let anchor = f.surface.reflow_anchor.unwrap();
    let y = f.surface.caret_doc(anchor.utf16).min_y() - f.surface.scroll.offset.y;
    let old_len = f.surface.input.len();
    let selection = anchor.utf16..anchor.utf16 + 2;
    f.surface.input.select_range(selection);
    let selected = f.surface.input.selected_text();
    let r = f.prepend().unwrap();
    paint(&mut f);
    assert_eq!(f.controls.focused(), Some(EDITOR));
    assert_eq!(f.controls.text_surface(EDITOR).unwrap().input.text, before);
    assert_eq!(f.surface.input.selected_text(), selected);
    assert_eq!(
        r.selection,
        tekes_ui_controls::ReadingSelectionUpdate::Preserved
    );
    let delta = f.surface.input.len() - old_len;
    let after = f.surface.caret_doc(anchor.utf16 + delta).min_y() - f.surface.scroll.offset.y;
    assert!((after - y).abs() <= 1.);
    f.controls.event(&ContentEvent::TextCommand("undo:".into()));
    assert_ne!(f.controls.text_surface(EDITOR).unwrap().input.text, before);
}
#[test]
fn original_button_activation_drives_public_prepend_once() {
    let mut f = ReadingFixture::new();
    paint(&mut f);
    f.controls.focus(Some(EARLIER), true);
    assert!(
        f.controls
            .event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert!(f.process_actions().unwrap());
    paint(&mut f);
    assert_eq!(f.pages, 1);
    assert!(!f.process_actions().unwrap());
    assert_eq!(f.pages, 1);
}

#[test]
fn readonly_textfield_adapter_reuses_versioned_anchor_without_taking_sibling_focus() {
    use tekes_ui_controls::{
        ReadingController, TextRangeMapping, TextSurface,
        extended::{Axis, Stack},
    };
    let old: String = (0..100)
        .map(|i| format!("Row {i}: 中文 e\u{301} 😀 retained content\n"))
        .collect();
    let mut field = TextFieldContent::new("History", "");
    field.surface = TextSurface::new("", true, true);
    field.reading = Some(ReadingController::default());
    let v = field
        .reading
        .as_mut()
        .unwrap()
        .set_document(&mut field.surface, "history", 1, &old)
        .unwrap();
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), field),
                Content::new(ComponentId(3), TextFieldContent::new("Draft", "keep me")),
            ],
        },
    ))
    .unwrap();
    let render = |host: &mut ContentHost| {
        host.render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(10., 10., 540., 600.),
            Rect::new(0., 0., 580., 640.),
            Theme::LIGHT.into(),
        )
        .unwrap();
    };
    render(&mut host);
    host.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .scroll
        .scroll_by(0., 713.25);
    render(&mut host);
    host.focus(Some(ComponentId(3)), true);
    let field = host
        .get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    let a = field.surface.reflow_anchor.unwrap();
    let before = field.surface.caret_doc(a.utf16).min_y() - field.surface.scroll.offset.y;
    let prefix = "Earlier 中文😀\n".repeat(12);
    let delta = prefix.encode_utf16().count();
    let len = old.encode_utf16().count();
    field
        .reading
        .as_mut()
        .unwrap()
        .update_document(
            &mut field.surface,
            &v,
            2,
            &format!("{prefix}{old}"),
            &[TextRangeMapping {
                old: 0..len,
                new: delta..delta + len,
            }],
        )
        .unwrap();
    render(&mut host);
    let field = host.text_surface(ComponentId(2)).unwrap();
    assert!(
        (field.caret_doc(a.utf16 + delta).min_y() - field.scroll.offset.y - before).abs() <= 1.
    );
    assert_eq!(host.focused(), Some(ComponentId(3)));
    assert_eq!(
        host.text_surface(ComponentId(3)).unwrap().input.text,
        "keep me"
    );
}
