//! Additional platform-independent component contracts.
//!
//! These types own behavior and layout data; a host remains responsible for
//! text shaping, event translation, clipping, and scheduling presentation.
use crate::{
    ButtonModel, ControlKey,
    layout::{ContainerLayout, Insets},
};
use tekes_ui_scene::{Rect, Scene};
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
    /// Consumer-owned minimum along the split axis, in logical points.
    /// Defaults to zero; no additional percentage floor is imposed.
    pub min_pane: f32,
    pub collapsed: Option<Pane>,
    drag: Option<(f32, f32, Option<Pane>, bool)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Leading,
    Trailing,
}

/// Shared focus stroke, gap and one backing-pixel coverage guard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusRingMetrics {
    pub width: f32,
    pub gap: f32,
    pub antialias_guard: f32,
}
impl FocusRingMetrics {
    pub fn new(backing_scale: f32) -> Self {
        let scale = if backing_scale.is_finite() && backing_scale > 0.0 {
            backing_scale
        } else {
            1.0
        };
        Self {
            width: proposal::metric("stroke", "c-focus-width").unwrap_or(2.0),
            gap: proposal::metric("spacing", "c-focus-offset").unwrap_or(2.0),
            antialias_guard: proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS / scale,
        }
    }
    pub fn paint(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, radius: f32) {
        crate::paint::focus_ring(scene, rect, theme, radius);
    }
    pub fn outset(self) -> f32 {
        self.width + self.gap
    }

    pub fn reserved_band(self) -> f32 {
        self.outset() + self.antialias_guard
    }

    pub fn bounds(self, frame: Rect) -> Rect {
        frame.inset(-self.reserved_band())
    }
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
        FocusRingMetrics::new(backing_scale).bounds(self.frame())
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
            ratio: finite_or(ratio, 0.5).clamp(0., 1.),
            divider: 1.,
            min_pane: 0.,
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
            (finite_nonnegative(self.min_pane) / usable).clamp(0., 0.5)
        } else {
            0.5
        }
    }
    pub fn set_ratio(&mut self, ratio: f32) -> bool {
        if !ratio.is_finite() {
            return false;
        }
        let next = ratio.clamp(0., 1.);
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
        if self.is_constrained(bounds, axis)
            || usable <= 0.
            || usable < 2. * finite_nonnegative(self.min_pane)
        {
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
            ControlKey::Home => self.set_ratio(0.),
            ControlKey::End => self.set_ratio(1.),
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

#[derive(Clone, Debug, Default)]
pub struct Link {
    pub button: ButtonModel,
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

mod datatable;
mod feedback_and_paint;
mod list_tree;
mod selection_models;

pub use datatable::{
    DataTable, KeyedDataTable, KeyedTableColumn, KeyedTableRow, KeyedTableSnapshot, SortDirection,
    TableBudget, TableColumnKey, TableDatasetKey, TableReconcile, TableRowKey, TableUpdateError,
};
pub use feedback_and_paint::{
    Banner, CodeBlock, ComboBox, CommandPalette, DiffKind, DiffLine, DiffView, EmptyState,
    Markdown, MarkdownBlock, NoticeTone, Popover, Progress, SliderGeometry, StatusBadge, Toast,
    Tooltip, link_underline, link_underline_on_surface, progress, progress_on_surface, radio,
    radio_with_state, separator, separator_at_scale, slider, slider_on_surface, status_badge,
    status_badge_on_surface, switch, switch_on_surface, switch_on_surface_with_thumb_progress,
    switch_with_state, switch_with_state_and_thumb_progress,
};
pub(crate) use feedback_and_paint::{
    ComboSelectionDecision, link_underline_with_content_on_surface,
    radio_on_surface_with_progress, status_badge_usage,
};
pub(crate) use list_tree::ListStage;
pub use list_tree::{
    KeyedList, KeyedListItem, KeyedListSnapshot, KeyedTreeNode, List, ListBudget, ListBudgetKind,
    ListDatasetKey, ListItemKey, ListReconcile, ListUpdateError, Tree, TreeDatasetKey, TreeNode,
    TreeNodeKey, TreeReconcile, TreeUpdateError,
};
pub use selection_models::{
    Breadcrumb, RadioGroup, RadioSelectionError, SegmentedControl, Slider, Switch,
};
pub(crate) use selection_models::{RadioSelectDecision, SegmentedSelectionDecision};

#[cfg(test)]
mod tests;

pub(super) fn paint_color(theme: Palette, role: &str) -> Rgba {
    theme.color(role).expect("Design System semantic token")
}

pub(super) fn finite_nonnegative(x: f32) -> f32 {
    if x.is_finite() { x.max(0.) } else { 0. }
}
pub(super) fn finite_or(x: f32, fallback: f32) -> f32 {
    if x.is_finite() { x } else { fallback }
}
pub(super) fn snap_clamped(value: f64, min: f64, max: f64, step: f64) -> f64 {
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
