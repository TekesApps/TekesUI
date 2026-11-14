# Banner

BannerContent 是持续可见的页内消息，可声明是否允许关闭；它没有 Toast 的自动超时计时。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::{Banner, NoticeTone}};
let banner = BannerContent::new("Announcement", Banner {
    message: "Connection unavailable".into(), tone: NoticeTone::Warning,
    dismissible: true, visible: true,
}, ComponentId(2)).unwrap();
```

## 状态和组合

`model()` 只读查看 message、tone、dismissible、visible；借用不跨 Host 更新持有。`layout()` 返回原 ViewLayout，`set_metadata` 更新名称。

挂载后使用 `host.replace_banner(id, model)` 发布新的声明。构造中的 dismiss_id 绑定原关闭动作；禁用或不可关闭不应被旁路点击处理强行关闭。消息 body 的实际布局由原 BannerView 处理，父容器分配有限空间。

Banner 不执行重连、不自动重试，也不是任意交互富内容容器。业务错误的持续性和何时清除由应用决定；换成 Success 色不证明操作已验证成功。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/notice.rs`。现有消费路径：`examples/composition-consumer/tests/notice_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
