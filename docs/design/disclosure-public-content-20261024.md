# Disclosure 公共组合契约（#38）

本阶段基于 Gallery worktree 全部 WIP，保留 #38 原验收条件及 #36 事件记录。当前已有 `extended::Disclosure`、`DisclosureView`、ButtonRuntime、默认方向图标及 ProgressTransition；缺口是公共 Content 接入与受控更新，不重写现有动画或基础交互。不改 AppHost，不恢复已删除的 CompoundButton/IconButton 入口。

## 原对象与职责

新增一个公开 DisclosureContent 适配，保留原 Disclosure 作为已确认 expanded/disabled 值。一个原触发器拥有动作、焦点、capture 和 Button/expanded 语义；默认箭头与任意被动标题 Content 不产生第二个动作或 AX 节点。标题槽拒绝交互后代。正文是独立的原 Content 子树，保留其 ID、编辑器和挂载身份。

复用 DisclosureView 的原 ButtonRuntime 与 reveal，不叠加 Button 状态机。为受控事件补必要的小型 request-only seam；原低层 eager-toggle 方法保持既有行为。默认行几何和圆角继续使用 Disclosure/Design System 规则，图标与字号取现有公共度量。标题和指示器必须使用同一原运行时解析的 action-quiet text/icon 完整结果；不得复制静态 primary/tertiary 前景 helper。该前景 override 只作用触发器，正文仍继承原 ContentTheme/Surface 或自身显式 Surface。expanded 不代表 selected 配色，不新增 Theme 词汇。

## 受控事件与确认

pointer、Enter、Space 和 AX Press 通过同一 owner、同一原动作队列，分别保留来源。事件只提出 `!confirmed_expanded`，不提前改变已确认值。沿用 ContentActionRecord 的 mount 与事件时 owned payload；Disclosure 意图包含所请求值及能够拒绝同一挂载内旧回调的不可变 generation/revision。不能以 mount 代替更新世代。

消费者只绑定业务命令，并通过公开原对象更新/确认接口应用结果。确认在原 trigger 上生效，不替换 Content 或事后补焦点。拒绝保持原值并结清相应意图；更新后旧确认不得覆盖新确认。业务持久偏好类型、session/folder ID 不进入 SDK。外部 disabled 更新必须在事件前同步并取消旧 arm；迟到释放、重复键、失活和旧 AX 目标沿用既有拒绝规则。

## 正文、动画与 IME

折叠/恢复保留原子树，不卸载重建。复用 Tabs 的 marked-text 检查：若折叠会隐藏正在组字的正文，拒绝折叠并保留焦点、选区和组字；不能强行取消或移动到另一编辑器。

确认折叠后正文立即退出输入、Tab 和 AX；原 reveal 可继续按既有 clip 绘制到结束。先核查 Core 已有能力；确需补缝时，仅在原 Content 树区分 retained paint 与 activity，默认行为保持不变，禁止第二份树、缓存 Scene 或第二个 reveal 状态机。原 Tabs 非活动面板必须继续不可绘制，原浮层的 closing/capture 契约不得回退。窗口失活、取消、重新展开及 Reduced Motion 使用已有时钟和清理规则。

RTL 使用既有方向能力；若无公共方向输入，仅为该组件提供直接方向声明，不扩成全局 RTL 框架。折叠指示镜像，展开指示向下；长中文/窄宽必须维持原触发器与指示器的命中和裁剪一致。

## 单一实现负责人和验证

固定实现会话单独负责 `composition/disclosure.rs`、原 `composition.rs`/`actions.rs` 必要接入、`layout_view.rs` 最小 seam、相关公开导出及独立消费者。Theme 会话仅提供现有语义审查；GPU fixture 另约文件和执行窗口。Root 负责原 Issue、依赖账本和独立接收，不同时修改上述共享源。

先运行能暴露缺口的公共消费者/定向用例，再修复：外部接受/拒绝/乱序回调；原 owner/mount/focus 连续；pointer/Enter/Space/AX 各一次；disabled/取消/旧目标；被动标题约束；正文 retained identity、closing 绘制但输入/Tab/AX 隔离；嵌套 marked text 不受扰；RTL、长中文/窄宽、可逆动画及 Reduced Motion。核心 gate 改动必须覆盖原 Tabs 和浮层回归。

普通 Rust 公共 API、独立消费者、实际 CoreText/Metal GPU、原生交互、物理 IME、VoiceOver 和 Gold7 分层记录；没有运行的层标待验收，不以模拟事件替代。复用原 build lock 与 review-cache，保留源码/锁文件/日志/程序；阶段后只清理确认无用的构建中间产物。不测性能，不提交、推送、合并、打 tag 或关 Issue。

只读原始审查：`design/reviews/disclosure-public-content-20261024/design-audit.json`。本文件是实施设计，尚非能力交付或验收完成证明。

### 实施细化（当前实现阶段，非验收完成）

Content 容器保留两个固定兄弟槽：显式 ComponentId 的唯一 trigger 和独立 body；不能将 body 放入 Button 语义 owner 之下，否则编辑器违反 Core 的 NestedInteractive 约束。标题是 trigger 的原装饰 Content，AX 隐藏，不能含交互/浮层。

`DisclosureIntent` 是事件时 owned payload，包含请求 expanded、全局唯一 generation 和已确认 revision。公开 `confirm(intent, accepted)` 只结清最新待确认意图；拒绝与外部同值 `set_expanded` 同样退休旧意图，disabled 变更也退休。adapter 不暴露可变 model，旧 eager DisclosureView API 保持。ContentActionPayload 增加 DisclosureExpansion 变体，穷举匹配消费者需处理新变体；不引入新的事件层或业务类型。

保留正文退出绘制使用默认 false 的内部 `retained_body_paint`，仅普通 paint 和 label/icon/text 服务预检读取。原 input/Tab/AX/overlay lookup/capture/reconcile 均不读取此标志，Tabs 仍使用原活动 gate。navigation.rs 仅将原被动槽验证和 composing 检查开放内部可见性复用。RTL 仅是该组件直接 bool 输入；header 使用原 quiet 状态的 text/icon packet，body 无前景覆盖。真实 SVG/CoreText GPU 与原生、VO/物理 IME 留后续独立验收。

### a1 真实 SVG 资源红例及 canonical 修复阶段

Theme 的真实 GPU a1 在第一张 PNG 之前失败：trigger ComponentId(2) 返回 IconResource::UnknownIcon。旧 943 源清单、a1 原程序及日志保持历史身份。独立 public consumer 的原 SvgIconCache 重现同根错误：新 DisclosureContent 和原 DisclosureView::render_icon 都发出未注册的 bare chevron 名称。此前 API lease 夹具仅证明 API 调用/几何/前景，不能证明 canonical 资源存在；本次加入原 SvgIconCache 的真实资源准备回归，不添加别名到缓存。

默认指示统一使用已有 canonical 资源：向下 tekes-expand、向右 tekes-forward、向左 tekes-back。修复覆盖 Disclosure 的 Content 与原 View 两条路径，以及 TreeView 中同根两处默认资源；不新增 SVG、Theme 词汇、Tree adapter 或迁移范围，不替代 #47 验收。必要旧名称断言按实际 canonical 契约机械更新。

Gallery 的既有 private native backend 接到 canonical forward/expand 时仍委托原 chevron/down painter，属于宿主/backend 接入适配；它既不新增公共兼容入口，也不代表 Gallery 完成 canonical Glyph/SVG 像素迁移。真正 SDK SVG 资源准备由独立 public SvgIconCache consumer 验证，真实 CoreText/Metal 帧由后续 Theme a2 单独收验。

Gallery 定向验证先被既有 #62 接入遗留阻塞：content_palette 仍直接构造已无 brand 字段的 Palette，编译报 E0560。经 Root 明确追加 owner，仅该处迁移到原公开 Palette::new(theme, brand).expect，保留 Workbench 原 brand、基础页 0，不修改 Theme/Palette 定义或引入回退配色。该机械接入修复与 canonical SDK 指示器修复分列，不代表 #62 或 Gallery 全体验收完成。

### focus-clearance 实施细化（a3 后）

a3 的原字形 atlas coverage 与焦点 stroke 相交，及 opaque 标准 Button 正文的原 Scene 排序红例，分别暴露间距与绘制顺序两个独立缺口。修复只复用原 runtime 的焦点状态和 Core 的 focused-branch-last 两遍绘制；不移动 children Vec，不改变 input/Tab/AX 顺序，不缓存 Scene。

header 与实际 body child 之间固定预留原 ControlFrame::focus_bounds(backing_scale) 下侧所需空间（含原 focus width/offset 与 AA backing pixel guard），与当前焦点无关。它是同一个 reveal block 的前置 inset，block 总高为 inset + 原 child 高，不建立另一 motion。公开 body_layout 描述的是含 inset 的完整 reveal block；实际 child frame 仍由原 Content 查询。occupied_height 与 visible_clip 被分配的 reveal 高度/祖先 viewport 限定；实际 child 的 clip 保持已分配、已按祖先裁剪的 reveal visible_clip，不缩到自身 frame；原 body focus ring 因而可使用已分配 inset，hit/AX 仍取原 child frame。折叠 progress=0 时整个 block 占位为0，不能遗留间距。该空间不是 Theme 词汇、fixture padding 或字体缩小。


#### body ancestor clip 更正

focus-clearance 第一阶段把 body clip 与自身 frame 相交，并在测试中把上侧 focus stroke 被裁当作正确；这不符合 Core 的 ancestor-viewport clip 契约。原阶段红绿/程序/身份保留，不再据此宣称 body 自身焦点装饰通过。本轮以真实 body-root 与后代 Exact Button 复现上侧完整直线 stroke 被错误裁掉，再仅把 body clip 恢复为原已分配 reveal visible_clip。固定布局/单 reveal 不变，关闭 activity gate 不变，不扩大已分配高度；gap 不成为 pointer/action/AX frame，正文自身获得焦点时使用的是其原 focus runtime。

## Public-main port (2026-10-30)

The candidate ports the existing Disclosure implementation to public baseline 8a6a7633. The public baseline has no TabsContent/navigation module: its two small title-validation and composing predicates stay private to Disclosure. The original Tabs regression is preserved under design/reviews/disclosure-public-content-20261030/tabs-regression-pending.rs for integration when that capability is present; it has not run here. Core activity uses the current refresh_activity notification path. Trigger delegates current window activity directly to the original view; no new host or runtime is introduced. Historical GPU receipts above describe the Gallery WIP only and are not acceptance of this port. Native input, physical IME, VoiceOver and current GPU screenshots remain not_run.

### Publication validation (2026-10-30)

The publication candidate is based on PR189 commit `7c6cbcdbcbbc21c1c46cba23373efbd8e8312e6f` and depends on that semantic-label change. The Disclosure delta does not include the separate input diagnostic work. Relevant crate sources, lockfiles and consumer support were compared with the tested MAIN combination; the only later source changes are export ordering and test whitespace from rustfmt. The four public Disclosure paths are classified under the existing `tui.disclosure` identity.

Actual scoped results, reused where relevant inputs remain unchanged:
- `cargo +1.95.0 test --locked --offline -p tekes-ui-controls --lib disclosure`: 4 passed.
- Controls `composition`, `popover_content`, `popover_layout_contract`: 16 + 15 + 1 passed.
- Independent composition consumer with `svg-icons`: `disclosure_content` 14, `disclosure_focus_paint` 6, `disclosure_integration` 4 passed; the latter two use headless SVG/GPU/Scene/atlas assertions, not native windows or screenshot acceptance.
- Generated public registry: 3 passed; capability generator `--check` and 45-component composition audit passed.
- Metadata regression: 53 capability and 8 audit tests passed after the PR189 fixture added its existing `tekes-ui-text/src` dependency. The preceding 9 fixture errors are retained separately; they are not reported as passes.
- Scoped controls-library and the three consumer-test Clippy targets passed with `-D warnings`; formatting checks passed after the two formatting-only corrections.

Native keyboard/pointer, physical IME, VoiceOver, current native screenshots, published exact pin and downstream AgentKit delegation remain unverified. At the initial public port, the excluded Tabs case needed a separately unavailable Content API; the Tree-specific case remains outside this Issue. This is a partial delivery referencing #38, not Issue closure or full five-AC acceptance. Cloud CI belongs to the actual PR head and is reported separately from local evidence.

### CI integration repair

The automatic PR191 run `41025939082` at `11956d39a9f4ced7539babe652764bd4285a1f86` failed in three steps, with two distinct causes. Gallery's existing native icon backend accepted only its legacy chevron names, so the canonical Disclosure names painted no indicator. It now maps `tekes-forward`/`tekes-expand` to the same existing artwork; the original visible-chevron assertion remains unchanged. The independent menu consumer and native review helpers must also exhaustively reject or ignore the new non-menu `DisclosureExpansion` payload, preserving their existing MenuSelection assertions. No native fixture phases or modes are included in this compatibility repair. The original failed run and logs remain separate evidence; repair verification and the next exact-head CI result are recorded separately.

Repair checks on the original controlled MAIN cache passed the formerly failing Gallery test (1), menu action-record tests (8), consumer `svg-icons --all-targets` compilation, and Gallery Clippy with `-D warnings`. Inputs remained unchanged during that run. Consumer all-target Clippy on the combined MAIN stopped only at a needless borrow in the separate r4 `examples/support/disclosure_review.rs:297`; that file and fixture are absent from this publication branch and were returned to their owner. This branch retains only the two native compatibility hunks. Exact-head cloud CI validates the resulting publication tree independently.

### Native consumer acceptance slice (#38, preparation)

`examples/composition-consumer/src/disclosure_review.rs` supplies one public
DisclosureContent with a passive compound Chinese title, its original body
TextField/Button and an unrelated editor. `native disclosure` reuses the existing
composition native Host, public SvgIconCache and AppKit selftest. Build this mode
with `--features svg-icons`; no Gallery or alternate trigger/painter is involved.
The only shared-host changes are the mode selection, original SVG renderer
adapter, action-record collection and phase-controller dispatch. Existing Menu
payload matches explicitly reject/ignore the new Disclosure payload as appropriate.

The 15 ordered phases require one request per positive pointer/Enter/Space/AX
gesture (seven total), then check controlled acceptance/rejection, rollback, stale generation,
retained mounts, collapsed Tab/AX exclusion, original marked-body collapse refusal,
disabled input, and narrow RTL light/dark presentation. Six PNGs are requested from
the actual window. The marked setup has a fixed UTF-16 baseline: `中文 body draft`,
selection `1..1`, marked `0..2`; it is checked again after refusal. The unrelated
editor must remain `Keep A😀B` with selection `0..0` and no marked range.

`tests/disclosure_review.rs` drives this exact controller through public ContentHost
APIs in normal and Reduced Motion modes. It uses CoreText and a declared image lease
for fixture validation, not AppKit or pixel acceptance. The existing 60 scoped
Disclosure tests remain the evidence for intermediate closing paint/input gates,
remount rejection, high contrast and the detailed theme/geometry contracts; the
native fixture does not relabel those tests as native results.

After a coordinator-controlled source switch, compile/test through the existing
`scripts/tekesui_build.py` lock/environment and registered SSD review cache:

```text
cargo +1.95.0 test --locked --offline --manifest-path examples/composition-consumer/Cargo.toml --features svg-icons --test disclosure_review
cargo +1.95.0 check --locked --offline --manifest-path examples/composition-consumer/Cargo.toml --features svg-icons --all-targets
cargo +1.95.0 clippy --locked --offline --manifest-path examples/composition-consumer/Cargo.toml --features svg-icons --all-targets -- -D warnings
cargo +1.95.0 build --locked --offline --manifest-path examples/composition-consumer/Cargo.toml --features svg-icons --example native
```

These are command payloads for the controlled entry point, not permission to bypass
cache/source ownership. No target/cache registration is introduced by this slice.

The future native window uses `scripts/diagnostics/disclosure38/run_native.py`.
It reuses the existing coordinator helper loader, shared build lock, GPU occupancy
guard, cache-owner check, native-process check and 60-second HID-idle check from the
accepted #46 runner. It never builds, retries, waits for an occupied desktop, or
terminates another task. The coordinator binds the actual archived successful build
and executable, runner/helper digests, related changed source/manifests and existing
guard paths in the same binding schema as #46 (`main`, `compiled_base`, `program`,
`build_result`, `runner_sha256`, `base_loader`, `gpu_guard`, `build_helper_sha256`,
`sources_and_manifests`, `native_process_pattern`, `external_display_id`). This does not add a source-closure
admission gate. A launch requires a separately allocated native window:

```text
python3 -B scripts/diagnostics/disclosure38/run_native.py --binding /absolute/prepared-binding.json --output-root /absolute/authorized-SSD/attempts --reduced-motion 0
```

Run the same bound program with `--reduced-motion 1` in its next authorized window.
Every invocation preserves a unique attempt, raw journal/logs, six screenshot files,
input digests and result or not-run reason. Independent journal validation runs even
if the process aborts late; unreached phases never count as passed. The offline reader
regression is `python3 -B scripts/diagnostics/disclosure38/test_evidence.py` and does
not import or invoke native `main()`.

Preparation and headless results do not satisfy native screenshots, physical IME,
VoiceOver, final Gold7, published exact consumer pin or AgentKit migration. Native
PNG existence is checked automatically; coordinator visual review remains required.

#### Consolidated candidate and evidence boundary

The fixture candidate is based on PR191 `0215b6a7fbb7d06fc197feeb6f6ec6981c20e69a` (CI run `41030568192`: SUCCESS). The published tree is preserved. This candidate consolidates the r4 fixture, display preflight, explicit public-SDK marked AX refusal, and the fixture-only needless-borrow correction. It also keeps the actual native window on an external screen and checks that screen at creation. All three original mounts (trigger, body editor, unrelated editor) are asserted throughout; after Unmark, body text and selection must remain `中文 body draft` and `1..1` through disabled/RTL/resize/final closing.

The retained r4 receipt is a failed headless fixture run (exit 101); native was not_run. It is not a passing result for this corrected candidate. The corrected fixture has now passed the controlled Rust test, all-target compilation/Clippy and native example build on the actual MAIN combination recorded below; those results do not replace the retained r4 failure or imply native execution. No native launch, physical IME, VoiceOver or pixel acceptance is authorized by preparing these commands. Display preflight proves the bound external target is present and unmirrored; SDK External placement plus the creation assertion enforces the external-screen class, not acceptance of a specific physical display or its pixels.

| Phase | Check / next input | Cumulative actions |
|---|---|---:|
| 38001 | Collapsed body absent from AX; capture, pointer click | 0 |
| 38002 | Pointer request accepted on same focused trigger; Enter | 1 |
| 38003 | Enter rejected, duplicate confirmation rejected; Space | 2 |
| 38004 | Space collapse accepted; Tab | 3 |
| 38005 | Tab skips body; hidden body AX refused; AX trigger | 3 |
| 38006 | AX accepted then authoritative rollback; AX trigger | 4 |
| 38007 | First AX intent remains pending; second AX trigger | 5 |
| 38008 | Old generation refused; select original editor and Preedit | 6 |
| 38009 | Exact marked baseline; external collapse and SDK cross-owner AX refused without action | 6 |
| 38010 | Pending collapse refused while marked; rejection settles without editor mutation; Unmark | 6 |
| 38011 | Exact unmarked state retained; disable and click | 6 |
| 38012 | Disabled click emits nothing; re-enable, RTL, narrow resize/dark | 6 |
| 38013 | Narrow RTL/dark state and owner retained; AX trigger | 6 |
| 38014 | Final AX collapse accepted; capture | 7 |
| 38015 | Fully closed reveal, original owners retained, complete | 7 |

Queued AXPress actions are positive AppKit callbacks. The marked negative AX case is explicitly a public SDK call and journal field; it is not relabeled as native AX or physical IME evidence. The offline reader rejects changed editor owners, changed post-Unmark state, and changed intermediate action owners/sources in addition to its existing phase/count/generation/refusal checks.

Current consolidated-candidate checks: offline Python evidence/display-preflight regression 6 passed; rustfmt parse/format checks and generated inventory/composition audit (45 components) passed. After the four fixture source files were accepted by the coordinator, the original helper/build lock/registered SSD MAIN cache ran all four requested commands successfully: `disclosure_review` test (1 test covering normal and reduced motion), consumer `svg-icons --all-targets` check, the same all-target Clippy with `-D warnings`, and `svg-icons --example native` build. Actual inputs were MAIN `77860b59df6eb8dc3432a96dd4bfd1730c1b215c` plus the coordinated AX/Settings/other retained WIP, not the candidate HEAD. Recorded source inputs stayed unchanged. The raw commands/results, scoped source/configuration identities and archived executable identity are retained in `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-38/native-fixture-20261030/consolidated-validation-r1/receipt.json`. The native executable is built and archived but was not launched. Native interaction, screenshots/pixels, physical IME and VoiceOver remain not_run; the older r4 exit101 receipt remains intact. No runtime repair was required after these four green checks.


### 2026-10-30 normal native failure repair candidate (not executed)

The 4815c5a/3b0bf8c normal native attempt exited 0 and reached all 15 checkpoints, but its original Python reader failed on obsolete PointerDown/PointerUp tokens. That independent two-file reader repair remains separate; the original failed receipt and six screenshots are immutable. All four screenshots named light were actually dark. WindowDark(false) intentionally restores system inheritance; it is not a light override. A proposed WindowLight selftest action selects NSAppearanceNameAqua for this window only; existing WindowDark semantics and system settings remain unchanged. The Disclosure startup requests that action before checkpoint38001; the original phase12 Dark action still tests the same window's later dark transition.

The body focus ring's horizontal cut is rooted in DisclosureContent::reveal: layout_body yields a reveal clip bounded in both axes to the body block although reveal motion is vertical. The isolated candidate retains the actual ancestor horizontal clip while preserving the previous vertical reveal extent. Body/child frames, occupied height, motion, input/AX frame checks, owners, and closed/inactive gates are unchanged. This grants the same existing ancestor paint room used by the header; it does not grant paint outside that ancestor or change TextField. Proposed API geometry/no-hit and existing Scene side-stroke assertions cover the new contract. No fixture padding or narrowed editor conceals the defect.

Native diagnostic additions expose the current typed NSWindow frame (global AppKit points, bottom-left origin) and NSScreenNumber/CGDirectDisplayID without moving windows, and record window number/backing scale plus immutable per-checkpoint snapshot copies. The actual runtime values must come from a newly compiled program; old PNGs/receipts are not backfilled. Shared platform selftest/window/screen hunks require serial platform-owner intake. Candidate Rust compilation, behavioral tests, native screenshots and real IME remain not_run pending controlled windows. Current system pinyin source was observed, but automated preedit has no real candidate/commit/hardware proof; no new IME state machine is introduced.


### 2026-10-31 Disclosure under retained Tabs panels — regression candidate

The original Tabs dependency is restored by #195 / PR200 (merge a883d00, CI41171126847). Its public TabItem::new(label, panel) stores the caller's original Content panel directly; TabsContent::select(index) synchronizes activity without rebuilding that panel. Here “panel” means this real TabItem.panel slot, not a new mock activity flag or an additional curve PanelContent requirement. The existing navigation_inactive_panel test already enables the preserved generic overlay/service-demand regression, and navigation_content tests cover generic panel capture/composing/retention. Their evidence remains within those contracts; neither substitutes for an actual Disclosure subtree under that slot. Navigation owner confirmed this interface and lack of a same-AC dedicated Disclosure result on 2026-10-31.

The independent consumer target disclosure_under_tabs now proposes three real ContentHost combinations: (1) a selected Disclosure's closing body continues clipped Scene paint while input/Tab/AX retire immediately, then the inactive Tabs panel suppresses even retained paint/icon demand and reselect preserves the closed state/mounts/draft; (2) original marked editor blocks both Tabs deselection and Disclosure collapse, then Unmark permits a switch that retires the original body-button capture and late release, preserves editor text/selection/mount and allows one fresh action after reselect; (3) leaving the panel retires a pending Disclosure intent, and returning needs a fresh generation on the same owner. The original public TextFieldContent, Button and DisclosureContent remain mounted; no Navigation/Core production implementation is changed. The icon lease is an API seam only; Scene/clip assertions are not framebuffer/native/SVG pixel acceptance.

Candidate formatting/patch applicability checks do not establish behavioral success. Actual Cargo test/scoped strict remain not_run until the integration executor grants the original serialized MAIN-bound build/cache/GPU guard window. Reuse existing generic Navigation/old native evidence only within unchanged contracts. Original pending file and all old failures remain preserved; this candidate does not close #38 or restore physical IME, VoiceOver, performance, complete Gold7 or downstream AgentKit scope.


#### 2026-10-31 first scoped execution: explicit normal-motion fixture correction

The original bc70023 scoped target compiled and ran all three tests: two passed, selected_closing_body_retires_input_tab_ax_and_inactive_panel_cannot_repaint_it failed at its required intermediate reveal assertion (line217); strict was not_run after the failure. Original source/program/logs and release remain preserved in artifacts/issue-38/disclosure-under-tabs-cpu-window01/run01/. The shared CoreText test helper defaults reduced_motion=true; the new fixture enabled services but did not explicitly request normal motion. The helper forwards that exact flag through motion_context to Disclosure's existing reveal retarget. Thus an intermediate normal-motion oracle was paired with a reduced-motion fixture. The isolated correction explicitly sets text.reduced_motion=false, matching this selected-closing scenario. No product source, timing, assertion, tolerance, positive painted-body oracle, inactive gate or expected test count is changed. The correction is not a new runtime passing result; a newly admitted source and new scoped grant are required before retest/strict.


#### 2026-10-31 second scoped execution: visible retained-paint sampling

The b9ab108 retest passes the original intermediate reveal condition but fails the retained-body painted-Button oracle; its other two tests pass, strict is not_run, and window02/run01 artifacts remain unchanged. The fixture sampled a trailing Button after the complete labelled editor. A shrinking vertical reveal is allowed to clip a trailing child completely even though the body is retained and partially visible. The source easing is a fast standard ease-out (0.16,1,0.3,1) over 300ms; the unchanged 10→40ms sample is not a promise that the last child is visible. This source observation is not a measured runtime clip/rect trace; neither initial failure logged those exact values, so it does not establish a shared painting defect.

The isolated fixture correction puts the same original exact primary Button first in the retained body stack, with the original editor following it; no padding, new painter or new state machine is added. Both controls and all mounts/IDs/retirement/selection contracts are unchanged; the times and every existing assertion remain. The positive oracle still requires the exact opaque original control quad intersected with the complete Scene clip stack. Additional assertions require a painted expanded baseline, an unchanged sample frame and actual sample/reveal intersection; failures print action/reveal geometry rather than being silently accepted. The inactive-panel negative oracle remains identical. This tests original retained body paint within the actual visible slice, not the impossible promise that every clipped-out child paints. New Rust/test/strict are not_run pending a fresh source admission and grant, and both prior reds remain preserved.


#### 2026-10-31 third scoped execution: selected-state oracle after real Tab routing

CPU03 actual8ec186a compiled and again ran3tests/2pass1fail0ignored; strictnot_run. The selected-closing test passed its normal-motion, expanded baseline/frame/reveal intersection, retained-paint positive, inactive-paint/icon negative and body-retirement assertions, then failed at line262 on assert(select(0)). Original false return is preserved; runtime did not print selected_before. Navigation's original public contract is changed-bool: same-index select returnsfalse but still synchronizes panels. Its existing TabsContent::set_focus restores the retained roving index, and TabsView::focus sets model.selected; the real Tab-routing check may therefore have already reselected panel0. With this fixture's enabled/window-active/two unmarked valid slots, false is not evidence of refusal.

The candidate keeps actual Tab events and every retire/retention/paint assertion. It records selected_before, requires select(0)'s boolean to equal (selected_before!=0), then independently asserts actual selected()==0 before the existing closed-progress/draft/mount checks. Thus it validates both exact selection return semantics and desired state rather than ignoring the return or suppressing a failed contract. A single marked clause is expressed with iter::once(range).collect(), matching existing consumer fixtures and preserving exactly the same range vector; this is source cleanup before scoped strict, not a claimed observed Clippy failure. No product/clock/time/tolerance/test count or other oracle changes. Newruntime remainsnot_run until fresh grant; allthree historical failures and successful subchecks remain distinct.


### 2026-10-31 Disclosure-under-Tabs CPU04 finite acceptance

Actual compiled source e4061e3c7d29e65d56ab9f3a7b0f520b67616b90, scoped CPU04: disclosure_under_tabs ran 3 passed / 0 failed / 0 ignored, followed by the same target Clippy -D warnings exit0; explicit jobs2, original registered cache/helper/guards. Exact program SHA256 d8e2f1334fa868856cbaa5e1b003f90ee0942103fe7a761669f69cc3c3fcbb51. Full original evidence and release: artifacts/issue-38/disclosure-under-tabs-cpu-window04/run01/HANDOFF.json. This result supersedes no historical failure; CPU01/02/03 source/program/logs remain intact.

- Selected closing retains visible original body paint while original body input/actual Tab/AX retire; deselected real Tabs panel suppresses retained paint/icon demand, then reselect preserves closed state, draft and mounts.
- Original marked editor blocks both hiding operations; Unmark permits panel selection, retires capture/late release and preserves draft/selection/mounts, and a fresh action is accepted after reselect.
- Pending Disclosure intent is retired across panel inactivity/reselect; stale confirmation remains rejected, a fresh generation on the same retained owner can confirm. Actual select changed-bool and selected0 state are independently asserted after real Tab routing.

3579 source entries, original8WIP, HEAD/branch and cache markers were unchanged; owned processes/groups and original guards were released before handoff. No Navigation/Core production change, GUI, OS input, GPU/native execution or new CI in this batch. The target covers CPU/API/CoreText and Scene/clip under its fixture's default/Light inputs; synthetic icon lease does not establish SVG pixels. Real OS pinyin/physical IME, VoiceOver, performance, complete Gold7, broader theme/native matrix and AgentKit migration remain unverified. Prior finite normal/reduced native evidence keeps its original source/scope. #38 stays open for remaining original AC; publication belongs to the integrator's next independent compatible batch, not PR208.

### 2026-11-01 AC5 CPU012: branch reveal clip repair design (interface review, not executed)

CPU012 standalone public a883 consumer produced 5 PASS / 1 FAIL. The paired and independent original Stack controls actually planned/committed offset 0→670 with the same leaf landing wholly at y272..300 and the original mounts. The delegated Disclosure's same initial leaf frame failed planning with UnsupportedLayout, while fully open (progress1). The body's declared visible clip was (0,37,400,263), whereas the Scroll viewport was (0,0,400,300). Original CPU011/012 artifacts remain unchanged. This is a known compatibility failure, not a speculative layout extension.

The Disclosure producer and content_reveal_plan.rs are byte-identical from a883 to normal-merged 7d9d792b4d5d26d9516671d8ff430d76a4e69a33. DisclosureView/layout_body and the Component reveal hooks are likewise unchanged; unrelated Select/Collections changes in their shared files are preserved. Candidate starts at 7d9d792, without writing MAIN/PUB.

Proposed minimal shared interface:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostRevealVerticalClip {
    pub top: f32,
    pub bottom: f32,
}
// Component: default None, no new layout qualification.
fn host_reveal_child_vertical_clip(
    &self, child: ComponentId,
) -> Option<HostRevealVerticalClip> { None }
```

The declaration describes ONLY the selected direct child edge's vertical boundary in last-arranged Host coordinates, BEFORE ancestor intersection. The planner translates top/bottom by the owner's accumulated ancestor Scroll y delta, then intersects with the current projected ancestor clip; it preserves that clip's x/width. It does not translate a previously viewport-clipped visible rectangle. Header edge returns None; body edge returns top=body_layout.body_rect.min_y(), bottom=top+body_layout.occupied_height. Thus the original reveal/clearance/clip painter and input geometry stay unchanged. No general callback framework, unknown-layout fallback, or consumer clip rewrite is introduced.

NodeState snapshots only the actually selected next child's declaration, validates finite top/bottom and top<=bottom once on entry, and includes it in exact snapshot equality at commit. Existing common local_clip remains unchanged and applies before the edge boundary. Root/row/pane/curve authority is unchanged. Unknown owners still require their existing positive passthrough capability; the new declaration grants none. An empty selected strip is NoFeasibleRegion. Invalid numeric declarations remain Geometry(InvalidInput). Path identity/mount and original geometry generations retain their existing authority.

DisclosureContent adds a last-arranged eligibility flag. measure/lifecycle/authoritative expansion changes invalidate it. A settled fully open (progress1) or fully closed (progress0) state after arrange can advertise translation passthrough; opening/closing intermediate layouts cannot predict geometry and remain UnsupportedLayout. The declaration uses the original body ID; children/order/mounts are never replaced. Existing closing retained paint, active=false body retirement, motion schedule, IME collapse barrier, input/AX ownership and palette are untouched. Closed descendants are still rejected first as UnavailableTarget. A business wrapper forwards the SDK's passthrough and new edge declaration, AND retains its own arrange/epoch/row-identity checks; a bare true is insufficient.

Exact writer boundary (one SDK domain patch writer; Core readonly review; Main final integration):

- crates/tekes-ui-controls/src/composition.rs: re-export the small geometry declaration and add the default Component hook beside host_reveal_local_clip.
- crates/tekes-ui-controls/src/composition/content_reveal_plan.rs: public declaration; NodeState read/snapshot/equality; projected edge intersection; unchanged fail-closed baseline comparison and atomic commit path. Cover all three snapshot call sites, including logical-focus validation, without weakening their contracts.
- crates/tekes-ui-controls/src/composition/disclosure.rs: retain latest arrange eligibility; exact body-edge declaration; settled passthrough hook. No Trigger painter, body arrange, public child hierarchy or existing actual clip change.
- examples/composition-consumer/tests/disclosure_host_reveal.rs and a local support module if needed: genuine direct control and delegated control, same-geometry original Stack, real Scroll nonzero commit and complete landing, stable mount, trigger branch, closed exact error, stale plans, intermediate motion refusal, nested/translated ancestors and numeric-invalid declaration. Reuse CPU012's 900pt/400x300 oracle, not a synthetic replacement layout.
- docs/design/disclosure-public-content-20261024.md: this design and subsequent actual results. No manifests/locks/features/host changes.

Complete affected verification batch (all NOT_RUN until one explicit local grant): controls unit tests (includes original planner/scroll/focus/curve/transaction tests), default public composition consumer suite (includes existing disclosure_content/theme/integration/under_tabs/host_reveal_plan/scroll_reveal plus new target), strict controls and consumer all-targets default plus svg-icons and bitmap-images feature compile checks. All implementation/exhaustive enum matches/fixture literals impacted by the new NodeState field must be updated in the same candidate. SDK tests use a CPU icon-service seam; this does not claim pixels. Then AgentKit owner reuses original standalone six-gate inputs and forwards the new declaration; full six must pass, followed by affected real Summary/Sidebar CAS/epoch/mount/draft/reveal and host/second-consumer checks on one exact SDK freeze/pin. Formal CI is one later compatible Main batch, not domain self-CI. No GPU/native/physicalIME/VO/performance expansion.

Reuse boundary: existing original native/pixel and theme/painter evidence keeps its old source/program identity; those painters/tokens are unchanged. CPU011/012 remain red on a883. Existing PR209 current-tree three under-Tabs passes remain valid historical evidence, but affected consumer/planner checks rerun for the new candidate. New pin/program results must never be inferred from old successes. SDK readiness requires actual affected checks; AC5 closure additionally requires AgentKit real consumption and normal public delivery.

#### Candidate implementation and CPU-only verification refinement (not executed)

Core readonly interface review accepted the design above. The implementation stores the last actual arrange's (child ID, mount) pair rather than a bare flag; selected edge snapshots share one helper across reveal, logical focus and commit reread. Authoritative expansion, disabled changes and lifecycle invalidate eligibility. Actual body arrange/clip/painter remain byte-for-byte unchanged except recording identities after arrange. Rustfmt 1.95 and git diff whitespace check passed; no Cargo/behavior validation has run.

The initial phrase "default public composition consumer suite" is narrowed to the complete named CPU targets below: the full auto-discovered suite contains GPU tests and must not be run under a CPU grant. Runtime CPU target whitelist: disclosure_host_reveal (12 new public cases), disclosure_content, disclosure_under_tabs, disclosure_review, host_reveal_plan, scroll_reveal, pure_translation_guards, nested_reading, reading_checkpoint, split_view_content, radio_scroll_transaction, segmented_scroll_actions, composing_geometry, editor_keys. Each target runs in full, not a chosen case filter. Controls library units and component_registry_api compile/execution cover the shared planner and public type binding. No original protocol 18 named or unrelated workspace targets are rerun.

Default/svg-icons/bitmap-images strict validation compiles Controls --lib --tests, and consumer --lib --tests plus exactly nine tracked examples: context_menu, hover_preview, icon_content, native, padding_focus, popover_content, rich_content, toolbar_mount, wake. Compiling native does not launch it. Do not auto-discover eight unrelated Unicode/WIP examples in the frozen MAIN source. No tests execute under SVG/bitmap strict commands. disclosure_integration and disclosure_focus_paint execute actual headless Renderer work behind svg-icons and are explicitly not run here; their painter/resource inputs are unchanged and their old qualified evidence keeps its original identity. Metadata ownership must classify composition::HostRevealVerticalClip and refresh the generated capability/API binding once against final source before this batch; no extra type/root re-export is proposed.

### 2026-11-01 CPU34 actual failure and bounded continuation candidate

CPU34 ran once on Main e0488c2f28fb5f52a52d1ee49314de6b42865f5b/tree0b98b591352d9765c2fe9defd41db531c3e4011c with the original accepted engine. Controls451 and component_registry_api3 passed; disclosure_host_reveal returned10PASS/2FAIL. The original six CPU012 oracles, direct control, header branch, invalid/empty/unknown and stale declaration cases passed. Failed motion fixture reported progress1 at the intermediate assertion; failed nested case returned NoFeasibleRegion at plan. Subsequent20 commands were NOT_RUN. Aggregate464PASS/2FAIL is not a full592 success. Raw stdout SHA0e8419df851073eefec686d2a182ce3906b200750554d3be070813ccabdceb97; program8226b5a50d6829519ac74aa092022ca17b55072cc095435a664ac3cd6ec0875e; depinfo c7145e7d2030a20ff81902d4e6ba4cee953fddb3f4bc0b6a1265e8d3160c026c. Source3618/cache markers unchanged; owned processes/PGs, target lock and guards were released before analysis. Original run/runner/card/grant are immutable; no retry used that lease.

Diagnosis separates actual assertions from source inference. The motion test used a closed Disclosure as a measured Scroll document: measure allocates only the header, so arrange has zero body allocation and the existing DisclosureView no-visible-animation-area branch snaps to its target. It therefore did not establish its required intermediate state. The corrected fixture uses the same real control, body and clock inside a fixed host allocation. It keeps the strict0<progress<1 assertion and UnsupportedLayout expectation, adds actual visible reveal area, and verifies the settled strip never exceeds allocated host height. No motion algorithm or expected refusal is changed.

The nested failure exposes a production declaration defect: the view treats a zero-area ancestor intersection as invalid visible geometry and returns occupied_height0 even for fully open progress1, while the actual body still has its original full frame. Deriving a predictive strip from this already ancestor-dependent occupied value loses the boundary required to bring it into view. The actual failure log does not trace a specific internal if; this is the fixed-source explanation of that NoFeasibleRegion.

Minimal production correction is private to DisclosureContent: record the last arrange's raw allocated vertical strip, top=block.min_y(), height=min(block.height*the original view progress, actual allocated body height). Publish it only through the already approved settled/identity qualification. The existing body_layout output, actual body frame/visible clip/input/paint, planner baseline comparison, public hook/type and all transaction/unknown-layout checks remain unchanged. This retains allocation limits while supporting an entirely offscreen owner. The nested fixture keeps the same original900pt spacers and full offset/complete landing/original mounts, adds a same-geometry original nested Stack control and explicit stage/error logs, and asserts that the actual offscreen visible clip/occupied height stay empty while the declared allocated strip is positive. No target is moved to visibility to obtain a pass.

This next candidate changes only disclosure.rs, disclosure_host_reveal.rs and this design document relative to e048. Core reviews only the exact new Disclosure delta; composition.rs and planner are unchanged. Format and diff whitespace checks only, new behavior NOT_RUN.

Continuation runtime plan: re-run the entire12-case changed public consumer target, the original13 remaining named consumer targets, consumer lib1, and API binding3 after final metadata refresh; six existing strict feature/example commands remain necessary because none ran in CPU34. Total22commands/16runtime programs/141expected fresh cases/6strict. Propose qualified reuse of Controls451 on its original e048/program identity: no Controls library unit instantiates DisclosureContent (source references exist only in its definition and the two exports), while the only changed producer code is private to that type; planner/projection/unit code and public API are unchanged. This is not a new-candidate unit execution claim; Main must accept that dependency-based reuse. API3 is rerun because generated metadata inputs change. Original consumer10PASS keep their e048 identity but do not replace the full changed12-target rerun. No GPU/native/wholeconsumer/wholeworkspace/18protocol duplication, and no automatic retry. Main owns final metadata refresh/freeze/admission/new grant.

### CPU014 real Sidebar strict-Scroll input failure: minimal Trigger qualification

The fixed ecdd product consumer actually failed its first normal-rendered Sidebar header AXPress activated assertion (ui/consumer/tests/summary_disclosure.rs49). Primary raw02-public-consumers.log SHA cf734c84c691235134531358ce4fbd02e4fdf13f63492cfb2a5d5e926af7db27 is preserved by AgentKit; this raw does not expose consumed or owned queue state. The earlier SDK141/6strict and DEV6 do not cover actual header action admission under the Sidebar's Scroll ancestor. Existing27 non-Scroll tests cannot establish that contract either.

Fixed-source diagnosis: action_preparation original_reveal_admissible rejects a Scroll ancestor rather than using legacy dynamic reveal. Ordinary local input then requires a recognized rectangular/curve input geometry; private Disclosure Trigger still inherited rectangular_input_geometry=false and CurveSubtreeCapability::Unsupported, so the default unsupported action contract can refuse before accessibility_action/request. The original request_activate reducer does not require forced focus. Keep this strict Core policy: no shared admission/focus/clip/prepared transaction/unknown-layout fallback change.

Minimal producer correction: private Trigger input_arranged starts false, is withdrawn on measure/unmount/window inactivity, and after the original actual title arrange records whether the trusted Host frame intersects its trusted clip with positive area. The existing rectangular_input_geometry hook returns that readiness. It declares only ordinary rectangular input using the original reducer/frame/clip; it grants neither Curve Local nor StableHooks/prepared effects nor ancestor translation. Host still checks current state/disabled/window/clip/focus plan and IME barriers. Actual paint/hit geometry, controlled intent ownership and confirmation remain untouched. No new public API or private consumer trigger is introduced, and no force-focus workaround is allowed.

New independent public CPU target disclosure_scroll_input has6 full cases: real Scroll AX with normal/preserving focus, owned once-only rich records and controlled accept/reject; real pointer capture/up once-only intent; unarranged/clipped/inactive refusal; disabled interval refuses resurrection; preserving header AX keeps the marked body editor/other input draft/selection while collapse confirmation refuses; rectangle declaration does not promote a Trigger into a curved Panel or grant an unknown owner input. It uses the existing CoreText CPU and prepared icon lease seam, not Renderer/native/device input. Response assertions include consumed and activated plus queue/mount checks. Format/whitespace only, behavior NOT_RUN.

Proposed affected local card: API binding3 after Main metadata refresh, full new6, existing disclosure_content15, disclosure_under_tabs3 and disclosure_review1;5 real programs/28expected fresh cases. Six default/SVG/bitmap strict compile commands retain explicit9 tracked consumer examples and exclude unrelated Unicode WIP auto-discovery. Submit dependency-based qualified reuse of the original Controls451 to Main: this private Trigger is constructed only by DisclosureContent; no library unit constructs either (only definitions and two exports), and shared action_preparation/planner/projection/policy-unit code and inputs are unchanged. Core reviews the exact new producer declaration against that original policy, not a new policy bypass. Keep the old e048 program identity rather than relabeling it as this source. The unaffected CPU35 host_reveal_plan15/other projection/reading/editor consumers, token/painter/GPU/native and original protocol18 retain exact old evidence because their relevant implementations/inputs/contracts did not change. The producer-focused old19 and new6 cases still run whole, plus API3; no wholesale592/wholeworkspace rerun or new GPU/foreground/CI scope. Main owns source admission/final metadata/freeze/new grant; no Cargo is authorized during the ongoing Web resource window. Original CPU014 log/source/program and all preceding red programs remain unchanged; real AgentKit product scope must retry only on a new admitted SDK source and grant.

Core input review required one explicit invalidation fix before accepting the candidate: public DisclosureContent::set_window_active formerly wrote the private fields directly, bypassing Trigger::window_active_changed(false). The public setter now retains arranged_children withdrawal and delegates to that existing single handler, so inactive clears input_arranged and active alone cannot restore it. The existing negative test covers public false/true without arrange (AX/pointer/queue refused), then actual arrange restores the positive AX request, while original Host-inactive checks remain. This is a required declared-state contract correction, not an observed CPU014 response beyond activated=false or a new policy framework. No behavior has run; original r1 patch/review and CPU014 raw remain preserved.


#### CPU35 finite acceptance (actual, 2026-11-01)

Main froze final ecdd393f397aac3f6fed962a8cecc2eb2d9a5fb8/tree d0ad5faa67fd34585c76a70aa75d9cca95984e0f,3618 files including the unchanged8 Unicode WIP; freeze SHA9c2a8bc79f22aad2873611ea18c0d3c1e5e1f71b68e129afdd390ea51724264b. Under the new explicit UI-Disclosure38-CPU35 lease, the original domain ran all22 admitted commands once with the accepted driver/runtime/helper/cache/guards, jobs2. Result:16 real programs/141PASS0FAIL0ignored and6 compile-only strict commands all exit0; default/SVG/bitmap Controls --lib --tests and consumer --lib --tests plus the9 exact tracked examples compiled. No whole-consumer runtime, GPU, native window or CI ran.

The full changed12-case target passed, including the previously failed intermediate-motion qualification and entirely-offscreen nested Scroll/Disclosure route. The original fixed allocation, strict intermediate progress/refusal, allocation bottom, same-geometry nested Stack control, dual nonzero original offsets, complete actual landing, IDs/mounts, closed exact errors and stale/unknown/numeric contracts were all retained. This demonstrates the original two repairs on the actual admitted program, without relabeling CPU34's10PASS2FAIL or the a883 red runs. The API3/13 prior consumer targets/lib also passed in full. The new12 program was freshly compiled (Cargo fresh=false), SHA17080d90af897ccda6c127646eb305599a22642a8ddff58b135943b274810091; dep-info c7145e7d2030a20ff81902d4e6ba4cee953fddb3f4bc0b6a1265e8d3160c026c; raw stdout6814f4c2dca6961f5843840dd4a59fec697f18a7677a65c8e1ace34e34b86388. Each actual test program/dep-info was archived before the next relink; strict metadata is not called an executed test program.

Main separately accepted qualified reuse of the original CPU34 Controls451 program on e048, SHA1ca6b83dc77f9ccd62cb8cf225e4fd8fb77297738a14c5b9c1f7b048221a0e4b. It is not part of the new141 executions and must not be called a new-source451 run. Relevant library unit paths/planner/projection/primitiveView/token/env/locks were unchanged; the changed private producer is not instantiated by that unit suite. API3 was rerun because metadata input changed.

The driver exited0; owned driver PID/PG24465 and all command processes/groups exited, the build/occupancy guards released, target lock had no holders, compilers were empty, source3618 before/after and cache markers matched. Resource release was handed back before offline analysis; no lease remainder was reused. Original raw results, archived executables and all earlier red evidence remain. Exact22-command receipt and the compact original-source/program mapping are in issue-38/cpu34-continuation-r1-20261101/cpu-run01/DOMAIN-HANDOFF.json (SHA34a92cb1e4012f8cb67de8cf2f9164e909250b715a794c02d0db3faf6f14fb22).

SDK scoped local acceptance is ready for Main integration/delivery. Formal required CI/normal merge/public pin and actual AgentKit product Summary/Sidebar/mint-claim/host/second-consumer acceptance remain separate. Earlier e048 DEV6PASS is preserved on its own program/source and is not transferred to the new source or whole product. AC5/#38 remain OPEN until original downstream consumption and delivery are accepted. No physicalIME/VO/performance/fullGold7/new native pixel gate is introduced.
