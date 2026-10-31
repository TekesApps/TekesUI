//! Search policy over the original editor and Button. SearchInput semantics
//! require the matched controls/wire/native closure tracked by #101.
use super::*;
use crate::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, TextEditBaseline,
    TextInput, TextSurface,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchCommandKind {
    Submit,
    Clear,
}

/// Immutable delivery data; the value is never polled again when draining.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchAction {
    pub kind: SearchCommandKind,
    /// Submitted query, or the query immediately before clear.
    pub query: String,
    binding: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum PreparedSearchEffect {
    Editor(super::adapters::PreparedFieldAction),
    ClearPointer(crate::widgets::ButtonActionDecision),
    ClearAccessibility {
        button: crate::widgets::ButtonActionDecision,
        insert: crate::text_input::PreparedTextInsert,
        action: SearchAction,
        activations: usize,
    },
}
#[derive(Debug, PartialEq)]
struct PreparedSearchAction {
    effect: PreparedSearchEffect,
    response: ContentResponse,
    focus: PreparedActionFocus,
    focus_rect: Option<Rect>,
}
impl super::action_preparation::OriginalActionDecision for PreparedSearchAction {
    fn focus_rect(&self) -> Option<Rect> {
        self.focus_rect
    }
}

/// Local, single-use request. Capture grants no authority; execute revalidates.
/// This deliberately has neither Clone nor a wire representation.
#[derive(Debug)]
pub struct SearchCommand {
    target: ComponentId,
    mount: u64,
    binding: Vec<u8>,
    baseline: TextEditBaseline,
    kind: SearchCommandKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchCommandError {
    Unavailable,
    WrongComponent,
    StaleMount,
    StaleEditor,
    StaleBaseline,
    Composition,
    InputInProgress,
    EmptyQuery,
}

/// One native text owner. Neither the clear action nor its semantic part owns
/// another TextInput, focus state machine, undo history or query cache.
#[derive(Debug)]
pub struct SearchFieldContent {
    editor: TextFieldContent,
    clear: Button,
    editor_frame: Rect,
    clear_frame: Rect,
    clear_clip: Option<Rect>,
    clear_captured: bool,
    editor_captured: bool,
    preserve_focus: bool,
    focused: bool,
    placeholder: Option<String>,
    last_action: Option<SearchAction>,
}

impl SearchFieldContent {
    pub fn new(name: impl Into<String>, query: &str) -> Self {
        let mut editor = TextFieldContent::new(name, query);
        editor.show_metadata = false;
        Self {
            editor,
            clear: Button::new("×").with_accessible_name("Clear"),
            editor_frame: Rect::default(),
            clear_frame: Rect::default(),
            clear_clip: None,
            clear_captured: false,
            editor_captured: false,
            preserve_focus: false,
            focused: false,
            placeholder: None,
            last_action: None,
        }
    }
    pub fn query(&self) -> &str {
        &self.editor.surface.input.text
    }
    pub fn input(&self) -> &TextInput {
        &self.editor.surface.input
    }
    pub fn set_submit_policy(&mut self, policy: TextSubmitPolicy) {
        self.editor.submit_policy = policy;
    }
    pub fn set_availability(&mut self, read_only: bool, disabled: bool) {
        self.editor.surface.input.read_only = read_only;
        self.editor.surface.input.disabled = disabled;
        self.sync_clear();
    }
    /// Original Field/TextSurface validation presentation, independent of the
    /// original submit policy and without a second validation engine.
    pub fn set_error(&mut self, error: Option<String>) {
        self.editor.surface.error = error.is_some();
        self.editor.field.error = error;
    }
    /// Host-owned copy, never inserted into the query or native text snapshot.
    pub fn set_placeholder(&mut self, placeholder: Option<String>) {
        self.placeholder = placeholder;
    }
    pub fn set_clear_label(&mut self, label: impl Into<String>) -> Result<(), CompositionError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.clear.set_accessible_name(Some(label));
        Ok(())
    }
    /// Authorized external publication retains the original conflict/idempotency
    /// policy, including readonly/disabled publication. It emits no user action.
    pub fn apply_query(
        &mut self,
        expected: &TextEditBaseline,
        value: &DraftRevision,
        selection: ExternalSelection,
        undo: ExternalUndo,
    ) -> ExternalValueResult {
        let result = self
            .editor
            .surface
            .apply_external_value(expected, value, selection, undo);
        if matches!(result, ExternalValueResult::Applied { .. }) {
            // A newly authorized publication invalidates a clear press over the
            // previous document. Original Surface owns editor/navigation cleanup.
            self.clear.button.cancel();
            self.clear_captured = false;
            self.sync_clear();
        }
        result
    }
    fn editable(&self) -> bool {
        !self.input().read_only && !self.input().disabled && self.input().marked.is_none()
    }
    fn can_clear(&self) -> bool {
        self.editable() && !self.query().is_empty()
    }
    fn sync_clear(&mut self) {
        self.clear.button.state.disabled = !self.can_clear();
        if self.clear.button.state.disabled {
            self.clear.button.cancel();
            self.clear_captured = false;
        }
    }
    fn begin_delivery(&mut self) {
        self.last_action = None;
        self.preserve_focus = false;
        self.sync_clear();
    }
    fn accept(&mut self, kind: SearchCommandKind) -> ContentResponse {
        if !self.editable() || (kind == SearchCommandKind::Clear && self.query().is_empty()) {
            return ContentResponse::default();
        }
        let action = SearchAction {
            kind,
            query: self.query().to_owned(),
            binding: ContentTextInput::local_binding(&self.editor.surface),
        };
        let changed = if kind == SearchCommandKind::Clear {
            let len = self.input().len();
            let changed = self.editor.surface.input.insert("", Some(0..len));
            self.editor.surface.clauses.clear();
            self.editor.surface.reveal();
            self.sync_clear();
            changed
        } else {
            false
        };
        self.last_action = Some(action);
        ContentResponse {
            activated: true,
            ..ContentResponse::consumed(changed)
        }
    }
    fn finish_editor(&mut self, mut response: ContentResponse) -> ContentResponse {
        if response.activated {
            let accepted = self.accept(SearchCommandKind::Submit);
            response.activated = accepted.activated;
        }
        self.sync_clear();
        response
    }
    fn clear_hit(&self, point: Point) -> bool {
        self.clear_clip.is_some_and(|clip| clip.contains(point)) && !self.query().is_empty()
    }
    fn cancel_gesture(&mut self) {
        self.clear.button.cancel();
        self.clear_captured = false;
        self.editor_captured = false;
        self.preserve_focus = false;
    }
    fn decide_prepared_action(
        &self,
        request: &super::action_preparation::OwnedActionRequest,
    ) -> Option<PreparedSearchAction> {
        match request {
            super::action_preparation::OwnedActionRequest::Pointer(event) => {
                let routed = event.routing_event();
                let ContentEvent::PointerDown(point) = routed.as_ref() else {
                    return None;
                };
                if matches!(event, ContentEvent::Pointer(pointer) if !pointer.is_primary()) {
                    return None;
                }
                let clear = self.clear_captured || !self.editor_captured && self.clear_hit(*point);
                if clear {
                    if !self.can_clear() {
                        return None;
                    }
                    let button = self.clear.button.prepare_content_down(
                        *point,
                        self.clear.pointer_activation_phase == ButtonActivationPhase::Press,
                    );
                    let response = ContentResponse {
                        consumed: button.consumed,
                        changed: button.consumed,
                        capture_pointer: button.consumed,
                        activated: button.activated,
                        ..Default::default()
                    };
                    Some(PreparedSearchAction {
                        effect: PreparedSearchEffect::ClearPointer(button),
                        response,
                        focus: PreparedActionFocus::Preserve,
                        focus_rect: None,
                    })
                } else {
                    let decision = self.editor.decide_action(request)?;
                    let response = decision.response;
                    let focus = if response.consumed {
                        PreparedActionFocus::Owner
                    } else {
                        PreparedActionFocus::None
                    };
                    let focus_rect =
                        super::action_preparation::OriginalActionDecision::focus_rect(&decision);
                    Some(PreparedSearchAction {
                        effect: PreparedSearchEffect::Editor(decision),
                        response,
                        focus,
                        focus_rect,
                    })
                }
            }
            super::action_preparation::OwnedActionRequest::Accessibility(
                Some(0),
                AccessibilityAction::Press,
            ) if self.clear_clip.is_some() => {
                if !self.can_clear() || !self.clear.focusable() {
                    return None;
                }
                let button = self.clear.button.prepare_accessibility_press();
                if !button.activated || self.clear.activations.checked_add(1).is_none() {
                    return None;
                }
                let input = &self.editor.surface.input;
                let insert = input.prepare_insert("", Some(0..input.len()))?;
                let action = SearchAction {
                    kind: SearchCommandKind::Clear,
                    query: self.query().to_owned(),
                    binding: ContentTextInput::local_binding(&self.editor.surface),
                };
                Some(PreparedSearchAction {
                    effect: PreparedSearchEffect::ClearAccessibility {
                        button,
                        insert,
                        action,
                        activations: self.clear.activations,
                    },
                    response: ContentResponse {
                        activated: true,
                        ..ContentResponse::consumed(true)
                    },
                    focus: PreparedActionFocus::Preserve,
                    focus_rect: None,
                })
            }
            super::action_preparation::OwnedActionRequest::Accessibility(_, _) => {
                let decision = self.editor.decide_action(request)?;
                let response = decision.response;
                let focus = if response.consumed {
                    PreparedActionFocus::Owner
                } else {
                    PreparedActionFocus::None
                };
                let focus_rect =
                    super::action_preparation::OriginalActionDecision::focus_rect(&decision);
                Some(PreparedSearchAction {
                    effect: PreparedSearchEffect::Editor(decision),
                    response,
                    focus,
                    focus_rect,
                })
            }
        }
    }
    fn apply_prepared_action(&mut self, decision: PreparedSearchAction) {
        match decision.effect {
            PreparedSearchEffect::Editor(editor) => {
                self.begin_delivery();
                if decision.response.capture_pointer {
                    self.editor_captured = true;
                }
                self.editor.apply_action(editor);
                self.sync_clear();
            }
            PreparedSearchEffect::ClearPointer(button) => {
                self.begin_delivery();
                self.preserve_focus = true;
                self.clear.button.apply_content_action(button);
                self.clear_captured = decision.response.capture_pointer;
            }
            PreparedSearchEffect::ClearAccessibility {
                button,
                insert,
                action,
                activations,
            } => {
                debug_assert_eq!(self.clear.activations, activations);
                self.begin_delivery();
                self.clear.button.apply_content_action(button);
                self.clear.activations = activations + 1;
                self.editor.surface.input.apply_insert(insert);
                self.editor.surface.clauses.clear();
                self.editor.surface.reveal();
                self.sync_clear();
                self.last_action = Some(action);
            }
        }
    }
}

impl Component for SearchFieldContent {
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        let owned = super::action_preparation::OwnedActionRequest::read(request);
        let recognized_clear = match &owned {
            super::action_preparation::OwnedActionRequest::Pointer(event) => {
                let routed = event.routing_event();
                match routed.as_ref() {
                    ContentEvent::PointerDown(point) => {
                        !matches!(event, ContentEvent::Pointer(pointer) if !pointer.is_primary())
                            && (self.clear_captured
                                || !self.editor_captured && self.clear_hit(*point))
                    }
                    _ => false,
                }
            }
            super::action_preparation::OwnedActionRequest::Accessibility(
                Some(0),
                AccessibilityAction::Press,
            ) => self.clear_clip.is_some(),
            _ => false,
        };
        if recognized_clear && self.decide_prepared_action(&owned).is_none() {
            return super::action_preparation::rejected();
        }
        super::action_preparation::prepare_original(
            self,
            request,
            Self::decide_prepared_action,
            Self::apply_prepared_action,
            |decision| (decision.response, decision.focus),
        )
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.editor.field.label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.clear.validate()?;
        self.editor.validate()
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
        self.focused = focused;
        if !focused {
            self.cancel_gesture();
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.last_action = None;
        self.cancel_gesture();
        self.editor.lifecycle(event);
        if event == Lifecycle::Unmounted {
            self.editor.set_focus(false, false);
            self.focused = false;
            self.clear_clip = None;
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = self.editor.semantics();
        // Intentionally requires #101. Never substitute ordinary TextInput.
        semantics.role = ContentRole::SearchInput;
        semantics
    }
    fn semantic_frame(&self, frame: Rect) -> Rect {
        frame
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if self.query().is_empty() {
            return Vec::new();
        }
        let Some(frame) = self.clear_clip else {
            return Vec::new();
        };
        let mut semantics = self.clear.semantics();
        semantics.disabled = !self.can_clear();
        vec![SemanticPart {
            index: 0,
            frame,
            focused: false,
            semantics,
        }]
    }
    fn semantic_part_preserves_activation_focus(&self, part: usize) -> bool {
        part == 0
    }
    fn preserves_activation_focus(&self) -> bool {
        self.preserve_focus
    }
    fn action_kinds(&self, response: &ContentResponse) -> Vec<ContentActionKind> {
        if !response.activated {
            return Vec::new();
        }
        match self.last_action.as_ref().map(|a| a.kind) {
            Some(SearchCommandKind::Submit) => vec![ContentActionKind::Submit],
            Some(SearchCommandKind::Clear) => vec![ContentActionKind::Clear],
            None => Vec::new(),
        }
    }
    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.last_action.clone().map(ContentActionPayload::Search)
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        matches!(payload, ContentActionPayload::Search(action)
            if self.editable() && action.binding == ContentTextInput::local_binding(&self.editor.surface))
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.begin_delivery();
        if part == Some(0)
            && matches!(action, AccessibilityAction::Press)
            && self.clear_clip.is_some()
        {
            let button = self.clear.accessibility_action(None, action);
            return if button.activated {
                self.accept(SearchCommandKind::Clear)
            } else {
                ContentResponse::default()
            };
        }
        let response = self.editor.accessibility_action(part, action);
        self.finish_editor(response)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let clear = self.clear.measure(text, available).size;
        let editor = self
            .editor
            .measure(
                text,
                Size::new(
                    (available.width - clear.width - tekes_ui_tokens::Spacing::Regular.value())
                        .max(0.),
                    available.height,
                ),
            )
            .size;
        MeasuredChild::new(Size::new(available.width, editor.height.max(clear.height)))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // Stable reservation, including an empty query; typing never resizes its owner.
        let width = self
            .clear
            .measure(text, frame.size)
            .size
            .width
            .min(frame.size.width.max(0.));
        self.clear_frame = Rect::new(
            frame.max_x() - width,
            frame.min_y(),
            width,
            frame.size.height,
        );
        self.clear_clip = self
            .clear_frame
            .intersection(&clip)
            .filter(|r| !r.is_empty());
        // Existing regular spacing keeps the original TextSurface focus-ring
        // outsets clear of the independent Button's chrome and hit frame.
        let gap = tekes_ui_tokens::Spacing::Regular
            .value()
            .min((frame.size.width - width).max(0.));
        self.editor_frame = Rect::new(
            frame.min_x(),
            frame.min_y(),
            (frame.size.width - width - gap).max(0.),
            frame.size.height,
        );
        self.editor.arrange(text, self.editor_frame, clip);
        self.clear.arrange(text, self.clear_frame, clip);
    }
    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        self.editor
            .prepare_text_input(text, self.editor_frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.sync_clear();
        if !self.query().is_empty() {
            self.clear.paint(scene, text, self.clear_frame, clip, theme);
        }
        self.editor
            .paint(scene, text, self.editor_frame, clip, theme);
        if self.query().is_empty()
            && !self.focused
            && let Some(placeholder) = &self.placeholder
            && let Some(visible) = self.editor.surface.inner.intersection(&clip)
        {
            let ink = tekes_ui_tokens::surface_palette::resolve_surface_with_options(
                theme.palette,
                theme.surface(),
                "input",
                if self.input().disabled {
                    "disabled"
                } else {
                    "rest"
                },
                tekes_ui_tokens::surface_palette::SurfaceOptions {
                    error: self.editor.surface.error,
                    read_only: self.input().read_only,
                },
            )
            .expect("validated original Search Surface")
            .helper
            .expect("original input helper ink");
            // The original Search placeholder is a single unwrapped label,
            // clipped to the real editor interior, never a second text owner.
            if let Some(host) = text.text_surface_host() {
                let paragraph = host.shape_surface(placeholder, self.editor.surface.role, None);
                let inner = self.editor.surface.inner;
                let origin = Point::new(
                    inner.min_x().round(),
                    (inner.center().y - paragraph.height as f32 * 0.5).round(),
                );
                scene.push_clip(visible);
                host.push_visible(scene, &paragraph, origin, ink, visible);
                scene.pop_clip();
            }
        }
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        self.begin_delivery();
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        let clear =
            self.clear_captured || !self.editor_captured && self.clear_hit(pointer.position);
        // Event-local policy is set even on a refused clear: never steal focus
        // when hitting disabled/marked auxiliary chrome.
        self.preserve_focus = clear && pointer.phase == PointerPhase::Down;
        if clear {
            if !self.can_clear() {
                return ContentResponse::default();
            }
            let mut response = self.clear.pointer_event(pointer);
            if response.capture_pointer {
                self.clear_captured = true;
            }
            if pointer.phase == PointerPhase::Up {
                self.clear_captured = false;
            }
            if response.activated {
                let accepted = self.accept(SearchCommandKind::Clear);
                response.activated = accepted.activated;
                response.changed |= accepted.changed;
            }
            return response;
        }
        let response = self.editor.pointer_event(pointer);
        if response.capture_pointer {
            self.editor_captured = true;
        }
        if pointer.phase == PointerPhase::Up {
            self.editor_captured = false;
        }
        self.finish_editor(response)
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.begin_delivery();
        let response = self.editor.key_event(key);
        self.finish_editor(response)
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Pointer(pointer) => return self.pointer_event(pointer),
            ContentEvent::PointerDown(position)
            | ContentEvent::PointerUp(position)
            | ContentEvent::PointerMove(position) => {
                return self.pointer_event(&PointerEvent {
                    position: *position,
                    phase: match event {
                        ContentEvent::PointerDown(_) => PointerPhase::Down,
                        ContentEvent::PointerUp(_) => PointerPhase::Up,
                        _ => PointerPhase::Move,
                    },
                    button: Some(PointerButton::Primary),
                    modifiers: InputModifiers::default(),
                    click_count: 1,
                });
            }
            _ => {}
        }
        self.begin_delivery();
        if matches!(event, ContentEvent::Cancel) {
            self.cancel_gesture();
        }
        // Keep legacy raw Key and native TextCommand behavior in the original
        // editor. In particular raw Key Enter must not enable submission.
        let response = self.editor.event(event);
        self.finish_editor(response)
    }
}

impl ContentHost {
    pub fn capture_search_command(
        &self,
        target: ComponentId,
        kind: SearchCommandKind,
    ) -> Result<SearchCommand, SearchCommandError> {
        let node = self
            .scope()
            .find_active(target)
            .ok_or(SearchCommandError::Unavailable)?;
        let search = node
            .component::<SearchFieldContent>()
            .ok_or(SearchCommandError::WrongComponent)?;
        Ok(SearchCommand {
            target,
            mount: node.mount,
            kind,
            binding: ContentTextInput::local_binding(&search.editor.surface),
            baseline: search.input().external_baseline(),
        })
    }
    pub fn execute_search_command(
        &mut self,
        command: SearchCommand,
    ) -> Result<ContentResponse, SearchCommandError> {
        // Nothing before the mutable lookup below reconciles/cancels/focuses or
        // writes a queue. All rejection paths leave every original owner intact.
        if !self.window_active || self.root.ids(&mut BTreeSet::new()).is_err() {
            return Err(SearchCommandError::Unavailable);
        }
        let node = self
            .scope()
            .find_active(command.target)
            .ok_or(SearchCommandError::Unavailable)?;
        if node.mount != command.mount {
            return Err(SearchCommandError::StaleMount);
        }
        let search = node
            .component::<SearchFieldContent>()
            .ok_or(SearchCommandError::WrongComponent)?;
        if ContentTextInput::local_binding(&search.editor.surface) != command.binding {
            return Err(SearchCommandError::StaleEditor);
        }
        if search.composing() || self.is_composing() {
            return Err(SearchCommandError::Composition);
        }
        if !self.text_keys.is_empty() || self.deferred_text_key.is_some() {
            return Err(SearchCommandError::InputInProgress);
        }
        if search.input().external_baseline() != command.baseline {
            return Err(SearchCommandError::StaleBaseline);
        }
        if !search.editable() {
            return Err(SearchCommandError::Unavailable);
        }
        if command.kind == SearchCommandKind::Clear && search.query().is_empty() {
            return Err(SearchCommandError::EmptyQuery);
        }
        let owner = self
            .root
            .find_mut(command.target)
            .expect("validated original Search owner");
        let search = owner
            .component_mut::<SearchFieldContent>()
            .expect("validated Search type");
        search.begin_delivery();
        let response = search.accept(command.kind);
        owner.record_actions(
            &response,
            ContentActionSource::Programmatic,
            true,
            &mut self.actions,
        );
        self.dirty |= response.changed;
        Ok(response)
    }
    /// Scope's original topmost interactive target, without dispatching a probe
    /// event. The Search-only guard cannot act on a sibling behind an overlay.
    pub(super) fn search_pointer_blocked(&self, event: &ContentEvent) -> bool {
        if !self.is_composing() {
            return false;
        }
        let routed = event.routing_event();
        let point = match routed.as_ref() {
            ContentEvent::PointerDown(p) | ContentEvent::PointerUp(p) => *p,
            _ => return false,
        };
        fn hit(content: &Content, point: Point) -> Option<&Content> {
            if !content.active() {
                return None;
            }
            if !content.component.semantics().role.is_action_owner()
                && let Some(child) = content
                    .component
                    .children()
                    .iter()
                    .rev()
                    .find_map(|c| hit(c, point))
            {
                return Some(child);
            }
            (content.component.interactive()
                && content.visible().is_some_and(|r| r.contains(point)))
            .then_some(content)
        }
        let target = self
            .captured
            .and_then(|id| self.scope().find_active(id))
            .or_else(|| hit(self.scope(), point));
        target.is_some_and(|node| {
            self.focused != Some(node.id)
                && node
                    .component::<SearchFieldContent>()
                    .is_some_and(|s| s.clear_captured || s.clear_hit(point))
        })
    }
    pub(super) fn search_ax_blocked(
        &self,
        id: ComponentId,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> bool {
        part == Some(0)
            && matches!(action, AccessibilityAction::Press)
            && self.is_composing()
            && self.focused != Some(id)
            && self
                .scope()
                .find_active(id)
                .is_some_and(|n| n.component::<SearchFieldContent>().is_some())
    }
}
