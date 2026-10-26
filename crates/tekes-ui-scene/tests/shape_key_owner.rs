use tekes_ui_scene::{ClipError, ClipShape, CornerRadii, Point, Rect, ShapeTable};
fn shape(width: f32) -> ClipShape {
    ClipShape::normalize(
        Rect::new(0., 0., width, 4.),
        CornerRadii::ZERO,
        Point::new(0., 0.),
    )
    .unwrap()
}
#[test]
fn same_generation_and_slot_do_not_authorize_other_table() {
    let mut a = ShapeTable::new(7, 2);
    let mut b = ShapeTable::new(7, 2);
    let ka = a.intern(shape(3.)).unwrap();
    let kb = b.intern(shape(4.)).unwrap();
    assert_ne!(
        ka, kb,
        "different owners must not alias at equal generation/index"
    );
    assert_eq!(b.get(ka.clone()), Err(ClipError::StaleShapeKey));
    assert_eq!(a.get(kb), Err(ClipError::StaleShapeKey));
    assert_eq!(a.get(ka), Ok(shape(3.)));
}
#[test]
fn equal_geometry_across_tables_is_not_key_authority() {
    let s = shape(3.);
    let mut a = ShapeTable::new(7, 2);
    let mut b = ShapeTable::new(7, 2);
    let ka = a.intern(s).unwrap();
    let kb = b.intern(s).unwrap();
    assert_ne!(ka, kb);
    assert_eq!(b.get(ka.clone()), Err(ClipError::StaleShapeKey));
    assert_eq!(
        a.intern(s).unwrap(),
        ka,
        "same-owner full-tuple dedup is preserved"
    );
}
#[test]
fn reconstruction_cannot_revive_dropped_table_key() {
    let old = {
        let mut t = ShapeTable::new(7, 1);
        t.intern(shape(3.)).unwrap()
    };
    for _ in 0..64 {
        let mut replacement = ShapeTable::new(7, 1);
        let next = replacement.intern(shape(3.)).unwrap();
        assert_ne!(old, next);
        assert_eq!(replacement.get(old.clone()), Err(ClipError::StaleShapeKey));
    }
}

#[test]
fn key_hash_uses_owner_identity_and_clone_preserves_authority() {
    let s = shape(3.);
    let mut a = ShapeTable::new(7, 1);
    let mut b = ShapeTable::new(7, 1);
    let ka = a.intern(s).unwrap();
    let kb = b.intern(s).unwrap();
    let mut set = std::collections::HashSet::new();
    set.insert(ka.clone());
    set.insert(ka.clone());
    set.insert(kb.clone());
    assert_eq!(set.len(), 2);
    assert_eq!(a.get(ka.clone()), Ok(s));
    assert_eq!(b.get(ka), Err(ClipError::StaleShapeKey));
    assert_eq!(b.get(kb), Ok(s));
}
#[test]
fn supported_numeric_domain_is_translation_dependent() {
    assert!(
        ClipShape::normalize(
            Rect::new(0., 0., 4., 4.),
            CornerRadii::ZERO,
            Point::new(0., 0.)
        )
        .is_ok()
    );
    assert_eq!(
        ClipShape::normalize(
            Rect::new(0., 0., 4., 4.),
            CornerRadii::ZERO,
            Point::new(2_f32.powi(27), 0.)
        ),
        Err(ClipError::UnsupportedNumericRange)
    );
}
