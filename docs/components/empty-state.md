# EmptyState

Use EmptyStateContent for an empty collection or absent content, with an optional explicit action.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::EmptyState};
let empty = EmptyStateContent::new("Documents", EmptyState {
    title: "No documents".into(), detail: "Create a document to begin.".into(),
    action: Some("Create".into()),
}, ComponentId(2)).unwrap();
```

## State and events

model and layout read state and geometry; disabled reads availability and set_metadata updates semantics. Use host.replace_empty_state and set_empty_state_disabled after mounting. The constructor action_id identifies the action in the host queue.

## Composition and limitations

Release model borrows before host updates. Parent layout supplies finite space and chooses between content and empty state. An empty state does not decide whether a query finished, fetch data or navigate. Treat loading errors separately from a successful empty result.

## Related APIs

[List](list.md)
