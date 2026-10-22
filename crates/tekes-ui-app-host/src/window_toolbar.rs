//! Window-owned toolbar contribution; the shared ToolbarControl owns interaction.
use crate::text::Text;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tekes_ui_controls::{
    ToolbarControl, ToolbarIconAction,
    composition::{
        ComponentId, Content, ContentEvent, ContentHost, TextContent, ToolbarContent, ToolbarGroup,
        ToolbarPlacement,
    },
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::Theme;
/// A consumer declares a semantic group; shared ToolbarContent owns spacing,
/// typography, label layout and the original ToolbarControl owns overflow/input.
pub(crate) fn command_group(placement: ToolbarPlacement) -> Content {
    Content::new(
        ComponentId(10),
        ToolbarContent::new(vec![Content::new(
            ComponentId(11),
            ToolbarGroup::new(
                placement,
                vec![Content::new(ComponentId(1), ToolbarControl::window())],
            ),
        )]),
    )
}
/// Main and Settings supply text to the same AppWindow title contribution.
pub(crate) fn title_group() -> Content {
    Content::new(
        ComponentId(10),
        ToolbarContent::new(vec![Content::new(
            ComponentId(11),
            ToolbarGroup::new(
                ToolbarPlacement::Navigation,
                vec![Content::new(ComponentId(2), TextContent::new(""))],
            ),
        )]),
    )
}
pub(crate) fn icon_image(
    images: &mut HashMap<(String, u32), ImageId>,
    renderer: &mut Renderer,
    name: &str,
    scale: f32,
) -> Result<ImageId> {
    let key = (name.to_owned(), (scale * 1000.).round() as u32);
    if let std::collections::hash_map::Entry::Vacant(slot) = images.entry(key.clone()) {
        // Visibility artwork is a Host presentation choice, not a shared toggle style.
        let svg = match name {
            "apphost-leading-collapsed" => include_str!("../assets/leading-collapsed.svg"),
            "apphost-trailing-collapsed" => include_str!("../assets/trailing-collapsed.svg"),
            _ => {
                tekes_ui_icons::Glyph::ALL
                    .iter()
                    .find(|g| g.name == name)
                    .context("unknown toolkit icon")?
                    .svg
            }
        };
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())?;
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
    collapsed_icon: &'static str,
}
impl VisibilityToolbar {
    pub fn new(id: u64, icon: &'static str, collapsed_icon: &'static str) -> Self {
        let mut host = ContentHost::new(command_group(ToolbarPlacement::Navigation))
            .expect("unique visibility contribution");
        host.set_action_events(true);
        Self {
            host,
            rect: Rect::default(),
            images: HashMap::new(),
            id,
            icon,
            collapsed_icon,
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
    ) -> Result<tekes_ui_scene::Size> {
        let image = icon_image(
            &mut self.images,
            renderer,
            if on { self.icon } else { self.collapsed_icon },
            scale * text.type_scale(),
        )?;
        let more = icon_image(
            &mut self.images,
            renderer,
            "tekes-more",
            scale * text.type_scale(),
        )?;
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
        toolbar.set_overflow_icon(Some(more));
        toolbar.set_icon_actions(actions);
        toolbar.set_action_help(self.id, Some(label.into()));
        toolbar.set_selected_ids(&[]);
        Ok(self
            .host
            .get_mut(ComponentId(10))
            .unwrap()
            .measure(text, viewport.size)
            .size)
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
        self.take_action()
    }
    pub fn take_action(&mut self) -> Option<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{ImageQuad, Quad};
    use tekes_ui_tokens::Rgba;
    #[test]
    fn original_toolbar_icons_keep_pixels_and_registration_during_panel_image_replacement() {
        let mut renderer = Renderer::new_headless().unwrap();
        let context = renderer.gpu_context();
        let mut text = Text::new();
        text.scale = 2.;
        let mut toolbar = VisibilityToolbar::new(1, "tekes-side-bar", "apphost-leading-collapsed");
        let whole = Rect::new(0., 0., 320., 120.);
        let mut reference = None;
        let mut icon_ids = None;
        for step in 0..12 {
            let panel = context.create_image((80 + step * 2, 80)).unwrap();
            let panel_id = renderer.register_gpu_image(&panel).unwrap();
            let size = toolbar
                .prepare(
                    &mut renderer,
                    2.,
                    whole,
                    &mut text,
                    true,
                    "Hide leading panel",
                )
                .unwrap();
            let mut scene = Scene::new();
            scene.push_quad(Quad::new(whole, Rgba::gray(1.)));
            scene.push_image(ImageQuad {
                rect: Rect::new(100., 50., 80. + step as f32 * 2., 60.),
                image: panel_id,
                opacity: 1.,
            });
            toolbar
                .render(
                    &mut scene,
                    &mut text,
                    Rect::new(0., 0., size.width, size.height),
                    whole,
                )
                .unwrap();
            let pixels = renderer
                .render_to_rgba8(&scene, &mut text.atlas, (640, 240), 2., Rgba::gray(1.))
                .unwrap();
            let crop: Vec<u8> = (0..84usize)
                .flat_map(|y| pixels[y * 640 * 4..y * 640 * 4 + 84 * 4].iter().copied())
                .collect();
            assert!(
                crop.chunks_exact(4).any(|p| p[0] < 180),
                "canonical icon disappeared"
            );
            if let Some(old) = &reference {
                assert_eq!(&crop, old, "original icon changed during panel replacement");
            } else {
                reference = Some(crop);
            }
            let ids: std::collections::HashSet<_> = toolbar.images.values().copied().collect();
            if let Some(old) = &icon_ids {
                assert_eq!(&ids, old, "icon cache rebuilt on pane resize");
            } else {
                icon_ids = Some(ids);
            }
            assert!(renderer.unregister_image(panel_id));
            assert_eq!(renderer.image_count(), toolbar.images.len());
        }
    }
    #[test]
    fn moving_original_toolbar_icon_keeps_same_pixels_at_fractional_split_positions() {
        for backing in [1., 2.] {
            for font in [13., 20.] {
                let mut renderer = Renderer::new_headless().unwrap();
                let mut text = Text::new();
                text.scale = backing;
                text.set_body_size(font);
                let whole = Rect::new(0., 0., 320., 120.);
                let mut toolbar =
                    VisibilityToolbar::new(1, "tekes-side-bar", "apphost-leading-collapsed");
                let size = toolbar
                    .prepare(
                        &mut renderer,
                        backing,
                        whole,
                        &mut text,
                        true,
                        "Hide leading panel",
                    )
                    .unwrap();
                let mut reference: Option<Vec<u8>> = None;
                for step in 0..17 {
                    let x = step as f32 / 8.;
                    let mut scene = Scene::new();
                    scene.push_quad(Quad::new(whole, Rgba::gray(1.)));
                    toolbar
                        .render(
                            &mut scene,
                            &mut text,
                            Rect::new(x, 0., size.width, size.height),
                            whole,
                        )
                        .unwrap();
                    let icon = scene
                        .primitives()
                        .iter()
                        .find_map(|p| {
                            if let tekes_ui_scene::Primitive::Image(q) = p {
                                Some(q.rect)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let width = (320. * backing) as u32;
                    let height = (120. * backing) as u32;
                    let pixels = renderer
                        .render_to_rgba8(
                            &scene,
                            &mut text.atlas,
                            (width, height),
                            backing,
                            Rgba::gray(1.),
                        )
                        .unwrap();
                    let offset = (icon.min_x() * backing).round() as usize;
                    let top = (icon.min_y() * backing).round() as usize;
                    let side = (icon.size.width * backing).ceil() as usize;
                    let crop: Vec<u8> = (top..top + side)
                        .flat_map(|y| {
                            pixels[(y * width as usize + offset) * 4
                                ..(y * width as usize + offset + side) * 4]
                                .iter()
                                .copied()
                        })
                        .collect();
                    assert!(crop.chunks_exact(4).any(|p| p[0] < 180));
                    if let Some(old) = &reference {
                        let differences = old.iter().zip(&crop).filter(|(a, b)| a != b).count();
                        assert_eq!(
                            differences, 0,
                            "moving original icon changed brightness/coverage: backing={backing} font={font} step={step}"
                        );
                    } else {
                        reference = Some(crop);
                    }
                }
            }
        }
    }
}
