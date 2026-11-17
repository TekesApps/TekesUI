#![cfg(target_os = "macos")]
use objc2_core_foundation::CFString;
use std::num::NonZeroUsize;
use tekes_ui_text::{
    LabelOptions, LabelOverflow, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
fn boundaries(text: &str) -> Vec<usize> {
    let source = CFString::from_str(text);
    let mut out = vec![0];
    let mut at = 0;
    while at < source.length() as usize {
        // SAFETY: at is a composed-sequence start inside this retained string.
        let r = unsafe { source.range_of_composed_characters_at_index(at as isize) };
        at = (r.location + r.length) as usize;
        out.push(at);
    }
    out
}
#[test]
fn default_label_options_preserve_original_shaping_byte_for_byte() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for text in [
        "",
        "a\n",
        "CJK 中文 👨‍👩‍👧‍👦 e\u{301} title",
        "אבג English 123 العربية",
    ] {
        for width in [None, Some(1.), Some(220.), Some(600.)] {
            let style = TextStyle::system(17.);
            let options = ShapeOptions {
                width,
                line_spacing: 2.,
            };
            let original = shaper.shape(text, &style, &options);
            let label = shaper.shape_label(text, &style, &options, LabelOptions::default());
            assert_eq!(label.paragraph(), &original);
            assert_eq!(label.source(), text);
            assert!(!label.is_truncated());
        }
    }
}
#[test]
fn line_limits_preserve_source_and_graphemes_across_fonts_rtl_hard_breaks_and_tiny_widths() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let mut cases = 0;
    for text in [
        "A long label with English words that should not create extra header rows",
        "中文混排 English 👨‍👩‍👧‍👦 e\u{301} 内容后续",
        "👨‍👩‍👧‍👦🇨🇳👩🏽‍💻 e\u{301} 文本继续",
        "שלום עולם כותרת ארוכה מאוד",
        "العربية عنوان طويل للاختبار",
        "אבג English 123 中文 😀 המשך",
        "first\nsecond\nthird",
        "a\r\nb\u{2028}c\u{2029}d",
        "title\n",
        "",
    ] {
        let safe = boundaries(text);
        for scale in [1., 1.5, 2.] {
            for mono in [false, true] {
                let style = if mono {
                    TextStyle::named("Menlo", 17. * scale)
                } else {
                    TextStyle::system(17. * scale)
                };
                for width in [0., 1., 8., 16., 24., 64., 180., 600.] {
                    for max in [1, 2, 3] {
                        for overflow in [LabelOverflow::Clip, LabelOverflow::TailEllipsis] {
                            let options = ShapeOptions {
                                width: Some(width),
                                line_spacing: 2.,
                            };
                            let label = shaper.shape_label(
                                text,
                                &style,
                                &options,
                                LabelOptions {
                                    max_lines: NonZeroUsize::new(max),
                                    overflow,
                                },
                            );
                            assert_eq!(label.source(), text);
                            assert_eq!(label.paragraph().utf16_len, text.encode_utf16().count());
                            assert!(label.paragraph().lines.len() <= max);
                            assert!(label.height().is_finite() && label.height() > 0.);
                            assert!(label.width().is_finite() && label.width() <= width + 0.001);
                            for omitted in label.elided_utf16() {
                                assert!(
                                    omitted.start < omitted.end
                                        && omitted.end <= text.encode_utf16().count()
                                );
                                assert!(
                                    safe.contains(&omitted.start) && safe.contains(&omitted.end),
                                    "source cluster split {text:?} {omitted:?}"
                                );
                            }
                            for line in &label.paragraph().lines {
                                assert!(line.height.is_finite() && line.height > 0.);
                                if overflow == LabelOverflow::TailEllipsis {
                                    assert!(
                                        line.width <= width + 0.001,
                                        "overwide ellipsis {text:?} {width}: {}",
                                        line.width
                                    );
                                }
                                for g in line.runs.iter().flat_map(|r| &r.glyphs) {
                                    assert!(g.x.is_finite() && g.y.is_finite());
                                    assert!(g.cluster_utf16.end <= label.paragraph().utf16_len);
                                    for omitted in label.elided_utf16() {
                                        assert!(
                                            g.cluster_utf16.is_empty()
                                                || g.cluster_utf16.end <= omitted.start
                                                || g.cluster_utf16.start >= omitted.end,
                                            "hidden glyph retained: {text:?} {:?} vs {omitted:?}",
                                            g.cluster_utf16
                                        );
                                    }
                                }
                            }
                            if text.is_empty() {
                                assert!(!label.is_truncated());
                                assert_eq!(label.paragraph().glyph_count(), 0);
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2880);
}
#[test]
fn hidden_hard_lines_force_ellipsis_even_when_visible_segment_fits() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for text in ["a\nb", "a\r\nb", "title\n", "\nnext", "one\n\nthree"] {
        for max in [1, 2] {
            let label = shaper.shape_label(
                text,
                &TextStyle::system(17.),
                &ShapeOptions::with_width(600.),
                LabelOptions {
                    max_lines: NonZeroUsize::new(max),
                    overflow: LabelOverflow::TailEllipsis,
                },
            );
            let normal = shaper.shape(
                text,
                &TextStyle::system(17.),
                &ShapeOptions::with_width(600.),
            );
            if normal.lines.len() > max {
                assert!(label.is_truncated());
                assert!(
                    label
                        .paragraph()
                        .lines
                        .last()
                        .unwrap()
                        .runs
                        .iter()
                        .flat_map(|r| &r.glyphs)
                        .any(|g| g.cluster_utf16.is_empty()),
                    "synthetic ellipsis must mark hidden hard lines {text:?}, max={max}, result={label:?}"
                );
            }
        }
    }
}
#[test]
fn constrained_ellipsis_without_space_drops_the_whole_token_and_keeps_normal_line_metrics() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for width in [0., 1., f64::NAN, f64::NEG_INFINITY] {
        let label = shaper.shape_label(
            "👨‍👩‍👧‍👦 title",
            &TextStyle::system(17.),
            &ShapeOptions::with_width(width),
            LabelOptions::single_line(LabelOverflow::TailEllipsis),
        );
        assert_eq!(label.paragraph().glyph_count(), 0);
        assert!(label.height() > 0. && label.height().is_finite());
        assert!(label.is_truncated());
    }
}

#[test]
fn long_emoji_tail_ellipsis_jumps_to_cut_without_quadratic_resizing() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let text = format!("{}{}", "a", "👨‍👩‍👧‍👦e\u{301}X".repeat(2000));
    assert!(text.encode_utf16().count() >= 18_000);
    let started = std::time::Instant::now();
    let label = shaper.shape_label(
        &text,
        &TextStyle::system(15.),
        &ShapeOptions::with_width(140.),
        LabelOptions::single_line(LabelOverflow::TailEllipsis),
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "TailEllipsis took too long ({elapsed:?})"
    );
    assert!(label.is_truncated());
    assert_eq!(label.paragraph().lines.len(), 1);
    assert!(label.paragraph().lines[0].width <= 140.001);
}

#[test]
fn with_min_line_height_preserves_line_spacing_and_larger_default_height() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let spaced = shaper.shape_label(
        "line one\nline two",
        &TextStyle::system(14.),
        &ShapeOptions {
            width: Some(300.),
            line_spacing: 6.0,
        },
        LabelOptions::default(),
    );
    let base_default = spaced.paragraph().default_line_height;
    let grown = spaced.with_min_line_height(24.0);
    let lines = &grown.paragraph().lines;
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].height, 24.0);
    assert!((lines[1].top - (lines[0].height + 6.0)).abs() < 1e-6);
    assert!((grown.height() - (24.0 + 6.0 + 24.0)).abs() < 1e-6);

    let larger_font = shaper
        .shape_label(
            "big",
            &TextStyle::system(28.),
            &ShapeOptions::default(),
            LabelOptions::default(),
        )
        .with_min_line_height(12.0);
    assert!(larger_font.paragraph().default_line_height >= base_default);
    assert!(larger_font.paragraph().default_line_height > 12.0);
}

#[test]
fn bidi_selection_rects_and_hit_test_handle_mixed_ltr_rtl_and_fixture() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let viewport = tekes_ui_scene::Rect::new(0., 0., 800., 600.);

    let hello_shalom = shaper.shape(
        "Hello שלום",
        &TextStyle::system(14.),
        &ShapeOptions::with_width(400.),
    );
    let space_rect = hello_shalom.selection_rects(5..6, viewport);
    let shin_rect = hello_shalom.selection_rects(6..7, viewport);
    let rtl_word_rect = hello_shalom.selection_rects(6..10, viewport);
    assert_eq!(space_rect.len(), 1);
    assert_eq!(shin_rect.len(), 1);
    assert_eq!(rtl_word_rect.len(), 1);
    assert!(space_rect[0].max_x() <= rtl_word_rect[0].min_x() + 0.01);
    // In LTR+RTL `"Hello שלום"`, index 6..7 (`ש`) is on the RIGHT end of `"שלום"`.
    assert!(shin_rect[0].min_x() > space_rect[0].max_x() + 5.0);
    let shin_mid = tekes_ui_scene::Point::new(shin_rect[0].center().x, shin_rect[0].center().y);
    let hit_shin = hello_shalom.hit_test(shin_mid);
    assert!(hit_shin == 6 || hit_shin == 7, "hit={hit_shin}");

    let single_rtl = shaper.shape(
        "Hello ש world",
        &TextStyle::system(14.),
        &ShapeOptions::with_width(400.),
    );
    let middle_space = single_rtl.selection_rects(5..6, viewport);
    let middle_shin = single_rtl.selection_rects(6..7, viewport);
    let tail_space = single_rtl.selection_rects(7..8, viewport);
    assert_eq!(middle_shin.len(), 1);
    assert!(middle_shin[0].min_x() >= middle_space[0].max_x() - 0.01);
    assert!(middle_shin[0].max_x() <= tail_space[0].min_x() + 0.01);

    let fixture = include_str!("../../../fixtures/text/rtl.txt");
    let para = shaper.shape(
        fixture,
        &TextStyle::system(14.),
        &ShapeOptions::with_width(420.),
    );
    for line in &para.lines {
        if line.utf16.is_empty() {
            continue;
        }
        assert!(!line.graphemes.is_empty());
        for g in &line.graphemes {
            if line.has_newline && g.end == line.utf16.end {
                continue;
            }
            let rects = para.selection_rects(g.start..g.end, viewport);
            assert!(
                !rects.is_empty() || (g.x_start - g.x_end).abs() < 1e-4,
                "missing selection rect for {:?} on {:?}",
                g,
                line.utf16
            );
            let mid_x = ((g.x_start + g.x_end) * 0.5) as f32;
            let mid_y = (line.top + line.height * 0.5) as f32;
            let hit = para.hit_test(tekes_ui_scene::Point::new(mid_x, mid_y));
            assert!(
                hit == g.start || hit == g.end,
                "hit={hit} expected {} or {} at {mid_x} on {:?}",
                g.start,
                g.end,
                line.utf16
            );
        }
    }
}
