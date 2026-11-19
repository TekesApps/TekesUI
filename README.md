# TekesUI

TekesUI (`v0.3.36`) is a pure-Rust, GPU-rendered design system and desktop component toolkit. It provides compiled design tokens and surface color systems, CoreText text shaping, glyph atlas and `wgpu` rendering, stateful UI controls and composition containers, accessibility/IME platform bridges, and an optional multi-container `AppHost` shell.

## Architecture & Crates (`v0.3.36`)

All 15 library/tool crates share workspace version `0.3.36` and live under [`crates/`](crates/):

| Crate | Purpose | Optional Feature Flags |
| --- | --- | --- |
| [`tekes-ui-tokens`](crates/tekes-ui-tokens/Cargo.toml) | Compiled theme palettes (`light`, `dark`, `light-ic`, `dark-ic`), brand derivation, and `resolve_surface*` packets | `token-audit` |
| [`tekes-ui-design-export`](crates/tekes-ui-design-export/Cargo.toml) | Authoritative JSON token & theme compiler CLI/library (`--generate` / `--check`) | — |
| [`tekes-ui-scene`](crates/tekes-ui-scene/Cargo.toml) | Platform-independent render primitives, clips, transforms, and glyph atlas structures | — |
| [`tekes-ui-motion`](crates/tekes-ui-motion/Cargo.toml) | Spring/eased motion timelines and reduced-motion state transitions | — |
| [`tekes-ui-icons`](crates/tekes-ui-icons/Cargo.toml) | **133** canonical core square SVG glyphs (`Glyph`) + **8** animated `Icon` variants; optional domain packs + Web bindings (`@tekesui/icons`) | `product-icons` (+46 glyphs, 179 total) |
| [`tekes-ui-text`](crates/tekes-ui-text/Cargo.toml) | Text layout/shaping traits and macOS CoreText shaper/rasterizer | `--no-default-features`, `qa` |
| [`tekes-ui-render`](crates/tekes-ui-render/Cargo.toml) | `wgpu` offscreen/window renderer, clip/fragment budgets, glyph/bitmap leases | `macos-shared-surface` |
| [`tekes-ui-controls`](crates/tekes-ui-controls/Cargo.toml) | Stateful controls, scrolling viewports, surface-token styles, and `ContentHost` composition adapters | `svg-icons`, `bitmap-images`, `syntax-highlighting`, `token-audit` |
| [`tekes-ui-platform-macos`](crates/tekes-ui-platform-macos/Cargo.toml) | AppKit window, event, `NSTextInputClient`, and accessibility protocol primitives | `selftest` |
| [`tekes-ui-macos-bridge`](crates/tekes-ui-macos-bridge/Cargo.toml) | Connects `ContentHost` controls, CoreText, and macOS `NSAccessibility` | `selftest` |
| [`tekes-ui-settings`](crates/tekes-ui-settings/Cargo.toml) | Typed schema settings store, owned `Value`/`Table`/`SettingsError` API, and forms (`default = ["fs-store", "ui"]`) | `fs-store`, `ui` |
| [`tekes-ui-app-protocol`](crates/tekes-ui-app-protocol/Cargo.toml) | Pure serializable protocol types for container & worker IPC | — |
| [`tekes-ui-module-runtime`](crates/tekes-ui-module-runtime/Cargo.toml) | Child module process lifecycle and Mach/`IOSurface` frame import | `svg-icons` |
| [`tekes-ui-app-host`](crates/tekes-ui-app-host/Cargo.toml) | `AppWindow`, container layout (`NavBar`, `focus`, `others`), toolbars, and settings integration | `window-acceptance` |
| [`tekes-ui-scene3d`](crates/tekes-ui-scene3d/Cargo.toml) | Optional glTF/3D scene preview (`wgpu`) | `simulation` |

Non-published workspace applications and examples:

- [`crates/tekes-ui-gallery`](crates/tekes-ui-gallery/Cargo.toml) (`publish = false`) — Interactive component gallery (`AppHost` workbench & stories).
- [`crates/tekes-ui-compare`](crates/tekes-ui-compare/Cargo.toml) (`publish = false`) — Cross-backend text/render comparison runner.
- [`examples/`](examples/) — Self-contained consumer crates covering public composition (`composition-consumer`), custom brands (`brand-consumer`), theme/surface contracts (`theme-system-consumer`), icon catalog (`icon-catalog-consumer`), media viewer (`media-viewer-content-consumer`), process containers (`process-content-consumer`), `AppHost` profiles (`app-host-demo`), and standalone 3D viewer demos (`examples/city-3d-demo`).

## Quickstart

Add TekesUI crates via Git tag (or local workspace path):

```toml
[dependencies]
tekes-ui-tokens = { git = "https://github.com/TekesApps/TekesUI", tag = "v0.3.36" }
tekes-ui-scene = { git = "https://github.com/TekesApps/TekesUI", tag = "v0.3.36" }
tekes-ui-controls = { git = "https://github.com/TekesApps/TekesUI", tag = "v0.3.36", features = ["svg-icons"] }
```

Enable opt-in crate features only when needed:

- `tekes-ui-controls`: `svg-icons` (`resvg`), `bitmap-images` (`tekes-ui-render`), `syntax-highlighting` (`syntect`).
- `tekes-ui-icons`: `product-icons` adds 46 domain/workflow icons (`tekes-cad-*`, `tekes-drama-*`, workflow glyphs) on top of the 133 core glyphs.
- `tekes-ui-settings`: `--no-default-features` keeps pure data schema/types with only `serde`; enable `fs-store` for TOML persistence and `ui` for Settings form views.
- `tekes-ui-scene3d`: `simulation` enables optional city/apartment demo simulation data.

## Custom Brands & Design System

Design system sources live under [`design/`](design/):

- [`design/system.json`](design/system.json) and [`DESIGN.md`](DESIGN.md) describe the authoritative design contract (`v0.3.36`, `theme-and-surface-packets-active`).
- Consumers can compile their own `BrandConfig.json` and theme definitions completely offline using [`scripts/compile-consumer-brand.py`](scripts/compile-consumer-brand.py):

```sh
python3 scripts/compile-consumer-brand.py \
  --brand path/to/BrandConfig.json \
  --output path/to/offline-design
```

Point `TEKESUI_DESIGN_ARTIFACT=/path/to/offline-design` at build time to compile `tekes-ui-tokens` with consumer brand colors (`examples/brand-consumer` demonstrates two isolated consumer brands).

## Icon Catalog (`@tekesui/icons`)

The root [`package.json`](package.json) (`0.3.36`) publishes Web/ESM bindings for the 133 canonical square core SVG glyphs in [`crates/tekes-ui-icons/web/`](crates/tekes-ui-icons/web/):

```sh
python3 scripts/generate-icon-web-bindings.py --check
node --test crates/tekes-ui-icons/web/icons.test.mjs
cargo run -p tekes-ui-icon-catalog-consumer --locked --offline
```

See [`docs/icons-distribution.md`](docs/icons-distribution.md) for catalog provenance and feature split details.

## Running Gallery & Packaging

Build and run the macOS Gallery app directly, or package a signed `.app` bundle via [`scripts/package-gallery.py`](scripts/package-gallery.py):

```sh
cargo run -p tekes-ui-gallery --locked
# Stage and codesign target/app/TekesUIGallery.app (pass --offline to avoid network access)
python3 scripts/package-gallery.py --release --verify-tag v0.3.36
```

## Verification & Documentation

Core local verification checks match `.github/workflows/ci.yml`:

```sh
# Formatting, lints, and cross-platform boundary checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --locked --no-fail-fast

# Design system, pure-Rust boundary, packaging, and Python checks
python3 -m pip install -r scripts/requirements-design.txt
python3 -m unittest discover -s scripts/tests
for test in scripts/test-*.py; do python3 "$test"; done
python3 -I scripts/check-pure-rust.py --self-test && python3 -I scripts/check-pure-rust.py
python3 scripts/generate-theme-system.py --check
python3 scripts/check-design-system.py
python3 scripts/check-gold7.py
python3 .agents/skills/gold7-calm-audit/scripts/audit_gold7_calm.py --check
python3 scripts/check-cargo-package.py --offline
```

See also:

- [`docs/platform-boundary.md`](docs/platform-boundary.md) — Crate boundaries & cross-platform tracking
- [`docs/dependencies.md`](docs/dependencies.md) — Audited third-party dependencies (`v0.3.36`)
- [`docs/design/app-host-architecture.md`](docs/design/app-host-architecture.md) — `AppHost` & process container architecture
- [`docs/archive/design-logs/INDEX.md`](docs/archive/design-logs/INDEX.md) — Archived historical design & review records
