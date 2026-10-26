//! Original Host read-only translation plans. No mutating component callbacks during planning.
use super::*;
use crate::{
    ReadingDocumentVersion, ScrollRevealError, ScrollRevealOptions, ScrollRevealPlacement,
};
use std::sync::Arc;
use tekes_ui_scene::{ClipError, ClipRegion};

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
    geometry: Option<u64>,
    input: Option<u64>,
    pub(super) pending: bool,
}
impl Default for RevealClock {
    fn default() -> Self {
        Self {
            owner: Arc::new(()),
            geometry: Some(0),
            input: Some(0),
            pending: false,
        }
    }
}
impl RevealClock {
    pub(super) fn invalidate_input(&mut self) {
        self.input = self.input.and_then(|n| n.checked_add(1));
    }
    pub(super) fn invalidate_geometry(&mut self) {
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
    item_mounts:Option<std::collections::BTreeMap<ComponentId,u64>>,
}
#[derive(Clone, Debug, PartialEq)]
struct NodeState {
    id: ComponentId,
    mount: u64,
    frame: Rect,
    clip: Rect,
    scroll: Option<ScrollState>,
    child_clip:Option<Rect>,
}
impl NodeState {
    fn read(node: &Content) -> Result<Self, HostRevealError> {
        let child_clip = node.component.host_reveal_local_clip();
        if child_clip.is_some_and(|r| ![r.origin.x, r.origin.y, r.size.width, r.size.height, r.max_x(), r.max_y()]
            .into_iter().all(f32::is_finite) || r.size.width < 0. || r.size.height < 0.) {
            return Err(HostRevealError::Geometry(ClipError::InvalidInput));
        }
        Ok(Self {
            id: node.id,
            mount: node.mount,
            frame: node.frame,
            clip: node.clip,
            child_clip,
            scroll: node.component::<ScrollContent>().map(|s| ScrollState {
                viewport: s.viewport,
                viewport_size: s.scroll.viewport_size(),
                document: s.scroll.document_size(),
                offset: s.scroll.offset,
                visual: s.scroll.visual_offset(),
                limit: s.scroll.limit(),
                reveal: s.scroll.reveal_identity(),
                version: s.reading_document_version().cloned(),
                item_mounts:s.reading.as_ref().map(|r|r.item_mounts.clone()),
            }),
        })
    }
}
fn read_reveal_nodes(path: &[&Content]) -> Result<Vec<NodeState>, HostRevealError> {
    let nodes = path.iter().map(|n| NodeState::read(n)).collect::<Result<Vec<_>, _>>()?;
    for node in path.iter().take(path.len().saturating_sub(1)) {
        if node.component::<StackContent>().is_none()
            && node.component::<GridContent>().is_none()
            && node.component::<ScrollContent>().is_none()
            && node.component::<ListContent>().is_none()
            && !node.component.host_reveal_passthrough() {
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
    fn require_passthrough(&mut self, root: &Content, depth: usize) -> Result<(), HostRevealError> {
        if self.node(root, depth).is_none_or(|n| !n.component.host_reveal_passthrough()) {
            return Err(HostRevealError::UnsupportedLayout);
        }
        if !self.passthrough_depths.contains(&depth) { self.passthrough_depths.push(depth); }
        Ok(())
    }
    fn passthrough_current(&self, root: &Content) -> bool {
        self.passthrough_depths.iter().all(|depth| self.node(root, *depth)
            .is_some_and(|n| n.component.host_reveal_passthrough()))
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
            {
                if !node.component.host_reveal_passthrough() { return false; }
                if !self.passthrough_depths.contains(&i) { self.passthrough_depths.push(i); }
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
fn projected(nodes: &[NodeState], targets: &[Point]) -> (Vec<Rect>, Vec<Rect>, Vec<Rect>) {
    let mut frames = Vec::with_capacity(nodes.len());
    let mut clips = Vec::with_capacity(nodes.len());
    let mut viewports = Vec::with_capacity(nodes.len());
    let mut delta = Point::default();
    let mut clip = nodes[0].clip;
    for (i, n) in nodes.iter().enumerate() {
        if let Some(boundary)=n.child_clip {clip=intersection(clip,boundary.offset(delta.x,delta.y));}
        frames.push(n.frame.offset(delta.x, delta.y));
        clips.push(clip);
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
                && !route.library_scroll_parent_path(&self.root, base + i) {
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
        &self, scope: &Content, item: ComponentId, rect: Rect,
        options: ScrollRevealOptions, constraints: &[(ComponentId, ClipRegion)],
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
        if path.iter().any(|n| n.component.requires_focus_reveal()) {
            // B1 must supply this structural owner's authoritative current
            // geometry. Caller-supplied extra constraints cannot replace it.
            return Err(HostRevealError::UnsupportedLayout);
        }
        let (mut route, scope_depth) = self.capture_reveal_route(&path, budget.max_path_nodes)?;
        let nodes=read_reveal_nodes(&path)?;
        for (i, n) in path.iter().take(path.len().saturating_sub(1)).enumerate() {
            // Original row authority was already resolved by its exact owner.
            // Only generic geometry paths use the separate positive opt-in.
            if n.component::<StackContent>().is_none()
                && n.component::<GridContent>().is_none()
                && n.component::<ScrollContent>().is_none()
                && n.component::<ListContent>().is_none() {
                route.require_passthrough(&self.root, scope_depth + i)?;
            }
        }
        if nodes.iter().any(|n| n.child_clip.is_some_and(|r| r.is_empty())) {
            return Err(HostRevealError::NoFeasibleRegion);
        }
        let mut targets: Vec<_> = nodes
            .iter()
            .map(|n| n.scroll.as_ref().map_or(Point::default(), |s| s.visual))
            .collect();
        for s in nodes.iter().filter_map(|n| n.scroll.as_ref()) {
            if s.visual != s.offset {
                return Err(HostRevealError::UnsupportedLayout);
            }
        }

        let expanded = Rect::new(
            rect.min_x() - m.left,
            rect.min_y() - m.top,
            rect.size.width + m.left + m.right,
            rect.size.height + m.top + m.bottom,
        );
        if constraints.is_empty()
            && options.placement == ScrollRevealPlacement::Nearest
            && expanded.intersection(&path.last().unwrap().clip) == Some(expanded)
        {
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
        let baseline = projected(&nodes, &targets);
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
                let (mut wanted, clamped) = numeric
                    .reveal_target(local, step_options)
                    .map_err(HostRevealError::InvalidOptions)?;
                outcome.limit_clamped |= clamped;
                outcome.partial |= local.size.width
                    + step_options.margins.left
                    + step_options.margins.right
                    > state.viewport_size.width
                    || local.size.height + step_options.margins.top + step_options.margins.bottom
                        > state.viewport_size.height;
                if let Some((_, region)) = constraints.iter().find(|(id, _)| *id == nodes[i].id) {
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
            let Some((_, region)) = constraints.iter().find(|(id, _)| *id == node.id) else {
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
            targets,
            options,
            outcome,
            explicit: true,
            path_limit: budget.max_path_nodes,
            purpose: PlanPurpose::Reveal,
        })
    }
    pub(super) fn validate_host_reveal(&self, plan: &HostRevealPlan) -> Result<(), HostRevealError> {
        self.validate_host_reveal_in_scope(plan, self.scope())
    }
    pub(super) fn validate_host_reveal_in_scope(&self, plan: &HostRevealPlan, scope: &Content) -> Result<(), HostRevealError> {
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
            || !plan.route.passthrough_current(&self.root) {
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
            || (if plan.purpose==PlanPurpose::Reveal {read_reveal_nodes(&path).map_err(|_|HostRevealError::StalePlan)?}else{path.iter().map(|n|NodeState::read(n)).collect::<Result<_, _>>().map_err(|_|HostRevealError::StalePlan)?})!=plan.nodes
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
    fn commit_host_reveal_validated(
        &mut self,
        plan: HostRevealPlan,
    ) -> Result<HostRevealOutcome, HostRevealError> {
        let ready = self.prepare_host_reveal_commit(plan, 0)?;
        Ok(self.commit_prepared_host_reveal(ready))
    }
    pub(super) fn prepare_host_reveal_commit(
        &self, plan: HostRevealPlan, prior_input_advances: u64,
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
        Ok(PreparedHostReveal { route: plan.route, writes: prepared, outcome: plan.outcome, next_input })
    }
    pub(super) fn commit_prepared_host_reveal(&mut self, ready: PreparedHostReveal) -> HostRevealOutcome {
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
}
impl ContentHost {
    pub(super) fn plan_focus_reveal(
        &self,
        item: ComponentId,
    ) -> Result<HostRevealPlan, HostRevealError> {
        self.plan_focus_reveal_with_rect(item,None)
    }
    pub(super) fn plan_focus_reveal_with_rect(&self,item:ComponentId,prepared_rect:Option<Rect>)->Result<HostRevealPlan,HostRevealError> {
        self.plan_focus_reveal_in_scope(item, self.scope(), prepared_rect)
    }
    /// Only the checked modal transaction selects its already-prepared post-close root scope.
    pub(super) fn plan_focus_reveal_after_modal_close(&self,item:ComponentId)->Result<HostRevealPlan,HostRevealError> {
        self.plan_focus_reveal_in_scope(item, &self.root, None)
    }
    fn plan_focus_reveal_in_scope(&self,item:ComponentId,scope:&Content,prepared_rect:Option<Rect>)->Result<HostRevealPlan,HostRevealError> {
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
        let requires_reveal = path.iter().any(|n| n.component.requires_focus_reveal());
        let has_scroll = path
            .iter()
            .take(path.len().saturating_sub(1))
            .any(|n| n.component::<ScrollContent>().is_some());
        if !has_scroll && !requires_reveal {
            // Explicit legacy policy chosen before any reveal attempt. Current
            // mount/scope/availability/IME still authorize logical focus, but
            // stale or offscreen Rect geometry grants no reveal guarantee.
            let (route, scope_depth) = self.capture_reveal_route(&path, budget.max_path_nodes)?;
            let nodes: Vec<_> = path.iter().map(|n| NodeState::read(n)).collect::<Result<_, _>>()?;
            return Ok(HostRevealPlan {
                owner: self.reveal_clock.owner.clone(),
                stamp: self.reveal_clock.stamp()?,
                focus: self.focused,
                focus_epoch: self.focus_epoch,
                targets: vec![Point::default(); nodes.len()],
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
        // B1 supplies the declared curve owner's current region through the
        // common strict planner. A declaration alone is never Rect permission.
        if requires_reveal {
            return Err(HostRevealError::UnsupportedLayout);
        }
        let rect = prepared_rect.unwrap_or_else(||target
            .component
            .text_surface()
            .filter(|s| s.presentation == crate::TextSurfacePresentation::Content)
            .map_or(target.frame, |s| s.caret()));
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
            scope, item,
            rect,
            ScrollRevealOptions {
                animated: false,
                ..Default::default()
            },
            &[],
            budget,
        )?;
        plan.explicit = false;
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
                    Content::new(a, ButtonContent::new("a")),
                    Content::new(b, ButtonContent::new("b")),
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
