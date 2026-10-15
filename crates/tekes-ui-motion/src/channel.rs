use crate::{Motion, MotionError, MotionSpec, MotionValue, Sample, model::validate_values};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason {
    Retargeted,
    Explicit,
    Unmounted,
    ReducedMotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelBehavior {
    Hold,
    Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishReason {
    Immediate,
    DurationElapsed,
    Converged,
    Deadline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    LogicalComplete,
    Settled(FinishReason),
    Cancelled(CancelReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub generation: u64,
    pub kind: EventKind,
}

/// Events belong to this operation only: there is no hidden callback queue.
/// Consumers dispatching events later must match both component identity and
/// generation. `needs_frame` reflects activity, not window visibility.
#[derive(Clone, Debug)]
pub struct Frame<V> {
    pub sample: Sample<V>,
    pub generation: u64,
    pub needs_frame: bool,
    pub events: Vec<Event>,
}

/// One exclusively-owned property channel. Drop it when its owner unmounts;
/// use `cancel` first if an explicit cancellation receipt is required.
#[derive(Clone, Debug)]
pub struct Animated<V> {
    sample: Sample<V>,
    target: V,
    origin: Sample<V>,
    motion: Motion,
    started: Duration,
    last: Duration,
    generation: u64,
    active: bool,
    logical_fired: bool,
}

impl<V: MotionValue> Animated<V> {
    pub fn new(value: V) -> Result<Self, MotionError> {
        if !value.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        let sample = Sample {
            value,
            velocity: value.zero(),
        };
        Ok(Self {
            sample,
            target: value,
            origin: sample,
            motion: Motion::new(MotionSpec::Instant),
            started: Duration::ZERO,
            last: Duration::ZERO,
            generation: 0,
            active: false,
            logical_fired: false,
        })
    }

    pub fn target(&self) -> V {
        self.target
    }
    pub fn current(&self) -> Sample<V> {
        self.sample
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn animate_to(
        &mut self,
        target: V,
        motion: Motion,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        self.start(target, None, motion, now)
    }

    /// For gesture release. Velocity is in the same coordinates as the value,
    /// expressed per second. A tween deliberately does not preserve it.
    pub fn animate_to_with_velocity(
        &mut self,
        target: V,
        velocity: V,
        motion: Motion,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        self.start(target, Some(velocity), motion, now)
    }

    fn start(
        &mut self,
        target: V,
        velocity: Option<V>,
        motion: Motion,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        // Work on a copy: validation/sampling failure is atomic for the caller.
        motion.options.validate()?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(MotionError::GenerationExhausted)?;
        let mut next = self.clone();
        let mut events = next.advance(now)?.events;
        let initial_velocity = velocity.unwrap_or(next.sample.velocity);
        validate_values(next.sample.value, target, initial_velocity)?;
        // Reject an unusable initial Bezier tangent before mutating anything.
        motion
            .spec
            .sample(next.sample.value, target, initial_velocity, Duration::ZERO)?;
        if next.active {
            events.push(Event {
                generation: next.generation,
                kind: EventKind::Cancelled(CancelReason::Retargeted),
            });
        }
        next.generation = generation;
        next.origin = Sample {
            value: next.sample.value,
            velocity: initial_velocity,
        };
        next.target = target;
        next.motion = motion;
        next.started = now;
        next.active = true;
        next.logical_fired = false;
        let first = next.advance(now)?;
        events.extend(first.events);
        let frame = next.frame(events);
        *self = next;
        Ok(frame)
    }

    /// Samples all properties at the caller's shared frame timestamp.
    pub fn advance(&mut self, now: Duration) -> Result<Frame<V>, MotionError> {
        if now < self.last {
            return Err(MotionError::TimeWentBackwards);
        }
        let mut events = Vec::new();
        if !self.active {
            self.last = now;
            return Ok(self.frame(events));
        }
        let elapsed = now - self.started;
        let finish = match self.motion.spec {
            MotionSpec::Instant => Some(FinishReason::Immediate),
            MotionSpec::Tween { duration, .. }
                if duration <= self.motion.options.max_duration && elapsed >= duration =>
            {
                Some(FinishReason::DurationElapsed)
            }
            _ if elapsed >= self.motion.options.max_duration => Some(FinishReason::Deadline),
            _ => None,
        };
        // Deadline check precedes sampling: even extreme elapsed times can
        // terminate without evaluating an overflowing polynomial.
        let sampled = if finish.is_some() {
            Sample {
                value: self.target,
                velocity: self.target.zero(),
            }
        } else {
            self.motion.spec.sample(
                self.origin.value,
                self.target,
                self.origin.velocity,
                elapsed,
            )?
        };
        let finish = finish.or_else(|| match self.motion.spec {
            MotionSpec::Spring(spring) => {
                let displacement = sampled.value.zip_map(self.target, |x, t| x - t);
                // Mechanical energy decreases under positive damping:
                // E = (v² + ω²y²)/2. Its current energy bounds every future
                // displacement AND velocity, even at oscillation crossings.
                let bounds =
                    displacement.zip_map(sampled.velocity, |y, v| y.hypot(v / spring.omega()));
                let limit = self
                    .motion
                    .options
                    .position_tolerance
                    .min(self.motion.options.velocity_tolerance / spring.omega());
                bounds
                    .all(|b| b <= limit)
                    .then_some(FinishReason::Converged)
            }
            _ => None,
        });
        let logical_due = self
            .motion
            .options
            .logical_duration
            .is_some_and(|t| elapsed >= t);
        if !self.logical_fired && (logical_due || finish.is_some()) {
            events.push(Event {
                generation: self.generation,
                kind: EventKind::LogicalComplete,
            });
            self.logical_fired = true;
        }
        self.sample = sampled;
        if let Some(reason) = finish {
            self.sample = Sample {
                value: self.target,
                velocity: self.target.zero(),
            };
            self.active = false;
            events.push(Event {
                generation: self.generation,
                kind: EventKind::Settled(reason),
            });
        }
        self.last = now;
        Ok(self.frame(events))
    }

    /// Cancels at the current timestamp, holding presentation or snapping to
    /// the semantic target. Cancellation does not emit successful completion.
    pub fn cancel(
        &mut self,
        now: Duration,
        reason: CancelReason,
        behavior: CancelBehavior,
    ) -> Result<Frame<V>, MotionError> {
        let mut events = self.advance(now)?.events;
        if self.active {
            events.push(Event {
                generation: self.generation,
                kind: EventKind::Cancelled(reason),
            });
            if behavior == CancelBehavior::Hold {
                self.target = self.sample.value;
            }
            self.sample = Sample {
                value: self.target,
                velocity: self.target.zero(),
            };
            self.active = false;
        }
        Ok(self.frame(events))
    }

    fn frame(&self, events: Vec<Event>) -> Frame<V> {
        Frame {
            sample: self.sample,
            generation: self.generation,
            needs_frame: self.active,
            events,
        }
    }
}
