# Ticking Number

`tekes_ui_controls::TickingNumber` 是固定数位的非负整数计数组件。
Gallery 的 **Foundations → Motion → Playground** 页面的 **Ticking Number** 独立 tab 提供交互演示（排在 Symbols 后）。

- 1–15 位，默认补前导零；例如三位的 `7` 显示为 `007`。
- 每个数字格宽度取当前字体 `0–9` 中的最大字形 advance，总宽度恒为格宽 × 位数。每个字形在格内居中，字体或字号改变后重新测量。
- 值超出预设位数时返回错误，不截断、不扩宽。
- 从当前呈现位置向目标插值；升降均支持，中途改目标保持位置连续。
- 强度 `0..=1`：0 为匀速，1 为完整的 quintic smoothstep 加减速，中间值混合两种曲线。默认 1，时长默认 1.8 秒。
- 时长与强度变化从当前位置重新计时；不承诺中断时速度连续。
- 减少动态效果或零时长立即呈现目标，完成后不再请求动画帧。

```rust
use std::time::Duration;
use tekes_ui_controls::TickingNumber;

let mut number = TickingNumber::new(137, 3).unwrap();
number.set_duration(Duration::from_millis(1800));
number.set_strength(1.0).unwrap();
number.set_target(546).unwrap();
// 在宿主每帧调用：
number.advance(Duration::from_secs_f64(1.0 / 120.0));
// 用 measure(...) 预留布局，draw(...) 通过宿主文字适配器绘制。
// needs_animation() 为 true 时继续调度帧；无自有 timer 或线程。
```

宿主负责裁切、系统减少动态效果偏好和无障碍名称/值。不要把每帧变化作为 live announcement；需要时在计数完成后报告最终值。

Gallery 支持切换 2 / 3 / 6 位、强度 0 / 0.5 / 1、时长 1 / 2 / 4 秒，以及反向计数。两位演示为 `37 ↔ 46`，其他位数为 `137 ↔ 546`。
