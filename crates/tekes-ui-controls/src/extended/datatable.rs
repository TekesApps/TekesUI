use crate::ControlKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataTable {
    pub selected_row: Option<usize>,
    pub sort: Option<(usize, SortDirection)>,
    pub rows: usize,
    pub columns: usize,
}
impl DataTable {
    pub fn new(rows: usize, columns: usize) -> Self {
        Self {
            selected_row: None,
            sort: None,
            rows,
            columns,
        }
    }
    pub fn select_row(&mut self, row: usize) -> bool {
        if row >= self.rows || self.selected_row == Some(row) {
            return false;
        }
        self.selected_row = Some(row);
        true
    }
    pub(crate) fn decided_sort(&self, column: usize) -> Option<(usize, SortDirection)> {
        if column >= self.columns {
            return None;
        }
        Some(match self.sort {
            Some((c, SortDirection::Ascending)) if c == column => {
                (column, SortDirection::Descending)
            }
            _ => (column, SortDirection::Ascending),
        })
    }
    pub fn sort_by(&mut self, column: usize) -> bool {
        let Some(next) = self.decided_sort(column) else {
            return false;
        };
        self.sort = Some(next);
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.rows == 0 {
            return false;
        }
        let row = match key {
            ControlKey::Up => self.selected_row.unwrap_or(self.rows).saturating_sub(1),
            ControlKey::Down => self.selected_row.map_or(0, |r| (r + 1).min(self.rows - 1)),
            ControlKey::Home => 0,
            ControlKey::End => self.rows - 1,
            _ => return false,
        };
        self.select_row(row)
    }
}

/// Caller-assigned dataset identity for a finite, flat table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableDatasetKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableRowKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableColumnKey(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTableColumn {
    pub key: TableColumnKey,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTableRow {
    pub key: TableRowKey,
    pub name: String,
    pub cells: Vec<String>,
}
/// A complete flat snapshot. Display order is a unique subset of row keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTableSnapshot {
    pub dataset: TableDatasetKey,
    pub columns: Vec<KeyedTableColumn>,
    pub rows: Vec<KeyedTableRow>,
    pub display_rows: Vec<TableRowKey>,
}
/// Mandatory finite model limits. Counts include retained hidden rows/cells.
/// Content/Host consumers must additionally budget their own slots and descendants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableBudget {
    pub max_units: usize,
    pub max_utf8_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableUpdateError {
    InvalidBudget,
    DuplicateRow(TableRowKey),
    DuplicateColumn(TableColumnKey),
    DuplicateDisplayRow(TableRowKey),
    MissingDisplayRow(TableRowKey),
    CellCount(TableRowKey),
    BudgetExceeded,
    Overflow,
    CounterExhausted,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableReconcile {
    pub old_rows_to_new: Vec<Option<usize>>,
    pub old_columns_to_new: Vec<Option<usize>>,
    pub revision: u64,
}
/// Owns the sole legacy reducer and its stable identity metadata. There is no
/// mutable legacy escape hatch; `original()` is diagnostic access only.
#[derive(Debug)]
pub struct KeyedDataTable {
    original: DataTable,
    snapshot: KeyedTableSnapshot,
    budget: TableBudget,
    row_indices: std::collections::BTreeMap<TableRowKey, usize>,
    column_indices: std::collections::BTreeMap<TableColumnKey, usize>,
    display_indices: Vec<usize>,
    row_incarnations: Vec<u64>,
    column_incarnations: Vec<u64>,
    identity: std::sync::Arc<()>,
    revision: u64,
    next_incarnation: u64,
}
impl Clone for KeyedDataTable {
    fn clone(&self) -> Self {
        Self {
            original: self.original.clone(),
            snapshot: self.snapshot.clone(),
            budget: self.budget,
            row_indices: self.row_indices.clone(),
            column_indices: self.column_indices.clone(),
            display_indices: self.display_indices.clone(),
            row_incarnations: self.row_incarnations.clone(),
            column_incarnations: self.column_incarnations.clone(),
            identity: std::sync::Arc::new(()),
            revision: self.revision,
            next_incarnation: self.next_incarnation,
        }
    }
}
impl KeyedDataTable {
    pub fn new(
        snapshot: KeyedTableSnapshot,
        budget: TableBudget,
    ) -> Result<Self, TableUpdateError> {
        let mut table = Self::stage(snapshot, budget)?;
        table.assign_incarnations(None)?;
        Ok(table)
    }
    fn model_units(
        rows: usize,
        columns: usize,
        displayed: usize,
    ) -> Result<usize, TableUpdateError> {
        // Include the display mapping as well as R+C+R*C; no multiplication or
        // sum is saturating, including zero-column and empty snapshots.
        rows.checked_mul(columns)
            .and_then(|m| m.checked_add(rows))
            .and_then(|n| n.checked_add(columns))
            .and_then(|n| n.checked_add(displayed))
            .ok_or(TableUpdateError::Overflow)
    }
    fn stage(snapshot: KeyedTableSnapshot, budget: TableBudget) -> Result<Self, TableUpdateError> {
        if budget.max_units == 0 || budget.max_utf8_bytes == 0 {
            return Err(TableUpdateError::InvalidBudget);
        }
        let units = Self::model_units(
            snapshot.rows.len(),
            snapshot.columns.len(),
            snapshot.display_rows.len(),
        )?;
        if units > budget.max_units {
            return Err(TableUpdateError::BudgetExceeded);
        }
        let mut bytes = 0usize;
        for s in snapshot.columns.iter().map(|c| &c.name).chain(
            snapshot
                .rows
                .iter()
                .flat_map(|r| std::iter::once(&r.name).chain(r.cells.iter())),
        ) {
            bytes = bytes
                .checked_add(s.len())
                .ok_or(TableUpdateError::Overflow)?;
            if bytes > budget.max_utf8_bytes {
                return Err(TableUpdateError::BudgetExceeded);
            }
        }
        let mut row_indices = std::collections::BTreeMap::new();
        let mut column_indices = std::collections::BTreeMap::new();
        for (i, c) in snapshot.columns.iter().enumerate() {
            if column_indices.insert(c.key, i).is_some() {
                return Err(TableUpdateError::DuplicateColumn(c.key));
            }
        }
        for (i, r) in snapshot.rows.iter().enumerate() {
            if r.cells.len() != snapshot.columns.len() {
                return Err(TableUpdateError::CellCount(r.key));
            }
            if row_indices.insert(r.key, i).is_some() {
                return Err(TableUpdateError::DuplicateRow(r.key));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut display_indices = Vec::new();
        for key in &snapshot.display_rows {
            let Some(i) = row_indices.get(key) else {
                return Err(TableUpdateError::MissingDisplayRow(*key));
            };
            if !seen.insert(*key) {
                return Err(TableUpdateError::DuplicateDisplayRow(*key));
            }
            display_indices.push(*i);
        }
        Ok(Self {
            original: DataTable::new(snapshot.rows.len(), snapshot.columns.len()),
            row_incarnations: vec![0; snapshot.rows.len()],
            column_incarnations: vec![0; snapshot.columns.len()],
            snapshot,
            budget,
            row_indices,
            column_indices,
            display_indices,
            identity: std::sync::Arc::new(()),
            revision: 0,
            next_incarnation: 0,
        })
    }
    fn assign_incarnations(&mut self, old: Option<&Self>) -> Result<(), TableUpdateError> {
        let old = old.filter(|o| o.dataset_key() == self.dataset_key());
        for i in 0..self.snapshot.rows.len() {
            let retained = old
                .and_then(|o| o.row_index(self.snapshot.rows[i].key).map(|j| (o, j)))
                .filter(|(o, j)| o.row_displayed(*j) == self.row_displayed(i));
            self.row_incarnations[i] = if let Some((o, j)) = retained {
                o.row_incarnations[j]
            } else {
                self.next_incarnation()?
            };
        }
        for i in 0..self.snapshot.columns.len() {
            self.column_incarnations[i] = if let Some((o, j)) =
                old.and_then(|o| o.column_index(self.snapshot.columns[i].key).map(|j| (o, j)))
            {
                o.column_incarnations[j]
            } else {
                self.next_incarnation()?
            };
        }
        Ok(())
    }
    fn next_incarnation(&mut self) -> Result<u64, TableUpdateError> {
        self.next_incarnation = self
            .next_incarnation
            .checked_add(1)
            .ok_or(TableUpdateError::CounterExhausted)?;
        Ok(self.next_incarnation)
    }
    pub fn original(&self) -> &DataTable {
        &self.original
    }
    pub fn snapshot(&self) -> &KeyedTableSnapshot {
        &self.snapshot
    }
    pub fn budget(&self) -> TableBudget {
        self.budget
    }
    pub fn dataset_key(&self) -> TableDatasetKey {
        self.snapshot.dataset
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn row_key(&self, index: usize) -> Option<TableRowKey> {
        self.snapshot.rows.get(index).map(|r| r.key)
    }
    pub fn column_key(&self, index: usize) -> Option<TableColumnKey> {
        self.snapshot.columns.get(index).map(|c| c.key)
    }
    pub fn row_index(&self, key: TableRowKey) -> Option<usize> {
        self.row_indices.get(&key).copied()
    }
    pub fn column_index(&self, key: TableColumnKey) -> Option<usize> {
        self.column_indices.get(&key).copied()
    }
    pub fn display_ordinal(&self, key: TableRowKey) -> Option<usize> {
        self.snapshot.display_rows.iter().position(|k| *k == key)
    }
    pub fn display_indices(&self) -> &[usize] {
        if self.original.columns == 0 {
            &[]
        } else {
            &self.display_indices
        }
    }
    pub fn selected_key(&self) -> Option<TableRowKey> {
        self.original.selected_row.and_then(|i| self.row_key(i))
    }
    pub fn sort_key_state(&self) -> Option<(TableColumnKey, SortDirection)> {
        self.original
            .sort
            .and_then(|(i, d)| self.column_key(i).map(|k| (k, d)))
    }
    pub fn select_key(&mut self, key: TableRowKey) -> bool {
        self.row_index(key)
            .filter(|i| self.row_displayed(*i))
            .is_some_and(|i| self.original.select_row(i))
    }
    pub fn sort_key(&mut self, key: TableColumnKey) -> bool {
        let Some(i) = self.column_index(key) else {
            return false;
        };
        let Some(next) = self.revision.checked_add(1) else {
            return false;
        };
        let changed = self.original.sort_by(i);
        if changed {
            self.revision = next;
        }
        changed
    }
    pub fn replace_keyed(
        &mut self,
        snapshot: KeyedTableSnapshot,
    ) -> Result<TableReconcile, TableUpdateError> {
        let (staged, result) = self.prepare_replace(snapshot)?;
        *self = staged;
        Ok(result)
    }
    pub(crate) fn prepare_replace(
        &self,
        snapshot: KeyedTableSnapshot,
    ) -> Result<(Self, TableReconcile), TableUpdateError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TableUpdateError::CounterExhausted)?;
        let mut staged = Self::stage(snapshot, self.budget)?;
        staged.next_incarnation = self.next_incarnation;
        staged.assign_incarnations(Some(self))?;
        let same = staged.dataset_key() == self.dataset_key();
        if same {
            if let Some(i) = self
                .selected_key()
                .and_then(|k| staged.row_index(k))
                .filter(|i| staged.row_displayed(*i))
            {
                staged.original.select_row(i);
            }
            staged.original.sort = self
                .sort_key_state()
                .and_then(|(k, d)| staged.column_index(k).map(|i| (i, d)));
        }
        let result = TableReconcile {
            old_rows_to_new: self
                .snapshot
                .rows
                .iter()
                .map(|r| same.then(|| staged.row_index(r.key)).flatten())
                .collect(),
            old_columns_to_new: self
                .snapshot
                .columns
                .iter()
                .map(|c| same.then(|| staged.column_index(c.key)).flatten())
                .collect(),
            revision,
        };
        staged.revision = revision;
        staged.identity = self.identity.clone();
        Ok((staged, result))
    }
    pub(crate) fn row_displayed(&self, index: usize) -> bool {
        self.original.columns > 0 && self.display_indices.contains(&index)
    }
    pub(crate) fn view_identity(&self) -> std::sync::Arc<()> {
        self.identity.clone()
    }
    pub(crate) fn row_incarnation(&self, index: usize) -> Option<u64> {
        self.row_incarnations.get(index).copied()
    }
    pub(crate) fn column_incarnation(&self, index: usize) -> Option<u64> {
        self.column_incarnations.get(index).copied()
    }
    // Short internal borrow only: no reference or user closure escapes this reducer.
    pub(crate) fn apply_prepared_action(
        &mut self,
        view: &mut crate::collections_view::DataTableView,
        p: crate::collections_view::CollectionAction,
    ) {
        let sort = p.sort;
        view.apply_table_action(p, &mut self.original);
        if sort {
            self.revision = self
                .revision
                .checked_add(1)
                .expect("validated table revision");
            view.guard_table(self);
        }
    }
    pub(crate) fn view_input(
        &mut self,
        view: &mut crate::collections_view::DataTableView,
        input: crate::collections_view::TableInput,
    ) -> crate::collections_view::CollectionEvent {
        let Some(next) = self.revision.checked_add(1) else {
            return crate::collections_view::CollectionEvent::Ignored;
        };
        let sort = self.original.sort;
        let event = view.reduce_table(input, &mut self.original);
        if sort != self.original.sort {
            self.revision = next;
        }
        event
    }
}

#[cfg(test)]
mod datatable_counter_tests {
    use super::*;
    fn table() -> KeyedDataTable {
        KeyedDataTable::new(
            KeyedTableSnapshot {
                dataset: TableDatasetKey(1),
                columns: vec![KeyedTableColumn {
                    key: TableColumnKey(1),
                    name: "C".into(),
                }],
                rows: vec![KeyedTableRow {
                    key: TableRowKey(1),
                    name: "R".into(),
                    cells: vec!["v".into()],
                }],
                display_rows: vec![TableRowKey(1)],
            },
            TableBudget {
                max_units: 100,
                max_utf8_bytes: 100,
            },
        )
        .unwrap()
    }
    #[test]
    fn checked_budget_and_counter_failures_are_atomic() {
        assert_eq!(
            KeyedDataTable::model_units(usize::MAX, 2, 0),
            Err(TableUpdateError::Overflow)
        );
        assert_eq!(
            KeyedDataTable::model_units(usize::MAX, 0, 1),
            Err(TableUpdateError::Overflow)
        );
        for revision in [true, false] {
            let mut t = table();
            t.select_key(TableRowKey(1));
            if revision {
                t.revision = u64::MAX;
            } else {
                t.next_incarnation = u64::MAX;
            }
            let before = format!("{t:?}");
            let mut snapshot = t.snapshot().clone();
            snapshot.dataset = TableDatasetKey(2);
            assert_eq!(
                t.replace_keyed(snapshot),
                Err(TableUpdateError::CounterExhausted)
            );
            assert_eq!(format!("{t:?}"), before);
            if revision {
                assert!(!t.sort_key(TableColumnKey(1)));
                assert_eq!(format!("{t:?}"), before);
            }
        }
    }
}
