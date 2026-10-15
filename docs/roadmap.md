# TekesUI 路线图

> 外部库版本均为 2026-10 调研时状态，开工前复核：wgpu 30、Parley 0.12、objc2 0.6、AccessKit 2026-09 版本、winit 0.31 beta。

**不设日期，按通过标准推进。** 每个阶段的通过标准都包含对应的 Gallery 页面；对比方法（像素差容差、排版数值、性能、输入法验收矩阵、VoiceOver 清单）统一见 [comparison-method.md](comparison-method.md)，本文不重复定义。分层与原则见 [charter.md](charter.md)。

## 阶段依赖

```
F0 --> F1 --> F2 --> F3 --> F4
                     |
                     +--> F5 --> F6

F7 依赖 F1 至 F5 的 trait 边界稳定
```

简述：F1 依赖 F0；F2 依赖 F1；F3 依赖 F1（文本）与 F2（滚动条等控件）；F4 依赖 F3；F5 依赖 F3；F6 依赖 F5，且贯穿式地在 F2 起就保留无障碍接口；F7 依赖 F1 至 F5 的 trait 边界稳定。

## F0：平台层、窗口、wgpu、Gallery 骨架

- **范围**：tekes-ui-platform-macos 的 NSApplication、窗口、菜单、事件泵；tekes-ui-render 的 wgpu 初始化与帧循环；tekes-ui-gallery 的骨架（导航、固定布局的页面容器、状态矩阵切换、并排参照图容器）。
- **对照对象**：无（基础设施）。
- **通过标准**：窗口可开关缩放、菜单可用；空闲时不重绘（0 CPU）；Gallery 里有一个占位页，能切换 light/dark/increase contrast，并展示参照图并排槽位。

## F1：CoreText 加 atlas 渲染静态文字

- **范围**：tekes-ui-text 的三个 trait 与 CoreText 实现；tekes-ui-scene 的 display list 与 glyph atlas；渲染静态文字。
- **对照对象**：MetalTextView。
- **通过标准**：HarnessStubSentences 渲染与 MetalTextView 对齐（容差见对比方法文档）；空闲 0 CPU；120Hz 下滚动或动画不掉帧。Gallery 页面：静态文字页，固定夹具为 HarnessStubSentences，并排展示 MetalTextView 参照图与像素差图。

## F2：设计 token 和基础控件

- **范围**：tekes-ui-tokens；按钮、开关、文本标签、滚动条、分隔条等基础控件；界面 chrome 的 taffy 布局。
- **依赖**：[icons-and-tokens-brief.md](design/icons-and-tokens-brief.md) 的结论决定 token 与图标的来源。
- **退路**：若用户决定不做自有 token，则 tekes-ui-tokens 退化为对系统语义色、系统字体和 SF Symbols 的薄封装（mac 上直接读取系统值），控件外观跟随系统；自有 token 部分推迟，不阻塞后续阶段。
- **对照对象**：现有 Swift 客户端对应控件及系统控件。
- **通过标准**：每个控件的 Gallery 页覆盖完整状态矩阵（hover、pressed、disabled、focus、light/dark、increase contrast）并有参照并排；拖动分隔条时 CPU 显著低于 AppKit 视图树方案（数值门槛见对比方法文档）。

## F3：只读虚拟文档、选择、复制、滚动

- **范围**：tekes-ui-doc 的 rope 与节点词汇；tekes-ui-layout 的虚拟布局（驱逐后锚点有效）；选择模型与粒度；复制；滚动（使用系统 momentum phase）。
- **对照对象**：MetalDocumentView。
- **通过标准**：同一份长文档夹具下，排版数值与 MetalDocumentView 一致；选择粒度、复制结果（纯文本与富文本）与参照一致；惯性滚动手感与系统一致；大文档下内存与帧时间达标。Gallery 页面：长文档页，含选择与复制演示和并排参照。

## F4：diff、列表、表格、终端网格

- **范围**：diff 视图（借用 diff 模型思路）；虚拟列表；表格；终端网格。
- **终端评估**：评估 alacritty_terminal 与 libghostty-vt，先核实许可证再决定是否采用；结论以文字记录在 docs 下，不满足许可或无法稳定使用时，终端自己写最小实现或推迟。
- **对照对象**：现有 diff 与列表实现、系统终端行为。
- **通过标准**：各组件有 Gallery 页和状态矩阵；虚拟化后大数据量滚动达标；终端网格通过选定的终端一致性用例（用例清单随评估结论补入对比方法文档）。

## F5：输入，从单行到 Markdown 编辑器

分三步，每步都有各自的 Gallery 页：

1. 单行输入：完整 NSTextInputClient、preedit 带分节区间与组字内光标；
2. 多行编辑器：tekes-ui-edit 的状态机、undo、icu_segmenter 断行；
3. Markdown 编辑器：Markdown 投影与编辑模型。

- **对照对象**：NSTextView / NSTextField 的行为，以及 MetalTextKit 中的 Markdown 编辑行为。
- **通过标准**：输入法验收矩阵全部通过（系统拼音、搜狗、微信输入法、听写、重新转换、候选窗跟随滚动；具体矩阵见对比方法文档）；CJK 下 caret 不漂移；undo 与系统一致。

## F6：无障碍

- **范围**：tekes-ui-a11y，基于 AccessKit，把控件与文档映射到系统无障碍树；文本选区、朗读、导航。
- **对照对象**：NSTextView 与系统控件在 VoiceOver 下的表现。
- **通过标准**：VoiceOver 清单（见对比方法文档）全部通过；Gallery 里每个控件页带一个无障碍树检视面板。

## F7：Windows 平台层

- **范围**：tekes-ui-platform-windows，Win32 加 TSF；tekes-ui-text 的 DirectWrite 实现；渲染走 wgpu 的 Windows 后端。Linux 不在本阶段，另行立项。
- **对照对象**：mac 上已通过的同一批 Gallery 夹具；Windows 系统的原生输入与文本行为。
- **通过标准**：F1 至 F5 的 Gallery 页面在 Windows 上渲染与行为达标（容差按对比方法文档中跨平台部分），输入法矩阵在 Windows 版本通过。
