//! Consumer declares data and command identity; SDK owns all Link interaction.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(20900);
pub const COMPOSER: ComponentId = ComponentId(20901);
pub const LINK: ComponentId = ComponentId(20902);
pub fn build() -> Result<ContentHost, CompositionError> {
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            flexible_child: None,
            children: vec![
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft😀")),
                Content::new(LINK, LinkContent::new("Read documentation · 阅读"))
                    .preserving_focus(),
            ],
        },
    ))?;
    host.set_action_events(true);
    Ok(host)
}
/// A real product opens its document here, once, using the originating record.
pub fn process_actions(host: &mut ContentHost) -> Vec<ContentActionRecord> {
    host.drain_action_records()
}
