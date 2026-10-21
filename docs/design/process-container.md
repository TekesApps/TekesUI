# ProcessComponent and ProcessContainer implementation boundary

User-authorized work, 2026-10-21. Shared implementation owner: Rust TekesUI. AppHost owns its thin consumer integration; AgentKit/ClientKit retain Agent presentation and product state. This is an implementation and acceptance plan, not a claim that the public container is ready.

## Required result

Provide a reusable **ProcessComponent** as the process/state/service foundation. View is an explicitly negotiated optional capability. **ProcessContainer** is the standard UI presentation/input adapter only for components with View; it does not own another process manager. Consumers declare fixed native modules; the framework owns process/message lifecycle and, only for View, frame/GPU retirement.

AppHost assembles three View containers and one headless navigation component (five application processes including Host). Headless workers must not create a container, synthetic Panel/View ID, Renderer, GPU device, Scene, glyph atlas or shared texture; services and toolbar cannot depend on frame-ready or GPU acknowledgements. Module-owned toolbar and root-window contributions remain distinct.

The headless navigation module alone owns history/cursor/transactions and nav.json; AppHost executes declared routes and acknowledges actual successful context publication. It keeps no second writable history and never writes nav.json. Physical pane order/visibility and the root window remain host state; hide preserves the process and does not enter nav.json. Ordinary containers do not acquire a subprocess boolean, and arbitrary Rust child trees are not automatically moved between processes.

Use the existing ModuleDefinition/PanelModule, Request/Frame, worker renderer and IOSurface/Mach transport. Keep pure Rust, JSON Design System styling, immutable GPU surfaces and the one-surface parent composition route. Do not transmit Scene JSON, CPU pixels, Rust pointers or closures across processes; do not add WASM, Swift, GPUI or another UI framework.

Authoritative integration requirements: [AppHost migration boundary](process-container-apphost-migration.md) and the human-confirmed [ProcessComponent/navigation contract](process-component-navigation.md). Latest Issue14 was read into `design/reviews/toolbar-splitter-defaults-20261021/issue14-current.json`; it remains an audit/planning entry and is not closed by this implementation.

## Current implementation

1. The **sole existing** parent ModuleProcess and owned-surface import implementation now live in `tekes-ui-module-runtime`. AppHost's old module_process and gpu paths are compatibility re-exports. Existing worker arguments and Request/Frame shape remain unchanged. This is dependency extraction, not a completed ProcessContainer.
2. V1 View-only `run_worker_from_args` allows a public consumer to handle the same fixed-catalogue worker entry before starting its UI, without depending on AppHost or copying its factory selection. Normal parent startup never invokes factories; malformed, unknown and ambiguous worker identities fail explicitly.
3. The V1 request path remains worker-thread-only. ReceivedFrame now owns every received Mach right with RAII; rejected messages, unimported frames and failed imports release exactly once. ProcessControl is the shared child owner for service and View transports; cancellation can precede launch, terminate a blocked producer and interrupt Mach receives, while reaping remains off the caller thread. AppHost has consumed these APIs. Evidence: `design/reviews/process-view-rights-20261021/acceptance.json`. The standard nonblocking Component adapter, public frame queues/backpressure and GPU-completion retirement remain incomplete.
4. Headless V2 services are implemented and independently verified without View resources: window-scoped toolbar, correlated messages, bounded queues, generation/instance/epoch checks and managed close. AppHost owns a 16-case real-process navigation/storage prototype using this API. Its final receipt explicitly excludes five-process native assembly and actual three-View context publication; neither this prototype nor the lower View backend proves the final container is ready.

5. ProcessComponent now also owns a private queued V1 View adapter. Request coalescing, the 256-entry bound, two-frame budget and actual GPU-completion retirement are in module-runtime; the independent public-API consumer passed four 1x/2x Metal comparisons, cancellation/late-frame/idle-exit checks and existing regressions. V1 limitations are machine-readable and unsupported requirements fail. AppHost queued migration and the standard Content adapter remain outstanding. See `docs/design/process-view-queue-20261021.md` and `design/reviews/process-view-queue-20261021/acceptance.json`.

6. A legacy ProcessContainer now implements the original Content Component protocol, with mount-only spawn, shared queue/prepare, original clipping/propagation, same-process hide/show, stale-scale input gates and unmount/reinsert behavior. Renderer-owned registration guards fix unmount cleanup. The independent consumer and actual Metal captures prove this bounded legacy scope; V2 remote disposition/editing/AX and AppHost/native/Gold7 remain pending. See `docs/design/process-container-component-20261021.md` and `design/reviews/process-container-component-20261021/acceptance.json`.

## Remaining implementation, in order

| Contract | Implementation and proof required |
| --- | --- |
| Optional View and service protocol | Explicit capability/version negotiation; headless handshake, toolbar registration, commands/events and stop work without frame/GPU resources. V1 View compatibility is adapted, not relabeled as headless support. |
| Stable module/container identity | Fixed catalogue, process instance/mount generation, monotonic request and viewport identities; no stale A-B-A or old-worker callback can mutate a replacement. |
| Nonblocking lifecycle | Spawn/request/receive/reap/GPU waits off the UI thread; observable starting/ready/failed/closed state; terminate a hung producer and release blocked receives. One owner for the process and queues. |
| Hide versus close | Hide cancels input and clears live focus/capture but preserves the process and module state; show requires valid geometry before precise input. Close invalidates pending input/toolbar/frames and reaps the owned child; unrelated workers survive. |
| Input and toolbar | Move existing coalescing, two-frame backpressure, original command IDs and geometry-dependent gates into the shared container. Resize may show monotonic older preview, but stale preview is never valid input geometry. Module toolbar state stays in its process; the shared Toolbar handles layout/overflow. |
| Frame ownership | Own every received Mach right with RAII, including rejected, queued and never-imported frames. Import consumes the right once; retirement follows actual consumer GPU completion. Closed/hung consumers cannot strand capacity or block teardown. |
| Standard UI composition | A real public Component adapter uses the original measure/layout/paint/clip/event/focus/mount boundary and framework-managed invalidation. Returning a texture/Rect plus a consumer-written state machine is insufficient. |
| Native capabilities | Explicitly negotiate/report pointer, keyboard, scroll, text/IME and accessibility support. V1 only has Layout, PointerDown/Move/Up, Command and Cancel. It must not silently accept unsupported editing or claim remote editor/AX parity. AgentKit must retain existing editing/IME/AX behavior; unsupported consumers stay unconverted. |
| Navigation consumer | Its single-writer nav.json/pending journal and execute-then-persist confirmation follow the separate navigation contract. Framework provides correlated, instance/epoch-safe messages; no history/cursor/persistence logic moves into AppHost. Failed or unknown execution is not a committed history entry. |
| Failure and recovery | Bad identity/geometry/protocol, early worker exit, blocked output, import failure, resize/reorder, hide/show and close must be verified. An explicit fresh instance may recover; no old callbacks revive it. |

## Acceptance and delivery

Keep API tests, real consumer integration, actual Metal rendering, external-display desktop interactions, accessibility/IME and Gold7 conclusions separate. Use the current small-host/shared review-cache flow, retain required executables and evidence, and remove unused regenerable test artifacts only after process/file/lock checks. Performance remains out of scope.

At least two public-API consumers must exercise the same container implementation: the fixed Single/Three AppHost profiles and a minimal independent consumer without AppHost/Gallery imports. Validate separate PIDs, content/toolbar state preservation, original command identities, current geometry/capture, GPU completion, teardown of hung workers and exact-once right release. Bind the final reproducible source/locks/executables and receipts to one immutable development identity.

Only after those requirements are proven, notify the authorized target sessions with the same exact candidate and evidence:

- `01a15c56-b5a8-73d2-991b-5e5ce3fbc611`: organize AppHost around ProcessContainer.
- `01a14920-0d6e-7652-9a60-c74ef1a1f70f`: use the supported public container capabilities for navbar, sidepanel tabs and runview, preserving existing Agent semantics and input behavior.

The AppHost session owns the navigation module consumer and routing integration; generic ProcessComponent lifecycle/protocol is owned here. Coordinate any shared navigation utility explicitly rather than placing a product state machine in the Host.

Until then, status is **partial implementation**, and no “ready to migrate” notice is sent. No commit, push, tag, Issue closure or frozen-package overwrite is authorized by this task.

## Navigation design review dependencies

The latest read-only advisor review confirms that the two P2 persistence/recovery issues and the P3 toolbar-scope issue are resolved in navigation sections 5.1/5.2, AppHost sections 4/9.0 and the migration contract. No new blocking design contradiction was found; the minimum prototype may proceed. Implementation and fault-injection validation remain outstanding:

- A successful rename followed by failed directory sync is **replacement occurred, durability unknown**, not “the old JSON was retained.” Keep pending state and reconcile by revision/last_commit_id; do not publish HistoryCommitted or start another transaction prematurely.
- A new Host epoch needs a recovery barrier: the navigation module serializes any in-flight persistence first, supplies a revision-bound recovery snapshot, and confirms the baseline is still current after Host restoration before reopening navigation. Retain execution receipts needed to reconcile an uncommitted transaction across navigation-module restart.
- Registration validation must distinguish window-scoped/headless toolbar contributions from View-scoped contributions; Panel/View IDs cannot be mandatory for a headless component.

The minimum navigation prototype must also pass the three fault groups in [navigation acceptance §6.1](process-component-navigation.md#61-最小原型故障验收): lost RecoveryReady and idempotent recovery confirmation; both HistoryCommitted/pending/receipt retirement orders; and either process restarting between RecoveryApplied and RecoveryReady, with old-token rejection and a responsive Host UI thread.

These contracts and acceptance requirements are implementation prerequisites, not persistence acceptance evidence or a ready-to-migrate notice. The shared service protocol must carry the identities, correlation and recovery states needed by the consumer; it must not invent a Host-owned history or cursor to bypass them.
