//! Master document transport; keyed List and ScrollContent retain all interaction state.
use tekes_ui_controls::{
    composition::*,
    extended::{KeyedListItem, KeyedListSnapshot, ListBudget, ListDatasetKey, ListItemKey},
};

const SCROLL: ComponentId = ComponentId(27401);
const ITEMS: ComponentId = ComponentId(27402);
pub(super) const LIST: ComponentId = ComponentId(27403);
fn row_id(index: usize) -> ComponentId {
    ComponentId(27500 + index as u64)
}

#[derive(Debug)]
pub(super) struct MasterHost {
    pub(super) host: ContentHost,
}
impl Default for MasterHost {
    fn default() -> Self {
        let snapshot = KeyedListSnapshot {
            dataset: ListDatasetKey(274),
            items: (0..80)
                .map(|n| KeyedListItem {
                    key: ListItemKey(n + 1),
                    name: format!("Document {:02}", n + 1),
                })
                .collect(),
            display_items: (1..=80).map(ListItemKey).collect(),
        };
        let mut rows = snapshot
            .items
            .iter()
            .enumerate()
            .map(|(n, item)| ListRowDisplay {
                key: item.key,
                row_id: row_id(n),
                display: ListDisplayUpdate::DefaultLabel,
            })
            .collect();
        let mut list = SelectableListContent::keyed(
            "Documents",
            &snapshot,
            &mut rows,
            ListBudget {
                max_items: 80,
                max_display_items: 80,
                // Fixed fixture allowance for retained/staged text across 80 labels.
                // 4 KiB failed the original List preflight; retain its budget checks.
                max_utf8_bytes: 8 * 1024,
                max_staging_units: 8192,
            },
        )
        .expect("fixed document dataset");
        list.select_key(ListItemKey(1))
            .expect("initial document key");
        let mut host = ContentHost::new(Content::new(
            SCROLL,
            ListContent::scrollable("Documents", ITEMS, vec![Content::new(LIST, list)]),
        ))
        .expect("fixed master List structure");
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self { host }
    }
}
