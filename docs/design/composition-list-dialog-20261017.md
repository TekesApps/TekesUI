# #14 最小场景第二批：异构 List 与 Rich Dialog

2026-10-17（Asia/Shanghai）。基于 `bc3de79466834e99062a1b4ef0872323e2860001` 加当前全部未提交 WIP；源身份、测试和图像哈希见 `design/reviews/composition-list-dialog-20261017/receipt.json`。本轮未提交、推送、打 tag、创建/关闭 Issue 或发布；性能按用户要求排除。#14 继续作为审计规划入口，本文不宣称其全组件矩阵、全部迁移或统一验收已完成。

## 实施结果与复用边界

| 根因 | 共享修复位置 | 适用范围、例外和验证 |
| --- | --- | --- |
| 公共契约没有可选的编辑文本服务，组合消费者无法接入生产输入 | `TextSurfaceHost`、`ButtonTextRenderer::text_surface_host`、`composition::TextFieldContent` | 直接复用 `TextSurface` 的 TextInput、UTF-16 选区/组字、排版缓存、绘制、命中、滚动及 caret；`FieldView` 负责标签、帮助和错误。宿主提供 shaping/glyph backend 与 caret blink；缺服务显式返回 MissingTextServices，禁止静态文字冒充编辑器。旧 label-only renderer 默认无此能力，不被迫实现新方法。 |
| 焦点/平台文本查询缺少唯一 owner | `ContentHost::focused_text`、TextCommand/IME 事件路由 | 返回当前作用域中真实 TextSurface，原生宿主从同一对象投影 selection、marked、caret 和可见 caret 集合；不引入 Gallery 类型。Tab/Shift+Tab selector 与键盘同路；composing 时 Tab 让路，Escape 先保留组字处理权。基本编辑命令已有适配，不声称完整编辑器命令/多击/修饰键/剪贴板合同已全部覆盖。 |
| Switch/Menu 尚无组合适配；浮层与普通子内容不在同一绘制/事件层级 | `SwitchContent`、`MenuContent`、Content 的 overlay pass | 模型和 View 都复用生产 Switch、SwitchView、MenuView；不仿造外观、不把 Menu 内部 item 当任意 UI 槽。Menu 最后绘制，保留自身键盘、item 选择和滚动语义；popup 外 ignored wheel 可冒泡到祖先 Scroll。关闭期间的 Menu presentation 仍由原实现管理。 |
| 通用 List 只有固定标签/单选表示，不能容纳独立异构内容 | `ListContent` 复用现有 StackContent 和 measured placement | 开放同一种 Content children，支持 Text、消费者 Badge、生产输入/Switch/Menu、自定义嵌套行及可变高。此入口表示通用内容组织，不自动叠加 ListBox 的选择/漫游语义。原 List/ListView 的 labels 与选择 API 保留；本轮没有把它们强制改为此试验表示。 |
| Dialog 固定动作布局无法承载复杂表单；通用宿主缺模态 scope | `DialogContent`、`ContentHost::show_dialog/close_dialog`，`DialogView::paint_content_frame` | 三个固定 Header/Body/Footer 槽都使用 Content；复用现有 Dialog 开关模型和 DialogView 的共享表面绘制。Host 保证焦点、语义和事件限制在模态子树，所有背景兄弟之后只绘制一次 Dialog，关闭按稳定 ID 恢复 invoker。模态内 body 滚动直接使用 ScrollContent。固定槽禁止增删/重排；动态表单在 Body 子树操作。嵌套 modal 目前显式报 ModalAlreadyOpen。 |
| Scroll 内容紧贴 viewport，输入/动作焦点环被自身容器裁掉 | 共享 `ScrollContent::arrange` | 复用 `ControlFrame::focus_bounds(backing_scale)` 的现有边界与物理像素余量，在滚动文档几何中统一预留 gutter。继续使用真实 viewport clip，绘制与命中坐标一致。适用于所有接入此协议的 Scroll 内容，不只 Dialog。修复前后图和几何断言留存。 |

合理例外：消费者 ActionRow 复用现有 Button 的 press/release/cancel/key 状态，完全不调用 Button 绘制，保持 ListItem 语义；其键盘环使用现有 `paint::focus_ring` 和 JSON `radius-row`。目前没有独立的公开 Press 状态机，因此不为这个最小场景另起交互抽象。生产组件的字体、边界、颜色、圆角与焦点都继续取 JSON Design System；TextSurface.draw 现在可接完整 Palette，旧 Theme 调用仍可用。

本轮保持 experimental。Owned tree 重排、主题/重布局和 Dialog 关闭重开不重建编辑实例；卸载仍返回 owned 内容供消费者自行保留。没有加入自动序列化、虚拟化或销毁后恢复协议。新增公开适配和语义字段属于未发布的最小协议试验；本地现有消费者兼容检查通过不代表全组件 Breaking Changes 审计已完成。

## 独立消费者与真实接入

`examples/composition-consumer` 是独立 Cargo workspace，不依赖 Gallery。两个 fixture、验证函数及原生例子均由该消费者拥有：

```sh
cargo test --manifest-path examples/composition-consumer/Cargo.toml --locked
cargo run --manifest-path examples/composition-consumer/Cargo.toml --example native -- list
cargo run --manifest-path examples/composition-consumer/Cargo.toml --example native -- dialog
```

List 覆盖混合节点、不同测量高度、稳定 ID 重排、选区/中文 marked 文本保留、Menu 键盘选择、行内 Switch 与行操作隔离，以及无按下/取消/区域外释放不激活。Rich Dialog 使用真实 Grid、TextSurface/FieldView、SwitchView、帮助文本、实际 Field.error 验证和 Footer Button；空白保存保留 Dialog 并聚焦错误输入，有效值清错。API 回归覆盖模态 Tab、组字保护、Escape 恢复和关闭重开状态。

Gallery 只在测试中嵌入相同消费者 fixture，提供现有真实 TextEngine/CoreText、批准图标和 GPU；没有迁移 Gallery 生产路由。List/Dialog 共 128 帧：四主题 × 两宽度 × 两倍率 × 两字号 × 两场景 × 普通/交互状态；另回归前批复合 Button 的 32 帧。完整数量、日志与哈希以收据为准；只重点样图经过人工复核，数量不代表逐图人工验收。

独立原生可执行例子使用公共 platform-macos、CoreText、Renderer、ContentHost，通过真实 AppKit 窗口交付输入和呈现。实测鼠标点击行内 Switch 不触发行操作、⌘A 选区替换、普通文本提交、Menu 的方向键/Enter 选择；Dialog 空值错误、Tab 作用域、Escape 返回 invoker、有效保存后重开保留输入与 Switch 状态。原生事件日志和窗口截图与 API/GPU 证据分别保存。首次宿主没有 Edit 菜单，⌘A 未连上；补同一公共 TextCommand 转发后重测通过，首次日志保留。

## Gold7 分层核对

以下“通过”仅限明确列出的最小场景观察，不升级仓库全量 Gold7 状态。

| 项目 | 设计契约 | 代码接入 | 实际体验与剩余项 |
| --- | --- | --- | --- |
| G1 人文质感 | 通过：沿用 JSON 暖中性色、语义文字层级 | 通过：生产 Palette/TextRole，消费者不维护调色表 | 通过（重点样图与原生普通主题抽查）；全品牌/全部显示器未测 |
| G2 微对比表面 | 通过：复用现有 floating/dialog surface、scrim | 通过：同一个共享 Dialog plate，Menu 自身浮层 | 通过（样图/原生表单层级）；不代表整页 Gallery 验收 |
| G3 细边与圆角 | 通过：既有密度/焦点与角色圆角规则 | 通过：Scroll 统一 focus gutter、输入/错误/焦点独立 | 通过（窄宽150%字号/@2x样图与原生焦点环抽查），其他显示器未测 |
| G4 交互物理 | 通过：既有 Button/Switch/Menu motion 契约 | 失败（部分缺失）：已有 View 的动效可复用，组合 Dialog 尚未接入原 OverlayPresence 的进入/退出过渡 | 未测完整动效体验；原生宿主已接系统 Reduced Motion 与 frame requests，但不把静态图或点击成功当动效通过 |
| G5 输入与滚动保护 | 通过：组字优先、稳定状态、单一编辑 owner | 通过（本批 API）：真实 TextSurface、组字/选区重排保留，滚动及菜单消费回归 | 普通原生输入和选择替换通过；物理中文 IME、候选窗及 native replacement-range 矩阵未测。原生工具直接输入中文没有产生可核对的 Commit，不能据此宣称中文输入法通过 |
| G6 桌面交互 | 通过：作用域、capture/cancel、键盘路径 | 通过（本批场景）：子事件隔离、模态 Tab/Escape/恢复、Menu 路由 | 通过（上述原生路径抽查）；原生可访问性树仍只有窗口/菜单，控件 AX 桥缺失，VoiceOver 未通过；多窗口/屏幕与全部输入矩阵未测 |
| G7 独立组件与预览 | 通过：消费者只使用公开 API | 通过：独立 workspace、两场景、原生例子和真实 GPU | 通过（本批可运行预览）；三个最小场景仍是“已实现待完整验收”，不授权全库迁移 |

## 执行顺序与剩余阻塞

两个场景的公共 API、真实后端接入和部分原生路径已验证，从“缺少适配阻塞”进入“已实现待完整验收”。继续在 #14/#12 的原任务范围记录以下条件：组合 Dialog 的生产 presentation 过渡、物理 IME/候选窗、控件 AX/VoiceOver，以及最小场景的完整交互/主题/品牌核对。#5 的平台 AX 缺口仍保留。上述条件未满足前，不开展 Tabs/Tree/DataTable 等组件族迁移或窗口级 Toolbar 实现，不创建重复 Issue。

本轮未运行性能验收；两项 GPU/CPU 计时测试显式过滤，原有可选测试忽略项不算通过。没有把本地检查称为 GitHub CI 成功。
