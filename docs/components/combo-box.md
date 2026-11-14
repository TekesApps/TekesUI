# ComboBox

ComboBox 是可编辑查询加候选列表。正式入口 `tekes_ui_controls::composition::ComboBoxContent` 保留一个原 TextField 编辑器及 ComboBox 模型、View；业务层提供候选数据并消费选择。

## 最小组合

```rust
use tekes_ui_controls::composition::*;

let combo = ComboBoxContent::new("Layout", vec!["Compact".into(), "Comfortable".into()]);
let mut host = ContentHost::new(Content::new(ComponentId(6), combo)).unwrap();
host.set_action_events(true);
```

正式消费者见 `examples/composition-consumer/tests/combobox_content.rs`。较早的 `src/combo_status.rs` 是手工组合 Query 的示例，不是当前 ComboBoxContent 的类型定义。

## 属性与读取

- `model()` 只读提供 items、query、selected、active、open 等原模型状态；active 候选不等于已选择值。
- `text_surface()` 读取实际编辑器；`popup_layout()` 读取可用的弹层布局，不驱动布局。
- `set_items(Vec<String>)` 返回 `Result<bool, ComboBoxUpdateError>`，相同声明返回未变更；更新会撤销旧候选授权。
- `set_query(...)` 同步查询。禁用时返回未变更；不同查询遇到正在组字返回 Composing，消费者应保留错误并在合法更新时机处理。
- `set_disabled(bool)` 更新可用性并取消旧交互；`set_error(Option<String>)` 设置字段错误。

## 事件与组合

启用动作后使用 `host.drain_action_records()`，按 target 过滤 `ContentActionPayload::ComboBoxSelection`。其 `index()`、`label()`、`declaration_revision()` 是交付时捕获的原始候选身份；不要用过滤后行号或稍后模型的 selected 代替事件值。读取动作会消费队列，Inspector 应只读 model/text_surface。

弹层及键盘导航由组件和 ContentHost 管理，原生文本服务由 AppHost 提供。组字时不把候选确认键同时当作列表选择。应用负责检索、异步结果时序和业务 ID 映射；不要增加第二个编辑器或弹层输入循环。

## 依据与限制

定义与更新语义见 `crates/tekes-ui-controls/src/composition/combobox.rs`；事件类型见 `composition/actions.rs`。候选是字符串列表，公共入口不是任意富内容或远程搜索服务。IdentityExhausted 等更新错误不能伪装成成功。本页是源码消费指南，本批未运行其既有测试、GUI、物理输入或无障碍验收。
