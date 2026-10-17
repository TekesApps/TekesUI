use std::time::Duration;
use tekes_ui_motion::{
    recipes::{RecipeKind, RecipePlayer},
    symbols::*,
};

#[test]
fn all_effects_have_bounded_frames_in_all_scopes_and_reduction() {
    for effect in SymbolEffect::ALL {
        for scope in [
            SymbolScope::Whole,
            SymbolScope::ByLayer,
            SymbolScope::Individually,
        ] {
            for reduced in [false, true] {
                for tick in 0..=120 {
                    for layer in 0..3 {
                        let f = sample(
                            effect,
                            f64::from(tick) / 120.0,
                            layer,
                            3,
                            SymbolOptions {
                                scope,
                                ..Default::default()
                            },
                            reduced,
                        )
                        .unwrap();
                        for value in [
                            f.scale,
                            f.rotation,
                            f.offset_y,
                            f.opacity,
                            f.draw_start,
                            f.draw_end,
                            f.replacement,
                        ] {
                            assert!(value.is_finite());
                        }
                        assert!((0.0..=1.0).contains(&f.opacity));
                        assert!(
                            f.draw_start >= 0.0 && f.draw_end <= 1.0 && f.draw_start <= f.draw_end
                        );
                        if reduced {
                            assert_eq!((f.scale, f.rotation, f.offset_y), (1.0, 0.0, 0.0));
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn scope_order_and_ordered_path_direction_are_observable() {
    let opts = SymbolOptions {
        scope: SymbolScope::Individually,
        ..Default::default()
    };
    assert_eq!(
        sample(SymbolEffect::DrawOn, 0.5, 0, 3, opts, false)
            .unwrap()
            .draw_end,
        1.0
    );
    assert_eq!(
        sample(SymbolEffect::DrawOn, 0.5, 1, 3, opts, false)
            .unwrap()
            .draw_end,
        0.5
    );
    assert_eq!(
        sample(SymbolEffect::DrawOn, 0.5, 2, 3, opts, false)
            .unwrap()
            .draw_end,
        0.0
    );
    let f = sample(
        SymbolEffect::DrawOff,
        0.25,
        0,
        1,
        SymbolOptions::default(),
        false,
    )
    .unwrap();
    assert_eq!((f.draw_start, f.draw_end), (0.0, 0.75));
    let f = sample(
        SymbolEffect::DrawOff,
        0.25,
        0,
        1,
        SymbolOptions {
            draw_reversed: false,
            ..Default::default()
        },
        false,
    )
    .unwrap();
    assert_eq!((f.draw_start, f.draw_end), (0.25, 1.0));
    let opts = SymbolOptions {
        cumulative: true,
        ..Default::default()
    };
    assert_eq!(
        sample(SymbolEffect::VariableColor, 1.0, 0, 3, opts, false)
            .unwrap()
            .opacity,
        1.0
    );
    assert_eq!(
        sample(SymbolEffect::VariableColor, 1.0, 2, 3, opts, false)
            .unwrap()
            .opacity,
        1.0
    );
}
#[test]
fn missing_asset_annotations_resolve_to_explicit_fallbacks() {
    let plain = SymbolCapabilities::default();
    assert_eq!(
        SymbolEffect::MagicReplace.resolve(plain),
        SymbolEffect::Replace
    );
    assert_eq!(SymbolEffect::DrawOn.resolve(plain), SymbolEffect::Appear);
    assert_eq!(
        SymbolEffect::DrawOff.resolve(plain),
        SymbolEffect::Disappear
    );
    assert_eq!(
        SymbolEffect::VariableDraw.resolve(plain),
        SymbolEffect::VariableColor
    );
    let capable = SymbolCapabilities {
        ordered_paths: true,
        correspondence: true,
    };
    for e in SymbolEffect::ALL {
        assert_eq!(e.resolve(capable), e);
    }
}
#[test]
fn symbol_runs_pause_resume_reverse_and_finish_without_background_frames() {
    for effect in SymbolEffect::ALL {
        let mut p = RecipePlayer::new(RecipeKind::Symbol(effect));
        p.replay().unwrap();
        p.advance(Duration::from_millis(400)).unwrap();
        let at = p.values()[0];
        assert!(at > 0.0 && at < 1.0);
        p.interrupt().unwrap();
        p.advance(Duration::from_secs(3)).unwrap();
        assert_eq!(p.values()[0], at);
        assert!(!p.needs_frame());
        p.resume().unwrap();
        assert_eq!(p.values()[0], at);
        p.advance(Duration::from_millis(3300)).unwrap();
        let at = p.values()[0];
        p.set_target(false).unwrap();
        assert_eq!(p.values()[0], at);
        p.advance(Duration::from_secs(8)).unwrap();
        assert_eq!(p.values()[0], 0.0);
        assert!(!p.needs_frame());
        p.replay().unwrap();
        p.set_reduced_motion(true).unwrap();
        assert_eq!(p.values()[0], 1.0);
        assert!(!p.needs_frame());
    }
}
#[test]
fn symbol_progress_is_independent_of_frame_partition_and_rejects_invalid_input() {
    let mut a = RecipePlayer::new(RecipeKind::Symbol(SymbolEffect::Breathe));
    a.replay().unwrap();
    let mut b = a.clone();
    a.advance(Duration::from_millis(900)).unwrap();
    for t in 1..=90 {
        b.advance(Duration::from_millis(t * 10)).unwrap();
    }
    assert_eq!(a.values(), b.values());
    for p in [f64::NAN, f64::INFINITY] {
        assert!(sample(SymbolEffect::Pulse, p, 0, 1, Default::default(), false).is_err());
    }
    assert!(sample(SymbolEffect::Pulse, 0.5, 3, 3, Default::default(), false).is_err());
    assert!(
        sample(
            SymbolEffect::Pulse,
            0.5,
            0,
            1,
            SymbolOptions {
                cycles: 0,
                ..Default::default()
            },
            false
        )
        .is_err()
    );
}

#[test]
fn wiggle_supports_directional_translation_and_rotation_has_handedness() {
    let p = SymbolOptions {
        wiggle_translation_angle: Some(0.0),
        ..Default::default()
    };
    let f = sample(SymbolEffect::Wiggle, 0.2, 0, 1, p, false).unwrap();
    assert!(f.offset_x.abs() > 0.1);
    assert_eq!(f.offset_y, 0.0);
    assert_eq!(f.rotation, 0.0);
    let a = sample(SymbolEffect::Rotate, 0.2, 0, 1, Default::default(), false).unwrap();
    let b = sample(
        SymbolEffect::Rotate,
        0.2,
        0,
        1,
        SymbolOptions {
            direction: -1.0,
            ..Default::default()
        },
        false,
    )
    .unwrap();
    assert_eq!(a.rotation, -b.rotation);
}
