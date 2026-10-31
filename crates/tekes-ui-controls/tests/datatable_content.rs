use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ScrollArea, composition::*, extended::*,
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const TABLE: ComponentId = ComponentId(10);
const READER: ComponentId = ComponentId(1);
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.len() as f32 * 8.).min(w),
            20. + 20. * s.matches('\n').count() as f32,
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
fn budget() -> TableBudget {
    TableBudget {
        max_units: 100_000,
        max_utf8_bytes: 100_000_000,
    }
}
fn snapshot() -> KeyedTableSnapshot {
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: vec![
            KeyedTableColumn {
                key: TableColumnKey(1),
                name: "name".into(),
            },
            KeyedTableColumn {
                key: TableColumnKey(2),
                name: "value".into(),
            },
        ],
        rows: (1..=3)
            .map(|k| KeyedTableRow {
                key: TableRowKey(k),
                name: format!("row{k}"),
                cells: vec![format!("r{k}"), "value".into()],
            })
            .collect(),
        display_rows: (1..=3).map(TableRowKey).collect(),
    }
}
fn displays(s: &KeyedTableSnapshot) -> TableDisplays {
    TableDisplays {
        headers: s
            .columns
            .iter()
            .map(|c| TableHeaderDisplay {
                key: c.key,
                display: TableDisplayUpdate::Keep,
            })
            .collect(),
        rows: s
            .rows
            .iter()
            .map(|r| TableRowDisplay {
                key: r.key,
                id: ComponentId(100 + r.key.0),
                replace_wrapper: false,
                cells: s
                    .columns
                    .iter()
                    .map(|c| TableCellDisplay {
                        column: c.key,
                        display: TableDisplayUpdate::Keep,
                        accessory: TableDisplayUpdate::Keep,
                    })
                    .collect(),
            })
            .collect(),
    }
}
fn build(custom: bool, reading: bool) -> ContentHost {
    let s = snapshot();
    let mut d = displays(&s);
    if custom {
        d.rows[0].cells[0].display = TableDisplayUpdate::Replace(Content::new(
            ComponentId(200),
            TextContent::new("custom\nsecond\nthird"),
        ));
        d.rows[0].cells[1].accessory =
            TableDisplayUpdate::Replace(Content::new(ComponentId(201), TextContent::new("tail")));
    }
    let table = Content::new(
        TABLE,
        DataTableContent::new("Table", &s, &mut d, budget()).unwrap(),
    );
    let mut h = ContentHost::new(Content::new(
        READER,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                ComponentId(2),
                ListContent::new(
                    "body",
                    vec![
                        table,
                        Content::new(ComponentId(900), TextContent::new("sibling")),
                    ],
                ),
            ),
        ),
    ))
    .unwrap();
    if reading {
        h.set_scroll_reading_document(
            READER,
            "A",
            1,
            vec![ComponentId(101), ComponentId(102), ComponentId(103)],
            |_| (),
        )
        .unwrap();
    }
    h.set_action_events(true);
    paint(&mut h);
    h
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 360., 230.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn t(h: &ContentHost) -> &DataTableContent {
    h.get(TABLE)
        .unwrap()
        .component::<DataTableContent>()
        .unwrap()
}
fn reader(h: &ContentHost) -> &ScrollContent {
    h.get(READER).unwrap().component::<ScrollContent>().unwrap()
}
fn update(
    h: &mut ContentHost,
    s: KeyedTableSnapshot,
) -> Result<Option<tekes_ui_controls::ReadingDocumentVersion>, TableContentError> {
    let mut d = displays(&s);
    let r = reader(h)
        .reading_document_version()
        .map(|v| TableReadingUpdate {
            owner: READER,
            expected: v.clone(),
            revision: v.revision() + 1,
            items: if s.columns.is_empty() {
                vec![]
            } else {
                s.display_rows
                    .iter()
                    .map(|k| ComponentId(100 + k.0))
                    .collect()
            },
        });
    h.replace_table_snapshot(TABLE, &s, &mut d, r.as_ref())
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == TABLE)
        .unwrap()
        .parts
}
fn part(h: &ContentHost, role: ContentRole) -> SemanticPart {
    parts(h)
        .into_iter()
        .find(|p| p.semantics.role == role)
        .unwrap()
}
fn press(h: &mut ContentHost, role: ContentRole) -> ContentResponse {
    let p = part(h, role);
    h.accessibility_action(TABLE, Some(p.index), AccessibilityAction::Press)
}
fn intent(h: &mut ContentHost) -> TableIntent {
    match h.drain_action_records().remove(0).payload.unwrap() {
        ContentActionPayload::TableIntent(i) => i,
        _ => panic!("table intent"),
    }
}
#[test]
fn table_family_parent_ordinals_and_readonly_cells_use_original_row_frames() {
    let mut h = build(true, false);
    let p = parts(&h);
    assert_eq!(p.len(), 2 * 2 + 3 + 6);
    let col = part(&h, ContentRole::TableColumn { index: 0 });
    let header = part(
        &h,
        ContentRole::TableColumnHeader {
            index: 0,
            sort: None,
        },
    );
    assert_eq!(h.semantic_part_parent(TABLE, header.index), Some(col.index));
    let row = part(&h, ContentRole::TableRow { index: 0 });
    let cell = part(&h, ContentRole::TableCell { row: 0, column: 0 });
    assert_eq!(h.semantic_part_parent(TABLE, cell.index), Some(row.index));
    assert_eq!(row.frame, h.get(ComponentId(101)).unwrap().frame());
    assert!(row.frame.size.height >= 60.);
    assert!(
        h.semantics()
            .iter()
            .all(|n| n.id != ComponentId(200) && n.id != ComponentId(201))
    );
    for p in [col, cell] {
        let before = format!("{:?}", t(&h));
        for a in [
            AccessibilityAction::Focus,
            AccessibilityAction::Press,
            AccessibilityAction::SetExpanded(true),
        ] {
            assert!(!h.accessibility_action(TABLE, Some(p.index), a).consumed);
        }
        assert_eq!(format!("{:?}", t(&h)), before);
    }
}
#[test]
fn keep_reorder_retains_wrapper_display_token_and_intent_and_reader() {
    let mut h = build(true, true);
    assert!(press(&mut h, ContentRole::TableRow { index: 0 }).consumed);
    let i = intent(&mut h);
    press(&mut h, ContentRole::TableRow { index: 0 });
    let token = part(&h, ContentRole::TableRow { index: 0 }).index;
    let row = h.semantic_mount(ComponentId(101));
    let display = h.semantic_mount(ComponentId(200));
    let before = reader(&h).reading_document_version().unwrap().clone();
    let mut s = snapshot();
    s.display_rows.reverse();
    update(&mut h, s).unwrap();
    assert!(t(&h).intent_is_current(&i));
    assert_eq!(h.drain_action_records().len(), 1);
    paint(&mut h);
    assert_eq!(t(&h).selected_key(), Some(TableRowKey(1)));
    assert_eq!(part(&h, ContentRole::TableRow { index: 2 }).index, token);
    assert_eq!(h.semantic_mount(ComponentId(101)), row);
    assert_eq!(h.semantic_mount(ComponentId(200)), display);
    assert_eq!(
        reader(&h).reading_document_version().unwrap().attachment(),
        before.attachment()
    );
    assert_eq!(
        reader(&h).reading_items(),
        &[ComponentId(103), ComponentId(102), ComponentId(101)]
    );
}
#[test]
fn hiding_removing_disabled_dataset_and_activity_aba_revoke_queued_and_drained() {
    for case in 0..6 {
        let mut h = build(false, false);
        press(&mut h, ContentRole::TableRow { index: 0 });
        let i = intent(&mut h);
        press(&mut h, ContentRole::TableRow { index: 1 });
        match case {
            0 => {
                let mut s = snapshot();
                s.display_rows.remove(0);
                update(&mut h, s).unwrap();
                update(&mut h, snapshot()).unwrap();
            }
            1 => {
                let mut s = snapshot();
                s.rows.remove(0);
                s.display_rows.remove(0);
                update(&mut h, s).unwrap();
                update(&mut h, snapshot()).unwrap();
            }
            2 => {
                h.get_mut(TABLE)
                    .unwrap()
                    .component_mut::<DataTableContent>()
                    .unwrap()
                    .set_disabled(true)
                    .unwrap();
                h.get_mut(TABLE)
                    .unwrap()
                    .component_mut::<DataTableContent>()
                    .unwrap()
                    .set_disabled(false)
                    .unwrap();
            }
            3 => {
                let mut s = snapshot();
                s.dataset = TableDatasetKey(2);
                update(&mut h, s).unwrap();
                update(&mut h, snapshot()).unwrap();
            }
            4 => {
                h.set_window_active(false);
                h.set_window_active(true);
            }
            _ => {
                let s = snapshot();
                let node = h.remove(TABLE).unwrap();
                h.insert(ComponentId(2), node).unwrap();
                assert!(parts(&h).is_empty(), "new mount requires a fresh arrange");
                assert_eq!(t(&h).model().snapshot(), &s);
            }
        }
        paint(&mut h);
        assert!(!t(&h).intent_is_current(&i), "case{case}");
        if case != 0 && case != 1 {
            assert!(h.drain_action_records().is_empty(), "case{case}");
        }
    }
}
#[test]
fn queries_are_pure_and_zero_frame_actions_fail_without_reveal() {
    let mut h = build(false, true);
    let f = Rect::new(0., 0., 360., 230.);
    let clip = Rect::new(0., 220., 360., 10.);
    h.render(&mut Scene::new(), &mut Text, f, clip, Theme::LIGHT.into())
        .unwrap();
    let before = format!("{h:?}");
    for _ in 0..8 {
        let p = parts(&h);
        for p in p {
            let _ = h.semantic_part_parent(TABLE, p.index);
            let _ = h.semantic_mount(TABLE);
        }
    }
    assert_eq!(format!("{h:?}"), before);
    for p in parts(&h) {
        if p.frame.is_empty() {
            assert!(
                !h.accessibility_action(TABLE, Some(p.index), AccessibilityAction::Press)
                    .consumed
            );
            assert!(
                !h.accessibility_action(TABLE, Some(p.index), AccessibilityAction::Focus)
                    .consumed
            );
        }
    }
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn failed_models_slots_global_ids_and_reading_versions_leave_entire_host_unchanged() {
    for case in 0..7 {
        let mut h = build(true, true);
        let row = part(&h, ContentRole::TableRow { index: 0 });
        h.event(&ContentEvent::PointerDown(Point::new(
            row.frame.min_x() + 5.,
            row.frame.min_y() + 5.,
        )));
        let before = format!("{h:?}");
        let mut s = snapshot();
        let mut d = displays(&s);
        let v = reader(&h).reading_document_version().unwrap().clone();
        let mut r = TableReadingUpdate {
            owner: READER,
            expected: v,
            revision: 2,
            items: vec![ComponentId(101), ComponentId(102), ComponentId(103)],
        };
        match case {
            0 => s.rows[0].cells.clear(),
            1 => d.rows[1].id = ComponentId(101),
            2 => {
                d.rows[2].cells[0].display = TableDisplayUpdate::Replace(Content::new(
                    ComponentId(900),
                    TextContent::new("conflict"),
                ))
            }
            3 => {
                d.rows[0].cells[0].display = TableDisplayUpdate::Replace(Content::new(
                    ComponentId(777),
                    Button::new("interactive"),
                ))
            }
            4 => r.revision = 1,
            5 => r.items.reverse(),
            _ => r.owner = TABLE,
        }
        assert!(
            h.replace_table_snapshot(TABLE, &s, &mut d, Some(&r))
                .is_err(),
            "case{case}"
        );
        assert_eq!(format!("{h:?}"), before, "case{case}");
    }
}
#[test]
fn hidden_keep_descendant_conflict_is_checked_even_when_not_painted() {
    let mut h = build(true, false);
    let mut s = snapshot();
    s.display_rows.remove(0);
    update(&mut h, s.clone()).unwrap();
    paint(&mut h);
    assert!(h.semantic_mount(ComponentId(200)).is_none());
    let mut d = displays(&s);
    d.headers[0].display = TableDisplayUpdate::Replace(Content::new(
        ComponentId(200),
        TextContent::new("collide hidden Keep"),
    ));
    let before = format!("{h:?}");
    assert!(h.replace_table_snapshot(TABLE, &s, &mut d, None).is_err());
    assert_eq!(format!("{h:?}"), before);
}
#[derive(Debug)]
struct FakePassive {
    children: Vec<Content>,
}
impl Component for FakePassive {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
}
#[test]
fn recursive_capability_check_rejects_fake_passive_input_scroll_and_selection_owners() {
    for c in [
        Content::new(ComponentId(300), Button::new("button")),
        Content::new(ComponentId(300), TextFieldContent::new("input", "x")),
        Content::new(
            ComponentId(300),
            ScrollContent::new(
                ScrollArea::default(),
                Content::new(ComponentId(301), TextContent::new("body")),
            ),
        ),
        Content::new(
            ComponentId(300),
            SelectableListContent::new("selectable", vec![]),
        ),
    ] {
        let s = snapshot();
        let mut d = displays(&s);
        d.rows[0].cells[0].display = TableDisplayUpdate::Replace(Content::new(
            ComponentId(299),
            FakePassive { children: vec![c] },
        ));
        assert!(matches!(
            DataTableContent::new("T", &s, &mut d, budget()),
            Err(TableContentError::InteractiveDisplay {
                row: Some(TableRowKey(1)),
                column: Some(TableColumnKey(1)),
                accessory: false,
                ..
            })
        ));
    }
}
#[test]
fn pointer_down_selects_once_header_sort_is_intent_only_and_cancel_keeps_history() {
    let mut h = build(false, false);
    let row = part(&h, ContentRole::TableRow { index: 0 });
    let p = Point::new(row.frame.min_x() + 5., row.frame.min_y() + 5.);
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::Cancel);
    h.event(&ContentEvent::PointerUp(p));
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert!(matches!(
        records[0].payload,
        Some(ContentActionPayload::TableIntent(TableIntent {
            kind: TableIntentKind::SelectionChanged,
            ..
        }))
    ));
    paint(&mut h);
    assert!(
        press(
            &mut h,
            ContentRole::TableColumnHeader {
                index: 0,
                sort: None
            }
        )
        .consumed
    );
    let intent = intent(&mut h);
    assert_eq!(
        intent.kind,
        TableIntentKind::SortChanged {
            direction: SortDirection::Ascending
        }
    );
    assert_eq!(
        t(&h).model().snapshot().display_rows,
        snapshot().display_rows
    );
    assert!(
        parts(&h).is_empty(),
        "sort invalidates geometry until arrange"
    );
}
#[test]
fn default_geometry_matches_original_view_and_narrow_zero_columns_publish_no_row_action() {
    let mut h = build(false, false);
    let f = h.get(TABLE).unwrap().frame();
    let mut v = tekes_ui_controls::collections_view::DataTableView::new();
    let l = v.render_keyed(
        &mut Scene::new(),
        &mut Text,
        t(&h).model(),
        f,
        Some(Rect::new(0., 0., 360., 230.)),
        Theme::LIGHT.into(),
    );
    assert_eq!(
        part(&h, ContentRole::TableRow { index: 0 }).frame,
        l.rows[0].frame
    );
    let mut s = snapshot();
    s.columns.clear();
    for r in &mut s.rows {
        r.cells.clear();
    }
    update(&mut h, s).unwrap();
    paint(&mut h);
    assert!(parts(&h).is_empty());
    let f = Rect::new(0., 0., 0., 230.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::DARK.into())
        .unwrap();
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn explicit_wrapper_replace_revokes_mount_and_old_token_while_plain_id_change_rejects() {
    let mut h = build(true, false);
    press(&mut h, ContentRole::TableRow { index: 0 });
    let old = intent(&mut h);
    let mount = h.semantic_mount(ComponentId(101));
    let s = snapshot();
    let mut d = displays(&s);
    d.rows[0].id = ComponentId(501);
    let before = format!("{h:?}");
    assert!(
        h.replace_table_snapshot(TABLE, &s.clone(), &mut d, None)
            .is_err()
    );
    assert_eq!(format!("{h:?}"), before);
    let mut d = displays(&s);
    d.rows[0].id = ComponentId(501);
    d.rows[0].replace_wrapper = true;
    h.replace_table_snapshot(TABLE, &s, &mut d, None).unwrap();
    paint(&mut h);
    assert_ne!(h.semantic_mount(ComponentId(501)), mount);
    assert!(h.get(ComponentId(200)).is_none());
    assert!(!t(&h).intent_is_current(&old));
    assert_ne!(
        part(&h, ContentRole::TableRow { index: 0 }).index,
        old.exposure
    );
}
#[test]
fn storage_and_column_reorder_keep_key_identity_separate_from_display_ordinal() {
    let mut h = build(true, false);
    let header = part(
        &h,
        ContentRole::TableColumnHeader {
            index: 0,
            sort: None,
        },
    )
    .index;
    let row = part(&h, ContentRole::TableRow { index: 0 }).index;
    let cell = part(&h, ContentRole::TableCell { row: 0, column: 0 }).index;
    let m = h.semantic_mount(ComponentId(200));
    let mut s = snapshot();
    s.rows.reverse();
    s.columns.reverse();
    for row in &mut s.rows {
        row.cells.reverse();
    }
    update(&mut h, s).unwrap();
    paint(&mut h);
    assert_eq!(part(&h, ContentRole::TableRow { index: 0 }).index, row);
    assert_eq!(
        part(
            &h,
            ContentRole::TableColumnHeader {
                index: 1,
                sort: None
            }
        )
        .index,
        header
    );
    assert_eq!(
        part(&h, ContentRole::TableCell { row: 0, column: 1 }).index,
        cell
    );
    assert_eq!(h.semantic_mount(ComponentId(200)), m);
}
#[test]
fn caller_row_id_reveals_original_arranged_row_and_hidden_target_never_redirects() {
    let mut h = build(false, true);
    let mut s = snapshot();
    for k in 4..30 {
        s.rows.push(KeyedTableRow {
            key: TableRowKey(k),
            name: format!("row{k}"),
            cells: vec!["a".into(), "b".into()],
        });
        s.display_rows.push(TableRowKey(k));
    }
    update(&mut h, s.clone()).unwrap();
    paint(&mut h);
    let version = reader(&h).reading_document_version().unwrap().clone();
    let options = tekes_ui_controls::ScrollRevealOptions {
        animated: false,
        ..Default::default()
    };
    let result = h
        .reveal_scroll_reading_item(READER, &version, ComponentId(129), options, &mut Text)
        .unwrap();
    assert_eq!(result.item, ComponentId(129));
    assert!(reader(&h).scroll.offset.y > 0.);
    s.display_rows.retain(|k| *k != TableRowKey(29));
    update(&mut h, s).unwrap();
    paint(&mut h);
    let current = reader(&h).reading_document_version().unwrap().clone();
    let before = format!("{h:?}");
    assert!(
        h.reveal_scroll_reading_item(READER, &current, ComponentId(129), options, &mut Text)
            .is_err()
    );
    assert_eq!(format!("{h:?}"), before);
    assert!(
        h.reveal_scroll_reading_item(READER, &version, ComponentId(128), options, &mut Text)
            .is_err()
    );
}
#[derive(Debug)]
struct Gate {
    active: std::rc::Rc<std::cell::Cell<bool>>,
    body: ListContent,
}
impl Component for Gate {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }

    fn active(&self) -> bool {
        self.active.get()
    }
    fn children(&self) -> &[Content] {
        self.body.children()
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.body.children_mut()
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, s: Size) -> MeasuredChild {
        self.body.measure(t, s)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.body.arrange(t, f, c)
    }
}
#[test]
fn ancestor_inactivity_and_reactivation_revoke_history_without_a_new_queue() {
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let s = snapshot();
    let table = DataTableContent::new("T", &s.clone(), &mut displays(&s), budget()).unwrap();
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        Gate {
            active: active.clone(),
            body: ListContent::new("gate", vec![Content::new(TABLE, table)]),
        },
    ))
    .unwrap();
    h.set_action_events(true);
    paint(&mut h);
    press(&mut h, ContentRole::TableRow { index: 0 });
    let i = intent(&mut h);
    press(&mut h, ContentRole::TableRow { index: 1 });
    active.set(false);
    paint(&mut h);
    active.set(true);
    paint(&mut h);
    assert!(
        !h.accessibility_action(TABLE, Some(i.exposure), AccessibilityAction::Press)
            .consumed,
        "old AX exposure must not revive after an inactive ancestor"
    );
    assert!(!t(&h).intent_is_current(&i));
    assert!(h.drain_action_records().is_empty());
}
#[derive(Default)]
struct Recording {
    ink: Vec<(String, Rgba)>,
}
impl ButtonTextRenderer for Recording {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text.measure(s, r, w)
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
        self.ink.push((s.into(), c));
    }
}
#[test]
fn passive_slot_selected_disabled_ink_comes_from_original_button_surface_colors() {
    let mut h = build(true, false);
    h.set_inherited_surface("canvas").unwrap();
    press(&mut h, ContentRole::TableRow { index: 0 });
    for disabled in [false, true, false] {
        h.get_mut(TABLE)
            .unwrap()
            .component_mut::<DataTableContent>()
            .unwrap()
            .set_disabled(disabled)
            .unwrap();
        let mut text = Recording::default();
        let f = Rect::new(0., 0., 360., 230.);
        h.render(&mut Scene::new(), &mut text, f, f, Theme::DARK.into())
            .unwrap();
        let color = if disabled {
            // Disabling row actions must not dim readonly cell information.
            tekes_ui_tokens::Palette::from(Theme::DARK)
                .color("primary-text")
                .unwrap()
        } else {
            tekes_ui_tokens::surface_palette::resolve_surface(
                Theme::DARK.into(),
                "canvas",
                "action-quiet",
                "selected",
            )
            .unwrap()
            .text
        };
        assert!(
            text.ink
                .iter()
                .any(|(s, c)| s == "custom\nsecond\nthird" && *c == color),
            "readonly row ink: disabled={disabled}, actual={:?}, expected={color:?}",
            text.ink
        );
    }
}

#[test]
fn typed_reading_plan_preserves_unchanged_sibling_items_and_order() {
    let mut h = build(false, true);
    let v = reader(&h).reading_document_version().unwrap().clone();
    h.update_scroll_reading_document(
        READER,
        &v,
        2,
        vec![
            ComponentId(101),
            ComponentId(102),
            ComponentId(103),
            ComponentId(900),
        ],
        |_| (),
    )
    .unwrap();
    let v = reader(&h).reading_document_version().unwrap().clone();
    let mut s = snapshot();
    s.display_rows.reverse();
    let mut d = displays(&s);
    h.replace_table_snapshot(
        TABLE,
        &s,
        &mut d,
        Some(&TableReadingUpdate {
            owner: READER,
            expected: v,
            revision: 3,
            items: vec![
                ComponentId(103),
                ComponentId(102),
                ComponentId(101),
                ComponentId(900),
            ],
        }),
    )
    .unwrap();
    assert_eq!(reader(&h).reading_items().last(), Some(&ComponentId(900)));
}

#[test]
fn header_ax_focus_keeps_original_keyboard_owner_and_enter_sorts_without_submit() {
    let mut h = build(false, false);
    let header = part(
        &h,
        ContentRole::TableColumnHeader {
            index: 0,
            sort: None,
        },
    );
    assert!(
        h.accessibility_action(TABLE, Some(header.index), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(TABLE));
    assert!(
        part(
            &h,
            ContentRole::TableColumnHeader {
                index: 0,
                sort: None
            }
        )
        .focused
    );
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Enter,
        down: true,
    });
    let record = h.drain_action_records().remove(0);
    assert_ne!(record.action.kind, ContentActionKind::Submit);
    assert!(matches!(
        record.payload,
        Some(ContentActionPayload::TableIntent(TableIntent {
            kind: TableIntentKind::SortChanged { .. },
            target: TableIntentTarget::Header(TableColumnKey(1)),
            ..
        }))
    ));
}

#[test]
fn inactive_window_keeps_original_table_paint_without_allowing_actions() {
    let mut h = build(false, false);
    let old = part(&h, ContentRole::TableRow { index: 0 }).index;
    h.set_window_active(false);
    let mut text = Recording::default();
    let f = Rect::new(0., 0., 360., 230.);
    h.render(&mut Scene::new(), &mut text, f, f, Theme::LIGHT.into())
        .unwrap();
    assert!(text.ink.iter().any(|(s, _)| s == "r1"));
    assert!(
        !h.accessibility_action(TABLE, Some(old), AccessibilityAction::Press)
            .consumed
    );
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn zero_rows_still_enter_original_header_keyboard_sort_without_a_row_owner() {
    let mut h = build(false, false);
    let mut s = snapshot();
    s.rows.clear();
    s.display_rows.clear();
    update(&mut h, s).unwrap();
    paint(&mut h);
    assert!(h.focus(Some(TABLE), true));
    assert!(
        part(
            &h,
            ContentRole::TableColumnHeader {
                index: 0,
                sort: None
            }
        )
        .focused
    );
    h.event(&ContentEvent::Key {
        key: tekes_ui_controls::ControlKey::Enter,
        down: true,
    });
    let i = intent(&mut h);
    assert_eq!(i.target, TableIntentTarget::Header(TableColumnKey(1)));
    assert!(matches!(i.kind, TableIntentKind::SortChanged { .. }));
    assert_eq!(t(&h).selected_key(), None);
}

#[test]
fn matching_numbers_from_another_host_never_authorize_a_drained_intent() {
    let mut a = build(false, false);
    let mut b = build(false, false);
    press(&mut a, ContentRole::TableRow { index: 0 });
    let i = intent(&mut a);
    assert_eq!(
        part(&b, ContentRole::TableRow { index: 0 }).index,
        i.exposure
    );
    assert!(!t(&b).intent_is_current(&i));
    press(&mut b, ContentRole::TableRow { index: 0 });
    let other = intent(&mut b);
    assert_eq!(i.target, other.target);
    assert_eq!(i.dataset, other.dataset);
    assert_ne!(i, other);
}
