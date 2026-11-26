use super::content_node::{TextWheelDispatch, TextWheelLease};
use super::*;
use crate::{ControlKey, TextEditCommand};
use std::collections::BTreeSet;
use tekes_ui_scene::Point;

impl ContentHost {
    pub(super) fn dispatch(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(
            event.routing_event().as_ref(),
            ContentEvent::PointerDown(_)
                | ContentEvent::PointerMove(_)
                | ContentEvent::PointerUp(_)
                | ContentEvent::Scroll { .. }
                | ContentEvent::Cancel
        ) {
            self.list_navigation_reveal = None;
        }
        if !self.split_structures_intact() {
            return ContentResponse::default();
        }
        if self.split_input_guard(event) {
            return ContentResponse::consumed(false);
        }
        if let ContentEvent::TextCommand(command) = event {
            let key = match command {
                TextEditCommand::InsertTab => Some(ControlKey::Tab { reverse: false }),
                TextEditCommand::InsertBacktab => Some(ControlKey::Tab { reverse: true }),
                TextEditCommand::CancelOperation
                    if self.focused_text().is_none_or(|s| s.input.marked.is_none()) =>
                {
                    Some(ControlKey::Escape)
                }
                _ => None,
            };
            if let Some(key) = key {
                return self.dispatch(&ContentEvent::Key { key, down: true });
            }
        }
        if !matches!(event, ContentEvent::Cancel) && self.preflight_notice_structures().is_err() {
            return ContentResponse::default();
        }
        if self.root.ids(&mut BTreeSet::new()).is_err() {
            self.cancel();
            return ContentResponse::default();
        }
        self.reconcile_active_inputs();
        // A stale spatial lease rejects positioned input, not keyboard or
        // lifecycle commands such as Escape closing the current modal scope.
        if matches!(
            event,
            ContentEvent::Pointer(_)
                | ContentEvent::PointerDown(_)
                | ContentEvent::PointerMove(_)
                | ContentEvent::PointerUp(_)
                | ContentEvent::Scroll { .. }
        ) && event.is_primary_pointer()
            && self.requires_curve_geometry()
            && self
                .geometry_snapshot()
                .is_none_or(|snapshot| !snapshot.is_current())
        {
            self.event_bubbled = Some(false);
            return ContentResponse::default();
        }
        if matches!(event, ContentEvent::Cancel) {
            self.cancel();
            return ContentResponse::consumed(true);
        }
        let routing = event.routing_event();
        let routed = routing.as_ref();
        if matches!(
            routed,
            ContentEvent::PointerDown(_) | ContentEvent::PointerUp(_)
        ) && self.pointer_capture().is_some()
            && event.pointer_button() != self.captured_button
        {
            // An additional button must neither replace the current gesture nor
            // reach its owner as an unrelated release through hit-test fallback.
            return ContentResponse::default();
        }
        if event.is_primary_pointer() && matches!(routed, ContentEvent::PointerDown(_)) {
            self.retired_combo_release = false;
        }
        if event.is_primary_pointer()
            && matches!(routed, ContentEvent::PointerUp(_))
            && self.retired_combo_release
        {
            self.retired_combo_release = false;
            return ContentResponse::consumed(false);
        }
        if matches!(routed, ContentEvent::PointerDown(_))
            && event.is_primary_pointer()
            && self.capture_epoch == u64::MAX
        {
            self.event_bubbled = Some(false);
            return ContentResponse::default();
        }
        if let Some(response) = self.context_menu_event(event) {
            return response;
        }
        if self.help_event(event) {
            return ContentResponse::consumed(true);
        }
        // The composing editor owns control keys before any overlay. Preserve
        // its existing Escape/caret behavior; unhandled keys remain available to
        // the native input method. Text commits/commands still route normally.
        if matches!(
            routed,
            ContentEvent::Key { .. }
                | ContentEvent::TextCommand(_)
                | ContentEvent::Commit { .. }
                | ContentEvent::Preedit { .. }
                | ContentEvent::Unmark
        ) && (self
            .focused_text()
            .is_some_and(|surface| surface.input.marked.is_some())
            || self
                .focused
                .and_then(|id| self.root.find(id))
                .is_some_and(|content| content.component.composing()))
        {
            return self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|content| content.deliver_actions(event, false, &mut self.actions))
                .unwrap_or_default();
        }
        // Text callbacks always belong to the actual focused text owner, not
        // to an unrelated menu overlay (including the first preedit callback).
        if matches!(
            event,
            ContentEvent::Commit { .. } | ContentEvent::Preedit { .. } | ContentEvent::Unmark
        ) && self.focused_text().is_some()
        {
            return self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|c| c.deliver_actions(event, false, &mut self.actions))
                .unwrap_or_default();
        }
        // A content drag keeps ownership until release, even if that gesture
        // opens an overlay. Candidate surfaces must not steal selection capture.
        if matches!(
            routed,
            ContentEvent::PointerMove(_) | ContentEvent::PointerUp(_)
        ) && event.pointer_button() == self.captured_button
            && let Some(id) = self.captured
        {
            let captured_overlay = self.captured_overlay;
            let captured_overlay_handler = self.captured_overlay_handler;
            if matches!(routed, ContentEvent::PointerUp(_)) {
                self.captured = None;
                self.captured_mount = None;
                self.captured_overlay = false;
                self.captured_overlay_handler = false;
            }
            let result = if captured_overlay_handler {
                self.root.find_mut(id).map(|node| {
                    let response = node.deliver_actions(event, true, &mut self.actions);
                    (id, response, false)
                })
            } else if captured_overlay {
                let (scope, actions) = self.scope_with_actions();
                scope.route_from(id, event, actions)
            } else {
                self.root.route_from(id, event, &mut self.actions)
            };
            return result
                .map(|(_, response, bubbles)| {
                    self.event_bubbled = Some(bubbles);
                    response
                })
                .unwrap_or_default();
        }
        if let Some(id) = self.scope().overlay_id() {
            let response =
                self.root
                    .find_mut(id)
                    .unwrap()
                    .deliver_actions(event, true, &mut self.actions);
            if response.consumed {
                // The original overlay owner owns this down and its release.
                // Existing Combo-specific capture lifecycle validation is unchanged.
                if response.capture_pointer
                    && matches!(routed, ContentEvent::PointerDown(_))
                    && event.is_primary_pointer()
                {
                    self.capture_epoch = self
                        .capture_epoch
                        .checked_add(1)
                        .expect("pointer capture identity exhausted");
                    self.captured = Some(id);
                    self.captured_button = event.pointer_button();
                    self.captured_mount = self.root.find_active(id).map(|node| node.mount);
                    self.captured_overlay = true;
                    self.captured_overlay_handler = true;
                }
                self.record_reading_scroll(event, id, &response);
                return response;
            }
            if let Some(key) = self.overlay_command_key(event) {
                let response = self.root.find_mut(id).unwrap().deliver_actions(
                    &ContentEvent::KeyInput(key),
                    true,
                    &mut self.actions,
                );
                if response.consumed {
                    return response;
                }
            }
        }
        let pointer_down = match event {
            ContentEvent::PointerDown(point) => Some(*point),
            ContentEvent::Pointer(pointer)
                if pointer.phase == PointerPhase::Down && pointer.is_primary() =>
            {
                Some(pointer.position)
            }
            _ => None,
        };
        if let Some(point) = pointer_down
            && let Some(response) = self.try_prepared_pointer(event, point)
        {
            return response;
        }
        if matches!(
            routed,
            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true
            }
        ) && self.modal.is_some()
            && self.focused_text().is_none_or(|s| s.input.marked.is_none())
        {
            self.close_dialog();
            return ContentResponse::consumed(true);
        }
        if let ContentEvent::Key {
            key: ControlKey::Tab { reverse },
            down: true,
        } = routed
        {
            if self
                .focused
                .and_then(|id| self.root.find(id))
                .is_some_and(|c| c.component.composing())
            {
                return ContentResponse::default();
            }
            // An editor may explicitly consume Tab (for indentation). Only
            // an unhandled key reaches the existing focus traversal fallback.
            if let Some((response, bubbles)) =
                self.focused.and_then(|id| self.root.find_mut(id)).map(|c| {
                    let response = c.deliver_actions(event, false, &mut self.actions);
                    let bubbles = c.bubbles(event, &response);
                    (response, bubbles)
                })
                && (response.consumed || !bubbles)
            {
                self.event_bubbled = Some(bubbles);
                return response;
            }
            if let Some(response) = self
                .focused
                .and_then(|id| self.root.find_mut(id))
                .map(|node| node.component.focus_boundary(*reverse))
                .filter(|response| response.consumed)
            {
                return response;
            }
            return self.advance_focus(*reverse);
        }
        if let ContentEvent::PointerMove(position) = routed {
            let (scope, actions) = self.scope_with_actions();
            let (hit, changed) = scope.move_pointer(event, *position, actions);
            let (_, mut response, bubbles) =
                hit.unwrap_or((ComponentId(0), ContentResponse::default(), true));
            response.changed |= changed;
            self.event_bubbled = Some(bubbles);
            return response;
        }
        if let Some(id) = self.padding_focus_target(event) {
            // Resolve before ordinary pointer dispatch. In particular there is
            // no blur/callback/refocus gap, and repeated padding clicks do not
            // re-run set_focus/reveal against a composing editor.
            let changed = if self.focused == Some(id) {
                self.pointer_focus_requested = true;
                false
            } else if let action_preparation::FocusAdmission::Accepted { changed } = self
                .admit_ordinary_focus(
                    Some(id),
                    false,
                    None,
                    action_preparation::FocusOrigin::Direct,
                )
            {
                self.pointer_focus_requested = true;
                changed
            } else {
                false
            };
            return ContentResponse::consumed(changed);
        }
        let result = match routed {
            ContentEvent::Scroll { position, .. } => {
                let target = self
                    .text_wheel_dispatch
                    .and(self.text_wheel.as_ref().map(|lease| lease.target));
                let (scope, actions) = self.scope_with_actions();
                if let Some(target) = target {
                    scope.route_text_wheel_from(target, event, actions)
                } else {
                    scope.route(event, *position, actions)
                }
            }
            ContentEvent::PointerUp(p) | ContentEvent::PointerDown(p) => {
                let (scope, actions) = self.scope_with_actions();
                scope.route(event, *p, actions)
            }
            _ => self.focused.and_then(|id| {
                let (scope, actions) = self.scope_with_actions();
                scope.route_from(id, event, actions)
            }),
        };
        if let Some((id, mut r, bubbles)) = result {
            if r.consumed
                && matches!(
                    routed,
                    ContentEvent::Key {
                        key: ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End,
                        down: true,
                    }
                )
                && self.queue_list_navigation_reveal(id)
            {
                // At a logical boundary the reducer can be unchanged while wheel
                // input has moved that row offscreen. Request its normal layout.
                r.changed = true;
            }
            if matches!(self.text_wheel_dispatch, Some(TextWheelDispatch::Start))
                && r.consumed
                && self
                    .scope()
                    .find_active(id)
                    .is_some_and(|node| node.component::<TextFieldContent>().is_some())
            {
                self.text_wheel = self.scope().text_wheel_path(id).map(|path| TextWheelLease {
                    target: id,
                    path,
                    focus_epoch: self.focus_epoch,
                    awaiting_momentum: false,
                });
            }
            self.record_reading_scroll(event, id, &r);
            self.event_bubbled = Some(bubbles);
            if matches!(routed, ContentEvent::PointerDown(_)) {
                if r.capture_pointer {
                    self.capture_epoch = self
                        .capture_epoch
                        .checked_add(1)
                        .expect("pointer capture identity exhausted");
                    self.captured = Some(id);
                    self.captured_button = event.pointer_button();
                    self.captured_mount = self.root.find_active(id).map(|node| node.mount);
                    self.captured_overlay = self.overlay_scope_id().is_some_and(|scope| {
                        self.root
                            .find_active(scope)
                            .is_some_and(|owner| owner.find(id).is_some())
                    });
                    self.captured_overlay_handler = false;
                }
                if event.is_primary_pointer()
                    && r.consumed
                    && self.root.find(id).is_some_and(|c| {
                        c.component.focusable()
                            && !c.preserve_activation_focus
                            && !c.component.preserves_activation_focus()
                    })
                {
                    self.pointer_focus_requested = matches!(
                        self.admit_ordinary_focus(
                            Some(id),
                            false,
                            None,
                            action_preparation::FocusOrigin::Direct
                        ),
                        action_preparation::FocusAdmission::Accepted { .. }
                    );
                }
            }
            r
        } else {
            if self.modal.is_some() {
                ContentResponse::consumed(false)
            } else {
                ContentResponse::default()
            }
        }
    }
    pub(super) fn base_scope(&self) -> &Content {
        self.modal
            .and_then(|(id, _)| self.root.find(id))
            .unwrap_or(&self.root)
    }
    pub(super) fn scope(&self) -> &Content {
        self.overlay_scope_id()
            .and_then(|id| self.root.find_active(id))
            .unwrap_or_else(|| self.base_scope())
    }
    pub(super) fn scope_with_actions(
        &mut self,
    ) -> (&mut Content, &mut Option<Vec<actions::PendingAction>>) {
        let scope = if let Some(id) = self.overlay_scope_id().or(self.modal.map(|(id, _)| id)) {
            self.root.find_mut(id).expect("mounted input scope")
        } else {
            &mut self.root
        };
        (scope, &mut self.actions)
    }
    /// Current original interactive overlay/modal authority. A retained exit
    /// painter has no input authority, and an inactive window publishes none.
    pub fn input_overlay_owner(&self) -> Option<(ComponentId, u64)> {
        if !self.window_active {
            return None;
        }
        let id = self
            .overlay_scope_id()
            .or(self.modal.map(|(id, _)| id))
            .or_else(|| self.scope().overlay_id())?;
        let node = self.root.find_active(id)?;
        Some((id, node.mount))
    }
    /// Original native key owner binding, retained across geometry-only changes
    /// and zero pane allocation. Focus epoch and mount authenticate key frames;
    /// editor/document identity remains owned by the original component.
    pub fn native_key_binding(&self) -> Option<Vec<u8>> {
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(self.focused?)?;
        if owner.component.semantics().disabled || owner.component.semantics().role.is_selection() {
            return None;
        }
        owner.component.native_key_binding()
    }
    /// Current UTF-16 editing/geometry source for a platform's text query bridge.
    pub fn focused_text(&self) -> Option<&crate::TextSurface> {
        self.focused.and_then(|id| self.text_surface(id))
    }
    /// Existing editor/document binding, unchanged by ordinary edits or selection.
    pub fn focused_text_binding(&self) -> Option<(u64, u64)> {
        self.focused_text()
            .map(|surface| surface.input.controlled_binding())
    }
    /// Read the original focused text owner without editing or reshaping it.
    pub fn text_input_snapshot(&self) -> Option<ContentTextInput> {
        self.semantic_text_input_snapshot(self.focused?)
    }
    /// Read the original semantic owner's text and visible caret geometry.
    pub fn semantic_text_input_snapshot(&self, id: ComponentId) -> Option<ContentTextInput> {
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        if owner.component.semantics().disabled || owner.component.semantics().role.is_selection() {
            return None;
        }
        let mut input = owner.component.text_input_snapshot()?;
        // Zero panes retain the original logical editor, selection and marked
        // state. They must not publish retained geometry from an earlier frame.
        if self.split_text_geometry_hidden(id) {
            input.caret = None;
            input.clip = None;
            input.carets.clear();
        }
        Some(input)
    }
    /// Complete original text geometry, including off-focus and offscreen text.
    /// Zero Split panes retain logical editing state but cannot publish old layout.
    pub fn semantic_text_layout_snapshot(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<ContentTextLayout> {
        if !self.window_active || self.split_text_geometry_hidden(id) {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        // Disabled controls remain readable to accessibility. This query grants
        // no input authority; accessibility_action retains the disabled guard.
        if let Some(part) = part {
            let layout = owner.component.semantic_part_text_layout(part)?;
            if !owner
                .component
                .semantic_parts()
                .iter()
                .any(|p| p.index == part && !p.semantics.role.is_selection())
            {
                return None;
            }
            Some(layout)
        } else {
            if owner.component.semantics().role.is_selection() {
                return None;
            }
            owner.component.semantic_text_layout()
        }
    }
    /// Published text queries for active semantic owners, including unfocused
    /// fields. Closed or background modal subtrees cannot expose editable text.
    pub fn text_surface(&self, id: ComponentId) -> Option<&crate::TextSurface> {
        self.scope().find_active(id).and_then(|c| {
            (!c.component.semantics().role.is_selection())
                .then(|| c.component.text_surface())
                .flatten()
        })
    }
    /// Current active owner's process-local mount identity for native bridges.
    /// Reorder retains it; replacement/remount changes it even for the same ID.
    /// This is not a persistent identifier and does not acquire focus or input.
    pub fn semantic_mount(&self, id: ComponentId) -> Option<u64> {
        if !self.window_active {
            return None;
        }
        self.scope().find_active(id).map(|node| node.mount)
    }
    /// Read the existing focus lease, including blur/refocus ABA.
    pub fn animation_frame_requested(&self) -> bool {
        self.window_active && self.animation_frame_requested
    }
    pub fn focus_epoch(&self) -> u64 {
        self.focus_epoch
    }
    /// Current original capture owner and mount; no synthetic gesture identity.
    pub fn pointer_capture(&self) -> Option<(ComponentId, u64)> {
        if self.requires_curve_geometry()
            && self
                .geometry_snapshot()
                .is_none_or(|snapshot| !snapshot.is_current())
        {
            return None;
        }
        self.captured.and_then(|id| {
            self.root
                .find_active(id)
                .filter(|node| {
                    self.captured_mount == Some(node.mount)
                        && node.visible().is_some()
                        && !node.component.semantics().disabled
                        && node.component.pointer_capture_current()
                        && node
                            .component::<ComboBoxContent>()
                            .is_none_or(|combo| combo.capture_current(self.captured_overlay))
                })
                .filter(|node| {
                    node.component::<CheckboxContent>()
                        .is_none_or(|check| check.focusable())
                })
                .map(|node| (id, node.mount))
        })
    }
    /// The button belonging to the current original capture. Window hosts use
    /// this to route secondary drags to the same ContentHost outside its bounds.
    pub fn pointer_capture_button(&self) -> Option<PointerButton> {
        self.pointer_capture().map(|_| self.captured_button)
    }
    /// Same validated original capture, with its existing acquisition epoch for
    /// asynchronous Content callbacks. The legacy pair query stays unchanged.
    pub fn pointer_capture_lease(&self) -> Option<(ComponentId, u64, u64)> {
        self.pointer_capture()
            .map(|(id, mount)| (id, mount, self.capture_epoch))
    }
    /// The existing focused editor/custom component composition barrier.
    pub fn is_composing(&self) -> bool {
        self.focused
            .and_then(|id| self.root.find_active(id))
            .is_some_and(|node| node.component.composing())
            || self
                .focused_text()
                .is_some_and(|s| s.input.marked.is_some())
    }
    /// Original window activity for platform publication admission.
    pub fn is_window_active(&self) -> bool {
        self.window_active
    }
    /// Revoke adapter authorization recursively, including hidden descendants.
    pub fn set_window_active(&mut self, active: bool) {
        if self.window_active == active {
            return;
        }
        let deactivated = self.window_active && !active;
        // Cancel while old input authority is still live, then update retained descendants.
        self.reveal_clock.invalidate_input();
        // Retire the transient session with its precise reason before generic cancellation.
        let mut ids = Vec::new();
        self.root.popover_ids(&mut ids);
        for id in ids {
            if let Some(popover) = self
                .root
                .find_mut(id)
                .and_then(|node| node.component_mut::<PopoverContent>())
            {
                popover.host_window_active(active);
            }
        }
        if deactivated {
            self.cancel();
        }
        self.window_active = active;
        self.root.refresh_activity(active);
        self.dirty = true;
    }
    /// Cancel only this original subtree before its typed declaration changes.
    pub fn cancel_subtree(&mut self, id: ComponentId) -> bool {
        let Some(node) = self.root.find(id) else {
            return false;
        };
        let focus = self.focused.is_some_and(|id| node.find(id).is_some());
        let capture = self.captured.is_some_and(|id| node.find(id).is_some());
        let wheel = self
            .text_wheel
            .as_ref()
            .is_some_and(|lease| node.find(lease.target).is_some());
        for scope in &mut self.overlay_scopes {
            if node.find(scope.id).is_some() {
                scope.restore = None;
                scope.focus_pending = false;
            }
        }
        fn cancel(node: &mut Content) {
            node.component.event(&ContentEvent::Cancel);
            node.component
                .event(&ContentEvent::PointerMove(Point::new(f32::MIN, f32::MIN)));
            if let Some(children) = node.component.children_mut() {
                for child in children {
                    cancel(child);
                }
            }
        }
        if wheel {
            self.retire_text_wheel();
        }
        cancel(self.root.find_mut(id).unwrap());
        if capture {
            self.captured = None;
            self.captured_mount = None;
            self.captured_overlay = false;
            self.captured_overlay_handler = false;
        }
        if focus {
            self.focus(None, false);
        }
        self.help.clear();
        self.dirty = true;
        self.reconcile_active_inputs();
        true
    }
    /// Read the existing original owner's part hierarchy; no focus or input.
    pub fn semantic_part_parent(&self, id: ComponentId, part: usize) -> Option<usize> {
        self.scope()
            .find_active(id)?
            .component
            .semantic_part_parent(part)
    }
    pub fn semantic_preserves_activation_focus(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> bool {
        self.scope().find_active(id).is_some_and(|node| {
            node.preserve_activation_focus
                || node.component.preserves_activation_focus()
                || part.is_some_and(|part| {
                    node.component
                        .semantic_part_preserves_activation_focus(part)
                })
        })
    }
    /// Actions use the same semantic owner and production state machine as input.
    /// Closed/dismissed/inactive subtrees and stale menu parts reject requests.
    pub fn accessibility_action(
        &mut self,
        id: ComponentId,
        part: Option<usize>,
        action: AccessibilityAction,
    ) -> ContentResponse {
        if self.split_ax_guard(id) {
            return ContentResponse::default();
        }
        if !self.window_active {
            return ContentResponse::default();
        }
        if self.is_composing()
            && self.focused != Some(id)
            && (matches!(action, AccessibilityAction::Focus)
                || (matches!(action, AccessibilityAction::Press)
                    && self.scope().find_active(id).is_some_and(|n| {
                        n.component.semantics().role.is_selection()
                            || n.component.semantics().role == ContentRole::Select
                    })
                    && !self.semantic_preserves_activation_focus(id, part)))
        {
            return ContentResponse::default();
        }
        // Validate finite selection authority before focus transfer or owner mutation.
        // The original IME/scope gate above still owns admission.
        let Some(node) = self.scope().find_active(id) else {
            // A closed scope still needs its original return-focus reconciliation.
            // Retire that lifecycle state without dispatching the stale action.
            self.reconcile_active_inputs();
            return ContentResponse::default();
        };
        let root = node.component.semantics();
        // Tabs use their own retained selection model, so they intentionally
        // are not in the Checkbox/Radio/Segment transaction classification.
        // Their native AX parts still need this local admission boundary.
        let tab_list = root.role == ContentRole::TabList;
        let selection_owner = root.role.is_selection() || tab_list;
        if selection_owner
            && ((!root.valid_selection(None) || (tab_list && root.expanded.is_some()))
                && !matches!(root.role, ContentRole::RadioButton | ContentRole::Segment))
        {
            return ContentResponse::default();
        }
        if selection_owner
            && (node.component.text_input_snapshot().is_some()
                || node.component.semantic_text_layout().is_some()
                || node.component.value_range().is_some())
        {
            return ContentResponse::default();
        }
        let parts = node.component.semantic_parts();
        let selected_part = part.and_then(|i| parts.iter().find(|p| p.index == i));
        let auxiliary_tab_action = tab_list
            && matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
            && part.is_some_and(|index| {
                node.component.semantic_part_is_auxiliary_action(index)
                    && selected_part.is_some_and(|p| {
                        p.semantics.role == ContentRole::Button
                            && !p.semantics.disabled
                            && !p.frame.is_empty()
                            && p.frame
                                .intersection(&node.clip)
                                .is_some_and(|r| !r.is_empty())
                            && node.component.semantic_part_text_layout(index).is_none()
                            && node.component.semantic_part_value_range(index).is_none()
                    })
                    && node
                        .component
                        .semantic_part_parent(index)
                        .is_some_and(|parent| {
                            node.component.semantic_part_text_layout(parent).is_none()
                                && node.component.semantic_part_value_range(parent).is_none()
                                && parts.iter().any(|p| {
                                    p.index == parent
                                        && p.semantics.role == ContentRole::Tab
                                        && p.semantics.valid_selection(Some(ContentRole::TabList))
                                        && p.semantics.expanded.is_none()
                                        && !p.semantics.disabled
                                        && !p.frame.is_empty()
                                        && p.frame
                                            .intersection(&node.clip)
                                            .is_some_and(|r| !r.is_empty())
                                })
                        })
            });
        if (selection_owner && !auxiliary_tab_action)
            || selected_part.is_some_and(|p| p.semantics.role.is_selection())
        {
            let (s, parent, visible, extra) = if let Some(i) = part {
                let Some(p) = selected_part else {
                    return ContentResponse::default();
                };
                if !(p.semantics.role.is_selection()
                    || tab_list && p.semantics.role == ContentRole::Tab)
                {
                    return ContentResponse::default();
                }
                let parent = match node.component.semantic_part_parent(i) {
                    Some(parent) => parts
                        .iter()
                        .find(|p| p.index == parent)
                        .map(|p| p.semantics.role),
                    None => Some(root.role),
                };
                (
                    &p.semantics,
                    parent,
                    !p.frame.is_empty()
                        && p.frame
                            .intersection(&node.clip)
                            .is_some_and(|r| !r.is_empty()),
                    node.component.semantic_part_text_layout(i).is_some()
                        || node.component.semantic_part_value_range(i).is_some(),
                )
            } else {
                let parent = self
                    .semantics()
                    .iter()
                    .find(|n| n.id == id)
                    .and_then(|n| n.parent)
                    .and_then(|id| self.scope().find_active(id))
                    .map(|n| n.component.semantics().role);
                (
                    &root,
                    parent,
                    node.frame
                        .intersection(&node.clip)
                        .is_some_and(|r| !r.is_empty()),
                    node.component.text_input_snapshot().is_some()
                        || node.component.semantic_text_layout().is_some()
                        || node.component.value_range().is_some(),
                )
            };
            if s.disabled
                || !visible
                || extra
                || !s.valid_selection(parent)
                || (tab_list
                    && (s.expanded.is_some()
                        || (part.is_some()
                            && (s.role != ContentRole::Tab
                                || parent != Some(ContentRole::TabList)))))
                || !matches!(
                    action,
                    AccessibilityAction::Focus | AccessibilityAction::Press
                )
                || (matches!(action, AccessibilityAction::Press)
                    && matches!(
                        s.role,
                        ContentRole::RadioGroup | ContentRole::SegmentedGroup
                    ))
            {
                return ContentResponse::default();
            }
        }
        if matches!(action, AccessibilityAction::ShowMenu) {
            return self.context_menu_accessibility_action(id, part);
        }
        if self.scope().find_active(id).is_some_and(|node| {
            node.component.semantics().role == ContentRole::Image
                || part.is_some_and(|part| {
                    node.component
                        .semantic_parts()
                        .iter()
                        .any(|p| p.index == part && p.semantics.role == ContentRole::Image)
                })
        }) {
            return ContentResponse::default();
        }
        if self.search_ax_blocked(id, part, &action) {
            return ContentResponse::default();
        }
        if let Some(response) = self.try_prepared_ax(id, part, &action) {
            return response;
        }
        self.reconcile_active_inputs();
        if self
            .scope()
            .find_active(id)
            .is_none_or(|n| n.component.semantics().disabled)
        {
            return ContentResponse::default();
        }
        // Preserve the original composing owner unless an auxiliary action
        // explicitly retains focus. Use the same marked/custom barrier as Tab.
        if self.is_composing()
            && self.focused != Some(id)
            && (matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
            ) || (part.is_none()
                && matches!(action, AccessibilityAction::Press)
                && self
                    .scope()
                    .find_active(id)
                    .is_some_and(|n| n.component.focusable() && !n.preserve_activation_focus)))
        {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Press) {
            self.help.suppress();
        }
        if part.is_none()
            && matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
            && (matches!(action, AccessibilityAction::Focus)
                || self
                    .scope()
                    .find_active(id)
                    .is_some_and(|n| !n.preserve_activation_focus))
            && self
                .scope()
                .find_active(id)
                .is_some_and(|n| n.component.focusable())
        {
            let changed = self.focus(Some(id), true);
            if matches!(action, AccessibilityAction::Focus) {
                return ContentResponse::consumed(changed);
            }
        }
        let owner = self
            .root
            .find_mut(id)
            .expect("validated active semantic owner");
        let mut result = owner.component.accessibility_action(part, &action);
        owner.record_actions(
            &result,
            ContentActionSource::Accessibility,
            true,
            &mut self.actions,
        );
        if result.consumed && matches!(action, AccessibilityAction::Press) {
            // Press may focus through the ordinary AX path above. It is still
            // an activation, not a fresh request for delayed keyboard help.
            self.help.suppress();
        }
        let delegated_focus = owner.component.accessibility_focus_requested();
        if result.consumed
            && (matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
            ) || delegated_focus)
        {
            // Original semantic part validation must precede keyboard ownership.
            result.changed |= self.focus(Some(id), true);
        }
        self.reconcile_active_inputs();
        self.dirty |= result.changed;
        result
    }
    pub fn show_dialog(&mut self, id: ComponentId) -> Result<(), CompositionError> {
        if self.modal.is_some() {
            return Err(CompositionError::ModalAlreadyOpen);
        }
        let dialog = self
            .root
            .find_mut(id)
            .and_then(|c| c.component_mut::<DialogContent>())
            .ok_or(CompositionError::NotDialog(id))?;
        dialog.model.show(0);
        if let Some(previous) = self.presented_dialog.filter(|old| *old != id)
            && let Some(dialog) = self
                .root
                .find_mut(previous)
                .and_then(|n| n.component_mut::<DialogContent>())
        {
            dialog.clear_presentation();
        }
        self.presented_dialog = Some(id);
        // Reordering preserves the mount; replacing or remounting the same ID
        // cannot inherit a previous object's modal return-focus intent.
        let restore = self.capture_focus();
        self.cancel();
        self.modal = Some((id, restore));
        self.reconcile_checkbox_scope_exposures();
        self.modal_focus_pending = true;
        Ok(())
    }
    /// Discard paint retained across dismissal before replacing atlas/backend resources.
    /// Semantic open/focus ownership is unchanged.
    pub fn clear_dialog_presentation(&mut self) {
        if let Some(id) = self.presented_dialog
            && let Some(dialog) = self
                .root
                .find_mut(id)
                .and_then(|n| n.component_mut::<DialogContent>())
        {
            dialog.clear_presentation();
        }
        self.dirty = true;
    }
    pub fn close_dialog(&mut self) -> bool {
        let Some((id, restore)) = self.modal else {
            return false;
        };
        self.cancel();
        if let Some(dialog) = self
            .root
            .find_mut(id)
            .and_then(|c| c.component_mut::<DialogContent>())
        {
            dialog.model.close();
        }
        if let Some(node) = self.root.find_mut(id) {
            empty_state::hide_subtree(node);
            notice::hide_subtree(node);
            node.reconcile_presentation_runnable(false);
        }
        self.modal = None;
        self.restore_focus(restore);
        true
    }
    pub fn cancel_pointer_capture(&mut self) -> bool {
        let Some(id) = self.captured.take() else {
            return false;
        };
        self.captured_mount = None;
        self.captured_overlay = false;
        self.captured_overlay_handler = false;
        if let Some(owner) = self.root.find_mut(id) {
            owner.component.cancel_pointer();
        }
        self.help.clear();
        self.dirty = true;
        true
    }
    pub fn cancel(&mut self) {
        self.list_navigation_reveal = None;
        self.retire_text_wheel();
        self.text_keys.clear();
        self.deferred_text_key = None;
        self.completed_text_key = None;
        for scope in &mut self.overlay_scopes {
            scope.restore = None;
            scope.focus_pending = false;
        }
        self.reading_scroll_source = None;
        self.help.clear();
        self.dirty = true;
        self.modal_focus_pending = false;
        fn cancel(c: &mut Content) {
            if let Some(popover) = c.component_mut::<PopoverContent>() {
                popover.close(PopoverCloseReason::Cancelled);
            }
            c.component.event(&ContentEvent::Cancel);
            if let Some(children) = c.component.children_mut() {
                for c in children {
                    cancel(c)
                }
            }
        }
        cancel(&mut self.root);
        self.captured = None;
        self.captured_mount = None;
        self.captured_overlay = false;
        self.captured_overlay_handler = false;
        self.focus(None, false);
    }
    /// Move the original mounted subtree, preserving its Content identities and
    /// help declarations. Unlike remove/insert this is not an unmount/remount.
    pub fn reparent(
        &mut self,
        id: ComponentId,
        parent: ComponentId,
    ) -> Result<(), CompositionError> {
        let node = self.root.find(id).ok_or(CompositionError::MissingId(id))?;
        if id == self.root.id || node.find(parent).is_some() {
            return Err(CompositionError::InvalidOrder);
        }
        if let Some(action) = self.root.action_ancestor(parent)
            && let Some(child) = node.interactive_id()
        {
            return Err(CompositionError::NestedInteractive { action, child });
        }
        if self.root.curve_ancestor(parent, false).unwrap_or(false) {
            node.validate_curve_tree(true)?;
        }
        let target = self
            .root
            .find(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if target.component.fixed_slots() || target.component.semantics().role.is_action_owner() {
            return Err(CompositionError::NotContainer(parent));
        }
        // Validate both mutable boundaries before touching the source.
        if self
            .root
            .find_mut(parent)
            .unwrap()
            .component
            .children_mut()
            .is_none()
        {
            return Err(CompositionError::NotContainer(parent));
        }
        fn mutable_parent(node: &mut Content, id: ComponentId) -> bool {
            let fixed =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
            let Some(children) = node.component.children_mut() else {
                return false;
            };
            if children.iter().any(|c| c.id == id) {
                return !fixed;
            }
            children.iter_mut().any(|c| mutable_parent(c, id))
        }
        if !mutable_parent(&mut self.root, id) {
            return Err(CompositionError::MissingId(id));
        }
        self.cancel_subtree(id);
        fn take(node: &mut Content, id: ComponentId) -> Option<Content> {
            let children = node.component.children_mut()?;
            if let Some(i) = children.iter().position(|c| c.id == id) {
                return Some(children.remove(i));
            }
            children.iter_mut().find_map(|c| take(c, id))
        }
        let node = take(&mut self.root, id).expect("validated source parent");
        self.root
            .find_mut(parent)
            .unwrap()
            .component
            .children_mut()
            .unwrap()
            .push(node);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
    pub fn insert(
        &mut self,
        parent: ComponentId,
        mut child: Content,
    ) -> Result<(), CompositionError> {
        if let Some(action) = self.root.action_ancestor(parent)
            && let Some(child) = child.interactive_id()
        {
            return Err(CompositionError::NestedInteractive { action, child });
        }
        let mut ids = BTreeSet::new();
        self.root.ids(&mut ids)?;
        child.ids(&mut ids)?;
        if self.root.curve_ancestor(parent, false).unwrap_or(false) {
            child.validate_curve_tree(true)?;
        }
        child.refresh_activity(self.window_active);
        let node = self
            .root
            .find_mut(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if node.component.fixed_slots() || node.component.semantics().role.is_action_owner() {
            return Err(CompositionError::NotContainer(parent));
        }
        let children = node
            .component
            .children_mut()
            .ok_or(CompositionError::NotContainer(parent))?;
        let mut child = child;
        child.lifecycle(Lifecycle::Mounted);
        children.push(child);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
    pub fn remove(&mut self, id: ComponentId) -> Result<Content, CompositionError> {
        // Check the actual mutable child boundary before dismissing a modal.
        // Root/fixed-slot/read-only containers cannot be removed; an error must
        // preserve the current scope, focus and retained presentation.
        fn can_remove(node: &mut Content, id: ComponentId) -> bool {
            let fixed =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
            let Some(children) = node.component.children_mut() else {
                return false;
            };
            if children.iter().any(|child| child.id == id) {
                return !fixed;
            }
            children.iter_mut().any(|child| can_remove(child, id))
        }
        if !can_remove(&mut self.root, id) {
            return Err(CompositionError::MissingId(id));
        }
        if self
            .modal
            .is_some_and(|(modal, _)| self.root.find(id).is_some_and(|n| n.find(modal).is_some()))
        {
            self.close_dialog();
        }
        fn remove(node: &mut Content, id: ComponentId) -> Option<Content> {
            let action =
                node.component.fixed_slots() || node.component.semantics().role.is_action_owner();
            let children = node.component.children_mut()?;
            if let Some(i) = children.iter().position(|c| c.id == id).filter(|_| !action) {
                return Some(children.remove(i));
            }
            children.iter_mut().find_map(|c| remove(c, id))
        }
        if self.text_wheel.as_ref().is_some_and(|lease| {
            self.root
                .find(id)
                .is_some_and(|node| node.find(lease.target).is_some())
        }) {
            self.retire_text_wheel();
        }
        let mut child = remove(&mut self.root, id).ok_or(CompositionError::MissingId(id))?;
        if self.focused.is_some_and(|id| child.find(id).is_some()) {
            self.focused = None;
        }
        if self.captured.is_some_and(|id| child.find(id).is_some()) {
            self.captured = None;
            self.captured_mount = None;
            self.captured_overlay = false;
            self.captured_overlay_handler = false;
        }
        child.lifecycle(Lifecycle::Unmounted);
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(child)
    }
    /// Validate the whole order before moving any child; keys retain owned state.
    pub fn reorder(
        &mut self,
        parent: ComponentId,
        order: &[ComponentId],
    ) -> Result<(), CompositionError> {
        let node = self
            .root
            .find_mut(parent)
            .ok_or(CompositionError::MissingId(parent))?;
        if node.component.fixed_slots() || node.component.semantics().role.is_action_owner() {
            return Err(CompositionError::NotContainer(parent));
        }
        let children = node
            .component
            .children_mut()
            .ok_or(CompositionError::NotContainer(parent))?;
        let actual: BTreeSet<_> = children.iter().map(|c| c.id).collect();
        if order.len() != children.len() || order.iter().copied().collect::<BTreeSet<_>>() != actual
        {
            return Err(CompositionError::InvalidOrder);
        }
        let mut old = std::mem::take(children);
        for id in order {
            let i = old.iter().position(|c| c.id == *id).unwrap();
            children.push(old.remove(i));
        }
        self.reveal_clock.invalidate_input();
        self.reveal_clock.pending = true;
        self.dirty = true;
        Ok(())
    }
    /// Current range metadata for an active semantic owner. Hidden/removed
    /// content cannot publish a stale adjustable value domain.
    pub fn value_range(&self, id: ComponentId) -> Option<std::ops::RangeInclusive<f64>> {
        self.scope()
            .find_active(id)
            .and_then(|n| {
                (!n.component.semantics().role.is_selection())
                    .then(|| n.component.value_range())
                    .flatten()
            })
            .filter(|range| {
                range.start().is_finite() && range.end().is_finite() && range.start() <= range.end()
            })
    }
    /// Query a published virtual part's original finite domain. The existing
    /// local-owner branch retains value_range's window-activity behavior.
    pub fn semantic_value_range(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<RangeInclusive<f64>> {
        let Some(part) = part else {
            return self.value_range(id);
        };
        if !self.window_active {
            return None;
        }
        let owner = self.scope().find_active(id)?;
        let range = owner.component.semantic_part_value_range(part)?;
        if !(range.start().is_finite() && range.end().is_finite() && range.start() <= range.end()) {
            return None;
        }
        owner
            .component
            .semantic_parts()
            .iter()
            .any(|p| p.index == part && !p.semantics.role.is_selection())
            .then_some(range)
    }
    /// Single-pass read-only semantic lookup table built from `scope()` so AX
    /// publication queries run in O(1) per node/part without re-walking the tree.
    pub fn semantic_scope(&self) -> SemanticScope<'_> {
        let scope = self.scope();
        let mut by_id = std::collections::HashMap::new();
        fn collect_active<'a>(
            c: &'a Content,
            out: &mut std::collections::HashMap<ComponentId, &'a Content>,
        ) {
            if !c.active() {
                return;
            }
            out.entry(c.id).or_insert(c);
            for child in c.component.children() {
                collect_active(child, out);
            }
        }
        collect_active(scope, &mut by_id);
        let mut split_hidden = std::collections::HashSet::new();
        fn collect_split_hidden(
            c: &Content,
            hidden: bool,
            out: &mut std::collections::HashSet<ComponentId>,
        ) {
            let pane = c.component::<super::split_view::PaneSlot<true>>().is_some()
                || c.component::<super::split_view::PaneSlot<false>>()
                    .is_some();
            let hidden = hidden || (pane && c.visible().is_none());
            if hidden {
                out.insert(c.id);
            }
            for child in c.component.children() {
                collect_split_hidden(child, hidden, out);
            }
        }
        collect_split_hidden(&self.root, false, &mut split_hidden);
        SemanticScope {
            host: self,
            scope,
            by_id,
            split_hidden,
        }
    }
    /// Actual mounted tree projection; this is not a platform accessibility bridge.
    pub fn semantics(&self) -> Vec<SemanticNode> {
        let mut nodes = Vec::new();
        self.scope().semantic_nodes(None, self.focused, &mut nodes);
        nodes
    }
}
/// Single-pass read-only semantic lookup table built from `ContentHost::scope()`.
#[derive(Debug)]
pub struct SemanticScope<'a> {
    host: &'a ContentHost,
    scope: &'a Content,
    by_id: std::collections::HashMap<ComponentId, &'a Content>,
    split_hidden: std::collections::HashSet<ComponentId>,
}
impl<'a> SemanticScope<'a> {
    pub fn semantics(&self) -> Vec<SemanticNode> {
        let mut nodes = Vec::new();
        self.scope
            .semantic_nodes(None, self.host.focused, &mut nodes);
        nodes
    }
    pub fn semantic_mount(&self, id: ComponentId) -> Option<u64> {
        if !self.host.window_active {
            return None;
        }
        self.by_id.get(&id).map(|node| node.mount)
    }
    pub fn text_surface(&self, id: ComponentId) -> Option<&crate::TextSurface> {
        self.by_id.get(&id).and_then(|c| {
            (!c.component.semantics().role.is_selection())
                .then(|| c.component.text_surface())
                .flatten()
        })
    }
    pub fn semantic_text_input_snapshot(&self, id: ComponentId) -> Option<ContentTextInput> {
        if !self.host.window_active {
            return None;
        }
        let owner = self.by_id.get(&id)?;
        if owner.component.semantics().disabled || owner.component.semantics().role.is_selection() {
            return None;
        }
        let mut input = owner.component.text_input_snapshot()?;
        if self.split_hidden.contains(&id) {
            input.caret = None;
            input.clip = None;
            input.carets.clear();
        }
        Some(input)
    }
    pub fn semantic_text_layout_snapshot(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<ContentTextLayout> {
        if !self.host.window_active || self.split_hidden.contains(&id) {
            return None;
        }
        let owner = self.by_id.get(&id)?;
        if let Some(part) = part {
            let layout = owner.component.semantic_part_text_layout(part)?;
            if !owner
                .component
                .semantic_parts()
                .iter()
                .any(|p| p.index == part && !p.semantics.role.is_selection())
            {
                return None;
            }
            Some(layout)
        } else {
            if owner.component.semantics().role.is_selection() {
                return None;
            }
            owner.component.semantic_text_layout()
        }
    }
    pub fn value_range(&self, id: ComponentId) -> Option<RangeInclusive<f64>> {
        self.by_id
            .get(&id)
            .and_then(|n| {
                (!n.component.semantics().role.is_selection())
                    .then(|| n.component.value_range())
                    .flatten()
            })
            .filter(|range| {
                range.start().is_finite() && range.end().is_finite() && range.start() <= range.end()
            })
    }
    pub fn semantic_value_range(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> Option<RangeInclusive<f64>> {
        let Some(part) = part else {
            return self.value_range(id);
        };
        if !self.host.window_active {
            return None;
        }
        let owner = self.by_id.get(&id)?;
        let range = owner.component.semantic_part_value_range(part)?;
        if !(range.start().is_finite() && range.end().is_finite() && range.start() <= range.end()) {
            return None;
        }
        owner
            .component
            .semantic_parts()
            .iter()
            .any(|p| p.index == part && !p.semantics.role.is_selection())
            .then_some(range)
    }
    pub fn semantic_part_parent(&self, id: ComponentId, part: usize) -> Option<usize> {
        self.by_id.get(&id)?.component.semantic_part_parent(part)
    }
    pub fn can_show_context_menu(&self, id: ComponentId) -> bool {
        self.host.can_show_context_menu(id)
    }
    pub fn semantic_preserves_activation_focus(
        &self,
        id: ComponentId,
        part: Option<usize>,
    ) -> bool {
        self.by_id.get(&id).is_some_and(|node| {
            node.preserve_activation_focus
                || node.component.preserves_activation_focus()
                || part.is_some_and(|part| {
                    node.component
                        .semantic_part_preserves_activation_focus(part)
                })
        })
    }
}
impl Drop for ContentHost {
    fn drop(&mut self) {
        self.reveal_clock.invalidate_geometry();
        self.root.lifecycle(Lifecycle::Unmounted)
    }
}
