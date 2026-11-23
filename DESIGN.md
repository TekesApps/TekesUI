# TekesUI Design System

后续设计与实现核对采用 [Gold7 七项体验基准](docs/design/gold7.md)。分别核验设计契约、代码接入和实际体验；已有设计选择不能单独作为偏离目标的理由。

2026-10-16 附件对齐核对与缺失契约见 [对照报告](docs/archive/design-logs/reference-alignment-20261016.md) 和 [机器补充契约](design/behavior/reference-alignment.json)。按语义和计算规律比较颜色；现有明确设计选择保留，业务组合规范补齐不表示运行时已经实现。

## 当前权威与合并规则

用户于 2026-10-15 提供 `TekesUI-Design-System-v0.3.0-proposal1.zip`，并明确要求：包里有的以包为准，包里没有的保留现有内容。本次整合采用该优先级，覆盖与此前原版冻结相冲突的设计定义；不把整合提案视为用户已亲验新画面或正式发布。

机器入口为 [design/system.json](design/system.json)，领域索引与完整导入来源为 [design/integration.json](design/integration.json)。原始 ZIP、全部代码、文档、设计 JSON、研究材料、生成报告原样存入 [design/imports/v0.3.0-proposal1](design/imports/v0.3.0-proposal1)。导入文件中的操作建议是资料，不自动授予网络、GitHub、发布或数据库权限。

## 一套体系的品牌与 token

### 架构关系与已交付边界（2026-10-26）

当前 `v0.4.10` 代码基线的机器状态为 `theme_system.delivery_status = theme-and-surface-packets-active`：`tekes-ui-tokens` 与 `tekes-ui-controls` 已在共享控件与组合适配器中接入 `resolve_surface*` / `resolve_toggle_surface` / `resolve_range_surface` 状态包并启用柔和对比度 token，Gallery 已接入同一 Theme 管线；Design System 统一拥有 Gold7 体验目标、Theme Schema/定义、生成校验和组件契约，完整整页视觉与原生交互验收继续单独追踪（[#55](https://github.com/TekesApps/TekesUI/issues/55)、[#56](https://github.com/TekesApps/TekesUI/issues/56)、[#12](https://github.com/TekesApps/TekesUI/issues/12)）。

### macOS 应用/窗口生命周期与层级 ⌘W 对齐（`v0.4.10` / #406）

- **原生驻留生命周期与 Opt-In 关窗退出**：`tekes_ui_platform_macos::AppConfig::default().quit_after_last_window_closed` 调整为 `false`（新增 `.name(...)` / `.quit_after_last_window_closed(...)` 构建器），主窗口关闭时进入 dormant 隐藏状态、停止 DisplayLink 与无效重绘、投递 `FocusChanged(false)`、撤回 AX publication 并在 `WindowWake` 中排队唤醒信号，应用继续驻留 Dock 与菜单栏；仅在 `⌘Q` / `NSApp.terminate:` 时执行 `WindowHandler::allows_app_quit()` 校验并完整 teardown，保留显式 `.quit_after_last_window_closed(true)` 给一次性工具或测试入口。
- **Dock / `⌘N` 原生重开与层级 `⌘W`**：`AppDelegate` 实现 `applicationShouldHandleReopen:hasVisibleWindows:` 并新增标准 `File → New Window (⌘N)` 菜单项以恢复隐藏主窗口、Key Window 焦点、渲染与 AX publication 并保留已挂载的 `LocalContainer` 状态；`WindowHandler::{close_menu_title, handle_close_shortcut}` 与 `File` 菜单动态标题（`Close Tab` / `Close Window`）实现原生 `⌘W` 层级路由——辅助/独占窗口优先关闭（保留 Settings 未保存草稿拦截），AppHost 主窗口按 `⌘W` 优先关闭当前活跃可关闭 `Others` 容器 Tab 而红色关闭按钮始终仅关闭主窗口。
- **`AppWindow` 身份配置与 Selftest 扩展**：`AppWindow` 新增 `.app_name(...)`、`.window_title(...)` 与 `.quit_after_last_window_closed(...)` 构建器替换硬编码 `"TekesUI AppHost Prototype"`，并扩展 `tekes_ui_platform_macos::selftest::Action::{Quit, Reopen, RedrawMark, ExpectLifecycle}` 与原生测试（`lifecycle_native`、`macos_lifecycle_native`）。

### Gold7 Token 与交互契约审计收敛（`v0.4.9` / #388–#395）

- **表面对比与告警/浮层语义（#388, #389, #390）**：上调暗色高层交互填充（`button-hover` / `button-pressed` / `row-selection`）与 `dark-ic` `secondary-text` / `tertiary-text` 梯度以消除悬停塌缩；修正 `notice-marker-toast-warning` 为 `status-running`（区分 Warning 琥珀色与 Error 红色），并为 `ToastView` 与 `TooltipView`（`"hint"` 表面）统一接入 `SurfaceKind::Floating`（常规模式投影、`-ic` 模式 1pt 边界）与 `OverlayPresence` 进退场动画。
- **焦点契约、控件 Surface 与分段圆角（#391, #392, #393）**：统一 `TextField` 焦点边界（1pt 常规 / 2pt `-ic` + 独立 2pt `border-focused` 聚焦环，#391）跨 `interaction-contracts.json` / `gold7.md` / `text_surface.rs`；将 `DialogView` / `DisclosureView` 迁入 `Surface` 契约并统一 `mix_rgba_linear` 线性 RGB 混色（#392）；`SegmentedControlView` 接入专用 `segmented-*` token 与 `CornerRadius::concentric`（#393）。
- **动效连续性与 Gold7 治理同步（#394, #395）**：消除 `motion.json` 双时长表（`hover` 120ms / `press` 80ms / `switch_thumb` 180ms）并为复选/单选/分段/标签页接入 `ProgressTransition`；同步 `docs/design/gold7.md`、`gold7-alignment.json`、`gold7.json` 与 `reference-alignment.json` 至当前 `MediaViewer`/macOS AX 基线，保留未启用消费者配方与实机/VoiceOver/IME/120Hz 独立验收边界。

### 集成胶囊分段工具栏组（`v0.4.8`）

- **控件层集成胶囊组**：`tekes-ui-controls::composition::ToolbarGroup` 新增 `ToolbarGroupChrome::{Plain, Capsule { divided: bool }}`（默认保持 `Plain` 完全兼容）、`.with_capsule(bool)` / `.with_divided(bool)` / `.with_label(...)` / `.with_help(...)`；启用 `Capsule` 后在所有可见分段外围绘制共享 `surface-2`（`"panel"/"surface-frame"/"rest"`）圆角胶囊与 `decorative_border` 外框线，组内零间距紧贴排布并按 `1/backing_scale` 像素网格对齐分段边界以消除亚像素舍入截断；`divided == true` 时在相邻静息分段间绘制 1px 分割线并在相邻段处于悬停/按下/选中/键盘聚焦/菜单打开时隐藏。
- **语义前景与原地子组同步**：`ButtonTone::{Default, Secondary, Warning, Error}` 与 `ToolbarHost::update_group_children` 允许在不重建无关节点或打断未受影响弹出层的情况下，原地增删/重排胶囊分段并保持每个分段真实的 `ComponentId`、挂载身份、聚焦环、`MenuContent` 分段锚点及 macOS AX 语义树。
- **AppHost 声明与溢出协同**：`tekes-ui-app-host` 新增 `LocalToolbarGroup`、`LocalToolbarItem::Group`、`LocalToolbarGroupOverflow::{Together, Split}` 与 `LocalToolbarTone`，校验顶层/组/成员 ID 非零且全局唯一并拒绝嵌套分组；支持仅图标触发器与无 `⌄` 紧凑触发器（`dropdown_indicator(false)`），并在窄栏时支持整组协同收折至 `"More actions"` 溢出子菜单或按前缀拆分，点击直达与溢出触发均投递一致的 `LocalToolbarAction { id: member_id, source, payload }`。

### 本地容器动态 Window Toolbar 与原生下拉菜单（`v0.4.7`）

- **每帧/按需工具栏同步**：`ContainerContent::{toolbar_items, on_toolbar_action}` 与 `LocalContainer::{toolbar_items, dynamic_toolbar, on_toolbar_action}` 支持在 `prepare_for_render` 之后向 `MountedToolbar` 原地增量同步 `LocalToolbarItem`（按钮与原生菜单），同时保持现有 `LocalContainer::toolbar` + `on_command` 完全兼容；未变更组保持 `mount_identity()` 与未受影响菜单弹出层不被重建或打断。
- **单一事实来源原生下拉菜单**：`LocalToolbarMenu` 直接通过 `MountedToolbar` 挂载 `tekes-ui-controls::composition::MenuContent` / `MenuEntry`（支持 `checked` / `disabled` / `Separator` / `Submenu`、`dropdown_indicator(false)` 锁定身份可读触发器、Light/Dark 主题、overlay 模态与失活收敛），并将受校验的 `LocalToolbarAction`（含 `MenuSelection` 路径）安全回传给所属 `LocalContainer`。
- **`Focus ↔ Others` 与窄栏溢出**：同一容器在 `Focus`（`WINDOW_BUSINESS`）与 `Others`（`WINDOW_OTHERS_BUSINESS`）之间互换时保留主体 `ContentHost` 挂载身份与输入态，协调 `Others` Tab 栏（`presentation_switcher_frame`）安全空间，并在窄宽时将溢出项收敛到 `"More actions"`（`Source::PanelOverflow`）子菜单而不遮挡 Tab 栏。

### 组合控件封装、列表行收敛与边界加固（`v0.4.6` / #370, #373, #380）

- **共享图标-文本组合行（#370）**：`tekes-ui-controls::composition` 新增 `IconTextContent`（使用 `Spacing::Snug` = `6.0pt` 间距、弹性换行文本、去重装饰语义图标抑制与 `host_reveal_passthrough` 透传），迁移 `examples/composition-consumer` 的 `tree_content`、`selectable_list_content`、`datatable_content`，移除 `selectable_list_content.rs` 中的重复装饰器 `ConsumerDecoration`，仅在 `datatable_content.rs` 保留 1 个将 `Component` 钩子委托给 `IconTextContent` 的定制包装见证。
- **列表内部节点封装与分节容器（#373）**：`ListContent::{name, rows}` 收紧为私有，保留 `items()` / `items_mut()` / `name()` / `gap()` 公开访问契约；`ListContent::section` 收口使用 crate 内部 `ListSectionContent`，在固定 2 槽与揭示透传不变式下保护内部行容器不被外部绕过。
- **公开控件字段全面封装与 AppHost 去魔数（#380）**：`Button`、`StackContent`、`GridContent`、`ScrollContent`、`MenuContent`、`SwitchContent`、`SliderContent`、`DialogContent`、`PopoverContent`、`ProgressContent`、`LinkContent`、`TabsContent`、`SeparatorContent`、`TextContent`、`TabStripContent`、`NumberFieldContent` 与 `ContentTheme` 的可变 `pub` 字段统一收紧并通过显式构造函数、不可变读取器与验证 Setter 暴露（仅 `legacy_selectable_list::SelectableListContent` 保留历史 `pub` 字段）；`crates/tekes-ui-app-host/src/window_settings.rs` 全面清理生产与测试路径中的 `ComponentId(4)` 魔数硬编码并收敛为 `SettingsSidebar` 语义接口。完整原生窗口、实机 AX、物理 IME、VoiceOver 与 120Hz 性能验收仍作为独立证据类别保留在 `#56` 单独追踪。

### 共享表面路由、容器组合与演示收敛（`v0.4.5` / #379, #371）

- **控件表面包与微调能力（#379）**：`NumberField` 支持 `{min, max, step}`、`ScrollArea` 支持 `scroll_wheel` / `wheel_step`，`tekes-ui-controls` 补齐 `slider_on_surface` 与 `switch_on_surface*` 并让 `NumberFieldView`、`SliderView`、`SwitchView`、`SelectView` 通过表面解析函数消费声明式表面；`IconContent` / `SeparatorContent` 在显式禁用与中性语义路径上支持 `CurveSubtreeCapability::Local`。
- **共享组合容器、外观与中性文本收口（#379, #371）**：`tekes-ui-controls::composition` 新增 `StepperContent`（紧凑步进数值适配器）、`SurfaceContent`（统一 `canvas`/`panel`/`framed` 表面容器）、`ResponsiveGridContent`（响应式自适应列网格）、`MaxWidthContent`（保持子项本征高度的居最大限宽容器）、`ThemeScopeContent`（局部主题作用域）、`ModalViewportContent`（弹窗视口宿主），并扩展 `ScrollContent::horizontal_strip`、`Button::icon_label` / `with_icon_ink_leading` 与 `DialogContent::actions_row`。
- **工作台与示例消费端收敛（#379, #371）**：`SettingsSidebar` / `SettingsForm` 绑定 `"sidebar"` / `"panel"` 表面；`AppHost` 移除空白面板历史网格并改用 `CreationGrid` 语义表面与 `Button::icon_label`、`OthersListScroll` 改用 `ScrollArea::scroll_wheel`；Gallery 移除 `Stepper`、`SectionBar`、`SampleGrid`、`CHOICE`、`Card`、`EmptyStateSurface`、`DialogViewport`、`Viewport` 等局部重复脚手架（收敛至 `SegmentedControlContent` 与 `SurfaceContent`，补齐 #371 剩余边界）；`examples/*`（`composition-consumer`、`process-content-consumer`、`panel-module-demo`、`panel-content-consumer`、`media-viewer-content-consumer`、`city-3d-demo/tekes-ui-3d`）移除自绘 `Popup`、`DialogScreen`、`Stepper` 与重复 `CoreText` 文本宿主，统一接入 `ContentHost`、`SegmentedControlContent`、`CoreTextHost` 与 `resolve_surface`。

### 表单字段、受控文本契约与分组设置行（`v0.4.4` / #372, #374, #369）

- **共享 Field 呈现与 Surface 对比度路由（#372）**：`FieldPresentation::emphasized_form()` 成为 `tekes-ui-controls` 共享字段呈现契约，`FieldView::set_surface` 按配置表面从 `tekes-ui-tokens` 语义表面包解析标签与提示/错误文本前景色；Gallery `apphost_patterns.rs` 的 `FormMetadata` 与 `apphost_field_meta.rs` 的 `Fixture` 包装层已全部移除，改为直接配置 `TextFieldContent`（`set_field_presentation`、`set_max_width`、`with_presentation`）。
- **受控文本适配器外观与封装边界（#374）**：`TextFieldContent::{field, view, surface}` 收紧为 `pub(crate)` 并移除 `SearchFieldContent` 的 `Deref` 透传；外部调用统一经由 `TextFieldContent` / `SearchFieldContent` 的配置、布局、阅读流、选区及外部值同步外观方法（`from_surface`、`multiline`、`apply_external_value`、`check_external_value`、`replace_value`、`set_reading_document`、`follow_latest`、`set_error`、`set_help`、`set_required` 等）驱动状态，确保衍生文本与 Clear/语义投影同步。
- **共享分组设置行与 Settings 布局（#369）**：原 `tekes-ui-settings` 私有 `SettingsRow` 升级为 `tekes-ui-controls::composition::GroupedFormRowContent`（默认继承 `"panel"` 表面、基于 `probe_w` 的统一本征列宽测量分配与 `SwitchView::TRACK_WIDTH = 48pt`、`c-settings-group-ground` / `radius-panel` 与内缩分割线）；`SettingsForm` 接入 schema 分组标题（`StackContent::titled_section` / `SECTION_HEADER_GAP = 8pt`）、`16pt` `PANEL_SECTION_GAP`、全宽通栏 `ScrollContent::document`（移除 `390pt` 宽度上限及 `tekes-ui-app-host/src/window_settings.rs` 外层 `16pt` 重复留白与四边裁剪内缩），以及采用 `TextContent::with_error_foreground()` 的水平 `save_error` 提示条。

### 组合容器单一留白与格式塔层级（`v0.4.3` / #376, #377, #375）

- **`tekes-ui-controls` 容器与分节 API**：`ScrollContent::content_insets(Insets)` 支持四边独立留白与外贴右缘滚动条，`ScrollContent::document(child)` 提供标准 `16pt`（`Spacing::Loose`）文档留白与滚动条；`ListContent::scrollable_with_header(...)` 统一固定头部与滚动体的 `16pt` 对齐基准及头部下方 `8pt`（`Spacing::Regular`）裁剪安全距；`StackContent::{DOCUMENT_SECTION_GAP (24pt), PANEL_SECTION_GAP (16pt), SECTION_HEADER_GAP (8pt), SPECIMEN_LABEL_GAP (4pt), document_sections, panel_sections, titled_section, labeled_specimen}` 与 `KeyValueTableContent::section(...)` 强制保证外层组间距严格大于组内标题/标签与内容间距。
- **单一 Inset 所有权与消费端收敛**：`tekes-ui-app-host` 容器默认零内边距（仅对无内层滚动/边距包装的容器显式指定 `.content_padding(...)`）；`tekes-ui-settings` 固定头部侧栏、AppHost 示例及 `tekes-ui-gallery` 全量页面（Documents / Inspector / Foundations / Stories / Navigation 与 12 处状态矩阵标注展品）已移除双/三重留白与历史手算坐标夹具，直接消费 `ContentHost` 实际布局几何。

Theme/Palette/Surface 基础 API 已交付；controls 中仍有直接 proposal/Palette 消费，完整组件迁移在 Gallery Root 与私有候选中有另外的源码身份。声明 Surface 解剖/语义用途的绘制必须消费共享完整结果，不能另配状态色表；metric/layout/motion 的 typed API 可以保留，文本颜色须核对实际用途，不能自动豁免 Surface 契约。Root 的 #62/#104/#92 等原接收不自动成为上述 main 的通过证据。

完整职责图、具体差距、原 Issue 去向及已确认架构方向见已有 [架构与平台边界](docs/platform-boundary.md)。#55 统筹视觉体系、#56 统筹组件行为/组合，#136 接收已交付差量，#113 证明受管来源闭包，#12 验实际体验；各自的原 AC 互不替代。本轮只纠正文档与调度，不修改设计 JSON/schema、生成器或原 AC。

消费者提供显式 BrandConfig；基础 token、品牌策略及该配置共同派生四套主题的强调色、主操作色、状态变体和配对前景。中性表面、正文、焦点、选区、业务状态、语法与图表色按包内隔离策略处理，不能把所有颜色统一染成品牌色。默认展示采用包里显式登记的 neutral 参考夹具，不能把示例色当成框架固定品牌。

作者输入统一生成 `design/tokens.json`，正常 Cargo 构建检查输入新鲜度并从该投影生成 Rust token。生成物禁止手改；品牌 seed 改动须重新生成并校验，不能维护一份与品牌策略无关的人工颜色表。参见 [design/README.md](design/README.md) 的实际命令。

布局、组件、交互、字体、状态、质量和品牌规范以导入包为优先。现有 Primitives、Token Swatches、静态文字三页及上下往返滚动夹具作为已有实现保留，采用合并后的 token 和适用规范。没有精确对应的旧角色作为明确补集保留，不仅凭名字相似强行替换。

## 实现边界与验收

原生 Gallery 继续使用纯 Rust 自研组件层，不加入 Swift、GPUI 或其他现成 UI 框架。wgpu 和 objc2 等基础依赖允许；AppKit/CoreText 经 Rust 系统绑定调用。Python 仅用于离线设计开发工具，不是 Gallery 运行时依赖。TekesMetalTextKit / TekesClientKit 只读参考，compare 不进入构建或运行依赖。

包包含 40 个组件规范及 10 个组合模式，不代表这些控件都已在 Rust 中实现。静态对比度、schema 或生成检查通过也不能代替真实键鼠、IME、VoiceOver、跨平台及性能验收。

原 `a8ab930` 的用户确认、24 张 golden 和 JSON 来源迁移记录保留为历史证据；不覆盖旧图来掩盖本次有意的设计变化。新画面要与旧版并排审查，并把已实现、规范待实现及未验证分开记录。后续可见变更仍应说明来源和影响，不因提案文件存在就声称已经通过人工验收。

包的 `v0.3.0-proposal1` 是输入提案标识，不是本仓库 SemVer、tag 或正式发布。提交不自动包含推送、打 tag 或发布权限。
