# ProcessComponent / ProcessContainer / AppHost migration boundary

## Implementation update — 2026-11-07 (#229, v0.2.0)

User authorization now covers implementation, affected verification and formal semantic-version Release. The dated design-only and prior acceptance statements below remain historical. New assembly is `AppWindow::from_containers(Vec<Container>, PresentationOptions)` with `Container::{Local, Process}` and `LocalContainer`; old local-only source callers can temporarily import `legacy::Container`. `Column/from_columns` retains its horizontal layout semantics through the same Host and container-owned Toolbar instances, not a second runtime. The module-runtime Content adapter is unchanged.

NavBar/focus/others use stable configured IDs. `navigation_toolbar` names a contribution display owner independently from `navigation_targets`. With no others configuration, unassigned extra containers are rejected because there is no Open/recovery area; configured off/empty others retains recovery. List height and eligibility are explicit, with outer scrolling and separate up/down order. Hidden retained commands use explicit target mappings and preserve original source/instance generation.

The current source/unit batch and the independent demo/native/public delivery remain separate evidence. See [#229](https://github.com/TekesApps/TekesUI/issues/229); no old native receipt is re-signed as this new architecture.


## Confirmed target — 2026-11-05 (#229; design only)

The [AppHost container/toolbar decision](app-host-architecture.md#container-owned-toolbar) supersedes the window-owned ToolbarHost and column naming in the earlier migration plan. This revision changes documentation only; current APIs and the dated acceptance records below remain unchanged. #70 is now closed for its original delivery; its historical partial-intake statement below is not a new blocker.

- Window directly assembles `Container`, with `LocalContainer` and `ProcessContainer` as the two backing modes. The target entry point is `from_containers`; there is no extra public Column/window Pane layer. Existing app-host and module-runtime ProcessContainer types are distinct and need an explicit reuse/adapter migration, not a text replacement.
- Each Container owns its optional Toolbar, optional title and Content. Reuse the shared Toolbar/Content interaction implementation. Remove the target architecture's window-level ToolbarHost; do not retain centralized item/focus/menu state under another name.
- Window allocates container rectangles, splitters, visibility and native safety regions, and coordinates cross-container input/focus routing without duplicating local interaction state. Each Container measures its Toolbar and computes its own content viewport; no mandatory common row height or empty toolbar row. Native window title metadata, system controls, drag/resize/fullscreen remain platform responsibilities even without a separate visible title bar.
- A button may opt into remaining visible when its declaring container is hidden. Window selects a visible target with a Toolbar according to assembly configuration; the concrete mapping API remains to be specified before implementation, with no automatic first-visible-container policy frozen here. Original identity, declaration lifetime and business command/state ownership remain; only one presentation is active. No arbitrary editor or whole-toolbar migration. If no target exists or the recovery button is inaccessible, use the window menu/shortcut recovery path; never invent an empty container.
- Headless navigation `window` scope is contribution authority/lifetime, not ToolbarHost ownership. Its contributions are assembled into a real Container's Toolbar while history/nav.json and command sources remain with the original service. No fake View, renderer or frame-ready dependency is introduced.

The process/transport, geometry, cancellation, navigation persistence and source-validation contracts below remain. New ownership and titlebar behavior require later scoped implementation and verification tracked in [#229](https://github.com/TekesApps/TekesUI/issues/229); historical checks do not certify them.

### AppHost presentation pattern — 2026-11-05 follow-up

The [confirmed focus/others pattern](app-host-architecture.md#apphost-focus-others) replaces the old fixed three-column shell as the target AppHost assembly: optional left NavBar Container; right-side focus above an optional, toggleable others area. Others has ordinary Container tabs plus one List tab vertically composing its list-capable containers. These are presentation roles, not new Pane/Container subclasses. Local/process modes retain the same structure.

Promoting one Container swaps it with the current focus; the old focus returns to others. The List tab cannot become focus as a whole. List ordering interaction remains undecided. Runtime off retains others state; no others in startup configuration means no area/toggle. NavBar response bindings belong to the app, remain tied to container identity across movement, and do not imply all containers or only focus react. Presentation focus is distinct from keyboard focus; the optional visible NavBar is distinct from the existing headless navigation-history service.

The lower sections' physical columns/splitter baseline and process-count checks describe the original migration scenario, not completion or permanent limits of this new pattern. No source, worker/API, native UI or IPC changes are authorized by this documentation update.



## Current public boundary — 2026-11-03

The AppWindow/Settings delivery tracked by Issue70 is complete. PR139 published
its accepted scope at `5634d2ab15379b172930416358ab8442de9434b3`, with final
independent checkout `45f1cf1ffd175403ae6e6f65065ebab8b2f06dc8` and same-source
CI40507011917. Original column/mixed-input and both Settings display-mode
save/Undo/close/focus receipts keep their original identities. See
[Issue70](https://github.com/TekesApps/TekesUI/issues/70) and the
[public demo guide](../../examples/app-host-demo/README.md). Issue136's finite
frozen intake is also complete; later component obligations remain with their
own Issues. The local intake below is historical, not a current unpublished
AppHost dependency or a reason to repeat accepted native checks.

The public column descriptor `tekes_ui_app_host::ProcessContainer::new` holds a
compiled ModuleDefinition; `Column::Container` selects the original local
ContentHost path. Neither API alone establishes complete remote editor support.
The negotiated Content adapter is
`tekes_ui_module_runtime::ProcessContainer::new_view`, with the explicit child
capabilities, text/input bridge and lifecycle fences described in the
[Process View contract](process-component-view-v2-20261021.md). Host assembly,
local Content consumption and negotiated remote Content acceptance must be
reported separately. No new process manager or Agent-specific presentation is
introduced by this boundary clarification.

The Runtime input repair and its acceptance/support documentation were published
through PR213 and PR214. Native070 remains the exact archived c138-plus-repair
and diagnostic-WIP cohort; publishing it does not certify a clean native run of
this source pin or newer shared libraries. Issue48's remaining original AC3
requires real Pinyin during auxiliary/reveal/background interaction, preserved
focus/marked text/no-submit behavior, and actual candidate-window movement/resize
observations. These require a human operator and a separately allocated window;
programmatic text queries/preedit cannot substitute. Lifecycle, independent native
wheel, consumer compatibility and existing CPU/GPU proofs are separate accepted
evidence, not work blocked on physical IME or Media acceptance. Late AX final
samples do not establish a product defect or add a new closing gate.

Issue45/82 retain their own applicable physical/VO/performance boundaries. No
VoiceOver/performance defer is resumed, no AgentKit adoption or Kernel change is
implied, and no extra native check is added to the completed Issue70 scope.
Any new consumer adoption must bind its own source and original acceptance;
actual foreground work remains allocated by the single integration executor.

## Historical local-column intake — 2026-10-27 (#70/#136)


Owner: root; validation baseline `b7634699b5b7026a28fa26cc2416b55f2235ff8d` plus the frozen 13 AppHost paths. Receives real local Container/Column/AppWindow composition, source-bound runtime declarations, one shared ToolbarHost, existing CoreText services and original platform multi-Host accessibility publication. Logical toolbar return intent is authenticated by the original semantic mount and declaration Source; temporary barriers revoke arms, while replacement sources cannot inherit focus. Oversized native rows preserve nonnegative full-content allocation; exact native center is not promised for oversized content.

Actual acceptance: AppHost all-target compile, all 32 library behavior tests and all-target strict pass. The original Window focus/capture/viewport tests remain, along with actual local columns, shared toolbar and native publication model tests. Strict first rejected only items after a test module; moving that exact module after the production impl fixed the lint, and only strict was rerun. The original behavior run retains its pre-reorder source identity. Evidence: SSD `artifacts/issue-136/apphost-columns-mainb763469-intake/round01` and `round02`.

At this historical intake, this was a partial #70 delivery: Settings/save/UndoRedo business, PR #52 independent fixed public dependencies and CI, real native window/physical IME/VoiceOver and remote process acceptance are not certified. No new process engine or shared Theme rules; no unchanged Controls/GPU rerun. Historical migration obligations below remain; this intake does not close #70 or #136.


The shared ProcessComponent foundation and optional-View ProcessContainer adapter are owned by the Rust TekesUI task. Rust AppHost owns its consumer migration; do not fork a second process manager. The following migration requirements retain their original checkpoint scope; current public delivery and remaining acceptance are distinguished above.

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

The module still declares its content and optional toolbar. Each Container owns an instance of the shared Toolbar component, which owns single-row measurement, icon/text layout, overflow and interaction. Window supplies container allocations and actual native safety regions; Container allocates its own toolbar/content bounds, without a mandatory common toolbar height. Preserve ContentHost event propagation, primary-button/window-active gates, and original native-down handoff cancellation rules.

## Window-scoped service contributions presented in Container toolbars

Pure-icon back/forward is declared by the headless navigation ProcessComponent, with availability projected from its authoritative committed history and pending transaction. These contributions bind owner instance + window scope + registration generation, remain visible independently of hidden/reordered/closed views, and require no Panel/View or frame-ready. View-scoped contributions additionally bind their real Panel/View; commands cannot cross owner/scope even with identical IDs. Toolbar owns icon layout/overflow and Host dispatches original owner/command IDs.

Leading/trailing visibility is still Host-owned layout state, not navigation history. Its controls bind physical slots, retain a recovery entry and do not follow module reorder. A hidden pane normally retains its process/state, cannot receive new input, and transfers focus to a visible target; hide differs from shutdown. Neither nav.json nor the navigation service takes ownership of this layout state.

Use the full normative contract in [ProcessComponent, headless navigation and nav.json](process-component-navigation.md). One navigation transaction is pending per history: durable pending intent → Host execute and report a genuine committed transition → navigation module atomically save nav.json → HistoryCommitted. Failures/cancellation do not move the committed cursor; duplicate/late replies and write/restart failures reconcile by transaction ID, revision and instance/epoch. Host does not keep a second history or write nav.json. Replacement-durability-unknown is distinct from pre-replacement failure: retain pending, verify revision/last_commit_id, retry persistence confirmation and never claim the old file was retained after rename. New Host recovery serializes through the navigation coordinator: settle writes, offer a revision-bound snapshot, verify RecoveryApplied against the current baseline, then issue RecoveryReady before reopening history actions. Host retains unresolved execution receipts across navigation-instance restart until commit/reconciliation; this is temporary execution state, not a second history.

The existing View toolbar path remains text-only. A separate V2 ProcessComponent navigation prototype now implements headless service messages, window-scoped icon registration and journaled nav.json persistence, with the three fault groups exercised by real subprocesses; see [implementation and evidence](navigation-prototype-20261021.md). A scoped three-View titlebar integration now passes native back/forward clicks for the panel-layout demo route; see [window evidence](navigation-window-20261021.md). It is not complete public ProcessContainer, arbitrary business-route joint publication or full GUI migration acceptance.

## Integration acceptance

1. Same fixed Single and Three profiles still compile; the View baseline remains 3 containers + 3 workers + 1 Host. The navigation-enabled assembly adds one headless worker, for 5 processes and still 3 content containers.
2. Content and toolbar state remain in the same module processes during resize and reorder; original command IDs reach only their module.
3. In the existing three-container layout, left splitter changes left/middle only; right splitter changes middle/right only. Each Container's toolbar height/allocation updates its content viewport and pointer coordinates together. The former mandatory unified top height is superseded by the 2026-11-05 target.
4. Real Text/TextFieldContent path and default Style remain; no WASM, Scene JSON, CPU pixel transport or duplicate business view tree.
5. Preserve two-frame bound, coalescing, monotonic intermediate resize presentation and precise-input gates; stale/future frames do not become valid input geometry.
6. Native toolbar ordinary click/drag, splitter consumption, focus-loss/late-up and narrow-menu behavior remain separate desktop acceptance items; component/GPU checks are not physical-input evidence.
7. Keyboard/IME/AX and the full geometry/capture/joint-switch contracts must be explicit capabilities. Existing V1 protocol supports only Layout, pointer down/move/up, Command and Cancel; do not imply complete remote editor or accessibility support.
8. A public-API consumer independent of AppHost must build and exercise the same ProcessContainer implementation. Reuse review-cache with the coordinated lock/environment; no cache cleanup, performance round, commit, push or Issue closure.

9. Headless navigation handshakes, registers toolbar and handles commands without renderer/GPU/atlas/shared-texture creation or frame/ack waits. Existing view factories remain compatible through the View adapter.
10. nav.json/history is written by the navigation component alone. Validate success-before-history ordering, atomic storage, branch truncation, idempotence, unknown results and restart reconciliation as specified in process-component-navigation.md. The new contract supersedes earlier AppHost-owned back/forward guidance.

11. Include fault injection for successful rename followed by failed directory sync, and Host restart with the write paused before/after rename. Recovery cannot become ready on an old revision; stale recovery tokens and navigation-instance replacement must not discard unresolved receipts. Toolbar registration validation uses owner/scope/generation; window-level headless contributions require no fake Panel/View and survive unrelated View lifecycle changes.

12. The minimum navigation prototype must pass all three fault groups in [navigation acceptance §6.1](process-component-navigation.md#61-最小原型故障验收): lost RecoveryReady with idempotent current-session query/retry or a fresh handshake; both HistoryCommitted/pending/receipt retirement orders with snapshot-bound reconciliation and no repeated navigation; and either process crashing between RecoveryApplied and RecoveryReady, rejecting old tokens while keeping the Host UI thread responsive. Record identities, revisions, gates and reconciliation outcomes. Design approval permits implementation; it is not runtime/persistence acceptance.
