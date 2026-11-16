# Icon

Use IconContent to display a named icon through the host icon renderer.

## Creating the content

```rust
use tekes_ui_controls::composition::IconContent;
let icon = IconContent::new("tekes-settings").unwrap();
```

## State and events

new and set_glyph validate canonical names. glyph reads the current Glyph. with_accessible_name or set_accessible_name supplies a name for meaningful images; decorations may omit one. set_status_tone, set_disabled, set_visible and set_indeterminate update presentation.

## Composition and limitations

The host owns animation lifecycle and prepared rendering resources. layout reads prepared geometry. IconContent is passive; wrap it in a named Button for an action. An unknown icon name is an error.

## Related APIs

[Icon Button](icon-button.md)
