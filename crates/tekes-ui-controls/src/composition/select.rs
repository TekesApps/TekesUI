//! Fixed-label projection of the original Select and SelectView owners.
use super::*;
use crate::layout_view::{
    SelectLayout, SelectPreparedAction, SelectPreparedInput, SelectPreparedState, SelectView,
};
use crate::{ButtonModel, ButtonSize, MenuEvent, MenuWheelInput, Select};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectUpdateError {
    IdentityExhausted,
}
impl std::fmt::Display for SelectUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("select identity exhausted")
    }
}
impl std::error::Error for SelectUpdateError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Parts {
    group: usize,
    rows: Vec<usize>,
}
impl Parts {
    fn reserve(count: usize) -> Result<Self, SelectUpdateError> {
        let fail = || SelectUpdateError::IdentityExhausted;
        let total = count.checked_add(1).ok_or_else(fail)?;
        // Reservation uses the original allocator. Failed namespace conversion
        // leaves permanent unpublished holes; it never rolls the counter back.
        let mut range = actions::reserve_mounts(total).ok_or_else(fail)?;
        let high = 1usize << (usize::BITS - 1);
        let token = |id| {
            usize::try_from(id)
                .ok()
                .filter(|id| *id < high)
                .and_then(|id| id.checked_add(high))
                .ok_or_else(fail)
        };
        let group = token(range.take())?;
        let rows = (0..count)
            .map(|_| token(range.take()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { group, rows })
    }
}

/// Fixed string options; the original model alone owns selection and open state.
#[derive(Debug)]
pub struct SelectContent {
    identity: std::sync::Arc<()>,
    name: String,
    labels: Vec<String>,
    model: Select,
    view: SelectView,
    revision: u64,
    authority: Option<u64>,
    parts: Option<Parts>,
    reserved: Option<Parts>,
    generation: u64,
    selection: Option<MenuSelection>,
    disabled: bool,
    error: bool,
    last_frame: Option<std::time::Duration>,
}
impl SelectContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self {
        let count = labels.len();
        Self {
            identity: std::sync::Arc::new(()),
            name: name.into(),
            labels,
            model: Select::new(count),
            view: SelectView::new(count),
            revision: 1,
            authority: Some(1),
            parts: None,
            reserved: None,
            generation: 0,
            selection: None,
            disabled: false,
            error: false,
            last_frame: None,
        }
    }
    pub fn labels(&self) -> &[String] {
        &self.labels
    }
    pub fn model(&self) -> &Select {
        &self.model
    }
    pub fn layout(&self) -> &SelectLayout {
        self.view.layout()
    }
    pub fn set_labels(&mut self, labels: Vec<String>) -> Result<bool, SelectUpdateError> {
        if labels == self.labels {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SelectUpdateError::IdentityExhausted)?;
        let authority = self
            .authority
            .and_then(|n| n.checked_add(1))
            .ok_or(SelectUpdateError::IdentityExhausted)?;
        self.view
            .next_open_generation()
            .ok_or(SelectUpdateError::IdentityExhausted)?;
        let parts = Parts::reserve(labels.len())?;
        // All explicit failures precede any owner/layout/authorization mutation.
        self.view.replace_options(&mut self.model, labels.len());
        self.labels = labels;
        self.revision = revision;
        self.authority = Some(authority);
        self.parts = None;
        self.reserved = Some(parts);
        self.selection = None;
        Ok(true)
    }
    pub fn set_selected(&mut self, index: usize) -> bool {
        if index >= self.labels.len() {
            return false;
        }
        let Some(authority) = self.authority.and_then(|n| n.checked_add(1)) else {
            return false;
        };
        self.view.cancel(&mut self.model);
        self.parts = None;
        self.selection = None;
        self.authority = Some(authority);
        self.model.choose(index)
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        self.view.set_disabled(disabled, &mut self.model);
        if disabled {
            self.revoke();
        }
    }
    pub fn set_error(&mut self, error: bool) {
        self.error = error;
        self.view.set_error(error);
    }
    fn title(&self) -> &str {
        self.labels
            .get(self.model.selected.min(self.labels.len().saturating_sub(1)))
            .map_or("", String::as_str)
    }
    fn revoke(&mut self) {
        self.view.cancel(&mut self.model);
        self.parts = None;
        self.selection = None;
        self.authority = self.authority.and_then(|n| n.checked_add(1));
        self.last_frame = None;
    }
    fn prepare_open(&mut self) -> bool {
        if self.disabled || self.labels.is_empty() || self.authority.is_none() {
            return false;
        }
        if self.model.menu.open {
            return true;
        }
        if self.view.next_open_generation().is_none() {
            return false;
        }
        if self.reserved.is_none() {
            let Ok(parts) = Parts::reserve(self.labels.len()) else {
                return false;
            };
            self.reserved = Some(parts);
        }
        true
    }
    fn opened(&mut self, was_open: bool) {
        if !was_open && self.model.menu.open {
            self.generation = self.view.open_generation();
            self.parts = self.reserved.take();
        }
    }
    fn response(&mut self, event: MenuEvent, consumed: bool) -> ContentResponse {
        if let MenuEvent::Activated { index, .. } = event {
            self.selection = Some(MenuSelection {
                context_request: None,
                index,
                label: self.labels[index].clone(),
                model_revision: self.authority.expect("admitted authority"),
                value_path: None,
            });
            return ContentResponse {
                consumed: true,
                changed: true,
                activated: true,
                ..Default::default()
            };
        }
        ContentResponse {
            consumed,
            changed: consumed,
            ..Default::default()
        }
    }
    fn index(&self, token: usize) -> Option<usize> {
        if !self.overlay_open() || self.generation != self.view.open_generation() {
            return None;
        }
        self.parts.as_ref()?.rows.iter().position(|id| *id == token)
    }
}
/// One original owner decision plus an unpublished identity reservation.
#[derive(Debug)]
struct PreparedSelectEffect {
    identity: std::sync::Arc<()>,
    stamp: SelectStamp,
    input: SelectPreparedInput,
    decision: SelectPreparedAction,
    parts: Option<Parts>,
    selection: Option<MenuSelection>,
}
impl PreparedActionEffect for PreparedSelectEffect {
    fn validate(&self, owner: &dyn Component) -> bool {
        let Some(owner) = (owner as &dyn Any).downcast_ref::<SelectContent>() else {
            return false;
        };
        std::sync::Arc::ptr_eq(&self.identity, &owner.identity)
            && self.stamp == owner.prepared_stamp()
            && owner
                .view
                .prepare_select_action(&owner.model, self.input.clone())
                .as_ref()
                == Some(&self.decision)
    }
    fn apply(self: Box<Self>, owner: &mut dyn Component) {
        let owner = (owner as &mut dyn Any)
            .downcast_mut::<SelectContent>()
            .expect("validated original Select owner");
        let Self {
            decision,
            parts,
            selection,
            ..
        } = *self;
        let was_open = owner.model.menu.open;
        if let Some(parts) = parts {
            owner.reserved = Some(parts);
        }
        owner.view.apply_select_action(&mut owner.model, decision);
        owner.opened(was_open);
        if let Some(selection) = selection {
            owner.selection = Some(selection);
        }
    }
}
#[derive(Debug)]
struct RejectedSelectEffect {
    identity: std::sync::Arc<()>,
    expected: (SelectStamp, SelectPreparedState),
}
impl PreparedActionEffect for RejectedSelectEffect {
    fn validate(&self, owner: &dyn Component) -> bool {
        (owner as &dyn Any)
            .downcast_ref::<SelectContent>()
            .is_some_and(|owner| {
                std::sync::Arc::ptr_eq(&self.identity, &owner.identity)
                    && self.expected
                        == (
                            owner.prepared_stamp(),
                            owner.view.prepared_state(&owner.model),
                        )
            })
    }
    fn apply(self: Box<Self>, _owner: &mut dyn Component) {}
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectStamp {
    labels: Vec<String>,
    revision: u64,
    authority: Option<u64>,
    generation: u64,
    parts: Option<Parts>,
    reserved: Option<Parts>,
    selection: Option<MenuSelection>,
    disabled: bool,
    error: bool,
}
impl SelectContent {
    fn prepared_stamp(&self) -> SelectStamp {
        SelectStamp {
            labels: self.labels.clone(),
            revision: self.revision,
            authority: self.authority,
            generation: self.generation,
            parts: self.parts.clone(),
            reserved: self.reserved.clone(),
            selection: self.selection.clone(),
            disabled: self.disabled,
            error: self.error,
        }
    }
    fn prepared_effect(
        &self,
        request: ComponentActionRequest<'_>,
    ) -> Option<(
        ContentResponse,
        PreparedActionFocus,
        Box<PreparedSelectEffect>,
    )> {
        if !self.focusable() {
            return None;
        }
        let input = match request {
            ComponentActionRequest::PointerDown(event) => {
                let routed = event.routing_event();
                if !event.is_primary_pointer() {
                    return None;
                }
                let ContentEvent::PointerDown(point) = routed.as_ref() else {
                    return None;
                };
                SelectPreparedInput::PointerDown(*point)
            }
            ComponentActionRequest::Accessibility { part, action } => match (part, action) {
                (None, AccessibilityAction::Press) => {
                    if self.model.menu.open {
                        SelectPreparedInput::Close
                    } else {
                        SelectPreparedInput::Open
                    }
                }
                (None, AccessibilityAction::SetExpanded(true)) => SelectPreparedInput::Open,
                (None, AccessibilityAction::SetExpanded(false)) => SelectPreparedInput::Close,
                (Some(token), AccessibilityAction::Focus) => {
                    let index = self.index(token)?;
                    SelectPreparedInput::Focus(index)
                }
                (Some(token), AccessibilityAction::Press) => {
                    let index = self.index(token)?;
                    SelectPreparedInput::Activate(index)
                }
                _ => return None,
            },
        };
        let decision = self
            .view
            .prepare_select_action(&self.model, input.clone())?;
        let parts = if decision.reserve_open && self.reserved.is_none() {
            let Ok(parts) = Parts::reserve(self.labels.len()) else {
                return None;
            };
            Some(parts)
        } else {
            None
        };
        let selection = if let MenuEvent::Activated { index, .. } = decision.event {
            Some(MenuSelection {
                context_request: None,
                index,
                label: self.labels[index].clone(),
                model_revision: self.authority.expect("available authority"),
                value_path: None,
            })
        } else {
            None
        };
        let response = ContentResponse {
            consumed: decision.consumed,
            changed: decision.consumed,
            activated: selection.is_some(),
            capture_pointer: matches!(input, SelectPreparedInput::PointerDown(_))
                && decision.consumed,
            ..Default::default()
        };
        let focus = if response.consumed {
            PreparedActionFocus::Owner
        } else {
            PreparedActionFocus::None
        };
        Some((
            response,
            focus,
            Box::new(PreparedSelectEffect {
                identity: self.identity.clone(),
                stamp: self.prepared_stamp(),
                input,
                decision,
                parts,
                selection,
            }),
        ))
    }
    fn prepare_original_select(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        match self.prepared_effect(request) {
            Some((response, focus, effect)) => {
                ActionPreparation::Ready(PreparedComponentAction::new(response, focus, effect))
            }
            None => {
                let known = match request {
                    ComponentActionRequest::PointerDown(event) => {
                        event.is_primary_pointer()
                            && matches!(
                                event.routing_event().as_ref(),
                                ContentEvent::PointerDown(_)
                            )
                    }
                    ComponentActionRequest::Accessibility { part: None, action } => matches!(
                        action,
                        AccessibilityAction::Press | AccessibilityAction::SetExpanded(_)
                    ),
                    ComponentActionRequest::Accessibility {
                        part: Some(_),
                        action,
                    } => matches!(
                        action,
                        AccessibilityAction::Press | AccessibilityAction::Focus
                    ),
                };
                if !known {
                    return ActionPreparation::Unsupported;
                }
                ActionPreparation::Ready(PreparedComponentAction::new(
                    ContentResponse::default(),
                    PreparedActionFocus::None,
                    Box::new(RejectedSelectEffect {
                        identity: self.identity.clone(),
                        expected: (self.prepared_stamp(), self.view.prepared_state(&self.model)),
                    }),
                ))
            }
        }
    }
}
impl Component for SelectContent {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        self.prepare_original_select(request)
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(ButtonModel::measure(
            &mut TextAdapter(text),
            self.title(),
            available.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated surface");
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.view.render_trigger(
            scene,
            &mut TextAdapter(text),
            &mut self.model,
            &labels,
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
        );
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if let Some(context) = text.motion_context() {
            self.view.set_reduced_motion(context.reduced_motion);
            if let Some(previous) = self.last_frame.replace(context.now) {
                self.view
                    .tick(context.now.saturating_sub(previous).as_secs_f32());
            }
        }
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.view.render_options(
            scene,
            &mut TextAdapter(text),
            &mut self.model,
            &labels,
            clip,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.view.needs_animation() {
            text.request_animation_frame();
        }
    }
    fn focusable(&self) -> bool {
        !self.disabled && !self.labels.is_empty() && self.authority.is_some()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(visible);
        } else {
            self.view.blur(&mut self.model);
            self.parts = None;
        }
    }
    fn overlay_open(&self) -> bool {
        self.focusable() && self.model.menu.open
    }
    fn overlay_presented(&self) -> bool {
        self.model.menu.open || self.view.is_presented()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Select, &self.name);
        s.value = Some(self.title().into());
        s.expanded = Some(self.overlay_open());
        s.disabled = !self.focusable();
        s.invalid = self.error;
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.overlay_open() || self.generation != self.view.open_generation() {
            return vec![];
        }
        let (Some(parts), Some(layout)) = (&self.parts, &self.view.layout().popup) else {
            return vec![];
        };
        let mut result = vec![SemanticPart {
            index: parts.group,
            frame: layout.popup,
            focused: false,
            semantics: ContentSemantics::new(ContentRole::Menu, ""),
        }];
        result.extend(layout.items.iter().zip(&parts.rows).enumerate().map(
            |(index, (row, token))| {
                let mut semantics =
                    ContentSemantics::new(ContentRole::MenuItem, &self.labels[index]);
                semantics.selected = self.model.selected == index;
                SemanticPart {
                    index: *token,
                    frame: row.frame,
                    focused: layout.active_item == index,
                    semantics,
                }
            },
        ));
        result
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        self.index(part)
            .and_then(|_| self.parts.as_ref().map(|p| p.group))
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let Some((response, _, effect)) =
            self.prepared_effect(ComponentActionRequest::Accessibility { part, action })
        else {
            return ContentResponse::default();
        };
        // Same-owner ordinary delivery has no Host transfer. It still consumes
        // the same original checked decision, never replays a mutable handler.
        if !effect.validate(self) {
            return ContentResponse::default();
        }
        effect.apply(self);
        response
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.revoke();
            return ContentResponse::consumed(true);
        }
        if !self.focusable() {
            return ContentResponse::default();
        }
        let was_open = self.model.menu.open;
        if !was_open
            && matches!(
                event,
                ContentEvent::PointerDown(_)
                    | ContentEvent::PointerUp(_)
                    | ContentEvent::Key {
                        key: ControlKey::Enter | ControlKey::Space,
                        ..
                    }
            )
            && !self.prepare_open()
        {
            return ContentResponse::default();
        }
        let result = match event {
            ContentEvent::PointerMove(p) => {
                let changed = self.view.pointer_move_changed(*p, &mut self.model);
                return ContentResponse {
                    consumed: was_open,
                    changed,
                    ..Default::default()
                };
            }
            ContentEvent::PointerDown(p) => {
                let capture = self.view.pointer_down(*p, &mut self.model);
                return ContentResponse {
                    consumed: capture,
                    changed: capture,
                    capture_pointer: capture,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up(*p, &mut self.model),
            ContentEvent::Key { key, down } => self.view.key(*key, *down, &mut self.model),
            ContentEvent::Scroll {
                position,
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
            } => {
                let handled = matches!(
                    self.view.wheel_event(
                        MenuWheelInput {
                            point: *position,
                            delta: *delta,
                            precise: *precise,
                            begin: *begin,
                            end: *end,
                            momentum: *momentum,
                            time: *time
                        },
                        &self.model
                    ),
                    crate::MenuWheelEvent::Consumed { .. }
                );
                return ContentResponse {
                    consumed: handled,
                    changed: handled,
                    ..Default::default()
                };
            }
            _ => return ContentResponse::default(),
        };
        self.opened(was_open);
        let consumed = was_open || self.model.menu.open;
        let mut response = self.response(result, consumed);
        if matches!(
            event,
            ContentEvent::Key {
                key: ControlKey::Tab { .. },
                ..
            }
        ) {
            response.consumed = false;
        }
        response
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.event(event)
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| {
                self.selection
                    .clone()
                    .map(ContentActionPayload::MenuSelection)
            })
            .flatten()
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        matches!(payload, ContentActionPayload::MenuSelection(s) if self.authority == Some(s.model_revision))
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.revoke();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.view.set_window_active(active, &mut self.model);
        if !active {
            self.revoke();
        }
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        if !runnable {
            self.revoke();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn select() -> SelectContent {
        SelectContent::new("Mode", vec!["One".into(), "Two".into()])
    }
    #[test]
    fn checked_revision_failure_and_equal_noop_preserve_full_owner() {
        let mut s = select();
        s.revision = u64::MAX;
        let old = format!("{s:?}");
        assert_eq!(s.set_labels(vec!["One".into(), "Two".into()]), Ok(false));
        assert_eq!(format!("{s:?}"), old);
        assert_eq!(
            s.set_labels(vec!["B".into()]),
            Err(SelectUpdateError::IdentityExhausted)
        );
        assert_eq!(format!("{s:?}"), old);
    }
    #[test]
    fn checked_allocator_failure_preserves_owner_and_never_wraps() {
        let mut s = select();
        let old = format!("{s:?}");
        let (_, counter) = actions::select_test_mount_boundary(u64::MAX, || {
            assert_eq!(
                s.set_labels(vec!["B".into()]),
                Err(SelectUpdateError::IdentityExhausted)
            );
        });
        assert_eq!(counter, u64::MAX);
        assert_eq!(format!("{s:?}"), old);
    }
    #[test]
    fn checked_namespace_failure_consumes_only_unpublished_reservation() {
        let mut s = select();
        let old = format!("{s:?}");
        let high = 1u64 << (usize::BITS - 1);
        let (_, counter) = actions::select_test_mount_boundary(high - 1, || {
            assert_eq!(
                s.set_labels(vec!["B".into()]),
                Err(SelectUpdateError::IdentityExhausted)
            );
        });
        assert_eq!(counter, high + 1);
        assert_eq!(format!("{s:?}"), old);
    }
    #[test]
    fn checked_part_count_overflow_does_not_allocate() {
        let (_, counter) =
            actions::select_test_mount_boundary(5, || assert!(Parts::reserve(usize::MAX).is_err()));
        assert_eq!(counter, 5);
    }
    #[test]
    fn checked_equal_declaration_allocates_nothing_at_exhaustion() {
        let mut s = select();
        let (_, counter) = actions::select_test_mount_boundary(u64::MAX, || {
            assert_eq!(s.set_labels(vec!["One".into(), "Two".into()]), Ok(false));
        });
        assert_eq!(counter, u64::MAX);
    }
    #[test]
    fn checked_generation_failure_preserves_full_owner_on_replace_and_open() {
        let mut s = select();
        s.view.select_test_generation(u64::MAX);
        let old = format!("{s:?}");
        assert_eq!(
            s.set_labels(vec!["B".into()]),
            Err(SelectUpdateError::IdentityExhausted)
        );
        assert_eq!(format!("{s:?}"), old);
        assert!(
            !s.accessibility_action(None, &AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(format!("{s:?}"), old);
    }
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 8.).min(w), 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    fn paint(s: &mut SelectContent) {
        let clip = Rect::new(0., 0., 320., 240.);
        let theme = ContentTheme::from(tekes_ui_tokens::Palette::from(
            tekes_ui_tokens::Theme::LIGHT,
        ));
        s.paint(
            &mut Scene::new(),
            &mut Text,
            Rect::new(20., 20., 200., 40.),
            clip,
            theme,
        );
        s.paint_overlay(
            &mut Scene::new(),
            &mut Text,
            Rect::new(20., 20., 200., 40.),
            clip,
            theme,
        );
    }
    #[test]
    fn checked_replace_at_generation_limit_keeps_old_popup_parts_and_armed_selection() {
        let mut s = select();
        paint(&mut s);
        s.view.select_test_generation(u64::MAX - 1);
        assert!(
            s.accessibility_action(None, &AccessibilityAction::Press)
                .consumed
        );
        paint(&mut s);
        let token = s.semantic_parts()[2].index;
        let p = s.layout().popup.as_ref().unwrap().items[1].frame.center();
        assert!(s.event(&ContentEvent::PointerDown(p)).capture_pointer);
        let old = format!("{s:?}");
        assert_eq!(
            s.set_labels(vec!["B".into()]),
            Err(SelectUpdateError::IdentityExhausted)
        );
        assert_eq!(format!("{s:?}"), old);
        assert_eq!(s.index(token), Some(1));
        assert!(s.event(&ContentEvent::PointerUp(p)).activated);
        assert_eq!(s.model.selected, 1);
    }
    #[test]
    fn checked_replace_revision_failure_retains_pending_original_host_queue() {
        let id = ComponentId(89);
        let mut h = ContentHost::new(Content::new(id, select())).unwrap();
        h.set_action_events(true);
        let frame = Rect::new(0., 0., 320., 240.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        h.focus(Some(id), true);
        h.accessibility_action(id, None, AccessibilityAction::Press);
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let token = h
            .get(id)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .semantic_parts()[2]
            .index;
        assert!(
            h.accessibility_action(id, Some(token), AccessibilityAction::Press)
                .activated
        );
        let queue = format!("{:?}", h.actions);
        let s = h
            .get_mut(id)
            .unwrap()
            .component_mut::<SelectContent>()
            .unwrap();
        s.revision = u64::MAX;
        let old = format!("{s:?}");
        assert_eq!(
            s.set_labels(vec!["New".into()]),
            Err(SelectUpdateError::IdentityExhausted)
        );
        assert_eq!(format!("{s:?}"), old);
        assert_eq!(format!("{:?}", h.actions), queue);
        assert_eq!(h.drain_action_records().len(), 1);
        assert!(h.drain_actions().is_empty());
    }

    #[test]
    fn rejected_pointer_focus_capacity_does_not_arm_or_open_original_select() {
        let id = ComponentId(89);
        let mut h = ContentHost::new(Content::new(id, select())).unwrap();
        let f = Rect::new(0., 0., 320., 240.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        h.focus_epoch = u64::MAX;
        let before = format!("{:?}", h.get(id));
        assert!(
            !h.event(&ContentEvent::PointerDown(Point::new(20., 20.)))
                .consumed
        );
        assert_eq!(format!("{:?}", h.get(id)), before);
        h.event(&ContentEvent::PointerUp(Point::new(20., 20.)));
        assert!(
            !h.get(id)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model
                .menu
                .open
        );
        assert!(h.pointer_capture().is_none());
    }
    #[test]
    fn failed_open_identity_preparation_cannot_transfer_focus_or_mutate_owner() {
        let id = ComponentId(89);
        let mut h = ContentHost::new(Content::new(id, select())).unwrap();
        let f = Rect::new(0., 0., 320., 240.);
        h.render(
            &mut Scene::new(),
            &mut Text,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let before = format!("{:?}", h.get(id));
        let epoch = h.focus_epoch;
        actions::select_test_mount_boundary(u64::MAX, || {
            assert!(
                !h.accessibility_action(id, None, AccessibilityAction::Press)
                    .consumed
            );
        });
        assert_eq!(h.focused(), None);
        assert_eq!(h.focus_epoch, epoch);
        assert_eq!(format!("{:?}", h.get(id)), before);
    }
    #[test]
    fn checked_replace_failure_preserves_original_scroll_gesture_and_parts() {
        let mut s = SelectContent::new("Mode", (0..40).map(|i| format!("Item {i}")).collect());
        paint(&mut s);
        assert!(
            s.accessibility_action(None, &AccessibilityAction::Press)
                .consumed
        );
        paint(&mut s);
        let point = s.layout().popup.as_ref().unwrap().visible.unwrap().center();
        let wheel = |begin, end, time| ContentEvent::Scroll {
            position: point,
            delta: Point::new(0., -20.),
            precise: true,
            begin,
            end,
            momentum: false,
            time,
        };
        assert!(s.event(&wheel(true, false, 1.)).consumed);
        s.revision = u64::MAX;
        let token = s.semantic_parts()[1].index;
        let before = format!("{s:?}");
        assert_eq!(
            s.set_labels(vec!["New".into()]),
            Err(SelectUpdateError::IdentityExhausted)
        );
        assert_eq!(format!("{s:?}"), before);
        assert_eq!(s.index(token), Some(0));
        assert!(s.event(&wheel(false, true, 2.)).consumed);
    }
    #[test]
    fn final_original_select_effect_validation_rejects_changed_owner_and_preparation_is_pure() {
        let mut s = select();
        paint(&mut s);
        let before = format!("{s:?}");
        let (_, _, effect) = s
            .prepared_effect(ComponentActionRequest::Accessibility {
                part: None,
                action: &AccessibilityAction::Press,
            })
            .unwrap();
        assert_eq!(format!("{s:?}"), before);
        assert!(effect.validate(&s));
        s.set_labels(vec!["Changed".into()]).unwrap();
        assert!(!effect.validate(&s));
        assert!(!s.model.menu.open);
    }
    #[test]
    fn dropped_preparation_leaves_exact_unpublished_hole_and_validation_never_reserves() {
        let mut s = select();
        paint(&mut s);
        let before = format!("{s:?}");
        let (_, next) = actions::select_test_mount_boundary(100, || {
            let (_, _, effect) = s
                .prepared_effect(ComponentActionRequest::Accessibility {
                    part: None,
                    action: &AccessibilityAction::Press,
                })
                .unwrap();
            assert!(effect.validate(&s));
            assert!(effect.validate(&s));
            assert_eq!(format!("{s:?}"), before);
            drop(effect);
        });
        // One group + two rows: [100,103), unpublished and never rolled back.
        assert_eq!(next, 103);
        let (_, after) = actions::select_test_mount_boundary(next, || {
            let mut range = actions::reserve_mounts(1).unwrap();
            assert_eq!(range.take(), 103);
        });
        assert_eq!(after, 104);
        assert_eq!(format!("{s:?}"), before);
    }
    // Original red retained as a regression against accepted98 preparation:
    // no owner apply may follow a refused focus epoch/reveal preflight.
    #[test]
    fn required_red_refused_host_focus_must_not_open_select() {
        let id = ComponentId(89);
        let mut h = ContentHost::new(Content::new(id, select())).unwrap();
        h.focus_epoch = u64::MAX;
        let response = h.accessibility_action(id, None, AccessibilityAction::Press);
        assert!(
            !response.consumed,
            "Host invoked Select after rejecting focus capacity"
        );
        assert!(
            !h.get(id)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model
                .menu
                .open
        );
    }
}
