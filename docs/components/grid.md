# Grid

Use GridContent for an ordered collection arranged in a fixed number of columns.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::Grid};
let grid = GridContent {
    model: Grid::new(2),
    children: vec![Content::new(ComponentId(2), TextContent::new("Cell"))],
};
```

## State and events

columns sets the column count; gap_x and gap_y set spacing. Children retain their own state and events, and layout follows declaration order.

## Composition and limitations

Provide a parent viewport when rows exceed the available height. Grid does not supply cell selection, sorting or editing. Use DataTable for keyed rows and columns.

## Related APIs

[Data Table](data-table.md)
