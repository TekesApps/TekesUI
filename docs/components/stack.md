# Stack

Stack 沿一个轴排列实际 Content 子树。组合入口 `StackContent` 使用测量后的自然尺寸；底层 `extended::Stack::layout` 的等分几何不是相同的尺寸策略。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::{Axis, Stack}};
let stack = StackContent {
    model: Stack { axis: Axis::Vertical, gap: 8. },
    children: vec![Content::new(ComponentId(2), TextContent::new("Title"))],
    flexible_child: None,
};
```

## 尺寸和职责

`model.axis` 与 `gap` 控制方向和间距；`children` 是真实组件，不是预绘制位图。`flexible_child: Some(id)` 让一个子项使用固定兄弟项之外的主轴空间，适合正文旁的图标或剩余高度的编辑区。`into_content_with_distribution(id, distribution)` 显式选择主轴分布。

Stack 自身没有业务事件或焦点停靠，子项仍走 ContentHost 的原输入树。它没有隐式的通用 flex/shrink 算法，也不创建滚动视口；溢出由宿主 clip 或 ScrollContent 管理。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
