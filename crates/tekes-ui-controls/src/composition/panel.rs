//! Original single-slot Panel. Geometry is frozen by the original Host after arrange.
use super::*;
use tekes_ui_scene::{ClipBudget, ClipRegion, ClipShape, CornerRadii, ShapeKey, ShapeTable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveSubtreeCapability {
    Unsupported,
    Local,
    Recursive,
}

#[derive(Clone, Debug)]
pub struct PanelGeometry {
    pub generation: u64,
    pub outer: Rect,
    pub allocation: Rect,
    pub shape: ClipShape,
}
/// Immutable geometry from the actual owning ContentHost, scoped to the
/// component receiving the callback. This projection carries identity and
/// normalized shapes; it is not a cross-process grant or proof receipt.
#[derive(Clone, Debug)]
pub struct CurveGeometryContext {
    generation: u64,
    shapes: std::sync::Arc<[ClipShape]>,
}
impl CurveGeometryContext {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn shapes(&self) -> &[ClipShape] {
        &self.shapes
    }
}
/// An exact current local constraint exposed to the existing Host reveal planner.
/// This is an ancestor boundary, never a target RequiredFocus witness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRevealGeometry {
    Curve { generation: u64, shape: ClipShape },
}
#[derive(Debug)]
pub struct PanelContent {
    child: Vec<Content>,
    canvas: bool,
    padding: Option<crate::Insets>,
    backing_scale: f32,
    geometry: Option<PanelGeometry>,
}
impl PanelContent {
    pub fn new(child: Content) -> Result<Self, CompositionError> {
        child.validate_curve_tree(true)?;
        Ok(Self {
            child: vec![child],
            canvas: false,
            padding: None,
            backing_scale: 1.,
            geometry: None,
        })
    }
    pub fn set_canvas(&mut self, canvas: bool) {
        self.canvas = canvas;
    }
    pub fn set_padding(&mut self, padding: Option<crate::Insets>) -> Result<(), CompositionError> {
        if padding.is_some_and(|p| {
            [p.top, p.right, p.bottom, p.left]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.)
        }) {
            return Err(CompositionError::InvalidPanelPadding);
        }
        self.padding = padding;
        Ok(())
    }
    pub fn child(&self) -> &Content {
        &self.child[0]
    }
    pub fn geometry(&self) -> Option<&PanelGeometry> {
        self.geometry.as_ref()
    }
    fn insets(&self) -> crate::Insets {
        self.padding.unwrap_or_else(|| {
            let p = tekes_ui_tokens::proposal::metric("spacing", "space-loose")
                .expect("Surface padding");
            crate::Insets {
                top: p,
                right: p,
                bottom: p,
                left: p,
            }
        })
    }
    pub(super) fn paint_boundary(&self, scene: &mut Scene, theme: ContentTheme, focused: bool) {
        let Some(g) = &self.geometry else {
            return;
        };
        crate::paint::panel_boundary(
            scene,
            g.outer,
            theme.palette,
            theme.surface(),
            self.canvas,
            self.backing_scale,
            focused,
        );
    }
}
impl Component for PanelContent {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn requires_focus_reveal(&self) -> bool {
        true
    }
    fn host_reveal_geometry_constraint(&self) -> Option<HostRevealGeometry> {
        self.geometry.as_ref().map(|g| HostRevealGeometry::Curve {
            generation: g.generation,
            shape: g.shape,
        })
    }
    fn default_surface(&self) -> Option<&'static str> {
        Some(if self.canvas { "canvas" } else { "panel" })
    }
    fn children(&self) -> &[Content] {
        &self.child
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.child)
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.child.len() != 1 {
            return Err(CompositionError::InvalidSlotCount);
        }
        self.child[0].validate_curve_tree(true)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let p = self.insets();
        let mut m = self.child[0].measure(
            text,
            Size::new(
                (available.width - p.left - p.right).max(0.),
                (available.height - p.top - p.bottom).max(0.),
            ),
        );
        m.size.width += p.left + p.right;
        m.size.height += p.top + p.bottom;
        m.first_baseline = m.first_baseline.map(|baseline| baseline + p.top);
        m
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.backing_scale = text.backing_scale();
        let p = self.insets();
        let allocation = Rect::new(
            frame.min_x() + p.left,
            frame.min_y() + p.top,
            (frame.size.width - p.left - p.right).max(0.),
            (frame.size.height - p.top - p.bottom).max(0.),
        );
        self.child[0].arrange(text, allocation, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        crate::paint::panel_background(
            scene,
            frame,
            theme.palette,
            theme.surface(),
            self.canvas,
            self.backing_scale,
        );
    }
}
#[derive(Clone, Debug)]
pub(super) struct NodeGeometry {
    pub(super) generation: u64,
    pub(super) shapes: Vec<ClipShape>,
    inherited_keys: Vec<ShapeKey>,
}
#[derive(Clone, Debug)]
struct GeometryEntry {
    id: ComponentId,
    mount: u64,
    part: Option<usize>,
    frame: Rect,
    shapes: Vec<ClipShape>,
}
/// Immutable original Host geometry. Frames describe bounds; point authority uses exact domains.
#[derive(Clone, Debug)]
pub struct ContentGeometrySnapshot {
    pub generation: u64,
    entries: Vec<GeometryEntry>,
    lease: std::sync::Arc<std::sync::atomic::AtomicU64>,
    lease_generation: u64,
}
impl ContentGeometrySnapshot {
    pub fn is_current(&self) -> bool {
        self.lease_generation != u64::MAX
            && self.lease.load(std::sync::atomic::Ordering::Acquire) == self.lease_generation
    }
    pub fn identity(&self, id: ComponentId, part: Option<usize>) -> Option<u64> {
        if !self.is_current() {
            return None;
        }
        self.entries
            .iter()
            .find(|e| e.id == id && e.part == part)
            .map(|e| e.mount)
    }
    pub fn hit_test(&self, point: Point) -> Option<(ComponentId, u64, Option<usize>)> {
        if !self.is_current() {
            return None;
        }
        self.entries
            .iter()
            .rev()
            .find(|e| {
                bounds(&e.shapes, e.frame).ok().flatten().is_some()
                    && e.frame.contains(point)
                    && e.shapes
                        .iter()
                        .all(|s| s.contains([point.x as f64, point.y as f64]) == Ok(true))
            })
            .map(|e| (e.id, e.mount, e.part))
    }
    pub fn bounds(
        &self,
        id: ComponentId,
        part: Option<usize>,
        rect: Rect,
    ) -> Result<Option<Rect>, tekes_ui_scene::ClipError> {
        if !self.is_current() {
            return Err(tekes_ui_scene::ClipError::StaleShapeKey);
        }
        let Some(e) = self.entries.iter().find(|e| e.id == id && e.part == part) else {
            return Ok(None);
        };
        bounds(&e.shapes, rect)
    }
}
fn policy() -> ClipBudget {
    ClipBudget {
        max_shapes: 4096,
        max_work: 100_000_000,
    }
}
fn bounds(shapes: &[ClipShape], rect: Rect) -> Result<Option<Rect>, tekes_ui_scene::ClipError> {
    let mut all = shapes.to_vec();
    all.push(ClipShape::normalize(
        rect,
        CornerRadii::default(),
        Point::default(),
    )?);
    Ok(ClipRegion::new(&all, policy())?.visible_bounds()?.map(|b| {
        let lower = |v: f64| {
            let f = v as f32;
            if f as f64 > v { f.next_down() } else { f }
        };
        let upper = |v: f64| {
            let f = v as f32;
            if (f as f64) < v { f.next_up() } else { f }
        };
        let x = lower(b.min[0].lower);
        let y = lower(b.min[1].lower);
        let right = upper(b.max[0].upper);
        let bottom = upper(b.max[1].upper);
        Rect::new(
            x,
            y,
            upper(right as f64 - x as f64),
            upper(bottom as f64 - y as f64),
        )
    }))
}
#[derive(Debug)]
struct GeometryWork {
    remaining: usize,
}
impl GeometryWork {
    fn reserve(&mut self, amount: usize) -> Result<(), CompositionError> {
        self.remaining =
            self.remaining
                .checked_sub(amount)
                .ok_or(CompositionError::CurveGeometry(
                    tekes_ui_scene::ClipError::QueryBudgetExceeded,
                ))?;
        Ok(())
    }
}
impl Content {
    pub(super) fn clear_curve_geometry_context(&mut self) {
        self.component.set_curve_geometry_context(None);
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.clear_curve_geometry_context();
            }
        }
    }

    pub(super) fn has_curve_panel(&self) -> bool {
        self.component::<PanelContent>().is_some()
            || self
                .component
                .children()
                .iter()
                .any(Content::has_curve_panel)
    }

    pub(super) fn validate_curve_tree(&self, in_curve: bool) -> Result<(), CompositionError> {
        fn visit(node: &Content, in_curve: bool, left: &mut usize) -> Result<(), CompositionError> {
            let in_curve = in_curve || node.component::<PanelContent>().is_some();
            if in_curve {
                *left = left.checked_sub(1).ok_or(CompositionError::CurveGeometry(
                    tekes_ui_scene::ClipError::InputBudgetExceeded,
                ))?;
                if node.component.curve_subtree_capability() == CurveSubtreeCapability::Unsupported
                {
                    return Err(CompositionError::UnsupportedCurveChild(node.id));
                }
            }
            for child in node.component.children() {
                visit(child, in_curve, left)?;
            }
            Ok(())
        }
        visit(
            self,
            in_curve,
            &mut HostRevealBudget::default().max_path_nodes,
        )
    }
    pub(super) fn curve_ancestor(&self, id: ComponentId, in_curve: bool) -> Option<bool> {
        let in_curve = in_curve || self.component::<PanelContent>().is_some();
        if self.id == id {
            Some(in_curve)
        } else {
            self.component
                .children()
                .iter()
                .find_map(|c| c.curve_ancestor(id, in_curve))
        }
    }
    pub(super) fn curve_contains(&self, p: Point) -> bool {
        self.curve_geometry.as_ref().is_none_or(|g| {
            g.shapes
                .iter()
                .all(|s| s.contains([p.x as f64, p.y as f64]) == Ok(true))
        })
    }
    pub(super) fn curve_bounds(
        &self,
        rect: Rect,
    ) -> Result<Option<Rect>, tekes_ui_scene::ClipError> {
        match &self.curve_geometry {
            Some(g) => bounds(&g.shapes, rect),
            None => Ok(rect.intersection(&self.clip)),
        }
    }
    pub(super) fn push_curve_scope(&self, scene: &mut Scene) -> usize {
        let Some(g) = &self.curve_geometry else {
            return 0;
        };
        for key in &g.inherited_keys {
            scene.push_shape_clip(key.clone());
        }
        g.inherited_keys.len()
    }
    #[allow(clippy::too_many_arguments)] // One original arranged snapshot and its inherited chain.
    fn freeze_curve(
        &mut self,
        table: &mut ShapeTable,
        generation: u64,
        ancestors: &[ClipShape],
        keys: &[ShapeKey],
        in_curve: bool,
        external_key_count: usize,
        work: &mut GeometryWork,
    ) -> Result<(), CompositionError> {
        let is_panel = self.component::<PanelContent>().is_some();
        let enabled = in_curve || is_panel;
        let mut shapes = ancestors.to_vec();
        let mut child_keys = keys.to_vec();
        if enabled {
            let rect = ClipShape::normalize(self.clip, CornerRadii::default(), Point::default())
                .map_err(CompositionError::CurveGeometry)?;
            if !shapes.contains(&rect) {
                shapes.push(rect);
            }
            let rect_key = table
                .intern(rect)
                .map_err(CompositionError::CurveGeometry)?;
            if !child_keys.contains(&rect_key) {
                child_keys.push(rect_key);
            }
        }
        // External ancestor shapes constrain the child's original geometry and
        // input, but their AA coverage is deferred to the parent image
        // composite. Only child-owned curve scopes are baked into its frame.
        let inherited_keys = child_keys
            .iter()
            .skip(external_key_count.min(child_keys.len()))
            .cloned()
            .collect();
        if is_panel {
            let radius = tekes_ui_tokens::proposal::metric("radius", "radius-panel")
                .expect("Surface radius");
            let shape =
                ClipShape::normalize(self.frame, CornerRadii::uniform(radius), Point::default())
                    .map_err(CompositionError::CurveGeometry)?;
            child_keys.push(
                table
                    .intern(shape)
                    .map_err(CompositionError::CurveGeometry)?,
            );
            shapes.push(shape);
            let outer = self.frame;
            let allocation = self.component.children()[0].frame;
            self.component_mut::<PanelContent>().unwrap().geometry = Some(PanelGeometry {
                generation,
                outer,
                allocation,
                shape,
            });
        }
        self.curve_geometry = if enabled {
            Some(NodeGeometry {
                generation,
                shapes: shapes.clone(),
                inherited_keys,
            })
        } else {
            None
        };
        let region = if enabled {
            Some(ClipRegion::new(&shapes, policy()).map_err(CompositionError::CurveGeometry)?)
        } else {
            None
        };
        if let Some(region) = &region {
            work.reserve(region.reserved_work().saturating_add(shapes.len()))?;
        }
        self.component.set_clip_region(region);
        self.component
            .set_curve_geometry_context(enabled.then(|| CurveGeometryContext {
                generation,
                shapes: std::sync::Arc::from(shapes.clone()),
            }));
        // Precompute the query now: a numeric failure cannot become a published bbox.
        if enabled {
            let mut query_shapes = shapes.clone();
            query_shapes.push(
                ClipShape::normalize(self.frame, CornerRadii::default(), Point::default())
                    .map_err(CompositionError::CurveGeometry)?,
            );
            let query = ClipRegion::new(&query_shapes, policy())
                .map_err(CompositionError::CurveGeometry)?;
            work.reserve(query.reserved_work())?;
            query
                .visible_bounds()
                .map_err(CompositionError::CurveGeometry)?;
        }
        if let Some(children) = self.component.children_mut() {
            for child in children {
                child.freeze_curve(
                    table,
                    generation,
                    &shapes,
                    &child_keys,
                    enabled,
                    external_key_count,
                    work,
                )?;
            }
        }
        Ok(())
    }
}
impl ContentHost {
    fn active_curve_tree_is_current(root: &Content, generation: u64) -> bool {
        fn current(node: &Content, in_curve: bool, generation: u64) -> bool {
            if !node.active() {
                return true;
            }
            let panel = node.component::<PanelContent>();
            let curved = in_curve || panel.is_some();
            if curved {
                if node
                    .curve_geometry
                    .as_ref()
                    .is_none_or(|frozen| frozen.generation != generation)
                {
                    return false;
                }
                if panel.is_some_and(|panel| {
                    panel
                        .geometry
                        .as_ref()
                        .is_none_or(|geometry| geometry.generation != generation)
                }) {
                    return false;
                }
            }
            node.component
                .children()
                .iter()
                .all(|child| current(child, curved, generation))
        }
        current(root, false, generation)
    }

    pub(super) fn active_curve_geometry_is_current(&self) -> bool {
        let Ok((generation, _)) = self.reveal_clock.stamp() else {
            return false;
        };
        // Curve ancestry begins at the real host root. A modal or overlay scope
        // can be a non-Panel descendant whose frozen geometry still inherits
        // an ancestor Panel's curve.
        Self::active_curve_tree_is_current(&self.root, generation)
    }

    /// Current original route target, without delivering an event or authorizing
    /// a Rect bbox. Disabled controls retain their ordinary shielding geometry.
    pub fn pointer_hit_test(&self, point: Point) -> Option<ComponentId> {
        if self.last_layout.is_none()
            || self.reveal_clock.pending
            || !self.active_curve_geometry_is_current()
        {
            return None;
        }
        fn visit(n: &Content, p: Point) -> Option<ComponentId> {
            if !n.active() || !n.curve_contains(p) {
                return None;
            }
            if n.routes_pointer_children() {
                for child in n.component.children().iter().rev() {
                    if let Some(id) = visit(child, p) {
                        return Some(id);
                    }
                }
            }
            n.pointer_target_at(p).then_some(n.id)
        }
        visit(self.scope(), point)
    }

    /// Structural requirement, including before layout and after cache invalidation.
    pub fn requires_curve_geometry(&self) -> bool {
        self.root.has_curve_panel() || self.external_curve_chain.is_some()
    }

    pub(super) fn freeze_curve_geometry(&mut self) -> Result<(), CompositionError> {
        let generation = self
            .reveal_clock
            .stamp()
            .map_err(|_| CompositionError::CurveGeometry(tekes_ui_scene::ClipError::StaleShapeKey))?
            .0;
        fn contains_panel(n: &Content) -> bool {
            n.component::<PanelContent>().is_some()
                || n.component.children().iter().any(contains_panel)
        }
        let has_panel = contains_panel(&self.root);
        if !has_panel && self.curve_shapes.is_none() {
            return Ok(());
        }
        let mut table = ShapeTable::new(generation, 4096);
        let external_shapes = self
            .external_curve_chain
            .as_ref()
            .map(|(_, shapes)| shapes.as_ref())
            .unwrap_or(&[]);
        let mut external_keys = Vec::with_capacity(external_shapes.len());
        for shape in external_shapes {
            external_keys.push(
                table
                    .intern(*shape)
                    .map_err(CompositionError::CurveGeometry)?,
            );
        }
        self.root.freeze_curve(
            &mut table,
            generation,
            external_shapes,
            &external_keys,
            !external_shapes.is_empty(),
            external_keys.len(),
            &mut GeometryWork {
                remaining: HostRevealBudget::default().max_geometry_work,
            },
        )?;
        self.curve_shapes = (has_panel || !external_shapes.is_empty()).then_some(table);
        Ok(())
    }
    pub fn geometry_snapshot(&self) -> Option<ContentGeometrySnapshot> {
        if self.reveal_clock.pending
            || self.last_layout.is_none()
            || !self.active_curve_geometry_is_current()
        {
            return None;
        }
        fn collect(n: &Content, out: &mut Vec<GeometryEntry>) {
            if !n.active() || n.component.semantic_subtree_hidden() {
                return;
            }
            let shapes = n.curve_geometry.as_ref().map_or_else(
                || {
                    ClipShape::normalize(n.clip, CornerRadii::default(), Point::default())
                        .into_iter()
                        .collect()
                },
                |g| g.shapes.clone(),
            );
            if !n.component.semantic_hidden() {
                out.push(GeometryEntry {
                    id: n.id,
                    mount: n.mount,
                    part: None,
                    frame: n.component.semantic_frame(n.frame),
                    shapes: shapes.clone(),
                });
                for p in n.component.semantic_parts() {
                    out.push(GeometryEntry {
                        id: n.id,
                        mount: n.mount,
                        part: Some(p.index),
                        frame: p.frame,
                        shapes: shapes.clone(),
                    });
                }
            }
            if !n.component.semantics().role.is_action_owner() {
                for child in n.component.children() {
                    collect(child, out);
                }
            }
        }
        let mut entries = Vec::new();
        collect(self.scope(), &mut entries);
        Some(ContentGeometrySnapshot {
            generation: self.reveal_clock.stamp().ok()?.0,
            entries,
            lease: self.reveal_clock.geometry_lease.clone(),
            lease_generation: self
                .reveal_clock
                .geometry_lease
                .load(std::sync::atomic::Ordering::Acquire),
        })
    }
}

#[cfg(test)]
mod frozen_generation_tests {
    use super::Button;
    use super::*;
    use crate::{ButtonTextAlign, ButtonTextRenderer};

    fn frozen_shape() -> ClipShape {
        ClipShape::normalize(
            Rect::new(0., 0., 100., 100.),
            CornerRadii::uniform(10.),
            Point::default(),
        )
        .unwrap()
    }

    fn freeze_test_tree(node: &mut Content, generation: u64, shape: ClipShape) {
        node.curve_geometry = Some(NodeGeometry {
            generation,
            shapes: vec![shape],
            inherited_keys: Vec::new(),
        });
        if let Some(panel) = node.component_mut::<PanelContent>() {
            panel.geometry = Some(PanelGeometry {
                generation,
                outer: Rect::new(0., 0., 100., 100.),
                allocation: Rect::new(10., 10., 80., 80.),
                shape,
            });
        }
        if let Some(children) = node.component.children_mut() {
            for child in children {
                freeze_test_tree(child, generation, shape);
            }
        }
    }

    fn test_panel() -> Content {
        Content::new(
            ComponentId(9900),
            PanelContent::new(Content::new(
                ComponentId(9901),
                Button::new("generation test"),
            ))
            .unwrap(),
        )
    }

    struct TestText;
    impl ButtonTextRenderer for TestText {
        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new((text.chars().count() as f32 * 7.).min(max_width), 18.)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    fn rendered_button_host() -> (ContentHost, Scene, TestText) {
        let button = Content::new(ComponentId(9911), Button::new("press"));
        let root = Content::new(
            ComponentId(9910),
            PanelContent::new(Content::new(
                ComponentId(9912),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![button],
                },
            ))
            .unwrap(),
        );
        let mut host = ContentHost::new(root).unwrap();
        let mut scene = Scene::new();
        let mut text = TestText;
        host.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 140., 90.),
            Rect::new(0., 0., 140., 90.),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        )
        .unwrap();
        (host, scene, text)
    }

    fn rendered_switch_host() -> (ContentHost, Scene, TestText) {
        let switch = Content::new(ComponentId(9921), SwitchContent::new("switch", false));
        let root = Content::new(
            ComponentId(9920),
            PanelContent::new(Content::new(
                ComponentId(9922),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![switch],
                },
            ))
            .unwrap(),
        );
        let mut host = ContentHost::new(root).unwrap();
        let mut scene = Scene::new();
        let mut text = TestText;
        host.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 140., 90.),
            Rect::new(0., 0., 140., 90.),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        )
        .unwrap();
        (host, scene, text)
    }

    #[test]
    fn active_curve_path_requires_current_frozen_generation_on_every_node() {
        let shape = frozen_shape();
        let mut root = test_panel();
        freeze_test_tree(&mut root, 7, shape);
        assert!(ContentHost::active_curve_tree_is_current(&root, 7));

        root.component.children_mut().unwrap()[0]
            .curve_geometry
            .as_mut()
            .unwrap()
            .generation = 6;
        assert!(!ContentHost::active_curve_tree_is_current(&root, 7));
    }

    #[test]
    fn active_panel_path_requires_its_own_frozen_geometry() {
        let shape = frozen_shape();
        let mut root = test_panel();
        freeze_test_tree(&mut root, 7, shape);
        root.component_mut::<PanelContent>().unwrap().geometry = None;
        assert!(!ContentHost::active_curve_tree_is_current(&root, 7));
    }

    fn scoped_host() -> ContentHost {
        let scope_id = ComponentId(9901);
        let child_id = ComponentId(9903);
        let tree = Content::new(
            ComponentId(9900),
            PanelContent::new(Content::new(
                ComponentId(9901),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![Content::new(child_id, Button::new("scoped child"))],
                },
            ))
            .unwrap(),
        );
        let mut host = ContentHost::new(tree).unwrap();
        let shape = frozen_shape();
        freeze_test_tree(&mut host.root, 0, shape);
        fn set_rects(node: &mut Content) {
            node.frame = Rect::new(0., 0., 100., 100.);
            node.clip = Rect::new(0., 0., 100., 100.);
            if let Some(children) = node.component.children_mut() {
                for child in children {
                    set_rects(child);
                }
            }
        }
        set_rects(&mut host.root);
        host.modal = Some((scope_id, None));
        host.last_layout = Some((Rect::new(0., 0., 100., 100.), Rect::new(0., 0., 100., 100.)));
        assert!(host.root.find(child_id).is_some());
        host
    }

    #[test]
    fn non_panel_scope_keeps_panel_ancestor_curve_validation_for_snapshot_and_hit() {
        let host = scoped_host();
        let point = Point::new(50., 50.);
        assert_eq!(host.pointer_hit_test(point), Some(ComponentId(9903)));
        let snapshot = host.geometry_snapshot().expect("fully frozen scoped tree");
        assert_eq!(snapshot.hit_test(point).unwrap().0, ComponentId(9903));
    }

    #[test]
    fn non_panel_scope_and_descendants_reject_stale_or_missing_curve_cache() {
        for invalidate_scope_root in [true, false] {
            for stale in [true, false] {
                let mut host = scoped_host();
                let target_id = if invalidate_scope_root {
                    ComponentId(9901)
                } else {
                    ComponentId(9903)
                };
                let target = host.root.find_mut(target_id).unwrap();
                if stale {
                    target.curve_geometry.as_mut().unwrap().generation = 1;
                } else {
                    target.curve_geometry = None;
                }
                assert!(host.geometry_snapshot().is_none());
                assert!(host.pointer_hit_test(Point::new(50., 50.)).is_none());
            }
        }
    }

    #[test]
    fn panel_button_pointer_capture_and_activation_use_prepared_original_owner() {
        let (mut host, _scene, _text) = rendered_button_host();
        let frame = host.get(ComponentId(9911)).unwrap().frame();
        let point = frame.center();
        let down = host.event(&ContentEvent::PointerDown(point));
        assert!(down.consumed && down.capture_pointer);
        assert!(!down.activated);
        let up = host.event(&ContentEvent::PointerUp(point));
        assert!(up.activated);
        assert_eq!(
            host.get(ComponentId(9911))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            1
        );
    }

    #[test]
    fn panel_corner_rejects_pointer_down_without_running_button_reducer() {
        let (mut host, _scene, _text) = rendered_button_host();
        let response = host.event(&ContentEvent::PointerDown(Point::new(1., 1.)));
        assert!(!response.consumed);
        assert_eq!(
            host.get(ComponentId(9911))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
    }

    #[test]
    fn panel_button_accessibility_press_uses_same_prepared_owner_once() {
        let (mut host, _scene, _text) = rendered_button_host();
        let response =
            host.accessibility_action(ComponentId(9911), None, AccessibilityAction::Press);
        assert!(response.consumed && response.activated);
        assert_eq!(host.focused(), Some(ComponentId(9911)));
        assert_eq!(
            host.get(ComponentId(9911))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            1
        );
    }

    #[test]
    fn stale_panel_geometry_rejects_pointer_before_original_button_effect() {
        let (mut host, _scene, _text) = rendered_button_host();
        let frame = host.get(ComponentId(9911)).unwrap().frame();
        let point = frame.center();
        let button = host.get_mut(ComponentId(9911)).unwrap();
        button
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = true;
        let response = host.event(&ContentEvent::PointerDown(point));
        assert!(!response.consumed);
        assert_eq!(
            host.get(ComponentId(9911))
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
    }

    #[test]
    fn unsupported_prepared_switch_keeps_its_original_accessibility_reducer() {
        let (mut host, _scene, _text) = rendered_switch_host();
        let response =
            host.accessibility_action(ComponentId(9921), None, AccessibilityAction::Press);
        assert!(response.consumed && response.activated);
        assert!(
            host.get(ComponentId(9921))
                .unwrap()
                .component::<SwitchContent>()
                .unwrap()
                .model
                .checked
        );
    }

    #[test]
    fn unsupported_prepared_switch_keeps_its_original_pointer_capture_and_toggle() {
        let (mut host, _scene, _text) = rendered_switch_host();
        let point = host
            .get(ComponentId(9921))
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .view
            .layout()
            .frame()
            .center();
        let down = host.event(&ContentEvent::PointerDown(point));
        assert!(down.consumed && down.capture_pointer);
        let up = host.event(&ContentEvent::PointerUp(point));
        assert!(up.consumed && up.changed);
        assert!(
            host.get(ComponentId(9921))
                .unwrap()
                .component::<SwitchContent>()
                .unwrap()
                .model
                .checked
        );
    }
    #[test]
    fn captured_switch_uses_actual_curve_membership_without_rewriting_point_or_canceling_reentry() {
        for release_outside in [true, false] {
            let child = Content::new(ComponentId(9931), SwitchContent::new("switch", false))
                .preserving_focus();
            let mut panel = PanelContent::new(child).unwrap();
            panel.set_padding(Some(crate::Insets::default())).unwrap();
            let mut host = ContentHost::new(Content::new(ComponentId(9930), panel)).unwrap();
            let frame = Rect::new(0., 0., 100., 32.);
            host.render(
                &mut Scene::new(),
                &mut TestText,
                frame,
                frame,
                Palette::from(tekes_ui_tokens::Theme::LIGHT),
            )
            .unwrap();
            let toggle = host
                .get(ComponentId(9931))
                .unwrap()
                .component::<SwitchContent>()
                .unwrap()
                .view
                .layout()
                .frame();
            let center = toggle.center();
            let corner = Point::new(toggle.min_x() + 0.5, toggle.min_y() + 0.5);
            let shape = host
                .get(ComponentId(9930))
                .unwrap()
                .component::<PanelContent>()
                .unwrap()
                .geometry()
                .unwrap()
                .shape;
            assert!(
                toggle.contains(corner),
                "the real release point remains in the same rectangular toggle bbox"
            );
            assert_eq!(
                shape.contains([f64::from(corner.x), f64::from(corner.y)]),
                Ok(false)
            );
            assert!(host.event(&ContentEvent::PointerDown(center)).consumed);
            assert!(host.pointer_capture().is_some());
            assert!(host.event(&ContentEvent::PointerMove(corner)).changed);
            assert!(
                host.pointer_capture().is_some(),
                "leaving the exact region does not cancel the original gesture"
            );
            let release = if release_outside {
                corner
            } else {
                assert!(host.event(&ContentEvent::PointerMove(center)).changed);
                center
            };
            assert!(host.event(&ContentEvent::PointerUp(release)).consumed);
            assert!(host.pointer_capture().is_none());
            assert_eq!(
                host.get(ComponentId(9931))
                    .unwrap()
                    .component::<SwitchContent>()
                    .unwrap()
                    .model
                    .checked,
                !release_outside
            );
        }
    }

    #[test]
    fn ordinary_curve_local_switch_refuses_actual_focus_capture_and_stale_geometry_gates() {
        for gate in 0..3 {
            let (mut host, _scene, _text) = rendered_switch_host();
            host.set_action_events(true);
            let point = host
                .get(ComponentId(9921))
                .unwrap()
                .component::<SwitchContent>()
                .unwrap()
                .view
                .layout()
                .frame()
                .center();
            if gate == 0 {
                host.focus_epoch = u64::MAX;
            }
            if gate == 1 {
                host.capture_epoch = u64::MAX;
            }
            if gate == 2 {
                let _ = host.get_mut(ComponentId(9921));
            }
            let response = if gate == 0 {
                host.accessibility_action(ComponentId(9921), None, AccessibilityAction::Press)
            } else {
                host.event(&ContentEvent::PointerDown(point))
            };
            assert!(!response.consumed && !response.activated);
            assert!(
                !host
                    .get(ComponentId(9921))
                    .unwrap()
                    .component::<SwitchContent>()
                    .unwrap()
                    .model
                    .checked
            );
            assert!(host.focused().is_none());
            assert!(host.pointer_capture().is_none());
            assert!(host.drain_actions().is_empty());
        }
    }

    #[test]
    fn non_panel_input_scope_keeps_fixed_outer_curves_and_only_moves_its_inner_scroll() {
        const OUTER: ComponentId = ComponentId(9940);
        const PANEL: ComponentId = ComponentId(9941);
        const SCOPE: ComponentId = ComponentId(9942);
        const INNER: ComponentId = ComponentId(9943);
        const TARGET: ComponentId = ComponentId(9944);
        fn host(viewport: f32) -> ContentHost {
            let target_row = Content::new(
                ComponentId(9950),
                StackContent {
                    model: Stack {
                        axis: Axis::Horizontal,
                        gap: 0.,
                    },
                    flexible_child: Some(TARGET),
                    children: vec![
                        Content::new(ComponentId(9951), TextContent::new("pad")),
                        Content::new(TARGET, Button::new("target")),
                        Content::new(ComponentId(9952), TextContent::new("pad")),
                    ],
                },
            );
            let mut rows: Vec<_> = (9960..9968)
                .map(|id| Content::new(ComponentId(id), TextContent::new("row")))
                .collect();
            rows.push(target_row);
            rows.push(Content::new(ComponentId(9970), TextContent::new("tail")));
            let body = Content::new(
                ComponentId(9953),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: rows,
                },
            );
            let mut inner = ScrollContent::new(ScrollArea::default(), body);
            inner.viewport_height = Some(viewport);
            let scope = Content::new(
                SCOPE,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![Content::new(INNER, inner)],
                },
            );
            let mut panel = PanelContent::new(scope).unwrap();
            panel.set_padding(Some(crate::Insets::default())).unwrap();
            let mut outer = ScrollContent::new(ScrollArea::default(), Content::new(PANEL, panel));
            outer.viewport_height = Some(140.);
            let mut host = ContentHost::new(Content::new(OUTER, outer)).unwrap();
            let frame = Rect::new(0., 0., 140., 140.);
            host.render(
                &mut Scene::new(),
                &mut TestText,
                frame,
                frame,
                Palette::from(tekes_ui_tokens::Theme::LIGHT),
            )
            .unwrap();
            // Same real non-Panel input scope used by the original scope tests.
            // The full original tree, renderer, Panel and both Scroll owners stay mounted.
            host.modal = Some((SCOPE, None));
            host
        }
        let mut current = host(60.);
        let outer_before = current
            .get(OUTER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        // This case asserts the immediate committed offset. Default navigation
        // is animated; this renderer intentionally supplies no animation clock.
        let plan = current
            .plan_host_reveal(
                TARGET,
                crate::ScrollRevealOptions {
                    animated: false,
                    ..Default::default()
                },
                &[],
                HostRevealBudget::default(),
            )
            .unwrap();
        assert_eq!(
            plan.planned_offsets().map(|(id, _)| id).collect::<Vec<_>>(),
            vec![INNER]
        );
        let planned = plan
            .planned_offsets()
            .find(|(id, _)| *id == INNER)
            .unwrap()
            .1;
        let inner = current
            .get(INNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        let target_frame = current.get(TARGET).unwrap().frame();
        eprintln!(
            "SCOPED_INNER_REVEAL target={target_frame:?} viewport={:?} document={:?} limit={:?} offset={:?} planned={planned:?}",
            inner.viewport,
            inner.scroll.document_size(),
            inner.scroll.limit(),
            inner.scroll.offset
        );
        assert!(
            target_frame.min_y() > inner.viewport.max_y(),
            "the full original target is genuinely below this viewport"
        );
        assert!(inner.scroll.limit().y > 0.);
        assert!(
            planned.y > inner.scroll.offset.y,
            "the solver must plan real inner movement before commit"
        );
        assert!(!plan.requested_options().animated);
        assert!(current.commit_host_reveal(plan).unwrap().revealed);
        assert!(
            current
                .get(INNER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset
                .y
                > 0.
        );
        assert_eq!(
            current
                .get(OUTER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            outer_before
        );
        let frame = Rect::new(0., 0., 140., 140.);
        current
            .render(
                &mut Scene::new(),
                &mut TestText,
                frame,
                frame,
                Palette::from(tekes_ui_tokens::Theme::LIGHT),
            )
            .unwrap();
        assert!(current.focus(Some(TARGET), true));
        assert_eq!(current.focused(), Some(TARGET));
        assert_eq!(
            current
                .get(OUTER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            outer_before
        );

        let mut stale = host(60.);
        let plan = stale
            .plan_host_reveal(TARGET, Default::default(), &[], HostRevealBudget::default())
            .unwrap();
        let before = stale
            .get(INNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        stale
            .get_mut(PANEL)
            .unwrap()
            .component_mut::<PanelContent>()
            .unwrap()
            .set_canvas(true);
        assert_eq!(
            stale.commit_host_reveal(plan),
            Err(HostRevealError::StalePlan)
        );
        assert_eq!(
            stale
                .get(INNER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            before
        );
        assert_eq!(
            stale
                .get(OUTER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            outer_before
        );
        assert!(stale.focused().is_none());

        let mut impossible = host(8.);
        let before = impossible
            .get(INNER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        assert!(
            impossible
                .plan_host_reveal(TARGET, Default::default(), &[], HostRevealBudget::default())
                .is_err()
        );
        assert!(!impossible.focus(Some(TARGET), true));
        assert_eq!(
            impossible
                .get(INNER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            before
        );
        assert_eq!(
            impossible
                .get(OUTER)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .offset,
            outer_before
        );
        assert!(impossible.focused().is_none());

        let mut no_scroll = scoped_host();
        assert_eq!(
            no_scroll
                .plan_host_reveal(
                    ComponentId(9903),
                    Default::default(),
                    &[],
                    HostRevealBudget::default()
                )
                .unwrap_err(),
            HostRevealError::NoFeasibleRegion
        );
        assert!(
            !no_scroll.focus(Some(ComponentId(9903)), true),
            "an inherited curve must not become LogicalFocus merely because its Panel is outside the input scope"
        );
        assert!(no_scroll.focused().is_none());
    }
}
