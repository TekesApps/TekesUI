//! A real public range adapter in the existing heterogeneous content tree.
use tekes_ui_controls::{composition::*, extended::Slider};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(8500);
pub const RANGE: ComponentId = ComponentId(8501);
pub const EDITOR: ComponentId = ComponentId(8502);
pub const ACTION: ComponentId = ComponentId(8503);
pub fn build() -> Result<ContentHost, CompositionError> {
    ContentHost::new(fixture_content(50.))
}
/// The same public subtree can be mounted by a receiving application Host.
pub fn fixture_content(value: f64) -> Content {
    Content::new(
        ROOT,
        StackContent::vertical(
            Spacing::Regular.value(),
            vec![
                Content::new(ComponentId(8504), TextContent::new("Volume")),
                Content::new(
                    RANGE,
                    SliderContent::new("Volume", Slider::new(value, 0., 100., 5.).unwrap()),
                ),
                Content::new(EDITOR, TextFieldContent::new("Draft", "Keep this draft")),
                Content::new(ACTION, Button::new("Continue")),
            ],
        ),
    )
}
