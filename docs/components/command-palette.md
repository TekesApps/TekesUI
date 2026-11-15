# CommandPalette

CommandPaletteContent 组合显示型触发器、原 TextFieldContent 查询和 CommandPalette 模型。选择命令后由消费者执行。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::{CommandPalette, ComboBox}};
let palette = CommandPaletteContent::new(
    Content::new(ComponentId(2), TextContent::new("Commands")),
    Content::new(ComponentId(3), TextFieldContent::new("Filter commands", "")),
    CommandPalette { combo: ComboBox { items: vec!["Open File".into()], ..Default::default() } },
);
```

## 查询和交付

`open()`、`close()` 改变打开状态，`is_open()` 与 `options()` 读取原弹层状态。触发器仅提供显示，动作与语义由 CommandPaletteView 拥有，不在触发器内部嵌套另一个业务按钮。

查询必须是原 TextFieldContent，关闭时保留挂载身份和草稿，但退出当前布局、输入、焦点及语义范围。键盘、IME 和候选选择由 Host 仲裁。CommandSelection payload 捕获 index 与 label，应用不能在收到选择后重新查询过滤结果来猜原命令。

模型中的字符串候选不构成插件发现、快捷键注册或权限系统；命令执行和失败反馈由消费者组织。此组件不自动打开全局窗口级命令面板。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/navigation/command.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
