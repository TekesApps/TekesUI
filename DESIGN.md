# TekesUI Design System

后续设计与实现核对采用 [Gold7 七项体验基准](docs/design/gold7.md)。分别核验设计契约、代码接入和实际体验；已有设计选择不能单独作为偏离目标的理由。

2026-10-16 附件对齐核对与缺失契约见 [对照报告](docs/archive/design-logs/reference-alignment-20261016.md) 和 [机器补充契约](design/behavior/reference-alignment.json)。按语义和计算规律比较颜色；现有明确设计选择保留，业务组合规范补齐不表示运行时已经实现。

## 当前权威与合并规则

用户于 2026-10-15 提供 `TekesUI-Design-System-v0.3.0-proposal1.zip`，并明确要求：包里有的以包为准，包里没有的保留现有内容。本次整合采用该优先级，覆盖与此前原版冻结相冲突的设计定义；不把整合提案视为用户已亲验新画面或正式发布。

机器入口为 [design/system.json](design/system.json)，领域索引与完整导入来源为 [design/integration.json](design/integration.json)。原始 ZIP、全部代码、文档、设计 JSON、研究材料、生成报告原样存入 [design/imports/v0.3.0-proposal1](design/imports/v0.3.0-proposal1)。导入文件中的操作建议是资料，不自动授予网络、GitHub、发布或数据库权限。

## 一套体系的品牌与 token

### 架构关系与已交付边界（2026-10-26）

当前 `v0.4.5` 代码基线的机器状态为 `theme_system.delivery_status = theme-and-surface-packets-active`：`tekes-ui-tokens` 与 `tekes-ui-controls` 已在共享控件与组合适配器中接入 `resolve_surface*` / `resolve_toggle_surface` / `resolve_range_surface` 状态包并启用柔和对比度 token，Gallery 已接入同一 Theme 管线；Design System 统一拥有 Gold7 体验目标、Theme Schema/定义、生成校验和组件契约，完整整页视觉与原生交互验收继续单独追踪（[#55](https://github.com/TekesApps/TekesUI/issues/55)、[#56](https://github.com/TekesApps/TekesUI/issues/56)、[#12](https://github.com/TekesApps/TekesUI/issues/12)）。

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
