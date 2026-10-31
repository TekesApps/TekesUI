//! Original-owner preparation for atomic action plus focus/reveal admission.
use super::*;
use content_reveal_plan::ContentRoute;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTransactionCompatibility {
    Unsupported,
    StableHooks,
}
#[derive(Clone, Copy, Debug)]
pub enum ComponentActionRequest<'a> {
    PointerDown(&'a ContentEvent),
    Accessibility {
        part: Option<usize>,
        action: &'a AccessibilityAction,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedActionFocus {
    None,
    Owner,
    Preserve,
}
/// Pure final validation authenticates this exact original reducer decision.
/// apply consumes that decision; it cannot refuse, reroute, or replay a handler.
/// Stable hooks and this contract exclude tree mutation/reentry and late refusal.
/// A decision uses only its owner-local state and remains applicable across the
/// declared focus hooks and preceding child effects in the prepared bubble chain.
/// Shared mutable action state requiring a child callback is Unsupported.
pub trait PreparedActionEffect: Debug {
    /// Optional post-decision caret geometry, authenticated by final validation.
    fn focus_rect(&self) -> Option<Rect> {
        None
    }
    fn validate(&self, owner: &dyn Component) -> bool;
    fn apply(self: Box<Self>, owner: &mut dyn Component);
}
#[derive(Debug)]
pub struct PreparedComponentAction {
    response: ContentResponse,
    focus: PreparedActionFocus,
    effect: Box<dyn PreparedActionEffect>,
    refused: bool,
}
impl PreparedComponentAction {
    pub fn new(
        response: ContentResponse,
        focus: PreparedActionFocus,
        effect: Box<dyn PreparedActionEffect>,
    ) -> Self {
        Self {
            response,
            focus,
            effect,
            refused: false,
        }
    }
}
#[derive(Debug)]
pub enum ActionPreparation {
    Unsupported,
    Ready(PreparedComponentAction),
}
/// A recognized owner request that must be refused before running the original
/// reducer. This differs from Unsupported, which deliberately falls back to the
/// owner's legacy event path.
pub(super) fn rejected() -> ActionPreparation {
    let mut prepared = PreparedComponentAction::new(
        ContentResponse::default(),
        PreparedActionFocus::None,
        Box::new(NoopEffect),
    );
    prepared.refused = true;
    ActionPreparation::Ready(prepared)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusOrigin {
    Direct,
    Pointer,
    Traversal,
    Accessibility,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusAdmission {
    Rejected,
    Accepted { changed: bool },
}
#[derive(Debug, PartialEq)]
struct EditorState {
    binding: Option<(u64, u64)>,
    composing: bool,
    text: Option<crate::text_input::TextActionInputSnapshot>,
}
impl EditorState {
    fn read(host: &ContentHost) -> Self {
        Self {
            binding: host
                .focus_admission_text_surface()
                .map(|s| s.input.controlled_binding()),
            composing: host.is_composing(),
            text: host
                .focus_admission_text_surface()
                .map(|s| s.input.action_snapshot()),
        }
    }
}
struct PreparedFocus {
    id: Option<ComponentId>,
    visible: bool,
    traversal: Option<bool>,
    origin: FocusOrigin,
    focus_scope: Option<(ComponentId, u64)>,
    dismiss_popovers: Vec<PopoverSession>,
    focused: Option<ComponentId>,
    epoch: u64,
    next_epoch: u64,
    stamp: (u64, u64),
    editor: EditorState,
    plan: Option<HostRevealPlan>,
    old: Option<ContentRoute>,
    new: Option<ContentRoute>,
}
impl ContentHost {
    // Focus transfer must authenticate the original mounted editor even when a
    // newly opened scope excludes it from public native/AX text queries.
    fn focus_admission_text_surface(&self) -> Option<&crate::TextSurface> {
        self.focused
            .and_then(|id| self.root.find(id))
            .and_then(|node| node.component.text_surface())
    }
    fn delegated_composition_needs_owner(&self) -> bool {
        self.requires_curve_geometry()
            && self.focus_admission_text_surface().is_none()
            && self
                .focused
                .and_then(|id| self.root.find_active(id))
                .is_some_and(|node| node.component.delegates_text_keys())
    }
    fn prepare_focus(
        &self,
        id: Option<ComponentId>,
        visible: bool,
        traversal: Option<bool>,
        origin: FocusOrigin,
    ) -> Result<PreparedFocus, HostRevealError> {
        self.prepare_focus_rect(id, visible, traversal, origin, None)
    }
    fn requested_focus_scope(
        &self,
        id: ComponentId,
        origin: FocusOrigin,
    ) -> Option<(&Content, Vec<PopoverSession>)> {
        if origin == FocusOrigin::Direct {
            self.direct_focus_plan(id)
        } else {
            let scope = self.scope();
            scope.find_active(id).map(|_| (scope, Vec::new()))
        }
    }
    fn prepare_focus_rect(
        &self,
        id: Option<ComponentId>,
        visible: bool,
        traversal: Option<bool>,
        origin: FocusOrigin,
        rect: Option<Rect>,
    ) -> Result<PreparedFocus, HostRevealError> {
        if !self.window_active && id.is_some() {
            return Err(HostRevealError::UnavailableTarget);
        }
        let focus_scope = if let Some(id) = id {
            let (scope, dismiss_popovers) = self
                .requested_focus_scope(id, origin)
                .ok_or(HostRevealError::UnavailableTarget)?;
            if scope
                .find_active(id)
                .is_none_or(|n| !n.component.focusable() || n.component.semantics().disabled)
            {
                return Err(HostRevealError::UnavailableTarget);
            }
            Some((scope.id, scope.mount, dismiss_popovers))
        } else {
            None
        };
        if id.is_some()
            && id != self.focused
            && self.is_composing()
            && (matches!(origin, FocusOrigin::Traversal | FocusOrigin::Accessibility)
                || self.delegated_composition_needs_owner())
        {
            return Err(HostRevealError::CompositionBarrier);
        }
        if id != self.focused
            && self
                .focus_admission_text_surface()
                .is_some_and(|s| !s.input.can_unmark())
        {
            return Err(HostRevealError::IdentityExhausted);
        }
        let next_epoch = if id != self.focused {
            self.focus_epoch
                .checked_add(1)
                .ok_or(HostRevealError::IdentityExhausted)?
        } else {
            self.focus_epoch
        };
        let plan = match (id, focus_scope.as_ref()) {
            (Some(id), Some((scope_id, scope_mount, _))) => {
                Some(self.plan_focus_reveal_in_focus_scope(id, *scope_id, *scope_mount, rect)?)
            }
            (None, None) => None,
            _ => return Err(HostRevealError::UnavailableTarget),
        };
        let new = plan.as_ref().map(HostRevealPlan::target_route);
        let old = if self.focused != id {
            self.focused
                .map(|old| {
                    ContentRoute::capture(
                        &self.root,
                        old,
                        HostRevealBudget::default().max_path_nodes,
                    )
                })
                .transpose()
                .or_else(|e| {
                    if matches!(e, HostRevealError::UnavailableTarget) {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })?
        } else {
            None
        };
        Ok(PreparedFocus {
            id,
            visible,
            traversal,
            origin,
            focus_scope: focus_scope.as_ref().map(|(id, mount, _)| (*id, *mount)),
            dismiss_popovers: focus_scope
                .map(|(_, _, dismiss)| dismiss)
                .unwrap_or_default(),
            focused: self.focused,
            epoch: self.focus_epoch,
            next_epoch,
            stamp: self.reveal_clock.stamp()?,
            editor: EditorState::read(self),
            plan,
            old,
            new,
        })
    }
    fn validate_focus(&self, p: &PreparedFocus) -> bool {
        let focus_scope = p.id.and_then(|id| self.requested_focus_scope(id, p.origin));
        let scope_matches = match (p.id, focus_scope.as_ref()) {
            (Some(_), Some((scope, dismiss))) => {
                Some((scope.id, scope.mount)) == p.focus_scope && dismiss == &p.dismiss_popovers
            }
            (None, None) => p.focus_scope.is_none() && p.dismiss_popovers.is_empty(),
            _ => false,
        };
        self.focused == p.focused
            && self.focus_epoch == p.epoch
            && self.reveal_clock.stamp().ok() == Some(p.stamp)
            && EditorState::read(self) == p.editor
            && (p.id.is_none() || self.window_active)
            && scope_matches
            && p.id.is_none_or(|id| {
                focus_scope.as_ref().is_some_and(|(scope, _)| {
                    scope.find_active(id).is_some_and(|n| {
                        n.component.focusable() && !n.component.semantics().disabled
                    })
                })
            })
            && (p.id == self.focused
                || self
                    .focus_admission_text_surface()
                    .is_none_or(|s| s.input.can_unmark()))
            && !(p.id.is_some()
                && p.id != self.focused
                && self.is_composing()
                && (matches!(
                    p.origin,
                    FocusOrigin::Traversal | FocusOrigin::Accessibility
                ) || self.delegated_composition_needs_owner()))
            && p.old
                .iter()
                .chain(p.new.iter())
                .all(|r| r.node(&self.root, r.depth()).is_some())
            && p.plan.as_ref().is_none_or(|plan| {
                p.focus_scope.is_some_and(|(scope, mount)| {
                    self.validate_host_reveal_in_focus_scope(plan, scope, mount)
                        .is_ok()
                })
            })
    }
    fn apply_focus(&mut self, p: PreparedFocus) -> FocusAdmission {
        if !self.validate_focus(&p) {
            return FocusAdmission::Rejected;
        }
        if let Some(plan) = p.plan {
            let Some((scope, mount)) = p.focus_scope else {
                return FocusAdmission::Rejected;
            };
            if self
                .commit_host_reveal_in_focus_scope(plan, scope, mount)
                .is_err()
            {
                return FocusAdmission::Rejected;
            }
        }
        // All target, editor, scope, session, focus-epoch and reveal checks are
        // complete before the first mutation. These sessions cannot change
        // without re-entry, which the prepared path excludes.
        for session in &p.dismiss_popovers {
            self.root
                .find_mut(session.owner())
                .and_then(|node| node.component_mut::<PopoverContent>())
                .filter(|popup| popup.current_session() == Some(*session) && popup.is_open())
                .expect("validated Popover session before prepared focus commit")
                .close(PopoverCloseReason::Dismiss);
        }
        let changed = self.focused != p.id;
        self.focus_epoch = p.next_epoch;
        if let Some(route) = p.old {
            route
                .node_mut(&mut self.root, route.depth())
                .expect("validated old owner")
                .component
                .set_focus(false, false);
        }
        self.focused = p.id;
        if let Some(route) = p.new {
            let c = route
                .node_mut(&mut self.root, route.depth())
                .expect("validated new owner");
            if let Some(reverse) = p.traversal {
                c.component.set_focus_from_traversal(reverse);
            } else {
                c.component.set_focus(true, p.visible);
            }
        }
        self.dirty = true;
        self.help_focus(p.id, p.visible);
        if !p.dismiss_popovers.is_empty() {
            // A newer Direct focus is already installed, so overlay retirement
            // cannot restore the old caller over the accepted target.
            self.reconcile_overlay_scopes();
        }
        FocusAdmission::Accepted { changed }
    }
    /// Used only at an ordinary Host entry after UnsupportedLayout or absence
    /// of a prepared-hook contract. It grants no prepared transaction capability.
    pub(super) fn original_reveal_admissible(&self, id: ComponentId) -> bool {
        let Ok(route) =
            ContentRoute::capture(&self.root, id, HostRevealBudget::default().max_path_nodes)
        else {
            return false;
        };
        if (0..=route.depth()).any(|depth| {
            route.node(&self.root, depth).is_none_or(|node| {
                node.component.requires_focus_reveal()
                    || node.component::<ScrollContent>().is_some()
            })
        }) {
            return false;
        }
        self.reveal_clock
            .stamp()
            .is_ok_and(|(_, input)| input.checked_add(1).is_some())
    }
    /// Ordinary focus keeps the pure prepared path whenever it is available.
    /// Legacy custom layout remains supported by the original dynamic hooks,
    /// without representing those hooks as a read-only/atomic reveal proof.
    pub(super) fn admit_ordinary_focus(
        &mut self,
        id: Option<ComponentId>,
        visible: bool,
        traversal: Option<bool>,
        origin: FocusOrigin,
    ) -> FocusAdmission {
        match self.prepare_focus(id, visible, traversal, origin) {
            Ok(prepared) => return self.apply_focus(prepared),
            Err(HostRevealError::UnsupportedLayout)
                if id.is_some_and(|id| self.original_reveal_admissible(id)) => {}
            Err(_) => return FocusAdmission::Rejected,
        }
        // prepare_focus completed window/availability/unmark/focus-counter
        // admission before encountering unsupported layout. No callback ran.
        let changed = self.focused != id;
        self.reveal_clock.invalidate_input();
        if changed {
            self.focus_epoch += 1;
        }
        if changed && let Some(owner) = self.focused.and_then(|old| self.root.find_mut(old)) {
            owner.component.set_focus(false, false);
        }
        self.focused = id;
        if let Some(id) = id {
            let frame = {
                let owner = self
                    .root
                    .find_mut(id)
                    .expect("validated ordinary focus owner");
                if let Some(reverse) = traversal {
                    owner.component.set_focus_from_traversal(reverse);
                } else {
                    owner.component.set_focus(true, visible);
                }
                owner
                    .component
                    .text_surface()
                    .filter(|surface| {
                        surface.presentation == crate::TextSurfacePresentation::Content
                    })
                    .map_or(owner.frame, |surface| surface.caret())
            };
            self.root.reveal(id, frame);
        }
        self.dirty = true;
        self.help_focus(id, visible);
        FocusAdmission::Accepted { changed }
    }
    /// Ordinary geometry opt-ins do not promise prepared-effect purity. The
    /// rectangular opt-in is unavailable whenever this Host needs curve geometry.
    /// Focus transfer still needs a successful current reveal plan; unsupported
    /// layout never counts as success.
    fn local_ordinary_action_admissible(
        &self,
        id: ComponentId,
        origin: FocusOrigin,
        wants_focus: bool,
    ) -> bool {
        let Some(node) = self.scope().find_active(id) else {
            return false;
        };
        if !self.window_active
            || node.component.semantics().disabled
            || !(node.component.curve_subtree_capability() == CurveSubtreeCapability::Local
                || node.component.delegates_curve_focus()
                    && node.component.rectangular_input_geometry()
                || !self.requires_curve_geometry() && node.component.rectangular_input_geometry())
            || !self
                .reveal_clock
                .stamp()
                .is_ok_and(|(_, input)| input.checked_add(1).is_some())
            || (self.requires_curve_geometry()
                && self
                    .geometry_snapshot()
                    .is_none_or(|snapshot| !snapshot.is_current()))
        {
            return false;
        }
        !wants_focus
            || self
                .prepare_focus(
                    Some(id),
                    matches!(origin, FocusOrigin::Accessibility),
                    None,
                    origin,
                )
                .is_ok()
    }
    fn ordinary_pointer_action_admissible(&self, id: ComponentId) -> bool {
        if self.original_reveal_admissible(id) {
            return true;
        }
        let wants_focus = self.scope().find_active(id).is_some_and(|node| {
            node.component.focusable()
                && !node.preserve_activation_focus
                && !node.component.preserves_activation_focus()
        });
        self.local_ordinary_action_admissible(id, FocusOrigin::Pointer, wants_focus)
    }
    fn ordinary_ax_focus_admissible(
        &self,
        id: ComponentId,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> bool {
        // Ordinary part Press dispatches to its existing owner without moving
        // keyboard focus; mirror the actual fallback rather than inventing a transfer.
        let candidate = (matches!(
            action,
            AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
        ) || (part.is_none()
            && matches!(action, AccessibilityAction::Press)
            && !self.semantic_preserves_activation_focus(id, part)))
            && self
                .scope()
                .find_active(id)
                .is_some_and(|node| node.component.focusable());
        if !self.original_reveal_admissible(id) {
            return self.local_ordinary_action_admissible(
                id,
                FocusOrigin::Accessibility,
                candidate,
            );
        }
        if !candidate {
            return true;
        }
        match self.prepare_focus(Some(id), true, None, FocusOrigin::Accessibility) {
            Ok(_) => self
                .reveal_clock
                .stamp()
                .is_ok_and(|(_, input)| input.checked_add(1).is_some()),
            Err(HostRevealError::UnsupportedLayout) => self.original_reveal_admissible(id),
            Err(_) => false,
        }
    }
    fn strict_focus_requirement(&self, id: ComponentId) -> bool {
        fn walk(n: &Content, id: ComponentId, required: bool) -> Option<bool> {
            let required = required
                || n.component.requires_focus_reveal()
                || n.component::<ScrollContent>().is_some();
            if n.id == id {
                return Some(required);
            }
            n.component
                .children()
                .iter()
                .find_map(|n| walk(n, id, required))
        }
        walk(self.scope(), id, false).unwrap_or(true)
    }
}

#[derive(Debug)]
struct NoopEffect;
impl PreparedActionEffect for NoopEffect {
    fn validate(&self, owner: &dyn Component) -> bool {
        owner.action_transaction_compatibility() == ActionTransactionCompatibility::StableHooks
    }
    fn apply(self: Box<Self>, _: &mut dyn Component) {}
}
pub(super) fn ignored_pointer(request: ComponentActionRequest<'_>) -> ActionPreparation {
    if matches!(request, ComponentActionRequest::PointerDown(_)) {
        ActionPreparation::Ready(PreparedComponentAction::new(
            ContentResponse::default(),
            PreparedActionFocus::None,
            Box::new(NoopEffect),
        ))
    } else {
        ActionPreparation::Unsupported
    }
}
struct OwnerAction {
    route: ContentRoute,
    action: PreparedComponentAction,
}
impl ContentHost {
    // None means an existing owner has no prepared-hook contract, so pointer
    // dispatch must keep its ordinary reducer path. Some(false) is a stale or
    // refused prepared decision and must never fall through to ordinary input.
    fn validate_action_owners(
        &self,
        actions: &[OwnerAction],
        focus: Option<&PreparedFocus>,
    ) -> Option<bool> {
        let mut unsupported = false;
        for action in actions {
            let Some(owner) = action.route.node(&self.root, action.route.depth()) else {
                return Some(false);
            };
            if !action.action.effect.validate(&*owner.component) {
                return Some(false);
            }
            for depth in 0..=action.route.depth() {
                let Some(owner) = action.route.node(&self.root, depth) else {
                    return Some(false);
                };
                unsupported |= owner.component.action_transaction_compatibility()
                    != ActionTransactionCompatibility::StableHooks;
            }
        }
        if let Some(focus) = focus {
            for route in focus.old.iter().chain(focus.new.iter()) {
                for depth in 0..=route.depth() {
                    let Some(owner) = route.node(&self.root, depth) else {
                        return Some(false);
                    };
                    unsupported |= owner.component.action_transaction_compatibility()
                        != ActionTransactionCompatibility::StableHooks;
                }
            }
            if !self.validate_focus(focus) {
                return Some(false);
            }
        }
        if unsupported { None } else { Some(true) }
    }
    pub(super) fn try_prepared_pointer(
        &mut self,
        event: &ContentEvent,
        point: Point,
    ) -> Option<ContentResponse> {
        fn hit<'a>(node: &'a Content, point: Point, path: &mut Vec<&'a Content>) -> bool {
            if !node.active() {
                return false;
            }
            if node.routes_pointer_children() {
                for child in node.component.children().iter().rev() {
                    if hit(child, point, path) {
                        path.push(node);
                        return true;
                    }
                }
            }
            if node.pointer_target_at(point) {
                path.push(node);
                return true;
            }
            false
        }
        let mut path = Vec::new();
        if !hit(self.scope(), point, &mut path) {
            return None;
        }
        let original_target = path.first()?.id;
        // A currently focused text editor already owns the keyboard/IME route.
        // Its pointer-down may extend the original selection, so leave that
        // same-owner reducer untouched; cross-owner transfers use preparation.
        if self.focused.is_some_and(|focused| {
            path.iter().any(|node| {
                node.id == focused
                    && node.component.text_surface().is_some()
                    && node.component::<ComboBoxContent>().is_none()
            })
        }) {
            return None;
        }
        let mut actions = Vec::new();
        let mut result = None;
        let mut focus_intent = PreparedActionFocus::None;
        let mut focus_rect = None;
        for node in path {
            if node.component.action_transaction_compatibility()
                != ActionTransactionCompatibility::StableHooks
            {
                // This original owner has no transaction contract. Keep its
                // existing reducer/capture behavior; do not reject a supported
                // pointer action merely because a focus plan cannot be staged.
                return if !self.ordinary_pointer_action_admissible(original_target)
                    || self.validate_action_owners(&actions, None) == Some(false)
                {
                    Some(ContentResponse::default())
                } else {
                    None
                };
            }
            let ActionPreparation::Ready(action) = node
                .component
                .prepare_action(ComponentActionRequest::PointerDown(event))
            else {
                // This original owner has no transaction contract. Keep its
                // existing reducer/capture behavior; do not reject a supported
                // pointer action merely because a focus plan cannot be staged.
                return if !self.ordinary_pointer_action_admissible(original_target)
                    || self.validate_action_owners(&actions, None) == Some(false)
                {
                    Some(ContentResponse::default())
                } else {
                    None
                };
            };
            if action.refused {
                self.event_bubbled = Some(false);
                return Some(ContentResponse::default());
            }
            let previous_owner = result.as_ref().map(|(id, _, _)| *id);
            let next = node.finish_bubble(event, result, action.response);
            if previous_owner != Some(next.0) {
                focus_intent = action.focus;
                focus_rect = action.effect.focus_rect();
            }
            let bubbles = next.2;
            let Ok(route) = ContentRoute::capture(
                &self.root,
                node.id,
                HostRevealBudget::default().max_path_nodes,
            ) else {
                return Some(ContentResponse::default());
            };
            actions.push(OwnerAction { route, action });
            result = Some(next);
            if !bubbles {
                break;
            }
        }
        let (id, response, bubbles) = result?;
        let wants_focus = response.consumed
            && focus_intent == PreparedActionFocus::Owner
            && self.root.find(id).is_some_and(|n| {
                n.component.focusable()
                    && !n.preserve_activation_focus
                    && !n.component.preserves_activation_focus()
            });
        let focus = if wants_focus {
            match self.prepare_focus_rect(Some(id), false, None, FocusOrigin::Pointer, focus_rect) {
                Ok(p) => Some(p),
                Err(_) => return Some(ContentResponse::default()),
            }
        } else {
            None
        };
        let next_capture = if response.capture_pointer {
            match self.capture_epoch.checked_add(1) {
                Some(e) => Some(e),
                None => return Some(ContentResponse::default()),
            }
        } else {
            None
        };
        let stamp = self.reveal_clock.stamp().ok();
        let capture = self.capture_epoch;
        if stamp.is_none() {
            return Some(ContentResponse::default());
        }
        let owners_current = self
            .validate_action_owners(&actions, focus.as_ref())
            .or_else(|| {
                if focus.is_some() || self.ordinary_pointer_action_admissible(original_target) {
                    None
                } else {
                    Some(false)
                }
            })?;
        if !owners_current
            || self.capture_epoch != capture
            || self.reveal_clock.stamp().ok() != stamp
        {
            return Some(ContentResponse::default());
        }
        if let Some(p) = focus {
            if matches!(self.apply_focus(p), FocusAdmission::Rejected) {
                return Some(ContentResponse::default());
            }
            self.pointer_focus_requested = true;
        }
        for a in actions {
            let n = a
                .route
                .node_mut(&mut self.root, a.route.depth())
                .expect("validated prepared bubble route");
            let r = a.action.response;
            a.action.effect.apply(&mut *n.component);
            n.record_actions(&r, event.action_source(), true, &mut self.actions);
        }
        if let Some(epoch) = next_capture {
            self.capture_epoch = epoch;
            self.captured = Some(id);
            self.captured_mount = self.root.find_active(id).map(|node| node.mount);
            self.captured_overlay_handler = false;
        }
        self.event_bubbled = Some(bubbles);
        Some(response)
    }
    pub(super) fn try_prepared_ax(
        &mut self,
        id: ComponentId,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> Option<ContentResponse> {
        // Existing-owner range/value operations keep their original authority;
        // they neither establish a new owner nor acquire a visibility gate.
        if self.focused == Some(id)
            && !matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
        {
            return None;
        }
        let combo_expansion = matches!(action, AccessibilityAction::SetExpanded(_))
            && self
                .scope()
                .find_active(id)
                .is_some_and(|node| node.component::<ComboBoxContent>().is_some());
        if !self.strict_focus_requirement(id) {
            // Other ordinary AX verbs keep their original non-transfer policy.
            // Strict cross-owner verbs below always need an original decision.
            if !matches!(
                action,
                AccessibilityAction::Focus
                    | AccessibilityAction::Press
                    | AccessibilityAction::SetSelection(_)
            ) && !combo_expansion
            {
                return None;
            }
            // Stable original Press owners use the same preflight/commit path
            // even on ordinary rectangles; this prevents an overflow or failed
            // focus admission from falling through to a partially mutating AX reducer.
            let stable_press = (matches!(action, AccessibilityAction::Press) || combo_expansion)
                && self.scope().find_active(id).is_some_and(|n| {
                    n.component.action_transaction_compatibility()
                        == ActionTransactionCompatibility::StableHooks
                });
            if !stable_press {
                return if self.ordinary_ax_focus_admissible(id, part, action) {
                    None
                } else {
                    Some(ContentResponse::default())
                };
            }
        }
        if part.is_none() && matches!(action, AccessibilityAction::Focus) {
            return Some(
                match self.admit_ordinary_focus(Some(id), true, None, FocusOrigin::Accessibility) {
                    FocusAdmission::Accepted { changed } => ContentResponse::consumed(changed),
                    FocusAdmission::Rejected => ContentResponse::default(),
                },
            );
        }
        let n = self.scope().find_active(id)?;
        if n.component.action_transaction_compatibility()
            != ActionTransactionCompatibility::StableHooks
        {
            return if self.ordinary_ax_focus_admissible(id, part, action) {
                None
            } else {
                Some(ContentResponse::default())
            };
        }
        let ActionPreparation::Ready(prepared) = n
            .component
            .prepare_action(ComponentActionRequest::Accessibility { part, action })
        else {
            return if self.ordinary_ax_focus_admissible(id, part, action) {
                None
            } else {
                Some(ContentResponse::default())
            };
        };
        if prepared.refused {
            return Some(ContentResponse::default());
        }
        let wants_focus = prepared.response.consumed
            && (matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::SetSelection(_)
            ) || (prepared.focus == PreparedActionFocus::Owner
                && !self.semantic_preserves_activation_focus(id, part)));
        let focus = if wants_focus {
            match self.prepare_focus_rect(
                Some(id),
                true,
                None,
                FocusOrigin::Accessibility,
                prepared.effect.focus_rect(),
            ) {
                Ok(p) => Some(p),
                Err(HostRevealError::UnsupportedLayout)
                    if self.original_reveal_admissible(id)
                        && prepared.effect.validate(&*n.component) =>
                {
                    return None;
                }
                Err(_) => return Some(ContentResponse::default()),
            }
        } else {
            None
        };
        let Ok(route) =
            ContentRoute::capture(&self.root, id, HostRevealBudget::default().max_path_nodes)
        else {
            return Some(ContentResponse::default());
        };
        let actions = [OwnerAction {
            route,
            action: prepared,
        }];
        match self.validate_action_owners(&actions, focus.as_ref()) {
            Some(true) => {}
            None if focus.is_some() || self.ordinary_ax_focus_admissible(id, part, action) => {
                return None;
            }
            _ => return Some(ContentResponse::default()),
        }
        let changed = if let Some(p) = focus {
            match self.apply_focus(p) {
                FocusAdmission::Rejected => return Some(ContentResponse::default()),
                FocusAdmission::Accepted { changed } => changed,
            }
        } else {
            false
        };
        let [a] = actions;
        let mut result = a.action.response;
        let n = a
            .route
            .node_mut(&mut self.root, a.route.depth())
            .expect("validated prepared AX route");
        a.action.effect.apply(&mut *n.component);
        n.record_actions(
            &result,
            ContentActionSource::Accessibility,
            true,
            &mut self.actions,
        );
        result.changed |= changed;
        self.dirty |= result.changed;
        if result.consumed && matches!(action, AccessibilityAction::Press) {
            self.help.suppress();
        }
        Some(result)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OwnedActionRequest {
    Pointer(ContentEvent),
    Accessibility(Option<usize>, AccessibilityAction),
}
impl OwnedActionRequest {
    pub(super) fn read(request: ComponentActionRequest<'_>) -> Self {
        match request {
            ComponentActionRequest::PointerDown(e) => Self::Pointer(e.clone()),
            ComponentActionRequest::Accessibility { part, action } => {
                Self::Accessibility(part, action.clone())
            }
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct OwnerIdentity(pub std::sync::Arc<()>);
impl PartialEq for OwnerIdentity {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
pub(super) trait OriginalActionDecision: Debug + PartialEq {
    fn focus_rect(&self) -> Option<Rect> {
        None
    }
}
#[derive(Debug)]
struct OriginalEffect<T: Component, D: OriginalActionDecision> {
    request: OwnedActionRequest,
    decision: D,
    decide: fn(&T, &OwnedActionRequest) -> Option<D>,
    apply: fn(&mut T, D),
}
impl<T: Component, D: OriginalActionDecision + 'static> PreparedActionEffect
    for OriginalEffect<T, D>
{
    fn focus_rect(&self) -> Option<Rect> {
        self.decision.focus_rect()
    }
    fn validate(&self, owner: &dyn Component) -> bool {
        (owner as &dyn Any)
            .downcast_ref::<T>()
            .is_some_and(|owner| {
                (self.decide)(owner, &self.request).as_ref() == Some(&self.decision)
            })
    }
    fn apply(self: Box<Self>, owner: &mut dyn Component) {
        let owner = (owner as &mut dyn Any)
            .downcast_mut::<T>()
            .expect("validated original prepared owner");
        (self.apply)(owner, self.decision);
    }
}
pub(super) fn prepare_original<T: Component, D: OriginalActionDecision + 'static>(
    owner: &T,
    request: ComponentActionRequest<'_>,
    decide: fn(&T, &OwnedActionRequest) -> Option<D>,
    apply: fn(&mut T, D),
    describe: fn(&D) -> (ContentResponse, PreparedActionFocus),
) -> ActionPreparation {
    let request = OwnedActionRequest::read(request);
    let Some(decision) = decide(owner, &request) else {
        return ActionPreparation::Unsupported;
    };
    let (response, focus) = describe(&decision);
    ActionPreparation::Ready(PreparedComponentAction::new(
        response,
        focus,
        Box::new(OriginalEffect {
            request,
            decision,
            decide,
            apply,
        }),
    ))
}
#[cfg(test)]
mod tests {
    use super::super::Button;
    use super::*;
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(
            &mut self,
            _: &str,
            _: tekes_ui_tokens::proposal::TextRole,
            _: f32,
        ) -> tekes_ui_scene::Size {
            tekes_ui_scene::Size::new(40., 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: tekes_ui_tokens::proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    const TARGET: ComponentId = ComponentId(9991);
    const OLD: ComponentId = ComponentId(9992);
    const SCROLL: ComponentId = ComponentId(9993);
    fn host() -> ContentHost {
        let mut scroll = ScrollContent::new(
            ScrollArea::default(),
            Content::new(TARGET, Button::new("target")),
        );
        scroll.viewport_height = Some(100.);
        let mut h = ContentHost::new(Content::new(
            ComponentId(9990),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(SCROLL, scroll),
                    Content::new(OLD, Button::new("old")),
                ],
            },
        ))
        .unwrap();
        render(&mut h);
        assert!(h.focus(Some(OLD), true));
        h.set_action_events(true);
        h
    }
    fn render(h: &mut ContentHost) {
        let f = Rect::new(0., 0., 300., 200.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    }
    #[test]
    fn prepared_pointer_publishes_only_admitted_focus_and_keeps_single_activation() {
        for (preserve, same_owner) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut h = host();
            if same_owner {
                assert!(h.focus(Some(TARGET), true));
            }
            let before_focus = h.focused();
            h.get_mut(TARGET).unwrap().preserve_activation_focus = preserve;
            // get_mut withdraws the geometry lease. Restore the normal render
            // lifecycle before testing accepted focus through the Scroll owner.
            render(&mut h);
            let point = h
                .get(TARGET)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .layout()
                .unwrap()
                .frame
                .center();
            assert!(h.get(TARGET).unwrap().visible().unwrap().contains(point));
            let response = h.event(&ContentEvent::PointerDown(point));
            assert!(
                response.consumed,
                "preserve={preserve}, same_owner={same_owner}"
            );
            assert_eq!(h.pointer_focus_requested(), !preserve);
            assert_eq!(
                h.focused(),
                if preserve { before_focus } else { Some(TARGET) }
            );
            h.event(&ContentEvent::PointerUp(point));
            assert!(
                !h.pointer_focus_requested(),
                "focus intent belongs only to pointer-down"
            );
            assert_eq!(
                h.get(TARGET)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .activations,
                1
            );
            assert_eq!(
                h.drain_actions()
                    .iter()
                    .filter(|action| action.target == TARGET
                        && action.kind == ContentActionKind::Activate)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn actual_capture_and_activation_maxima_refuse_before_first_effect() {
        for capture in [true, false] {
            let mut h = host();
            if capture {
                h.capture_epoch = u64::MAX;
            } else {
                h.get_mut(TARGET)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .activations = usize::MAX;
                render(&mut h);
            }
            let epoch = h.focus_epoch;
            let point = h
                .get(TARGET)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .layout()
                .unwrap()
                .frame
                .center();
            assert!(h.get(TARGET).unwrap().visible().unwrap().contains(point));
            let r = if capture {
                h.event(&ContentEvent::PointerDown(point))
            } else {
                h.accessibility_action(TARGET, None, AccessibilityAction::Press)
            };
            assert!(!r.consumed && !r.activated);
            assert_eq!(h.focused, Some(OLD));
            assert_eq!(h.focus_epoch, epoch);
            assert!(h.captured.is_none());
            assert!(h.drain_action_records().is_empty());
            assert!(
                !h.get(TARGET)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .button
                    .state
                    .pressed
            );
        }
    }
    #[test]
    fn stale_geometry_focus_plan_and_owner_decision_reject_before_effect() {
        let mut h = host();
        let p = h
            .prepare_focus(Some(TARGET), true, None, FocusOrigin::Accessibility)
            .unwrap();
        h.get_mut(SCROLL)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .viewport_height = Some(80.);
        assert!(!h.validate_focus(&p));
        assert_eq!(h.apply_focus(p), FocusAdmission::Rejected);
        assert_eq!(h.focused, Some(OLD));
        render(&mut h);
        let n = h.get(TARGET).unwrap();
        let ActionPreparation::Ready(p) =
            n.component
                .prepare_action(ComponentActionRequest::Accessibility {
                    part: None,
                    action: &AccessibilityAction::Press,
                })
        else {
            panic!("original Button supports preparation")
        };
        assert!(p.effect.validate(&*n.component));
        h.get_mut(TARGET)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .activations += 1;
        assert!(!p.effect.validate(&*h.get(TARGET).unwrap().component));
        assert_eq!(h.focused, Some(OLD));
        assert!(h.drain_action_records().is_empty());
    }

    #[test]
    fn ordinary_rect_focus_counter_failure_precedes_target_action() {
        let mut button = Button::new("target");
        button.pointer_activation_phase = ButtonActivationPhase::Press;
        let mut h = ContentHost::new(Content::new(
            ComponentId(9990),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(TARGET, button),
                    Content::new(OLD, Button::new("old")),
                ],
            },
        ))
        .unwrap();
        render(&mut h);
        assert!(h.focus(Some(OLD), true));
        h.set_action_events(true);
        h.focus_epoch = u64::MAX;
        let point = h.get(TARGET).unwrap().frame.center();
        let response = h.event(&ContentEvent::PointerDown(point));
        let target = h.get(TARGET).unwrap().component::<Button>().unwrap();
        let observed = (
            response.consumed,
            response.activated,
            h.captured.is_some(),
            h.focused,
            h.focus_epoch,
            target.activations,
            target.button.state.pressed,
        );
        assert_eq!(
            observed,
            (false, false, false, Some(OLD), u64::MAX, 0, false)
        );
        assert!(h.drain_action_records().is_empty());
        assert_eq!(h.focused, Some(OLD));
        assert_eq!(h.focus_epoch, u64::MAX);
        assert_eq!(
            h.get(TARGET)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
        assert!(
            !h.get(TARGET)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .pressed
        );
    }

    #[test]
    fn ordinary_ax_part_counter_gate_precedes_action_but_same_owner_remains_allowed() {
        #[derive(Debug)]
        struct Owner {
            calls: usize,
        }
        impl Component for Owner {
            fn focusable(&self) -> bool {
                true
            }
            fn measure(
                &mut self,
                _: &mut dyn ButtonTextRenderer,
                _: tekes_ui_scene::Size,
            ) -> crate::layout::MeasuredChild {
                crate::layout::MeasuredChild::new(tekes_ui_scene::Size::new(100., 40.))
            }
            fn accessibility_action(
                &mut self,
                part: Option<usize>,
                action: &AccessibilityAction,
            ) -> ContentResponse {
                if part == Some(7) && matches!(action, AccessibilityAction::Focus) {
                    self.calls += 1;
                    ContentResponse::consumed(true)
                } else {
                    ContentResponse::default()
                }
            }
        }
        for same_owner in [true, false] {
            let mut h = ContentHost::new(Content::new(
                ComponentId(9990),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![
                        Content::new(TARGET, Owner { calls: 0 }),
                        Content::new(OLD, Button::new("old")),
                    ],
                },
            ))
            .unwrap();
            render(&mut h);
            let old = if same_owner { TARGET } else { OLD };
            assert!(h.focus(Some(old), true));
            h.focus_epoch = u64::MAX;
            let response = h.accessibility_action(TARGET, Some(7), AccessibilityAction::Focus);
            let observed = (
                response.consumed,
                h.get(TARGET).unwrap().component::<Owner>().unwrap().calls,
                h.focused,
                h.focus_epoch,
            );
            assert_eq!(
                observed,
                (same_owner, usize::from(same_owner), Some(old), u64::MAX)
            );
        }
    }

    #[test]
    fn ordinary_predictable_layout_without_prepared_action_keeps_explicit_ime_and_capacity_refusals()
     {
        #[derive(Debug)]
        struct OrdinaryLayout {
            children: Vec<Content>,
            explicit_reveal: bool,
            composing: bool,
            arranged: bool,
        }
        impl Component for OrdinaryLayout {
            fn host_reveal_passthrough(&self) -> bool {
                self.arranged && self.children.len() == 1
            }
            fn children(&self) -> &[Content] {
                &self.children
            }
            fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
                Some(&mut self.children)
            }
            fn requires_focus_reveal(&self) -> bool {
                self.explicit_reveal
            }
            fn composing(&self) -> bool {
                self.composing
            }
            fn focusable(&self) -> bool {
                self.children.is_empty()
            }
            fn measure(
                &mut self,
                text: &mut dyn ButtonTextRenderer,
                available: tekes_ui_scene::Size,
            ) -> crate::layout::MeasuredChild {
                if let Some(child) = self.children.first_mut() {
                    child.measure(text, available);
                }
                crate::layout::MeasuredChild::new(tekes_ui_scene::Size::new(
                    available.width,
                    if self.children.is_empty() { 40. } else { 400. },
                ))
            }
            fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
                self.arranged = false;
                if let Some(child) = self.children.first_mut() {
                    child.arrange(
                        text,
                        Rect::new(frame.min_x(), frame.min_y() + 300., frame.size.width, 40.),
                        clip,
                    );
                }
                self.arranged = [frame, clip].into_iter().all(|rect| {
                    [
                        rect.min_x(),
                        rect.min_y(),
                        rect.max_x(),
                        rect.max_y(),
                        rect.size.width,
                        rect.size.height,
                    ]
                    .into_iter()
                    .all(f32::is_finite)
                        && rect.size.width >= 0.
                        && rect.size.height >= 0.
                });
            }
        }
        fn ordinary(explicit_reveal: bool, composing: bool) -> ContentHost {
            let body = Content::new(
                ComponentId(9994),
                OrdinaryLayout {
                    children: vec![Content::new(TARGET, Button::new("ordinary target"))],
                    explicit_reveal,
                    composing: false,
                    arranged: false,
                },
            );
            let mut scroll = ScrollContent::new(ScrollArea::default(), body);
            scroll.viewport_height = Some(100.);
            let old = Content::new(
                OLD,
                OrdinaryLayout {
                    children: vec![],
                    explicit_reveal: false,
                    composing,
                    arranged: false,
                },
            );
            let mut h = ContentHost::new(Content::new(
                ComponentId(9990),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 0.,
                    },
                    flexible_child: None,
                    children: vec![Content::new(SCROLL, scroll), old],
                },
            ))
            .unwrap();
            render(&mut h);
            assert!(h.focus(Some(OLD), true));
            h.set_action_events(true);
            h
        }
        for explicit in [false, true] {
            let mut h = ordinary(explicit, false);
            assert_eq!(
                h.prepare_focus(Some(TARGET), true, None, FocusOrigin::Accessibility)
                    .is_ok(),
                !explicit,
                "the fixed arranged wrapper declares geometry, not prepared-action support"
            );
            let response = h.accessibility_action(TARGET, None, AccessibilityAction::Press);
            assert_eq!(response.consumed, !explicit);
            assert_eq!(h.focused(), Some(if explicit { OLD } else { TARGET }));
            assert_eq!(
                h.get(TARGET)
                    .unwrap()
                    .component::<Button>()
                    .unwrap()
                    .activations,
                usize::from(!explicit)
            );
            assert_eq!(h.drain_actions().len(), usize::from(!explicit));
            assert_eq!(
                h.get(SCROLL)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
                    .scroll
                    .offset
                    .y
                    > 0.,
                !explicit
            );
        }
        for explicit in [false, true] {
            let mut h = ordinary(explicit, false);
            let response = h.event(&ContentEvent::Key {
                key: ControlKey::Tab { reverse: false },
                down: true,
            });
            assert!(response.consumed);
            assert_eq!(response.changed, !explicit);
            assert_eq!(h.focused(), Some(if explicit { OLD } else { TARGET }));
            // Tab may commit a real Scroll move; refresh normal geometry before re-entry.
            render(&mut h);
            h.focus(None, false);
            assert_eq!(h.enter_focus(false), !explicit);
            assert_eq!(h.focused(), if explicit { None } else { Some(TARGET) });
        }
        let mut stale = ordinary(false, false);
        let owner = stale.get(TARGET).unwrap();
        let ActionPreparation::Ready(action) =
            owner
                .component
                .prepare_action(ComponentActionRequest::Accessibility {
                    part: None,
                    action: &AccessibilityAction::Press,
                })
        else {
            panic!("original Button has a prepared decision");
        };
        let route = ContentRoute::capture(
            &stale.root,
            TARGET,
            HostRevealBudget::default().max_path_nodes,
        )
        .unwrap();
        let actions = [OwnerAction { route, action }];
        assert_eq!(
            stale.validate_action_owners(&actions, None),
            None,
            "current ordinary wrapper lacks the prepared-hook contract"
        );
        stale
            .get_mut(TARGET)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .activations += 1;
        assert_eq!(
            stale.validate_action_owners(&actions, None),
            Some(false),
            "unknown ancestry cannot mask a stale original decision"
        );
        let mut pointer_max = ordinary(false, false);
        assert!(pointer_max.focus(Some(TARGET), true));
        render(&mut pointer_max);
        assert!(pointer_max.focus(Some(OLD), true));
        let point = pointer_max.get(TARGET).unwrap().frame.center();
        assert!(
            pointer_max
                .get(TARGET)
                .unwrap()
                .visible()
                .is_some_and(|r| r.contains(point))
        );
        let button = pointer_max
            .get_mut(TARGET)
            .unwrap()
            .component_mut::<Button>()
            .unwrap();
        button.pointer_activation_phase = ButtonActivationPhase::Press;
        button.activations = usize::MAX;
        let before = format!("{:?}", pointer_max.root);
        let response = pointer_max.event(&ContentEvent::PointerDown(point));
        assert!(!response.consumed && !response.activated);
        assert_eq!(pointer_max.focused(), Some(OLD));
        assert!(pointer_max.pointer_capture().is_none());
        assert_eq!(format!("{:?}", pointer_max.root), before);
        assert!(pointer_max.drain_actions().is_empty());
        for failure in 0..3 {
            let mut h = ordinary(false, failure == 1);
            if failure == 0 {
                h.focus_epoch = u64::MAX;
            }
            if failure == 2 {
                h.get_mut(TARGET)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .activations = usize::MAX;
            }
            let before = format!("{:?}", h.root);
            let epoch = h.focus_epoch;
            let response = h.accessibility_action(TARGET, None, AccessibilityAction::Press);
            assert!(!response.consumed && !response.activated);
            assert_eq!(h.focused(), Some(OLD));
            assert_eq!(h.focus_epoch, epoch);
            assert_eq!(format!("{:?}", h.root), before);
            assert!(h.drain_actions().is_empty());
        }
    }
    #[derive(Debug)]
    struct RectangularInput {
        opt_in: bool,
        arranged: bool,
        insufficient: bool,
        disabled: bool,
        pointer_downs: usize,
    }
    impl Component for RectangularInput {
        fn rectangular_input_geometry(&self) -> bool {
            self.opt_in && self.arranged
        }
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            available: tekes_ui_scene::Size,
        ) -> crate::layout::MeasuredChild {
            crate::layout::MeasuredChild::new(tekes_ui_scene::Size::new(available.width, 32.))
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
            self.arranged = [frame, clip].into_iter().all(|r| {
                [
                    r.min_x(),
                    r.min_y(),
                    r.max_x(),
                    r.max_y(),
                    r.size.width,
                    r.size.height,
                ]
                .into_iter()
                .all(f32::is_finite)
                    && r.size.width > 0.
                    && r.size.height > 0.
            });
        }
        fn interactive(&self) -> bool {
            true
        }
        fn focusable(&self) -> bool {
            true
        }
        fn requires_focus_reveal(&self) -> bool {
            self.insufficient
        }
        fn required_focus_space(&self) -> RequiredFocusSpace {
            if self.insufficient {
                RequiredFocusSpace::Insufficient
            } else {
                RequiredFocusSpace::NotRequired
            }
        }
        fn semantics(&self) -> ContentSemantics {
            let mut semantics = ContentSemantics::new(ContentRole::Group, "rectangular input");
            semantics.disabled = self.disabled;
            semantics
        }
        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            if matches!(event, ContentEvent::PointerDown(_)) {
                self.pointer_downs += 1;
                ContentResponse::consumed(false)
            } else {
                ContentResponse::default()
            }
        }
    }
    fn rectangular_scroll(opt_in: bool, insufficient: bool, disabled: bool) -> ContentHost {
        ContentHost::new(Content::new(
            SCROLL,
            ScrollContent::new(
                ScrollArea::default(),
                Content::new(
                    TARGET,
                    RectangularInput {
                        opt_in,
                        arranged: false,
                        insufficient,
                        disabled,
                        pointer_downs: 0,
                    },
                ),
            ),
        ))
        .unwrap()
    }
    #[test]
    fn rectangular_input_geometry_opt_in_uses_scroll_focus_plan_and_unknown_still_refuses() {
        for opt_in in [false, true] {
            let mut h = rectangular_scroll(opt_in, false, false);
            assert!(
                !h.get(TARGET)
                    .unwrap()
                    .component::<RectangularInput>()
                    .unwrap()
                    .rectangular_input_geometry()
            );
            render(&mut h);
            assert_eq!(
                h.get(TARGET)
                    .unwrap()
                    .component::<RectangularInput>()
                    .unwrap()
                    .curve_subtree_capability(),
                CurveSubtreeCapability::Unsupported
            );
            if opt_in {
                assert!(
                    h.prepare_focus(Some(TARGET), false, None, FocusOrigin::Pointer)
                        .is_ok()
                );
            }
            let response = h.event(&ContentEvent::PointerDown(Point::new(10., 10.)));
            assert_eq!(response.consumed, opt_in);
            assert_eq!(h.focused(), opt_in.then_some(TARGET));
            assert_eq!(h.pointer_focus_requested(), opt_in);
            assert_eq!(
                h.get(TARGET)
                    .unwrap()
                    .component::<RectangularInput>()
                    .unwrap()
                    .pointer_downs,
                usize::from(opt_in)
            );
        }
    }
    #[test]
    fn rectangular_input_geometry_retains_space_inactive_and_disabled_refusals() {
        for (insufficient, inactive, disabled) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let mut h = rectangular_scroll(true, insufficient, disabled);
            render(&mut h);
            assert!(
                h.get(TARGET)
                    .unwrap()
                    .component::<RectangularInput>()
                    .unwrap()
                    .rectangular_input_geometry()
            );
            if inactive {
                h.set_window_active(false);
            }
            assert!(
                h.prepare_focus(Some(TARGET), false, None, FocusOrigin::Pointer)
                    .is_err()
            );
            let response = h.event(&ContentEvent::PointerDown(Point::new(10., 10.)));
            assert!(!response.consumed);
            assert!(h.focused().is_none());
            assert!(!h.pointer_focus_requested());
            assert_eq!(
                h.get(TARGET)
                    .unwrap()
                    .component::<RectangularInput>()
                    .unwrap()
                    .pointer_downs,
                0
            );
        }
    }
    #[test]
    fn rectangular_input_geometry_does_not_grant_curved_panel_admission() {
        let mut original = Content::new(
            TARGET,
            RectangularInput {
                opt_in: true,
                arranged: false,
                insufficient: false,
                disabled: false,
                pointer_downs: 0,
            },
        );
        let frame = Rect::new(0., 0., 100., 32.);
        original.measure(&mut Text, frame.size);
        original.arrange(&mut Text, frame, frame);
        assert!(
            original
                .component::<RectangularInput>()
                .unwrap()
                .rectangular_input_geometry()
        );
        assert!(matches!(
            PanelContent::new(original),
            Err(CompositionError::UnsupportedCurveChild(TARGET))
        ));
    }
}

#[cfg(test)]
mod overlay_focus_admission_tests {
    use super::*;
    #[derive(Debug)]
    struct Scope {
        open: bool,
        children: Vec<Content>,
    }
    impl Component for Scope {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(available)
        }
        fn children(&self) -> &[Content] {
            &self.children
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.children)
        }
        fn overlay_open(&self) -> bool {
            self.open
        }
        fn overlay_scope(&self) -> Option<OverlayScope> {
            Some(OverlayScope::default())
        }
    }
    #[test]
    fn new_scope_cannot_hide_original_editor_from_focus_transfer_admission() {
        let editor = ComponentId(80901);
        let target = ComponentId(80903);
        let field = TextFieldContent::new("Composer", "");
        let mut host = ContentHost::new(Content::new(
            ComponentId(80900),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(editor, field),
                    Content::new(
                        ComponentId(80902),
                        Scope {
                            open: true,
                            children: vec![Content::new(target, Button::new("Popup action"))],
                        },
                    ),
                ],
            },
        ))
        .unwrap();
        host.focused = Some(editor);
        // Mount first: the real lifecycle establishes the original editor identity.
        let field = host
            .root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        field.set_focus(true, false);
        field.event(&ContentEvent::Preedit {
            text: "中".into(),
            selection: 1..1,
            clauses: vec![],
            replacement: None,
        });
        field.surface.input.test_edit_epoch(u64::MAX);
        let original = field.surface.input.action_snapshot();
        let binding = field.surface.input.controlled_binding();
        assert!(host.scope().find_active(editor).is_none());
        assert!(host.focused_text().is_none());
        let state = EditorState::read(&host);
        assert_eq!(state.binding, Some(binding));
        assert_eq!(state.text, Some(original));
        assert!(state.composing);
        assert!(matches!(
            host.prepare_focus(None, false, None, FocusOrigin::Direct),
            Err(HostRevealError::IdentityExhausted)
        ));
        for origin in [FocusOrigin::Traversal, FocusOrigin::Accessibility] {
            assert!(matches!(
                host.prepare_focus(Some(target), true, None, origin),
                Err(HostRevealError::CompositionBarrier)
            ));
        }
        assert_eq!(host.focused(), Some(editor));
        assert!(
            host.focus_admission_text_surface()
                .unwrap()
                .input
                .marked
                .is_some()
        );
        // Restore the test-injected capacity after asserting refusal, so normal
        // host destruction can unmark the still-focused original editor.
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .test_edit_epoch(0);
    }

    #[test]
    fn scope_open_capacity_refusal_does_not_cancel_original_editor_scroll() {
        let editor = ComponentId(80911);
        let scope = ComponentId(80912);
        let target = ComponentId(80913);
        let field = TextFieldContent::new("Composer", "");
        let mut host = ContentHost::new(Content::new(
            ComponentId(80910),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(editor, field),
                    Content::new(
                        scope,
                        Scope {
                            open: false,
                            children: vec![Content::new(target, Button::new("Popup action"))],
                        },
                    ),
                ],
            },
        ))
        .unwrap();
        host.focused = Some(editor);
        // Capacity exhaustion is an input after mounting, not a lifecycle failure.
        let field = host
            .root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        field.set_focus(true, false);
        field.event(&ContentEvent::Preedit {
            text: "中".into(),
            selection: 1..1,
            clauses: vec![],
            replacement: None,
        });
        field.surface.input.test_edit_epoch(u64::MAX);
        // This exercises the original focus admission before any target reveal
        // work. The active original scope still has an eligible focus intent.
        host.root.find_mut(target).unwrap().frame = Rect::new(10., 10., 80., 24.);
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .scroll
            .overscroll = Point::new(0., 8.);
        let before_input = host
            .focus_admission_text_surface()
            .unwrap()
            .input
            .action_snapshot();
        let before_scroll = format!("{:?}", host.focus_admission_text_surface().unwrap().scroll);
        let before_epoch = host.focus_epoch();
        host.root
            .find_mut(scope)
            .unwrap()
            .component_mut::<Scope>()
            .unwrap()
            .open = true;

        host.reconcile_active_inputs();

        assert_eq!(host.focused(), Some(editor));
        assert_eq!(host.focus_epoch(), before_epoch);
        let surface = host.focus_admission_text_surface().unwrap();
        assert_eq!(surface.input.action_snapshot(), before_input);
        assert_eq!(surface.scroll.overscroll, Point::new(0., 8.));
        assert_eq!(format!("{:?}", surface.scroll), before_scroll);
        assert!(surface.input.marked.is_some());
        assert!(host.overlay_scopes.last().unwrap().focus_pending);
        assert!(host.drain_actions().is_empty());
        // Restore the test-injected capacity after asserting refusal, so normal
        // host destruction can unmark the still-focused original editor.
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .test_edit_epoch(0);
    }

    struct TestText;
    impl ButtonTextRenderer for TestText {
        fn measure(&mut self, _: &str, _: tekes_ui_tokens::proposal::TextRole, _: f32) -> Size {
            Size::new(48., 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: tekes_ui_tokens::proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }

        fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
            Some(self)
        }
    }
    impl crate::TextSurfaceTextRenderer for TestText {
        fn push_visible(
            &mut self,
            _: &mut Scene,
            _: &tekes_ui_text::Paragraph,
            _: Point,
            _: tekes_ui_tokens::Rgba,
            _: Rect,
        ) {
        }
    }
    impl crate::TextSurfaceHost for TestText {
        fn shape_surface(
            &mut self,
            text: &str,
            role: proposal::TextRole,
            _: Option<f32>,
        ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
            // Deterministic paragraph service for pure focus-admission units.
            // Actual font shaping remains covered by text_query's macOS CoreText tests.
            let line_height = f64::from(role.style().line_height);
            std::rc::Rc::new(tekes_ui_text::Paragraph {
                lines: Vec::new(),
                height: line_height,
                width: 0.,
                default_line_height: line_height,
                fonts: Vec::new(),
                utf16_len: text.encode_utf16().count(),
            })
        }
    }
    fn render_popover_host(host: &mut ContentHost) {
        let frame = Rect::new(0., 0., 360., 280.);
        host.render(
            &mut Scene::new(),
            &mut TestText,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
    }
    fn escape(host: &mut ContentHost) {
        host.event(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true,
        });
    }
    #[test]
    fn real_popover_direct_focus_dismisses_atomically_and_escape_keeps_new_focus() {
        let root = ComponentId(80930);
        let caller = ComponentId(80931);
        let outside = ComponentId(80932);
        let disabled_target = ComponentId(80933);
        let popup = ComponentId(80934);
        let inside = ComponentId(80935);
        let mut host = ContentHost::new(Content::new(
            root,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(caller, Button::new("Caller")),
                    Content::new(outside, TextFieldContent::new("Outside", "")),
                    Content::new(disabled_target, Button::new("Disabled target")),
                    Content::new(
                        popup,
                        PopoverContent::new(
                            "Transient",
                            Content::new(inside, Button::new("Inside")),
                        ),
                    ),
                ],
            },
        ))
        .unwrap();
        render_popover_host(&mut host);
        assert!(host.focus(Some(caller), true));
        let session = host.show_popover(popup, caller).unwrap();
        render_popover_host(&mut host);
        host.reconcile_active_inputs();
        assert!(
            host.get(popup)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert_eq!(host.overlay_scope_id(), Some(popup));

        let focus = host.focused();
        let root_before = format!("{:?}", host.root);
        assert!(!host.focus(Some(ComponentId(80936)), true));
        assert_eq!(host.focused(), focus);
        assert_eq!(format!("{:?}", host.root), root_before);

        host.get_mut(disabled_target)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = true;
        let disabled_before = format!("{:?}", host.root);
        assert!(!host.focus(Some(disabled_target), true));
        assert_eq!(host.focused(), focus);
        assert_eq!(format!("{:?}", host.root), disabled_before);
        host.get_mut(disabled_target)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = false;

        let epoch = host.focus_epoch();
        host.focus_epoch = u64::MAX;
        let exhausted_before = format!("{:?}", host.root);
        assert!(!host.focus(Some(outside), true));
        assert_eq!(host.focused(), focus);
        assert_eq!(format!("{:?}", host.root), exhausted_before);
        host.focus_epoch = epoch;

        for origin in [
            FocusOrigin::Pointer,
            FocusOrigin::Traversal,
            FocusOrigin::Accessibility,
        ] {
            assert!(matches!(
                host.prepare_focus(Some(outside), true, None, origin),
                Err(HostRevealError::UnavailableTarget)
            ));
        }
        assert!(host.focus(Some(outside), true));
        assert_eq!(host.focused(), Some(outside));
        let closed = host
            .get_mut(popup)
            .unwrap()
            .component_mut::<PopoverContent>()
            .unwrap()
            .take_closed();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].session, session);
        assert_eq!(closed[0].reason, PopoverCloseReason::Dismiss);
        let popup = host
            .get(popup)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap();
        assert!(!popup.is_open());
        assert!(
            popup.is_presented(),
            "Dismiss retains its ordinary exit paint"
        );
        assert!(host.scope().find_active(outside).is_some());
        assert!(
            host.focused_text().is_some(),
            "native text query follows the accepted scope"
        );
        escape(&mut host);
        assert_eq!(host.focused(), Some(outside));
        assert!(host.focused_text().is_some());
    }

    #[test]
    fn nested_real_popover_focus_retires_only_the_inner_session() {
        let root = ComponentId(80940);
        let caller = ComponentId(80941);
        let outer = ComponentId(80942);
        let parent_target = ComponentId(80943);
        let inner_anchor = ComponentId(80944);
        let inner = ComponentId(80945);
        let inner_target = ComponentId(80946);
        let body = Content::new(
            ComponentId(80947),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(parent_target, Button::new("Parent target")),
                    Content::new(inner_anchor, Button::new("Inner anchor")),
                    Content::new(
                        inner,
                        PopoverContent::new(
                            "Inner",
                            Content::new(inner_target, Button::new("Inner body")),
                        ),
                    ),
                ],
            },
        );
        let mut host = ContentHost::new(Content::new(
            root,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(caller, Button::new("Caller")),
                    Content::new(outer, PopoverContent::new("Outer", body)),
                ],
            },
        ))
        .unwrap();
        render_popover_host(&mut host);
        assert!(host.focus(Some(caller), true));
        host.show_popover(outer, caller).unwrap();
        render_popover_host(&mut host);
        host.reconcile_active_inputs();
        let outer_session = host
            .get(outer)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .current_session()
            .unwrap();
        host.show_popover(inner, inner_anchor).unwrap();
        render_popover_host(&mut host);
        host.reconcile_active_inputs();
        let inner_session = host
            .get(inner)
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .current_session()
            .unwrap();
        assert_eq!(host.overlay_scope_id(), Some(inner));

        assert!(host.focus(Some(parent_target), true));
        assert_eq!(host.focused(), Some(parent_target));
        assert!(
            host.get(outer)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert!(
            !host
                .get(inner)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert_eq!(
            host.get_mut(inner)
                .unwrap()
                .component_mut::<PopoverContent>()
                .unwrap()
                .take_closed()[0]
                .session,
            inner_session
        );
        assert!(host.scope().find_active(parent_target).is_some());
        assert_eq!(host.input_overlay_owner().map(|(id, _)| id), Some(outer));
        escape(&mut host);
        assert!(
            !host
                .get(outer)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert_eq!(
            host.get_mut(outer)
                .unwrap()
                .component_mut::<PopoverContent>()
                .unwrap()
                .take_closed()[0]
                .session,
            outer_session
        );
        assert_eq!(host.focused(), Some(caller));
    }

    #[test]
    fn real_popover_inside_dialog_cannot_direct_focus_through_modal_boundary() {
        let root = ComponentId(80950);
        let dialog = ComponentId(80951);
        let background = ComponentId(80952);
        let caller = ComponentId(80953);
        let outside_popup = ComponentId(80954);
        let popup = ComponentId(80955);
        let inside = ComponentId(80956);
        let body = Content::new(
            ComponentId(80957),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(caller, Button::new("Dialog caller")),
                    Content::new(outside_popup, TextFieldContent::new("Dialog target", "")),
                    Content::new(
                        popup,
                        PopoverContent::new(
                            "Dialog popup",
                            Content::new(inside, Button::new("Inside")),
                        ),
                    ),
                ],
            },
        );
        let mut host = ContentHost::new(Content::new(
            root,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(background, Button::new("Background")),
                    Content::new(
                        dialog,
                        DialogContent::new(
                            "Dialog",
                            Content::new(ComponentId(80958), TextContent::new("Header")),
                            body,
                            Content::new(ComponentId(80959), TextContent::new("Footer")),
                        ),
                    ),
                ],
            },
        ))
        .unwrap();
        host.show_dialog(dialog).unwrap();
        render_popover_host(&mut host);
        assert_eq!(host.modal.map(|(id, _)| id), Some(dialog));
        // Dialog may already have selected its first eligible caller. The
        // public bool reports a change; assert the actual owner instead.
        host.focus(Some(caller), true);
        assert_eq!(host.focused(), Some(caller));
        host.show_popover(popup, caller).unwrap();
        render_popover_host(&mut host);
        host.reconcile_active_inputs();
        assert!(
            host.get(popup)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        let focus = host.focused();
        let before = format!("{:?}", host.root);
        assert!(!host.focus(Some(background), true));
        assert_eq!(host.focused(), focus);
        assert_eq!(format!("{:?}", host.root), before);
        assert_eq!(host.modal.map(|(id, _)| id), Some(dialog));

        assert!(host.focus(Some(outside_popup), true));
        assert!(
            !host
                .get(popup)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert_eq!(host.modal.map(|(id, _)| id), Some(dialog));
        assert!(host.scope().find_active(outside_popup).is_some());
        assert!(host.focused_text().is_some());
        assert!(!host.focus(Some(background), true));
        assert_eq!(host.focused(), Some(outside_popup));
        assert_eq!(host.modal.map(|(id, _)| id), Some(dialog));
    }

    #[test]
    fn actual_popover_preserves_marked_editor_and_scroll_when_unmark_capacity_is_exhausted() {
        let root = ComponentId(80960);
        let caller = ComponentId(80961);
        let popup = ComponentId(80962);
        let editor = ComponentId(80963);
        let outside = ComponentId(80964);
        let mut scroll = ScrollContent::new(
            ScrollArea::default(),
            Content::new(editor, TextFieldContent::new("Composer", "draft")),
        );
        scroll.viewport_height = Some(80.);
        let mut host = ContentHost::new(Content::new(
            root,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(caller, Button::new("Caller")),
                    Content::new(outside, Button::new("Outside")),
                    Content::new(
                        popup,
                        PopoverContent::new(
                            "Composer popup",
                            Content::new(ComponentId(80965), scroll),
                        ),
                    ),
                ],
            },
        ))
        .unwrap();
        render_popover_host(&mut host);
        assert!(host.focus(Some(caller), true));
        host.show_popover(popup, caller).unwrap();
        render_popover_host(&mut host);
        host.reconcile_active_inputs();
        assert!(host.focus(Some(editor), true));
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .event(&ContentEvent::Preedit {
                text: "中".into(),
                selection: 1..1,
                clauses: Vec::new(),
                replacement: None,
            });
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .test_edit_epoch(u64::MAX);
        let before_root = format!("{:?}", host.root);
        let before_focus = host.focused();
        let before_epoch = host.focus_epoch();
        let before_scroll = format!(
            "{:?}",
            host.get(popup).unwrap().component.children()[0]
                .component::<ScrollContent>()
                .unwrap()
                .scroll
        );
        assert!(!host.focus(Some(outside), true));
        assert_eq!(host.focused(), before_focus);
        assert_eq!(host.focus_epoch(), before_epoch);
        assert_eq!(format!("{:?}", host.root), before_root);
        assert!(
            host.get(popup)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open()
        );
        assert!(
            host.focus_admission_text_surface()
                .unwrap()
                .input
                .marked
                .is_some()
        );
        let after_scroll = format!(
            "{:?}",
            host.get(popup).unwrap().component.children()[0]
                .component::<ScrollContent>()
                .unwrap()
                .scroll
        );
        assert_eq!(after_scroll, before_scroll);
        host.root
            .find_mut(editor)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .test_edit_epoch(0);
    }
}
