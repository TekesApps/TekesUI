# Native module runtime (experimental)

A build-selected `ModuleDefinition { id, create }` creates a `Box<dyn PanelModule>` only inside its worker process. `PanelModule::update` handles typed input, populates a process-local Scene/atlas and returns toolbar/state metadata. The shared runtime renders with TekesUI/wgpu and exports an immutable IOSurface via Mach transport; Host imports it using the optional shared-surface renderer adapter.

There is no guest VM, dynamic library ABI or plugin discovery. Native factories are ordinary Rust source APIs compiled in the same build as the consuming app. Process isolation is not an OS permission sandbox. Production resource pools, timeout/restart policy, full input semantics and geometry/capture contracts remain unfinished.
