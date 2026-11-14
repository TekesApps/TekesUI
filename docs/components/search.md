# Search

Search 是一个查询输入框。查询文字、选区、撤销和输入法组字由原编辑器管理；消费者负责过滤或执行检索。

## 最小组合

```rust
use tekes_ui_controls::composition::*;

let mut search = SearchFieldContent::new("Search documents", "");
search.set_placeholder(Some("按名称搜索…".into()));
search.set_submit_policy(TextSubmitPolicy::Enter);
let mut host = ContentHost::new(Content::new(ComponentId(1), search)).unwrap();
host.set_action_events(true);
```

该组合对应现有独立消费者 `examples/composition-consumer/src/search_field.rs`。
真实窗口由 AppHost 接入平台文本服务；上面只创建内容，不自行启动输入循环。

## 属性与读取

- `SearchFieldContent::new(name, query)` 设置可访问名称和初始查询。
- `set_placeholder(Some(text))` 设置空值提示；提示不是查询值。
- `set_submit_policy(...)` 设置提交策略。
- `set_availability(read_only, disabled)` 区分只读与禁用。
- `set_error(...)` 设置错误信息；错误、禁用和焦点保持各自语义。
- `set_clear_label(...)` 设置清除动作的可访问名称并返回校验结果。
- `query()` 直接读取当前查询；`apply_query(...)` 使用 baseline、DraftRevision、选区和撤销策略同步外部查询，返回原 ExternalValueResult，不产生用户动作。
- `input()` 只读访问原 TextInput。通过 Host 的 `get(id)` 和 `component::<SearchFieldContent>()` 读取当前实例。

## 事件

启用动作记录后，在一次正式 Host 输入处理完成后读取 `drain_action_records()`。
`ContentActionPayload::Search` 携带 `SearchAction`：Submit 使用提交时的查询；Clear 携带清空前的查询。
不能收到提交事件后重新读取文字，冒充当时提交的值。读取动作队列会消费记录，Inspector 不能用它刷新属性。

## 输入与布局

组件只有一个文本输入所有者，清除按钮不会创建第二套焦点或撤销状态。
输入法候选确认和普通提交是不同阶段；窗口必须保留原生键来源与组字边界。
搜索结果使用独立 List 等组件，由消费者根据查询更新，不把网络请求放入 Search。

## 限制

此页描述正式本地 Search 内容合同。自动原生输入、物理输入、VoiceOver 和性能证据分别记录；不由示例创建成功推定全部验收通过。

公共定义见 `crates/tekes-ui-controls/src/composition/search.rs`；动作类型见 `composition/actions.rs`。本批仅核对定义和消费者，未重新执行示例或原生验收。
