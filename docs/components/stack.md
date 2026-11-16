# Stack

Use StackContent to arrange actual Content children along one axis. It measures their natural sizes before allocation.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::{Axis, Stack}};
let stack = StackContent {
    model: Stack { axis: Axis::Vertical, gap: 8. },
    children: vec![Content::new(ComponentId(2), TextContent::new("Title"))],
    flexible_child: None,
};
```

## State and events

model.axis and gap choose direction and spacing. flexible_child assigns remaining main-axis space to one child. into_content_with_distribution selects an explicit distribution. The lower-level Stack::layout uses a different equal-allocation policy.

## Composition and limitations

Child components retain their input, focus and actions. Stack has no business actions, general flex/shrink algorithm or implicit viewport. Use ScrollContent when content can exceed the available space.

## Related APIs

[Scroll Area](scroll-area.md)
