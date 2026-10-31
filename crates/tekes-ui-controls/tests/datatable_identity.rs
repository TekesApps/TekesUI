use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    collections_view::{CollectionEvent as E, DataTableView},
    extended::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
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
fn budget() -> TableBudget {
    TableBudget {
        max_units: 1000,
        max_utf8_bytes: 1000,
    }
}
fn snapshot() -> KeyedTableSnapshot {
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: vec![
            KeyedTableColumn {
                key: TableColumnKey(1),
                name: "same".into(),
            },
            KeyedTableColumn {
                key: TableColumnKey(2),
                name: "same".into(),
            },
        ],
        rows: (1..=3)
            .map(|i| KeyedTableRow {
                key: TableRowKey(i),
                name: "same".into(),
                cells: vec![format!("R{i}"), "v".into()],
            })
            .collect(),
        display_rows: vec![TableRowKey(3), TableRowKey(1), TableRowKey(2)],
    }
}
fn table() -> KeyedDataTable {
    KeyedDataTable::new(snapshot(), budget()).unwrap()
}
fn bounds() -> Rect {
    Rect::new(0., 0., 300., 220.)
}
fn render(v: &mut DataTableView, t: &KeyedDataTable) {
    v.render_keyed(
        &mut Scene::new(),
        &mut Text,
        t,
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
}
fn row_point(v: &DataTableView, t: &KeyedDataTable, key: u64) -> Point {
    let f = v
        .layout_keyed(t)
        .unwrap()
        .rows
        .iter()
        .find(|r| r.id == t.row_index(TableRowKey(key)).unwrap())
        .unwrap()
        .frame;
    Point::new(f.min_x() + 5., f.min_y() + 5.)
}
#[test]
fn key_index_ordinal_and_reordering_keep_single_legacy_state() {
    let mut t = table();
    assert_eq!(t.display_ordinal(TableRowKey(3)), Some(0));
    assert_eq!(t.row_index(TableRowKey(3)), Some(2));
    assert!(t.select_key(TableRowKey(1)));
    assert!(t.sort_key(TableColumnKey(2)));
    assert!(t.sort_key(TableColumnKey(2)));
    let mut s = snapshot();
    s.rows.reverse();
    s.columns.reverse();
    for r in &mut s.rows {
        r.cells.reverse();
    }
    s.rows[2].name = "renamed".into();
    s.display_rows.reverse();
    let r = t.replace_keyed(s).unwrap();
    assert_eq!(r.old_rows_to_new, vec![Some(2), Some(1), Some(0)]);
    assert_eq!(r.old_columns_to_new, vec![Some(1), Some(0)]);
    assert_eq!(t.original().selected_row, Some(2));
    assert_eq!(t.selected_key(), Some(TableRowKey(1)));
    assert_eq!(t.original().sort, Some((0, SortDirection::Descending)));
    assert_eq!(
        t.sort_key_state(),
        Some((TableColumnKey(2), SortDirection::Descending))
    );
}
#[test]
fn hiding_deleting_dataset_and_zero_columns_clear_without_revival() {
    let mut t = table();
    t.select_key(TableRowKey(1));
    t.sort_key(TableColumnKey(1));
    let mut s = snapshot();
    s.display_rows.retain(|k| *k != TableRowKey(1));
    t.replace_keyed(s).unwrap();
    assert_eq!(t.selected_key(), None);
    assert!(!t.select_key(TableRowKey(1)));
    t.replace_keyed(snapshot()).unwrap();
    assert_eq!(t.selected_key(), None);
    t.select_key(TableRowKey(2));
    let mut s = snapshot();
    s.rows.remove(1);
    s.display_rows.retain(|k| *k != TableRowKey(2));
    s.columns.remove(0);
    for r in &mut s.rows {
        r.cells.remove(0);
    }
    t.replace_keyed(s).unwrap();
    assert_eq!(t.selected_key(), None);
    assert_eq!(t.sort_key_state(), None);
    let mut s = snapshot();
    s.columns.clear();
    for r in &mut s.rows {
        r.cells.clear();
    }
    t.replace_keyed(s).unwrap();
    assert!(t.display_indices().is_empty());
    assert!(!t.select_key(TableRowKey(1)));
    t.replace_keyed(snapshot()).unwrap();
    t.select_key(TableRowKey(1));
    t.sort_key(TableColumnKey(1));
    let mut s = snapshot();
    s.dataset = TableDatasetKey(2);
    t.replace_keyed(s).unwrap();
    t.replace_keyed(snapshot()).unwrap();
    assert_eq!(t.selected_key(), None);
    assert_eq!(t.sort_key_state(), None);
}
#[test]
fn all_invalid_snapshots_preserve_model_view_and_capture() {
    let mut invalid = Vec::new();
    let mut s = snapshot();
    s.rows[1].key = s.rows[0].key;
    invalid.push(s);
    let mut s = snapshot();
    s.columns[1].key = s.columns[0].key;
    invalid.push(s);
    let mut s = snapshot();
    s.rows[0].cells.pop();
    invalid.push(s);
    let mut s = snapshot();
    s.display_rows.push(TableRowKey(1));
    invalid.push(s);
    let mut s = snapshot();
    s.display_rows.push(TableRowKey(99));
    invalid.push(s);
    let mut s = snapshot();
    s.rows[0].name = "x".repeat(1001);
    invalid.push(s);
    let mut s = snapshot();
    for i in 4..=400 {
        s.rows.push(KeyedTableRow {
            key: TableRowKey(i),
            name: String::new(),
            cells: vec![String::new(); 2],
        });
    }
    invalid.push(s);
    for s in invalid {
        let mut t = table();
        let mut v = DataTableView::new();
        render(&mut v, &t);
        let p = row_point(&v, &t, 1);
        v.pointer_down_keyed(p, &mut t);
        let model = format!("{t:?}");
        let view = format!("{v:?}");
        assert!(t.replace_keyed(s).is_err());
        assert_eq!(format!("{t:?}"), model);
        assert_eq!(format!("{v:?}"), view);
        assert_eq!(v.pointer_up_keyed(p, &mut t), E::Handled);
    }
    assert_eq!(
        KeyedDataTable::new(
            snapshot(),
            TableBudget {
                max_units: 0,
                max_utf8_bytes: 1
            }
        )
        .unwrap_err(),
        TableUpdateError::InvalidBudget
    );
    assert_eq!(
        KeyedDataTable::new(
            snapshot(),
            TableBudget {
                max_units: 1,
                max_utf8_bytes: 0
            }
        )
        .unwrap_err(),
        TableUpdateError::InvalidBudget
    );
}
#[test]
fn old_coordinates_keys_and_legacy_paths_are_rejected_before_arrange() {
    let mut t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    let p = row_point(&v, &t, 1);
    assert_eq!(v.pointer_down_keyed(p, &mut t), E::Changed);
    let mut s = snapshot();
    s.display_rows.reverse();
    t.replace_keyed(s).unwrap();
    let before = format!("{v:?}");
    assert!(v.layout_keyed(&t).is_none());
    assert!(v.focused_row_keyed(&t).is_none());
    assert_eq!(format!("{v:?}"), before);
    assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
    assert_eq!(v.key_keyed(ControlKey::Down, true, &mut t), E::Ignored);
    assert!(!v.pointer_move_keyed(p, &t));
    assert!(!v.focus_keyed(true, &t));
    assert!(v.layout().rows.is_empty());
    assert!(!v.pointer_move(p));
    v.focus_row(2, true);
    v.focus_header(0, true);
    let mut legacy = DataTable::new(3, 2);
    assert_eq!(v.pointer_down(p, &mut legacy), E::Ignored);
    assert_eq!(v.pointer_up(p, &mut legacy), E::Ignored);
    assert_eq!(v.key(ControlKey::Enter, true, &mut legacy), E::Ignored);
    assert_eq!(legacy.selected_row, None);
    render(&mut v, &t);
    assert_eq!(v.focused_row_keyed(&t), Some(TableRowKey(1)));
    assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
}
#[test]
fn multi_update_aba_clone_and_whole_replacement_revoke_focus_capture() {
    // No View call occurs between the two replacements.
    for mode in 0..7 {
        let mut t = table();
        let mut v = DataTableView::new();
        render(&mut v, &t);
        let p = row_point(&v, &t, 1);
        v.pointer_down_keyed(p, &mut t);
        match mode {
            0 => {
                let mut s = snapshot();
                s.display_rows.retain(|k| *k != TableRowKey(1));
                t.replace_keyed(s).unwrap();
                t.replace_keyed(snapshot()).unwrap();
            }
            1 => {
                let mut s = snapshot();
                s.rows.remove(0);
                s.display_rows.retain(|k| *k != TableRowKey(1));
                t.replace_keyed(s).unwrap();
                t.replace_keyed(snapshot()).unwrap();
            }
            2 => {
                let mut s = snapshot();
                s.dataset = TableDatasetKey(2);
                t.replace_keyed(s).unwrap();
                t.replace_keyed(snapshot()).unwrap();
            }
            3 => {
                t = t.clone();
            }
            4 => {
                t = table();
            }
            5 => {
                let mut s = snapshot();
                s.rows[0].key = TableRowKey(9);
                s.display_rows.iter_mut().for_each(|k| {
                    if *k == TableRowKey(1) {
                        *k = TableRowKey(9)
                    }
                });
                t.replace_keyed(s).unwrap();
                t.replace_keyed(snapshot()).unwrap();
            }
            _ => {
                let mut s = snapshot();
                s.columns.clear();
                for r in &mut s.rows {
                    r.cells.clear();
                }
                t.replace_keyed(s).unwrap();
                t.replace_keyed(snapshot()).unwrap();
            }
        }
        assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored, "mode {mode}");
        render(&mut v, &t);
        assert_eq!(v.focused_row_keyed(&t), None, "mode {mode}");
        assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
    }
}
#[test]
fn column_reorder_keeps_focus_but_shrink_and_aba_never_redirect() {
    for mode in 0..4 {
        let mut t = table();
        let mut v = DataTableView::new();
        render(&mut v, &t);
        assert!(v.focus_header_keyed(TableColumnKey(1), true, &t));
        let mut s = snapshot();
        match mode {
            0 => {
                s.columns.reverse();
                for r in &mut s.rows {
                    r.cells.reverse();
                }
            }
            1 | 2 => {
                s.columns.remove(0);
                for r in &mut s.rows {
                    r.cells.remove(0);
                }
            }
            _ => {
                s.columns[0].key = TableColumnKey(9);
            }
        }
        t.replace_keyed(s).unwrap();
        if mode == 2 {
            t.replace_keyed(snapshot()).unwrap();
        }
        assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut t), E::Ignored);
        render(&mut v, &t);
        assert_eq!(
            v.focused_header_keyed(&t),
            if mode == 0 {
                Some(TableColumnKey(1))
            } else {
                None
            }
        );
        assert_eq!(t.sort_key_state(), None);
        if mode == 0 {
            assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut t), E::Changed);
            assert_eq!(
                t.sort_key_state(),
                Some((TableColumnKey(1), SortDirection::Ascending))
            );
        }
    }
}
#[test]
fn pointer_timing_keyboard_navigation_cancel_and_sort_use_original_reducer() {
    let mut t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    let p = row_point(&v, &t, 3);
    assert_eq!(v.pointer_down_keyed(p, &mut t), E::Changed);
    assert_eq!(t.selected_key(), Some(TableRowKey(3)));
    assert_eq!(v.pointer_up_keyed(p, &mut t), E::Handled);
    assert_eq!(
        v.key_keyed(ControlKey::Enter, true, &mut t),
        E::Activated(2)
    );
    assert_eq!(v.key_keyed(ControlKey::Down, true, &mut t), E::Changed);
    assert_eq!(t.selected_key(), Some(TableRowKey(1)));
    assert_eq!(v.key_keyed(ControlKey::End, true, &mut t), E::Changed);
    assert_eq!(t.selected_key(), Some(TableRowKey(2)));
    v.pointer_down_keyed(p, &mut t);
    assert_eq!(v.key_keyed(ControlKey::Escape, true, &mut t), E::Handled);
    assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
    assert_eq!(t.selected_key(), Some(TableRowKey(3)));
    assert!(v.focus_row_keyed(TableRowKey(1), true, &t));
    assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut t), E::Changed);
    assert_eq!(
        v.key_keyed(ControlKey::Enter, true, &mut t),
        E::Activated(0)
    );
    let h = v.layout_keyed(&t).unwrap().headers[0].frame;
    let hp = Point::new(h.min_x() + 5., h.min_y() + 5.);
    assert_eq!(v.pointer_down_keyed(hp, &mut t), E::Changed);
    assert_eq!(
        t.sort_key_state(),
        Some((TableColumnKey(1), SortDirection::Ascending))
    );
    assert!(v.layout_keyed(&t).is_none());
    assert_eq!(v.pointer_up_keyed(hp, &mut t), E::Ignored);
    render(&mut v, &t);
    assert_eq!(v.key_keyed(ControlKey::Enter, true, &mut t), E::Changed);
    assert_eq!(
        t.sort_key_state(),
        Some((TableColumnKey(1), SortDirection::Descending))
    );
    // Metadata never reorders business data.
    assert_eq!(t.snapshot().display_rows, snapshot().display_rows);
}
#[test]
fn keyed_layout_matches_legacy_geometry_and_default_measurement() {
    for frame in [
        bounds(),
        Rect::new(0., 0., 0., 0.),
        Rect::new(0., 0., 7., 100.),
        Rect::new(0., 0., 300., 22.),
    ] {
        for clip in [
            None,
            Some(Rect::new(0., 500., 100., 100.)),
            Some(Rect::new(0., 0., 100., 70.)),
        ] {
            let t = table();
            let mut keyed = DataTableView::new();
            let mut legacy = DataTableView::new();
            let headers = t
                .snapshot()
                .columns
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>();
            let rows = t
                .snapshot()
                .rows
                .iter()
                .map(|r| r.cells.clone())
                .collect::<Vec<_>>();
            let a = keyed.render_keyed(
                &mut Scene::new(),
                &mut Text,
                &t,
                frame,
                clip,
                Theme::LIGHT.into(),
            );
            let b = legacy.render(
                &mut Scene::new(),
                &mut Text,
                t.original(),
                &headers,
                &rows,
                t.display_indices(),
                frame,
                clip,
                Theme::LIGHT.into(),
            );
            assert_eq!(a, b);
        }
    }
    let t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    let l = v.layout_keyed(&t).unwrap();
    assert_eq!(l.headers[0].frame.size.height, 34.);
    assert_eq!(l.rows[0].frame.min_y(), 40.);
    assert_eq!(l.rows[1].frame.min_y(), 78.);
    assert_eq!(l.columns[0].size.width, 150.);
}
#[test]
fn zero_rows_columns_disabled_and_window_resume_do_not_restore_focus() {
    let mut t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    v.focus_keyed(true, &t);
    v.set_disabled(true);
    assert_eq!(v.focused_row_keyed(&t), None);
    assert_eq!(v.key_keyed(ControlKey::Down, true, &mut t), E::Ignored);
    v.set_disabled(false);
    assert_eq!(v.focused_row_keyed(&t), None);
    v.focus_keyed(true, &t);
    v.set_window_active(false);
    v.set_window_active(true);
    assert_eq!(v.focused_row_keyed(&t), None);
    let mut s = snapshot();
    s.columns.clear();
    for r in &mut s.rows {
        r.cells.clear();
    }
    t.replace_keyed(s).unwrap();
    render(&mut v, &t);
    assert!(v.layout_keyed(&t).unwrap().rows.is_empty());
    assert!(!v.focus_keyed(true, &t));
    assert_eq!(
        v.pointer_down_keyed(Point::new(5., 45.), &mut t),
        E::Ignored
    );
    let mut s = snapshot();
    s.rows.clear();
    s.display_rows.clear();
    t.replace_keyed(s).unwrap();
    render(&mut v, &t);
    assert!(!v.focus_keyed(true, &t));
    assert!(v.focus_header_keyed(TableColumnKey(1), true, &t));
}
#[test]
fn hidden_retained_cells_and_utf8_bytes_are_budgeted() {
    let mut s = snapshot();
    s.display_rows.clear();
    let units = 3 + 2 + 3 * 2;
    assert!(
        KeyedDataTable::new(
            s.clone(),
            TableBudget {
                max_units: units - 1,
                max_utf8_bytes: 1000
            }
        )
        .is_err()
    );
    assert!(
        KeyedDataTable::new(
            s.clone(),
            TableBudget {
                max_units: units,
                max_utf8_bytes: 1000
            }
        )
        .is_ok()
    );
    s.rows[0].cells[0] = "汉".into();
    let bytes = s.columns.iter().map(|c| c.name.len()).sum::<usize>()
        + s.rows
            .iter()
            .map(|r| r.name.len() + r.cells.iter().map(String::len).sum::<usize>())
            .sum::<usize>();
    assert!(
        KeyedDataTable::new(
            s.clone(),
            TableBudget {
                max_units: 1000,
                max_utf8_bytes: bytes - 1
            }
        )
        .is_err()
    );
    assert!(
        KeyedDataTable::new(
            s,
            TableBudget {
                max_units: 1000,
                max_utf8_bytes: bytes
            }
        )
        .is_ok()
    );
}

#[test]
fn same_key_reorder_keeps_focus_across_many_unobserved_snapshots() {
    let mut t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    assert!(v.focus_row_keyed(TableRowKey(1), true, &t));
    for i in 0..4 {
        let mut s = snapshot();
        if i % 2 == 0 {
            s.rows.reverse();
            s.columns.reverse();
            for r in &mut s.rows {
                r.cells.reverse();
            }
        }
        s.display_rows.rotate_left(i % 3);
        s.rows[0].name = format!("label {i}");
        t.replace_keyed(s).unwrap();
    }
    render(&mut v, &t);
    assert_eq!(v.focused_row_keyed(&t), Some(TableRowKey(1)));
    let before = format!("{v:?}");
    let revision = t.revision();
    assert!(t.select_key(TableRowKey(2)));
    assert_eq!(t.revision(), revision);
    assert!(v.layout_keyed(&t).is_some());
    assert_eq!(format!("{v:?}"), before);
    assert!(t.sort_key(TableColumnKey(1)));
    assert!(v.layout_keyed(&t).is_none());
    render(&mut v, &t);
    assert_eq!(v.focused_row_keyed(&t), Some(TableRowKey(1)));
}

#[test]
fn capture_never_retargets_after_reorder_delete_or_same_index_new_key() {
    for mode in 0..3 {
        let mut t = table();
        let mut v = DataTableView::new();
        render(&mut v, &t);
        let p = row_point(&v, &t, 1);
        v.pointer_down_keyed(p, &mut t);
        let mut s = snapshot();
        match mode {
            0 => s.display_rows.reverse(),
            1 => {
                s.rows.remove(0);
                s.display_rows.retain(|k| *k != TableRowKey(1));
            }
            _ => {
                s.rows[0].key = TableRowKey(9);
                s.display_rows.iter_mut().for_each(|k| {
                    if *k == TableRowKey(1) {
                        *k = TableRowKey(9)
                    }
                });
            }
        }
        t.replace_keyed(s).unwrap();
        let selected = t.selected_key();
        assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
        render(&mut v, &t);
        assert_eq!(v.pointer_up_keyed(p, &mut t), E::Ignored);
        assert_eq!(t.selected_key(), selected);
        assert_eq!(
            v.focused_row_keyed(&t),
            if mode == 0 {
                Some(TableRowKey(1))
            } else {
                None
            }
        );
    }
}

#[test]
fn full_clip_preserves_logical_focus_without_pointer_hits() {
    let mut t = table();
    let mut v = DataTableView::new();
    render(&mut v, &t);
    assert!(v.focus_row_keyed(TableRowKey(1), true, &t));
    v.render_keyed(
        &mut Scene::new(),
        &mut Text,
        &t,
        bounds(),
        Some(Rect::new(0., 500., 100., 100.)),
        Theme::LIGHT.into(),
    );
    assert_eq!(v.focused_row_keyed(&t), Some(TableRowKey(1)));
    assert!(
        v.layout_keyed(&t)
            .unwrap()
            .rows
            .iter()
            .all(|r| r.visible.is_none())
    );
    assert_eq!(
        v.pointer_down_keyed(Point::new(5., 45.), &mut t),
        E::Ignored
    );
    render(&mut v, &t);
    assert_eq!(v.focused_row_keyed(&t), Some(TableRowKey(1)));
}

#[test]
fn multiline_measurement_is_shared_with_original_layout() {
    struct Wrap;
    impl ButtonTextRenderer for Wrap {
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
            Size::new(
                w,
                (s.len() as f32 * 7. / w.max(1.)).ceil().max(1.) * r.style().line_height,
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
    let mut s = snapshot();
    s.columns[0].name = "header ".repeat(12);
    s.rows[0].cells[1] = "cell ".repeat(30);
    let t = KeyedDataTable::new(s, budget()).unwrap();
    let headers = t
        .snapshot()
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();
    let rows = t
        .snapshot()
        .rows
        .iter()
        .map(|r| r.cells.clone())
        .collect::<Vec<_>>();
    let mut v = DataTableView::new();
    let a = v.render_keyed(
        &mut Scene::new(),
        &mut Wrap,
        &t,
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    let b = DataTableView::new().render(
        &mut Scene::new(),
        &mut Wrap,
        t.original(),
        &headers,
        &rows,
        t.display_indices(),
        bounds(),
        None,
        Theme::LIGHT.into(),
    );
    assert_eq!(a, b);
    assert!(a.headers[0].frame.size.height > 34.);
    assert!(a.rows.iter().find(|r| r.id == 0).unwrap().frame.size.height > 34.);
}
