# Link

Use LinkContent for a navigation action. Your application chooses the route or URL and how to open it.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let link = Content::new(ComponentId(2), LinkContent::new("Open documentation"));
```

## State and events

label supplies the visible and semantic name; model and view hold component state. Enter activates a link; Space remains available for scrolling. Enable host action records and handle Activate. Content::preserving_focus supports auxiliary actions beside editors.

## Composition and limitations

Theme supplies text, underline and focus appearance. LinkContent does not open a browser or perform network requests. Links inside selectable documents use RichContent and its range-aware link actions.

## Related APIs

[Markdown](markdown.md)
