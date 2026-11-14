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

定义：`crates/tekes-ui-controls/src/composition/datatable.rs`。现有消费路径：`examples/composition-consumer/src/datatable_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
