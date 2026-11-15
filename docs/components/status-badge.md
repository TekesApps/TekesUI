# StatusBadge

StatusBadgeContent 展示状态语义与文字，是被动反馈组件。状态由消费者根据业务事实提供。

## 短用法

```rust
use tekes_ui_controls::{composition::StatusBadgeContent, extended::StatusBadge};
let badge = StatusBadgeContent::new(StatusBadge::Info, "Ready").unwrap();
```

## 读取与更新

`model()`、`label()`、`layout()` 返回原状态和已有几何；`update(model, label, name, description, decorative)` 更新状态、标签和语义并返回校验结果。需要纯装饰时使用 `decorative(model, label)`，不为装饰制造额外焦点停靠。

状态和文字一起表达含义，不能只靠颜色判断。Theme 解析原 StatusBadgeView 的语义色，不把颜色常量散落在消费者。Badge 不发业务动作，也不会因为 Running 自动执行工作。

进度数值与成功结果是不同信息：100% 不应自动转成 Succeeded，未知和未验证也不能当作成功。TickingNumber 的数字动画另见既有 `docs/components/ticking-number.md`，不属于 Badge 的状态模型。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/status_badge.rs`。现有消费路径：`examples/composition-consumer/src/status_badge_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
