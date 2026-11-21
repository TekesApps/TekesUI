//! Canonical non-editable bounded numeric Stepper over `NumberField` and `NumberFieldView`.
use super::*;
use crate::ButtonModel;
use crate::ButtonTextRenderer;
use crate::feedback_view::{NumberFieldEvent, NumberFieldView};
use crate::{ButtonSize, ControlKey, NumberField};
use std::cell::RefCell;
use std::rc::Rc;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::proposal;

#[derive(Debug)]
struct StepperState {
    model: NumberField,
    view: NumberFieldView,
    disabled: bool,
}

impl StepperState {
    fn step_enabled(&self, increment: bool) -> bool {
        !self.disabled && self.model.can_increment(increment)
    }

    fn key_single(&mut self, key: ControlKey, down: bool) -> ContentResponse {
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
            self.model.set(if key == ControlKey::Home {
                self.model.min()
            } else {
                self.model.max()
            })
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

/// Non-editable bounded numeric stepper backed by `NumberField` and `NumberFieldView`.
///
/// Supports either a single focusable host node exposing two `SemanticPart`s
/// (`StepperContent::new`), or two mounted child step buttons (`StepperContent::with_step_ids`)
/// sharing one `NumberField` model and `NumberFieldView`.
#[derive(Debug)]
pub struct StepperContent {
    name: String,
    description: Option<String>,
    state: Rc<RefCell<StepperState>>,
    child_mode: bool,
    children: Vec<Content>,
}

impl StepperContent {
    /// Create a single-node Stepper publishing `"Decrease"` and `"Increase"` `SemanticPart`s.
    pub fn new(name: impl Into<String>, model: NumberField) -> Self {
        Self {
            name: name.into(),
            description: None,
            state: Rc::new(RefCell::new(StepperState {
                model,
                view: NumberFieldView::default(),
                disabled: false,
            })),
            child_mode: false,
            children: Vec::new(),
        }
    }

    /// Create a Stepper mounting separate decrement and increment `Content` button nodes.
    pub fn with_step_ids(
        name: impl Into<String>,
        model: NumberField,
        decrement_id: ComponentId,
        increment_id: ComponentId,
    ) -> Self {
        let state = Rc::new(RefCell::new(StepperState {
            model,
            view: NumberFieldView::default(),
            disabled: false,
        }));
        let children = vec![
            Content::new(
                decrement_id,
                StepperStep {
                    increment: false,
                    state: state.clone(),
                },
            ),
            Content::new(
                increment_id,
                StepperStep {
                    increment: true,
                    state: state.clone(),
                },
            ),
        ];
        Self {
            name: name.into(),
            description: None,
            state,
            child_mode: true,
            children,
        }
    }

    pub fn with_disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self.state.borrow_mut().view.set_disabled(disabled);
        self
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        let mut state = self.state.borrow_mut();
        state.disabled = disabled;
        state.view.set_disabled(disabled);
    }

    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    pub fn model(&self) -> NumberField {
        self.state.borrow().model
    }

    pub fn value(&self) -> f64 {
        self.state.borrow().model.value()
    }

    pub fn set_value(&mut self, value: f64) -> bool {
        self.state.borrow_mut().model.set(value)
    }

    pub fn increment(&mut self, up: bool) -> bool {
        let mut state = self.state.borrow_mut();
        if state.disabled {
            return false;
        }
        state.model.increment(up)
    }
}

impl Component for StepperContent {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        if self.child_mode {
            CurveSubtreeCapability::Recursive
        } else {
            CurveSubtreeCapability::Local
        }
    }

    fn has_stable_hooks(&self) -> bool {
        true
    }

    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        if self.child_mode && self.children.len() != 2 {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.child_mode.then_some(&mut self.children)
    }

    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let state = self.state.borrow();
        Some(vec![
            InspectionProperty::new("component", "Stepper"),
            InspectionProperty::new("value", state.model.value().to_string()),
            InspectionProperty::new(
                "range",
                format!(
                    "{}..{}; step {}",
                    state.model.min(),
                    state.model.max(),
                    state.model.step()
                ),
            ),
            InspectionProperty::new("editable", "false"),
        ])
    }

    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let state = self.state.borrow();
        let action = ButtonModel::measure(
            &mut text,
            "+",
            48.,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let max_sample = state.model.max().to_string();
        let min_sample = state.model.min().to_string();
        let value_h = text
            .measure(&max_sample, proposal::TextRole::UiHeading, 80.)
            .height
            .max(
                text.measure(&min_sample, proposal::TextRole::UiHeading, 80.)
                    .height,
            );
        let row = proposal::metric("size", "size-button-md").unwrap_or(36.);
        let h = action.height.max(value_h).max(row);
        MeasuredChild::new(Size::new(available.width.min(198.), h))
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if !self.child_mode {
            return;
        }
        let (minus, _, plus) = NumberFieldView::slot_frames(frame);
        self.children[0].arrange(text, minus, clip);
        self.children[1].arrange(text, plus, clip);
    }

    fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let mut state = self.state.borrow_mut();
        let _ = state.view.set_surface(theme.surface());
        let StepperState { model, view, .. } = &mut *state;
        view.render(
            scene,
            &mut text,
            model,
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
        );
    }

    fn focusable(&self) -> bool {
        !self.child_mode && !self.state.borrow().disabled
    }

    fn set_focus(&mut self, focused: bool, visible: bool) {
        if self.child_mode {
            return;
        }
        let mut state = self.state.borrow_mut();
        if focused && !state.disabled {
            let prefer_plus = !state.model.can_increment(false);
            state.view.focus(prefer_plus, visible);
        } else {
            state.view.blur();
        }
    }

    fn semantics(&self) -> ContentSemantics {
        let state = self.state.borrow();
        let mut result = ContentSemantics::new(ContentRole::Group, &self.name);
        result.value = Some(state.model.value().to_string());
        result.disabled = state.disabled;
        result.description = self.description.clone();
        result
    }

    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if self.child_mode {
            return Vec::new();
        }
        let state = self.state.borrow();
        let layout = state.view.layout();
        [
            (false, "Decrease", layout.decrement),
            (true, "Increase", layout.increment),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (up, name, frame))| {
            let mut semantics = ContentSemantics::new(ContentRole::Button, name);
            semantics.disabled = !state.step_enabled(up);
            SemanticPart {
                index,
                frame: frame.frame,
                focused: state.view.focused_index() == Some(up),
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
        if self.child_mode {
            return ContentResponse::default();
        }
        let mut state = self.state.borrow_mut();
        if state.disabled {
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
        if !state.model.can_increment(up) {
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Focus) {
            state.view.focus(up, true);
            return ContentResponse::consumed(true);
        }
        let changed = state.model.increment(up);
        ContentResponse {
            consumed: changed,
            changed,
            activated: changed,
            ..Default::default()
        }
    }

    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if self.child_mode || !pointer.is_primary() {
            return ContentResponse::default();
        }
        let p: Point = pointer.position;
        self.event(&match pointer.phase {
            PointerPhase::Down => ContentEvent::PointerDown(p),
            PointerPhase::Up => ContentEvent::PointerUp(p),
            _ => ContentEvent::PointerMove(p),
        })
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if self.child_mode {
            return ContentResponse::default();
        }
        let mut state = self.state.borrow_mut();
        if state.disabled {
            return ContentResponse::default();
        }
        match *event {
            ContentEvent::PointerMove(p) => {
                state.view.pointer_move(p);
                ContentResponse::consumed(true)
            }
            ContentEvent::PointerDown(p) => {
                let target = state.view.pointer_down(p);
                if let Some(up) = target {
                    state.view.focus(up, false);
                }
                ContentResponse {
                    consumed: target.is_some(),
                    capture_pointer: target.is_some(),
                    changed: target.is_some(),
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) => {
                let StepperState { model, view, .. } = &mut *state;
                let changed = view.pointer_up(p, model) == NumberFieldEvent::Changed;
                ContentResponse {
                    consumed: true,
                    changed,
                    activated: changed,
                    ..Default::default()
                }
            }
            ContentEvent::Key { key, down } => state.key_single(key, down),
            ContentEvent::Cancel => {
                state.view.cancel();
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }

    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if self.child_mode {
            return ContentResponse::default();
        }
        key.key.map_or_else(ContentResponse::default, |code| {
            self.state.borrow_mut().key_single(code, key.down)
        })
    }

    fn window_active_changed(&mut self, active: bool) {
        if !active {
            self.state.borrow_mut().view.cancel();
        }
    }

    fn presentation_runnable_changed(&mut self, runnable: bool) {
        if !runnable {
            self.state.borrow_mut().view.cancel();
        }
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.state.borrow_mut().view.blur();
        }
    }
}

#[derive(Debug)]
struct StepperStep {
    increment: bool,
    state: Rc<RefCell<StepperState>>,
}

impl StepperStep {
    fn activate(&mut self) -> ContentResponse {
        let mut state = self.state.borrow_mut();
        if !state.step_enabled(self.increment) {
            return ContentResponse::default();
        }
        let StepperState { model, view, .. } = &mut *state;
        view.focus(self.increment, false);
        let changed = view.key_event(model, ControlKey::Enter, true) == NumberFieldEvent::Changed;
        ContentResponse {
            consumed: true,
            changed,
            activated: changed,
            ..Default::default()
        }
    }
}

impl Component for StepperStep {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }

    fn has_stable_hooks(&self) -> bool {
        true
    }

    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn rectangular_input_geometry(&self) -> bool {
        true
    }

    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }

    fn focusable(&self) -> bool {
        self.state.borrow().step_enabled(self.increment)
    }

    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut state = self.state.borrow_mut();
        if focused && state.step_enabled(self.increment) {
            state.view.focus(self.increment, visible);
        } else if state.view.focused_index() == Some(self.increment) {
            state.view.blur();
        }
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.set_focus(false, false);
        }
    }

    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(
            ContentRole::Button,
            if self.increment {
                "Increase"
            } else {
                "Decrease"
            },
        );
        semantics.disabled = !self.state.borrow().step_enabled(self.increment);
        semantics
    }

    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_none() && *action == AccessibilityAction::Press {
            self.activate()
        } else {
            ContentResponse::default()
        }
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let mut state = self.state.borrow_mut();
        let StepperState { model, view, .. } = &mut *state;
        let changed = match event {
            ContentEvent::PointerMove(p) => {
                view.pointer_move(*p);
                return ContentResponse::consumed(true);
            }
            ContentEvent::PointerDown(p) => {
                let accepted = view.pointer_down(*p) == Some(self.increment);
                return ContentResponse {
                    consumed: accepted,
                    changed: accepted,
                    capture_pointer: accepted,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => view.pointer_up(*p, model),
            ContentEvent::Key { key, down } => {
                if *down
                    && matches!(
                        key,
                        ControlKey::Up | ControlKey::Down | ControlKey::Left | ControlKey::Right
                    )
                {
                    view.key(model, *key)
                } else if matches!(key, ControlKey::Enter | ControlKey::Space) {
                    view.key_event(model, *key, *down)
                } else {
                    return ContentResponse::default();
                }
            }
            ContentEvent::KeyInput(key) => {
                let Some(control) = key.key else {
                    return ContentResponse::default();
                };
                if key.down
                    && matches!(
                        control,
                        ControlKey::Up | ControlKey::Down | ControlKey::Left | ControlKey::Right
                    )
                {
                    view.key(model, control)
                } else if matches!(control, ControlKey::Enter | ControlKey::Space) {
                    view.key_event(model, control, key.down)
                } else {
                    return ContentResponse::default();
                }
            }
            ContentEvent::Cancel => {
                view.cancel();
                return ContentResponse::consumed(true);
            }
            _ => return ContentResponse::default(),
        } == NumberFieldEvent::Changed;
        ContentResponse {
            consumed: true,
            changed,
            activated: changed,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonTextAlign;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Theme;

    struct Text(Vec<(String, Rgba)>);
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (value.chars().count() as f32 * 8.).min(width),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            value: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            ink: Rgba,
        ) {
            self.0.push((value.into(), ink));
        }
    }

    #[test]
    fn single_node_stepper_handles_keys_parts_and_bounds() {
        let model = NumberField::new(4., 1., 32., 1.).unwrap();
        let mut host = ContentHost::new(Content::new(
            ComponentId(1),
            StepperContent::new("Bounded numeric stepper", model)
                .with_description("Range 1 to 32; step 1."),
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 240., 48.);
        let mut text = Text(Vec::new());
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.focus(Some(ComponentId(1)), true));
        host.event(&ContentEvent::Key {
            key: ControlKey::End,
            down: true,
        });
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<StepperContent>()
                .unwrap()
                .value(),
            32.
        );
        assert!(
            !host
                .accessibility_action(ComponentId(1), Some(1), AccessibilityAction::Press)
                .changed
        );
        assert!(
            host.accessibility_action(ComponentId(1), Some(0), AccessibilityAction::Press)
                .changed
        );
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<StepperContent>()
                .unwrap()
                .value(),
            31.
        );
        host.event(&ContentEvent::Key {
            key: ControlKey::Home,
            down: true,
        });
        assert_eq!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<StepperContent>()
                .unwrap()
                .value(),
            1.
        );
    }

    #[test]
    fn child_mode_stepper_mounts_buttons_and_respects_panel_surface_ink() {
        let model = NumberField::new(2., 0., 4., 1.).unwrap();
        let content = Content::new(
            ComponentId(10),
            StepperContent::with_step_ids("Quantity", model, ComponentId(11), ComponentId(12)),
        )
        .with_surface("panel")
        .unwrap();
        let mut host = ContentHost::new(content).unwrap();
        let frame = Rect::new(0., 0., 240., 48.);
        let mut text = Text(Vec::new());
        let mut scene = Scene::new();
        host.render(&mut scene, &mut text, frame, frame, Theme::DARK.into())
            .unwrap();
        assert_eq!(
            host.focus_targets()
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![ComponentId(11), ComponentId(12)]
        );
        let inc_center = host.get(ComponentId(12)).unwrap().frame().center();
        assert!(
            host.event(&ContentEvent::PointerDown(inc_center))
                .capture_pointer
        );
        assert!(host.event(&ContentEvent::PointerUp(inc_center)).activated);
        assert_eq!(
            host.get(ComponentId(10))
                .unwrap()
                .component::<StepperContent>()
                .unwrap()
                .value(),
            3.
        );
        let expected_value_ink = tekes_ui_tokens::surface_palette::resolve_surface(
            Theme::DARK.into(),
            "panel",
            "content",
            "rest",
        )
        .unwrap()
        .text;
        assert!(
            text.0
                .iter()
                .any(|(label, ink)| label == "2" && *ink == expected_value_ink)
        );
        let _ = Primitive::Quad;
    }
}
