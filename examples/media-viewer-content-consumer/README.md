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


## Independent A1 designated-display acceptance

`MEDIA_VIEWER_DISPLAY_POLICY=m8-actual-designated` additionally requires explicit
`MEDIA_VIEWER_M8_ACTUAL_A1=1`, display ID/class and the original normal nonmodal
case without IME/read-only/stale/remount probes. Default external-only and the
separate R1-only policy keep their existing contracts.

CPU141 built this independent candidate on public a81b7cd plus its exact scoped
source/documentation overlay and preserved WIP. Native142 used a fresh signed
bundle and grant: one actual bottom-border shrink admitted Actual at f=24/25,
then one activation produced Manual s=1, q=25/24 and a 200x100 logical image.
This finite A1 result preserves resize-capture timing differences and a 2pt
vertical crop on each side in the 96pt viewport; it does not claim settled
resize pixels, a fully visible uncropped image or a native run of a future
public merge. See design section 21 and the immutable Native142 receipts.

q>8, held gestures, backing migration, physical IME, VoiceOver, performance and
other remaining acceptance retain their original separate scope.


## Independent MAX designated-display acceptance

`MEDIA_VIEWER_DISPLAY_POLICY=m8-actual-max-designated` requires its own
`MEDIA_VIEWER_M8_ACTUAL_MAX=1`, explicit `MEDIA_VIEWER_DISPLAY_ID` and
`MEDIA_VIEWER_DISPLAY_BUILTIN`, normal nonmodal mode and no IME/read-only/stale/
remount probes. Default external-only, R1-only and A1-only policies keep their
contracts; the A1 flag, programme or grant cannot authorize this MAX case.
The original logical image remains 200x100, bitmap 16x8 and q range [1,8].

CPU144 built exact source `8c606323` on compiled public `a81b7cd` plus its scoped
overlay and preserved WIP. Native149 used that archive and a separate MAX signed
bundle, not a new native build of the later public PR223 merge. One actual
bottom-border shrink reached content 480x212 and positive viewport 480x8:
Fit f=2/25, candidate qActual=25/2 >8, original Actual disabled with the maximum
8 times Fit reason in raw semantics and AX. Current Manual q stayed 5/4,
scale was 1/10 and the image was 20x10 logical points. Candidate qActual is not
the current zoom. No Actual click or second resize occurred.

Main and the independent domain review accepted only this finite raw/semantic/
AX projection. The original resized screenshot retains the older Fit reason;
settled MAX copy/focus-ring pixels and disabled-action rejection are not proven.
The 8pt viewport crops 1pt at each vertical edge of the 10pt image, and this
single reachable size does not establish an OS minimum. Native146/148 startup
failures and Native147 zero-launch operator failure remain unchanged.
See design section 22 and the immutable CPU144/Native149 receipts for programme,
source, input and release identities. Whole M9, held gestures, backing migration,
physical IME, VoiceOver, performance and other remaining scope stay open.


## Independent D1 finite atomic pan acceptance

`MEDIA_VIEWER_DISPLAY_POLICY=m8-drag-designated` requires its own
`MEDIA_VIEWER_M8_DRAG_D1=1`, explicit display ID/class and normal nonmodal mode
without IME/read-only/stale/remount probes. Default/R1/A1/MAX contracts stay
separate. This source is `8af5915e`; it cannot run under an old MAX/A1 programme
or grant. Main built it once on public f4e7772 plus the preserved scoped overlay
and WIP; see design section 23 for the exact programme and new-case grant.

The sole CU executed one actual image pan after four original Zoom actions used
only to create overflow on both axes and four Tabs stopping at the first VIEW.
Original Down(240,462), Dragged through (420,176), and Up(420,176) yielded
T=(-165.9375,-69.9375) from (-345.9375,-34.96875): X moved +180pt and Y clamped
exactly to the lower bound. Manual q=625/256 stayed fixed. Independent raw
reviews accept only this finite geometry and native Up outside the viewport,
inside the same window's noninteractive status region. Pan notifications are
0/0/1/0; business Close/Retry remain zero. The window then closed normally.

Original pre/post images show translation, but the post image has no visible
ring despite retained VIEW focus; no settled ring-pixel acceptance is claimed.
Only content/viewport stability and absence of later Moved/Resized are observed;
there is no fresh before/after CG origin pair. No unheld movement follows Up,
so full D1 residual-capture and outside-OS-window release remain unverified.
Held-interleave D2, different-backing-scale S1 and whole M9 remain open.
Copied Rich IME description fields were corrected by an append-only Main errata;
this run provides no IME/physical/VoiceOver/performance acceptance. No original
raw, grant or earlier finite evidence was rewritten, and no runtime was repeated
for these final result documents.
