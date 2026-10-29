# Original Content context menus (#44)

Scope: extend the existing MenuContent/MenuRuntime and immutable MenuSelection
contract from #36. The effective acceptance criteria remain in
[Issue #44](https://github.com/TekesApps/TekesUI/issues/44). This does not reopen
menu declarations, text layout or ordinary menu triggers. Passive hover content
is a subsequent batch under [#46](https://github.com/TekesApps/TekesUI/issues/46).

`MenuContent::context_menu(name, entries)` declares a real menu node in the same
ContentHost. It contributes no trigger view, focus target or primary action.
`bind_context_menu(owner, menu)` attaches the original mounted owner to that menu.
There is no hidden Content trigger, second host or consumer-supplied popup frame.
The retained public `trigger` model supplies the existing availability/name
contract only; its button input, layout and painting are not used in this mode.

The host captures original owner/menu mount identities. Native secondary pointer
and macOS Control-primary use the original clipped/curved owner hit path.
Shift+F10/Menu key intents use the focused original owner. Explicit keyboard or
assistive intents use `request_context_menu(binding, source)` and the same policy.
Native adapters already preserve characters_ignoring_modifiers in KeyInput;
U+F70D with Shift and U+F735 are recognized without changing platform sources.
The host retains pointer/owner-rect anchors and delegates viewport clamping,
submenu layout, paint, capture, Escape, disabled rows and reduced motion to the
existing MenuRuntime. The host retires an open context menu on primary down
outside every original panel, including delivered window-margin clicks beyond
MenuRuntime's visible bounds; the dismissal consumes the click and preserves
the original focus and selection.

Opening/closing produces no MenuSelection. Selection retains immutable value
path/model revision and a `context_request()` with original owner/mount, request
source and opaque anchor identity. Delayed A/B actions cannot be retargeted by
current selection or later openings. Model replacement revokes context actions;
removing/remounting the original owner drops its undrained records and invalidates
the binding. A new binding is required after remount. Hide, disable, anchor
movement/clipping and window deactivation retire open context menus. Scrolling
outside menu panels retires the context presentation before ordinary scrolling;
menu-internal scrolling retains MenuRuntime ownership.

Opening retains the editor focus/selection, and never commits or cancels marked
text. Marked/composing input explicitly returns `Composing`; active pointer/native-key-chain
capture, stale/no layout and unrelated overlays return distinct refusals. The
request path respects the existing active/modal/popover scope and cannot select
a row behind an interactive sibling. The composing editor retains normal key
and native text callbacks; the context menu does not create a second editor.

Verification sources are `crates/tekes-ui-controls/tests/context_menu.rs` and
`examples/composition-consumer/{src,tests}/context_menu.rs`. The independent
consumer maps captured original owner IDs to session IDs, keeps primary selection
A while requesting B's menu, and uses the existing TextFieldContent for A's draft.
It contains no popup coordinate compensation or copied menu state machine.

Scoped programmatic verification has passed: 13 context-owner tests, 57 platform
unit/bridge tests, 42 affected control/menu/action/pointer tests, 2 independent
CoreText consumer tests and 60 existing consumer menu/overlay/native-translation
regressions (174 in the final scoped run). Two earlier action-freshness tests are
reused because their implementation, inputs and contract are unchanged. Controls,
platform, consumer and native-example scoped strict checks pass. The new native
example builds; AppHost, Gallery with composition-review, and affected native /
toolbar examples compile. Raw first failures and repaired results remain under
the coordinator's SSD `artifacts/issue-44/domain-20261029/`, with actual source,
command, cache/configuration and archived executable identities in the formal
handoff. No whole-workspace CI was run by this domain.

The macOS bridge now exposes the original-row AXShowMenu selector only for an
eligible original owner's current context attachment. It forwards ShowMenu to
the same SDK policy and never converts AXPress into a menu request. Old native
keys, disabled/inactive/remounted owners, unsupported parts and marked/composing
input are refused. Remote wire nodes do not advertise this local capability;
the existing remote protocol explicitly rejects this unsupported intent without
sending it or mapping it to Press. This batch does not extend wire schemas.

The native consumer is `examples/composition-consumer/examples/context_menu.rs`.
`TEKESUI_CONTEXT_MENU_EVIDENCE` enables source-owner/mount/state journals;
`TEKESUI_CONTEXT_MENU_NATIVE_SELFTEST` invokes actual published native ShowMenu
selectors through the existing platform selftest adapter. Its program is built
and archived, but has not been launched in this domain window. Native window
dispatch, physical Chinese IME, VoiceOver, GPU pixels and public exact-pin delivery
remain separate coordinator-owned acceptance/delivery categories. Programmatic
Preedit or AX API calls do not prove physical IME or VoiceOver. Issue #44 stays
open until every effective criterion has actual evidence. #46 is not part of
this batch.


## Remaining native software acceptance batch

The coordinator ran the archived original AX selector program `9c7d3c...` in an
allocated native window. Its raw 93-state journal and stderr independently
confirmed original B/A ShowMenu selectors, 21 B-open states preserving A's
primary/draft/focus/selection, exactly one B/pin action and Escape without another
action. This evidence is limited to that native software AX scope.

The next isolated candidate extends only the existing platform selftest and the
independent review consumer: published-frame local secondary clicks, native
window key events with retained modifiers, actual native menu role/frame/state
queries, and one fixed 23-checkpoint script. The review setup uses stock
ScrollContent/StackContent and the original retained rows; it adds no SDK menu,
focus, hit-testing or popup capability. Checkpoints cover submenu/disabled rows,
Shift+F10, scroll, anchor clipping, removal/true remount with explicit rebinding,
narrow edge clamping, outside close and window deactivation. Immutable original
selection records and raw native events are retained beside state/journal and
per-checkpoint results. The public business consumer still owns session values.

The final extended program passed all 23 software-native checkpoints and four
actual Cocoa AX frame/role/state audits under the existing shared build/GPU locks,
SSD routing, occupancy and HID idle guards. Its SHA256 is
`de6ec0b9049687e1a3c810bc5d11dfa68aa5d1f3df20a9347636fe224580335d`.
Raw evidence is retained in `artifacts/issue-44/domain-20261029/native-outside-final-03-runner/attempts/run-20261029T171146.662239Z-a2b44519/`.
The coordinator checked the original logs and all checkpoints: secondary input
uses a coherent CG-backed event through the actual NSView right-mouse selector;
Shift-F10, scroll and outside-click callbacks use the native routes. The final
immutable sequence is B/archive, B/pin, B/pin across original/remounted owners.
All 324 journal states retain primary A and its draft. Deactivation clears focus
according to the existing cancellation policy; reactivation invents no owner.

The final context tests (15), controls strict and example build/strict passed.
Unchanged relevant menu/action, CoreText and platform checks are reused with their
original source identities. Three original failures remain preserved: incoherent
synthetic mouse metadata, a clipped test fixture, and the actual window-margin
outside-dismissal defect fixed in this batch. Physical Chinese IME, VoiceOver,
pixel acceptance and performance remain independently unverified. This is a
partial Issue #44 delivery using Refs; it does not close those acceptance gaps.
