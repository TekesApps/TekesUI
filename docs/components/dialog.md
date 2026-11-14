# Dialog

DialogContent 组合 header、body、footer 三个真实 Content，模态作用域和焦点由 ContentHost 管理。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let dialog = DialogContent::new("Confirm",
    Content::new(ComponentId(2), TextContent::new("Save changes?")),
    Content::new(ComponentId(3), TextContent::new("Review before continuing.")),
    Content::new(ComponentId(4), Button::new("Save")));
// 挂载后通过 host.show_dialog(dialog_id)? 打开。
```

## 展示与事件

`host.show_dialog(id)` 校验并进入模态作用域；`host.close_dialog()` 关闭。header/body/footer 子控件的动作沿原 Host 路径消费，确认按钮由业务决定保存是否成功，Dialog 不自动提交表单。

`is_open()` 与 `is_presented()` 分开，退出动画期间仍可有呈现；`modal_frame()` 读取实际几何，`needs_animation()` 表示原呈现需求。不要通过清除画面代替正常模态关闭与焦点恢复。

宿主拥有窗口键盘与文本服务，不能在对话框内部再启动一个输入循环。Dialog 不等于系统原生消息框；长 body 的滚动及业务取消策略需显式组合。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/adapters.rs`。现有消费路径：`examples/composition-consumer/tests/dialog_accessibility.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
