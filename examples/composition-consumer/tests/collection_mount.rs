use tekes_ui_controls::collections_view::{CollectionEvent, ListView, TreeView};
use tekes_ui_controls::extended::{List, Tree};
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
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
#[test]
fn public_list_rejects_unmounted_model_rows_but_retains_clipped_keyboard_targets() {
    let mut model = List::new(5);
    let mut view = ListView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["one".into(), "two".into(), "three".into()],
        Rect::new(0., 0., 300., 200.),
        Some(Rect::new(0., 0., 300., 20.)),
        Theme::LIGHT.into(),
    );
    view.focus(4, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(model.selected, None);
    assert!(view.layout().items[2].visible.is_none());
    view.focus(2, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(2)
    );
    assert_eq!(model.selected, Some(2));
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(2)
    );
    view.set_disabled(true);
    view.focus(0, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(model.selected, Some(2));
}
#[test]
fn public_tree_newly_expanded_rows_require_layout_before_activation() {
    let mut model = Tree::default();
    let parent = model.add("parent", None).unwrap();
    let child = model.add("child", Some(parent)).unwrap();
    let mut view = TreeView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        Rect::new(0., 0., 300., 200.),
        None,
        Theme::LIGHT.into(),
    );
    assert!(model.toggle(parent));
    view.focus(child, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(model.selected, None);
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        Rect::new(0., 0., 300., 200.),
        Some(Rect::new(0., 0., 300., 20.)),
        Theme::LIGHT.into(),
    );
    assert!(
        view.layout()
            .items
            .iter()
            .find(|row| row.id == child)
            .unwrap()
            .visible
            .is_none()
    );
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(child)
    );
    assert_eq!(model.selected, Some(child));
}
