# DataTable

DataTableContent 通过 KeyedTableSnapshot 挂载稳定行列身份的数据表。底层 KeyedDataTable、DataTableView 与 Host 共同管理布局、排序意图、选择和输入。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::*};
fn mount_table(snapshot: &KeyedTableSnapshot, displays: &mut TableDisplays)
    -> Result<DataTableContent, TableContentError>
{
    DataTableContent::new("Documents", snapshot, displays,
        TableBudget { max_units: 100_000, max_utf8_bytes: 100_000_000 })
}
```

## 快照、事件与职责

snapshot 包含 dataset、columns、rows、display_rows；TableDisplays 给表头和单元格指定 Keep/Replace 显示与附件。实体 key 与 row wrapper ComponentId 分开，重排不能换掉连续实体的 wrapper。

`model()`、`selected_key()`、`sort_key_state()` 读取状态。挂载更新用 `host.replace_table_snapshot(id, snapshot, displays, reading)`；需要阅读保持时携带 TableReadingUpdate。TableIntent 区分 Header/Row target，kind 包含选择、激活、排序；`intent_is_current` 核对捕获身份，不按新快照索引解释旧事件。

被动装饰及明确附件槽服从原事务兼容合同；不能把任意交互单元格当作已支持的电子表格。父 ScrollContent 提供滚动，业务层提供数据和显示顺序。此处不承诺无限数据、远程排序、公式或通用单元格编辑；示例预算不是性能保证。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/datatable.rs`。现有消费路径：`examples/composition-consumer/src/datatable_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
