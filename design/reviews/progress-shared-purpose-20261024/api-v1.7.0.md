# Shared Theme / Surface contract 1.7.0 — passive Progress

This increment extends the retained [1.6.1 contract](../theme-surface-integration-20261023/api-v1.6.1-accepted.md). It does not rewrite that evidence or claim component integration, original-page after images, native acceptance, or full Gold7.

Design System owns shared generation, constraints, component anatomy and the Surface vocabulary. Theme Definition Schema remains 2.0.0; specific Theme definitions retain their own versions and four internal appearance/contrast modes. Surface Schema is 1.3.0 and shared generation rules are 1.7.0. Instance structure remains `tekesui.theme-instances/2`; contract structure remains 1.0.0. Definition versions and color inputs are unchanged, while the generated capability changes with the shared rules/schema and exact source/lock hashes.

The seven formal Surfaces reference one shared `progress-track` profile. Its exact state set is `{rest}` and all seven channels are present: background `surface-2`, text and icon `primary-text`, boundary `border`, marker `c-progress-active`, focus/helper null. Marker and boundary are required. Marker must contrast with rail by at least 3; boundary must independently contrast with rail and declared parent Surface by at least 3. Text/icon retain the existing 4.5 minimum. No RGB-to-Surface inference or per-component color table is introduced.

This passive anatomy has no hover, pressed, disabled, selected, focus or helper states. Determinate value and indeterminate presentation remain original Progress model data, separate from interaction states. Host loading/error/success compositions retain their separate required migration and acceptance obligations; adding this packet does not declare them implemented. Neutral rail/boundary and brand marker retain their own semantics. Status text/icon belongs to an independent host composition; this progress-track packet provides primary-text for its text/icon channels.

A container declares a canonical Surface; composition owns propagation; the original component consumes the whole generated packet. The generic resolver needs no change. A shared component implementation can consume the existing public API:

```rust
use tekes_ui_tokens::{Palette, Theme, surface_palette::{canonical_surface, resolve_surface}};
let palette = Palette::for_theme_definition(Theme::LIGHT, "blue-inputs").unwrap();
let surface = canonical_surface("canvas").unwrap();
let colors = resolve_surface(palette, surface, "progress-track", "rest").unwrap();
assert!(colors.boundary.is_some() && colors.marker.is_some());
assert!(colors.focus.is_none() && colors.helper.is_none());
assert!(resolve_surface(palette, surface, "progress-track", "hover").is_err());
```

Application callers provide the model and Theme/Surface identity through the component contract; they do not derive colors or populate each state. Progress View/Content/AX/Gallery source wiring belongs to the sole component writer and is still a separate delivery.

The rest-only Schema branch is strict and separate from the original rest+disabled branch. After inheritance expansion every previous use still requires its exact original state set. Missing channels/uses, inapplicable overrides, invalid unused profiles, unknown identities/roles, low marker contrast and either insufficient boundary contrast are rejected. Existing solid status, focus, helper and range guards remain active in Python and ordinary Rust builds.

An external Theme artifact and independent lock may change allowed Theme input data, not package shared Surface rules. Coherently rewriting external rules, bindings, contract/provenance and lock still rejects an extra Surface. A new Surface is a deliberate Design System rule input, validated and compiled by the same mechanism.

`inspector-progress` is an approved experiment only: its declared parent ground equals `surface-2` to exercise a necessary boundary at equal ground. Its eight-Surface rule input and actual Rust consumer are retained separately; formal seven-Surface sources/outputs must be restored byte-for-byte afterward. Generic consumer success is not actual Progress component/GPU integration.

The original Gallery before uses GalleryBrand index 0 (default projection), not a `blue-inputs` ThemeDefinition page. Its actual `component-progress/variants` route, 48 PNGs, executable and source identity are retained in `gallery-before/`. It shows an absent Light-IC 0% rail and faint other rails. The old images remain before evidence; neither arithmetic nor generator success resolves that actual visual gap until the original component is wired and the same page is rendered again.
