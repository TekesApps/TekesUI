# Chart

Use ChartContent for a single-series bar chart built from ChartDatum values.

## Creating the content

```rust
use tekes_ui_controls::{ChartDatum, composition::ChartContent};
let chart = ChartContent::try_new("Sales", vec![
    ChartDatum::new("A", 12.), ChartDatum::new("B", 24.),
]).unwrap().preferred_height(240.);
```

## State and events

try_new rejects empty names and non-finite values. update_data returns a Result and invalidates old layout. data, selected_index and selected_datum read state. Pointer, keyboard and accessibility selection use the host input route; there is no separate chart data action payload.

## Composition and limitations

preferred_height is a preference; the parent determines final size. Host supplies text shaping, Theme and clipping. Read selection after input rather than inventing data on Activate. This entry does not provide line, scatter, multi-series or time-series charts, scrolling or network retrieval.

## Related APIs

[Data Table](data-table.md)
