# #8 当前组件登记：公开编译绑定与历史身份映射

2026-10-17。基于 `bc3de79466834e99062a1b4ef0872323e2860001` 加活跃 checkout 的全部未提交 WIP。GitHub 当前仍为 14 个开放 Issue、全部状态 0 个 PR；最新 CI 是旧 `bc3de79` 的失败，本批未推送，不能据本地结果声称远端 CI 恢复。#13 当前总览见 [整改当前状态](issue-remediation-current-20261017.md)。冻结源、编译/GPU/门禁结果和逐验收项状态见 `design/reviews/component-registry-api-20261017/receipt.json`。

## 根因与共享修复

根因属于公共组件契约与治理：旧清单用全仓同名 `pub` 声明证明 API，既会漏掉 ScrollArea 的真实 re-export alias，也可能接受 `tekes_ui_controls::widgets::Button` 这样的私有模块路径。17 项没有 API 字段，Gallery 又有自己的 renderer→API 回退表，导致清单、文档和实际消费入口不一致。

现在复用 `design/gallery/workbench.json` 为登记唯一入口，补齐当前 45 项的主 API 和 80 条模型/视图公开路径。Text、TextField、TextArea、SearchField 明确共享 TextSurface/TextInput 能力，按原变体配置使用；没有为消除登记缺项另建编辑器或 UI 类型。Surface、Icon、Skeleton 直接登记实际公共 View；Toolbar 只登记当前动作组模型/View，不冒充 #14 的窗口级注册与生命周期能力。

`scripts/generate-component-capabilities.py` 按模块与 re-export/alias 定位真实声明和当前行号，不再从全仓同名类型随意选文件。定位器保守支持当前明确的外部模块/扁平 use 组；它不是完整 Rust 解析器或可见性证明。生成的 `crates/tekes-ui-controls/tests/component_registry_api.rs` 才是独立的编译验证：在库外 integration-test 上下文使用全部 80 条实际类型路径，并逐项比较当前 JSON 的组件 ID、route 和 binding。API/route 改动而测试未重生成会失败；同名私有路径不能靠声明扫描通过。

生成的 JSON 继续写 `compiler-check-required`，不把“生成过测试”写成“已经通过编译”。本批通过结果由冻结源收据保存，未来源码变化必须重新编译。JSON 原有字段类型保持兼容，新编译信息为增量字段。

Gallery 的所有组件 API 引用改为读取同一登记；移除组件 renderer 的独立猜测表，缺项显式显示登记失败。旧文档对所有基础组件一律列出 button-hover 等通用 token，不能证明真实依赖，现改为明确未逐项审计；本批没有伪造完整 token 映射。

## 40 → 45 的显式映射

新增的 `design/gallery/historical-component-map.json` 完整解释原 40 个 ID 和当前 45 个 ID：

- 39 项沿用稳定 ID；这只表示身份延续，不表示旧提案的 API、状态和体验已经实现。
- 当前增加 Surface、Toolbar、Field、Select、NumberField、Skeleton，共 6 项，分别注明当前能力和例外。
- 原 SelectionSurface 与当前 TextSurface 有部分能力重叠；跨节点富文档选择仍未实现。明确记录部分重叠与缺口，不静默丢弃、不标作已合并完成。

缺少/重复历史 ID、未解释的当前新增项、未知目标、伪装为 same-id 的改名都会使生成失败。历史原包、40 项 planned 登记和原截图完全不改。

## 公共 API、真实接入与 Gold7

| 层级 | 证据及边界 |
| --- | --- |
| 公共 API | 当前 45 项、80 条真实模型/View 路径全部在外部 Rust integration test 中编译通过。另两个 workspace 外临时 Cargo 消费者分别引用 widgets::Button、extended::Button，实际编译均以 E0603 拒绝，完整日志保留。12 项 Python 治理测试覆盖 alias、错误模块、缺失 API/绑定、历史缺项/重复/未知 ID。 |
| 真实接入 | Gallery 逐组件检查 Usage 与同一 JSON API 一致；45 个真实 docs route × 4 主题 × 两组布局/倍率，输出 360 张真实 GPU 图。另回归既有 Gallery 页面 520 帧和独立组合场景 80 帧；组件数维持 45，没有扩充迁移。独立组合消费者 13 项回归通过。 |
| 最终体验 | 抽查 ScrollArea 普通浅色页和 TextArea 深色 IC、窄窗口大字号/@2x 文档页，确认真实 API 名称可读、按模板排版。非背景像素或编译成功都不是全页体验通过；没有把 960 张图写成逐图人工验收。G1–G3 沿用 JSON 视觉规则，未改 token 或组件外观；G4/G5/G6 的物理动效、候选窗、VoiceOver 不因本批晋级；G7 增强公开路径和可运行 Gallery 的一致性。 |

工作区最终 Rust 测试 897 passed、0 failed、1 原有 ignored；两个计时测试按用户范围显式过滤。fmt、严格 workspace Clippy、DS、纯 Rust、Gold7 mechanical、reference alignment 及生成清单检查通过。既有 CI 已运行该生成检查和 workspace tests，因此新编译绑定会进入同一门禁；无需另增一条只看声明存在的检查，也没有修改或绕过原门禁。

## 原 Issue 未完成项

#8 仍为部分完成：编译绑定、45 项主 API 补齐与完整历史身份说明已交付；全体 public/helper 类型的反向分类、每个组件必需状态的覆盖，以及完整 token 依赖映射仍未闭合。新增未登记类型仍需这一反向清单才能精确指出缺项，不能声称本批已经解决全部公开面治理。所有组件继续 experimental，体验状态未自动改为 pass。

后续继续原 Issue 的反向类型分类和状态/token 审计；#7 消费方品牌入口、#10 公共阅读控制器、#11 草稿协议仍按现有 WIP 补公共边界，不重做已有 Gallery 实现。#5/#12 的候选窗和 VoiceOver 验收也继续保留。未自动提交、推送、打 tag、关闭 Issue 或发布，原 WIP 与历史证据保持保护。

## 2026-10-26 实用 Theme 覆盖清单（现行审计范围）

本节对应 #113/#114 及 #121–#125，取自现有 `design/gallery/workbench.json` 的52根，登记仍以该文件为唯一来源。上文2026-10-17的45项测试与历史统计保持原身份，不是当前52根全量通过。基线为本地 main `3d7ff9eade945390131028a22a915603d5a8829b`；本次仅核对已知接收范围，不重新全面审计或为未知项补造通过。

验收条款见 [Theme foundation 的现行审计契约](theme-foundation-main-intake-20261026.md)。每根只补消费、状态和实际集成差异；Palette是合格语义入口，不强制为统一形式而迁移Surface。未知状态标待核对；不把没有登记例外当作已证明零硬编码。无障碍/原生及完整产品能力仍归原票，不由审计简化自动通过。

证据简写：候选89位于SSD `artifacts/issue-113/button-layout-producer89-20261026-01`；其他main批次见相关设计文档与原Issue收据。此表是现有registry的人工验收说明，不是第二套ID或机器登记。#114机器投影尚待实现/验收，不把本表写入等同该票完成。

| 登记根 | 公开入口/主题消费 | 已验状态与身份 | 合理例外 | 未完成项 | 证据/Issue |
|---|---|---|---|---|---|
| `tui.stack` | `tekes_ui_controls::extended::Stack`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.grid` | `tekes_ui_controls::extended::Grid`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.separator` | `tekes_ui_controls::extended::Separator`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.split-view` | `tekes_ui_controls::extended::SplitView`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.scroll-area` | `tekes_ui_controls::ScrollArea`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.disclosure` | `tekes_ui_controls::extended::Disclosure`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.surface` | `tekes_ui_controls::feedback_view::SurfaceView`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.toolbar` | `tekes_ui_controls::Toolbar`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#121 |
| `tui.text` | `tekes_ui_controls::TextSurface`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.icon` | `tekes_ui_controls::layout_view::IconView`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.button` | `ButtonContent → Button::render → paint::button_variant_with_interaction → Palette` | main: public state/focus/transition regressions; actual public ink/fill contrast; real Gallery theme/brand switching, pointer activation/cancel and disabled-focus regressions passed | 无新增豁免；普通几何/索引数字不要求token化 | Representative label-Button theme acceptance passed; remaining #60 content/Host functionality and native/Gold7 obligations stay separate, not claimed complete | button-public-contrast-main2a / button-gallery-main-cef receipts (SSD issue-113); candidate89 remains historical; #113/#60；#113 |
| `tui.icon-button` | `tekes_ui_controls::extended::IconButton`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.link` | `tekes_ui_controls::extended::Link`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.text-field` | `tekes_ui_controls::TextSurface`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.text-area` | `tekes_ui_controls::TextSurface`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.search-field` | `tekes_ui_controls::TextSurface`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.combo-box` | `tekes_ui_controls::extended::ComboBox`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.slider` | `tekes_ui_controls::extended::Slider`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.checkbox` | `tekes_ui_controls::Checkbox`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.radio-group` | `tekes_ui_controls::extended::RadioGroup`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.switch` | `tekes_ui_controls::extended::Switch`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.segmented-control` | `tekes_ui_controls::extended::SegmentedControl`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.field` | `tekes_ui_controls::Field`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.select` | `tekes_ui_controls::Select`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.number-field` | `tekes_ui_controls::NumberField`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#122 |
| `tui.tabs` | `tekes_ui_controls::Tabs`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.breadcrumb` | `tekes_ui_controls::extended::Breadcrumb`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.command-palette` | `tekes_ui_controls::extended::CommandPalette`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.list` | `tekes_ui_controls::extended::List`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `tui.tree` | `tekes_ui_controls::extended::Tree`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `tui.data-table` | `tekes_ui_controls::extended::DataTable`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `tui.chart` | ChartView → Theme/Palette/Surface；ChartContent另属#92 | ChartView有限CPU/strict/外部consumer/GPU已入main `3bd9f00` | 无新增豁免；普通几何/索引数字不要求token化 | ChartContent及必要Host/Scene依赖接收；不转签View验证 | chart-content-20261024.md；#104/#92；#124 |
| `tui.menu` | `tekes_ui_controls::Menu`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.tooltip` | `tekes_ui_controls::extended::Tooltip`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.popover` | `tekes_ui_controls::extended::Popover`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#123 |
| `tui.dialog` | retained Dialog → ContentTheme.surface；继承祖先/Host Surface，支持显式覆盖与清除 | 有限继承与公开场景回归已入本地main `8e4c01d`；不等于完整Dialog主题/原生验收 | 无新增豁免；普通几何/索引数字不要求token化 | 补核Dialog自身状态/切换/对比及原票剩余项，不重跑输入未变的继承回归 | #62/#97；SSD `artifacts/issue-136/surface62-dialog-main-bcea-20261026-01/commit-receipt.json`；#123 |
| `tui.toast` | `tekes_ui_controls::extended::Toast`；语义消费入口待逐项核对 | 共享规则#93已入main `3e989f1`；不等于Content接入通过 | 无新增豁免；普通几何/索引数字不要求token化 | 接收#79现有Content/Gallery候选及必要依赖；只补实际缺口 | #93/#79原收据；本篇Theme foundation链接；#125 |
| `tui.banner` | `tekes_ui_controls::extended::Banner`；语义消费入口待逐项核对 | 共享规则#93已入main `3e989f1`；不等于Content接入通过 | 无新增豁免；普通几何/索引数字不要求token化 | 接收#79现有Content/Gallery候选及必要依赖；只补实际缺口 | #93/#79原收据；本篇Theme foundation链接；#125 |
| `tui.status-badge` | `tekes_ui_controls::extended::StatusBadge`；语义消费入口待逐项核对 | 共享规则#83已入main `8fb1909`；不等于Content接入通过 | 无新增豁免；普通几何/索引数字不要求token化 | 接收#76现有Content/Gallery候选及必要依赖；只补实际缺口 | #83/#76原收据；本篇Theme foundation链接；#125 |
| `tui.progress` | 共享progress-track/rest规则已接收；ProgressContent消费候选尚未接收 | 共享规则#71已入main `dc7283f9`；#72公共消费者在当前main缺共享接口而编译失败，不能转签Root证据 | 无新增豁免；普通几何/索引数字不要求token化 | 先接收#5共享Content文本binding/完整carets快照与真实macOS语义投影，再接收#72候选并验证 | #71/#72原收据；SSD `artifacts/issue-136/progress72-surface62-main-bcea-20261026-01/dependency-map-text-and-platform.md`；#125 |
| `tui.empty-state` | `tekes_ui_controls::extended::EmptyState`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `tui.skeleton` | `tekes_ui_controls::feedback_view::SkeletonView`；语义消费入口待逐项核对 | 共享规则#83已入main `8fb1909`；不等于Content接入通过 | 无新增豁免；普通几何/索引数字不要求token化 | 接收#77现有Content/Gallery候选及必要依赖；只补实际缺口 | #83/#77原收据；本篇Theme foundation链接；#125 |
| `tui.markdown` | `tekes_ui_controls::extended::Markdown`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `tui.code-block` | `tekes_ui_controls::extended::CodeBlock`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `tui.diff-view` | `tekes_ui_controls::extended::DiffView`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#124 |
| `pattern-form` | `form`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `pattern-filter` | `filter`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `pattern-master` | `master`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `pattern-async` | `async`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `pattern-danger` | `danger`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `example-settings` | `settings`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
| `component-scene3d` | `scene3d`；语义消费入口待逐项核对 | 未在本批复验；既有证据保留原身份 | 无新增豁免；普通几何/索引数字不要求token化 | 核对原票候选、消费映射及必要状态/切换/对比 | 原族票；本批不声明通过；#125 |
