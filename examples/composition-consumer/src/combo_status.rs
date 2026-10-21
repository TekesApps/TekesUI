//! Caller-owned ComboBox refresh using the original TextFieldContent and View.
use crate::content_status::{Feedback, Status};
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey, TextSurface,
    composition::*,
    extended::{Axis, ComboBox, Stack},
    layout::MeasuredChild,
    layout_view::ComboBoxView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing, proposal};
pub const ROOT: ComponentId = ComponentId(8300);
pub const QUERY: ComponentId = ComponentId(8301);
pub const FEEDBACK: ComponentId = ComponentId(8302);
#[derive(Debug)]
struct Query {
    editor: TextFieldContent,
    model: ComboBox,
    view: ComboBoxView,
    selected_value: Option<String>,
}
impl Query {
    fn sync_query(&mut self) {
        if self.model.query != self.editor.surface.input.text {
            self.model.set_query(self.editor.surface.input.text.clone());
        }
    }
    fn selection(&mut self, index: usize) {
        self.selected_value = self.model.items.get(index).cloned();
        let value = self.model.query.clone();
        let len = self.editor.surface.input.len();
        self.editor.surface.input.insert(&value, Some(0..len));
        self.editor.surface.clauses.clear();
    }
    fn menu_key(&mut self, key: ControlKey, down: bool) -> ContentResponse {
        let before = (
            self.model.open,
            self.model.active,
            self.model.selected,
            self.model.query.clone(),
        );
        let selected = self.view.key(key, down, &mut self.model);
        if let Some(index) = selected {
            self.selection(index);
        }
        ContentResponse {
            consumed: matches!(
                key,
                ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
            ),
            changed: before
                != (
                    self.model.open,
                    self.model.active,
                    self.model.selected,
                    self.model.query.clone(),
                ),
            activated: selected.is_some(),
            ..Default::default()
        }
    }
}
impl Component for Query {
    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.editor.prepare_text_input(text, frame, clip);
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if let Some(control) = key.key
            && matches!(
                control,
                ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
            )
        {
            return self.event(&ContentEvent::Key {
                key: control,
                down: key.down,
            });
        }
        let response = self.editor.key_event(key);
        self.sync_query();
        response
    }

    fn requires_text_services(&self) -> bool {
        true
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.editor.text_surface()
    }
    fn composing(&self) -> bool {
        self.editor.composing()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.editor.measure(text, a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.editor.arrange(text, f, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.editor.paint(s, text, f, c, theme);
    }
    fn semantic_frame(&self, f: Rect) -> Rect {
        self.editor.semantic_frame(f)
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = self.editor.semantics();
        s.expanded = Some(self.model.open);
        s
    }
    fn focusable(&self) -> bool {
        self.editor.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.editor.set_focus(focused, visible);
        if !focused {
            self.sync_query();
            self.model.open = false;
            self.view.options.cancel();
        }
    }
    fn paint_overlay(
        &mut self,
        s: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(
            s,
            &mut text,
            &mut self.model,
            self.editor.surface.outer,
            c,
            Some(c),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.view.options.needs_animation() {
            text.request_animation_frame();
        }
    }
    fn overlay_open(&self) -> bool {
        self.model.open
    }
    fn overlay_presented(&self) -> bool {
        self.model.open || self.view.options.is_presented()
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        // The host owns raw-key IME arbitration. Also reject candidate pointer
        // activation while marked text is owned by the native input method.
        if self.editor.composing()
            && matches!(
                e,
                ContentEvent::PointerMove(_)
                    | ContentEvent::PointerDown(_)
                    | ContentEvent::PointerUp(_)
                    | ContentEvent::Scroll { .. }
            )
        {
            return ContentResponse::default();
        }
        let inside = match e {
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerUp(p)
            | ContentEvent::PointerMove(p) => self
                .view
                .options
                .layout()
                .is_some_and(|l| l.visible.is_some_and(|r| r.contains(*p))),
            _ => false,
        };
        match e {
            ContentEvent::Key { key, down }
                if matches!(
                    key,
                    ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
                ) =>
            {
                self.menu_key(*key, *down)
            }
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p, &mut self.model);
                ContentResponse {
                    changed: inside,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(p) if inside => {
                self.view.pointer_down(*p);
                ContentResponse {
                    consumed: true,
                    changed: true,
                    capture_pointer: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(p) if !self.editor.surface.contains_visible(*p) => {
                self.model.open = false;
                self.view.options.cancel();
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) => {
                let selected = self.view.pointer_up(*p, &mut self.model);
                if let Some(index) = selected {
                    self.selection(index);
                }
                ContentResponse {
                    consumed: inside || selected.is_some(),
                    changed: selected.is_some(),
                    activated: selected.is_some(),
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if self.editor.composing() && !matches!(e, ContentEvent::Cancel) {
            let response = self.editor.event(e);
            self.sync_query();
            return response;
        }
        if let ContentEvent::Key { key, down } = e
            && matches!(
                key,
                ControlKey::Enter | ControlKey::Up | ControlKey::Down | ControlKey::Escape
            )
        {
            return self.menu_key(*key, *down);
        }
        let mut r = self.editor.event(e);
        self.sync_query();
        if matches!(e, ContentEvent::PointerDown(_)) && r.consumed {
            self.model.open = true;
            if self.model.active.is_none() {
                self.model.active = self.model.filtered().first().copied();
            }
            r.changed = true;
        }
        if matches!(e, ContentEvent::Cancel) {
            self.model.open = false;
            self.view.options.cancel();
        }
        r
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.model.open {
            return Vec::new();
        }
        let live = self.model.filtered();
        self.view
            .options
            .layout()
            .map(|popup| {
                popup
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        let original = self.view.options.original_index(index)?;
                        if live.get(index) != Some(&original) {
                            return None;
                        }
                        let mut semantics = ContentSemantics::new(
                            ContentRole::MenuItem,
                            self.model.items.get(original)?,
                        );
                        semantics.selected = self.model.selected == Some(original);
                        Some(SemanticPart {
                            index: original,
                            frame: item.frame,
                            focused: self.model.active == Some(original),
                            semantics,
                        })
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
        if let Some(part) = part {
            if self.editor.composing()
                || !self.model.open
                || !matches!(action, AccessibilityAction::Press)
            {
                return ContentResponse::default();
            }
            if let Some(original) = self.view.activate_option(part, &mut self.model) {
                self.selection(original);
                return ContentResponse {
                    consumed: true,
                    changed: true,
                    activated: true,
                    ..Default::default()
                };
            }
            return ContentResponse::default();
        }
        let r = self.editor.accessibility_action(part, action);
        self.sync_query();
        r
    }
}
#[derive(Debug)]
pub struct Scenario {
    pub host: ContentHost,
    status: Rc<Cell<Status>>,
}
impl Scenario {
    pub fn new(items: Vec<String>) -> Result<Self, CompositionError> {
        let status = Rc::new(Cell::new(Status::Ready));
        let mut view = ComboBoxView::default();
        view.options.set_reduced_motion(true);
        let query = Query {
            editor: TextFieldContent::new("Search available layouts", ""),
            model: ComboBox {
                items,
                ..Default::default()
            },
            view,
            selected_value: None,
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
                    Content::new(QUERY, query),
                ],
                flexible_child: None,
            },
        ))?;
        Ok(Self { host, status })
    }
    /// The existing owned presentation may outlive logical menu dismissal.
    pub fn is_presented(&self) -> bool {
        self.host
            .get(QUERY)
            .unwrap()
            .component::<Query>()
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
    pub fn query(&self) -> &str {
        &self
            .host
            .get(QUERY)
            .unwrap()
            .component::<Query>()
            .unwrap()
            .editor
            .surface
            .input
            .text
    }
    pub fn selected(&self) -> Option<&str> {
        let q = self.host.get(QUERY).unwrap().component::<Query>().unwrap();
        q.model
            .selected
            .and_then(|i| q.model.items.get(i))
            .map(String::as_str)
    }
    pub fn active(&self) -> Option<usize> {
        self.host
            .get(QUERY)
            .unwrap()
            .component::<Query>()
            .unwrap()
            .model
            .active
    }
    pub fn open(&self) -> bool {
        self.host
            .get(QUERY)
            .unwrap()
            .component::<Query>()
            .unwrap()
            .model
            .open
    }
    pub fn popup_frame(&self) -> Option<Rect> {
        self.host
            .get(QUERY)
            .unwrap()
            .component::<Query>()
            .unwrap()
            .view
            .options
            .layout()
            .map(|l| l.popup)
    }
    pub fn set_validation_error(&mut self, error: Option<String>) {
        self.host
            .get_mut(QUERY)
            .unwrap()
            .component_mut::<Query>()
            .unwrap()
            .editor
            .field
            .error = error;
    }
    /// Native preedit owns its data boundary. Reject a response while composing;
    /// unique labels are fixture domain IDs, as in the existing Select example.
    pub fn set_items(&mut self, items: Vec<String>) -> bool {
        let q = self
            .host
            .get_mut(QUERY)
            .unwrap()
            .component_mut::<Query>()
            .unwrap();
        if q.editor.composing()
            || items
                .iter()
                .enumerate()
                .any(|(i, v)| items[..i].contains(v))
        {
            return false;
        }
        let selected = if items.is_empty() {
            None
        } else if let Some(value) = &q.selected_value {
            let Some(i) = items.iter().position(|v| v == value) else {
                return false;
            };
            Some(i)
        } else {
            None
        };
        let active = q.model.active.and_then(|i| q.model.items.get(i)).cloned();
        if q.model.items != items {
            q.view.options.cancel();
        }
        q.model.items = items;
        q.model.selected = selected;
        let filtered = q.model.filtered();
        q.model.active = active
            .and_then(|v| q.model.items.iter().position(|x| x == &v))
            .filter(|i| filtered.contains(i))
            .or_else(|| filtered.first().copied());
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
        s: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        p: Palette,
    ) -> Result<(), CompositionError> {
        self.host.render(s, text, f, c, p)
    }
}
