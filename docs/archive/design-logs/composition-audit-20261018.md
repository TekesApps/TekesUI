# #14：当前全组件组合能力矩阵与实施依赖（2026-10-18）

> 当前入口已更新为 [2026-10-20 机器矩阵与兼容说明](composition-audit-current-20261020.md)。下文表格中的旧统计、声明行与“当前”描述按其历史时点理解，不覆盖最新 source-bound JSON 或后续验收结论。

基线 `bc3de79466834e99062a1b4ef0872323e2860001` 加活跃 Gallery checkout 的**全部 WIP**；不是远端旧提交的推断。实际检查 DESIGN.md、docs/charter.md、docs/roadmap.md、当前 registry/公开 API、45组件实现、相关 Issue/PR 快照和已有消费者。机器明细：[composition-audit.json](../../design/remediation/composition-audit.json)，绑定27个作者/实现文件的SHA-256及真实声明行；本批收据：[receipt.json](../../design/reviews/composition-audit-20261018/receipt.json)。

旧 Container 表及最小协议第一、二批保留历史身份；本表替代其当前阻塞判断，不改写冻结收据。#14 是审计/规划入口，本批未执行全库迁移、创建子 Issue 或实现窗口 Toolbar。共享最小协议的原子移除缺陷按此前持续整改授权单独修复。

## 两个维度及证据边界

45项均记录。协议列：9项有当前公开接入，5项部分接入，31项缺少内建接入；这不是组件可用性或成熟度评级，也不是说消费者不能写公开newtype适配。所有94条API声明/适配证据都有当前源路径和行号。

“叶子”不要求children；“内容责任”包括既有字符串模板或几何入口，只有“共享Content槽”非空才表示真实owned子树。模型数据、Markdown AST、DiffLine、ChartDatum、Scene图元不是UI子节点。Menu可约束菜单项行为，不能以缺少任意表单控件判它失败。可参与组合与可容纳内容分别记录。

协议/内容状态使用已实现、部分实现、缺失、不适用；体验与原生未验证单列。当前全部组件仍experimental。已有独立消费者的系统AX/Preedit历史证据不能代替本轮全组件或生产Gallery验收。

## 全组件矩阵

| 组件 | 内容责任 / 既有输入 | 当前公共组合接入 | 共享Content槽 | 内容能力 | 主要边界 | 源证据 |
|---|---|---|---|---|---|---|
| Stack | 内容责任；count→Rect/StackView placement; no legacy owned children | 已实现：`StackContent` | `children*` | 已实现 | Intrinsic measured children; one optional flexible child; no implicit viewport or virtualization | `crates/tekes-ui-controls/src/extended.rs:20`<br>`crates/tekes-ui-controls/src/layout_view.rs:206`<br>`crates/tekes-ui-controls/src/composition.rs:1219` |
| Grid | 内容责任；count/columns/row_height→Rect; legacy fixed cell sizing | 已实现：`GridContent` | `cells*` | 已实现 | Shared measured row maxima; no per-cell span/alignment API or virtualization | `crates/tekes-ui-controls/src/extended.rs:61`<br>`crates/tekes-ui-controls/src/layout_view.rs:247`<br>`crates/tekes-ui-controls/src/composition.rs:1314` |
| Separator | 叶子；axis and frame; no child content | 缺失：无内建适配 | 无 | 不适用 | Decorative leaf; arbitrary children not applicable; no built-in Component adapter | `crates/tekes-ui-controls/src/extended.rs:103`<br>`crates/tekes-ui-controls/src/layout_view.rs:299` |
| SplitView | 内容责任；leading/divider/trailing geometry; caller paints panes | 缺失：无内建适配 | 无 | 部分实现 | Two pane geometry and existing drag/cancel state; no owned Content panes or recursive event/focus wiring | `crates/tekes-ui-controls/src/extended.rs:113`<br>`crates/tekes-ui-controls/src/layout_view.rs:344` |
| ScrollArea | 内容责任；ScrollArea viewport/content size and scrolling only | 已实现：`ScrollContent` | `children*` | 已实现 | Constructor accepts one subtree; current traversal supports multiple vertical children; no virtualized unmount/reconstruction guarantee | `crates/tekes-ui-controls/src/scroll_view.rs:27`<br>`crates/tekes-ui-controls/src/composition.rs:1371` |
| Disclosure | 内容责任；label string, optional icon and layout_body geometry | 缺失：无内建适配 | 无 | 部分实现 | Open state/reveal clip exists; body painting/state routing remain host-owned, no Content body slot | `crates/tekes-ui-controls/src/extended.rs:508`<br>`crates/tekes-ui-controls/src/layout_view.rs:708` |
| Surface / Panel | 内容责任；background and content Rect | 缺失：无内建适配 | 无 | 部分实现 | SurfaceView does not own/route contents; no arbitrary Content adapter despite reusable plate geometry | `crates/tekes-ui-controls/src/feedback_view.rs:152` |
| Toolbar | 内容责任；labels slice, equal-width production Button actions | 缺失：无内建适配 | 无 | 缺失 | Content ToolbarView is not Window UI; no Window registration/group/placement/overflow/custom Content contract | `crates/tekes-ui-controls/src/widgets.rs:2582`<br>`crates/tekes-ui-controls/src/feedback_view.rs:207` |
| Text | 叶子；TextSurface text/selection/scroll; TextContent string+TextRole | 部分实现：`TextContent` | 无 | 不适用 | TextContent is static plain text; TextSurface read-only via configured input adapter is a distinct selectable-text contract | `crates/tekes-ui-controls/src/text_surface.rs:60`<br>`crates/tekes-ui-controls/src/composition.rs:1641` |
| Icon | 叶子；canonical icon name, host IconRenderer and tint | 部分实现：无内建适配 | 无 | 不适用 | Independent consumer wraps actual IconView as a leaf Component; wrapper is consumer-owned, not a built-in IconContent | `crates/tekes-ui-controls/src/layout_view.rs:117` |
| Button | 动作/叶子及内容责任；label string; render_content_frame reuses original Button painter | 已实现：`ButtonContent`, `CompoundButton` | `content1` | 已实现 | Arbitrary decorative Content tree, one action/name/focus; nested independent interaction rejected; fixed child replacement requires explicit subtree rebuild | `crates/tekes-ui-controls/src/widgets.rs:298`<br>`crates/tekes-ui-controls/src/composition.rs:1518`<br>`crates/tekes-ui-controls/src/composition.rs:1698` |
| IconButton | 叶子；canonical icon renderer + IconButton model | 缺失：无内建适配 | 无 | 不适用 | Special icon-only convenience may retain restrictions; no direct Component adapter; CompoundButton+consumer Icon is a separate supported route | `crates/tekes-ui-controls/src/extended.rs:530`<br>`crates/tekes-ui-controls/src/layout_view.rs:929` |
| Link | 叶子；text label and Link model | 缺失：无内建适配 | 无 | 不适用 | Link semantics/presentation still use action runtime; no Content Link role/adapter or URL host callback in composition | `crates/tekes-ui-controls/src/extended.rs:559`<br>`crates/tekes-ui-controls/src/layout_view.rs:1102` |
| TextField | 叶子；TextSurface + TextInput/selection/IME state | 已实现：`TextFieldContent` | 无 | 不适用 | Real shared editor and FieldView; native text services required; no arbitrary label/accessory Content slots | `crates/tekes-ui-controls/src/text_surface.rs:60`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:164` |
| TextArea | 叶子；multiline TextSurface | 部分实现：`TextFieldContent` | 无 | 不适用 | Same public surface can be configured multiline; independent reading consumer exercises multiline; all TextArea-specific composed variants unverified | `crates/tekes-ui-controls/src/text_surface.rs:60`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:164` |
| SearchField | 叶子；TextSurface shared input model | 部分实现：`TextFieldContent` | 无 | 不适用 | Can host same text state, but search accessory/clear/search semantics are not a dedicated composition contract; do not infer SearchField-specific migration | `crates/tekes-ui-controls/src/text_surface.rs:60`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:164` |
| ComboBox | 动作/叶子及内容责任；items/query strings + filtered MenuView | 缺失：无内建适配 | 无 | 缺失 | Fixed selection/filter semantics; no Content label/editor/item slot; not an arbitrary form container | `crates/tekes-ui-controls/src/extended.rs:1240`<br>`crates/tekes-ui-controls/src/layout_view.rs:2311` |
| Slider | 叶子；finite range/value model and SliderView | 缺失：无内建适配 | 无 | 不适用 | Range is data, not child UI; no Component range semantics/actions adapter | `crates/tekes-ui-controls/src/selection.rs:34`<br>`crates/tekes-ui-controls/src/extended.rs:583` |
| Checkbox | 叶子；checkbox model and text label | 缺失：无内建适配 | 无 | 不适用 | No Checkbox Content role/adapter; do not reuse Switch role to claim semantic completion | `crates/tekes-ui-controls/src/widgets.rs:803` |
| RadioGroup | 内容责任；labels slice; indexed selection/roving buttons | 缺失：无内建适配 | 无 | 缺失 | Semantic item collection has fixed string presentation; no arbitrary label Content or radio/group AX composition | `crates/tekes-ui-controls/src/selection.rs:260`<br>`crates/tekes-ui-controls/src/extended.rs:665` |
| Switch | 叶子；Switch model plus optional label string | 已实现：`SwitchContent` | 无 | 不适用 | Real SwitchView behavior and checked value, no general child content; label/accessory freedom not required to preserve toggle semantics | `crates/tekes-ui-controls/src/selection.rs:604`<br>`crates/tekes-ui-controls/src/extended.rs:782`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:52` |
| SegmentedControl | 内容责任；labels slice and selected index | 缺失：无内建适配 | 无 | 缺失 | Own selection model but action-button presentation; no Content segments/selection-group role or panels | `crates/tekes-ui-controls/src/extended.rs:807`<br>`crates/tekes-ui-controls/src/layout_view.rs:1277` |
| Field | 内容责任；Field label/help/error strings and input Rect | 部分实现：`TextFieldContent` | 无 | 部分实现 | Input adapter reuses FieldView for real text editor; generic Field child/accessory Content slot is absent | `crates/tekes-ui-controls/src/widgets.rs:1230`<br>`crates/tekes-ui-controls/src/feedback_view.rs:321`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:164` |
| Select | 内容责任；labels slice, selection model and MenuView | 缺失：无内建适配 | 无 | 缺失 | Fixed menu-choice templates; no arbitrary Content item/label adapter; preserve choice semantics | `crates/tekes-ui-controls/src/widgets.rs:1165`<br>`crates/tekes-ui-controls/src/layout_view.rs:1979` |
| NumberField | 叶子；numeric value/step model, rendered field + increment/decrement | 缺失：无内建适配 | 无 | 不适用 | No composed numeric editor/range action contract; fixed stepper presentation is not arbitrary Content | `crates/tekes-ui-controls/src/widgets.rs:1186`<br>`crates/tekes-ui-controls/src/feedback_view.rs:518` |
| Tabs | 内容责任；labels slice + selected/roving Tabs model | 缺失：无内建适配 | 无 | 缺失 | No owned label/panel Content or stable label↔panel semantic relation in composition | `crates/tekes-ui-controls/src/widgets.rs:1065`<br>`crates/tekes-ui-controls/src/composite.rs:91` |
| Breadcrumb | 内容责任；hierarchical labels/current location | 缺失：无内建适配 | 无 | 缺失 | Fixed navigation hierarchy; no arbitrary label Content; does not become a free-form box | `crates/tekes-ui-controls/src/extended.rs:867`<br>`crates/tekes-ui-controls/src/layout_view.rs:1549` |
| CommandPalette | 内容责任；query/items strings + trigger and filtered menu | 缺失：无内建适配 | 无 | 缺失 | Caller still owns query editor wiring; no unified input/item Content subtree or semantic command palette adapter | `crates/tekes-ui-controls/src/extended.rs:1306`<br>`crates/tekes-ui-controls/src/layout_view.rs:2414` |
| List | 内容责任；List/ListView labels, count and index selection | 已实现：`ListContent` | `rows*` | 已实现 | General heterogeneous content organization is separate from legacy selectable List; no ListBox selection/virtualization guarantee | `crates/tekes-ui-controls/src/collections_view.rs:195`<br>`crates/tekes-ui-controls/src/extended.rs:877`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:14` |
| Tree | 内容责任；TreeNode label/parent indices/expanded/disabled | 缺失：无内建适配 | 无 | 缺失 | No arbitrary node Content; indices are not stable external child keys across reconstruction; tree semantics adapter absent | `crates/tekes-ui-controls/src/collections_view.rs:467`<br>`crates/tekes-ui-controls/src/extended.rs:918` |
| DataTable | 内容责任；headers strings, rows Vec<Vec<String>>, row_order indices | 缺失：无内建适配 | 无 | 缺失 | No Content header/cell/accessory slots, stable cell identity or composed cell focus/AX | `crates/tekes-ui-controls/src/collections_view.rs:860`<br>`crates/tekes-ui-controls/src/extended.rs:1062` |
| Chart | 叶子；BarChart/ChartDatum data and ChartView | 缺失：无内建适配 | 无 | 不适用 | ChartDatum is data, not a UI child; general composition adapter/range AX absent | `crates/tekes-ui-controls/src/chart.rs:33`<br>`crates/tekes-ui-controls/src/collections_view.rs:1307` |
| Menu | 动作/叶子及内容责任；item strings and production MenuView | 已实现：`MenuContent` | 无 | 部分实现 | MenuContent owns trigger/popup routing and semantic item parts; fixed menu item templates, not arbitrary inputs; dynamic item identity remains index-based; arbitrary interactive item content is intentionally outside menu semantics, not a missing general-form container | `crates/tekes-ui-controls/src/widgets.rs:1095`<br>`crates/tekes-ui-controls/src/composite.rs:446`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:500` |
| Tooltip | 叶子；Tooltip text, anchor and requested size | 缺失：无内建适配 | 无 | 不适用 | Constrained explanatory text, no Content subtree; arbitrary interactive content not implied | `crates/tekes-ui-controls/src/extended.rs:1112`<br>`crates/tekes-ui-controls/src/feedback_view.rs:758` |
| Popover | 内容责任；text plus render_with_content/render_with_text_content callbacks | 缺失：无内建适配 | 无 | 部分实现 | Open paint callback can draw arbitrary Scene primitives; it does not supply Content measure/event/focus/lifecycle/AX ownership | `crates/tekes-ui-controls/src/extended.rs:1117`<br>`crates/tekes-ui-controls/src/feedback_view.rs:893` |
| Dialog | 内容责任；title string + fixed action model/painter | 已实现：`DialogContent` | `header1`, `body1`, `footer1` | 已实现 | Three fixed Content slots, dynamic body subtree; modal scope and exit paint separate; nested modal rejected, no arbitrary slot reorder | `crates/tekes-ui-controls/src/widgets.rs:1139`<br>`crates/tekes-ui-controls/src/composite.rs:1336`<br>`crates/tekes-ui-controls/src/composition/adapters.rs:714` |
| Toast | 内容责任；message/tone/dismissible model | 缺失：无内建适配 | 无 | 缺失 | Text/dismiss action are fixed; no arbitrary Content or live-region semantics adapter | `crates/tekes-ui-controls/src/extended.rs:1139`<br>`crates/tekes-ui-controls/src/feedback_view.rs:1316` |
| Banner | 内容责任；message/tone/dismissible model | 缺失：无内建适配 | 无 | 缺失 | Fixed notification text/dismiss presentation; no Content or announcement semantics | `crates/tekes-ui-controls/src/extended.rs:1185`<br>`crates/tekes-ui-controls/src/feedback_view.rs:1578` |
| StatusBadge | 叶子；closed Info/Running/Succeeded/Failed enum | 缺失：无内建适配 | 无 | 不适用 | Leaf status glyph; consumer Badge(String) fixture is custom content, not production StatusBadgeView migration | `crates/tekes-ui-controls/src/extended.rs:1201`<br>`crates/tekes-ui-controls/src/feedback_view.rs:1815` |
| Progress | 叶子；value/indeterminate model | 缺失：无内建适配 | 无 | 不适用 | Range/animation data, no UI children; Component progress/range semantics absent | `crates/tekes-ui-controls/src/extended.rs:1208`<br>`crates/tekes-ui-controls/src/feedback_view.rs:1854` |
| EmptyState | 内容责任；title/detail/action strings | 缺失：无内建适配 | 无 | 缺失 | Fixed title/detail/action layout; no arbitrary Content slots | `crates/tekes-ui-controls/src/extended.rs:1233`<br>`crates/tekes-ui-controls/src/feedback_view.rs:1884` |
| Skeleton | 叶子；frame plus placeholder paint | 缺失：无内建适配 | 无 | 不适用 | Decorative leaf, children not applicable; no built-in Component adapter | `crates/tekes-ui-controls/src/feedback_view.rs:2067` |
| Markdown | 叶子；MarkdownBlock enum document data | 缺失：无内建适配 | 无 | 不适用 | Document blocks are not arbitrary UI children; no Component document-text/selection adapter; do not promote document AST into Component | `crates/tekes-ui-controls/src/collections_view.rs:1548`<br>`crates/tekes-ui-controls/src/extended.rs:1332` |
| CodeBlock | 叶子；code/language strings | 缺失：无内建适配 | 无 | 不适用 | Code is document data; no generic child slots or composed document-text semantics | `crates/tekes-ui-controls/src/collections_view.rs:1658`<br>`crates/tekes-ui-controls/src/extended.rs:1397` |
| DiffView | 叶子；DiffLine/DiffKind data and selection/scroll runtime | 缺失：无内建适配 | 无 | 不适用 | Lines remain data; no arbitrary Content cells or document accessibility adapter | `crates/tekes-ui-controls/src/collections_view.rs:1764`<br>`crates/tekes-ui-controls/src/extended.rs:1418` |

## 每项继承的运行时协作契约

| 维度 | 已接入Content的路径 | 尚为model/view的路径 |
|---|---|---|
| 身份/状态 | ComponentId独立于数组位置，Content持有Box<dyn Component>及原生产对象；主题/重布局不重建 | model/view对象持有状态，宿主保留实例；集合多使用索引，不能等同稳定子组件key |
| 测量/布局 | Component只必须实现measure；arrange/paint等可选，复用MeasuredChild及原布局原语；Stack/Grid/Scroll递归处理 | render接收frame/clip及文本/数据，部分内部测量固定标签；不等同通用子树布局 |
| 绘制/主题/裁剪 | 同一Scene、Palette及真实祖先clip；装饰前景显式继承，焦点不扩大命中框；Scroll统一gutter | 原View仍走共享JSON token与局部clip，但外部负责子内容拼接 |
| 事件/焦点 | child-first、消费/冒泡、pointer capture、Cancel、模态scope；输入法组字优先；动画复用host clock | 原View保留自身按钮/导航/拖拽等状态；全局路由和焦点尚需宿主接线 |
| 挂载/卸载 | insert/remove触发生命周期；卸载取消交互及焦点，返回owned子树；reorder移动对象 | 没有统一生命周期，宿主必须在禁用/失活/移除时取消原View交互 |
| 更新 | 事件changed、焦点和结构变化置dirty；render确认；frame request沿原文本服务接口 | 宿主处理返回值、重绘及动画，无ContentHost自动遍历 |
| 语义/AX | 同一实际active树的role/name/value/state/bounds/action；独立native例子桥接macOS | 大部分没有Content语义适配，数值/按钮样式不代表自身语义已导出 |
| 动态/重建 | Any/owned动态异构表示要求static对象；detach/reattach可由消费者保留实例。没有自动重建、序列化或虚拟化保证 | 实例和数据的重建/编辑保留由宿主负责，不能用列表字符串重排代替控件状态保留 |

共享协作源码锚点：

- `crates/tekes-ui-controls/src/composition.rs:174` — `pub trait Component: Debug + Any {`
- `crates/tekes-ui-controls/src/composition.rs:646` — `pub fn render(`
- `crates/tekes-ui-controls/src/composition.rs:795` — `pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {`
- `crates/tekes-ui-controls/src/composition.rs:969` — `pub fn focused_text(&self) -> Option<&crate::TextSurface> {`
- `crates/tekes-ui-controls/src/composition.rs:1092` — `pub fn insert(&mut self, parent: ComponentId, child: Content) -> Result<(), CompositionError> {`
- `crates/tekes-ui-controls/src/composition.rs:1118` — `pub fn remove(&mut self, id: ComponentId) -> Result<Content, CompositionError> {`
- `crates/tekes-ui-controls/src/composition.rs:1163` — `pub fn reorder(`
- `crates/tekes-ui-controls/src/composition.rs:1203` — `pub fn semantics(&self) -> Vec<SemanticNode> {`
- `crates/tekes-ui-controls/src/composition.rs:1023` — `pub fn show_dialog(&mut self, id: ComponentId) -> Result<(), CompositionError> {`
- `crates/tekes-ui-controls/src/composition.rs:1061` — `pub fn close_dialog(&mut self) -> bool {`
- `crates/tekes-ui-controls/src/composition.rs:431` — `fn lifecycle(&mut self, event: Lifecycle) {`
- `crates/tekes-ui-platform-macos/src/shared.rs:386` — `pub fn set_accessibility_nodes(&self, nodes: Vec<crate::accessibility::AccessibilityNode>) {`
- `examples/composition-consumer/examples/native.rs:302` — `w.set_accessibility_nodes(nodes);`

原生边界：主Gallery Host目前未发布控件AX节点；独立composition-consumer的native例子已连接现有macOS桥，两者不能混为一谈。ContentRole目前只有Group/Text/Button/TextInput/Switch/List/ListItem/Dialog/MenuButton/MenuItem；未来Slider/Checkbox/Radio/Tabs/Link/Progress/文档等接入前须补各自需要的语义，复用已有状态/边界，不把所有控件降成Button或Switch。

直接取得typed component并修改结构是低层逃生口，依赖下一次render重新检查duplicate key、slot数量和非法嵌套；几何改变后输入前必须render。受支持的Host insert/remove/reorder路径负责验证与生命周期。没有声称任意内部Vec写入都自动发Mount/Unmount。

## 最小场景与当前共享修复

三个独立最小场景已经有公共API、真实控件、真实文本/GPU及限定原生证据；旧表中的“缺少公共协议/输入适配/模态scope”不再是当前结论。本批在当前源复测独立消费者和协议测试，并重跑复合Button、异构List/RichDialog、presence的GPU场景；物理键盘、中日文候选窗、跨屏输入和VoiceOver完整体验仍不算通过。其他族迁移与窗口Toolbar继续等最小场景统一验收，未以本次矩阵授权全库迁移。

本批发现 `ContentHost::remove` 在确认目标可移除前就关闭其包含的Dialog，导致根节点/固定或不可变槽的失败操作改变模态、焦点和presentation。修复只增加实际可变子边界的预检查，复用原错误、remove和生命周期路径；无新状态机/token。适用于所有组合树，不只是RichDialog；可移除子树仍正常卸载/恢复invoker。

公共回归先在旧实现复现失败；修复后真实Dialog保持打开和焦点。独立RichDialog还保留TextSurface文本、选区、marked状态及完整语义，旧Theme参数形式的TextSurface.draw函数指针依旧可编译。实际GPU四主题×两宽度×两倍率×操作前后共32帧，16对像素完全一致；这是失败API调用不应改变体验的证据，不是原生输入/VoiceOver通过。

## 兼容性与迁移审计

| 当前变化 | 影响与策略 |
|---|---|
| composition模块及适配是添加型experimental入口 | 旧字符串/model/view入口保留；不批量替换旧消费者。Box<dyn Component>是当前最小消费者选择，不宣称借用式/静态DSL已定案。 |
| ButtonTextRenderer增加带默认实现的可选文本服务 | 原label-only实现继续有效；需要编辑器的组合明确报MissingTextServices，不要求所有文字适配器实现完整编辑服务。 |
| TextSurface.draw允许Palette及?Sized renderer | 旧Theme调用及显式旧函数指针在独立消费者编译回归通过；这不等同所有外部函数类型/二进制ABI兼容审计。 |
| 原Button增加render_content_frame | 复用同一press/cancel/focus/painter，没有新ButtonVariant；普通CompoundButton内独立交互被拒绝，这是新增组合API的约束。 |
| Event::Accessibility | 原Event已non_exhaustive，新增variant保持要求通配分支的源合同；辅助语义类型仍experimental。平台AccessibilityRole当前非non_exhaustive，未来扩role会影响穷举匹配，须显式迁移。 |
| Palette未知索引不再静默回退 | 行为改变已在#7明确：配置/选择边界用Palette::new或检查Option；非法索引不再冒充默认品牌。 |
| 新协议公开struct的字段/枚举 | 对未发布WIP仍有源码形状风险，需保持精确源版本；不宣称全外部consumer无Breaking Changes。后续减少公开struct literal耦合需有明确迁移方案，不能悄悄重构。 |
| 拒绝移除的原子性 | 既有错误类型和成功路径不变；失败不再隐式关闭Dialog。当前代码行为修正已由红→绿公共回归和独立真实输入证据确认。 |

## 窗口级Toolbar现状与规划

当前Toolbar::layout和ToolbarView只提供内容区几何/固定Button行；Window没有Toolbar注册或生命周期协议。以下是规划，不是存在的API，也不自动新增registry组件。

| 所需能力 | 复用与实施边界 |
|---|---|
| Window拥有稳定item ID、注册/注销/动态更新 | 复用ContentId/owned对象思路和现有Window/CommandId窄边界；每个窗口独立持有，不放Gallery全局单例 |
| Group、Leading/Center/Trailing、固定/弹性间隔 | 先用既有measure/layout/spacing；仅现有JSON不能表达确定语义时再补设计定义；不新建一套私有像素布局 |
| 自定义内容 | item使用同一种Component/Content边界，复用真实Search/TextField/Menu/Switch/IconButton及消费者组件；不是闭合ItemVariant枚举。缺适配/语义先在共享层补齐 |
| 尺寸、压缩、overflow | child报告测量/约束，宿主按公开优先级协调窗口宽度；移入overflow保持稳定状态。菜单不强塞编辑器；编辑中条目保留/取消策略须先明确 |
| 命令、状态、快捷键、窗口隔离 | 复用Window菜单/CommandId派发及相同业务回调，状态来自真实组件；不复制第二套command或toggle状态机 |
| Chrome/内容与关闭 | Window协调chrome/content viewport、输入焦点与AX parent；关闭取消capture/IME/动画、卸载owned子树和AX节点；不能仅在内容顶部画Stack |
| 跨平台与扩展布局 | 公共API不暴露NSToolbar；继续纯Rust+单surface路线。必要系统能力依据charter走平台窄接口，不增加Swift、GPUI或AppKit子View树 |

## 执行顺序、依赖与剩余验收

1. 当前公共核心与三个最小消费者：保留已有实现，修复本批原子移除缺陷并完成相应回归。继续原IME候选窗/VoiceOver和输入、焦点、Reduced Motion统一验收；仅Gallery桌面操作已授权，系统设置和其他原生应用不自动获得授权。
2. 可并行推进未迁移组件族的既有状态与真实token依赖核验；不因原生局部阻塞停掉可执行测试/修复。优先补交互/语义共享缺口，不能只换Button外观。
3. 最小协议验收满足后选择一个组件族迁移：先Tabs label/panel关系或Disclosure/Popover的已存在内容入口，再Tree/Table等集合；旧便利API保留。每批必须同时有公共API、独立真实接入和Gold7证据；不是按此表立即全库迁移。
4. 窗口Toolbar复用已经验证的组合、焦点/语义、命令边界，先独立消费者单窗口，再动态内容/overflow/多窗口；只有明确新增范围后才登记新组件。
5. 虚拟化必须先规定稳定数据key、实例驻留/卸载/重建和编辑保留。当前owned detach/reattach不是虚拟化；活跃系统组字不能假装由普通JSON快照恢复。复用DraftJournal保存业务草稿，但不把它当作IME会话序列化。

ListBox、Sidebar、Toggle、Accordion、ToolbarGroup、ToolbarItem未发现当前独立公开组件/registry项；候选名称不表示已经存在，也不自动扩入实现范围。性能按用户要求排除，不作为本次授权工作的阻塞。#5保留系统AX/VoiceOver责任，#8保留能力与状态登记，#12保留完整统一验收，#13总账继续active。未创建/关闭Issue、提交、推送、tag或发布。

2026-10-18动作族复核：IconButton/Link在事件边界读取当前模型可用状态，SegmentedControl清除禁用期间旧Space按压并修复漫游焦点。公共组合槽和接入状态不变；对应类型声明行号与源哈希已针对该修改更新，旧冻结审计收据不改写。

2026-10-18 整体可用性复核：SegmentedControlView 复用 cancel/blur 增加整体禁用，保留模型值与单项选择；未改变组合矩阵的内容容量或适配结论。

2026-10-18 图标按钮复核：自然尺寸与语义圆角绑定已有 token；复用 Button 交互和绘制，显式 frame 保持。组合内容容量、协议适配结论不变。

2026-10-18 导航族复核：Disclosure/Breadcrumb 的可用性、索引和语义半径修正；Icon/Tree不存在尺寸引用改用已有语义。未新增组合适配，Tabs/Disclosure 的内容状态仍单列未验证。

导航族最终复核：Disclosure 展开动作使用已有 Quiet 强调角色及 radius-row；所有消费者默认受益。Gallery 的 Link/Breadcrumb Space 路由保留公共导航语义，未改变组合接口与内容容量结论。

## 内容状态最小消费者补证（2026-10-18）

复用 active、StackContent、TextFieldContent、Progress/Banner/EmptyStateView，实现 Tabs/Disclosure 调用方组合；未新增内置适配器或全库迁移。ContentHost 现在在输入边界核对完整 active 祖先链，取消隐藏焦点/捕获并清理隐藏祖先内的模态。引用文字渲染器沿原服务接口转发；完整语义角色和生产 Gallery AX 不因此晋级。详见 [分层证据](content-status-20261018.md)。

选择弹层契约补证（2026-10-18）：现有 FilteredMenuView 缓存的候选映射和标签共同约束指针释放；实时禁用、关闭和数据变化撤销旧捕获。CommandPalette 禁用时不能打开或清除查询；Select 复用已有 setter。内置组合适配和语义能力维持原状态，见 [分层证据](choice-contract-20261018.md)。

弹层几何补证（2026-10-18）：MenuView::measure 与生产行测量共用实现；原 overlay 定位函数复用真实锚点，菜单按可用一侧限制高度并保持滚动，无法显示一整行的极小视口保留边界内覆盖降级。FilteredMenuView 空候选由宿主呈现反馈；未新增内容适配器或门户窗口，见 [分层证据](popup-geometry-20261018.md)。

CommandPalette 触发器状态补证：pointer_move 复用已有 Button；新增 blur_trigger 只转交触发器焦点，不关闭正在查询的弹层。Gallery 同步冷启动焦点、查询接管、Escape 恢复，并结束拖出后的触发器捕获。五项触发器状态不代表整个 choice 协议完成；见 [分层证据](choice-hover-20261018.md)。

Select 关闭点击由原 View 结束按压，Gallery 不再以 trigger blur 补救；Select／ComboBox 复用已测量夹具和 TextSurface 指针能力。原生发现的确认键76遗漏在 Gallery 统一入口归一为既有 Enter 路径，独立消费者已有相同绑定；平台事件原码不变。15项状态仍与 loading/error、Select空反馈和原生AX区分，见 [选择状态证据](choice-selection-20261018.md)。

选择族关闭焦点补证：SelectView 在键盘关闭后恢复其自有触发器焦点呈现；普通 Menu 仍返回 caller target，由 Gallery 同步已有 Button 的键盘焦点状态。CommandPalette 既有关闭入口已同步 trigger focus。复用既有能力，不改变内容或组合协议；见 [选择关闭焦点](choice-focus-20261018.md)。

ComboBox／CommandPalette 的 key() 确认事件与选中值变化分离，重复执行同一命令仍通知消费者；沿用既有 key_event，无新协议。CommandPalette 三项结果状态和测量夹具补证见 [证据](command-results-20261018.md)。

Select 空选项复用既有取消与不可用呈现，当前count在下一帧前也约束事件；set_error投影已有共享验证边界，不阻塞修正或重置值。无新协议/token/state层，异步内容失败不算本批验证错误完成，见 [证据](select-availability-20261018.md)。

Select独立后台刷新消费者复用ContentHost/Stack/Status/Progress/Banner；SelectView拆出触发器和菜单绘制入口，原render仍委托相同实现，无新异步协议。候选、焦点、缓存值在状态变更中保留，初始空集和验证错误分别处理；见 [证据](select-async-20261018.md)。

ComboBox独立刷新消费者复用TextFieldContent、ComboBoxView与现有Status/Feedback。ContentHost先交付合成编辑器的控制键和捕获指针的移动/松手，避免候选弹层抢走IME或选区生命周期。无新增抽象/token；见 [证据](combo-async-20261018.md)。

CommandPalette独立刷新消费者复用原CommandPaletteView触发器/菜单、TextFieldContent、Status/Feedback和已有ContentHost路由。状态变更保留查询/选区/IME/候选；确认或取消恢复原触发器焦点。无需新增共享协议/token，见 [证据](command-async-20261018.md)。

选择运行时在事件边界观察到禁用时释放旧交互：Slider复用blur，Switch/RadioGroup复用cancel，无新抽象。Slider独立workspace六态/真实token及Gallery拖动样例已核验；Component range语义/actions适配仍缺失，见 [证据](range-availability-20261018.md)。

SliderContent已接入原Component协议，复用原Slider/SliderView，范围元数据为可选trait方法，值/焦点/捕获/相对增减统一验证。native快照字段未改变，范围描述随Slider角色传递；新Role/Action的穷举匹配需显式更新。实际原生AX/VoiceOver仍待验收，见 [证据](slider-composition-20261018.md)。


## 2026-10-19 用户补充：嵌套组件的 Toolbar 注册与呈现位置

当前源码再次核对：`widgets.rs::Toolbar` 只计算条目矩形，`feedback_view.rs::ToolbarView` 接收 labels 并拥有固定 Button；公共 `Component`、`ContentHost` 和平台 `Window` 均没有 Toolbar 贡献／注册／解析／注销接口。现有 `WindowWake` 注册与菜单 CommandId 不构成 Toolbar 注册机制。以下是用户明确的目标与实施验收补充，不是现存 API 或已通过的实现。

### 两个独立的位置维度

- **呈现归属／Target：**所属 Root Window 的窗口级 Toolbar，或声明者 Component 自己的局部 Toolbar。这里的 Root 是该组件当前所属的窗口；不解释为进程全局窗口，也不凭此增加一个独立浮动窗口。普通容器嵌套不截断 Root 归属。
- **排列位置／Placement：**目标 Toolbar 内的 Leading／Center／Trailing、分组及组内顺序。Target 与 Placement 不混为一个 Position 枚举，避免无法同时表达“根窗口＋右侧”或“组件内部＋左侧”。

任意深度的消费者 Component 都应能贡献 Toolbar 内容；这是可选能力，不要求每个叶子组件提供工具项，也不把大型注册职责强塞进每个组件必须实现的 Trait。工具项使用同一公共 Component／内容组合机制，允许按钮、搜索、菜单、状态或消费者自定义复合内容。先复用既有 ComponentId、生命周期、测量、焦点、语义和动作出口；只有现有能力不足以准确表达注册所有权时才补最小契约。

### 所有权与合并规则

1. 声明者是来源与业务状态所有者；Root Window 管理该窗口工具项的注册、布局与呈现。局部 Target 由对应组件的内容宿主管理。一个工具项同一时刻只呈现在一个目标，不复制编辑器或业务状态机。
2. 稳定身份使用“来源组件＋来源内条目 ID”的边界；重复声明更新同一条目，冲突须明确拒绝或诊断，不允许每帧累积注册。重排不重建条目内容。
3. 同一 Root 中来自多个嵌套组件的工具项统一合并，按明确分组／位置／顺序确定性排列，不由渲染调用先后决定。声明的可见性与启用状态影响呈现；不得擅自把“所有已挂载组件的贡献”改为“仅当前焦点组件的贡献”。若后续需要上下文替换策略，须单独明确。
4. 挂载注册、卸载注销；窗口关闭清理全部贡献。组件迁入另一个 Root 时撤销旧归属并重新绑定，不在其他窗口留下条目。普通布局、主题切换、窗口缩放不重复注册。
5. 工具项动作回到正确来源／既有命令处理路径；禁用、失活、卸载后的迟到动作被拒绝。来源位置与实际呈现位置不同，焦点遍历、命中和 AX parent/frame 必须以真实呈现为准。
6. 修改 Target、移入 overflow 或卸载编辑型条目时，明确焦点、capture 和 IME 的保留／取消；不得复制正在组字的编辑器，也不得用序列化伪造 IME 恢复。纯布局改变优先保留同一实例。

### 最小实施和分层验收

先交付“一个 Root Window＋父／子／孙三个来源组件”的独立公共消费者：验证三个深度均能向根窗口贡献工具项，并能选择局部呈现；同一个来源贡献左／中／右分组，注册／更新／重排／卸载可复现。然后验证第二个窗口的隔离及组件归属变化，再补压缩／overflow 与窗口 Chrome 协调；不得以 Gallery 内容顶部的一排按钮代替窗口级注册。

公共 API 回归检查稳定身份、重复注册诊断、动态状态、生命周期及命令来源；真实消费者检查嵌套组合和事件／焦点／编辑保留；外接屏原生与 Gold7 检查真实窗口呈现、键盘／AX 和适用 IME。沿用小宿主与公共缓存，验收后清理产物；不测性能。当前只补审计与要求，不将 Toolbar 成熟度提升，不改 AgentKit 冻结包，不提交／推送／关闭 Issue。

## 2026-10-20 当前 WIP 复核

逐项审阅矩阵过期的 11 个源输入后，修正以下当前结论：Surface 虽无内置 Content 适配器，外部消费者已用公开 Component 与 SurfaceView 组合只读正文、按钮及脚注；TextFieldContent 的无装饰只读模式已公开；NumberField 可用性和旧按压取消已修复，但仍是非编辑型步进器；Gallery 已有 host/default-page/workbench 语义导出，不能继续描述成尚未导出。全族 Inspector／VoiceOver 验收仍独立未完成。

共享所有权、事件、焦点、IME、生命周期和兼容边界不变。Scene3D 与 TickingNumber 相关 WIP 保留；审阅它们对现有绑定的影响不代表接受新增体验。较早 #17–#21 的“物理输入／公开 pin 待补”检查点明确标记为历史，后续 formal431 独立验收与关闭记录取代其交付阻塞，不覆盖旧源码包或原始失败证据。

当前矩阵检查覆盖 45 个组件及精确源哈希／声明行号，8 项审计回归通过。证据：`design/reviews/composition-audit-current-20261020/`。品牌生成一致性、参考关系负例、DS 完整性及 Gold7 机械检查本轮通过；不将这些机械证据提升为整页或原生体验通过。第二批仍缺同一公开交付版本的远端门禁和独立最终审阅，后续 Toolbar 注册／全族迁移按既定批次继续。
