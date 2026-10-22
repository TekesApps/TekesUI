//! Standard CompoundButton, with a decorative toolkit SVG and original Text.
use crate::text::Text;
use anyhow::Result;
use std::collections::HashMap;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Component, ComponentId, CompoundButton, Content, ContentHost, ContentTheme, StackContent,
        TextContent,
    },
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, ImageQuad, Rect, Scene, Size};
use tekes_ui_tokens::proposal;

#[derive(Debug, Default)]
struct BackIcon {
    image: Option<ImageId>,
    images: HashMap<(String, u32), ImageId>,
}
impl Component for BackIcon {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, _available: Size) -> MeasuredChild {
        let side = text.resolved_font_size(proposal::TextRole::UiBody);
        MeasuredChild::new(Size::new(side, side))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        _theme: ContentTheme,
    ) {
        if let Some(image) = self.image {
            scene.push_clip(clip);
            scene.push_image(ImageQuad {
                rect: frame,
                image,
                opacity: 1.,
            });
            scene.pop_clip();
        }
    }
}
pub(super) fn button() -> Result<ContentHost> {
    let row = Content::new(
        ComponentId(2),
        StackContent {
            model: Stack {
                axis: Axis::Horizontal,
                gap: proposal::metric("spacing", "space-tight").unwrap(),
            },
            children: vec![
                Content::new(ComponentId(3), BackIcon::default()),
                Content::new(ComponentId(4), TextContent::new("Back")),
            ],
            flexible_child: None,
        },
    );
    let button =
        CompoundButton::new("Back", row).map_err(|e| anyhow::anyhow!("Settings Back: {e:?}"))?;
    let mut host = ContentHost::new(Content::new(ComponentId(1), button).with_help("返回主窗口"))
        .map_err(|e| anyhow::anyhow!("Settings Back: {e:?}"))?;
    host.set_action_events(true);
    Ok(host)
}
pub(crate) fn prepare(
    host: &mut ContentHost,
    renderer: &mut Renderer,
    text: &Text,
    scale: f32,
) -> Result<()> {
    let icon = host
        .get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<BackIcon>()
        .unwrap();
    icon.image = Some(crate::window_toolbar::icon_image(
        &mut icon.images,
        renderer,
        "tekes-back",
        scale * text.type_scale(),
    )?);
    host.get_mut(ComponentId(2))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .model
        .gap = proposal::metric("spacing", "space-tight").unwrap() * text.type_scale();
    Ok(())
}
