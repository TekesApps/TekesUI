# Motion Gallery 交接

日期：2026-10-15。范围：已有动画库与用户要求的模型/预设、调参、中途改目标；未增加其他 UI toolkit 或窗口。

## 文件与接口

- `crates/tekes-ui-motion/`：纯 Rust、零第三方依赖，24 项集成测试及 1 doctest。新增 `Spring::from_physical(mass, stiffness, damping)`，用于转换既有设计参数。
- `crates/tekes-ui-gallery/src/gallery/motion_preview.rs`：预设适配、属性通道、有限轨迹记录、目标和参数更新。稳定 slug `motion-foundation`，token 引用 `TOKENS`。
- `crates/tekes-ui-gallery/src/gallery/pages/motion.rs`：统一 Scene / TextEngine 绘制与共用布局命中，无 AppKit 控件或独立帧循环。

`MotionPreview::default()` 初始静止。用户选择预设或重播才开始运动。

```text
pages::motion::handle_input(&mut state, &Event, content: Rect) -> bool
state.animate(dt_seconds: f32) -> bool
state.needs_animation() -> bool
pages::motion::build(&mut PageCtx, content: Rect, &mut Scene, &state)
pages::motion::extent(content: Rect) -> Size
state.deactivate()  // 离页/失焦时释放拖动和 hover/pressed
state.set_reduced_motion(bool) -> bool
state.set_system_reduced_motion(bool) -> bool
state.manual_reduced_motion() -> bool
```

宿主只在 Motion 可见时推进它；`needs_animation()` 为 false 后关闭共享显示链接（其他可见动画也都为空时）。模块不建立线程、timer 或 display link。滑块捕获期间继续传递 drag/up，即使指针出了视口；坐标与绘制统一使用 `Viewport::content()`。宿主负责 Page 枚举、registry、侧栏 section、viewport 和窗口接线。

## 参数来源

Gallery 编译时读取 `design/source/motion.json`，测试确认其与 `design/tokens.json` 的 `proposal.motion` 投影一致。使用现有 JSON 的：

- 6 个 duration：instant、stateChange、geometry、listExpansion、expansion、press；非零值使用 JSON easing.standard。
- 4 个 physical spring：micro、reveal、panel、feedback，保留 JSON 的来源说明及 m/k/c。
- 转换：`ω = sqrt(k/m)`，`frequency_hz = ω/(2π)`，`damping_ratio = c/(2*m*ω)`。

不是把 duration 的倒数当 frequency，也不是 Apple 的 perceptual duration/bounce 映射。原研究文档第 5、6 节的临时预设与 motion.toml 建议保留历史，但不用于当前 Gallery。库内早期 `presets.rs` 同样不参与此页的设计系统适配。

滑块是实验覆盖：Hz、阻尼比、Tween 秒数、Spring 释放速度。覆盖后标明自定义，重新选择预设恢复 JSON 值。Release velocity 只在重播起步使用，运行中换目标与调参数继续保留当前弹簧速度。

显式改目标或调整运动参数会取消待触发的中断演示，避免旧的演示事件随后覆盖用户输入。Reduce Motion 有独立 manual/system 状态，有效值为 OR；页面按钮切换 manual 值，系统优先，系统恢复后保留 manual 选择。重复偏好更新不会重复派发完成/取消事件。

页面最低文档宽度为 840pt，高度为 816pt；窄窗口使用已有水平视口。绘制、命中和 extent 使用同一宽度，底部事件行包含在滚动范围内。

## 已验证

- 内核 24 测试：三种阻尼与独立 RK4 数值积分对照、临界极限、物理参数转换、重定向位置/速度连续、取消与有限截止。
- 预览独立测试：JSON 一致、10 预设收敛、连续 100 次改目标/调参、Reduce Motion 中途取消、Instant 无额外调度、短 Tween 在结束前改目标、NaN/极端 delta、slider 捕获与释放。
- 最终在实际 `tekes-ui-gallery` 二进制测试目标中运行 `motion` 过滤：15 项通过。新增覆盖演示取消、真实按钮命中下 manual/system 独立、平移/窄文档边界、四主题 × 两倍率的动态文字缓存及减运动按钮绘制层级。
- 每个主题/倍率下连续构建 240 帧，预热后 CoreText shaping 计数不增长。内核 Clippy（all-targets）及独立页面 harness Clippy（all-targets）零警告；未代替共享 Gallery 全量 Clippy。
- 独立 harness 直接编译此 worktree 的页面、状态、TextEngine、Scene 与 renderer，未创建第二套 UI。四主题 × 1x/2x 共 8 张离屏截图，路径 `design/reviews/motion-gallery/`。
- 看图后修正 Reduce Motion 按钮的绘制层级、动态数字字距、禁用参数的数字颜色；动态数字逐字缓存，不为每帧新数值重新 shaping 一整串文字。

headless debug 的场景构建诊断曾测得 p50 0.369ms / p95 0.442ms；这是 CPU 编图的局部诊断，不含 GPU、合成、输入到呈现与真实刷新间隔，不能作为 8.33ms 或真实 120Hz 验收。

## 尚待主聊天统一验证

- 完整 Gallery 原生窗口中的点击/拖参/连续改目标与页面滚动。
- 可见性和显示链接生命周期：离开 Motion、收敛、Reduce Motion 后无空转，重新进入不跳时钟。
- 真正 120Hz 硬件上的刷新间隔与整帧预算 8.33ms；不得由 120Hz 数值采样或 headless CPU 时间推断。
- 全部主题、缩放、视口、键盘/AX 交互与其他正在修正组件的集成回归。
- 系统 Reduce Motion 由平台读取并投递，页面提供显式覆盖演示；需要和实际系统偏好变化接线核对。

Apple 官方研究来源见 [motion-foundation.md](../design/motion-foundation.md)。

## 最终自测命令

```sh
cargo test --offline --locked -p tekes-ui-motion
cargo clippy --offline --locked -p tekes-ui-motion --all-targets -- -D warnings
cargo test --offline --locked -p tekes-ui-gallery --bin tekes-ui-gallery motion -- --test-threads=1
```

这轮没有改 Host/Gallery/Perf/平台/导航接线，没有新建窗口、聊天，也没有提交或推送共享 WIP。Host/Perf 的四项已报告问题由主聊天统一修复和运行验证。

## 2026-10-15 Apple defaults 增补

本节更新上面的初始 10 预设说明：默认 bank 现为 Apple，9 项（Default、Spring、Smooth、Snappy、Bouncy、Ease In、Ease Out、Ease In Out、Linear），默认选中 Spring、打开仍静止。`PresetCatalog::{Apple,DesignSystem}` / `select_catalog` 切换 bank；项目 JSON 的原 10 项没有改值。点击 bank 会重放所选默认项，Host 不需要新增接线。不要假定 `presets.len()` 恒等于 10。

具体参数与官方链接见 [apple-motion-defaults.md](../design/apple-motion-defaults.md)；跨品牌来源与尚未实现的配方建议见 [motion-references.md](../design/motion-references.md)。Apple 公共数值 oracle 63 样本核对通过；内核现 27 项测试 + 1 doctest，实际 Gallery `motion` 过滤 16 项通过。两份 Clippy（内核与独立页面 harness）通过。新截图默认状态改为 Apple Spring，保留相同中断时间与捕获时刻。真实窗口与 120Hz 仍由主聊天统一验证。

## 2026-10-16 四种可交互配方

新增 `tekes_ui_motion::recipes::{RecipePlayer,RecipeKind,RecipeParameters,Phase}`，无依赖、无时钟/线程；共享元素、多属性面板、分阶段成功、阅读补偿四种示例使用可打断的独立轨道和可取消延迟阶段。`PresetCatalog::Recipes` 是第三个 bank（4 项），上方按钮进入。主体时长/阻尼/延迟/幅度可调；阅读例禁用无意义的延迟参数，并有左右补偿对照及向上/下阅读按钮。暂停期间改参数不自动继续；展开继续，重播重置；演示反向计划被显式用户操作取代。

验证：内核 33 项 + 1 doctest；实际 Gallery motion 过滤 18 项（含实际命中下暂停/继续/反向/调参/阅读滚动，以及四主题×两倍率×四配方的动态文字缓存和有限几何）均通过。内核和独立页面 harness all-targets Clippy 通过。Gallery binary 构建通过，现有 icon_catalog Entry.label 有一项未使用警告，未改其文件。三种配方的四主题×两倍率离屏图已生成，选取三张放 `design/reviews/motion-recipes/`，已看图。阅读截图与全部原生窗口交互交由主聊天统一验收。没有触碰共享 Host/Gallery/Perf/平台/导航或 Cargo 接线。

临时持续 Gallery 进程 PID 8084 已确认不存在，也无本聊天的持续 headless renderer；不再启动动画/性能窗口，以免干扰主聊天的 120 Hz/CPU 测量。最新二进制 `/tmp/tekesui-motion-preview-check/target/debug/tekes-ui-gallery`。操作说明与边界：[motion-recipes.md](../design/motion-recipes.md)。

## 2026-10-16 十种新增配方收口

`RecipeKind::ALL` 现 14 项，新增列表重排、Shared Axis、Fade Through、DragSnap、Stagger、Popover、Press、Skeleton、Numeric、PageSwitch；所有 owned match 已穷尽。四个主属性轨道之外新增六个项目轨道，用于稳定 ID 的列表与交错队列，数字示例有独立连续数值通道。UI 增加继续当前按钮，三个选择行；HEIGHT 从 816 改为 858pt，阶段/图表/控件/页脚一起下移42pt，保留固定宽度和共享 extent。拖拽/按压捕获复用原 `pressed` 路由，未改 Host/Gallery。

收口验证：`cargo +1.95.0 check --offline --locked -p tekes-ui-gallery --target-dir /tmp/tekesui-motion-preview-check/target` 通过；内核37 tests + 1 doctest 通过；实际 Gallery motion过滤19 tests 通过（14配方×四主题×两倍率×120帧的缓存/有限几何，以及真实函数命中路径的拖拽捕获、失焦、按压拖出/拖回、暂停/反向/继续/调参）；内核all-targets Clippy与独立页面harness lib Clippy均通过；scoped diff-check通过。

按主聊天本轮隔离请求，没有启动原生窗口、headless GPU 渲染或性能测试。新十种的真实窗口效果/像素验收和全帧性能仍待主聊天统一完成；原三张截图不作为这十种的新截图。代码已收口，通知主聊天后冻结本批 Gallery 所需源文件，避免干扰其 ScrollArea/TextEditor/120Hz 验证。用户操作说明见 motion-recipes.md 扩展章节。

## 2026-10-16 斜向文字扫描增补

用户新指令新增第15项 TextScan，仍仅改 owned motion lib/model/page/tests/docs。新增 RecipeParameters.scan_angle 与 TextScanBand::span，模型是单次线性扫描+可取消启动延迟。页面通过文字字形与1pt行切片柔边clip形成斜向亮带，无共享renderer改动。四参数为时长/方向角/启动延迟/亮带宽度；Reduce Motion 静态正文。第三行刚好容纳15项，HEIGHT仍858。

内核39项+1doctest通过，Gallery motion过滤19项通过（现涵盖15配方四主题两倍率缓存和几何），内核Clippy通过；原生视觉/像素与扫描额外图元的整帧开销未验收。没有启动原生窗口、GPU离屏渲染或性能任务，避免干扰主聊天当前测量。主聊天的protected WIP hash需只对本轮上述owned文件的预期变更更新，不能把这些新用户授权修改当作无意覆盖。

## 2026-10-16 旋转组增补收口

用户在TextScan之后又明确要求水平/垂直旋转、加速与刹车。本轮新增Rotate2D/FlipX/FlipY/SpinBrake/RotateSettle（索引15–19），ALL20，选择区四行、HEIGHT900，下方所有内容和按钮再下移42pt。新增rotation_degrees/brake_bias参数、RotationAxis/project_rotation数学投影与私有AngularBrake连续初速度五次轨迹；卡片矢量面使用页面自有有界扫描行填充，不改renderer。角度改参会保留当前实际角度/角速度。SpinBrake制动偏置代替阻尼滑块；其他旋转显示阻尼。默认值与实现边界见motion-recipes.md旋转组。

内核42项+1doctest通过（新增角速度/加速制动、角度改参连续、X/Y/Z投影）；实际Gallery motion过滤19项通过，覆盖20配方四主题两倍率缓存/有限几何和控制命中。内核Clippy通过。未启动原生/GPU任务，尊重统一性能验收隔离；尚无本组原生像素/高倍率边缘/8.33ms验收。源码收口后不再构建、测试或预览，待主聊天统一验证；本轮仅owned库/model/page/tests/docs变动属于用户预期修改。

## 2026-10-16 Typing收口与参考源边界修复

用户在旋转之后明确要求研究Claude Code文字浮现与时间片批量Typing。本轮新增零依赖typing.rs与6项单测；Recipes新增Typing/StreamTyping（索引20/21，总22项），固定段落排版、完整预分割显示单元、字形簇归属过滤后按每单元alpha绘制。流式速度0默认按到达即显；正数限速且积压最多250ms；单字/多字到达、淡入、暂停冻结、消息替换取消旧队列、ReduceMotion均覆盖。下方由无意义的平直几何曲线换为Typing单元显现网格和到达/显示计数。选择区五行、HEIGHT942。

验证：内核49项+1doctest、实际Gallery motion过滤20项通过（含22配方缓存/有限几何及Typing文案切换计数）；actual Gallery cargo check通过，内核alltargets Clippy和页面harness lib Clippy通过，pure-Rust source guard通过，scoped diff-check通过。研究来源和宿主Unicode分段契约见docs/design/typing-engine.md。没有启动原生/GPU或性能任务，本轮原生视觉与真实输入流/Markdown/AX集成仍未验收。源码/测试已收尾，再次冻结供主聊天统一原生性能验收。

同时按主聊天边界要求，将apple_oracle.swift原样迁至项目外 `/tmp/tekesui-apple-motion-reference/apple_oracle.swift`，没有删除原脚本。项目内保留CSV（Rust测试读取）与同源63样本JSON、reference README，相关文档路径已更新；不再混入Swift源文件。pure-Rust guard当前通过，未改guard规则或共享接线。保护WIP的hash应针对这些用户授权owned文件增补更新。

### 2026-10-16：Symbols 效果族新增（用户继续授权）

- 新增零依赖 `tekes-ui-motion/src/symbols.rs`：14 个官方效果族/变量描绘模式，三种分层组合、二维位移/缩放/旋转/透明度、弧长描绘与替换权重，以及路径/对应关系缺失的显式降级。官方版本来源与非兼容边界见 `docs/design/symbol-effects.md`。
- Gallery Motion 增加第四个 **Symbols / 图标** bank；原 22 个 Recipes 数量、942pt 页面高度不变。自制三层铃铛对比 Whole/ByLayer/Individually，全部复用参数、暂停继续、目标 A/B、重播、中途反向；Symbols 的分层参数是轮比例，已避免将其误用为 demo 的启动延迟，最短 0.3s + 最大错层也会在 45% 时反向。
- 独立预设：Bounce 0.7s/1，Pulse 1.6s/2，Breathe 2.4s/1，Wiggle 0.9s/3，Rotate 1.6s/1，Scale/Appear/Disappear 0.6s，Replace/Magic 0.8s，VariableColor 1.8s，Draw 家族 1.2s。强度 0.6，分层比例 0.12，均为 TekesUI 调校值，不是 Apple 官方常量。
- 实现边界：模型与自制图标场景完成；未接入现有图标资源的注释/路径/锚点，未实现通用 SF Symbols 导入、全部变体或复杂非刚性编舞。Magic 仅保留已知共同元素并插值徽记对应顶点；无原生逐帧等价声明。
- 验证：motion 55 集成测试 + 1 doctest 通过；真实 Gallery bin 的 motion 过滤 21 项测试通过，含全部 36 个 Recipes/Symbols 场景四主题双倍率的 CPU 构建、标签缓存与有限几何；最后 Symbols demo 时间修正另单测通过。motion 全 targets 和页面隔离 harness lib Clippy `-D warnings` 通过；Gallery check、纯 Rust 边界（512 文件）和 scoped diff check 通过。
- 持续遵守主集成安排：未开原生窗口、未做 headless GPU 渲染、未跑性能测试；原生视觉、真实输入和 120Hz/CPU 未验收。编译与 CPU 检查结束后冻结，主会话统一验收。只更新 motion 库与 Motion 两文件和相关文档的预期哈希；图标资产、Host/渲染器/性能代码不在本轮实现范围。

### 2026-10-16：现有图标资源接入及原生预览

用户明确要求补全路径/分层并立即打开 Gallery。新增 icons/src/animation.rs 的 8 图标注释，SVG 几何保持单一来源；Gallery 新 symbol_assets.rs 解析和光栅化原路径，26 项 Motion 测试通过。8×4×2 完成态与原 SVG 像素严格相等，8×14 实际效果光栅化、路径引用无遗漏重复、共同圆环拓扑与降级、替换端点均通过。

为避免逐帧创建纹理，本轮对 image_pass.rs / renderer.rs 增加了窄接口 update_image（原位上传，ID/texture/bind group 不变）及 offscreen 更新验证，真实 GPU 测试通过。Gallery::prepare 只增加 Motion 的资源准备调用，main.rs/text_demo.rs 的主会话布局 WIP 保留。Icons 页面及已确认 SVG 未更改。

原生独立预览为 target/TekesUI Motion Preview.app，bundle com.tekesui.gallery.motion-preview。没有替换或关闭主会话 Gallery，已用 CUA 实际观察/操作 Symbols 页、目标图标呼吸、暂停在进度 0.24（速度0）、继续后中途反向（速度负、目标0）、路由弧长描绘、新建目标到完成目标的共同圆环替换、设置图标摇摆。构建签名/源文件/静态资源哈希见 design/reviews/symbols-icons/preview-build.json；原生动作证据在本会话 CUA 截图。未跑新的基准，不能据此宣称120Hz达标。

最终补充：Gallery/icons/renderer 的 all-targets Clippy `-D warnings` 均通过，图标 lookup 1 项通过，纯 Rust guard 516 文件通过；预览代码签名验证通过，原生 PID 29162。用户随后自行切换了预览页面，因此停止 UI 自动化，未将最后被中断的拖参操作算作通过。

### 2026-10-16：图标120Hz / CPU真实窗口矩阵

用户要求测多图标、连续调参、中途切目标。新增 symbol_perf.rs，perf.rs 新Symbols场景（真实MotionPreview控制/路径光栅化/纹理上传/原生呈现），3个 scripts复现与汇总。只测量，生产动画实现未改；严格8.333ms/实际presented收据阈值不放宽。

M4、内建120Hz/2×、正式组GPU查询关闭：3实例30s CPU32.21%/长呈现间隔1；24持续60s CPU46.24%/3；24调参60s CPU45.81%/2；24切目标60s CPU46.49%/4；72同时调参切目标30s CPU98.90%/40、最长25ms、build p99 10.80ms；42全14效果20s实际长间隔0但callback漏1/CPU峰超预算。不能宣布稳定零掉帧通过。

静态纹理24实例诊断CPU16.85%/build均0.047ms，仍2次呈现长间隔；GPU查询24实例诊断GPU均0.418/p99 1.537/max10.899ms，长呈现间隔3。主要CPU成本在SVG重绘和上传准备，但不能把所有异常归给motion；24稳态drawable等待最长13.39ms。所有正式/诊断组模型错误0、presented收据完整、结束所有控制器pause、idle额外callback0，5秒CPU0.037～0.087%。

原始10组（含2smoke）gzip无损归档、原文/压缩哈希和二进制源码SHA：design/reviews/symbols-performance/。完整本地日志/每帧JSON/进程清单在target/symbol-perf/。测量期间为主会话滚动修复只暂停调度父进程，当前组与idle完整结束，之后恢复相同冻结二进制；后来的scroll修复源不自动继承这份性能结果。

8项perf单测/严格Clippy/guard542/scoped diff通过；全部基准和构建结束，资源已交还主会话。普通用户预览未关闭或切页。


### 2026-10-16：GPU图层缓存、匀速旋转及实机复测

新增外部时钟恒角速度LinearRotation，Symbols第15项（Recipes仍22）；GPU TransformedImageQuad与分层图集复用几何/alpha动画，Draw/Magic共享变形保留CPU动态路径。64静态完成态exact及120运动组合coverage/centroid通过；motion57+1doc、Gallery Motion31/perf8、GPU18/offscreen23、严格Clippy和guard563通过。

独立冻结签名二进制75114371cb3d7e84b5725c0d0d69786cbf0366e60fb0261f1d19feafd09ef3c9完成原8组+Linear24共9组。24混合CPU46.24→19.32%、build均3.208→.847ms；72压力buildp9910.800→5.432ms。Linear24 CPU10.09%、0重绘/上传、idlecallback0，但actualgaps64，严格120Hz未通过。后台其他项目rustc/du干扰，不宣称受控A/B；参数/retarget跳过渲染且旧serial重复，notpresented不能当独立drawable数。

全部原始gzip/SHA/汇总/限制见design/reviews/symbols-gpu-cache/，旧symbols-performance证据不变。native资源已交还主控做修正收据、可见性、drawable/displaydriver A/B，本轮冻结二进制不包含那些后续变更。不再编译或启动GPU。
