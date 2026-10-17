//! 场景：有序的图元列表。

use crate::glyph::{GlyphRun, ImageQuad, TransformedImageQuad};
use crate::quad::Quad;
use crate::shadow::ShadowQuad;

/// 一个绘制图元。见 crate 文档里关于扩展方式的说明。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Primitive {
    Quad(Quad),
    /// Gaussian-blurred rounded-rectangle shadow, painted in painter order.
    Shadow(ShadowQuad),
    /// Intersect with the current rectangular clip, in logical points.
    PushClip(crate::Rect),
    /// Restore the enclosing clip. Unmatched pops restore the target bounds.
    PopClip,
    /// 一串同色字形，引用 [`crate::GlyphAtlas`] 里的位图。
    Glyphs(GlyphRun),
    /// 渲染层登记过的图像，画进矩形。
    Image(ImageQuad),
    TransformedImage(TransformedImageQuad),
}

/// 一帧要画的内容，按 painter's order 排列：先加入的在下，后加入的在上。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    primitives: Vec<Primitive>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene::default()
    }

    /// 清空图元，保留已分配的容量，便于每帧复用。
    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    pub fn push(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.primitives.push(Primitive::Quad(quad));
    }

    pub fn push_shadow(&mut self, shadow: ShadowQuad) {
        self.primitives.push(Primitive::Shadow(shadow));
    }

    pub fn push_glyphs(&mut self, run: GlyphRun) {
        self.primitives.push(Primitive::Glyphs(run));
    }

    pub fn push_image(&mut self, image: ImageQuad) {
        self.primitives.push(Primitive::Image(image));
    }

    pub fn push_transformed_image(&mut self, image: TransformedImageQuad) {
        self.primitives.push(Primitive::TransformedImage(image));
    }

    pub fn push_clip(&mut self, rect: crate::Rect) {
        self.push(Primitive::PushClip(rect));
    }

    pub fn pop_clip(&mut self) {
        self.push(Primitive::PopClip);
    }

    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use tekes_ui_tokens::Rgba;

    #[test]
    fn push_keeps_painters_order() {
        let mut s = Scene::new();
        let a = Quad::new(Rect::new(0.0, 0.0, 1.0, 1.0), Rgba::gray(0.0));
        let b = Quad::new(Rect::new(1.0, 1.0, 1.0, 1.0), Rgba::gray(1.0));
        s.push_quad(a);
        s.push_quad(b);
        assert_eq!(s.len(), 2);
        assert_eq!(s.primitives()[0], Primitive::Quad(a));
        assert_eq!(s.primitives()[1], Primitive::Quad(b));
    }

    #[test]
    fn shadow_is_an_ordered_primitive_without_changing_quad_values() {
        let mut scene = Scene::new();
        let quad = Quad::new(Rect::new(1.0, 2.0, 3.0, 4.0), Rgba::gray(1.0));
        let shadow = ShadowQuad::new(Rect::new(1.0, 2.0, 3.0, 4.0), Rgba::gray(0.2));
        scene.push_shadow(shadow);
        scene.push_quad(quad);
        assert_eq!(scene.primitives()[0], Primitive::Shadow(shadow));
        assert_eq!(scene.primitives()[1], Primitive::Quad(quad));
        assert_eq!(quad.border_width, 0.0);
    }

    #[test]
    fn clear_empties_but_keeps_capacity() {
        let mut s = Scene::new();
        for _ in 0..100 {
            s.push_quad(Quad::new(Rect::new(0.0, 0.0, 1.0, 1.0), Rgba::gray(0.0)));
        }
        let cap = s.primitives.capacity();
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.primitives.capacity(), cap);
    }
}
