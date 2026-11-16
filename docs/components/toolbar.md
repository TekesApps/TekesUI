# Toolbar

Use ToolbarContent and ToolbarGroup to organize controls in the Content tree. In AppHost, the toolbar belongs to its Container.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let bar = ToolbarContent::new(vec![Content::new(
    ComponentId(2),
    ToolbarGroup::new(ToolbarPlacement::PrimaryAction, vec![
        Content::new(ComponentId(3), Button::new("Save")),
    ]),
)]);
```

## State and events

ToolbarPlacement identifies roles such as Navigation, Status and PrimaryAction. set_radio and set_selected_id configure exclusive groups using real child IDs. Child actions use the normal host action queue. Keep status text as TextContent.

## Composition and limitations

A group with visible = false retains its instance but contributes no measurement, spacing or width budget. Restoring visibility uses the normal layout. Toolbar does not register commands or permissions. The host owns window input and compact layout.

## Related APIs

[Button](button.md)
