# AppHost Container consumer (#229)

The `apphost-containers` feature uses the public `Container::{Local, Process}`,
`LocalContainer` and `AppWindow::from_containers` APIs. It reuses the shared Host,
container-owned Toolbar, menus and existing process factories. It does not copy
window, input or rendering code.

Build through the existing shared SSD build entry point:

```sh
cargo build --manifest-path examples/app-host-demo/Cargo.toml --features apphost-containers --locked --offline
cargo test --manifest-path examples/app-host-demo/Cargo.toml --features apphost-containers --locked --offline
```

Launch the retained built executable under the acceptance runner's isolated HOME.
Keep worker argv unchanged; choose presentation with environment variables:

| Variable | Values | Default |
|---|---|---|
| `TEKESUI_APPHOST_LAYOUT` | `nav-others`, `others`, `nav`, `focus` | `nav-others` |
| `TEKESUI_APPHOST_BACKING` | `mixed`, `local`, `process` | `mixed` |

For an assembly-only public API check, pass `--verify-presentation`. This does not
start native windows or workers, and is not native behavior acceptance. The unit
test covers all twelve layout/backing combinations. When others is absent, only
focus and optional NavBar are mounted; no hidden unreachable extra container is
created.

The mixed default mounts editable `notes` as focus, optional editable `navbar`,
and `tools`, `plain`, `demo-first` in others. `plain` deliberately has no title or
Toolbar. `tools` and the real `demo-first` process support List, as does `notes`
when it returns to others after promotion. The `local` mode replaces the process
with editable `preview`. `process` uses the fixed process factories for every
mounted container. Process startup is handled before Settings and consumer HOME
migration, regardless of the presentation environment inherited by workers.

Use the shared **Containers** menu to Select/Open/Close/Promote a stable ID,
select List, move List members up/down, and show/hide others. The menu is absent
when others was not configured. Tab switching does not promote; promotion swaps
one container with focus. Typing in local Content and then moving it lets the
original text prove that the backing was preserved. Close/Hide only changes
presentation; use Open to restore it. List scrolling uses the original wheel
path; configured item heights keep content allocation while clipping offscreen
items.

`tools` command 101 is explicitly retained in `notes`; command 102 is explicitly
retained in `demo-first` (or `preview` in local mode). Select the latter while
`tools` is hidden to expose both destinations at once. `Owner only` (103) is not
retained. Returning to `tools` restores its own commands. If neither target is
visible, restoration remains available through the Containers menu. Retained
commands still log the original `tools` owner and its single incrementing count.

The optional headless document history contributes its controls explicitly to
`notes` (process mode: `demo-first`). Its explicit target set contains `notes`
and the initially hidden remote/preview, or the corresponding fixed process ID.
Normal Tabs/List/focus changes do not change these subscriptions or write history.
History is stored below the isolated HOME in `container-navigation`, separately
from the older demonstration. Navigation commands delivered to local targets log
their command and original text; process targets use the original module's
document command and imported frame. This consumer is a routing demonstration,
not a full business navigation application.

Observe stderr `DEMO_ASSEMBLY`, `DEMO_COMMAND owner=... command=... count=...
text=...`, and `DEMO_ACTION` alongside the shared runtime's
`CONTAINER_GEOMETRY`, `PRESENTATION_CHANGED`, `TOOLBAR_INVOCATION` and navigation
JSONL events. Compare command counts before and after each actual click; a tool
call or screenshot alone does not establish delivery. Normal window close or
Quit exits through the shared Host. Verify all owned worker groups exit and the
acceptance runner restores input/resources afterward.

The existing Settings entry remains available in this feature;
`independent-settings` still chooses independent mode. The older features and
acceptance evidence below retain their original scope and program identities.

---

# Fixed native App consumer

This separate Cargo workspace uses the shared AppWindow, Column and module runtime. On macOS the default is one ProcessContainer; `two-panels` selects two, `three-panels` selects three. `mixed-columns` includes `three-panels` and replaces the trailing process column with the original editable local Container. The leading two remain real process-backed modules. No Host/render/event-loop implementation is copied.

Three-column profiles retain document A/B/C navigation. The selected document maps to a demo module command; the shared Host waits for that command's content frame and GPU submission before confirming navigation. The module retains its counter Button, Increment/Reset toolbar and counter revision. Document switching changes the real read-only text body without changing that counter state.

The Settings schema, migration, configuration Undo and exclusive window mode remain in place; `independent-settings` selects the independent mode. Worker argv is handled by the fixed registry before consumer Settings construction or HOME migration. Native runs must use an isolated HOME during acceptance; do not migrate a user's configuration as a test.

From the repository root, use the existing shared build lock, SSD output routing and runner environment for these Cargo targets (do not create another target):

```sh
cargo check --manifest-path examples/app-host-demo/Cargo.toml --all-targets --locked --offline
cargo clippy --manifest-path examples/app-host-demo/Cargo.toml --all-targets --all-features --locked --offline -- -D warnings
cargo run --manifest-path examples/app-host-demo/Cargo.toml --locked --offline -- --verify
cargo run --manifest-path examples/app-host-demo/Cargo.toml --features two-panels --locked --offline -- --verify
cargo run --manifest-path examples/app-host-demo/Cargo.toml --features three-panels --locked --offline -- --verify
cargo run --manifest-path examples/app-host-demo/Cargo.toml --features mixed-columns --locked --offline -- --verify
```

`--verify` retains the counter, cancellation, resize, text-glyph and real cross-process GPU checks, then verifies document A/B/C produce distinct imported text bodies while preserving counter state. It probes the compiled module registry: three/mixed profiles probe three factories. The actual mixed AppWindow launches two processes and one local Container; the verifier is not a native local-input or mixed-window acceptance claim. Run each profile without `--verify` for native acceptance, including process counts, document navigation and local input where applicable.

For the bounded three/mixed native check, the existing `--window-acceptance` plus `TEKESUI_COLUMNS_WINDOW_ACCEPTANCE=1` continues after original Main23 READY: it opens the initially collapsed leading column through the shared Toolbar keyboard path, waits for all real process frames, and exercises mixed local TextField focus/input through the window's current AX publication. It emits a separate `COLUMNS_WINDOW_READY` marker. Without the exact opt-in, the original Main23 behavior remains. This is program-driven native verification, not physical keyboard/IME/VoiceOver certification.

Dependencies are explicit in-checkout source paths, not paths into another development repository. Issue #70 is complete: final independent checkout `45f1cf1ffd175403ae6e6f65065ebab8b2f06dc8`, PR139, same-source CI40507011917, public Main `5634d2ab15379b172930416358ab8442de9434b3`, and annotated `checkpoint-20261027-2`. The original `cd27d0b` native receipts retain their exact source identity: both Settings display modes passed Main → open → edit/autosave → Undo → close/focus restoration. They are not re-signed as a fresh run at a later pin.

For the current ordinary local-core checkpoint, use the complete public repository at `52ed62b1468a332b208cf61f4da06dc45ac72764` (`checkpoint-20261029-2`), retaining the relative `crates/` and `examples/` layout. See the root README for the bounded Settings/Panel/list/text-input stage. Physical IME and VoiceOver are not certified here; the new #81 B2 and #127 prepared Toolbar support remain separate and do not block ordinary local components. The fixed demo's existing process modes do not opt into B2. No new native acceptance was run for this documentation update.

Native acceptance may set `TEKESUI_APPHOST_DATA_DIR` to an absolute task-owned directory. Navigation and Settings then use that directory; the default Application Support location is unchanged, and HOME is not replaced. The existing `--verify` also checks a real cold hidden-worker Command/reply before its first Layout; this is IPC evidence, not visible-frame acceptance.
