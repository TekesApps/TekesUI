# Tree

Use TreeContent for a finite hierarchy with stable node keys, expansion and selection.

## Creating the content

```rust
use tekes_ui_controls::{composition::*, extended::{KeyedTreeNode, TreeDatasetKey, TreeNodeKey}};
let tree = TreeContent::new("Entities", TreeDatasetKey(1), vec![TreeContentNode {
    row_id: ComponentId(2),
    node: KeyedTreeNode { key: TreeNodeKey(1), parent: None, label: "Root".into(), disabled: false, initially_expanded: true },
    display: TreeDisplayUpdate::Keep,
}]).unwrap();
```

## State and events

TreeContentNode separates key, parent key, row_id and display content. Keep row_id for continuing entities. Keep, DefaultLabel and Replace describe display updates. Use host.replace_tree_nodes for mounted structural changes. selected_key, select_key and set_expanded_key access selection and expansion.

## Composition and limitations

TreeIntent captures dataset, key, kind and selected_key. It distinguishes selection, activation and expansion; intent_is_current checks delayed use. Row decorations are passive. A parent supplies scrolling and the application supplies children; expanding does not fetch data. Handle invalid parent relations and unsupported decorations as errors.

## Related APIs

[List](list.md)
