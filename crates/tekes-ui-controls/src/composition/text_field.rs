//! Original single/multiline editable text component backed by `Field`,
//! `FieldView`, and `TextSurface`.
use super::*;
use crate::feedback_view::FieldView;
use crate::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, Field, ReadingController,
    TextEditBaseline, TextInput, TextSurface, TextSurfacePresentation,
};

mod component_impl;
pub(super) use component_impl::PreparedFieldAction;
#[cfg(test)]
mod tests;
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

/// TextField composes the existing Field metadata/view and TextSurface. All
/// native queries read this same surface; no parallel string/selection state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextViewport {
    /// Existing form slot, including its normal minimum line height.
    #[default]
    Form,
    /// Input viewport height, excluding label/help/error geometry.
    Fixed(f32),
    /// Use the remaining allocated height after label/help/error geometry.
    /// Select this child using the existing StackContent.flexible_child.
    Fill,
    /// Grow with shaped content, bounded by input viewport heights.
    Grow { min: f32, max: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FieldActivationKind {
    Submit,
    Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FieldActivation {
    pub(super) kind: FieldActivationKind,
    pub(super) text: String,
    pub(super) binding: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub(super) enum PreparedTextFieldEffect {
    Editor(PreparedFieldAction),
    ClearPointer(crate::widgets::ButtonActionDecision),
    ClearAccessibility {
        button: crate::widgets::ButtonActionDecision,
        insert: crate::text_input::PreparedTextInsert,
        activation: FieldActivation,
        activations: usize,
    },
}

#[derive(Debug, PartialEq)]
pub(super) struct PreparedTextFieldAction {
    pub(super) effect: PreparedTextFieldEffect,
    pub(super) response: ContentResponse,
    pub(super) focus: PreparedActionFocus,
    pub(super) focus_rect: Option<Rect>,
}

impl action_preparation::OriginalActionDecision for PreparedTextFieldAction {
    fn focus_rect(&self) -> Option<Rect> {
        self.focus_rect
    }
}

#[derive(Clone, Debug)]
pub(super) struct CachedPlaceholder {
    text: String,
    role: proposal::TextRole,
    width_bits: Option<u32>,
    type_scale_bits: u32,
    paragraph: std::rc::Rc<tekes_ui_text::Paragraph>,
}

#[derive(Debug)]
pub struct TextFieldContent {
    pub field: Field,
    pub view: FieldView,
    pub surface: TextSurface,
    /// Optional explicit reading intent; ordinary editor instances leave this None.
    pub reading: Option<ReadingController>,
    pub tab_policy: TextTabPolicy,
    pub submit_policy: TextSubmitPolicy,
    pub viewport: TextViewport,
    /// Hide visible form label/help layout for inline controls. Semantic field
    /// metadata and the original TextSurface chrome/editing remain available.
    pub show_metadata: bool,
    pub(super) clear: Button,
    pub(super) clear_enabled: bool,
    pub(super) editor_frame: Rect,
    pub(super) clear_frame: Rect,
    pub(super) clear_clip: Option<Rect>,
    pub(super) clear_captured: bool,
    pub(super) clear_baseline: Option<TextEditBaseline>,
    pub(super) editor_captured: bool,
    pub(super) preserve_focus: bool,
    pub(super) placeholder: Option<String>,
    placeholder_cache: Option<CachedPlaceholder>,
    last_activation: Option<FieldActivation>,
    inline_width_em: Option<f32>,
    pub(super) focused: bool,
    backing_scale: f32,
}

impl TextFieldContent {
    pub(super) fn field_focus_bounds(&self) -> Option<Rect> {
        (self.surface.presentation == TextSurfacePresentation::Field).then(|| {
            // A Fill viewport may be empty; decoration cannot create input space.
            if self.surface.outer.is_empty() {
                self.surface.outer
            } else {
                crate::extended::ControlFrame::new(self.surface.outer, None)
                    .focus_bounds(self.backing_scale)
            }
        })
    }

    pub(crate) fn has_focus(&self) -> bool {
        self.focused
    }

    pub fn new(label: impl Into<String>, value: &str) -> Self {
        let mut content = Self {
            field: Field {
                label: label.into(),
                ..Default::default()
            },
            view: FieldView::default(),
            surface: TextSurface::new(value, false, false),
            reading: None,
            tab_policy: TextTabPolicy::Navigate,
            submit_policy: TextSubmitPolicy::None,
            viewport: TextViewport::Form,
            show_metadata: true,
            clear: Button::new("×").with_accessible_name("Clear"),
            clear_enabled: true,
            editor_frame: Rect::default(),
            clear_frame: Rect::default(),
            clear_clip: None,
            clear_captured: false,
            clear_baseline: None,
            editor_captured: false,
            preserve_focus: false,
            placeholder: None,
            placeholder_cache: None,
            last_activation: None,
            inline_width_em: None,
            focused: false,
            backing_scale: 1.,
        };
        content.surface.scroll.set_scrollbar_visibility(false, true);
        content
    }

    /// Show or hide the horizontal overflow indicator without changing text
    /// scrolling or insertion-point reveal. Vertical visibility is preserved.
    pub fn scrollbar(mut self, visible: bool) -> Self {
        let vertical = self.surface.scroll.scrollbar_requested(true);
        self.surface
            .scroll
            .set_scrollbar_visibility(visible, vertical);
        self
    }

    /// A single-line original editor for compact rows. Width follows body
    /// typography and the receiving allocation; editing/IME stay in TextSurface.
    pub fn inline(name: impl Into<String>, value: &str, width_em: f32) -> Self {
        let mut content = Self::new(name, value);
        content.show_metadata = false;
        content.inline_width_em = Some(if width_em.is_finite() {
            width_em.max(0.)
        } else {
            0.
        });
        content
    }

    /// Selectable document text without a visible form label, help, input
    /// background or border. The parent supplies decoration and document scroll.
    /// Native text services, controlled updates and selection reuse this field.
    pub fn read_only_content(
        name: impl Into<String>,
        value: &str,
        role: proposal::TextRole,
    ) -> Self {
        let mut content = Self::new(name, value);
        content.surface = TextSurface::new(value, true, true);
        content.surface.presentation = TextSurfacePresentation::Content;
        content.surface.set_role(role);
        content
            .surface
            .scroll
            .set_scroll_policy(crate::ScrollPolicy::Disabled, crate::ScrollPolicy::Disabled);
        content.viewport = TextViewport::Grow {
            min: 0.,
            max: f32::MAX,
        };
        content
    }

    pub fn text(&self) -> &str {
        self.surface.input.text()
    }

    pub fn input(&self) -> &TextInput {
        &self.surface.input
    }

    pub fn set_submit_policy(&mut self, policy: TextSubmitPolicy) {
        self.submit_policy = policy;
    }

    pub fn set_availability(&mut self, read_only: bool, disabled: bool) {
        self.surface.input.set_read_only(read_only);
        self.surface.input.set_disabled(disabled);
        self.sync_clear();
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.surface.error = error.is_some();
        self.field.error = error;
    }

    pub fn set_placeholder(&mut self, placeholder: Option<String>) {
        self.placeholder = placeholder;
        self.placeholder_cache = None;
    }

    pub fn set_clear_label(&mut self, label: impl Into<String>) -> Result<(), CompositionError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.clear.set_accessible_name(Some(label));
        Ok(())
    }

    /// Enable the built-in trailing Clear action. Enabled by default for
    /// nonempty single-line fields; multiline/document content never show it.
    pub fn clear_button(mut self, enabled: bool) -> Self {
        self.clear_enabled = enabled;
        self
    }

    pub(super) fn apply_external_and_sync(
        &mut self,
        expected: &TextEditBaseline,
        value: &DraftRevision,
        selection: ExternalSelection,
        undo: ExternalUndo,
    ) -> ExternalValueResult {
        let result = self
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

    pub(super) fn editable(&self) -> bool {
        !self.surface.input.read_only
            && !self.surface.input.disabled
            && self.surface.input.marked.is_none()
    }

    fn can_clear(&self) -> bool {
        self.clear_visible() && self.editable()
    }

    fn clear_visible(&self) -> bool {
        self.clear_enabled
            && !self.surface.input.multiline
            && self.surface.presentation == TextSurfacePresentation::Field
            && !self.text().is_empty()
    }

    pub(super) fn clear_hit(&self, point: Point) -> bool {
        self.clear_clip.is_some_and(|clip| clip.contains(point)) && self.clear_visible()
    }

    pub(super) fn recognizes_clear(&self, owned: &action_preparation::OwnedActionRequest) -> bool {
        match owned {
            action_preparation::OwnedActionRequest::Pointer(event) => {
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
            action_preparation::OwnedActionRequest::Accessibility(
                Some(0),
                AccessibilityAction::Press,
            ) => self.clear_clip.is_some(),
            _ => false,
        }
    }

    pub(super) fn last_activation(&self) -> Option<&FieldActivation> {
        self.last_activation.as_ref()
    }

    fn sync_clear(&mut self) {
        self.clear.button.state.disabled = !self.can_clear();
        // Public TextField consumers can publish directly through the original
        // Surface while a Clear press is held. That press must not clear the
        // replacement draft on release.
        let replaced = self
            .clear_baseline
            .as_ref()
            .is_some_and(|baseline| *baseline != self.surface.input.external_baseline());
        if self.clear.button.state.disabled || replaced {
            self.clear.button.cancel();
            self.clear_captured = false;
            self.clear_baseline = None;
        }
    }

    pub(super) fn begin_delivery(&mut self) {
        self.last_activation = None;
        self.preserve_focus = false;
        self.sync_clear();
    }

    pub(super) fn accept_activation(&mut self, kind: FieldActivationKind) -> ContentResponse {
        if !self.editable() || (kind == FieldActivationKind::Clear && self.text().is_empty()) {
            return ContentResponse::default();
        }
        let activation = FieldActivation {
            kind,
            text: self.text().to_owned(),
            binding: ContentTextInput::local_binding(&self.surface),
        };
        let changed = if kind == FieldActivationKind::Clear {
            let len = self.input().len();
            let changed = self.surface.input.insert("", Some(0..len));
            self.surface.clauses.clear();
            self.surface.reveal();
            self.sync_clear();
            changed
        } else {
            false
        };
        self.last_activation = Some(activation);
        ContentResponse {
            activated: true,
            ..ContentResponse::consumed(changed)
        }
    }

    fn finish_editor(&mut self, mut response: ContentResponse) -> ContentResponse {
        if response.activated {
            let accepted = self.accept_activation(FieldActivationKind::Submit);
            response.activated = accepted.activated;
        }
        self.sync_clear();
        response
    }

    fn cancel_gesture(&mut self) -> bool {
        let before = self.clear.button.state;
        self.clear.set_focus(false, false);
        self.clear_captured = false;
        self.clear_baseline = None;
        self.editor_captured = false;
        self.preserve_focus = false;
        before != self.clear.button.state
    }

    pub(super) fn decide_prepared_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedTextFieldAction> {
        match request {
            action_preparation::OwnedActionRequest::Pointer(event) => {
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
                    Some(PreparedTextFieldAction {
                        effect: PreparedTextFieldEffect::ClearPointer(button),
                        response,
                        focus: PreparedActionFocus::Preserve,
                        focus_rect: None,
                    })
                } else {
                    let decision = self.decide_action(request)?;
                    let response = decision.response;
                    let focus = if response.consumed {
                        PreparedActionFocus::Owner
                    } else {
                        PreparedActionFocus::None
                    };
                    let focus_rect =
                        action_preparation::OriginalActionDecision::focus_rect(&decision);
                    Some(PreparedTextFieldAction {
                        effect: PreparedTextFieldEffect::Editor(decision),
                        response,
                        focus,
                        focus_rect,
                    })
                }
            }
            action_preparation::OwnedActionRequest::Accessibility(
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
                let input = &self.surface.input;
                let insert = input.prepare_insert("", Some(0..input.len()))?;
                let activation = FieldActivation {
                    kind: FieldActivationKind::Clear,
                    text: self.text().to_owned(),
                    binding: ContentTextInput::local_binding(&self.surface),
                };
                Some(PreparedTextFieldAction {
                    effect: PreparedTextFieldEffect::ClearAccessibility {
                        button,
                        insert,
                        activation,
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
            action_preparation::OwnedActionRequest::Accessibility(_, _) => {
                let decision = self.decide_action(request)?;
                let response = decision.response;
                let focus = if response.consumed {
                    PreparedActionFocus::Owner
                } else {
                    PreparedActionFocus::None
                };
                let focus_rect = action_preparation::OriginalActionDecision::focus_rect(&decision);
                Some(PreparedTextFieldAction {
                    effect: PreparedTextFieldEffect::Editor(decision),
                    response,
                    focus,
                    focus_rect,
                })
            }
        }
    }

    pub(super) fn apply_prepared_action(&mut self, decision: PreparedTextFieldAction) {
        match decision.effect {
            PreparedTextFieldEffect::Editor(editor) => {
                self.begin_delivery();
                if decision.response.capture_pointer {
                    self.editor_captured = true;
                }
                self.apply_action(editor);
                self.sync_clear();
            }
            PreparedTextFieldEffect::ClearPointer(button) => {
                self.begin_delivery();
                self.preserve_focus = true;
                self.clear.button.apply_content_action(button);
                self.clear_captured = decision.response.capture_pointer;
                self.clear_baseline = self
                    .clear_captured
                    .then(|| self.surface.input.external_baseline());
            }
            PreparedTextFieldEffect::ClearAccessibility {
                button,
                insert,
                activation,
                activations,
            } => {
                debug_assert_eq!(self.clear.activations, activations);
                self.begin_delivery();
                self.clear.button.apply_content_action(button);
                self.clear.activations = activations + 1;
                self.surface.input.apply_insert(insert);
                self.surface.clauses.clear();
                self.surface.reveal();
                self.sync_clear();
                self.last_activation = Some(activation);
            }
        }
    }

    fn viewport_height(
        &self,
        t: &mut dyn ButtonTextRenderer,
        frame: Rect,
        metadata: f32,
        minimum: f32,
    ) -> f32 {
        let sane = |n: f32| if n.is_finite() { n.max(0.) } else { 0. };
        match self.viewport {
            TextViewport::Form => minimum,
            TextViewport::Fixed(height) => sane(height).max(minimum),
            TextViewport::Fill => (sane(frame.size.height) - metadata).max(0.),
            TextViewport::Grow { min, max } => {
                let min = sane(min).max(minimum);
                let max = sane(max).max(min);
                let scale = t.backing_scale();
                let height = if let Some(host) = t.text_surface_host() {
                    let role = self.surface.role;
                    self.surface.measured_outer_height(
                        frame.size.width,
                        scale,
                        host.type_scale(),
                        |s, w| host.shape_surface(s, role, w),
                    )
                } else {
                    minimum
                };
                height.clamp(min, max)
            }
        }
    }

    fn field_presentation(
        &self,
        t: &mut dyn ButtonTextRenderer,
    ) -> crate::feedback_view::FieldPresentation {
        let mut presentation = crate::feedback_view::FieldPresentation::default();
        if self.surface.input.multiline {
            let type_scale = t.text_surface_host().map_or(1., |host| host.type_scale());
            presentation.slot_min_height = self.surface.minimum_outer_height(type_scale);
        }
        presentation
    }

    pub(super) fn effective_frame(&self, t: &mut dyn ButtonTextRenderer, f: Rect) -> Rect {
        if let Some(em) = self.inline_width_em {
            let max_w = (t.resolved_font_size(proposal::TextRole::UiBody) * em).max(0.);
            Rect::new(
                f.origin.x,
                f.origin.y,
                f.size.width.min(max_w),
                f.size.height,
            )
        } else {
            f
        }
    }

    pub(super) fn field_layout(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
    ) -> crate::feedback_view::FieldLayout {
        let f = self.effective_frame(t, f);
        if !self.show_metadata || self.surface.presentation == TextSurfacePresentation::Content {
            let type_scale = t.text_surface_host().map_or(1., |host| host.type_scale());
            let minimum = self.surface.minimum_outer_height(type_scale);
            let height = self.viewport_height(t, f, 0., minimum);
            return crate::feedback_view::FieldLayout {
                frame: f,
                slot: Rect::new(f.min_x(), f.min_y(), f.size.width, height),
                visible: f.intersection(&c),
                ..Default::default()
            };
        }
        let mut presentation = self.field_presentation(t);
        if self.viewport != TextViewport::Form {
            let base = FieldView::default().measure_with_presentation(
                &mut TextAdapter(t),
                &self.field,
                f,
                Some(c),
                proposal::TextRole::UiBody,
                presentation,
            );
            let metadata = base.slot.min_y() - f.min_y() + base.message.max_y() - base.slot.max_y();
            let minimum = presentation.slot_min_height.max(base.slot.size.height);
            let height = self.viewport_height(t, f, metadata, minimum);
            presentation.slot_height = Some(height);
        }
        self.view.measure_with_presentation(
            &mut TextAdapter(t),
            &self.field,
            f,
            Some(c),
            proposal::TextRole::UiBody,
            presentation,
        )
    }

    #[cfg(feature = "inspection")]
    pub(super) fn editor_inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", "TextFieldContent"),
            InspectionProperty::new("label", &self.field.label),
            InspectionProperty::new("text", &self.surface.input.text),
            InspectionProperty::new(
                "selection_utf16",
                format!("{:?}", self.surface.input.selection()),
            ),
            InspectionProperty::new("marked_utf16", format!("{:?}", self.surface.input.marked)),
            InspectionProperty::new("read_only", self.surface.input.read_only),
            InspectionProperty::new("disabled", self.surface.input.disabled),
            InspectionProperty::new("focus_visible", self.surface.focus_visible),
            InspectionProperty::new("viewport", format!("{:?}", self.viewport)),
            InspectionProperty::new("show_metadata", self.show_metadata),
        ])
    }

    pub(super) fn editor_key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.keyboard(key, true)
    }

    pub(super) fn editor_accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || self.surface.input.disabled || self.surface.input.marked.is_some() {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::SetSelection(_)) {
            let request =
                action_preparation::OwnedActionRequest::Accessibility(part, action.clone());
            let Some(decision) = self.decide_action(&request) else {
                return ContentResponse::default();
            };
            let response = decision.response;
            self.apply_action(decision);
            return response;
        }
        if self.surface.input.read_only {
            return ContentResponse::default();
        }
        if let AccessibilityAction::ReplaceSelectedText(value) = action {
            let changed = self.surface.input.insert(value, None);
            self.surface.clauses.clear();
            if changed {
                self.surface.reveal();
            }
            return ContentResponse::consumed(changed);
        }
        if let AccessibilityAction::SetValue(value) = action {
            let len = self.surface.input.len();
            let changed = self.surface.input.insert(value, Some(0..len));
            self.surface.clauses.clear();
            if changed {
                self.surface.reveal();
            }
            return ContentResponse::consumed(changed);
        }
        ContentResponse::default()
    }

    pub(super) fn editor_requires_text_services(&self) -> bool {
        true
    }

    pub(super) fn editor_text_surface(&self) -> Option<&TextSurface> {
        Some(&self.surface)
    }

    pub(super) fn editor_measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> MeasuredChild {
        let a = if let Some(em) = self.inline_width_em {
            Size::new(
                a.width
                    .min(t.resolved_font_size(proposal::TextRole::UiBody) * em),
                a.height,
            )
        } else {
            a
        };
        let l = self.field_layout(
            t,
            Rect::new(0., 0., a.width, a.height),
            Rect::new(0., 0., a.width, a.height),
        );
        MeasuredChild::new(Size::new(a.width, l.slot.max_y().max(l.message.max_y())))
    }

    pub(super) fn editor_arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let l = self.field_layout(t, f, c);
        self.surface.error = self.field.error.is_some();
        self.layout_surface(t, l.slot, c);
    }

    pub(super) fn editor_prepare_text_input(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
    ) {
        if !self.surface.outer.is_empty() {
            self.layout_surface(t, self.surface.outer, clip);
        }
    }

    pub(super) fn editor_paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let f = self.effective_frame(t, f);
        // Arrangement owns the exact viewport. Paint must not shape/resize a
        // second time or derive a different slot from an unrelated height.
        if self.show_metadata && self.surface.presentation == TextSurfacePresentation::Field {
            let mut presentation = self.field_presentation(t);
            if self.viewport != TextViewport::Form {
                presentation.slot_height = Some(self.surface.outer.size.height);
            }
            self.view.render_with_presentation(
                s,
                &mut TextAdapter(t),
                &self.field,
                f,
                Some(c),
                theme.palette,
                proposal::TextRole::UiBody,
                presentation,
            );
        }
        if let Some(host) = t.text_surface_host() {
            let caret_visible = host.caret_visible();
            self.surface
                .draw(host, s, theme.palette, self.has_focus(), caret_visible);
        }
    }

    pub(super) fn editor_focusable(&self) -> bool {
        !self.surface.input.disabled
    }

    pub(super) fn editor_set_focus(&mut self, f: bool, visible: bool) {
        self.surface.focus_visible = f && visible;
        if self.focused && !f {
            self.surface.break_kill_sequence();
            // Blur finalizes the visible draft using the existing TextInput
            // transaction; only explicit cancellation restores preedit text.
            self.surface.input.unmark();
            self.surface.clauses.clear();
        }
        self.focused = f;
        if !f {
            self.surface.cancel_interaction();
        }
    }

    pub(super) fn editor_composing(&self) -> bool {
        self.surface.input.marked.is_some()
    }

    pub(super) fn editor_semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::TextInput, &self.field.label);
        s.value = Some(self.surface.input.text.clone());
        s.disabled = self.surface.input.disabled;
        s.invalid = self.field.error.is_some();
        s.required = self.field.required;
        s.description = (!self.field.message().is_empty()).then(|| self.field.message().to_owned());
        s
    }

    pub(super) fn editor_event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.input_event(e, None)
    }

    pub(super) fn editor_pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        self.input_event(&pointer.routing_event(), Some(pointer))
    }

    fn layout_surface(&mut self, t: &mut dyn ButtonTextRenderer, slot: Rect, c: Rect) {
        let scale = t.backing_scale();
        if let Some(host) = t.text_surface_host() {
            let type_scale = host.type_scale();
            let role = self.surface.role;
            if let Some(reading) = &mut self.reading {
                reading.layout_with(
                    &mut self.surface,
                    slot,
                    Some(c),
                    scale,
                    type_scale,
                    |s, w| host.shape_surface(s, role, w),
                );
            } else {
                self.surface
                    .layout_with(slot, Some(c), scale, type_scale, |s, w| {
                        host.shape_surface(s, role, w)
                    });
            }
            // Keep focus coverage on the same arranged surface and actual scale.
            self.backing_scale = scale;
        }
    }

    fn keyboard(&mut self, key: &ContentKeyEvent, allow_submit: bool) -> ContentResponse {
        if !key.down || self.surface.input.disabled {
            return ContentResponse::default();
        }
        let mut result = ContentResponse::default();
        let modifiers = key.modifiers;
        let plain = !modifiers.command && !modifiers.alt && !modifiers.control;
        match key.key {
            Some(ControlKey::Enter) => {
                if self.surface.input.marked.is_some() {
                    // The input method owns an unresolved raw Enter. The host
                    // composition barrier already isolates overlays/navigation.
                    return ContentResponse::default();
                }
                if self.surface.input.read_only {
                    return ContentResponse::consumed(false);
                }
                let submit = allow_submit
                    && !modifiers.shift
                    && !modifiers.alt
                    && !modifiers.control
                    && match self.submit_policy {
                        TextSubmitPolicy::None => false,
                        TextSubmitPolicy::Enter => !modifiers.command,
                        TextSubmitPolicy::CommandEnter => modifiers.command,
                    };
                if submit {
                    return ContentResponse {
                        activated: !key.repeat,
                        ..ContentResponse::consumed(false)
                    };
                }
                result = ContentResponse::consumed(
                    plain
                        && self
                            .surface
                            .command(TextEditCommand::InsertNewline)
                            .unwrap_or(false),
                );
            }
            Some(ControlKey::Tab { reverse })
                if self.tab_policy == TextTabPolicy::Insert
                    && self.surface.input.multiline
                    && plain =>
            {
                result = ContentResponse::consumed(
                    !reverse
                        && !self.surface.input.read_only
                        && self.surface.input.marked.is_none()
                        && self.surface.input.insert("\t", None),
                );
            }
            Some(ControlKey::Left | ControlKey::Right | ControlKey::Up | ControlKey::Down) => {
                if self.surface.input.marked.is_some() {
                    return ContentResponse::default();
                }
                if let Some(reading) = &mut self.reading {
                    reading.stop_following(&mut self.surface);
                }
                let command = match key.key {
                    Some(ControlKey::Left) if modifiers.command => {
                        TextEditCommand::MoveToBeginningOfLine
                    }
                    Some(ControlKey::Right) if modifiers.command => {
                        TextEditCommand::MoveToEndOfLine
                    }
                    Some(ControlKey::Left) if modifiers.alt => TextEditCommand::MoveWordLeft,
                    Some(ControlKey::Right) if modifiers.alt => TextEditCommand::MoveWordRight,
                    Some(ControlKey::Left) => TextEditCommand::MoveLeft,
                    Some(ControlKey::Right) => TextEditCommand::MoveRight,
                    Some(ControlKey::Up) if modifiers.command => {
                        TextEditCommand::MoveToBeginningOfDocument
                    }
                    Some(ControlKey::Down) if modifiers.command => {
                        TextEditCommand::MoveToEndOfDocument
                    }
                    Some(ControlKey::Up | ControlKey::Down)
                        if modifiers.alt || modifiers.control =>
                    {
                        return ContentResponse::default();
                    }
                    Some(ControlKey::Up) => TextEditCommand::MoveUp,
                    Some(ControlKey::Down) => TextEditCommand::MoveDown,
                    _ => unreachable!(),
                };
                let changed = self
                    .surface
                    .command(command.with_extend_selection(modifiers.shift));
                result = ContentResponse {
                    consumed: changed.is_some(),
                    changed: changed.unwrap_or(false),
                    ..Default::default()
                };
            }
            Some(ControlKey::Tab { .. }) => {}
            Some(control) => {
                return self.input_event(
                    &ContentEvent::Key {
                        key: control,
                        down: key.down,
                    },
                    None,
                );
            }
            None => {}
        }
        if result.changed {
            self.surface.reveal();
        }
        result
    }

    pub(super) fn captured_wheel_event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.input_event_with_wheel_capture(e, None, true)
    }

    fn input_event(&mut self, e: &ContentEvent, pointer: Option<&PointerEvent>) -> ContentResponse {
        self.input_event_with_wheel_capture(e, pointer, false)
    }

    fn input_event_with_wheel_capture(
        &mut self,
        e: &ContentEvent,
        pointer: Option<&PointerEvent>,
        captured_wheel: bool,
    ) -> ContentResponse {
        if matches!(e, ContentEvent::PointerDown(_)) {
            let event =
                pointer.map_or_else(|| e.clone(), |pointer| ContentEvent::Pointer(*pointer));
            let Some(decision) =
                self.decide_action(&action_preparation::OwnedActionRequest::Pointer(event))
            else {
                return ContentResponse::default();
            };
            let response = decision.response;
            self.apply_action(decision);
            return response;
        }

        let mut r = ContentResponse::default();
        if self.surface.input.disabled && !matches!(e, ContentEvent::Cancel) {
            return r;
        }
        match e {
            ContentEvent::PointerMove(p) => r.changed = self.surface.pointer_drag(*p, 24.),
            ContentEvent::PointerUp(_) => {
                self.surface.pointer_up();
                r = ContentResponse::consumed(true);
            }
            ContentEvent::Commit { text, replacement } => {
                r = ContentResponse::consumed(self.surface.input.insert(text, replacement.clone()));
                self.surface.clauses.clear();
            }
            ContentEvent::Preedit {
                text,
                selection,
                clauses,
                replacement,
            } => {
                r = ContentResponse::consumed(self.surface.input.preedit(
                    text,
                    selection.clone(),
                    replacement.clone(),
                ));
                self.surface.clauses = clauses.clone();
            }
            ContentEvent::Unmark => {
                self.surface.input.unmark();
                self.surface.clauses.clear();
                r = ContentResponse::consumed(true);
            }
            ContentEvent::Key {
                key: ControlKey::Enter | ControlKey::Tab { .. },
                ..
            } => {
                let ContentEvent::Key { key, down } = e else {
                    unreachable!()
                };
                return self.keyboard(
                    &ContentKeyEvent {
                        key: Some(*key),
                        down: *down,
                        repeat: false,
                        modifiers: InputModifiers::default(),
                        native_code: 0,
                        characters: String::new(),
                        characters_ignoring_modifiers: String::new(),
                    },
                    false,
                );
            }
            ContentEvent::TextCommand(c) => {
                if (c.is_movement() || *c == TextEditCommand::SelectAll)
                    && let Some(reading) = &mut self.reading
                {
                    reading.stop_following(&mut self.surface);
                }
                let changed = self.surface.command(*c);
                r = ContentResponse {
                    consumed: changed.is_some(),
                    changed: changed.unwrap_or(false),
                    ..Default::default()
                };
            }

            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            } if self.surface.input.marked.is_some() => {
                let changed = self.surface.input.cancel_composition();
                self.surface.clauses.clear();
                r = ContentResponse::consumed(changed);
            }
            ContentEvent::Key {
                key:
                    key @ (ControlKey::Left | ControlKey::Right | ControlKey::Up | ControlKey::Down),
                down: true,
            } => {
                if let Some(reading) = &mut self.reading {
                    reading.stop_following(&mut self.surface);
                }
                let command = match key {
                    ControlKey::Left => TextEditCommand::MoveLeft,
                    ControlKey::Right => TextEditCommand::MoveRight,
                    ControlKey::Up => TextEditCommand::MoveUp,
                    ControlKey::Down => TextEditCommand::MoveDown,
                    _ => unreachable!(),
                };
                let changed = self.surface.command(command);
                r = ContentResponse {
                    consumed: changed.is_some(),
                    changed: changed.unwrap_or(false),
                    ..Default::default()
                };
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
                if self.surface.contains_visible(*position)
                    && let Some(reading) = &mut self.reading
                {
                    reading.observe_scroll(
                        &mut self.surface,
                        Point::new(-delta.x, -delta.y),
                        false,
                    );
                }
                let wheel = if captured_wheel {
                    TextSurface::wheel_captured_at
                } else {
                    TextSurface::wheel_at
                };
                let result = wheel(
                    &mut self.surface,
                    *position,
                    Point::new(-delta.x, -delta.y),
                    *precise,
                    *begin,
                    *end,
                    *momentum,
                    *time,
                    false,
                );
                r.changed = result.unwrap_or(false);
                r.consumed = result.is_some();
            }
            ContentEvent::Cancel => self.surface.cancel_interaction(),
            _ => {}
        }
        // Wheel scrolling owns the viewport; revealing the old insertion caret
        // here would undo a user's scroll, including read-only history reading.
        if r.changed && !matches!(e, ContentEvent::Scroll { .. }) {
            self.surface.reveal();
        }
        r
    }
}
