//! Caret, hit-test and selection rectangles share the very same shaped line boxes.
use crate::Paragraph;
use std::ops::Range;
use tekes_ui_scene::{Point, Rect};
impl Paragraph {
    pub fn caret_rect(&self, index: usize, width: f32) -> Rect {
        let Some(line) = self
            .lines
            .iter()
            .find(|l| index >= l.utf16.start && index < l.utf16.end)
            .or_else(|| self.lines.last())
        else {
            return Rect::default();
        };
        let x = line
            .carets
            .iter()
            .find(|v| v.0 == index)
            .map_or(line.width, |v| v.1);
        Rect::new(x as f32, line.top as f32, width, line.height as f32)
    }
    /// Hit-test a document-local point. An explicit newline belongs to its old
    /// line; clicking to the right of text must not jump to the next line.
    pub fn hit_test(&self, point: Point) -> usize {
        let Some(line) = self
            .lines
            .iter()
            .find(|l| f64::from(point.y) < l.top + l.height)
            .or_else(|| self.lines.last())
        else {
            return 0;
        };
        line.carets
            .iter()
            .filter(|v| !line.has_newline || v.0 < line.utf16.end)
            .min_by(|a, b| {
                (a.1 - f64::from(point.x))
                    .abs()
                    .total_cmp(&(b.1 - f64::from(point.x)).abs())
            })
            .map_or(line.utf16.start, |v| v.0)
    }
    /// Tight advance-width rectangles per selected visual span. Explicit newline
    /// selection fills to the viewport edge; soft wraps do not add artificial padding.
    pub fn selection_rects(&self, range: Range<usize>, viewport: Rect) -> Vec<Rect> {
        let mut out = Vec::new();
        for line in &self.lines {
            let y = line.top as f32;
            let height = line.height as f32;
            if range.end <= line.utf16.start
                || range.start >= line.utf16.end
                || y + height < viewport.min_y()
                || y > viewport.max_y()
            {
                continue;
            }
            let mut intervals: Vec<(f32, f32)> = line
                .carets
                .windows(2)
                .filter(|pair| pair[0].0 >= range.start && pair[0].0 < range.end)
                .map(|pair| {
                    let a = pair[0].1 as f32;
                    let b = if line.has_newline && pair[1].0 == line.utf16.end {
                        viewport.max_x().max(a + 1.0)
                    } else {
                        pair[1].1 as f32
                    };
                    (a.min(b), a.max(b))
                })
                .filter(|(a, b)| b > a)
                .collect();
            intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f32, f32)> = Vec::new();
            for (x, end) in intervals {
                if let Some(last) = merged.last_mut()
                    && x <= last.1 + 0.01
                {
                    last.1 = last.1.max(end);
                } else {
                    merged.push((x, end));
                }
            }
            for (x, end) in merged {
                out.push(Rect::new(x, y, end - x, height));
            }
        }
        out
    }
    /// Single-line controls center their typographic line box vertically. Multiline
    /// documents remain top aligned. Text is left aligned to the padded content edge.
    pub fn origin_in(&self, viewport: Rect, center_vertically: bool) -> Point {
        Point::new(
            viewport.min_x(),
            viewport.min_y()
                + if center_vertically {
                    (viewport.size.height - self.height as f32).max(0.0) * 0.5
                } else {
                    0.0
                },
        )
    }
}
