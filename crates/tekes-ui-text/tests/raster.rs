//! CoreText 光栅化器的测试：确定性、边距、格式、坐标约定。
#![cfg(target_os = "macos")]

use tekes_ui_scene::{FontKey, RasterFormat, RasterGlyph};
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper, PADDING};
use tekes_ui_text::model::ShapeOptions;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::{Rasterizer, Shaper};

/// 排一个字，返回它的字体键和字形编号。
fn glyph_of(fonts: &CoreTextFonts, text: &str, size: f32) -> (FontKey, u16) {
    let mut s = CoreTextShaper::new(fonts.clone());
    let p = s.shape(text, &TextStyle::system(size), &ShapeOptions::default());
    let run = &p.lines[0].runs[0];
    (run.font, run.glyphs[0].id)
}

fn alpha(g: &RasterGlyph, x: u32, y: u32) -> u8 {
    g.pixels[((y * g.width + x) * 4 + 3) as usize]
}

/// 有墨迹的像素的外接矩形 (x0, y0, x1, y1)，半开。
fn ink_bounds(g: &RasterGlyph) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..g.height {
        for x in 0..g.width {
            let p = &g.pixels[((y * g.width + x) * 4) as usize..][..4];
            if p.iter().any(|&v| v != 0) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x0, y0, x1, y1)
}

#[test]
fn rasterizing_is_deterministic() {
    let fonts = CoreTextFonts::new();
    let mut r1 = CoreTextRasterizer::new(fonts.clone());
    let mut r2 = CoreTextRasterizer::new(CoreTextFonts::new());
    for text in ["A", "g", "中", "@", "😀"] {
        let (font, glyph) = glyph_of(&fonts, text, 14.0);
        for scale in [1.0_f32, 2.0] {
            let a = r1.rasterize(font, glyph, scale).unwrap();
            let b = r1.rasterize(font, glyph, scale).unwrap();
            assert_eq!(a, b, "{text} 同一光栅化器两次结果不同");
            // 另一张登记表（字体键的分配顺序不同也无妨）里重建同一个字形。
            let fonts2 = r2.fonts().clone();
            let (font2, glyph2) = glyph_of(&fonts2, text, 14.0);
            assert_eq!(glyph, glyph2);
            let c = r2.rasterize(font2, glyph2, scale).unwrap();
            assert_eq!(a, c, "{text} 两个光栅化器的结果不同");
        }
    }
}

#[test]
fn bitmap_has_two_pixels_of_padding_on_every_side() {
    assert_eq!(PADDING, 2);
    let fonts = CoreTextFonts::new();
    let mut r = CoreTextRasterizer::new(fonts.clone());
    for text in ["A", "j", "中", "W", ".", "😀"] {
        for scale in [1.0_f32, 2.0] {
            let (font, glyph) = glyph_of(&fonts, text, 14.0);
            let g = r.rasterize(font, glyph, scale).unwrap();
            assert_eq!(g.pixels.len(), (g.width * g.height * 4) as usize);
            let (x0, y0, x1, y1) = ink_bounds(&g);
            // 外圈 2 像素没有墨迹（抗锯齿的扩散不超过边界取整多出来的那一点）。
            assert!(x0 >= 2 && y0 >= 2, "{text} @{scale}x：左/上边距 {x0},{y0}");
            assert!(
                g.width - x1 >= 2 && g.height - y1 >= 2,
                "{text} @{scale}x：右/下边距 {},{}",
                g.width - x1,
                g.height - y1
            );
            // 边距不应大得离谱：墨迹边界取整多出 < 1 像素，加上 2 像素的边距。
            assert!(x0 <= 4 && y0 <= 4, "{text} @{scale}x：边距过大 {x0},{y0}");
        }
    }
}

#[test]
fn monochrome_glyphs_are_masks_and_color_glyphs_are_color() {
    let fonts = CoreTextFonts::new();
    let mut r = CoreTextRasterizer::new(fonts.clone());
    let (font, glyph) = glyph_of(&fonts, "A", 14.0);
    let mask = r.rasterize(font, glyph, 2.0).unwrap();
    assert_eq!(mask.format, RasterFormat::Mask);
    // 白色覆盖率，预乘：BGR 三个通道都等于 alpha。
    let mut ink = 0;
    for px in mask.pixels.chunks_exact(4) {
        assert_eq!(px[0], px[3]);
        assert_eq!(px[1], px[3]);
        assert_eq!(px[2], px[3]);
        if px[3] > 0 {
            ink += 1;
        }
    }
    assert!(ink > 20);
    // 抗锯齿：既有全覆盖也有部分覆盖的像素。
    assert!(mask.pixels.chunks_exact(4).any(|p| p[3] == 255));
    assert!(mask.pixels.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));

    let (efont, eglyph) = glyph_of(&fonts, "😀", 14.0);
    assert!(fonts.info(efont).unwrap().is_color);
    let color = r.rasterize(efont, eglyph, 2.0).unwrap();
    assert_eq!(color.format, RasterFormat::Color);
    // 预乘有效：rgb 不超过 alpha；并且确实有彩色（不是灰度）。
    let mut colorful = false;
    for px in color.pixels.chunks_exact(4) {
        assert!(px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]);
        if px[3] > 0 && (px[0].abs_diff(px[2]) > 20) {
            colorful = true;
        }
    }
    assert!(colorful);
}

#[test]
fn offsets_follow_the_documented_convention() {
    let fonts = CoreTextFonts::new();
    let mut r = CoreTextRasterizer::new(fonts.clone());
    // 'A' 全在基线之上：位图顶边在原点之上，底边在原点附近（边距 2 像素之内）。
    let (font, glyph) = glyph_of(&fonts, "A", 14.0);
    for scale in [1.0_f32, 2.0] {
        let g = r.rasterize(font, glyph, scale).unwrap();
        assert!(g.top < 0, "top 应为负：{}", g.top);
        let bottom = g.top + g.height as i32;
        assert!((0..=3).contains(&bottom), "底边 {bottom} 应贴近基线");
        // 'A' 的墨迹底边正好落在基线上：第 (-top) 行是基线以下的第一行。
        let (_, _, _, y1) = ink_bounds(&g);
        assert_eq!(
            g.top + y1 as i32,
            0,
            "A 的墨迹底边应在基线上（scale {scale}）"
        );
    }
    // 'g' 有下伸部分：位图越过基线。
    let (gfont, gglyph) = glyph_of(&fonts, "g", 14.0);
    let g = r.rasterize(gfont, gglyph, 2.0).unwrap();
    assert!(g.top + g.height as i32 > 4);
    // 2x 的位图大约是 1x 的两倍。
    let a1 = r.rasterize(font, glyph, 1.0).unwrap();
    let a2 = r.rasterize(font, glyph, 2.0).unwrap();
    let (w1, w2) = (a1.width as f64 - 4.0, a2.width as f64 - 4.0);
    assert!((w2 / w1 - 2.0).abs() < 0.4, "{w1} -> {w2}");
}

#[test]
fn glyphs_without_ink_and_bad_input_give_none() {
    let fonts = CoreTextFonts::new();
    let mut r = CoreTextRasterizer::new(fonts.clone());
    let mut s = CoreTextShaper::new(fonts.clone());
    let p = s.shape(" ", &TextStyle::system(14.0), &ShapeOptions::default());
    let space = &p.lines[0].runs[0];
    assert!(r.rasterize(space.font, space.glyphs[0].id, 2.0).is_none());
    let (font, glyph) = glyph_of(&fonts, "A", 14.0);
    assert!(r.rasterize(font, glyph, 0.0).is_none());
    assert!(r.rasterize(font, glyph, f32::NAN).is_none());
    assert!(r.rasterize(FontKey(9999), glyph, 2.0).is_none());
}

#[test]
fn font_smoothing_is_off_for_monochrome_and_has_no_effect_on_color_here() {
    // 参照实现：单色字形关闭字体平滑，只有彩色字体开。实测：开平滑会让单色字形明显变粗；
    // 对 Apple Color Emoji（sbix 位图）开或关输出逐字节相同。
    let fonts = CoreTextFonts::new();
    let (font, glyph) = glyph_of(&fonts, "A", 14.0);
    let sum = |g: &RasterGlyph| {
        g.pixels
            .chunks_exact(4)
            .map(|p| u64::from(p[3]))
            .sum::<u64>()
    };
    let off = CoreTextRasterizer::new(fonts.clone())
        .with_smoothing_override(Some(false), None)
        .rasterize(font, glyph, 2.0)
        .unwrap();
    let on = CoreTextRasterizer::new(fonts.clone())
        .with_smoothing_override(Some(true), None)
        .rasterize(font, glyph, 2.0)
        .unwrap();
    assert!(
        sum(&on) > sum(&off) * 11 / 10,
        "开平滑应更粗：{} vs {}",
        sum(&on),
        sum(&off)
    );
    let default = CoreTextRasterizer::new(fonts.clone())
        .rasterize(font, glyph, 2.0)
        .unwrap();
    assert_eq!(default, off, "默认应关闭单色字形的平滑");

    let (efont, eglyph) = glyph_of(&fonts, "😀", 14.0);
    let e_on = CoreTextRasterizer::new(fonts.clone())
        .with_smoothing_override(None, Some(true))
        .rasterize(efont, eglyph, 2.0)
        .unwrap();
    let e_off = CoreTextRasterizer::new(fonts.clone())
        .with_smoothing_override(None, Some(false))
        .rasterize(efont, eglyph, 2.0)
        .unwrap();
    assert_eq!(e_on, e_off);
    assert_eq!(alpha(&e_on, e_on.width / 2, e_on.height / 2), 255);
}
