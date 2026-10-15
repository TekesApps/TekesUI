# v0.3.0-proposal.1 合并映射与数值变化

本报告对应用户授权的“包内已有定义优先、现有补集保留”。原 a8/24 图只作历史证据；新外观不是旧图像无变化迁移。

现有公开 API 76 个枚举角色＋1 个常量均保留；169 个模式/字段绑定采用包定义。原 508 个数值通道中 **315 改变，193 保留**（按 f32 bits 比较，含字重）。

活跃作者输入在 `design/source/*.json` 与 `design/brand/*.json`；`design/tokens.json` 是生成投影，不允许手编。默认显式配置从包 `gallery.reference_brand=example-neutral` 选取。包内 base+config+policy 经原 `compile_config` 生成品牌角色；普通 Rust 构建核输入与投影哈希，陈旧即失败。

## 现有颜色 API（四主题逐项）

| 旧 role / 包语义 ID | 改变主题 | 说明 |
|---|---|---|
| `scrim` | light, dark, light-ic, dark-ic | 包明确最终 alpha-wash=.45，不乘父子透明度 |
| `warning-text` | 无，四主题保留 | 同名语义精确对应；不猜测改名 |
| `warning-foreground` | light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `row-selection` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `button-hover` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `transcript-input` | light, dark, dark-ic | 同名语义精确对应；不猜测改名 |
| `border` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `floating-panel-border` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `floating-panel-divider` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `border-focused` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `primary-text` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `secondary-text` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `tertiary-text` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `selected-text` | light, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `document-canvas` | light, dark, dark-ic | 同名语义精确对应；不猜测改名 |
| `text-selection` | light, dark, light-ic, dark-ic | 普通不透明，IC 最终 alpha-selection-ic=.75 |
| `accent` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `status-running` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `warning` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `status-failed` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `status-succeeded` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `diff-added-marker` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `diff-removed-marker` | light, dark, light-ic, dark-ic | 同名语义精确对应；不猜测改名 |
| `hover-wash` | light, dark, light-ic | 包 usage 明确为 primary-text 叠色源；兼容 API 使用 faint/IC soft 最终 alpha |
| `separator` | light, dark, light-ic, dark-ic | 包显式不透明色（不保留旧透明派生） |
| `diff-added-surface` | light, dark, light-ic, dark-ic | 包显式不透明色（不保留旧透明派生） |
| `diff-removed-surface` | light, dark, light-ic, dark-ic | 包显式不透明色（不保留旧透明派生） |

## 现有度量、动效与文字 API

| 旧 API | proposal 对应 | 变化 |
|---|---|---|
| `spacing.none` | `space-none` | 0 → 0 pt |
| `spacing.micro` | `space-micro` | 2 → 2 pt |
| `spacing.tight` | `space-tight` | 4 → 4 pt |
| `spacing.snug` | `space-snug` | 6 → 6 pt |
| `spacing.regular` | `space-regular` | 8 → 8 pt |
| `spacing.relaxed` | `space-relaxed` | 12 → 12 pt |
| `spacing.loose` | `space-loose` | 16 → 16 pt |
| `spacing.section` | `space-section` | 24 → 24 pt |
| `radius.square` | `radius-square` | 0 → 0 pt |
| `radius.agent-marker` | `radius-agent-marker` | 3 → 3 pt |
| `radius.badge` | `radius-badge` | 4 → 4 pt |
| `radius.row` | `radius-row` | 6 → 6 pt |
| `radius.control` | `radius-control` | 28 → 8 pt |
| `radius.input` | `radius-input` | 16 → 8 pt |
| `radius.panel` | `radius-panel` | 12 → 10 pt |
| `radius.column` | `radius-column` | 18 → 18 pt |
| `radius.floating` | `radius-floating` | 14 → 12 pt |
| `stroke.hairline` | `stroke-hairline` | 0.5 → 0.5 pt |
| `stroke.rule` | `stroke-rule` | 1 → 1 pt |
| `stroke.emphasis` | `stroke-emphasis` | 2 → 2 pt |
| `size.focus-bar` | `size-focus-bar` | 2 → 2 pt |
| `size.dot` | `size-dot` | 6 → 6 pt |
| `size.agent-marker-width` | `size-agent-marker-width` | 8 → 8 pt |
| `size.agent-marker-height` | `size-agent-marker-height` | 14 → 14 pt |
| `size.inline-badge` | `size-inline-badge` | 16 → 16 pt |
| `size.selector-track` | `size-selector-track` | 20 → 20 pt |
| `size.inline-action` | `size-inline-action` | 24 → 24 pt |
| `size.compact-row` | `size-compact-row` | 24 → 24 pt |
| `size.selector-knob` | `size-selector-knob` | 24 → 24 pt |
| `size.action-button` | `size-action-button` | 24 → 24 pt |
| `size.icon-button` | `size-icon-button` | 28 → 32 pt |
| `size.pill-height` | `size-pill-height` | 28 → 28 pt |
| `size.row-height` | `size-row-height` | 32 → 32 pt |
| `size.truncation-slot` | `size-truncation-slot` | 180 → 180 pt |
| `size.truncation-slot-wide` | `size-truncation-slot-wide` | 240 → 240 pt |
| `Motion::Instant` | `motion.duration.instant` | 0 → 0 s；reduced 全部 instant=0 |
| `Motion::StateChange` | `motion.duration.stateChange` | 0.12 → 0.15 s；reduced 全部 instant=0 |
| `Motion::Geometry` | `motion.duration.geometry` | 0.18 → 0.12 s；reduced 全部 instant=0 |
| `Motion::ListExpansion` | `motion.duration.listExpansion` | 0.3 → 0.3 s；reduced 全部 instant=0 |
| `Motion::Expansion` | `motion.duration.expansion` | 0.5 → 0.5 s；reduced 全部 instant=0 |
| `Alpha::Faint` | `alpha-faint` | 标准值不变；Faint 在 IC 仍采用 Soft（包 usage 明定） |
| `Alpha::Soft` | `alpha-soft` | 标准值不变；Faint 在 IC 仍采用 Soft（包 usage 明定） |
| `Alpha::Wash` | `alpha-wash` | 标准值不变；Faint 在 IC 仍采用 Soft（包 usage 明定） |
| `TEXT_SELECTION_INCREASED` | `alpha-selection-ic` | 保留 .75，并按包 usage 应用于 IC text-selection |
| `TextRole::Body` | `ui-body`，精确 Body 映射 | 13/16/400，不变 |
| `TextRole::BodyEmphasized` | 包无同名/明确等价角色，保留旧补集 | 13/16/600，不变 |
| `TextRole::RowMeta` | 包无同名/明确等价角色，保留旧补集 | 11/16/400，不变 |
| `TextRole::SectionHeader` | 包无同名/明确等价角色，保留旧补集 | 11/16/700，不变 |
| `TextRole::IdentityBadge` | 包无同名/明确等价角色，保留旧补集 | 10/16/600，不变 |
| `TextRole::Brand` | 包无同名/明确等价角色，保留旧补集 | 17/22/700，不变 |

## 新定义完整保留与消费边界

包 token 数量：color 152、spacing 30、radius 42、stroke 7、opacity 12、size 52；16 种文字样式完整保留字体族/字号/行高/字重/颜色，6 个动效时长、曲线与 spring、布局、交互和品牌 policy 全部保留。

`tekes_ui_tokens::proposal` 提供 raw color / final render_color，以及 metric/text/text_family/text_color/duration，以及 typed TextRole 与捕获/侧栏常量。完整 40 组件、10 recipes 是设计级规范；本轮未把它们伪称为原生组件库。

Gallery 固定捕获从 1200×800 → 1440×960，目录从 220 → 288；三校准页及文字滚动保留。目录/sidebar、selected-hover、按钮pressed/focus使用包对应语义，测量夹具专用宽高仍保留。

`proposal::color` 返回包 hex 定义的 raw opaque sRGB；`proposal::render_color` 按 JSON render_opacity 规则替换最终 alpha，并沿 alias 继承。legacy ColorRole resolve 消费最终语义值。

品牌源使用原包算法与验证（OKLCH constrained → 8bit opaque sRGB；对量化后结果做对比度/状态差异校验），native 值映射为 byte/255 的 f32，不再次量化现有补集。UI/runtime无Python依赖；Python只在显式构建期merge步骤使用。

## 验证边界

旧 508 通道 fixture 仅历史证据。当前 runtime getter 逐位对照新生成投影，完整 proposal token/16styles 也逐项核对。品牌 seed → merge → normal Cargo getter 的隔离探针另见任务 verification/proposal1-merge。新截图与和谐性由主控接手；未批准 release/品牌所有权、未实现全部组件/recipes/品牌原生热切换。
