# Select

Use SelectContent to choose one value from a finite list of strings. Use ComboBoxContent when users need to type a query.

## Creating the content

```rust
use tekes_ui_controls::composition::*;

let select = SelectContent::new("Mode", vec!["One".into(), "Two".into()]);
let mut host = ContentHost::new(Content::new(ComponentId(7), select)).unwrap();
host.set_action_events(true);
```

## State and events

labels and model read the declaration and current state; layout reads prepared geometry. set_labels returns Result<bool, SelectUpdateError> and invalidates old option actions when changed. set_selected returns false for invalid indices and cancels previous popup interaction. set_disabled and set_error update availability and validation.

## Composition and limitations

Consume MenuSelection snapshots through the host action queue, using index and label from the event. Keep business IDs alongside the matching declaration; string options do not supply value_path. The host owns focus and popup routing. Select does not accept text, support multiple selection or provide arbitrary row content.

## Related APIs

[Combo Box](combo-box.md)
