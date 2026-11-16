# Popover

Use PopoverContent to present interactive content anchored within the same ContentHost.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let popover = PopoverContent::new("Preferences",
    Content::new(ComponentId(2), TextFieldContent::new("Name", "")));
// After mounting both the popover and its anchor:
// let session = host.show_popover(popover_id, anchor_id)?;
// host.close_popover(session);
```

## State and events

show_popover(owner, anchor) returns a PopoverSession; use that session to close the matching presentation. is_open reports logical state, while is_presented includes exit presentation. current_session and layout read state; take_closed consumes closure records. set_dismiss_visible and set_enabled configure interaction.

## Composition and limitations

Host owns anchor identity, focus scope, restoration and dismissal. The body retains its text services and actions. A popover is not another window or host. HoverPreviewContent has a separate passive-preview contract; clear_presentation is cleanup, not saving.

## Related APIs

[Dialog](dialog.md)
