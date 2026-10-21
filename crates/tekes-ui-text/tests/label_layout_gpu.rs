#![cfg(target_os = "macos")]
use objc2_core_foundation::{
    CFArray, CFDictionary, CFMutableAttributedString, CFRange, CFRetained, CFString, CGPoint,
};
use objc2_core_text::{CTFont, CTLine, CTLineTruncationType, CTRun, kCTFontAttributeName};
use std::{ffi::c_void, ptr::NonNull};
use tekes_ui_scene::{GlyphAtlas, GlyphKey, GlyphRun, Point, PositionedGlyph, Scene};
use tekes_ui_text::{
    LabelOptions, LabelOverflow, ShapeOptions, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
    traits::Rasterizer,
};
use tekes_ui_tokens::{Palette, Theme};
fn native(text: &str, font: &CTFont) -> CFRetained<CTLine> {
    let a = CFMutableAttributedString::new(None, 0).unwrap();
    let s = CFString::from_str(text);
    // SAFETY: all ranges cover retained native values of the expected types.
    unsafe {
        CFMutableAttributedString::replace_string(Some(&a), CFRange::new(0, 0), Some(&s));
        CFMutableAttributedString::set_attribute(
            Some(&a),
            CFRange::new(0, s.length()),
            Some(kCTFontAttributeName),
            Some(font.as_ref()),
        );
        CTLine::with_attributed_string(&a)
    }
}
fn native_reference(
    fonts: &CoreTextFonts,
    line: &CTLine,
    raster: &mut CoreTextRasterizer,
    atlas: &mut GlyphAtlas,
    scale: f32,
    ink: tekes_ui_tokens::Rgba,
) -> Scene {
    let mut ascent = 0.;
    // SAFETY: ascent is a valid local metric buffer; other metric pointers are optional.
    unsafe {
        line.typographic_bounds(&mut ascent, std::ptr::null_mut(), std::ptr::null_mut());
    }
    // SAFETY: CoreText's glyph_runs array contains CTRun values.
    let runs = unsafe { CFRetained::cast_unchecked::<CFArray<CTRun>>(line.glyph_runs()) };
    let mut result = Vec::new();
    for run in runs.iter() {
        // SAFETY: native attributes contain CTFont at the documented key; buffers match count.
        unsafe {
            let attrs: CFRetained<CFDictionary> = run.attributes();
            let ptr = attrs.value(kCTFontAttributeName as *const CFString as *const c_void);
            let font = NonNull::new(ptr as *mut CTFont)
                .map(|p| CFRetained::retain(p))
                .unwrap();
            let key = fonts.key_for(&font);
            let n = run.glyph_count() as usize;
            let mut ids = vec![0u16; n];
            let mut positions = vec![CGPoint { x: 0., y: 0. }; n];
            if n > 0 {
                run.glyphs(CFRange::new(0, 0), NonNull::new(ids.as_mut_ptr()).unwrap());
                run.positions(
                    CFRange::new(0, 0),
                    NonNull::new(positions.as_mut_ptr()).unwrap(),
                );
            }
            for (id, p) in ids.into_iter().zip(positions) {
                let k = GlyphKey::new(key, id, GlyphKey::scale_key_for(scale));
                if let Some(glyph) =
                    atlas.get_or_insert_with(k, || raster.rasterize(key, id, scale))
                {
                    result.push(PositionedGlyph {
                        origin: Point::new(8. + p.x as f32, 8. + (ascent - p.y) as f32),
                        glyph,
                    });
                }
            }
        }
    }
    let mut scene = Scene::new();
    if !result.is_empty() {
        scene.push_glyphs(GlyphRun {
            color: ink,
            glyphs: result,
        });
    }
    scene
}
#[test]
fn label_glyph_output_matches_independent_native_tail_line_in_real_gpu() {
    let mut gpu = tekes_ui_render::Renderer::new_headless()
        .expect("real GPU is required for this label test");
    let mut cases = Vec::new();
    let out = std::env::var_os("TEKESUI_LABEL_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &out {
        std::fs::create_dir_all(out).unwrap();
    }
    for text in [
        "A long English title with enough content to require ellipsis",
        "中文混排 English 👨‍👩‍👧‍👦 e\u{301} 内容后续",
        "שלום עולם כותרת ארוכה מאוד",
        "العربية عنوان طويل للاختبار",
        "אבג English 123 中文 😀 המשך",
        "first\nsecond\nthird",
    ] {
        for width in [80., 220.] {
            for type_scale in [1., 1.5, 2.] {
                for scale in [1., 2.] {
                    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
                        let fonts = CoreTextFonts::new();
                        let mut shaper = CoreTextShaper::new(fonts.clone());
                        let mut raster = CoreTextRasterizer::new(fonts.clone());
                        let mut atlas = GlyphAtlas::default();
                        atlas.begin_frame();
                        let style = TextStyle::system(17. * type_scale);
                        let layout = shaper.shape_label(
                            text,
                            &style,
                            &ShapeOptions::with_width(width),
                            LabelOptions::single_line(LabelOverflow::TailEllipsis),
                        );
                        let palette = Palette::from(theme);
                        let ink = palette.color("primary-text").unwrap();
                        let clear = palette.color("surface-1").unwrap();
                        let mut actual = Scene::new();
                        push_paragraph(
                            &mut actual,
                            &mut atlas,
                            &mut raster,
                            layout.paragraph(),
                            Point::new(8., 8.),
                            scale,
                            ink,
                        );
                        // Independent native input: no label-layout result or synthetic source mapping is reused.
                        let expected_text = if text.contains('\n') {
                            "first…"
                        } else {
                            text
                        };
                        let base = fonts.create_font(&style);
                        let full = native(expected_text, &base);
                        let token = native("…", &base);
                        // SAFETY: immutable native lines and positive finite constrained width.
                        let expected = unsafe {
                            full.truncated_line(width, CTLineTruncationType::End, Some(&token))
                        }
                        .unwrap();
                        let reference = native_reference(
                            &fonts,
                            &expected,
                            &mut raster,
                            &mut atlas,
                            scale,
                            ink,
                        );
                        let size = (((width + 16.) * scale as f64) as u32, (120. * scale) as u32);
                        let pixels = gpu
                            .render_to_rgba8(&actual, &mut atlas, size, scale, clear)
                            .unwrap();
                        let expected_pixels = gpu
                            .render_to_rgba8(&reference, &mut atlas, size, scale, clear)
                            .unwrap();
                        assert_eq!(
                            pixels, expected_pixels,
                            "native tail mismatch {text:?},width={width},type={type_scale},scale={scale},theme={ti}"
                        );
                        let name = format!("case{}", cases.len());
                        let saved = scale == 2. && type_scale == 1.5;
                        if saved && let Some(out) = &out {
                            let f = std::fs::File::create(out.join(format!("{name}.png"))).unwrap();
                            let mut e = png::Encoder::new(f, size.0, size.1);
                            e.set_color(png::ColorType::Rgba);
                            e.set_depth(png::BitDepth::Eight);
                            e.write_header().unwrap().write_image_data(&pixels).unwrap();
                        }
                        cases.push(serde_json::json!({"name":name,"text":text,"width":width,"type_scale":type_scale,"scale":scale,"theme":ti,"native_pixel_equal":true,"image_saved":saved}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 288);
    if let Some(out) = out {
        std::fs::write(out.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"frames":576,"scope":"Text core only: label paragraph glyph output vs independent native CTLine tail positions through actual GPU; not controls/AX/native window/whole experience acceptance"})).unwrap()).unwrap();
    }
}

#[test]
fn multiple_hard_lines_match_literal_native_layout_for_clip_and_tail() {
    use std::num::NonZeroUsize;
    use tekes_ui_text::Shaper;
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("real GPU required");
    let mut cases = Vec::new();
    for max in [1, 2, 3] {
        for overflow in [LabelOverflow::Clip, LabelOverflow::TailEllipsis] {
            for type_scale in [1., 1.5, 2.] {
                for scale in [1., 2.] {
                    for (ti, theme) in Theme::ALL.into_iter().enumerate() {
                        let fonts = CoreTextFonts::new();
                        let mut shaper = CoreTextShaper::new(fonts.clone());
                        let mut raster = CoreTextRasterizer::new(fonts.clone());
                        let mut atlas = GlyphAtlas::default();
                        atlas.begin_frame();
                        let style = TextStyle::system(17. * type_scale);
                        let layout = shaper.shape_label(
                            "A\nB\nC\nD",
                            &style,
                            &ShapeOptions::with_width(160.),
                            LabelOptions {
                                max_lines: NonZeroUsize::new(max),
                                overflow,
                            },
                        );
                        let literal = match (max, overflow) {
                            (1, LabelOverflow::Clip) => "A",
                            (2, LabelOverflow::Clip) => "A\nB",
                            (3, LabelOverflow::Clip) => "A\nB\nC",
                            (1, _) => "A…",
                            (2, _) => "A\nB…",
                            _ => "A\nB\nC…",
                        };
                        let reference = shaper.shape(literal, &style, &ShapeOptions::default());
                        let palette = Palette::from(theme);
                        let ink = palette.color("primary-text").unwrap();
                        let clear = palette.color("surface-1").unwrap();
                        let mut actual = Scene::new();
                        let mut expected = Scene::new();
                        push_paragraph(
                            &mut actual,
                            &mut atlas,
                            &mut raster,
                            layout.paragraph(),
                            Point::new(8., 8.),
                            scale,
                            ink,
                        );
                        push_paragraph(
                            &mut expected,
                            &mut atlas,
                            &mut raster,
                            &reference,
                            Point::new(8., 8.),
                            scale,
                            ink,
                        );
                        let size = ((176. * scale) as u32, (180. * scale) as u32);
                        let pixels = gpu
                            .render_to_rgba8(&actual, &mut atlas, size, scale, clear)
                            .unwrap();
                        let reference_pixels = gpu
                            .render_to_rgba8(&expected, &mut atlas, size, scale, clear)
                            .unwrap();
                        assert!(
                            pixels == reference_pixels,
                            "multiline literal mismatch: max={max},overflow={overflow:?},type={type_scale},scale={scale},theme={ti}"
                        );
                        assert_eq!(layout.height(), reference.height);
                        cases.push(serde_json::json!({"max_lines":max,"overflow":format!("{overflow:?}"),"type_scale":type_scale,"scale":scale,"theme":ti,"literal":literal,"pixel_equal":true,"height_equal":true}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 144);
    if let Some(out) = std::env::var_os("TEKESUI_LABEL_GPU_OUT") {
        std::fs::write(std::path::PathBuf::from(out).join("hard-lines.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"frames":288,"scope":"Read-only label core1/2/3lines Clip/Tail versus explicit literal text shaped by the original path; not controls or native window acceptance"})).unwrap()).unwrap();
    }
}
