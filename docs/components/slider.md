# Slider

SliderContent 为有限数值范围提供滑动条；原模型是 extended::Slider，原 View 拥有拖动和键盘行为。

## 短用法

```rust
use tekes_ui_controls::{composition::SliderContent, extended::Slider};
let slider = SliderContent::new("Volume", Slider::new(50., 0., 100., 5.).unwrap());
```

## 值和输入

`model.value()` 与 `model.range()` 读取值和范围，`model.disabled` 控制可用性，`name` 提供语义名称。构造的 Option 必须处理，业务配置不应无条件 unwrap。

拖动可以多次改变值；方向键、Home/End 和可访问增减沿原 View/Host 路由。一次 ContentResponse.changed 也可能来自交互状态变化，业务应读取实际数值并比较自己的已发布值；此入口没有专门的 Slider 数值快照 payload，不把普通 Activate 当成唯一数值更新来源。

父布局提供可用滑轨范围。它不是文字数值编辑器，不承担单位解析、持久化或业务撤销；精确输入可组合 NumberField。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/adapters.rs`。现有消费路径：`examples/composition-consumer/src/slider_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
