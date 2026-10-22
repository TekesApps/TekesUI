//! Keyboard provenance and editor routing in the existing composition host.
use super::{ComponentId, ContentEvent, ContentHost, ContentResponse, InputModifiers};
use crate::ControlKey;

/// Lossless host key data. `key` is the portable control-key interpretation;
/// native code/characters remain available even when it is None.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentKeyEvent {
    pub key: Option<ControlKey>,
    pub down: bool,
    pub repeat: bool,
    pub modifiers: InputModifiers,
    pub native_code: u16,
    pub characters: String,
    pub characters_ignoring_modifiers: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTabPolicy {
    #[default]
    Navigate,
    /// Multiline editors insert a tab; Backtab is consumed without outdenting.
    Insert,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSubmitPolicy {
    #[default]
    None,
    Enter,
    CommandEnter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextKeyDisposition {
    InputConsumed,
    EditHandled,
    Submitted,
    Navigated,
    Unhandled,
    Cancelled,
}
/// The result acknowledges this key chain, not business delivery. Submitted
/// accompanies `activated`; the owner decides whether/where/how to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextKeyOutcome {
    pub id: u64,
    pub target: Option<ComponentId>,
    pub disposition: TextKeyDisposition,
}
#[derive(Debug)]
pub(super) struct KeyFrame {
    id: u64,
    owner: Option<ComponentId>,
    focus_epoch: u64,
    mount: Option<u64>,
    binding: Option<(u64, u64)>,
    key: ContentKeyEvent,
    composing: bool,
    text: bool,
    handled: bool,
    candidate: bool,
}
impl ContentHost {
    pub(super) fn overlay_command_key(&self, event: &ContentEvent) -> Option<ContentKeyEvent> {
        if self
            .focused_text()
            .is_some_and(|s| s.input.marked.is_some())
            || self
                .focused
                .and_then(|id| self.root.find(id))
                .is_some_and(|c| c.component.composing())
        {
            return None;
        }
        let ContentEvent::TextCommand(command) = event else {
            return None;
        };
        let key = match command.as_str() {
            "moveUp:" => ControlKey::Up,
            "moveDown:" => ControlKey::Down,
            _ => return None,
        };
        let mut input = self
            .text_keys
            .last()
            .map(|frame| frame.key.clone())
            .unwrap_or(ContentKeyEvent {
                key: None,
                down: true,
                repeat: false,
                modifiers: InputModifiers::default(),
                native_code: 0,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            });
        input.key = Some(key);
        input.down = true;
        Some(input)
    }
    fn key_owner_valid(
        &self,
        owner: Option<ComponentId>,
        epoch: u64,
        mount: Option<u64>,
        binding: Option<(u64, u64)>,
    ) -> bool {
        owner.is_some()
            && self.focused == owner
            && self.focus_epoch == epoch
            && owner
                .and_then(|id| self.root.find_active(id))
                .map(|c| c.mount)
                == mount
            && self.focused_text().map(|s| s.input.controlled_binding()) == binding
    }
    pub(super) fn key_routed_event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.reconcile_active_inputs();
        match event {
            ContentEvent::TextKeyStart { id, key, composing } => {
                // Native IDs are process-unique and increasing. Reject replayed
                // starts rather than allowing a late finish to submit twice.
                if *id <= self.last_text_key || !key.down {
                    return ContentResponse::consumed(false);
                }
                self.last_text_key = *id;
                self.text_keys.push(KeyFrame {
                    id: *id,
                    owner: self.focused,
                    focus_epoch: self.focus_epoch,
                    mount: self
                        .focused
                        .and_then(|id| self.root.find_active(id))
                        .map(|c| c.mount),
                    binding: self.focused_text().map(|s| s.input.controlled_binding()),
                    key: key.clone(),
                    composing: *composing
                        || self
                            .focused_text()
                            .is_some_and(|s| s.input.marked.is_some())
                        || self
                            .focused
                            .and_then(|id| self.root.find(id))
                            .is_some_and(|c| c.component.composing()),
                    text: false,
                    handled: false,
                    candidate: false,
                });
                return ContentResponse::consumed(false);
            }
            ContentEvent::TextKeyEnd {
                id,
                input_context_consumed,
            } => {
                if self.text_keys.last().is_none_or(|f| f.id != *id) {
                    return ContentResponse::default();
                }
                let frame = self.text_keys.pop().unwrap();
                let valid = self.key_owner_valid(
                    frame.owner,
                    frame.focus_epoch,
                    frame.mount,
                    frame.binding,
                );
                let composing_now = self
                    .focused_text()
                    .is_some_and(|s| s.input.marked.is_some())
                    || self
                        .focused
                        .and_then(|id| self.root.find(id))
                        .is_some_and(|c| c.component.composing());
                let blocked = frame.composing || composing_now || *input_context_consumed;
                let mut result = ContentResponse::consumed(false);
                let disposition = if !valid {
                    TextKeyDisposition::Cancelled
                } else if blocked {
                    TextKeyDisposition::InputConsumed
                } else if frame.handled {
                    TextKeyDisposition::EditHandled
                } else if frame.text {
                    TextKeyDisposition::InputConsumed
                } else if frame.candidate {
                    result = self.dispatch(&ContentEvent::KeyInput(frame.key));
                    if result.activated {
                        TextKeyDisposition::Submitted
                    } else if self.focused != frame.owner {
                        TextKeyDisposition::Navigated
                    } else if result.consumed {
                        TextKeyDisposition::EditHandled
                    } else {
                        TextKeyDisposition::Unhandled
                    }
                } else {
                    TextKeyDisposition::Unhandled
                };
                result.text_key = Some(TextKeyOutcome {
                    id: *id,
                    target: frame.owner,
                    disposition,
                });
                return result;
            }
            ContentEvent::Cancel => self.text_keys.clear(),
            _ => {}
        }
        if let Some(frame) = self.text_keys.last() {
            let owner = frame.owner;
            let focus_epoch = frame.focus_epoch;
            let mount = frame.mount;
            let binding = frame.binding;
            let id = frame.id;
            let text = matches!(
                event,
                ContentEvent::Commit { .. } | ContentEvent::Preedit { .. } | ContentEvent::Unmark
            );
            let command = matches!(event, ContentEvent::TextCommand(_));
            let candidate = match event {
                ContentEvent::KeyInput(key) if key.down && *key == frame.key => true,
                ContentEvent::Key { key, down: true } => Some(*key) == frame.key.key,
                ContentEvent::TextCommand(command) => match command.as_str() {
                    "insertNewline:" | "insertLineBreak:" => {
                        frame.key.key == Some(ControlKey::Enter)
                    }
                    // Numeric wrappers and text editors share this once-only
                    // boundary. A candidate-confirming arrow must not step a
                    // number after insertText has just cleared marked text.
                    "moveUp:" | "moveDown:" => {
                        let m = frame.key.modifiers;
                        !m.command
                            && !m.alt
                            && !m.control
                            && !m.shift
                            && frame.key.key
                                == Some(if command == "moveUp:" {
                                    ControlKey::Up
                                } else {
                                    ControlKey::Down
                                })
                    }
                    "insertTab:" => frame.key.key == Some(ControlKey::Tab { reverse: false }),
                    "insertBacktab:" => frame.key.key == Some(ControlKey::Tab { reverse: true }),
                    "cancelOperation:" => {
                        frame.key.key == Some(ControlKey::Escape)
                            && self.focused_text().is_none_or(|s| s.input.marked.is_none())
                    }
                    _ => false,
                },
                _ => false,
            };
            if candidate {
                self.text_keys.last_mut().unwrap().candidate = true;
                // Defer control-key effects until all callbacks have arrived.
                return ContentResponse::consumed(false);
            }
            if text
                || command
                || matches!(event, ContentEvent::KeyInput(_) | ContentEvent::Key { .. })
            {
                if text {
                    self.text_keys.last_mut().unwrap().text = true;
                }
                // A native callback must never edit a newly focused component.
                if !self.key_owner_valid(owner, focus_epoch, mount, binding) {
                    return ContentResponse::consumed(false);
                }
                let result = self.dispatch(event);
                if let Some(frame) = self.text_keys.last_mut().filter(|f| f.id == id) {
                    frame.handled |= result.consumed;
                }
                return result;
            }
        }
        self.dispatch(event)
    }
}
