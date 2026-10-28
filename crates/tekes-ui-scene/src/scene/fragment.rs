//! Atomic CPU display-list composition. Device/scale/atlas preflight remains in Renderer.
use super::{Primitive, Scene};
use crate::{ClipError, ClipShape, CornerRadii, Point, Rect};

/// Cumulative destination policy, including the caller's existing inventory and paint.
/// These are resource bounds, independent of theme and device clip-frame budgets.
#[derive(Clone, Copy, Debug)]
pub struct FragmentBudget {
    pub max_primitives: usize,
    pub max_sources: usize,
    pub max_shapes: usize,
    pub max_glyphs: usize,
    pub max_depth: usize,
    pub max_work: usize,
    pub max_bytes: usize,
}
impl Default for FragmentBudget {
    fn default() -> Self {
        let bytes = 128 * 1024 * 1024;
        Self {
            max_primitives: bytes / size_of::<Primitive>(),
            max_sources: bytes / size_of::<crate::clip::ShapeSnapshot>(),
            max_shapes: bytes / size_of::<ClipShape>(),
            max_glyphs: bytes / size_of::<crate::PositionedGlyph>(),
            max_depth: bytes / size_of::<Primitive>(),
            max_work: 100_000_000,
            max_bytes: bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentError {
    UnsupportedClipGeometry(u32),
    IncompatibleSource,
    Geometry(ClipError),
    KeyedQuadMismatch,
    UnbalancedClip,
    BudgetExceeded,
    UnsupportedTransform,
    UnsupportedPrimitive,
}
impl From<ClipError> for FragmentError {
    fn from(error: ClipError) -> Self {
        Self::Geometry(error)
    }
}

/// Translation plus the original overlay's per-primitive alpha multiplier.
/// This is not isolated group opacity. Scale other than 1 is unsupported.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FragmentTransform {
    pub translation: Point,
    pub scale: f32,
    pub opacity: f32,
}
impl Default for FragmentTransform {
    fn default() -> Self {
        Self {
            translation: Point::new(0., 0.),
            scale: 1.,
            opacity: 1.,
        }
    }
}

fn add(total: &mut usize, amount: usize, limit: usize) -> Result<(), FragmentError> {
    *total = total
        .checked_add(amount)
        .filter(|value| *value <= limit)
        .ok_or(FragmentError::BudgetExceeded)?;
    Ok(())
}
fn bytes(count: usize, stride: usize) -> Result<usize, FragmentError> {
    count
        .checked_mul(stride)
        .ok_or(FragmentError::BudgetExceeded)
}

impl Scene {
    /// Start a balanced fragment with the caller's immutable source inventory.
    /// Existing primitives and clip state are not copied. Sources cannot mint or mutate keys.
    pub fn fragment(&self) -> Self {
        Self {
            primitives: Vec::new(),
            clip_shapes: self.clip_shapes.clone(),
            required_clip_version: self.required_clip_version,
            composition_error: self.composition_error,
        }
    }

    /// Explicitly import a table snapshot through the same atomic composition gate.
    pub fn try_import_clip_shapes(
        &mut self,
        table: &crate::ShapeTable,
        budget: FragmentBudget,
    ) -> Result<(), FragmentError> {
        self.try_append_fragment(
            &Self::with_clip_shapes(table),
            budget,
            FragmentTransform::default(),
        )
    }

    /// Preflight both inventories and every primitive before changing the caller.
    /// The fragment must be balanced and cannot pop a caller scope. The caller may
    /// have open scopes: they continue to enclose all appended paint. Same-owner
    /// immutable snapshots extend only with an identical common prefix. Every old
    /// key/source slot remains valid. Foreign keys require an explicit source import.
    /// Device, target-scale, atlas and actual GPU work checks remain Renderer-owned.
    pub fn try_append_fragment(
        &mut self,
        fragment: &Self,
        budget: FragmentBudget,
        transform: FragmentTransform,
    ) -> Result<(), FragmentError> {
        if let Some(error) = self.composition_error.or(fragment.composition_error) {
            return Err(error);
        }
        if transform.scale != 1.
            || !transform.opacity.is_finite()
            || !(0. ..=1.).contains(&transform.opacity)
            || !transform.translation.x.is_finite()
            || !transform.translation.y.is_finite()
        {
            return Err(FragmentError::UnsupportedTransform);
        }
        for scene in [&*self, fragment] {
            if scene.required_clip_version > 1 {
                return Err(FragmentError::UnsupportedClipGeometry(
                    scene.required_clip_version,
                ));
            }
        }
        let mut work = 0;
        let mut primitive_count = self.len();
        add(&mut primitive_count, fragment.len(), budget.max_primitives)?;
        add(&mut work, primitive_count, budget.max_work)?;
        add(&mut work, self.clip_shapes.len(), budget.max_work)?;
        add(&mut work, fragment.clip_shapes.len(), budget.max_work)?;
        let mut sources: Vec<&crate::clip::ShapeSnapshot> = Vec::new();
        let mut shape_count = 0;
        for source in self.clip_shapes.iter().chain(&fragment.clip_shapes) {
            add(&mut work, sources.len(), budget.max_work)?;
            if let Some(index) = sources.iter().position(|old| old.same_owner(source)) {
                let old = sources[index];
                add(&mut work, old.len().min(source.len()), budget.max_work)?;
                if !old.compatible(source) {
                    return Err(FragmentError::IncompatibleSource);
                }
                if source.len() > old.len() {
                    add(
                        &mut shape_count,
                        source.len() - old.len(),
                        budget.max_shapes,
                    )?;
                    sources[index] = source;
                }
            } else {
                if sources.len() >= budget.max_sources {
                    return Err(FragmentError::BudgetExceeded);
                }
                add(&mut shape_count, source.len(), budget.max_shapes)?;
                sources.push(source);
            }
        }
        let mut storage = bytes(primitive_count, size_of::<Primitive>())?;
        add(
            &mut storage,
            bytes(shape_count, size_of::<ClipShape>())?,
            budget.max_bytes,
        )?;
        add(
            &mut storage,
            bytes(sources.len(), size_of::<crate::clip::ShapeSnapshot>())?,
            budget.max_bytes,
        )?;
        let mut caller_depth = 0usize;
        let mut glyphs = 0usize;
        for (scene, complete) in [(&*self, false), (fragment, true)] {
            let mut depth = 0;
            for primitive in &scene.primitives {
                match primitive {
                    Primitive::PushClip(_) => {
                        add(&mut depth, 1, budget.max_depth)?;
                    }
                    Primitive::PushShapeClip(key) => {
                        if scene.required_clip_version != 1 {
                            return Err(FragmentError::UnsupportedClipGeometry(
                                scene.required_clip_version,
                            ));
                        }
                        add(&mut work, sources.len(), budget.max_work)?;
                        resolve(&sources, key)?;
                        add(&mut depth, 1, budget.max_depth)?;
                    }
                    Primitive::PopClip => {
                        depth = depth.checked_sub(1).ok_or(FragmentError::UnbalancedClip)?;
                    }
                    Primitive::KeyedQuad { quad, outer } => {
                        if scene.required_clip_version != 1 {
                            return Err(FragmentError::UnsupportedClipGeometry(
                                scene.required_clip_version,
                            ));
                        }
                        let shape =
                            ClipShape::normalize(quad.rect, quad.corner_radii, Point::new(0., 0.))?;
                        add(&mut work, sources.len(), budget.max_work)?;
                        if shape != resolve(&sources, outer)? {
                            return Err(FragmentError::KeyedQuadMismatch);
                        }
                    }
                    Primitive::Glyphs(run) => {
                        add(&mut glyphs, run.glyphs.len(), budget.max_glyphs)?;
                    }
                    Primitive::Quad(_)
                    | Primitive::Shadow(_)
                    | Primitive::Image(_)
                    | Primitive::TransformedImage(_)
                    | Primitive::ControlledImage { .. }
                    | Primitive::ControlledTransformedImage { .. } => {}
                }
                if complete
                    && depth
                        .checked_add(caller_depth)
                        .is_none_or(|depth| depth > budget.max_depth)
                {
                    return Err(FragmentError::BudgetExceeded);
                }
            }
            if complete && depth != 0 {
                return Err(FragmentError::UnbalancedClip);
            }
            if !complete {
                caller_depth = depth;
            }
        }
        add(
            &mut storage,
            bytes(glyphs, size_of::<crate::PositionedGlyph>())?,
            budget.max_bytes,
        )?;
        // Account for cloning/traversing every glyph before staging any output.
        // Resolver reads above already charged their bounded source scans.
        add(&mut work, glyphs, budget.max_work)?;
        add(&mut work, shape_count, budget.max_work)?;
        // Translation requires a fresh derived key, never a reassignment of any
        // original source slot. Both keyed geometry and clips resolve that key.
        add(
            &mut work,
            bytes(
                fragment.len(),
                sources
                    .len()
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(FragmentError::BudgetExceeded)?,
            )?,
            budget.max_work,
        )?;
        let staged = fragment
            .primitives
            .iter()
            .map(|primitive| transform_primitive(primitive, &sources, transform))
            .collect::<Result<Vec<_>, _>>()?;
        let inventory = sources.into_iter().cloned().collect();
        // No ordinary fallible tail after this point.
        self.primitives.extend(staged);
        self.clip_shapes = inventory;
        self.required_clip_version = self
            .required_clip_version
            .max(fragment.required_clip_version);
        Ok(())
    }
}

fn translated_rect(rect: Rect, delta: Point) -> Result<Rect, FragmentError> {
    Ok(ClipShape::normalize(rect, CornerRadii::ZERO, delta)?.rect())
}
fn transform_primitive(
    primitive: &Primitive,
    sources: &[&crate::clip::ShapeSnapshot],
    transform: FragmentTransform,
) -> Result<Primitive, FragmentError> {
    let mut result = primitive.clone();
    let delta = transform.translation;
    let translated = delta.x != 0. || delta.y != 0.;
    let alpha = transform.opacity;
    match &mut result {
        Primitive::Quad(quad) => {
            if translated {
                quad.rect = translated_rect(quad.rect, delta)?;
            }
            quad.fill.a *= alpha;
            quad.border_color.a *= alpha;
        }
        Primitive::KeyedQuad { quad, outer } => {
            if translated {
                let current = resolve(sources, outer)?;
                *outer = outer.translated(current, delta)?;
                let shape = resolve(sources, outer)?;
                quad.rect = shape.rect();
                quad.corner_radii = shape.radii();
            }
            quad.fill.a *= alpha;
            quad.border_color.a *= alpha;
        }
        Primitive::PushShapeClip(key) => {
            if translated {
                let current = resolve(sources, key)?;
                *key = key.translated(current, delta)?;
                resolve(sources, key)?;
            }
        }
        Primitive::PushClip(rect) => {
            if translated {
                *rect = translated_rect(*rect, delta)?;
            }
        }
        Primitive::PopClip => {}
        Primitive::Shadow(shadow) => {
            if translated {
                shadow.rect = translated_rect(shadow.rect, delta)?;
            }
            shadow.color.a *= alpha;
        }
        Primitive::Glyphs(run) => {
            for glyph in &mut run.glyphs {
                if translated {
                    let point =
                        translated_rect(Rect::new(glyph.origin.x, glyph.origin.y, 0., 0.), delta)?
                            .origin;
                    glyph.origin = point;
                }
            }
            run.color.a *= alpha;
        }
        Primitive::Image(image) => {
            if translated {
                image.rect = translated_rect(image.rect, delta)?;
            }
            image.opacity *= alpha;
        }
        Primitive::TransformedImage(image) => {
            if translated {
                if image.rotation != 0. {
                    return Err(FragmentError::UnsupportedTransform);
                }
                image.quad.rect = translated_rect(image.quad.rect, delta)?;
            }
            image.quad.opacity *= alpha;
        }
        Primitive::ControlledImage { quad: image, .. } => {
            if translated {
                image.rect = translated_rect(image.rect, delta)?;
            }
            image.opacity *= alpha;
        }
        Primitive::ControlledTransformedImage { quad: image, .. } => {
            if translated {
                if image.rotation != 0. {
                    return Err(FragmentError::UnsupportedTransform);
                }
                image.quad.rect = translated_rect(image.quad.rect, delta)?;
            }
            image.quad.opacity *= alpha;
        }
    }
    Ok(result)
}

fn resolve(
    sources: &[&crate::clip::ShapeSnapshot],
    key: &crate::ShapeKey,
) -> Result<ClipShape, FragmentError> {
    for source in sources {
        match source.get(key) {
            Err(ClipError::StaleShapeKey) => {}
            result => return result.map_err(FragmentError::from),
        }
    }
    Err(FragmentError::Geometry(ClipError::StaleShapeKey))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impossible_from_public_api_conflicting_same_owner_snapshot_is_atomic() {
        let mut table = crate::ShapeTable::new(1, 1);
        table
            .intern(
                ClipShape::normalize(
                    Rect::new(0., 0., 8., 8.),
                    CornerRadii::ZERO,
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let mut destination = Scene::with_clip_shapes(&table);
        let before = destination.clone();
        for generation in [false, true] {
            let mut bad = before.fragment();
            bad.clip_shapes[0] = bad.clip_shapes[0].conflicting_for_test(generation);
            assert_eq!(
                destination.try_append_fragment(&bad, Default::default(), Default::default()),
                Err(FragmentError::IncompatibleSource)
            );
            assert_eq!(destination, before);
        }
    }
}
