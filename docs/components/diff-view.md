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

定义：`crates/tekes-ui-controls/src/extended.rs`。现有消费路径：`examples/composition-consumer/src/rich_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
