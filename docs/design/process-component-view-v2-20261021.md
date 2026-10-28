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

Rectangular parent Scroll admission uses the default-false `Component::rectangular_input_geometry` opt-in. ProcessContainer declares it only for ready, negotiated Content input; the original current focus/reveal checks still run. This does not grant prepared-action purity or curved Panel support, and Curve V1 remains reject-only. The public consumer retains its real outer Scroll and Grid, with a valid arranged translation contract for its fixed row.
