# Disclosure

Use DisclosureContent for an expandable region whose application decides whether to accept expansion requests.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let title = Content::new(ComponentId(3), TextContent::new("Details"));
let body = Content::new(ComponentId(4), TextFieldContent::new("Notes", ""));
let disclosure = DisclosureContent::new(ComponentId(2), title, body, false);
```

## State and events

Read expanded, disabled and trigger_id. Consume DisclosureExpansion and confirm its DisclosureIntent with confirm(intent, accepted). The intent carries the requested expanded state and declaration identity. set_expanded synchronizes authoritative state; set_disabled, set_rtl and set_accessible_name update presentation.

## Composition and limitations

with_section_header_layout configures the header; with_trigger_focusable configures keyboard focus. Keep the body instance to preserve drafts and scroll state when collapsed. The application decides whether unsaved content may be hidden.

## Related APIs

[Field](field.md)
