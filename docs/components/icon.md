# Icon

IconContent 使用规范图标名，复用原 IconView 和宿主图标渲染服务；它是图像内容，不是可点击按钮。

## 短用法

```rust
use tekes_ui_controls::composition::IconContent;
let icon = IconContent::new("tekes-settings").unwrap();
```

## 属性和资源

`set_glyph` 校验规范名称并返回是否变更；`glyph()` 读取当前 Glyph。`with_accessible_name` 或 `set_accessible_name` 设置语义名称，装饰用途可不提供独立名称；`set_status_tone`、`set_disabled`、`set_visible` 更新呈现。

`set_indeterminate` 声明不确定状态，帧推进与活动状态仍由原 Host 生命周期管理，不自建 timer。`layout()` 读取原布局。宿主需要实际 IconRenderer；WithIconRenderer 提供渲染器组合，创建图标不代表 GPU 资源已准备。

要点击时把 IconContent 作为 Button 的装饰内容并给按钮命名，不给图标重复加第二个动作。未知图标名称是错误，不应静默换成不相关图标。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/icon.rs`。现有消费路径：`examples/composition-consumer/src/icon_content.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
