//! Host layout presentation. Semantics remain in Host; DS owns the motion recipe.
use std::time::Duration;
use tekes_ui_motion::{
    Motion, MotionError, MotionSpec, TimingCurve, state_transition::StateTransition,
};
use tekes_ui_tokens::proposal;

pub(crate) struct SidebarMotion {
    sides: [StateTransition<f64>; 2],
}
impl SidebarMotion {
    pub fn new(leading: bool, trailing: bool) -> Self {
        Self {
            sides: [leading, trailing].map(|on| {
                StateTransition::new(if on { 1. } else { 0. }, Duration::ZERO, false).unwrap()
            }),
        }
    }
    pub fn retarget(&mut self, side: usize, on: bool, now: Duration) -> Result<(), MotionError> {
        let seconds = proposal::control_duration("disclosure_reveal").expect("DS reveal duration");
        let [x1, y1, x2, y2] =
            proposal::control_easing("disclosure_reveal").expect("DS reveal easing");
        let motion = Motion::new(MotionSpec::Tween {
            duration: Duration::from_secs_f64(f64::from(seconds)),
            curve: TimingCurve::cubic_bezier(x1.into(), y1.into(), x2.into(), y2.into())?,
        });
        self.sides[side].animate_to(if on { 1. } else { 0. }, motion, now)?;
        Ok(())
    }
    pub fn sample(
        &mut self,
        now: Duration,
        reduced: bool,
        active: bool,
    ) -> Result<[f32; 2], MotionError> {
        for side in &mut self.sides {
            side.set_reduced_motion(reduced, now)?;
            side.set_paused(!active, now)?;
            side.advance(now)?;
        }
        Ok(self
            .sides
            .each_ref()
            .map(|s| s.current().value.clamp(0., 1.) as f32))
    }
    pub fn needs_frame(&self) -> bool {
        self.sides.iter().any(StateTransition::needs_frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverses_from_current_geometry_without_moving_other_side() {
        let mut s = SidebarMotion::new(false, true);
        s.retarget(0, true, Duration::ZERO).unwrap();
        let midway = s.sample(Duration::from_millis(60), false, true).unwrap();
        assert!(midway[0] > 0. && midway[0] < 1.);
        assert_eq!(midway[1], 1.);
        s.retarget(0, false, Duration::from_millis(60)).unwrap();
        assert_eq!(
            s.sample(Duration::from_millis(60), false, true).unwrap(),
            midway
        );
        assert_eq!(
            s.sample(Duration::from_secs(1), false, true).unwrap(),
            [0., 1.]
        );
        assert!(!s.needs_frame());
    }
    #[test]
    fn reduced_motion_finishes_and_inactive_time_does_not_advance() {
        let mut s = SidebarMotion::new(false, true);
        s.retarget(0, true, Duration::ZERO).unwrap();
        let before = s.sample(Duration::from_millis(40), false, false).unwrap();
        assert!(!s.needs_frame());
        assert_eq!(
            s.sample(Duration::from_secs(3), false, false).unwrap(),
            before
        );
        assert_eq!(
            s.sample(Duration::from_secs(3), false, true).unwrap(),
            before
        );
        assert_eq!(
            s.sample(Duration::from_secs(3), true, true).unwrap(),
            [1., 1.]
        );
        s.retarget(1, false, Duration::from_secs(3)).unwrap();
        assert_eq!(
            s.sample(Duration::from_secs(3), true, true).unwrap(),
            [1., 0.]
        );
        assert!(!s.needs_frame());
    }
}
