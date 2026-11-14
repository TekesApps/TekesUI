# Disclosure

DisclosureContent 是受控展开区域：标题、触发器和 body 保持真实组件身份，业务层决定是否接受展开请求。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let title = Content::new(ComponentId(3), TextContent::new("Details"));
let body = Content::new(ComponentId(4), TextFieldContent::new("Notes", ""));
let disclosure = DisclosureContent::new(ComponentId(2), title, body, false);
```

## 请求与状态

`expanded()`、`disabled()`、`trigger_id()` 读取状态。消费 `ContentActionPayload::DisclosureExpansion` 中的 DisclosureIntent，并调用 `confirm(&intent, accepted)`；`expanded()` 读取请求目标，generation/revision 绑定当时声明。不要用读取文档的操作确认业务请求。

`set_expanded` 用于权威状态同步；`set_disabled`、`set_rtl`、`set_accessible_name` 管理元信息。`with_section_header_layout` 提供分组标题布局，`with_trigger_focusable` 控制触发器焦点策略。

body 的编辑器和滚动由自身持有，折叠不应通过重建 body 丢失草稿。此入口不是自动保存表单，也不替消费者决定是否允许收起未保存内容。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/disclosure.rs`。现有消费路径：`examples/composition-consumer/src/disclosure_review.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
