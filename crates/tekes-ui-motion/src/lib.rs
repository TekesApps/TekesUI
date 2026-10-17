//! Deterministic animation math and interruptible value channels.
//!
//! The caller supplies monotonic [`std::time::Duration`] timestamps relative to
//! its own epoch. This crate does not read a clock, create threads, schedule
//! frames, access platform preferences, or own components.
//!
//! ```
//! use std::time::Duration;
//! use tekes_ui_motion::{Animated, presets};
//! let mut x = Animated::new(0.0)?;
//! x.animate_to(100.0, presets::geometry(), Duration::ZERO)?;
//! let frame = x.advance(Duration::from_millis(100))?;
//! assert!(frame.sample.value > 0.0 && frame.sample.value < 100.0);
//! // Retargeting carries the current position and velocity into a new spring.
//! x.animate_to(25.0, presets::geometry(), Duration::from_millis(100))?;
//! # Ok::<(), tekes_ui_motion::MotionError>(())
//! ```

pub mod apple_defaults;
mod channel;
mod curve;
pub mod linear_rotation;
mod model;
pub mod presets;
pub mod recipes;
pub mod state_transition;
pub mod symbols;
pub mod typing;
mod value;

pub use channel::{Animated, CancelBehavior, CancelReason, Event, EventKind, FinishReason, Frame};
pub use curve::TimingCurve;
pub use model::{Motion, MotionOptions, MotionSpec, Sample, Spring};
pub use value::MotionValue;

/// Invalid input never partially changes an animation channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionError {
    NonFiniteValue,
    InvalidCurve,
    InvalidSpring,
    InvalidOptions,
    TimeWentBackwards,
    GenerationExhausted,
    NumericalOverflow,
}

impl std::fmt::Display for MotionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NonFiniteValue => "animation values must be finite",
            Self::InvalidCurve => "Bezier controls must be finite and in [0, 1]",
            Self::InvalidSpring => "spring frequency and damping must be positive and numerically representable",
            Self::InvalidOptions => "tolerances and maximum duration must be positive; logical duration must not exceed the deadline",
            Self::TimeWentBackwards => "animation timestamps must be monotonic",
            Self::GenerationExhausted => "animation generation counter exhausted",
            Self::NumericalOverflow => "animation sample exceeds the representable numeric range",
        })
    }
}

impl std::error::Error for MotionError {}
