//! Public finite List consumer: owns keys, text and tree construction only.
use tekes_ui_controls::{ScrollArea, composition::*, extended::*};
pub const LIST: ComponentId = ComponentId(10);
pub const COMPOSER: ComponentId = ComponentId(20);
pub const READER: ComponentId = ComponentId(30);
pub fn snapshot(reordered: bool) -> KeyedListSnapshot {
    KeyedListSnapshot {
        dataset: ListDatasetKey(1),
        items: (1..=3)
            .map(|k| KeyedListItem {
                key: ListItemKey(k),
                name: format!("Entity {k}"),
            })
            .collect(),
        display_items: if reordered {
            vec![ListItemKey(3), ListItemKey(1), ListItemKey(2)]
        } else {
            (1..=3).map(ListItemKey).collect()
        },
    }
}
pub fn rows(s: &KeyedListSnapshot) -> Vec<ListRowDisplay> {
    s.items
        .iter()
        .map(|r| ListRowDisplay {
            key: r.key,
            row_id: ComponentId(100 + r.key.0),
            display: ListDisplayUpdate::Keep,
        })
        .collect()
}
/// A consumer-defined passive wrapper opting into the same generic hook contract
/// used by Table. This demonstrates support without a component-name whitelist.
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
        self.body.arrange(t, f, c)
    }
}
pub fn build() -> Result<ContentHost, ListContentError> {
    let s = snapshot(false);
    let mut d = rows(&s);
    d[0].display = ListDisplayUpdate::Replace(Content::new(
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
                        TextContent::new("Text with Icon\nsecond line"),
                    ),
                ],
            },
        },
    ));
    let list = SelectableListContent::keyed(
        "Public list",
        &s,
        &mut d,
        ListBudget {
            max_items: 100,
            max_display_items: 100,
            max_utf8_bytes: 1000000,
            max_staging_units: 100000,
        },
    )?;
    let mut h = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), TextContent::new("Before list")),
                Content::new(
                    READER,
                    ScrollContent::new(ScrollArea::default(), Content::new(LIST, list)),
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(ComponentId(3), TextContent::new("After list")),
            ],
        },
    ))?;
    h.set_scroll_reading_document(
        READER,
        "entities",
        1,
        vec![ComponentId(101), ComponentId(102), ComponentId(103)],
        |_| (),
    )
    .map_err(ListContentError::Reading)?;
    Ok(h)
}
pub fn replace(
    h: &mut ContentHost,
    s: &KeyedListSnapshot,
    d: &mut Vec<ListRowDisplay>,
) -> Result<(), ListContentError> {
    let expected = h
        .get(READER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone();
    h.replace_keyed_list(
        LIST,
        s,
        d,
        Some(&ListReadingUpdate {
            owner: READER,
            revision: expected.revision() + 1,
            expected,
            items: s
                .display_items
                .iter()
                .map(|k| ComponentId(100 + k.0))
                .collect(),
        }),
    )?;
    Ok(())
}
pub fn reorder(h: &mut ContentHost) -> Result<(), ListContentError> {
    let s = snapshot(true);
    replace(h, &s, &mut rows(&s))
}
