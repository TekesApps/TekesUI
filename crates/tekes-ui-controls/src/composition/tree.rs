//! Public finite tree composition. Tree and TreeView remain the sole model and
//! interaction owners; row slots contain decorative Content only.
use super::*;
use crate::collections_view::{CollectionEvent, TreeView};
use crate::extended::{KeyedTreeNode, Tree, TreeDatasetKey, TreeNodeKey, TreeUpdateError};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct TreeContentNode {
    /// Caller-owned wrapper identity, independent of the entity key/display IDs.
    /// A continuously present key in one dataset must retain this ID.
    pub row_id: ComponentId,
    pub node: KeyedTreeNode,
    pub display: TreeDisplayUpdate,
}
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // Public snapshot consumes the original Content directly.
pub enum TreeDisplayUpdate {
    Keep,
    DefaultLabel,
    Replace(Content),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeIntentKind {
    SelectionChanged,
    Activated,
    ExpansionChanged { expanded: bool },
}
#[derive(Clone, Debug)]
pub struct TreeIntent {
    // Original Tree provenance, never a second identity registry or model.
    origin: std::sync::Arc<()>,
    pub dataset: TreeDatasetKey,
    pub key: TreeNodeKey,
    pub exposure: usize,
    pub model_revision: u64,
    pub activity_epoch: u64,
    pub kind: TreeIntentKind,
    pub selected_key: Option<TreeNodeKey>,
}
impl PartialEq for TreeIntent {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.origin, &other.origin)
            && self.dataset == other.dataset
            && self.key == other.key
            && self.exposure == other.exposure
            && self.model_revision == other.model_revision
            && self.activity_epoch == other.activity_epoch
            && self.kind == other.kind
            && self.selected_key == other.selected_key
    }
}
impl Eq for TreeIntent {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeContentError {
    Model(TreeUpdateError),
    Composition(CompositionError),
    InteractiveDisplay {
        key: TreeNodeKey,
        child: ComponentId,
    },
    HostTransactionRequired,
    TokenCapacity,
    InvalidStructure,
    InvalidTarget(ComponentId),
    RowIdentityChanged {
        key: TreeNodeKey,
        expected: ComponentId,
        actual: ComponentId,
    },
}
impl From<TreeUpdateError> for TreeContentError {
    fn from(e: TreeUpdateError) -> Self {
        Self::Model(e)
    }
}
impl From<CompositionError> for TreeContentError {
    fn from(e: CompositionError) -> Self {
        Self::Composition(e)
    }
}

#[derive(Debug)]
struct RowSlot {
    identity: std::sync::Arc<()>,
    display: Vec<Content>,
    disabled: bool,
}
impl Component for RowSlot {
    fn child_theme(&self, theme: ContentTheme) -> ContentTheme {
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(
            theme.palette,
            theme.surface(),
            "content",
            if self.disabled { "disabled" } else { "rest" },
        )
        .expect("validated content Surface");
        ContentTheme {
            foreground: Some(colors.text),
            icon_foreground: Some(colors.icon),
            ..theme
        }
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.display
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.display)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.display.first_mut().map_or_else(
            || MeasuredChild::new(Size::default()),
            |c| c.measure(text, a),
        )
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        if let Some(display) = self.display.first_mut() {
            display.arrange(text, f, c);
        }
    }
}
fn new_row(id: ComponentId) -> Content {
    let mut c = Content::new(
        id,
        RowSlot {
            identity: std::sync::Arc::new(()),
            display: Vec::new(),
            disabled: false,
        },
    );
    c.suppress_semantics();
    c
}
fn passive(content: &Content, key: TreeNodeKey) -> Result<(), TreeContentError> {
    let c = &*content.component;
    if c.interactive()
        || c.focusable()
        || c.text_input_snapshot().is_some()
        || c.text_surface().is_some()
        || c.overlay_scope().is_some()
        || c.overlay_open()
        || c.overlay_presented()
        || c.composing()
    {
        return Err(TreeContentError::InteractiveDisplay {
            key,
            child: content.id(),
        });
    }
    c.validate()?;
    for child in c.children() {
        passive(child, key)?;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug)]
struct Exposure {
    token: usize,
    incarnation: u64,
}
#[derive(Debug)]
struct Prepared {
    retained: BTreeMap<TreeNodeKey, usize>,
    nodes: Vec<TreeContentNode>,
    exposures: BTreeMap<TreeNodeKey, Exposure>,
    next: usize,
    ids: BTreeSet<ComponentId>,
}
type TreeMetricCacheKey = (u32, u32, u32, usize, u64, bool);
type TreeMetricCache = (TreeMetricCacheKey, Vec<(usize, tekes_ui_scene::Size, bool)>);

#[derive(Debug)]
pub struct TreeContent {
    name: String,
    model: Tree,
    view: TreeView,
    rows: Vec<Content>,
    // Declaration/instance bindings stay independent of the publicly mutable Vec.
    registered_rows: BTreeMap<TreeNodeKey, (ComponentId, std::sync::Arc<()>)>,
    exposures: BTreeMap<TreeNodeKey, Exposure>,
    next: usize,
    activity_epoch: u64,
    window_active: bool,
    mounted: bool,
    exhausted: bool,
    last_intent: Option<TreeIntent>,
    icon_error: Option<IconRenderError>,
    icon_images: Vec<IconImage>,
    metrics_cache: Option<TreeMetricCache>,
}
impl TreeContent {
    pub fn new(
        name: impl Into<String>,
        dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>,
    ) -> Result<Self, TreeContentError> {
        let mut tree = Self {
            name: name.into(),
            model: Tree::default(),
            view: TreeView::new(),
            rows: Vec::new(),
            registered_rows: BTreeMap::new(),
            exposures: BTreeMap::new(),
            next: 0,
            activity_epoch: 0,
            window_active: true,
            mounted: false,
            exhausted: false,
            last_intent: None,
            icon_error: None,
            icon_images: Vec::new(),
            metrics_cache: None,
        };
        let prepared = tree.prepare(dataset, nodes)?;
        tree.commit(dataset, prepared)?;
        Ok(tree)
    }
    pub fn replace_nodes(
        &mut self,
        dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>,
    ) -> Result<(), TreeContentError> {
        let p = self.prepare(dataset, nodes)?;
        if self.mounted && p.ids != self.collect_ids()? {
            return Err(TreeContentError::HostTransactionRequired);
        }
        self.commit(dataset, p)
    }
    pub fn with_compact(mut self, compact: bool) -> Self {
        self.view.set_compact(compact);
        self
    }
    pub fn set_compact(&mut self, compact: bool) {
        if self.view.compact() != compact {
            self.view.set_compact(compact);
            self.invalidate();
        }
    }
    pub fn compact(&self) -> bool {
        self.view.compact()
    }
    pub fn selected_key(&self) -> Option<TreeNodeKey> {
        self.model.selected_key()
    }
    pub fn select_key(&mut self, key: TreeNodeKey) -> bool {
        if self.validate_structure().is_err() {
            return false;
        }
        self.model.select_key(key)
    }
    pub fn set_expanded_key(&mut self, key: TreeNodeKey, expanded: bool) -> bool {
        self.last_intent = None;
        let Some(i) = self.model.node_index(key) else {
            return false;
        };
        if !self.available() || !self.geometry_actionable(i) {
            return false;
        }
        self.set_expanded_at_checked(i, expanded)
    }
    fn set_expanded_at_checked(&mut self, i: usize, expanded: bool) -> bool {
        if self.model.nodes()[i].expanded == expanded
            || !self.model.nodes().iter().any(|n| n.parent == Some(i))
        {
            return false;
        }
        let mut staged = self.model.clone();
        if !staged.toggle(i) {
            return false;
        }
        if self.plan_exposures(&staged).is_err() {
            return false;
        }
        let result = self.view.set_expanded(i, expanded, &mut self.model);
        let changed = matches!(result, CollectionEvent::Toggled(_));
        if changed {
            self.sync_exposures();
            self.invalidate();
        }
        changed
    }
    pub fn intent_is_current(&self, intent: &TreeIntent) -> bool {
        self.available()
            && std::sync::Arc::ptr_eq(&intent.origin, &self.model.model_identity())
            && intent.activity_epoch == self.activity_epoch
            && self.model.dataset_key() == Some(intent.dataset)
            && self
                .exposures
                .get(&intent.key)
                .is_some_and(|e| e.token == intent.exposure)
            && self
                .model
                .node_index(intent.key)
                .is_some_and(|i| !self.model.nodes()[i].disabled && self.model.is_node_visible(i))
    }
    fn available(&self) -> bool {
        self.mounted && self.window_active && !self.exhausted && self.validate_rows().is_ok()
    }
    fn validate_structure(&self) -> Result<(), TreeContentError> {
        let invalid = TreeContentError::InvalidStructure;
        if self.rows.len() != self.model.nodes().len()
            || self.registered_rows.len() != self.model.nodes().len()
        {
            return Err(invalid);
        }
        for (i, row) in self.rows.iter().enumerate() {
            let key = self
                .model
                .node_key(i)
                .ok_or(TreeContentError::InvalidStructure)?;
            let (id, identity) = self
                .registered_rows
                .get(&key)
                .ok_or(TreeContentError::InvalidStructure)?;
            let slot = row
                .component::<RowSlot>()
                .ok_or(TreeContentError::InvalidStructure)?;
            if row.id() != *id
                || !std::sync::Arc::ptr_eq(&slot.identity, identity)
                || slot.display.len() > 1
            {
                return Err(TreeContentError::InvalidStructure);
            }
        }
        Ok(())
    }
    fn collect_ids(&self) -> Result<BTreeSet<ComponentId>, TreeContentError> {
        let mut ids = BTreeSet::new();
        for row in &self.rows {
            row.ids(&mut ids)?;
        }
        Ok(ids)
    }
    fn validate_rows(&self) -> Result<(), TreeContentError> {
        self.validate_structure()?;
        for (i, row) in self.rows.iter().enumerate() {
            passive(row, self.model.node_key(i).expect("keyed row"))?;
        }
        self.collect_ids()?;
        Ok(())
    }
    fn prepare(
        &self,
        dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>,
    ) -> Result<Prepared, TreeContentError> {
        self.validate_structure()?;
        let mut target = self.model.clone();
        target.replace_keyed(dataset, nodes.iter().map(|n| n.node.clone()).collect())?;
        let (exposures, next) = self.plan_exposures(&target)?;
        let same = self.model.dataset_key() == Some(dataset);
        let mut retained = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for entry in &nodes {
            let key = entry.node.key;
            let old = if same {
                self.model.node_index(key).map(|i| (i, &self.rows[i]))
            } else {
                None
            };
            if let Some((index, row)) = old {
                if row.id() != entry.row_id {
                    return Err(TreeContentError::RowIdentityChanged {
                        key,
                        expected: row.id(),
                        actual: entry.row_id,
                    });
                }
                retained.insert(key, index);
            }
            if !ids.insert(entry.row_id) {
                return Err(CompositionError::DuplicateId(entry.row_id).into());
            }
            match &entry.display {
                TreeDisplayUpdate::Keep => {
                    if let Some((_, row)) = old {
                        passive(row, key)?;
                        for display in row.component::<RowSlot>().expect("row").display.iter() {
                            display.ids(&mut ids)?;
                        }
                    }
                }
                TreeDisplayUpdate::Replace(display) => {
                    passive(display, key)?;
                    display.ids(&mut ids)?;
                }
                TreeDisplayUpdate::DefaultLabel => {}
            }
        }
        Ok(Prepared {
            retained,
            nodes,
            exposures,
            next,
            ids,
        })
    }

    fn plan_exposures(
        &self,
        target: &Tree,
    ) -> Result<(BTreeMap<TreeNodeKey, Exposure>, usize), TreeContentError> {
        let mut next = self.next;
        let mut map = BTreeMap::new();
        for i in target.visible() {
            let key = target.node_key(i).expect("keyed target");
            let incarnation = target.incarnation(i).expect("keyed target incarnation");
            let old = self.exposures.get(&key).filter(|e| {
                self.model.dataset_key() == target.dataset_key() && e.incarnation == incarnation
            });
            let exposure = if let Some(old) = old {
                *old
            } else {
                next = next.checked_add(1).ok_or(TreeContentError::TokenCapacity)?;
                Exposure {
                    token: next,
                    incarnation,
                }
            };
            map.insert(key, exposure);
        }
        Ok((map, next))
    }
    fn commit(&mut self, dataset: TreeDatasetKey, p: Prepared) -> Result<(), TreeContentError> {
        // This is the unique mutation of the original model, after all rejectable
        // checks. Clone dry-run never becomes an authorization/model owner.
        self.model
            .replace_keyed(dataset, p.nodes.iter().map(|n| n.node.clone()).collect())?;
        let mut retained = std::mem::take(&mut self.rows)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        let mut registered = BTreeMap::new();
        for entry in p.nodes {
            let key = entry.node.key;
            let mut row = if let Some(&i) = p.retained.get(&entry.node.key) {
                retained[i].take().expect("retained row")
            } else {
                new_row(entry.row_id)
            };
            let is_new = !p.retained.contains_key(&entry.node.key);
            match entry.display {
                TreeDisplayUpdate::Keep => {}
                update => {
                    let slot = row.component_mut::<RowSlot>().expect("row");
                    for mut old in std::mem::take(&mut slot.display) {
                        if self.mounted && !is_new {
                            old.lifecycle(Lifecycle::Unmounted);
                        }
                    }
                    if let TreeDisplayUpdate::Replace(mut display) = update {
                        if self.mounted && !is_new {
                            display.lifecycle(Lifecycle::Mounted);
                            display.refresh_activity(self.window_active);
                        }
                        slot.display.push(display);
                    }
                }
            }
            if self.mounted && is_new {
                row.lifecycle(Lifecycle::Mounted);
                row.refresh_activity(self.window_active);
            }
            registered.insert(
                key,
                (
                    entry.row_id,
                    row.component::<RowSlot>()
                        .expect("validated row")
                        .identity
                        .clone(),
                ),
            );
            self.rows.push(row);
        }
        self.registered_rows = registered;
        for mut old in retained.into_iter().flatten() {
            if self.mounted {
                old.lifecycle(Lifecycle::Unmounted);
            }
        }
        self.exposures = p.exposures;
        self.next = p.next;
        self.update_active_validated();
        self.invalidate();
        Ok(())
    }
    fn update_active_validated(&mut self) {
        let visible = self.model.visible_flags();
        for (i, row) in self.rows.iter_mut().enumerate() {
            row.set_composition_active(visible[i]);
            row.component_mut::<RowSlot>().expect("row").disabled = self.model.nodes()[i].disabled;
        }
    }
    fn sync_exposures(&mut self) {
        if self.validate_structure().is_err() {
            return;
        }
        match self.plan_exposures(&self.model) {
            Ok((m, n)) => {
                self.exposures = m;
                self.next = n;
            }
            Err(_) => {
                self.exhausted = true;
                self.exposures.clear();
            }
        }
        self.update_active_validated();
    }
    fn invalidate(&mut self) {
        self.view.reconcile_model(&self.model);
        self.view.invalidate_content_layout();
        self.metrics_cache = None;
        self.icon_images.clear();
    }
    fn measure_key(
        &self,
        text: &mut dyn ButtonTextRenderer,
        width: f32,
        compact: bool,
    ) -> TreeMetricCacheKey {
        (
            width.to_bits(),
            text.resolved_font_size(proposal::TextRole::UiBody)
                .to_bits(),
            text.backing_scale().to_bits(),
            std::sync::Arc::as_ptr(&self.model.model_identity()) as usize,
            self.model.model_revision(),
            compact,
        )
    }
    fn compute_metrics(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        width: f32,
        height: f32,
        compact: bool,
    ) -> Vec<(usize, tekes_ui_scene::Size, bool)> {
        let rows = &mut self.rows;
        TreeView::measure_rows(
            &self.model,
            width,
            text,
            compact,
            |text, i, w, custom_width| {
                let custom = !rows[i]
                    .component::<RowSlot>()
                    .expect("row")
                    .display
                    .is_empty();
                let size = if custom {
                    rows[i].measure(text, Size::new(custom_width, height)).size
                } else {
                    text.measure(&self.model.nodes()[i].label, proposal::TextRole::UiBody, w)
                };
                (size, custom)
            },
        )
    }
    fn geometry_actionable(&self, index: usize) -> bool {
        self.view.layout_for(&self.model).is_some_and(|l| {
            l.items
                .iter()
                .any(|r| r.id == index && r.visible.is_some_and(|r| !r.is_empty()))
        })
    }
    fn intent(&mut self, key: TreeNodeKey, kind: TreeIntentKind) {
        self.last_intent = self.exposures.get(&key).map(|e| TreeIntent {
            origin: self.model.model_identity(),
            dataset: self.model.dataset_key().expect("keyed"),
            key,
            exposure: e.token,
            model_revision: self.model.model_revision(),
            activity_epoch: self.activity_epoch,
            kind,
            selected_key: self.selected_key(),
        });
    }
    fn expansion_budget(&self, event: &ContentEvent) -> bool {
        let candidate = match event {
            ContentEvent::PointerUp(point) => self.view.layout_for(&self.model).and_then(|l| {
                l.items
                    .iter()
                    .find(|r| r.visible.is_some_and(|r| r.contains(*point)))
                    .map(|r| r.id)
            }),
            ContentEvent::Key {
                key: ControlKey::Left | ControlKey::Right,
                down: true,
            } => self.view.focused_index(&self.model),
            _ => None,
        };
        let Some(i) = candidate else {
            return true;
        };
        let expanding = match event {
            ContentEvent::Key {
                key: ControlKey::Left,
                ..
            } => false,
            ContentEvent::Key {
                key: ControlKey::Right,
                ..
            } => true,
            _ => !self.model.nodes()[i].expanded,
        };
        if self.model.nodes()[i].expanded == expanding {
            return true;
        }
        let mut target = self.model.clone();
        !target.toggle(i) || self.plan_exposures(&target).is_ok()
    }
    fn response(&mut self, event: CollectionEvent, before: Option<TreeNodeKey>) -> ContentResponse {
        let mut changed = matches!(
            event,
            CollectionEvent::Changed | CollectionEvent::Toggled(_)
        );
        let target = match event {
            CollectionEvent::Activated(i) => self
                .model
                .node_key(i)
                .map(|k| (k, TreeIntentKind::Activated)),
            CollectionEvent::Toggled(i) => {
                self.sync_exposures();
                self.model.node_key(i).map(|k| {
                    (
                        k,
                        TreeIntentKind::ExpansionChanged {
                            expanded: self.model.nodes()[i].expanded,
                        },
                    )
                })
            }
            _ => {
                let selected = self.selected_key();
                changed |= selected != before;
                selected
                    .filter(|_| selected != before)
                    .map(|k| (k, TreeIntentKind::SelectionChanged))
            }
        };
        if let Some((key, kind)) = target {
            self.intent(key, kind);
        }
        ContentResponse {
            consumed: !matches!(event, CollectionEvent::Ignored),
            changed,
            activated: matches!(event, CollectionEvent::Activated(_)),
            ..Default::default()
        }
    }
}
impl Component for TreeContent {
    fn rectangular_input_geometry(&self) -> bool {
        // TreeView retains rectangular clipped rows; geometry_actionable validates the original row before mutation.
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        self.validate_rows().map_err(|e| match e {
            TreeContentError::Composition(e) => e,
            TreeContentError::InteractiveDisplay { child, .. } => {
                CompositionError::NestedInteractive {
                    action: child,
                    child,
                }
            }
            _ => CompositionError::InvalidSlotCount,
        })
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        if self.validate_structure().is_err() {
            return &[];
        }
        &self.rows
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        if self.validate_structure().is_err() {
            return None;
        }
        Some(&mut self.rows)
    }
    fn requires_icon_services(&self) -> bool {
        self.model.nodes().iter().any(|n| n.parent.is_some())
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.icon_error
    }
    fn action_kinds(&self, _: &ContentResponse) -> Vec<ContentActionKind> {
        if self.last_intent.is_some() {
            vec![ContentActionKind::Activate]
        } else {
            Vec::new()
        }
    }
    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.last_intent
            .clone()
            .map(ContentActionPayload::TreeIntent)
    }
    fn action_payload_current(&self, p: &ContentActionPayload) -> bool {
        match p {
            ContentActionPayload::TreeIntent(i) => self.intent_is_current(i),
            _ => true,
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        if self.window_active == active {
            return;
        }
        if !active {
            if let Some(epoch) = self.activity_epoch.checked_add(1) {
                self.activity_epoch = epoch;
            } else {
                self.exhausted = true;
            }
            self.invalidate();
        }
        self.window_active = active;
        self.view.set_window_active(active);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.last_intent = None;
        match event {
            Lifecycle::Mounted => {
                self.mounted = true;
                self.sync_exposures();
            }
            Lifecycle::Unmounted => {
                self.mounted = false;
                self.exposures.clear();
                self.view.blur();
                self.invalidate();
            }
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        if self.validate_rows().is_err() {
            self.invalidate();
            return MeasuredChild::new(Size::default());
        }
        let compact = self.view.compact();
        let key = self.measure_key(text, available.width, compact);
        let metrics = self.compute_metrics(text, available.width, available.height, compact);
        self.metrics_cache = Some((key, metrics.clone()));
        MeasuredChild::new(Size::new(
            available.width,
            TreeView::rows_height(text, available.width, &metrics, compact),
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if self.validate_rows().is_err() {
            self.invalidate();
            return;
        }
        let compact = self.view.compact();
        let key = self.measure_key(text, frame.size.width, compact);
        let metrics = if let Some((cached_key, cached)) = &self.metrics_cache
            && *cached_key == key
        {
            cached.clone()
        } else {
            let computed = self.compute_metrics(text, frame.size.width, frame.size.height, compact);
            self.metrics_cache = Some((key, computed.clone()));
            computed
        };
        self.view.arrange_rows(
            &mut TextAdapter(text),
            &self.model,
            frame,
            Some(clip),
            &metrics,
        );
        if let Some(layout) = self.view.content_rows(&self.model) {
            for row in layout {
                self.rows[row.id].arrange(
                    text,
                    row.content,
                    clip.intersection(&frame).unwrap_or_default(),
                );
            }
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.icon_error = None;
        if !self.available() {
            return;
        }
        let surface = if theme.surface_declared {
            theme.surface()
        } else {
            "base"
        };
        self.view.paint_rows(
            scene,
            &mut TextAdapter(text),
            &self.model,
            Some(clip),
            theme.palette,
            surface,
        );
        let backing = text.backing_scale();
        self.icon_images.clear();
        if self.requires_icon_services() {
            if let Some(service) = text.icon_renderer() {
                struct Icons<'a> {
                    service: &'a mut dyn crate::layout_view::IconRenderer,
                    images: &'a mut Vec<IconImage>,
                    error: &'a mut Option<IconRenderError>,
                    backing: f32,
                }
                impl crate::layout_view::IconRenderer for Icons<'_> {
                    fn draw_icon(
                        &mut self,
                        scene: &mut Scene,
                        name: &str,
                        frame: Rect,
                        tint: Rgba,
                    ) {
                        match self
                            .service
                            .prepare_icon(name, frame.size.width, self.backing, tint)
                        {
                            Ok(image) => {
                                scene.push_image(tekes_ui_scene::ImageQuad {
                                    rect: frame,
                                    image: image.id(),
                                    opacity: 1.,
                                });
                                self.images.push(image);
                            }
                            Err(e) => *self.error = Some(e),
                        }
                    }
                }
                self.view.render_icons_model(
                    scene,
                    &mut Icons {
                        service,
                        images: &mut self.icon_images,
                        error: &mut self.icon_error,
                        backing,
                    },
                    &mut self.model,
                    Some(clip),
                    theme.palette,
                    proposal::metric("size", "size-icon-standard").expect("icon size"),
                );
            } else {
                self.icon_error = Some(IconRenderError::UnsupportedRenderer);
            }
        }
    }
    fn interactive(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        self.available()
            && self
                .model
                .visible()
                .iter()
                .any(|i| !self.model.nodes()[*i].disabled)
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.last_intent = None;
        if !focused {
            self.view.blur();
            return;
        }
        if !self.available() {
            return;
        }
        if let Some(index) = self
            .view
            .focused_index(&self.model)
            .or_else(|| {
                self.model
                    .selected_key()
                    .and_then(|k| self.model.node_index(k))
            })
            .or_else(|| {
                self.model
                    .visible()
                    .into_iter()
                    .find(|i| !self.model.nodes()[*i].disabled)
            })
        {
            self.view.focus_model(index, visible, &mut self.model);
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Tree, &self.name);
        s.disabled = !self.window_active || self.exhausted || self.validate_structure().is_err();
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.available() {
            return Vec::new();
        }
        let Some(layout) = self.view.layout_for(&self.model) else {
            return Vec::new();
        };
        let nodes = self.model.nodes();
        let mut has_child = vec![false; nodes.len()];
        for n in nodes {
            if let Some(p) = n.parent
                && let Some(slot) = has_child.get_mut(p)
            {
                *slot = true;
            }
        }
        layout
            .items
            .iter()
            .filter_map(|row| {
                let key = self.model.node_key(row.id)?;
                let e = self.exposures.get(&key)?;
                let node = &nodes[row.id];
                let mut s =
                    ContentSemantics::new(ContentRole::TreeItem { level: row.depth }, &node.label);
                s.selected = self.selected_key() == Some(key);
                s.disabled = node.disabled;
                s.expanded = has_child[row.id].then_some(node.expanded);
                Some(SemanticPart {
                    index: e.token,
                    frame: row.visible.unwrap_or(Rect::new(
                        row.frame.min_x(),
                        row.frame.min_y(),
                        0.,
                        0.,
                    )),
                    focused: row.visible.is_some()
                        && self.view.focused_index(&self.model) == Some(row.id),
                    semantics: s,
                })
            })
            .collect()
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        if !self.available() {
            return None;
        }
        self.view.layout_for(&self.model)?;
        let (key, _) = self.exposures.iter().find(|(_, e)| e.token == part)?;
        let index = self.model.node_index(*key)?;
        let parent = self.model.nodes()[index].parent?;
        self.exposures
            .get(&self.model.node_key(parent)?)
            .map(|e| e.token)
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last_intent = None;
        if !self.available() {
            self.invalidate();
            return ContentResponse::default();
        }
        let Some((key, _)) = part.and_then(|p| self.exposures.iter().find(|(_, e)| e.token == p))
        else {
            return ContentResponse::default();
        };
        let key = *key;
        let Some(i) = self.model.node_index(key) else {
            return ContentResponse::default();
        };
        if self.model.nodes()[i].disabled || !self.geometry_actionable(i) {
            return ContentResponse::default();
        }
        let before = self.selected_key();
        match action {
            AccessibilityAction::Focus => {
                ContentResponse::consumed(self.view.focus_model(i, true, &mut self.model))
            }
            AccessibilityAction::Press => {
                self.model.select_key(key);
                self.view.focus_model(i, true, &mut self.model);
                self.response(CollectionEvent::Activated(i), before)
            }
            AccessibilityAction::SetExpanded(expanded) => {
                let changed = self.set_expanded_at_checked(i, *expanded);
                if changed {
                    self.intent(
                        key,
                        TreeIntentKind::ExpansionChanged {
                            expanded: *expanded,
                        },
                    );
                }
                if changed {
                    ContentResponse::consumed(true)
                } else {
                    ContentResponse::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_intent = None;
        if matches!(event, ContentEvent::Cancel) {
            self.view.cancel();
            return ContentResponse::default();
        }
        if !self.available() {
            self.invalidate();
            return ContentResponse::default();
        }
        if !self.expansion_budget(event) {
            return ContentResponse::default();
        }
        let before = self.selected_key();
        let result = match event {
            ContentEvent::PointerMove(p) => {
                return ContentResponse {
                    changed: self.view.pointer_move_model(*p, &mut self.model),
                    ..Default::default()
                };
            }
            ContentEvent::PointerDown(p) => self.view.pointer_down(*p, &mut self.model),
            ContentEvent::PointerUp(p) => self.view.pointer_up(*p, &mut self.model),
            ContentEvent::Key { key, down } => self.view.key(*key, *down, &mut self.model),
            _ => CollectionEvent::Ignored,
        };
        let mut response = self.response(result, before);
        response.capture_pointer =
            matches!(event, ContentEvent::PointerDown(_)) && response.consumed;
        response
    }
}
impl ContentHost {
    /// Validate the complete target, including retained display subtrees and
    /// every other mounted branch, before moving or unmounting any old Content.
    pub fn replace_tree_nodes(
        &mut self,
        id: ComponentId,
        dataset: TreeDatasetKey,
        nodes: Vec<TreeContentNode>,
    ) -> Result<(), TreeContentError> {
        if self.reading_transaction {
            return Err(CompositionError::ReadingTransactionInProgress.into());
        }
        let tree = self
            .root
            .find(id)
            .and_then(|c| c.component::<TreeContent>())
            .ok_or(TreeContentError::InvalidTarget(id))?;
        let prepared = tree.prepare(dataset, nodes)?;
        fn outside(
            c: &Content,
            target: ComponentId,
            ids: &mut BTreeSet<ComponentId>,
        ) -> Result<(), CompositionError> {
            if !ids.insert(c.id()) {
                return Err(CompositionError::DuplicateId(c.id()));
            }
            if c.id() != target {
                c.component.validate()?;
                for child in c.component.children() {
                    outside(child, target, ids)?;
                }
            }
            Ok(())
        }
        let mut ids = BTreeSet::new();
        outside(&self.root, id, &mut ids)?;
        for child in &prepared.ids {
            if !ids.insert(*child) {
                return Err(CompositionError::DuplicateId(*child).into());
            }
        }
        self.root
            .find_mut(id)
            .expect("preflight owner")
            .component_mut::<TreeContent>()
            .expect("tree")
            .commit(dataset, prepared)?;
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
mod tree_content_counters {
    use super::*;
    fn n(k: u64) -> TreeContentNode {
        TreeContentNode {
            row_id: ComponentId(100 + k),
            node: KeyedTreeNode {
                key: TreeNodeKey(k),
                parent: None,
                label: format!("node{k}"),
                disabled: false,
                initially_expanded: false,
            },
            display: TreeDisplayUpdate::Keep,
        }
    }
    #[test]
    fn capacity_error_preserves_every_existing_counter_mapping_and_slot() {
        let mut t = TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap();
        t.next = usize::MAX;
        let revision = t.model.model_revision();
        let token = t.exposures[&TreeNodeKey(1)].token;
        let mount = t.rows[0].mount;
        assert_eq!(
            t.replace_nodes(TreeDatasetKey(1), vec![n(1), n(2)]),
            Err(TreeContentError::TokenCapacity)
        );
        assert_eq!(t.next, usize::MAX);
        assert_eq!(t.model.model_revision(), revision);
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.rows[0].mount, mount);
        assert_eq!(t.exposures[&TreeNodeKey(1)].token, token);
        // A reorder/label-only update allocates no new exposure even at MAX.
        let mut update = n(1);
        update.node.label = "rename".into();
        t.replace_nodes(TreeDatasetKey(1), vec![update]).unwrap();
        assert_eq!(t.next, usize::MAX);
        assert_eq!(t.rows[0].mount, mount);
    }
    #[test]
    fn clear_dataset_aba_never_resets_counter_or_recovers_old_wrapper() {
        let mut t = TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap();
        let old = t.exposures[&TreeNodeKey(1)].token;
        let mount = t.rows[0].mount;
        t.replace_nodes(TreeDatasetKey(1), vec![]).unwrap();
        assert_eq!(t.next, old);
        t.replace_nodes(TreeDatasetKey(1), vec![n(1)]).unwrap();
        assert!(t.next > old);
        assert_ne!(t.rows[0].mount, mount);
        let second = t.next;
        t.replace_nodes(TreeDatasetKey(2), vec![n(1)]).unwrap();
        t.replace_nodes(TreeDatasetKey(1), vec![n(1)]).unwrap();
        assert!(t.next > second);
    }
    #[test]
    fn activity_epoch_overflow_is_permanently_closed_and_repeated_observation_is_noop() {
        let mut t = TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap();
        t.lifecycle(Lifecycle::Mounted);
        t.activity_epoch = u64::MAX;
        t.window_active_changed(true);
        assert!(!t.exhausted);
        t.window_active_changed(false);
        assert!(t.exhausted);
        t.window_active_changed(true);
        assert!(!t.available());
        assert!(t.semantic_parts().is_empty());
    }
    #[test]
    fn host_token_capacity_failure_preserves_capture_queue_and_original_geometry() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
                Size::new(w.min(80.), 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let id = ComponentId(1);
        let mut host = ContentHost::new(Content::new(
            id,
            TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap(),
        ))
        .unwrap();
        host.set_action_events(true);
        let frame = Rect::new(0., 0., 300., 200.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let parts = host.semantics()[0].parts.clone();
        let point = parts[0].frame.center();
        assert!(
            host.event(&ContentEvent::PointerDown(point))
                .capture_pointer
        );
        host.root.component_mut::<TreeContent>().unwrap().next = usize::MAX;
        let capture = host.pointer_capture();
        let queue = host.actions.as_ref().unwrap().len();
        let revision = host
            .root
            .component::<TreeContent>()
            .unwrap()
            .model
            .model_revision();
        assert_eq!(
            host.replace_tree_nodes(id, TreeDatasetKey(1), vec![n(1), n(2)]),
            Err(TreeContentError::TokenCapacity)
        );
        assert_eq!(host.pointer_capture(), capture);
        assert_eq!(host.actions.as_ref().unwrap().len(), queue);
        assert_eq!(
            host.semantics()[0].parts,
            parts
                .iter()
                .cloned()
                .map(|mut p| {
                    p.focused = true;
                    p.semantics.selected = true;
                    p
                })
                .collect::<Vec<_>>()
        );
        let tree = host.root.component::<TreeContent>().unwrap();
        assert_eq!(tree.next, usize::MAX);
        assert_eq!(tree.model.model_revision(), revision);
        assert_eq!(tree.selected_key(), Some(TreeNodeKey(1)));
        let records = host.drain_action_records();
        assert_eq!(records.len(), 1);
        assert!(host.event(&ContentEvent::PointerUp(point)).consumed);
        assert!(host.drain_action_records().is_empty());
    }
    #[test]
    fn malformed_display_slot_count_rejects_without_query_side_effects() {
        let mut tree = TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap();
        tree.lifecycle(Lifecycle::Mounted);
        let next = tree.next;
        let revision = tree.model.model_revision();
        let exposure = tree.exposures[&TreeNodeKey(1)].token;
        let slot = tree.rows[0].component_mut::<RowSlot>().unwrap();
        slot.display
            .push(Content::new(ComponentId(800), TextContent::new("first")));
        slot.display
            .push(Content::new(ComponentId(801), TextContent::new("extra")));
        for _ in 0..3 {
            assert_eq!(
                tree.validate_structure(),
                Err(TreeContentError::InvalidStructure)
            );
            assert!(tree.semantic_parts().is_empty());
            assert_eq!(tree.semantic_part_parent(exposure), None);
            assert!(tree.children().is_empty());
            assert_eq!(tree.next, next);
            assert_eq!(tree.model.model_revision(), revision);
        }
        assert!(tree.children_mut().is_none());
        assert_eq!(
            tree.replace_nodes(TreeDatasetKey(1), vec![n(1)]),
            Err(TreeContentError::InvalidStructure)
        );
        assert_eq!(tree.next, next);
        assert_eq!(tree.exposures[&TreeNodeKey(1)].token, exposure);
    }

    #[test]
    fn typed_tree_updates_revoke_host_geometry_only_after_success() {
        use std::sync::atomic::Ordering;
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
                Size::new(w.min(80.), 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        let id = ComponentId(1);
        let mut h = ContentHost::new(Content::new(
            id,
            TreeContent::new("Tree", TreeDatasetKey(1), vec![n(1)]).unwrap(),
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 300., 200.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let lease = h.reveal_clock.geometry_lease.clone();
        let old_lease = lease.load(Ordering::Acquire);
        h.replace_tree_nodes(id, TreeDatasetKey(1), vec![n(1)])
            .unwrap();
        assert_eq!(lease.load(Ordering::Acquire), old_lease + 1);
        assert!(h.reveal_clock.pending);
        assert!(h.dirty);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(!h.reveal_clock.pending);
        let before = format!("{h:?}");
        let current_lease = lease.load(Ordering::Acquire);
        assert!(
            h.replace_tree_nodes(id, TreeDatasetKey(1), vec![n(1), n(1)])
                .is_err()
        );
        assert_eq!(lease.load(Ordering::Acquire), current_lease);
        assert_eq!(format!("{h:?}"), before);
    }

    #[test]
    fn children_mut_corruption_still_rejects_replace_expand_validate_and_ax_actions() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
                Size::new(w.min(80.), 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        struct Icons;
        impl crate::layout_view::IconRenderer for Icons {
            fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
            fn prepare_icon(
                &mut self,
                _: &str,
                _: f32,
                _: f32,
                _: Rgba,
            ) -> Result<IconImage, IconRenderError> {
                Ok(IconImage::new(
                    tekes_ui_scene::ImageId(1),
                    std::rc::Rc::new(()),
                ))
            }
        }
        let parent = TreeContentNode {
            row_id: ComponentId(101),
            node: KeyedTreeNode {
                key: TreeNodeKey(1),
                parent: None,
                label: "root".into(),
                disabled: false,
                initially_expanded: true,
            },
            display: TreeDisplayUpdate::Keep,
        };
        let child = TreeContentNode {
            row_id: ComponentId(102),
            node: KeyedTreeNode {
                key: TreeNodeKey(2),
                parent: Some(TreeNodeKey(1)),
                label: "child".into(),
                disabled: false,
                initially_expanded: false,
            },
            display: TreeDisplayUpdate::Keep,
        };
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            TreeContent::new("Tree", TreeDatasetKey(1), vec![parent, child]).unwrap(),
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 300., 200.);
        host.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut Text, &mut Icons),
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let token = host.root.component::<TreeContent>().unwrap().exposures[&TreeNodeKey(1)].token;
        // Mutate rows vec via public Component::children_mut().
        host.root
            .component_mut::<TreeContent>()
            .unwrap()
            .children_mut()
            .unwrap()
            .pop();
        let tree = host.root.component_mut::<TreeContent>().unwrap();
        assert!(tree.validate().is_err());
        assert!(tree.semantic_parts().is_empty());
        assert!(!tree.set_expanded_key(TreeNodeKey(1), false));
        assert!(
            !tree
                .accessibility_action(Some(token), &AccessibilityAction::SetExpanded(false))
                .consumed
        );
        assert_eq!(
            tree.replace_nodes(TreeDatasetKey(1), vec![n(1)]),
            Err(TreeContentError::InvalidStructure)
        );
    }

    #[test]
    fn direct_set_expanded_and_ax_set_expanded_preserve_admission_intent_and_expanded_metadata() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
                Size::new(w.min(80.), 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        struct Icons;
        impl crate::layout_view::IconRenderer for Icons {
            fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
            fn prepare_icon(
                &mut self,
                _: &str,
                _: f32,
                _: f32,
                _: Rgba,
            ) -> Result<IconImage, IconRenderError> {
                Ok(IconImage::new(
                    tekes_ui_scene::ImageId(1),
                    std::rc::Rc::new(()),
                ))
            }
        }
        let make_nodes = || {
            vec![
                TreeContentNode {
                    row_id: ComponentId(101),
                    node: KeyedTreeNode {
                        key: TreeNodeKey(1),
                        parent: None,
                        label: "root".into(),
                        disabled: false,
                        initially_expanded: true,
                    },
                    display: TreeDisplayUpdate::Keep,
                },
                TreeContentNode {
                    row_id: ComponentId(102),
                    node: KeyedTreeNode {
                        key: TreeNodeKey(2),
                        parent: Some(TreeNodeKey(1)),
                        label: "mid".into(),
                        disabled: false,
                        initially_expanded: true,
                    },
                    display: TreeDisplayUpdate::Keep,
                },
                TreeContentNode {
                    row_id: ComponentId(103),
                    node: KeyedTreeNode {
                        key: TreeNodeKey(3),
                        parent: Some(TreeNodeKey(2)),
                        label: "leaf".into(),
                        disabled: false,
                        initially_expanded: false,
                    },
                    display: TreeDisplayUpdate::Keep,
                },
            ]
        };
        // Unmounted & unlaid-out tree refuses direct set_expanded_key.
        let mut unmounted = TreeContent::new("Unmounted", TreeDatasetKey(1), make_nodes()).unwrap();
        assert!(!unmounted.set_expanded_key(TreeNodeKey(1), false));

        let id = ComponentId(1);
        let mut host = ContentHost::new(Content::new(
            id,
            TreeContent::new("Tree", TreeDatasetKey(1), make_nodes()).unwrap(),
        ))
        .unwrap();
        // Mounted before first layout still refuses set_expanded_key.
        assert!(
            !host
                .root
                .component_mut::<TreeContent>()
                .unwrap()
                .set_expanded_key(TreeNodeKey(1), false)
        );
        let frame = Rect::new(0., 0., 300., 200.);
        host.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut Text, &mut Icons),
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();

        let tree = host.root.component::<TreeContent>().unwrap();
        let parts = tree.semantic_parts();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].semantics.expanded, Some(true));
        assert_eq!(parts[1].semantics.expanded, Some(true));
        assert_eq!(parts[2].semantics.expanded, None);

        let root_tok = parts[0].index;
        let leaf_tok = parts[2].index;
        // Leaf cannot expand; same state is no-op.
        let tc = host.root.component_mut::<TreeContent>().unwrap();
        assert!(!tc.set_expanded_key(TreeNodeKey(3), true));
        assert!(!tc.set_expanded_key(TreeNodeKey(1), true));

        // Press leaf to record leaf intent, then collapse root via AX SetExpanded(false).
        assert!(
            tc.accessibility_action(Some(leaf_tok), &AccessibilityAction::Press)
                .activated
        );
        let leaf_intent = tc.last_intent.clone().unwrap();
        assert!(tc.intent_is_current(&leaf_intent));

        let resp =
            tc.accessibility_action(Some(root_tok), &AccessibilityAction::SetExpanded(false));
        assert!(resp.changed);
        assert!(resp.consumed);
        let collapse_intent = tc.last_intent.clone().unwrap();
        assert_eq!(
            collapse_intent.kind,
            TreeIntentKind::ExpansionChanged { expanded: false }
        );
        assert!(tc.intent_is_current(&collapse_intent));
        // Previously visible leaf intent is now invalid after collapse.
        assert!(!tc.intent_is_current(&leaf_intent));

        // Render and verify direct set_expanded_key re-expands cleanly; inactive window blocks mutation.
        host.render(
            &mut Scene::new(),
            &mut WithIconRenderer::new(&mut Text, &mut Icons),
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let tc = host.root.component_mut::<TreeContent>().unwrap();
        assert!(tc.set_expanded_key(TreeNodeKey(1), true));
        tc.window_active_changed(false);
        assert!(!tc.set_expanded_key(TreeNodeKey(1), false));
        assert!(!tc.intent_is_current(&collapse_intent));
    }
}
