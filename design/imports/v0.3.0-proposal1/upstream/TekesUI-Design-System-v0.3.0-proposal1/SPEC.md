# TekesUI Design System
## v0.3.0-proposal.1 · 2026-10-15

**方向：克制、清晰、可检查的专业桌面界面。** 采用暖灰中性色、消费者可配置品牌强调、紧凑导航和舒展文档；状态可组合，执行过程与证据可检查。颜色、尺寸和预算是本提案的设计决定，不是 Claude 的官方 token，也不是已经测得的产品性能。

**交付状态：设计提案＋机器可读候选输入。** 本包没有修改仓库，没有读取或更新私有 Claude artifact，没有实现 Rust 组件或 Gallery。文件中的 `planned`、`unverified` 和发布阻断均为真实状态。现有图标资产、实际负责人和真实 artifact revision 未得到核验，不以占位图、虚构哈希或假通过补齐。

### 对齐基线与参考范围

v0.1 已读取 TekesApps/TekesUI 的 README、章程、设计保存约定、图标/token 任务书和对比方法。检索时 main 指针为 `a3bdf046891db2b216ef3640a2ca7f56c1893514`；这些文档分别按 main 读取，未做整树 checkout，因此该指针是读取时观察值，不是声称所有读取经过一个原子快照。[R1–R5]

本提案沿用 v0.1 已核实的架构定位，未在本次重新审计 TekesUI 仓库：**纯 Rust、mac 优先的通用 UI foundation/components**；不能把它当作以前的 SwiftUI 客户端。既有私有 `TekesUI Design System` artifact 保持设计作者真源地位。[R1–R3]

| 参考 | 核实到的信息 | 本案吸收 | 不照搬 |
|---|---|---|---|
| Claude Code 官方 Desktop 文档 | 会话侧栏、并行会话、重排面板、终端、diff 与审阅 | 导航密集、主内容清楚、按需展开检查器 | 不克隆产品专属导航和模型功能 |
| ClaudeDevs，2026-10-01 的 Mods 帖 | 可调整行为、UI 与功能，作为插件分发 | 明确定义扩展插槽与 token/焦点边界 | 不允许扩展跳过审批或劫持输入 |
| Daniel Avila 的 Turn Inspector 帖 | 检索于 10-15，索引显示“2 天前”；逐 turn 检查上下文 | 检查器结构化展示范围、来源、结果与证据 | 不把看起来完成当作已验证 |
| ClaudeDevs 的 Projects 帖 | 用一个主会话组织并行 threads | 任务状态、层级与后台工作可见 | 不把每条内部事件都铺在主文档中 |

X 来源只能核实公开索引中的帖子内容，部分正文页无法打开；**未完整读取回复串，也不据此声称社区形成一致意见**。此前版本的视觉方向是本案设计决策。[R6–R9]

**v0.2锁定参考（本次沿用）：** `Caho1/open-claude-ui@2834bc158a4849f21c97cbb2e0b10525f6e646f1`，提交时间 2026-10-15 02:02:11 UTC。逐文件读取范围、Git blob ID 和永久链接见 `research/open-claude-ui.lock.json`；对齐与偏离决定见 `docs/ALIGNMENT.md`。它是社区实现与作者逆向报告，不是 Anthropic 官方设计真源；本次未构建参考仓库、未审计官方二进制、未进行运行时像素对照。

**冲突裁决：**参考配色和几何以锁定提交的 CSS / 组件常量为准，文档只补足设计意图；第8篇与 CSS、动效源文件的冲突保留记录。该规则只选择外部参考，并不把外部代码升级为 Tekes 设计真源。无阴影、原生输入、四主题、对比度和三层验收优先保留。

**v0.3重点参考：**Claude Design官方文章/帮助文档，以及Figma关于语义变量、扩展集合、组件代码映射与Agent修复流程的文章。具体采纳与边界见 `docs/REFERENCE-POSTS.md`；来源日期与读取范围见 `research/claude-figma.sources.json`。新增的是多品牌与工具适配契约，不是把这些工具或帖子改成设计真源。

<a id="s1"></a>
## 1. 定位与原则

**探索可以非线性，发布不能无依据。** 可以从真实布局、原型或视觉细节起步，也可以在交接后继续改进；这些变化必须回写批准设计并重新生成。规范和流程产物不是完成证据，实际任务体验才是验收对象。此条受 Jenny Wen 的公开文章启发，具体边界由 Tekes 决定；来源与人物职责核实见 `docs/JENNY-WEN-POSTS.md`。[JW-01–03；REVIEW-01]

系统分为三层：**Foundation → Generic Components → Consumer Recipes**。前两层属于 TekesUI；Agent Workbench 只是第一个使用规范，项目、会话、模型、权限、审批和费用等业务模型留在消费者。CAD 与其他专业工具可以使用同一基础系统，不被聊天应用布局绑死。

设计取舍顺序：输入/可访问性正确性 > 内容与任务层级 > 状态一致性 > 视觉装饰。默认 normal 密度，列表可以 compact，长文不能跟随密度缩小字号。正文、错误、权限和验证结论不能藏在 hover 中。

**既定边界不变：**不使用阴影；不使用系统焦点环；所有可点击元素有 hover，输入框有 focus；用 ground/foreground/outline/marker 四通道组合状态。普通焦点轮廓为中性 `stroke-rule=1pt`，不引起布局位移，不改变色相；强底组件使用专用中性反差角色，不能直接套低对比度普通轮廓。[R4]

强底主按钮的焦点线使用 `c-button-primary-focus-outline`，线宽为 `stroke-rule`，画在按钮内部，外侧留一个 `stroke-rule` 宽的强底，使线两侧均与按钮强底相邻。该定位不移动按钮或改变布局；内部线对强底检查≥3:1。不是拿“只对内部可见”的颜色去假冒同时贴着外部浅底的外边框。

mac 平台层负责窗口、菜单、事件泵、剪贴板、输入法协议、系统惯性阶段和辅助技术挂接；CoreText 负责 mac 排版/回退/光栅化，Rust 层负责编辑状态机、文档、布局、绘制与可访问树构造。不要因追求一致外观重新实现系统候选窗或模拟 mac 惯性；也不能因使用系统字体就声称已经接入输入法或 VoiceOver。沿用章程的独立平台边界，不引入 GPUI 或用 winit 替换 mac 输入层。Windows、Linux 是后续平台，目前不声明已支持。[R2]

<a id="s2"></a>
## 2. 唯一设计真源与导入协议

**作者真源：现有私有 `TekesUI Design System` artifact。发布真源：一次批准并冻结的设计发布包 manifest。** 后者原子绑定 artifact 原始导出、motion 与布局补充契约、schema、资产索引和规范版本；补充文件是同一设计发布的组成部分，不是消费者代码里的第二套常量。

v0.3修订候选导入链：冻结设计导出映射到 `design/tokens/base.tokens.json`，再与批准的BrandConfig和版本化policy编译生成 `design/tokens/tokens.json`；保留原有发布消费路径。schema 放在 `design/tokens/tokens.schema.json`，motion 独立为 `design/tokens/motion.toml`，原始导出归档到 `design/handoffs/`，生成 Rust 到 `crates/tekes-ui-tokens/src/generated.rs`。本包的 `type` 内部字段是候选 profile；真实导出未提供，**只保证符合已公开的 family 外壳，不能声称已通过实际 artifact round-trip**。[R3]

同步只走：**设计/品牌配置改动 → 对应负责人评审 → 冻结输入 → 品牌编译与校验 → 原子导入 → 类型化生成 → 消费者验收**。生成物禁止手改。当前仓库允许手改 tokens；为避免后续导出覆盖，本提案新增变更 DS-001：手改只能形成待回写的差异补丁，不能静默晋升为第二个真源，须回到 artifact 采纳并重新导出后才能发布。这个流程收紧应作为显式 RFC 审批，而非悄悄改约定。

每次导入 receipt 还须绑定brand_id/brand_version、原始与规范化配置hash、base/policy/品牌schema/编译器hash及theme_contract_sha256；同一seed在新base或policy下不能复用旧缓存。每次导入 receipt 至少记录：design version、artifact revision、原始导出 SHA-256、规范化输入 SHA-256、schema 版本与 SHA-256、generator 版本与 SHA-256、源仓库基线、导入者、批准者、生成文件哈希与消费者 commit。私有工作区 URL/ID 不进入公开仓库；私有来源记录可独立保存，以可公开的版本及摘要绑定。

哈希不能证明来源；本包 `bundle_source_sha256` 只校验本包字节，**不等于私有 artifact 的导出哈希**。`source_revision` 和 `source_export_sha256` 当前为 null，发布门禁必须失败。

新鲜度有两层：一是重生成与已提交代码一致；二是消费版本是否落后于独立维护的 `approved-design-head`。仅比较本地 tokens 与 generated.rs 发现不了尚未导入的新设计。无法访问最新批准设计版本时，报告 `upstream-freshness=unverified`，不能报最新。导入须先校验全部文件，再以一个事务/提交替换；失败不做部分导入。

<a id="s3"></a>
## 3. Token 体系与类型约束

三层命名：`p-*` 是基础值；`primary-text`、`space-regular` 是语义角色；`c-button-quiet-hover` 是组件专用别名。只允许 **组件 → 语义 → 基础** 或同层无环别名。业务页面只能使用语义/组件 token，不能直接取基础色，更不能复制解析后的数值。

品牌派生由构建期编译器集中完成：seed → 受约束OKLCH色阶 → 四主题语义色对 → 组件别名。输出为显式sRGB；不得在组件内计算深浅变体。品牌模块详见 `docs/BRANDING.md`。颜色全部为显式 sRGB；长度为 logical pt（mac 映射 pt，其他平台映射逻辑单位），与 backing scale 分开；透明度是 0–1 比例，时长是秒，层级是整数顺序。单位、层级、owner role、状态、since 和来源在 `token-metadata.json` 中登记。数值去重不意味着合并语义名：`size-inline-action` 和 `size-compact-row` 同为 24，但不是同一个概念。

| 族 | 决定 |
|---|---|
| 间距 | 0、2、4、6、8、10、12、14、16、20、24、32、48；10/14/20 为具名组件内边距，不开放任意数值；2pt 基栅格 |
| 圆角 | square=0、marker=3、badge=4、row/XS=6、control/input=8、panel=10、code/popover/LG=12、dialog=16、message=18、composer=20、pill=999（按高度钳制）；保留 column=18 |
| 描边 | hairline=.5 仅为兼容名；装饰发丝线由平台按 1/backingScale logical pt 解析；rule=1、emphasis=2；必要控件边界、焦点与 IC 不用发丝线替代 |
| 透明度 | faint=.07、soft=.14、wash=.45；IC 的 faint 使用 soft；文本 IC 选区最终 alpha=.75 |
| 尺寸 | 按钮 XS/SM/MD/LG 高24/28/32/40；图标12/16，默认命中32、显式compact28、行内24；列表仍24/32/36 |
| 动效 | 时长0/.07/.12/.15/.3/.5秒；标准曲线(0.16,1,0.3,1)；另有四套具名 spring，减少动态直接到终态；不缩放文字/按钮、不做逐字 blur |
| 层级 | content 0、sticky 10、dropdown 20、popover 30、scrim 40、modal 50、modal 内弹层 60、tooltip 70 |

圆角不是随手 `min/max`：只有 pill 变体的胶囊半径取 `min(radius-pill, height/2)`；普通按钮采用 XS/SM/MD/LG 的6/8/8/12，不再用胶囊半径；声明为同心的嵌套形状取 `max(radius-badge, outer-radius − inset)`，再钳制到自身半短边。非同心形状不能套该公式。子组件缩小时不得产生负半径。

层级由 overlay manager 分配语义层，不让业务写任意 z-index。系统菜单、IME 候选窗和操作系统对话框的堆叠属于平台，不受引擎层级整数控制。禁止阴影、背景毛玻璃和正文透明度叠加。

JSON Schema 校验形状、字段、数值类型与四主题键。**按 name 去重、别名存在性/类型/无环、元数据完整性、最终合成对比度必须由程序校验；不能说标准 JSON Schema 已自动证明引用图正确。** 本包校验器已实现这些候选输入检查，未实现业务 Rust 的 AST 硬编码扫描。

<a id="s4"></a>
## 4. 多品牌 × 四套完整主题

主题 ID 固定为 `light / dark / light-ic / dark-ic`。IC 是完整的增强对比度语义映射，不是提高一层全局 opacity。正文、辅助文字、边框、选中态、浮层、代码、diff 和图表均有四主题解析结果，详见 `docs/PALETTE.md` 颜色表及 `reports/resolved-colors.json`。

颜色表按显式品牌生成，不再在主规范中把陶土值写成系统身份。`docs/PALETTE.md` 是 `example-neutral` 参考夹具的完整色表；`docs/BRAND-EXAMPLES.md` 展示neutral/blue/clay三组四主题结果。基础表面与非品牌语义沿用v0.2；品牌seed不是默认前景色。

普通文字目标 ≥4.5:1；IC 的所有可读正文/辅助文字目标 ≥7:1；必要控件边界与状态标记 ≥3:1。disabled 例外不扩张到错误说明和禁用原因；装饰线允许较低对比，但不能承担唯一分组/操作含义。这里采用 WCAG 数值作为 native 设计基线，不据此宣布整个原生应用 WCAG 合规。[R10–R12]

配置拆成两个独立轴：`appearance=system|light|dark`、`contrast=system|standard|increased`，默认均 system。手动主题不隐式关闭系统增强对比度；只有用户明确选择 standard 才覆盖 contrast，并显示当前覆盖状态。系统变更触发同帧原子切换所有控件、浮层、代码/ANSI 和图表资源，不闪回另一个主题。

**品牌是独立配置轴。**消费者必须提供 `brand_id/version/seed_srgb`；装饰 `brand-decoration` 保留原seed，链接 `accent*`、主操作 `action-primary-*`、弱底与checked/选中标记统一派生。强底前景与其焦点是配对中性色，不必跟着品牌色相变化。旧 `brand-clay` 仅为废弃别名，不再保证陶土值。

品牌变更不影响错误/警告/成功、中性表面、正文、普通焦点、文本选区、代码、diff或图表。通用列表默认中性选中底＋品牌标记；显式brand-subtle样式才使用品牌弱底。配置覆盖必须提供完整四主题值和理由并通过原有对比门禁。错误输入拒绝，不静默回退陶土；运行时保留最后已验证主题。跨窗口/浮层的原子切换、输入法与选择保持仍需真实实现验证。

**声明与证据：**本包附构建期品牌编译器和三组配置；不是平台运行时。Figma/Claude Design只是可选受控视图，映射规划与Agent交接见 `docs/DESIGN-TOOL-ADAPTERS.md`；没有执行在线同步或导入。

`hover-wash` 只作为叠色源；最终 alpha 一次性指定，不与父 opacity 相乘。固定 UI 底上的叠色先按本包 sRGB source-over 配方预合成，再传入 scene；原生渲染器要核对最终像素，不能把 GPU 线性混色的不同结果当作同一证据。被选中的代码/链接/diff 文本统一使用 `selected-text`，避免语法色在选区上失去对比。

<a id="s5"></a>
## 5. 文字系统

使用平台字体服务，**不分发系统字体文件**。mac 的 UI 使用系统 UI 字体与 CoreText 语言回退，中文/日文按系统语言解析；代码使用系统等宽字体。Windows、Linux 的字体回退由其平台适配器定义并记录实际解析结果，不能要求不同平台像素一致。固定截图环境必须记录 OS、字体 PostScript 名/版本、语言、字体大小、backing scale 和用户字号倍率。[R2、R13]

| 文本样式 | 大小/行高 | 字重 | 颜色与用途 |
|---|---|---|---|
| ui-body | 13/16 | 400 | primary；界面正文 |
| ui-label | 13/16 | 500 | primary；字段和控件标签 |
| ui-secondary | 12/16 | 400 | secondary；短说明 |
| ui-caption | 11/16 | 400 | tertiary；仅非关键时间戳，不能承载权限/错误结论 |
| ui-heading | 16/22 | 600 | primary；页内小标题 |
| code | 12/18 | 400 | mono；代码、路径、日志 |
| doc-body | 14/22 | 400 | primary；阅读正文，独立于 chrome 密度 |
| doc-h1 / h2 / h3 | 24/32、20/28、16/24 | 600 | primary；文档结构 |

前六个是候选基础样式，后四个是文档 profile 扩展；追加 chat-body/composer-body 为14/20，四档 button profile 为12/16、12/16、14/20、15/20，共16种样式。旧 `TextRole` 的精确 case 映射须根据真实导出补齐，不假定名字相同即语义相同。

基线由字体 ascent/descent/leading 计算，同行不同语言、emoji 与图标按文字基线光学校准，不靠随意 `offset(y:)` 修正。若回退字体超过行框，扩大行高而非裁掉顶部/底部。中英混排不修改原文加空格；界面自己的格式化文本可依据本地化模板处理单位。数字列用 tabular figures，单位保留不换行关系。

普通段落自动换行；长路径优先保留末段并中间截断，完整值可焦点访问与复制；任务标题最多两行，关键审批正文和错误不得仅 ellipsis。代码默认不软换行并在代码区横向滚动，提供显式 wrap 开关。截断按 grapheme cluster，不能切开 surrogate pair、combining mark 或 ZWJ emoji。

选择保留字/词/行/段的系统习惯。跨节点复制提供纯文本，适用时另提供 HTML；代码复制原始代码，不夹带行号、diff 装饰或 UI 标签；不静默 NFC 归一化。RTL 文本、组合字符、中文禁则、日语组字、emoji、1x/2x 与字号100/125/150/200%使用固定公开夹具；`fixtures.json` 的文本是测试输入，不是已通过结果。

<a id="s6"></a>
## 6. 布局、密度与缩放

通用布局提供 Stack/Grid/SplitView/ScrollArea；产品布局在 recipe 中选用。固定布局只用于截图夹具、明确的 canvas 或有约束的控件内部；运行时页面使用 intrinsic size、min/max 与优先级适配，不给长文本写死高度。

Agent Workbench 的默认窗口 1360×860 logical pt，最小 720×480。≥1200 采用侧栏＋主内容＋检查器；960–1199 默认隐藏检查器，由按钮弹出；<960 默认单主面板，导航与检查器成为可切换面板。随着字号增大，实际最小内容约束可以提前触发收拢，不能死守这些断点导致裁切。

侧栏默认288、范围224–360；检查器默认320、范围280–420；长文阅读列最大768；代码/diff/表格可使用整个主面板。这是 Tekes 的明确布局选择，不是声称参考仓库所有页面都有相同尺寸。Composer 整体最小72，常规编辑区44–240；整体预算上限384，附件＋工具栏计入预算且发送动作始终可达。显式扩展允许编辑区最多384，只有窗口剩余空间足够时启用；不能把编辑区上限误当作完整外壳高度。200%文字时允许重排，而非裁切命中区。

Compact 行高24、normal32、comfortable36，均为最小值；字体增大后行高随内容增长。密度不改变字号、状态语义和快捷键。图标视觉12/16与命中区24/28/32分开；相邻命中区不重叠。控制栅格用于 chrome，文字基线使用字体真实度量，不强行对齐2pt栅格。

窗口缩放、UI字号倍率、backing scale 是不同概念：字号更改要重排，不能整体 bitmap 放大。像素描边在绘制末端对齐设备像素，但禁止因逐 glyph 四舍五入造成累积字距误差。空内容、8倍长标题、10,000行增长、四种字号倍率和窄窗口都属于必测布局夹具。

<a id="s7"></a>
## 7. 图标与资产

统一单色线性风格，24×24 viewBox，母版描边1.5，圆头与圆连接；标准显示16，辅助12，24用于 Gallery 检查。几何缩放后对应约1pt和.75pt描边；小尺寸若辨识不佳，应提供经审查的光学修正版，不允许每个消费者私自加粗或偏移。状态变化优先改变颜色/底/标记，而不是从别的图标集换一张近似图。

图标目录以 kebab-case 槽位登记；每个槽位只有一个批准母版，可登记光学变体。记录名称、文件 SHA-256、作者/来源、许可、修改说明、12/16可读性和批准记录；公开 MIT 仓库只能收录经许可核验可公开的资产。品牌/provider logo、HUD 与 Claude 品牌资产不进入通用图标集。[R3、R4]

既有任务书记录的20个待重画项目写入本包 `asset-worklist.json`，**未附任何占位SVG，也未声称完成了20个原创图标或现有133个资产的版权核验**。settings 必须读成齿轮，不能退化为准星。权限图标配可读标签，不能仅靠形状传递 read-only / workspace-write / full-access。[R4]

导入拒绝外部 URL、script、foreignObject、滤镜、渐变、文字及未批准颜色；黑色统一规范化为 currentColor。保留原始导出和来源证明；剥除 metadata 只做文件规范化，**不构成原创性证明，也不能消除许可义务**。命名碰撞、缺失槽位、无来源资产都让正式导入失败。

<a id="s8"></a>
## 8. 组件目录与 API 边界

`design/registry/components.json` 是40个候选通用组件的唯一登记。每项都有稳定 ID、名称、分类、负责人角色、拟归属 crate、状态、规范路径、设计级 API、用法、禁区、状态矩阵、主题、夹具和证据字段。`owner_handle` 未获指派时留空；planned 允许，beta/stable 不允许。

| 分组 | 目录 |
|---|---|
| 基础/布局 | Text、Icon、Separator、Stack、Grid、SplitView、ScrollArea |
| 操作/输入 | Button、IconButton、Link、Checkbox、RadioGroup、Switch、TextField、TextArea、SearchField、ComboBox、Slider、SegmentedControl |
| 导航 | Tabs、List、Tree、Breadcrumb、CommandPalette |
| 浮层/反馈 | Menu、Tooltip、Popover、Dialog、Toast、Banner、StatusBadge、Progress、EmptyState、Disclosure |
| 文档/数据 | SelectionSurface、MarkdownView、CodeBlock、DiffView、DataTable、Chart |

沿用 text/doc/edit/layout/scene 等已有分层；**建议新增** `tekes-ui-controls` 承载通用控件，依赖 tokens/layout/text/scene/a11y，不依赖消费者或任何 agent runtime。这是架构增量提案，不声称该 crate 现在存在。Gallery 只依赖公开API，不能通过私有路径制造展示专用实现。[R2]

API 采用受控值＋语义事件：`Checkbox(value, on_change)`、`TextField(value, validation, on_edit)`。`selected`、`checked`、`busy`、`enabled` 是不同输入，不合并成含混的 `active`。Style 参数只能接收枚举/类型化token；variant不是任意 CSS/closure 绘制后门。通过 crate 的显式 export manifest 与 registry 双向核对，不能仅 grep `pub` 判断所有 UI 组件。

品牌不是组件variant：不创建blue-button/clay-button或按主题复制组件。所有组件目录、状态、recipe仍共享同一套稳定ID。Agent 特有 Composer、Message、ThinkingSummary、Attachment、ToolDecision、ModelUsage、SessionRow、ArtifactPreview，以及可选 Popout/TaskBoard 共10项使用规范登记在 `design/registry/recipes.json`。它们留在消费层，以通用组件组合；接受同样的主题、状态、行为和证据门禁，不在通用 crate 内创建模型/网络依赖。Thinking 只展示服务公开给用户的执行摘要，不伪造或索取模型隐藏推理。

<a id="s9"></a>
## 9. 完整状态与组合规则

**生成内容可以不确定，控件行为不能含糊。** 对相同输入事件、业务状态和配置，UI reducer 必须产生确定状态；模型文案、工具输出和任务路径通过空内容、增长、延迟、中断、畸形富文本和拒绝审批等夹具覆盖，不宣称穷举全部回答。`design/quality/craft-review.json` 记录必须真实执行的体验检查。[JW-05；本地规范化决定]

**click 是事件，不是状态；hover、selected、checked、focus 不能互相替代。** 状态 reducer 同时产生四通道：ground 表面、foreground 内容、outline 轮廓、marker 持久标识。

| 条件 | ground | foreground | outline | marker |
|---|---|---|---|---|
| rest | 组件默认底 | 默认语义前景 | 必要边界 | 值状态标记 |
| hover | button-hover 或 hover-wash 配方 | 通常不变 | 不伪装focus | 保留selected/checked |
| pressed | button-pressed | 保持可读 | 不发生几何位移 | 保留值标记 |
| selected | row-selection | primary | 普通边界 | 可见侧条/标志，配无障碍selected |
| selected+hover | row-selection-hover | primary | 普通边界 | 持久标记仍可辨认 |
| checked | 控件专属底 | 对应前景 | 控件边界 | 勾选/混合线，不使用选中侧条替代 |
| focus | 保留底状态 | 保留前景 | 中性1pt focus | 不清除selected/checked |
| error+focus | 保留编辑底 | 正常文字＋错误说明 | focus保持中性 | error图标/标记持续存在 |
| disabled | 静止底；抑制hover/pressed | disabled-text | 静止边界 | 原checked/selected仍保留 |
| loading | 不改变既有宽度 | 保留标签，可加进度 | 已有焦点不丢失 | busy标记；不能重复提交 |

强底主按钮焦点使用 `c-button-primary-focus-outline`；这仍是中性轮廓，并与强底比较，不拿普通浅底的验证结果冒充。减少动态时用静态“处理中”状态，不用缩短旋转来代替关闭动画。

pointer up 在组件外、捕获丢失、窗口失活、取消手势都必须清掉 pressed。操作中被禁用时明确迁移焦点，不留不可见焦点。loading 不等同 disabled；阻止重复动作但保留焦点，可取消动作由独立按钮承载。read-only 仍可聚焦、选择和复制，不是 disabled。

每组件登记全部适用状态及不适用理由。只读 Text/Icon/StatusBadge 不能为了凑表增加 hover；容器的交互子组件单独验收。除了单状态四主题，至少覆盖 selected+hover、selected+focus、checked+disabled、error+focus、loading+focus、pressed+focus 和输入法组合；非法状态用 reducer 性质测试拒绝。

<a id="s10"></a>
## 10. 交互契约

指针在释放且仍满足激活条件时触发动作，取消按下不触发。单击选择不自动等于打开文件；双击、菜单和回车的含义由组件/消费者公开。拖放必须提供按钮或菜单的单指针替代，也必须有键盘替代；键盘可用并不自动满足“不拖动”要求。[R14]

Tab/Shift+Tab 按可解释的结构移动；列表、树、tabs 与菜单内部使用方向键与 roving focus。焦点与选中值分离。模态 Dialog 约束焦点并在关闭后还给触发点；非模态 Popover 不默认陷住焦点。若原触发点已删除，回到最近有效上下文，不跳到页面首项。Tooltip 可被 Esc 关闭、可持续观察，不会因指针移向说明立即消失。[R15]

通用 TextArea 的 Enter 换行。Agent Composer recipe：Enter发送、Shift+Enter换行、Cmd+Enter显式发送；已有产品设置优先保留，视觉迁移不改既有发送偏好。**有 marked text 或该事件已被 IME 消费时，所有发送快捷键都不得提交业务动作**。先交给输入法再评估应用命令，不靠仅检查某个 keyCode 猜测组字状态。

mac 的 Cmd+C/X/V、Cmd+Z、Shift+Cmd+Z、Cmd+A 服从当前编辑焦点；全局命令不能抢编辑器已有快捷键。命令板建议Cmd+K，窗口内查找Cmd+F，具体冲突经 command registry 检查。Windows 按平台规则映射，不简单对每个 Cmd 替换 Ctrl。

输入法适配完整保留 UTF-16 范围、grapheme 映射、replacementRange、marked text 分节、组字光标与候选窗矩形；滚动/缩放后更新候选锚点。同一轮组字提交作为一个undo单元；自动保存不清空撤销栈；流式 AI 内容不能插入用户正在编辑的 undo 序列，合并需显式事务。

滚动遵循系统 momentum phase。图钉容差40pt、用户输入保护150ms、跳至最新提示阈值120pt；这些是可验证的初始参数，不是替代状态机。用户上滚意图立即锁存 reading-history，即使仍在40pt容差内也不能被动重新吸底；冷却结束不解除锁存。只有显式跳至最新或用户主动回到底部才恢复跟随。上滚查看时保持内容锚点并显示“有新内容”。重新排版和虚拟化回收不能破坏选择范围、caret 或稳定文档 ID。滚动条、选择拖拽、滚轮和触控板分别测试。

分割拖动至少移动4pt才进入拖动态；Esc、丢失捕获和窗口失活均恢复起始比例。先判断内容最小宽度可行，再计算 clamp；不可行时改为可切换面板，禁止产生 minRatio > maxRatio。双击均分仍服从最小尺寸。指针跟手不经过弹簧，只有释放后的有界吸附可使用 spring。规范见 `design/interaction-contracts.json`。

发送与重试由消费者保证：不自动trim原文，提交前保存不可变草稿/待发送记录，只有匹配草稿获得持久接受确认后才清除编辑内容。此契约不授权视觉迁移去改已有发送偏好、后端业务协议或权限语义；有差异时单独做行为迁移评审。

<a id="s11"></a>
## 11. 内容、反馈与工作台 recipe

**渐进披露不等于隐藏风险。** 可以收起底层技术细节，但权限范围、计划批准、取消入口、错误和证据状态必须持续可达。品牌表现优先服务识别、层级和反馈，不把产品愉悦感简化为到处上色或播放动画。[JW-05、08；本地取舍]

文案表达“发生了什么、影响什么、下一步是什么”。按钮用动词＋对象，标题表达对象而非泛泛“提示”。placeholder 提供例子，不代替标签。错误示例：“未能保存更改。连接已断开；内容仍保留在编辑器中。”随后提供“重试”，而不是只显示“失败”。

| 状态 | 表达规则 |
|---|---|
| 首次空状态 | “还没有会话”＋一个主要入口；不制造示例会话冒充真实数据 |
| 筛选无结果 | “没有符合条件的结果”＋清除筛选；不同于数据加载失败 |
| 加载 | 有真实分母才显示百分比；否则显示不确定进度与正在做什么 |
| 成功 | 状态文字＋标记；普通成功可用短toast，重要产物保留可访问结果 |
| 失败 | 保留输入、失败范围、原因与恢复动作；必要详情可展开 |
| 权限受限 | 说明主体、资源、范围和授权方式；不与网络错误合并 |
| 未验证 | 独立中性状态“未验证”；不能显示绿色成功勾 |
| 取消/超时 | 与失败区分；说明部分结果是否保留、能否继续 |

Agent Workbench 采用**侧栏定位、中心连续文档、右侧按需检查器、底部输入/可选终端**。不是把所有信息装进卡片：普通助手解释用透明文档流；用户消息可用18pt圆角浅底，Composer20pt独立边界；执行调用默认是一行概要＋状态＋耗时，失败、待批准与证据缺失才提升显著性。工具详情显示输入、输出、来源与范围，但秘密信息先脱敏。

审批卡必须同时展示动作、作用资源/路径、权限范围与潜在影响；“允许一次/记住规则”等不同权限语义不能仅靠按钮颜色区分。破坏性授权不是默认回车目标。最终回答只代表模型停止本轮，不自动把任务标成验证成功；证据状态由消费者的验证结果输入。

扩展仅通过登记的 sidebar item、toolbar action、inspector section 等插槽接入，同样遵守组件、token、主题、a11y与焦点规则。扩展 UI 不能覆盖系统审批层、截获 IME 或通过自定义渲染绕过产品行为检查。此规范吸收 Mods 的可定制方向，但不为 TekesUI 定义插件执行权限模型。[R7]

<a id="s12"></a>
## 12. 无障碍与平台验收

目标是每个公开组件具有正确的角色、可读名称、值/状态、关系、焦点和动作映射。通过 tekes-ui-a11y 构造语义树并由平台挂接；不是给自绘矩形写一个名字就算完成。按钮名称包含可见标签；图标按钮有独立可访问名称；Checkbox 传达 checked/mixed；输入框关联 label/help/error；表格提供表头和行列关系；树提供层级和展开状态；对话框有标题与正确模态性。[R2、R10]

文字200%时关键内容和操作仍可达；错误/权限/图表不只依赖颜色。图表同时提供数据表、图例、系列线型与点型；复杂图像有可访问说明。流式内容不逐 token 打断朗读，不重复播报整段；完成、阻断和新错误以合理频率通知。用户可以关闭非必要通知。

减少动态设置：状态切换为硬切；关闭打字机、shimmer、旋转spinner和大面积展开动画。进度以静态图标＋文字更新表达；滚动输入保留系统行为。增强对比度与减少动态相互独立，四主题都要与减少动态组合测试。

本案采用普通文本4.5:1、IC文本7:1、必要非文字信息3:1作为量化基线。**1pt焦点轮廓不被宣称满足 WCAG 2.4.13 的 AAA 面积要求，IC 也不等于整体AAA**；焦点可见和不被遮挡仍需真实操作验收。[R10–R12]

自动检查语义树、标签关系、焦点顺序、对比与几何；真实 mac 使用 VoiceOver 和 Accessibility Inspector，完成逐字/词/行/段导航、选区同步、编辑朗读、表格导航与模态焦点测试。原生应用不能用浏览器 axe 扫描代替这些证据。系统拼音、搜狗、微信输入法、日本语、听写和重新转换按既定矩阵逐项验收；未接入/无可用环境时标 `unverified`，不降低标准。[R5]

<a id="s13"></a>
## 13. Gallery：唯一的组件目测入口

Gallery 是 **tekes-ui-gallery 原生 Rust 应用**，实际调用公开组件和同一 token 解析器。本文、生成的Markdown页面和颜色表都不是这个Gallery。没有真实实例的登记只能显示planned与缺失原因，不能放一张“看起来完成”的截图作为替代。[R1、R2]

固定页模板：组件名称/ID/状态 → 用法与禁区 → 可操作实例 → 完整状态矩阵 → 显式品牌×四主题对比 → 宽度/字号/语言夹具 → token依赖 → a11y树 → 三层证据。Gallery导航、搜索、按钮、标题、空状态和间距也由同一组件系统组成；不豁免自己的设计规范。

registry 生成目录、组件文档、导航与快照期望清单。可按名称、ID、分类、状态、crate搜索；稳定路径形如 `tekes-gallery://component/checkbox?brand=example-blue&theme=dark-ic&fixture=cjk&state=checked`。深链载入白名单枚举，不执行任意参数内容。改名保留slug和重定向；废弃页保留迁移说明。

真实状态通过共享 reducer/事件注入进入，不能由 Gallery 独立伪造 `hover=true` 的专用样式。固定夹具时钟、随机种子、加载阶段、滚动位置、光标与选择范围；交互实例可自由操作，截图实例另有确定性模式。鼠标悬停和真实键盘路径另外验证，状态注入截图不能代替输入路径。

本包生成40页组件设计级文档、10页消费者使用规范及各自最低期望样例清单；所有运行时状态仍为unverified。本包期望清单同时覆盖neutral/blue/clay三组，不是假运行结果。以后导出的 screenshot manifest 必须同时包含品牌ID/版本/契约hash、主题、状态/组合、fixture、字号倍率、窗口尺寸、backing scale、OS/字体版本、组件/设计/代码hash和实际图片摘要。无图片文件的记录不算快照。

<a id="s14"></a>
## 14. 自动质量门禁

门禁结果严格区分 pass/fail/unverified/not-applicable；N/A须有组件级理由。required的unverified会阻止相应阶段发布。禁止以“测试成功启动”“截图文件存在”或“编译成功”代替语义通过。

| Gate | 校验与失败条件 | 本包状态 |
|---|---|---|
| token-structure | schema、类型、范围、重复名、四主题、单位、引用图、分层 | 已有可运行校验 |
| token-contrast | 登记的前景/背景与选区/hover合成，阈值比较前不四舍五入 | 已有可运行数值检查；最终像素未验证 |
| source-integrity | 本包文件摘要和聚合摘要；原始artifact来源另核验 | 本包完整性可检；上游来源未验证 |
| generated-freshness | 重新生成无diff；schema/generator变化也触发 | 组件文档可检；Rust生成器未实现 |
| component-coverage | 导出UI符号↔registry↔真实Gallery工厂双向一致 | 登记表可检；代码和工厂未验证 |
| theme-state-coverage | 适用状态、风险组合与四主题都有运行时证据 | 声明完整性可检；运行时未验证 |
| orphan-pages-assets | 未登记页面、缺页、缺图标、重复ID/slug | 生成文档可检；真实资产未验证 |
| hardcoding | Rust AST 中绕过typed token的颜色/尺寸/style；声明值源文件白名单 | 规范已定义；扫描器未实现 |
| layout | 文本/焦点/控件不可见裁切、命中区重叠、布局溢出 | 需原生布局探针，未验证 |
| snapshots | 期望矩阵与实际manifest集合一致；无多余/缺失截图 | 期望清单已生成；无运行时截图 |
| interactions-a11y | reducer性质、事件回放、AX树、真实IME/VO记录 | 未验证 |
| brand-and-context | 品牌schema、派生、独立语义不泄漏、三品牌生成物/上下文/映射规划新鲜度 | 已有静态检查；原生与在线适配未验证 |
| consumer-behavior-performance | 页面契约、回调payload、审批语义、性能/内存预算 | 未验证 |

CI建议命令：`cargo xtask design validate`、`design generate --check`、`gallery audit`、`ui-test --manifest …`；**这些是待实现的仓库命令，不是本次已执行的命令**。本包现在能运行的是 `python tools/validate.py`、`--self-test`、`--release`。v0.2追加recipe schema、类型化组件token引用、来源锁定、输入/滚动契约、recipe文档新鲜度与16项规范文件存在性检查；这些仍是静态声明校验，不能证明操作实际发生。v0.3另提供 `tools/brand.py`、`tools/test_brand.py`、`tools/context.py --check`；`generate.py`统一生成品牌、目录与交接视图。AI只可按明确迁移表提出机械修复，歧义保留待评审，不自动覆盖共享设计。

例外文件必须有规则ID、精确文件/组件/主题范围、具体理由、关联问题、批准者与到期日期；不能整目录豁免，更不能静默忽略来源缺失或把unverified改为pass。到期例外导致失败。视觉基线更新必须人工审查热图/几何报告，不能一键全量接受后宣布修复。

<a id="s15"></a>
## 15. 实际应用验收与预算

额外执行 `design/quality/craft-review.json` 的体验审查：原型与正式实现分离、真实任务走查、控制状态与生成内容场景、渐进披露、品牌反馈边界、原生行为回归及反馈来源。每项都要有证据与 reviewer；当前全部 unverified，正式发布阻断。它不取代以下性能预算和三层证据，也不是 Jenny Wen 的原始技术规范。

`pages.json` 定义六条消费者验收recipe：工作台导航、设置表单、富文本/diff审阅、审批弹层、执行与证据检查器、数据用量。每页覆盖消费者真实品牌及四主题，以及适用的空/加载/错误/权限/断连/内容增长；真实业务事件由消费者自身测试夹具输入，不嵌入通用组件库。

证据分三级：**L1组件预览**证明局部样式与契约；**L2真实应用页面**证明集成、焦点和业务行为没有回归；**L3真实环境**证明系统输入、辅助技术和性能。任一层的pass都不能代替另一层。单独保存artifact/设计/代码/机器配置hash；修复后生成新版本并重验，旧证据不自动继承。品牌配置、策略或基础表面变化也使有关证据失效。

| 指标 | 初始预算 / 判据 |
|---|---|
| 空闲CPU | 静止可见60秒，采样噪声之外接近0；阈值<0.1%；无持续redraw/空转timer |
| 120Hz滚动 | p99帧处理时间≤8.3ms且不劣于同机参照；连续掉帧失败 |
| 输入呈现 | p95不劣于参照＋1帧；绝对延迟先记录，F5实测后批准阈值 |
| glyph atlas | 不超过同夹具参照1.25倍；重复滚动无无界增长 |
| 资源硬上限候选 | atlas 128MiB、Gallery大型夹具进程RSS 512MiB；需F1实测批准，不是当前测量结果 |
| 冷启动 | 记录进程到首帧的中位/最差值；沿用现有对比方法，暂不假造通过阈值 |
| 流式增长 | 10,000行夹具，选区/内容锚点稳定；只布局可见区和有界overscan，后台页不持续刷新 |

基准使用Release构建，记录具体机器、OS、GPU后端、刷新率、字号、语言与电源状态，至少5次，报告中位数与最差值。网络/模型延迟不并入UI延迟；memory报告说明是否含子进程与缓存。[R5]

排版先于像素：断行位置一致，基线容差1x≤.5pt/2x≤.25pt，字形原点≤.25pt；随后使用同平台同字体的像素容差。沿用对比方法的起始阈值：单通道差≤24/255不计差异，超阈值比例≤.5%(1x)/.3%(2x)，整字形缺失/整行偏移直接失败，不准先自动平移把偏移抹掉。容差只吸收抗锯齿，不掩盖排版错误；阈值本身仍需F1按既定流程校准。[R5]

**本次证据状态：**候选输入的静态校验可复现；L1原生组件、L2应用页面、L3真实环境均未验证。v0.1读取的仓库基线只有文档；本次未重新审计其实现进度，也不能凭本包得出当前真实应用已经通过的结论。[R1]

<a id="s16"></a>
## 16. 版本、维护与交接

设计版本独立于crate版本，以发布receipt建立映射。token的语义、组件API或默认交互破坏性改变升major；兼容新增升minor；不改变契约的修正升patch。可见颜色/几何变动即使只是patch也要出视觉变更说明并更新审查过的基线。proposal/beta不能伪装成stable。

变更流程：问题/目标 → 影响token/组件/页面集合 → 设计源修改 → 对比/交互/a11y评审 → 冻结导出 → 原子导入＋生成 → 三层验收 → changelog与版本映射。每个token和组件可查询来源、since、deprecated、replacement及反向依赖；新增组件必须先入registry再实现。

废弃至少跨两个minor且不少于90天，满足后也只在major移除公共API；安全等例外要明确记录。保留旧ID/slug与迁移页，不默默复用被删除的名字。本proposal尚未建立稳定API，但从首次公开发布起执行该规则。

与当前仓库相比需要明确审批的增量：DS-001手改token回写规则；DS-002三层token元数据与发布receipt；DS-003候选四主题定值/字体profile；DS-004新增controls crate；DS-005 Gallery和实际页面证据门禁。旧27颜色角色保留迁移入口，但未取得的旧值/原始字体case/全量图标映射不猜测。v0.2 另增加 DS-006：采纳锁定的 open-claude-ui 参考，调整可见颜色/几何/字号与动效；全部旧token和组件ID保留，具体差异见 `design/migration-v0.1-to-v0.2.json`。这是提案版本内的显式可见破坏性调整，不能假装只是无影响的配色补丁。

v0.3增加DS-007消费者品牌与构建期派生、DS-008受控工具视图与Agent交接；迁移见 `design/migration-v0.2-to-v0.3.json`，取舍见 `docs/REFERENCE-POSTS.md`。两者仍为未批准提案；没有改变真实业务行为。

按通过标准分段落地，不承诺日期：先在F0建立registry/模板/证据结构，按既有F1–F2完成文本、token和资产基础；随后文档、编辑、a11y逐项到达对应路线阶段；最后做消费者迁移。每阶段都维护同一个Gallery，不能在终点才补真实输入与VoiceOver。

---

## 交付文件索引

`design/brand/`：品牌schema/policy、三组样例、生成bundle与receipt。`design/adapters/`：可选Figma映射规划与Agent上下文。`design/tokens/`：候选四主题token、schema、分层/单位元数据、motion。`design/layout.json`：布局/密度/层级契约。`design/registry/`：组件、状态、消费者页面。`design/quality/`：固定夹具、对比度配对、图标待办、门禁与发布条件。`docs/components/`：从registry生成的40个组件设计级规范及10项消费者recipe。`tools/`：校验器与文档/期望用例生成器。`reports/`：静态结果、对比度明细、失败路径测试和未验证Gallery用例。

本包不附字体、现有私有图标、Claude品牌资产、Rust运行时截图或无法证明来源的资产。发布缺口不能靠截图占位或把状态写成pass消除。

## 参考资料

来源索引、公开地址和读取范围见 `research/sources.json`；R编号是本规范内部引用，不是声称读取了被列出的私有artifact。v0.2 标准表面色与部分几何取自锁定参考文件；v0.3的多品牌和工具契约参考Claude Design/Figma官方资料，派生算法为Tekes决定。IC、可读色对、原生适配和性能预算为 Tekes 提案决定；逐项见 `docs/ALIGNMENT.md`。

### 本版追加的方法参考

Jenny Wen 的本人文章、X 原帖与访谈独立登记在 `research/jenny-wen.sources.json`；采纳映射见 `design/quality/reference-adoption.json`。她曾负责 Claude/Cowork 的设计，但本次没有证据证明她负责名为 Claude Design 的产品；本人已公告离开 Anthropic 并加入 Cursor。X 仅索引可见，访谈未观看录像，不将这些范围说成完整线程/录音审计。
