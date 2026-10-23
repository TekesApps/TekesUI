# TekesUI public icons

The complete 177-icon catalog is owned and distributed by TekesUI under MIT.
All SVGs are self-contained, use a square 24×24pt canvas, optically tuned rounded strokes and
`currentColor`. There is no private directory, network fetch or generation step
needed to consume the published catalog.

- `catalog.json`: canonical names, exact SHA256 hashes and provenance categories.
- `assets/`: the single authoritative SVG files for Rust, Gallery and Web.
- Rust: `Glyph::ALL`, `Glyph::named("tekes-shell")` and `.svg` expose the complete set.
- Web: `@tekesui/icons` exports `iconNames`, typed `IconName`, `getIconUrl(name)`.
  Each URL is a static module-relative SVG reference, suitable for browser bundlers.
- The existing eight-variant `Icon` and its animation API remain unchanged for
  compatibility. The complete catalog is additive through `Glyph`.

The Gold7 soft-contour revision keeps canonical identities and square 24×24
artboards. The approved C direction adjusts 158 applicable assets and retains 19 with recorded reasons;
the existing animation element/contour identities remain stable. See
`docs/design/icon-visual-language.md` and the complete coverage manifest in
`design/reviews/icon-gold7-57/coverage.json`.

The promoted existing catalog and published proposal artwork are publicly reusable.
The C revision uses 1.25pt for regular strokes, retaining 1.5pt for dense CAD
construction and preserving filled silhouettes. Paint remains full-alpha `currentColor`.
The 5 retired original names remain excluded; their independently drawn
replacement slots are included. Provenance source/output hashes are retained.

The Gallery loads these public assets by default. `TEKESUI_ICON_DIR` is an optional
explicit historical-comparison override, never a startup requirement. Shared
artwork is not mislabeled as a historical before/after comparison.

From a clean checkout:

```sh
npm run test:icons
cargo test --locked -p tekes-ui-icons
npm pack --dry-run
```

Install the exact public Git commit or an `icons-v*` tag as an npm dependency. The
root npm package contains only the icon assets, manifest, small JS/types entrypoints
and MIT notice. It has no lifecycle scripts, Rust build requirement or dependencies.

## Existing animation subset

`Icon::animation_layers()` retains stable layer IDs, anchors, SVG references and
correspondence keys for Settings/NewGoal/GoalCompleted/Memory/Think/Router/Provider/
Appearance. See `docs/design/symbol-effects.md` for the native animation contract.
