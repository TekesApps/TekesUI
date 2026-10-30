# #91 selection adapters：main 接收的有限设计

2026-10-24。优先级：末尾 main corrections > amendment2 > remote failure amendment > original design；历史未验声明留作设计阶段记录。仅接收设计及拆分，不声称实现。

# Issue #91：三个原选择控件的有限 Content 设计候选

2026-10-24，Asia/Shanghai。状态：**设计交接，待 main + advisor 裁定与建票；未获实施接入**。

唯一代码源 R=`/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD=`1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` 加现场 WIP。只在本 QA 目录写设计与取证。没有改 live、正式 docs、Issue、公共 API、target；没有构建、GPU、GUI、委派、联系其他会话、提交、发布或清理。最终交接由 main 读取本文件；本会话不向其他聊天发消息。

## 原五项与本交付的对应关系

2026-10-24 只读初读 [#91](https://github.com/TekesApps/TekesUI/issues/91)，open，0 comments；交接取证复核时为 1 comment，原五项正文未变。新增 [主协调分派评论](https://github.com/TekesApps/TekesUI/issues/91#issuecomment-6381512257) 确认本独立有限设计目录、#101 边界和公共接缝接收后串行整合，未启动生产/构建；`issue-91-read.json`、`issue-91-comments-read.json` 保留本次只读结果。父 #56，关联 #55/#65。原验收逐项保留：

1. 先设计三控件各自有限 API/验收，再按实际共享依赖拆实施子任务；分别复用 Checkbox 三态及 ButtonModel 几何、RadioGroupView roving selection、SegmentedControlView focus 与 selected 区别，不以 Switch 替代 Checkbox。
2. 三 adapter 各自独立源/consumer 断言；原 disabled/keyboard/pointer/cancel/状态更新和唯一 reducer 保持，stable mount/子 part 动作 once、卸载/重挂/旧 capture 拒绝、Theme/Surface/clip 继承。
3. 设计并交付最小正确 ContentRole/共享语义/协议 role 与 Mac bridge 兼容扩展（当前缺 Checkbox/Radio/Segmented 适当角色），不以 Group+selected 或 Switch 伪装；旧 peer 未知 role 不得无条件升级授权。协调 #5 但不要求 #5 整个 VoiceOver 先完成。
4. 独立 public consumer、原 View 参照、默认/外部 Theme/必要实际 GPU 与三控件各原回归/strict 通过，真实程序/源/hash/未验分层回填。
5. 公共库存/Workbench/设计/父票同步。原真实 Composer/reading 按适用范围保持，无新业务选择引擎。

本文件的 API/行为对应 1–2，角色与兼容计划对应 3，验证矩阵对应 4，串行 hunk 计划对应 5。五项均**未完成实施验收**，不可用本设计关闭 #91。

## 三控件：先定各自 API 与原验收

下列符号都是拟议草图，不声称现场已存在。三个文件独立：`composition/checkbox.rs`、`composition/radio.rs`、`composition/segmented.rs`。不要继续向 shared `adapters.rs` 堆三个实现。它们共享 Component/ContentHost 既有桥接机制，不创建通用 ChoiceEngine、第二 reducer、第二 native 控件或通用选择命令框架。

### CheckboxContent

真实源：`widgets.rs` 中 `Checkbox { button: ButtonModel, checked: CheckState }`，`measure`、`render_on_surface`、`pointer_*_at`、`key`、`toggle`。CheckState 是 Unchecked/Checked/Mixed；toggle：Checked→Unchecked，另外两态→Checked。按钮 disabled **或 loading** 拒绝 toggle。布局是完整行，原 ButtonModel placement 与 paint/clip 同源；16pt mark、6pt label gap、第一行对齐和换行测量不重算成另一套尺寸。

```rust
pub struct CheckboxContent { /* private Checkbox + label + text role + last delivery */ }
impl CheckboxContent {
    pub fn new(label: impl Into<String>, checked: CheckState) -> Self;
    pub fn from_original(label: impl Into<String>, model: Checkbox) -> Self;
    pub fn model(&self) -> &Checkbox;
    pub fn set_checked(&mut self, checked: CheckState); // external update; no action
    pub fn set_availability(&mut self, disabled: bool, loading: bool);
    pub fn set_error(&mut self, error: bool);
}
```

不开放 `&mut Checkbox` 绕过可用性/取消入口；`from_original` 移入原 owner，先取消旧 gesture/清旧 placement/blur，保 checked/error/disabled/loading 等受控数据。输入返回值直接取原 pointer_up/key/toggle；不能先 toggle 再调用原 pointer_up 导致两次反转。Space 的 down/up/arm/重复遵循 `Checkbox::key`，Enter 不自加 Checkbox 激活。Hover/press/focus repaint 的 changed 不等于动作。

有限原验收：三态读取/外部设置；Mixed 指针或有效 Space 激活一次成为 Checked；Checked 下一有效激活为 Unchecked；拖出/取消/失活/禁用/loading 不变；整行含 label 命中与祖先 clip 一致；无效 frame/完全隐藏清旧 gesture；focus ring 与实际 ButtonModel 一致。单个 Checkbox role 根节点，无 mark/label 的第二 action part。AX Press 用原 `toggle()` 一次；AX Focus 走 ContentHost；不创建 Toggle/SetValue 字符串解析协议。

### RadioGroupContent

真实源：`extended::RadioGroup` + `selection::RadioGroupView`。model 只有一个 `selected: Option<usize>` 和原 enabled slots；View 保 focused/hovered/pressed/window_active/disabled/error。`focus(model,index,visible)` 可只移动焦点，不能把该请求变成选择；整体禁用时原函数拒绝迟到焦点。箭头/Home/End 在 **key-down** 从实际 focused 调原 `key_from`，含原 wrap 与 disabled skipping；Space 在 **key-down** 选择，不强改成按钮 release 语义。

```rust
pub struct RadioGroupContent { /* private RadioGroup + RadioGroupView + fixed labels */ }
impl RadioGroupContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self;
    pub fn from_original(name: impl Into<String>, model: RadioGroup,
        view: RadioGroupView, labels: Vec<String>) -> Result<Self, SelectionContentError>;
    pub fn model(&self) -> &RadioGroup;
    pub fn view(&self) -> &RadioGroupView;
    pub fn set_selected(&mut self, selected: Option<usize>) -> Result<(), SelectionContentError>;
    pub fn set_item_enabled(&mut self, index: usize, enabled: bool) -> Result<(), SelectionContentError>;
    pub fn set_disabled(&mut self, disabled: bool);
    pub fn set_error(&mut self, error: bool);
}
```

固定顺序/固定数量/固定 labels 的有限列表；无增删、过滤、重排、异步装载、业务 key/store。构造先核 labels.len==model.len，selected 为 None 或 enabled 的合法槽；空列表合法但无 focus/action/part。`set_selected(None)` 使用原模型支持的无选择状态；Some 必须合法可用，不靠 `RadioGroup::select(false)` 混淆非法与无变化。外部 set 只改变受控值，不自动聚焦、不发 action；不复制导航算法。`set_item_enabled` 先验证 index，然后调用原 model.set_enabled（原已选项禁用可能自动找下项），并立即取消 View 旧 gesture/重新核实际焦点；不等 re-enable 后 render 才发现 disabled 曾发生。禁用/恢复不得遗留迟到焦点。

有限原验收：独立键盘/指针/AX 脚本对比原 model+View 的 selected/focused/SelectionEvent；空/None/全禁用/中间禁用/末项禁用；方向 wrap、Home/End、Space down/up；只有 `SelectionEvent::Changed` 产生一次 Activate，同一已选项的 Handled 不发选择动作。根 RadioGroup role，原 clipped row 是 RadioButton part，索引是固定 model slot；只发布可见 part，focused 读取 view.focused。Focus(part) 调原 view.focus，不改变 selected；Press(part) 核 visible/enabled 后 cancel 原 gesture、model.select 一次并按原焦点策略 view.focus，不能再伪造一对 pointer events。根 Press、越界/隐藏/禁用 part 及文本动作拒绝。

### SegmentedControlContent

真实源：`extended::SegmentedControl` + `layout_view::SegmentedControlView`。View 持有原 ButtonModel 数组、focused、pressed、space_armed；selected 只在原 model。布局和命中 slots 仍按索引，实际焦点 segment 最后绘制，避免邻项背景覆盖焦点环。方向键从 focused 而非 selected 起步；selected 未改变也可发生焦点移动，不得把 false 当 unhandled。

```rust
pub struct SegmentedControlContent { /* private original model/view + fixed labels + compact */ }
impl SegmentedControlContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self;
    // The adapter constructs the original View with exactly model.len() buttons.
    pub fn from_model(name: impl Into<String>, model: SegmentedControl,
        labels: Vec<String>) -> Result<Self, SelectionContentError>;
    pub fn model(&self) -> &SegmentedControl;
    pub fn view(&self) -> &SegmentedControlView;
    pub fn set_selected(&mut self, index: usize) -> Result<(), SelectionContentError>;
    pub fn set_item_enabled(&mut self, index: usize, enabled: bool) -> Result<(), SelectionContentError>;
    pub fn set_disabled(&mut self, disabled: bool);
    pub fn set_compact(&mut self, compact: bool);
}
```

`from_model` 而非随便接一个 View：原 render 的 count 是 model/labels/buttons 三者 min，私有 buttons 数量目前无公共查询，不能静默少渲染。用原 new(model.len()) 构造唯一 retained View；不为这个 constructor 扩展全库 View 生命周期 API。原 View 消费者仍是对照参照。空/全禁用合法但没有可激活 part；不能把全禁用时遗留 selected 数字制造成可操作选择。非空构造 selected 必须在范围内；禁用选择显示/语义按原模型保留，不擅改 model 的 fallback 策略。外部 set_selected 先核 enabled/范围、无变化返回成功，调用 model.select，不调用 view.select 抢焦点。set_item_enabled 委托原 model.set_enabled 并立即取消旧 pointer/Space，保持合法现有 focus；若 focused 失效则 blur，待新 focus 请求按原策略进入。

有限原验收：焦点可与 selected 不同；指针 down 只聚焦、有效 up 才选择；Space down 只 arm、up 只选择一次，repeat down 不造多个提交，cancel/disabled/window inactive/焦点转移都取消旧 arm；方向/Home/End 用原 key_event 的 consumed/changed，保原非 wrap 路径与 End 向后查可用项；已选目标也可收到焦点且不发选择 action。根 SegmentedGroup，原 ButtonLayout.visible 是 Segment part，实际 focused/selected 独立投影；AX Focus 只调 focus_with_visibility，AX Press 调原 view.select，一次且仅 changed 产 Activate。根不 Press，不以 Tabs/panel 引擎替代。

## 公共组合契约：最小新增，唯一 owner

三个私有 adapter 只保存原 model/View、固定 labels/presentation 与“最近这一次 delivery”数据；last delivery 不是状态 reducer，不参与下次选择。model/view 只读 getter 供真实 consumer 校验；不依赖 polling 触发 callback。外部设置 checked/selected/availability/error/compact 不发 Activate；状态改动与业务动作分离。

原 `ContentActionKind::Activate` 足够，不增加 SelectionChanged 通用 kind。为避免 drain 前外部 set 把动作值改写，建议三个有限 `ContentActionPayload` variants：`CheckboxState { checked: CheckState }`、`RadioSelection { index: usize }`、`SegmentSelection { index: usize }`，在实际 changed 的输入/AX delivery 时捕获。字段命名/CheckState 类型引用由 main 核 public API。固定 labels/slots + mount 已足够表达本范围身份，不添加 revision 引擎或借用 MenuSelection。不同 delivery 可以分别产生动作；同一次事件只把原成功一次映射为 activated 一次。paint、arrange、set_focus、程序受控更新、重绘、drain 第二次均零动作。移除/重挂后的未 drain 记录按原 actions.rs 丢弃，不保留死 owner。

一个 ContentId/原 mount 拥有整控件，radio/segment 的 part 不是新的 Content/owner。可见/可用 part 的 Focus、Press 必须回到该原 owner。mount 使用现有 ContentHost/bridge Target，不把业务索引当 mount。part 固定索引只在这个 mount 内有效；不支持同 mount 更换 labels/slots。卸载/替换/同 ID 重挂、模态遮挡、非活动 subtree/window 由原 Host gate 拒绝旧 capture/AX target；adapter 的 Unmounted/Cancel/set_focus(false) 仍必须实际调用原 button/view.cancel、blur/clear_layout，不能只依赖 host 清队列。重新挂载不恢复旧 pointer 或 Space。

window active 与 disabled 必须在 input 前同步到原 owner，非活动时无新 focus、无 pointer/key/AX activation。不要自己缓存一个“selected”或“focused”boolean 镜像。Radio 与 Segmented 的整体 disabled 可保存配置，但 View 的现有 disabled owner 通过 setter 同步，不能创建独立交互判断树。Checkbox 直接原 ButtonModel 的 disabled/loading。局部无效更新先完整验证再 mutation，失败不清原选择/焦点/动作队列；真正 availability 变化立即取消 gesture，避免 false→true→release 复活。

实际生命周期接缝为 `Component::window_active_changed(active)`；Host 既有递归分发，直接委托原 `set_window_active`，不要添加一个新 WindowFocus 事件。AX virtual part 的接受 Focus/Press 必须与 Host 的键盘 owner 一致：有效 Focus 响应 consumed；成功 Press 若需要 host 转焦点，复用 `accessibility_focus_requested()` 的本次 delivery receipt，不能只改 view.focused 留 Composer 作为 host.focused。Host 随后调用 adapter.set_focus(true,visible) 时优先保留原 view.focused（仍合法）而非重置成 selected，防止刚接受的 part 焦点被覆盖。每次 delivery 先清 receipt/last action；隐藏/禁用/非法 part 不触发 host focus transfer。这是既有 Host 委托契约，不是第二焦点状态机。

Component::measure 用原测量/字体服务；radio/segment 仅提取原 render 必要测量逻辑，不能复制 paint/hit 几何。Component contract 要求 host 在 geometry 改动后 render 再 input；paint 直接调原 render，并在返回 layout 上投影 pointer_regions/semantics。若 arrange 阶段必须建立 geometry，调用同一个原 View 的 render 到 scratch Scene（不调用 activated、不复制 View），由 main 按现有 adapter 风格裁定，不能新造静态 bounds 代替真实布局。TextAdapter 沿原 ButtonTextRenderer 路径。

Theme.palette 来自实际 ContentTheme；Checkbox render_on_surface(theme.surface())，Radio/Segment 调原 set_surface 后 render。Surface 已先验证；非法 Surface 不触碰 model/layout/capture。继承 actual ancestor clip，不新增控件外框裁剪来截断 focus ring；窄框/负数/非 finite/完全遮挡遵循原 invalid path。背景/标记/标签/focus ink 使用原 painter/theme tokens，不用默认 LIGHT/硬编码 canvas。compact 只转原尺寸参数；不新增 token 或 component registry 业务引擎。

## 必要角色闭包：建议先独立有限依赖票

现场 `ContentRole` 和 app-protocol `SemanticRole` 均无下列五个语义；macOS `AccessibilityRole` **已经有** Checkbox、RadioGroup、RadioButton，且 native value 对 Checkbox mixed 返回 NSNumber(-1)，其他 boolean selection 返回 0/1。Workbench 已把原 radio/segmented 映射为 RadioGroup/RadioButton。复用这些真实 native 路径，不凭角色名称声称完整能力。

| 拟新增 ContentRole / 同名 wire role | 本控件用法 | native role / 值 | 允许动作与文本能力 |
|---|---|---|---|
| Checkbox | Checkbox 根 | 已有 Checkbox → NSAccessibilityCheckBoxRole；selected=Checked，value=off/on/mixed，Mixed native -1 | Focus/Press；无 SetValue/SetSelection/ReplaceSelectedText、数值 range、text binding/layout/input |
| RadioGroup | Radio 根 | 已有 RadioGroup → NSAccessibilityRadioGroupRole；根无选择值 | 根不 Press；Host 将键盘焦点交原 roving owner；无文本/数值修改 |
| RadioButton | Radio 子 part | 已有 RadioButton → NSAccessibilityRadioButtonRole；selected bool → 0/1 | 可见可用 part Focus/Press；无编辑文本能力 |
| SegmentedGroup | Segmented 根 | 沿当前 Workbench 的 RadioGroup native 表达；根无选择值 | 原 roving owner；不宣称 TabGroup/panel 关系或文本编辑 |
| Segment | Segmented 子 part | 沿当前 Workbench 的 RadioButton native 表达；selected bool → 0/1 | 可见可用 part Focus/Press；focused 与 selected 分别读取 |

SegmentedGroup/Segment 的 Content/wire 身份保持明确，native 借用单选组表达是当前既有 bridge/Workbench 的具体映射，不是假 Group+selected；不需要为每个名字再造一个 native reducer。若 main/advisor 要求 SDK 特有 segmented subrole，先列最小真实 selector/SDK 依据再裁定，不在本只读设计中假称已核真实 NSSegmentedControl 或 VoiceOver。不增加第二个全库 native 前提。

实现必须核完整链：controls composition.rs role/interactivity + actions payload exports → module-runtime/content_accessibility.rs 全量 snapshot mapping → app-protocol/component/content/accessibility.rs role serde → module-runtime/container_semantics.rs validate/import → platform-macos/content_accessibility.rs 与 app-host/accessibility.rs **两条**投影 → 原 native accessibility.rs value/selectors/action routing。不得让现有 wildcard Group fallback 吞掉本组新 role；本组穷举后覆盖测试。不要顺便修复不相关 Tab 投影或重构所有 role。

最小形状验证建议：RadioButton part 必须归 RadioGroup、Segment part 必须归 SegmentedGroup（native 同映射不表示 wire 可混用），本 adapter 的 groups 无 text/range，Checkbox 禁不一致 mixed+selected；如需 remote 通用节点层级支持，用原节点/part parent 链核关系，不拿名字猜 parent。不要顺手限制合法旧 Group/Text snapshots 的可选属性。原 semantic part visible、disabled 与 input gate 同源。

### 与 #101 串行衔接、旧 peer 兼容

只读核 [#101](https://github.com/TekesApps/TekesUI/issues/101) 原四层 SearchInput/text 能力闭包，不复制或并行修改其候选。#101 负责 SearchInput、原 TextInput/Area 文本查询与编辑能力、新/旧 schema decoder、容器完整验证后的发布约定。#91 的角色依赖票在 main 接收 #101 后从**新实际源 SHA**开始，仅追加本组五个角色、非文本 action/value 正确性及自身回归；不重复实现 SearchInput 或通用协商框架。

沿 Tab/Tree 的 matched-schema：外层 VERSION 不改，所有旧 role 字符串/旧 Input/Reply 字段编码保持。新 role 仅匹配 sender/receiver schema 可用；旧 peer 对快照里任意一个本组未知 root/part role 必须拒绝**整份**快照，不能改成 Group/Switch、忽略 part 或部分成功。不以新版枚举拒绝 UnsupportedFutureRole 的测试冒充真实旧 decoder。

旧 decoder 验证：冻结追加本组角色**之前的实际 app-protocol 源**（最好是已接收 #101 schema），记录文件完整 SHA，并以那个实际旧 schema 的完整 Reply/semantic snapshot decoder 解析新生产 encoder 输出。分别把五种新角色放于合法 root/part，前置一个合法旧节点，尾部一个合法旧节点，断言整份失败；旧 roles 的完整有效快照仍成功。新 schema roundtrip 保全部值/parent/part/Target mount，无 loss。若旧 fixture 采用源码摘取，必须能证明就是该旧 revision 的完整相关 decode 类型/入口，不编一个极简旧 enum 替代。计划不在这里运行 decoder。

当前源的重要差异：`RemoteSemantics::update` 先 validate；validation error 调 `revoke()` 清 entries/snapshot，**不是**保留当前 snapshot 不变；成功时先在本地构造 entries，再发布。#101 原要求失败不改变当前语义快照，所以主协调需先明确接收后的原子边界（decode/validate/import/publish）与失效/断连的独立 revoke 策略。本组不能拿当前先 validate 当作 #101 已完成，也不在这里并行改事务。有限验收需真实 container 测试：已发布 A；B 带合法前缀、尾部 malformed geometry/parent/本组不一致 shape 或旧 decoder unknown role；无前缀 B 发布、失败无动作授权；按接收后 #101 契约断言 A 的身份/内容保持，独立 detach/revoke 仍失效旧 target。跨 process 旧 mount/part callbacks 也需真实路由测试。

## 有限实施拆分建议（main + advisor 裁定、复用/建票）

不在本会话建票或写 Issue。编号不是新 GitHub Issue，先以本组 P0–P4 指代；main 应全状态去重后复用/建票，均关联 #91/#56。

| 顺序 | 有限任务及接收出口 | 共享写入/依赖 |
|---|---|---|
| P0 | 本组五角色的完整非文本语义/wire/两条 Mac 投影、真实旧 schema 拒绝、container 原子失败/完整发布和本组能力测试；不是只加 enum | 先 main 接收 #101；main 独占 composition.rs、protocol、runtime、platform、AppHost，候选只交必要 hunks；#5 协调不等整票 |
| P1 | CheckboxContent 单原 owner + 独立 public consumer/原 Checkbox 对照 + 三态/几何/输入/lifecycle/actions | P0 API 已接收；独立 checkbox.rs、独立 tests；共享导出/payload 交 main 串行 |
| P2 | RadioGroupContent 单 model/View + 独立 consumer/原 View 对照 + roving/disabled/part/lifecycle/actions | P0；独立 radio.rs/tests；若原 selection.rs 必须最小方法补充，先交 hunk 及必要性，不并行 live |
| P3 | SegmentedControlContent 单 model/View + 独立 consumer/原 View 对照 + focus/selected/Space/focus paint | P0；独立 segmented.rs/tests；extended.rs/layout_view.rs 已有 WIP 仅规划 hunks，不覆盖 |
| P4 | main 串行整合并冻结实际源；三者公共 consumer、默认/外部 Theme、真实 GPU、原回归/strict、适用 Composer/reading；库存/Workbench/正式设计/父票回填 | P0–P3 实际接收；公共 consumer lib 导出、registry/库存/Workbench/docs 均按 hunk，由 main 串行 |

P1–P3 是否能独立排期由 main 决定；此表不授权并行 shared writer，不要求另开长生命周期协调框架。无新 target/prepare/cache reset/clean/install、Git 发布、pin 或清理。

## 验证计划：每层证据独立，不将计划当通过

**本设计实际执行：只读源码/Issue 核查及 QA 文档/哈希记录。下列测试、构建、GPU、native 全部未运行。**

| 层 | 必须实际验证的有限矩阵 | 留证/通过含义 |
|---|---|---|
| 三独立 API/consumer | 各自 constructor/invalid update/空列表/全部禁用、原 getter、真实 ContentHost、动作 source 与 delivery payload；render 前后不重复动作；drain 后再 drain 空 | 独立 checkbox_content/radio_content/segmented_content 源与 tests；不能用同一个 SwitchContent 断言代替 |
| 原 View 对照 | 相同文本、frame/clip/palette/Surface、同输入序列，逐步比较原模型值、实际 focus、layout.visible、响应 consumed/changed、gesture cancellation | Checkbox 原对象、Radio model+View、Segment model+View 三份参照；不复制 painter 或 reducer 作“oracle” |
| 原输入/lifecycle | 每种在 pointer down/Space arm 后 cancel、disable→enable、window loss→restore、hide/reveal、remove/replace/同 ID remount、旧 native target/part/capture、错误 Surface；radio disabled 迟到 focus；segment focus!=selected/End/首中项焦点环 | 保原 source/source-at-event 语义；Focus/repaint/外部状态更新无选择动作；原支持重复导航不被强行去重 |
| 角色/协议/runtime | 五 role 新完整 snapshot roundtrip；旧真实 decoder 整份拒绝；旧 Switch/TextInput/Tab/Tree 编码行为无变；parent/part/capability/value/混态；container A→bad B→good C 完整发布与旧 target 拒绝 | module snapshot、wire、container、platform 与 AppHost 两链；不能用新 enum 随机字符串拒绝替代 |
| native 有限程序 | 实际 bridge 的 CheckBox mixed=-1/off=0/on=1、radio/segment 0/1、准确 native role；可见可用 Focus/Press 一次、禁用/失活拒绝、文本编辑 selectors 无授权 | 在实际 macOS 有限程序观测 selector/value/action，不只 Rust enum 名称；若缺窗口/VO/物理输入则明确未验，#5 整体无关闭前提 |
| Theme/Surface/clip | Theme::ALL，默认与真实外部 compile-consumer-brand/theme definition；canvas/sidebar/field、窄/宽、长中英换行；scale 1/1.5/2、部分/全遮挡；禁用/error/selected/focus（error 仅原支持控件） | 外部 Theme 需真实生成输入并实际 consumer build，不能 Palette brand loop 冒充；记录输入哈希与 token 观测范围 |
| 必要真实 GPU | CoreText 实际 glyph atlas + Metal renderer；逐控件与原 View 相同状态参照比较，Checkbox 三态与 label first-line，Radio row clipping/focus，Segment 首/中/末 selected 与不同 focus、邻项覆盖/原祖先 clip | 输出 RGBA/PNG、场景输入、比较阈值/实际差异、renderer/scale、程序 SHA/depinfo；Scene 数量、非背景像素检查不足以证明上述几何/颜色 |
| Composer/reading 适用保护 | 将三个真实 adapter 放入既有 mixed reader + 原 TextFieldContent Composer，设有 selection/marked/draft；普通选择控件点击可正常转移焦点，IME 期间沿原 barrier 拒绝；显式 preserving_focus 辅助配置才保编辑焦点 | 不宣称所有选择都“保 Composer 焦点”；比较 text/selection/marked/undo/binding，无替换 editor；reading prepend/stream/anchor/reveal 及手动历史行为不被选择动作改写，无自动 scroll/latest |
| strict / 原三回归 | 受影响 controls、protocol、module-runtime、platform-macos、app-host 和独立 consumer tests + Clippy all-targets -D warnings；适用格式/组合/设计门禁 | 三原回归均需真实运行；不沿用旧 ledger 的 passed；不把静态检查当 GUI/GPU/VO/性能 |

已有真实入口参考（必须按最终源/功能选择执行，不声称现在全通过）：

- controls `widgets.rs` 内 Checkbox 单测、`selection.rs` 的 Radio 单测、`layout_view.rs`/`extended.rs` 的 Segmented 单测；`tests/selection_contract.rs`、`selection_availability.rs`、`action_contract.rs`、`capture_origin.rs`、`composition_adapters.rs`、`composition_focus_restore.rs`、`external_preflight.rs`。`choice_selection_states.rs` 主要是 Select/Combo，不冒充这三者完整原回归。
- consumer `selection_surface.rs`（原 Checkbox/Radio/Switch Surface）、`unified_button_gpu.rs`（真实 GPU 写法参照）、`content_actions.rs`、`pointer_metadata.rs`、`collection_mount.rs`、`focus_boundary.rs`、`surface_context.rs`、`theme_definition_content.rs`；三者新增独立 tests，不能只跑旧 selection_surface。
- mixed fixture 使用 `src/reading_stream.rs`、真实 `TextFieldContent`，现有 `tests/scenarios.rs` 的 structured prepend/composer、mixed A-B-A；`controlled_reading.rs`、`controlled_text.rs`、`reading_stream.rs`、`reading_prepend.rs`、`text_mount.rs` 按适用变更回归，不造模拟 Composer 字符串。
- runtime `content_accessibility.rs`/`container_semantics.rs` 与 native routing；platform `content_accessibility.rs`/`accessibility.rs`；AppHost `accessibility.rs`；新增本组 role/value/action/文本 capability 测试。

实施获指令后才使用**当时最新** `scripts/tekesui_build.py` 的 `build_lock()` / `environment()`、Rust `+1.95.0 --locked --offline`、原 `target/review-cache`。此设计不调用 helper、target 程序或 prepare。锁被占即等可用时段/重试，不打断持锁程序，不绕开锁。命令计划是同一原 helper 环境下的 `cargo +1.95.0 test --locked --offline -p <受影响包>` 以及 consumer `--manifest-path examples/composition-consumer/Cargo.toml` 对应独立 tests，Clippy 同参数加 `--all-targets -- -D warnings`；最终真实完整 argv/env/log 由执行轮次记录，不能把这里的占位符当已执行命令。

冻结证据时记录完整 HEAD、实际 before/after source SHA256、Cargo.lock、外部 Theme 输入、完整命令/退出码、程序 SHA256、depinfo 实际输入、失败重试、native/GPU 的实际 renderer 和输入；变化后新轮次，不能刷新旧 receipt。不得把本目录的“所读文件窗口”说成可构建全树闭包。若源窗口发生变化，main 先重核 hunks 和依赖再接入。

## 仅规划的共享 hunk 与同步

- `composition.rs`：三个模块声明/公共 re-export + 五 role + 最小 interactive 判断；`composition/actions.rs`：三个有限 immutable payload；每一 hunk 同 #101 等已接收改动合并，不整文件覆盖。
- `widgets.rs` / `selection.rs` / `extended.rs` / `layout_view.rs`：默认不改原算法；如果只读测量/同步接口确有缺口，独立列原 owner 方法的小 hunk，说明 public 兼容与原回归。保留所有已有 WIP。特别不复制 Segment buttons、Radio navigation 或 Checkbox toggle。
- protocol/runtime/platform/AppHost：按 P0 全链 hunk，在 #101 接收后的实际 SHA 上串行接入；本设计没有并行生产候选。
- `examples/composition-consumer/src/lib.rs` 导出、三个独立场景与 tests；真实外部 Theme/GPU 入口由 main 整合。
- `design/gallery/public-api-surface.json`、`design/gallery/workbench.json`、实际 registry API 入口、组合库存/审计矩阵、正式 `docs/design/` 与 #91/#56/#55/#65 适用依赖及实施子票：由 main 根据最终 API 同步。Workbench 保留原 View 参照并接真实 Content，不能改登记就声称实现。当前不写上述文件或 Issue。

## 交接结论与待裁定

建议 main + advisor 接收三个有限 API/原验收，裁定五 role 命名及 P0 排期；先接收 #101 的共享闭包/原子发布结果，再建/复用本组角色依赖与三个 adapter 子任务。没有 #101 的生产 diff 可由本会话评价或合并。默认不增加可序列化选择命令、动态 items/revision 引擎、全库语义重构或整 VoiceOver/性能关闭前提。

未验：所有编译/单测/strict、旧 decoder、container 实际原子发布、两条 Mac 实际投影/selector/value、GPU/GUI/native window/物理键盘/IME/VoiceOver、外部 Theme 生成消费和性能。读源确认不等于这些层通过。本会话完成设计交接后等待 main 的实施指令。


# #91 设计补充：原 remote 失败授权规则

2026-10-24（Asia/Shanghai）。本补充依据 main + advisor 已裁定的修正，取代历史 `../design.md:132` 中“失败后 A 身份/内容保持”的验收要求，以及任何据此理解为“保留可授权 A”的文字。原设计、源窗口与 `handoff-check.json` seal 全部保留，不回写历史。三 adapter API、五角色命名与其余拆票建议仍待 main + advisor 审，不启动实施。

唯一代码源仍为 R=`/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`。本次只读核对正式 `docs/design/searchfield-content-20261024.md` 最后的 **Main correction: preserve original remote failure authorization**，及该 R 的真实 container/RemoteSemantics owner；只在本 amendment 子目录写入。未构建、运行程序、GPU/GUI、委派、联系其他会话、改 live/正式 docs/Issue 或改旧 seal。

## 修正理由与真实接缝

`container.rs:720–725` 中，`semantic_projection.update(next.clone())` 失败仅记录 error，之后仍设置 `content_snapshot = Some(next)`。语义投影 A 和当前 content B 因而可能不同步。“失败保持 A 且继续授权 A”会让旧语义回调对着新 content 生效，是错误的新增假设。**原子性约束是失败候选不能部分发布，且旧权限沿原 owner 的 dirty/revoke 策略失效；不是把可用 A 永远保留。**

`container_semantics.rs` 的当前路径：update 入口置 dirty；decoded validation Err 调 revoke 清 entries/snapshot/attachment；本地构造 entries 后才整体赋值；`parts`、`text_layout`、`callback` 等受 needs_refresh/dirty 屏障保护。当前 allocator `self.next.checked_add` 的失败与 staging 接收方案，由 #101 owner 负责有限实现与实测；本设计不改它或复制第二个错误处理器。

`container.rs:1684–1692` 的原 AX request/refresh 错误路径置 semantic_projection.dirty，拒绝后续旧 action，不取消较新的 editor/gesture。decode/unknown-role 发生在 Request/Reply owner 层，不能虚称已经进入 RemoteSemantics::validate，亦不能只给 RemoteSemantics 喂一个人为 malformed enum 便称完成真实 decoder/owner 回归。

## 替代 §132 的明确验收

| 失败/恢复层 | 设计要求 | 禁止误判 |
|---|---|---|
| 整份 decode / unknown role | 沿真实 request owner 的 failure/dirty/revoke 路径；新候选没有任何合法前缀被发布；旧投影不能继续提供 action/text-layout 授权 | decoder 返回 Err 但 owner 仍可用 A，不能算过；不凭新 enum 拒绝随机字符串代替旧 decoder |
| decoded validation Err | 清 entries、snapshot、attachment，保持 dirty；旧 callback/text geometry 无法授权 | 不断言 invalid B 后 A 原样且仍可用；不能只保 A 的 value 而忽略 callback gate |
| import ID 容量失败 | 至少保 dirty 屏障；旧 A 不授权，无半份 entries/snapshot 发布，无 ID 回绕；局部 staged next 可不推进 | dirty 内部可能仍保历史存储，但这不等于 A 可公开使用；不要求为 rollback 恢复旧权限 |
| disconnect / hidden / inactive / cancel / retirement | 保原各生命周期 owner 的 revoke 规则，旧 target/mount/part 失效 | 不绕过原撤销去满足“快照不变”断言；不全 Host.cancel 误取消邻居 Composer |
| 完整合法 C 恢复 | 完整 decode/validate/import 后才整体发布当前有效语义；已撤销的旧 callbacks/text binding/part 身份不复活，ID 单调且不回绕 | 不能用 C 到来掩盖 A 在失败窗口仍有权限；不能仅看最终 selected/value |

本补充的“整体发布”专指语义候选完整发布及权限边界；不扩大为必须原子回滚 `content_snapshot`、整个 Host、邻居 editor 或跨库所有状态。保持 #101 原 SearchInput 文本能力、single owner、旧 TextInput 编码/能力和本组非文本能力要求。

## 旧 matched-schema 策略的补充

历史 §128–130 的外层 VERSION 不变、旧编码不变、五角色完整快照由真实旧 schema decoder 整份拒绝要求保持。额外必须连真实 request owner 验证失败后的授权结果，不只停留在 serde Err。

1. 冻结 main 已接收的 #101 schema 后、本组五角色追加前的真实完整 decode 输入/源码 SHA。新 encoder 的完整 Reply/semantic snapshot 含合法旧前缀/新 root 或 part role/合法旧后缀；分别覆盖 Checkbox、RadioGroup、RadioButton、SegmentedGroup、Segment。旧 decoder 整份失败，不降为 Group/Switch，不跳过未知 part。
2. 将真实 decode failure 沿原 request owner 路径送达 container，断言没有候选前缀 publication；旧 action target、text-layout query 和 callback 权限被原 dirty/revoke 屏障拒绝。旧 schema 自身不能构造新 role 的 Rust enum，因此不把新的 enum 值直接喂给旧 validator。
3. 相同旧 schema 对真实旧 roles 完整快照继续成功；新 matched schema 对本组合法完整数据 roundtrip；不新增通用协商机制或更改旧角色授权范围。
4. 若错误发生在 refresh / AX request 等不同原入口，按其各自原 owner 路径验证；不造一个统一 fake failure callback 绕过 request owner，也不把所有失败强行转为全 Host.cancel。

## 有限实测计划（本次全部未运行）

- **A → decoded malformed B**：先发布可用 A 并捕获旧 callback/part/text-layout 引用；B 保合法前缀、尾部错误 geometry/parent/本组形状。实际 container 接收 B 内容的路径中，验证失败清 entries/snapshot/attachment，原 A 权限不可用、B 前缀未发布。不能用独立 validator 单测代替 container 接收链。
- **A → import 容量失败 B**：在真实 import 边界注入有限 allocator 容量失败，覆盖已有旧 entry 与新 entry 混合候选。dirty 保持；没有半份候选、无回绕；实际旧 action/text-layout 路径拒绝。staged next 是否未推进按 #101 接收实现记录，不能把它等同于保可用 A。
- **A → 真实旧 decoder unknown role / request error**：完整 Reply decode 失败接入实际 request owner；核 dirty/revoke 和无旧权限，不以 decoded malformed B 测试替代。
- **失败后合法 C / 生命周期撤销恢复**：完整 C 可发布，A 的已撤销 callback/旧 mount/part/text binding 仍拒绝；分别覆盖断连、hidden、inactive、cancel、retirement 原规则及合法恢复。记录 ID 单调性、实际授权前后状态，不用最终页面值代替。
- **邻居真实 Composer**：同原 Host 内保留独立 TextFieldContent/TextSurface，含 draft、selection、marked/native binding；失败 remote owner 的撤销不重挂、不取消或改写邻居 editor，不在这里引入业务发送/清空/IME 状态机。原窗口失活等全局生命周期行为按原规则，不能与 remote 局部失败混为同一断言。

上面是原 #91 P0/#101 公共接缝的有限失败回归，后续以 main 接收的实际源窗口为准。没有新增全库 native/VoiceOver/性能关闭前提，没有重新排期或建票。#101 正在恢复原规则，本会话不检查/改写其独立候选，不复制实施。

## 证据与交接

`amendment-manifest.json` 记录本次 main correction 文档和 canonical owner 的完整 SHA、源码片段范围、当前 HEAD、旧 seal 文件的核对结果，以及本补充的 SHA。本次没有程序/depinfo/测试/GPU/native evidence，不能提升原五项状态。旧 `handoff-check.json` 的源窗口结论只属于历史设计时刻；本补充不刷新旧身份。

交接后继续等待 main 实施指令；其余三 adapter / 五角色方案维持待审。


# #91 amendment2：曝光授权、共享几何与受控更新

2026-10-24（Asia/Shanghai）。main 转交 advisor 审查意见后补充的有限设计；三 adapter + 一语义桥/五 roles 方向可供 main 接收，尚未实施。原 `design.md`、amendment1、两者 manifest/seal 全部保留；本文件覆盖原稿中的固定 index 授权假设、默认 scratch Scene arrange 方案及未规定的受控更新 gesture 政策。amendment1 的 remote failure/dirty/revoke 修正持续有效。

只读 R=`/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI` 的 canonical source 和 [#74](https://github.com/TekesApps/TekesUI/issues/74)；只写本新 QA 子目录。未改 live、正式 docs、Issue，未构建/执行/GPU/GUI/委派或联系其他会话。新源 SHA 见本目录 manifest，不以历史源窗口替代现状。

## 1. 固定 slot 与曝光授权分别约束

labels、顺序、数量在同一 adapter 实例内固定；空 label、重复 label、空组、全 disabled 均合法。label 不是 key，不加非空/去重约束。整控件内容替换使用新 Content mount；不加 labels revision 引擎、动态 items/store 或 ChoiceEngine。**固定 index 只是逻辑槽，不能表示一次可交互曝光。**

现有真实可复用接缝：

- #74 原 TreeContent 用 monotonic exposure token 作为 `SemanticPart.index`，回调先 token→原逻辑 key/index，hide/disable/remove/ABA 后不能只靠逻辑 key 复活；`activity_epoch` 和 `intent_is_current` 过滤旧动作；`action_payload_current` 是原队列的纯授权过滤入口。复用这一有限身份模式，不复制 Tree model/dataset/navigation 或新建通用选择引擎。
- 现有 `Component::presentation_runnable_changed(bool)` 传入 effective active ancestry 与非空 arranged visible clip；`window_active_changed(bool)` 传入窗口活动，适合在原 owner 撤销曝光与 cancel。per-item 可见性仍取原 View 的实际 layout/clip，不能用根 runnable 推断每个 part 可见。
- `composition/action_proxy.rs::Exposure` 已有 revoke/pending/reserve/published；`empty_state.rs` 与 `notice.rs` 的 Host publisher 在原布局后先 reserve 全部所需 mount，再 commit，恢复曝光不由任意通知当场分配。这是**根身份更新的可复用机制**；它目前不是任何新 adapter 自动享有的通用能力。
- `actions.rs::drain_action_records` 查相同 Content mount，再做 EmptyState/Notice 专用 guard 及 component.action_payload_current；默认 hook 为 true。它不会自动证明本组三者 hidden/disabled/scope gate 正确，尤其普通 `find` 并非 active subtree 查询。

### 有限曝光设计

1. **持续可用**：mount/part token 保持；布局移动、Theme/Surface 变化但仍连续可见可用、合法 checked/selected 更新不更新曝光身份。原布局几何仍决定命中。不是每帧分新 token。
2. **Radio/Segment part**：用 #74 相同的 checked monotonic token→固定逻辑 index 映射。对外 `SemanticPart.index` 发布 token，不能发布逻辑 index；consumer 动作 payload 仍读逻辑 index。part 隐藏/失活/禁用后撤销 token；恢复成功布局并重新曝光时取得新 token，旧 AX token 即使 enabled/index 又相同仍拒绝。组整体 hidden/inactive/disabled 时撤销全部 parts。token 不复用、不回绕；容量不足不得部分发布或保旧权限。
3. **Checkbox 根**：原 AX key 为 mount+None，无 part token。仅内部 boolean/pending 不足以拒绝恢复后旧 native key；需 main 接入最小 Host hunk，沿现有 Exposure 的 reserve/commit/published 机制在重新曝光时更新**这个原 Content 根的 semantic mount**，同时保原 Checkbox model/ButtonModel，不制造 proxy Checkbox 或第二 input owner。失活期间旧 mount 无授权，恢复只认新曝光 key。完整控件重挂/替换仍按原 lifecycle 换 mount。
4. **Host 最小接缝明确列依赖**：复用 `presentation_runnable_changed`/`window_active_changed` 通知，沿原 layout 后 publication 阶段做本组必要 reserve→commit；如果根 mount renewal 的既有 publisher 当前仅识别 EmptyState/Notice，则只加本组有限 branch/hook，由 main 串行审 hunk。未接入并实测前不能承诺 Checkbox hidden→restore 旧 AX 拒绝。不要直接在 component.paint 偷改 Host mount，也不借 mount renewal 解决每次 setter。
5. **两层顺序**：AX 先过原 ContentHost 活动窗口、scope、disabled 和 IME transfer barrier；再解析当前曝光 token/根 key、可见性和原 View 可用性；最后才调用原 View.focus/select 或 Checkbox.toggle。拒绝不能先挪原 View 焦点再补 gate。root Focus 由原 Host 路径；组 root 不 Press。

曝光 token/活动 stamp 只用于“这次 exposure 还能否授权”，不参与 checked/selected/navigation 计算。实现若发现精确 Host seam 与 proposal 不兼容，先把有限缺口交 main 裁定，不退回 enabled+index 或新造命令 framework。

### 原 queued action 的精确 current 政策

原稿三个 payload 的公开值仍是 delivery 时的 checked/index；补充为**有限 opaque intent payload**，内部附当前原 owner 的曝光凭据/活动 stamp，公开只读 checked()/index()。可继续用三个 ContentActionPayload variants；不是通用 revision 引擎或可执行命令。原队列已有 mount，adapter 提供 `action_payload_current` 纯过滤并核当前曝光；stamp 不与当前 checked/selected 比较。

| 之后发生的变化 | 已成立但未 drain 的记录 | 原 gesture / AX |
|---|---|---|
| 合法 checked/selected setter，曝光持续 | 保原 delivery 的 checked/index，不改写、不因当前值不同抹去 | setter cancel 原未完成 pointer/Space，见第 3 节；AX 同持续曝光 key 合法 |
| 无变化合法 setter | 历史记录保持 | 完全 no-op；不 cancel、不造 action |
| 单纯 Cancel（当前仍可用） | 保已经成立的意图，按 #74；不能因取消未完成 gesture 抹去已成立动作 | 原 owner cancel 后迟到 up 无新动作 |
| item disabled/hidden 或组 disabled/hidden/inactive，之后又恢复 | 该旧曝光的记录拒绝且不复活；组整体撤销影响全部本组记录 | 旧 part/root key 拒绝；恢复是新曝光 |
| item 不相关 sibling 状态变化且目标曝光连续可用 | 保目标记录；不得无理由抹去所有选项历史 | 仅撤销实际受影响授权，不取消邻居 Composer |
| unmount/replace/同 ID remount | 原队列 mount gate 拒绝旧记录；不能复活旧 token | lifecycle cancel；旧 target/mount/part 拒绝 |
| 非法更新 | 原状态、gesture、曝光、队列全不变 | 验证前不 cancel/分配/退休 |

“hidden”包括原 runnable/祖先 active scope 失效及该 part 实际 clip 消失。需要在本组曝光 owner 接收到原通知时记录撤销，不能只在 drain 时看目前是否可见：否则 hidden→restore 后会误接纳旧记录。对于原 Host scope 的变化，实施要核该 hook 在原 reconcile/layout 的真实调用时序；如无通知须明确最小 hunk，不声称现有 hook 已覆盖所有 scope。旧记录不能只因一个空标签/重复标签被误判为另一项。

## 2. 默认取消 scratch Scene arrange：原几何只保一份

原稿“arrange 可调 render 到 scratch Scene”的默认选项撤回。原 render 确实会同步 placement、availability/focus/gesture/presentation：Checkbox 调 synchronize_activation/refresh_pointer_state，Radio render 会修 focused/cancel/blur，Segment render 会同步 buttons/space_armed/focused。重复 render 不能未经审查当纯 arrange。

优先从原 View 提取**最小同源 measure/arrange** 方法（草图，不是已有 API）：

```rust
// Checkbox：沿原 measure，原 render 的 placement/label/first-line 算法抽成共享 arrange。
// RadioGroupView：同一个 labels metrics/row/indicator/label-slot 计算与原状态核对。
// SegmentedControlView：同一个原 Stack/Button placement 算法与原 focus/availability 核对。
// 原 render 调此 arrange + 原 painter；adapter Component::arrange 也调同一入口。
// paint 不再第二次推进状态；若原 render 仍调用 arrange，其相同输入必须幂等且不二次通知。
```

只提取必要几何与原 owner reconciliation，不复制 adapter 版本测量、不造第二布局 cache/View/renderer，不全库重构。保公开原 render 签名/行为兼容；原 view 和 Content 皆由相同函数得到布局。传原 ButtonTextRenderer、有限 frame/ancestor clip；原指标、ButtonModel geometry、Radio first-line、Segment focused-last 绘制次序仍由原实现拥有。新 API 是否仅 crate-visible 由 main 按 consumer 需求决定，不为测试扩大 API。

scratch Scene 仅保留为**未接收备选**：必须先逐语句列副作用并证明同帧不重复推进动效、修改 focus/availability、发送通知/帧请求或重复同步 gesture；独立原 View/adapter 比较实际调用计数及状态。此证明前不得以 scratch 为可交付方案，也不能拿视觉像素相同当状态副作用证明。

## 3. 外部受控更新：验证 → 原 cancel → 发布

采用 main 建议：**合法且实际改变受控值的更新，先完整 preflight，再沿原 owner cancel 退休旧 gesture，然后发布值**。取消不 blur 合法焦点，不伪造一次选择动作。无变化合法 setter 保 no-op；非法更新在所有副作用之前拒绝。固定 labels 不更新，暂无第二 model/binding/revision。

| 入口/API 草图 | 原 seam 与精确政策 |
|---|---|
| CheckboxContent::set_checked(state) | CheckState 全三态合法；相同不变。变化时先 `checkbox.button.cancel()`（真实存在，清 pointer/Space arm），再原 checked 赋值；保 focus/placement，零 Activate。旧 release/重复 Space up 不 toggle 新值 |
| RadioGroupContent::set_selected(Option<usize>) | Some 必须范围内且 enabled；None 合法。原 `RadioGroup::select(usize)` 不能表达 None，建议补一个有限 model owner 方法 `set_selected(Option<usize>) -> Result<bool, …>`，内部验证后用原 select(Some) / 原 selected=None，返回实际变化。不可重建 model、重置 enabled 或复制 key_from 导航。adapter 验证后 `view.cancel()` 再调该原方法，保合法 focused；Space 本来 key-down 选择，无 arm，但旧 pointer up 仍必须失效 |
| SegmentedControlContent::set_selected(index) | 完整范围/enabled preflight；变化前 `view.cancel()` 清 pressed/space_armed 和原 buttons arms，再 `model.select(index)`。不用 view.select 自动移动焦点，保焦点与 selected 的区别 |
| item availability / 整体 disabled / loading | 先验证 index/状态合法性；真正变化按原 model setter 与原 view/button cancel 同步；依第 1 节撤销受影响曝光及 current gate。不可 false→true 后旧 release 复活 |

Radio 的新增 model 方法是 shared `extended.rs` 最小 owner hunk，交 main 串行；不由 adapter 直接写 selected 字段绕过设计方法（即使原字段目前 public）。原旧 API 兼容保留。此新方法只管受控发布校验，不发业务动作，不管理焦点/gesture，后两者仍是原 View 所有。方法不能在返回 Err 前动 selected/enabled。可用性修改导致原模型 fallback 的行为保持，不复制另一个 fallback 算法。

原模型无 readonly，本组不新增 readonly，不用 disabled 模拟 readonly。AX 文本修改动作拒绝源于无文本能力，不是创建一个只读 editor。空组/全 disabled、空/重复 labels 按原有限设计合法；setter 的 Some/index 对不可用项仍明确 Err，不把合法空构造等同于能选越界项。

## 4. 五角色桥：接收边界保持

Checkbox/RadioGroup/RadioButton 映射已有 native 角色；SegmentedGroup/Segment 保独立 Content/wire 身份，native 采用现有单选组/单选项的有限表达。不是 Tabs/panel 关系，不能据此宣称 VoiceOver 实听或真实 NSSegmentedControl 行为已验。

桥的有限验收仍包含：Checkbox mixed 与 selected 一致（Mixed 不等同 selected=true）；RadioButton/Segment 的 part parent role 匹配，不允许交叉父组；组根无 Press/伪 selection；本组无文本 input/binding/layout/range，SetValue/SetSelection/ReplaceSelectedText 与文字 selector 不获授权；platform Content 投影和 AppHost 投影同时闭包；真实旧 matched schema 整份拒绝及 amendment1 原 owner dirty/revoke。AX part 必须先原 Host IME/scope gate 后变 View，不能只测直调 adapter。

## 5. 有限验证补充（全部未执行）

1. 每 adapter 原 View/模型与独立 public consumer 比较：down/Space arm → 合法异值 setter → late up；新受控值保持且零额外 Activate。Radio 包含 pointer arm→None 与 Some，Segment 包含 focused!=selected；Checkbox 包含 Mixed。相同 setter 不取消；非法范围/disabled目标 setter 的前后完整 gesture/queue/focus/exposure 不变，原 up 仍按原语义完成。每项有独立断言，不共用 Switch 模拟。
2. 同 mount/固定 slots 捕获真实 native/wire AX key/token → hidden/inactive/disabled → 恢复 → 重放旧 key：零 action/零 focus 转移；新曝光 key 有效。重复 label/空 label 不混淆。单个 part 部分 clip 隐藏与整组祖先 inactive 分别测；失活 callback 不能只靠 enabled+index 通过。
3. 交付 action 后不 drain → 合法异值 setter → drain：payload 仍是旧 delivery 的 checked/index，一次；再 drain 空。交付后 Cancel 保成立意图；交付后 exposure revoke→restore 的旧记录拒绝；unmount/remount 拒绝。核原 `action_payload_current` 纯过滤不分配/修改值、不做第二 reducer。
4. exposure 容量失败先拒绝完整发布，不部分更新 token/mount/queue，不回绕。撤销期间旧授权仍无效；#74 token 的模式与固定根 Exposure 两阶段 publisher 分别实测，不凭“有 guard”推断通过。
5. 原几何提取：原 View API 回归与三 adapter 同输入布局/paint 对照；arrange/paint 次数、focus/gesture/presentation/请求帧计数核对。默认/真实外部 Theme、Surface、clip、CoreText/必要实际 GPU 计划延续原稿；未证明 scratch 副作用就不启用该备选。
6. 同 Host 真实 Composer marked/selection/binding：AX Focus/Press 在原 gate 被拒时，原 View 也完全不变；本组受控 setter/Theme/曝光更新不取消或重挂邻居 editor。保持 reading/anchor/stream 适用保护，不新增全库 native/VO/性能关闭前提。

## 6. 拆票草案与串行 hunk

main 未来全状态去重后复用/建票，本会话不建票：

1. **#101 实际接收**：共享语义接缝和 amendment1 原失败授权实测，取新源 SHA；本组不并行复制 SearchInput 实现。
2. **一张本组五角色语义桥票**：五 roles、值/父role/无文本能力、两投影、旧完整 decoder/原 owner 失败、匹配 schema。同步准备 exposure 最小 Host 接缝的逐 hunk 审查；它不变成新通用选择 framework。
3. **三张独立 adapter 票**：各自原 owner/API/原几何提取、原 cancel 受控更新、曝光/current gate/独立 consumer/原回归。Checkbox 根曝光 publisher、Radio controlled None 方法、两组 part token 映射按最小所需 hunk 列入对应票；共享 composition.rs/actions.rs/extended.rs/selection.rs/layout_view.rs、Host publication 仅 main 串行接入。若桥无需依赖这些 adapter hunk 不提前扩 scope。
4. **main 组合收口**：真实默认/外部 Theme、原 View/必要 GPU、Composer/reading、strict；公共库存/Workbench/正式设计/父票按原五项同步，保未验。无 ChoicesEngine、额外命令层、readonly 模式或 labels revision。

新公共 intent/方法签名、具体 Host hunk 和曝光容量 error 类型仍由 main 在实施票固定；本设计不宣称这些 API 已存在或已接收。旧源码、design seal 和 amendment1 不改，交 main 接收后继续等待实施指令。


## Main acceptance corrections — actual root identity and capacity, 2026-10-24

Main/advisor accept the three separate adapters + one five-role bridge finite split, with this correction taking precedence over amendment2's semantic-mount wording. Existing Exposure::reserve calls original try_next_mount; publishing Content.mount changes the actual Content mount, not an independent semantic-only field. Checkbox root re-exposure may use a controlled renewal of its original Content identity, retaining the same Checkbox/ButtonModel and ComponentId without synthetic Mounted/Unmounted, proxy or neighbor remount. Stable mount holds for continuous active exposure and value/Theme/layout updates; do not promise stable root mount across deliberate authorization renewal.

Checkbox implementation must explicitly handle this root's capture, pending action records, focus restore/reveal and original reading item_mounts. If registered as a reading item, prove the existing reading identity contract; do not silently rewrite caches as if identity never changed. Inadequate owner seams return a concrete blocker to main. No whole-Host.cancel or neighbor Composer cancellation. Root publisher belongs Checkbox ticket, not five-role bridge.

Original Exposure multi-reserve only guarantees no partially published mounts, not unchanged global allocator after failed reservation. If counter-unchanged is promised, reuse accepted original batch checked reservation; otherwise document consumed monotonic reservations, never rollback/reuse/wrap. Part token plan fully prepared before publication; exhaustion leaves pending and old authorization revoked. Preserve distinction between remote dirty/revoke failure and local controlled setter's unchanged-state-on-Err.

Plan: #101 actual acceptance -> one five-role semantic closure -> three independent adapter tasks. Radio None setter belongs Radio ticket; each group owns its part mapping; original View geometry extraction belongs corresponding adapter. Main serially merges shared files and accepts required source/public consumer/GPU/strict/docs. Formal design acceptance is not implementation acceptance. Parent #91 remains OPEN, original five criteria unchanged.


## Main #107 finite shared-role acceptance — 2026-10-24

The five-role shared semantics bridge is accepted in current HEAD+preserved WIP. Main merged 14 necessary paths/21 clean hunks plus seven-file reviewed union, preserving accepted #95/#101/#106 and integrated Select #105. Original five #107 criteria are met; #108 Checkbox, #109 Radio and #110 Segmented adapters remain separate.

Main ran eight combined root lanes (556 test invocations) and two meaningful original-CoreText capability tests plus follow-up strict. Final production role/validation/projection bytes are unchanged by follow-up test-only fixes. Both public projections withhold a real original TextSurface for all five selection roles; complete sender validation refuses actual text metadata under a valid full parent hierarchy. Actual native Selection391 checks, Select289/Search337/Link6 regressions use original objects, no windows. Main source/program/depinfo and exact failures are recorded at design/reviews/selection107-main-intake-20261024/root-acceptance.json.

The added test fixtures required full parent hierarchy, proper semantic mount key, original child arrangement and a nonempty-range assertion lint fix; failed rounds are preserved, not production regressions or accepted tests. Source/runtime/strict acceptance is finite. No NSWindow/physical IME/VoiceOver/performance/GPU/publication/PR claim. #98 strict focus action regression and #111 remote geometry remain separate and unresolved; their presence does not add whole-ticket prerequisites to this finite bridge. Normal #8 metadata refresh/canonical bindings remain stale after newer source; existing old observations are not re-signed.


## Main #108 reading identity decision (2026-10-24)

原 Exposure publisher 的实际 mount 换代保留；Checkbox 原 model/ComponentId 不变，不伪造 lifecycle，也不另设 semantic mount。连续曝光中的值、Theme、位置变化保持原身份。已登记 reading item 的 mount 换代后，旧登记按原合同拒绝 TargetReplaced，旧 AX/focus-return/prepared reveal 不自动继承新身份。合法业务 caller 若决定继续该文档 item，使用原 expected-version reading 更新，推进自身 revision 并提交原 item 列表；新布局后登记实际新 mount。adapter 不自行增加业务 reading revision，不静默改签 item_mounts。新登记后旧 expected version 仍拒绝，删除/替换/同 ID ABA 仍保持原 stale 保护。

这是 #108 原第二项“按原合同处理”的具体执行方案，不新增关闭前提；自动无 revision 恢复是未实现的独立功能，不列为本票能力。现 EmptyState probe 只证明边界；必须以实际 Checkbox public consumer 验证连续曝光保持、换代旧登记拒绝、caller 原更新+新布局后重新可用，以及原 anchor/offset 行为。主会话采纳 advisor A；不新增前置票。


## Main #109 Radio finite acceptance — 2026-10-25

Main accepted original#109 six criteria after applying12-file equivalent current-baseline packet, retaining#98 prepared admission/#116 ComboBox/#88Search/#79clock. Original RadioGroup/RadioGroupView remains one owner; validated controlled Option selection supportsNone and preserves original fallback/disabled behavior; per-part checked exposure and original once-queue separate logical index from authorization. ExistingRadio Playground uses retained ContentHost/RadioGroupContent via original pointer/key/AX and next normal dirtyframe; no second reducer/queue/scratchrender. Original model/View remain references.

Receipt `design/reviews/radio109-main-intake-20261025/root-acceptance.json`: current core11/public9/defaultroute6/externalroute6, three strict lanes,361-type default/SVG compiler tests; actualGPU60default+60fresh lockedexternal cases compare originalView and retain120PNG with positiveink/clip assertions. Final API-note-only registry sync separately revalidated default/SVG bindings and realRadio route; added unrelated formalR1virtualrow design is reconstructed to exact old prefix and absent from compiler deps. All old programs keep exact source windows, not re-signed. Four public support paths map existingtui.radio-group, including original model RadioSelectionError;45components/52roots unchanged.

Experimental maturity stays. No shownwindow/physicalIME/VoiceOver/performance/release/pin. #108/#110 separate, parent#91 not closed. #98 originalvirtualrow reading completion and#111remoteauthorization are independent of these localRadio criteria. Full#8normalfreshness/fullownedclosure remains open; old18 unchanged. PRnull.


## Main #110 Segmented finite acceptance — 2026-10-25

Main accepted the original#110 six criteria after serially applying the14-file/62-hunk Radio-base packet, preserving Radio/Search/clock/ComboBox and the original single SegmentedControl/SegmentedControlView/ButtonModel owners. Original pointer/Space down-up/repeat/navigation and focus-not-selection remain; controlled changed values fully validate, cancel original arms and publish, unchanged no-op/invalid unchanged. Checked per-part exposure, fixed logical indices, original once-intents/Cancel/lifecycle/ABA and original single-select group/item roles remain, with no Tabs impersonation or second reducer/queue. Existing Segmented Playground uses its retained Host and original dirty next frame; no new component identity.

Receipt `design/reviews/segment110-main-intake-20261025/root-acceptance.json`: current384 passing test invocations including Controls327, finitepublic23, default/external actualWorkbench9each, Radio coexist6, originalViewprobe1,default/SVG6bindings,explicitGPU processes; three strict lanes pass. A separately reconstructed current-before baseline executes one more probe, all648 layout/Scene outputs byte-identical after sharedgeometry extraction. BaselineCargo canonicalizesDEV/dev; initial source-prefix selector omitteddepinfo and was corrected in a separate samefile/sourcehash/currentexecutedprogram-SHA-bound addendum recovering11 .d and13freshlycompiled localartifacts, without rewriting the original receipt. Explicit-output probe now ignored by default and actually run withSSDoutput/--ignored in bothbefore/after, not counted as a normal ignored pass.

ActualMetal/CoreText pairs:108default+216two externalTheme definitions=324 cases/648actual-originalPNG, allRGBAexact and positiveink asserted; externalblue default108equal, externalinputs99/108differ provingusedoverride. Four additional actualGalleryThemeframes show selected1/focused0 with originalkeys, not a pixeloracle or shownwindow. Currentproducer freshly compiled byte-identical externalconfigs with independent/2lock; old artifact guard/oldsource seals not weakened/re-signed. PublicAPI inventory adds three supports to existingtui.segmented-control,364paths/defaultSVGcompiled,45components/52roots unchanged.

Experimental maturity unchanged; no shownwindow/physicalIME/VoiceOver/performance/release/pin. Checkbox#108 and parent#91 remain open, as do independent#98virtualrowreading/#111authorization/#8freshness/fullownedclosure. No全DS/native额外前提. PRnull.


## Main original Scroll interaction completion required — 2026-10-25

The earlier finite Radio/Segmented receipts remain valid for their exact source/test windows, but their acceptance conclusion is superseded for actual Scroll descendant interaction. Both original adapters inherit Unsupported action-transaction hooks. Original Host strict focus/interaction admission correctly refuses them under Scroll, so visible controls cannot obtain normal pointer/AX/Tab focus. Root-host interaction and controlled updates preserving Composer/reading do not prove that original composition works. Main/advisor classify this as original #109/#110 adapter completion, not a new enhancement or whole-library gate. #109 is reopened; #110 remains open.

Each original owner must supply pure original reducer decisions, final validation and consuming apply with compatible focus hooks. No unconditional StableHooks declaration, duplicate reducer/model/queue, or weakening Host Unsupported refusal is accepted. Add real visible Scroll pointer/AX/Tab positives, once action/focus-versus-selection and original marked Composer pointer-versus-strict-AX policy checks. Checkbox #108 must similarly verify its original Button owner path before main intake. Existing geometry and actual GPU seals keep their source identity; unrelated complete GPU matrices need not repeat for this finite interaction correction. Main has requested a private public-only red/root-positive audit; no native-window or full DS prerequisite is introduced.


### Private strict Scroll candidate receipt (2026-10-25)

The isolated follow-up candidate extracts finite pure decisions from the original RadioGroup selection and RadioGroupView focus/pointer owners. Existing mutable APIs and the Content adapter use the same decisions through the existing `prepare_original` final-validation and consuming-apply path. It does not replay mutable handlers, add a reducer/queue, change Host/Scroll policy, or modify Segmented production.

The candidate also corrects a disabled-row pointer case: ordinary Host focus may establish `Some(0)` while the original View event leaves focus unchanged, so applying a saved full focus snapshot would erase the Host result. The original decision now applies only fields written by that event. Its private ordinary/strict ordering probes and visible Scroll disabled-row test pass.

The handoff records 38 passing test invocations and three strict lanes: controls/private probes 15, public Scroll consumer 8, existing public Radio consumer 9, and Workbench route 6. One explicit actual-GPU test was ignored. Existing default/external Theme GPU seals retain their original source identity and were not rerun. The private evidence remains separate from main live integration; new GPU/pixel, native window/VoiceOver transport, physical IME and performance validation were not run.


## Main finite acceptance — 2026-10-25 / #108

CheckboxContent is integrated in the authoritative main worktree with its original model/View owner and retained ContentHost. The #108/#90 combined main window passed 462 CPU/consumer/Gallery/binding test invocations, two strict commands, and four actual Metal/CoreText GPU tests covering 712 original-View equality cases (Checkbox324+324, Combo32+32 default/external). Sources1689 were identical before/after; actual programs/dep-info/generated hashes are archived. Per-Issue original criteria remain separate. Receipt: `design/reviews/priority-intake-queue-20261025/main03-validation.json`.

Canonical public API inventory is387 types (45 components/52 roots unchanged); original default/SVG bindings passed3 each. Workbench retains original component identities. Scoped Checkbox/Combo composition rows carry exact current source anchors/hashes in their finite addendum. Old whole-library capabilities/probe/audit source identities are unchanged and stale aggregate freshness is tracked in #8/#14; this acceptance does not claim a fresh45-row audit. Shown window, physical IME, VoiceOver, performance and publish were not run; no PR/commit/push/release. Parent synchronization: #91 and #56.


## Main #110 finite acceptance - 2026-10-25

The five-file original input/Scroll correction is integrated on the authoritative Root387 inventory. Actual main six-lane validation passed controls356, public51 (Segmented14/Scroll4 plus Checkbox20/Combo13), original Gallery Segmented9 and Radio coexistence6:422 test invocations; controls+Gallery all-targets and finite public strict both passed. One explicit Gallery GPU fixture remained ignored. Sources1690 were identical before/after; actual programs/dep-info/generated hashes are archived in `design/reviews/segment110-scroll-main-intake-20261025/main-acceptance.json`.

Earlier384+1 main CPU and324 default/external GPU cases retain their original source/program identity. This correction does not change measure, arrange, paint or Theme, so those GPU fixtures were not rerun or relabelled. Public API inventory remains387 types, components45/roots52 unchanged. The current Segment row and affected Checkbox/Combo declaration anchors are finite increments; historical whole-library aggregate pins remain stale in #8/#14. Parent#91/#56 synchronizes this finite child only. Shown window/physical IME/VoiceOver/performance/publish not run; no PR/commit/push/release.


## #109 strict Scroll main CPU intake — 2026-10-25

Main integrated the five-path strict Radio correction against the current worktree, preserving the later #110 design text. The original RadioGroup/RadioGroupView remain the single decision owners; prepared execution consumes those same decisions and preserves Host-hook writes to fields the original event does not modify. No Host/Scroll admission policy or second reducer was added.

The new main batch radio109-main-04 passed 38 test invocations (15 owner, 8 public Scroll, 9 public Radio, 6 Workbench) and three strict lanes (controls all-targets, scoped public consumer, Gallery all-targets). All seven lanes retained 1704 source hashes and original runtime/cache identity; 320 archived program/library/dep-info/generated records were independently rehashed. The public actual-GPU test was explicitly ignored in this CPU batch. Original private 38/3-strict and earlier GPU receipts keep their own source/program identities. Current GPU follow-up and original Issue closure remain pending; no window, VO, physical IME, performance or publication claim is made. Receipt: design/reviews/radio109-strict-main-intake-20261025/main-cpu-strict-audit.json.


## #109 strict Scroll finite main acceptance — 2026-10-25

The current-main follow-up explicitly executed the original public actual-GPU test in two separate builds: default Palette and a freshly generated radio-external-inputs artifact with its independent source lock outside the artifact root. Each build passed the original 60 tuples (four modes, canvas/sidebar/toolbar, rest/focus/error/disabled/pressed) at 2x backing and 1.5 type scale. All 120 actual ContentHost/original RadioGroupView RGBA pairs matched with the original positive ink assertions; the exact tuple sets and PNG hashes were checked. The two program hashes differ, their Root source maps match, and all 1704 inputs stayed stable inside the original shared/GPU locks. Seventy-two GPU program/library/dep-info/generated archive records were rehashed.

Together with the separately recorded new main 38 tests and three strict passes, this accepts #109 original six criteria only. Public type exports, feature availability and actual Workbench Radio routes retain their existing identities; no new registered component or second state owner was added. Historical GPU/CPU/private receipts remain historical. Main receipt: design/reviews/radio109-strict-main-intake-20261025/root-acceptance.json. No native window, VoiceOver transport, physical IME, performance, PR, push or publication is claimed. Parent #91 overall completion is not implied.


## #91 finite parent acceptance — 2026-10-25

Main accepts original #91 five criteria through actual accepted #107 shared selection roles/bridge and #108 Checkbox, #109 Radio, #110 Segmented adapters. Each adapter retains its original model/View/reducer and independently exercised public consumer; original Composer/reading boundaries and typed once-only actions remain with their owners. API inventory, actual Workbench routes and this formal design are synchronized; no second selection engine or fake Group/Switch role is substituted.

Evidence is layered, not resigned: Checkbox keeps its current-main648 GPU cases; Radio has new main38 tests/three strict and default/external120 actual View/Content pairs. Segmented retains its earlier accepted324 GPU cases under their original source/program; the later exact five-path input/prepared-Scroll correction is paint-neutral and has main422 tests/two strict. Main/advisor verified that correction does not change measure/arrange/paint/Theme, so no duplicate three-component GPU sweep is required. The earlier segmented GPU is not claimed as rerun on current source.

Receipt: design/reviews/selection91-parent-main-intake-20261025/root-acceptance.json. This closes the three-control finite composition delivery only; #8 capability pins, complete Design System, real-window/VO/physical-IME/performance/publication remain independently scoped. No PR, push or release.


## 2026-10-30 当前主线恢复（#196）

原 #91/#107–110 的有限实现和收据保持原身份。当前 a0882d1 缺三 adapter 和五角色，本批仅回收固定 label、唯一 model/View、原操作一次和曝光退休合同。#101 的 remote dirty/revoke、checked-add 与 mount 批量预约已存在；局部 staged ID 计数尚缺，本批仅回收该最小差量及原失败测试。SearchInput/搜索 adapter 不属于本批。AppHost 已复用平台 snapshot，保留当前共享投影而不恢复旧独立接口。当前候选未编译，等待唯一集成冻结来源与分配共享构建窗口后定向验证。全局库存由集成聚合，不以历史 GPU/strict 改签当前源码通过。


## 2026-10-30 Menu 前置的 Selection 增量（#196 r5）

本轮以冻结 Menu widgets 7f621a46… 与 canonical role/schema 源为前置，Selection widgets 并集6193a0f4…直接复用已存双序证据。五角色、wire/native 映射、完整快照验证与 staged remote ID 已在共享源，不再次加入。仅回收原三 adapter、原 model/View 几何及 Host 曝光/查询接缝。原r4的根text snapshot和即时capture查询两处漏接明确在本轮按函数修正，原字节与历史收据保全；新增断言待冻结窗口执行。固定labels/唯一model/View/once/退休合同不变，没有Menu合同变更。只做源码/静态回放，未编译、未运行行为/GPU/native/strict。


## 2026-10-31 实际 Combo17 前置独立接纳（#196 r6）

本次来源固定 ce69f8e92b5c0dc48e7b2cd0704bd8a4042f4b4d。增量恢复 Selection 专属五角色、is_selection、三个module/export/payload与原r5三adapter/Host修正；新Content/wire各22role，仅比实际Combo17多五role。保留Combo、SetExpanded、native0aec修复、当前Text getter/准备路径及Menu当前使用者。不依赖Role32、Search/Collections/Navigation主体。remote staged ID和cfgtest快照helper已在Combo来源，不重复添加。旧完整decoder原字节捕获实际Combo17，保留Combo和SetExpanded，测试旧完整Reply拒新五role/无前缀与17个原role兼容；manifest不冒称已运行。三adapter源码hash与r4/r5相同，原固定label/唯一owner/once/退休合同保持。当前仅精确隔离回放/语法与身份检查；未Cargo/行为/GPU/native/Clippy。集成合并widgets当前Menu精确hunks后再冻结验证，不覆盖全文件、不等待未来Menu alias/Select/header。
