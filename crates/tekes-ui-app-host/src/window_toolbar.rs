//! Window-owned toolbar contribution; the shared ToolbarControl owns interaction.
use crate::text::Text;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tekes_ui_controls::{
    ToolbarControl, ToolbarIconAction,
    composition::{ComponentId, Content, ContentEvent, ContentHost},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::Theme;
pub(crate) fn icon_image(
    images: &mut HashMap<(String, u32), ImageId>,
    renderer: &mut Renderer,
    name: &str,
    scale: f32,
) -> Result<ImageId> {
    let key = (name.to_owned(), (scale * 1000.).round() as u32);
    if let std::collections::hash_map::Entry::Vacant(slot) = images.entry(key.clone()) {
        let glyph = tekes_ui_icons::Glyph::ALL
            .iter()
            .find(|g| g.name == name)
            .context("unknown toolkit icon")?;
        let tree = resvg::usvg::Tree::from_str(glyph.svg, &resvg::usvg::Options::default())?;
        let pixels = (24. * scale).ceil().max(1.) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(pixels, pixels).context("icon allocation")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(pixels as f32 / 24., pixels as f32 / 24.),
            &mut pixmap.as_mut(),
        );
        slot.insert(renderer.register_image(pixmap.data(), pixels, pixels)?);
    }
    Ok(images[&key])
}
pub(crate) struct VisibilityToolbar {
    pub host: ContentHost,
    pub rect: Rect,
    images: HashMap<(String, u32), ImageId>,
    id: u64,
    icon: &'static str,
}
impl VisibilityToolbar {
    pub fn new(id: u64, icon: &'static str) -> Self {
        let mut host = ContentHost::new(Content::new(ComponentId(1), ToolbarControl::window()))
            .expect("unique visibility contribution");
        host.set_action_events(true);
        Self {
            host,
            rect: Rect::default(),
            images: HashMap::new(),
            id,
            icon,
        }
    }
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        scale: f32,
        viewport: Rect,
        text: &mut Text,
        on: bool,
        label: &str,
    ) -> Result<f32> {
        let image = icon_image(&mut self.images, renderer, self.icon, scale)?;
        let actions = vec![ToolbarIconAction {
            id: self.id,
            image,
            label: label.into(),
            enabled: true,
        }];
        let toolbar = self
            .host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        toolbar.set_viewport(viewport);
        toolbar.set_icon_actions(actions);
        toolbar.set_selected_ids(if on {
            std::slice::from_ref(&self.id)
        } else {
            &[]
        });
        Ok(toolbar.measure(text).height)
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut Text,
        rect: Rect,
        viewport: Rect,
    ) -> Result<()> {
        self.rect = rect;
        self.host
            .render(scene, text, rect, viewport, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("visibility toolbar: {e:?}"))?;
        Ok(())
    }
    pub fn event(&mut self, event: &ContentEvent) -> Option<u64> {
        self.host.event(event);
        let id = self
            .host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap()
            .take_action();
        self.host.drain_actions();
        id
    }
    pub fn cancel(&mut self) {
        self.event(&ContentEvent::Cancel);
    }
}
