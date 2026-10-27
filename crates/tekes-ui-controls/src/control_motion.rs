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

/// Retains only paint after semantic dismissal. Input and focus remain owned by
/// the component's open model. A host must render while `is_presented` is true,
/// and clear this state when unmounting or replacing its text/image resources.
/// Components rebuild the paint on every exit frame so retained glyph coordinates
/// never outlive the host atlas frame that produced them. Alpha is applied per
/// primitive, rather than an isolated compositing group.
#[derive(Clone, Debug, Default)]
pub(super) struct OverlayPresence {
    progress: ProgressTransition,
    retained: tekes_ui_scene::Scene,
    context: String,
    now: Duration,
}

impl OverlayPresence {
    pub(super) fn is_presented(&self) -> bool {
        !self.retained.is_empty()
            && (self.progress.value() > 0.0 || self.progress.needs_animation())
    }

    pub(super) fn needs_animation(&self) -> bool {
        self.progress.needs_animation()
    }

    pub(super) fn preflight_clears(&self, count: usize) -> bool {
        let mut progress = self.progress.clone();
        (0..count).all(|_| progress.snap(0.0, self.now).is_ok())
    }
    pub(super) fn clear(&mut self) {
        self.retained.clear();
        self.context.clear();
        let _ = self.progress.snap(0.0, self.now);
    }

    /// End an interrupted presentation at its semantic state. An open modal
    /// keeps passive paint; a dismissed transient retains neither paint nor work.
    pub(super) fn settle(&mut self, open: bool) {
        if open {
            let _ = self.progress.snap(1.0, self.now);
        } else {
            self.clear();
        }
    }

    pub(super) fn paint(
        &mut self,
        destination: &mut tekes_ui_scene::Scene,
        text: &mut impl crate::ButtonTextRenderer,
        open: bool,
        context: String,
        current: Option<tekes_ui_scene::Scene>,
    ) {
        if let Err(error) = self.try_paint(destination, text, open, context, current) {
            destination.record_composition_error(error);
        }
    }

    pub(super) fn try_paint(
        &mut self,
        destination: &mut tekes_ui_scene::Scene,
        text: &mut impl crate::ButtonTextRenderer,
        open: bool,
        context: String,
        current: Option<tekes_ui_scene::Scene>,
    ) -> Result<(), tekes_ui_scene::FragmentError> {
        if let Some(error) = destination.composition_error().or(current
            .as_ref()
            .and_then(tekes_ui_scene::Scene::composition_error))
        {
            return Err(error);
        }
        let clock = text.motion_context();
        let now = clock.map_or(self.now, |clock| self.now.max(clock.now));
        // Stage only this component's transition. Borrow retained geometry until
        // append commits; no mutable Host/editor or full destination clone.
        let mut progress = self.progress.clone();
        if !open && self.context != context
            || current
                .as_ref()
                .is_some_and(tekes_ui_scene::Scene::is_empty)
        {
            let _ = progress.snap(0.0, now);
            self.now = now;
            self.progress = progress;
            self.retained.clear();
            self.context.clear();
            return Ok(());
        }
        let retained = current.as_ref().unwrap_or(&self.retained);
        if let Some(error) = retained.composition_error() {
            return Err(error);
        }
        let target = if open { 1.0 } else { 0.0 };
        let value = if let Some(clock) = clock {
            progress.retarget(
                target,
                motion_for("overlay_presence"),
                now,
                clock.reduced_motion,
            )
        } else {
            progress.snap(target, now)
        }
        .unwrap_or(target)
        .clamp(0.0, 1.0) as f32;
        if value > 0.0 {
            destination.try_append_fragment(
                retained,
                tekes_ui_scene::FragmentBudget::default(),
                tekes_ui_scene::FragmentTransform {
                    opacity: value,
                    ..Default::default()
                },
            )?;
        }
        // No ordinary fallible tail. Failed append leaves retained/context/
        // progress/clock and the entire caller Scene unchanged.
        self.now = now;
        self.progress = progress;
        if let Some(current) = current {
            self.retained = current;
            self.context = context;
        }
        if self.progress.needs_animation() {
            text.request_animation_frame();
        } else if !open {
            self.retained.clear();
        }

        Ok(())
    }
}

#[cfg(test)]
mod fragment_green {
    use super::*;
    use tekes_ui_scene::*;
    struct Text;
    impl crate::ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(0., 0.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    #[test]
    fn retained_replay_must_not_truncate_at_shape_clip() {
        let mut table = ShapeTable::new(1, 8);
        let key = table
            .intern(
                ClipShape::normalize(
                    Rect::new(0., 0., 40., 40.),
                    CornerRadii::uniform(8.),
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let mut current = Scene::with_clip_shapes(&table);
        current.push_quad(Quad::new(
            Rect::new(0., 0., 1., 1.),
            tekes_ui_tokens::Rgba::gray(1.),
        ));
        current.push_shape_clip(key.clone());
        current.push_keyed_quad(
            Quad::new(Rect::new(0., 0., 40., 40.), tekes_ui_tokens::Rgba::gray(1.))
                .with_corner_radius(8.),
            key.clone(),
        );
        current.pop_clip();
        let expected = current.len();
        let mut caller = Scene::new();
        let mut presence = OverlayPresence::default();
        presence.paint(&mut caller, &mut Text, true, String::new(), Some(current));
        assert_eq!(
            caller.len(),
            expected,
            "retained replay truncated a shape fragment"
        );
        assert!(caller.clip_shape(&key).is_ok());
    }
}
#[cfg(test)]
mod atomic_presence {
    use super::*;
    use tekes_ui_scene::*;
    struct Text;
    impl crate::ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(1., 1.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    #[test]
    fn rejected_replay_preserves_presence_and_fallible_caller_legacy_marks_first_error() {
        let mut presence = OverlayPresence::default();
        let mut caller = Scene::new();
        let mut valid = Scene::new();
        valid.push_quad(Quad::new(
            Rect::new(1., 1., 4., 4.),
            tekes_ui_tokens::Rgba::gray(1.),
        ));
        presence
            .try_paint(&mut caller, &mut Text, true, "old".into(), Some(valid))
            .unwrap();
        let mut table = ShapeTable::new(1, 1);
        let key = table
            .intern(
                ClipShape::normalize(
                    Rect::new(0., 0., 8., 8.),
                    CornerRadii::ZERO,
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let mut invalid = Scene::new();
        invalid.require_clip_geometry(1);
        invalid.push_shape_clip(key);
        invalid.pop_clip();
        let before = caller.clone();
        let state = format!("{presence:?}");
        let error = presence
            .try_paint(
                &mut caller,
                &mut Text,
                true,
                "new".into(),
                Some(invalid.clone()),
            )
            .unwrap_err();
        assert_eq!(error, FragmentError::Geometry(ClipError::StaleShapeKey));
        assert_eq!(caller, before);
        assert_eq!(format!("{presence:?}"), state);
        presence.paint(&mut caller, &mut Text, true, "new".into(), Some(invalid));
        assert_eq!(caller.primitives(), before.primitives());
        assert_eq!(caller.clip_shape_count(), before.clip_shape_count());
        assert_eq!(caller.composition_error(), Some(error));
        assert_eq!(format!("{presence:?}"), state);
        caller.record_composition_error(FragmentError::UnsupportedTransform);
        assert_eq!(caller.composition_error(), Some(error));
    }
}
