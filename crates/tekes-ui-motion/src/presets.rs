//! TekesUI tuning starting points, NOT measured Apple control constants.
//! Components can override options with property-specific units/tolerances.
use crate::{Motion, MotionOptions, MotionSpec, Spring, TimingCurve};
use std::time::Duration;

fn tween(milliseconds: u64) -> Motion {
    Motion::new(MotionSpec::Tween {
        duration: Duration::from_millis(milliseconds),
        curve: TimingCurve::cubic_bezier(0.2, 0.0, 0.0, 1.0).expect("valid preset curve"),
    })
}

fn spring(hz: f64, damping: f64, logical_ms: u64) -> Motion {
    Motion::new(MotionSpec::Spring(
        Spring::new(hz, damping).expect("valid preset spring"),
    ))
    .with_options(MotionOptions {
        logical_duration: Some(Duration::from_millis(logical_ms)),
        ..MotionOptions::default()
    })
}

pub fn instant() -> Motion {
    Motion::new(MotionSpec::Instant)
}
pub fn feedback() -> Motion {
    tween(120)
}
pub fn fade_in() -> Motion {
    tween(120)
}
pub fn fade_out() -> Motion {
    tween(100)
}
pub fn geometry() -> Motion {
    spring(4.5, 1.0, 180)
}
pub fn selection() -> Motion {
    spring(4.5, 0.9, 180)
}
pub fn disclosure() -> Motion {
    spring(3.5, 1.0, 300)
}
pub fn surface() -> Motion {
    spring(3.0, 1.0, 300)
}
pub fn celebrate() -> Motion {
    spring(3.0, 0.75, 500)
}
