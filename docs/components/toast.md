# Toast

ToastContent 是限时通知，保持原 Toast/ToastView 与关闭动作身份。计时来自宿主明确的单调 motion context。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::{Toast, NoticeTone}};
let toast = ToastContent::new("Notice",
    Toast::new("Saved", NoticeTone::Success, 3000), ComponentId(2)).unwrap();
```

## 生命周期和事件

`model()` 返回只读借用，`layout()` 读取已有几何，`set_metadata` 更新语义名称。不要持有 model guard 跨 Host 更新、布局或回调。

挂载后用 Host 的 `show_toast(id, message, tone, duration_ms)`、`set_toast_visible`、`set_toast_paused`、`dismiss_toast` 操作原 owner。关闭按钮使用构造时的 dismiss_id，消费正式动作即可，不给每次绘制都记一次关闭。计时、暂停和活动状态沿原生命周期，缺失时钟会报错，不能另起 wall-clock timer 掩盖它。

父布局提供有限通知空间与锚点安排，组件不提供系统通知中心或通知队列。显示 Saved 文案必须基于消费者真实保存结果，计时结束不是业务成功证据。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/notice.rs`。现有消费路径：`examples/composition-consumer/tests/notice_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
