//! 矩形图元。

use crate::geometry::Rect;
use tekes_ui_tokens::Rgba;

/// 四个角的圆角半径，单位 pt。
///
/// 半径超过矩形短边的一半时，渲染层会钳到短边的一半（相邻的两个角按比例
/// 缩小以保持不重叠）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const ZERO: CornerRadii = CornerRadii::uniform(0.0);

    /// 四个角相同。
    pub const fn uniform(r: f32) -> CornerRadii {
        CornerRadii {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }

    /// 只有上面两个角圆（例如贴着下边缘的标签页）。
    pub const fn top(r: f32) -> CornerRadii {
        CornerRadii {
            top_left: r,
            top_right: r,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// 只有下面两个角圆。
    pub const fn bottom(r: f32) -> CornerRadii {
        CornerRadii {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: r,
            bottom_left: r,
        }
    }

    /// 按 `[左上, 右上, 右下, 左下]` 的顺序给出，与着色器里的顺序一致。
    pub const fn to_array(self) -> [f32; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

impl From<f32> for CornerRadii {
    fn from(r: f32) -> CornerRadii {
        CornerRadii::uniform(r)
    }
}

/// 一个实心或带描边的圆角矩形。
///
/// 描边画在 `rect` **内部**：外边缘就是 `rect` 的边界，向内 `border_width`
/// 为描边带。这样控件的占位尺寸不随描边变化。`border_width` 为 0 时没有描边，
/// `border_color` 被忽略。
///
/// `fill` 铺满整个 `rect`（包括描边下面），描边按 alpha 叠在 fill 之上。
/// 所以半透明的描边会透出 fill，而不是透出背后的内容。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub fill: Rgba,
    pub corner_radii: CornerRadii,
    pub border_width: f32,
    pub border_color: Rgba,
}

impl Quad {
    /// 纯色矩形，直角，无描边。
    pub const fn new(rect: Rect, fill: Rgba) -> Quad {
        Quad {
            rect,
            fill,
            corner_radii: CornerRadii::ZERO,
            border_width: 0.0,
            border_color: Rgba::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    /// 设置四角相同的圆角半径。
    pub fn with_corner_radius(mut self, r: f32) -> Quad {
        self.corner_radii = CornerRadii::uniform(r);
        self
    }

    pub fn with_corner_radii(mut self, radii: CornerRadii) -> Quad {
        self.corner_radii = radii;
        self
    }

    /// 设置描边。
    pub fn with_border(mut self, width: f32, color: Rgba) -> Quad {
        self.border_width = width;
        self.border_color = color;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_fields() {
        let red = Rgba::rgb8(255, 0, 0);
        let q = Quad::new(Rect::new(0.0, 0.0, 10.0, 10.0), red)
            .with_corner_radius(3.0)
            .with_border(1.0, Rgba::gray(0.0));
        assert_eq!(q.corner_radii, CornerRadii::uniform(3.0));
        assert_eq!(q.border_width, 1.0);
        assert_eq!(q.fill, red);
    }

    #[test]
    fn radii_order_is_tl_tr_br_bl() {
        let r = CornerRadii {
            top_left: 1.0,
            top_right: 2.0,
            bottom_right: 3.0,
            bottom_left: 4.0,
        };
        assert_eq!(r.to_array(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(CornerRadii::top(5.0).to_array(), [5.0, 5.0, 0.0, 0.0]);
        assert_eq!(CornerRadii::bottom(5.0).to_array(), [0.0, 0.0, 5.0, 5.0]);
    }
}
