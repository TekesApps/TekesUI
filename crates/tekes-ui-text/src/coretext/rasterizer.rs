//! CoreText 光栅化：一个 (字体, 字形, scale) 对应一张位图。
//!
//! 做法对照参照实现的 `MetalGlyphAtlas.rasterize`：
//!
//! - 边界：`CTFontGetBoundingRectsForGlyphs(font, .default, ...)`；空矩形（null、零宽零高）
//!   视为没有墨迹，返回 `None`，不是失败；
//! - 尺寸：墨迹边界乘 scale 后**向外取整**，四边各加 [`PADDING`] 个设备像素；
//! - 位图：8 位每分量、DeviceRGB、`premultipliedFirst | byteOrder32Little`，即 BGRA8 预乘；
//! - 抗锯齿开；字体平滑（`setShouldSmoothFonts`/`setAllowsFontSmoothing`）**只对彩色字体开**，
//!   单色字形关闭，以便与灰度抗锯齿的粗细一致；
//! - 字形用白色填充，颜色在着色器里乘上去，所以位图里存的是白色覆盖率；
//! - 用 `CTFontDrawGlyphs` 画在**整数像素原点**上，不做亚像素偏移（键里没有亚像素 bin）；
//! - 单色字形交付 [`RasterFormat::Mask`]，彩色字体（`kCTFontTraitColorGlyphs`，
//!   如 Apple Color Emoji 的 sbix）交付 [`RasterFormat::Color`]。

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo, CGImageByteOrderInfo,
};
use objc2_core_text::CTFontOrientation;

use super::fonts::CoreTextFonts;
use crate::traits::Rasterizer;
use tekes_ui_scene::{FontKey, RasterFormat, RasterGlyph};

/// 位图四边各留的设备像素数。
pub const PADDING: i32 = 2;

/// 单张位图的边长上限（设备像素），超过视为失败，避免异常字体耗尽内存。
const MAX_SIDE: i64 = 4096;

/// 位图格式：BGRA8，预乘 alpha（`premultipliedFirst` + `byteOrder32Little`）。
pub(crate) fn bgra_bitmap_info() -> u32 {
    CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0
}

/// CoreText 光栅化器。
#[derive(Clone, Debug, Default)]
pub struct CoreTextRasterizer {
    fonts: CoreTextFonts,
    /// 覆盖字体平滑开关：`(单色, 彩色)`。默认 `None`：单色关、彩色开（参照实现的做法）。
    /// 只用于对比测量，见 docs/handoffs/F1.md。
    smoothing_override: (Option<bool>, Option<bool>),
}

impl CoreTextRasterizer {
    pub fn new(fonts: CoreTextFonts) -> CoreTextRasterizer {
        CoreTextRasterizer {
            fonts,
            smoothing_override: (None, None),
        }
    }

    /// 测量用：强制字体平滑开关（单色字形、彩色字形各一个），`None` 保持默认。
    pub fn with_smoothing_override(
        mut self,
        mono: Option<bool>,
        color: Option<bool>,
    ) -> CoreTextRasterizer {
        self.smoothing_override = (mono, color);
        self
    }

    pub fn fonts(&self) -> &CoreTextFonts {
        &self.fonts
    }
}

fn is_null_rect(r: &CGRect) -> bool {
    !r.origin.x.is_finite()
        || !r.origin.y.is_finite()
        || !r.size.width.is_finite()
        || !r.size.height.is_finite()
        || r.size.width <= 0.0
        || r.size.height <= 0.0
}

impl Rasterizer for CoreTextRasterizer {
    fn rasterize(&mut self, font: FontKey, glyph: u16, scale: f32) -> Option<RasterGlyph> {
        if !(scale.is_finite() && scale > 0.0) {
            return None;
        }
        let scale = f64::from(scale);
        let info = self.fonts.info(font)?;
        let ct_font = self.fonts.font(font)?;

        let mut glyph_id = glyph;
        let mut rect = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0));
        // SAFETY: 一个字形、一个矩形的缓冲。
        unsafe {
            ct_font.bounding_rects_for_glyphs(
                CTFontOrientation::Default,
                NonNull::from(&mut glyph_id),
                &mut rect,
                1,
            );
        }
        if is_null_rect(&rect) {
            return None;
        }

        // 墨迹边界（设备像素，y 向上，原点在字形原点）向外取整，再加边距。
        let min_x = (rect.origin.x * scale).floor() as i64 - i64::from(PADDING);
        let min_y = (rect.origin.y * scale).floor() as i64 - i64::from(PADDING);
        let max_x = ((rect.origin.x + rect.size.width) * scale).ceil() as i64 + i64::from(PADDING);
        let max_y = ((rect.origin.y + rect.size.height) * scale).ceil() as i64 + i64::from(PADDING);
        let (w, h) = (max_x - min_x, max_y - min_y);
        if w <= 0 || h <= 0 || w > MAX_SIDE || h > MAX_SIDE {
            return None;
        }
        let (w, h) = (w as usize, h as usize);

        let space = CGColorSpace::new_device_rgb()?;
        let mut pixels = vec![0u8; w * h * 4];
        // SAFETY: pixels 的大小是 w * h * 4，行距 w * 4；上下文在本函数结束前释放，
        // 之后才会动 pixels。
        let ctx = unsafe {
            CGBitmapContextCreate(
                pixels.as_mut_ptr().cast::<c_void>(),
                w,
                h,
                8,
                w * 4,
                Some(&space),
                bgra_bitmap_info(),
            )
        }?;

        let is_color = info.is_color;
        CGContext::set_allows_antialiasing(Some(&ctx), true);
        CGContext::set_should_antialias(Some(&ctx), true);
        let smooth = if is_color {
            self.smoothing_override.1.unwrap_or(true)
        } else {
            self.smoothing_override.0.unwrap_or(false)
        };
        CGContext::set_allows_font_smoothing(Some(&ctx), smooth);
        CGContext::set_should_smooth_fonts(Some(&ctx), smooth);
        // 不做亚像素：字形原点就是整数像素。
        CGContext::set_should_subpixel_position_fonts(Some(&ctx), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&ctx), false);
        CGContext::set_rgb_fill_color(Some(&ctx), 1.0, 1.0, 1.0, 1.0);

        // 先平移到整数像素原点，再缩放，字形画在 (0, 0)。
        CGContext::translate_ctm(Some(&ctx), (-min_x) as f64, (-min_y) as f64);
        CGContext::scale_ctm(Some(&ctx), scale, scale);
        let mut position = CGPoint::new(0.0, 0.0);
        // SAFETY: 一个字形、一个位置。
        unsafe {
            ct_font.draw_glyphs(
                NonNull::from(&mut glyph_id),
                NonNull::from(&mut position),
                1,
                &ctx,
            );
        }
        drop(ctx);

        Some(RasterGlyph {
            left: min_x as i32,
            // 位图第 0 行是顶部，y 向下；顶边在原点之上 max_y 个像素。
            top: -(max_y as i32),
            width: w as u32,
            height: h as u32,
            format: if is_color {
                RasterFormat::Color
            } else {
                RasterFormat::Mask
            },
            pixels,
        })
    }
}
