# Issues #1–#14 当前审计与后续顺序（2026-10-19）

审计基线：TekesUI checkout `80d6b05649234f953ea452d42e1cbed369fdddbb` 加当前全部 WIP。所有结论均限定于该工作树和所列证据。#1–#14 中 #2 已在 main 完整 CI 通过后关闭，其余继续整改；#15–#21 已在公开 `431093786de47b48cd566d0d9c0eafa088b10d68` 精确源码及 revision 独立复验并关闭，不把这组消费方验收当成 Gallery 的全组件验收。

本文件是 #13 的当前进度入口，并同时更新 #14 审计/规划的可审阅结论。它补充、不覆盖 [2026-10-17 基线审计](../../design/reviews/issues1-14-closeout-20261019/issue-13.md)、[原 #14 范围与标准](../../design/reviews/issues1-14-closeout-20261019/issue-14.md)、[2026-10-18 组合审计历史](composition-audit-20261018.md) 与 [原始 remediation ledger](../../design/remediation/ledger.json)。机器可读状态与结论见 [audit-13-14-review.json](../../design/reviews/issues1-14-closeout-20261019/audit-13-14-review.json)；#14 的组件级证据仍以 [composition-audit.json](../../design/remediation/composition-audit.json) 为准。

## 状态口径

- **已完成**：本 Issue 的明确范围和验收证据完整；不代表其他 Issue 或整库体验完成。
- **已实现待验收**：核心实现和限定场景证据已在，尚有指定消费者、原生或发布验收。
- **部分完成**：有真实实现或局部验收，但原定系统范围仍缺关键族、证据层或端到端门禁。
- **未实现**：本范围的主要能力尚无可确认实现。
- **依赖阻塞**：所需前置交付或授权尚未满足；不把此类状态折算成完成。

#2 已完成并关闭。其余条目按各自原验收条件独立收口，不统一等待 #12；审计交付与后续实现分开判断。下表不是对尚未交付候选的关闭声明。

## #1–#12 当前进度

| Issue / 原标准 | 当前状态与实际证据 | 依赖及后续 |
|---|---|---|
| [#1 共享视觉强调与边界层级](https://github.com/TekesApps/TekesUI/issues/1)：设计源统一规则，迁移所有组件并验收整页层级。 | **部分完成**。设计源、语义 tokens 与组件绘制已有实质实现；部分组件族有当前状态/token 检查。全组件一致迁移、完整 Gallery 页面和原生/Gold7 体验验收仍未完成。参见 [设计源](../../design/source/)、[Gallery 设计基线](../../design/gallery/workbench.json) 与 [#13/#8 当前门禁及范围说明](gallery-lifecycle-gates-20261019.md)。 | 按共享规则继续覆盖组件族，结合 #8 的实际 token 依赖和 #12 页面验收；不以单个控件或抽样页面代替全范围。 |
| [#2 CI 恢复](https://github.com/TekesApps/TekesUI/issues/2) | **已完成，已关闭**。当前作者投影与历史包完整性检查已分离；截图语言环境和 fallback 行度量测试已修复，未放宽门禁。 | 精简 7 文件 [PR #24](https://github.com/TekesApps/TekesUI/pull/24) 经明确授权合入；main `dbe5f72170c26bc1d4c227f8274bb922e3dbf1ca` 的[完整 CI](https://github.com/TekesApps/TekesUI/actions/runs/39228327983) 全绿后关闭。 |
| [#3 原生 Tab / Shift+Tab 焦点遍历](https://github.com/TekesApps/TekesUI/issues/3)：接入统一遍历，覆盖编辑器命令与全族路由。 | **部分完成**。已有 ContentHost 路由、TextSurface 编辑命令优先及 Gallery 局部原生 Tab/Shift+Tab 场景；不能据此推断全组件族和物理键矩阵已验收。Gallery 当前冷启动失活语义动作也有单独原生回归，见 [门禁报告](gallery-lifecycle-gates-20261019.md) 与 [组合协议审计](composition-audit-20261018.md)。 | 扩展公共路由和物理键矩阵，覆盖受控编辑器、复合控件、窗口切换及失活恢复；与 #4/#5 的真实生产者边界联验。 |
| [#4 浮层焦点生命周期](https://github.com/TekesApps/TekesUI/issues/4) | **已实现待验收**。公共调用者交接已实现，Gallery 与独立消费者接入；公共 235、consumer 132、Gallery 412 通过；U2790B 最终原生非空查询三种关闭与迟到文本通过，源码和证据见 [本轮报告](command-palette-focus-20261019.md)。 | 按本项五条条件补齐候选交付及组字 Escape 优先权证据；不以 #12 全库完成为关闭前提，不将协议模拟当作物理候选验证。 |
| [#5 VoiceOver 控件语义树](https://github.com/TekesApps/TekesUI/issues/5)：真实控件 AX 语义、原生桥接、VoiceOver 验收。 | **部分完成**。10/19 的现有 Gallery producer 已覆盖 Button/Text/Dialog、选择控件、List/Tree/Table、Menu/Select、FilteredMenu/ComboBox/CommandPalette 等实际生产者；相关独立批次都有源、公共消费者与限定原生 AX 证据。旧 ledger 中“FilteredMenu/Combo/Palette/List/Tree/DataTable 未接入”的概括已过期，具体范围见 [selection](../../design/reviews/selection-accessibility-20261019/receipt.json)、[list](../../design/reviews/list-accessibility-20261019/receipt.json)、[menu](../../design/reviews/menu-accessibility-20261019/receipt.json)、[FilteredMenu](../../design/reviews/filtered-menu-accessibility-20261019/receipt.json)、[Tree/Table](../../design/reviews/tree-table-accessibility-20261019/receipt.json)。完整生产 Gallery 语义层级、Inspector、通知矩阵、完整 VoiceOver 和物理 IME 仍未验收。 | 继续剩余 producer 家族和 Inspector/通知语义，再按授权完成 VoiceOver/物理输入验收；不得把局部原生 AX 观察说成全量 VoiceOver 接受。 |
| [#6 设计投影门禁与 CI](https://github.com/TekesApps/TekesUI/issues/6)：校验生成品牌投影实际内容，接入完整检查。 | **部分完成**。本地 freshness、schema、设计合并、golden/历史资产等门禁在对应 WIP 有运行记录；本项新增门禁的最终交付版本仍未通过远端 CI；#2 的旧故障已经解决，但 #2 的精简 PR 不包含本项全部 WIP 门禁。见 [门禁报告](gallery-lifecycle-gates-20261019.md)。 | 保持内容校验，准备本项门禁的精简交付并完成同提交远端验证；不能把 #2 的 CI 通过表述为本项所有新增门禁已上线。 |
| [#7 消费者 BrandConfig 与独立接入示例](https://github.com/TekesApps/TekesUI/issues/7)：公开消费者品牌配置，独立项目可接入且不改库内部。 | **已实现待验收**。公共 palette/品牌配置与离线编译入口已有两套独立消费者 workspace、默认/自定义 token 和真实绘制检查记录。证据与当前边界见 [ledger](../../design/remediation/ledger.json) 的 #7 条目及 [独立消费者示例](../../examples/composition-consumer/)。剩余是按原条件逐项核验最终交付身份与持续验证。 | 按原五条条件核验交付方式、两个同 revision 不同 seed 的 workspace 外消费者、错误/版本边界与持续验证。原条目不要求全库 native/AX 或 #12 先完成，不额外增加关闭条件。 |
| [#8 能力与状态登记](https://github.com/TekesApps/TekesUI/issues/8)：当前有效状态/API/token 依赖清单，废弃过期机器报告。 | **部分完成**。45 组件、状态和 API 绑定已重新生成并对当前输入复核；账本仍记录 269 项要求中 131 个公开状态已核验、125 个夹具待验、13 个语义例外，实际 token 依赖和体验范围不完整。全部组件继续标记 experimental。见 [当前登记与门禁报告](gallery-lifecycle-gates-20261019.md)、[capabilities.json](../../design/gallery/capabilities.json) 与 [component-coverage.json](../../design/gallery/component-coverage.json)。 | 完成剩余状态、真实 token 来源和对应原生/成熟度证据；消费最新 Gallery WIP 变化后再确认来源绑定。 |
| [#9 Menu / Popover / Dialog 动效生命周期](https://github.com/TekesApps/TekesUI/issues/9)：进入/退出/中断/失活状态正确，真实 Gallery 生命周期和 Gold7 观察。 | **部分完成**。Gallery 私有 Menu/Popover 在窗口失活且未绘制中间帧时残留退出 paint 的问题已修，Select 已作为现有取消清理对照；真实 Gallery 87 交互、29 AX 与严格 Clippy 通过。新修复仍缺原生窗口、GPU 中间帧和 Gold7 复验。Dialog 逻辑模态跨短暂失活保留是现行有意语义。详见 [transient lifecycle 报告](gallery-transient-lifecycle-20261019.md)。 | 先完成 #2 的 golden 输入调查，再对当前修复补原生窗口、GPU 中间帧及有界 Gold7；既有 Dialog 原生证据不能替代新私有修复验收。 |
| [#10 阅读位置保护](https://github.com/TekesApps/TekesUI/issues/10) | **已实现待验收**。公开 ReadingController、真实 Text/ScrollArea 与独立消费者、160 帧 GPU 场景已有证据，见 [阅读协议](reading-controller-20261018.md)。五项原条件逐项复验与当前最终候选身份尚待收口。 | 补当前候选原生跟随/历史/变窄/回到最新操作；VoiceOver 与物理 IME 的全局剩余项归 #5/#12，不增列为 #10 未要求的实现。 |
| [#11 草稿确认与恢复](https://github.com/TekesApps/TekesUI/issues/11) | **已实现待验收**。DraftJournal、实际编辑器、原子文件适配、失败/重试/迟到确认与销毁重建均已有代码和原生证据，见 [草稿协议](draft-protocol-20261018.md)。 | 按原五项条件复核最终候选并交付；表单 AX/VoiceOver 继续在 #5/#12 跟踪，不扩充 #11 关闭条件。 |
| [#12 整页视觉与原生交互验收](https://github.com/TekesApps/TekesUI/issues/12)：整页、原生输入、视觉、可访问性和成熟度形成统一验收。 | **部分完成**。已有多个独立批次按 API、consumer、GPU/Gold7、native、AX 分层收据；当前并没有覆盖全部组件、主题、品牌、窗口和页面的统一验收。组件仍是 experimental。见 [Gallery 验收范围与层次](gallery-lifecycle-gates-20261019.md)、[组件登记](../../design/gallery/workbench.json)。 | 聚合当前有效且精确绑定源码的逐族证据；只为实际完成的范围晋级，不由抽样小宿主、过往快照或模型测试推导整页/全库成熟。 |

## #13–#14 当前交付

### #13 总体进度与可恢复账本

**状态：部分完成；本文件与配套 JSON 提供 10/19 当前增量总览，整体整改仍进行中。** 根账本保留历史轨迹与机器检查状态；它不是当前每个 Issue 的唯一结论来源，尤其 #5、#9 的陈旧汇总应以本报告和当批 receipt 为准。#2 的最小修复已合入并通过 main 完整 CI，按授权关闭。

当前工作树有大量继承 WIP。本报告同步本轮实现、矩阵与验收进度；#2 的明确授权合入与关闭单独留证，AgentKit 已验收消费包保持不变。AgentKit 的 #15–#21 独立关闭只证明它们声明的消费者范围，不提升 #1–#14 的状态。

### #14 全组件组合协议审计与规划

**原标准：**覆盖全部实际公开 UI 组件，分别审查“可被组合”和“可容纳内容槽”；核对身份与状态、Measure/Layout、Paint/clip、事件消费/取消、Focus、Theme、失效、AX、消费者扩展和兼容性；先用复合 Button、异构 List、Rich Dialog 验证最小协议，再分族迁移。Window Toolbar 必须由 Window 注册/持有，支持稳定条目、分组与位置、可组合自定义内容、溢出、命令状态、窗口隔离和关闭生命周期。全程保持纯 Rust 与现有窄平台接口，不把内容区控件行说成窗口 Toolbar。

**当前审计状态：审计/规划主体已实现，仍待 Issue 审阅与后续限定验收；不是全库迁移完成。** [机器矩阵](../../design/remediation/composition-audit.json)列出 45 个登记组件及当前声明/适配证据；[审计正文](composition-audit-20261018.md)解释分类口径、公共组合与拥有式内容的差异、运行时能力和各族限制。文档给出的公开声明锚点应按该审计版本理解，不要只数 JSON 中证据条目代替其锚点规则。

已具备的最小协议基础包含 owned Component/content host、测量与布局、裁剪/主题、事件消费与捕获取消、焦点、生命周期/显式卸载、失效以及有限语义投影；对消费者可扩展。三类最小场景已有 public/GPU 和分项消费者证据，但完整组合后的物理 IME、原生候选窗、全量 VoiceOver 和当前统一接受仍未完成。静态槽内容和 API 适配不能代表所有组件族均已迁移。

**具体缺口与计划：**

1. 继续使用当前 45 组件矩阵对照真实公共声明和适配，不为尚不存在的 ListBox、Sidebar、Toggle、Accordion、ToolbarGroup 或 ToolbarItem 创建虚构条目。
2. 保持原组件便利 API 和状态语义；先选 Tabs label/panel 或 Disclosure/Popover 等有明确内容责任的一个族，在独立消费者和 Gallery 中逐批迁移。之后再看 Tree/DataTable 稳定身份、动态集合及虚拟化策略。虚拟化目前没有统一重建和编辑状态保留政策。
3. 语义桥继续依赖 #5 的真实 producer、AX 和辅助技术工作；任何全局 Content 抽象必须补现有所有者边界的证明，不能把数据模型或绘制命令变成子控件。
4. Toolbar 仍属未来架构。当前 `ToolbarView` 是内容区的固定按钮/几何入口，不是 Window 级 Toolbar；须等组合协议及焦点/语义依赖稳定后，先做独立消费者单窗口注册/布局，再考虑 overflow 与多窗口生命周期。不要把规划写成当前实现。
5. 兼容方面已有新增协议类型、非穷举事件和类型默认方法的讨论，但旧 API、公开 struct literal、平台角色穷举匹配和 palette 错误行为仍需逐个按公开版本及消费者迁移策略处理。该审计不是完整外部消费者 Breaking Change 认证。

建议顺序：持续整理 #2/#6 的可信生成与远端门禁；按 #1/#8 规则完成实际 token/视觉状态族；#3/#4/#5 并行补全焦点、浮层和真实语义生产者；#9 完成当前修复的原生/GPU/Gold7；#7/#10/#11 补各自指定的消费/原生验收；最后由 #12 汇总逐族、逐主题/产品范围的有效证据。#14 的后续实现按小批选型，并以 #5/#8/#12 与独立消费者结果作为依赖；不在此报告中授权或声称已执行整条路线图。

## 明确保留的验收边界

- 机器测试和 Clippy 通过只说明相应代码/用例通过，不证明当前 Gallery 的完整 native UX。
- Gallery 局部原生 AX/键盘、AgentKit 独立消费者和 VoiceOver 人工结果各有边界，不互相替代。
- #9 已补当前反馈范围的新原生/GPU/Gold7 记录，见 empty-state-measure-20261019/acceptance.json；完整浮层族与原 Issue 其余条件继续单独核对，不从局部接受推导全部完成。
- #2 旧失败保留为负例；独立候选与 main 最小修复均实际远端通过。PR #24 已合入，main dbe5f72 的完整 CI 39228327983 通过后，#2 以 completed 关闭。
- 性能测量不属于这批授权验收，未测，且不作为本轮可执行工作的阻塞理由。

## 证据入口

- [#13 原始审计与 Issue 标准](../../design/reviews/issues1-14-closeout-20261019/issue-13.md)
- [#14 原始完整审计标准](../../design/reviews/issues1-14-closeout-20261019/issue-14.md)
- [10/18 全组件组合能力正文](composition-audit-20261018.md)
- [45 组件机器审计矩阵](../../design/remediation/composition-audit.json)
- [remediation ledger（保留历史）](../../design/remediation/ledger.json)
- [#13/#8/#2/#6 当前门禁与生命周期报告](gallery-lifecycle-gates-20261019.md)
- [#9 Gallery 瞬态浮层修复报告](gallery-transient-lifecycle-20261019.md)
- [#5 selection receipt](../../design/reviews/selection-accessibility-20261019/receipt.json)、[List receipt](../../design/reviews/list-accessibility-20261019/receipt.json)、[Menu receipt](../../design/reviews/menu-accessibility-20261019/receipt.json)、[FilteredMenu receipt](../../design/reviews/filtered-menu-accessibility-20261019/receipt.json)、[Tree/Table receipt](../../design/reviews/tree-table-accessibility-20261019/receipt.json)
- [#15–#21 独立消费者验收边界](agentkit-final-acceptance-20261019.md)

## 关闭范围复核（#2 合入后）

已有独立只读审计：audit_13_14 核对总览和架构审计范围；audit_remaining_acceptance 核对已有共享实现与真实生产者；audit_delivery_docs 整理证据索引。它们不是对当前最终 WIP 的完整独立构建、原生或 VoiceOver 验收。

#14 的审计/规划主体已交付，下一步是将精确可获取版本、矩阵、兼容性和未完成项回填原 Issue 并复核；不等待窗口 Toolbar 或全组件迁移。#13 可继续作为用户指定的总览入口，这属于跟踪安排，不代表每项修复必须等总览关闭。#4/#7/#10/#11 各按自己的原验收条件完成最终候选复核后独立关闭；#1/#3/#5/#6/#8/#9/#12 仍有实现或范围证据缺口，不能仅以统筹安排解释其开放状态。

## 八批执行进度与本轮反馈修复

用户确认按八批顺序推进，执行入口见 [issue-component-batches-20261019.json](../../design/remediation/issue-component-batches-20261019.json)。第一批现已按 EmptyState 最终测量修复版本收口，当前有效入口为 [限定接受记录](../../design/reviews/empty-state-measure-20261019/acceptance.json)，不再使用先前被后续视觉缺陷撤回的 notice-alignment 接受结论。所有旧截图和失败记录保留；原生宿主已关闭清理，没有仍在使用的反馈 App。

第二批的当前 WIP 本地门禁、七项真实 token 探针与精确源码矩阵已刷新。原提交加 WIP 的 304 文件审计候选 `audit-213cfe8c81b687850aff683a96b5549277fa22cb1c3face7f4cca633dcb529c1` 已在独立目录实际重放能力清单／矩阵检查和各自负例回归。交接说明见 [HANDOFF](../../design/reviews/batch2-delivery-20261019/HANDOFF.md)。这是可独立复核的本地审计包，未冒称另一位审阅者已接受或公开 Git pin／远端 CI 已通过。

#6 补充普通 Cargo 路径的实际反例：独立最小双包宿主正常／恢复构建通过，色值篡改、缺输入 hash、缺主题和前景背景同色四个变异都被真实 build.rs 拒绝；详见 [收据](../../design/reviews/brand-build-path-20261019-attempt3/receipt.json)。原工作树哈希不变，产物已清理。此前两次宿主准备失败完整保留，不计入产品负例通过。

第三批不依赖发布的预核验同步推进，逐项范围以 [原 Issue 对照](../../design/reviews/mature-capabilities-20261019/remaining-by-original-issue.json) 为准。#7 的同源码双外部消费者／16帧真实GPU复验通过并清理；#10 已有 ReadingController，#4 已有公共调用者交接，不重复实现。#7/#10/#11 不额外以全库 VoiceOver 或 #12 完成为关闭前提，但当前最终候选的指定消费者／原生证据和交付身份仍须补齐。第二批公开交付／远端 CI 和最终独立审阅继续单列，不自动提交、推送或关闭 Issue。

## 最新增量：反馈批次完成，进入审计／门禁批次

本轮第一批已按实际最终共享源码完成限定接受。EmptyState 缺少公共自然尺寸入口、Gallery 重复测量与 Surface 留白不符，导致窄宽／150% 的两行按钮由 56pt 裁成 50pt；公共 measure 与 render 已共用计算，Playground／矩阵／变体和独立消费者统一接入。按钮默认居中、通知首行标记对齐及拥有动作焦点路径均保留。公共 API 321、独立消费者 134、Gallery 417 通过／1 既有忽略；24 帧 EmptyState 与64 帧通知 GPU、外接 U2790B @2x 原生／限定 Gold7 通过。宿主和自身构建产物已清理，VoiceOver 未启动。详见 [限定接受记录](../../design/reviews/empty-state-measure-20261019/acceptance.json)。

第二批已重新读取 #6/#8/#13/#14 最新正文，保存原始响应；七项实际 token 探针按当前共享源码重新执行，45 组件清单与组合审计精确源码／行号经复核刷新。顶层脚本回归、品牌投影、合并设计、设计校验、参考语义、Gold7 机械检查、纯 Rust 边界与能力／审计负例均本地通过。一次使用系统 Python 缺 jsonschema 的失败日志保留，改用已有 review venv 后原检查通过，没有放宽检查。

这批结果是当前 WIP 的本地证据，不是新的公开提交或该提交远端 CI。#6 仍需最终交付版本的远端验证；#14 仍需原 Issue 回填及最终独立审阅，完整 Toolbar 架构与全库迁移继续为后续规划。#8 的有限状态/token 覆盖保持有限，不自动提升 experimental。#13 的过期“#15–#21 pin 阻塞／#2 CI 故障”活动描述已纠正，历史记录保留。见 [审计门禁记录](../../design/reviews/audit-gates-empty-measured-20261019/acceptance.json)。本轮没有提交、推送或关闭 Issue。
