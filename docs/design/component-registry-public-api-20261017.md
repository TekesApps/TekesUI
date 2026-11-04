# #8 当前组件登记：公开编译绑定与历史身份映射

当前入口：原 Workbench/能力清单与 context 仍是唯一登记来源；52根实用 Theme 状态见本文现行表，45个公共组件与5 patterns/2 examples边界保持不变。#8 AC6的当前数值对齐报告见 [现有对齐文档](reference-alignment-20261016.md)，原2026-10-16报告不覆盖。下段的Issue数量、旧CI与测试统计属于2026-10-17历史阶段，当前交付状态以原Issue的唯一顶部摘要为准。

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

## 2026-10-17 原 Issue 未完成项（历史记录，当前状态见原 Issue）

#8 仍为部分完成：编译绑定、45 项主 API 补齐与完整历史身份说明已交付；全体 public/helper 类型的反向分类、每个组件必需状态的覆盖，以及完整 token 依赖映射仍未闭合。新增未登记类型仍需这一反向清单才能精确指出缺项，不能声称本批已经解决全部公开面治理。所有组件继续 experimental，体验状态未自动改为 pass。

后续继续原 Issue 的反向类型分类和状态/token 审计；#7 消费方品牌入口、#10 公共阅读控制器、#11 草稿协议仍按现有 WIP 补公共边界，不重做已有 Gallery 实现。#5/#12 的候选窗和 VoiceOver 验收也继续保留。未自动提交、推送、打 tag、关闭 Issue 或发布，原 WIP 与历史证据保持保护。

## 2026-10-26 实用 Theme 覆盖清单（现行审计范围）

本节对应 #113/#114 及 #121–#125，取自现有 `design/gallery/workbench.json` 的52根，登记仍以该文件为唯一来源。上文2026-10-17的45项测试与历史统计保持原身份，不是当前52根全量通过。起点为本地 main `3d7ff9eade945390131028a22a915603d5a8829b`。截至已验源码 `fc2f81a3983474e21b03041bf4dfa090409fcd6f`，本表52根已逐项完成当前实用 Theme 消费与适用集成接收；各自实际验证仍只属于原收据源码/程序身份，不把这一汇总提交当作重跑或全产品认证。该全量实用审计已随PR154必要CI通过并正常交付至origin/main `5879fe88d98cc2038eec5bc83baf5498ddd7f0bc`、annotated `checkpoint-20261028-14`；#113/#125按此范围关闭。后续独立组件修复按各行最新收据单独验收，不把新源码/运行转签为该检查点或原GPU；尚未远端交付的行仍只称本地接收。

验收条款见 [Theme foundation 的现行审计契约](theme-foundation-main-intake-20261026.md)。每根只补消费、状态和实际集成差异；Palette是合格语义入口，不强制为统一形式而迁移Surface。未知状态标待核对；不把没有登记例外当作已证明零硬编码。无障碍/原生及完整产品能力仍归原票，不由审计简化自动通过。

### #114 机器投影的历史观察规则（已接收）

现有 `capabilities.json` 继续消费 Workbench、coverage、公开 API surface 与来源 SHA；不新增登记表。Runtime observation 的 artifact、原 `inputs_sha256`、cases、tokens 与范围说明原样保留。若当前源码哈希与历史观察不一致，context 只显示 `historical-stale`、原/当前 SHA 差异及受影响状态；这些状态的当前 `public_api_verified` 置空，原历史声明放在 `historical_public_api_verified`，maturity 不变。该身份过期只表示旧观察不能证明当前源码状态，不表示组件功能失败，也不要求把所有旧探针重跑；新的 Button/Chart 证据继续按各自真实范围独立投影。`--context` 生成临时当前投影并注明与落盘清单是否一致，不写文件；`--check` 继续严格检查落盘清单；投影已由唯一负责人生成并在 `71c3d21`、`a7aae42` 接收。损坏的观察格式、身份哈希、路径、缺失来源、跨组件用例或状态记录仍拒绝，不把它们降级成 stale。

证据简写：候选89位于SSD `artifacts/issue-113/button-layout-producer89-20261026-01`；其他main批次见相关设计文档与原Issue收据。此表是现有registry的人工验收说明，不是第二套ID或机器登记。#114机器投影已按简化 AC 验收并关闭，具体接收范围见下文；本表中其余组件的未完成项不因此消失。

本族票 #123 的十个动作/导航根已在 PR141、checkpoint-20261028-1 按四条实用 Theme AC 完成。各行旧批次中的“本族未完成”保留当时证据身份；当前整族结论见原 Issue。此收口不表示 #113 全52根、旧闭包证明或全库原生验收完成。

本族票 #124 的七个数据/只读文档根已在 PR147、checkpoint-20261028-7 按四条实用 Theme AC 完成并关闭；各根实际运行/远端交付保持各自提交与收据身份。只读 subset、透明父背景和未验 GPU/原生等适用边界仍保留在原行与原票，不表示完整文档编辑器、全52根或旧闭包证明完成。

本族票 #121 的八个布局/容器根已在 PR148、checkpoint-20261028-8 按四条实用 Theme AC 完成并关闭。实际语义消费、必要状态/布局/捕获/滚动/动作和适用Gallery证据保留各自提交身份；#103 B Splitter当前main补交付、#12整页观察及全库原生等原目标仍由原票承担，不由此关闭或冒签。

#122十三根已随PR152／`checkpoint-20261028-12`完成实用AC并关闭。#125十三根本地接收已齐：复用Progress/Skeleton/EmptyState/Banner/StatusBadge既有成果，只补Toast与实际Form/Filter/Async/Settings的独立集成差异；Master补真实行到只读详情断言，Danger原组合具名运行，Scene3D两个原真实GPU测试显式执行通过（2 passed、0 ignored）。3D材质/灯光是场景数据，不强制UI token化。原#12布局、#81 B2、#103 B、#127组合与原生/a11y等产品缺口继续原票，不由本审计豁免。

机器边界保持明确：原coverage ledger投影45组件；5pattern与2example使用原登记route和experimental maturity，实际接收写本表/原Issue，不造component ID或扩生成框架。`accepted-bounded-evidence`不是全面runtime/native或maturity晋级。

| 登记根 | 公开入口/主题消费 | 已验状态与身份 | 合理例外 | 未完成项 | 证据/Issue |
|---|---|---|---|---|---|
| `tui.stack` | Existing Stack geometric model/StackView and measured StackContent ROW/COLUMN policy; original ContentTheme forwards declared panel Surface to real Text/Button children | Same retained StackContent/Host across both axes and Theme::ALL preserves child frame/visible/mount, exact child semantic text/Button packet ink and sole child focus target; actual Gallery three-cell StackView Theme/layout/labels and strict passed | Pure layout has no own background/hover/pressed/disabled/selected/focus state; caller gap/equal division/index geometry stays data; shared leaf contrast/states and #62 inheritance reused | No new dedicated brand/scale/GPU/native-window/IME/VoiceOver certification; old clipping/reveal/geometry evidence retains identity | #121/#113; SSD artifacts/issue-121/stack-main8253390-candidate/main-intake/commit-receipt.json; public test fixture corrected leaf-specific ink, no production changes |
| `tui.grid` | Actual Grid/GridView and measured GridContent; declared panel Surface/Theme propagates to original real Text/Button children | Same retained two-column/three-child GridContent across Theme::ALL keeps measured-row geometry/clip/mount and sole child focus, exact leaf-specific packet/text paint; actual Gallery nine cells/labels/geometry/clip and strict passed | Pure layout owns no paint/action/state; ordinary gap/indices/column geometry stays data; shared leaf contrast/states and #62 inheritance reused | No new dedicated brand/scale/GPU/native-window/IME/VoiceOver certification; other layout roots remain separate | #121/#113; SSD artifacts/issue-121/grid-maina8e985b-candidate/main-intake/commit-receipt.json; tests only, no production change |
| `tui.separator` | Existing SeparatorView plus public SeparatorContent → original scale-aware SeparatorView/Palette separator and decorative stroke; same registered root | Original retained View/Gallery Theme evidence kept; current ordinary Content9/public binding/scale/reading+marked/checked mount-retirement/strict passed; actual8 GPU frames256cases match original normal View fullROI/clip and all8 reviewed | Passive decorative leaf has no input/focus/AX; normal fractional allocation retains ancestor scissor, only overflowing ink gets allocation clip; curve remains Unsupported | Current source/program identities separate from original a1fail/a2 pass; no new native-window/physicalIME/VoiceOver/performance certification; #54 AgentKit pin remains | #54/#121/#113; SSD artifacts/issue-54/separator-content-main8de6c24-candidate/main-intake/commit-receipt.json; original #121 ceeee8a remains own identity |
| `tui.split-view` | Actual prepared SplitViewContent/OriginalDivider uses original SplitView/SplitViewView; divider separator/decorative-border and declared Surface to real Text/Button panes | Public both axes retained Theme during real capture preserves ratio/layout/clip/mount/focus and leaf semantic paint, Cancel restores; actual Gallery drag/Theme/pane-label paint then Escape/keyboard alternatives and strict passed | Current divider paints decorative separator with no own state color; ordinary ratio/constraints/hit geometry stays data; no invented semantic Splitter or focus palette | Original #103 B semantic Splitter/platform-wire/externalTheme/GPU main delivery remains missing, not waived; #12 current Split page repair separately verifies full pane/help/log order, stable extent and four-Theme full GPU drag pages; original bad pixels retained; no new native/IME/VoiceOver certification | #121/#113/#103; SSD artifacts/issue-121/split-view-main97bd424-candidate/main-intake/commit-receipt.json; initial mixed-label sampler failure retained, only corrected Gallery/strict rerun, byte-identical public pass reused; #12 separate repair: SSD artifacts/issue-12/split-page-capture-main4b574f2-candidate/repair/main-intake/green-repair01/visual-review.json |
| `tui.scroll-area` | Geometry/physics-only public ScrollArea alias and no-paint ScrollContent forward declared Surface/Theme to real Text/Button; actual dedicated Gallery route uses readonly TextSurface | Retained scroll offset/overscroll/range/child layout/viewport clip/mount across four Themes; exact child leaf roles; real public Gallery scroll route preserves historical offset/frame/thumb and actual thumb3/text4.5/7 over parent surface-1, strict passed | Container owns no Palette/thumb/interaction-color API; actual TextSurface owns Gallery thumb; ordinary offsets/ranges/thumb ratios remain geometry; shared leaf and original scroll/reveal/reading evidence reused | No new full editor/selection/IME/native-window/GPU/VoiceOver certification; toolbar separate | #121/#113; SSD artifacts/issue-121/scroll-area-main356ad1d-candidate/main-intake/commit-receipt.json; tests only, no new scroll framework or production change |
| `tui.disclosure` | Actual Disclosure/DisclosureView header/body and original Gallery native right/down chevron; Quiet palette/text/radius/focus and caller panel Surface | Retained four Themes closed/open/rest/hover/pressed/disabled/re-enable/focus, header/body real glyph4.5/7, body reveal/clip and original key/press/cancel; actual Workbench native chevron direction/ink/contrast3/header/body and strict passed | No builtin DisclosureContent or fake public Glyph alias; header and render_icon APIs separate, required real native icon covered in Gallery; disabled trigger does not invent caller-child policy; ordinary placement stays geometry | Original ContentHost/status/IME functionality evidence retained, not re-signed; no new GPU/whole-window/native AX/physicalIME/VoiceOver certificate | #121/#113; SSD artifacts/issue-121/disclosure-main6594bad-candidate/main-intake/commit-receipt.json; tests only, no production/shared-rule change |
| `tui.surface` | Existing feedback_view::SurfaceView -> surface_kind_at_scale/decorative_border uses Palette surface-1/surface-panel, separator, DS panel radius/inset/stroke; public TextContent owns real content ink | Same retained canvas/panel View + real TextContent/CoreText Theme::ALL fill/border/geometry/clip and4.5/7 contrast passed; actual Gallery panel heading/Theme and passive container versus nested Button action passed; Gallery strict passed | Passive Surface has no independent hover/action/focus/disabled state; decorative border is not a necessary control boundary. Ordinary geometry remains layout. #62 default/inherited/override/clear evidence reused | No new full-brand/GPU/native-window/IME/VoiceOver certification; Gallery variants prose is not another Surface render instance | #121/#62/#113; SSD artifacts/issue-121/surface-maine90220b-candidate/main-intake/commit-receipt.json; no new Surface model/API or generation changes |
| `tui.toolbar` | Real public ToolbarControl/ContentHost uses toolbar action-quiet packets; actual Gallery SurfaceView+ToolbarView uses first Primary/rest Quiet; widgets::Toolbar stays geometry-only | Retained four Themes preserve action IDs/selection/availability/geometry; exact own surfaces and title4.5/7/rest/hover/press/focus; real command7 and grouped overflow42 once-only payload/mount; actual Gallery Primary/Quiet states and Rename/Create pointer/key/cancel plus strict passed | Different original visual implementations remain distinct; no color API added to geometry helper; explicit original Host action-queue opt-in is fixture setup; widths/gaps/indices stay data | Original #69 Host/overlay/window/IME/image/GPU evidence reused at original identity; no new #127 full-combination/native/IME/VoiceOver/GPU certificate | #121/#113; SSD artifacts/issue-121/toolbar-main511badf-candidate/main-intake/commit-receipt.json; public initial101 retained, only one-line queue opt-in/public rerun; Gallery/strict original passes not repeated |
| `tui.text` | Primary registered TextSurface/real Page::Text TextDemo::text_only, plus original TextContent adapter; semantic Field surface-1/primary/selected/text-selection and original roles/metrics | Retained four Themes preserve readonly source/selection/clipboard query/reflow/scroll/native payload; actual normal and selected CoreText4.5/7 on own ground and actual alpha-composited selection; minimal TextContent semantic UiBody/Canvas path and real Gallery route plus strict passed | Source-over uses actual encoded-sRGB premul/non-Srgb target contract and emitted alpha, not a new color engine/GPU pixel claim; new copy coverage is selection query, old copy-command/native dispatch retains old identity; ordinary geometry stays data | No new dedicated external Theme/GPU/whole-window/physicalIME/VoiceOver certification or full editor requirement; original broader products retained | #122/#113; SSD artifacts/issue-122/text-main6594bad-candidate/main-intake/commit-receipt.json; false preflight unused-trait removal caused E0599, original import restored, only public rerun; Gallery/strict original passes retained |
| `tui.icon` | IconView consumes Palette primary-text/size-icon-preview; explicit tint is caller-owned; actual Gallery catalog uses production native adapter | Same public View/canonical SVG backend and real catalog adapter switch Theme::ALL with actual tint/geometry/clip checks | Pure Icon has no interaction/disabled/text model; caller tint is explicit. Unified Button image/artwork is a separate action owner. Shared full-catalog/brand/GPU results reused | No new full catalog/cache branch, dedicated external Theme/GPU/native-window/physicalIME/VoiceOver or text-contrast certification | #123; SSD `artifacts/issue-123/icon-main7a0f6e7-candidate/main-intake/commit-receipt.json`; actual catalog preview parameters, no fake image handle |
| `tui.button` | `Button → ButtonModel → existing Palette/declared Surface` | Unified public shapes, original state/center/cancel, real Gallery regressions and finite Metal/CoreText passed; exact round identities retained | DS Regular artwork/shared size explicitly replaces old compact/caller20px; ordinary geometry is not tokenized | Full AppHost/native/Gold7 and existing Dialog/Scroll/marked-IME failures remain separate | SSD issue-136/unified-button-main36dd132-intake/root-review.json; previous issue-113 receipts remain historical; #113/#136/#69 |
| `tui.icon-button` | Existing unified Button::new(real ImageId) owns chrome/DS sizing/opacity; caller owns image pixels; Gallery original decorative slot receives semantic action ink | Same registered-image Button with real Metal and actual Gallery Quiet/native Plus switch Theme::ALL; representative hover/press/outside-cancel/focus/disabled and public image disabled/loading pass | One fixture uses update_image with retained ID, not automatic image recoloring or a new app lifecycle requirement; approved DS Regular sizing and original behavior/GPU scale evidence reused | No new external Theme/native-window/physicalIME/VoiceOver/Gold7 certification; shared original product gaps stay in their original Issues | #123/#113; SSD artifacts/issue-123/icon-button-mainc4d956b-candidate/main-intake/commit-receipt.json; original unified-button-main36dd132-intake/round04,round05,gpu05 |
| `tui.link` | 公共 Link → LinkView::render → link_underline → Palette：c-link-foreground/hover/pressed、disabled-text；原 focus outline | 当前 main 批次：同一 View/model 保持 normal/hover/pressed/disabled/focus 并切换 Theme::ALL；真实 Gallery Link states 文字 ink/换肤及 strict 通过 | 原 View 布局与几何不新增 token 要求；共享生成/对比守卫未变 | 本批未新增 GPU/外部 Theme/原生结论；#114 已准确投影该有限范围，族中其他根未完成 | SSD artifacts/issue-123/link-main113c0f9/{round01,round02,commit-receipt.json}；#123/#113/#114 |
| `tui.text-field` | Primary registered single-line TextSurface/TextInput and actual TextFieldContent adapter with genuine text-host services; actual Gallery component-text-field route | Retained four Themes/value/selection/geometry, actual input/readonly ground/radius/primary-disabled ink4.5/7 and invalid boundary3, declared focus/error/readonly/disabled/re-enable/edit/SetValue refusal; real Gallery value/query/error/disabled/empty unfocused placeholder then edit and strict passed | No standalone TextField or public placeholder invented; original Gallery-only placeholder path retained; Surface has no independent hover/pressed skin; Field metadata matrix reused separately; geometry stays data | No new dedicated external Theme/GPU/native-window/physicalIME/VoiceOver/copy-command certification; remaining controls separate | #122/#113; SSD artifacts/issue-122/text-field-main2c6d7db-candidate/main-intake/commit-receipt.json; tests only, TextSurfaceHost trait and original text services opt-in preserved |
| `tui.text-area` | Actual multiline TextSurface ComposerBody; existing TextFieldContent configured with multiline surface/fixed viewport; retained Gallery TextDemo area | Public multiline selection/scroll/value/geometry across four Themes, actual glyph/selection composite4.5/7, focus/scrollbar3, multiline editing; real Gallery retained selection/scroll/caret and edit passed; strict passed | Ordinary layout numbers remain geometry; LIGHT primary/selected role alias handled by actual visible selection paint rather than color identity; shared TextField state/AX/font/IME evidence reused | No new external Theme/GPU/native-window/physicalIME/VoiceOver certification; historical failed sampler preserved, remaining roots separate | #122/#113; SSD artifacts/issue-122/text-area-mainfc261e4-candidate/main-intake/selection-oracle-correction01/commit-receipt.json; tests only, actual original multiline paths |
| `tui.search-field` | Actual single-line TextSurface via existing TextFieldContent::inline Search; real Gallery TextDemo search wrapper with original Clear query Button/empty unfocused placeholder | Retained inline query/Theme/geometry/semantic name and actual glyph4.5/7, representative edit/delete/reinsert; real Gallery query persistence, pointer Clear22 once, empty placeholder4.5/7/focus suppression/new input plus strict passed | No Search-specific API/results list/search icon/public placeholder invented; shared full TextField state/AX/font/IME matrix reused at33b74b8, ordinary inline frame stays data | No new dedicated external Theme/GPU/native-window/physicalIME/VoiceOver/filter-engine/copy-command certification; remaining controls separate | #122/#113; SSD artifacts/issue-122/search-mainfc261e4-candidate/main-intake/commit-receipt.json; tests only, Clear is actual Gallery pointer action rather than programmatic fake |
| `tui.combo-box` | Actual extended ComboBox + ComboBoxView/FilteredMenuView; existing caller-owned combo_status Scenario with real TextFieldContent/Host; Gallery same original View/TextDemo | Same popup/query/source-index/active-vs-commit retained across four Themes, actual query/active own-ground ink4.5/7; pointer once/repeated keyboard/Escape and disabled-arm/empty refusal; real Gallery native query/AX/source mappings/key and pointer commits/empty/cancel plus strict passed | No ComboBoxContent/new icon/filter engine invented; ordinary geometry and caller-owned composition preserved; unchanged TextField/Menu/Select/IME matrices reused | No new external Theme/GPU/native-window/physicalIME/VoiceOver certification; original source-specific functional evidence retained | #122/#113; SSD artifacts/issue-122/combo-box-main9048146-candidate/main-intake/commit-receipt.json; tests only, filtered positions never renumber action IDs |
| `tui.slider` | Existing public Slider/SliderView and actual Gallery consume surface-3/accent/hover/pressed, surface-0/border/focus and actual primary-text Value | Retained four Themes/value/geometry, actual active-rest and thumb-fill-or-boundary contrast3; pointer hover before focus/drag/cancel/live value, key bounds, disabled/window cancellation/re-enable and real Gallery Value4.5/7; original slider/AX plus disabled traversal and strict passed | Public Slider has no text/Content API; text belongs to original Gallery readout; ordinary value/track geometry stays data; cancel ends capture without rolling back live value | No new external Theme/GPU/native-window/IME/VoiceOver/motion certification; other roots remain separate | #122/#113; SSD artifacts/issue-122/slider-maina8e985b-candidate/main-intake/commit-receipt.json; Slider-only real-focus-loss/disabled blur guard preserves unfocused hover, Switch unchanged |
| `tui.checkbox` | Existing Checkbox::render -> paint::checkbox / original ButtonModel; Palette checked ground/hover/pressed/mark, primary/tertiary text, border/focus and DS indicator metrics | Retained Unchecked/Checked/Mixed switch Theme::ALL with real own indicator/mark/label contrast3/4.5/7; public and actual Gallery representative hover/press/cancel/Space-release/Enter-refusal/focus/disabled/re-enable passed; Gallery strict passed | Ordinary mark/row geometry stays layout; disabled tertiary text not claimed as active contrast; Gallery label uses actual parent or real renderer clear, no inserted scene | No new dedicated external Theme/GPU/native-window/physicalIME/VoiceOver/motion-timing certification; original AX/shared behavior evidence retained | #122/#113; SSD artifacts/issue-122/checkbox-main0a389a9-candidate/main-intake/commit-receipt.json; no production changes |
| `tui.radio-group` | Original RadioGroup/RadioGroupView -> radio_with_state consumes semantic checked/unselected/hover/press/disabled roles, DS text/row/stroke and focus | Public retained selected/unselected/item-disabled Theme::ALL, dot3/text4.5/7, representative cancel/roving-skip/group-disabled/re-enable passed; actual Gallery same paths and all affected radio regressions/strict passed | Ordinary circle/row geometry stays data; disabled-text not active contrast; real parent/renderer clear. Float dot containment uses bounds, no assertion/threshold removed | No new external Theme/GPU/native-window/IME/VoiceOver/motion-timing certification; original shared inputs/AX evidence retained | #122/#113; SSD artifacts/issue-122/radio-group-maine90220b-candidate/main-intake/commit-receipt.json; fixes Gallery draw erasing unfocused hover while preserving real blur/disabled guards |
| `tui.switch` | Existing Switch/SwitchView labeled renderer consumes semantic on/off/hover/press/disabled ground/thumb/text, DS text/stroke and focus | Same public and real Gallery on/off Theme::ALL, actual thumb/track or necessary-boundary3 and label4.5/7, unfocused hover, cancel/Space-release/Enter refusal/focus/disabled/re-enable passed; Gallery affected switch/AX/unique-hit regressions and strict passed | Ordinary track/label geometry stays layout; disabled ink not active contrast; off thumb uses its real boundary, not invented contrasting fill. Reduced-motion endpoints do not certify animation timing | No new dedicated external Theme/GPU/native-window/IME/VoiceOver certification; old baseline bug was source-established, not claimed as executed red | #122/#113; SSD artifacts/issue-122/switch-main9f827c2-candidate/main-intake/commit-receipt.json; minimal Switch draw guard preserves hover, Slider and real blur/disable guards retained |
| `tui.segmented-control` | Public extended::SegmentedControl / SegmentedControlView and actual Gallery original host; Palette row-selection/hover, surface-2, Quiet Button semantic text/radius/stroke and border-focused | Retained four Themes preserve selected/enabled/model/geometry; own text4.5/7 and selected boundary3, pointer hover/press/cancel, Space release, disabled capture/re-enable, keyboard skip/roving/focus plus original Gallery segmented/AX regressions and strict passed | Ordinary equal-cell/index geometry stays layout; disabled text exact role without active contrast threshold; shared Button/token/brand/scale evidence reused | No new dedicated external Theme/GPU/native-window/IME/VoiceOver/motion certification; other family roots remain separate | #122/#113; SSD artifacts/issue-122/segmented-control-main8253390-candidate/main-intake/commit-receipt.json; only Gallery pointer-modality retention and consumed-key host-focus sync changed |
| `tui.field` | Actual Field metadata/feedback_view::FieldView label/help/error/required plus input-slot allocation; original semantic text roles/spacing and real Gallery emphasized form | Retained four Themes help/error/required public metadata glyph4.5/7/slot geometry/error precedence; actual Gallery real Error property toggles original metadata roles while input frame/value remain, strict passed | No FieldContent/editor/focus/disabled reducer invented; caller/Gallery owns transparent backdrop and actual TextDemo owns input; default and emphasized presentation geometry remain original data | Original required/disabled/readonly/native/editor evidence retains identity; no new GPU/native-window/physicalIME/VoiceOver Required publication certificate | #122/#113; SSD artifacts/issue-122/field-main2c6d7db-candidate/main-intake/commit-receipt.json; tests only, metadata paint interval excludes unrelated scene text without ink filtering |
| `tui.select` | Actual Select/SelectView Primary trigger and external-model MenuView popup; original semantic trigger packets/surface-popover/row/primary-text/focus-border | Retained four Themes with same open popup preserve committed index separately from active/focused row; real trigger states/options own-ground4.5/7/ring3, disabled/re-enable/Enter/outside guard/Escape/inactive and actual Gallery AX/value/log/focus restoration plus strict passed | No builtin SelectContent or chevron/check artwork invented; standalone MenuEvent/model index is action contract, no Host journal; ordinary widths/indices stay geometry; shared motion/Menu results reused | No new dedicated external Theme/GPU/native-window/physicalIME/VoiceOver certificate or #127 full combination; remaining controls separate | #122/#113; SSD artifacts/issue-122/select-main2c6d7db-candidate/main-intake/commit-receipt.json; tests only, no production/API change |
| `tui.number-field` | Actual NumberFieldContent editable numeric controller through existing TextFieldContent/Button; Gallery NumberFieldView bounded stepper | Public valid/incomplete/invalid drafts retain value/focus/validation/mount/Theme/geometry and actual ink4.5/7/boundary3; Submit/Step/Escape changes; real Gallery normal/upper/disabled Theme/state/negative pointer and decrement, original pointer routing and strict passed | Disabled-only text exception preserved; ordinary numeric layout remains geometry; Gallery explicitly remains noneditable stepper | No new parser/locale/IME/GPU/native/VO certification; original shared/TextField/parser results retain identities | #122/#113; SSD artifacts/issue-122/number-field-mainfc261e4-candidate/main-intake/commit-receipt.json; real existing Disabled property/action10 exposed and release routed before stepper adapter |
| `tui.tabs` | TabsView→共享Quiet Button→Palette的row-selection/row-selection-hover/surface-2/button-hover/button-pressed及文字/焦点角色 | 同一实例四Theme选择/hover/pressed/disabled/focus、拖出取消及键盘；真实Gallery路由选择、换肤、正文4.5/7与panel保留已验 | selected Quiet按原共享契约复用selected-hover；普通几何不token化 | 无新增external Theme/GPU/native/物理IME/VoiceOver认证；不声明侧栏跨scope hover退出通过 | #123；SSD `artifacts/issue-123/tabs-main5634d2a-candidate/main-intake/commit-receipt.json`；main f7f43fe加本组件差量 |
| `tui.breadcrumb` | BreadcrumbView→共享Quiet Button→Palette surface-2/hover/pressed、primary/tertiary-text及焦点角色 | 同一实例四Theme必要状态、真实CoreText文字/背景4.5/7、current不激活/disabled/键鼠保护；真实Gallery保留路径与导航已验 | current沿既有noninteractive/tertiary-text且实测可读，无独立色角色要求；共享brand结果复用，普通几何不token化 | 无新增external Theme/GPU/native/物理IME/VoiceOver认证；不声称窗口外鼠标移动会路由至控件 | #123；SSD `artifacts/issue-123/breadcrumb-main5634d2a-candidate/main-intake/commit-receipt.json`；main fc7272b加本组件差量 |
| `tui.command-palette` | CommandPaletteView Primary trigger + FilteredMenu/MenuView rows; real host-owned TextSurface query consumes Palette and existing metrics | Same public Scenario/View and real Gallery retain query/active/selection across Theme::ALL; actual query/row glyph contrast4.5/7, execution/empty/disabled/focus return verified | Query is host-owned, shared Button/text/brand behavior reused; ordinary popup geometry stays layout | No new dedicated external Theme/GPU/native/physicalIME/VoiceOver/motion-timing certification | #123; SSD `artifacts/issue-123/command-palette-main7a0f6e7-candidate/main-intake/commit-receipt.json`; main3b6d460 plus owned tests |
| `tui.list` | Existing extended::List + collections_view::ListView consume Palette row-selection/hover/surface-2, primary/tertiary text and border-focused; Gallery uses real retained adapter | Same public instance across Theme::ALL preserves selected/hover/focus/disabled/re-enable and real CoreText row contrast4.5/7; actual Gallery selection/hover/focus, five ListItem semantics and Home/End passed; Gallery strict passed | General heterogeneous ListContent is non-selecting layout, no invented states; Gallery has no List disabled toggle, public set_disabled is verified; disabled ink not active-text contrast | No new external Theme/GPU/native-window/IME/VoiceOver certification; accepted Row/List/Settings and shared selection regressions retain original identities | #124/#113; SSD artifacts/issue-124/list-main0a389a9-candidate/main-intake/commit-receipt.json; production unchanged |
| `tui.tree` | Existing extended::Tree + TreeView consumes selected/hover/disabled row and text/focus roles; actual Gallery render_icons -> IconView -> native chevron renderer | Public retained Tree/View Theme::ALL selection/hover/node-disabled/group-disabled/restore, actual CoreText4.5/7 and hierarchy/navigation passed; actual Gallery real chevron3, collapse/expand geometry and AX parent/level/hidden children passed; strict passed | Node identity/depth/layout remains data; disabled ink not active contrast, no fake renderer or selecting passive TreeContent; no Settings/Disclosure/#38 expansion | No new dedicated external Theme/GPU/native-window/IME/VoiceOver certification; shared original Row/List/Tree behavior retains identity | #124/#113; SSD artifacts/issue-124/tree-maine90220b-candidate/main-intake/commit-receipt.json; production unchanged |
| `tui.data-table` | Actual extended::DataTable / collections_view::DataTableView and Gallery collections adapter consume surface-2, row-selection/hover, primary/tertiary text and border-focused | Retained public four Themes keep original row IDs/order/selection/geometry, actual header and readonly-cell own-ground contrast4.5/7, hover/focus/disabled/re-enable/navigation/sort; real Gallery sort identity/Table/Column/Header/readonly Cell semantics and strict passed | Caller owns row order/ordinary geometry; disabled Quiet header uses tertiary-text while readonly information cells remain primary-text; no DataTableContent or new Gallery disabled toggle invented | No new dedicated external Theme/GPU/native-window/IME/VoiceOver certification; Markdown/CodeBlock/DiffView remain separate | #124/#113; SSD artifacts/issue-124/data-table-main8253390-candidate/main-intake/commit-receipt.json; tests only, existing production/contracts unchanged |
| `tui.chart` | ChartContent → original ChartView → Theme/Palette/Surface | View `3bd9f00` plus current #92 main intake: public Host, default/external Metal and original paired focus/blur gate passed | 普通几何/索引数字不要求token化；normal /1 不声明 /2 sidecar | 原生窗口/发布未验；不转签旧Root证据 | chart-content-20261024.md；SSD chart92-main909130a-intake；#104/#92/#124 |
| `tui.menu` | MenuContent Quiet trigger / MenuView rows consume Palette; actual Gallery Primary invoker uses its hovered semantic role while popup owns input | Retained mounted/open instance switches Theme::ALL; actual title/item glyph contrast4.5/7, checked/disabled/input/focus/action contracts and real Gallery navigation verified | Application invoker variant/state differs from MenuContent default; shared Button/brand results reused, ordinary geometry remains layout | No new dedicated external Theme/GPU/native/physicalIME/VoiceOver certification | #123; SSD `artifacts/issue-123/menu-main7a0f6e7-candidate/main-intake/commit-receipt.json`; main9fb plus owned test delta |
| `tui.tooltip` | 公共 Tooltip → TooltipView::render → Palette surface-3 / primary-text；proposal radius-floating 与 UiBody | 本地 main 批次：保留 View 切换 Theme::ALL；真实 Gallery hidden/hover/focus-visible/Escape 矩阵的两处提示背景、正文 ink 与标准/增强文字对比通过；Controls/Gallery strict 通过 | 普通定位、宽度及内边距为原布局几何；Tooltip 无独立 pressed/disabled 外观状态；共享生成输入未变 | 本批未新增 GPU/外部 Theme/物理输入/VoiceOver；动态行帮助原生缺口留 #35；全族仍未完成，机器投影仅汇总本有限范围 | SSD artifacts/issue-123/tooltip-maincd27d0b/main-intake/commit-receipt.json；#123/#113/#114/#35 |
| `tui.popover` | 公共 Popover → PopoverView/render_with_content → Palette surface-popover、radius-floating/scale hairline；正文由公开 callback 消费 Theme primary-text | 当前本地 main：同一 model/View 保持 open 跨 Theme::ALL 与 scale1/2；真实 Gallery closed/open/focus-visible 样本的两处浮层、独立于 Dismiss 的正文 ink 及普通4.5/增强7对比通过；Controls/Gallery strict 通过 | 原定位/尺寸为普通布局；Dismiss 复用原 Button 语义与共享检查，未重复全排列；正文 callback 由消费者持有 | 本批无新 GPU/专门外部 Theme/native/physicalIME/VoiceOver结论；原 lifecycle/焦点恢复/外部点击/Escape 证据保留身份；全族仍未完成，机器入口按实际投影范围记录 | SSD artifacts/issue-123/popover-main1ca-candidate/main-intake/commit-receipt.json；#123/#113/#114 |
| `tui.dialog` | retained公共DialogView→Palette的surface-popover/scrim/primary-text/radius；原ContentTheme Surface继承沿8e4c01d | 同一公共实例四Theme切换、可见/焦点/禁用恢复/Cancel归焦，真实Gallery同实例四Theme、标题glyph及4.5/7对比已验 | 复用共享Button hover/pressed；普通几何不token化，2x装饰发丝线沿既有绘制 | 未新增专门external Theme/GPU/native/物理IME/VoiceOver认证；原功能缺口仍归原票 | #123/#62/#97；SSD `artifacts/issue-123/dialog-main45f1cf1-candidate/main-intake/commit-receipt.json`；实际main5634d2a加本组件测试差量 |
| `tui.toast` | Original ToastContent/ToastView via MountedToast retained ContentHost and inherited canvas Surface | Prior #79 public/mixedHost/lifecycle/default-external GPU/144 Gallery evidence retained; new same mounted Gallery fourTheme full model/action slot/clock/geometry, own input ground text4.5/7/marker3, actual advance timer and once-only Dismiss plus strict passed | Original success fixture, no allTone/brand permutations; actual ContentHost inherited canvas overrides bare View panel fallback; ordinary geometry stays data | No new GPU/native/physicalIME/VoiceOver/performance certification; old states not present in actual Toast API not invented | #125/#79/#113; SSD artifacts/issue-125/toast-maind706eca-candidate/main-intake/commit-receipt.json; tests only, initial Surface-oracle failure retained with original source/program |
| `tui.banner` | Original BannerContent/BannerView and inherited Surface; shared DS ButtonModel natural label measurement | Original #79 public/Gallery/default-external GPU retained; new #12 real CoreText320pt/type150 red confirmed92<102.91614, fixed inline natural width; four narrow/stacked/wide/scale cases, full message/glyph baseline, original cancel/once Dismiss, six owned controls/two consumer regressions and strict passed locally | Original layout anchors/message allocation/stack threshold and shared Button/Theme unchanged; no Dismiss-specific branch or new size policy | New original320 Gallery GPU/native page/VoiceOver/IME/performance not rerun; #12 other page/Progress/Split gaps remain | #12/#79/#125/#113; SSD artifacts/issue-12/banner-label-main4b574f2-candidate/main-intake/commit-receipt.json; local repair after tag14, old failed source/program and old GPU retain identity |
| `tui.status-badge` | Existing StatusBadgeContent/View/solid painter consume original four status-badge rest purposes, c-status-badge-foreground and inherited Surface | Existing 8fb1909/23447d2 public/default-external/GPU/readability/Gallery acceptance retained; new same ContentHost four tones x Theme::ALL and actual public Gallery route confirm semantic solids, label4.5/7, solid-parent3, pill geometry and passivity; strict passed | Passive badge has no hover/pressed/focus/disabled state; ordinary pill/caller geometry remains layout. No replay of original brand/seven-Surface/external/GPU matrices | Native-window/VoiceOver/IME/performance and arbitrary narrow-allocation limits remain original; no new certification or old-program re-signing | #76/#125/#113; SSD artifacts/issue-125/status-badge-main9f827c2-candidate/main-intake/commit-receipt.json; source-reuse.json preserves exact/full versus bounded formatting-only identities |
| `tui.progress` | Progress painter/View/ProgressContent → progress-track/rest via Palette/Surface | 本地main基线63214be接收批：公共消费者、原Gallery、默认/外部Theme GPU与strict已验；证据保持各轮原身份 | 父布局提供有限尺寸；未知进度保留静态片段，不新增动画 | 原生专用role/VoiceOver属#5；#12窄150%导航已另按实际DS自然测量修复，原24整页GPU、真实导航/滚动/Reset及strict通过，旧坏图保留；更广原生/整页不因此完成 | SSD `artifacts/issue-136/progress72-main-63214be-intake/`；#71/#72/#125；#12 `artifacts/issue-12/progress-navigation-main679db39-candidate/green-repair01/` |
| `tui.empty-state` | `EmptyStateContent → original EmptyStateView/Button → Surface` | local main: original action/geometry, stale authorization, mixed Composer/reading, default/external GPU and original Gallery body passed | Parent supplies finite allocation; original View owns action and layout | Native/VoiceOver/physical IME/performance/wire and other roots remain unverified | SSD empty78-mainb4fbed1-intake/root-review.json; #78/#125/#136 |
| `tui.skeleton` | `SkeletonContent → SkeletonView → skeleton_on_surface → Surface` | local main: passive/named semantics, mixed Composer/reading, default/external GPU and original Gallery route passed | Parent owns extent; original radius4 retained; static, no ticker | Native/VoiceOver/physical IME/performance and other roots remain unverified | SSD skeleton77-main23447d2-intake/root-review.json; #77/#125/#136 |
| `tui.markdown` | Existing Markdown::parse and MarkdownView/PassiveCollectionView; transparent primary-text block paint with original shared text roles, actual caller DocumentCanvas | Retained four Themes and supported heading/paragraph/bullet/fenced-code IDs/order, actual narrow/wide wrap/clip/passivity and each-block glyph/ground contrast4.5/7; actual registered Gallery adapter keeps blocks/footer/input/lifecycle and strict passed | Original readonly subset, no invented Content/edit/focus/disabled API; transparent component uses caller Quad in consumer and exact Gallery DocumentCanvas clear input, not an owned surface or pixel claim | No new external Theme/GPU/whole-window/native AX/IME/VoiceOver certification; broader editor/full-document goals unchanged; DiffView remains separate | #124/#113; SSD artifacts/issue-124/markdown-maineb94677-candidate/main-intake/commit-receipt.json; tests only, original parser/render unchanged |
| `tui.code-block` | Existing CodeBlock/CodeBlockView/PassiveCollectionView and actual registered Gallery collections adapter; code-surface/primary-text/separator/c-code-radius and original text metrics | Same public model/View four Themes at narrow/wide widths keeps source IDs/wrap/empty line/clip/passivity and real each-line contrast4.5/7; actual Gallery adapter retains source/layout/footer separation and rejects unrelated input; original code_block regressions and strict passed | Plain read-only source, no existing hover/press/disabled/focus/edit/syntax or Content API invented; language is caller data; geometry/indexing remains data | Adapter component build is not whole-window/native AX certification; no new external Theme/GPU/IME/VoiceOver certification; Markdown/DiffView separate | #124/#113; SSD artifacts/issue-124/code-block-mainbf518c0-candidate/main-intake/commit-receipt.json; tests only, production unchanged |
| `tui.diff-view` | Existing DiffView::between/LCS and DiffViewRuntime/Presenter; transparent Code-role primary-text/status-succeeded/status-failed over actual caller DocumentCanvas | Retained four Themes preserves all Equal/Removed/Added IDs/order/full prefixes and real CoreText glyph colors/ground contrast4.5/7; narrow150%/wrap/clip/empty/passive cancel and actual registered Gallery rows/footer/input/lifecycle plus strict passed | Readonly plain diff, no invented Content/editor/hover/focus/disabled API; transparent parent Quad in public, exact Gallery clear input in adapter, no pixel claim; ordinary LCS/index/geometry data | No new dedicated external Theme/GPU/whole-window/native AX/IME/VoiceOver certification; original broader product targets unchanged | #124/#113; SSD artifacts/issue-124/diff-view-maineb94677-candidate/main-intake/commit-receipt.json; tests only, real recording renderer delegates all measurement and painting to existing CoreText |
| `pattern-form` | Actual Gallery form composition using TextDemo/TextSurface, FieldView/Button/checkbox preference and public DraftJournal adapter; no FormContent | Real empty-invalid/edit/persist/Save-Pending/Failed/interrupt recovery, same failed draft revision/selection/preference across four Themes with actual input/selection/action/metadata ink4.5/7, retry same revision2/ack and real disabled blocking passed; strict passed | Parent composition test reuses shared child state/font/IME/Journal/GPU results; ordinary layout stays data; Pending/disabled Save uses declared disabled roles | No new full editor/native/physicalIME/VoiceOver/GPU certification; other patterns remain separate | #125/#113; SSD artifacts/issue-125/pattern-form-main9048146-candidate/main-intake/commit-receipt.json; only one real Gallery test hunk and temporary explicit fixture, no production API |
| `pattern-filter` | Actual Gallery Workbench filter route: original TextDemo/TextSurface input, Quiet Clear and deterministic passive file-result labels | Same query/results/child geometry through four Themes; NOTES two exact files, no-match message, real once-only Clear then all three; actual result glyph identity/order/count and own input/result/action ink4.5/7; strict passed | No FilterContent/search engine/checkbox/tab/selected-result state invented; original narrow/wide/150% layout and shared Text/Button checks reused | No new GPU/native-window/physicalIME/VoiceOver/performance certification; actual Workbench component route test is not native-window acceptance | #125/#113; SSD artifacts/issue-125/pattern-filter-mainab5869c-candidate/main-intake/commit-receipt.json; one real Gallery Workbench test with fixed expected filenames, no copied filtering algorithm |
| `pattern-master` | Actual Gallery Master with 80 ButtonModel rows, independent ScrollArea and read-only TextDemo/TextSurface detail; not ListView | Original real scrolled-row selection/clip retained; added full selected Document3 detail binding and actual pointer-focus read-only assertion passed at final source; strict passed | Shared Button/TextSurface/ScrollArea Theme and original keyboard/reveal results reused; no copied full List/Theme matrix or invented ListItem semantics | No new native-window/physicalIME/VoiceOver/performance certification | #125/#113; SSD artifacts/issue-125/master-danger-scene3d-final-intake-20261028/master-commit-receipt.json; original scenario kept, only precise detail assertions/focus added |
| `pattern-async` | Actual nonstore Gallery async through shared form/settings/async TextDemo/Field/Button render and original submit/resolve controller | Real native edit/preference, Ready/Pending/Failed/retry Saved with Theme changes preserving text/selection/caret/phase/attempts/geometry, no durable-store/recover acquisition, and real Reset passed; strict passed | Actual shared consumption/necessary own-ground contrast reused from Form ab5869c, no duplicate child matrices or invented AsyncContent/store | No new GPU/native-window/physicalIME/VoiceOver/performance certification; old private W5 not current-main GPU evidence | #125/#113; SSD artifacts/issue-125/pattern-async-mainfa8bcc0-candidate/main-intake/commit-receipt.json; one real Gallery test, original state/action semantics |
| `pattern-danger` | Actual Gallery dangerous-confirmation fixture through existing DialogView and original local submit/resolve controller | Existing named Cancel keeps Ready, Confirm/Resolve reaches simulated Failed passed at exact final source | Actual Dialog retained Theme/own-ground/focus/scrim and Form phase consumption reused; fixture explicitly never deletes files, no new destructive capability | No new native/GPU/IME/VO/performance certification; old private stdout retains old identity | #125/#113; SSD artifacts/issue-125/master-danger-scene3d-final-intake-20261028/danger-commit-receipt.json; no production delta or duplicate Dialog matrix |
| `example-settings` | Actual Gallery settings example through shared FieldView/TextDemo/Button/notification model; separate from #70 AppWindow Settings | Real edited name/preference/Save Failed instance retains values/phase/log/actions/geometry through four Themes and actual input/metadata Palette roles; strict passed | Shared Form ab5869c necessary own-ground contrast and original Gallery Error/Disabled/Reset/retry reused; no duplicate child matrix or SettingsContent/store invented | No new AppWindow/store/physicalIME/VoiceOver/native/GPU/performance certification; #70 original completed delivery not reopened | #125/#113; SSD artifacts/issue-125/example-settings-mainfa8bcc0-candidate/main-intake/commit-receipt.json; one actual Gallery Theme integration test, #70 evidence remains separate |
| `component-scene3d` | Actual Gallery Preview3D uses Theme chrome/background, public SecondaryText instruction and PrimaryText error roles; real GPU image from glTF scene data | Both original real-wgpu Gallery tests explicitly enabled and passed: image composition/animation/off-page stop and clipped camera input/stable GPU geometry/blur; real1280x900 capture retained; strict passed | Scene material/base/emissive/light/model colors are scene data, not UI tokens; UI semantic roles and existing shared Gallery Theme chain reused, no PBR expansion | No new full PBR/native-window/physicalIME/VoiceOver/performance/external-brand Scene3D certification; old ignored result remains not_run | #125/#113; SSD artifacts/issue-125/master-danger-scene3d-final-intake-20261028/scene3d-commit-receipt.json and scene3d-default.png; actual2 passed/0 ignored, not source-only inference |

<details>
<summary>早期有限投影批次（历史身份；当前范围以上方52根表和原Issue为准）</summary>

### 已接收的有限实证投影

现有 `component-coverage.json` 对 `tui.button`、`tui.chart` 和 `tui.progress` 增加了 `practical_intake`，生成器将其投影到各自的 `state_coverage.practical_intake`。 本批沿同一入口增加 `tui.link` 的有限语义消费、保留实例换肤和真实 Gallery 结果；原生候选已撤回，投影只取已接收 main 源码，旧状态与 maturity 不晋级。它记录原 Issue、接受提交、证据文件 SHA-256、明确范围、未声明范围，以及本次冻结基线下可核对的有限源码路径哈希。Chart 项保留历史 V5 证据，并加入 b763469 的 ChartContent/Host 及默认/外部成对 GPU 接收；当前7条源码路径仅是可核对子集，完整运行身份仍由冻结清单和原锁内收据约束。它只提供已接收证据的定位，不替代原状态矩阵、根验收或 maturity。

- Button 当前投影包括 #113 的 72 组公共 ink/fill 对比和实际 Gallery theme/brand 切换代表状态回归；GPU、原生/VoiceOver、完整 #136 统一Button接收／#69 Host/content 与全部 52 根仍未由这些记录证明。
- Chart 当前投影包括 #104 的有限 Theme/Surface 与 focus GPU 门禁；#92 Host/reveal/paired、#112 `/2` sidecar、native/IME/AX 和发布仍明确未证明。
- 证据 commit、receipt 和原始哈希保持原身份。源码哈希匹配只说明该有限证据输入仍对应当前投影源码；不把旧 18 组观察改签为当前证据，也不将 16 个输入差异解释成功能失败。
- 若投影源码发生变化，`practical_intake.status` 会变成 `accepted-source-subset-stale` 并给出逐路径差异；旧证据保留，但不再称为当前源码身份匹配。损坏的状态、路径、哈希或跨组件证据会被拒绝。

- Progress 当前投影引用本地 `3f13a1e` 的原公共组件/Gallery/实际GPU及严格检查，必要共享查询与平台投影已接收；专用原生role、VoiceOver、全Gold7及Gallery文案/窄宽布局仍未声明完成。
- 生成器核对源码子集哈希和字段结构；receipt字段是保留的不可变引用，不声称自动认证其内容。本次主协调另核实了全部五份receipt字节与记录SHA一致，记录在SSD `artifacts/issue-114/main3f13-intake/receipt-reference-check.json`。
- 当前main实际验证：48项Python测试、生成器一致性、公开API编译绑定与strict；Button/Chart/Progress机器入口查询。未重跑生产Rust/GPU；未把其余49根标complete。首批未包含的Dialog等记录由下节补齐；pattern/example继续保留原registry状态，投影完成不代表所有组件范围完成。

### 其余已接收 Theme 批次

以下记录复用既有 `practical_intake` 字段。`accepted-bounded-evidence` 标识原批次按其验收边界被接收；记录保留原提交和 receipts，生成器会在最终主线输入上逐路径比较源哈希。后续批次改动同一文件时会显示源码身份差异；这既不表示旧批次功能失败，也不把旧证据改签为当前源码验证。

| 组件入口 | 已接收范围 | 该记录不证明 |
| --- | --- | --- |
| `tui.dialog` | #62，`8e4c01de5fae9a429016a039f53034b5737a0a2b`：Surface 默认、继承、显式覆盖/清除及真实 retained Dialog 祖先链；公共 fragment/Dialog regressions 通过。 | 这条只投影 2026-10-26 Surface 批次；native/VoiceOver、ProgressContent 完整验收、Panel 450/18 集成。 |
| `tui.status-badge` | `StatusBadgeContent → StatusBadgeView → status_badge_on_surface → Surface` | local main: passive semantics, dynamic mixed Host, default/external Theme GPU, normal-label readability and actual Gallery verified | Parent owns allocation; narrow clipping does not promise full-text readability | Native/VoiceOver/physical IME/performance and other roots remain unverified | SSD status76-main437b5b6-intake/root-review.json; #76/#125/#136 |
| `tui.skeleton` | #83，同一提交：一个 rest-only shared Surface 用途及 boundary 对比守卫。 | 只证明共享规则与 handoff；不证明 #77 Skeleton Content/View/Gallery/GPU 消费。 |
| `tui.toast` | #93，`3e989f1a6d05466d157b60209d2194348ab131ce`：仅 rest-only `toast-warning` marker 共享 Theme 用途。 | 不证明 #79 Toast Content/View/Gallery/GPU 消费。 |
| `tui.banner` | #93，同一提交：仅 rest-only `banner-warning` marker 共享 Theme 用途。 | 不证明 #79 Banner Content/View/Gallery/GPU 消费。 |

#83/#93 的测试证明 shared Theme 规则、生成产物、Rust 校验与外部拒绝/合法消费边界；它们不替代组件 adapter acceptance。组件行把共享用途映射到便于查阅的消费入口，不等于该组件已消费这些用途。未在本批重跑旧 GPU，也未扩展 52 根状态。

本次补录后，#114按2026-10-26明确简化的四项AC完成：复用52根登记及原API/route/state/maturity；八个有界接收入口保留scope/claims/exceptions/gaps和证据；历史观察与当前源码不符有定位诊断；实际生成/context/公开编译绑定检查通过。新补录运行五个context查询、receipt字节核对及三项受影响Python检查，未重跑未变化的Rust/GPU。隔离检查曾发现examples源码未列入生成输入，现将既有source_identity路径纳入原输入清单并验证修复。完整组件主题接入、#8其余原AC、native/视觉成熟度均未因此完成；后续每批接收继续更新同一记录。

</details>

## #1 / #55 当前剩余验收矩阵（2026-11-02，接纳PR214–217有限公开证据）

本节是上述唯一52根表的父票验收说明，Refs #1 / #55；不新增组件登记、机器状态或成熟度。
原2026-10-30文档候选基线 `b4b1f92f472353a24febe3e968b7b47214922422` 保持历史身份；本次只读公开 `a96602d00db783b894fa34469c182125aecab8de`，接纳下列Main公开回执与原owner已验范围，不是新一轮产品验收。
#113/#114/#121–#125已完成原实用Theme范围，不重新实施Theme，不恢复owned闭包门禁。
#55历史S2/S4/S5中的“其余52根待迁移”、本表旧行中的“全族未完成”不能覆盖后来的实际收口；S6和原产品体验仍开放。
依赖落实到下表的场景或接口，不使用 #55→#1→#12→#55 的整票等待。

### 有效证据与身份边界

下列SSD相对路径均位于 `/Volumes/HP P900/DEV/TekesUI/`。复用条件是相关源码、配置、依赖、输入和契约未变；新提交号本身不使证据失效。发生差异时只补受影响项，历史结果及失败不改签。

- **T（52根实用Theme）**：`artifacts/issue-113/practical-all52-close-20261028/closure-receipt.json` 实列52根：Button1＋#121布局8＋#122文本输入13＋#123动作导航10＋#124数据内容7＋#125反馈组合13。收据 `accepted_head=4b574f2f114d46b44198bec17c842991a8f07017`，PR154／CI40582809137／公开交付 `5879fe88d98cc2038eec5bc83baf5498ddd7f0bc`／checkpoint-20261028-14。上文 `fc2f81a` 是另一阶段身份，不能冒充该收据的 accepted_head。每根具体源/程序/状态仍沿原行收据，不以汇总证明全部原生或Gold7体验。 #55历史details中CodeBlock/Markdown/Icon前景绕过和Chart用途待迁移，不能覆盖本表后续接收：Icon `aec642f`、Markdown `781f2ab`、CodeBlock `e5369fc` 的原public/Gallery/strict收据均exit0（路径见各原行），Chart沿#104/#92接收；这不扩展到整页或物理体验。
- **P（准确投影）**：#114闭合范围为45组件机器投影＋5patterns／2examples原route和maturity。`artifacts/issue-114/main3f13-intake/receipt-reference-check.json` 仅列Button／Chart／Progress的五份引用哈希，不是52份重跑证明。历史stale是来源差异，不是功能失败；不得从 `accepted-bounded-evidence` 自动晋级。
- **S（Settings初始Ready）**：[现有说明](../../design/reviews/settings-whole-page-20261030/README.md)、同目录 `execution.json` / `visual-review.json`；SSD `artifacts/issue-12/settings-main77860b5-20261030/run01/`。实际MAIN `77860b59df6eb8dc3432a96dd4bfd1730c1b215c` 加明确WIP，3469输入清单SHA `27ff7ba1cc1ff42f569c3314b35f89ba0279512f2e74d9d10e7e946005380e9a`，程序SHA `aea0f5944f6af79c3cd235402dcb0852771a339283de0bbd643577253867a5c7`。4主题×双倍率×两viewport/type×三位置=48 PNG均有逐图有限审查；窄窗offset 0/201/402，宽窗range=0。只限brand0、720×512/type150与1440×960/type100、初始Ready；不含其他状态、宽大字/窄正常字号、真实输入、焦点reveal、物理显示/IME/VO/性能。#12已记录PR191集成交接；本节不判定其后续CI/merge状态。
- **B（#12已有局部修复）**：Banner PR155／`ba78773` 保留320pt/type150 CoreText红绿及4个布局样本，不称新原生GPU整页。Split PR156／`2770493` 的 `artifacts/issue-12/split-page-capture-main4b574f2-candidate/repair/main-intake/green-repair01/` 保留四主题完整GPU页及逐图审查。Progress PR158／`5fdab2c` 的 `artifacts/issue-12/progress-navigation-main679db39-candidate/green-repair01/` 保留24生成图，其中实际审查8张；其余16张不改成已目测。均复用其原有效范围。
- **H（Gold7历史分层）**：[原Gold7规则](gold7.md)、`design/behavior/gold7.json`、`gold7-alignment.json` 的23要求及历史receipts保持原样。其旧 `missing/partial` 不能直接当当前实现缺失清单；T、后续#81 B2、#103 A+B等实际交付优先于旧实现摘要。后两票保留物理IME/VO/可见原生/性能边界，不能借它们宣布全Gold7完成。#57图标由其独立owner核对，本节不重验；Settings文件名不是Settings页面证明。

### #1 原八项AC与 #55 阶段对应

AC序号按 #1 原文顺序，原checkbox不更改。唯一owner表示接收该行结果的领域职责，不授予MAIN写权或构建/桌面窗口；主协调负责分配执行者和最终接收。

| 原AC / #55关联 | 已有效的证据 | 剩余缺口与最小补验场景 / 现有入口 | 唯一owner / 资源 |
|---|---|---|---|
| AC1 统一视觉强调、边界、状态、例外规则 / S1–S2 | T及现行Theme foundation契约、每根例外、Gold7原规则 | 设计规则已交付；体验发现具体偏离才回原组件票修复，不新开Theme迁移 | #1/#55父票覆盖领域 / 文档 |
| AC2 共享token/样式/API默认受益 / S3–S4 | T的真实公共消费；P保留准确来源 | 实用接入已完成；后续受影响组件仅补其公共默认入口，不要求52根重跑 | #1/#55父票覆盖领域 / 文档；变更验证归原组件owner |
| AC3 全组件、pattern/example映射、状态与证据 / S5 | T完整52根与本表，P准确45＋5＋2 | 覆盖组织已完成；不把旧族“未完成”作为新迁移任务，不把本节当成熟度通过 | #1/#55父票覆盖领域 / 文档 |
| AC4 Quiet/工具动作/Search权重与边界职责 / S6 | T动作/搜索消费、原Button接收；S仅Settings初始页 | 整页注意力顺序仍需实际体验：pattern-filter/playground输入/清除、工具动作、导航Search与主要Save同屏对照；复用旧有效局部图 | #12整页视觉领域 / 离屏GPU＋逐图审查；物理阅读另分窗口 |
| AC5 必要边界、文字/图标、错误/选中/focus普通与IC可辨 / S6 | T必要对比/状态、B、S有限阅读 | pattern-form/playground错误/disabled及真实Tab焦点reveal；component-menu/dialog和选择组件的普通/IC选中与焦点；图标消费引用#57原结论 | #12整页视觉领域 / GPU＋原生键鼠；#57保持独立 |
| AC6 自动语义/主题/状态约束与绕过防护 / S3 | T原正负例、P来源/投影约束；现行生成器及原实用AC | 已交付项复用；只有新/变更路径才跑对应自动检查，不因本文触发Rust/CI | #1/#55父票覆盖领域 / 文档核对 |
| AC7 四主题、代表品牌/尺寸、1x2x、Light/Dark整页前后与注意力 / S6 | T代表组件，B局部页，S两代表组合初始页 | 补下表视觉批次及适用物理显示；S没有非默认brand、失败/保存状态、宽大字/窄常规或全页族结论；旧golden保留并标源差异 | #12整页视觉领域 / GPU＋人工视觉＋物理屏幕 |
| AC8 当前候选设计决定、证据、历史图与使用指南 / S6 | 现行规则、原registry及T/P/B/S的身份边界 | 每次补验回填原收据/本表对应行；未验保持未验，原图不覆盖，原AC未全满足不关#1/#55 | #1/#55父票覆盖领域 / 文档 |

#55 S1/S3及S2/S4/S5的实用Theme交付复用T/P；S4中的原生/产品体验残余与S6按具体Gold7场景推进。父票不等待子票整票关闭才接收已完成项。

### Gold7 原23要求的剩余场景

ID沿 `gold7-alignment.json`，不改变其历史assessment。下表“缺口”是仍需取得的体验结果或明确能力核对，不是宣称所有现行实现缺失。契约、代码、GPU、程序化原生、物理输入、VO和性能分别记录；不将本次文档审计算作验收。

| 原要求ID | 可复用证据 | 缺口 / 最小补验场景、现有入口 | 唯一owner / 资源 |
|---|---|---|---|
| G1/warm-neutral-reading-foundation | T、S、H | 长正文普通/disabled在Light/Dark/IC的持续阅读与主次顺序；example-settings、pattern-master/playground | #12整页视觉 / 物理屏幕 |
| G1/consumer-brand-derivation | T共享品牌与隔离正负例；S仅brand0 | 在已有品牌预设中选代表非默认品牌，检查真实Menu/Dialog叠层及页面强调比例，记录选择理由；不重跑无变化品牌编译链 | #12整页视觉 / GPU＋逐图审查 |
| G1/whole-page-reading-composition | S、B、H几何检查 | Settings Pending/Failed/Saved，复杂表单/搜索/主从组合，窄常规/宽大字补代表对照；非空或动作不重叠不足以判阅读通过 | #12整页视觉 / GPU＋逐图审查 |
| G2/four-semantic-surface-roles | T Surface消费、H映射 | 同页canvas/region/panel/overlay层级，使用Settings与component-dialog/playground；核对实际语义映射后审图 | #12整页视觉 / GPU＋逐图审查 |
| G2/same-color-geometric-distinction | H原四主题1x2x浮层GPU、T | 相邻同色层级的真实边缘/阴影，component-menu/popover/dialog；物理屏幕观察不由旧栅格结果替代 | #12整页视觉 / 物理屏幕 |
| G3/hairline-and-required-boundary-roles | T必要边界/状态对比、B/S | form错误/required/focus与装饰线同屏，普通及IC；保留必要对比，不机械换separator | #12整页视觉 / GPU＋物理屏幕 |
| G3/component-role-radius-map | T各根角色/例外、H | 真实控件/面板/浮层/代码表面角色复核及物理阅读；只检查适用角色，不强行统一圆角 | #12整页视觉 / 物理屏幕 |
| G3/nested-radius-geometry | H公式及既有接入、T | 只对已声明同心的真实嵌套实例记录outer/inset/inner并看实际像素；普通相邻控件不当同心fixture | #12整页视觉 / GPU＋逐图审查 |
| G3/real-scale-rendering | H焦点/阴影1x2x、B/S | 实际1x/2x屏幕与代表尺寸的边界、焦点边缘；窄窗Tab到离屏项，检查祖先clip与reveal | #12整页视觉 / 物理显示＋原生键盘 |
| G4/named-press-expansion-snap-presets | H命名预设、#9有限交付 | component-button/switch/disclosure实际按压/展开/反向时序；保留原预设，不未经决定调参 | #12真实交互 / 原生键鼠 |
| G4/real-component-motion-integration | H原生产接线、原浮层帧 | 上述控件与Menu/Dialog/Popover进入/退出、中间帧和页面卸载；按适用通道，非要求每个被动组件动画 | #12真实交互 / 原生＋GPU帧 |
| G4/direct-manipulation-and-interruption | H、B Split、#103 A+B已交付 | component-split-view/playground拖动跨侧栏、丢捕获/失活/取消；可见窗口实测，hidden Window断言不代签 | #12真实交互 / 原生指针＋窗口 |
| G4/reduced-motion-path | H系统/renderer降级接线 | 系统Reduced Motion开/关下Button/Switch/Disclosure与浮层重开/取消，动作结果一致且无补播 | #12真实交互 / 系统设置＋原生 |
| G5/ime-and-suggestion-priority | T/H；PR214完整Field焦点边界；PR215 Native095的三组程序化composing barriers及Viewer正向控制有限通过 | pattern-form/search、component-combo-box/dialog的物理中日IME、真实候选窗/确认取消仍未验；095不是物理IME，也不改签为clean-public整M6 | #12物理输入；Media原#204消费验收 / 原输入法与候选窗环境、Main具名窗口 |
| G5/durable-draft-recovery | T pattern-form、H真实本地journal/outbox | pattern-form/playground显式TEKESUI_GALLERY_DRAFT_FIXTURE临时路径，编辑→提交→中断/重启→旧ack后新编辑保留；仅本地fixture，不冒充真实服务/断电 | #12真实交互 / 原生＋隔离fixture存储 |
| G5/manual-scroll-reading-anchor | H append-only anchor | 现有scroll路由/Page::ScrollArea阅读历史后append/resize/选区/嵌套滚动；不外推prepend/任意替换 | #12真实交互 / 原生滚动＋键鼠 |
| G5/explicit-return-to-latest | H真实Button回最新 | 同scroll入口手动到底不自动repin；点击/键盘显式返回后检查body焦点与后续append | #12真实交互 / 原生键鼠 |
| G6/zoom-preserves-reading-anchor | PR212已有MediaViewer公开入口；PR215有限Viewer焦点/普通Right控制；PR216 disabled Actual说明公开 | 不再登记为“无入口”。enabled Actual/真实viewport anchor仍按M8-A1验；说明重排后的whole-observation transform不变为NOT_MET，不由有限说明PASS补签 | #204原consumer验收（#201/#202原geometry oracle）/ measured viewport使f∈[1/8,1]、可启用Actual及新具名窗口 |
| G6/bounded-split-and-pan-geometry | B/#103原Split；Media公开pan及原GPU/Native095各自有限正向证据 | Split原场景继续由#12验；Media真实OS拖拽、界外释放、后续unheld move及border resize不由程序化Right或atomic drag单独证明 | #12负责Split；#204负责Media / D1已定义MacVec2坐标域与ownwindow端点、R1真实border几何；不重造引擎 |
| G6/cancel-and-non-gesture-alternatives | H/#103；Media Zoom/Fit/Close/普通Escape入口及原M1/M2/095有限证据已存在 | active-drag Escape/失焦/resize/source变动等held组合仍未验；post-dismiss旧native对象已有限接收，remount生命周期另列未验，不相互代替 | #12负责Split；#204负责Media、#205原observer协作 / 支持分开held Down/Move/Up的授权工具合同；remount需新parent/child窗口 |
| G7/reusable-real-component-contract | T/P、各公共consumer；#81/#103限定交付 | 每个拟晋级组件只补其原适用native/键盘/输入/AX结果；public编译和截图不等于原生体验 | #12成熟度验收 / 文档＋对应原生窗口 |
| G7/component-documentation-and-runnable-preview | 原registry route/API、T/P、H状态story | 对变更或拟晋级入口对照原公开API及可运行预览；程序化AX与VO分开；不自动将旧“bridge missing”作为新实现任务 | #12成熟度验收 / 文档＋对应原生窗口 |
| G7/business-combinations-are-real | T五pattern及Settings真实流程、S | 完整窄/大字form/filter/master/async/danger及Settings状态链的实际交互与阅读；Danger仅模拟失败不删文件，Async不冒充远端服务 | #12真实交互 / 原生键鼠；视觉交给#12整页视觉 |

### 可直接排期的最小批次与边界

执行领域按行唯一接收；主协调分配具体聊天/时间段，不由本矩阵争抢资源。#12视觉owner负责图像生成与判断，本父票领域只组织覆盖。

1. **离屏视觉续批 → #12整页视觉**：先Settings Pending→Failed→Saved，再pattern-form错误/disabled、pattern-filter非空/空/clear、pattern-master滚动选行详情、pattern-async重试、pattern-danger确认/取消。入口均为各slug的 `/playground`；原 `settings_full_failure_retry_and_danger_cancel_recovery`、`pattern_async_gallery_keeps_nonstore_draft_through_themes_and_real_retry_reset` 和 `pattern_filter_retained_themes_keep_real_query_results_and_clear_action` 提供状态/action参照。当前Settings专用GPU fixture只采Ready，component-only state fixture不含pattern/example，需该owner准备最小场景增量，不能直接声称已有采图覆盖状态。复用VIEWPORTS/POSITIONS和旧有效B/S；四主题、1x2x，明确代表品牌/尺寸与比较理由，新增结果逐图审查。完成该批无需等待物理IME/VO/性能。
2. **真实键鼠与TextCommand → #12真实交互**：上列页面加component-menu/dialog/select/combo-box及导航Search；实际Tab/Shift-Tab、方向、Escape、提交/失败/重试、剪贴板/选择的原生TextCommand，离屏焦点reveal、Split捕获和Reduced Motion。现有合成 `Event::TextCommand` 测试只作参照；不等于真实AppKit输入。需要明确原生/桌面窗口。
3. **物理IME → #12物理输入**：在第2批输入入口执行中/日候选窗、marked提交/取消和祖先Escape优先；单独记录输入法、系统、程序、操作和结果。需要物理输入窗口，程序化AX/Key/TextCommit不能代签。
4. **持续阅读/物理倍率 → #12整页视觉**：复用通过GPU页，补实际显示器倍率、注意力、边缘与焦点；不重生成无变化图。性能另由#12性能验收记录帧节奏/空闲CPU/持续滚动/适用120Hz硬件，截图或平均帧时间不能抵消未测。当前无获分配窗口，不执行。
5. **保持后置/外部边界**：VoiceOver按已确认延期保留未验，恢复需原协调决定，不能由本表自动启动；AccessKit、#131–#135、跨repo迁移不恢复。Media公共入口已交付，剩余验收沿下方原owner具体依赖，不重新创建组件或引擎；#54外部消费不占本仓实施线；#57由图标独立owner处理。它们均不阻塞前四批中已经具备接口的独立交付。

本轮实际执行仅原文/源码入口/收据核对和文档差量检查，无Cargo/GPU/native/桌面/CI；未判断新图、未晋级registry、未关闭父票。后续补验结果写回原Issue与原收据，本节同步具体行，不建立另一份动态台账。


## Search / Settings r2 兼容交付候选（2026-10-31，未发布）

Refs #199 / #207 / #1 / #55。固定基线为已发布PR200 `a883d00a29b98cdfb5262c23a2e728e6908384e0`；Search实现唯一采用原owner的 `pr200-standalone-source-20261031-r17/source-only.patch`，不再维护r1共享实现。r17的43路径在追加Settings前逐hash一致；仅host/main/workbench三文件增加Settings纯接线，另加两份原7414私有fixture，包含f569 observer与7414 BackTab修正。r1包、18差异及失败记录均冻结保留，r18独立后续测试不进入本批。

公共类型为原384加Search/Select七路径共391；实际24角色加SearchInput/Select为26。primary API、api_bindings、45组件/5patterns/2examples和experimental保持。历史Search611a/70ee的CPU/GPU、Settings7414/program2c6bf的Native02有限结果不改签为新26-role组合验收；新peer/schema、bridge/consumer/strict等需原owner按真实相关输入确认或在集成分配窗口复验。原设计与r17 canonical说明同包，原native/物理IME/VoiceOver/性能/完整Gold7边界保留。

metadata从实际r17加Settings候选生成；保留历史audit基线和runtime observation/practical-intake输入，actual源码hash/锚点按此候选绑定。Collections49路径、Media五条可选导出及AX205生产不进入此批，不读取当前MAIN的Table TEMP来生成发布指纹。若最终source/feature/config/依赖改变，先重生成actual metadata并复核受影响锚点；提交号本身不改签未变的原运行证据。


## PR214–217 父票覆盖接续（2026-11-02，仅接纳既有公开回执）

本节修正上文2026-10-30时点的Media“无入口”与未收录范围，不改变原8项AC、23项Gold7要求、T/P实用Theme闭合结论或组件成熟度。公开基线为PR217 normal merge `a96602d00db783b894fa34469c182125aecab8de` / tree `0b1962fc51866db73378e93ab680d535f22bcc1b`；当前446公共类型、45组件/5patterns/2examples保持。PR212完整exact13双owner测试、53Python及原API顺序/8vs13失败均保留，不因后续发布重跑或改签。

### 已交付差量与有限范围

下列Main回执均为SSD根 `/Volumes/HP P900/DEV/TekesUI/artifacts/` 下原文件；本次只读接收，不重验raw、旧programme或归档。各PR已normal merge、原required CI全22步成功并有annotated checkpoint；不是新的clean-public native执行。

| 公开批次 / Main原回执 | 可接纳范围 | 必须保留的界限 |
|---|---|---|
| [PR214](https://github.com/TekesApps/TekesUI/pull/214) → `b71a369099fddb3dadf716acd950755c808ef7f7`；CI41286318209；checkpoint-20261101-05；`issue-48/field-core-runtime-docs-publication-20261101-r1/PUBLIC-HANDOFF.json` | Field实际input viewport完整ring/backing-pixel bounds；CPU087有效RED、CPU088 Panel26＋strict；兼容Process设计/六AC文档回填 | 原native fixture的2x fullring/caret/三barriers是限定cohort，整M6当时仍因Viewer positive失败；不冒称clean-public native或物理IME |
| [PR215](https://github.com/TekesApps/TekesUI/pull/215) → `f41476fb31506f7b24d6ee675772a7def7c45c45`；CI41294204933；checkpoint-20261101-06；`issue-203/viewer-pointer-publication-20261101-r1/PUBLIC-HANDOFF.json` | Viewer prepared-pointer原owner路径；CPU09215P/5GPU ignored＋strict、GPU093同case oldfocus RED/new1PASS；Native095三组composing barriers和原正向链有限PASS | 095为程序化AppKit输入，不是物理中日IME/候选窗；格式qualified公开native不等于merge新programme；整M6/M8/视觉/性能不升级 |
| [PR216](https://github.com/TekesApps/TekesUI/pull/216) → `a77f93cebecc7b7cfd4fb90109bc6376e891e746`；CI41297606279；checkpoint-20261101-07；`issue-204/disabled-actual-copy-publication-20261101-r1/PUBLIC-HANDOFF.json` | CPU097/Native098有限disabled Actual原因说明：两行可读文字＋对应AX help，普通窗口正常关闭 | longer copy使viewport/Fit正常居中变化，严格whole-observation transform-unchanged为 **NOT_MET**；无稳定pre-read baseline。不是无条件whole-case/M8 PASS；q>8上限分支仍未验；095PASS/096FAIL各保身份 |
| [PR217](https://github.com/TekesApps/TekesUI/pull/217) → `a96602d00db783b894fa34469c182125aecab8de`；CI41304630699；checkpoint-20261101-08；`issue-204/retained-ax-publication-20261101-r1/PUBLIC-HANDOFF.json` | 原native retained reference经dismiss后的native BOOL拒绝、whole7 witness不变及实际cleanup，Main接受有限post-dismiss业务证据；独立pure605在原sealed101上29断言通过 | 原Native101 controller仍FAIL/exit1/empty-stderr NOT_MET/null及下游NOT_REACHED；605接受精确Escape trace不是重跑或倒签101 PASS。**remount仍NOT_RUN**，whole205/wholeM6/其它M8不跟关 |

Native095已通过的正向链仅沿原programme：完整焦点环/caret、三组composing Tab/Right/Escape与七计数不逃逸、Viewer Down/Up取焦capture、普通Right Down −32pt/Up无增量、Escape恢复20461/ExpectNoText、exit0/实际释放。`issue-203/viewer-prepared-pointer-tested-handoff-20261101-r1/REMAINING-AC.json` 是较早原owner依赖表；其中“post-dismiss witness尚待准备”“disabled说明未验”已由PR216/217限定范围取代，其他D/R/A/S及物理层条件继续有效，不能照抄旧状态。

### 剩余AC → 原owner → 具体依赖

本表只组织现有票和入口，不分配资源、不新增验收门槛或第二套动态台账。不等外接屏来完成这次文档同步；硬件条件仅约束其对应执行行。

| 剩余AC / 当前状态 | 唯一接收owner（协作边界） | 具体依赖 / 原入口 |
|---|---|---|
| M6 remount旧对象拒绝、新对象正向：**NOT_RUN**；post-dismiss已有限接受，不重复095或101 | #204 consumer/operator；#205只协作原Session/observer，Main授parent/真实child窗口 | 现有`issue-204/remount-complete-preparation-20261101-r3/HANDOFF.json`已offline READY。CPU103/签名归原身份；Native104因fresh HID5.268699s<60s在准入拒绝，native/observer/Session和业务均NOT_REACHED，不是remount行为失败。需fresh HID合格、真实parent/205 child lease及same-source/bundle绑定；按原fresh-positive→old-ref Press次序和cleanup执行，不能用post-dismiss或fresh AX整数代替 |
| M8 disabled说明有限交付之外的strict transform条件：**NOT_MET**；另q>8原因分支未验 | #204原说明/consumer验收；#203仅接实际组件缺陷 | PR216说明不是transform不变PASS。若继续原条件，须先有稳定pre-read baseline并明确原case下的真实viewport/transform对照；不能覆盖旧NOT_MET或把正常Fit重排偷换成“无变化”。q>8分支需真实适用状态，未授权不强造 |
| M8-A1 enabled Actual/anchor：未验 | #204；#201/#202提供既有geometry oracle | 实测viewport与200×100逻辑图使f=min(Vw/200,Vh/100)∈[1/8,1]且Actual实际enabled；当前disabled解释/无操作不代正向通过。原R1场景和Main新具名窗口，不新增geometry实现 |
| M8-D1/R1真实OS pan/界外release/后续move、border resize：未验 | #204原输入验收；Main协调工具合同/资源 | 原M8卡：MacVec2原点/单位/global-vs-app映射和安全ownwindow端点必须明确；R1另需真实resize border。atomic drag不是残留capture或held组合证明，programmatic MouseMove/NSWindow resize不冒充OS gesture |
| M8-D2 held drag中Escape/失焦/resize/source切换：未验 | #204；Main协调受支持输入合同 | 当前只有atomic drag；需受支持且授权的分开Down/Move/Up/interleave合同。不得自造CGEvent/AppleScript/private driver或用CPU/Host结果代签 |
| M8-S1不同backing scale：未验 | Main环境协调；#204原operator | 真正允许的不同倍率外屏与正常移动路径；当前同倍率/不支持的scale setter不代签，不改系统设置。只阻此行，不阻文档、普通源码或其它独立AC |
| 物理中日IME/真实候选窗：未验/原物理层边界 | #204负责Media；#12负责原页面物理输入；Main协调原窗口 | 实际输入法/候选窗/操作来源和原oracle。095的程序化NSTextInputClient、UTF16/caret/barrier证据只能沿其原范围；不因此新开输入实现或重跑095 |
| VoiceOver、性能与pinch原延期/范围外 | 原#204适用范围；Main恢复范围/性能owner | VO需真实导航/播报环境；性能需预定义workload/cadence/threshold/测量cohort；pinch需原协调明确恢复范围和支持工具。不从AX snapshot、测试耗时或Zoom按钮推导通过，不恢复成普通批新前置 |
| 父AC4/5/7整页/Gold7与其余23项体验 | #12原整页视觉/真实交互owner；图标#57独立 | 上表原Settings/五patterns/普通与IC焦点、错误、窄大字等具体场景保持，按相关输入补受影响项。PR214–217提供其有限局部证据，不是全页族、全品牌、全硬件/120Hz或全产品Gold7 |

本次仅更新父票覆盖与剩余AC映射；Main的公开receipt、原owner有效范围和所有旧红/原source/program身份保留。#1/#55保持OPEN，#203/#204/#205的有效AC和关闭由原owner/主协调接收。未改registry/capabilities/production、未运行构建/CI/GPU/native/UI或新checker，无Release/部署。


## 2026-11-05 恢复批次：原验收范围与缺项更新

本节覆盖前文较早快照中的当前状态，不改原八条 AC、23 个 Gold7 ID、
原失败或具体 source/program 身份。已完成且相关输入未变的检查继续复用；
模块基础供应完成不等于关联父票全部完成。VoiceOver 按用户要求后置；
原性能/120Hz、AccessKit、#131–#135 与跨仓迁移边界不自动恢复。

### 八条 AC 的现有证据与下一交付

| 原 AC | 本轮接收与范围 | 仍需完成 |
|---|---|---|
| AC1/AC2/AC3/AC6 | 原52根实用 Theme、语义/状态映射及45组件＋5 patterns＋2 examples准确投影复用；#206共享可读性已闭合 | 仅对真实新增/变更路径补相应检查，不重做52根迁移 |
| AC4 | 原 Quiet/Search/工具动作合同，以及 Settings A、Master B、PR218 Filter/Async/Danger有限阅读图复用 | 原代表非默认品牌与页面强调对照；真实操作中主次层级的未覆盖部分 |
| AC5 | 必要对比/状态、#206、Settings #207和#3原焦点/TextCommand接收保持各自范围 | 原表单/菜单/选择/浮层适用真实输入和物理阅读缺项，不把图片当操作证明 |
| AC7 | A48、B24、C64、Progress既有1x/2x及PR218的192图＋4回归分别按原身份接收 | 原非默认品牌、窄常规/宽大字等代表差量，以及实际倍率/物理显示；不做品牌×尺寸笛卡尔全排列 |
| AC8 | 原原件、red/修复/通过记录与公开批次保留；以下逐ID对应更新 | 后续真实结果持续回填；未完成父票不关闭，registry不由文档更新自动晋级 |

### 原23个 Gold7 ID 的证据去重与缺项

| 原要求ID | 本轮处理 |
|---|---|
| G1/warm-neutral-reading-foundation | 原主题与有限页面图复用；持续物理阅读/主次观察仍单列未验 |
| G1/consumer-brand-derivation | 原品牌生成/隔离证据复用；代表非默认品牌的实际 Menu/Dialog 叠层与页面对照继续补缺 |
| G1/whole-page-reading-composition | PR218 Filter48/Async64/Danger80以及A/B/C/Progress已有阅读批次复用；只补原未覆盖代表组合 |
| G2/four-semantic-surface-roles | 原Surface消费合同复用；具体canvas/region/panel/overlay代表页面去重后补图审 |
| G2/same-color-geometric-distinction | 原1x/2x浮层GPU保留；物理屏幕边缘观察未由GPU代签 |
| G3/hairline-and-required-boundary-roles | 原必要边界、错误/focus和#206阈值复用；未覆盖同屏/物理阅读继续单列 |
| G3/component-role-radius-map | 原角色映射/合理例外复用；仅补适用实例的实际阅读 |
| G3/nested-radius-geometry | 只验已经声明同心的原实例outer/inset/inner与像素，不给普通相邻控件新增同心要求 |
| G3/real-scale-rendering | 原离屏双倍率不冒实际双倍率；真实显示与适用reveal仍按原来源要求 |
| G4/named-press-expansion-snap-presets | 原#9/#38有限合同及已接受结果复用；未覆盖真实按压/反向时序仍保留 |
| G4/real-component-motion-integration | 原生产接线/已有浮层帧复用；原适用进入/退出/中间帧/卸载缺项继续 |
| G4/direct-manipulation-and-interruption | 原Split软件/GPU接收不代签真实中断；Media有限拖动见下方，held取消尚未完成 |
| G4/reduced-motion-path | 原降级接线复用；系统设置下适用控件/浮层动作及取消仍需实际证据 |
| G5/ime-and-suggestion-priority | #3原真人有限接收、真拼音一次你好和M7/M8有限组字链分源复用；候选跟随及其余适用中日IME缺项保留 |
| G5/durable-draft-recovery | 原journal/outbox/受控草稿证据复用；原本地fixture实际编辑/中断/重启/旧ack路径只补未覆盖部分，不冒真实断电 |
| G5/manual-scroll-reading-anchor | M3 #22/#53原有限完成复用；本地阅读与Process #48真实交互分开，不重复M3，不代签跨进程 |
| G5/explicit-return-to-latest | 原M3实际latest/嵌套阅读按原身份复用；其他原页面缺项逐入口处理 |
| G6/zoom-preserves-reading-anchor | #201/#202设计/几何已关闭，后续#204的Actual/resize/最大说明按其原有限收据复用；不再仅凭本表旧“未验”重跑；未闭合严格条件保留 |
| G6/bounded-split-and-pan-geometry | 已有普通图片pan；本轮新程序实际+260pt并在完整窗口外20pt松手，q/Y不变、Up无重复变换。未验的是释放后unheld移动；Split原真实项独立 |
| G6/cancel-and-non-gesture-alternatives | 原按钮/键盘/生命周期证据分源复用；原子drag后再Esc不证明held取消。实际按住期间Esc/失焦等组合仍未验 |
| G7/reusable-real-component-contract | M1/M3/M4/M6已有SDK有限供应及M2原AC复用；完整#5语义控件族仍逐项对账，不能凭modal少数case代签 |
| G7/component-documentation-and-runnable-preview | 既有公共API/路由/预览复用；只有变化或拟晋级入口补适用检查，无成熟度自动晋级 |
| G7/business-combinations-are-real | PR218完整有限192图与4回归已公开；form/filter/master/async/danger及Settings的真实操作与图审分列，Danger仍只模拟失败 |

### 原范围内继续推进的明确任务

- #48 CI197 的 geometry-pending native Tab continuation 和 owned Reader crash status
  两个原失败未因随后通过而闭合；当前按精确失败日志定位，不循环跑通过掩盖。
- #48 三子容器真实拼音 auxiliary/reveal/append 与候选移动缩放仍是独立 AC；
  local Panel 系统菜单跨屏成功不能替代实际 Process parent/child/session 交互。
- #25 真实组字期间 Append/Toggle 尚未完成。用户接受的仅是打开原生菜单后的
  候选消失/拉丁拼音观察，不能扩大为后台更新通过。无既有不扰组字入口时保留未验。
- #5/#205 原 AXPress 返回错误及外部通知/撤权未完成；内容点击和截图成功不覆盖它。
- CU direct typeText 在系统标准文本框和SDK均丢CJK，#41保留工具边界；
  真拼音输入成功不改签该工具路径，也不据该工具失败改写SDK正常输入。
- PR225 的原代码批次 `55b574fa4e9a952589edbb1f28065168d6628b18` 包含有限D1、
  windowDidMove坐标失效通知和consumer状态布局修复。本地受影响build/strict、
  窗口外pan及完整fixture真实Renderer回归通过；native仍绑定MAIN overlay，
  不称clean-public-head GUI通过。所需CI、合并及checkpoint单独记录，不提前称发布。

Refs #1 #5 #12 #25 #41 #44 #46 #48 #55 #56 #203 #204 #205。


### 2026-11-05 bounded visual and semantic evidence intake

T32 supplements existing A48/B24/C64/PR218 acceptance with Settings at
720x512/100% and 1440x960/150%, four themes and top/bottom positions (16 files),
plus original Menu/Dialog overlays with existing nondefault brand 1 (8 files).
The two explicitly allocated GPU capture tests, Gallery test-target build and
strict clippy passed. All 24 original PNGs were opened individually and passed
bounded visual review. They contain 20 unique pixel outputs: wide top/bottom
pairs have zero scroll range and are not independent scrolling evidence.
The large-text Save settings label wraps into two complete readable lines.

The original E0425 failure is preserved. Its minimal test-only repair uses the
existing tekes_ui_tokens::palette::gallery_brand_preset path; no new brand or
production behavior was added. The declared concentric-geometry pixel contract
reuses the unchanged PR217 passing source. Native interaction, physical display
and scale, candidate-window following, and deferred VoiceOver remain separate.
This batch does not close #1/#12/#55/#56.

Evidence: artifacts/issue-12/t32-visual-remainder-20261105-r1/captures-002/.
Per-image review SHA256:
3209d6291b6b6d7627fb7ac3f1df6b9387e3a548b85054dd42adb1a18a0da9c4.
Actual rendering binds the MAIN overlay source-freeze SHA256:
a2546ea690c615718429a284cb276263721d29f5221c0f5f176dfa725ad62ee3.
It is not clean-public-head native acceptance. The original 13 WIP files are
preserved separately and are not imported by this batch.

T31 maps Button/Icon, selection, text input, collections, Menu/Select and Dialog
to current source, named tests and original native receipts; nine source hashes
and nine primary receipt hashes were checked. The accepted Gallery native-tree.json
export and actual readback satisfy the bounded original exported-tree inspection
criterion. A historical future interactive Inspector note does not add a new
panel to this batch. External AX actions/notifications in #205 remain unresolved,
VoiceOver remains deferred, and #5 remains open.

PR225 CI exposed stale composition-audit and then capability-input source hashes;
both failures are preserved. Regeneration keeps 45 components, five patterns, two
examples and capability declarations unchanged. Publication still requires actual
required CI success on the final combined head, a normal merge and an annotated
checkpoint. CI runner-capacity retry limits have been withdrawn by the user;
real test failures require diagnosis rather than unchanged blind retries.


### 2026-11-05 actual Gallery interaction defects and targeted repair

The original normal Gallery batch exercised Filter, Async, Danger, Select,
ComboBox and local Form interruption/restart through the single official CU
operator. Accepted sequences retain their original signed-program identity
44849438f6c9b99eb825dcae6cbcfb9307131e9c686839c756a8659c0a8d9dc2.
These are software-driven native interactions, not physical-input, VoiceOver,
network-service or power-loss acceptance. Original screenshots/AX observations
remain in the named CU tool calls; no exported PNG files are claimed.

Two concrete failures were repaired within existing #5/#12/#56 contracts:

- Filter's actual AX input name was Project name because its renderer fell into
  the generic name branch. Name only that original input Filter files. The
  existing narrow/large-text regression now checks its mounted node name/value.
- After a grounded coordinate click established Document 01 focus, Down selected
  Document 02 but left Document 1 detail. Reuse the existing activation path when
  the keyboard target changes, keeping focus/reveal. Home/End now implement the
  already-advertised endpoints; repeated boundary navigation preserves detail
  selection. Extend the existing Master reveal regression to cover this path.

The exact tests are in gallery::workbench::interaction_tests:
search_filter_row_wraps_without_overlapping_hits_or_results and
master_detail_keyboard_focus_reveals_scrolled_rows_with_focus_bounds.
Each ran once and passed; Gallery strict and normal build passed. The first
qualified-name invocation selected zero tests and was correctly rejected; its
failure is retained and the already-compiled binary was reused for the corrected
names. No zero-test result is counted as a pass.

Fresh signed programme
7980760588ffa1b9957cd76fc3f73dd0693b6f685cfd9e067526a8f023a18fb5
actually exposes Filter files, and Master Down/Up/Home/End selected rows match
Document 2/13/1/80 details; scrolling list and detail remains separate. Original
indexed-click focus uncertainty and a CU half-scale coordinate error remain
separate from the confirmed product mismatch. Read-only detail drag selection
and Copy were not established by that run and remain unverified.

Form's first normal exit persisted draft B at revision28 while acknowledging
revision15; a same-file restart visibly restored B and notifications, without
reset/edit/submit. This is the explicit local draft fixture only. Original
successful Filter/Async/Danger/Select/Combo/Form sequences were not repeated for
the two repairs. System Reduced Motion propagation and other original motion,
physical-input/candidate and external-AX criteria remain separately outstanding.

Evidence: artifacts/issue-12/t33-page-native-20261105-r1/ and -r2/;
source-freeze SHA256
613e0565b8b11dd45b1e5f7964c13e075d89a8919a06dbe0dda86192d1412ea8.
Actual native evidence remains MAIN-overlay evidence, not a clean-public-head
claim. The original13 WIP files remain byte-identical and excluded from delivery.
