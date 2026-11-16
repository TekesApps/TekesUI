# DataTable

Use DataTableContent with KeyedTableSnapshot for a finite table with stable row and column identities.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::*};
fn mount_table(snapshot: &KeyedTableSnapshot, displays: &mut TableDisplays)
    -> Result<DataTableContent, TableContentError>
{
    DataTableContent::new("Documents", snapshot, displays,
        TableBudget { max_units: 100_000, max_utf8_bytes: 100_000_000 })
}
```

## State and events

Snapshots contain dataset, columns, rows and display_rows. TableDisplays declares Keep/Replace displays and attachments. Preserve wrapper ComponentIds when sorting continuing rows. model, selected_key and sort_key_state read state; host.replace_table_snapshot performs mounted updates, optionally with TableReadingUpdate.

## Composition and limitations

TableIntent identifies Header or Row targets and selection, activation or sorting requests. Check intent_is_current before delayed use. The application owns data and display order; ScrollContent provides a viewport. Passive decorations and attachment slots must satisfy transaction contracts. This is not a spreadsheet, remote sorting service or unbounded data source.

## Related APIs

[Grid](grid.md)
