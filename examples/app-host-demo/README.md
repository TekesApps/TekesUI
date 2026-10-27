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

Dependencies are explicit local development source paths. PR52/#70 AC3 still requires an independent final checkout, immutable dependency delivery and same-source CI; these samples and local evidence do not establish that remote delivery.
