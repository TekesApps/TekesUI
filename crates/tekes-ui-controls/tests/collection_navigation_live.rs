use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    collections_view::{CollectionEvent as E, DataTableView},
    extended::DataTable,
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

fn table(order: &[usize], columns: usize) -> (DataTable, DataTableView) {
    let model = DataTable::new(3, columns);
    let mut view = DataTableView::new();
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["A".into(), "B".into(), "C".into()],
        &[vec!["A".into()], vec!["B".into()], vec!["C".into()]],
        order,
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    (model, view)
}
#[test]
fn shrinking_sorted_source_cannot_move_keyboard_focus_to_a_removed_row() {
    for key in [ControlKey::Down, ControlKey::End] {
        let order = if key == ControlKey::Down {
            [0, 2, 1]
        } else {
            [0, 1, 2]
        };
        let (mut model, mut view) = table(&order, 1);
        view.focus_row(0, true);
        model.rows = 2;
        assert_eq!(
            view.key(key, true, &mut model),
            E::Changed,
            "live source must filter the old mounted row order"
        );
        assert_eq!(model.selected_row, Some(1));
        assert_ne!(
            view.key(ControlKey::Enter, true, &mut model),
            E::Ignored,
            "navigation must not poison the focus owner"
        );
    }
}
#[test]
fn growing_source_cannot_focus_a_header_that_has_not_been_mounted() {
    let (mut model, mut view) = table(&[0, 1, 2], 1);
    view.focus_header(0, true);
    model.columns = 3;
    view.key(ControlKey::Right, true, &mut model);
    assert_ne!(
        view.key(ControlKey::Enter, true, &mut model),
        E::Ignored,
        "old mounted header stays usable until relayout"
    );
    assert_eq!(
        model.sort.map(|(column, _)| column),
        Some(0),
        "unmounted new header cannot capture focus"
    );
}

#[test]
fn freshly_mounted_columns_remain_navigable_after_model_growth() {
    let (mut model, mut view) = table(&[0, 1, 2], 1);
    view.focus_header(0, true);
    model.columns = 3;
    view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        &["A".into(), "B".into(), "C".into()],
        &[vec!["A".into()], vec!["B".into()], vec!["C".into()]],
        &[0, 1, 2],
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    view.key(ControlKey::Right, true, &mut model);
    assert_ne!(view.key(ControlKey::Enter, true, &mut model), E::Ignored);
    assert_eq!(model.sort.map(|(column, _)| column), Some(1));
}
