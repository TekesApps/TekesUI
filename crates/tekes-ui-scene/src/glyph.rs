//! 字形与图像相关的图元类型。
//!
//! 这些类型是 `tekes-ui-text`（产生字形位图）、[`crate::GlyphAtlas`]（缓存位图）和
//! `tekes-ui-render`（上传与绘制）之间约定的数据，本身不依赖任何平台或 GPU API。

use crate::geometry::{Point, Rect};
use tekes_ui_tokens::Rgba;

/// 字体身份，由文字层分配的不透明数字。
///
/// 同一字体（同一字体文件加同一字号和变体设置）在整个进程里必须得到同一个 `FontKey`，
/// 不同字体必须不同；atlas 只拿它当缓存键的一部分，不解释它的含义。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontKey(pub u32);

/// atlas 里一个字形位图的缓存键。前景色不进键：颜色在着色器里乘上去。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font: FontKey,
    /// 字体里的字形编号。
    pub glyph: u16,
    /// `round(scale * 1000)`，scale 是逻辑点到设备像素的比例。见 [`GlyphKey::scale_key_for`]。
    pub scale_key: u32,
}

impl GlyphKey {
    pub const fn new(font: FontKey, glyph: u16, scale_key: u32) -> GlyphKey {
        GlyphKey {
            font,
            glyph,
            scale_key,
        }
    }

    /// 由 scale 算 `scale_key`：`round(scale * 1000)`，非有限或非正数得 0。
    pub fn scale_key_for(scale: f32) -> u32 {
        if scale.is_finite() && scale > 0.0 {
            (scale * 1000.0).round() as u32
        } else {
            0
        }
    }
}

/// 位图的两种用法。两者在 atlas 里存的格式相同（BGRA8，预乘 alpha），区别只在着色方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RasterFormat {
    /// 单色字形：覆盖率在 alpha 通道，rgb 被忽略；颜色由 [`GlyphRun::color`] 提供。
    Mask,
    /// 彩色字形（emoji 等）：自带颜色，rgb 已乘 alpha；`GlyphRun::color` 的 alpha 只当作不透明度。
    Color,
}

/// 文字层光栅化出来的一个字形位图，交给 atlas 存放。
///
/// 位图自带的边距（抗锯齿边缘外多留的像素）由光栅化器负责，atlas 只在位图之间留缝隙。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterGlyph {
    /// 位图左边缘相对字形原点（基线上的点）的设备像素偏移，向右为正。
    pub left: i32,
    /// 位图上边缘相对字形原点的设备像素偏移，y 向下，所以位图在基线之上时为负。
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub format: RasterFormat,
    /// BGRA8，预乘 alpha，行紧密排列，长度 `width * height * 4`。
    pub pixels: Vec<u8>,
}

/// 字形在 atlas 里的位置，加上画出它所需的偏移。可复制，放进 [`PositionedGlyph`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AtlasGlyph {
    /// atlas 页号。
    pub page: u32,
    /// 位图在页里的左上角（像素）。
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// 同 [`RasterGlyph::left`] / [`RasterGlyph::top`]。
    pub left: i32,
    pub top: i32,
    pub format: RasterFormat,
}

/// 一个已定位的字形。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// 字形原点（基线上的点），单位 pt，y 向下。渲染层按
    /// `round(origin * scale) + (left, top)` 把位图放到设备像素上，位图不缩放。
    pub origin: Point,
    pub glyph: AtlasGlyph,
}

/// 一串同色字形。
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRun {
    /// 非预乘 sRGB。单色字形用它着色；彩色字形只用它的 alpha 当不透明度。
    pub color: Rgba,
    pub glyphs: Vec<PositionedGlyph>,
}

/// 渲染层登记的图像句柄。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(pub u32);

/// 把一张已登记的图像画进矩形（线性采样，随矩形缩放）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageQuad {
    /// 单位 pt。
    pub rect: Rect,
    pub image: ImageId,
    /// 0 到 1，整体乘在预乘后的像素上。
    pub opacity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_key_is_rounded_thousandths() {
        assert_eq!(GlyphKey::scale_key_for(1.0), 1000);
        assert_eq!(GlyphKey::scale_key_for(2.0), 2000);
        assert_eq!(GlyphKey::scale_key_for(1.5), 1500);
        assert_eq!(GlyphKey::scale_key_for(2.0004), 2000);
        assert_eq!(GlyphKey::scale_key_for(2.0006), 2001);
        assert_eq!(GlyphKey::scale_key_for(0.0), 0);
        assert_eq!(GlyphKey::scale_key_for(f32::NAN), 0);
    }
}
