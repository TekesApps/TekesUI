# SplitView

Use SplitViewContent to arrange two persistent panes with an adjustable divider. Construction supplies configuration, slot identities and a work budget.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let split = SplitViewContent::try_new(SplitViewConstruction {
    config: SplitViewConfig::default(),
    slots: SplitViewSlotIds { leading: ComponentId(2), trailing: ComponentId(3), divider: ComponentId(4) },
    leading: Content::new(ComponentId(5), TextContent::new("Leading")),
    trailing: Content::new(ComponentId(6), TextContent::new("Trailing")),
    budget: 64 * 1024 * 1024,
}).unwrap();
```

## State and events

SplitViewConfig contains axis, ratio, divider, min_pane and collapsed. config and layout return available state. After mounting, use host.split_view_expected, update_split_view and replace_split_view_pane for versioned changes. Handle rejected updates without discarding their inputs.

## Composition and limitations

The component handles divider pointer and keyboard input. Each pane retains its own focus, text services and scrolling. SplitView does not virtualize content. Choose a budget for the actual data size; the example is not a universal recommendation.

## Related APIs

[Scroll Area](scroll-area.md)
