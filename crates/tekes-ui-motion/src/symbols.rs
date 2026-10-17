//! Platform-neutral symbol choreography inspired by Apple's public effect families.
//! These equations/defaults are TekesUI's, not Apple's private animation curves.
//! Hosts supply stable layers, stroke paths and correspondence; no assets or clocks live here.
use crate::MotionError;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolEffect {
    Bounce,
    Pulse,
    VariableColor,
    Scale,
    Appear,
    Disappear,
    Replace,
    Breathe,
    Wiggle,
    Rotate,
    MagicReplace,
    DrawOn,
    DrawOff,
    VariableDraw,
}
impl SymbolEffect {
    pub const ALL: [Self; 14] = [
        Self::Bounce,
        Self::Pulse,
        Self::VariableColor,
        Self::Scale,
        Self::Appear,
        Self::Disappear,
        Self::Replace,
        Self::Breathe,
        Self::Wiggle,
        Self::Rotate,
        Self::MagicReplace,
        Self::DrawOn,
        Self::DrawOff,
        Self::VariableDraw,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bounce => "Bounce 弹跳",
            Self::Pulse => "Pulse 脉冲",
            Self::VariableColor => "Variable Color 分层",
            Self::Scale => "Scale 状态缩放",
            Self::Appear => "Appear 出现",
            Self::Disappear => "Disappear 消失",
            Self::Replace => "Replace 图标替换",
            Self::Breathe => "Breathe 呼吸",
            Self::Wiggle => "Wiggle 摇摆",
            Self::Rotate => "Rotate 旋转",
            Self::MagicReplace => "Magic Replace 关联替换",
            Self::DrawOn => "Draw On 描绘出现",
            Self::DrawOff => "Draw Off 描绘消失",
            Self::VariableDraw => "Variable Draw 进度描绘",
        }
    }
    /// Tuned demonstration values owned by TekesUI, not system defaults.
    pub fn preview_defaults(self) -> (f64, SymbolOptions) {
        let (duration, cycles) = match self {
            Self::Bounce => (0.7, 1),
            Self::Pulse => (1.6, 2),
            Self::Breathe => (2.4, 1),
            Self::Wiggle => (0.9, 3),
            Self::Rotate => (1.6, 1),
            Self::Scale | Self::Appear | Self::Disappear => (0.6, 1),
            Self::Replace | Self::MagicReplace => (0.8, 1),
            Self::VariableColor => (1.8, 1),
            Self::DrawOn | Self::DrawOff | Self::VariableDraw => (1.2, 1),
        };
        (
            duration,
            SymbolOptions {
                cycles,
                ..SymbolOptions::default()
            },
        )
    }
    pub const fn ios_version(self) -> u8 {
        match self {
            Self::Breathe | Self::Wiggle | Self::Rotate | Self::MagicReplace => 18,
            Self::DrawOn | Self::DrawOff | Self::VariableDraw => 26,
            _ => 17,
        }
    }
    pub const fn requires_paths(self) -> bool {
        matches!(self, Self::DrawOn | Self::DrawOff | Self::VariableDraw)
    }
    /// Generic fallback is explicit; a raster icon cannot acquire stroke order or
    /// topology simply by choosing an effect. Variable Draw is value-driven rendering.
    pub const fn resolve(self, capabilities: SymbolCapabilities) -> Self {
        match self {
            Self::MagicReplace if !capabilities.correspondence => Self::Replace,
            Self::DrawOn if !capabilities.ordered_paths => Self::Appear,
            Self::DrawOff if !capabilities.ordered_paths => Self::Disappear,
            Self::VariableDraw if !capabilities.ordered_paths => Self::VariableColor,
            _ => self,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolCapabilities {
    pub ordered_paths: bool,
    pub correspondence: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolScope {
    Whole,
    ByLayer,
    Individually,
}
#[derive(Clone, Copy, Debug)]
pub struct SymbolOptions {
    pub strength: f64,
    /// Finite repeats; the host may schedule another run while its activity is active.
    pub cycles: u8,
    /// Normalized layer offset relative to one run, independent of wall-clock speed.
    pub layer_delay: f64,
    pub scope: SymbolScope,
    /// -1 reverses rotation / vertical travel. Reading-direction mapping is the host's job.
    pub direction: f64,
    /// Wiggle translation direction in radians; None uses angular wiggle.
    pub wiggle_translation_angle: Option<f64>,
    pub breathe_pulse: bool,
    pub cumulative: bool,
    pub hide_inactive: bool,
    pub reversing: bool,
    pub draw_reversed: bool,
}
impl Default for SymbolOptions {
    fn default() -> Self {
        Self {
            strength: 0.6,
            cycles: 3,
            layer_delay: 0.12,
            scope: SymbolScope::Whole,
            direction: 1.0,
            wiggle_translation_angle: None,
            breathe_pulse: true,
            cumulative: false,
            hide_inactive: false,
            reversing: false,
            draw_reversed: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymbolFrame {
    pub scale: f64,
    pub rotation: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub opacity: f64,
    /// Arc-length interval of the ordered stroke to render, normalized 0..1.
    pub draw_start: f64,
    pub draw_end: f64,
    /// Target weight for replacement. Matching shared layers stay outside this crossfade.
    pub replacement: f64,
}
impl Default for SymbolFrame {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            opacity: 1.0,
            draw_start: 0.0,
            draw_end: 1.0,
            replacement: 0.0,
        }
    }
}
/// Sample one stable layer. `progress` is continuous presentation progress, so
/// reversing the controlling channel or holding it pauses every property together.
/// Param edits can change the pose immediately; this is not a per-property spring solver.
pub fn sample(
    effect: SymbolEffect,
    progress: f64,
    layer: usize,
    layers: usize,
    options: SymbolOptions,
    reduced_motion: bool,
) -> Result<SymbolFrame, MotionError> {
    if !progress.is_finite()
        || !options.strength.is_finite()
        || !options.layer_delay.is_finite()
        || options
            .wiggle_translation_angle
            .is_some_and(|v| !v.is_finite())
        || !options.direction.is_finite()
        || !(0.0..=1.0).contains(&options.strength)
        || !(0.0..=0.6).contains(&options.layer_delay)
        || !(-1.0..=1.0).contains(&options.direction)
        || !(1..=8).contains(&options.cycles)
        || layers == 0
        || layers > 65536
        || layer >= layers
    {
        return Err(MotionError::InvalidOptions);
    }
    let p = progress.clamp(0.0, 1.0);
    let u = match options.scope {
        SymbolScope::Whole => p,
        SymbolScope::ByLayer => (p * (1.0 + options.layer_delay * (layers - 1) as f64)
            - options.layer_delay * layer as f64)
            .clamp(0.0, 1.0),
        SymbolScope::Individually => (p * layers as f64 - layer as f64).clamp(0.0, 1.0),
    };
    let smooth = |x: f64| x * x * (3.0 - 2.0 * x);
    let q = smooth(u);
    let wave = (1.0 - (TAU * f64::from(options.cycles) * u).cos()) * 0.5;
    let strength = options.strength;
    let mut f = SymbolFrame::default();
    match effect {
        SymbolEffect::Bounce if !reduced_motion => {
            f.offset_y = -24.0
                * strength
                * (PI * f64::from(options.cycles) * u).sin().abs()
                * options.direction;
            f.scale = 1.0 + 0.15 * strength * wave;
        }
        SymbolEffect::Pulse if !reduced_motion => f.opacity = 1.0 - 0.7 * strength * wave,
        SymbolEffect::Breathe if !reduced_motion => {
            f.scale = 1.0 + 0.24 * strength * wave;
            if options.breathe_pulse {
                f.opacity = 1.0 - 0.45 * strength * wave;
            }
        }
        SymbolEffect::Wiggle if !reduced_motion => {
            let oscillation = strength
                * options.direction
                * (TAU * f64::from(options.cycles) * u).sin()
                * (PI * u).sin();
            if let Some(angle) = options.wiggle_translation_angle {
                f.offset_x = 18.0 * oscillation * angle.cos();
                f.offset_y = 18.0 * oscillation * angle.sin();
            } else {
                f.rotation = 0.5 * oscillation;
            }
        }
        SymbolEffect::Rotate if !reduced_motion => {
            f.rotation = TAU * f64::from(options.cycles) * u * options.direction
        }
        SymbolEffect::Scale if !reduced_motion => {
            f.scale = 1.0 + 0.4 * strength * q * options.direction
        }
        SymbolEffect::Appear | SymbolEffect::Disappear => {
            let visible = if effect == SymbolEffect::Appear {
                q
            } else {
                1.0 - q
            };
            f.opacity = visible;
            if !reduced_motion {
                f.scale = 0.65 + 0.35 * visible;
                f.offset_y = (1.0 - visible) * 20.0 * options.direction;
            }
        }
        SymbolEffect::VariableColor => {
            // Rendering strength, not an RGB hue change: Apple's variable color uses layer opacity.
            let value = if options.reversing {
                1.0 - (2.0 * p - 1.0).abs()
            } else {
                p
            };
            let local = value * layers as f64 - layer as f64;
            let active = if options.cumulative {
                smooth(local.clamp(0.0, 1.0))
            } else {
                (1.0 - (local - 0.5).abs() * 2.0).clamp(0.0, 1.0)
            };
            f.opacity = if reduced_motion {
                1.0
            } else {
                let floor = if options.hide_inactive { 0.0 } else { 0.2 };
                floor + (1.0 - floor) * active
            };
        }
        SymbolEffect::Replace | SymbolEffect::MagicReplace => f.replacement = q,
        SymbolEffect::DrawOn | SymbolEffect::VariableDraw => {
            f.draw_end = if reduced_motion {
                if p >= 0.5 { 1.0 } else { 0.0 }
            } else {
                u
            }
        }
        SymbolEffect::DrawOff => {
            if reduced_motion {
                f.opacity = if p >= 0.5 { 0.0 } else { 1.0 };
            } else if options.draw_reversed {
                f.draw_end = 1.0 - u;
            } else {
                f.draw_start = u;
            }
        }
        _ => {}
    }
    Ok(f)
}
