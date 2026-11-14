# Surface

Surface 的绘制入口是 `feedback_view::SurfaceView`；需要包裹实际组件并同步圆角几何时使用 `PanelContent`。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let child = Content::new(ComponentId(2), TextContent::new("Panel body"));
let panel = PanelContent::new(child).unwrap();
```

## 几何与子树

`set_canvas` 选择 canvas 语义；`set_padding` 接受可选 Insets 并返回校验结果。`child()` 读取子内容，`geometry()` 返回已生成的 PanelGeometry，未布局时可为空。

Panel 负责背景、padding 与子树几何合同；子项保留输入和状态，Panel 本身不是一个按钮。圆角区域涉及绘制、命中与 reveal 的统一几何，不能只画圆角后继续使用矩形交互边界。

构造和 padding 的 Result 必须处理；自定义子树需要满足它声明的几何能力。Panel 不自动生成阴影、窗口、滚动或业务事件，也不修改子 TextField 的 Field presentation。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/panel.rs`。现有消费路径：`examples/composition-consumer/tests/panel_content.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
