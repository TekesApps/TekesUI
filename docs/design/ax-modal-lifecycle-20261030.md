# Modal AX scope and closed-window lifecycle

Scope: [Issue #5](https://github.com/TekesApps/TekesUI/issues/5), limited to the existing macOS accessibility bridge and its original ContentHost publication path. This is the next batch after the separately verified ProgressIndicator delivery. It changes no component focus traversal, input routing, Menu/Disclosure consumer, runtime/protocol or global metadata.

## Current contracts and evidence boundary

ContentHost already limits semantics to its active modal/overlay scope. DialogContent owns the original open/close model; show_dialog/close_dialog preserve the invoker mount and retained child state. ContentAccessibilitySnapshot and WindowAccessibilityPublication already carry that original tree to Window; native elements route actions through the original publication and handler. Existing source tests cover Dialog model/actions, modal API routing and caller identity. These are not evidence that the new native probe has run.

The broader existing schema covers Button and composed icon actions, selection roles, TextInput, List, Menu/MenuButton and Dialog, with name/value/disabled/selected/expanded/invalid/required metadata and real original actions. See composition.rs, composition/button.rs, composition/adapters.rs, composition/menu.rs and platform content_accessibility.rs. Native role/name/value/action coverage and actual VoiceOver acceptance for all these families remain separate from this limited batch.

## Concrete failure and smallest repair

At the inspected MAIN source, AppDelegate.windowWillClose set Shared.closed but retained the whole-batch publication, AX nodes, shaped text layouts and cached native elements. Shared.finish revoked the publication only later. Most native node/children queries checked closed, but TekesView.accessibilityFocusedUIElement directly searched its cached focused node without that guard. A client retaining the view could therefore receive its old focused element after actual window closure, and a retained publication clone could remain current until application teardown. This is a direct close path, not a proposed race.

Shared.revoke_accessibility now revokes and removes the existing publication and clears the three AX stores. Both windowWillClose and finish call this one teardown responsibility after marking closed. The view's focused-element query also checks closed and preserves its existing self fallback. No input/editor state is reset, no control state machine is copied, and no extra notifications are posted after closure.

## Regression paths

`tests/ax_modal_projection.rs` uses the original DialogContent, ListContent, Button, SwitchContent, ContentHost and platform publication adapter. It checks hidden closed Dialog nodes, modal-only projection and background action rejection, old geometry authority revocation, original Switch action/state, invoker restoration, reopen identity/state and inactive-host withdrawal. A local text backend supplies only measurement/paint services for this data test; the fixture owns no replacement Dialog or focus model.

`examples/ax_modal_lifecycle.rs` is an explicit native example, so ordinary test execution cannot accidentally launch it. In an allocated native window it uses public App/Window, publishes the original Host snapshot through the production Window setter, finds the real exported children, and invokes three native Press selectors. The ordinary handler uses publication.dispatch for the original actions and original host show/close callbacks. It checks modal role, hidden background, selection value, restored native focus, then calls Window.close. Before App.run finishes it requires saved publication revocation, no exported children, view focus fallback and inert retained nodes; it repeats retained-client queries after finish. It creates one real window and submits no GPU work or physical input. It does not verify actual VoiceOver speech or externally observed AX notifications.

The existing notification path compares structure/focus/value/selection in TekesView.accessibility_nodes_changed and publishes native notifications. This batch neither changes that algorithm nor claims an external notification observer has verified it. Actual notification delivery, VoiceOver, physical IME, GPU pixels and performance remain unverified.

## Identity and execution plan

The isolated checkout remains at `8a6a7633eec88af86353ad55da1aa90608faf69d` plus preserved WIP. The three production files and platform manifest were compared byte-for-byte with the inspected MAIN inputs before editing; current integration labels do not replace those hashes. The handoff records MAIN-before and candidate-after identities for every allowed file. Prior Progress patches and receipts stay untouched.

No Cargo, native executable, GPU, desktop input or system setting changes ran in preparation. Rustfmt syntax/format, patch applicability and pure-Rust source checks are recorded separately from unexecuted behavioral tests. The finite handoff runner prepares the data test, scoped strict checks for feature/default builds and the native example binary only after coordinator integration and an allocated shared Rust window. Native execution needs a separate allocated window through the existing desktop guard. Do not create a target/cache or change bindings to test this isolated source.

Accept the source batch after the new data behavior and scoped strict checks pass; keep actual native-selector acceptance and parent Issue #5 open until their corresponding allocated execution succeeds. A missing native window does not justify new control behavior or replacing VoiceOver with programmatic AX.

## Integration repair verification — 2026-10-30

The original `311ceb4d…` eight-file patch and original assertions were retained. A real feature Clippy failure in the native example (E0277/E0308) was returned to the original AX domain; it supplied only the two retained-object borrowing fixes, patch SHA-256 `4a5b54e79f91054296b9eaa27442bbd50db9ea809af1792a1c39df4d21782597`. The repaired example SHA-256 is `afe0686f9b5007dc9147c9c78d37e23bcb7583f07d30f5dd0ab04999cfd0abd4`.

Actual repair inputs were MAIN HEAD `29b62511d2797e0389cc4ed6aae7f76904093a31` plus those exact source patches. Feature strict Clippy (lib/test/example), default lib strict Clippy, and the content-accessibility example build all passed. The earlier two actual passing data tests were reused after byte comparisons confirmed unchanged test/fixture/production/dependency inputs; the example-only repair does not affect those tests. Original failure logs remain at SSD `artifacts/delivery/integration-executor-20261030-07/ax05/`, and repaired input comparisons/commands/executable copies are at `artifacts/delivery/integration-executor-20261030-08/ax-repair01/`.

The compiled native example has not been executed. External AX notifications/selectors, VoiceOver, physical IME, native window behavior and performance remain unverified; no native acceptance or maturity value was promoted. The existing composition audit only updates the three changed platform source hashes and three real shared.rs line anchors.


## Allocated native verification — 2026-10-30 round04

The fixture-only timing repair was admitted as MAIN
`8c5c3bab5b17c5ddc1a04a5226216a27d28e3f1a`; example SHA256
`78eb9073ff133838c538b0b14a3bcbca85eb804addd0b889c1b4ba56532cf7cf`.
Scoped example Clippy with `-D warnings` and Cargo JSON build passed in the
allocated original helper/cache environment. The actual Cargo program was
archived with SHA256
`afe01421d17a30b94ed1c11b5ff01959cc16264b69a0fea615e4efb44c8188d5`.
Earlier data/production/default-library passes are reused only for unchanged
inputs; this new fixture was independently compiled and run.

One separately allocated native attempt exited 0 without timeout or detected
user-input interruption. All four fixture screen queries reported actual
external display ID2; the named probe CG frame was X2072/Y239/800x632. Original
modal role/value/focus, three native Press selectors, close revocation/cache/
focus/stale-client assertions and after-App.run retained-client assertions all
completed. `AX_MODAL_CLOSE_COMPLETED` appeared once before the original final
success marker. This is finite same-process native selector/lifecycle evidence.

Raw result, independent final build binding, stdout/stderr, actual window/display
observations and immediate release receipt are preserved under
`/Volumes/HP P900/DEV/TekesUI/artifacts/issue-5/native-round04-once/`.
`result.json` SHA256 is
`b782d920d61ed09fa603788e69ebaf52e926b29f9eed95f75447e588c9b46d4e`.
Owned PID83764 was reaped. Both original guards released, and their existing
advisory locks could be acquired and immediately released after context exit.
No retry, Cargo, settings, MAIN/Git or metadata write occurred in the native
window. Earlier role/subrole abort and round03 timeout evidence are unchanged.
The resource/source lease ended before offline evidence analysis.

External AX notifications, physical IME, VoiceOver, GPU pixel acceptance,
performance and other Issue #5 acceptance criteria remain unverified. Keep the
Issue open; this result does not replace those acceptance categories.
