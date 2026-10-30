//! Independent finite Checkbox consumer; original View remains the reference.
use tekes_ui_controls::{CheckState, composition::*};
pub const CHECKBOX: ComponentId = ComponentId(10810);
pub fn build(label: &str, checked: CheckState) -> Result<ContentHost, CompositionError> {
    let mut host = ContentHost::new(Content::new(CHECKBOX, CheckboxContent::new(label, checked)))?;
    host.set_action_events(true);
    Ok(host)
}
