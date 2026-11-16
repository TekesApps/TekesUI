# CommandPalette

Use CommandPaletteContent to combine a display trigger, a TextFieldContent query and a command list.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::{CommandPalette, ComboBox}};
let palette = CommandPaletteContent::new(
    Content::new(ComponentId(2), TextContent::new("Commands")),
    Content::new(ComponentId(3), TextFieldContent::new("Filter commands", "")),
    CommandPalette { combo: ComboBox { items: vec!["Open File".into()], ..Default::default() } },
);
```

## State and events

open and close change presentation; is_open and options read state. CommandSelection captures index and label. Execute the captured command instead of reconstructing it from a later filtered list.

## Composition and limitations

The query retains its instance and draft when closed but leaves active layout, input, focus and semantics. Host coordinates keyboard input and composition. Keep the trigger display-only. The application owns command execution, permissions and failures; this control does not register global shortcuts or discover plugins.

## Related APIs

[Search](search.md)
