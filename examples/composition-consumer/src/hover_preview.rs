//! Two-layer transcript preview from original Content in the original editor host.
use std::num::NonZeroUsize;
use tekes_ui_controls::{
    TextSurface,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_text::{LabelOptions, LabelOverflow};
pub const ROOT: ComponentId = ComponentId(24600);
pub const TICK_A: ComponentId = ComponentId(24601);
pub const TICK_B: ComponentId = ComponentId(24602);
pub const COMPOSER: ComponentId = ComponentId(24603);
pub const PREVIEW: ComponentId = ComponentId(24604);
pub const BODY: ComponentId = ComponentId(24605);
pub const INPUT: ComponentId = ComponentId(24606);
pub const OUTPUT: ComponentId = ComponentId(24607);
pub fn content(input: &str, output: &str) -> Content {
    let label = |id, source: &str, lines| {
        Content::new(id, TextContent::new(source))
            .with_label_layout(LabelOptions {
                max_lines: NonZeroUsize::new(lines),
                overflow: LabelOverflow::TailEllipsis,
            })
            .unwrap()
    };
    Content::new(
        BODY,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![label(INPUT, input, 2), label(OUTPUT, output, 4)],
        },
    )
}
#[derive(Debug)]
pub struct Transcript {
    pub host: ContentHost,
    pub version: HoverPreviewVersion,
}
impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}
impl Transcript {
    pub fn new() -> Self {
        let mut composer = TextFieldContent::new("Composer", "");
        composer.surface = TextSurface::new("Original composer draft", true, false);
        let mut preview =
            HoverPreviewContent::new("Turn preview", content("Input A", "Output A")).unwrap();
        preview.set_max_content_width(Some(220.));
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 8.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(TICK_A, Button::new("Turn A")),
                    Content::new(TICK_B, Button::new("Turn B")),
                    Content::new(COMPOSER, composer),
                    Content::new(PREVIEW, preview),
                ],
            },
        ))
        .unwrap();
        let version = host.bind_hover_preview(TICK_A, PREVIEW).unwrap();
        Self { host, version }
    }
    pub fn update(&mut self, input: &str, output: &str) -> Result<(), HoverPreviewError> {
        self.version = self
            .host
            .update_hover_preview(self.version, content(input, output))?;
        Ok(())
    }
    pub fn bind_b(&mut self) -> Result<(), HoverPreviewError> {
        self.version = self.host.bind_hover_preview(TICK_B, PREVIEW)?;
        Ok(())
    }
}
