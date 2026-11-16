# Tooltip

Use Content::with_help for short contextual help. The host help overlay uses TooltipView.

## Creating the content

```rust
use tekes_ui_controls::{Button, composition::*};
let action = Content::new(ComponentId(2), Button::new("Save"))
    .with_help("Save this document");
```

## State and events

set_help updates optional help and help_text reads it. host.help_tooltip reports the current owner, text and OverlayLayout. Direct TooltipView users must supply anchor, viewport, hover/focus, window activity and Escape handling themselves.

## Composition and limitations

Help does not activate a control or take editing focus. Placement follows the visible anchor and viewport. Tooltip is passive short text; use PopoverContent for interactive content and visible field errors for persistent validation.

## Related APIs

[Popover](popover.md)
