# Apple 官方默认动画配置

核对日期：2026-10-15。实现：`tekes_ui_motion::apple_defaults::AppleDefault`。Gallery → Motion → **Apple defaults**。项目 JSON 语义预设仍放在独立的 **Design System** 中。

| API | 默认参数 | 官方来源 |
| --- | --- | --- |
| `.default`（现代 SwiftUI） | response 0.55s；dampingFraction 1；blendDuration 0 | [Animation.default](https://developer.apple.com/documentation/swiftui/animation/default) |
| `.spring()` | duration 0.5s；bounce 0；blendDuration 0 | [spring](https://developer.apple.com/documentation/swiftui/animation/spring(duration:bounce:blendduration:)) |
| `.smooth` | duration 0.5s；基础 bounce 0；extraBounce 0 | [smooth](https://developer.apple.com/documentation/swiftui/animation/smooth(duration:extrabounce:)) |
| `.snappy` | duration 0.5s；基础 bounce 0.15；extraBounce 0 | [snappy](https://developer.apple.com/documentation/swiftui/animation/snappy(duration:extrabounce:)) |
| `.bouncy` | duration 0.5s；基础 bounce 0.3；extraBounce 0 | [bouncy](https://developer.apple.com/documentation/swiftui/animation/bouncy(duration:extrabounce:)) |
| `.easeIn` | 0.35s；Bézier (0.42, 0, 1, 1) | [Animation](https://developer.apple.com/documentation/swiftui/animation/easein)、[UnitCurve](https://developer.apple.com/documentation/swiftui/unitcurve/easein) |
| `.easeOut` | 0.35s；Bézier (0, 0, 0.58, 1) | [Animation](https://developer.apple.com/documentation/swiftui/animation/easeout)、[UnitCurve](https://developer.apple.com/documentation/swiftui/unitcurve/easeout) |
| `.easeInOut` | 0.35s；Bézier (0.42, 0, 0.58, 1) | [Animation](https://developer.apple.com/documentation/swiftui/animation/easeinout)、[UnitCurve](https://developer.apple.com/documentation/swiftui/unitcurve/easeinout) |
| `.linear` | 0.35s；线性 | [Animation.linear](https://developer.apple.com/documentation/swiftui/animation/linear) |

注意 `.default` 与 `.spring()` 不同；旧系统的 `.default` 也不同。Spring 的 duration 是感知时长，不是强行把运动在该时刻截断。API 默认值不等于每一种 Apple 系统控件都使用这些参数。

当前目录里的非负 bounce 映射到阻尼比 `1 - bounce`，频率为 `1 / duration`；response 模型频率为 `1 / response`。使用解析振子，与现有保速重定向机制兼容。只复现这些单属性轨迹，不承诺 SwiftUI transaction、跨动画 blending、布局和 completion 回调完全等价；未支持的 `.interactiveSpring` 没有冒充普通弹簧加入目录。

## 数值核对

项目外临时参考脚本 `/tmp/tekesui-apple-motion-reference/apple_oracle.swift`（已迁出项目，保留原文件） 仅作数值参考，调用公开 SwiftUI `Spring` / `UnitCurve`；实际 Rust UI 无 Swift 运行时实现。macOS SDK 26.5 / Swift 6.3.3 生成 `apple-oracle.csv`，含 63 个位置/速度或曲线样本。Default 的 oracle 是按文档参数构造的 Spring，不是对不透明 Animation.default 的直接取样。

Rust 测试对弹簧位置误差阈值 1e-11、速度 1e-9；UnitCurve 输出与精确 Bézier 反解允许 2e-6 差异（公开接口的近似求值精度）。这验证采样位置，不代替所有时间、所有初速度、事务及宿主行为的原生一致性验证。

```rust
use tekes_ui_motion::apple_defaults::AppleDefault;
let motion = AppleDefault::Snappy.motion();
```

跨品牌参考与下一层配方见 [motion-references.md](motion-references.md)。
