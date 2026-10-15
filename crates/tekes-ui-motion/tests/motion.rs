use std::time::Duration;
use tekes_ui_motion::{
    Animated, CancelBehavior, CancelReason, EventKind, FinishReason, Motion, MotionError,
    MotionOptions, MotionSpec, Spring, TimingCurve, presets,
};

fn time(seconds: f64) -> Duration {
    Duration::from_secs_f64(seconds)
}
fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected} (tolerance {tolerance})"
    );
}
fn oscillator(hz: f64, damping: f64) -> MotionSpec {
    MotionSpec::Spring(Spring::new(hz, damping).unwrap())
}

#[test]
fn bezier_inverts_time_instead_of_treating_it_as_parameter() {
    // x(t)=t^3 and y(t)=3t^2-2t^3, so x=1/8 -> t=1/2 -> y=1/2.
    let curve = TimingCurve::cubic_bezier(0.0, 0.0, 0.0, 1.0).unwrap();
    let (value, slope) = curve.evaluate(0.125).unwrap();
    close(value, 0.5, 1e-12);
    close(slope, 2.0, 1e-12);
}

#[test]
fn bounded_curves_and_endpoint_slopes() {
    for points in [
        [0.2, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 0.0],
    ] {
        let curve = TimingCurve::cubic_bezier(points[0], points[1], points[2], points[3]).unwrap();
        for i in 0..=100 {
            let (p, _) = curve.evaluate(f64::from(i) / 100.0).unwrap();
            assert!((0.0..=1.0).contains(&p));
        }
        close(curve.evaluate(0.0).unwrap().0, 0.0, 0.0);
        close(curve.evaluate(1.0).unwrap().0, 1.0, 0.0);
    }
    let curve = TimingCurve::cubic_bezier(0.0, 0.0, 1.0, 1.0).unwrap();
    close(curve.evaluate(0.0).unwrap().1, 1.0, 0.0);
    close(curve.evaluate(1.0).unwrap().1, 1.0, 0.0);
}

#[test]
fn invalid_parameters_are_rejected() {
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1e300, 1e-300] {
        assert!(Spring::new(value, 1.0).is_err());
    }
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        assert!(Spring::new(4.5, value).is_err());
    }
    assert!(TimingCurve::cubic_bezier(-0.1, 0.0, 1.0, 1.0).is_err());
    assert!(TimingCurve::cubic_bezier(0.0, 0.0, 1.0, f64::NAN).is_err());
    assert!(TimingCurve::LINEAR.evaluate(f64::NAN).is_err());
    assert!(Animated::new([0.0, f64::INFINITY]).is_err());
}

#[test]
fn critical_spring_matches_closed_form() {
    let spring = oscillator(1.0 / std::f64::consts::TAU, 1.0); // omega=1
    let sample = spring.sample(0.0, 1.0, 0.0, time(1.0)).unwrap();
    close(sample.value, 1.0 - 2.0 / std::f64::consts::E, 1e-14);
    close(sample.velocity, 1.0 / std::f64::consts::E, 1e-14);
}

#[test]
fn physical_parameter_mapping_preserves_the_differential_equation() {
    let (mass, stiffness, damping) = (0.1, 1200.0, 80.0);
    let spring = Spring::from_physical(mass, stiffness, damping).unwrap();
    let omega = spring.frequency_hz() * std::f64::consts::TAU;
    close(omega * omega, stiffness / mass, 1e-10);
    close(2.0 * spring.damping_ratio() * omega, damping / mass, 1e-10);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Spring::from_physical(bad, stiffness, damping).is_err());
        assert!(Spring::from_physical(mass, bad, damping).is_err());
        assert!(Spring::from_physical(mass, stiffness, bad).is_err());
    }
}

// Independent numerical reference: RK4 integrates x'=v, v'=-2ζωv-ω²x.
// It does not reuse the closed form used by the production sampler.
fn rk4(hz: f64, z: f64, total: f64) -> (f64, f64) {
    let w = std::f64::consts::TAU * hz;
    let derivative = |x: f64, v: f64| (v, -2.0 * z * w * v - w * w * x);
    let (mut x, mut v) = (-2.5, 3.0);
    let steps = 20_000;
    let dt = total / f64::from(steps);
    for _ in 0..steps {
        let a = derivative(x, v);
        let b = derivative(x + a.0 * dt / 2.0, v + a.1 * dt / 2.0);
        let c = derivative(x + b.0 * dt / 2.0, v + b.1 * dt / 2.0);
        let d = derivative(x + c.0 * dt, v + c.1 * dt);
        x += dt * (a.0 + 2.0 * b.0 + 2.0 * c.0 + d.0) / 6.0;
        v += dt * (a.1 + 2.0 * b.1 + 2.0 * c.1 + d.1) / 6.0;
    }
    (x, v)
}

#[test]
fn all_damping_regimes_match_independent_integrator() {
    for z in [0.1, 0.75, 0.999999999, 1.0, 1.000000001, 1.5, 10.0] {
        for t in [0.001, 0.13, 0.7] {
            let actual = oscillator(2.0, z).sample(-0.5, 2.0, 3.0, time(t)).unwrap();
            let expected = rk4(2.0, z, t);
            close(actual.value, 2.0 + expected.0, 1e-9);
            close(actual.velocity, expected.1, 1e-8);
        }
    }
}

#[test]
fn vector_sampling_matches_scalar_channels() {
    let spec = oscillator(3.0, 0.8);
    let vector = spec
        .sample([0.0, 50.0], [100.0, -10.0], [5.0, -2.0], time(0.15))
        .unwrap();
    for (i, (a, b, v)) in [(0.0, 100.0, 5.0), (50.0, -10.0, -2.0)]
        .into_iter()
        .enumerate()
    {
        let scalar = spec.sample(a, b, v, time(0.15)).unwrap();
        close(vector.value[i], scalar.value, 0.0);
        close(vector.velocity[i], scalar.velocity, 0.0);
    }
}

#[test]
fn derivatives_match_finite_differences() {
    for z in [0.1, 1.0, 1.000000001, 2.0] {
        let spec = oscillator(3.0, z);
        for t in [0.03, 0.15, 0.4] {
            let a = spec.sample(0.0, 100.0, 23.0, time(t - 1e-6)).unwrap();
            let b = spec.sample(0.0, 100.0, 23.0, time(t + 1e-6)).unwrap();
            let middle = spec.sample(0.0, 100.0, 23.0, time(t)).unwrap();
            close(middle.velocity, (b.value - a.value) / 2e-6, 1e-6);
        }
    }
}

#[test]
fn retarget_preserves_position_and_velocity_and_cancels_old_generation() {
    let mut channel = Animated::new([0.0, 2.0]).unwrap();
    channel
        .animate_to([100.0, -20.0], presets::selection(), Duration::ZERO)
        .unwrap();
    let before = channel.advance(time(0.11)).unwrap();
    let after = channel
        .animate_to([-50.0, 30.0], presets::geometry(), time(0.11))
        .unwrap();
    for i in 0..2 {
        close(after.sample.value[i], before.sample.value[i], 1e-12);
        close(after.sample.velocity[i], before.sample.velocity[i], 1e-12);
    }
    assert_eq!(after.generation, 2);
    assert!(
        after
            .events
            .iter()
            .any(|e| e.generation == 1 && e.kind == EventKind::Cancelled(CancelReason::Retargeted))
    );
    let end = channel.advance(time(3.0)).unwrap();
    assert_eq!(end.sample.value, [-50.0, 30.0]);
    assert!(end.events.iter().all(|e| e.generation == 2));
}

#[test]
fn release_velocity_and_rapid_reversals_are_continuous() {
    let mut channel = Animated::new(0.0).unwrap();
    let first = channel
        .animate_to_with_velocity(100.0, -50.0, presets::geometry(), Duration::ZERO)
        .unwrap();
    assert_eq!(first.sample.velocity, -50.0);
    for tick in 1..=100 {
        let now = time(f64::from(tick) / 100.0);
        let before = channel.advance(now).unwrap();
        let target = if tick % 2 == 0 { 100.0 } else { 0.0 };
        let after = channel
            .animate_to(target, presets::geometry(), now)
            .unwrap();
        close(after.sample.value, before.sample.value, 1e-12);
        close(after.sample.velocity, before.sample.velocity, 1e-12);
    }
    let end = channel.advance(time(4.0)).unwrap();
    assert_eq!(end.sample.value, 100.0);
    assert!(!end.needs_frame);
}

#[test]
fn absolute_time_sampling_does_not_depend_on_frame_rate() {
    fn run(hz: u32) -> (f64, f64) {
        let mut x = Animated::new(0.0).unwrap();
        x.animate_to(100.0, presets::geometry(), Duration::ZERO)
            .unwrap();
        for i in 0..hz / 10 {
            x.advance(time(f64::from(i) / f64::from(hz))).unwrap();
        }
        let frame = x.advance(time(0.15)).unwrap();
        (frame.sample.value, frame.sample.velocity)
    }
    assert_eq!(run(60), run(120));
}

#[test]
fn logical_and_settled_events_fire_once_and_idle_stops() {
    let mut x = Animated::new(0.0).unwrap();
    x.animate_to(100.0, presets::geometry(), Duration::ZERO)
        .unwrap();
    let logical = x.advance(time(0.18)).unwrap();
    assert_eq!(logical.events.len(), 1);
    assert_eq!(logical.events[0].kind, EventKind::LogicalComplete);
    assert!(logical.needs_frame);
    let settled = x.advance(time(2.0)).unwrap();
    assert_eq!(settled.events.len(), 1);
    assert_eq!(
        settled.events[0].kind,
        EventKind::Settled(FinishReason::Converged)
    );
    assert!(!settled.needs_frame);
    assert_eq!(settled.sample.value, 100.0);
    assert_eq!(settled.sample.velocity, 0.0);
    assert!(x.advance(time(3.0)).unwrap().events.is_empty());
}

#[test]
fn oscillation_crossing_does_not_prematurely_settle() {
    let mut x = Animated::new(0.0).unwrap();
    let motion = Motion::new(oscillator(1.0, 0.05)).with_options(MotionOptions {
        position_tolerance: 0.1,
        velocity_tolerance: 0.01,
        ..MotionOptions::default()
    });
    x.animate_to(1.0, motion, Duration::ZERO).unwrap();
    let frame = x.advance(time(0.26)).unwrap();
    assert!((frame.sample.value - 1.0).abs() < 0.1);
    assert!(frame.sample.velocity.abs() > 1.0);
    assert!(frame.needs_frame);
}

#[test]
fn energy_bound_covers_future_trajectory_after_settling() {
    for z in [0.1, 0.75, 1.0, 2.0] {
        let spec = oscillator(2.0, z);
        let mut x = Animated::new(0.0).unwrap();
        x.animate_to(1.0, Motion::new(spec), Duration::ZERO)
            .unwrap();
        for tick in 1..10_000 {
            let now = f64::from(tick) / 1000.0;
            let frame = x.advance(time(now)).unwrap();
            if !frame.needs_frame {
                assert!(
                    !frame
                        .events
                        .iter()
                        .any(|e| e.kind == EventKind::Settled(FinishReason::Deadline))
                );
                for future in 0..1000 {
                    let s = spec
                        .sample(0.0, 1.0, 0.0, time(now + f64::from(future) / 100.0))
                        .unwrap();
                    assert!((s.value - 1.0).abs() <= 1e-4);
                    assert!(s.velocity.abs() <= 1e-3);
                }
                break;
            }
            assert!(tick < 9_999, "did not converge");
        }
    }
}

#[test]
fn instant_zero_duration_and_tween_end_are_exact() {
    for spec in [
        MotionSpec::Instant,
        MotionSpec::Tween {
            duration: Duration::ZERO,
            curve: TimingCurve::LINEAR,
        },
    ] {
        let mut x = Animated::new(0.0).unwrap();
        let frame = x
            .animate_to(10.0, Motion::new(spec), Duration::ZERO)
            .unwrap();
        assert_eq!(frame.sample.value, 10.0);
        assert_eq!(frame.events.len(), 2);
        assert!(!frame.needs_frame);
    }
    let mut x = Animated::new(0.0).unwrap();
    x.animate_to(10.0, presets::feedback(), Duration::ZERO)
        .unwrap();
    let end = x.advance(time(0.12)).unwrap();
    assert_eq!(end.sample.value, 10.0);
    assert_eq!(end.sample.velocity, 0.0);
    assert_eq!(
        end.events[1].kind,
        EventKind::Settled(FinishReason::DurationElapsed)
    );
}

#[test]
fn cancellation_holds_or_finishes_without_success_events() {
    for behavior in [CancelBehavior::Hold, CancelBehavior::Finish] {
        let mut x = Animated::new(0.0).unwrap();
        x.animate_to(100.0, presets::geometry(), Duration::ZERO)
            .unwrap();
        let before = x.advance(time(0.1)).unwrap();
        let frame = x
            .cancel(time(0.1), CancelReason::ReducedMotion, behavior)
            .unwrap();
        assert_eq!(frame.events.len(), 1);
        assert_eq!(
            frame.events[0].kind,
            EventKind::Cancelled(CancelReason::ReducedMotion)
        );
        assert_eq!(
            frame.sample.value,
            if behavior == CancelBehavior::Hold {
                before.sample.value
            } else {
                100.0
            }
        );
        assert_eq!(frame.sample.velocity, 0.0);
        assert!(!frame.needs_frame);
        assert!(
            x.cancel(time(0.1), CancelReason::Explicit, behavior)
                .unwrap()
                .events
                .is_empty()
        );
    }
}

#[test]
fn invalid_updates_are_atomic_and_time_never_rewinds() {
    let mut x = Animated::new(0.0).unwrap();
    x.animate_to(100.0, presets::geometry(), Duration::ZERO)
        .unwrap();
    let frame = x.advance(time(0.1)).unwrap();
    assert_eq!(
        x.animate_to(f64::NAN, presets::geometry(), time(0.2))
            .unwrap_err(),
        MotionError::NonFiniteValue
    );
    let invalid = Motion::new(MotionSpec::Instant).with_options(MotionOptions {
        position_tolerance: 0.0,
        ..MotionOptions::default()
    });
    assert_eq!(
        x.animate_to(1.0, invalid, time(0.2)).unwrap_err(),
        MotionError::InvalidOptions
    );
    assert_eq!(x.current(), frame.sample);
    assert_eq!(x.generation(), 1);
    assert_eq!(x.target(), 100.0);
    assert!(x.advance(time(0.15)).is_ok()); // Failed updates did not advance time.
    assert_eq!(
        x.advance(time(0.14)).unwrap_err(),
        MotionError::TimeWentBackwards
    );
    assert_eq!(
        x.cancel(time(0.14), CancelReason::Explicit, CancelBehavior::Finish)
            .unwrap_err(),
        MotionError::TimeWentBackwards
    );
}

#[test]
fn deadline_stops_slow_springs_and_long_tweens_even_after_time_jump() {
    for spec in [
        oscillator(0.01, 1.0),
        MotionSpec::Tween {
            duration: time(20.0),
            curve: TimingCurve::LINEAR,
        },
    ] {
        let mut x = Animated::new(0.0).unwrap();
        x.animate_to(100.0, Motion::new(spec), Duration::ZERO)
            .unwrap();
        let end = x.advance(time(100.0)).unwrap();
        assert_eq!(end.sample.value, 100.0);
        assert!(!end.needs_frame);
        assert_eq!(
            end.events[1].kind,
            EventKind::Settled(FinishReason::Deadline)
        );
    }
}

#[test]
fn extreme_damping_and_near_critical_values_remain_finite() {
    for z in [1.0 - 1e-14, 1.0, 1.0 + 1e-14, 1e100] {
        let spec = oscillator(4.5, z);
        let initial = spec.sample(0.0, 100.0, 3.0, Duration::ZERO).unwrap();
        assert_eq!(initial.value, 0.0);
        assert_eq!(initial.velocity, 3.0);
        let sample = spec.sample(0.0, 100.0, 3.0, time(0.1)).unwrap();
        assert!(sample.value.is_finite() && sample.velocity.is_finite());
    }
}

#[test]
fn stationary_spring_needs_no_extra_frame() {
    let mut x = Animated::new(4.0).unwrap();
    let frame = x
        .animate_to(4.0, presets::geometry(), Duration::ZERO)
        .unwrap();
    assert!(!frame.needs_frame);
    assert_eq!(frame.events.len(), 2);
}

#[test]
fn numerical_failure_does_not_corrupt_the_channel() {
    let mut x = Animated::new(0.0).unwrap();
    let bad_tangent = TimingCurve::cubic_bezier(0.0, 1.0, 0.0, 1.0).unwrap();
    let motion = Motion::new(MotionSpec::Tween {
        duration: time(1.0),
        curve: bad_tangent,
    });
    assert_eq!(
        x.animate_to(1.0, motion, Duration::ZERO).unwrap_err(),
        MotionError::NumericalOverflow
    );
    assert_eq!(x.current().value, 0.0);
    assert_eq!(x.generation(), 0);
    assert!(!x.is_active());
    assert!(
        MotionSpec::Instant
            .sample(-f64::MAX, f64::MAX, 0.0, Duration::ZERO)
            .is_err()
    );
}

#[test]
fn spring_initial_sample_preserves_small_position_with_large_target() {
    let sample = oscillator(4.5, 1.0)
        .sample(0.25, 1e16, -2.0, Duration::ZERO)
        .unwrap();
    assert_eq!(sample.value, 0.25);
    assert_eq!(sample.velocity, -2.0);
}

#[test]
fn interior_infinite_slope_is_rejected_but_degenerate_identity_is_valid() {
    assert_eq!(
        TimingCurve::cubic_bezier(1.0, 0.2, 0.0, 0.8),
        Err(MotionError::InvalidCurve)
    );
    let identity = TimingCurve::cubic_bezier(1.0, 1.0, 0.0, 0.0).unwrap();
    let (value, slope) = identity.evaluate(0.5).unwrap();
    close(value, 0.5, 1e-12);
    close(slope, 1.0, 1e-12);
}

#[test]
fn tween_retarget_is_position_continuous_but_uses_its_own_velocity() {
    let mut x = Animated::new(0.0).unwrap();
    x.animate_to(100.0, presets::geometry(), Duration::ZERO)
        .unwrap();
    let before = x.advance(time(0.1)).unwrap();
    let after = x.animate_to(-10.0, presets::feedback(), time(0.1)).unwrap();
    assert_eq!(after.sample.value, before.sample.value);
    assert_eq!(after.sample.velocity, 0.0);
    assert_ne!(after.sample.velocity, before.sample.velocity);
}
