//! Caller-owned composition of the existing bounded NumberField View.
//! Two ordinary semantic/focus nodes delegate to one original model/View;
//! they do not paint another stepper or implement another activation machine.
use std::{cell::RefCell, rc::Rc};
use tekes_ui_controls::{
    ButtonModel, ButtonSize, ButtonTextRenderer, ControlKey, NumberField,
    composition::*,
    extended::{Axis, Stack},
    feedback_view::{NumberFieldEvent, NumberFieldView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, Theme, proposal};

pub const ROOT: ComponentId = ComponentId(9200);
pub const QUANTITY: ComponentId = ComponentId(9201);
pub const DECREMENT: ComponentId = ComponentId(9202);
pub const INCREMENT: ComponentId = ComponentId(9203);
pub const LIMIT: ComponentId = ComponentId(9211);
pub const LIMIT_DECREMENT: ComponentId = ComponentId(9212);
pub const LIMIT_INCREMENT: ComponentId = ComponentId(9213);
pub const DISABLED: ComponentId = ComponentId(9221);
pub const AFTER: ComponentId = ComponentId(9230);

#[derive(Debug)]
struct State {
    model: NumberField,
    view: NumberFieldView,
    disabled: bool,
}
impl State {
    fn enabled(&self, increment: bool) -> bool {
        !self.disabled && self.model.can_increment(increment)
    }
}
#[derive(Debug)]
struct NumberContent {
    name: String,
    state: Rc<RefCell<State>>,
    children: Vec<Content>,
}
impl Component for NumberContent {
    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let role = proposal::TextRole::UiBody;
        let action = ButtonModel::measure(&mut text, "+", 48., ButtonSize::Body, role);
        let number = text.measure("4", proposal::TextRole::UiHeading, 80.);
        MeasuredChild::new(Size::new(
            198_f32.min(available.width),
            action.height.max(number.height),
        ))
    }
    fn arrange(&mut self, mut text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let mut state = self.state.borrow_mut();
        let State { model, view, .. } = &mut *state;
        // Geometry comes from the same public View. The discarded scene does
        // not replace its actual themed paint below.
        let layout = view.render(
            &mut Scene::new(),
            &mut text,
            model,
            frame,
            Some(clip),
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
        );
        self.children[0].arrange(text, layout.decrement.frame, clip);
        self.children[1].arrange(text, layout.increment.frame, clip);
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
        let State { model, view, .. } = &mut *state;
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
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, &self.name);
        s.value = Some(self.state.borrow().model.value().to_string());
        s
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[derive(Debug)]
struct Step {
    increment: bool,
    state: Rc<RefCell<State>>,
}
impl Step {
    fn activate(&mut self) -> ContentResponse {
        let mut state = self.state.borrow_mut();
        if !state.enabled(self.increment) {
            return ContentResponse::default();
        }
        let State { model, view, .. } = &mut *state;
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
impl Component for Step {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn focusable(&self) -> bool {
        self.state.borrow().enabled(self.increment)
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut state = self.state.borrow_mut();
        if focused && state.enabled(self.increment) {
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
        let mut s = ContentSemantics::new(
            ContentRole::Button,
            if self.increment {
                "Increase"
            } else {
                "Decrease"
            },
        );
        s.disabled = !self.state.borrow().enabled(self.increment);
        s
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
        let State { model, view, .. } = &mut *state;
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

fn number(
    id: ComponentId,
    decrement: ComponentId,
    increment: ComponentId,
    name: &str,
    value: f64,
    disabled: bool,
) -> Content {
    let mut view = NumberFieldView::default();
    view.set_disabled(disabled);
    let state = Rc::new(RefCell::new(State {
        model: NumberField::new(value, 0., 4., 1.).unwrap(),
        view,
        disabled,
    }));
    Content::new(
        id,
        NumberContent {
            name: name.into(),
            state: state.clone(),
            children: vec![
                Content::new(
                    decrement,
                    Step {
                        increment: false,
                        state: state.clone(),
                    },
                ),
                Content::new(
                    increment,
                    Step {
                        increment: true,
                        state,
                    },
                ),
            ],
        },
    )
}
pub fn build() -> Result<ContentHost, CompositionError> {
    ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children: vec![
                Content::new(
                    ComponentId(9209),
                    TextContent {
                        text: "Quantity · bounded stepper".into(),
                        role: proposal::TextRole::UiBody,
                    },
                ),
                number(QUANTITY, DECREMENT, INCREMENT, "Quantity", 2., false),
                Content::new(
                    ComponentId(9219),
                    TextContent {
                        text: "At upper limit".into(),
                        role: proposal::TextRole::UiBody,
                    },
                ),
                number(
                    LIMIT,
                    LIMIT_DECREMENT,
                    LIMIT_INCREMENT,
                    "At upper limit",
                    4.,
                    false,
                ),
                Content::new(
                    ComponentId(9229),
                    TextContent {
                        text: "Disabled quantity".into(),
                        role: proposal::TextRole::UiBody,
                    },
                ),
                number(
                    DISABLED,
                    ComponentId(9222),
                    ComponentId(9223),
                    "Disabled quantity",
                    2.,
                    true,
                ),
                Content::new(AFTER, Button::new("After number")),
            ],
        },
    ))
}
/// Caller changes its authoritative model, without painting as a side effect.
pub fn set_value(host: &mut ContentHost, id: ComponentId, value: f64) -> bool {
    host.get(id)
        .unwrap()
        .component::<NumberContent>()
        .unwrap()
        .state
        .borrow_mut()
        .model
        .set(value)
}
pub fn value(host: &ContentHost, id: ComponentId) -> f64 {
    host.get(id)
        .unwrap()
        .component::<NumberContent>()
        .unwrap()
        .state
        .borrow()
        .model
        .value()
}
