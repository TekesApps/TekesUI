//! A real public range adapter in the existing heterogeneous content tree.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Slider, Stack},
};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(8500);
pub const RANGE: ComponentId = ComponentId(8501);
pub const EDITOR: ComponentId = ComponentId(8502);
pub const ACTION: ComponentId = ComponentId(8503);
pub fn build() -> Result<ContentHost, CompositionError> {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(ComponentId(8504), TextContent::new("Volume")),
                Content::new(
                    RANGE,
                    SliderContent::new("Volume", Slider::new(50., 0., 100., 5.).unwrap()),
                ),
                Content::new(EDITOR, TextFieldContent::new("Draft", "Keep this draft")),
                Content::new(ACTION, ButtonContent::new("Continue")),
            ],
        },
    ))
}
