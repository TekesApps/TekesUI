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

定义：`crates/tekes-ui-controls/src/composition/panel.rs`。现有消费路径：`examples/composition-consumer/tests/panel_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
