# Chart

当前 Chart 正式内容入口 ChartContent 是单系列柱状图，复用 BarChart 与 ChartView，不代表通用图表库。

## 短用法

```rust
use tekes_ui_controls::{ChartDatum, composition::ChartContent};
let chart = ChartContent::try_new("Sales", vec![
    ChartDatum::new("A", 12.), ChartDatum::new("B", 24.),
]).unwrap().preferred_height(240.);
```

## 数据、选择和布局

`try_new` 拒绝空名称和非有限数值。`update_data` 返回 Result 并使旧布局失效；`data()`、`selected_index()`、`selected_datum()` 读取当前原模型。不能因数据已更新就把上一帧坐标当作当前柱位置。

指针、方向键及可访问行为走 ChartContent/Host 的原路径，选择读取不代表业务数据改变。此入口没有独立 Chart 数据 payload；消费者应在正式输入后读取选择，不能给普通 Activate 虚构字段。

preferred_height 是偏好，最终宽高由父分配。宿主提供文字 shaping、Theme 与裁剪，图表不自行滚动或请求网络数据。不支持由此推定的折线、散点、多系列或时间序列图；完整尺度与几何应看 chart.rs。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/chart_content.rs`。现有消费路径：`examples/composition-consumer/tests/chart_content_host.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
