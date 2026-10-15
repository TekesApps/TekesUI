# 与 open-claude-ui 对齐

版本：**0.3.0-proposal.1** · 2026-10-15 · 设计修订，未实施。

## 1. 参考边界与取舍

参考仓库固定为 `Caho1/open-claude-ui` 的提交 `2834bc158a4849f21c97cbb2e0b10525f6e646f1`，提交时间2026-10-15 02:02:11 UTC。实际读取了主题CSS、按钮、动效、两种Composer、滚动hook、分割拖动处理及有关文档的明确范围。其余模块仅核实清单与文档描述，没有声称逐一复核实现。永久链接、Git blob ID 与阅读范围见 [来源锁定](../research/open-claude-ui.lock.json)。

目标不是复制React组件，而是把视觉关系和有价值的交互转译为 **Rust/mac原生、类型化token、四主题、可组合状态** 的契约。没有审计Claude原始二进制或验证该项目的“像素级”“生产级”自述；第9篇关于产品更新的内容仅作为作者报告。

优先级：Tekes输入与无障碍约束 → 用户16项验收要求 → 批准后的Tekes设计发布 → 外部参考。数值发生冲突时，优先以同一提交的CSS或组件常量选择参考值，文档只解释意图。外部仓库不是新增设计真源。

## 2. 四主题视觉映射

| 层级 | Light | Dark | Light-IC | Dark-IC | Tekes用途 |
|---|---|---|---|---|---|
| surface-0 | #F5F5F3 | #0B0B0A | #F4F4F1 | #000000 | 窗口根底、侧栏 |
| surface-1 | #FCFCFB | #151515 | #FFFFFF | #111110 | 主文档画布 |
| surface-2 | #FFFFFF | #1A1A19 | #FFFFFF | #171715 | 独立面板、代码区 |
| surface-3 | #FFFFFF | #20201E | #FFFFFF | #20201E | Composer / 输入高层 |
| surface-popover | #FFFFFF | #222220 | #FFFFFF | #252523 | 菜单、浮层、Dialog |

标准主题数值取自 OCUI-01；角色分工参考 OCUI-18 并明确为 Tekes 选择。CSS注释对canvas/sidebar的措辞和第1篇不同，不把两种角色解释同时导入。IC由本提案定义；IC不能只换背景，所有语义前景、描边、选区、代码、diff与图表继续独立映射。

v0.3不再把参考项目的陶土色当作框架身份。`example-clay` 仅保留其seed作为示例；品牌派生与对比度角色见 [BRANDING.md](BRANDING.md)。`brand-decoration`、可读 `accent`、强底 `action-primary-*` 各有职责，但共享消费者输入。v0.2普通主题的陶土底黑字选择是历史决策，不再是本版固定配对。

标准表面阶梯、几何、交互与无阴影/中性焦点继续沿用以下对齐条目；品牌四主题值由编译器生成。另加入Claude Design/Figma的多品牌与交接方法，见 [重点参考](REFERENCE-POSTS.md)；未执行在线工具导入。

## 3. 几何、字号与布局

| 对象 | v0.2契约 | 依据 / 偏离 |
|---|---|---|
| Button XS | 高24、圆角6、左右8、文字12/16 | OCUI-02的24/6/8；文字11.5提升为12 |
| Button SM | 高28、圆角8、左右10、文字12/16 | OCUI-02 |
| Button MD | 高32、圆角8、左右14、文字14/20 | OCUI-02 |
| Button LG | 高40、圆角12、左右20、文字15/20 | OCUI-02 |
| IconButton | 默认命中32，显式compact28；图标16 | OCUI-02＋原生密度适配 |
| TextField / Panel | 圆角8 / 10 | OCUI-01、18 |
| Code / Popover / Dialog | 圆角12 / 12 / 16 | OCUI-01的阶梯；角色明确化 |
| 用户消息 / Composer | 圆角18 / 20 | OCUI-01、04；助手仍为透明文档流 |
| Pill | 仅Segmented、标签等具名变体使用999并按高度钳制 | 不再让所有控件成为胶囊 |
| Chrome / 文档 / 对话 | UI13/16、长文14/22、对话与Composer14/20 | 分离导航密度与阅读节奏 |
| 侧栏 / 阅读列 | 288，范围224–360 / 最大768 | Tekes明确采用的布局选择，不宣称全仓统一像素规范 |

系统sans用于UI，系统mono用于代码；不复制Anthropic字体、不分发系统字体、不套用Web rem与浏览器缩放修正。标题、中文、日文与emoji由CoreText真实字体度量处理；禁止全局半像素偏移或统一暗色字重465之类的未经本平台验证补丁。

窗口1360×860，最小720×480。≥1200为建议三栏，960–1199建议两栏，更窄采用可切换面板；实际文本、字号和最小宽度约束优先于断点。容器级动作分组按可用宽度收拢，不藏掉发送/取消/授权等必要动作。

Composer采用“附件区＋编辑区＋工具栏”。基本编辑区44–240；整体最小72、普通上限384；可显式扩展，但始终保留必要动作。参考Basic注释48–384与Rich实现44–240并不一致，本案选择Rich的正常编辑范围，而不是混成两个冲突常量。[OCUI-04、05]

## 4. 表面、边界和完整状态

借鉴“减少套娃边框”：普通嵌套容器用间距和表面关系分组，面板内操作默认secondary/ghost。但这**只约束装饰性边框**，不能删除必要的输入框、Checkbox、独立编辑器、焦点或错误标记。

不采用阴影、陶土色发光焦点、背景毛玻璃或按压scale。浮层使用surface-popover和边界，焦点保留中性1pt；主按钮使用其专属反差内部线。视觉高度与场景z-order是不同概念。

hover仅是瞬时指针状态；selected保留值状态标记；checked表达布尔值；focus不清除selected；error+focus同时保留错误标记和中性焦点。loading维持标签与焦点，只抑制重复提交，不能照搬OCUI-02的loading→原生disabled。不存在独立的“click状态”。

## 5. 组件与工作台使用规范

通用组件由38项扩展为40项：新增 **SegmentedControl** 和 **Disclosure**。SegmentedControl单选值使用radio-group语义，切换页面的控件仍用Tabs；Disclosure提供expanded和控制对象关系，不把Thinking内容放进通用组件类型。

消费层登记10项：Composer、Message、ThinkingSummary、Attachment、ToolDecision、ModelUsage、SessionRow、ArtifactPreview，以及可选Popout、TaskBoard。数据模型、网络、权限、费用、模型能力仍由消费者提供。

ModelUsage严格区分“模型”“推理能力”“当前上下文占用”“账户额度/重置时间”。不复制参考示例的固定模型名，不把上下文容量的产品特定限制写死在通用控件中。SessionRow可提供分组和排序能力，但不照搬第9篇作者描述的历史文件夹静默归并；视觉迁移不得改变用户历史组织方式。

Popout仅为可选窗口契约：同一稳定会话ID、明确主次窗口、保留草稿、后台窗口不抢焦点；它不是本次已实现的多窗口功能。Thinking只展示后端公开给用户的摘要与执行状态，不伪造内部推理内容。

目录与设计文档由登记表生成。40个通用组件和10项使用规范都是planned；没有“看起来完整”的网页或截图冒充真实Rust Gallery。

## 6. 可迁移的交互，必须修正的缺口

| 来源与观察 | Tekes明确契约 | 验收 |
|---|---|---|
| OCUI-04 Basic Composer的Enter处理无合成守卫；OCUI-05 Rich有守卫 | 所有提交入口都先过平台marked-text；输入法先于补全和应用快捷键 | 中文/日语真实组字，候选确认不发送 |
| 两个示例提交后直接清空并trim | 保留原文；持久接受后只清对应草稿；失败可见、重试去重 | 断连、失败、并发编辑、重复点击 |
| OCUI-06：40pt底部容差、150ms输入冷却、120pt提示 | 增加reading-history锁存；上滚后不因冷却结束或小距离而强行吸底 | 流式更新与滚轮/触控板并发 |
| OCUI-07注释声明4px死区，但读取的pointer处理未见阈值判断 | 真正经过4pt移动才开始；捕获丢失/Esc恢复原始比例 | 微抖、离窗、取消、真实拖动 |
| OCUI-07 min/max依赖380+420宽度 | 先检查约束可行；不可行直接收拢，不能产生倒置区间 | 最小窗口、200%字号、长标题 |
| OCUI-02把loading绑定disabled | busy和disabled分开；保留焦点、标签与取消入口 | 键盘提交中仍可定位操作 |

这些是本提案的明确契约，不是本次已证明运行时满足的行为。既有业务发送偏好、回调payload和授权语义必须另做回归。

## 7. 动效：参数对齐，不复制持续视觉噪声

| Tekes名称 | 参考实际导出 | k / c / m | 允许用途 |
|---|---|---|---|
| spring.micro | SPRING_BOUNCY | 600 / 30 / 1 | 选中滑块等小型位移 |
| spring.reveal | SPRING_SNAPPY | 320 / 28 / 1 | 有界Disclosure/浮层展开 |
| spring.feedback | SPRING_GENTLE | 380 / 18 / 1 | 显式选择的非关键反馈；不用于正文/审批 |
| spring.panel | SPRING_HEAVY_SNAP | 1200 / 80 / 0.1 | 释放后的面板吸附；拖动中直接跟手 |

数值取OCUI-03，而不是第8篇相互冲突的表格。系数作为归一化UI模拟参数；不声明真实SI单位、固定收敛耗时或已测得帧性能。动画由初始值/速度继续，打断不能跳回初态；达到位置与速度终止条件后停止重绘，原生适配器必须在实施时定义并验证终止容差。

reduce-motion直接到终态，绕过spring模拟。关闭逐字blur、shimmer、持续背景光晕和空闲重绘；不靠缩短旋转时间假装关闭动画。

## 8. 明确记录的来源冲突

| 冲突 | 决定 |
|---|---|
| 第8篇surface色表与cds-theme.css不一致 | 标准主题以OCUI-01的CSS值为数值参考；不混用两张表 |
| 第8篇spring名称/数值与ClaudeMotionPresets.ts不一致 | 按实际导出值映射，保留外部名称和本地语义名 |
| CSS canvas/sidebar注释与第1篇用途不同 | 固定Tekes surface-0为窗口/侧栏，surface-1为文档 |
| Basic与Rich的编辑高度、IME处理不同 | 采用完整原生输入契约，选择一套尺寸并记录偏离 |
| 分割器注释的死区不等于已实现的处理 | 以实际处理流程判断已有证据，增加待执行行为测试 |
| README/文档称生产级、像素级或官方逆向 | 不据此宣称可靠性、官方地位、授权链或已通过验收 |

没有复制第三方字体、Claude品牌资产、音频、原始反编译片段。使用任何资产前仍需逐项核验来源与许可；仓库顶层的开源声明不自动解决其所有引用资产的权利。

## 9. 交付与可验证范围

保留原有16项验收框架，补充recipe schema、组件token引用、来源锁定、交互sidecar、迁移清单与生成文档。所有导入仍要回到私有artifact评审采纳，不改变唯一设计真源和单向同步。

`reports/proposal-validation.json`：本包schema/别名/单位/四主题/登记引用/文档新鲜度/配对对比度及字节完整性。`reports/negative-tests.json`：故意输入违规数据能否被拒绝。`reports/expected-gallery-cases.json`和`expected-recipe-cases.json`：待执行用例，不是通过记录。

L1原生组件、L2真实页面、L3真实环境全部unverified。布局越界、输入法、VoiceOver、最终像素、性能与Rust硬编码扫描未执行。缺少这些证据和真实设计导出时，`--release`必须失败。

## 来源索引

每个OCUI编号均指向 [锁定清单](../research/open-claude-ui.lock.json) 的文件、Git blob与读取范围。下表的“清单级”不会被当作源代码已检查。

| ID | 文件 | 检查范围 |
|---|---|---|
| OCUI-01 | [code-examples/cds-theme.css](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/cds-theme.css) | source-read；1–240: theme, radii, text and optical rules |
| OCUI-02 | [code-examples/CdsButton.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/CdsButton.tsx) | source-read；full |
| OCUI-03 | [code-examples/ClaudeMotionPresets.ts](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ClaudeMotionPresets.ts) | source-read；full |
| OCUI-04 | [code-examples/ChatComposer.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ChatComposer.tsx) | source-read；1–120: keyboard, submit, shell |
| OCUI-05 | [code-examples/RichComposerWithMentions.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/RichComposerWithMentions.tsx) | source-read；90–230: completion, IME, submit |
| OCUI-06 | [code-examples/useClaudeScrollPinning.ts](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/useClaudeScrollPinning.ts) | source-read；full |
| OCUI-07 | [code-examples/ArtifactsSplitPaneResizer.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ArtifactsSplitPaneResizer.tsx) | source-read；1–130: pointer handlers and constraints |
| OCUI-08 | [code-examples/SegmentedControl.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/SegmentedControl.tsx) | inventory-and-documentation-only；tree and README descriptions; implementation not reviewed |
| OCUI-09 | [code-examples/ThinkingAccordion.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ThinkingAccordion.tsx) | inventory-and-documentation-only；tree and README descriptions; implementation not reviewed |
| OCUI-10 | [code-examples/ChatMessageBubbles.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ChatMessageBubbles.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-11 | [code-examples/AttachmentCard.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/AttachmentCard.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-12 | [code-examples/McpToolDecisionCard.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/McpToolDecisionCard.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-13 | [code-examples/ModelSwitcherWithTokenGauge.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/ModelSwitcherWithTokenGauge.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-14 | [code-examples/DraggableSessionList.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/DraggableSessionList.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-15 | [code-examples/StreamingDiffViewer.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/StreamingDiffViewer.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-16 | [docs/09-Claude-Desktop-2.0-全量版本更新深度逆向分析.md](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/docs/09-Claude-Desktop-2.0-全量版本更新深度逆向分析.md) | author-analysis-read；1–145 and public rendered index; no upstream binary verification |
| OCUI-17 | [code-examples/CoworkAgentCanvas.tsx](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/code-examples/CoworkAgentCanvas.tsx) | inventory-and-documentation-only；tree and README descriptions |
| OCUI-18 | [docs/01-设计哲学与设计令牌体系.md](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/docs/01-设计哲学与设计令牌体系.md) | author-analysis-read；pinned 15–76; typography and other sections read on main web view, not claimed atomic |
| OCUI-19 | [docs/08-AI-Agent-前端-UI-与动效开发指南.md](https://github.com/Caho1/open-claude-ui/blob/2834bc158a4849f21c97cbb2e0b10525f6e646f1/docs/08-AI-Agent-前端-UI-与动效开发指南.md) | author-analysis-read；pinned 45–105; public rendered sections for context |
