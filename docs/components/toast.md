# Toast

Use ToastContent for a temporary notification driven by the host monotonic motion clock.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::{Toast, NoticeTone}};
let toast = ToastContent::new("Notice",
    Toast::new("Saved", NoticeTone::Success, 3000), ComponentId(2)).unwrap();
```

## State and events

model reads state and layout reads prepared geometry; set_metadata updates the semantic name. After mounting, use host.show_toast, set_toast_visible, set_toast_paused and dismiss_toast. The dismiss_id supplied at construction identifies its close action.

## Composition and limitations

Do not hold a model guard across host updates or callbacks. Host owns timing, pause and activity; a missing clock is an error. The parent provides finite space and placement. Toast does not implement a notification queue or system notification center. Display success only after the application operation succeeds.

## Related APIs

[Banner](banner.md)
