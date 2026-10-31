# MediaViewer public consumer (#204)

This standalone workspace consumes the public `ContentHost`, `MediaViewerContent`,
`ImageContent`, original Buttons/Dialog/Panel and Renderer. It owns no duplicate
transform, load, pointer-capture or focus state machine, and depends on no Gallery
route, decoder, network source or private execution script.

The local package paths are explicit development overrides. This example is not
published; release consumers require published exact package pins.

## Targets and prerequisites

All contract tests and the native example are macOS fixtures. A zero-test result
on another platform is not acceptance. Use the repository's pinned Rust toolchain
and existing build environment; `--offline` needs the dependencies already cached.

| Target | Scope | Runtime requirements |
|---|---|---|
| `cpu_contract` | Two Loading/Missing public-contract tests; no fake Ready | CoreText; creates no Renderer/window |
| `public_viewer` | Nine ignored real ImageContent/Renderer/Host cases | Allocated real GPU window |
| `gpu_matrix` | One ignored test, 112 inputs: four themes × 1x/2x × regular/narrow150 × seven states | Allocated GPU window; new evidence directory; each image reviewed separately |
| `--example native` | Original platform input and AX publication, nonmodal or original Dialog | Exclusive desktop-input lease, actual unmirrored external screen, fresh evidence/binding, separately built `.app` for native UI control |

Representative scoped commands, to run only in an allocated build/GPU window:

```sh
cargo +1.95.0 test --locked --offline --manifest-path examples/media-viewer-content-consumer/Cargo.toml --test cpu_contract -- --test-threads=1
cargo +1.95.0 test --locked --offline --manifest-path examples/media-viewer-content-consumer/Cargo.toml --test public_viewer --no-run --message-format=json
cargo +1.95.0 clippy --locked --offline --manifest-path examples/media-viewer-content-consumer/Cargo.toml --test public_viewer -- -D warnings
```

A controlled GPU executor runs the actual newly compiled public test executable
with `--ignored --test-threads=1`; expected count is nine. Keep its compiled source,
Cargo configuration, compiler JSON, executable and dependency identities. Preserve
failure logs and stop the dependent batch after the first failure. Compilation or
an ignored test listing does not establish a runtime pass.

For `gpu_matrix`, `MEDIA_VIEWER_BINDING` names an immutable program/run receipt and
`MEDIA_VIEWER_OUTPUT` names a fresh evidence directory. The test retains PNG,
scene/transform text, independent pixel-oracle text and NOT_REVIEWED records.
Numeric checks do not replace individual image review. The adapter's four scoped
GPU tests live in `tekes-ui-controls`; this consumer does not duplicate them.

## Native boundary

The normal native example explicitly enables the platform's existing
`content-accessibility` feature. It uses public snapshot/publication/dispatch and
original key/pointer translation; it is not an AX-free executable. It does not use
`ax_modal_lifecycle`, AX experiment observers, private event injection or TEMP
tracing. The platform APIs it calls already exist on public baseline `04992fa`.
Platform notification implementation changes and their own acceptance remain a
separate integration decision, not files owned by this example.

Native is Light/nonmodal by default. `MEDIA_VIEWER_MODAL=1` opens the original
Dialog; its invoker has no reopen callback. Close and Escape tests therefore use
two explicitly planned launches, not a private reopen or an automatic retry.
The executable requires:

- `MEDIA_VIEWER_BINDING`: immutable actual compiled-program/run receipt JSON;
- `MEDIA_VIEWER_OUTPUT`: a fresh raw-evidence directory;
- `MEDIA_VIEWER_EXTERNAL_DISPLAY_ID`: actual granted CoreGraphics display ID.

The native fixture verifies its actual external screen, logs process/current_exe,
raw platform events, actions, focus, transform, geometry and semantics, and injects
no input. An external screen does not provide an independent keyboard/mouse lease.
The coordinator owns launch admission, `.app` derivation, screen/idle/occupancy
checks, cleanup deadline and the single input operator.

## Evidence scope

These source bytes come from historical frozen source `fdb0a534`. On that source,
the public no-run build, scoped strict and all nine real public tests passed;
the native example build/strict and separately granted modal Close/Tab-trap and
single Escape/return-focus runs passed within their named bounds. Earlier CPU,
112-image and focused-image receipts retain their own actual source identities.
This source package has not been compiled or run on the new public integration
baseline; historical evidence is not a claim about that combined tree.

Ordinary pointer drag/outside release, OS resize, enabled Actual logical size,
backing-scale transitions, interleaved active-drag cancellation, retained stale
native AX identity, focused/physical IME, VoiceOver, performance and pinch retain
separate unverified boundaries. No source package or image count closes them.
