# Tooltip

普通控件帮助使用 Content::with_help，由 Host 的帮助 overlay 复用 TooltipView；底层 extended::Tooltip 是文本模型。

## 短用法

```rust
use tekes_ui_controls::{Button, composition::*};
let action = Content::new(ComponentId(2), Button::new("Save"))
    .with_help("Save this document");
```

## 显示和输入

`set_help(Option<String>)` 更新帮助，`help_text()` 读取声明；`host.help_tooltip()` 返回当前帮助 owner、文字和 OverlayLayout。直接使用 TooltipView 时消费者还需负责锚点、视口、hover/focus、活动状态与 Escape，不能只调用绘制就声称完整帮助接入。

帮助不产生业务激活，不应夺取编辑器焦点。布局受锚点可见区域和视口约束，不用离屏控件的旧矩形定位提示。宿主可按既有 deferred help overlay 路径统一末层绘制。

Tooltip 是短帮助，不是包含按钮和表单的弹窗，也不替代持续可见的错误说明；交互弹层使用 PopoverContent。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/help.rs`。现有消费路径：`examples/composition-consumer/src/help_list.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
