//! Finite Content adapter over the original Checkbox and its ButtonModel.
use super::action_proxy::Exposure;
use super::*;
use crate::{CheckState, Checkbox, CheckboxLayout, widgets::CheckboxPrepared};

/// Checked value captured at delivery. Authorization is the original published mount.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckboxIntent {
    checked: CheckState,
    mount: u64,
}
impl CheckboxIntent {
    pub fn checked(&self) -> CheckState {
        self.checked
    }
}

/// A single original three-state Checkbox, with a fixed (possibly empty) label.
#[derive(Debug)]
pub struct CheckboxContent {
    model: Checkbox,
    label: String,
    prepared: CheckboxPrepared,
    exposure: Exposure,
    published_mount: Option<u64>,
    runnable: bool,
    window_active: bool,
    last: Option<CheckboxIntent>,
}
impl CheckboxContent {
    pub fn new(label: impl Into<String>, checked: CheckState) -> Self {
        Self::from_original(
            label,
            Checkbox {
                checked,
                ..Default::default()
            },
        )
    }
    pub fn from_original(label: impl Into<String>, mut model: Checkbox) -> Self {
        model.clear_layout();
        Self {
            model,
            label: label.into(),
            prepared: CheckboxPrepared::default(),
            exposure: Exposure::default(),
            published_mount: None,
            runnable: true,
            window_active: true,
            last: None,
        }
    }
    pub fn model(&self) -> &Checkbox {
        &self.model
    }
    pub fn layout(&self) -> CheckboxLayout {
        self.prepared.layout
    }
    pub fn set_checked(&mut self, checked: CheckState) {
        if self.model.checked == checked {
            return;
        }
        self.model.button.cancel();
        self.model.checked = checked;
    }
    pub fn set_availability(&mut self, disabled: bool, loading: bool) {
        if (
            self.model.button.state.disabled,
            self.model.button.state.loading,
        ) == (disabled, loading)
        {
            return;
        }
        self.revoke();
        self.model.button.state.disabled = disabled;
        self.model.button.state.loading = loading;
    }
    pub fn set_error(&mut self, error: bool) {
        self.model.button.state.error = error;
    }
    fn ready(&self) -> bool {
        self.runnable
            && self.window_active
            && !self.model.button.state.disabled
            && !self.model.button.state.loading
            && self.prepared.layout.visible.is_some()
    }
    fn available(&self) -> bool {
        self.ready() && !self.exposure.pending() && self.published_mount.is_some()
    }
    fn revoke(&mut self) {
        self.exposure.revoke();
        self.published_mount = None;
        self.model.button.cancel();
    }
    fn receipt(&mut self, activated: bool) -> ContentResponse {
        if activated {
            self.last = Some(CheckboxIntent {
                checked: self.model.checked,
                mount: self.published_mount.expect("available original root"),
            });
        }
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
}
impl Component for CheckboxContent {
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(
            self, request, Self::decide_action, Self::apply_action,
            |d| (d.response, if d.response.consumed {
                PreparedActionFocus::Owner
            } else { PreparedActionFocus::None }),
        )
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Checkbox::measure(
            &mut TextAdapter(text, None, true),
            &self.label,
            available.width,
            proposal::TextRole::UiBody,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let previous = self.prepared.layout.visible.is_some();
        self.prepared = self.model.prepare_layout(
            &mut TextAdapter(text, None, true),
            &self.label,
            frame,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        if previous && self.prepared.layout.visible.is_none() {
            self.revoke();
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        self.model.paint_prepared(
            scene,
            &mut TextAdapter(text, None, true),
            &self.label,
            self.prepared,
            theme.palette,
            proposal::TextRole::UiBody,
            theme.surface(),
        );
    }
    fn pointer_regions(&self, _frame: Rect, _clip: Rect) -> Vec<Rect> {
        if self.available() {
            self.prepared.layout.visible.into_iter().collect()
        } else {
            vec![]
        }
    }
    fn focusable(&self) -> bool {
        self.available()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused && self.available() {
            self.model.button.state.focused = true;
            self.model.button.state.focus_visible = visible;
        } else {
            self.model.button.blur();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        if self.window_active && !active {
            self.revoke();
        }
        self.window_active = active;
        self.model
            .button
            .set_window_active(active, std::time::Duration::ZERO);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        if self.runnable && !runnable {
            self.revoke();
            self.model.clear_layout();
            self.prepared = CheckboxPrepared::default();
        }
        self.runnable = runnable;
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.revoke();
            self.model.clear_layout();
            self.prepared = CheckboxPrepared::default();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Checkbox, &self.label);
        s.selected = self.model.checked == CheckState::Checked;
        s.value = Some(
            match self.model.checked {
                CheckState::Unchecked => "off",
                CheckState::Checked => "on",
                CheckState::Mixed => "mixed",
            }
            .into(),
        );
        // Configuration/runnable availability is distinct from a temporarily revoked
        // callback lease. In particular an original Popover anchor stays enabled.
        s.disabled = !self.ready();
        s.invalid = self.model.button.state.error;
        s
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last = None;
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || !self.available() {
            return ContentResponse::default();
        }
        let Some(decision) = self.decide_action(
            &action_preparation::OwnedActionRequest::Accessibility(part, action.clone()),
        ) else { return ContentResponse::default(); };
        let response = decision.response;
        self.apply_action(decision);
        response
    }
    fn action_payload(&self, _kind: ContentActionKind) -> Option<ContentActionPayload> {
        self.last.clone().map(ContentActionPayload::CheckboxState)
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        matches!(payload, ContentActionPayload::CheckboxState(intent)
            if self.available() && self.published_mount == Some(intent.mount))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last = None;
        if matches!(event, ContentEvent::Cancel) {
            self.model.button.cancel();
            return ContentResponse {
                changed: true,
                ..Default::default()
            };
        }
        if !self.available() {
            return ContentResponse::default();
        }
        match event {
            ContentEvent::PointerMove(point) => {
                let before = self.model.button.state;
                self.model.pointer_move_at(*point);
                ContentResponse {
                    changed: before != self.model.button.state,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(point) => {
                let Some(decision) = self.decide_action(
                    &action_preparation::OwnedActionRequest::Pointer(ContentEvent::PointerDown(*point)),
                ) else { return ContentResponse::default(); };
                let response = decision.response;
                self.apply_action(decision);
                response
            }
            ContentEvent::PointerUp(point) => {
                let activated = self.model.pointer_up_at(*point);
                let mut response = self.receipt(activated);
                response.changed = true;
                response
            }
            ContentEvent::Key { key, down } => {
                let before = self.model.button.state;
                let activated = self.model.key(*key, *down);
                let mut response = self.receipt(activated);
                response.consumed |= *key == ControlKey::Space;
                response.changed |= before != self.model.button.state;
                response
            }
            _ => ContentResponse::default(),
        }
    }
}
/// Both ordinary delivery and strict admission consume the original model decision.
#[derive(Debug, PartialEq)]
struct PreparedCheckboxAction {
    decision: crate::widgets::CheckboxActionDecision,
    mount: u64,
    response: ContentResponse,
}
impl action_preparation::OriginalActionDecision for PreparedCheckboxAction {}
impl CheckboxContent {
    fn decide_action(&self, request: &action_preparation::OwnedActionRequest) -> Option<PreparedCheckboxAction> {
        if !self.available() { return None; }
        let pointer = matches!(request, action_preparation::OwnedActionRequest::Pointer(_));
        let decision = match request {
            action_preparation::OwnedActionRequest::Pointer(event) => {
                let routed = event.routing_event();
                let ContentEvent::PointerDown(point) = routed.as_ref() else { return None; };
                self.model.prepare_content_down(*point)
            }
            action_preparation::OwnedActionRequest::Accessibility(part, action) => {
                if part.is_some() || !matches!(action, AccessibilityAction::Press) { return None; }
                self.model.prepare_content_press()
            }
        };
        let response = ContentResponse {
            consumed: decision.consumed, changed: decision.consumed,
            activated: decision.activated, capture_pointer: pointer && decision.consumed,
            ..Default::default()
        };
        Some(PreparedCheckboxAction {
            decision, mount: self.published_mount.expect("available original root"), response,
        })
    }
    fn apply_action(&mut self, decision: PreparedCheckboxAction) {
        self.last = None;
        self.model.apply_content_action(decision.decision);
        if decision.response.activated {
            self.last = Some(CheckboxIntent { checked: self.model.checked, mount: decision.mount });
        }
    }
}
impl ContentHost {
    // Reserve the complete batch on the original allocator before publishing
    // any root. Exhaustion leaves all revoked owners pending and cannot wrap.
    pub(super) fn reconcile_checkbox_scope_exposures(&mut self) {
        let scope = self.scope().id;
        fn revoke_outside(
            node: &mut Content,
            scope: ComponentId,
            inside: bool,
            retired: &mut Vec<ComponentId>,
        ) {
            let inside = inside || node.id == scope;
            if !inside && let Some(check) = node.component_mut::<CheckboxContent>() {
                check.revoke();
                check.model.button.blur();
                retired.push(node.id);
            }
            if let Some(children) = node.component.children_mut() {
                for child in children {
                    revoke_outside(child, scope, inside, retired);
                }
            }
        }
        let mut retired = Vec::new();
        revoke_outside(&mut self.root, scope, false, &mut retired);
        if self.captured.is_some_and(|id| retired.contains(&id)) {
            self.captured = None;
            self.captured_mount = None;
            self.captured_overlay = false;
        }
    }
    pub(super) fn publish_checkbox_exposures(&mut self) -> Result<(), CompositionError> {
        fn collect(
            node: &Content,
            scope: ComponentId,
            inside: bool,
            out: &mut Vec<(ComponentId, bool)>,
        ) {
            if !node.active() {
                return;
            }
            let inside = inside || node.id == scope;
            if let Some(check) = node.component::<CheckboxContent>()
                && inside
                && check.ready()
                && (check.exposure.pending() || check.published_mount.is_none())
            {
                out.push((node.id, check.exposure.pending()));
            }
            for child in node.component.children() {
                collect(child, scope, inside, out);
            }
        }
        let mut roots = Vec::new();
        collect(&self.root, self.scope().id, false, &mut roots);
        let count = roots.iter().filter(|(_, renew)| *renew).count();
        let mut tickets = actions::reserve_mounts(count).ok_or(CompositionError::MountExhausted)?;
        for (id, renew) in roots {
            if renew {
                self.cancel_subtree(id);
            }
            let node = self.root.find_mut(id).expect("collected original root");
            if renew {
                node.mount = tickets.take();
            }
            let mount = node.mount;
            let check = node
                .component_mut::<CheckboxContent>()
                .expect("collected Checkbox");
            check.published_mount = Some(mount);
            check.exposure.published();
        }
        debug_assert!(tickets.exhausted());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Text {
        measured: usize,
        drawn: usize,
        requested: usize,
    }
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
            self.measured += 1;
            Size::new(w.min(80.), 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
            self.drawn += 1;
        }
        fn request_animation_frame(&mut self) {
            self.requested += 1;
        }
    }
    #[test]
    fn checkbox_paint_never_rearranges_or_reconciles_original_button() {
        let mut c = CheckboxContent::new("label", crate::CheckState::Mixed);
        let mut t = Text::default();
        let f = Rect::new(0., 0., 200., 50.);
        c.arrange(&mut t, f, f);
        let measured = t.measured;
        c.model.button.state.focused = true;
        c.model.button.key(ControlKey::Space, true);
        let before = format!("{:?}", c.model);
        let layout = c.layout();
        for _ in 0..3 {
            c.paint(
                &mut Scene::new(),
                &mut t,
                f,
                f,
                ContentTheme::from(Palette::from(tekes_ui_tokens::Theme::LIGHT)),
            );
        }
        assert_eq!(t.measured, measured);
        assert_eq!(t.drawn, 3);
        assert_eq!(t.requested, 0);
        assert_eq!(format!("{:?}", c.model), before);
        assert_eq!(c.layout(), layout);
    }
    #[test]
    fn checkbox_ancestor_inactive_restore_revokes_even_without_intermediate_paint() {
        let root = Content::new(
            ComponentId(1),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![Content::new(
                    ComponentId(2),
                    CheckboxContent::new("label", crate::CheckState::Mixed),
                )],
            },
        );
        let mut h = ContentHost::new(root).unwrap();
        let f = Rect::new(0., 0., 200., 80.);
        let mut t = Text::default();
        h.render(
            &mut Scene::new(),
            &mut t,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let old = h.semantic_mount(ComponentId(2)).unwrap();
        h.root.set_composition_active(false);
        h.reconcile_active_inputs();
        h.root.set_composition_active(true);
        h.reconcile_active_inputs();
        h.render(
            &mut Scene::new(),
            &mut t,
            f,
            f,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert_ne!(h.semantic_mount(ComponentId(2)), Some(old));
    }
}
