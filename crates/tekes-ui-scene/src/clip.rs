//! Bounded CPU geometry for axis-aligned closed clip domains. No render capability is enabled.
//!
//! Inputs/identity are normalized f32; predicates, sections and arrangement use f64 with
//! outward-rounded intervals. An unresolved comparison is an error, never permission.
//! Visible bounds enclose the exact extrema; they are not a point-hit authority.
use crate::{CornerRadii, Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipError {
    InvalidInput,
    UnsupportedNumericRange,
    InputBudgetExceeded,
    QueryBudgetExceeded,
    NumericallyIndeterminate,
    StaleShapeKey,
}
type Result<T> = std::result::Result<T, ClipError>;

/// Caller-supplied resource policy. All upper bounds are checked before arrangement work.
#[derive(Clone, Copy, Debug)]
pub struct ClipBudget {
    pub max_shapes: usize,
    pub max_work: usize,
}

/// A normalized, immutable complete tuple. Full equality resolves any hash collision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipShape {
    bits: [u32; 8],
}

/// Closed axis-aligned bounds in physical pixels for a finite full-coverage
/// interior witness. Values are produced only by `ClipShape`'s checked helper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullCoverageInnerRectPx {
    bounds: [f32; 4],
}
impl FullCoverageInnerRectPx {
    pub fn bounds_px(self) -> [f32; 4] {
        self.bounds
    }
}

impl ClipShape {
    /// Translation is rounded to f32 exactly once, before identity is formed. Zero-area
    /// inputs are empty domains. Negative sizes/nonfinite values/overflow are rejected.
    pub fn normalize(rect: Rect, radii: CornerRadii, translation: Point) -> Result<Self> {
        let mut v = [
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
            radii.top_left,
            radii.top_right,
            radii.bottom_right,
            radii.bottom_left,
        ];
        if !v
            .iter()
            .chain([&translation.x, &translation.y])
            .all(|x| x.is_finite())
            || v[2] < 0.0
            || v[3] < 0.0
        {
            return Err(ClipError::InvalidInput);
        }
        v[0] += translation.x;
        v[1] += translation.y;
        if !v[0].is_finite()
            || !v[1].is_finite()
            || !(v[0] + v[2]).is_finite()
            || !(v[1] + v[3]).is_finite()
        {
            return Err(ClipError::InvalidInput);
        }
        let limit = v[2].min(v[3]) * 0.5;
        for r in &mut v[4..] {
            *r = r.clamp(0.0, limit);
        }
        // Supported nonzero normalized components: [2^-60,2^60], with largest/
        // smallest <=2^24. The latter leaves <=48 significant lattice bits across
        // f32 inputs, so f64 endpoints/centers (a few sums) remain exact. Squared
        // predicates use interval arithmetic. Outside is an error, never a clamp.
        if v.iter()
            .any(|x| x.abs() > 2_f32.powi(60) || (*x != 0.0 && x.abs() < 2_f32.powi(-60)))
        {
            return Err(ClipError::UnsupportedNumericRange);
        }
        let nonzero: Vec<_> = v
            .iter()
            .copied()
            .filter(|x| *x != 0.0)
            .map(f32::abs)
            .collect();
        if let (Some(lo), Some(hi)) = (
            nonzero.iter().copied().reduce(f32::min),
            nonzero.iter().copied().reduce(f32::max),
        ) && f64::from(hi) / f64::from(lo) > 2_f64.powi(24)
        {
            return Err(ClipError::UnsupportedNumericRange);
        }
        for x in &mut v {
            if *x == 0.0 {
                *x = 0.0;
            }
        }
        Ok(Self {
            bits: v.map(f32::to_bits),
        })
    }
    pub fn rect(self) -> Rect {
        let v = self.bits.map(f32::from_bits);
        Rect::new(v[0], v[1], v[2], v[3])
    }
    pub fn radii(self) -> CornerRadii {
        let v = self.bits.map(f32::from_bits);
        CornerRadii {
            top_left: v[4],
            top_right: v[5],
            bottom_right: v[6],
            bottom_left: v[7],
        }
    }
    pub fn is_rect(self) -> bool {
        self.bits[4..].iter().all(|x| *x == 0)
    }
    pub fn is_empty(self) -> bool {
        self.bits[2] == 0 || self.bits[3] == 0
    }

    /// Return a finite physical-pixel rectangle whose points are at least one
    /// device pixel inside the flat center of the shader's rounded rectangle.
    ///
    /// This witness is intentionally limited to the renderer's first boundary
    /// proof slice: origin zero, one sample, dimensions up to 4096, and one of
    /// five dyadic scales. It mirrors the clip shader's f32 operation order.
    /// `None` is unsupported input or a nonpositive safe rectangle; it never
    /// snaps values onto the supported grid.
    pub fn full_coverage_inner_rect_px(
        self,
        scale: f32,
        target_px: (u32, u32),
    ) -> Result<Option<FullCoverageInnerRectPx>> {
        if !(1..=4096).contains(&target_px.0)
            || !(1..=4096).contains(&target_px.1)
            || !(scale == 0.25 || scale == 0.5 || scale == 1.0 || scale == 2.0 || scale == 4.0)
        {
            return Ok(None);
        }

        // Shape fields are canonical f32 values admitted by normalize(), which
        // already enforces the original numeric range and aspect-ratio limits.
        let rect = self.rect();
        let radii = self.radii().to_array();
        let origin_px = [rect.origin.x * scale, rect.origin.y * scale];
        let width_px = rect.size.width * scale;
        let height_px = rect.size.height * scale;
        let half_px = [width_px * 0.5, height_px * 0.5];
        let center_px = [
            (rect.origin.x - 0.0) * scale + half_px[0],
            (rect.origin.y - 0.0) * scale + half_px[1],
        ];
        let physical_rect = [
            center_px[0] - half_px[0],
            center_px[1] - half_px[1],
            center_px[0] + half_px[0],
            center_px[1] + half_px[1],
        ];
        let radii_px = radii.map(|r| r * scale);
        let max_radius = radii_px.into_iter().fold(0.0_f32, f32::max);
        let inset = max_radius;
        let safe = [
            physical_rect[0] + inset + 1.0,
            physical_rect[1] + inset + 1.0,
            physical_rect[2] - inset - 1.0,
            physical_rect[3] - inset - 1.0,
        ];

        let exact_f32 = |expected: f64, actual: f32| {
            expected.is_finite() && actual.is_finite() && expected == f64::from(actual)
        };
        let exact = exact_f32(f64::from(rect.origin.x) * f64::from(scale), origin_px[0])
            && exact_f32(f64::from(rect.origin.y) * f64::from(scale), origin_px[1])
            && exact_f32(f64::from(rect.size.width) * f64::from(scale), width_px)
            && exact_f32(f64::from(rect.size.height) * f64::from(scale), height_px)
            && exact_f32(
                f64::from(rect.size.width) * f64::from(scale) * 0.5,
                half_px[0],
            )
            && exact_f32(
                f64::from(rect.size.height) * f64::from(scale) * 0.5,
                half_px[1],
            )
            && exact_f32(
                f64::from(rect.origin.x) * f64::from(scale) + f64::from(half_px[0]),
                center_px[0],
            )
            && exact_f32(
                f64::from(rect.origin.y) * f64::from(scale) + f64::from(half_px[1]),
                center_px[1],
            )
            && exact_f32(
                f64::from(center_px[0]) - f64::from(half_px[0]),
                physical_rect[0],
            )
            && exact_f32(
                f64::from(center_px[1]) - f64::from(half_px[1]),
                physical_rect[1],
            )
            && exact_f32(
                f64::from(center_px[0]) + f64::from(half_px[0]),
                physical_rect[2],
            )
            && exact_f32(
                f64::from(center_px[1]) + f64::from(half_px[1]),
                physical_rect[3],
            )
            && radii
                .iter()
                .zip(radii_px)
                .all(|(logical, actual)| exact_f32(f64::from(*logical) * f64::from(scale), actual))
            && exact_f32(f64::from(max_radius), inset)
            && exact_f32(
                f64::from(physical_rect[0]) + f64::from(inset) + 1.0,
                safe[0],
            )
            && exact_f32(
                f64::from(physical_rect[1]) + f64::from(inset) + 1.0,
                safe[1],
            )
            && exact_f32(
                f64::from(physical_rect[2]) - f64::from(inset) - 1.0,
                safe[2],
            )
            && exact_f32(
                f64::from(physical_rect[3]) - f64::from(inset) - 1.0,
                safe[3],
            );
        let all_grid = origin_px.into_iter().all(on_dyadic_pixel_grid)
            && on_dyadic_pixel_grid(width_px)
            && on_dyadic_pixel_grid(height_px)
            && half_px.into_iter().all(on_dyadic_pixel_grid)
            && center_px.into_iter().all(on_dyadic_pixel_grid)
            && physical_rect.into_iter().all(on_dyadic_pixel_grid)
            && radii_px.into_iter().all(on_dyadic_pixel_grid)
            && on_dyadic_pixel_grid(inset)
            && safe.into_iter().all(on_dyadic_pixel_grid);
        if !exact
            || !all_grid
            || origin_px.iter().any(|v| v.abs() > 4096.0)
            || physical_rect.iter().any(|v| v.abs() > 4096.0)
            || width_px > 4096.0
            || height_px > 4096.0
            || radii_px.iter().any(|v| *v > 4096.0)
        {
            return Ok(None);
        }
        if !(safe[0] < safe[2] && safe[1] < safe[3]) {
            return Ok(None);
        }
        Ok(Some(FullCoverageInnerRectPx { bounds: safe }))
    }

    /// Two conservative straight-side interior slabs of the same finite parent.
    /// The original center witness remains unchanged. Every slab is two physical
    /// pixels inside the outer edges and outside every corner quadrant, so at
    /// most one SDF q component is positive. On the inherited 1/256 grid and
    /// <=4096-pixel domain the nonzero length input is normal; WGSL's inherited
    /// sqrt/inverseSqrt/division error is below 1/16 pixel, leaving more than
    /// the 0.5-pixel clamp threshold. This does not certify curved AA-band input.
    pub fn full_coverage_axis_rects_px(
        self,
        scale: f32,
        target_px: (u32, u32),
    ) -> Result<Option<[FullCoverageInnerRectPx; 2]>> {
        if self
            .full_coverage_inner_rect_px(scale, target_px)?
            .is_none()
        {
            return Ok(None);
        }
        let r = self.rect();
        let half = [r.size.width * scale * 0.5, r.size.height * scale * 0.5];
        let center = [r.origin.x * scale + half[0], r.origin.y * scale + half[1]];
        let edge = [
            center[0] - half[0],
            center[1] - half[1],
            center[0] + half[0],
            center[1] + half[1],
        ];
        let radius = self
            .radii()
            .to_array()
            .into_iter()
            .map(|v| v * scale)
            .fold(0.0_f32, f32::max);
        let horizontal = [
            edge[0] + 2.0,
            edge[1] + radius + 2.0,
            edge[2] - 2.0,
            edge[3] - radius - 2.0,
        ];
        let vertical = [
            edge[0] + radius + 2.0,
            edge[1] + 2.0,
            edge[2] - radius - 2.0,
            edge[3] - 2.0,
        ];
        // All operands retain the previously checked physical dyadic grid;
        // bounded additions/subtractions stay exactly representable here.
        if [horizontal, vertical]
            .iter()
            .any(|b| b[0] >= b[2] || b[1] >= b[3])
        {
            return Ok(None);
        }
        Ok(Some([
            FullCoverageInnerRectPx { bounds: horizontal },
            FullCoverageInnerRectPx { bounds: vertical },
        ]))
    }

    fn values(self) -> [f64; 8] {
        self.bits.map(|x| f64::from(f32::from_bits(x)))
    }
    fn edges(self) -> [f64; 4] {
        let v = self.values();
        [v[0], v[1], v[0] + v[2], v[1] + v[3]]
    }
    /// Logical closed-domain hit testing. Legacy Rect::contains remains half-open.
    pub fn contains(self, p: [f64; 2]) -> Result<bool> {
        validate_point(p)?;
        if self.is_empty() {
            return Ok(false);
        }
        self.test([Iv::point(p[0]), Iv::point(p[1])], false)
    }
    fn test(self, p: [Iv; 2], strict: bool) -> Result<bool> {
        let e = self.edges();
        for (q, lo, hi) in [(p[0], e[0], e[2]), (p[1], e[1], e[3])] {
            if q.hi < lo || q.lo > hi || (strict && (q.hi <= lo || q.lo >= hi)) {
                return Ok(false);
            }
            if q.lo < lo || q.hi > hi || (strict && (q.lo <= lo || q.hi >= hi)) {
                return Err(ClipError::NumericallyIndeterminate);
            }
        }
        for b in self.boundary() {
            if let Boundary::Arc { c, r, s } = b {
                let d = [p[0] - Iv::point(c[0]), p[1] - Iv::point(c[1])];
                // A corner circle constrains only its outward quadrant.
                if d[0].scale(s[0]).hi <= 0.0 || d[1].scale(s[1]).hi <= 0.0 {
                    continue;
                }
                let dist = d[0].square() + d[1].square() - Iv::point(r).square();
                if dist.lo > 0.0 || (strict && dist.lo >= 0.0) {
                    return Ok(false);
                }
                if dist.hi > 0.0 || (strict && dist.hi >= 0.0) {
                    return Err(ClipError::NumericallyIndeterminate);
                }
            }
        }
        Ok(true)
    }
    /// A geometric inward offset. Border <= 0 returns the original contour; an
    /// over-thick border returns an empty inner domain, never a negative rectangle.
    /// No f32 re-rounding: this contour stays in the same f64 logical domain.
    pub fn inset(self, border: f32) -> Result<InnerClipShape> {
        if !border.is_finite() {
            return Err(ClipError::InvalidInput);
        }
        let b = f64::from(border.max(0.0));
        let v = self.values();
        let empty = self.is_empty() || 2.0 * b >= v[2] || 2.0 * b >= v[3];
        if empty {
            return Ok(InnerClipShape {
                outer: self,
                border: b,
                empty: true,
            });
        }
        let mut nonzero: Vec<_> = self
            .values()
            .into_iter()
            .filter(|x| *x != 0.0)
            .map(f64::abs)
            .collect();
        if b != 0.0 {
            nonzero.push(b);
        }
        if let (Some(lo), Some(hi)) = (
            nonzero.iter().copied().reduce(f64::min),
            nonzero.iter().copied().reduce(f64::max),
        ) && hi / lo > 2_f64.powi(24)
        {
            return Err(ClipError::UnsupportedNumericRange);
        }
        Ok(InnerClipShape {
            outer: self,
            border: b,
            empty,
        })
    }
    fn boundary(self) -> Vec<Boundary> {
        let v = self.values();
        let [x, y, x1, y1] = self.edges();
        let r = [v[4], v[5], v[6], v[7]];
        let mut out = vec![
            Boundary::Line {
                axis: 1,
                fixed: y,
                lo: x + r[0],
                hi: x1 - r[1],
            },
            Boundary::Line {
                axis: 0,
                fixed: x1,
                lo: y + r[1],
                hi: y1 - r[2],
            },
            Boundary::Line {
                axis: 1,
                fixed: y1,
                lo: x + r[3],
                hi: x1 - r[2],
            },
            Boundary::Line {
                axis: 0,
                fixed: x,
                lo: y + r[0],
                hi: y1 - r[3],
            },
        ];
        for (i, (c, s)) in [
            ([x + r[0], y + r[0]], [-1., -1.]),
            ([x1 - r[1], y + r[1]], [1., -1.]),
            ([x1 - r[2], y1 - r[2]], [1., 1.]),
            ([x + r[3], y1 - r[3]], [-1., 1.]),
        ]
        .into_iter()
        .enumerate()
        {
            if r[i] > 0.0 {
                out.push(Boundary::Arc { c, r: r[i], s });
            }
        }
        out
    }
    fn section(self, axis: usize, at: f64) -> Result<Option<[Iv; 2]>> {
        let e = self.edges();
        let a = axis;
        let b = 1 - axis;
        if self.is_empty() || at < e[b] || at > e[b + 2] {
            return Ok(None);
        }
        let mut lo = Iv::point(e[a]);
        let mut hi = Iv::point(e[a + 2]);
        for edge in self.boundary() {
            if let Boundary::Arc { c, r, s } = edge {
                let d = Iv::point(at) - Iv::point(c[b]);
                if d.scale(s[b]).hi <= 0.0 {
                    continue;
                }
                let h = (Iv::point(r).square() - d.square()).sqrt()?;
                if s[a] < 0.0 {
                    lo = lo.max(Iv::point(c[a]) - h);
                } else {
                    hi = hi.min(Iv::point(c[a]) + h);
                }
            }
        }
        Ok(Some([lo, hi]))
    }
}

/// Exact membership in the finite 1/256 physical-pixel domain.
fn on_dyadic_pixel_grid(value: f32) -> bool {
    let scaled = value * 256.0;
    value.is_finite() && scaled.is_finite() && scaled.fract() == 0.0
}

/// Exact offset membership is equivalent to the closed Euclidean ball of radius
/// border being contained in outer. For these circular corners this is the inset
/// rectangle plus radius max(r-border,0), and is necessarily a subset of outer.
#[derive(Clone, Copy, Debug)]
pub struct InnerClipShape {
    outer: ClipShape,
    border: f64,
    empty: bool,
}
impl InnerClipShape {
    pub fn is_empty(self) -> bool {
        self.empty
    }
    pub fn contains(self, p: [f64; 2]) -> Result<bool> {
        validate_point(p)?;
        if self.empty {
            return Ok(false);
        }
        // A ball is inside an axis-aligned rounded rectangle iff its four support
        // distances (including each active corner's offset circle) are nonnegative.
        let e = self.outer.edges();
        let b = self.border;
        if p[0] < e[0] + b || p[0] > e[2] - b || p[1] < e[1] + b || p[1] > e[3] - b {
            return Ok(false);
        }
        for edge in self.outer.boundary() {
            if let Boundary::Arc { c, r, s } = edge {
                let inner = (r - b).max(0.0);
                let d = [
                    Iv::point(p[0]) - Iv::point(c[0]),
                    Iv::point(p[1]) - Iv::point(c[1]),
                ];
                if d[0].scale(s[0]).hi > 0.0 && d[1].scale(s[1]).hi > 0.0 {
                    let v = d[0].square() + d[1].square() - Iv::point(inner).square();
                    if v.lo > 0.0 {
                        return Ok(false);
                    }
                    if v.hi > 0.0 {
                        return Err(ClipError::NumericallyIndeterminate);
                    }
                }
            }
        }
        // Explicit subset check prevents arithmetic drift authorizing an outer miss.
        self.outer.contains(p)
    }
}

// Fixed-size immutable result of sequential canonical translation. Admission
// remains anchored to the original immutable source slot; no transform history
// or second mutable geometry table exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct KeyDerivation {
    source: ClipShape,
    canonical: ClipShape,
}

#[derive(Clone, Debug)]
pub struct ShapeKey {
    // Retaining the allocation prevents address reuse while any key can be queried.
    // Arc<()> value equality is deliberately NOT used for owner identity.
    owner: std::sync::Arc<()>,
    generation: u64,
    index: usize,
    // A canonical result authorized by this original source owner/generation/slot.
    derived: Option<KeyDerivation>,
}
impl PartialEq for ShapeKey {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, &other.owner)
            && self.generation == other.generation
            && self.index == other.index
            && self.derived == other.derived
    }
}
impl Eq for ShapeKey {}
impl std::hash::Hash for ShapeKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(std::sync::Arc::as_ptr(&self.owner), state);
        std::hash::Hash::hash(&self.generation, state);
        std::hash::Hash::hash(&self.index, state);
        std::hash::Hash::hash(&self.derived, state);
    }
}
/// Every new table owns a fresh private allocation, independently of its supplied
/// geometry generation. Keys retain that identity; another table cannot authorize
/// them even with the same generation, slot and geometry. The mutable table is not
/// Clone, so two divergent intern inventories cannot share one owner identity.
///
/// ```compile_fail
/// use tekes_ui_scene::ShapeTable;
/// let table = ShapeTable::new(7, 1);
/// let divergent_table = table.clone();
/// ```
#[derive(Debug)]
pub struct ShapeTable {
    owner: std::sync::Arc<()>,
    generation: u64,
    limit: usize,
    shapes: Vec<ClipShape>,
}
impl ShapeTable {
    pub fn new(generation: u64, max_shapes: usize) -> Self {
        Self {
            owner: std::sync::Arc::new(()),
            generation,
            limit: max_shapes,
            shapes: Vec::new(),
        }
    }
    pub fn intern(&mut self, shape: ClipShape) -> Result<ShapeKey> {
        if let Some(index) = self.shapes.iter().position(|x| *x == shape) {
            return Ok(ShapeKey {
                owner: std::sync::Arc::clone(&self.owner),
                generation: self.generation,
                index,
                derived: None,
            });
        }
        if self.shapes.len() >= self.limit {
            return Err(ClipError::InputBudgetExceeded);
        }
        let key = ShapeKey {
            owner: std::sync::Arc::clone(&self.owner),
            generation: self.generation,
            index: self.shapes.len(),
            derived: None,
        };
        self.shapes.push(shape);
        Ok(key)
    }
    pub fn get(&self, key: ShapeKey) -> Result<ClipShape> {
        if !std::sync::Arc::ptr_eq(&key.owner, &self.owner) || key.generation != self.generation {
            return Err(ClipError::StaleShapeKey);
        }
        let shape = self
            .shapes
            .get(key.index)
            .copied()
            .ok_or(ClipError::StaleShapeKey)?;
        key.resolve(shape)
    }
}

/// A certified interval enclosure of a scalar exact endpoint (not a hit epsilon).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipInterval {
    pub lower: f64,
    pub upper: f64,
}
/// None means empty or zero area; Some is a positive-area intersection whose exact
/// min/max coordinates lie in these outward intervals. Errors propagate unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleClipBounds {
    pub min: [ClipInterval; 2],
    pub max: [ClipInterval; 2],
}
#[derive(Clone, Debug)]
pub struct ClipRegion {
    shapes: Vec<ClipShape>,
    work: usize,
}
impl ClipRegion {
    pub fn new(shapes: &[ClipShape], budget: ClipBudget) -> Result<Self> {
        if shapes.len() > budget.max_shapes {
            return Err(ClipError::InputBudgetExceeded);
        }
        // <=8 pieces/shape, <=2 roots/pair; 16 units/pair reserve endpoint equation
        // recovery as well as root classification, then each candidate's n domain
        // predicates. Remaining terms reserve extrema and interior section work.
        // Units bound geometric operations, not wall time. Charge before allocation.
        let n = shapes.len();
        if n == 0 {
            return Err(ClipError::InvalidInput);
        } // unbounded is not a finite visible rect
        let pieces = n.checked_mul(8).ok_or(ClipError::QueryBudgetExceeded)?;
        let work = pieces
            .checked_mul(pieces)
            .and_then(|x| x.checked_mul(16))
            .and_then(|x| pieces.checked_mul(4).and_then(|p| x.checked_add(p)))
            .and_then(|x| x.checked_mul(n))
            .and_then(|x| n.checked_mul(n).and_then(|p| x.checked_add(p)))
            .ok_or(ClipError::QueryBudgetExceeded)?;
        if work > budget.max_work {
            return Err(ClipError::QueryBudgetExceeded);
        }
        let mut unique = Vec::new();
        for shape in shapes {
            if !unique.contains(shape) {
                unique.push(*shape);
            }
        }
        Ok(Self {
            shapes: unique,
            work,
        })
    }
    pub fn reserved_work(&self) -> usize {
        self.work
    }
    pub fn contains(&self, p: [f64; 2]) -> Result<bool> {
        let mut uncertain = false;
        for s in &self.shapes {
            match s.contains(p) {
                Ok(false) => return Ok(false),
                Err(e) => {
                    if e != ClipError::NumericallyIndeterminate {
                        return Err(e);
                    }
                    uncertain = true;
                }
                _ => {}
            }
        }
        if uncertain {
            Err(ClipError::NumericallyIndeterminate)
        } else {
            Ok(true)
        }
    }
    /// axis=0: x interval at y=at; axis=1: y interval at x=at.
    pub fn section(&self, axis: usize, at: f64) -> Result<Option<[ClipInterval; 2]>> {
        if axis > 1 {
            return Err(ClipError::InvalidInput);
        }
        validate_point([at, 0.0])?;
        let mut result: Option<[Iv; 2]> = None;
        for s in &self.shapes {
            let Some([lo, hi]) = s.section(axis, at)? else {
                return Ok(None);
            };
            result = Some(match result {
                Some([a, b]) => [a.max(lo), b.min(hi)],
                None => [lo, hi],
            });
        }
        let [lo, hi] = result.ok_or(ClipError::InvalidInput)?;
        if lo.lo > hi.hi {
            return Ok(None);
        }
        if lo.hi > hi.lo {
            return Err(ClipError::NumericallyIndeterminate);
        }
        Ok(Some([lo.public(), hi.public()]))
    }
    /// Common interval valid over an entire orthogonal span. Convexity proves
    /// sufficiency of the two endpoint sections; this does not choose scroll offsets.
    pub fn section_for_span(
        &self,
        axis: usize,
        cross: [f64; 2],
    ) -> Result<Option<[ClipInterval; 2]>> {
        if cross[0] > cross[1] {
            return Err(ClipError::InvalidInput);
        }
        let Some(a) = self.section(axis, cross[0])? else {
            return Ok(None);
        };
        let Some(b) = self.section(axis, cross[1])? else {
            return Ok(None);
        };
        let lo = ClipInterval {
            lower: a[0].lower.max(b[0].lower),
            upper: a[0].upper.max(b[0].upper),
        };
        let hi = ClipInterval {
            lower: a[1].lower.min(b[1].lower),
            upper: a[1].upper.min(b[1].upper),
        };
        if lo.lower > hi.upper {
            return Ok(None);
        }
        if lo.upper > hi.lower {
            return Err(ClipError::NumericallyIndeterminate);
        }
        Ok(Some([lo, hi]))
    }
    /// Positive-area visibility, distinct from closed point containment at a tangent.
    pub fn has_positive_area(&self) -> Result<bool> {
        self.visible_bounds().map(|v| v.is_some())
    }
    pub fn visible_bounds(&self) -> Result<Option<VisibleClipBounds>> {
        if self.shapes.iter().any(|s| s.is_empty()) {
            return Ok(None);
        }
        let mut aabb = [
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::INFINITY,
        ];
        for s in &self.shapes {
            let e = s.edges();
            for a in 0..2 {
                aabb[a] = aabb[a].max(e[a]);
                aabb[a + 2] = aabb[a + 2].min(e[a + 2]);
            }
        }
        if aabb[0] >= aabb[2] || aabb[1] >= aabb[3] {
            return Ok(None);
        }
        let boundaries: Vec<_> = self.shapes.iter().map(|s| s.boundary()).collect();
        let mut candidates = Vec::new();
        for (i, edges) in boundaries.iter().enumerate() {
            for edge in edges {
                for p in edge.ends() {
                    candidates.push((p, i, i));
                }
            }
            for (j, previous) in boundaries.iter().enumerate().take(i) {
                for a in edges {
                    for b in previous {
                        for p in intersections(*a, *b, aabb)? {
                            candidates.push((p, i, j));
                        }
                    }
                }
            }
        }
        let mut accepted = Vec::new();
        let mut uncertain = false;
        for (p, i, j) in candidates {
            let mut ok = true;
            let mut maybe = false;
            for (k, s) in self.shapes.iter().enumerate() {
                if k == i || k == j {
                    continue;
                }
                match s.test(p, false) {
                    Ok(false) => {
                        ok = false;
                        break;
                    }
                    Err(ClipError::NumericallyIndeterminate) => maybe = true,
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            if ok {
                if maybe {
                    uncertain = true;
                } else {
                    accepted.push(p);
                }
            }
        }
        // No valid vertices/extrema => empty (containment is covered by every shape's
        // boundary endpoints, including straight corners and circular axis extrema).
        if uncertain {
            return Err(ClipError::NumericallyIndeterminate);
        }
        if accepted.is_empty() {
            return Ok(None);
        }
        let mut lo = accepted[0];
        let mut hi = accepted[0];
        for p in &accepted {
            for a in 0..2 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        if (0..2).any(|a| lo[a].lo == hi[a].hi) {
            return Ok(None);
        }
        // Positive area is certified by a strictly interior section through mid-y.
        // This is an interior witness after the full boundary arrangement, not sampling
        // as a substitute for intersection or extrema computation.
        let y = (lo[1].hi + hi[1].lo) * 0.5;
        if !(y > lo[1].hi && y < hi[1].lo) {
            return Err(ClipError::NumericallyIndeterminate);
        }
        let Some([left, right]) = self.section(0, y)? else {
            return Err(ClipError::NumericallyIndeterminate);
        };
        if left.upper >= right.lower {
            return Err(ClipError::NumericallyIndeterminate);
        }
        Ok(Some(VisibleClipBounds {
            min: lo.map(Iv::public),
            max: hi.map(Iv::public),
        }))
    }
}

fn validate_point(p: [f64; 2]) -> Result<()> {
    if p.iter().any(|x| {
        !x.is_finite() || x.abs() > 2_f64.powi(62) || (*x != 0.0 && x.abs() < 2_f64.powi(-62))
    }) {
        Err(ClipError::InvalidInput)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct Iv {
    lo: f64,
    hi: f64,
}
impl Iv {
    fn point(v: f64) -> Self {
        Self { lo: v, hi: v }
    }
    fn public(self) -> ClipInterval {
        ClipInterval {
            lower: self.lo,
            upper: self.hi,
        }
    }
    fn min(self, b: Self) -> Self {
        Self {
            lo: self.lo.min(b.lo),
            hi: self.hi.min(b.hi),
        }
    }
    fn max(self, b: Self) -> Self {
        Self {
            lo: self.lo.max(b.lo),
            hi: self.hi.max(b.hi),
        }
    }
    fn scale(self, b: f64) -> Self {
        self * Self::point(b)
    }
    fn square(self) -> Self {
        if self.lo >= 0.0 {
            self * self
        } else if self.hi <= 0.0 {
            let p = -self;
            p * p
        } else {
            Self {
                lo: 0.0,
                hi: (self.lo * self.lo).max(self.hi * self.hi).next_up(),
            }
        }
    }
    fn sqrt(self) -> Result<Self> {
        if self.lo < 0.0 {
            return Err(ClipError::NumericallyIndeterminate);
        }
        let a = self.lo.sqrt();
        let b = self.hi.sqrt();
        if self.lo == self.hi && a * a == self.lo && a.mul_add(a, -self.lo) == 0.0 {
            return Ok(Self::point(a));
        }
        Ok(Self {
            lo: a.next_down().max(0.0),
            hi: b.next_up(),
        })
    }
    fn div(self, b: Self) -> Result<Self> {
        if b.lo <= 0.0 && b.hi >= 0.0 {
            return Err(ClipError::NumericallyIndeterminate);
        }
        if self.lo == self.hi && b.lo == b.hi {
            let q = self.lo / b.lo;
            if q.mul_add(b.lo, -self.lo) == 0.0 {
                return Ok(Self::point(q));
            }
        }
        let inv = Self {
            lo: (1.0 / b.hi).next_down(),
            hi: (1.0 / b.lo).next_up(),
        };
        Ok(self * inv)
    }
}
impl std::ops::Neg for Iv {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}
impl std::ops::Add for Iv {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        if self.lo == self.hi && b.lo == b.hi {
            let s = self.lo + b.lo;
            let z = s - self.lo;
            let err = (self.lo - (s - z)) + (b.lo - z);
            if err == 0.0 {
                return Self::point(s);
            }
        }
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
}
impl std::ops::Sub for Iv {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        self + (-b)
    }
}
impl std::ops::Mul for Iv {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        if self.lo == self.hi && b.lo == b.hi {
            let p = self.lo * b.lo;
            if self.lo.mul_add(b.lo, -p) == 0.0 {
                return Self::point(p);
            }
        }
        let v = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self {
            lo: v.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: v.into_iter().fold(f64::NEG_INFINITY, f64::max).next_up(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Boundary {
    Line {
        axis: usize,
        fixed: f64,
        lo: f64,
        hi: f64,
    },
    Arc {
        c: [f64; 2],
        r: f64,
        s: [f64; 2],
    },
}
impl Boundary {
    fn ends(self) -> Vec<[Iv; 2]> {
        match self {
            Self::Line {
                axis,
                fixed,
                lo,
                hi,
            } => [lo, hi]
                .map(|v| {
                    let mut p = [Iv::point(v); 2];
                    p[axis] = Iv::point(fixed);
                    p
                })
                .to_vec(),
            Self::Arc { c, r, s } => vec![
                [Iv::point(c[0] + s[0] * r), Iv::point(c[1])],
                [Iv::point(c[0]), Iv::point(c[1] + s[1] * r)],
            ],
        }
    }
    fn equation_zero(self, p: [Iv; 2]) -> bool {
        let value = match self {
            Self::Line { axis, fixed, .. } => p[axis] - Iv::point(fixed),
            Self::Arc { c, r, .. } => {
                (p[0] - Iv::point(c[0])).square() + (p[1] - Iv::point(c[1])).square()
                    - Iv::point(r).square()
            }
        };
        value.lo == 0.0 && value.hi == 0.0
    }
    fn accepts(self, p: [Iv; 2]) -> Result<bool> {
        match self {
            Self::Line { axis, lo, hi, .. } => {
                let v = p[1 - axis];
                if v.hi < lo || v.lo > hi {
                    return Ok(false);
                }
                if v.lo < lo || v.hi > hi {
                    return Err(ClipError::NumericallyIndeterminate);
                }
                Ok(true)
            }
            Self::Arc { c, s, .. } => {
                let d = [
                    (p[0] - Iv::point(c[0])).scale(s[0]),
                    (p[1] - Iv::point(c[1])).scale(s[1]),
                ];
                if d.iter().any(|v| v.hi < 0.0) {
                    return Ok(false);
                }
                if d.iter().any(|v| v.lo < 0.0) {
                    return Err(ClipError::NumericallyIndeterminate);
                }
                Ok(true)
            }
        }
    }
}
fn intersections(a: Boundary, b: Boundary, limit: [f64; 4]) -> Result<Vec<[Iv; 2]>> {
    use Boundary::*;
    let mut points = Vec::new();
    match (a, b) {
        (
            Line {
                axis: i, fixed: x, ..
            },
            Line {
                axis: j, fixed: y, ..
            },
        ) => {
            if i != j {
                let mut p = [Iv::point(0.0); 2];
                p[i] = Iv::point(x);
                p[j] = Iv::point(y);
                points.push(p);
            }
            // Collinear overlap's extrema are already in the endpoint inventory.
        }
        (Line { axis, fixed, .. }, Arc { c, r, .. })
        | (Arc { c, r, .. }, Line { axis, fixed, .. }) => {
            let d = Iv::point(fixed) - Iv::point(c[axis]);
            let rad = Iv::point(r).square() - d.square();
            if rad.hi < 0.0 {
                return Ok(points);
            }
            let h = rad.sqrt()?;
            for sign in [-1.0, 1.0] {
                let mut p = [Iv::point(0.0); 2];
                p[axis] = Iv::point(fixed);
                p[1 - axis] = Iv::point(c[1 - axis]) + h.scale(sign);
                points.push(p);
            }
        }
        (Arc { c: u, r: r1, .. }, Arc { c: v, r: r2, .. }) => {
            if u == v {
                return Ok(points);
            } // coincident arcs: endpoints; concentric unequal: no crossing
            let d = [
                Iv::point(v[0]) - Iv::point(u[0]),
                Iv::point(v[1]) - Iv::point(u[1]),
            ];
            let d2 = d[0].square() + d[1].square();
            let outside = d2 - (Iv::point(r1) + Iv::point(r2)).square();
            let inside = d2 - (Iv::point(r1) - Iv::point(r2)).square();
            if outside.lo > 0.0 || inside.hi < 0.0 {
                return Ok(points);
            }
            if outside.hi > 0.0 || inside.lo < 0.0 {
                return Err(ClipError::NumericallyIndeterminate);
            }
            // P = u + t*d +/- k*perp(d), avoiding sqrt(d²) cancellation.
            let t = (Iv::point(r1).square() - Iv::point(r2).square() + d2).div(d2.scale(2.0))?;
            let h2 = Iv::point(r1).square() - t.square() * d2;
            let k =
                if outside.lo == 0.0 && outside.hi == 0.0 || inside.lo == 0.0 && inside.hi == 0.0 {
                    Iv::point(0.0)
                } else {
                    h2.div(d2)?.sqrt()?
                };
            for sign in [-1.0, 1.0] {
                points.push([
                    Iv::point(u[0]) + t * d[0] - k * d[1].scale(sign),
                    Iv::point(u[1]) + t * d[1] + k * d[0].scale(sign),
                ]);
            }
        }
    }
    // Algebraic endpoint recovery is permitted only when the two root boxes are
    // disjoint. An exact endpoint satisfying both equations then identifies the
    // unique root in that box. This is an exact equality proof, never epsilon snap.
    if points.len() == 2
        && (0..2).any(|axis| {
            points[0][axis].hi < points[1][axis].lo || points[1][axis].hi < points[0][axis].lo
        })
    {
        for p in &mut points {
            for endpoint in a.ends().into_iter().chain(b.ends()) {
                if (0..2)
                    .all(|axis| p[axis].lo <= endpoint[axis].lo && p[axis].hi >= endpoint[axis].hi)
                    && a.equation_zero(endpoint)
                    && b.equation_zero(endpoint)
                {
                    *p = endpoint;
                    break;
                }
            }
        }
    }
    let mut out = Vec::new();
    for p in points {
        if (0..2).any(|axis| p[axis].hi < limit[axis] || p[axis].lo > limit[axis + 2]) {
            continue;
        }
        match (a.accepts(p), b.accepts(p)) {
            (Ok(false), _) | (_, Ok(false)) => {}
            (Ok(true), Ok(true)) => out.push(p),
            (Err(e), _) | (_, Err(e)) => return Err(e),
        }
    }
    Ok(out)
}

// Immutable per-Scene registry. Unlike a mutable ShapeTable clone, two snapshots
// can never diverge at an existing slot or mint keys for different geometry.
#[derive(Clone, Debug)]
pub(crate) struct ShapeSnapshot {
    owner: std::sync::Arc<()>,
    generation: u64,
    shapes: Vec<ClipShape>,
}
impl PartialEq for ShapeSnapshot {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, &other.owner)
            && self.generation == other.generation
            && self.shapes == other.shapes
    }
}
impl ShapeSnapshot {
    pub(crate) fn get(&self, key: &ShapeKey) -> Result<ClipShape> {
        if !std::sync::Arc::ptr_eq(&self.owner, &key.owner) || self.generation != key.generation {
            return Err(ClipError::StaleShapeKey);
        }
        let shape = self
            .shapes
            .get(key.index)
            .copied()
            .ok_or(ClipError::StaleShapeKey)?;
        key.resolve(shape)
    }
    pub(crate) fn len(&self) -> usize {
        self.shapes.len()
    }
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, &other.owner)
    }
    pub(crate) fn compatible(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.shapes.iter().zip(&other.shapes).all(|(a, b)| a == b)
    }
}
impl ShapeKey {
    fn resolve(&self, source: ClipShape) -> Result<ClipShape> {
        match self.derived {
            Some(derived) if derived.source != source => Err(ClipError::StaleShapeKey),
            Some(derived) => Ok(derived.canonical),
            None => Ok(source),
        }
    }
    /// Apply this step to the already resolved canonical tuple, exactly like
    /// ordinary primitive coordinates. Never sum deltas against an older origin.
    pub(crate) fn translated(&self, current: ClipShape, delta: Point) -> Result<Self> {
        let source = match self.derived {
            Some(derived) if derived.canonical != current => return Err(ClipError::StaleShapeKey),
            Some(derived) => derived.source,
            None => current,
        };
        let canonical = ClipShape::normalize(current.rect(), current.radii(), delta)?;
        let mut result = self.clone();
        result.derived = (canonical != source).then_some(KeyDerivation { source, canonical });
        Ok(result)
    }
}

impl ShapeTable {
    pub(crate) fn snapshot(&self) -> ShapeSnapshot {
        ShapeSnapshot {
            owner: std::sync::Arc::clone(&self.owner),
            generation: self.generation,
            shapes: self.shapes.clone(),
        }
    }
}
#[cfg(test)]
impl ShapeSnapshot {
    pub(crate) fn conflicting_for_test(&self, generation: bool) -> Self {
        let mut result = self.clone();
        if generation {
            result.generation = result.generation.wrapping_add(1);
        } else {
            result.shapes[0] = ClipShape::normalize(
                Rect::new(4., 4., 8., 8.),
                CornerRadii::ZERO,
                Point::new(0., 0.),
            )
            .unwrap();
        }
        result
    }
}
#[cfg(test)]
mod sequential_key_authority {
    use super::*;
    #[test]
    fn immutable_derived_geometry_cannot_bypass_original_source_slot_tuple() {
        let source = ClipShape::normalize(
            Rect::new(1_000_000., 0., 8., 8.),
            CornerRadii::uniform(2.),
            Point::new(0., 0.),
        )
        .unwrap();
        let mut table = ShapeTable::new(9, 1);
        let original = table.intern(source).unwrap();
        let derived = original.translated(source, Point::new(0.0625, 0.)).unwrap();
        let mut conflicting = table.snapshot();
        conflicting.shapes[0] = ClipShape::normalize(
            Rect::new(f32::from_bits(0x4974_2402), 0., 8., 8.),
            CornerRadii::uniform(2.),
            Point::new(0., 0.),
        )
        .unwrap();
        assert_eq!(conflicting.get(&derived), Err(ClipError::StaleShapeKey));
        assert_eq!(
            table.get(derived).unwrap().rect().origin.x.to_bits(),
            0x49742401
        );
        assert_eq!(table.get(original), Ok(source));
    }
}

#[cfg(test)]
mod finite_full_coverage_witness {
    use super::*;

    #[test]
    fn returns_shader_order_dyadic_inner_rectangle() {
        let shape = ClipShape::normalize(
            Rect::new(0.0, 0.0, 64.0, 64.0),
            CornerRadii::uniform(4.0),
            Point::new(0.0, 0.0),
        )
        .unwrap();
        let rect = shape
            .full_coverage_inner_rect_px(1.0, (64, 64))
            .unwrap()
            .unwrap();
        assert_eq!(rect.bounds_px(), [5.0, 5.0, 59.0, 59.0]);
    }

    #[test]
    fn unsupported_scale_target_and_nondyadic_physical_values_are_not_snapped() {
        let shape = ClipShape::normalize(
            Rect::new(0.0, 0.0, 64.0, 64.0),
            CornerRadii::uniform(4.0),
            Point::new(0.0, 0.0),
        )
        .unwrap();
        assert_eq!(shape.full_coverage_inner_rect_px(0.3, (64, 64)), Ok(None));
        assert_eq!(shape.full_coverage_inner_rect_px(1.0, (4097, 64)), Ok(None));

        let fractional = ClipShape::normalize(
            Rect::new(0.001, 0.0, 64.0, 64.0),
            CornerRadii::ZERO,
            Point::new(0.0, 0.0),
        )
        .unwrap();
        assert_eq!(
            fractional.full_coverage_inner_rect_px(1.0, (64, 64)),
            Ok(None)
        );

        // Multiplying this origin is representable, but adding half the width
        // to its center rounds away the low bit in f32. The origin itself must
        // be checked so the later center cannot disguise that loss.
        let swallowed_origin = ClipShape::normalize(
            Rect::new(1.0 + 2.0_f32.powi(-23), 0.0, 64.0, 64.0),
            CornerRadii::ZERO,
            Point::new(0.0, 0.0),
        )
        .unwrap();
        assert_eq!(
            swallowed_origin.full_coverage_inner_rect_px(1.0, (64, 64)),
            Ok(None)
        );
    }
    #[test]
    fn axis_witness_covers_original_toolbar_scissor_but_not_outer_aa_band() {
        let shape = ClipShape::normalize(
            Rect::new(0., 0., 320., 144.),
            CornerRadii::uniform(10.),
            Point::new(0., 0.),
        )
        .unwrap();
        let [h, v] = shape
            .full_coverage_axis_rects_px(2., (640, 288))
            .unwrap()
            .unwrap();
        assert_eq!(h.bounds_px(), [2., 22., 638., 266.]);
        assert_eq!(v.bounds_px(), [22., 2., 618., 286.]);
        let old = shape
            .full_coverage_inner_rect_px(2., (640, 288))
            .unwrap()
            .unwrap()
            .bounds_px();
        assert_eq!(old, [21., 21., 619., 267.]);
        assert!(h.bounds_px()[0] <= 15. && h.bounds_px()[2] >= 625.);
        assert!(h.bounds_px()[1] <= 107. && h.bounds_px()[3] >= 181.);
        for b in [h.bounds_px(), v.bounds_px()] {
            assert!(b[0] >= 2. && b[1] >= 2. && b[2] <= 638. && b[3] <= 286.);
        }
        assert_eq!(shape.full_coverage_axis_rects_px(0.3, (640, 288)), Ok(None));
        assert_eq!(shape.full_coverage_axis_rects_px(2., (4097, 288)), Ok(None));
        let off_grid = ClipShape::normalize(
            Rect::new(0.001, 0., 320., 144.),
            CornerRadii::uniform(10.),
            Point::new(0., 0.),
        )
        .unwrap();
        assert_eq!(
            off_grid.full_coverage_axis_rects_px(2., (640, 288)),
            Ok(None)
        );
    }
}
