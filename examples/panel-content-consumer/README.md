# Issue #82 independent Panel consumer — local main intake

The public Panel consumer source is pinned to Main `da532a84098bf552b1e6c2b37aa643e632c7f97b`; the original six package files were byte-matched against the public pin in `artifacts/issue-82/public-pin-da532a8-20261029-01/verified.json`. The native candidate adds a standalone example and a shared test fixture only. It uses the current repository crates through `../../crates`; no SDK is copied. This public-source receipt establishes source/lock identity only; it does not transfer the original 450/18 GPU results or claim native acceptance.

## Preserved acceptance inputs

- `panel_content`: all three original CPU scenarios and assertions, including actual CoreText editor/IME state, retained history, zero-inset refusal, nested Panel and real Split ownership. The original `Button` action owner and activation path are retained.
- `panel_gpu`: the original 450 combinations (five primitive pipelines × five width/clip modes × three backing scales × three text scales × two themes) and 18 full-frame single-coverage oracles remain unchanged. The supplementary per-glyph/sentinel test remains separately named and does not replace the original matrix.
- `support/coretext.rs`: the original CoreText shaper, rasterizer, atlas and TextSurface renderer are preserved. No stub text or alternate geometry is introduced.
- The lockfile is current main's independent composition-consumer graph with only its root package name changed. Registry versions/checksums are unchanged. Obsolete `svg-icons` forwarding is removed; these sources do not require it. `token-audit` forwarding remains supported.

## Validation entry points

The main coordinator runs these from the final example directory, under the existing shared SSD cache/source-identity and build-lock rules. Reuse that target/cache; do not create a target beside this manifest. CPU, compile-only GPU, strict checks and actual GPU execution are separate evidence.

```sh
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_content
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu --no-run
cargo clippy --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_content --test panel_gpu -- -D warnings
```

Actual GPU matrix execution requires macOS/Metal and a fresh `TEKESUI_PANEL82_GPU_ARTIFACT_DIR` beneath `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/`. The source enforces that SSD Issue #82 root before creating outputs. The coordinator must verify its selected fresh run subdirectory does not exist and must not reuse the old R19 output.

```sh
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu actual_coretext_all_gpu_pipelines_and_full_frame_single_coverage_oracle -- --exact --nocapture
cargo test --locked --manifest-path examples/panel-content-consumer/Cargo.toml --test panel_gpu actual_coretext_each_glyph_run_scissor_matches_parent_fallback_and_restores_state -- --exact --nocapture
```

The supplementary test uses public production rendering and pixels. Its render-event metadata cannot expose or certify private prepared scissor/fallback state. Historical R2 diagnostic SDK logs remain separate historical evidence; current event ordinals must not be joined to those logs. No diagnostic cfg or private SDK API is required by this current consumer.

## Native Panel window acceptance

The standalone native executable builds the exact original `panel_content` tree through `tests/support/panel_fixture.rs`, uses the existing CoreText/TextSurface service, and renders the original `ContentHost` to a real AppKit Window and Metal layer. It publishes the current `ContentAccessibilitySnapshot`, which retains the original curved Panel geometry; the legacy rectangular `content_nodes` projection is intentionally not used for this curve-owning tree. Native input callbacks return to the same original Host. The shared fixture explicitly enables vertical Auto on the existing ScrollArea when declaring multiline Fill; toggling the input multiline flag alone leaves the single-line constructor’s vertical Disabled policy unchanged. Horizontal Auto, the original text, owners and layout remain. The window supplies the original #45 application-owned outside inset using the existing `space-loose` metric; this keeps the same Panel rounded-corner probe inside the native content view rather than in NSWindow resize chrome. The original tree and curve assertions are retained.

The native event journal records real window focus, mouse and scroll callbacks, actual curve hit-test results, response/action receipts, owner mounts, editor UTF-16 selection/marked ranges, text and internal scroll offset, ScrollContent offset/limit/following/history result, reading source identity, Panel geometry generation, and current geometry state. A selftest script starts only after the first arranged frame and derives all event coordinates from the original Host's visible nodes. It sends a native wheel event, clicks the original Button and TextField, exercises AX selection, exact UTF-16 text queries/replacement, undo, long multiline caret reveal, Escape and Tab, and closes through `CloseRequested -> Window::close`. The script does not synthesize preedit or claim physical IME behavior.

Run the real window manually to verify the system Pinyin candidate interaction without changing input-method settings:

```sh
TEKESUI_PANEL82_NATIVE_EVIDENCE='/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/native-panel82-manual-20261029-01' \
  cargo run --locked --manifest-path examples/panel-content-consumer/Cargo.toml --example native
```

Click the original Composer, enter Chinese through the system Pinyin input method, choose a candidate using the real candidate window, and close the window normally. The journal records actual `TextPreedit`, `TextCommit`, and `TextUnmark` callbacks and resulting original TextField state. This manual path has not been run; synthetic text actions below do not represent a physical IME session.

For the separate AppKit event script, use this command with its separate evidence directory:

```sh
TEKESUI_PANEL82_NATIVE_SELFTEST=1 \
TEKESUI_PANEL82_NATIVE_EVIDENCE='/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/native-panel82-selftest-20261029-01' \
  cargo run --locked --manifest-path examples/panel-content-consumer/Cargo.toml --example native
```

The script uses the platform's existing AppKit event, AX and text-input hooks. Phase commands assert actual wheel movement, curved outside/inside pointer behavior and action ownership, native UTF-16 selection/replacement/undo, long-text caret reveal/undo, and Tab before close. The helper's frame clock is held at zero for deterministic non-animation rendering; `SystemMotionChanged` updates and journals the actual system reduced-motion value. This does not test animation or reduced-motion behavior. These hooks are not physical pointer, physical keyboard, Pinyin candidate UI, or VoiceOver evidence. The example returns an error if the run loop ends without the real close-request/close path.

## Current status and original limits

The coordinator actually passed the original CPU3, current Controls312 (including three directed shared-query contracts), affected reading/Split/pointer regressions and both library/public-target strict checks. The original production450/18 matrix passed on actual Metal at main7ab plus the adapted consumer, with its exact source/program receipt retained; the later readonly query and accepted-wheel observation change no GPU rendering inputs. The required library dependency is local commit `22cb05e0032d47e031ab30c51a68132ae1f3c1a5`. Current supplementary per-run execution remains **not_run** and no historical private prepared-state diagnostic is transferred. Actual logs and original failures are under SSD `artifacts/issue-136/panel82-main7ab8e32-intake/`.

Public pinned source/lock identity is verified separately as described above. The complete program-driven native exercise has now passed on the exact archived program `df04d85d9e591622a4bc654526601bf15a89864db616337c69f224509e15e84b`: actual wheel offset6, curved outside rejection and original Button activation once, three exact UTF-16 queries/replacement/Undo, the original80-line caret reveal with offset876, Undo restoration, Tab and real close. The root review verifies all six stages and state changes; exit0 alone was not accepted. Receipts: SSD `artifacts/issue-82/main131f9ef-quad-work-20261029-01/{main-rust01,main-rust02,native01/root-review.json}`. The actual program source identity is retained in that receipt; later README-only acceptance wording is not a new native run.

The original native08 rejection remains failed: enabling vertical Auto added a real scrollbar Quad and exceeded the previous full-target Quad estimate. The shared bounded Quad/KeyedQuad correction charges the existing prepared hardware scissor, preserving its actual draw state, shader and100m cap; original/new CPU work contracts and strict pass. The earlier Glyph correction and GPU450/18 retain their own exact inputs. Physical Pinyin candidate interaction and VoiceOver are still **not_run**, so #82/#45 remain open. Native API callbacks and automated text actions do not certify physical input or system IME behavior. No formal Release or deployment is claimed.
