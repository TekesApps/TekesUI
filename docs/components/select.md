# Select

Select 从有限选项中选择一个值，不提供可编辑查询。正式组合入口是 `tekes_ui_controls::composition::SelectContent`；需要输入筛选时使用 ComboBoxContent。

## 最小组合

```rust
use tekes_ui_controls::composition::*;

let select = SelectContent::new("Mode", vec!["One".into(), "Two".into()]);
let mut host = ContentHost::new(Content::new(ComponentId(7), select)).unwrap();
host.set_action_events(true);
```

对应 `examples/composition-consumer/tests/select_content.rs` 和 `select_content_mixed.rs` 的正式入口。这里只挂载内容，窗口、布局及平台输入由宿主提供。

## 属性与读取

- `labels()` 与 `model()` 只读提供选项声明及当前 Select 状态；空选项列表不应被当作已选中有效业务值。
- `layout()` 返回原 SelectView 的布局结果，读取不重新排版。
- `set_labels(...)` 返回 `Result<bool, SelectUpdateError>`；相同列表不变，更新声明会废弃旧选项动作。
- `set_selected(index)` 返回 bool；越界返回 false。程序设置值会取消旧弹层交互，不等于用户选择事件。
- `set_disabled(bool)` 管理可用性；`set_error(bool)` 控制错误状态。

## 事件与输入

通过 `drain_action_records()` 读取 `ContentActionPayload::MenuSelection`，使用 `index()` 和 `label()` 获取交付时捕获的选择。Select 的字符串选项不提供业务 value_path；业务 ID 应由消费者和对应声明一起管理。即使再次选择同一值，也应以正式动作记录判断用户意图，不以模型差值推测。

Host 统一处理焦点、弹层键盘、指针和可访问动作；组件管理打开、选择、关闭和取消。Select 不接受文本输入，和旁边 TextField 组合时不能把弹层键再派给编辑器执行第二个动作。

## 布局与限制

外层布局分配触发器空间，弹层使用原 SelectView/Host 的 overlay 路径和裁剪，不用另一个窗口伪造列表。说明文档不保证当前弹层已布局或可见。

定义：`crates/tekes-ui-controls/src/composition/select.rs`；动作合同：`composition/actions.rs`。本页描述字符串选项和单选入口，不承诺多选或任意自定义行。本批仅核对源码，未运行既有测试、GUI 或平台输入验收。
