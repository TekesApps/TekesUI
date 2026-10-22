//! Public-only toolbar composition; business text does not become a button.
use tekes_ui_controls::composition::*;
pub const BAR: ComponentId = ComponentId(19500);
pub const NAV: ComponentId = ComponentId(19501);
pub const TITLE: ComponentId = ComponentId(19502);
pub const STATUS_GROUP: ComponentId = ComponentId(19503);
pub const STATUS: ComponentId = ComponentId(19504);
pub const ACTION_GROUP: ComponentId = ComponentId(19505);
pub const SAVE: ComponentId = ComponentId(19506);
pub const OTHER: ComponentId = ComponentId(19507);
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut host = ContentHost::new(Content::new(
        BAR,
        ToolbarContent::new(vec![
            Content::new(
                ACTION_GROUP,
                ToolbarGroup::new(
                    ToolbarPlacement::PrimaryAction,
                    vec![
                        Content::new(SAVE, ButtonContent::new("Save"))
                            .with_help("Save this document"),
                        Content::new(OTHER, ButtonContent::new("Publish")),
                    ],
                ),
            ),
            Content::new(
                STATUS_GROUP,
                ToolbarGroup::new(
                    ToolbarPlacement::Status,
                    vec![Content::new(STATUS, TextContent::new("Up to date"))],
                ),
            ),
            Content::new(
                NAV,
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![Content::new(TITLE, TextContent::new("Document B"))],
                ),
            ),
        ]),
    ))?;
    host.set_action_events(true);
    Ok(host)
}
