//! Finite passive table composition over the original keyed model and View.
use super::*;
use crate::collections_view::{
    CollectionEvent, DataTableLayout, DataTableView, TableContentMetrics,
};
use crate::extended::{
    KeyedDataTable, KeyedTableSnapshot, SortDirection, TableBudget, TableColumnKey as C,
    TableDatasetKey as D, TableRowKey as R, TableUpdateError,
};
use crate::{ReadingDocumentVersion, ReadingUpdateError};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TableDisplayUpdate {
    Keep,
    DefaultText,
    Replace(Content),
}
#[derive(Debug)]
pub struct TableHeaderDisplay {
    pub key: C,
    pub display: TableDisplayUpdate,
}
#[derive(Debug)]
pub struct TableCellDisplay {
    pub column: C,
    pub display: TableDisplayUpdate,
    pub accessory: TableDisplayUpdate,
}
#[derive(Debug)]
pub struct TableRowDisplay {
    pub key: R,
    pub id: ComponentId,
    pub replace_wrapper: bool,
    pub cells: Vec<TableCellDisplay>,
}
#[derive(Debug, Default)]
pub struct TableDisplays {
    pub headers: Vec<TableHeaderDisplay>,
    pub rows: Vec<TableRowDisplay>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableIntentTarget {
    Header(C),
    Row(R),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableIntentKind {
    SelectionChanged,
    Activated,
    SortChanged { direction: SortDirection },
}
#[derive(Clone, Debug)]
pub struct TableIntent {
    origin: Arc<()>,
    pub dataset: D,
    pub target: TableIntentTarget,
    pub exposure: usize,
    pub model_revision: u64,
    pub activity_epoch: u64,
    pub kind: TableIntentKind,
    pub selected_key: Option<R>,
    pub sort: Option<(C, SortDirection)>,
}
impl PartialEq for TableIntent {
    fn eq(&self, b: &Self) -> bool {
        Arc::ptr_eq(&self.origin, &b.origin)
            && self.dataset == b.dataset
            && self.target == b.target
            && self.exposure == b.exposure
            && self.model_revision == b.model_revision
            && self.activity_epoch == b.activity_epoch
            && self.kind == b.kind
            && self.selected_key == b.selected_key
            && self.sort == b.sort
    }
}
impl Eq for TableIntent {}
#[derive(Debug)]
pub struct TableReadingUpdate {
    pub owner: ComponentId,
    pub expected: ReadingDocumentVersion,
    pub revision: u64,
    pub items: Vec<ComponentId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableContentError {
    Model(TableUpdateError),
    Composition(CompositionError),
    Reading(ReadingUpdateError),
    InvalidStructure,
    InvalidDisplays,
    InteractiveDisplay {
        row: Option<R>,
        column: Option<C>,
        accessory: bool,
        child: ComponentId,
    },
    RowIdentityChanged {
        key: R,
    },
    HostTransactionRequired,
    InvalidTarget(ComponentId),
    TokenCapacity,
    MountCapacity,
    UnknownTransactionHooks {
        child: ComponentId,
    },
    BudgetExceeded,
    Overflow,
    ReadingOwner,
    ReadingItems,
}
impl From<TableUpdateError> for TableContentError {
    fn from(e: TableUpdateError) -> Self {
        Self::Model(e)
    }
}
impl From<CompositionError> for TableContentError {
    fn from(e: CompositionError) -> Self {
        Self::Composition(e)
    }
}
impl From<ReadingUpdateError> for TableContentError {
    fn from(e: ReadingUpdateError) -> Self {
        Self::Reading(e)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    Header(C),
    Cell(R, C),
    Accessory(R, C),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Column(C),
    Header(C),
    Row(R),
    Cell(R, C),
}
#[derive(Clone, Copy, Debug)]
struct Exposure {
    token: usize,
    row: u64,
    column: u64,
}
#[derive(Debug)]
struct RowWrapper {
    identity: Arc<()>,
    slots: Vec<Option<ComponentId>>,
    children: Vec<Content>,
    disabled: bool,
    ink: Option<(Rgba, Rgba)>,
}
impl Component for RowWrapper {
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
        if let Some((text, icon)) = self.ink {
            return ContentTheme {
                foreground: Some(text),
                icon_foreground: Some(icon),
                ..t
            };
        }
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(
            t.palette,
            t.surface(),
            "content",
            if self.disabled { "disabled" } else { "rest" },
        );
        colors.map_or(t, |c| ContentTheme {
            foreground: Some(c.text),
            icon_foreground: Some(c.icon),
            ..t
        })
    }
}
fn passive(c: &Content) -> Result<(usize, usize), TableContentError> {
    passive_impl(c, true)
}
fn passive_count(c: &Content) -> Result<(usize, usize), TableContentError> {
    passive_impl(c, false)
}
fn passive_impl(c: &Content, validate: bool) -> Result<(usize, usize), TableContentError> {
    let p = &*c.component;
    if p.interactive()
        || p.focusable()
        || p.text_input_snapshot().is_some()
        || p.text_surface().is_some()
        || p.overlay_scope().is_some()
        || p.overlay_open()
        || p.overlay_presented()
        || p.composing()
        || p.reading_scroll_source()
    {
        return Err(TableContentError::InteractiveDisplay {
            row: None,
            column: None,
            accessory: false,
            child: c.id(),
        });
    }
    if validate {
        p.validate()?;
    }
    let s = p.semantics();
    let mut bytes = s
        .name
        .len()
        .checked_add(s.value.as_ref().map_or(0, String::len))
        .and_then(|n| n.checked_add(s.description.as_ref().map_or(0, String::len)))
        .ok_or(TableContentError::Overflow)?;
    let mut units = 1usize;
    for child in p.children() {
        let (u, b) = passive_impl(child, validate)?;
        units = add(units, u)?;
        bytes = add(bytes, b)?;
    }
    if p.table_transaction_compatibility() != TableTransactionCompatibility::StableHooks {
        return Err(TableContentError::UnknownTransactionHooks { child: c.id() });
    }
    Ok((units, bytes))
}
fn passive_slot(c: &Content, field: Field) -> Result<(usize, usize), TableContentError> {
    passive_slot_impl(c, field, true)
}
fn passive_slot_count(c: &Content, field: Field) -> Result<(usize, usize), TableContentError> {
    passive_slot_impl(c, field, false)
}
fn passive_slot_impl(
    c: &Content,
    field: Field,
    validate: bool,
) -> Result<(usize, usize), TableContentError> {
    passive_impl(c, validate).map_err(|error| match error {
        TableContentError::InteractiveDisplay { child, .. } => {
            let (row, column, accessory) = match field {
                Field::Header(c) => (None, Some(c), false),
                Field::Cell(r, c) => (Some(r), Some(c), false),
                Field::Accessory(r, c) => (Some(r), Some(c), true),
            };
            TableContentError::InteractiveDisplay {
                row,
                column,
                accessory,
                child,
            }
        }
        other => other,
    })
}
fn add(a: usize, b: usize) -> Result<usize, TableContentError> {
    a.checked_add(b).ok_or(TableContentError::Overflow)
}
fn mul(a: usize, b: usize) -> Result<usize, TableContentError> {
    a.checked_mul(b).ok_or(TableContentError::Overflow)
}
fn limits(b: TableBudget, u: usize, s: usize) -> Result<(), TableContentError> {
    if u > b.max_units || s > b.max_utf8_bytes {
        Err(TableContentError::BudgetExceeded)
    } else {
        Ok(())
    }
}
#[derive(Debug)]
struct Prepared {
    model: KeyedDataTable,
    mounts: usize,
    exposures: BTreeMap<Part, Exposure>,
    next: usize,
    ids: BTreeSet<ComponentId>,
}
#[derive(Debug)]
pub struct DataTableContent {
    name: String,
    model: KeyedDataTable,
    view: DataTableView,
    children: Vec<Content>,
    headers: BTreeMap<C, Option<ComponentId>>,
    rows: BTreeMap<R, (ComponentId, Arc<()>)>,
    exposures: BTreeMap<Part, Exposure>,
    next: usize,
    mounted: bool,
    disabled: bool,
    window_active: bool,
    activity_epoch: u64,
    exhausted: bool,
    last_intent: Option<TableIntent>,
    metrics: TableContentMetrics,
}
impl DataTableContent {
    /// Borrowed staging: every error preserves the complete caller input, including
    /// Unknown components without invoking their Drop. Success takes displays,
    /// leaving it empty. StableHooks is stronger than merely passive: lazy trees
    /// must be constructed before this call. OOM/custom panic is not rollback-safe.
    pub fn new(
        name: &str,
        snapshot: &KeyedTableSnapshot,
        displays: &mut TableDisplays,
        budget: TableBudget,
    ) -> Result<Self, TableContentError> {
        let empty = KeyedTableSnapshot {
            dataset: snapshot.dataset,
            columns: vec![],
            rows: vec![],
            display_rows: vec![],
        };
        let mut t = Self {
            name: name.into(),
            model: KeyedDataTable::new(empty, budget)?,
            view: DataTableView::new(),
            children: vec![],
            headers: BTreeMap::new(),
            rows: BTreeMap::new(),
            exposures: BTreeMap::new(),
            next: 0,
            mounted: false,
            disabled: false,
            window_active: true,
            activity_epoch: 0,
            exhausted: false,
            last_intent: None,
            metrics: TableContentMetrics::default(),
        };
        let p = t.prepare(snapshot, displays, (0, 0))?;
        let mut range =
            actions::reserve_mounts(p.mounts).ok_or(TableContentError::MountCapacity)?;
        t.commit(p, std::mem::take(displays), &mut range);
        Ok(t)
    }
    pub fn model(&self) -> &KeyedDataTable {
        &self.model
    }
    pub fn selected_key(&self) -> Option<R> {
        self.model.selected_key()
    }
    pub fn sort_key_state(&self) -> Option<(C, SortDirection)> {
        self.model.sort_key_state()
    }
    /// Failure retains every display in the caller; success empties displays.
    pub fn replace_snapshot(
        &mut self,
        s: &KeyedTableSnapshot,
        d: &mut TableDisplays,
    ) -> Result<(), TableContentError> {
        if self.mounted {
            return Err(TableContentError::HostTransactionRequired);
        }
        let p = self.prepare(s, d, (0, 0))?;
        let mut range =
            actions::reserve_mounts(p.mounts).ok_or(TableContentError::MountCapacity)?;
        self.commit(p, std::mem::take(d), &mut range);
        Ok(())
    }
    pub fn set_disabled(&mut self, disabled: bool) -> Result<bool, TableContentError> {
        self.validate_all()?;
        if self.disabled == disabled {
            return Ok(false);
        }
        if self.exhausted {
            return Err(TableContentError::TokenCapacity);
        }
        let epoch = self
            .activity_epoch
            .checked_add(1)
            .ok_or(TableContentError::TokenCapacity)?;
        let next = self
            .next
            .checked_add(self.exposures.len())
            .ok_or(TableContentError::TokenCapacity)?;
        let exposures = self
            .exposures
            .iter()
            .enumerate()
            .map(|(i, (part, e))| {
                (
                    *part,
                    Exposure {
                        token: self.next + i + 1,
                        ..*e
                    },
                )
            })
            .collect();
        self.disabled = disabled;
        self.activity_epoch = epoch;
        self.exposures = exposures;
        self.next = next;
        self.view.set_disabled(disabled);
        Ok(true)
    }
    fn bump_epoch(&mut self) {
        if let Some(e) = self.activity_epoch.checked_add(1) {
            self.activity_epoch = e;
        } else {
            self.exhausted = true;
        }
    }
    fn validate_structure(&self) -> Result<(), TableContentError> {
        let invalid = TableContentError::InvalidStructure;
        let hs = self.headers.values().filter(|v| v.is_some()).count();
        if self.headers.len() != self.model.snapshot().columns.len()
            || self.rows.len() != self.model.snapshot().rows.len()
            || self.children.len() != hs + self.rows.len()
        {
            return Err(invalid);
        }
        let mut j = 0;
        for col in &self.model.snapshot().columns {
            let Some(binding) = self.headers.get(&col.key) else {
                return Err(invalid);
            };
            if let Some(id) = binding {
                if self.children.get(j).is_none_or(|c| c.id() != *id) {
                    return Err(invalid);
                }
                j += 1;
            }
        }
        for row in &self.model.snapshot().rows {
            let Some((id, marker)) = self.rows.get(&row.key) else {
                return Err(invalid);
            };
            let Some(c) = self.children.get(j) else {
                return Err(invalid);
            };
            let Some(w) = c.component::<RowWrapper>() else {
                return Err(invalid);
            };
            if c.id() != *id
                || !Arc::ptr_eq(marker, &w.identity)
                || w.slots.len() != mul(self.headers.len(), 2)?
                || w.children.len() != w.slots.iter().flatten().count()
                || w.children
                    .iter()
                    .map(Content::id)
                    .ne(w.slots.iter().flatten().copied())
            {
                return Err(invalid);
            }
            j += 1;
        }
        Ok(())
    }
    fn validate_traversal(&self) -> Result<(), TableContentError> {
        self.validate_structure()?;
        for c in &self.children {
            passive_count(c)?;
        }
        Ok(())
    }
    fn field_ref(&self, field: Field) -> Option<&Content> {
        match field {
            Field::Header(key) => {
                self.headers.get(&key).copied().flatten()?;
                let index = self
                    .model
                    .snapshot()
                    .columns
                    .iter()
                    .take_while(|c| c.key != key)
                    .filter(|c| self.headers.get(&c.key).is_some_and(Option::is_some))
                    .count();
                self.children.get(index)
            }
            Field::Cell(row, column) | Field::Accessory(row, column) => {
                let r = self.model.row_index(row)?;
                let c = self.model.column_index(column)?;
                let h = self.headers.values().filter(|v| v.is_some()).count();
                let w = self.children.get(h + r)?.component::<RowWrapper>()?;
                let slot = c
                    .checked_mul(2)?
                    .checked_add(usize::from(matches!(field, Field::Accessory(..))))?;
                w.slots.get(slot).copied().flatten()?;
                let index = w.slots[..slot].iter().filter(|v| v.is_some()).count();
                w.children.get(index)
            }
        }
    }
    fn target_field<'a>(
        &'a self,
        field: Field,
        update: &'a TableDisplayUpdate,
        reuse: bool,
    ) -> Option<&'a Content> {
        match update {
            TableDisplayUpdate::Replace(c) => Some(c),
            TableDisplayUpdate::Keep if reuse => self.field_ref(field),
            _ => None,
        }
    }
    fn validate_all(&self) -> Result<(), TableContentError> {
        self.validate_structure()?;
        let mut ids = BTreeSet::new();
        for (field, c) in self.fields() {
            passive_slot(c, field)?;
        }
        for c in &self.children {
            c.ids(&mut ids)?;
        }
        Ok(())
    }
    fn fields(&self) -> BTreeMap<Field, &Content> {
        let mut map = BTreeMap::new();
        let mut i = 0;
        for col in &self.model.snapshot().columns {
            if self.headers.get(&col.key).is_some_and(Option::is_some) {
                if let Some(c) = self.children.get(i) {
                    map.insert(Field::Header(col.key), c);
                }
                i += 1;
            }
        }
        for row in &self.model.snapshot().rows {
            if let Some(w) = self
                .children
                .get(i)
                .and_then(|c| c.component::<RowWrapper>())
            {
                let mut j = 0;
                for (k, col) in self.model.snapshot().columns.iter().enumerate() {
                    for (slot, f) in [
                        (2 * k, Field::Cell(row.key, col.key)),
                        (2 * k + 1, Field::Accessory(row.key, col.key)),
                    ] {
                        if w.slots.get(slot).is_some_and(Option::is_some) {
                            if let Some(c) = w.children.get(j) {
                                map.insert(f, c);
                            }
                            j += 1;
                        }
                    }
                }
            }
            i += 1;
        }
        map
    }
    fn plan_exposures(
        &self,
        m: &KeyedDataTable,
    ) -> Result<(BTreeMap<Part, Exposure>, usize), TableContentError> {
        let mut next = self.next;
        let mut result = BTreeMap::new();
        let same = m.dataset_key() == self.model.dataset_key();
        let mut insert = |part, row, column| -> Result<(), TableContentError> {
            let old = self
                .exposures
                .get(&part)
                .filter(|e| same && e.row == row && e.column == column);
            let e = if let Some(e) = old {
                *e
            } else {
                next = next
                    .checked_add(1)
                    .ok_or(TableContentError::TokenCapacity)?;
                Exposure {
                    token: next,
                    row,
                    column,
                }
            };
            result.insert(part, e);
            Ok(())
        };
        for (j, col) in m.snapshot().columns.iter().enumerate() {
            let inc = m.column_incarnation(j).unwrap_or(0);
            insert(Part::Column(col.key), 0, inc)?;
            insert(Part::Header(col.key), 0, inc)?;
        }
        for &i in m.display_indices() {
            let row = &m.snapshot().rows[i];
            let inc = m.row_incarnation(i).unwrap_or(0);
            insert(Part::Row(row.key), inc, 0)?;
            for (j, col) in m.snapshot().columns.iter().enumerate() {
                insert(
                    Part::Cell(row.key, col.key),
                    inc,
                    m.column_incarnation(j).unwrap_or(0),
                )?;
            }
        }
        Ok((result, next))
    }
    fn prepare(
        &self,
        s: &KeyedTableSnapshot,
        d: &TableDisplays,
        extra: (usize, usize),
    ) -> Result<Prepared, TableContentError> {
        self.validate_structure()?;
        let r = s.rows.len();
        let c = s.columns.len();
        let v = s.display_rows.len();
        let cells = mul(r, c)?;
        let shown = mul(v, c)?;
        // Full peak: model+maps, snapshot/reconcile vectors, logical slots,
        // wrappers, parts+exposure maps, displays+lookup plans, reader plan.
        let model = add(add(add(r, c)?, cells)?, v)?;
        let slots = add(c, mul(cells, 2)?)?;
        let parts = add(add(add(1, mul(c, 2)?)?, v)?, shown)?;
        let mut units = add(
            add(mul(model, 4)?, mul(slots, 3)?)?,
            add(mul(parts, 3)?, mul(r, 4)?)?,
        )?;
        let old_r = self.model.snapshot().rows.len();
        let old_c = self.model.snapshot().columns.len();
        units = add(units, mul(add(add(old_r, old_c)?, mul(old_r, old_c)?)?, 8)?)?;
        units = add(units, extra.0)?;
        let mut bytes = add(
            extra.1,
            mul(units, std::mem::size_of::<Content>().max(128))?,
        )?;
        for text in std::iter::once(&self.name)
            .chain(s.columns.iter().map(|c| &c.name))
            .chain(
                s.rows
                    .iter()
                    .flat_map(|r| std::iter::once(&r.name).chain(r.cells.iter())),
            )
        {
            bytes = add(bytes, mul(text.capacity(), 4)?)?;
        }
        let (vu, vb) = self
            .view
            .content_retained_cost()
            .ok_or(TableContentError::Overflow)?;
        units = add(units, vu)?;
        bytes = add(bytes, vb)?;
        let mut incoming = add(
            add(s.rows.capacity(), s.columns.capacity())?,
            s.display_rows.capacity(),
        )?;
        incoming = add(incoming, add(d.rows.capacity(), d.headers.capacity())?)?;
        for row in &d.rows {
            incoming = add(incoming, row.cells.capacity())?;
        }
        for row in &s.rows {
            incoming = add(incoming, row.cells.capacity())?;
        }
        units = add(units, incoming)?;
        bytes = add(
            bytes,
            mul(incoming, std::mem::size_of::<Content>().max(128))?,
        )?;
        for text in self.model.snapshot().columns.iter().map(|c| &c.name).chain(
            self.model
                .snapshot()
                .rows
                .iter()
                .flat_map(|r| std::iter::once(&r.name).chain(r.cells.iter())),
        ) {
            bytes = add(bytes, mul(text.capacity(), 2)?)?;
        }
        for child in &self.children {
            let (n, b) = passive_count(child)?;
            units = add(units, n)?;
            bytes = add(
                bytes,
                add(b, mul(n, std::mem::size_of::<Content>().max(128))?)?,
            )?;
        }
        limits(self.model.budget(), units, bytes)?;
        if d.headers.len() != c || d.rows.len() != r {
            return Err(TableContentError::InvalidDisplays);
        }
        // Validate declarations without allocating lookup/uniqueness maps first.
        for (i, col) in s.columns.iter().enumerate() {
            if d.headers[i].key != col.key {
                return Err(TableContentError::InvalidDisplays);
            }
        }
        for (i, row) in s.rows.iter().enumerate() {
            if d.rows[i].key != row.key
                || d.rows[i].cells.len() != c
                || d.rows[i]
                    .cells
                    .iter()
                    .zip(&s.columns)
                    .any(|(a, b)| a.column != b.key)
            {
                return Err(TableContentError::InvalidDisplays);
            }
        }
        let same = s.dataset == self.model.dataset_key();
        // Count the complete target without constructing a field map or ID set.
        // Every retained hidden descendant is visited through the original tree.
        let mut target_nodes = r;
        let mut count = |field: Field,
                         update: &TableDisplayUpdate,
                         reuse: bool|
         -> Result<(), TableContentError> {
            if let Some(c) = self.target_field(field, update, reuse) {
                let (n, b) = passive_slot_count(c, field)?;
                target_nodes = add(target_nodes, n)?;
                units = add(units, mul(n, 3)?)?;
                bytes = add(
                    bytes,
                    add(
                        mul(b, 2)?,
                        mul(mul(n, 3)?, std::mem::size_of::<Content>().max(128))?,
                    )?,
                )?;
            }
            Ok(())
        };
        for header in &d.headers {
            count(Field::Header(header.key), &header.display, same)?;
        }
        for row in &d.rows {
            if same
                && !row.replace_wrapper
                && self.rows.get(&row.key).is_some_and(|(id, _)| *id != row.id)
            {
                return Err(TableContentError::RowIdentityChanged { key: row.key });
            }
            for cell in &row.cells {
                count(
                    Field::Cell(row.key, cell.column),
                    &cell.display,
                    same && !row.replace_wrapper,
                )?;
                count(
                    Field::Accessory(row.key, cell.column),
                    &cell.accessory,
                    same && !row.replace_wrapper,
                )?;
            }
        }
        // Own target set and its contribution to the simultaneous full Host set;
        // outer Host nodes and reader collections were already included in extra.
        units = add(units, mul(target_nodes, 2)?)?;
        bytes = add(
            bytes,
            mul(
                mul(target_nodes, 2)?,
                std::mem::size_of::<Content>().max(128),
            )?,
        )?;
        limits(self.model.budget(), units, bytes)?;
        // Only after complete peak admission may current/target uniqueness maps
        // and the model/token plans allocate their collections.
        self.validate_all()?;
        let mut ids = BTreeSet::new();
        for header in &d.headers {
            if let Some(c) = self.target_field(Field::Header(header.key), &header.display, same) {
                c.ids(&mut ids)?;
            }
        }
        for row in &d.rows {
            if !ids.insert(row.id) {
                return Err(CompositionError::DuplicateId(row.id).into());
            }
            for cell in &row.cells {
                for (field, update) in [
                    (Field::Cell(row.key, cell.column), &cell.display),
                    (Field::Accessory(row.key, cell.column), &cell.accessory),
                ] {
                    if let Some(c) = self.target_field(field, update, same && !row.replace_wrapper)
                    {
                        c.ids(&mut ids)?;
                    }
                }
            }
        }
        let (model, _) = self.model.prepare_replace(s.clone())?;
        let (mut exposures, mut next) = self.plan_exposures(&model)?;
        for row in &d.rows {
            if row.replace_wrapper {
                for (part, e) in &mut exposures {
                    if matches!(part,Part::Row(r)|Part::Cell(r,_) if *r==row.key) {
                        next = next
                            .checked_add(1)
                            .ok_or(TableContentError::TokenCapacity)?;
                        e.token = next;
                    }
                }
            }
        }

        Ok(Prepared {
            model,
            mounts: self.mount_cost(s, d)?,
            exposures,
            next,
            ids,
        })
    }
    fn mount_cost(
        &self,
        s: &KeyedTableSnapshot,
        d: &TableDisplays,
    ) -> Result<usize, TableContentError> {
        let same = s.dataset == self.model.dataset_key();
        let mut cost = 0;
        if self.mounted {
            for (field, c) in self.fields() {
                let kept =
                    match field {
                        Field::Header(k) => {
                            same && d.headers.iter().any(|h| {
                                h.key == k && matches!(h.display, TableDisplayUpdate::Keep)
                            })
                        }
                        Field::Cell(r, k) | Field::Accessory(r, k) => {
                            same && d
                                .rows
                                .iter()
                                .find(|row| row.key == r && !row.replace_wrapper)
                                .is_some_and(|row| {
                                    row.cells.iter().find(|cell| cell.column == k).is_some_and(
                                        |cell| {
                                            matches!(
                                                if matches!(field, Field::Accessory(..)) {
                                                    &cell.accessory
                                                } else {
                                                    &cell.display
                                                },
                                                TableDisplayUpdate::Keep
                                            )
                                        },
                                    )
                                })
                        }
                    };
                if !kept {
                    cost = add(cost, passive(c)?.0)?;
                }
            }
            for key in self.rows.keys() {
                if !(same && d.rows.iter().any(|r| r.key == *key && !r.replace_wrapper)) {
                    cost = add(cost, 1)?;
                }
            }
            for h in &d.headers {
                if let TableDisplayUpdate::Replace(c) = &h.display {
                    cost = add(cost, passive(c)?.0)?;
                }
            }
        }
        for row in &d.rows {
            let new = !(same && !row.replace_wrapper && self.rows.contains_key(&row.key));
            if new {
                cost = add(cost, 1)?;
            } // constructor, even when detached
            if self.mounted {
                if new {
                    cost = add(cost, 1)?;
                } // wrapper Mounted
                for cell in &row.cells {
                    for (field, u) in [
                        (Field::Cell(row.key, cell.column), &cell.display),
                        (Field::Accessory(row.key, cell.column), &cell.accessory),
                    ] {
                        if (new || matches!(u, TableDisplayUpdate::Replace(_)))
                            && let Some(c) =
                                self.target_field(field, u, same && !row.replace_wrapper)
                        {
                            cost = add(cost, passive(c)?.0)?;
                        }
                    }
                }
            }
        }
        Ok(cost)
    }
    fn commit(&mut self, p: Prepared, displays: TableDisplays, range: &mut actions::MountRange) {
        // Everything below consumes already validated owned data. No rejectable
        // query or user update callback occurs after the first live mutation.
        let same = self.model.dataset_key() == p.model.dataset_key();
        let reset_focus = displays
            .rows
            .iter()
            .any(|r| r.replace_wrapper && self.view.focused_row_keyed(&self.model) == Some(r.key));
        let mut old_fields = BTreeMap::new();
        let mut old_rows = BTreeMap::new();
        let mut children = std::mem::take(&mut self.children).into_iter();
        for col in &self.model.snapshot().columns {
            if self.headers.get(&col.key).is_some_and(Option::is_some)
                && let Some(c) = children.next()
            {
                old_fields.insert(Field::Header(col.key), c);
            }
        }
        for row in &self.model.snapshot().rows {
            if let Some(mut c) = children.next() {
                if let Some(w) = c.component_mut::<RowWrapper>() {
                    let mut cells = std::mem::take(&mut w.children).into_iter();
                    for (j, col) in self.model.snapshot().columns.iter().enumerate() {
                        for (slot, f) in [
                            (2 * j, Field::Cell(row.key, col.key)),
                            (2 * j + 1, Field::Accessory(row.key, col.key)),
                        ] {
                            if w.slots.get(slot).is_some_and(Option::is_some)
                                && let Some(v) = cells.next()
                            {
                                old_fields.insert(f, v);
                            }
                        }
                    }
                }
                old_rows.insert(row.key, c);
            }
        }
        self.headers.clear();
        self.rows.clear();
        fn slot(
            old: &mut BTreeMap<Field, Content>,
            field: Field,
            u: TableDisplayUpdate,
            same: bool,
            mounted: bool,
            active: bool,
            range: &mut actions::MountRange,
        ) -> Option<Content> {
            if same && matches!(u, TableDisplayUpdate::Keep) {
                return old.remove(&field);
            }
            if let TableDisplayUpdate::Replace(mut c) = u {
                c.suppress_semantics();
                if mounted {
                    c.lifecycle_reserved(Lifecycle::Mounted, range);
                    c.refresh_activity(active);
                }
                Some(c)
            } else {
                None
            }
        }
        for h in displays.headers {
            let content = slot(
                &mut old_fields,
                Field::Header(h.key),
                h.display,
                same,
                self.mounted,
                self.window_active,
                range,
            );
            self.headers
                .insert(h.key, content.as_ref().map(Content::id));
            if let Some(c) = content {
                self.children.push(c);
            }
        }
        for row in displays.rows {
            let retained = if same && !row.replace_wrapper {
                old_rows.remove(&row.key)
            } else {
                None
            };
            let new = retained.is_none();
            let mut wrapper = retained.unwrap_or_else(|| {
                Content::new_with_mount(
                    row.id,
                    RowWrapper {
                        identity: Arc::new(()),
                        slots: vec![],
                        children: vec![],
                        disabled: self.disabled,
                        ink: None,
                    },
                    range.take(),
                )
            });
            let mut slots = Vec::new();
            let mut contents = Vec::new();
            for cell in row.cells {
                for (field, u) in [
                    (Field::Cell(row.key, cell.column), cell.display),
                    (Field::Accessory(row.key, cell.column), cell.accessory),
                ] {
                    let content = slot(
                        &mut old_fields,
                        field,
                        u,
                        same && !row.replace_wrapper,
                        self.mounted && !new,
                        self.window_active,
                        range,
                    );
                    slots.push(content.as_ref().map(Content::id));
                    if let Some(c) = content {
                        contents.push(c);
                    }
                }
            }
            if let Some(w) = wrapper.component_mut::<RowWrapper>() {
                w.slots = slots;
                w.children = contents;
                w.disabled = self.disabled;
                self.rows.insert(row.key, (row.id, w.identity.clone()));
            }
            wrapper.suppress_semantics();
            if self.mounted && new {
                wrapper.lifecycle_reserved(Lifecycle::Mounted, range);
                wrapper.refresh_activity(self.window_active);
            }
            self.children.push(wrapper);
        }
        for mut c in old_fields.into_values() {
            if self.mounted {
                c.lifecycle_reserved(Lifecycle::Unmounted, range);
            }
        }
        for mut c in old_rows.into_values() {
            if self.mounted {
                c.lifecycle_reserved(Lifecycle::Unmounted, range);
            }
        }
        self.model = p.model;
        self.exposures = p.exposures;
        self.next = p.next;
        self.metrics = TableContentMetrics::default();
        self.view.cancel();
        if reset_focus {
            self.view.blur();
        }
        self.update_active();
        debug_assert!(range.exhausted());
    }
    fn update_active(&mut self) {
        let h = self.headers.values().filter(|v| v.is_some()).count();
        for (i, row) in self.model.snapshot().rows.iter().enumerate() {
            if let Some(c) = self.children.get_mut(h + i) {
                c.set_composition_active(
                    self.model.display_ordinal(row.key).is_some()
                        && !self.model.snapshot().columns.is_empty(),
                );
            }
        }
    }
    fn refresh_exposures(&mut self) {
        if let Ok((m, n)) = self.plan_exposures(&self.model) {
            self.exposures = m;
            self.next = n;
        } else {
            self.exhausted = true;
            self.exposures.clear();
        }
    }
    fn publishable(&self) -> bool {
        self.mounted && self.window_active && !self.exhausted && self.validate_all().is_ok()
    }
    fn available(&self) -> bool {
        self.publishable() && !self.disabled
    }
    pub fn intent_is_current(&self, i: &TableIntent) -> bool {
        let part = match i.target {
            TableIntentTarget::Header(c) => Part::Header(c),
            TableIntentTarget::Row(r) => Part::Row(r),
        };
        self.available()
            && Arc::ptr_eq(&i.origin, &self.model.view_identity())
            && i.dataset == self.model.dataset_key()
            && i.activity_epoch == self.activity_epoch
            && self
                .exposures
                .get(&part)
                .is_some_and(|e| e.token == i.exposure)
    }
    fn emit(&mut self, target: TableIntentTarget, kind: TableIntentKind) {
        let p = match target {
            TableIntentTarget::Header(c) => Part::Header(c),
            TableIntentTarget::Row(r) => Part::Row(r),
        };
        self.last_intent = self.exposures.get(&p).map(|e| TableIntent {
            origin: self.model.view_identity(),
            dataset: self.model.dataset_key(),
            target,
            exposure: e.token,
            model_revision: self.model.revision(),
            activity_epoch: self.activity_epoch,
            kind,
            selected_key: self.selected_key(),
            sort: self.sort_key_state(),
        });
    }
    fn response(
        &mut self,
        event: CollectionEvent,
        selected: Option<R>,
        sort: Option<(C, SortDirection)>,
    ) -> ContentResponse {
        if let Some((c, d)) = self.sort_key_state().filter(|s| Some(*s) != sort) {
            self.emit(
                TableIntentTarget::Header(c),
                TableIntentKind::SortChanged { direction: d },
            );
        } else if self.selected_key() != selected {
            if let Some(r) = self.selected_key() {
                self.emit(TableIntentTarget::Row(r), TableIntentKind::SelectionChanged);
            }
        } else if let CollectionEvent::Activated(i) = event
            && let Some(r) = self.model.row_key(i)
        {
            self.emit(TableIntentTarget::Row(r), TableIntentKind::Activated);
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
    fn measured(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        width: f32,
        height: f32,
    ) -> TableContentMetrics {
        let cw = width.max(0.) / self.model.snapshot().columns.len().max(1) as f32;
        let mut result = TableContentMetrics::default();
        let mut i = 0;
        for col in &self.model.snapshot().columns {
            let v = if self.headers.get(&col.key).is_some_and(Option::is_some) {
                let size = self.children[i]
                    .measure(text, Size::new((cw - 16.).max(0.), height))
                    .size;
                i += 1;
                Some(size)
            } else {
                None
            };
            result.headers.push(v);
        }
        for _ in &self.model.snapshot().rows {
            let mut row = Vec::new();
            if let Some(w) = self
                .children
                .get_mut(i)
                .and_then(|c| c.component_mut::<RowWrapper>())
            {
                let mut k = 0;
                let mut refs = Vec::new();
                for id in &w.slots {
                    refs.push(id.map(|_| {
                        let j = k;
                        k += 1;
                        j
                    }));
                }
                for col in 0..self.model.snapshot().columns.len() {
                    let accessory = refs[col * 2 + 1].map(|j| {
                        w.children[j]
                            .measure(text, Size::new((cw - 16.).max(0.), height))
                            .size
                    });
                    let aw = accessory.map_or(0., |s| s.width.min((cw - 16.).max(0.)));
                    let main = refs[col * 2].map(|j| {
                        w.children[j]
                            .measure(text, Size::new((cw - 16. - aw).max(0.), height))
                            .size
                    });
                    row.push((main, accessory));
                }
            }
            result.cells.push(row);
            i += 1;
        }
        result
    }
    fn arrange_slots(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        l: &DataTableLayout,
        clip: Rect,
    ) {
        let own = clip.intersection(&l.frame).unwrap_or_default();
        let mut i = 0;
        for (j, col) in self.model.snapshot().columns.iter().enumerate() {
            if self.headers.get(&col.key).is_some_and(Option::is_some) {
                if let Some(h) = l.headers.get(j) {
                    let f = Rect::new(
                        h.frame.min_x() + 8.,
                        h.frame.min_y(),
                        (h.frame.size.width - 16.).max(0.),
                        h.frame.size.height,
                    );
                    self.children[i].arrange(text, f, own);
                }
                i += 1;
            }
        }
        for row in &self.model.snapshot().rows {
            if let Some(index) = self.model.row_index(row.key)
                && let Some(layout) = l.rows.iter().find(|x| x.id == index)
            {
                let wrapper = &mut self.children[i];
                wrapper.frame = layout.frame;
                wrapper.clip = own;
                if let Some(w) = wrapper.component_mut::<RowWrapper>() {
                    let mut k = 0;
                    for j in 0..self.model.snapshot().columns.len() {
                        let cw = l.columns[j].size.width;
                        let x = l.columns[j].min_x() + 8.;
                        let aw = self.metrics.cells[index][j]
                            .1
                            .map_or(0., |s| s.width.min((cw - 16.).max(0.)));
                        for (slot, f) in [
                            (
                                2 * j,
                                Rect::new(
                                    x,
                                    layout.frame.min_y(),
                                    (cw - 16. - aw).max(0.),
                                    layout.frame.size.height,
                                ),
                            ),
                            (
                                2 * j + 1,
                                Rect::new(
                                    x + (cw - 16. - aw).max(0.),
                                    layout.frame.min_y(),
                                    aw,
                                    layout.frame.size.height,
                                ),
                            ),
                        ] {
                            if w.slots[slot].is_some() {
                                w.children[k].arrange(text, f, own);
                                k += 1;
                            }
                        }
                    }
                }
            }
            i += 1;
        }
    }
}
struct MeasureText<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for MeasureText<'_> {
    fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
        self.0.measure(t, r, w)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.0.text_surface_host()
    }
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(r)
    }
    fn motion_context(&self) -> Option<crate::ControlMotionContext> {
        self.0.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.0.animation_frames_allowed()
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, t: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.0.first_line_height(t, r, w)
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
impl Component for DataTableContent {
    fn host_reveal_passthrough(&self) -> bool {
        self.publishable() && self.view.layout_keyed(&self.model).is_some()
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
    fn validate(&self) -> Result<(), CompositionError> {
        self.validate_all()
            .map_err(|_| CompositionError::InvalidSlotCount)
    }
    fn child_theme(&self, t: ContentTheme) -> ContentTheme {
        if !self.disabled {
            return t;
        }
        let c = crate::paint::button_surface_colors(
            t.palette,
            if t.surface() == "base" {
                "canvas"
            } else {
                t.surface()
            },
            crate::ControlState {
                disabled: true,
                ..Default::default()
            },
            crate::paint::ButtonVariant::Quiet,
            0.,
            0.,
        );
        ContentTheme {
            foreground: Some(c.text),
            icon_foreground: Some(c.icon),
            ..t
        }
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        if self.validate_traversal().is_ok() {
            &self.children
        } else {
            &[]
        }
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.validate_traversal()
            .is_ok()
            .then_some(&mut self.children)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        self.available()
            && (!self.model.display_indices().is_empty()
                || !self.model.snapshot().columns.is_empty())
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
            .map(ContentActionPayload::TableIntent)
    }
    fn action_payload_current(&self, p: &ContentActionPayload) -> bool {
        if let ContentActionPayload::TableIntent(i) = p {
            self.intent_is_current(i)
        } else {
            true
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        if self.window_active != active {
            if !active {
                self.bump_epoch();
                self.exposures.clear();
                self.view.invalidate_content_layout();
            } else {
                self.refresh_exposures();
            }
            self.window_active = active;
            self.view.set_window_active(active);
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.last_intent = None;
        self.view.invalidate_content_layout();
        match event {
            Lifecycle::Mounted => {
                self.mounted = true;
                self.refresh_exposures();
            }
            Lifecycle::Unmounted => {
                self.mounted = false;
                self.exposures.clear();
                self.view.blur();
            }
        }
    }
    fn set_focus(&mut self, focused: bool, keyboard: bool) {
        if !focused {
            self.view.blur();
        } else if self.available() {
            if let Some(column) = self.view.focused_header_keyed(&self.model) {
                self.view.focus_header_keyed(column, keyboard, &self.model);
            } else if let Some(row) = self.view.focused_row_keyed(&self.model) {
                self.view.focus_row_keyed(row, keyboard, &self.model);
            } else if !self.view.focus_keyed(keyboard, &self.model)
                && let Some(column) = self.model.snapshot().columns.first()
            {
                self.view
                    .focus_header_keyed(column.key, keyboard, &self.model);
            }
        }
    }
    fn accessibility_focus_requested(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        if self.validate_all().is_err() {
            return MeasuredChild::new(Size::default());
        }
        let m = self.measured(text, a.width, a.height);
        let mut view = self.view.clone();
        let l = view.render_content_keyed(
            &mut Scene::new(),
            &mut MeasureText(text),
            &self.model,
            Rect::new(0., 0., a.width, 1e8),
            None,
            tekes_ui_tokens::Theme::LIGHT.into(),
            m,
        );
        MeasuredChild::new(Size::new(a.width, l.content_height))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        if self.validate_all().is_err() {
            return;
        }
        self.metrics = self.measured(text, f.size.width, f.size.height);
        let l = self.view.render_content_keyed(
            &mut Scene::new(),
            &mut MeasureText(text),
            &self.model,
            f,
            Some(c),
            tekes_ui_tokens::Theme::LIGHT.into(),
            self.metrics.clone(),
        );
        self.arrange_slots(text, &l, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        t: ContentTheme,
    ) {
        if !self.mounted || self.exhausted || self.validate_all().is_err() {
            return;
        }
        if let Some(l) = self.view.layout_keyed(&self.model) {
            let frame = l.frame;
            let mut metrics = self.metrics.clone();
            metrics.surface = t.surface_declared.then_some(t.surface());
            self.view.render_content_keyed(
                s,
                &mut TextAdapter(text),
                &self.model,
                frame,
                Some(c),
                t.palette,
                metrics,
            );
        }
        let h = self.headers.values().filter(|v| v.is_some()).count();
        for i in 0..self.model.snapshot().rows.len() {
            let ink = self.view.content_row_ink(i, t.palette, t.surface());
            if let Some(w) = self
                .children
                .get_mut(h + i)
                .and_then(|c| c.component_mut::<RowWrapper>())
            {
                w.ink = ink;
            }
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Table, &self.name);
        s.disabled =
            self.disabled || !self.window_active || self.exhausted || self.validate_all().is_err();
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.publishable() {
            return vec![];
        }
        let Some(l) = self.view.layout_keyed(&self.model) else {
            return vec![];
        };
        let mut out = vec![];
        let mut push =
            |part: Part, frame: Rect, visible: Option<Rect>, s: ContentSemantics, focused: bool| {
                if let Some(e) = self.exposures.get(&part) {
                    out.push(SemanticPart {
                        index: e.token,
                        frame: visible.unwrap_or(Rect::new(frame.min_x(), frame.min_y(), 0., 0.)),
                        focused: visible.is_some() && focused,
                        semantics: {
                            let mut s = s;
                            s.disabled |= self.disabled;
                            s
                        },
                    });
                }
            };
        for (j, col) in self.model.snapshot().columns.iter().enumerate() {
            let frame = l.columns[j];
            push(
                Part::Column(col.key),
                frame,
                frame.intersection(&l.visible.unwrap_or_default()),
                ContentSemantics::new(ContentRole::TableColumn { index: j }, &col.name),
                false,
            );
            if let Some(h) = l.headers.get(j) {
                push(
                    Part::Header(col.key),
                    h.frame,
                    h.visible,
                    ContentSemantics::new(
                        ContentRole::TableColumnHeader {
                            index: j,
                            sort: self
                                .sort_key_state()
                                .filter(|(k, _)| *k == col.key)
                                .map(|(_, d)| d),
                        },
                        &col.name,
                    ),
                    self.view.focused_header_keyed(&self.model) == Some(col.key),
                );
            }
        }
        for (ordinal, &i) in self.model.display_indices().iter().enumerate() {
            let row = &self.model.snapshot().rows[i];
            if let Some(r) = l.rows.iter().find(|r| r.id == i) {
                let mut s =
                    ContentSemantics::new(ContentRole::TableRow { index: ordinal }, &row.name);
                s.selected = self.selected_key() == Some(row.key);
                push(
                    Part::Row(row.key),
                    r.frame,
                    r.visible,
                    s,
                    self.view.focused_row_keyed(&self.model) == Some(row.key),
                );
                for (j, col) in self.model.snapshot().columns.iter().enumerate() {
                    let frame = Rect::new(
                        l.columns[j].min_x(),
                        r.frame.min_y(),
                        l.columns[j].size.width,
                        r.frame.size.height,
                    );
                    let mut s = ContentSemantics::new(
                        ContentRole::TableCell {
                            row: ordinal,
                            column: j,
                        },
                        "",
                    );
                    s.value = Some(row.cells[j].clone());
                    push(
                        Part::Cell(row.key, col.key),
                        frame,
                        frame.intersection(&r.visible.unwrap_or_default()),
                        s,
                        false,
                    );
                }
            }
        }
        out
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        if !self.available() || self.view.layout_keyed(&self.model).is_none() {
            return None;
        }
        let p = self.exposures.iter().find(|(_, e)| e.token == part)?.0;
        let parent = match p {
            Part::Header(c) => Part::Column(*c),
            Part::Cell(r, _) => Part::Row(*r),
            _ => return None,
        };
        self.exposures.get(&parent).map(|e| e.token)
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
        let selected = self.selected_key();
        let sort = self.sort_key_state();
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
        let mut r = self.response(event, selected, sort);
        r.capture_pointer = matches!(e, ContentEvent::PointerDown(_)) && r.consumed;
        r
    }
}
impl ContentHost {
    /// One synchronous typed update: the full target and reader are staged before
    /// borrowing the original disjoint fields, and neither commit can fail.
    /// All inputs are borrowed through staging. Any Err leaves snapshot, displays
    /// and reading unchanged; success alone takes displays. Caller destruction of
    /// a refused Unknown tree is outside this transaction and may run dynamic Drop.
    pub fn replace_table_snapshot(
        &mut self,
        id: ComponentId,
        s: &KeyedTableSnapshot,
        d: &mut TableDisplays,
        reading: Option<&TableReadingUpdate>,
    ) -> Result<Option<ReadingDocumentVersion>, TableContentError> {
        if self.reading_transaction {
            return Err(CompositionError::ReadingTransactionInProgress.into());
        }
        let table_node = self
            .root
            .find(id)
            .ok_or(TableContentError::InvalidTarget(id))?;
        let table_mount = table_node.mount;
        let table = table_node
            .component::<DataTableContent>()
            .ok_or(TableContentError::InvalidTarget(id))?;
        fn outside_count(c: &Content, target: ComponentId) -> Result<usize, TableContentError> {
            let mut n = 1;
            if c.id() != target {
                for child in c.component.children() {
                    n = add(n, outside_count(child, target)?)?;
                }
            }
            Ok(n)
        }
        let outer = outside_count(&self.root, id)?;
        let mut extra = if let Some(r) = reading {
            let scroll = self
                .root
                .find(r.owner)
                .and_then(|owner| owner.component::<ScrollContent>())
                .ok_or(TableContentError::ReadingOwner)?;
            let state = scroll
                .reading
                .as_ref()
                .ok_or(ReadingUpdateError::WrongDocument)?;
            content_reading::reading_plan_cost(state, &r.expected, r.items.capacity())
                .ok_or(TableContentError::Overflow)?
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
        fn access_path(
            c: &Content,
            target: ComponentId,
            path: &mut Vec<usize>,
        ) -> Result<bool, TableContentError> {
            if c.id() == target {
                return Ok(true);
            }
            for (index, child) in c.component.children().iter().enumerate() {
                path.push(index);
                if access_path(child, target, path)? {
                    // This stateless built-in organization wrapper exposes its
                    // original child vector; unknown custom components still
                    // need to opt in explicitly.
                    if c.component.table_transaction_compatibility()
                        != TableTransactionCompatibility::StableHooks
                        && c.component::<ListContent>().is_none()
                    {
                        return Err(TableContentError::UnknownTransactionHooks { child: c.id() });
                    }
                    return Ok(true);
                }
                path.pop();
            }
            Ok(false)
        }
        fn at_path_mut<'a>(c: &'a mut Content, path: &[usize]) -> Option<&'a mut Content> {
            if let Some((index, rest)) = path.split_first() {
                at_path_mut(c.component.children_mut()?.get_mut(*index)?, rest)
            } else {
                Some(c)
            }
        }
        let mut table_path = Vec::with_capacity(outer);
        if !access_path(&self.root, id, &mut table_path)? {
            return Err(TableContentError::InvalidTarget(id));
        }
        let mut owner_path = Vec::with_capacity(outer);
        if let Some(r) = reading
            && (!access_path(&self.root, r.owner, &mut owner_path)?
                || !table_path.starts_with(&owner_path)
                || table_path.len() == owner_path.len())
        {
            return Err(TableContentError::ReadingOwner);
        }

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
        for i in &p.ids {
            if !ids.insert(*i) {
                return Err(CompositionError::DuplicateId(*i).into());
            }
        }
        if let Some(r) = reading {
            let owner = self
                .root
                .find(r.owner)
                .ok_or(TableContentError::ReadingOwner)?;
            let owner_mount = owner.mount;
            let scroll = owner
                .component::<ScrollContent>()
                .ok_or(TableContentError::ReadingOwner)?;
            if !scroll.children.iter().any(|c| c.find(id).is_some()) {
                return Err(TableContentError::ReadingOwner);
            }
            let expected = p.model.display_indices().iter().map(|i| d.rows[*i].id);
            let is_row = |item: &ComponentId| d.rows.iter().any(|row| row.id == *item);
            if expected.ne(r.items.iter().copied().filter(is_row)) {
                return Err(TableContentError::ReadingItems);
            }
            // Other declared items belong to unchanged branches of this same
            // reader. Preserve their order; table decorations cannot masquerade
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
                return Err(TableContentError::ReadingItems);
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
            let owner =
                at_path_mut(&mut self.root, &owner_path).ok_or(TableContentError::ReadingOwner)?;
            if owner.mount != owner_mount {
                return Err(TableContentError::ReadingOwner);
            }
            let scroll = owner
                .component_mut::<ScrollContent>()
                .ok_or(TableContentError::ReadingOwner)?;
            let ScrollContent {
                children, reading, ..
            } = scroll;
            let state = reading.as_mut().ok_or(ReadingUpdateError::WrongDocument)?;
            // The owner is a proven strict ancestor. Walk only the selected
            // child and suffix, never a sibling's mutable getter.
            let (index, suffix) = table_path[owner_path.len()..]
                .split_first()
                .ok_or(TableContentError::ReadingOwner)?;
            let node = children
                .get_mut(*index)
                .and_then(|child| at_path_mut(child, suffix))
                .ok_or(TableContentError::InvalidTarget(id))?;
            if node.mount != table_mount {
                return Err(TableContentError::InvalidTarget(id));
            }
            let table = node
                .component_mut::<DataTableContent>()
                .ok_or(TableContentError::InvalidTarget(id))?;
            content_reading::validate_reading_current(state, &r.expected)?;
            let mut range =
                actions::reserve_mounts(p.mounts).ok_or(TableContentError::MountCapacity)?;
            table.commit(p, std::mem::take(d), &mut range);
            let result = reader.commit(state);
            self.reveal_clock.invalidate_input();
            self.reveal_clock.pending = true;
            self.dirty = true;
            Ok(Some(result))
        } else {
            // A table inside a declared reading document cannot silently change
            // ordered row identities without the typed reading update.
            fn declared(c: &Content, target: ComponentId) -> bool {
                if let Some(s) = c.component::<ScrollContent>()
                    && s.reading_document_version().is_some()
                    && s.children.iter().any(|c| c.find(target).is_some())
                {
                    return true;
                }
                c.component.children().iter().any(|c| declared(c, target))
            }
            if declared(&self.root, id) {
                return Err(TableContentError::HostTransactionRequired);
            }
            let node = at_path_mut(&mut self.root, &table_path)
                .ok_or(TableContentError::InvalidTarget(id))?;
            if node.mount != table_mount {
                return Err(TableContentError::InvalidTarget(id));
            }
            let table = node
                .component_mut::<DataTableContent>()
                .ok_or(TableContentError::InvalidTarget(id))?;
            let mut range =
                actions::reserve_mounts(p.mounts).ok_or(TableContentError::MountCapacity)?;
            table.commit(p, std::mem::take(d), &mut range);
            self.reveal_clock.invalidate_input();
            self.reveal_clock.pending = true;
            self.dirty = true;
            Ok(None)
        }
    }
}

#[derive(Debug, PartialEq)]
struct PreparedOwnerAction {
    identity: action_preparation::OwnerIdentity,
    revision: u64,
    activity: u64,
    snapshot: crate::collections_view::TableActionSnapshot,
    action: Option<crate::collections_view::CollectionAction>,
    response: ContentResponse,
    before: Option<R>,
    sort: Option<(C, SortDirection)>,
}
impl DataTableContent {
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
                if self.model.revision() == u64::MAX {
                    return None;
                }
                self.view.prepare_table_down(*point, &self.model)
            }
            action_preparation::OwnedActionRequest::Accessibility(part, action) => {
                if !matches!(
                    action,
                    AccessibilityAction::Focus | AccessibilityAction::Press
                ) {
                    return None;
                }
                let exposed = self
                    .semantic_parts()
                    .iter()
                    .any(|p| Some(p.index) == *part && !p.frame.is_empty());
                if exposed {
                    self.exposures
                        .iter()
                        .find(|(_, e)| Some(e.token) == *part)
                        .and_then(|(p, _)| match *p {
                            Part::Header(k) => self.model.column_index(k).and_then(|i| {
                                self.view.prepare_table_part(
                                    i,
                                    true,
                                    matches!(action, AccessibilityAction::Press),
                                    &self.model,
                                )
                            }),
                            Part::Row(k) => self.model.row_index(k).and_then(|i| {
                                self.view.prepare_table_part(
                                    i,
                                    false,
                                    matches!(action, AccessibilityAction::Press),
                                    &self.model,
                                )
                            }),
                            _ => None,
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
            snapshot: self.view.table_action_snapshot(),
            action,
            response,
            before: self.selected_key(),
            sort: self.sort_key_state(),
        })
    }
    fn apply_action(&mut self, d: PreparedOwnerAction) {
        self.last_intent = None;
        if let Some(action) = d.action {
            let event = action.event;
            self.model.apply_prepared_action(&mut self.view, action);
            self.response(event, d.before, d.sort);
        }
    }
}

impl action_preparation::OriginalActionDecision for PreparedOwnerAction {}

#[cfg(test)]
mod datatable_private_tests {
    use super::*;
    fn s(n: u64) -> KeyedTableSnapshot {
        KeyedTableSnapshot {
            dataset: D(1),
            columns: vec![crate::extended::KeyedTableColumn {
                key: C(1),
                name: "c".into(),
            }],
            rows: (1..=n)
                .map(|k| crate::extended::KeyedTableRow {
                    key: R(k),
                    name: "r".into(),
                    cells: vec!["v".into()],
                })
                .collect(),
            display_rows: (1..=n).map(R).collect(),
        }
    }
    fn d(s: &KeyedTableSnapshot) -> TableDisplays {
        TableDisplays {
            headers: s
                .columns
                .iter()
                .map(|c| TableHeaderDisplay {
                    key: c.key,
                    display: TableDisplayUpdate::Keep,
                })
                .collect(),
            rows: s
                .rows
                .iter()
                .map(|r| TableRowDisplay {
                    key: r.key,
                    id: ComponentId(100 + r.key.0),
                    replace_wrapper: false,
                    cells: s
                        .columns
                        .iter()
                        .map(|c| TableCellDisplay {
                            column: c.key,
                            display: TableDisplayUpdate::Keep,
                            accessory: TableDisplayUpdate::Keep,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
    fn table() -> DataTableContent {
        let s = s(2);
        DataTableContent::new(
            "t",
            &s.clone(),
            &mut d(&s),
            TableBudget {
                max_units: 10000,
                max_utf8_bytes: 1_000_000,
            },
        )
        .unwrap()
    }
    #[test]
    fn datatable_token_capacity_and_checked_budget_errors_are_atomic() {
        let mut t = table();
        t.next = usize::MAX;
        let before = format!("{t:?}");
        let s = s(3);
        assert_eq!(
            t.replace_snapshot(&s.clone(), &mut d(&s)),
            Err(TableContentError::TokenCapacity)
        );
        assert_eq!(format!("{t:?}"), before);
        assert_eq!(mul(usize::MAX, 2), Err(TableContentError::Overflow));
        assert_eq!(add(usize::MAX, 1), Err(TableContentError::Overflow));
        let mut s = s;
        s.rows[0].cells[0] = "x".repeat(1_000_001);
        assert_eq!(
            t.replace_snapshot(&s.clone(), &mut d(&s)),
            Err(TableContentError::BudgetExceeded)
        );
        assert_eq!(format!("{t:?}"), before);
    }
    #[test]
    fn datatable_disabled_capacity_failure_preserves_all_state() {
        let mut t = table();
        t.next = usize::MAX;
        let before = format!("{t:?}");
        assert_eq!(t.set_disabled(true), Err(TableContentError::TokenCapacity));
        assert_eq!(format!("{t:?}"), before);
        t.next = 10;
        t.activity_epoch = u64::MAX;
        let before = format!("{t:?}");
        assert_eq!(t.set_disabled(true), Err(TableContentError::TokenCapacity));
        assert_eq!(format!("{t:?}"), before);
    }
    #[test]
    fn datatable_wrapper_registry_rejects_count_type_id_marker_and_slot_tampering() {
        for case in 0..5 {
            let mut t = table();
            match case {
                0 => {
                    t.children.pop();
                }
                1 => {
                    t.children[0] = Content::new(ComponentId(101), TextContent::new("fake wrapper"))
                }
                2 => t.children[0].id = ComponentId(999),
                3 => {
                    t.children[0]
                        .component_mut::<RowWrapper>()
                        .unwrap()
                        .identity = Arc::new(())
                }
                _ => t.children[0]
                    .component_mut::<RowWrapper>()
                    .unwrap()
                    .slots
                    .clear(),
            };
            let before = format!("{t:?}");
            assert!(matches!(
                t.validate_structure(),
                Err(TableContentError::InvalidStructure)
            ));
            assert!(t.semantic_parts().is_empty());
            assert!(t.children().is_empty());
            assert!(t.children_mut().is_none());
            let s = s(2);
            assert_eq!(
                t.replace_snapshot(&s.clone(), &mut d(&s)),
                Err(TableContentError::InvalidStructure)
            );
            assert_eq!(format!("{t:?}"), before);
        }
    }
    #[test]
    fn datatable_activity_overflow_stays_closed_without_reset() {
        let mut t = table();
        t.mounted = true;
        t.activity_epoch = u64::MAX;
        t.window_active_changed(false);
        assert!(t.exhausted);
        t.window_active_changed(true);
        assert!(!t.available());
    }

    #[test]
    fn typed_table_updates_revoke_host_geometry_only_after_success() {
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
        for reading in [false, true] {
            let mut h = ContentHost::new(Content::new(
                ComponentId(1),
                ScrollContent::new(
                    crate::ScrollArea::default(),
                    Content::new(ComponentId(10), table()),
                ),
            ))
            .unwrap();
            if reading {
                h.set_scroll_reading_document(
                    ComponentId(1),
                    "rows",
                    1,
                    vec![ComponentId(101), ComponentId(102)],
                    |_| (),
                )
                .unwrap();
            }
            let frame = Rect::new(0., 0., 300., 200.);
            h.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                tekes_ui_tokens::Theme::LIGHT.into(),
            )
            .unwrap();
            let snap = s(2);
            let update = reading.then(|| TableReadingUpdate {
                owner: ComponentId(1),
                expected: h
                    .get(ComponentId(1))
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .reading_document_version()
                    .unwrap()
                    .clone(),
                revision: 2,
                items: vec![ComponentId(101), ComponentId(102)],
            });
            let lease = h.reveal_clock.geometry_lease.clone();
            let old_lease = lease.load(Ordering::Acquire);
            h.replace_table_snapshot(ComponentId(10), &snap, &mut d(&snap), update.as_ref())
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
            if let Some(stale) = update.as_ref() {
                let before = format!("{h:?}");
                let current_lease = lease.load(Ordering::Acquire);
                assert!(
                    h.replace_table_snapshot(ComponentId(10), &snap, &mut d(&snap), Some(stale))
                        .is_err()
                );
                assert_eq!(lease.load(Ordering::Acquire), current_lease);
                assert_eq!(format!("{h:?}"), before);
            }
            let before = format!("{h:?}");
            let current_lease = lease.load(Ordering::Acquire);
            let mut bad = snap.clone();
            bad.display_rows.push(R(999));
            let refused = reading.then(|| TableReadingUpdate {
                owner: ComponentId(1),
                expected: h
                    .get(ComponentId(1))
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .reading_document_version()
                    .unwrap()
                    .clone(),
                revision: 3,
                items: vec![ComponentId(101), ComponentId(102)],
            });
            let mut displays = d(&bad);
            let before_displays = format!("{displays:?}");
            assert!(
                h.replace_table_snapshot(ComponentId(10), &bad, &mut displays, refused.as_ref())
                    .is_err()
            );
            assert_eq!(lease.load(Ordering::Acquire), current_lease);
            assert_eq!(format!("{h:?}"), before);
            assert_eq!(format!("{displays:?}"), before_displays);
        }
    }
}
