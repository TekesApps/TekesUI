use crate::{MotionError, MotionValue, TimingCurve};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample<V> {
    pub value: V,
    pub velocity: V,
}

/// A finite damped oscillator. These are physical parameters, not Apple's
/// perceptual duration/bounce parameterization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    frequency_hz: f64,
    damping_ratio: f64,
}

impl Spring {
    /// Converts mass/stiffness/damping to the equivalent oscillator. No Apple
    /// perceptual-duration or bounce mapping is implied.
    pub fn from_physical(mass: f64, stiffness: f64, damping: f64) -> Result<Self, MotionError> {
        if [mass, stiffness, damping]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(MotionError::InvalidSpring);
        }
        let omega = (stiffness / mass).sqrt();
        Self::new(
            omega / std::f64::consts::TAU,
            (damping / mass) / (2.0 * omega),
        )
    }

    pub fn new(frequency_hz: f64, damping_ratio: f64) -> Result<Self, MotionError> {
        let omega = frequency_hz * std::f64::consts::TAU;
        if !frequency_hz.is_finite()
            || frequency_hz <= 0.0
            || !damping_ratio.is_finite()
            || damping_ratio <= 0.0
            || !(omega * omega).is_finite()
            || omega * omega == 0.0
            || !(2.0 * omega * damping_ratio).is_finite()
        {
            return Err(MotionError::InvalidSpring);
        }
        Ok(Self {
            frequency_hz,
            damping_ratio,
        })
    }

    pub fn frequency_hz(self) -> f64 {
        self.frequency_hz
    }
    pub fn damping_ratio(self) -> f64 {
        self.damping_ratio
    }
    pub(crate) fn omega(self) -> f64 {
        self.frequency_hz * std::f64::consts::TAU
    }

    fn scalar(self, displacement: f64, velocity: f64, t: f64) -> (f64, f64) {
        if t == 0.0 {
            return (displacement, velocity);
        }
        let w = self.omega();
        let z = self.damping_ratio;
        if z < 1.0 {
            let a = z * w;
            let b = w * ((1.0 - z) * (1.0 + z)).sqrt();
            let bt = b * t;
            let sinc = if bt.abs() < 1e-4 {
                1.0 - bt * bt / 6.0 + bt.powi(4) / 120.0
            } else {
                bt.sin() / bt
            };
            let decay = (-a * t).exp();
            if decay == 0.0 {
                return (0.0, 0.0);
            }
            let s = t * sinc;
            (
                decay * (displacement * bt.cos() + (velocity + a * displacement) * s),
                decay * (velocity * bt.cos() - (a * velocity + w * w * displacement) * s),
            )
        } else {
            // Stable real roots; exp_m1 removes cancellation near z=1.
            let root = (1.0 - (1.0 / z).powi(2)).sqrt();
            let slow = -(w / z) / (1.0 + root);
            let fast = -(w * z) * (1.0 + root);
            let d = fast - slow;
            let decay = (slow * t).exp();
            if decay == 0.0 {
                return (0.0, 0.0);
            }
            let phi = if d == 0.0 { t } else { (d * t).exp_m1() / d };
            let q = velocity - slow * displacement;
            let y = displacement + q * phi;
            (decay * y, decay * (slow * y + q * (d * t).exp()))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionSpec {
    Instant,
    Tween {
        duration: Duration,
        curve: TimingCurve,
    },
    Spring(Spring),
}

impl MotionSpec {
    /// Stateless, seekable sampling. No settling or deadline truncation occurs
    /// here; channels apply those policies. Tween ignores initial velocity.
    pub fn sample<V: MotionValue>(
        self,
        from: V,
        target: V,
        initial_velocity: V,
        elapsed: Duration,
    ) -> Result<Sample<V>, MotionError> {
        validate_values(from, target, initial_velocity)?;
        let sample = match self {
            Self::Instant => Sample {
                value: target,
                velocity: target.zero(),
            },
            Self::Tween { duration, curve } if duration.is_zero() || elapsed >= duration => {
                let _ = curve;
                Sample {
                    value: target,
                    velocity: target.zero(),
                }
            }
            Self::Tween { duration, curve } => {
                let seconds = duration.as_secs_f64();
                let (p, slope) = curve.evaluate(elapsed.as_secs_f64() / seconds)?;
                let delta = target.zip_map(from, |a, b| a - b);
                Sample {
                    value: from.zip_map(delta, |a, d| a + d * p),
                    // A degenerate Bezier may have infinite endpoint speed.
                    // Report overflow instead of passing NaN to consumers.
                    velocity: delta.map(|d| if d == 0.0 { 0.0 } else { d * slope / seconds }),
                }
            }
            Self::Spring(spring) => {
                if elapsed.is_zero() {
                    return Ok(Sample {
                        value: from,
                        velocity: initial_velocity,
                    });
                }
                let y = from.zip_map(target, |a, b| a - b);
                let t = elapsed.as_secs_f64();
                let displacement = y.zip_map(initial_velocity, |y, v| spring.scalar(y, v, t).0);
                let velocity = y.zip_map(initial_velocity, |y, v| spring.scalar(y, v, t).1);
                Sample {
                    value: target.zip_map(displacement, |a, b| a + b),
                    velocity,
                }
            }
        };
        if !sample.value.is_finite() || !sample.velocity.is_finite() {
            return Err(MotionError::NumericalOverflow);
        }
        Ok(sample)
    }
}

pub(crate) fn validate_values<V: MotionValue>(
    from: V,
    target: V,
    velocity: V,
) -> Result<(), MotionError> {
    if !from.is_finite() || !target.is_finite() || !velocity.is_finite() {
        return Err(MotionError::NonFiniteValue);
    }
    if !from.zip_map(target, |a, b| a - b).is_finite() {
        return Err(MotionError::NumericalOverflow);
    }
    Ok(())
}

/// Tolerances use value units (and units/second), never implicit pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionOptions {
    pub position_tolerance: f64,
    pub velocity_tolerance: f64,
    /// None reports logical completion together with physical completion.
    pub logical_duration: Option<Duration>,
    /// A hard finite-animation guard. Deadline completion is distinguishable.
    pub max_duration: Duration,
}

impl Default for MotionOptions {
    fn default() -> Self {
        Self {
            position_tolerance: 1e-4,
            velocity_tolerance: 1e-3,
            logical_duration: None,
            max_duration: Duration::from_secs(10),
        }
    }
}

impl MotionOptions {
    pub(crate) fn validate(self) -> Result<(), MotionError> {
        if !self.position_tolerance.is_finite()
            || self.position_tolerance <= 0.0
            || !self.velocity_tolerance.is_finite()
            || self.velocity_tolerance <= 0.0
            || self.max_duration.is_zero()
            || self.logical_duration.is_some_and(|t| t > self.max_duration)
        {
            return Err(MotionError::InvalidOptions);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub spec: MotionSpec,
    pub options: MotionOptions,
}

impl Motion {
    pub fn new(spec: MotionSpec) -> Self {
        Self {
            spec,
            options: MotionOptions::default(),
        }
    }
    pub fn with_options(self, options: MotionOptions) -> Self {
        Self { options, ..self }
    }
}
