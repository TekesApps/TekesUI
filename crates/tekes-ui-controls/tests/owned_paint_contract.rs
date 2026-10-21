use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(80., 24.)
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
#[derive(Debug)]
struct Marker(Rc<Cell<usize>>);
impl Component for Marker {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(100., 40.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        f: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.0.set(self.0.get() + 1);
        scene.push_quad(Quad::new(f, theme.palette.color("primary-text").unwrap()));
    }
    fn focusable(&self) -> bool {
        true
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Button, "Owned child")
    }
}
#[derive(Debug)]
struct Owner {
    children: Vec<Content>,
    open: bool,
    exit: bool,
}
impl Component for Owner {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(text, f, c);
    }
    fn active(&self) -> bool {
        self.open
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_open(&self) -> bool {
        self.open
    }
    fn overlay_presented(&self) -> bool {
        self.open || self.exit
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        for child in &mut self.children {
            child.paint(scene, text, theme);
            child.paint_overlays(scene, text, theme);
        }
    }
}
fn render(host: &mut ContentHost) -> Scene {
    let mut scene = Scene::new();
    let f = Rect::new(0., 0., 300., 200.);
    host.render(&mut scene, &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    scene
}
#[test]
fn consumer_owned_paint_is_once_and_retained_exit_has_no_input_or_semantics() {
    let paints = Rc::new(Cell::new(0));
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Owner {
            children: vec![Content::new(ComponentId(2), Marker(paints.clone()))],
            open: true,
            exit: false,
        },
    ))
    .unwrap();
    assert!(!render(&mut host).is_empty());
    assert_eq!(paints.get(), 1);
    assert!(host.focus(Some(ComponentId(2)), true));
    let owner = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Owner>()
        .unwrap();
    owner.open = false;
    owner.exit = true;
    assert!(!render(&mut host).is_empty());
    assert_eq!(
        paints.get(),
        2,
        "ordinary and overlay passes must not double paint owned children"
    );
    assert_eq!(host.focused(), None);
    assert!(host.semantics().is_empty());
    assert!(
        !host
            .accessibility_action(ComponentId(2), None, AccessibilityAction::Press)
            .consumed
    );
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Owner>()
        .unwrap()
        .exit = false;
    assert!(render(&mut host).is_empty());
    assert_eq!(paints.get(), 2);
}
#[test]
fn public_focus_capture_reuses_mount_identity_without_a_modal() {
    for mode in [
        "same",
        "reorder",
        "replacement",
        "remount",
        "disabled",
        "loading",
        "removed",
    ] {
        let mut host = ContentHost::new(Content::new(
            ComponentId(10),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(ComponentId(11), ButtonContent::new("Caller")),
                    Content::new(ComponentId(12), ButtonContent::new("Other")),
                ],
            },
        ))
        .unwrap();
        render(&mut host);
        host.focus(Some(ComponentId(11)), true);
        let captured = host.capture_focus();
        assert!(captured.is_some());
        host.focus(Some(ComponentId(12)), true);
        match mode {
            "reorder" => host
                .reorder(ComponentId(10), &[ComponentId(12), ComponentId(11)])
                .unwrap(),
            "replacement" => {
                host.remove(ComponentId(11)).unwrap();
                host.insert(
                    ComponentId(10),
                    Content::new(ComponentId(11), ButtonContent::new("Replacement")),
                )
                .unwrap();
            }
            "remount" => {
                let original = host.remove(ComponentId(11)).unwrap();
                host.insert(ComponentId(10), original).unwrap();
            }
            "removed" => {
                host.remove(ComponentId(11)).unwrap();
            }
            "disabled" | "loading" => {
                let button = &mut host
                    .get_mut(ComponentId(11))
                    .unwrap()
                    .component_mut::<ButtonContent>()
                    .unwrap()
                    .button;
                button.state.disabled = mode == "disabled";
                button.state.loading = mode == "loading";
            }
            _ => {}
        }
        host.restore_focus(captured);
        assert_eq!(
            host.focused(),
            matches!(mode, "same" | "reorder").then_some(ComponentId(11)),
            "{mode}"
        );
    }
}

#[test]
fn retained_owned_overlay_still_requires_the_real_text_backend() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Owner {
            children: vec![Content::new(
                ComponentId(2),
                TextFieldContent::new("Draft", "Preserved"),
            )],
            open: false,
            exit: true,
        },
    ))
    .unwrap();
    let frame = Rect::new(0., 0., 300., 200.);
    assert_eq!(
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into()
        ),
        Err(CompositionError::MissingTextServices(ComponentId(2)))
    );
    assert!(
        host.text_surface(ComponentId(2)).is_none(),
        "paint-only exit must not publish inactive input"
    );
}
