use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    collections_view::{CollectionEvent as E, DataTableView, ListView, TreeView},
    extended::{DataTable, List, Tree},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (text.len() as f32 * 7.).min(width),
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
fn bounds() -> Rect {
    Rect::new(0., 0., 320., 300.)
}
fn list() -> (List, ListView, Point) {
    let model = List::new(2);
    let mut view = ListView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["A".into(), "B".into()],
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    let point = view.layout().items[0].frame.center();
    (model, view, point)
}
fn tree() -> (Tree, TreeView, usize, usize, Point) {
    let mut model = Tree::default();
    let root = model.add("root", None).unwrap();
    let branch = model.add("branch", Some(root)).unwrap();
    model.add("leaf", Some(branch)).unwrap();
    model.toggle(root);
    let mut view = TreeView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    let point = view
        .layout()
        .items
        .iter()
        .find(|r| r.id == branch)
        .unwrap()
        .frame
        .center();
    (model, view, root, branch, point)
}
fn table() -> (DataTable, DataTableView, Point, Point) {
    let model = DataTable::new(2, 1);
    let mut view = DataTableView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["Name".into()],
        &[vec!["A".into()], vec!["B".into()]],
        &[1, 0],
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    let row = view.layout().rows[0].frame.center();
    let header = view.layout().headers[0].frame.center();
    (model, view, row, header)
}

#[test]
fn removed_list_and_table_sources_reject_cached_pointer_targets_before_relayout() {
    let (mut model, mut view, point) = list();
    model.count = 0;
    assert_eq!(view.pointer_down(point, &mut model), E::Ignored);
    assert_eq!(model.selected, None);
    model.count = 2;
    assert_eq!(
        view.key(ControlKey::Enter, true, &mut model),
        E::Ignored,
        "rejected hit must not install focus"
    );
    let (mut model, mut view, row, header) = table();
    model.rows = 0;
    assert_eq!(view.pointer_down(row, &mut model), E::Ignored);
    assert_eq!(model.selected_row, None);
    model.columns = 0;
    assert_eq!(view.pointer_down(header, &mut model), E::Ignored);
    assert_eq!(model.sort, None);
    model.rows = 2;
    model.columns = 1;
    assert_eq!(view.key(ControlKey::Enter, true, &mut model), E::Ignored);
}

#[test]
fn externally_collapsed_tree_rejects_hidden_down_and_old_captured_release() {
    let (mut model, mut view, root, branch, point) = tree();
    model.toggle(root);
    assert_eq!(view.pointer_down(point, &mut model), E::Ignored);
    assert_eq!(model.selected, None);
    model.toggle(root);
    assert_eq!(view.pointer_down(point, &mut model), E::Changed);
    model.toggle(root);
    assert_eq!(view.pointer_up(point, &mut model), E::Ignored);
    assert!(
        !model.nodes()[branch].expanded,
        "hidden branch must not open from cached geometry"
    );
    model.toggle(root);
    assert_eq!(
        view.pointer_up(point, &mut model),
        E::Ignored,
        "old release cannot revive after expansion"
    );
    view.pointer_down(point, &mut model);
    assert_eq!(
        view.pointer_up(point, &mut model),
        E::Toggled(branch),
        "fresh gesture remains usable"
    );
}

#[test]
fn escape_cancels_captured_row_in_all_three_existing_semantic_collections() {
    let (mut model, mut view, point) = list();
    view.pointer_down(point, &mut model);
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    model.selected = None;
    assert_eq!(view.pointer_up(point, &mut model), E::Ignored);
    assert_eq!(model.selected, None);
    assert_eq!(
        view.key(ControlKey::Escape, true, &mut model),
        E::Ignored,
        "unowned Escape must bubble"
    );
    assert_eq!(view.pointer_down(point, &mut model), E::Changed);
    assert_eq!(view.pointer_up(point, &mut model), E::Handled);

    let (mut model, mut view, _, branch, point) = tree();
    view.pointer_down(point, &mut model);
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    assert_eq!(view.pointer_up(point, &mut model), E::Ignored);
    assert!(!model.nodes()[branch].expanded);
    view.pointer_down(point, &mut model);
    assert_eq!(
        view.key(ControlKey::Escape, false, &mut model),
        E::Ignored,
        "key-up is not cancellation"
    );
    assert_eq!(view.pointer_up(point, &mut model), E::Toggled(branch));

    let (mut model, mut view, row, _) = table();
    view.pointer_down(row, &mut model);
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    model.selected_row = None;
    assert_eq!(view.pointer_up(row, &mut model), E::Ignored);
    assert_eq!(model.selected_row, None);
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Ignored);
    assert_eq!(view.pointer_down(row, &mut model), E::Changed);
    assert_eq!(
        model.selected_row,
        Some(1),
        "source row identity survives sorted display order"
    );
}

#[test]
fn escape_still_cancels_capture_when_source_disappeared_before_the_key() {
    let (mut model, mut view, point) = list();
    view.pointer_down(point, &mut model);
    model.count = 0;
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    model.count = 2;
    model.selected = None;
    assert_eq!(view.pointer_up(point, &mut model), E::Ignored);

    let (mut model, mut view, root, branch, point) = tree();
    view.pointer_down(point, &mut model);
    model.toggle(root);
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    model.toggle(root);
    assert_eq!(view.pointer_up(point, &mut model), E::Ignored);
    assert!(!model.nodes()[branch].expanded);

    let (mut model, mut view, row, _) = table();
    view.pointer_down(row, &mut model);
    model.rows = 0;
    assert_eq!(view.key(ControlKey::Escape, true, &mut model), E::Handled);
    model.rows = 2;
    model.selected_row = None;
    assert_eq!(view.pointer_up(row, &mut model), E::Ignored);
}
