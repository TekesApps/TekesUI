use tekes_ui_controls::{
    BarChart, ButtonTextAlign, ButtonTextRenderer, ChartDatum, ControlKey,
    collections_view::{ChartView, CollectionEvent as E, DataTableView, ListView, TreeView},
    extended::{DataTable, List, Tree},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), r.style().line_height)
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
    Rect::new(8., 8., 300., 220.)
}
#[test]
fn list_late_focus_during_disabled_or_inactive_does_not_survive_restoration() {
    for disabled in [false, true] {
        let mut model = List::new(2);
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
        if disabled {
            view.set_disabled(true);
        } else {
            view.set_window_active(false);
        }
        view.focus(1, true);
        if disabled {
            view.set_disabled(false);
        } else {
            view.set_window_active(true);
        }
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            E::Ignored,
            "late focus must not become active: disabled={disabled}"
        );
        assert_eq!(model.selected, None);
        view.focus(1, true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            E::Activated(1)
        );
        assert_eq!(model.selected, Some(1));
    }
}
#[test]
fn tree_late_focus_during_disabled_or_inactive_does_not_survive_restoration() {
    for disabled in [false, true] {
        let mut model = Tree::default();
        let row = model.add("Row", None).unwrap();
        let mut view = TreeView::new();
        view.render(
            &mut Scene::new(),
            &mut Text,
            &model,
            bounds(),
            None,
            Theme::LIGHT.into(),
        );
        if disabled {
            view.set_disabled(true);
        } else {
            view.set_window_active(false);
        }
        view.focus(row, true);
        if disabled {
            view.set_disabled(false);
        } else {
            view.set_window_active(true);
        }
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            E::Ignored,
            "late focus disabled={disabled}"
        );
        assert_eq!(model.selected, None);
        view.focus(row, true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            E::Activated(row)
        );
        assert_eq!(model.selected, Some(row));
    }
}
fn table() -> (DataTable, DataTableView) {
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
    (model, view)
}
#[test]
fn table_row_late_focus_is_not_rearmed_by_restoration() {
    for disabled in [false, true] {
        let (mut model, mut view) = table();
        if disabled {
            view.set_disabled(true);
        } else {
            view.set_window_active(false);
        }
        view.focus_row(1, true);
        if disabled {
            view.set_disabled(false);
        } else {
            view.set_window_active(true);
        }
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), E::Ignored);
        assert_eq!(model.selected_row, None);
        assert_eq!(model.sort, None);
        view.focus_row(1, true);
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), E::Changed);
        assert_eq!(model.selected_row, Some(1));
    }
}
#[test]
fn table_header_late_focus_cannot_sort_after_restoration() {
    for disabled in [false, true] {
        let (mut model, mut view) = table();
        if disabled {
            view.set_disabled(true);
        } else {
            view.set_window_active(false);
        }
        view.focus_header(0, true);
        if disabled {
            view.set_disabled(false);
        } else {
            view.set_window_active(true);
        }
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), E::Ignored);
        assert_eq!(model.sort, None);
        assert_eq!(model.selected_row, None);
        view.focus_header(0, true);
        assert_eq!(view.key(ControlKey::Enter, true, &mut model), E::Changed);
        assert!(model.sort.is_some());
    }
}
#[test]
fn chart_late_inactive_focus_does_not_mutate_model_or_restore_keyboard_ownership() {
    let mut model = BarChart::new(vec![ChartDatum::new("A", 1.), ChartDatum::new("B", 2.)]);
    let mut view = ChartView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &mut model,
        bounds(),
        None,
        Theme::LIGHT.into(),
        1.,
    );
    let before = model.selected();
    view.set_window_active(false, &mut model);
    view.focus(true, &mut model);
    assert!(!model.is_focused());
    view.set_window_active(true, &mut model);
    assert_eq!(view.key(ControlKey::Right, true, &mut model), E::Ignored);
    assert_eq!(model.selected(), before);
    view.focus(true, &mut model);
    assert_eq!(view.key(ControlKey::Right, true, &mut model), E::Changed);
    assert_ne!(model.selected(), before);
}
