# Issue 27 — layout-activated text service preflight

Status: implemented in current WIP; scoped public API, real consumer, GPU and native evidence below. Receiver review and exact public input/CI remain separate. Owner: root for shared implementation; AgentKit owns its Agent presentation and acceptance. No commit, push, tag or Issue closure in this round.

## Root cause and shared fix

`ContentHost::render` checked active TextSurfaceHost dependencies only at entry. An application-owned Component could activate its original field during measure/arrange and reach painting/native publication without a host. The independent consumer reproduces the pre-fix inconsistency (three failures, one passing always-active case).

Reuse the existing dependency scan, `MissingTextServices`, cancellation and retained-dialog cleanup in `ContentHost::check_text_services`. Run it at entry, after measure, and after arrange, before modal focus selection or paint. No new public wrapper, editor, state machine or component abstraction. The JSON interaction contract records the same timing and recovery boundary; tokens, sets and API token values are unchanged.

Applies to every original text-surface descendant activated by layout, including editable fields and read-only reading-backed text, not just the named fixture. Plain TextContent labels do not acquire a TextSurfaceHost requirement. Entry-active subtrees still fail fast: collapse them with a valid backend first, then remove the backend while inactive. Layout changes are retained after rejection; transient input is cancelled. Retry with a valid backend, and publish native text/AX only after render succeeds.

## Public API and actual consumer

`examples/composition-consumer/src/layout_activation.rs` contains an application-owned Pane with the original TextFieldContent; it changes active during measure or arrange. Five public consumer tests cover always-active parity, both activation stages, zero-width/height hidden cases, original editor baseline/value/reading version, no stolen focus, recovery, and unaffected label-only renderers.

Local logs under `design/reviews/layout-text-preflight-20261021`:

- 426 control tests and 184 independent consumer tests, including five new regression tests.
- Strict controls/Gallery/consumer Clippy; generated public API binding test; native example build and strict Clippy.
- 17 canonical actual token observations, 6,432 bounded cases. This is not a claim of exhaustive token/state coverage.
- Existing label Gallery GPU regression replayed after the shared fix: 432 cases/864 frames.

Use the repository shared-cache policy, e.g. `scripts/tekesui_build.py` environment, and `cargo test --locked --manifest-path examples/composition-consumer/Cargo.toml --test layout_text_services`. `refresh.py` records the complete scoped sequence and cleans exact test executables. It runs no performance measurements.

## GPU / Gold7 scope

The small host under `design/reviews/layout-text-preflight-20261021/host` reuses the existing CoreText and GPU adapter. 48 cases/144 real GPU frames cover editable/readonly, four themes, three font scales and 280@1 / 520@2. Rejected scenes are empty. Recovery and collapse/reopen match independent fresh valid consumers pixel for pixel; input baseline and reading attachment are retained. PNGs and result.json are retained.

Gold7 comparison here is a scoped no-visual-delta check: valid rendering remains identical to a fresh valid original field, rejected frames paint/publish nothing, and recovery does not steal focus. It does not establish a matched ClientKit product-page visual review, VoiceOver hearing or whole-library maturity.

## External-screen native host

Run the existing native consumer with `TEKESUI_COMPONENT_CASE=layout` and `TEKESUI_COMPONENT_EVIDENCE=<owned directory>`. It reuses the normal native input and AX adapter. A deliberately failed frame clears native input/AX publication and fences ordinary input; only the review recovery shortcuts remain available. No cloned text editor or traversal is introduced.

- Command+1: collapse using a valid backend.
- Command+5: remove backend while hidden.
- Command+2: reopen without a backend (must reject).
- Command+3: restore backend and reopen (no automatic focus).
- Command+4: remove backend while already active (must reject and cancel focus).
- Command+6: toggle font scale 1 / 1.5.

`native-runtime.json` binds this binary to exact WIP input hashes; `native/checkpoints.json`, actual `valid-AX.txt` / `rejected-AX.txt`, screenshots and `native-acceptance.json` separate native evidence from the GPU results. The first host attempt incorrectly fenced TextKey Finished acknowledgements along with fresh input, leaving an old key chain and preventing later Tab entry. Its failed observation is preserved under native-attempt1; the final host lets acknowledgements drain through the existing public key path. All three final real Tab entries pass, including after active-backend removal and collapse/reopen at 1.5 type scale. Native publication/input remain fenced during failure. VoiceOver stayed off.

Do not rebind the previous Issue31 native receipt to this newer composition source, and do not modify the frozen 545ef8d70d89 archive being accepted by AgentKit. Any consumer delivery of this fix needs a new immutable source identity and independent acceptance.
