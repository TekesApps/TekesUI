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
            binding: host.focused_text_binding(),
            composing: host.is_composing(),
            text: host.focused_text().map(|s| s.input.action_snapshot()),
        }
    }
}
struct PreparedFocus {
    id: Option<ComponentId>,
    visible: bool,
    traversal: Option<bool>,
    origin: FocusOrigin,
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
    fn prepare_focus(
        &self,
        id: Option<ComponentId>,
        visible: bool,
        traversal: Option<bool>,
        origin: FocusOrigin,
    ) -> Result<PreparedFocus, HostRevealError> {
        self.prepare_focus_rect(id, visible, traversal, origin, None)
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
        if let Some(id) = id
            && self
                .scope()
                .find_active(id)
                .is_none_or(|n| !n.component.focusable() || n.component.semantics().disabled)
        {
            return Err(HostRevealError::UnavailableTarget);
        }
        if id.is_some()
            && id != self.focused
            && self.is_composing()
            && matches!(origin, FocusOrigin::Traversal | FocusOrigin::Accessibility)
        {
            return Err(HostRevealError::CompositionBarrier);
        }
        if id != self.focused && self.focused_text().is_some_and(|s| !s.input.can_unmark()) {
            return Err(HostRevealError::IdentityExhausted);
        }
        let next_epoch = if id != self.focused {
            self.focus_epoch
                .checked_add(1)
                .ok_or(HostRevealError::IdentityExhausted)?
        } else {
            self.focus_epoch
        };
        let plan = id
            .map(|id| match rect {
                Some(rect) => self.plan_focus_reveal_with_rect(id, Some(rect)),
                None => self.plan_focus_reveal(id),
            })
            .transpose()?;
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
        self.focused == p.focused
            && self.focus_epoch == p.epoch
            && self.reveal_clock.stamp().ok() == Some(p.stamp)
            && EditorState::read(self) == p.editor
            && (p.id.is_none() || self.window_active)
            && p.id.is_none_or(|id| {
                self.scope()
                    .find_active(id)
                    .is_some_and(|n| n.component.focusable() && !n.component.semantics().disabled)
            })
            && (p.id == self.focused || self.focused_text().is_none_or(|s| s.input.can_unmark()))
            && !(p.id.is_some()
                && p.id != self.focused
                && self.is_composing()
                && matches!(
                    p.origin,
                    FocusOrigin::Traversal | FocusOrigin::Accessibility
                ))
            && p.old
                .iter()
                .chain(p.new.iter())
                .all(|r| r.node(&self.root, r.depth()).is_some())
            && p.plan
                .as_ref()
                .is_none_or(|p| self.validate_host_reveal(p).is_ok())
    }
    fn apply_focus(&mut self, p: PreparedFocus) -> FocusAdmission {
        if !self.validate_focus(&p) {
            return FocusAdmission::Rejected;
        }
        if let Some(plan) = p.plan
            && self.commit_host_reveal(plan).is_err()
        {
            return FocusAdmission::Rejected;
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
        FocusAdmission::Accepted { changed }
    }
    /// Used only at an ordinary Host entry after UnsupportedLayout or absence
    /// of a prepared-hook contract. It grants no prepared transaction capability.
    pub(super) fn original_reveal_admissible(&self, id: ComponentId) -> bool {
        let Ok(route) = ContentRoute::capture(
            &self.root, id, HostRevealBudget::default().max_path_nodes,
        ) else { return false; };
        if (0..=route.depth()).any(|depth| route.node(&self.root, depth)
            .is_none_or(|node| node.component.requires_focus_reveal() || node.component::<ScrollContent>().is_some())) {
            return false;
        }
        self.reveal_clock.stamp().is_ok_and(|(_, input)| input.checked_add(1).is_some())
    }
    /// Ordinary focus keeps the pure prepared path whenever it is available.
    /// Legacy custom layout remains supported by the original dynamic hooks,
    /// without representing those hooks as a read-only/atomic reveal proof.
    pub(super) fn admit_ordinary_focus(&mut self, id: Option<ComponentId>, visible: bool, traversal: Option<bool>, origin: FocusOrigin) -> FocusAdmission {
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
        if changed { self.focus_epoch += 1; }
        if changed && let Some(owner) = self.focused.and_then(|old| self.root.find_mut(old)) {
            owner.component.set_focus(false, false);
        }
        self.focused = id;
        if let Some(id) = id {
            let frame = {
                let owner = self.root.find_mut(id).expect("validated ordinary focus owner");
                if let Some(reverse) = traversal { owner.component.set_focus_from_traversal(reverse); }
                else { owner.component.set_focus(true, visible); }
                owner.component.text_surface()
                    .filter(|surface| surface.presentation == crate::TextSurfacePresentation::Content)
                    .map_or(owner.frame, |surface| surface.caret())
            };
            self.root.reveal(id, frame);
        }
        self.dirty = true;
        self.help_focus(id, visible);
        FocusAdmission::Accepted { changed }
    }
    /// CurveLocal qualifies only the original ordinary geometry consumer, not
    /// prepared-effect purity. A real successful current reveal plan is required
    /// for focus transfer; UnsupportedLayout is never a success on this path.
    fn local_ordinary_action_admissible(&self, id: ComponentId, origin: FocusOrigin, wants_focus: bool) -> bool {
        let Some(node) = self.scope().find_active(id) else { return false; };
        if !self.window_active || node.component.semantics().disabled
            || node.component.curve_subtree_capability() != CurveSubtreeCapability::Local
            || !self.reveal_clock.stamp().is_ok_and(|(_, input)| input.checked_add(1).is_some())
            || (self.requires_curve_geometry() && self.geometry_snapshot().is_none_or(|snapshot| !snapshot.is_current())) {
            return false;
        }
        !wants_focus || self.prepare_focus(Some(id), matches!(origin, FocusOrigin::Accessibility), None, origin).is_ok()
    }
    fn ordinary_pointer_action_admissible(&self, id: ComponentId) -> bool {
        if self.original_reveal_admissible(id) { return true; }
        let wants_focus = self.scope().find_active(id).is_some_and(|node| node.component.focusable()
            && !node.preserve_activation_focus && !node.component.preserves_activation_focus());
        self.local_ordinary_action_admissible(id, FocusOrigin::Pointer, wants_focus)
    }
    fn ordinary_ax_focus_admissible(&self, id: ComponentId, part: Option<usize>, action: &AccessibilityAction) -> bool {
        let candidate = matches!(action, AccessibilityAction::Focus | AccessibilityAction::Press | AccessibilityAction::SetSelection(_))
            && self.scope().find_active(id).is_some_and(|node| node.component.focusable())
            && (!matches!(action, AccessibilityAction::Press) || !self.semantic_preserves_activation_focus(id, part));
        if !self.original_reveal_admissible(id) {
            return self.local_ordinary_action_admissible(id, FocusOrigin::Accessibility, candidate);
        }
        if !candidate { return true; }
        match self.prepare_focus(Some(id), true, None, FocusOrigin::Accessibility) {
            Ok(_) => self.reveal_clock.stamp().is_ok_and(|(_, input)| input.checked_add(1).is_some()),
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
            if !action.action.effect.validate(&*owner.component) { return Some(false); }
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
                    let Some(owner) = route.node(&self.root, depth) else { return Some(false); };
                    unsupported |= owner.component.action_transaction_compatibility()
                        != ActionTransactionCompatibility::StableHooks;
                }
            }
            if !self.validate_focus(focus) { return Some(false); }
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
                node.id == focused && node.component.text_surface().is_some()
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
                    || self.validate_action_owners(&actions, None) == Some(false) {
                    Some(ContentResponse::default())
                } else { None };
            }
            let ActionPreparation::Ready(action) = node
                .component
                .prepare_action(ComponentActionRequest::PointerDown(event))
            else {
                // This original owner has no transaction contract. Keep its
                // existing reducer/capture behavior; do not reject a supported
                // pointer action merely because a focus plan cannot be staged.
                return if !self.ordinary_pointer_action_admissible(original_target)
                    || self.validate_action_owners(&actions, None) == Some(false) {
                    Some(ContentResponse::default())
                } else { None };
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
        if stamp.is_none() { return Some(ContentResponse::default()); }
        let owners_current = self.validate_action_owners(&actions, focus.as_ref()).or_else(|| {
            if focus.is_some() || self.ordinary_pointer_action_admissible(original_target) { None } else { Some(false) }
        })?;
        if !owners_current
            || self.capture_epoch != capture
            || self.reveal_clock.stamp().ok() != stamp
        {
            return Some(ContentResponse::default());
        }
        if let Some(p) = focus
            && matches!(self.apply_focus(p), FocusAdmission::Rejected)
        {
            return Some(ContentResponse::default());
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
        if !self.strict_focus_requirement(id) {
            // Other ordinary AX verbs keep their original non-transfer policy.
            // Strict cross-owner verbs below always need an original decision.
            if !matches!(
                action,
                AccessibilityAction::Focus
                    | AccessibilityAction::Press
                    | AccessibilityAction::SetSelection(_)
            ) {
                return None;
            }
            // Stable original Press owners use the same preflight/commit path
            // even on ordinary rectangles; this prevents an overflow or failed
            // focus admission from falling through to a partially mutating AX reducer.
            let stable_press = matches!(action, AccessibilityAction::Press)
                && self.scope().find_active(id).is_some_and(|n| {
                    n.component.action_transaction_compatibility()
                        == ActionTransactionCompatibility::StableHooks
                });
            if !stable_press {
                return if self.ordinary_ax_focus_admissible(id, part, action) { None }
                    else { Some(ContentResponse::default()) };
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
            return if self.ordinary_ax_focus_admissible(id, part, action) { None }
                else { Some(ContentResponse::default()) };
        }
        let ActionPreparation::Ready(prepared) = n
            .component
            .prepare_action(ComponentActionRequest::Accessibility { part, action })
        else {
            return if self.ordinary_ax_focus_admissible(id, part, action) { None }
                else { Some(ContentResponse::default()) };
        };
        if prepared.refused { return Some(ContentResponse::default()); }
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
                    if self.original_reveal_admissible(id) && prepared.effect.validate(&*n.component) => return None,
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
            None if focus.is_some() || self.ordinary_ax_focus_admissible(id, part, action) => return None,
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
    use super::*;
    use super::super::ButtonContent;
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
            Content::new(TARGET, ButtonContent::new("target")),
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
                    Content::new(OLD, ButtonContent::new("old")),
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
    fn actual_capture_and_activation_maxima_refuse_before_first_effect() {
        for capture in [true, false] {
            let mut h = host();
            if capture {
                h.capture_epoch = u64::MAX;
            } else {
                h.get_mut(TARGET)
                    .unwrap()
                    .component_mut::<ButtonContent>()
                    .unwrap()
                    .activations = usize::MAX;
                render(&mut h);
            }
            let epoch = h.focus_epoch;
            let point = h.get(TARGET).unwrap().frame.center();
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
                    .component::<ButtonContent>()
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
            .component_mut::<ButtonContent>()
            .unwrap()
            .activations += 1;
        assert!(!p.effect.validate(&*h.get(TARGET).unwrap().component));
        assert_eq!(h.focused, Some(OLD));
        assert!(h.drain_action_records().is_empty());
    }

    #[test]
    fn ordinary_rect_focus_counter_failure_precedes_target_action() {
        let mut button = ButtonContent::new("target");
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
                    Content::new(OLD, ButtonContent::new("old")),
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
        let target = h.get(TARGET).unwrap().component::<ButtonContent>().unwrap();
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
                .component::<ButtonContent>()
                .unwrap()
                .activations,
            0
        );
        assert!(
            !h.get(TARGET)
                .unwrap()
                .component::<ButtonContent>()
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
                        Content::new(OLD, ButtonContent::new("old")),
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
    fn ordinary_predictable_layout_without_prepared_action_keeps_explicit_ime_and_capacity_refusals() {
        #[derive(Debug)]
        struct OrdinaryLayout {
            children: Vec<Content>,
            explicit_reveal: bool,
            composing: bool,
            arranged: bool,
        }
        impl Component for OrdinaryLayout {
            fn host_reveal_passthrough(&self) -> bool { self.arranged && self.children.len() == 1 }
            fn children(&self) -> &[Content] { &self.children }
            fn children_mut(&mut self) -> Option<&mut Vec<Content>> { Some(&mut self.children) }
            fn requires_focus_reveal(&self) -> bool { self.explicit_reveal }
            fn composing(&self) -> bool { self.composing }
            fn focusable(&self) -> bool { self.children.is_empty() }
            fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: tekes_ui_scene::Size) -> crate::layout::MeasuredChild {
                if let Some(child) = self.children.first_mut() { child.measure(text, available); }
                crate::layout::MeasuredChild::new(tekes_ui_scene::Size::new(available.width, if self.children.is_empty() {40.} else {400.}))
            }
            fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
                self.arranged = false;
                if let Some(child) = self.children.first_mut() {
                    child.arrange(text, Rect::new(frame.min_x(), frame.min_y()+300., frame.size.width, 40.), clip);
                }
                self.arranged = [frame, clip].into_iter().all(|rect| [rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y(), rect.size.width, rect.size.height].into_iter().all(f32::is_finite) && rect.size.width >= 0. && rect.size.height >= 0.);
            }
        }
        fn ordinary(explicit_reveal: bool, composing: bool) -> ContentHost {
            let body = Content::new(ComponentId(9994), OrdinaryLayout {
                children: vec![Content::new(TARGET, ButtonContent::new("ordinary target"))],
                explicit_reveal,
                composing: false,
                arranged: false,
            });
            let mut scroll = ScrollContent::new(ScrollArea::default(), body);
            scroll.viewport_height = Some(100.);
            let old = Content::new(OLD, OrdinaryLayout { children: vec![], explicit_reveal: false, composing, arranged: false });
            let mut h = ContentHost::new(Content::new(ComponentId(9990), StackContent {
                model: Stack { axis: Axis::Vertical, gap: 0. }, flexible_child: None,
                children: vec![Content::new(SCROLL, scroll), old],
            })).unwrap();
            render(&mut h);
            assert!(h.focus(Some(OLD), true));
            h.set_action_events(true);
            h
        }
        for explicit in [false, true] {
            let mut h = ordinary(explicit, false);
            assert_eq!(h.prepare_focus(Some(TARGET), true, None, FocusOrigin::Accessibility).is_ok(), !explicit, "the fixed arranged wrapper declares geometry, not prepared-action support");
            let response = h.accessibility_action(TARGET, None, AccessibilityAction::Press);
            assert_eq!(response.consumed, !explicit);
            assert_eq!(h.focused(), Some(if explicit {OLD} else {TARGET}));
            assert_eq!(h.get(TARGET).unwrap().component::<ButtonContent>().unwrap().activations, usize::from(!explicit));
            assert_eq!(h.drain_actions().len(), usize::from(!explicit));
            assert_eq!(h.get(SCROLL).unwrap().component::<ScrollContent>().unwrap().scroll.offset.y > 0., !explicit);
        }
        for explicit in [false, true] {
            let mut h = ordinary(explicit, false);
            let response = h.event(&ContentEvent::Key { key: ControlKey::Tab { reverse: false }, down: true });
            assert!(response.consumed);
            assert_eq!(response.changed, !explicit);
            assert_eq!(h.focused(), Some(if explicit {OLD} else {TARGET}));
            // Tab may commit a real Scroll move; refresh normal geometry before re-entry.
            render(&mut h);
            h.focus(None, false);
            assert_eq!(h.enter_focus(false), !explicit);
            assert_eq!(h.focused(), if explicit {None} else {Some(TARGET)});
        }
        let mut stale = ordinary(false, false);
        let owner = stale.get(TARGET).unwrap();
        let ActionPreparation::Ready(action) = owner.component.prepare_action(ComponentActionRequest::Accessibility {
            part: None, action: &AccessibilityAction::Press,
        }) else { panic!("original Button has a prepared decision"); };
        let route = ContentRoute::capture(&stale.root, TARGET, HostRevealBudget::default().max_path_nodes).unwrap();
        let actions = [OwnerAction { route, action }];
        assert_eq!(stale.validate_action_owners(&actions, None), None, "current ordinary wrapper lacks the prepared-hook contract");
        stale.get_mut(TARGET).unwrap().component_mut::<ButtonContent>().unwrap().activations += 1;
        assert_eq!(stale.validate_action_owners(&actions, None), Some(false), "unknown ancestry cannot mask a stale original decision");
        let mut pointer_max = ordinary(false, false);
        assert!(pointer_max.focus(Some(TARGET), true));
        render(&mut pointer_max);
        assert!(pointer_max.focus(Some(OLD), true));
        let point = pointer_max.get(TARGET).unwrap().frame.center();
        assert!(pointer_max.get(TARGET).unwrap().visible().is_some_and(|r| r.contains(point)));
        let button = pointer_max.get_mut(TARGET).unwrap().component_mut::<ButtonContent>().unwrap();
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
            if failure == 0 { h.focus_epoch = u64::MAX; }
            if failure == 2 { h.get_mut(TARGET).unwrap().component_mut::<ButtonContent>().unwrap().activations = usize::MAX; }
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
}
