//! Independent motion-page state. Presets come from the canonical JSON Design
//! System; there are no UI-owned replacement preset constants.
use std::{collections::VecDeque, time::Duration};
use tekes_ui_motion::{
    Animated, CancelBehavior, CancelReason, EventKind, Motion, MotionError, MotionOptions,
    MotionSpec, Spring, TimingCurve,
};

pub const SLUG: &str = "motion-foundation";
#[cfg(test)]
pub const TOKENS: &[&str] = &[
    "motion.duration",
    "motion.easing.standard",
    "motion.spring",
    "motion.accessibility.reduce_motion",
    "motion.policy.idle_animation",
];

#[derive(Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub label: String,
    pub source: String,
    pub motion: Motion,
}

/// m*x'' + c*x' + k*(x-target)=0 maps to ω=sqrt(k/m), ζ=c/(2mω).
pub fn design_presets() -> Vec<Preset> {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("../../../../design/source/motion.json"))
            .expect("validated canonical motion JSON");
    let controls: [f64; 4] =
        std::array::from_fn(|i| json["easing"]["standard"][i].as_f64().unwrap());
    let curve =
        TimingCurve::cubic_bezier(controls[0], controls[1], controls[2], controls[3]).unwrap();
    let mut presets = Vec::new();
    for (id, label) in [
        ("instant", "Instant"),
        ("stateChange", "State change"),
        ("geometry", "Geometry"),
        ("listExpansion", "List expansion"),
        ("expansion", "Expansion"),
        ("press", "Press"),
    ] {
        let duration = Duration::from_secs_f64(json["duration"][id].as_f64().unwrap());
        let spec = if duration.is_zero() {
            MotionSpec::Instant
        } else {
            MotionSpec::Tween { duration, curve }
        };
        presets.push(Preset {
            id: format!("duration.{id}"),
            label: label.into(),
            source: format!("motion.duration.{id} / easing.standard"),
            motion: Motion::new(spec),
        });
    }
    for (id, label) in [
        ("micro", "Micro spring"),
        ("reveal", "Reveal spring"),
        ("panel", "Panel spring"),
        ("feedback", "Feedback spring"),
    ] {
        let p = &json["spring"][id];
        let spring = Spring::from_physical(
            p["mass"].as_f64().unwrap(),
            p["stiffness"].as_f64().unwrap(),
            p["damping"].as_f64().unwrap(),
        )
        .expect("validated JSON spring");
        presets.push(Preset {
            id: format!("spring.{id}"),
            label: label.into(),
            source: format!(
                "motion.spring.{id} · m={} k={} c={}",
                p["mass"], p["stiffness"], p["damping"]
            ),
            motion: Motion::new(MotionSpec::Spring(spring)),
        });
    }
    presets
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    Frequency,
    Damping,
    Duration,
    ReleaseVelocity,
}
impl Parameter {
    pub const ALL: [Self; 4] = [
        Self::Frequency,
        Self::Damping,
        Self::Duration,
        Self::ReleaseVelocity,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Frequency => "频率 / Hz",
            Self::Damping => "阻尼比 / ζ",
            Self::Duration => "时长 / s",
            Self::ReleaseVelocity => "起始速度 / track·s⁻¹",
        }
    }
    pub fn range(self) -> (f64, f64) {
        match self {
            Self::Frequency => (0.5, 24.0),
            Self::Damping => (0.05, 6.0),
            Self::Duration => (0.01, 2.0),
            Self::ReleaseVelocity => (-3.0, 3.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Trace {
    pub time: f64,
    pub value: f64,
    pub velocity: f64,
    pub target: f64,
}

#[derive(Clone, Debug)]
pub struct MotionPreview {
    pub channel: Animated<f64>,
    pub presets: Vec<Preset>,
    pub selected: usize,
    pub custom: bool,
    pub frequency: f64,
    pub damping: f64,
    pub duration: f64,
    pub release_velocity: f64,
    pub reduced_motion: bool,
    manual_reduced_motion: bool,
    system_reduced_motion: bool,
    pub trace: VecDeque<Trace>,
    pub events: VecDeque<String>,
    pub error: Option<String>,
    pub now: Duration,
    /// Set by the view while dragging. Cleared on release/focus/page change.
    pub dragging: Option<Parameter>,
    pub hovered: Option<usize>,
    pub pressed: Option<usize>,
    interrupt_at: Option<Duration>,
}

impl Default for MotionPreview {
    fn default() -> Self {
        let presets = design_presets();
        let selected = presets
            .iter()
            .position(|p| p.id == "spring.reveal")
            .unwrap();
        let mut state = Self {
            channel: Animated::new(0.12).unwrap(),
            presets,
            selected,
            custom: false,
            frequency: 1.0,
            damping: 1.0,
            duration: 0.5,
            release_velocity: 0.0,
            reduced_motion: false,
            manual_reduced_motion: false,
            system_reduced_motion: false,
            trace: VecDeque::with_capacity(512),
            events: VecDeque::new(),
            error: None,
            now: Duration::ZERO,
            dragging: None,
            hovered: None,
            pressed: None,
            interrupt_at: None,
        };
        state.load_parameters();
        // Start stationary: opening a page never starts an unsolicited loop.
        state
    }
}

impl MotionPreview {
    pub fn is_spring(&self) -> bool {
        matches!(
            self.presets[self.selected].motion.spec,
            MotionSpec::Spring(_)
        )
    }
    pub fn is_tween(&self) -> bool {
        matches!(
            self.presets[self.selected].motion.spec,
            MotionSpec::Tween { .. }
        )
    }
    pub fn parameter_enabled(&self, p: Parameter) -> bool {
        match p {
            Parameter::Frequency | Parameter::Damping | Parameter::ReleaseVelocity => {
                self.is_spring()
            }
            Parameter::Duration => self.is_tween(),
        }
    }
    fn load_parameters(&mut self) {
        match self.presets[self.selected].motion.spec {
            MotionSpec::Spring(s) => {
                self.frequency = s.frequency_hz();
                self.damping = s.damping_ratio();
            }
            MotionSpec::Tween { duration, .. } => self.duration = duration.as_secs_f64(),
            MotionSpec::Instant => {}
        }
    }
    pub fn motion(&self) -> Result<Motion, MotionError> {
        if self.reduced_motion {
            return Ok(Motion::new(MotionSpec::Instant));
        }
        let original = self.presets[self.selected].motion;
        let spec = if !self.custom {
            original.spec
        } else {
            match original.spec {
                MotionSpec::Spring(_) => {
                    MotionSpec::Spring(Spring::new(self.frequency, self.damping)?)
                }
                MotionSpec::Tween { curve, .. } => MotionSpec::Tween {
                    duration: Duration::try_from_secs_f64(self.duration)
                        .map_err(|_| MotionError::InvalidOptions)?,
                    curve,
                },
                MotionSpec::Instant => MotionSpec::Instant,
            }
        };
        Ok(Motion::new(spec).with_options(MotionOptions {
            // Track-normalized channels; independent of display backing scale.
            position_tolerance: 1e-4,
            velocity_tolerance: 1e-3,
            ..original.options
        }))
    }
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.presets.len() {
            return false;
        }
        self.selected = index;
        self.custom = false;
        self.release_velocity = 0.0;
        self.load_parameters();
        self.replay();
        true
    }
    pub fn replay(&mut self) {
        self.now = Duration::ZERO;
        self.trace.clear();
        self.events.clear();
        self.error = None;
        self.interrupt_at = None;
        self.channel = Animated::new(0.12).unwrap();
        let result = self.motion().and_then(|motion| {
            self.channel
                .animate_to_with_velocity(0.88, self.release_velocity, motion, self.now)
        });
        self.accept(result);
        self.record();
    }
    pub fn retarget(&mut self, target: f64) {
        if !target.is_finite() {
            return;
        }
        // An explicit user/host target takes ownership from the one-shot demo.
        // Otherwise a pending fixture event could overwrite it a frame later.
        self.interrupt_at = None;
        let target = target.clamp(0.05, 0.95);
        let result = self
            .motion()
            .and_then(|motion| self.channel.animate_to(target, motion, self.now));
        self.accept(result);
        self.record();
    }
    pub fn demonstrate_interrupt(&mut self) {
        self.replay();
        if !self.reduced_motion && self.channel.is_active() {
            let delay = match self.motion().map(|motion| motion.spec) {
                Ok(MotionSpec::Tween { duration, .. }) => duration.mul_f64(0.4),
                _ => Duration::from_millis(140),
            };
            self.interrupt_at = Some(delay);
        }
    }
    pub fn parameter_value(&self, p: Parameter) -> f64 {
        match p {
            Parameter::Frequency => self.frequency,
            Parameter::Damping => self.damping,
            Parameter::Duration => self.duration,
            Parameter::ReleaseVelocity => self.release_velocity,
        }
    }
    /// Slider inputs are clamped, NaN is ignored. Changing a running spring
    /// retargets from the same position and velocity; release speed is for Replay.
    pub fn set_parameter(&mut self, p: Parameter, value: f64) -> bool {
        if !self.parameter_enabled(p) || !value.is_finite() {
            return false;
        }
        let (lo, hi) = p.range();
        let value = value.clamp(lo, hi);
        if (self.parameter_value(p) - value).abs() < 1e-9 {
            return false;
        }
        match p {
            Parameter::Frequency => self.frequency = value,
            Parameter::Damping => self.damping = value,
            Parameter::Duration => self.duration = value,
            Parameter::ReleaseVelocity => self.release_velocity = value,
        }
        self.custom = true;
        if p != Parameter::ReleaseVelocity {
            self.retarget(self.channel.target());
        }
        true
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        if self.manual_reduced_motion == reduced {
            return false;
        }
        self.manual_reduced_motion = reduced;
        self.apply_reduced_motion()
    }
    /// The demonstration toggle changes this remembered manual choice, not
    /// the effective OR with the system accessibility preference.
    pub fn manual_reduced_motion(&self) -> bool {
        self.manual_reduced_motion
    }
    /// System accessibility always wins over the manual demonstration toggle.
    /// Clearing the system preference restores the remembered manual choice;
    /// it never starts a new animation by itself.
    pub fn set_system_reduced_motion(&mut self, reduced: bool) -> bool {
        if self.system_reduced_motion == reduced {
            return false;
        }
        self.system_reduced_motion = reduced;
        self.apply_reduced_motion()
    }
    fn apply_reduced_motion(&mut self) -> bool {
        let effective = self.manual_reduced_motion || self.system_reduced_motion;
        if self.reduced_motion == effective {
            return false;
        }
        self.reduced_motion = effective;
        if effective {
            let result = self.channel.cancel(
                self.now,
                CancelReason::ReducedMotion,
                CancelBehavior::Finish,
            );
            self.accept(result);
            self.interrupt_at = None;
            self.record();
        }
        true
    }
    /// Actual elapsed simulation time, supplied by the shared Gallery host.
    /// No thread, NSTimer, display link or wall clock exists in this module.
    pub fn animate(&mut self, dt: f32) -> bool {
        if !self.needs_animation() || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let Ok(delta) = Duration::try_from_secs_f64(f64::from(dt)) else {
            return false;
        };
        let Some(end) = self.now.checked_add(delta) else {
            return false;
        };
        if let Some(at) = self.interrupt_at.filter(|at| *at <= end) {
            self.sample_at(at);
            self.interrupt_at = None;
            self.retarget(0.28);
        }
        self.sample_at(end);
        true
    }
    pub fn needs_animation(&self) -> bool {
        self.error.is_none() && (self.channel.is_active() || self.interrupt_at.is_some())
    }
    pub fn deactivate(&mut self) {
        self.dragging = None;
        self.hovered = None;
        self.pressed = None;
    }
    fn sample_at(&mut self, time: Duration) {
        self.now = time;
        let result = self.channel.advance(time);
        self.accept(result);
        self.record();
    }
    fn accept(&mut self, result: Result<tekes_ui_motion::Frame<f64>, MotionError>) {
        match result {
            Ok(frame) => {
                self.error = None;
                for event in frame.events {
                    let label = match event.kind {
                        EventKind::LogicalComplete => "逻辑完成".into(),
                        EventKind::Settled(reason) => format!("停止 · {reason:?}"),
                        EventKind::Cancelled(reason) => format!("取消 · {reason:?}"),
                    };
                    self.events
                        .push_front(format!("#{:02}  {label}", event.generation));
                }
                self.events.truncate(3);
            }
            Err(error) => {
                self.error = Some(error.to_string());
                self.interrupt_at = None;
            }
        }
    }
    fn record(&mut self) {
        let sample = self.channel.current();
        // At most one trace point per display callback, bounded memory.
        if self
            .trace
            .back()
            .is_some_and(|point| point.time == self.now.as_secs_f64())
        {
            self.trace.pop_back();
        }
        self.trace.push_back(Trace {
            time: self.now.as_secs_f64(),
            value: sample.value,
            velocity: sample.velocity,
            target: self.channel.target(),
        });
        if self.trace.len() > 480 {
            self.trace.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_source_and_generated_design_projection_agree() {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/source/motion.json")).unwrap();
        let merged: serde_json::Value =
            serde_json::from_str(include_str!("../../../../design/tokens.json")).unwrap();
        assert_eq!(source, merged["proposal"]["motion"]);
        assert_eq!(design_presets().len(), 10);
        assert!(TOKENS.iter().all(|s| s.starts_with("motion.")));
    }
    #[test]
    fn presets_settle_and_opening_is_idle() {
        let mut state = MotionPreview::default();
        assert!(!state.needs_animation());
        for i in 0..state.presets.len() {
            state.select(i);
            for _ in 0..1500 {
                state.animate(1.0 / 120.0);
            }
            assert!(!state.needs_animation(), "{}", state.presets[i].id);
            assert_eq!(state.channel.current().value, 0.88);
        }
    }
    #[test]
    fn repeated_targets_and_parameter_changes_keep_spring_continuity() {
        let mut state = MotionPreview::default();
        state.replay();
        for i in 0..100 {
            state.animate(1.0 / 120.0);
            let old = state.channel.current();
            state.retarget(if i % 2 == 0 { 0.15 } else { 0.85 });
            assert_eq!(old, state.channel.current());
            state.set_parameter(Parameter::Frequency, 3.0 + f64::from(i) / 100.0);
            assert_eq!(old, state.channel.current());
        }
        assert!(state.events.len() <= 3);
        assert!(state.trace.len() <= 480);
    }
    #[test]
    fn interruption_time_is_split_exactly_and_reduce_motion_cancels_it() {
        let run = |dt: f32| {
            let mut state = MotionPreview::default();
            state.demonstrate_interrupt();
            while state.now.as_secs_f64() < 0.2 {
                state.animate(dt);
            }
            state
        };
        assert_eq!(run(1.0 / 60.0).channel.target(), 0.28);
        assert_eq!(run(1.0 / 120.0).channel.target(), 0.28);
        let mut state = MotionPreview::default();
        state.demonstrate_interrupt();
        state.animate(0.05);
        state.set_reduced_motion(true);
        assert!(!state.needs_animation());
        assert_eq!(state.channel.current().value, 0.88);
        state.replay();
        assert!(!state.needs_animation());
        state.retarget(0.28);
        assert_eq!(state.channel.current().value, 0.28);
    }
    #[test]
    fn release_speed_and_invalid_or_disabled_parameters() {
        let mut state = MotionPreview::default();
        state.set_parameter(Parameter::ReleaseVelocity, -2.0);
        state.replay();
        assert_eq!(state.channel.current().velocity, -2.0);
        assert!(!state.set_parameter(Parameter::Frequency, f64::NAN));
        state.select(2);
        assert!(!state.set_parameter(Parameter::Frequency, 8.0));
        assert!(state.set_parameter(Parameter::Duration, 0.4));
        assert!(state.custom);
        state.select(2);
        assert!(!state.custom);
    }

    #[test]
    fn instant_does_not_schedule_work_and_short_tween_interrupts_before_end() {
        let mut state = MotionPreview::default();
        state.select(0);
        state.demonstrate_interrupt();
        assert!(!state.needs_animation());
        state.select(5); // JSON press duration = 0.07s, interruption at 40%.
        state.demonstrate_interrupt();
        state.animate(0.03);
        assert_eq!(state.channel.target(), 0.28);
        assert!(state.now < Duration::from_millis(70));
        let before = state.now;
        assert!(!state.animate(f32::MAX));
        assert_eq!(state.now, before);
    }

    #[test]
    fn system_reduction_cannot_be_disabled_by_the_manual_toggle() {
        let mut state = MotionPreview::default();
        state.replay();
        state.animate(0.05);
        assert!(state.set_system_reduced_motion(true));
        assert!(state.reduced_motion);
        assert_eq!(state.channel.current().value, 0.88);
        assert!(!state.needs_animation());
        let events = state.events.clone();
        assert!(!state.set_reduced_motion(false));
        assert!(!state.set_system_reduced_motion(true));
        assert!(state.reduced_motion);
        assert_eq!(state.events, events);
        state.retarget(0.28);
        assert_eq!(state.channel.current().value, 0.28);
        assert!(!state.needs_animation());
    }

    #[test]
    fn clearing_system_reduction_restores_manual_choice_without_replay() {
        let mut state = MotionPreview::default();
        assert!(state.set_reduced_motion(true));
        assert!(!state.set_system_reduced_motion(true));
        assert!(!state.set_system_reduced_motion(false));
        assert!(state.reduced_motion); // Manual override remains on.
        assert!(state.set_reduced_motion(false));
        assert!(!state.reduced_motion);
        assert!(!state.needs_animation());
        assert!(state.set_system_reduced_motion(true));
        assert!(!state.set_reduced_motion(true));
        assert!(!state.set_reduced_motion(false));
        assert!(state.set_system_reduced_motion(false));
        assert!(!state.reduced_motion);
        assert!(!state.needs_animation());
    }

    #[test]
    fn reduced_motion_cancellation_and_repeated_preference_updates_are_idempotent() {
        let mut state = MotionPreview::default();
        state.demonstrate_interrupt();
        state.animate(0.05);
        let generation = state.channel.generation();
        state.set_reduced_motion(true);
        let events = state.events.clone();
        let cancel = events
            .iter()
            .filter(|text| text.contains("ReducedMotion"))
            .count();
        assert_eq!(cancel, 1);
        for _ in 0..10 {
            assert!(!state.set_reduced_motion(true));
            assert!(!state.set_system_reduced_motion(true));
            assert!(!state.set_system_reduced_motion(false));
            assert!(!state.animate(1.0 / 120.0));
        }
        assert_eq!(state.channel.generation(), generation);
        assert_eq!(state.events, events);
        assert_eq!(state.channel.current().value, 0.88);
        assert_eq!(state.channel.current().velocity, 0.0);
        assert!(!state.needs_animation());
    }

    #[test]
    fn explicit_target_or_parameter_change_cancels_the_pending_demo_target() {
        let mut state = MotionPreview::default();
        state.demonstrate_interrupt();
        state.animate(0.05);
        state.retarget(0.75);
        state.animate(0.2);
        assert_eq!(state.channel.target(), 0.75);
        state.demonstrate_interrupt();
        state.animate(0.05);
        state.set_parameter(Parameter::Frequency, 4.0);
        state.animate(0.2);
        assert_eq!(state.channel.target(), 0.88);
    }
}
