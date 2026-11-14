# Grid

Grid 提供固定列数的网格；正式内容组合用 `GridContent`，低层模型和绘制入口是 `extended::Grid`、`layout_view::GridView`。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::Grid};
let grid = GridContent {
    model: Grid::new(2),
    children: vec![Content::new(ComponentId(2), TextContent::new("Cell"))],
};
```

## 状态、布局与限制

`columns` 控制列数，`gap_x`、`gap_y` 控制间隔，子项按声明顺序排列。GridContent 将测量与分配交给原布局路径，不要求消费者给每个格子重复计算坐标。

输入、状态和动作属于各子组件；Grid 没有选中单元格或表格编辑协议。行可能超出外层高度，需由父 ScrollContent 裁切和滚动。需要稳定行列 key、排序和行选择时用 DataTable，而不是把 Grid 当作数据表。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition.rs`。现有消费路径：`examples/composition-consumer/tests/grid_theme.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
