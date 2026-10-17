//! Interruptible, host-clocked composition examples. Parameters are TekesUI
//! experiments, not proprietary Airbnb/Figma/Claude/Cursor configurations.
use crate::{
    Animated, CancelBehavior, CancelReason, Motion, MotionError, MotionSpec, Spring, TimingCurve,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeKind {
    SharedElement,
    Panel,
    Success,
    Reading,
    ListReorder,
    SharedAxis,
    FadeThrough,
    DragSnap,
    Stagger,
    Popover,
    Press,
    Skeleton,
    Numeric,
    PageSwitch,
    TextScan,
    Rotate2D,
    FlipX,
    FlipY,
    SpinBrake,
    RotateSettle,
    Typing,
    StreamTyping,
    Symbol(crate::symbols::SymbolEffect),
    LinearRotation,
}
impl RecipeKind {
    pub const ALL: [Self; 22] = [
        Self::SharedElement,
        Self::Panel,
        Self::Success,
        Self::Reading,
        Self::ListReorder,
        Self::SharedAxis,
        Self::FadeThrough,
        Self::DragSnap,
        Self::Stagger,
        Self::Popover,
        Self::Press,
        Self::Skeleton,
        Self::Numeric,
        Self::PageSwitch,
        Self::TextScan,
        Self::Rotate2D,
        Self::FlipX,
        Self::FlipY,
        Self::SpinBrake,
        Self::RotateSettle,
        Self::Typing,
        Self::StreamTyping,
    ];
    pub const fn is_typing(self) -> bool {
        matches!(self, Self::Typing | Self::StreamTyping)
    }
    pub const fn is_rotation(self) -> bool {
        matches!(
            self,
            Self::Rotate2D | Self::FlipX | Self::FlipY | Self::SpinBrake | Self::RotateSettle
        )
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::SharedElement => "共享元素转场",
            Self::Panel => "多属性面板",
            Self::Success => "分阶段成功反馈",
            Self::Reading => "保持阅读位置",
            Self::ListReorder => "列表增删与重排",
            Self::SharedAxis => "Shared Axis 导航",
            Self::FadeThrough => "Fade Through 切换",
            Self::DragSnap => "拖拽惯性与吸附",
            Self::Stagger => "Stagger 交错入场",
            Self::Popover => "锚定 Popover",
            Self::Press => "按压反馈分轨",
            Self::Skeleton => "Skeleton → 内容",
            Self::Numeric => "数字内容转场",
            Self::PageSwitch => "顶层页面切换",
            Self::TextScan => "斜向文字扫描",
            Self::Rotate2D => "平面旋转 Z",
            Self::FlipX => "水平轴翻转 X",
            Self::FlipY => "垂直轴翻转 Y",
            Self::SpinBrake => "加速旋转与刹停",
            Self::RotateSettle => "旋转回摆落定",
            Self::Typing => "定时打字与浮现",
            Self::StreamTyping => "流式批量文字浮现",
            Self::Symbol(effect) => effect.label(),
            Self::LinearRotation => "Linear rotation",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecipeParameters {
    pub duration: f64,
    pub damping: f64,
    pub delay: f64,
    /// Points: corner radius, panel travel, success pulse size or insertion height.
    pub amount: f64,
    pub scan_angle: f64,
    pub rotation_degrees: f64,
    pub brake_bias: f64,
    pub typing_rate: f64,
    pub typing_fade: f64,
    pub typing_batch: usize,
    pub symbol_strength: f64,
    pub symbol_cycles: u8,
}
impl Default for RecipeParameters {
    fn default() -> Self {
        Self {
            duration: 0.5,
            damping: 0.85,
            delay: 0.12,
            amount: 72.0,
            scan_angle: 45.0,
            rotation_degrees: 90.0,
            brake_bias: 0.2,
            typing_rate: 28.0,
            typing_fade: 0.18,
            typing_batch: 5,
            symbol_strength: 0.6,
            symbol_cycles: 3,
        }
    }
}
impl RecipeParameters {
    fn validate(self) -> Result<(), MotionError> {
        if !self.symbol_strength.is_finite()
            || !(0.0..=1.0).contains(&self.symbol_strength)
            || !(1..=8).contains(&self.symbol_cycles)
            || !self.duration.is_finite()
            || !(0.15..=4.0).contains(&self.duration)
            || !self.damping.is_finite()
            || !(0.35..=1.5).contains(&self.damping)
            || !self.delay.is_finite()
            || !(0.0..=0.6).contains(&self.delay)
            || !self.typing_rate.is_finite()
            || !(0.0..=240.0).contains(&self.typing_rate)
            || !self.typing_fade.is_finite()
            || !(0.0..=0.8).contains(&self.typing_fade)
            || !(1..=12).contains(&self.typing_batch)
            || !self.brake_bias.is_finite()
            || !(-1.0..=1.0).contains(&self.brake_bias)
            || !self.rotation_degrees.is_finite()
            || !(15.0..=2160.0).contains(&self.rotation_degrees)
            || !self.scan_angle.is_finite()
            || !(-75.0..=75.0).contains(&self.scan_angle)
            || !self.amount.is_finite()
            || !(16.0..=160.0).contains(&self.amount)
        {
            return Err(MotionError::InvalidOptions);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Preparing,
    Working,
    Success,
    Cancelled,
}
#[derive(Clone, Copy, Debug)]
struct Cue {
    at: Duration,
    track: usize,
    target: f64,
    phase: Option<Phase>,
}

/// Four independently interruptible property tracks, plus bounded pending cues.
/// Explicit targets/parameter edits replace pending cues; old phases cannot fire.
#[derive(Clone, Debug)]
pub struct RecipePlayer {
    pub kind: RecipeKind,
    pub parameters: RecipeParameters,
    motor: crate::linear_rotation::LinearRotation,
    tracks: [Animated<f64>; 4],
    pending: Vec<Cue>,
    items: [Animated<f64>; 6],
    snap_target: f64,
    pub typing: crate::typing::TypingEngine,
    pub typing_unit_count: usize,
    typing_sent: usize,
    typing_next: Option<Duration>,
    brake: Option<AngularBrake>,
    brake_sample: crate::Sample<f64>,
    now: Duration,
    pub phase: Phase,
    target: bool,
    reduced: bool,
    paused: bool,
    pub user_scroll: f64,
}
impl Default for RecipePlayer {
    fn default() -> Self {
        Self::new(RecipeKind::SharedElement)
    }
}
impl RecipePlayer {
    pub fn new(kind: RecipeKind) -> Self {
        Self {
            kind,
            motor: crate::linear_rotation::LinearRotation::new(0.5).unwrap(),
            parameters: if kind == RecipeKind::LinearRotation {
                RecipeParameters {
                    duration: 2.0,
                    symbol_cycles: 1,
                    ..RecipeParameters::default()
                }
            } else if let RecipeKind::Symbol(effect) = kind {
                let (duration, options) = effect.preview_defaults();
                RecipeParameters {
                    duration,
                    symbol_cycles: options.cycles,
                    symbol_strength: options.strength,
                    delay: options.layer_delay,
                    ..RecipeParameters::default()
                }
            } else if kind == RecipeKind::TextScan {
                RecipeParameters {
                    duration: 1.2,
                    amount: 64.0,
                    ..RecipeParameters::default()
                }
            } else if kind.is_rotation() {
                RecipeParameters {
                    duration: if kind == RecipeKind::SpinBrake {
                        1.8
                    } else {
                        0.6
                    },
                    damping: if kind == RecipeKind::RotateSettle {
                        0.45
                    } else {
                        0.9
                    },
                    rotation_degrees: match kind {
                        RecipeKind::SpinBrake => 1080.0,
                        RecipeKind::FlipX | RecipeKind::FlipY => 180.0,
                        _ => 90.0,
                    },
                    ..RecipeParameters::default()
                }
            } else if kind == RecipeKind::StreamTyping {
                RecipeParameters {
                    typing_rate: 0.0,
                    ..RecipeParameters::default()
                }
            } else {
                RecipeParameters::default()
            },
            tracks: std::array::from_fn(|_| Animated::new(0.0).unwrap()),
            pending: Vec::new(),
            items: std::array::from_fn(|i| {
                Animated::new(if kind == RecipeKind::ListReorder {
                    i as f64
                } else if kind == RecipeKind::Numeric && i == 0 {
                    128.0
                } else {
                    0.0
                })
                .unwrap()
            }),
            snap_target: 0.0,
            typing: crate::typing::TypingEngine::new(Default::default()).unwrap(),
            typing_unit_count: 56,
            typing_sent: 0,
            typing_next: None,
            brake: None,
            brake_sample: crate::Sample {
                value: 0.0,
                velocity: 0.0,
            },
            now: Duration::ZERO,
            phase: Phase::Idle,
            target: false,
            reduced: false,
            paused: false,
            user_scroll: 72.0,
        }
    }
    pub fn values(&self) -> [f64; 4] {
        std::array::from_fn(|i| {
            if i == 0 && self.kind == RecipeKind::LinearRotation {
                self.motor.sample().value
            } else if i == 0 && self.kind == RecipeKind::SpinBrake {
                self.brake_sample.value
            } else {
                self.tracks[i].current().value
            }
        })
    }
    pub fn velocities(&self) -> [f64; 4] {
        std::array::from_fn(|i| {
            if i == 0 && self.kind == RecipeKind::LinearRotation {
                self.motor.sample().velocity
            } else if i == 0 && self.kind == RecipeKind::SpinBrake {
                self.brake_sample.velocity
            } else {
                self.tracks[i].current().velocity
            }
        })
    }
    pub fn item_values(&self) -> [f64; 6] {
        std::array::from_fn(|i| self.items[i].current().value)
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    pub fn snap_target(&self) -> f64 {
        self.snap_target
    }
    /// Direct manipulation replaces the presentation value, cancelling inertia.
    pub fn drag_to(&mut self, position: f64) -> Result<(), MotionError> {
        if !position.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        self.interrupt()?;
        self.tracks[0] = Animated::new(position.clamp(0.0, 1.0))?;
        Ok(())
    }
    /// Velocity is normalized track units / second. Predict then snap to one
    /// of five stops; amount controls the velocity projection horizon.
    pub fn release_drag(&mut self, velocity: f64) -> Result<(), MotionError> {
        if !velocity.is_finite() {
            return Err(MotionError::NonFiniteValue);
        }
        self.pending.clear();
        self.paused = false;
        self.phase = Phase::Working;
        let v = velocity.clamp(-6.0, 6.0);
        let predicted = self.values()[0] + v * self.parameters.amount / 400.0;
        self.snap_target = (predicted * 4.0).round().clamp(0.0, 4.0) / 4.0;
        self.target = self.snap_target >= 0.5;
        self.tracks[0].animate_to_with_velocity(self.snap_target, v, self.motion(0), self.now)?;
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), MotionError> {
        if self.kind.is_typing() {
            self.typing.resume(self.now)?;
            self.paused = false;
            self.phase = Phase::Working;
            if self.typing_sent < self.typing_unit_count {
                self.typing_next = Some(
                    self.now
                        .saturating_add(Duration::from_secs_f64(self.parameters.delay.max(0.03))),
                );
            }
            return Ok(());
        }
        if self.kind == RecipeKind::DragSnap {
            self.paused = false;
            self.start(0, self.snap_target)
        } else {
            self.set_target(self.target)
        }
    }
    pub fn target(&self) -> bool {
        self.target
    }
    pub fn now(&self) -> Duration {
        self.now
    }
    pub fn needs_frame(&self) -> bool {
        self.motor.is_active()
            || (self.kind.is_typing()
                && !self.paused
                && (self.typing.needs_frame() || self.typing_next.is_some()))
            || self.brake.is_some()
            || !self.pending.is_empty()
            || self
                .tracks
                .iter()
                .chain(self.items.iter())
                .any(Animated::is_active)
    }
    /// Inserted document height and compensated scroll use exactly the same
    /// presentation value, so old content never drifts during the animation.
    pub fn inserted_height(&self) -> f64 {
        self.values()[0].max(0.0) * self.parameters.amount
    }
    pub fn scroll_offset(&self) -> f64 {
        self.user_scroll + self.inserted_height()
    }
    pub fn anchor_screen_y(&self, document_y: f64) -> f64 {
        document_y + self.inserted_height() - self.scroll_offset()
    }
    pub fn scroll_by(&mut self, delta: f64) {
        if delta.is_finite() {
            self.user_scroll = (self.user_scroll + delta).clamp(0.0, 144.0);
        }
    }
    fn typing_config(&self) -> crate::typing::TypingConfig {
        crate::typing::TypingConfig {
            mode: if self.parameters.typing_rate == 0.0 {
                crate::typing::TypingMode::Arrivals
            } else {
                crate::typing::TypingMode::Paced
            },
            units_per_second: self.parameters.typing_rate.max(1.0),
            fade_duration: Duration::from_secs_f64(self.parameters.typing_fade),
            max_latency: if self.kind == RecipeKind::StreamTyping {
                Some(Duration::from_millis(250))
            } else {
                None
            },
        }
    }
    fn begin_typing(&mut self) -> Result<(), MotionError> {
        self.phase = Phase::Working;
        self.typing = crate::typing::TypingEngine::new(self.typing_config())?;
        self.typing.advance(self.now)?;
        self.typing.set_reduced_motion(self.reduced)?;
        self.typing_sent = 0;
        self.typing_next = None;
        if self.kind == RecipeKind::Typing || self.reduced {
            self.typing.append(self.typing_unit_count, self.now)?;
            self.typing_sent = self.typing_unit_count;
        } else {
            self.typing_next = Some(self.now);
            self.advance_typing(self.now)?;
        }
        Ok(())
    }
    fn advance_typing(&mut self, now: Duration) -> Result<(), MotionError> {
        if !self.kind.is_typing() {
            return Ok(());
        }
        if !self.paused {
            while let Some(at) = self.typing_next.filter(|at| *at <= now) {
                let batch = if self.typing_sent == 0 {
                    1
                } else {
                    self.parameters.typing_batch
                };
                let count = batch.min(self.typing_unit_count - self.typing_sent);
                self.typing.append(count, at)?;
                self.typing_sent += count;
                self.typing_next = if self.typing_sent < self.typing_unit_count {
                    Some(
                        at.saturating_add(Duration::from_secs_f64(self.parameters.delay.max(0.03))),
                    )
                } else {
                    None
                };
            }
        }
        self.typing.advance(now)?;
        Ok(())
    }
    fn motion(&self, track: usize) -> Motion {
        if self.reduced {
            return Motion::new(MotionSpec::Instant);
        }
        if self.kind == RecipeKind::TextScan || matches!(self.kind, RecipeKind::Symbol(_)) {
            return Motion::new(MotionSpec::Tween {
                duration: Duration::from_secs_f64(self.parameters.duration),
                curve: TimingCurve::cubic_bezier(0.0, 0.0, 1.0, 1.0).unwrap(),
            });
        }
        if track == 1
            || (self.kind == RecipeKind::Success && track == 0)
            || (self.kind == RecipeKind::Numeric && track >= 4)
        {
            Motion::new(MotionSpec::Tween {
                duration: Duration::from_secs_f64(self.parameters.duration),
                curve: TimingCurve::cubic_bezier(0.42, 0.0, 0.58, 1.0).unwrap(),
            })
        } else {
            Motion::new(MotionSpec::Spring(
                Spring::new(1.0 / self.parameters.duration, self.parameters.damping).unwrap(),
            ))
        }
    }
    fn start(&mut self, track: usize, target: f64) -> Result<(), MotionError> {
        if track == 0 && self.kind == RecipeKind::SpinBrake {
            if self.reduced {
                self.brake = None;
                self.brake_sample = crate::Sample {
                    value: target,
                    velocity: 0.0,
                };
            } else {
                self.brake = Some(AngularBrake {
                    origin: self.brake_sample,
                    target,
                    started: self.now,
                    duration: Duration::from_secs_f64(self.parameters.duration),
                    bias: self.parameters.brake_bias,
                });
            }
            return Ok(());
        }
        let motion = self.motion(track);
        if track < 4 {
            self.tracks[track].animate_to(target, motion, self.now)?;
        } else {
            self.items[track - 4].animate_to(target, motion, self.now)?;
        }
        Ok(())
    }
    fn cue(&mut self, delay: f64, track: usize, target: f64, phase: Option<Phase>) {
        self.pending.push(Cue {
            at: self.now + Duration::from_secs_f64(delay),
            track,
            target,
            phase,
        });
    }
    pub fn set_target(&mut self, expanded: bool) -> Result<(), MotionError> {
        self.pending.clear();
        self.paused = false;
        self.target = expanded;
        if self.kind == RecipeKind::LinearRotation {
            self.phase = Phase::Working;
            self.motor.set_speed(
                (if expanded { 1.0 } else { -1.0 }) * f64::from(self.parameters.symbol_cycles)
                    / self.parameters.duration,
                self.now,
            )?;
            if !self.reduced {
                self.motor.resume(self.now)?;
            }
            return Ok(());
        }
        if self.kind.is_typing() {
            return self.begin_typing();
        }
        if !expanded
            && !matches!(
                self.kind,
                RecipeKind::ListReorder | RecipeKind::Stagger | RecipeKind::Skeleton
            )
        {
            if self.kind == RecipeKind::DragSnap {
                self.snap_target = 0.0;
            }
            if self.kind == RecipeKind::Numeric {
                self.start(4, 128.0)?;
            }
            self.phase = Phase::Cancelled;
            for i in 0..4 {
                self.start(i, 0.0)?;
            }
            return Ok(());
        }
        let delay = if self.reduced {
            0.0
        } else {
            self.parameters.delay
        };
        match self.kind {
            RecipeKind::SharedElement | RecipeKind::Panel => {
                self.start(0, 1.0)?;
                self.cue(delay, 1, 1.0, None);
                self.cue(delay, 2, 1.0, None);
            }
            RecipeKind::Success => {
                self.phase = Phase::Preparing;
                self.start(3, 1.0)?;
                self.cue(delay, 0, 1.0, Some(Phase::Working));
                self.cue(
                    delay
                        + if self.reduced {
                            0.0
                        } else {
                            self.parameters.duration
                        },
                    2,
                    1.0,
                    Some(Phase::Success),
                );
                self.cue(
                    delay
                        + if self.reduced {
                            0.0
                        } else {
                            self.parameters.duration
                        },
                    1,
                    1.0,
                    None,
                );
            }
            RecipeKind::ListReorder => {
                self.start(0, f64::from(expanded))?;
                self.start(1, f64::from(expanded))?;
                let positions = if expanded {
                    [3.0, 2.0, 0.0, 1.0, 4.0, 0.0]
                } else {
                    [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]
                };
                for (i, value) in positions.into_iter().enumerate() {
                    self.start(4 + i, value)?;
                }
            }
            RecipeKind::Stagger => {
                self.start(0, f64::from(expanded))?;
                for i in 0..6 {
                    let order = if expanded { i } else { 5 - i };
                    self.cue(delay * order as f64, 4 + i, f64::from(expanded), None);
                }
            }
            RecipeKind::Skeleton => {
                self.start(0, f64::from(expanded))?;
                self.cue(delay, 1, f64::from(expanded), None);
                self.cue(delay, 2, f64::from(expanded), None);
            }
            RecipeKind::DragSnap => {
                self.snap_target = 1.0;
                self.start(0, 1.0)?;
            }
            RecipeKind::Numeric => {
                self.start(0, 1.0)?;
                self.start(1, 1.0)?;
                self.start(4, 128.0 + self.parameters.amount * 10.0)?;
            }
            RecipeKind::Typing | RecipeKind::StreamTyping | RecipeKind::LinearRotation => {
                unreachable!("handled above")
            }
            RecipeKind::Rotate2D
            | RecipeKind::FlipX
            | RecipeKind::FlipY
            | RecipeKind::SpinBrake
            | RecipeKind::RotateSettle => {
                self.cue(delay, 0, 1.0, None);
            }
            RecipeKind::Symbol(_) => {
                self.start(0, 1.0)?;
            }
            RecipeKind::TextScan => {
                self.cue(delay, 0, 1.0, None);
            }
            RecipeKind::PageSwitch => {
                self.start(0, 1.0)?;
                self.cue(delay, 1, 1.0, None);
            }
            RecipeKind::SharedAxis | RecipeKind::FadeThrough => {
                self.start(0, 1.0)?;
                self.start(1, 1.0)?;
            }
            RecipeKind::Popover | RecipeKind::Press => {
                self.start(0, 1.0)?;
                self.start(1, 1.0)?;
                self.cue(delay, 2, 1.0, None);
            }
            RecipeKind::Reading => {
                self.start(0, 1.0)?;
                self.start(1, 1.0)?;
            }
        }
        self.advance(self.now)?;
        Ok(())
    }
    pub fn replay(&mut self) -> Result<(), MotionError> {
        let kind = self.kind;
        let parameters = self.parameters;
        let reduced = self.reduced;
        let typing_total = self.typing_unit_count;
        *self = Self::new(kind);
        self.typing_unit_count = typing_total;
        self.parameters = parameters;
        self.reduced = reduced;
        self.set_target(true)
    }
    /// Hold the exact presented values and drop every future cue.
    pub fn interrupt(&mut self) -> Result<(), MotionError> {
        self.pending.clear();
        self.motor.pause(self.now)?;
        if self.kind.is_typing() {
            self.typing.pause(self.now)?;
        }
        self.paused = true;
        self.brake = None;
        self.brake_sample.velocity = 0.0;
        self.phase = Phase::Cancelled;
        for track in self.tracks.iter_mut().chain(self.items.iter_mut()) {
            track.cancel(self.now, CancelReason::Explicit, CancelBehavior::Hold)?;
        }
        Ok(())
    }
    pub fn set_parameters(&mut self, parameters: RecipeParameters) -> Result<(), MotionError> {
        parameters.validate()?;
        let old_parameters = self.parameters;
        let sample = crate::Sample {
            value: self.values()[0],
            velocity: self.velocities()[0],
        };
        self.parameters = parameters;
        if self.kind == RecipeKind::LinearRotation {
            return self.motor.set_speed(
                (if self.target { 1.0 } else { -1.0 }) * f64::from(parameters.symbol_cycles)
                    / parameters.duration,
                self.now,
            );
        }
        if self.kind.is_typing() {
            self.typing.set_config(self.typing_config())?;
            return Ok(());
        }
        let remap_rotation = self.kind.is_rotation()
            && old_parameters.rotation_degrees != parameters.rotation_degrees;
        let remap = (self.kind == RecipeKind::Reading
            && old_parameters.amount != parameters.amount)
            || remap_rotation;
        let ratio = if remap_rotation {
            old_parameters.rotation_degrees / parameters.rotation_degrees
        } else {
            old_parameters.amount / parameters.amount
        };
        if remap && self.kind == RecipeKind::SpinBrake {
            self.brake_sample = crate::Sample {
                value: sample.value * ratio,
                velocity: sample.velocity * ratio,
            };
        } else if remap {
            self.tracks[0] = Animated::new(sample.value * ratio)?;
        }
        if self.paused {
            return Ok(());
        }
        if self.kind == RecipeKind::DragSnap {
            self.pending.clear();
            self.start(0, self.snap_target)?;
        } else if self.kind == RecipeKind::SpinBrake {
            self.pending.clear();
            self.start(0, f64::from(self.target))?;
        } else {
            self.set_target(self.target)?;
        }
        if remap && self.kind != RecipeKind::SpinBrake {
            self.pending.retain(|cue| cue.track != 0);
            self.tracks[0].animate_to_with_velocity(
                f64::from(self.target),
                sample.velocity * ratio,
                self.motion(0),
                self.now,
            )?;
        }
        Ok(())
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) -> Result<(), MotionError> {
        if self.reduced == reduced {
            return Ok(());
        }
        self.reduced = reduced;
        if self.kind == RecipeKind::LinearRotation {
            if reduced {
                self.motor.pause(self.now)?;
                self.paused = true;
            }
            return Ok(());
        }
        if self.kind.is_typing() {
            self.typing.set_reduced_motion(reduced)?;
            if reduced {
                self.typing
                    .append(self.typing_unit_count - self.typing_sent, self.now)?;
                self.typing_sent = self.typing_unit_count;
                self.typing_next = None;
            }
            return Ok(());
        }
        if reduced {
            self.set_target(self.target)?;
            if self.kind == RecipeKind::Stagger {
                self.advance(self.now)?;
            }
        }
        Ok(())
    }
    pub fn advance(&mut self, now: Duration) -> Result<(), MotionError> {
        if now < self.now {
            return Err(MotionError::TimeWentBackwards);
        }
        self.motor.advance(now)?;
        self.pending.sort_by_key(|cue| cue.at);
        while self.pending.first().is_some_and(|cue| cue.at <= now) {
            let cue = self.pending.remove(0);
            for track in self.tracks.iter_mut().chain(self.items.iter_mut()) {
                track.advance(cue.at)?;
            }
            self.advance_brake(cue.at);
            self.now = cue.at;
            if let Some(phase) = cue.phase {
                self.phase = phase;
            }
            self.start(cue.track, cue.target)?;
        }
        for track in self.tracks.iter_mut().chain(self.items.iter_mut()) {
            track.advance(now)?;
        }
        self.advance_brake(now);
        self.advance_typing(now)?;
        self.now = now;
        Ok(())
    }
    fn advance_brake(&mut self, now: Duration) {
        if let Some(flight) = self.brake {
            self.brake_sample = flight.sample(now);
            if now.saturating_sub(flight.started) >= flight.duration {
                self.brake = None;
            }
        }
    }
}

/// A moving planar band projected onto text-local coordinates. Painting through
/// a glyph mask keeps the highlight inside strokes; no component is owned here.
#[derive(Clone, Copy, Debug)]
pub struct TextScanBand {
    pub angle_degrees: f64,
    pub width: f64,
}
impl TextScanBand {
    pub fn span(self, size: [f64; 2], row_y: f64, progress: f64) -> Option<(f64, f64)> {
        let [w, h] = size;
        if [w, h, row_y, progress, self.angle_degrees, self.width]
            .iter()
            .any(|v| !v.is_finite())
            || w <= 0.0
            || h <= 0.0
            || self.width <= 0.0
            || self.angle_degrees.abs() > 75.0
        {
            return None;
        }
        let angle = self.angle_degrees.to_radians();
        let c = angle.cos();
        let s = angle.sin();
        let min = (h * s).min(0.0);
        let max = w * c + (h * s).max(0.0);
        let center = min - self.width / 2.0 + progress.clamp(0.0, 1.0) * (max - min + self.width);
        let x = (center - row_y * s) / c;
        let half = self.width / (2.0 * c);
        Some((x - half, x + half))
    }
}

/// Quintic angular travel with adjustable time bias: continuous initial
/// velocity and zero terminal velocity/acceleration. Retarget from presentation.
#[derive(Clone, Copy, Debug)]
struct AngularBrake {
    bias: f64,
    origin: crate::Sample<f64>,
    target: f64,
    started: Duration,
    duration: Duration,
}
impl AngularBrake {
    fn sample(self, now: Duration) -> crate::Sample<f64> {
        let duration = self.duration.as_secs_f64();
        let t = (now.saturating_sub(self.started).as_secs_f64() / duration).clamp(0.0, 1.0);
        if t >= 1.0 {
            return crate::Sample {
                value: self.target,
                velocity: 0.0,
            };
        }
        let derivative = 1.0 + 4.0 * self.bias * t * (1.0 - t) * (1.0 - 2.0 * t);
        let t = t + 2.0 * self.bias * t * t * (1.0 - t) * (1.0 - t);
        let d = self.target - self.origin.value;
        let v = self.origin.velocity * duration;
        let a = 10.0 * d - 6.0 * v;
        let b = -15.0 * d + 8.0 * v;
        let c = 6.0 * d - 3.0 * v;
        crate::Sample {
            value: self.origin.value + v * t + a * t.powi(3) + b * t.powi(4) + c * t.powi(5),
            velocity: (v + 3.0 * a * t.powi(2) + 4.0 * b * t.powi(3) + 5.0 * c * t.powi(4))
                * derivative
                / duration,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationAxis {
    X,
    Y,
    Z,
}
/// Perspective-project one local point; the Gallery uses a bounded vector card.
pub fn project_rotation(
    point: [f64; 2],
    angle_degrees: f64,
    axis: RotationAxis,
    depth: f64,
) -> Option<[f64; 2]> {
    if point
        .iter()
        .chain([angle_degrees, depth].iter())
        .any(|v| !v.is_finite())
        || depth <= 0.0
    {
        return None;
    }
    let [x, y] = point;
    let a = angle_degrees.to_radians();
    let (s, c) = a.sin_cos();
    let (x, y, z) = match axis {
        RotationAxis::X => (x, y * c, y * s),
        RotationAxis::Y => (x * c, y, -x * s),
        RotationAxis::Z => (x * c - y * s, x * s + y * c, 0.0),
    };
    let denominator = depth - z;
    if denominator <= 1.0 {
        return None;
    }
    Some([x * depth / denominator, y * depth / denominator])
}
