# TekesUI 路线图

> 2026-10-15 最新设计权威：用户提供的 v0.3.0-proposal1 有定义时优先，缺项保留现有补集。参见 [DESIGN.md](../DESIGN.md) 和完整导入索引；本文此前固定尺寸、旧值和原版优先约定仅在不冲突时适用。规范存在不等于原生实现已验收。


> 历史优先级记录（2026-10-15）：`a8ab930` 原版 Gallery 的确认与 golden 保留原身份；当前提案优先级及可见变更边界以 [DESIGN.md](../DESIGN.md) 为准。以下阶段规划不自动授权重画、替换 token 或采用旧 SF/Swift 退路。

> 外部库版本均为 2026-10 调研时状态，开工前复核：wgpu 30、Parley 0.12、objc2 0.6、AccessKit 2026-09 版本、winit 0.31 beta。

**不设日期，按通过标准推进。** 每个阶段的通过标准都包含对应的 Gallery 页面；对比方法（像素差容差、排版数值、性能、输入法验收矩阵、VoiceOver 清单）统一见 [comparison-method.md](comparison-method.md)，本文不重复定义。分层与原则见 [charter.md](charter.md)。

## 2026-10-26 架构核对与计划去向

已交付代码基线为本地 main `e39bfaa1785dbedd9672081a8877e27047155885`。下面 F0–F7 的功能范围和通过标准继续保留；crate 名称、旧工具选型与阶段顺序不等于当前实现或新的代码授权。实际职责及已确认 A1–A3 见 [架构与平台边界](platform-boundary.md)，决定入口 #137。此表没有修改任何原 Issue AC，也不因整理关闭 Issue。

| 原计划 | 去向 | 理由与现有唯一入口 |
|---|---|---|
| F0 平台/Gallery、F1 text/atlas/render | 继续既有能力，调整架构描述 | main 已有具体实现，render 有 mac surface、text 有 CoreText；历史实测不改签。已有产出经 #136 接收，当前体验仍按 #12；#70 的 AppWindow/Settings 交付保持自身范围 |
| F2 自有 token 是否采用的早期退路 | 该未决问题被现行输入体系取代；不重开选型 | 用户提案优先、自有作者 JSON/BrandConfig/生成链已采用，见 DESIGN；#55/#1 做适用迁移，#57 图标，#113/#114 来源证明/投影。旧“退化系统色”不是现行授权 |
| F2 指定 taffy / 独立 layout 包 | 调整为待评估的实现设想 | 当前 controls 有自己的 ContainerLayout 和控件几何，未采用 taffy；本轮不引依赖。布局功能要求保留，是否提取协议/包随 #137 决定及 #131 真实需求 |
| F3 虚拟文档、F4 大规模集合 | #131 继续后置设计 | 现有 reading/稳定 key/基本集合不等于通用虚拟化；rope 是否必要先设计，原排版/锚点/选择/复制/性能要求不删减 |
| F4 终端网格 | #134 继续后置设计 | 许可、复用路线和一致性范围未冻结，不把显示网格当完整终端 |
| F5 单行/多行输入 | 继续原功能及缺陷跟踪 | 已有 Rust 编辑/IME 接线不是全部矩阵通过；#128 连续组字缺陷、#41 注入基线各沿原 AC，reading/组合原票不被编辑器重构取代 |
| F5 Markdown 与 doc/edit 包 | #132 继续后置设计；A1/A3 已确认 | 源文档/投影映射、独立事务与公共边界有价值，但不按旧图自动建包或取消功能 |
| F6 a11y / 指定 AccessKit | #5 分期推进；A2 已确认 | 先明确中立基础接口；核心组件/组合稳定后做按钮、输入框和一个组合控件的真实验证；AccessKit 选型和接入后置，完整原 AC 保留 |
| 系统拼写、查词、翻译、朗读 | #133 继续后置设计 | 与编辑器状态/平台服务契约有关，不为全部普通控件添加系统服务前提 |
| F7 Windows、更后 Linux | #135 继续后置设计 | 先查实际 text/platform/render/AX 可复用边界，不据 trait 名称声称已跨平台；保留平台顺序和原验收 |
| AppHost / ProcessContainer | 继续同仓可选集成定位，非所有消费者前提 | #48 完整交互/公开交付原 AC 保留；AppHost 的实验与 Root 候选不改称 main 正式能力 |

本轮暂停新实施，先完成文档与 Issue 对齐提交；审计结束后可沿既有授权收口 #136 的最小完整交付及 #113 首个完整 Button 根。A1–A3 已确认，但具体实施仍须明确范围和依赖；其他原 AC 实质变化须单独展示影响并确认，不追加为所有既有工作的前提。#121–#125 等待方法/共享接口冻结，#131–#135 本轮不启动。具体 owner、允许范围、依赖和退出条件见架构文档中的执行队列。

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
- **token 占位**：Design 交付之前，token 先用 Tekes 现有数值做临时占位，这是历史阶段安排；现行输入、生成与来源规则见 [design/README.md](../design/README.md)。

## F1：CoreText 加 atlas 渲染静态文字

- **范围**：tekes-ui-text 的三个 trait 与 CoreText 实现；tekes-ui-scene 的 display list 与 glyph atlas；渲染静态文字。
- **对照对象**：MetalTextView。
- **通过标准**：HarnessStubSentences 渲染与 MetalTextView 对齐（容差见对比方法文档）；空闲 0 CPU；120Hz 下滚动或动画不掉帧。Gallery 页面：静态文字页，固定夹具为 HarnessStubSentences，并排展示 MetalTextView 参照图与像素差图。

## F2：设计 token 和基础控件

- **范围**：tekes-ui-tokens；按钮、开关、文本标签、滚动条、分隔条等基础控件；界面 chrome 的 taffy 布局。
- **设计产出工具链**：早期计划为 `cargo xtask design import` / checked-in generated.rs；当前已采用作者 JSON、离线合并工具、Cargo OUT_DIR 生成与新鲜度守卫，约定见 [design/README.md](../design/README.md)。
- **依赖**：[icons-and-tokens-brief.md](design/icons-and-tokens-brief.md) 的结论决定 token 与图标的来源。
- **退路**：若用户决定不做自有 token，则 tekes-ui-tokens 退化为对系统语义色、系统字体和 SF Symbols 的薄封装（mac 上直接读取系统值），控件外观跟随系统；自有 token 部分推迟，不阻塞后续阶段。
- **对照对象**：现有 Swift 客户端对应控件及系统控件。
- **通过标准**：每个控件的 Gallery 页覆盖完整状态矩阵（hover、pressed、disabled、focus、light/dark、increase contrast）并有参照并排；拖动分隔条时 CPU 显著低于 AppKit 视图树方案（数值门槛见对比方法文档）。完成真实 Design System 交付的导入与来源校验；现已导入的 v0.3.0-proposal1 与原生体验验收分别记录。

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

- **范围**：框架解耦的组件/文档语义与系统无障碍映射；文本选区、朗读、导航。按 #5 先明确基础接口，稳定后进行三个控件的真实有限验证；AccessKit 与原生适配的比较、路线确定和完整验收后置。小范围验证不替代以下通过标准。
- **对照对象**：NSTextView 与系统控件在 VoiceOver 下的表现。
- **通过标准**：VoiceOver 清单（见对比方法文档）全部通过；Gallery 里每个控件页带一个无障碍树检视面板。

## F7：Windows 平台层

- **范围**：tekes-ui-platform-windows，Win32 加 TSF；tekes-ui-text 的 DirectWrite 实现；渲染走 wgpu 的 Windows 后端。Linux 不在本阶段，另行立项。
- **对照对象**：mac 上已通过的同一批 Gallery 夹具；Windows 系统的原生输入与文本行为。
- **通过标准**：F1 至 F5 的 Gallery 页面在 Windows 上渲染与行为达标（容差按对比方法文档中跨平台部分），输入法矩阵在 Windows 版本通过。
