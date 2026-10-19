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

    pub(super) fn clear(&mut self) {
        self.retained.clear();
        self.context.clear();
        let _ = self.progress.snap(0.0, self.now);
    }

    pub(super) fn paint(
        &mut self,
        destination: &mut tekes_ui_scene::Scene,
        text: &mut impl crate::ButtonTextRenderer,
        open: bool,
        context: String,
        current: Option<tekes_ui_scene::Scene>,
    ) {
        let clock = text.motion_context();
        if let Some(clock) = clock {
            self.now = self.now.max(clock.now);
        }
        if !open && self.context != context {
            self.clear();
            return;
        }
        if let Some(current) = current {
            if current.is_empty() {
                self.clear();
                return;
            }
            self.retained = current;
            self.context = context;
        }
        let target = if open { 1.0 } else { 0.0 };
        let value = if let Some(clock) = clock {
            self.progress.retarget(
                target,
                motion_for("overlay_presence"),
                self.now,
                clock.reduced_motion,
            )
        } else {
            self.progress.snap(target, self.now)
        }
        .unwrap_or(target)
        .clamp(0.0, 1.0) as f32;
        if value > 0.0 {
            for primitive in self.retained.primitives() {
                let mut primitive = primitive.clone();
                use tekes_ui_scene::Primitive;
                match &mut primitive {
                    Primitive::Quad(quad) => {
                        quad.fill.a *= value;
                        quad.border_color.a *= value;
                    }
                    Primitive::Shadow(shadow) => shadow.color.a *= value,
                    Primitive::Glyphs(run) => run.color.a *= value,
                    Primitive::Image(image) => image.opacity *= value,
                    Primitive::TransformedImage(image) => image.quad.opacity *= value,
                    Primitive::PushClip(_) | Primitive::PopClip => {}
                    // Future primitives must explicitly support retained fading.
                    _ => return,
                }
                destination.push(primitive);
            }
        }
        if self.progress.needs_animation() {
            text.request_animation_frame();
        } else if !open {
            self.retained.clear();
        }
    }
}
