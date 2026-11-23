use super::{ComponentEvent, has_area, safe_clip, valid};
use crate::control_motion::{ProgressTransition, motion_for};
use crate::{
    ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::{Axis, Breadcrumb, ControlFrame, SegmentedControl, Stack},
    paint::ButtonVariant,
};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SegmentedLayout {
    pub frame: Rect,
    pub items: Vec<ButtonLayout>,
}
#[derive(Clone, Debug)]
pub struct SegmentedControlView {
    buttons: Vec<ButtonModel>,
    layout: SegmentedLayout,
    surface: &'static str,
    focused: Option<usize>,
    focus_visible: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    space_armed: Option<usize>,
    window_active: bool,
    disabled: bool,
    thumb_x: ProgressTransition,
    thumb_width: ProgressTransition,
    thumb_motion_initialized: bool,
    last_selected: Option<usize>,
    motion_now: Duration,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SegmentedActionSnapshot {
    layout: SegmentedLayout,
    buttons: Vec<crate::widgets::ButtonActionSnapshot>,
    focus: (Option<usize>, bool),
    hovered: Option<usize>,
    pressed: Option<usize>,
    space_armed: Option<usize>,
    window_active: bool,
    disabled: bool,
}
#[derive(Debug, PartialEq)]
pub(crate) struct SegmentedViewDecision {
    expected: SegmentedActionSnapshot,
    effect: SegmentedViewEffect,
}
#[derive(Debug, PartialEq)]
enum SegmentedViewEffect {
    Pointer {
        index: usize,
        button: crate::widgets::ButtonActionDecision,
        cancel: bool,
    },
    Focus {
        index: usize,
        visible: bool,
        cancel: bool,
    },
    Select {
        index: usize,
        selection: Option<crate::extended::SegmentedSelectionDecision>,
    },
}
impl SegmentedViewDecision {
    pub(crate) fn changed_selection(&self) -> bool {
        matches!(
            self.effect,
            SegmentedViewEffect::Select {
                selection: Some(_),
                ..
            }
        )
    }
}
impl SegmentedControlView {
    pub(crate) fn action_snapshot(&self) -> SegmentedActionSnapshot {
        SegmentedActionSnapshot {
            layout: self.layout.clone(),
            buttons: self
                .buttons
                .iter()
                .map(ButtonModel::action_snapshot)
                .collect(),
            focus: (self.focused, self.focus_visible),
            hovered: self.hovered,
            pressed: self.pressed,
            space_armed: self.space_armed,
            window_active: self.window_active,
            disabled: self.disabled,
        }
    }
    pub(crate) fn prepare_pointer_down(
        &self,
        p: Point,
        cancel: bool,
    ) -> Option<SegmentedViewDecision> {
        if !self.window_active || self.disabled {
            return None;
        }
        let index = self.layout.items.iter().position(|l| l.hit_test(p))?;
        let button = self.buttons.get(index)?;
        if button.state.disabled {
            return None;
        }
        Some(SegmentedViewDecision {
            expected: self.action_snapshot(),
            effect: SegmentedViewEffect::Pointer {
                index,
                button: button.prepare_content_down_after_cancel(p, false, cancel),
                cancel,
            },
        })
    }
    pub(crate) fn prepare_focus(
        &self,
        index: usize,
        visible: bool,
        cancel: bool,
    ) -> Option<SegmentedViewDecision> {
        if self.disabled || self.buttons.get(index).is_none_or(|b| b.state.disabled) {
            return None;
        }
        Some(SegmentedViewDecision {
            expected: self.action_snapshot(),
            effect: SegmentedViewEffect::Focus {
                index,
                visible,
                cancel,
            },
        })
    }
    pub(crate) fn prepare_select(
        &self,
        index: usize,
        model: &SegmentedControl,
    ) -> SegmentedViewDecision {
        SegmentedViewDecision {
            expected: self.action_snapshot(),
            effect: SegmentedViewEffect::Select {
                index,
                selection: (!self.disabled)
                    .then(|| model.prepare_select(index))
                    .flatten(),
            },
        }
    }
    fn apply_pointer_decision(&mut self, decision: SegmentedViewDecision) {
        let SegmentedViewEffect::Pointer {
            index,
            button,
            cancel,
        } = decision.effect
        else {
            unreachable!()
        };
        if cancel {
            self.cancel();
        }
        self.buttons[index].apply_content_action(button);
        self.pressed = Some(index);
        self.focused = Some(index);
        self.focus_visible = false;
    }
    fn apply_focus_decision(&mut self, decision: SegmentedViewDecision) {
        let SegmentedViewEffect::Focus {
            index,
            visible,
            cancel,
        } = decision.effect
        else {
            unreachable!()
        };
        if cancel {
            self.cancel();
        }
        self.focused = Some(index);
        self.focus_visible = visible;
    }
    // Consume an authenticated original decision. No handler replay or late refusal.
    // Parent set_focus hooks only touch this View's focus/arms: these assignments
    // deliberately establish the same original final focus/gesture after that hook.
    pub(crate) fn apply_decision(
        &mut self,
        decision: SegmentedViewDecision,
        model: &mut SegmentedControl,
    ) {
        match decision.effect {
            SegmentedViewEffect::Pointer { .. } => self.apply_pointer_decision(decision),
            SegmentedViewEffect::Focus { .. } => self.apply_focus_decision(decision),
            SegmentedViewEffect::Select {
                index,
                selection: Some(selection),
            } => {
                model.apply_prepared_select(selection);
                self.focused = Some(index);
                self.focus_visible = false;
                self.cancel();
            }
            SegmentedViewEffect::Select {
                selection: None, ..
            } => {}
        }
    }

    /// Apply the parent's semantic Surface to the existing retained buttons.
    /// Validation completes before mutation; input/focus/capture are unchanged.
    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)?;
        self.surface = surface;
        for button in &mut self.buttons {
            button.set_surface(surface);
        }
        Ok(())
    }
    pub fn new(count: usize) -> Self {
        let mut buttons = vec![ButtonModel::default(); count];
        for button in &mut buttons {
            button.set_surface("canvas");
        }
        Self {
            buttons,
            layout: SegmentedLayout::default(),
            surface: "canvas",
            focused: None,
            focus_visible: false,
            hovered: None,
            pressed: None,
            space_armed: None,
            window_active: true,
            disabled: false,
            thumb_x: ProgressTransition::default(),
            thumb_width: ProgressTransition::default(),
            thumb_motion_initialized: false,
            last_selected: None,
            motion_now: Duration::default(),
        }
    }
    pub(crate) fn measure(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        available: Size,
        role: proposal::TextRole,
        compact: bool,
    ) -> Size {
        let count = labels.len();
        if count == 0 {
            return Size::default();
        }
        let size = if compact {
            ButtonSize::Sm
        } else {
            ButtonSize::Md
        };
        let inset = proposal::metric("spacing", "space-segmented-inset").unwrap_or(2.0);
        let width = ((available.width - inset * 2.0).max(0.) / count as f32).max(0.);
        let mut result = Size::default();
        for label in labels {
            let measured = ButtonModel::measure(text, label, width, size, role);
            result.width = result.width.max(measured.width);
            result.height = result.height.max(measured.height);
        }
        result.width = result.width * count as f32 + inset * 2.0;
        result.height += inset * 2.0;
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn arrange(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &SegmentedControl,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
        compact: bool,
    ) -> SegmentedLayout {
        if !valid(frame, clip) {
            self.cancel();
            self.focused = None;
            self.hovered = None;
            self.layout = SegmentedLayout::default();
            return self.layout.clone();
        }
        let count = model.len().min(labels.len()).min(self.buttons.len());
        if self
            .space_armed
            .is_some_and(|i| i >= count || !model.is_enabled(i))
            || self
                .pressed
                .is_some_and(|i| i >= count || !model.is_enabled(i))
        {
            self.cancel();
        }
        if self
            .focused
            .is_some_and(|i| i >= count || !model.is_enabled(i))
        {
            self.focused = (0..count).find(|i| model.is_enabled(*i));
        }
        let inset = proposal::metric("spacing", "space-segmented-inset").unwrap_or(2.0);
        let inset_x = inset.min(frame.size.width * 0.5).max(0.0);
        let inset_y = inset.min(frame.size.height * 0.5).max(0.0);
        let inner = Rect::new(
            frame.min_x() + inset_x,
            frame.min_y() + inset_y,
            (frame.size.width - inset_x * 2.0).max(0.0),
            (frame.size.height - inset_y * 2.0).max(0.0),
        );
        let cells = Stack::new(Axis::Horizontal).layout(inner, count);
        let mut items = Vec::with_capacity(count);
        for i in 0..count {
            let button = &mut self.buttons[i];
            let disabled = self.disabled || !model.is_enabled(i);
            button.state.disabled = disabled;
            button.state.selected = model.selected == i;
            button.state.focused = self.window_active && self.focused == Some(i);
            button.state.focus_visible = button.state.focused && self.focus_visible;
            button.state.hovered = self.window_active && self.hovered == Some(i);
            button.state.pressed = self.window_active && self.pressed == Some(i);
            let size = if compact {
                ButtonSize::Sm
            } else {
                ButtonSize::Md
            };
            items.push(button.prepare_layout(
                text,
                labels[i],
                cells[i],
                clip,
                size,
                role,
                ButtonTextAlign::Center,
                None,
            ));
        }
        let track_height = items
            .iter()
            .map(|item| item.frame.max_y() - frame.min_y() + inset_y)
            .fold(frame.size.height, f32::max);
        let track_frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, track_height);
        self.layout = SegmentedLayout {
            frame: track_frame,
            items,
        };
        if !self
            .layout
            .items
            .iter()
            .any(|item| item.visible.is_some_and(has_area))
        {
            self.cancel();
            self.focused = None;
            self.hovered = None;
        }
        self.layout.clone()
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_arranged(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &SegmentedControl,
        labels: &[&str],
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        _compact: bool,
    ) {
        let count = self.layout.items.len();
        if count == 0 {
            return;
        }
        let focus_last = self.focused.filter(|&index| {
            index < count
                && self.focus_visible
                && self.window_active
                && !self.disabled
                && model.is_enabled(index)
        });
        let clipped = safe_clip(scene, clip);
        let frame = self.layout.frame;
        let inset = proposal::metric("spacing", "space-segmented-inset").unwrap_or(2.0);
        let effective_inset = inset
            .min(frame.size.width * 0.5)
            .min(frame.size.height * 0.5)
            .max(0.0);
        let outer_radius = proposal::metric("radius", "c-segmented-radius")
            .unwrap_or(9999.0)
            .min(frame.size.height * 0.5)
            .min(frame.size.width * 0.5)
            .max(0.0);
        let thumb_radius = tekes_ui_tokens::CornerRadius::concentric(outer_radius, effective_inset);

        let track_fill = palette
            .color("segmented-track")
            .expect("segmented-track token");
        let frame_colors = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            self.surface,
            "surface-frame",
            "rest",
        )
        .expect("validated Segmented frame");
        let (border_width, _) = crate::paint::decorative_border(palette, text.backing_scale());
        let mut track_quad =
            tekes_ui_scene::Quad::new(frame, track_fill).with_corner_radius(outer_radius);
        if let Some(boundary) = frame_colors.boundary {
            track_quad = track_quad.with_border(border_width, boundary);
        }
        scene.push_quad(track_quad);

        let motions: Vec<(f32, f32)> = (0..count)
            .map(|i| self.buttons[i].update_segment_motion(text, palette))
            .collect();

        let hover_color = palette.color("button-hover").expect("button-hover token");
        let pressed_color = palette
            .color("button-pressed")
            .expect("button-pressed token");
        for (i, &(hover, pressed)) in motions.iter().enumerate() {
            if i == model.selected || self.buttons[i].state.disabled {
                continue;
            }
            if !self.layout.items[i].visible.is_some_and(has_area) {
                continue;
            }
            if hover <= 0. && pressed <= 0. {
                continue;
            }
            let hover_fill = crate::paint::mix_rgba_linear(track_fill, hover_color, hover);
            let fill = crate::paint::mix_rgba_linear(hover_fill, pressed_color, pressed);
            scene.push_quad(
                tekes_ui_scene::Quad::new(self.layout.items[i].frame, fill)
                    .with_corner_radius(thumb_radius),
            );
        }

        if model.selected < count
            && self.layout.items[model.selected]
                .visible
                .is_some_and(has_area)
        {
            let target = self.layout.items[model.selected].frame;
            let thumb_rect = if let Some(context) = text.motion_context() {
                self.motion_now = self.motion_now.max(context.now);
                let tx = f64::from(target.min_x());
                let tw = f64::from(target.size.width);
                if !self.thumb_motion_initialized || context.reduced_motion {
                    let _ = self.thumb_x.snap(tx, self.motion_now);
                    let _ = self.thumb_width.snap(tw, self.motion_now);
                    self.thumb_motion_initialized = true;
                    self.last_selected = Some(model.selected);
                } else if self.last_selected != Some(model.selected)
                    || self.thumb_x.needs_animation()
                    || self.thumb_width.needs_animation()
                {
                    let motion = motion_for("switch_thumb");
                    let _ = self.thumb_x.retarget(tx, motion, self.motion_now, false);
                    let _ = self
                        .thumb_width
                        .retarget(tw, motion, self.motion_now, false);
                    self.last_selected = Some(model.selected);
                } else {
                    let _ = self.thumb_x.snap(tx, self.motion_now);
                    let _ = self.thumb_width.snap(tw, self.motion_now);
                }
                if self.thumb_x.needs_animation() || self.thumb_width.needs_animation() {
                    text.request_animation_frame();
                }
                Rect::new(
                    self.thumb_x.value() as f32,
                    target.min_y(),
                    (self.thumb_width.value() as f32).max(0.),
                    target.size.height,
                )
            } else if self.thumb_x.needs_animation() || self.thumb_width.needs_animation() {
                let _ = self
                    .thumb_x
                    .snap(f64::from(target.min_x()), self.motion_now);
                let _ = self
                    .thumb_width
                    .snap(f64::from(target.size.width), self.motion_now);
                self.last_selected = Some(model.selected);
                target
            } else {
                target
            };
            let thumb_fill = palette
                .color("segmented-thumb")
                .expect("segmented-thumb token");
            let mut thumb_quad =
                tekes_ui_scene::Quad::new(thumb_rect, thumb_fill).with_corner_radius(thumb_radius);
            let thumb_disabled = self.disabled || !model.is_enabled(model.selected);
            if !thumb_disabled {
                if palette.theme.is_increased() {
                    let selection_colors = tekes_ui_tokens::surface_palette::resolve_surface(
                        palette,
                        self.surface,
                        "selection",
                        "selected",
                    )
                    .expect("validated Segmented selection");
                    let stroke = proposal::metric("stroke", "stroke-rule").unwrap_or(1.0);
                    if let Some(boundary) = selection_colors.boundary {
                        thumb_quad = thumb_quad.with_border(stroke, boundary);
                    }
                } else if let Some(boundary) = frame_colors.boundary {
                    thumb_quad = thumb_quad.with_border(border_width, boundary);
                }
            }
            scene.push_quad(thumb_quad);
        }

        for i in 0..count {
            let item = self.layout.items[i];
            let Some(visible) = item.visible.filter(|v| has_area(*v)) else {
                continue;
            };
            let label = labels.get(i).copied().unwrap_or("");
            if !label.is_empty() && !item.content.is_empty() {
                let ink = self.buttons[i]
                    .surface_colors(palette, ButtonVariant::Quiet, self.surface)
                    .text;
                scene.push_clip(visible);
                text.draw(
                    scene,
                    label,
                    role,
                    item.content,
                    ButtonTextAlign::Center,
                    ink,
                );
                scene.pop_clip();
            }
        }

        if let Some(i) = focus_last
            && self.layout.items[i].visible.is_some_and(has_area)
        {
            let colors =
                self.buttons[i].surface_colors(palette, ButtonVariant::Quiet, self.surface);
            if let Some(focus) = colors.focus {
                crate::paint::focus_ring_with_color(
                    scene,
                    self.layout.items[i].frame,
                    focus,
                    thumb_radius,
                );
            }
        }

        if clipped {
            scene.pop_clip();
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut SegmentedControl,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> SegmentedLayout {
        self.arrange(text, model, labels, frame, clip, role, compact);
        if valid(frame, clip) {
            self.paint_arranged(scene, text, model, labels, clip, palette, role, compact);
        }
        self.layout.clone()
    }
    pub fn pointer_move(&mut self, p: Point) {
        if self.disabled {
            self.hovered = None;
            return;
        }
        self.hovered = self.layout.items.iter().position(|l| l.hit_test(p));
        for (i, b) in self.buttons.iter_mut().enumerate() {
            b.pointer_move_at(p);
            if self.pressed == Some(i) {
                b.state.pressed = self.hovered == Some(i)
            }
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<usize> {
        let decision = self.prepare_pointer_down(p, false)?;
        let SegmentedViewEffect::Pointer { index, .. } = &decision.effect else {
            unreachable!()
        };
        let index = *index;
        // Pointer decision has no model effect; use the same consuming primitive.
        self.apply_pointer_decision(decision);
        Some(index)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut SegmentedControl) -> Option<usize> {
        let i = self.pressed.take()?;
        let ok = self.buttons.get_mut(i).is_some_and(|b| b.pointer_up_at(p));
        if ok && self.window_active && !self.disabled && model.select(i) {
            Some(i)
        } else {
            None
        }
    }
    pub fn key_event(
        &mut self,
        key: ControlKey,
        down: bool,
        model: &mut SegmentedControl,
    ) -> ComponentEvent {
        let Some(index) = self
            .focused
            .filter(|_| self.window_active && !self.disabled)
        else {
            return ComponentEvent::default();
        };
        if !model.is_enabled(index) {
            self.cancel();
            return ComponentEvent::default();
        }
        if key == ControlKey::Space {
            if down {
                if self.space_armed.is_none() {
                    self.space_armed = Some(index);
                }
                return ComponentEvent {
                    consumed: true,
                    changed: false,
                };
            }
            let armed = self.space_armed.take() == Some(index);
            let changed = armed && model.select(index);
            return ComponentEvent {
                consumed: true,
                changed,
            };
        }
        if !down {
            return ComponentEvent::default();
        }
        let supported = matches!(
            key,
            ControlKey::Left
                | ControlKey::Right
                | ControlKey::Up
                | ControlKey::Down
                | ControlKey::Home
                | ControlKey::End
        );
        if !supported {
            return ComponentEvent::default();
        }
        self.focus_visible = true;
        let changed = model.key_from(key, index);
        if self.focused != Some(model.selected) {
            self.cancel();
        }
        self.focused = Some(model.selected);
        ComponentEvent {
            consumed: true,
            changed,
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut SegmentedControl) -> bool {
        self.key_event(key, down, model).changed
    }
    pub fn select(&mut self, index: usize, model: &mut SegmentedControl) -> bool {
        let decision = self.prepare_select(index, model);
        let changed = decision.changed_selection();
        self.apply_decision(decision, model);
        changed
    }
    pub fn focus(&mut self, i: usize) {
        self.focus_with_visibility(i, true)
    }
    pub fn focus_with_visibility(&mut self, i: usize, visible: bool) {
        if let Some(decision) = self.prepare_focus(i, visible, false) {
            self.apply_focus_decision(decision);
        }
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        let pointer = self.pressed.take().is_some();
        let keyboard = self.space_armed.take().is_some();
        for b in &mut self.buttons {
            b.cancel();
        }
        pointer || keyboard
    }
    /// Temporarily disable the whole view without changing the controlled
    /// selection or any per-item availability in the model. Disabling cancels
    /// pending pointer/Space activation and clears focus; re-enabling does not
    /// restore an old press or automatically move focus.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.blur();
            self.hovered = None;
        }
    }
    pub fn needs_animation(&self) -> bool {
        self.thumb_x.needs_animation()
            || self.thumb_width.needs_animation()
            || self.buttons.iter().any(ButtonModel::needs_animation)
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
            let tx = self.thumb_x.target();
            let tw = self.thumb_width.target();
            let _ = self.thumb_x.snap(tx, self.motion_now);
            let _ = self.thumb_width.snap(tw, self.motion_now);
        }
    }
    /// The current roving-focus item, independent of controlled selection.
    /// Hosts can project the actual focused part without mirroring its state.
    pub fn focused(&self) -> Option<usize> {
        self.focused
    }
    pub fn layout(&self) -> &SegmentedLayout {
        &self.layout
    }
}
impl Default for SegmentedControlView {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BreadcrumbLayout {
    pub frame: Rect,
    pub items: Vec<ControlFrame>,
}

#[derive(Clone, Debug, Default)]
struct BreadcrumbMeasure {
    items: Vec<Rect>,
    separators: Vec<Rect>,
    single_row: bool,
    size: Size,
}

// Breadcrumb owns its path flow, not a general wrapping container. Item and
// passive leading separator positions are the sole geometry used by rendering.
fn measure_breadcrumb(
    text: &mut impl ButtonTextRenderer,
    labels: &[&str],
    max_width: f32,
    role: proposal::TextRole,
) -> BreadcrumbMeasure {
    let width = if max_width.is_finite() {
        max_width.max(0.)
    } else {
        0.
    };
    if labels.is_empty() || width == 0. {
        return BreadcrumbMeasure {
            items: vec![Rect::default(); labels.len()],
            separators: vec![Rect::default(); labels.len().saturating_sub(1)],
            size: Size::new(width, 0.),
            ..Default::default()
        };
    }
    let scale = text.resolved_font_size(role) / role.style().size;
    let scale = if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    };
    let spacing = proposal::metric("spacing", "space-tight").expect("Breadcrumb spacing") * scale;
    let row_gap = proposal::metric("spacing", "space-regular").expect("Breadcrumb row gap") * scale;
    let slash = text.measure("/", role, width);
    let glyph_width = if slash.width.is_finite() {
        slash.width.max(0.).min(width)
    } else {
        0.
    };
    let natural_gap = glyph_width + 2. * spacing;
    let insets = Some((spacing, spacing));
    let natural: Vec<_> = labels
        .iter()
        .map(|label| {
            ButtonModel::measure_with_insets(text, label, width, ButtonSize::Body, role, insets)
                .width
                .max(0.)
        })
        .collect();
    let total: f64 = natural.iter().map(|w| f64::from(*w)).sum::<f64>()
        + labels.len().saturating_sub(1) as f64 * f64::from(natural_gap);
    let single_row = total <= f64::from(width);
    let mut placements = Vec::with_capacity(labels.len());
    let mut row_heights = vec![0.0_f32];
    let mut row = 0;
    let mut x = 0.;
    for (index, (label, preferred)) in labels.iter().zip(&natural).enumerate() {
        let gap = if index == 0 { 0. } else { natural_gap };
        let starts_new_row = !single_row && x > 0. && x + gap + preferred > width;
        if starts_new_row {
            row += 1;
            row_heights.push(0.);
            x = 0.;
        }
        // An extremely narrow row must still leave a nonnegative item region;
        // a slash without its original focus clearance is not painted.
        let available = (width - x).max(0.);
        let mut gap = if gap >= available {
            available * 0.5
        } else {
            gap
        };
        // A leading passive separator must not turn an otherwise nonempty
        // original Button content slot into an empty one. Empty-label measure
        // supplies the original scaled horizontal insets without new constants.
        // Only this already-wrapped row may omit its separator and its budget;
        // a full row that also has no content keeps the existing narrow fallback.
        if starts_new_row {
            let empty_insets =
                ButtonModel::measure_with_insets(text, "", width, ButtonSize::Body, role, insets)
                    .width;
            if width > empty_insets && (available - gap).max(0.) <= empty_insets {
                gap = 0.;
            }
        }
        let item_x = x + gap;
        let item_width = if single_row && index + 1 == labels.len() {
            (width - item_x).max(0.)
        } else {
            preferred.min((width - item_x).max(0.))
        };
        let height = ButtonModel::measure_with_insets(
            text,
            label,
            item_width,
            ButtonSize::Body,
            role,
            insets,
        )
        .height
        .max(0.);
        row_heights[row] = row_heights[row].max(height);
        placements.push((row, item_x, item_width, x, gap));
        x = item_x + item_width;
    }
    let mut y = 0.;
    let row_tops: Vec<_> = row_heights
        .iter()
        .map(|height| {
            let top = y;
            y += height + row_gap;
            top
        })
        .collect();
    let height = (y - row_gap).max(0.);
    let mut items = Vec::with_capacity(labels.len());
    let mut separators = Vec::with_capacity(labels.len().saturating_sub(1));
    for (index, (row, x, width, separator_x, separator_width)) in placements.into_iter().enumerate()
    {
        let row_height = row_heights[row];
        items.push(Rect::new(x, row_tops[row], width, row_height));
        if index > 0 {
            let fits = separator_width >= glyph_width + spacing;
            let glyph_height = slash.height.max(0.).min(row_height);
            separators.push(if fits {
                Rect::new(
                    separator_x + (separator_width - glyph_width) * 0.5,
                    row_tops[row] + (row_height - glyph_height) * 0.5,
                    glyph_width,
                    glyph_height,
                )
            } else {
                Rect::default()
            });
        }
    }
    BreadcrumbMeasure {
        items,
        separators,
        single_row,
        size: Size::new(width, height),
    }
}

#[derive(Clone, Debug)]
pub struct BreadcrumbView {
    pub(super) layout: BreadcrumbLayout,
    pub(super) buttons: Vec<ButtonModel>,
    pub(super) focused: Option<usize>,
    pub(super) focus_visible: bool,
    pub(super) hovered: Option<usize>,
    pub(super) pressed: Option<usize>,
    pub(super) window_active: bool,
    pub(super) disabled: bool,
    pub(super) surface: &'static str,
    pub(super) rendered_path: Vec<String>,
}
impl BreadcrumbView {
    /// Measure the multi-line breadcrumb at an available width. The returned
    /// height is the same geometry used by `render`; callers can allocate it
    /// before painting while existing fixed-frame callers remain compatible.
    pub fn measure(
        &self,
        text: &mut impl ButtonTextRenderer,
        model: &Breadcrumb,
        labels: &[&str],
        max_width: f32,
        role: proposal::TextRole,
    ) -> Size {
        let count = model.items.len().min(labels.len());
        measure_breadcrumb(text, &labels[..count], max_width, role).size
    }

    // A positional path cannot transfer an old gesture to different business
    // content. Retain only the original layout's data identity, not input state.
    fn path_is_current(&mut self, model: &Breadcrumb) -> bool {
        if self.rendered_path == model.items {
            return true;
        }
        self.blur();
        self.hovered = None;
        self.layout = BreadcrumbLayout::default();
        false
    }
    /// Set the semantic parent before painting, preserving the original gesture
    /// and current-page policy. Newly added path items inherit the same parent.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        self.surface = surface;
        for button in &mut self.buttons {
            button.set_surface(surface);
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Breadcrumb,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> BreadcrumbLayout {
        if !valid(frame, clip) {
            self.cancel();
            self.focused = None;
            self.hovered = None;
            self.layout = BreadcrumbLayout::default();
            return self.layout.clone();
        }
        if !self.path_is_current(model) {
            self.rendered_path.clone_from(&model.items);
        }
        let n = model.items.len().min(labels.len());
        if self.buttons.len() != n {
            self.cancel();
        }
        if self.focused.is_some_and(|i| i + 1 >= n) {
            self.focused = None;
            self.focus_visible = false;
        }
        if self.hovered.is_some_and(|i| i + 1 >= n) {
            self.hovered = None;
        }
        self.buttons.resize_with(n, ButtonModel::default);
        let measured = measure_breadcrumb(text, &labels[..n], frame.size.width, role);
        let scale = crate::button_typography_scale(text, role);
        let item_inset =
            proposal::metric("spacing", "space-tight").expect("Breadcrumb spacing") * scale;
        let frame = Rect::new(
            frame.min_x(),
            frame.min_y(),
            measured.size.width,
            frame.size.height.max(measured.size.height),
        );
        let clipped = safe_clip(scene, clip);
        let mut items = Vec::new();
        for (i, label) in labels[..n].iter().enumerate() {
            let b = &mut self.buttons[i];
            b.set_surface(self.surface);
            b.set_horizontal_inset_override(Some((item_inset, item_inset)));
            b.state.disabled = self.disabled || i + 1 == n;
            b.state.selected = i + 1 == n;
            b.state.focused = self.window_active && self.focused == Some(i);
            b.state.focus_visible = b.state.focused && self.focus_visible;
            b.state.hovered = self.window_active && self.hovered == Some(i);
            b.state.pressed = self.window_active && self.pressed == Some(i);
            let mut local = measured.items[i];
            if measured.single_row {
                local.size.height = frame.size.height;
            }
            let cell = local.offset(frame.min_x(), frame.min_y());
            let l = b.render(
                scene,
                text,
                label,
                cell,
                clip,
                palette,
                ButtonVariant::Quiet,
                ButtonSize::Body,
                role,
                ButtonTextAlign::Leading,
            );
            items.push(ControlFrame::new(l.frame, clip));
            // A cross-row slash is the successor's leading passive region.
            // It has no ButtonModel, hit rectangle, focus or AX identity.
            if i > 0 {
                let mut glyph = measured.separators[i - 1];
                if measured.single_row {
                    glyph.origin.y = (frame.size.height - glyph.size.height) * 0.5;
                }
                if !glyph.is_empty() {
                    text.draw(
                        scene,
                        "/",
                        role,
                        glyph.offset(frame.min_x(), frame.min_y()),
                        ButtonTextAlign::Center,
                        tekes_ui_tokens::surface_palette::resolve_surface(
                            palette,
                            self.surface,
                            "data-category-label",
                            "rest",
                        )
                        .expect("validated Breadcrumb parent secondary label")
                        .text,
                    );
                }
            }
        }
        if clipped {
            scene.pop_clip();
        }
        self.layout = BreadcrumbLayout { frame, items };
        if !self
            .layout
            .items
            .iter()
            .any(|item| item.visible_frame().is_some_and(has_area))
            || self.focused.is_some_and(|index| {
                self.layout
                    .items
                    .get(index)
                    .is_none_or(|item| !item.visible_frame().is_some_and(has_area))
            })
        {
            self.cancel();
            self.focused = None;
            self.hovered = None;
        }
        self.layout.clone()
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.hovered = self
            .window_active
            .then(|| self.layout.items.iter().position(|l| l.contains(p)))
            .flatten();
        for b in &mut self.buttons {
            b.pointer_move_at(p);
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<usize> {
        if !self.window_active || self.disabled {
            return None;
        }
        let i = self.layout.items.iter().position(|l| l.contains(p))?;
        if i + 1 == self.layout.items.len() || self.buttons[i].state.disabled {
            return None;
        }
        self.buttons[i].pointer_down_at(p);
        self.pressed = Some(i);
        self.focused = Some(i);
        self.focus_visible = false;
        Some(i)
    }
    pub fn pointer_up(&mut self, p: Point, model: &Breadcrumb) -> Option<usize> {
        if !self.path_is_current(model) {
            return None;
        }
        let i = self.pressed.take()?;
        if self.window_active
            && !self.disabled
            && self
                .buttons
                .get_mut(i)
                .is_some_and(|button| button.pointer_up_at(p))
        {
            model.activate(i)
        } else {
            None
        }
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &Breadcrumb,
    ) -> (bool, Option<usize>) {
        if !self.path_is_current(model) {
            return (false, None);
        }
        let Some(i) = self
            .focused
            .filter(|_| self.window_active && !self.disabled)
        else {
            return (false, None);
        };
        // Breadcrumb ancestors are links; Space stays available for page scroll.
        if k == ControlKey::Enter {
            if down && k == ControlKey::Enter {
                return (true, model.activate(i));
            }
            return (true, None);
        }
        (false, None)
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &Breadcrumb) -> Option<usize> {
        self.key_event(k, down, model).1
    }
    pub fn activate(&mut self, index: usize, model: &Breadcrumb) -> Option<usize> {
        if !self.path_is_current(model)
            || self.disabled
            || !self.window_active
            || index >= self.layout.items.len().saturating_sub(1)
            || !self
                .layout
                .items
                .get(index)
                .is_some_and(|item| item.visible_frame().is_some_and(has_area))
        {
            return None;
        }
        let index = model.activate(index)?;
        self.focused = Some(index);
        self.focus_visible = false;
        Some(index)
    }
    pub fn focus(&mut self, i: usize) {
        self.focus_with_visibility(i, true)
    }
    pub fn focus_with_visibility(&mut self, i: usize, visible: bool) {
        if !self.disabled
            && i < self.buttons.len().saturating_sub(1)
            && !self.buttons[i].state.disabled
        {
            self.focused = Some(i);
            self.focus_visible = visible;
        }
    }
    /// Original focused ancestor, shared by composition semantics and focus traversal.
    pub fn focused_index(&self) -> Option<usize> {
        self.focused
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        let had = self.pressed.take().is_some();
        for b in &mut self.buttons {
            b.cancel();
        }
        had
    }
    /// Disable all navigable ancestors without changing the breadcrumb path.
    /// The current page remains noninteractive regardless of this gate.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.blur();
            self.hovered = None;
        }
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
        }
    }
    pub fn layout(&self) -> &BreadcrumbLayout {
        &self.layout
    }
}
impl Default for BreadcrumbView {
    fn default() -> Self {
        Self {
            layout: BreadcrumbLayout::default(),
            buttons: Vec::new(),
            focused: None,
            focus_visible: false,
            hovered: None,
            pressed: None,
            window_active: true,
            disabled: false,
            surface: "canvas",
            rendered_path: Vec::new(),
        }
    }
}

#[cfg(test)]
mod breadcrumb_separator_tests {
    use super::*;

    #[test]
    fn breadcrumb_uses_muted_ink_for_separators() {
        struct Text(Vec<(String, tekes_ui_tokens::Rgba)>);
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
                Size::new(
                    (value.chars().count() as f32 * 8.).min(width),
                    role.style().line_height,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                value: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                ink: tekes_ui_tokens::Rgba,
            ) {
                self.0.push((value.to_owned(), ink));
            }
        }
        let palette = Palette::new(tekes_ui_tokens::Theme::LIGHT, 0).expect("valid palette");
        let model = crate::extended::Breadcrumb {
            items: vec!["Home".into(), "Library".into(), "Current".into()],
        };
        let mut view = BreadcrumbView::default();
        let mut text = Text(Vec::new());
        view.render(
            &mut Scene::new(),
            &mut text,
            &model,
            &["Home", "Library", "Current"],
            Rect::new(0., 0., 400., 40.),
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        let expected_muted = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            "canvas",
            "data-category-label",
            "rest",
        )
        .unwrap()
        .text;
        let slashes: Vec<_> = text.0.iter().filter(|(s, _)| s == "/").collect();
        assert!(!slashes.is_empty());
        assert!(slashes.iter().all(|(_, ink)| *ink == expected_muted));
    }
}
