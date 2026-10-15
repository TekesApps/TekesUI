# tekes-ui-motion

TekesUI 的独立动画模型库，纯 Rust、零第三方依赖。可以在平台层、渲染和 components 之前开发与验证。

已实现：

- `MotionSpec`：Instant、有界三次 Bézier / Linear Tween、欠阻尼 / 临界阻尼 / 过阻尼 Spring。
- 纯函数时间采样，输出值和速度；支持 `f64` 与 `[f64; N]`。
- `Animated<V>`：目标与呈现分离、改目标承接弹簧速度、手势释放初速度、取消与有限动画截止时间。
- generation 事件：LogicalComplete、Settled（区分自然收敛与强制截止）、Cancelled。
- feedback、fade、geometry、selection、disclosure、surface、celebrate 调参起点。

实现背景与 Apple 官方来源见 [研究文档](../../docs/design/motion-foundation.md)。该库采用独立阻尼振子实现，不声称复刻 Apple 的内部实现或 `duration/bounce` 映射；预设是 TekesUI 参数。

## 使用

消费方在 Cargo.toml 引用本地库：

```toml
[dependencies]
tekes-ui-motion = { path = "../TekesUI/crates/tekes-ui-motion" }
```

path 按消费方位置调整。仓库内其他 crate 可使用 `../tekes-ui-motion`。

```rust
use std::time::Duration;
use tekes_ui_motion::{Animated, presets};

let mut position = Animated::new([0.0, 0.0])?;
position.animate_to([100.0, 30.0], presets::geometry(), Duration::ZERO)?;

// 每帧传入从同一 epoch 起算的单调时间，所有 channels 用同一个时间戳。
let now = Duration::from_millis(100);
let frame = position.advance(now)?;
// 把 frame.sample.value 写入 scene 的呈现属性。
// 只有 frame.needs_frame 且窗口可见，宿主才继续请求动画帧。

// 中途换目标：使用当前呈现位置和速度建立新弹簧。
let frame = position.animate_to([40.0, 10.0], presets::geometry(), now)?;
// frame.events 包含旧 generation 的取消通知。
# Ok::<(), tekes_ui_motion::MotionError>(())
```

`MotionSpec::sample(from, target, initial_velocity, elapsed)` 是可 seek 的纯采样：可用于预览、曲线绘制、测试，不应用运行时收敛或 deadline 截断。`Animated` 是带生命周期的单属性通道，要求操作时间不倒退；出错不改变通道。

## 集成边界

| 这个库负责 | 宿主 / 组件负责 |
|---|---|
| 数值采样、速度、重定向、收敛 | 时钟来源、帧调度、窗口遮挡、脏区 |
| 单属性通道、generation 与事件收据 | ComponentId 世代、属性唯一写入者、卸载通道 |
| 同坐标系数值与向量 | 布局、绘制、clip、transform 分解、颜色空间 |
| 同单位的释放初速度 | 手势识别、速度估算、目标吸附、边界碰撞 |
| Hold / Finish 取消策略 | Reduce Motion 读取、降级配方与偏好变更 |

组件为每个属性保存一个通道，无须引入全局 runtime 或组件注册表。组件 identity 不能用虚拟列表可复用 slot 代替。调用方若延迟派发事件，要携带组件 identity 并核对 generation，避免旧结果作用于新实例。

每次操作返回该次的事件，无隐藏回调或计时器。新目标前先在同一时刻采样旧动画：旧动画已经自然完成时可报告旧完成，否则报告取消；取消前已经到达的逻辑完成也可能出现在同次事件中。取消后不会再产生该 generation 的新完成事件。

`cancel(now, CancelReason::ReducedMotion, CancelBehavior::Finish)` 可让大幅运动立即落到目标，只报告该次取消；微弱淡化是否保留由组件策略决定。`Hold` 将当前呈现值设为新静止目标，适合手势接管；释放后调用 `animate_to_with_velocity`。

## 单位与数值

- 时间：`Duration`；速度：值单位/秒；频率：Hz；阻尼比无量纲。
- 默认位置/速度容差 `1e-4`、`1e-3` 适合归一化标量；几何应按 backing scale 配置（例如位置 `0.1 / scale` pt、速度 `0.5 / scale` pt/s）。同一向量各维必须使用可比较的单位与容差；不要把 pt 与 rad 塞入同一通道。
- 弹簧以解析式采样，近临界分支使用 `exp_m1`，过阻尼根避免大数相消。收敛使用递减机械能提供的未来位移/速度上界，不因振荡恰好过零就结束。
- 默认 10 秒 hard deadline 防止有限动画永远占用帧调度；`Settled(Deadline)` 应记录诊断。`logical_duration` 只是配方节奏，不是 Apple 的感知时长。
- 有界 Bézier 的控制点均在 `[0,1]`；有无限端点速度的退化曲线不能启动非静止 Tween，会返回 `NumericalOverflow`。内部产生无限斜率的退化曲线在构造时拒绝。超出 f64 可表示范围的运算也返回错误。
- Tween 改目标仅保证位置连续；Spring 保证位置与速度连续。标量没有属性语义，弹簧本身不做 opacity clamp、尺寸限制或碰撞，组件必须选择合适模型。

当前不含关键帧、循环、延时编排、共享几何、transition 节点保留、Liquid Glass、窗口/Gallery 与系统偏好桥接。这些可逐项建立在数值内核之上；库的测试通过不代表组件视觉或原生行为验收通过。

## 验证

在仓库根目录运行：

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p tekes-ui-motion --example sample_motion > motion.csv
```

示例输出 120Hz 的数值轨迹，在 0.15 秒从目标 100 改为 25，用于复现中断行为。无 GUI，也不要求 GPU。
