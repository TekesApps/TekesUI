# Progress

ProgressContent 展示已知比例或明确未知的进度，复用原 Progress 模型和 ProgressView，是非交互指示器。

## 短用法

```rust
use tekes_ui_controls::composition::ProgressContent;
let mut progress = ProgressContent::new("Transfer", 0.45);
progress.description = Some("Downloading document".into());
progress.model.indeterminate = false;
```

## 数值、布局与语义

已知进度使用 0..1 比例，`model.value()` 读取、`model.set(value)` 更新；语义输出按百分比表达。`model.indeterminate = true` 表示未知，不能用保留的 scalar 伪报 0%。`layout()` 读取原布局。

父级必须提供有限条形区域，标签和加载、失败、成功文案是普通兄弟组件。Progress 不持有键盘焦点、不处理增减数值动作、不发完成事件。

当前未知进度呈现是静态段，不应许诺持续动画。到达 1.0 只是测量值，是否成功需要业务结果，不能把它当作自动验收或重试控制器。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/progress.rs`。现有消费路径：`examples/composition-consumer/src/progress_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
