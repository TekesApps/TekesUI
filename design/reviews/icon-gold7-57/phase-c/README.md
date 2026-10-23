# Final C acceptance — issue #57

2026-10-23. The user's final direction is: “C 更接近，按这个方向推广到全部适用图标，保持正方形画布，完成全量验收后再关闭 #57。” The prior v2 result was explicitly insufficient and is historical evidence, not this acceptance.

## Delivered scope

177 canonical SVG identities were reviewed: **158 changed, 19 byte-retained**. `../coverage.json` records each reason and before/after SHA256. All sources remain `width=24 height=24 viewBox="0 0 24 24"`. Twelve representative files match the exact approved C bytes in `../approved-c/`.

Regular strokes use 1.25pt, full-alpha `currentColor`; dense CAD detail retains 1.5pt and pure filled states retain their silhouette. C folder/document families share their representative contours. Native Gallery primitives consume the generated artwork metrics through the existing quad renderer; rounded ends/bends and centered square fitting apply to all ten existing primitive identities. Element/contour identities, names, animation references, event routing, AX ownership and component layout/state remain unchanged. No new renderer, ticker, AppHost or Kernel changes.

Two public constants (`tekes_ui_icons::REGULAR_STROKE_WIDTH` and `ARTBOARD_SIZE`) expose artwork metrics, not component sizes. The controls-only public type inventory explicitly excludes constants and other crates; no controls type classification is added.

## Bound sources and evidence

- Clean public base: `50ec49f3446924fdbc66b3cff22cd8cfe150fe7c`, branch `codex/icon-gold7-57`.
- Public reproducible source: `tools/soften.py --check`, canonical catalog hashes, retained original and approved C inputs. `public-source-sha256.json` binds this delivery's files.
- Frozen full Gallery SDK: `validated-square-source-sha256.json`; `before-source-sha256.json` binds the v2 baseline. This snapshot includes other owners' WIP for existing IconContent/cache/component paths; it is evidence only and is not included in the icon PR or represented as released #47 infrastructure.
- Raw final evidence remains under `icon-c-rollout-57-20261023` in the active Gallery worktree's `design/reviews/`. Original and v2 baselines remain under `icon-full-rollout-57-20261023`. Neither was overwritten. `gpu-artifact-sha256.json` records final artifact hashes.
- `run.py`, `run-square.py` and `summarize.py` record exact frozen evidence commands. Public code and independent consumers run from a clean checkout using the commands below; WIP fixture recreation requires the exact locally retained frozen SDK, not arbitrary current main.
- `native-binary-receipt.json` binds the actual ad-hoc signed local C app to the tested frozen source. Screenshots are JPEG, not offscreen images.

## Validation and visual review

| Layer | Result and precise scope |
| --- | --- |
| Source / public API | Pass: 177 names and exact hashes, square canvases, no active content; eight animated defaults retain public references; deterministic regeneration. Rust 2 tests, Web 3 tests, standalone public Rust consumer and separately packed npm consumer. |
| Canonical GPU | Pass: **4,248 configurations** = 177 × 3 sizes (12/16/24pt) × 4 modes × 2 backing scales. Runtime compiled SVG hashes match final sources. All **456** retained configurations are pixel-identical to original. All names reviewed in six actual GPU crop sheets; original 1x sheets and component/whole-page images reviewed at their native dimensions for sampling and clipping. |
| Real component GPU | Pass within artwork scope: **64 pages** with existing Toolbar, Button, TextField, list and status IconContent/cache paths, four modes, 2 representative brands, 1x/2x, normal/narrow and 100%/150% type. **15 component/resource/motion tests passed**, one pre-existing capability-negative test remains ignored. No asset-induced tint, ownership, lifetime or layout change found. |
| Actual Gallery GPU | Pass within icon scope: **144 frames** = four modes × three brands × two backing scales × two window/type allocations × top/interior/bottom. Uses original page prepare/build_scene and actual SVG Image primitives. Top of narrow 150% page contains header only; interior/bottom provide the icon evidence. |
| Native primitive GPU | Pass: **240 configurations**, ten identities × 4 modes × 2 scales × 3 sizes. Six public Gallery regressions pass, including exact wide/tall slot vs centered-square pixels, plus symmetry, bounds, semantic tint and existing 40-component states/hit geometry. |
| Native window | Observed C at 12/16/24pt in four modes, 150% type, actual narrow window and scrollbar drag. Search text survived appearance/size changes; icon canvas remained proportional. CUA system-generated input; not physical input. Screenshots are source/binary bound. |
| Physical IME / VoiceOver / performance | **Untested**. Editing, semantic/AX owner and motion service code are unchanged; this artwork acceptance does not certify them or platform-wide Gold7. |

Existing Gallery limitations remain visible and are outside the assigned icon-painting scope: inner icon filter lacks an AX node, long diagnostic copy is clipped in narrow 150% layouts, and CUA wheel attempts in the narrow window did not move the page; scrollbar dragging did. These are not reported as passed general Gallery accessibility/scroll behavior. Native text selection/marked text and Composer were not exercised.

## Gold7 mapping

“Pass” below is restricted to this artwork revision, not all of #12 or #47.

| Principle | Design contract | Code integration | Actual experience |
| --- | --- | --- | --- |
| G1 | Pass: user chose C after rejecting v2 | Pass: exact C representatives and full coverage | Pass in scope: all-family GPU review and C native window show softer contours, lighter regular weight and retained recognition |
| G2 | Pass: inherit complete Theme/Surface foreground | Pass: no color/alpha override; existing semantic tint regression | Pass in scope: four modes and brand component scenes; entire theme system remains #55 |
| G3 | Pass: square canvas, proportional fitting, optical exceptions | Pass: deterministic 177 assets, generated metrics, square GPU regression | Pass in scope: 12/16/24pt at 1x/2x, counters/terminals/cropping reviewed; native 2x observed |
| G4 | Pass: preserve animation identity and existing lifecycle | Pass: component/motion/resource tests; no new ticker | Untested physical motion; existing service unchanged, no whole-motion acceptance claimed |
| G5 | Pass: artwork must not own focus/events/reading | Pass by scoped diff and existing component tests | Search text retention and scrollbar drag observed with system input; physical IME, selection/marked text and Composer untested |
| G6 | Pass: retain directional and status meaning | Pass: names/paths/action ownership retained | Pass in scope: arrows, expansion, error/success/run cues reviewed; media/pan/split features not applicable because unchanged |
| G7 | Pass: single canonical catalog, docs and real preview | Pass: public Rust/npm consumers, complete source coverage, existing renderer | Pass in scope: runnable C Gallery and actual-component GPU evidence; #47 release/service maturity remains separate |

## Recheck from the public checkout

```sh
python3 crates/tekes-ui-icons/tools/soften.py --check
python3 scripts/generate-icon-web-bindings.py --check
cargo test --locked -p tekes-ui-icons
npm run test:icons
cargo run --locked --manifest-path examples/icon-catalog-consumer/Cargo.toml
cargo test --locked -p tekes-ui-gallery gallery::pages::showcase::tests -- --test-threads=1
cargo clippy --locked -p tekes-ui-icons -p tekes-ui-gallery -- -D warnings
# npm pack; extract tarball; run check-packed.mjs with extracted package directory
```

Local development paths are explicit overrides. Public consumers can pin the final public Git commit exactly; no npm publish or release tag is part of this task.

CI first run stopped at stale capability source hashes after the owned icon changes. The generated inventory was refreshed for the three changed source files and the new generated style module only; capability conclusions and public bindings are unchanged. Composition audit and public formatting checks pass. The failed CI log is retained.

An independent consumer also fetched the exact public Git revision `ac37506ddcaf168df4aa99e74689ca64d65b70be` from GitHub and ran successfully: 177 square canonical glyphs and eight matching animated defaults. This confirms the remote commit rather than a local path override.
