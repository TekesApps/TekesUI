# 现有组件的动画接入审计与状态过渡接口

2026-10-16。只审计当前 Rust 实现，新增独立数值适配层；不新增组件或展示，不修改 DS token、组件外观、Gallery、Text 或 ScrollArea 接线。

## 2026-10-16 的审计基线（历史）

|位置|已存在的能力|尚未完成的接入|
|---|---|---|
|`tekes-ui-motion::Animated`|标量/向量；Spring 改目标承接位置和速度；Tween 承接位置；Hold/Finish 取消；generation 事件|没有统一的外部暂停时钟或动态 Reduced Motion 适配层|
|`gallery/motion_preview.rs`|直接消费 Animated、Recipes、Symbols、Typing；预览自己维护播放时间与系统/手动偏好|这是 Gallery 预览接入，不能当作生产控件已经接入的证明|
|`controls/widgets.rs` Button、Checkbox、Tabs、Menu、Dialog、Select|逻辑状态、键盘和指针行为；paint 根据当前状态直接选 token|`controls/Cargo.toml` 没有 motion 依赖，没有逐帧呈现状态通道|
|`controls/extended.rs` Disclosure、Switch、SegmentedControl|逻辑 open/checked/selected 状态切换|没有用共享动画通道管理当前呈现进度；Gallery 的 Disclosure recipe 不等于控件本体接入|
|`controls/scroll_view.rs`|自己的手势、惯性/回弹与 reduced_motion；开启偏好会清 overscroll/rebound|不是 Animated 消费方；不应为统一接口而替换已有物理、几何或滚动锚点行为|
|macOS `shared::check_appearance` → `Event::SystemMotionChanged` → Gallery host|读取 NSWorkspace 偏好并传递变化；Gallery 继续分发到预览和各滚动实例|系统偏好来源已存在；新组件应复用宿主事件，不在模型中再次读取系统或建立全局订阅|

以上为代码审计，未启动原生、GPU 或性能测量。Text/ScrollArea 的布局、viewport、显示/交互边界由主聊天负责。

## 独立新增模块

文件：`crates/tekes-ui-motion/src/state_transition.rs`。

主聊天集成时只需先在该 crate 的 `src/lib.rs` 添加：

```rust
pub mod state_transition;
```

当前本聊天不修改共享导出、Cargo 依赖或消费方。调用路径为 `tekes_ui_motion::state_transition::StateTransition`，无需根目录 re-export。

```rust
use std::time::Duration;
use tekes_ui_motion::{presets, state_transition::StateTransition};

let mut progress = StateTransition::new(0.0, Duration::ZERO, false)?;
progress.animate_to(1.0, presets::geometry(), Duration::ZERO)?;
let now = Duration::from_millis(80);
let frame = progress.advance(now)?;
// 逻辑状态已由组件自己更新；仅将 frame.sample 用于呈现。
let reversed = progress.animate_to(0.0, presets::geometry(), now)?;
assert_eq!(frame.sample, reversed.sample); // Spring 的位置、速度均连续
# Ok::<(), tekes_ui_motion::MotionError>(())
```

API：`new(value, now, reduced_motion)`；`animate_to(target, motion, now)`；`animate_to_with_velocity(target, velocity, motion, now)`；`advance(now)`；`set_paused(paused, now)`；`set_reduced_motion(reduced, now)`；`cancel(now, reason, behavior)`。只读接口为 `current/target/generation/is_paused/is_reduced_motion/needs_frame`。

## 行为契约

- 宿主提供同一 epoch 的单调 `Duration`，构造时间之前的时间不计入。模型没有墙钟、线程、回调队列、系统偏好读取或帧调度。
- Pause 会先采样到当前时间，再冻结内部活动时间；Resume 排除暂停期间的全部外部时间。暂停不取消、不重建 generation、不丢失 Spring 的续播速度；此时 sample.velocity 是每活动秒的续播速度，不代表屏幕正在移动。
- 重复 pause/resume 允许；重复 resume 时未暂停的时间仍正常推进。暂停中调用 animate_to 换目标/换参数会取消旧 generation，承接冻结的位置/速度，新 generation 保持暂停；Tween 仍只保证位置连续。没有单独的隐式“反向”规则，组件明确传入反向目标。
- Reduced Motion 策略明确限定为本适配层的保守状态/空间过渡：开启时 Finish 当前语义目标并报告 `Cancelled(ReducedMotion)`；之后新目标立即完成。关闭不重播旧动画。暂停中开启同样生效；不自动解除暂停。组件若要保留轻微 opacity fade，应另选明确策略，不隐式套用到全部属性或滚动物理。
- 操作先采样旧动画，所以已自然完成的旧 generation 仍可发完成事件；不将既有完成改写成取消。真正被取消的 generation 不会在后续 resume 时发成功完成。
- 不合法目标、时钟倒退或采样错误原子失败；内部时钟、偏好、暂停状态和 generation 均不部分提交。大 dt 沿用 Animated 的 finite deadline；暂停时的大 dt 不消耗 deadline。
- `needs_frame` 只表示通道活跃且未暂停；宿主还要检查窗口可见性、页面活动性并合并所有通道的请求。Reduced Motion 或取消使通道静止后，不再为该通道安排帧。
- 每属性通道归组件实例独占，事件核对组件 identity 和 generation。卸载时需要收据可调用 `cancel(Unmounted, Hold/Finish)`，再丢弃通道；此模块不处理节点保留、销毁或命中测试生命周期。

## 后续组件接线建议（由主聊天集成）

先补模块导出并完成常规 crate 检查。后续按组件的既有呈现合同逐个接线，不在本轮默认增加动效。控件逻辑 state 和语义命中区域保持同步更新，动画只承载展示进度；Button 的 hover/pressed 等已有 token 切换不因新模块自动变成混色或缩放。Disclosure/Switch 若要动画，应先明确呈现进度的拥有者、布局/clip/命中边界，以及关闭过程的内容生命周期。颜色空间、opacity clamp、尺寸边界均不是本数值模块的职责。

## 可重复验证与边界

```sh
python3 scripts/check-motion-transition.py
```

该脚本仅读取真实 motion 源码，在临时独立 crate 中增加待集成的导出，使用 Rust 1.95.0 离线运行新增测试、现有六组模型测试、doc tests 和 all-targets Clippy `-D warnings`。日志写到 `target/motion-state-transition-check/`，不修改仓库 Cargo/lock/lib.rs，不构建 Gallery，不运行任何原生/GPU 程序。

新增12项回归包括：暂停/继续排除时间、Spring 暂停中改目标/改参数、正常反向、重复操作/零 dt、大 dt/暂停 deadline、Hold/Finish 取消后不发旧完成、初始及运行中/暂停中 Reduced Motion 开关、错误原子性、手势释放速度。加上既有57项模型测试和1项文档示例，共70项通过；严格 Clippy 通过。这里的临时 crate 验证不等于真实共享导出、组件接线、视觉或性能验收。

## 性能现状和下一步

Metal 缩放恢复和暂停/继续/换目标已有原生功能证据；最终测量候选为 `0d19db7609ef9ad29e4ab9aafc0a30a1d5901d28d52a85438c234e17306be3c4`。长稳120Hz/8.33ms、CPU与启动约30秒的缺口仍未通过；正式五组空闲前检到期，启动0组。采样诊断有明显扰动，尚未定因，不能据此修改 autoreleasepool/latency。

证据见 `design/reviews/symbols-metal-lifecycle/`、`design/reviews/scroll-pages-retest/`、`design/reviews/symbols-drain-followup/`。等用户给出新的空闲窗口并和主聊天排期后，冻结当时实际源码/二进制，顺序复测24稳态、24调参、24切目标、72组合和24匀速旋转；保留原门槛、实际呈现收据、完整5秒空闲以及全部失败。当前不自动预检或测量。


## 2026-10-17 生产接线

本节取代上表中“controls 没有 motion 依赖/通道”的当前状态描述，保留原审计作为溯源。

`controls::control_motion::ProgressTransition` 由生产组件实例持有，复用 `StateTransition`。作者入口是 `design/source/motion.json` 的 `control_duration` 和 `control_transitions`；Cargo 编译生成每个角色的 duration/easing，组件不复制常数。此次采用有明确截止时间的 Tween，不改原 Motion Lab 的 recipe/spring 参数。

|生产呈现|JSON 角色|时长|语义与几何|
|---|---|---|---|
|Button hover|button_hover|200ms|中性/强调语义状态色过渡；按钮框、标签和命中坐标不移动|
|Button press|button_press|60ms|独立 pressed 通道；指针/Space 激活规则仍即时生效，不加缩放|
|Switch thumb|switch_thumb|120ms|逻辑 checked 即时更新，thumb 连续移动；track hit 和文字位置固定|
|Disclosure body|disclosure_reveal|300ms|组件返回完整 body_rect、visible_clip、occupied_height；关闭开始即禁止正文交互，反向承接当前进度|

Gallery `TextEngine` 实现可选 `ButtonTextRenderer::motion_context` 与帧请求，默认静态适配器保持源码兼容和确定性夹具。实际 Gallery 开启上下文，以宿主 delta 累加同一单调 Duration。每次场景 build 清理并重新收集可见组件的帧请求，Host 在绘制后同步 display link，避免首次 hover 请求滞留。组件不读取墙钟、不创建线程或独立计时器。Reduced Motion 直接收敛到语义终点；失活清除捕获、旧 hover 和呈现过渡；主题/品牌变化不会把两套语义颜色混成过渡。

真实 Button/Switch/Disclosure 的组件测试检查中途改目标、终点、Reduced Motion、可见几何和失活；`gallery/quality_effects.rs` 使用真实 CoreText 适配器与 GPU 比较三者起始/中间/结束帧，验证像素确实变化且命中框固定。生产浮层 GPU 检查另覆盖三种浮层、四主题与 1×/2×；截图、最终源码与日志绑定在 Gold7 本轮收据。它们不是性能/持续120Hz测试，也不能代替真实输入法或读屏验收。

后续浮层接线已完成 Menu/Dialog/Popover 的共享 `OverlayPresence`，由 JSON `overlay_presence` 的 150ms 标量过渡驱动。语义关闭即刻移除命中并恢复焦点，退场帧重建当前 atlas 字形；反向重开、Reduced Motion、上下文失效和卸载分别处理。当前逐图元 alpha 并非离屏图层合成，静止几何和 elevation 不变。Select/过滤菜单复用这一生命周期；四主题 GPU 及有限原生样本见 [Gold7 第三批收据](../../design/reviews/gold7-20261017/alignment-pass3/receipt.json)。剩余完整组件状态覆盖仍需逐项核对，不能扩大解释为全部 Gold7 或全部组件体验验收通过。
