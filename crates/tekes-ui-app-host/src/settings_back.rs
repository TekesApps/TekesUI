//! Shared image-and-text Button; the toolkit owns content geometry and input.
use crate::text::Text;
use anyhow::Result;
use std::collections::HashMap;
use tekes_ui_controls::{
    Button,
    composition::{ComponentId, Content, ContentHost},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::ImageId;

pub(super) fn button() -> Result<ContentHost> {
    // Register the actual image in prepare before the first native paint.
    let button = Button::new((ImageId(0), "Back")).with_help("返回主窗口");
    let mut host = ContentHost::new(Content::new(ComponentId(1), button))
        .map_err(|e| anyhow::anyhow!("Settings Back: {e:?}"))?;
    host.set_action_events(true);
    Ok(host)
}
pub(crate) fn prepare(
    host: &mut ContentHost,
    renderer: &mut Renderer,
    text: &Text,
    scale: f32,
    images: &mut HashMap<(String, u32), ImageId>,
) -> Result<()> {
    let image = crate::window_toolbar::icon_image(
        images,
        renderer,
        "tekes-back",
        scale * text.type_scale(),
    )?;
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Button>()
        .unwrap()
        .set_contents((image, "Back"));
    Ok(())
}
