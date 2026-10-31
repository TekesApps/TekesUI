use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, collections_view::DataTableView, composition::*,
    extended::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text(Vec<(String, Rgba)>);
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        if s.starts_with("cell") {
            self.0.push((s.into(), c));
        }
    }
}
fn setup() -> ContentHost {
    let s = KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: vec![KeyedTableColumn {
            key: TableColumnKey(1),
            name: "Header".into(),
        }],
        rows: (1..=3)
            .map(|k| KeyedTableRow {
                key: TableRowKey(k),
                name: format!("row{k}"),
                cells: vec![format!("cell{k}")],
            })
            .collect(),
        display_rows: (1..=3).map(TableRowKey).collect(),
    };
    let mut d = TableDisplays {
        headers: vec![TableHeaderDisplay {
            key: TableColumnKey(1),
            display: TableDisplayUpdate::Keep,
        }],
        rows: s
            .rows
            .iter()
            .map(|r| TableRowDisplay {
                key: r.key,
                id: ComponentId(100 + r.key.0),
                replace_wrapper: false,
                cells: vec![TableCellDisplay {
                    column: TableColumnKey(1),
                    display: TableDisplayUpdate::Replace(Content::new(
                        ComponentId(200 + r.key.0),
                        TextContent::new(&r.cells[0]),
                    )),
                    accessory: TableDisplayUpdate::Keep,
                }],
            })
            .collect(),
    };
    ContentHost::new(Content::new(
        ComponentId(10),
        DataTableContent::new(
            "Table",
            &s,
            &mut d,
            TableBudget {
                max_units: 10000,
                max_utf8_bytes: 100000,
            },
        )
        .unwrap(),
    ))
    .unwrap()
}
#[test]
fn legacy_and_custom_passive_slots_share_original_row_ink_and_input_states() {
    for theme in Theme::ALL {
        let mut model = DataTable::new(3, 1);
        let mut view = DataTableView::new();
        let mut h = setup();
        let f = Rect::new(0., 0., 300., 200.);
        let rows = (1..=3)
            .map(|k| vec![format!("cell{k}")])
            .collect::<Vec<_>>();
        let paint = |view: &mut DataTableView, model: &DataTable, h: &mut ContentHost| {
            let mut legacy = Text::default();
            let l = view.render(
                &mut Scene::new(),
                &mut legacy,
                model,
                &["Header".into()],
                &rows,
                &[0, 1, 2],
                f,
                Some(f),
                theme.into(),
            );
            let mut slots = Text::default();
            h.render(&mut Scene::new(), &mut slots, f, f, theme.into())
                .unwrap();
            legacy.0.sort_by(|a, b| a.0.cmp(&b.0));
            slots.0.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(legacy.0, slots.0, "actual row state + {theme:?}");
            assert_eq!(legacy.0.len(), 3);
            for (index, row) in l.rows.iter().enumerate() {
                let part = h
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == ComponentId(10))
                    .unwrap()
                    .parts
                    .into_iter()
                    .find(|p| p.semantics.role == ContentRole::TableRow { index })
                    .unwrap();
                assert_eq!(part.frame, row.frame);
            }
        };
        paint(&mut view, &model, &mut h);
        let p = Point::new(30., 55.);
        view.pointer_move(p);
        h.event(&ContentEvent::PointerMove(p));
        paint(&mut view, &model, &mut h);
        view.pointer_down(p, &mut model);
        h.event(&ContentEvent::PointerDown(p));
        paint(&mut view, &model, &mut h);
        assert_eq!(model.selected_row, Some(0));
        assert_eq!(
            h.get(ComponentId(10))
                .unwrap()
                .component::<DataTableContent>()
                .unwrap()
                .selected_key(),
            Some(TableRowKey(1))
        );
        view.pointer_up(p, &mut model);
        h.event(&ContentEvent::PointerUp(p));
        paint(&mut view, &model, &mut h);
        for disabled in [true, false] {
            view.set_disabled(disabled);
            h.get_mut(ComponentId(10))
                .unwrap()
                .component_mut::<DataTableContent>()
                .unwrap()
                .set_disabled(disabled)
                .unwrap();
            paint(&mut view, &model, &mut h);
            assert_eq!(model.selected_row, Some(0));
            assert_eq!(
                h.get(ComponentId(10))
                    .unwrap()
                    .component::<DataTableContent>()
                    .unwrap()
                    .selected_key(),
                Some(TableRowKey(1))
            );
        }
    }
}
