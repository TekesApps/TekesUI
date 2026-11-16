# Button

Use Button to perform an action. Text, images and composed decorative content share the same focus, press, cancellation and activation behavior.

## Creating the content

```rust
use tekes_ui_controls::{Button, composition::{ComponentId, Content, ContentHost}};

let mut host = ContentHost::new(Content::new(ComponentId(4), Button::new("Apply"))).unwrap();
host.set_action_events(true);
```

## State and events

Configure contents with new and set_contents. Give image-only buttons an accessible name with with_accessible_name. variant selects visual meaning; pointer_activation_phase defaults to Release. image_position and content_alignment arrange the content. After enabling host.set_action_events(true), drain_action_records delivers Activate records with a stable target. Property updates and focus movement do not activate the button.

## Composition and limitations

The host owns pointer cancellation, keyboard routing and image resources. Keep one activation handler per button. Theme supplies spacing and state colors; Toolbar provides its own compact composition. Image registration and preparation belong to the host.

## Related APIs

[Icon Button](icon-button.md)
