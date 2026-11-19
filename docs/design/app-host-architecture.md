# AppHost Architecture

TekesUI's `AppHost` layer provides a shared pure-Rust application shell for multi-container desktop windows combining local `ContentHost` UI trees and isolated child-process modules (`ProcessContainer`).

## 1. Crate Layering & Boundaries

```text
crates/
  tekes-ui-app-protocol/     Pure serializable IPC messages, generations, toolbar descriptors
  tekes-ui-module-runtime/   In-process navigation/undo, child lifecycle, Mach + IOSurface transport
  tekes-ui-app-host/         AppWindow assembly, NavBar/focus/others layout, Toolbar & Settings UI
```

Dependency flow strictly points upward into the UI stack:

- **`tekes-ui-app-protocol`** has no UI or GPU dependencies.
- **`tekes-ui-module-runtime`** depends on `tekes-ui-app-protocol` and lower rendering/platform primitives (`wgpu`, `mach2`, `objc2-io-surface`, `tekes-ui-render` on macOS). Core libraries (`tokens`, `scene`, `text`, `controls`, `render`) never depend back on runtime or host.
- **`tekes-ui-app-host`** depends on `tekes-ui-app-protocol`, `tekes-ui-module-runtime`, `tekes-ui-settings`, and core TekesUI crates on macOS. Applications register fixed `ModuleDefinition` factories at build time via `AppProfile` rather than loading dynamic plugins at runtime.

## 2. Window & Container Model

`AppWindow::from_containers(...)` mounts containers directly inside a single GPU-rendered window without duplicating window chrome or toolkit state machines:

```text
AppWindow
├── NavBar (optional left sidebar container; navigation list & history controls)
├── focus  (primary single container with optional per-container Toolbar + Content)
└── others (optional right Tabs / aggregated vertical List with preserved container identity)
```

Each container uses either:

- **`LocalContainer`**: In-process TekesUI `ContentHost` tree.
- **`ProcessContainer`**: Fault-isolated child process running `ModuleProcess` from the same binary via fixed compile-time module factory selection (`run_worker_from_args`).

Key properties of the container model:

- **Container-owned Toolbars**: Each container owns its optional `Toolbar` and contributes command/navigation items by stable ID and scope generation. `AppHost` respects native macOS window traffic-light safe areas only where toolbars intersect the top leading edge.
- **Stable Identity on Reorder/Swap**: Moving any single container between `others` and `focus` swaps the container identity directly without cloning module state or restarting worker processes. The aggregated `List` tab displays participating non-focused `others` containers vertically with reorder support while sharing the exact same container state.
- **Settings & Display Preferences**: `SettingsOwner` loads and validates TOML-backed schema settings (`tekes-ui-settings`) on background threads, manages undo/redo revisions in process (`settings_undo`), and propagates committed text-size and appearance (`System` / `Light` / `Dark`) changes to live hosts via weak wake subscriptions.
- **Navigation History**: Navigation history and cursor state (`navigation`) run in-process without blocking the UI thread or spawning helper OS processes for undo/history storage.

## 3. Process Transport & Compositing

When a `ProcessContainer` enables `View` capabilities:

1. **Nonblocking IPC**: Requests, pointer/keyboard/IME events, toolbar actions, and resize notifications carry monotonic instance, request, and viewport generations so late frames or stale inputs cannot overwrite newer state.
2. **Zero-copy Frame Sharing (macOS)**: Worker processes render with their own local `Scene`, text shaper, and `wgpu` device into an `IOSurface`-backed target (`tekes-ui-render` with `macos-shared-surface`), exporting Mach rights (`tekes-ui-module-runtime` via `mach2`) through RAII `ReceivedFrame` handles that guarantee single-ownership cleanup and GPU-completion retirement before texture reuse.
3. **Fault Isolation**: Process isolation boundaries contain worker panics or crashes to the affected container while preserving other containers and keeping host layout/recovery actions responsive. It provides crash isolation rather than a OS security sandbox.

## 4. Accessibility & Focus Routing

- Focus traversal, pointer capture, and IME preedit arbitration follow explicit scope boundaries across visible toolbars, optional navigation lists, and active container content.
- `tekes-ui-macos-bridge::content_accessibility` projects mounted container semantics (`ContentSemantics`), text layouts (`content_text_layouts`), and accessible actions (`content_action`) to AppKit `NSAccessibility` elements when geometry is current (`!requires_curve_geometry()`).

## 5. Verification Entrypoints

- Protocol & runtime tests: `cargo test -p tekes-ui-app-protocol -p tekes-ui-module-runtime -p tekes-ui-app-host`
- Multi-panel consumer demo: `cargo test -p tekes-ui-app-host-demo --all-targets` and `cargo build -p tekes-ui-app-host-demo --features apphost-containers`
- Headless process container consumer: `cargo run -p tekes-ui-process-content-consumer --locked --offline`
- Native module worker consumer: `cargo test -p tekes-ui-panel-module-demo --locked --offline`
