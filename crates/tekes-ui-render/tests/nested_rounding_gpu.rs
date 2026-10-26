//! Exact GPU readback after canonical half-ULP steps, rebased into the target.
use tekes_ui_render::Renderer;
use tekes_ui_scene::*;
use tekes_ui_tokens::Rgba;
fn mixed(x: f32, image: ImageId, glyph: AtlasGlyph) -> Scene {
    let rect = Rect::new(x, 4., 8., 8.);
    let shape = ClipShape::normalize(rect, CornerRadii::uniform(2.), Point::new(0., 0.)).unwrap();
    let mut table = ShapeTable::new(91, 2);
    let key = table.intern(shape).unwrap();
    let mut scene = Scene::with_clip_shapes(&table);
    scene.push_shape_clip(key.clone());
    scene.push_clip(rect);
    scene.push_keyed_quad(
        Quad::new(rect, Rgba::new(0.8, 0.2, 0.1, 0.6))
            .with_corner_radius(2.)
            .with_border(1., Rgba::new(0.1, 0.6, 0.2, 0.4)),
        key,
    );
    scene.push_quad(Quad::new(rect, Rgba::new(0.1, 0.3, 0.6, 0.3)));
    scene.push_image(ImageQuad {
        rect,
        image,
        opacity: 0.5,
    });
    scene.push_glyphs(GlyphRun {
        color: Rgba::new(0.7, 0.3, 0.8, 0.5),
        glyphs: vec![PositionedGlyph {
            origin: Point::new(x, 4.),
            glyph,
        }],
    });
    scene.pop_clip();
    scene.pop_clip();
    scene
}
#[test]
fn repeated_half_ulp_shape_quad_rect_glyph_image_match_direct_scene_at_every_backing() {
    let mut renderer = Renderer::new_headless().expect("actual GPU required; no skip");
    let image = renderer.register_image(&[20, 50, 90, 128], 1, 1).unwrap();
    let output = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_FRAGMENT_GPU_OUT").expect("explicit QA output"),
    );
    std::fs::create_dir_all(&output).unwrap();
    for scale in [0.5, 1., 1.25, 1.5, 2.] {
        let mut atlas = GlyphAtlas::default();
        let glyph = atlas
            .get_or_insert_with(
                GlyphKey::new(FontKey(1), 1, GlyphKey::scale_key_for(scale)),
                || {
                    Some(RasterGlyph {
                        left: 0,
                        top: 0,
                        width: 8,
                        height: 8,
                        format: RasterFormat::Mask,
                        pixels: [0, 0, 0, 200].repeat(64),
                    })
                },
            )
            .unwrap();
        let mut fragment = mixed(1_000_000., image, glyph);
        for delta in [0.03125, 0.03125, -999_996.] {
            let mut next = Scene::new();
            next.try_append_fragment(
                &fragment,
                Default::default(),
                FragmentTransform {
                    translation: Point::new(delta, 0.),
                    ..Default::default()
                },
            )
            .unwrap();
            fragment = next;
        }
        let reference = mixed(4., image, glyph);
        let size = ((32_f32 * scale) as u32, (32_f32 * scale) as u32);
        let expected = renderer
            .render_to_rgba8(&reference, &mut atlas, size, scale, Rgba::gray(0.))
            .unwrap();
        let actual = renderer
            .render_to_rgba8(&fragment, &mut atlas, size, scale, Rgba::gray(0.))
            .unwrap();
        std::fs::write(
            output.join(format!("nested-half-ulp-{scale}-reference.rgba")),
            &expected,
        )
        .unwrap();
        std::fs::write(
            output.join(format!("nested-half-ulp-{scale}-composed.rgba")),
            &actual,
        )
        .unwrap();
        assert_eq!(
            actual, expected,
            "{scale}: keyed contour and ordinary content must remain exactly aligned"
        );
        assert!(
            actual.chunks_exact(4).any(|pixel| pixel[0] > 50),
            "positive ink required"
        );
        assert_eq!(&actual[..4], &[0, 0, 0, 255], "outside clip stays clear");
    }
}
