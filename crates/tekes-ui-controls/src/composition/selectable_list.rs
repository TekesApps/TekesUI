//! Finite passive rows over the original KeyedList and ListView.
use super::*;
use crate::collections_view::{CollectionEvent, ListView};
use crate::extended::{
    KeyedList, KeyedListSnapshot, ListBudget, ListDatasetKey as D, ListItemKey as K, ListStage,
    ListUpdateError,
};
use crate::{ReadingDocumentVersion, ReadingUpdateError};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ListDisplayUpdate {
    Keep,
    DefaultLabel,
    Replace(Content),
}
#[derive(Debug)]
pub struct ListRowDisplay {
    pub key: K,
    pub row_id: ComponentId,
    pub display: ListDisplayUpdate,
}
#[derive(Debug)]
pub struct ListReadingUpdate {
    pub owner: ComponentId,
    pub expected: ReadingDocumentVersion,
    pub revision: u64,
    pub items: Vec<ComponentId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListIntentKind {
    SelectionChanged,
    Activated,
}
#[derive(Clone, Debug)]
pub struct ListIntent {
    origin: Arc<()>,
    pub dataset: D,
    pub key: K,
    pub incarnation: u64,
    pub exposure: usize,
    pub activity_epoch: u64,
    pub model_revision: u64,
    pub selected_key: Option<K>,
    pub kind: ListIntentKind,
}
impl PartialEq for ListIntent {
    fn eq(&self, b: &Self) -> bool {
        Arc::ptr_eq(&self.origin, &b.origin)
            && self.dataset == b.dataset
            && self.key == b.key
            && self.incarnation == b.incarnation
            && self.exposure == b.exposure
            && self.activity_epoch == b.activity_epoch
            && self.model_revision == b.model_revision
            && self.selected_key == b.selected_key
            && self.kind == b.kind
    }
}
impl Eq for ListIntent {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListContentError {
    Model(ListUpdateError),
    Composition(CompositionError),
    Reading(ReadingUpdateError),
    InvalidStructure,
    InvalidDisplays,
    RowIdentityChanged {
        key: K,
    },
    InteractiveDisplay {
        key: K,
        child: ComponentId,
    },
    UnknownTransactionHooks {
        child: ComponentId,
    },
    BudgetExceeded {
        kind: &'static str,
        key: Option<K>,
        child: Option<ComponentId>,
    },
    Overflow,
    HostTransactionRequired,
    InvalidTarget(ComponentId),
    ReadingOwner,
    ReadingItems,
    TokenCapacity,
    MountCapacity,
}
impl From<ListUpdateError> for ListContentError {
    fn from(e: ListUpdateError) -> Self {
        Self::Model(e)
    }
}
impl From<CompositionError> for ListContentError {
    fn from(e: CompositionError) -> Self {
        Self::Composition(e)
    }
}
impl From<ReadingUpdateError> for ListContentError {
    fn from(e: ReadingUpdateError) -> Self {
        Self::Reading(e)
    }
}
fn add(a: usize, b: usize) -> Result<usize, ListContentError> {
    a.checked_add(b).ok_or(ListContentError::Overflow)
}
fn mul(a: usize, b: usize) -> Result<usize, ListContentError> {
    a.checked_mul(b).ok_or(ListContentError::Overflow)
}
#[derive(Debug)]
struct ListRowSlot {
    owner: Arc<()>,
    display: Option<ComponentId>,
    children: Vec<Content>,
    ink: Option<(Rgba, Rgba)>,
    arranged: bool,
}
impl Component for ListRowSlot {
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
            && self.display.is_some()
            && self.children.len() == 1
            && self.children.first().map(Content::id) == self.display
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn child_theme(&self, t: ContentTheme) -> ContentTheme {
        self.ink
            .map_or(t, |(foreground, icon_foreground)| ContentTheme {
                foreground: Some(foreground),
                icon_foreground: Some(icon_foreground),
                ..t
            })
    }
}
#[derive(Clone, Copy, Debug)]
struct Exposure {
    token: usize,
    incarnation: u64,
}
#[derive(Debug)]
struct Prepared {
    model: ListStage,
    view: ListView,
    exposures: BTreeMap<K, Exposure>,
    next: usize,
    ids: BTreeSet<ComponentId>,
    mounts: usize,
}
/// All limits are one-dimensional. max_staging_units also caps target nodes,
/// depth, parts and the conservative simultaneous structural staging peak.
/// max_utf8_bytes bounds retained and staged text (not process memory/OOM).
#[derive(Debug)]
pub struct KeyedSelectableListContent {
    name: String,
    model: KeyedList,
    view: ListView,
    children: Vec<Content>,
    registered: BTreeMap<K, (ComponentId, Arc<()>)>,
    exposures: BTreeMap<K, Exposure>,
    next: usize,
    mounted: bool,
    window_active: bool,
    disabled: bool,
    needs_exposure: bool,
    activity_epoch: u64,
    exhausted: bool,
    last_intent: Option<ListIntent>,
}
impl SelectableListContent {
    pub fn keyed(
        name: impl Into<String>,
        s: &KeyedListSnapshot,
        rows: &mut Vec<ListRowDisplay>,
        budget: ListBudget,
    ) -> Result<KeyedSelectableListContent, ListContentError> {
        KeyedSelectableListContent::new(name, s, rows, budget)
    }
}
impl KeyedSelectableListContent {
    /// Borrow through the entire preflight. Every Err retains all caller inputs,
    /// including Unknown trees whose Drop may have ordinary dynamic behavior.
    pub fn new(
        name: impl Into<String>,
        s: &KeyedListSnapshot,
        rows: &mut Vec<ListRowDisplay>,
        budget: ListBudget,
    ) -> Result<Self, ListContentError> {
        let mut c = Self {
            name: name.into(),
            model: KeyedList::new(
                KeyedListSnapshot {
                    dataset: s.dataset,
                    items: vec![],
                    display_items: vec![],
                },
                budget,
            )?,
            view: ListView::new(),
            children: vec![],
            registered: BTreeMap::new(),
            exposures: BTreeMap::new(),
            next: 0,
            mounted: false,
            window_active: true,
            disabled: false,
            needs_exposure: true,
            activity_epoch: 0,
            exhausted: false,
            last_intent: None,
        };
        let p = c.prepare(s, rows, (0, 0))?;
        let mut range = actions::reserve_mounts(p.mounts).ok_or(ListContentError::MountCapacity)?;
        c.commit(p, std::mem::take(rows), &mut range);
        Ok(c)
    }
    pub fn model(&self) -> &KeyedList {
        &self.model
    }
    pub fn selected_key(&self) -> Option<K> {
        self.validate_all().ok()?;
        self.model.selected_key()
    }
    pub fn focused_key(&self) -> Option<K> {
        self.available()
            .then(|| self.view.focused_keyed(&self.model))
            .flatten()
    }
    pub fn select_key(&mut self, k: K) -> Result<bool, ListContentError> {
        self.validate_all()?;
        Ok(self.model.select_key(k))
    }
    pub fn clear_selection(&mut self) -> Result<bool, ListContentError> {
        self.validate_all()?;
        Ok(self.model.clear_selection())
    }
    pub fn replace_keyed(
        &mut self,
        s: &KeyedListSnapshot,
        rows: &mut Vec<ListRowDisplay>,
    ) -> Result<(), ListContentError> {
        self.structure()?;
        if self.mounted {
            return Err(ListContentError::HostTransactionRequired);
        }
        let p = self.prepare(s, rows, (0, 0))?;
        let mut range = actions::reserve_mounts(p.mounts).ok_or(ListContentError::MountCapacity)?;
        self.commit(p, std::mem::take(rows), &mut range);
        Ok(())
    }
    fn structure(&self) -> Result<(), ListContentError> {
        if self.children.len() != self.model.original().count
            || self.registered.len() != self.children.len()
        {
            return Err(ListContentError::InvalidStructure);
        }
        for (item, c) in self.model.snapshot().items.iter().zip(&self.children) {
            let Some((id, owner)) = self.registered.get(&item.key) else {
                return Err(ListContentError::InvalidStructure);
            };
            let Some(slot) = c.component::<ListRowSlot>() else {
                return Err(ListContentError::InvalidStructure);
            };
            if c.id() != *id
                || !Arc::ptr_eq(owner, &slot.owner)
                || slot.children.len() != usize::from(slot.display.is_some())
                || slot.children.first().map(Content::id) != slot.display
            {
                return Err(ListContentError::InvalidStructure);
            }
        }
        Ok(())
    }
    fn passive(
        &self,
        c: &Content,
        key: K,
        validate: bool,
        depth: usize,
    ) -> Result<(usize, usize), ListContentError> {
        let limit = self.model.budget().max_staging_units;
        if depth > limit {
            return Err(ListContentError::BudgetExceeded {
                kind: "depth",
                key: Some(key),
                child: Some(c.id()),
            });
        }
        let p = &*c.component;
        if p.interactive()
            || p.focusable()
            || p.text_surface().is_some()
            || p.text_input_snapshot().is_some()
            || p.reading_scroll_source()
            || p.composing()
            || p.overlay_scope().is_some()
            || p.overlay_open()
            || p.overlay_presented()
        {
            return Err(ListContentError::InteractiveDisplay { key, child: c.id() });
        }
        if p.table_transaction_compatibility() != TableTransactionCompatibility::StableHooks {
            return Err(ListContentError::UnknownTransactionHooks { child: c.id() });
        }
        if validate {
            p.validate()?;
        }
        let sem = p.semantics();
        let mut units = 1;
        let mut bytes = add(
            add(sem.name.len(), sem.value.as_ref().map_or(0, String::len))?,
            sem.description.as_ref().map_or(0, String::len),
        )?;
        for child in p.children() {
            let (n, b) = self.passive(child, key, validate, add(depth, 1)?)?;
            units = add(units, n)?;
            bytes = add(bytes, b)?;
            if units > limit {
                return Err(ListContentError::BudgetExceeded {
                    kind: "nodes",
                    key: Some(key),
                    child: Some(child.id()),
                });
            }
        }
        Ok((units, bytes))
    }
    fn traversal(&self) -> Result<(), ListContentError> {
        self.structure()?;
        for (item, c) in self.model.snapshot().items.iter().zip(&self.children) {
            self.passive(c, item.key, false, 1)?;
        }
        Ok(())
    }
    fn validate_all(&self) -> Result<(), ListContentError> {
        self.structure()?;
        let mut ids = BTreeSet::new();
        for (item, c) in self.model.snapshot().items.iter().zip(&self.children) {
            self.passive(c, item.key, true, 1)?;
            c.ids(&mut ids)?;
        }
        Ok(())
    }
    fn old_display(&self, key: K) -> Option<&Content> {
        self.children
            .get(self.model.item_index(key)?)?
            .component::<ListRowSlot>()?
            .children
            .first()
    }
    fn target<'a>(&'a self, key: K, u: &'a ListDisplayUpdate, same: bool) -> Option<&'a Content> {
        match u {
            ListDisplayUpdate::Replace(c) => Some(c),
            ListDisplayUpdate::Keep if same => self.old_display(key),
            _ => None,
        }
    }
    fn limit(&self, units: usize, bytes: usize) -> Result<(), ListContentError> {
        let b = self.model.budget();
        if units > b.max_staging_units {
            return Err(ListContentError::BudgetExceeded {
                kind: "staging",
                key: None,
                child: None,
            });
        }
        if bytes > b.max_utf8_bytes {
            return Err(ListContentError::BudgetExceeded {
                kind: "utf8",
                key: None,
                child: None,
            });
        }
        Ok(())
    }
    fn plan_exposures(
        &self,
        s: &KeyedListSnapshot,
        inc: &[u64],
        reuse: bool,
    ) -> Result<(BTreeMap<K, Exposure>, usize), ListContentError> {
        let mut next = self.next;
        let mut map = BTreeMap::new();
        // All V capacity is checked before constructing the first replacement.
        let needed = s
            .items
            .iter()
            .zip(inc)
            .filter(|(i, n)| {
                s.display_items.contains(&i.key)
                    && (!reuse
                        || self
                            .exposures
                            .get(&i.key)
                            .is_none_or(|e| e.incarnation != **n))
            })
            .count();
        self.next
            .checked_add(needed)
            .ok_or(ListContentError::TokenCapacity)?;
        for (item, incarnation) in s.items.iter().zip(inc) {
            if !s.display_items.contains(&item.key) {
                continue;
            }
            let exposure = if reuse {
                self.exposures
                    .get(&item.key)
                    .filter(|e| e.incarnation == *incarnation)
                    .copied()
            } else {
                None
            };
            map.insert(
                item.key,
                exposure.unwrap_or_else(|| {
                    next += 1;
                    Exposure {
                        token: next,
                        incarnation: *incarnation,
                    }
                }),
            );
        }
        Ok((map, next))
    }
    fn prepare(
        &self,
        s: &KeyedListSnapshot,
        rows: &[ListRowDisplay],
        extra: (usize, usize),
    ) -> Result<Prepared, ListContentError> {
        self.structure()?;
        if self.exhausted {
            return Err(ListContentError::TokenCapacity);
        }
        if rows.len() != s.items.len() || rows.iter().zip(&s.items).any(|(a, b)| a.key != b.key) {
            return Err(ListContentError::InvalidDisplays);
        }
        let same = s.dataset == self.model.dataset_key();
        // Deterministic simultaneous structural peak, never Table R*C.
        let mut units = add(
            extra.0,
            add(
                mul(add(s.items.len(), self.children.len())?, 12)?,
                mul(s.display_items.len(), 5)?,
            )?,
        )?;
        let mut bytes = add(extra.1, self.name.len())?;
        for item in self.model.snapshot().items.iter().chain(&s.items) {
            bytes = add(bytes, mul(item.name.capacity(), 3)?)?;
        }
        for (item, c) in self.model.snapshot().items.iter().zip(&self.children) {
            let (n, b) = self.passive(c, item.key, false, 1)?;
            units = add(units, n)?;
            bytes = add(bytes, b)?;
        }
        let mut nodes = rows.len();
        for row in rows {
            if same
                && self
                    .registered
                    .get(&row.key)
                    .is_some_and(|(id, _)| *id != row.row_id)
            {
                return Err(ListContentError::RowIdentityChanged { key: row.key });
            }
            if let Some(c) = self.target(row.key, &row.display, same) {
                let (n, b) = self.passive(c, row.key, false, 2)?;
                nodes = add(nodes, n)?;
                units = add(units, mul(n, 5)?)?;
                bytes = add(bytes, mul(b, 2)?)?;
            }
        }
        units = add(units, mul(nodes, 2)?)?;
        self.limit(units, bytes)?;
        self.validate_all()?;
        let mut ids = BTreeSet::new();
        for row in rows {
            if !ids.insert(row.row_id) {
                return Err(CompositionError::DuplicateId(row.row_id).into());
            }
            if let Some(c) = self.target(row.key, &row.display, same) {
                self.passive(c, row.key, true, 2)?;
                c.ids(&mut ids)?;
            }
        }
        let model = self.model.prepare_list_replace(s.clone())?;
        let publish = self.mounted && self.window_active && !self.disabled;
        let (exposures, next) = if publish {
            self.plan_exposures(s, &model.incarnations, same && !self.needs_exposure)?
        } else {
            (BTreeMap::new(), self.next)
        };
        let mut mounts = 0;
        if self.mounted {
            for (item, c) in self.model.snapshot().items.iter().zip(&self.children) {
                let retained = same && rows.iter().any(|r| r.key == item.key);
                let kept = retained
                    && rows
                        .iter()
                        .any(|r| r.key == item.key && matches!(r.display, ListDisplayUpdate::Keep));
                if !retained {
                    mounts = add(mounts, 1)?;
                }
                if !kept
                    && let Some(d) = c
                        .component::<ListRowSlot>()
                        .and_then(|w| w.children.first())
                {
                    mounts = add(mounts, self.passive(d, item.key, false, 2)?.0)?;
                }
            }
        }
        for row in rows {
            let new = !(same && self.registered.contains_key(&row.key));
            if new {
                mounts = add(mounts, 1 + usize::from(self.mounted))?;
            }
            if self.mounted
                && let Some(c) = self.target(row.key, &row.display, same)
                && (new || matches!(row.display, ListDisplayUpdate::Replace(_)))
            {
                mounts = add(mounts, self.passive(c, row.key, false, 2)?.0)?;
            }
        }
        let view = self.view.prepare_list_update(&self.model, &model);
        Ok(Prepared {
            model,
            view,
            exposures,
            next,
            ids,
            mounts,
        })
    }
    fn commit(&mut self, p: Prepared, rows: Vec<ListRowDisplay>, range: &mut actions::MountRange) {
        let same = self.model.dataset_key() == p.model_dataset();
        let mut old: BTreeMap<K, Content> = self
            .model
            .snapshot()
            .items
            .iter()
            .map(|i| i.key)
            .zip(std::mem::take(&mut self.children))
            .collect();
        let mut discarded = Vec::new();
        self.registered.clear();
        for row in rows {
            let retained = if same { old.remove(&row.key) } else { None };
            let new = retained.is_none();
            let mut wrapper = retained.unwrap_or_else(|| {
                Content::new_with_mount(
                    row.row_id,
                    ListRowSlot {
                        owner: Arc::new(()),
                        display: None,
                        children: vec![],
                        ink: None,
                        arranged: false,
                    },
                    range.take(),
                )
            });
            let slot = wrapper
                .component_mut::<ListRowSlot>()
                .expect("preflight slot");
            let display = match row.display {
                ListDisplayUpdate::Keep if same => slot.children.pop(),
                ListDisplayUpdate::Replace(mut c) => {
                    c.suppress_semantics();
                    if self.mounted && !new {
                        c.lifecycle_reserved(Lifecycle::Mounted, range);
                        c.refresh_activity(self.window_active);
                    }
                    Some(c)
                }
                _ => None,
            };
            discarded.append(&mut slot.children);
            slot.display = display.as_ref().map(Content::id);
            slot.children = display.into_iter().collect();
            slot.ink = None;
            slot.arranged = false;
            self.registered
                .insert(row.key, (row.row_id, slot.owner.clone()));
            wrapper.suppress_semantics();
            if self.mounted && new {
                wrapper.lifecycle_reserved(Lifecycle::Mounted, range);
                wrapper.refresh_activity(self.window_active);
            }
            self.children.push(wrapper);
        }
        if self.mounted {
            for mut c in discarded {
                c.lifecycle_reserved(Lifecycle::Unmounted, range);
            }
            for mut c in old.into_values() {
                c.lifecycle_reserved(Lifecycle::Unmounted, range);
            }
        }
        self.model.commit_list_replace(p.model);
        self.exposures = p.exposures;
        self.next = p.next;
        self.needs_exposure = !(self.mounted && self.window_active && !self.disabled);
        self.view = p.view;
        self.last_intent = None;
        for (item, c) in self.model.snapshot().items.iter().zip(&mut self.children) {
            c.set_composition_active(self.model.display_ordinal(item.key).is_some());
        }
        debug_assert!(range.exhausted());
    }
    fn revoke(&mut self) {
        self.exposures.clear();
        self.needs_exposure = true;
        self.view.blur();
        self.view.invalidate_list_layout(&self.model);
        self.last_intent = None;
        if let Some(n) = self.activity_epoch.checked_add(1) {
            self.activity_epoch = n;
        } else {
            self.exhausted = true;
        }
    }
    /// Fallible full-V capacity reservation after mount/activity/enable. Failure
    /// leaves authorization withdrawn and never publishes a partial token map.
    pub fn prepare_exposure(&mut self) -> Result<(), ListContentError> {
        self.validate_all()?;
        if self.exhausted {
            return Err(ListContentError::TokenCapacity);
        }
        if !self.needs_exposure || !self.mounted || !self.window_active || self.disabled {
            return Ok(());
        }
        self.next
            .checked_add(self.model.display_indices().len())
            .ok_or(ListContentError::TokenCapacity)?;
        let inc = (0..self.model.original().count)
            .map(|i| self.model.item_incarnation(i).expect("validated model"))
            .collect::<Vec<_>>();
        let (map, next) = self.plan_exposures(self.model.snapshot(), &inc, false)?;
        self.exposures = map;
        self.next = next;
        self.needs_exposure = false;
        Ok(())
    }
    pub fn set_disabled(&mut self, value: bool) -> Result<bool, ListContentError> {
        self.validate_all()?;
        if self.disabled == value {
            return Ok(false);
        }
        self.activity_epoch
            .checked_add(1)
            .ok_or(ListContentError::TokenCapacity)?;
        self.revoke();
        self.disabled = value;
        self.view.set_disabled(value);
        Ok(true)
    }
    fn publishable(&self) -> bool {
        self.mounted
            && self.window_active
            && !self.exhausted
            && !self.needs_exposure
            && self.validate_all().is_ok()
    }
    fn available(&self) -> bool {
        self.publishable() && !self.disabled
    }
    pub fn intent_is_current(&self, i: &ListIntent) -> bool {
        self.available()
            && Arc::ptr_eq(&i.origin, &self.model.view_identity())
            && i.dataset == self.model.dataset_key()
            && i.activity_epoch == self.activity_epoch
            && self
                .exposures
                .get(&i.key)
                .is_some_and(|e| e.token == i.exposure && e.incarnation == i.incarnation)
    }
    fn emit(&mut self, key: K, kind: ListIntentKind) {
        self.last_intent = self.exposures.get(&key).map(|e| ListIntent {
            origin: self.model.view_identity(),
            dataset: self.model.dataset_key(),
            key,
            incarnation: e.incarnation,
            exposure: e.token,
            activity_epoch: self.activity_epoch,
            model_revision: self.model.revision(),
            selected_key: self.model.selected_key(),
            kind,
        });
    }
    fn response(&mut self, event: CollectionEvent, before: Option<K>) -> ContentResponse {
        if let CollectionEvent::Activated(i) = event {
            if let Some(k) = self.model.item_key(i) {
                self.emit(k, ListIntentKind::Activated);
            }
        } else if self.model.selected_key() != before
            && let Some(k) = self.model.selected_key()
        {
            self.emit(k, ListIntentKind::SelectionChanged);
        }
        ContentResponse {
            consumed: event != CollectionEvent::Ignored,
            changed: matches!(
                event,
                CollectionEvent::Changed | CollectionEvent::Activated(_)
            ),
            activated: matches!(event, CollectionEvent::Activated(_)),
            ..Default::default()
        }
    }
    fn measured(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> Vec<Option<Size>> {
        let w = ListView::content_width(&mut TextAdapter(text), a.width);
        self.children
            .iter_mut()
            .map(|c| {
                c.component_mut::<ListRowSlot>()
                    .and_then(|w| w.children.first_mut())
                    .map(|c| c.measure(text, Size::new(w, a.height)).size)
            })
            .collect()
    }
}
impl Prepared {
    fn model_dataset(&self) -> D {
        self.model.snapshot.dataset
    }
}
impl Component for KeyedSelectableListContent {
    fn host_reveal_passthrough(&self) -> bool {
        self.publishable()
            && self.validate_all().is_ok()
            && self.view.layout_keyed(&self.model).is_some()
    }
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        if !self.publishable() {
            return None;
        }
        self.view
            .layout_keyed(&self.model)
            .map(|layout| layout.frame)
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(
            self,
            request,
            Self::decide_action,
            Self::apply_action,
            |d| {
                (
                    d.response,
                    if d.response.consumed {
                        PreparedActionFocus::Owner
                    } else {
                        PreparedActionFocus::None
                    },
                )
            },
        )
    }

    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn interactive(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        self.validate_all()
            .map_err(|_| CompositionError::InvalidSlotCount)
    }
    fn children(&self) -> &[Content] {
        if self.traversal().is_ok() {
            &self.children
        } else {
            &[]
        }
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.traversal().is_ok().then_some(&mut self.children)
    }
    fn focusable(&self) -> bool {
        self.available() && !self.model.display_indices().is_empty()
    }
    fn window_active_changed(&mut self, active: bool) {
        if self.window_active != active {
            if !active {
                self.revoke();
            }
            self.window_active = active;
            self.view.set_window_active(active);
        }
    }
    // Clip-only presentation changes never revoke logical identity or focus.
    fn lifecycle(&mut self, event: Lifecycle) {
        self.revoke();
        self.mounted = event == Lifecycle::Mounted;
    }
    fn set_focus(&mut self, focused: bool, keyboard: bool) {
        if !focused {
            self.view.blur();
        } else if self.available() {
            if let Some(k) = self.view.focused_keyed(&self.model) {
                self.view.focus_item_keyed(k, keyboard, &self.model);
            } else {
                self.view.focus_keyed(keyboard, &self.model);
            }
        }
    }
    fn accessibility_focus_requested(&self) -> bool {
        true
    }
    fn action_kinds(&self, _: &ContentResponse) -> Vec<ContentActionKind> {
        if self.last_intent.is_some() {
            vec![ContentActionKind::Activate]
        } else {
            vec![]
        }
    }
    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.last_intent
            .clone()
            .map(ContentActionPayload::ListIntent)
    }
    fn action_payload_current(&self, p: &ContentActionPayload) -> bool {
        if let ContentActionPayload::ListIntent(i) = p {
            self.intent_is_current(i)
        } else {
            true
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        if self.validate_all().is_err() {
            return MeasuredChild::new(Size::default());
        }
        let sizes = self.measured(text, a);
        let labels = self
            .model
            .snapshot()
            .items
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>();
        MeasuredChild::new(ListView::measure_content_rows(
            &mut TextAdapter(text),
            &labels,
            self.model.display_indices(),
            a.width,
            &sizes,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        for child in &mut self.children {
            if let Some(slot) = child.component_mut::<ListRowSlot>() {
                slot.arranged = false;
            }
        }
        if self.prepare_exposure().is_err() {
            self.view.invalidate_list_layout(&self.model);
            return;
        }
        let sizes = self.measured(text, f.size);
        self.view
            .arrange_content_keyed(&mut TextAdapter(text), &self.model, f, Some(c), &sizes);
        let Some(layout) = self.view.layout_keyed(&self.model) else {
            return;
        };
        let layout = layout.clone();
        let clip = c.intersection(&f).unwrap_or_default();
        for row in &layout.items {
            let wrapper = &mut self.children[row.id];
            wrapper.frame = row.frame;
            wrapper.clip = clip;
            if let Some(slot) = wrapper.component_mut::<ListRowSlot>()
                && let Some(child) = slot.children.first_mut()
                && let Some(frame) = self.view.list_content_frame(row.id)
            {
                child.arrange(
                    text,
                    frame,
                    clip.intersection(&row.frame).unwrap_or_default(),
                );
                slot.arranged = true;
            }
        }
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        t: ContentTheme,
    ) {
        if !self.mounted || self.exhausted || self.validate_all().is_err() {
            return;
        }
        let surface = t.surface_declared.then_some(t.surface());
        self.view
            .paint_content_keyed(s, &mut TextAdapter(text), &self.model, t.palette, surface);
        for (i, c) in self.children.iter_mut().enumerate() {
            if let Some(w) = c.component_mut::<ListRowSlot>() {
                w.ink = self.view.list_row_ink(i, t.palette, surface);
            }
        }
    }
    fn semantic_hidden(&self) -> bool {
        self.validate_all().is_err()
            || self
                .view
                .layout_keyed(&self.model)
                .is_none_or(|l| l.visible.is_none())
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        if !self.available() {
            return vec![];
        }
        self.view
            .layout_keyed(&self.model)
            .map(|l| l.items.iter().filter_map(|r| r.visible).collect())
            .unwrap_or_default()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::List, &self.name);
        s.disabled = !self.available();
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.publishable() {
            return vec![];
        }
        let Some(l) = self.view.layout_keyed(&self.model) else {
            return vec![];
        };
        l.items
            .iter()
            .filter_map(|row| {
                let frame = row
                    .visible
                    .filter(|v| v.size.width > 0. && v.size.height > 0.)?;
                let item = self.model.snapshot().items.get(row.id)?;
                let e = self.exposures.get(&item.key)?;
                let mut semantics = ContentSemantics::new(ContentRole::ListItem, &item.name);
                semantics.selected = self.model.selected_key() == Some(item.key);
                semantics.disabled = self.disabled;
                Some(SemanticPart {
                    index: e.token,
                    frame,
                    focused: self.view.focused_keyed(&self.model) == Some(item.key),
                    semantics,
                })
            })
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last_intent = None;
        let Some(d) = self.decide_action(&action_preparation::OwnedActionRequest::Accessibility(
            part,
            action.clone(),
        )) else {
            return ContentResponse::default();
        };
        let r = d.response;
        self.apply_action(d);
        r
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.last_intent = None;
        if matches!(e, ContentEvent::PointerDown(_)) {
            let Some(d) =
                self.decide_action(&action_preparation::OwnedActionRequest::Pointer(e.clone()))
            else {
                return ContentResponse::default();
            };
            let r = d.response;
            self.apply_action(d);
            return r;
        }

        if matches!(e, ContentEvent::Cancel) {
            self.view.cancel();
            return ContentResponse::default();
        }
        if !self.available() {
            return ContentResponse::default();
        }
        let before = self.model.selected_key();
        let event = match e {
            ContentEvent::PointerMove(p) => {
                return ContentResponse {
                    changed: self.view.pointer_move_keyed(*p, &self.model),
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up_keyed(*p, &mut self.model),
            ContentEvent::Key { key, down } => self.view.key_keyed(*key, *down, &mut self.model),
            _ => CollectionEvent::Ignored,
        };
        let mut r = self.response(event, before);
        r.capture_pointer = matches!(e, ContentEvent::PointerDown(_)) && r.consumed;
        r
    }
}

use super::collection_transaction::*;
impl ContentHost {
    /// One synchronous typed update: the full target and reader are staged before
    /// borrowing the original disjoint fields, and neither commit can fail.
    /// All inputs are borrowed through staging. Any Err leaves snapshot, displays
    /// and reading unchanged; success alone takes displays. Caller destruction of
    /// a refused Unknown tree is outside this transaction and may run dynamic Drop.
    pub fn replace_keyed_list(
        &mut self,
        id: ComponentId,
        s: &KeyedListSnapshot,
        d: &mut Vec<ListRowDisplay>,
        reading: Option<&ListReadingUpdate>,
    ) -> Result<Option<ReadingDocumentVersion>, ListContentError> {
        if self.reading_transaction {
            return Err(CompositionError::ReadingTransactionInProgress.into());
        }
        let table_node = self
            .root
            .find(id)
            .ok_or(ListContentError::InvalidTarget(id))?;
        let table_mount = table_node.mount;
        let table = table_node
            .component::<KeyedSelectableListContent>()
            .ok_or(ListContentError::InvalidTarget(id))?;

        let outer = transaction_outside_count(&self.root, id)?;
        let mut extra = if let Some(r) = reading {
            let scroll = self
                .root
                .find(r.owner)
                .and_then(|owner| owner.component::<ScrollContent>())
                .ok_or(ListContentError::ReadingOwner)?;
            let state = scroll
                .reading
                .as_ref()
                .ok_or(ReadingUpdateError::WrongDocument)?;
            content_reading::reading_plan_cost(state, &r.expected, r.items.capacity())
                .ok_or(ListContentError::Overflow)?
        } else {
            (0, 0)
        };
        // Two Host ID sets plus two fixed-capacity index paths. The paths are
        // allocated only after prepare admits this conservative simultaneous peak.
        extra.0 = add(extra.0, mul(outer, 4)?)?;
        extra.1 = add(extra.1, mul(outer, std::mem::size_of::<ComponentId>())?)?;
        let p = table.prepare(s, d, extra)?;
        // Plan exact immutable index paths. A mutable DFS would visit unrelated
        // Unknown siblings even though only target ancestors were admitted.

        let mut table_path = Vec::with_capacity(outer);
        if !transaction_access_path(&self.root, id, &mut table_path)? {
            return Err(ListContentError::InvalidTarget(id));
        }
        let mut owner_path = Vec::with_capacity(outer);
        if let Some(r) = reading
            && (!transaction_access_path(&self.root, r.owner, &mut owner_path)?
                || !table_path.starts_with(&owner_path)
                || table_path.len() == owner_path.len())
        {
            return Err(ListContentError::ReadingOwner);
        }

        let mut ids = BTreeSet::new();
        transaction_outside(&self.root, id, &mut ids)?;
        for i in &p.ids {
            if !ids.insert(*i) {
                return Err(CompositionError::DuplicateId(*i).into());
            }
        }
        if let Some(r) = reading {
            let owner = self
                .root
                .find(r.owner)
                .ok_or(ListContentError::ReadingOwner)?;
            let owner_mount = owner.mount;
            let scroll = owner
                .component::<ScrollContent>()
                .ok_or(ListContentError::ReadingOwner)?;
            if !scroll.children.iter().any(|c| c.find(id).is_some()) {
                return Err(ListContentError::ReadingOwner);
            }
            let expected = s
                .display_items
                .iter()
                .filter_map(|key| d.iter().find(|row| row.key == *key).map(|row| row.row_id));
            let is_row = |item: &ComponentId| d.iter().any(|row| row.row_id == *item);
            if expected.ne(r.items.iter().copied().filter(is_row)) {
                return Err(ListContentError::ReadingItems);
            }
            // Other declared items belong to unchanged branches of this same
            // reader. Preserve their order; list decorations cannot masquerade
            // as row items, and removed old subtree IDs cannot enter the plan.
            let outside = |item: &ComponentId| !is_row(item);
            let old_outside = scroll
                .reading_items()
                .iter()
                .copied()
                .filter(|item| table_node.find(*item).is_none());
            if old_outside.ne(r.items.iter().copied().filter(outside))
                || r.items.iter().any(|item| {
                    !is_row(item)
                        && (table_node.find(*item).is_some()
                            || !scroll.children.iter().any(|c| c.find(*item).is_some()))
                })
            {
                return Err(ListContentError::ReadingItems);
            }
            let state = scroll
                .reading
                .as_ref()
                .ok_or(ReadingUpdateError::WrongDocument)?;
            let reader = content_reading::prepare_reading_advance(
                state,
                &scroll.scroll,
                &r.expected,
                r.revision,
                r.items.clone(),
            )?;
            // Obtain BOTH original mutable references and revalidate identities
            // before the first mutation. No lookup/Result after table.commit.
            let owner = transaction_at_path_mut(&mut self.root, &owner_path)
                .ok_or(ListContentError::ReadingOwner)?;
            if owner.mount != owner_mount {
                return Err(ListContentError::ReadingOwner);
            }
            let scroll = owner
                .component_mut::<ScrollContent>()
                .ok_or(ListContentError::ReadingOwner)?;
            let ScrollContent {
                children, reading, ..
            } = scroll;
            let state = reading.as_mut().ok_or(ReadingUpdateError::WrongDocument)?;
            // The owner is a proven strict ancestor. Walk only the selected
            // child and suffix, never a sibling's mutable getter.
            let (index, suffix) = table_path[owner_path.len()..]
                .split_first()
                .ok_or(ListContentError::ReadingOwner)?;
            let node = children
                .get_mut(*index)
                .and_then(|child| transaction_at_path_mut(child, suffix))
                .ok_or(ListContentError::InvalidTarget(id))?;
            if node.mount != table_mount {
                return Err(ListContentError::InvalidTarget(id));
            }
            let table = node
                .component_mut::<KeyedSelectableListContent>()
                .ok_or(ListContentError::InvalidTarget(id))?;
            content_reading::validate_reading_current(state, &r.expected)?;
            let mut range =
                actions::reserve_mounts(p.mounts).ok_or(ListContentError::MountCapacity)?;
            table.commit(p, std::mem::take(d), &mut range);
            let result = reader.commit(state);
            self.reveal_clock.invalidate_input();
            self.reveal_clock.pending = true;
            self.dirty = true;
            Ok(Some(result))
        } else {
            // A list inside a declared reading document cannot silently change
            // ordered row identities without the typed reading update.

            if transaction_declared(&self.root, id) {
                return Err(ListContentError::HostTransactionRequired);
            }
            let node = transaction_at_path_mut(&mut self.root, &table_path)
                .ok_or(ListContentError::InvalidTarget(id))?;
            if node.mount != table_mount {
                return Err(ListContentError::InvalidTarget(id));
            }
            let table = node
                .component_mut::<KeyedSelectableListContent>()
                .ok_or(ListContentError::InvalidTarget(id))?;
            let mut range =
                actions::reserve_mounts(p.mounts).ok_or(ListContentError::MountCapacity)?;
            table.commit(p, std::mem::take(d), &mut range);
            self.reveal_clock.invalidate_input();
            self.reveal_clock.pending = true;
            self.dirty = true;
            Ok(None)
        }
    }
}

impl From<super::collection_transaction::CollectionTransactionError> for ListContentError {
    fn from(e: super::collection_transaction::CollectionTransactionError) -> Self {
        use super::collection_transaction::CollectionTransactionError as E;
        match e {
            E::Composition(e) => Self::Composition(e),
            E::UnknownTransactionHooks { child } => Self::UnknownTransactionHooks { child },
            E::Overflow => Self::Overflow,
        }
    }
}

#[derive(Debug, PartialEq)]
struct PreparedOwnerAction {
    identity: action_preparation::OwnerIdentity,
    revision: u64,
    activity: u64,
    snapshot: crate::collections_view::ListActionSnapshot,
    action: Option<crate::collections_view::CollectionAction>,
    response: ContentResponse,
    before: Option<K>,
}
impl KeyedSelectableListContent {
    fn decide_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedOwnerAction> {
        if !self.available() {
            return None;
        }
        let action = match request {
            action_preparation::OwnedActionRequest::Pointer(e) => {
                let routed = e.routing_event();
                let ContentEvent::PointerDown(point) = routed.as_ref() else {
                    return None;
                };
                self.view.prepare_list_down(*point, &self.model)
            }
            action_preparation::OwnedActionRequest::Accessibility(part, action) => {
                if !matches!(
                    action,
                    AccessibilityAction::Focus | AccessibilityAction::Press
                ) {
                    return None;
                }
                if self.semantic_parts().iter().any(|p| Some(p.index) == *part) {
                    self.exposures
                        .iter()
                        .find(|(_, e)| Some(e.token) == *part)
                        .and_then(|(k, _)| {
                            self.view.prepare_list_part(
                                *k,
                                matches!(action, AccessibilityAction::Press),
                                &self.model,
                            )
                        })
                } else {
                    None
                }
            }
        };
        let event = action
            .as_ref()
            .map_or(CollectionEvent::Ignored, |a| a.event);
        let pointer = matches!(request, action_preparation::OwnedActionRequest::Pointer(_));
        let response = action_preparation::collection_response(
            event,
            pointer && event != CollectionEvent::Ignored,
        );
        Some(PreparedOwnerAction {
            identity: action_preparation::OwnerIdentity(self.model.view_identity()),
            revision: self.model.revision(),
            activity: self.activity_epoch,
            snapshot: self.view.list_action_snapshot(),
            action,
            response,
            before: self.model.selected_key(),
        })
    }
    fn apply_action(&mut self, d: PreparedOwnerAction) {
        self.last_intent = None;
        if let Some(action) = d.action {
            let event = action.event;
            self.model.apply_prepared_action(&mut self.view, action);
            self.response(event, d.before);
        }
    }
}

impl action_preparation::OriginalActionDecision for PreparedOwnerAction {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extended::KeyedListItem;
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new(
                (s.len() as f32 * 8.).min(w),
                if s.is_empty() {
                    0.
                } else {
                    20. + 20. * s.matches('\n').count() as f32
                },
            )
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
    fn s() -> KeyedListSnapshot {
        KeyedListSnapshot {
            dataset: D(1),
            items: (1..=3)
                .map(|k| KeyedListItem {
                    key: K(k),
                    name: format!("row{k}"),
                })
                .collect(),
            display_items: (1..=3).map(K).collect(),
        }
    }
    fn b() -> ListBudget {
        ListBudget {
            max_items: 20,
            max_display_items: 20,
            max_utf8_bytes: 100000,
            max_staging_units: 10000,
        }
    }
    fn d(s: &KeyedListSnapshot) -> Vec<ListRowDisplay> {
        s.items
            .iter()
            .map(|r| ListRowDisplay {
                key: r.key,
                row_id: ComponentId(100 + r.key.0),
                display: ListDisplayUpdate::Keep,
            })
            .collect()
    }
    fn list() -> KeyedSelectableListContent {
        KeyedSelectableListContent::new("list", &s(), &mut d(&s()), b()).unwrap()
    }
    fn f() -> Rect {
        Rect::new(0., 0., 200., 300.)
    }
    fn mounted() -> KeyedSelectableListContent {
        let mut c = list();
        c.lifecycle(Lifecycle::Mounted);
        c.arrange(&mut Text, f(), f());
        c
    }
    fn host(reading: bool) -> ContentHost {
        let mut h = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(
                crate::ScrollArea::default(),
                Content::new(ComponentId(10), list()),
            ),
        ))
        .unwrap();
        if reading {
            h.set_scroll_reading_document(
                ComponentId(1),
                "rows",
                1,
                vec![ComponentId(101), ComponentId(102), ComponentId(103)],
                |_| (),
            )
            .unwrap();
        }
        paint(&mut h);
        h.set_action_events(true);
        h
    }
    fn paint(h: &mut ContentHost) {
        h.render(
            &mut Scene::new(),
            &mut Text,
            f(),
            f(),
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn list_ref(h: &ContentHost) -> &KeyedSelectableListContent {
        h.get(ComponentId(10))
            .unwrap()
            .component::<KeyedSelectableListContent>()
            .unwrap()
    }
    fn read(h: &ContentHost, s: &KeyedListSnapshot) -> ListReadingUpdate {
        let scroll = h
            .get(ComponentId(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        let expected = scroll.reading_document_version().unwrap().clone();
        ListReadingUpdate {
            owner: ComponentId(1),
            revision: expected.revision() + 1,
            expected,
            items: s
                .display_items
                .iter()
                .map(|k| ComponentId(100 + k.0))
                .collect(),
        }
    }
    #[test]
    fn wrapper_identity_all_variants_and_hidden_reappearance_are_atomic() {
        for variant in 0..3 {
            for hidden in [false, true] {
                let mut c = list();
                let mut snap = s();
                if hidden {
                    snap.display_items.clear();
                    c.replace_keyed(&snap, &mut d(&snap)).unwrap();
                    snap = s();
                }
                let mut rows = d(&snap);
                rows[0].row_id = ComponentId(900);
                rows[0].display = match variant {
                    0 => ListDisplayUpdate::Keep,
                    1 => ListDisplayUpdate::DefaultLabel,
                    _ => ListDisplayUpdate::Replace(Content::new(
                        ComponentId(200),
                        TextContent::new("text"),
                    )),
                };
                let before = format!("{c:?}");
                let input = format!("{rows:?}");
                assert_eq!(
                    c.replace_keyed(&snap, &mut rows),
                    Err(ListContentError::RowIdentityChanged { key: K(1) })
                );
                assert_eq!(format!("{c:?}"), before);
                assert_eq!(format!("{rows:?}"), input);
            }
        }
    }
    #[test]
    fn all_vec_and_owner_attacks_fail_closed_without_query_mutation() {
        for case in 0..8 {
            let mut c = mounted();
            match case {
                0 => c.children.clear(),
                1 => {
                    c.children.pop();
                }
                2 => c.children.swap(0, 1),
                3 => c
                    .children
                    .push(Content::new(ComponentId(900), TextContent::new("extra"))),
                4 => c.children[0] = Content::new(ComponentId(101), TextContent::new("wrong type")),
                5 => c.children[0] = list().children.remove(0),
                6 => c.children[0]
                    .component_mut::<ListRowSlot>()
                    .unwrap()
                    .children
                    .push(Content::new(
                        ComponentId(200),
                        TextContent::new("extra display"),
                    )),
                _ => {
                    c.children[0]
                        .component_mut::<ListRowSlot>()
                        .unwrap()
                        .display = Some(ComponentId(500))
                }
            }
            let before = format!("{c:?}");
            assert_eq!(c.structure(), Err(ListContentError::InvalidStructure));
            for _ in 0..3 {
                assert!(c.children().is_empty());
                assert!(c.children_mut().is_none());
                assert!(c.semantic_parts().is_empty());
                assert_eq!(c.selected_key(), None);
                assert_eq!(c.focused_key(), None);
                assert!(!c.focusable());
                assert!(c.validate().is_err());
            }
            let mut rows = d(&s());
            assert_eq!(
                c.replace_keyed(&s(), &mut rows),
                Err(ListContentError::InvalidStructure)
            );
            assert_eq!(format!("{c:?}"), before);
            assert!(
                !c.event(&ContentEvent::PointerDown(Point::new(10., 10.)))
                    .consumed
            );
            assert!(
                !c.accessibility_action(Some(1), &AccessibilityAction::Press)
                    .consumed
            );
            assert_eq!(format!("{c:?}"), before);
            c.mounted = false;
            assert_eq!(
                c.replace_keyed(&s(), &mut rows),
                Err(ListContentError::InvalidStructure)
            );
        }
    }
    #[test]
    fn keep_and_replace_preserve_row_marker_mount_and_original_model_arc() {
        let mut h = host(false);
        let id = list_ref(&h).model.view_identity();
        let marker = list_ref(&h).registered[&K(1)].1.clone();
        let mount = h.get(ComponentId(101)).unwrap().mount;
        for variant in 0..3 {
            let mut snap = s();
            snap.items.swap(0, 2);
            snap.display_items.reverse();
            let mut rows = d(&snap);
            let r = rows.iter_mut().find(|r| r.key == K(1)).unwrap();
            r.display = match variant {
                0 => ListDisplayUpdate::Replace(Content::new(
                    ComponentId(200),
                    TextContent::new("decorative"),
                )),
                1 => ListDisplayUpdate::Keep,
                _ => ListDisplayUpdate::DefaultLabel,
            };
            h.replace_keyed_list(ComponentId(10), &snap, &mut rows, None)
                .unwrap();
            assert!(rows.is_empty());
            assert!(Arc::ptr_eq(&id, &list_ref(&h).model.view_identity()));
            assert!(Arc::ptr_eq(&marker, &list_ref(&h).registered[&K(1)].1));
            assert_eq!(h.get(ComponentId(101)).unwrap().mount, mount);
        }
    }
    #[test]
    fn passive_capability_diagnostics_and_unknown_inputs_are_borrowed() {
        #[derive(Debug)]
        struct Unknown;
        impl Component for Unknown {
            fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
                MeasuredChild::new(Size::default())
            }
        }
        #[derive(Debug)]
        struct Capability(usize);
        impl Component for Capability {
            fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
                TableTransactionCompatibility::StableHooks
            }
            fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
                MeasuredChild::new(Size::default())
            }
            fn overlay_open(&self) -> bool {
                self.0 == 0
            }
            fn overlay_presented(&self) -> bool {
                self.0 == 1
            }
            fn overlay_scope(&self) -> Option<OverlayScope> {
                (self.0 == 2).then_some(OverlayScope::default())
            }
            fn composing(&self) -> bool {
                self.0 == 3
            }
            fn reading_scroll_source(&self) -> bool {
                self.0 == 4
            }
        }
        for child in [
            Content::new(ComponentId(204), Button::new("button")),
            Content::new(ComponentId(205), LinkContent::new("link")),
            Content::new(ComponentId(206), MenuContent::new("menu", vec![])),
            Content::new(ComponentId(207), Capability(0)),
            Content::new(ComponentId(208), Capability(1)),
            Content::new(ComponentId(209), Capability(2)),
            Content::new(ComponentId(210), Capability(3)),
            Content::new(ComponentId(211), Capability(4)),
            Content::new(
                ComponentId(200),
                SelectableListContent::new("empty", vec![]),
            ),
            Content::new(ComponentId(201), TextFieldContent::new("edit", "value")),
            Content::new(
                ComponentId(202),
                ScrollContent::new(
                    crate::ScrollArea::default(),
                    Content::new(ComponentId(203), TextContent::new("read")),
                ),
            ),
        ] {
            let c = list();
            let mut rows = d(&s());
            let id = child.id();
            rows[0].display = ListDisplayUpdate::Replace(Content::new(
                ComponentId(300),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![child],
                },
            ));
            let input = format!("{rows:?}");
            assert_eq!(
                c.prepare(&s(), &rows, (0, 0)).unwrap_err(),
                ListContentError::InteractiveDisplay {
                    key: K(1),
                    child: id
                }
            );
            assert_eq!(format!("{rows:?}"), input);
        }
        let mut rows = d(&s());
        rows[0].display = ListDisplayUpdate::Replace(Content::new(ComponentId(200), Unknown));
        let input = format!("{rows:?}");
        assert!(matches!(
            list().prepare(&s(), &rows, (0, 0)),
            Err(ListContentError::UnknownTransactionHooks {
                child: ComponentId(200)
            })
        ));
        assert_eq!(format!("{rows:?}"), input);
        assert!(ContentHost::new(Content::new(ComponentId(200), Unknown)).is_ok());
    }
    #[test]
    fn activity_callbacks_never_allocate_tokens_or_change_mount_and_full_v_recovers_once() {
        let mut h = host(false);
        let old = list_ref(&h).semantic_parts();
        let root = h.semantic_mount(ComponentId(10));
        let next = list_ref(&h).next;
        h.set_window_active(false);
        assert_eq!(list_ref(&h).next, next);
        assert!(list_ref(&h).semantic_parts().is_empty());
        h.set_window_active(true);
        assert_eq!(list_ref(&h).next, next);
        paint(&mut h);
        assert_eq!(h.semantic_mount(ComponentId(10)), root);
        assert_eq!(list_ref(&h).next, next + 3);
        for p in old {
            assert!(
                !h.accessibility_action(ComponentId(10), Some(p.index), AccessibilityAction::Press)
                    .consumed
            );
        }
        let now = list_ref(&h).semantic_parts();
        paint(&mut h);
        assert_eq!(list_ref(&h).next, next + 3);
        assert!(
            h.accessibility_action(
                ComponentId(10),
                Some(now[0].index),
                AccessibilityAction::Focus
            )
            .consumed
        );
    }
    #[test]
    fn actual_part_extreme_recovery_is_atomic_and_void_epoch_overflow_is_closed() {
        let mut c = mounted();
        c.next = usize::MAX - 2;
        let old = c.next;
        c.window_active_changed(false);
        c.window_active_changed(true);
        assert_eq!(c.next, old);
        let before = format!("{c:?}");
        assert_eq!(c.prepare_exposure(), Err(ListContentError::TokenCapacity));
        assert_eq!(format!("{c:?}"), before);
        assert!(c.semantic_parts().is_empty());
        c.next = usize::MAX - 3;
        c.prepare_exposure().unwrap();
        assert_eq!(c.next, usize::MAX);
        c.arrange(&mut Text, f(), f());
        assert_eq!(c.semantic_parts().len(), 3);
        c.window_active_changed(false);
        assert_eq!(c.next, usize::MAX);
        c.window_active_changed(true);
        assert!(c.prepare_exposure().is_err());
        c.activity_epoch = u64::MAX;
        c.window_active_changed(false);
        c.window_active_changed(true);
        assert!(c.exhausted);
        assert!(!c.available());
    }
    #[test]
    fn queued_and_drained_intents_are_once_only_and_follow_identity_not_revision() {
        let mut h = host(false);
        h.event(&ContentEvent::PointerDown(Point::new(10., 10.)));
        h.event(&ContentEvent::PointerUp(Point::new(10., 10.)));
        let mut snap = s();
        snap.display_items.reverse();
        h.replace_keyed_list(ComponentId(10), &snap, &mut d(&snap), None)
            .unwrap();
        let actions = h.drain_action_records();
        assert_eq!(actions.len(), 1);
        assert!(h.drain_action_records().is_empty());
        let ContentActionPayload::ListIntent(i) = actions[0].payload.as_ref().unwrap() else {
            panic!()
        };
        assert_eq!(i.kind, ListIntentKind::SelectionChanged);
        assert!(list_ref(&h).intent_is_current(i));
        paint(&mut h);
        h.event(&ContentEvent::Cancel);
        assert!(list_ref(&h).intent_is_current(i));
        assert!(h.focus(Some(ComponentId(10)), true));
        h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
        let a = h.drain_action_records();
        assert_eq!(a.len(), 1);
        assert!(matches!(
            a[0].payload,
            Some(ContentActionPayload::ListIntent(ListIntent {
                kind: ListIntentKind::Activated,
                ..
            }))
        ));
        h.set_window_active(false);
        h.set_window_active(true);
        paint(&mut h);
        assert!(!list_ref(&h).intent_is_current(i));
        let part = list_ref(&h).semantic_parts()[0].index;
        h.accessibility_action(ComponentId(10), Some(part), AccessibilityAction::Press);
        h.set_window_active(false);
        h.set_window_active(true);
        paint(&mut h);
        assert!(h.drain_action_records().is_empty());
    }
    #[test]
    fn reading_rejection_preserves_model_view_queue_mounts_and_caller_input() {
        let mut h = host(true);
        h.event(&ContentEvent::PointerDown(Point::new(10., 10.)));
        let mut snap = s();
        snap.display_items.reverse();
        let mut rows = d(&snap);
        let mut reading = read(&h, &snap);
        reading.revision = reading.expected.revision();
        let before = format!("{h:?}");
        let input = format!("{rows:?}");
        assert!(
            h.replace_keyed_list(ComponentId(10), &snap, &mut rows, Some(&reading))
                .is_err()
        );
        assert_eq!(format!("{h:?}"), before);
        assert_eq!(format!("{rows:?}"), input);
        let mut reading = read(&h, &snap);
        reading.items.swap(0, 1);
        assert_eq!(
            h.replace_keyed_list(ComponentId(10), &snap, &mut rows, Some(&reading)),
            Err(ListContentError::ReadingItems)
        );
        assert_eq!(format!("{h:?}"), before);
        assert_eq!(
            h.replace_keyed_list(ComponentId(10), &snap, &mut rows, None),
            Err(ListContentError::HostTransactionRequired)
        );
        assert_eq!(format!("{h:?}"), before);
        let reading = read(&h, &snap);
        h.replace_keyed_list(ComponentId(10), &snap, &mut rows, Some(&reading))
            .unwrap();
        assert_eq!(
            h.get(ComponentId(1))
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .reading_items(),
            reading.items
        );
        assert!(rows.is_empty());
        assert!(
            list_ref(&h)
                .view
                .layout_keyed(&list_ref(&h).model)
                .is_none()
        );
    }
    #[test]
    fn full_target_hidden_keep_descendant_and_outside_duplicate_ids_reject_atomically() {
        let mut h = host(false);
        let mut rows = d(&s());
        rows[0].display = ListDisplayUpdate::Replace(Content::new(
            ComponentId(1),
            TextContent::new("collision with reader"),
        ));
        let before = format!("{h:?}");
        assert!(matches!(
            h.replace_keyed_list(ComponentId(10), &s(), &mut rows, None),
            Err(ListContentError::Composition(
                CompositionError::DuplicateId(ComponentId(1))
            ))
        ));
        assert_eq!(format!("{h:?}"), before);
        let mut snap = s();
        snap.display_items = vec![K(2)];
        let mut rows = d(&snap);
        rows[0].display = ListDisplayUpdate::Replace(Content::new(
            ComponentId(200),
            TextContent::new("hidden retained"),
        ));
        h.replace_keyed_list(ComponentId(10), &snap, &mut rows, None)
            .unwrap();
        let mut rows = d(&snap);
        rows[1].display = ListDisplayUpdate::Replace(Content::new(
            ComponentId(200),
            TextContent::new("duplicate"),
        ));
        let before = format!("{h:?}");
        assert!(matches!(
            h.replace_keyed_list(ComponentId(10), &snap, &mut rows, None),
            Err(ListContentError::Composition(
                CompositionError::DuplicateId(ComponentId(200))
            ))
        ));
        assert_eq!(format!("{h:?}"), before);
    }
    #[test]
    fn budget_and_overflow_preflight_are_nonmutating() {
        let mut c = list();
        let mut snap = s();
        snap.items[0].name = "x".repeat(100001);
        let before = format!("{c:?}");
        assert!(matches!(
            c.replace_keyed(&snap, &mut d(&snap)),
            Err(ListContentError::BudgetExceeded { kind: "utf8", .. })
        ));
        assert_eq!(format!("{c:?}"), before);
        assert_eq!(add(usize::MAX, 1), Err(ListContentError::Overflow));
        assert_eq!(mul(usize::MAX, 2), Err(ListContentError::Overflow));
        assert!(matches!(
            c.prepare(&s(), &d(&s()), (10001, 0)),
            Err(ListContentError::BudgetExceeded {
                kind: "staging",
                ..
            })
        ));
    }
    #[test]
    fn positive_visible_parts_share_hit_geometry_clip_does_not_revoke_and_zero_width_is_closed() {
        let mut c = mounted();
        let all = c.semantic_parts();
        let before = c.next;
        c.arrange(&mut Text, f(), Rect::new(0., 10., 200., 15.));
        let parts = c.semantic_parts();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].frame, Rect::new(0., 10., 200., 15.));
        assert_eq!(parts[0].index, all[0].index);
        assert_eq!(c.next, before);
        assert!(
            !c.event(&ContentEvent::PointerDown(Point::new(10., 5.)))
                .consumed
        );
        assert!(
            c.event(&ContentEvent::PointerDown(Point::new(10., 15.)))
                .consumed
        );
        assert_eq!(c.selected_key(), Some(K(1)));
        c.arrange(&mut Text, Rect::new(0., 0., 0., 300.), f());
        assert!(c.semantic_parts().is_empty());
        assert!(
            !c.accessibility_action(Some(all[0].index), &AccessibilityAction::Press)
                .consumed
        );
    }
    #[test]
    fn unmount_same_object_invalidates_already_drained_intents_without_void_token_demand() {
        let mut c = mounted();
        let token = c.semantic_parts()[0].index;
        c.accessibility_action(Some(token), &AccessibilityAction::Press);
        let i = c.last_intent.clone().unwrap();
        let next = c.next;
        c.lifecycle(Lifecycle::Unmounted);
        c.lifecycle(Lifecycle::Mounted);
        assert_eq!(c.next, next);
        assert!(!c.intent_is_current(&i));
        c.arrange(&mut Text, f(), f());
        assert!(c.semantic_parts()[0].index > token);
        assert!(!c.intent_is_current(&i));
    }

    #[test]
    fn typed_list_updates_revoke_host_geometry_only_after_success() {
        use std::sync::atomic::Ordering;
        for reading in [false, true] {
            let mut h = host(reading);
            let snap = s();
            let update = reading.then(|| read(&h, &snap));
            let lease = h.reveal_clock.geometry_lease.clone();
            let old_lease = lease.load(Ordering::Acquire);
            // Same entities and Keep displays still require a fresh Host layout.
            h.replace_keyed_list(ComponentId(10), &snap, &mut d(&snap), update.as_ref())
                .unwrap();
            assert_eq!(lease.load(Ordering::Acquire), old_lease + 1);
            assert!(h.reveal_clock.pending);
            assert!(h.dirty);
            paint(&mut h);
            assert!(!h.reveal_clock.pending);
            if let Some(stale) = update.as_ref() {
                let before = format!("{h:?}");
                let current_lease = lease.load(Ordering::Acquire);
                assert!(
                    h.replace_keyed_list(ComponentId(10), &snap, &mut d(&snap), Some(stale))
                        .is_err()
                );
                assert_eq!(lease.load(Ordering::Acquire), current_lease);
                assert_eq!(format!("{h:?}"), before);
            }
            let before = format!("{h:?}");
            let current_lease = lease.load(Ordering::Acquire);
            let mut bad = snap.clone();
            bad.display_items.push(K(999));
            let refused = reading.then(|| read(&h, &bad));
            let mut displays = d(&bad);
            let before_displays = format!("{displays:?}");
            assert!(
                h.replace_keyed_list(ComponentId(10), &bad, &mut displays, refused.as_ref())
                    .is_err()
            );
            assert_eq!(lease.load(Ordering::Acquire), current_lease);
            assert_eq!(format!("{h:?}"), before);
            assert_eq!(format!("{displays:?}"), before_displays);
        }
    }
}
