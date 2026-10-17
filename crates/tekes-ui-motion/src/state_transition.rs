//! Component-owned transition lifecycle over an external monotonic clock.
//!
//! This adapter has no scheduler or system-preference reader. Hosts supply a
//! timestamp and the current preference. Pausing excludes wall time while
//! preserving spring velocity; it does not cancel or replace a generation.
use crate::{
    Animated, CancelBehavior, CancelReason, Frame, Motion, MotionError, MotionSpec, MotionValue,
    Sample,
};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct StateTransition<V> {
    channel: Animated<V>,
    wall: Duration,
    local: Duration,
    paused: bool,
    reduced: bool,
}

impl<V: MotionValue> StateTransition<V> {
    /// `now` establishes the host epoch; time before construction is excluded.
    pub fn new(value: V, now: Duration, reduced_motion: bool) -> Result<Self, MotionError> {
        Ok(Self {
            channel: Animated::new(value)?,
            wall: now,
            local: Duration::ZERO,
            paused: false,
            reduced: reduced_motion,
        })
    }

    /// Velocity is measured in active-clock seconds. While paused it is the
    /// retained continuation velocity, not a claim that pixels are moving.
    pub fn current(&self) -> Sample<V> {
        self.channel.current()
    }
    pub fn target(&self) -> V {
        self.channel.target()
    }
    pub fn generation(&self) -> u64 {
        self.channel.generation()
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    pub fn is_reduced_motion(&self) -> bool {
        self.reduced
    }
    pub fn needs_frame(&self) -> bool {
        self.channel.is_active() && !self.paused
    }

    pub fn advance(&mut self, now: Duration) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| next.channel.advance(next.local))
    }

    /// Retargeting preserves the current spring position and velocity, even
    /// while paused. Tween retargeting preserves position only, as in Animated.
    pub fn animate_to(
        &mut self,
        target: V,
        motion: Motion,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| {
            next.channel
                .animate_to(target, next.resolve(motion), next.local)
        })
    }

    /// Gesture velocity uses value units per active second. A paused release
    /// is stored and starts progressing only when the host resumes the channel.
    pub fn animate_to_with_velocity(
        &mut self,
        target: V,
        velocity: V,
        motion: Motion,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| {
            next.channel.animate_to_with_velocity(
                target,
                velocity,
                next.resolve(motion),
                next.local,
            )
        })
    }

    pub fn set_paused(&mut self, paused: bool, now: Duration) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| {
            let frame = next.channel.advance(next.local)?;
            next.paused = paused;
            Ok(frame)
        })
    }

    /// Conservative policy for spatial/state transitions: enabling reduced
    /// motion finishes the current target with a cancellation receipt. New
    /// targets resolve immediately. Disabling it never replays old movement.
    /// Components wanting a retained opacity fade should use a separate channel
    /// and an explicit component policy rather than bypassing this preference.
    pub fn set_reduced_motion(
        &mut self,
        reduced: bool,
        now: Duration,
    ) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| {
            let frame = if reduced && !next.reduced {
                next.channel.cancel(
                    next.local,
                    CancelReason::ReducedMotion,
                    CancelBehavior::Finish,
                )?
            } else {
                next.channel.advance(next.local)?
            };
            next.reduced = reduced;
            Ok(frame)
        })
    }

    pub fn cancel(
        &mut self,
        now: Duration,
        reason: CancelReason,
        behavior: CancelBehavior,
    ) -> Result<Frame<V>, MotionError> {
        self.update(now, |next| {
            next.channel.cancel(next.local, reason, behavior)
        })
    }

    fn resolve(&self, mut motion: Motion) -> Motion {
        if self.reduced {
            motion.spec = MotionSpec::Instant;
        }
        motion
    }

    fn update(
        &mut self,
        now: Duration,
        operation: impl FnOnce(&mut Self) -> Result<Frame<V>, MotionError>,
    ) -> Result<Frame<V>, MotionError> {
        if now < self.wall {
            return Err(MotionError::TimeWentBackwards);
        }
        // Clock, preference and channel changes commit together; invalid target,
        // options or samples cannot consume paused time or change a generation.
        let mut next = self.clone();
        if !next.paused {
            next.local = next
                .local
                .checked_add(now - next.wall)
                .ok_or(MotionError::NumericalOverflow)?;
        }
        next.wall = now;
        let mut frame = operation(&mut next)?;
        frame.needs_frame = next.needs_frame();
        *self = next;
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventKind, TimingCurve, presets};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn linear() -> Motion {
        Motion::new(MotionSpec::Tween {
            duration: ms(1000),
            curve: TimingCurve::LINEAR,
        })
    }

    #[test]
    fn pause_excludes_elapsed_wall_time_and_preserves_generation() {
        let mut x = StateTransition::new(0.0, ms(9000), false).unwrap();
        x.animate_to(1.0, linear(), ms(9000)).unwrap();
        let paused = x.set_paused(true, ms(9250)).unwrap();
        assert_eq!(paused.sample.value, 0.25);
        assert!(!paused.needs_frame);
        assert!(paused.events.is_empty());
        let resumed = x.set_paused(false, ms(60000)).unwrap();
        assert_eq!(resumed.sample, paused.sample);
        assert_eq!(resumed.generation, paused.generation);
        assert!(resumed.events.is_empty());
        assert!(resumed.needs_frame);
        assert_eq!(x.advance(ms(60250)).unwrap().sample.value, 0.5);
        assert!(!x.advance(ms(60750)).unwrap().needs_frame);
    }

    #[test]
    fn paused_retarget_keeps_spring_position_and_velocity() {
        let mut x = StateTransition::new([0.0, 2.0], ms(0), false).unwrap();
        x.animate_to([1.0, 4.0], presets::geometry(), ms(0))
            .unwrap();
        let before = x.set_paused(true, ms(70)).unwrap();
        assert_ne!(before.sample.velocity, [0.0, 0.0]);
        let redirected = x
            .animate_to([-1.0, -2.0], presets::geometry(), ms(5000))
            .unwrap();
        assert_eq!(redirected.sample, before.sample);
        assert!(!redirected.needs_frame);
        assert!(
            redirected
                .events
                .iter()
                .any(|event| event.generation == before.generation
                    && event.kind == EventKind::Cancelled(CancelReason::Retargeted))
        );
        assert_eq!(x.set_paused(false, ms(6000)).unwrap().sample, before.sample);
        assert_ne!(
            x.advance(ms(6010)).unwrap().sample.value,
            before.sample.value
        );
    }

    #[test]
    fn reduced_motion_finishes_paused_target_once_and_does_not_replay() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, presets::geometry(), ms(0)).unwrap();
        x.set_paused(true, ms(40)).unwrap();
        let reduced = x.set_reduced_motion(true, ms(1000)).unwrap();
        assert_eq!(reduced.sample.value, 1.0);
        assert_eq!(reduced.sample.velocity, 0.0);
        assert!(!reduced.needs_frame);
        assert_eq!(reduced.events.len(), 1);
        assert_eq!(
            reduced.events[0].kind,
            EventKind::Cancelled(CancelReason::ReducedMotion)
        );
        assert!(
            x.set_reduced_motion(true, ms(2000))
                .unwrap()
                .events
                .is_empty()
        );
        let instant = x.animate_to(2.0, presets::geometry(), ms(3000)).unwrap();
        assert_eq!(instant.sample.value, 2.0);
        assert!(!instant.needs_frame);
        x.set_reduced_motion(false, ms(4000)).unwrap();
        assert!(!x.set_paused(false, ms(5000)).unwrap().needs_frame);
        assert!(x.advance(ms(6000)).unwrap().events.is_empty());
    }

    #[test]
    fn invalid_operations_do_not_consume_clock_or_mutate_policy() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, linear(), ms(0)).unwrap();
        assert_eq!(
            x.animate_to(f64::NAN, linear(), ms(500)).unwrap_err(),
            MotionError::NonFiniteValue
        );
        // Earlier than the rejected operation is still legal.
        assert_eq!(x.set_paused(true, ms(250)).unwrap().sample.value, 0.25);
        assert_eq!(
            x.set_reduced_motion(true, ms(100)).unwrap_err(),
            MotionError::TimeWentBackwards
        );
        assert!(!x.is_reduced_motion());
        assert!(x.is_paused());
        assert_eq!(x.generation(), 1);
    }

    #[test]
    fn cancelled_generation_never_completes_after_resume() {
        for behavior in [CancelBehavior::Hold, CancelBehavior::Finish] {
            let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
            x.animate_to(1.0, linear(), ms(0)).unwrap();
            x.set_paused(true, ms(250)).unwrap();
            let cancelled = x
                .cancel(ms(1000), CancelReason::Unmounted, behavior)
                .unwrap();
            assert_eq!(
                cancelled.sample.value,
                if behavior == CancelBehavior::Hold {
                    0.25
                } else {
                    1.0
                }
            );
            assert_eq!(cancelled.events.len(), 1);
            assert_eq!(
                cancelled.events[0].kind,
                EventKind::Cancelled(CancelReason::Unmounted)
            );
            assert!(!x.set_paused(false, ms(2000)).unwrap().needs_frame);
            assert!(x.advance(ms(9000)).unwrap().events.is_empty());
        }
    }

    #[test]
    fn repeated_pause_resume_and_zero_dt_do_not_restart_or_emit_events() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, linear(), ms(0)).unwrap();
        let a = x.set_paused(true, ms(100)).unwrap();
        assert_eq!(x.set_paused(true, ms(200)).unwrap().sample, a.sample);
        let b = x.set_paused(false, ms(300)).unwrap();
        assert_eq!(a.sample, b.sample);
        let c = x.set_paused(false, ms(400)).unwrap();
        assert!((c.sample.value - 0.2).abs() < 1e-12);
        let d = x.advance(ms(400)).unwrap();
        assert_eq!(c.sample, d.sample);
        assert_eq!(d.generation, 1);
        assert!(d.events.is_empty());
    }

    #[test]
    fn paused_parameter_change_preserves_velocity_and_uses_new_spring_on_resume() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, presets::geometry(), ms(0)).unwrap();
        let old = x.set_paused(true, ms(60)).unwrap();
        let new_motion = Motion::new(MotionSpec::Spring(crate::Spring::new(1.1, 0.8).unwrap()));
        let changed = x.animate_to(1.0, new_motion, ms(1000)).unwrap();
        assert_eq!(old.sample, changed.sample);
        x.set_paused(false, ms(2000)).unwrap();
        let actual = x.advance(ms(2010)).unwrap().sample;
        let expected = new_motion
            .spec
            .sample(old.sample.value, 1.0, old.sample.velocity, ms(10))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn large_dt_finishes_once_but_large_paused_dt_never_hits_deadline() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, presets::geometry(), ms(0)).unwrap();
        let paused = x.set_paused(true, ms(20)).unwrap();
        assert_eq!(
            x.advance(Duration::from_secs(1_000_000)).unwrap().sample,
            paused.sample
        );
        x.set_paused(false, Duration::from_secs(1_000_000)).unwrap();
        let finished = x.advance(Duration::from_secs(2_000_000)).unwrap();
        assert!(!finished.needs_frame);
        assert_eq!(finished.sample.value, 1.0);
        assert_eq!(
            finished
                .events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Settled(_)))
                .count(),
            1
        );
        assert!(
            x.advance(Duration::from_secs(2_000_000))
                .unwrap()
                .events
                .is_empty()
        );
    }

    #[test]
    fn initially_reduced_is_immediate_and_disabled_preference_allows_only_new_motion() {
        let mut x = StateTransition::new(0.0, ms(0), true).unwrap();
        assert_eq!(
            x.animate_to(1.0, linear(), ms(0)).unwrap().sample.value,
            1.0
        );
        assert!(!x.set_reduced_motion(false, ms(0)).unwrap().needs_frame);
        assert!(x.animate_to(0.0, linear(), ms(0)).unwrap().needs_frame);
        assert_eq!(x.advance(ms(500)).unwrap().sample.value, 0.5);
    }

    #[test]
    fn enabling_reduced_motion_during_playback_stops_work_without_replay() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, linear(), ms(0)).unwrap();
        assert_eq!(x.advance(ms(200)).unwrap().sample.value, 0.2);
        let stopped = x.set_reduced_motion(true, ms(300)).unwrap();
        assert_eq!(stopped.sample.value, 1.0);
        assert!(!stopped.needs_frame);
        assert_eq!(stopped.events.len(), 1);
        assert_eq!(
            stopped.events[0].kind,
            EventKind::Cancelled(CancelReason::ReducedMotion)
        );
        assert!(
            x.set_reduced_motion(false, ms(400))
                .unwrap()
                .events
                .is_empty()
        );
        assert!(!x.advance(ms(500)).unwrap().needs_frame);
        assert!(x.animate_to(0.0, linear(), ms(500)).unwrap().needs_frame);
    }

    #[test]
    fn explicit_gesture_velocity_survives_pause_and_release() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.set_paused(true, ms(0)).unwrap();
        let frame = x
            .animate_to_with_velocity(1.0, -2.0, presets::geometry(), ms(500))
            .unwrap();
        assert_eq!(frame.sample.velocity, -2.0);
        assert!(!frame.needs_frame);
        let resumed = x.set_paused(false, ms(1000)).unwrap();
        assert_eq!(frame.sample, resumed.sample);
        assert!(x.advance(ms(1001)).unwrap().sample.value < 0.0);
    }

    #[test]
    fn reverse_at_same_timestamp_is_continuous() {
        let mut x = StateTransition::new(0.0, ms(0), false).unwrap();
        x.animate_to(1.0, presets::geometry(), ms(0)).unwrap();
        let current = x.advance(ms(80)).unwrap();
        let reversed = x.animate_to(0.0, presets::geometry(), ms(80)).unwrap();
        assert_eq!(current.sample, reversed.sample);
        assert_ne!(current.generation, reversed.generation);
    }
}
