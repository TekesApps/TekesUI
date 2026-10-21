//! Opt-in, ordered provenance for the existing text-service callbacks.
//! No editor, focus, submission policy or runtime dependency lives here.
use crate::{KeyEvent, KeyEventKind};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextKeyContext {
    /// Process-unique native key-down span, including repeated key-downs.
    pub id: u64,
    pub key: KeyEvent,
    /// Captured before the input context can commit/unmark the conversion.
    pub composing_at_start: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextKeyDisposition {
    /// Commit/preedit/unmark callbacks were dispatched, not an editor acknowledgement.
    TextDispatched,
    /// At least one selector was dispatched; the receiving editor decides if handled.
    CommandDispatched,
    /// handleEvent returned true with no callbacks (e.g. input-method state change).
    InputContextConsumed,
    /// handleEvent returned false with no callbacks.
    Unhandled,
    /// Command keys or a missing input context were not offered to the input system.
    NotOffered,
}
/// Started and Finished enclose the ordinary TextCommit/TextPreedit/TextUnmark/
/// TextCommand (or unhandled Key) events in the same FIFO, including reentrancy.
/// Nesting uses IDs. Callbacks outside any span have no key provenance and must
/// never be guessed to represent a submit key. Existing callback delivery remains
/// once-only; consumers must not replay those edits on receipt of Finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextKeyEvent {
    Started(TextKeyContext),
    Finished {
        context: TextKeyContext,
        input_context_handled: Option<bool>,
        disposition: TextKeyDisposition,
        text_callbacks: u64,
        command_callbacks: u64,
    },
}
#[derive(Debug)]
struct Frame {
    context: TextKeyContext,
    text_callbacks: u64,
    command_callbacks: u64,
}
#[derive(Debug, Default)]
pub(crate) struct TextKeyTracker {
    enabled: bool,
    stack: Vec<Frame>,
}
impl TextKeyTracker {
    pub(crate) fn set_enabled(&mut self, enabled: bool) -> bool {
        if enabled == self.enabled {
            return true;
        }
        if !self.stack.is_empty() {
            return false;
        }
        self.enabled = enabled;
        true
    }
    pub(crate) fn begin(&mut self, key: KeyEvent, composing: bool) -> Option<TextKeyEvent> {
        if !self.enabled || key.kind != KeyEventKind::Down {
            return None;
        }
        let id = NEXT_KEY
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .expect("native text-key identity exhausted");
        let context = TextKeyContext {
            id,
            key,
            composing_at_start: composing,
        };
        self.stack.push(Frame {
            context: context.clone(),
            text_callbacks: 0,
            command_callbacks: 0,
        });
        Some(TextKeyEvent::Started(context))
    }
    pub(crate) fn note_callback(&mut self, command: bool) {
        if let Some(frame) = self.stack.last_mut() {
            if command {
                frame.command_callbacks = frame.command_callbacks.saturating_add(1);
            } else {
                frame.text_callbacks = frame.text_callbacks.saturating_add(1);
            }
        }
    }
    pub(crate) fn can_fallback(&self, id: u64) -> bool {
        self.stack.last().is_some_and(|frame| {
            frame.context.id == id && frame.text_callbacks == 0 && frame.command_callbacks == 0
        })
    }
    pub(crate) fn finish(&mut self, id: u64, handled: Option<bool>) -> Option<TextKeyEvent> {
        if self.stack.last().is_none_or(|frame| frame.context.id != id) {
            return None;
        }
        let frame = self.stack.pop().unwrap();
        let disposition = if frame.text_callbacks > 0 {
            TextKeyDisposition::TextDispatched
        } else if frame.command_callbacks > 0 {
            TextKeyDisposition::CommandDispatched
        } else {
            match handled {
                Some(true) => TextKeyDisposition::InputContextConsumed,
                Some(false) => TextKeyDisposition::Unhandled,
                None => TextKeyDisposition::NotOffered,
            }
        };
        Some(TextKeyEvent::Finished {
            context: frame.context,
            input_context_handled: handled,
            disposition,
            text_callbacks: frame.text_callbacks,
            command_callbacks: frame.command_callbacks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Modifiers;
    fn key() -> KeyEvent {
        KeyEvent {
            kind: KeyEventKind::Down,
            key_code: 36,
            characters: "\r".into(),
            characters_ignoring_modifiers: "\r".into(),
            modifiers: Modifiers {
                shift: true,
                command: true,
                alt: true,
                control: true,
                caps_lock: true,
                function: true,
            },
            is_repeat: true,
        }
    }
    fn start(t: &mut TextKeyTracker, marked: bool) -> TextKeyContext {
        let Some(TextKeyEvent::Started(c)) = t.begin(key(), marked) else {
            panic!("enabled tracker")
        };
        c
    }
    #[test]
    fn opt_in_preserves_all_key_metadata_and_starting_conversion_after_callbacks() {
        let mut t = TextKeyTracker::default();
        assert!(t.set_enabled(true));
        let c = start(&mut t, true);
        assert_eq!(c.key, key());
        t.note_callback(false);
        t.note_callback(false);
        t.note_callback(true);
        let Some(TextKeyEvent::Finished {
            context,
            input_context_handled,
            disposition,
            text_callbacks,
            command_callbacks,
        }) = t.finish(c.id, Some(true))
        else {
            panic!("finish")
        };
        assert_eq!(context, c);
        assert!(context.composing_at_start);
        assert_eq!(input_context_handled, Some(true));
        assert_eq!(disposition, TextKeyDisposition::TextDispatched);
        assert_eq!((text_callbacks, command_callbacks), (2, 1));
        assert!(t.finish(c.id, Some(true)).is_none());
    }
    #[test]
    fn default_is_legacy_and_outside_callbacks_are_not_assigned_to_a_future_key() {
        let mut t = TextKeyTracker::default();
        assert!(t.begin(key(), false).is_none());
        t.note_callback(false);
        assert!(t.set_enabled(true));
        let c = start(&mut t, false);
        assert!(matches!(
            t.finish(c.id, Some(false)),
            Some(TextKeyEvent::Finished {
                disposition: TextKeyDisposition::Unhandled,
                text_callbacks: 0,
                ..
            })
        ));
    }
    #[test]
    fn command_delivery_does_not_claim_edit_handled_or_ime_consumption() {
        let mut t = TextKeyTracker::default();
        t.set_enabled(true);
        let c = start(&mut t, false);
        t.note_callback(true);
        assert!(matches!(
            t.finish(c.id, Some(true)),
            Some(TextKeyEvent::Finished {
                disposition: TextKeyDisposition::CommandDispatched,
                text_callbacks: 0,
                command_callbacks: 1,
                ..
            })
        ));
        for (handled, want) in [
            (Some(true), TextKeyDisposition::InputContextConsumed),
            (Some(false), TextKeyDisposition::Unhandled),
            (None, TextKeyDisposition::NotOffered),
        ] {
            let c = start(&mut t, false);
            let Some(TextKeyEvent::Finished { disposition, .. }) = t.finish(c.id, handled) else {
                panic!("finish")
            };
            assert_eq!(disposition, want);
        }
    }
    #[test]
    fn nested_spans_keep_their_own_callbacks_and_cannot_change_mode_mid_key() {
        let mut t = TextKeyTracker::default();
        t.set_enabled(true);
        let a = start(&mut t, true);
        t.note_callback(false);
        let b = start(&mut t, false);
        assert_ne!(a.id, b.id);
        t.note_callback(true);
        assert!(!t.set_enabled(false));
        assert!(t.finish(a.id, Some(true)).is_none());
        assert!(matches!(
            t.finish(b.id, Some(true)),
            Some(TextKeyEvent::Finished {
                text_callbacks: 0,
                command_callbacks: 1,
                ..
            })
        ));
        t.note_callback(false);
        assert!(matches!(
            t.finish(a.id, Some(true)),
            Some(TextKeyEvent::Finished {
                text_callbacks: 2,
                command_callbacks: 0,
                ..
            })
        ));
        assert!(t.set_enabled(false));
    }
    #[test]
    fn callbacks_prevent_raw_fallback_even_when_input_context_returns_false() {
        let mut tracker = TextKeyTracker::default();
        tracker.set_enabled(true);
        assert!(!tracker.can_fallback(0));
        for command in [false, true] {
            let context = start(&mut tracker, false);
            assert!(tracker.can_fallback(context.id));
            assert!(!tracker.can_fallback(context.id + 1));
            tracker.note_callback(command);
            assert!(!tracker.can_fallback(context.id));
            let Some(TextKeyEvent::Finished { disposition, .. }) =
                tracker.finish(context.id, Some(false))
            else {
                panic!("finish")
            };
            assert_eq!(
                disposition,
                if command {
                    TextKeyDisposition::CommandDispatched
                } else {
                    TextKeyDisposition::TextDispatched
                }
            );
            assert!(!tracker.can_fallback(context.id));
        }
    }
    #[test]
    fn independent_hosts_and_repeat_downs_never_reuse_an_identity() {
        let mut a = TextKeyTracker::default();
        let mut b = TextKeyTracker::default();
        a.set_enabled(true);
        b.set_enabled(true);
        let first = start(&mut a, false);
        a.finish(first.id, Some(false));
        let repeat = start(&mut a, false);
        let other = start(&mut b, false);
        assert_ne!(first.id, repeat.id);
        assert_ne!(repeat.id, other.id);
        let mut up = key();
        up.kind = KeyEventKind::Up;
        assert!(a.begin(up, false).is_none());
    }
}
