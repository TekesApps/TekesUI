//! CoreText 排版器的行为测试：断行、行度量、行尾空白、字素簇、字体登记。
//!
//! 字体取自本机系统；没有的字体（`Menlo` 等）会让相应测试直接跳过。
#![cfg(target_os = "macos")]

use std::ops::Range;
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextShaper, framesetter_line_ranges};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::{FontFallback, Shaper};

const LATIN: &str = "The quick brown fox jumps over the lazy dog while the band plays on and on";
const CJK: &str =
    "今天天气很好，我们一起去公园散步吧。路上遇到了老朋友！他说下周有空，约好一起吃饭。";
const MIXED: &str =
    "使用 TekesUI 构建 2026 年的界面，支持 CJK 和 Latin 混排（含 123 个数字）。结束。";

#[test]
fn prepared_document_reflow_matches_fresh_layout_across_widths_and_styles() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for value in [LATIN, CJK, MIXED, "A中👨‍👩‍👧‍👦 e\u{301}\nالعربية אבג\n", ""]
    {
        for style in [
            TextStyle::system(14.0),
            TextStyle::named("Menlo", 17.0).with_weight(600),
        ] {
            let prepared = shaper.prepare(value, &style);
            for width in [Some(700.0), Some(140.0), Some(51.5), None, Some(700.0)] {
                let options = ShapeOptions {
                    width,
                    line_spacing: 2.0,
                };
                let reused = shaper.shape_prepared(&prepared, &options);
                let fresh = shaper.shape(value, &style, &options);
                assert_eq!(reused, fresh, "{value:?}, {style:?}, {width:?}");
                assert_eq!(reused.utf16_len, value.encode_utf16().count());
            }
        }
    }
}

fn shape_with(
    fonts: &CoreTextFonts,
    text: &str,
    style: &TextStyle,
    width: Option<f64>,
) -> Paragraph {
    let mut s = CoreTextShaper::new(fonts.clone());
    s.shape(
        text,
        style,
        &ShapeOptions {
            width,
            line_spacing: 0.0,
        },
    )
}

fn shape(text: &str, size: f32, width: Option<f64>) -> Paragraph {
    shape_with(&CoreTextFonts::new(), text, &TextStyle::system(size), width)
}

fn ranges(p: &Paragraph) -> Vec<Range<usize>> {
    p.lines.iter().map(|l| l.utf16.clone()).collect()
}

/// 每个段落都必须满足的结构性质。
fn check_invariants(text: &str, p: &Paragraph) {
    let utf16_len = text.encode_utf16().count();
    assert_eq!(p.utf16_len, utf16_len);
    let mut next = 0;
    let mut prev_bottom = 0.0_f64;
    for (i, l) in p.lines.iter().enumerate() {
        assert_eq!(l.index, i);
        assert_eq!(l.utf16.start, next, "第 {i} 行的起点不连续");
        next = l.utf16.end;
        // 字节范围必须落在字符边界上，并且与 UTF-16 范围对应同一段文字。
        let slice = &text[l.bytes.clone()];
        assert_eq!(slice.encode_utf16().count(), l.utf16.len(), "第 {i} 行");
        assert_eq!(
            l.height,
            (l.ascent + l.descent + l.leading.max(0.0)).ceil(),
            "行高公式"
        );
        assert!((l.top - prev_bottom).abs() < 1e-9, "行顶应紧接上一行");
        assert!((l.baseline_y - (l.top + l.ascent)).abs() < 1e-9);
        prev_bottom = l.top + l.height;
        assert!(l.width_trimmed <= l.width + 1e-9);
        for run in &l.runs {
            for g in &run.glyphs {
                assert!(
                    g.cluster_utf16.start >= l.utf16.start && g.cluster_utf16.end <= l.utf16.end
                );
            }
        }
    }
    assert_eq!(next, utf16_len, "各行没有覆盖全文");
    assert!((p.height - prev_bottom).abs() < 1e-9);
}

#[test]
fn line_breaks_match_framesetter_for_latin_cjk_and_mixed() {
    let fonts = CoreTextFonts::new();
    let style = TextStyle::system(14.0);
    for (name, text) in [("latin", LATIN), ("cjk", CJK), ("mixed", MIXED)] {
        for width in [30.0, 60.0, 95.0, 130.0, 200.0, 320.0, 2000.0] {
            let p = shape_with(&fonts, text, &style, Some(width));
            check_invariants(text, &p);
            let reference = framesetter_line_ranges(&fonts, text, &style, width);
            assert_eq!(
                ranges(&p),
                reference,
                "{name} 宽度 {width}：与 CTFramesetter 的断行不同"
            );
        }
    }
}

#[test]
fn wrapped_lines_fit_the_width_once_trailing_whitespace_is_ignored() {
    let p = shape(LATIN, 14.0, Some(120.0));
    assert!(p.lines.len() > 2);
    for l in &p.lines {
        assert!(
            l.width_trimmed <= 120.0 + 1e-6,
            "第 {} 行 trimmed 宽度 {} 超过 120",
            l.index,
            l.width_trimmed
        );
    }
}

#[test]
fn cjk_lines_never_start_with_closing_punctuation() {
    // 避头：句号、逗号、问号、叹号、右括号不出现在行首。CoreText 自己负责，这里只核实结果。
    let closing = ['，', '。', '！', '？', '、', '）', '」', '；', '：'];
    for width in [28.0, 42.0, 56.0, 70.0, 98.0, 126.0] {
        let p = shape(CJK, 14.0, Some(width));
        for l in p.lines.iter().skip(1) {
            let first = CJK[l.bytes.clone()].chars().next().unwrap();
            assert!(
                !closing.contains(&first),
                "宽度 {width} 第 {} 行以 {first} 开头",
                l.index
            );
        }
    }
}

#[test]
fn trailing_whitespace_is_included_in_width_and_excluded_in_trimmed() {
    let p = shape("Hello   ", 14.0, None);
    let bare = shape("Hello", 14.0, None);
    let l = &p.lines[0];
    assert!(l.width > l.width_trimmed + 1.0, "三个空格应该占明显的宽度");
    assert!(
        (l.width_trimmed - bare.lines[0].width).abs() < 1e-6,
        "trimmed 宽度应等于不带空格的宽度：{} vs {}",
        l.width_trimmed,
        bare.lines[0].width
    );
    // 换行时，行尾空白留在上一行，宽度里含它。
    let p = shape("Hello world foo bar", 14.0, Some(60.0));
    let first = &p.lines[0];
    assert_eq!(&"Hello world foo bar"[first.bytes.clone()], "Hello ");
    assert!(first.width > first.width_trimmed);
}

#[test]
fn trailing_newline_adds_an_empty_line_of_default_height() {
    let p = shape("ab\n", 14.0, None);
    check_invariants("ab\n", &p);
    assert_eq!(p.lines.len(), 2);
    let empty = &p.lines[1];
    assert_eq!(empty.utf16, 3..3);
    assert_eq!(empty.height, p.default_line_height);
    assert!(empty.runs.is_empty());
    assert_eq!(p.height, p.lines[0].height + p.default_line_height);

    // 中间的 \n 不额外补行；空文本是一个空行。
    let p = shape("a\nb", 14.0, None);
    assert_eq!(p.lines.len(), 2);
    let p = shape("", 14.0, None);
    assert_eq!(p.lines.len(), 1);
    assert_eq!(p.height, p.default_line_height);
}

#[test]
fn line_spacing_goes_after_each_line_and_not_after_the_last() {
    let fonts = CoreTextFonts::new();
    let style = TextStyle::system(14.0);
    let mut s = CoreTextShaper::new(fonts);
    let p = s.shape(
        "a\nb\nc",
        &style,
        &ShapeOptions {
            width: None,
            line_spacing: 3.0,
        },
    );
    assert_eq!(p.lines.len(), 3);
    assert!((p.lines[1].top - (p.lines[0].height + 3.0)).abs() < 1e-9);
    assert!((p.lines[2].top - (p.lines[0].height * 2.0 + 6.0)).abs() < 1e-9);
    assert!((p.height - (p.lines[0].height * 3.0 + 6.0)).abs() < 1e-9);
    // 负的行间距按 0 处理。
    let p = s.shape(
        "a\nb",
        &style,
        &ShapeOptions {
            width: None,
            line_spacing: -5.0,
        },
    );
    assert!((p.lines[1].top - p.lines[0].height).abs() < 1e-9);
}

#[test]
fn baselines_are_not_rounded() {
    let p = shape("one\ntwo", 14.0, None);
    // 系统字体 14pt 的 ascent 不是整数，基线保持小数。
    let b = p.lines[0].baseline_y;
    assert!(b.fract().abs() > 1e-3, "基线 {b} 不应取整");
    assert!((p.lines[0].baseline_y - p.lines[0].ascent).abs() < 1e-9);
}

#[test]
fn system_font_cjk_fallback_keeps_the_line_height() {
    // 实测：系统界面字体的 CJK 回退是 Interface 变体（HiraKakuInterface、
    // HiraginoSansGBInterface），度量与 SF 一致，所以 CJK 行与 Latin 行同高。
    let latin = shape("Hello world", 14.0, None);
    for text in ["你好世界你好", "Hello 你好 world", "日本語のテキスト"] {
        let p = shape(text, 14.0, None);
        assert_eq!(p.lines[0].height, latin.lines[0].height, "{text}");
        assert!(p.fonts.len() >= 2, "{text} 应该用到回退字体：{:?}", p.fonts);
        assert_ne!(p.fonts[1].key, p.fonts[0].key);
    }
}

#[test]
fn non_system_font_fallback_line_metrics_match_coretext() {
    use objc2_core_foundation::{CFMutableAttributedString, CFRange, CFString};
    use objc2_core_text::{CTLine, kCTFontAttributeName};

    // The fallback family and leading depend on preferred languages. Verify
    // against CoreText itself instead of assuming a Hiragino-specific metric.
    let fonts = CoreTextFonts::new();
    let style = TextStyle::named("Menlo", 14.0);
    let font = fonts.create_font(&style);
    let latin = shape_with(&fonts, "Hello world", &style, None);
    let cjk = shape_with(&fonts, "你好世界你好", &style, None);
    let mixed = shape_with(&fonts, "Hello 你好 world", &style, None);
    assert!(cjk.fonts.len() >= 2, "CJK must use actual fallback");
    assert_ne!(cjk.fonts[0].key, cjk.fonts[1].key);
    for (text, paragraph) in [
        ("Hello world", &latin),
        ("你好世界你好", &cjk),
        ("Hello 你好 world", &mixed),
    ] {
        let string = CFString::from_str(text);
        let attributed = CFMutableAttributedString::new(None, 0).unwrap();
        // SAFETY: ranges cover the valid string; CTFont is the attribute type.
        let reference = unsafe {
            CFMutableAttributedString::replace_string(
                Some(&attributed),
                CFRange::new(0, 0),
                Some(&string),
            );
            CFMutableAttributedString::set_attribute(
                Some(&attributed),
                CFRange::new(0, string.length()),
                Some(kCTFontAttributeName),
                Some(font.as_ref()),
            );
            CTLine::with_attributed_string(&attributed)
        };
        let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
        // SAFETY: reference and all output pointers are valid.
        unsafe { reference.typographic_bounds(&mut ascent, &mut descent, &mut leading) };
        let line = &paragraph.lines[0];
        assert_eq!(line.ascent, ascent, "{text}");
        assert_eq!(line.descent, descent, "{text}");
        assert_eq!(line.leading, leading, "{text}");
        assert_eq!(
            line.height,
            (ascent + descent + leading.max(0.0)).ceil(),
            "{text}"
        );
    }
    let both = shape_with(&fonts, "Hello world\n你好世界你好", &style, None);
    assert_eq!(both.lines[0].height, latin.lines[0].height);
    assert_eq!(both.lines[1].height, cjk.lines[0].height);
    assert!((both.height - (both.lines[0].height + both.lines[1].height)).abs() < 1e-9);
}

/// 一个很小的字素边界判断，覆盖这些测试用到的序列。
fn is_boundary(text: &str, utf16: usize) -> bool {
    let units: Vec<u16> = text.encode_utf16().collect();
    if utf16 == 0 || utf16 == units.len() {
        return true;
    }
    // 不能落在代理对中间。
    if (0xDC00..0xE000).contains(&units[utf16]) {
        return false;
    }
    let chars: Vec<(usize, char)> = {
        let mut off = 0;
        text.chars()
            .map(|c| {
                let at = off;
                off += c.len_utf16();
                (at, c)
            })
            .collect()
    };
    let idx = chars.iter().position(|(o, _)| *o == utf16).unwrap();
    let (cur, prev) = (chars[idx].1, chars[idx - 1].1);
    let extends = matches!(cur as u32,
        0x0300..=0x036F | 0x200D | 0xFE00..=0xFE0F | 0x1F3FB..=0x1F3FF | 0x20E3 | 0xE0020..=0xE007F);
    if extends || prev == '\u{200D}' {
        return false;
    }
    // 国旗：区域指示符两个一组。
    let is_ri = |c: char| (0x1F1E6..=0x1F1FF).contains(&(c as u32));
    if is_ri(cur) && is_ri(prev) {
        let run = chars[..idx]
            .iter()
            .rev()
            .take_while(|(_, c)| is_ri(*c))
            .count();
        if run % 2 == 1 {
            return false;
        }
    }
    true
}

#[test]
fn surrogate_pairs_emoji_and_combining_marks_are_never_split() {
    let samples = [
        "a😀b😀😀c",
        "e\u{301}e\u{301}e\u{301} x",
        "家人👨\u{200D}👩\u{200D}👧\u{200D}👦同行",
        "旗🇨🇳🇯🇵🇺🇸好",
        "👍🏽👍🏻 ok 1\u{FE0F}\u{20E3}2\u{FE0F}\u{20E3}",
        "𠮷野家𠮷野家",
    ];
    for text in samples {
        for width in [0.1, 1.0, 5.0, 12.0, 20.0, 31.0, 60.0] {
            let p = shape(text, 14.0, Some(width));
            check_invariants(text, &p);
            for l in &p.lines {
                assert!(
                    is_boundary(text, l.utf16.start),
                    "{text:?} 宽度 {width}：第 {} 行起点 {} 不在字素边界上",
                    l.index,
                    l.utf16.start
                );
            }
        }
    }
}

#[test]
fn every_grapheme_gets_its_own_line_when_nothing_fits() {
    // 宽度小到一个字都放不下时，每行恰好前进一个字素簇，不会死循环也不会切开簇。
    let text = "a😀e\u{301}b";
    let p = shape(text, 14.0, Some(0.5));
    let lines: Vec<&str> = p.lines.iter().map(|l| &text[l.bytes.clone()]).collect();
    assert_eq!(lines, ["a", "😀", "e\u{301}", "b"]);
}

#[test]
fn glyph_clusters_and_run_ranges_are_consistent() {
    let text = "fi 你好 e\u{301}x";
    let p = shape(text, 14.0, None);
    let l = &p.lines[0];
    for run in &l.runs {
        assert!(run.utf16.start <= run.utf16.end && run.utf16.end <= p.utf16_len);
        for g in &run.glyphs {
            assert!(g.cluster_utf16.start <= g.utf16_index);
            assert!(g.utf16_index < g.cluster_utf16.end.max(g.utf16_index + 1));
            assert!(g.utf16_index >= run.utf16.start && g.utf16_index < run.utf16.end);
        }
    }
    // 字形 x 在一个 LTR run 里单调不减。
    for run in &l.runs {
        for pair in run.glyphs.windows(2) {
            assert!(pair[1].x >= pair[0].x - 1e-9);
        }
    }
    assert!(p.glyph_count() >= 6);
}

#[test]
fn font_keys_are_stable_and_distinguish_fonts() {
    let fonts = CoreTextFonts::new();
    let style = TextStyle::system(14.0);
    let a = shape_with(&fonts, "Hello 你好", &style, None);
    let b = shape_with(&fonts, "World 世界", &style, None);
    // 同一登记表里，同样的字体得到同样的键。
    assert_eq!(a.fonts[0].key, b.fonts[0].key);
    assert_eq!(a.fonts[1].key, b.fonts[1].key);
    let count = fonts.len();
    let _ = shape_with(&fonts, "again 你好", &style, None);
    assert_eq!(fonts.len(), count, "重复排版不应登记新字体");
    // 字号、字重不同，键不同。
    let big = shape_with(&fonts, "Hello", &TextStyle::system(20.0), None);
    let bold = shape_with(&fonts, "Hello", &style.clone().with_weight(700), None);
    assert_ne!(big.fonts[0].key, a.fonts[0].key);
    assert_ne!(bold.fonts[0].key, a.fonts[0].key);
    // 键能取回字体信息。
    let info = fonts.info(a.fonts[0].key).unwrap();
    assert_eq!(info.size, 14.0);
    assert!(fonts.font(a.fonts[0].key).is_some());
    // 彩色字体被识别。
    let emoji = shape_with(&fonts, "😀", &style, None);
    assert!(emoji.fonts.iter().any(|f| f.is_color));
    assert!(!a.fonts[0].is_color);
}

#[test]
fn font_fallback_reports_runs_covering_the_text() {
    let fonts = CoreTextFonts::new();
    let mut s = CoreTextShaper::new(fonts);
    let text = "Hello 你好 😀 end";
    let runs = s.font_runs(text, &TextStyle::system(14.0));
    assert!(runs.len() >= 4, "{runs:?}");
    let mut next = 0;
    let mut next_byte = 0;
    for r in &runs {
        assert_eq!(r.utf16.start, next);
        assert_eq!(r.bytes.start, next_byte);
        next = r.utf16.end;
        next_byte = r.bytes.end;
    }
    assert_eq!(next, text.encode_utf16().count());
    assert_eq!(next_byte, text.len());
    assert_eq!(&text[runs[0].bytes.clone()], "Hello ");
    let cjk = runs
        .iter()
        .find(|r| text[r.bytes.clone()].contains('你'))
        .unwrap();
    assert_ne!(cjk.font.key, runs[0].font.key);
    let emoji = runs
        .iter()
        .find(|r| text[r.bytes.clone()].contains('😀'))
        .unwrap();
    assert!(emoji.font.is_color);
    // 相邻段的字体不同（同字体的相邻段已合并）。
    for w in runs.windows(2) {
        assert_ne!(w[0].font.key, w[1].font.key);
    }
    assert!(s.font_runs("", &TextStyle::default()).is_empty());
}
