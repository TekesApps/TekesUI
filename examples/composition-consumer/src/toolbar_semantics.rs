//! Independent original ToolbarControl + editor consumer; no private Gallery API.
use tekes_ui_controls::{
    ToolbarAction, ToolbarControl,
    composition::{
        ComponentId, CompositionError, Content, ContentHost, ListContent, TextFieldContent,
    },
};
pub const ROOT: ComponentId = ComponentId(19300);
pub const TOOLBAR: ComponentId = ComponentId(19301);
pub const EDITOR: ComponentId = ComponentId(19302);
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut toolbar = ToolbarControl::window();
    toolbar.set_actions(
        [
            (10, "First action", true),
            (20, "Unavailable action", false),
            (30, "Selected action", true),
            (40, "Additional action A", true),
            (50, "Additional action B", true),
            (60, "Last action", true),
        ]
        .into_iter()
        .map(|(id, title, enabled)| ToolbarAction {
            id,
            title: title.into(),
            enabled,
        })
        .collect(),
    );
    toolbar.set_selected_ids(&[30]);
    let mut host = ContentHost::new(Content::new(
        ROOT,
        ListContent::new(
            "Toolbar consumer",
            vec![
                Content::new(TOOLBAR, toolbar),
                Content::new(
                    EDITOR,
                    TextFieldContent::new("Message", "Preserve this draft"),
                ),
            ],
        ),
    ))?;
    host.set_action_events(true);
    Ok(host)
}
