# ScrollArea

ScrollArea 是滚动模型，`ScrollContent` 把一个真实 Content 子树接入 Host 的视口、裁剪、滚轮和阅读路径。

## 短用法

```rust
use tekes_ui_controls::{ScrollArea, composition::*};
let body = Content::new(ComponentId(2), TextContent::new("Scrollable content"));
let scroll = ScrollContent::new(ScrollArea::default(), body).scrollbar(true);
```

## 状态与阅读职责

`scroll` 公开原模型；`viewport_height` 可声明视口高度。`horizontal(true)` 选择自然内容宽度并禁用纵向滚动；`scrollbar(visible)` 改变溢出条显示，不改变滚轮或 reveal 策略。默认构造隐藏滚动条，不代表滚动被禁用。

阅读集合可通过 Host 的 `set_scroll_reading_document` 声明文档身份和顺序，使用原阅读更新事务保留锚点；不能把每次追加内容都解释成用户要求跳到末尾。子 TextSurface 仍拥有自己的文字、选区和插入点显露。

父布局必须给视口有限空间。ScrollContent 不移除字段边框，不自动把所有子项虚拟化，也不替业务读取远程分页。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition.rs`。现有消费路径：`examples/composition-consumer/src/scroll_updates.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
