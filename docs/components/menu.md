# Menu

MenuContent 组合触发器和原菜单。简单字符串菜单可直接构造；业务命令应使用 MenuEntry 的显式 value，避免依赖可能变化的标签。

## 短用法

```rust
use tekes_ui_controls::composition::{MenuContent, MenuEntry};
let mut menu = MenuContent::new("Actions", vec![]);
menu.set_entries(vec![MenuEntry::item("save", "Save")]).unwrap();
```

## 声明和动作

MenuEntry 支持 item、submenu 及声明式项目，`set_entries` 返回 Result。`set_values`、`set_item_checked`、`set_item_disabled`、`set_item_separator` 修改原声明；`model_revision()` 可读取版本。弹层几何用 `set_popup_viewport` 限定。

Host 动作中的 MenuSelection 捕获 index、label、model_revision、可选 value_path；消费路径身份，不用关闭后的模型反推选项。上下文菜单还有 context_request，必须使用 Host 的 bind/request context menu 合同绑定 owner。

原 View/Host 负责子菜单、键盘、指针和关闭清理；消费者执行命令，不给每一行再安装独立点击代理。菜单不是应用全局快捷键或权限注册器，更新声明失败不能当作成功。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/menu.rs`。现有消费路径：`examples/composition-consumer/src/declared_menu.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
