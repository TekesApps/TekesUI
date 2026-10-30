#![cfg(target_os = "macos")]
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    *,
};
fn model(text: &str, runs: &[(std::ops::Range<usize>, LabelForeground)]) -> SemanticLabel {
    SemanticLabel::new(
        LabelSourceVersion {
            source: 25,
            revision: 1,
        },
        text,
        runs.iter()
            .map(|(utf16, foreground)| LabelForegroundRun {
                utf16: utf16.clone(),
                foreground: *foreground,
            })
            .collect(),
    )
    .unwrap()
}
fn exact_roles(label: &SemanticLabel, layout: &LabelLayout) {
    assert_eq!(layout.source(), label.text());
    for glyph in layout
        .paragraph()
        .lines
        .iter()
        .flat_map(|l| &l.runs)
        .flat_map(|r| &r.glyphs)
    {
        for offset in glyph.cluster_utf16.clone() {
            assert_eq!(
                label.foreground_at(offset),
                label.foreground_at(glyph.utf16_index),
                "a glyph must not cross a foreground boundary: {:?} {:?}",
                label.text(),
                glyph
            );
        }
    }
}
#[test]
fn native_attributed_shaping_respects_foreground_boundaries_inside_fi_and_ffi_ligatures() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    // The boundary is a legal grapheme boundary inside a possible native ligature.
    // Do not substitute whole-cluster start coloring if CoreText retains a ligature.
    for (text, range) in [("fi", 1..2), ("ffi", 1..2), ("office fi ffi", 2..3)] {
        let label = model(text, &[(range.clone(), LabelForeground::Positive)]);
        for style in [
            TextStyle::system(24.),
            TextStyle::named("Times New Roman", 24.),
        ] {
            let layout = shaper.shape_semantic_label(
                &label,
                &style,
                &ShapeOptions::with_width(600.),
                LabelOptions::default(),
            );
            exact_roles(&label, &layout);
            assert_eq!(layout.paragraph().utf16_len, text.encode_utf16().count());
            assert!(
                layout
                    .paragraph()
                    .lines
                    .iter()
                    .flat_map(|l| &l.runs)
                    .flat_map(|r| &r.glyphs)
                    .any(|g| range.contains(&g.utf16_index)),
                "styled source must produce its own glyphs"
            );
        }
    }
}
#[test]
fn unicode_source_clip_ellipsis_scales_and_fallback_keep_exact_role_ranges() {
    let text = "工具👩🏽‍💻 · 中文 e\u{301}    +12 −3";
    let label = model(
        text,
        &[
            (9..17, LabelForeground::Secondary),
            (21..24, LabelForeground::Positive),
            (25..27, LabelForeground::Negative),
        ],
    );
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for scale in [1., 1.5, 2., 3.] {
        for width in [1., 40., 100., 180., 600.] {
            for overflow in [LabelOverflow::Clip, LabelOverflow::TailEllipsis] {
                let layout = shaper.shape_semantic_label(
                    &label,
                    &TextStyle::system(17. * scale),
                    &ShapeOptions::with_width(width),
                    LabelOptions::single_line(overflow),
                );
                exact_roles(&label, &layout);
                assert!(layout.paragraph().lines.len() <= 1);
                assert!(layout.width() <= width + 0.001);
                assert_eq!(layout.paragraph().utf16_len, 27);
                for omitted in layout.elided_utf16() {
                    assert!(
                        SemanticLabel::new(
                            label.version(),
                            text,
                            vec![LabelForegroundRun {
                                utf16: omitted.clone(),
                                foreground: LabelForeground::Secondary
                            }]
                        )
                        .is_ok()
                    );
                }
                if width == 600. && scale == 1. {
                    assert!(!layout.is_truncated());
                    assert!(
                        layout.paragraph().fonts.len() > 1,
                        "actual emoji/CJK fallback"
                    );
                }
            }
        }
    }
}
#[test]
fn unstyled_semantic_source_preserves_plain_layout_exactly() {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let label = model("fi ffi 中文 e\u{301} 👩🏽‍💻", &[]);
    for overflow in [LabelOverflow::Clip, LabelOverflow::TailEllipsis] {
        let style = TextStyle::system(17.);
        let shape = ShapeOptions::with_width(90.);
        let options = LabelOptions::single_line(overflow);
        assert_eq!(
            shaper.shape_semantic_label(&label, &style, &shape, options),
            shaper.shape_label(label.text(), &style, &shape, options)
        );
    }
}
