use tekes_ui_scene::*;
use tekes_ui_tokens::Rgba;
fn fragment(x: f32) -> (ShapeTable, ShapeKey, Scene) {
    let shape = ClipShape::normalize(
        Rect::new(x, 0., 8., 8.),
        CornerRadii::uniform(2.),
        Point::new(0., 0.),
    )
    .unwrap();
    let mut table = ShapeTable::new(9, 2);
    let key = table.intern(shape).unwrap();
    let mut scene = Scene::with_clip_shapes(&table);
    scene.push_shape_clip(key.clone());
    scene.push_keyed_quad(
        Quad::new(shape.rect(), Rgba::gray(1.)).with_corner_radius(2.),
        key.clone(),
    );
    scene.push_quad(Quad::new(shape.rect(), Rgba::gray(1.)));
    scene.pop_clip();
    (table, key, scene)
}
fn translated(source: &Scene, dx: f32) -> Scene {
    let mut scene = Scene::new();
    scene
        .try_append_fragment(
            source,
            Default::default(),
            FragmentTransform {
                translation: Point::new(dx, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
}
#[test]
fn signed_even_and_odd_half_ulp_origins_have_exact_sequential_results() {
    for (start, expected) in [
        (0x49742400, 0x49742400),
        (0x49742401, 0x49742402),
        (0xc9742400, 0xc9742400),
        (0xc9742401, 0xc9742400),
    ] {
        let (_, _, source) = fragment(f32::from_bits(start));
        let twice = translated(&translated(&source, 0.03125), 0.03125);
        for primitive in twice.primitives() {
            let x = match primitive {
                Primitive::PushShapeClip(key) => twice.clip_shape(key).unwrap().rect().origin.x,
                Primitive::KeyedQuad { quad, .. } | Primitive::Quad(quad) => quad.rect.origin.x,
                Primitive::PopClip => continue,
                _ => panic!("unexpected primitive"),
            };
            assert_eq!(x.to_bits(), expected, "start bits {start:x}");
        }
    }
}
#[test]
fn noop_rounding_keeps_original_key_but_one_combined_translation_is_distinct() {
    let (table, key, source) = fragment(1_000_000.);
    let twice = translated(&translated(&source, 0.03125), 0.03125);
    let combined = translated(&source, 0.0625);
    let Primitive::PushShapeClip(twice_key) = &twice.primitives()[0] else {
        panic!()
    };
    let Primitive::PushShapeClip(combined_key) = &combined.primitives()[0] else {
        panic!()
    };
    assert_eq!(twice_key, &key);
    assert_ne!(combined_key, &key);
    assert_eq!(
        twice
            .clip_shape(twice_key)
            .unwrap()
            .rect()
            .origin
            .x
            .to_bits(),
        0x49742400
    );
    assert_eq!(
        combined
            .clip_shape(combined_key)
            .unwrap()
            .rect()
            .origin
            .x
            .to_bits(),
        0x49742401
    );
    assert_eq!(
        table.get(key).unwrap().rect().origin.x.to_bits(),
        0x49742400
    );
}
#[test]
fn derived_key_still_requires_exact_original_owner_and_snapshot_slot() {
    let (table, _, source) = fragment(1_000_000.);
    let derived = translated(&source, 0.0625);
    let Primitive::PushShapeClip(key) = &derived.primitives()[0] else {
        panic!()
    };
    let (mut other, _, mut caller) = fragment(1_000_000.);
    let before = caller.clone();
    assert_eq!(other.get(key.clone()), Err(ClipError::StaleShapeKey));
    let mut foreign = Scene::new();
    foreign.require_clip_geometry(1);
    foreign.push_shape_clip(key.clone());
    foreign.pop_clip();
    assert_eq!(
        caller.try_append_fragment(&foreign, Default::default(), Default::default()),
        Err(FragmentError::Geometry(ClipError::StaleShapeKey))
    );
    assert_eq!(caller, before);
    assert_eq!(
        table.get(key.clone()).unwrap().rect().origin.x.to_bits(),
        0x49742401
    );
    // Importing the original source is explicit and preserves both existing owners.
    caller
        .try_import_clip_shapes(&table, Default::default())
        .unwrap();
    caller
        .try_append_fragment(&foreign, Default::default(), Default::default())
        .unwrap();
    assert_eq!(caller.clip_shape(key), table.get(key.clone()));
    let _ = other
        .intern(
            ClipShape::normalize(
                Rect::new(0., 0., 4., 4.),
                CornerRadii::ZERO,
                Point::new(0., 0.),
            )
            .unwrap(),
        )
        .unwrap();
}
#[test]
fn nested_numeric_or_budget_rejection_preserves_caller_registry_and_primitives() {
    let (_, _, source) = fragment(1_000_000.);
    let once = translated(&source, 0.0625);
    let mut caller = once.clone();
    let before = caller.clone();
    let error = caller.try_append_fragment(
        &once,
        FragmentBudget {
            max_work: 1,
            ..Default::default()
        },
        FragmentTransform {
            translation: Point::new(0.03125, 0.),
            ..Default::default()
        },
    );
    assert_eq!(error, Err(FragmentError::BudgetExceeded));
    assert_eq!(caller, before);
    let error = caller.try_append_fragment(
        &once,
        Default::default(),
        FragmentTransform {
            translation: Point::new(f32::MAX, 0.),
            ..Default::default()
        },
    );
    assert!(matches!(error, Err(FragmentError::Geometry(_))));
    assert_eq!(caller, before);
}
