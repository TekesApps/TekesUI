//! 三个平台无关的 trait。mac 用 CoreText 实现，以后 Windows 用 DirectWrite，Linux 用 Parley。

use crate::label::{LabelLayout, LabelOptions, SemanticLabel};
use crate::model::{FontRun, Paragraph, ShapeOptions};
use crate::style::TextStyle;
use tekes_ui_scene::{FontKey, RasterGlyph};

/// 字体回退：一段文字在给定样式下实际落在哪些字体上。
///
/// 实现必须使用平台自己的自动回退（mac 上是 CoreText 的 cascade），
/// 不自建回退列表，这样才和系统控件、参照实现一致。
pub trait FontFallback {
    /// 返回覆盖整段文本的字体段，按文本顺序，相邻同字体的段已合并。
    fn font_runs(&mut self, text: &str, style: &TextStyle) -> Vec<FontRun>;
}

/// 排版：把一段文字（单一样式）按宽度断行，并给出每行的度量和字形。
pub trait Shaper {
    fn shape(&mut self, text: &str, style: &TextStyle, options: &ShapeOptions) -> Paragraph;

    /// Format an opt-in read-only label using this shaper backend.
    ///
    /// Default implementation delegates to [`Shaper::shape`] and enforces positive line limits
    /// across platforms; native backends such as CoreText override this to supply native truncation.
    fn shape_label(
        &mut self,
        text: &str,
        style: &TextStyle,
        shape: &ShapeOptions,
        options: LabelOptions,
    ) -> LabelLayout {
        shape_label_with_shaper(self, text, style, shape, options)
    }

    /// Format an opt-in semantic label with mixed inline spans across platforms.
    fn shape_semantic_label(
        &mut self,
        label: &SemanticLabel,
        style: &TextStyle,
        shape: &ShapeOptions,
        options: LabelOptions,
    ) -> LabelLayout {
        self.shape_label(label.text(), style, shape, options)
    }
}

/// Platform-neutral prepared text payload for shapers that do not require an OS typesetter handle.
#[derive(Clone, Debug, PartialEq)]
pub struct GenericPreparedText {
    pub text: String,
    pub style: TextStyle,
}

/// Reusable pre-shaped text document contract across platform backends.
pub trait PreparedShaper: Shaper {
    type Prepared;

    fn prepare(&self, text: &str, style: &TextStyle) -> Self::Prepared;

    fn shape_prepared(&mut self, prepared: &Self::Prepared, options: &ShapeOptions) -> Paragraph;
}

fn shape_label_with_shaper<S: Shaper + ?Sized>(
    shaper: &mut S,
    text: &str,
    style: &TextStyle,
    shape: &ShapeOptions,
    options: LabelOptions,
) -> LabelLayout {
    let mut paragraph = shaper.shape(text, style, shape);
    let total_utf16 = paragraph
        .lines
        .last()
        .map_or_else(|| text.encode_utf16().count(), |line| line.utf16.end);
    let mut elided_utf16 = Vec::new();
    if let Some(max_lines) = options.max_lines
        && paragraph.lines.len() > max_lines.get()
    {
        let keep = max_lines.get();
        let cut_utf16 = paragraph.lines[keep].utf16.start;
        paragraph.lines.truncate(keep);
        paragraph.width = paragraph
            .lines
            .iter()
            .map(|line| line.width)
            .fold(0.0, f64::max);
        let spacing = shape.line_spacing.max(0.0);
        let lines_height = paragraph.lines.iter().map(|line| line.height).sum::<f64>();
        let gaps = spacing * (keep.saturating_sub(1) as f64);
        paragraph.height = (lines_height + gaps).max(paragraph.default_line_height);
        if cut_utf16 < total_utf16 {
            elided_utf16.push(cut_utf16..total_utf16);
        }
    }
    let width = shape.width.unwrap_or(paragraph.width).max(0.0);
    LabelLayout::new(
        text,
        paragraph,
        width,
        shape.line_spacing.max(0.0),
        elided_utf16,
    )
}

/// 光栅化：一个字形在某个 scale 下的位图。
///
/// 每个 (字体, 字形, scale) 只需要光栅化一次，缓存由 atlas 负责。
/// 没有墨迹的字形（空格）和失败都返回 `None`。
pub trait Rasterizer {
    fn rasterize(&mut self, font: FontKey, glyph: u16, scale: f32) -> Option<RasterGlyph>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::label::LabelSourceVersion;
    use crate::model::{GraphemeSpan, Line};

    #[derive(Default)]
    struct MockShaper;

    impl Shaper for MockShaper {
        fn shape(&mut self, text: &str, style: &TextStyle, _options: &ShapeOptions) -> Paragraph {
            let mut lines = Vec::new();
            let mut top = 0.0;
            let height = f64::from(style.size) * 1.25;
            let mut offset = 0usize;
            for (index, part) in text.split('\n').enumerate() {
                let start = offset;
                let end = start + part.len();
                let u_start = text[..start].encode_utf16().count();
                let u_end = text[..end].encode_utf16().count();
                let width = (part.chars().count() as f64) * 8.0;
                lines.push(Line {
                    index,
                    utf16: u_start..u_end,
                    bytes: start..end,
                    ascent: height * 0.8,
                    descent: height * 0.2,
                    leading: 0.0,
                    height,
                    top,
                    baseline_y: top + height * 0.8,
                    width,
                    width_trimmed: width,
                    runs: Vec::new(),
                    carets: vec![(u_start, 0.0), (u_end, width)],
                    graphemes: if part.is_empty() {
                        Vec::new()
                    } else {
                        vec![GraphemeSpan {
                            start: u_start,
                            end: u_end,
                            x_start: 0.0,
                            x_end: width,
                        }]
                    },
                    has_newline: end < text.len(),
                });
                top += height;
                offset = end + usize::from(end < text.len());
            }
            Paragraph {
                width: lines.iter().map(|l| l.width).fold(0.0, f64::max),
                height: top.max(height),
                default_line_height: height,
                lines,
                fonts: Vec::new(),
                utf16_len: text.encode_utf16().count(),
            }
        }
    }

    impl PreparedShaper for MockShaper {
        type Prepared = GenericPreparedText;

        fn prepare(&self, text: &str, style: &TextStyle) -> Self::Prepared {
            GenericPreparedText {
                text: text.to_owned(),
                style: style.clone(),
            }
        }

        fn shape_prepared(
            &mut self,
            prepared: &Self::Prepared,
            options: &ShapeOptions,
        ) -> Paragraph {
            self.shape(&prepared.text, &prepared.style, options)
        }
    }

    #[test]
    fn generic_shaper_label_and_prepared_boundaries_work_without_coretext() {
        let mut shaper = MockShaper;
        let style = TextStyle::system(14.0);
        let prepared = PreparedShaper::prepare(&shaper, "first\nsecond\nthird", &style);
        let paragraph = shaper.shape_prepared(&prepared, &ShapeOptions::with_width(120.0));
        assert_eq!(paragraph.lines.len(), 3);

        let label = shaper.shape_label(
            "first\nsecond\nthird",
            &style,
            &ShapeOptions::with_width(120.0),
            LabelOptions {
                max_lines: std::num::NonZeroUsize::new(2),
                overflow: crate::label::LabelOverflow::TailEllipsis,
            },
        );
        assert_eq!(label.source(), "first\nsecond\nthird");
        assert_eq!(label.paragraph().lines.len(), 2);
        assert_eq!(label.width(), 120.0);
        assert_eq!(label.elided_utf16(), std::slice::from_ref(&(13..18)));
        assert!(label.is_truncated());

        let semantic = SemanticLabel::new(
            LabelSourceVersion {
                source: 1,
                revision: 1,
            },
            "alpha\nbeta",
            Vec::new(),
        )
        .unwrap();
        let semantic_layout = shaper.shape_semantic_label(
            &semantic,
            &style,
            &ShapeOptions::with_width(80.0),
            LabelOptions::default(),
        );
        assert_eq!(semantic_layout.source(), "alpha\nbeta");
        assert_eq!(semantic_layout.paragraph().lines.len(), 2);
    }
}
