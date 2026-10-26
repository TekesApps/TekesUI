//! Original scrollable List rows with mutable help, not a second row/tooltip view.
use tekes_ui_controls::{
    ScrollArea,
    composition::*,
    extended::{Axis, Stack},
};
pub const ROOT: ComponentId = ComponentId(16000);
pub const SCROLL: ComponentId = ComponentId(16001);
pub const LIST: ComponentId = ComponentId(16002);
pub const FIRST: ComponentId = ComponentId(16003);
pub const COMPOSER: ComponentId = ComponentId(16020);
pub fn build() -> Result<ContentHost, CompositionError> {
    let rows = (0..8)
        .map(|i| {
            Content::new(
                ComponentId(FIRST.0 + i),
                Button::new(format!("Session {} · 会话", i + 1)),
            )
            .with_help(format!("query A · 第{}项搜索摘要 /folder/{}", i + 1, i + 1))
        })
        .collect();
    let mut scroll = ScrollContent::new(
        ScrollArea::default(),
        Content::new(LIST, ListContent::new("Search results", rows)),
    );
    scroll.viewport_height = Some(120.);
    let mut h = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: tekes_ui_tokens::Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(SCROLL, scroll),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "Keep A😀B")),
            ],
        },
    ))?;
    h.set_action_events(true);
    Ok(h)
}
