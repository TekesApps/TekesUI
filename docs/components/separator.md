# Separator

Use SeparatorContent for a passive line between related regions.

## Creating the content

```rust
use tekes_ui_controls::{composition::SeparatorContent, extended::Axis};
let separator = SeparatorContent::new(Axis::Horizontal);
```

## State and events

new(axis) chooses horizontal or vertical orientation. layout returns prepared geometry. The parent provides the available length; Theme provides the stroke and color.

## Composition and limitations

A separator has no focus or action and does not create a background or viewport. Use SplitView for a divider that users can move.

## Related APIs

[Split View](split-view.md)
