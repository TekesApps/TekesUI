# Tabs

Use TabsContent to combine actual tab labels and panels with persistent Content identities.

## Creating the content

```rust
use tekes_ui_controls::composition::*;
let tabs = TabsContent::new(vec![TabItem::new(
    Content::new(ComponentId(2), TextContent::new("Overview")),
    Content::new(ComponentId(3), TextFieldContent::new("Draft", "")),
)], 0);
```

## State and events

selected reads the current index; select changes it programmatically. set_tab_disabled, set_tab_open and reorder update the declaration; handle their results. TabItem can declare name, disabled and closable. TabSelection captures index, label and panel. TabCloseRequest requests closure without automatically closing or selecting the panel.

## Composition and limitations

The application approves closing and manages unsaved data. Hidden panel input, focus and semantics remain under the component lifecycle. Use the captured panel identity rather than interpreting an event against a later index. TabStripContent is a separate title-strip contract.

## Related APIs

[Segmented Control](segmented-control.md)
