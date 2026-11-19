# #26 Controlled update preflight

`TextInput::check_external_value` and `TextSurface::check_external_value` accept the same `TextEditBaseline`, `DraftRevision`, `ExternalSelection` and `ExternalUndo` arguments as `apply_external_value`, returning the same `ExternalValueResult`. Both paths use the same private preparation function: normalization, identity/publication checks, baseline/composition checks, grapheme selection mapping and history flags are not duplicated in consumers. No stored prepared handle, editor clone, queue or new UI layer is introduced.

The check takes `&self`. It does not acknowledge a publication, advance editor/binding epochs, change text/selection/history/preedit/clauses, reveal a caret, shape text, invalidate layout, move scroll, or dispatch focus/reading changes. `Applied` is a prediction, not an action. Invoke `apply_external_value` on the original instance with the original baseline and arguments; apply validates again. Do not yield, dispatch events, switch the document or mutate the editor between a successful check and a larger synchronous commit.

The existing exact duplicate acknowledgement remains `Unchanged`, even after local editing or during composition. It does not replay selection/undo changes. A foreign editor is still rejected first. Revision, InvalidIdentity, Composition, StaleBaseline, DocumentResetRequired and Selection retain their existing precedence. Normalized text and raw publication identity remain distinct. Direct public-field edit-and-restore ABA remains outside the existing controlled contract; method-based edit/undo ABA is fenced.

All public TextInput/TextSurface consumers can use this contract, including TextField/TextArea/SearchField wrappers and read-only content. Read-only/disabled fence user input, not an explicitly authorized external publication. There is no automatic component migration, new token or Agent-specific policy.

## Complete public consumer

`examples/composition-consumer/src/controlled_reading.rs::publish` reuses the actual heterogeneous `scroll_updates::Scenario`, original TextFieldContent/TextSurface, stable Content IDs, and the existing reading token:

1. Resolve the original editor and check its controlled update without mutation.
2. Check application tree/type/ID preconditions and construct the new transcript item.
3. Call `update_scroll_reading_document` with the retained expected token. A stale reading request does not enter the callback or apply the editor update.
4. In the synchronous callback, apply the same editor update, insert/reorder the already-validated transcript item, then let normal layout restore reading position.

No copied editor validation, internal-field access, alternative native editor, pixel compensation or Agent business concept is required. The fixture owns all mutations during this turn; its invariants/assertions are not a promise to roll back arbitrary caller callbacks. Existing reading callbacks still install the accepted version before running; an arbitrary application error/panic after mutation is not rolled back. Applications must preflight *all* fallible input/model/tree work before invoking the callback.

Public regressions cover every current conflict, duplicate acknowledgements, legal apply predictions, normalization/selection/history flags, document changes, publication replacement, composition changes and edit/undo ABA. The independent consumer additionally proves invalid editor input, stale reading tokens and duplicate tree IDs leave editor/Transcript/reading state intact. Real CoreText/GPU checks exercise the original editor with synthetic preedit across themes/brands/widths/type sizes; this is not physical IME, native hearing, ClientKit visual parity or performance acceptance.

Current receipts live in `design/reviews/controlled-preflight-20261020/`. Any local portable candidate is separately identified from current all-WIP and from bd3; bd3 is immutable and excludes this fix. A local package, sender replay and technical AgentKit acceptance do not replace an exact public commit plus its CI. No public pin or Issue closure is implied by this document.
