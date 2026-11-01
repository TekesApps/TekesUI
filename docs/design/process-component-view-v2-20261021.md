# Negotiated ProcessComponent View checkpoint

Status: experimental public API and independent real-GPU consumer verified. Not a complete native editor/container release.

`ProcessComponentOptions::with_view(window, view, required)` attaches an optional View to the same session, process owner and service connection. Ordinary headless Start and Ready messages retain their existing shape. A View-capable factory can be used headlessly without constructing a Renderer or producing a frame. StartView/ReadyView bind the host window, View identity and supported native capabilities; requesting unsupported IME/keyboard/AX fails explicitly.

One module actor owns business state. The existing PanelModule prepares a local Scene, moved to a GPU worker inside that child process; Scene is never serialized across processes. GPU allocation begins on the first render job. Services, generation-bound toolbar Invoke and Stop are handled by the actor while the two-frame consumer budget is occupied. The existing owned IOSurface rights, queued View lane, GPU completion retirement and ProcessControl are reused. The actor recovers its atlas before publishing a finished frame, so returned parent credit cannot race actor ownership restoration. Exit wake callbacks run after releasing the state mutex.

View queue/credit notifications also synchronize with the service writer's existing
state mutex. The View queue has a different mutex: notifying without that handshake
can slip between the writer's empty-queue check and its conditional wait, leaving a
new layout pending without another service message to wake it. Render leases are
released outside the service mutex on failure/exit; malformed frame metadata is
validated before taking its lease. This keeps credit-return callbacks from reentering
the writer lock. A controlled real-queue regression fails the old notification path
and checks terminal lease cleanup. Actual original IPC/AX/projection/scroll and mixed
native checks passed with this repair; CI's earlier clip-restore timeout remains a
failed historical result. Evidence: SSD
`artifacts/issue-48/main2bcfc17-view-wake-20261028-01/{red01,green01,native01}`.

`ProcessContainer::new_view` adapts this negotiated path through the existing Component mount/unmount, arrange and prepare contracts. Module ViewInvalidated is coalesced and causes the next Content arrangement to request a fresh frame. Toolbar actions use the independent validated Invoke lane. `new_v1` remains an explicit compatibility entry. Pointer interception is still an explicit opaque opt-in: neither profile claims remote consumed/disposition or transparent hit testing.

Independent replay: build the `process_component_view` example in the shared review cache and run it. It checks headless use of a View-capable definition, toolbar before any frame, Invoke while two surfaces are held, actual imported black pixels in 16 render/retirement cycles, invalidation take-once, Stop with an unretired frame, explicit unsupported IME rejection, and the real shared Component mount/arrange/prepare/command/invalidation/unmount path. This is public API and real GPU evidence, not desktop or Gold7 experience acceptance.

Original headless service, V1 queued/standard Component and six original GPU cases remain regression consumers. Exact logs are retained under `design/reviews/process-component-view-v2-20261021`; source-only checkpoint publication must not promote the old frozen AgentKit delivery.

Remaining: View-scoped/multiple toolbar contributions; remote event disposition; full keyboard/scroll/IME/AX; final AppHost and AgentKit native integration; external-display interaction and matched Gold7 acceptance; final independent consumer verification. No performance test was run.


## Finite Content foundation intake (#48, 2026-10-28)

This section describes the source intake; the checkpoint verification above remains attached to its original source. Current-main execution and downstream native adoption are separate acceptance records.

`ProcessModule::content` opts a negotiated View into the original child `ContentHost` plus its existing text backend. The actor applies bounded data-only Content requests to that same tree, then returns disposition, capture identity, focus epoch, UTF-16 text/marked/selection geometry, reading results and semantics. No Scene, resource ID or editable replica crosses this lane. `ProcessContentBridge` and `ProcessContainer` consume the stamped reply through the existing container lifecycle and platform `content-accessibility` adapter. Session, View, viewport, mount, capture acquisition and editor binding retain their original fences. Hidden/inactive scopes revoke publication and input authority.

V1 remains the explicit opaque-pointer profile. The new `content_pointer=false` capability is omitted from JSON, preserving its old wire shape; requesting true requires a matched schema and explicit negotiation. Curve V1 remains reject-only, including valid offers. Remote semantics cover exactly the roles currently public in Controls (Group, Text, Image, Button, Link, TextInput, Slider, Switch, List, ListItem, Dialog, MenuButton, Menu and MenuItem). Image carries only informative alternate text or explicitly decorative empty text, never an image/GPU resource. Unknown roles reject the complete reply; malformed remote Slider domains or Image alternate text withdraw the complete projection. Virtual Slider parts use the original finite range; the existing local Slider range query retains its prior inactive-window behavior.

The Component defaults remain local: optional snapshot/ack, focus-boundary, delegated-key, delegated AX-focus, bubbling barrier and virtual range hooks opt in only the process adapter. Delegated text keys may return Deferred; only the matching original ack may finish fallback, exactly once. Cancel, focus/mount/binding changes and transport failure cancel that original intent instead of replaying it. Clipboard claims reuse the original key-frame ownership once; no separate clipboard model is introduced. Pointer focus intent is emitted only when the existing focus admission accepted it; preserving-focus controls do not manufacture parent focus.

The independent `examples/process-content-consumer` retains the original real-IPC and mounted-container scenarios. Finite acceptance uses its non-window modes and the protocol/runtime/platform tests; native-window, physical IME/VoiceOver, three-container visual/native matrices, AgentKit integration, final published pin and performance acceptance remain outside this foundation intake. The current local Image/AppHost and atomic platform snapshot contracts are retained, not relabeled as remote acceptance.

## Bounded mixed native consumer intake (2026-10-28, Issue 48)

`--native-scroll` mounts three actual IPC instances of the original Reader: nested
ScrollContent/history, Message and Other editors, preserving-focus Button and Popup.
The existing non-window Reader retains its original capabilities and focus policy.
The window adapter publishes the existing ContentAccessibilitySnapshot and atomic
Window publication, retaining the original geometry lease. Raw node publication is
insufficient: legitimate clipped zero-sized nodes otherwise reject the entire AX tree.

The opt-in `TEKESUI_PROCESS_NATIVE_SELFTEST_EVIDENCE` script exercises real AppKit
events and native AX selectors; coordinate it under the original shared locks and an
idle desktop. Actual bounded acceptance records auxiliary mouse/AX activation once,
marked UTF-16 text through append/scroll/resize, two container-level Tab transitions,
window deactivation revocation, explicit user re-entry and child-process cleanup.
Deactivation deliberately cancels focus; the script does not require automatic restore.
Container-level Tab is not evidence that the target child editor acquired focus.
Independent native nested-scroll/bubbling, full child Tab entry, physical IME,
VoiceOver and downstream adoption remain separate gaps. No full Issue 48 closure is
claimed. Evidence: SSD `artifacts/issue-48/maind29f654-native-mixed-candidate-20261028-01/`
(`main-cpu08`, `native-round06`); earlier failed runs retain their original identities.

The subsequent Issue 48 child-entry fix retains the actual first history target:
an internally scrolling readonly Content surface enters through its visible viewport
without resetting its reading offset. The original factory red case now passes.
Actual AppKit Tab now enters container 201's history and then its Message editor;
the native text query reports `draft` and the original child focus is EDIT. The
strengthened unchanged-scene script, affected Controls focus/reading regressions and
strict checks passed in `artifacts/issue-48/mainf10df2b-child-focus-20261028-01/`
(`focus-fix01`, `native01`). This supersedes only the child-entry gap above; independent
native nested-scroll/bubbling and the other unexecuted Issue 48 scopes remain open.

The separate opt-in `TEKESUI_PROCESS_NATIVE_READING_SELFTEST=1` (with the existing
evidence-directory variable and `--native-scroll`) retains that complete scene and
checks original nested-wheel paths. Actual native `+4` nonprecise lines move only
the first history by 128 logical points (four size-row-height steps); `-4` restores
its prior offset, and another `-4` at the boundary returns unconsumed. Parent and
neighbor offsets, editor values and focus stay unchanged; all three children are
reaped on close. Exact event/disposition and snapshot assertions are retained with
the actual program in SSD `artifacts/issue-48/main44cabb3-native-reading-20261028-01/`
(`cpu02`, `native02/verify_native_reading.py`). The first script's reversed direction
expectation remains a failed acceptance even though its process exited normally.
This closes only that independent native wheel gap, not the remaining original AC.

Issue 48 AC4 also has an independent `--reading-anchor-only` mode in the same
public consumer and CI step. Three original IPC Readers use the existing CoreText
Code/mono backend and reading transaction APIs. Actual append/prepend, narrower
width, type-scale 1.5, restoration and explicit latest preserve the original item
mount/local position and composing editor. The first actual run exposed an 864 pt
width-reflow displacement: a prepended edge sliver replaced the stable anchor.
The bounded shared capture fix retains the original visible item until new input
or actual scrolling, and preserves it before document revision advancement. Public
reading/focus/composition regressions and strict passed; actual three-child anchor
error is 0 pt while page height changes 1314→2178→3915. Original headless scroll and
both real native scenarios passed after this relevant shared-source change.
Receipts: SSD `artifacts/issue-48/main33d35b3-reading-anchor-20261028-01/`.
This is the original item-local contract, not a new text/scroll/proof engine or a
claim about physical Pinyin candidates, crash recovery or all Issue 48 acceptance.

The finite AC5 `--reading-lifecycle-only` mode uses three original mixed Readers
with negotiated semantics and actual imported child images. It edits the original
editor, hides/shows without replacing its process/session or saved value, changes
viewport and checks both the bridge fence and an old-viewport request sent through
actual IPC. It then deliberately kills only its own first Reader PID, observes
Failed and the expected prepare error, removes/remounts that original component,
and verifies a new PID/session/parent mount. Retained old-session input and AX
callbacks are rejected; one fresh Auxiliary press produces exactly one activation
and no submission. The other two Reader processes/values survive; all owned PIDs
are reaped and imported images return to the renderer's original count.

Actual strict, original consumer fixture tests and this complete finite lifecycle
case passed on `958ed1a` plus the test-only delta (`cpu01`). A subsequent finite
extension (`cpu02`) retains an actual original Reader frame at viewport1, closes
that connection, starts the same original Reader under a new session at viewport2,
then requires the old frame import to fail before GPU import. Both workers are
also reaped and image counts remain at baseline. This exercises the original
closed-owner frame fence, not a fabricated response or a frame rebound to the new
session. Strict and the affected lifecycle mode passed again; unchanged fixture
and native results retain their identities. Evidence is in SSD
`artifacts/issue-48/main958ed1a-reader-lifecycle-20261028-01/{cpu01,cpu02}/`.
No native-window or physical IME acceptance is added by this headless IPC/GPU
resource case, and no runtime API is changed.

Rectangular parent Scroll admission uses the default-false `Component::rectangular_input_geometry` opt-in. ProcessContainer declares it only for ready, negotiated Content input; the original current focus/reveal checks still run. This does not grant prepared-action purity or curved Panel support, and Curve V1 remains reject-only. The public consumer retains its real outer Scroll and Grid, with a valid arranged translation contract for its fixed row.

## Logical AX focus during paint-only invalidation (#48 / #198, 2026-10-31)

Logical keyboard ownership and current input geometry have separate lifetimes.
A child AX action can accept focus and invalidate its collection paint in the same
reply. The containing Host must still be able to admit that acknowledged owner
while the new paint is pending; it must not treat the paint invalidation as a
failed child focus request.

The bounded normal-runtime path applies only when the existing AX focus receipt
and one-shot content-route pointer are present, the cached reply reports a child
Host owner, the process remains available, and only repaint is pending. Geometry
and deferred-input invalidation must be clear, and this path is limited to the
ordinary non-Curve View. The original Host prepare/apply/reconcile policy still
decides admission. Its existing `set_focus` consumes the receipt and uses the
acknowledged-child path without another Focus RPC. No focus call, replay, new
receipt field, protocol event or reducer is introduced.

`input_ready`, bridge and semantic geometry admission remain unchanged: the new
frame must pass the original acknowledgement before geometry-dependent callbacks
resume. Hidden/inactive, stale session/View/viewport/mount/part, cancelled input
and Curve admission keep their existing fences. A consumed response alone, or a
public `focus()` result of `false` (which reports only `changed`), does not prove
that a Host owner was installed.

The actual diagnostic source `df5e27e1db6551b821d02428acaaed5b3c872f70`
showed the failure before clip restoration: stderr lines 1611–1627 correlate child
Focus acceptance and its delivered owner/request receipt; line 1628 records
repaint pending with input unavailable; lines 1631–1632 show the containing Host
rejecting admission as `UnavailableTarget`. No `set_focus(true)` followed. The
restored header later had raw child focus and a nonempty frame, but projection
correctly remained unfocused because its parent focus input was false. Final
Cancel/blur records were teardown, not the initial failure. This evidence remains
attached to that failed diagnostic program; it is not a passing normal run.

The one-file Runtime adapter repair was source-admitted at
`089dc65d958d62900fc68ba6e7ce41f603f05200` after bounded Core contract review.
That exact Runtime source subsequently passed its two registered receiver tests,
SVG example strict/build and one normal real-IPC run of the unchanged Table
consumer. The run exited 0 with one `TABLE_PROCESS_PASS` and all five malformed
receiver modes rejected (`ordinal`, `orphan`, `duplicate`, `kind`, `budget`). Its
normal program SHA-256 is
`ffca741cbdc18c3adbc740f2868d1001f68eca1e6c0275ad7d7d234a8c545045`.
The original restore/business-state, stale identity and inactive-scope assertions
were retained; this is finite validation of the acknowledged-focus repair for
that consumer, not a new per-delivery trace or a general acceptance claim.

The tested source remains `089dc65d958d62900fc68ba6e7ce41f603f05200`;
the documentation-only successor `853eed2f20792a2c1c64ae485b5aa82f8f650de5`
is not relabeled as a compiled input. The old diagnostic restore assertion
failure and its later `NOT_REACHED` gates remain attached to the failed diagnostic
program. This result establishes neither publication nor native-window, physical
IME, VoiceOver or performance acceptance, and does not expand the existing
Issue 48 acceptance scope. Primary receipts: SSD
`artifacts/issue-48/table-process-actualTEMP01-analysis-r1/` and
`artifacts/delivery/integration-executor-20261030-09/table-runtime-ack-normal-window01/run/integrator-key-acceptance.json`.

Public source audit (2026-11-01): main `eec9aebe555a85a4caecf57034d331def2bb867d`
contains the repaired container with SHA-256
`05db7983122cf203fa534a8ea69ab4a5be7277188f6792a1f0832933e600ba43`.
The original tested commit is not an ancestor of this publication; file identity
establishes inclusion of the repair, without signing the old run as a current-main run.
The independent consumer support/migration and execution map is in
[`examples/process-content-consumer/README.md`](../../examples/process-content-consumer/README.md).

## Native input uses the acknowledged View context (#48, 2026-11-01)

The native consumer dispatches input against the already acknowledged child frame.
It must not advance and broadcast `ViewContext.elapsed_nanos` immediately before
forwarding the same input: even a clock-only `set_view_context` correctly marks the
child repaint pending, so the original `input_ready` gate rejects that event.
Clock sampling continues through existing AnimationFrame/CaretBlink and environment
updates; no input replay, geometry bypass, forced focus or fixed selftest clock is
introduced. The native adapter simply avoids creating its own invalidation at the
input-dispatch boundary. Curve routing and all original native assertions remain.

Native063 on public `c138a7b4f456961132a35fa424bd11c9b5353e87`, program
`96f0c8195d0d4f7e013423070b6a315ec6b88f2a7072a07244ec394b676737fb`,
reached all three Ready children. Its first click at (166.5, 388) is inside the
published first Message frame [0, 372, 333, 32]. Immediately before Down and Up,
the journaled adapter advanced the context; both events returned unconsumed with
no focus. TextCommit/Preedit then also remained unconsumed, and the unchanged
UTF-16 expectation aborted with NSNotFound. This is a programmatic native adapter
failure, not a physical IME failure. Later auxiliary/Tab/resize/reactivation/final
phases were not reached. The original failure and resource-release evidence stay
under SSD `artifacts/issue-48/public-native-c138-20261101-r1/`.

The isolated adapter correction removes only that pre-dispatch context update.
Compilation and the same original complete native script must still validate it;
no passing behavior or Issue closure follows from this source analysis. Existing
Table ACK, Host normal, independent native wheel, and Panel six-stage proofs retain
their original identities. Physical Pinyin/candidate observations and applicable
VoiceOver/performance are neither substituted nor resumed by this repair.


### Scoped verification and remaining observations (2026-11-01)

CPU069 passed the affected consumer's offline all-target strict Clippy and one
matching build. Native070 then ran the unchanged complete programmatic native
script once against archived program
`dce85d5a73790637cfef39fbde24b8df42df0f3e4e250b44c7bc9cba145275e4`:
exit 0 in 22.34 seconds. The first click was consumed and focused Message 200;
all seven original phases reached normal close, including auxiliary append/scroll,
Tab, resize, reactivation, UTF-16 selection/marked text, and Unmark assertions.
The original Native063 failure remains preserved. The compiled inputs were public
c138 plus the exact two-path repair and eight preserved diagnostic WIP files;
this is the archived run's identity, not a claim of a clean public-tree execution.
The later publication supplement changes documentation only.

Final and before-close journal snapshots contained four Group AX nodes. The Host
contract does not make input readiness an AX publication fence, and those captures
did not record or await the separate semantic/image publication fences. This is a
sampling evidence gap, not a demonstrated product defect or proof of full AX
restoration. Physical Pinyin/candidate observation, VoiceOver, performance and the
other effective #48 acceptance criteria remain open; this finite adapter fix does
not close the Issue.

Primary immutable receipts are under SSD
`artifacts/issue-48/native063-input-clock-repair-20261101-r1/`:
`main-acceptance-r1/run01/main-release-acceptance.json` (CPU069),
`main-native-r1/NATIVE070-PASS-AND-RELEASE.json` (Native070), and
`domain-review-native070-r1/` (original-owner raw/contract review).
