# #8 必需状态与契约 token 覆盖

2026-10-17。继续使用 `gallery-design-baseline/TekesUI` 的 `bc3de79466834e99062a1b4ef0872323e2860001` 加全部未提交 WIP。GitHub 重新读取仍为14个开放 Issue、全部状态0个PR；#14 最新正文保持 `2026-10-17T08:15:16Z`。本批继续原 #8，不创建重复 Issue、不扩大组件迁移、不实施窗口级 Toolbar。

证据入口：`design/reviews/component-state-token-coverage-20261017/receipt.json`。当前状态见 [#13 本地总览](issue-remediation-current-20261017.md)；前批报告、源覆盖层、截图和收据保留原样。

## 根因与共享修复

根因属于设计语义与公共组件契约治理：有一个 `states` route，不能证明它覆盖设计要求的所有状态；而统一 Docs 原先只说明 token 依赖未审计，无法列出已有 JSON 中的确切引用。没有为填清单新增 token、状态枚举、运行时协议或 UI 抽象。

复用既有 package state-profiles、components、component-contracts、当前 tokens 投影、生成器及公开 integration test。新增 `design/gallery/component-coverage.json` 只记录逐项审计结果、证据和当前新增组件的语义适用性；不复制状态 profile 或维护另一套 token 值。当前 capability inventory 从有效输入重新生成，所有新输入都进入内容哈希。

全部45个当前组件共269项设计必需状态已逐项记录。39项同身份的历史组件从原 profile 与该组件的明确 states 求并集，保留 Checkbox 的 mixed 等额外要求；不将历史 SelectionSurface 的未实现跨节点选择静默合并进 Text，也不删除原包记录。缺失、重复、未知状态记录和组件记录都会报出具体 route/状态。

已审阅38项真实生产 View 状态夹具的源码分支，覆盖 Button、Checkbox、三个输入变体、Field、Toolbar、NumberField。其余231项是**夹具待核验记录**，不等于231项运行时功能未实现；先前自动、GPU、IME/AX 原生证据仍分别保留。即使源码夹具已核对，运行时交互、组合状态和最终体验也没有由此自动通过。

## 适用范围与合理例外

统一治理和 Docs 接入覆盖45个组件。现有原包没有对应项的6个组件复用既有 profile，并保留理由：

- Surface、Skeleton 使用 static；子组件或宿主拥有交互与 loading 生命周期。
- 当前 Toolbar 是动作组，复用动作状态词汇；loading 属于子动作，不新增组级 busy 状态，也不冒充窗口级 Toolbar。
- Field 负责 label/control/message，保留 rest/disabled/error；编辑器的 focus、IME、选区等由子输入承担，不复制一套编辑状态机。
- Select 复用 choice；它仍是固定选项选择，不按 ComboBox 方式新增编辑器。
- NumberField 复用既有指针/焦点/禁用词汇；它是有上下限的 stepper，dragging 不适用，minimum/maximum 是现有边界夹具，不新增 profile。

这些例外只适用于没有历史同身份规范的当前新增项。生成器拒绝用例外静默删除原组件已有的必需状态。

## Token 引用的准确边界

从现有 JSON component-contracts 提取精确的规范 token ID、family 和 JSON pointer，并展开颜色/尺寸/文字样式等 alias 依赖。当前18个组件有90条直接引用，共62个去重 token；其余项明确显示没有该项的完整 JSON 引用，不能从“未登记”推断“不用 token”。

TextArea、SearchField 已经共享 TextSurface 编辑器，因此显式复用 TextField 的既有 JSON token 契约，记录来源与理由；不复制 token 定义，也不把这些引用写成完整运行时依赖。未知 token 形态、断裂或循环 alias 会失败。索引覆盖精确 canonical ID 和明确 token 形态，不是任意自然语言描述的解析器。

这是**契约引用索引**：`token_dependencies.tokens` 仍为空、status仍为 `runtime-unverified`。它不证明组件只访问这些 token，不覆盖全部布局/动效/间接调用，也不替代已有 effective layout/elevation 对齐检查。尤其不能将原包 component-contracts 的旧 shadow 字段推断为当前 Dialog 没有阴影。

## 公共 API、真实接入与 Gold7

| 层级 | 本批验证及边界 |
| --- | --- |
| 公共 API | 三项库外 integration tests通过：保留45项/80条组件绑定和180条公开类型绑定；新增从原 profile/组件声明独立重算必需状态，并通过公开 tokens API 检查90条契约引用在四主题可解析。编译/解析不证明实际组件已调用这些 token。 |
| 治理门禁 | 28项 Python tests通过。新增检查覆盖遗漏 composing、profile增加要求、重复状态/组件、静默删除历史状态、失效夹具源码锚点、伪造原生通过、未知 token、断裂/循环 alias 和无效契约继承。两个独立完整源副本的实际 CLI 负例分别准确拒绝遗漏状态与无效 token，活跃WIP没有注入负例。 |
| 真实接入 | 统一 Docs helper读取同一 capability inventory，为全部45页显示必需状态、夹具核对数量、契约token及未验收边界。保留既有字体、排版模板、视觉token和组件实现；两个 Docs renderer分支统一接入，没有页面局部样式补丁。独立 composition-consumer 13项通过。45页×四主题×两组布局/倍率输出360张真实GPU图。 |
| Gold7 | 本批改善G7的能力可查性和状态透明度；抽查浅色Button与窄窗口大字号/@2x深色IC TextArea，确认长状态/token文字按现有模板换行、滚动可达。前后差异限于统一文档内容，配对图清单独立保存。抽查同时发现旧取证只滚动一次，换行改变范围后未到真实底部；已复用现有 state_capture 的布局稳定/滚动校准方法，并断言360例均到真实底部后再绘制。仅调整测试辅助方法的可见性，没有新增滚动实现或修改产品行为。没有把360帧或源码锚点当成人工逐页、键鼠、IME、VoiceOver通过；G4/G5/G6待验收项不晋级。 |

fmt、controls/Gallery all-targets严格Clippy、DS、纯Rust、Gold7 mechanical、生成新鲜度和diff检查通过。本批是元数据、开发者Docs与验证变更，没有修改生产控件状态机/绘制或视觉规则；未重跑全工作区测试，不将旧897项结果记成本批。没有性能测量。

## 原 Issue 未完成项与下一步

#8整体仍为部分完成。状态要求与记录遗漏的门禁已补齐；记录中的231项夹具核验、实际状态/API与组合行为覆盖、完整逐组件运行时token依赖、原生/整页最终验收仍未闭合。45项继续 experimental，experience不自动晋级。CI可以指出缺失的状态审计记录，但不能仅凭记录推断运行时状态已经实现。

接下来按已有组件族与证据逐项收拢这些记录，先检查共享状态/语义能力是否缺失，再决定修公共实现还是仅补夹具。不要为凑状态或token表新增抽象。随后按依赖推进#7/#10/#11公共边界，并继续在#5/#12保留原生验收阻塞。未提交、推送、tag、关闭Issue或发布。
