# City Block：CPU 优化后放开至 120 Hz，持续旋转 60 秒

日期：2026-10-21。继续当前 gallery-design-baseline/TekesUI worktree，HEAD f604eda。未修改实现源码，保留所有原有 WIP；本报告未提交。

## 结果

内置 Retina 屏，夜景、街区动态、自动旋转，R 默认视角无额外缩放；窗口 3024×1730、3D target 2960×1362、4× MSAA。关闭 H 60 FPS 限制，原生 display link 自动适配屏幕，host 请求最高 120 Hz。B 测试开关开启，采样前后实际截图可见街区在旋转和播放，HUD 分别约 118.3 / 114.5 FPS；最终结果以 Metal 原生 layer 呈现计数器为准。B 不绕过系统遮挡节流。

| 指标 | 本次 120 Hz 目标 | 上一轮固定 60 FPS（参照） |
| --- | ---: | ---: |
| 进程 CPU 平均 | 13.77% | 14.04% |
| CPU P95（每秒采样桶） | 18.73% | 15.82% |
| GPU Begin to End，帧加权平均 | 7.665 ms | 7.291 ms |
| 原生实际 FPS，时间加权平均 | **119.85** | 59.998 |
| 原生 FPS 计数窗口最小值 | 117.00 | 59.00 |
| GPU 每窗口忙碌度均值 | 89.60% | 41.68% |

当前默认视角已经接近 120 FPS，GPU 每帧耗时接近 8.33 ms 的 120 Hz 预算，余量较小。原生 GPU Begin to End 的计数窗口均值范围 6.810–8.487 ms；这不是逐帧最值或逐帧 P95。原生 FPS 窗口上限 121.21 是计数窗口/时间边界波动，不代表屏幕刷新率超过 120 Hz。不能据默认视角结果宣称更近视角也稳定 120 FPS。

CPU 与此前 60 FPS 结果相近，不能据此推断 120 FPS 更省 CPU，也不单独据此归因于某项优化：本次复用同一已运行、预热的进程，系统负载、缓存和调度条件不同。原生平均帧率和 GPU 忙碌度表明 GPU 确实承受了接近双倍的帧数，不能只看单帧 GPU 时间推断每秒 GPU 总负载。

CPU 100% 表示一个逻辑核心；GPU 忙碌度来自该原生窗口计数器，不是整机 GPU 利用率。CPU 共采样 60.005 秒，Metal 请求 60 秒，计数器覆盖 59.566 秒，含 60 个窗口。进程 unified footprint 结束约 519.27 MiB，不是独立显存读数。

## 复现与证据

CPU：沿用标准库 proc_pid_rusage 采样脚本，每秒采样，共 60 秒。GPU：xctrace 的 Metal Performance Overview，附加 PID 62295，请求 60 秒，与 CPU 同时进行。没有在采样期间构建、运行测试或 sample。

测试前核对现有 bundle binary 和 Scene3D / Viewer / 模型 SHA，与上一轮验收结果完全一致；无需重新构建。测试后再次核对源码 SHA 一致。对比数据来自 city-block-cpu-light-dispatch-20261021.json 的 shared-pass 阶段，是历史参照，非本轮重新采样的成对对照。

机器可读报告：city-block-postcpu-120hz-20261021.json。
验收目录：target/verification-receipts/city-block-postcpu-120hz-20261021，包含 CPU JSON、Metal 安全 XML/JSON 导出、前后截图、录制日志、源码及可执行文件 SHA、清理收据。复用先前保留的验收 executable，无重复复制。未导出含环境信息的完整 trace TOC。

## 清理与结束状态

本轮无构建、未创建新 target/cache。确认导出完成并用 lsof 检查无文件占用后，仅删除本轮原始 spin120.trace，释放 **16.86 MiB**（文件实际分配块）。保留截图、日志、XML/JSON、已有可执行文件、源码、依赖锁和其他任务产物。未清理用途不明的缓存。

结束后停止自动旋转、关闭 B 测试开关；保留 H 关闭，即继续按屏幕自动适配最高 120 Hz。街区动态继续播放。
