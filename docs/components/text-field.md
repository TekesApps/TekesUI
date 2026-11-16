# TextField

Use TextFieldContent for single-line editing. It owns text, selection, undo and composition; your application owns validation and saving.

## Creating the content

```rust
use tekes_ui_controls::composition::*;

let field = TextFieldContent::new("Document title", "A quieter workspace");
let host = ContentHost::new(Content::new(ComponentId(2), field)).unwrap();
```

## State and events

new(label, value) creates the field. surface holds TextSurface and field holds metadata. viewport chooses the editing viewport; show_metadata controls metadata presentation. tab_policy defaults to Navigate and submit_policy to None. inline supports compact rows; read_only_content creates selectable prose. Enable host action events to consume submit intentions.

## Composition and limitations

AppHost provides platform text services and window key routing. External values use surface.apply_external_value with its baseline, revision, selection and undo policies; handle its result. Preserve the instance and ComponentId across surrounding UI updates. scrollbar only changes the horizontal indicator, not text scrolling or caret reveal. Ordinary single-line fields show a clear button for nonempty text by default; clear_button(bool) controls whether it is offered. Multiline and content-presentation fields do not show it. Read-only, disabled and composing fields may retain a visible clear button, but it is disabled and cannot activate. The shared component owns clear-button layout, cancellation and focus behavior. Editable text uses the I-beam cursor by default.

## Related APIs

[Text Area](text-area.md)
