# #13 / #8 / #14 当前审计入口（2026-10-27维护，沿原2026-10-21入口）

本页原始审计快照 HEAD `b8999fe006adfcd70587f07af50ef9e240422788` 是历史身份，保留在机器矩阵 `prior_audit_heads`。当前维护基线为 `cdcc806f983b75a2d8285417b17114e51c4ce4f9`；本次重绑当前45项登记入口与源码锚点，并纳入已接收的有限 Content 适配器。历史包/原生收据不改身份、不用新 HEAD 代替。

机器矩阵仍覆盖45项：公共组合协议 **18已实现、11部分实现、16缺失**。这统计通用 Content 协议接入，不是控件整体完成度；叶子无需 children。全部45个 `native_acceptance` 仍为“未验证”，不从局部测试升级成熟度。旧入口内行号属于旧时点；以下表由当前矩阵投影。

## 当前组合维度

| 组件 | 维度 | 协议 | 内容能力 | 槽位 | 当前首条源证据 |
|---|---|---|---|---|---|
| Stack | slots | 已实现 | 已实现 | children* | `crates/tekes-ui-controls/src/extended.rs:20` |
| Grid | slots | 已实现 | 已实现 | cells* | `crates/tekes-ui-controls/src/extended.rs:61` |
| Separator | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:103` |
| SplitView | slots | 已实现 | 已实现 | leading1, trailing1 | `crates/tekes-ui-controls/src/extended.rs:113` |
| ScrollArea | slots | 已实现 | 已实现 | children* | `crates/tekes-ui-controls/src/scroll_view.rs:32` |
| Disclosure | slots | 缺失 | 部分实现 | 无 | `crates/tekes-ui-controls/src/extended.rs:537` |
| Surface / Panel | slots | 已实现 | 已实现 | body1 | `crates/tekes-ui-controls/src/feedback_view.rs:207` |
| Toolbar | slots | 部分实现 | 部分实现 | original Content groups in navigation/automatic/principal/status/primary/secondary regions | `crates/tekes-ui-controls/src/widgets.rs:3132` |
| Text | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:113` |
| Icon | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/layout_view.rs:117` |
| Button | leaf-and-slots | 已实现 | 已实现 | content1 | `crates/tekes-ui-controls/src/composition/button.rs:79` |
| IconButton | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/composition/button.rs:79` |
| Link | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:559` |
| TextField | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:113` |
| TextArea | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:113` |
| SearchField | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/text_surface.rs:113` |
| ComboBox | leaf-and-slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1246` |
| Slider | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/selection.rs:34` |
| Checkbox | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1211` |
| RadioGroup | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/selection.rs:288` |
| Switch | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/selection.rs:638` |
| SegmentedControl | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:808` |
| Field | slots | 部分实现 | 部分实现 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1641` |
| Select | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1576` |
| NumberField | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1597` |
| Tabs | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/widgets.rs:1476` |
| Breadcrumb | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:873` |
| CommandPalette | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/extended.rs:1312` |
| List | slots | 已实现 | 已实现 | rows* | `crates/tekes-ui-controls/src/collections_view.rs:230` |
| Tree | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:527` |
| DataTable | slots | 缺失 | 缺失 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:981` |
| Chart | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/chart.rs:39` |
| Menu | leaf-and-slots | 已实现 | 部分实现 | trigger | `crates/tekes-ui-controls/src/widgets.rs:1506` |
| Tooltip | leaf | 部分实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1118` |
| Popover | slots | 部分实现 | 部分实现 | one content callback; real minimum consumer owns one Content subtree | `crates/tekes-ui-controls/src/extended.rs:1123` |
| Dialog | slots | 已实现 | 已实现 | header1, body1, footer1 | `crates/tekes-ui-controls/src/widgets.rs:1550` |
| Toast | slots | 部分实现 | 部分实现 | fixed action proxy1 | `crates/tekes-ui-controls/src/extended.rs:1145` |
| Banner | slots | 部分实现 | 部分实现 | fixed action proxy1 | `crates/tekes-ui-controls/src/extended.rs:1191` |
| StatusBadge | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1207` |
| Progress | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/extended.rs:1214` |
| EmptyState | slots | 部分实现 | 部分实现 | fixed action proxy1 | `crates/tekes-ui-controls/src/extended.rs:1239` |
| Skeleton | leaf | 已实现 | 不适用 | 无 | `crates/tekes-ui-controls/src/feedback_view.rs:2731` |
| Markdown | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:1818` |
| CodeBlock | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:1928` |
| DiffView | leaf | 缺失 | 不适用 | 无 | `crates/tekes-ui-controls/src/collections_view.rs:2034` |

## 后续证据已解决的旧“待验”描述

- Label core 的公开标签策略和 renderer/Content 接入已有 #31 后续实现、26个限定原生检查和545ef8d7独立包重放；不再把 renderer 服务桥记为尚未实现。
- #27 布局驱动文本服务前置条件已有14个限定原生检查和292f5fe6独立包重放；实际消费与公开交付另列。
- #28 的 viewport 原语只是历史前置步骤。稳定 item、嵌套坐标、几何过期与 Start 对齐已有10b5f235/5882774a包和后续限定原生证据；不能继续把已实现的主命令列为缺失。
- #29 原紧凑动作已有7项限定原生检查和c7797b1e包；#33 已修复合法委托包装器焦点装饰转发并交付95b140c2。实际产品的最终焦点环体验与公共版本仍单列。
- Tooltip 混合所有权修复后来补齐16项真实系统失活/恢复检查，保持显式 caller 与被动焦点的区别；适用听读和统一页面验收未完成。
- #30 输入带留白已由原编辑器公共契约处理；#34 又补只读焦点意图查询，避免宿主 IME 屏障复制私有命中。生产方21专项、24GPU、10外接屏原生检查及a30b18db干净解包通过，AgentKit 后续有界实际接入和主Composer/SideChat合成AppKit组字验证已通过，物理输入法及完整体验仍待验；接收收据在 `design/reviews/agentkit-padding-intent-receiver-20261021/receipt.json`。

精确收据/哈希及替代关系在机器矩阵 `current_scope_reconciliation_20261021` 中；旧收据不改，不把生产方 WIP、独立包和实际 AgentKit 视为同一原生身份。

## 依赖与验收次序

1. root 维护唯一共享实现与当前门禁，现成能力不足才补公共契约；历史已实现项不重做。
2. AgentKit 以新固定a30b18db输入完成真实 Composer/rail 接入、组字屏障及匹配 ClientKit 的最终体验。私有包通过不代表公开 pin 或允许关闭。
3. root 继续同一最终组合候选的文本、动作/选择、Menu/Dialog/Popover、集合/阅读组件族验收；适用 VoiceOver 人声验证依用户延后决定，当前保持关闭。
4. #12 统一组件及页面：同内容/宽度/主题/品牌/backing/type scale 对照；真实 GPU、原生键鼠、物理 IME、听读分别留证，不从非空光栅或源代码推断体验。
5. #13 总览、#8 能力投影、#14 组合审计仅按各原 Issue 的完整条件收口；公开源、同版本远端CI及独立审阅须有对应授权和证据。

Toolbar 的单 AppWindow 集成已有有界接收：多个列共享 ToolbarHost，含稳定声明/group ID、window-edge region、placement/order、支持的 command overflow，以及有限真实 Window/owner/focus 路径。一般多窗口生命周期、任意自定义 Content overflow 和全动作原生语义/VoiceOver仍未验证；不把这些限制写成整个 Window 注册/placement/overflow 尚不存在。Tree/DataTable 有模型/View/语义修复不等于已新增通用 Content 适配器；只在公共契约能表达时归入对应维度。

## 兼容边界

兼容性按具体入口逐项记录：统一 Button 的旧 ButtonContent/CompoundButton/IconButton 等入口已明确删除且无别名；使用 composition::Button/ ButtonContents 迁移，自定义装饰树仍由 ButtonContents::Custom/From<Content> 支持，原 ButtonModel 继续拥有动作状态。其他组件保留的原实例/挂载身份、真实动作/AX节点和输入状态机按各自记录适用。`ReadingTransactionInProgress` 等已公开枚举扩展需要下游穷举分支迁移；required 语义或公开模型字段增加可能影响直接 struct literal，不能笼统称全部源码兼容。新增 trait 焦点装饰查询有默认false，正常Minimum/Release保持原行为；Exact/Press显式选用。查询不是异步授权或旧mount预约；实际 dispatch/apply 仍重检。

## 当前 CI 证据边界

`design/reviews/frozen-ci-after-padding-intent-20261021/manifest.json` 固定1250个直接源码/设计/门禁证据文件；本地CI配方已通过：1218workspace/253consumer、全工作区严格Clippy及设计门禁；18ignored与3timing skip不升级为通过，详情以同目录 `checks.json` 和 `acceptance.json` 为准。该快照为本机当前WIP一致性验证，不是公开发布候选、远端CI或独立审计。后续账本/文档修正另记差异；并发3D只参与工作区常规编译，不接受其新体验，也不改它的WIP。

共享target复用、阶段后已闲置编译产物清理；源码、WIP、固定输入与验收文件保留。性能延期，未提交、推送、打tag、关闭Issue或发布。

#35 原Content动态帮助增加了原owner的公共说明附件接入，故Tooltip协议从缺失改为部分实现；未提供任意交互内容槽/独立TooltipContent，也未升级45个native状态。详见 `docs/design/content-help-20261021.md`。此前完整本地CI7e5092c9位于#35之前，新增量须看自己的验证收据，不以旧CI冒充新源码。

#36 P1 历史增量：原 Menu 的选择事件现在捕获不可变模型身份和值，打开触发器不产生选择；仅此事件阶段通过公共 API／consumer／Clippy。在该 P1 阶段，嵌套、勾选、禁用和最终原生体验仍未完成；后续 b430 固定包见下。旧完整 CI 7e5092c9 早于 #35／#36，不是当前全部 WIP 的 CI。详见 `design/reviews/menu-selection-event-20261021/progress.json`。


#36 固定输入跟进：`b430dae10b89` 基于已接受 `3366559a` 的515文件／22路径菜单增量已干净解包复验；原菜单声明、受控勾选／逐项禁用、紧凑命名原触发器、原动作队列不可变路径和公开 Menu 层级均有实际公共消费者证据。没有把无关 Root collection/layout/Scene3D 或新锁带入。395controls／302consumer、严格2Clippy、288个真实 Menu token 场景（complete=false）及16组CoreText/Metal通过；同一签名程序的外接屏键鼠／AX、窄窗、Tab、最小化撤销和失活迟到输入已有界定证据。真正恢复系统活动状态、AgentKit真实Sidebar及整页ClientKit/Gold7仍待验，不提升任何组件全族原生状态。原始 `7e5092c9` 全CI仍只对应旧输入，不是当前 Root 全WIP；并行新锁造成的当前门禁陈旧另列。详见 `design/reviews/menu-portable-candidate-20261021-attempt2/issue36-acceptance-matrix.json` 与 `HANDOFF.md`。


## 2026-10-27 当前接收的 Content 适配器

Chart、StatusBadge、Progress、Skeleton 的有限 leaf 适配器已接入当前45项审计；EmptyState、Toast、Banner 已接入固定 action-proxy 的 Content 适配器，但没有任意 caller body slot，Toast/Banner 也没有 live-region announcement 契约。协议状态按各自有限能力维护，所有 family `native_acceptance` 仍为“未验证”。对应接收收据保留原身份：`artifacts/issue-136/chart92-main909130a-intake/commit-receipt.json`、`artifacts/issue-136/status76-main437b5b6-intake/commit-receipt.json`、`artifacts/issue-136/progress72-main-63214be-intake/commit-receipt.json`、`artifacts/issue-136/skeleton77-main23447d2-intake/commit-receipt.json`、`artifacts/issue-136/empty78-mainb4fbed1-intake/commit-receipt.json`、`artifacts/issue-136/notice79-main08d845f-intake/commit-receipt.json`。


## 当前已接收增量（2026-10-27）

- Button：统一为单一 content-driven Button，旧 ButtonContent、CompoundButton、extended::IconButton、layout_view::IconButtonView 明确移除且无兼容别名；自定义 decorative Content 树和原 ButtonModel action owner 保留。
- IconButton：在45项审计中按统一 Button 的 ImageId 路径记为 owned-content leaf；不要求 children，也不推导专用 Icon role 或原生验收。
- Toolbar：当前单 AppWindow 多列共用 ToolbarHost，有限接收 stable group/declaration ID、window edge、placement/order、supported command overflow 和原 owner/focus 路径。一般多窗口生命周期、任意 Content overflow、全动作原生语义与 VoiceOver仍未验证。
- owned-content theme：继承 Palette 和规范化 Surface；Surface 改变时清除继承 foreground/icon foreground，任意子节点的前景色传递仍依赖显式 child_theme/component 规则。
