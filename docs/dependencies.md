# Dependencies (`v0.4.7`)

All third-party workspace dependencies use permissive licenses (MIT, Apache-2.0, Zlib, Unicode-3.0, or BSD-style equivalents compatible with TekesUI's MIT license). No GPL, LGPL, or MPL crates are present in the dependency graph.

External versions below reflect `Cargo.lock` at `v0.4.7`.

## Zero-dependency Core & Token Crates

- **`tekes-ui-tokens`**: Zero third-party runtime dependencies (`default = []`; optional `token-audit`).
- **`tekes-ui-motion`**: Zero third-party runtime dependencies.
- **`tekes-ui-icons`**: Zero third-party runtime dependencies (`default = []`; `133` core square glyphs + `8` legacy `Icon` variants, with `46` additional workflow glyphs behind optional `product-icons`).
- **`tekes-ui-scene`**: Depends only on internal `tekes-ui-tokens`.
- **`tekes-ui-design-export`**: Uses `serde_json` (`1.0.154`) and `sha2` (`0.11.1`).

## Graphics, Text & Controls

- **`tekes-ui-render`**: `wgpu` (`30.0.1`), `bytemuck` (`1.25.2`), `pollster` (`1.0.1`); on macOS adds `anyhow` (`1.0.104`), `wgpu-hal` (`30.0.1`), and `objc2*` (`0.6.5` / `0.3.2`) bindings (`objc2-metal`, `objc2-io-surface`, `objc2-core-foundation`, `objc2-core-graphics`, `objc2-quartz-core`) supporting zero-copy shared surfaces (`macos-shared-surface` feature).
- **`tekes-ui-text`**: `serde` (`1.0.229`), `serde_json` (`1.0.154`), `unicode-segmentation` (`1.13.3`); on macOS adds `objc2-core-foundation`, `objc2-core-graphics`, and `objc2-core-text` (`0.3.2`) for CoreText shaping and glyph rasterization (`--no-default-features` builds cleanly cross-platform).
- **`tekes-ui-controls`**: `unicode-segmentation` (`1.13.3`), `unicode-normalization` (`0.1.25`); optional feature-gated dependencies:
  - `svg-icons`: `resvg` (`0.48.1`, `default-features = false`) + `tekes-ui-render`.
  - `bitmap-images`: `tekes-ui-render`.
  - `syntax-highlighting`: `syntect` (`5.3.0`, `default-features = false`, `default-syntaxes` + `regex-fancy`).

## Platform Bridge, Settings & Application Host

- **`tekes-ui-platform-macos`** (macOS target): `objc2` (`0.6.5`), `block2` (`0.6.2`), `objc2-foundation`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-metal`, `objc2-core-foundation`, `objc2-core-graphics` (`0.3.2`).
- **`tekes-ui-macos-bridge`**: Internal bridge combining `tekes-ui-platform-macos`, `tekes-ui-controls`, `tekes-ui-scene`, `tekes-ui-tokens`, and `tekes-ui-text`.
- **`tekes-ui-settings`** (`default = ["fs-store", "ui"]`): `serde` (`1.0.229`), internal `toml` (`0.8.23`) behind optional `fs-store` (hidden behind owned `Value`/`Table`/`SettingsError` public API), and optional `ui` connecting `tekes-ui-controls` / `tekes-ui-scene` / `tekes-ui-tokens` (`--no-default-features` compiles with only `serde`).
- **`tekes-ui-app-protocol`**: `serde` (`1.0.229`), `serde_json` (`1.0.154`).
- **`tekes-ui-module-runtime`**: `anyhow` (`1.0.104`), `serde_json` (`1.0.154`); on macOS adds `wgpu` (`30.0.1`), `mach2` (`0.5.0`) for Mach port transfer, `objc2-io-surface` (`0.3.2`), plus internal TekesUI UI/render crates.
- **`tekes-ui-app-host`**: `anyhow` (`1.0.104`), `serde` (`1.0.229`), `serde_json` (`1.0.154`); on macOS adds `wgpu` (`30.0.1`), `resvg` (`0.48.1`), and internal UI/platform crates (`window-acceptance` gates selftest dependencies).

## Optional 3D Scene Crate & Internal Apps/Demos

- **`tekes-ui-scene3d`**: `gltf` (`1.4.1`), `glam` (`0.30.10`), `bytemuck` (`1.25.2`), `wgpu` (`30.0.1`), `serde_json` (`1.0.154`), plus `tekes-ui-scene` and `tekes-ui-render` (`default = []`; domain city/apartment simulation data is isolated behind optional `simulation` feature).
- **Non-published workspace binaries/examples (`publish = false`)**: `tekes-ui-gallery` (`png`, `resvg`, `serde_json`, `unicode-segmentation`, `anyhow`), `tekes-ui-compare` (`png`, `sha2`, `serde`, `serde_json`), and `examples/city-3d-demo/crates/{tekes-ui-city,tekes-ui-3d}`.

## Python Tooling (`scripts/requirements-design.txt`)

- `jsonschema==4.26.0` for validating JSON design sources (`design/system.schema.json`, `design/brand/*.schema.json`, `design/theme/*.schema.json`, `design/source/tokens.schema.json`). Core Pure Rust boundary checks (`scripts/check-pure-rust.py`) and packaging helpers (`scripts/package-gallery.py`) work on stock Python 3.9+ without third-party packages.
