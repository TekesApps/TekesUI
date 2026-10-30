//! Reuse native CoreText shaping/truncation, font registration and glyph output.
use super::shaper::{
    CoreTextShaper, apply_label_foregrounds, grapheme_len_at, line_height, line_metrics, runs_of,
    typeset,
};
use crate::{
    LabelLayout, LabelOptions, LabelOverflow, Line, ShapeOptions, Shaper, TextStyle,
    utf16::Utf16Map,
};
use objc2_core_foundation::{
    CFDictionary, CFMutableAttributedString, CFRange, CFRetained, CFString,
};
use objc2_core_text::{CTFont, CTLine, CTLineTruncationType};
use std::{ffi::c_void, ops::Range};
const MARKER: &str = "TekesUI.Label.SyntheticEllipsis";
fn mark(text: &CFMutableAttributedString, range: CFRange) {
    // SAFETY: caller owns the attributed string and provides an in-bounds token range.
    unsafe {
        CFMutableAttributedString::set_attribute(
            Some(text),
            range,
            Some(&CFString::from_str(MARKER)),
            Some(CFString::from_str("ellipsis").as_ref()),
        );
    }
}
fn marker(line: &CTLine) -> Vec<bool> {
    runs_of(line)
        .into_iter()
        .map(|r| {
            // SAFETY: attributes is a valid immutable dictionary; custom marker is not a pointer payload.
            unsafe {
                let attrs: CFRetained<CFDictionary> = r.attributes();
                let key = CFString::from_str(MARKER);
                !attrs
                    .value(&*key as *const CFString as *const c_void)
                    .is_null()
            }
        })
        .collect()
}
fn width(line: &CTLine, base: &CTFont) -> f64 {
    line_metrics(line, base).width
}
fn empty_line(template: &Line, base: &CTFont) -> Line {
    let ts = typeset("", base);
    // SAFETY: empty range is valid for this empty typesetter.
    let native = unsafe { ts.typesetter.line(CFRange::new(0, 0)) };
    let m = line_metrics(&native, base);
    Line {
        index: template.index,
        utf16: template.utf16.clone(),
        bytes: template.bytes.clone(),
        ascent: m.ascent,
        descent: m.descent,
        leading: m.leading,
        height: line_height(&m),
        top: 0.,
        baseline_y: m.ascent,
        width: 0.,
        width_trimmed: 0.,
        runs: Vec::new(),
        carets: Vec::new(),
        has_newline: false,
    }
}
impl CoreTextShaper {
    /// Shape an opt-in read-only label. Existing `shape` / `ShapeOptions` are
    /// unchanged. Zero/invalid constrained widths yield deterministic empty
    /// ellipsis lines; hard breaks remain real line boundaries.
    pub fn shape_label(
        &mut self,
        text: &str,
        style: &TextStyle,
        shape: &ShapeOptions,
        options: LabelOptions,
    ) -> LabelLayout {
        self.shape_label_inner(text, style, shape, options, None)
    }

    /// One native attributed paragraph; semantic boundaries participate in the
    /// original shaper and truncation, never a consumer's split-label layout.
    pub fn shape_semantic_label(
        &mut self,
        label: &crate::SemanticLabel,
        style: &TextStyle,
        shape: &ShapeOptions,
        options: LabelOptions,
    ) -> LabelLayout {
        self.shape_label_inner(label.text(), style, shape, options, Some(label))
    }

    fn shape_label_inner(
        &mut self,
        text: &str,
        style: &TextStyle,
        shape: &ShapeOptions,
        options: LabelOptions,
        semantic: Option<&crate::SemanticLabel>,
    ) -> LabelLayout {
        let mut paragraph = match semantic {
            Some(label) => self.shape_prepared(&self.prepare_semantic_label(label, style), shape),
            None => self.shape(text, style, shape),
        };
        if options == LabelOptions::default() {
            let width = paragraph.width;
            return LabelLayout {
                source: text.into(),
                paragraph,
                width,
                elided_utf16: Vec::new(),
            };
        }
        let available = shape.width.map(|w| {
            if w == f64::INFINITY {
                f64::MAX
            } else if w.is_finite() {
                w.max(0.)
            } else {
                0.
            }
        });
        let limit = options.max_lines.map_or(paragraph.lines.len(), |n| n.get());
        let hidden_lines = paragraph.lines.len() > limit;
        paragraph.lines.truncate(limit);
        let original_len = text.encode_utf16().count();
        let map = Utf16Map::new(text);
        let cf = CFString::from_str(text);
        let mut boundaries = vec![0];
        let mut at = 0;
        while at < original_len {
            at += grapheme_len_at(&cf, at);
            boundaries.push(at);
        }
        let base = self.fonts().create_font(style);
        let base_key = self.fonts().key_for(&base);
        let mut elided = Vec::new();
        let last = paragraph.lines.len().saturating_sub(1);
        for index in 0..paragraph.lines.len() {
            let overflow = available.is_some_and(|w| paragraph.lines[index].width > w);
            let hidden = index == last && hidden_lines;
            if options.overflow == LabelOverflow::Clip {
                if hidden {
                    let line = &paragraph.lines[index];
                    let visible = text[line.bytes.clone()]
                        .trim_end_matches(['\n', '\r', '\u{2028}', '\u{2029}']);
                    let cut = line.utf16.start + visible.encode_utf16().count();
                    let line = &mut paragraph.lines[index];
                    for run in &mut line.runs {
                        run.glyphs.retain(|g| g.utf16_index < cut);
                        run.utf16.end = run.utf16.end.min(cut);
                    }
                    line.runs.retain(|run| !run.glyphs.is_empty());
                    line.utf16.end = cut;
                    line.bytes.end = map.byte_of(cut);
                    line.carets.retain(|(index, _)| *index <= cut);
                    line.has_newline = false;
                    elided.push(cut..original_len);
                }
                continue;
            }
            if !hidden && !overflow {
                continue;
            }
            let old = paragraph.lines[index].clone();
            let start = old.utf16.start;
            let remainder = &text[map.byte_of(start)..];
            let segment_bytes = if hidden {
                remainder
                    .find(['\n', '\r', '\u{2028}', '\u{2029}'])
                    .unwrap_or(remainder.len())
            } else {
                text[old.bytes.clone()]
                    .trim_end_matches(['\n', '\r', '\u{2028}', '\u{2029}'])
                    .len()
            };
            let segment = &remainder[..segment_bytes];
            let mut end = start + segment.encode_utf16().count();
            let omitted_end = if hidden { original_len } else { old.utf16.end };
            let token_ts = typeset("…", &base);
            mark(&token_ts.attributed, CFRange::new(0, 1));
            // SAFETY: token contains exactly one UTF-16 unit and retained font attributes.
            let token = unsafe { CTLine::with_attributed_string(&token_ts.attributed) };
            if available.is_some_and(|w| w < width(&token, &base) || w <= 0.) {
                paragraph.lines[index] = empty_line(&old, &base);
                elided.push(start..omitted_end);
                continue;
            }
            loop {
                // Last visible line is shaped as one line; inserting only the
                // synthetic token preserves original source and bidi ordering.
                let prefix = &text[map.byte_of(start)..map.byte_of(end)];
                let count = end - start;
                let candidate = typeset(&format!("{prefix}…"), &base);
                if let Some(label) = semantic {
                    apply_label_foregrounds(&candidate.attributed, label, start..end);
                }
                mark(&candidate.attributed, CFRange::new(count as isize, 1));
                // SAFETY: complete immutable candidate after marker publication; a typesetter
                // constructed before the attribute edit would hold an older snapshot.
                let line = unsafe { CTLine::with_attributed_string(&candidate.attributed) };
                let result = if let Some(w) = available.filter(|w| width(&line, &base) > *w) {
                    // SAFETY: finite nonnegative width and valid retained token.
                    unsafe { line.truncated_line(w, CTLineTruncationType::End, Some(&token)) }
                } else {
                    Some(line)
                };
                let Some(result) = result else {
                    paragraph.lines[index] = empty_line(&old, &base);
                    elided.push(start..omitted_end);
                    break;
                };
                let flags = marker(&result);
                let mut runs =
                    self.collect_runs(&result, base_key, &mut paragraph.fonts, count + 1);
                let cut = runs
                    .iter()
                    .zip(&flags)
                    .filter(|(_, synthetic)| **synthetic)
                    .map(|(r, _)| start + r.utf16.start.min(count))
                    .min()
                    .unwrap_or(end);
                // SAFETY: immutable native token; glyph count may exceed one
                // for a fallback font, even though U+2026 is one grapheme.
                let expected_token_glyphs = unsafe { token.glyph_count() as usize };
                let synthetic_glyphs = runs
                    .iter()
                    .zip(&flags)
                    .filter(|(_, s)| **s)
                    .map(|(r, _)| r.glyphs.len())
                    .sum::<usize>();
                // Native truncation is authoritative for shaping; source
                // grapheme boundaries and one synthetic token remain checked.
                if !boundaries.contains(&cut) || synthetic_glyphs != expected_token_glyphs {
                    let previous = boundaries
                        .iter()
                        .copied()
                        .rfind(|p| *p < end && *p >= start);
                    if let Some(previous) = previous {
                        end = previous;
                        continue;
                    }
                    paragraph.lines[index] = empty_line(&old, &base);
                    elided.push(start..omitted_end);
                    break;
                }
                for (run, synthetic) in runs.iter_mut().zip(flags) {
                    if synthetic {
                        run.utf16 = cut..cut;
                        for g in &mut run.glyphs {
                            g.utf16_index = cut;
                            g.cluster_utf16 = cut..cut;
                        }
                    } else {
                        run.utf16 = (start + run.utf16.start.min(count))
                            ..(start + run.utf16.end.min(count));
                        for g in &mut run.glyphs {
                            g.utf16_index = start + g.utf16_index.min(count);
                            g.cluster_utf16 = (start + g.cluster_utf16.start.min(count))
                                ..(start + g.cluster_utf16.end.min(count));
                        }
                    }
                }
                let m = line_metrics(&result, &base);
                let end = if hidden { original_len } else { old.utf16.end };
                paragraph.lines[index] = Line {
                    index,
                    utf16: start..end,
                    bytes: map.byte_range(start..end),
                    ascent: m.ascent,
                    descent: m.descent,
                    leading: m.leading,
                    height: line_height(&m),
                    top: 0.,
                    baseline_y: m.ascent,
                    width: m.width,
                    width_trimmed: m.width - m.trailing_whitespace,
                    runs,
                    carets: Vec::new(),
                    has_newline: false,
                };
                elided.push(cut..omitted_end);
                break;
            }
        }
        let spacing = if shape.line_spacing.is_finite() {
            shape.line_spacing.max(0.)
        } else {
            0.
        };
        let mut y = 0.;
        for line in &mut paragraph.lines {
            line.top = y;
            line.baseline_y = y + line.ascent;
            y += line.height + spacing;
        }
        paragraph.height = paragraph.default_line_height.max(y - spacing);
        paragraph.width = paragraph.lines.iter().map(|l| l.width).fold(0., f64::max);
        let measured = available.map_or(paragraph.width, |w| paragraph.width.min(w));
        elided.retain(|r: &Range<usize>| r.start < r.end);
        LabelLayout {
            source: text.into(),
            paragraph,
            width: measured,
            elided_utf16: elided,
        }
    }
}
