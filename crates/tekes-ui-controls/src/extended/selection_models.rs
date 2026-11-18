use super::{ControlFrame, finite_nonnegative, snap_clamped};
use crate::ControlKey;
use tekes_ui_scene::Rect;

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

/// Consumed by the original selection owner after its caller validates the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RadioSelectDecision {
    before: Option<usize>,
    next: Option<usize>,
}
impl RadioSelectDecision {
    pub(crate) fn changed(self) -> bool {
        self.before != self.next
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioGroup {
    pub selected: Option<usize>,
    enabled: Vec<bool>,
}
/// A controlled Radio selection must address an enabled original slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioSelectionError {
    InvalidIndex,
    DisabledItem,
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
    /// Validate the entire update before changing the original selection owner.
    /// None is the original unselected state; existing select/key APIs retain
    /// their behavior. Focus and gesture cancellation belong to the View.
    pub fn validate_selected(&self, selected: Option<usize>) -> Result<(), RadioSelectionError> {
        if let Some(index) = selected {
            if index >= self.len() {
                return Err(RadioSelectionError::InvalidIndex);
            }
            if !self.is_enabled(index) {
                return Err(RadioSelectionError::DisabledItem);
            }
        }
        Ok(())
    }
    pub fn set_selected(&mut self, selected: Option<usize>) -> Result<bool, RadioSelectionError> {
        self.validate_selected(selected)?;
        let changed = self.selected != selected;
        if let Some(index) = selected {
            self.select(index);
        } else {
            self.selected = None;
        }
        Ok(changed)
    }
    pub(crate) fn prepare_select(&self, index: usize) -> RadioSelectDecision {
        RadioSelectDecision {
            before: self.selected,
            next: if self.is_enabled(index) {
                Some(index)
            } else {
                self.selected
            },
        }
    }
    pub(crate) fn apply_select(&mut self, decision: RadioSelectDecision) -> bool {
        let changed = decision.before != decision.next;
        self.selected = decision.next;
        changed
    }
    pub fn select(&mut self, index: usize) -> bool {
        self.apply_select(self.prepare_select(index))
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedSelectionDecision {
    index: usize,
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
    pub(crate) fn prepare_select(&self, i: usize) -> Option<SegmentedSelectionDecision> {
        (self.is_enabled(i) && self.selected != i)
            .then_some(SegmentedSelectionDecision { index: i })
    }
    pub(crate) fn apply_prepared_select(&mut self, decision: SegmentedSelectionDecision) {
        self.selected = decision.index;
    }
    pub fn select(&mut self, i: usize) -> bool {
        let Some(decision) = self.prepare_select(i) else {
            return false;
        };
        self.apply_prepared_select(decision);
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
