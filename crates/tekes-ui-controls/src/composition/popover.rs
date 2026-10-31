use super::*;
use crate::extended::Popover;
use crate::feedback_view::{OverlayEvent, OverlayLayout, PopoverView};

/// Capability for one opening of one original mounted owner, not a business ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopoverSession {
    owner: ComponentId,
    mount: u64,
    generation: u64,
}
impl PopoverSession {
    pub fn owner(self) -> ComponentId {
        self.owner
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverCloseReason {
    Requested,
    OutsidePointer,
    Escape,
    Dismiss,
    Disabled,
    WindowDeactivated,
    AnchorHidden,
    AnchorUnmounted,
    AnchorReplaced,
    OwnerHidden,
    OwnerUnmounted,
    Cancelled,
}
impl PopoverCloseReason {
    fn returns_focus(self) -> bool {
        matches!(
            self,
            Self::Requested | Self::OutsidePointer | Self::Escape | Self::Dismiss
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopoverClosed {
    pub session: PopoverSession,
    pub reason: PopoverCloseReason,
}

/// Original Content in the existing Popover shell. ContentHost resolves the
/// original anchor after ordinary layout; consumers never calculate offsets or
/// forward pointer/text/AX events into a second host.
#[derive(Debug)]
pub struct PopoverContent {
    pub name: String,
    pub initial_focus: Option<ComponentId>,
    /// Optional body constraint. The existing shell owns insets/placement/clip.
    pub max_content_width: Option<f32>,
    model: Popover,
    view: PopoverView,
    body: Vec<Content>,
    enabled: bool,
    anchor: Option<(ComponentId, u64)>,
    restore: Option<(ComponentId, u64)>,
    session: Option<PopoverSession>,
    generation: u64,
    paint_session: Option<PopoverSession>,
    closed: Vec<PopoverClosed>,
    last_close: Option<PopoverCloseReason>,
    viewport: Rect,
    clip: Rect,
    anchor_frame: Rect,
    size: Size,
    dismiss_focused: bool,
}
impl PopoverContent {
    pub fn new(name: impl Into<String>, body: Content) -> Self {
        Self {
            name: name.into(),
            initial_focus: None,
            max_content_width: None,
            model: Popover::default(),
            view: PopoverView::default(),
            body: vec![body],
            enabled: true,
            anchor: None,
            restore: None,
            session: None,
            generation: 0,
            paint_session: None,
            closed: Vec::new(),
            last_close: None,
            viewport: Rect::default(),
            clip: Rect::default(),
            anchor_frame: Rect::default(),
            size: Size::default(),
            dismiss_focused: false,
        }
    }
    pub fn is_open(&self) -> bool {
        self.model.open
    }
    pub fn is_presented(&self) -> bool {
        self.model.open || self.view.is_presented()
    }
    pub fn needs_animation(&self) -> bool {
        self.view.needs_animation()
    }
    pub fn current_session(&self) -> Option<PopoverSession> {
        self.session
    }
    pub fn layout(&self) -> OverlayLayout {
        self.view.layout()
    }
    pub fn set_dismiss_visible(&mut self, visible: bool) {
        self.view.set_dismiss_visible(visible);
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.close(PopoverCloseReason::Disabled);
        }
        self.enabled = enabled;
        self.view.set_disabled(!enabled, &mut self.model);
    }
    pub(super) fn host_window_active(&mut self, active: bool) {
        if !active {
            self.close(PopoverCloseReason::WindowDeactivated);
        }
        self.view.set_window_active(active, &mut self.model);
    }
    /// Once-only semantic closure notifications, separate from business actions.
    pub fn take_closed(&mut self) -> Vec<PopoverClosed> {
        std::mem::take(&mut self.closed)
    }
    pub fn clear_presentation(&mut self) {
        self.view.clear_presentation();
    }
    pub(super) fn return_focus(&self) -> Option<(ComponentId, u64)> {
        self.restore
    }
    fn record_close(&mut self, reason: PopoverCloseReason) {
        if let Some(session) = self.session.take() {
            self.closed.push(PopoverClosed { session, reason });
            self.last_close = Some(reason);
        }
    }
    pub(super) fn close(&mut self, reason: PopoverCloseReason) -> bool {
        if !self.model.open {
            return false;
        }
        self.view.close(&mut self.model);
        self.record_close(reason);
        if matches!(
            reason,
            PopoverCloseReason::OwnerHidden | PopoverCloseReason::OwnerUnmounted
        ) {
            self.view.clear_presentation();
        }
        true
    }
    fn place(&mut self, text: &mut dyn ButtonTextRenderer, viewport: Rect, clip: Rect) {
        self.viewport = viewport;
        self.clip = clip;
        if !self.is_presented() {
            return;
        }
        let mut text = TextAdapter(text);
        // Ask the existing layout for available content geometry, then measure
        // the original body and arrange it once at the final SDK placement.
        let available = self.view.arrange(
            &mut text,
            &self.model,
            self.anchor_frame,
            viewport.size,
            viewport,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        let mut constraints = available.content.size;
        if let Some(width) = self.max_content_width {
            constraints.width = constraints.width.min(finite(width));
        }
        let body = self.body[0].measure(&mut text, constraints).size;
        self.size = self
            .view
            .measure_content(&mut text, body, proposal::TextRole::UiBody);
        let layout = self.view.arrange(
            &mut text,
            &self.model,
            self.anchor_frame,
            self.size,
            viewport,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        self.body[0].arrange(
            &mut text,
            layout.content,
            layout.visible.unwrap_or_default(),
        );
    }
}
impl Component for PopoverContent {
    // The existing SDK shell has arranged the original body inside this frame.
    // Like Dialog, the active input scope keeps those relative child bounds
    // fixed while an inner Scroll body performs a pure focus reveal.
    fn host_reveal_passthrough(&self) -> bool {
        self.model.open && !self.view.layout().frame.is_empty()
    }
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        Some(self.view.layout().frame)
    }

    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }

    fn default_surface(&self) -> Option<&'static str> {
        Some("popover")
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else if self.body.len() != 1 {
            Err(CompositionError::InvalidSlotCount)
        } else {
            Ok(())
        }
    }
    fn children(&self) -> &[Content] {
        &self.body
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.body)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_open(&self) -> bool {
        self.model.open
    }
    fn overlay_presented(&self) -> bool {
        self.is_presented()
    }
    fn overlay_scope(&self) -> Option<OverlayScope> {
        Some(OverlayScope {
            initial_focus: self.initial_focus,
        })
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated Popover Surface");
        let body = &mut self.body[0];
        let result = self.view.try_render_with_text_content(
            scene,
            &mut TextAdapter(text),
            &self.model,
            self.anchor_frame,
            self.size,
            self.viewport,
            Some(self.clip),
            theme.palette,
            proposal::TextRole::UiBody,
            "original-content",
            |scene, text, _| {
                body.paint(scene, text, theme);
                body.paint_overlays(scene, text, theme);
            },
        );
        if let Err(error) = result {
            scene.record_composition_error(error);
            return;
        }
        if self.view.needs_animation() {
            text.request_animation_frame();
        }
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        let was_open = self.model.open;
        let mut reason = PopoverCloseReason::Dismiss;
        let mut key_armed = false;
        let result = match event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p);
                return ContentResponse {
                    changed: true,
                    ..Default::default()
                };
            }
            ContentEvent::PointerDown(p) => {
                let handled = self.view.pointer_down(&mut self.model, *p);
                return ContentResponse {
                    consumed: handled,
                    changed: handled,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => {
                if !self
                    .view
                    .layout()
                    .visible
                    .is_some_and(|frame| frame.contains(*p))
                {
                    reason = PopoverCloseReason::OutsidePointer;
                }
                self.view.pointer_up(&mut self.model, *p)
            }
            ContentEvent::Key { key, down } => {
                key_armed =
                    self.dismiss_focused && matches!(key, ControlKey::Space | ControlKey::Enter);
                if *key == ControlKey::Escape {
                    reason = PopoverCloseReason::Escape;
                }
                self.view.key_event(&mut self.model, *key, *down)
            }
            _ => OverlayEvent::None,
        };
        if was_open && !self.model.open {
            self.record_close(reason);
        }
        if matches!(result, OverlayEvent::Dismissed { .. }) || key_armed {
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.view.cancel();
        }
        ContentResponse::default()
    }
    fn focusable(&self) -> bool {
        self.model.open && self.view.logical_focus_target().is_some()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.dismiss_focused = focused;
        self.view.focus_dismiss(focused, visible);
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.view.layout().frame
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, &self.name);
        s.disabled = !self.enabled;
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.model.open {
            return Vec::new();
        }
        self.view
            .logical_focus_target()
            .map(|frame| SemanticPart {
                index: 0,
                frame,
                focused: self.dismiss_focused,
                semantics: ContentSemantics::new(ContentRole::Button, "Dismiss"),
            })
            .into_iter()
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if self.model.open
            && part == Some(0)
            && self.view.logical_focus_target().is_some()
            && matches!(action, AccessibilityAction::Press)
        {
            self.close(PopoverCloseReason::Dismiss);
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.close(PopoverCloseReason::OwnerUnmounted);
            self.view.clear_presentation();
        }
    }
}

impl Content {
    pub(super) fn popover_ids(&self, ids: &mut Vec<ComponentId>) {
        if self.component::<PopoverContent>().is_some() {
            ids.push(self.id);
        }
        for child in self.component.children() {
            child.popover_ids(ids);
        }
    }
    fn popover_ancestor(
        &self,
        target: ComponentId,
        parent: Option<ComponentId>,
    ) -> Option<ComponentId> {
        let parent = if self.component::<PopoverContent>().is_some() {
            Some(self.id)
        } else {
            parent
        };
        if self.id == target {
            return parent;
        }
        self.component
            .children()
            .iter()
            .find_map(|child| child.popover_ancestor(target, parent))
    }
}
impl ContentHost {
    /// Original anchor and return intent are captured here, never from a caller's
    /// numeric stand-in or screen rectangle. Calling again while open preserves IME.
    pub fn show_popover(
        &mut self,
        owner: ComponentId,
        anchor: ComponentId,
    ) -> Result<PopoverSession, CompositionError> {
        let existing = self
            .root
            .find(owner)
            .and_then(|node| node.component::<PopoverContent>())
            .ok_or(CompositionError::NotPopover(owner))?;
        if let Some(session) = existing.session {
            let matches = existing.anchor.is_some_and(|(id, mount)| {
                id == anchor
                    && self.root.find_active(id).is_some_and(|node| {
                        node.mount == mount && !node.component.semantics().disabled
                    })
            });
            return if self.window_active
                && existing.enabled
                && matches
                && self.root.find_in_active_ancestors(owner).is_some()
            {
                Ok(session)
            } else {
                Err(CompositionError::PopoverUnavailable(owner))
            };
        }
        if self.root.find_in_active_ancestors(owner).is_none() {
            return Err(CompositionError::PopoverUnavailable(owner));
        }
        let invalid = CompositionError::InvalidPopoverAnchor { owner, anchor };
        let source = self
            .scope()
            .find_active(anchor)
            .filter(|node| !node.component.semantics().disabled)
            .ok_or(invalid)?;
        let binding = (anchor, source.mount);
        let restore = if source.component.focusable() {
            Some(binding)
        } else {
            self.capture_focus()
        };
        let node = self
            .root
            .find(owner)
            .ok_or(CompositionError::NotPopover(owner))?;
        if node.find(anchor).is_some() {
            return Err(invalid);
        }
        let mut dependency = self.root.popover_ancestor(anchor, None);
        let mut seen = BTreeSet::new();
        while let Some(id) = dependency {
            if id == owner || !seen.insert(id) {
                return Err(invalid);
            }
            dependency = self
                .root
                .find(id)
                .and_then(|node| node.component::<PopoverContent>())
                .and_then(|popup| popup.anchor)
                .and_then(|(id, _)| self.root.popover_ancestor(id, None));
        }
        let mount = node.mount;
        let popup = self
            .root
            .find_mut(owner)
            .unwrap()
            .component_mut::<PopoverContent>()
            .ok_or(CompositionError::NotPopover(owner))?;
        if !self.window_active || !popup.enabled {
            return Err(CompositionError::PopoverUnavailable(owner));
        }
        if let Some(session) = popup.session {
            return if popup.anchor == Some(binding) {
                Ok(session)
            } else {
                Err(CompositionError::PopoverUnavailable(owner))
            };
        }
        popup.generation = popup
            .generation
            .checked_add(1)
            .expect("popover opening identity exhausted");
        let session = PopoverSession {
            owner,
            mount,
            generation: popup.generation,
        };
        popup.anchor = Some(binding);
        popup.restore = restore;
        popup.session = Some(session);
        popup.paint_session = Some(session);
        popup.last_close = None;
        popup.view.set_window_active(true, &mut popup.model);
        popup
            .view
            .show(&mut popup.model, usize::try_from(anchor.0).unwrap_or(0));
        self.popover_order.push(session);
        self.reconcile_checkbox_scope_exposures();
        self.dirty = true;
        Ok(session)
    }
    /// A stale completion from an earlier opening or replaced owner cannot close
    /// the current form. Saving/error updates stay in the original body tree.
    pub fn close_popover(&mut self, session: PopoverSession) -> bool {
        let restore = self
            .root
            .find(session.owner)
            .filter(|node| node.mount == session.mount)
            .and_then(|node| node.component::<PopoverContent>())
            .and_then(PopoverContent::return_focus);
        let closed = self
            .root
            .find_mut(session.owner)
            .filter(|node| node.mount == session.mount)
            .and_then(|node| node.component_mut::<PopoverContent>())
            .filter(|popup| popup.session == Some(session))
            .is_some_and(|popup| popup.close(PopoverCloseReason::Requested));
        if closed {
            self.reconcile_active_inputs();
            // Closing an ancestor may retire the active nested popup first.
            // Finish the explicit original owner's intent only if no newer
            // eligible focus owner has already taken over.
            if self.focused.is_none() {
                self.restore_focus(restore);
            }
            self.dirty = true;
        }
        closed
    }
    fn anchor_state(&self, owner: ComponentId) -> Result<Rect, PopoverCloseReason> {
        let node = self
            .root
            .find_in_active_ancestors(owner)
            .ok_or(PopoverCloseReason::OwnerHidden)?;
        let popup = node
            .component::<PopoverContent>()
            .expect("original Popover owner");
        let (id, mount) = popup.anchor.ok_or(PopoverCloseReason::AnchorUnmounted)?;
        let anchor = self
            .root
            .find(id)
            .ok_or(PopoverCloseReason::AnchorUnmounted)?;
        if anchor.mount != mount {
            return Err(PopoverCloseReason::AnchorReplaced);
        }
        if self.root.find_active(id).is_none() || anchor.component.semantics().disabled {
            return Err(PopoverCloseReason::AnchorHidden);
        }
        let frame = anchor.component.semantic_frame(anchor.frame);
        if frame
            .intersection(&anchor.clip)
            .is_none_or(|frame| frame.is_empty())
        {
            return Err(PopoverCloseReason::AnchorHidden);
        }
        Ok(frame)
    }
    pub(super) fn standard_popover_scope(&self) -> Option<ComponentId> {
        self.popover_order.iter().rev().find_map(|session| {
            self.base_scope()
                .find_active(session.owner)
                .filter(|node| node.mount == session.mount)
                .and_then(|node| node.component::<PopoverContent>())
                .filter(|popup| popup.session == Some(*session))
                .map(|_| session.owner)
        })
    }
    pub(super) fn popover_animation_pending(&self) -> bool {
        self.popover_order.iter().any(|session| {
            self.root
                .find(session.owner)
                .filter(|node| node.mount == session.mount)
                .and_then(|node| node.component::<PopoverContent>())
                .is_some_and(|popup| {
                    popup.paint_session == Some(*session) && popup.needs_animation()
                })
        })
    }
    pub(super) fn paint_registered_popovers(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        // A retiring nonmodal popup must never paint above a newly opened Dialog.
        let scope = self
            .modal
            .and_then(|(id, _)| self.root.find(id))
            .unwrap_or(&self.root);
        self.popover_order.retain(|session| {
            scope
                .find_in_active_ancestors(session.owner)
                .filter(|node| node.mount == session.mount)
                .and_then(|node| node.component::<PopoverContent>())
                .is_some_and(|popup| popup.paint_session == Some(*session) && popup.is_presented())
        });
        for session in self.popover_order.clone() {
            let theme = self
                .root
                .theme_for(session.owner, theme)
                .expect("original popover theme");
            if let Some(node) = self.root.find_mut(session.owner) {
                node.component
                    .paint_overlay(scene, text, node.frame, node.clip, theme);
            }
        }
    }
    pub(super) fn reconcile_popovers(&mut self) {
        let mut ids = Vec::new();
        self.root.popover_ids(&mut ids);
        for id in ids {
            let open = self
                .root
                .find(id)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .is_open();
            let invalid = open.then(|| self.anchor_state(id).err()).flatten();
            let popup = self
                .root
                .find_mut(id)
                .unwrap()
                .component_mut::<PopoverContent>()
                .unwrap();
            if let Some(reason) = invalid {
                popup.close(reason);
            }
            if popup
                .last_close
                .is_some_and(|reason| !reason.returns_focus())
            {
                for scope in &mut self.overlay_scopes {
                    if scope.id == id {
                        scope.restore = None;
                    }
                }
            }
        }
    }
    pub(super) fn arrange_popovers(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        viewport: Rect,
        clip: Rect,
    ) {
        let mut pending = Vec::new();
        self.root.popover_ids(&mut pending);
        while !pending.is_empty() {
            let index = pending.iter().position(|id| {
                let popup = self
                    .root
                    .find(*id)
                    .unwrap()
                    .component::<PopoverContent>()
                    .unwrap();
                !popup.is_open()
                    || popup
                        .anchor
                        .and_then(|(anchor, _)| self.root.popover_ancestor(anchor, None))
                        .is_none_or(|dependency| !pending.contains(&dependency))
            });
            let Some(index) = index else {
                // Defensive against externally altered trees creating a cycle.
                for id in pending {
                    self.root
                        .find_mut(id)
                        .unwrap()
                        .component_mut::<PopoverContent>()
                        .unwrap()
                        .close(PopoverCloseReason::AnchorHidden);
                }
                break;
            };
            let id = pending.remove(index);
            let state = self.anchor_state(id);
            let node = self.root.find_mut(id).unwrap();
            let owner_clip = node.clip.intersection(&clip).unwrap_or_default();
            let popup = node.component_mut::<PopoverContent>().unwrap();
            match state {
                Ok(anchor) => popup.anchor_frame = anchor,
                Err(reason) => {
                    popup.close(reason);
                }
            }
            if owner_clip.is_empty() {
                popup.close(PopoverCloseReason::OwnerHidden);
            }
            popup.place(text, viewport, owner_clip);
            node.frame = popup.layout().frame;
        }
    }
}

impl ContentHost {
    pub(super) fn replacement_keeps_popovers(
        &self,
        retired: &Content,
        closing_modal: bool,
    ) -> bool {
        let mut ids = Vec::new();
        self.root.popover_ids(&mut ids);
        for id in ids {
            let popup = self
                .root
                .find(id)
                .unwrap()
                .component::<PopoverContent>()
                .unwrap();
            if popup.is_open()
                && (closing_modal
                    || self.anchor_state(id).is_err()
                    || popup
                        .anchor
                        .is_some_and(|(id, _)| retired.find(id).is_some()))
            {
                return false;
            }
            if popup.last_close.is_some_and(|r| !r.returns_focus())
                && self
                    .overlay_scopes
                    .iter()
                    .any(|s| s.id == id && s.restore.is_some())
            {
                return false;
            }
        }
        true
    }
}

/// One original anchor and one passive Content slot, in this same ContentHost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverPreviewVersion {
    owner: ComponentId,
    owner_mount: u64,
    preview: ComponentId,
    preview_mount: u64,
    generation: u64,
}
impl HoverPreviewVersion {
    pub fn owner(self) -> ComponentId {
        self.owner
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverPreviewError {
    NotPreview,
    OwnerUnavailable,
    StaleVersion,
    Composition(CompositionError),
}
/// Noninteractive original Content in the existing Popover shell. No overlay
/// input scope, focus target, synthetic trigger or second ContentHost is created.
#[derive(Debug)]
pub struct HoverPreviewContent {
    popup: PopoverContent,
    version: Option<HoverPreviewVersion>,
    hovered: bool,
    blocked: bool,
    owner_frame: Option<Rect>,
}
impl HoverPreviewContent {
    pub fn new(name: impl Into<String>, body: Content) -> Result<Self, CompositionError> {
        validate_passive_preview(&body)?;
        let mut popup = PopoverContent::new(name, body);
        popup.set_dismiss_visible(false);
        popup
            .view
            .set_placement(crate::feedback_view::PopoverPlacement::RightStart);
        popup.validate()?;
        Ok(Self {
            popup,
            version: None,
            hovered: false,
            blocked: false,
            owner_frame: None,
        })
    }
    pub fn set_placement(&mut self, placement: crate::feedback_view::PopoverPlacement) {
        self.popup.view.set_placement(placement);
    }
    pub fn set_max_content_width(&mut self, width: Option<f32>) {
        self.popup.max_content_width = width;
    }
    pub fn is_open(&self) -> bool {
        self.popup.is_open()
    }
    pub fn layout(&self) -> OverlayLayout {
        self.popup.layout()
    }
    pub fn version(&self) -> Option<HoverPreviewVersion> {
        self.version
    }
    fn retire(&mut self, blocked: bool) {
        self.popup.view.close(&mut self.popup.model);
        self.popup.view.clear_presentation();
        self.blocked |= blocked;
        self.owner_frame = None;
    }
}
fn validate_passive_preview(body: &Content) -> Result<(), CompositionError> {
    if let Some(child) = body.interactive_id() {
        return Err(CompositionError::NestedInteractive {
            action: body.id,
            child,
        });
    }
    Ok(())
}
impl Component for HoverPreviewContent {
    fn validate(&self) -> Result<(), CompositionError> {
        self.popup.validate()?;
        validate_passive_preview(&self.popup.body[0])
    }
    fn default_surface(&self) -> Option<&'static str> {
        Some("popover")
    }
    fn children(&self) -> &[Content] {
        &self.popup.body
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.popup.body)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn active(&self) -> bool {
        self.popup.is_open()
    }
    fn overlay_presented(&self) -> bool {
        self.popup.is_open()
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::default())
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.popup.paint_overlay(scene, text, frame, clip, theme);
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.popup.layout().frame
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, &self.popup.name)
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.retire(true);
        }
        ContentResponse::default()
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.retire(true);
            self.version = None;
            self.hovered = false;
        }
    }
}
impl Content {
    fn hover_preview_ids(&self, ids: &mut Vec<ComponentId>) {
        if self.component::<HoverPreviewContent>().is_some() {
            ids.push(self.id);
        }
        for child in self.component.children() {
            child.hover_preview_ids(ids);
        }
    }
    fn preview_body_ids(&self, ids: &mut BTreeSet<ComponentId>) {
        ids.insert(self.id);
        for child in self.component.children() {
            child.preview_body_ids(ids);
        }
    }
}
impl ContentHost {
    /// Bind current original mounts. Removal/reinsertion requires a fresh bind.
    pub fn bind_hover_preview(
        &mut self,
        owner: ComponentId,
        preview: ComponentId,
    ) -> Result<HoverPreviewVersion, HoverPreviewError> {
        let anchor = self
            .scope()
            .find_active(owner)
            .filter(|node| !node.component.semantics().disabled)
            .ok_or(HoverPreviewError::OwnerUnavailable)?;
        let owner_mount = anchor.mount;
        let node = self
            .scope()
            .find_in_active_ancestors(preview)
            .ok_or(HoverPreviewError::OwnerUnavailable)?;
        let popup = node
            .component::<HoverPreviewContent>()
            .ok_or(HoverPreviewError::NotPreview)?;
        if owner == preview || popup.popup.body[0].find(owner).is_some() {
            return Err(HoverPreviewError::OwnerUnavailable);
        }
        let version = HoverPreviewVersion {
            owner,
            owner_mount,
            preview,
            preview_mount: node.mount,
            generation: actions::next_mount(),
        };
        let popup = self
            .root
            .find_mut(preview)
            .unwrap()
            .component_mut::<HoverPreviewContent>()
            .unwrap();
        popup.retire(false);
        popup.version = Some(version);
        popup.hovered = false;
        popup.blocked = false;
        self.dirty = true;
        Ok(version)
    }
    fn preview_anchor(&self, version: HoverPreviewVersion) -> Option<Rect> {
        let owner = self.scope().find_active(version.owner)?;
        let preview = self.scope().find_in_active_ancestors(version.preview)?;
        if owner.mount != version.owner_mount
            || preview.mount != version.preview_mount
            || owner.component.semantics().disabled
        {
            return None;
        }
        owner
            .component
            .semantic_frame(owner.frame)
            .intersection(&owner.clip)
            .filter(|frame| !frame.is_empty())
    }
    fn preview_versions(&self) -> Vec<HoverPreviewVersion> {
        let mut ids = Vec::new();
        self.root.hover_preview_ids(&mut ids);
        ids.into_iter()
            .filter_map(|id| {
                self.root
                    .find(id)?
                    .component::<HoverPreviewContent>()?
                    .version
            })
            .collect()
    }
    /// Replace the passive original subtree only if this exact owner/mount/data
    /// version is current. Failed validation leaves the existing subtree intact.
    /// A static pointer may keep showing a newly accepted streaming version.
    pub fn update_hover_preview(
        &mut self,
        version: HoverPreviewVersion,
        mut body: Content,
    ) -> Result<HoverPreviewVersion, HoverPreviewError> {
        let popup = self
            .root
            .find(version.preview)
            .filter(|node| node.mount == version.preview_mount)
            .and_then(|node| node.component::<HoverPreviewContent>())
            .filter(|popup| popup.version == Some(version))
            .ok_or(HoverPreviewError::StaleVersion)?;
        if self.preview_anchor(version).is_none() {
            return Err(HoverPreviewError::OwnerUnavailable);
        }
        validate_passive_preview(&body).map_err(HoverPreviewError::Composition)?;
        let mut existing = BTreeSet::new();
        self.root
            .ids(&mut existing)
            .map_err(HoverPreviewError::Composition)?;
        let mut retired = BTreeSet::new();
        popup.popup.body[0].preview_body_ids(&mut retired);
        existing.retain(|id| !retired.contains(id));
        body.ids(&mut existing)
            .map_err(HoverPreviewError::Composition)?;
        let next = HoverPreviewVersion {
            generation: actions::next_mount(),
            ..version
        };
        let popup = self
            .root
            .find_mut(version.preview)
            .unwrap()
            .component_mut::<HoverPreviewContent>()
            .unwrap();
        popup.popup.body[0].lifecycle(Lifecycle::Unmounted);
        body.lifecycle(Lifecycle::Mounted);
        popup.popup.body[0] = body;
        let anchor = popup.owner_frame;
        popup.retire(false);
        popup.owner_frame = anchor;
        popup.version = Some(next);
        self.last_layout = None;
        self.dirty = true;
        Ok(next)
    }
    pub(super) fn reconcile_hover_previews(&mut self) {
        for version in self.preview_versions() {
            let anchor = self.preview_anchor(version);
            let available = self.window_active
                && self.captured.is_none()
                && self.scope().overlay_id().is_none();
            let popup = self
                .root
                .find_mut(version.preview)
                .unwrap()
                .component_mut::<HoverPreviewContent>()
                .unwrap();
            if !available
                || anchor.is_none()
                || popup.owner_frame.is_some_and(|frame| Some(frame) != anchor)
            {
                popup.retire(true);
            }
        }
    }
    pub(super) fn hover_preview_event(&mut self, event: &ContentEvent) -> bool {
        let versions = self.preview_versions();
        let routing = event.routing_event();
        match routing.as_ref() {
            ContentEvent::PointerMove(point) if event.is_primary_pointer() => {
                let owners = versions
                    .iter()
                    .map(|version| version.owner)
                    .collect::<Vec<_>>();
                let hit = self.scope().anchor_path_at(*point, &owners);
                let active = hit.and_then(|path| {
                    path.into_iter()
                        .find(|id| versions.iter().any(|version| version.owner == *id))
                });
                let available = self.window_active
                    && self.captured.is_none()
                    && self.scope().overlay_id().is_none()
                    && self.last_layout.is_some();
                for version in versions {
                    let frame = self.preview_anchor(version);
                    let popup = self
                        .root
                        .find_mut(version.preview)
                        .unwrap()
                        .component_mut::<HoverPreviewContent>()
                        .unwrap();
                    let hovered = active == Some(version.owner) && frame.is_some();
                    if !hovered {
                        popup.retire(false);
                        popup.hovered = false;
                        popup.blocked = false;
                    } else {
                        if !popup.hovered {
                            popup.blocked = false;
                        }
                        popup.hovered = true;
                        if available && !popup.blocked {
                            popup.owner_frame = frame;
                            popup.popup.anchor_frame = frame.unwrap();
                            popup.popup.view.show(
                                &mut popup.popup.model,
                                usize::try_from(version.owner.0).unwrap_or(0),
                            );
                        }
                    }
                }
                self.dirty = true;
            }
            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            } => {
                let visible = versions.iter().any(|version| {
                    self.root
                        .find(version.preview)
                        .unwrap()
                        .component::<HoverPreviewContent>()
                        .unwrap()
                        .is_open()
                });
                for version in versions {
                    self.root
                        .find_mut(version.preview)
                        .unwrap()
                        .component_mut::<HoverPreviewContent>()
                        .unwrap()
                        .retire(true);
                }
                let composing = self
                    .focused_text()
                    .is_some_and(|surface| surface.input.marked.is_some())
                    || self
                        .focused
                        .and_then(|id| self.root.find(id))
                        .is_some_and(|node| node.component.composing());
                self.dirty |= visible;
                return visible && !composing;
            }
            ContentEvent::Scroll { .. }
            | ContentEvent::PointerDown(_)
            | ContentEvent::PointerUp(_)
            | ContentEvent::Cancel => {
                for version in versions {
                    self.root
                        .find_mut(version.preview)
                        .unwrap()
                        .component_mut::<HoverPreviewContent>()
                        .unwrap()
                        .retire(true);
                }
            }
            _ => {}
        }
        false
    }
    pub(super) fn arrange_hover_previews(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        viewport: Rect,
        clip: Rect,
    ) {
        self.reconcile_hover_previews();
        for version in self.preview_versions() {
            let anchor = self.preview_anchor(version);
            let popup = self
                .root
                .find_mut(version.preview)
                .unwrap()
                .component_mut::<HoverPreviewContent>()
                .unwrap();
            if let Some(anchor) = anchor.filter(|_| popup.hovered && !popup.blocked) {
                popup.owner_frame = Some(anchor);
                popup.popup.view.show(
                    &mut popup.popup.model,
                    usize::try_from(version.owner.0).unwrap_or(0),
                );
                popup.popup.anchor_frame = anchor;
                popup.popup.place(text, viewport, clip);
            }
        }
    }
}
