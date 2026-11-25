# Complete public icon distribution

The TekesUI icon catalog is owned and distributed by `crates/tekes-ui-icons` under the MIT license, with no external application directory required for Rust, Gallery, or Web consumers.

As of `v0.4.12`, the default build ships **133** canonical square core glyphs (`Glyph`) alongside **8** legacy `Icon` enum variants. An additional **46** domain/product-workflow glyphs (14 `tekes-cad-*`, 14 `tekes-drama-*`, and 18 agent/goal workflow icons) live in `crates/tekes-ui-icons/packs/product-extensions/` behind the opt-in Cargo feature `product-icons`, bringing the full enabled set to **179** canonical square glyphs.

The 8 previously approved defaults retain their bytes, SVG black paint is normalized to `currentColor` while preserving source geometry, and `catalog.json` / `packs/product-extensions/catalog.json` record canonical provenance hashes.

The root npm package `@tekesui/icons` (`package.json`, `crates/tekes-ui-icons/web/`) exports the 133 canonical square core assets (plus compatibility aliases `iconAliases` mapping legacy product names `tekes-side-chat` -> `tekes-chat`, `tekes-goal-completed` -> `tekes-check-circle`, `tekes-access-workspace` -> `tekes-folder-edit`) for web consumers. Native hosts can register domain/host extension glyphs end-to-end via `AppWindow::{from_columns_with_icons, from_containers_with_icons, extension_icons}` without adding them to the core catalog. Native application integrations and visual acceptance tracking remain separate from catalog packaging.
