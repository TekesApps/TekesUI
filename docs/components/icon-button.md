# IconButton

Create an icon action by placing IconContent inside Button. The button owns its accessible name, focus and activation.

## Creating the content

```rust
use tekes_ui_controls::{Button, composition::*};
let icon = Content::new(ComponentId(2), IconContent::new("tekes-settings").unwrap());
let action = Button::new(icon).with_accessible_name("Settings");
```

## State and events

Use the Button properties variant, pointer_activation_phase, image_position and content_alignment. Replace content with set_contents. Enable host action records and handle Activate for the button target. Provide a meaningful accessible name when there is no visible text.

## Composition and limitations

Keep the icon subtree decorative; do not nest another interactive button. ImageId content also requires host resource preparation. The layout determines hit bounds, which may be larger than the icon artwork.

## Related APIs

[Button](button.md)
