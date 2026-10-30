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

Native keyboard/pointer, physical IME, VoiceOver, current native screenshots, published exact pin and downstream AgentKit delegation remain unverified. The excluded Tabs case needs its separately unavailable Content API; the Tree-specific case remains outside this Issue. This is a partial delivery referencing #38, not Issue closure or full five-AC acceptance. Cloud CI belongs to the actual PR head and is reported separately from local evidence.
