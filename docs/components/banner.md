# Banner

Use BannerContent for a persistent message inside a page, optionally dismissible.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::{Banner, NoticeTone}};
let banner = BannerContent::new("Announcement", Banner {
    message: "Connection unavailable".into(), tone: NoticeTone::Warning,
    dismissible: true, visible: true,
}, ComponentId(2)).unwrap();
```

## State and events

model exposes message, tone, dismissible and visible; layout reads geometry and set_metadata updates the name. Publish mounted changes through host.replace_banner. The construction dismiss_id identifies the close action.

## Composition and limitations

Release model borrows before host updates. Parent layout provides finite space. Banner has no automatic timeout, retry or reconnection behavior and is not an arbitrary rich-content container. Your application decides when to clear the message.

## Related APIs

[Toast](toast.md)
