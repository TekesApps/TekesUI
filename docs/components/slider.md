# Slider

Use SliderContent to choose a value in a finite numeric range.

## Creating the content

```rust
use tekes_ui_controls::{composition::SliderContent, extended::Slider};
let slider = SliderContent::new("Volume", Slider::new(50., 0., 100., 5.).unwrap());
```

## State and events

model.value and model.range read state; model.disabled controls availability and name supplies semantics. Handle the constructor Option. Pointer dragging, arrow keys, Home/End and accessibility adjustments use the component input route.

## Composition and limitations

After host input, compare the actual value with your published value. ContentResponse.changed can also describe interaction changes; this control does not provide a dedicated value snapshot payload. The parent allocates track space. Parsing, persistence and business undo belong to your application.

## Related APIs

[Number Field](number-field.md)
