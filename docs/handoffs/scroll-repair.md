# Scroll / Text 基础组件修复与验收

本轮没有增加基础组件。实现及参数保留纯 Rust / WGSL；JSON Design System 的视觉 token 保持，弹性参数由 `design/behavior/scroll.json` 自动生成 Rust 常量。此次验收不是发布，也没有改写既有参考 PNG。

## 已完成的修复

- ScrollArea、Text、TextEditor 的 live panel 撑满真实窗口剩余 viewport，底边保留 JSON Section 间距，不沿用固定校准画布高度。多行内容在自身内部滚动；ScrollArea 页面没有外层页面滚动。
- 修复性能测试路径：窗口在 Created 后被系统从 960 pt 高调整到 827 pt 高时，Perf 也立即更新布局。原本普通 Gallery 已适配，Perf 内部 Gallery 却仍使用启动高度，导致用户看到测试页底部越界。
- 原生窗口在 1440×827、1000×520、720×480 下截图；布局单测包含更大窗口。文本选择、插入光标和单行文本按同一排版行框定位；四主题、1x / 2x 的像素测试输出可放大检查。
- Text 页是只读、多行、可选择复制、可滚动展示；TextEditor 为可编辑展示。两者改用原创连贯长文 `fixtures/text/gallery-reading.txt`，不再用每行重复文本。ScrollArea 测试文档没有多余末尾空白行；框内上下 padding 相等。
- 横纵独立 Enabled / Disabled / Auto；禁用方向也限制回弹、程序滚动和滚动条捕获。Icons 横向 Disabled、纵向 Auto，窄窗口改成一列，搜索框随可用宽度伸缩。
- 回弹时滚动条按可见比例缩短并锚定边缘，回弹结束恢复。原生 400/3000 文档越界 31 pt 后 knobProportion = 0.123，Rust 同值；最小尺寸 thumb 也保留缩短反馈。
- 修复细线滑过亚像素位置时亮度变化：重叠的内外 AA 覆盖率应相减，不能相乘。修复前 0.5 设备像素线在 16 个相位的积分亮度为 128–143，修复后差值不超过 2。22 项真实 GPU 像素测试通过。
- 保留旧参考 PNG。原三页 2x 内容仍逐像素精确；1x 仅允许解析识别的 AA 重叠像素变化，每通道最多 32、每图少于 2000 像素，其他位置仍逐像素比较。这是修复的明确例外，不是重新批准新外观或任意放宽截图容差。

## 帧节奏及 CPU：仍有未完成项

显示链接请求 120 Hz，标称间隔 1000/120 = 8.333333 ms（8.33 ms 为舍入值）。滚动事件与 redraw 合并到显示回调，避免同一次刷新之间额外提交；修复 pending 标记的短路残留。Metal 使用三个 drawable（wgpu frame latency = 2），FIFO / VSync 保持。

公开 CAMetalLayer 子类的 nextDrawable + MTLDrawable.addPresentedHandler 收集实际 presentedTime。默认预览关闭测量观察器；基准开启。报告同时保留回调、CPU、drawable 等待、GPU pass 和实际呈现，不能把回调未丢等同于实际未丢。

**严格零掉帧尚未通过，不能把本轮标为全部完成。** 详细原始指标见 `design/reviews/scroll-repair/verification.json`。失败记录保留，不以短测或 p99 覆盖。

- ScrollArea 60 秒（GPU 计时开启、旧测试 viewport 路径）：7199 帧，回调丢 3 次刷新；CPU 帧峰值 10.817 ms，主要在 drawable 等待，GPU 峰值 7.372 ms。
- 修正测试 viewport 后，关闭 GPU 查询的 60 秒对照：7198 帧，实际呈现仍有 4 次超过 1.5 刷新的间隔；CPU 帧峰值 16.484 ms，其中 acquire 16.021 ms。p99 实际间隔 8.333333 ms，但不能因此宣称每帧达标。GPU 查询不是唯一原因。
- 上述无 GPU 查询对照：平均进程 CPU 9.959%，250 ms 采样峰值 24.648%；测量后五秒空闲均值 0.0213%，显示链接回调 0。100% 表示一个逻辑 CPU 核，含 user + system。250 ms 峰值不是瞬时指令峰值。
- 连续 TextEditor 滚动 30 秒（GPU 计时开启）：仍有 1 次实际漏帧；GPU 峰值 6.314 ms，CPU 帧峰值 12.218 ms，平均进程 CPU 18.118%。
- 实际呈现 target-to-present 延迟另列，约 25–33 ms 级；8.33 ms 指刷新节奏，不是输入到显示的全程延迟。

下一步仍需围绕 drawable / 合成器背压与调度做根因归属和优化。不能通过关闭 VSync、隐去峰值、扩大丢帧阈值或只报 p99 来宣布通过。测量在用户共享桌面进行，其他项目的编译和系统负载不可假定已完全隔离。

`target/scroll-repair/perf-scroll-cpu-only-60s.json` 是一次构建失败后误运行旧二进制的历史文件：缺少 gpu_timing.enabled，仍有 GPU 计时，**不作为 CPU-only 证据**。真正的 CPU-only 对照是 `perf-scroll-validated-cpu-only.json`，enabled=false。未完成且无报告的尝试也不列为通过。

## NSScrollView 对照的已知边界

原生测量工具是纯 Rust example，使用 NSScrollView / NSTextView 作为测量参照，没有嵌入 Gallery。每 20 pt 有编号刻度与位置，固定 HUD 显示 NS / Rust 位移、pt/s、帧间隔和阶段，便于录屏。

`native-top.json`：22 个输入，前一秒 RMSE 约 0.843 pt，释放后约 0.811 pt；原先同类轨迹误差约 28 pt。`native-momentum.json`：44 个输入，完整原始误差约 4.619 pt（含原生采样一个刷新左右的相位差），尾段回弹误差约 0.250 pt。两个原始轨迹重放纳入 controls 回归测试。

**快速反向拖动、进入内区后的释放尾段仍未对齐。** `native-reverse.json` 保留未通过数据，不参与通过门槛；原生有额外尾段移动。补发惯性阶段的对照也未闭合差异；构造底层 CGEvent 的实验未形成有效对照，已从工具代码移除。当前是本地公开 NSEvent 适配器，不是物理触控板捕获，不能仅凭该工具把差异全归于 Rust，也不能声称 NSScrollView 全功能等价。需要继续核对真实阶段序列，再决定是否实现原生尾段行为，避免把系统已提供的惯性重复积分。

## 验证与来源

完整 raw perf JSON / 日志 / 原生窗口截图 / 高倍率文字图在 `target/scroll-repair/`。小型指标、原生曲线与说明保存在 `design/reviews/scroll-repair/`。截图和性能测试分开运行。

测量二进制 SHA256：`a6bbc87af1c4e652e7800599cf90478e4e38e294319ad9fb6e0552dcb44dd02a`；对应源码清单 `target/scroll-repair/frozen-source-sha256.json`。后续预览候选含 Icons 窄窗口修复，使用第二份 delivery-source 清单，不能把旧测量假称为第二候选的严格验收。

动画聊天在收尾期间仍有经用户授权的新增内容，主目录出现过中间态编译错误；全部 WIP 保留，未回退其他聊天。统一预览和检查采用可复现的源码快照，生产新增的后续动画没有因主控验收而被静默覆盖。预览二进制来源和最终测试数另存 `design/reviews/scroll-repair/delivery.json`。

项目纯 Rust 源码边界检查已通过；动画用 Swift 参考脚本由其作者原样迁出项目，只保留数值及来源。JSON Design System 检查通过。真实物理触控板、真实中文候选窗口及完整无障碍不属于本轮已通过结论。


最终预览已打开在 ScrollArea；原生窗口滚到末尾时，最后一行与底边留白均在框内，框的底边留在真实 viewport 内。独立预览静止十秒，top 每秒采样的 CPU 中位数 / 最大值均为 0.0%（一位小数精度，不表示绝对零 CPU）；完整 log 保留。交付候选 391 项通过、0 失败、1 项跳过，严格全 workspace/all-targets Clippy 通过。并行聊天后续新图标/动效 WIP 未覆盖，也不包含在上述冻结验收结论中。


## 容器宽度变化与文字重排（2026-10-16）

统一使用“容器”描述可用布局区域。Text / TextEditor 的容器变窄、变宽时：

- 按物理像素量化换行宽度；同一个可见像素内的小数抖动不触发重复排版。
- 连续缩放保留最初的 UTF-16 阅读锚点和行内偏移，不逐帧重新捕获行首，避免反复换行后阅读位置逐渐漂移。原来在末尾时保持贴底；用户重新滚动或改文会重建锚点。
- 每个 live 文本容器只保留当前文档的 CoreText `PreparedText`，宽度变化复用属性字符串、typesetter 和 UTF-16 映射；文档修改即替换，不把历次输入加入永久标签缓存。新段落完成后再整体替换，重排期间没有主动清空文字的中间状态。
- 缩放前可见的编辑插入光标保持可见；用户已经滚离光标时不强行拉回。选择范围保持 UTF-16 索引；选区与新行框重新计算。
- 自动换行的多行容器明确禁用横向滚动、纵向 Auto，消除行尾空白/小数宽度导致的细小横向条；单行输入为横向 Auto、纵向禁用。

相关测试覆盖连续缩窄/放宽、往返阅读位置、末尾贴底、用户重新滚动、改文失效、可见/已滚离光标、只读/编辑两种模式、两种倍率，以及 CJK / Latin / emoji / 组合字符 / RTL 的 prepared 与 fresh 段落精确一致。

优化前后 68 张文字渲染 PNG 完全相同（该对照只针对复用文档准备数据的改动）。恢复最初宽度后画面逐像素相同，各测试帧都有文字像素。一次优化构建的 68 个重排样本中位 3.403ms、p95 5.186ms、最大 5.678ms；范围只含文字 layout，**不含 GPU 与实际呈现，不能作为整帧 8.33ms 通过证据**。未复用文档的先前同类样本最大 9.256ms；单次样本不代替长期性能统计。

原生自动缩放脚本对 Text / TextEditor 各执行 241 个尺寸更新，再捕获 720 / 1080 / 1440pt 宽度。该轮提交 492 次 redraw，进程正常退出；6 次截图中 1 次系统截图失败，后用 CUA 实际拖动补看窄容器 TextEditor、选区及加宽后的排版。截图不是每个刷新周期的录屏证明。该脚本含启动及静置等待的 15.04s CPU 均值 28.09%（100%=一个逻辑核），不能与前面的稳定滚动 CPU 混用。

并行源与冻结候选曾共享 target，live symbols 测试出现旧依赖接口的编译错误，而当前源码导出和 variant 均存在；未回写或重置他人源码。为规避交叉缓存风险，最终重排候选改用独立 `target/scroll-repair/reflow-target` 构建目录，来源与验收收据见 `design/reviews/scroll-repair/reflow.json`。此候选基于先前交付源，保留本次文字修复；其他聊天后续 Symbols 基准开发仍在 live 源中，不能把候选验证冒充整个持续变化的 live workspace 验证。

本次最终独立构建：Gallery / text 共 182 项通过、0 失败、1 项按原条件跳过，release/all-targets 严格 Clippy 通过。最终 Layout Review 在原生 1440→720→1080pt 缩放并选中全文后确认横向条已消失，底边及末行仍在容器内。预览保持打开。


## 双指越界后反向滑动费力（2026-10-16）

原生公开 NSEvent 校准复现：从距离顶部 20pt 的位置，手指输入先向外 30pt、再反向 30pt；NSScrollView 回到文档内约 20pt，修复前 Rust 仍在弹性区（约 -8.5pt）。旧模型将惯性越界首笔的 1:1 处理用于手指输入，随后却按 stiffness=20 消耗反向位移，造成额外反向行程；向外跨得越多，反向越费力。

修复仅区分手指与惯性：手指的第一笔越界部分同样应用弹性阻力，反向消耗与该阻力保持一致；惯性首笔路径保持。没有降低正常内容区的滚动比例，也没有调整视觉 token。语义记入 scroll behavior JSON 的 boundary_entry_policy，数值仍使用 JSON 生成常量。

新增顶部/底部各 100 次独立手势与 100 次不抬手反复越界/反向回归；等量往返输入恢复初始位置，没有累计“欠下的位移”。原生 active-finger 轨迹也纳入固定回归。14 项 controls 测试通过，包含原有顶部回弹和底部系统惯性记录，release/all-targets 严格 Clippy 通过。

实际原生校准前后，活动手指阶段按已说明的一帧 bounds 发布差异对齐，误差 RMSE 从约 150.59pt 降到 0.125pt、修复后最大 0.5pt；原始记录及范围见 finger-reversal.json。该对照没有认证释放后的额外原生滑行，不替代真实双指体验确认，也不把此前整帧 120Hz 零掉帧与快速释放尾段未通过项标为已完成。


## 图标动画性能并入统一验收（2026-10-16）

已读取并核对 `design/reviews/symbols-performance/summary.json` 与 README，独立验证 raw manifest 中全部 10 份 gzip/原文 SHA256、长度和 JSON 可读性。正式结果未达到严格 120Hz 零掉帧：24 实例三种负载各 60s 出现 2～4 次实际呈现长间隔，平均 CPU 约 45.8%～46.5%；72 实例压力 30s 出现 40 次，CPU 约 98.9%（100%=一个逻辑核心），建场景 p99 10.80ms。42 实例全效果短测未出现实际长间隔，仍因回调/CPU 峰值不满足而保留未通过。

固定纹理诊断仍有呈现长间隔，说明不能把全部问题归到动画数学或逐帧重画；逐帧 SVG 光栅化/上传的主要 CPU 开销、drawable 等待和实际呈现应分别继续优化与验证。停止后额外回调为零和低空闲 CPU 属于已通过项。完整负载定义、指标、限制、冻结二进制及每帧证据以该目录为准；本次双指滚动修复不自动继承另一份冻结动画二进制的验收结果。

## #12 / T43-C：有界 AppKit CPU / 滚动状态参照（候选）

原 `scroll_oracle` 默认校准路径不变。新增可选模式只比较同一内文区域的进程 CPU 和滚动状态；不替代原 CPU/GPU 帧耗时、实际呈现、零 gap、同机非劣化验收。窗口/完整 Gallery chrome 的工作量不同，不能把正文参照叫作整页性能等价。

Gallery 新程序以 `TEKES_PERF_REFERENCE=1` 开启附加观测。每帧只保存 target/logical/visual 三个标量；停止后的 JSON 报告一次性导出 `reference_fixture`：当前已布局 TextSurface 的真实 inner/outer rect、wrap width、原文、字体 PostScript 名/字号/回退、行 UTF-16 范围/高度/基线和滚动 extent。导出经 `TextDemo` 和 `Gallery` 的只读方法，不重排、不改变控件或字体行为。新字段与程序身份只属于新构建，不能倒贴旧 f3a 程序或旧报告。

在主协调原 guard/runtime 独占窗口内，先用新 Gallery 程序取得完整可见的 ScrollArea 报告，保持英文、内建屏、同机/OS/电源/scale：

```sh
TEKES_PERF_REFERENCE=1 TEKES_PERF=scroll:60 TEKES_PERF_CONTENT=scroll-area \
TEKES_PERF_WORKBENCH=1 TEKESUI_GALLERY_LOCALE=en TEKESUI_WINDOW_PLACEMENT=builtin \
TEKES_PERF_GPU_TIMING=1 TEKES_PERF_FRAME_LATENCY=2 TEKES_METAL_DISPLAY_LINK=0 \
TEKES_PERF_OUT=/absolute/fresh/gallery-reference-input.json /absolute/new-release/tekes-ui-gallery
```

现有 `measure-scroll-pages-quiet.py --reference-metadata` 显式传递该观测开关；默认仍清理继承的 TEKES_PERF 环境变量。此处仅说明原程序参数，实际执行仍绑定既有 quiet runner/进程组 timeout/清退与二进制 hash；不绕过 admission。Gallery 的既有性能拒绝理由原样保留。`reference_fixture=null`、未完成/隐藏/失焦报告或缺少 reference_scroll 时，不可启动参照模式。新环境变量不要在旧程序上使用后宣称取得了新元数据。

```sh
TEKES_SCROLL_REFERENCE=/absolute/fresh/gallery-reference-input.json \
TEKES_SCROLL_SECONDS=60 TEKES_SCROLL_ORACLE_OUT=/absolute/fresh/appkit-reference.json \
/absolute/new-release/scroll_oracle
```

oracle 在建窗前校验 JSON、原始内文（与公开夹具 `gallery-reading-en.txt.trim_end()` 完全一致）、必要有限正数、完整 Gallery 观测和新输出路径；秒数必须有限且大于零，并与输入Gallery报告相同，warmup须为原1.5s。当前有界模式仅支持一个非彩色已解析字体和一致的实际行高，遇到回退字体/变行高明确拒绝，不替换为任意字体。AppKit 使用该 PostScript 字体，核实际解析名、backing scale、viewport、NSLayoutManager 行 UTF-16/行高/基线和 used height。行几何容差沿原 comparison-method：1x 半点、2x 四分之一点，文档总高一点评差；不同排版是参照输入未准入，不据此宣称产品渲染缺陷。原始内文一致不证明栅格、完整页面或呈现等价。

一次真实本地 scrollWheel 事件仍用于提供原公开 NSEvent adapter 的窗口/位置种子。它被消费为种子，不作为正式手势；不能冒充物理触控板采集。无种子不开始计时，主协调须使用原进程组 timeout（60s 模式外层180s），10s 内未获得种子应停止，不依赖旧36000回调后备退出。

种子后先预热1.5s，复用 Gallery 的2s交替 top/bottom、.35s finger、.65s `exp(-8*t)` momentum、1s settle、相同 speed/extent 公式，经原 local adapter 送 AppKit。记录 native bounds.y 与原始输入，允许识别原生与 Rust 的实际物理轨迹差异，不强制把每帧结果改成同值。性能模式关闭编号网格、HUD和 Rust 校准模型推进。CPU 使用与 Gallery 相同的 macOS process CPU clock（user+system，100%=一逻辑核），记录均值及至少250ms间隔样本，不换算成每帧8.3ms预算。

到测量终点停止输入并 invalidate CAD link；仅安排一个5秒 one-shot timer，记录 idle CPU/残余显示回调后写报告、正常关窗。提前关闭、准入失败、可见性无效或 idle 不完整时 `completed=false`/exit2；缺图或 CPU 读数不补零。`completed`只表示本次记录完整，不自动判定CPU性能通过。输出不覆盖旧文件。机器/OS/显示器和程序 source/profile/hash 的实际绑定由主协调执行收据记录，不能用输入 Gallery 机器字段冒充 oracle 当次机器。

`frame_cpu_ms`、`frame_gpu_ms`、`actual_presentation` 明确为 null/unmeasured。CAD target/callback、NSLayoutManager/drawRect 和 one-shot timer 都不是实际呈现收据；全 WindowServer CPU/GPU 也不归给此窗口。即使正文布局与进程 CPU 对照成功，GPU/实际呈现/完整三页参照仍未验，原7gap失败与后续零 gap 约束不变。

受影响验证（由主协调集成后依原资源窗口执行）：release Gallery + `scroll_oracle` 同源构建；原 Gallery perf/ScrollArea 行为回归；oracle 默认校准路径短回归；缺文件/缺元数据/NaN秒数/旧输出路径的启动前 exit2；匹配或真实不匹配布局的可审计准入；一轮完整60s+idle正常退出/自有group空/输入hash不变。仅在准入、环境及指标覆盖明确后再排5轮 median/worst，当前候选没有执行 Cargo/GUI，也没有宣称这些验证通过。
