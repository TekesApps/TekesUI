use tekes_ui_scene::{ClipShape, CornerRadii, Point, Rect, Scene, ShapeTable};
#[test]
fn atomic_replay_retains_imported_inventory() {
    let mut table = ShapeTable::new(1, 8);
    let key = table
        .intern(
            ClipShape::normalize(
                Rect::new(0., 0., 40., 40.),
                CornerRadii::uniform(8.),
                Point::new(0., 0.),
            )
            .unwrap(),
        )
        .unwrap();
    let mut fragment = Scene::with_clip_shapes(&table);
    fragment.push_shape_clip(key.clone());
    fragment.pop_clip();
    let mut caller = Scene::new();
    caller.push_clip(Rect::new(0., 0., 80., 80.));
    caller
        .try_append_fragment(&fragment, Default::default(), Default::default())
        .unwrap();
    assert!(
        caller.clip_shape(&key).is_ok(),
        "primitive replay lost immutable source owner"
    );
}
use tekes_ui_scene::{
    ClipError, FragmentBudget, FragmentError, FragmentTransform, ImageId, ImageQuad, Primitive,
    Quad, TransformedImageQuad,
};
fn shape(x: f32) -> ClipShape {
    ClipShape::normalize(
        Rect::new(x, 2., 24., 24.),
        CornerRadii::uniform(6.),
        Point::new(0., 0.),
    )
    .unwrap()
}
fn fragment(table: &mut ShapeTable, x: f32) -> (Scene, tekes_ui_scene::ShapeKey) {
    let key = table.intern(shape(x)).unwrap();
    let mut scene = Scene::with_clip_shapes(table);
    scene.push_shape_clip(key.clone());
    scene.push_keyed_quad(
        Quad::new(shape(x).rect(), tekes_ui_tokens::Rgba::gray(0.5)).with_corner_radius(6.),
        key.clone(),
    );
    scene.pop_clip();
    (scene, key)
}
fn append(a: &mut Scene, b: &Scene) -> Result<(), FragmentError> {
    a.try_append_fragment(b, Default::default(), Default::default())
}
#[test]
fn two_owners_same_generation_preserve_keys_order_and_caller_clip() {
    let mut a = ShapeTable::new(7, 8);
    let mut b = ShapeTable::new(7, 8);
    let (mut caller, ka) = fragment(&mut a, 2.);
    caller.push_clip(Rect::new(1., 1., 60., 60.));
    let prefix = caller.primitives().to_vec();
    let (other, kb) = fragment(&mut b, 30.);
    append(&mut caller, &other).unwrap();
    assert_eq!(&caller.primitives()[..prefix.len()], prefix);
    assert_eq!(&caller.primitives()[prefix.len()..], other.primitives());
    assert_eq!(caller.clip_shape(&ka), Ok(shape(2.)));
    assert_eq!(caller.clip_shape(&kb), Ok(shape(30.)));
    assert_ne!(ka, kb);
    caller.pop_clip();
}
#[test]
fn same_owner_extension_and_old_snapshot_do_not_remove_slots() {
    let mut a = ShapeTable::new(7, 8);
    let (old, k0) = fragment(&mut a, 2.);
    let (new, k1) = fragment(&mut a, 30.);
    let mut caller = old.clone();
    append(&mut caller, &new).unwrap();
    append(&mut caller, &old).unwrap();
    assert_eq!(caller.clip_shape_count(), 2);
    assert_eq!(caller.clip_shape(&k0), Ok(shape(2.)));
    assert_eq!(caller.clip_shape(&k1), Ok(shape(30.)));
    let mut import = Scene::new();
    import
        .try_import_clip_shapes(&a, Default::default())
        .unwrap();
    assert!(import.is_empty());
    assert_eq!(import.clip_shape(&k1), Ok(shape(30.)));
}
#[test]
fn foreign_post_snapshot_and_bad_tuple_reject_without_any_mutation() {
    let mut a = ShapeTable::new(7, 8);
    let (mut caller, key) = fragment(&mut a, 2.);
    let before = caller.clone();
    let mut foreign = ShapeTable::new(7, 8);
    let (_, foreign_key) = fragment(&mut foreign, 2.);
    for bad in [foreign_key, a.intern(shape(30.)).unwrap()] {
        let mut f = caller.fragment();
        f.push_shape_clip(bad);
        f.pop_clip();
        assert_eq!(
            append(&mut caller, &f),
            Err(FragmentError::Geometry(ClipError::StaleShapeKey))
        );
        assert_eq!(caller, before);
    }
    let mut f = caller.fragment();
    f.push_keyed_quad(
        Quad::new(Rect::new(0., 0., 1., 1.), tekes_ui_tokens::Rgba::gray(1.)),
        key,
    );
    assert_eq!(
        append(&mut caller, &f),
        Err(FragmentError::KeyedQuadMismatch)
    );
    assert_eq!(caller, before);
}
#[test]
fn capabilities_balance_and_cumulative_budgets_reject_atomically() {
    let mut a = ShapeTable::new(1, 8);
    let (mut caller, _) = fragment(&mut a, 2.);
    let (f, _) = fragment(&mut a, 30.);
    let before = caller.clone();
    let mut bad = f.clone();
    bad.require_clip_geometry(2);
    assert_eq!(
        append(&mut caller, &bad),
        Err(FragmentError::UnsupportedClipGeometry(2))
    );
    assert_eq!(caller, before);
    for push in [true, false] {
        let mut bad = Scene::new();
        if push {
            bad.push_clip(Rect::new(0., 0., 1., 1.));
        } else {
            bad.pop_clip();
        }
        assert_eq!(
            append(&mut caller, &bad),
            Err(FragmentError::UnbalancedClip)
        );
        assert_eq!(caller, before);
    }
    let d = FragmentBudget::default();
    for budget in [
        FragmentBudget {
            max_primitives: 5,
            ..d
        },
        FragmentBudget { max_shapes: 1, ..d },
        FragmentBudget {
            max_sources: 0,
            ..d
        },
        FragmentBudget { max_depth: 0, ..d },
        FragmentBudget { max_bytes: 1, ..d },
        FragmentBudget { max_work: 1, ..d },
    ] {
        assert_eq!(
            caller.try_append_fragment(&f, budget, Default::default()),
            Err(FragmentError::BudgetExceeded)
        );
        assert_eq!(caller, before);
    }
    let mut nested = Scene::new();
    nested.push_clip(Rect::new(0., 0., 80., 80.));
    let before = nested.clone();
    assert_eq!(
        nested.try_append_fragment(&f, FragmentBudget { max_depth: 1, ..d }, Default::default()),
        Err(FragmentError::BudgetExceeded)
    );
    assert_eq!(nested, before);
}
#[test]
fn translation_derives_keys_without_changing_source_or_existing_keys() {
    let mut table = ShapeTable::new(9, 8);
    let (source, key) = fragment(&mut table, 2.);
    let before = source.clone();
    let mut caller = source.clone();
    caller
        .try_append_fragment(
            &source,
            Default::default(),
            FragmentTransform {
                translation: Point::new(0.25, 0.5),
                opacity: 0.5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(source, before);
    assert_eq!(caller.clip_shape(&key), Ok(shape(2.)));
    assert_eq!(&caller.primitives()[..source.len()], source.primitives());
    let Primitive::PushShapeClip(derived) = &caller.primitives()[source.len()] else {
        panic!()
    };
    assert_ne!(&key, derived);
    let expected =
        ClipShape::normalize(shape(2.).rect(), shape(2.).radii(), Point::new(0.25, 0.5)).unwrap();
    assert_eq!(caller.clip_shape(derived), Ok(expected));
    let Primitive::KeyedQuad { quad, outer } = &caller.primitives()[source.len() + 1] else {
        panic!()
    };
    assert_eq!(derived, outer);
    assert_eq!(quad.rect, expected.rect());
    assert_eq!(quad.fill.a, 0.5);
    // Derived keys still require the immutable original source inventory.
    assert_eq!(
        Scene::new().clip_shape(derived),
        Err(ClipError::StaleShapeKey)
    );
}
#[test]
fn unsupported_scale_opacity_rotation_and_numeric_translation_are_atomic() {
    let mut table = ShapeTable::new(1, 8);
    let (source, _) = fragment(&mut table, 2.);
    let mut caller = source.clone();
    let before = caller.clone();
    for transform in [
        FragmentTransform {
            scale: 2.,
            ..Default::default()
        },
        FragmentTransform {
            opacity: f32::NAN,
            ..Default::default()
        },
        FragmentTransform {
            opacity: 1.1,
            ..Default::default()
        },
        FragmentTransform {
            translation: Point::new(f32::INFINITY, 0.),
            ..Default::default()
        },
    ] {
        assert_eq!(
            caller.try_append_fragment(&source, Default::default(), transform),
            Err(FragmentError::UnsupportedTransform)
        );
        assert_eq!(caller, before);
    }
    assert!(
        caller
            .try_append_fragment(
                &source,
                Default::default(),
                FragmentTransform {
                    translation: Point::new(2_f32.powi(27), 0.),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(caller, before);
    let mut rotated = Scene::new();
    rotated.push_quad(Quad::new(
        Rect::new(0., 0., 2., 2.),
        tekes_ui_tokens::Rgba::gray(1.),
    ));
    rotated.push_transformed_image(TransformedImageQuad::new(
        ImageQuad {
            rect: Rect::new(0., 0., 2., 2.),
            image: ImageId(1),
            opacity: 1.,
        },
        0.5,
    ));
    assert_eq!(
        caller.try_append_fragment(
            &rotated,
            Default::default(),
            FragmentTransform {
                translation: Point::new(1., 1.),
                ..Default::default()
            }
        ),
        Err(FragmentError::UnsupportedTransform)
    );
    assert_eq!(caller, before);
}
#[test]
fn first_scene_error_clone_clear_and_failed_fragment_are_explicit() {
    let mut caller = Scene::new();
    caller.push_clip(Rect::new(0., 0., 80., 80.));
    let before = caller.clone();
    let mut failed = Scene::new();
    failed.record_composition_error(FragmentError::UnsupportedTransform);
    failed.record_composition_error(FragmentError::UnbalancedClip);
    assert_eq!(
        failed.clone().composition_error(),
        Some(FragmentError::UnsupportedTransform)
    );
    assert_eq!(
        append(&mut caller, &failed),
        Err(FragmentError::UnsupportedTransform)
    );
    assert_eq!(caller, before);
    caller.record_composition_error(FragmentError::BudgetExceeded);
    let marked = caller.clone();
    assert_eq!(
        append(&mut caller, &Scene::new()),
        Err(FragmentError::BudgetExceeded)
    );
    assert_eq!(caller, marked);
    caller.clear();
    assert_eq!(caller.composition_error(), None);
    assert!(caller.is_empty());
}
#[test]
fn glyph_budget_counts_caller_and_fragment_together() {
    let glyph = tekes_ui_scene::PositionedGlyph {
        origin: Point::new(1., 1.),
        glyph: tekes_ui_scene::AtlasGlyph {
            page: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            left: 0,
            top: 0,
            format: tekes_ui_scene::RasterFormat::Mask,
        },
    };
    let mut fragment = Scene::new();
    fragment.push_glyphs(tekes_ui_scene::GlyphRun {
        color: tekes_ui_tokens::Rgba::gray(1.),
        glyphs: vec![glyph],
    });
    let mut caller = fragment.clone();
    let before = caller.clone();
    assert_eq!(
        caller.try_append_fragment(
            &fragment,
            FragmentBudget {
                max_glyphs: 1,
                ..Default::default()
            },
            Default::default()
        ),
        Err(FragmentError::BudgetExceeded)
    );
    assert_eq!(caller, before);
}
