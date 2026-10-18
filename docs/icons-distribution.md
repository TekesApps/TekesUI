# Complete public icon distribution

As requested on 2026-10-19, the complete 176-item valid catalog is now a public MIT
resource in `crates/tekes-ui-icons`. The earlier handoff's local-only restriction
is superseded for this promoted catalog. Five retired originals remain excluded.
No external application directory is required for Rust, Gallery or Web consumers.

The 8 previously approved defaults retain their bytes. Other canonical resources
come from the existing valid 133-item catalog plus the independent 51-item proposal
set (8 overwrite existing defaults), yielding 176 unique SVGs. Black paint is
normalized to `currentColor`; geometry remains unchanged. Catalog provenance records
both source and output SHA256 hashes. Old renderer-specific scaled duplicates are
not separate icons.

The npm root package `@tekesui/icons` is a thin public distribution of this same
crate's assets, not a second implementation. Consumers lock a Git SHA/tag; Rust
consumers use `Glyph` while the eight-icon animated `Icon` API stays compatible.
The Gallery's default source is the shared public directory, with an explicit
external override retained only for historical comparison.

Native application package migrations, changes to animation behavior and unrelated
Gallery WIP are outside this catalog-publication change.
