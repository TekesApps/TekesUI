//! CoreText 直绘的对照图（oracle）。
//!
//! 在参照仓库的对比工具建好之前，用它独立检验"atlas 加 GPU"这条路径：同一个段落，
//! 不经过 atlas 和 wgpu，直接用 `CTLineDraw` 画进一张 `CGBitmapContext`，得到 RGBA8。
//! 再用 [`crate::compare`] 与 `Renderer::render_to_rgba8` 的结果比较。
//!
//! 条件与 atlas 路径对齐：同样的尺寸、scale、前景/背景色，抗锯齿开，字体平滑关，
//! 行基线的设备像素位置按 `round((origin.y + baseline_y) * scale)` 取整（与渲染层给
//! 字形原点取整的做法一致）。字形在行内的水平位置怎么取整由 [`OracleOptions`] 决定。

use std::ffi::c_void;

use objc2_core_foundation::CGPoint;
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo, CGImageByteOrderInfo,
};

use super::fonts::CoreTextFonts;
use super::shaper::typeset;
use crate::model::Paragraph;
use crate::style::TextStyle;
use tekes_ui_scene::Point;
use tekes_ui_tokens::Rgba;

/// 字形怎么落到设备像素上。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// `CTLineDraw`，由 CoreGraphics 决定行内每个字形的取整。`subpixel` 控制
    /// `setShouldSubpixelPositionFonts` 与 `setShouldSubpixelQuantizeFonts`。
    Line { subpixel: bool },
    /// 逐个字形用 `CTFontDrawGlyphs` 画在 `round(原点 * scale)` 的整数像素上，
    /// 与 atlas 路径"每个字形原点取整"的规则一致。
    GlyphsRounded,
    /// 逐个字形画在水平方向**不取整**的设备像素位置上（基线 y 仍取整），`subpixel` 同上。
    /// 只用来查明 `Line` 与 `GlyphsRounded` 的差异来自哪里。
    GlyphsFractional { subpixel: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct OracleOptions {
    pub placement: Placement,
    /// 字体平滑。默认 false，与单色字形的光栅化一致。
    pub font_smoothing: bool,
}

impl Default for OracleOptions {
    fn default() -> OracleOptions {
        OracleOptions {
            placement: Placement::Line { subpixel: false },
            font_smoothing: false,
        }
    }
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// 渲染对照图，返回 `size_px.0 * size_px.1 * 4` 字节，RGBA 顺序，左上角在前，
/// 背景 `bg` 不透明。
#[allow(clippy::too_many_arguments)]
pub fn render_oracle(
    fonts: &CoreTextFonts,
    paragraph: &Paragraph,
    text: &str,
    style: &TextStyle,
    origin: Point,
    size_px: (u32, u32),
    scale: f32,
    fg: Rgba,
    bg: Rgba,
    opts: OracleOptions,
) -> Vec<u8> {
    let (w, h) = (size_px.0 as usize, size_px.1 as usize);
    let mut pixels = vec![0u8; w * h * 4];
    let space = CGColorSpace::new_device_rgb().expect("DeviceRGB");
    // RGBA 顺序、预乘 alpha：premultipliedLast，默认（大端）字节序。
    // SAFETY: pixels 大小为 w * h * 4；上下文在读取 pixels 之前释放。
    let ctx = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast::<c_void>(),
            w,
            h,
            8,
            w * 4,
            Some(&space),
            CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::OrderDefault.0,
        )
    }
    .expect("创建位图上下文失败");

    CGContext::set_allows_antialiasing(Some(&ctx), true);
    CGContext::set_should_antialias(Some(&ctx), true);
    CGContext::set_allows_font_smoothing(Some(&ctx), opts.font_smoothing);
    CGContext::set_should_smooth_fonts(Some(&ctx), opts.font_smoothing);
    let subpixel = matches!(
        opts.placement,
        Placement::Line { subpixel: true } | Placement::GlyphsFractional { subpixel: true }
    );
    CGContext::set_should_subpixel_position_fonts(Some(&ctx), subpixel);
    CGContext::set_should_subpixel_quantize_fonts(Some(&ctx), subpixel);

    // 背景。
    CGContext::set_rgb_fill_color(
        Some(&ctx),
        f64::from(to_u8(bg.r)) / 255.0,
        f64::from(to_u8(bg.g)) / 255.0,
        f64::from(to_u8(bg.b)) / 255.0,
        1.0,
    );
    CGContext::fill_rect(
        Some(&ctx),
        objc2_core_foundation::CGRect::new(
            CGPoint::new(0.0, 0.0),
            objc2_core_foundation::CGSize::new(w as f64, h as f64),
        ),
    );
    CGContext::set_rgb_fill_color(
        Some(&ctx),
        f64::from(to_u8(fg.r)) / 255.0,
        f64::from(to_u8(fg.g)) / 255.0,
        f64::from(to_u8(fg.b)) / 255.0,
        f64::from(fg.a),
    );

    let base = fonts.create_font(style);
    let ts = typeset(text, &base);
    let scale = f64::from(scale);
    for line in &paragraph.lines {
        if line.runs.is_empty() {
            continue;
        }
        // 设备像素里的基线（y 向下），取整。CG 的 y 向上：h - y。
        let origin_x_px = (f64::from(origin.x) * scale).round();
        let baseline_px = ((f64::from(origin.y) + line.baseline_y) * scale).round();
        match opts.placement {
            Placement::Line { .. } => {
                let range = objc2_core_foundation::CFRange::new(
                    line.utf16.start as isize,
                    (line.utf16.end - line.utf16.start) as isize,
                );
                // SAFETY: 范围在字符串之内。
                let ct_line = unsafe { ts.typesetter.line(range) };
                CGContext::save_g_state(Some(&ctx));
                CGContext::translate_ctm(Some(&ctx), origin_x_px, h as f64 - baseline_px);
                CGContext::scale_ctm(Some(&ctx), scale, scale);
                CGContext::set_text_position(Some(&ctx), 0.0, 0.0);
                // SAFETY: ctx 有效。
                unsafe { ct_line.draw(&ctx) };
                CGContext::restore_g_state(Some(&ctx));
            }
            Placement::GlyphsRounded | Placement::GlyphsFractional { .. } => {
                let round_x = opts.placement == Placement::GlyphsRounded;
                for run in &line.runs {
                    let Some(font) = fonts.font(run.font) else {
                        continue;
                    };
                    for g in &run.glyphs {
                        let gx = (f64::from(origin.x) + g.x) * scale;
                        let gx = if round_x { gx.round() } else { gx };
                        let gy = ((f64::from(origin.y) + line.baseline_y + g.y) * scale).round();
                        let mut glyph = g.id;
                        let mut pos = CGPoint::new(0.0, 0.0);
                        CGContext::save_g_state(Some(&ctx));
                        CGContext::translate_ctm(Some(&ctx), gx, h as f64 - gy);
                        CGContext::scale_ctm(Some(&ctx), scale, scale);
                        // SAFETY: 一个字形、一个位置。
                        unsafe {
                            font.draw_glyphs(
                                std::ptr::NonNull::from(&mut glyph),
                                std::ptr::NonNull::from(&mut pos),
                                1,
                                &ctx,
                            );
                        }
                        CGContext::restore_g_state(Some(&ctx));
                    }
                }
            }
        }
    }
    drop(ctx);
    pixels
}

/// 用 `CTFramesetter` 排同一段文字，返回各行的 UTF-16 范围。
///
/// 只用来做合理性核对：我们的排版器是 `CTTypesetter` 逐行断行（对照参照实现），
/// `CTFramesetter` 是 CoreText 自己的成框排版，两者的断行应当一致。
pub fn framesetter_line_ranges(
    fonts: &CoreTextFonts,
    text: &str,
    style: &TextStyle,
    width: f64,
) -> Vec<std::ops::Range<usize>> {
    use objc2_core_foundation::{CFArray, CFRange, CFRetained, CGRect, CGSize};
    use objc2_core_graphics::CGPath;
    use objc2_core_text::{CTFramesetter, CTLine};

    let base = fonts.create_font(style);
    let ts = typeset(text, &base);
    let height = 1.0e6;
    // SAFETY: 矩形路径、属性字符串都有效；frame 的属性字典传 None。
    let lines = unsafe {
        let path = CGPath::with_rect(
            CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(width.max(1.0), height)),
            std::ptr::null(),
        );
        let framesetter = CTFramesetter::with_attributed_string(&ts.attributed);
        let frame = framesetter.frame(CFRange::new(0, 0), &path, None);
        let lines = frame.lines();
        CFRetained::cast_unchecked::<CFArray<CTLine>>(lines)
    };
    lines
        .iter()
        .map(|line| {
            // SAFETY: 只读查询。
            let r = unsafe { line.string_range() };
            r.location as usize..(r.location + r.length) as usize
        })
        .collect()
}
