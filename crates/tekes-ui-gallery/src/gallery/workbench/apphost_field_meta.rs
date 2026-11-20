//! Original emphasized Field metadata and state fixtures using shared TextFieldContent.
use super::StoryState;
use tekes_ui_controls::{composition::*, feedback_view::FieldPresentation};
pub(super) fn matrix(state: &StoryState) -> Content {
    let mut children = Vec::new();
    for (i, (name, mut control)) in super::apphost_basic::standard_states()
        .into_iter()
        .filter(|(name, _)| !matches!(*name, "hover" | "pressed" | "loading"))
        .enumerate()
    {
        control.error = name == "error + focus";
        let mut field = TextFieldContent::new("Workspace name", "Studio")
            .with_presentation(FieldPresentation::emphasized_form())
            .with_required(state.field.required)
            .with_help("Use a clear name for your workspace.")
            .with_error(control.error.then(|| "A name is required.".into()))
            .with_disabled(control.disabled);
        field.set_focus(control.focused, control.focus_visible);
        children.push(StackContent::labeled_specimen(
            ComponentId(120150 + i as u64),
            ComponentId(120100 + i as u64 * 2),
            name,
            Content::new(ComponentId(120101 + i as u64 * 2), field),
        ));
    }
    StackContent::panel_sections(ComponentId(120099), children)
}
