//! Host-clocked state transitions owned by production controls.
//!
//! Components supply motion choices and monotonic timestamps. This adapter
//! keeps transition state with the component and never reads a clock or
//! schedules its own frames.
use std::time::Duration;

use tekes_ui_motion::state_transition::StateTransition;
use tekes_ui_motion::{CancelBehavior, CancelReason, Motion, MotionError, MotionSpec, TimingCurve};
use tekes_ui_tokens::proposal;

#[derive(Clone, Debug)]
pub(super) struct ProgressTransition {
    channel: Option<StateTransition<f64>>,
    value: f64,
    target: f64,
    reduced_motion: bool,
    last_now: Duration,
}

impl Default for ProgressTransition {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl ProgressTransition {
    pub(super) fn new(value: f64) -> Self {
        Self {
            channel: None,
            value,
            target: value,
            reduced_motion: false,
            last_now: Duration::ZERO,
        }
    }

    pub(super) fn value(&self) -> f64 {
        self.channel
            .as_ref()
            .map_or(self.value, |channel| channel.current().value)
    }

    pub(super) fn target(&self) -> f64 {
        self.target
    }

    pub(super) fn needs_animation(&self) -> bool {
        self.channel
            .as_ref()
            .is_some_and(StateTransition::needs_frame)
    }

    pub(super) fn retarget(
        &mut self,
        target: f64,
        motion: Motion,
        now: Duration,
        reduced_motion: bool,
    ) -> Result<f64, MotionError> {
        if now < self.last_now {
            return Err(MotionError::TimeWentBackwards);
        }
        if self.channel.is_none() {
            self.channel = Some(StateTransition::new(self.value, now, reduced_motion)?);
        }
        let channel = self.channel.as_mut().expect("initialized transition");
        if self.reduced_motion != reduced_motion {
            channel.set_reduced_motion(reduced_motion, now)?;
            self.reduced_motion = reduced_motion;
        }
        if target != self.target {
            channel.animate_to(target, motion, now)?;
            self.target = target;
        }
        let frame = channel.advance(now)?;
        self.value = frame.sample.value;
        self.last_now = now;
        Ok(self.value)
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<f64, MotionError> {
        if now < self.last_now {
            return Err(MotionError::TimeWentBackwards);
        }
        let Some(channel) = &mut self.channel else {
            self.last_now = now;
            return Ok(self.value);
        };
        self.value = channel.advance(now)?.sample.value;
        self.last_now = now;
        Ok(self.value)
    }

    pub(super) fn set_reduced_motion(
        &mut self,
        reduced_motion: bool,
        now: Duration,
    ) -> Result<f64, MotionError> {
        if now < self.last_now {
            return Err(MotionError::TimeWentBackwards);
        }
        if let Some(channel) = &mut self.channel {
            self.value = channel
                .set_reduced_motion(reduced_motion, now)?
                .sample
                .value;
        }
        self.reduced_motion = reduced_motion;
        self.last_now = now;
        Ok(self.value)
    }

    pub(super) fn snap(&mut self, target: f64, now: Duration) -> Result<f64, MotionError> {
        if now < self.last_now {
            return Err(MotionError::TimeWentBackwards);
        }
        if self.channel.is_none() {
            self.channel = Some(StateTransition::new(self.value, now, self.reduced_motion)?);
        }
        let channel = self.channel.as_mut().expect("initialized transition");
        channel.cancel(now, CancelReason::Explicit, CancelBehavior::Finish)?;
        channel.animate_to(target, Motion::new(MotionSpec::Instant), now)?;
        self.value = channel.advance(now)?.sample.value;
        self.target = target;
        self.last_now = now;
        Ok(self.value)
    }
}

pub(super) fn motion_for(role: &str) -> Motion {
    let seconds = proposal::control_duration(role).expect("component motion duration role");
    let [x1, y1, x2, y2] =
        proposal::control_easing(role).expect("component Design System motion easing");
    Motion::new(MotionSpec::Tween {
        duration: Duration::from_secs_f64(f64::from(seconds.max(0.0))),
        curve: TimingCurve::cubic_bezier(
            f64::from(x1),
            f64::from(y1),
            f64::from(x2),
            f64::from(y2),
        )
        .expect("validated Design System easing"),
    })
}
