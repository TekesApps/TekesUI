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
#[derive(Debug)]
pub(super) struct PendingAction {
    pub(super) action: ContentAction,
    pub(super) mount: u64,
}
static NEXT_MOUNT: AtomicU64 = AtomicU64::new(1);
pub(super) fn next_mount() -> u64 {
    NEXT_MOUNT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("content mount identity exhausted")
}
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
        self.actions
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|pending| {
                self.root
                    .find(pending.action.target)
                    .filter(|content| content.mount == pending.mount)
                    .map(|_| pending.action)
            })
            .collect()
    }
}
