# ProcessComponent / ProcessContainer / AppHost migration boundary

The shared ProcessComponent foundation and optional-View ProcessContainer adapter are owned by the Rust TekesUI task. Rust AppHost owns its consumer migration; do not fork a second process manager. This file records integration requirements against the existing live WIP, not an assertion that the public API is already implemented.

## Move into the public process foundation / View adapter

Reuse the existing parent ModuleProcess, child PanelModule/runtime, app-protocol and Metal/IOSurface/Mach-right transport. Extend versioned capability/message contracts for optional View and service messages, without introducing a parallel IPC/process manager. Move the following existing parent responsibilities out of AppHost window.rs:

- Fixed module identity / worker launch and process lifecycle; no arbitrary executable or plugin discovery.
- Bounded/coalesced input queue, request seq and content viewport generation; producer thread and wake notification.
- Frame identity validation, monotonic resize preview and geometry-dependent command/new-down gates.
- Shared GPU import and image lifetime; at most two immutable outstanding frames, producer readiness and consumer GPU completion acknowledgements. Dropping a rejected/pending frame must release its Mach right exactly once.
- Module-owned toolbar registration and original command IDs, including window-level contributions from headless modules. Transport owns no navigation history or window visibility state.
- Observable process/error/frame state sufficient for a host to show failure and acceptance diagnostics. UI thread must not block on request, process wait or GPU completion.

Shutdown needs an explicit ownership contract: the current connection can block in worker output reads, and Pending/Frame currently carries a raw surface right. The public API must state what Drop/close guarantees for blocked workers, pending acknowledgements and unimported rights. Do not infer safe deterministic teardown merely from wrapping the legacy thread.

## Keep in AppHost

AppHost owns the window, two independent splitters / physical column allocations, module order, navigation execution/results, focus and native titlebar dragging. The navigation ProcessComponent alone owns history/cursor/nav.json. AppHost composes three ProcessContainer instances plus the independent headless navigation component; it must no longer spawn ModuleProcess, manipulate transport rights, copy queue/backpressure state or validate worker frame internals itself.

The module still declares its content and optional toolbar. The shared Toolbar component owns its single-row measurement, icon/text layout, overflow and button/menu interaction. AppHost supplies parent constraints: unify registered window/Panel toolbar measurements with native chrome safety, reserve real traffic-light space, allocate content bounds and forward local input. Preserve ContentHost event propagation, primary-button/window-active gates, and the original native-down handoff cancellation rules.

## Window toolbar contributions requested during migration

Pure-icon back/forward is declared by the headless navigation ProcessComponent, with availability projected from its authoritative committed history and pending transaction. These contributions bind owner instance + window scope + registration generation, remain visible independently of hidden/reordered/closed views, and require no Panel/View or frame-ready. View-scoped contributions additionally bind their real Panel/View; commands cannot cross owner/scope even with identical IDs. Toolbar owns icon layout/overflow and Host dispatches original owner/command IDs.

Leading/trailing visibility is still Host-owned layout state, not navigation history. Its controls bind physical slots, retain a recovery entry and do not follow module reorder. A hidden pane normally retains its process/state, cannot receive new input, and transfers focus to a visible target; hide differs from shutdown. Neither nav.json nor the navigation service takes ownership of this layout state.

Use the full normative contract in [ProcessComponent, headless navigation and nav.json](process-component-navigation.md). One navigation transaction is pending per history: durable pending intent → Host execute and report a genuine committed transition → navigation module atomically save nav.json → HistoryCommitted. Failures/cancellation do not move the committed cursor; duplicate/late replies and write/restart failures reconcile by transaction ID, revision and instance/epoch. Host does not keep a second history or write nav.json. Replacement-durability-unknown is distinct from pre-replacement failure: retain pending, verify revision/last_commit_id, retry persistence confirmation and never claim the old file was retained after rename. New Host recovery serializes through the navigation coordinator: settle writes, offer a revision-bound snapshot, verify RecoveryApplied against the current baseline, then issue RecoveryReady before reopening history actions. Host retains unresolved execution receipts across navigation-instance restart until commit/reconciliation; this is temporary execution state, not a second history.

The existing View toolbar path remains text-only. A separate V2 ProcessComponent navigation prototype now implements headless service messages, window-scoped icon registration and journaled nav.json persistence, with the three fault groups exercised by real subprocesses; see [implementation and evidence](navigation-prototype-20261021.md). A scoped three-View titlebar integration now passes native back/forward clicks for the panel-layout demo route; see [window evidence](navigation-window-20261021.md). It is not complete public ProcessContainer, arbitrary business-route joint publication or full GUI migration acceptance.

## Integration acceptance

1. Same fixed Single and Three profiles still compile; the View baseline remains 3 containers + 3 workers + 1 Host. The navigation-enabled assembly adds one headless worker, for 5 processes and still 3 content containers.
2. Content and toolbar state remain in the same module processes during resize and reorder; original command IDs reach only their module.
3. Left splitter changes left/middle only; right splitter changes middle/right only. Unified top height updates content viewport and pointer coordinates together.
4. Real Text/TextFieldContent path and default Style remain; no WASM, Scene JSON, CPU pixel transport or duplicate business view tree.
5. Preserve two-frame bound, coalescing, monotonic intermediate resize presentation and precise-input gates; stale/future frames do not become valid input geometry.
6. Native toolbar ordinary click/drag, splitter consumption, focus-loss/late-up and narrow-menu behavior remain separate desktop acceptance items; component/GPU checks are not physical-input evidence.
7. Keyboard/IME/AX and the full geometry/capture/joint-switch contracts must be explicit capabilities. Existing V1 protocol supports only Layout, pointer down/move/up, Command and Cancel; do not imply complete remote editor or accessibility support.
8. A public-API consumer independent of AppHost must build and exercise the same ProcessContainer implementation. Reuse review-cache with the coordinated lock/environment; no cache cleanup, performance round, commit, push or Issue closure.

9. Headless navigation handshakes, registers toolbar and handles commands without renderer/GPU/atlas/shared-texture creation or frame/ack waits. Existing view factories remain compatible through the View adapter.
10. nav.json/history is written by the navigation component alone. Validate success-before-history ordering, atomic storage, branch truncation, idempotence, unknown results and restart reconciliation as specified in process-component-navigation.md. The new contract supersedes earlier AppHost-owned back/forward guidance.

11. Include fault injection for successful rename followed by failed directory sync, and Host restart with the write paused before/after rename. Recovery cannot become ready on an old revision; stale recovery tokens and navigation-instance replacement must not discard unresolved receipts. Toolbar registration validation uses owner/scope/generation; window-level headless contributions require no fake Panel/View and survive unrelated View lifecycle changes.

12. The minimum navigation prototype must pass all three fault groups in [navigation acceptance §6.1](process-component-navigation.md#61-最小原型故障验收): lost RecoveryReady with idempotent current-session query/retry or a fresh handshake; both HistoryCommitted/pending/receipt retirement orders with snapshot-bound reconciliation and no repeated navigation; and either process crashing between RecoveryApplied and RecoveryReady, rejecting old tokens while keeping the Host UI thread responsive. Record identities, revisions, gates and reconciliation outcomes. Design approval permits implementation; it is not runtime/persistence acceptance.
