> 当前本地 main 接收（2026-10-27）：以下 2026-10-24 设计及阶段记录保留历史身份。基线 `437b5b6` 加本次精确差量已完成公共 API、6 项混合 Host/状态回归、Gallery 回归、公共类型绑定和严格检查；默认/外部 Theme 实际 GPU、正常标签可读性与真实 Gallery 页/variants 通过。详情与失败见 SSD `artifacts/issue-136/status76-main437b5b6-intake/root-review.json`。不宣称原生窗口、VoiceOver、物理 IME、性能或发布通过；当前 ContentTheme 无历史 icon_foreground 输入，其独立覆盖不转签。本次验证保留 foreground override 断言，徽标本身不绘制图标。

> 2026-10-24 主协调接收共享 Theme 阶段 #83：rules 1.8.0 / schema 1.3.0 已串行集成；原32用途的21168状态包逐字保持，新增四Badge及Skeleton共5用途。正式外部artifact与独立锁、正常Rust消费280包/Python0已核验；详见 design/reviews/passive-status-theme-implementation-20261024/root-intake.json。本文的组件实施/GPU验收继续由 #76/#77 独立交付，不能以生成校验代替。

# StatusBadge 实心共享语义与被动 Content：有限设计

2026-10-24；关联 [#76](https://github.com/TekesApps/TekesUI/issues/76)，父 #55/#56。状态：**主协调已接收本有限设计；未实施、未编译、未 GPU 验证，Issue 保持 OPEN**。仅完成设计交接，生产代码另行实施。唯一 active 源为 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，观察 HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290` 加 WIP；不是公开 main。精确输入见 `design/reviews/passive-status-content-design-20261024/source-manifest.json`，完整输入副本和两票原文在同目录。

## 原实现、适用状态与缺口

`extended.rs::StatusBadge` 只有 Info/Running/Succeeded/Failed；原 `status_badge` painter 选择 status-info/status-running/status-succeeded/status-failed，画填满父 frame 的实心 pill，圆角 `max(height,0)/2`。`feedback_view.rs::StatusBadgeView` 保留 frame/content/visible，按 ancestor clip 画原 painter；`label_color` 单独读取 c-status-badge-foreground。原 View 没有 label 参数或文字量测。原 Gallery `feedback_adapter.rs` 的 status-badge 页面/矩阵用 UiCaption、`frame.inset(5)` 画文字；grid 以原文字量测加10及 fixture 下限28排布。5/10/28 是现有 Gallery fixture 的事实，**不是新增公共 Content 默认 padding/高度规范**。不将 radius-badge=4 强加到原 pill；原 painter 的圆角才是本阶段参照。

`content-status-*` 是父 Surface 上的被动状态前景包：background=$surface，rest 的 text/icon=status-*，disabled 的 text/icon=disabled-text，boundary/focus/marker/helper=null。它不能拿来画实心 badge，也不能覆盖 badge 内的文字。现有 Python `verify_solid_status_foregrounds` 与 Rust 同名检查独立保证 c-status-badge-foreground 对四种实心底的普通4.5、增强7.0对比；必须原样保留。不能把 glyph #47 当本票交付。

原登记 `rest/loading/error/success/unverified/cancelled` 保留，四 tone 不扩成业务 reducer。调用者提供 label/description 表达未验证、已取消、错误与成功；未验证不映射成功，Running 不自动计时或旋转。主协调已接收沿 #72 **rest-only 视觉用途**：badge 是状态读数，disabled/hover/pressed/selected/focus 不适用；不新增 set_disabled，也不把宿主不可操作误解为成功/失败被灰掉。父容器 inactive/hidden 仍由原 Host 负责。上游 status-badge 文档原本列 disabled 不适用；本票“disabled 可用性”由显式拒绝与被动不获焦证明，而不是伪造 disabled 色表。单独 availability 扩展未采用；后续若改变需另审，不能默默加状态。

## 已接收的完整共享 packet

新增四个用途 `status-badge-info/running/succeeded/failed`，各仅 `rest`。沿现有 Palette → resolve_surface → SurfaceColors，保持七通道完整，不新增色 token、RGB 参数或第二个 Theme 层。

| 通道 | 每个 tone 的共享角色/政策 |
| --- | --- |
| background | 对应 `status-info/running/succeeded/failed`；不是 `$surface` |
| text | `c-status-badge-foreground`，必须存在 |
| icon | `c-status-badge-foreground`，完整包保留；本阶段不添加 icon anatomy |
| boundary | `null`，已接收政策要求实心底自身对父 Surface ≥3；无需额外轮廓 |
| focus | `null`，非交互 |
| marker | `null`，没有额外 marker anatomy |
| helper | `null`，description 走语义，非独立 helper ink |

七 Surface 为 canvas/code/hint/panel/popover/sidebar/toolbar；四模式 light/dark/light-ic/dark-ic。默认/default、blue、clay、blue-inputs 当前角色的448组静态核算中，文字与父底守卫均无失败，见 contrast-audit.json；这不是新 packet 已通过或 GPU 可见的证据。必要边界政策是显式的：本有限方案无 boundary，**任何外部 Theme 若 fill/parent <3 即拒绝整个 Theme**，即使 label/fill 仍达标；不在 consumer 猜测补边。若未来必须容纳这样的 Theme，可另审共享必要 boundary（至少对 parent 与 fill 都≥3），不能把 `border` 无条件填入：它未必与每种 solid fill 对比充足。本阶段不做运行时颜色推断或条件派色。

生成器与纯 Rust 正常校验必须共同增加用途/精确状态白名单、精确通道绑定与上述父底对比守卫。仅颜色角色已知/七通道齐全不够；badge text/icon 必须来自专用前景，不能绑定 status-* 或 primary-text 再仅靠数值偶然通过。固化四 tone→角色对应，不允许 succeeded 被换成 failed 底。保留全部现有32用途的状态与逐字段 packet（包括 progress-track/rest）；新增4用途后应为36用途。不得把旧用途放宽为 rest-only。没有新状态枚举或正常构建 Python 调用。

拒绝集：未知/缺失 Surface，用途/tone/state 未知；缺 background/text/icon 或任一通道字段；错误 tone 底/专用前景；非 null boundary/focus/marker/helper；hover/pressed/selected/disabled/focus 等伪状态；低 label/fill（普通4.5、增强7）或 fill/parent<3；别名循环/缺角色/不完整四模式、输入版本/源锁/生成输出不匹配。Python 离线重算与 Rust 正常校验分别有必要负例；自洽重生成全哈希仍须语义拒绝，不能只靠 stale hash 拦截。合法→语义负例→合法恢复绑定同一源、独立外部 source lock。

## 原 View 与公共 adapter 合同

建议新增 fallible `extended::status_badge_on_surface`，原 `status_badge` 签名委托 canvas；resolve 完整包后才写 Scene，任何错误不留部分绘制。StatusBadgeView 增 `set_surface/surface`（非法名在替换 retained context 前拒绝），旧 render/label_color 保留兼容。新增原 View 的 labelled render/measure 入口：同一 ButtonTextRenderer、UiCaption 和同一文字 layout 供测量与绘制，adapter 不复制文本排版。原未标注 View 用途仍可独立画底。

公共 `composition::StatusBadgeContent` 只持有原 StatusBadge、受控 label/name/description 与一个原 View；构造/变更先验证再提交。默认非装饰，name 默认来自非空 label，description 可选；空/全空白 label/name 原子拒绝，原 mount/model/布局不变。若明确 decorative，则 semantic_hidden=true，不产生重复可访问节点；可无语义名字，但有可见文字时仍用同一原文字服务。非装饰使用原 ContentRole::Text + name/description（不添加 wire enum、value_range、native role 或 live announcement）。视觉 label 与可访问 name 可显式不同；不能用色相作为唯一状态信息。

父布局提供有限 badge frame；本阶段无猜测 intrinsic/default 高度或公共 padding。若需要无显式高度的 intrinsic badge，先接收原规范/现有 geometry 的明确归属，再在原 View 内统一量测。最小方案把文字限制在父 frame 内、按原 TextRenderer 的测量/绘制契约，不单独定义省略、wrap、裁字规则，更不能靠字符串截断替代原文字量度/绘制。Gallery 原 inset=5 可留在 Gallery 参数/原布局，不提升为 SDK 魔数。实现若无法在无新 padding 参数下保持可读 label，应把此问题回到接收裁决，不能自行扩大配置表。

ContentTheme 的 palette（含 brand/external Theme）与 surface() 透传原 View。原 Host 负责默认 canvas、host inherited、祖先继承、显式覆盖、clear 恢复；adapter 不保存另一份继承状态。solid 的 text/icon 来自自己的 packet，不接受祖先 decorative `foreground/icon_foreground` 把专用 ink 覆盖掉。badge 也不向兄弟/子树发布自己的 solid ground：它是叶子。若未来要承载任意孩子，需要另审独立 Surface 边界，不得把背景 RGB 反推 Surface。

measure/render/layout 使用同一有限父分配。原 Host 的 clean_size 已会清洗非有限/负尺寸；这不等于 fallible 原子拒绝。本票不为此扩改全局 Host：父 finite 非负尺寸是挂载前置条件，独立 consumer 必须先校验自有分配；若 adapter 新增显式尺寸配置，则其 setter 必须 reject-before-commit，不能只在 paint 清洗后声称拒绝。零分配/完全 clip 合法但预期无 ink，正常分配必须 positive ink。保留原 View 的 clip，不额外加第二层叶子 scissor；部分 clip、非整数边缘是否吞字需 GPU 实测。

默认 Component 被动事件/非 focusable/无 TextSurface/无 value_range；pointer/wheel/key/AX Press/SetValue/增减无结果。模型/label/Theme 更新复用 mount，不调用原 reading navigation/reveal/anchor API。原 Composer 的焦点、binding、selection、marked、caret、发送门禁由原 Host 保持；不新建 controller/reading owner/editor。

## 最小生产文件与单 writer 接入

以下是设计接收后允许形成候选的精确集合，不是本阶段已改文件：

| owner | 文件/最小变化 |
| --- | --- |
| 共享 Theme 唯一 writer（两票合并一个窗口） | `design/theme/rules.json`；`scripts/generate-theme-system.py`；`scripts/test-theme-system.py`；`crates/tekes-ui-design-export/src/theme_system.rs`；`crates/tekes-ui-tokens/src/surface_palette.rs` 的对应 tests；`design/system.json` 契约/输入 pins；`design/system.schema.json` rules_version const版本pin；`design/theme/generated/instances.json`、`design/theme/generated/surface_rules.rs`、`design/theme/artifact.lock.json` 原生成输出 |
| badge 组件候选 | `crates/tekes-ui-controls/src/extended.rs` 原 painter；`crates/tekes-ui-controls/src/feedback_view.rs` 原 View/文字 measure；新 `crates/tekes-ui-controls/src/composition/status_badge.rs`；`crates/tekes-ui-controls/src/composition.rs` 导出 |
| 原 Gallery/独立 consumer | `crates/tekes-ui-gallery/src/gallery/workbench/extended/feedback_adapter.rs`；新 `examples/composition-consumer/src/status_badge_content.rs`、`examples/composition-consumer/tests/status_badge_content.rs`；`examples/composition-consumer/src/lib.rs` 模块导出；必要 GPU harness 仅复用现有路径/依赖 |
| 主协调 metadata 唯一 writer | `design/gallery/public-api-surface.json`、`design/gallery/workbench.json`、原生成输出 `crates/tekes-ui-controls/tests/component_registry_api.rs`；`scripts/generate-component-capabilities.py` 同时输出 `design/gallery/capabilities.json`，仅主协调在稳定批次接收其全局增量，不手写 hash 或修改旧 evidence |

现有 schema **结构**已容纳七通道/rest-only，无需扩展 schema 形状。版本pin是另一回事：live `design/system.schema.json` 第411–412行将rules_version固定为1.7.0；共享规则升级必须同步该const及 `design/system.json` 契约/pins，连同相关生成输出/锁由唯一writer接收，否则不一致。`design/theme/rules.schema.json`、`design/theme/theme.schema.json`、`design/theme/artifact-lock.schema.json` 目前无结构扩展需求，base.tokens/token-metadata不新增颜色。具体新规则版本由共享writer统一确定；这里不抢占版本号。无需 protocol/runtime/platform/AppHost/Kernel。原 `StatusBadgeView::label_color` 在 Banner/Toast/Gallery 的现有使用必须不回归；不顺带迁移 Banner/Toast。

badge/skeleton adapter 的新模块可以隔离候选并行；extended/paint/feedback_view/composition/Theme/全局 metadata 均交 candidate+hunk，由主协调分配唯一 writer 串行接入，与 Progress/Tree/Panel 错开。先 Theme 稳定 source/hash/packet handoff，再绑定组件编译/实际原页与 GPU；不直接编辑 shared live 文件。

## 验证计划与精确缺口

1. 共享层：默认品牌/现有全部 Theme+外部独立 Theme，7 Surface×4 mode×4 tone；比较旧32用途全部 packet/state/compiled role map，无颜色基线变化；原 solid/link/range/progress/输入/selection守卫继续通过。必要反例见拒绝集；Rust 普通 workspace 外合法 warm→语义负例→合法恢复，正常构建不跑 Python。只运行原 lock/environment、+1.95.0 --locked --offline 和原 target/review-cache。
2. API/原 View：真实公共类型缺失编译红→同源绿；四 tone、long/CJK/graphemes/空名拒绝、父 finite/零/窄/非整数、默认/继承/explicit/clear/未知拒绝。原裸 painter、labelled View、Content 必须同 palette/packet/geometry。保留旧 Gallery 四态/标签和实际品牌。
3. 原 ContentHost 混合树：Badge+普通 Text+独立操作+原 Composer，dynamic tone/label/description/Theme/Surface 更新、reading transaction 内更新，断言 mount/原 reading 锚点、focus/binding/selection/marked/caret/无 Submit；被动事件无 action/focus/editor。程序化 Preedit 只证明程序化路径。
4. 必要实际 GPU：先代表 Theme 校准，最终同源四 mode、默认实际 Gallery 品牌及外部 Theme分开，七 Surface必要对照、四 tone、1x/2x、非整数原点、正常/窄/CJK大字/部分clip。原 bare View 为几何参照，labelled View 与 Content 同图读回；文字、solid、父底及边界政策实际读回，正常帧必须 positive ink，clip外无 ink。保留真实红例；不能用全空图/Scene断言/独立微宿主冒充实际 Gallery 页。
5. 本设计仅有源审计和静态角色对比；build/test/协议/GPU/原生窗口/物理IME/VO/性能/发布均未验证。沿原有限验收结项，不追加全DS、专用AX、全部native或物理IME/VO关闭前提；分层据实回填 #76/#55/#56，PR 当前无。

## 主协调接收结论与后续责任

主协调2026-10-24明确接收：rest-only/no disabled视觉；原四tone solid+专用foreground；fill/parent≥3、boundary=null，不合格外部Theme在编译期拒绝；父finite分配、无猜测intrinsic高度/padding；UiCaption沿原文字服务量度/绘制，不能字符串截断。上述替代选项不进入本阶段。

共享新policy代码由主协调后续拆出的**独立共享Theme有限依赖票**负责，adapter仍属#76/#77；本设计会话不建票、不实施、不改生产schema。共享writer先交精确source/hash/packet handoff，再接组件候选，保留#71进度包、旧32用途/全部通道守卫。版本const同步属于必要共享文件，不是schema结构扩展。

早期设计/旧hash保留于证据 `pre-acceptance-design/` 与 `handoff-v1.json`；源审计/contrast/hash不改签。设计已接收不代表实施/运行时验收完成，无需新增全DS/native/物理IME/VO门槛。本会话不再委派、不联系旧保存会话。


## 2026-10-24 labelled 默认对齐接收修订

主协调查看 #76 候选真实 GPU 图，确认 Leading + 整个父 frame 会令正常 Info 标签贴近 pill 左圆角、部分文字落在父底，原 positive ink 不能证明可读性。接收有限修正：原 StatusBadgeView 新 labelled 默认入口使用现有 ButtonTextAlign::Center，Content 直接复用；Gallery 显式保留既有 Leading 与其 inset5 fixture 布局。不新增公共 padding/猜测高度/新文字布局引擎。原 View 量度与绘制仍共用原文字权威；正常 frame 的文字可读与极窄合法裁剪分开验证，不能宣称所有文本在任意窄宽都可读。旧失败图/程序/source 保留，组件实现和影响范围验证仍由 #76 交付。

## 2026-10-24 实施期间接收的文字对齐修订（主协调接收）

主协调检查真实 GPU 图后接收：Content 将 label Leading 放在完整无 padding 的 pill frame 时，正常 Info 也贴近圆角，positive ink 不足以证明可读。最初关于部分 ink 落在父底的推测未获后续实际字形 mask 验证（旧 Leading 的 224 个 opaque-solid 案例通过），不能声称已证实对比失效；原文字服务 Center 对照在旧程序首个 Info@2x 案例真实失败，修订后默认/外部各 224 案例通过。原反例、旧源/程序/depinfo/hash 留在 `design/reviews/status-badge-content-implementation-20261024/`，不改签原通过记录。

原 StatusBadgeView 的 `render_labelled` 默认沿既有 `ButtonTextAlign::Center`；Content 直接复用此入口。原 Gallery 显式调用同一 View 的 `render_labelled_aligned`，保留其既有 Leading 和 inset=5。两入口共用一个量测/绘制实现与 UiCaption/原文字服务，不引入新配置表/颜色/布局引擎或公共 padding/高度。Gallery 的 5/10/28 仍为宿主 fixture 分配。

正常父 frame 的四 tone 需验证文字 ink 位于有效实心区域，labelled View 与 mounted Content 实际 GPU 读回相等。极窄/CJK 和部分 clip 沿原文字服务合法有限呈现，可能截去部分可见文字；只报告实际 ink/clip，不把它宣称为完整文本可读。语义仍保留完整 name/description。只重验该对齐变化影响的必要案例，原 Gallery 同 Leading/inset 参数的结果可按精确源桥接。

后补证据更正：上一节“部分文字落在父底”为当时视觉推测；后续旧 Leading 的224项实际 opaque-solid glyph-mask 检查通过，该推测未获证实，以本节更正为准。旧记录保留。
