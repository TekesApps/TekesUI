# #12 Batch C: Settings non-Ready page states

Status **NEEDS_COORDINATION**: the capture fixture is ready in isolation; it still needs a scoped compile/GPU window and per-image review. It follows the #1/#55 parent matrix's remaining `G1/whole-page-reading-composition` and `G7/business-combinations-are-real` rows, and the matrix's prescribed Settings Pending → Failed → Saved sequence. It does not reopen the completed #113 52-root practical Theme migration.

## Scope and evidence boundaries

A's Settings initial Ready capture remains frozen at commit `aa50a24c37cf022cbd0f90448e53ec28b2b99323`, with its 48 images, program identity and visual review. B's master-detail candidate `13e9c3c6aaa6d58d3090ac71f02ccc72269edab4` and its unexecuted copy patch remain untouched. C is based on B and adds the current real Settings route's Invalid, Pending, Failed and Saved whole-page captures.

Existing `settings_full_failure_retry_and_danger_cancel_recovery` is the source entrypoint for Pending → Failed → Retry → Saved plus the unrelated Danger cancel path; it does not itself empty the Settings name or cover validation. Its presence is not proof of a current passing execution. Reuse an existing passing receipt only when the relevant inputs remain unchanged; otherwise the scoped execution packet below includes this existing regression. `settings_gallery_retained_themes_keep_fixture_values_and_actual_palette` already exercises a Failed Settings instance across all four Themes with actual input and palette roles, but it asserts selected paint primitives and geometry, not whole-page images. `draft_protocol_form_scenes_use_current_journal_and_real_gpu` captures form, not the Settings route. The concrete gap is therefore per-image whole-page review of non-Ready Settings states.

## Candidate

Test-only module declaration in `workbench.rs` plus isolated `workbench/settings_phase_capture.rs`. It drives the actual existing `StoryState::activate(20/21, "settings")` owner in a fresh Gallery per phase/theme/scale, which invokes `submit` and `resolve`: blank name produces `Invalid`; a real Gallery Settings submission reaches `Pending`; first resolve produces `Failed`; resubmission and the second resolve produce `Saved`. It asserts that the selected route is the registered `example-settings/playground` example and that Gallery Settings owns no durable Form store. No replacement state machine, product control, mock UI or production behavior is introduced.

The capture test emits 64 fresh original PNGs: those four real phases × four themes × @1x/@2x × actual page top/bottom, 720×512 at type scale 150%. Its manifest records route, phase, theme, scale, offset/range, document height, input, attempts and original event log. It never clears or replaces an existing directory; manifest visual review remains `unverified` until the original files are reviewed individually.

## Scoped execution

The specialized rendering integration session owns MAIN and window/cache scheduling. This candidate does not run Rust or GPU. At the next allocated window, after confirming source identity and that the output directory is fresh, run through its existing `scripts/tekesui_build.py` build lock/environment and GPU occupancy guard with the already registered MAIN SSD cache:

```text
cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery settings_non_ready_phases_render_real_gallery_page_across_themes --no-run

TEKESUI_SETTINGS_PHASE_OUT=<fresh-absolute-output> cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery settings_non_ready_phases_render_real_gallery_page_across_themes -- --ignored --nocapture --test-threads=1

cargo +1.95.0 clippy --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery --tests -- -D warnings
```

Also run the existing named regression `settings_full_failure_retry_and_danger_cancel_recovery` against the same resulting binary/source. The unchanged input/Theme tests above can be reused; the new GPU frames cannot be relabeled as native input, physical screen, IME, AX, VoiceOver or performance acceptance. Preserve source and executable hashes, logs, manifest, image hashes and lock release receipt. Review every PNG for complete labels/status, readable wrapping, Save/Retry disabled state during Pending, retained input during Failed, Saved confirmation, helper text, action order, clipping and bottom reachability. If any frame fails, retain it and fix only the established defect.

## Checks run on the isolated candidate

- Scoped `rustfmt +1.95.0 --edition 2024 --check` for the touched Rust files: pass.
- `git diff --check`: pass.
- Source inspection confirms the new test module and keeps the existing settings retry regression present.
- Rust compile, regression execution, strict Clippy, GPU PNG generation and per-image review: not run; awaiting the shared rendering session's window.

## Parent design/AC mapping and current scope

The authoritative parent matrix is the existing section in
`docs/design/component-registry-public-api-20261017.md`, integrated as `53fdb80` per the
2026-10-30 #1/#55 current summaries. Its original eight AC and 23 Gold7 rows are retained.
The source candidate patch is
`/Volumes/HP P900/DEV/TekesUI/artifacts/issue-55/parent-ac-matrix-20261030-01/candidate.patch`,
SHA256 `001b099d75802cf521005859dbf5c2a74f72b5821856dc1b2e46e9244b96846c`.
C directly supplies the first ordered follow-up: Settings Pending → Failed → Saved;
Invalid is the existing required-name failure preceding that flow. It contributes only to
AC7 (four-theme/1x2x representative state pages) and the visual portion of
G1/whole-page-reading-composition and G7/business-combinations-are-real.
It does not close these rows, AC5's native focus requirements, or the remaining 23-row matrix.
`docs/gallery.md` receives only this fixture pointer; no existing design contract, token or
registry/maturity rule changes. No new matrix replaces the parent section.

A and B remain frozen. The latest #12 integration result separately records B's real
Document word-break visual failure; C does not relabel that failure or repair B in this batch.
The specialized integration session owns actual MAIN/source selection and build/GPU scheduling.
No C build/GPU/native window has been used by this domain.

## Preparation revision

Initial uncompiled C commit `d9ccec4184247ee96f0b83bda7987d2db734c6ac` is retained.
Source review found `Workbench.text_scale: usize` assigned `1.5`; 150% is the existing
index `2`. This is a confirmed preparation type error, not a claimed executed compiler failure.
The revised candidate corrects it before any shared compilation, reuses the existing
stabilized page scrolling helpers, keeps PNG encoding local because the A/B PNG helper is private, and adds phase/value/action/layout assertions.
This revision changes only the new C fixture and its documentation; A/B test source and
historical results stay unchanged. The current source hashes and static results are in
`preparation.json`; runtime compile, Clippy, regression, all64 GPU frames and visual review
remain unverified until the execution packet is run. No product defect is established by
an unexecuted fixture and no production repair is included.

## Integration execution — 2026-10-30

The two candidate commits were applied in order as `c29b395` and `13c0ef1`, independently of the unaccepted Master-detail B files. Actual test inputs were HEAD `13c0ef1911ccb7db0ebc78c593b68863b60f4828` plus the preserved Unicode WIP, through the original registered SSD/MAIN cache and build/GPU guards. Compilation, the ignored 64-frame GPU test, the existing Settings failure/retry/danger regression (one actual test) and Gallery strict Clippy all passed. The source, executable and raw command identities remain bound to that actual input, not the final documentation commit.

All 64 original PNGs were individually reviewed by the scoped Worker. The integration executor independently opened eight top/bottom images covering Invalid/light-standard/1x, Pending/dark-increased/2x, Failed/light-increased/2x and Saved/dark-standard/1x and verified all 64 hashes. Required-name validation, pending Save/Resolve, retained Studio input/Retry, saved confirmation and event tails are readable in these finite scrolled views. No inspected whole-word split or overlap was found. Exact per-image results and execution references are in `visual-review.json` and `execution.json`; originals remain on the SSD.

This accepts only the requested 720×512/type150, four-theme, 1x/2x top/bottom phase matrix. A's original Ready images and B's actual visual failures remain unchanged. Native input, physical display, IME, VoiceOver, performance and the rest of Gold7 remain unverified. All build/GPU contexts exited; no native app or desktop input was used.
