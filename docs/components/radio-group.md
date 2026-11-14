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

定义：`crates/tekes-ui-controls/src/composition/radio.rs`。现有消费路径：`examples/composition-consumer/tests/radio_content.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
