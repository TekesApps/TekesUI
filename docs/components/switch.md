# Switch

Use SwitchContent for an on/off setting. The application decides whether changes are saved immediately.

## Creating the content

```rust
use tekes_ui_controls::composition::SwitchContent;
let toggle = SwitchContent::new("Notifications", false);
```

## State and events

model.checked is the value; model.disabled controls availability and label provides a name. Pointer and keyboard changes need not produce Activate records. Read checked after host input and compare with your published state; this API has no captured-value payload equivalent to CheckboxIntent.

## Composition and limitations

The View owns focus, press and animation state. Do not add a second click handler that toggles again. Disabled does not mean off. Switch has no Mixed state; pending saves, failures and rollback belong to the application.

## Related APIs

[Checkbox](checkbox.md)
