//! 排版结果的数据模型。平台无关：CoreText、DirectWrite、Parley 的实现都产出这些类型。
//!
//! # 单位与坐标
//!
//! - 长度、位置一律是逻辑点（pt），用 `f64`（CoreText 本身给 `f64`，不在这里损失精度）。
//! - 文本偏移有两套：UTF-16 偏移（`utf16`，与 NSString/CoreText 一致，导出格式用这个）
//!   和 UTF-8 字节偏移（`bytes`，用来切 Rust 的 `&str`）。两者都是半开区间。
//! - 段落坐标系：原点在段落左上角，x 向右，y 向下。
//! - 字形位置 `x`、`y` 相对**行原点**（基线的左端）：`x` 向右，`y` 向下，正常字形 `y` 为 0。

use std::ops::Range;
use tekes_ui_scene::FontKey;

/// 排版选项。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeOptions {
    /// 换行宽度（pt）。`None` 表示不换行，只在硬换行（`\n` 等）处断行。
    pub width: Option<f64>,
    /// 每行之后追加的行间距（pt），下限 0。没有段间距。
    pub line_spacing: f64,
}

impl Default for ShapeOptions {
    fn default() -> ShapeOptions {
        ShapeOptions {
            width: None,
            line_spacing: 0.0,
        }
    }
}

impl ShapeOptions {
    pub fn with_width(width: f64) -> ShapeOptions {
        ShapeOptions {
            width: Some(width),
            line_spacing: 0.0,
        }
    }
}

/// 一个字体的描述。
#[derive(Clone, Debug, PartialEq)]
pub struct FontInfo {
    pub key: FontKey,
    /// PostScript 名，比如 `.SFNS-Regular`、`PingFangSC-Regular`、`AppleColorEmoji`。
    pub post_script_name: String,
    /// 字号（pt）。
    pub size: f64,
    /// 是彩色字体（CoreText 的 `kCTFontTraitColorGlyphs`）。
    pub is_color: bool,
}

/// 字体回退的一段：一段连续文本落在同一个字体上。
#[derive(Clone, Debug, PartialEq)]
pub struct FontRun {
    pub utf16: Range<usize>,
    pub bytes: Range<usize>,
    pub font: FontInfo,
}

/// 排版出来的一个字形。
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedGlyph {
    /// 字形编号（所在字体里的编号）。
    pub id: u16,
    /// 字形原点相对行原点的位置（pt）。
    pub x: f64,
    pub y: f64,
    /// 这个字形对应的 UTF-16 起点（CTRun 的 string index）。
    pub utf16_index: usize,
    /// 这个字形所属的簇的 UTF-16 范围：从 `utf16_index` 到同一 run 里下一个不同的索引
    /// （连字、组合字符会让几个字符落在一个簇里）。
    pub cluster_utf16: Range<usize>,
}

/// 同一字体的一串连续字形（CoreText 的 CTRun）。
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRun {
    pub font: FontKey,
    pub utf16: Range<usize>,
    pub glyphs: Vec<ShapedGlyph>,
}

/// 一行。
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub index: usize,
    /// 这一行覆盖的文本，包含行尾的换行符（如果有）。
    pub utf16: Range<usize>,
    pub bytes: Range<usize>,
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
    /// `ceil(ascent + descent + max(0, leading))`。
    pub height: f64,
    /// 行顶（段落坐标系，pt）。
    pub top: f64,
    /// 基线 y（段落坐标系，pt），**不取整**。
    pub baseline_y: f64,
    /// 排版宽度，含行尾空白。
    pub width: f64,
    /// 不含行尾空白的宽度，`width - trailing_whitespace_width`。
    pub width_trimmed: f64,
    pub runs: Vec<ShapedRun>,
}

/// 排版结果。
#[derive(Clone, Debug, PartialEq)]
pub struct Paragraph {
    pub lines: Vec<Line>,
    /// 总高度：`max(default_line_height, 最后一行底 - line_spacing)`。
    pub height: f64,
    /// 各行 `width` 的最大值。
    pub width: f64,
    /// 基础字体算出的行高，空文本和文末空行用它。
    pub default_line_height: f64,
    /// 排版用到的全部字体（基础字体在最前，其余按首次出现的顺序，含回退字体）。
    pub fonts: Vec<FontInfo>,
    /// 文本的 UTF-16 长度。
    pub utf16_len: usize,
}

impl Paragraph {
    pub fn font(&self, key: FontKey) -> Option<&FontInfo> {
        self.fonts.iter().find(|f| f.key == key)
    }

    pub fn glyph_count(&self) -> usize {
        self.lines
            .iter()
            .flat_map(|l| &l.runs)
            .map(|r| r.glyphs.len())
            .sum()
    }
}
