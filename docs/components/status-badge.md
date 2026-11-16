# StatusBadge

Use StatusBadgeContent for a passive status label whose meaning comes from application state.

## Creating the content

```rust
use tekes_ui_controls::{composition::StatusBadgeContent, extended::StatusBadge};
let badge = StatusBadgeContent::new(StatusBadge::Info, "Ready").unwrap();
```

## State and events

model, label and layout read state and prepared geometry. update validates the model, text and accessibility metadata. decorative creates a display without an extra semantic focus stop.

## Composition and limitations

Combine text and status color so meaning does not depend on color alone. Theme resolves the appearance. Badge emits no business action and does not start work. A progress value of 100% does not itself prove success.

## Related APIs

[Progress](progress.md)
