# Fixed native App consumer

This separate Cargo workspace consumes AppWindow, the neutral native module factories and declarative Settings. Default builds one panel, `two-panels` builds leading/center, and `three-panels` builds three fixed module IDs with document navigation. Settings shares the window shell and uses local TekesUI sidebar/form content; its navigation and configuration Undo/Redo remain no-view processes. `independent-settings` selects a separate Settings window instead of the default exclusive presentation.

Shared controls, text, render and module-runtime use the exact `controls-number-field-20261022` tag. AppHost, Settings and the native multiwindow platform adapter are workspace code in this repository; they are not silently substituted for the pinned controls implementation.

`src/main.rs` only selects the build profile and calls the shared entry point. No Host/render/event-loop code is copied. The module implementation is in `examples/panel-module-demo`, with state instantiated separately in each worker process.

All macOS profiles now assemble `Vec<Column>` and call `AppWindow::from_columns`. Default is one ProcessContainer; `two-panels` is two ProcessContainers; `three-panels` is three ProcessContainers. `mixed-columns` keeps leading/center as ProcessContainers and uses a local editable Container in trailing. Settings uses the same API with two local Containers. The legacy AppProfile constructor remains a compatibility API but is no longer used by sample assembly.

These consumers currently require the unpublished unified Button API described in `docs/unified-button.md`; this draft cannot yet build independently against its old base controls. Public UI implementation/pinning is a separate dependency.
