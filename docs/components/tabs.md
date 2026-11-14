# Tabs

TabsContent 组合实际标签和面板；TabItem 保存两个 Content 的身份。标签页选择与关闭请求是不同意图。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let tabs = TabsContent::new(vec![TabItem::new(
    Content::new(ComponentId(2), TextContent::new("Overview")),
    Content::new(ComponentId(3), TextFieldContent::new("Draft", "")),
)], 0);
```

## 状态和事件

`selected()` 读取当前索引，`select(index)` 程序选择；`set_tab_disabled`、`set_tab_open`、`reorder` 操作真实声明，返回值须处理。TabItem 可声明 name、disabled、closable。

启用 Host 动作后，TabSelection 捕获 index、label、panel；TabCloseRequest 只请求关闭，面板保持挂载直到消费者批准，不自动选择该 tab。不要用当前索引代替事件里的 label/panel 身份。

隐藏面板的输入、焦点和语义由原组件管理，页面不应另做一套“透明但可点击”的隐藏层。TabsContent 负责面板组合；仅页签标题条另有 TabStripContent 合同，不把它的请求类型混用。关闭前保存策略与文档历史属于消费者。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/navigation.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
