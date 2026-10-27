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

## Accepted sizing decision (2026-10-27)

The user explicitly approved replacing the former standalone IconButton compact natural-size and caller-selected 20px artwork requirements with unified Button DS sizing. Image-only buttons use the shared Button insets/minimum height and Regular artwork scaled by resolved body typography. Explicit caller frames remain supported and do not stretch the artwork; the same centered placement and real pointer activation cases remain required.

This changes those two old size-specific acceptance requirements; it does not claim they still pass unchanged. Disabled/loading rejection, capture, activation, focus and accessible naming remain required. Historical standalone Icon evidence keeps its original source identity. Current-main reception and actual verification are tracked in #136; the full Toolbar/AppHost integration remains under #69/#70.

Default Button variants keep their existing presentation when no Surface has been declared. An explicitly declared Content/ancestor/Host Surface, including explicit canvas, uses the shared semantic Surface rules. Clearing a local declaration restores inheritance; reusing a Button must not retain a previous frame's explicit Surface. This is a presentation-boundary fact, not a new theme engine or provenance proof system.

## Local main intake evidence (2026-10-27)

Received from base `36dd13258d2025d606789363c97a691c15650050` through #136. Core/public/consumer migration, generated public API bindings and the real Gallery ButtonSpecimen are one dependency-complete batch: removing old entry points cannot be committed without their consumers. Actual checks and immutable per-round source identities are recorded at SSD `artifacts/issue-136/unified-button-main36dd132-intake/root-review.json`. Controls and Gallery/platform strict passed; original Controls, approved sizing/centering, public content shapes/cancellation and Gallery Button/Icon regressions passed. Actual Metal/CoreText rendered four content shapes in four themes and three typography scales. Earlier successful evidence retains its own input identity.

The full composition `scenarios` target did not pass: eleven existing RichDialog/Scroll reveal-support cases and one marked-IME cross-owner Accessibility Focus case remain failed. Their existing HEAD admission/fixture paths were compared; no reveal capability, IME barrier or assertion was weakened to receive Button. #136 retains these gaps alongside #98/#69/#70 integration boundaries. This intake does not establish full Toolbar/AppHost, native/VoiceOver, Gold7, all52 roots or remote delivery.
