//! Fixed-slot integer counter. The consumer supplies elapsed frame time; no timer is owned here.
use std::time::Duration;

use crate::{ButtonTextAlign, ButtonTextRenderer};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, proposal};

/// Invalid digit count, out-of-range value, or invalid animation settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickingNumberError {
    Digits,
    Value,
    Strength,
}

/// An unsigned, zero-padded counter with a permanently reserved number of slots.
///
/// Values that do not fit are rejected, never silently clipped or allowed to grow
/// the layout. Up to 15 digits are supported so interpolation retains integer precision.
#[derive(Clone, Debug)]
pub struct TickingNumber {
    digits: usize,
    current: f64,
    from: f64,
    target: u64,
    elapsed: Duration,
    duration: Duration,
    strength: f64,
    reduced_motion: bool,
}

impl TickingNumber {
    pub fn new(value: u64, digits: usize) -> Result<Self, TickingNumberError> {
        if !(1..=15).contains(&digits) {
            return Err(TickingNumberError::Digits);
        }
        if value >= 10_u64.pow(digits as u32) {
            return Err(TickingNumberError::Value);
        }
        Ok(Self {
            digits,
            current: value as f64,
            from: value as f64,
            target: value,
            elapsed: Duration::ZERO,
            duration: Duration::from_millis(1800),
            strength: 1.0,
            reduced_motion: false,
        })
    }

    pub fn digits(&self) -> usize {
        self.digits
    }
    pub fn value(&self) -> u64 {
        self.current.round() as u64
    }
    pub fn target(&self) -> u64 {
        self.target
    }
    pub fn strength(&self) -> f64 {
        self.strength
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn text(&self) -> String {
        format!("{:0width$}", self.value(), width = self.digits)
    }
    pub fn needs_animation(&self) -> bool {
        self.current != self.target as f64
    }

    /// 0 = linear, 1 = full smooth acceleration/deceleration.
    pub fn set_strength(&mut self, strength: f64) -> Result<(), TickingNumberError> {
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            return Err(TickingNumberError::Strength);
        }
        self.strength = strength;
        // Restart from the currently presented position, without a visual jump.
        self.from = self.current;
        self.elapsed = Duration::ZERO;
        Ok(())
    }

    /// Zero duration finishes immediately. Changes preserve the current position.
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
        self.from = self.current;
        self.elapsed = Duration::ZERO;
        if duration.is_zero() {
            self.finish();
        }
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced {
            self.finish();
        }
    }

    pub fn set_target(&mut self, target: u64) -> Result<bool, TickingNumberError> {
        if target >= 10_u64.pow(self.digits as u32) {
            return Err(TickingNumberError::Value);
        }
        if target == self.target {
            return Ok(false);
        }
        self.from = self.current;
        self.target = target;
        self.elapsed = Duration::ZERO;
        if self.reduced_motion || self.duration.is_zero() {
            self.finish();
        }
        Ok(true)
    }

    fn finish(&mut self) {
        self.current = self.target as f64;
    }

    /// Advance with actual elapsed time. Long/dropped frames land exactly on the target.
    pub fn advance(&mut self, delta: Duration) -> bool {
        if delta.is_zero() || !self.needs_animation() {
            return false;
        }
        self.elapsed = self.elapsed.saturating_add(delta);
        if self.elapsed >= self.duration {
            self.finish();
            return true;
        }
        let t = self.elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let smooth = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let progress = t + self.strength * (smooth - t);
        self.current = self.from + (self.target as f64 - self.from) * progress;
        true
    }

    /// Measure all ten glyphs, never only the currently visible value. A slot is
    /// the widest glyph advance; each glyph is centered independently in its slot.
    /// Thus even a proportional host font has identical digit placement and width.
    pub fn measure(&self, text: &mut dyn ButtonTextRenderer, role: proposal::TextRole) -> Size {
        let mut cell = Size::new(0.0, 0.0);
        for digit in '0'..='9' {
            let size = text.measure(&digit.to_string(), role, f32::MAX);
            cell.width = cell.width.max(size.width);
            cell.height = cell.height.max(size.height);
        }
        Size::new(cell.width * self.digits as f32, cell.height)
    }

    /// Paint at the reserved natural width. The consumer can use `measure` for
    /// layout; scaling/font changes require remeasurement, value changes do not.
    pub fn draw(
        &self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        origin: tekes_ui_scene::Point,
        role: proposal::TextRole,
        ink: Rgba,
    ) -> Rect {
        let size = self.measure(text, role);
        let width = size.width / self.digits as f32;
        for (index, digit) in self.text().chars().enumerate() {
            text.draw(
                scene,
                &digit.to_string(),
                role,
                Rect::new(
                    origin.x + index as f32 * width,
                    origin.y,
                    width,
                    size.height,
                ),
                ButtonTextAlign::Center,
                ink,
            );
        }
        Rect::new(origin.x, origin.y, size.width, size.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accelerates_then_decelerates_and_lands_in_both_directions() {
        for (a, b) in [(137, 546), (546, 137)] {
            let mut n = TickingNumber::new(a, 3).unwrap();
            n.set_duration(Duration::from_secs(1));
            n.set_target(b).unwrap();
            let mut steps = vec![];
            for _ in 0..10 {
                let before = n.current;
                n.advance(Duration::from_millis(100));
                steps.push((n.current - before).abs());
                assert!((a.min(b)..=a.max(b)).contains(&n.value()));
            }
            assert!(steps[0] < steps[1] && steps[1] < steps[4]);
            assert!(steps[5] > steps[8] && steps[8] > steps[9]);
            assert_eq!(n.value(), b);
            assert!(!n.needs_animation());
        }
    }
    #[test]
    fn fixed_digits_validation_interrupt_and_reduction() {
        assert!(TickingNumber::new(0, 0).is_err());
        assert!(TickingNumber::new(100, 2).is_err());
        let mut n = TickingNumber::new(7, 3).unwrap();
        assert_eq!(n.text(), "007");
        assert!(n.set_target(1000).is_err());
        assert!(n.set_strength(f64::NAN).is_err());
        n.set_target(546).unwrap();
        n.advance(Duration::from_millis(300));
        let before = n.current;
        n.set_target(137).unwrap();
        assert_eq!(before, n.current);
        n.set_reduced_motion(true);
        assert_eq!(n.value(), 137);
        n.set_target(999).unwrap();
        assert_eq!(n.text(), "999");
        assert!(!n.needs_animation());
    }
    #[test]
    fn strength_zero_is_linear_and_dropped_frames_finish() {
        let mut n = TickingNumber::new(0, 3).unwrap();
        n.set_duration(Duration::from_secs(1));
        n.set_strength(0.0).unwrap();
        n.set_target(500).unwrap();
        n.advance(Duration::from_millis(100));
        assert_eq!(n.value(), 50);
        n.advance(Duration::MAX);
        assert_eq!(n.value(), 500);
        assert!(!n.needs_animation());
    }
    #[test]
    fn proportional_glyphs_reserve_identical_slots_for_every_value() {
        #[derive(Default)]
        struct Text {
            cells: Vec<Rect>,
        }
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
                Size::new(if text == "1" { 4.0 } else { 12.0 }, 24.0)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                rect: Rect,
                align: ButtonTextAlign,
                _: Rgba,
            ) {
                assert_eq!(align, ButtonTextAlign::Center);
                self.cells.push(rect);
            }
        }
        let mut text = Text::default();
        let mut n = TickingNumber::new(111, 3).unwrap();
        n.set_duration(Duration::ZERO);
        let mut previous = None;
        for value in 0..1000 {
            n.set_target(value).unwrap();
            text.cells.clear();
            let bounds = n.draw(
                &mut Scene::new(),
                &mut text,
                tekes_ui_scene::Point::new(10.0, 20.0),
                proposal::TextRole::UiBody,
                Rgba::new(0.0, 0.0, 0.0, 1.0),
            );
            assert_eq!(bounds.size.width, 36.0);
            assert_eq!(text.cells.len(), 3);
            assert!(text.cells.iter().all(|r| r.size.width == 12.0));
            if let Some(ref cells) = previous {
                assert_eq!(cells, &text.cells);
            }
            previous = Some(text.cells.clone());
        }
    }
}
