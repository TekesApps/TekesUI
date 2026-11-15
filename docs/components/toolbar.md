# Toolbar

ToolbarContent 和 ToolbarGroup 在真实 Content 树里组织工具栏。AppHost 中 Toolbar 归 Container 所有；页面不应再安装一个窗口级输入循环。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let bar = ToolbarContent::new(vec![Content::new(
    ComponentId(2),
    ToolbarGroup::new(ToolbarPlacement::PrimaryAction, vec![
        Content::new(ComponentId(3), Button::new("Save")),
    ]),
)]);
```

## 分组和事件

ToolbarPlacement 表达 Navigation、Status、PrimaryAction 等角色。文字状态保持 TextContent，不应为对齐而伪装成按钮。ToolbarGroup 的 `set_radio` 和 `set_selected_id` 可声明互斥组，选择 ID 必须对应该组真实子项。

各控件沿原 Host 动作路径交付；启用 `set_action_events(true)` 后按 target 处理按钮等动作。工具栏不是业务命令注册表，不能从位置或显示文字推断授权。

低层 Toolbar/ToolbarControl/ToolbarView 与 Content 入口同时存在，需按宿主选一个实际消费路径。紧凑样式及溢出策略由原布局实现决定，不把所有内容强制变成等宽图标按钮。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/toolbar_content.rs`。现有消费路径：`examples/composition-consumer/src/toolbar_groups.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
