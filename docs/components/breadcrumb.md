# Breadcrumb

BreadcrumbContent 显示当前位置路径，每个祖先是导航链接，末项是当前页。它不会自行修改应用路由。

## 短用法

```rust
use tekes_ui_controls::{composition::BreadcrumbContent, extended::Breadcrumb};
let path = BreadcrumbContent::new(Breadcrumb {
    items: vec!["Workspace".into(), "Documents".into(), "Current".into()],
});
```

## 导航和布局

`model.items` 是显示路径，`view` 保留原 BreadcrumbView。`set_disabled` 和 `set_window_active` 更新输入可用性。

Host 动作中的 BreadcrumbNavigation 捕获 index 和 path，消费者根据当时路径导航。当前页不是可导航祖先；更新路径后不能用上一帧命中矩形激活新的祖先。

键盘、指针与可访问动作走原 View，外层提供宽度及裁剪。字符串标签不等于唯一业务路由 ID，应用需要维护对应关系；该入口不是自动面包屑折叠菜单或文件系统浏览器。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/navigation/breadcrumb.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
