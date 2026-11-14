# Button

Button 表示一次操作。文字、图片、图文和装饰内容共用正式按钮交互，不需要消费者自己处理按住、取消和重复激活。

## 最小组合

```rust
use tekes_ui_controls::{Button, composition::{ComponentId, Content, ContentHost}};

let mut host = ContentHost::new(Content::new(ComponentId(4), Button::new("Apply"))).unwrap();
host.set_action_events(true);
```

正式公共类型由 `tekes_ui_controls::Button` 导出；组合与资源能力的完整消费例见 `examples/composition-consumer/tests/unified_button.rs`。

## 属性与状态

- `new(contents)` 创建按钮；`set_contents(...)` 更新内容。
- `with_accessible_name(...)` 为只有图片或装饰内容的按钮提供明确名称。
- `variant` 表示操作的视觉语义；普通按钮默认 Quiet。
- `button.state` 中的交互状态由原按钮路由维护；消费者不能在绘制时伪造 hover 或 pressed。
- `pointer_activation_phase` 区分按下激活与释放激活，默认 Release。
- `image_position` 和 `content_alignment` 控制内容排列；几何与命中仍使用原布局结果。

## 事件与取消

启用动作记录后，在 Host 正式输入之后读取 `drain_action_records()`，按稳定 `target` 和 `ContentActionKind::Activate` 分发业务操作。
程序化属性更新、焦点移动和 Inspector 读取不是激活。不要再给同一按钮叠加第二套点击处理而执行两次。
按住后拖出、失活、禁用或卸载需要经过原取消链；事件与其挂载身份保持绑定。

## 组合和 token

尺寸、内容间距、文字与状态色来自共享 Theme/Surface 路径。Toolbar 的紧凑图标表现属于正式工具栏组合合同，不应把图标截图当成普通按钮的统一尺寸规则。
运行时 Inspector 应报告这个实例实际解析的值；本页不硬编码一套看起来相似的色值。

## 限制

图像资源由宿主注册和准备。创建 Button 不表示图片资源已上传，也不证明原生、无障碍或性能验收完成。
