//! Original non-editable NumberField model/view in the receiving ContentHost.
//! The newer editable NumberFieldContent has a different public contract, so this
//! Gallery adapter retains the existing stepper instead of changing the story.
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey, NumberField,
    composition::*,
    feedback_view::{NumberFieldEvent, NumberFieldView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
#[derive(Debug)]
pub(super) struct Stepper {
    pub model: NumberField,
    view: NumberFieldView,
    disabled: bool,
}
impl Stepper {
    pub fn new(model: NumberField, disabled: bool) -> Self {
        let mut view = NumberFieldView::default();
        view.set_disabled(disabled);
        Self {
            model,
            view,
            disabled,
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.view.set_disabled(disabled);
    }
    fn key(&mut self, key: ControlKey, down: bool) -> ContentResponse {
        if self.disabled {
            return ContentResponse::default();
        }
        if let ControlKey::Tab { reverse } = key {
            if down
                && self.view.focused_index() == Some(reverse)
                && self.model.can_increment(!reverse)
            {
                self.view.focus(!reverse, true);
                return ContentResponse::consumed(true);
            }
            return ContentResponse::default();
        }
        let changed = if down
            && matches!(
                key,
                ControlKey::Left | ControlKey::Right | ControlKey::Up | ControlKey::Down
            ) {
            self.view.key(&mut self.model, key) == NumberFieldEvent::Changed
        } else if down && matches!(key, ControlKey::Home | ControlKey::End) {
            self.model
                .set(if key == ControlKey::Home { 1. } else { 32. })
        } else {
            self.view.key_event(&mut self.model, key, down) == NumberFieldEvent::Changed
        };
        ContentResponse {
            consumed: matches!(
                key,
                ControlKey::Left
                    | ControlKey::Right
                    | ControlKey::Up
                    | ControlKey::Down
                    | ControlKey::Home
                    | ControlKey::End
                    | ControlKey::Enter
                    | ControlKey::Space
            ),
            changed,
            activated: changed,
            ..Default::default()
        }
    }
}
impl Component for Stepper {
    fn rectangular_input_geometry(&self) -> bool {
        // The original view publishes parts from this Host's frame and clip.
        true
    }

    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", "NumberField"),
            InspectionProperty::new("value", self.model.value().to_string()),
            InspectionProperty::new("range", "1..32; step 1"),
            InspectionProperty::new("editable", "false"),
        ])
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let h = text
            .measure(
                "32",
                tekes_ui_tokens::proposal::TextRole::UiBody,
                available.width,
            )
            .height
            .max(36.);
        MeasuredChild::new(Size::new(available.width.min(198.), h))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(
            scene,
            &mut text,
            &self.model,
            frame,
            Some(clip),
            theme.palette,
            tekes_ui_tokens::proposal::TextRole::UiBody,
        );
    }
    fn focusable(&self) -> bool {
        !self.disabled
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(!self.model.can_increment(false), visible);
        } else {
            self.view.blur();
        }
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn semantics(&self) -> ContentSemantics {
        let mut result = ContentSemantics::new(ContentRole::Group, "Bounded numeric stepper");
        result.value = Some(self.model.value().to_string());
        result.disabled = self.disabled;
        result.description = Some("Range 1 to 32; step 1. Read-only numeric value.".into());
        result
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let layout = self.view.layout();
        [
            (false, "Decrease", layout.decrement),
            (true, "Increase", layout.increment),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (up, name, frame))| {
            let mut semantics = ContentSemantics::new(ContentRole::Button, name);
            semantics.disabled = self.disabled || !self.model.can_increment(up);
            SemanticPart {
                index,
                frame: frame.frame,
                focused: self.view.focused_index() == Some(up),
                semantics,
            }
        })
        .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if self.disabled {
            return ContentResponse::default();
        }
        let direction = match (part, action) {
            (Some(0), AccessibilityAction::Press | AccessibilityAction::Focus)
            | (None, AccessibilityAction::Decrement) => Some(false),
            (Some(1), AccessibilityAction::Press | AccessibilityAction::Focus)
            | (None, AccessibilityAction::Increment) => Some(true),
            _ => None,
        };
        let Some(up) = direction else {
            return ContentResponse::default();
        };
        if !self.model.can_increment(up) {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Focus) {
            self.view.focus(up, true);
            return ContentResponse::consumed(true);
        }
        let changed = self.model.increment(up);
        ContentResponse {
            consumed: changed,
            changed,
            activated: changed,
            ..Default::default()
        }
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        let p = pointer.position;
        self.event(&match pointer.phase {
            PointerPhase::Down => ContentEvent::PointerDown(p),
            PointerPhase::Up => ContentEvent::PointerUp(p),
            _ => ContentEvent::PointerMove(p),
        })
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if self.disabled {
            return ContentResponse::default();
        }
        match *event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(p);
                ContentResponse::consumed(true)
            }
            ContentEvent::PointerDown(p) => {
                let target = self.view.pointer_down(p);
                if let Some(up) = target {
                    self.view.focus(up, false);
                }
                ContentResponse {
                    consumed: target.is_some(),
                    capture_pointer: target.is_some(),
                    changed: target.is_some(),
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) => {
                let changed = self.view.pointer_up(p, &mut self.model) == NumberFieldEvent::Changed;
                ContentResponse {
                    consumed: true,
                    changed,
                    activated: changed,
                    ..Default::default()
                }
            }
            ContentEvent::Key { key, down } => self.key(key, down),
            ContentEvent::Cancel => {
                self.view.cancel();
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        key.key
            .map_or_else(ContentResponse::default, |code| self.key(code, key.down))
    }
    fn window_active_changed(&mut self, active: bool) {
        if !active {
            self.view.cancel();
        }
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        if !runnable {
            self.view.cancel();
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.view.blur();
        }
    }
}

pub(super) fn icon_button(state: tekes_ui_controls::ControlState) -> Button {
    let mut button = Button::new(Content::new(
        ComponentId(122001),
        IconContent::new("tekes-new").expect("canonical plus glyph"),
    ))
    .with_accessible_name("Create item");
    button.button.state = state;
    button
}
#[derive(Debug)]
pub(super) struct IconFixture(Button);
impl IconFixture {
    pub(super) fn new(state: tekes_ui_controls::ControlState) -> Self {
        Self(icon_button(state))
    }
}
impl Component for IconFixture {
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.0.children()[0]
            .component::<IconContent>()
            .expect("original icon slot")
            .icon_error()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.0
            .measure(&mut super::apphost_basic::FixtureText(text), available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.0.arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint(
            scene,
            &mut super::apphost_basic::FixtureText(text),
            frame,
            clip,
            theme,
        );
    }
}
