# ScrollArea

Use ScrollContent to give one Content subtree a viewport, clipping, wheel input and reading behavior. ScrollArea is the underlying model.

## Creating the content

```rust
use tekes_ui_controls::{ScrollArea, composition::*};
let body = Content::new(ComponentId(2), TextContent::new("Scrollable content"));
let scroll = ScrollContent::new(ScrollArea::default(), body).scrollbar(true);
```

## State and events

scroll exposes the model; viewport_height can set a viewport height. horizontal(true) uses natural content width and disables vertical scrolling. scrollbar changes indicator visibility without disabling scrolling or reveal. set_scroll_reading_document and the host reading transactions preserve document anchors across updates.

## Composition and limitations

The parent must allocate finite viewport space. TextSurface retains selection and caret reveal. ScrollContent does not remove field borders, virtualize every child or fetch pages. An append does not necessarily mean the user wants to jump to the end.

## Related APIs

[List](list.md)
