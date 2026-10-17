//! GPU 实例数据：scene 的 Quad 打包成 `#[repr(C)]` 的 POD 结构。

use bytemuck::{Pod, Zeroable};
use tekes_ui_scene::{Quad, ShadowQuad};

/// 一个 quad 实例；shadow-only 图元沿用同一实例/管线以保留 painter's order。
///
/// 颜色保持**非预乘** sRGB（和 token 一致），预乘在着色器里做。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct QuadInstance {
    /// x, y, width, height，单位 pt。
    pub rect: [f32; 4],
    pub fill: [f32; 4],
    pub border_color: [f32; 4],
    /// 左上、右上、右下、左下，单位 pt，已钳到短边的一半。
    pub radii: [f32; 4],
    /// x：描边宽度（pt），已钳到非负；其余保留。
    pub params: [f32; 4],
    /// 非预乘 sRGB RGBA；alpha 为阴影峰值不透明度。非阴影实例全零。
    pub shadow_color: [f32; 4],
    /// offset x/y、CSS blur radius、outset，单位 pt。非阴影实例全零。
    pub shadow_params: [f32; 4],
}

impl QuadInstance {
    pub fn from_quad(q: &Quad) -> QuadInstance {
        let w = q.rect.size.width.max(0.0);
        let h = q.rect.size.height.max(0.0);
        // 圆角钳到短边的一半：同一条边上两个角的半径之和不超过这条边的长度。
        let max_r = w.min(h) * 0.5;
        let radii = q.corner_radii.to_array().map(|r| r.clamp(0.0, max_r));
        let bw = if q.border_width.is_finite() {
            q.border_width.max(0.0)
        } else {
            0.0
        };
        let c = |c: tekes_ui_tokens::Rgba| [c.r, c.g, c.b, c.a];
        QuadInstance {
            rect: [q.rect.origin.x, q.rect.origin.y, w, h],
            fill: c(q.fill),
            border_color: c(q.border_color),
            radii,
            params: [bw, 0.0, 0.0, 0.0],
            shadow_color: [0.0; 4],
            shadow_params: [0.0; 4],
        }
    }

    pub fn from_shadow(shadow: &ShadowQuad) -> QuadInstance {
        let w = finite_nonnegative(shadow.rect.size.width);
        let h = finite_nonnegative(shadow.rect.size.height);
        let max_r = w.min(h) * 0.5;
        let radii = shadow.corner_radii.to_array().map(|r| {
            if r.is_finite() {
                r.clamp(0.0, max_r)
            } else {
                0.0
            }
        });
        let finite = |value: f32| if value.is_finite() { value } else { 0.0 };
        let color = shadow.color;
        QuadInstance {
            rect: [shadow.rect.origin.x, shadow.rect.origin.y, w, h],
            fill: [0.0; 4],
            border_color: [0.0; 4],
            radii,
            params: [0.0; 4],
            shadow_color: [color.r, color.g, color.b, color.a],
            shadow_params: [
                finite(shadow.offset.x),
                finite(shadow.offset.y),
                finite_nonnegative(shadow.blur_radius),
                finite(shadow.outset),
            ],
        }
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{CornerRadii, Rect};
    use tekes_ui_tokens::Rgba;

    #[test]
    fn instance_is_112_bytes() {
        assert_eq!(size_of::<QuadInstance>(), 112);
    }

    #[test]
    fn radii_are_clamped_to_half_the_short_side() {
        let q = Quad::new(Rect::new(0.0, 0.0, 20.0, 10.0), Rgba::gray(0.0)).with_corner_radii(
            CornerRadii {
                top_left: 100.0,
                top_right: 3.0,
                bottom_right: -1.0,
                bottom_left: 5.0,
            },
        );
        assert_eq!(QuadInstance::from_quad(&q).radii, [5.0, 3.0, 0.0, 5.0]);
    }

    #[test]
    fn negative_size_and_border_are_clamped() {
        let q = Quad::new(Rect::new(1.0, 2.0, -5.0, 4.0), Rgba::gray(0.0))
            .with_border(-2.0, Rgba::gray(1.0));
        let i = QuadInstance::from_quad(&q);
        assert_eq!(i.rect, [1.0, 2.0, 0.0, 4.0]);
        assert_eq!(i.params[0], 0.0);
    }

    #[test]
    fn ordinary_quad_leaves_shadow_parameters_zero() {
        let quad = Quad::new(Rect::new(1.0, 2.0, 3.0, 4.0), Rgba::gray(1.0));
        let instance = QuadInstance::from_quad(&quad);
        assert_eq!(instance.shadow_color, [0.0; 4]);
        assert_eq!(instance.shadow_params, [0.0; 4]);
    }

    #[test]
    fn shadow_instance_keeps_css_blur_units_and_sanitizes_nonfinite_values() {
        let shadow = ShadowQuad::new(
            Rect::new(1.0, 2.0, 20.0, 10.0),
            Rgba::new(0.3, 0.3, 0.3, 0.3),
        )
        .with_offset(tekes_ui_scene::Point::new(2.0, -3.0))
        .with_blur_radius(8.0)
        .with_outset(-1.0);
        let instance = QuadInstance::from_shadow(&shadow);
        assert_eq!(instance.shadow_params, [2.0, -3.0, 8.0, -1.0]);
        assert_eq!(instance.shadow_color, [0.3, 0.3, 0.3, 0.3]);

        let invalid = ShadowQuad::new(Rect::new(0.0, 0.0, 4.0, 4.0), Rgba::gray(1.0))
            .with_offset(tekes_ui_scene::Point::new(f32::NAN, f32::INFINITY))
            .with_blur_radius(f32::NAN)
            .with_outset(f32::NEG_INFINITY);
        assert_eq!(QuadInstance::from_shadow(&invalid).shadow_params, [0.0; 4]);
    }
}
