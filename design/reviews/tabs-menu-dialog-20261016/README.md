# Tabs / Menu / Dialog offscreen probe

This is a target-only probe. It clones the Gallery workspace under `sources/`, restores
the prior Gallery workbench and controls export for the baseline, and injects a small
private-state reader into each clone. The reader only obtains frames from the actual
completed render; all activation is sent back through `Gallery::handle_input` as
mouse/key events. It never writes a control's open or selected fields directly.

The default probe renders five cases (`Tabs`, `Menu` with its second row active,
`Select`, `Dialog`, and the destructive-action Dialog) at 1440×960 logical points,
all four themes, 1×/2×, and the actual default 100% text scale. It writes 40 PNGs
and a state receipt. For an optional larger-type diagnostic, pass `--text-scale 2`
and a separate output directory; this produces 150% captures. This README documents
the probe setup, not a GPU or visual-test result. The root task runs captures separately
and tracks their outcome in its run receipt; this artifact's creation status should not
be treated as the latest capture status.

After the current implementation is ready, run from the repository root:

```sh
python3 target/tabs-menu-dialog-20261016/prepare_probe.py both
CARGO_TARGET_DIR=target/tabs-menu-dialog-20261016/probe-build/baseline CARGO_BUILD_JOBS=4 cargo build --manifest-path target/tabs-menu-dialog-20261016/sources/baseline/Cargo.toml -p tekes-ui-gallery --bin offscreen-overlay-probe --features baseline --locked
CARGO_TARGET_DIR=target/tabs-menu-dialog-20261016/probe-build/current CARGO_BUILD_JOBS=4 cargo build --manifest-path target/tabs-menu-dialog-20261016/sources/current/Cargo.toml -p tekes-ui-gallery --bin offscreen-overlay-probe --features current --locked
target/tabs-menu-dialog-20261016/probe-build/baseline/debug/offscreen-overlay-probe target/tabs-menu-dialog-20261016/captures/baseline
target/tabs-menu-dialog-20261016/probe-build/current/debug/offscreen-overlay-probe target/tabs-menu-dialog-20261016/captures/current
python3 target/tabs-menu-dialog-20261016/compare_probe.py target/tabs-menu-dialog-20261016/captures/baseline target/tabs-menu-dialog-20261016/captures/current target/tabs-menu-dialog-20261016/pixel-diff
```

For 150% diagnostics, run both binaries with `--text-scale 2` into separate
`captures/baseline-150` and `captures/current-150` directories, then compare those
directories separately. The default acceptance comparison remains 100%.

Use the same process serially for GPU runs. A state receipt must show tab index 2,
Menu open with selected index 1, Select open, and both Dialogs open before treating
the image set as valid. Native external-screen acceptance remains separate.
