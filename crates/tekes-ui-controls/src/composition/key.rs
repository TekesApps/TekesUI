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
    /// The original delegated key has not finished; no fallback may run yet.
    Deferred,
    InputConsumed,
    EditHandled,
    Submitted,
    Navigated,
    Unhandled,
    Cancelled,
}
/// The result acknowledges this key chain, not business delivery. Submitted
/// accompanies local `activated`; delegation only acknowledges the original
/// child's business delivery and must not emit a second parent activation.
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
    binding: Option<Vec<u8>>,
    delegated: bool,
    input_context_consumed: bool,
    key: ContentKeyEvent,
    composing: bool,
    text: bool,
    handled: bool,
    candidate: bool,
}
impl ContentHost {
    /// Check the pending host-key frame before its completion event removes it.
    /// Notification still requires an accepted result and the same reader lease.
    pub(super) fn completing_reading_navigation(&self, event: &ContentEvent) -> bool {
        let ContentEvent::TextKeyEnd { id, .. } = event else {
            return false;
        };
        self.text_keys.last().is_some_and(|frame| {
            frame.id == *id
                && matches!(
                    frame.key.key,
                    Some(
                        ControlKey::Left
                            | ControlKey::Right
                            | ControlKey::Up
                            | ControlKey::Down
                            | ControlKey::Home
                            | ControlKey::End
                    )
                )
        })
    }

    pub(super) fn advance_focus(&mut self, reverse: bool) -> ContentResponse {
        let mut ids = Vec::new();
        self.scope().focus_ids(&mut ids);
        if ids.is_empty() {
            return ContentResponse::default();
        }
        let next = match self
            .focused
            .and_then(|id| ids.iter().position(|v| *v == id))
        {
            Some(i)
                if !self.focus_wrap
                    && self.modal.is_none()
                    && self.overlay_scope_id().is_none()
                    && ((reverse && i == 0) || (!reverse && i + 1 == ids.len())) =>
            {
                return ContentResponse::default();
            }
            Some(i) if reverse => (i + ids.len() - 1) % ids.len(),
            Some(i) => (i + 1) % ids.len(),
            None if reverse => ids.len() - 1,
            None => 0,
        };
        let changed = matches!(
            self.admit_ordinary_focus(
                Some(ids[next]),
                true,
                Some(reverse),
                super::action_preparation::FocusOrigin::Traversal
            ),
            super::action_preparation::FocusAdmission::Accepted { changed: true }
        );
        ContentResponse::consumed(changed)
    }
    fn finish_delegated_key(
        &mut self,
        frame: &KeyFrame,
        mut result: ContentResponse,
        valid: bool,
    ) -> ContentResponse {
        let mut disposition = if !valid {
            TextKeyDisposition::Cancelled
        } else {
            result.text_key.filter(|key| key.id == frame.id).map_or(
                if result.consumed {
                    TextKeyDisposition::InputConsumed
                } else {
                    TextKeyDisposition::Cancelled
                },
                |key| key.disposition,
            )
        };
        if valid && frame.handled && disposition == TextKeyDisposition::Unhandled {
            disposition = TextKeyDisposition::EditHandled;
            result.consumed = true;
        }
        if valid
            && !frame.composing
            && !frame.input_context_consumed
            && disposition == TextKeyDisposition::Unhandled
            && !result.consumed
            && self.event_bubbled != Some(false)
            && let Some(ControlKey::Tab { reverse }) = frame.key.key
        {
            let local = frame
                .owner
                .and_then(|id| self.root.find_mut(id))
                .filter(|node| Some(node.mount) == frame.mount)
                .map(|node| node.component.focus_boundary(reverse))
                .unwrap_or_default();
            result = if local.consumed {
                local
            } else {
                self.advance_focus(reverse)
            };
            if result.consumed {
                disposition = TextKeyDisposition::Navigated;
            }
        }
        result.activated = false;
        result.text_key = Some(TextKeyOutcome {
            id: frame.id,
            target: frame.owner,
            disposition,
        });
        result
    }
    pub(super) fn reconcile_deferred_text_key(&mut self) {
        let Some(frame) = self.deferred_text_key.take() else {
            return;
        };
        // Outer ownership must still match. The delegate validates its original
        // receipt/binding: the completed key may itself have moved child focus.
        let mut valid = self.window_active
            && self.focused == frame.owner
            && self.focus_epoch == frame.focus_epoch
            && frame
                .owner
                .and_then(|id| self.scope().find_active(id))
                .is_some_and(|node| {
                    Some(node.mount) == frame.mount && !node.component.semantics().disabled
                });
        let response = frame
            .owner
            .and_then(|id| self.root.find_mut(id))
            .filter(|node| Some(node.mount) == frame.mount)
            .and_then(|node| {
                node.component
                    .take_text_key_result(frame.id, frame.binding.as_deref())
                    .map(|response| {
                        let bubbles = node.bubbles(
                            &ContentEvent::TextKeyEnd {
                                id: frame.id,
                                input_context_consumed: frame.input_context_consumed,
                            },
                            &response,
                        );
                        (response, bubbles)
                    })
            });
        let (response, bubbles) = if !valid {
            (ContentResponse::consumed(false), false)
        } else if let Some((response, bubbles)) = response.filter(|(response, _)| {
            response.text_key.is_some_and(|key| {
                key.id == frame.id && key.disposition != TextKeyDisposition::Deferred
            })
        }) {
            (response, bubbles)
        } else {
            self.deferred_text_key = Some(frame);
            return;
        };
        // An unhandled key did not itself move child focus. Re-check the
        // current binding before continuing outside that child; an unrelated
        // post-ack refocus/rebind must cancel the old boundary intent.
        if response
            .text_key
            .is_some_and(|key| key.disposition == TextKeyDisposition::Unhandled)
            && self.native_key_binding().as_deref() != frame.binding.as_deref()
        {
            valid = false;
        }
        let previous = self.event_bubbled;
        self.event_bubbled = Some(bubbles);
        let response = self.finish_delegated_key(&frame, response, valid);
        let bubbles = self.event_bubbled.unwrap_or(!response.consumed);
        self.event_bubbled = previous;
        self.dirty |= response.changed;
        self.completed_text_key = Some((response, bubbles));
    }
    /// Observe/drain the original latest deferred key acknowledgement. Superseded
    /// native IDs are Cancelled; this query never executes the key a second time.
    pub fn text_key_result(&mut self, id: u64) -> ContentResponse {
        self.reconcile_active_inputs();
        if self
            .completed_text_key
            .as_ref()
            .is_some_and(|(response, _)| response.text_key.is_some_and(|key| key.id == id))
        {
            let (response, bubbles) = self.completed_text_key.take().unwrap();
            self.event_bubbled = Some(bubbles);
            return response;
        }
        let pending = self
            .deferred_text_key
            .as_ref()
            .filter(|frame| frame.id == id);
        let mut response = ContentResponse::consumed(false);
        response.text_key = Some(TextKeyOutcome {
            id,
            target: pending.and_then(|frame| frame.owner),
            disposition: if pending.is_some() {
                TextKeyDisposition::Deferred
            } else {
                TextKeyDisposition::Cancelled
            },
        });
        self.event_bubbled = Some(false);
        response
    }
    /// Claim one external clipboard command in the original native chain.
    /// Returns the same original read-only text snapshot used for this claim;
    /// callbacks from a refocused/rebound owner and duplicate commands fail.
    /// IME commits are not deduplicated by this window-service boundary.
    pub fn claim_clipboard_input(&mut self) -> Option<super::ContentTextInput> {
        self.reconcile_active_inputs();
        let snapshot = self.text_input_snapshot()?;
        if snapshot.marked.is_some() {
            return None;
        }
        if let Some(frame) = self.text_keys.last() {
            if frame.handled
                || self.focused != frame.owner
                || self.focus_epoch != frame.focus_epoch
                || frame
                    .owner
                    .and_then(|id| self.scope().find_active(id))
                    .map(|node| node.mount)
                    != frame.mount
                || frame.binding.as_deref() != Some(snapshot.binding.as_slice())
            {
                return None;
            }
            self.text_keys.last_mut().unwrap().handled = true;
        }
        Some(snapshot)
    }
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
        binding: Option<&[u8]>,
    ) -> bool {
        owner.is_some()
            && self.focused == owner
            && self.focus_epoch == epoch
            && owner
                .and_then(|id| self.root.find_active(id))
                .map(|c| c.mount)
                == mount
            && self.native_key_binding().as_deref() == binding
    }
    fn deliver_native_owner(
        &mut self,
        owner: Option<ComponentId>,
        mount: Option<u64>,
        event: &ContentEvent,
    ) -> ContentResponse {
        let Some(node) = owner
            .and_then(|id| self.root.find_mut(id))
            .filter(|n| Some(n.mount) == mount)
        else {
            return ContentResponse::default();
        };
        // The original nested host owns its business queue. No second parent
        // action is recorded; native acknowledgement remains independent.
        let mut result = event.deliver(&mut *node.component);
        result.activated = false;
        self.event_bubbled = Some(node.bubbles(event, &result));
        result
    }
    pub(super) fn key_routed_event(&mut self, event: &ContentEvent) -> ContentResponse {
        // Cancellation revokes pending navigation before any ready result can
        // be reconciled, including the original native event-with-text path.
        if matches!(event, ContentEvent::Cancel) {
            self.text_keys.clear();
            self.deferred_text_key = None;
            self.completed_text_key = None;
        }
        self.reconcile_active_inputs();
        match event {
            ContentEvent::TextKeyStart { id, key, composing } => {
                // Native IDs are process-unique and increasing. Reject replayed
                // starts rather than allowing a late finish to submit twice.
                if *id <= self.last_text_key || !key.down {
                    return ContentResponse::consumed(false);
                }
                self.last_text_key = *id;
                // A newer native chain supersedes an unresolved parent
                // continuation, never replaying an older navigation later.
                self.deferred_text_key = None;
                self.completed_text_key = None;
                self.text_keys.push(KeyFrame {
                    id: *id,
                    owner: self.focused,
                    focus_epoch: self.focus_epoch,
                    mount: self
                        .focused
                        .and_then(|id| self.root.find_active(id))
                        .map(|c| c.mount),
                    binding: self.native_key_binding(),
                    delegated: self
                        .focused
                        .and_then(|id| self.root.find_active(id))
                        .is_some_and(|c| c.component.delegates_text_keys()),
                    input_context_consumed: false,
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
                self.help.suppress();
                if self.text_keys.last().is_some_and(|f| f.delegated) {
                    let frame = self.text_keys.last().unwrap();
                    return self.deliver_native_owner(frame.owner, frame.mount, event);
                }
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
                    frame.binding.as_deref(),
                ) || (frame.owner.is_none()
                    && frame.mount.is_none()
                    && frame.binding.is_none()
                    && self.window_active
                    && self.focused.is_none()
                    && frame.focus_epoch == self.focus_epoch
                    && frame.candidate
                    && matches!(frame.key.key, Some(ControlKey::Tab { .. }))
                    && !frame.key.modifiers.command
                    && !frame.key.modifiers.alt
                    && !frame.key.modifiers.control);
                if frame.delegated {
                    let cleanup = ContentEvent::TextKeyEnd {
                        id: *id,
                        input_context_consumed: *input_context_consumed || !valid,
                    };
                    let mut result = self.deliver_native_owner(frame.owner, frame.mount, &cleanup);
                    let mut frame = frame;
                    frame.input_context_consumed = *input_context_consumed;
                    if valid
                        && result.text_key.is_some_and(|key| {
                            key.id == *id && key.disposition == TextKeyDisposition::Deferred
                        })
                    {
                        result.activated = false;
                        result.text_key.as_mut().unwrap().target = frame.owner;
                        self.deferred_text_key = Some(frame);
                        return result;
                    }
                    return self.finish_delegated_key(&frame, result, valid);
                }
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
            _ => {}
        }
        if let Some(frame) = self.text_keys.last() {
            let owner = frame.owner;
            let focus_epoch = frame.focus_epoch;
            let mount = frame.mount;
            let binding = frame.binding.clone();
            let delegated = frame.delegated;
            let id = frame.id;
            let text = matches!(
                event,
                ContentEvent::Commit { .. } | ContentEvent::Preedit { .. } | ContentEvent::Unmark
            );
            let command = matches!(event, ContentEvent::TextCommand(_));
            if delegated
                && (text
                    || command
                    || matches!(event, ContentEvent::KeyInput(_) | ContentEvent::Key { .. }))
            {
                if !self.key_owner_valid(owner, focus_epoch, mount, binding.as_deref()) {
                    return ContentResponse::consumed(false);
                }
                return self.deliver_native_owner(owner, mount, event);
            }
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
                if !self.key_owner_valid(owner, focus_epoch, mount, binding.as_deref()) {
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
