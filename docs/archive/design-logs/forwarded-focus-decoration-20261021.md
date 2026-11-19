# Issue 33 — forwarded original focus decoration

Root cause: ordinary Content paint traversal identified original Exact focus only by downcasting ButtonContent/CompoundButton. A legitimate wrapper forwarding one retained CompoundButton appeared as another concrete type; the subsequent adjacent row covered its lower ring. The before-fix real GPU test failed for wrapped row15002 at y31 with both baseline/focus pixels white. This is a producer reproduction of the receiver boundary, not a claim that direct-widget evidence covers the receiver.

Shared fix: default-false `Component::focus_decoration_above_siblings()`. The original ButtonContent declares its existing Exact/focused/focus_visible/enabled/non-loading predicate; CompoundButton forwards to that SAME action. Consumer wrapper forwards one method to its original CompoundButton. Ordinary traversal queries the trait plus active ancestry, retaining its existing focused-branch-last order. No new focus state, paint engine, layers/protocol type, overlay, token value or host-specific logic.

Applicability: direct compact actions and legitimate components delegating original action layout/input/paint/semantics. Tests mix direct and delegated original Exact10pt rows with a normal Minimum action. Pointer phases, cancel/capture, keyboard/AX and lifecycle stay with the original Button. Wrappers capture business activation results only. Exceptions: inactive/disabled/loading/non-keyboard focus is not raised; Minimum defaults stay unchanged; a component painting its own children keeps its declared ordering. This only changes ordinary sibling paint traversal, not layout, half-open hit/AX order or overlays.

Validation must separately bind public API, real forwarding consumer, real GPU top/bottom ring pixels, actual external native, scoped Gold7 and actual AgentKit wrapper. Do not relabel direct-widget receipts or a Root receipt as isolated archive/receiver acceptance. No performance testing; VoiceOver remains off unless separately needed.

Builds reuse `target/review-cache` with the existing build lock. Remove unused regenerable owned artifacts after checks; keep source/WIP/frozen input/logs/receipts/images and the final native executable needed for reproduction. Do not clean unknown caches or live app outputs.

Producer evidence: 5 targeted API tests, 426 controls/245 consumer regressions; 72 actual GPU cases/288 frames across4themes/3type scales/2width-backing pairs;17actual token probes/6432bounded cases (`complete=false`); strict Clippy and DS/pure-Rust/Gold7-contract/composition gates. Exact Root native source+harness identity0e8578af063e45a9d475b8bac7dc59048bb6cb15206875a69053079f570cf34a passed10active externalU2790B@2 checkpoints. Original pointer Release dispatched after release and Press during down exactly once, with keyboard/AX separate; disabled/loading refused input; original wrapper keyboard ring survived adjacent rows and large type+ReducedMotion. Ordinary Minimum36pt remained. No physical held-pointer Escape/unmount or VoiceOver hearing claim; cancel/unmount are public real-component tests. Small-host visual evidence is scoped, not matched AgentKit/ClientKit product-page acceptance. The unused final native app is retained as an acceptance executable; temporary test binaries and compilation intermediates are disposable after unused/dependency checks.


## Unified Button consumer acceptance — 2026-10-29

The retained public consumer now delegates to the unified `composition::Button`
(the earlier `CompoundButton` name in the historical evidence above is unchanged
there). No new production forwarding seam, state machine, host adapter or paint
engine was needed. The original five public API tests remain unchanged. A sixth
adds held-pointer Cancel, disabled, loading and unmount for both Release and Press
activation phases: Press has already activated once on down; interruption cannot
undo that business action or activate it again on up. Re-enabled actions accept the
next complete gesture once.

The executable public pixel regression is
`examples/composition-consumer/tests/forwarded_actions_gpu.rs`. It uses the
existing `forwarded_actions` consumer, with direct and delegated Exact 10pt rows
and the unchanged Minimum 36pt row. Its reference paints the same retained action
subtree through public `Content::paint` over the unfocused neighbor scene. The
actual image uses ordinary `ContentHost::render`. It compares every pixel in the
upper and lower out-of-frame ring bands, including AA and corners, and separately
requires visible ring ink on both edges. It does not copy the control's ring,
focus predicates or event engine. Repainting the reference action interior cannot
satisfy an out-of-frame assertion.

| Original #33 acceptance | Current public evidence |
| --- | --- |
| Direct and forwarding Exact/10pt rows; Minimum default unchanged | Existing `build()` fixture; API declaration/geometry tests; GPU checks all original frames and Minimum36 |
| Both keyboard-visible ring edges survive adjacent chrome; geometry/hit/AX order unchanged | 72 real Metal cases: four Themes × three type scales × two width/backing pairs × direct/Release-wrapper/Press-wrapper; full upper/lower band comparison, frame and semantic-order equality; original half-open API test |
| Original Press/Release, Cancel, disabled/loading and unmount own input | Existing original once-only and keyboard interruption tests plus new eight-combination held-pointer interruption/recovery test |
| Decoration remains one whole AX action/name and no second input owner | Original full-name/semantic-order test, original keyboard/AX activation test and wrapper business capture count |
| Shared property delegates to retained original action | `ForwardedAction::focus_decoration_above_siblings` still directly calls the same `Button` method; unchanged trait default and ordinary traversal |

Actual main validation used `a6a2dc796b6a0c9c2afc79af2944e3fd7f1b7eb7`
plus the two explicitly received test paths. All six CPU tests, the 72-case Metal
test (216 original PNGs), and strict Clippy for both tests passed. The existing
build lock/environment, registered SSD review cache, GPU occupancy lock and
Rust 1.95.0 locked/offline commands were used. Source bytes remained unchanged
before/after all three commands. Exact source/program/log/image receipts are at
SSD `artifacts/issue-33/mainb6e4755-forwarded-focus-20261029-01/main-rust01`.
The smallest observed visible ring area was 340 pixels per edge; this is an
observed result, not a new API threshold or performance metric.

Historical native evidence retains its original program identity and is not
relabelled as unified-Button native acceptance. This batch ran no native window,
physical pointer/IME, VoiceOver, product-page/AgentKit integration or performance
measurement. It proves the original public consumer and shared-library contract;
actual downstream product adoption remains separately recorded. Remote delivery
of these new tests and Issue closure are owned by the coordinator.
