# Fixed native App consumer

This separate Cargo workspace depends on the shared Host and the neutral native module consumer. `src/main.rs` only selects the build profile and calls the shared entry point. No Host/render/event-loop code is copied. Default builds one panel; `--features three-panels` builds three fixed module IDs.

From the TekesUI repository root use `scripts/app-host-prototype.sh verify-three` or `window-three`. The module implementation is in `examples/panel-module-demo`, with state instantiated separately in each worker process. Dependencies are explicit local development source overrides, not release pins.
