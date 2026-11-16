# RadioGroup

Use RadioGroupContent for a group of mutually exclusive choices.

## Creating the content

```rust
use tekes_ui_controls::composition::RadioGroupContent;
let group = RadioGroupContent::new("Mode", vec!["One".into(), "Two".into()]);
```

## State and events

model and view read state. set_selected and set_item_enabled return Results; set_disabled and set_error update availability and validation. set_axis changes arrangement. RadioSelection carries a RadioSelectionIntent with the captured index; intent_is_current validates delayed handling.

## Composition and limitations

The component owns keyboard navigation and semantic options. Parent ScrollContent owns overflow. Semantic part tokens are not persistent business IDs. Use Checkbox for independent selections; this API does not promise remote or arbitrary rich options.

## Related APIs

[Checkbox](checkbox.md)
