//! 三个平台无关的 trait。mac 用 CoreText 实现，以后 Windows 用 DirectWrite，Linux 用 Parley。

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
}

/// 光栅化：一个字形在某个 scale 下的位图。
///
/// 每个 (字体, 字形, scale) 只需要光栅化一次，缓存由 atlas 负责。
/// 没有墨迹的字形（空格）和失败都返回 `None`。
pub trait Rasterizer {
    fn rasterize(&mut self, font: FontKey, glyph: u16, scale: f32) -> Option<RasterGlyph>;
}
