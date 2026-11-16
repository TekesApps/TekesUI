# SegmentedControl

Use SegmentedControlContent for a compact choice among a few views or modes.

## Creating the content

```rust
use tekes_ui_controls::composition::SegmentedControlContent;
let segments = SegmentedControlContent::new("View", vec!["List".into(), "Grid".into()]);
```

## State and events

model and view read state. set_selected and set_item_enabled return Results. set_disabled controls the group and set_compact chooses compact presentation. SegmentSelection carries a SegmentIntent; index reads the captured choice and intent_is_current checks its declaration.

## Composition and limitations

The component owns layout, pressing and arrow-key navigation. Your application provides the selected content. Semantic part tokens are not business keys. For persistent panels and close requests, use TabsContent.

## Related APIs

[Tabs](tabs.md)
