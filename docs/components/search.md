# Search

Use SearchFieldContent for a query editor. Your application filters local data or starts a search.

## Creating the content

```rust
use tekes_ui_controls::composition::*;

let mut search = SearchFieldContent::new("Search documents", "");
search.set_placeholder(Some("按名称搜索…".into()));
search.set_submit_policy(TextSubmitPolicy::Enter);
let mut host = ContentHost::new(Content::new(ComponentId(1), search)).unwrap();
host.set_action_events(true);
```

## State and events

new(name, query) supplies a name and initial query. set_placeholder configures empty-value guidance; set_submit_policy selects submission. set_availability distinguishes read-only and disabled, and set_error adds validation feedback. set_clear_label validates the clear action name. query and input read current state; apply_query publishes versioned external state without a user action.

## Composition and limitations

After host input, consume ContentActionPayload::Search. Submit captures the submitted query; Clear captures the previous query. Use those snapshots instead of rereading later text. The original editor owns selection, undo and composition; the clear button has no separate editing focus. Search retains its SearchAction contract while sharing the field clear interaction. Editable query text uses the I-beam cursor by default. Empty queries reserve no clear-button spacing. A nonempty read-only, disabled or composing query retains a disabled clear button; it cannot activate until the editor is editable again. Results and asynchronous request ordering belong to the application.

## Related APIs

[List](list.md)
