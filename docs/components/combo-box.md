# ComboBox

Use ComboBoxContent for an editable query with string suggestions.

## Creating the content

```rust
use tekes_ui_controls::composition::*;

let combo = ComboBoxContent::new("Layout", vec!["Compact".into(), "Comfortable".into()]);
let mut host = ContentHost::new(Content::new(ComponentId(6), combo)).unwrap();
host.set_action_events(true);
```

## State and events

model reads items, query, selected, active and open; an active suggestion is not a committed selection. text_surface and popup_layout expose available editor and layout state. set_items returns a checked update; set_query rejects a changed query during composition. set_disabled cancels interaction and set_error updates field feedback.

## Composition and limitations

Consume ComboBoxSelection snapshots from the host action queue: index, label and declaration_revision identify the original suggestion. Host and AppHost coordinate popup navigation, text services and composition. Your application owns retrieval, result ordering and business IDs. Suggestions are strings, not arbitrary interactive rows.

## Related APIs

[Select](select.md)
