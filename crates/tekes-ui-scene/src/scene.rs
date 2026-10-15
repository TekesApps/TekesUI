//! 场景：有序的图元列表。

use crate::glyph::{GlyphRun, ImageQuad};
use crate::quad::Quad;

/// 一个绘制图元。见 crate 文档里关于扩展方式的说明。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Primitive {
    Quad(Quad),
    /// Intersect with the current rectangular clip, in logical points.
    PushClip(crate::Rect),
    /// Restore the enclosing clip. Unmatched pops restore the target bounds.
    PopClip,
    /// 一串同色字形，引用 [`crate::GlyphAtlas`] 里的位图。
    Glyphs(GlyphRun),
    /// 渲染层登记过的图像，画进矩形。
    Image(ImageQuad),
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

    pub fn push_glyphs(&mut self, run: GlyphRun) {
        self.primitives.push(Primitive::Glyphs(run));
    }

    pub fn push_image(&mut self, image: ImageQuad) {
        self.primitives.push(Primitive::Image(image));
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
