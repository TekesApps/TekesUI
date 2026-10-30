# #12 Batch C: Settings non-Ready page states

Status **NEEDS_COORDINATION**: the capture fixture is ready in isolation; it still needs a scoped compile/GPU window and per-image review. It follows the #1/#55 parent matrix's remaining `G1/whole-page-reading-composition` and `G7/business-combinations-are-real` rows, and the matrix's prescribed Settings Pending → Failed → Saved sequence. It does not reopen the completed #113 52-root practical Theme migration.

## Scope and evidence boundaries

A's Settings initial Ready capture remains frozen at commit `aa50a24c37cf022cbd0f90448e53ec28b2b99323`, with its 48 images, program identity and visual review. B's master-detail candidate `13e9c3c6aaa6d58d3090ac71f02ccc72269edab4` and its unexecuted copy patch remain untouched. C is based on B and adds the current real Settings route's Invalid, Pending, Failed and Saved whole-page captures.

Existing `settings_full_failure_retry_and_danger_cancel_recovery` covers Gallery Settings validation, real input/action progression, first failure, retry success, and the unrelated Danger cancel path. Reuse that passing test when its inputs remain unchanged. `settings_gallery_retained_themes_keep_fixture_values_and_actual_palette` already exercises a Failed Settings instance across all four Themes with actual input and palette roles, but it asserts selected paint primitives and geometry, not whole-page images. `draft_protocol_form_scenes_use_current_journal_and_real_gpu` captures form, not the Settings route. The concrete gap is therefore per-image whole-page review of non-Ready Settings states.

## Candidate

Test-only module declaration in `workbench.rs` plus isolated `workbench/settings_phase_capture.rs`. It drives the actual existing `StoryState::submit` and `resolve` controller in sequence: blank name produces `Invalid`; a real Gallery Settings submission reaches `Pending`; first resolve produces `Failed`; resubmission and the second resolve produce `Saved`. It asserts that the selected route is the registered `example-settings/playground` example and that Gallery Settings owns no durable Form store. No replacement state machine, product control, mock UI or production behavior is introduced.

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
