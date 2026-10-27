//! Independent public section layout. Business labels/constraints stay here;
//! original SDK Content owns the separator painter, palette and passive policy.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(54000);
pub const FIRST: ComponentId = ComponentId(54001);
pub const DIVIDER: ComponentId = ComponentId(54002);
pub const LAST: ComponentId = ComponentId(54003);
pub fn build(axis: Axis) -> Result<ContentHost, CompositionError> {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: if axis == Axis::Horizontal {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
                gap: 4.,
            },
            flexible_child: None,
            children: vec![
                Content::new(FIRST, TextContent::new("Input 中文")),
                Content::new(DIVIDER, SeparatorContent::new(axis)),
                Content::new(LAST, TextContent::new("Output 中文")),
            ],
        },
    ))
}
