# Claude Design / Figma：重点参考与采纳记录

v0.3 · 检索日期：2026-10-15。优先使用官方正文；不是以社交帖子替代验收。

**最相关的变化：**品牌不是一个固定色值，也不是几张换色截图，而是可继承、可发布、可追溯的配置；AI需要语义与真实组件映射。

| 来源 | 日期 | 对 v0.3 的影响 |
|---|---|---|
| [Introducing Claude Design by Anthropic Labs](https://www.anthropic.com/news/claude-design-anthropic-labs) · `CD-01` | 2026-04-17 | 从既有代码与设计资料建立可复用系统，并在项目中应用品牌、字体与组件。 品牌配置与组件契约分离；所有生成项目绑定明确设计版本。 |
| [Set up your design system in Claude Design](https://support.claude.com/en/articles/14604397-set-up-your-design-system-in-claude-design) · `CD-02` | 持续更新／日期未核实 | 同时涵盖品牌、排版、组件和布局；建议用真实项目核对；当前指南区分品牌资料导入和React组件同步。 交接包包含用法与页面上下文；Rust适配未验证；不照搬文中组件预览的键盘限制。 |
| [Get started with Claude Design](https://support.claude.com/en/articles/14604416-get-started-with-claude-design) · `CD-03` | 持续更新／日期未核实 | 设计工作使用已有系统组件与上下文，并支持向代码工作流交接。 让AI接收结构化token、组件ID、API与证据状态，而非凭截图重建。 |
| [New in Claude Design: stays on brand across projects](https://x.com/claudeai/status/2067325887909884315) · `CD-X01` | 持续更新／日期未核实 | 公开索引提及跨项目品牌一致性及与Claude Code同步。 仅作发现线索；具体契约以官方正文与本地验收为准。 |
| [The future of design systems is semantic](https://www.figma.com/blog/the-future-of-design-systems-is-semantic/) · `FG-01` | 2023-07-27 | 用语义变量与引用关系连接设计和代码，不把浅深主题扩展成重复组件。 稳定语义ID；保留别名图；品牌不是组件variant。 |
| [Extend a variable collection](https://help.figma.com/hc/en-us/articles/36346281624471-Extend-a-variable-collection) · `FG-02` | 持续更新／日期未核实 | 子集合继承父集合未覆盖值；结构与modes由父集合统一；颜色和透明度覆盖不可分。当前功能有Enterprise限制。 共享base＋显式品牌覆盖；四主题固定；Figma工具适配可选；不把颜色覆盖误作仅alpha变化。 |
| [Schema 2025: Design systems for a new era](https://www.figma.com/blog/schema-2025-design-systems-recap/) · `FG-03` | 2025-10-28 | 扩展集合用于多品牌设计系统，减少重复维护。 单套组件目录，多个品牌配置；不按品牌复制组件库。 |
| [Design context, everywhere you build](https://www.figma.com/blog/design-context-everywhere-you-build/) · `FG-04` | 2025-09-23 | Code Connect把设计组件映射到代码位置与用法，MCP传递结构化上下文。 登记ID↔API↔源码位置↔Gallery；未知源码保持null，不假装已连接生产代码。 |
| [Workflow lab: Staying in the flow with Figma’s agent](https://www.figma.com/blog/workflow-lab-staying-in-the-flow-with-the-figma-agent/) · `FG-05` | 2026-10-06 | 先检查旧组件及变量绑定；明确可替换项可修复，歧义项留给设计者判断。 自动修复限定稳定ID与显式迁移表；视觉或行为语义不明时产生待评审差异。 |
| [Guide to variables in Figma](https://help.figma.com/hc/en-us/articles/15339657135383-Guide-to-variables-in-Figma) · `FG-06` | 持续更新／日期未核实 | 变量、集合、modes、别名与设计代码同步具有不同职责。 Figma映射规划保留别名和四主题，不把解析结果反写为作者常量。 |

## 裁决与不照搬的边界

这些资料支持语义分层、品牌继承、组件复用与上下文交接；**没有规定从一个seed生成Tekes四主题的具体算法**。OKLCH目标、状态间隔与搜索策略是本提案选择，需另经消费者视觉验收。

Figma扩展集合是工具能力，不是运行时依赖。原生TekesUI不得为了换品牌强制购买某种Figma计划。本包生成的是适配规划JSON，**不是Figma API原生请求或已导入的.fig文件**。[FG-02]

Claude当前指南推荐React现有组件使用其代码同步流程；本案是Rust系统，未证明可以直接同步。仅采纳“读取真实组件、保留使用规则”的交接方法，不声称已实现一键集成。[CD-02]

Claude帮助文档也注明其某些设计系统实时预览无法用Tab操作。本系统仍要求真实Gallery键盘可达，不继承这项限制。[CD-02]

Figma迁移文档包含删除旧mode和重新应用mode的步骤。本案没有执行这些操作；实际迁移必须先备份、比较绑定、评审差异，再发布。[FG-02]

多工具可参与编辑，但任何未批准改动都进入差异评审；现有私有设计artifact与冻结发布包的权威关系不变。同步传输能力不等于任意双向覆盖许可。

完整来源与读取边界见 [来源清单](../research/claude-figma.sources.json)。数值基线见 [CSS Color 4](https://www.w3.org/TR/css-color-4/) 与 [WCAG对比度说明](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)。

## Jenny Wen：方法论参考补充

[本人文章与原帖、访谈分别整理](JENNY-WEN-POSTS.md)。它们补足真实产品体验、灵活探索与 AI 内容不确定性设计，并不提供本项目的品牌生成算法。新增 `craft-review.json` 将采纳决定变成可审查契约；全部运行时证据仍未验证。人物职责、文章日期和 X 访问限制均单独登记，不能混同 Claude 产品设计与 Claude Design 产品负责人。
