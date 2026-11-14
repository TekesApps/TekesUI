# TextArea

TextArea 用于多行草稿、备注和消息输入。公共组合入口是 `TextFieldContent` 配合多行 `TextSurface`，没有独立的 TextAreaContent 类型。

## 最小组合

```rust
use tekes_ui_controls::{TextSurface, composition::*};

let mut area = TextFieldContent::new("消息", "");
area.surface = TextSurface::new("", true, false);
area.viewport = TextViewport::Grow { min: 56., max: 160. };
area.tab_policy = TextTabPolicy::Insert;
area.submit_policy = TextSubmitPolicy::CommandEnter;
let mut host = ContentHost::new(Content::new(ComponentId(5), area)).unwrap();
host.set_action_events(true);
```

这段采用 `examples/composition-consumer/src/text_viewport.rs` 中 composer 的公共构造路径。TextSurface 的后两个参数分别表示多行和只读；不要把第三个参数误当成自动换行开关。

## 属性、输入和事件

`surface.input` 保存文字、选区与 marked text；`field` 保存标签、帮助和错误等元信息。外部草稿更新使用 `surface.apply_external_value(...)` 的 baseline、版本、选区与撤销策略，避免直接覆写文字破坏组字或已有编辑。

`viewport` 控制输入视口：Fixed 为固定高度，Grow 在 min/max 之间增长，Fill 使用扣除字段元信息后的剩余分配。Fill 子项还需要父 Stack 的 `flexible_child` 分配剩余空间。

默认 Tab 策略 Navigate 用于表单；Insert 用于多行编辑，Backtab 会被消费但不执行反缩进。默认提交策略 None；上例 CommandEnter 让普通 Enter 保留多行编辑含义。提交意图经 Host 动作通道交给业务层，不表示消息已发送。IME 确认与提交由原生输入链仲裁，不自行再监听一次 Enter。

## 滚动与组合

TextSurface 管理编辑视口与插入点显露。外部 ScrollContent 负责页面滚动，不能替代编辑器内的 caret、选区或组字所有权。切换 Documents 时保留组件实例和 ComponentId，不重建用户草稿。

只读长文可用 `TextFieldContent::read_only_content(...)` 或 RichDocumentContent；普通多行 Field 保留输入轮廓，不能因为放进 ScrollArea 就变成长文阅读样式。

## 依据与限制

定义：`crates/tekes-ui-controls/src/composition/adapters.rs` 的 TextFieldContent、TextViewport；键策略见 `composition/key.rs`。示例是已有消费者的构造摘录，本批未编译 snippet、未运行物理 IME、GUI 或 VoiceOver 验收。
