//! Motion page: same JSON colors/type/spacing, Scene and TextEngine as Gallery.
//! No host, window, renderer, timers or extra UI toolkit is created here.
use super::PageCtx;
use crate::gallery::motion_preview::{MotionPlacement, MotionPreview, Parameter, PresetCatalog};
use tekes_ui_controls::{ButtonSize, ButtonTextAlign, paint::ButtonVariant};
use tekes_ui_motion::recipes::{Phase, RecipeKind};
use tekes_ui_platform_macos::{Event, MouseButton, MouseEventKind};
use tekes_ui_scene::{Point, Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{CornerRadius, Rgba, Stroke, Theme, proposal};

#[cfg(test)]
pub const HEIGHT: f32 = 942.0;
const TARGETS: [f64; 3] = [0.15, 0.5, 0.85];

pub(crate) fn focus_edge(state: &mut MotionPreview, reverse: bool) -> Option<Rect> {
    let id = if reverse {
        state.focus_order.last().copied()
    } else {
        state.focus_order.first().copied()
    }?;
    state.focused_button = Some(id);
    for (button_id, button) in &mut state.button_models {
        button.state.focused = *button_id == id;
        button.state.focus_visible = *button_id == id;
    }
    state
        .button_models
        .get(&id)?
        .layout()
        .map(|layout| layout.frame)
}

pub(crate) fn focused_button_frame(state: &MotionPreview) -> Option<Rect> {
    state
        .focused_button
        .and_then(|id| state.button_models.get(&id))
        .and_then(|button| button.layout())
        .map(|layout| layout.frame)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Catalog(PresetCatalog),
    Preset(usize),
    SymbolIcon(usize),
    PressDemo,
    Target(usize),
    Replay,
    Interrupt,
    ReduceMotion,
    Resume,
    ReadingUp,
    ReadingDown,
    TickReplay,
    TickDigits,
    TickStrength,
    TickDuration,
}

fn action_label(action: Action, state: &MotionPreview) -> &str {
    match action {
        Action::Catalog(PresetCatalog::Apple) => "Apple defaults",
        Action::Catalog(PresetCatalog::DesignSystem) => "Design System",
        Action::Catalog(PresetCatalog::Recipes) => "Recipes / 配方",
        Action::Catalog(PresetCatalog::Symbols) => "Symbols / 图标",
        Action::Catalog(PresetCatalog::TickingNumber) => "Ticking Number",
        Action::SymbolIcon(i) => tekes_ui_icons::Icon::ALL[i].label(),
        Action::Preset(i) => state.presets[i].label.as_str(),
        Action::Target(0) if state.is_recipe() => "目标 A / 反向",
        Action::Target(1) if state.is_recipe() => "暂停",
        Action::Target(_) if state.is_recipe() => "目标 B / 展开",
        Action::Target(0) => "A · 0.15",
        Action::Target(1) => "B · 0.50",
        Action::Target(_) => "C · 0.85",
        Action::TickReplay => "计数 / 反向",
        Action::TickDigits => "位数 2 / 3 / 6",
        Action::TickStrength => "强度 0 / 0.5 / 1",
        Action::TickDuration => "时长 1 / 2 / 4 秒",
        Action::Replay => "重播",
        Action::Resume => "继续当前",
        Action::Interrupt => "演示中途改目标",
        Action::ReduceMotion if state.reduced_motion => "减少动态效果 · 开",
        Action::ReduceMotion => "减少动态效果 · 关",
        Action::ReadingUp => "向上阅读",
        Action::ReadingDown => "向下阅读",
        Action::PressDemo => "按下并保持",
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub content: Rect,
    pub stage: Rect,
    pub graph: Rect,
    pub inspector: Rect,
    text_scale: f32,
    footer_top: f32,
    ticking_top: f32,
    button_frames: [Option<(Action, Rect)>; 64],
    button_count: usize,
    reading_inside_stage: bool,
}
impl Layout {
    pub fn new(content: Rect) -> Self {
        let mut text = crate::gallery::text::TextEngine::new();
        Self::for_state(content, &MotionPreview::default(), &mut text)
    }
    pub fn for_state(
        content: Rect,
        state: &MotionPreview,
        text: &mut super::super::text::TextEngine,
    ) -> Self {
        let text_scale = text.type_scale().clamp(1.0, 1.5);
        let scaled = |n: f32| n * text_scale;
        let width = content.size.width.max(1.0);
        let content = Rect::new(content.min_x(), content.min_y(), width, 0.0);
        let gap = scaled(24.0);
        let side = scaled(280.0).min((width - gap).max(scaled(220.0)));
        let split = width >= scaled(1080.0);
        let left = if split {
            (width - side - gap).max(1.0)
        } else {
            width
        };
        let button_size = ButtonSize::Xs;
        let role = proposal::TextRole::UiBody;
        let measure = |text: &mut super::super::text::TextEngine,
                       label: &str,
                       width: f32,
                       min_height: f32|
         -> f32 {
            text.measure_button(label, width, button_size, role)
                .height
                .max(min_height)
        };

        let mut button_frames = Vec::with_capacity(32);
        let mut y_cursor = 0.0;
        let catalog_gap = scaled(8.0);
        let catalogs = [
            PresetCatalog::Apple,
            PresetCatalog::DesignSystem,
            PresetCatalog::Recipes,
            PresetCatalog::Symbols,
            PresetCatalog::TickingNumber,
        ];
        let catalog_columns = if width >= scaled(720.0) { 5 } else { 2 };
        let catalog_width = if catalog_columns == 5 {
            scaled(148.0).min((width - 4.0 * catalog_gap) / 5.0)
        } else {
            (width - catalog_gap) / 2.0
        };
        let catalog_height = Self::flow_buttons(
            &mut button_frames,
            &mut y_cursor,
            text,
            state,
            catalogs.into_iter().map(Action::Catalog).collect(),
            catalog_width.max(1.0),
            catalog_gap,
            content,
            scaled(26.0),
        );
        if state.catalog == PresetCatalog::TickingNumber {
            let stage_top = catalog_height + scaled(24.0);
            let ticking_top = stage_top + scaled(20.0);
            let controls_top = stage_top + scaled(320.0);
            let controls_height = Self::flow_buttons_with_widths(
                &mut button_frames,
                &mut y_cursor,
                text,
                state,
                vec![
                    Action::TickReplay,
                    Action::TickDigits,
                    Action::TickStrength,
                    Action::TickDuration,
                    Action::ReduceMotion,
                ],
                vec![scaled(160.0); 5],
                scaled(8.0),
                Rect::new(
                    content.min_x() + scaled(16.0),
                    content.min_y(),
                    (width - scaled(32.0)).max(1.0),
                    0.0,
                ),
                controls_top,
                scaled(34.0),
            );
            let height = controls_top + controls_height + scaled(24.0);
            let content = Rect::new(content.min_x(), content.min_y(), width, height);
            let button_count = button_frames.len();
            let mut stored = [None; 64];
            for (slot, value) in stored.iter_mut().zip(button_frames) {
                *slot = Some(value);
            }
            let empty = Rect::new(0.0, 0.0, 0.0, 0.0);
            return Self {
                content,
                stage: Rect::new(
                    content.min_x(),
                    content.min_y() + stage_top,
                    width,
                    height - stage_top,
                ),
                graph: empty,
                inspector: empty,
                text_scale,
                footer_top: 0.0,
                ticking_top,
                button_frames: stored,
                button_count,
                reading_inside_stage: false,
            };
        }
        y_cursor = catalog_height + scaled(6.0);
        let preset_columns = if width >= scaled(900.0) {
            5
        } else if width >= scaled(540.0) {
            3
        } else {
            2
        };
        let preset_gap = scaled(8.0);
        let preset_width =
            ((width - (preset_columns as f32 - 1.0) * preset_gap) / preset_columns as f32).max(1.0);
        let preset_actions: Vec<_> = (0..state.presets.len()).map(Action::Preset).collect();
        let preset_rows = preset_actions.len().div_ceil(preset_columns).max(1);
        let mut preset_row_heights = vec![scaled(34.0); preset_rows];
        for (i, action) in preset_actions.iter().enumerate() {
            let row = i / preset_columns;
            preset_row_heights[row] = preset_row_heights[row].max(measure(
                text,
                action_label(*action, state),
                preset_width,
                scaled(34.0),
            ));
        }
        for (i, action) in preset_actions.iter().enumerate() {
            let row = i / preset_columns;
            let col = i % preset_columns;
            button_frames.push((
                *action,
                Rect::new(
                    content.min_x() + col as f32 * (preset_width + preset_gap),
                    content.min_y()
                        + y_cursor
                        + preset_row_heights[..row].iter().sum::<f32>()
                        + row as f32 * preset_gap,
                    preset_width,
                    preset_row_heights[row],
                ),
            ));
        }
        let mut presets_bottom = y_cursor;
        for (row, row_height) in preset_row_heights.iter().copied().enumerate() {
            presets_bottom += row_height;
            if row + 1 < preset_rows {
                presets_bottom += preset_gap;
            }
        }
        y_cursor = presets_bottom + scaled(8.0);
        let symbol_top = y_cursor;
        let symbol_columns = if width >= scaled(720.0) { 4 } else { 2 };
        let symbol_rows = if state.catalog == PresetCatalog::Symbols {
            tekes_ui_icons::Icon::ALL.len().div_ceil(symbol_columns)
        } else {
            0
        };
        let symbol_gap = scaled(8.0);
        let symbol_width =
            ((width - (symbol_columns as f32 - 1.0) * symbol_gap) / symbol_columns as f32).max(1.0);
        let mut symbol_row_heights = vec![scaled(30.0); symbol_rows.max(1)];
        if symbol_rows > 0 {
            for i in 0..tekes_ui_icons::Icon::ALL.len() {
                let row = i / symbol_columns;
                symbol_row_heights[row] = symbol_row_heights[row].max(measure(
                    text,
                    action_label(Action::SymbolIcon(i), state),
                    symbol_width,
                    scaled(30.0),
                ));
            }
            for i in 0..tekes_ui_icons::Icon::ALL.len() {
                let row = i / symbol_columns;
                let col = i % symbol_columns;
                let prior: f32 =
                    symbol_row_heights[..row].iter().sum::<f32>() + row as f32 * symbol_gap;
                button_frames.push((
                    Action::SymbolIcon(i),
                    Rect::new(
                        content.min_x() + col as f32 * (symbol_width + symbol_gap),
                        content.min_y() + symbol_top + prior,
                        symbol_width,
                        symbol_row_heights[row],
                    ),
                ));
            }
        }
        let symbol_bottom = if symbol_rows > 0 {
            symbol_top
                + symbol_row_heights[..symbol_rows].iter().sum::<f32>()
                + (symbol_rows - 1) as f32 * symbol_gap
        } else {
            presets_bottom
        };
        let legacy_actions = [
            Action::Target(0),
            Action::Target(1),
            Action::Target(2),
            Action::Replay,
            Action::Interrupt,
            Action::Resume,
        ];
        let legacy_widths: Vec<_> = legacy_actions
            .iter()
            .map(|action| {
                if *action == Action::Interrupt {
                    scaled(140.0)
                } else {
                    scaled(84.0)
                }
            })
            .collect();
        let legacy_fit = legacy_actions
            .iter()
            .zip(&legacy_widths)
            .all(|(action, width)| {
                measure(text, action_label(*action, state), *width, scaled(26.0))
                    <= scaled(34.0) + 0.01
            })
            && legacy_widths.iter().sum::<f32>() + scaled(8.0) * 5.0 <= left + 0.01;
        let rows_fit = catalog_height <= scaled(26.0) + 0.01
            && preset_row_heights
                .iter()
                .all(|height| *height <= scaled(34.0) + 0.01)
            && symbol_row_heights
                .iter()
                .take(symbol_rows)
                .all(|height| *height <= scaled(30.0) + 0.01);
        let symbols_control_row_fits = state.catalog != PresetCatalog::Symbols
            || legacy_actions.iter().all(|action| {
                let width = if *action == Action::Interrupt {
                    140.0
                } else {
                    84.0
                };
                measure(text, action_label(*action, state), width, 0.0) <= 26.0 + 0.01
            });
        let preserve_wide_default = text_scale <= 1.001
            && width >= 1080.0
            && legacy_fit
            && rows_fit
            && symbols_control_row_fits;
        let controls_top = if preserve_wide_default {
            if state.catalog == PresetCatalog::Symbols {
                scaled(238.0)
            } else {
                scaled(752.0)
            }
        } else {
            symbol_bottom + scaled(10.0)
        };
        let mut control_actions = [
            Action::Target(0),
            Action::Target(1),
            Action::Target(2),
            Action::Replay,
            Action::Resume,
            Action::Interrupt,
        ];
        if preserve_wide_default {
            control_actions = [
                Action::Target(0),
                Action::Target(1),
                Action::Target(2),
                Action::Replay,
                Action::Interrupt,
                Action::Resume,
            ];
        }
        let control_widths: Vec<f32> = control_actions
            .iter()
            .map(|a| match (a, preserve_wide_default) {
                (Action::Interrupt, _) => scaled(140.0),
                (_, true) => scaled(84.0),
                _ => scaled(92.0),
            })
            .collect();
        let symbol_control_baseline =
            if preserve_wide_default && state.catalog == PresetCatalog::Symbols {
                scaled(26.0)
            } else {
                scaled(34.0)
            };
        let control_height = Self::flow_buttons_with_widths(
            &mut button_frames,
            &mut y_cursor,
            text,
            state,
            control_actions.to_vec(),
            control_widths,
            scaled(8.0),
            content,
            controls_top,
            symbol_control_baseline,
        );
        let control_height = if preserve_wide_default
            && state.catalog != PresetCatalog::Symbols
            && control_actions.len() == 6
        {
            button_frames.truncate(button_frames.len() - control_actions.len());
            let positions = [0.0, 92.0, 184.0, 284.0, 376.0, 524.0];
            for (action, x) in control_actions.into_iter().zip(positions) {
                let width = if action == Action::Interrupt {
                    140.0
                } else {
                    84.0
                };
                let height = measure(text, action_label(action, state), width, scaled(34.0));
                button_frames.push((
                    action,
                    Rect::new(
                        content.min_x() + x,
                        content.min_y() + controls_top,
                        width,
                        height,
                    ),
                ));
            }
            symbol_control_baseline
        } else {
            control_height
        };
        let stage_top = if preserve_wide_default && state.catalog == PresetCatalog::Symbols {
            (controls_top + control_height + scaled(6.0)).max(scaled(270.0))
        } else if preserve_wide_default {
            scaled(270.0)
        } else {
            (controls_top + control_height + scaled(18.0)).max(scaled(270.0))
        };
        let stage_height = scaled(240.0);
        let graph_top = stage_top + stage_height + scaled(36.0);
        let graph_height = scaled(180.0);
        let mut graph_top = graph_top;
        let mut reading_inside_stage = true;
        if state.is_recipe() && state.recipe.kind == RecipeKind::Press {
            let area = Rect::new(
                content.min_x() + 20.0,
                content.min_y() + stage_top + 38.0,
                (left - 40.0).max(1.0),
                154.0,
            );
            let geometry = state.recipe.values()[0] as f32;
            let amount = state.recipe.parameters.amount as f32;
            let width = (220.0 - geometry * amount * 0.25)
                .min(area.size.width)
                .max(1.0);
            let max_height = 64.0 - geometry * amount * 0.08;
            let height = measure(
                text,
                action_label(Action::PressDemo, state),
                width,
                max_height,
            );
            button_frames.push((
                Action::PressDemo,
                Rect::new(
                    area.center().x - width / 2.0,
                    area.center().y - height / 2.0,
                    width,
                    height,
                ),
            ));
        }
        if state.is_recipe() && state.recipe.kind == RecipeKind::Reading {
            let reading_width = scaled(112.0);
            let reading_height = measure(text, "向下阅读", reading_width, scaled(34.0));
            if left >= scaled(560.0) {
                let y = content.min_y() + stage_top + stage_height - reading_height - scaled(8.0);
                button_frames.push((
                    Action::ReadingUp,
                    Rect::new(
                        content.min_x() + left - 2.0 * reading_width - scaled(16.0),
                        y,
                        reading_width,
                        reading_height,
                    ),
                ));
                button_frames.push((
                    Action::ReadingDown,
                    Rect::new(
                        content.min_x() + left - reading_width - scaled(8.0),
                        y,
                        reading_width,
                        reading_height,
                    ),
                ));
            } else {
                reading_inside_stage = false;
                let row_y = graph_top;
                let button_width = ((left - scaled(8.0)) / 2.0).max(1.0);
                button_frames.push((
                    Action::ReadingUp,
                    Rect::new(
                        content.min_x(),
                        content.min_y() + row_y,
                        button_width,
                        reading_height,
                    ),
                ));
                button_frames.push((
                    Action::ReadingDown,
                    Rect::new(
                        content.min_x() + button_width + scaled(8.0),
                        content.min_y() + row_y,
                        button_width,
                        reading_height,
                    ),
                ));
                graph_top += reading_height + scaled(12.0);
            }
        }
        let inspector_height = if split {
            graph_top + graph_height - stage_top
        } else {
            scaled(420.0)
        };
        let inspector_top = if split {
            stage_top
        } else {
            graph_top + graph_height + scaled(24.0)
        };
        let reduce_width = ((if split { side } else { left }) - scaled(32.0)).max(1.0);
        let reduce_height = measure(
            text,
            action_label(Action::ReduceMotion, state),
            reduce_width,
            scaled(34.0),
        );
        let inspector_height =
            inspector_height.max((reduce_height + scaled(48.0)).max(scaled(420.0)));
        let reduce_frame = Rect::new(
            (if split {
                content.min_x() + left + gap
            } else {
                content.min_x()
            }) + scaled(16.0),
            content.min_y() + inspector_top + inspector_height - reduce_height - scaled(18.0),
            reduce_width,
            reduce_height,
        );
        button_frames.push((Action::ReduceMotion, reduce_frame));
        let footer_top = (inspector_top + inspector_height + scaled(30.0))
            .max(scaled(816.0))
            .max(controls_top + control_height + scaled(30.0));
        let footer_height = measured_footer_height(text, state, width, text_scale);
        let ticking_top = 0.0;
        let height = footer_top + footer_height + scaled(16.0);
        let content = Rect::new(content.min_x(), content.min_y(), width, height);
        let button_count = button_frames.len().min(64);
        let mut stored = [None; 64];
        for (i, value) in button_frames.into_iter().take(button_count).enumerate() {
            stored[i] = Some(value);
        }
        Self {
            content,
            stage: Rect::new(
                content.min_x(),
                content.min_y() + stage_top,
                left,
                stage_height,
            ),
            graph: Rect::new(
                content.min_x(),
                content.min_y() + graph_top,
                left,
                graph_height,
            ),
            inspector: Rect::new(
                if split {
                    content.min_x() + left + gap
                } else {
                    content.min_x()
                },
                content.min_y() + inspector_top,
                if split { side } else { left },
                inspector_height,
            ),
            text_scale,
            footer_top,
            ticking_top,
            button_frames: stored,
            button_count,
            reading_inside_stage,
        }
    }
    pub fn button(self, action: Action) -> Rect {
        self.button_frames[..self.button_count]
            .iter()
            .flatten()
            .find(|(candidate, _)| *candidate == action)
            .map_or(Rect::new(0.0, 0.0, 0.0, 0.0), |(_, rect)| *rect)
    }
    pub fn action_id(action: Action) -> u64 {
        match action {
            Action::Preset(i) => 0x1000 + i as u64,
            Action::SymbolIcon(i) => 0x2000 + i as u64,
            Action::Target(i) => 0x3000 + i as u64,
            Action::Catalog(c) => {
                0x4000
                    + match c {
                        PresetCatalog::Apple => 0,
                        PresetCatalog::DesignSystem => 1,
                        PresetCatalog::Recipes => 2,
                        PresetCatalog::Symbols => 3,
                        PresetCatalog::TickingNumber => 4,
                    }
            }
            Action::TickReplay => 0x5010,
            Action::TickDigits => 0x5011,
            Action::TickStrength => 0x5012,
            Action::TickDuration => 0x5013,
            Action::Replay => 0x5000,
            Action::Resume => 0x5001,
            Action::Interrupt => 0x5002,
            Action::ReduceMotion => 0x5003,
            Action::ReadingUp => 0x5004,
            Action::ReadingDown => 0x5005,
            Action::PressDemo => 0x5006,
        }
    }
    pub fn slider(self, p: Parameter) -> Rect {
        if self.inspector.is_empty() {
            return self.inspector;
        }
        let i = Parameter::ALL.iter().position(|a| *a == p).unwrap();
        Rect::new(
            self.inspector.min_x() + 20.0,
            self.inspector.min_y() + (72.0 + i as f32 * 82.0) * self.text_scale,
            self.inspector.size.width - 40.0,
            24.0 * self.text_scale,
        )
    }
    #[cfg(test)]
    pub fn actions(self, count: usize) -> Vec<(Action, Rect)> {
        self.button_frames[..self.button_count]
            .iter()
            .flatten()
            .filter(|(a, _)| !matches!(a, Action::Preset(i) if *i >= count))
            .copied()
            .collect()
    }
    pub(crate) fn state_actions(self, state: &MotionPreview) -> Vec<(Action, Rect)> {
        self.button_frames[..self.button_count]
            .iter()
            .flatten()
            .filter(|(a, _)| match a {
                Action::Preset(i) => *i < state.presets.len(),
                Action::SymbolIcon(_) => state.catalog == PresetCatalog::Symbols,
                Action::ReadingUp | Action::ReadingDown => {
                    state.is_recipe() && state.recipe.kind == RecipeKind::Reading
                }
                Action::PressDemo => state.is_recipe() && state.recipe.kind == RecipeKind::Press,
                _ => true,
            })
            .copied()
            .collect()
    }
    pub fn track(self) -> Rect {
        self.stage.inset(32.0)
    }

    #[allow(clippy::too_many_arguments)]
    fn flow_buttons(
        output: &mut Vec<(Action, Rect)>,
        y_cursor: &mut f32,
        text: &mut super::super::text::TextEngine,
        state: &MotionPreview,
        actions: Vec<Action>,
        item_width: f32,
        gap: f32,
        content: Rect,
        min_height: f32,
    ) -> f32 {
        let widths = vec![item_width; actions.len()];
        Self::flow_buttons_with_widths(
            output, y_cursor, text, state, actions, widths, gap, content, *y_cursor, min_height,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn flow_buttons_with_widths(
        output: &mut Vec<(Action, Rect)>,
        _y_cursor: &mut f32,
        text: &mut super::super::text::TextEngine,
        state: &MotionPreview,
        actions: Vec<Action>,
        widths: Vec<f32>,
        gap: f32,
        content: Rect,
        top: f32,
        min_height: f32,
    ) -> f32 {
        let mut rows: Vec<Vec<(Action, f32, f32)>> = Vec::new();
        let mut row: Vec<(Action, f32, f32)> = Vec::new();
        let mut row_width = 0.0;
        let mut row_height = 0.0_f32;
        let mut total_height = 0.0;
        for (action, requested) in actions.into_iter().zip(widths) {
            let item_width = requested.min(content.size.width.max(1.0));
            let measured = text
                .measure_button(
                    action_label(action, state),
                    item_width,
                    ButtonSize::Xs,
                    proposal::TextRole::UiBody,
                )
                .height
                .max(min_height);
            if !row.is_empty() && row_width + gap + item_width > content.size.width {
                total_height += row_height + gap;
                rows.push(std::mem::take(&mut row));
                row_width = 0.0;
                row_height = 0.0;
            }
            row_width += if row.is_empty() {
                item_width
            } else {
                gap + item_width
            };
            row_height = row_height.max(measured);
            row.push((action, item_width, measured));
        }
        if !row.is_empty() {
            total_height += row_height;
            rows.push(row);
        }
        let mut y = top;
        for entries in rows {
            let height = entries
                .iter()
                .map(|(_, _, h)| *h)
                .fold(min_height, f32::max);
            let mut x = content.min_x();
            for (action, width, _) in entries {
                output.push((action, Rect::new(x, content.min_y() + y, width, height)));
                x += width + gap;
            }
            y += height + gap;
        }
        total_height
    }
}

fn apply_action(state: &mut MotionPreview, action: Action) {
    match action {
        Action::Catalog(catalog) => {
            state.select_catalog(catalog);
        }
        Action::SymbolIcon(i) => state.select_symbol_icon(i),
        Action::Preset(i) => {
            state.select(i);
        }
        Action::Target(i) => state.retarget(TARGETS[i]),
        Action::TickReplay => {
            let (a, b) = if state.ticking.digits() == 2 {
                (37, 46)
            } else {
                (137, 546)
            };
            let _ = state
                .ticking
                .set_target(if state.ticking.target() == b { a } else { b });
        }
        Action::TickDigits => {
            let digits = match state.ticking.digits() {
                2 => 3,
                3 => 6,
                _ => 2,
            };
            let mut ticking =
                tekes_ui_controls::TickingNumber::new(if digits == 2 { 37 } else { 137 }, digits)
                    .unwrap();
            ticking.set_duration(state.ticking.duration());
            let _ = ticking.set_strength(state.ticking.strength());
            ticking.set_reduced_motion(state.reduced_motion);
            state.ticking = ticking;
        }
        Action::TickStrength => {
            let next = if state.ticking.strength() >= 1.0 {
                0.0
            } else {
                state.ticking.strength() + 0.5
            };
            let _ = state.ticking.set_strength(next);
        }
        Action::TickDuration => {
            let seconds = match state.ticking.duration().as_secs() {
                1 => 2,
                2 => 4,
                _ => 1,
            };
            state
                .ticking
                .set_duration(std::time::Duration::from_secs(seconds));
        }
        Action::Replay => state.replay(),
        Action::Resume => {
            if state.is_recipe() {
                state.resume_recipe();
            }
        }
        Action::Interrupt => state.demonstrate_interrupt(),
        Action::ReduceMotion => {
            state.set_reduced_motion(!state.manual_reduced_motion());
        }
        Action::ReadingUp => state.recipe.scroll_by(-36.0),
        Action::ReadingDown => state.recipe.scroll_by(36.0),
        Action::PressDemo => {}
    }
}

#[cfg(test)]
fn install_button_placements(state: &mut MotionPreview, layout: Layout, theme: Theme) {
    let actions = layout.state_actions(state);
    state.focus_order.clear();
    let mut scene = Scene::new();
    let mut text = super::super::text::TextEngine::new();
    for (action, frame) in &actions {
        let id = Layout::action_id(*action);
        state.focus_order.push(id);
        let label = action_label(*action, state).to_owned();
        let model = state.button_models.entry(id).or_default();
        model.state.selected = matches!(
            action,
            Action::Catalog(catalog) if *catalog == state.catalog
        ) || matches!(action, Action::Preset(i) if *i == state.selected)
            || matches!(action, Action::SymbolIcon(i) if tekes_ui_icons::Icon::ALL[*i] == state.symbol_icon)
            || matches!(action, Action::ReduceMotion if state.reduced_motion);
        text.render_button(
            &mut scene,
            model,
            &label,
            *frame,
            Some(layout.content),
            theme,
            ButtonVariant::Panel,
            ButtonSize::Xs,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
    }
    state.placement = Some(MotionPlacement {
        stage: layout.stage,
        sliders: std::array::from_fn(|i| layout.slider(Parameter::ALL[i])),
    });
}

#[cfg(test)]
pub fn extent(content: Rect) -> Size {
    let mut text = super::super::text::TextEngine::new();
    Layout::for_state(content, &MotionPreview::default(), &mut text)
        .content
        .size
}

pub fn extent_for_state(
    content: Rect,
    state: &MotionPreview,
    text: &mut super::super::text::TextEngine,
) -> Size {
    Layout::for_state(content, state, text).content.size
}

/// Pass the same live-content rectangle used by `build` and raw window event
/// positions together. The host applies viewport bounds routing first; a
/// captured slider must continue receiving drag/up outside those bounds.
#[cfg(test)]
pub fn handle_input(state: &mut MotionPreview, event: &Event, content: Rect) -> bool {
    let changed = handle_input_clipped(state, event, content, content);
    // Unit tests model the host's redraw between native events. Production
    // always supplies the placement retained by the just-completed build.
    state.placement = None;
    changed
}

/// `visible` is the renderer's clipped viewport in window coordinates. Keep
/// captured drags alive outside it, but never hover or activate clipped rows.
pub fn handle_input_clipped(
    state: &mut MotionPreview,
    event: &Event,
    content: Rect,
    visible: Rect,
) -> bool {
    #[cfg(not(test))]
    let _ = content;
    if state.placement.is_none() {
        #[cfg(test)]
        {
            let mut text = super::super::text::TextEngine::new();
            let layout = Layout::for_state(content, state, &mut text);
            install_button_placements(state, layout, Theme::LIGHT);
        }
        #[cfg(not(test))]
        return false;
    }
    let placement = state.placement.as_ref().expect("motion placement").clone();
    let actions: Vec<Action> = (0..state.presets.len())
        .map(Action::Preset)
        .chain((0..3).map(Action::Target))
        .chain([
            Action::TickReplay,
            Action::TickDigits,
            Action::TickStrength,
            Action::TickDuration,
            Action::Replay,
            Action::Resume,
            Action::Interrupt,
            Action::ReduceMotion,
            Action::Catalog(PresetCatalog::Apple),
            Action::Catalog(PresetCatalog::DesignSystem),
            Action::Catalog(PresetCatalog::Recipes),
            Action::Catalog(PresetCatalog::Symbols),
            Action::Catalog(PresetCatalog::TickingNumber),
        ])
        .chain(
            (state.is_recipe() && state.recipe.kind == RecipeKind::Press)
                .then_some([Action::PressDemo])
                .into_iter()
                .flatten(),
        )
        .chain(
            (state.catalog == PresetCatalog::Symbols)
                .then_some(0..tekes_ui_icons::Icon::ALL.len())
                .into_iter()
                .flatten()
                .map(Action::SymbolIcon),
        )
        .chain(
            (state.is_recipe() && state.recipe.kind == RecipeKind::Reading)
                .then_some([Action::ReadingUp, Action::ReadingDown])
                .into_iter()
                .flatten(),
        )
        .filter(|action| state.focus_order.contains(&Layout::action_id(*action)))
        .collect();
    let set_from_x = |state: &mut MotionPreview, p: Parameter, x: f32| {
        let r = placement.sliders[Parameter::ALL.iter().position(|a| *a == p).unwrap()];
        let unit = ((x - r.min_x()) / r.size.width).clamp(0.0, 1.0);
        let (lo, hi) = state.parameter_range(p);
        state.set_parameter(p, lo + f64::from(unit) * (hi - lo))
    };
    match event {
        Event::Focus(false) => {
            let dirty =
                state.dragging.is_some() || state.hovered.is_some() || state.pressed.is_some();
            state.deactivate();
            dirty
        }
        Event::Mouse(mouse) => {
            let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
            let inside_visible = visible.contains(point);
            let mut hit = None;
            for (index, action) in actions.iter().enumerate() {
                let id = Layout::action_id(*action);
                if let Some(button) = state.button_models.get_mut(&id) {
                    if inside_visible {
                        button.pointer_move_at(point);
                    } else {
                        button.pointer_move_at(Point::new(f32::NAN, f32::NAN));
                    }
                    if inside_visible
                        && button.layout().is_some_and(|layout| layout.hit_test(point))
                    {
                        hit = Some(index);
                    }
                }
            }
            let hover_changed = state.hovered != hit;
            state.hovered = hit;
            match mouse.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } => {
                    if !visible.contains(point) {
                        return false;
                    }
                    for (i, p) in Parameter::ALL.into_iter().enumerate() {
                        if placement.sliders[i].contains(point) && state.parameter_enabled(p) {
                            state.dragging = Some(p);
                            set_from_x(state, p, point.x);
                            return true;
                        }
                    }
                    if let Some(index) = hit {
                        let action = actions[index];
                        let id = Layout::action_id(action);
                        if state
                            .button_models
                            .get_mut(&id)
                            .is_some_and(|button| button.pointer_down_at(point))
                        {
                            state.focused_button = Some(id);
                            state.pressed = Some(index);
                            if action == Action::PressDemo {
                                state.retarget(0.85);
                            }
                            return true;
                        }
                    }
                    if state.is_recipe()
                        && state.recipe.kind == RecipeKind::DragSnap
                        && placement.stage.contains(point)
                    {
                        let track = placement.stage.inset(32.0);
                        state.drag_recipe(f64::from((point.x - track.min_x()) / track.size.width));
                        // Reuse the host's established captured-mouse route.
                        state.pressed = Some(usize::MAX);
                        return true;
                    }
                    if state.catalog != PresetCatalog::TickingNumber
                        && placement.stage.contains(point)
                    {
                        let track = placement.stage.inset(32.0);
                        state.retarget(f64::from((point.x - track.min_x()) / track.size.width));
                        return true;
                    }
                    false
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } => {
                    if state
                        .pressed
                        .and_then(|index| actions.get(index))
                        .is_some_and(|action| *action == Action::PressDemo)
                    {
                        let id = Layout::action_id(Action::PressDemo);
                        let inside_button = state
                            .button_models
                            .get(&id)
                            .and_then(|button| button.layout())
                            .is_some_and(|layout| layout.hit_test(point));
                        state.retarget(if inside_button { 0.85 } else { 0.15 });
                        true
                    } else if state.recipe_dragging {
                        let track = placement.stage.inset(32.0);
                        state.drag_recipe(f64::from((point.x - track.min_x()) / track.size.width));
                        true
                    } else {
                        state
                            .dragging
                            .is_some_and(|p| set_from_x(state, p, point.x))
                    }
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => {
                    if state.recipe_dragging {
                        state.release_recipe();
                        state.pressed = None;
                        return true;
                    }
                    if state.dragging.take().is_some() {
                        return true;
                    }
                    let pressed = state.pressed.take();
                    if pressed
                        .and_then(|index| actions.get(index))
                        .is_some_and(|action| *action == Action::PressDemo)
                    {
                        state.retarget(0.15);
                    }
                    let mut activated = None;
                    for action in &actions {
                        let id = Layout::action_id(*action);
                        let release_point = if inside_visible {
                            point
                        } else {
                            Point::new(f32::NAN, f32::NAN)
                        };
                        if state
                            .button_models
                            .get_mut(&id)
                            .is_some_and(|button| button.pointer_up_at(release_point))
                        {
                            activated = Some(*action);
                        }
                    }
                    if let Some(action) = activated {
                        apply_action(state, action);
                    }
                    pressed.is_some()
                }
                MouseEventKind::Moved | MouseEventKind::Entered => hover_changed,
                MouseEventKind::Exited => {
                    for button in state.button_models.values_mut() {
                        button.pointer_move_at(Point::new(f32::NAN, f32::NAN));
                    }
                    state.hovered.take().is_some()
                }
                _ => false,
            }
        }
        Event::Key(key) => {
            if key.kind == tekes_ui_platform_macos::KeyEventKind::Down
                && key.key_code == 48
                && !state.focus_order.is_empty()
            {
                let reverse = key.modifiers.shift;
                let current = state
                    .focused_button
                    .and_then(|id| state.focus_order.iter().position(|v| *v == id));
                let next = match (current, reverse) {
                    (Some(0), true) => return false,
                    (Some(i), true) => i - 1,
                    (Some(i), false) if i + 1 == state.focus_order.len() => return false,
                    (Some(i), false) => i + 1,
                    (None, true) => state.focus_order.len() - 1,
                    (None, false) => 0,
                };
                state.focused_button = Some(state.focus_order[next]);
                for (id, button) in &mut state.button_models {
                    button.state.focused = *id == state.focus_order[next];
                    button.state.focus_visible = *id == state.focus_order[next];
                }
                return true;
            }
            let Some(id) = state.focused_button else {
                return false;
            };
            let control_key = match key.key_code {
                36 => tekes_ui_controls::ControlKey::Enter,
                49 => tekes_ui_controls::ControlKey::Space,
                _ => return false,
            };
            let down = key.kind == tekes_ui_platform_macos::KeyEventKind::Down;
            let activate = state
                .button_models
                .get_mut(&id)
                .is_some_and(|button| button.key(control_key, down));
            if activate
                && let Some(action) = actions
                    .iter()
                    .copied()
                    .find(|action| Layout::action_id(*action) == id)
            {
                apply_action(state, action);
                return true;
            }
            activate || control_key == tekes_ui_controls::ControlKey::Space
        }
        _ => false,
    }
}

fn color(theme: Theme, id: &str) -> Rgba {
    proposal::render_color(id, theme).expect("validated motion page color")
}

struct Painter<'a, 'b> {
    ctx: &'a mut PageCtx<'b>,
    scene: &'a mut Scene,
}
impl Painter<'_, '_> {
    fn quad(&mut self, r: Rect, fill: &str, radius: f32, border: bool) {
        let mut quad = Quad::new(r, color(self.ctx.theme, fill)).with_corner_radius(radius);
        if border {
            quad = quad.with_border(Stroke::Rule.value(), color(self.ctx.theme, "border"));
        }
        self.scene.push_quad(quad);
    }
    fn panel(&mut self, r: Rect, fill: &str, radius: f32) {
        let (width, border) =
            tekes_ui_controls::paint::decorative_border(self.ctx.theme, self.ctx.text.scale());
        self.scene.push_quad(
            Quad::new(r, color(self.ctx.theme, fill))
                .with_corner_radius(radius)
                .with_border(width, border),
        );
    }
    fn label(&mut self, r: Rect, text: &str, role: proposal::TextRole, ink: &str) {
        let paragraph = self.ctx.text.proposal_label(text, role, Some(r.size.width));
        let wrapped = Rect::new(
            r.min_x(),
            r.min_y(),
            r.size.width,
            r.size.height.max(paragraph.height as f32),
        );
        self.ctx.text.proposal_wrapped_in(
            self.scene,
            self.ctx.theme,
            wrapped,
            text,
            role,
            color(self.ctx.theme, ink),
        );
    }
    fn body(&mut self, r: Rect, text: &str) {
        self.label(r, text, proposal::TextRole::UiBody, "primary-text");
    }
    fn caption(&mut self, r: Rect, text: &str) {
        self.label(r, text, proposal::TextRole::UiCaption, "secondary-text");
    }
    // Per-digit cached paragraphs prevent 120Hz telemetry from generating an
    // unbounded set of label shapes or repeatedly invoking CoreText shaping.
    fn number(&mut self, r: Rect, value: f64) {
        self.digits(r, &format!("{value:.2}"), "primary-text");
    }
    fn digits(&mut self, r: Rect, text: &str, ink: &str) {
        let mut x = r.min_x();
        for c in text.chars() {
            let digit = c.to_string();
            let width = self
                .ctx
                .text
                .proposal_label(&digit, proposal::TextRole::UiBody, None)
                .width
                .ceil() as f32;
            self.label(
                Rect::new(x, r.min_y(), width, r.size.height),
                &digit,
                proposal::TextRole::UiBody,
                ink,
            );
            x += width;
        }
    }
    fn faded_quad(&mut self, r: Rect, fill: &str, radius: f32, alpha: f32) {
        self.scene.push_quad(
            Quad::new(
                r,
                color(self.ctx.theme, fill).with_alpha(alpha.clamp(0.0, 1.0)),
            )
            .with_corner_radius(radius),
        );
    }
    fn faded_body(&mut self, r: Rect, text: &str, alpha: f32) {
        self.ctx.text.proposal_label_in(
            self.scene,
            self.ctx.theme,
            r,
            text,
            proposal::TextRole::UiBody,
            color(self.ctx.theme, "primary-text").with_alpha(alpha.clamp(0.0, 1.0)),
        );
    }
    fn polygon(&mut self, points: &[Point], ink: &str) {
        let min = points
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
            .floor();
        let max = points
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil();
        for row in 0..((max - min).max(0.0) as usize).min(320) {
            let y = min + row as f32 + 0.5;
            let mut hits = Vec::with_capacity(4);
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                    hits.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            hits.sort_by(f32::total_cmp);
            for pair in hits.chunks_exact(2) {
                self.quad(
                    Rect::new(pair[0], y - 0.5, (pair[1] - pair[0]).max(0.0), 1.0),
                    ink,
                    0.0,
                    false,
                );
            }
        }
    }
    fn line(&mut self, a: Point, b: Point, ink: &str) {
        let count = ((b.x - a.x).abs().max((b.y - a.y).abs()) / 1.5)
            .ceil()
            .max(1.0) as usize;
        for i in 0..=count {
            let t = i as f32 / count as f32;
            self.quad(
                Rect::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, 1.5, 1.5),
                ink,
                0.75,
                false,
            );
        }
    }
}

#[derive(Clone, Copy)]
enum FooterItem<'a> {
    Caption(&'a str),
    Error(&'a str),
    Event(&'a str, &'a str),
}

fn footer_items(state: &MotionPreview) -> Vec<FooterItem<'_>> {
    let source = if state.catalog == PresetCatalog::Symbols {
        "现有 8 个已确认图标 · 原 SVG 几何 · 稳定分层 / 锚点 / 有序路径"
    } else if state.is_recipe() {
        "TekesUI 实验配方 · 独立属性与项目轨道 · 参数调整替换待执行阶段"
    } else if state.custom {
        "当前为实验参数；重新选择预设可恢复原值"
    } else if state.catalog == PresetCatalog::Apple {
        "Apple 官方公开默认值 · Spring duration 为感知时长，不是强制结束时间"
    } else {
        "预设数值来自现有 JSON Design System"
    };
    let guidance = if state.is_recipe() {
        "收起会反向并取消旧阶段；暂停保持当前画面；展开继续，重播从初态开始。"
    } else if state.catalog == PresetCatalog::Apple {
        "参考 SwiftUI Animation / UnitCurve 官方文档；不代表每个系统控件都使用同一组参数。"
    } else {
        "模型参考 Apple 官方机制；参数来自项目设计系统，非 Apple 系统控件常量。"
    };
    let mut items = vec![
        FooterItem::Caption(source),
        FooterItem::Caption(state.presets[state.selected].source.as_str()),
        FooterItem::Caption(guidance),
    ];
    if state.catalog == PresetCatalog::Symbols {
        items.push(FooterItem::Caption(state.symbol_icon.name()));
    }
    if let Some(error) = &state.error {
        items.push(FooterItem::Error(error));
    } else if let Some(event) = state.events.front()
        && let Some((number, label)) = event.split_once("  ")
    {
        items.push(FooterItem::Event(number, label));
    }
    items
}

fn measured_footer_height(
    text: &mut super::super::text::TextEngine,
    state: &MotionPreview,
    width: f32,
    scale: f32,
) -> f32 {
    let gap = 8.0 * scale;
    let items = footer_items(state);
    let height = items
        .iter()
        .map(|item| {
            let (label, role, inset) = match item {
                FooterItem::Caption(label) => (*label, proposal::TextRole::UiCaption, 0.0),
                FooterItem::Error(label) => (*label, proposal::TextRole::UiBody, 0.0),
                FooterItem::Event(_, label) => {
                    (*label, proposal::TextRole::UiCaption, 80.0 * scale)
                }
            };
            text.proposal_label(label, role, Some((width - inset).max(1.0)))
                .height
                .max(f64::from(24.0 * scale)) as f32
        })
        .sum::<f32>();
    height + gap * items.len().saturating_sub(1) as f32
}

pub fn build(
    ctx: &mut PageCtx<'_>,
    content: Rect,
    visible_clip: Rect,
    scene: &mut Scene,
    state: &mut MotionPreview,
) {
    let layout = Layout::for_state(content, state, ctx.text);
    let content = layout.content;
    let x = content.min_x();
    let y = content.min_y();
    let mut p = Painter { ctx, scene };
    let radius = CornerRadius::Panel.value();
    // Inspector background precedes its interactive Reduce Motion button.
    if state.catalog == PresetCatalog::TickingNumber {
        p.panel(layout.stage, "surface-panel", radius);
    } else {
        p.panel(layout.inspector, "surface-panel", radius);
    }
    if state.catalog != PresetCatalog::TickingNumber {
        p.caption(
            if content.size.width >= 760.0 {
                Rect::new(x + 632.0, y, content.size.width - 632.0, 24.0)
            } else {
                Rect::new(
                    x,
                    y + layout.stage.origin.y - y - 24.0,
                    content.size.width,
                    24.0,
                )
            },
            "MOTION LAB",
        );
    }
    let actions = layout.state_actions(state);
    state.focus_order.clear();
    for (action, rect) in &actions {
        let id = Layout::action_id(*action);
        state.focus_order.push(id);
        if *action == Action::PressDemo {
            continue;
        }
        let label = action_label(*action, state).to_owned();
        let selected = match action {
            Action::Catalog(catalog) => *catalog == state.catalog,
            Action::Preset(i) => *i == state.selected,
            Action::SymbolIcon(i) => tekes_ui_icons::Icon::ALL[*i] == state.symbol_icon,
            Action::ReduceMotion => state.reduced_motion,
            _ => false,
        };
        let model = state.button_models.entry(id).or_default();
        model.state.selected = selected;
        if state.focused_button == Some(id) {
            model.state.focused = true;
        } else {
            model.state.focused = false;
            model.state.focus_visible = false;
        }
        p.ctx.text.render_button(
            p.scene,
            model,
            &label,
            *rect,
            Some(visible_clip),
            p.ctx.theme,
            ButtonVariant::Panel,
            ButtonSize::Xs,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
    }
    state.placement = Some(MotionPlacement {
        stage: layout.stage,
        sliders: std::array::from_fn(|i| layout.slider(Parameter::ALL[i])),
    });

    if state.catalog == PresetCatalog::TickingNumber {
        let tick_y = y + layout.ticking_top;
        p.caption(
            Rect::new(
                x + 16.0 * layout.text_scale,
                tick_y,
                (content.size.width - 32.0 * layout.text_scale).max(1.0),
                28.0 * layout.text_scale,
            ),
            "Ticking Number / 等宽数字计数",
        );
        let preview = Rect::new(
            layout.stage.min_x() + 16.0 * layout.text_scale,
            layout.stage.min_y() + 64.0 * layout.text_scale,
            (layout.stage.size.width - 32.0 * layout.text_scale).max(1.0),
            192.0 * layout.text_scale,
        );
        let mut digits =
            crate::gallery::text::TickingPreviewText(p.ctx.text, 64.0 * layout.text_scale);
        let mut number_size = state
            .ticking
            .measure(&mut digits, proposal::TextRole::UiHeading);
        if number_size.width > preview.size.width {
            digits.1 *= preview.size.width / number_size.width;
            number_size = state
                .ticking
                .measure(&mut digits, proposal::TextRole::UiHeading);
        }
        let tick_origin = Point::new(
            preview.center().x - number_size.width / 2.0,
            preview.center().y - number_size.height / 2.0,
        );
        state.ticking.draw(
            p.scene,
            &mut digits,
            tick_origin,
            proposal::TextRole::UiHeading,
            color(p.ctx.theme, "primary-text"),
        );
        p.label(
            Rect::new(
                x + 16.0 * layout.text_scale,
                layout.stage.min_y() + 272.0 * layout.text_scale,
                (content.size.width - 32.0 * layout.text_scale).max(1.0),
                40.0 * layout.text_scale,
            ),
            &format!(
                "{} 位 · 强度 {:.1} · {:.1} 秒 · 目标 {}",
                state.ticking.digits(),
                state.ticking.strength(),
                state.ticking.duration().as_secs_f64(),
                state.ticking.target()
            ),
            proposal::TextRole::UiCaption,
            "secondary-text",
        );

        return;
    }

    let sample = if state.is_recipe() {
        tekes_ui_motion::Sample {
            value: state.recipe.values()[0],
            velocity: state.recipe.velocities()[0],
        }
    } else {
        state.channel.current()
    };
    if state.is_recipe() {
        build_recipe_stage(&mut p, layout, state, visible_clip);
    } else {
        p.panel(layout.stage, "surface-panel", radius);
        p.caption(
            Rect::new(
                layout.stage.min_x() + 24.0,
                layout.stage.min_y() + 16.0,
                layout.stage.size.width - 48.0,
                24.0,
            ),
            "LIVE PREVIEW     点击轨道或 A / B / C，运动中也可切换",
        );
        let track = layout.track();
        let middle = layout.stage.center().y;
        p.quad(
            Rect::new(track.min_x(), middle - 1.0, track.size.width, 2.0),
            "separator",
            1.0,
            false,
        );
        for mark in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mx = track.min_x() + track.size.width * mark;
            p.quad(Rect::new(mx, middle - 5.0, 1.0, 10.0), "border", 0.0, false);
        }

        let target_x = track.min_x() + state.channel.target() as f32 * track.size.width;
        p.quad(
            Rect::new(target_x - 19.0, middle - 19.0, 38.0, 38.0),
            "surface-panel",
            19.0,
            true,
        );
        p.caption(
            Rect::new(target_x - 20.0, middle + 30.0, 60.0, 22.0),
            "目标",
        );
        p.scene.push_clip(layout.stage);
        let ball_x = track.min_x() + sample.value as f32 * track.size.width;
        p.quad(
            Rect::new(ball_x - 15.0, middle - 15.0, 30.0, 30.0),
            "primary-text",
            15.0,
            false,
        );
        p.scene.pop_clip();
        let status = if state.reduced_motion {
            "Reduce Motion · 即时到达"
        } else if state.channel.is_active() {
            "运行中 · 可随时改目标"
        } else {
            "已停止 · 无动画帧请求"
        };
        p.caption(
            Rect::new(
                track.min_x(),
                layout.stage.max_y() - 40.0,
                track.size.width,
                24.0,
            ),
            status,
        );
    }
    let row = layout.stage.max_y() + 8.0;
    for (i, (label, value)) in [
        (
            if state.is_recipe() && state.recipe.kind.is_rotation() {
                "角度 / °"
            } else {
                "位置"
            },
            if state.is_recipe() && state.recipe.kind.is_rotation() {
                sample.value * state.recipe.parameters.rotation_degrees
            } else {
                sample.value
            },
        ),
        (
            if state.is_recipe() && state.recipe.kind.is_rotation() {
                "角速度 / °s"
            } else {
                "速度 / s"
            },
            if state.is_recipe() && state.recipe.kind.is_rotation() {
                sample.velocity * state.recipe.parameters.rotation_degrees
            } else {
                sample.velocity
            },
        ),
        (
            "目标",
            if state.is_recipe() {
                f64::from(state.recipe.target())
                    * if state.recipe.kind.is_rotation() {
                        state.recipe.parameters.rotation_degrees
                    } else {
                        1.0
                    }
            } else {
                state.channel.target()
            },
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mx = x + i as f32 * 180.0;
        p.caption(Rect::new(mx, row, 80.0, 24.0), label);
        p.number(Rect::new(mx + 82.0, row, 96.0, 24.0), value);
    }

    p.panel(layout.graph, "surface-panel", radius);
    p.caption(
        Rect::new(
            layout.graph.min_x() + 20.0,
            layout.graph.min_y() + 10.0,
            layout.graph.size.width - 40.0,
            24.0,
        ),
        if state.is_recipe() && state.recipe.kind.is_typing() {
            "REVEAL     每格一个完整显示单元 · 深色为已浮现"
        } else {
            "TRAJECTORY     实线：位置    虚线：目标    下方：速度"
        },
    );
    let graph = Rect::new(
        layout.graph.min_x() + 20.0,
        layout.graph.min_y() + 42.0,
        layout.graph.size.width - 40.0,
        100.0,
    );
    let bottom = graph.max_y();
    let end = state.now.as_secs_f64().max(1.0);
    let start = (end - 4.0).max(0.0);
    let duration = (end - start).max(1.0);
    let to_x = |t: f64| graph.min_x() + ((t - start) / duration) as f32 * graph.size.width;
    p.quad(
        Rect::new(graph.min_x(), bottom, graph.size.width, 1.0),
        "separator",
        0.0,
        false,
    );
    p.scene.push_clip(graph);
    if state.is_recipe() && state.recipe.kind.is_typing() {
        let columns = 12;
        let width = (graph.size.width - 11.0 * 4.0) / 12.0;
        for i in 0..state.recipe.typing_unit_count {
            let cell = Rect::new(
                graph.min_x() + (i % columns) as f32 * (width + 4.0),
                graph.min_y() + (i / columns) as f32 * 18.0,
                width,
                14.0,
            );
            p.quad(
                cell,
                if i < state.recipe.typing.available() {
                    "row-selection"
                } else {
                    "surface-panel"
                },
                3.0,
                true,
            );
            p.faded_quad(
                cell,
                "primary-text",
                3.0,
                state.recipe.typing.opacity(i) as f32,
            );
        }
    } else {
        let points: Vec<_> = state.trace.iter().filter(|s| s.time >= start).collect();
        let stride = points.len().div_ceil(160).max(1);
        let mut previous: Option<(Point, Point)> = None;
        for (index, point) in points.iter().step_by(stride).enumerate() {
            let px = to_x(point.time);
            let pos = Point::new(px, bottom - 20.0 - point.value as f32 * 65.0);
            let velocity = Point::new(
                px,
                bottom - 8.0 - point.velocity.clamp(-6.0, 6.0) as f32 * 1.0,
            );
            if let Some((old_pos, old_velocity)) = previous {
                p.line(old_pos, pos, "primary-text");
                p.line(old_velocity, velocity, "secondary-text");
            }
            if index % 2 == 0 {
                p.quad(
                    Rect::new(px, bottom - 20.0 - point.target as f32 * 65.0, 3.0, 1.5),
                    "tertiary-text",
                    0.0,
                    false,
                );
            }
            previous = Some((pos, velocity));
        }
    }
    p.scene.pop_clip();
    p.caption(
        Rect::new(graph.min_x(), layout.graph.max_y() - 30.0, 100.0, 22.0),
        "时间 / s",
    );
    p.number(
        Rect::new(
            graph.max_x() - 64.0,
            layout.graph.max_y() - 30.0,
            64.0,
            22.0,
        ),
        state.now.as_secs_f64(),
    );

    p.body(
        Rect::new(
            layout.inspector.min_x() + 20.0,
            layout.inspector.min_y() + 12.0,
            240.0,
            28.0,
        ),
        "PARAMETERS / 参数",
    );
    for parameter in Parameter::ALL {
        let r = layout.slider(parameter);
        let enabled = state.parameter_enabled(parameter);
        let ink = if enabled {
            "primary-text"
        } else {
            "disabled-text"
        };
        p.label(
            Rect::new(r.min_x(), r.min_y() - 28.0, 170.0, 24.0),
            state.parameter_label(parameter),
            proposal::TextRole::UiCaption,
            ink,
        );
        p.digits(
            Rect::new(r.max_x() - 56.0, r.min_y() - 28.0, 56.0, 24.0),
            &if enabled {
                format!("{:.2}", state.parameter_value(parameter))
            } else {
                "—".into()
            },
            ink,
        );
        p.quad(
            Rect::new(r.min_x(), r.center().y - 2.0, r.size.width, 4.0),
            "separator",
            2.0,
            false,
        );
        let (lo, hi) = state.parameter_range(parameter);
        let unit = ((state.parameter_value(parameter) - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
        if enabled {
            p.quad(
                Rect::new(r.min_x(), r.center().y - 2.0, r.size.width * unit, 4.0),
                ink,
                2.0,
                false,
            );
        }
        p.quad(
            Rect::new(
                r.min_x() + r.size.width * unit - 7.0,
                r.center().y - 7.0,
                14.0,
                14.0,
            ),
            if enabled {
                "primary-text"
            } else {
                "disabled-text"
            },
            7.0,
            false,
        );
    }

    let footer = y + layout.footer_top;
    let footer_gap = 8.0 * layout.text_scale;
    let mut footer_y = footer;
    for item in footer_items(state) {
        let (label, role, ink, inset) = match item {
            FooterItem::Caption(label) => {
                (label, proposal::TextRole::UiCaption, "secondary-text", 0.0)
            }
            FooterItem::Error(label) => (label, proposal::TextRole::UiBody, "primary-text", 0.0),
            FooterItem::Event(number, label) => {
                p.digits(
                    Rect::new(
                        x,
                        footer_y,
                        80.0 * layout.text_scale,
                        24.0 * layout.text_scale,
                    ),
                    number,
                    "secondary-text",
                );
                (
                    label,
                    proposal::TextRole::UiCaption,
                    "secondary-text",
                    80.0 * layout.text_scale,
                )
            }
        };
        let width = (content.size.width - inset).max(1.0);
        let measured = p
            .ctx
            .text
            .proposal_label(label, role, Some(width))
            .height
            .max(f64::from(24.0 * layout.text_scale)) as f32;
        p.label(
            Rect::new(x + inset, footer_y, width, measured),
            label,
            role,
            ink,
        );
        footer_y += measured + footer_gap;
    }
}

fn build_symbol_stage(
    p: &mut Painter<'_, '_>,
    area: Rect,
    state: &MotionPreview,
    effect: tekes_ui_motion::symbols::SymbolEffect,
) {
    use crate::gallery::motion_preview::symbol_assets::CANVAS;
    for (column, label) in ["Whole / 整体", "By Layer / 分层", "Individual / 逐层"]
        .into_iter()
        .enumerate()
    {
        let width = area.size.width / 3.0;
        let cx = area.min_x() + width * (column as f32 + 0.5);
        state.symbol_assets.append_scope(
            p.scene,
            column,
            Rect::new(cx - CANVAS * 0.5, area.min_y() + 4.0, CANVAS, CANVAS),
        );
        p.caption(
            Rect::new(cx - width * 0.5, area.min_y() + 114.0, width, 24.0),
            if state.recipe.kind == RecipeKind::LinearRotation && column == 2 {
                "Layer phase"
            } else {
                label
            },
        );
    }
    let resolved = state
        .symbol_assets
        .resolved_effect(state.symbol_icon, effect);
    if effect != resolved {
        p.caption(
            Rect::new(area.min_x(), area.min_y() + 138.0, area.size.width, 20.0),
            "无共享对应关系 · Magic Replace 降级为 Replace",
        );
    }
}

fn build_recipe_stage(
    p: &mut Painter<'_, '_>,
    layout: Layout,
    state: &mut MotionPreview,
    visible_clip: Rect,
) {
    let r = layout.stage;
    let recipe = &state.recipe;
    let [geometry, opacity, detail, preparation] = recipe.values().map(|v| v as f32);
    let g = geometry.clamp(-0.05, 1.1);
    let alpha = opacity.clamp(0.0, 1.0);
    let amount = recipe.parameters.amount as f32;
    p.panel(r, "surface-panel", CornerRadius::Panel.value());
    p.caption(
        Rect::new(r.min_x() + 20.0, r.min_y() + 8.0, r.size.width - 40.0, 24.0),
        recipe.kind.label(),
    );
    let area = Rect::new(
        r.min_x() + 20.0,
        r.min_y() + 38.0,
        r.size.width - 40.0,
        154.0,
    );
    p.scene.push_clip(area);
    match recipe.kind {
        RecipeKind::Symbol(effect) => build_symbol_stage(p, area, state, effect),
        RecipeKind::LinearRotation => build_symbol_stage(
            p,
            area,
            state,
            tekes_ui_motion::symbols::SymbolEffect::Rotate,
        ),
        RecipeKind::SharedElement => {
            let from = Rect::new(area.min_x() + 12.0, area.min_y() + 34.0, 136.0, 88.0);
            let to = Rect::new(
                area.min_x() + area.size.width * 0.29,
                area.min_y() + 4.0,
                area.size.width * 0.68,
                146.0,
            );
            // One stable element is rendered at interpolated geometry, rather
            // than crossfading duplicate cards with unrelated identities.
            let mix = |a: f32, b: f32| a + (b - a) * g;
            let card = Rect::new(
                mix(from.min_x(), to.min_x()),
                mix(from.min_y(), to.min_y()),
                mix(from.size.width, to.size.width),
                mix(from.size.height, to.size.height),
            );
            p.quad(from, "surface-panel", 12.0, true);
            p.quad(to, "surface-panel", 12.0, true);
            let radius = (12.0 + amount * 0.15 * g).max(0.0);
            p.quad(card, "row-selection", radius, true);
            let art = Rect::new(
                card.min_x() + 12.0,
                card.min_y() + 12.0,
                36.0 + 28.0 * g,
                36.0 + 28.0 * g,
            );
            p.quad(art, "primary-text", (8.0 + g * 8.0).max(0.0), false);
            let label_x = art.max_x() + 12.0;
            let label_width = (card.max_x() - label_x - 12.0).max(0.0);
            let label =
                p.ctx
                    .text
                    .proposal_label("Mountain stay", proposal::TextRole::UiBody, None);
            // Keep text shaping independent of the animated card width. The
            // label enters when its measured one-line slot fits, then stays
            // on that stable paragraph for the rest of the transition.
            if label.width <= f64::from(label_width.max(0.0)) {
                p.ctx.text.push(
                    p.scene,
                    &label,
                    Point::new(label_x.round(), (card.min_y() + 12.0).round()),
                    color(p.ctx.theme, "primary-text"),
                );
            }
            p.faded_body(
                Rect::new(
                    art.max_x() + 12.0,
                    card.min_y() + 42.0,
                    card.size.width - 90.0,
                    24.0,
                ),
                "同一张卡片 · 同一个身份",
                alpha,
            );
            p.faded_quad(
                Rect::new(
                    card.min_x() + 12.0,
                    card.max_y() - 26.0,
                    (card.size.width - 24.0).max(0.0),
                    6.0,
                ),
                "secondary-text",
                3.0,
                alpha * 0.35,
            );
        }
        RecipeKind::Panel => {
            p.caption(
                Rect::new(area.min_x() + 12.0, area.min_y() + 8.0, 180.0, 24.0),
                "工作区 / Document",
            );
            for i in 0..4 {
                p.quad(
                    Rect::new(
                        area.min_x() + 12.0,
                        area.min_y() + 46.0 + i as f32 * 24.0,
                        area.size.width * 0.5,
                        6.0,
                    ),
                    "separator",
                    3.0,
                    false,
                );
            }
            p.faded_quad(area, "primary-text", 8.0, 0.10 * geometry.clamp(0.0, 1.0));
            let scale = 0.88 + detail.clamp(-0.05, 1.1) * 0.12;
            let width = (area.size.width * 0.62).min(380.0) * scale;
            let height = (146.0 * geometry.max(0.0)).min(160.0);
            let panel = Rect::new(
                area.max_x() - width + amount * (1.0 - g),
                area.center().y - height * 0.5,
                width,
                height,
            );
            p.quad(panel, "surface-panel", 12.0, true);
            p.scene.push_clip(panel);
            p.faded_body(
                Rect::new(
                    panel.min_x() + 16.0,
                    panel.min_y() + 10.0,
                    width - 32.0,
                    26.0,
                ),
                "Inspector",
                alpha,
            );
            for (i, label) in ["布局与对齐", "外观与透明度", "交互与行为"]
                .iter()
                .enumerate()
            {
                p.faded_quad(
                    Rect::new(
                        panel.min_x() + 16.0,
                        panel.min_y() + 46.0 + i as f32 * 28.0,
                        width - 32.0,
                        22.0,
                    ),
                    "row-selection",
                    5.0,
                    alpha,
                );
                p.faded_body(
                    Rect::new(
                        panel.min_x() + 24.0,
                        panel.min_y() + 44.0 + i as f32 * 28.0,
                        width - 48.0,
                        24.0,
                    ),
                    label,
                    alpha,
                );
            }
            p.scene.pop_clip();
        }
        RecipeKind::Success => {
            let cx = area.center().x;
            let cy = area.min_y() + 55.0;
            let pulse = (preparation - detail).clamp(0.0, 1.0) * amount * 0.12;
            let radius = 28.0 + pulse;
            p.quad(
                Rect::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0),
                "row-selection",
                radius,
                false,
            );
            for i in 0..32 {
                let angle = i as f32 / 32.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                p.quad(
                    Rect::new(
                        cx + angle.cos() * 39.0 - 2.0,
                        cy + angle.sin() * 39.0 - 2.0,
                        4.0,
                        4.0,
                    ),
                    if i as f32 / 32.0 < geometry {
                        "primary-text"
                    } else {
                        "separator"
                    },
                    2.0,
                    false,
                );
            }
            let check = detail.clamp(0.0, 1.0);
            p.scene
                .push_clip(Rect::new(cx - 16.0, cy - 14.0, 34.0 * check, 30.0));
            p.line(
                Point::new(cx - 13.0, cy),
                Point::new(cx - 3.0, cy + 10.0),
                "primary-text",
            );
            p.line(
                Point::new(cx - 3.0, cy + 10.0),
                Point::new(cx + 17.0, cy - 12.0),
                "primary-text",
            );
            p.scene.pop_clip();
            for (i, (phase, label)) in [
                (Phase::Preparing, "1 · 准备"),
                (Phase::Working, "2 · 处理中"),
                (Phase::Success, "3 · 成功"),
            ]
            .into_iter()
            .enumerate()
            {
                let width = (area.size.width - 16.0) / 3.0;
                let step = Rect::new(
                    area.min_x() + i as f32 * (width + 8.0),
                    area.max_y() - 34.0,
                    width,
                    30.0,
                );
                p.quad(
                    step,
                    if recipe.phase == phase {
                        "row-selection"
                    } else {
                        "surface-panel"
                    },
                    6.0,
                    true,
                );
                p.body(
                    Rect::new(step.min_x() + 10.0, step.min_y(), width - 20.0, 30.0),
                    label,
                );
            }
        }
        RecipeKind::ListReorder => {
            let step = 21.0 + amount / 80.0;
            let left = area.min_x() + 16.0;
            let width = area.size.width * 0.64;
            let positions = recipe.item_values();
            for (i, label) in [
                "A · 项目计划",
                "B · 设计规范",
                "C · 待删除项目",
                "D · 交互实现",
                "E · 验证记录",
                "F · 新加入项目",
            ]
            .into_iter()
            .enumerate()
            {
                let visibility = if i == 2 {
                    1.0 - alpha
                } else if i == 5 {
                    alpha
                } else {
                    1.0
                };
                let row = Rect::new(
                    left,
                    area.min_y() + positions[i] as f32 * step,
                    width,
                    step - 1.0,
                );
                p.faded_quad(row, "row-selection", 4.0, visibility);
                p.faded_body(
                    Rect::new(
                        row.min_x() + 10.0,
                        row.min_y(),
                        width - 20.0,
                        row.size.height,
                    ),
                    label,
                    visibility,
                );
            }
            p.caption(
                Rect::new(
                    area.min_x() + area.size.width * 0.69,
                    area.min_y() + 12.0,
                    area.size.width * 0.3,
                    24.0,
                ),
                "稳定 ID · 连续排序",
            );
            p.caption(
                Rect::new(
                    area.min_x() + area.size.width * 0.69,
                    area.min_y() + 40.0,
                    area.size.width * 0.3,
                    24.0,
                ),
                "A：原始顺序",
            );
            p.caption(
                Rect::new(
                    area.min_x() + area.size.width * 0.69,
                    area.min_y() + 68.0,
                    area.size.width * 0.3,
                    24.0,
                ),
                "B：增删与重排",
            );
        }
        RecipeKind::SharedAxis => {
            for (incoming, title, subtitle) in [
                (false, "01 · 选择内容", "回到上一步"),
                (true, "02 · 确认设置", "继续到下一步"),
            ] {
                let progress = if incoming { g - 1.0 } else { g };
                let fade = if incoming { alpha } else { 1.0 - alpha };
                let card = Rect::new(
                    area.min_x() + area.size.width * 0.13 - progress * amount,
                    area.min_y() + 12.0,
                    area.size.width * 0.74,
                    128.0,
                );
                p.faded_quad(card, "row-selection", 12.0, fade);
                p.faded_body(
                    Rect::new(
                        card.min_x() + 24.0,
                        card.min_y() + 24.0,
                        card.size.width - 48.0,
                        28.0,
                    ),
                    title,
                    fade,
                );
                p.faded_body(
                    Rect::new(
                        card.min_x() + 24.0,
                        card.min_y() + 64.0,
                        card.size.width - 48.0,
                        24.0,
                    ),
                    subtitle,
                    fade,
                );
            }
        }
        RecipeKind::FadeThrough => {
            let split = 0.2 + recipe.parameters.delay as f32 * 0.6;
            for (incoming, title) in [(false, "项目总览"), (true, "数据分析")] {
                let fade = if incoming {
                    ((alpha - split) / (1.0 - split)).clamp(0.0, 1.0)
                } else {
                    (1.0 - alpha / split).clamp(0.0, 1.0)
                };
                let scale = if incoming {
                    1.0 - (1.0 - g) * amount / 1600.0
                } else {
                    1.0
                };
                let width = area.size.width * 0.76 * scale;
                let card = Rect::new(
                    area.center().x - width / 2.0,
                    area.min_y() + 10.0,
                    width,
                    130.0,
                );
                p.faded_quad(card, "row-selection", 12.0, fade);
                p.faded_body(
                    Rect::new(card.min_x() + 24.0, card.min_y() + 18.0, width - 48.0, 28.0),
                    title,
                    fade,
                );
                for i in 0..3 {
                    p.faded_quad(
                        Rect::new(
                            card.min_x() + 24.0,
                            card.min_y() + 62.0 + i as f32 * 18.0,
                            width - 48.0 - i as f32 * 24.0,
                            6.0,
                        ),
                        "secondary-text",
                        3.0,
                        fade * 0.35,
                    );
                }
            }
        }
        RecipeKind::DragSnap => {
            let track = layout.track();
            let y = area.center().y;
            p.quad(
                Rect::new(track.min_x(), y, track.size.width, 2.0),
                "separator",
                1.0,
                false,
            );
            for i in 0..5 {
                let x = track.min_x() + i as f32 * track.size.width / 4.0;
                p.quad(Rect::new(x - 3.0, y - 3.0, 6.0, 6.0), "border", 3.0, false);
            }
            let x = track.min_x() + geometry * track.size.width;
            let target = track.min_x() + recipe.snap_target() as f32 * track.size.width;
            p.quad(
                Rect::new(target - 20.0, y - 20.0, 40.0, 40.0),
                "surface-panel",
                20.0,
                true,
            );
            p.quad(
                Rect::new(x - 16.0, y - 16.0, 32.0, 32.0),
                "primary-text",
                16.0,
                false,
            );
            p.caption(
                Rect::new(
                    area.min_x() + 8.0,
                    area.max_y() - 28.0,
                    area.size.width - 16.0,
                    24.0,
                ),
                "拖动圆点并松手 · 惯性途中可再次抓住 · 五个吸附点",
            );
        }
        RecipeKind::Stagger => {
            let items = recipe.item_values();
            let width = (area.size.width - 24.0) / 3.0;
            for (i, label) in [
                "01 · 内容",
                "02 · 布局",
                "03 · 外观",
                "04 · 交互",
                "05 · 状态",
                "06 · 验证",
            ]
            .into_iter()
            .enumerate()
            {
                let value = items[i] as f32;
                let fade = value.clamp(0.0, 1.0);
                let card = Rect::new(
                    area.min_x() + (i % 3) as f32 * (width + 12.0),
                    area.min_y() + (i / 3) as f32 * 72.0 + (1.0 - value) * amount * 0.35,
                    width,
                    60.0,
                );
                p.faded_quad(card, "row-selection", 10.0, fade);
                p.faded_body(
                    Rect::new(card.min_x() + 12.0, card.min_y() + 16.0, width - 24.0, 28.0),
                    label,
                    fade,
                );
            }
        }
        RecipeKind::Popover => {
            let anchor = Rect::new(
                area.min_x() + area.size.width * 0.21,
                area.min_y() + 8.0,
                110.0,
                30.0,
            );
            p.quad(anchor, "row-selection", 6.0, true);
            p.body(
                Rect::new(anchor.min_x() + 12.0, anchor.min_y(), 90.0, 30.0),
                "操作菜单 ▾",
            );
            let scale = (0.8 + 0.2 * g).max(0.1);
            let panel = Rect::new(
                anchor.min_x(),
                anchor.max_y() + 6.0 + (1.0 - g) * amount * 0.2,
                220.0 * scale,
                102.0 * scale,
            );
            p.faded_quad(panel, "row-selection", 10.0, alpha);
            p.scene.push_clip(panel);
            for (i, label) in ["编辑内容", "复制链接", "归档项目"].iter().enumerate() {
                p.faded_body(
                    Rect::new(
                        panel.min_x() + 16.0,
                        panel.min_y() + 6.0 + i as f32 * 30.0,
                        panel.size.width - 32.0,
                        26.0,
                    ),
                    label,
                    detail.clamp(0.0, 1.0),
                );
            }
            p.scene.pop_clip();
        }
        RecipeKind::Press => {
            let button_rect = layout.button(Action::PressDemo);
            let id = Layout::action_id(Action::PressDemo);
            let label = action_label(Action::PressDemo, state).to_owned();
            let model = state.button_models.entry(id).or_default();
            if state.focused_button == Some(id) {
                model.state.focused = true;
            } else {
                model.state.focused = false;
                model.state.focus_visible = false;
            }
            let button_clip = visible_clip.intersection(&area).unwrap_or(Rect::new(
                visible_clip.min_x(),
                visible_clip.min_y(),
                0.0,
                0.0,
            ));
            let mut button_scene = Scene::new();
            p.ctx.text.render_button(
                &mut button_scene,
                model,
                &label,
                button_rect,
                Some(button_clip),
                p.ctx.theme,
                ButtonVariant::Panel,
                ButtonSize::Xs,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Center,
            );
            let mut adjusted_surface = false;
            for primitive in button_scene.primitives().iter().cloned() {
                if let Primitive::Quad(mut quad) = primitive {
                    if !adjusted_surface {
                        adjusted_surface = true;
                        quad.corner_radii =
                            tekes_ui_scene::CornerRadii::uniform(12.0 + detail.max(0.0) * 12.0);
                        p.scene.push_quad(quad);
                        p.scene.push_quad(
                            Quad::new(
                                button_rect,
                                color(p.ctx.theme, "row-selection")
                                    .with_alpha(alpha.clamp(0.0, 1.0)),
                            )
                            .with_corner_radius(12.0 + detail.max(0.0) * 12.0),
                        );
                    } else {
                        p.scene.push_quad(quad);
                    }
                } else {
                    p.scene.push(primitive);
                }
            }
            p.caption(
                Rect::new(
                    area.min_x() + 12.0,
                    area.max_y() - 26.0,
                    area.size.width - 24.0,
                    24.0,
                ),
                "形状：弹簧回弹     填充：有界效果轨道",
            );
        }
        RecipeKind::Skeleton => {
            let loaded = alpha;
            let x = area.min_x() + area.size.width * 0.14;
            let width = area.size.width * 0.72;
            let shimmer = ((recipe.now().as_secs_f64() / recipe.parameters.duration) % 1.0) as f32;
            for i in 0..4 {
                let row = Rect::new(
                    x,
                    area.min_y() + 12.0 + i as f32 * 32.0,
                    width - i as f32 * 18.0,
                    22.0,
                );
                p.faded_quad(row, "row-selection", 5.0, 1.0 - loaded);
                p.scene.push_clip(row);
                p.faded_quad(
                    Rect::new(
                        x + (width + amount) * shimmer - amount,
                        row.min_y(),
                        amount,
                        row.size.height,
                    ),
                    "secondary-text",
                    4.0,
                    (1.0 - loaded) * 0.12,
                );
                p.scene.pop_clip();
                p.faded_body(
                    row,
                    [
                        "真实内容 · 项目更新",
                        "新增三条设计评审意见",
                        "所有修改已同步",
                        "点击目标 A 模拟重新加载",
                    ][i],
                    loaded,
                );
            }
        }
        RecipeKind::Numeric => {
            let value = recipe.item_values()[0].clamp(0.0, 99999.0);
            let width = 26.0;
            let left = area.center().x - width * 2.5;
            let top = area.center().y - 24.0;
            p.caption(
                Rect::new(left, area.min_y() + 6.0, 200.0, 24.0),
                "TOTAL / 实时指标",
            );
            p.scene.push_clip(Rect::new(left, top, width * 5.0, 40.0));
            for i in 0..5 {
                let place = 10.0_f64.powi(4 - i);
                let scaled = value / place;
                let floor = scaled.floor();
                let fraction = (scaled - floor) as f32;
                let digit = (floor as u32) % 10;
                let next = (digit + 1) % 10;
                let x = left + i as f32 * width;
                p.body(
                    Rect::new(x, top - fraction * 40.0, 24.0, 40.0),
                    &digit.to_string(),
                );
                p.body(
                    Rect::new(x, top + (1.0 - fraction) * 40.0, 24.0, 40.0),
                    &next.to_string(),
                );
            }
            p.scene.pop_clip();
            p.caption(
                Rect::new(
                    area.min_x() + 16.0,
                    area.max_y() - 26.0,
                    area.size.width - 32.0,
                    24.0,
                ),
                "固定数字槽位 · 更新中反向 · 调整增量再切换",
            );
        }
        RecipeKind::Typing | RecipeKind::StreamTyping => {
            let units =
                crate::gallery::motion_preview::typing_fixture(state.locale, recipe.target());
            let full = units.concat();
            let role = proposal::TextRole::UiBody;
            let paragraph = p
                .ctx
                .text
                .proposal_document(&full, role, Some(area.size.width - 32.0));
            let origin = Point::new(area.min_x() + 16.0, area.min_y() + 12.0);
            let mut utf16_end = 0;
            for (i, unit) in units.iter().enumerate() {
                let start = utf16_end;
                utf16_end += unit.encode_utf16().count();
                let alpha = recipe.typing.opacity(i) as f32;
                if alpha <= 0.0 {
                    continue;
                }
                // Keep the original paragraph geometry; never reshape growing
                // prefixes. A ligature or emoji cluster becomes visible only
                // when its full UTF-16 cluster has reached the reveal frontier.
                let mut layer = (*paragraph).clone();
                for line in &mut layer.lines {
                    for run in &mut line.runs {
                        run.glyphs.retain(|g| {
                            g.cluster_utf16.end > start && g.cluster_utf16.end <= utf16_end
                        });
                    }
                }
                p.ctx.text.push(
                    p.scene,
                    &layer,
                    origin,
                    color(p.ctx.theme, "primary-text").with_alpha(alpha),
                );
            }
            p.caption(
                Rect::new(area.min_x() + 16.0, area.max_y() - 28.0, 76.0, 24.0),
                "已到达",
            );
            p.number(
                Rect::new(area.min_x() + 78.0, area.max_y() - 28.0, 60.0, 24.0),
                recipe.typing.available() as f64,
            );
            p.caption(
                Rect::new(area.min_x() + 150.0, area.max_y() - 28.0, 76.0, 24.0),
                "已显示",
            );
            p.number(
                Rect::new(area.min_x() + 212.0, area.max_y() - 28.0, 60.0, 24.0),
                recipe.typing.visible() as f64,
            );
            p.caption(
                Rect::new(
                    area.min_x() + 288.0,
                    area.max_y() - 28.0,
                    area.size.width - 304.0,
                    24.0,
                ),
                "A/B 切换文本",
            );
        }
        RecipeKind::Rotate2D
        | RecipeKind::FlipX
        | RecipeKind::FlipY
        | RecipeKind::SpinBrake
        | RecipeKind::RotateSettle => {
            use tekes_ui_motion::recipes::{RotationAxis, project_rotation};
            let axis = match recipe.kind {
                RecipeKind::FlipX => RotationAxis::X,
                RecipeKind::FlipY => RotationAxis::Y,
                _ => RotationAxis::Z,
            };
            let degrees = f64::from(geometry) * recipe.parameters.rotation_degrees;
            let center = Point::new(area.center().x, area.min_y() + 72.0);
            let project = |x: f64, y: f64| {
                let [px, py] = project_rotation([x, y], degrees, axis, 420.0).unwrap();
                Point::new(center.x + px as f32, center.y + py as f32)
            };
            let corners = [
                project(-52.0, -36.0),
                project(52.0, -36.0),
                project(52.0, 36.0),
                project(-52.0, 36.0),
            ];
            let back = axis != RotationAxis::Z && degrees.to_radians().cos() < 0.0;
            p.polygon(&corners, if back { "separator" } else { "row-selection" });
            for i in 0..4 {
                p.line(corners[i], corners[(i + 1) % 4], "primary-text");
            }
            let mark = [
                project(-40.0, -24.0),
                project(-14.0, -24.0),
                project(-14.0, -16.0),
                project(-40.0, -16.0),
            ];
            p.polygon(&mark, "primary-text");
            // Asymmetric vector marks make orientation legible even after
            // full turns; no upright text is pasted onto a rotating face.
            for (a, b) in [
                ((-12.0, -10.0), (-12.0, 16.0)),
                ((-12.0, -10.0), (12.0, -10.0)),
                ((-12.0, 2.0), (6.0, 2.0)),
            ] {
                p.line(project(a.0, a.1), project(b.0, b.1), "primary-text");
            }
            p.quad(
                Rect::new(center.x - 2.0, center.y - 2.0, 4.0, 4.0),
                "primary-text",
                2.0,
                false,
            );
            p.caption(
                Rect::new(
                    area.min_x() + 8.0,
                    area.max_y() - 24.0,
                    area.size.width - 16.0,
                    22.0,
                ),
                match recipe.kind {
                    RecipeKind::FlipX => "X 轴 · 上下翻转 · 透视投影",
                    RecipeKind::FlipY => "Y 轴 · 左右翻转 · 透视投影",
                    RecipeKind::SpinBrake => "慢起步 → 加速 → 减速刹停 · 中途换向保留角速度",
                    RecipeKind::RotateSettle => "弹簧越过目标后轻微回摆 · 阻尼可调",
                    _ => "Z 轴 · 平面旋转 · 中途反向",
                },
            );
        }
        RecipeKind::TextScan => {
            let role = proposal::TextRole::UiHeading;
            let lines = [
                "DESIGN IN MOTION",
                "让想法逐渐清晰",
                "Thinking, then creating.",
            ];
            let width = lines
                .iter()
                .map(|text| p.ctx.text.proposal_label(text, role, None).width as f32)
                .fold(0.0_f32, f32::max)
                .min(area.size.width - 32.0)
                + 8.0;
            let block = Rect::new(
                area.center().x - width / 2.0,
                area.min_y() + 10.0,
                width,
                108.0,
            );
            let band = tekes_ui_motion::recipes::TextScanBand {
                angle_degrees: recipe.parameters.scan_angle,
                width: recipe.parameters.amount,
            };
            for (i, text) in lines.iter().enumerate() {
                let slot = Rect::new(
                    block.min_x(),
                    block.min_y() + i as f32 * 34.0,
                    block.size.width,
                    30.0,
                );
                p.label(
                    slot,
                    text,
                    role,
                    if state.reduced_motion {
                        "primary-text"
                    } else {
                        "secondary-text"
                    },
                );
                // Raster-aligned horizontal clips approximate a diagonal mask
                // without changing shared renderer shaders. The text itself is
                // redrawn at one stable origin, so only glyph pixels light up.
                for row in 0..30 {
                    let y = slot.min_y() + row as f32;
                    for (fraction, alpha) in [(1.0, 0.22), (0.68, 0.35), (0.36, 0.85)] {
                        if let Some((lo, hi)) = band.span(
                            [f64::from(block.size.width), f64::from(block.size.height)],
                            f64::from(y - block.min_y() + 0.5),
                            f64::from(geometry),
                        ) {
                            let middle = (lo + hi) * 0.5;
                            let half = (hi - lo) * 0.5 * fraction;
                            let lo = middle - half;
                            let hi = middle + half;
                            let left = (block.min_x() + lo as f32).max(slot.min_x());
                            let right = (block.min_x() + hi as f32).min(slot.max_x());
                            if right > left {
                                p.scene.push_clip(Rect::new(left, y, right - left, 1.0));
                                p.ctx.text.proposal_label_in(
                                    p.scene,
                                    p.ctx.theme,
                                    slot,
                                    text,
                                    role,
                                    color(p.ctx.theme, "primary-text").with_alpha(alpha),
                                );
                                p.scene.pop_clip();
                            }
                        }
                    }
                }
            }
            p.caption(
                Rect::new(
                    area.min_x() + 12.0,
                    area.max_y() - 26.0,
                    area.size.width - 24.0,
                    24.0,
                ),
                "亮带只照亮字形 · A/B 改变扫描方向 · 暂停 / 继续当前",
            );
        }
        RecipeKind::PageSwitch => {
            let rail = Rect::new(area.min_x(), area.min_y(), 92.0, area.size.height);
            p.quad(rail, "row-selection", 8.0, false);
            for (i, label) in ["总览", "项目", "设置"].iter().enumerate() {
                p.caption(
                    Rect::new(
                        rail.min_x() + 12.0,
                        rail.min_y() + 12.0 + i as f32 * 34.0,
                        68.0,
                        26.0,
                    ),
                    label,
                );
            }
            let bounds = Rect::new(
                rail.max_x() + 16.0,
                area.min_y(),
                area.size.width - 108.0,
                area.size.height,
            );
            for (incoming, title) in [(false, "工作区 A"), (true, "工作区 B")] {
                let fade = if incoming { alpha } else { 1.0 - alpha };
                let scale = if incoming {
                    1.0 - (1.0 - g) * amount / 2000.0
                } else {
                    1.0
                };
                let width = bounds.size.width * scale;
                let card = Rect::new(
                    bounds.center().x - width / 2.0,
                    bounds.min_y() + (1.0 - g) * if incoming { 16.0 } else { 0.0 },
                    width,
                    bounds.size.height,
                );
                p.faded_quad(card, "surface-panel", 8.0, fade);
                p.faded_body(
                    Rect::new(card.min_x() + 16.0, card.min_y() + 12.0, width - 32.0, 28.0),
                    title,
                    fade,
                );
                for i in 0..3 {
                    p.faded_quad(
                        Rect::new(
                            card.min_x() + 16.0,
                            card.min_y() + 56.0 + i as f32 * 26.0,
                            width - 32.0,
                            18.0,
                        ),
                        "row-selection",
                        4.0,
                        fade,
                    );
                }
            }
        }
        RecipeKind::Reading => {
            let height = recipe.inserted_height() as f32;
            let column = (area.size.width - 16.0) / 2.0;
            for (i, anchored) in [false, true].into_iter().enumerate() {
                let pane = Rect::new(
                    area.min_x() + i as f32 * (column + 16.0),
                    area.min_y(),
                    column,
                    area.size.height,
                );
                p.caption(
                    Rect::new(pane.min_x() + 8.0, pane.min_y(), column - 16.0, 22.0),
                    if anchored {
                        "补偿滚动 · 阅读位置稳定"
                    } else {
                        "对照 · 未补偿滚动"
                    },
                );
                let viewport = Rect::new(
                    pane.min_x(),
                    pane.min_y() + 26.0,
                    column,
                    pane.size.height - 26.0,
                );
                p.quad(viewport, "surface-panel", 6.0, true);
                p.scene.push_clip(viewport);
                let scroll = if anchored {
                    recipe.scroll_offset() as f32
                } else {
                    recipe.user_scroll as f32
                };
                let inserted = Rect::new(
                    viewport.min_x() + 4.0,
                    viewport.min_y() - scroll,
                    column - 8.0,
                    height.max(0.0),
                );
                p.faded_quad(inserted, "row-selection", 4.0, alpha);
                if height > 20.0 {
                    p.faded_body(
                        Rect::new(
                            inserted.min_x() + 8.0,
                            inserted.min_y() + 4.0,
                            column - 24.0,
                            24.0,
                        ),
                        "新到达的内容",
                        alpha,
                    );
                }
                for row in 0..9 {
                    let y = viewport.min_y() + row as f32 * 36.0 + height - scroll;
                    let rect = Rect::new(viewport.min_x() + 4.0, y, column - 8.0, 32.0);
                    if row == 2 {
                        p.quad(rect, "row-selection", 4.0, true);
                    }
                    p.body(
                        Rect::new(rect.min_x() + 8.0, y, column - 24.0, 30.0),
                        [
                            "01 · 背景与目标",
                            "02 · 设计约束",
                            "03 · 正在阅读的内容",
                            "04 · 交互方案",
                            "05 · 视觉细节",
                            "06 · 验证记录",
                            "07 · 后续工作",
                            "08 · 附录",
                            "09 · 参考资料",
                        ][row],
                    );
                }
                p.scene.pop_clip();
            }
        }
    }
    p.scene.pop_clip();
    let status = if state.reduced_motion {
        "减少动态效果 · 即时完成"
    } else if recipe.phase == Phase::Cancelled {
        "已暂停 / 反向 · 可继续或重播"
    } else if recipe.needs_frame() {
        "运行中 · 可随时改参或反向"
    } else {
        "已停止 · 无动画帧请求"
    };
    p.caption(
        Rect::new(
            r.min_x() + 20.0,
            r.max_y() - 34.0,
            if recipe.kind == RecipeKind::Reading && layout.reading_inside_stage {
                r.size.width - 270.0
            } else {
                r.size.width - 40.0
            },
            24.0,
        ),
        status,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_platform_macos::{Modifiers, MouseEvent, Point as NativePoint};
    fn mouse(kind: MouseEventKind, p: Point) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            position: NativePoint {
                x: f64::from(p.x),
                y: f64::from(p.y),
            },
            modifiers: Modifiers::NONE,
        })
    }
    #[test]
    fn ticking_tab_follows_symbols_and_owns_only_its_counter_controls() {
        let mut state = MotionPreview::default();
        let mut text = crate::gallery::text::TextEngine::new();
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let layout = Layout::for_state(content, &state, &mut text);
        let catalogs: Vec<_> = layout
            .state_actions(&state)
            .into_iter()
            .filter_map(|(action, _)| match action {
                Action::Catalog(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(
            &catalogs[3..],
            &[PresetCatalog::Symbols, PresetCatalog::TickingNumber]
        );
        assert!(layout.button(Action::TickReplay).is_empty());
        apply_action(&mut state, Action::Catalog(PresetCatalog::TickingNumber));
        let layout = Layout::for_state(content, &state, &mut text);
        let actions = layout.state_actions(&state);
        assert!(actions.iter().all(|(a, _)| matches!(
            a,
            Action::Catalog(_)
                | Action::TickReplay
                | Action::TickDigits
                | Action::TickStrength
                | Action::TickDuration
                | Action::ReduceMotion
        )));
        assert_eq!(actions.len(), 10);
        assert!(layout.button(Action::Target(0)).is_empty());
        let point = layout.button(Action::TickReplay).center();
        for kind in [
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
            MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            },
        ] {
            assert!(handle_input(
                &mut state,
                &mouse(kind, point),
                layout.content
            ));
        }
        assert_eq!(state.ticking.target(), 546);
        assert!(state.needs_animation());
        // Leaving this tab suspends its frame work without losing its current pose.
        apply_action(&mut state, Action::Catalog(PresetCatalog::Apple));
        let value = state.ticking.value();
        state.animate(1.0);
        assert_eq!(state.ticking.value(), value);
    }

    #[test]
    fn ticking_demo_actions_drive_counter_and_system_reduction() {
        let mut state = MotionPreview::default();
        state.select_catalog(PresetCatalog::TickingNumber);
        assert!(!state.needs_animation());
        apply_action(&mut state, Action::TickReplay);
        assert_eq!(state.ticking.target(), 546);
        assert!(state.needs_animation());
        assert!(state.animate(0.9));
        assert!(state.ticking.value() > 137 && state.ticking.value() < 546);
        assert!(state.animate(2.0)); // Includes the final frame.
        assert_eq!(state.ticking.value(), 546);
        assert!(!state.needs_animation());
        apply_action(&mut state, Action::TickReplay);
        assert_eq!(state.ticking.target(), 137);
        state.set_system_reduced_motion(true);
        assert_eq!(state.ticking.value(), 137);
        assert!(!state.needs_animation());
        apply_action(&mut state, Action::TickDigits);
        assert_eq!(state.ticking.digits(), 6);
        assert_eq!(state.ticking.text(), "000137");
        apply_action(&mut state, Action::TickDigits);
        assert_eq!(state.ticking.digits(), 2);
        apply_action(&mut state, Action::TickReplay);
        assert_eq!(state.ticking.text(), "46");
        apply_action(&mut state, Action::TickStrength);
        assert_eq!(state.ticking.strength(), 0.0);
        apply_action(&mut state, Action::TickDuration);
        assert_eq!(state.ticking.duration().as_secs(), 2);
    }

    #[test]
    fn existing_icon_selectors_use_native_motion_hit_routing() {
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let mut state = MotionPreview::default();
        state.select_catalog(PresetCatalog::Symbols);
        let mut text = crate::gallery::text::TextEngine::new();
        let layout = Layout::for_state(content, &state, &mut text);
        for i in 0..tekes_ui_icons::Icon::ALL.len() {
            let point = layout.button(Action::SymbolIcon(i)).center();
            for kind in [
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ] {
                assert!(handle_input(&mut state, &mouse(kind, point), content));
            }
            assert_eq!(state.symbol_icon, tekes_ui_icons::Icon::ALL[i]);
            assert!(state.needs_animation());
            state.pause_recipe();
            assert!(!state.needs_animation());
        }
    }

    #[test]
    fn every_action_uses_same_layout_for_drawing_and_hit_routing() {
        let content = Rect::new(312.0, 80.0, 1104.0, 790.0);
        let mut state = MotionPreview::default();
        let actions = {
            let mut text = crate::gallery::text::TextEngine::new();
            Layout::for_state(content, &state, &mut text)
                .state_actions(&state)
                .into_iter()
                .map(|(action, _)| action)
                .collect::<Vec<_>>()
        };
        for action in actions {
            let mut text = crate::gallery::text::TextEngine::new();
            let layout = Layout::for_state(content, &state, &mut text);
            let Some((_, rect)) = layout
                .state_actions(&state)
                .into_iter()
                .find(|(candidate, _)| *candidate == action)
            else {
                continue;
            };
            assert!(rect.min_x() >= content.min_x() && rect.max_x() <= content.max_x());
            // Actions can change footer height; reproduce the host's layout pass
            // before routing the next event through retained button geometry.
            install_button_placements(&mut state, layout, Theme::LIGHT);
            let at = rect.center();
            assert!(handle_input(
                &mut state,
                &mouse(
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count: 1
                    },
                    at
                ),
                layout.content
            ));
            assert!(handle_input(
                &mut state,
                &mouse(
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        click_count: 1
                    },
                    at
                ),
                layout.content
            ));
            if let Action::Preset(i) = action {
                assert_eq!(state.selected, i);
            }
        }
    }

    #[test]
    fn clipped_input_cancels_outside_button_release_but_keeps_slider_capture() {
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let visible = Rect::new(24.0, 24.0, 1104.0, 420.0);
        let layout = Layout::new(content);
        let mut state = MotionPreview::default();
        let button = layout
            .button(Action::Catalog(PresetCatalog::Recipes))
            .center();
        let outside = Point::new(visible.max_x() + 4.0, visible.max_y() + 4.0);
        assert!(handle_input_clipped(
            &mut state,
            &mouse(
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                button,
            ),
            content,
            visible,
        ));
        assert!(handle_input_clipped(
            &mut state,
            &mouse(
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                outside,
            ),
            content,
            visible,
        ));
        assert_eq!(state.catalog, PresetCatalog::Apple);

        let parameter = Parameter::ALL[0];
        let slider = layout.slider(parameter);
        let before = state.parameter_value(parameter);
        assert!(handle_input_clipped(
            &mut state,
            &mouse(
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                slider.center(),
            ),
            content,
            visible,
        ));
        assert!(handle_input_clipped(
            &mut state,
            &mouse(
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
                outside,
            ),
            content,
            visible,
        ));
        assert_ne!(state.parameter_value(parameter), before);
        assert!(handle_input_clipped(
            &mut state,
            &mouse(
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                outside,
            ),
            content,
            visible,
        ));
        assert_eq!(state.dragging, None);
    }
    #[test]
    fn motion_recipes_controls_pause_reverse_scroll_and_change_parameters() {
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let click = |state: &mut MotionPreview, action: Action| {
            let mut text = crate::gallery::text::TextEngine::new();
            let point = Layout::for_state(content, state, &mut text)
                .button(action)
                .center();
            for kind in [
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ] {
                handle_input(state, &mouse(kind, point), content);
            }
        };
        let mut state = MotionPreview::default();
        click(&mut state, Action::Catalog(PresetCatalog::Recipes));
        assert!(state.is_recipe());
        for kind in 0..RecipeKind::ALL.len() {
            click(&mut state, Action::Preset(kind));
            state.animate(0.07);
            let before = state.recipe.values();
            click(&mut state, Action::Target(1));
            state.animate(2.0);
            assert_eq!(state.recipe.values(), before);
            assert!(!state.needs_animation());
            click(&mut state, Action::Target(2));
            assert!(state.needs_animation());
            let slider_at = {
                let mut text = crate::gallery::text::TextEngine::new();
                Layout::for_state(content, &state, &mut text)
                    .slider(Parameter::Frequency)
                    .center()
            };
            for kind in [
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
            ] {
                handle_input(&mut state, &mouse(kind, slider_at), content);
            }
            let (lo, hi) = state.parameter_range(Parameter::Frequency);
            assert!((state.parameter_value(Parameter::Frequency) - (lo + hi) * 0.5).abs() < 0.001);
            click(&mut state, Action::Target(0));
            for _ in 0..500 {
                state.animate(0.02);
            }
            assert_eq!(state.recipe.values(), [0.0; 4]);
            assert!(!state.needs_animation());
        }
        state.select(3);
        state.pause_recipe();
        let old = state.recipe.user_scroll;
        click(&mut state, Action::ReadingDown);
        assert_eq!(state.recipe.user_scroll, old + 36.0);
        assert!(!state.needs_animation());
    }

    #[test]
    fn motion_recipe_drag_and_press_use_captured_mouse_routes() {
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let layout = Layout::new(content);
        let down = MouseEventKind::Down {
            button: MouseButton::Left,
            click_count: 1,
        };
        let up = MouseEventKind::Up {
            button: MouseButton::Left,
            click_count: 1,
        };
        let drag = MouseEventKind::Dragged {
            button: MouseButton::Left,
        };
        let mut state = MotionPreview::default();
        state.select_catalog(PresetCatalog::Recipes);
        state.select(7);
        let point = layout.stage.center();
        handle_input(&mut state, &mouse(down, point), content);
        assert!(state.recipe_dragging);
        assert!(state.pressed.is_some());
        handle_input(
            &mut state,
            &mouse(drag, Point::new(layout.stage.max_x() + 120.0, point.y)),
            content,
        );
        assert_eq!(state.recipe.values()[0], 1.0);
        handle_input(&mut state, &mouse(up, point), content);
        assert!(!state.recipe_dragging);
        assert!(state.pressed.is_none());
        state.animate(0.1);
        handle_input(&mut state, &mouse(down, point), content);
        handle_input(&mut state, &Event::Focus(false), content);
        assert!(!state.recipe_dragging);
        assert!(!state.needs_animation());
        state.select(10);
        handle_input(&mut state, &mouse(down, point), content);
        assert!(state.recipe.target());
        handle_input(
            &mut state,
            &mouse(drag, Point::new(-100.0, -100.0)),
            content,
        );
        assert!(!state.recipe.target());
        handle_input(&mut state, &mouse(drag, point), content);
        assert!(state.recipe.target());
        handle_input(&mut state, &mouse(up, point), content);
        assert!(!state.recipe.target());
    }

    #[test]
    fn slider_capture_drag_and_focus_loss() {
        let content = Rect::new(312.0, 80.0, 1104.0, 790.0);
        let layout = Layout::new(content);
        let mut state = MotionPreview::default();
        let p = layout.slider(Parameter::Frequency).center();
        handle_input(
            &mut state,
            &mouse(
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                p,
            ),
            content,
        );
        assert_eq!(state.dragging, Some(Parameter::Frequency));
        handle_input(
            &mut state,
            &mouse(
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
                Point::new(3000.0, p.y),
            ),
            content,
        );
        assert_eq!(state.frequency, Parameter::Frequency.range().1);
        assert!(handle_input(&mut state, &Event::Focus(false), content));
        assert!(state.dragging.is_none());
    }

    #[test]
    fn manual_toggle_is_independent_of_system_reduction_in_the_real_hit_path() {
        let content = Rect::new(312.0, 80.0, 1104.0, HEIGHT);
        let mut state = MotionPreview::default();
        state.set_system_reduced_motion(true);
        let point = Layout::new(content).button(Action::ReduceMotion).center();
        for (kind, count) in [
            (
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                0,
            ),
            (
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                1,
            ),
        ] {
            handle_input(&mut state, &mouse(kind, point), content);
            assert_eq!(state.manual_reduced_motion(), count == 1);
            assert!(state.reduced_motion);
        }
        state.set_system_reduced_motion(false);
        assert!(state.reduced_motion);
        assert!(!state.needs_animation());
    }

    #[test]
    fn translated_and_narrow_document_layout_keeps_controls_and_footer_in_extent() {
        for width in [100.0, 640.0, 840.0, 1104.0] {
            let content = Rect::new(-40.0, -230.0, width, 200.0);
            let layout = Layout::new(content);
            assert_eq!(layout.content.size, extent(content));
            for (_, rect) in layout.actions(RecipeKind::ALL.len()) {
                assert!(rect.min_x() >= layout.content.min_x());
                assert!(rect.max_x() <= layout.content.max_x());
                assert!(rect.min_y() >= layout.content.min_y());
                assert!(rect.max_y() <= layout.content.max_y());
            }
            assert!(layout.content.min_y() + 894.0 + 24.0 < layout.content.max_y());
            assert!(layout.stage.size.width > 0.0);
            for parameter in Parameter::ALL {
                assert!(layout.slider(parameter).size.width > 0.0);
            }
        }
    }

    #[test]
    fn measured_motion_buttons_wrap_without_overflow_or_overlap() {
        use crate::gallery::locale::Locale;

        for locale in [Locale::En, Locale::ZhCn] {
            for scale in [1.0, 1.25, 1.5] {
                for width in [720.0, 800.0, 1000.0, 1440.0] {
                    for catalog in [
                        PresetCatalog::Apple,
                        PresetCatalog::DesignSystem,
                        PresetCatalog::Recipes,
                        PresetCatalog::Symbols,
                    ] {
                        let content = Rect::new(24.0, 32.0, width, 760.0);
                        let mut state = MotionPreview::default();
                        state.locale = locale;
                        state.select_catalog(catalog);
                        let mut text = crate::gallery::text::TextEngine::new();
                        text.set_type_scale(scale);
                        let layout = Layout::for_state(content, &state, &mut text);
                        let actions = layout.state_actions(&state);
                        for (i, (action, frame)) in actions.iter().enumerate() {
                            assert!(frame.min_x() >= content.min_x() - 0.01);
                            assert!(frame.max_x() <= content.max_x() + 0.01);
                            assert!(frame.min_y() >= content.min_y() - 0.01);
                            assert!(frame.max_y() <= layout.content.max_y() + 0.01);
                            let measured = text.measure_button(
                                action_label(*action, &state),
                                frame.size.width,
                                ButtonSize::Xs,
                                proposal::TextRole::UiBody,
                            );
                            assert!(
                                frame.size.height + 0.05 >= measured.height,
                                "{locale:?} {scale}x {width}px {action:?}: frame={frame:?}, measured={measured:?}"
                            );
                            for (other, other_frame) in actions.iter().skip(i + 1) {
                                let overlaps = frame.min_x() < other_frame.max_x() - 0.01
                                    && frame.max_x() > other_frame.min_x() + 0.01
                                    && frame.min_y() < other_frame.max_y() - 0.01
                                    && frame.max_y() > other_frame.min_y() + 0.01;
                                assert!(
                                    !overlaps,
                                    "button {action:?} overlaps {other:?} at {locale:?} {scale}x {width}px"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn wide_default_layout_preserves_approved_motion_positions() {
        let content = Rect::new(0.0, 0.0, 1440.0, 900.0);
        let state = MotionPreview::default();
        let mut text = crate::gallery::text::TextEngine::new();
        let layout = Layout::for_state(content, &state, &mut text);
        assert_eq!(layout.stage.min_y(), 270.0);
        assert_eq!(layout.stage.size.height, 240.0);
        assert_eq!(layout.graph.min_y(), 546.0);
        assert_eq!(layout.graph.size.height, 180.0);
        let controls = [
            (Action::Target(0), 0.0),
            (Action::Target(1), 92.0),
            (Action::Target(2), 184.0),
            (Action::Replay, 284.0),
            (Action::Interrupt, 376.0),
            (Action::Resume, 524.0),
        ];
        for (action, x) in controls {
            let frame = layout.button(action);
            assert_eq!(frame.min_x(), x, "{action:?}");
            assert_eq!(frame.min_y(), 752.0, "{action:?}");
        }
    }

    #[test]
    fn motion_recipes_reuse_cached_labels_and_render_finite_geometry() {
        use crate::gallery::{static_text::StaticText, text::TextEngine};
        use tekes_ui_scene::Primitive;
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let mut text = TextEngine::new();
        let mut fixture = StaticText::new(None);
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                text.set_scale(scale);
                let mut ctx = PageCtx {
                    theme,
                    text: &mut text,
                    static_text: &mut fixture,
                };
                let mut scene = Scene::new();
                let mut state = MotionPreview::default();
                for catalog in [PresetCatalog::Recipes, PresetCatalog::Symbols] {
                    state.select_catalog(catalog);
                    Painter {
                        ctx: &mut ctx,
                        scene: &mut scene,
                    }
                    .digits(content, "0123456789.-#", "primary-text");
                    for index in 0..state.presets.len() {
                        state.select(index);
                        for dt in [0.0, 0.2, 0.6, 4.0] {
                            state.animate(dt);
                            build(&mut ctx, content, content, &mut scene, &mut state);
                        }
                        let shapes = ctx.text.shape_count();
                        state.replay();
                        for frame in 0..120 {
                            state.animate(1.0 / 120.0);
                            ctx.text.begin_frame();
                            scene.clear();
                            build(&mut ctx, content, content, &mut scene, &mut state);
                            assert_eq!(
                                ctx.text.shape_count(),
                                shapes,
                                "recipe {index} must reuse shapes: frame={frame}, events={:?}, scale={scale}, theme={theme:?}",
                                state.events
                            );
                            assert!(scene.primitives().iter().all(|p| match p {
                                Primitive::Quad(q) =>
                                    q.rect.origin.x.is_finite()
                                        && q.rect.origin.y.is_finite()
                                        && q.rect.size.width.is_finite()
                                        && q.rect.size.height.is_finite()
                                        && q.rect.size.width >= 0.0
                                        && q.rect.size.height >= 0.0,
                                _ => true,
                            }));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn rebuilds_cache_telemetry_and_keep_reduce_motion_button_above_background() {
        use crate::gallery::{static_text::StaticText, text::TextEngine};
        use tekes_ui_scene::Primitive;
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let mut text = TextEngine::new();
        let mut fixture = StaticText::new(None);
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                text.set_scale(scale);
                let mut ctx = PageCtx {
                    theme,
                    text: &mut text,
                    static_text: &mut fixture,
                };
                let mut scene = Scene::new();
                let mut state = MotionPreview::default();
                // Prime the finite telemetry alphabet and all lifecycle labels.
                Painter {
                    ctx: &mut ctx,
                    scene: &mut scene,
                }
                .digits(content, "0123456789.-#", "primary-text");
                build(&mut ctx, content, content, &mut scene, &mut state);
                state.replay();
                build(&mut ctx, content, content, &mut scene, &mut state);
                state.animate(0.5);
                build(&mut ctx, content, content, &mut scene, &mut state);
                state.animate(2.0);
                build(&mut ctx, content, content, &mut scene, &mut state);
                let shapes = ctx.text.shape_count();
                state.replay();
                for _ in 0..240 {
                    state.animate(1.0 / 120.0);
                    ctx.text.begin_frame();
                    scene.clear();
                    build(&mut ctx, content, content, &mut scene, &mut state);
                    assert_eq!(
                        ctx.text.shape_count(),
                        shapes,
                        "telemetry must reuse cached shapes"
                    );
                }
                state.set_reduced_motion(true);
                ctx.text.begin_frame();
                scene.clear();
                build(&mut ctx, content, content, &mut scene, &mut state);
                let center = Layout::new(content).button(Action::ReduceMotion).center();
                let top = scene
                    .primitives()
                    .iter()
                    .rev()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect.contains(center) => Some(q),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(top.fill, color(theme, "row-selection"));
                assert!(scene.primitives().iter().all(|p| match p {
                    Primitive::Quad(q) =>
                        q.rect.size.width.is_finite() && q.rect.size.height.is_finite(),
                    _ => true,
                }));
            }
        }
    }
}
