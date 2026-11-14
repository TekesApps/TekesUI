# SplitView

SplitViewContent 挂载两个真实 pane 和分隔条，保留各 pane 内的编辑器、滚动和焦点身份。构造显式提供配置、slot ID 和工作预算。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let split = SplitViewContent::try_new(SplitViewConstruction {
    config: SplitViewConfig::default(),
    slots: SplitViewSlotIds { leading: ComponentId(2), trailing: ComponentId(3), divider: ComponentId(4) },
    leading: Content::new(ComponentId(5), TextContent::new("Leading")),
    trailing: Content::new(ComponentId(6), TextContent::new("Trailing")),
    budget: 64 * 1024 * 1024,
}).unwrap();
```

## 更新、输入和尺寸

`SplitViewConfig` 包含 axis、ratio、divider、min_pane、collapsed。`config()` 和 `layout()` 是可选状态读取；未准备布局不能冒充测量成功。ratio 是分配配置，不是每个 pane 的最终像素尺寸。

挂载后使用 Host 的 `split_view_expected`、`update_split_view` 和 `replace_split_view_pane`，让版本、挂载身份和替换事务保持一致；拒绝值保留输入供调用者处理。分隔条的指针和键盘调整由原组件处理，消费者不增加第二个拖动层。

pane 自己决定是否滚动；SplitView 不虚拟化内容，也不将一个 pane 的文本服务转移给另一个。上例预算取自现有消费者，不是所有业务规模的推荐常量。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/split_view.rs`。现有消费路径：`examples/composition-consumer/tests/split_view_content.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
