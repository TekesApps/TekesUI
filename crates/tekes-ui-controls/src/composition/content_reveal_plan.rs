//! Original Host read-only translation plans. No mutating component callbacks during planning.
use super::*;
use crate::{
    ReadingDocumentVersion, ScrollRevealError, ScrollRevealOptions, ScrollRevealPlacement,
};
use std::sync::Arc;
use tekes_ui_scene::{ClipError, ClipRegion, ClipShape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRevealError {
    UnlaidOut,
    UnavailableTarget,
    UnsupportedLayout,
    UnsupportedPlacement,
    InvalidOptions(ScrollRevealError),
    Geometry(ClipError),
    NoFeasibleRegion,
    NoConvergence,
    PlanBudgetExceeded,
    StalePlan,
    CompositionBarrier,
    IdentityExhausted,
}
/// Explicit work policy, independent of depth or wall-clock performance acceptance.
#[derive(Clone, Copy, Debug)]
pub struct HostRevealBudget {
    pub max_path_nodes: usize,
    pub max_passes: usize,
    pub max_queries: usize,
    pub max_geometry_work: usize,
}
impl Default for HostRevealBudget {
    fn default() -> Self {
        Self {
            max_path_nodes: 4096,
            max_passes: 4098,
            max_queries: 16384,
            max_geometry_work: 100_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostRevealOutcome {
    /// A validated landing was planned; logical focus plans never claim this.
    pub revealed: bool,
    pub changed: bool,
    pub partial: bool,
    pub limit_clamped: bool,
    pub shape_clamped: bool,
    pub animated: bool,
}
#[derive(Debug)]
pub(super) struct RevealClock {
    owner: Arc<()>,
    pub(super) geometry_lease: Arc<std::sync::atomic::AtomicU64>,
    geometry: Option<u64>,
    input: Option<u64>,
    pub(super) pending: bool,
}
impl Default for RevealClock {
    fn default() -> Self {
        Self {
            owner: Arc::new(()),
            geometry_lease: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            geometry: Some(0),
            input: Some(0),
            pending: false,
        }
    }
}
impl RevealClock {
    pub(super) fn invalidate_input(&mut self) {
        let _ = self.geometry_lease.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| Some(n.saturating_add(1)),
        );
        self.input = self.input.and_then(|n| n.checked_add(1));
    }
    pub(super) fn invalidate_geometry(&mut self) {
        let _ = self.geometry_lease.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| Some(n.saturating_add(1)),
        );
        self.geometry = self.geometry.and_then(|n| n.checked_add(1));
        self.pending = true;
    }
    pub(super) fn stamp(&self) -> Result<(u64, u64), HostRevealError> {
        Ok((
            self.geometry.ok_or(HostRevealError::IdentityExhausted)?,
            self.input.ok_or(HostRevealError::IdentityExhausted)?,
        ))
    }
}
#[derive(Clone, Debug, PartialEq)]
struct ScrollState {
    viewport: Rect,
    viewport_size: Size,
    document: Size,
    offset: Point,
    visual: Point,
    limit: Point,
    reveal: u64,
    version: Option<ReadingDocumentVersion>,
    item_mounts: Option<std::collections::BTreeMap<ComponentId, u64>>,
}
#[derive(Clone, Debug, PartialEq)]
struct NodeState {
    id: ComponentId,
    mount: u64,
    frame: Rect,
    clip: Rect,
    scroll: Option<ScrollState>,
    child_clip: Option<Rect>,
    pane_boundary: Option<Rect>,
    curve_requirement: bool,
    reveal_geometry: Option<HostRevealGeometry>,
    fixed_ancestor_curves: Vec<HostRevealGeometry>,
}
impl NodeState {
    fn read(node: &Content) -> Result<Self, HostRevealError> {
        let child_clip = node.component.host_reveal_local_clip();
        if child_clip.is_some_and(|r| {
            ![
                r.origin.x,
                r.origin.y,
                r.size.width,
                r.size.height,
                r.max_x(),
                r.max_y(),
            ]
            .into_iter()
            .all(f32::is_finite)
                || r.size.width < 0.
                || r.size.height < 0.
        }) {
            return Err(HostRevealError::Geometry(ClipError::InvalidInput));
        }
        Ok(Self {
            id: node.id,
            mount: node.mount,
            frame: node.frame,
            clip: node.clip,
            child_clip,
            pane_boundary: split_view::reveal_pane_boundary(node),
            curve_requirement: node.component.requires_focus_reveal(),
            reveal_geometry: node.component.host_reveal_geometry_constraint(),
            fixed_ancestor_curves: Vec::new(),
            scroll: node.component::<ScrollContent>().map(|s| ScrollState {
                viewport: s.viewport,
                viewport_size: s.scroll.viewport_size(),
                document: s.scroll.document_size(),
                offset: s.scroll.offset,
                visual: s.scroll.visual_offset(),
                limit: s.scroll.limit(),
                reveal: s.scroll.reveal_identity(),
                version: s.reading_document_version().cloned(),
                item_mounts: s.reading.as_ref().map(|r| r.item_mounts.clone()),
            }),
        })
    }
}
fn read_reveal_nodes(
    path: &[&Content],
    fixed_ancestor_curves: Vec<HostRevealGeometry>,
) -> Result<Vec<NodeState>, HostRevealError> {
    let mut nodes = path
        .iter()
        .map(|n| NodeState::read(n))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(first) = nodes.first_mut() {
        first.fixed_ancestor_curves = fixed_ancestor_curves;
    }
    for node in path.iter().take(path.len().saturating_sub(1)) {
        if node.component::<StackContent>().is_none()
            && node.component::<GridContent>().is_none()
            && node.component::<ScrollContent>().is_none()
            && node.component::<ListContent>().is_none()
            && node.component.host_reveal_geometry_constraint().is_none()
            && !node.component.host_reveal_passthrough()
        {
            return Err(HostRevealError::UnsupportedLayout);
        }
    }
    Ok(nodes)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlanPurpose {
    Reveal,
    LogicalFocus,
}
/// A readonly-resolved original tree route. Mutable access follows only the
/// recorded child indices; it never probes siblings with children_mut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ContentRoute {
    identities: Vec<(ComponentId, u64)>,
    indices: Vec<usize>,
    // Only positive qualifications ACTUALLY relied upon by this plan.
    // Empty for ordinary logical paths which never used translation geometry.
    passthrough_depths: Vec<usize>,
}
impl ContentRoute {
    pub(super) fn capture(
        root: &Content,
        id: ComponentId,
        max: usize,
    ) -> Result<Self, HostRevealError> {
        fn visit(
            node: &Content,
            id: ComponentId,
            route: &mut ContentRoute,
            left: &mut usize,
        ) -> Result<bool, HostRevealError> {
            *left = left
                .checked_sub(1)
                .ok_or(HostRevealError::PlanBudgetExceeded)?;
            route.identities.push((node.id, node.mount));
            if node.id == id {
                return Ok(true);
            }
            for (index, child) in node.component.children().iter().enumerate() {
                route.indices.push(index);
                if visit(child, id, route, left)? {
                    return Ok(true);
                }
                route.indices.pop();
            }
            route.identities.pop();
            Ok(false)
        }
        let mut route = Self {
            identities: Vec::new(),
            indices: Vec::new(),
            passthrough_depths: Vec::new(),
        };
        let mut left = max;
        if !visit(root, id, &mut route, &mut left)? {
            return Err(HostRevealError::UnavailableTarget);
        }
        Ok(route)
    }
    fn fixed_ancestor_curves(
        &self,
        root: &Content,
        scope_depth: usize,
    ) -> Result<Vec<HostRevealGeometry>, HostRevealError> {
        let mut curves = Vec::new();
        for depth in 0..scope_depth {
            let node = self.node(root, depth).ok_or(HostRevealError::StalePlan)?;
            match node.component.host_reveal_geometry_constraint() {
                Some(curve) if !curves.contains(&curve) => {
                    curves.push(curve);
                }
                None if node.component.requires_focus_reveal() => {
                    return Err(HostRevealError::UnsupportedLayout);
                }
                _ => {}
            }
        }
        Ok(curves)
    }
    fn require_passthrough(&mut self, root: &Content, depth: usize) -> Result<(), HostRevealError> {
        if self
            .node(root, depth)
            .is_none_or(|n| !n.component.host_reveal_passthrough())
        {
            return Err(HostRevealError::UnsupportedLayout);
        }
        if !self.passthrough_depths.contains(&depth) {
            self.passthrough_depths.push(depth);
        }
        Ok(())
    }
    fn passthrough_current(&self, root: &Content) -> bool {
        self.passthrough_depths.iter().all(|depth| {
            self.node(root, *depth)
                .is_some_and(|n| n.component.host_reveal_passthrough())
        })
    }
    fn library_scroll_parent_path(&mut self, root: &Content, depth: usize) -> bool {
        let mut node = root;
        for i in 0..depth {
            if (node.id, node.mount) != self.identities[i] {
                return false;
            }
            // Only actual planned ancestors need this original pure Vec access.
            // Unknown sibling subtrees are neither inspected mutably nor rejected.
            if node.component::<StackContent>().is_none()
                && node.component::<GridContent>().is_none()
                && node.component::<ListContent>().is_none()
                && node.component::<ScrollContent>().is_none()
                && node.component::<DialogContent>().is_none()
                && node.component.host_reveal_geometry_constraint().is_none()
            {
                if !node.component.host_reveal_passthrough() {
                    return false;
                }
                if !self.passthrough_depths.contains(&i) {
                    self.passthrough_depths.push(i);
                }
            }
            let Some(child) = node.component.children().get(self.indices[i]) else {
                return false;
            };
            node = child;
        }
        true
    }
    pub(super) fn depth(&self) -> usize {
        self.indices.len()
    }
    pub(super) fn node<'a>(&self, root: &'a Content, depth: usize) -> Option<&'a Content> {
        if depth >= self.identities.len() {
            return None;
        }
        let mut node = root;
        for index in 0..=depth {
            if (node.id, node.mount) != self.identities[index] {
                return None;
            }
            if index < depth {
                node = node.component.children().get(self.indices[index])?;
            }
        }
        Some(node)
    }
    pub(super) fn node_mut<'a>(
        &self,
        root: &'a mut Content,
        depth: usize,
    ) -> Option<&'a mut Content> {
        fn walk<'a>(
            node: &'a mut Content,
            ids: &[(ComponentId, u64)],
            indices: &[usize],
            depth: usize,
        ) -> Option<&'a mut Content> {
            if (node.id, node.mount) != *ids.first()? {
                return None;
            }
            if depth == 0 {
                return Some(node);
            }
            let child = node.component.children_mut()?.get_mut(*indices.first()?)?;
            walk(child, &ids[1..], &indices[1..], depth - 1)
        }
        if depth >= self.identities.len() {
            return None;
        }
        walk(root, &self.identities, &self.indices, depth)
    }
}
/// Opaque, single-use plan tied to this Host allocation, mounts, epochs and exact
/// scroll/reading/layout inputs. Clone cannot mint a second commit authority.
#[derive(Debug)]
pub struct HostRevealPlan {
    owner: Arc<()>,
    stamp: (u64, u64),
    focus: Option<ComponentId>,
    focus_epoch: u64,
    nodes: Vec<NodeState>,
    route: ContentRoute,
    scope_depth: usize,
    target_rect: Rect,
    required_witness: Option<Rect>,
    required_evidence: Option<RequiredFocusEvidence>,
    target_caret: Option<Rect>,
    targets: Vec<Point>,
    options: ScrollRevealOptions,
    outcome: HostRevealOutcome,
    explicit: bool,
    path_limit: usize,
    purpose: PlanPurpose,
}
/// Original reveal owner's fully reserved writes. No validation or allocation at commit.
#[derive(Debug)]
pub(super) struct PreparedHostReveal {
    route: ContentRoute,
    writes: Vec<(usize, crate::scroll_view::reveal::PreparedReveal)>,
    outcome: HostRevealOutcome,
    next_input: u64,
}
fn collect_path<'a>(
    node: &'a Content,
    item: ComponentId,
    path: &mut Vec<&'a Content>,
    max: usize,
) -> Result<bool, HostRevealError> {
    fn visit<'a>(
        node: &'a Content,
        item: ComponentId,
        path: &mut Vec<&'a Content>,
        remaining: &mut usize,
    ) -> Result<bool, HostRevealError> {
        if !node.active() {
            return Ok(false);
        }
        *remaining = remaining
            .checked_sub(1)
            .ok_or(HostRevealError::PlanBudgetExceeded)?;
        path.push(node);
        if node.id == item {
            return Ok(true);
        }
        for child in node.component.children() {
            if visit(child, item, path, remaining)? {
                return Ok(true);
            }
        }
        path.pop();
        Ok(false)
    }
    let mut remaining = max;
    visit(node, item, path, &mut remaining)
}
fn record_numeric_state(
    history: &mut Vec<Vec<Point>>,
    targets: &[Point],
) -> Result<(), HostRevealError> {
    if history.iter().any(|seen| seen.as_slice() == targets) {
        return Err(HostRevealError::NoConvergence);
    }
    history.push(targets.to_vec());
    Ok(())
}
fn intersection(a: Rect, b: Rect) -> Rect {
    a.intersection(&b).unwrap_or_default()
}
fn translated_witness(nodes: &[NodeState], targets: &[Point], witness: Rect) -> Option<Rect> {
    let (frames, _, _) = projected(nodes, targets);
    let original = nodes.last()?.frame;
    let projected = *frames.last()?;
    Some(witness.offset(
        projected.min_x() - original.min_x(),
        projected.min_y() - original.min_y(),
    ))
}
fn contains_rect(outer: Rect, inner: Rect) -> bool {
    [
        outer.min_x(),
        outer.min_y(),
        outer.max_x(),
        outer.max_y(),
        inner.min_x(),
        inner.min_y(),
        inner.max_x(),
        inner.max_y(),
    ]
    .into_iter()
    .all(f32::is_finite)
        && inner.size.width > 0.0
        && inner.size.height > 0.0
        && inner.min_x() >= outer.min_x()
        && inner.min_y() >= outer.min_y()
        && inner.max_x() <= outer.max_x()
        && inner.max_y() <= outer.max_y()
}
fn projected(nodes: &[NodeState], targets: &[Point]) -> (Vec<Rect>, Vec<Rect>, Vec<Rect>) {
    let mut frames = Vec::with_capacity(nodes.len());
    let mut clips = Vec::with_capacity(nodes.len());
    let mut viewports = Vec::with_capacity(nodes.len());
    let mut delta = Point::default();
    let mut clip = nodes[0].clip;
    for (i, n) in nodes.iter().enumerate() {
        // SplitView's parent already clips the original Slot owner to its pane;
        // ordinary custom local clips below still apply only to descendants.
        if let Some(boundary) = n.pane_boundary {
            clip = intersection(clip, boundary.offset(delta.x, delta.y));
        }
        frames.push(n.frame.offset(delta.x, delta.y));
        clips.push(clip);
        // The owner receives the ancestor clip. Its local boundary constrains
        // descendants, in the owner's translated coordinates before its Scroll.
        if let Some(boundary) = n.child_clip {
            clip = intersection(clip, boundary.offset(delta.x, delta.y));
        }
        if let Some(s) = &n.scroll {
            let viewport = s.viewport.offset(delta.x, delta.y);
            viewports.push(viewport);
            clip = intersection(clip, viewport);
            delta.x += s.visual.x - targets[i].x;
            delta.y += s.visual.y - targets[i].y;
        } else {
            viewports.push(Rect::default());
        }
    }
    (frames, clips, viewports)
}
fn safe_f32(v: f64) -> Result<f32, HostRevealError> {
    let f = v as f32;
    if !v.is_finite() || !f.is_finite() {
        return Err(HostRevealError::Geometry(
            ClipError::UnsupportedNumericRange,
        ));
    }
    Ok(f)
}
/// Return an offset plus reasons. Endpoints use inward interval bounds; every
/// rounded f32 landing is checked again, uncertainty is never widened to success.
#[allow(clippy::too_many_arguments)] // Explicit immutable original geometry and navigation inputs.
fn constrained_axis(
    region: &ClipRegion,
    axis: usize,
    rect: Rect,
    viewport: Rect,
    origin: Point,
    offset: Point,
    limit: Point,
    options: ScrollRevealOptions,
) -> Result<(Point, HostRevealOutcome), HostRevealError> {
    let m = options.margins;
    let (cross, start, end, vstart, vend, before, after, old, max, base) = if axis == 1 {
        (
            [
                f64::from(rect.min_x()) - f64::from(m.left),
                f64::from(rect.max_x()) + f64::from(m.right),
            ],
            f64::from(rect.min_y()),
            f64::from(rect.max_y()),
            f64::from(viewport.min_y()),
            f64::from(viewport.max_y()),
            f64::from(m.top),
            f64::from(m.bottom),
            f64::from(offset.y),
            f64::from(limit.y),
            f64::from(origin.y),
        )
    } else {
        (
            [
                f64::from(rect.min_y()) - f64::from(m.top),
                f64::from(rect.max_y()) + f64::from(m.bottom),
            ],
            f64::from(rect.min_x()),
            f64::from(rect.max_x()),
            f64::from(viewport.min_x()),
            f64::from(viewport.max_x()),
            f64::from(m.left),
            f64::from(m.right),
            f64::from(offset.x),
            f64::from(limit.x),
            f64::from(origin.x),
        )
    };
    let section = region
        .section_for_span(axis, cross)
        .map_err(HostRevealError::Geometry)?
        .ok_or(HostRevealError::NoFeasibleRegion)?;
    let low = section[0].upper.max(vstart) + before;
    let high = section[1].lower.min(vend) - after;
    if low >= high {
        return Err(HostRevealError::NoFeasibleRegion);
    }
    // start/end currently include -old. Document coordinates remove that once.
    let document_start = start - base;
    let document_end = end - base;
    let leading = document_start - (low - (base + old));
    let trailing = document_end - (high - (base + old));
    let partial = end - start > high - low;
    let wanted = match options.placement {
        ScrollRevealPlacement::Start(_) => leading,
        ScrollRevealPlacement::Nearest if !partial => {
            old.clamp(trailing.min(leading), trailing.max(leading))
        }
        ScrollRevealPlacement::Nearest
            if old >= leading.min(trailing) && old <= leading.max(trailing) =>
        {
            old
        }
        ScrollRevealPlacement::Nearest => {
            if (leading - old).abs() < (trailing - old).abs() {
                leading
            } else {
                trailing
            }
        }
        ScrollRevealPlacement::Centered => return Err(HostRevealError::UnsupportedPlacement),
    };
    let applied = wanted.clamp(0., max);
    let value = safe_f32(applied)?;
    let landing_start = start + old - f64::from(value);
    let landing_end = end + old - f64::from(value);
    // A limit clamp or an oversized target may be partial, but must leave a
    // positive body interval; an arbitrary corner sliver is never full success.
    if landing_end <= low || landing_start >= high {
        return Err(HostRevealError::NoFeasibleRegion);
    }
    let limit_clamped = applied != wanted;
    if !partial && !limit_clamped && (landing_start < low || landing_end > high) {
        return Err(HostRevealError::Geometry(
            ClipError::NumericallyIndeterminate,
        ));
    }
    let mut target = offset;
    if axis == 1 {
        target.y = value;
    } else {
        target.x = value;
    }
    Ok((
        target,
        HostRevealOutcome {
            partial: partial || landing_start < low || landing_end > high,
            limit_clamped,
            shape_clamped: low > vstart + before || high < vend - after,
            ..Default::default()
        },
    ))
}
impl ContentHost {
    fn validate_curve_path(&self, path: &[&Content]) -> Result<(), HostRevealError> {
        if !self.active_curve_geometry_is_current() {
            return Err(HostRevealError::UnlaidOut);
        }
        let mut in_curve = false;
        for node in path {
            in_curve |= node.component::<PanelContent>().is_some();
            if in_curve && node.curve_geometry.is_none() {
                return Err(HostRevealError::UnlaidOut);
            }
            if in_curve
                && node.component.curve_subtree_capability() == CurveSubtreeCapability::Unsupported
            {
                return Err(HostRevealError::UnsupportedLayout);
            }
        }
        Ok(())
    }
    fn capture_reveal_route(
        &self,
        path: &[&Content],
        budget: usize,
    ) -> Result<(ContentRoute, usize), HostRevealError> {
        let target = path.last().ok_or(HostRevealError::UnavailableTarget)?;
        let mut route = ContentRoute::capture(&self.root, target.id, budget)?;
        let base = route
            .identities
            .len()
            .checked_sub(path.len())
            .ok_or(HostRevealError::StalePlan)?;
        if path.iter().enumerate().any(|(i, node)| {
            route
                .node(&self.root, base + i)
                .is_none_or(|current| current.id != node.id || current.mount != node.mount)
        }) {
            return Err(HostRevealError::StalePlan);
        }
        for (i, node) in path.iter().enumerate() {
            if node.component::<ScrollContent>().is_some()
                && !route.library_scroll_parent_path(&self.root, base + i)
            {
                return Err(HostRevealError::UnsupportedLayout);
            }
        }
        Ok((route, base))
    }
    /// Compute an immutable plan using the current original arrangement. Additional
    /// regions are fixed Host-coordinate constraints, intersected with original
    /// viewport clips. Moving/reflowing custom clip owners require a pure geometry
    /// query and are unsupported here. This does not install Panel geometry.
    pub fn plan_host_reveal(
        &self,
        item: ComponentId,
        options: ScrollRevealOptions,
        constraints: &[(ComponentId, ClipRegion)],
        budget: HostRevealBudget,
    ) -> Result<HostRevealPlan, HostRevealError> {
        let target = self
            .scope()
            .find_active(item)
            .ok_or(HostRevealError::UnavailableTarget)?;
        self.plan_host_reveal_rect(item, target.frame, options, constraints, budget)
    }
    pub(super) fn plan_host_reveal_rect(
        &self,
        item: ComponentId,
        rect: Rect,
        options: ScrollRevealOptions,
        constraints: &[(ComponentId, ClipRegion)],
        budget: HostRevealBudget,
    ) -> Result<HostRevealPlan, HostRevealError> {
        self.plan_host_reveal_rect_in_scope(self.scope(), item, rect, options, constraints, budget)
    }
    fn plan_host_reveal_rect_in_scope(
        &self,
        scope: &Content,
        item: ComponentId,
        rect: Rect,
        options: ScrollRevealOptions,
        constraints: &[(ComponentId, ClipRegion)],
        budget: HostRevealBudget,
    ) -> Result<HostRevealPlan, HostRevealError> {
        let m = options.margins;
        if [
            rect.min_x(),
            rect.min_y(),
            rect.max_x(),
            rect.max_y(),
            rect.size.width,
            rect.size.height,
        ]
        .into_iter()
        .any(|v| !v.is_finite())
            || rect.size.width <= 0.
            || rect.size.height <= 0.
        {
            return Err(HostRevealError::InvalidOptions(
                ScrollRevealError::InvalidRect,
            ));
        }
        if [m.top, m.right, m.bottom, m.left]
            .into_iter()
            .any(|v| !v.is_finite() || v < 0.)
        {
            return Err(HostRevealError::InvalidOptions(
                ScrollRevealError::InvalidMargins,
            ));
        }
        if !self.window_active || self.reading_transaction {
            return Err(HostRevealError::UnavailableTarget);
        }
        if self.last_layout.is_none() || self.reveal_clock.pending {
            return Err(HostRevealError::UnlaidOut);
        }
        let stamp = self.reveal_clock.stamp()?;
        let mut path = Vec::new();
        if !collect_path(scope, item, &mut path, budget.max_path_nodes)? {
            return Err(HostRevealError::UnavailableTarget);
        }
        self.validate_curve_path(&path)?;
        let target_node = path.last().expect("collected reveal target");
        let (required_witness, required_evidence) = match target_node
            .component
            .required_focus_space()
        {
            RequiredFocusSpace::Sufficient { witness, evidence } => (Some(witness), Some(evidence)),
            RequiredFocusSpace::Insufficient | RequiredFocusSpace::Unmeasured => {
                return Err(HostRevealError::NoFeasibleRegion);
            }
            RequiredFocusSpace::Legacy if target_node.component.requires_focus_reveal() => {
                return Err(HostRevealError::UnsupportedLayout);
            }
            RequiredFocusSpace::Legacy | RequiredFocusSpace::NotRequired => (None, None),
        };
        if target_node.component.requires_focus_reveal() && required_witness.is_none() {
            return Err(HostRevealError::UnsupportedLayout);
        }
        let rect = required_witness.unwrap_or(rect);
        if let Some(witness) = required_witness {
            let edges = [
                witness.min_x(),
                witness.min_y(),
                witness.max_x(),
                witness.max_y(),
                witness.size.width,
                witness.size.height,
            ];
            if edges.into_iter().any(|v| !v.is_finite())
                || witness.size.width <= 0.0
                || witness.size.height <= 0.0
                || !contains_rect(target_node.frame, witness)
            {
                return Err(HostRevealError::NoFeasibleRegion);
            }
        }
        let (mut route, scope_depth) = self.capture_reveal_route(&path, budget.max_path_nodes)?;
        let nodes =
            read_reveal_nodes(&path, route.fixed_ancestor_curves(&self.root, scope_depth)?)?;
        validate_curve_constraints(&nodes, path.len().saturating_sub(1), stamp.0)?;
        for (i, n) in path.iter().take(path.len().saturating_sub(1)).enumerate() {
            // Original row authority was already resolved by its exact owner.
            // Only generic geometry paths use the separate positive opt-in.
            if n.component::<StackContent>().is_none()
                && n.component::<GridContent>().is_none()
                && n.component::<ScrollContent>().is_none()
                && n.component::<ListContent>().is_none()
                && nodes[i].reveal_geometry.is_none()
            {
                route.require_passthrough(&self.root, scope_depth + i)?;
            }
        }
        if nodes
            .iter()
            .any(|n| n.child_clip.is_some_and(|r| r.is_empty()))
        {
            return Err(HostRevealError::NoFeasibleRegion);
        }
        let mut targets: Vec<_> = nodes
            .iter()
            .map(|n| n.scroll.as_ref().map_or(Point::default(), |s| s.offset))
            .collect();
        // Original frames include the current visual transform (offset plus
        // overscroll); accepted explicit commits cancel that overscroll. Keep
        // baseline reconstruction separate from the final-offset projection.
        let visual_targets: Vec<_> = nodes
            .iter()
            .map(|n| n.scroll.as_ref().map_or(Point::default(), |s| s.visual))
            .collect();

        let expanded = Rect::new(
            rect.min_x() - m.left,
            rect.min_y() - m.top,
            rect.size.width + m.left + m.right,
            rect.size.height + m.top + m.bottom,
        );
        if constraints.is_empty()
            && targets == visual_targets
            && !requires_complete_curve_landing(&nodes)
            && options.placement == ScrollRevealPlacement::Nearest
            && expanded.intersection(&path.last().unwrap().clip) == Some(expanded)
        {
            let (frames, _, _) = projected(&nodes, &targets);
            let shapes = translated_curve_shapes(&nodes, &frames, path.len().saturating_sub(1))?;
            validate_rect_in_curve_shapes(expanded, &shapes)?;
            // No prediction or custom reveal is needed for a fully visible
            // target. Preserve opaque containers' current original geometry.
            return Ok(HostRevealPlan {
                owner: self.reveal_clock.owner.clone(),
                stamp,
                focus: self.focused,
                focus_epoch: self.focus_epoch,
                target_caret: path
                    .last()
                    .unwrap()
                    .component
                    .text_surface()
                    .map(|s| s.caret()),
                nodes,
                route,
                scope_depth,
                target_rect: rect,
                required_witness,
                required_evidence,
                targets,
                options,
                outcome: HostRevealOutcome {
                    revealed: true,
                    ..Default::default()
                },
                explicit: true,
                path_limit: budget.max_path_nodes,
                purpose: PlanPurpose::Reveal,
            });
        }
        let mut work = 0usize;
        for (id, region) in constraints {
            if nodes.iter().all(|n| n.id != *id || n.scroll.is_none())
                || constraints.iter().filter(|(other, _)| other == id).count() != 1
            {
                return Err(HostRevealError::UnsupportedLayout);
            }
            work = work
                .checked_add(region.reserved_work())
                .ok_or(HostRevealError::PlanBudgetExceeded)?;
        }
        let mut queries = 0usize;
        let mut history = Vec::new();
        let mut outcome = HostRevealOutcome::default();
        let baseline = projected(&nodes, &visual_targets);
        // Only the original passthrough clips and ScrollContent viewport clipping
        // can be predicted without an application layout callback.
        if nodes.iter().zip(&baseline.1).any(|(n, c)| n.clip != *c) {
            return Err(HostRevealError::UnsupportedLayout);
        }
        let mut settled = false;
        for pass in 0..budget.max_passes {
            work = work
                .checked_add(nodes.len())
                .ok_or(HostRevealError::PlanBudgetExceeded)?;
            if work > budget.max_geometry_work {
                return Err(HostRevealError::PlanBudgetExceeded);
            }
            record_numeric_state(&mut history, &targets)?;
            let previous = targets.clone();
            outcome = HostRevealOutcome::default();
            let mut provisional_start_margin = false;
            for i in (0..nodes.len().saturating_sub(1)).rev() {
                let Some(state) = &nodes[i].scroll else {
                    continue;
                };
                work = nodes
                    .len()
                    .checked_mul(3)
                    .and_then(|n| work.checked_add(n))
                    .ok_or(HostRevealError::PlanBudgetExceeded)?;
                queries = queries
                    .checked_add(1)
                    .ok_or(HostRevealError::PlanBudgetExceeded)?;
                if queries > budget.max_queries || work > budget.max_geometry_work {
                    return Err(HostRevealError::PlanBudgetExceeded);
                }
                let (frames, clips, viewports) = projected(&nodes, &targets);
                let mut target = rect.offset(
                    frames.last().unwrap().min_x() - nodes.last().unwrap().frame.min_x(),
                    frames.last().unwrap().min_y() - nodes.last().unwrap().frame.min_y(),
                );
                for view in viewports
                    .iter()
                    .enumerate()
                    .skip(i + 1)
                    .filter(|(j, _)| nodes[*j].scroll.is_some())
                    .map(|(_, v)| v)
                {
                    target = target
                        .intersection(view)
                        .ok_or(HostRevealError::NoFeasibleRegion)?;
                }
                let viewport = viewports[i];
                let visible = viewport.intersection(&clips[i]);
                let visible = match visible {
                    Some(v) => v,
                    None if pass == 0 => viewport,
                    None => return Err(HostRevealError::NoFeasibleRegion),
                };
                let origin = Point::new(
                    viewport.min_x() - targets[i].x,
                    viewport.min_y() - targets[i].y,
                );
                let local = target.offset(-origin.x, -origin.y);
                let scroll = path[i].component::<ScrollContent>().unwrap();
                let mut step_options = options;
                let m = &mut step_options.margins;
                m.top += (visible.min_y() - viewport.min_y()).max(0.);
                m.bottom += (viewport.max_y() - visible.max_y()).max(0.);
                m.left += (visible.min_x() - viewport.min_x()).max(0.);
                m.right += (viewport.max_x() - visible.max_x()).max(0.);
                // Original pure rectangle calculation, with the virtual offset.
                let mut numeric = scroll.scroll.clone();
                numeric.offset = targets[i];
                numeric.overscroll = Point::default();
                let (mut wanted, clamped) = match numeric.reveal_target(local, step_options) {
                    Err(ScrollRevealError::InvalidMargins)
                        if matches!(options.placement, ScrollRevealPlacement::Start(_)) =>
                    {
                        // A parent can enlarge a currently tiny visible slice.
                        // Validate the original margins against the full real
                        // viewport and use only a provisional numeric target.
                        // A settled pass must satisfy the actual visible slice.
                        let result = numeric
                            .reveal_target(local, options)
                            .map_err(HostRevealError::InvalidOptions)?;
                        step_options = options;
                        provisional_start_margin = true;
                        result
                    }
                    result => result.map_err(HostRevealError::InvalidOptions)?,
                };
                outcome.limit_clamped |= clamped;
                outcome.partial |= local.size.width
                    + step_options.margins.left
                    + step_options.margins.right
                    > state.viewport_size.width
                    || local.size.height + step_options.margins.top + step_options.margins.bottom
                        > state.viewport_size.height;
                let projected_region = projected_curve_region(&nodes, &frames, i, budget, stamp.0)?;
                let external = constraints
                    .iter()
                    .find(|(id, _)| *id == nodes[i].id)
                    .map(|(_, r)| r);
                if projected_region.is_some() && external.is_some() {
                    return Err(HostRevealError::UnsupportedLayout);
                }
                if let Some(region) = projected_region.as_ref().or(external) {
                    work = work
                        .checked_add(region.reserved_work())
                        .ok_or(HostRevealError::PlanBudgetExceeded)?;
                    if work > budget.max_geometry_work {
                        return Err(HostRevealError::PlanBudgetExceeded);
                    }
                    let axis = match options.placement {
                        ScrollRevealPlacement::Start(Axis::Vertical) => 1,
                        ScrollRevealPlacement::Start(Axis::Horizontal) => 0,
                        ScrollRevealPlacement::Nearest if state.limit.x == 0. => 1,
                        _ => return Err(HostRevealError::UnsupportedPlacement),
                    };
                    if (axis == 1 && wanted.x != targets[i].x)
                        || (axis == 0 && wanted.y != targets[i].y)
                    {
                        return Err(HostRevealError::UnsupportedPlacement);
                    }
                    let (p, reasons) = constrained_axis(
                        region,
                        axis,
                        target,
                        visible,
                        origin,
                        targets[i],
                        state.limit,
                        options,
                    )?;
                    wanted = p;
                    outcome.partial |= reasons.partial;
                    outcome.limit_clamped |= reasons.limit_clamped;
                    outcome.shape_clamped |= reasons.shape_clamped;
                }
                targets[i] = wanted;
            }
            if targets == previous {
                if provisional_start_margin {
                    return Err(HostRevealError::NoFeasibleRegion);
                }
                settled = true;
                break;
            }
        }
        if !settled {
            return Err(HostRevealError::PlanBudgetExceeded);
        }
        let (frames, clips, _) = projected(&nodes, &targets);
        let delta = Point::new(
            frames.last().unwrap().min_x() - nodes.last().unwrap().frame.min_x(),
            frames.last().unwrap().min_y() - nodes.last().unwrap().frame.min_y(),
        );
        let landing = rect.offset(delta.x, delta.y);
        let visible = landing
            .intersection(clips.last().unwrap())
            .ok_or(HostRevealError::NoFeasibleRegion)?;
        outcome.partial |= visible != landing;
        // Recheck the rounded f32 projected landing against the exact region.
        // The numeric offset solver's f64 result alone cannot authorize a
        // landing whose conversion/translation moved across a curved boundary.
        for (i, node) in nodes.iter().enumerate() {
            if node.scroll.is_none() {
                continue;
            }
            let projected_region = projected_curve_region(&nodes, &frames, i, budget, stamp.0)?;
            let external = constraints
                .iter()
                .find(|(id, _)| *id == node.id)
                .map(|(_, r)| r);
            if projected_region.is_some() && external.is_some() {
                return Err(HostRevealError::UnsupportedLayout);
            }
            let Some(region) = projected_region.as_ref().or(external) else {
                continue;
            };
            work = work
                .checked_add(region.reserved_work())
                .ok_or(HostRevealError::PlanBudgetExceeded)?;
            queries = queries
                .checked_add(1)
                .ok_or(HostRevealError::PlanBudgetExceeded)?;
            if work > budget.max_geometry_work || queries > budget.max_queries {
                return Err(HostRevealError::PlanBudgetExceeded);
            }
            let (_, final_clips, final_views) = projected(&nodes, &targets);
            let mut body = landing;
            for (j, view) in final_views.iter().enumerate().skip(i + 1) {
                if nodes[j].scroll.is_some() {
                    body = body
                        .intersection(view)
                        .ok_or(HostRevealError::NoFeasibleRegion)?;
                }
            }
            let axis = match options.placement {
                ScrollRevealPlacement::Start(Axis::Horizontal) => 0,
                _ => 1,
            };
            let (cross, a, b, vmn, vmx, before, after) = if axis == 1 {
                (
                    [
                        f64::from(body.min_x()) - f64::from(m.left),
                        f64::from(body.max_x()) + f64::from(m.right),
                    ],
                    f64::from(body.min_y()),
                    f64::from(body.max_y()),
                    f64::from(final_views[i].min_y().max(final_clips[i].min_y())),
                    f64::from(final_views[i].max_y().min(final_clips[i].max_y())),
                    f64::from(m.top),
                    f64::from(m.bottom),
                )
            } else {
                (
                    [
                        f64::from(body.min_y()) - f64::from(m.top),
                        f64::from(body.max_y()) + f64::from(m.bottom),
                    ],
                    f64::from(body.min_x()),
                    f64::from(body.max_x()),
                    f64::from(final_views[i].min_x().max(final_clips[i].min_x())),
                    f64::from(final_views[i].max_x().min(final_clips[i].max_x())),
                    f64::from(m.left),
                    f64::from(m.right),
                )
            };
            let section = region
                .section_for_span(axis, cross)
                .map_err(HostRevealError::Geometry)?
                .ok_or(HostRevealError::NoFeasibleRegion)?;
            let low = section[0].upper.max(vmn) + before;
            let high = section[1].lower.min(vmx) - after;
            if !outcome.partial && !outcome.limit_clamped && (a < low || b > high) {
                return Err(HostRevealError::Geometry(
                    ClipError::NumericallyIndeterminate,
                ));
            }
            let lo = a.max(low);
            let hi = b.min(high);
            if lo >= hi {
                return Err(HostRevealError::NoFeasibleRegion);
            }
            for x in cross {
                for y in [lo, hi] {
                    let point = if axis == 1 { [x, y] } else { [y, x] };
                    if !region.contains(point).map_err(HostRevealError::Geometry)? {
                        return Err(HostRevealError::Geometry(
                            ClipError::NumericallyIndeterminate,
                        ));
                    }
                }
            }
        }
        let (final_frames, final_clips, _) = projected(&nodes, &targets);
        if requires_complete_curve_landing(&nodes) {
            let landing = expanded_rect(required_witness.unwrap_or(rect), options.margins);
            let projected_landing = translated_witness(&nodes, &targets, landing)
                .ok_or(HostRevealError::NoFeasibleRegion)?;
            let curve_shapes =
                translated_curve_shapes(&nodes, &final_frames, path.len().saturating_sub(1))?;
            if !final_clips
                .last()
                .is_some_and(|clip| contains_rect(*clip, projected_landing))
            {
                return Err(HostRevealError::NoFeasibleRegion);
            }
            validate_rect_in_curve_shapes(projected_landing, &curve_shapes)?;
        } else if let Some(witness) = required_witness {
            let projected_witness = translated_witness(&nodes, &targets, witness)
                .ok_or(HostRevealError::NoFeasibleRegion)?;
            if !final_clips
                .last()
                .is_some_and(|clip| contains_rect(*clip, projected_witness))
            {
                return Err(HostRevealError::NoFeasibleRegion);
            }
        }
        outcome.revealed = true;
        Ok(HostRevealPlan {
            owner: self.reveal_clock.owner.clone(),
            stamp,
            focus: self.focused,
            focus_epoch: self.focus_epoch,
            target_caret: path
                .last()
                .unwrap()
                .component
                .text_surface()
                .map(|s| s.caret()),
            nodes,
            route,
            scope_depth,
            target_rect: rect,
            required_witness,
            required_evidence,
            targets,
            options,
            outcome,
            explicit: true,
            path_limit: budget.max_path_nodes,
            purpose: PlanPurpose::Reveal,
        })
    }
    pub(super) fn validate_host_reveal(
        &self,
        plan: &HostRevealPlan,
    ) -> Result<(), HostRevealError> {
        self.validate_host_reveal_in_scope(plan, self.scope())
    }
    pub(super) fn validate_host_reveal_in_focus_scope(
        &self,
        plan: &HostRevealPlan,
        scope_id: ComponentId,
        scope_mount: u64,
    ) -> Result<(), HostRevealError> {
        let scope = self
            .root
            .find_active(scope_id)
            .filter(|node| node.mount == scope_mount)
            .ok_or(HostRevealError::StalePlan)?;
        self.validate_host_reveal_in_scope(plan, scope)
    }
    pub(super) fn validate_host_reveal_in_scope(
        &self,
        plan: &HostRevealPlan,
        scope: &Content,
    ) -> Result<(), HostRevealError> {
        if !Arc::ptr_eq(&plan.owner, &self.reveal_clock.owner)
            || self.reveal_clock.stamp()? != plan.stamp
            || (plan.purpose == PlanPurpose::Reveal && self.reveal_clock.pending)
            || !self.window_active
            || self.reading_transaction
            || self.focused != plan.focus
            || self.focus_epoch != plan.focus_epoch
        {
            return Err(HostRevealError::StalePlan);
        }
        if plan.route.node(&self.root, plan.route.depth()).is_none()
            || !plan.route.passthrough_current(&self.root)
        {
            return Err(HostRevealError::StalePlan);
        }
        let mut path = Vec::new();
        if !collect_path(
            scope,
            plan.nodes.last().unwrap().id,
            &mut path,
            plan.path_limit,
        )
        .map_err(|_| HostRevealError::StalePlan)?
            || path.len() != plan.nodes.len()
            || (if plan.purpose == PlanPurpose::Reveal {
                read_reveal_nodes(
                    &path,
                    plan.route
                        .fixed_ancestor_curves(&self.root, plan.scope_depth)
                        .map_err(|_| HostRevealError::StalePlan)?,
                )
                .map_err(|_| HostRevealError::StalePlan)?
            } else {
                path.iter()
                    .map(|n| NodeState::read(n))
                    .collect::<Result<_, _>>()
                    .map_err(|_| HostRevealError::StalePlan)?
            }) != plan.nodes
            || path
                .last()
                .unwrap()
                .component
                .text_surface()
                .map(|s| s.caret())
                != plan.target_caret
        {
            return Err(HostRevealError::StalePlan);
        }
        self.validate_curve_path(&path)
            .map_err(|_| HostRevealError::StalePlan)?;
        if let Some(witness) = plan.required_witness {
            let current = path.last().map(|n| n.component.required_focus_space());
            if current
                != Some(RequiredFocusSpace::Sufficient {
                    witness,
                    evidence: plan
                        .required_evidence
                        .clone()
                        .ok_or(HostRevealError::StalePlan)?,
                })
            {
                return Err(HostRevealError::StalePlan);
            }
        }
        validate_curve_constraints(
            &plan.nodes,
            plan.nodes.len().saturating_sub(1),
            plan.stamp.0,
        )
        .map_err(|_| HostRevealError::StalePlan)?;
        let (frames, clips, _) = projected(&plan.nodes, &plan.targets);
        if requires_complete_curve_landing(&plan.nodes) {
            let landing = expanded_rect(
                plan.required_witness.unwrap_or(plan.target_rect),
                plan.options.margins,
            );
            let projected_landing = translated_witness(&plan.nodes, &plan.targets, landing)
                .ok_or(HostRevealError::StalePlan)?;
            let curve_shapes =
                translated_curve_shapes(&plan.nodes, &frames, plan.nodes.len().saturating_sub(1))
                    .map_err(|_| HostRevealError::StalePlan)?;
            if !clips
                .last()
                .is_some_and(|clip| contains_rect(*clip, projected_landing))
            {
                return Err(HostRevealError::StalePlan);
            }
            validate_rect_in_curve_shapes(projected_landing, &curve_shapes)
                .map_err(|_| HostRevealError::StalePlan)?;
        } else if let Some(witness) = plan.required_witness {
            let projected_witness = translated_witness(&plan.nodes, &plan.targets, witness)
                .ok_or(HostRevealError::StalePlan)?;
            if !clips
                .last()
                .is_some_and(|clip| contains_rect(*clip, projected_witness))
            {
                return Err(HostRevealError::StalePlan);
            }
        }
        if plan.purpose == PlanPurpose::LogicalFocus
            && (path.iter().any(|n| {
                n.component.requires_focus_reveal() || n.component::<ScrollContent>().is_some()
            }) || path
                .last()
                .is_none_or(|n| !n.component.focusable() || n.component.semantics().disabled))
        {
            return Err(HostRevealError::StalePlan);
        }
        Ok(())
    }
    /// Revalidate the entire plan before any offset/reading/focus mutation. It
    /// consumes the plan; no layout callbacks or ordinary fallible tail run after
    /// this gate. Request normal rendering before publishing new geometry.
    pub fn commit_host_reveal(
        &mut self,
        plan: HostRevealPlan,
    ) -> Result<HostRevealOutcome, HostRevealError> {
        self.validate_host_reveal(&plan)?;
        self.commit_host_reveal_validated(plan)
    }
    pub(super) fn commit_host_reveal_in_focus_scope(
        &mut self,
        plan: HostRevealPlan,
        scope_id: ComponentId,
        scope_mount: u64,
    ) -> Result<HostRevealOutcome, HostRevealError> {
        self.validate_host_reveal_in_focus_scope(&plan, scope_id, scope_mount)?;
        self.commit_host_reveal_validated(plan)
    }
    fn commit_host_reveal_validated(
        &mut self,
        plan: HostRevealPlan,
    ) -> Result<HostRevealOutcome, HostRevealError> {
        let ready = self.prepare_host_reveal_commit(plan, 0)?;
        Ok(self.commit_prepared_host_reveal(ready))
    }
    pub(super) fn prepare_host_reveal_commit(
        &self,
        plan: HostRevealPlan,
        prior_input_advances: u64,
    ) -> Result<PreparedHostReveal, HostRevealError> {
        let next_input = self
            .reveal_clock
            .input
            .and_then(|n| n.checked_add(prior_input_advances))
            .and_then(|n| n.checked_add(1))
            .ok_or(HostRevealError::IdentityExhausted)?;
        let count = plan
            .nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                n.scroll
                    .as_ref()
                    .is_some_and(|s| plan.explicit || plan.targets[*i] != s.offset)
            })
            .count();
        let first = crate::scroll_view::reveal::reserve_reveal_identities(count)
            .map_err(HostRevealError::InvalidOptions)?;
        let mut prepared = Vec::with_capacity(count);
        for (i, n) in plan.nodes.iter().enumerate() {
            if n.scroll
                .as_ref()
                .is_some_and(|s| plan.explicit || plan.targets[i] != s.offset)
            {
                let scroll = plan
                    .route
                    .node(&self.root, plan.scope_depth + i)
                    .expect("validated readonly scroll route")
                    .component::<ScrollContent>()
                    .unwrap();
                let generation = first + u64::try_from(prepared.len()).expect("reserved count");
                prepared.push((
                    plan.scope_depth + i,
                    scroll.scroll.prepare_reveal_commit(
                        plan.targets[i],
                        plan.outcome.limit_clamped,
                        plan.options,
                        generation,
                    ),
                ));
            }
        }
        Ok(PreparedHostReveal {
            route: plan.route,
            writes: prepared,
            outcome: plan.outcome,
            next_input,
        })
    }
    pub(super) fn commit_prepared_host_reveal(
        &mut self,
        ready: PreparedHostReveal,
    ) -> HostRevealOutcome {
        // All identity, numeric, allocation and animation work precedes mutation.
        let mut outcome = ready.outcome;
        for (depth, prepared) in ready.writes {
            let scroll = ready
                .route
                .node_mut(&mut self.root, depth)
                .expect("validated exact scroll index route")
                .component_mut::<ScrollContent>()
                .unwrap();
            let result = scroll.scroll.commit_reveal(prepared);
            outcome.changed |= result.changed;
            outcome.animated |= result.animated;
            scroll.stop_following();
            if let Some(reading) = &mut scroll.reading {
                reading.explicit_reveal = Some(scroll.scroll.reveal_identity());
                reading.fence_momentum();
            }
        }
        let _ = self.reveal_clock.geometry_lease.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| Some(n.saturating_add(1)),
        );
        self.reveal_clock.input = Some(ready.next_input);
        if outcome.changed {
            self.reveal_clock.pending = true;
            self.dirty = true;
        }
        outcome
    }
}

impl HostRevealPlan {
    pub(super) fn target_route(&self) -> ContentRoute {
        self.route.clone()
    }
    pub(super) fn item_steps(&self) -> Vec<content_reveal::Step> {
        let (frames, _, viewports) = projected(&self.nodes, &self.targets);
        let delta = Point::new(
            frames.last().unwrap().min_x() - self.nodes.last().unwrap().frame.min_x(),
            frames.last().unwrap().min_y() - self.nodes.last().unwrap().frame.min_y(),
        );
        let mut rect = self.target_rect.offset(delta.x, delta.y);
        let mut steps = Vec::new();
        for (i, n) in self.nodes.iter().enumerate().rev().skip(1) {
            if let Some(s) = &n.scroll {
                let origin = Point::new(
                    viewports[i].min_x() - self.targets[i].x,
                    viewports[i].min_y() - self.targets[i].y,
                );
                steps.push(content_reveal::Step {
                    id: n.id,
                    mount: n.mount,
                    rect: rect.offset(-origin.x, -origin.y),
                    target: self.targets[i],
                    viewport: s.viewport_size,
                    document: s.document,
                    generation: 0,
                });
                rect = intersection(rect, viewports[i]);
            }
        }
        steps
    }
}
impl ContentHost {
    // Existing chart unit verifies the default-scope query; production focus
    // now plans against its explicitly authenticated post-dismiss scope.
    #[cfg(test)]
    pub(super) fn plan_focus_reveal(
        &self,
        item: ComponentId,
    ) -> Result<HostRevealPlan, HostRevealError> {
        self.plan_focus_reveal_in_scope(item, self.scope(), None)
    }
    pub(super) fn plan_focus_reveal_in_focus_scope(
        &self,
        item: ComponentId,
        scope_id: ComponentId,
        scope_mount: u64,
        prepared_rect: Option<Rect>,
    ) -> Result<HostRevealPlan, HostRevealError> {
        let scope = self
            .root
            .find_active(scope_id)
            .filter(|node| node.mount == scope_mount)
            .ok_or(HostRevealError::UnavailableTarget)?;
        self.plan_focus_reveal_in_scope(item, scope, prepared_rect)
    }
    /// Only the checked modal transaction selects its already-prepared post-close root scope.
    pub(super) fn plan_focus_reveal_after_modal_close(
        &self,
        item: ComponentId,
    ) -> Result<HostRevealPlan, HostRevealError> {
        self.plan_focus_reveal_in_scope(item, &self.root, None)
    }
    fn plan_focus_reveal_in_scope(
        &self,
        item: ComponentId,
        scope: &Content,
        prepared_rect: Option<Rect>,
    ) -> Result<HostRevealPlan, HostRevealError> {
        let budget = HostRevealBudget::default();
        if !self.window_active {
            return Err(HostRevealError::UnavailableTarget);
        }
        let mut path = Vec::new();
        if !collect_path(scope, item, &mut path, budget.max_path_nodes)? {
            return Err(HostRevealError::UnavailableTarget);
        }
        let target = *path.last().unwrap();
        if !target.component.focusable() || target.component.semantics().disabled {
            return Err(HostRevealError::UnavailableTarget);
        }
        let target_requirement = target.component.required_focus_space();
        let requires_reveal = self.root.curve_ancestor(scope.id, false).unwrap_or(false)
            || path.iter().any(|n| n.component.requires_focus_reveal())
            || matches!(
                target_requirement,
                RequiredFocusSpace::Sufficient { .. }
                    | RequiredFocusSpace::Insufficient
                    | RequiredFocusSpace::Unmeasured
            );

        let has_scroll = path
            .iter()
            .take(path.len().saturating_sub(1))
            .any(|n| n.component::<ScrollContent>().is_some());
        if !has_scroll && !requires_reveal {
            // Explicit legacy policy chosen before any reveal attempt. Current
            // mount/scope/availability/IME still authorize logical focus, but
            // stale or offscreen Rect geometry grants no reveal guarantee.
            let (route, scope_depth) = self.capture_reveal_route(&path, budget.max_path_nodes)?;
            let nodes: Vec<_> = path
                .iter()
                .map(|n| NodeState::read(n))
                .collect::<Result<_, _>>()?;
            return Ok(HostRevealPlan {
                owner: self.reveal_clock.owner.clone(),
                stamp: self.reveal_clock.stamp()?,
                focus: self.focused,
                focus_epoch: self.focus_epoch,
                targets: vec![Point::default(); nodes.len()],
                target_rect: target.frame,
                required_witness: None,
                required_evidence: None,
                target_caret: target.component.text_surface().map(|s| s.caret()),
                nodes,
                route,
                scope_depth,
                options: ScrollRevealOptions {
                    animated: false,
                    ..Default::default()
                },
                outcome: HostRevealOutcome::default(),
                explicit: false,
                path_limit: budget.max_path_nodes,
                purpose: PlanPurpose::LogicalFocus,
            });
        }
        // Target focus-space remains independent of any certified ancestor clip.
        let (required_witness, required_evidence) = match target.component.required_focus_space() {
            RequiredFocusSpace::Sufficient { witness, evidence } => (Some(witness), Some(evidence)),
            RequiredFocusSpace::Insufficient | RequiredFocusSpace::Unmeasured => {
                return Err(HostRevealError::NoFeasibleRegion);
            }
            RequiredFocusSpace::Legacy if target.component.requires_focus_reveal() => {
                return Err(HostRevealError::UnsupportedLayout);
            }
            RequiredFocusSpace::NotRequired if target.component.requires_focus_reveal() => {
                return Err(HostRevealError::UnsupportedLayout);
            }
            RequiredFocusSpace::Legacy | RequiredFocusSpace::NotRequired => (None, None),
        };
        let rect = required_witness.or(prepared_rect).unwrap_or_else(|| {
            target
                .component
                .text_surface()
                .filter(|s| s.presentation == crate::TextSurfacePresentation::Content)
                .map_or(target.frame, |s| {
                    let (horizontal, vertical) = s.scroll.scroll_policy();
                    if s.input.read_only
                        && (horizontal != crate::ScrollPolicy::Disabled
                            || vertical != crate::ScrollPolicy::Disabled)
                    {
                        // Internally scrolled readonly content paints its focus
                        // indicator on the viewport, not an insertion caret.
                        // Keep its reading position when entering that viewport.
                        s.outer
                    } else {
                        s.caret()
                    }
                })
        });
        let rect = if let Some(scroll) = path
            .iter()
            .rev()
            .find_map(|n| n.component::<ScrollContent>())
        {
            crate::extended::ControlFrame::new(rect, None).focus_bounds(scroll.backing_scale)
        } else {
            rect
        };
        let mut plan = self.plan_host_reveal_rect_in_scope(
            scope,
            item,
            rect,
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &[],
            budget,
        )?;
        plan.explicit = false;
        plan.required_witness = required_witness;
        plan.required_evidence = required_evidence;
        if let Some(witness) = required_witness {
            let (_, final_clips, _) = projected(&plan.nodes, &plan.targets);
            let projected_witness = translated_witness(&plan.nodes, &plan.targets, witness);
            if !projected_witness.is_some_and(|w| {
                final_clips
                    .last()
                    .is_some_and(|clip| contains_rect(*clip, w))
            }) {
                return Err(HostRevealError::NoFeasibleRegion);
            }
        }
        Ok(plan)
    }
}

impl HostRevealPlan {
    pub fn requested_options(&self) -> ScrollRevealOptions {
        self.options
    }
    pub fn planned_offsets(&self) -> impl Iterator<Item = (ComponentId, Point)> + '_ {
        self.nodes
            .iter()
            .zip(&self.targets)
            .filter(|(n, _)| n.scroll.is_some())
            .map(|(n, p)| (n.id, *p))
    }
    pub fn geometry_generation(&self) -> u64 {
        self.stamp.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Insets;
    use tekes_ui_scene::{ClipBudget, ClipShape, CornerRadii};
    fn region() -> ClipRegion {
        ClipRegion::new(
            &[ClipShape::normalize(
                Rect::new(0., 0., 100., 100.),
                CornerRadii::uniform(20.),
                Point::default(),
            )
            .unwrap()],
            ClipBudget {
                max_shapes: 1,
                max_work: 100_000,
            },
        )
        .unwrap()
    }
    fn solve(
        y: f32,
        h: f32,
        offset: f32,
        limit: f32,
        placement: ScrollRevealPlacement,
    ) -> Result<(Point, HostRevealOutcome), HostRevealError> {
        constrained_axis(
            &region(),
            1,
            Rect::new(2., y - offset, 6., h),
            Rect::new(0., 0., 100., 100.),
            Point::new(0., -offset),
            Point::new(0., offset),
            Point::new(0., limit),
            ScrollRevealOptions {
                placement,
                margins: Insets::new(2., 2., 2., 2.),
                animated: false,
            },
        )
    }
    #[test]
    fn design_start_nearest_and_tall_contract() {
        let start = ScrollRevealPlacement::Start(Axis::Vertical);
        assert_eq!(solve(50., 10., 0., 100., start).unwrap().0.y, 28.);
        assert_eq!(
            solve(50., 10., 40., 100., ScrollRevealPlacement::Nearest)
                .unwrap()
                .0
                .y,
            28.
        );
        assert_eq!(
            solve(50., 10., 10., 100., ScrollRevealPlacement::Nearest)
                .unwrap()
                .0
                .y,
            10.
        );
        let (p, r) = solve(50., 80., 40., 100., ScrollRevealPlacement::Nearest).unwrap();
        assert_eq!(p.y, 40.);
        assert!(r.partial);
        let (p, r) = solve(50., 80., 0., 100., ScrollRevealPlacement::Nearest).unwrap();
        assert_eq!(p.y, 28.);
        assert!(r.partial);
        let (p, r) = solve(50., 10., 0., 10., start).unwrap();
        assert_eq!(p.y, 10.);
        assert!(r.limit_clamped);
    }
    #[test]
    fn outside_cross_axis_is_not_corner_visibility() {
        let result = constrained_axis(
            &region(),
            1,
            Rect::new(1., 50., 7., 10.),
            Rect::new(0., 0., 100., 100.),
            Point::default(),
            Point::default(),
            Point::new(0., 100.),
            ScrollRevealOptions {
                placement: ScrollRevealPlacement::Start(Axis::Vertical),
                margins: Insets::new(2., 2., 2., 2.),
                animated: false,
            },
        );
        assert_eq!(result, Err(HostRevealError::NoFeasibleRegion));
    }
    #[test]
    fn epoch_exhaustion_never_wraps() {
        let mut clock = RevealClock {
            geometry: Some(u64::MAX),
            ..Default::default()
        };
        clock.invalidate_geometry();
        assert_eq!(clock.stamp(), Err(HostRevealError::IdentityExhausted));
        let mut clock = RevealClock {
            input: Some(u64::MAX),
            ..Default::default()
        };
        clock.invalidate_input();
        assert_eq!(clock.stamp(), Err(HostRevealError::IdentityExhausted));
        if usize::BITS == u64::BITS {
            assert_eq!(
                crate::scroll_view::reveal::reserve_reveal_identities(usize::MAX),
                Err(ScrollRevealError::IdentityExhausted)
            );
        }
    }
}

#[cfg(test)]
mod additional_negatives {
    use super::*;
    use tekes_ui_scene::{ClipBudget, ClipShape, CornerRadii};
    #[test]
    fn fractional_landing_uncertainty_is_explicit() {
        let region = ClipRegion::new(
            &[ClipShape::normalize(
                Rect::new(0., 0., 100., 100.),
                CornerRadii::uniform(20.),
                Point::default(),
            )
            .unwrap()],
            ClipBudget {
                max_shapes: 1,
                max_work: 10000,
            },
        )
        .unwrap();
        let result = constrained_axis(
            &region,
            1,
            Rect::new(2.1875, 50., 6., 10.),
            Rect::new(0., 0., 100., 100.),
            Point::default(),
            Point::default(),
            Point::new(0., 100.),
            ScrollRevealOptions {
                placement: ScrollRevealPlacement::Start(Axis::Vertical),
                margins: crate::Insets::new(2., 2., 2., 2.),
                animated: false,
            },
        );
        assert!(matches!(
            result,
            Err(HostRevealError::Geometry(
                ClipError::NumericallyIndeterminate
            ))
        ));
    }
}

#[cfg(test)]
mod counter_commit_tests {
    use super::*;
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 7.).min(w), 18.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    #[test]
    fn actual_maximum_input_and_focus_epochs_reject_before_original_owner_mutation() {
        let a = ComponentId(2);
        let b = ComponentId(3);
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(a, Button::new("a")),
                    Content::new(b, Button::new("b")),
                ],
            },
        ))
        .unwrap();
        let f = Rect::new(0., 0., 200., 100.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.focus(Some(a), true));
        host.reveal_clock.input = Some(u64::MAX);
        let epoch = host.focus_epoch;
        let p = host
            .plan_host_reveal(
                b,
                ScrollRevealOptions {
                    animated: false,
                    ..Default::default()
                },
                &[],
                HostRevealBudget::default(),
            )
            .unwrap();
        assert_eq!(
            host.commit_host_reveal(p),
            Err(HostRevealError::IdentityExhausted)
        );
        assert_eq!(host.focused, Some(a));
        assert_eq!(host.focus_epoch, epoch);
        assert_eq!(host.reveal_clock.input, Some(u64::MAX));
        assert!(!host.focus(Some(b), true));
        assert_eq!(host.focused, Some(a));
        assert_eq!(host.reveal_clock.input, Some(u64::MAX));
        host.reveal_clock.input = Some(1);
        host.focus_epoch = u64::MAX;
        assert!(!host.focus(Some(b), true));
        assert_eq!(host.focused, Some(a));
        assert_eq!(host.focus_epoch, u64::MAX);
        assert_eq!(host.reveal_clock.input, Some(1));
    }
}

#[cfg(test)]
mod convergence_tests {
    use super::*;
    #[test]
    fn repeated_complete_numeric_state_rejects_without_adding_a_fallback_state() {
        let mut history = Vec::new();
        let a = [Point::new(0., 28.)];
        let b = [Point::new(0., 40.)];
        record_numeric_state(&mut history, &a).unwrap();
        record_numeric_state(&mut history, &b).unwrap();
        assert_eq!(
            record_numeric_state(&mut history, &a),
            Err(HostRevealError::NoConvergence)
        );
        assert_eq!(history.len(), 2);
    }
}

fn expanded_rect(r: Rect, m: crate::Insets) -> Rect {
    Rect::new(
        r.min_x() - m.left,
        r.min_y() - m.top,
        r.size.width + m.left + m.right,
        r.size.height + m.top + m.bottom,
    )
}

fn projected_curve_region(
    nodes: &[NodeState],
    frames: &[Rect],
    through: usize,
    budget: HostRevealBudget,
    generation: u64,
) -> Result<Option<ClipRegion>, HostRevealError> {
    use tekes_ui_scene::{ClipBudget, ClipShape};
    let mut shapes = Vec::new();
    if let Some(first) = nodes.first() {
        for HostRevealGeometry::Curve { shape, .. } in &first.fixed_ancestor_curves {
            if !shapes.contains(shape) {
                shapes.push(*shape);
            }
        }
    }
    for (i, node) in nodes.iter().enumerate().take(through.saturating_add(1)) {
        if node.curve_requirement && node.reveal_geometry.is_none() {
            return Err(HostRevealError::UnsupportedLayout);
        }
        let Some(HostRevealGeometry::Curve {
            generation: current_generation,
            shape,
        }) = node.reveal_geometry
        else {
            continue;
        };
        if current_generation != generation || shape.is_empty() {
            return Err(HostRevealError::NoFeasibleRegion);
        }
        let delta = Point::new(
            frames[i].min_x() - node.frame.min_x(),
            frames[i].min_y() - node.frame.min_y(),
        );
        let moved = ClipShape::normalize(shape.rect(), shape.radii(), delta)
            .map_err(HostRevealError::Geometry)?;
        if !shapes.contains(&moved) {
            shapes.push(moved);
        }
    }
    if shapes.is_empty() {
        Ok(None)
    } else {
        ClipRegion::new(
            &shapes,
            ClipBudget {
                max_shapes: budget.max_path_nodes,
                max_work: budget.max_geometry_work,
            },
        )
        .map(Some)
        .map_err(HostRevealError::Geometry)
    }
}

fn requires_complete_curve_landing(nodes: &[NodeState]) -> bool {
    nodes
        .first()
        .is_some_and(|node| !node.fixed_ancestor_curves.is_empty())
        || nodes
            .iter()
            .take(nodes.len().saturating_sub(1))
            .any(|node| node.curve_requirement || node.reveal_geometry.is_some())
}

fn validate_curve_constraints(
    nodes: &[NodeState],
    ancestor_count: usize,
    generation: u64,
) -> Result<(), HostRevealError> {
    if let Some(first) = nodes.first() {
        for HostRevealGeometry::Curve {
            generation: current,
            shape,
        } in &first.fixed_ancestor_curves
        {
            if *current != generation {
                return Err(HostRevealError::StalePlan);
            }
            if shape.is_empty() {
                return Err(HostRevealError::NoFeasibleRegion);
            }
        }
    }
    for node in nodes.iter().take(ancestor_count) {
        if node.curve_requirement && node.reveal_geometry.is_none() {
            return Err(HostRevealError::UnsupportedLayout);
        }
        if let Some(HostRevealGeometry::Curve {
            generation: current,
            shape,
        }) = node.reveal_geometry
        {
            if current != generation {
                return Err(HostRevealError::StalePlan);
            }
            if shape.is_empty() {
                return Err(HostRevealError::NoFeasibleRegion);
            }
        }
    }
    Ok(())
}

fn translated_curve_shapes(
    nodes: &[NodeState],
    frames: &[Rect],
    ancestor_count: usize,
) -> Result<Vec<ClipShape>, HostRevealError> {
    let mut shapes = Vec::new();
    if let Some(first) = nodes.first() {
        for HostRevealGeometry::Curve { shape, .. } in &first.fixed_ancestor_curves {
            if !shapes.contains(shape) {
                shapes.push(*shape);
            }
        }
    }
    for (i, node) in nodes.iter().enumerate().take(ancestor_count) {
        if node.curve_requirement && node.reveal_geometry.is_none() {
            return Err(HostRevealError::UnsupportedLayout);
        }
        let Some(HostRevealGeometry::Curve { shape, .. }) = node.reveal_geometry else {
            continue;
        };
        let delta = Point::new(
            frames[i].min_x() - node.frame.min_x(),
            frames[i].min_y() - node.frame.min_y(),
        );
        let moved = ClipShape::normalize(shape.rect(), shape.radii(), delta)
            .map_err(HostRevealError::Geometry)?;
        if !shapes.contains(&moved) {
            shapes.push(moved);
        }
    }
    Ok(shapes)
}

fn validate_rect_in_curve_shapes(rect: Rect, shapes: &[ClipShape]) -> Result<(), HostRevealError> {
    if [
        rect.min_x(),
        rect.min_y(),
        rect.max_x(),
        rect.max_y(),
        rect.size.width,
        rect.size.height,
    ]
    .into_iter()
    .any(|v| !v.is_finite())
        || rect.size.width <= 0.
        || rect.size.height <= 0.
    {
        return Err(HostRevealError::NoFeasibleRegion);
    }
    // ClipShape currently represents only convex axis-aligned rectangles or
    // rounded rectangles. Four-corner containment is valid for these domains;
    // do not generalize it to arbitrary masks.
    for shape in shapes {
        for point in [
            [rect.min_x(), rect.min_y()],
            [rect.max_x(), rect.min_y()],
            [rect.min_x(), rect.max_y()],
            [rect.max_x(), rect.max_y()],
        ] {
            match shape.contains([f64::from(point[0]), f64::from(point[1])]) {
                Ok(true) => {}
                Ok(false) => return Err(HostRevealError::NoFeasibleRegion),
                Err(error) => return Err(HostRevealError::Geometry(error)),
            }
        }
    }
    Ok(())
}
