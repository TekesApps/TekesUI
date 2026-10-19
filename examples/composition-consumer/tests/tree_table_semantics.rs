use tekes_ui_controls::collections_view::{CollectionEvent, DataTableView, TreeView};
use tekes_ui_controls::extended::{DataTable, Tree};
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
fn table_headers_and_rows_require_current_mount_and_window() {
    let mut model = DataTable::new(3, 3);
    let mut view = DataTableView::new();
    view.focus_header(1, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(model.sort, None);
    let rows = vec![vec!["A".into(); 3]; 3];
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["A".into(), "B".into(), "C".into()],
        &rows,
        &[2, 0],
        Rect::new(0., 0., 300., 300.),
        None,
        Theme::LIGHT.into(),
    );
    view.focus_header(1, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Changed
    );
    view.focus_row(1, true);
    assert_eq!(
        view.key(ControlKey::Down, true, &mut model),
        CollectionEvent::Ignored
    );
    view.focus_row(2, true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Changed
    );
    assert_eq!(model.selected_row, Some(2));
    view.set_window_active(false);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    view.set_window_active(true);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
    view.focus_header(1, true);
    model.columns = 1;
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Ignored
    );
}

#[test]
fn tree_disclosure_is_explicit_idempotent_and_repairs_hidden_focus_and_selection() {
    let mut model = Tree::default();
    let root = model.add("root", None).unwrap();
    let child = model.add("child", Some(root)).unwrap();
    let mut view = TreeView::new();
    assert_eq!(
        view.set_expanded(root, true, &mut model),
        CollectionEvent::Ignored
    );
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        Rect::new(0., 0., 300., 300.),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(
        view.set_expanded(root, true, &mut model),
        CollectionEvent::Toggled(root)
    );
    assert_eq!(
        view.set_expanded(root, true, &mut model),
        CollectionEvent::Handled
    );
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        Rect::new(0., 0., 300., 300.),
        None,
        Theme::LIGHT.into(),
    );
    view.focus(child, true);
    model.select(child);
    let point = view
        .layout()
        .items
        .iter()
        .find(|item| item.id == child)
        .unwrap()
        .frame
        .center();
    assert_ne!(
        view.pointer_down(point, &mut model),
        CollectionEvent::Ignored
    );
    assert_eq!(
        view.set_expanded(root, false, &mut model),
        CollectionEvent::Toggled(root)
    );
    assert_eq!(model.selected, Some(root));
    assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Ignored);
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        CollectionEvent::Activated(root)
    );
    assert_eq!(
        view.set_expanded(child, true, &mut model),
        CollectionEvent::Ignored
    );
    view.set_window_active(false);
    assert_eq!(
        view.set_expanded(root, true, &mut model),
        CollectionEvent::Ignored
    );
    view.set_window_active(true);
    view.set_disabled(true);
    assert_eq!(
        view.set_expanded(root, true, &mut model),
        CollectionEvent::Ignored
    );
}
