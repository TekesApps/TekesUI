//! 独立的圆角矩形阴影图元；与源 Quad 分开，以免改变现有 Quad 的结构/API。

use crate::{CornerRadii, Point, Rect};
use tekes_ui_tokens::Rgba;

/// 一块连续 Gaussian 模糊的圆角矩形阴影。
///
/// `blur_radius` 使用 CSS box-shadow 半径语义，渲染器将其换算为 sigma=`radius / 2`；
/// `outset` 是阴影源形状的扩张量（可为负），颜色 alpha 是峰值不透明度。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowQuad {
    pub rect: Rect,
    pub corner_radii: CornerRadii,
    pub color: Rgba,
    pub offset: Point,
    pub blur_radius: f32,
    pub outset: f32,
}

impl ShadowQuad {
    pub const fn new(rect: Rect, color: Rgba) -> Self {
        Self {
            rect,
            corner_radii: CornerRadii::ZERO,
            color,
            offset: Point::new(0.0, 0.0),
            blur_radius: 0.0,
            outset: 0.0,
        }
    }

    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radii = CornerRadii::uniform(radius);
        self
    }

    pub const fn with_corner_radii(mut self, radii: CornerRadii) -> Self {
        self.corner_radii = radii;
        self
    }

    pub const fn with_offset(mut self, offset: Point) -> Self {
        self.offset = offset;
        self
    }

    pub const fn with_blur_radius(mut self, radius: f32) -> Self {
        self.blur_radius = radius;
        self
    }

    pub const fn with_outset(mut self, outset: f32) -> Self {
        self.outset = outset;
        self
    }
}
