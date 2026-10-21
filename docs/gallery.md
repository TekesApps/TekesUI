# Gallery workbench

Gallery 是纯 Rust 原生工作台，使用现有 wgpu / CoreText / AppKit Rust 绑定；没有 Web Storybook、Swift、GPUI 或第三方 UI 组件库。设计权威仍是 [design/system.json](../design/system.json) 和 [DESIGN.md](../DESIGN.md)。本文是当前导航、Story 和扩展约定的唯一维护文档；旧 handoff 和 golden 仅保留历史证据。

## 注册和信息架构

日常整改优先使用当前组件已有的独立小宿主，完整 Gallery 留给共享契约或组件族的集成检查点。
构建、共享缓存、分层验收与验收后清理统一通过 [组件验收流程](component-review.md) 执行；
没有独立场景的组件随实际整改接入，不提前扩范围，也不把运行时页面过滤称为小构建。

[design/gallery/workbench.json](../design/gallery/workbench.json) 集中维护名称、规范入口、分类、渲染器、文档、关键词和成熟度。`build.rs` 从同一注册表生成页面枚举列表；侧栏、菜单、搜索、路由和快照遍历共同使用它。新增已有渲染器的 Story 只加一条注册记录，不再重复登记名称、分组和路由。新增真实组件仍须实现生产控件和对应渲染适配器。

新导航完全替换旧目录：

- Overview：Introduction、Getting started、Component index（可点击组件索引）。
- Foundations：Colors & themes、Typography、Iconography、Spacing & sizing、Shape & elevation、Motion、Interaction & accessibility。
- Components：Layout、Content、Actions、Inputs & selection、Navigation、Collections & data、Overlays、Feedback、Documents & code；同行组件各有独立入口。
- Patterns：Forms & validation、Search & filtering、Master–detail、Async & error recovery、Destructive actions。
- Examples：Settings。File browser / Data workspace 未建立空入口。
- Quality lab：Rendering calibration、Cross-component consistency、Theme & brand matrix、Accessibility checks、Stress & performance；另有 Raster & compositing checks 和 Internal text diagnostics。

按后续用户指示，缺失组件曾保留明确标注 `planned` 的坑位；本次组件补齐将原 28 个坑位接入生产模型和运行示例。当前共 68 条记录、45 个唯一组件 ID，均为 `experimental` 组件入口。`experimental` 表示已有基础实现但未完成全部生产验收，不能理解为 stable，也不能理解为导入规范中的全部能力均已完成。

TextEditor 当前是内部文字适配器，和单行 / 多行 TextInput 共用 UTF-16、选区、IME、撤销和滚动模型，没有独立公开文档编辑契约。公开入口是 TextField / TextArea / Text；内部调试留在 Quality lab。SelectionSurface 同样没有独立公开契约，不建立重复公开组件入口。

## 旧新映射

旧页面枚举和少量数值命令暂供历史校准、性能夹具使用，不出现在新导航，也不是另一套规范目录。新的组件和 Story 以注册表路由为准；不再要求新目录沿用旧 ID。

| 旧页面 / 内容 | 新规范入口 |
|---|---|
| Token Swatches 的完整色板、间距、圆角和描边校准 | Foundations / Colors & themes → Complete token atlas，直接复用原 tokens::build；Semantic colors 保留新语义色板 |
| 原色板页的间距和尺寸 | Foundations / Spacing & sizing |
| Primitives 的圆角、描边、层次规则 | Foundations / Shape & elevation：数值、描边样本及必要/装饰边界规则 |
| Primitives 的 alpha 色块叠放 | Foundations / Colors & themes：Opacity & compositing，展示各档透明度和交叠 |
| Primitives 的通用状态插图 | Foundations / Interaction & accessibility；明确是规则样本，实际交互在组件 Story 验证 |
| Primitives 的像素对齐与合成检查 | Quality lab / Raster & compositing checks (`lab-geometry`)：整数/半点位置与 alpha 接缝检查 |
| Primitives 的未接入参照槽 | 不在新工作台绘制空参照图/热图；原始完整 build 只保留为历史 golden 夹具 |
| Static text / static-text | Quality lab / Rendering calibration |
| Foundations 分组的 Text、ScrollArea | Components / Content / Text；Components / Layout / ScrollArea |
| Icons 资产和名称搜索 | Components / Content / Icon |
| 字体 / 图标规范 | Foundations / Typography；Foundations / Iconography；不复制组件 Story |
| TextEditing 单行 / 多行演示 | Components / Inputs & selection / TextField、TextArea |
| TextEditor / SelectionSurface 内部实现 | Quality lab / Internal text diagnostics |
| Motion | Foundations / Motion，继续复用现有动效控制器 |
| Actions、Forms、Navigation、Overlays、Feedback、Documents、Data 的 40 项插图 | Quality lab / Cross-component consistency；原插图实现只维护一份，有交互实现的规范入口移到相应 Components 分类，其余为 planned |

`catalog.json` 仅用于历史状态插图，不再驱动规范组件导航。旧的 `TEKES_SELFTEST=gallery|components|foundation` 中包含旧布局坐标，属于历史脚本；本次验收使用新的 `workbench` 脚本，不声称旧坐标脚本适用于新导航。

## 布局、组件和窗口边界

Design System JSON 是语义 Token、间距/尺寸、交互约定和设计规则的权威来源；它不是运行时页面树，也不会自动计算 Gallery 的页面或窗口布局。Gallery 当前用 Rust 几何代码组合页面；Stack、Grid、SplitView 等提供有限的可复用布局能力，但项目还没有统一的保留式布局树或通用约束求解器。容器仍由页面适配器确定尺寸、裁切范围、滚动策略和子项位置。

`tekes-ui-controls::Button` 管理控件状态、Token 绘制、布局和保留的可见命中范围；Gallery 的 `TextEngine` 只适配文字塑形与字形输出。宿主用同一个 Button 布局结果绘制、裁切和路由指针，Button 内边距与字号通过 JSON Token 解析。Workbench 的 action / quiet action、Motion 动作和静态文字校准页的实时夹具 / 宽度选择器都使用生产 Button；Motion 也不再用自制的按钮绘制加独立命中框。`Locale::Reference` 只用于离线历史截图校准：它保留旧选择器的静态绘制和固定行距，不创建生产 Button 布局、命中区或键盘焦点。该只读路径不属于可访问的 Gallery locale。静态 state/raster/历史截图矩阵仍是明确的视觉校准夹具，不是组件交互实例。其他控件由各自当前的生产模型或适配器负责，本约定不表示所有控件都已统一为完整组件对象。


Workbench 中的实时渲染页面使用 `Workbench::preview_rect`，独立页面使用 viewport 内容矩形。Motion 的绘制与事件路由都必须使用 `Gallery::live_content_bounds()`；此前 Motion 事件路由固定使用 viewport 内容矩形，导致 Foundation / Motion 的绘制原点在预览区，而 hover/点击原点仍在页面区。坐标回归覆盖真实 Gallery 场景构建、预览 hover/click、滚动、重排和独立 Motion 页面。任何新增 renderer 适配都应让绘制、裁切和事件命中使用相同坐标空间。

文字组件同样由页面/预览容器提供唯一的祖先裁切矩形。Text、TextArea、TextField、搜索框以及 ComboBox / CommandPalette 查询框使用同一 `TextDemo` / `TextSurface` 绘制、caret 和可见命中几何；Workbench 先用当前 `TextEngine` 测量输入行高，再排布后续说明和操作，缩放字号时不让控件压住相邻内容。指针落在裁切范围以外不能让隐藏文字面获得焦点。浮层可以超出输入框本身，但仍受预览和窗口 viewport 裁切。TextEditor 的多行区域按剩余内容高度布局；ScrollArea Story 直接把组件 viewport 填入预览剩余空间，不再套 Gallery 页面滚动区。独立页面和预览内页面均由实际可见的宿主 viewport 最终裁切。

共享几何继续扩展到侧栏导航和 Master–detail 主列表；两者都从 `ScrollArea::geometry` 取得 viewport、document、offset 与可见区域。布局和绘制始终使用完整的组件 frame，`visible` 区域只用于裁切、可见性筛选和命中转换；不要把裁切后的矩形重新作为组件 frame 排版，否则文字基线和控件对齐会随裁切边缘漂移。Checkbox 使用生产 `Checkbox` 绘制，标签按实际首行 linebox 的中心对齐复选框；长标签保持首行对齐，续行按文本排版继续显示。此前共享组件布局的阶段记录见[组件布局跟进记录](../design/reviews/component-layout-followup-20261016/summary.json)。

`SplitView`、`Slider`、`RadioGroup`、`Switch`、`IconButton` 和 `Link` 已消费生产几何。`SliderView`、`RadioGroupView`、`SwitchView` 将选择控件的尺寸、文字槽位、绘制、裁切命中范围以及键盘 / 指针输入状态保留在控件包；Gallery 只提供文字塑形、Token、标签和故事夹具。RadioGroup 全组禁用不改动模型选择；多行标签按首行 linebox 对齐；Slider 拖动可超出 frame 并夹取到端点；窗口失活、焦点离开和组件被裁切会取消输入臂。`TabsView`、`MenuView` 和 `DialogView` 同样由生产控件包持有布局、绘制、裁切命中范围和键盘 / 指针状态。Select 保留其公开 `Select.menu` 模型，并借用 MenuView 的运行时呈现，不复制菜单状态。Tabs 仅选中项是 Tab 停靠点；Menu 的弹层滚轮由其 ScrollArea 消费；Dialog 在 Story 范围拦截背景点击和滚轮，并在离开窗口时保留打开状态、取消输入臂。Gallery 的外部 invoker 恢复仍会校验可见且启用。本轮未新增 Gallery 组件，也未重设计 Design System 视觉；外接屏原生复测仍按用户要求留待办。阶段边界与检查见[Tabs / Menu / Dialog 接线记录](../design/reviews/tabs-menu-dialog-20261016/summary.json)及[Slider / RadioGroup / Switch 接线记录](../design/reviews/selection-components-20261017/summary.json)。

布局与导航原语继续使用 `StackView`、`GridView`、`SeparatorView`、`SplitViewView`、`DisclosureView`、`IconButtonView`、`LinkView`、`SegmentedControlView`、`BreadcrumbView`、`ComboBoxView` 和 `CommandPaletteView`。Gallery 只提供容器、稳定 fixture 和现有图标绘制后端；组件视图拥有排布、Token 绘制、裁切后的命中区域、键盘 / 指针按压状态及失焦取消。ComboBox / CommandPalette 的候选项复用 `MenuView`，不再由页面各自摆放普通按钮；Gallery 路由将“事件已消费”与“模型状态已变化”分开处理。堆栈与网格布局从各自视图持有的模型生成，避免页面再维护一份脱节的布局状态。集合、文档和反馈页面也通过各自组件适配器接入同一 Gallery 容器；静态状态矩阵仍是比较样本，不替代真实交互实例。

### 共享对齐策略

`design/source/layout.json` 的 `alignment.presets` 是布局对齐策略的唯一设计来源；`scripts/merge-design-system.py` 将其投影到 `design/tokens.json`，设计导出 crate 再编译成 `tekes_ui_tokens::proposal::layout` 中的 Rust 枚举、`AlignmentPolicy`、命名常量和 `alignment(AlignmentPreset)` getter。运行时不读取 JSON。预设区分主轴分布、副轴对齐和文字对齐：`ROW` 是主轴起始、副轴居中；`COLUMN` 是主轴起始、副轴 leading；`TEXT_ROW` 使用首基线；`BUTTON_CONTENT`、`ICON_ARTBOARD` 和 `CHART_LABEL` 为各自用途表达居中规则。这些是项目的布局约定，不是把特定框架的默认行为当作平台规范。

生产几何可调用 `tekes_ui_controls::layout::layout_measured_children(bounds, axis, children, gap, policy, overrides)`，输入已测量的逻辑像素尺寸和可选首基线，输出完整子项 frame 与 bounds 内的可见裁切矩形。它保留小数坐标与固有尺寸，不按像素取整，也不在几何阶段重测文字；调用方应先按实际可用宽度测量长文本，再用 `visible` 裁切绘制和命中。首基线缺失时采用副轴居中回退；垂直轴不将首基线作为对齐规则。当前 `StackView` / `GridView` 的等分 cell API 继续兼容，本 helper 用于显式按测量尺寸排布的共享行列，不替代窗口约束求解器。

当前 Gallery 入口创建一个 Gallery 状态并放入一个原生窗口。多个容器可在单个窗口内使用独立布局和滚动区域；Gallery 尚无多原生窗口之间的布局、焦点转移或窗口级组件编排层。

策略的实际消费范围需和可用 API 区分：Disclosure 使用共享行布局，Tree 使用固定图标槽保持同层级标签对齐，IconView 使用图标画板居中，Chart 使用柱中心文字锚点，NumberField 的步进按钮使用按钮文字居中。`COLUMN` / `TEXT_ROW` 已有几何测试，尚未据此重写全部组件。容器副轴居中、段落文字对齐、字体基线和图标笔画中心是不同的约束，不能用一个全局 center 替代。示例中的说明也必须按真实可用宽度测量，放在组件返回的 frame 之后，不能借用窄按钮的 frame 或固定行高。

参考 [Apple HIG Layout](https://developer.apple.com/design/human-interface-guidelines/layout)、[Stack 布局文档](https://developer.apple.com/documentation/swiftui/building-layouts-with-stack-views)及 [SF Symbols](https://developer.apple.com/design/human-interface-guidelines/sf-symbols) 中的阅读顺序、主副轴区分和符号光学对齐原则；这里的项目预设不等于某个框架的强制默认。Figma 可作为对照稿，JSON 仍是规范真源。本轮不改变既有颜色、字号、间距、圆角和品牌值；修复证据集中保存在 [对齐检查回执](../design/reviews/alignment-20261017/summary.json)，不将离屏渲染视作原生性能或辅助功能验收。

## Story 模板

组件共用 `gallery/workbench.rs` 的示例容器、属性控件、事件日志、重置和初始夹具。可用栏目为 Docs、Playground、State matrix、Behaviors、Edge cases、Variants / sizes；依据实际能力隐藏不适用栏目，不补空示例。Button 正式展示 Primary / Quiet 与三个示例尺寸，实例的完整绘制和输入路由由生产 `Button` 持有；状态矩阵使用共享绘制适配器，模拟 focus-visible 是视觉夹具，实际 Tab 焦点另行驱动。

优先组件：

| 入口 | 实际模型 / 行为 |
|---|---|
| Button | `controls::Button`；按下、内部释放触发、外部释放取消；Enter 按下 / Space 释放；disabled / loading 禁止触发 |
| Checkbox | `controls::Checkbox`；unchecked / checked / mixed，Space 切换；checked + disabled |
| Tabs | `controls::TabsView`；生产 Button 几何与状态，单一 roving tab stop，Left / Right、Home / End，Enter 按下 / Space 释放时激活 |
| Menu | `controls::MenuView` + `controls::Menu`；生产弹层和内部滚动，Up / Down、Home / End、Enter / Space 按下执行、Escape、外部点击关闭并继续派发 |
| Dialog | `controls::DialogView` + `controls::Dialog`；生产模态布局与动作，Tab / Shift+Tab 约束、Escape 取消、遮罩阻断穿透、关闭恢复有效 invoker |
| TextField / TextArea / SearchField | 复用生产 `TextInput` 和原生文字桥；单行 / 多行 / 查询变体，实际选择、合成输入、剪贴板、撤销；disabled 在模型层阻止编辑 |
| ScrollArea | 原有生产 `ScrollArea`；独立轴策略、viewport、原生 momentum、bounce 和 thumb；展示页的文字区域填满剩余 preview，避免额外外层滚动 |
| Text / Icon | 复用原有可选择只读文字、真实 SVG 图标资产和查询适配器，不用截图替代 |

单个 Story 的状态局部保留，切页会释放活动输入和浮层，Reset 恢复初始数据和日志。没有真实网络、随机数、用户文件或墙上时钟依赖。异步场景由显式 Resolve fixture 完成，第一次失败，重试成功。

## 组合场景和 P0 盘点

真实组合场景均复用 `tekes-ui-controls` 的模型和共享绘制路径：

- Forms & validation：标签、帮助信息、空值校验、提交中、失败、保留输入和重试。
- Search & filtering：受控查询、有结果、无结果、清空过滤。搜索框与标准 Button 按真实内容测量，通过共享布局居中排列，以 `space-regular` 分隔；宽度不足时按钮换到下一行，结果区从整组控件底部加 `space-section` 开始，绘制与命中共用同一组矩形。
- Master–detail：80 条固定列表数据，选择更新详情；列表和详情使用独立 ScrollArea；详情是连贯段落。
- Async & error recovery：受控 pending → failure → retry → success，不发网络请求。
- Destructive actions：确认、取消、执行中、失败、重新确认恢复；仅修改 Story 状态，不删除任何文件。
- Settings：真实文字输入、通知偏好、校验、保存中、失败和重试成功；不修改系统偏好。

| P0 能力 | 本轮边界 |
|---|---|
| Field | 标签 / 帮助 / 错误描述模型，输入槽复用 TextInput；不是另一套编辑器 |
| Surface / Panel | 共享语义表面绘制，Panel 为描边变体 |
| Toolbar | 生产布局辅助与真实 Button 组合，窄容器按可用宽度分配 |
| Select | 单选模型复用 Menu 的选项导航；不冒充可编辑 ComboBox |
| NumberField | 仅有有界数值模型和步进器，支持钳制、无效值拒绝；**可编辑数字文本、locale 数字解析尚未实现** |
| Skeleton | 静态语义加载占位，没有未经控制的 shimmer |

Image、Avatar、Tag、Keycap、DropZone、日期 / 颜色选择器继续作为后续候选。未实现的注册项维持 planned，不将导入包的规范存在视为已实现。

### 组件补齐阶段（2026-10-16）

按用户最新调整，暂缓把呈现时机排查作为组件开发的阻塞项，继续补齐注册表内的 45 个规范组件；上述后续候选不因此自动进入本轮范围。掉帧、CPU 和 120Hz / 8.33ms 的未通过结论及测量原件仍保留在 [性能 handoff](handoffs/gallery-language-performance.md)，没有将暂缓解释为问题已解决。

参考依据为现有 JSON 设计源、布局和交互契约。导入包的 `planned-not-runtime` 描述属于目标规范，不能作为运行实现或验证通过的证据。新增组件先实现可供消费者复用的生产模型，再接入统一 Story；保持 `experimental`，明确受限能力。复杂的拖拽、数据、文档组件如未满足真实契约，应继续保留具体缺口，不能只用静态示意图替换 `planned`。

新增模型集中在 `tekes-ui-controls::extended`；Chart 使用 `tekes-ui-controls::BarChart`。组件注册记录的 `api` 指向实际 Rust 入口，名称、分类和路由仍由同一 JSON 注册表维护。Gallery 的新增适配器在 `gallery/workbench/extended.rs`，复用工作台容器、操作控件、确定性重置和事件日志，文字排版继续由现有 CoreText Rust 桥完成。

| 新增基础实现 | 已实现范围与仍保留的边界 |
|---|---|
| Stack / Grid / Separator | 轴向分配、固定列网格与分隔线；Grid 返回自然内容范围，由宿主 viewport 裁切和滚动 |
| SplitView / Disclosure | 4pt 拖动死区、取消回滚、键盘调整、最小窗格不足时切换单窗格；内容展开/折叠 |
| IconButton / Link | 复用 Button 交互；IconButton 要求非空可访问名称，Link 使用 Enter 和下划线；不自动打开真实网络地址 |
| ComboBox / Slider / RadioGroup / Switch / SegmentedControl | 编辑筛选和选择、有限区间步进/拖动、单选和布尔值；IME 合成优先，禁用不触发修改 |
| Breadcrumb / CommandPalette | 祖先导航回调和受控命令筛选；示例命令只修改本地夹具 |
| List / Tree / DataTable | 单选、树展开及可见节点导航、确定性表格排序和稳定行身份；没有声称完成大数据虚拟化 |
| Chart | 单系列柱状图、正负值、无效数值过滤、选择和文字缩放；没有折线、散点、堆叠或多系列 |
| Tooltip / Popover | hover/focus 展示、Escape/outside 关闭及适用的焦点恢复；Popover 为非模态 |
| Toast / Banner / StatusBadge / Progress / EmptyState | 可控时钟、显隐/关闭、语义状态、真实进度值和空态操作；不依赖真实服务 |
| Markdown / CodeBlock / DiffView | Markdown 标题/段落/列表/围栏代码子集、只读等宽代码、行级 LCS 差异；大输入差异退回增删块，没有完整 CommonMark、语法高亮或编辑器承诺 |

NumberField 可编辑数字文本、locale 解析、完整辅助技术映射、物理输入法与长列表规模验收仍是开放项。新组件的基础实现不改变此前这些缺口和性能门槛的结论。

## 环境、搜索和直接定位

原生 Appearance 菜单沿用 Follow System / Light / Dark 和 Increase Contrast；四种有效主题名称保持不变。Preview 菜单提供：

- Fill / 420 pt / 720 pt 预览宽度。
- Comfortable / Compact：改变共享操作控件的密度，不虚构另一套 token。
- 100% / 125% / 150% 文字缩放：改变真实字体排版、行高和输入几何；侧栏不随预览缩放。
- Reduced motion：接入既有动效 / ScrollArea 的减运动路径。
- Consumer reference / example-blue / example-clay：作用于真实控件的语义颜色，包括浮层内的控件。按既有 policy，中性表面、文字、焦点和选区保持隔离，不把全 UI 染成品牌色。

品牌投影由 `scripts/generate-gallery-brands.py` 调用既有 `brand.compile_config`，读取当前 `design/source` / `design/brand` JSON，生成 `design/gallery/brands.json`，再生成无运行时 JSON 依赖的 Rust `Palette`。正常构建校验输入 hash，拒绝陈旧投影。不得手工修改生成的色值。

```sh
target/design-tools-venv/bin/python scripts/generate-gallery-brands.py
target/design-tools-venv/bin/python scripts/generate-gallery-brands.py --check
```

侧栏搜索使用真实 TextInput，支持组件名、规范 ID、路由和关键词。直接打开使用现有环境入口：

```sh
TEKESUI_GALLERY_PAGE=component-button/playground cargo run --locked -p tekes-ui-gallery
TEKESUI_GALLERY_PAGE=component-dialog/behaviors cargo run --locked -p tekes-ui-gallery
TEKESUI_GALLERY_PAGE=example-settings cargo run --locked -p tekes-ui-gallery
```

Story 后缀为 `docs`、`playground`、`states`、`behaviors`、`edge-cases`、`variants`。`Entry::supports_story` 统一约束显示标签、直接路由和 Docs 链接，不支持的栏目不再生成可打开的假路由。Reset 不改变全局主题、品牌或预览选项。默认英文；原有 Language 菜单和文字夹具切换保留，新工作台的部分新增文档文案目前仍为英文回退，未声称完成全量中文翻译。

## 验证和证据

```sh
CARGO_TARGET_DIR=target/gallery-integration cargo build --release --locked --offline -p tekes-ui-gallery
CARGO_TARGET_DIR=target/gallery-integration cargo test --release --locked --offline -p tekes-ui-gallery -p tekes-ui-controls -p tekes-ui-tokens -p tekes-ui-platform-macos
CARGO_TARGET_DIR=target/gallery-integration cargo clippy --release --locked --offline -p tekes-ui-gallery -p tekes-ui-controls -p tekes-ui-tokens --all-targets -- -D warnings
python3 scripts/check-pure-rust.py
target/design-tools-venv/bin/python scripts/check-design-system.py
TEKES_SELFTEST=workbench TEKES_SELFTEST_OUT=<absolute-directory> cargo run --release --locked -p tekes-ui-gallery
```

按用户要求，非 120Hz 专项的原生交互、截图、目测和普通 Gallery 启动固定使用 `TEKESUI_WINDOW_PLACEMENT=external`，回执保存实际 `placement` 日志。当前外接屏 U2790B 为 30Hz；这里的功能测试不能作为 120Hz / 8.33ms 验收。无头快照与单元测试不依赖屏幕。120Hz / 8.33ms 性能专项固定使用内建屏（`TEKESUI_WINDOW_PLACEMENT=builtin`），即使外接屏已连接也不改到外接屏；普通布局、交互和视觉复测继续使用外接屏。每轮性能回执须确认实际目标屏、回调周期与呈现间隔，不能只凭设备支持 120Hz 判定达到 120Hz。

新增组件的原生回归入口为 `TEKES_SELFTEST=workbench-extended`。ComboBox 的候选导航使用 `TextCommand` 经 `NSTextInputClient::doCommandBySelector` 路径验证，保留精确的合成预编辑、提交、指针选项和焦点查询断言。原有合成按键的空字符 payload 无法完整触发文本输入命令，因此不能把这条脚本当作物理键盘验收。

Motion 预览坐标回归使用 `TEKES_SELFTEST=motion-layout`：脚本从 Gallery 实际渲染的 canonical `foundation-motion/playground` 预览布局生成 hover/click 点，覆盖 Symbols、Linear rotation、Design System、Recipes 切换及目标按钮 hover/click，并截取窄窗口、150% 文字和滚动顶底画面；缩放后滚动命中另由 Gallery 单元回归验证。Symbols 的目标按钮有意从底部移到图标网格下方：图标绘制到 y+236，按钮从 y+238 开始，两个矩形不重叠；所有控制的绘制和命中矩形来自同一份 `state_actions`。运行示例：`TEKESUI_WINDOW_PLACEMENT=external TEKES_SELFTEST=motion-layout TEKES_SELFTEST_OUT=<absolute-directory> cargo run --release --locked -p tekes-ui-gallery`。该脚本是外接屏功能 smoke，不是 120Hz 验收。

Motion 的 headless snapshot 现经 `snapshot::render_png` 的专用路径构建真实 Gallery host renderer，覆盖 canonical 与旧入口，并用同一套生产 Button 布局、绘制及命中几何；导出矩阵可验证这些场景的绘制，不替代原生窗口中的指针交互检查。其余通用静态快照与历史校准夹具仍按各自快照路径生成。外接屏 30Hz smoke 不构成 120Hz 验收。

新的原生 smoke 脚本走实际 AppKit 鼠标 / 菜单 / 文字服务入口，捕获 Settings 初始、提交中、失败、成功；菜单键盘；Dialog 焦点与恢复；明暗 / 品牌；大字号窄预览；主从和 ScrollArea viewport。合成 preedit / commit 有实际文字桥断言；它不等于物理输入法验收。程序测试另覆盖真实输入事件、Space 释放、取消激活、modal 焦点范围、复制 / 撤销、disabled 编辑保护、确定性和 token 一致性。

`TEKES_SELFTEST=scroll-repair` 依次覆盖 internal TextEditor、Text 和 ScrollArea 的默认尺寸场景，再追加 canonical Story 图：ScrollArea 的 720×480、150% 顶部与末尾（末尾通过实际 surface 点击、全文查询和移至文档末尾验证），TextArea 的 720×480、150% 顶部及 PageDown 后画面，TextField 的 1000×720、150%，以及 Master–detail 的 720×480、150%。TextArea 滚动画面需在外接屏复核；内建屏执行结果不计为外接屏验收。这批截图不作为冻结 golden，也不将脚本定义本身视为已运行或已通过的视觉验收。

历史 golden 不覆盖。新检查证据保存于 [design/reviews/workbench](../design/reviews/workbench)，以其中机器回执和实际图像为准。构建、单测和截图均不能替代 VoiceOver、物理输入法、辅助设备、跨平台及长时间 120 Hz / 8.33 ms 验收。原有孤立掉帧问题仍保留原测试结论，本轮未宣称解决。

本轮实际结果（2026-10-16）：Gallery 170 项通过、1 项原有忽略；controls 19、macOS platform 19、tokens 18 项通过；release build、Clippy `-D warnings`、纯 Rust 边界、设计包完整性及品牌投影新鲜度检查通过。原生 smoke 捕获并目测检查 14 张图，2 条文字桥查询断言通过，观察到正常 CloseRequested；该独立进程未收集 shell exit code，因此其退出状态单列为未记录。最后仅调整了 Overview 说明文案，随后重新运行 Gallery 全部测试与构建、Clippy，最终启动在 Introduction 页。机器回执区分原生截图源码与最终启动源码，不将未执行的人工验收写成通过。

组件补齐阶段的最终验证（同日，以上段落为此前历史结果）：Gallery **185 通过、1 项原有忽略**，controls **47 通过**，macOS platform **19 通过**，tokens **18 通过**；合计 **269 通过、1 忽略**。release build、Clippy all-targets `-D warnings`、纯 Rust 边界及现行 Design System 校验通过。冻结后的 Rust / 注册表源码保持不变，DS 权威源和品牌投影 hash 未变，未删除此前冻结的任何源文件。

两套最终原生脚本均显式固定外接 U2790B，窗口日志确认 `builtin=false / scale=2 / max_fps=30`：扩展脚本退出 **0**、34 张截图、4 项文字查询通过；原有工作台脚本退出 **0**、20 张截图、2 项查询通过；两套均无截图捕获失败。目测复核了候选列表关闭、选项提交与焦点、行内文字居中、表格排序、图表与日志的边界；没有声称逐张人工审查全部截图，也没有用合成输入替代物理输入法验收。

最终无头渲染 **672 张 PNG**（84 个含历史夹具的页面 × 4 主题 × 2 缩放）。与此前冻结版本比较，55 个旧页面的内容区在四个 RGBA 通道上逐像素一致（440 张）；变化为原 28 个坑位和组件索引。全图侧栏随 `planned` 标记移除而变化，不能将这 672 张全图称为字节一致。历史 golden 与审批 pin 未修改；既有历史 JSON authority 审批校验未在此阶段改写为通过。

### Button 与 Motion 布局最终复核（2026-10-16）

Button 共享布局与文本适配的最终检查为 Gallery **207 项通过**、controls **58 项通过**（1 项既有可选图标语料测试忽略），strict Clippy `-D warnings` 通过；192 张矩阵导出及几何审计通过。`TEKES_SELFTEST=motion-layout` 原生运行捕获 24 张窗口截图并正常结束，覆盖 Motion 分类切换、目标按钮 hover/点击、滚动顶底，以及 720/800 宽度下中英文 125%/150% 文字场景。运行显示在外接 U2790B（30Hz）；这只验证功能流程，不代表 120Hz / 8.33ms 或 CPU 性能验收。检查汇总与机器回执见 [Button 封装最终检查](../design/reviews/button-encapsulation-20261016/checks-final.json)。

原生验证曾暴露候选选择的文字命令路径和点击后焦点恢复缺陷，均加回归后修复；空字符的合成按键不会完整模拟原生编辑命令，故该部分改走 selector 入口。首轮 `text=None` 查询失败没有在后续查询运行复现，原因未独立确认，保留失败日志而不归因于 AppKit。最终机器回执、失败诊断、源码与二进制 hash、选定截图和全页像素比较见 [component-completion-20261016](../design/reviews/component-completion-20261016/summary.json)。物理键盘、物理输入法、VoiceOver、大数据规模与 120Hz / CPU 专项仍按各自开放项处理。

## macOS App 图标和打包

用户提供的 `Gallery-macOS-Icon-Pack.zip` 中的原始 ICNS 位于 `crates/tekes-ui-gallery/resources/macos/Gallery.icns`；预览图、原始源图和来源 hash 一并保留。图像没有重新绘制。旧系统通过 `CFBundleIconFile=Gallery.icns` 引用它。macOS 26 使用同目录的 `Gallery.icon`，由 Apple `actool` 编译为 `Assets.car`，通过 `CFBundleIconName=Gallery` 引用，避免传统 ICNS 被系统再次缩进底板。图案仍是原 PNG，Icon Composer 的图层缩放为 1.40，将原图自带的圆角底板边缘与外侧阴影裁到系统遮罩之外（1.18 仍会留下双层外缘）；不重绘图案、不改原始像素。

```sh
python3 scripts/package-gallery.py
# 或只打包已验证的二进制：
python3 scripts/package-gallery.py --binary target/review-cache/debug/tekes-ui-gallery
```

脚本需要 Xcode 的 `actool`，但不编译 Swift 或 Xcode App 项目。脚本在临时 bundle 中装入 Rust 二进制、Info.plist、编译后的图标目录和旧版 ICNS，完成本地 ad-hoc 签名及验证后替换 `target/TekesUI Gallery.app`。它不启动、提交或发布。`cargo run` 直接启动可执行文件，不等于运行带图标的 `.app`；检查 Dock/Finder 图标请使用打包产物。Python 仅用于离线打包，不进入 App 运行时。

## 边界层级

`design/source/layout.json` 的 `boundary_styles` 明确区分必要控件边界与装饰框：前者使用 `border` / `stroke-rule` 并保留既有最低 3:1 要求，焦点使用 `border-focused`；后者使用已有 `separator`，不再把必要边界色套到所有容器。Motion 的非交互预览框采用按 backing scale 计算的一物理像素 hairline；Increase Contrast 回到 `stroke-rule`。生产 Panel 的默认逻辑线宽保持不变，边界改用装饰语义色。按钮、输入框、错误和焦点的必要识别边界没有全局调淡。旧 `stroke-hairline=0.5pt` 仍是 @2x 校准样本，不能视作所有倍率下的一物理像素。

迁移验收必须检查旧 renderer 的实际入口，不以映射文档代替可达性。Foundations 展示设计词汇和规则；Lab 展示像素对齐与合成诊断。两者复用 primitives.rs 的 stroke_samples / alpha_samples / state_samples，不复制维护实现。原始完整 build 保持 golden 不变，但不把空参照图和热图放进新工作台。

## 共享布局与 TextSurface 接入

Gallery 的文本示例复用生产 `TextSurface`：它统一处理内容布局、绘制、选择、caret、指针命中和文本内部滚动。Gallery 负责按页面组合该控件、提供 Design System 字体角色与 token 参数，接入 macOS 的文本输入桥，并把实际容器与祖先裁切传给组件。TextArea 与内部 TextEditor 示例沿用同一实现；内部 TextEditor 仍是 Lab 示例，不据此宣称它是独立公开组件。

`ScrollArea` 的 viewport / ancestor clip 使用共享 `ScrollGeometry` 快照；Gallery 自测坐标从真实页面构建后保留的 TextSurface 可见 frame 读取。长内容的 lazy provider、逐帧变更通知合并、120Hz / CPU 验收、物理输入法和 VoiceOver 仍是未完成或未验证项。实现边界与后续缺口记录在[布局与滚动接入回执](../design/reviews/layout-text-scroll-20261016/summary.json)；该记录不代表原生验收已通过。

## 整体验证跟进（2026-10-16）

本次按用户要求冻结当前 Design System，不调整边框 token、线宽或对比度规则。整体边框偏重作为最终视觉 review 的待决项集中处理；规范提案与控件真实边界、焦点、disabled 等状态一并评估，不能在个别页面硬编码变淡。

导航搜索复用生产 TextInput 与现有 `border` / `border-focused`，现在补齐窗口激活/失焦传递；占位文字和实际输入复用同一内容槽。过滤后的导航在窗口缩放时继续使用过滤后的 extent，避免出现空白滚动范围。Story 标签、直接路由及 Docs 链接由统一能力判断生成，长路由说明按真实文字高度排版。

[本轮机器回执](../design/reviews/workbench/followup-20261016/evidence.json) 记录 Gallery 176 项、controls 21 项、platform 19 项、tokens 18 项通过，Gallery 另有 1 项原有忽略。release build、Clippy `-D warnings`、DS/品牌投影和纯 Rust 检查通过。原生 AppKit smoke 正常退出（exit 0），20 张截图、2 条文字桥断言；回执逐项列出已目视检查的代表性截图，截图存在本身不等于人工验收。原有 14 张历史证据保留，不覆盖。

仍未完成的验收包括 VoiceOver、物理中日文输入法、全量中文翻译、持续 120 Hz / 8.33 ms 和 CPU 目标；本轮不把旧性能缺口写为通过。planned 组件继续保留缺口，不新增组件凑数。

性能继续工作见 [当前工作台性能复测](handoffs/gallery-language-performance.md#当前工作台性能复测2026-10-16)：已交付导航缓存与可见区绘制优化，外观像素对照通过；默认 CAD、可选 Metal、较浅队列和 GPU 计数器的失败结果均保留。持续 120Hz 零掉帧仍未通过，不由工作台功能验收替代。Design System 视觉优化继续留到最终 review。

## Component runtime ownership

The Gallery is a consumer of production controls, not a second implementation. Existing public model APIs remain unchanged; Gallery route IDs and grouping are owned by the single navigation registry. Presentation geometry, clipping, hover/press capture and keyboard arming belong to the production view/runtime. The Gallery supplies deterministic fixture data, available viewport bounds, Design System palette/text roles, platform event translation and event logs. This separation is the contract for future component work: preserve model APIs, let the producer own resolved layout and interaction state, and keep Gallery-only diagnostics out of production APIs.

The 45 component entries in `design/gallery/workbench.json` map to these production owners:

- `tekes-ui-controls::layout_view`: Stack, Grid, Separator, SplitView, Disclosure, Icon, IconButton, Link, ComboBox, SegmentedControl, Breadcrumb, Select and CommandPalette.
- `tekes-ui-controls::scroll_view`: ScrollArea and its viewport / content geometry.
- `tekes-ui-controls::widgets`: Button and Checkbox.
- `tekes-ui-controls::text_surface` and `feedback_view`: Text; TextField, TextArea, SearchField, Field and NumberField; Surface / Panel, Toolbar, Tooltip, Popover, Toast, Banner, StatusBadge, Progress, EmptyState and Skeleton.
- `tekes-ui-controls::selection`: Slider, RadioGroup and Switch.
- `tekes-ui-controls::composite`: Tabs, Menu and Dialog.
- Gallery `collections_adapter` delegates List, Tree, DataTable, Chart, Markdown, CodeBlock and DiffView stories to their production `collections_view` runtimes.
TextEditor and SelectionSurface are internal primitives used by the lab; they are not counted among the public 45 entries. This mapping does not create a second API or renderer in Gallery.
This mapping records renderer ownership, not maturity. Existing `experimental` status is retained where assigned; native performance, sustained 120 Hz / 8.33 ms, VoiceOver and physical input-method acceptance remain open until independently measured. The component-completion audit and old-to-new Gallery mapping are tracked in [the remaining components review](../design/reviews/remaining-components-20261017/summary.json).
