# Authorized independent-layer composition — 2026-10-29

The user selected an AppKit-style independent child raster followed by the parent
container's mask once. Correct clipping, image quality and pointer/AX/text mapping
remain required; equality with rerendering all commands in one different context
is not an acceptance condition. This is an explicit new composition contract,
not a proof failure fallback. The original finite `VerifiedFrameOutput` contract
and closed #127 keep their original semantics. Render API and same-raster quality
references are described in
[the existing boundary design](remote-prepared-frame-boundary-proof-20261026.md).

## Current B2 domain acceptance — 2026-10-29

The original three-Reader native sequence now passes on exact program
`b52ef50d31c2690b7c150a7370ca52f0f1b27f88a0db9b65087ac4f9fe7949df`.
The coordinator reviewed `native-r15`: normal exit0, no timeout, original
reveal/capture/release, parent clipped pixels, pointer count1, same native AX
key/selector count2, rounded outside refusal, B2 text/unmarked, actual Window
substring/selection and candidate rect height16, real resize old-frame input/query
NotReady with no replay, and final close assertions. Its source manifest is
`verification-r29/main-source-manifest.json` (SHA256
`9fa600df73e1c58bde95973c7c9cabfb2cbeae9517ffa64a6ca4f80e4e0246f9`).
Artifacts are under the retained `independent-layer-r1/` evidence root. This is
the B2 domain result; final combined CI, public revision and Issue acceptance
remain coordinator-owned. Earlier candidate and failure records below retain
their original epochs and do not supersede this result.

The actual no-window main selector also passes same-key AX publication,
65-clock backpressure/latest catch-up, exact captured Up, sibling repaint,
original IME/owner binding, queued native commit plus CaretBlink and current
positive-caret recovery, refused-corner redraw, and true resize/stale negatives.
Platform default45/feature55 (overlapping), affected strict/build, Runtime60
and Controls344/Panel22 evidence is retained with its source identities.

The main/native milestone was committed as `602d444` and PR184 opened; that
records submitted source, not a merge or public checkpoint claim. The existing
`none` and `remount` scripts subsequently ran against the exact r29 program:
None passed, while same-instance pending remount reached its begin marker and
failed new-session publication. The retained old-session pending response/revoke
queues motivated the finite session-resource reset. A subsequent default-both
run reached None and exposed a pending Prepare/Layout ordering failure; None
now shares the existing one-outstanding-grant coalescing path. Both real failures
remain recorded, distinct from earlier pre-begin r16 failures.

The complete lifecycle follow-up now passes on exact program
`421dcbdc516a5e7029a53443e306838b5baf1d30a8b7c8205ae2cedb1e45b4bc`,
manifest `verification-r31/main-source-manifest.json` (SHA256
`90200f291f58d84c10d946a499b9d931b8e69dce6df1caf2dbc1f1313004bc88`).
The original isolated None, isolated remount, and unchanged default-both each
reach their actual begin/pass markers. Default-both records both lifecycle checks:
same-session context removal/recovery with stale callback refusal, and a fresh
process session after same-instance remount with old weak frame handle/snapshot
refusal and reset Button/capture state. The original full main prefix also
passes in these runs; the entire consumer test source is byte-identical to r29.
Runtime60, Runtime/consumer strict, complete build and normal generation/composition
pass. Raw evidence and immutable failed/successful programs are under
`lifecycle-r1-602d444-exact/`, `verification-r30/`, and `verified-lifecycle-r31/`.

This resolves the two requested existing design9.5 lifecycle interleavings for
coordinator acceptance; it does not add new lifecycle cases or claim all possible
interleavings. The new lifecycle program did not run a native Window. Original
Window/AX/text/resize/close evidence remains attached to exact r29/native-r15,
without re-signing it to the new program. Physical IME and VoiceOver remain
not_run. Final common CI, commit/merge, public revision and Issue closure remain
coordinator-owned; no whole-library or immutable public delivery is inferred.

## Parent ordinary input and delegated focus — current candidate

Native R3 reached the real parent pixels, but ordinary parent Down was refused
before reaching the child: `ProcessContainer` has an original delegated tree,
not a local prepared-action contract. The Panel's required reveal boundary also
correctly prevents treating its entire rounded allocation as a focus witness.
The new default-false `Component::delegates_curve_focus` hook explicitly bridges
this seam. It does not change `Recursive` into `Local`, fabricate a target
witness, or grant prepared effects. Ordinary input still requires current parent
curve geometry and the component's current input-ready gate. The runtime opts in
only for an available, acknowledged independent layer at the same spatial
placement with no geometry/cancellation transition.

An explicit private delegated-focus plan preserves the original window, scope,
disabled, mount, focus-epoch, editor and IME checks, revalidates the exact parent
geometry and delegation, and adopts only logical keyboard ownership. The child
original Host admits its actual target against the exact external chain. This
plan has no target witness and never calls `root.reveal` or writes a Scroll
offset. It applies only when there is no ancestor Scroll; an ancestor Scroll
continues through the original pure reveal planner and its existing explicit
refusal when a sufficient plan cannot be formed. This does not claim all remote
focus/reveal or parent scrolling configurations are accepted. Existing explicit
minimum-focus requirements remain on the original strict path.

The actual accepted child pointer/AX focus handoff suppresses just that duplicate
focus delivery when the parent adopts it. Later explicit focus, blur and
traversal retain their original delegation. A composing delegated keyboard owner
without a local `can_unmark` representation must commit/unmark in its original
Host before cross-owner focus can transfer; pointer and AX refusal preserve the
original marked text and the other child. `key.rs` session/mount/focus-epoch and
binding fences are unchanged.

A changed stamped child reply withdraws `input_ready` immediately. An already
established captured gesture may retain a primary Up/Move/Drag in the existing
queue while the same spatial layer republishes. This keeps the original
session, View and capture target/mount/serial; it never re-hit-tests old Up at a
new owner. Commit drains each changed callback once, waiting for its replacement
publication before later callbacks. Real geometry revocation cancels capture,
so an old queued Up is explicitly refused without activation. New Down never
receives this continuation permission. Service inspection consumes only its own
response, preserving Curve frame/query replies in the shared queue.

The r14 actual failure identified two Prepare requests (proposal12 then13) before
any corresponding Layout reached the child: `rendering=false`, no queued Layout.
Direct service Prepare and the coalesced View Layout lane are therefore not an
atomic pair. The bounded repair keeps one sent Prepare/Layout per container until
its exact existing `GrantPrepared` is consumed. While it is outstanding, the
container keeps only its latest unsent parent context, withdraws input readiness,
and issues no new outer request ID. Same-space captured continuation retains its
original spatial identity; actual allocation changes keep geometry/cancel fences.
The old request/proposal reply is validated and its frame credit retired without
publishing old pixels or queries. The next ordinary Host freeze submits the then
current context; an actual geometry revoke precedes that next grant. No new View
scheduler, wire operation or Render contract is introduced. The real regression
adds a second Host render before importing the first grant's frame, then requires
the same original Button action exactly once. The r15 affected runtime60,
runtime/consumer strict/build and actual three-Reader chain passed under their
exact receipts. Subsequent r16 test-only runs failed before either new lifecycle
case, so they are not lifecycle reproduction evidence.

The r19 identity trace observes the earlier main-chain defect directly. Both
actual cases show a child with unchanged shapes, no dirty geometry, no pending
cancellation and no own repaint immediately sending Revoke on another parent
clock generation. One trace retains the original capture `(target3,mount29,serial4)`
through that erroneous Revoke. Later geometry/cancel flags and viewport revisions
churn although all three processes remain Ready and allocation sequence stays1;
neither case reaches its lifecycle begin marker. The narrow candidate removes
only the own-`repaint_dirty` requirement from same-space typed-layer regrant.
Actual geometry/cancellation flags and exact full shape equality continue to
control the original revoke path; frame/input/query authority still withdraws.
This preserves a gesture regardless of which sibling caused the parent clock to
advance. The original repeated-render/Up exact-count and resize/stale negatives
remain required. Candidate verification is pending; pending-cancel and global
publication convergence must still be assessed from actual results, without
weakening any assertion. The directed original regression now first settles a
captured Button's own repaint, updates only a sibling's original View context,
then refreshes the real parent Host. The exact capture target/mount/serial and
pressed state must survive before its one Up activates exactly once. The two
prior fast/interleaved clicks stay intact; the extra actual click raises only the
later cumulative AX/corner/resize count oracles by one. The scoped runner selects
`TEKESUI_PROCESS_LAYER_LIFECYCLE_CASE=main` to run the complete main chain while
skipping the frozen lifecycle cases. The original default still runs both
lifecycle cases; `none`, `remount` and `both` remain selectable. They are not a
prerequisite for this scoped main-chain batch. Native-r5 has only pixel/Down-Up consumption evidence;
its business-count/AX/text/resize sequence remains incomplete. No native timing
sample alone establishes a continuous acquire wait or missing present.

The actual r21 main-only run passes all three original clicks (including the
directed sibling-context capture) and AX with exact counts1,2,3,4. It reaches the
true resize negative, then fails settle: allocation sequence remains2 while
viewport revisions rise, and same-shape contexts have geometry not dirty but
`cancel_pending=true` before repeated revoke/geometry-dirty promotion. The cancel
wait is an input-admission fence, not evidence of another spatial change.
The next bounded candidate keeps that fence active across an unchanged typed
spatial regrant, carries its exact already-validated revoke acknowledgement,
and does not promote it to dirty geometry. A real allocation/shape/dirty change
still issues a fresh exact revoke and clears confirmation. Input becomes ready
only after the current exact frame/query publication checks the pending revoke
queue is empty and cancellation was confirmed; that commit alone clears the
pending fence. Initial grant with no old lease retains the original no-revoke
behavior. The unchanged true-resize regression must still reject its original
queued Up, preserve exact business count4, leave no capture/pressed state, and
refuse old mutating input and read-only query. Candidate results are pending;
lifecycle candidates remain frozen and excluded by the explicit main selector.

The actual r22 main-only no-window chain passes all original checks, including
count4, sibling capture, true resize and old Up/input/query refusal. Native-r9
retains a different failure identity: the parent continuously acquires/submits/
presents, and the queued original Up replies `activated=true` with capture
released, but its replacement frame/count is not confirmed. After clock updates
begin, 64 AnimationFrames precede `component closing`; this alone is not count1
acceptance or a proof of which bounded queue failed. The native driver forwards
each clock through `get_mut`, marks Host dirty, and skips its frame/reply handler
when dirty. This can prevent both current publication and draining the original
SetViewContext replies; the service queue is bounded at64.

The related consumer repair shares one original native/headless phase order:
apply all original Host updates, freeze current geometry, progress every exact
frame/reply handle, then commit with a geometry-preserving repaint. Clock-only
updates use the existing whole child batch's frame readiness as backpressure
because they share one parent geometry clock. The parent clock continues
advancing; the next ready tick forwards the latest time. Font/theme,
caret and other non-clock context changes retain their original immediate path.
No new scheduler, timer, capacity, sleep, VSync or drawable latency is introduced.
The original fast-Up regression now sends65 clock ticks while publication is
pending, then requires latest-clock catch-up and the same exact business count.
An explicit `TEKESUI_PROCESS_LAYER_LEGACY_CLOCK_PROBE=1` diagnostic uses the old
non-coalesced forwarding call pattern against real children, to distinguish a
bounded transport failure from a platform wait. It is not an acceptance pass.
Closing errors now retain the original component status for diagnosis. These
new candidate results and original native business/AX/text/resize are pending.
Lifecycle candidates remain frozen outside the explicit main-only chain.

The actual r24 no-window shared driver passes Runtime60/strict and consumer
strict/build, then exposes a remaining main-chain publication loop after the
first exact click: all children stay Ready, unchanged-space grant replies retire,
and parent geometry generations advance without current publication. A deferred
grant acknowledgement was still classified as a Host/model update, so each
completion invalidated the same parent clock needed by its siblings. This is a
resource-progress defect, not a lifecycle red example or evidence of a platform
wait. The next bounded repair resumes the latest already-frozen context in the
original ProcessContainer paint owner after exact acknowledgement. It calls the
existing setter, retaining all real geometry/revoke/cancel checks; it does not
mark the Host dirty solely for resource completion. The shared driver always
performs a geometry-preserving repaint after polling all receipts, including
when an old grant was retired rather than a new image prepared. Real Host/model
updates still perform the original layout/freeze. No new state/API/scheduler is
added; the clock/Up/catch-up/sibling/true-resize assertions remain unchanged.
These candidate checks and native business completion remain pending.

The actual r25 shared-driver main chain passes normal Runtime60/strict,
consumer strict/build and the complete real three-Reader check, including65
pending clock ticks and latest-time catch-up. Its single old producer-pattern
probe fails at tick63 with `component request queue full`; that is an actual
bounded request failure, not proof of native-r9's hidden specific queue.
Native-r11 then passes real pointer count1/unpressed and the parent pixels,
but the fixed-time AX step sees only groups while a clock grant is unpublished.
It does not reach the original native selector. This is distinct from the old
closing failure and is not evidence of a changed key or a platform-render defect.

The related headless regression now checks the same saved native AX key across a
real clock grant: absent while input is withdrawn, denied without business effect,
then restored with exactly the same key after current publication before the
original accepted Press/count4. The B2 native script opts into a selftest-only AX
publication gate: it waits for that exact key, runs the unchanged original
AXPressKey/native selector after the real handler has returned, and continues
with the original remaining action delay deltas. Default run_script remains
unchanged. No retry timer, polling loop, new deadline or change to key/action,
selector assertions, original business counters, scene or45second guard is added.
Only the explicit selftest has a pending step; ordinary windows have none.
This preserves missing/stale-element and disabled-action failures in the original
selector path. The new regression and original native completion are pending;
lifecycle candidates stay frozen outside the main selector.

The original child remains the business-action owner. `ContentResponse.activated`
is intentionally not forwarded as a second parent action. Native Up and AX each
must increment the actual original Button count by exactly one and leave the
actual parent action queue empty; Down must acquire capture without activation.
The no-window original three-Reader regression now covers that fast parent
Down/Up path and the explicit Down → Host render starts a same-position grant
→ Up → frame publication ordering. The latter asserts withdrawn input readiness,
unchanged published revision and the original spatial delegation before delivering
Up; it uses no timing delay. Real parent AX, text ownership and callbacks,
cross-owner marked text refusal and rounded-corner rejection remain required.
A second grant-in-flight sequence inserts real geometry revocation before the
queued Up is published (zero extra activations/no residual capture). The existing
production clear/dirty transition remains unchanged: a successful synchronous
`begin_curve_grant` restores the typed layer and geometry state before returning.
These regressions test the observable phase rather than assigning authority to an
unobservable intermediate state. The text replacement
range is the original current UTF-16 length, rather than the old four-unit
`A😀B` fixture's range applied to Reader's five-unit `draft`; the required final
`B2 text` assertion remains. These new candidate checks and the next original
native run are still pending; earlier receipts retain their exact old scope.

The wire opts in with `NegotiateLayer`/`LayerNegotiated` and
`PrepareLayerGrant`. A `LayerFramePublished` is ordinary independent pixel data
bound to the current original geometry lease; it never claims parent-AA proof.
The old negotiation/grant/frame variants retain their old proof path. A wrong
composition acknowledgement, untyped layer frame, extent/phase mismatch or stale
lease rejects before new image/input publication. There is no implicit fallback.

Retain the actual original Grid, Reader and controls. The child Request's integer
logical layout (333x500 in the original case) and the parent's fractional
allocation (333.33334 points wide) have different meanings. With scale `s` and
original allocation translation `T_px`, choose `P=floor(T_px)` and
`phase_px=T_px-P`. The backing extent is
`ceil(Request.logical_size*s+phase_px)`. Paint the SAME original child Scene
translated only by `phase_px/s` into that zero-origin independent raster. The
parent places its bitmap at `P/s` with size `pixel_extent/s`, preserving one texel
per device pixel. It does not stretch the old 666-pixel bitmap to 333.33334 points.
The original parent allocation clips raster padding and its ancestor curve is
applied once. Unused fractional allocation retains transparency/parent background.

Pointer inverse translation and AX/text projection continue to use original T;
the raster's P plus its internal phase reconstructs exactly that T. The explicit
layer grant carries original geometry and integer content layout separately.
Both producer and receiver validate scale, layout, phase-derived extent and lease
identity. The scene fragment transform uses the existing bounded fallible bridge;
no new Host/editor, child rebuild, shader, AA policy or mutable state replica is
introduced. Existing reader capture/reveal/owner continuity and resize/stale
refusals remain required.

The original B2 native fixture selects Reader. Its previously intended opaque
background witness is now inserted on that selected B2-only path, retaining all
68 original primitives, controls and decorations. It makes a missing parent mask
observable at the rounded corner; it does not certify other drawing or bypass
Renderer validation. The independent-layer candidate passed protocol lib16,
runtime lib60, protocol/runtime and consumer strict checks, a full consumer build,
the two Render GPU/IOSurface quality fixtures, and the real original three-Reader
IPC/GPU integration without a desktop Window. Panel's 20 public Host tests and
its directed strict target also passed, including stale geometry, actual layout
failure, recovery and explicit unmount context removal.

The actual integration retained Request333x500/Panel333.33334 at scale2 and
imported 1,667,884 opaque witness pixels. Initial read-only publication has no
focused editor/text. Explicit focus produces the original draft projection;
focus/capture/reading reveal/release each run once and wait for their actual
replacement frame/projection. Original owner, focus epoch, text binding, capture
and full text/caret continuity pass. An unchanged-placement repaint preserves
capture, while a real resize cancels it and refuses both old mutating AX input
and old semantic query. The native and no-window entry share this original
setup; readiness never substitutes for focus or publication.

Actual evidence remains under
`artifacts/issue-81/b2-domain-20261029-01/independent-layer-r1/`:
Render quality in `verification-r1/`, final runtime/consumer/real IPC in
`verification-r8/`, and Panel20/strict in `verification-r9/`. Earlier native
`native-r1/` text-projection failure, premature-query/false-removal/capture failures,
compile errors and the invalid NaN negative-fixture premise retain their own
unmodified receipts. NaN layout is normalized by the existing public layout
boundary; the final failed-layout test withdraws required text services and
asserts the actual `MissingTextServices` error instead. No predicate in the
original native text/capture/reveal/release assertions was weakened.

This is bounded domain evidence. The original desktop-native pointer/AX/text,
reading/capture and resize/stale script, and final combined CI, still require
coordinator acceptance. Physical IME and VoiceOver remain unverified.

## Public consumer entry (macOS with Metal)

After the coordinator publishes a checkpoint containing this batch, clone TekesUI
and check out that exact published checkpoint. The independent Cargo workspace
uses public path dependencies and needs no private worktree, artifact directory
or local source override. With Rust 1.95.0 installed, run from that clone:

```sh
TEKESUI_PROCESS_NATIVE_B2_SELFTEST=1 cargo +1.95.0 run --locked \
  --manifest-path examples/process-content-consumer/Cargo.toml \
  -- --independent-layer-only
```

This launches three original Reader child processes and uses real Metal and
IOSurface transport. It opens no desktop window and prints its actual integration
result. With no lifecycle selector, this runs the verified original `both`
suite after the main chain. `TEKESUI_PROCESS_LAYER_LIFECYCLE_CASE=main` selects
only the main chain; `none` and `remount` select the two existing independently
verified lifecycle cases. The default and each case keep their original checks.
A first build downloads the locked dependencies; the validation runner's
`--offline` setting is not required for a fresh consumer clone.

The original desktop B2 entry uses the same package and opt-in fixture:

```sh
TEKESUI_PROCESS_NATIVE_B2_SELFTEST=1 cargo +1.95.0 run --locked \
  --manifest-path examples/process-content-consumer/Cargo.toml \
  -- --native
```

This opens one native window and runs the original scripted pointer, AX, text,
reading/capture and resize/stale acceptance. Run it alone on an idle desktop;
repository validation reserves the window/input resource before launch. These
programmatic checks do not establish physical IME or VoiceOver acceptance.

---

# Local main delivery boundary — 2026-10-27

This is the existing Panel/curve design. Historical Gallery Root evidence below retains its original source identity; it is not a remote release or a current-main acceptance result.

Local main already contains the #80 shared shape/clip and R1/R2 glyph-scissor implementation, #97 fallible Scene fragments, Surface inheritance and #102 A. The batch committed as `7ab8e32`, based on `8511dad88d98f47aeb64085d7210d113a4114500`, receives the real single-slot `PanelContent`, original-tree curve paint/input and frozen geometry, required-focus evidence, and the shared readonly Host/reading plan→validate→commit path. It also includes the necessary #98 Start/Nearest/overscroll compatibility corrections and #102 successful typed-update geometry-lease retirement. These shared seams cannot be separated without leaving the original public consumers broken. The existing52 registry identities and maturity remain unchanged; `PanelContent` is an additional `tui.surface` API binding.

The reading transaction no longer performs trial layout, live offset mutation or rollback. Custom translation owners must supply their actual arranged pure query; unknown layouts stay unsupported. Library SplitView slots retain their original owner-level pane clip, separately from descendant-only custom clips. Planning binds the current visual/offset state; accepted explicit navigation uses the existing commit that clears old overscroll/motion. Partial visibility of a tall target remains distinct from an actual limit/shape clamp. Ordinary CurveLocal controls retain their original reducer after successful current focus/reveal gates; this does not invent a prepared-action contract.

The legacy macOS Rect-only adapter explicitly refuses curve trees, including actions. The real Host snapshot/hit/parts remain curve-aware; a bounding rectangle is not point authority. This refusal prevents a false positive and does **not** complete the original B1 positive native/Window atomic-publication, point/range/text-generation chain. That chain, B2 parent-image clipping and cross-process authority, the original #82 450/18 combination, and immutable public delivery remain separate unreceived or unverified work. #81 stays open. Full virtual-row #98 acceptance on local main is not inferred from this finite common-core batch.

Validation and exact round/source/program identities are recorded on SSD under `artifacts/issue-136/panel81-main8511dad-intake/`. The actual Panel Metal checks cover five pipelines × five scales × light/dark (50 captures), plus ancestor-outside and zero-width negatives; the glyph input is a synthetic atlas clipping check, not native text appearance. The original Host15, item-navigation25, scroll11, Controls309 and selected public/editor/reading/action regressions are separate runtime receipts. Later pane-boundary repair reruns only the affected Split/Host/local-clip checks and pending strict checks. Earlier failures, candidate results and historical receipts remain intact; no evidence is reassigned to a different source. No GPU/native/IME/VoiceOver/performance result is inferred from compilation or from this source commit. No push, merge or publication.

The next received common dependency restores `ContentTextLayout` from the original TextSurface/Paragraph and guarded whole/part Host queries; it adds no shaper/editor or virtual-part imitation. Zero Split panes cannot publish retained layout. Accepted reading-wheel observation records only the actual consumed, current scoped owner/mount and clears before the next event/refusal. Original independent #82 CPU3, the three directed query/source regressions, Controls312, affected reading/zero-pane/public contracts and strict checks passed. This is a finite text/query foundation, not positive native/Window publication acceptance. The independent consumer and its actual450/18 Metal run retain their recorded source epochs under SSD `artifacts/issue-136/panel82-main7ab8e32-intake/`; adding these readonly queries and event-result observation does not change the validated GPU rendering inputs.

---

<details><summary>Earlier local-main foundation-only boundary (historical)</summary>

# Local main delivery boundary — 2026-10-26

This is the existing Panel/curve design, received from Gallery Root with its history retained below. “Main” in the historical sections means the Gallery Root integration at that date, not this checkout or a remote release.

The current local-main batch receives the #80 shared shape/clip and R1/R2 glyph-scissor implementation plus #97 fallible Scene fragments, pre-submit Renderer rejection and existing Dialog/PopoverView/OverlayPresence seams. These layers share Scene/Renderer dependencies and are committed together. The current checkout does not yet expose PanelContent or the later PopoverContent owner; future owner intake must preserve the same fallible bridge. No placeholder API or silent rectangular fallback was added.

On base `33c2d61` plus the bounded batch: Scene tests, Renderer unit/actual shape and fragment GPU checks, Controls lib tests, independent fragment-consumer tests, strict checks and Gallery/AppHost downstream compilation passed. Legacy compatibility now compares ten original PNG source fixtures unconditionally on main; capture output uses the configured SSD target. The original name-dependent private test did not compare on this checkout, so its first result was not counted as compatibility evidence. The corrected run passed; all earlier records remain intact.

Evidence: SSD `artifacts/issue-136/phase2-r2-main33-20261026-01/` and `artifacts/issue-136/fragment97-main33-20261026-01/`. Rustfmt subsequently changed only owned hunks (receipt retained); no generator, configuration, shader semantics or runtime behavior changed, and the prior runtime evidence keeps its exact pre-format source identity.

**Remaining integration acceptance:** the original #82 450/18 Panel matrix depends on #81 Panel/Host APIs absent from this local-main batch. It has not run here and is not replaced by the shared-library GPU checks. The supplementary per-run source contains 448 glyphs/12 runs, not a dedicated 260-glyph/7-run pairing. Receive #62 Surface inheritance, #98 Host/reveal and #81 Panel dependencies before running the original combination. Keep #81 B2/native/VoiceOver/publication obligations separate and incomplete. This local source commit does not claim all original #80/#81/#82 criteria passed on current main or close a planning Issue.


</details>

## Existing design and historical Gallery Root evidence

> 2026-10-26 当前主协调：#81 B1 本地原树实现与有限验收已接收；r11代际/既有reveal兼容、consumer/平台投影及五管线GPU通过。#81仍OPEN：B2远端父最终clip与跨进程曲线授权未交付，local-only明确拒绝，不能视为#45完成。历史段落保留原身份；当前接收见末节及 design/reviews/panel81-main-intake-20261026/root-acceptance.json。

> 2026-10-24 主协调接收并串行集成 #80 共享 CPU/Scene/Renderer 基础：1562输入组合验证稳定，Scene44/Renderer91、strict/fmt及controls/Gallery/AppHost编译通过，实际201 shape读回与候选逐字相等，10legacy原始golden无条件比较通过。接收说明及限制见末节。#81/#82与Panel/Content/native/远端仍待独立交付；下文旧阶段“未集成”保留时间身份。

# #45 原 Panel Content 与曲线子树裁剪：设计阶段候选

日期：2026-10-24；修订：amendment-1（主协调六项审阅修正，仅设计）。关联：[#45](https://github.com/TekesApps/TekesUI/issues/45)、#14、#22、#5、#56。状态：交 UI 主会话审议，未实现、未建执行票、未发布，#45 保持 open。新公开 API、协议与 breaking 决策须由主会话先问 advisor，本会话不调用 advisor、不再委派。

## 1. 输入身份、授权与现状

源码为 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` 加并行 WIP；不是公开可消费 revision。#45 原文审计的是公开 `09b4bab44b6dfa65487250ea816b610b9df59095`，其最新摘要另列公开 main `7c13ec0909601ac1e8a7b4a0a67f6e086d2307b7`；这些是 Issue 引用身份，本轮未对远端 main 作重新接收。逐文件哈希与定位见本设计同名 review 目录的 `source-audit.json`；不是全部 WIP 的冻结 manifest。

本轮唯一写入为本文件和 `design/reviews/panel-content-curve-clip-20261024/`。不写 Theme、公共导出、矩阵、#71/#72 候选、#66 Tree 文件，不修改旧冻结、生产源码、target 或 cache。没有构建、GPU、窗口、物理 IME、VoiceOver、性能、Git 发布。无需创建945源码红例：下述为源级可推导反例与必要未来测试，不记作已执行 Scene/consumer 失败。

| 原拥有者 | 当前入口与具体事实 | #45 缺口 |
| --- | --- | --- |
| Surface/Frame | `feedback_view.rs` 的 SurfaceView::render_at_scale 返回 ViewLayout；content 用 space-loose 内缩，visible/hit_test 是 Rect。`paint.rs` 的 surface_kind_on_surface_with_elevation_at_scale 用原 corner、elevation、边框与 Surface 解析 | 无原 children、descendant focus 状态或子树裁剪声明；画圆角背景不裁后续图元 |
| Frame/Split | `extended.rs` ControlFrame 复用 ContainerLayout，contains 只测 visible_frame；focus_bounds 使用原 focus width/offset/AA guard。`layout_view.rs` SplitViewView::arrange 产生两个 pane、divider 与矩形 divider_hit | pane 和 divider 不提供圆角子树能力；不能把分割线 hit slop 或 focus outset 当作 Panel radius |
| Scene | `scene.rs` 只有 PushClip(Rect)/PopClip；CornerRadii 已有 TL/TR/BR/BL 数据 | 没有 shape scope 或共享 CPU contains/曲线相交查询 |
| Renderer | `frame.rs` clip 栈只做 Rect intersection→整数 scissor；三类绘制批 Quads/Glyphs/Image。`quad.wgsl` 有 sd_round_rect 与四角半径 AA；`instance.rs` 对 Quad 半径钳到短边一半 | SDF 只作用自身 Quad/Shadow，不作用后续字形/图像；render pass 无 stencil attachment，不存在现成 stencil 子树 clip |
| 原 Content | `composition.rs` Component::arrange/paint 传 Rect clip，Content::paint_except 在 component paint 后 pop，再遍历 children；paints_children 是现有特殊拥有权 | 在 Panel 自己 paint 中 push/pop 不能约束普通后代；把 Panel 改成第二 Host 会丢原 mount、路由、焦点与 reading 事务 |
| Hit/焦点/reading | Content::visible、route、padding_focus_at、observe、pointer_regions 用矩形；ScrollContent 保留原 focus gutter，content_reading/reveal 使用可见 Rect | 仅视觉 mask 会留下圆角外命中、padding-focus、wheel 与错误的 reveal 成功 |
| AX/远端 | SemanticNode 有 frame/visible Rect；parts 只有 frame。macOS content_nodes 投影 n.frame/part.frame，不读取 n.visible；module-runtime/content_accessibility 与 app-protocol 同样无 shape 信息 | 仅改 SemanticNode.visible 还不够。原生/远端需要接收当前曲线与拒绝不可见目标的原 Host 权威 |

因此本设计不把 SurfaceView 存在、单矩形 clip 或 API 静态检查当作 #45 完成。

## 2. 原树语义与 Panel 几何

候选 `PanelContent` 是原 Component 容器，只拥有一个原 Content 根槽，根槽可为 Stack/Grid/Scroll 等任意原树。通过 children/children_mut 暴露这一原实例；不 clone/recreate editor，不添加 ContentHost，不复制 focus/capture/reading 状态。原 ComponentId、mount、TextSurface binding、选区、marked、ReadingDocumentVersion/owner/items、动作 drain 与 AX 父子关系继续由原 Host 管理。固定单槽应复用原 fixed_slots 校验；不允许普通 reorder 将固定槽变成多个布局 owner。

Canvas/Panel 是库语义外观，颜色从原 Surface context 继承/显式覆盖，anatomy 沿原 SurfaceKind。Canvas 为无边框的底层 canvas，Panel 为原装饰边界；当前原 painter 两者都使用 radius-panel，flat 不等于强改零圆角。默认 padding 引用 SurfaceView 已有 space-loose；默认 corner 引用 radius-panel；Panel hairline 通过原 decorative_border/backing/increased-contrast 政策；focus 用 boundary_styles.focus 与 paint::focus_ring 的 c-focus-width、c-focus-offset、border-focused、物理 AA guard。elevation 沿原 painter 的原角色，不另造阴影。引用源为 `design/source/base.tokens.json`、`design/source/layout.json`、`paint.rs`、`feedback_view.rs`。不抄数值、不新增 fallback/magic values、不改 Theme schema。

业务 regular 外侧 inset 仍由外部 ContainerLayout/父 pane 提供。Panel 仅从父分配得到 outer frame；库默认 padding 决定 child allocation。可配置 padding/clip 策略的公开形状待主会话审议；首批只讨论沿用默认和显式零内距以承载 self-framed 内容，不接产品 preset。零内距不关闭祖先 clip，也不消除原 child 的自主边框。

一份 arrange 结果（暂称 PanelGeometry，名称非已承诺 API）同时包含 outer、child allocation、已规范化的 outer rounded shape、ancestor clip chain 和 focus decoration bounds。默认裁剪语义为：子树受 Panel outer rounded silhouette 约束；padding 只影响布局，不偷偷再添加一个 child-frame clip，也不把 radius 简单减 padding 当第二轮廓。若未来需要内边界裁剪，必须显式定义 inset 曲线几何并独立接收，不能在各 consumer 推算。背景/阴影按原 ancestor clip 画；子树在曲线 scope 内画；Panel border 与 descendant-focus ring 最后画，仍受真实祖先约束，不受自己的子树 clip。这样边框不被不透明 child 覆盖，也不把 ring 按 outer frame 剪掉。

Panel 默认非交互、非 Tab stop；没有另一个 focus owner。原树中任何 active 后代持有原焦点时，Panel 只派生 descendant_focus/focus_visible 用于原 focus 边界，退出时消失，不触发 focus/blur、draft submit、selection、IME commit/cancel。装饰环不扩大 hit。默认不把 padding 点击变成聚焦命令；若外部已有原 padding-focus 声明，继续原 Host 规则并受同一曲线包含检查。语义父节点用原 Group（可选名称）；不吞子节点、不过滤 editor 身份、不把整个 Panel 当一个按钮。

普通 Content 与 self-framed child 用同一个槽和原 arrange。self-framed 表示 child 自己拥有内层 chrome/布局/服务，Panel 不重绘其 frame、不猜其像素、不剥离其 owner。公开的原 child 能力才可组合；任意 callback/原生嵌入/ProcessContainer 若不能接收统一 shape 命中与 AX 契约，应明确返回 unsupported 或标记阶段未接收，不能用第二 NSView/Host、业务 mask 或图像代理假称支持。Split 仍拥有 pane 分配/divider 拖拽，Panel 挂在分配 pane 内；divider 是外部兄弟，不被某一 pane 的曲线吞掉。

## 3. 一个共享 shape 权威

最小共享类型候选放 scene geometry 层：Rect 和 RoundedRect { rect, CornerRadii }，加不可变祖先 chain/句柄；仅支持 axis-aligned 几何与 translation；旋转、缩放变换、skew、perspective 的 shape/祖先组合显式返回 UnsupportedTransform，不能降级 AABB。原 TransformedImage 可以独立绘制，但若其参与本能力的几何/跨进程组合含非translation变换则拒绝。坐标统一为原 host logical points，scroll 的平移由原 arrange 结果给出；不在 shader/hit/AX 再推导 child frame。规范化必须一次完成：拒绝非有限 rect/溢出边、空宽高为空 shape；新 clip 输入的非有限半径拒绝；有限负半径为零，其余每角 clamp 到短边一半，与原 QuadInstance 既有有限值行为一致。旧 Quad 的非有限兼容处理不借此暗改；wire 只接受规范化有限记录。把规范化 helper 提到共享层并让 renderer/CPU 复用，避免新的 shape 与原背景半径漂移。

chain 代表精确集合交：`visible_region = node geometry ∩ ancestor Rect/rounded shapes`。AABB 只能用于粗筛、兼容布局参数和 AX bounding frame，不能用 AABB 代替 region。基础查询至少包含 contains(point)、非空交、可见交的 bounding rect、指定轴横/纵截面，以及 target rect 在可见区域中是否满足 reveal 的落点条件。非空检测须处理“target 与 AABB 相交但全在圆角缺口”的情况，不能只测 target 中心/四角：交集可能是一条曲线段。圆弧/线段相交或凸域精确求交算法须有切线、微小透镜、四角不同半径的定向用例，数值容差沿现有几何/reading约定，不新增扩大命中 epsilon。

Content 内部按原 arrange 单次传播有效 chain。原 Component Rect clip 保留为保守 broad phase 兼容旧签名；Content 自身遍历在 paint、pointer、padding-focus、scroll/wheel、新 selection 起点/read-only reading 的点命中、pointer_regions、semantic parts、reveal 时检查 chain；已有 owner 的编辑/选区/reading 另按第9.2节权限，不受“全clip即禁用”规则。普通子组件不需要各自实现圆角公式。paints_children/label/retained overlay 的特殊路径必须经同一个 scoped traversal，不能只覆盖普通 children。新声明 hook 或参数的具体公开形式交主会话 advisor，禁止先改 trait/导出。

paint clip push 必须覆盖整个普通 descendant traversal，不是 component.paint 一段；使用 scope/平衡检查保证 early return、skip 分支、空节点后 pop 正确，focused branch raised paint 仍处于同一祖先 shape 链。Panel 的外层 focus/border 位于自己的 shape scope 外、祖先 scope 内。嵌套 rounded shape 保留所有祖先，不能合并为一个半径或只取最近 Panel。

普通新命中一律要求 contains，圆角缺口没有新的 pointer/wheel/hover/selection-start/padding-focus 目标；已有选区拖拽与合法 owner 操作不因此撤销。既有 capture 拖出仍交付原 Move/Up/Cancel 完成手势，不因 shape 边界新造第二 capture；激活仍检查当前几何，clip/布局变化后的旧坐标按原 dirty-layout 守卫拒绝。键盘/原编辑器焦点在部分或完全裁剪时不被后台外观更新强行迁移；可视性不等于 mounted/active。Tab/显式 AX Focus 进入离屏目标需要原 reveal 成功与原组字转移 barrier，不能只因 frame 非空就宣称可见。无合法落点则失败并保留原 owner。

## 4. Renderer 最小方案及边界

首选共享解析 clip，而非引入 stencil attachment 或业务离屏 Host：Scene 新增 shape push（保留 PushClip(Rect) 兼容）与原 PopClip 平衡。Renderer frame 构建不可变 chain records，每段 batch/instance 持有 clip chain 索引；不能跨不同 chain 合批。复用 quad.wgsl 的 rounded SDF/四角选择作为共享 WGSL 片段，由 quad/glyph/image 三管线包含同一函数，Shadow/TransformedImage 也受该 chain。上传只读 buffer 的 records 和父索引，片元从当前 head 向祖先计算覆盖率；CPU 已完成半径规范化。目标空间由 fragment position 与原 scale 转换，TransformedImage 仍按 host 空间裁剪，不跟 UV 旋转。

coverage、规范化 shape identity、fill/border 合成与 Shadow 的区别，以第9.1节公式为本修订权威。不得由 shader 比较近似 float 猜测同轮廓，不去重独立 draw 的真实 alpha/层叠/阴影渐变。逻辑 hit/AX 使用几何内域，边界 AA 的亚像素 fringe 不是扩张的点击区。

scissor 只作保守加速：含 shape 的新 scope 用 bounds 的 outward floor(min)/ceil(max)，AA支持域与pixel-center换算精确见第9.6节，扩展后截 target bounds；解析 clip 才决定最终覆盖。严禁沿用 ceil(min)/floor(max) 作为新曲线的唯一裁剪。原无 shape 的 Rect 场景保持已有输出政策；shape 内继承的 Rect 也进入解析 chain，不能仍被内部向内 scissor 提前吞 ink。Separator 已观察到 x=280.0073852539、width=1 在1x向内 scissor 变零的教训，必须保留并做定向 GPU 回归，不能以非空连续 Rect 相交证明有像素。

不是多矩形近似，也不只改 Quad shader。Glyph/Image 漏接任一路径即不允许发布 shape clip 能力。实现须按设备 buffer limits 检查记录数、索引/深度与上传大小，溢出返回明确错误、不截链、不无声放开 clip；不先猜一个固定最大嵌套数。不能让 Primitive 的 `_ => {}` 对新 shape 指令静默忽略，Scene/Renderer 要有能力版本/不支持错误的配套测试。若解析链在设备上不可用，备选为 SDK renderer 内部 R8 coverage mask（同一 SDF 生成、所有管线采样、链集合交），但先报主会话评审；它不是产品蒙版或第二 Host。Stencil/tessellation 需要额外资源与 AA 语义，当前没有可直接复用的 stencil 路径，不作为本批先行扩展。

## 5. AX、滚动、reveal 与身份

AX 仍用原 id/mount/part key，region/chain 与 geometry generation 是原 Host 快照元数据，不另建语义树。新点命中、已有 owner 编辑和 AX Focus 三种权限、原平台点查询路径、part/text-range 同代及离屏结构的具体投影见第9.2/9.3节；首版“完全不可见则拒绝 SetSelection”等表述撤销。

当前 macOS AccessibilityNode 无 offscreen 字段，validator 拒绝零面积 frame，content_nodes 投影 full frame；因此不能假定现有平台已具备通用 offscreen/hidden 状态。修订明确所需最小桥接变化及失败行为，平台行为仍须针对 #45 consumer 单独接收。跨进程协商、图像最终裁剪、输入双侧校验及旧 peer 拒绝见第9.5节，具体公开 wire/API 继续由主协调决定。

ScrollContent 继续原 ScrollArea/reading owner/事务，曲线是 viewport 的额外约束，不重算文档位置、不把外观变化当新文档版本。reading 捕获/恢复仍用原 item/mount/anchor；新可见性查询按真实 region；已有 anchor/reading owner 不因全clip撤销，对上滚、explicit latest、prepend/reflow 的原优先级不变。原 focus gutter 继续给内容保留原 ring outset；padding 不足导致 ring被真实 Panel祖先截掉是布局约束，不能放宽 curve/hit 来掩盖。

reveal 先在不可变几何计划中求解，沿原 reveal_target 的纯目标计算入口扩展曲线约束，成功后才由原 reading/reveal 事务提交，见第9.4节。禁止先改真实 offset/用户状态再回滚试探。无可行解/过期/数值不确定不提交任何 owner、selection、reading intent 或 scroll offset。
## 6. 源级最小反例与必要测试

下述数值仅为合成测试几何，不是设计 token 或产品默认值。设 frame=(0,0,100,100)，radius=20，点 p=(1,1)，角圆心=(20,20)。p 在 Rect 内，但距离 sqrt(19²+19²)>20，在 rounded shape 外。SurfaceView 的 ViewLayout.hit_test 接受 p；原圆角背景在 p 的 coverage 为0。把真实 child 的分配/原 semantic_frame 设在角缺口内（可由自定义原 Component 复现），目前 Content::route 会按矩形接受，Content::semantic_nodes 给出矩形 visible，macOS content_nodes 仍投影 full frame。Scene 的 Rect scope 无法阻止 child 的不透明 Quad/Glyph/Image 在同一区域绘制。这里证明的是缺失契约可导致的不一致，不声称已通过 PanelContent 实例/GPU复现——该 API 尚不存在。

实施后至少需要以下有独立输入身份的测试：

1. shared shape 单元：四角不同半径、负/非有限/退化、切线、AABB假阳性、微小曲线交、重复clip幂等、嵌套恢复/空clip/栈不平衡/设备容量拒绝；CPU点包含与GPU边界相同规范化。
2. 原 Content consumer：文本+按钮+真实TextField/Composer+ScrollContent，保留原 ID/mount/editor binding、marked/selection/focus/原动作；圆角缺口拒绝pointer/hover/wheel/padding-focus/AX点命中，capture出界仍完成原手势，部分可见part检查。不能用测试专用Hit代替原route。
3. 同树原reading事务：prepend/分页/流式、宽度重排、嵌套scroll、用户上滚和explicit latest；检查原version/anchor/item mount/无手动offset补偿。reveal覆盖“仅在AABB角可见”、交叉轴无解、过大target、几何变更与旧请求拒绝。
4. GPU：Quad/Shadow/Glyph/Image/TransformedImage逐一跨曲线，异色背景显漏、透明叠加/重复clip显双AA；0.5、1、1.25、1.5、2 backing及原text scale组合，窄/零/非整数起点/嵌套Rect与rounded clip；保存有ink的正例和缺口无ink负例，避免双方scissor都空的假通过。深浅与increased contrast用原主题，不能复制Theme闭包。
5. 独立第二消费者及普通/self-framed/Split组合，真实平台AX adapter与本地/远端数据接收；固定依赖 revision、manifest、程序/命令和GPU收据。真实窗口、中文物理组字/candidate UI、VoiceOver、性能分别记录，不以静态/package/API或GPU代替。

## 7. 有限执行 Issue 候选（未创建）

按 A→B→C 串行，不把共享文件拆给并行 writer。以下是预留独占集合，实际开工前主会话与#71/#72/#66核对新基线；没有本设计对生产文件的写授权。`composition.rs`、公共导出、协议枚举等共享接缝由主协调指定单一 writer，不与 Tree/Progress 同时修改。Theme、公共矩阵不在三个候选中；需要能力记录由主会话统一维护。

### A. Scene/Renderer 共享 RoundedRect clip 基础

依赖：主会话 advisor 确认公开 shape/指令兼容性和设备不支持策略。独占拟改：`tekes-ui-scene/src/{geometry.rs,quad.rs,scene.rs,lib.rs}`、拟新增 `clip.rs`；`tekes-ui-render/src/{frame.rs,instance.rs,common.rs,quad_pass.rs,glyph_pass.rs,image_pass.rs,quad.wgsl,glyph.wgsl,image.wgsl,error.rs}`、拟新增共享 shader clip 片段；定向 scene/render tests。导出修改须主协调交接。

验收：统一normalize/contains/交与截面查询；Scene平衡/不支持/容量失败；所有原图元同chain；fractional scissor和重复AA定向GPU通过；无shape既有输出回归。只完成A不宣称Content/AX通过。

### B. 原 PanelContent 与原树几何、输入、reading/AX 接收

依赖：A精确冻结通过；主会话 advisor 批准Component/Content内部接缝及protocol/native影响；取得与#66/#71/#72共享文件的串行窗口。独占拟改：`tekes-ui-controls/src/composition.rs`、拟新增 `composition/panel.rs`，`composition/{content_reading.rs,content_reveal.rs}`；原 `feedback_view.rs`/`paint.rs`仅提取复用geometry所需最小helper（不改Theme）；`tekes-ui-platform-macos/src/{content_accessibility.rs,accessibility.rs,view.rs,shared.rs}`（Shared内原Window发布方法）；`tekes-ui-app-protocol/src/{component.rs,component/content.rs,component/content/accessibility.rs}`；`tekes-ui-module-runtime/src/{content_input.rs,content_accessibility.rs,native_content.rs,content_reading.rs,container.rs,container_semantics.rs}`；`tekes-ui-app-host/src/{accessibility.rs,columns.rs}`（两处原ProcessContainer是不同实现，均须核对）。相关controls/native/protocol测试同归B。crate前缀均为`crates/`。这是一组有限候选文件，不含其他runtime/窗口/工具代码；冻结后若调用图要求越界，先交主会话修订设计与票，不自动扩大writer范围。

B内部先本地B1、再远端B2串行，不增票：B1独占controls/platform/app-host本地投影，B2接手protocol/runtime及两处ProcessContainer接缝；B1不改远端字段，B2不并行重写B1。B1构造/挂载曲线Panel时递归核验子树能力，遇ProcessContainer、原生嵌入、不透明callback或非translation变换而未协商完整curve能力，立即返回UnsupportedChildCapability/UnsupportedTransform，不成功创建半可用组合；后续replace/mount也同检。不能只写local-only标签后继续接收不支持的child。

验收：单原槽/无第二Host；Panel geometry/theme/token引用与descendant focus边界；曲线覆盖所有普通/特殊paint遍历与原route/parts；AX原生和远端不静默Rect降级；原editor/IME API、reading事务/reveal原路径与负例通过。若ProcessContainer/self-framed接收无法在有限改动内闭合，B只可标local-only且C不得宣称#45全完成，主会话须决定追加独立工作，不扩大本票。

### C. 独立原消费者、GPU与公开交付接收

依赖：B精确冻结，真实平台接收能力明确；新revision发布由主会话另行授权。独占拟新增：`examples/panel-content-consumer/`（Cargo清单/锁与src，是否workspace成员由主协调管）；定向review程序/收据目录，现有gallery示例只在主协调指定互不冲突文件落地。不修改core/Theme/公共矩阵；发现共享bug返回A/B writer。

验收：第6节consumer/所有图元GPU/第二消费者/窄零嵌套self-framed用例逐项绑定同一manifest；原文本/按钮/真实编辑器/Scroll组合，默认样式变动无需consumer手写frame；原reading/选区/marked/焦点进出与无submit验证。精确公开可消费revision、示例和测试必须单独提供；#45原验收与新增风险观测逐项边界见第9.7节；不自动增加全库native/IME/VoiceOver/性能关闭前提。

## 8. 本阶段交付与保留项

已完成：#45全原文与评论读取（当时评论0）；原Frame/Surface/Split、Scene/Renderer、Content input/AX/reading/reveal协议源审计；本设计、源级反例、三个候选与依赖/独占范围。review目录保存原Issue输入、逐文件source hashes、几何算例和文档检查收据。没有新增API、执行Issue、生产实现、编译红例、Cargo/GPU/native/物理IME/VO/性能接收或PR。

#45回填仅标设计阶段，PR无；GET读回确认。主会话统一验收、问advisor和拆票。公开revision及原native限制保持原状，不因本设计完成而关单。

## 9. amendment-1：主协调六项审阅修正（设计，未执行）

本节与前文同步修正，为首版模糊表述的替代契约。首版文档原字节保存在 review 的 `amendment-v1-design-preserved.md`，SHA256 `c85c29d6e0c51bec71cae4e0d2552a28ca6d00ccb22048cb83d676d1ebaf9988`；原source-audit/design-checks/Issue读回收据不覆盖，不改标为本修订通过。本次仅补充只读调用点、文档/算术核验与新amendment收据，未重新冻结或接收现场生产WIP。

### 9.1 规范化 identity 与 AA/alpha 合成

Host安排时对有限f32逻辑rect/radii做规范化，负零统一为正零，有限负半径归零、半径clamp与原有限Quad规则相同；RoundedRect全零半径规范为Rect。translation只由同一安排结果应用一次。每个geometry generation内，完整规范元组 `(kind, rect, TL/TR/BR/BL)` 的位级相等才是几何等价；hash只加速，碰撞须比较完整元组。intern后分配ShapeKey，Scene/CPU/renderer显式携带该key；不同来源也只有完整等价才可共享key。重复引用key可去重；不同key即使视觉接近也不合并，不加epsilon、不在shader比较float。frame/backing/translation改变导致重新验证generation；同一key不能指代新几何。DeviceScale不纳入逻辑shape等价，但coverage只在同一render target/scale/origin下计算。

令每个不同祖先shape的coverage为 `c_i=clamp(0.5-d_i_px,0,1)`，`K=min_i(c_i)`（空链K=1）。这是解析交coverage约定，不是纹理颜色/真实alpha。glyph atlas和image像素的真实premultiplied内容乘K；不能因两张image/两个draw形状相同而去重绘制。真实图层继续原source-over：`out=A+B*(1-A.a)`。

对一个Quad，O为其规范outer轮廓coverage，I为原border内轮廓coverage（无border时取I=O；内域退化取I=0）。仅当Quad显式携带的规范outer ShapeKey与clip记录相等时，排除这些重复clip记录；其余祖先交得到K。定义 `U=min(O,K)`、`V=min(I,K)`、`R=max(U-V,0)`。设F/B为原fill/border的premultiplied颜色，公式为 `Q = B*R + F*(U-B.a*R)`；无border时Q=F*U。R是几何边带，B.a仍是真实透明度；填充仍在透明边框下方，不把border改成不透明cutout。不同轮廓的min合成是同一解析交约定，需交点GPU验收。

合成算例（纯算术，非GPU）：O=0.5、I=0.2、B.a=0.4、F.a=0.6，无其他clip时U=0.5、V=0.2、R=0.3，Q.a=0.12+0.6*(0.5-0.12)=0.348；重复相同outer key后仍0.348，而不是再次乘0.5。两个独立相同draw仍按真实alpha叠为0.348+0.348*(1-0.348)=0.574896，不合成一个draw。

Shadow不是outer coverage别名：blur/offset/spread和alpha产生的渐变S保留原算法，最终 `S_clipped=S*K_all`，不从阴影强度推回shape、不把Gaussian渐变与clip做min、不因与Panel轮廓相同而消除阴影。fill/border与Shadow各自按原painter顺序合成；例如shadow alpha0.2、clip0.5仍为0.1。零blur阴影也不自动取得Quad去重资格。测试须分别检查重合outer的半透明fill/border、独立重叠draw与真实阴影渐变，不能只测opaque背景。

### 9.2 三类权限与原 owner 生命周期

| 请求类别 | 权威与几何条件 | 全clip时的行为 |
| --- | --- | --- |
| 新pointer/wheel/hover/selection-start、AX点命中/Press | 当前generation的原Host目标/part、真实region contains；active/enabled/mount与原路由屏障 | 不产生新目标/激活；不以AABB或旧frame授权 |
| 已有owner的键盘/IME、合法SetSelection/ReplaceSelectedText、已开始的选区capture与reading事务 | 原mount/editor binding/focus epoch/part身份、范围合法性、原composition transfer barrier；几何可见性不取代这些权限 | 原合法owner继续；跨可见区选择可含离屏文本；保留marked、原reading anchor/版本，不因clip取消或重建。范围的文字操作不要求所有字符可见 |
| Tab或AX Focus请求建立/转移owner | 原focus规则和IME barrier先允许，再以当前generation做只读reveal可行性计划，提交后由原Host转移 | 可reveal才转移；无解/过期不新建焦点，也不提交/取消现有editor |

第二类不是任意后台节点的编辑许可：仍须满足现有API本来的owner/binding/只读/disabled/范围检查，旧AX对象或错part不能借“已有”绕过校验。selection point查询/新拖拽起点属第一类，已有合法selection range编辑属第二类。capture向曲线外拖拽仍交原Move/Up/Cancel，末次激活按当前点检查。读屏的文本值/合法文本范围可跨离屏内容，几何查询另要求同代；不能把reading owner的持续存在误作新命中许可。

### 9.3 平台点查询、同代parts/text与离屏结构

增加的是原桥接路径，不是每个Panel自己的AX树：原ContentHost安排后生成不可变GeometryQuerySnapshot，含host/mount/part映射、shape链、布局geometry generation与原text binding/revision。原平台Shared/Window以一次原子发布安装nodes、text layouts及此查询入口（现分开的set_accessibility_nodes/set_accessibility_text_layouts不足以保证同代）。原生view/element的accessibilityHitTest把screen point按当前window/view转换到host logical，然后调用同一个原Host提供的只读查询入口，结果为原AccessibilityKey与generation。原Host的query实现复用route的几何/层序/part逻辑；平台不自行遍历AABB挑目标、不新建可变Host、不在AX回调重入input/render。若只能跨线程，应读取已安装的原Host不可变快照；无当前快照同步返回无目标，不延迟返回旧命中。

同次查询校验generation、mount、part exposure及text binding/revision。text range/caret/line几何与parts须来自同代快照，range几何返回真实region交的bounding frame，仅用来描述位置，不赋予矩形点命中。旧generation的点/位置查询拒绝并要求重新查询；现有owner的SetSelection等仍可在通过原文字身份/范围屏障后执行，不把它们错误绑定旧坐标。布局变化先使旧几何失效，重新安排/发布后才恢复点查询；paint提交和AX不能混用新frame与旧clip。原屏幕坐标/scale/origin变更同样换代。

离屏结构必须显式改造当前平台模型：AccessibilityNode目前无offscreen字段，validator要求正面积。候选新增原平台内部发布元数据 `geometry=None | Visible { bounds, query_key, generation }` 与 `structural_only`，而不是假装系统已有通用offscreen属性。完全clip但需父子闭包的原祖先/当前editor保留原key和parent；几何None时accessibilityFrame返回NSRect::ZERO、点查询不返回它。validator仅对显式None允许零frame，拒绝任何“Visible却零/非法”的记录；父子/环校验保留。structural_only的被动祖先通过原accessibilityChildren维持后代连接，并用原element的isAccessibilityElement实现仅结构投影；已有editor仍保留文字与selection接口和原key，不降成无文本祖先。Visible祖先的bounds可用真实可见后代并集的包围盒，但不由此获得交互命中。

这些是需要在原accessibility.rs/shared.rs/view.rs接入并验证的具体设计，未证明AppKit/VoiceOver对零frame结构和离屏editor遍历可接受。若平台无法保留该结构，报告该#45 consumer的native AX维度未接收，回主协调评议；不得虚构正面积frame、重挂父级、丢editor key或用全库VO通过替代。

### 9.4 reveal：先可行性计划，再原事务提交

沿原ScrollArea::reveal_target（只读目标计算）扩展纯计划器，读取布局generation、目标mount/reading version与各scroll当前offset/limits，不调用arrange/restore/set_offset去试探live状态。计划持有数值/geometry约束和原owner引用身份，不克隆可变ContentHost/editor。目标若依赖会因宽度重排而变化的自定义布局，需其原纯layout-query支持；不能纯预测时明确NeedsLayoutPlan/UnsupportedLayout，不偷偷在live树试排。

垂直reveal以目标交叉轴完整范围 `[x_min-margin.left,x_max+margin.right]` 求每个祖先shape允许的共同y区间，再按top/bottom margin与目标高度求合法offset集合并交scroll limits；水平同理。margin使用原请求Insets合法性/非负校验，不自动加产品值。Start(Vertical)保持x，取合法safe-start对应offset；Nearest在可行集合中取与当前offset距离最小值，合法则不动，平局遵循原trailing选择。超大目标沿原Nearest“已跨满可见段则保留当前offset”的政策，返回clamped/partial；不能把任意角落一丝交当完整可见。Centered如本批未实现同一可行约束则拒绝，不默默当Nearest。

数值例仅为测试：圆角viewport=(0,0,100,100)、r=20；目标x=[2,8]、左右margin=2，expanded交叉轴=[0,10]，共同允许y=[20,80]。上下margin=2后目标本体安全y=[22,78]。文档目标y=[50,60]、垂直offset limit=[0,100]时，完整可见offset集合为[0,28]：Start选28（目标落22..32），Nearest当前40选28、当前10不动。若expanded x=[-1,10]且只许垂直滚动，无论y offset多少都无解，返回NoFeasibleRegion；不改x或放宽clip。

同一viewport/safe区间下目标y=[50,130]，高度80大于safe高度56：Start仍可落起点22（offset28），报告clamped=true、partial=true，实际可见本体22..78。Nearest当前40时目标10..90已覆盖整个safe区间，保留offset40并报告partial/clamped；当前0不跨满safe区间，比较对齐起/末的offset28/52，取28。若scroll limit缩至[0,10]，原limit clamp也计入结果，不把受限Start声称精确对齐。返回中区分requested placement、applied placement、partial及limit/shape clamp原因，避免无解和过大目标混为成功。

嵌套计划按原内→外owner链建立有限约束；translation/凸域下每个目标截面给闭区间，先解并检查所有祖先，再提交。只有各owner/mount/reading version/geometry generation和用户输入epoch仍相等，才在原事务中一次应用计划并安排/发布新几何。用户在求解期间滚动/编辑即StalePlan，不用回滚用户状态。依赖布局无法一次闭合时，用不可变数值计划refinement，记录完整计划状态与严格缩小的区间；重复状态/无严格进展立即NoConvergence。求解budget来自显式接收的plan工作量/设备资源限制；耗尽返回PlanBudgetExceeded，不“最后一次offset成功”。NoFeasibleRegion、InvalidMargins、StalePlan、NumericallyIndeterminate、UnsupportedTransform/Layout均无live提交。错误名是设计候选，公开兼容方式仍待主协调。

### 9.5 远端入口、图像边界与几何版本

能力入口在原ViewCapabilities协商，不从“已有image/primary_pointer/accessibility=true”推断curve：候选要求curve_clip_geometry版本、支持axis-aligned+translation、统一AA/backing/origin、点/AX查询和geometry lease均被明确接收。协商缺失/旧peer拒绝本组合；在原ProcessContainer构造/挂载与allocation更新前检测，不生成只有矩形的成功Panel。B1本地阶段也明确拒绝未协商的ProcessContainer/不透明原生child，不能先挂上再关闭部分输入。B2在同一B候选内串行接收远端，不另开并行core writer。

远端子进程仍是其原ContentHost，父进程仍是原容器宿主，不增加Host。父Host拥有父级最终几何generation Gp与host逻辑chain，把分配translation t、backing及物理origin和转换后的child-logical外部chain发送给原child。child安排后回Gc、child mount/part/text generation及外部chain acknowledgement；有效lease绑定(session,view,viewport,Gp,Gc,allocation,scale/origin,chain identity)。内部shape保持child坐标；父投影AX时只加t一次，point发child时只减t一次，不从frame尺寸猜比例。

**谁裁图像**：父Scene/Renderer在原远端ImageQuad合成处应用外部祖先chain，是最终可见像素裁剪权威；child用已ack的外部chain做input/AX/reveal约束，输出图像只烘焙自己的内部clip，不再次烘焙同一外部clip，避免父子重复AA。若内部Quad/clip与外部边界规范等价，B2必须协商原renderer的boundary-deferred标记：该ShapeKey的outer AA延后到父合成，child按第9.1节保留inner/border/真实alpha信息所需的原合成表示；不能只“除alpha”或把真实图像alpha当coverage。若当前扁平图像通道不能无损表达该表示，则这类重合边界组合明确UnsupportedFlattenedBoundary，不能降级近似。非重合内部绘制的真实alpha仍与最终外部coverage相乘。该限制需要C的self-framed消费者明确披露；消除此限制须另行审议，不宣称远端任意子树已完成。

**谁验输入/AX**：父以Gp在原Host先验外部contains与原容器lease，新点才转发；child以相同ack外部chain+Gc及自己的内部chain在原Host验目标/part，不能信父传来的“命中成功”。已有capture/编辑owner按第9.2节原lease而非新点资格继续；AXFocus由原child的reveal计划提出、父级嵌套计划共同可行后才提交。native AX点入口仍回原Host查询，跨进程若当前lease无同步可用查询结果则无目标/明确需刷新，不返回旧AABB。image frame必须携带相同lease；父拒绝过期frame/semantic/text几何，不把新图像和旧Gc数据拼接。

wire验证双方都做：记录数/深度/字节数经协商且不超过设备与协议资源限制；每个索引在界内，parent严格先于child（形成有界DAG，拒环/自指），head有效，声明depth与实际走链一致；拒NaN/Inf、负size、溢出边、未规范radii、非法translation/scale/origin、未知kind/transform、身份复用与generation倒退。空region可合法表达但不授新命中；不能截链或默认full。session/view重启、mount替换、allocation/scale/布局失效立即撤lease；所有迟到pointer/AX/parts/text-range/image查询拒绝过期。错误返回unsupported/stale/malformed，旧peer无协商绝不进入“成功但Rect”的路径。

### 9.6 pixel中心、支持域、凸域算法与终止契约

唯一坐标换算由render target记录的scale s>0和逻辑原点o确定：fragment物理pixel中心p_px（栅格builtin position，像素索引i对应i+0.5）对应 `p_logical=o+p_px/s`；逻辑signed distance换设备单位 `d_px=s*d_logical`。origin先按同一target协议处理，不能再加一次0.5。原quad shader的coverage `clamp(0.5-d_px,0,1)` 定义AA总宽1物理pixel、向外支持半宽0.5；0.5来自这份已有公式，不新增Panel魔数。Scene CPU与三shader共享此版本化AA约定，physical backing与text scale不互相替代。

新shape scope的保守scissor先取每个域的logical AABB并换到target物理坐标，再向外扩该AA外侧支持半宽（当前由公式推导为0.5物理pixel），floor(min)/ceil(max)并截target。primitive顶点仍沿原AA_PAD_PX支持，不因scissor缩减；可取更大保守域但不能把支持截掉。shape内的Rect祖先也用同一解析距离/coverage/支持域，不能再插ceil(min)/floor(max)的内缩legacy scissor。无shape旧Scene输出仍单独回归。CPU命中是逻辑几何，不拿像素量化AABB作命中，也不把AA fringe扩成点权限。

CPU不能只写“凸域交即可”：有限圆角矩形是直线段与四段圆弧构成的凸域。候选算法枚举各边界端点、圆弧轴向极值、line-line/line-circle/circle-circle交点，保留同时满足全部域约束的候选；截面用解析区间逐域取交，极值用于交集bounds，containment/某域完全包含于其他域需额外检查，不能只靠边界交点存在。纯切线/零面积与正面积微小lens分别分类；行动可见区域要求正面积，边界点contains仍按原闭域规则。全退化链明确为空，不因r=0除零。计算用至少f64中间量与区间误差界，必要时有限refinement；无法在声明预算内判定即NumericallyIndeterminate，不用扩大epsilon接受不可见hit，也不把不确定默认为无clip。

A的前置验证契约：两圆相切、相切两侧相邻可表示坐标的微小透镜/分离、同心/重合弧、不同四角、全包含无交点、Rect角AABB假阳性、嵌套零宽高、translation后非整数origin、巨大有限值与overflow拒绝；验证截面端点/交bounds/contains与同一个规范域一致。必须用独立高精度或区间oracle和CPU/GPU边界对照验证，不用实现镜像公式自证，也不用多矩形采样近似。此为必要设计测试，当前均未执行；若算法复杂度超出A有限接收能力，先停在设计/明确不支持，不能先发Panel API掩盖。

### 9.7 C对照原#45验收与未验范围

| #45原验收 | C对应接收与本轮状态 |
| --- | --- |
| 独立公开consumer含文本、按钮、真实editor与ScrollContent；原children/身份/事件/AX层级 | 指定这棵consumer树和第二public API消费者，检查原mount/binding/AX key；本轮未实现未运行 |
| Canvas/Panel原token、默认样式变动不需consumer改画法、产品外inset仍外供 | 原painter/Theme输入与几何权威接收；不要求全库Theme/native回归；本轮仅设计 |
| 深浅主题、高DPI、窄/零尺寸、嵌套clip、曲线paint/hit/AX与滚动reveal | 本consumer逐图元GPU、原Host点路由、parts/text与平台查询；API与GPU/native结果分开；本轮均未验 |
| 焦点进入/退出、中文组字、流式/分页更新保留原editor/reading事务 | 本consumer程序化preedit/selection/reading与真实中文输入法/candidate UI分别记录；后者未验就保留该原验收项，不能用API代替，也不扩为全库所有IME |
| 普通框架与self-framed组合 | 指定普通/Split/原self-framed用例；local-only拒绝规则和远端flattened-boundary限制逐项披露，未接收的组合不勾选 |
| 精确公开消费revision、示例和测试 | 主协调授权发布后固定pin与收据；本轮无revision/PR/建票 |

真实窗口/原生AX与VoiceOver是对本#45消费者“AX层级/曲线边界”结论的验证层，未跑则不宣称其通过；不把全库所有native/VO或完整产品验收加成#45新关闭条件。性能并非#45原文验收，当前仅记录未测，不新增性能关闭门槛。主协调按原票范围验收和拆票，本设计/QA元数据修订不触发实施。


## 2026-10-24 Phase 1 key-v2 主协调接收补充

主协调接收有限 CPU geometry 候选以继续隔离 Phase 2；尚未接入 live，也未接受 Scene/Renderer/GPU/Panel 完整能力。证据：design/reviews/panel-shared-clip-implementation-20261024/root-key-v2-intake.json。原 Phase 1 失败候选保留，表身份契约由以下修订取代。

# Proposed amendment delta for Phase 1 — coordinator acceptance pending

This candidate supplements amendment-1 sections 9.1/9.6. It does not change live design documents, Theme, controls, renderer, or public inventory. The initial Phase 1 source/receipt is preserved and its ShapeKey contract is superseded here.

1. **Table identity is authority, generation alone is insufficient.** Every `ShapeTable::new` creates a fresh private `Arc<()>` allocation. Each `ShapeKey` retains that Arc; equality and hashing explicitly use allocation pointer identity plus generation/index. They do not use Arc value equality. Thus equal generation/index/geometry in different tables cannot authorize access, and the retained Arc prevents allocator address reuse while an old key exists. `get` validates owner identity before generation/index. Keys are Clone, no longer Copy. Mutable ShapeTable is not Clone; compile-fail coverage forbids divergent inventories sharing one identity. Full normalized tuple equality still deduplicates within the same table. Generation continues to describe the owner's arrangement version, but callers are no longer required to provide globally unique u64 values for key safety. This identity is process-local; no wire/serialization authority is implied.

2. **Finite numeric implementation limits are an additional supported subset.** Normalized nonzero f32 tuple components have absolute magnitude in [2^-60,2^60] and max/min absolute magnitude ratio <=2^24. Empty shapes, f32 edge overflow, invalid/nonfinite inputs and query coordinate checks retain the initial candidate's documented behavior. Nonempty inset inputs additionally satisfy the span check; any finite over-thick border produces empty inner geometry first. Identical dimensions may be supported near the origin and rejected after translation: Rect(0,0,4,4) is supported, but translation (2^27,0) returns `UnsupportedNumericRange`. This is not translation-invariant support. It is never repaired by clamping a coordinate, broadening a hit, or dropping a clip. Phase 2 must propagate both `UnsupportedNumericRange` and `NumericallyIndeterminate` through validation before any draw submission, including when a valid existing scope becomes unsupported after translation.

3. **Verification scope is finite.** The independent 90-digit Decimal solver's 60 intersections, 300 sections and 480 points are an algorithmically independent finite cross-check. They are not a formal proof of all input values or all outward interval operations. Error paths remain explicit. The exact normalized f32 identity, f64 logical predicates and f32 GPU AA approximation are distinct contracts, without a bit-equivalence claim.

4. **The current budget is per query.** Input count and checked polynomial geometric operation reservation bound one ClipRegion query. They do not bound total per-frame shape chains, instances, repeated queries, upload bytes or shader work. Phase 2 must add accumulated frame/resource budgets and reject the complete frame before submission, rather than relying on every individual query being under budget. No pre-draw gate exists in Phase 1.

5. **Inner geometry does not accept AA/alpha.** The conservative inner subset and explicit empty policy are retained. GPU U/V bounds, ShapeKey-based duplicate-AA exclusion, transparent fill/border compositing, real layer alpha and Shadow gradient behavior still require Phase 2 implementation/readback. Neither the subset check nor V=min(I,U) can replace those tests.


## 2026-10-24 Phase 2 主协调接收与实施边界

# #80 Phase 2 契约修订（主协调已接收）

This supplements the accepted CPU key-v2 and amendment-1. It is a proposal under this review directory; live design, Theme and public inventories are not changed here.

- `Scene::with_clip_shapes(&ShapeTable)` captures an immutable registry. The snapshot retains the same private owner identity and copied slot inventory. It cannot intern or change existing slots; therefore cloning Scene is safe while cloning a mutable ShapeTable remains forbidden. Foreign, reconstructed-table and post-snapshot keys reject. No caller-global u64 uniqueness is required. Scene's required clip geometry version is explicit; unknown versions reject, including declarations without any draw. Renderer reports capability version 1. Future protocol/native capability negotiation remains B's responsibility.
- Add `PushShapeClip(ShapeKey)` and explicitly keyed Quad primitives. A keyed Quad's normalized outer tuple must match its key. GPU records use dense u32 identities assigned only after owner validation and exact full-tuple equality; no shader float comparison or float-encoded identity exists. Existing raw Rect clips may join the frame's exact tuple interner. Repeated identities in a chain are removed; independent draw instances are preserved.
- Analytic mode is **scoped**, not an automatic change to every draw in a capability-bearing frame. Raw Rect branches outside shape scopes keep legacy scissor/foreground behavior. Both their legacy scissor and conservative AA support are retained: entering a descendant shape scope promotes all actual Rect ancestors, and leaving restores the legacy parent. An explicitly keyed draw can promote the same ancestors for that draw alone. Bind-state changes split batches even between adjacent keyed/ordinary draws. GPU readback includes a before/after regression for unrelated Rect siblings and a direct keyed-to-ordinary switch.
- Keep the original three instanced passes and painter order. Group 0 shares a dynamic uniform header plus readonly bounded-DAG records; all three shaders include the same clip WGSL. Slot zero is legacy mode; slot one is an empty analytic chain; subsequent slots correspond to validated nonzero heads. Primitive buffers retain their original payload except exact integer Quad metadata, increasing the private QuadInstance stride from 112 to 128 bytes. No stencil, R8 fallback, product mask, second Host or window is introduced.
- Current render target APIs support logical origin `(0,0)` and positive uniform scale. Fragment builtin position already equals the physical pixel center. The mathematical conversion is `p_logical=origin+p_px/scale`, with `d_px=scale*d_logical`; the shared SDF evaluates the equivalent physical geometry. There is no extra 0.5 coordinate shift. Coverage is `clamp(0.5-d_px,0,1)`, with 0.5 physical pixel outward support. Scissors use outward floor/ceil of that support, including Rect ancestors. Nonzero render-target origin and transformed clip domains are not exposed by this API; fractional shape origins and rotated image draws are tested.
- CPU logical predicates stay f64/interval; GPU AA stays a measured f32 approximation. In addition to key-v2's normalized CPU range/span, nonzero shape/draw geometric scalar products with target scale must have absolute magnitude in [2^-60,2^60], preventing squared f32 distance overflow/underflow in the supported path. CPU-supported inputs can therefore reject at another backing scale. Capability frames require finite unit-range RGBA, finite image opacity, finite UV/rotation/positions and valid glyph atlas regions; opacity keeps its existing finite clamp. Errors propagate before uploads/submission, never as AABB or unclipped success. Legacy scenes retain their original numeric/output path.
- Inner geometry is separately constructed/validated by the accepted CPU helper. The explicit empty-inner flag forces I=0. A finite over-thick border is encoded as half the short side once the CPU inner domain is empty, because further width cannot change the specified Q; this avoids uploading an overflow-prone width and is not a geometric relaxation. Zero outer domains produce O=0 without an AA fringe. On supported analytic draws use U=min(O,K), V=min(I,K,U), R=max(U-V,0), Q=B*R+F*(U-B.a*R). The V<=U clamp constrains the f32 approximation and does not replace inner subset geometry. Texture/glyph real alpha and Shadow gradients multiply the full chain; only an explicitly keyed Quad excludes its matching outer identity. Independent instances still source-over.
- A closed CPU tangent has no positive logical area, yet intersecting analytic AA supports can cover a pixel. Logical `visible_bounds=None` is not an inward GPU cull. Empty shapes, conversely, have zero coverage. Both are verified by actual readback.
- Complete CPU preflight precedes atlas sync, queue writes, callbacks, encoder creation/submission in all surface/offscreen/external-target entry paths. It validates capability, owner/key/tuple, balance, canonical records, indices, strictly preceding parents, depth, duplicate/invalid identity and numeric query results. It performs no trial rendering. Callbacks may still independently fail or violate their pre-existing GPU API contract; device faults and arbitrary producer misuse are outside this clip validator's guarantee.
- Budgets are per complete frame: primitives, registry/canonical shapes, unique records, all instances, actual CPU queries and cumulative reserved geometric work, conservative full-target fragment chain visits, and uploads. Upload reservation includes dynamic uniform slots, record bytes, instance data and a full-atlas bound even when dirty/cache writes are smaller. Shader work counts clip-record visits; it does not claim to bound the pre-existing Gaussian shader or arbitrary producer work. Shadow traverses the clip chain once. Device storage/buffer/uniform-offset/target capacities independently reject before upload. Instance buffer growth is clamped to device capacity after required bytes validate. Default CPU and shader clip-work policy is 100,000,000 units each; memory/input/record/depth capacities derive from device limits and the explicit 128 MiB upload policy. Caller changes cannot raise device limits. These operation budgets are not wall-clock/performance acceptance.

All API/error/inventory additions remain candidates for coordinator serial integration. In particular new RenderError variants affect exhaustive downstream matches; no published compatibility/revision acceptance is implied. Panel, Content, AX, reading, wire/remote and consumer work remains #81/#82 and is not started here.


上述候选措辞保留来源；本节与顶部主协调接收状态为准。数值支持域同时受scale限制，有限GPU corpus与≤2byte容差不代表全域证明。预检原子性只覆盖clip validator的拒绝，不覆盖任意producer/device故障。全atlas/全target保守预算可能拒绝较轻场景，不是性能验收。正式API评审库存见 design/reviews/panel-shared-clip-implementation-20261024/phase2/root-integration/public-api-review.json：新增scene九个geometry support类型、Scene方法/Primitive变体、ClipFrameBudget/版本能力及RenderError四变体。现有公开库存仅审controls类型，不把这些基础类型错记成新UI组件。RenderError变体对外部穷举match有源码兼容影响；本地消费编译绿不代表公开发行兼容。

主协调另修复测试集成边界：输出不依赖QA祖先目录，legacy无条件include_bytes比较10份原renderer固定黄金bytes；原两文件及候选结果保留。root测试与严格检查在原锁内现场留存11实际程序及depinfo，不能反向补称原候选depinfo已被封存。原gpu_images在此feature组合0pass/3ignored，不称可选IOSurface功能验收。


## Amendment 2: B1 shared prerequisites (2026-10-24)

Main/advisor source audit, not implementation acceptance: design/reviews/panel-content-implementation-20261024/feasibility-plan.json.

S1: Scene owns one unified resolver inventory retaining immutable source Arc owner/generation/slot identities. Add bounded fallible fragment append; Host stages a fragment after layout and atomically appends it without replacing caller primitives, order, clip state or existing keys. Explicitly import immutable sources; same-owner snapshots may extend only when all existing slots match. Unimported foreign keys remain rejected. Preflight capabilities, every key, balanced fragment and cumulative budgets before any caller mutation. Dialog/Popover and retained OverlayPresence replay use this same composition seam; unsupported primitives return an explicit failure before partial output. Translation must transform both immutable geometry and keyed Quad/clip via one shared normalization helper; unsupported scale/opacity combinations fail explicitly. No second mutable geometry authority, key reassignment or relabeling of #80 receipts.

R1: Extract original Host readonly reveal plan/commit. Resolve target mount, geometry generation, ancestor ScrollContent, reading version, input epoch and offset/limits without calling mutating Component::reveal. Reuse original rectangle calculations plus ClipRegion cross-section constraints. Revalidate identities and original IME transfer barrier before changing focus or scroll; commit has no ordinary fallible tail. Infeasible/stale plans preserve owner, marked text and offsets. Initially support original ScrollContent translation and predictable existing layout; width-reflow/custom reveal without pure-query capability is explicitly unsupported. No Host cloning or trial-scroll rollback. #85 reader data-update transaction does not replace focus/reveal planning.

S1 and R1 are independent candidate prerequisites, integrated serially by main, then consumed by #81 B1. B1 retains Panel geometry/input/AX identity and atomic native projection responsibilities. Add conservative curve-subtree capability to original Component: unknown opaque components are not automatically authorized; known local containers validate recursively. ProcessContainer rejects local-only curve subtrees before construct/mount/replace/arrange side effects without new wire fields or claiming B2 support. B2 leases/remote composition/UnsupportedFlattenedBoundary and #82 remain pending. Finite prerequisites require public consumers, directed negatives and actual program/input identity; native/physical IME/VO/performance/release remain separately reported.


### Amendment2 S1 compatibility decision

A legal unsupported/budget/foreign-key rejection must not become expect/panic. Add limited try_paint/try_render paths for retained overlay composition; new fallible callers propagate Result. Preserve old infallible signatures with a Scene first-composition-error slot: a failed legacy composition leaves existing primitives/registry unchanged and makes the whole Scene unsubmitable. Renderer checks this error at the start of every original whole-frame preflight, before callbacks/upload/draw. clear resets the error, clone retains it, appending an errored fragment rejects it, and callers may query it. Returned legacy layout does not prove successful rendering. Stage only necessary OverlayPresence retained/context/progress before append, commit them after success, so legal rejection preserves those states as well. No whole-Host cloning or weak foreign-key guard. This is the accepted design for #97, not accepted implementation.


### Amendment2 R1 compatibility decision (2026-10-24)

Readonly reveal planning must preserve the original ordinary rectangular focus/restore contract. After the existing mount, active/enabled, focus-scope and IME transfer gates succeed, a legacy Rect target without a scroll/curve reveal requirement may receive a logical-focus-only plan, including pre-layout restore or an offscreen target for which the old Host did not promise a scroll. Such success does not claim that the target was revealed or visible and must not manufacture geometry, offsets or a reading update. The original composition_focus_restore tests remain unchanged.

Explicit reveal, a target requiring original scroll reveal, and Panel/curve focus transfers remain strict: require a feasible readonly plan and current geometry/generation, or fail without changing owner, marked/selection, offsets, reading or queues. Determine curve requirements from structure/declaration, not merely an available geometry cache; stale or absent curve geometry cannot downgrade the request to legacy Rect. Identity, disabled/inactive and IME errors always reject. Do not implement a generic Err-to-allow fallback or call mutating reveal as a probe. This clarifies the applicable domain of the earlier unconditional reveal wording; it does not relax Panel/curve acceptance or claim #98 implementation acceptance.


## Main serial S1 source-intake record (2026-10-24; WIP, not acceptance)

The current source exports `FragmentBudget`, `FragmentError`, and `FragmentTransform` from `tekes_ui_scene`, and the new fragment methods on `Scene`; Controls and Renderer expose `CompositionError::SceneFragment(FragmentError)` and `RenderError::SceneComposition(FragmentError)`. These paths belong to the Scene/Renderer crate surface. They are intentionally not inserted into `design/gallery/public-api-surface.json`, whose generator audits reachable `tekes_ui_controls` public type paths only. The controls inventory refresh is tracked separately in the notice sync draft.

This records source intake for Amendment 2 S1 only. It does not replace or revise the existing Amendment 2 R1 readonly-reveal decision, does not claim Panel/B1 integration, and does not close Issue #97; the latest issue readback remains OPEN. No published downstream compatibility or acceptance is implied.


## Main S1 implementation acceptance (2026-10-24)

Issue #97 original five conditions are now accepted against the serial #97+#79 source union. Root evidence: `design/reviews/scene-notice-main-integration-20261024/`. Scene/Renderer/Controls/public-consumer/overlay/strict/downstream checks passed. Actual GPU covers five nested backing scales, ten mixed composition pairs,201 shape cases and ten legacy fixtures byte-identical to the original #80 baseline. The half-ULP source/canonical derivation retains original Arc owner/generation/slot authority and stepwise f32 normalization; foreign imports and unsupported/budget/stack errors reject before append commits. Preserve original #80 receipts.

This accepts Amendment2 S1 only. R1/#98 and Panel B1/#81 are separate and not accepted by this result; the original R1 compatibility decision above remains unchanged. Native/physical IME/VoiceOver/performance/public release remain unverified. FragmentError is the actual public Scene error export, separate from the323 controls-only type inventory.


## Main R1 action preparation decision — 2026-10-24 revision4

Current main source is NOT accepted for #98. Original #95 pointer test is red; revision4 reproduces direct-focus regression and pointer/root AX/part AX effects after strict focus refusal. Raw evidence: design/reviews/host-reveal-plan-20261024/revision4/handoff-blocker/. Do not change original assertions or delete old receipts.

Main accepts advisor's finite original-owner preparation seam. Keep public focus(id, visible) changed-bool signature and source-independent geometry planner. Private FocusOrigin comes only from original direct/pointer/traversal/AX entrypoints: ordinary/direct retain original blur/unmark transfer; Tab/AX keep original marked gate, same-owner is not a transfer rejection. Structured admission distinguishes Rejected from Accepted{changed:false}; root AX and padding must not act after rejection. All ordinary scope, exact mount/routes/input/layout/reader/offset/counter checks precede first effect, with no reveal error downgraded to logical fallback.

For response-dependent pointer bubbling and virtual AX parts, add an optional original Component preparation contract, default Unsupported. Pure preparation authenticates current event/part/token/action and fixes consumed/bubble, focus/preserve and capture intent. The single-call immutable preparation binds original owner and relevant state revisions; final pure owner validation and PreparedFocus validation precede any effect. Commit applies the original reducer's already-decided effect and has no ordinary refusal/recomputed routing. Extract original validation/decision and application shared by old handlers and new prepared entry; do not replay a mutable handler then compare its result, pre-focus every semantic part, prequery then mutate without final validation, or rollback effects.

Prepare the actual topmost-to-ancestor bubble chain under original finish_bubble rules. Existing captured Move/Up/Cancel do not become new cross-owner focus transactions. Preserve-focus auxiliary actions are explicit. Original List/Header/Row token and action-kind checks happen in owner preparation; exposed Table Column/Cell do not imply Focus permission. Root Press must distinguish accepted-no-change. No synthetic identities, parallel selection/input engine or global ticket registry.

This is an additive strict-transaction capability, not a new restriction on every ordinary Host/custom component. Default Unsupported is never assumed success. Ordinary paths with no strict fallible focus/reveal requirement retain original behavior. Strict atomic action+focus paths cannot invoke unknown mutable handlers before admission: they refuse without callbacks. Builtins actually participating (Button/editor, List/Table, actual bubbling owners) and compliant external custom owner use the same public contract, no type whitelist. A prepared owner must ensure original blur/unmark/hooks cannot invalidate its decided effect into a late ordinary failure; report precise unsupported owner sequences rather than pretending generic hook atomicity.

Minimum red-to-green scope: unchanged #95 and original focus/restore; ordinary direct focus; Tab/AX marked rejection; pointer/root/part strict-refusal with owner/marked/selection/queue/capture/sort/reading/offset unchanged; stale part and nonfocusable Cell; same-owner accepted-no-change; preserve-focus auxiliary action; real bubbling and external prepared owner; unknown strict no-callback refusal while ordinary old path works; counter/stale-ticket/geometry failure before first effect. Then run original full #98 targeted regression/strict. This does not make #81 Panel or #89 adapter accepted. Original five criteria remain unchanged; finite source/runtime/native evidence levels stay distinct.


## Main R1 #98 finite implementation acceptance — 2026-10-24

Main has accepted original#98 five criteria on current main source, following the revision4 action-preparation decision above. Receipt `design/reviews/host98-main-intake-20261024/root-acceptance.json` binds the original19 r5 hunks plus finite public classification/binding, preserving the current#115 layout in collections_view. Two candidate EOF blank lines were removed; no reducer semantics changed. Original core/Composer/reading/focus-restore, two independent consumer packages pass433 test invocations. Original strict checks for controls lib/tests and both finite consumer sets pass; default and actual svg-icons public bindings pass3 each, covering357 classified public paths. The six new types under composition are shared preparation support with no component identities/routes; Component optional preparation methods preserve existing signatures and default to Unsupported for strict requests.

The prepared decision contract requires original owner-local state and stable no-reentry mounted-tree hooks across focus and preceding bubble effects. It consumes original decided reducers with unit-return apply and final pure validation. This is a semantic opt-in capability, not runtime isolation of arbitrary malicious/custom callbacks. Ordinary direct/pointer blur/unmark and logical Rect focus remain; Tab/AX transfer marked barriers and strict Scroll/curve feasibility remain. Unknown strict owners/custom reflow remain unsupported. Same-owner editing keeps original input authority. Original red-to-green tests and prior receipts remain unchanged.

Whole accessibility entry is not claimed pure before its first effect: original input-clock invalidation/reconciliation still precede the prepared AX seam. #111 authorization must independently guard projected entry before those effects. This acceptance does not close Panel#81, Select#89, remote#111 or all strict controls. Full capability inventory currently reports stale bounded observations after#115/#112/source progression; the normal reverse classification/original binding producer and actual357 compiler bindings are verified here, while normal capability freshness/full52 closure remain#8 and no old18 identity is re-signed. Initial missing explicit request lifetime binding and wrong svg feature invocation remain recorded; existing explicit type_arguments fixes the former, actual svg-icons run proves the latter. No GPU/native/physicalIME/VoiceOver/performance/publication claim.


## Main R1 original List activation follow-up — 2026-10-25

The original AppHost once-queue test exposed a real#98 extraction regression during#116 integration: first AX List Press changed selection but returned Changed instead of original Enter Activated. #98 originalAC4 was reopened; only shared List prepare_list_part event decision is corrected so everyPress returns Activated, Focus remains Changed. Table original Enter deliberately returns Changed for first selection and Activated for repeat, so its decision is unchanged. Original assertion is untouched. Main current originalAppHost1+Listpublic4+allfivepackages527+related strict pass. Receipt `design/reviews/host98-main-intake-20261024/list-activation-followup-acceptance-20261025.json`; old433/strict/357 receipts retain original input window. This does not broaden the prepared capability or accept Panel/remote geometry.


## Main B1 window publication completion decision — 2026-10-25

The sealed B1 candidate is partial; its single-Host atomic snapshot is not sufficient for the actual AppHost toolbar+multiple local Hosts path. Main authorizes the minimal owning Window/window_columns/shared/view implementation window below as original B1 completion, not a new framework or a new acceptance prerequisite. No live B1 acceptance or B2/remote authority is implied.

Build one immutable window publication from original per-Host snapshots/fragments. Retain every original geometry lease, key-to-original-owner mapping and existing hit/paint order, nodes and text layouts; never merge or re-sign independent Host generations. Current means every participating lease remains current. A legacy Rect fragment must be explicit original window-owned data with publication lifetime, never a fallback for missing curve provenance. Collect actual toolbar/modal scope: open toolbar publishes only its original scope, not background panels.

Prepare and validate all fragments/key uniqueness/legal parent closure/text ownership/budgets/current leases before any window store mutation. Setter performs final full lease validation, then installs one complete publication and only then sends native notifications, without RefCell borrow held across callbacks. Failure rejects any successful prefix and revokes the entire old publication action/geometry authority; returning false while retaining old cachedAX authority is insufficient. SingleHost setter delegates this same commit. Existing separate nodes/layout setters must not erase newly installed geometry authority; use one common publication authority even if compatibility projections remain.

All point/range/frame/text queries and native action routing validate the whole publication and route by its exact key-to-fragment/Host ownership. A stale source disables the batch; no bbox/oldlayout fallback, and no loop trying different Hosts until consumed. Next lawful complete publication may recover without resurrecting old objects. Minimal shared file scope: platform content_accessibility.rs,shared.rs,view.rs and actual query/action helper sites; app-host window_columns.rs and window.rs. Keep shape/identity/IME/contracts with original owners and #111 projected authorization independent.

Minimum actual tests: toolbar+two localHosts with one curved/text tree; duplicate key/illegal parent/one stale source reject whole batch and invalidate oldAX; lawful next publication recovery; point/range/text/actions go only to correct original owner; toolbar scope excludes background. Programmatic/native-object contracts suffice; shownwindow/physicalIME/VoiceOver are not added gates. Preserve all earlier candidate/failed/source windows.

The recorded runtime Table compile blocker must first be resolved as an input-identity discrepancy: current source, frozen baseline and candidate access.rs share bytes with no Table variants, while oldE0004 claims five variants. Check actual rustc manifest/src_path/rlib/depinfo and rebuild the correct frozen input under original cache/lock; do not add fictitious role mappings or change protocol based only on mismatched diagnostics. Any actual uncovered mapping found after this audit remains explicit. #98 original List Press event extraction has been corrected to original Activated semantics without changing tests; the broader prepared/focus policy is unchanged.


## Main R1 original virtual-row reading reveal completion — 2026-10-25

Two unchanged original explicit reading reveal tests remain red in independently rebuilt#102 baseline and candidate: controls datatable_content::caller_row_id_reveals_original_arranged_row_and_hidden_target_never_redirects and selectable_list_content::caller_wrapper_reveal_is_original_and_hidden_deleted_anchor_never_redirects. The current planner rejects their original DataTableContent/KeyedSelectableListContent layout-owner paths before solving Scroll offsets. Main/advisor classify this as finite#98 predictable original-owner/reading completion, not a SplitA failure or a reason to relax custom layout refusal. Original tests stay unchanged and original#98 scope is reused.

Authorize crate-private, read-only original owner proofs for caller row wrappers. DataTable derives the displayed original row frame from current layout_keyed/model row key/source index/incarnation/displayordinal and the private registered wrapper id/marker/mount. Keyed List similarly uses original layout_keyed/current row frame/displayregistration. Proofs preserve original model/View owner, no measure/arrange/reconcile or cloned model, no arbitrary public translation-safe boolean. A wrapper target does not authorize its unknown custom children. If later cell/accessory/display-child support is needed, share actual original layout helper output, never copy inset/8pt/accessory formulas into planner.

Bind Host scope/geometry/input stamps/all route mounts, model origin/dataset/revision, key/incarnation/displayordinal, wrapper structure/id/marker, actual View arranged identity/row/owner frame and local clip/layout metrics, original reading version/item_mounts and Scroll offset/viewport/limits. Final pure validation precedes any effect. Only original ancestor Scroll offset translation is first supported; width/font/Theme metrics/columns/reorder/filter/display/structure changes invalidate old plans.

Prediction must translate full stored row/owner geometry, then recompute fixed ancestor viewport intersection with translated owner bounds, and any relevant original local row/cell boundary. Never translate an already clipped visible rect (it loses offscreen area), and never mark every local clip passthrough. Zero-delta projection must match original arranged frame/clip, otherwise reject. A legitimate displayed mounted row fully outside the viewport may plan/reveal; hidden/deleted/notdisplayed rows refuse without redirect. Preserve original Start/Nearest/margins/partialvisibility contract; no new fullvisibility gate.

Required new evidence: both original tests green; pure planning keeps owner/reading/offsets; commit actually scrolls original key/mount; postplan reorder/filter/replace/width changes reject without partialeffects; original offscreen/partialclip positives and hidden/deleted/customunpredictable negatives. Original focus/restore/admission/reading/List/Table regressions and relevant strict remain. No#81curve/globalgeometry/native/GPU prerequisites or generic analyzer are introduced. Historical closed/reopened receipts keep exact source identity; this is additional original public path evidence.


## Main ordinary Ready bubble compatibility decision — 2026-10-25

Main/advisor reviewed the R1 Select candidate and found a real ordinary bubble compatibility defect before live intake: inner TextContent ignored Ready plus an outer explicit StableHooks owner with default Unsupported must still reach its original ordinary handler. StableHooks does not promise Ready for every verb. Returning Some(default) on that needed ancestor silently swallows the old route.

Prepare only the actual bubble chain chosen by original finish_bubble; stop at its original stopping point, without querying unreachable ancestors. Cache each decision exactly once. Before all needed decisions are prepared, run no effect/focus/capture/queue writes. An ordinary needed Unsupported returns to the original ordinary route after its existing admission checks; a strict Unsupported refuses. Final validation/identity/capacity failures never become fallback. A full Ready chain retains original final validation/focus admission and child-to-parent consuming apply. Never apply a child then discover unsupported/replay the old handler.

Resource boundary: Select preparation may use original Parts::reserve/NEXT_MOUNT checked allocator to reserve unpublished identities. Main accepts the original documented irreversible unused holes, not reuse/rollback. Describe this as no business owner/queue/focus/capture effects before admission with possible unpublished allocator consumption, not completely global-state-pure or counter-unchanged. Final validate re-evaluates the original View decision without another reservation. No duplicate prepare to probe support and no new public API required merely to prevent such duplicate calls. An ancestor whose decision depends on a preceding mutable child callback remains Unsupported and ordinary takes its original route.

Required finite regressions: actual external inner ignored Text plus StableHooks/defaultUnsupported parent executes parent ordinary handler exactly once with old response/capture/action; bubble cutoff does not query unused ancestor; allReady child-parent prepares once/applies child then parent once; strict sameUnsupported chain zeroapply; explicit reservation-hole accounting if used. Keep ordinary direct/pointer marked blur-unmark and sameowner AX text/range authority; no new geometry gate. Preserve original Select rejection/once/overlay/checked declaration cases, current Radio/Search/clock/ComboBox branches, relevant regression/strict and exact source/program/depinfo windows. Original sealed336/default48/external96GPU stays its original window; do not re-sign it as corrected Host evidence. #89 remains open until corrected actual main intake; #98virtualRow/#111 are independent. No new transaction engine, fullDS/native gate or governance Issue is added.


## Main R1 original virtual-row completion accepted — 2026-10-25

Main integrated the six exact r6 necessary path changes, preserving the live Segmented exports and all other WIP. No public API changed; the current 364-type inventory is unchanged. Main ran 413 passing test invocations: controls lib plus original Table/List and directed row reveal 361, and independent public focus/reveal/scroll/Table/List 52. Both controls all-target and finite consumer strict Clippy passed. Both original caller-row tests are unchanged and now pass on live source. Current source inputs were stable and exact across all four lanes; retained programs, generated files and dep-info archive hashes were independently checked. Receipt: design/reviews/host98-main-row-intake-20261025/root-acceptance.json.

The first main harness command named nonexistent internal integration-test targets; Cargo refused before tests. Its failed round remains intact, and only the target list was corrected for the successful fresh round. No old receipt was re-signed. This closes the original #98 AC3/AC4 virtual-row gap, retaining earlier finite action/IME/identity evidence under its actual source windows. #102 geometry qualification, #109/#110 Scroll interaction, #111 remote authorization and #81 publication remain separate open work. No shown window, physical IME, VoiceOver, GPU, performance or release acceptance is claimed.

## Main B1 curve and existing owner compatibility decision — 2026-10-25

The private r7 source candidate is approved for main implementation verification, not runtime acceptance. Target RequiredFocus evidence and additional ancestor curve geometry remain separate contracts. A Chart target keeps its original Sufficient/Insufficient/Unmeasured decisions; an ancestor Panel does not fabricate a descendant witness. Certified convex rounded ClipShape ancestors bind route, mount, coordinates, generation and shape. Scroll projections and the final f32 landing must satisfy those constraints, with target and ancestor evidence rechecked before any commit. Unsupported structures and numerically indeterminate plans refuse without offset/focus/selection effects.

Plan and commit use the same three landing branches: with a certified curve, validate the expanded landing against rectangular and curved constraints; without a curve but with RequiredFocus evidence, require the original unexpanded witness to be fully visible while retaining the original margins clamp policy; without either, preserve ordinary Start/Nearest, partial reveal and oversized-target clamp. This does not add a full-target-plus-margins requirement to existing rectangular reveal.

Primary Down retains the original prepared action/focus/reveal/IME and padding-focus path. Captured Move/Up retain the original point and mounted owner reducer, with Host-certified region admission; no general prepared Move/Up protocol is introduced. Original primary/context filtering remains in force, including Chart hover. Region-aware Button and Chart helpers preserve their original state owners and ordinary paths. Capture invalidation matches the original owner/mount before cancellation. These decisions do not qualify remote B2/#45 or replace #111 authorization.

Finite verification must cover ordinary oversized and witness-with-clamped-margins positives, Panel Button/editor positives, Chart witness refusal and positive Scroll cases, stale evidence and curve no-solution zero effects, primary/non-primary hover, capture and existing Split/Scroll/prepared-update compatibility. The original B1 publication, platform projection and consumer/GPU checks still require actual main execution; no whole-Design-System, native-window or VoiceOver prerequisite is added.

## Main #81 B1 local acceptance — 2026-10-26

The original single Panel/Content tree shares frozen Surface geometry across ordinary and special paint, point/part bounds, focus/reveal and local publication. r11 checks inherited curve generations from the actual Host structure root while retaining modal/overlay input scope; stale/missing cache refuses, legal relayout retains original capture/mount behavior. Target RequiredFocus and ancestor curves remain separate; ordinary oversized reveal and witness-with-clamped-margins keep their original contracts.

Actual phases retain their own identities: 378 controls library tests, 16 Panel regressions; current independent consumers/publication 9+9, runtime local-only 2, platform projection 4; Controls all-target and genuine production Gallery default/SVG strict, feature-enabled 411-type bindings (3 tests), and GPU source strict passed. Actual Panel GPU covers five scales × Light/Dark × Quad/Image/TransformedImage/Glyph/Shadow: 50 corner-gap/positive-ink cases, plus two ancestor/zero-width full-frame negatives. Glyph input is a synthetic opaque atlas mask for pipeline clipping, not a native-font appearance observation. Output is explicitly on the SSD.

Audit: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-81/main-b1-acceptance-20261026/audit.json`; main receipt: `design/reviews/panel81-main-intake-20261026/root-acceptance.json`. The broader AppHost parallel suite had 40 pass/1 settings-writer lock failure; that exact case passed isolated serially, without relabeling the parallel suite green. Full capability generation still rejects historical Stack observation pins; current classifications/bindings were verified independently without changing guards or old evidence.

B1 is locally accepted. #81 remains open for original B2 parent-image/child-pointer/AX, generation and unsupported-boundary requirements. Runtime explicitly refuses unsupported remote curve composition. No full #45, NSWindow, physical IME, VoiceOver, published revision, PR or push is claimed.


## 2026-10-26 finite Panel own-effects follow-up

Independent Issue #82 exposed that PanelContent declared StableHooks but omitted its own prepared-effects capability. Main added owner-bound local_work/local only for Panel own default hooks; Split still recursively audits every child. Current main06 passed 393 Controls lib, 18 Panel regressions (including nested Split positive and unsupported descendant rejection with original input returned), 115 runtime tests and Controls/Panel strict. This adds a finite repair to the original B1 receipt; old source/program receipts retain their identity. The new 1702-file SDK snapshot is local WIP. Its independent consumer passes the former Effects(Unavailable) construction point but still has later layout/focus failures; GPU and B2 remain unverified. See design/reviews/panel81-main-intake-20261026/effects-followup.json.


## 2026-10-26 Glyph hardware-scissor budget amendment

Issue #82's real 260-visible-glyph ordinary scene exceeded the original 100m conservative full-target clip-record budget. The previous rejection and #80 original acceptance remain valid at their own source identities. For the reopened finite #80 follow-up, only Glyph work may use the final prepared integer hardware-scissor area when checked endpoints lie within the target. The exact same state is stored in prepared.states, encoded as Batch::Clip, and applied with set_scissor_rect before Glyph draw; batching does not cross Clip batches. Zero scissor skips draw. Missing/invalid scissor falls back to full-target area. Count all run glyphs independently, including overlaps/transparent or undrawn atlas entries; multiply original depth with checked arithmetic and keep original accumulation/cap. Other pipelines, all CPU/instance/upload/device caps, shaders, AA outward support and pre-submit rejection order remain unchanged.

This retains the original logical covered-pixel × clip-record-access model; it does not claim a hardware instruction/helper-invocation or wall-clock bound. Hardware scissor excludes rasterization points outside its rectangle (https://gpuweb.github.io/gpuweb/#rasterization). No glyph dest/viewport rounding assumption or new draw/scissor behavior is introduced. Source-scissor equivalence and fallback/overflow/zero/overlap/depth tests are required, followed by the unchanged #82 original text and 450/18 matrix on a new fixed SDK. Current candidate is not accepted until actual main verification.


### R2 bounded Glyph-run hardware scissor

The first shared follow-up (Glyph charging by the existing hardware scissor) passed its 10 clip tests and Render strict checks. The unchanged #82 corpus then passed the former 260-glyph half-viewport case, but a 448-glyph/12-line Glyph-only frame still reached the full-target scissor and exceeded the unchanged 100m policy. Its original failure and fixed-SDK identity remain archived.

The next approved candidate may tighten only an analytic Glyph primitive's actual hardware scissor to the union bounding rectangle of its actual uploaded GlyphInstance dest values, with a 16-device-pixel outward gutter. Target axes are 1..16384; all dest positions and nonnegative sizes must be finite integers, sizes at most16384, and mathematical endpoints (checked with f64) integral and within ±16384. Any out-of-domain glyph, empty run or unavailable geometry falls back for the whole run to its parent scissor. Intersect the expanded rectangle with parent and target using wide signed/f64 arithmetic. Preserve complete glyph counts, clip head/support/uniform, other primitive accounting, every cap and pre-submit rejection order. Store the temporary scissor only in that primitive's prepared state; the unchanged frame encoder applies it and restores the next primitive's original state. No shader or public caller protocol changes.

The finite integer-domain model uses the normative WGSL/viewport/polygon point-sampling geometry with a loose 16u(|p|+V) ≤0.0625px arithmetic envelope (u=2^-23); the16px gutter is a conservative strategy, not a universal fixed-function backend error theorem. Budget coverage is enforced by the actual scissor even if a visual assumption later requires fallback. Geometry/source/state-restoration tests and actual Metal pixel equivalence remain required; no untested-backend visual claim or helper/instruction/time bound is added. See https://www.w3.org/TR/WGSL/#floating-point-accuracy and https://gpuweb.github.io/gpuweb/#rasterization. Controlled per-run parent-scissor references must stay within the original budget; production caps may not be raised or disabled to obtain acceptance. Original #82 text and450/18 axes remain unchanged.


## Main R2 Glyph-scissor follow-up acceptance — 2026-10-26

Main accepts the finite #80 shared follow-up using production `clip.rs` SHA256 `f64ecf09ca4f05f6ad4783164b3ae637a595db18a79dc84aaee0e5bf6ea57bc9`. Main18 passed 12 clip preparation tests and Render lib/tests strict checks. The 100,000,000 logical covered-pixel × clip-record-visit cap, full glyph count, other pipeline costs, preflight order and existing CPU/instance/upload/device limits remain; temporary run scissors use actual prepared glyph bitmap destinations plus the bounded-domain outward gutter, with conservative parent fallback. The additional bounded glyph traversal is explicit; no shader or frame encoder changes are part of this follow-up.

The non-instrumented 1707-file production SDK and six-file independent R19 consumer passed CPU3, GPU compilation and strict checks. Original **450 pipeline combinations / 18 full-frame single-coverage oracles** passed under the original shared build lock followed by the original mapped GPU acceptance lock. Evidence: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-82/local-consumer-r19-prod-r2-main18-output-guard-20261026/main-run/matrix-450-18-gpu-lock-rerun-20261026/`; actual executable SHA256 `f69d0a9ea91e6e2a6613bd0635be65f3a5829d33308e22c79d16648cd06bc4d6`. Inputs/runtime remained stable. The earlier unlocked matrix is retained as raw historical evidence and is not the formal acceptance run.

A separate diagnostic SDK changed only the opt-in trace observer, not production state/caps. Its archived test passed exact-RGBA per-run reference comparisons under both original locks. Independent main verification joins **32 render events/frames, 34 Glyph records and 15 normal/reference pairs**, checks actual scissor/depth/count/work and all P-row sums against T. The first verifier wrongly rejected a legitimate offscreen no-intersection run; the corrected verifier follows the existing strict-negative-span parent fallback, preserves touching zero-area scissors and rejects forged alternatives. Four focused verifier geometry checks pass. The original rejection, trace and events remain unchanged; no GPU rerun was used to change that data. Evidence: `.../local-consumer-r15-diag-r2-main18-qa-kindfix-20261026/main-run/supplemental-r15/` and `main-prep/verifier-r2-no-intersection/main-reverification.json`. These local comparisons are not a full-scene equivalence theorem; the independent original450/18 run supplies its own finite acceptance.

This accepts #80's finite shared repair and the corresponding local #82 evidence. Remote B2 curve transport/input/AX/text, public immutable revision, NSWindow, physical IME, VoiceOver and performance remain separately unverified; #81/#82/#45 are not closed by this result. Historical R1/R14 budget failures retain their original identities.


## Local main native publication intake — 2026-10-27

Received on main20d8cba plus this scoped delta: original Host geometry/text snapshots, native-key aliases, multi-Host Window publication, atomic replacement/revocation, native point/range queries and queued-action freshness. Legacy Rect-only node publication still refuses curve trees. Disabled original text remains accessible as readonly shaped layout, while input/actions remain denied; inactive windows and zero-pane stale layouts remain rejected. No new roles, editor, shaper or application state are introduced.

Actual finite checks: Controls text-query4 and strict; platform lib47, original text5, Panel/publication5; original independent public Window consumer2 and strict; existing AppHost callers compile. These checks run at final local main source plus the exact intake delta, under the original shared build lock and SSD cache. Evidence: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-136/panel81-native-main20d8cba-intake/`. Original assertions/scenes retained. Previous450/18 GPU evidence keeps its own source identity: no render/Theme/shader/corpus input changed, so it was not rerun.

This is the platform publication slice of original AC3, not complete AppHost or system accessibility acceptance. Actual AppHost Window assembly still needs its existing Column/Container/AppWindow and Toolbar/Settings integration dependencies; the earlier four-Window-file estimate omitted those real types. No substitute hosts or copied unrelated WIP are accepted. Runtime AppKit callbacks, physical IME/VoiceOver and B2 remain unverified; original AC and product scope remain unchanged.

## Main #82 finite per-instance Glyph shader-work acceptance — 2026-10-29

The preserved #82 native failure is the full original 720×640 logical / 1440×1280 drawable at backing scale 2, with 210 primitives and 636 actual Glyph instances. It reaches the unchanged shader-work cap after the actual scroll, curve outside/inside pointer, single Button action, AX selection and UTF-16 replacement/Undo phases; the original 80-line text and assertions remain intact. The old R2 run-scissor accounting and its 260/448-glyph receipts retain their own identities.

The accepted finite change tightens only the shader-work estimate for an analytic Glyph primitive when every exact instance destination is representable in the existing bounded integer domain. For each actual `GlyphInstance::dest=[x,y,w,h]` produced by the same constructor used by frame encoding, use the integer half-open rectangle `[x-16,x+w+16) × [y-16,y+h+16)`, intersect it with the final prepared hardware run scissor and target, count its pixels, and sum once per Glyph instance. The implementation does this during the existing per-run scan by intersecting each padded instance rectangle with the run's parent scissor. This is equivalent: each padded instance rectangle is a subset of the padded union used for the final run scissor, and the final run scissor is that union intersected with the parent and target. Multiply the sum by the existing chain depth with checked arithmetic. Overlapping instances count repeatedly, transparent/undrawn-atlas Glyph entries are retained, and no Glyph is removed. The 16-pixel outward gutter is the already accepted bounded-run scissor margin and is deliberately conservative around the exact integer raster rectangle; the final scissor remains the same run scissor as before.

This bounds clip-record visits because the Glyph fragment shader calls `clip_coverage` for each fragment rasterized by the instance and the chain depth bounds the maximum records it can inspect. It is not a GPU instruction or wall-clock estimate. If any target/scissor/destination cannot be proven inside the existing finite integer domain, the whole run uses the prior `prepared run-scissor area × Glyph count × depth` charge. A zero-area glyph still pays for its conservative 16-pixel expansion; only an empty run contributes zero. No out-of-domain fallback is weakened. The existing shader cap, CPU/instance/upload/device caps, accumulation and pre-submit rejection order remain unchanged. The added constant arithmetic stays within the existing one-visit-per-Glyph scan and its declared visitation budget; this does not claim zero added CPU instructions or a wall-clock bound. The draw data, scissor, shader, pixels and run ordering do not change.

Local shared acceptance: original/new clip contracts, Render all-target strict, and actual Glyph/curve/frozen legacy pixel regressions pass. The unchanged previously rejected 636-Glyph native input completes 25 actual rendered frames in native05, eliminating the shader-work refusal; its subsequent caret-scroll assertion exposed a separate consumer configuration precondition (vertical Disabled after toggling the multiline input flag). The shared fix does not claim that whole window sequence passed. The final standalone consumer declares the existing vertical Auto policy and remains under its own native/physical-IME acceptance. Current receipts: SSD artifacts/issue-82/main-da532a8-native-candidate-20261029-01/root-intake01/{main-shared08,main-final09,native05}. Old failures and the original 450/18 matrix retain their own inputs and identities; no old receipt is transferred. Shader, scissor, drawing data, budgets and publication scope remain unchanged.

The fractional/negative estimator unit retains all 260 original instances and the exact `[0,0,17,17]` scissor. Its independent expected work changes from `260 × 17 × 17` to `130 × (17 × 17 + 17 × 15)`: the two 1×1 destination groups round to `(0,2)` and `(4,-2)`, so their existing gutters cover 17 or 15 clipped rows. The expectation is a literal calculation, not a call to the new analysis helper. This is the intended estimator change, not a pixel/input/acceptance assertion removed or a cap raised. The original nested/overlap regression keeps its prior independent numeric assertion.


## #82 caret-scroll Quad work correction — 2026-10-29 (local acceptance)

The retained native08 run uses the unchanged original 80-line editor input. Enabling its existing vertical Auto policy really moves the caret-scroll offset to 876, adding the actual vertical scrollbar Quad at `(682,383.8889,4,136.11111)`; it then reaches `ClipBudgetExceeded("shader work")`. The frame has seven Quads rather than six. This is a separate false rejection after the accepted Glyph-only correction, not a successful complete native run or a reason to shorten the input.

For ordinary Quad and KeyedQuad only, inside the already-used 1..16384 target-axis domain, charge the checked area of the **existing final prepared hardware scissor** times the existing chain depth. The same scissor is stored in the prepared state and applied by the existing draw path; fragments outside it cannot evaluate the clip chain. This deliberately retains a conservative whole-scissor charge rather than analyzing Quad geometry. Invalid/unavailable scissor or out-of-domain targets keep the original full-target fallback. Other pipelines retain their existing estimates, including Glyph's accepted per-instance bound and fallback. The 100,000,000 cap, checked accumulation, CPU/instance/upload/device limits, rejection ordering, shaders, actual scissor and rasterization are unchanged. This is not a new geometry/AA/transport support domain or a wall-clock bound.

The old neighbor and nested unit inputs/scissors stay unchanged. Their explicit Quad work expectations change from full target area to the existing 81x81 or 16x16 scissor; the Image still pays full-target work. A directed ordinary/Keyed Quad regression checks the unchanged 9x9 scissor, exact 81-unit acceptance and 80-unit refusal. The existing out-of-domain overflow refusal remains required. Historical failures and prior GPU evidence retain their identities. Actual local acceptance: the retained 33 passing clip contracts plus the corrected affected nested contract, Render all-target strict, and the final native program build pass. The first new nested test used an incorrect 14x14 oracle; the original Rect(2,2,14,14) has existing outward support [1,17), hence 16x16. That failure remains in main-rust01 and the affected correction passes in main-rust02. No production change followed the strict check. The actual original full AppKit sequence in native01 now passes wheel offset6, rounded outside rejection/inside original action once, three exact UTF-16 queries, all80 lines with caret offset876, Undo, Tab and CloseRequested/close; root reviewed all six stage journals, exact program hash and normal exit. Source/program/raw failures remain under SSD artifacts/issue-82/main131f9ef-quad-work-20261029-01/. No shader/scissor/draw input changed, so the prior actual pixel regressions retain their identities without another broad GPU run. Physical Pinyin and VoiceOver remain unverified; this does not close #82 or #45.


## #81 refused input and frozen repaint — 2026-10-29

The original native-r12 sequence passes the same cached AX key/selector and actual child count2, then refuses both rounded-corner pointer events. The next Redraw fails `StaleShapeKey`: `ContentHost::event` retires the input lease for every active input, but `needs_render` previously considered only the response's business dirty flag. The shared driver therefore attempted frozen repaint after a refused event without first acknowledging its new input generation. `needs_render` now includes pending geometry and the current Host/input stamp versus the last successful render. The original render/publish path renews geometry before repaint; the stale-shape guard and rounded input refusal are unchanged. A public Host regression requires stale repaint refusal before render and successful repaint after render, and the original three-process driver adds corner Down/Up directly followed by shared Redraw with unchanged original count4 and no capture/action. Actual native text/resize and final close remain required; no lifecycle or physical IME/VoiceOver acceptance is inferred.


## #81 native logical text during layer publication — 2026-10-29

Original native-r13 confirms pointer count1, original cached AX selector/count2, rounded-corner refusal and subsequent redraw. The editor click and TextCommit are admitted; the commit queues and delivers to the same original child owner. The subsequent Window ExpectText sees None while another same-spatial grant is pending: the Curve branch of ProcessContainer's read-only text/binding query required input_ready, so every new frame withdrew the logical editor as well as stale geometry. The query now retains its original snapshot under the existing active/available and exact session/view checks; text_input_snapshot still withholds caret/clip/carets unless input_ready. Mutable input and queued delivery retain the original owner/focus epoch/text-binding checks. The actual parent native text adapter is exercised through marked-state clock publication, native TextCommit conversion while pending, and another clock publication after B2 text acknowledgement. No platform text-state, TextCommit, Window assertion, permission or renderer implementation changes; native full text/resize/close remains required.


## #81 native text geometry publication readiness — 2026-10-29

Original native-r14 retains B2 text/unmarked in Window and passes attributedSubstring, then the unchanged firstRect assertion sees height0. The trace has a prior actual draft caret with height16 at grant13. TextCommit retires that spatial authority and starts grant14; the next real CaretBlink arrives before its completion and immediately supersedes it with deferred grant15. Fixed-time ExpectText executes before grant15's current publication. This supports a publication-timing hypothesis, not a claim that the new B2 caret has already passed native acceptance. The shared real-three-process regression now injects the same CaretBlink while the original native commit is queued, and requires positive finite current native caret geometry after acknowledgement and after the next clock publication, retaining zero geometry while pending.

B2's existing opted-in publication script also fences ExpectText on a positive finite caret in the actual Window text state, after the original handler/queue drain. It does not match or repair the expected text/marked value; the unchanged original attributedSubstring, hasMarkedText, selectedRange and firstRect assertions run together and still fail wrong values. The default script, original B2 actions/delay deltas, 45-second external guard, logical text fix and all production spatial admission remain unchanged. No polling/timer/retry or stale-caret reuse is added. Actual complete native text/resize/stale/close is still required; physical IME/VoiceOver and frozen lifecycle cases remain unverified.


## #81 pending remount session-resource boundary — 2026-10-29

After the main/native batch was committed as source602d444 and PR184 opened (not yet merged), the two existing lifecycle cases were run using the exact r29 program and a separately audited formatting/prose-only source equivalence. `none` reached begin and passed. `remount` reached begin, started a fresh instance4 process, acknowledged grant30 and returned a frame, but refused its publication and failed the original settle deadline. The old session's pending Revoke queue survived Unmounted inside the same FrameResources; the new process cannot acknowledge that old transport request, so the unchanged no-pending-revokes publication guard remains closed. This is a newly executed lifecycle failure, not an inference from the old r16 pre-begin failures.

Unmount now clears the complete old FrameResources and the old grant/native-finish receipts. Starting a new process session creates a new resource Arc with the original configured view/visibility before queuing its context. Old weak handles cannot bind to that new resource identity. Host sequence/viewport and grant transaction counters retain their monotonic behavior; all publication, cancellation and input identity guards are unchanged. The existing same-instance remount and None scripts and assertions are unchanged; they supply the regression. No r16 None coalescing candidate is applied, since the current exact None case already passes. Actual post-fix remount, default both, affected unit/strict/build and final integration remain required before receiving this follow-up.


## #81 pending None intent — follow-up observed interleaving

The session-resource fix makes the original isolated remount case pass, including fresh session/grant, old weak handle/snapshot rejection and reset Button state. The first post-fix default-both run then reaches the original None begin marker but fails its existing settle deadline: None clears the sent pending grant29's resource metadata and immediately sends revoke29 plus grant30 before consuming GrantPrepared29. The actor keeps the original Prepare/Layout ordering, so the replacement is not acknowledged; no new frame/input authority is published. The baseline isolated None pass is retained and does not erase this actual interleaving failure.

The existing single-outstanding-grant branch now also coalesces None. It immediately withdraws parent geometry/input, retains the sent Prepare/Layout identity until its original acknowledgement, retires that old frame credit, and only then applies the latest frozen None/replacement context through the existing setter. Actual unmount still resets the full old-session resources. The original None/remount/default-both scripts, stale/owner/cancel guards and timeouts are unchanged; no previous frozen patch is mechanically applied. Post-fix original cases and affected checks remain required.


## Final native composition after lifecycle repair

The coordinator also ran the complete unchanged native sequence on the r31 program
`421dcbdc516a5e7029a53443e306838b5baf1d30a8b7c8205ae2cedb1e45b4bc`.
`native-r16` matched its exact source manifest and exited0 without timeout: original
pixel, pointer count1, native AX/count2, rounded refusal, Window text/selection/positive
candidate rect, resize stale input/query rejection and final close all passed.
This supplements the actual default-both lifecycle integration and retained B1 evidence;
physical IME and VoiceOver remain not_run. Final remote CI and merge are recorded in #81.
