//! An independent public Table/Composer consumer. Business data, order and IDs
//! belong here; all measurement, focus, reading and interactions belong to Kit.
use tekes_ui_controls::{ScrollArea, composition::*, extended::*};
pub const TABLE: ComponentId = ComponentId(10);
pub const COMPOSER: ComponentId = ComponentId(20);
pub const READER: ComponentId = ComponentId(30);
pub fn snapshot(reordered: bool) -> KeyedTableSnapshot {
    KeyedTableSnapshot {
        dataset: TableDatasetKey(1),
        columns: vec![
            KeyedTableColumn {
                key: TableColumnKey(1),
                name: "Name".into(),
            },
            KeyedTableColumn {
                key: TableColumnKey(2),
                name: "Status".into(),
            },
        ],
        rows: (1..=3)
            .map(|k| KeyedTableRow {
                key: TableRowKey(k),
                name: format!("Entity {k}"),
                cells: vec![format!("Item {k}"), "Ready".into()],
            })
            .collect(),
        display_rows: if reordered {
            vec![TableRowKey(3), TableRowKey(1), TableRowKey(2)]
        } else {
            vec![TableRowKey(1), TableRowKey(2), TableRowKey(3)]
        },
    }
}
pub fn displays(s: &KeyedTableSnapshot) -> TableDisplays {
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
/// Consumer-owned wrapper: this stronger contract is required only for typed
/// Table decoration. Its complete child tree is built before submission; these
/// hooks and Drop never create identities, mutate topology, or reenter. A lazy
/// component may still use ordinary Host, or materialize its tree before opt-in.
#[derive(Debug)]
pub struct ConsumerDecoration {
    pub body: StackContent,
}
impl Component for ConsumerDecoration {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn children(&self) -> &[Content] {
        &self.body.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.body.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        a: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.body.measure(t, a)
    }
    fn arrange(
        &mut self,
        t: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        f: tekes_ui_scene::Rect,
        c: tekes_ui_scene::Rect,
    ) {
        self.body.arrange(t, f, c);
    }
}
pub fn build() -> Result<ContentHost, TableContentError> {
    let s = snapshot(false);
    let mut d = displays(&s);
    d.headers[0].display = TableDisplayUpdate::Replace(Content::new(
        ComponentId(190),
        TextContent::new("Custom header\nsecond line"),
    ));
    d.rows[0].cells[0].display = TableDisplayUpdate::Replace(Content::new(
        ComponentId(200),
        ConsumerDecoration {
            body: StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: 6.,
                },
                flexible_child: Some(ComponentId(202)),
                children: vec![
                    Content::new(
                        ComponentId(201),
                        IconContent::new("tekes-settings").unwrap(),
                    ),
                    Content::new(
                        ComponentId(202),
                        TextContent::new("Text with Icon\nmore text"),
                    ),
                ],
            },
        },
    ));
    d.rows[0].cells[1].accessory =
        TableDisplayUpdate::Replace(Content::new(ComponentId(203), TextContent::new("✓")));
    let table = DataTableContent::new(
        "Public data",
        &s,
        &mut d,
        TableBudget {
            max_units: 100_000,
            max_utf8_bytes: 100_000_000,
        },
    )?;
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), TextContent::new("Before table")),
                Content::new(
                    READER,
                    ScrollContent::new(ScrollArea::default(), Content::new(TABLE, table)),
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(ComponentId(3), TextContent::new("After table")),
            ],
        },
    ))?;
    host.set_scroll_reading_document(
        READER,
        "entities",
        1,
        vec![ComponentId(101), ComponentId(102), ComponentId(103)],
        |_| (),
    )
    .map_err(TableContentError::Reading)?;
    Ok(host)
}
pub fn replace(
    host: &mut ContentHost,
    s: &KeyedTableSnapshot,
    d: &mut TableDisplays,
) -> Result<(), TableContentError> {
    let scroll = host
        .get(READER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    let v = scroll.reading_document_version().unwrap().clone();
    let items = if s.columns.is_empty() {
        vec![]
    } else {
        s.display_rows
            .iter()
            .map(|k| ComponentId(100 + k.0))
            .collect()
    };
    host.replace_table_snapshot(
        TABLE,
        s,
        d,
        Some(&TableReadingUpdate {
            owner: READER,
            revision: v.revision() + 1,
            expected: v,
            items,
        }),
    )?;
    Ok(())
}
pub fn reorder(host: &mut ContentHost) -> Result<(), TableContentError> {
    let s = snapshot(true);
    let mut d = displays(&s);
    replace(host, &s, &mut d)
}
