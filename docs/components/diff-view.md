# DiffView

DiffView::between 生成确定的行级 diff；DiffViewRuntime 是原直接绘制入口，可选择阅读使用 RichDocument::from_diff 与 RichContent。

## 短用法

```rust
use tekes_ui_controls::{composition::{RichContent, RichDocument}, extended::DiffView};
let diff = DiffView::between("old\nkeep", "new\nkeep");
let content = RichContent::new("diff-1", 1, RichDocument::from_diff(&diff)).unwrap();
```

## 数据和阅读

`DiffView.lines` 中每个 DiffLine 含 kind（Equal、Added、Removed）和 text。转换成 RichDocument 后使用共享文字选择、来源映射和版本化更新；颜色与前缀来自原文档/绘制路径，消费者不再复制一套行坐标来命中。

算法是有界 LCS；输入规模超过源码预算时回退为删除块加新增块，不能声称始终给出最小编辑序列。没有行内字符高亮、合并冲突解决或 patch 应用行为。

父级负责滚动，业务负责 before/after 来源与版本；显示差异不等于文件已修改。需要接受变更时另行组合明确动作，DiffView 不自动写文件或调用 Git。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/extended.rs`。现有消费路径：`examples/composition-consumer/src/rich_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
