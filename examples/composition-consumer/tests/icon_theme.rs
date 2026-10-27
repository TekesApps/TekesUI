//! Pure IconView owns fitted geometry/tint; interaction belongs to its consumer.
#![cfg(target_os = "macos")]
use resvg::{tiny_skia, usvg};
use tekes_ui_controls::layout_view::{IconRenderer, IconView};
use tekes_ui_icons::Glyph;
use tekes_ui_scene::{Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct SvgArtwork {
    glyph: Glyph,
    tree: usvg::Tree,
}
impl IconRenderer for SvgArtwork {
    fn draw_icon(&mut self, scene: &mut Scene, canonical_name: &str, frame: Rect, tint: Rgba) {
        assert_eq!(canonical_name, self.glyph.name);
        // A real canonical SVG raster, emitted into the Scene without a GPU or
        // an unregistered image handle. This backend is local to this test.
        let side = frame.size.width.round() as u32;
        let mut pixels = tiny_skia::Pixmap::new(side, side).unwrap();
        resvg::render(
            &self.tree,
            tiny_skia::Transform::from_scale(
                frame.size.width / self.tree.size().width(),
                frame.size.height / self.tree.size().height(),
            ),
            &mut pixels.as_mut(),
        );
        for (index, pixel) in pixels.data().chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let x = (index as u32 % side) as f32;
            let y = (index as u32 / side) as f32;
            scene.push_quad(Quad::new(
                Rect::new(frame.min_x() + x, frame.min_y() + y, 1., 1.),
                tint.with_alpha(tint.a * f32::from(pixel[3]) / 255.),
            ));
        }
    }
}

#[test]
fn retained_public_icon_repaints_semantic_and_explicit_tints_without_an_interaction_model() {
    let glyph = Glyph::named("tekes-settings").expect("existing canonical public artwork");
    let tree = usvg::Tree::from_str(glyph.svg, &usvg::Options::default()).unwrap();
    let mut artwork = SvgArtwork { glyph, tree };
    let mut view = IconView::default();
    let frame = Rect::new(20., 30., 80., 64.);
    let side = proposal::metric("size", "size-icon-preview").unwrap();
    let mut original_geometry = None;
    let mut previous_ink = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        let primary = palette.color("primary-text").unwrap();
        // IconView has no disabled property. An explicit disabled tint is owned
        // by the caller; returning to render() restores the default semantics.
        let disabled = palette.color("disabled-text").unwrap();
        assert_ne!(primary, disabled);
        for tint in [None, Some(disabled), None] {
            let mut scene = Scene::new();
            let layout = if let Some(tint) = tint {
                view.render_tinted(
                    &mut scene,
                    &mut artwork,
                    glyph.name,
                    frame,
                    0.,
                    Some(frame),
                    palette,
                    tint,
                )
            } else {
                view.render(
                    &mut scene,
                    &mut artwork,
                    glyph.name,
                    frame,
                    0.,
                    Some(frame),
                    palette,
                )
            };
            assert_eq!(view.layout(), layout);
            assert_eq!(layout.artboard.size, Size::new(side, side));
            assert_eq!(layout.artboard.center(), frame.center());
            assert_eq!(layout.visible, Some(frame));
            assert!(scene.composition_error().is_none());
            let quads: Vec<_> = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Quad(q) => Some(q),
                    _ => None,
                })
                .collect();
            assert!(
                !quads.is_empty(),
                "canonical SVG produces real painted pixels"
            );
            let expected = tint.unwrap_or(primary);
            assert!(quads.iter().all(|q| {
                (q.fill.r, q.fill.g, q.fill.b) == (expected.r, expected.g, expected.b)
                    && q.fill.a > 0.
                    && q.fill.a <= expected.a
                    && q.rect.min_x() >= layout.artboard.min_x()
                    && q.rect.max_x() <= layout.artboard.max_x()
                    && q.rect.min_y() >= layout.artboard.min_y()
                    && q.rect.max_y() <= layout.artboard.max_y()
            }));
            let geometry: Vec<_> = quads.iter().map(|q| q.rect).collect();
            if let Some(original) = &original_geometry {
                assert_eq!(&geometry, original);
            } else {
                original_geometry = Some(geometry);
            }
        }
        if let Some(previous) = previous_ink
            && theme.is_dark()
        {
            assert_ne!(
                primary, previous,
                "same IconView repaints on appearance changes"
            );
        }
        previous_ink = Some(primary);
    }
    let mut hidden = Scene::new();
    let layout = view.render(
        &mut hidden,
        &mut artwork,
        glyph.name,
        frame,
        side,
        Some(Rect::new(500., 500., 10., 10.)),
        Theme::LIGHT.into(),
    );
    assert!(layout.visible.is_none());
    assert!(
        hidden.primitives().is_empty(),
        "offscreen Icon does not call its real artwork backend"
    );
}
