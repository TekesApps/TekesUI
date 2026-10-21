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
    let mut normal = TextFieldContent::new(
        "Required workspace display name with a wrapping label",
        "Workspace",
    );
    normal.field.required = true;
    normal.field.help = "Use a name that other people can recognize.".into();
    let mut error = TextFieldContent::new("Required project name needing correction", "Existing");
    error.field.required = true;
    error.field.error = Some("This name already exists. Choose another name.".into());
    let mut disabled = TextFieldContent::new("Disabled required workspace name", "Preserved");
    disabled.field.required = true;
    disabled.field.help = "Unavailable while the workspace is locked.".into();
    disabled.surface.input.disabled = true;
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
