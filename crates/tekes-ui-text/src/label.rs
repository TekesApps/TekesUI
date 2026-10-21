//! Opt-in, read-only label layout. The original source remains separate from
//! visible glyphs; this result is not an editable caret/selection document.
use crate::Paragraph;
use std::{num::NonZeroUsize, ops::Range};
/// Behaviour at the last visible line, or an indivisible glyph wider than its slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelOverflow {
    #[default]
    Clip,
    TailEllipsis,
}
/// Defaults preserve the existing unlimited wrapping path. Limits are positive;
/// caller-provided zero cannot silently turn into an arbitrary line count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LabelOptions {
    pub max_lines: Option<NonZeroUsize>,
    pub overflow: LabelOverflow,
}
impl LabelOptions {
    pub fn single_line(overflow: LabelOverflow) -> Self {
        Self {
            max_lines: NonZeroUsize::new(1),
            overflow,
        }
    }
}
/// Immutable label result: measure and glyph paint consume the same paragraph.
/// Synthetic ellipsis glyphs have a zero-length source cluster at their cut;
/// `elided_utf16` records omitted source. Never use this as editor geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelLayout {
    pub(crate) source: String,
    pub(crate) paragraph: Paragraph,
    pub(crate) width: f64,
    pub(crate) elided_utf16: Vec<Range<usize>>,
}
impl LabelLayout {
    /// Apply the host design system's minimum line box without changing glyph
    /// advances, visible rows, source identity or native fallback-font metrics.
    pub fn with_min_line_height(mut self, minimum: f64) -> Self {
        if !minimum.is_finite() || minimum <= 0. {
            return self;
        }
        let mut top = 0.;
        for line in &mut self.paragraph.lines {
            let height = line.height.max(minimum);
            line.top = top;
            line.height = height;
            line.baseline_y = top + (height - line.ascent - line.descent) * 0.5 + line.ascent;
            top += height;
        }
        self.paragraph.height = top.max(minimum);
        self.paragraph.default_line_height = minimum;
        self
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn paragraph(&self) -> &Paragraph {
        &self.paragraph
    }
    /// Slot-constrained measured width; Clip may retain glyphs beyond it.
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.paragraph.height
    }
    pub fn elided_utf16(&self) -> &[Range<usize>] {
        &self.elided_utf16
    }
    pub fn is_truncated(&self) -> bool {
        !self.elided_utf16.is_empty() || self.paragraph.width > self.width
    }
}
