# Gallery 3D 自适应分辨率与持续旋转

2026-10-20，使用当前 `gallery-design-baseline/TekesUI` worktree，保留其他未提交工作。

Gallery 的 3D 预览按实际方形图像区域的逻辑边长 × backing scale 分配 color/depth target，四舍五入到像素。支持 1×、2× 和分数倍率。单边上限为设备限制与 4096 的较小值；4096² color + depth 的请求大小合计 128 MiB，不包含在途资源、材质或驱动开销。

布局记录目标尺寸，下一次 prepare 在 GPU 上 resize 并用 `replace_gpu_image` 替换 backing；ImageId、顶点、索引、morph 和变形输出 buffer 保持稳定。首帧仍有 512px 预览作为布局完成前的后备。暂停或 Reduced Motion 也会安排一次尺寸收敛帧，之后恢复空闲；普通旋转不重建 target。

沿用同一 wgpu device/queue：GPU 蒙皮、morph、索引渲染、法线与纹理合成。纯模型旋转只更新相机/model uniform，姿态上传和 compute dispatch 为零。动画关键帧采样及小型节点矩阵仍由 CPU 管理，顶点计算和像素合成留在 GPU；没有逐帧 CPU 顶点上传或像素读回。

## 原生持续测试

复用现有 Gallery `TEKES_PERF` harness，新增 `scene3d` workload，使用真实 `component-scene3d/playground` 路由。Release、内建 120 Hz 屏、1440×917 pt、2× backing；实际 3D target 为 **1218×1218**，不是整个窗口的 drawable 大小。

| 场景，每轮 60 秒 | 平均进程 CPU | 250 ms 样本峰值 | 实际呈现 FPS | CPU active p95 | 呈现间隔 >1.5 帧 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 整体旋转 | 9.92% | 25.22% | 119.88 | 0.487 ms | 7 |
| 整体旋转＋顶层动画 | 14.66% | 37.43% | 119.85 | 1.333 ms | 9 |

100% CPU 表示占满一个逻辑核。两轮均全程 key、未遮挡、呈现收据无缺失/重复，呈现间隔 p95 为 8.333 ms；保留偶发长间隔，未声称零掉帧。测量包含整个 Gallery 与呈现仪表开销，不与早先独立应用的短测直接比较。

两轮测量期间 target 重建次数、顶点 buffer 分配数和几何 resident 请求字节均不变。纯旋转 pose 上传/变形 dispatch 为零；带动画累计上传 2,612,160 字节紧凑姿态数据、处理 32,652 次网格变形，等价于仅在顶层 pose 变化时计算 9 个网格。停转后观察 5 秒，显示链接回调为零，CPU 约 0.06%。

原始数据与二进制/源码哈希见 [results.json](../../design/reviews/scene3d-adaptive-20261020/results.json)。首次失焦样本另存为 `rotation-interrupted-60s.json`，不计入上述前台成绩。本轮关闭 GPU counter readback；报告中的 CPU 和实际呈现覆盖整个工作负载，没有声称测得 3D pass GPU 执行时间。

```sh
# 仅整体旋转；使用 Release，测试窗口需持续处于前台
TEKES_PERF=scroll:60 TEKES_PERF_CONTENT=scene3d TEKES_PERF_GPU_TIMING=0 \
TEKES_METAL_DISPLAY_LINK=0 TEKESUI_WINDOW_PLACEMENT=builtin \
TEKES_PERF_OUT=/tmp/scene3d-rotation.json \
cargo run -p tekes-ui-gallery --release --locked
# 同时播放顶层动画：在相同命令前增加 TEKES_PERF_SCENE3D_ANIMATION=1
```

## 回归

- Gallery 6 项定向测试：分辨率取整/边界、1×/2×/分数倍率与缩放、暂停/Reduced Motion 收敛、同一 ImageId/几何复用、输入裁剪及路由。
- Scene3D 15 项测试，包含实际 GPU 蒙皮/morph、索引与法线、Reversed-Z 紧贴表面、动画及 resize。
- 现有性能框架 15 项回归及 Gallery Clippy 通过。
- 167 个范围外 Rust 源文件与执行前哈希一致。
