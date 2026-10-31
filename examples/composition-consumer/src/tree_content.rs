//! Independent public finite-tree consumer: sibling Text/Composer, decorative
//! Text + Icon row, original Host input and controlled atomic snapshot updates.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, KeyedTreeNode, Stack, TreeDatasetKey, TreeNodeKey},
};
pub const TREE: ComponentId = ComponentId(10);
pub const COMPOSER: ComponentId = ComponentId(20);
pub fn snapshot(reordered: bool) -> Vec<TreeContentNode> {
    let entries = if reordered {
        vec![(3, None), (1, None), (2, Some(1))]
    } else {
        vec![(1, None), (2, Some(1)), (3, None)]
    };
    entries
        .into_iter()
        .map(|(k, p)| TreeContentNode {
            row_id: ComponentId(100 + k),
            node: KeyedTreeNode {
                key: TreeNodeKey(k),
                parent: p.map(TreeNodeKey),
                label: format!("Entity {k}"),
                disabled: false,
                initially_expanded: true,
            },
            display: TreeDisplayUpdate::Keep,
        })
        .collect()
}
pub fn build() -> Result<ContentHost, TreeContentError> {
    let mut nodes = snapshot(false);
    nodes[0].display = TreeDisplayUpdate::Replace(Content::new(
        ComponentId(200),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: 6.,
            },
            flexible_child: Some(ComponentId(202)),
            children: vec![
                Content::new(
                    ComponentId(201),
                    IconContent::new("tekes-settings").expect("canonical icon"),
                ),
                Content::new(ComponentId(202), TextContent::new("Public passive row")),
            ],
        },
    ));
    ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(2), TextContent::new("Before tree")),
                Content::new(
                    TREE,
                    TreeContent::new("Entities", TreeDatasetKey(1), nodes)?,
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(ComponentId(3), TextContent::new("After tree")),
            ],
        },
    ))
    .map_err(Into::into)
}
pub fn reorder(host: &mut ContentHost) -> Result<(), TreeContentError> {
    host.replace_tree_nodes(TREE, TreeDatasetKey(1), snapshot(true))
}
