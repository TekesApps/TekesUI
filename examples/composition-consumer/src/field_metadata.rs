//! Existing form metadata and editor adapter, without a caller-owned view fork.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;

pub const ROOT: ComponentId = ComponentId(9400);
pub const NORMAL: ComponentId = ComponentId(9401);
pub const ERROR: ComponentId = ComponentId(9402);
pub const DISABLED: ComponentId = ComponentId(9403);

pub fn build() -> Result<ContentHost, CompositionError> {
    let normal = TextFieldContent::new(
        "Required workspace display name with a wrapping label",
        "Workspace",
    )
    .with_required(true)
    .with_help("Use a name that other people can recognize.");
    let error = TextFieldContent::new("Required project name needing correction", "Existing")
        .with_required(true)
        .with_error(Some(
            "This name already exists. Choose another name.".into(),
        ));
    let disabled = TextFieldContent::new("Disabled required workspace name", "Preserved")
        .with_required(true)
        .with_help("Unavailable while the workspace is locked.")
        .with_disabled(true);
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(NORMAL, normal),
                Content::new(ERROR, error),
                Content::new(DISABLED, disabled),
            ],
        },
    ))
}
