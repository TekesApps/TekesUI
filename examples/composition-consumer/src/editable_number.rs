//! Consumer uses only the shared public numeric Content interface.
use tekes_ui_controls::{
    TextSurfaceAlignment,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_tokens::Spacing;
pub const ROOT: ComponentId = ComponentId(9400);
pub const PERCENT: ComponentId = ComponentId(9401);
pub const INTEGER: ComponentId = ComponentId(9402);
pub const DECIMAL: ComponentId = ComponentId(9403);
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut percent = NumberFieldContent::new(
        "Percentage",
        100.,
        50.,
        300.,
        10.,
        DecimalNumberFormat { suffix: "%".into() },
    )
    .unwrap();
    percent.description =
        Some("Enter a value with or without the percent sign. Up and Down adjust by 10".into());
    percent.set_input_digits(3);
    percent.set_alignment(TextSurfaceAlignment::Trailing);
    let integer =
        NumberFieldContent::new("Quantity", 3., 0., 10., 1., DecimalNumberFormat::default())
            .unwrap();
    let mut decimal = NumberFieldContent::new(
        "Decimal",
        1.25,
        0.,
        5.,
        0.25,
        DecimalNumberFormat::default(),
    )
    .unwrap();
    decimal.set_borderless(true);
    decimal.set_input_digits(4);
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(PERCENT, percent)
                    .with_help("Percentage: 50%–300%. Enter commits; Escape cancels."),
                Content::new(INTEGER, integer).with_help("Quantity"),
                Content::new(DECIMAL, decimal).with_help("Borderless decimal; step 0.25"),
                Content::new(
                    ComponentId(9404),
                    ButtonContent::new("After numeric fields"),
                ),
            ],
        },
    ))
}
