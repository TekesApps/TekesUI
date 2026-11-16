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
fn leading_ink_inset(name: &str) -> Result<f32, IconRenderError> {
    let glyph = tekes_ui_icons::Glyph::named(name).ok_or(IconRenderError::UnknownIcon)?;
    let tree = resvg::usvg::Tree::from_str(glyph.svg, &resvg::usvg::Options::default())
        .map_err(|_| IconRenderError::Raster)?;
    Ok(tree.root().abs_stroke_bounding_box().left() / tree.size().width())
}
impl IconRenderer for SvgIconRenderer<'_> {
    fn leading_ink_inset(&mut self, name: &str) -> Result<f32, IconRenderError> {
        leading_ink_inset(name)
    }
    fn resource_identity(&self) -> Option<u64> {
        Some(self.renderer.resource_identity())
    }
    fn register_icon_bitmap(
        &mut self,
        rgba8_premul: &[u8],
        width: u32,
        height: u32,
    ) -> Result<IconImage, IconRenderError> {
        let owned = self
            .renderer
            .register_bitmap_image(rgba8_premul, width, height)
            .map_err(|_| IconRenderError::Upload)?;
        let image = IconImage::new(owned.id(), Rc::new(owned));
        self.cache.frame.push(image.clone());
        Ok(image)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // CPU-only resource seam: rasterize the actual canonical SVG, retain the
    // ordinary image lease, and independently observe its painted alpha edge.
    #[derive(Default)]
    struct OutlineIcons {
        queried: usize,
        raster_leading: f32,
        unsupported: bool,
    }
    impl IconRenderer for OutlineIcons {
        fn leading_ink_inset(&mut self, name: &str) -> Result<f32, IconRenderError> {
            self.queried += 1;
            if self.unsupported {
                return Err(IconRenderError::UnsupportedRenderer);
            }
            leading_ink_inset(name)
        }
        fn prepare_icon(
            &mut self,
            name: &str,
            size: f32,
            backing: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            let glyph = tekes_ui_icons::Glyph::named(name).unwrap();
            let tree =
                resvg::usvg::Tree::from_str(glyph.svg, &resvg::usvg::Options::default()).unwrap();
            let pixels = (size * backing).ceil() as u32;
            let mut bitmap = resvg::tiny_skia::Pixmap::new(pixels, pixels).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    pixels as f32 / tree.size().width(),
                    pixels as f32 / tree.size().height(),
                ),
                &mut bitmap.as_mut(),
            );
            self.raster_leading = (0..pixels)
                .find(|x| (0..pixels).any(|y| bitmap.pixel(*x, y).unwrap().alpha() >= 128))
                .unwrap() as f32
                / backing;
            Ok(IconImage::new(ImageId(1), Rc::new(())))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("IconContent must retain its prepared image lease");
        }
    }
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(20., 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: Rgba,
        ) {
        }
        fn backing_scale(&self) -> f32 {
            4.
        }
    }
    #[test]
    fn canonical_back_ink_leading_preserves_slot_and_default_toolbar_center() {
        let frame = Rect::new(28., 20., 16., 16.);
        let mut icons = OutlineIcons::default();
        let mut text = Text;
        let mut icon = IconContent::new("tekes-back").unwrap();
        let natural = icon.measure(&mut text, frame.size).size;
        icon.set_ink_leading(true);
        assert_eq!(icon.measure(&mut text, frame.size).size, natural);
        let paint = |icon: &mut IconContent, icons: &mut OutlineIcons| {
            icon.paint(
                &mut Scene::new(),
                &mut WithIconRenderer::new(&mut Text, icons),
                frame,
                frame,
                tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT).into(),
            );
            assert_eq!(icon.icon_error(), None);
        };
        paint(&mut icon, &mut icons);
        assert_eq!(icon.layout().frame, frame);
        assert_eq!(icon.layout().visible, Some(frame));
        let ink_x = icon.layout().artboard.min_x() + icons.raster_leading;
        assert!(
            (ink_x - frame.min_x()).abs() <= 0.25,
            "actual raster outline: {ink_x}"
        );
        paint(&mut icon, &mut icons);
        assert_eq!(
            icons.queried, 1,
            "canonical bounds are queried once per glyph"
        );

        let mut toolbar = Button::new(Content::new(
            ComponentId(2),
            IconContent::new("tekes-back").unwrap(),
        ))
        .with_accessible_name("Back");
        toolbar.set_toolbar_icon_slot(true);
        let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
        let button_frame = Rect::new(100., 20., 40., 40.);
        host.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut text, &mut icons),
            button_frame,
            button_frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let toolbar_icon = host
            .get(ComponentId(2))
            .unwrap()
            .component::<IconContent>()
            .unwrap();
        assert_eq!(
            toolbar_icon.layout().artboard.center(),
            button_frame.center()
        );
        assert_eq!(
            icons.queried, 1,
            "default toolbar never requests outline alignment"
        );

        icon.set_ink_leading(false);
        paint(&mut icon, &mut icons);
        assert_eq!(icon.layout().artboard.center(), frame.center());
        assert!(icon.layout().artboard.min_x() + icons.raster_leading > frame.min_x() + 4.);
    }
    #[test]
    fn ink_leading_reports_backend_without_outline_support() {
        let mut icon = IconContent::new("tekes-back").unwrap();
        icon.set_ink_leading(true);
        let mut icons = OutlineIcons {
            unsupported: true,
            ..Default::default()
        };
        let frame = Rect::new(0., 0., 16., 16.);
        let mut scene = Scene::new();
        icon.paint(
            &mut scene,
            &mut WithIconRenderer::new(&mut Text, &mut icons),
            frame,
            frame,
            tekes_ui_tokens::Palette::from(tekes_ui_tokens::Theme::LIGHT).into(),
        );
        assert_eq!(
            icon.icon_error(),
            Some(IconRenderError::UnsupportedRenderer)
        );
        assert!(scene.primitives().is_empty());
    }

    #[test]
    #[ignore = "requires the registered GPU acceptance lane"]
    fn bitmap_icon_service_keeps_frame_leases_and_renderer_namespaces_separate() {
        let mut renderer = Renderer::new_headless().expect("headless icon renderer");
        let mut cache = SvgIconCache::default();
        let original_identity = renderer.resource_identity();
        let image = {
            let mut service = cache.renderer(&mut renderer, 1.);
            assert_eq!(service.resource_identity(), Some(original_identity));
            assert!(matches!(
                service.register_icon_bitmap(&[255], 1, 1),
                Err(IconRenderError::Upload)
            ));
            service
                .register_icon_bitmap(&[255, 0, 0, 255], 1, 1)
                .unwrap()
        };
        assert_eq!(renderer.image_count(), 1);
        let mut scene = Scene::new();
        scene.push_image(ImageQuad {
            rect: Rect::new(0., 0., 1., 1.),
            image: image.id(),
            opacity: 1.,
        });
        drop(image);
        renderer.collect_retired_images();
        assert_eq!(renderer.image_count(), 1, "frame owns submitted resources");
        let pixels = renderer
            .render_to_rgba8(
                &scene,
                &mut tekes_ui_scene::GlyphAtlas::default(),
                (1, 1),
                1.,
                Rgba::gray(0.),
            )
            .unwrap();
        assert_eq!(&pixels[..4], &[255, 0, 0, 255]);
        let mut replacement = Renderer::new_headless().expect("replacement icon renderer");
        let replacement_image = {
            let mut service = cache.renderer(&mut replacement, 1.);
            assert_ne!(service.resource_identity(), Some(original_identity));
            service
                .register_icon_bitmap(&[0, 0, 255, 255], 1, 1)
                .unwrap()
        };
        renderer.collect_retired_images();
        assert_eq!(renderer.image_count(), 0);
        assert_eq!(replacement.image_count(), 1);
        drop(replacement_image);
        cache.clear();
        replacement.collect_retired_images();
        assert_eq!(replacement.image_count(), 0);
    }
}
