//! Finite adapter retaining the original ComboBox reducer, View and one editor.
//! Pointer/AX transactions consume the accepted original Host preparation gate.
//! Root semantics use the accepted matched-schema editable ComboBox bridge.
use super::*;
use crate::{TextSurface, extended::ComboBox, layout_view::ComboBoxView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboBoxUpdateError {
    IdentityExhausted,
    Composing,
}
/// Immutable original source selection captured at delivery, never polled later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboBoxSelection {
    index: usize,
    label: String,
    declaration_revision: u64,
    authorization: u64,
}
impl ComboBoxSelection {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn declaration_revision(&self) -> u64 {
        self.declaration_revision
    }
}
#[derive(Clone, Debug, PartialEq)]
struct CandidatePlan {
    parts: Vec<usize>,
    group: usize,
    generation: Option<u64>,
}
#[derive(Debug)]
pub struct ComboBoxContent {
    editor: TextFieldContent,
    model: ComboBox,
    view: ComboBoxView,
    candidates: Option<CandidatePlan>,
    declaration: u64,
    payload_authority: Option<u64>,
    last_selection: Option<ComboBoxSelection>,
    window_active: bool,
    runnable: bool,
    last_frame: Option<std::time::Duration>,
}
impl ComboBoxContent {
    pub fn new(name: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            editor: TextFieldContent::new(name, ""),
            model: ComboBox {
                items,
                ..Default::default()
            },
            view: ComboBoxView::default(),
            candidates: None,
            declaration: 0,
            payload_authority: None,
            last_selection: None,
            window_active: true,
            runnable: true,
            last_frame: None,
        }
    }
    pub fn model(&self) -> &ComboBox {
        &self.model
    }
    pub fn text_surface(&self) -> &TextSurface {
        &self.editor.surface
    }
    pub fn popup_layout(&self) -> Option<&crate::MenuLayout> {
        self.view.options.layout()
    }
    fn available(&self) -> bool {
        !self.model.disabled && self.window_active && self.runnable
    }
    fn popup_available(&self) -> bool {
        self.available() && self.editor.has_focus()
    }
    fn prepare_candidates(
        &self,
        count: usize,
        open: bool,
    ) -> Result<CandidatePlan, ComboBoxUpdateError> {
        let generation = if open && count != 0 {
            Some(
                self.view
                    .next_open_generation()
                    .ok_or(ComboBoxUpdateError::IdentityExhausted)?,
            )
        } else {
            None
        };
        let total = count
            .checked_add(1)
            .ok_or(ComboBoxUpdateError::IdentityExhausted)?;
        let mut range =
            actions::reserve_mounts(total).ok_or(ComboBoxUpdateError::IdentityExhausted)?;
        let group =
            usize::try_from(range.take()).map_err(|_| ComboBoxUpdateError::IdentityExhausted)?;
        let mut parts = Vec::with_capacity(count);
        for _ in 0..count {
            parts.push(
                usize::try_from(range.take())
                    .map_err(|_| ComboBoxUpdateError::IdentityExhausted)?,
            );
        }
        Ok(CandidatePlan {
            parts,
            group,
            generation,
        })
    }
    fn publish_candidates(&mut self, plan: CandidatePlan) {
        if !self.popup_available() {
            self.revoke_candidates();
            return;
        }
        self.view.revoke_candidates();
        if let Some(generation) = plan.generation {
            self.view.apply_open_generation(generation);
        }
        self.candidates = Some(plan);
    }
    fn revoke_candidates(&mut self) {
        self.candidates = None;
        self.view.revoke_candidates();
    }
    fn revoke(&mut self) {
        self.model.open = false;
        self.revoke_candidates();
        self.payload_authority = None;
        self.last_selection = None;
    }
    pub fn set_items(&mut self, items: Vec<String>) -> Result<bool, ComboBoxUpdateError> {
        if self.model.items == items {
            return Ok(false);
        }
        let declaration = self
            .declaration
            .checked_add(1)
            .ok_or(ComboBoxUpdateError::IdentityExhausted)?;
        let plan =
            self.prepare_candidates(items.len(), self.model.open && self.popup_available())?;
        // All deterministic identity failures are before any original owner mutation.
        let payload_authority = plan.group as u64;
        self.model.items = items;
        self.declaration = declaration;
        self.payload_authority = Some(payload_authority);
        self.last_selection = None;
        self.publish_candidates(plan);
        Ok(true)
    }
    pub fn set_query(&mut self, query: impl Into<String>) -> Result<bool, ComboBoxUpdateError> {
        if self.model.disabled {
            return Ok(false);
        }
        let query = query.into();
        let different = self.editor.surface.input.text != query;
        if different && self.editor.composing() {
            return Err(ComboBoxUpdateError::Composing);
        }
        let editor = if different {
            Some(
                self.editor
                    .surface
                    .input
                    .prepare_insert(&query, Some(0..self.editor.surface.input.len()))
                    .ok_or(ComboBoxUpdateError::IdentityExhausted)?,
            )
        } else {
            None
        };
        let plan = self.prepare_candidates(self.model.items.len(), self.popup_available())?;
        let before = self.model.clone();
        let payload_authority = plan.group as u64;
        if let Some(editor) = editor {
            self.editor.surface.input.apply_insert(editor);
            self.editor.surface.clauses.clear();
            self.editor.surface.reveal();
        }
        self.model.set_query(self.editor.surface.input.text.clone());
        self.payload_authority = Some(payload_authority); // explicit program setter revokes older queued selections
        self.last_selection = None;
        self.publish_candidates(plan);
        Ok(before != self.model || different)
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        if self.model.disabled == disabled {
            return;
        }
        self.model.disabled = disabled;
        self.editor.surface.input.disabled = disabled;
        self.revoke(); // void revocation never allocates a new identity
        self.editor.surface.cancel_interaction();
    }
    pub fn set_error(&mut self, error: Option<String>) {
        self.editor.field.error = error;
    }
    /// Real editor success cannot be rolled back because popup capacity ran out.
    fn sync_real_edit(&mut self) {
        if self.model.query == self.editor.surface.input.text {
            return;
        }
        if !self.popup_available() {
            self.model.set_query(self.editor.surface.input.text.clone());
            self.revoke_candidates();
            return;
        }
        let plan = self.prepare_candidates(self.model.items.len(), true);
        self.model.set_query(self.editor.surface.input.text.clone());
        match plan {
            Ok(plan) => {
                if self.payload_authority.is_none() {
                    self.payload_authority = Some(plan.group as u64);
                }
                self.publish_candidates(plan);
            }
            Err(_) => self.revoke_candidates(), // query remains synchronized; no actionable publication
        }
    }
    fn editor_event(&mut self, event: &ContentEvent) -> ContentResponse {
        let r = self.editor.event(event);
        self.sync_real_edit();
        r
    }
    fn confirm(&mut self, index: usize) -> ContentResponse {
        if !self.available() || self.payload_authority.is_none() || self.candidates.is_none() {
            return ContentResponse::default();
        }
        let Some(prepared) =
            self.view
                .prepare_confirmation(&self.model, &self.editor.surface.input, index, false)
        else {
            return ContentResponse::default();
        };
        let label = self.model.items[index].clone();
        let Some(index) =
            self.view
                .apply_confirmation(&mut self.model, &mut self.editor.surface.input, prepared)
        else {
            return ContentResponse::default();
        };
        self.finish_selection(index, label);
        ContentResponse {
            activated: true,
            ..ContentResponse::consumed(true)
        }
    }
    fn finish_selection(&mut self, index: usize, label: String) {
        self.editor.surface.clauses.clear();
        self.editor.surface.reveal();
        self.candidates = None;
        self.last_selection = Some(ComboBoxSelection {
            index,
            label,
            declaration_revision: self.declaration,
            authorization: self
                .payload_authority
                .expect("prepared payload authorization"),
        });
    }
    fn combo_key(&mut self, key: ControlKey, down: bool) -> ContentResponse {
        if !self.popup_available() {
            return ContentResponse::default();
        }
        if down
            && (!self.model.open || self.candidates.is_none())
            && matches!(key, ControlKey::Enter | ControlKey::Down)
        {
            let Ok(plan) = self.prepare_candidates(self.model.items.len(), true) else {
                return ContentResponse::consumed(false);
            };
            if self.payload_authority.is_none() {
                self.payload_authority = Some(plan.group as u64);
            }
            self.publish_candidates(plan);
        }
        if key == ControlKey::Enter && down && self.model.open {
            let response = self
                .model
                .active
                .map(|index| self.confirm(index))
                .unwrap_or_default();
            return if response.activated {
                response
            } else {
                ContentResponse::consumed(false)
            };
        }
        let r = self.view.key_event(key, down, &mut self.model);
        if !self.model.open {
            self.candidates = None;
        }
        ContentResponse {
            consumed: r.consumed,
            changed: r.changed,
            ..Default::default()
        }
    }
    fn key(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if self.editor.composing() {
            let r = self.editor.key_event(key);
            self.sync_real_edit();
            return r;
        }
        if let Some(control) = key.key
            && matches!(
                control,
                ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
            )
        {
            return self.combo_key(control, key.down);
        }
        let r = self.editor.key_event(key);
        self.sync_real_edit();
        r
    }
}
impl Component for ComboBoxContent {
    // Original single editor under ordinary rectangular hosts. Popup geometry
    // does not claim the newer curved Panel subtree contract.
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        self.editor.prepared_effects_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        // Validate the retained editor's unmark/counter contract first. The
        // wrapper owns these hooks; its remaining revocations allocate nothing.
        self.editor.prepare_effects(hooks)?;
        Ok(PreparedEffects::local(self, hooks))
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        self.prepare_combo_action(request)
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.editor.field.label.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn requires_text_services(&self) -> bool {
        self.editor.requires_text_services()
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.editor.text_surface()
    }
    fn composing(&self) -> bool {
        self.editor.composing()
    }
    fn focusable(&self) -> bool {
        self.editor.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.editor.set_focus(focused, visible);
        if !focused {
            self.sync_real_edit();
            self.model.open = false;
            self.revoke_candidates();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.revoke();
            self.editor.surface.cancel_interaction();
        }
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.runnable = runnable;
        if !runnable {
            self.revoke();
            self.editor.surface.cancel_interaction();
        }
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.revoke();
        self.editor.surface.cancel_interaction();
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        self.editor.surface.cancel_interaction();
        self.view.options.cancel_pointer_arm();
        ContentResponse::consumed(true)
    }
    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // The original Host calls this before input as well as after it. Apply
        // the existing backend preference before a wheel callback can use the
        // previous frame's scroll policy; no paint or second clock is needed.
        if let Some(clock) = text.motion_context() {
            self.view.options.set_reduced_motion(clock.reduced_motion);
        }
        self.editor.prepare_text_input(text, frame, clip);
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.editor.measure(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.editor.arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.editor.paint(scene, text, frame, clip, theme);
    }
    fn semantic_frame(&self, frame: Rect) -> Rect {
        self.editor.semantic_frame(frame)
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = self.editor.semantics();
        s.role = ContentRole::ComboBox;
        s.expanded = Some(self.overlay_open());
        s
    }
    fn overlay_open(&self) -> bool {
        self.popup_available() && self.model.open && self.candidates.is_some()
    }
    fn overlay_presented(&self) -> bool {
        self.overlay_open() || self.view.options.is_presented()
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if self.model.open && !self.overlay_open() {
            return;
        }
        if let Some(clock) = text.motion_context() {
            self.view.options.set_reduced_motion(clock.reduced_motion);
            if let Some(last) = self.last_frame {
                self.view
                    .options
                    .tick(clock.now.saturating_sub(last).as_secs_f32());
            }
            self.last_frame = Some(clock.now);
        }
        self.view.render(
            scene,
            &mut text,
            &mut self.model,
            self.editor.surface.outer,
            clip,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.view.options.needs_animation() {
            text.request_animation_frame();
        }
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.key(key)
    }
    fn overlay_key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.key(key)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if !self.editor.composing()
            && let ContentEvent::TextCommand(command) = e
        {
            let key = match command.as_str() {
                "insertNewline:" | "insertLineBreak:" => Some(ControlKey::Enter),
                "moveUp:" => Some(ControlKey::Up),
                "moveDown:" => Some(ControlKey::Down),
                _ => None,
            };
            if let Some(key) = key {
                return self.combo_key(key, true);
            }
        }
        if let ContentEvent::Key { key, down } = e {
            if self.editor.composing() {
                return self.editor_event(e);
            }
            if matches!(
                key,
                ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
            ) {
                return self.combo_key(*key, *down);
            }
        }
        if matches!(e, ContentEvent::Cancel) {
            self.revoke();
            return self.editor.event(e);
        }
        if matches!(e, ContentEvent::PointerDown(_)) {
            return ContentResponse::default();
        }
        self.editor_event(e)
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        if matches!(pointer.phase, PointerPhase::Down) {
            return ContentResponse::default();
        }
        let r = self.editor.pointer_event(pointer);
        self.sync_real_edit();
        r
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        if !self.editor.composing()
            && matches!(e, ContentEvent::TextCommand(c) if c == "insertNewline:" || c == "insertLineBreak:")
        {
            return self.combo_key(ControlKey::Enter, true);
        }
        match e {
            ContentEvent::PointerUp(point) => {
                if self.editor.composing() {
                    return ContentResponse::consumed(false);
                }
                if let Some(prepared) = self.view.prepare_pointer_confirmation(
                    &self.model,
                    &self.editor.surface.input,
                    *point,
                ) {
                    let index = prepared.index();
                    let label = self.model.items[index].clone();
                    self.view
                        .apply_confirmation(
                            &mut self.model,
                            &mut self.editor.surface.input,
                            prepared,
                        )
                        .expect("same-operation checked original confirmation");
                    self.finish_selection(index, label);
                    return ContentResponse {
                        activated: true,
                        ..ContentResponse::consumed(true)
                    };
                }
                self.view.options.cancel_pointer_arm();
                ContentResponse::consumed(false)
            }
            ContentEvent::PointerDown(p)
                if !self.editor.surface.contains_visible(*p)
                    && !self
                        .view
                        .options
                        .layout()
                        .is_some_and(|l| l.visible.is_some_and(|r| r.contains(*p))) =>
            {
                self.model.open = false;
                self.revoke_candidates();
                ContentResponse {
                    capture_pointer: true,
                    ..ContentResponse::consumed(true)
                }
            }
            ContentEvent::PointerDown(p) => {
                if self.editor.surface.contains_visible(*p) {
                    return ContentResponse::default();
                }
                if self.editor.composing() || !self.view.options.is_current(&self.model) {
                    return ContentResponse::consumed(false);
                }
                self.view.pointer_down(*p);
                ContentResponse {
                    capture_pointer: self.view.candidate_pointer_armed(),
                    ..ContentResponse::consumed(true)
                }
            }
            ContentEvent::PointerMove(p) => {
                if !self.editor.composing() {
                    self.view.pointer_move(*p, &mut self.model);
                }
                ContentResponse::consumed(true)
            }
            ContentEvent::Scroll {
                position,
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
            } => {
                if self.editor.composing() {
                    return ContentResponse::consumed(false);
                }
                let result = self.view.options.wheel_event(crate::MenuWheelInput {
                    point: *position,
                    delta: *delta,
                    precise: *precise,
                    begin: *begin,
                    end: *end,
                    momentum: *momentum,
                    time: *time,
                });
                match result {
                    crate::MenuWheelEvent::Consumed { scrolled } => {
                        ContentResponse::consumed(scrolled)
                    }
                    crate::MenuWheelEvent::Ignored => ContentResponse::consumed(false),
                }
            }
            ContentEvent::Key {
                key: ControlKey::Tab { .. },
                ..
            } => ContentResponse::default(),
            ContentEvent::Key { key, down } => self.combo_key(*key, *down),
            _ => ContentResponse::default(),
        }
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.overlay_open() || !self.view.options.is_current(&self.model) {
            return Vec::new();
        }
        let Some(plan) = &self.candidates else {
            return Vec::new();
        };
        let Some(layout) = self.view.options.layout() else {
            return Vec::new();
        };
        let mut parts = vec![SemanticPart {
            index: plan.group,
            frame: layout.popup,
            focused: false,
            semantics: ContentSemantics::new(ContentRole::Menu, ""),
        }];
        for (filtered, row) in layout.items.iter().enumerate() {
            let Some(index) = self.view.options.original_index(filtered) else {
                continue;
            };
            let Some(token) = plan.parts.get(index) else {
                continue;
            };
            let mut semantics =
                ContentSemantics::new(ContentRole::MenuItem, &self.model.items[index]);
            semantics.selected = self.model.selected == Some(index);
            parts.push(SemanticPart {
                index: *token,
                frame: row.frame,
                focused: self.model.active == Some(index),
                semantics,
            });
        }
        parts
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        let plan = self.candidates.as_ref()?;
        (self.overlay_open() && plan.parts.contains(&part)).then_some(plan.group)
    }
    fn semantic_part_preserves_activation_focus(&self, _part: usize) -> bool {
        true
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if matches!(
            action,
            AccessibilityAction::Press | AccessibilityAction::SetExpanded(_)
        ) || part.is_some()
        {
            let ActionPreparation::Ready(action) =
                self.prepare_action(ComponentActionRequest::Accessibility { part, action })
            else {
                return ContentResponse::default();
            };
            return action_preparation::apply_local_prepared(action, self);
        }
        let r = self.editor.accessibility_action(part, action);
        self.sync_real_edit();
        r
    }
    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.activated {
            vec![ContentActionKind::Activate]
        } else {
            Vec::new()
        }
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| self.last_selection.clone())
            .flatten()
            .map(ContentActionPayload::ComboBoxSelection)
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        match payload {
            ContentActionPayload::ComboBoxSelection(s) => {
                self.available()
                    && self.payload_authority == Some(s.authorization)
                    && self.declaration == s.declaration_revision
                    && self
                        .model
                        .items
                        .get(s.index())
                        .is_some_and(|label| label == s.label())
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
enum ComboOperation {
    Editor {
        decision: Box<adapters::PreparedFieldAction>,
        request: action_preparation::OwnedActionRequest,
        plan: Option<CandidatePlan>,
    },
    Open(CandidatePlan),
    Close,
    Confirm {
        prepared: Box<crate::layout_view::PreparedComboConfirmation>,
        index: usize,
        label: String,
    },
    FocusOption(usize),
}
#[derive(Debug)]
struct ComboEffect {
    model: ComboBox,
    input: crate::text_input::TextActionInputSnapshot,
    declaration: u64,
    payload_authority: Option<u64>,
    candidates: Option<CandidatePlan>,
    next_generation: Option<u64>,
    window_active: bool,
    runnable: bool,
    operation: ComboOperation,
}
impl PreparedActionEffect for ComboEffect {
    fn validate(&self, owner: &dyn Component) -> bool {
        let Some(c) = (owner as &dyn Any).downcast_ref::<ComboBoxContent>() else {
            return false;
        };
        if c.model != self.model
            || c.editor.surface.input.action_snapshot() != self.input
            || c.declaration != self.declaration
            || c.payload_authority != self.payload_authority
            || c.candidates != self.candidates
            || c.view.next_open_generation() != self.next_generation
            || c.window_active != self.window_active
            || c.runnable != self.runnable
        {
            return false;
        }
        match &self.operation {
            ComboOperation::Editor {
                decision, request, ..
            } => c.editor.decide_action(request).as_ref() == Some(decision.as_ref()),
            ComboOperation::Confirm { prepared, .. } => {
                c.view
                    .confirmation_current(&c.model, &c.editor.surface.input, prepared)
            }
            ComboOperation::FocusOption(index) => {
                c.view.options.is_current(&c.model) && c.model.filtered().contains(index)
            }
            _ => true,
        }
    }
    fn apply(self: Box<Self>, owner: &mut dyn Component) {
        let c = (owner as &mut dyn Any)
            .downcast_mut::<ComboBoxContent>()
            .expect("validated ComboBox owner");
        match self.operation {
            ComboOperation::Editor { decision, plan, .. } => {
                c.editor.apply_action(*decision);
                if let Some(plan) = plan {
                    if c.payload_authority.is_none() {
                        c.payload_authority = Some(plan.group as u64);
                    }
                    if c.model.query != c.editor.surface.input.text {
                        c.model.set_query(c.editor.surface.input.text.clone());
                    }
                    c.model.open = true;
                    if c.model.active.is_none() {
                        c.model.active = c.model.filtered().first().copied();
                    }
                    c.publish_candidates(plan);
                }
            }
            ComboOperation::Open(plan) => {
                if c.payload_authority.is_none() {
                    c.payload_authority = Some(plan.group as u64);
                }
                c.model.open = true;
                if c.model.active.is_none() {
                    c.model.active = c.model.filtered().first().copied();
                }
                c.publish_candidates(plan);
            }
            ComboOperation::Close => {
                c.model.open = false;
                c.revoke_candidates();
            }
            ComboOperation::Confirm {
                prepared,
                index,
                label,
            } => {
                c.view
                    .commit_confirmation(&mut c.model, &mut c.editor.surface.input, *prepared);
                c.finish_selection(index, label);
            }
            ComboOperation::FocusOption(index) => {
                let _ = c.view.focus_option(index, &mut c.model);
            }
        }
    }
}
impl ComboBoxContent {
    fn prepare_combo_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        if !self.available() {
            return ActionPreparation::Unsupported;
        }
        let (operation, response, focus) = match request {
            ComponentActionRequest::PointerDown(event) => {
                let request = action_preparation::OwnedActionRequest::Pointer(event.clone());
                let Some(decision) = self.editor.decide_action(&request) else {
                    return ActionPreparation::Unsupported;
                };
                let response = decision.response;
                let plan = if response.consumed {
                    let Ok(plan) = self.prepare_candidates(
                        self.model.items.len(),
                        !self.model.open || self.candidates.is_none(),
                    ) else {
                        return ActionPreparation::Unsupported;
                    };
                    Some(plan)
                } else {
                    None
                };
                (
                    ComboOperation::Editor {
                        decision: Box::new(decision),
                        request,
                        plan,
                    },
                    response,
                    if response.consumed {
                        PreparedActionFocus::Owner
                    } else {
                        PreparedActionFocus::None
                    },
                )
            }
            ComponentActionRequest::Accessibility {
                part: None,
                action: AccessibilityAction::SetSelection(range),
            } => {
                let request = action_preparation::OwnedActionRequest::Accessibility(
                    None,
                    AccessibilityAction::SetSelection(range.clone()),
                );
                let Some(decision) = self.editor.decide_action(&request) else {
                    return ActionPreparation::Unsupported;
                };
                let response = decision.response;
                (
                    ComboOperation::Editor {
                        decision: Box::new(decision),
                        request,
                        plan: None,
                    },
                    response,
                    PreparedActionFocus::Owner,
                )
            }
            ComponentActionRequest::Accessibility {
                part: None,
                action: AccessibilityAction::SetExpanded(false),
            } => (
                ComboOperation::Close,
                ContentResponse::consumed(self.model.open),
                PreparedActionFocus::Preserve,
            ),
            ComponentActionRequest::Accessibility {
                part: None,
                action: AccessibilityAction::SetExpanded(true) | AccessibilityAction::Press,
            } => {
                if self.editor.composing() {
                    return ActionPreparation::Unsupported;
                }
                let Ok(plan) = self.prepare_candidates(
                    self.model.items.len(),
                    !self.model.open || self.candidates.is_none(),
                ) else {
                    return ActionPreparation::Unsupported;
                };
                (
                    ComboOperation::Open(plan),
                    ContentResponse::consumed(!self.model.open),
                    PreparedActionFocus::Owner,
                )
            }
            ComponentActionRequest::Accessibility {
                part: Some(token),
                action,
            } => {
                if self.editor.composing()
                    || !self.overlay_open()
                    || !self.view.options.is_current(&self.model)
                {
                    return ActionPreparation::Unsupported;
                }
                let Some(index) = self
                    .candidates
                    .as_ref()
                    .and_then(|p| p.parts.iter().position(|id| *id == token))
                else {
                    return ActionPreparation::Unsupported;
                };
                match action {
                    AccessibilityAction::Press => {
                        if self.payload_authority.is_none() {
                            return ActionPreparation::Unsupported;
                        }
                        let Some(prepared) = self.view.prepare_confirmation(
                            &self.model,
                            &self.editor.surface.input,
                            index,
                            true,
                        ) else {
                            return ActionPreparation::Unsupported;
                        };
                        (
                            ComboOperation::Confirm {
                                prepared: Box::new(prepared),
                                index,
                                label: self.model.items[index].clone(),
                            },
                            ContentResponse {
                                activated: true,
                                ..ContentResponse::consumed(true)
                            },
                            PreparedActionFocus::Preserve,
                        )
                    }
                    AccessibilityAction::Focus => (
                        ComboOperation::FocusOption(index),
                        ContentResponse::consumed(self.model.active != Some(index)),
                        PreparedActionFocus::Preserve,
                    ),
                    _ => return ActionPreparation::Unsupported,
                }
            }
            _ => return ActionPreparation::Unsupported,
        };
        ActionPreparation::Ready(PreparedComponentAction::new(
            response,
            focus,
            Box::new(ComboEffect {
                model: self.model.clone(),
                input: self.editor.surface.input.action_snapshot(),
                declaration: self.declaration,
                payload_authority: self.payload_authority,
                candidates: self.candidates.clone(),
                next_generation: self.view.next_open_generation(),
                window_active: self.window_active,
                runnable: self.runnable,
                operation,
            }),
        ))
    }
    pub(super) fn capture_current(&self, overlay: bool) -> bool {
        self.available()
            && if overlay {
                !self.model.open || self.view.candidate_pointer_armed()
            } else {
                self.editor.surface.pointer_active()
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn combo() -> ComboBoxContent {
        let mut c = ComboBoxContent::new(
            "Layout",
            vec![
                "".into(),
                "Compact".into(),
                "Compact".into(),
                "Comfortable".into(),
            ],
        );
        c.editor.set_focus(true, false);
        c
    }
    #[test]
    fn focus_hook_preserves_prepared_confirmation_and_raw_label_policy() {
        let mut c = combo();
        c.set_items(vec!["A\nB".into()]).unwrap();
        c.set_query("").unwrap();
        assert!(c.confirm(0).activated);
        let model = c.model.clone();
        let editor = c.editor.surface.input.external_baseline();
        let generation = c.view.next_open_generation();
        let payload = c.action_payload(ContentActionKind::Activate).unwrap();
        c.set_focus(true, true);
        assert_eq!(c.model, model);
        assert_eq!(c.model.query, "A\nB");
        assert_eq!(c.editor.surface.input.external_baseline(), editor);
        assert_eq!(c.view.next_open_generation(), generation);
        assert!(c.action_payload_current(&payload));
    }
    #[test]
    fn checked_confirmation_has_no_partial_model_or_editor_mutation() {
        let mut c = combo();
        c.set_query("Com").unwrap();
        let prepared = c
            .view
            .prepare_confirmation(&c.model, &c.editor.surface.input, 2, false)
            .unwrap();
        c.editor.surface.input.select_all(); // real editor epoch/selection changed before apply
        let before = c.model.clone();
        let selection = c.editor.surface.input.selection();
        assert!(
            c.view
                .apply_confirmation(&mut c.model, &mut c.editor.surface.input, prepared)
                .is_none()
        );
        assert_eq!(c.model, before);
        assert_eq!(c.editor.surface.input.text, "Com");
        assert_eq!(c.editor.surface.input.selection(), selection);
        let prepared = c
            .view
            .prepare_confirmation(&c.model, &c.editor.surface.input, 2, false)
            .unwrap();
        assert_eq!(
            c.view
                .apply_confirmation(&mut c.model, &mut c.editor.surface.input, prepared),
            Some(2)
        );
        assert_eq!(c.model.selected, Some(2));
        assert_eq!(c.model.query, "Compact");
        assert_eq!(c.editor.surface.input.text, "Compact");
        assert!(!c.model.open);
        assert!(c.editor.surface.input.undo());
        assert_eq!(c.editor.surface.input.text, "Com");
    }
    #[test]
    fn prepared_keyboard_confirmation_cannot_survive_items_aba_reauthorization() {
        let mut c = combo();
        c.set_query("Com").unwrap();
        let prepared = c
            .view
            .prepare_confirmation(&c.model, &c.editor.surface.input, 2, false)
            .unwrap();
        let original = c.model.items.clone();
        c.set_items(vec!["temporary".into()]).unwrap();
        c.set_items(original).unwrap();
        let before = c.model.clone();
        let editor = c.editor.surface.input.external_baseline();
        assert!(
            c.view
                .apply_confirmation(&mut c.model, &mut c.editor.surface.input, prepared)
                .is_none()
        );
        assert_eq!(c.model, before);
        assert_eq!(c.editor.surface.input.external_baseline(), editor);
        assert!(c.confirm(2).activated, "a fresh decision remains valid");
    }
    #[test]
    fn marked_items_update_accepts_empty_duplicate_labels_without_editing() {
        let mut c = combo();
        c.set_query("Com").unwrap();
        c.editor_event(&ContentEvent::Preedit {
            text: "候補".into(),
            selection: 1..1,
            replacement: Some(0..3),
            clauses: std::iter::once(0..2).collect(),
        });
        let baseline = c.editor.surface.input.external_baseline();
        let clauses = c.editor.surface.clauses.clone();
        let query = c.model.query.clone();
        assert!(
            c.set_items(vec!["".into(), "".into(), "候補".into()])
                .unwrap()
        );
        assert_eq!(c.editor.surface.input.external_baseline(), baseline);
        assert_eq!(c.editor.surface.clauses, clauses);
        assert_eq!(c.model.query, query);
        assert_eq!(
            c.set_query("replacement"),
            Err(ComboBoxUpdateError::Composing)
        );
        assert!(!c.combo_key(ControlKey::Enter, true).activated);
        assert_eq!(c.editor.surface.input.external_baseline(), baseline);
        assert!(
            c.event(&ContentEvent::Key {
                key: ControlKey::Escape,
                down: true
            })
            .consumed
        );
        assert_eq!(c.editor.surface.input.text, "Com");
        assert!(c.editor.surface.input.marked.is_none());
        assert!(c.model.open);
    }
    #[test]
    fn capacity_failure_preserves_controlled_state_but_never_rolls_back_real_text() {
        let mut c = combo();
        c.set_query("Com").unwrap();
        let model = c.model.clone();
        let editor = c.editor.surface.input.external_baseline();
        let tokens = c.candidates.as_ref().unwrap().parts.clone();
        actions::with_exhausted_mounts(|| {
            assert_eq!(
                c.set_items(vec!["new".into()]),
                Err(ComboBoxUpdateError::IdentityExhausted)
            );
            assert_eq!(
                c.set_query("new"),
                Err(ComboBoxUpdateError::IdentityExhausted)
            );
            assert_eq!(c.model, model);
            assert_eq!(c.editor.surface.input.external_baseline(), editor);
            assert_eq!(c.candidates.as_ref().unwrap().parts, tokens);
            c.editor_event(&ContentEvent::Commit {
                text: "x".into(),
                replacement: None,
            });
            assert_eq!(c.model.query, c.editor.surface.input.text);
            assert_eq!(c.model.query, "Comx");
            assert!(c.candidates.is_none());
            assert!(!c.overlay_open());
            assert!(!c.combo_key(ControlKey::Enter, true).activated);
        });
    }
    #[test]
    fn confirm_payload_survives_own_query_and_close_but_program_setters_revoke() {
        let mut c = combo();
        c.set_query("Com").unwrap();
        assert!(c.confirm(2).activated);
        let payload = c.action_payload(ContentActionKind::Activate).unwrap();
        assert!(c.action_payload_current(&payload));
        assert!(!c.model.open);
        assert!(!c.combo_key(ControlKey::Enter, false).activated);
        assert!(c.action_payload_current(&payload));
        assert!(c.combo_key(ControlKey::Enter, true).consumed);
        assert!(c.model.open);
        assert!(c.confirm(2).activated);
        assert!(c.action_payload_current(&payload));
        c.set_query("Compact").unwrap();
        assert!(!c.action_payload_current(&payload));
        let before = c.model.selected;
        c.set_items(vec![]).unwrap();
        assert_eq!(c.model.selected, before);
        assert!(c.model.filtered().is_empty());
    }
}
