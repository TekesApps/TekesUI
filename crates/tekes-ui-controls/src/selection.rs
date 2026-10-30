//! Retained interaction and presentation runtimes for the existing selection controls.
//!
//! Gallery and consumers provide text shaping and the semantic model. These
//! runtimes own normalized/clipped geometry, paint, pointer capture, focus, and
//! keyboard activation without changing the model structs or their literals.
use crate::control_motion::{ProgressTransition, motion_for};
use crate::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext, ControlState,
    extended::{
        ControlFrame, RadioGroup, Slider, Switch, radio_on_surface, slider, switch_with_state,
        switch_with_state_and_thumb_progress,
    },
};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

fn invalid_rect(rect: Rect) -> bool {
    !rect.origin.x.is_finite() || !rect.origin.y.is_finite()
}
fn invalid_clip(clip: Option<Rect>) -> bool {
    clip.is_some_and(invalid_rect)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub track: Rect,
    pub thumb: Rect,
}

#[derive(Clone, Debug)]
pub struct SliderView {
    layout: SliderLayout,
    hovered: bool,
    focused: bool,
    focus_visible: bool,
    captured: bool,
    window_active: bool,
}
impl SliderView {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        model: Slider,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> SliderLayout {
        if invalid_rect(frame) || invalid_clip(clip) {
            self.layout = SliderLayout::default();
            self.blur();
            return self.layout;
        }
        if model.disabled {
            self.blur();
        }
        let component = model.layout(frame, clip);
        let normalized = component.frame();
        let visible = component.visible_frame();
        if visible.is_some() {
            // Focus rings extend beyond the thumb. Clip to the ancestor's
            // viewport while keeping pointer hit testing on `visible`.
            if let Some(clip) = clip {
                scene.push_clip(clip);
            }
        }
        let geom = if visible.is_some() {
            slider(
                scene,
                normalized,
                palette,
                model,
                ControlState {
                    hovered: self.hovered,
                    pressed: self.captured,
                    focused: self.focused && self.window_active,
                    focus_visible: self.focus_visible,
                    disabled: model.disabled,
                    ..Default::default()
                },
            )
        } else {
            crate::extended::SliderGeometry {
                track: Rect::default(),
                thumb: Rect::default(),
                thumb_center_x: 0.,
            }
        };
        if visible.is_some() && clip.is_some() {
            scene.pop_clip();
        }
        self.layout = SliderLayout {
            frame: normalized,
            visible,
            track: geom.track,
            thumb: geom.thumb,
        };
        if self.layout.visible.is_none() || model.disabled {
            self.blur();
        }
        self.layout
    }
    pub fn focus(&mut self, focus_visible: bool) {
        self.focused = true;
        self.focus_visible = focus_visible;
    }
    pub fn blur(&mut self) {
        self.focused = false;
        self.focus_visible = false;
        self.hovered = false;
        self.cancel();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.blur();
        }
    }
    /// Semantic value update on the current visible mount. `None` rejects the
    /// request; `Some(false)` consumes a valid request already at its value.
    /// A valid semantic update supersedes an in-flight pointer gesture.
    pub fn set_value(&mut self, value: f64, model: &mut Slider) -> Option<bool> {
        if !value.is_finite() || !self.value_update_available(model) {
            return None;
        }
        self.cancel();
        Some(model.set(value))
    }
    /// Relative semantic update using the original model's step and bounds.
    pub fn adjust_value(&mut self, increase: bool, model: &mut Slider) -> Option<bool> {
        if !self.value_update_available(model) {
            return None;
        }
        self.cancel();
        Some(model.key(if increase {
            ControlKey::Up
        } else {
            ControlKey::Down
        }))
    }
    fn value_update_available(&mut self, model: &Slider) -> bool {
        if model.disabled {
            self.blur();
        }
        !model.disabled && self.window_active && self.layout.visible.is_some()
    }
    pub fn pointer_move(&mut self, point: Point, model: &mut Slider) -> bool {
        if model.disabled {
            let changed = self.captured || self.hovered || self.focused;
            self.blur();
            return changed;
        }

        let hovered = self.window_active
            && !model.disabled
            && self.layout.visible.is_some_and(|r| r.contains(point));
        let mut changed = std::mem::replace(&mut self.hovered, hovered) != hovered;
        if self.captured && (!self.window_active || model.disabled) {
            self.cancel();
        }
        if self.captured && self.window_active && !model.disabled {
            changed |= model.drag(point.x, self.layout.track);
        }
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &mut Slider) -> bool {
        // Availability observations end the current gesture even before paint.
        if model.disabled {
            self.blur();
            return false;
        }

        if !self.window_active
            || model.disabled
            || !self.layout.visible.is_some_and(|r| r.contains(point))
        {
            return false;
        }
        self.captured = true;
        self.focused = true;
        self.focus_visible = false;
        model.drag(point.x, self.layout.track);
        true
    }
    pub fn pointer_up(&mut self, _point: Point, model: &Slider) -> bool {
        // Availability observations end the current gesture even before paint.
        if model.disabled {
            self.blur();
            return false;
        }

        let captured = std::mem::replace(&mut self.captured, false);
        captured && self.window_active && !model.disabled
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut Slider) -> bool {
        // Availability observations end the current gesture even before paint.
        if model.disabled {
            self.blur();
            return false;
        }

        if !self.focused || !self.window_active {
            return false;
        }
        if !down {
            return matches!(
                key,
                ControlKey::Left
                    | ControlKey::Right
                    | ControlKey::Up
                    | ControlKey::Down
                    | ControlKey::Home
                    | ControlKey::End
            );
        }
        if matches!(
            key,
            ControlKey::Left
                | ControlKey::Right
                | ControlKey::Up
                | ControlKey::Down
                | ControlKey::Home
                | ControlKey::End
        ) {
            self.focus_visible = true;
            model.key(key)
        } else {
            false
        }
    }
    pub fn cancel(&mut self) -> bool {
        std::mem::replace(&mut self.captured, false)
    }
    pub fn layout(&self) -> SliderLayout {
        self.layout
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn is_captured(&self) -> bool {
        self.captured
    }
}

impl Default for SliderView {
    fn default() -> Self {
        Self {
            layout: SliderLayout::default(),
            hovered: false,
            focused: false,
            focus_visible: false,
            captured: false,
            window_active: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadioGroupLayout {
    pub frame: Rect,
    pub rows: Vec<ControlFrame>,
    pub indicators: Vec<Rect>,
    pub labels: Vec<Rect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionEvent {
    #[default]
    Ignored,
    Handled,
    Changed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwitchLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub toggle: ControlFrame,
    pub label: Rect,
}

#[derive(Clone, Debug)]
pub struct RadioGroupView {
    surface: &'static str,
    layout: RadioGroupLayout,
    focused: Option<usize>,
    focus_visible: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    window_active: bool,
    disabled: bool,
    error: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RadioActionState {
    focused: Option<usize>,
    focus_visible: bool,
    pressed: Option<usize>,
    window_active: bool,
    disabled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RadioViewDecision {
    before: RadioActionState,
    next: RadioActionState,
    pub(crate) event: SelectionEvent,
    write_focus: bool,
    write_pressed: bool,
}
impl RadioGroupView {
    /// Surface identity affects paint only, retaining original focus and armed gestures.
    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)?;
        Ok(())
    }

    /// Validation is independent of selection, focus and disabled interaction.
    /// The host supplies the explanatory message (for example through Field).
    pub fn set_error(&mut self, error: bool) {
        self.error = error;
    }
    pub fn error(&self) -> bool {
        self.error
    }
    pub fn new() -> Self {
        Self::default()
    }
    fn measured_layout(
        text: &mut impl ButtonTextRenderer,
        model: &RadioGroup,
        bounds: Rect,
        clip: Option<Rect>,
        labels: &[&str],
    ) -> RadioGroupLayout {
        if invalid_rect(bounds) || invalid_clip(clip) {
            return RadioGroupLayout::default();
        }
        let normalized = ControlFrame::new(bounds, clip).frame();
        let minimum = proposal::metric("size", "size-comfortable-row")
            .expect("design system comfortable row");
        let row_gap = 6.0;
        let label_width = (normalized.size.width - 40.).max(0.);
        let labels_metrics: Vec<_> = (0..model.len())
            .map(|index| {
                let label = labels.get(index).copied().unwrap_or("");
                let measured = text
                    .measure(label, proposal::TextRole::UiBody, label_width)
                    .height;
                let first_line =
                    text.first_line_height(label, proposal::TextRole::UiBody, label_width);
                let measured = measured.max(first_line).max(0.0);
                (minimum.max(measured + 8.), measured, first_line.max(0.0))
            })
            .collect();
        let mut total = 0.;
        let rows: Vec<_> = labels_metrics
            .iter()
            .map(|(height, _, _)| *height)
            .scan(normalized.min_y(), |y, height| {
                let row = ControlFrame::new(
                    Rect::new(normalized.min_x(), *y, normalized.size.width, height),
                    clip,
                );
                total += height;
                *y += height + row_gap;
                Some(row)
            })
            .collect();
        let frame = Rect::new(
            normalized.min_x(),
            normalized.min_y(),
            normalized.size.width,
            total + row_gap * rows.len().saturating_sub(1) as f32,
        );
        let mut indicators = Vec::with_capacity(rows.len());
        let mut label_slots = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let r = row.frame();
            let (_, measured, first_line) = labels_metrics[index];
            // Text is vertically centered as a paragraph in its row. Align
            // the control to the first line box, not the paragraph midpoint.
            let first_line_center = r.min_y() + (r.size.height - measured) * 0.5 + first_line * 0.5;
            let indicator = Rect::new(r.min_x() + 8., first_line_center - 9., 18., 18.);
            let label = Rect::new(
                r.min_x() + 36.,
                r.min_y(),
                (r.size.width - 40.).max(0.),
                r.size.height,
            );
            indicators.push(indicator);
            label_slots.push(label);
        }
        RadioGroupLayout {
            frame,
            rows,
            indicators,
            labels: label_slots,
        }
    }
    pub(crate) fn measure_content(
        text: &mut impl ButtonTextRenderer,
        model: &RadioGroup,
        width: f32,
        labels: &[&str],
    ) -> Size {
        Self::measured_layout(text, model, Rect::new(0., 0., width, 0.), None, labels)
            .frame
            .size
    }
    pub(crate) fn arrange_content(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &RadioGroup,
        bounds: Rect,
        clip: Option<Rect>,
        labels: &[&str],
    ) -> RadioGroupLayout {
        self.layout = Self::measured_layout(text, model, bounds, clip, labels);
        if self
            .focused
            .is_some_and(|i| i >= model.len() || !model.is_enabled(i))
        {
            self.focused = model
                .selected
                .filter(|i| model.is_enabled(*i))
                .or_else(|| (0..model.len()).find(|i| model.is_enabled(*i)));
        }
        let all_hidden = self
            .layout
            .rows
            .iter()
            .all(|row| row.visible_frame().is_none());
        if self.disabled || self.pressed.is_some_and(|index| !model.is_enabled(index)) {
            self.cancel();
        }
        if all_hidden {
            self.blur();
        }
        if all_hidden
            || self
                .pressed
                .is_some_and(|index| rows_visibility_for(&self.layout, index).is_none())
        {
            self.cancel();
        }
        self.layout.clone()
    }
    /// Paint only the original arranged geometry, without a second reconciliation.
    pub(crate) fn paint_content(
        &self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &RadioGroup,
        clip: Option<Rect>,
        palette: Palette,
        labels: &[&str],
    ) {
        for (index, row) in self.layout.rows.iter().enumerate() {
            let indicator = self.layout.indicators[index];
            let label = self.layout.labels[index];
            if row.visible_frame().is_some() {
                if let Some(clip) = clip {
                    scene.push_clip(clip);
                }
                let focused = self.focused == Some(index) && self.window_active && !self.disabled;
                let state = ControlState {
                    error: self.error,
                    selected: model.selected == Some(index),
                    disabled: self.disabled || !model.is_enabled(index),
                    hovered: self.hovered == Some(index),
                    pressed: self.pressed == Some(index),
                    focused,
                    focus_visible: focused && self.focus_visible,
                    ..Default::default()
                };
                radio_on_surface(scene, indicator, palette, state, self.surface);
                let label_visible = ControlFrame::new(label, row.visible_frame()).visible_frame();
                if let Some(label_clip) = label_visible {
                    scene.push_clip(label_clip);
                    text.draw(
                        scene,
                        labels.get(index).copied().unwrap_or(""),
                        proposal::TextRole::UiBody,
                        label,
                        ButtonTextAlign::Leading,
                        if state.disabled {
                            palette
                                .color("disabled-text")
                                .expect("Radio disabled label role")
                        } else {
                            crate::paint::choice_label_color(palette, self.surface, state)
                        },
                    );
                    scene.pop_clip();
                }
                if clip.is_some() {
                    scene.pop_clip();
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &RadioGroup,
        bounds: Rect,
        clip: Option<Rect>,
        palette: Palette,
        labels: &[&str],
    ) -> RadioGroupLayout {
        self.arrange_content(text, model, bounds, clip, labels);
        self.paint_content(scene, text, model, clip, palette, labels);
        self.layout.clone()
    }
    pub(crate) fn clear_content_layout(&mut self) {
        self.blur();
        self.layout = RadioGroupLayout::default();
    }
    fn action_state(&self) -> RadioActionState {
        RadioActionState {
            focused: self.focused,
            focus_visible: self.focus_visible,
            pressed: self.pressed,
            window_active: self.window_active,
            disabled: self.disabled,
        }
    }
    pub(crate) fn prepare_content_focus(
        &self,
        model: &RadioGroup,
        index: Option<usize>,
        visible: bool,
    ) -> RadioViewDecision {
        let before = self.action_state();
        let mut next = before;
        next.focused = if self.disabled {
            None
        } else {
            index
                .filter(|i| *i < model.len() && model.is_enabled(*i))
                .or(model.selected.filter(|i| model.is_enabled(*i)))
                .or_else(|| (0..model.len()).find(|i| model.is_enabled(*i)))
        };
        let write_pressed = next.focused != before.focused || self.disabled;
        if write_pressed {
            next.pressed = None;
        }
        next.focus_visible = next.focused.is_some() && visible;
        RadioViewDecision {
            before,
            next,
            event: SelectionEvent::Handled,
            write_focus: true,
            write_pressed,
        }
    }
    pub(crate) fn prepare_content_down(
        &self,
        point: Point,
        model: &RadioGroup,
    ) -> RadioViewDecision {
        let before = self.action_state();
        let mut next = before;
        let mut write_focus = false;
        let mut write_pressed = false;
        if next.pressed.is_some_and(|i| !model.is_enabled(i)) {
            next.pressed = None;
            write_pressed = true;
        }
        let event = if !self.window_active {
            SelectionEvent::Ignored
        } else if let Some(i) = self.layout.rows.iter().position(|r| r.contains(point)) {
            if !self.disabled && model.is_enabled(i) {
                next.pressed = Some(i);
                next.focused = Some(i);
                next.focus_visible = false;
                write_focus = true;
                write_pressed = true;
            }
            SelectionEvent::Handled
        } else {
            SelectionEvent::Ignored
        };
        RadioViewDecision {
            before,
            next,
            event,
            write_focus,
            write_pressed,
        }
    }
    pub(crate) fn prepare_content_press(
        &self,
        model: &RadioGroup,
        index: usize,
    ) -> RadioViewDecision {
        let mut decision = self.prepare_content_focus(model, Some(index), true);
        decision.next.pressed = None;
        decision.write_pressed = true;
        decision
    }
    pub(crate) fn apply_content_action(&mut self, decision: RadioViewDecision) {
        // Final validation precedes Host focus hooks. Consume the original next
        // state; a hook may have focused the owner, but cannot invalidate it.
        // Preserve hook writes for fields the original event did not write.
        if decision.write_focus {
            self.focused = decision.next.focused;
            self.focus_visible = decision.next.focus_visible;
        }
        if decision.write_pressed {
            self.pressed = decision.next.pressed;
        }
    }
    pub fn focus(&mut self, model: &RadioGroup, index: Option<usize>, focus_visible: bool) {
        let decision = self.prepare_content_focus(model, index, focus_visible);
        if self.disabled {
            self.hovered = None;
        }
        self.apply_content_action(decision);
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.hovered = None;
        self.cancel();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.blur();
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.cancel();
            self.focus_visible = false;
            self.focused = None;
        }
    }
    pub fn pointer_move(&mut self, point: Point) -> bool {
        let next = (self.window_active && !self.disabled)
            .then(|| self.layout.rows.iter().position(|row| row.contains(point)))
            .flatten();
        let changed = self.hovered != next;
        self.hovered = next;
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &RadioGroup) -> SelectionEvent {
        let decision = self.prepare_content_down(point, model);
        let event = decision.event;
        self.apply_content_action(decision);
        event
    }
    pub fn pointer_up(&mut self, point: Point, model: &mut RadioGroup) -> SelectionEvent {
        let Some(index) = self.pressed.take() else {
            return SelectionEvent::Ignored;
        };
        if self.disabled || !self.window_active {
            return SelectionEvent::Handled;
        }
        if self
            .layout
            .rows
            .get(index)
            .is_some_and(|row| row.contains(point))
        {
            if model.select(index) {
                SelectionEvent::Changed
            } else {
                SelectionEvent::Handled
            }
        } else {
            SelectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut RadioGroup) -> SelectionEvent {
        // A disabled or removed captured row cannot revive on a later release.
        if self.pressed.is_some_and(|index| !model.is_enabled(index)) {
            self.cancel();
        }

        if !self.window_active || self.focused.is_none() {
            return SelectionEvent::Ignored;
        }
        if self.disabled {
            return if matches!(
                key,
                ControlKey::Left
                    | ControlKey::Right
                    | ControlKey::Up
                    | ControlKey::Down
                    | ControlKey::Home
                    | ControlKey::End
                    | ControlKey::Space
            ) {
                SelectionEvent::Handled
            } else {
                SelectionEvent::Ignored
            };
        }
        let navigation = matches!(
            key,
            ControlKey::Left
                | ControlKey::Right
                | ControlKey::Up
                | ControlKey::Down
                | ControlKey::Home
                | ControlKey::End
        );
        if !down {
            return if navigation || key == ControlKey::Space {
                SelectionEvent::Handled
            } else {
                SelectionEvent::Ignored
            };
        }
        if navigation {
            self.focus_visible = true;
            let previous = model.selected;
            model.key_from(key, self.focused);
            let changed = model.selected != previous;
            if self.focused != model.selected {
                self.cancel();
            }
            self.focused = model.selected;
            return if changed {
                SelectionEvent::Changed
            } else {
                SelectionEvent::Handled
            };
        }
        if key == ControlKey::Space {
            self.focus_visible = true;
            return if self.focused.is_some_and(|i| model.select(i)) {
                SelectionEvent::Changed
            } else {
                SelectionEvent::Handled
            };
        }
        SelectionEvent::Ignored
    }
    pub fn cancel(&mut self) -> bool {
        self.pressed.take().is_some()
    }
    pub fn focused(&self) -> Option<usize> {
        self.focused
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn layout(&self) -> &RadioGroupLayout {
        &self.layout
    }
}

impl Default for RadioGroupView {
    fn default() -> Self {
        Self {
            surface: "canvas",
            layout: RadioGroupLayout::default(),
            focused: None,
            focus_visible: false,
            hovered: None,
            pressed: None,
            window_active: true,
            disabled: false,
            error: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SwitchView {
    error: bool,
    placement: ControlFrame,
    hovered: bool,
    focused: bool,
    focus_visible: bool,
    pointer_armed: bool,
    space_armed: bool,
    window_active: bool,
    thumb_transition: ProgressTransition,
    motion_palette: Option<(
        tekes_ui_tokens::Theme,
        tekes_ui_tokens::palette::PaletteSource,
    )>,
    motion_initialized: bool,
    motion_now: Duration,
    motion_suspended: bool,
}
impl SwitchView {
    /// Host validation does not disable toggling or erase the current value.
    pub fn set_error(&mut self, error: bool) {
        self.error = error;
    }
    pub fn error(&self) -> bool {
        self.error
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ControlFrame {
        self.render_with_motion_context(scene, model, frame, clip, palette, None)
    }

    /// Render with explicit host-clock motion; model and hit geometry remain immediate.
    #[allow(clippy::too_many_arguments)]
    pub fn render_animated(
        &mut self,
        scene: &mut Scene,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        now: Duration,
        reduced_motion: bool,
    ) -> ControlFrame {
        self.render_with_motion_context(
            scene,
            model,
            frame,
            clip,
            palette,
            Some(ControlMotionContext {
                now,
                reduced_motion,
            }),
        )
    }

    fn render_with_motion_context(
        &mut self,
        scene: &mut Scene,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        context: Option<ControlMotionContext>,
    ) -> ControlFrame {
        if invalid_rect(frame) || invalid_clip(clip) {
            self.placement = ControlFrame::default();
            self.blur();
            let _ = self
                .thumb_transition
                .snap(if model.checked { 1.0 } else { 0.0 }, self.motion_now);
            return self.placement;
        }
        if model.disabled {
            self.blur();
        }
        self.placement = model.layout(frame, clip);
        if self.placement.visible_frame().is_some() {
            if let Some(clip) = clip {
                scene.push_clip(clip);
            }
            let state = ControlState {
                error: self.error,
                selected: model.checked,
                disabled: model.disabled,
                hovered: self.hovered,
                pressed: (self.pointer_armed && self.hovered) || self.space_armed,
                focused: self.focused && self.window_active,
                focus_visible: self.focus_visible,
                ..Default::default()
            };
            if let Some(context) = context {
                let progress =
                    self.prepare_thumb_progress(model.checked, model.disabled, palette, context);
                switch_with_state_and_thumb_progress(
                    scene,
                    self.placement.frame(),
                    palette,
                    state,
                    progress,
                );
            } else {
                let _ = self
                    .thumb_transition
                    .snap(if model.checked { 1.0 } else { 0.0 }, self.motion_now);
                switch_with_state(scene, self.placement.frame(), palette, state);
            }
            if clip.is_some() {
                scene.pop_clip();
            }
        }
        if self.placement.visible_frame().is_none() || model.disabled {
            if self.placement.visible_frame().is_none() {
                self.blur();
                let _ = self
                    .thumb_transition
                    .snap(f64::from(model.checked as u8), self.motion_now);
            } else {
                self.cancel();
            }
        }
        self.placement
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_labeled(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        label_text: &str,
    ) -> SwitchLayout {
        let context = text.motion_context();
        self.render_labeled_with_motion_context(
            scene, text, model, frame, clip, palette, label_text, context,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_labeled_animated(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        label_text: &str,
        now: Duration,
        reduced_motion: bool,
    ) -> SwitchLayout {
        self.render_labeled_with_motion_context(
            scene,
            text,
            model,
            frame,
            clip,
            palette,
            label_text,
            Some(ControlMotionContext {
                now,
                reduced_motion,
            }),
        )
    }

    fn prepare_thumb_progress(
        &mut self,
        checked: bool,
        disabled: bool,
        palette: Palette,
        context: ControlMotionContext,
    ) -> f32 {
        self.motion_now = context.now;
        let palette_key = (palette.theme, palette.source());
        let palette_changed = self
            .motion_palette
            .is_some_and(|previous| previous != palette_key);
        self.motion_palette = Some(palette_key);
        let target = if checked { 1.0 } else { 0.0 };
        let initialize = !self.motion_initialized;
        self.motion_initialized = true;
        let value = if initialize || palette_changed || disabled || self.motion_suspended {
            self.thumb_transition.snap(target, context.now)
        } else {
            self.thumb_transition.retarget(
                target,
                motion_for("switch_thumb"),
                context.now,
                context.reduced_motion,
            )
        }
        .unwrap_or_else(|_| self.thumb_transition.value());
        value.clamp(0.0, 1.0) as f32
    }

    pub fn needs_animation(&self) -> bool {
        self.thumb_transition.needs_animation()
    }

    /// Advance the thumb presentation using a host monotonic timestamp.
    pub fn tick(&mut self, now: Duration) -> bool {
        self.motion_now = now;
        let before = self.thumb_transition.value();
        let _ = self.thumb_transition.advance(now);
        before != self.thumb_transition.value()
    }

    /// Apply the host reduced-motion preference immediately.
    pub fn set_reduced_motion(&mut self, reduced: bool, now: Duration, model: &Switch) -> bool {
        self.motion_now = now;
        let before = self.thumb_transition.value();
        let _ = self.thumb_transition.set_reduced_motion(reduced, now);
        if reduced {
            let _ = self
                .thumb_transition
                .snap(if model.checked { 1.0 } else { 0.0 }, now);
        }
        before != self.thumb_transition.value()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_labeled_with_motion_context(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: Switch,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        label_text: &str,
        context: Option<ControlMotionContext>,
    ) -> SwitchLayout {
        if invalid_rect(frame) || invalid_clip(clip) {
            self.placement = ControlFrame::default();
            self.blur();
            let _ = self
                .thumb_transition
                .snap(if model.checked { 1.0 } else { 0.0 }, self.motion_now);
            return SwitchLayout::default();
        }
        if model.disabled {
            self.blur();
        }
        let label_width = (frame.size.width - 62.0).max(0.0);
        let measured = text
            .measure(label_text, proposal::TextRole::UiBody, label_width)
            .height;
        let first_line =
            text.first_line_height(label_text, proposal::TextRole::UiBody, label_width);
        let row_height = 32.0_f32.max(measured.max(first_line).max(0.0) + 8.0);
        let outer = ControlFrame::new(
            Rect::new(frame.min_x(), frame.min_y(), frame.size.width, row_height),
            clip,
        );
        let normalized = outer.frame();
        let first_line_center = normalized.min_y()
            + (normalized.size.height - measured.max(first_line).max(0.0)) * 0.5
            + first_line.max(0.0) * 0.5;
        let toggle_frame = Rect::new(
            normalized.min_x(),
            first_line_center + 1.0 - 13.0,
            48.0,
            26.0,
        );
        let paint_clip = match (clip, outer.visible_frame()) {
            (_, None) => Some(Rect::default()),
            (Some(clip), Some(_)) => Some(clip),
            (None, Some(_)) => None,
        };
        self.placement = ControlFrame::new(toggle_frame, paint_clip);
        if self.placement.visible_frame().is_some() {
            if let Some(clip) = clip {
                scene.push_clip(clip);
            }
            let state = ControlState {
                error: self.error,
                selected: model.checked,
                disabled: model.disabled,
                hovered: self.hovered,
                pressed: (self.pointer_armed && self.hovered) || self.space_armed,
                focused: self.focused && self.window_active,
                focus_visible: self.focus_visible,
                ..Default::default()
            };
            if let Some(context) = context {
                let progress =
                    self.prepare_thumb_progress(model.checked, model.disabled, palette, context);
                switch_with_state_and_thumb_progress(
                    scene,
                    self.placement.frame(),
                    palette,
                    state,
                    progress,
                );
                if self.needs_animation() {
                    text.request_animation_frame();
                }
            } else {
                let _ = self
                    .thumb_transition
                    .snap(if model.checked { 1.0 } else { 0.0 }, self.motion_now);
                switch_with_state(scene, self.placement.frame(), palette, state);
            }
            if clip.is_some() {
                scene.pop_clip();
            }
        }
        let label = Rect::new(
            normalized.min_x() + 62.0,
            normalized.min_y(),
            (normalized.size.width - 62.0).max(0.0),
            normalized.size.height,
        );
        let label_visible = outer
            .visible_frame()
            .and_then(|visible| ControlFrame::new(label, Some(visible)).visible_frame());
        if let Some(label_clip) = label_visible {
            scene.push_clip(label_clip);
            text.draw(
                scene,
                label_text,
                proposal::TextRole::UiBody,
                label,
                ButtonTextAlign::Leading,
                palette
                    .color(if model.disabled {
                        "disabled-text"
                    } else {
                        "primary-text"
                    })
                    .expect("Design System label text"),
            );
            scene.pop_clip();
        }
        if outer.visible_frame().is_none()
            || self.placement.visible_frame().is_none()
            || model.disabled
        {
            if outer.visible_frame().is_none() || self.placement.visible_frame().is_none() {
                self.blur();
                let _ = self
                    .thumb_transition
                    .snap(if model.checked { 1.0 } else { 0.0 }, self.motion_now);
            } else {
                self.cancel();
            }
        }
        SwitchLayout {
            frame: normalized,
            visible: outer.visible_frame(),
            toggle: self.placement,
            label,
        }
    }
    pub fn focus(&mut self, focus_visible: bool) {
        self.focused = true;
        self.focus_visible = focus_visible;
    }
    pub fn blur(&mut self) {
        self.focused = false;
        self.focus_visible = false;
        self.cancel();
        self.hovered = false;
        let _ = self
            .thumb_transition
            .snap(self.thumb_transition.target(), self.motion_now);
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        self.motion_suspended = !active;
        if !active {
            self.blur();
        }
    }
    pub fn pointer_move(&mut self, point: Point) -> bool {
        self.pointer_move_in_region(point, true)
    }
    pub(crate) fn pointer_move_in_region(&mut self, point: Point, inside: bool) -> bool {
        let next = inside && self.window_active && self.placement.contains(point);
        let changed = self.hovered != next;
        self.hovered = next;
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &Switch) -> bool {
        if model.disabled {
            self.cancel();
            return false;
        }

        if !self.window_active
            || model.disabled
            || self.space_armed
            || !self.placement.contains(point)
        {
            return false;
        }
        self.pointer_armed = true;
        self.hovered = true;
        self.focused = true;
        self.focus_visible = false;
        true
    }
    pub fn pointer_up(&mut self, point: Point, model: &mut Switch) -> SelectionEvent {
        self.pointer_up_in_region(point, model, true)
    }
    pub(crate) fn pointer_up_in_region(
        &mut self,
        point: Point,
        model: &mut Switch,
        inside: bool,
    ) -> SelectionEvent {
        if model.disabled {
            return if self.cancel() {
                SelectionEvent::Handled
            } else {
                SelectionEvent::Ignored
            };
        }

        if !std::mem::replace(&mut self.pointer_armed, false) {
            return SelectionEvent::Ignored;
        }
        if !self.window_active || model.disabled {
            return SelectionEvent::Handled;
        }
        if inside && self.placement.contains(point) && model.toggle() {
            SelectionEvent::Changed
        } else {
            SelectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut Switch) -> SelectionEvent {
        // Availability observations terminate either old activation owner even
        // when this key belongs to a parent. Preserve normal key consumption.
        if model.disabled {
            self.cancel();
            return if self.focused && self.window_active && key == ControlKey::Space {
                SelectionEvent::Handled
            } else {
                SelectionEvent::Ignored
            };
        }
        if !self.focused || !self.window_active || key != ControlKey::Space {
            return SelectionEvent::Ignored;
        }
        if down {
            if self.pointer_armed {
                return SelectionEvent::Handled;
            }
            self.space_armed = true;
            self.focus_visible = true;
            return SelectionEvent::Handled;
        }
        if std::mem::replace(&mut self.space_armed, false) {
            if model.toggle() {
                SelectionEvent::Changed
            } else {
                SelectionEvent::Handled
            }
        } else {
            SelectionEvent::Handled
        }
    }
    pub fn cancel(&mut self) -> bool {
        let was = self.pointer_armed || self.space_armed;
        self.pointer_armed = false;
        self.space_armed = false;
        was
    }
    pub fn layout(&self) -> ControlFrame {
        self.placement
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

fn rows_visibility_for(layout: &RadioGroupLayout, index: usize) -> Option<Rect> {
    layout.rows.get(index).and_then(|row| row.visible_frame())
}

impl Default for SwitchView {
    fn default() -> Self {
        Self {
            error: false,
            placement: ControlFrame::default(),
            hovered: false,
            focused: false,
            focus_visible: false,
            pointer_armed: false,
            space_armed: false,
            window_active: true,
            thumb_transition: ProgressTransition::new(0.0),
            motion_palette: None,
            motion_initialized: false,
            motion_now: Duration::ZERO,
            motion_suspended: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_tokens::Theme;

    #[test]
    fn hidden_or_invalid_selection_layout_releases_keyboard_focus_and_capture() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        for (bounds, clip) in [
            (
                Rect::new(0., 0., 160., 30.),
                Some(Rect::new(500., 500., 1., 1.)),
            ),
            (Rect::new(f32::NAN, 0., 160., 30.), None),
        ] {
            let mut scene = Scene::new();
            let mut text = TestText::default();
            let mut slider = Slider::new(50., 0., 100., 5.).unwrap();
            let mut slider_view = SliderView::new();
            slider_view.render(
                &mut scene,
                slider,
                Rect::new(0., 0., 160., 30.),
                None,
                palette,
            );
            assert!(slider_view.pointer_down(Point::new(80., 15.), &mut slider));
            let before = slider.value();
            slider_view.render(&mut scene, slider, bounds, clip, palette);
            assert!(!slider_view.is_focused());
            assert!(!slider_view.is_captured());
            assert!(!slider_view.key(ControlKey::Right, true, &mut slider));
            assert_eq!(slider.value(), before);

            let mut radio = RadioGroup::new(2);
            let mut radio_view = RadioGroupView::new();
            radio_view.render(
                &mut scene,
                &mut text,
                &radio,
                Rect::new(0., 0., 160., 0.),
                None,
                palette,
                &["One", "Two"],
            );
            radio_view.focus(&radio, Some(0), true);
            radio_view.render(
                &mut scene,
                &mut text,
                &radio,
                bounds,
                clip,
                palette,
                &["One", "Two"],
            );
            assert_eq!(radio_view.focused(), None);
            assert_eq!(
                radio_view.key(ControlKey::Right, true, &mut radio),
                SelectionEvent::Ignored
            );
            assert_eq!(radio.selected, Some(0));
        }
    }

    #[test]
    fn switch_overlapping_pointer_and_space_have_one_activation_owner() {
        let mut scene = Scene::new();
        let mut model = Switch::default();
        let mut view = SwitchView::new();
        view.render(
            &mut scene,
            model,
            Rect::new(0., 0., 48., 26.),
            None,
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
        );
        let inside = Point::new(12., 12.);
        assert!(view.pointer_down(inside, &model));
        view.key(ControlKey::Space, true, &mut model);
        assert_eq!(view.pointer_up(inside, &mut model), SelectionEvent::Changed);
        assert_eq!(
            view.key(ControlKey::Space, false, &mut model),
            SelectionEvent::Handled
        );
        assert!(model.checked);
        view.key(ControlKey::Space, true, &mut model);
        assert!(!view.pointer_down(inside, &model));
        assert_eq!(view.pointer_up(inside, &mut model), SelectionEvent::Ignored);
        assert_eq!(
            view.key(ControlKey::Space, false, &mut model),
            SelectionEvent::Changed
        );
        assert!(!model.checked);
        assert_eq!(
            view.key(ControlKey::Space, false, &mut model),
            SelectionEvent::Handled
        );
    }

    #[test]
    fn radio_disabling_an_armed_row_cancels_even_after_reenable() {
        let mut scene = Scene::new();
        let mut text = TestText::default();
        let mut model = RadioGroup::new(2);
        let mut view = RadioGroupView::new();
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let bounds = Rect::new(0., 0., 160., 0.);
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            bounds,
            None,
            palette,
            &["One", "Two"],
        );
        let inside = layout.rows[1].frame().center();
        assert_eq!(view.pointer_down(inside, &model), SelectionEvent::Handled);
        model.set_enabled(1, false);
        view.render(
            &mut scene,
            &mut text,
            &model,
            bounds,
            None,
            palette,
            &["One", "Two"],
        );
        model.set_enabled(1, true);
        assert_eq!(view.pointer_up(inside, &mut model), SelectionEvent::Ignored);
        assert_eq!(model.selected, Some(0));
    }

    #[test]
    fn slider_view_keeps_finite_thumb_and_bounded_value_for_extreme_ranges_and_steps() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut view = SliderView::new();
        let mut scene = Scene::new();
        let frame = Rect::new(0., 0., 160., 30.);
        let snapped = Slider::new(9.8, 0., 10., 6.).unwrap();
        assert_eq!(snapped.value(), 10.);
        let mut model = Slider::new(0., -f64::MAX, f64::MAX, 1.).unwrap();
        let layout = view.render(&mut scene, model, frame, None, palette);
        assert!(layout.thumb.min_x().is_finite());
        assert_eq!(layout.thumb.center().x, frame.center().x);
        assert!(view.pointer_down(layout.thumb.center(), &mut model));
        view.pointer_move(Point::new(layout.track.max_x(), 15.), &mut model);
        assert_eq!(model.value(), f64::MAX);
        let layout = view.render(&mut scene, model, frame, None, palette);
        assert!(layout.thumb.max_x().is_finite());
        view.pointer_move(Point::new(layout.track.min_x(), 15.), &mut model);
        assert_eq!(model.value(), -f64::MAX);
        let mut large_step = Slider::new(f64::MAX * 0.75, 0., f64::MAX, f64::MAX * 0.75).unwrap();
        view.focus(true);
        assert!(view.key(ControlKey::Right, true, &mut large_step));
        assert_eq!(large_step.value(), f64::MAX);
    }

    #[test]
    fn radio_navigation_starts_at_retained_focus_when_selection_differs() {
        let mut model = RadioGroup::new(4);
        model.set_enabled(3, false);
        let mut view = RadioGroupView::new();
        view.focus(&model, Some(2), true);
        assert_eq!(
            view.key(ControlKey::Right, true, &mut model),
            SelectionEvent::Handled
        );
        assert_eq!(model.selected, Some(0));
        assert_eq!(view.focused(), Some(0));
        assert_eq!(
            view.key(ControlKey::Home, true, &mut model),
            SelectionEvent::Handled
        );
        view.focus(&model, Some(2), true);
        assert_eq!(
            view.key(ControlKey::Left, true, &mut model),
            SelectionEvent::Changed
        );
        assert_eq!(model.selected, Some(1));
        assert_eq!(view.focused(), Some(1));
    }

    #[test]
    fn radio_disabled_surface_view_retains_original_boundary_marker_and_label() {
        for theme in Theme::ALL {
            for brand in [0, 1, 2] {
                let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                for surface in ["canvas", "sidebar", "toolbar"] {
                    for error in [false, true] {
                        let model = RadioGroup::new(1);
                        let mut view = RadioGroupView::new();
                        view.set_surface(surface).unwrap();
                        view.set_disabled(true);
                        view.set_error(error);
                        let mut actual = Scene::new();
                        let mut text = TestText::default();
                        let layout = view.render(
                            &mut actual,
                            &mut text,
                            &model,
                            Rect::new(0., 0., 240., 48.),
                            None,
                            palette,
                            &["Choice"],
                        );
                        // The existing public primitive is an unchanged independent
                        // oracle for Radio's disabled channel roles, not Checkbox's.
                        let mut original = Scene::new();
                        crate::extended::radio_with_state(
                            &mut original,
                            layout.indicators[0],
                            palette,
                            ControlState {
                                selected: true,
                                disabled: true,
                                error,
                                ..Default::default()
                            },
                        );
                        let actual: Vec<_> = actual
                            .primitives()
                            .iter()
                            .filter_map(|p| {
                                if let tekes_ui_scene::Primitive::Quad(q) = p {
                                    Some(q)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let original: Vec<_> = original
                            .primitives()
                            .iter()
                            .filter_map(|p| {
                                if let tekes_ui_scene::Primitive::Quad(q) = p {
                                    Some(q)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        assert_eq!(actual.len(), 2);
                        assert_eq!(original.len(), 2);
                        assert_eq!(
                            actual[0].border_color, original[0].border_color,
                            "{theme:?} {brand} {surface} {error}"
                        );
                        assert_eq!(
                            actual[1].fill, original[1].fill,
                            "{theme:?} {brand} {surface} {error}"
                        );
                        assert_eq!(text.inks, vec![original[1].fill]);
                    }
                }
            }
        }
    }

    #[test]
    fn selection_labels_use_disabled_semantic_ink_for_group_item_and_switch() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut text = TestText::default();
        let mut model = RadioGroup::new(2);
        model.set_enabled(1, false);
        let mut view = RadioGroupView::new();
        let bounds = Rect::new(0., 0., 180., 0.);
        view.render(
            &mut scene,
            &mut text,
            &model,
            bounds,
            None,
            palette,
            &["One", "Two"],
        );
        assert_eq!(
            text.inks,
            vec![
                palette.color("primary-text").unwrap(),
                palette.color("disabled-text").unwrap()
            ]
        );
        text.inks.clear();
        view.set_disabled(true);
        view.render(
            &mut scene,
            &mut text,
            &model,
            bounds,
            None,
            palette,
            &["One", "Two"],
        );
        assert_eq!(text.inks, vec![palette.color("disabled-text").unwrap(); 2]);
        text.inks.clear();
        SwitchView::new().render_labeled(
            &mut scene,
            &mut text,
            Switch {
                checked: true,
                disabled: true,
            },
            Rect::new(0., 100., 180., 32.),
            None,
            palette,
            "Disabled",
        );
        assert_eq!(text.inks, vec![palette.color("disabled-text").unwrap()]);
    }

    #[test]
    fn switch_pointer_leaving_clears_pressed_paint_and_returning_restores_it() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut view = SwitchView::new();
        let mut model = Switch::default();
        let frame = Rect::new(0., 0., 48., 26.);
        view.render(&mut scene, model, frame, None, palette);
        let inside = Point::new(12., 12.);
        assert!(view.pointer_down(inside, &model));
        let track_ink = |scene: &Scene| {
            scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(quad) => Some(quad.fill),
                    _ => None,
                })
                .unwrap()
        };
        scene.clear();
        view.render(&mut scene, model, frame, None, palette);
        assert_eq!(track_ink(&scene), palette.color("button-pressed").unwrap());
        view.pointer_move(Point::new(100., 100.));
        scene.clear();
        view.render(&mut scene, model, frame, None, palette);
        assert_eq!(track_ink(&scene), palette.color("surface-3").unwrap());
        view.pointer_move(inside);
        scene.clear();
        view.render(&mut scene, model, frame, None, palette);
        assert_eq!(track_ink(&scene), palette.color("button-pressed").unwrap());
        assert_eq!(view.pointer_up(inside, &mut model), SelectionEvent::Changed);
    }

    #[test]
    fn slider_owns_clipped_drag_mapping_and_cancels_capture() {
        let mut model = Slider::new(50., 0., 100., 5.).unwrap();
        let mut view = SliderView::new();
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            model,
            Rect::new(10., 10., 100., 30.),
            Some(Rect::new(20., 0., 80., 60.)),
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
        );
        assert_eq!(layout.visible, Some(Rect::new(20., 10., 80., 30.)));
        assert!(view.pointer_down(Point::new(layout.track.center().x, 20.), &mut model));
        view.pointer_move(Point::new(layout.track.max_x() + 20., 20.), &mut model);
        assert_eq!(model.value(), 100.);
        assert!(view.cancel());
        assert!(!view.pointer_up(Point::new(0., 0.), &model));
    }

    #[test]
    fn radio_runtime_owns_roving_focus_pointer_selection_and_disabled_skip() {
        let mut model = RadioGroup::new(3);
        model.set_enabled(1, false);
        let mut view = RadioGroupView::new();
        let mut scene = Scene::new();
        let mut text = TestText::default();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 180., 0.),
            None,
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
            &["One", "Two", "Three"],
        );
        assert_eq!(layout.rows.len(), 3);
        view.focus(&model, Some(0), true);
        assert_eq!(
            view.key(ControlKey::Right, true, &mut model),
            SelectionEvent::Changed
        );
        assert_eq!(model.selected, Some(2));
        assert_eq!(view.focused(), Some(2));
        let point = layout.rows[0].frame().center();
        assert_eq!(view.pointer_down(point, &model), SelectionEvent::Handled);
        assert_eq!(view.pointer_up(point, &mut model), SelectionEvent::Changed);
        assert_eq!(model.selected, Some(0));
    }

    #[test]
    fn switch_activates_on_matching_pointer_release_or_space_release_only() {
        let mut model = Switch::default();
        let mut view = SwitchView::new();
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            model,
            Rect::new(0., 0., 48., 26.),
            None,
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
        );
        assert!(view.pointer_down(Point::new(5., 5.), &model));
        assert_eq!(
            view.pointer_up(Point::new(80., 80.), &mut model),
            SelectionEvent::Handled
        );
        assert!(!model.checked);
        view.focus(true);
        assert_eq!(
            view.key(ControlKey::Space, true, &mut model),
            SelectionEvent::Handled
        );
        assert!(!model.checked);
        assert_eq!(
            view.key(ControlKey::Space, false, &mut model),
            SelectionEvent::Changed
        );
        assert!(model.checked);
    }

    #[test]
    fn whole_radio_group_disable_preserves_model_and_rejects_pointer_and_keys() {
        let mut model = RadioGroup::new(3);
        model.select(2);
        let mut view = RadioGroupView::new();
        let mut text = TestText::default();
        let mut scene = Scene::new();
        view.focus(&model, Some(2), true);
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 200., 0.),
            None,
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
            &["One", "Two", "Three"],
        );
        view.set_disabled(true);
        assert_eq!(
            view.key(ControlKey::Left, true, &mut model),
            SelectionEvent::Ignored
        );
        let point = layout.rows[0].frame().center();
        assert_eq!(view.pointer_down(point, &model), SelectionEvent::Handled);
        assert_eq!(model.selected, Some(2));
        view.set_disabled(false);
        assert_eq!(model.selected, Some(2));
        view.focus(&model, Some(2), false);
        assert_eq!(
            view.key(ControlKey::Right, true, &mut model),
            SelectionEvent::Changed
        );
        assert_eq!(model.selected, Some(0));
    }

    #[test]
    fn radio_focus_change_cancels_a_pointer_arm_before_late_release() {
        let mut model = RadioGroup::new(3);
        let mut view = RadioGroupView::new();
        let mut text = TestText::default();
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 200., 0.),
            None,
            Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand"),
            &["One", "Two", "Three"],
        );
        view.focus(&model, Some(0), false);
        let row0 = layout.rows[0].frame().center();
        assert_eq!(view.pointer_down(row0, &model), SelectionEvent::Handled);
        assert_eq!(
            view.key(ControlKey::Right, true, &mut model),
            SelectionEvent::Changed
        );
        assert_eq!(model.selected, Some(1));
        assert_eq!(view.pointer_up(row0, &mut model), SelectionEvent::Ignored);
        assert_eq!(model.selected, Some(1));

        let row1 = layout.rows[1].frame().center();
        view.pointer_down(row1, &model);
        view.focus(&model, Some(0), true);
        assert_eq!(view.pointer_up(row1, &mut model), SelectionEvent::Ignored);
        assert_eq!(model.selected, Some(1));
    }

    #[test]
    fn default_views_are_active_and_nan_or_complete_clip_never_paints_or_captures() {
        let mut slider = Slider::new(25., 0., 100., 5.).unwrap();
        let mut slider_view = SliderView::default();
        let switch = Switch::default();
        let mut switch_view = SwitchView::default();
        let mut scene = Scene::new();
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        assert_eq!(
            slider_view
                .render(
                    &mut scene,
                    slider,
                    Rect::new(f32::NAN, 2., 90., 32.),
                    None,
                    palette
                )
                .visible,
            None
        );
        assert_eq!(
            switch_view
                .render(
                    &mut scene,
                    switch,
                    Rect::new(0., 0., 48., 26.),
                    Some(Rect::new(80., 80., 10., 10.)),
                    palette
                )
                .visible_frame(),
            None
        );
        assert!(!slider_view.pointer_down(Point::new(5., 5.), &mut slider));
        assert!(!switch_view.pointer_down(Point::new(5., 5.), &switch));
    }

    #[test]
    fn radio_and_labeled_switch_expand_for_wrapped_text_and_align_to_first_line() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut text = TestText::default();
        let mut scene = Scene::new();
        let radio = RadioGroup::new(1);
        let mut radio_view = RadioGroupView::new();
        let radio_layout = radio_view.render(
            &mut scene,
            &mut text,
            &radio,
            Rect::new(0., 0., 90., 0.),
            None,
            palette,
            &["A long label that wraps across several lines"],
        );
        assert!(radio_layout.rows[0].frame().size.height > 36.0);
        let radio_measure = text.measure(
            "A long label that wraps across several lines",
            proposal::TextRole::UiBody,
            50.0,
        );
        let first_line_center = radio_layout.rows[0].frame().min_y()
            + (radio_layout.rows[0].frame().size.height - radio_measure.height) * 0.5
            + 10.0;
        assert_eq!(radio_layout.indicators[0].center().y, first_line_center);

        let switch = Switch::default();
        let mut switch_view = SwitchView::new();
        let switch_layout = switch_view.render_labeled(
            &mut scene,
            &mut text,
            switch,
            Rect::new(0., 100., 90., 32.),
            None,
            palette,
            "A long label that wraps across several lines",
        );
        assert!(switch_layout.frame.size.height > 32.0);
        let label_measure = text.measure(
            "A long label that wraps across several lines",
            proposal::TextRole::UiBody,
            28.0,
        );
        let expected_center = switch_layout.frame.min_y()
            + (switch_layout.frame.size.height - label_measure.height) * 0.5
            + 10.0
            + 1.0;
        assert_eq!(switch_layout.toggle.frame().center().y, expected_center);
    }

    #[test]
    fn disabled_or_inactive_changes_cancel_armed_controls_without_mutating_values() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut slider = Slider::new(50., 0., 100., 1.).unwrap();
        let mut slider_view = SliderView::new();
        let slider_layout = slider_view.render(
            &mut scene,
            slider,
            Rect::new(0., 0., 120., 32.),
            None,
            palette,
        );
        assert!(
            slider_view.pointer_down(Point::new(slider_layout.track.center().x, 16.), &mut slider)
        );
        slider.disabled = true;
        slider_view.pointer_move(Point::new(slider_layout.track.max_x(), 16.), &mut slider);
        assert!(!slider_view.is_captured());
        assert_eq!(slider.value(), 50.);
        slider.disabled = false;
        assert!(
            slider_view.pointer_down(Point::new(slider_layout.track.center().x, 16.), &mut slider)
        );
        slider_view.set_window_active(false);
        assert!(!slider_view.is_captured());
        assert!(!slider_view.pointer_up(Point::new(slider_layout.track.max_x(), 16.), &slider));
        assert_eq!(slider.value(), 50.);

        let mut switch = Switch::default();
        let mut switch_view = SwitchView::new();
        switch_view.render(
            &mut scene,
            switch,
            Rect::new(0., 0., 48., 26.),
            None,
            palette,
        );
        assert!(switch_view.pointer_down(Point::new(5., 5.), &switch));
        switch.disabled = true;
        assert_eq!(
            switch_view.pointer_up(Point::new(5., 5.), &mut switch),
            SelectionEvent::Handled
        );
        assert!(!switch.checked);
        switch.disabled = false;
        assert!(switch_view.pointer_down(Point::new(5., 5.), &switch));
        switch_view.set_window_active(false);
        assert_eq!(
            switch_view.pointer_up(Point::new(5., 5.), &mut switch),
            SelectionEvent::Ignored
        );
        assert!(!switch.checked);
        switch_view.set_window_active(true);
        switch_view.focus(true);
        switch_view.key(ControlKey::Space, true, &mut switch);
        switch.disabled = true;
        assert_eq!(
            switch_view.key(ControlKey::Space, false, &mut switch),
            SelectionEvent::Handled
        );
        assert!(!switch.checked);
    }

    #[test]
    fn completely_hidden_radio_rows_cancel_pointer_arm_and_switch_focus_ring_uses_ancestor_clip() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut text = TestText::default();
        let model = RadioGroup::new(2);
        let mut radio_view = RadioGroupView::new();
        let visible = radio_view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 180., 0.),
            None,
            palette,
            &["One", "Two"],
        );
        let point = visible.rows[0].frame().center();
        radio_view.pointer_down(point, &model);
        assert!(radio_view.cancel());
        radio_view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 180., 0.),
            Some(Rect::new(300., 300., 1., 1.)),
            palette,
            &["One", "Two"],
        );
        assert_eq!(
            radio_view.pointer_up(point, &mut model.clone()),
            SelectionEvent::Ignored
        );

        let switch = Switch::default();
        let mut switch_view = SwitchView::new();
        switch_view.focus(true);
        let clip = Rect::new(0., 0., 60., 26.);
        scene.clear();
        switch_view.render(
            &mut scene,
            switch,
            Rect::new(0., 0., 48., 26.),
            Some(clip),
            palette,
        );
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                tekes_ui_scene::Primitive::PushClip(rect) => Some(*rect),
                _ => None,
            })
            .collect();
        assert!(clips.contains(&clip));
    }

    #[test]
    fn narrow_radio_and_switch_label_slots_do_not_call_text_painter_outside_the_frame() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut scene = Scene::new();
        let mut text = TestText::default();
        let radio = RadioGroup::new(1);
        RadioGroupView::new().render(
            &mut scene,
            &mut text,
            &radio,
            Rect::new(0., 0., 32., 0.),
            None,
            palette,
            &["Too narrow"],
        );
        assert_eq!(text.draw_count, 0);

        let mut switch_view = SwitchView::new();
        switch_view.render_labeled(
            &mut scene,
            &mut text,
            Switch::default(),
            Rect::new(0., 40., 60., 32.),
            None,
            palette,
            "No label space",
        );
        assert_eq!(text.draw_count, 0);
    }

    #[derive(Default)]
    struct TestText {
        draw_count: usize,
        motion: Option<ControlMotionContext>,
        frame_requests: usize,
        inks: Vec<tekes_ui_tokens::Rgba>,
    }
    impl ButtonTextRenderer for TestText {
        fn motion_context(&self) -> Option<ControlMotionContext> {
            self.motion
        }

        fn request_animation_frame(&mut self) {
            self.frame_requests += 1;
        }

        fn measure(
            &mut self,
            text: &str,
            _: proposal::TextRole,
            max_width: f32,
        ) -> tekes_ui_scene::Size {
            let chars_per_line = (max_width / 7.).floor().max(1.);
            let lines = (text.chars().count() as f32 / chars_per_line)
                .ceil()
                .max(1.);
            tekes_ui_scene::Size::new((text.len() as f32 * 7.).min(max_width), lines * 20.)
        }
        fn first_line_height(&mut self, _: &str, _: proposal::TextRole, _: f32) -> f32 {
            20.
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            ink: tekes_ui_tokens::Rgba,
        ) {
            self.draw_count += 1;
            self.inks.push(ink);
        }
    }

    #[test]
    fn switch_thumb_is_component_owned_interruptible_and_model_changes_immediately() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut model = Switch::default();
        let mut view = SwitchView::new();
        let mut text = TestText {
            motion: Some(ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: false,
            }),
            ..Default::default()
        };
        let mut scene = Scene::new();
        let frame = Rect::new(0., 0., 180., 32.);
        view.render_labeled(
            &mut scene, &mut text, model, frame, None, palette, "Enabled",
        );

        let toggle = view.layout().frame();
        let center = Point::new(toggle.center().x, toggle.center().y);
        assert!(view.pointer_down(center, &model));
        assert_eq!(view.pointer_up(center, &mut model), SelectionEvent::Changed);
        assert!(
            model.checked,
            "the semantic model changes on pointer release immediately"
        );

        text.motion = Some(ControlMotionContext {
            now: Duration::from_millis(1),
            reduced_motion: false,
        });
        scene.clear();
        view.render_labeled(
            &mut scene, &mut text, model, frame, None, palette, "Enabled",
        );
        assert!(view.needs_animation());
        assert!(text.frame_requests > 0);
        assert!(view.tick(Duration::from_millis(60)));
        let midflight = view.thumb_transition.value();
        assert!(
            midflight > 0.0 && midflight < 1.0,
            "thumb should be between both endpoints"
        );

        assert!(model.set(false));
        text.motion = Some(ControlMotionContext {
            now: Duration::from_millis(60),
            reduced_motion: false,
        });
        scene.clear();
        view.render_labeled(
            &mut scene, &mut text, model, frame, None, palette, "Enabled",
        );
        view.tick(Duration::from_millis(90));
        let reversed = view.thumb_transition.value();
        assert!(
            reversed >= 0.0 && reversed < midflight,
            "reversal should retarget without jumping"
        );
    }

    #[test]
    fn switch_reduced_motion_snaps_and_fully_clipped_render_blurs() {
        let palette = Palette::new(Theme::LIGHT, 0).expect("registered Gallery brand");
        let mut view = SwitchView::new();
        let mut text = TestText {
            motion: Some(ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: false,
            }),
            ..Default::default()
        };
        let mut scene = Scene::new();
        let mut model = Switch::default();
        let frame = Rect::new(0., 0., 180., 32.);
        view.render_labeled(
            &mut scene, &mut text, model, frame, None, palette, "Enabled",
        );
        let point = Point::new(
            view.layout().frame().center().x,
            view.layout().frame().center().y,
        );
        assert!(view.pointer_down(point, &model));
        view.pointer_up(point, &mut model);
        assert!(model.checked);
        view.set_reduced_motion(true, Duration::from_millis(1), &model);
        assert_eq!(view.thumb_transition.value(), 1.0);
        assert!(!view.needs_animation());

        view.focus(true);
        view.render(
            &mut scene,
            model,
            frame,
            Some(Rect::new(500., 500., 1., 1.)),
            palette,
        );
        assert!(
            !view.is_focused(),
            "a fully clipped switch must release focus"
        );
        assert_eq!(view.pointer_up(point, &mut model), SelectionEvent::Ignored);
    }
}
