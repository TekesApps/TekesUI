# Field

Use Field metadata for an input label, required marker, help and error. TextFieldContent combines that metadata with an editor.

## Creating the content

```rust
use tekes_ui_controls::composition::TextFieldContent;
let mut field = TextFieldContent::new("Project name", "");
field.field.required = true;
field.field.help = "Use a recognizable name.".into();
field.field.error = Some("A name is required.".into());
```

## State and events

field.label, help, error and required describe the field. TextSurface.input owns text, disabled/read-only state, composition and selection. Required does not automatically validate or prevent saving. Field itself emits no save action; use the input component action contract.

## Composition and limitations

Metadata takes layout height, including in Fill and Grow viewports. Keep errors outside the editable text area. The application owns cross-field validation and asynchronous errors. Visible field help and transient hover help serve different purposes.

## Related APIs

[Text Field](text-field.md)
