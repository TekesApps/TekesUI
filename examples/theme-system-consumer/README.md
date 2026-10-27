# Surface contract candidate

This isolated workspace compiles the original new modules by path while the
component session owns the shared crate exports/build wiring. It is an honest
**pre-integration harness**, not proof of a released public API or composed UI.
It uses explicit development path dependencies on the exact active WIP. No SDK
checkout mutation or consumer state color tables are required.

```sh
target/design-tools-venv/bin/python scripts/generate-theme-system.py --check
cargo run --manifest-path examples/theme-system-consumer/Cargo.toml --locked
cargo test --manifest-path examples/theme-system-consumer/Cargo.toml --locked
```

After the single shared-file owner applies `integration.patch`, replace the two
`#[path]` modules in main.rs with `use tekes_ui_tokens::surface_palette` and
`use tekes_ui_design_export::theme_system`. Then rerun outside the SDK workspace,
using a published exact revision for release. Current WIP is not that revision.

Proposed public consumption (following integration):

```rust,ignore
use tekes_ui_tokens::{Palette, Theme};
use tekes_ui_tokens::surface_palette::resolve_surface;
let palette = Palette::from(Theme::DARK); // existing appearance/contrast mode
let colors = resolve_surface(palette, "panel", "action-quiet", "hover")?;
// All channels arrive together; component state chooses the presentation state.
// Composition inherits the declared Surface identity from its nearest container.
// Draw background/text/icon/boundary and an independent focus-visible layer.
```

No RGB inference or color fallback; unsupported combinations return an error.
The API accepts an explicit already-compiled Palette. Mode/theme changes must
invalidate the entire resolved bundle and any interpolation endpoints. Mount,
document, context generation and ABA rejection stay in original composition.
This resolver owns no focus, input, capture, IME, layout or interaction state.

`content` supports rest/disabled; `link` supports rest/hover/pressed/disabled;
interactive input/action/selection uses also include selected combinations.
Disabled suppresses focus; focus-visible remains a separate boolean owned by
composition. Error/status overlays retain original semantic roles and remain a
migration dependency, not an invitation to derive brand colors in a component.
