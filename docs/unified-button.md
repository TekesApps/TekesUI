# Unified Button

Use `tekes_ui_controls::Button` (also exported from `composition`) for every button content shape:

```rust
use tekes_ui_controls::{Button, composition::{Content, ComponentId, TextContent}};
use tekes_ui_scene::ImageId;

let text = Button::new("Save");
let icon = Button::new(ImageId(1)).with_accessible_name("Settings");
let image_and_text = Button::new((ImageId(1), "Settings"));
let custom = Button::new(Content::new(ComponentId(2), TextContent::new("Local")))
    .with_accessible_name("Open local settings");
let help = Button::new("Save").with_help("Save the current document");
```

ImageId refers to a host-registered approved image, as in the existing image capability. The standard icon extent comes from the Design System Regular icon token and resolved body typography. Consumers do not supply per-button icon dimensions. Button insets, minimum height and icon/text gap reuse existing tokens. Command plus/minus works through the host's existing resolved typography context.

Content as a whole is centered by default. `image_position` selects Leading, Trailing, Above or Below; `content_alignment` uses the existing layout MainAxisDistribution. Custom Content uses normal shared containers to declare its own internal arrangement. ToolbarContent takes these same ordinary buttons, and ToolbarControl retains command/overflow ownership while reusing the same Button presentation and interaction.

Every form has one mounted action and one accessible identity. Text supplies the default accessible name. Image-only/custom contents require an explicit nonempty name; help is optional and cannot substitute for that name. Decorative content cannot contain interactive descendants. Button.with_help/set_help uses the existing ContentHost tooltip dwell, dismissal, revision and AX-description routing. Content.with_help can override component help; clearing that override exposes component help again. Clear Button help to revoke that source. Unmount revokes both sources.

This is a breaking WIP API change. CompoundButton, ButtonContent, extended::IconButton and layout_view::IconButtonView were removed without aliases. Replace compound construction with Button::new(original_content).with_accessible_name(name), and remove the former .action indirection. Replace icon controls with Button::new(image).with_accessible_name(name). For framework state/painter integrations, the former low-level root Button is now explicitly ButtonModel; it is not a separate UI component choice. AppHost and independently pinned consumers are left for their owners to update. No AppHost source or frozen delivery package is modified here.

Validation evidence: design/reviews/unified-button-content-20261022/. Public API, real consumer, actual Metal/CoreText rendering, native interaction and Gold7 remain separate evidence layers. API compilation or a GPU screenshot does not establish final native/VoiceOver/Gold7 acceptance.
