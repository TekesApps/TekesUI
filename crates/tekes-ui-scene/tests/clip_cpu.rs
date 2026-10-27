use tekes_ui_scene::{
    ClipBudget, ClipError, ClipRegion, ClipShape, CornerRadii, Point, Rect, ShapeTable,
};
#[path = "clip_cpu/fixtures.rs"]
mod fixtures;
fn shape(v: [f32; 8]) -> ClipShape {
    ClipShape::normalize(
        Rect::new(v[0], v[1], v[2], v[3]),
        CornerRadii {
            top_left: v[4],
            top_right: v[5],
            bottom_right: v[6],
            bottom_left: v[7],
        },
        Point::new(0., 0.),
    )
    .unwrap()
}
fn region(s: &[ClipShape]) -> ClipRegion {
    ClipRegion::new(
        s,
        ClipBudget {
            max_shapes: 32,
            max_work: 10_000_000,
        },
    )
    .unwrap()
}
#[test]
fn independent_decimal_oracle_bounds() {
    for (name, inputs, expected) in fixtures::ORACLE {
        let s: Vec<_> = inputs.iter().copied().map(shape).collect();
        let got = region(&s).visible_bounds();
        match (got, expected) {
            (Ok(None), None) => {}
            (Ok(Some(b)), Some(v)) => {
                for (i, iv) in [b.min[0], b.min[1], b.max[0], b.max[1]]
                    .into_iter()
                    .enumerate()
                {
                    assert!(
                        iv.lower <= v[i].next_up() && iv.upper >= v[i].next_down(),
                        "{name} coordinate {i}: {iv:?} vs {}",
                        v[i]
                    );
                    assert!(
                        iv.upper - iv.lower <= 1e-10 * v[i].abs().max(1.0),
                        "{name}: need a usable enclosure {iv:?}"
                    );
                }
            }
            (a, b) => panic!("{name}: CPU {a:?}, independent oracle {b:?}"),
        }
    }
}
#[test]
fn normalization_identity_generation_and_input_budgets() {
    let a = shape([-0., 0., 10., 8., -2., 99., 0., 1.]);
    assert_eq!(a.radii().to_array(), [0., 4., 0., 1.]);
    assert_eq!(a, shape([0., 0., 10., 8., 0., 4., 0., 1.]));
    assert!(shape([0., 0., 2., 2., 0., -1., 0., 0.]).is_rect());
    let translated =
        ClipShape::normalize(Rect::new(1., 2., 10., 8.), a.radii(), Point::new(-1., -2.)).unwrap();
    assert_eq!(translated, a);
    let mut t = ShapeTable::new(7, 1);
    let key = t.intern(a).unwrap();
    assert_eq!(t.intern(a).unwrap(), key);
    assert_eq!(
        t.intern(shape([0., 0., 10., 8., 0., 0., 0., 0.])),
        Err(ClipError::InputBudgetExceeded)
    );
    assert_eq!(
        ShapeTable::new(8, 1).get(key),
        Err(ClipError::StaleShapeKey)
    );
    assert!(matches!(
        ClipRegion::new(
            &[a, a],
            ClipBudget {
                max_shapes: 1,
                max_work: usize::MAX
            }
        ),
        Err(ClipError::InputBudgetExceeded)
    ));
    assert!(matches!(
        ClipRegion::new(
            &[a],
            ClipBudget {
                max_shapes: 1,
                max_work: 0
            }
        ),
        Err(ClipError::QueryBudgetExceeded)
    ));
    assert!(matches!(
        ClipRegion::new(
            &[],
            ClipBudget {
                max_shapes: 1,
                max_work: 1000
            }
        ),
        Err(ClipError::InvalidInput)
    ));
}
#[test]
fn closed_domain_and_legacy_half_open_are_distinct() {
    let r = Rect::new(0., 0., 100., 100.);
    let s = shape([0., 0., 100., 100., 20., 20., 20., 20.]);
    assert!(!r.contains(Point::new(100., 50.)));
    assert_eq!(s.contains([100., 50.]), Ok(true));
    assert_eq!(s.contains([1., 1.]), Ok(false));
    assert_eq!(s.contains([20., 0.]), Ok(true));
    let c = shape([-1., -1., 2., 2., 1., 1., 1., 1.]);
    assert_eq!(c.contains([0., 1.]), Ok(true));
    assert_eq!(c.contains([1., 1.]), Ok(false));
    assert_eq!(c.contains([0., 0.]), Ok(true));
}
#[test]
fn sections_are_analytic_and_nested() {
    let s = shape([0., 0., 100., 100., 20., 20., 20., 20.]);
    let r = region(&[s]);
    let sec = r.section(1, 0.).unwrap().unwrap();
    assert_eq!(
        (sec[0].lower, sec[0].upper, sec[1].lower, sec[1].upper),
        (20., 20., 80., 80.)
    );
    let sec = r.section(0, 1.).unwrap().unwrap();
    let expected = 20. - 39_f64.sqrt();
    assert!(sec[0].lower <= expected && sec[0].upper >= expected);
    assert_eq!(r.section(0, -1.), Ok(None));
    assert_eq!(r.section(2, 0.), Err(ClipError::InvalidInput));
    assert_eq!(
        region(&[s, shape([0., 0., 1., 1., 0., 0., 0., 0.])]).section(0, 0.),
        Ok(None)
    );
}
#[test]
fn inset_is_subset_and_thick_border_is_empty() {
    for v in [
        [0., 0., 20., 16., 8., 2., 0., 4.],
        [-1., -1., 2., 2., 1., 1., 1., 1.],
    ] {
        let s = shape(v);
        for b in [0., 0.25, 1., 3., 8., 100.] {
            let inner = s.inset(b).unwrap();
            for x in -8..90 {
                for y in -8..70 {
                    let p = [f64::from(x) * 0.25, f64::from(y) * 0.25];
                    if inner.contains(p) == Ok(true) {
                        assert_eq!(s.contains(p), Ok(true), "{v:?}, {b}, {p:?}");
                    }
                }
            }
        }
        assert!(s.inset(100.).unwrap().is_empty());
        assert!(s.inset(f32::MAX).unwrap().is_empty());
    }
    let s = shape([0., 0., 20., 16., 8., 2., 0., 4.]);
    assert_eq!(s.inset(f32::NAN).unwrap_err(), ClipError::InvalidInput);
    assert_eq!(
        s.inset(-1.).unwrap().contains([10., 0.]),
        s.contains([10., 0.])
    );
}
#[test]
fn invalid_overflow_and_uncertainty_fail_closed() {
    for v in [
        [0., 0., -1., 1., 0., 0., 0., 0.],
        [0., 0., 1., 1., f32::NAN, 0., 0., 0.],
        [f32::MAX, 0., f32::MAX, 1., 0., 0., 0., 0.],
    ] {
        assert!(
            ClipShape::normalize(
                Rect::new(v[0], v[1], v[2], v[3]),
                CornerRadii {
                    top_left: v[4],
                    top_right: v[5],
                    bottom_right: v[6],
                    bottom_left: v[7]
                },
                Point::new(0., 0.)
            )
            .is_err()
        );
    }
    assert_eq!(
        ClipShape::normalize(
            Rect::new(0., 0., 2_f32.powi(61), 1.),
            CornerRadii::ZERO,
            Point::new(0., 0.)
        ),
        Err(ClipError::UnsupportedNumericRange)
    );
    let c = shape([-1., -1., 2., 2., 1., 1., 1., 1.]);
    assert_eq!(c.contains([f64::NAN, 0.]), Err(ClipError::InvalidInput));
    let p = [0.5, 0.75_f64.sqrt()];
    assert_eq!(c.contains(p), Err(ClipError::NumericallyIndeterminate));
    assert_eq!(
        region(&[c]).contains(p),
        Err(ClipError::NumericallyIndeterminate)
    );
}
#[test]
fn independent_decimal_sections_and_points() {
    for (name, inputs, axis, at, expected) in fixtures::SECTIONS {
        let s: Vec<_> = inputs.iter().copied().map(shape).collect();
        let got = region(&s).section(*axis, *at);
        match (got, expected) {
            (Ok(None), None) => {}
            (Ok(Some(v)), Some(e)) => {
                for i in 0..2 {
                    assert!(
                        v[i].lower <= e[i].next_up() && v[i].upper >= e[i].next_down(),
                        "{name} section axis {axis} at {at}: {v:?} expected {e:?}"
                    );
                }
            }
            (a, b) => panic!("{name} section axis {axis} at {at}: {a:?} expected {b:?}"),
        }
    }
    for (name, inputs, p, expected) in fixtures::POINTS {
        let s: Vec<_> = inputs.iter().copied().map(shape).collect();
        assert_eq!(region(&s).contains(*p), Ok(*expected), "{name} at {p:?}");
    }
}

#[test]
fn whole_span_and_visible_uncertainty_are_explicit() {
    let s = shape([0., 0., 100., 100., 20., 20., 20., 20.]);
    let r = region(&[s, s, s]);
    assert_eq!(r.visible_bounds(), region(&[s]).visible_bounds());
    let v = r.section_for_span(1, [0., 10.]).unwrap().unwrap();
    assert_eq!(
        (v[0].lower, v[0].upper, v[1].lower, v[1].upper),
        (20., 20., 80., 80.)
    );
    assert_eq!(r.section_for_span(1, [-1., 10.]), Ok(None));
    assert_eq!(
        r.section_for_span(1, [10., 0.]),
        Err(ClipError::InvalidInput)
    );
    let small = 2_f32.powi(-50);
    let big = 2_f32.powi(50);
    let tiny = shape([
        -small,
        -small,
        2. * small,
        2. * small,
        small,
        small,
        small,
        small,
    ]);
    let huge = shape([0., -big, 2. * big, 2. * big, big, big, big, big]);
    let uncertain = region(&[tiny, huge]);
    assert_eq!(
        uncertain.visible_bounds(),
        Err(ClipError::NumericallyIndeterminate)
    );
    assert_eq!(
        uncertain.has_positive_area(),
        Err(ClipError::NumericallyIndeterminate)
    );
    let tangent = region(&[
        shape([-1., -1., 2., 2., 1., 1., 1., 1.]),
        shape([1., -1., 2., 2., 1., 1., 1., 1.]),
    ]);
    assert_eq!(tangent.contains([1., 0.]), Ok(true));
    assert_eq!(tangent.has_positive_area(), Ok(false));
}
