use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextInput, composition::*,
    feedback_view::PopoverPlacement, layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, max: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(max), 24.)
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
const ROOT: ComponentId = ComponentId(4600);
const A: ComponentId = ComponentId(4601);
const B: ComponentId = ComponentId(4602);
const EDITOR: ComponentId = ComponentId(4603);
const PREVIEW: ComponentId = ComponentId(4604);
#[derive(Debug)]
struct Message(&'static str);
impl Component for Message {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(140., 64.))
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Text, self.0)
    }
}
#[derive(Debug)]
struct Draft(TextInput);
impl Component for Draft {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(220., 40.))
    }
    fn focusable(&self) -> bool {
        true
    }
    fn composing(&self) -> bool {
        self.0.marked.is_some()
    }
}
#[derive(Debug)]
struct Canvas {
    children: Vec<Content>,
    offset: f32,
}
impl Component for Canvas {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(320., 240.))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        for child in &mut self.children {
            let frame = match child.id() {
                A => Rect::new(20., 30. + self.offset, 24., 24.),
                B => Rect::new(50., 30., 180., 80.),
                EDITOR => Rect::new(20., 160., 220., 40.),
                _ => Rect::default(),
            };
            child.arrange(text, frame, clip);
        }
    }
}
fn body(name: &'static str) -> Content {
    Content::new(ComponentId(4610), Message(name))
}
fn host() -> (ContentHost, HoverPreviewVersion) {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        Canvas {
            children: vec![
                Content::new(A, Button::new("A")),
                Content::new(B, Button::new("B")),
                Content::new(EDITOR, Draft(TextInput::new("draft", true))),
                Content::new(
                    PREVIEW,
                    HoverPreviewContent::new("Turn preview", body("Input and output")).unwrap(),
                ),
            ],
            offset: 0.,
        },
    ))
    .unwrap();
    host.set_action_events(true);
    let version = host.bind_hover_preview(A, PREVIEW).unwrap();
    render(&mut host);
    host.focus(Some(EDITOR), true);
    (host, version)
}
fn render(host: &mut ContentHost) {
    host.render(
        &mut Scene::new(),
        &mut Text,
        Rect::new(0., 0., 320., 240.),
        Rect::new(0., 0., 320., 240.),
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn hover(host: &mut ContentHost) {
    host.event(&ContentEvent::PointerMove(Point::new(30., 40.)));
    render(host);
}
fn preview(host: &ContentHost) -> &HoverPreviewContent {
    host.get(PREVIEW)
        .unwrap()
        .component::<HoverPreviewContent>()
        .unwrap()
}
#[test]
fn hover_preview_original_slot_does_not_focus_intercept_or_add_dismiss() {
    let (mut host, _) = host();
    hover(&mut host);
    assert!(preview(&host).is_open());
    assert_eq!(host.focused(), Some(EDITOR));
    assert!(
        host.semantics()
            .iter()
            .any(|node| node.semantics.name == "Input and output")
    );
    assert!(
        !host
            .semantics()
            .iter()
            .flat_map(|node| &node.parts)
            .any(|part| part.semantics.name == "Dismiss")
    );
    assert!(!host.focus_targets().iter().any(|(id, _)| *id == PREVIEW));
    let frame = preview(&host).layout().frame;
    assert!(frame.min_x() > host.get(A).unwrap().frame().max_x());
    let point = Point::new(80., 60.);
    assert!(frame.contains(point));
    assert!(host.get(B).unwrap().frame().contains(point));
    host.event(&ContentEvent::PointerDown(point));
    host.event(&ContentEvent::PointerUp(point));
    assert!(!preview(&host).is_open());
    assert_eq!(host.drain_actions()[0].target, B);
}
#[test]
fn hover_preview_static_stream_update_rejects_old_version_and_escape_remains_dismissed() {
    let (mut host, old) = host();
    hover(&mut host);
    let next = host
        .update_hover_preview(old, body("Output streaming update"))
        .unwrap();
    render(&mut host);
    assert!(preview(&host).is_open());
    assert_eq!(preview(&host).version(), Some(next));
    assert!(
        host.semantics()
            .iter()
            .any(|node| node.semantics.name == "Output streaming update")
    );
    assert_eq!(
        host.update_hover_preview(old, body("late")),
        Err(HoverPreviewError::StaleVersion)
    );
    assert!(
        host.event(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true
        })
        .consumed
    );
    let next2 = host
        .update_hover_preview(next, body("Next update"))
        .unwrap();
    render(&mut host);
    assert!(!preview(&host).is_open());
    hover(&mut host);
    assert!(!preview(&host).is_open());
    host.event(&ContentEvent::PointerMove(Point::new(0., 0.)));
    hover(&mut host);
    assert!(preview(&host).is_open());
    assert_eq!(preview(&host).version(), Some(next2));
}
#[test]
fn hover_preview_marked_text_is_preserved_and_escape_keeps_original_input_route() {
    let (mut host, _) = host();
    host.get_mut(EDITOR)
        .unwrap()
        .component_mut::<Draft>()
        .unwrap()
        .0
        .preedit("中文", 1..1, None);
    let before = {
        let input = &host.get(EDITOR).unwrap().component::<Draft>().unwrap().0;
        (input.text.clone(), input.marked.clone(), input.selection())
    };
    hover(&mut host);
    assert!(preview(&host).is_open());
    assert!(
        !host
            .event(&ContentEvent::Key {
                key: ControlKey::Escape,
                down: true
            })
            .consumed
    );
    let input = &host.get(EDITOR).unwrap().component::<Draft>().unwrap().0;
    assert_eq!(
        (input.text.clone(), input.marked.clone(), input.selection()),
        before
    );
    assert_eq!(host.focused(), Some(EDITOR));
}
#[test]
fn hover_preview_clip_scroll_deactivation_and_owner_mount_retire() {
    let (mut host, old) = host();
    hover(&mut host);
    host.get_mut(ROOT)
        .unwrap()
        .component_mut::<Canvas>()
        .unwrap()
        .offset = 12.;
    render(&mut host);
    assert!(!preview(&host).is_open());
    host.event(&ContentEvent::PointerMove(Point::new(0., 0.)));
    host.get_mut(ROOT)
        .unwrap()
        .component_mut::<Canvas>()
        .unwrap()
        .offset = 0.;
    render(&mut host);
    hover(&mut host);
    host.set_window_active(false);
    assert!(!preview(&host).is_open());
    host.set_window_active(true);
    render(&mut host);
    host.event(&ContentEvent::PointerMove(Point::new(0., 0.)));
    hover(&mut host);
    host.event(&ContentEvent::Scroll {
        position: Point::new(30., 40.),
        delta: Point::new(0., 10.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 0.,
    });
    assert!(!preview(&host).is_open());
    let row = host.remove(A).unwrap();
    host.insert(ROOT, row).unwrap();
    render(&mut host);
    hover(&mut host);
    assert!(!preview(&host).is_open());
    assert_eq!(
        host.update_hover_preview(old, body("old owner")),
        Err(HoverPreviewError::OwnerUnavailable)
    );
    host.bind_hover_preview(A, PREVIEW).unwrap();
    hover(&mut host);
    assert!(preview(&host).is_open());
}
#[test]
fn hover_preview_rejects_interactive_subtrees_and_duplicate_updates_atomically() {
    assert!(
        HoverPreviewContent::new(
            "invalid",
            Content::new(ComponentId(4620), Button::new("Interactive"))
        )
        .is_err()
    );
    let (mut host, version) = host();
    hover(&mut host);
    assert!(matches!(
        host.update_hover_preview(version, Content::new(B, Message("duplicate"))),
        Err(HoverPreviewError::Composition(
            CompositionError::DuplicateId(B)
        ))
    ));
    assert!(
        host.semantics()
            .iter()
            .any(|node| node.semantics.name == "Input and output")
    );
}
#[test]
fn hover_preview_right_placement_alignment_flips_and_clamps_narrow_viewport() {
    let (mut host, _) = host();
    for placement in [
        PopoverPlacement::RightStart,
        PopoverPlacement::RightCenter,
        PopoverPlacement::RightEnd,
    ] {
        host.get_mut(PREVIEW)
            .unwrap()
            .component_mut::<HoverPreviewContent>()
            .unwrap()
            .set_placement(placement);
        hover(&mut host);
        let frame = preview(&host).layout().frame;
        assert!(
            frame.min_x() >= 0.
                && frame.max_x() <= 320.
                && frame.min_y() >= 0.
                && frame.max_y() <= 240.
        );
        host.render(
            &mut Scene::new(),
            &mut Text,
            Rect::new(0., 0., 100., 100.),
            Rect::new(0., 0., 100., 100.),
            Theme::LIGHT.into(),
        )
        .unwrap();
        let frame = preview(&host).layout().frame;
        assert!(
            frame.min_x() >= 0.
                && frame.max_x() <= 100.
                && frame.min_y() >= 0.
                && frame.max_y() <= 100.
        );
    }
}

#[test]
fn hover_preview_placement_reuses_default_below_and_exact_right_alignment() {
    use tekes_ui_controls::{extended::Popover, feedback_view::PopoverView};
    let mut model = Popover::default();
    model.show(0);
    let mut view = PopoverView::default();
    let anchor = Rect::new(100., 50., 20., 20.);
    let size = Size::new(80., 60.);
    let viewport = Rect::new(0., 0., 300., 240.);
    let frame = view
        .arrange(
            &mut Text,
            &model,
            anchor,
            size,
            viewport,
            Some(viewport),
            proposal::TextRole::UiBody,
        )
        .frame;
    assert_eq!(frame, Rect::new(100., 76., 80., 60.));
    for (placement, y) in [
        (PopoverPlacement::RightStart, 50.),
        (PopoverPlacement::RightCenter, 30.),
        (PopoverPlacement::RightEnd, 10.),
    ] {
        view.set_placement(placement);
        assert_eq!(
            view.arrange(
                &mut Text,
                &model,
                anchor,
                size,
                viewport,
                Some(viewport),
                proposal::TextRole::UiBody
            )
            .frame,
            Rect::new(126., y, 80., 60.)
        );
    }
    view.set_placement(PopoverPlacement::RightStart);
    assert_eq!(
        view.arrange(
            &mut Text,
            &model,
            Rect::new(280., 50., 20., 20.),
            size,
            viewport,
            Some(viewport),
            proposal::TextRole::UiBody
        )
        .frame,
        Rect::new(194., 50., 80., 60.)
    );
}
