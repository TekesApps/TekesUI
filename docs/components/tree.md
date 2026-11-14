# Tree

TreeContent 是稳定节点 key 的有限树。原 Tree/TreeView 负责展开、选择和行输入；消费者负责节点关系和业务数据。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::{KeyedTreeNode, TreeDatasetKey, TreeNodeKey}};
let tree = TreeContent::new("Entities", TreeDatasetKey(1), vec![TreeContentNode {
    row_id: ComponentId(2),
    node: KeyedTreeNode { key: TreeNodeKey(1), parent: None, label: "Root".into(), disabled: false, initially_expanded: true },
    display: TreeDisplayUpdate::Keep,
}]).unwrap();
```

## 更新和意图

TreeContentNode 分开实体 key、父 key、row_id 和装饰内容。连续存在的节点应保留 row_id；display 的 Keep、DefaultLabel、Replace 表示明确的显示更新。挂载后使用 `host.replace_tree_nodes(...)` 执行结构事务。

`selected_key()` 读取选择，`select_key` 与 `set_expanded_key` 提供状态操作。Host 的 TreeIntent 包含 dataset、key、kind、selected_key；kind 区分选择、激活和展开。`intent_is_current` 可用于延迟处理时判断声明有效性。

树行装饰只允许被动内容，不能把独立编辑器嵌进去同时抢行输入。展开不自动从服务器加载子项，滚动属于父视口；无效父关系、身份变化及不支持的装饰须按 Result 处理，不忽略错误继续展示旧数据为成功。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/tree.rs`。现有消费路径：`examples/composition-consumer/src/tree_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
