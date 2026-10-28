use super::*;
use std::{collections::HashMap, rc::Weak};
use tekes_ui_render::Renderer;
type Key = (u64, &'static str, u32, [u32; 4]);
/// Pixel-resource cache only. The existing Renderer registration owns GPU
/// lifetime; weak variants never keep hidden/unmounted components alive.
#[derive(Debug, Default)]
pub struct SvgIconCache {
    entries: HashMap<Key, (ImageId, Weak<dyn Debug>)>,
    frame: Vec<IconImage>,
}
impl SvgIconCache {
    pub fn renderer<'a>(
        &'a mut self,
        renderer: &'a mut Renderer,
        backing: f32,
    ) -> SvgIconRenderer<'a> {
        self.frame.clear();
        self.entries.retain(|key, (_, lease)| {
            key.0 == renderer.resource_identity() && lease.strong_count() > 0
        });
        renderer.collect_retired_images();
        SvgIconRenderer {
            cache: self,
            renderer,
            backing,
        }
    }
    pub fn clear(&mut self) {
        self.frame.clear();
        self.entries.clear();
    }
}
#[derive(Debug)]
pub struct SvgIconRenderer<'a> {
    cache: &'a mut SvgIconCache,
    renderer: &'a mut Renderer,
    backing: f32,
}
impl IconRenderer for SvgIconRenderer<'_> {
    fn prepare_icon(
        &mut self,
        name: &str,
        size: f32,
        backing: f32,
        tint: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        let glyph = tekes_ui_icons::Glyph::named(name).ok_or(IconRenderError::UnknownIcon)?;
        let pixels = (size * backing).ceil();
        if !size.is_finite()
            || size <= 0.
            || !backing.is_finite()
            || backing <= 0.
            || !(1. ..=4096.).contains(&pixels)
            || ![tint.r, tint.g, tint.b, tint.a]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(&v))
        {
            return Err(IconRenderError::InvalidSize);
        }
        let key = (
            self.renderer.resource_identity(),
            glyph.name,
            pixels as u32,
            [tint.r, tint.g, tint.b, tint.a].map(f32::to_bits),
        );
        if let Some((id, lease)) = self.cache.entries.get(&key)
            && let Some(lease) = lease.upgrade()
        {
            let image = IconImage::new(*id, lease);
            self.cache.frame.push(image.clone());
            return Ok(image);
        }
        let tree = resvg::usvg::Tree::from_str(glyph.svg, &resvg::usvg::Options::default())
            .map_err(|_| IconRenderError::Raster)?;
        let side = pixels as u32;
        let mut bitmap =
            resvg::tiny_skia::Pixmap::new(side, side).ok_or(IconRenderError::Raster)?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(
                pixels / tree.size().width(),
                pixels / tree.size().height(),
            ),
            &mut bitmap.as_mut(),
        );
        for pixel in bitmap.data_mut().chunks_exact_mut(4) {
            let alpha = f32::from(pixel[3]) / 255. * tint.a;
            for (i, c) in [tint.r, tint.g, tint.b].into_iter().enumerate() {
                pixel[i] = (c * alpha * 255.).round() as u8;
            }
            pixel[3] = (alpha * 255.).round() as u8;
        }
        let owned = self
            .renderer
            .register_bitmap_image(bitmap.data(), side, side)
            .map_err(|_| IconRenderError::Upload)?;
        let id = owned.id();
        let lease: Rc<dyn Debug> = Rc::new(owned);
        self.cache.entries.insert(key, (id, Rc::downgrade(&lease)));
        let image = IconImage::new(id, lease);
        self.cache.frame.push(image.clone());
        Ok(image)
    }
    fn draw_icon(&mut self, scene: &mut Scene, name: &str, frame: Rect, tint: Rgba) {
        let image = self
            .prepare_icon(name, frame.size.width, self.backing, tint)
            .expect("checked canonical SVG resource");
        scene.push_image(ImageQuad {
            rect: frame,
            image: image.id(),
            opacity: 1.,
        });
    }
}
