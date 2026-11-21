//! Caller-owned composition of the existing bounded NumberField View.
//! Two ordinary semantic/focus nodes delegate to one original model/View;
//! they do not paint another stepper or implement another activation machine.
use tekes_ui_controls::{NumberField, composition::*};
use tekes_ui_tokens::{Spacing, proposal};

pub const ROOT: ComponentId = ComponentId(9200);
pub const QUANTITY: ComponentId = ComponentId(9201);
pub const DECREMENT: ComponentId = ComponentId(9202);
pub const INCREMENT: ComponentId = ComponentId(9203);
pub const LIMIT: ComponentId = ComponentId(9211);
pub const LIMIT_DECREMENT: ComponentId = ComponentId(9212);
pub const LIMIT_INCREMENT: ComponentId = ComponentId(9213);
pub const DISABLED: ComponentId = ComponentId(9221);
pub const AFTER: ComponentId = ComponentId(9230);

fn number(
    id: ComponentId,
    decrement: ComponentId,
    increment: ComponentId,
    name: &str,
    value: f64,
    disabled: bool,
) -> Content {
    Content::new(
        id,
        StepperContent::with_step_ids(
            name,
            NumberField::new(value, 0., 4., 1.).unwrap(),
            decrement,
            increment,
        )
        .with_disabled(disabled),
    )
}
pub fn build() -> Result<ContentHost, CompositionError> {
    ContentHost::new(Content::new(
        ROOT,
        StackContent::vertical(
            Spacing::Regular.value(),
            vec![
                Content::new(
                    ComponentId(9209),
                    TextContent::new("Quantity · bounded stepper")
                        .with_role(proposal::TextRole::UiBody),
                ),
                number(QUANTITY, DECREMENT, INCREMENT, "Quantity", 2., false),
                Content::new(
                    ComponentId(9219),
                    TextContent::new("At upper limit").with_role(proposal::TextRole::UiBody),
                ),
                number(
                    LIMIT,
                    LIMIT_DECREMENT,
                    LIMIT_INCREMENT,
                    "At upper limit",
                    4.,
                    false,
                ),
                Content::new(
                    ComponentId(9229),
                    TextContent::new("Disabled quantity").with_role(proposal::TextRole::UiBody),
                ),
                number(
                    DISABLED,
                    ComponentId(9222),
                    ComponentId(9223),
                    "Disabled quantity",
                    2.,
                    true,
                ),
                Content::new(AFTER, Button::new("After number")),
            ],
        ),
    ))
}
/// Caller changes its authoritative model, without painting as a side effect.
pub fn set_value(host: &mut ContentHost, id: ComponentId, value: f64) -> bool {
    host.get_mut(id)
        .unwrap()
        .component_mut::<StepperContent>()
        .unwrap()
        .set_value(value)
}
pub fn value(host: &ContentHost, id: ComponentId) -> f64 {
    host.get(id)
        .unwrap()
        .component::<StepperContent>()
        .unwrap()
        .value()
}
