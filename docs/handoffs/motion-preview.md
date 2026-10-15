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
