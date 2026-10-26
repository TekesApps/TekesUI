> 当前本地 main 接收（2026-10-27）：基线 `23447d2` 加精确 Skeleton 差量已通过公共 API、默认/外部混合 Host、共享包消费、类型绑定、生产与专用测试 strict；实际默认/外部 GPU 及原 Gallery route 验收通过。生产源码在所有运行间未变；旧 Gallery Root 记录保持原身份。证据：SSD `artifacts/issue-136/skeleton77-main23447d2-intake/root-review.json`。原生窗口、VoiceOver、物理 IME、性能和发布仍未验。以下为保留的原设计和阶段记录。

> 2026-10-24 主协调最终接收 #77：15文件已串行集成，1501候选输入/44755证据hash已核验，默认+正式外部Theme/原Gallery共10688实际GPU帧，原View与mountedContent像素一致、祖先clip外严格零。controls lib及4个新consumer定向strict通过；broader all-targets原capture_origin.rs既有type_complexity失败单独保留，未改作通过。证据 design/reviews/skeleton-content-implementation-20261024/root-intake.json。原生窗口/物理IME/VO/性能/发行未验；下方“待接收”段是保留的阶段历史。

> 2026-10-24 主协调接收共享 Theme 阶段 #83：rules 1.8.0 / schema 1.3.0 已串行集成；原32用途的21168状态包逐字保持，新增四Badge及Skeleton共5用途。正式外部artifact与独立锁、正常Rust消费280包/Python0已核验；详见 design/reviews/passive-status-theme-implementation-20261024/root-intake.json。本文的组件实施/GPU验收继续由 #76/#77 独立交付，不能以生成校验代替。

# Skeleton 静态几何、共享用途与被动 Content：有限设计

2026-10-24；关联 [#77](https://github.com/TekesApps/TekesUI/issues/77)，父 #55/#56。状态：**主协调已接收本有限设计；未实施、未编译、未GPU验证，Issue保持OPEN**。active源 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`，HEAD `1a7614a273e1a897ddc8a7ad8cb41d303a4a1290`+WIP，非公开main。完整输入/manifest/原票/静态数据见 `design/reviews/passive-status-content-design-20261024/`；共享 Theme 改动与 #76 同一个 writer 窗口。

## 原实现与可见性缺口

`paint.rs::skeleton` 画一个与父 frame 相同的 surface-2 Quad，corner_radius=4；`feedback_view.rs::SkeletonView` 保存原 ViewLayout、按 frame 与 ancestor clip 的交集绘制。没有 Skeleton 模型、loading reducer、动画、shimmer、phase/ticker。原 Gallery `workbench.rs` 的 skeleton 分支画四行，fixture高18、行距30、交错缩短80、最大宽500；这些不是 SDK 默认高度/行数/尺寸。保留原矩形/圆角几何，不把它改成文字、Progress或动画图。

静态审计默认/default、blue、clay、blue-inputs ×4模式×7Surface共112组合：**52组合原 surface-2 与父底精确同色（contrast=1）**。例：panel/code 各模式；light hint/popover；详见 contrast-audit.json，不能宣称完整GPU矩阵已失败，也不能把它们当可见占位通过。其余原fill/parent也很低。装饰不承担状态信息，因此不强制填充本身3:1，但“占位存在”仍需可见性政策。

已接收 A：原 surface-2 fill 不变，新增共享必要边界=border；边界对 fill、parent 均≥3，保留原圆角4与外部 frame。当前112组角色静态核算边界对父最小约4.357、对fill最小约4.756，具备有限候选可行性；外部Theme仍需重验。该边界是本票有意的可见性增量，不伪称原View已有，不用任意提高填充对比或增加色token。早期B选项为保留原无边界装饰及其同色限制，主协调未采用；本阶段固定A，仍不实施生产代码。

## 完整共享 packet 与边界

新增 `skeleton/rest`，rest-only，不新增 hover/pressed/focus/disabled/loading/error 等视觉状态；loading语义由宿主普通 Text/Group 表达，静态Skeleton本体没有状态机。沿现有Palette/resolve_surface/SurfaceColors。

| 通道 | 已接收A固定来源/用途 |
| --- | --- |
| background | `surface-2`，原占位fill |
| text | `primary-text`，完整包必需，但Skeleton不画文字 |
| icon | `primary-text`，完整包必需，但Skeleton不画图标 |
| boundary | `border`，必须存在并绘制，保证轮廓可见 |
| focus | `null` |
| marker | `null`，没有独立填充进度/标记 |
| helper | `null`，无helper anatomy |

必要边界的3:1对比是共享规则编译期政策，检查 parent 与 fill 两侧，不拿foreground校验替代。text/icon完整性沿现有七通道契约校验，未绘制不意味着可删除；不把它们称为本体实际文字验收。原generic foreground守卫继续适用，不放宽旧用途。boundary使用现有 `stroke-rule`，保留frame和原corner_radius=4；若使用token替代现常量，必须引用原 `p-radius-4` 数值来源或经审核的用途别名，不能因为另一组件叫radius-badge就借用其语义。边界绘制在fill之后，沿原Quad内边界/renderer契约验证，不扩张原frame，不新建geometry/layout状态。

7Surface canvas/code/hint/panel/popover/sidebar/toolbar，4mode light/dark/light-ic/dark-ic，default及原品牌、外部Theme一体生成。#71 progress-track/rest 稳定包保持 background=surface-2、text/icon=primary-text、boundary=border、marker=c-progress-active、focus/helper=null；Skeleton与它的几何/marker不同，**不能借progress-track当Skeleton packet**。单独增用途，不复制Theme引擎。badge增加4用途、Skeleton增加1用途，二者统一接收后原32变37；若分批接收，基数必须绑定当时精确manifest，不写死过期计数。

拒绝：未知/缺失Surface/use/state；缺任一通道或必要boundary；boundary=null、错误background/text/icon/boundary来源；伪hover/pressed/selected/disabled/focus；非nullfocus/marker/helper；边界与parent或fill任一<3；未知角色、别名循环、不全四模式、rules/contract/source-lock/generated不匹配。Python与纯Rust均增加本用途的精确状态/通道绑定及边界守卫；自洽重生成全hash的低对比外部Theme也须被普通Rust构建拒绝。所有旧32用途与状态、默认色角色/现有solid/link/range/progress等守卫不变。合法→负例→合法恢复使用workspace外独立source lock，正常Rust构建不调用Python、不安装依赖。

## 原 View 与公共 Content

建议 `paint::skeleton_on_surface` 为完整resolve后才写Scene的fallible入口；旧 `skeleton` 签名委托canvas兼容。SkeletonView增加canonical `set_surface/surface`，未知名先拒绝再保留旧context；旧render签名保留，由原View调用新的共享packet painter。不复制到adapter画Quad，不重新解释fill/边界/圆角。

公共 `composition::SkeletonContent` 只有一个原SkeletonView及可选受控语义配置，无模型值/clock/phase/timer。默认 decorative、semantic_hidden=true：装饰占位不发布重复“加载中”或假文字/进度节点。**命名非装饰用法可选**：沿#72原有ContentRole::Group+非空name/可选description、无value_range、无动作/焦点/TextSurface，不增protocol enum或native role。宿主需要告诉用户加载中/失败/完成时，用现有Text或Group及业务受控数据；不能根据Skeleton存在/消失推导成功。切换装饰/非装饰、name/description须验证后一次提交，空白非装饰name原子拒绝并保留原语义/mount。

measure读取同一父有限分配，render/paint/layout不另计算默认高度。父提供finite非负宽高（包含0），这是挂载前置条件。原Host已有clean_size清洗，并非fallible拒绝；本票不扩改全局Host。consumer校验自己提供的NaN/Infinity/负值并保留原分配，若新增显式尺寸setter也必须先拒绝再保留旧值，不能静默清洗成0再称拒绝。没有父高分配的intrinsic纵Stack不假装知道高度。Gallery18是原fixture；无新高度token/猜测4行。默认rounding取原paint的4，不发明新radius。父布局分配与实际ViewLayout一致；零/完全clip预期无ink，正常分配必须positive ink。

ContentTheme完整palette（含brand/external）与surface()透传原View；默认canvas、host inherited、祖先继承、node explicit、clear恢复均沿原ContentHost。未知Surface拒绝不换node/mount/context；不再建继承栈。Skeleton叶子不用祖先decorative foreground/icon_foreground染fill/边界，不向其他孩子提供新Surface。动态Theme只更新原绘制输入，背景相同不能推断Surface相同。

clip沿原SkeletonView，保留parent viewport与原frame布局；不顺手添加额外叶子clip，不用integer scissor吞掉非整数窄边界后报通过。必要边界是否在1x/2x实际可见、是否越界由GPU测定；若出现红例，只修原geometry/renderer必要契约，不增加第二套clip owner。

默认原Component被动路由：pointer/wheel/key/AX Press/SetValue/增减均无effect，不focus/捕获指针/请求编辑服务/发Submit。动态Theme/装饰语义/host loading更新保持原mount/reading锚点，同树原Composer marked/selection/binding/focus/caret/发送门禁不变；不请求reading reveal/anchor重置，不新建reader/editor。无shimmer、无新时钟、无动画帧请求；Reduced Motion/隐藏时也不启动额外引擎。

## 最小生产集合与依赖

共享Theme唯一writer与badge设计相同：`design/theme/rules.json`、`scripts/generate-theme-system.py`、`scripts/test-theme-system.py`、`crates/tekes-ui-design-export/src/theme_system.rs`、`crates/tekes-ui-tokens/src/surface_palette.rs` 的对应tests、`design/system.json`、`design/system.schema.json` 的rules_version const版本pin、原三输出 `design/theme/generated/instances.json`、`design/theme/generated/surface_rules.rs`、`design/theme/artifact.lock.json`。schema结构已容纳rest-only和七通道，无需形状扩展；但live `design/system.schema.json` 第411–412行rules_version const=1.7.0，规则版本升级必须同步此const、`design/system.json`契约/pins和生成输出/锁，全部由共享唯一writer处理。本阶段不改生产schema，不新增base.tokens/token-metadata颜色。不得把“无结构扩展”误写成“无需更新版本pin”，不人工重签旧产物。

组件候选只需 `crates/tekes-ui-controls/src/paint.rs` 原painter、`crates/tekes-ui-controls/src/feedback_view.rs` 原View、新 `crates/tekes-ui-controls/src/composition/skeleton.rs`、`crates/tekes-ui-controls/src/composition.rs` 导出。原Gallery路径 `crates/tekes-ui-gallery/src/gallery/workbench.rs` skeleton分支；新独立 `examples/composition-consumer/src/skeleton_content.rs`、`examples/composition-consumer/tests/skeleton_content.rs` 和原 `examples/composition-consumer/src/lib.rs` 模块导出。必要GPU复用既有harness/依赖，不增target/prepare。

公共类型 `design/gallery/public-api-surface.json`、Workbench `design/gallery/workbench.json` 和原binding生成输出 `crates/tekes-ui-controls/tests/component_registry_api.rs` 由主协调唯一metadata writer接收。原 `scripts/generate-component-capabilities.py` 同时输出 `design/gallery/capabilities.json`，该全局增量与coverage由主协调稳定批次处理，不手写hash、不更改历史证据状态。无protocol/runtime/platform/AppHost/Kernel变更。

两票可并行隔离adapter候选，但feedback_view/composition/Theme/metadata等共享文件只交candidate+hunk、单一writer串行接入，避开Progress/Tree/Panel窗口。先主协调接收本设计/边界与语义选择，再Theme稳定handoff，再组件编译/原页/GPU；本阶段不动live SDK。

## 有限验收计划与QA缺口

- 源/API：缺SkeletonContent公共类型真实编译红→同源绿；原View/painter/adapter同palette/Surface/packet/frame/radius；finite/zero/narrow/非整数/partial/fullclip；unknown Surface与空白name原子拒绝；继承/显式/clear恢复不换mount。
- 共享：7Surface×4mode×全部现Theme/default与workspace外外部Theme；已接收A的boundary对parent/fill≥3、完整七通道与旧32用途无回归；必要缺boundary/低对比/伪状态/未知组合/自洽重生成负例及合法恢复。保留原无边界同色52组合审计红事实，不重写为旧实现合格。
- 同树consumer：Skeleton+普通Text/独立Button+原Composer，宿主loading/error/success只是受控兄弟；装饰AX隐藏，命名Group可见无range/操作，动态Theme/name/description/业务状态保留mount/原reading身份及Composer状态。程序化Preedit/AX snapshot按其层次记录，不冒充物理IME/VO。
- GPU：先代表校准，再最终同源四模式、代表Theme、1x/2x、非整数原点、零/极窄/正常/部分clip、七Surface的必要可见性案例；原SkeletonView原静态geometry作参照，接受边界后的View与Content同源对照。真实像素读回fill/边界/parent，正常例positive ink和clip外无ink；同色fill靠必要边界仍应可见，空scene不能算成功。原Gallery实际skeleton入口after与默认品牌，外部Theme独立consumer分开，不用showcase代替原页。不增加shimmer截图或动画验证目标。
- 本阶段只完成静态设计/源审计/角色算术；build/test/GPU/实际窗口/协议/物理IME/VO/性能/发布均未验。实施时只用原build_lock/environment、+1.95.0 --locked --offline与原target/review-cache；不newtarget/prepare/clean/install/Git发布/清理。原有限验收满足后即可回填 #77/#55/#56，不追加全DS/native/物理IME/VO关闭前提；当前无PR。

## 主协调接收结论与后续责任

主协调2026-10-24固定选择A：原fill/frame/radius保留+共享必要border，两侧对比≥3；默认装饰+可选命名Group，无range/action；不新增AX role/ticker；父finite分配，无fixture18pt默认。仅装饰本体或无边界B替代方案不进入本阶段。

新policy代码由主协调后续拆出的**独立共享Theme有限依赖票**负责；adapter仍#76/#77。先唯一共享writer形成稳定source/hash/packet handoff，再绑定adapter实施/编译/必要GPU；保持#71进度包和旧32用途、全部通道守卫，避免共享文件双写。本设计会话不建票、不实施、不改生产schema。

早期设计/旧hash保留在 `pre-acceptance-design/`、`handoff-v1.json`；原源审计及同色52组合算术不改签。接收仅完成有限设计，不代表runtime验收；不增加全DS/native/物理IME/VO门槛。本会话不再委派、不联系旧保存会话。

## #77 隔离实施候选（2026-10-24，待主协调接收）

候选与证据仅位于 `design/reviews/skeleton-content-implementation-20261024/`；live SDK 未写入。原 SkeletonView 接收独立 skeleton/rest packet 并保留 fill/frame/radius4，SkeletonContent 默认装饰、可原子切换命名 Group。原 measure 使用父有限分配；consumer 对非法分配先验证并保留旧值。

实际 1x GPU 校准发现原 View 对自身 frame 再建 scissor 会抹除非整数 0.75pt 窄占位。候选仅在原 SkeletonView 保留祖先 clip 为裁剪 owner；仍以 frame 与父 clip 交集记录 visible，Quad 内边界和 shape AA 不扩张几何。无全局 renderer 变更。祖先 clip 经原 renderer ceil(min)/floor(max) 向内量化后严格零越界；父 clip 自身量化为空应无 ink。只保留同色内部而把必要边界全部裁掉也预期无 ink，不能算正向可见性成功。旧失败源码/程序/日志/读回保留，后续成功不追认旧结果。

完整验收阶段、精确命令及 source/binary/artifact 锁见该 QA 目录最终 handoff；协议投影/程序化 preedit 不代表真实窗口、物理 IME、VoiceOver、性能或发布验收。Issue 保持 OPEN，由主协调验收后串行集成共享 hunk。
