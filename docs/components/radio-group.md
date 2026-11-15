# RadioGroup

RadioGroupContent 提供一组互斥选项，内部仍由原 RadioGroup/RadioGroupView 管理选择和语义子项。

## 短用法

```rust
use tekes_ui_controls::composition::RadioGroupContent;
let group = RadioGroupContent::new("Mode", vec!["One".into(), "Two".into()]);
```

## 值、事件和焦点

`model()`、`view()` 只读访问原状态；`set_selected(Option<usize>)` 和 `set_item_enabled` 返回 Result，`set_disabled` 与 `set_error` 分别管理可用性和错误；`set_axis` 调整排列方向。

启用 Host 动作后消费 RadioSelection payload，`RadioSelectionIntent::index()` 是交付时的索引。延迟业务消费可用 `intent_is_current` 判断意图是否仍对应原声明；不要把语义 part token 当持久业务 ID。

原组件管理组选项的键盘与可访问动作；外层滚动仍由 ScrollContent 管理。这是单选组，独立多选状态用 Checkbox；不承诺任意富内容候选或远程列表。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/radio.rs`。现有消费路径：`examples/composition-consumer/tests/radio_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
