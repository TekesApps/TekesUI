# Canonical Gallery catalog and self-contained delivery — #57

Scope: the [2026-10-26 incremental acceptance](https://github.com/TekesApps/TekesUI/issues/57#issuecomment-6405840705).
The approved C artwork and the historical [phase-C receipt](../../design/reviews/icon-gold7-57/phase-c/README.md)
remain unchanged. This candidate does not close #57: Rust and real packaged-app
execution await the integration chat's allocated windows.

## Behavior

The default `IconCatalog::from_env()` parses the exact SVG bytes embedded by
`tekes_ui_icons::Glyph::ALL`. All 177 entries are available without a source
checkout, filesystem asset lookup, or an environment override. The existing
SVG parser, renderer, tinting, image cache and ownership remain in use.

Historical comparisons require `TEKESUI_ICON_COMPARE=historical`. The 51
proposal references are compared with canonical artwork; the 29 differing
SVGs become the existing `before` side of their canonical card. Identical
references, including eight references to today's shared `Icon` artwork, are
not mislabeled as old drafts. The published SVG remains the current/default side. The
existing two-panel captions distinguish old artwork from the current default.
No historical SVG silently replaces canonical artwork in default mode.

`TEKESUI_ICON_DIR=/absolute/external/directory` selects external comparison and
takes precedence over historical mode. Same-name external entries become the
`before` side of canonical cards. External-only identities remain inspectable;
read/parse errors remain notices. Existing external directory parsing limits,
retired-name and scaled-variant handling are preserved. A missing external
directory is reported and does not remove the embedded canonical entries.

The package script keeps its binary, app-icon, signing and replacement flow.
It also includes the existing public `catalog.json` as
`Contents/Resources/IconCatalog.json`. The SVGs themselves are embedded in the
executable through `Glyph::ALL`; no second asset directory or runtime resolver
is introduced. The manifest records expected identities and is not proof that
an arbitrary `--binary` contains them: the runtime probe compares that binary's
resolved SVG bytes with the packaged manifest and rejects a stale/mismatched binary.

## Evidence entry points

Run script-only checks with Python 3.11 or newer (the existing packaging script
uses `tomllib`). No Rust, GPU, AppKit, signing or build locks are exercised:

```sh
PYTHONDONTWRITEBYTECODE=1 python3.11 -m unittest discover -s scripts/diagnostics/icon57 -p 'test_*.py' -v
python3.11 scripts/diagnostics/icon57/verify_package.py inventory --root /path/to/accepted/source --out /path/to/fresh/source-inventory.json
```

The inventory checks `Glyph::ALL`, public catalog names/hashes and source SVG
bytes, then enumerates all 51 proposal overlaps, including the 29 differing
1.5pt historical / 1.25pt canonical identities. It writes source evidence only.
The script tests use synthetic runtime reports and mocked platform packaging
tools; passing them does not mean the native app was built or launched.

The opt-in `TEKESUI_ICON_CATALOG_REPORT=/absolute/fresh/report.json` emits
`tekesui-icon-catalog/1` after the actual Gallery catalog is constructed. Its
entries include the exact source bytes used to parse each rendered SVG tree,
canonical-match flags and comparison-side presence. The app continues normal
startup. A requested report write failure is explicit; normal launches do not
write a report. This report proves catalog resolution, not rendered pixels.

## Integration execution package — not executed in this candidate

Only integration chat `01a18c4f-d227-7900-8390-520b735d0c4e` assigns the existing
serial build/cache and native/GPU windows. It applies this candidate's explicit
files to its agreed MAIN baseline and uses the already registered SSD routing,
`gallery-design-baseline/TekesUI/scripts/tekesui_build.py` build lock/environment
and MAIN-bound review cache. Do not execute cargo in this isolated worktree,
create a target/cache, rebind caches or alter timestamps/flags.

Within that allocated build window, use `cargo +1.95.0` with `--locked --offline`
for the targeted `tekes-ui-gallery` build and these non-GPU behavioral tests:

```text
canonical_catalog_uses_every_published_svg_byte_without_a_source_directory
historical_mode_pairs_proposals_beneath_canonical_current_artwork
external_catalogue_stays_the_comparison_side_of_canonical_current_artwork
missing_external_directory_keeps_canonical_entries_and_reports_the_load_error
catalogue_report_serializes_the_resolved_svg_and_comparison_side
```

Run each as `cargo +1.95.0 test --locked --offline -p tekes-ui-gallery TEST_NAME`
under that existing controlled entry point, then compile the Gallery binary
there. Do not run all `icon_catalog` tests without a GPU window: that module
also retains GPU tests. Package the matching compiled binary using
`python3.11 scripts/package-gallery.py --binary /absolute/compiled/binary`
from the authorized MAIN workspace under the existing packaging build lock.
Preserve the exact source/program/configuration evidence from that build. This
document supplies no new cache scheduler or source-closure gate.

Within the separately allocated native window and existing occupancy guard:

```sh
python3.11 scripts/diagnostics/icon57/verify_package.py run \
  --app '/absolute/packaged/TekesUI Gallery.app' \
  --source-root /actual/compiled/source/workspace \
  --source-root /other/relevant/source/checkout \
  --out '/Volumes/HP P900/DEV/TekesUI/artifacts/issue-57/FRESH-NATIVE-RUN'
```

Use the actual compiled Cargo workspace path from the build receipt; repeat
`--source-root` for relevant original/candidate checkouts. The driver copies
the app outside those roots, scrubs inherited icon overrides/selftests/perf
modes, and uses macOS `sandbox-exec` to deny source reads in the launched app.
A preflight must demonstrate denial of each otherwise readable `Cargo.toml`;
failure prevents app launch. No checkout is moved, hidden or renamed. The
driver retains the sandbox profile, executable hash, source-denial results,
native log, actual runtime catalog and success/failure result. It compares all
177 runtime SVG hashes/names to the package's manifest, requires canonical
mode with no comparison/error entries, observes the child alive for three
seconds after report creation and stops only that child.

If the local sandbox cannot provide this isolation, retain its failure. A run
with source access is not equivalent. The driver does not activate VoiceOver,
generate physical input or claim window pixel acceptance. Any necessary icon
page observation remains a separately recorded native result. Existing C
artwork/GPU evidence is reusable for unchanged source artwork and render paths;
it is not evidence that this new catalog/packaging candidate ran.

## Candidate results and remaining acceptance

Source inventory: 177 canonical entries, 51 proposal overlaps, 29 differing
artwork identities. Script checks: nine passed. The first script invocation
used system Python 3.9 and failed to import the existing `tomllib` requirement;
Python 3.11 passed without a compatibility fallback or product code workaround.
Rust tests/build, real signing/package execution, runtime report creation,
source-read denial and native window observation are **not run** here. #57
stays open until the effective incremental AC are actually verified.

Physical IME, VoiceOver and performance remain untested and are not added as
gates to this finite icon batch. No Theme, C assets, main checkout, other owner
candidates or other Issues are changed.

## Integration validation — 2026-10-30

The isolated commit `92372453` was applied as `f7229e6` without its unrelated parent candidates. The first scoped formatting failure was retained; only `icon_catalog.rs` was formatted, then all five named behavioral tests each ran once and passed, Gallery strict Clippy passed, and the normal binary was rebuilt. The actual compiler inputs were `f7229e6` plus the recorded one-file format change (the same Rust bytes later committed as `168b57f`), not a relabelled later checkout. Raw programs, hashes and failures are in SSD `artifacts/delivery/integration-executor-20261030-08/rust01/format-rerun/`.

Real actool/ad-hoc signing and strict codesign verification passed using that archived binary and MAIN's actual package assets/catalog. MAIN's default app destination had no registered SSD route; the executor instead explicitly selected the existing registered Gallery app output under the original Gallery helper/guards. No target/cache/storage binding was created or changed; the previous app was fully archived first. Signing changes the Mach-O: the compiler binary and signed executable have separately recorded hashes. `IconCatalog.json` contains 177 entries and is byte-identical to the source catalog. The immutable signed bundle and route/program identities are in SSD `artifacts/delivery/integration-executor-20261030-08/package-final/identity.json`; it has not been launched.

The outside-checkout/source-denial native run, visible icon-page observation, physical input, VoiceOver and performance remain unverified. These checks and the original Issue acceptance criteria are not replaced by the five unit tests or package construction.
