//! Actions are collected at the original delivery boundary, never by polling
//! control state or replaying a callback during painting/focus changes.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentActionKind {
    Activate,
    /// An editor's configured submission intent, not business delivery.
    Submit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentActionSource {
    Pointer,
    Keyboard,
    /// The semantic action entry point; this alone does not prove VoiceOver.
    Accessibility,
    Programmatic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentAction {
    pub target: ComponentId,
    pub kind: ContentActionKind,
    pub source: ContentActionSource,
}
/// Owned data captured by the original component at delivery time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentActionPayload {
    MenuSelection(MenuSelection),
}
/// Item identity in one immutable model revision. Explicit values provide a
/// stable path; legacy positional menus retain index, label and model revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuSelection {
    pub(super) index: usize,
    pub(super) label: String,
    pub(super) model_revision: u64,
    pub(super) value_path: Option<Vec<String>>,
}
impl MenuSelection {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn model_revision(&self) -> u64 {
        self.model_revision
    }
    pub fn value_path(&self) -> Option<&[String]> {
        self.value_path.as_deref()
    }
}
/// Rich view of the same once-only queue used by `drain_actions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentActionRecord {
    pub action: ContentAction,
    /// Opaque original Content mount identity, not a business identifier.
    pub mount: u64,
    pub payload: Option<ContentActionPayload>,
}
#[derive(Debug)]
pub(super) struct PendingAction {
    pub(super) action: ContentAction,
    pub(super) mount: u64,
    pub(super) payload: Option<ContentActionPayload>,
}
static NEXT_MOUNT: AtomicU64 = AtomicU64::new(1);
pub(super) fn next_mount() -> u64 {
    try_next_mount().expect("content mount identity exhausted")
}
/// Same allocator and successful sequence as Content::new/lifecycle.
pub(super) fn try_next_mount() -> Result<u64, CompositionError> {
    #[cfg(test)]
    if FAIL_MOUNT.with(|flag| flag.get()) { return Err(CompositionError::MountExhausted); }
    NEXT_MOUNT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| CompositionError::MountExhausted)
}
#[cfg(test)]
mod mount_capacity_test {
    thread_local! { pub(crate) static FAIL_MOUNT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
}
#[cfg(test)]
pub(super) use mount_capacity_test::FAIL_MOUNT;
impl ContentEvent {
    fn action_source(&self) -> ContentActionSource {
        match self {
            Self::Pointer(_)
            | Self::PointerDown(_)
            | Self::PointerUp(_)
            | Self::PointerMove(_)
            | Self::Scroll { .. } => ContentActionSource::Pointer,
            Self::KeyInput(_)
            | Self::Key { .. }
            | Self::TextKeyStart { .. }
            | Self::TextKeyEnd { .. }
            | Self::Commit { .. }
            | Self::Preedit { .. }
            | Self::Unmark
            | Self::TextCommand(_) => ContentActionSource::Keyboard,
            Self::Cancel => ContentActionSource::Programmatic,
        }
    }
}
impl Content {
    pub(super) fn record_actions(
        &self,
        result: &ContentResponse,
        source: ContentActionSource,
        allowed: bool,
        pending: &mut Option<Vec<PendingAction>>,
    ) {
        if allowed && let Some(pending) = pending {
            pending.extend(self.component.action_kinds(result).into_iter().map(|kind| {
                PendingAction {
                    action: ContentAction {
                        target: self.id,
                        kind,
                        source,
                    },
                    mount: self.mount,
                    payload: self.component.action_payload(kind),
                }
            }));
        }
    }
    pub(super) fn deliver_actions(
        &mut self,
        event: &ContentEvent,
        overlay: bool,
        pending: &mut Option<Vec<PendingAction>>,
    ) -> ContentResponse {
        let allowed =
            pending.is_some() && self.component.active() && !self.component.semantics().disabled;
        let result = if overlay {
            event.deliver_overlay(&mut *self.component)
        } else {
            event.deliver(&mut *self.component)
        };
        self.record_actions(&result, event.action_source(), allowed, pending);
        result
    }
}
impl ContentHost {
    /// Opt into typed action collection before dispatch. Legacy hosts do not
    /// accumulate an unconsumed queue. Re-enabling is idempotent; disabling
    /// explicitly discards pending records. This never replays control counters.
    pub fn set_action_events(&mut self, enabled: bool) {
        if enabled {
            self.actions.get_or_insert_default();
        } else {
            self.actions = None;
        }
    }
    /// Take pending actions in actual component-delivery order, then each
    /// component's declared kind order. The returned Vec owns its data: handlers
    /// may redraw, focus, remove/reinsert or dispatch another event without a
    /// borrowed queue/callback. Empty drains never replay previous actions.
    /// Undrained records belonging to an unmounted/replaced instance
    /// are discarded, including a retained instance remounted under the same ID.
    /// Capture consumer-owned business context when handling the originating
    /// event; IDs/source never encode a Session/destination.
    pub fn drain_actions(&mut self) -> Vec<ContentAction> {
        self.drain_action_records()
            .into_iter()
            .map(|record| record.action)
            .collect()
    }
    /// Same queue with original mount and owned delivery data. Later model or
    /// selection changes cannot rewrite it. Unmount discards undrained records;
    /// either drain consumes the queue, so switching drains cannot replay it.
    pub fn drain_action_records(&mut self) -> Vec<ContentActionRecord> {
        self.actions
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|pending| {
                self.root
                    .find(pending.action.target)
                    .filter(|content| content.mount == pending.mount
                        && empty_state::authorized_action(&self.root, pending.action.target)
                        && notice::authorized_action(&self.root, pending.action.target)
                        && pending.payload.as_ref().is_none_or(|p| content.component.action_payload_current(p)))
                    .map(|_| ContentActionRecord {
                        action: pending.action,
                        mount: pending.mount,
                        payload: pending.payload,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    #[derive(Debug)]
    struct Producer { revision: Rc<Cell<u64>>, filter: bool }
    impl Component for Producer {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild { MeasuredChild::new(available) }
        fn semantics(&self) -> ContentSemantics { ContentSemantics::new(ContentRole::Button, "Produce") }
        fn accessibility_action(&mut self, _: Option<usize>, action: &AccessibilityAction) -> ContentResponse {
            ContentResponse { activated: matches!(action, AccessibilityAction::Press), ..Default::default() }
        }
        fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
            Some(ContentActionPayload::MenuSelection(MenuSelection { index: 0, label: "item".into(), model_revision: self.revision.get(), value_path: None }))
        }
        fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
            let ContentActionPayload::MenuSelection(value) = payload;
            !self.filter || value.model_revision() == self.revision.get()
        }
    }
    #[test]
    fn stale_payload_is_filtered_at_original_once_only_drain() {
        for filter in [false, true] {
            let revision = Rc::new(Cell::new(1));
            let mut h = ContentHost::new(Content::new(ComponentId(740), Producer { revision: revision.clone(), filter })).unwrap();
            h.set_action_events(true);
            h.accessibility_action(ComponentId(740), None, AccessibilityAction::Press);
            revision.set(2);
            assert_eq!(h.drain_action_records().len(), usize::from(!filter));
            assert!(h.drain_actions().is_empty());
            assert_eq!(revision.get(), 2);
        }
    }
    #[test]
    fn ordinary_owners_default_to_accepting_payloads() {
        let owner = TextContent::new("ordinary");
        let payload = ContentActionPayload::MenuSelection(MenuSelection { index: 0, label: "item".into(), model_revision: 1, value_path: None });
        assert!(owner.action_payload_current(&payload));
    }
}
