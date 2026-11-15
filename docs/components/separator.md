# Separator

Separator 是被动分隔线，使用 `composition::SeparatorContent` 挂载原 SeparatorView。

## 短用法

```rust
use tekes_ui_controls::{composition::SeparatorContent, extended::Axis};
let separator = SeparatorContent::new(Axis::Horizontal);
```

## 几何和组合

`new(axis)` 选择线的方向；`layout()` 返回已有 SeparatorLayout。父容器提供可用长度，实际线宽与颜色走原 View 的 Theme 路径，不在业务代码硬编码像素颜色。

分隔线不接受焦点、不发业务动作，也不是可拖动的 splitter。需要调整两侧空间时选择 SplitView；它不创建滚动或替代父背景。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/separator.rs`。现有消费路径：`examples/composition-consumer/tests/separator_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
