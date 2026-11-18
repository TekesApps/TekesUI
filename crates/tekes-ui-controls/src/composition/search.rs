//! Shared trailing Clear behavior for the original TextFieldContent editor.
//! SearchFieldContent adds SearchInput semantics and captured Search commands.
use super::text_field::{FieldActivationKind, PreparedTextFieldAction};
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

/// Search policy over the original clearable TextFieldContent. One native text owner.
#[derive(Debug)]
pub struct SearchFieldContent {
    editor: TextFieldContent,
}

impl SearchFieldContent {
    pub fn new(name: impl Into<String>, query: &str) -> Self {
        let single_line: String = query.chars().filter(|c| !c.is_control()).collect();
        let mut editor = TextFieldContent::new(name, &single_line);
        editor.show_metadata = false;
        Self { editor }
    }

    pub fn query(&self) -> &str {
        self.editor.text()
    }

    pub fn input(&self) -> &TextInput {
        self.editor.input()
    }

    pub fn surface(&self) -> &TextSurface {
        &self.editor.surface
    }

    pub fn field(&self) -> &crate::Field {
        &self.editor.field
    }

    pub fn clear_button(mut self, enabled: bool) -> Self {
        self.editor = self.editor.clear_button(enabled);
        self
    }

    pub fn scrollbar(mut self, visible: bool) -> Self {
        self.editor = self.editor.scrollbar(visible);
        self
    }

    pub fn set_submit_policy(&mut self, policy: TextSubmitPolicy) {
        self.editor.set_submit_policy(policy);
    }

    pub fn set_availability(&mut self, read_only: bool, disabled: bool) {
        self.editor.set_availability(read_only, disabled);
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.editor.set_error(error);
    }

    pub fn set_placeholder(&mut self, placeholder: Option<String>) {
        self.editor.set_placeholder(placeholder);
    }

    pub fn set_clear_label(&mut self, label: impl Into<String>) -> Result<(), CompositionError> {
        self.editor.set_clear_label(label)
    }

    pub fn apply_query(
        &mut self,
        expected: &TextEditBaseline,
        value: &DraftRevision,
        selection: ExternalSelection,
        undo: ExternalUndo,
    ) -> ExternalValueResult {
        self.editor
            .apply_external_and_sync(expected, value, selection, undo)
    }
}

impl std::ops::Deref for SearchFieldContent {
    type Target = TextFieldContent;
    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl SearchFieldContent {
    pub(super) fn search_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        precheck_dispatch::prepared_effects_work(&self.editor, n)
    }
    pub(super) fn prepare_search_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        precheck_dispatch::prepare_effects(&self.editor, hooks)?;
        Ok(PreparedEffects::local(self, hooks))
    }
    pub(super) fn prepare_search_action(
        &self,
        request: ComponentActionRequest<'_>,
    ) -> ActionPreparation {
        let owned = super::action_preparation::OwnedActionRequest::read(request);
        if self.editor.recognizes_clear(&owned)
            && self.editor.decide_prepared_action(&owned).is_none()
        {
            return super::action_preparation::rejected();
        }
        super::action_preparation::prepare_original(
            self,
            request,
            |owner, request| owner.editor.decide_prepared_action(request),
            |owner, decision| owner.editor.apply_prepared_action(decision),
            |decision: &PreparedTextFieldAction| (decision.response, decision.focus),
        )
    }
}
impl Component for SearchFieldContent {
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let mut properties = self.editor.inspection_properties()?;
        properties[0] = InspectionProperty::new("component", "SearchFieldContent");
        Some(properties)
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        self.editor.curve_subtree_capability()
    }

    fn has_stable_hooks(&self) -> bool {
        self.editor.has_stable_hooks()
    }

    fn supports_prepared_lifecycle(&self) -> bool {
        self.editor.supports_prepared_lifecycle()
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.editor.field.label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
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
        self.editor.set_focus(focused, visible)
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        self.editor.lifecycle(event)
    }

    fn window_active_changed(&mut self, active: bool) {
        self.editor.window_active_changed(active);
    }

    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.editor.presentation_runnable_changed(runnable);
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
        self.editor.semantic_parts()
    }

    fn semantic_part_preserves_activation_focus(&self, part: usize) -> bool {
        self.editor.semantic_part_preserves_activation_focus(part)
    }

    fn preserves_activation_focus(&self) -> bool {
        self.editor.preserves_activation_focus()
    }

    fn action_kinds(&self, response: &ContentResponse) -> Vec<ContentActionKind> {
        self.editor.action_kinds(response)
    }

    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.editor.last_activation().map(|activation| {
            ContentActionPayload::Search(SearchAction {
                kind: match activation.kind {
                    FieldActivationKind::Submit => SearchCommandKind::Submit,
                    FieldActivationKind::Clear => SearchCommandKind::Clear,
                },
                query: activation.text.clone(),
                binding: activation.binding.clone(),
            })
        })
    }

    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        matches!(payload, ContentActionPayload::Search(action)
            if self.editor.editable()
                && action.binding == ContentTextInput::local_binding(&self.editor.surface))
    }

    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.editor.accessibility_action(part, action)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.editor.measure(text, available)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.editor.arrange(text, frame, clip)
    }

    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.editor.prepare_text_input(text, frame, clip)
    }

    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.editor.paint(scene, text, frame, clip, theme)
    }

    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        self.editor.pointer_event(pointer)
    }

    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.editor.key_event(key)
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.editor.event(event)
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
        if !search.editor.editable() {
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
        search.editor.begin_delivery();
        let response = search.editor.accept_activation(match command.kind {
            SearchCommandKind::Submit => FieldActivationKind::Submit,
            SearchCommandKind::Clear => FieldActivationKind::Clear,
        });
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
                && clearable_field(node).is_some_and(|s| s.clear_captured || s.clear_hit(point))
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
                .is_some_and(|n| clearable_field(n).is_some())
    }
}

fn clearable_field(node: &Content) -> Option<&TextFieldContent> {
    node.component::<TextFieldContent>()
        .or_else(|| node.component::<SearchFieldContent>().map(|s| &s.editor))
}
