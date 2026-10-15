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
    let start = paragraph
        .lines
        .partition_point(|line| origin.y + ((line.top + line.height) as f32) < visible_y.start);
    let end = paragraph
        .lines
        .partition_point(|line| origin.y + line.top as f32 <= visible_y.end);
    let start = start.saturating_sub(1);
    let end = (end + 1).min(paragraph.lines.len());
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
