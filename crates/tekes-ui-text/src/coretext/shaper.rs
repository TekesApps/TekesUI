//! CoreText 排版与字体回退。
//!
//! 流程对照参照实现的多行 shaper（`MetalTextShaper.shape`）：
//!
//! 1. 以基础字体建一个属性字符串（只设字体属性，整串一个字体），字体回退完全交给
//!    CoreText 的自动级联：不调 `CTFontCreateForString`，不设级联列表；
//! 2. `CTTypesetterCreateWithAttributedString`，逐行 `CTTypesetterSuggestLineBreak`
//!    加 `CTTypesetterCreateLine`；
//! 3. 每行的 ascent/descent/leading/宽度取自 `CTLineGetTypographicBounds`，所以回退字体
//!    的度量会把所在行撑高；行高 `ceil(ascent + descent + max(0, leading))`；
//! 4. 基线 `= 行顶 + ascent`，不取整；行间距加在每行之后，没有段间距；
//! 5. 文本为空或以 `\n` 结尾时，补一个基础字体行高的空行。
//!
//! # 与参照实现的有意差异
//!
//! - 兜底前进：`SuggestLineBreak` 返回 0 时，参照实现强制前进 1 个 UTF-16 单元，
//!   可能切开代理对和组合字符；这里前进一个**字素簇**（`CFStringGetRangeOfComposedCharacterSequence`），
//!   不会把 emoji、组合字符序列切开。

use std::ffi::c_void;
use std::ops::Range;
use std::ptr::NonNull;

use objc2_core_foundation::{
    CFArray, CFDictionary, CFMutableAttributedString, CFRange, CFRetained, CFString, CGPoint,
};
use objc2_core_text::{CTFont, CTLine, CTRun, CTTypesetter, kCTFontAttributeName};

use super::fonts::CoreTextFonts;
use crate::model::{FontInfo, FontRun, Line, Paragraph, ShapeOptions, ShapedGlyph, ShapedRun};
use crate::style::TextStyle;
use crate::traits::{FontFallback, Shaper};
use crate::utf16::Utf16Map;
use tekes_ui_scene::FontKey;

/// 参照实现传给 CoreText 的宽度下限（pt）：`max(1, width)`。
const MIN_WIDTH: f64 = 1.0;
/// 不换行时传给断行函数的宽度。
const UNBOUNDED_WIDTH: f64 = 1.0e7;

/// CoreText 排版器，同时实现 [`Shaper`] 与 [`FontFallback`]。
#[derive(Clone, Debug, Default)]
pub struct CoreTextShaper {
    fonts: CoreTextFonts,
}

impl CoreTextShaper {
    pub fn new(fonts: CoreTextFonts) -> CoreTextShaper {
        CoreTextShaper { fonts }
    }

    pub fn fonts(&self) -> &CoreTextFonts {
        &self.fonts
    }
}

/// Immutable document preparation, reusable across container widths. Keeping it
/// with the document avoids repeating CoreText fallback/shaping setup on resize.
/// It shares the same single-thread ownership as CoreTextFonts.
#[derive(Debug)]
pub struct PreparedText {
    text: String,
    base: CFRetained<CTFont>,
    typeset: Typeset,
    map: Utf16Map,
}

/// 一个属性字符串和它的排版器。
#[derive(Debug)]
pub(crate) struct Typeset {
    pub cf_text: CFRetained<CFString>,
    pub attributed: CFRetained<CFMutableAttributedString>,
    pub typesetter: CFRetained<CTTypesetter>,
    pub utf16_len: usize,
}

pub(crate) fn typeset(text: &str, font: &CTFont) -> Typeset {
    let cf_text = CFString::from_str(text);
    let utf16_len = cf_text.length() as usize;
    let attributed = CFMutableAttributedString::new(None, 0).expect("创建属性字符串失败");
    // SAFETY: attributed 与 cf_text 有效；范围在字符串之内；字体属性的值是 CTFont。
    unsafe {
        CFMutableAttributedString::replace_string(
            Some(&attributed),
            CFRange::new(0, 0),
            Some(&cf_text),
        );
        CFMutableAttributedString::set_attribute(
            Some(&attributed),
            CFRange::new(0, utf16_len as isize),
            Some(kCTFontAttributeName),
            Some(font.as_ref()),
        );
    }
    // SAFETY: 属性字符串有效。
    let typesetter = unsafe { CTTypesetter::with_attributed_string(&attributed) };
    Typeset {
        cf_text,
        attributed,
        typesetter,
        utf16_len,
    }
}

/// 从 `location` 起的下一个字素簇的 UTF-16 长度，至少为 1。
pub(crate) fn grapheme_len_at(text: &CFString, location: usize) -> usize {
    // SAFETY: location 在字符串之内（调用方保证）。
    let r = unsafe { text.range_of_composed_characters_at_index(location as isize) };
    (r.length.max(1)) as usize
}

pub(crate) struct LineMetrics {
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
    pub width: f64,
    pub trailing_whitespace: f64,
}

pub(crate) fn line_metrics(line: &CTLine, base: &CTFont) -> LineMetrics {
    let (mut ascent, mut descent, mut leading) = (0.0_f64, 0.0_f64, 0.0_f64);
    // SAFETY: 三个指针都指向本地变量。
    let width = unsafe { line.typographic_bounds(&mut ascent, &mut descent, &mut leading) };
    // SAFETY: 只读查询。
    let trailing_whitespace = unsafe { line.trailing_whitespace_width() };
    if ascent + descent <= 0.0 {
        // 参照实现：只有 ascent + descent 不为正时才退回基础字体的度量。
        // SAFETY: 只读查询。
        unsafe {
            ascent = base.ascent();
            descent = base.descent();
            leading = base.leading();
        }
    }
    LineMetrics {
        ascent,
        descent,
        leading,
        width,
        trailing_whitespace,
    }
}

pub(crate) fn line_height(m: &LineMetrics) -> f64 {
    (m.ascent + m.descent + m.leading.max(0.0)).ceil()
}

fn base_line_height(base: &CTFont) -> f64 {
    // SAFETY: 只读查询。
    let (a, d, l) = unsafe { (base.ascent(), base.descent(), base.leading()) };
    (a + d + l.max(0.0)).ceil()
}

/// 取一行里的全部 CTRun。
fn runs_of(line: &CTLine) -> Vec<CFRetained<CTRun>> {
    // SAFETY: glyph_runs 返回元素为 CTRun 的数组。
    let array = unsafe { line.glyph_runs() };
    // SAFETY: 数组元素类型确实是 CTRun。
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CTRun>>(array) };
    array.iter().collect()
}

/// CTRun 的字体。
fn run_font(run: &CTRun) -> Option<CFRetained<CTFont>> {
    // SAFETY: attributes 返回有效字典；kCTFontAttributeName 对应的值是 CTFont 或缺失。
    unsafe {
        let attrs: CFRetained<CFDictionary> = run.attributes();
        let key = kCTFontAttributeName as *const CFString as *const c_void;
        let value = attrs.value(key);
        NonNull::new(value as *mut CTFont).map(|p| CFRetained::retain(p))
    }
}

fn cf_range(r: &Range<usize>) -> CFRange {
    CFRange::new(r.start as isize, (r.end - r.start) as isize)
}

impl CoreTextShaper {
    fn collect_runs(
        &self,
        line: &CTLine,
        base_key: FontKey,
        fonts_used: &mut Vec<FontInfo>,
        total_len: usize,
    ) -> Vec<ShapedRun> {
        let mut out = Vec::new();
        for run in runs_of(line) {
            // SAFETY: 以下全是对有效 run 的只读查询，缓冲区大小与字形数一致。
            let (glyph_count, string_range) =
                unsafe { (run.glyph_count() as usize, run.string_range()) };
            let font_key = match run_font(&run) {
                Some(f) => self.fonts.key_for(&f),
                None => base_key,
            };
            if !fonts_used.iter().any(|f| f.key == font_key)
                && let Some(info) = self.fonts.info(font_key)
            {
                fonts_used.push(info);
            }
            let mut glyphs = vec![0u16; glyph_count];
            let mut positions = vec![CGPoint { x: 0.0, y: 0.0 }; glyph_count];
            let mut indices = vec![0isize; glyph_count];
            if glyph_count > 0 {
                // SAFETY: 缓冲区各有 glyph_count 个元素；CFRange(0, 0) 表示整个 run。
                unsafe {
                    run.glyphs(
                        CFRange::new(0, 0),
                        NonNull::new(glyphs.as_mut_ptr()).unwrap(),
                    );
                    run.positions(
                        CFRange::new(0, 0),
                        NonNull::new(positions.as_mut_ptr()).unwrap(),
                    );
                    run.string_indices(
                        CFRange::new(0, 0),
                        NonNull::new(indices.as_mut_ptr()).unwrap(),
                    );
                }
            }
            let run_start = string_range.location as usize;
            let run_end = (string_range.location + string_range.length) as usize;
            // 簇边界：本 run 里出现过的全部索引，加上 run 的末端。
            let mut boundaries: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
            boundaries.push(run_end.min(total_len));
            boundaries.sort_unstable();
            boundaries.dedup();
            let glyphs = (0..glyph_count)
                .map(|i| {
                    let start = indices[i] as usize;
                    let next = boundaries
                        .iter()
                        .copied()
                        .find(|&b| b > start)
                        .unwrap_or(start);
                    ShapedGlyph {
                        id: glyphs[i],
                        x: positions[i].x,
                        // CoreText 的 y 向上；段落坐标系 y 向下。
                        y: -positions[i].y,
                        utf16_index: start,
                        cluster_utf16: start..next,
                    }
                })
                .collect();
            out.push(ShapedRun {
                font: font_key,
                utf16: run_start..run_end.min(total_len),
                glyphs,
            });
        }
        out
    }
}

impl Shaper for CoreTextShaper {
    fn shape(&mut self, text: &str, style: &TextStyle, options: &ShapeOptions) -> Paragraph {
        self.shape_prepared(&self.prepare(text, style), options)
    }
}

impl CoreTextShaper {
    pub fn prepare(&self, text: &str, style: &TextStyle) -> PreparedText {
        let base = self.fonts.create_font(style);
        PreparedText {
            text: text.into(),
            typeset: typeset(text, &base),
            map: Utf16Map::new(text),
            base,
        }
    }

    /// Recompute line breaks and geometry without replacing the document's
    /// attributed string or typesetter. Font registration stays in this shaper.
    pub fn shape_prepared(&mut self, prepared: &PreparedText, options: &ShapeOptions) -> Paragraph {
        let text = prepared.text.as_str();
        let base = &prepared.base;
        let base_key = self.fonts.key_for(base);
        let ts = &prepared.typeset;
        let map = &prepared.map;
        let n = ts.utf16_len;
        let default_line_height = base_line_height(base);
        let line_spacing = options.line_spacing.max(0.0);
        let width = options.width.map_or(UNBOUNDED_WIDTH, |w| w.max(MIN_WIDTH));

        let mut fonts_used = vec![self.fonts.info(base_key).expect("基础字体已登记")];
        let mut lines: Vec<Line> = Vec::new();
        let mut cursor_y = 0.0_f64;
        let mut location = 0usize;
        while location < n {
            // SAFETY: location < n，排版器有效。
            let suggested = unsafe { ts.typesetter.suggest_line_break(location as isize, width) };
            let mut length = if suggested <= 0 {
                // 有意差异：按字素簇前进，而不是参照实现的 1 个 UTF-16 单元。
                grapheme_len_at(&ts.cf_text, location)
            } else {
                suggested as usize
            };
            length = length.min(n - location);
            let utf16 = location..location + length;
            // SAFETY: 范围在字符串之内。
            let line = unsafe { ts.typesetter.line(cf_range(&utf16)) };
            let m = line_metrics(&line, base);
            let height = line_height(&m);
            let runs = self.collect_runs(&line, base_key, &mut fonts_used, n);
            let mut carets = Vec::new();
            let mut index = utf16.start;
            while index < utf16.end {
                // SAFETY: index is a composed-character boundary inside this CTLine.
                let x =
                    unsafe { line.offset_for_string_index(index as isize, std::ptr::null_mut()) };
                carets.push((index, x));
                index += grapheme_len_at(&ts.cf_text, index);
            }
            carets.push((utf16.end, unsafe {
                line.offset_for_string_index(utf16.end as isize, std::ptr::null_mut())
            }));
            let has_newline = text[map.byte_range(utf16.clone())].ends_with('\n');
            lines.push(Line {
                has_newline,
                carets,
                index: lines.len(),
                bytes: map.byte_range(utf16.clone()),
                utf16,
                ascent: m.ascent,
                descent: m.descent,
                leading: m.leading,
                height,
                top: cursor_y,
                baseline_y: cursor_y + m.ascent,
                width: m.width,
                width_trimmed: m.width - m.trailing_whitespace,
                runs,
            });
            cursor_y += height + line_spacing;
            location += length;
        }
        if n == 0 || text.ends_with('\n') {
            // SAFETY: 只读查询。
            let (ascent, descent, leading) =
                unsafe { (base.ascent(), base.descent(), base.leading()) };
            lines.push(Line {
                index: lines.len(),
                utf16: n..n,
                bytes: text.len()..text.len(),
                ascent,
                descent,
                leading,
                height: default_line_height,
                top: cursor_y,
                baseline_y: cursor_y + ascent,
                width: 0.0,
                width_trimmed: 0.0,
                runs: Vec::new(),
                carets: vec![(n, 0.0)],
                has_newline: false,
            });
            cursor_y += default_line_height + line_spacing;
        }
        let height = default_line_height.max(cursor_y - line_spacing);
        let width = lines.iter().map(|l| l.width).fold(0.0, f64::max);
        Paragraph {
            lines,
            height,
            width,
            default_line_height,
            fonts: fonts_used,
            utf16_len: n,
        }
    }
}

impl FontFallback for CoreTextShaper {
    fn font_runs(&mut self, text: &str, style: &TextStyle) -> Vec<FontRun> {
        let base = self.fonts.create_font(style);
        let base_key = self.fonts.key_for(&base);
        let ts = typeset(text, &base);
        let map = Utf16Map::new(text);
        let n = ts.utf16_len;
        if n == 0 {
            return Vec::new();
        }
        // 整串排成一行（不断行），读每个 run 的字体和字符范围。
        // SAFETY: 范围覆盖整个字符串。
        let line = unsafe { ts.typesetter.line(CFRange::new(0, n as isize)) };
        let mut out: Vec<FontRun> = Vec::new();
        for run in runs_of(&line) {
            // SAFETY: 只读查询。
            let sr = unsafe { run.string_range() };
            let range = sr.location as usize..(sr.location + sr.length) as usize;
            let key = run_font(&run).map_or(base_key, |f| self.fonts.key_for(&f));
            let info = self.fonts.info(key).expect("字体已登记");
            if let Some(last) = out.last_mut()
                && last.font.key == key
                && last.utf16.end == range.start
            {
                last.utf16.end = range.end;
                last.bytes.end = map.byte_of(range.end);
                continue;
            }
            out.push(FontRun {
                bytes: map.byte_range(range.clone()),
                utf16: range,
                font: info,
            });
        }
        // 双向文本的 run 不一定按逻辑顺序排列，按起点排序后再合并相邻同字体段。
        out.sort_by_key(|r| r.utf16.start);
        let mut merged: Vec<FontRun> = Vec::new();
        for r in out {
            if let Some(last) = merged.last_mut()
                && last.font.key == r.font.key
                && last.utf16.end == r.utf16.start
            {
                last.utf16.end = r.utf16.end;
                last.bytes.end = r.bytes.end;
                continue;
            }
            merged.push(r);
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len_at(text: &str, utf16: usize) -> usize {
        grapheme_len_at(&CFString::from_str(text), utf16)
    }

    #[test]
    fn forced_progress_advances_by_a_whole_grapheme() {
        // 参照实现这里前进 1 个 UTF-16 单元，会切开代理对和组合字符；我们前进整个字素簇。
        assert_eq!(len_at("ab", 0), 1);
        assert_eq!(len_at("😀x", 0), 2);
        assert_eq!(len_at("e\u{301}x", 0), 2);
        assert_eq!(len_at("🇨🇳x", 0), 4);
        assert_eq!(len_at("👨\u{200D}👩\u{200D}👧x", 0), 8);
        assert_eq!(len_at("a😀", 1), 2);
    }

    #[test]
    fn forced_progress_is_at_least_one() {
        // 即使 CoreText 返回 0 长度，也要保证前进。
        assert!(len_at("a", 0) >= 1);
    }
}
