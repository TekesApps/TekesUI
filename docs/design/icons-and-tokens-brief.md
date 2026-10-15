# 委托包：TekesUI 图标集与 token 集（给 Design Agent）

状态：草案，**待用户决定是否发出**。数字统计于 2026-10-15。

> 更新：设计工作已转到 TekesUI Design System artifact（claude.ai 上的私有 Design System artifact，不在此处给链接）。本文作为历史记录与需求来源保留；交付格式以 [design/README.md](../../design/README.md) 为准。

本文分两部分：第一部分是给用户看的"值不值得"评估；第二部分起是发给另一个 Claude 会话（Design Agent）的委托正文，需自足。发出时把第二部分起的内容连同附件一起发。

---

## 第一部分：值不值得（供用户决定）

### 要做的事

TekesUI 是 MIT 公开的纯 Rust UI 组件库，其 Gallery 要长期稳定。现有的图标与 token 来自私有仓库（TekesMetalTextKit 的 Design 目录、Tekes 的 docs/design/icons/），其中一部分依赖 macOS 系统资源（SF Symbols、NSColor 系统语义色、系统 text style）。这些不能直接公开，也不能跨平台。需要一套 TekesUI 自有、跨平台、可随 MIT 公开的图标集和 token 集。

### 现状统计

图标：

- `IconRole` 共 148 个槽位。129 个已映射到可作参照的自绘 `tekes-*` 资源（引用 123 个不同资源）。资产目录 TekesIcons 下可交付的 imageset 为 133 个、399 个 SVG，全部 viewBox 0 0 24 24、stroke-width 1.5。
- **需要从零设计的共 20 个**：19 个 IconRole 槽位，外加 1 个非槽位资源。
  - (a) 15 个只有 SF-only 回退：listRemove、zoomIn、zoomOut、settings、openExternal、authorized、missedRun、diffAnnotation、fileLink、attach、gitAhead、gitBehind、settingsMarkdownFormat、cron、timezone。其中 settings 的自绘定稿在小尺寸下读成准星而被否决，临时回退 SF，等第二版。
  - (b) 5 个原稿已作废：`annotation`（资源 `tekes-annotation`）、`permissionReadOnly`（`tekes-perm-read-only`）、`permissionWorkspaceWrite`（`tekes-perm-workspace-write`）、`permissionFullAccess`（`tekes-perm-full-access`），以及非槽位资源 `tekes-computer-use`（Settings 的 Computer Use 页按名字直接加载）。原因：原稿含来源不明的 C2PA 内容出处元数据，已从 base 和交付范围移除。这 5 个由 Design 从零设计，不参照原稿，也不会收到原稿。
- 另有 SourceControl 下约 30 个文件类型图标（FileIconC 等），以及品牌/provider logo。**品牌 logo 是第三方商标，不得进 MIT 仓库，本委托明确排除。**
- 不一致项：设计页《Tekes Icon Set》文字写"2pt 圆头描边"，而 `Icon Tokens.md` 与实际 SVG 都是 1.5pt。需要 Design 统一说明：以实际 SVG（1.5pt）为准，还是重定。

颜色：`ColorRole` 共 27 个。

- 完全平台中立（定值）3 个：scrim（黑）、warningText（黑）、warningForeground（light #A03C00 / dark #FFA85C）。
- 半中立 7 个：rowSelection、buttonHover、transcriptInput、border、floatingPanelBorder、floatingPanelDivider、borderFocused。light 为产品校准的 sRGB 定值（如 buttonHover (0.90, 0.90, 0.90)、rowSelection (0.935, 0.941, 0.941)），dark 与 Increase Contrast 仍由系统色混合得出，需要给定值。
- 完全依赖系统语义色 17 个：primaryText / secondaryText / tertiaryText（label 系）、selectedText、documentCanvas（文本背景色）、textSelection（系统选区色，高对比下 alpha .75）、accent（系统强调色，用户可在系统设置里改）、statusRunning（systemOrange）、warning（systemYellow）、statusFailed（systemRed）、statusSucceeded（systemGreen）、diffAddedMarker / diffRemovedMarker（green / red），以及由系统色乘 alpha 得到的 hoverWash（label@faint）、separator（secondaryLabel@soft）、diffAddedSurface / diffRemovedSurface（green / red @faint）。
- 合计：27 个里有 24 个至少在某个外观下需要给出平台中立取值。
- 另需决定：accent 是固定品牌色，还是由平台层提供系统强调色并给出中立默认值；textSelection 同理。

本来就平台中立、可直接搬的：Alpha 三档（faint .07 / soft .14 / wash .45，Increase Contrast 下 faint 提到 soft）、Spacing 8 档、CornerRadius 9 档、Stroke 3 档、Motion 5 档、ControlSize 15 项。

文字：`TextRole` 6 个角色目前解析自 macOS 系统 text style（body 13、subheadline 11、caption 10，默认行高 16）。需要平台中立的字号/字重/行高表，并说明 mac 上是否仍跟随系统动态字号。

### 结论

- **工作量**：从零设计 20 个图标（15 个 SF 回退 + 5 个原稿作废）；统一笔法规范；约 24 个颜色角色在 dark / Increase Contrast 下给定值（外加 accent、textSelection 的策略决定）；文本角色表；对现有 129 个自绘图标逐个做原创声明；映射表。体量属于一轮到两轮设计委托，不是大工程；最耗时的是图标原创性核查。
- **收益**：Gallery 的视觉输入稳定且不依赖系统资源；跨平台（Windows、Linux）可用同一套值；可随 MIT 仓库公开；token 真源脱离 Swift，Rust 与 Swift 两边可以从同一文件生成。
- **风险**：与 Tekes 现有视觉漂移（两套 token 并存，需要映射表与回归比对来约束）；版权核查成本（现有图标是否有参照 SF 形、重画量未知）；dark / Increase Contrast 新定值要重新做对比度验证。
- **建议**：值得做，但分两步：先发"token 与笔法规范 + 20 个从零设计"，图标原创声明与重画作为同一委托的第二阶段；F1 阶段 Gallery 可先用临时占位 token，不阻塞组件开发。**是否发出、何时发出，由用户决定。**

---

## 第二部分：委托正文（发给 Design Agent）

> 使用方式：连同下列附件一起发给一个新的 Claude 会话：词汇表《_design-vocabulary》、Icon Tokens.md、Tekes Icon Set 设计页、token 索引。这些在私有仓库 Tekes 的 docs/design/ 下，本仓库不收录。

### 1. 背景与目标

TekesUI 是纯 Rust 的 UI 组件库，mac 优先，之后 Windows、Linux，MIT 许可，公开发布。它的 Gallery（组件展示应用）用固定夹具、固定布局展示每个组件的 rest / hover / pressed / disabled / focus 状态，覆盖 light / dark / increase contrast。

目标：交付一套 **TekesUI 自有、跨平台可用、可以随 MIT 仓库公开** 的图标集和 token 集，让 Gallery 长期稳定。

产出不是 Swift 代码，也不是对 Tekes 现有界面的重设计。是把现有已定案的设计词汇，整理成平台中立的真源，并补齐缺口。

### 2. 底稿（base）

- 图标：Tekes Icon Set（自绘 SVG，24pt 画布，1.5pt 描边，圆头圆角，单色）。资源命名 `tekes-*`。
- token：TekesMetalTextKit 的 Design 目录里的一组枚举：Spacing、ColorRole、TextRole、IconRole、CornerRadius、ControlSize、Stroke、Motion，及公共词汇表。
- 第 0 轮裁决（已定案，不要重新讨论，除非你有明确不成立的证据）：
  1. 不要阴影；"浮在内容之上"靠轮廓、底色、层级表达。
  2. 一切可点元素必须有 hover，输入框必须有 focus。
  3. 不使用系统焦点环；能获得焦点的控件自绘细焦点轮廓（`borderFocused`，1pt `rule` 线宽，零位移，不换色相）。
  4. 状态由四个正交通道叠加推导：底（ground）、前景（foreground）、轮廓（outline）、标记（marker）。
  5. 警告字用专门的前景角色，不把黄色当透明底上的小字。
  6. 品牌 logo、HUD 不在设计系统范围内。

### 3. 现状表

#### 3.1 平台中立、可直接搬的数值

Spacing（pt，2pt 栅格）：

| case | 值 |
|---|---|
| none | 0 |
| micro | 2 |
| tight | 4 |
| snug | 6 |
| regular | 8 |
| relaxed | 12 |
| loose | 16 |
| section | 24 |

CornerRadius（pt）：

| case | 值 | 说明 |
|---|---|---|
| square | 0 | 硬角 |
| agentMarker | 3 | 身份标记 |
| badge | 4 | 小标记；同心圆角推导的下限 |
| row | 6 | 列表行选中/hover 底 |
| control | 28 | 文本框、chip、搜索框（等于 pillHeight，即胶囊） |
| input | 16 | 输入卡与其叠放卡片 |
| panel | 12 | 卡片、分组面板 |
| column | 18 | 嵌入窗口列的卡片 |
| floating | 14 | 浮层、popover |

另有一条派生规则：子形状圆角 = 外圆角 − 内缩距离，下限 `badge`（同心）。

Stroke（pt）：hairline 0.5、rule 1、emphasis 2。

Motion（秒）：instant 0（减弱动态效果时所有档位退化为它，硬切而不是缩短淡入）、stateChange 0.12（颜色/透明度）、geometry 0.18（位置/尺寸）、listExpansion 0.3（紧凑列表展开）、expansion 0.5（较大面的展开）。

ControlSize（pt；刻意允许不同 case 同值，名字承载语义，不要合并）：

| case | 值 |
|---|---|
| focusBar | 2 |
| dot | 6 |
| agentMarkerWidth | 8 |
| agentMarkerHeight | 14 |
| inlineBadge | 16 |
| selectorTrack | 20 |
| inlineAction / compactRow / selectorKnob / actionButton | 24 |
| iconButton / pillHeight | 28 |
| rowHeight | 32 |
| truncationSlot | 180 |
| truncationSlotWide | 240 |

Alpha：faint .07、soft .14、wash .45。Increase Contrast 下 faint 提升为 soft 的值；高对比文本选区另用 .75。显式 alpha 表示最终 alpha，不相乘。

图标尺寸档：标准 16pt（toolbar、sidebar、菜单、行内操作）、辅助 12pt（关闭、勾选、展开）。点击区：普通按钮 28、行内操作 24、列表行至少 32；视觉尺寸与命中区分离。

#### 3.2 颜色角色（27 个）

按"平台依赖程度"分三组。要求你为第 2、3 组在 light / dark / increase contrast 三套下都给出明确值（sRGB，含需要时的 alpha）。

| 组 | 角色 | 现状 |
|---|---|---|
| 1 完全中立（3） | scrim | 黑 |
| | warningText | 黑（只落在 warning 面上） |
| | warningForeground | light #A03C00 / dark #FFA85C；对 documentCanvas、buttonHover、hoverWash@soft 三种底在三种外观下都 ≥ 4.5:1 |
| 2 半中立（7） | rowSelection | light 定值 (0.935, 0.941, 0.941)；dark、IC 待定 |
| | buttonHover | light 定值 (0.90, 0.90, 0.90)；dark、IC 待定 |
| | transcriptInput | light 定值；dark、IC 待定 |
| | border | light 定值；dark、IC 待定 |
| | floatingPanelBorder | 同上 |
| | floatingPanelDivider | 同上 |
| | borderFocused | 同上（比 border 更强的中性轮廓） |
| 3 系统依赖（17） | primaryText / secondaryText / tertiaryText | 当前取系统 label 色系 |
| | selectedText | 当前取系统"选中控件文本色" |
| | documentCanvas | 当前取系统文本背景色 |
| | textSelection | 当前取系统选区色；IC 下 alpha .75 |
| | accent | 当前取系统强调色，用户可改 |
| | statusRunning | 当前取系统橙 |
| | warning | 当前取系统黄 |
| | statusFailed / statusSucceeded | 当前取系统红 / 绿 |
| | diffAddedMarker / diffRemovedMarker | 系统绿 / 红 |
| | hoverWash | label 色 @faint |
| | separator | secondaryLabel 色 @soft |
| | diffAddedSurface / diffRemovedSurface | 绿 / 红 @faint |

需要你决定并说明：

- accent：固定品牌色，还是"平台层提供系统强调色 + 本包给出中立默认值"。若选后者，派生色（hover/pressed 叠加、实心控件的 wash）如何随任意强调色自动成立。
- textSelection：同上。
- 橙色原本承担三种身份（running / 审批 / warning），已拆为 statusRunning、warning、warningForeground；新定值不要再混用。
- 对比度要求：文字对其底 ≥ 4.5:1；非文字关键轮廓 ≥ 3:1；Increase Contrast 下整体更强。请给出你的验算表（角色 × 底 × 三种外观）。

#### 3.3 文字角色（6 个）

| 角色 | 当前来源（macOS） | 字重 | 实测字号 / 行高 |
|---|---|---|---|
| body | body | regular | 13 / 16 |
| bodyEmphasized | body | semibold | 13 / 16 |
| rowMeta | subheadline | regular | 11 / 16 |
| sectionHeader | subheadline | bold | 11 / 16 |
| identityBadge | caption1 | semibold | 10 / 16 |
| brand | title2 | bold | 待核实（随系统） |

等宽是修饰子（数字等宽、代码等宽），不是角色。需要你给：平台中立的字号 / 字重 / 行高 / 字距表；字体族的回退链（Rust 侧各平台默认系统 UI 字体）；mac 上是否仍跟随系统动态字号（建议明确写出：跟随，还是固定值 + 缩放系数）；中日韩与拉丁混排时行高是否固定。

#### 3.4 图标

- 画布 24×24，描边 1.5pt，圆头圆角，单色，着色走 ColorRole。资源命名 `tekes-*`。
- 尺寸档：标准 16、辅助 12；自绘资源保留 24 画布和原始描边，不对单个图标额外加粗。
- 数量：槽位 148；已有可作参照的自绘映射 129（123 个不同资源）；需要从零设计的共 20 个（19 个槽位 + 1 个非槽位资源）。

  (a) 只有 SF-only 回退的 15 个：

  listRemove、zoomIn、zoomOut、settings、openExternal、authorized、missedRun、diffAnnotation、fileLink、attach、gitAhead、gitBehind、settingsMarkdownFormat、cron、timezone

  settings 的说明：之前的自绘定稿在小尺寸下读成准星，被否决；请给出在 12 与 16 下可读的新稿。

  (b) 原稿已作废的 5 个：

  | 槽位 | 资源名 |
  |---|---|
  | annotation | tekes-annotation |
  | permissionReadOnly | tekes-perm-read-only |
  | permissionWorkspaceWrite | tekes-perm-workspace-write |
  | permissionFullAccess | tekes-perm-full-access |
  | （非槽位，Settings 的 Computer Use 页直接按名字加载） | tekes-computer-use |

  原因：原稿含来源不明的 C2PA 内容出处元数据，已从 base 移除。请从零设计，**不得参照原稿**，也不会收到原稿。
- 文件类型图标（约 30 个，如 C 语言文件图标）：范围待定，见第 6 节。
- 描边宽度不一致：设计页写 2pt，Icon Tokens.md 与 SVG 是 1.5pt。**请明确说明以哪个为准并给理由；如果重定，需要列出所有受影响图标的处理方式。**

### 4. 要回答的问题

1. 笔法规范：1.5pt 还是 2pt；端点、拐角、间距、最小笔画、关键线（keyline）；小尺寸（12pt）下的简化规则。
2. 从零设计 20 个图标（(a) 15 个 SF 回退 + (b) 5 个原稿作废）：每个给 SVG，并给 12 与 16 两种渲染下的可读性说明。
3. 颜色：第 3.2 节的全部缺口；accent 与 textSelection 的策略；三种外观的对比度验算表。
4. 文字：平台中立字号表与动态字号策略。
5. 命名：图标用 kebab-case；是否保留 `tekes-` 前缀（保留利于与私有资产区分且避免与其他库冲突，去掉利于公开库的简洁与通用）。请给出推荐与理由。token 名同理（如 `color.row-selection`）。
6. 对现有 129 个自绘图标：逐个声明是否原创（见第 5 节）。
7. 与 Tekes 现有 token 的映射表（见第 7 节）。
8. 跨平台差异：Windows、Linux 上哪些 token 允许平台覆盖（如 accent、字体），哪些必须保持一致。

### 5. 版权与原创性（硬要求）

- 所有交付的图标必须是**原创或 Tekes 自有作品**，才能随 MIT 公开。
- **不得描摹或派生 SF Symbols**。SF-only 回退的 15 个必须从零原创，不参考 SF 的具体形状；原稿作废的 5 个同样从零设计，不得参照原稿。
- 现有 129 个自绘图标在公开前，请逐个声明："原创"或"曾参照 SF 形"。凡曾参照 SF 形的，标出并**重画**，交付重画稿。声明格式见第 7 节。
- 不得引入第三方图标库、字体图标、商标形状。
- 品牌 / provider logo（如 Anthropic、OpenAI 等）是第三方商标，**一律不在范围内，也不得放进交付物**。

### 6. 范围

在范围内：

- 通用界面图标集（148 个槽位对应的图标，含从零设计的 20 个）。
- 全部 token：颜色、文字、间距、圆角、描边、动效、控件尺寸、alpha。
- 笔法规范与命名规范。
- 映射表与原创性声明。

明确不在范围内：

- 品牌 / provider logo。
- 文件类型图标（约 30 个）：默认**不在本轮**；若你认为应纳入，请单列提案（含原创性核查），由用户另行决定。
- Markdown 正文排版（已定案，不重开）。
- HUD（明确不做，不进设计系统）。
- 阴影（已裁决不要）。
- 对 Tekes 应用界面的重设计；本委托只整理词汇和补缺口。

### 7. 交付格式

#### 7.1 token 真源

> **已被 artifact 格式取代，见 design/README.md。** 下面的 schema 草案仅作历史保留。

平台中立的 JSON（或 TOML，二选一并说明理由），含 light、dark、increase-contrast 三套。建议 schema 草案如下，你可以改进，但需保持"一个文件、三种外观、可机器校验"：

```json
{
  "schema": "tekesui-tokens/0",
  "color": {
    "row-selection": {
      "light": "#EEF0F0",
      "dark": "#...",
      "increase-contrast-light": "#...",
      "increase-contrast-dark": "#...",
      "role": "ground",
      "note": "持久行选中底，不用于凹槽或 pending"
    },
    "hover-wash": {
      "light": { "base": "primary-text", "alpha": "faint" },
      "dark": { "base": "primary-text", "alpha": "faint" },
      "increase-contrast-light": { "base": "primary-text", "alpha": "soft" },
      "increase-contrast-dark": { "base": "primary-text", "alpha": "soft" }
    }
  },
  "alpha": { "faint": 0.07, "soft": 0.14, "wash": 0.45 },
  "spacing": { "none": 0, "micro": 2, "tight": 4, "snug": 6, "regular": 8, "relaxed": 12, "loose": 16, "section": 24 },
  "radius": { "square": 0, "badge": 4, "row": 6, "panel": 12, "floating": 14, "input": 16, "column": 18, "control": 28 },
  "stroke": { "hairline": 0.5, "rule": 1, "emphasis": 2 },
  "motion": { "instant": 0, "state-change": 0.12, "geometry": 0.18, "list-expansion": 0.3, "expansion": 0.5 },
  "control-size": { "dot": 6, "icon-button": 28 },
  "text": {
    "body": { "size": 13, "weight": 400, "line-height": 16 }
  }
}
```

要点：

- 颜色值既可以是绝对 sRGB，也可以是"另一角色 + alpha 档"的引用；引用必须无环。
- Increase Contrast 要分别给出 light 与 dark 下的值（因此是四个取值：light、dark、IC-light、IC-dark），或说明可以由 light / dark 推导。
- 每个 token 带一句用途说明和"谁不能用它"的禁区，沿用现有词汇表的写法。
- 请同时给出一个校验清单（引用无环、对比度达标、三套完整）。

#### 7.2 图标

- 格式：SVG，viewBox `0 0 24 24`，单色，描边使用 `currentColor`，不嵌入颜色、渐变、滤镜、文字。
- 描边宽度：统一一个值，写在笔法规范里。
- 命名：kebab-case，无空格；前缀取舍由你推荐（见第 4 节第 5 条）。
- 提供一张索引：语义槽位名 → 文件名 → 12 与 16 下的可读性备注 → 原创性声明。
- 同一槽位只给一个最终稿；备选稿另放。

#### 7.3 映射表

Tekes 现有 case 名 → TekesUI token 名，逐行照抄，列包括：

| 列 | 内容 |
|---|---|
| 类别 | color / text / spacing / radius / stroke / motion / control-size / icon |
| Tekes case 名 | 如 `rowSelection`、`pillHeight` |
| TekesUI token 名 | 如 `color.row-selection`、`control-size.pill-height` |
| 值是否变化 | 无变化 / 有变化（写出新旧值与理由） |
| 备注 | 例如"新增"、"合并"、"废弃" |

规则：数值无变化的，不要为改名而改值；有变化的单列出来，供用户审阅"与 Tekes 现有视觉漂移"。

#### 7.4 原创性声明

每个图标一行：槽位名、文件名、"原创" / "重画（曾参照 SF 形）"、重画完成与否、备注。

### 8. 禁区

- 不要引入新的 case 或数值而不说明：任何新增都要写明"现有 case 为什么不够"。
- 不要自绘阴影，不要渐变高光，不要玻璃效果依赖系统材质。
- 不要提出需要平台私有 API 才能实现的效果作为必选项。
- 不要修改 Tekes 私有仓库的任何文件；你的交付物是独立文件。
- 不要在交付物里复制 Swift 源码段落；用 case 名、数值和文字描述。
- 不要把品牌 logo 或第三方商标形状放进图标集。
- 不要重开第 0 轮已定案的裁决，除非给出明确不成立的证据。

### 9. 收到后的验收

- token 文件通过第 7.1 节的校验清单。
- 所有颜色在三种外观下有值；对比度验算表齐全。
- 图标全部符合 viewBox / currentColor / 命名规范，且在 12 与 16 下的可读性已逐个说明。
- 原创性声明覆盖全部图标，重画稿已交付。
- 映射表覆盖第 3 节列出的全部 case。
