# Theme foundation intake on local main

Delivery entry: #136, dependency slice of #62 and the existing #6/#7/#55 Theme contracts. Baseline: `361f418612693069d6a6534aafff2cc79154f91b`. Root is the sole integrator. Source provenance is the hash-frozen 1509-file historical observed source plus the exact sixteen accepted #62 Theme interface increments; this is not a copy of current Gallery Root WIP.

This slice supplies the complete selected historical Theme schema/rules/instances, Palette ThemeDefinition identity, full generated Surface packet facade, and normal pure Rust source/artifact/independent `/1` lock validation. Python remains offline generation tooling. The existing main Toolbar 20pt base inputs and metadata are preserved, with accurate reviewed source pins; no historical 13pt rollback is included. Existing default/Gallery BrandConfig semantics and role alpha remain authoritative.

## Source compatibility and dependency closure

Palette now selects an explicit `GalleryBrand` or `ThemeDefinition` source. External `Palette { theme, brand }` literals and `.brand` field access migrate to `Palette::new`, `Palette::from`, `source()` and `brand_index()`. The repository's direct uses are included in this batch. The two motion cache keys retain `(Theme, PaletteSource)` so independent ThemeDefinition choices cannot alias brand zero. This is a source API migration; unchanged rendered default/brand behavior is an actual regression gate, not an assertion of struct-literal compatibility.

The independent public consumer selects blue/clay/default in all four appearance/contrast modes, checks unknown identity rejection, accent isolation, neutral/default compatibility, complete packet consumption and original alpha. A separate external-artifact QA consumer is required for normal independent-lock acceptance and rejection/recovery. The ordinary `/1` lock is retained; this batch does not claim the later provenance sidecar or full-root proof contract.

## Acceptance boundary

The final local-main checks cover generator freshness, complete Theme rules and negative cases, normal tokens/design-export tests, the necessary Palette call-site builds/regressions/strict checks, and real independent public consumption of valid external input, rejected missing/stale/tampered input or lock, and valid recovery. Evidence is stored on SSD under `artifacts/issue-62/main-361-theme-foundation-r1` with source/program/depinfo/OUT_DIR identities and separate failed rounds.

This dependency slice does not re-accept all #62 ContentTheme inheritance/override/clear, Button/Text/TextField mounting or visual migration. #104 Chart usages/focus have now completed local-main acceptance on base `3e989f1` plus the exact source receipt; see [Chart design and current evidence](chart-content-20261024.md). The original #115 geometry formulas and bare-reference records are preserved. #92 ContentHost remains a separate intake requiring action-preparation/reveal dependencies. Historical Root evidence retains its original identity. #113's first Button root and all52 requirement remain pending under their own acceptance chain; this foundation does not mark any root complete. Native windows, physical IME, VoiceOver, whole-library Gold7 and public release are not claimed here.

## Shared rules versus component intake (2026-10-26)

#71 Progress rules (`dc7283f9`), #83 StatusBadge/Skeleton rules (`8fb1909`) and #93 Notice rules (`3e989f1`) are accepted on local main. Their actual public Content/Gallery migrations remain the existing #72/#76/#77/#79 deliveries to receive; original Gallery Root proofs are not transferred to main. #104 now connects its two shared purposes to the actual public ChartView and independent default/external consumer. It does not receive #92 ChartContent. Receive each complete component slice with its actual dependencies, without waiting for every shared rule or redevelopment of accepted parts.
