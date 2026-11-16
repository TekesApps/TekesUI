# Breadcrumb

Use BreadcrumbContent to show a location path with navigable ancestors and a current item.

## Creating the content

```rust
use tekes_ui_controls::{composition::BreadcrumbContent, extended::Breadcrumb};
let path = BreadcrumbContent::new(Breadcrumb {
    items: vec!["Workspace".into(), "Documents".into(), "Current".into()],
});
```

## State and events

model.items holds the displayed path and view holds layout/input state. set_disabled and set_window_active control input availability. BreadcrumbNavigation captures index and path; the application performs navigation.

## Composition and limitations

The last item is the current location, not a navigable ancestor. The parent supplies width and clipping. Maintain business route identity separately from display labels. Breadcrumb does not supply a file browser or automatic overflow menu.

## Related APIs

[Link](link.md)
