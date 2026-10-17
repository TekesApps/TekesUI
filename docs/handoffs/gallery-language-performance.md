# Gallery language and rendering follow-up — 2026-10-16

最新执行状态见本文末的“当前工作台性能复测”。此前的预览路径、并行任务分工与测量租约是历史记录。

## Scope

English by default, an in-place English / Simplified Chinese menu, one constant-speed rotation animation, and rendering performance work. No additional component families; pure Rust / WGSL and the approved JSON Design System remain the implementation boundary.

## Verified before the native performance matrix

- Gallery release tests: 157 passed, 0 failed, 1 pre-existing ignored reference-dependent test.
- Release all-targets Clippy: Gallery, renderer and macOS platform passed with `-D warnings`; macOS platform unit tests: 19 passed.
- Design System archive/schema/refs/generated projection and pure Rust boundary checks passed.
- Historical Primitives / Tokens / StaticText screenshot comparisons passed without replacing original PNGs (existing analytic hairline-AA correction remains the only allowed original pixel change).
- Real application: default English, Language menu live switch both ways, English TextEditor aligned to remaining viewport, and Symbols / Linear rotation running and pausing inspected through CUA.
- The compact English Target/Retarget button copy was shortened after a real-window crop was found. A measured text-width regression now checks both languages against the existing slots.
- Locale switching preserves edited documents and active IME. Typing fixtures use the selected language's complete graphemes and original UTF-16 ranges; cached document paragraphs bypass interface translation.

## Performance acceptance

Use actual Metal `presentedTime` receipts, CPU user+system rates (100% = one logical core), frame build/acquire/encode/submit timings, and raster/upload counters. A 120Hz request, an 8.33ms average or a good p99 alone does not satisfy the zero-gap requirement. Results and unresolved limitations are added after sequential native runs; no success is inferred from compilation or model tests.

`TEKES_PERF_FRAME_LATENCY=1|2` is a benchmark-only FIFO queue-depth diagnostic: the selected wgpu Metal backend maps it to two or three drawables. Normal application behavior retains latency 2 / three drawables. Reports record the selected queue; display synchronization stays enabled. Do not use simultaneous GPU tests, compilation, screenshots or a second active preview during measurements.

## Display-wait diagnostic

`scripts/analyze-present-waits.py` joins frame costs and public presentation receipts by target timestamp, reports acquire-to-present latency separately, and keeps the surrounding callback/acquire/present timeline for each long interval. The fixed-texture historical `diagnostic-static-24` group has an acquire peak of 0.132ms, but mean acquired-to-present latency of 30.190ms and two actual presentation gaps. This rules out blocking in `nextDrawable` as the sole explanation for that group. It does not identify a private system cause, and it does not justify attributing a delayed presentation to the callback with the same array index; queueing shifts the timeline.

## Optional Metal display-link experiment

`TEKES_METAL_DISPLAY_LINK=1` selects the macOS 14+ public `CAMetalDisplayLink` for an explicit run. The default remains the existing view display link. No Swift, private API, method swizzling, busy wait, or disabled VSync is used. A retained drawable supplied by the update is passed through a main-thread-local scope to the owned CAMetalLayer subclass; the ordinary wgpu Metal HAL consumes it once. A consumed scope returns `None` on a retry instead of performing a second acquisition. The scope releases any unused drawable on exit. Reentrant event dispatch retains the layer/drawable in the internal main-thread event queue and installs the scope only when that handler runs. Paused/closed queued Metal items are released; queue delay and remaining CPU submission budget are measured at dequeue.

The animation target is Apple's `targetPresentationTimestamp`; `targetTimestamp` is separately retained as the CPU submission deadline and used for budget checks. The nominal 120Hz gate period is requested from the display-link range; it must not be described as measured refresh evidence without actual target/presentation records. See [Apple's update API](https://developer.apple.com/documentation/quartzcore/cametaldisplaylinkupdate) and [delegate contract](https://developer.apple.com/documentation/quartzcore/cametaldisplaylinkdelegate).

Independent Sol high read-only review found and prompted fixes for deadline/display-target separation, reentrant scope lifetime, and stale drawable receipt reuse. Native resize, occlusion recovery and cadence are separate acceptance requirements; static review alone does not authorize promotion to the default driver. Initial creation is deferred beyond startup events. Stopping invalidates the experimental link to release its prefetched drawables; resuming recreates it. The callback retains its link through any in-callback invalidation. Real-window rotation pause/resume passed after this change, but active live resize subsequently stalled event/render updates. This is an explicit failed experimental case; the normal Gallery stays on CADisplayLink.

The measurement harness now joins only rendered nonzero unique drawable serials. It separately reports skipped render callbacks, duplicate serials, missing receipts, occlusion/key-window changes, reentrant Metal updates and acquisition counters during idle. Older reports remain immutable and retain their documented stale-serial limitation.


## Corrected quiet-window preflight outcome

The frozen CAD / three-drawable five-case matrix was not started. Its preflight waited up to 600 seconds but never obtained the required consecutive 15 seconds of low background CPU; exit code 3, gate_passed=false, 197 preflight samples and zero during/native samples. This is an **unmeasured** result, neither a performance pass nor a measured workload failure. Sanitized receipt: `design/reviews/display-waits/quiet-preflight.json`; the full ambient process record remains local under `target/symbol-perf-corrected/`. The animation task is awaiting the user's background-cleanup status before retrying. No threshold was relaxed and no unrelated process was stopped.

## Quiet-window retry after background cleanup

The user confirmed that background cleanup ended. The animation task owns the
Metal resize lifecycle repair and its real-window regression before freezing a
new candidate and running its sequential five-case Symbols matrix. The main task
keeps build/GPU/capture work stopped during that measurement, then runs
`scripts/measure-scroll-pages-quiet.py` for ScrollArea, TextEditor and Icons using
the same frozen binary. This runner reuses the existing CPU-delta gate (15s
stable, per-process 30%, total external 100%) and records exact before/after
executable hashes; it does not stop unrelated processes.

The legacy `f1-scroll-summary.py` exit status now also requires zero actual
presentation gaps, complete unique receipts covering the measurement duration,
zero skipped callbacks, a visible/key window, callback CPU peaks within the
nominal period, zero missed refreshes and idle callback silence. Legacy reports
missing integrity metadata cannot pass. GPU-disabled runs are explicitly
unmeasured for GPU costs. Four adversarial summary regression tests passed,
including a single displayed-frame gap despite good CPU/p99 diagnostics. New
formal native results remain pending; these script checks are not performance
acceptance.

## Completed scrolling diagnostics and boundary repair

See `design/reviews/scroll-pages-retest/README.md` and `summary.json` for all six
60s CAD/Metal actual-page scrolling reports, source snapshots and raw hashes.
These are shared-desktop diagnostics; neither failed preflight was overwritten.
All six retain actual gaps, so long-running 120Hz remains unaccepted.

The final frozen boundary candidate is recorded in
`design/reviews/scroll-pages-retest/build-measured-drain-final.json` (signed SHA
`0d19db7609ef9ad29e4ab9aafc0a30a1d5901d28d52a85438c234e17306be3c4`). It drains
all measured presentation receipts before link invalidation, has an independent
main-queue timeout wake, and requires a complete five-second idle observation.
CAD/Metal positive native boundary regressions and a real early-close negative
regression passed. Gallery 162 pass / 1 existing ignored, platform 19 pass,
strict release Clippy and 8 Python acceptance regressions passed. Only Perf,
its Host branch and selftest support changed; approved visuals/JSON are unchanged.
The animation task next owns the launch+30s main-thread diagnostic and any quiet
matrix; do not run competing native/GPU/build/capture work during that lease.


The user replied that manual Codex minimization is currently inconvenient. Do not
repeat this request or keep launching quiet preflights in the unchanged desktop.
Formal acceptance remains unmeasured; the ~30s gap remains unresolved. The latest
normal Gallery is running from the final compiled drain candidate (default CAD,
English, ScrollArea), with native AX/screenshot verified; launch provenance is
`design/reviews/scroll-pages-retest/gallery-launch.json`. Source WIP is preserved,
and no new component or appearance change was introduced.

## 当前工作台性能复测（2026-10-16）

Design System、外观、公开控件 API 和默认 CAD/FIFO latency 2 均保留，不新增组件。代码修复：导航按查询变化缓存 section/item 数据；逐帧只生成视口内的标题、条目和文字图元。原来每帧克隆整个导航并生成屏幕外字形的工作已移除。四种主题 × 两个倍率 × 三个含分数位的滚动位置，共 24 组离屏 GPU 输出逐字节一致；测试还要求短视口下字形数至少减半。旧三页 golden 对照保留并通过。

[机器回执与完整每帧报告](../../design/reviews/performance-followup-20261016/summary.json) 保留两个签名二进制 SHA、源码快照、十次各 60 秒原生运行、全部失败原因和外部 CPU 摘要。完整外部进程记录与失败的系统 trace 仅保留在 target 下。每份 gzip 原始报告都由 manifest 校验压缩和原文 SHA256；不覆盖历史失败。

同一旧校准 ScrollArea 负载的顺序观察：字形中位数 3804 → 3157（减少约 17%），建场景均值 0.165 → 0.123ms，进程 CPU 9.18% → 7.83%。这是共享桌面下的观测，不能将不同时间的外部负载当作严格隔离的 A/B，也不能据此把掉帧判为修复。

性能工具新增可选 `--workbench`，实际打开当前注册的 ScrollArea、内部 TextEditor 和 Icon 页面，并在 JSON 写出 `config.gallery_route`，避免旧校准页替代当前工作台验收。`--content` 选择串行负载；`--frame-latency` 与 `--gpu-timing` 是明确标记的诊断选项，不改变普通应用默认设置。

| 运行 | CPU（100%=一逻辑核） | 实际长间隔 | presentedTime=0 |
|---|---:|---:|---:|
| current-cad-scroll-area | 9.18% | 0 | 0 |
| optimized-cad-scroll-area | 7.83% | 2 | 0 |
| workbench-cad-gpu-scroll-area | 12.51% | 47 | 0 |
| workbench-cad-latency1-scroll-area | 10.47% | 6 | 0 |
| workbench-cad-scroll-area | 14.89% | 3 | 0 |
| workbench-cad-editor | 15.77% | 3 | 0 |
| workbench-cad-icons | 18.05% | 1 | 0 |
| workbench-metal-scroll-area | 11.82% | 3 | 3 |
| workbench-metal-editor | 18.66% | 4 | 3 |
| workbench-metal-icons | 14.57% | 5 | 5 |

默认 CAD 当前工作台三个页面均未通过零掉帧；Metal 虽将 acquire 峰值降低至约 0.05ms，仍存在未呈现/跳过回执，不能提升为默认。降低 FIFO 队列深度的单独诊断也未通过。带 GPU 时间戳的额外 ScrollArea 运行，render pass 平均约 1.38ms、最大约 2.97ms，但实际长间隔变多，按带观测开销的诊断单列。没有把关闭查询时的 GPU 成本记为零。

六个当前工作台的建场景 + encode + submit 墙钟耗时峰值均小于 3ms；默认 CAD 的 acquire 峰值约 15.8–16.3ms。Metal 的取图等待很短仍会出现实际长间隔，说明 nextDrawable 阻塞不是全部问题的唯一解释。新的时间线分析将这两类数据与未呈现目标分开；不把时间相关性当作 GPU 队列或合成器的因果证明。不同运行的长间隔位置并不固定于启动 +30 秒。

系统 `Metal System Trace` 尝试产生了本地原始数据，但采集 exit 1，导出失败为 `Document Missing Template Error`；这不是有效 trace，未用其宣称定位根因。未修改系统安全配置，也未停止其他项目进程。

所有 60 秒报告均记录完整五秒 idle，停止后显示回调/取图计数为 0，优化候选的 idle CPU 约 0.016%–0.050%（一逻辑核口径）。外部 CPU 仍未满足已有安静门槛，严格长期 120Hz/8.33ms 验收保持 **未通过**。下一步需要有效的 GPU 队列/系统呈现跟踪及可隔离的测试窗口，不能靠放宽阈值、丢弃帧或切换到另一套失败驱动来过关。

验证：Rust 284 项通过、1 项既有忽略；另有 14 项 Python 验收回归通过；release build、Clippy `-D warnings`、设计源/投影与纯 Rust 检查通过。冻结后 Rust 和 design/source、design/brand JSON hash 均未变化。没有提交、推送或发布。


## GPUI / Metal 呈现参考对照（2026-10-16）

本轮只读对照官方资料与实现，未改变默认驱动、Design System 或外观，也未重新运行性能验收。

- [Zed 2024-02-07 工程文章](https://zed.dev/blog/120fps) 必须连同文末 addendum 阅读：早先的 `waitUntilCompleted → waitUntilScheduled` 并非最终方案。最终普通绘制使用 `presentDrawable → commit`，关闭 `presentsWithTransaction`；启动/调整窗口等需要与 Core Animation 同步的路径例外。文章记录过计算不足 4ms 仍出现 8/16ms 呈现交替，不能据此认定今天的本项目与其同一根因。
- 当前源码固定到 Zed `28d0a3547f16b9609b301b48ab363bfca649c864`：[renderer](https://github.com/zed-industries/zed/blob/28d0a3547f16b9609b301b48ab363bfca649c864/crates/gpui_apple/src/metal_renderer.rs#L495)、[display link](https://github.com/zed-industries/zed/blob/28d0a3547f16b9609b301b48ab363bfca649c864/crates/gpui_macos/src/display_link.rs)。renderer 保留三 drawable 和上述双呈现路径；macOS pacing 使用每 display 的 CVDisplayLink，通过 main-queue dispatch source 通知窗口，并专门处理 link 生命周期。无需为了参考它引入 GPUI。
- 本项目 `view.rs` 仅 live resize 打开 transaction；本地实际依赖 `wgpu-hal 30.0.1/src/metal/mod.rs:802` 的普通 present 也是 `presentDrawable → commit`，transaction 路径才 `waitUntilScheduled → drawable.present`。未发现普通滚动错误使用全程同步等待这个旧问题。区别是 wgpu 在渲染提交后另建一个 present command buffer，而 GPUI 将 present 编进渲染 command buffer；这是待测差异，不能由源码差异直接宣布为缺陷。
- [Apple nextDrawable](https://developer.apple.com/documentation/quartzcore/cametallayer/nextdrawable()) 明确描述 drawable 池耗尽会等待。[Metal display link 示例](https://developer.apple.com/documentation/metal/achieving-smooth-frame-rates-with-a-metal-display-link) 提供另一种 pacing 路线；我们的实验路径已经使用它，但实际未呈现回执尚未消除，所以不能仅因 API 更新就提升为默认。
- [wgpu #8309](https://github.com/gfx-rs/wgpu/issues/8309) 的遮挡/切回一秒等待已有修复，本地 HAL 包含 occlusion 检查。当前可见/key 窗口的孤立帧间隔不等于此问题。[Zed #53390](https://github.com/zed-industries/zed/issues/53390) 是长时间挂起报告，维护者按重复问题关闭；报告者的根因推断不作为我们的已证实结论。

后续排查顺序：先取得有效 GPU 队列跟踪，关联 CPU submit、GPU scheduled/completed 与实际 presentedTime，定位等待发生在提交前、队列中或合成/扫描阶段；再用同一冻结场景对照窗口/全屏呈现模式和驱动的回调相位。只有证据指向 wgpu 的提交边界后，才做隔离的同 command-buffer present 原型，不先替换生产渲染器。连续输入与从 idle 恢复分开检查 ProMotion，避免用永久重绘抬高 CPU 换表面平滑。每种实验同时报告实际呈现间隔、CPU 和 idle，保持零掉帧验收阈值。


## 呈现队列有效跟踪（2026-10-16）

新增 `scripts/trace-gallery-presentation.py`：用冻结的签名二进制启动真实工作台 ScrollArea，再 attach Metal System Trace；采集结束与应用退出分离，记录 TOC/逐表导出状态与源 SHA。启动环境采用 allowlist，导出的 TOC 删除 environment 值。原始 trace 仅留在 ignored target，不加入 Git。采集自身的开销和共享桌面状态使其只能用于诊断，不能算安静验收。

5 秒 idle attach smoke 的保存与 TOC 导出成功。首次 40 秒连续滚动 attach 在收尾超过脚本原有 120 秒保存余量，被 timeout 中断，失败回执保留；已将余量增加到 600 秒。随后 10 秒目标采集完整保存，record、TOC 和八张事件表导出均 exit 0，核心队列/GPU 表非空。对应应用运行的是 35 秒滚动 workload。没有用失败的长 trace 宣称定位完成。

[有效跟踪分析](../../design/reviews/presentation-chain-20261016/analysis.json)、[采集回执](../../design/reviews/presentation-chain-20261016/capture-receipt.json) 和 `target-tables.json.gz` 保存到同一证据目录。表投影仅保留本应用 PID 的行，完成事件按 command-buffer ID 选择；完整源 XML 的 hash 仍在回执。分析器验证输入校验和，拒绝未完成采集，按 PID、精确 command-buffer ID 关联，未知/边界缺失不补零，也不把整个显示器的 surface 归给 Gallery。

| 有效 trace 中的量 | 结果 |
|---|---:|
| 关联到 GPU 的渲染 command buffer | 1300；另有 1 个边界 buffer 未匹配 |
| 渲染 GPU 执行跨度 | 均值 1.3115ms，最大 3.1368ms |
| CPU commit 结束 → GPU 开始 | 均值 0.5556ms，最大 6.0567ms |
| CPU commit 结束 → completed 事件 | 均值 2.0573ms，最大 8.5160ms |
| 本应用 drawable 等待 | 1301 次，最大 32.2452ms；11 次超过 1000/120ms |

最长 32.2452ms 等待期间，本应用只有一段 1.3740ms 的渲染 GPU span 与之重叠；另一段 25.5408ms 等待期间未见相交的本应用 render span。这里只说明长等待没有被等长的本应用 GPU 渲染工作填满；**不能**证明 GPU 全局空闲，不能排除其他 GPU 工作，更不能据此判定合成器/Metal 有 bug。GPU 调度峰值也已接近一个刷新周期，必须继续与呈现和回调时机一起检查。不同钟域的 harness media time 与 trace-relative time 尚未逐帧对齐，不能用相邻时刻冒充因果链。

同次 35 秒 harness 记录 CPU 11.62%（一逻辑核口径）、21 个实际长呈现间隔，收据完整、无零时间戳、无隐藏/失焦回调；这是带 profiler 的诊断结果，不与先前未采集运行作严格 A/B。完整 5 秒 idle 的回调/取图为 0，CPU 0.0251%。**120Hz/8.33ms 零掉帧仍未通过，生产渲染行为未修改。**

复现投影分析（不需要重新启动应用或加载原始 trace）：

```sh
python3 scripts/analyze-metal-trace.py design/reviews/presentation-chain-20261016 --out target/presentation-chain-analysis.json
```

已验证输出与保存的 analysis.json 逐字节一致。新增采集/分析器共 9 项测试通过，另有 14 项已有性能门禁回归通过。本轮未改 Rust/WGSL/DS，未重跑 Rust 构建；不将此前构建结果当作本轮新检查。普通 Gallery 已重启并通过原生 AX 确认 ScrollArea / Light 窗口；仍为原签名二进制、默认 CAD、英语。没有提交、推送或发布。


## 低复杂度 CPU 修复与提交余量实验（2026-10-16）

按用户最新意见保留线程模型，不引入线程池、多线程渲染或新组件。Metal 的 GPU 并行和 CPU 多线程编码是不同机制；[Apple 并行编码说明](https://developer.apple.com/library/archive/documentation/Miscellaneous/Conceptual/MetalProgrammingGuide/Render-Ctx/Render-Ctx.html) 不能当成增加线程一定能解决 drawable 等待的证据。本轮最终只保留有明确重复工作来源、无需同步架构改造的改动：

- `tekes-ui-render/src/common.rs`：viewport/scale uniform 仅变化时上传；实例缓冲保留已排入队列的数据，只写发生变化的四字节对齐区间。扩容时清除缓存并完整上传，缩短/增长/清空均保留正确绘制范围。没有新增锁、后台线程、公开组件 API 或修改 token。
- `gallery/pages/icons.rs`：空查询直接匹配，不逐名称分配小写字符串；计算页面 extent 时直接计数，不再构建并排序完整临时结果。这由实际 CPU 采样发现的重复筛选触发，大小写、中文和 literal substring 规则保留。
- `analyze-present-waits.py`：分开报告 handler 墙钟耗时与已报告的提交预算。这个差值不是原生 GPU scheduled 时间，也不能单独证明成功赶上显示 deadline。

CPU 采样使用独立的 Time Profiler attach，record/TOC/time-profile 导出成功，原始 XML 与回执压缩保留在证据目录。只统计目标 PID 的 Running 样本；sample weight 是统计权重，不等于精确逐函数 CPU 耗时。样本明确出现 Icons 的 filtered/extent、重复 to_lowercase、GlyphAtlas 查找和 wgpu write_buffer；没有依据据此把整个场景搬到后台线程。原 Metal trace 的 3.14ms GPU span / 32.25ms drawable wait 仍是另一份运行的证据，不混记为本轮无查询运行的 GPU 成本。

试验 CAMetalDisplayLink preferredFrameLatency 1/2（[Apple 只允许这两个值](https://developer.apple.com/documentation/quartzcore/cametaldisplaylink/preferredframelatency)），各 60 秒，两者均失败。2 帧有 11 个零 presentedTime，相关 handler 的已测工作量远低于报告的约 8ms 预算；更多 CPU 提交余量并没有消除漏呈现。没有将它升级为默认，实验开关、平台 helper 和额外测试已从最终代码撤回，实验源码、运行脚本和报告独立归档。正常路径仍为 CAD / FIFO / desired latency 2，未更换 GPU 框架。

[九次各 60 秒运行汇总](../../design/reviews/presentation-repair-20261016/summary.json) 保存全部成功/失败项、外部 CPU 审计、两个中间候选和最终二进制 hash、选定源码快照、267 项最终 Rust 检查（另 1 项既有忽略）与 25 项 Python 检查。所有运行都是共享桌面诊断，外部 CPU 没有全程通过低负载门槛。没有重复要求用户最小化窗口，没有停止其他项目进程。

| 运行 | CPU（100%=一逻辑核） | 实际长间隔 | 零 presentedTime |
|---|---:|---:|---:|
| Metal 2 帧余量 | 14.17% | 12 | 11 |
| Metal 1 帧余量 | 12.56% | 7 | 6 |
| CAD ScrollArea 对照 | 9.04% | 2 | 0 |
| CAD 上传缓存 ScrollArea | 9.49% | 5 | 0 |
| CAD 上传缓存 TextEditor | 12.34% | 1 | 0 |
| CAD 上传缓存 Icons | 11.22% | 1 | 0 |
| CAD 上传缓存 ScrollArea 重复 | 13.73% | 5 | 0 |
| CAD 原版 ScrollArea 重复 | 17.58% | 1 | 0 |
| 最终 Icons（含空查询快速路径） | 8.93% | 3 | 0 |

ScrollArea 两次顺序对照的 encode 均值分别为 0.100 → 0.067ms、0.206 → 0.099ms；进程 CPU 在运行时段间大幅漂移，不能只挑较低的一次数值宣称因果。Icons 空查询改动前后 build 均值 0.162 → 0.113ms，进程 CPU 观测 11.22% → 8.93%，同样受共享环境限制。最终默认路径回执没有缺失/重复或零时间戳，但仍有实际长间隔，**120Hz / 1000÷120≈8.33ms 零掉帧验收仍未通过**。最终 Icons 完整 5 秒 idle CPU 0.0160%、显示回调与取图均为 0。

验证与保留的失败：

- GPU 缓存测试验证实际排队字节数、缓冲扩容重新上传；像素测试覆盖中间内容变化、缩短再增长、清空、视口和缩放变化。
- 原版与最终版全部 672 张工作台 PNG 逐字节一致，且未更改任何 golden。现行合并 DS、原始 134 文件包与投影校验通过；旧 `check-design-baseline.py` 仍因历史 JSON authority bytes/semantics mismatch 拒绝现行 JSON，保留为未通过，不修改其审批 pin。当前 Python 缺 jsonschema，DS 校验使用已有 `target/design-check-venv` 后通过。
- 默认 target 的旧 Cargo dep-info 引用了 `scroll-repair/frozen-source`，导致构建错误使用不含当前动画/图标 API 的库。未改这些库 API 或清理共享缓存；改用独立 target 完整重建，release build、Clippy `-D warnings` 通过。最后重新构建、用相同文件名和签名参数校验，二进制与最终实测 hash 一致；第一次用不同文件名签名产生的无效对照也记录在回执。
- 最终被测源码冻结后无变化；与本轮开始前相比，Rust/WGSL 内容变化仅为 common.rs、icons.rs 与 offscreen.rs，Design System / 动画 / 平台层源码均保留。
- Gallery 用最终冻结二进制打包、签名校验后重启；原生 AX 确认 `TekesUI Gallery — ScrollArea — Light`，默认 CAD、英语。没有提交、推送或发布。

剩余重点仍是将具体长呈现间隔与 drawable 释放、显示合成及回调时机对齐。当前证据既不能证明单纯多核可以修复，也不能将根因归给 Metal 或外部应用。

### 后续优先级（2026-10-16）

用户同意在不严重影响使用的前提下先记录性能问题，继续完成已注册组件。当前孤立长呈现间隔、drawable 回收关联和受共享桌面影响的 CPU 对照仍是开放问题；120Hz / 8.33ms 门槛没有通过，也不因暂缓排查而豁免。保留已验证的上传缓存和 Icons 空查询优化、现有默认呈现路径及全部原始回执，不引入未经测量证明必要的线程池或并行渲染。

组件开发沿用现有 JSON Design System 和纯 Rust 架构。组件完成度以真实模型、交互和 Story 验证为准，进度统一维护在 [Gallery 工作台文档](../gallery.md)；组件扩展后需重新验证构建、交互及渲染，不能借用本节此前冻结二进制的性能结果。

### 屏幕分工与预检（2026-10-17）

用户确认：120Hz / 8.33ms 专项固定在支持 120Hz 的内建屏，普通布局、视觉和交互复测使用外接屏。现有性能 runner 已显式设置 `TEKESUI_WINDOW_PLACEMENT=builtin`，不因外接屏连接而改目标。系统清单确认内建 Color LCD 与外接 U2790B（30Hz）同时连接；清单本身不证明实际呈现达到 120Hz。

使用最新已验证 release 二进制，对当前 ScrollArea Story 做有界低负载预检：30 秒内未满足连续 15 秒安静门槛，exit 3、gate_passed=false，正式原生测量未启动。没有停止其他项目进程，也没有放宽阈值；本轮没有新的 Gallery CPU 负载或 120Hz 呈现结论。[脱敏预检回执](../../design/reviews/builtin-120hz-retest-20261017/preflight.json)。
