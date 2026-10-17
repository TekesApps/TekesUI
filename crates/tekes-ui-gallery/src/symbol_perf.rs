//! Symbol stress workloads use the actual Gallery controller, SVG adapter and
//! renderer. Parameter/target changes are automated, not physical mouse events.
use crate::gallery::{
    Gallery,
    motion_preview::{MotionPreview, Parameter, PresetCatalog, symbol_assets::CANVAS},
};
use serde_json::{Value, json};
use std::time::Instant;
use tekes_ui_icons::Icon;
use tekes_ui_motion::symbols::SymbolEffect;
use tekes_ui_render::{Distribution, Renderer};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Steady,
    Parameters,
    Retarget,
    Combined,
}
pub struct SymbolWorkload {
    states: Vec<MotionPreview>,
    mode: Mode,
    last_step: Option<u64>,
    elapsed: f64,
    stages: Vec<[f64; 3]>,
    edits: u64,
    targets: u64,
    errors: Vec<String>,
    title: String,
    freeze_raster: bool,
    stopped: bool,
    linear: bool,
    upload_baseline: Option<[u64; 4]>,
}
impl SymbolWorkload {
    pub fn from_env() -> Self {
        let count = std::env::var("TEKES_SYMBOL_SOURCES")
            .unwrap_or_else(|_| "8".into())
            .parse::<usize>()
            .ok()
            .filter(|v| [1, 8, 14, 24].contains(v))
            .unwrap_or_else(|| panic!("TEKES_SYMBOL_SOURCES must be 1,8,14,24"));
        let mode = match std::env::var("TEKES_SYMBOL_MODE")
            .as_deref()
            .unwrap_or("steady")
        {
            "steady" => Mode::Steady,
            "parameters" => Mode::Parameters,
            "retarget" => Mode::Retarget,
            "combined" => Mode::Combined,
            _ => panic!("invalid TEKES_SYMBOL_MODE"),
        };
        let mut workload = Self::new(count, mode);
        workload.freeze_raster = std::env::var("TEKES_SYMBOL_FREEZE_RASTER").as_deref() == Ok("1");
        if workload.freeze_raster {
            workload.title.push_str(" · STATIC TEXTURE DIAGNOSTIC");
        }
        workload.linear = std::env::var("TEKES_SYMBOL_LINEAR").as_deref() == Ok("1");
        if workload.linear {
            for state in &mut workload.states {
                state.select(SymbolEffect::ALL.len());
            }
            workload.title.push_str(" · LINEAR ROTATION");
        }
        workload
    }
    fn new(count: usize, mode: Mode) -> Self {
        let mixed = [
            (Icon::Settings, SymbolEffect::Breathe),
            (Icon::NewGoal, SymbolEffect::Bounce),
            (Icon::Think, SymbolEffect::Pulse),
            (Icon::Provider, SymbolEffect::Wiggle),
            (Icon::Appearance, SymbolEffect::Rotate),
            (Icon::Router, SymbolEffect::DrawOn),
            (Icon::NewGoal, SymbolEffect::MagicReplace),
            (Icon::Memory, SymbolEffect::VariableColor),
        ];
        let states = (0..count)
            .map(|i| {
                let (icon, effect) = if count == 14 {
                    (Icon::ALL[i % 8], SymbolEffect::ALL[i])
                } else {
                    mixed[i % 8]
                };
                let mut state = MotionPreview::default();
                state.select_catalog(PresetCatalog::Symbols);
                state.select(SymbolEffect::ALL.iter().position(|e| *e == effect).unwrap());
                state.select_symbol_icon(Icon::ALL.iter().position(|v| *v == icon).unwrap());
                state
            })
            .collect();
        Self {
            states,
            mode,
            last_step: None,
            elapsed: 0.0,
            stages: Vec::new(),
            edits: 0,
            targets: 0,
            errors: Vec::new(),
            freeze_raster: false,
            stopped: false,
            linear: false,
            upload_baseline: None,
            title: format!(
                "SYMBOL PERFORMANCE · {} simultaneous views · {mode:?}",
                count * 3
            ),
        }
    }
    pub fn tick(&mut self, elapsed: f64, dt: f64) {
        let start = Instant::now();
        if elapsed >= crate::perf::WARMUP_S && self.upload_baseline.is_none() {
            self.upload_baseline = Some(self.upload_totals());
        }
        self.elapsed = elapsed;
        let period = if self.mode == Mode::Parameters {
            1.2
        } else {
            0.18
        };
        let step = (elapsed / period).floor() as u64;
        let retarget = matches!(
            self.mode,
            Mode::Parameters | Mode::Retarget | Mode::Combined
        ) && self.last_step != Some(step);
        for (i, state) in self.states.iter_mut().enumerate() {
            state.animate(dt as f32);
            if matches!(self.mode, Mode::Parameters | Mode::Combined) {
                let wave = 0.5 + 0.5 * (elapsed * 3.1 + i as f64 * 0.17).sin();
                for (p, value) in [
                    (Parameter::Frequency, 0.7 + 1.7 * wave),
                    (Parameter::Damping, 0.25 + 0.65 * wave),
                    (Parameter::Duration, 0.05 + 0.3 * wave),
                ] {
                    if state.set_parameter(p, value) {
                        self.edits += 1;
                    }
                }
            }
            if retarget {
                state.retarget(if step.is_multiple_of(2) { 0.85 } else { 0.15 });
                self.targets += 1;
            } else if !matches!(self.mode, Mode::Retarget | Mode::Combined)
                && !state.needs_animation()
            {
                state.replay();
            }
        }
        self.last_step = Some(step);
        self.stages
            .push([elapsed, start.elapsed().as_secs_f64() * 1000.0, 0.0]);
    }
    pub fn build(
        &mut self,
        gallery: &mut Gallery,
        renderer: &mut Renderer,
        scene: &mut Scene,
        theme: Theme,
        scale: f32,
    ) {
        let start = Instant::now();
        let ink = proposal::render_color("primary-text", theme).unwrap();
        gallery.text_mut().proposal_label_in(
            scene,
            theme,
            Rect::new(24.0, 16.0, 1350.0, 26.0),
            &self.title,
            proposal::TextRole::UiBody,
            ink,
        );
        let columns = if self.states.len() > 8 { 12 } else { 6 };
        for (i, state) in self.states.iter_mut().enumerate() {
            if !self.freeze_raster || !state.symbol_assets.is_prepared() {
                state.prepare_symbols(renderer, theme, scale);
            }
            if let Some(e) = &state.error
                && self.errors.len() < 8
                && !self.errors.contains(e)
            {
                self.errors.push(e.clone());
            }
            for scope in 0..3 {
                let slot = i * 3 + scope;
                state.symbol_assets.append_scope(
                    scene,
                    scope,
                    Rect::new(
                        24.0 + (slot % columns) as f32 * CANVAS,
                        54.0 + (slot / columns) as f32 * CANVAS,
                        CANVAS,
                        CANVAS,
                    ),
                );
            }
        }
        if let Some(row) = self.stages.last_mut() {
            row[2] = start.elapsed().as_secs_f64() * 1000.0;
        }
    }
    pub fn stop(&mut self) {
        for state in &mut self.states {
            state.pause_recipe();
            state.deactivate();
        }
        self.stopped = self.states.iter().all(|s| !s.needs_animation());
    }
    fn upload_totals(&self) -> [u64; 4] {
        self.states.iter().fold([0; 4], |mut sums, state| {
            let s = state.symbol_assets.stats;
            for (i, v) in [s.rasterizations, s.texture_creations, s.uploads, s.bytes]
                .into_iter()
                .enumerate()
            {
                sums[i] += v;
            }
            sums
        })
    }
    pub fn report(&self) -> Value {
        let rows: Vec<_> = self
            .stages
            .iter()
            .filter(|r| r[0] >= crate::perf::WARMUP_S)
            .copied()
            .collect();
        let dist = |i| {
            let d = Distribution::from_samples(&rows.iter().map(|r| r[i]).collect::<Vec<_>>());
            json!({"count":d.count,"mean":d.mean,"p95":d.p95,"p99":d.p99,"max":d.max})
        };
        let totals = self.upload_totals();
        let base = self.upload_baseline.unwrap_or([0; 4]);
        let upload: Vec<u64> = totals.iter().zip(base).map(|(a, b)| a - b).collect();
        json!({"sources":self.states.len(),"animated_views":self.states.len()*3,"scope":"Whole + ByLayer + Individually per source",
            "mode":format!("{:?}",self.mode),"linear_rotation":self.linear,"measured_asset_work":{"rasterizations":upload[0],"texture_creations":upload[1],"uploads":upload[2],"upload_bytes":upload[3]},"static_texture_diagnostic":self.freeze_raster,"all_models_stopped":self.stopped,"input":"same MotionPreview parameter/target APIs; deterministic automated changes, not physical input",
            "parameter_edits_including_warmup":self.edits,"target_changes_including_warmup":self.targets,"target_period_s":if self.mode==Mode::Parameters {1.2} else {0.18},
            "parameter_edits":"up to three enabled sliders per source per display callback",
            "errors":self.errors,"model_and_control_ms":dist(1),"raster_and_upload_ms":dist(2),
            "stage_columns":["elapsed_including_warmup_s","model_and_control_ms","raster_and_upload_ms"],"stages":rows})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workload_changes_real_controllers_without_stale_or_invalid_state() {
        for mode in [
            Mode::Steady,
            Mode::Parameters,
            Mode::Retarget,
            Mode::Combined,
        ] {
            let mut workload = SymbolWorkload::new(8, mode);
            for i in 0..360 {
                workload.tick(i as f64 / 120.0, 1.0 / 120.0);
            }
            assert!(workload.states.iter().all(|s| s.error.is_none()));
            assert!(
                workload
                    .states
                    .iter()
                    .all(|s| s.recipe.values().iter().all(|v| v.is_finite()))
            );
            assert_eq!(workload.report()["animated_views"], 24);
            workload.stop();
            assert!(workload.stopped);
            if matches!(mode, Mode::Parameters | Mode::Combined) {
                assert!(workload.edits > 100);
            }
            if matches!(mode, Mode::Retarget | Mode::Combined) {
                assert!(workload.targets > 80);
            }
        }
    }
}
