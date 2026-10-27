//! 场景：有序的图元列表。

use crate::glyph::{GlyphRun, ImageQuad, TransformedImageQuad};
use crate::quad::Quad;
use crate::shadow::ShadowQuad;
use std::any::Any;
use std::sync::Arc;

/// Process-local opaque ownership token carried with a controlled bitmap
/// primitive. Render backends must authenticate the concrete token before use.
#[derive(Clone)]
pub struct OpaqueImageLease(Arc<dyn Any + Send + Sync>);

impl OpaqueImageLease {
    pub fn from_owner<T: Any + Send + Sync>(owner: Arc<T>) -> Self {
        Self(owner)
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl std::fmt::Debug for OpaqueImageLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpaqueImageLease")
            .field(&(Arc::as_ptr(&self.0) as *const () as usize))
            .finish()
    }
}
impl PartialEq for OpaqueImageLease {
    fn eq(&self, other: &Self) -> bool {
        self.same_owner(other)
    }
}
impl Eq for OpaqueImageLease {}

/// 一个绘制图元。见 crate 文档里关于扩展方式的说明。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Primitive {
    Quad(Quad),
    /// Quad with explicit normalized outer identity. Enables same-key AA exclusion.
    KeyedQuad {
        quad: Quad,
        outer: crate::ShapeKey,
    },
    /// Intersect with a key in this Scene's immutable clip registry (version 1).
    PushShapeClip(crate::ShapeKey),
    /// Gaussian-blurred rounded-rectangle shadow, painted in painter order.
    Shadow(ShadowQuad),
    /// Intersect with the current rectangular clip, in logical points.
    PushClip(crate::Rect),
    /// Restore the enclosing clip. Legacy scenes allow unmatched pops; scenes
    /// requiring analytic clipping must have fully balanced scopes.
    PopClip,
    /// 一串同色字形，引用 [`crate::GlyphAtlas`] 里的位图。
    Glyphs(GlyphRun),
    /// 渲染层登记过的图像，画进矩形。
    Image(ImageQuad),
    TransformedImage(TransformedImageQuad),
    /// Image carrying a process-local opaque lease which the selected renderer authenticates.
    /// This primitive cannot be serialized or replayed on another process without its lease.
    ControlledImage {
        quad: ImageQuad,
        lease: OpaqueImageLease,
    },
    ControlledTransformedImage {
        quad: TransformedImageQuad,
        lease: OpaqueImageLease,
    },
}

/// 一帧要画的内容，按 painter's order 排列：先加入的在下，后加入的在上。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    primitives: Vec<Primitive>,
    clip_shapes: Vec<crate::clip::ShapeSnapshot>,
    required_clip_version: u32,
    composition_error: Option<FragmentError>,
}

mod fragment;
pub use fragment::{FragmentBudget, FragmentError, FragmentTransform};

impl Scene {
    pub fn new() -> Scene {
        Scene::default()
    }

    /// Snapshot an arrangement's immutable key inventory. Later interning does not
    /// retroactively extend this Scene, and foreign/rebuilt table keys still reject.
    pub fn with_clip_shapes(table: &crate::ShapeTable) -> Scene {
        Scene {
            clip_shapes: vec![table.snapshot()],
            required_clip_version: 1,
            ..Scene::default()
        }
    }
    pub fn clip_shape(&self, key: &crate::ShapeKey) -> Result<crate::ClipShape, crate::ClipError> {
        for source in &self.clip_shapes {
            match source.get(key) {
                Err(crate::ClipError::StaleShapeKey) => {}
                result => return result,
            }
        }
        Err(crate::ClipError::StaleShapeKey)
    }
    pub fn clip_shape_count(&self) -> usize {
        self.clip_shapes.iter().map(|s| s.len()).sum()
    }
    /// A capability requirement, never an instruction to approximate an unknown version.
    pub fn require_clip_geometry(&mut self, version: u32) {
        self.required_clip_version = version;
    }
    pub fn required_clip_geometry(&self) -> u32 {
        self.required_clip_version
    }
    pub fn push_shape_clip(&mut self, key: crate::ShapeKey) {
        self.push(Primitive::PushShapeClip(key));
    }
    pub fn push_keyed_quad(&mut self, quad: Quad, outer: crate::ShapeKey) {
        self.push(Primitive::KeyedQuad { quad, outer });
    }

    /// Clear primitives while retaining capacity and the immutable clip registry.
    /// A changed geometry arrangement requires a new registry snapshot/Scene.
    pub fn clear(&mut self) {
        self.primitives.clear();
        self.composition_error = None;
    }

    /// The first rejected legacy composition makes this frame non-submittable.
    /// Layout values returned by infallible Views do not imply successful paint.
    pub fn composition_error(&self) -> Option<FragmentError> {
        self.composition_error
    }

    /// Preserve the first composition failure; Renderer rejects before GPU work.
    /// Only clear() starts a new frame and resets this sticky error.
    pub fn record_composition_error(&mut self, error: FragmentError) {
        if self.composition_error.is_none() {
            self.composition_error = Some(error);
        }
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

    pub fn push_controlled_image(&mut self, image: ImageQuad, lease: OpaqueImageLease) {
        self.primitives
            .push(Primitive::ControlledImage { quad: image, lease });
    }

    pub fn push_controlled_transformed_image(
        &mut self,
        image: TransformedImageQuad,
        lease: OpaqueImageLease,
    ) {
        self.primitives
            .push(Primitive::ControlledTransformedImage { quad: image, lease });
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
