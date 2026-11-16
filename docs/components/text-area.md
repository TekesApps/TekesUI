# TextArea

Use TextFieldContent with a multiline TextSurface for notes, drafts and messages. There is no separate TextAreaContent type.

## Creating the content

```rust
use tekes_ui_controls::{TextSurface, composition::*};

let mut area = TextFieldContent::new("Message", "");
area.surface = TextSurface::new("", true, false);
area.viewport = TextViewport::Grow { min: 56., max: 160. };
area.tab_policy = TextTabPolicy::Insert;
area.submit_policy = TextSubmitPolicy::CommandEnter;
let mut host = ContentHost::new(Content::new(ComponentId(5), area)).unwrap();
host.set_action_events(true);
```

## State and events

The final TextSurface constructor flags mean multiline and read-only. surface.input holds text, selection and marked text. Fixed, Grow and Fill viewport modes control height; Fill also needs remaining-space allocation from the parent Stack. Use apply_external_value for versioned external drafts.

## Composition and limitations

Navigate is the default Tab policy. Insert permits tabs; Backtab is consumed without unindent. CommandEnter submission preserves ordinary Enter for newlines. Submission is an intent, not proof that a message was sent. TextSurface owns editing scroll and caret reveal; ScrollContent owns page scrolling. Use read_only_content or RichContent for reading.

## Related APIs

[Text Field](text-field.md)
