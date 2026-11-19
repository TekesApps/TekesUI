# Original-owner prepared text actions

Scope: dependency 1 of [Issue #194](https://github.com/TekesApps/TekesUI/issues/194). The ComboBox recovery owns its adapter and original design separately. This dependency supplies only private controls interfaces; it does not add ComboBox, Menu, AX roles, a second editor, or another transaction/state machine.

## Existing and recovered contract

The current `action_preparation::prepare_original` already decides, authenticates the original mounted owner once against current state, admits focus/reveal, and consumes the validated effect. TextField previously used its immediate pointer/selection path without the historical decision adapter. TextInput already owns exact native marked replacement, grapheme caret affinity, controlled binding, history and edit-epoch capacity. These current contracts take precedence over the older code.

TextField now exposes the private `has_focus`, `PreparedFieldAction`, `decide_action` and `apply_action` seam. Ordinary pointer/SetSelection calls and the prepared host use the same original effects. Decisions include the original input snapshot, focus and relevant layout/interaction data. A refused pointer does not pause its reader, unmark or acquire capture. Read-only text remains selectable; marked SetSelection remains refused. No focus traversal, Tab policy or external value protocol is changed.

TextInput insertion is split into pure `prepare_insert`, current-editor/binding validation and consuming `apply_insert`. The ordinary insert method uses the same path. Exact owned marked ranges are preserved before grapheme normalization, and the committed caret advances to the next whole grapheme. An IME commit retains its original one-step undo/redo snapshot. Counter exhaustion remains a preparation refusal, or the existing immediate insert panic; it is never a successful fallback.

TextSurface pointer preparation computes selection, optional scrollbar effect and post-effect focus geometry without mutating the input, reading intent, scroll or capture. Apply retains today's pointer_dragging/scrollbar_pointer_active/scrollbar_reveal_suppressed fields; it does not import the historical replacement interaction state. Unicode word/paragraph/shift selection uses pure range helpers and the original epoch counts. There is no restored hover-position state. The original pointer entry point consumes the same decision.

The minimal ScrollPointerDecision uses the current cancellation and scroll_by reducers on a value copy, then applies the authenticated drag/offset on the real ScrollView. Existing scrollbar gesture and one-shot reveal suppression keep precedence. SetSelection geometry predicts the actual reveal gate; pointer geometry accounts for clearing one-shot suppression during its own effect. A focus rectangle must equal the resulting caret geometry, including when a scrollbar prevents internal reveal.

## Recovery boundary and compatibility

Historical references are read-only `gallery-design-baseline` source and the Issue194 recovery excerpts. Only the required private signatures and original-owner operations are recovered. The older insert algorithm would normalize an active combining-mark range too broadly; this candidate preserves the current exact-range implementation. Historical TextSurface interaction and hover fields are not imported. Two now-unused private mutating word/paragraph selection wrappers are replaced by their shared pure helpers; no public symbol is removed.

The caller of a prepared operation must validate at its final original-owner boundary and consume it in that operation. These private values are not native authorization leases. Existing `OriginalEffect` validation is reused; no new manager or generation mechanism is added. The Combo prepared editor operation can hold `PreparedFieldAction` and compare a fresh decision before applying it, while its original View confirmation can use `PreparedTextInsert`.

Only composition/adapters.rs, text_input.rs, text_surface.rs and scroll_view.rs change. The integration executor owns source admission, build allocation and the necessary inventory/hash/line refresh. Do not copy historical or candidate global metadata over the current aggregate. Menu generation/pointer helpers and Combo semantic bridging remain independent dependencies owned by their existing domains.

## Verification boundary

Added tests cover pure preparation, stale editor/binding rejection, unchanged-caret text replacement, marked commit/undo/redo, UTF-16 range helpers, shift/word/paragraph selection epochs, scrollbar capture/paging and reveal suppression, reader/focus preservation on refusal, focus-capacity refusal through the real ContentHost, and predicted versus actual caret geometry. They are test source, not executed evidence until the integration executor grants a frozen MAIN build window.

Scoped rustfmt and whitespace checks are the only executed checks in this source batch. Rust compilation, unit/integration behavior, strict lint, Combo consumer, GPU/native, physical IME, VoiceOver and performance are not run here. Historical #90/#116 receipts remain references to their exact original inputs; they are not transferred as current passes. This dependency can be admitted and tested independently of the complete Combo adapter.
