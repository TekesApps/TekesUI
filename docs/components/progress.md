# Progress

Use ProgressContent to display a known fraction or an explicitly indeterminate operation.

## Creating the content

```rust
use tekes_ui_controls::composition::ProgressContent;
let mut progress = ProgressContent::new("Transfer", 0.45);
progress.description = Some("Downloading document".into());
progress.model.indeterminate = false;
```

## State and events

For known progress use a value from 0 to 1. model.value reads it and model.set updates it. model.indeterminate represents unknown progress; do not interpret its retained scalar as zero percent. layout reads prepared geometry.

## Composition and limitations

The parent supplies a finite bar region; add labels and result text as siblings. Progress has no focus, adjustment action or completion event. Indeterminate presentation is currently a static segment. The application determines success independently from the numeric fraction.

## Related APIs

[Status Badge](status-badge.md)
