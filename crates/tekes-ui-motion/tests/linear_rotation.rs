use std::time::Duration;
use tekes_ui_motion::{
    linear_rotation::LinearRotation,
    recipes::{RecipeKind, RecipePlayer},
};
#[test]
fn constant_angular_speed_survives_pause_reverse_and_parameter_edits() {
    let mut motor = LinearRotation::new(0.5).unwrap();
    motor.resume(Duration::ZERO).unwrap();
    let at = motor.advance(Duration::from_secs(4)).unwrap();
    assert_eq!(at.value, 2.0);
    assert_eq!(at.velocity, 0.5);
    motor.pause(Duration::from_secs(4)).unwrap();
    motor.advance(Duration::from_secs(10)).unwrap();
    assert_eq!(motor.sample().value, 2.0);
    assert_eq!(motor.sample().velocity, 0.0);
    motor.resume(Duration::from_secs(10)).unwrap();
    motor.set_speed(-0.25, Duration::from_secs(10)).unwrap();
    assert_eq!(motor.sample().value, 2.0);
    assert_eq!(motor.advance(Duration::from_secs(12)).unwrap().value, 1.5);
    let mut a = LinearRotation::new(0.37).unwrap();
    a.resume(Duration::ZERO).unwrap();
    let mut b = a.clone();
    a.advance(Duration::from_secs(3)).unwrap();
    for t in 1..=360 {
        b.advance(Duration::from_secs_f64(t as f64 / 120.0))
            .unwrap();
    }
    assert!((a.sample().value - b.sample().value).abs() < 1e-12);
    let saved = a.sample();
    assert!(a.set_speed(f64::NAN, Duration::from_secs(4)).is_err());
    assert_eq!(a.sample(), saved);
    assert!(a.advance(Duration::from_secs(2)).is_err());
    assert_eq!(a.sample(), saved);
}
#[test]
fn linear_recipe_retains_unwrapped_angle_and_disables_frames_when_paused() {
    let mut p = RecipePlayer::new(RecipeKind::LinearRotation);
    p.replay().unwrap();
    p.advance(Duration::from_secs(5)).unwrap();
    assert_eq!(p.values()[0], 2.5);
    assert_eq!(p.velocities()[0], 0.5);
    let angle = p.values()[0];
    p.set_target(false).unwrap();
    assert_eq!(p.values()[0], angle);
    assert_eq!(p.velocities()[0], -0.5);
    let mut params = p.parameters;
    params.duration = 1.0;
    p.set_parameters(params).unwrap();
    assert_eq!(p.values()[0], angle);
    assert_eq!(p.velocities()[0], -1.0);
    p.advance(Duration::from_secs(6)).unwrap();
    assert_eq!(p.values()[0], 1.5);
    p.interrupt().unwrap();
    p.advance(Duration::from_secs(12)).unwrap();
    assert_eq!(p.values()[0], 1.5);
    assert!(!p.needs_frame());
    p.resume().unwrap();
    p.advance(Duration::from_secs(13)).unwrap();
    assert_eq!(p.values()[0], 0.5);
    p.set_reduced_motion(true).unwrap();
    let held = p.values()[0];
    assert!(!p.needs_frame());
    p.advance(Duration::from_secs(15)).unwrap();
    assert_eq!(p.values()[0], held);
    p.set_reduced_motion(false).unwrap();
    assert!(!p.needs_frame());
    p.resume().unwrap();
    assert!(p.needs_frame());
}
