# Field

Field 提供表单标签、required、帮助与错误元信息；实际输入组合通常使用 TextFieldContent，FieldView 负责元信息和几何。

## 短用法

```rust
use tekes_ui_controls::composition::TextFieldContent;
let mut field = TextFieldContent::new("Project name", "");
field.field.required = true;
field.field.help = "Use a recognizable name.".into();
field.field.error = Some("A name is required.".into());
```

## 职责边界

`field.label`、`help`、`error` 与 `required` 是说明和校验状态，输入文字、disabled、read_only、组字和选区仍在 TextSurface.input。required 标记不自动验证或拒绝保存；错误提示也不是自动弹出 Dialog。

按正式 TextFieldContent 路径提交与读取动作，Field 本身不生成业务保存事件。元信息占用布局高度，Fill/Grow 视口需要扣除这部分空间；不要把错误文字遮盖在输入区上。

这不是任意组件的自动表单框架。业务验证、跨字段依赖和异步错误归消费者，帮助 hover 与可见 field.help 也有不同呈现职责。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/adapters.rs`。现有消费路径：`examples/composition-consumer/src/field_metadata.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
