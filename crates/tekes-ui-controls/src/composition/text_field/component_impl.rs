use super::super::*;
use super::{CachedPlaceholder, FieldActivationKind, TextFieldContent};
use crate::{TextSurface, TextSurfacePresentation};

// Original-owner effects used both by the host's atomic admission and the
// ordinary immediate entry points. No component state lives in the decision.
#[derive(Debug, PartialEq)]
pub(super) enum PreparedFieldEffect {
    Pointer(Box<crate::text_surface::PreparedTextPointer>),
    Selection { anchor: usize, caret: usize },
}

#[derive(Debug, PartialEq)]
pub(in crate::composition) struct PreparedFieldAction {
    pub(super) focus_rect: Option<Rect>,
    pub(super) effect: PreparedFieldEffect,
    pub(in crate::composition) response: ContentResponse,
    input: crate::text_input::TextActionInputSnapshot,
    focused: bool,
}

impl TextFieldContent {
    pub(in crate::composition) fn decide_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedFieldAction> {
        if self.surface.input.disabled {
            return None;
        }
        let (effect, response) = match request {
            action_preparation::OwnedActionRequest::Pointer(event) => {
                let routed = event.routing_event();
                let ContentEvent::PointerDown(point) = routed.as_ref() else {
                    return None;
                };
                let pointer = if let ContentEvent::Pointer(pointer) = event {
                    Some(pointer)
                } else {
                    None
                };
                if pointer.is_some_and(|pointer| !pointer.is_primary()) {
                    return None;
                }
                let decision = self.surface.prepare_pointer_down(
                    *point,
                    pointer.is_some_and(|pointer| pointer.modifiers.shift),
                    pointer.map_or(1, |pointer| pointer.click_count),
                    24.,
                )?;
                let response = ContentResponse {
                    consumed: decision.accepted,
                    changed: decision.accepted,
                    capture_pointer: decision.accepted,
                    ..Default::default()
                };
                (PreparedFieldEffect::Pointer(Box::new(decision)), response)
            }
            action_preparation::OwnedActionRequest::Accessibility(
                part,
                AccessibilityAction::SetSelection(range),
            ) => {
                if part.is_some()
                    || self.surface.input.marked.is_some()
                    || range.start > range.end
                    || range.end > self.surface.input.len()
                    || !self.surface.input.can_advance_edit_epochs(1)
                {
                    return None;
                }
                let range = self.surface.input.normalized_selection(range.clone());
                let changed = self.surface.input.selection() != range;
                (
                    PreparedFieldEffect::Selection {
                        anchor: range.start,
                        caret: range.end,
                    },
                    ContentResponse::consumed(changed),
                )
            }
            _ => return None,
        };
        let focus_rect = if self.surface.presentation == TextSurfacePresentation::Content {
            match &effect {
                PreparedFieldEffect::Pointer(pointer) => pointer.focus_rect,
                PreparedFieldEffect::Selection { caret, .. } => {
                    Some(self.surface.decided_selection_focus_rect(
                        *caret,
                        None,
                        response.changed && self.surface.caret_reveal_allowed(),
                    ))
                }
            }
        } else {
            None
        };
        Some(PreparedFieldAction {
            focus_rect,
            effect,
            response,
            input: self.surface.input.action_snapshot(),
            focused: self.focused,
        })
    }

    pub(in crate::composition) fn apply_action(&mut self, decision: PreparedFieldAction) {
        match decision.effect {
            PreparedFieldEffect::Pointer(pointer) => {
                if !pointer.accepted {
                    return;
                }
                let reveal = pointer.reveal;
                if let Some(reading) = &mut self.reading {
                    reading.stop_following(&mut self.surface);
                }
                self.surface.apply_pointer_down(*pointer);
                if reveal {
                    self.surface.reveal();
                }
            }
            PreparedFieldEffect::Selection { anchor, caret } => {
                self.surface
                    .input
                    .apply_decided_selection(anchor, caret, 1, false);
                if decision.response.changed {
                    self.surface.reveal();
                }
            }
        }
    }
}

impl action_preparation::OriginalActionDecision for PreparedFieldAction {
    fn focus_rect(&self) -> Option<Rect> {
        self.focus_rect
    }
}

impl TextFieldContent {
    pub(in crate::composition) fn prepare_field_action(
        &self,
        request: ComponentActionRequest<'_>,
    ) -> ActionPreparation {
        let owned = action_preparation::OwnedActionRequest::read(request);
        if self.recognizes_clear(&owned) && self.decide_prepared_action(&owned).is_none() {
            return action_preparation::rejected();
        }
        action_preparation::prepare_original(
            self,
            request,
            Self::decide_prepared_action,
            Self::apply_prepared_action,
            |decision| (decision.response, decision.focus),
        )
    }
    pub(in crate::composition) fn prepare_field_effects_work(
        &self,
        n: usize,
    ) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)?
            .checked_add(
                self.surface
                    .input
                    .text
                    .len()
                    .checked_add(self.field.label.len())
                    .and_then(|v| v.checked_add(self.field.message().len()))
                    .and_then(|v| v.checked_mul(n.max(1)))
                    .ok_or(PreparedEffectsError::CounterCapacity)?,
            )
            .ok_or(PreparedEffectsError::CounterCapacity)
    }
    pub(in crate::composition) fn prepare_field_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        let mut focused = self.focused;
        let mut unmark = false;
        for h in hooks {
            if let PreparedHookKind::Focus { focused: next, .. } = h.kind {
                if focused && !next {
                    unmark = true;
                }
                focused = next;
            }
        }
        if !self.surface.input.preflight_unmark(unmark) {
            return Err(PreparedEffectsError::CounterCapacity);
        }
        Ok(PreparedEffects::local(self, hooks))
    }
}
impl Component for TextFieldContent {
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let mut properties = self.editor_inspection_properties()?;
        properties[0] = InspectionProperty::new("component", "TextFieldContent");
        properties.push(InspectionProperty::new(
            "placeholder",
            self.placeholder.as_deref().unwrap_or(""),
        ));
        Some(properties)
    }

    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn validate(&self) -> Result<(), CompositionError> {
        self.clear.validate()?;
        Ok(())
    }

    fn requires_text_services(&self) -> bool {
        self.editor_requires_text_services()
    }

    fn text_surface(&self) -> Option<&TextSurface> {
        self.editor_text_surface()
    }

    fn composing(&self) -> bool {
        self.editor_composing()
    }

    fn focusable(&self) -> bool {
        self.editor_focusable()
    }

    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.editor_set_focus(focused, visible);
        self.focused = focused;
        if !focused {
            self.cancel_gesture();
        }
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        self.last_activation = None;
        self.cancel_gesture();

        if event == Lifecycle::Unmounted {
            self.editor_set_focus(false, false);
            self.focused = false;
            self.clear_clip = None;
        }
    }

    fn semantics(&self) -> ContentSemantics {
        self.editor_semantics()
    }

    fn semantic_frame(&self, _: Rect) -> Rect {
        self.surface.outer
    }

    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.clear_visible() {
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
        match self.last_activation.as_ref().map(|a| a.kind) {
            Some(FieldActivationKind::Submit) => vec![ContentActionKind::Submit],
            Some(FieldActivationKind::Clear) => vec![ContentActionKind::Clear],
            None => Vec::new(),
        }
    }

    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        None
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
                self.accept_activation(FieldActivationKind::Clear)
            } else {
                ContentResponse::default()
            };
        }
        let response = self.editor_accessibility_action(part, action);
        self.finish_editor(response)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.editor_measure(text, available)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        // The original editor owns the entire input chrome. Its text/selection/
        // IME viewport alone reserves the internal trailing action's space.
        let slot = self.field_layout(text, frame, clip).slot;
        let inset = tekes_ui_tokens::Spacing::Micro.value();
        let edge = (slot.size.height.min(frame.size.height) - 2. * inset).max(0.);
        let width = if !self.clear_visible() {
            0.
        } else {
            edge.min((slot.size.width - 2. * inset).max(0.))
        };
        self.clear_frame = Rect::new(
            slot.max_x() - inset - width,
            slot.min_y() + inset,
            width,
            edge,
        );
        self.clear_clip = self
            .clear_frame
            .intersection(&clip)
            .filter(|r| !r.is_empty());
        self.surface.trailing_action_width = if width > 0. { width + inset } else { 0. };
        self.editor_frame = self.effective_frame(text, frame);
        self.editor_arrange(text, frame, clip);
        self.clear.button.frame_policy = crate::ButtonFramePolicy::Exact;
        if self.clear_clip.is_some() {
            self.clear.arrange(text, self.clear_frame, clip);
        } else {
            // Empty, multiline and fully clipped fields have no Clear slot.
            // Do not ask a text backend to measure an invisible zero-width label.
            self.clear.button.clear_layout();
            self.clear_captured = false;
            self.clear_baseline = None;
        }
    }

    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        self.editor_prepare_text_input(text, self.editor_frame, clip);
    }

    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.sync_clear();
        self.editor_paint(scene, text, frame, clip, theme);
        if self.clear_visible() && self.clear_clip.is_some() {
            self.clear.paint(scene, text, self.clear_frame, clip, theme);
        }
        if self.text().is_empty()
            && !self.focused
            && self.placeholder.is_some()
            && let Some(visible) = self
                .surface
                .visible_content()
                .and_then(|vc| vc.intersection(&clip))
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
                    error: self.surface.error,
                    read_only: self.input().read_only,
                },
            )
            .expect("validated original Field Surface")
            .helper
            .expect("original input helper ink");
            let backing_scale = if text.backing_scale().is_finite() {
                text.backing_scale().max(1.)
            } else {
                1.
            };
            let wrap_width = (self.surface.input.multiline && self.surface.wrap).then(|| {
                ((self.surface.inner.size.width * backing_scale).floor() / backing_scale).max(1.)
            });
            let inner = self.surface.inner;
            let role = self.surface.role;
            let multiline = self.surface.input.multiline;
            let alignment = self.surface.alignment;
            if let Some(host) = text.text_surface_host()
                && let Some(placeholder) = self.placeholder.as_deref()
            {
                let type_scale = host.type_scale();
                let width_bits = wrap_width.map(f32::to_bits);
                let type_scale_bits = type_scale.to_bits();
                let cached = self.placeholder_cache.as_ref().is_some_and(|c| {
                    c.text == placeholder
                        && c.role == role
                        && c.width_bits == width_bits
                        && c.type_scale_bits == type_scale_bits
                });
                if !cached {
                    let paragraph = host.shape_surface(placeholder, role, wrap_width);
                    self.placeholder_cache = Some(CachedPlaceholder {
                        text: placeholder.to_owned(),
                        role,
                        width_bits,
                        type_scale_bits,
                        paragraph,
                    });
                }
                let paragraph = &self
                    .placeholder_cache
                    .as_ref()
                    .expect("populated placeholder cache")
                    .paragraph;
                let base = paragraph.origin_in(inner, !multiline);
                let align_dx = if !multiline && alignment != crate::TextSurfaceAlignment::Leading {
                    (inner.size.width
                        - paragraph.width as f32
                        - tekes_ui_tokens::Stroke::Rule.value())
                    .max(0.)
                        * if alignment == crate::TextSurfaceAlignment::Center {
                            0.5
                        } else {
                            1.0
                        }
                } else {
                    0.
                };
                let origin = Point::new((base.x + align_dx).round(), base.y.round());
                scene.push_clip(visible);
                host.push_visible(scene, paragraph, origin, ink, visible);
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
                self.clear_baseline = Some(self.surface.input.external_baseline());
            }
            if pointer.phase == PointerPhase::Up {
                self.clear_captured = false;
                self.clear_baseline = None;
            }
            if response.activated {
                let accepted = self.accept_activation(FieldActivationKind::Clear);
                response.activated = accepted.activated;
                response.changed |= accepted.changed;
            }
            return response;
        }
        // The compound field owns delivery to its auxiliary button, including
        // outside moves that retire hover. Only its redraw joins the editor's
        // response; capture, focus and activation stay with the routed owner.
        let clear_changed =
            pointer.phase == PointerPhase::Move && self.clear.pointer_event(pointer).changed;
        let mut response = self.editor_pointer_event(pointer);
        response.changed |= clear_changed;
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
        let response = self.editor_key_event(key);
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
        let clear_changed = matches!(event, ContentEvent::Cancel) && self.cancel_gesture();
        // Keep legacy raw Key and native TextCommand behavior in the original
        // editor. In particular raw Key Enter must not enable submission.
        let mut response = self.editor_event(event);
        response.changed |= clear_changed;
        self.finish_editor(response)
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
}
