//! CPU-only complete-frame preflight. No queue writes, atlas mutation or encoders.
use crate::error::RenderError;
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use tekes_ui_scene::{
    ClipBudget, ClipError, ClipRegion, ClipShape, CornerRadii, GlyphAtlas, Point, Primitive, Quad,
    Rect, Scene,
};

pub const CLIP_GEOMETRY_VERSION: u32 = 1;
/// Explicit renderer resource policy, applied cumulatively to the complete frame.
/// Depth is limited by record capacity, not a separately guessed nesting constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipFrameBudget {
    pub max_primitives: u64,
    pub max_shapes: u64,
    pub max_records: u64,
    pub max_instances: u64,
    pub max_upload_bytes: u64,
    pub max_cpu_work: u64,
    pub max_shader_work: u64,
    pub max_queries: u64,
}
impl ClipFrameBudget {
    pub(crate) fn for_device(limits: &wgpu::Limits) -> Self {
        let uploads = limits.max_buffer_size.min(128 * 1024 * 1024);
        let records = (limits.max_storage_buffer_binding_size / 48)
            .min(
                limits.max_buffer_size
                    / u64::from(limits.min_uniform_buffer_offset_alignment.max(32)),
            )
            .saturating_sub(2)
            .min(u64::from(u32::MAX) - 1);
        Self {
            max_primitives: uploads / std::mem::size_of::<Primitive>() as u64,
            max_shapes: records,
            max_records: records,
            max_instances: uploads / 128,
            max_upload_bytes: uploads,
            max_cpu_work: 100_000_000,
            max_shader_work: 100_000_000,
            max_queries: records,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub(crate) struct ClipRecord {
    pub rect: [f32; 4],
    pub radii: [f32; 4],
    /// parent (one-based, zero root), canonical shape id, empty flag, actual depth.
    pub meta: [u32; 4],
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClipState {
    pub head: u32,
    pub binding_slot: u32,
    pub analytic: bool,
    pub scissor: [u32; 4],
    pub outer_id: u32,
    pub inner_empty: bool,
    pub quad_shape: Option<ClipShape>,
}
#[derive(Debug)]
pub(crate) struct ClipFrame {
    pub states: Vec<ClipState>,
    pub records: Vec<ClipRecord>,
    pub analytic: bool,
    pub cpu_work: u64,
    pub shader_work: u64,
    pub queries: u64,
    pub upload_bytes: u64,
}
#[derive(Clone, Copy)]
struct Scope {
    head: u32,
    scissor: [u32; 4],
    support: [u32; 4],
    analytic: bool,
    legacy: Rect,
}
fn geometry<T>(r: Result<T, ClipError>) -> Result<T, RenderError> {
    r.map_err(RenderError::ClipGeometry)
}
fn charge(total: &mut u64, n: u64, limit: u64, name: &'static str) -> Result<(), RenderError> {
    *total = total
        .checked_add(n)
        .ok_or(RenderError::ClipBudgetExceeded(name))?;
    if *total > limit {
        return Err(RenderError::ClipBudgetExceeded(name));
    }
    Ok(())
}
fn checked_product(a: u64, b: u64, name: &'static str) -> Result<u64, RenderError> {
    a.checked_mul(b)
        .ok_or(RenderError::ClipBudgetExceeded(name))
}
/// Return the pixel area of a prepared hardware scissor when it is provably
/// contained by the render target. `None` means the caller must conservatively
/// charge the full target. This consumes the final integer scissor, not an
/// inward analytic clip bound: the render pass uses the same rectangle.
fn scissor_pixel_area(scissor: Option<[u32; 4]>, target: (u32, u32)) -> Option<u64> {
    let [x, y, width, height] = scissor?;
    let right = x.checked_add(width)?;
    let bottom = y.checked_add(height)?;
    if right > target.0 || bottom > target.1 {
        return None;
    }
    u64::from(width).checked_mul(u64::from(height))
}
/// Build a temporary hardware scissor for one glyph run from the same prepared
/// instances the frame encoder will upload. Coordinates outside this deliberately
/// small integer domain use the parent's scissor unchanged.
fn glyph_run_scissor(
    run: &tekes_ui_scene::GlyphRun,
    scale: f32,
    parent: [u32; 4],
    target: (u32, u32),
) -> Option<[u32; 4]> {
    const DOMAIN: f64 = 16_384.0;
    const GUTTER: f64 = 16.0;
    if run.glyphs.is_empty()
        || !(1..=16_384).contains(&target.0)
        || !(1..=16_384).contains(&target.1)
    {
        return None;
    }

    let mut bounds: Option<[f64; 4]> = None;
    for glyph in &run.glyphs {
        let instance =
            crate::glyph_pass::GlyphInstance::new(glyph.origin, &glyph.glyph, run.color, scale)?;
        let [x, y, width, height] = instance.dest;
        if ![x, y, width, height]
            .into_iter()
            .all(|v| v.is_finite() && v.fract() == 0.0)
            || x.abs() as f64 > DOMAIN
            || y.abs() as f64 > DOMAIN
            || !(0.0..=DOMAIN).contains(&(width as f64))
            || !(0.0..=DOMAIN).contains(&(height as f64))
        {
            return None;
        }
        let right = f64::from(x) + f64::from(width);
        let bottom = f64::from(y) + f64::from(height);
        if !right.is_finite()
            || !bottom.is_finite()
            || right.fract() != 0.0
            || bottom.fract() != 0.0
            || right.abs() > DOMAIN
            || bottom.abs() > DOMAIN
        {
            return None;
        }
        bounds = Some(match bounds {
            Some([left, top, old_right, old_bottom]) => [
                left.min(f64::from(x)),
                top.min(f64::from(y)),
                old_right.max(right),
                old_bottom.max(bottom),
            ],
            None => [f64::from(x), f64::from(y), right, bottom],
        });
    }

    let [left, top, right, bottom] = bounds?;
    let parent_right = i64::from(parent[0]).checked_add(i64::from(parent[2]))?;
    let parent_bottom = i64::from(parent[1]).checked_add(i64::from(parent[3]))?;
    if parent_right > i64::from(target.0) || parent_bottom > i64::from(target.1) {
        return None;
    }
    let left = (left - GUTTER).max(f64::from(parent[0])).max(0.0);
    let top = (top - GUTTER).max(f64::from(parent[1])).max(0.0);
    let right = (right + GUTTER)
        .min(parent_right as f64)
        .min(f64::from(target.0));
    let bottom = (bottom + GUTTER)
        .min(parent_bottom as f64)
        .min(f64::from(target.1));
    if !(left.is_finite() && top.is_finite() && right.is_finite() && bottom.is_finite())
        || right < left
        || bottom < top
    {
        return None;
    }
    let x = left as u32;
    let y = top as u32;
    let right = right as u32;
    let bottom = bottom as u32;
    Some([x, y, right - x, bottom - y])
}
fn f32_gpu_scalar(v: f32, scale: f32) -> Result<(), RenderError> {
    let n = f64::from(v) * f64::from(scale);
    if !n.is_finite() || n.abs() > 2_f64.powi(60) || (n != 0. && n.abs() < 2_f64.powi(-60)) {
        return Err(RenderError::ClipGeometry(
            ClipError::UnsupportedNumericRange,
        ));
    }
    Ok(())
}
fn gpu_shape(shape: ClipShape, scale: f32) -> Result<(), RenderError> {
    let r = shape.rect();
    for v in [r.origin.x, r.origin.y, r.size.width, r.size.height]
        .into_iter()
        .chain(shape.radii().to_array())
    {
        f32_gpu_scalar(v, scale)?;
    }
    Ok(())
}
fn normalize(rect: Rect, radii: CornerRadii, scale: f32) -> Result<ClipShape, RenderError> {
    let s = geometry(ClipShape::normalize(rect, radii, Point::new(0., 0.)))?;
    gpu_shape(s, scale)?;
    Ok(s)
}
pub(crate) fn validate_color(c: tekes_ui_tokens::Rgba) -> Result<(), RenderError> {
    if [c.r, c.g, c.b, c.a]
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(RenderError::InvalidClipFrame(
            "analytic path requires finite unit-range RGBA",
        ));
    }
    Ok(())
}
fn validate_opacity(a: f32) -> Result<(), RenderError> {
    if !a.is_finite() {
        return Err(RenderError::InvalidClipFrame("nonfinite image opacity"));
    }
    Ok(())
}
fn quad_shape(q: &Quad, scale: f32) -> Result<(ClipShape, bool), RenderError> {
    validate_color(q.fill)?;
    validate_color(q.border_color)?;
    let s = normalize(q.rect, q.corner_radii, scale)?;
    if !q.border_width.is_finite() {
        return Err(RenderError::InvalidClipFrame("nonfinite border"));
    }
    let inner = geometry(s.inset(q.border_width))?;
    if !inner.is_empty() {
        f32_gpu_scalar(q.border_width.max(0.), scale)?;
    }
    Ok((s, inner.is_empty()))
}
fn shape_id(
    map: &mut HashMap<ClipShape, u32>,
    shape: ClipShape,
    budget: ClipFrameBudget,
) -> Result<u32, RenderError> {
    if let Some(id) = map.get(&shape) {
        return Ok(*id);
    }
    if map.len() as u64 >= budget.max_shapes || map.len() >= u32::MAX as usize {
        return Err(RenderError::ClipBudgetExceeded("shapes"));
    }
    let id = map.len() as u32 + 1;
    map.insert(shape, id);
    Ok(id)
}
fn intersect_scissor(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    [
        x,
        y,
        (a[0] + a[2]).min(b[0] + b[2]).saturating_sub(x),
        (a[1] + a[3]).min(b[1] + b[3]).saturating_sub(y),
    ]
}
/// Half a physical pixel is the analytic coverage function's outward support.
/// Pixel centers are already builtin position i+0.5; no second half-pixel shift.
fn support_scissor(shape: ClipShape, scale: f32, size: (u32, u32)) -> [u32; 4] {
    if shape.is_empty() {
        return [0; 4];
    }
    let r = shape.rect();
    let s = f64::from(scale);
    let x = (f64::from(r.origin.x) * s - 0.5)
        .floor()
        .clamp(0., f64::from(size.0)) as u32;
    let y = (f64::from(r.origin.y) * s - 0.5)
        .floor()
        .clamp(0., f64::from(size.1)) as u32;
    let right = ((f64::from(r.origin.x) + f64::from(r.size.width)) * s + 0.5)
        .ceil()
        .clamp(f64::from(x), f64::from(size.0)) as u32;
    let bottom = ((f64::from(r.origin.y) + f64::from(r.size.height)) * s + 0.5)
        .ceil()
        .clamp(f64::from(y), f64::from(size.1)) as u32;
    [x, y, right - x, bottom - y]
}
fn legacy_scissor(rect: Rect, scale: f32, size: (u32, u32)) -> [u32; 4] {
    let x = (rect.min_x() * scale).ceil().max(0.).min(size.0 as f32) as u32;
    let y = (rect.min_y() * scale).ceil().max(0.).min(size.1 as f32) as u32;
    let r = (rect.max_x() * scale)
        .floor()
        .max(x as f32)
        .min(size.0 as f32) as u32;
    let b = (rect.max_y() * scale)
        .floor()
        .max(y as f32)
        .min(size.1 as f32) as u32;
    [x, y, r - x, b - y]
}
/// Independently verify the built upload's bounded DAG, indices and declared depth.
/// This is also the final gate used for intentionally malformed internal QA records.
pub(crate) fn validate_records(
    records: &[ClipRecord],
    shapes: usize,
    limit: u64,
) -> Result<(), RenderError> {
    if records.len() as u64 > limit {
        return Err(RenderError::ClipBudgetExceeded("records"));
    }
    let mut identities = HashMap::new();
    for (i, r) in records.iter().enumerate() {
        let [parent, id, empty, depth] = r.meta;
        if parent as usize > i || id == 0 || id as usize > shapes || empty > 1 {
            return Err(RenderError::InvalidClipFrame(
                "clip record index/kind/cycle",
            ));
        }
        let actual = if parent == 0 {
            1
        } else {
            records[parent as usize - 1].meta[3]
                .checked_add(1)
                .ok_or(RenderError::InvalidClipFrame("clip depth overflow"))?
        };
        if depth != actual {
            return Err(RenderError::InvalidClipFrame("clip record depth"));
        }
        if !r.rect.iter().chain(&r.radii).all(|v| v.is_finite()) {
            return Err(RenderError::InvalidClipFrame("nonfinite clip record"));
        }
        let shape = geometry(ClipShape::normalize(
            Rect::new(r.rect[0], r.rect[1], r.rect[2], r.rect[3]),
            CornerRadii {
                top_left: r.radii[0],
                top_right: r.radii[1],
                bottom_right: r.radii[2],
                bottom_left: r.radii[3],
            },
            Point::new(0., 0.),
        ))?;
        let rect = shape.rect();
        let normalized = [
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ];
        if normalized.map(f32::to_bits) != r.rect.map(f32::to_bits)
            || shape.radii().to_array().map(f32::to_bits) != r.radii.map(f32::to_bits)
            || u32::from(shape.is_empty()) != empty
        {
            return Err(RenderError::InvalidClipFrame("noncanonical clip record"));
        }
        if let Some(previous) = identities.insert(id, shape)
            && previous != shape
        {
            return Err(RenderError::InvalidClipFrame("clip identity reuse"));
        }
        let mut cursor = parent;
        while cursor != 0 {
            let ancestor = &records[cursor as usize - 1];
            if ancestor.meta[1] == id {
                return Err(RenderError::InvalidClipFrame(
                    "duplicate clip identity in chain",
                ));
            }
            cursor = ancestor.meta[0];
        }
    }
    Ok(())
}

impl ClipFrame {
    pub(crate) fn uniform_slots(&self) -> usize {
        if self.analytic {
            self.records.len() + 2
        } else {
            1
        }
    }
    pub(crate) fn prepare(
        scene: &Scene,
        atlas: &GlyphAtlas,
        size: (u32, u32),
        scale: f32,
        budget: ClipFrameBudget,
        limits: &wgpu::Limits,
    ) -> Result<Self, RenderError> {
        if let Some(error) = scene.composition_error() {
            return Err(RenderError::SceneComposition(error));
        }
        if !scale.is_finite() || scale <= 0. || size.0 == 0 || size.1 == 0 {
            return Err(RenderError::InvalidArgument("invalid clip target"));
        }
        if size.0 > limits.max_texture_dimension_2d || size.1 > limits.max_texture_dimension_2d {
            return Err(RenderError::InvalidArgument(
                "target exceeds device dimensions",
            ));
        }
        let analytic = scene.required_clip_geometry() != 0
            || scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::PushShapeClip(_) | Primitive::KeyedQuad { .. }));
        if analytic && scene.required_clip_geometry() != CLIP_GEOMETRY_VERSION {
            return Err(RenderError::UnsupportedClipGeometry(
                scene.required_clip_geometry(),
            ));
        }
        if scene.len() as u64 > budget.max_primitives {
            return Err(RenderError::ClipBudgetExceeded("primitives"));
        }
        if scene.clip_shape_count() as u64 > budget.max_shapes {
            return Err(RenderError::ClipBudgetExceeded("shapes"));
        }
        let full = Rect::new(0., 0., size.0 as f32 / scale, size.1 as f32 / scale);
        if analytic && (!full.size.width.is_finite() || !full.size.height.is_finite()) {
            return Err(RenderError::ClipGeometry(
                ClipError::UnsupportedNumericRange,
            ));
        }
        let mut scope = vec![Scope {
            head: 0,
            scissor: [0, 0, size.0, size.1],
            support: [0, 0, size.0, size.1],
            analytic: false,
            legacy: full,
        }];
        let mut cpu_work = 0;
        charge(
            &mut cpu_work,
            scene.len() as u64,
            budget.max_cpu_work,
            "CPU work",
        )?;
        let mut result = Self {
            states: Vec::with_capacity(scene.len()),
            records: Vec::new(),
            analytic,
            cpu_work,
            shader_work: 0,
            queries: 0,
            upload_bytes: 0,
        };
        let mut shapes = HashMap::new();
        let mut chain_cache = HashMap::new();
        let mut record_shapes = Vec::new();
        let mut quads = 0_u64;
        let mut glyphs = 0_u64;
        let mut images = 0_u64;
        for p in scene.primitives() {
            let current = *scope.last().unwrap();
            let mut state = ClipState {
                head: if current.analytic { current.head } else { 0 },
                binding_slot: 0,
                analytic: current.analytic,
                scissor: current.scissor,
                outer_id: 0,
                inner_empty: false,
                quad_shape: None,
            };
            match p {
                Primitive::PushClip(rect) if !analytic => {
                    let clip = current
                        .legacy
                        .intersection(rect)
                        .unwrap_or(Rect::new(0., 0., 0., 0.));
                    scope.push(Scope {
                        head: 0,
                        scissor: legacy_scissor(clip, scale, size),
                        support: current.support,
                        analytic: false,
                        legacy: clip,
                    });
                }
                Primitive::PushClip(_) | Primitive::PushShapeClip(_) => {
                    let shape = match p {
                        Primitive::PushClip(rect) => normalize(*rect, CornerRadii::ZERO, scale)?,
                        Primitive::PushShapeClip(key) => {
                            let s = geometry(scene.clip_shape(key))?;
                            gpu_shape(s, scale)?;
                            s
                        }
                        _ => unreachable!(),
                    };
                    let id = shape_id(&mut shapes, shape, budget)?;
                    let mut cursor = current.head;
                    let mut duplicate = false;
                    while cursor != 0 {
                        charge(&mut result.cpu_work, 1, budget.max_cpu_work, "CPU work")?;
                        let r = &result.records[cursor as usize - 1];
                        if r.meta[1] == id {
                            duplicate = true;
                            break;
                        }
                        cursor = r.meta[0];
                    }
                    let head = if duplicate {
                        current.head
                    } else if let Some(head) = chain_cache.get(&(current.head, id)) {
                        *head
                    } else {
                        if result.records.len() as u64 >= budget.max_records
                            || result.records.len() >= u32::MAX as usize
                        {
                            return Err(RenderError::ClipBudgetExceeded("records"));
                        }
                        let mut domains = vec![shape];
                        let mut cursor = current.head;
                        while cursor != 0 {
                            domains.push(record_shapes[cursor as usize - 1]);
                            cursor = result.records[cursor as usize - 1].meta[0];
                        }
                        charge(&mut result.queries, 1, budget.max_queries, "queries")?;
                        let remaining = budget.max_cpu_work - result.cpu_work;
                        let region = geometry(ClipRegion::new(
                            &domains,
                            ClipBudget {
                                max_shapes: budget.max_shapes.min(usize::MAX as u64) as usize,
                                max_work: remaining.min(usize::MAX as u64) as usize,
                            },
                        ))?;
                        charge(
                            &mut result.cpu_work,
                            region.reserved_work() as u64,
                            budget.max_cpu_work,
                            "CPU work",
                        )?;
                        // Used as a numerical validation query, not as an inward AA scissor.
                        // Zero logical area may still have overlapping analytic AA support.
                        let _logical_bounds = geometry(region.visible_bounds())?;
                        let r = shape.rect();
                        let depth = if current.head == 0 {
                            1
                        } else {
                            result.records[current.head as usize - 1].meta[3] + 1
                        };
                        result.records.push(ClipRecord {
                            rect: [r.origin.x, r.origin.y, r.size.width, r.size.height],
                            radii: shape.radii().to_array(),
                            meta: [current.head, id, u32::from(shape.is_empty()), depth],
                        });
                        record_shapes.push(shape);
                        let head = result.records.len() as u32;
                        chain_cache.insert((current.head, id), head);
                        head
                    };
                    // Retain both policies. A raw Rect stays legacy outside an
                    // explicit shape scope, but its AA support is ready for promotion
                    // when a descendant shape (or an explicitly keyed draw) needs it.
                    let support =
                        intersect_scissor(current.support, support_scissor(shape, scale, size));
                    let active = current.analytic || matches!(p, Primitive::PushShapeClip(_));
                    let legacy = if let Primitive::PushClip(rect) = p {
                        current
                            .legacy
                            .intersection(rect)
                            .unwrap_or(Rect::new(0., 0., 0., 0.))
                    } else {
                        current.legacy
                    };
                    scope.push(Scope {
                        head,
                        scissor: if active {
                            support
                        } else {
                            legacy_scissor(legacy, scale, size)
                        },
                        support,
                        analytic: active,
                        legacy,
                    });
                }
                Primitive::PopClip => {
                    if scope.len() > 1 {
                        scope.pop();
                    } else if analytic {
                        return Err(RenderError::InvalidClipFrame("unmatched clip pop"));
                    }
                }
                Primitive::Quad(q) | Primitive::KeyedQuad { quad: q, .. } => {
                    quads = quads
                        .checked_add(1)
                        .ok_or(RenderError::ClipBudgetExceeded("instances"))?;
                    if analytic {
                        let (shape, empty) = quad_shape(q, scale)?;
                        if let Primitive::KeyedQuad { outer, .. } = p {
                            state.analytic = true;
                            state.head = current.head;
                            state.scissor = current.support;
                            let keyed = geometry(scene.clip_shape(outer))?;
                            if keyed != shape {
                                return Err(RenderError::InvalidClipFrame(
                                    "quad outer key/normalized geometry mismatch",
                                ));
                            }
                            state.outer_id = shape_id(&mut shapes, keyed, budget)?;
                        }
                        if state.analytic {
                            state.quad_shape = Some(shape);
                            state.inner_empty = empty;
                        }
                    }
                }
                Primitive::Shadow(s) => {
                    quads = quads
                        .checked_add(1)
                        .ok_or(RenderError::ClipBudgetExceeded("instances"))?;
                    if analytic {
                        validate_color(s.color)?;
                        normalize(s.rect, s.corner_radii, scale)?;
                        for v in [s.offset.x, s.offset.y, s.blur_radius, s.outset] {
                            f32_gpu_scalar(v, scale)?;
                        }
                    }
                }
                Primitive::Glyphs(run) => {
                    glyphs = glyphs
                        .checked_add(run.glyphs.len() as u64)
                        .ok_or(RenderError::ClipBudgetExceeded("instances"))?;
                    if analytic {
                        validate_color(run.color)?;
                        charge(
                            &mut result.cpu_work,
                            run.glyphs.len() as u64,
                            budget.max_cpu_work,
                            "glyph validation work",
                        )?;
                        for g in &run.glyphs {
                            f32_gpu_scalar(g.origin.x, scale)?;
                            f32_gpu_scalar(g.origin.y, scale)?;
                            if g.glyph.page >= atlas.page_count()
                                || u64::from(g.glyph.x) + u64::from(g.glyph.width)
                                    > u64::from(atlas.page_size())
                                || u64::from(g.glyph.y) + u64::from(g.glyph.height)
                                    > u64::from(atlas.page_size())
                            {
                                return Err(RenderError::InvalidClipFrame(
                                    "invalid glyph atlas region",
                                ));
                            }
                        }
                    }
                }
                Primitive::Image(q) | Primitive::ControlledImage { quad: q, .. } => {
                    images += 1;
                    if analytic {
                        validate_opacity(q.opacity)?;
                        normalize(q.rect, CornerRadii::ZERO, scale)?;
                    }
                }
                Primitive::TransformedImage(q)
                | Primitive::ControlledTransformedImage { quad: q, .. } => {
                    images += 1;
                    if analytic {
                        validate_opacity(q.quad.opacity)?;
                        normalize(q.quad.rect, CornerRadii::ZERO, scale)?;
                        if ![
                            q.uv.origin.x,
                            q.uv.origin.y,
                            q.uv.size.width,
                            q.uv.size.height,
                            q.uv.max_x(),
                            q.uv.max_y(),
                        ]
                        .iter()
                        .all(|v| v.is_finite())
                            || q.uv.size.width < 0.
                            || q.uv.size.height < 0.
                        {
                            return Err(RenderError::InvalidClipFrame("invalid image UV geometry"));
                        }
                        if !q.rotation.is_finite() {
                            return Err(RenderError::InvalidClipFrame("nonfinite image transform"));
                        }
                    }
                }
                _ => {
                    return Err(RenderError::InvalidClipFrame(
                        "unsupported primitive capability",
                    ));
                }
            }
            if scope.len() as u64 > budget.max_primitives.saturating_add(1) {
                return Err(RenderError::ClipBudgetExceeded("stack"));
            }
            let next = *scope.last().unwrap();
            if matches!(
                p,
                Primitive::PushClip(_) | Primitive::PushShapeClip(_) | Primitive::PopClip
            ) {
                state.analytic = next.analytic;
                state.head = if next.analytic { next.head } else { 0 };
                state.scissor = next.scissor;
            }
            state.binding_slot = if state.analytic {
                state
                    .head
                    .checked_add(1)
                    .ok_or(RenderError::ClipBudgetExceeded("uniform index"))?
            } else {
                0
            };
            if state.analytic
                && let Primitive::Glyphs(run) = p
                && let Some(scissor) = glyph_run_scissor(run, scale, state.scissor, size)
            {
                state.scissor = scissor;
            }
            if state.analytic
                && !matches!(
                    p,
                    Primitive::PushClip(_) | Primitive::PushShapeClip(_) | Primitive::PopClip
                )
            {
                let depth = if state.head == 0 {
                    0
                } else {
                    result.records[state.head as usize - 1].meta[3] as u64
                };
                let count = if let Primitive::Glyphs(run) = p {
                    run.glyphs.len() as u64
                } else {
                    1
                };
                let target_pixels =
                    checked_product(u64::from(size.0), u64::from(size.1), "shader work")?;
                let pixels = if matches!(p, Primitive::Glyphs(_)) {
                    scissor_pixel_area(Some(state.scissor), size).unwrap_or(target_pixels)
                } else {
                    target_pixels
                };
                let work = checked_product(
                    checked_product(pixels, depth, "shader work")?,
                    count,
                    "shader work",
                )?;
                charge(
                    &mut result.shader_work,
                    work,
                    budget.max_shader_work,
                    "shader work",
                )?;
            }
            let instances = quads
                .checked_add(glyphs)
                .and_then(|n| n.checked_add(images))
                .ok_or(RenderError::ClipBudgetExceeded("instances"))?;
            if instances > budget.max_instances || instances > u64::from(u32::MAX) {
                return Err(RenderError::ClipBudgetExceeded("instances"));
            }
            result.states.push(state);
        }
        if analytic && scope.len() != 1 {
            return Err(RenderError::InvalidClipFrame("unclosed clip scope"));
        }
        let count = result.records.len() as u64;
        let validation = checked_product(
            count,
            count
                .checked_add(1)
                .ok_or(RenderError::ClipBudgetExceeded("validation work"))?,
            "validation work",
        )? / 2;
        charge(
            &mut result.cpu_work,
            validation,
            budget.max_cpu_work,
            "validation CPU work",
        )?;
        validate_records(&result.records, shapes.len(), budget.max_records)?;
        let alignment = u64::from(limits.min_uniform_buffer_offset_alignment.max(32));
        let globals = checked_product(result.uniform_slots() as u64, alignment, "uniform upload")?;
        if globals > limits.max_buffer_size
            || globals.saturating_sub(alignment) > u64::from(u32::MAX)
        {
            return Err(RenderError::ClipBudgetExceeded("uniform device capacity"));
        }
        let record_bytes =
            checked_product(result.records.len().max(1) as u64, 48, "record upload")?;
        if record_bytes > limits.max_storage_buffer_binding_size
            || record_bytes > limits.max_buffer_size
        {
            return Err(RenderError::ClipBudgetExceeded("record device capacity"));
        }
        let quad_bytes = checked_product(
            quads,
            std::mem::size_of::<crate::instance::QuadInstance>() as u64,
            "quad upload",
        )?;
        let glyph_bytes = checked_product(
            glyphs,
            std::mem::size_of::<crate::glyph_pass::GlyphInstance>() as u64,
            "glyph upload",
        )?;
        let image_bytes = checked_product(
            images,
            std::mem::size_of::<crate::image_pass::ImageInstance>() as u64,
            "image upload",
        )?;
        for bytes in [quad_bytes, glyph_bytes, image_bytes] {
            if bytes > limits.max_buffer_size {
                return Err(RenderError::ClipBudgetExceeded("instance device capacity"));
            }
        }
        // Conservative full-atlas bound includes sync writes, even if only a dirty
        // subsection is actually uploaded. Registered images are separate APIs.
        let atlas_bytes = checked_product(
            checked_product(
                u64::from(atlas.page_size()),
                u64::from(atlas.page_size()),
                "atlas upload",
            )?,
            u64::from(atlas.page_count()) * 4,
            "atlas upload",
        )?;
        for bytes in [
            globals,
            record_bytes,
            quad_bytes,
            glyph_bytes,
            image_bytes,
            atlas_bytes,
        ] {
            charge(
                &mut result.upload_bytes,
                bytes,
                budget.max_upload_bytes,
                "upload bytes",
            )?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{
        AtlasConfig, FontKey, GlyphKey, GlyphRun, ImageId, ImageQuad, Point, PositionedGlyph,
        RasterFormat, RasterGlyph, ShapeTable,
    };
    use tekes_ui_tokens::Rgba;
    fn shape(x: f32) -> ClipShape {
        ClipShape::normalize(
            Rect::new(x, 0., 8., 8.),
            CornerRadii::uniform(2.),
            Point::new(0., 0.),
        )
        .unwrap()
    }
    fn scene() -> (Scene, ShapeTable, tekes_ui_scene::ShapeKey) {
        let mut table = ShapeTable::new(7, 8);
        let key = table.intern(shape(0.)).unwrap();
        (Scene::with_clip_shapes(&table), table, key)
    }
    fn prepare(scene: &Scene, budget: Option<ClipFrameBudget>) -> Result<ClipFrame, RenderError> {
        let limits = wgpu::Limits::default();
        ClipFrame::prepare(
            scene,
            &GlyphAtlas::default(),
            (32, 32),
            1.,
            budget.unwrap_or(ClipFrameBudget::for_device(&limits)),
            &limits,
        )
    }

    fn atlas_with_glyph() -> (GlyphAtlas, tekes_ui_scene::AtlasGlyph) {
        let mut atlas = GlyphAtlas::new(AtlasConfig {
            page_size: 16,
            max_pages: 1,
            padding: 1,
        });
        let glyph = atlas
            .get_or_insert_with(GlyphKey::new(FontKey(1), 1, 1000), || {
                Some(RasterGlyph {
                    left: 0,
                    top: 0,
                    width: 1,
                    height: 1,
                    format: RasterFormat::Mask,
                    pixels: vec![0; 4],
                })
            })
            .unwrap();
        (atlas, glyph)
    }

    fn prepare_with_atlas(
        scene: &Scene,
        atlas: &GlyphAtlas,
        size: (u32, u32),
        budget: ClipFrameBudget,
    ) -> Result<ClipFrame, RenderError> {
        let limits = wgpu::Limits::default();
        ClipFrame::prepare(scene, atlas, size, 1., budget, &limits)
    }

    fn clipped_glyph_scene(
        glyph: tekes_ui_scene::AtlasGlyph,
        origins: &[(f32, f32)],
        nested_shapes: usize,
        with_quad: bool,
    ) -> (Scene, ShapeTable) {
        let mut table = ShapeTable::new(7, nested_shapes.max(1));
        let mut keys = Vec::new();
        for depth in 0..nested_shapes {
            let clip = ClipShape::normalize(
                Rect::new(
                    depth as f32,
                    depth as f32,
                    16. - depth as f32,
                    16. - depth as f32,
                ),
                CornerRadii::ZERO,
                Point::new(0., 0.),
            )
            .unwrap();
            keys.push(table.intern(clip).unwrap());
        }
        let mut scene = Scene::with_clip_shapes(&table);
        for key in keys {
            scene.push_shape_clip(key);
        }
        if with_quad {
            scene.push_quad(Quad::new(Rect::new(1., 1., 4., 4.), Rgba::gray(1.)));
        }
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: origins
                .iter()
                .map(|&(x, y)| PositionedGlyph {
                    origin: Point::new(x, y),
                    glyph,
                })
                .collect(),
        });
        for _ in 0..nested_shapes {
            scene.pop_clip();
        }
        (scene, table)
    }

    #[test]
    fn glyph_scissor_area_is_checked_and_falls_back_for_unprovable_rectangles() {
        assert_eq!(scissor_pixel_area(Some([2, 3, 4, 5]), (8, 9)), Some(20));
        assert_eq!(scissor_pixel_area(Some([8, 9, 0, 0]), (8, 9)), Some(0));
        assert_eq!(scissor_pixel_area(None, (8, 9)), None);
        assert_eq!(scissor_pixel_area(Some([u32::MAX, 0, 1, 1]), (8, 9)), None);
        assert_eq!(scissor_pixel_area(Some([7, 0, 2, 1]), (8, 9)), None);
        assert_eq!(scissor_pixel_area(Some([0, 8, 1, 2]), (8, 9)), None);
        assert_eq!(scissor_pixel_area(Some([0, 0, u32::MAX, 2]), (8, 9)), None);
    }

    #[test]
    fn run_scissor_expands_clamps_and_falls_back_outside_the_bounded_domain() {
        let (_atlas, glyph) = atlas_with_glyph();
        let run = |origin: (f32, f32)| GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(origin.0, origin.1),
                glyph,
            }],
        };
        let parent = [0, 0, 100, 100];
        assert_eq!(
            glyph_run_scissor(&run((40.25, 50.25)), 1., parent, (100, 100)),
            Some([24, 34, 33, 33])
        );
        assert_eq!(
            glyph_run_scissor(&run((-10., -10.)), 1., parent, (100, 100)),
            Some([0, 0, 7, 7])
        );
        assert_eq!(
            glyph_run_scissor(&run((99., 99.)), 1., parent, (100, 100)),
            Some([83, 83, 17, 17])
        );
        assert_eq!(
            glyph_run_scissor(&run((16_385., 1.)), 1., parent, (100, 100)),
            None
        );
        let mixed_run = GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![
                PositionedGlyph {
                    origin: Point::new(40., 50.),
                    glyph,
                },
                PositionedGlyph {
                    origin: Point::new(16_385., 1.),
                    glyph,
                },
            ],
        };
        assert_eq!(
            glyph_run_scissor(&mixed_run, 1., parent, (100, 100)),
            None,
            "one out-of-domain glyph falls back the whole run to its parent scissor"
        );
        assert_eq!(
            glyph_run_scissor(&run((1., 1.)), 1., parent, (16_385, 100)),
            None
        );
        assert_eq!(
            glyph_run_scissor(
                &GlyphRun {
                    color: Rgba::gray(1.),
                    glyphs: Vec::new(),
                },
                1.,
                parent,
                (100, 100),
            ),
            None
        );
        assert_eq!(
            glyph_run_scissor(&run((f32::MAX, 1.)), 1., parent, (100, 100)),
            None
        );

        let mut zero = glyph;
        zero.width = 0;
        zero.height = 0;
        let zero_run = GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(40., 50.),
                glyph: zero,
            }],
        };
        assert_eq!(
            glyph_run_scissor(&zero_run, 1., parent, (100, 100)),
            Some([24, 34, 32, 32])
        );
        let mut too_wide = glyph;
        too_wide.width = 16_385;
        let too_wide_run = GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(1., 1.),
                glyph: too_wide,
            }],
        };
        assert_eq!(
            glyph_run_scissor(&too_wide_run, 1., parent, (100, 100)),
            None
        );
    }

    #[test]
    fn prepared_run_scissors_restore_for_neighbor_draws_and_clip_stack() {
        let (atlas, glyph) = atlas_with_glyph();
        let mut table = ShapeTable::new(7, 2);
        let parent = table
            .intern(
                ClipShape::normalize(
                    Rect::new(0., 0., 80., 80.),
                    CornerRadii::ZERO,
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let empty = table
            .intern(
                ClipShape::normalize(
                    Rect::new(20., 20., 0., 0.),
                    CornerRadii::ZERO,
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let mut scene = Scene::with_clip_shapes(&table);
        scene.push_shape_clip(parent);
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(40.25, 50.25),
                glyph,
            }],
        });
        scene.push_quad(Quad::new(Rect::new(1., 1., 4., 4.), Rgba::gray(1.)));
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![
                PositionedGlyph {
                    origin: Point::new(40., 50.),
                    glyph,
                },
                PositionedGlyph {
                    origin: Point::new(16_385., 1.),
                    glyph,
                },
            ],
        });
        scene.push_image(ImageQuad {
            rect: Rect::new(2., 2., 3., 3.),
            image: ImageId(0),
            opacity: 1.,
        });
        scene.pop_clip();
        scene.push_shape_clip(empty);
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(40., 50.),
                glyph,
            }],
        });
        scene.pop_clip();
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(40., 50.),
                glyph,
            }],
        });
        scene.push_quad(Quad::new(Rect::new(3., 3., 4., 4.), Rgba::gray(1.)));

        let limits = wgpu::Limits::default();
        let frame = prepare_with_atlas(
            &scene,
            &atlas,
            (100, 100),
            ClipFrameBudget::for_device(&limits),
        )
        .unwrap();
        assert_eq!(frame.states[1].scissor, [24, 34, 33, 33]);
        assert_eq!(frame.states[2].scissor, [0, 0, 81, 81]);
        assert_eq!(frame.states[3].scissor, [0, 0, 81, 81]);
        assert_eq!(frame.states[4].scissor, [0, 0, 81, 81]);
        assert_eq!(frame.states[5].scissor, [0, 0, 100, 100]);
        assert_eq!(frame.states[7].scissor, [0; 4]);
        assert_eq!(frame.states[8].scissor, [0, 0, 100, 100]);
        assert_eq!(frame.states[9].scissor, [0, 0, 100, 100]);
        assert_eq!(frame.states[10].scissor, [0, 0, 100, 100]);
        let expected_work = u64::from(33_u32 * 33) + 2 * u64::from(81_u32 * 81) + 2 * 100 * 100;
        assert_eq!(frame.shader_work, expected_work);

        let before = atlas.stats();
        assert!(matches!(
            prepare_with_atlas(
                &scene,
                &atlas,
                (100, 100),
                ClipFrameBudget {
                    max_shader_work: expected_work - 1,
                    ..ClipFrameBudget::for_device(&limits)
                },
            ),
            Err(RenderError::ClipBudgetExceeded("shader work"))
        ));
        assert_eq!(
            atlas.stats(),
            before,
            "rejected preflight must stay side-effect free"
        );
    }

    #[test]
    fn glyph_work_uses_final_scissor_for_fractional_negative_and_overlapping_instances() {
        let (atlas, glyph) = atlas_with_glyph();
        let origins: Vec<_> = (0..260)
            .map(|i| {
                if i % 2 == 0 {
                    (-0.25, 1.5)
                } else {
                    (3.75, -2.25)
                }
            })
            .collect();
        let (scene, _table) = clipped_glyph_scene(glyph, &origins, 1, false);
        let before = atlas.stats();
        let limits = wgpu::Limits::default();
        let frame = prepare_with_atlas(
            &scene,
            &atlas,
            (32, 32),
            ClipFrameBudget::for_device(&limits),
        )
        .unwrap();
        let state = frame.states[1];
        let pixels = scissor_pixel_area(Some(state.scissor), (32, 32)).unwrap();
        assert_eq!(state.scissor, [0, 0, 17, 17]);
        assert_eq!(frame.shader_work, pixels * 260);
        assert_eq!(atlas.stats(), before, "preflight must not mutate the atlas");
    }

    #[test]
    fn empty_prepared_scissor_charges_zero_glyph_shader_work() {
        let (atlas, glyph) = atlas_with_glyph();
        let mut table = ShapeTable::new(7, 1);
        let empty = ClipShape::normalize(
            Rect::new(2., 2., 0., 0.),
            CornerRadii::ZERO,
            Point::new(0., 0.),
        )
        .unwrap();
        let key = table.intern(empty).unwrap();
        let mut scene = Scene::with_clip_shapes(&table);
        scene.push_shape_clip(key);
        scene.push_glyphs(GlyphRun {
            color: Rgba::gray(1.),
            glyphs: vec![PositionedGlyph {
                origin: Point::new(-0.5, 1.25),
                glyph,
            }],
        });
        scene.pop_clip();
        let limits = wgpu::Limits::default();
        let frame = prepare_with_atlas(
            &scene,
            &atlas,
            (32, 32),
            ClipFrameBudget::for_device(&limits),
        )
        .unwrap();
        assert_eq!(frame.states[1].scissor, [0; 4]);
        assert_eq!(frame.shader_work, 0);
    }

    #[test]
    fn nested_glyph_work_accumulates_per_instance_and_keeps_other_pipeline_full_target() {
        let (atlas, glyph) = atlas_with_glyph();
        let origins = vec![(1., 1.); 3];
        let (scene, _table) = clipped_glyph_scene(glyph, &origins, 3, true);
        let limits = wgpu::Limits::default();
        let base = ClipFrameBudget::for_device(&limits);
        let frame = prepare_with_atlas(&scene, &atlas, (32, 32), base).unwrap();
        let glyph_state = frame.states[4];
        let glyph_pixels = scissor_pixel_area(Some(glyph_state.scissor), (32, 32)).unwrap();
        let glyph_depth = u64::from(frame.records[glyph_state.head as usize - 1].meta[3]);
        let quad_state = frame.states[3];
        let quad_depth = u64::from(frame.records[quad_state.head as usize - 1].meta[3]);
        let expected = glyph_pixels * glyph_depth * 3 + 32 * 32 * quad_depth;
        assert_eq!(frame.shader_work, expected);
        assert!(matches!(
            prepare_with_atlas(
                &scene,
                &atlas,
                (32, 32),
                ClipFrameBudget {
                    max_shader_work: expected - 1,
                    ..base
                },
            ),
            Err(RenderError::ClipBudgetExceeded("shader work"))
        ));
    }
    #[test]
    fn repeat_identity_is_one_record_and_one_coverage_evaluation() {
        let (mut s, _table, key) = scene();
        s.push_shape_clip(key.clone());
        s.push_shape_clip(key.clone());
        s.push_keyed_quad(
            Quad::new(Rect::new(0., 0., 8., 8.), Rgba::gray(1.)).with_corner_radius(2.),
            key,
        );
        s.pop_clip();
        s.pop_clip();
        let frame = prepare(&s, None).unwrap();
        assert_eq!(frame.records.len(), 1);
        assert_eq!(frame.queries, 1);
        assert_eq!(frame.states[0].head, frame.states[1].head);
        assert_eq!(frame.states[2].outer_id, frame.records[0].meta[1]);
    }
    #[test]
    fn malformed_records_reject_indices_cycles_depth_kind_and_identity_reuse() {
        let r = ClipRecord {
            rect: [0., 0., 8., 8.],
            radii: [2.; 4],
            meta: [0, 1, 0, 1],
        };
        assert!(validate_records(&[r], 1, 1).is_ok());
        for meta in [
            [1, 1, 0, 1],
            [5, 1, 0, 1],
            [0, 2, 0, 1],
            [0, 1, 2, 1],
            [0, 1, 0, 2],
        ] {
            assert!(
                validate_records(&[ClipRecord { meta, ..r }], 1, 1).is_err(),
                "{meta:?}"
            );
        }
        let bad = ClipRecord {
            rect: [1., 0., 8., 8.],
            meta: [0, 1, 0, 1],
            ..r
        };
        assert!(validate_records(&[r, bad], 1, 2).is_err());
        assert!(
            validate_records(
                &[
                    r,
                    ClipRecord {
                        meta: [1, 1, 0, 2],
                        ..r
                    }
                ],
                1,
                2
            )
            .is_err()
        );
        assert!(
            validate_records(
                &[ClipRecord {
                    radii: [9.; 4],
                    ..r
                }],
                1,
                1
            )
            .is_err()
        );
        assert!(
            validate_records(
                &[ClipRecord {
                    rect: [f32::NAN, 0., 8., 8.],
                    ..r
                }],
                1,
                1
            )
            .is_err()
        );
    }
    #[test]
    fn frame_sums_queries_instances_shader_work_and_uploads() {
        let mut table = ShapeTable::new(7, 2);
        let a = table.intern(shape(0.)).unwrap();
        let b = table.intern(shape(12.)).unwrap();
        let mut s = Scene::with_clip_shapes(&table);
        for key in [a, b] {
            s.push_shape_clip(key);
            s.push_quad(Quad::new(Rect::new(0., 0., 32., 32.), Rgba::gray(1.)));
            s.pop_clip();
        }
        let full = prepare(&s, None).unwrap();
        assert_eq!(full.queries, 2);
        assert_eq!(full.records.len(), 2);
        let base = ClipFrameBudget::for_device(&wgpu::Limits::default());
        for (name, budget) in [
            (
                "queries",
                ClipFrameBudget {
                    max_queries: 1,
                    ..base
                },
            ),
            (
                "instances",
                ClipFrameBudget {
                    max_instances: 1,
                    ..base
                },
            ),
            (
                "shader",
                ClipFrameBudget {
                    max_shader_work: full.shader_work - 1,
                    ..base
                },
            ),
            (
                "upload",
                ClipFrameBudget {
                    max_upload_bytes: full.upload_bytes - 1,
                    ..base
                },
            ),
            (
                "CPU",
                ClipFrameBudget {
                    max_cpu_work: full.cpu_work - 1,
                    ..base
                },
            ),
            (
                "records",
                ClipFrameBudget {
                    max_records: 1,
                    ..base
                },
            ),
            (
                "inputs",
                ClipFrameBudget {
                    max_primitives: s.len() as u64 - 1,
                    ..base
                },
            ),
        ] {
            assert!(
                prepare(&s, Some(budget)).is_err(),
                "{name} must reject entire frame"
            );
        }
    }
    #[test]
    fn balance_version_owner_geometry_and_indeterminate_errors_propagate() {
        let (mut s, _table, key) = scene();
        s.push_shape_clip(key.clone());
        assert!(prepare(&s, None).is_err());
        s.pop_clip();
        assert!(prepare(&s, None).is_ok());
        s.pop_clip();
        assert!(prepare(&s, None).is_err());
        let (mut s, _, key) = scene();
        s.require_clip_geometry(999);
        assert!(matches!(
            prepare(&s, None),
            Err(RenderError::UnsupportedClipGeometry(999))
        ));
        let (mut foreign, _, _) = scene();
        foreign.push_shape_clip(key);
        foreign.pop_clip();
        assert!(matches!(
            prepare(&foreign, None),
            Err(RenderError::ClipGeometry(ClipError::StaleShapeKey))
        ));
        let (mut mismatch, _, key) = scene();
        mismatch.push_keyed_quad(Quad::new(Rect::new(1., 0., 8., 8.), Rgba::gray(1.)), key);
        assert!(prepare(&mismatch, None).is_err());
        let small = 2_f32.powi(-50);
        let big = 2_f32.powi(50);
        let mut table = ShapeTable::new(1, 2);
        let a = table
            .intern(
                ClipShape::normalize(
                    Rect::new(-small, -small, 2. * small, 2. * small),
                    CornerRadii::uniform(small),
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let b = table
            .intern(
                ClipShape::normalize(
                    Rect::new(0., -big, 2. * big, 2. * big),
                    CornerRadii::uniform(big),
                    Point::new(0., 0.),
                )
                .unwrap(),
            )
            .unwrap();
        let mut s = Scene::with_clip_shapes(&table);
        s.push_shape_clip(a);
        s.push_shape_clip(b);
        s.pop_clip();
        s.pop_clip();
        assert!(matches!(
            prepare(&s, None),
            Err(RenderError::ClipGeometry(
                ClipError::NumericallyIndeterminate
            ))
        ));
    }
    #[test]
    fn conservative_support_does_not_swallow_fractional_rect_ink() {
        let s = ClipShape::normalize(
            Rect::new(280.007_4, 0., 1., 8.),
            CornerRadii::ZERO,
            Point::new(0., 0.),
        )
        .unwrap();
        assert_eq!(legacy_scissor(s.rect(), 1., (400, 10))[2], 0);
        assert!(support_scissor(s, 1., (400, 10))[2] >= 2);
    }
    #[test]
    fn device_storage_uniform_and_target_limits_reject_without_allocating_gpu_resources() {
        let (mut s, _, key) = scene();
        s.push_shape_clip(key);
        s.push_quad(Quad::new(Rect::new(0., 0., 32., 32.), Rgba::gray(1.)));
        s.pop_clip();
        let original = wgpu::Limits::default();
        let budget = ClipFrameBudget::for_device(&original);
        let atlas = GlyphAtlas::default();
        let mut storage = original.clone();
        storage.max_storage_buffer_binding_size = 47;
        assert!(matches!(
            ClipFrame::prepare(&s, &atlas, (32, 32), 1., budget, &storage),
            Err(RenderError::ClipBudgetExceeded("record device capacity"))
        ));
        let mut uniform = original.clone();
        uniform.max_buffer_size = u64::from(uniform.min_uniform_buffer_offset_alignment.max(32));
        assert!(matches!(
            ClipFrame::prepare(&s, &atlas, (32, 32), 1., budget, &uniform),
            Err(RenderError::ClipBudgetExceeded("uniform device capacity"))
        ));
        let mut target = original;
        target.max_texture_dimension_2d = 16;
        assert!(matches!(
            ClipFrame::prepare(&s, &atlas, (32, 32), 1., budget, &target),
            Err(RenderError::InvalidArgument(
                "target exceeds device dimensions"
            ))
        ));
    }
}
