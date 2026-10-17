//! Independent motion-page state. Apple references and the canonical JSON
//! Design System have separate catalogs; neither overwrites the other.
#[path = "symbol_assets.rs"]
pub mod symbol_assets;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};
use tekes_ui_controls::Button;
use tekes_ui_motion::apple_defaults::{AppleDefault, Parameters as AppleParameters};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresetCatalog {
    Recipes,
    Symbols,
    Apple,
    DesignSystem,
}

pub fn apple_presets() -> Vec<Preset> {
    AppleDefault::ALL.into_iter().map(|preset| {
        let parameters = match preset.parameters() {
            AppleParameters::ResponseSpring { response,damping_fraction,blend_duration } =>
                format!("response {response:.2} · dampingFraction {damping_fraction:.2} · blend {blend_duration:.2}"),
            AppleParameters::Spring { duration,bounce } => format!("duration {duration:.2}s · bounce {bounce:.2}"),
            AppleParameters::Tween { duration,control_points } => format!("{duration:.2}s · Bézier {control_points:?}"),
        };
        Preset { id:format!("apple.{}",preset.id()),label:preset.name().into(),
            source:format!("Apple .{} · {parameters}",preset.id()),motion:preset.motion() }
    }).collect()
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

/// Complete display units for the preview, including indivisible emoji and
/// combining sequences. Production callers segment their incoming text before
/// appending unit counts to the independent engine.
pub fn typing_fixture(locale: super::locale::Locale, target: bool) -> Vec<&'static str> {
    if locale == super::locale::Locale::En {
        use unicode_segmentation::UnicodeSegmentation;
        return if target {
            "Text appears naturally over time, independent of refresh rate.\nArriving characters can appear together.\nTyping 👩‍💻 e\u{301} 🇨🇳"
        } else {
            "A different passage is ready.\nThe old reveal queue is cancelled; the new message starts here.\nKeep reading without a jump."
        }.graphemes(true).collect();
    }
    let mut units = if target {
        vec![
            "文",
            "字",
            "会",
            "按",
            "时",
            "间",
            "自",
            "然",
            "浮",
            "现",
            "，",
            "不",
            "受",
            "刷",
            "新",
            "率",
            "影",
            "响",
            "。",
            "\n",
            "一",
            "次",
            "到",
            "达",
            "多",
            "个",
            "字",
            "，",
            "就",
            "可",
            "以",
            "一",
            "批",
            "显",
            "示",
            "。",
            "\n",
            "T",
            "y",
            "p",
            "i",
            "n",
            "g",
            " ",
            "👩‍💻",
            " ",
            "e\u{301}",
            " ",
            "🇨🇳",
        ]
    } else {
        vec![
            "已", "切", "换", "到", "另", "一", "段", "文", "本", "。", "\n", "旧", "消", "息",
            "的", "待", "显", "示", "队", "列", "已", "取", "消", "，", "新", "消", "息", "从",
            "这", "里", "开", "始", "。", "\n", "继", "续", "阅", "读", "，", "位", "置", "保",
            "持", "稳", "定", "。",
        ]
    };
    units.shrink_to_fit();
    units
}

#[derive(Clone, Debug)]
pub struct MotionPreview {
    pub locale: super::locale::Locale,
    pub symbol_icon: tekes_ui_icons::Icon,
    pub symbol_assets: symbol_assets::SymbolAssets,
    pub recipe: tekes_ui_motion::recipes::RecipePlayer,
    pub recipe_dragging: bool,
    gesture_sample: Option<(f64, std::time::Instant)>,
    pub gesture_velocity: f64,
    pub channel: Animated<f64>,
    pub presets: Vec<Preset>,
    pub catalog: PresetCatalog,
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
    /// Shared production Button models, keyed by stable Motion action ids.
    pub button_models: HashMap<u64, Button>,
    /// Final geometry from the most recent Gallery layout/render pass. Pointer
    /// handling consumes this snapshot so it cannot drift from painted frames.
    pub placement: Option<MotionPlacement>,
    pub focus_order: Vec<u64>,
    pub focused_button: Option<u64>,
    interrupt_at: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct MotionPlacement {
    pub stage: tekes_ui_scene::Rect,
    pub sliders: [tekes_ui_scene::Rect; 4],
}

impl Default for MotionPreview {
    fn default() -> Self {
        let presets = apple_presets();
        let selected = presets.iter().position(|p| p.id == "apple.spring").unwrap();
        let mut state = Self {
            locale: super::locale::Locale::default(),
            symbol_icon: tekes_ui_icons::Icon::NewGoal,
            symbol_assets: Default::default(),
            recipe: Default::default(),
            recipe_dragging: false,
            gesture_sample: None,
            gesture_velocity: 0.0,
            channel: Animated::new(0.12).unwrap(),
            presets,
            catalog: PresetCatalog::Apple,
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
            button_models: HashMap::new(),
            placement: None,
            focus_order: Vec::new(),
            focused_button: None,
            interrupt_at: None,
        };
        state.load_parameters();
        // Start stationary: opening a page never starts an unsolicited loop.
        state
    }
}

impl MotionPreview {
    pub fn prepare_symbols(
        &mut self,
        renderer: &mut tekes_ui_render::Renderer,
        theme: tekes_ui_tokens::Theme,
        scale: f32,
    ) {
        if self.catalog != PresetCatalog::Symbols {
            return;
        }
        let result = match self.recipe.kind {
            tekes_ui_motion::recipes::RecipeKind::Symbol(effect) => self.symbol_assets.prepare(
                renderer,
                self.symbol_icon,
                effect,
                self.recipe.values()[0],
                self.recipe.parameters,
                self.reduced_motion,
                theme,
                scale,
            ),
            tekes_ui_motion::recipes::RecipeKind::LinearRotation => {
                self.symbol_assets.prepare_linear(
                    renderer,
                    self.symbol_icon,
                    self.recipe.values()[0],
                    self.recipe.parameters,
                    self.reduced_motion,
                    theme,
                    scale,
                )
            }
            _ => return,
        };
        if let Err(error) = result {
            self.error = Some(error);
        }
    }
    pub fn select_symbol_icon(&mut self, index: usize) {
        if let Some(icon) = tekes_ui_icons::Icon::ALL.get(index) {
            self.symbol_icon = *icon;
            self.replay();
        }
    }
    pub fn select_catalog(&mut self, catalog: PresetCatalog) -> bool {
        if self.catalog == catalog {
            return false;
        }
        self.deactivate();
        self.catalog = catalog;
        self.presets = match catalog {
            PresetCatalog::Symbols => {
                let mut presets: Vec<Preset> = tekes_ui_motion::symbols::SymbolEffect::ALL
                    .into_iter()
                    .map(|effect| Preset {
                        id: format!("symbol.{effect:?}"),
                        label: effect.label().into(),
                        source: format!(
                            "Apple iOS {}+ 效果族 · TekesUI 自定曲线与参数 · 非系统逐帧复刻",
                            effect.ios_version()
                        ),
                        motion: AppleDefault::Snappy.motion(),
                    })
                    .collect();
                presets.push(Preset {
                    id: "tekes.linear_rotation".into(),
                    label: "Linear rotation".into(),
                    source: "TekesUI continuous angular motor; constant speed, no easing".into(),
                    motion: AppleDefault::Snappy.motion(),
                });
                presets
            }
            PresetCatalog::Apple => apple_presets(),
            PresetCatalog::DesignSystem => design_presets(),
            PresetCatalog::Recipes => tekes_ui_motion::recipes::RecipeKind::ALL
                .into_iter()
                .map(|kind| Preset {
                    id: format!("recipe.{kind:?}"),
                    label: kind.label().into(),
                    source: "TekesUI 实验配方 · 非品牌内部参数".into(),
                    motion: AppleDefault::Snappy.motion(),
                })
                .collect(),
        };
        let index = if catalog == PresetCatalog::Apple {
            1
        } else {
            0
        };
        self.select(index)
    }
    pub fn is_recipe(&self) -> bool {
        matches!(
            self.catalog,
            PresetCatalog::Recipes | PresetCatalog::Symbols
        )
    }
    pub fn parameter_label(&self, p: Parameter) -> &'static str {
        if !self.is_recipe() {
            return p.label();
        }
        if self.catalog == PresetCatalog::Symbols
            && self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::LinearRotation
        {
            return match p {
                Parameter::Frequency => "Rotation period / s",
                Parameter::Damping => "Not used",
                Parameter::Duration => "Layer phase / turns",
                Parameter::ReleaseVelocity => "Turns per period",
            };
        }
        if self.catalog == PresetCatalog::Symbols {
            return match p {
                Parameter::Frequency => "整轮时长 / s",
                Parameter::Damping => "效果强度 / 0–1",
                Parameter::Duration => "分层错开 / 轮比例",
                Parameter::ReleaseVelocity => "有限循环 / 次",
            };
        }
        if self.recipe.kind.is_typing() {
            return match p {
                Parameter::Frequency => {
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::StreamTyping {
                        "限速 / 字·s⁻¹（0 即时）"
                    } else {
                        "显示速度 / 字·s⁻¹"
                    }
                }
                Parameter::Damping => "柔和浮现 / s",
                Parameter::Duration => "到达间隔 / s",
                Parameter::ReleaseVelocity => "每批到达 / 字",
            };
        }
        if self.recipe.kind.is_rotation() {
            return match p {
                Parameter::Frequency => "旋转时长 / s",
                Parameter::Damping => {
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::SpinBrake {
                        "制动偏置 / 提前量"
                    } else {
                        "落定阻尼比 / ζ"
                    }
                }
                Parameter::Duration => "启动延迟 / s",
                Parameter::ReleaseVelocity => "目标角度 / °",
            };
        }
        if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::TextScan {
            return match p {
                Parameter::Frequency => "扫描时长 / s",
                Parameter::Damping => "扫描方向 / °",
                Parameter::Duration => "起始间隔 / s",
                Parameter::ReleaseVelocity => "亮带宽度 / pt",
            };
        }
        match p {
            Parameter::Frequency => "主体时长 / s",
            Parameter::Damping => "阻尼比 / ζ",
            Parameter::Duration => "阶段延迟 / s",
            Parameter::ReleaseVelocity => match self.recipe.kind {
                tekes_ui_motion::recipes::RecipeKind::SharedElement => "圆角幅度 / pt",
                tekes_ui_motion::recipes::RecipeKind::Panel => "位移幅度 / pt",
                tekes_ui_motion::recipes::RecipeKind::Success => "脉冲幅度 / pt",
                tekes_ui_motion::recipes::RecipeKind::Reading => "插入高度 / pt",
                tekes_ui_motion::recipes::RecipeKind::ListReorder => "行间距 / pt",
                tekes_ui_motion::recipes::RecipeKind::DragSnap => "惯性投射 / pt",
                tekes_ui_motion::recipes::RecipeKind::Numeric => "数值增量 / ×10",
                _ => "位移 / 缩放幅度",
            },
        }
    }
    pub fn parameter_range(&self, p: Parameter) -> (f64, f64) {
        if !self.is_recipe() {
            return p.range();
        }
        if self.catalog == PresetCatalog::Symbols {
            return match p {
                Parameter::Frequency => (0.3, 4.0),
                Parameter::Damping => (0.0, 1.0),
                Parameter::Duration => (0.0, 0.6),
                Parameter::ReleaseVelocity => (1.0, 8.0),
            };
        }
        if self.recipe.kind.is_typing() {
            return match p {
                Parameter::Frequency => (
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::StreamTyping {
                        0.0
                    } else {
                        5.0
                    },
                    160.0,
                ),
                Parameter::Damping => (0.0, 0.8),
                Parameter::Duration => (0.03, 0.6),
                Parameter::ReleaseVelocity => (1.0, 12.0),
            };
        }
        if self.recipe.kind.is_rotation() {
            return match p {
                Parameter::Frequency => (0.3, 4.0),
                Parameter::ReleaseVelocity => (
                    15.0,
                    match self.recipe.kind {
                        tekes_ui_motion::recipes::RecipeKind::FlipX
                        | tekes_ui_motion::recipes::RecipeKind::FlipY => 360.0,
                        tekes_ui_motion::recipes::RecipeKind::SpinBrake => 2160.0,
                        _ => 720.0,
                    },
                ),
                Parameter::Damping => {
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::SpinBrake {
                        (-1.0, 1.0)
                    } else {
                        (0.35, 1.5)
                    }
                }
                Parameter::Duration => (0.0, 0.6),
            };
        }
        if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::TextScan {
            return match p {
                Parameter::Frequency => (0.3, 4.0),
                Parameter::Damping => (-75.0, 75.0),
                Parameter::Duration => (0.0, 0.6),
                Parameter::ReleaseVelocity => (16.0, 160.0),
            };
        }
        match p {
            Parameter::Frequency => (0.15, 1.5),
            Parameter::Damping => (0.35, 1.5),
            Parameter::Duration => (0.0, 0.6),
            Parameter::ReleaseVelocity => (16.0, 160.0),
        }
    }
    pub fn drag_recipe(&mut self, position: f64) {
        let time = std::time::Instant::now();
        if let Some((old, at)) = self.gesture_sample {
            let dt = time.duration_since(at).as_secs_f64();
            if dt > 0.001 {
                self.gesture_velocity = ((position - old) / dt).clamp(-6.0, 6.0);
            }
        } else {
            self.gesture_velocity = 0.0;
        }
        self.gesture_sample = Some((position, time));
        self.recipe_dragging = true;
        self.interrupt_at = None;
        if let Err(e) = self.recipe.drag_to(position) {
            self.error = Some(e.to_string());
        }
    }
    pub fn release_recipe(&mut self) {
        if !self.recipe_dragging {
            return;
        }
        self.recipe_dragging = false;
        if self
            .gesture_sample
            .is_some_and(|(_, at)| at.elapsed().as_secs_f64() > 0.15)
        {
            self.gesture_velocity = 0.0;
        }
        self.gesture_sample = None;
        if let Err(e) = self.recipe.release_drag(self.gesture_velocity) {
            self.error = Some(e.to_string());
        }
    }
    pub fn resume_recipe(&mut self) {
        self.interrupt_at = None;
        if let Err(e) = self.recipe.resume() {
            self.error = Some(e.to_string());
        }
    }
    pub fn pause_recipe(&mut self) {
        self.interrupt_at = None;
        if let Err(e) = self.recipe.interrupt() {
            self.error = Some(e.to_string());
        }
    }
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
        if self.catalog == PresetCatalog::Symbols {
            use tekes_ui_motion::symbols::SymbolEffect::*;
            if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::LinearRotation {
                return p != Parameter::Damping;
            }
            let tekes_ui_motion::recipes::RecipeKind::Symbol(effect) = self.recipe.kind else {
                return false;
            };
            return match p {
                Parameter::Frequency | Parameter::Duration => true,
                Parameter::Damping => matches!(effect, Bounce | Pulse | Breathe | Scale | Wiggle),
                Parameter::ReleaseVelocity => {
                    matches!(effect, Bounce | Pulse | Breathe | Wiggle | Rotate)
                }
            };
        }
        if self.is_recipe() && self.recipe.kind.is_typing() {
            return self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::StreamTyping
                || matches!(p, Parameter::Frequency | Parameter::Damping);
        }
        if self.is_recipe() {
            use tekes_ui_motion::recipes::RecipeKind::*;
            return match p {
                Parameter::Duration => !matches!(
                    self.recipe.kind,
                    Reading | DragSnap | ListReorder | SharedAxis | Numeric
                ),
                Parameter::Damping => !matches!(self.recipe.kind, Numeric | Skeleton),
                _ => true,
            };
        }
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
        if self.is_recipe() {
            self.recipe = tekes_ui_motion::recipes::RecipePlayer::new(
                if self.catalog == PresetCatalog::Symbols {
                    if index == tekes_ui_motion::symbols::SymbolEffect::ALL.len() {
                        tekes_ui_motion::recipes::RecipeKind::LinearRotation
                    } else {
                        tekes_ui_motion::recipes::RecipeKind::Symbol(
                            tekes_ui_motion::symbols::SymbolEffect::ALL[index],
                        )
                    }
                } else {
                    tekes_ui_motion::recipes::RecipeKind::ALL[index]
                },
            );
            if self.recipe.kind.is_typing() {
                self.recipe.typing_unit_count = typing_fixture(self.locale, true).len();
            }
            self.recipe.set_reduced_motion(self.reduced_motion).unwrap();
        }
        self.custom = false;
        self.release_velocity = 0.0;
        self.load_parameters();
        self.replay();
        true
    }
    pub fn replay(&mut self) {
        if self.is_recipe() {
            self.interrupt_at = None;
            self.trace.clear();
            self.events.clear();
            self.error = None;
            if self.recipe.kind.is_typing() {
                self.recipe.typing_unit_count = typing_fixture(self.locale, true).len();
            }
            if let Err(e) = self.recipe.replay() {
                self.error = Some(e.to_string());
            }
            self.now = self.recipe.now();
            self.record();
            return;
        }
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
        if self.is_recipe() {
            self.interrupt_at = None;
            if (target - 0.5).abs() < 0.01 {
                self.pause_recipe();
            } else {
                if self.recipe.kind.is_typing() {
                    self.recipe.typing_unit_count = typing_fixture(self.locale, target > 0.5).len();
                }
                if let Err(e) = self.recipe.set_target(target > 0.5) {
                    self.error = Some(e.to_string());
                }
            }
            self.record();
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
        if self.is_recipe() {
            if !self.reduced_motion {
                self.interrupt_at = Some(Duration::from_secs_f64(
                    (if self.catalog == PresetCatalog::Symbols {
                        0.0
                    } else {
                        self.recipe.parameters.delay
                    }) + self.recipe.parameters.duration * 0.45,
                ));
            }
            return;
        }
        if !self.reduced_motion && self.channel.is_active() {
            let delay = match self.motion().map(|motion| motion.spec) {
                Ok(MotionSpec::Tween { duration, .. }) => duration.mul_f64(0.4),
                _ => Duration::from_millis(140),
            };
            self.interrupt_at = Some(delay);
        }
    }
    pub fn parameter_value(&self, p: Parameter) -> f64 {
        if self.catalog == PresetCatalog::Symbols {
            let params = self.recipe.parameters;
            return match p {
                Parameter::Frequency => params.duration,
                Parameter::Damping => params.symbol_strength,
                Parameter::Duration => params.delay,
                Parameter::ReleaseVelocity => f64::from(params.symbol_cycles),
            };
        }
        if self.is_recipe() && self.recipe.kind.is_typing() {
            let params = self.recipe.parameters;
            return match p {
                Parameter::Frequency => params.typing_rate,
                Parameter::Damping => params.typing_fade,
                Parameter::Duration => params.delay,
                Parameter::ReleaseVelocity => params.typing_batch as f64,
            };
        }
        if self.is_recipe() {
            let params = self.recipe.parameters;
            return match p {
                Parameter::Frequency => params.duration,
                Parameter::Damping => {
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::SpinBrake {
                        params.brake_bias
                    } else if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::TextScan {
                        params.scan_angle
                    } else {
                        params.damping
                    }
                }
                Parameter::Duration => params.delay,
                Parameter::ReleaseVelocity => {
                    if self.recipe.kind.is_rotation() {
                        params.rotation_degrees
                    } else {
                        params.amount
                    }
                }
            };
        }
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
        let (lo, hi) = self.parameter_range(p);
        let value = value.clamp(lo, hi);
        if (self.parameter_value(p) - value).abs() < 1e-9 {
            return false;
        }
        if self.is_recipe() {
            self.interrupt_at = None;
            let mut params = self.recipe.parameters;
            if self.catalog == PresetCatalog::Symbols {
                match p {
                    Parameter::Frequency => params.duration = value,
                    Parameter::Damping => params.symbol_strength = value,
                    Parameter::Duration => params.delay = value,
                    Parameter::ReleaseVelocity => params.symbol_cycles = value.round() as u8,
                }
                self.custom = true;
                if let Err(e) = self.recipe.set_parameters(params) {
                    self.error = Some(e.to_string());
                }
                self.record();
                return true;
            }
            if self.recipe.kind.is_typing() {
                match p {
                    Parameter::Frequency => params.typing_rate = value,
                    Parameter::Damping => params.typing_fade = value,
                    Parameter::Duration => params.delay = value,
                    Parameter::ReleaseVelocity => params.typing_batch = value.round() as usize,
                }
                self.custom = true;
                if let Err(e) = self.recipe.set_parameters(params) {
                    self.error = Some(e.to_string());
                }
                self.record();
                return true;
            }
            match p {
                Parameter::Frequency => params.duration = value,
                Parameter::Damping => {
                    if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::SpinBrake {
                        params.brake_bias = value;
                    } else if self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::TextScan {
                        params.scan_angle = value;
                    } else {
                        params.damping = value;
                    }
                }
                Parameter::Duration => params.delay = value,
                Parameter::ReleaseVelocity => {
                    if self.recipe.kind.is_rotation() {
                        params.rotation_degrees = value;
                    } else {
                        params.amount = value;
                    }
                }
            }
            self.custom = true;
            if let Err(e) = self.recipe.set_parameters(params) {
                self.error = Some(e.to_string());
            }
            self.record();
            return true;
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
        if let Err(e) = self.recipe.set_reduced_motion(effective) {
            self.error = Some(e.to_string());
        }
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
        if self.is_recipe() {
            if let Some(at) = self.interrupt_at.filter(|at| *at <= end) {
                if let Err(e) = self.recipe.advance(at) {
                    self.error = Some(e.to_string());
                }
                self.interrupt_at = None;
                if self.recipe.kind.is_typing() {
                    self.recipe.typing_unit_count = typing_fixture(self.locale, false).len();
                }
                if let Err(e) = self.recipe.set_target(false) {
                    self.error = Some(e.to_string());
                }
            }
            if let Err(e) = self.recipe.advance(end) {
                self.error = Some(e.to_string());
            }
            self.now = end;
            self.record();
            return true;
        }
        if let Some(at) = self.interrupt_at.filter(|at| *at <= end) {
            self.sample_at(at);
            self.interrupt_at = None;
            self.retarget(0.28);
        }
        self.sample_at(end);
        true
    }
    pub fn needs_animation(&self) -> bool {
        self.error.is_none()
            && ((if self.is_recipe() {
                self.recipe.needs_frame()
            } else {
                self.channel.is_active()
            }) || self.interrupt_at.is_some())
    }
    pub fn deactivate(&mut self) {
        if self.pressed == Some(usize::MAX - 1) && self.is_recipe() {
            let _ = self.recipe.set_target(false);
        }
        if self.recipe_dragging {
            self.pause_recipe();
        }
        self.recipe_dragging = false;
        self.gesture_sample = None;
        self.dragging = None;
        self.hovered = None;
        self.pressed = None;
        self.focused_button = None;
        for button in self.button_models.values_mut() {
            button.clear_layout();
        }
        self.placement = None;
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
        let sample = if self.is_recipe() {
            tekes_ui_motion::Sample {
                value: self.recipe.values()[0],
                velocity: self.recipe.velocities()[0],
            }
        } else {
            self.channel.current()
        };
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
            value: if self.is_recipe()
                && self.recipe.kind == tekes_ui_motion::recipes::RecipeKind::LinearRotation
            {
                sample.value.rem_euclid(1.0)
            } else {
                sample.value
            },
            velocity: sample.velocity,
            target: if self.is_recipe() {
                f64::from(self.recipe.target())
            } else {
                self.channel.target()
            },
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
    fn symbol_catalog_is_separate_and_preserves_pose_on_reverse_and_pause() {
        use tekes_ui_motion::symbols::SymbolEffect;
        let mut state = MotionPreview::default();
        state.select_catalog(PresetCatalog::Symbols);
        assert_eq!(state.presets.len(), SymbolEffect::ALL.len() + 1);
        for index in 0..state.presets.len() {
            state.select(index);
            state.animate(0.7);
            let pose = state.recipe.values();
            state.pause_recipe();
            state.animate(1.0);
            assert_eq!(state.recipe.values(), pose);
            assert!(!state.needs_animation());
            state.resume_recipe();
            state.retarget(0.0);
            assert_eq!(state.recipe.values(), pose);
            state.set_parameter(Parameter::Frequency, 1.4);
            state.animate(2.0);
            if state.recipe.kind == tekes_ui_motion::recipes::RecipeKind::LinearRotation {
                assert!(state.recipe.velocities()[0] < 0.0);
                assert!(state.needs_animation());
            } else {
                assert_eq!(state.recipe.values()[0], 0.0);
                assert!(!state.needs_animation());
            }
            assert!(state.error.is_none());
            state.set_parameter(Parameter::Frequency, 0.3);
            state.set_parameter(Parameter::Duration, 0.6);
            state.demonstrate_interrupt();
            state.animate(0.13);
            assert!(state.recipe.target());
            state.animate(0.02);
            assert!(!state.recipe.target());
        }
        state.select_catalog(PresetCatalog::Recipes);
        assert_eq!(state.presets.len(), 22);
    }

    #[test]
    fn localized_typing_units_keep_graphemes_and_utf16_ranges_intact() {
        for locale in super::super::locale::Locale::ALL {
            for target in [false, true] {
                let units = typing_fixture(locale, target);
                assert!(units.iter().all(|unit| !unit.is_empty()));
                if locale == super::super::locale::Locale::En {
                    assert!(!super::super::locale::has_cjk(&units.concat()));
                }
                if target {
                    assert!(units.contains(&"👩‍💻"));
                    assert!(units.contains(&"e\u{301}"));
                    assert!(units.contains(&"🇨🇳"));
                }
                let mut controller = MotionPreview {
                    locale,
                    ..MotionPreview::default()
                };
                controller.select_catalog(PresetCatalog::Recipes);
                controller.select(21);
                controller.retarget(if target { 1.0 } else { 0.0 });
                assert_eq!(controller.recipe.typing_unit_count, units.len());
            }
        }
    }

    #[test]
    fn typing_demo_switches_source_units_and_pause_freezes_fade() {
        let mut s = MotionPreview::default();
        s.select_catalog(PresetCatalog::Recipes);
        s.select(21);
        s.demonstrate_interrupt();
        s.animate(0.4);
        assert!(!s.recipe.target());
        assert_eq!(
            s.recipe.typing_unit_count,
            typing_fixture(s.locale, false).len()
        );
        s.pause_recipe();
        let count = s.recipe.typing.visible();
        let alpha = s.recipe.typing.opacity(0);
        s.animate(2.0);
        assert_eq!(s.recipe.typing.visible(), count);
        assert_eq!(s.recipe.typing.opacity(0), alpha);
        s.resume_recipe();
        for _ in 0..500 {
            s.animate(0.02);
        }
        assert_eq!(
            s.recipe.typing.available(),
            typing_fixture(s.locale, false).len()
        );
        assert!(!s.needs_animation());
    }
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
        state.select_catalog(PresetCatalog::DesignSystem);
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
        state.select_catalog(PresetCatalog::DesignSystem);
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

    #[test]
    fn apple_and_design_catalogs_are_separate_and_both_settle() {
        let mut state = MotionPreview::default();
        assert_eq!(state.catalog, PresetCatalog::Apple);
        assert_eq!(state.presets.len(), 9);
        assert_eq!(state.presets[state.selected].id, "apple.spring");
        assert!(!state.needs_animation());
        for catalog in [PresetCatalog::Apple, PresetCatalog::DesignSystem] {
            state.select_catalog(catalog);
            for index in 0..state.presets.len() {
                state.select(index);
                assert!(
                    state.presets[index]
                        .id
                        .starts_with(if catalog == PresetCatalog::Apple {
                            "apple."
                        } else {
                            ""
                        })
                );
                for _ in 0..1500 {
                    state.animate(1.0 / 120.0);
                }
                assert!(!state.needs_animation(), "{}", state.presets[index].id);
                assert_eq!(state.channel.current().value, 0.88);
            }
        }
        assert_eq!(state.presets.len(), 10);
    }
}
