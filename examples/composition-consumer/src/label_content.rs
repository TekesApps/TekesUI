//! Readonly label policies on original production components; no business UI.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_text::{LabelOptions, LabelOverflow};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(9900);
pub const ACTION: ComponentId = ComponentId(9902);
pub const SOURCE: &str =
    "完整标题 English 👨‍👩‍👧‍👦 e\u{301} שלום العربية 123，需要保留完整原文和可访问性名称";
pub fn build() -> Result<ContentHost, CompositionError> {
    let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
    let children = vec![
        Content::new(ComponentId(9901), TextContent::new(SOURCE)).with_label_layout(options)?,
        Content::new(ACTION, Button::new(SOURCE)).with_label_layout(options)?,
        Content::new(
            ComponentId(9903),
            Button::new(Content::new(ComponentId(9904), TextContent::new(SOURCE))
                    .with_label_layout(options)?).with_accessible_name(SOURCE),
        ),
    ];
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children,
        },
    ))
}
