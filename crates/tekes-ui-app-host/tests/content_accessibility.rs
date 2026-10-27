#![cfg(target_os = "macos")]
use tekes_ui_app_host::accessibility::{content_action, content_key, content_nodes};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ToolbarAction, ToolbarControl, composition::*,
    layout::MeasuredChild,
};
use tekes_ui_platform_macos::{AccessibilityAction as A, AccessibilityRole as R};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(width), 24.)
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
const ROOT: ComponentId = ComponentId(1);
const BUTTON: ComponentId = ComponentId(2);
fn host() -> ContentHost {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        ListContent::new("Actions", vec![Content::new(BUTTON, Button::new("Send"))]),
    ))
    .unwrap();
    host.set_action_events(true);
    paint(&mut host);
    host
}
fn paint(host: &mut ContentHost) {
    let f = Rect::new(0., 0., 500., 200.);
    host.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
#[test]
fn exact_original_mount_blocks_foreign_and_replaced_same_id_owners() {
    let mut a = host();
    let b = host();
    let old = content_key(&a, BUTTON, None).unwrap();
    let foreign = content_key(&b, BUTTON, None).unwrap();
    assert_ne!(old, foreign);
    assert!(!content_action(&mut a, true, foreign, A::Press).consumed);
    assert!(!content_action(&mut a, false, old, A::Press).consumed);
    assert!(a.drain_actions().is_empty());
    let removed = a.remove(BUTTON).unwrap();
    a.insert(ROOT, Content::new(BUTTON, Button::new("Replacement")))
        .unwrap();
    paint(&mut a);
    let new = content_key(&a, BUTTON, None).unwrap();
    assert_ne!(new, old);
    assert!(!content_action(&mut a, true, old, A::Press).consumed);
    assert!(content_action(&mut a, true, new, A::Press).activated);
    assert_eq!(a.drain_actions().len(), 1);
    a.remove(BUTTON).unwrap();
    a.insert(ROOT, removed).unwrap();
    paint(&mut a);
    assert_ne!(content_key(&a, BUTTON, None).unwrap(), old);
    assert!(!content_action(&mut a, true, old, A::Focus).consumed);
}
#[test]
fn reorder_preserves_keys_original_focus_and_one_action_source() {
    let mut h = host();
    h.insert(ROOT, Content::new(ComponentId(3), Button::new("Other")))
        .unwrap();
    paint(&mut h);
    let key = content_key(&h, BUTTON, None).unwrap();
    h.reorder(ROOT, &[ComponentId(3), BUTTON]).unwrap();
    paint(&mut h);
    assert_eq!(content_key(&h, BUTTON, None), Some(key));
    assert!(content_action(&mut h, true, key, A::Focus).consumed);
    assert_eq!(h.focused(), Some(BUTTON));
    assert!(h.drain_actions().is_empty());
    let nodes = content_nodes(&h, true);
    let node = nodes.iter().find(|n| n.key == key).unwrap();
    assert_eq!(node.role, R::Button);
    assert!(node.focused);
    assert_eq!(node.label, "Send");
    assert!(content_action(&mut h, true, key, A::Press).activated);
    let actions = h.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].source, ContentActionSource::Accessibility);
    assert!(content_nodes(&h, false).iter().all(|n| !n.focused));
    h.get_mut(BUTTON)
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .button
        .state
        .disabled = true;
    assert!(!content_action(&mut h, true, key, A::Press).consumed);
    assert!(
        !content_nodes(&h, true)
            .iter()
            .find(|n| n.key == key)
            .unwrap()
            .enabled
    );
    assert!(!content_action(&mut h, true, key, A::SetExpanded(true)).consumed);
}
#[derive(Debug)]
struct Gate {
    active: bool,
    children: Vec<Content>,
}
impl Component for Gate {
    fn active(&self) -> bool {
        self.active
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[test]
fn inactive_ancestor_removes_original_mount_from_projection_and_dispatch() {
    let mut h = ContentHost::new(Content::new(
        ROOT,
        Gate {
            active: true,
            children: vec![Content::new(BUTTON, Button::new("Send"))],
        },
    ))
    .unwrap();
    paint(&mut h);
    let key = content_key(&h, BUTTON, None).unwrap();
    h.get_mut(ROOT)
        .unwrap()
        .component_mut::<Gate>()
        .unwrap()
        .active = false;
    assert!(content_key(&h, BUTTON, None).is_none());
    assert!(content_nodes(&h, true).is_empty());
    assert!(!content_action(&mut h, true, key, A::Press).consumed);
}
#[test]
fn native_toolbar_parts_share_original_mount_and_more_hierarchy() {
    let mut toolbar = ToolbarControl::default();
    toolbar.set_viewport(Rect::new(0., 0., 500., 400.));
    toolbar.set_actions(vec![
        ToolbarAction {
            id: 10,
            title: "First".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 20,
            title: "Last".into(),
            enabled: true,
        },
    ]);
    let mut h = ContentHost::new(Content::new(ROOT, toolbar)).unwrap();
    h.set_action_events(true);
    let f = Rect::new(0., 0., 45., 48.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    let more = content_nodes(&h, true)
        .into_iter()
        .find(|n| n.role == R::MenuButton)
        .unwrap()
        .key;
    assert!(content_action(&mut h, true, more, A::Press).consumed);
    assert!(h.drain_actions().is_empty());
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    let nodes = content_nodes(&h, true);
    assert!(
        nodes
            .iter()
            .find(|n| n.key == more)
            .unwrap()
            .expanded
            .unwrap()
    );
    let last = nodes.iter().find(|n| n.label == "Last").unwrap();
    assert_eq!(last.role, R::MenuItem);
    assert_eq!(last.parent, Some(more));
    let key = last.key;
    assert!(content_action(&mut h, true, key, A::Focus).consumed);
    assert_eq!(h.focused(), Some(ROOT));
    assert!(content_action(&mut h, true, key, A::Press).activated);
    assert_eq!(h.drain_actions().len(), 1);
}
