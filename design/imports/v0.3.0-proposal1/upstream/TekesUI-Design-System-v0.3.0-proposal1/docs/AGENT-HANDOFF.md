# Agent 设计交接索引

版本：`0.3.0-proposal.1`。上下文摘要：`4653d73115702d463263262af89808ba51b4895e12507002cd27412c725f98f4`。

由 `tools/context.py` 生成；所有 API 是提案，不能当作已存在的 Rust 符号。

[主规范](../SPEC.md) · [品牌契约](BRANDING.md) · [组件/API上下文](../design/adapters/generated/agent-context.json) · [Figma映射规划](../design/adapters/generated/figma.variables-plan.json)

显式参考品牌：example-neutral, example-blue, example-clay，每品牌四主题。品牌不是组件variant，未配置不得默认为陶土。

40项组件、10项消费层recipe和真实页面契约来自登记表。使用语义token；保留别名图，不以解析色值反写设计源。界面自动修复只能提出明确迁移差异，歧义留待评审。

Figma变量键、组件键、Rust源码位置/commit均未核实，保持null。规划不是在线导入证明，颜色表不是原生Gallery。

[Claude/Figma参考](REFERENCE-POSTS.md) · [Jenny Wen原帖与访谈](JENNY-WEN-POSTS.md) · [真实体验评审](../design/quality/craft-review.json)

流程可以缩短，验收不能省略。L1组件、L2应用、L3真实环境各自留证；批准版本与生成物绑定，修改后重验。
