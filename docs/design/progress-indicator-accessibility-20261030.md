# Read-only progress accessibility

Scope: [Issue #5](https://github.com/TekesApps/TekesUI/issues/5), the dedicated ProgressIndicator gap left by the original ProgressContent delivery. This extends the existing Content and macOS bridge; drawing and host result text keep their existing behavior.

## Contract

ProgressContent publishes a passive ProgressIndicator role. Known progress preserves the existing percentage display string and supplies a finite 0–100 domain. Explicit indeterminate progress keeps the existing “进度未知” display string and omits the domain. Its retained scalar is not an accessibility value.

The existing matched-schema Content wire transports that role, display value and optional range. Existing role encodings and protocol version stay unchanged. Older receivers reject the new role through their existing enum decoder, rather than silently accepting it as Group.

The macOS projection uses NSAccessibilityProgressIndicatorRole. Known progress exposes an NSNumber value and min/max; unknown progress exposes nil value/min/max. Both are read-only, remain outside keyboard focus, and expose no press, set-value, increment or decrement actions. The native publication boundary rejects malformed known numeric values, writable progress snapshots and keyboard-focused progress snapshots.

No new reducer, animation, progress model, focus policy, protocol negotiation, or AccessKit dependency is introduced. Names, descriptions, mount identities and ordinary sibling status text remain owned by the original content tree.

## Verification and remaining acceptance

Affected checks cover Content metadata and passivity, complete wire reply roundtrip and legacy decoder rejection, runtime producer/receiver mapping, native snapshot validation, and an independent mounted consumer through known→unknown→known transitions. Existing consumer coverage also protects drawing and Composer/reading state.

Actual execution results belong in the Issue handoff and artifact receipts. Compilation and offscreen projection do not establish native-window selector behavior, VoiceOver speech, physical IME, GPU pixels, or performance acceptance. Those require separately allocated windows and remain unverified by this candidate.

## 2026-10-30 isolated review handoff

The candidate is based on `8a6a7633eec88af86353ad55da1aa90608faf69d` in the progress-accessibility workspace. Direct component tests cover known→unknown→known metadata and action passivity. Protocol tests assert the entire reply roundtrip, optional range and rejection by a frozen older receiver. Runtime producer tests exercise JSON transport and passive actions; platform tests validate raw native snapshots, including rejection of writable or keyboard-focused progress. The independent composition consumer exercises the original mounted tree and platform projection without opening a window.

A separate exact test-only delta for `native_content.rs` exercises the real `RemoteSemantics::update` with a serialized producer reply, checks stable receiver identities and optional numeric ranges across known→unknown→known, and verifies invalid-range revocation. That file belongs to the cross-process domain, so this delta is handed to the coordinator without modifying its source. The shared `ContentRole` export is also supplied separately. Both deltas are dependencies of complete Rust verification.

At the initial isolated handoff, static syntax/format and pure-Rust boundary checks passed. Rust compilation and behavioral tests had not run: the shared cache is bound to MAIN and no source-switch/build window was allocated to this isolated tree. Generated global metadata must be refreshed by its owner after integration; the old composition audit correctly reports its source digest stale after this review's changes. No native selectors, windows, VoiceOver, GPU or performance checks were executed.

The handoff bundle beside the worktree contains exact source patches, SHA-256 identities, source allowlists, raw static results and a finite build-window runner. The runner uses the registered gallery review-cache policy, its existing build lock and environment, requires the cache owner to remain MAIN, and never prepares/rebinds/cleans a cache. The coordinator must integrate the complete candidate and grant the build window before running it. The runner stops on ordinary failures; the domain lead owns their repair. Passing these tests does not close the remaining parent Issue #5 criteria.

## Allocated MAIN verification result

Subsequent coordinator-allocated Rust windows tested MAIN `77860b59df6eb8dc3432a96dd4bfd1730c1b215c` plus the preserved WIP and the exact integrated candidate. Two compile defects in this batch were repaired: the objc2 method_id nullable return now uses a tail expression, and the producer/receiver test uses a local ButtonTextRenderer adapter instead of a nonexistent library test type.

Protocol roundtrip: 1 passed. Component metadata/passivity: 1 passed. Runtime producer/wire/receiver: 3 passed. Platform snapshot admission: 1 passed. Independent mounted consumer: 7 passed, 1 pre-existing external Theme artifact test explicitly ignored. Scoped Clippy with -D warnings passed for Controls/Runtime/Platform library and tests, the protocol progress_schema target, and the independent consumer progress_content target. The runtime repair affected only its test fixture, so the prior four passing behavior groups were reused. The coordinator's later disclosure fixtures were recorded as unrelated WIP without re-signing prior receipts.

The external handoff bundle retains build-results, resume-build-results, independent-build-results and final-build-results with commands, source inputs and raw results, including both compile failures. All runners exited and released the existing build lock. The global composition metadata refresh remains with its owner. No native selectors/windows, VoiceOver, physical IME, GPU or performance checks ran; these scoped results do not close the parent Issue #5.
