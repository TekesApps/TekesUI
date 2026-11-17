//! 把排版结果编进 [`Scene`]。

use crate::model::Paragraph;
use crate::traits::Rasterizer;
use tekes_ui_scene::{GlyphAtlas, GlyphKey, GlyphRun, Point, PositionedGlyph, Scene};
use tekes_ui_tokens::Rgba;

/// 把一个段落编成 scene 里的字形图元：每行一个 [`GlyphRun`]。
///
/// - `origin`：段落左上角（pt），第 `i` 行的字形原点是
///   `origin + (glyph.x, line.baseline_y + glyph.y)`，**不取整**；渲染层按
///   `round(origin * scale) + (left, top)` 把位图对齐到设备像素；
/// - `scale`：逻辑点到设备像素的比例，决定光栅化的尺寸和 atlas 键；
/// - 光栅化只在 atlas 未命中时发生（每个 字体 / 字形 / scale 一次）；没有位图的字形（空格）
///   不进 `GlyphRun`；
/// - 调用前先 `atlas.begin_frame()`；一个 scene 里的 `AtlasGlyph` 只在编出它的那一帧有效。
pub fn push_paragraph<R: Rasterizer + ?Sized>(
    scene: &mut Scene,
    atlas: &mut GlyphAtlas,
    rasterizer: &mut R,
    paragraph: &Paragraph,
    origin: Point,
    scale: f32,
    color: Rgba,
) {
    push_paragraph_visible(
        scene,
        atlas,
        rasterizer,
        paragraph,
        origin,
        scale,
        color,
        f32::NEG_INFINITY..f32::INFINITY,
    );
}

/// Rasterize only visible lines; a one-line overscan handles tall fallback glyphs.
#[allow(clippy::too_many_arguments)]
pub fn push_paragraph_visible<R: Rasterizer + ?Sized>(
    scene: &mut Scene,
    atlas: &mut GlyphAtlas,
    rasterizer: &mut R,
    paragraph: &Paragraph,
    origin: Point,
    scale: f32,
    color: Rgba,
    visible_y: std::ops::Range<f32>,
) {
    if paragraph.lines.is_empty()
        || visible_y.start.is_nan()
        || visible_y.end.is_nan()
        || visible_y.start > visible_y.end
        || !origin.x.is_finite()
        || !origin.y.is_finite()
    {
        return;
    }
    let start = paragraph
        .lines
        .partition_point(|line| origin.y + ((line.top + line.height) as f32) < visible_y.start);
    let end = paragraph
        .lines
        .partition_point(|line| origin.y + line.top as f32 <= visible_y.end);
    let end = (end + 1).min(paragraph.lines.len());
    let start = start.saturating_sub(1).min(end);
    let scale_key = GlyphKey::scale_key_for(scale);
    for line in &paragraph.lines[start..end] {
        let mut glyphs = Vec::new();
        for run in &line.runs {
            for g in &run.glyphs {
                let key = GlyphKey::new(run.font, g.id, scale_key);
                let Some(atlas_glyph) =
                    atlas.get_or_insert_with(key, || rasterizer.rasterize(run.font, g.id, scale))
                else {
                    continue;
                };
                glyphs.push(PositionedGlyph {
                    origin: Point::new(
                        origin.x + g.x as f32,
                        origin.y + (line.baseline_y + g.y) as f32,
                    ),
                    glyph: atlas_glyph,
                });
            }
        }
        if !glyphs.is_empty() {
            scene.push_glyphs(GlyphRun { color, glyphs });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Line, Paragraph, ShapedGlyph, ShapedRun};
    use tekes_ui_scene::{FontKey, RasterFormat, RasterGlyph};

    struct CountingRasterizer(usize);
    impl Rasterizer for CountingRasterizer {
        fn rasterize(&mut self, _: FontKey, _: u16, _: f32) -> Option<RasterGlyph> {
            self.0 += 1;
            Some(RasterGlyph {
                left: 0,
                top: 0,
                width: 1,
                height: 1,
                format: RasterFormat::Mask,
                pixels: vec![255, 255, 255, 255],
            })
        }
    }

    fn dummy_line(index: usize, top: f64) -> Line {
        Line {
            index,
            utf16: index..index + 1,
            bytes: index..index + 1,
            ascent: 10.0,
            descent: 2.0,
            leading: 0.0,
            height: 16.0,
            top,
            baseline_y: top + 10.0,
            width: 8.0,
            width_trimmed: 8.0,
            runs: vec![ShapedRun {
                font: FontKey(1),
                utf16: index..index + 1,
                glyphs: vec![ShapedGlyph {
                    id: (index + 1) as u16,
                    x: 0.0,
                    y: 0.0,
                    utf16_index: index,
                    cluster_utf16: index..index + 1,
                }],
            }],
            carets: vec![],
            graphemes: vec![],
            has_newline: false,
        }
    }

    #[test]
    fn push_paragraph_visible_handles_reversed_nan_and_narrow_ranges() {
        let paragraph = Paragraph {
            lines: vec![dummy_line(0, 0.0), dummy_line(1, 20.0), dummy_line(2, 40.0)],
            width: 8.0,
            height: 56.0,
            default_line_height: 16.0,
            fonts: vec![],
            utf16_len: 3,
        };
        let mut atlas = GlyphAtlas::default();
        for range in [
            100.0..0.0,
            f32::NAN..50.0,
            0.0..f32::NAN,
            500.0..600.0,
            -200.0..-100.0,
        ] {
            let mut scene = Scene::new();
            let mut raster = CountingRasterizer(0);
            push_paragraph_visible(
                &mut scene,
                &mut atlas,
                &mut raster,
                &paragraph,
                Point::new(0.0, 0.0),
                1.0,
                Rgba::gray(1.0),
                range.clone(),
            );
            if range.start.is_nan() || range.end.is_nan() || range.start > range.end {
                assert_eq!(raster.0, 0);
                assert!(scene.primitives().is_empty());
            }
        }
    }
}
