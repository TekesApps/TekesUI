# Link

LinkContent 把原 Link 模型与 LinkView 挂载到 Host。它发出导航意图，具体 URL、路由与打开策略属于消费者。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let link = Content::new(ComponentId(2), LinkContent::new("Open documentation"));
```

## 输入和状态

`label` 是可见及语义名称，不能为空；公开 `model` 和 `view` 是原组件状态。Enter 激活，Space 保留给滚动。需要编辑器旁辅助操作保留编辑焦点时，可按现有 `Content::preserving_focus` 合同组合。

Host 动作启用后消费 Activate；组件本身不会联网或打开浏览器。Theme 负责文字、下划线和焦点外观。Link 不承载富正文中的任意范围链接布局；富文档链接使用 RichContent 自己的 action/映射路径。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/link.rs`。现有消费路径：`examples/composition-consumer/src/link_content.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
