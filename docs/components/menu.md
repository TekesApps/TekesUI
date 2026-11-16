# Menu

Use MenuContent for a trigger and menu. For business commands, declare explicit MenuEntry values rather than relying on display text.

## Creating the content

```rust
use tekes_ui_controls::composition::{MenuContent, MenuEntry};
let mut menu = MenuContent::new("Actions", vec![]);
menu.set_entries(vec![MenuEntry::item("save", "Save")]).unwrap();
```

## State and events

MenuEntry supports items and submenus. set_entries validates a declaration; set_values, set_item_checked, set_item_disabled and set_item_separator update it. model_revision reads the declaration version and set_popup_viewport bounds the overlay.

## Composition and limitations

MenuSelection captures index, label, model_revision and optional value_path. Use that captured identity after the menu closes. Context menus additionally use the host binding/request contract and context_request. Host and View own submenu input and dismissal. The application owns command execution, permissions and global shortcuts.

## Related APIs

[Command Palette](command-palette.md)
