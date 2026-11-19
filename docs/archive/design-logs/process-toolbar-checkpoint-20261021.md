# Process and native-toolbar development checkpoint

This checkpoint follows main1294987 and retains the full active WIP in its shared checkout. It consolidates the verified optional-View context, window-bound service/toolbar scope contract, standard ProcessContainer local toolbar, native default calibration, sidebar disclosure animation/removal of Swap controls, and window-level keyboard ownership. Recent Scene3D city-view/source changes are included after the source-stable current-workspace correctness run, rather than inferred from their older standalone reports. Their original dedicated report remains historical; current combined validation is recorded here.

Public ProcessComponent declarations retain original Session and registration revision for delayed actions. Window bindings reject foreign registration; View placement uses original ToolbarControl/ContentHost and preserves worker/state when changing scope. Coalesced View invalidations do not fill the bounded business queue. Headless consumers remain UI/GPU-free. `ProcessContainer` forwards original services and borrows caller icon images. Existing legacy V1 explicit opaque-primary input remains a documented compatibility profile.

Window Toolbar uses measured AppKit allocation/traffic-light center separately, JSON toolbar-label/compact-chrome, original >=32pt hit geometry, shared canonical artwork and originalMore image. Default geometry remains native-owned. Original ContentHosts implement group focus/roving/activation/cancellation; Window only owns group order and a logical owner. Temporary async busy/layout barriers blur/cancel actual input and restore a still-valid owner without replaying a press or stealing explicit focus transfers. Native review on exact producer da51e14a reproduced and closed the earlier5b27 busy-focus failure.

## Validation and limits

- Current combined workspace:1296 passed,23 ignored, the same3 timing tests excluded as CI. Source bytes remained stable during the run. No performance measurements were run.
- Independent composition consumer:307 tests passed in the preceding shared-repair run; Host-only follow-up adds19 actual original-host tests and strict Clippy.
- Final current-source service, V2 View, local-toolbar/CoreText/GPU and original V1 Component/queued/legacy consumers passed again. V2 has16 actual imported GPU pixel cases; local toolbar has12 theme/brand cases and one-RPC activation/resource/cancellation checks.
-18 fresh bounded token observations/6624 cases, including144 original Toolbar inline/window cases. These do not assert full dependency/state or native experience coverage.
- Native source-bound receipts distinguish actual original App key events/clicks from physical keyboard, VoiceOver, display perception and full Gold7. The final navigation busy transfer was observed before Execute/HistoryCommitted and retained its explicitly selected owner afterward.
- The original imported134-file package and historical goldens are retained. The published icon catalog/Web projection is repaired and checked in CI; no new UI framework is introduced.

This is an experimental development checkpoint, not a declaration that the eight batches or original Issues are complete. Remote V2 keyboard/IME/AX, arbitrary Content Toolbar groups/registry, complete per-action native semantics, remaining icon optical classification, physical held-key geometry acceptance and whole-page Gold7 remain. No Issue is closed by this checkpoint. The accepted older AgentKit freeze is immutable; new consumers review this distinct exact Git input before promotion.

## Source compatibility

Old constructors/defaults and the original Start/StartView wire variants remain. Adding generated TextRole::ToolbarLabel is potentially source-breaking for external exhaustive matches: handle the new variant or apply an intentional wildcard. Current public consumers compile, but that is not proof that every downstream exhaustive match remains compatible. No package-version/semver-release claim is made by this development tag.

## Reproduction

Use the repository rust-toolchain, root and consumer locked dependencies, and the existing independent public examples. Run `cargo test --workspace --locked -- --skip timing_is_off_by_default_and_costs_nothing_to_drain --skip cpu_segments_are_consistent_and_frames_are_numbered --skip gpu_time_agrees_with_wall_clock_on_a_heavy_frame`, strict workspace/all-target Clippy, `examples/composition-consumer`, and the module-runtime examples. `process_container_toolbar` takes an output directory; `process_container_backend` supports `--component-review OUTPUT` and `--queue-review`. Desktop native/VoiceOver/physical IME remain separate steps and are not established by these commands.

Publication preflight and selected portable receipts are under `design/reviews/process-toolbar-checkpoint-20261021`. Build caches, binaries/Apps, source-copy trees, archives and unrelated reference-language fixtures are not publication inputs. Exact immutable source plus selected evidence is the handoff; old local receipts and fixed snapshots are preserved in place.
