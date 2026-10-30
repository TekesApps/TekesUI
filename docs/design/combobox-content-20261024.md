# ComboBoxContent original-owner finite design

Implementation baseline: design/reviews/combobox-content-design-20261024/design.md and validation-plan.md; the following main acceptance fixes are authoritative.

## Main finite design acceptance and implementation amendment — 2026-10-24

Main reviewed design/reviews/combobox-content-design-20261024/design.md and validation-plan.md and consulted advisor. Accept their finite original-owner design for isolated implementation, with these corrections authoritative. This is not code/runtime acceptance; original #90 five criteria remain.

Keep original ComboBox/ComboBoxView/FilteredMenu reducer and one retained TextFieldContent/TextSurface. Legal empty/duplicate labels, positional source indices, original lower-case contains order, set_query retaining selected/opening/resetting active, and closed Enter/Down opening policy stay. No new editor/selection engine, async data source or Theme usage.

1. Confirm a candidate through one prepared original Combo+editor operation: validate current candidate, no marked, editor epoch/replacement range and popup identity/capacity before effects, then apply original reducer plus original TextInput decided writes. Extract decision/application at the original View; old entrypoints share it. Do not call model.key(Enter) then discover insert cannot finish, replay Enter, or rollback.
2. Real Commit/Preedit owns text. Required deterministic authorization resources are preflighted before the input or, if text already succeeds, keep query projection synchronized while revoking popup/part authority and explicitly withholding publication. Never lose/rollback text or keep stale actionable candidates after capacity failure. Distinguish fallible controlled set_query/set_items Err-preserves-state from real editor events. Void disabled/revoke paths allocate no new identity; reauthorization needs checked preflight.
3. Consume actual accepted finite #98 prepared action and structured focus admission where pointer/AX opens/selects; source-origin rules remain ordinary/direct versus Tab/AX. Rejection precedes selection/capture/scope/actions. Legacy focus bool or post-action accessibility_focus_requested cannot authorize. Original MenuRuntime still owns gesture/scroll; real Host outside-dismissal down/up barrier needs explicit delivery and tests. Prepare local original View seams independently; do not wait for whole #98 as an extra prerequisite.
4. Separate valid action payload from popup authority. Confirmation changing query and closing popup does not revoke its own accepted selection; capture original index/label/declaration revision and deliver once through existing queue. Specify later programmatic replacements revocation separately. Same-value confirmation produces its original single event; a later legitimate Enter can reopen.

Create a deduplicated finite shared ComboBox semantics bridge dependency: real SDK NSAccessibilityComboBoxRole exists, but complete editable text selector/binding/layout/selection qualification must be implemented and tested. It is not SearchInput or PopUpButton. Root exposes one original editable owner; popup candidates reuse existing option semantics/parents, not extra editors. Matching schema, actual complete old decoder reject, original dirty/revoke and neighbor Composer preservation apply. #111 projected authorization stays separate; bridge alone does not accept remote geometry or #90 adapter.

Implementation order: finite bridge in parallel with #90 original View checked seams/adapter; action integration consumes accepted #98 needed seam; fixed same-source public/GPU comparisons follow. Main integrates shared hunks and metadata serially; no publication or downstream pin implied.


## Main #116 matched-schema editable ComboBox bridge — 2026-10-24

The current WIP adds ContentRole::ComboBox, wire SemanticRole::ComboBox and the actual native AccessibilityRole::ComboBox/NSAccessibilityComboBoxRole. This is an editable original TextFieldContent/TextSurface capability, preserving draft, binding, UTF16 selection/marked state, complete shaped text layout and original selectors in both platform and AppHost projections. It is single-line; multiline wire layouts reject. Select remains noneditable and the five selection-family roles acquire no text capability.

Outer protocol VERSION and all24 existing role encodings remain unchanged, but a peer receiving the new role must implement the matched schema. The exact immediately-pre-ComboBox complete decoder rejects both root and part snapshots after a valid prefix; unchanged outer VERSION is not old-peer interoperability. Roll back the matched peer set together. Public exhaustive matches on the role enums need the new variant. This WIP bridge is not a public release, immutable downstream pin or ComboBoxContent adapter. The original library type inventory has no newly added top-level type; the enum variants carry the finite API delta.

Keep existing ComboBox View/Workbench registration and unaccepted Content adapter status until#90 completes. #98 original prepared action contract is now separately accepted; #111 allocation/current-geometry authorization remains independent and unaccepted. SDK object/hidden callback window evidence is separate from shown-window, physicalIME, VoiceOver, GPU and performance. Main root acceptance/evidence must be linked separately; no old seal is re-signed.

Main original five criteria accepted after serial integration. Evidence: `design/reviews/combobox116-main-intake-20261024/root-acceptance.json`; actual527 package libs+30schema+9projection+25public tests=591, native SDK checks146ComboBox plus original related contracts, three related strict lanes. Original#98 List Press regression was exposed, reopened and fixed without changing its assertion; extra1AppHost+4List focused tests pass. Source/program/depinfo hashes retained. No shown window or physical input claim.


## Main finite acceptance — 2026-10-25 / #90

ComboBoxContent is integrated in the authoritative main worktree with its original model/View owner and retained ContentHost. The #108/#90 combined main window passed 462 CPU/consumer/Gallery/binding test invocations, two strict commands, and four actual Metal/CoreText GPU tests covering 712 original-View equality cases (Checkbox324+324, Combo32+32 default/external). Sources1689 were identical before/after; actual programs/dep-info/generated hashes are archived. Per-Issue original criteria remain separate. Receipt: `design/reviews/priority-intake-queue-20261025/main03-validation.json`.

Canonical public API inventory is387 types (45 components/52 roots unchanged); original default/SVG bindings passed3 each. Workbench retains original component identities. Scoped Checkbox/Combo composition rows carry exact current source anchors/hashes in their finite addendum. Old whole-library capabilities/probe/audit source identities are unchanged and stale aggregate freshness is tracked in #8/#14; this acceptance does not claim a fresh45-row audit. Shown window, physical IME, VoiceOver, performance and publish were not run; no PR/commit/push/release. Parent synchronization: #56.


## Current-main recovery candidate — 2026-10-30 / #194

Historical #90/#116 finite acceptance remains attached to its original inputs and programs. Current-main recovery reuses the original fixed string labels and single editor; it does not add rich items, asynchronous search, or another editor. This isolated candidate restores the adapter and original public consumers, plus precise ComboBox reducer/View and export/payload hunks. Shared TextField preparation, Menu checked-generation/pointer seams and the ComboBox matched-schema semantic bridge require the current owners to admit compatible hunks. SelectContent is not a direct dependency. No Cargo, GPU or native validation has run on this candidate; old passes do not prove this changed baseline. Physical IME, VoiceOver and performance remain unverified. PR=null.


### Current Host contract adaptation — 2026-10-30 / #194 revision 2

This source candidate retains the current generic Host transaction engine. ComboBox alone opts into ordinary rectangular input and validates its retained editor's prepared hooks before binding the wrapper's allocation-free lifecycle/revocations. Curved Panel eligibility is not added. Same-editor ComboBox pointer down still uses the prepared original Combo/editor operation; unrelated text editors retain their current same-owner route. Cross-owner SetExpanded receives the original prepared focus gate before opening, while same-owner text commands retain existing authority. Retired Combo popup capture rejects its trailing primary up without canceling the editor's text/mark state or replaying a background action. These exact Host hunks are prepared for the single integration writer, not applied to MAIN. Assigned dependency owners remain unchanged. Rust/behavior/public consumer validation is pending their compatible frozen interfaces.


### Complete current-source closure — 2026-10-30 / #194 r4

The source package applies the original prepared-text owner patch plus the
accepted window02 test-only scrollbar fixture repair, Menu frozen superset,
Combo-only 19-file AX bridge, and Combo subject r2 to current integration
baseline 0aec889. Each shared block has one frozen owner version. Current
WindowLight fixture code is retained; no historical shared file replacement.
The public CPU and original-View comparison GPU consumers are mounted through
the same current Host. The original Combo capacity regression now uses the
current allocator's thread-local failure seam; all original assertions remain.
No Select/Search/selection/navigation business implementation is imported.
The source package is uncompiled: static assembly/parsing and patch replay do
not establish behavior, strict, GPU, physical IME, VoiceOver or performance.
