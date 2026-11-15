# IconButton

IconButton 复用正式 Button，不另造按钮类型。图标属于内容，按钮拥有可访问名称、焦点、按压、取消和一次激活。

## 短用法

```rust
use tekes_ui_controls::{Button, composition::*};
let icon = Content::new(ComponentId(2), IconContent::new("tekes-settings").unwrap());
let action = Button::new(icon).with_accessible_name("Settings");
```

## 状态和组合

使用 Button 的 `variant`、`pointer_activation_phase`、`image_position`、`content_alignment` 与 `set_contents`。需要图像 ID 时也支持 `Button::new(ImageId(...))`，宿主必须持有并准备对应资源。

将按钮放入 ContentHost，启用动作记录后处理该 target 的 Activate。不要因图标 hover、Inspector 选择或绘制而触发业务动作。图标子树应为被动装饰，不能嵌套另一个可交互按钮。

无文字按钮必须提供有意义的名称；尺寸和工具栏紧凑行为遵循实际布局，不能从图标像素推断命中范围。普通按钮的完整说明见 `docs/components/button.md`。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/button.rs`。现有消费路径：`examples/composition-consumer/tests/unified_button.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
