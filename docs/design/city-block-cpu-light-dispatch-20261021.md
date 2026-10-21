# City Block CPU / GPU 命令准备优化与固定 60 FPS 复测

日期：2026-10-21。当前 worktree / 分支保留所有原有 WIP；本报告随本轮优化提交。

## 结论

同为内置 Retina 屏、夜景、街区动态、持续旋转、固定 60 FPS，各采样 60 秒：CPU 均值从 **16.39% 降到 14.04%（下降 14.4%）**，GPU Begin to End 从 **7.577 ms 降到 7.291 ms（下降 3.8%）**。原生呈现帧率分别为 **60.000 / 59.998 FPS**。没有以降帧、减少灯光/窗户/人物效果换取结果。

这是本轮当前 WIP 的前后对比。此前报告的 21.82% CPU 属于另一组历史测量，不能拿来代替本轮新构建的 16.39% 基线，也不能把两组差异都归因于本轮优化。

## 定位与改动

5 秒原生进程 sample 的活跃调用栈主要落在 wgpu 的 CommandEncoder::finish、render bundle 重放、compute pass 命令编码，以及 Queue::write_buffer 的 staging 分配。没有观察到逐窗口、逐灯光的 CPU 动画模拟热点。sample 是墙钟栈采样，等待栈的数量不能直接当 CPU 百分比，也不足以因果解释历史全部涨幅；它用于选择可消除的工作。

1. Scene3D 灯光准备原来有四个计算管线/dispatch：附件运动、阴影候选、网格边界、网格掩码。现在在一个 64 线程 workgroup 内完成，以 storageBarrier 保持依赖。64 灯、96 遮挡代理上限、32×32 网格、原始顺序/法线/阴影公式均保留；不可把该入口改成多个 workgroup。新增显存为零，原有空间筛选继续留在 GPU。
2. Scene3D 的变形、LOD 剔除、共享窗户状态、灯光准备统一编码进一个 compute pass。wgpu 处理 dispatch 间 storage hazard，随后 render pass 消费结果；减少反复建立/结束后端计算编码器。蒙皮和 morph 仍由 GPU 计算，动态实例和几何不逐帧上传。
3. 新增 set_environment_and_clocks：验证全部参数后原子应用，一次 80 B uniform 写入代替两次 16 B 写入，保留中间静态灯光参数。每帧多复制 48 B（60 FPS 约 2.8 KiB/s），换掉一次 staging 分配和队列操作；原有单独 setter 保持可用。相同值跳过，非法输入不部分修改状态。
4. Viewer 绘制直接借用缓存 UI Scene，去掉每帧深拷贝；FPS 文本仍每半秒更新。变化主要在通用 Scene3D，Viewer 只接入组合上传和缓存借用。

## 完整测量过程

所有阶段使用同一模型、相同窗口尺寸、4× MSAA、R 默认视角不额外缩放；N 夜景、A 自动旋转、街区播放、H 60 FPS、B 后台测试开启，采样窗口前台无遮挡。B 不绕过系统遮挡节流。每阶段重新启动 release App。CPU 与 Metal Overview 同时采样，期间没有编译、测试或 sample 干扰。

| 阶段 | CPU 平均 | CPU P95（1 秒桶） | GPU 平均 ms | 原生 FPS |
| --- | ---: | ---: | ---: | ---: |
| baseline：本轮当前 WIP、原四 dispatch | 16.391% | 18.726% | 7.577 | 60.000 |
| optimized：仅灯光 dispatch 合并 | 16.375% | 18.064% | 5.720 | 60.001 |
| final：再合并 uniform 上传、借用 UI 缓存 | 15.955% | 19.537% | 7.368 | 59.999 |
| shared-pass：最终，再共用 compute pass | **14.037%** | **15.822%** | **7.291** | **59.998** |

中间文件名 optimized / final 是实验阶段名，最终验收版本是 shared-pass。第一步 CPU 基本持平，第二步 2.7% 的下降可能受采样波动影响，因此继续优化计算 pass。中间 GPU 时间波动较明显，不把 5.720 ms 当成最终收益承诺。

最终 CPU P95 比基线下降约 15.5%；Metal 每窗口 GPU Usage 计数器均值从 44.16% 到 41.68%。该计数器不是整机 GPU 利用率。最终进程 unified footprint 结束值约 513.74 MiB（基线 522.88 MiB），不是独立显存读数，不作为显存优化证明。

窗口采样前后截图均为 3024×1730，实际 3D target 2960×1362；CPU 100% 表示一个逻辑核心。GPU 时间按帧加权，FPS 按原生 layer 计数器时间加权；GPU 窗口均值不是逐帧 P95，也不能用其倒数表示 FPS。每阶段仅一次桌面采样，转动相位、系统/温度负载有差异，结果是本机观察值，不是隔离环境下的统计因果证明。最终原生 FPS 的 60 个窗口范围为 58.999–61.003。

## 验证

- cargo +1.95.0 test -p tekes-ui-scene3d -- --include-ignored：22 单元/GPU + 4 组件测试通过。
- cargo +1.95.0 test -p tekes-ui-3d -- --include-ignored：18 测试通过，覆盖城市 GPU 动态、夜间停工、竣工、共享窗户/灯光、近远 LOD、透明材质、共享/独立渲染一致性等。
- 灯光 GPU 回归保留原四 dispatch/full scan 参考；测试 64 灯、96 遮挡体和 1/31/32/33/63/0 边界、运动与淡出；图像逐通道差 <= 1/255。组合 uniform 与旧分开上传使用非默认灯光比较，并验证重复值跳过、NaN 输入不部分更新。
- 相关两包 strict Clippy（--all-targets --no-deps -- -D warnings）、release 构建、Scene3D diff whitespace 检查通过。
- 首次含依赖的 strict Clippy 被已有 macOS shared.rs 的 collapsible_if 警告阻挡，未改该无关 WIP；不宣称全 workspace lint 通过。
- 对比基线保存的 host 源码 SHA：render 和 macOS platform 未变化，Viewer 仅有本轮有意改动。全部其他未提交改动保留。提交中的 Viewer 以 HEAD 既有接口为基础接入缓存借用及组合上传，沿用原先全零的独立时钟；当前 WIP 中已有的昼夜和独立时钟接线继续保留，未夹带进本次提交。性能验收对应当前完整 WIP，不能据此声称干净检出该提交就具有所有未提交的街区效果。

## 证据与清理

机器可读结果：city-block-cpu-light-dispatch-20261021.json。
本地验收目录：target/verification-receipts/city-block-cpu-light-dispatch-20261021，含原始 CPU JSON、Metal 安全导出 XML/JSON、各阶段截图、构建/测试日志、sample 文本、源文件与 binary SHA、基线及验收 executable、Cargo.lock 快照和 cleanup.json。未保存含环境信息的完整 trace TOC。

复用现有 target；无新增独立 Cargo target。确认 trace 已导出、无进程占用，检查 Cargo 独占锁和增量文件占用后，删除本轮四份原始 Instruments trace 及本轮验证的两个 3D crate 的可再生 debug incremental 缓存。源码、WIP、锁文件、可执行文件、验收资料和其他 crate 缓存均保留。具体释放空间见 cleanup.json；按已删除文件实际分配块统计，不用整盘可用空间变化推算。

完成后停止自动旋转、关闭 B 和 H 测试开关，恢复屏幕刷新率自动适配，街区继续播放。

本次共释放 **370.66 MiB**；占用检查未发现需要保留的本轮候选缓存。
