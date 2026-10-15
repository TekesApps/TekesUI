//! 端到端对照：排版 -> push_paragraph -> atlas -> wgpu 离屏渲染，与 CoreText 直绘的对照图比较。
//!
//! 门禁用 `Placement::GlyphsRounded` 的对照图：每个字形用 `CTFontDrawGlyphs` 直接画在
//! `round(原点 * scale)` 的整数像素上，不经过 atlas、不经过 wgpu。它检验的是 atlas 与 GPU
//! 路径本身：位图偏移（left/top）、边距、取整规则、sRGB 空间里的 alpha 混合。
//!
//! `CTLineDraw` 的对照图只做测量：CoreGraphics 把行内字形放在自己的亚像素网格上，
//! 而本项目（和参照实现）每个字形原点取整到设备像素，两者差的是字形位置，不是光栅化，
//! 数值见 docs/handoffs/F1.md。
//!
//! 没有 GPU 适配器时跳过。设置环境变量 `TEKES_UI_TEXT_ARTIFACTS=<目录>` 会把对照图、
//! GPU 图和热图写成 PNG，方便目测。
#![cfg(target_os = "macos")]

use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Point, Scene};
use tekes_ui_text::compare::{DiffOptions, DiffReport, diff_rgba8};
use tekes_ui_text::coretext::{
    CoreTextFonts, CoreTextRasterizer, CoreTextShaper, OracleOptions, Placement, render_oracle,
};
use tekes_ui_text::model::{Paragraph, ShapeOptions};
use tekes_ui_text::push::push_paragraph;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::Shaper;
use tekes_ui_tokens::Rgba;

const CJK: &str = "这是一段用来对比渲染结果的中文句子，包含标点。再来一句，看看换行。";
const MIXED: &str = "TekesUI 支持 CJK 与 Latin 混排，共 123 项；Hello, world!";
const EMOJI: &str = "表情 😀 与文字 abc 混排。";

const FG: (u8, u8, u8) = (0x20, 0x24, 0x30);
const BG: (u8, u8, u8) = (0xf6, 0xf5, 0xf0);
const WIDTH_PT: f64 = 160.0;
const MARGIN_PT: f64 = 8.0;

fn renderer() -> Option<Renderer> {
    match Renderer::new_headless() {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter) => {
            eprintln!("跳过：没有可用的 GPU 适配器");
            None
        }
        Err(e) => panic!("创建渲染器失败：{e}"),
    }
}

struct Case {
    fonts: CoreTextFonts,
    paragraph: Paragraph,
    text: &'static str,
    style: TextStyle,
    scale: f32,
    size: (u32, u32),
    origin: Point,
    gpu: Vec<u8>,
}

fn px(v: f64, scale: f32) -> u32 {
    (v * f64::from(scale)).ceil() as u32
}

fn render_case(r: &mut Renderer, text: &'static str, scale: f32, origin_x: f32) -> Case {
    let fonts = CoreTextFonts::new();
    let style = TextStyle::system(14.0);
    let mut shaper = CoreTextShaper::new(fonts.clone());
    let mut rasterizer = CoreTextRasterizer::new(fonts.clone());
    let paragraph = shaper.shape(text, &style, &ShapeOptions::with_width(WIDTH_PT));
    // 原点带小数，既检验取整，又不让字形原点恰好落在整数像素上。
    let origin = Point::new(origin_x, MARGIN_PT as f32 + 0.3);
    let size = (
        px(WIDTH_PT + 2.0 * MARGIN_PT, scale),
        px(paragraph.height + 2.0 * MARGIN_PT, scale),
    );
    let fg = Rgba::rgb8(FG.0, FG.1, FG.2);
    let bg = Rgba::rgb8(BG.0, BG.1, BG.2);
    let mut atlas = GlyphAtlas::default();
    atlas.begin_frame();
    let mut scene = Scene::new();
    push_paragraph(
        &mut scene,
        &mut atlas,
        &mut rasterizer,
        &paragraph,
        origin,
        scale,
        fg,
    );
    assert!(!scene.is_empty());
    let gpu = r
        .render_to_rgba8(&scene, &mut atlas, size, scale, bg)
        .unwrap();
    Case {
        fonts,
        paragraph,
        text,
        style,
        scale,
        size,
        origin,
        gpu,
    }
}

fn oracle(c: &Case, placement: Placement) -> Vec<u8> {
    render_oracle(
        &c.fonts,
        &c.paragraph,
        c.text,
        &c.style,
        c.origin,
        c.size,
        c.scale,
        Rgba::rgb8(FG.0, FG.1, FG.2),
        Rgba::rgb8(BG.0, BG.1, BG.2),
        OracleOptions {
            placement,
            font_smoothing: false,
        },
    )
}

fn compare(c: &Case, other: &[u8]) -> DiffReport {
    let bg = [BG.0, BG.1, BG.2, 255];
    let opts = DiffOptions::for_scale(f64::from(c.scale), 14.0 * f64::from(c.scale), bg);
    diff_rgba8(other, &c.gpu, c.size.0, c.size.1, opts)
}

fn save_png(dir: &str, name: &str, size: (u32, u32), rgba: &[u8]) {
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(file, size.0, size.1);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

fn save_artifacts(tag: &str, c: &Case, o: &[u8], rep: &DiffReport) {
    let Ok(dir) = std::env::var("TEKES_UI_TEXT_ARTIFACTS") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let scale = c.scale as u32;
    save_png(&dir, &format!("{tag}-{scale}x-oracle"), c.size, o);
    save_png(&dir, &format!("{tag}-{scale}x-gpu"), c.size, &c.gpu);
    save_png(&dir, &format!("{tag}-{scale}x-heat"), c.size, &rep.heatmap);
}

/// 门禁：对照图与 atlas 路径的差异在 comparison-method 第 2.1 节的容差之内。
fn assert_matches_oracle(tag: &str, text: &'static str) {
    let Some(mut r) = renderer() else { return };
    for scale in [1.0_f32, 2.0] {
        for origin_x in [MARGIN_PT as f32, MARGIN_PT as f32 + 0.37] {
            let c = render_case(&mut r, text, scale, origin_x);
            let o = oracle(&c, Placement::GlyphsRounded);
            let rep = compare(&c, &o);
            println!("{tag} {scale}x 原点x={origin_x}：{}", rep.summary());
            save_artifacts(tag, &c, &o, &rep);
            assert!(rep.text_bbox.is_some(), "对照图和 GPU 图都没有文字");
            assert!(
                rep.passed(),
                "{tag} {scale}x 原点x={origin_x}：{}",
                rep.summary()
            );
            // 比门槛严得多的预期：同样的位图、同样的整数原点，只差混合的舍入。
            assert!(
                rep.max_pixel_diff <= 3,
                "{tag} {scale}x：最大单像素差 {}",
                rep.max_pixel_diff
            );
            assert_eq!(rep.over_threshold, 0);
        }
    }
}

#[test]
fn cjk_sentence_matches_coretext_direct_draw() {
    assert_matches_oracle("cjk", CJK);
}

#[test]
fn mixed_sentence_matches_coretext_direct_draw() {
    assert_matches_oracle("mixed", MIXED);
}

#[test]
fn color_emoji_sentence_matches_coretext_direct_draw() {
    assert_matches_oracle("emoji", EMOJI);
}

#[test]
fn ctlinedraw_differs_only_by_glyph_position_not_baseline_or_rasterization() {
    // 测量：CTLineDraw 的字形在 CoreGraphics 的亚像素网格上，和整数原点的 atlas 路径逐字形
    // 相差最多半个像素，所以超阈值比例远高于门槛；这是设计上的取舍（键里没有亚像素 bin），
    // 不是 bug。这里断言它"确实不同、但整体位置对得上"，防止以后有人把它当门禁。
    let Some(mut r) = renderer() else { return };
    for scale in [1.0_f32, 2.0] {
        let c = render_case(&mut r, MIXED, scale, MARGIN_PT as f32);
        let line = oracle(&c, Placement::Line { subpixel: false });
        let rep = compare(&c, &line);
        println!("CTLineDraw {scale}x：{}", rep.summary());
        assert!(
            rep.ratio > rep.options.max_ratio,
            "若这条通过了，说明 CTLineDraw 也能当门禁"
        );
        // 包围盒（文字范围）与整数原点路径几乎重合：基线和整体位置没有偏。
        let ground = compare(&c, &oracle(&c, Placement::GlyphsRounded));
        let (a, b) = (rep.text_bbox.unwrap(), ground.text_bbox.unwrap());
        assert!(
            a.x0.abs_diff(b.x0) <= 1 && a.x1.abs_diff(b.x1) <= 2,
            "{a:?} vs {b:?}"
        );
        assert!(
            a.y0.abs_diff(b.y0) <= 1 && a.y1.abs_diff(b.y1) <= 1,
            "{a:?} vs {b:?}"
        );
        // 总墨量：逐字形的 CTFontDrawGlyphs 即使放在小数位置，总墨量也与 atlas 路径相同
        // （差异纯粹是位置）；CTLineDraw 在同样的上下文设置下却重约 13%（实测 1x、2x 都是），
        // 与亚像素开关无关，说明它还有自己的加粗，所以它不能当像素门禁。
        let ink = |img: &[u8]| -> f64 {
            img.chunks_exact(4)
                .map(|p| f64::from(255 - p[2]))
                .sum::<f64>()
        };
        let frac = oracle(&c, Placement::GlyphsFractional { subpixel: true });
        let (i_line, i_frac, i_gpu) = (ink(&line), ink(&frac), ink(&c.gpu));
        println!("总墨量 gpu {i_gpu} 小数位置逐字形 {i_frac} CTLineDraw {i_line}");
        assert!((i_frac / i_gpu - 1.0).abs() < 0.01, "{i_frac} vs {i_gpu}");
        assert!(
            i_line / i_gpu > 1.05 && i_line / i_gpu < 1.30,
            "{i_line} vs {i_gpu}"
        );
    }
}

#[test]
fn a_one_pixel_position_error_is_detected() {
    // 反向校验：把 GPU 图的原点挪 1 个设备像素，门禁必须失败，说明比较确实在看位置。
    let Some(mut r) = renderer() else { return };
    let c = render_case(&mut r, CJK, 2.0, MARGIN_PT as f32);
    let o = oracle(&c, Placement::GlyphsRounded);
    let (w, h) = (c.size.0 as usize, c.size.1 as usize);
    let mut shifted = vec![0u8; o.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x.saturating_sub(1)) * 4;
            shifted[(y * w + x) * 4..][..4].copy_from_slice(&o[src..src + 4]);
        }
    }
    let rep = compare(&c, &shifted);
    println!("偏移 1 像素：{}", rep.summary());
    assert!(!rep.passed());
}

#[test]
fn second_frame_reuses_the_atlas_without_rasterizing_again() {
    let Some(mut r) = renderer() else { return };
    let fonts = CoreTextFonts::new();
    let style = TextStyle::system(14.0);
    let mut shaper = CoreTextShaper::new(fonts.clone());
    let mut rasterizer = CoreTextRasterizer::new(fonts);
    let p = shaper.shape(MIXED, &style, &ShapeOptions::with_width(WIDTH_PT));
    let mut atlas = GlyphAtlas::default();
    let bg = Rgba::rgb8(BG.0, BG.1, BG.2);
    let fg = Rgba::rgb8(FG.0, FG.1, FG.2);
    let size = (px(WIDTH_PT + 16.0, 2.0), px(p.height + 16.0, 2.0));
    let mut frames = Vec::new();
    let mut misses = Vec::new();
    for _ in 0..2 {
        atlas.begin_frame();
        let mut scene = Scene::new();
        push_paragraph(
            &mut scene,
            &mut atlas,
            &mut rasterizer,
            &p,
            Point::new(8.0, 8.0),
            2.0,
            fg,
        );
        frames.push(
            r.render_to_rgba8(&scene, &mut atlas, size, 2.0, bg)
                .unwrap(),
        );
        misses.push(atlas.stats().misses);
    }
    assert_eq!(misses[0], misses[1], "第二帧不应再有 atlas 未命中");
    assert!(frames[0] == frames[1], "两帧结果应逐字节相同");
}
