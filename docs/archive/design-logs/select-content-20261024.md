> #199 standalone source candidate (2026-10-31): PR200 a883d00 + Search/Select only, roles24->26. Original finite acceptance remains611a/33; this source candidate has not been compiled or run.

> 2026-10-26 主协调当前接收：#89原五项有限验收满足；当前主树569 CPU调用、default48/external96真实GPU与strict通过，413类型分类/feature bindings、Workbench及父票同步。旧R2/R3/失败保持原身份；完整接收见末节与 design/reviews/select89-main-intake-20261026/root-acceptance.json。

# #89 SelectContent：main 接收的有限设计

2026-10-24。main 接收原设计及 amendment1；后附 amendment1 覆盖原稿中不精确的 focus/IME来源、无检查set_labels与overlay屏障描述。具体接口仍须以实际原owner事务实现，先red再改，不允许从focus bool推断授权。语义专用依赖待main去重建票；沿#101原dirty/revoke契约，最终整合等#101实际接收。#89原View seams留在本票；不另造Menu引擎。设计接收不等于运行/实现验收，#89保持OPEN。

# 有限设计：原 Select 的 Composition 适配

## 1. 当前事实与 owner

`widgets.rs::Select` 唯一持有 `selected: usize` 和 `menu: Menu`；`choose` 校验 count、写 index、关菜单，同值选择仍返回 true。`SelectView` 唯一持有原 trigger ButtonModel 和 `MenuView::new_external()`。后者绑定的就是 `Select.menu`，不是第二份业务模型。MenuRuntime 唯一负责选项 hover/press/keyboard arm、open generation、scroll/reveal、popup placement、presence/motion 与行绘制。

`SelectView::render_trigger/render_options` 已允许 base/overlay 分离；`render` 是原 combined convenience。`focus_option/activate_option` 已有原语义操作入口；后者通过原 MenuEvent 调用 choose 并恢复 trigger focus。`set_surface` 只传给原 trigger；MenuView 初始 Surface 为 popover，frame/rows 都消费它。原 `MenuContent` 自有 MenuView、trigger Button、声明树/values/selection 生命周期；把它包在 Select 上将产生第二套原 Menu/trigger 权威，故不这样实现。原 `PopoverContent` 是任意 Content body 的互动弹层，Select 固定字符串行不需要第二壳/host/scroll。

ContentHost 继续唯一负责一棵树的 focus/capture/mount、overlay pass、输入路由、typed action queue 与 native text owner。SelectContent 只是原 Select+SelectView 的 Component 投影，以及声明更新和过期输入的授权围栏；不持有另一个 selected/open/focused/scroll 状态机。

## 2. 有限公开 API（提案，尚未存在）

```rust
pub struct SelectContent { /* private original Select + SelectView and declaration */ }
impl SelectContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self;
    pub fn labels(&self) -> &[String];
    pub fn set_labels(&mut self, labels: Vec<String>);
    pub fn model(&self) -> &Select;
    pub fn set_selected(&mut self, index: usize) -> bool;
    pub fn set_disabled(&mut self, disabled: bool);
    pub fn set_error(&mut self, error: bool);
    pub fn layout(&self) -> &SelectLayout;
}
```

`name` 是可访问字段名，空 name 由现有 Component::validate/EmptyAccessibleName 拒绝；字符串 labels 沿用原 Select，不加非空 label、唯一 label 或业务 value 要求。原模型可只读检视；不提供 `model_mut/view_mut/labels_mut` 破坏前帧内更新围栏。disabled/error 是受控声明，投影到原 setters，不从主题颜色反推状态。文本角色固定原 UiBody，compact=false；如 main 有现成的共享尺寸输入要求再复用，不为了适配先扩展 API。

`set_labels` 同内容是 no-op；不同内容先取消旧 trigger/menu 输入、presence 和几何，撤销旧语义 parts/待交付授权，再替换 labels 与 count。raw selected 不自动改写：原 View 在展示时把索引夹到末尾，空集显示空标题，但原模型并未自动 choose。`model().selected` 仍是真实 raw index；语义 value 对应实际展示标题，选项 selected 仅在 raw index 有效时成立。空集没有有效选中项，disabled trigger，不能制造占位选项。重排/改名/同长度替换均是新声明 revision；不保证实体跨重排保留，业务稳定 ID 不在范围内。

`set_selected` 必须在有效范围内经原 choose；不发用户 Activate，不通过模拟点击/AX 完成。有效程序赋值取消当前弹层输入并撤销旧待交付授权；raw 同值也遵循原 choose 的关闭语义。无效输入返回 false 且不部分修改。公开原 Select/SelectView 仍保留，不移除、不迫使老调用方采用新 wrapper。

## 3. 必须先交 main 的最小共享接缝

### A. SelectView 内部接缝（原 owner）

- 新增 `pub(crate)` 开选项方法，复用现有 pointer_up/key 中相同 show_with_model、selected clamp 和 availability 检查；触发器语义 Press 走此入口。不能由 adapter 直接写 `menu.open=true`，也不能通过伪造 Enter/down/up 偷用 arm。
- 新增 `pub(crate)` 替换选项生命周期方法，绑定原外部 Menu 模型，调用原 cancel/deactivate/clear_layout/clear_presentation 和取消 scroll gesture，更新 count。现有 MenuView::replace_items 断言 `!bound_external`，**不能**直接拿来处理 Select 的外部模型。若需抽取 `replace_items_with_model`，仅抽这一原 runtime 清理操作，给 owned/外部入口共同调用；禁止整块 Menu 重写。
- 原 ButtonModel::measure 是纯测量入口，SelectContent 可直接以当前展示标题 + Body/UiBody 调用；不必新增测量引擎、渲染到临时 Scene 探尺寸。原 trigger layout 在 render_trigger 建立，overlay 使用同帧结果；若 Host 布局后/paint前确需命中几何，则从原 Button/Select layout 抽只读 arrange seam，先写实际失败案例再决定，不能假称现在已有纯 Select arrange API。
- `SelectView` 私有 Menu 不暴露给消费者；语义 Focus/Press、pointer/key/wheel 均继续走原 View。Host 级取消/隐藏必须清除旧布局可交互身份；原布局只读 getter 不成为复活输入的凭证。

### B. 共享语义角色/bridge（需 main 接收后才能实施）

当前 ContentRole、wire SemanticRole、平台 AccessibilityRole 都有 MenuButton/Menu/MenuItem，没有 Select/PopUpButton 专用角色。selected/value/expanded/invalid 字段已足够表达数据，但角色含义需明确。**提议最小新增 `ContentRole::Select`、对应 wire variant 和真实 native popup-select role 映射；最终命名与 native 常量由 main/语义 owner 核实后确定。此处不是已存在 API，也未验证 native 角色支持。** 不用 TextInput/ComboBox 假装可编辑，不降为 Group，不把通用 MenuButton 宣称 Select 专用语义已经合格。

依赖 hunk：controls/composition.rs（含 is_action_owner 的适用性审查）→ app-protocol semantic enum/serde 正反例 → module-runtime content_accessibility 与 container_semantics 双向转换/验证 → platform-macos content_accessibility 和 accessibility native role/action mapping。Gallery actions_acceptance 的手写映射也须逐项审查，不能用 fallback Group 避开 guard。旧 peer 遇到新角色保持拒绝，不静默降级；需要匹配双方 schema 的边界单列。原 Menu/MenuItem 行角色按原菜单契约复用，popup group 若用现有 Menu part，不新增第二 UI 实例。

若 main 决定复用 MenuButton，必须明确接收其作为该有限 Select 的真实语义契约，并留下独立 bridge/value/selected/action 证据；本设计不预先做该决策、不先写生产绕过。

### C. Theme / Popover

**不需要新 Theme 角色，也不需要共享 Popover API 变更。** trigger 从 ContentTheme.surface() 传原 SelectView::set_surface，Palette 原样传；popup 保持原 MenuView popover Surface，不广播父 Surface。原 MenuRuntime 已在同一 surface 上画 floating frame、Quiet rows、glyph/icon/focus。父 clip/viewport 由 Host 给原 render_options，不在 consumer 手算 popup offsets/row heights。若外部 Theme 的真实 popup 色/语义束仍不全，记录原失败交 main 的 Theme owner，不新增局部配色或假 token。本次发现的是语义/原 View 接缝，未发现必须发明 Theme role 的理由。

## 4. Component 与 Host 接入

measure 使用原 ButtonModel::measure 的原标题和布局参数；arrange 接收原 Content 的 frame/clip。paint 先以同一 SelectView 调 render_trigger，paint_overlay 再调 render_options；不调用 combined render 后又重复 options。沿用 Host 的 overlay_open/overlay_presented 分离，关闭后的退出仅绘制，无 input/AX/actions。固定行没有 Content children，因此不滥用为 interactive Content body 提供的 OverlayScope，也不创建新的 PopoverContent。是否 overlay-only 路由能覆盖所有原 dismiss/barrier，要以 overlap/outside regression 证明；若 Host 接缝不足仅交最小失败给 main。

`paint_overlay` 的 c 是 Host 实际 clip，可作为原 visible bounds 和 ancestor clip，沿用当前 MenuContent 的桥接模式；不能把 trigger 小矩形当 viewport。trigger 仍按普通祖先 clip 决定可用性。滚动父节点的 clip 对 popup 是否形成约束按原 Host 路由实际契约，写精确预期与原 View 的相同输入对照；不无条件 escape clip。popup 盖住后绘制 sibling；超界点既不能命中行也不能激活后面的文档。wheel/trackpad 原参数逐字段传 MenuWheelInput；原 MenuRuntime 拥有 scroll/momentum/reveal，consumer 不补 offset。

原 motion_context clock 每帧按已有模式传 set_reduced_motion/tick，needs_animation 请求帧；不新开计时器。Theme/scale/布局变化使原 runtime 取消旧 paint snapshot 的规则保留。trigger focus decoration 如有 Host 末尾层，必须验证只绘一次、遵守原 gutter/clip，不新画焦点圈。

set_focus(true, visible) → 原 focus；false → 原 blur。Mounted/Unmounted、presentation_runnable_changed(false)、window_active_changed(false)、Cancel/disabled 通过原 cancel/blur/window_active 方法收尾并撤销 parts/授权；再次可用需要新输入。焦点恢复始终是 ContentHost 的原节点+mount，MenuEvent 内的整数 0 不能当 ComponentId 使用。正常激活/Escape/dismiss 以原 trigger 为返回目标；隐藏/移除/禁用不能复活它，采用 Host 原失效焦点规则。不 clone Composer 或另建 focus manager。

## 5. 输入、旧能力与 once

- 事件调用前调和 labels/availability；不等待下一 paint 才发现替换。仅 primary pointer 使用原按钮规则，secondary/multi-click按原公共 PointerEvent 适配，不能抹掉 metadata。原 View 拒绝/忽略的键不得合成 selection。
- trigger down/up 和 Enter/Space 沿用原 ButtonModel arm；只开菜单不发 Activate。原 Menu Up/Down/Home/End 调整 active，不提前 choose；Enter/Space 和 pointer-up 最终提交都由原 MenuEvent::Activated→Select::choose 一次完成。adapter **不再 choose 第二次**（当前 Gallery menu_event 有重复 choose，未来该 hunk 只消费动作，不能复制这段）。同值重新确认仍是一条真实 Activate，changed 不能作为去重依据。
- SelectContent 保存受控声明 revision 和 opaque semantic part tokens，令牌来源复用现有 mount allocator/菜单 part 思路；映射只在当前 mount+popup generation+declaration revision有效。part 不是业务 stable ID。close/reopen、相同 ComponentId remount、label A→B→A 不能复活旧 part，旧 semantic Focus/Press 均拒绝。不要只校验 index<new_count。
- pointer arm、scroll gesture 在更新/hidden/unmount/inactive时取消；过期 up 既不能选旧行、也不能开新 trigger。reopen 后没有新 down 的 up 无效。不要只检查最终坐标或 count。若原 runtime 在同长度内容更新的迟到 up 还有缺口，以最小失败修原 cancel/invalidation seam，不新增选择 reducer。
- 只有交付 MenuEvent::Activated 的那次 response 设置 activated=true。建议复用现有 ContentActionPayload::MenuSelection，index/label/revision 在交付瞬间冻结，value_path=None；这是原菜单确认快照，**不**提供 MenuEntry/values API。若 main要求独立 SelectSelection 名称，只做数据变体，不能扩业务引擎。
- 使用原 Host typed queue/drain_action_records（与 drain_actions 为同一队列），不在 paint/focus/tick/查询/model setter轮询发事件。action_payload_current 只验证对应 mount/声明授权，不能因为选择后关闭菜单便撤销刚产生的合法动作；update/disable/hidden/unmount撤销未交付授权按 Host 原队列规则。已 drain 的动作不重放。捕获 label 后更新 labels，不允许动作被重新解释成新 label。

## 6. 混合 Composer / reading 的适用范围

真正 fixture 是原 TextFieldContent/TextSurface + ContentHost text services/原发送 gate，旁边原 ScrollContent/ReadingController；不造“等价” string/marked bool。hover、被动 labels/Theme更新、布局、paint、popup滚动、过期输入必须不改仍获焦 Composer 的 selection/marked/undo/binding/send gate；如果更新使当前 Select 失效，由原 Host lifecycle处理，不能顺便 focus Composer。

用户明确 focus/activate Select 是实际焦点转移，应服从原 TextFieldContent blur/unmark 事务：保留可见草稿，结束 marked，原 undo仍一笔；不能要求交互后 marked 永不变。原 TextKeyStart/End、Preedit/Commit/Unmark 回调只归原 text owner。Select 自身不处理 IME；确认选项不能穿透为 Composer Submit。发送 gate 的 expected 根据真实 composing/consumption状态，不强写一个总为 true 的断言。

阅读在不操作 reader 时保持原 item key/mount/anchor viewport位置及 history/latest 意图；popup wheel 不冒记为背景 reader 用户滚动。显式把焦点移到 Select 时，原 Host reveal 合理影响祖先滚动，单列该例，不错误声称所有 offset 必须恒定。读者显式滚动与 latest 仍按原公共接口测试，consumer 不补偿像素。物理 IME、GUI、VO 未执行；不把合成 marked 测试升级为真实输入法结果。

## 7. hunk 交付次序

1. main/advisor 接收 A/B 最小共享依赖及有限 API；#90 后置，不拉入 async/query/custom Content/stableID。
2. 独立 candidate 只在获实施授权后产生：原 SelectView 开菜单/撤销 seam，必要 MenuRuntime 外部模型共享小函数；原 View 红例与回归先行。main 串行整合 layout_view.rs/composite.rs，保护其余 WIP。
3. 语义角色/wire/native 双向 hunk 由 main 指定 owner串行接入，不先假角色；Theme不变。若未接收此依赖，adapter阶段保持阻断，不能说全部公共/AX完成。
4. composition/select.rs 新 adapter 与现有 exports；composition.rs共享 hunk最小；typed queue 复用。只读设计没有这些文件。
5. independent consumer fixture/tests → 原 Gallery workbench Select 路由迁移（去重复 choose、保留原对照）→ inventory/current audit/design/父票同步。design/remediation/composition-audit-current.json、design/gallery/capabilities.json、design/gallery/workbench.json、docs/design 当前文档等先查实际库存再由 main 接入，不能改历史冻结证明或纯换 SHA 求绿。
6. 最终精确源窗口/程序/log/hash/原失败与分层结果回填 #89、父 #56 和关联 #55/#65（由 main操作）；有 PR 才贴真实链接。不因本适配重开 #4/#36，不将 #60 当 adapter 交付。


# 三项修订契约（覆盖原稿对应不足）

## 1. 按来源授权，菜单/scope开启前完成

当前原源码：ContentHost::focus直接调用focus_target；普通focus_target校验active/scope/disabled但没有统一is_composing检查。enter_focus有composing屏障，Tab fallback先把键交原editor并保留composing屏障；AX跨owner的Focus/SetSelection/SetExpanded，以及会转移焦点的Press，在交给组件前受原is_composing规则阻拦。普通pointer/focus路径另循既有行为。不能把这些统写成“选择Select一定结束marked”，更不能为保证一个测试通过而擅自统一所有来源。

| 来源 | 授权/转移与结果 |
|---|---|
| Tab/Shift-Tab、enter_focus | 先原editor/key与composing屏障，获准才转移；拒绝时不能开popup/取得scope/改变Select selection或editorfocus/marked |
| AX跨owner Focus/SetExpanded/Press | 先原AX action门禁；辅助preserve-focus例外只按原声明，不给Select伪造豁免；拒绝无popup/focus/scope/selected副作用 |
| 普通primary pointer | 保留原pointer路由、activation-focus来源与原blur/unmark行为；不要套AX屏障，也不要绕开可能由#98正式接收的共享gate。获准后才允许原View执行open/commit |
| 普通Host::focus/恢复焦点 | 按原显式focus契约；恢复仅有效原owner/mount。focus的bool当前表示是否变化，不是通用“授权成功”票据；already-focused返回false不能当拒绝，非法目标也返回false不能当允许 |
| 已在Select内的合法keyboard/option操作 | 验证当前mount/revision/generation和原可用性，再走原View。合成Preedit/Commit只归真实text owner，Select不另处理IME |
| 被动labels/theme/paint/hover或旧能力请求 | 不请求跨owner转移；不能改变仍获焦Composer的marked/selection/binding/gate或打开scope |

**严格顺序**：解析原输入来源/目标/mount与当前scope → 原owner的既有gate检查（及必要身份容量预检）→ 接收转移意图/原focus事务 → 原SelectView open/commit → Host原overlay路由/语义发布。门禁拒绝时Select selected、popup open/generation、focus/scope、旧授权/queue均不变。不得先route导致MenuView show，再调用Host.focus发现失败后兜底；这种回滚已触发scope/blur/selection等不可冒称无副作用。

当前接口不足以让组件在open前取得同一次原Host授权时，先以反例定位，向**原Host/menu owner**提最小接缝（区分accepted-no-change、accepted-transfer、rejected，具体形态main决定），不能从focus bool推断。这个接缝承载原gate和输入来源，不新增Select独有composing判断/focus引擎/授权队列。#98接收后同一次原gate成为权威，Select仅调用，不复制分支或继续使用旧私有preflight。不存在已实施的通用gate声明。

mixed Composer验收按来源分别列期望：Tab/AX被拒时marked/selection保留；普通pointer/focus若按原路径合法转移，才按原TextFieldContent blur/unmark事务核对可见草稿/undo。任何选择确认不得穿透为Composer Submit。保留旧稿相应统一描述作为已纠正的历史，不再作为断言依据。

## 2. checked更新：预检全部成功后才撤销与替换

修订提案签名（尚未有生产API）：

```rust
pub enum SelectUpdateError {
    IdentityExhausted,
    // Any actually necessary additional checked error must be justified first.
}
impl SelectContent {
    pub fn set_labels(&mut self, labels: Vec<String>)
        -> Result<bool, SelectUpdateError>;
}
```

`Ok(false)`相同声明，无操作；`Ok(true)`一次原owner提交；`Err`保证原labels、raw selected、Menu模型/count/open、popup/arm/scroll状态、旧parts/授权、pending action queue均不变。所有可失败的声明revision checked_add、popup generation预算、part token总量/usize转换/命名空间范围/身份预约检查，**先于**cancel/replace/clear parts/撤queue。不能先close再发现revision溢出，不使用会panic的next_mount/expect作为checked保证。一般进程OOM不是此API虚构的可恢复承诺，显式单调ID/容量错误必须覆盖。

步骤：

1. 判断完全相同声明；计算所需revision/part身份空间与转换上限。
2. 在原owner内准备完整新声明及checked身份计划/预约；原模型/公开状态尚未动。若共用单调allocator允许保留已消耗预约，其不回绕/不重用，不能通过回滚全局计数复活旧token；失败不更改组件及队列。
3. 所有可失败前置完成后，在同一次可变owner操作中调用原SelectView checked replacement seam，取消原arm/scroll/presence/geometry，再替换labels/count并切revision/parts授权。不安排后续可返回Err的身份分配，不异步等待，不插事件/paint，不建补偿队列。
4. 成功的新声明撤销旧未交付payload授权；同值保持旧queue。具体queue仍由原Host drain/filter承载；Select仅提供原action_payload_current所需授权检查，不维护第二queue。

原MenuView::replace_items owned-only，不能套外部Select.menu；必要共享外部模型cancel/replace在#89原View hunk中解决。受控selection setter仍调用原choose、无用户Activate；labels缩短不改raw selected，原展示clamp保留。程序setter不是新reducer。

### 身份分层，不互相替代

| 变化 | 原身份权威与授权核对 |
|---|---|
| close→reopen | **原MenuRuntime open_generation**；新的opening分配checked generation后才show，旧popup part无效。closed paint不产生新input授权 |
| 同长度替换/改名/重排；A→B→A | **声明revision**；不靠count/最终label相等。有效part映射同时记录其opening generation，不能回到A复活旧part |
| unmount→同ComponentId remount | **原Content mount**；由原Host挂载/回调/queue过滤，不用声明revision假扮mount |
| 正常选择后close | popup input capability结束，但该次合法交付的action payload继续属于其已接受mount+声明授权，能drain一次 |
| 后续labels替换/programsetter重授权、disabled/hidden/unmount | 撤销旧未交付授权，原Host filter拒绝旧payload；已drain不重放 |

opaque part token只是当前映射的外部句柄，需要分别关联mount/revision/open_generation。复用checked分配工具须由原owner保留上述关联，不能泛称“借mount allocator就安全”，也不创建新选择/队列系统。初次mount/open同样须checked capacity preflight；失败不改变旧已挂载实例、不发布部分parts，不把新失败视作“成功但没有AX”。generation耗尽不能回绕或在取消后才报错。

## 3. 开启前原focus/scope授权；开启后原overlay屏障

沿原Menu/SelectView拥有的popup与Host overlay流程接入，授权在open前完成。Select只有原固定Menu行，没有任意Content子树；不能为取得屏障再包PopoverContent或第二ContentHost，也不能伪称原Menu overlay等同已接收的interactive Content OverlayScope。这里的“scope”同时要求核对Host当前实际输入/焦点/AX约束；若原Menu接口表达不足，最小补接口仍归原owner。

获准open后，必须实测popup盖住后绘制sibling时pointer down/up、outside down/up、wheel/gesture和Enter各路径：命中popup只交原MenuView，outside-dismiss的down/up不能落给背景按钮/reader；wheel在popup区域不滚背景；Enter确认不Submit Composer/激活背景；未处理事件是否能bubble按原Menu契约核对，不用consumer手工总吞事件作补丁。capture结束与popup关闭的同一次release不能重新arm trigger或穿透。

关闭可保留原outgoing presence，**仅绘制**：无旧part/AX操作、无hit/capture、无scope占用或用户action。原关闭后已有合法action仍可drain一次，与禁止退出再激活并不冲突。hidden/unmount/windowinactive按原owner撤销，不能用透明覆盖层挡输入或用退出paint状态当open。

Gallery迁移只消费原Activate记录；删除menu_event对Select重复choose，原SelectView仍choose一次。同值确认仍一次真实动作，programsetter无动作。raw selected不随labels缩短改写；这三条保持原稿，并在新验证中明确。


# amendment1新增/纠正验证（全部待执行）

保留原validation-plan的原View/publicconsumer、默认与外部Theme、@1x/@2x真实GPU正向文字ink/父clip、原Select回归、strict与程序/精确源要求。以下修订不能以Scene几何断言替代真实输入/焦点授权。

## 来源门禁：open之前验证，拒绝无副作用

用同一原Host真实Composer/TextSurface marked+UTF16 selection+sendgate，分开测Tab/Shift-Tab、enter_focus、AX Focus/SetExpanded/Press、普通pointer、显式focus，以及已focused的no-change情况。先记录原focus/mount/epoch/binding/selection/marked/undo/gate、Select.selected/menu.open/原generation、scope、capture、parts和action queue。

- 原门禁拒绝来源：以上所有可观察状态不变，popup从未open、scope从未取得，事件期间也无短暂blur/unmark/原show调用；仅比较最终close后的状态不足。通过原接缝可观察结果/定向单测证据确认顺序，不加生产日志或consumer补偿。
- 同一目标已focused返回false与非法目标返回false，不能都当原授权失败/成功；分别有反例。
- 普通pointer/显式focus按原路径接受时，预期与原契约对照，不能套Tab/AX的composing屏障；合法blur/unmark按原事务验证。#98接收后重新冻结实际gate版本并沿其原行为，不复制Select特判。
- 旧AX part/迟到pointerup、被动更新/hover不能借跨owner操作；Enter不能穿透Composer Submit。以真实marked状态/发送策略验证，不伪造composing bool替代。

## checked replace与身份容量

为每一种显式失败点建立受控边界fixture（若已有原allocator失效注入，复用；不改生产上限求绿）：声明revision checked_add耗尽、原open_generation耗尽、所需part token数/usize转换容量失败、身份预约失败、部分预约后失败。错误必须在任何cancel/clear/replace/queue撤销前返回。

断言失败前后原Select/Menu模型、labels/raw selected、popup布局/armed pointer/scroll、旧part授权与pending action记录完全保留，原旧合法操作仍能继续；不能只断selected。allocator不回绕/不重用，预约被消耗不要求回滚全局counter。相同声明Ok(false)无分配/撤销；成功Ok(true)一次原取消/替换、count同步、revision前进。程序set_selected不发userActivate；raw index越界保留但展示clamp。

分开验证原generation(close/reopen)、declaration revision(同长替换及A-B-A)、原mount(同ID remount)；旧parts/旧up分别拒绝，新input有效。正常选择后close的合法action drain一次；后续labels替换/disabled/unmount使旧未交付授权过滤。复用原Host一队列，不写回调缓存。

## 原Menu overlay屏障实测

原SelectContent/Host与后绘制的按钮、真实Composer、可滚reader重叠；获准open后逐项输入：popup内down/up、outside down/up、trigger重复点击、wheel/precise/momentum、Enter、Escape、Tab。分别断原Menu选择/关闭/恢复的结果，以及后sibling无激活、reader未被菜单wheel误滚/记history、Composer无Submit。capture release与close同一次事件不穿透。

退出presence真实图像仍可有，退出期间旧part/AX/命中/动作/scope均不可用；合法已生成action不因退出被误删。真实GPU保留后sibling遮挡/父clip/文字正向ink证据，但不以像素遮住替代路由不穿透。

## 共享语义依赖票的独立反例

真实旧schema decoder + Select新snapshot整份失败；旧MenuButton/Menu/MenuItem编码roundtrip不变；真实native投影等于SDK PopUpButtonRole（不是MenuButton/ComboBox）。A合法快照→malformed/unknown B→owner dirty或validation revoke；B无发布前缀，A旧callback与textlayout不可授权；容量失败dirty/不回绕；合法C恢复/断连hidden/inactive继续拒旧目标。保持邻居Composer binding/marked/selection/gate，不全Host.cancel。区分远端fail-closed与本地checked set_labels失败保旧状态，不能互相套用。

本轮无运行、无结果数字、无生产hunk。后续仍用原build_lock/environment与review-cache、+1.95.0 --locked --offline，不newtarget/prepare/clean/install。新增View seam归#89同票，不扩为额外View依赖票；语义桥依赖票通过后才解除该部分门禁。


## Main #105 finite shared Select-role acceptance — 2026-10-24

The original five #105 criteria are met in current HEAD+preserved WIP. Main integrated21 necessary paths preserving95/101/106 and107 roles. Root10 selected lanes passed:557 ordinary test invocations plus original Select289/Selection391/Search337/Link6 native-object checks, scoped strict and Gallery. This includes a real closed Select test-only owner receiving public native/AppHost Press: source gate rejects marked/inactive/foreign-mount before callback; accepted transfer and same-owner/no-change open the original SelectView once, expanded reflects actual model, no option-selection action is duplicated. The fixture has no independent focus authorization guard masking an invalid Host callback.

Root receipt design/reviews/select105-main-intake-20261024/root-acceptance.json records actual source/program/depinfo and original criteria mapping. Formal public enum rationale and Workbench role notes synced; SelectContent API remains absent pending89, no new route/publictype count. Actual old complete schema and original dirty/revoke/capacity fences passed; no remote-invalidB retains old A authorization.

Preserve failed follow-ups: initial new fixture API compile errors, then full Host Debug showed only R1 reveal input clock2->3 on refused AX. Advisor confirmed whole Host byte equality was an extra condition; final test checks explicit original owner/editor/focus/binding/capture/help/actions and real callback/open sequence. Current rejected AX can invalidate an old reveal plan via this clock; this is retained evidence under98, not hidden and not a claim of strict reveal/whole Host atomicity. No production fix was made for this test.

#89 adapter/checked updates/overlay/GPU,98 prepared strict actions and111 remote geometry remain independent. NSWindow/physicalIME/VO/performance/publication/pin not verified; PR null. Canonical capabilities/old bindings/observations remain their historical source identity; current normal8 refresh is still required.


## Main ordinary Ready bubble compatibility decision — 2026-10-25

Main/advisor reviewed the R1 Select candidate and found a real ordinary bubble compatibility defect before live intake: inner TextContent ignored Ready plus an outer explicit StableHooks owner with default Unsupported must still reach its original ordinary handler. StableHooks does not promise Ready for every verb. Returning Some(default) on that needed ancestor silently swallows the old route.

Prepare only the actual bubble chain chosen by original finish_bubble; stop at its original stopping point, without querying unreachable ancestors. Cache each decision exactly once. Before all needed decisions are prepared, run no effect/focus/capture/queue writes. An ordinary needed Unsupported returns to the original ordinary route after its existing admission checks; a strict Unsupported refuses. Final validation/identity/capacity failures never become fallback. A full Ready chain retains original final validation/focus admission and child-to-parent consuming apply. Never apply a child then discover unsupported/replay the old handler.

Resource boundary: Select preparation may use original Parts::reserve/NEXT_MOUNT checked allocator to reserve unpublished identities. Main accepts the original documented irreversible unused holes, not reuse/rollback. Describe this as no business owner/queue/focus/capture effects before admission with possible unpublished allocator consumption, not completely global-state-pure or counter-unchanged. Final validate re-evaluates the original View decision without another reservation. No duplicate prepare to probe support and no new public API required merely to prevent such duplicate calls. An ancestor whose decision depends on a preceding mutable child callback remains Unsupported and ordinary takes its original route.

Required finite regressions: actual external inner ignored Text plus StableHooks/defaultUnsupported parent executes parent ordinary handler exactly once with old response/capture/action; bubble cutoff does not query unused ancestor; allReady child-parent prepares once/applies child then parent once; strict sameUnsupported chain zeroapply; explicit reservation-hole accounting if used. Keep ordinary direct/pointer marked blur-unmark and sameowner AX text/range authority; no new geometry gate. Preserve original Select rejection/once/overlay/checked declaration cases, current Radio/Search/clock/ComboBox branches, relevant regression/strict and exact source/program/depinfo windows. Original sealed336/default48/external96GPU stays its original window; do not re-sign it as corrected Host evidence. #89 remains open until corrected actual main intake; #98virtualRow/#111 are independent. No new transaction engine, fullDS/native gate or governance Issue is added.


## Main ordinary action and focus-route support clarification — 2026-10-25

Prepare only the actual action bubble chain, cache each decision once and stop at the original finish_bubble cutoff. An unreachable ancestor has no action-support or compatibility query. The cached decision may independently require focus; old/new focus-route hooks still need their original StableHooks support even when an ancestor is not an action recipient. Before final validation, ordinary required Unsupported/Unknown capabilities discard unapplied decisions and return through original admission to the legacy handler; strict and existing Combo paths refuse. After final validation begins, identity, capacity, stale geometry and admission failures never fall back.

The ordinary_unknown_ancestor positive requests Owner focus through an Unknown ancestor, so its original legacy child event, zero prepared apply/validation, once-only business action, focus, capture and queue assertions remain. Its prepare count may change from zero to one to discover the focus intent before support classification. The nonfocusable cutoff positive requests Focus::None, so child prepare/apply run once and unused parent compatibility/prepare queries remain zero. This is a finite source-contract clarification pending actual main verification, not a new API or whole-library acceptance gate.

## Main original five-criteria acceptance — 2026-10-26

The retained SelectContent/SelectStoryHost route uses the original Select/SelectView/Menu reducer, fixed labels and original popup lifecycle. Raw selected values are not rewritten on shrink; checked updates preflight before business effects, and queues retain index/label/declaration identity. Existing unpublished reservation holes remain irreversible. Ordinary preparation follows only actual bubble participants; actual focus-route hooks are independently checked before final validation. Required Unsupported/Unknown takes original ordinary admission/legacy with zero prepared apply; strict and final-validation failure never fall back. Root AX Focus retains original Host admission.

Main integrated 17 exact paths preserving unrelated WIP; it removed one candidate-added getter identical to the existing method and two test-only redundant casts. Actual CPU phases total 569 passing invocations, including 392 library, new public bubble/root-focus and Select cases, prior focus/reveal/Radio/Segmented, original Menu/trigger/placement, Panel and Gallery neighbors. Two existing unrelated GPU tests remain ignored under their original scope. Controls/Gallery and finite consumer strict pass. Default 48 and two external definitions 96 actual GPU cases compare original View and real Host open/exit pixels with positive CoreText glyph contribution and original Surface/clip; the external artifact was freshly produced from 36 stable source/fixture inputs, independently locked outside its artifact.

Audit: `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-89/main-acceptance-20261026/audit.json`; main receipt: `design/reviews/select89-main-intake-20261026/root-acceptance.json`. All image/program/library/dep-info/generated archives were rehashed. Old private R2/R3 and main failures keep original identities; old missing generated/samefile receipts are not reconstructed from mutable caches. Current public classification/bindings cover 413 types. Full capability regeneration still rejects historical observation source pins; guards and old evidence are unchanged, with global DS freshness tracked separately rather than added as a new #89 gate.

No NSWindow, physical IME, VoiceOver, global performance, full Design System closure, public revision, commit, PR or push acceptance is claimed.

Main closed [#89 under its original five criteria](https://github.com/TekesApps/TekesUI/issues/89#issuecomment-6400630153); later shared changes remain separately tested under their own source epochs.


## 2026-10-30 current-main recovery note (candidate; not accepted)

Original #89/#105 are closed with finite evidence attached to their own historical source/runtime. Current MAIN at `4815c5abca796820f4d906858ea42c7dceb8e690` does not export `SelectContent` and has no `ContentRole::Select`; therefore those receipts do not establish this recovery candidate's compilation or acceptance. The historical adapter and three independent-consumer tests are restored here as candidate material. Current MAIN's `SelectView` retains the legacy View path and lacks the checked prepare/apply/replacement APIs required by this adapter; the Select owner must review and provide that narrow seam before integration. `MenuSelection` and shared action queue already exist on MAIN and are reused. The Select role bridge is also absent from current controls/protocol/runtime/macOS projections and remains with the existing AX owner. No Rust build, behavior test, GPU run, strict check, native input, VoiceOver, or performance verification was run for this recovery candidate.


## #199 当前主线恢复接缝（2026-10-31）

本轮独立来源基于 02456a6 的已接 Combo/Text 与 Combo-only Menu 修复。只从原 Menu owner 已冻结 deferred source 重接 Select 的原类型、checked prepare/apply/replacement/runtime helpers 与对应测试；不把无本域 caller 的 header 接口作为共同前提。Select 的 SetExpanded 被纳入原 prepared AX 路由，沿既有 Host focus/reveal admission 在 open 前授权，保留 root Focus 和同 owner 路径；Composer marked 拒绝和正常许可的公开回归尚未运行。原 View/model、once payload/旧 popup/迟到事件拒绝和原有限 AC 不变；本说明不代表编译、行为、GPU、native 或公开交付通过。


## r4 接纳前缀修订（2026-10-31）

本轮以实际冻结 `8f274af682289b92400a29187c04bcfed3288c57` 的22-role前缀为准，仅新增 SearchInput/Select 至24角色。保已接Combo/五Selection角色、三Selection payload、Host退役与文本能力规则，以及SetExpanded和当前Combo can_show_menu fixture。旧r3全角色并集及其派生decoder不作本轮输入；旧peer直接捕获该22-role的完整生产decoder。主体/消费者/原业务算法及原AC保持，本轮仅重基必要接缝，仍未编译或执行行为验收。


## r4 实际接纳范围（2026-10-31）

本次候选固定在 `8f274af682289b92400a29187c04bcfed3288c57`：保留实际22角色及现有Combo/Selection，只新增SearchInput与Select至24角色。不采用r3的完整角色并集或其派生旧decoder。兼容fixture直接捕获该前缀的完整生产decoder，保留全部22角色和SetExpanded；只为同目录编译改一处module路径。原主体/consumer/Gallery及有限AC延续，仅必要接缝重基和已发现的Gallery未定义变量一行修正；未编译或执行行为验证。


## #199 finite recovery and blur/window authority — 2026-10-31

Main accepted the finite component at immutable `611a1f4ba08e53a60258259812b0dbe9e5337659` (33 roles), recorded in `/Volumes/HP P900/DEV/TekesUI/artifacts/delivery/integration-executor-20261030-09/search199-integrator-acceptance.json`; earlier historical source/status/results above retain their own identities. Original Select/SelectView/Menu remain sole selection/popup owners.

Focus departure and real window deactivation are distinct original operations: `SelectView::blur` clears presentation, blurs the trigger and calls existing `MenuView::deactivate_with_model` to retire interaction/close the model, preserving actual window authority. It does not use `set_window_active(false)` for ordinary focus departure. Existing SelectContent parts retirement, true window inactive/hidden clip/cancel paths, Menu inactive check and pure prepare behavior remain. Ordinary public `focus(None)` → `focus(SELECT)` → root AX Press needs no intervening render under the original LogicalFocus contract.

The new public `public_blur_refocus_reopens_without_a_render_and_retires_old_parts` actually passed at 611a both the exact test (1 pass) and within the full public Select target (11 passes): initially closed and previously open, immutable model/parts observations, old parts rejection/new identity, one original option payload and real window inactive refusal. The new-case runtime red on old 70ee is **NOT_RUN**; no old-baseline red was fabricated. r13 full two-path patch was admitted once; r12 was not stacked.

CPU06 passed 11 commands / 153 invocations/0 failures with 2 strict commands, including original View30/Menu30/privateSelect14/mixed8/Host+IME contracts. GPU default 48/external 96 receipt rows+PNG passed full original View/actual Host open/exit RGBA, original direct glyph > 25 and exit input parts 0. Domain total 3952 PNG all mapped/reviewed; no native-input claim.

Source/program epochs are mixed deliberately: Search24/prepared5/schema17 and bridge checks retain their actual b222 identity and unchanged-input mapping; Gallery/default-review Search16+Select5 retain 70ee identity; CPU06 and all four GPU lanes bind 611a. Historical failures and programs are unchanged; no combined fresh full-suite or publication is claimed.

Existing identity/route remain `tui.select` / `component-select`; the primary API remains `tekes_ui_controls::Select`, and its existing Select/SelectView Workbench `api_bindings` remain unchanged. SelectContent is only a `component-adapter` owned by `tui.select`, with SelectUpdateError as `component-support`. The adapter is separately listed in source-audit `protocol.public_adapters`; neither adapter nor support enters Workbench `api_bindings`. No full Gallery route migration is claimed. Metadata is not applied here. The canonical-doc candidate, seven-path map, all 14 effective criteria and publication delta are at `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-199/finite-closeout-20261031-r15/`. Main reports first-three-domain [PR200](https://github.com/TekesApps/TekesUI/pull/200) at `86c50`; this is not #199 publication or a new611a evidence identity. Later compatible merge/metadata/parent/PR/CI/publication remain pending; native window/OS input/physical IME/VoiceOver/performance/live child IPC/full Gold7/global CI unverified. Original #89/#105 remain closed; #199 stays open.


## Standalone PR200 source intake — 2026-10-31

PR200 merged as `a883d00a29b98cdfb5262c23a2e728e6908384e0` and excludes #199. This private source-only candidate adds exactly SearchInput/Select to its actual24-role prefix (candidate26), retaining all published primary APIs/bindings and existing role contracts. It carries the original Search editor/Button and Select/View/Menu implementation plus the already accepted finite fixes; no Collections, Media or unrelated AX role body is imported. The whole old-peer Reply/Input decoder is newly captured from actual PR200 production24, with only its sibling module path adapted. Old611a33 decoder results are not re-signed as this candidate26 run.

Original CPU06 153 test invocations/two strict and GPU01 four tests/3952 reviewed PNG remain under their actual611a identities, with earlier b222/70ee results unchanged. This standalone source has only patch/identity/static checks, no current compiler/behavior/GPU result. Original blur/window authority and the public no-render regression are included; old70ee added-case red remains NOT_RUN.

The complete source/dependency/provenance packet is `/Volumes/HP P900/DEV/TekesUI/artifacts/issue-199/pr200-standalone-source-20261031-r17/`. Main may admit the source into the registered publication tree after exact-before review; metadata/source-audit/compiler-bindings are supplied by its separate owner, preserving the seven correct categories and existing primary bindings. Settings may be combined through shared-file integration; Table/Media full-domain acceptance is not a prerequisite for this #199 source cohort. Current PR200 Quiet-disabled palette/input drift and a focused SearchField native-text positive-fixture follow-up are separately recorded, not mixed into the accepted finite runtime identity or disguised as passing. No PUB/MAIN/Cargo/GPU/cache/CI/native writes or publication occurred here.
