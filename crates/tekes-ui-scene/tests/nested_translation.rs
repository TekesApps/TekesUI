//! Main/advisor counterexample: repeated f32 rounding must agree for every paint kind.
use tekes_ui_scene::*;
use tekes_ui_tokens::Rgba;
#[test]
fn two_half_ulp_translations_keep_keyed_and_unkeyed_geometry_together() {
    let x = 1_000_000_f32;
    let rect = Rect::new(x, 0., 8., 8.);
    let shape = ClipShape::normalize(rect, CornerRadii::uniform(2.), Point::new(0., 0.)).unwrap();
    let mut table = ShapeTable::new(7, 4);
    let key = table.intern(shape).unwrap();
    let mut source = Scene::with_clip_shapes(&table);
    source.push_shape_clip(key.clone());
    source.push_keyed_quad(Quad::new(rect, Rgba::gray(1.)).with_corner_radius(2.), key);
    source.push_quad(Quad::new(rect, Rgba::gray(1.)));
    source.push_clip(rect);
    source.push_glyphs(GlyphRun {
        color: Rgba::gray(1.),
        glyphs: vec![PositionedGlyph {
            origin: Point::new(x, 0.),
            glyph: AtlasGlyph {
                page: 0,
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                left: 0,
                top: 0,
                format: RasterFormat::Mask,
            },
        }],
    });
    source.push_image(ImageQuad {
        rect,
        image: ImageId(1),
        opacity: 1.,
    });
    source.push_transformed_image(TransformedImageQuad::new(
        ImageQuad {
            rect,
            image: ImageId(1),
            opacity: 1.,
        },
        0.,
    ));
    source.push_shadow(ShadowQuad::new(rect, Rgba::gray(1.)));
    source.pop_clip();
    source.pop_clip();
    let translation = FragmentTransform {
        translation: Point::new(0.03125, 0.),
        ..Default::default()
    };
    let mut once = Scene::new();
    once.try_append_fragment(&source, Default::default(), translation)
        .unwrap();
    let mut twice = Scene::new();
    twice
        .try_append_fragment(&once, Default::default(), translation)
        .unwrap();
    let positions: Vec<_> = twice
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::PushShapeClip(key) => {
                Some(("shape clip", twice.clip_shape(key).unwrap().rect().origin.x))
            }
            Primitive::KeyedQuad { quad, .. } => Some(("keyed quad", quad.rect.origin.x)),
            Primitive::Quad(quad) => Some(("ordinary quad", quad.rect.origin.x)),
            Primitive::PushClip(rect) => Some(("rect clip", rect.origin.x)),
            Primitive::Glyphs(run) => Some(("glyph", run.glyphs[0].origin.x)),
            Primitive::Image(image) => Some(("image", image.rect.origin.x)),
            Primitive::TransformedImage(image) => {
                Some(("transformed image", image.quad.rect.origin.x))
            }
            Primitive::Shadow(shadow) => Some(("shadow", shadow.rect.origin.x)),
            Primitive::PopClip => None,
            _ => panic!("unhandled test primitive"),
        })
        .collect();
    assert_eq!(positions.len(), 8);
    assert!(
        positions
            .iter()
            .all(|(_, actual)| actual.to_bits() == x.to_bits()),
        "two sequential half-ULP translations must each round to original x={x:?}; actual={positions:?}"
    );
}
