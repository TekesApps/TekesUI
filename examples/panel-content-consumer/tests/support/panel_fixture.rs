//! Shared original Issue #82 Panel/Stack/Text/Scroll consumer fixture.
use tekes_ui_controls::ScrollArea;
use tekes_ui_controls::composition::{
    Button, ComponentId, Content, ScrollContent, StackContent, TextContent, TextFieldContent,
    TextSubmitPolicy, TextViewport,
};
use tekes_ui_controls::extended::{Axis, Stack};

pub const ROOT: ComponentId = ComponentId(8200);
pub const BODY: ComponentId = ComponentId(8201);
pub const LABEL: ComponentId = ComponentId(8202);
pub const ACTION: ComponentId = ComponentId(8203);
pub const EDITOR: ComponentId = ComponentId(8204);
pub const SCROLL: ComponentId = ComponentId(8205);
pub const ROWS: ComponentId = ComponentId(8206);

pub fn composer() -> TextFieldContent {
    let mut composer = TextFieldContent::new("Composer", "draft 0123456789");
    composer.surface.input.multiline = true;
    // Changing input.multiline does not change the independently configured
    // ScrollArea axes. Retain horizontal Auto and enable the real vertical
    // viewport used by this Fill editor; never write an offset in the consumer.
    composer.surface.scroll.set_scroll_policy(
        tekes_ui_controls::ScrollPolicy::Auto,
        tekes_ui_controls::ScrollPolicy::Auto,
    );
    composer.viewport = TextViewport::Fill;
    composer.show_metadata = false;
    composer.submit_policy = TextSubmitPolicy::None;
    composer
}

pub fn row_ids() -> Vec<ComponentId> {
    (0..18).map(|i| ComponentId(8210 + i)).collect()
}

pub fn panel_body(composer: TextFieldContent, row_ids: &[ComponentId]) -> Content {
    let rows = Content::new(
        ROWS,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 2.,
            },
            children: row_ids
                .iter()
                .map(|id| {
                    Content::new(
                        *id,
                        TextContent::new(format!("阅读行 {id:?} — original text")),
                    )
                })
                .collect(),
            flexible_child: None,
        },
    );
    let mut scroll = ScrollContent::new(ScrollArea::default(), rows);
    scroll.viewport_height = Some(68.);
    let button = Content::new(ACTION, Button::new("原按钮")).preserving_focus();
    Content::new(
        BODY,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            children: vec![
                Content::new(
                    LABEL,
                    TextContent::new("原 Panel / Stack / Button / Composer / Scroll"),
                ),
                button,
                Content::new(EDITOR, composer),
                Content::new(SCROLL, scroll),
            ],
            // TextViewport::Fill uses remaining space after fixed siblings.
            flexible_child: Some(EDITOR),
        },
    )
}
