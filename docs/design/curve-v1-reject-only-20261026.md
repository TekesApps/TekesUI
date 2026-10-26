# Curve V1 codec and fail-closed routing — Issue #81

This phase adds the bounded Curve V1 wire codec and explicit typed refusal routes to the existing process protocol. It does not negotiate or publish curve capability. The original Issue remains open for successful grant publication, owner staging, input/text/accessibility/reveal, and final parent-boundary integration.

## Source boundary

The protocol's `component/curve.rs` owns bounded wire records, operation/transaction/TTL contracts and rejection codes. `component.rs` adds the Curve request and reply variants. The runtime's existing `services.rs` associates Curve pending state with the original request slot and configured View/transaction, validates outer session/serial first, and refuses generic or mismatched substitutes. `process_view.rs` handles Curve requests before the business/render queue. The independent consumer example exercises real child processes; `tests/process_view.rs` checks the pure typed-rejection builder, not a native View.

The batch includes those six files together because the new enum variants, matching routes and consumer tests are one interface change. No new Content/ViewContext/ProjectedAccessibility API, renderer change, Cargo manifest or production lock change is required. Existing Toolbar, NumberField and Gold7 implementations remain on the latest main baseline.

## Validation identity

Validation receipts distinguish the private port based on upstream `7c13ec0909601ac1e8a7b4a0a67f6e086d2307b7` from the actual main checkout. Gallery Root R7/R8 evidence is historical and is not reused as a pass for either tree. The Issue record carries the actual command/results, receipt paths, commit SHA and remaining scope. A local commit does not prove native behavior or public immutable release.

The old-peer compatibility harness compiles the complete frozen old decoder source and checks explicit rejection of unknown Curve variants plus acceptance of the existing Start envelope. It does not claim that old peers support the new protocol. No native/GPU acceptance, successful grant path, push or release is part of this phase.

## Actual final-main validation

On 2026-10-26, after formatting exactly the six batch-owned Rust files, actual main worktree checks passed: protocol library 7 tests, runtime Curve services 4 tests, the pure View rejection builder 1 test, the real headless process example (four independent PIDs), AppHost all-targets check, and protocol/runtime all-targets strict Clippy. A separate harness directly referencing this final main encoder passed the three real frozen-old-decoder tests. The 1,279 declared main source hashes stayed unchanged throughout the main suite; the compatibility run additionally guarded its adapter and complete frozen decoder inputs.

Actual main suite: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-81/main-clean-worktree-validation-r2-20261026/runs/main-clean-r2-01/result.json`. Actual final-main old-peer tests: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-81/main-final-compat-r3-20261026/main-compat-01/result.json`. The adapter changes only Cargo package identity to avoid a same-name/version lock collision and compiles the unchanged original old source.

Earlier Gallery, unformatted port/main, failed runner-count check and first Cargo package-collision receipts retain their original identities. The final source is not retroactively attributed to those older runs. GPU/native behavior, successful B2 grant/publication and remote CI were not run in this batch.
