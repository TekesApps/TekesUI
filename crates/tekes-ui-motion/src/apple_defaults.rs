//! Apple SwiftUI's published default configurations, checked 2026-10-15.
//!
//! These are reference configurations, not TekesUI's semantic design tokens.
//! `Default` follows the documented modern `.default` response of 0.55; it is
//! deliberately distinct from `.spring()` / `.smooth` (duration 0.5).
//! Single-property trajectories are mapped to this crate's oscillator/tween;
//! SwiftUI transaction, spring blending and callback semantics are not emulated.
use crate::{Motion, MotionOptions, MotionSpec, Spring, TimingCurve};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppleDefault {
    Default,
    Spring,
    Smooth,
    Snappy,
    Bouncy,
    EaseIn,
    EaseOut,
    EaseInOut,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Parameters {
    ResponseSpring {
        response: f64,
        damping_fraction: f64,
        blend_duration: f64,
    },
    Spring {
        duration: f64,
        bounce: f64,
    },
    Tween {
        duration: f64,
        control_points: [f64; 4],
    },
}

impl AppleDefault {
    pub const ALL: [Self; 9] = [
        Self::Default,
        Self::Spring,
        Self::Smooth,
        Self::Snappy,
        Self::Bouncy,
        Self::EaseIn,
        Self::EaseOut,
        Self::EaseInOut,
        Self::Linear,
    ];
    pub const fn id(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Spring => "spring",
            Self::Smooth => "smooth",
            Self::Snappy => "snappy",
            Self::Bouncy => "bouncy",
            Self::EaseIn => "ease-in",
            Self::EaseOut => "ease-out",
            Self::EaseInOut => "ease-in-out",
            Self::Linear => "linear",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Spring => "Spring",
            Self::Smooth => "Smooth",
            Self::Snappy => "Snappy",
            Self::Bouncy => "Bouncy",
            Self::EaseIn => "Ease In",
            Self::EaseOut => "Ease Out",
            Self::EaseInOut => "Ease In Out",
            Self::Linear => "Linear",
        }
    }
    pub const fn parameters(self) -> Parameters {
        match self {
            Self::Default => Parameters::ResponseSpring {
                response: 0.55,
                damping_fraction: 1.0,
                blend_duration: 0.0,
            },
            Self::Spring | Self::Smooth => Parameters::Spring {
                duration: 0.5,
                bounce: 0.0,
            },
            Self::Snappy => Parameters::Spring {
                duration: 0.5,
                bounce: 0.15,
            },
            Self::Bouncy => Parameters::Spring {
                duration: 0.5,
                bounce: 0.3,
            },
            Self::EaseIn => Parameters::Tween {
                duration: 0.35,
                control_points: [0.42, 0.0, 1.0, 1.0],
            },
            Self::EaseOut => Parameters::Tween {
                duration: 0.35,
                control_points: [0.0, 0.0, 0.58, 1.0],
            },
            Self::EaseInOut => Parameters::Tween {
                duration: 0.35,
                control_points: [0.42, 0.0, 0.58, 1.0],
            },
            Self::Linear => Parameters::Tween {
                duration: 0.35,
                control_points: [0.0, 0.0, 1.0, 1.0],
            },
        }
    }
    /// Official configuration source. Easing control points are independently
    /// documented in SwiftUI's `UnitCurve` type.
    pub const fn source_url(self) -> &'static str {
        match self {
            Self::Default => "https://developer.apple.com/documentation/swiftui/animation/default",
            Self::Spring => {
                "https://developer.apple.com/documentation/swiftui/animation/spring(duration:bounce:blendduration:)"
            }
            Self::Smooth => {
                "https://developer.apple.com/documentation/swiftui/animation/smooth(duration:extrabounce:)"
            }
            Self::Snappy => {
                "https://developer.apple.com/documentation/swiftui/animation/snappy(duration:extrabounce:)"
            }
            Self::Bouncy => {
                "https://developer.apple.com/documentation/swiftui/animation/bouncy(duration:extrabounce:)"
            }
            Self::EaseIn => "https://developer.apple.com/documentation/swiftui/animation/easein",
            Self::EaseOut => "https://developer.apple.com/documentation/swiftui/animation/easeout",
            Self::EaseInOut => {
                "https://developer.apple.com/documentation/swiftui/animation/easeinout"
            }
            Self::Linear => "https://developer.apple.com/documentation/swiftui/animation/linear",
        }
    }
    pub fn motion(self) -> Motion {
        let (spec, logical_duration) = match self.parameters() {
            Parameters::ResponseSpring {
                response,
                damping_fraction,
                ..
            } => (
                MotionSpec::Spring(
                    Spring::new(1.0 / response, damping_fraction).expect("published Apple default"),
                ),
                None,
            ),
            Parameters::Spring { duration, bounce } => (
                MotionSpec::Spring(
                    Spring::new(1.0 / duration, 1.0 - bounce).expect("published Apple default"),
                ),
                Some(Duration::from_secs_f64(duration)),
            ),
            Parameters::Tween {
                duration,
                control_points: [x1, y1, x2, y2],
            } => (
                MotionSpec::Tween {
                    duration: Duration::from_secs_f64(duration),
                    curve: TimingCurve::cubic_bezier(x1, y1, x2, y2).expect("published UnitCurve"),
                },
                None,
            ),
        };
        Motion::new(spec).with_options(MotionOptions {
            logical_duration,
            ..MotionOptions::default()
        })
    }
}
