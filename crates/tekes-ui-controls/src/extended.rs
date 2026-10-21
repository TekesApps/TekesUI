//! Additional platform-independent component contracts.
//!
//! These types own behavior and layout data; a host remains responsible for
//! text shaping, event translation, clipping, and scheduling presentation.
use crate::{
    Button, ControlKey, ControlState,
    layout::{ContainerLayout, Insets},
};
use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, proposal};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stack {
    pub axis: Axis,
    pub gap: f32,
}
impl Stack {
    pub fn new(axis: Axis) -> Self {
        Self { axis, gap: 0. }
    }
    /// Divide the available main axis evenly. Negative/non-finite gaps are treated as zero.
    pub fn layout(&self, bounds: Rect, count: usize) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }
        let length = match self.axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let gap = finite_nonnegative(self.gap).min(length / count.saturating_sub(1).max(1) as f32);
        let gap_total = gap * count.saturating_sub(1) as f32;
        let item = ((length - gap_total).max(0.)) / count as f32;
        (0..count)
            .map(|i| match self.axis {
                Axis::Horizontal => Rect::new(
                    bounds.min_x() + i as f32 * (item + gap),
                    bounds.min_y(),
                    item,
                    bounds.size.height.max(0.),
                ),
                Axis::Vertical => Rect::new(
                    bounds.min_x(),
                    bounds.min_y() + i as f32 * (item + gap),
                    bounds.size.width.max(0.),
                    item,
                ),
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub gap_x: f32,
    pub gap_y: f32,
}
impl Grid {
    pub fn new(columns: usize) -> Self {
        Self {
            columns: columns.max(1),
            gap_x: 0.,
            gap_y: 0.,
        }
    }
    /// Lay out fixed-height cells in natural content order. Rows can extend past
    /// `bounds.max_y()`; the owning viewport must clip and scroll that content.
    pub fn layout(&self, bounds: Rect, count: usize, row_height: f32) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }
        let columns = self.columns.max(1);
        let gx = finite_nonnegative(self.gap_x)
            .min(bounds.size.width.max(0.) / columns.saturating_sub(1).max(1) as f32);
        let gy = finite_nonnegative(self.gap_y);
        let width = ((bounds.size.width.max(0.) - gx * columns.saturating_sub(1) as f32).max(0.))
            / columns as f32;
        let height = finite_nonnegative(row_height);
        (0..count)
            .map(|i| {
                let col = i % columns;
                let row = i / columns;
                Rect::new(
                    bounds.min_x() + col as f32 * (width + gx),
                    bounds.min_y() + row as f32 * (height + gy),
                    width,
                    height,
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Separator {
    pub axis: Axis,
}
impl Separator {
    pub fn new(axis: Axis) -> Self {
        Self { axis }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitView {
    ratio: f32,
    pub divider: f32,
    pub min_pane: f32,
    pub collapsed: Option<Pane>,
    drag: Option<(f32, f32, Option<Pane>, bool)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Leading,
    Trailing,
}

/// Geometry shared by an existing control's drawing and pointer hit target.
/// `frame` remains the normalized component frame; `visible_frame` is the
/// portion that survives clipping by the owning container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlFrame {
    layout: ContainerLayout,
}

impl ControlFrame {
    pub fn new(frame: Rect, ancestor_clip: Option<Rect>) -> Self {
        Self {
            layout: ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, ancestor_clip),
        }
    }

    pub const fn frame(self) -> Rect {
        self.layout.frame()
    }

    pub const fn visible_frame(self) -> Option<Rect> {
        self.layout.visible_clip()
    }

    /// Full document-space bounds needed to display the focus ring without
    /// clipping, including one backing pixel for antialiasing.
    pub fn focus_bounds(self, backing_scale: f32) -> Rect {
        let ring_width = proposal::metric("stroke", "c-focus-width").expect("focus width token");
        let ring_gap = proposal::metric("spacing", "c-focus-offset").expect("focus offset token");
        let scale = if backing_scale.is_finite() && backing_scale > 0.0 {
            backing_scale
        } else {
            1.0
        };
        self.frame().inset(
            -(ring_width + ring_gap + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS / scale),
        )
    }

    pub fn contains(self, point: tekes_ui_scene::Point) -> bool {
        self.visible_frame()
            .is_some_and(|frame| frame.contains(point))
    }
}

impl Default for ControlFrame {
    fn default() -> Self {
        Self::new(Rect::default(), None)
    }
}

/// One normalized split layout, with per-pane visible regions for clipping and
/// hit testing. The existing `SplitView::layout` API remains available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitViewLayout {
    pub frame: ControlFrame,
    pub leading: ControlFrame,
    pub divider: ControlFrame,
    pub trailing: ControlFrame,
}

impl SplitView {
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: finite_or(ratio, 0.5).clamp(0.05, 0.95),
            divider: 1.,
            min_pane: 24.,
            collapsed: None,
            drag: None,
        }
    }
    pub fn ratio(&self) -> f32 {
        self.ratio
    }
    /// Ratio after applying the minimum pane size for these bounds.
    pub fn ratio_in(&self, bounds: Rect, axis: Axis) -> f32 {
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let usable = (available - finite_nonnegative(self.divider).min(available)).max(0.);
        self.ratio
            .clamp(self.min_ratio(usable), 1. - self.min_ratio(usable))
    }
    fn min_ratio(&self, usable: f32) -> f32 {
        if usable >= 2. * finite_nonnegative(self.min_pane) && usable > 0. {
            (finite_nonnegative(self.min_pane) / usable).clamp(0.05, 0.5)
        } else {
            0.5
        }
    }
    pub fn set_ratio(&mut self, ratio: f32) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        let next = ratio.clamp(0.05, 0.95);
        let changed = self.ratio != next;
        self.ratio = next;
        changed
    }
    /// Restore an even, expanded split and finish any in-flight gesture.
    pub fn reset(&mut self) -> bool {
        self.drag = None;
        let restored = self.collapsed.take().is_some();
        self.set_ratio(0.5) || restored
    }
    pub fn collapse(&mut self, pane: Option<Pane>) {
        self.collapsed = pane;
    }
    /// True when both content minima cannot fit beside the divider.
    pub fn is_constrained(&self, bounds: Rect, axis: Axis) -> bool {
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let usable = (available - finite_nonnegative(self.divider).min(available)).max(0.);
        usable < 2. * finite_nonnegative(self.min_pane)
    }
    /// Select which panel occupies a constrained split view.
    pub fn choose_visible(&mut self, pane: Pane) {
        self.collapsed = Some(match pane {
            Pane::Leading => Pane::Trailing,
            Pane::Trailing => Pane::Leading,
        });
    }
    pub fn visible_pane(&self) -> Pane {
        match self.collapsed {
            Some(Pane::Leading) => Pane::Trailing,
            Some(Pane::Trailing) => Pane::Leading,
            None => Pane::Leading,
        }
    }
    /// Begin tracking at the divider. `bounds` are the complete split view bounds.
    pub fn begin_drag(&mut self, pointer: f32, bounds: Rect, axis: Axis) -> bool {
        if !pointer.is_finite() {
            return false;
        }
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let effective_divider = finite_nonnegative(self.divider).min(available);
        let usable = (available - effective_divider).max(0.);
        if self.is_constrained(bounds, axis) || usable < 2. * finite_nonnegative(self.min_pane) {
            return false;
        }
        self.drag = Some((pointer, self.ratio_in(bounds, axis), self.collapsed, false));
        true
    }
    /// A four point dead zone filters accidental touches; after it is crossed,
    /// pointer motion maps to the split proportion.
    pub fn drag_to(&mut self, pointer: f32, bounds: Rect, axis: Axis) -> bool {
        let Some((origin, initial_ratio, initial_collapsed, active)) = self.drag else {
            return false;
        };
        let delta = pointer - origin;
        if !pointer.is_finite() || (!active && delta.abs() < 4.) {
            return false;
        }
        if !active {
            self.drag = Some((origin, initial_ratio, initial_collapsed, true));
            self.collapsed = None;
        }
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let effective_divider = finite_nonnegative(self.divider).min(available);
        let usable = (available - effective_divider).max(0.);
        if usable <= 0. {
            return false;
        }
        let min_ratio = self.min_ratio(usable);
        let next = (initial_ratio + delta / usable).clamp(min_ratio, 1. - min_ratio);
        self.set_ratio(next)
    }
    pub fn end_drag(&mut self) {
        self.drag = None;
    }
    /// Roll back a drag on Escape or loss of pointer capture.
    pub fn cancel_drag(&mut self) -> bool {
        let Some((_, initial_ratio, initial_collapsed, _)) = self.drag.take() else {
            return false;
        };
        let changed = self.ratio != initial_ratio || self.collapsed != initial_collapsed;
        self.ratio = initial_ratio;
        self.collapsed = initial_collapsed;
        changed
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        match key {
            ControlKey::Left | ControlKey::Up => self.set_ratio(self.ratio - 0.02),
            ControlKey::Right | ControlKey::Down => self.set_ratio(self.ratio + 0.02),
            ControlKey::Home => self.set_ratio(0.05),
            ControlKey::End => self.set_ratio(0.95),
            ControlKey::Escape => self.cancel_drag(),
            ControlKey::Enter | ControlKey::Space => self.reset(),
            _ => false,
        }
    }
    /// Keyboard alternative that honors the same pixel minima as pointer dragging.
    pub fn key_in(&mut self, key: ControlKey, bounds: Rect, axis: Axis) -> bool {
        if key == ControlKey::Escape {
            return self.cancel_drag();
        }
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let usable = (available - finite_nonnegative(self.divider).min(available)).max(0.);
        if self.is_constrained(bounds, axis) {
            return match key {
                ControlKey::Left | ControlKey::Up | ControlKey::Home => {
                    self.choose_visible(Pane::Leading);
                    true
                }
                ControlKey::Right | ControlKey::Down | ControlKey::End => {
                    self.choose_visible(Pane::Trailing);
                    true
                }
                _ => false,
            };
        }
        if usable < 2. * finite_nonnegative(self.min_pane) {
            return false;
        }
        if matches!(key, ControlKey::Enter | ControlKey::Space) {
            return self.reset();
        }
        let min_ratio = self.min_ratio(usable);
        let current_ratio = self.ratio_in(bounds, axis);
        let next = match key {
            ControlKey::Left | ControlKey::Up => current_ratio - 0.02,
            ControlKey::Right | ControlKey::Down => current_ratio + 0.02,
            ControlKey::Home => min_ratio,
            ControlKey::End => 1. - min_ratio,
            _ => return false,
        };
        // A resize may make both panes feasible again after constrained-mode
        // keyboard selection. Divider keys must restore a usable split even
        // when the requested ratio is already at its clamped endpoint.
        let restored = self.collapsed.take().is_some();
        self.set_ratio(next.clamp(min_ratio, 1. - min_ratio)) || restored
    }
    pub fn layout(&self, bounds: Rect, axis: Axis) -> (Rect, Rect, Rect) {
        let available = match axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.);
        let divider = finite_nonnegative(self.divider).min(available);
        let split = (available - divider).max(0.);
        if self.is_constrained(bounds, axis) {
            let visible = self.visible_pane();
            return match axis {
                Axis::Horizontal => match visible {
                    Pane::Leading => (
                        bounds,
                        Rect::new(
                            bounds.max_x(),
                            bounds.min_y(),
                            0.,
                            bounds.size.height.max(0.),
                        ),
                        Rect::new(
                            bounds.max_x(),
                            bounds.min_y(),
                            0.,
                            bounds.size.height.max(0.),
                        ),
                    ),
                    Pane::Trailing => (
                        Rect::new(
                            bounds.min_x(),
                            bounds.min_y(),
                            0.,
                            bounds.size.height.max(0.),
                        ),
                        Rect::new(
                            bounds.min_x(),
                            bounds.min_y(),
                            0.,
                            bounds.size.height.max(0.),
                        ),
                        bounds,
                    ),
                },
                Axis::Vertical => match visible {
                    Pane::Leading => (
                        bounds,
                        Rect::new(bounds.min_x(), bounds.max_y(), 0., 0.),
                        Rect::new(
                            bounds.min_x(),
                            bounds.max_y(),
                            bounds.size.width.max(0.),
                            0.,
                        ),
                    ),
                    Pane::Trailing => (
                        Rect::new(
                            bounds.min_x(),
                            bounds.min_y(),
                            bounds.size.width.max(0.),
                            0.,
                        ),
                        Rect::new(bounds.min_x(), bounds.min_y(), 0., 0.),
                        bounds,
                    ),
                },
            };
        }
        let lead = match self.collapsed {
            Some(Pane::Leading) => 0.,
            Some(Pane::Trailing) => split,
            None => split * self.ratio_in(bounds, axis),
        };
        match axis {
            Axis::Horizontal => (
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y(),
                    lead,
                    bounds.size.height.max(0.),
                ),
                Rect::new(
                    bounds.min_x() + lead,
                    bounds.min_y(),
                    (available - split).min(divider),
                    bounds.size.height.max(0.),
                ),
                Rect::new(
                    bounds.min_x() + lead + divider,
                    bounds.min_y(),
                    (split - lead).max(0.),
                    bounds.size.height.max(0.),
                ),
            ),
            Axis::Vertical => (
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y(),
                    bounds.size.width.max(0.),
                    lead,
                ),
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y() + lead,
                    bounds.size.width.max(0.),
                    (available - split).min(divider),
                ),
                Rect::new(
                    bounds.min_x(),
                    bounds.min_y() + lead + divider,
                    bounds.size.width.max(0.),
                    (split - lead).max(0.),
                ),
            ),
        }
    }

    pub fn layout_clipped(
        &self,
        bounds: Rect,
        axis: Axis,
        ancestor_clip: Option<Rect>,
    ) -> SplitViewLayout {
        let frame = ControlFrame::new(bounds, ancestor_clip);
        let (leading, divider, trailing) = self.layout(frame.frame(), axis);
        SplitViewLayout {
            frame,
            leading: ControlFrame::new(leading, ancestor_clip),
            divider: ControlFrame::new(divider, ancestor_clip),
            trailing: ControlFrame::new(trailing, ancestor_clip),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Disclosure {
    pub open: bool,
    pub disabled: bool,
}
impl Disclosure {
    pub fn toggle(&mut self) -> bool {
        if self.disabled {
            return false;
        }
        self.open = !self.open;
        true
    }
    pub fn set_open(&mut self, open: bool) -> bool {
        if self.disabled || self.open == open {
            return false;
        }
        self.open = open;
        true
    }
}

#[derive(Clone, Debug)]
pub struct IconButton {
    pub button: Button,
    accessible_name: String,
}
impl IconButton {
    pub fn new(accessible_name: impl Into<String>) -> Option<Self> {
        let accessible_name = accessible_name.into();
        (!accessible_name.trim().is_empty()).then_some(Self {
            button: Button::default(),
            accessible_name,
        })
    }
    pub fn accessible_name(&self) -> &str {
        &self.accessible_name
    }
    pub fn layout(&self, frame: Rect, ancestor_clip: Option<Rect>) -> ControlFrame {
        ControlFrame::new(frame, ancestor_clip)
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        self.button.key(key, down)
    }
    pub fn pointer_down(&mut self) {
        self.button.pointer_down()
    }
    pub fn pointer_up(&mut self, inside: bool) -> bool {
        self.button.pointer_up(inside)
    }
}
#[derive(Clone, Debug, Default)]
pub struct Link {
    pub button: Button,
}
impl Link {
    pub fn layout(&self, frame: Rect, ancestor_clip: Option<Rect>) -> ControlFrame {
        ControlFrame::new(frame, ancestor_clip)
    }
    /// Links activate with Enter; Space remains available to page scrolling.
    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        self.button.synchronize_activation();
        if key == ControlKey::Enter {
            self.button.key(key, down)
        } else {
            false
        }
    }
    pub fn pointer_down(&mut self) {
        self.button.pointer_down()
    }
    pub fn pointer_up(&mut self, inside: bool) -> bool {
        self.button.pointer_up(inside)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    pub disabled: bool,
}
impl Slider {
    pub fn new(value: f64, min: f64, max: f64, step: f64) -> Option<Self> {
        if ![value, min, max, step].iter().all(|x| x.is_finite()) || min > max || step <= 0. {
            return None;
        }
        Some(Self {
            value: snap_clamped(value.clamp(min, max), min, max, step),
            min,
            max,
            step,
            disabled: false,
        })
    }
    /// The original finite bounds; no separate range state is introduced.
    pub fn range(self) -> std::ops::RangeInclusive<f64> {
        self.min..=self.max
    }
    pub fn value(self) -> f64 {
        self.value
    }
    pub fn layout(self, frame: Rect, ancestor_clip: Option<Rect>) -> ControlFrame {
        ControlFrame::new(frame, ancestor_clip)
    }
    /// Normalized position in the finite range; a degenerate range is zero.
    pub fn normalized_value(self) -> f64 {
        if self.max <= self.min {
            0.
        } else {
            let span = self.max - self.min;
            if span.is_finite() {
                ((self.value - self.min) / span).clamp(0., 1.)
            } else {
                // Halving first keeps a range spanning both finite extremes finite.
                ((self.value * 0.5 - self.min * 0.5) / (self.max * 0.5 - self.min * 0.5))
                    .clamp(0., 1.)
            }
        }
    }
    pub fn set(&mut self, value: f64) -> bool {
        if self.disabled || !value.is_finite() {
            return false;
        }
        let next = snap_clamped(
            value.clamp(self.min, self.max),
            self.min,
            self.max,
            self.step,
        );
        let changed = next != self.value;
        self.value = next;
        changed
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.disabled {
            return false;
        }
        match key {
            ControlKey::Left | ControlKey::Down => self.set((self.value - self.step).max(self.min)),
            ControlKey::Right | ControlKey::Up => self.set((self.value + self.step).min(self.max)),
            ControlKey::Home => self.set(self.min),
            ControlKey::End => self.set(self.max),
            _ => false,
        }
    }
    pub fn drag(&mut self, x: f32, track: Rect) -> bool {
        if self.disabled || track.size.width <= 0. || !x.is_finite() {
            return false;
        }
        let t = ((x - track.min_x()) / track.size.width).clamp(0., 1.) as f64;
        // A convex combination avoids overflowing the span for finite endpoints.
        self.set(self.min * (1. - t) + self.max * t)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioGroup {
    pub selected: Option<usize>,
    enabled: Vec<bool>,
}
impl RadioGroup {
    pub fn new(count: usize) -> Self {
        Self {
            selected: (count > 0).then_some(0),
            enabled: vec![true; count],
        }
    }
    pub fn len(&self) -> usize {
        self.enabled.len()
    }
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
    pub fn is_enabled(&self, index: usize) -> bool {
        self.enabled.get(index).copied().unwrap_or(false)
    }
    /// Place radio rows at the requested height and gap while sharing
    /// normalized layout and ancestor clipping with drawing and hit testing.
    pub fn row_layout(
        &self,
        bounds: Rect,
        row_height: f32,
        row_gap: f32,
        ancestor_clip: Option<Rect>,
    ) -> Vec<ControlFrame> {
        if !bounds.origin.x.is_finite() || !bounds.origin.y.is_finite() {
            return Vec::new();
        }
        let bounds = ControlFrame::new(bounds, ancestor_clip).frame();
        let row_height = finite_nonnegative(row_height);
        let row_gap = finite_nonnegative(row_gap);
        (0..self.len())
            .map(|index| {
                ControlFrame::new(
                    Rect::new(
                        bounds.min_x(),
                        bounds.min_y() + index as f32 * (row_height + row_gap),
                        bounds.size.width,
                        row_height,
                    ),
                    ancestor_clip,
                )
            })
            .collect()
    }
    pub fn set_enabled(&mut self, index: usize, enabled: bool) -> bool {
        let Some(slot) = self.enabled.get_mut(index) else {
            return false;
        };
        *slot = enabled;
        if !enabled && self.selected == Some(index) {
            self.selected = self.next_from(index, 1)
        }
        true
    }
    pub fn select(&mut self, index: usize) -> bool {
        if self.enabled.get(index) != Some(&true) || self.selected == Some(index) {
            return false;
        }
        self.selected = Some(index);
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        self.key_from(key, self.selected)
    }
    pub(crate) fn key_from(&mut self, key: ControlKey, focused: Option<usize>) -> bool {
        let Some(cur) = focused.filter(|index| self.is_enabled(*index)) else {
            return false;
        };
        let n = self.enabled.len();
        if n == 0 {
            return false;
        }
        let step = match key {
            ControlKey::Left | ControlKey::Up => -1,
            ControlKey::Right | ControlKey::Down => 1,
            ControlKey::Home => return self.find_enabled(0, 1),
            ControlKey::End => return self.find_enabled(n - 1, -1),
            _ => return false,
        };
        let next = self.next_from(cur, step);
        if let Some(i) = next {
            self.select(i)
        } else {
            false
        }
    }
    fn next_from(&self, from: usize, step: isize) -> Option<usize> {
        let n = self.enabled.len();
        if n == 0 {
            return None;
        }
        (1..=n)
            .map(|d| (from as isize + step * d as isize).rem_euclid(n as isize) as usize)
            .find(|i| self.enabled[*i])
    }
    fn find_enabled(&mut self, from: usize, step: isize) -> bool {
        let n = self.enabled.len();
        if n == 0 {
            return false;
        }
        if let Some(i) = (0..n)
            .map(|d| (from as isize + step * d as isize).rem_euclid(n as isize) as usize)
            .find(|i| self.enabled[*i])
        {
            self.select(i) || self.selected == Some(i)
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Switch {
    pub checked: bool,
    pub disabled: bool,
}
impl Switch {
    pub fn layout(&self, frame: Rect, ancestor_clip: Option<Rect>) -> ControlFrame {
        ControlFrame::new(frame, ancestor_clip)
    }
    pub fn toggle(&mut self) -> bool {
        if self.disabled {
            return false;
        }
        self.checked = !self.checked;
        true
    }
    pub fn set(&mut self, value: bool) -> bool {
        if self.disabled || self.checked == value {
            return false;
        }
        self.checked = value;
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedControl {
    pub selected: usize,
    enabled: Vec<bool>,
}
impl SegmentedControl {
    pub fn new(count: usize) -> Self {
        Self {
            selected: 0,
            enabled: vec![true; count],
        }
    }
    pub fn set_enabled(&mut self, i: usize, v: bool) -> bool {
        let Some(x) = self.enabled.get_mut(i) else {
            return false;
        };
        *x = v;
        if !v
            && self.selected == i
            && let Some(j) = self.enabled.iter().position(|e| *e)
        {
            self.selected = j
        }
        true
    }
    pub fn len(&self) -> usize {
        self.enabled.len()
    }
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
    pub fn is_enabled(&self, i: usize) -> bool {
        self.enabled.get(i).copied().unwrap_or(false)
    }
    pub fn select(&mut self, i: usize) -> bool {
        if self.enabled.get(i) != Some(&true) || self.selected == i {
            return false;
        }
        self.selected = i;
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        self.key_from(key, self.selected)
    }
    pub(crate) fn key_from(&mut self, key: ControlKey, focused: usize) -> bool {
        if self.enabled.is_empty() {
            return false;
        }
        let raw = match key {
            ControlKey::Left | ControlKey::Up => focused.saturating_sub(1),
            ControlKey::Right | ControlKey::Down => (focused + 1).min(self.enabled.len() - 1),
            ControlKey::Home => 0,
            ControlKey::End => self.enabled.len() - 1,
            _ => return false,
        };
        let i = match key {
            ControlKey::Left | ControlKey::Up | ControlKey::End => {
                (0..=raw).rev().find(|i| self.enabled[*i])
            }
            _ => (raw..self.enabled.len()).find(|i| self.enabled[*i]),
        };
        i.is_some_and(|i| self.select(i))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Breadcrumb {
    pub items: Vec<String>,
}
impl Breadcrumb {
    pub fn activate(&self, index: usize) -> Option<usize> {
        (index < self.items.len().saturating_sub(1)).then_some(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    pub selected: Option<usize>,
    pub count: usize,
}
impl List {
    pub fn new(count: usize) -> Self {
        Self {
            selected: None,
            count,
        }
    }
    pub fn select(&mut self, i: usize) -> bool {
        if i >= self.count || self.selected == Some(i) {
            return false;
        }
        self.selected = Some(i);
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.count == 0 {
            return false;
        }
        let next = match key {
            ControlKey::Up => self.selected.unwrap_or(self.count).saturating_sub(1),
            ControlKey::Down => (self.selected.map_or(0, |i| i + 1)).min(self.count - 1),
            ControlKey::Home => 0,
            ControlKey::End => self.count - 1,
            _ => return false,
        };
        self.select(next)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub label: String,
    pub parent: Option<usize>,
    pub expanded: bool,
    pub disabled: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tree {
    nodes: Vec<TreeNode>,
    pub selected: Option<usize>,
}
impl Tree {
    pub fn nodes(&self) -> &[TreeNode] {
        &self.nodes
    }
    pub fn add(&mut self, label: impl Into<String>, parent: Option<usize>) -> Option<usize> {
        if parent.is_some_and(|p| p >= self.nodes.len()) {
            return None;
        }
        let i = self.nodes.len();
        self.nodes.push(TreeNode {
            label: label.into(),
            parent,
            expanded: false,
            disabled: false,
        });
        Some(i)
    }
    pub fn set_disabled(&mut self, i: usize, disabled: bool) -> bool {
        let Some(node) = self.nodes.get_mut(i) else {
            return false;
        };
        node.disabled = disabled;
        if disabled && self.selected == Some(i) {
            self.selected = None
        }
        true
    }
    pub fn select(&mut self, i: usize) -> bool {
        if self.nodes.get(i).is_none_or(|n| n.disabled) || self.selected == Some(i) {
            return false;
        }
        self.selected = Some(i);
        true
    }
    pub fn toggle(&mut self, i: usize) -> bool {
        if self.nodes.get(i).is_none_or(|n| n.disabled)
            || !self.nodes.iter().any(|n| n.parent == Some(i))
        {
            return false;
        }
        self.nodes[i].expanded = !self.nodes[i].expanded;
        true
    }
    pub fn visible(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for i in 0..self.nodes.len() {
            let mut p = self.nodes[i].parent;
            let mut visible = true;
            while let Some(parent) = p {
                if !self.nodes[parent].expanded {
                    visible = false;
                    break;
                }
                p = self.nodes[parent].parent
            }
            if visible {
                out.push(i)
            }
        }
        out
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        let visible = self.visible();
        let Some(current) = self.selected else {
            return match key {
                ControlKey::End => visible
                    .iter()
                    .rev()
                    .copied()
                    .find(|i| !self.nodes[*i].disabled)
                    .is_some_and(|i| self.select(i)),
                _ => visible
                    .iter()
                    .copied()
                    .find(|i| !self.nodes[*i].disabled)
                    .is_some_and(|i| self.select(i)),
            };
        };
        let Some(pos) = visible.iter().position(|i| *i == current) else {
            return false;
        };
        match key {
            ControlKey::Down => visible
                .iter()
                .skip(pos + 1)
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Up => visible
                .iter()
                .take(pos)
                .rev()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Home => visible
                .iter()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::End => visible
                .iter()
                .rev()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Right => {
                if self.nodes[current].expanded {
                    visible
                        .iter()
                        .skip(pos + 1)
                        .copied()
                        .find(|i| {
                            self.nodes[*i].parent == Some(current) && !self.nodes[*i].disabled
                        })
                        .is_some_and(|i| self.select(i))
                } else {
                    self.toggle(current)
                }
            }
            ControlKey::Left => {
                if self.nodes[current].expanded {
                    self.toggle(current)
                } else if let Some(p) = self.nodes[current].parent {
                    self.select(p)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataTable {
    pub selected_row: Option<usize>,
    pub sort: Option<(usize, SortDirection)>,
    pub rows: usize,
    pub columns: usize,
}
impl DataTable {
    pub fn new(rows: usize, columns: usize) -> Self {
        Self {
            selected_row: None,
            sort: None,
            rows,
            columns,
        }
    }
    pub fn select_row(&mut self, row: usize) -> bool {
        if row >= self.rows || self.selected_row == Some(row) {
            return false;
        }
        self.selected_row = Some(row);
        true
    }
    pub fn sort_by(&mut self, column: usize) -> bool {
        if column >= self.columns {
            return false;
        }
        self.sort = Some(match self.sort {
            Some((c, SortDirection::Ascending)) if c == column => {
                (column, SortDirection::Descending)
            }
            _ => (column, SortDirection::Ascending),
        });
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.rows == 0 {
            return false;
        }
        let row = match key {
            ControlKey::Up => self.selected_row.unwrap_or(self.rows).saturating_sub(1),
            ControlKey::Down => self.selected_row.map_or(0, |r| (r + 1).min(self.rows - 1)),
            ControlKey::Home => 0,
            ControlKey::End => self.rows - 1,
            _ => return false,
        };
        self.select_row(row)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tooltip {
    pub visible: bool,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Popover {
    pub open: bool,
    pub return_focus: Option<usize>,
}
impl Popover {
    pub fn show(&mut self, invoker: usize) {
        self.open = true;
        self.return_focus = Some(invoker)
    }
    pub fn close(&mut self) -> Option<usize> {
        self.open = false;
        self.return_focus.take()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeTone {
    Info,
    Success,
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub message: String,
    pub tone: NoticeTone,
    pub visible: bool,
    remaining_ms: u32,
    pub paused: bool,
}
impl Toast {
    pub fn new(message: impl Into<String>, tone: NoticeTone, duration_ms: u32) -> Self {
        Self {
            message: message.into(),
            tone,
            visible: true,
            remaining_ms: duration_ms,
            paused: false,
        }
    }
    pub fn remaining_ms(&self) -> u32 {
        self.remaining_ms
    }
    pub fn show(&mut self, message: impl Into<String>, tone: NoticeTone, duration_ms: u32) {
        self.message = message.into();
        self.tone = tone;
        self.remaining_ms = duration_ms;
        self.visible = true;
        self.paused = false
    }
    /// Advance deterministic fixture time. Paused toasts do not expire.
    pub fn advance(&mut self, elapsed_ms: u32) -> bool {
        if !self.visible || self.paused {
            return false;
        }
        self.remaining_ms = self.remaining_ms.saturating_sub(elapsed_ms);
        if self.remaining_ms == 0 {
            self.visible = false;
            true
        } else {
            false
        }
    }
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.remaining_ms = 0;
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banner {
    pub message: String,
    pub tone: NoticeTone,
    pub dismissible: bool,
    pub visible: bool,
}
impl Banner {
    pub fn dismiss(&mut self) -> bool {
        if !self.dismissible || !self.visible {
            return false;
        }
        self.visible = false;
        true
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusBadge {
    Info,
    Running,
    Succeeded,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    value: f32,
    pub indeterminate: bool,
}
impl Progress {
    pub fn new(value: f32) -> Self {
        Self {
            value: finite_or(value, 0.).clamp(0., 1.),
            indeterminate: false,
        }
    }
    pub fn value(self) -> f32 {
        self.value
    }
    pub fn set(&mut self, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let v = value.clamp(0., 1.);
        let changed = self.value != v;
        self.value = v;
        changed
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyState {
    pub title: String,
    pub detail: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComboBox {
    pub items: Vec<String>,
    pub query: String,
    pub selected: Option<usize>,
    pub open: bool,
    pub active: Option<usize>,
    pub disabled: bool,
}
impl ComboBox {
    pub fn filtered(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.to_lowercase().contains(&q).then_some(i))
            .collect()
    }
    pub fn set_query(&mut self, query: impl Into<String>) {
        if !self.disabled {
            self.query = query.into();
            self.open = true;
            self.active = self.filtered().first().copied()
        }
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        if self.disabled {
            return None;
        }
        let filtered = self.filtered();
        if !self.open && matches!(key, ControlKey::Enter | ControlKey::Down) {
            self.open = true;
            self.active = filtered.first().copied();
            return None;
        }
        match key {
            ControlKey::Escape => self.open = false,
            ControlKey::Up | ControlKey::Down => {
                if filtered.is_empty() {
                    return None;
                }
                let pos = self
                    .active
                    .and_then(|i| filtered.iter().position(|x| *x == i))
                    .unwrap_or(0);
                let next = if key == ControlKey::Down {
                    (pos + 1).min(filtered.len() - 1)
                } else {
                    pos.saturating_sub(1)
                };
                self.active = Some(filtered[next]);
            }
            ControlKey::Enter => {
                let i = self.active?;
                if filtered.contains(&i) {
                    self.selected = Some(i);
                    self.query = self.items[i].clone();
                    self.open = false;
                    return Some(i);
                }
            }
            _ => {}
        }
        None
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandPalette {
    pub combo: ComboBox,
}
impl CommandPalette {
    pub fn open(&mut self) {
        if self.combo.disabled {
            return;
        }
        self.combo.open = true;
        self.combo.query.clear();
        self.combo.active = self.combo.filtered().first().copied()
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        self.combo.key(key)
    }
}

/// A deliberately bounded Markdown subset: headings, paragraphs, bullet lines, and fenced code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownBlock {
    Heading { level: u8, text: String },
    Paragraph(String),
    Bullet(String),
    Code { language: String, source: String },
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Markdown {
    pub blocks: Vec<MarkdownBlock>,
}
impl Markdown {
    pub fn parse(source: &str) -> Self {
        let mut blocks = Vec::new();
        let mut paragraph = Vec::new();
        let mut code: Option<(String, Vec<String>)> = None;
        for line in source.lines() {
            if let Some((lang, lines)) = &mut code {
                if line.trim_start().starts_with("```") {
                    blocks.push(MarkdownBlock::Code {
                        language: lang.clone(),
                        source: lines.join("\n"),
                    });
                    code = None
                } else {
                    lines.push(line.to_owned())
                }
                continue;
            }
            if let Some(fence) = line.trim().strip_prefix("```") {
                flush_paragraph(&mut blocks, &mut paragraph);
                code = Some((fence.trim().to_owned(), Vec::new()));
                continue;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                flush_paragraph(&mut blocks, &mut paragraph);
                continue;
            }
            let hashes = trimmed.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&hashes) && trimmed.as_bytes().get(hashes) == Some(&b' ') {
                flush_paragraph(&mut blocks, &mut paragraph);
                blocks.push(MarkdownBlock::Heading {
                    level: hashes as u8,
                    text: trimmed[hashes..].trim().to_owned(),
                })
            } else if let Some(text) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                flush_paragraph(&mut blocks, &mut paragraph);
                blocks.push(MarkdownBlock::Bullet(text.to_owned()))
            } else {
                paragraph.push(trimmed.to_owned())
            }
        }
        if let Some((language, lines)) = code {
            blocks.push(MarkdownBlock::Code {
                language,
                source: lines.join("\n"),
            })
        }
        flush_paragraph(&mut blocks, &mut paragraph);
        Self { blocks }
    }
}
fn flush_paragraph(blocks: &mut Vec<MarkdownBlock>, lines: &mut Vec<String>) {
    if !lines.is_empty() {
        blocks.push(MarkdownBlock::Paragraph(lines.join(" ")));
        lines.clear()
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeBlock {
    pub language: String,
    pub source: String,
}
impl CodeBlock {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.source.lines()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Equal,
    Added,
    Removed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffView {
    pub lines: Vec<DiffLine>,
}
impl DiffView {
    /// Line-oriented LCS diff. For very large inputs, falls back to a linear delete/add block.
    pub fn between(before: &str, after: &str) -> Self {
        let a: Vec<_> = before.lines().collect();
        let b: Vec<_> = after.lines().collect();
        if a.len().saturating_mul(b.len()) > 1_000_000
            || a.len() + b.len() > 4_000
            || before.len().saturating_add(after.len()) > 1_000_000
        {
            return Self {
                lines: a
                    .iter()
                    .map(|s| DiffLine {
                        kind: DiffKind::Removed,
                        text: (*s).into(),
                    })
                    .chain(b.iter().map(|s| DiffLine {
                        kind: DiffKind::Added,
                        text: (*s).into(),
                    }))
                    .collect(),
            };
        }
        let mut dp = vec![vec![0u32; b.len() + 1]; a.len() + 1];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                dp[i][j] = if a[i] == b[j] {
                    dp[i + 1][j + 1] + 1
                } else {
                    dp[i + 1][j].max(dp[i][j + 1])
                }
            }
        }
        let (mut i, mut j) = (0, 0);
        let mut lines = Vec::new();
        while i < a.len() && j < b.len() {
            if a[i] == b[j] {
                lines.push(DiffLine {
                    kind: DiffKind::Equal,
                    text: a[i].into(),
                });
                i += 1;
                j += 1
            } else if dp[i + 1][j] >= dp[i][j + 1] {
                lines.push(DiffLine {
                    kind: DiffKind::Removed,
                    text: a[i].into(),
                });
                i += 1
            } else {
                lines.push(DiffLine {
                    kind: DiffKind::Added,
                    text: b[j].into(),
                });
                j += 1
            }
        }
        lines.extend(a[i..].iter().map(|s| DiffLine {
            kind: DiffKind::Removed,
            text: (*s).into(),
        }));
        lines.extend(b[j..].iter().map(|s| DiffLine {
            kind: DiffKind::Added,
            text: (*s).into(),
        }));
        Self { lines }
    }
}

// Token-driven painters for models whose visuals are shared by consumers.
/// Decorative separator with the compatible 1x backing-scale default.
/// Hosts that know their renderer scale should use [`separator_at_scale`].
pub fn separator(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, axis: Axis) {
    separator_at_scale(scene, rect, theme, axis, 1.0);
}
/// Decorative separator using the existing JSON-backed hairline rule.
/// Does not change the supplied layout frame or imply a draggable boundary.
pub fn separator_at_scale(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    axis: Axis,
    backing_scale: f32,
) {
    let (width, color) = crate::paint::decorative_border(theme, backing_scale);
    let r = match axis {
        Axis::Horizontal => Rect::new(
            rect.min_x(),
            rect.center().y - width * 0.5,
            rect.size.width.max(0.),
            width,
        ),
        Axis::Vertical => Rect::new(
            rect.center().x - width * 0.5,
            rect.min_y(),
            width,
            rect.size.height.max(0.),
        ),
    };
    scene.push_quad(Quad::new(r, color));
}
pub fn switch(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    checked: bool,
    disabled: bool,
) {
    switch_with_state(
        scene,
        rect,
        theme,
        ControlState {
            selected: checked,
            disabled,
            ..ControlState::default()
        },
    );
}
pub fn switch_with_state(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) {
    switch_with_state_and_thumb_progress(
        scene,
        rect,
        theme,
        state,
        if state.selected { 1. } else { 0. },
    );
}
/// Thumb presentation progress never changes the model or full-track hit area.
pub fn switch_with_state_and_thumb_progress(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
    thumb_progress: f32,
) {
    let theme = theme.into();
    let checked = state.selected;
    let role = if state.disabled {
        "surface-2"
    } else if checked && state.pressed {
        "control-checked-pressed"
    } else if checked && state.hovered {
        "control-checked-hover"
    } else if checked {
        "c-switch-on-ground"
    } else if state.pressed {
        "button-pressed"
    } else if state.hovered {
        "button-hover"
    } else {
        "surface-3"
    };
    let track = paint_color(theme, role);
    let radius = rect.size.height.max(0.) * 0.5;
    let stroke = proposal::metric("stroke", "stroke-rule").expect("Design System control stroke");
    let boundary = paint_color(
        theme,
        if state.error {
            "status-failed"
        } else if state.disabled {
            "disabled-text"
        } else {
            "border"
        },
    );
    scene.push_quad(
        Quad::new(rect, track)
            .with_corner_radius(radius)
            .with_border(stroke, boundary),
    );
    let d = (rect.size.height - 4.).max(0.);
    let progress = if thumb_progress.is_finite() {
        thumb_progress.clamp(0., 1.)
    } else if checked {
        1.
    } else {
        0.
    };
    let x = rect.min_x() + 2. + (rect.size.width - d - 4.).max(0.) * progress;
    let thumb = if state.disabled {
        "disabled-text"
    } else if checked {
        "c-switch-on-thumb"
    } else {
        "surface-0"
    };
    scene.push_quad(
        Quad::new(
            Rect::new(x, rect.min_y() + 2., d, d),
            paint_color(theme, thumb),
        )
        .with_corner_radius(d * 0.5)
        .with_border(
            stroke,
            paint_color(
                theme,
                if state.disabled {
                    "disabled-text"
                } else {
                    "border"
                },
            ),
        ),
    );
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, rect, theme);
    }
}
pub fn radio(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    checked: bool,
    disabled: bool,
) {
    radio_with_state(
        scene,
        rect,
        theme,
        ControlState {
            selected: checked,
            disabled,
            ..ControlState::default()
        },
    );
}
pub fn radio_with_state(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) {
    let theme = theme.into();
    let checked = state.selected;
    let size = rect.size.width.min(rect.size.height).max(0.);
    let frame = Rect::new(
        rect.center().x - size * 0.5,
        rect.center().y - size * 0.5,
        size,
        size,
    );
    let fill = if state.disabled {
        paint_color(theme, "surface-2")
    } else if checked && state.pressed {
        paint_color(theme, "control-checked-pressed")
    } else if checked && state.hovered {
        paint_color(theme, "control-checked-hover")
    } else if checked {
        paint_color(theme, "c-radio-checked-ground")
    } else if state.pressed {
        paint_color(theme, "button-pressed")
    } else if state.hovered {
        paint_color(theme, "button-hover")
    } else {
        paint_color(theme, "surface-0")
    };
    scene.push_quad(
        Quad::new(frame, fill)
            .with_corner_radius(size * 0.5)
            .with_border(
                proposal::metric("stroke", "stroke-rule").expect("Design System control stroke"),
                paint_color(
                    theme,
                    if state.error {
                        "status-failed"
                    } else if state.disabled {
                        "disabled-text"
                    } else {
                        "border"
                    },
                ),
            ),
    );
    if checked {
        let inset = size * 0.28;
        scene.push_quad(
            Quad::new(
                frame.inset(inset),
                paint_color(
                    theme,
                    if state.disabled {
                        "disabled-text"
                    } else {
                        "c-radio-checked-mark"
                    },
                ),
            )
            .with_corner_radius(size * 0.5),
        );
    }
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, frame, theme);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderGeometry {
    pub track: Rect,
    pub thumb: Rect,
    pub thumb_center_x: f32,
}
pub fn slider(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    model: Slider,
    state: ControlState,
) -> SliderGeometry {
    let theme = theme.into();
    let disabled = state.disabled || model.disabled;
    let diameter = rect.size.height.max(0.).min(rect.size.width.max(0.));
    let radius = diameter * 0.5;
    let travel = (rect.size.width.max(0.) - diameter).max(0.);
    let center_x = rect.min_x() + radius + travel * model.normalized_value() as f32;
    let track = Rect::new(rect.min_x() + radius, rect.center().y - 2., travel, 4.);
    let thumb = Rect::new(
        center_x - radius,
        rect.center().y - radius,
        diameter,
        diameter,
    );
    let track_role = if disabled { "surface-2" } else { "surface-3" };
    scene.push_quad(Quad::new(track, paint_color(theme, track_role)).with_corner_radius(2.));
    let fill_role = if disabled {
        "disabled-text"
    } else if state.pressed {
        "accent-pressed"
    } else if state.hovered {
        "accent-hover"
    } else {
        "accent"
    };
    scene.push_quad(
        Quad::new(
            Rect::new(
                track.min_x(),
                track.min_y(),
                (center_x - track.min_x()).max(0.),
                track.size.height,
            ),
            paint_color(theme, fill_role),
        )
        .with_corner_radius(2.),
    );
    scene.push_quad(
        Quad::new(
            thumb,
            paint_color(theme, if disabled { "surface-2" } else { "surface-0" }),
        )
        .with_corner_radius(radius)
        .with_border(
            1.,
            paint_color(theme, if disabled { "disabled-text" } else { "border" }),
        ),
    );
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, thumb, theme);
    }
    SliderGeometry {
        track,
        thumb,
        thumb_center_x: center_x,
    }
}
fn focus_outline(scene: &mut Scene, rect: Rect, theme: Palette) {
    crate::paint::focus_ring(
        scene,
        rect,
        theme,
        rect.size.width.min(rect.size.height) * 0.5,
    );
}
/// The host shapes the label text and uses the returned semantic ink color.
pub fn link_underline(
    scene: &mut Scene,
    rect: Rect,
    theme: impl Into<Palette>,
    state: ControlState,
) -> Rgba {
    let theme = theme.into();
    let role = if state.disabled {
        "disabled-text"
    } else if state.pressed {
        "c-link-pressed"
    } else if state.hovered {
        "c-link-hover"
    } else {
        "c-link-foreground"
    };
    let ink = paint_color(theme, role);
    if state.focused && state.focus_visible && !state.disabled {
        focus_outline(scene, rect, theme);
    }
    let thickness = if state.focused && state.focus_visible && !state.disabled {
        2.
    } else {
        1.
    };
    scene.push_quad(Quad::new(
        Rect::new(
            rect.min_x(),
            rect.max_y() - thickness,
            rect.size.width.max(0.),
            thickness,
        ),
        ink,
    ));
    ink
}
pub fn progress(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, model: Progress) {
    let theme = theme.into();
    let width = rect.size.width.max(0.);
    let height = rect.size.height.max(0.);
    scene.push_quad(
        Quad::new(
            Rect::new(rect.min_x(), rect.min_y(), width, height),
            paint_color(theme, "surface-2"),
        )
        .with_corner_radius(height * 0.5),
    );
    let active = if model.indeterminate {
        width.min(width * 0.3)
    } else {
        width * model.value
    };
    scene.push_quad(
        Quad::new(
            Rect::new(rect.min_x(), rect.min_y(), active, height),
            paint_color(theme, "c-progress-active"),
        )
        .with_corner_radius(height * 0.5),
    );
}
pub fn status_badge(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, status: StatusBadge) {
    let theme = theme.into();
    let role = match status {
        StatusBadge::Info => "status-info",
        StatusBadge::Running => "status-running",
        StatusBadge::Succeeded => "status-succeeded",
        StatusBadge::Failed => "status-failed",
    };
    scene.push_quad(
        Quad::new(rect, paint_color(theme, role))
            .with_corner_radius(rect.size.height.max(0.) * 0.5),
    );
}
fn paint_color(theme: Palette, role: &str) -> Rgba {
    theme.color(role).expect("Design System semantic token")
}
fn finite_nonnegative(x: f32) -> f32 {
    if x.is_finite() { x.max(0.) } else { 0. }
}
fn finite_or(x: f32, fallback: f32) -> f32 {
    if x.is_finite() { x } else { fallback }
}
fn snap_clamped(value: f64, min: f64, max: f64, step: f64) -> f64 {
    if value <= min {
        return min;
    }
    if value >= max {
        return max;
    }
    let snapped = min + ((value - min) / step).round() * step;
    if snapped.is_finite() {
        snapped.clamp(min, max)
    } else {
        // Extremely fine steps or wide ranges can overflow intermediate arithmetic.
        // The already finite, bounded input remains the closest representable value.
        value.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::Point;
    #[test]
    fn stack_grid_and_split_respect_bounds() {
        let s = Stack {
            axis: Axis::Horizontal,
            gap: 10.,
        };
        assert_eq!(
            s.layout(Rect::new(0., 0., 110., 20.), 2),
            vec![Rect::new(0., 0., 50., 20.), Rect::new(60., 0., 50., 20.)]
        );
        let g = Grid::new(2);
        assert_eq!(g.layout(Rect::new(0., 0., 100., 100.), 3, 20.).len(), 3);
        let narrow = s.layout(Rect::new(0., 0., 5., 20.), 3);
        assert!(narrow.iter().all(|r| r.min_x() >= 0. && r.max_x() <= 5.));
        let mut huge = Stack {
            axis: Axis::Vertical,
            gap: 1000.,
        };
        let items = huge.layout(Rect::new(0., 0., 20., 5.), 3);
        assert!(items.iter().all(|r| r.min_y() >= 0. && r.max_y() <= 5.));
        huge.gap = f32::NAN;
        assert!(
            huge.layout(Rect::new(0., 0., 20., 5.), 2)
                .iter()
                .all(|r| r.size.height.is_finite())
        );
        let mut split = SplitView::new(9.);
        assert_eq!(split.ratio(), 0.95);
        split.collapse(Some(Pane::Leading));
        assert_eq!(
            split
                .layout(Rect::new(0., 0., 101., 40.), Axis::Horizontal)
                .0
                .size
                .width,
            0.
        );
        split.divider = 10.;
        let tiny = split.layout(Rect::new(0., 0., 3., 4.), Axis::Horizontal);
        assert!(tiny.1.max_x() <= 3. && tiny.2.min_x() <= 3.);
    }

    #[test]
    fn existing_control_layouts_keep_draw_frames_separate_from_visible_hits() {
        let clip = Rect::new(80., 20., 40., 60.);
        let slider = Slider::new(50., 0., 100., 5.).unwrap();
        let slider_frame = slider.layout(Rect::new(60., 30., 120., 36.), Some(clip));
        assert_eq!(slider_frame.frame(), Rect::new(60., 30., 120., 36.));
        assert_eq!(
            slider_frame.visible_frame(),
            Some(Rect::new(80., 30., 40., 36.))
        );
        assert!(slider_frame.contains(Point::new(90., 40.)));
        assert!(!slider_frame.contains(Point::new(70., 40.)));

        let radios = RadioGroup::new(3);
        let rows = radios.row_layout(Rect::new(60., 10., 180., f32::NAN), 36., 6., Some(clip));
        assert_eq!(rows[0].frame(), Rect::new(60., 10., 180., 36.));
        assert_eq!(rows[0].visible_frame(), Some(Rect::new(80., 20., 40., 26.)));
        assert_eq!(rows[1].frame(), Rect::new(60., 52., 180., 36.));
        assert_eq!(rows[1].visible_frame(), Some(Rect::new(80., 52., 40., 28.)));
        assert_eq!(rows[2].frame(), Rect::new(60., 94., 180., 36.));
        assert_eq!(rows[2].visible_frame(), None);
        assert!(
            radios
                .row_layout(Rect::new(f32::NAN, 10., 180., 0.), 36., 6., Some(clip))
                .is_empty()
        );
        assert_eq!(
            radios
                .row_layout(Rect::new(60., 10., 180., 0.), 36., 6., Some(clip))
                .len(),
            radios.len(),
            "a valid zero-height flow origin still lays out radio rows"
        );

        let split = SplitView::new(0.5);
        let split =
            split.layout_clipped(Rect::new(10., 10., 300., 80.), Axis::Horizontal, Some(clip));
        assert_eq!(split.leading.frame().size.width, 149.5);
        assert_eq!(
            split.leading.visible_frame(),
            Some(Rect::new(80., 20., 40., 60.))
        );
        assert_eq!(split.frame.visible_frame(), Some(clip));

        let switch = Switch::default().layout(Rect::new(60., 30., 48., 26.), Some(clip));
        assert_eq!(switch.frame(), Rect::new(60., 30., 48., 26.));
        assert_eq!(switch.visible_frame(), Some(Rect::new(80., 30., 28., 26.)));
    }

    #[test]
    fn control_frame_focus_bounds_include_ring_gap_and_backing_pixel_guard() {
        let frame = Rect::new(20.0, 30.0, 40.0, 24.0);
        let control = ControlFrame::new(frame, Some(Rect::new(10.0, 10.0, 80.0, 80.0)));
        let focus_1x = control.focus_bounds(1.0);
        let focus_2x = control.focus_bounds(2.0);
        let focus_outset = proposal::metric("spacing", "c-focus-offset").unwrap()
            + proposal::metric("stroke", "c-focus-width").unwrap();
        assert_eq!(focus_1x, frame.inset(-(focus_outset + 1.0)));
        assert_eq!(focus_2x, frame.inset(-(focus_outset + 0.5)));
        assert_eq!(control.frame(), frame);
        assert_eq!(control.visible_frame(), Some(frame));
    }

    #[test]
    fn clipped_control_frames_do_not_retain_hit_regions_after_resize_or_disable() {
        let icon = IconButton::new("Add").unwrap();
        let original = icon.layout(
            Rect::new(10., 10., 44., 40.),
            Some(Rect::new(0., 0., 80., 80.)),
        );
        assert!(original.contains(Point::new(20., 20.)));
        let resized = icon.layout(
            Rect::new(110., 110., 44., 40.),
            Some(Rect::new(0., 0., 80., 80.)),
        );
        assert_eq!(resized.visible_frame(), None);
        assert!(!resized.contains(Point::new(20., 20.)));

        let mut switch_model = Switch {
            checked: false,
            disabled: true,
        };
        assert!(!switch_model.toggle());
        assert!(!switch_model.set(true));
        assert!(!switch_model.checked);

        let mut radio = RadioGroup::new(2);
        radio.set_enabled(0, false);
        assert!(!radio.select(0));
        assert_eq!(radio.selected, Some(1));
        assert!(!radio.key(ControlKey::Left));
        assert_eq!(radio.selected, Some(1));
    }
    #[test]
    fn slider_rejects_nonfinite_and_maps_drag_and_keys() {
        let mut s = Slider::new(5., 0., 10., 2.).unwrap();
        assert!(s.drag(75., Rect::new(0., 0., 100., 10.)));
        assert_eq!(s.value(), 8.);
        assert!(s.key(ControlKey::Home));
        assert_eq!(s.value(), 0.);
        assert!(!s.set(f64::NAN));
        assert!(Slider::new(0., 0., 1., 0.).is_none());
    }
    #[test]
    fn radio_and_segmented_navigation_skip_disabled() {
        let mut r = RadioGroup::new(3);
        r.set_enabled(1, false);
        assert!(r.key(ControlKey::Right));
        assert_eq!(r.selected, Some(2));
        let mut s = SegmentedControl::new(3);
        s.set_enabled(1, false);
        assert!(s.key(ControlKey::Right));
        assert_eq!(s.selected, 2);
    }
    #[test]
    fn tree_expansion_and_navigation_are_structural() {
        let mut t = Tree::default();
        let root = t.add("root", None).unwrap();
        let child = t.add("child", Some(root)).unwrap();
        assert_eq!(t.visible(), vec![root]);
        assert!(t.select(root));
        assert!(t.toggle(root));
        assert_eq!(t.visible(), vec![root, child]);
        assert!(t.key(ControlKey::Right));
        assert_eq!(t.selected, Some(child));
        assert!(t.key(ControlKey::Left));
        assert_eq!(t.selected, Some(root));
        assert!(t.key(ControlKey::End));
        assert_eq!(t.selected, Some(child));
        assert!(t.set_disabled(child, true));
        assert_eq!(t.selected, None);
        assert!(t.key(ControlKey::End));
        assert_eq!(t.selected, Some(root));
        assert_eq!(t.add("bad", Some(99)), None);
    }
    #[test]
    fn combo_filters_and_commits_only_visible_items() {
        let mut c = ComboBox {
            items: vec!["Apple".into(), "Pear".into()],
            ..Default::default()
        };
        c.set_query("pea");
        assert_eq!(c.filtered(), vec![1]);
        assert_eq!(c.key(ControlKey::Enter), Some(1));
        assert_eq!(c.selected, Some(1));
        assert!(!c.open);
    }
    #[test]
    fn markdown_subset_and_lcs_diff_are_explicit_and_ordered() {
        let m = Markdown::parse("# Title\n\nhello\nworld\n\n- item\n```rs\nx\n```");
        assert_eq!(
            m.blocks,
            vec![
                MarkdownBlock::Heading {
                    level: 1,
                    text: "Title".into()
                },
                MarkdownBlock::Paragraph("hello world".into()),
                MarkdownBlock::Bullet("item".into()),
                MarkdownBlock::Code {
                    language: "rs".into(),
                    source: "x".into()
                }
            ]
        );
        let d = DiffView::between("a\nb", "a\nc");
        assert_eq!(
            d.lines.iter().map(|l| l.kind).collect::<Vec<_>>(),
            vec![DiffKind::Equal, DiffKind::Removed, DiffKind::Added]
        );
    }
    #[test]
    fn disabled_components_and_focus_return_have_contracts() {
        let mut d = Disclosure {
            open: false,
            disabled: true,
        };
        assert!(!d.toggle());
        let mut p = Popover::default();
        p.show(42);
        assert_eq!(p.close(), Some(42));
        assert_eq!(p.close(), None);
        let mut b = Banner {
            message: "x".into(),
            tone: NoticeTone::Info,
            dismissible: false,
            visible: true,
        };
        assert!(!b.dismiss());
    }
    #[test]
    fn split_drag_deadzone_cancel_and_keyboard_restore() {
        let mut s = SplitView::new(0.5);
        let bounds = Rect::new(0., 0., 204., 40.);
        assert!(s.begin_drag(100., bounds, Axis::Horizontal));
        assert!(!s.drag_to(103., bounds, Axis::Horizontal));
        assert!(s.drag_to(120., bounds, Axis::Horizontal));
        assert!(s.ratio() > 0.5);
        assert!(s.drag_to(102., bounds, Axis::Horizontal));
        assert!(
            s.ratio() > 0.5,
            "the dead zone must stay latched after drag begins"
        );
        assert!(s.cancel_drag());
        assert_eq!(s.ratio(), 0.5);
        assert!(s.begin_drag(100., bounds, Axis::Horizontal));
        assert!(s.drag_to(120., bounds, Axis::Horizontal));
        s.end_drag();
        assert!(s.key(ControlKey::Left));
        assert!(!s.key(ControlKey::Escape));
        assert!(s.key_in(ControlKey::Home, bounds, Axis::Horizontal));
        let panes = s.layout(bounds, Axis::Horizontal);
        assert!(panes.0.size.width >= s.min_pane);
        assert!(panes.2.size.width >= s.min_pane);
    }
    #[test]
    fn split_reset_restores_balanced_expanded_state_and_ends_gesture() {
        let mut split = SplitView::new(0.7);
        let bounds = Rect::new(0., 0., 200., 100.);
        split.collapse(Some(Pane::Leading));
        assert!(split.begin_drag(140., bounds, Axis::Horizontal));
        assert!(split.drag_to(160., bounds, Axis::Horizontal));
        assert!(split.key_in(ControlKey::Enter, bounds, Axis::Horizontal));
        assert_eq!(split.ratio(), 0.5);
        assert_eq!(split.collapsed, None);
        assert!(!split.drag_to(180., bounds, Axis::Horizontal));
        assert!(
            !split.cancel_drag(),
            "reset cannot later roll back to the old ratio"
        );
        split.set_ratio(0.8);
        assert!(split.key_in(ControlKey::Space, bounds, Axis::Vertical));
        assert_eq!(split.ratio(), 0.5);
    }
    #[test]
    fn split_infeasible_minima_switches_single_visible_panel() {
        let mut split = SplitView::new(0.4);
        let narrow = Rect::new(0., 0., 40., 80.); // 39 content points cannot hold two 24pt panes.
        assert!(split.is_constrained(narrow, Axis::Horizontal));
        assert!(!split.begin_drag(20., narrow, Axis::Horizontal));
        let (leading, divider, trailing) = split.layout(narrow, Axis::Horizontal);
        assert_eq!(leading, narrow);
        assert!(divider.is_empty());
        assert!(trailing.is_empty());
        assert!(split.key_in(ControlKey::Right, narrow, Axis::Horizontal));
        assert_eq!(split.visible_pane(), Pane::Trailing);
        let (leading, divider, trailing) = split.layout(narrow, Axis::Horizontal);
        assert!(leading.is_empty());
        assert!(divider.is_empty());
        assert_eq!(trailing, narrow);
        assert!(split.key_in(ControlKey::Left, narrow, Axis::Horizontal));
        assert_eq!(split.visible_pane(), Pane::Leading);
    }
    #[test]
    fn split_cancel_restores_ratio_and_switchable_pane_after_latched_drag() {
        let mut split = SplitView::new(0.5);
        split.choose_visible(Pane::Trailing);
        let bounds = Rect::new(0., 0., 160., 50.);
        assert!(split.begin_drag(80., bounds, Axis::Horizontal));
        assert!(!split.drag_to(83., bounds, Axis::Horizontal));
        assert!(split.drag_to(84., bounds, Axis::Horizontal));
        assert_eq!(split.collapsed, None);
        assert!(split.drag_to(82., bounds, Axis::Horizontal)); // latch persists inside the dead zone.
        assert!(split.cancel_drag());
        assert_eq!(split.visible_pane(), Pane::Trailing);
        let (leading, _, trailing) = split.layout(Rect::new(0., 0., 40., 50.), Axis::Horizontal);
        assert!(leading.is_empty());
        assert_eq!(trailing.size.width, 40.);
    }
    #[test]
    fn toast_uses_reproducible_and_paused_time() {
        let mut toast = Toast::new("saved", NoticeTone::Success, 10);
        toast.paused = true;
        assert!(!toast.advance(100));
        assert_eq!(toast.remaining_ms(), 10);
        toast.paused = false;
        assert!(!toast.advance(4));
        assert!(toast.advance(20));
        assert!(!toast.visible);
        toast.show("retry", NoticeTone::Info, 5);
        assert_eq!(toast.remaining_ms(), 5);
    }
    #[test]
    fn progress_painter_clamps_negative_geometry_and_semantic_tokens_exist() {
        for theme in tekes_ui_tokens::Theme::ALL {
            let mut scene = Scene::new();
            let palette: Palette = theme.into();
            progress(
                &mut scene,
                Rect::new(0., 0., -4., -3.),
                theme,
                Progress::new(1.),
            );
            switch(&mut scene, Rect::new(0., 0., 32., 18.), theme, true, false);
            radio(&mut scene, Rect::new(0., 0., 20., 20.), theme, true, false);
            status_badge(
                &mut scene,
                Rect::new(0., 0., 80., 20.),
                theme,
                StatusBadge::Succeeded,
            );
            let ink = link_underline(
                &mut scene,
                Rect::new(0., 0., 80., 20.),
                theme,
                ControlState {
                    focused: true,
                    focus_visible: true,
                    ..ControlState::default()
                },
            );
            assert_eq!(ink, palette.color("c-link-foreground").unwrap());
            assert!(scene.len() >= 9);
        }
    }
    #[test]
    fn stateful_control_painters_use_brand_tokens_for_all_themes() {
        for theme in tekes_ui_tokens::Theme::ALL {
            for brand in [0, 1, 2] {
                let palette = Palette { theme, brand };
                let checked_focus = ControlState {
                    selected: true,
                    focused: true,
                    focus_visible: true,
                    ..ControlState::default()
                };
                let mut scene = Scene::new();
                switch_with_state(
                    &mut scene,
                    Rect::new(0., 0., 42., 24.),
                    palette,
                    checked_focus,
                );
                let stroke = proposal::metric("stroke", "stroke-rule").unwrap();
                for index in [0, 1] {
                    let tekes_ui_scene::Primitive::Quad(quad) = &scene.primitives()[index] else {
                        panic!("switch paints quads")
                    };
                    assert_eq!(quad.border_width, stroke);
                    assert_eq!(quad.border_color, palette.color("border").unwrap());
                }
                radio_with_state(
                    &mut scene,
                    Rect::new(50., 0., 24., 24.),
                    palette,
                    checked_focus,
                );
                let model = Slider::new(25., 0., 100., 1.).unwrap();
                assert_eq!(model.normalized_value(), 0.25);
                let geometry = slider(
                    &mut scene,
                    Rect::new(0., 40., 120., 24.),
                    palette,
                    model,
                    ControlState {
                        hovered: true,
                        ..ControlState::default()
                    },
                );
                assert!(geometry.thumb_center_x > geometry.track.min_x());
                assert!(geometry.thumb_center_x < geometry.track.max_x());
                let roles = [
                    "c-switch-on-ground",
                    "c-switch-on-thumb",
                    "control-checked-hover",
                    "control-checked-pressed",
                    "c-radio-checked-ground",
                    "c-radio-checked-mark",
                    "c-progress-active",
                    "border-focused",
                ];
                assert!(roles.iter().all(|role| palette.color(role).is_some()));
                let focus_rings = scene.primitives().iter().filter(|p| matches!(p,
                    tekes_ui_scene::Primitive::Quad(q) if q.fill.a == 0.0
                        && q.border_color == palette.color("border-focused").unwrap()
                        && q.border_width == proposal::metric("stroke", "c-focus-width").unwrap()
                )).count();
                assert_eq!(
                    focus_rings, 2,
                    "Switch and Radio keep independent focus rings"
                );

                for (state, role) in [
                    (
                        ControlState {
                            selected: true,
                            hovered: true,
                            ..ControlState::default()
                        },
                        "control-checked-hover",
                    ),
                    (
                        ControlState {
                            selected: true,
                            pressed: true,
                            ..ControlState::default()
                        },
                        "control-checked-pressed",
                    ),
                ] {
                    let mut interactive = Scene::new();
                    switch_with_state(
                        &mut interactive,
                        Rect::new(0., 0., 42., 24.),
                        palette,
                        state,
                    );
                    radio_with_state(
                        &mut interactive,
                        Rect::new(50., 0., 24., 24.),
                        palette,
                        state,
                    );
                    assert_eq!(
                        scene_quad_fill(&interactive, 0),
                        palette.color(role).unwrap()
                    );
                    assert_eq!(
                        scene_quad_fill(&interactive, 2),
                        palette.color(role).unwrap()
                    );
                }

                let mut disabled = Scene::new();
                let state = ControlState {
                    selected: true,
                    disabled: true,
                    focused: true,
                    focus_visible: true,
                    ..ControlState::default()
                };
                switch_with_state(&mut disabled, Rect::new(0., 0., 42., 24.), palette, state);
                radio_with_state(&mut disabled, Rect::new(50., 0., 24., 24.), palette, state);
                assert_eq!(
                    disabled.len(),
                    4,
                    "disabled Switch/Radio have no focus rings"
                );
                assert_eq!(
                    scene_quad_fill(&disabled, 3),
                    palette.color("disabled-text").unwrap()
                );
                for index in [0, 1] {
                    let tekes_ui_scene::Primitive::Quad(quad) = &disabled.primitives()[index]
                    else {
                        panic!("switch paints quads")
                    };
                    assert_eq!(quad.border_width, stroke);
                    assert_eq!(quad.border_color, palette.color("disabled-text").unwrap());
                }
            }
        }
    }
    #[test]
    fn split_keyboard_restores_both_panes_after_constrained_resize() {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let narrow = Rect::new(0., 0., 120., 120.);
            let wide = Rect::new(0., 0., 1000., 1000.);
            let mut split = SplitView::new(0.5);
            split.min_pane = 160.;
            assert!(split.key_in(ControlKey::Right, narrow, axis));
            assert!(split.collapsed.is_some());
            assert!(split.key_in(ControlKey::Home, wide, axis));
            assert_eq!(split.collapsed, None);
            let (leading, divider, trailing) = split.layout(wide, axis);
            assert!(leading.size.width > 0. && leading.size.height > 0.);
            assert!(divider.size.width > 0. && divider.size.height > 0.);
            assert!(trailing.size.width > 0. && trailing.size.height > 0.);
        }
    }

    fn scene_quad_fill(scene: &Scene, index: usize) -> Rgba {
        match &scene.primitives()[index] {
            tekes_ui_scene::Primitive::Quad(q) => q.fill,
            _ => panic!("expected a component quad"),
        }
    }
    #[test]
    fn data_table_selects_rows_and_toggles_sort_direction() {
        let mut table = DataTable::new(3, 2);
        assert!(table.sort_by(0));
        assert_eq!(table.sort, Some((0, SortDirection::Ascending)));
        assert!(table.sort_by(0));
        assert_eq!(table.sort, Some((0, SortDirection::Descending)));
        assert!(table.key(ControlKey::End));
        assert_eq!(table.selected_row, Some(2));
        assert!(!table.sort_by(2));
    }
}
