use crate::ControlKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    pub selected: Option<usize>,
    pub count: usize,
}
impl List {
    pub fn new(count: usize) -> Self {
        Self {
            selected: None,
            count,
        }
    }
    pub fn select(&mut self, i: usize) -> bool {
        if i >= self.count || self.selected == Some(i) {
            return false;
        }
        self.selected = Some(i);
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.count == 0 {
            return false;
        }
        let next = match key {
            ControlKey::Up => self.selected.unwrap_or(self.count).saturating_sub(1),
            ControlKey::Down => (self.selected.map_or(0, |i| i + 1)).min(self.count - 1),
            ControlKey::Home => 0,
            ControlKey::End => self.count - 1,
            _ => return false,
        };
        self.select(next)
    }
}

/// Caller-owned identity, independent of labels, source indices and display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListDatasetKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListItemKey(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedListItem {
    pub key: ListItemKey,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedListSnapshot {
    pub dataset: ListDatasetKey,
    pub items: Vec<KeyedListItem>,
    /// All logical rows, including those outside the viewport; unique subset.
    pub display_items: Vec<ListItemKey>,
}
/// Finite one-dimensional model limits. Content budgets belong to Content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListBudget {
    pub max_items: usize,
    pub max_display_items: usize,
    pub max_utf8_bytes: usize,
    /// Structural staging units: item, index, incarnation, display, old mapping.
    /// This bounds structure counts, not allocator bytes or process memory.
    pub max_staging_units: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListBudgetKind {
    Items,
    DisplayItems,
    Utf8Bytes,
    StagingUnits,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListUpdateError {
    InvalidBudget,
    BudgetExceeded(ListBudgetKind),
    Overflow,
    CounterExhausted,
    DuplicateItem(ListItemKey),
    DuplicateDisplayItem(ListItemKey),
    MissingDisplayItem(ListItemKey),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListReconcile {
    pub old_items_to_new: Vec<Option<usize>>,
    pub revision: u64,
}
/// Owns the sole selection reducer. No mutable original or snapshot escape.
#[derive(Debug)]
pub struct KeyedList {
    original: List,
    snapshot: KeyedListSnapshot,
    budget: ListBudget,
    indices: std::collections::BTreeMap<ListItemKey, usize>,
    display_indices: Vec<usize>,
    displayed_mask: Vec<bool>,
    display_ordinals: Vec<Option<usize>>,
    incarnations: Vec<u64>,
    identity: std::sync::Arc<()>,
    revision: u64,
    next_incarnation: u64,
}
impl Clone for KeyedList {
    fn clone(&self) -> Self {
        Self {
            original: self.original.clone(),
            snapshot: self.snapshot.clone(),
            budget: self.budget,
            indices: self.indices.clone(),
            display_indices: self.display_indices.clone(),
            displayed_mask: self.displayed_mask.clone(),
            display_ordinals: self.display_ordinals.clone(),
            incarnations: self.incarnations.clone(),
            identity: std::sync::Arc::new(()),
            revision: self.revision,
            next_incarnation: self.next_incarnation,
        }
    }
}
#[derive(Debug)]
pub(crate) struct ListStage {
    pub(crate) snapshot: KeyedListSnapshot,
    indices: std::collections::BTreeMap<ListItemKey, usize>,
    pub(crate) display_indices: Vec<usize>,
    displayed_mask: Vec<bool>,
    display_ordinals: Vec<Option<usize>>,
    pub(crate) incarnations: Vec<u64>,
    next_incarnation: u64,
    selected: Option<usize>,
    pub(crate) reconcile: ListReconcile,
}
impl KeyedList {
    fn staging_units(items: usize, displayed: usize, old: usize) -> Result<usize, ListUpdateError> {
        items
            .checked_mul(3)
            .and_then(|n| n.checked_add(displayed))
            .and_then(|n| n.checked_add(old))
            .ok_or(ListUpdateError::Overflow)
    }
    // Complete read-only preflight, including counter demand, before allocating
    // any library-owned staging maps/vectors or mutating the original instance.
    fn stage(
        snapshot: KeyedListSnapshot,
        budget: ListBudget,
        old: Option<&Self>,
        revision: u64,
    ) -> Result<ListStage, ListUpdateError> {
        use ListBudgetKind::*;
        if [
            budget.max_items,
            budget.max_display_items,
            budget.max_utf8_bytes,
            budget.max_staging_units,
        ]
        .contains(&0)
        {
            return Err(ListUpdateError::InvalidBudget);
        }
        for (count, limit, kind) in [
            (snapshot.items.len(), budget.max_items, Items),
            (
                snapshot.display_items.len(),
                budget.max_display_items,
                DisplayItems,
            ),
            (
                Self::staging_units(
                    snapshot.items.len(),
                    snapshot.display_items.len(),
                    old.map_or(0, |o| o.snapshot.items.len()),
                )?,
                budget.max_staging_units,
                StagingUnits,
            ),
        ] {
            if count > limit {
                return Err(ListUpdateError::BudgetExceeded(kind));
            }
        }
        let mut bytes = 0usize;
        let mut indices = std::collections::BTreeMap::new();
        for (i, item) in snapshot.items.iter().enumerate() {
            bytes = bytes
                .checked_add(item.name.len())
                .ok_or(ListUpdateError::Overflow)?;
            if bytes > budget.max_utf8_bytes {
                return Err(ListUpdateError::BudgetExceeded(Utf8Bytes));
            }
            if indices.insert(item.key, i).is_some() {
                return Err(ListUpdateError::DuplicateItem(item.key));
            }
        }
        let mut displayed_mask = vec![false; snapshot.items.len()];
        let mut display_ordinals = vec![None; snapshot.items.len()];
        let mut display_indices = Vec::with_capacity(snapshot.display_items.len());
        for (ord, key) in snapshot.display_items.iter().enumerate() {
            let Some(&idx) = indices.get(key) else {
                return Err(ListUpdateError::MissingDisplayItem(*key));
            };
            if displayed_mask[idx] {
                return Err(ListUpdateError::DuplicateDisplayItem(*key));
            }
            displayed_mask[idx] = true;
            display_ordinals[idx] = Some(ord);
            display_indices.push(idx);
        }
        let same_dataset = old.filter(|o| o.dataset_key() == snapshot.dataset);
        let continuous = |new_idx: usize, key: ListItemKey| -> Option<u64> {
            let o = same_dataset?;
            let i = o.item_index(key)?;
            (o.item_displayed(i) == displayed_mask[new_idx]).then_some(o.incarnations[i])
        };
        let mut needed = 0u64;
        for (new_idx, item) in snapshot.items.iter().enumerate() {
            if continuous(new_idx, item.key).is_none() {
                needed = needed
                    .checked_add(1)
                    .ok_or(ListUpdateError::CounterExhausted)?;
            }
        }
        let mut counter = old.map_or(0, |o| o.next_incarnation);
        let final_counter = counter
            .checked_add(needed)
            .ok_or(ListUpdateError::CounterExhausted)?;
        let incarnations = snapshot
            .items
            .iter()
            .enumerate()
            .map(|(new_idx, r)| {
                continuous(new_idx, r.key).unwrap_or_else(|| {
                    // Capacity was checked for the entire transaction above.
                    counter += 1;
                    counter
                })
            })
            .collect();
        let selected = same_dataset
            .and_then(Self::selected_key)
            .and_then(|k| indices.get(&k).copied())
            .filter(|&idx| displayed_mask[idx]);
        let old_items_to_new = old
            .map(|o| {
                o.snapshot
                    .items
                    .iter()
                    .map(|r| {
                        (o.dataset_key() == snapshot.dataset)
                            .then(|| indices.get(&r.key).copied())
                            .flatten()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(ListStage {
            snapshot,
            indices,
            display_indices,
            displayed_mask,
            display_ordinals,
            incarnations,
            next_incarnation: final_counter,
            selected,
            reconcile: ListReconcile {
                old_items_to_new,
                revision,
            },
        })
    }
    pub fn new(snapshot: KeyedListSnapshot, budget: ListBudget) -> Result<Self, ListUpdateError> {
        let s = Self::stage(snapshot, budget, None, 0)?;
        Ok(Self {
            original: List::new(s.snapshot.items.len()),
            snapshot: s.snapshot,
            budget,
            indices: s.indices,
            display_indices: s.display_indices,
            displayed_mask: s.displayed_mask,
            display_ordinals: s.display_ordinals,
            incarnations: s.incarnations,
            identity: std::sync::Arc::new(()),
            revision: 0,
            next_incarnation: s.next_incarnation,
        })
    }
    pub fn original(&self) -> &List {
        &self.original
    }
    pub fn snapshot(&self) -> &KeyedListSnapshot {
        &self.snapshot
    }
    pub fn budget(&self) -> ListBudget {
        self.budget
    }
    pub fn dataset_key(&self) -> ListDatasetKey {
        self.snapshot.dataset
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn selected_key(&self) -> Option<ListItemKey> {
        self.original.selected.and_then(|i| self.item_key(i))
    }
    pub fn item_key(&self, source_index: usize) -> Option<ListItemKey> {
        self.snapshot.items.get(source_index).map(|r| r.key)
    }
    pub fn item_index(&self, key: ListItemKey) -> Option<usize> {
        self.indices.get(&key).copied()
    }
    pub fn display_ordinal(&self, key: ListItemKey) -> Option<usize> {
        self.indices
            .get(&key)
            .and_then(|&idx| self.display_ordinals.get(idx).copied().flatten())
    }
    pub fn display_indices(&self) -> &[usize] {
        &self.display_indices
    }
    pub fn select_key(&mut self, key: ListItemKey) -> bool {
        self.item_index(key)
            .filter(|i| self.item_displayed(*i))
            .is_some_and(|i| self.original.select(i))
    }
    pub fn clear_selection(&mut self) -> bool {
        self.original.selected.take().is_some()
    }
    pub fn replace_keyed(
        &mut self,
        snapshot: KeyedListSnapshot,
    ) -> Result<ListReconcile, ListUpdateError> {
        let stage = self.prepare_list_replace(snapshot)?;
        let result = stage.reconcile.clone();
        self.commit_list_replace(stage);
        Ok(result)
    }
    pub(crate) fn prepare_list_replace(
        &self,
        snapshot: KeyedListSnapshot,
    ) -> Result<ListStage, ListUpdateError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(ListUpdateError::CounterExhausted)?;
        Self::stage(snapshot, self.budget, Some(self), revision)
    }
    pub(crate) fn commit_list_replace(&mut self, s: ListStage) {
        self.original = List {
            count: s.snapshot.items.len(),
            selected: s.selected,
        };
        self.snapshot = s.snapshot;
        self.indices = s.indices;
        self.display_indices = s.display_indices;
        self.displayed_mask = s.displayed_mask;
        self.display_ordinals = s.display_ordinals;
        self.incarnations = s.incarnations;
        self.next_incarnation = s.next_incarnation;
        self.revision = s.reconcile.revision;
    }
    pub(crate) fn item_displayed(&self, index: usize) -> bool {
        self.displayed_mask.get(index).copied().unwrap_or(false)
    }
    pub(crate) fn item_incarnation(&self, index: usize) -> Option<u64> {
        self.incarnations.get(index).copied()
    }
    pub(crate) fn view_identity(&self) -> std::sync::Arc<()> {
        self.identity.clone()
    }
    pub(crate) fn apply_prepared_action(
        &mut self,
        view: &mut crate::collections_view::ListView,
        p: crate::collections_view::CollectionAction,
    ) {
        view.apply_list_action(p, &mut self.original);
    }
    pub(crate) fn view_input(
        &mut self,
        view: &mut crate::collections_view::ListView,
        input: crate::collections_view::ListInput,
    ) -> crate::collections_view::CollectionEvent {
        view.reduce_list(input, &mut self.original)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub label: String,
    pub parent: Option<usize>,
    pub expanded: bool,
    pub disabled: bool,
}
/// Caller-assigned entity identity; labels and storage indices are not keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeNodeKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeDatasetKey(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedTreeNode {
    pub key: TreeNodeKey,
    pub parent: Option<TreeNodeKey>,
    pub label: String,
    pub disabled: bool,
    pub initially_expanded: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeUpdateError {
    DuplicateKey(TreeNodeKey),
    InvalidParent(TreeNodeKey),
    NonPreorder(TreeNodeKey),
    CounterExhausted,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeReconcile {
    pub old_to_new: Vec<Option<usize>>,
    pub removed: Vec<usize>,
    pub hidden: Vec<usize>,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct KeyedTreeState {
    dataset: TreeDatasetKey,
    keys: Vec<TreeNodeKey>,
    indices: std::collections::BTreeMap<TreeNodeKey, usize>,
    incarnations: Vec<u64>,
    selected: Option<TreeNodeKey>,
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Tree {
    nodes: Vec<TreeNode>,
    /// Compatibility projection. In keyed mode use select/select_key instead.
    pub selected: Option<usize>,
    keyed: Option<KeyedTreeState>,
    revision: u64,
    next_incarnation: u64,
    identity: std::sync::Arc<()>,
}
impl Clone for Tree {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            selected: self.selected,
            keyed: self.keyed.clone(),
            revision: self.revision,
            next_incarnation: self.next_incarnation,
            // A clone is a new model, never authority for another model's View.
            identity: std::sync::Arc::new(()),
        }
    }
}
impl Tree {
    pub fn nodes(&self) -> &[TreeNode] {
        &self.nodes
    }
    pub fn is_keyed(&self) -> bool {
        self.keyed.is_some()
    }
    pub fn model_revision(&self) -> u64 {
        self.revision
    }
    pub fn dataset_key(&self) -> Option<TreeDatasetKey> {
        self.keyed.as_ref().map(|k| k.dataset)
    }
    pub fn node_key(&self, index: usize) -> Option<TreeNodeKey> {
        self.keyed.as_ref()?.keys.get(index).copied()
    }
    pub fn node_index(&self, key: TreeNodeKey) -> Option<usize> {
        self.keyed.as_ref()?.indices.get(&key).copied()
    }
    pub fn selected_key(&self) -> Option<TreeNodeKey> {
        self.keyed.as_ref()?.selected
    }
    pub fn select_key(&mut self, key: TreeNodeKey) -> bool {
        self.node_index(key).is_some_and(|i| self.select(i))
    }
    pub(crate) fn model_identity(&self) -> std::sync::Arc<()> {
        self.identity.clone()
    }
    pub(crate) fn incarnation(&self, index: usize) -> Option<u64> {
        self.keyed.as_ref()?.incarnations.get(index).copied()
    }
    pub(crate) fn selection_projection_current(&self) -> bool {
        !self.is_keyed()
            || self.selected == self.selected_key().and_then(|key| self.node_index(key))
    }
    pub(crate) fn restore_selection_projection(&mut self) -> bool {
        if self.selection_projection_current() {
            return false;
        }
        self.selected = self.selected_key().and_then(|key| self.node_index(key));
        true
    }
    /// Validate the complete snapshot and all counter allocations before swapping.
    pub fn replace_keyed(
        &mut self,
        dataset: TreeDatasetKey,
        nodes: Vec<KeyedTreeNode>,
    ) -> Result<TreeReconcile, TreeUpdateError> {
        let mut indices = std::collections::BTreeMap::new();
        let mut ancestors = Vec::new();
        let same_dataset = self.dataset_key() == Some(dataset);
        let mut staged = self.clone();
        staged.nodes.clear();
        let mut keys = Vec::with_capacity(nodes.len());
        for (i, node) in nodes.into_iter().enumerate() {
            if indices.contains_key(&node.key) {
                return Err(TreeUpdateError::DuplicateKey(node.key));
            }
            let parent = if let Some(parent_key) = node.parent {
                let Some(&parent) = indices.get(&parent_key) else {
                    return Err(TreeUpdateError::InvalidParent(node.key));
                };
                let Some(position) = ancestors.iter().rposition(|key| *key == parent_key) else {
                    return Err(TreeUpdateError::NonPreorder(node.key));
                };
                ancestors.truncate(position + 1);
                Some(parent)
            } else {
                ancestors.clear();
                None
            };
            let expanded = if same_dataset {
                self.node_index(node.key)
                    .map(|j| self.nodes[j].expanded)
                    .unwrap_or(node.initially_expanded)
            } else {
                node.initially_expanded
            };
            staged.nodes.push(TreeNode {
                label: node.label,
                parent,
                expanded,
                disabled: node.disabled,
            });
            indices.insert(node.key, i);
            ancestors.push(node.key);
            keys.push(node.key);
        }
        staged.keyed = Some(KeyedTreeState {
            dataset,
            incarnations: vec![0; keys.len()],
            keys,
            indices,
            selected: None,
        });
        staged.selected = None;
        let visible_flags = staged.visible_flags();
        if same_dataset {
            staged.recover_selection_with_visible(self, &visible_flags);
        }
        staged.finish_update_with_visible(self, &visible_flags)?;
        let old_to_new = (0..self.nodes.len())
            .map(|i| {
                same_dataset
                    .then(|| self.node_key(i).and_then(|key| staged.node_index(key)))
                    .flatten()
            })
            .collect::<Vec<_>>();
        let result = TreeReconcile {
            removed: old_to_new
                .iter()
                .enumerate()
                .filter_map(|(i, j)| j.is_none().then_some(i))
                .collect(),
            hidden: old_to_new
                .iter()
                .enumerate()
                .filter_map(|(i, j)| j.filter(|j| !visible_flags[*j]).map(|_| i))
                .collect(),
            old_to_new,
            revision: staged.revision,
        };
        staged.identity = self.identity.clone();
        *self = staged;
        Ok(result)
    }
    // Recover only through the old ancestor chain. A disabled target clears,
    // whereas disappearance/concealment may recover to a surviving ancestor.
    fn recover_selection_with_visible(&mut self, old: &Self, visible: &[bool]) {
        let Some(key) = old.selected_key() else {
            return;
        };
        if let Some(i) = self.node_index(key) {
            if self.nodes[i].disabled {
                return;
            }
            if visible[i] {
                self.select(i);
                return;
            }
        }
        let mut ancestor = old.node_index(key).and_then(|i| old.nodes[i].parent);
        while let Some(i) = ancestor {
            if let Some(j) = old.node_key(i).and_then(|key| self.node_index(key))
                && visible[j]
                && !self.nodes[j].disabled
            {
                self.select(j);
                return;
            }
            ancestor = old.nodes[i].parent;
        }
    }
    fn finish_update(&mut self, old: &Self) -> Result<(), TreeUpdateError> {
        let visible = self.visible_flags();
        self.finish_update_with_visible(old, &visible)
    }
    fn finish_update_with_visible(
        &mut self,
        old: &Self,
        visible: &[bool],
    ) -> Result<(), TreeUpdateError> {
        self.revision = old
            .revision
            .checked_add(1)
            .ok_or(TreeUpdateError::CounterExhausted)?;
        let same = old.dataset_key() == self.dataset_key();
        let old_visible = same.then(|| old.visible_flags());
        let Some(state) = self.keyed.as_mut() else {
            return Ok(());
        };
        for (i, key) in state.keys.iter().enumerate() {
            let retained = same.then(|| old.node_index(*key)).flatten().filter(|j| {
                visible[i] == old_visible.as_ref().is_some_and(|v| v[*j])
                    && self.nodes[i].disabled == old.nodes[*j].disabled
            });
            state.incarnations[i] = if let Some(j) = retained {
                old.incarnation(j).expect("retained keyed identity")
            } else {
                self.next_incarnation = self
                    .next_incarnation
                    .checked_add(1)
                    .ok_or(TreeUpdateError::CounterExhausted)?;
                self.next_incarnation
            };
        }
        Ok(())
    }
    pub fn add(&mut self, label: impl Into<String>, parent: Option<usize>) -> Option<usize> {
        if self.is_keyed() || parent.is_some_and(|p| p >= self.nodes.len()) {
            return None;
        }
        let i = self.nodes.len();
        self.nodes.push(TreeNode {
            label: label.into(),
            parent,
            expanded: false,
            disabled: false,
        });
        Some(i)
    }
    pub fn set_disabled(&mut self, i: usize, disabled: bool) -> bool {
        let Some(node) = self.nodes.get(i) else {
            return false;
        };
        if node.disabled == disabled {
            return true;
        }
        let mut staged = self.clone();
        staged.restore_selection_projection();
        staged.nodes[i].disabled = disabled;
        if disabled && staged.selected == Some(i) {
            staged.clear_selection();
        }
        if self.is_keyed() && staged.finish_update(self).is_err() {
            return false;
        }
        staged.identity = self.identity.clone();
        *self = staged;
        true
    }
    pub fn clear_selection(&mut self) {
        self.selected = None;
        if let Some(state) = self.keyed.as_mut() {
            state.selected = None;
        }
    }
    pub fn select(&mut self, i: usize) -> bool {
        self.restore_selection_projection();
        if self.nodes.get(i).is_none_or(|n| n.disabled) || self.selected == Some(i) {
            return false;
        }
        self.selected = Some(i);
        if let Some(state) = self.keyed.as_mut() {
            state.selected = Some(state.keys[i]);
        }
        true
    }
    pub fn toggle(&mut self, i: usize) -> bool {
        if self.nodes.get(i).is_none_or(|n| n.disabled)
            || !self.nodes.iter().any(|n| n.parent == Some(i))
        {
            return false;
        }
        let mut staged = self.clone();
        staged.restore_selection_projection();
        staged.nodes[i].expanded = !staged.nodes[i].expanded;
        if self.is_keyed() {
            let visible_flags = staged.visible_flags();
            staged.clear_selection();
            staged.recover_selection_with_visible(self, &visible_flags);
            if staged
                .finish_update_with_visible(self, &visible_flags)
                .is_err()
            {
                return false;
            }
        }
        staged.identity = self.identity.clone();
        *self = staged;
        true
    }
    pub(crate) fn is_node_visible(&self, index: usize) -> bool {
        let mut p = self.nodes.get(index).and_then(|n| n.parent);
        while let Some(parent) = p {
            let Some(node) = self.nodes.get(parent) else {
                return false;
            };
            if !node.expanded {
                return false;
            }
            p = node.parent;
        }
        index < self.nodes.len()
    }
    pub(crate) fn visible_flags(&self) -> Vec<bool> {
        let mut flags = vec![false; self.nodes.len()];
        for (i, node) in self.nodes.iter().enumerate() {
            flags[i] = match node.parent {
                None => true,
                Some(p) if p < i => flags[p] && self.nodes[p].expanded,
                Some(_) => self.is_node_visible(i),
            };
        }
        flags
    }
    pub fn visible(&self) -> Vec<usize> {
        self.visible_flags()
            .into_iter()
            .enumerate()
            .filter_map(|(i, visible)| visible.then_some(i))
            .collect()
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        self.restore_selection_projection();
        let visible = self.visible();
        let Some(current) = self.selected else {
            return match key {
                ControlKey::End => visible
                    .iter()
                    .rev()
                    .copied()
                    .find(|i| !self.nodes[*i].disabled)
                    .is_some_and(|i| self.select(i)),
                _ => visible
                    .iter()
                    .copied()
                    .find(|i| !self.nodes[*i].disabled)
                    .is_some_and(|i| self.select(i)),
            };
        };
        let Some(pos) = visible.iter().position(|i| *i == current) else {
            return false;
        };
        match key {
            ControlKey::Down => visible
                .iter()
                .skip(pos + 1)
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Up => visible
                .iter()
                .take(pos)
                .rev()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Home => visible
                .iter()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::End => visible
                .iter()
                .rev()
                .copied()
                .find(|i| !self.nodes[*i].disabled)
                .is_some_and(|i| self.select(i)),
            ControlKey::Right => {
                if self.nodes[current].expanded {
                    visible
                        .iter()
                        .skip(pos + 1)
                        .copied()
                        .find(|i| {
                            self.nodes[*i].parent == Some(current) && !self.nodes[*i].disabled
                        })
                        .is_some_and(|i| self.select(i))
                } else {
                    self.toggle(current)
                }
            }
            ControlKey::Left => {
                if self.nodes[current].expanded {
                    self.toggle(current)
                } else if let Some(p) = self.nodes[current].parent {
                    self.select(p)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tree_identity_counters {
    use super::*;
    fn node(key: u64) -> KeyedTreeNode {
        KeyedTreeNode {
            key: TreeNodeKey(key),
            parent: None,
            label: "x".into(),
            disabled: false,
            initially_expanded: false,
        }
    }
    #[test]
    fn revision_overflow_is_atomic_for_replace_and_disable() {
        let mut tree = Tree::default();
        tree.replace_keyed(TreeDatasetKey(1), vec![node(1)])
            .unwrap();
        tree.select(0);
        tree.revision = u64::MAX;
        let before = tree.clone();
        assert_eq!(
            tree.replace_keyed(TreeDatasetKey(1), vec![node(2)]),
            Err(TreeUpdateError::CounterExhausted)
        );
        assert_eq!(tree, before);
        assert!(!tree.set_disabled(0, true));
        assert_eq!(tree, before);
    }
    #[test]
    fn incarnation_overflow_is_atomic_and_counter_never_resets() {
        let mut tree = Tree::default();
        tree.replace_keyed(TreeDatasetKey(1), vec![node(1)])
            .unwrap();
        let original = tree.incarnation(0).unwrap();
        tree.replace_keyed(TreeDatasetKey(2), vec![]).unwrap();
        tree.replace_keyed(TreeDatasetKey(1), vec![node(1)])
            .unwrap();
        assert!(tree.incarnation(0).unwrap() > original);
        tree.next_incarnation = u64::MAX;
        let before = tree.clone();
        assert_eq!(
            tree.replace_keyed(TreeDatasetKey(1), vec![node(1), node(2)]),
            Err(TreeUpdateError::CounterExhausted)
        );
        assert_eq!(tree, before);
        assert!(!tree.set_disabled(0, true));
        assert_eq!(tree, before);
    }
    #[test]
    fn collapse_counter_overflow_is_atomic() {
        let mut tree = Tree::default();
        let mut child = node(2);
        child.parent = Some(TreeNodeKey(1));
        tree.replace_keyed(TreeDatasetKey(1), vec![node(1), child])
            .unwrap();
        tree.next_incarnation = u64::MAX;
        let before = tree.clone();
        assert!(!tree.toggle(0));
        assert_eq!(tree, before);
    }
    #[test]
    fn deep_and_wide_keyed_trees_preserve_preorder_visibility_recovery_and_incarnation_rules() {
        let deep_nodes = |d: usize| {
            (0..d)
                .map(|i| KeyedTreeNode {
                    key: TreeNodeKey((i + 1) as u64),
                    parent: (i > 0).then_some(TreeNodeKey(i as u64)),
                    label: format!("d{i}"),
                    disabled: false,
                    initially_expanded: true,
                })
                .collect::<Vec<_>>()
        };
        let mut deep = Tree::default();
        deep.replace_keyed(TreeDatasetKey(1), deep_nodes(500))
            .unwrap();
        assert_eq!(deep.visible(), (0..500).collect::<Vec<_>>());
        assert!(deep.select_key(TreeNodeKey(400)));

        // NonPreorder & InvalidParent reject atomically without corrupting model.
        let before = deep.clone();
        assert_eq!(
            deep.replace_keyed(
                TreeDatasetKey(1),
                vec![
                    node(1),
                    KeyedTreeNode {
                        key: TreeNodeKey(2),
                        parent: Some(TreeNodeKey(1)),
                        label: "c1".into(),
                        disabled: false,
                        initially_expanded: false,
                    },
                    node(3),
                    KeyedTreeNode {
                        key: TreeNodeKey(4),
                        parent: Some(TreeNodeKey(1)),
                        label: "late".into(),
                        disabled: false,
                        initially_expanded: false,
                    },
                ],
            ),
            Err(TreeUpdateError::NonPreorder(TreeNodeKey(4)))
        );
        assert_eq!(deep, before);
        assert_eq!(
            deep.replace_keyed(
                TreeDatasetKey(1),
                vec![KeyedTreeNode {
                    key: TreeNodeKey(2),
                    parent: Some(TreeNodeKey(999)),
                    label: "orphan".into(),
                    disabled: false,
                    initially_expanded: false,
                }],
            ),
            Err(TreeUpdateError::InvalidParent(TreeNodeKey(2)))
        );
        assert_eq!(deep, before);

        // Collapsing ancestor 250 recovers selection to index 249 (key 250), hides 250..500,
        // and retains incarnations for 0..=249 while bumping hidden/shown descendants on re-toggle.
        let inc_anc = deep.incarnation(249).unwrap();
        let inc_desc = deep.incarnation(300).unwrap();
        assert!(deep.toggle(249));
        assert_eq!(deep.selected_key(), Some(TreeNodeKey(250)));
        assert_eq!(deep.visible(), (0..250).collect::<Vec<_>>());
        assert_eq!(deep.incarnation(249), Some(inc_anc));
        assert!(deep.incarnation(300).unwrap() > inc_desc);

        // Removing the selected node (key 250) clears or recovers through surviving ancestor (249).
        let mut trimmed = deep_nodes(249);
        trimmed[248].disabled = false;
        let rec = deep.replace_keyed(TreeDatasetKey(1), trimmed).unwrap();
        assert_eq!(rec.removed, (249..500).collect::<Vec<_>>());
        assert!(rec.hidden.is_empty());
        assert_eq!(deep.selected_key(), Some(TreeNodeKey(249)));

        // Disabling selected target clears selection cleanly.
        let mut with_disabled = deep_nodes(10);
        assert!(
            deep.replace_keyed(TreeDatasetKey(2), with_disabled.clone())
                .is_ok()
        );
        assert!(deep.select_key(TreeNodeKey(5)));
        with_disabled[4].disabled = true;
        deep.replace_keyed(TreeDatasetKey(2), with_disabled)
            .unwrap();
        assert_eq!(deep.selected_key(), None);

        // Wide sibling tree: collapse hides siblings 1..=500 and updates reconcile.hidden in preorder.
        let mut wide = Tree::default();
        let mut wnodes = vec![KeyedTreeNode {
            key: TreeNodeKey(1),
            parent: None,
            label: "root".into(),
            disabled: false,
            initially_expanded: true,
        }];
        for k in 2..=501u64 {
            wnodes.push(KeyedTreeNode {
                key: TreeNodeKey(k),
                parent: Some(TreeNodeKey(1)),
                label: format!("w{k}"),
                disabled: false,
                initially_expanded: false,
            });
        }
        wide.replace_keyed(TreeDatasetKey(3), wnodes.clone())
            .unwrap();
        assert_eq!(wide.visible(), (0..=500).collect::<Vec<_>>());
        assert!(wide.toggle(0));
        assert_eq!(wide.visible(), vec![0]);
        let rec_w = wide.replace_keyed(TreeDatasetKey(3), wnodes).unwrap();
        assert_eq!(rec_w.hidden, (1..=500).collect::<Vec<_>>());
    }
}
