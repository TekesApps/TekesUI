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
use tekes_ui_controls::extended::{Axis, Stack};
use tekes_ui_controls::{ScrollArea, ScrollPolicy, TextSurfacePresentation};
use tekes_ui_scene::{Point, Primitive};

#[test]
fn plain_document_has_no_form_geometry_and_parent_clipping_does_not_reshape_it() {
    let value = "中文😀 e\u{301} selects and wraps without input decoration.\n".repeat(20);
    let mut body =
        TextFieldContent::read_only_content("Document name", &value, proposal::TextRole::UiBody);
    body.field.help = "semantic description only".into();
    let size = body.measure(&mut Text, Size::new(300., 10000.)).size;
    let frame = Rect::new(30., 40., 300., size.height);
    body.arrange(&mut Text, frame, frame);
    assert_eq!(body.surface.outer, body.surface.inner);
    assert_eq!(
        body.surface.outer.size.height,
        body.surface.paragraph.as_ref().unwrap().height as f32
    );
    let shaped = body.surface.paragraph.clone().unwrap();
    body.arrange(&mut Text, frame, Rect::new(70., 110., 160., 60.));
    assert!(Rc::ptr_eq(
        &shaped,
        body.surface.paragraph.as_ref().unwrap()
    ));
    assert_eq!(body.surface.width, Some(300.));
    let mut scene = Scene::new();
    body.paint(
        &mut scene,
        &mut Text,
        frame,
        frame,
        tekes_ui_tokens::Palette::from(Theme::LIGHT).into(),
    );
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Quad(_)))
    );
    assert_eq!(body.semantics().name, "Document name");
    assert!(body.surface.input.read_only);

    let mut form = TextFieldContent::new("Ordinary field", "value");
    assert_eq!(form.surface.presentation, TextSurfacePresentation::Field);
    let h = form.measure(&mut Text, Size::new(300., 1000.)).size.height;
    form.arrange(
        &mut Text,
        Rect::new(0., 0., 300., h),
        Rect::new(0., 0., 300., h),
    );
    let mut normal = Scene::new();
    form.paint(
        &mut normal,
        &mut Text,
        Rect::new(0., 0., 300., h),
        Rect::new(0., 0., 300., h),
        tekes_ui_tokens::Palette::from(Theme::LIGHT).into(),
    );
    assert!(
        normal
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Quad(_)))
    );
    assert!(form.surface.inner.size.width < form.surface.outer.size.width);
}

#[test]
fn text_cursor_regions_follow_visible_text_viewports_without_focusing_or_editing() {
    let mut disabled = TextFieldContent::new("Disabled", "disabled value");
    disabled.surface.input.disabled = true;
    let root = Content::new(
        ComponentId(200),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.,
            },
            children: vec![
                Content::new(ComponentId(201), TextFieldContent::new("Editable", "draft")),
                Content::new(
                    ComponentId(202),
                    TextFieldContent::read_only_content(
                        "Selectable document",
                        "read only text",
                        proposal::TextRole::UiBody,
                    ),
                ),
                Content::new(ComponentId(203), disabled),
            ],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(root).unwrap();
    let frame = Rect::new(0., 0., 280., 180.);
    let clip = Rect::new(16., 8., 190., 92.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();

    let regions = host.text_cursor_regions();
    let expected = [ComponentId(201), ComponentId(202)]
        .into_iter()
        .map(|id| {
            host.get(id)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .inner
                .intersection(&clip)
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(regions, expected);
    assert!(
        regions
            .iter()
            .all(|region| region.intersection(&clip) == Some(*region))
    );
    assert!(
        host.get(ComponentId(203))
            .unwrap()
            .component::<TextFieldContent>()
            .is_some()
    );

    let before = [ComponentId(201), ComponentId(202)].map(|id| {
        let input = &host
            .get(id)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        (
            input.text.clone(),
            input.anchor,
            input.caret,
            input.marked.clone(),
        )
    });
    host.event(&ContentEvent::PointerMove(regions[0].center()));
    assert_eq!(
        host.focused(),
        None,
        "cursor hover must not acquire text focus"
    );
    for (index, id) in [ComponentId(201), ComponentId(202)].into_iter().enumerate() {
        let input = &host
            .get(id)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        assert_eq!(
            (
                input.text.clone(),
                input.anchor,
                input.caret,
                input.marked.clone(),
            ),
            before[index],
            "cursor hover must preserve editor state"
        );
    }

    host.set_window_active(false);
    assert!(host.text_cursor_regions().is_empty());
}

#[test]
fn plain_document_delegates_precise_and_wheel_scroll_to_its_single_parent() {
    let text = "History 中文😀 e\u{301}\n".repeat(100);
    let body = Content::new(
        ComponentId(2),
        TextFieldContent::read_only_content("Body", &text, proposal::TextRole::UiBody),
    );
    let child = Content::new(
        ComponentId(3),
        StackContent {
            model: Stack::new(Axis::Vertical),
            children: vec![body, Content::new(ComponentId(4), Button::new("Action"))],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        ScrollContent::new(ScrollArea::default(), child),
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 300., 180.);
    for precise in [true, false] {
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Palette::from(Theme::LIGHT),
        )
        .unwrap();
        let surface = host.text_surface(ComponentId(2)).unwrap();
        assert_eq!(
            surface.scroll.scroll_policy(),
            (ScrollPolicy::Disabled, ScrollPolicy::Disabled)
        );
        assert_eq!(surface.scroll.limit(), Point::new(0., 0.));
        let paragraph = surface.paragraph.clone().unwrap();
        let before = host
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y;
        assert!(
            host.event(&ContentEvent::Scroll {
                position: Point::new(50., 50.),
                delta: Point::new(0., 80.),
                precise,
                begin: true,
                end: false,
                momentum: false,
                time: 1.
            })
            .consumed
        );
        let after = host
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y;
        assert!(after > before);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Palette::from(Theme::LIGHT),
        )
        .unwrap();
        assert!(Rc::ptr_eq(
            &paragraph,
            host.text_surface(ComponentId(2))
                .unwrap()
                .paragraph
                .as_ref()
                .unwrap()
        ));
        assert_eq!(
            host.text_surface(ComponentId(2)).unwrap().scroll.offset,
            Point::new(0., 0.)
        );
    }
}

#[test]
fn plain_document_keeps_utf16_selection_across_controlled_append_and_rejects_stale_values() {
    use tekes_ui_controls::{
        DraftRevision, ExternalSelection as S, ExternalUndo as U, ExternalValueResult as R,
    };
    let original = "中文😀e\u{301} tail";
    let mut body =
        TextFieldContent::read_only_content("Document", original, proposal::TextRole::UiBody);
    let first = DraftRevision {
        identity: "document-A".into(),
        version: 1,
        payload: original.into(),
    };
    let initial = body.surface.input.external_baseline();
    assert!(matches!(
        body.surface
            .apply_external_value(&initial, &first, S::Reset, U::Reset),
        R::Applied { .. }
    ));
    assert!(
        body.accessibility_action(None, &AccessibilityAction::SetSelection(2..4))
            .consumed
    );
    assert_eq!(body.surface.input.selected_text(), "😀");
    let selected = body.surface.input.external_baseline();
    let second = DraftRevision {
        identity: "document-A".into(),
        version: 2,
        payload: format!("{original}\n流式追加"),
    };
    assert!(matches!(
        body.surface
            .apply_external_value(&selected, &second, S::Preserve, U::Preserve),
        R::Applied { .. }
    ));
    assert_eq!(body.surface.input.selection(), 2..4);
    assert_eq!(body.surface.input.selected_text(), "😀");
    assert!(matches!(
        body.surface
            .apply_external_value(&initial, &first, S::Reset, U::Reset),
        R::Conflict(_)
    ));
    assert!(matches!(
        body.surface.apply_external_value(
            &body.surface.input.external_baseline(),
            &first,
            S::Reset,
            U::Reset
        ),
        R::Conflict(_)
    ));
    assert_eq!(body.surface.input.text, second.payload);
    assert!(body.surface.input.read_only);
    assert_eq!(body.semantics().value, Some(second.payload));
}
