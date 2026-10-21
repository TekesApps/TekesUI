//! Investigation only: raw native truncation is not the public label contract.
#![cfg(target_os = "macos")]
use objc2_core_foundation::{
    CFArray, CFDictionary, CFMutableAttributedString, CFRange, CFRetained, CFString, CGPoint,
};
use objc2_core_text::{CTFont, CTLine, CTLineTruncationType, CTRun, kCTFontAttributeName};
use std::{ffi::c_void, ptr::NonNull};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
fn line(text: &str, font: &CTFont, marker: bool) -> CFRetained<CTLine> {
    let a = CFMutableAttributedString::new(None, 0).unwrap();
    let s = CFString::from_str(text);
    // SAFETY: all retained objects live across native construction; range covers this string.
    unsafe {
        CFMutableAttributedString::replace_string(Some(&a), CFRange::new(0, 0), Some(&s));
        CFMutableAttributedString::set_attribute(
            Some(&a),
            CFRange::new(0, s.length()),
            Some(kCTFontAttributeName),
            Some(font.as_ref()),
        );
        if marker {
            CFMutableAttributedString::set_attribute(
                Some(&a),
                CFRange::new(0, s.length()),
                Some(&CFString::from_str("TekesUI.Probe.Ellipsis")),
                Some(CFString::from_str("synthetic").as_ref()),
            );
        }
        CTLine::with_attributed_string(&a)
    }
}
fn width(line: &CTLine) -> f64 {
    // SAFETY: no out metrics requested; valid immutable line.
    unsafe {
        line.typographic_bounds(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}
fn runs(line: &CTLine) -> Vec<serde_json::Value> {
    // SAFETY: CoreText returns an array of retained CTRun values.
    let a = unsafe { CFRetained::cast_unchecked::<CFArray<CTRun>>(line.glyph_runs()) };
    a.iter().map(|r|{
  // SAFETY: buffers match the native glyph count; range0,0 means all glyphs.
  unsafe{
   let n=r.glyph_count()as usize;let mut indices=vec![0isize;n];let mut positions=vec![CGPoint{x:0.,y:0.};n];
   if n>0{r.string_indices(CFRange::new(0,0),NonNull::new(indices.as_mut_ptr()).unwrap());r.positions(CFRange::new(0,0),NonNull::new(positions.as_mut_ptr()).unwrap());}
   let attrs:CFRetained<CFDictionary>=r.attributes();let key=CFString::from_str("TekesUI.Probe.Ellipsis");let synthetic=!attrs.value(&*key as*const CFString as*const c_void).is_null();let range=r.string_range();
   serde_json::json!({"synthetic":synthetic,"range":[range.location,range.length],"indices":indices,"x":positions.into_iter().map(|p|p.x).collect::<Vec<_>>()})
  }
 }).collect()
}
#[test]
fn observe_native_label_truncation_boundaries_before_designing_public_policy() {
    let fonts = CoreTextFonts::new();
    let mut shaper = CoreTextShaper::new(fonts.clone());
    let texts = [
        "The long Summary section title that must remain a single line",
        "长标题混排 English 👨‍👩‍👧‍👦 结束更多内容",
        "e\u{301}e\u{301} e\u{301}e\u{301} long title",
        "שלום עולם כותרת ארוכה מאוד",
        "العربية عنوان طويل للاختبار",
        "אבג English 123 中文 😀 המשך",
        "first\nsecond\nthird",
        "",
        "👨‍👩‍👧‍👦🇨🇳👩🏽‍💻 e\u{301} 文本继续",
    ];
    let mut cases = Vec::new();
    for text in texts {
        let source = CFString::from_str(text);
        let mut boundaries = vec![0usize];
        let mut cursor = 0usize;
        while cursor < source.length() as usize {
            // SAFETY: cursor is an in-bounds composed-sequence boundary.
            let range = unsafe { source.range_of_composed_characters_at_index(cursor as isize) };
            cursor = (range.location + range.length) as usize;
            boundaries.push(cursor);
        }
        for scale in [1., 1.5, 2.] {
            let style = TextStyle::system(17. * scale);
            let base = fonts.create_font(&style);
            let full = line(text, &base, false);
            let token = line("…", &base, true);
            for max_width in [0., 1., 8., 16., 24., 64., 120., 220., 480.] {
                // SAFETY: nonnegative finite width, retained line and valid token.
                let truncated = unsafe {
                    full.truncated_line(max_width, CTLineTruncationType::End, Some(&token))
                };
                let baseline = shaper.shape(text, &style, &ShapeOptions::with_width(max_width));
                let output = truncated.as_ref().map(|l| {
                    let w = width(l);
                    assert!(w.is_finite());
                    serde_json::json!({"width":w,"runs":runs(l)})
                });
                cases.push(serde_json::json!({"text":text,"scale":scale,"width":max_width,"full_width":width(&full),"token_width":width(&token),"original_runs":runs(&full),"source_grapheme_boundaries":boundaries,"raw_truncated":output,"existing_wrapped_lines":baseline.lines.len(),"existing_wrapped_height":baseline.height}));
            }
        }
    }
    assert_eq!(cases.len(), 243);
    if let Some(path) = std::env::var_os("TEKESUI_LABEL_NATIVE_PROBE_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"scope":"raw CoreText mechanism investigation only; no public opt-in API, measurement/draw or AX acceptance inferred","cases":cases})).unwrap()).unwrap();
    }
}
