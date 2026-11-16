# Checkbox

Use CheckboxContent for an independent checked value, including Unchecked, Checked and Mixed.

## Creating the content

```rust
use tekes_ui_controls::{CheckState, composition::*};

let mut host = ContentHost::new(Content::new(
    ComponentId(3),
    CheckboxContent::new("Published", CheckState::Unchecked),
)).unwrap();
host.set_action_events(true);
```

## State and events

new creates a model; from_original accepts an existing one. model and layout read state and prepared geometry. set_checked synchronizes a value without a user action. set_availability controls disabled and loading; set_error updates validation.

## Composition and limitations

Enable host action events and consume CheckboxState. CheckboxIntent.checked contains the captured value. Keep a stable ComponentId and treat the label and mark as one semantic control. Hover, focus and property reads do not save a value. Use RadioGroup for mutually exclusive choices.

## Related APIs

[Radio Group](radio-group.md)
