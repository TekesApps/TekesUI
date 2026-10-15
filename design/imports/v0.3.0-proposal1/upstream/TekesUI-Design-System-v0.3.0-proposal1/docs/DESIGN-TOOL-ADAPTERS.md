# Claude Design / Figma / Coding Agent 适配契约

**DS-008 · v0.3设计契约；没有操作任何在线工作区。**

## 一个系统，多个受控视图

现有私有设计artifact仍是作者真源；批准发布包绑定共享base、消费者品牌配置与派生策略。Figma、Claude Design和代码侧可生成提案或查看投影，但不能绕过评审改写冻结产物。外部工具回传的改动必须指出原版本、稳定token/组件ID、差异、影响面和审批依据；过时版本先rebase。没有自动last-write-wins。

来源方法见 [REFERENCE-POSTS](REFERENCE-POSTS.md)。本契约不要求使用任何指定厂商；工具适配是可选能力。

## Figma变量映射

`tools/context.py` 生成 `design/adapters/generated/figma.variables-plan.json`。这是**Tekes自己的映射规划格式**，不是可直接提交Figma REST API的请求，也不是Figma原生变量导入文件。

共享父集合提供相同稳定变量ID、四个mode、描述与语义别名。品牌扩展只覆盖派生基础颜色值，其他变量继承。图层绑定语义/组件变量；不能按品牌、light/dark复制一套Button。variant保留尺寸、用途、适用状态，brand/theme在上下文选择。[FG-01、FG-02]

规划保留 `token_id`、拟显示名称、类型、单位、tier、alias_target与解析值对照。实际 `figma_variable_key` / `figma_component_key` 未建立时为null。别名必须在当前品牌集合语境解析；**能否正确穿过扩展集合边界须用真实往返测试证明**，不能只看规划JSON判断成功。

Figma扩展集合的mode和变量结构由父集合控制；本设计同样不允许消费者临时添加第五主题或改token类型。颜色＋alpha一起覆写；本版品牌仅支持opaque sRGB，导入透明色不拆成无关修改，也不无声丢alpha。[FG-02]

无扩展集合能力时，可由同一输入生成独立品牌集合投影；不是独立维护副本。每次都记录输入版本、受控映射与新鲜度。排版复合样式、真实组件绑定、资产及双向导入器均待实现；规划包含数值token但不声称Figma已完成单位转换。

## Claude Design与Coding Agent交接

`design/adapters/generated/agent-context.json` 从同一目录与receipt生成，包含系统版本、所有显式品牌配置、token路径、状态矩阵、组件API、用法/禁区、recipe与页面契约、证据状态和输入哈希。`docs/AGENT-HANDOFF.md` 是可读索引，非另一套规范。

设计资料要包含真实已验收组件用法、典型页面和反例，而不是只给调色板。[CD-01、CD-02、FG-04] 本包没有真实原生示例，故引用现有设计级recipe并保留L1/L2/L3未验证标记；不能让AI把planned API当作已有库直接导入。

实施后再填 `code_symbol`、repo-relative source path、代码commit、公开API与Gallery factory。本次只建立字段与缺失检查，不虚构路径。Claude官方指南中的React导入建议不是本Rust系统已支持的证据。[CD-02]

## Agent修改边界

Agent可发现硬编码、失效绑定与已废弃ID；只有迁移表给出一对一且语义明确的替代才可提出机械修改。所有拟发布改动仍须评审；无法确定用途、行为或两个候选都可能正确时保留原样并记录问题，禁止从外观猜测。[FG-05]

变更先输出影响集合：token→组件→recipe→页面→brand/mode→证据。不得创建 `blue-button` 或 `clay-card` 副本，不得flatten别名，不得把Figma局部覆盖自动提升成共享token。品牌切换不能改变快捷键、输入法、授权、撤销或回调payload。

## 自动检查与未验证边界

已实现：规划与目录的稳定ID集合一致、别名图保留、非品牌值无覆盖、四mode完备、context输入摘要/生成物新鲜度、来源采纳映射中的ID和文件有效。明确新增颜色/状态由原品牌校验器复核。

未验证：真实Figma变量导入/导出与扩展别名解析、实际Code Connect映射、Claude Design导入、Rust生成器、原生Gallery及真实页面。只有采用对应适配器的消费者才需要其往返证据；不把工具接入设成所有使用方的强制依赖。
