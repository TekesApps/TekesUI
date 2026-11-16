# Surface

Use PanelContent to wrap a real component in a themed surface with shared rounded geometry. SurfaceView provides the lower-level drawing entry.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let child = Content::new(ComponentId(2), TextContent::new("Panel body"));
let panel = PanelContent::new(child).unwrap();
```

## State and events

set_canvas chooses surface semantics; set_padding validates optional Insets. child reads the content and geometry returns prepared PanelGeometry, which may be absent before layout. Handle construction and padding errors.

## Composition and limitations

Panel owns background, padding and the geometry used by paint, hit testing and reveal. Children retain input and state. It does not create a window, scrolling, shadows or business actions, and it does not change a child field presentation.

## Related APIs

[Stack](stack.md)
