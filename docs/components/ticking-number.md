# Ticking Number

Use TickingNumber for a fixed-width nonnegative integer display with animated digits.

## Creating the content

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

## State and events

Choose 1 to 15 digits; leading zeros are shown by default. Each cell uses the widest digit advance in the current font, keeping total width stable. Out-of-range values return an error. Targets can increase or decrease, and retargeting preserves position continuity.

## Composition and limitations

Intensity from 0 to 1 blends linear motion with quintic smoothstep; defaults are intensity 1 and duration 1.8 seconds. Changing duration or intensity restarts from the current position without guaranteeing velocity continuity. Reduced motion or zero duration presents the target immediately. Host owns clipping, reduced-motion preference and accessibility; announce a final value if needed, not every frame. This is a display, not numeric input.

## Related APIs

[Number Field](number-field.md)
