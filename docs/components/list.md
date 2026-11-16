# List

Use ListContent for ordinary vertical content, and SelectableListContent::keyed for a finite collection with stable business selection.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let list = ListContent::scrollable(
    "Documents",
    ComponentId(2),
    vec![Content::new(ComponentId(3), TextContent::new("Document"))],
);
// To hide the scrollbar: let list = list.scrollbar(false);
```

```rust
use tekes_ui_controls::{composition::*, extended::*};
// Build snapshot: KeyedListSnapshot and rows: Vec<ListRowDisplay> with stable keys.
fn mount_list(snapshot: &KeyedListSnapshot, rows: &mut Vec<ListRowDisplay>)
    -> Result<KeyedSelectableListContent, ListContentError>
{
    SelectableListContent::keyed("Documents", snapshot, rows, ListBudget {
        max_items: 100, max_display_items: 100,
        max_utf8_bytes: 1_000_000, max_staging_units: 100_000,
    })
}
```

## State and events

ListContent::scrollable includes a ScrollArea with shared Loose padding and leading row alignment. content_padding replaces the default. ListContent::new organizes rows or nested groups without another viewport inset. Keyed snapshots separate dataset, items and display_items; preserve each continuing row_id during reordering.

## Composition and limitations

Read selected_key, focused_key and model. Use host.replace_keyed_list and set_keyed_list_selection for mounted changes. ListIntent distinguishes SelectionChanged and Activated and captures dataset/key; intent_is_current supports delayed handling. Use one scroll owner. scrollbar(false) hides the indicator without disabling wheel/reveal. Row decoration must be passive and transaction-compatible; finite collections do not imply remote paging or automatic virtualization.

## Related APIs

[Tree](tree.md)
