//! Independent caller refresh scenario. Cached options remain usable while a
//! background request loads or fails; validation and initial empty data differ.
use crate::content_status::{Feedback, Status};
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonModel, ButtonSize, ButtonTextRenderer, ControlKey, MenuEvent, Select,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
    layout_view::SelectView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing, proposal};
pub const ROOT: ComponentId = ComponentId(8200);
pub const CHOICE: ComponentId = ComponentId(8201);
pub const FEEDBACK: ComponentId = ComponentId(8202);
#[derive(Debug)]
struct Choice {
    model: Select,
    view: SelectView,
    labels: Vec<String>,
    // Fixture domain values are unique labels; keep identity across data reorder.
    selected_value: Option<String>,
}
impl Choice {
    fn handle(&mut self, event: &ContentEvent) -> ContentResponse {
        let before = (
            self.model.menu.open,
            self.model.selected,
            self.model.menu.items.selected,
        );
        let mut consumed = false;
        let mut result = MenuEvent::None;
        match event {
            ContentEvent::PointerMove(p) => self.view.pointer_move(*p, &mut self.model),
            ContentEvent::PointerDown(p) => consumed = self.view.pointer_down(*p, &mut self.model),
            ContentEvent::PointerUp(p) => {
                result = self.view.pointer_up(*p, &mut self.model);
                consumed = true;
            }
            ContentEvent::Key { key, down } => {
                consumed = matches!(key, ControlKey::Enter | ControlKey::Space)
                    || self.model.menu.open
                        && matches!(
                            key,
                            ControlKey::Up
                                | ControlKey::Down
                                | ControlKey::Home
                                | ControlKey::End
                                | ControlKey::Escape
                        );
                result = self.view.key(*key, *down, &mut self.model);
            }
            ContentEvent::Cancel => self.view.cancel(&mut self.model),
            _ => {}
        }
        if matches!(result, MenuEvent::Activated { .. }) {
            self.selected_value = self.labels.get(self.model.selected).cloned();
        }
        ContentResponse {
            text_key: None,
            consumed,
            changed: before
                != (
                    self.model.menu.open,
                    self.model.selected,
                    self.model.menu.items.selected,
                ),
            activated: matches!(result, MenuEvent::Activated { .. }),
            capture_pointer: consumed && matches!(event, ContentEvent::PointerDown(_)),
        }
    }
}
impl Component for Choice {
    fn measure(&mut self, mut text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let title = self
            .labels
            .get(self.model.selected)
            .map(String::as_str)
            .unwrap_or("");
        MeasuredChild::new(ButtonModel::measure(
            &mut text,
            title,
            available.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let labels: Vec<_> = self.labels.iter().map(String::as_str).collect();
        self.view.render_trigger(
            scene,
            &mut text,
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
        mut text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let labels: Vec<_> = self.labels.iter().map(String::as_str).collect();
        self.view.render_options(
            scene,
            &mut text,
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
    fn overlay_open(&self) -> bool {
        self.model.menu.open
    }
    fn overlay_presented(&self) -> bool {
        self.model.menu.open || self.view.is_presented()
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.handle(event)
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.handle(event)
    }
    fn focusable(&self) -> bool {
        !self.labels.is_empty()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(visible);
        } else {
            self.view.blur(&mut self.model);
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut result = ContentSemantics::new(ContentRole::MenuButton, "Layout selection");
        result.value = self.labels.get(self.model.selected).cloned();
        result.disabled = self.labels.is_empty();
        result.expanded = Some(self.model.menu.open);
        result
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.model.menu.open {
            return Vec::new();
        }
        self.view
            .layout()
            .popup
            .as_ref()
            .map(|popup| {
                popup
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let mut semantics =
                            ContentSemantics::new(ContentRole::MenuItem, &self.labels[index]);
                        semantics.selected = self.model.selected == index;
                        SemanticPart {
                            index,
                            frame: item.frame,
                            focused: self.model.menu.items.selected == index,
                            semantics,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if !matches!(action, AccessibilityAction::Press) || self.labels.is_empty() {
            return ContentResponse::default();
        }
        if let Some(index) = part {
            if !self.model.menu.open
                || self
                    .view
                    .layout()
                    .popup
                    .as_ref()
                    .and_then(|p| p.items.get(index))
                    .is_none_or(|i| i.visible.is_none())
            {
                return ContentResponse::default();
            }
            let activated = matches!(
                self.view.activate_option(index, &mut self.model),
                MenuEvent::Activated { .. }
            );
            if activated {
                self.selected_value = self.labels.get(index).cloned();
            }
            ContentResponse {
                consumed: activated,
                changed: activated,
                activated,
                ..Default::default()
            }
        } else if !self.model.menu.open {
            self.view.focus(true);
            self.view.key(ControlKey::Enter, true, &mut self.model);
            ContentResponse::consumed(self.model.menu.open)
        } else {
            ContentResponse::consumed(false)
        }
    }
}
/// Application-owned refresh status. This is an example, not a built-in async
/// Select protocol; stable content and production model/view remain mounted.
#[derive(Debug)]
pub struct Scenario {
    pub host: ContentHost,
    status: Rc<Cell<Status>>,
}
impl Scenario {
    pub fn new(labels: Vec<String>) -> Result<Self, CompositionError> {
        let status = Rc::new(Cell::new(Status::Ready));
        let mut view = SelectView::default();
        view.set_reduced_motion(true);
        let choice = Choice {
            model: Select::new(labels.len()),
            view,
            selected_value: labels.first().cloned(),
            labels,
        };
        let host = ContentHost::new(Content::new(
            ROOT,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children: vec![
                    Content::new(FEEDBACK, Feedback::new(status.clone())),
                    Content::new(CHOICE, choice),
                ],
                flexible_child: None,
            },
        ))?;
        Ok(Self { host, status })
    }
    /// The existing owned presentation may outlive logical menu dismissal.
    pub fn is_presented(&self) -> bool {
        self.host
            .get(CHOICE)
            .unwrap()
            .component::<Choice>()
            .unwrap()
            .overlay_presented()
    }
    pub fn set_status(&mut self, status: Status) {
        self.status.set(status);
        self.host.get_mut(FEEDBACK);
    }
    pub fn status(&self) -> Status {
        self.status.get()
    }
    pub fn selected(&self) -> usize {
        self.host
            .get(CHOICE)
            .unwrap()
            .component::<Choice>()
            .unwrap()
            .model
            .selected
    }
    pub fn open(&self) -> bool {
        self.host
            .get(CHOICE)
            .unwrap()
            .component::<Choice>()
            .unwrap()
            .model
            .menu
            .open
    }
    pub fn popup_frame(&self) -> Option<Rect> {
        self.host
            .get(CHOICE)
            .unwrap()
            .component::<Choice>()
            .unwrap()
            .view
            .layout()
            .popup
            .as_ref()
            .map(|p| p.popup)
    }
    /// This example uses unique labels as domain identity. A response omitting
    /// the retained choice is rejected atomically; the caller may show failure
    /// while cached options remain usable. Empty initial/loading data is allowed.
    pub fn set_options(&mut self, labels: Vec<String>) -> bool {
        let choice = self
            .host
            .get_mut(CHOICE)
            .unwrap()
            .component_mut::<Choice>()
            .unwrap();
        let next = if labels.is_empty() {
            None
        } else if let Some(value) = &choice.selected_value {
            let Some(index) = labels.iter().position(|label| label == value) else {
                return false;
            };
            Some(index)
        } else {
            Some(0)
        };
        if labels
            .iter()
            .enumerate()
            .any(|(i, value)| labels[..i].contains(value))
        {
            return false;
        }
        // Data replacement cancels old popup geometry/capture. Status-only
        // transitions keep it; a stale pointer release must not choose a new row.
        if choice.labels != labels {
            choice.view.cancel(&mut choice.model);
        }
        choice.model.menu.items.count = labels.len();
        choice.labels = labels;
        if let Some(index) = next {
            choice.model.selected = index;
            choice.selected_value = choice.labels.get(index).cloned();
        } else {
            choice.view.cancel(&mut choice.model);
        }
        true
    }
    pub fn measure(&mut self, text: &mut dyn ButtonTextRenderer, width: f32) -> Size {
        self.host
            .get_mut(ROOT)
            .unwrap()
            .measure(text, Size::new(width, 4096.))
            .size
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), CompositionError> {
        self.host.render(scene, text, frame, clip, palette)
    }
}
