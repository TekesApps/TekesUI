//! Independent public finite-tree consumer: sibling Text/Composer, decorative
//! Text + Icon row, original Host input and controlled atomic snapshot updates.
use tekes_ui_controls::{
    composition::*,
    extended::{KeyedTreeNode, TreeDatasetKey, TreeNodeKey},
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
        IconTextContent::icon_label(
            ComponentId(201),
            "tekes-settings",
            ComponentId(202),
            "Public passive row",
        )
        .expect("canonical icon"),
    ));
    ContentHost::new(Content::new(
        ComponentId(1),
        StackContent::vertical(
            8.,
            vec![
                Content::new(ComponentId(2), TextContent::new("Before tree")),
                Content::new(
                    TREE,
                    TreeContent::new("Entities", TreeDatasetKey(1), nodes)?,
                ),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(ComponentId(3), TextContent::new("After tree")),
            ],
        ),
    ))
    .map_err(Into::into)
}
pub fn reorder(host: &mut ContentHost) -> Result<(), TreeContentError> {
    host.replace_tree_nodes(TREE, TreeDatasetKey(1), snapshot(true))
}
