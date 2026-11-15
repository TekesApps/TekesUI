# Popover

PopoverContent 在同一 Host 中挂载交互 body，并将一次展示绑定到原 anchor 的挂载身份。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let popover = PopoverContent::new("Preferences",
    Content::new(ComponentId(2), TextFieldContent::new("Name", "")));
// 将 popover 和 anchor 都挂载后：
// let session = host.show_popover(popover_id, anchor_id)?;
// host.close_popover(session);
```

## 会话、关闭和焦点

`show_popover(owner, anchor)` 返回 PopoverSession，关闭使用该 session；旧会话不能误关新展示。`is_open()` 是逻辑状态，`is_presented()` 还包含退出呈现；`needs_animation()` 不等于可交互。

`current_session()`、`layout()` 只读，`take_closed()` 消费 PopoverClosed 记录。`set_dismiss_visible` 控制关闭动作显示，`set_enabled` 管理可用性。Host 负责锚点、作用域、焦点恢复和关闭路由；body 内保留原文本服务和动作。

它不是第二个 ContentHost 或独立窗口。被动 hover preview 有 HoverPreviewContent 的独立合同，不能把 hover 自动当成打开可编辑 Popover 的用户意图。clear_presentation 是呈现清理，不是业务保存。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/popover.rs`。现有消费路径：`examples/composition-consumer/src/popover_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
