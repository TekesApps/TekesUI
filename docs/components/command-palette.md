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

定义：`crates/tekes-ui-controls/src/composition/navigation/command.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
