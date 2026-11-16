# Dialog

Use DialogContent for modal content composed from real header, body and footer components.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let dialog = DialogContent::new("Confirm",
    Content::new(ComponentId(2), TextContent::new("Save changes?")),
    Content::new(ComponentId(3), TextContent::new("Review before continuing.")),
    Content::new(ComponentId(4), Button::new("Save")));
// After mounting, open with host.show_dialog(dialog_id)?.
```

## State and events

host.show_dialog validates and opens a modal scope; host.close_dialog closes it. Child action records carry confirmation and cancellation intentions. is_open differs from is_presented during exit animation; modal_frame reads geometry and needs_animation reports presentation work.

## Composition and limitations

Host owns window keys, text services and focus restoration. Your application decides whether saving succeeded and whether to close. Compose scrolling explicitly for long bodies. Dialog is not a native system message box and does not automatically submit forms.

## Related APIs

[Popover](popover.md)
