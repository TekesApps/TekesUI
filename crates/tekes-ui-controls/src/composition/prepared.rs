//! Finite checked-entry hooks on the original owners. Ordinary hooks remain dynamic.
use super::content_reading::{ReaderUnmountWrites, prepare_reader_writes};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedPhase {
    Retained,
    Detached,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedHookKind {
    ChildrenAccess,
    Cancel,
    CancelPointer,
    PointerExit,
    Focus { focused: bool, visible: bool },
    LifecycleStart(Lifecycle),
    Lifecycle(Lifecycle),
    ActivityState { parent: bool, effective: bool },
    WindowActive(bool),
    PresentationRunnable(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedHook {
    pub kind: PreparedHookKind,
    pub phase: PreparedPhase,
}
impl PreparedHook {
    pub fn retained(kind: PreparedHookKind) -> Self {
        Self {
            kind,
            phase: PreparedPhase::Retained,
        }
    }
    pub fn detached(kind: PreparedHookKind) -> Self {
        Self {
            kind,
            phase: PreparedPhase::Detached,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedEffectsError {
    Unavailable,
    CounterCapacity,
    ResourceCapacity,
    InvalidSequence,
    StaleOwner,
}

/// Owner-bound, data-only proof for this exact whole ordered sequence. Implementing
/// preparation promises no eligibility/access-path changes outside declared scope;
/// retained calls preserve eligibility (legal unmark/edit changes are projected).
/// Detached calls may change only the detached owner/declared descendants. This is
/// additive to StableHooks, not a runtime sandbox for dishonest implementations.
#[derive(Debug)]
pub struct PreparedEffects {
    owner: usize,
    hooks: Vec<PreparedHook>,
    readers: Vec<(usize, ReaderUnmountWrites)>,
}
fn identity<T: ?Sized>(owner: &T) -> usize {
    std::ptr::from_ref(owner).cast::<()>() as usize
}
impl PreparedEffects {
    pub fn local_work(calls: usize) -> Result<usize, PreparedEffectsError> {
        calls
            .checked_mul(256)
            .and_then(|n| n.checked_add(512))
            .ok_or(PreparedEffectsError::CounterCapacity)
    }

    /// For a compliant consumer's own finite hooks, including owned fields.
    /// The caller audits/project-checks all its own counters and forwarding first.
    /// No arbitrary resource counts or foreign identities can be inserted here.
    pub fn local<T: Component + ?Sized>(owner: &T, hooks: &[PreparedHook]) -> Self {
        Self {
            owner: identity(owner),
            hooks: hooks.to_vec(),
            readers: Vec::new(),
        }
    }
    pub(super) fn reader_count(&self) -> usize {
        self.readers.len()
    }
    fn into_context(self) -> PreparedLifecycleContext {
        PreparedLifecycleContext {
            owner: self.owner,
            hooks: self.hooks,
            pending: self.readers,
            readers: Vec::new(),
            step: 0,
        }
    }
}
/// Nonconstructible/noncloneable context bound to the original prepared owner.
/// A wrapper delegates BOTH prepare_effects and lifecycle_prepared to its retained
/// original field. It cannot forge, move between owners, or replay resource tickets.
#[derive(Debug)]
pub struct PreparedLifecycleContext {
    owner: usize,
    hooks: Vec<PreparedHook>,
    pending: Vec<(usize, ReaderUnmountWrites)>,
    readers: Vec<(usize, (ReaderUnmountWrites, crate::ReadingDocumentVersion))>,
    step: usize,
}
impl PreparedLifecycleContext {
    fn bind_step(&mut self, step: usize, range: &mut crate::reading::AttachmentRange) {
        if let Some(index) = self.pending.iter().position(|(i, _)| *i == step) {
            let (_, writes) = self.pending.remove(index);
            let version = crate::ReadingDocumentVersion::from_reserved(
                writes.key.clone(),
                writes.revision,
                range.take(),
            );
            self.readers.push((step, (writes, version)));
        }
    }

    pub(super) fn set_step(&mut self, step: usize) {
        self.step = step;
    }
    pub fn original_lifecycle<T: Component + ?Sized>(&mut self, owner: &mut T, event: Lifecycle) {
        assert_eq!(
            self.owner,
            identity(owner),
            "prepared lifecycle delegated to a different owner"
        );
        assert!(matches!(self.hooks[self.step].kind,PreparedHookKind::Lifecycle(e) if e==event));
        assert!(
            !self.readers.iter().any(|(step, _)| *step == self.step),
            "reader owner must consume original prepared outcome"
        );
        owner.lifecycle(event);
    }
    pub(super) fn scroll_lifecycle(&mut self, owner: &mut ScrollContent, event: Lifecycle) {
        assert_eq!(
            self.owner,
            identity(owner),
            "prepared reader delegated to a different owner"
        );
        assert!(matches!(self.hooks[self.step].kind,PreparedHookKind::Lifecycle(e) if e==event));
        owner.reading_mounted = event == Lifecycle::Mounted;
        if event == Lifecycle::Unmounted {
            owner.last_frame = None;
            if let Some(index) = self.readers.iter().position(|(step, _)| *step == self.step) {
                let (_, (writes, version)) = self.readers.remove(index);
                super::content_reading::apply_reader_writes(
                    owner.reading.as_mut().expect("prepared attached reader"),
                    writes,
                    version,
                );
            } else {
                assert!(owner.reading.is_none(), "unprepared reader attachment");
            }
        }
    }
}
impl ScrollContent {
    /// The actual original owner projects its successive Cancel/Unmounted state.
    /// This method is public through Component for external finite wrappers.
    pub(super) fn prepare_scroll_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        let mut result = PreparedEffects::local(self, hooks);
        let mut scroll = self.scroll.clone();
        let mut previous: Option<ReaderUnmountWrites> = None;
        for (index, hook) in hooks.iter().enumerate() {
            match hook.kind {
                PreparedHookKind::Cancel | PreparedHookKind::CancelPointer => {
                    scroll.cancel();
                }
                PreparedHookKind::Lifecycle(Lifecycle::Unmounted) => {
                    if hook.phase != PreparedPhase::Detached {
                        return Err(PreparedEffectsError::InvalidSequence);
                    }
                    if let Some(r) = &self.reading {
                        // After the first Unmounted, original frames are empty and capture
                        // returns the previous pending anchor. No clone of untouched memories.
                        let writes = previous
                            .clone()
                            .unwrap_or_else(|| prepare_reader_writes(r, &scroll));
                        previous = Some(writes.clone());
                        result.readers.push((index, writes));
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }
}
impl Content {
    pub(super) fn prepare_node(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        if self.component.table_transaction_compatibility()
            != TableTransactionCompatibility::StableHooks
        {
            return Err(PreparedEffectsError::Unavailable);
        }
        if self.help.is_some()
            && hooks
                .iter()
                .any(|h| h.kind == PreparedHookKind::Lifecycle(Lifecycle::Unmounted))
            && self.help_revision == u64::MAX
        {
            return Err(PreparedEffectsError::CounterCapacity);
        }
        self.component.prepared_effects_work(hooks.len())?;
        self.component.prepare_effects(hooks)
    }
    pub(super) fn execute_hook(
        &mut self,
        context: &mut PreparedLifecycleContext,
        mounts: &mut actions::MountRange,
        i: usize,
    ) {
        context.set_step(i);
        match context.hooks[i].kind {
            PreparedHookKind::ChildrenAccess => {}
            PreparedHookKind::Cancel => {
                self.component.event(&ContentEvent::Cancel);
            }
            PreparedHookKind::CancelPointer => {
                self.component.cancel_pointer();
            }
            PreparedHookKind::PointerExit => {
                self.component
                    .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
            }
            PreparedHookKind::Focus { focused, visible } => {
                self.component.set_focus(focused, visible)
            }
            PreparedHookKind::LifecycleStart(event) => {
                self.lifecycle_start(event, mounts.take());
            }
            PreparedHookKind::Lifecycle(event) => self.component.lifecycle_prepared(event, context),
            PreparedHookKind::ActivityState { parent, effective } => {
                let (actual, _) = self.window_active_state(parent);
                debug_assert_eq!(actual, effective);
            }
            PreparedHookKind::WindowActive(active) => self.component.window_active_changed(active),
            PreparedHookKind::PresentationRunnable(runnable) => {
                self.component.presentation_runnable_changed(runnable)
            }
        }
    }
}
// Content::lifecycle_with's implicit Cancel/blur are included in owner preparation
// as explicit hooks, but execution above owns them in the original lifecycle slot.
pub(super) fn owner_sequence(hooks: &[PreparedHook]) -> Vec<PreparedHook> {
    let mut out = Vec::new();
    for hook in hooks {
        if let PreparedHookKind::Lifecycle(event) = hook.kind {
            out.push(PreparedHook {
                kind: PreparedHookKind::LifecycleStart(event),
                phase: hook.phase,
            });
        }
        if hook.kind == PreparedHookKind::Lifecycle(Lifecycle::Unmounted) {
            out.push(PreparedHook {
                kind: PreparedHookKind::Cancel,
                phase: hook.phase,
            });
            out.push(PreparedHook {
                kind: PreparedHookKind::Focus {
                    focused: false,
                    visible: false,
                },
                phase: hook.phase,
            });
        }
        out.push(*hook);
    }
    out
}

#[derive(Debug)]
struct NodePlan {
    path: Vec<usize>,
    id: ComponentId,
    mount: u64,
    hooks: Vec<PreparedHook>,
    effects: Option<PreparedEffects>,
    context: Option<PreparedLifecycleContext>,
}
#[derive(Debug, Default)]
pub(super) struct TreePlan {
    nodes: Vec<NodePlan>,
    ops: Vec<(usize, usize)>,
    pub(super) mounts: usize,
}
pub(super) fn path_to(root: &Content, id: ComponentId) -> Option<Vec<usize>> {
    fn walk(c: &Content, id: ComponentId, p: &mut Vec<usize>) -> bool {
        if c.id == id {
            return true;
        }
        for (i, child) in c.component.children().iter().enumerate() {
            p.push(i);
            if walk(child, id, p) {
                return true;
            }
            p.pop();
        }
        false
    }
    let mut p = vec![];
    walk(root, id, &mut p).then_some(p)
}
pub(super) fn at<'a>(mut c: &'a Content, path: &[usize]) -> Option<&'a Content> {
    for i in path {
        c = c.component.children().get(*i)?;
    }
    Some(c)
}
pub(super) fn at_mut<'a>(mut c: &'a mut Content, path: &[usize]) -> Option<&'a mut Content> {
    for i in path {
        c = c.component.children_mut()?.get_mut(*i)?;
    }
    Some(c)
}
pub(super) fn count(c: &Content) -> Option<usize> {
    c.component
        .children()
        .iter()
        .try_fold(1usize, |n, c| n.checked_add(count(c)?))
}
pub(super) fn data_work(c: &Content, calls: usize) -> Result<usize, PreparedEffectsError> {
    let own = match c.component.prepared_effects_work(calls) {
        Ok(n) => n,
        Err(PreparedEffectsError::Unavailable) => 0,
        Err(e) => return Err(e),
    };
    c.component.children().iter().try_fold(own, |n, c| {
        n.checked_add(data_work(c, calls)?)
            .ok_or(PreparedEffectsError::CounterCapacity)
    })
}
impl TreePlan {
    pub(super) fn add(
        &mut self,
        root: &Content,
        path: &[usize],
        hook: PreparedHook,
    ) -> Result<(), PreparedEffectsError> {
        let c = at(root, path).ok_or(PreparedEffectsError::StaleOwner)?;
        let n = if let Some(n) = self.nodes.iter().position(|n| n.path == path) {
            n
        } else {
            self.nodes.push(NodePlan {
                path: path.to_vec(),
                id: c.id,
                mount: c.mount,
                hooks: vec![],
                effects: None,
                context: None,
            });
            self.nodes.len() - 1
        };
        for expanded in owner_sequence(&[hook]) {
            if matches!(expanded.kind, PreparedHookKind::LifecycleStart(_)) {
                self.mounts = self
                    .mounts
                    .checked_add(1)
                    .ok_or(PreparedEffectsError::CounterCapacity)?;
            }
            let index = self.nodes[n].hooks.len();
            self.nodes[n].hooks.push(expanded);
            self.ops.push((n, index));
        }
        Ok(())
    }
    pub(super) fn subtree(
        &mut self,
        root: &Content,
        path: &[usize],
        event: Lifecycle,
    ) -> Result<(), PreparedEffectsError> {
        let phase = if event == Lifecycle::Unmounted {
            PreparedPhase::Detached
        } else {
            PreparedPhase::Retained
        };
        self.add(
            root,
            path,
            PreparedHook {
                kind: PreparedHookKind::Lifecycle(event),
                phase,
            },
        )?;
        let c = at(root, path).ok_or(PreparedEffectsError::StaleOwner)?;
        // Traversal uses declared children irrespective of hidden/zero/active state.
        for i in 0..c.component.children().len() {
            let mut p = path.to_vec();
            p.push(i);
            self.subtree(root, &p, event)?;
        }
        Ok(())
    }
    pub(super) fn activity(
        &mut self,
        root: &Content,
        path: &[usize],
        parent: bool,
    ) -> Result<(), PreparedEffectsError> {
        let c = at(root, path).ok_or(PreparedEffectsError::StaleOwner)?;
        let effective = c.notification_activity(parent);
        let previous = self
            .nodes
            .iter()
            .find(|n| n.path == path)
            .and_then(|n| {
                n.hooks.iter().rev().find_map(|h| {
                    if let PreparedHookKind::ActivityState { effective, .. } = h.kind {
                        Some(effective)
                    } else {
                        None
                    }
                })
            })
            .or(Some(c.activity_effective));
        self.add(
            root,
            path,
            PreparedHook::retained(PreparedHookKind::ActivityState { parent, effective }),
        )?;
        if previous != Some(effective) {
            self.add(
                root,
                path,
                PreparedHook::retained(PreparedHookKind::WindowActive(effective)),
            )?;
        }
        let child_active = parent && c.active();
        for i in 0..c.component.children().len() {
            let mut p = path.to_vec();
            p.push(i);
            self.activity(root, &p, child_active)?;
        }
        Ok(())
    }
    pub(super) fn cancel(
        &mut self,
        root: &Content,
        path: &[usize],
    ) -> Result<(), PreparedEffectsError> {
        self.add(root, path, PreparedHook::retained(PreparedHookKind::Cancel))?;
        let c = at(root, path).ok_or(PreparedEffectsError::StaleOwner)?;
        for i in 0..c.component.children().len() {
            let mut p = path.to_vec();
            p.push(i);
            self.cancel(root, &p)?;
        }
        Ok(())
    }
    pub(super) fn prepare(&mut self, root: &Content) -> Result<(), PreparedEffectsError> {
        for n in &mut self.nodes {
            let c = at(root, &n.path).ok_or(PreparedEffectsError::StaleOwner)?;
            n.effects = Some(c.prepare_node(&n.hooks)?);
        }
        Ok(())
    }
    pub(super) fn readers(&self) -> usize {
        self.nodes
            .iter()
            .map(|n| n.effects.as_ref().expect("prepared").reader_count())
            .sum()
    }
    pub(super) fn validate(&self, root: &Content) -> Result<(), PreparedEffectsError> {
        for n in &self.nodes {
            let c = at(root, &n.path).ok_or(PreparedEffectsError::StaleOwner)?;
            if c.id != n.id || c.mount != n.mount {
                return Err(PreparedEffectsError::StaleOwner);
            }
            let current = c.prepare_node(&n.hooks)?;
            let original = n.effects.as_ref().expect("prepared origin");
            if current.owner != original.owner
                || current.hooks != original.hooks
                || current.readers != original.readers
            {
                return Err(PreparedEffectsError::StaleOwner);
            }
        }
        Ok(())
    }
    pub(super) fn mutable_boundaries(
        &self,
        root: &mut Content,
    ) -> Result<(), PreparedEffectsError> {
        for n in &self.nodes {
            let c = at_mut(root, &n.path).ok_or(PreparedEffectsError::StaleOwner)?;
            let count = c.component.children().len();
            if count > 0 && c.component.children_mut().is_none() {
                return Err(PreparedEffectsError::Unavailable);
            }
        }
        Ok(())
    }
    pub(super) fn bind(&mut self, range: &mut crate::reading::AttachmentRange) {
        for n in &mut self.nodes {
            n.context = Some(n.effects.take().expect("prepared").into_context());
        }
        // Bind original reading IDs in actual hook order, not first node-access
        // order (a captured descendant may have been registered before its parent).
        for &(node, step) in &self.ops {
            self.nodes[node]
                .context
                .as_mut()
                .expect("prepared context")
                .bind_step(step, range);
        }
    }
    pub(super) fn assert_consumed(&self) {
        debug_assert!(
            self.nodes.iter().all(|n| n
                .context
                .as_ref()
                .is_none_or(|c| c.readers.is_empty() && c.pending.is_empty())),
            "prepared reader outcome was not delegated/consumed"
        );
    }
    pub(super) fn len(&self) -> usize {
        self.ops.len()
    }
    pub(super) fn commit_range(
        &mut self,
        root: &mut Content,
        mounts: &mut actions::MountRange,
        start: usize,
        end: usize,
        strip: &[usize],
    ) {
        let mut registered = false;
        for &(node, step) in &self.ops[start..end] {
            if !registered
                && matches!(
                    self.nodes[node].context.as_ref().expect("bound").hooks[step].kind,
                    PreparedHookKind::ActivityState { .. }
                )
            {
                super::split_view::refresh_split_baselines(root);
                registered = true;
            }
            let n = &mut self.nodes[node];
            let path = n
                .path
                .strip_prefix(strip)
                .expect("prepared retirement scope");
            let c = at_mut(root, path).expect("admitted stable indexed path");
            c.execute_hook(n.context.as_mut().expect("bound resources"), mounts, step);
        }
    }
    pub(super) fn commit(&mut self, root: &mut Content, mounts: &mut actions::MountRange) {
        self.commit_range(root, mounts, 0, self.len(), &[]);
    }
}

/// Checked finite-tree constructor, independent of ordinary dynamic Host::new.
impl ContentHost {
    pub fn try_new_prepared(
        mut root: Content,
        budget: usize,
    ) -> Result<Self, super::split_view::Rejected<Content>> {
        use super::split_view::{Rejected, SplitViewError};
        let prepare = |root: &Content| -> Result<TreePlan, SplitViewError> {
            let n = count(root).ok_or(SplitViewError::Budget)?;
            if n.checked_mul(n)
                .and_then(|n| n.checked_mul(512))
                .and_then(|n| n.checked_add(data_work(root, 16).ok()?))
                .is_none_or(|n| n > budget)
            {
                return Err(SplitViewError::Budget);
            }
            root.ids(&mut BTreeSet::new())
                .map_err(SplitViewError::Composition)?;
            let mut plan = TreePlan::default();
            plan.subtree(root, &[], Lifecycle::Mounted)?;
            plan.activity(root, &[], true)?;
            plan.prepare(root)?;
            Ok(plan)
        };
        let mut plan = match prepare(&root) {
            Ok(p) => p,
            Err(error) => {
                return Err(Rejected {
                    error,
                    input: Box::new(root),
                });
            }
        };
        if let Err(e) = plan.mutable_boundaries(&mut root) {
            return Err(Rejected {
                error: e.into(),
                input: Box::new(root),
            });
        }
        let mut reading = match if plan.readers() == 0 {
            Ok(crate::reading::AttachmentRange::empty())
        } else {
            crate::reading::reserve_attachments(plan.readers())
        } {
            Ok(r) => r,
            Err(_) => {
                return Err(Rejected {
                    error: SplitViewError::ResourceCapacity,
                    input: Box::new(root),
                });
            }
        };
        let Some(mut mounts) = actions::reserve_mounts(plan.mounts) else {
            return Err(Rejected {
                error: SplitViewError::ResourceCapacity,
                input: Box::new(root),
            });
        };
        if let Err(e) = plan.validate(&root) {
            return Err(Rejected {
                error: e.into(),
                input: Box::new(root),
            });
        }
        plan.bind(&mut reading);
        plan.commit(&mut root, &mut mounts);
        debug_assert!(mounts.exhausted());
        plan.assert_consumed();
        super::split_view::refresh_split_baselines(&mut root);
        Ok(Self::from_mounted(root))
    }
}
