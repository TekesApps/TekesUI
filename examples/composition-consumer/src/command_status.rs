//! Caller-owned async command lookup, using original public trigger/menu/editor.
use crate::content_status::{Feedback, Status};
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonModel, ButtonSize, ButtonTextRenderer, ControlKey, TextSurface,
    composition::*,
    extended::{Axis, ComboBox, CommandPalette, Stack},
    layout::MeasuredChild,
    layout_view::CommandPaletteView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Spacing, proposal};
pub const ROOT: ComponentId = ComponentId(8400);
pub const COMMAND: ComponentId = ComponentId(8401);
pub const FEEDBACK: ComponentId = ComponentId(8402);
const LABEL: &str = "Find a command";
#[derive(Debug)]
struct Commands {
    editor: TextFieldContent,
    model: CommandPalette,
    view: CommandPaletteView,
    focused: bool,
    keyboard: bool,
    trigger_press: bool,
    editor_frame: Rect,
    last_executed: Option<String>,
}
impl Commands {
    fn open(&self) -> bool {
        self.model.combo.open
    }
    fn sync_query(&mut self) {
        if self.model.combo.query != self.editor.surface.input.text {
            self.model
                .combo
                .set_query(self.editor.surface.input.text.clone());
        }
    }
    fn opened(&mut self) {
        if !self.view.open_from(&mut self.model, COMMAND.0 as usize) {
            return;
        }
        let n = self.editor.surface.input.len();
        self.editor
            .surface
            .input
            .insert(&self.model.combo.query, Some(0..n));
        self.view.blur_trigger();
        self.editor.set_focus(self.focused, self.keyboard);
    }
    fn closed(&mut self, selected: Option<usize>) {
        if let Some(i) = selected {
            self.last_executed = self.model.combo.items.get(i).cloned();
            let n = self.editor.surface.input.len();
            self.editor
                .surface
                .input
                .insert(&self.model.combo.query, Some(0..n));
        }
        self.editor.set_focus(false, false);
        let caller = self.view.take_restore_focus(&self.model);
        if self.focused && caller == Some(COMMAND.0 as usize) && !self.model.combo.disabled {
            self.view.focus_trigger(self.keyboard);
        }
    }
    fn menu_key(&mut self, key: ControlKey, down: bool) -> ContentResponse {
        let before = self.open();
        let old = self.model.clone();
        let selected = self.view.key(key, down, &mut self.model);
        if before && !self.open() {
            self.closed(selected);
        }
        ContentResponse {
            consumed: before
                && matches!(
                    key,
                    ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
                ),
            changed: old != self.model,
            activated: selected.is_some(),
            ..Default::default()
        }
    }
}
impl Component for Commands {
    fn prepare_text_input(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if self.open() {
            self.editor.prepare_text_input(text, frame, clip);
        }
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if !self.open() {
            return key
                .key
                .map(|key_control| {
                    self.event(&ContentEvent::Key {
                        key: key_control,
                        down: key.down,
                    })
                })
                .unwrap_or_default();
        }
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
        self.open()
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.open().then_some(&self.editor.surface)
    }
    fn composing(&self) -> bool {
        self.open() && self.editor.composing()
    }
    fn focusable(&self) -> bool {
        true
    }
    fn set_focus(&mut self, focused: bool, keyboard: bool) {
        self.focused = focused;
        self.keyboard = keyboard;
        if !focused {
            self.editor.set_focus(false, false);
            self.sync_query();
            self.view.blur(&mut self.model);
            self.trigger_press = false;
        } else if self.open() {
            self.view.blur_trigger();
            self.editor.set_focus(true, keyboard);
        } else {
            self.view.focus_trigger(keyboard);
        }
    }
    fn measure(&mut self, mut t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let trigger = ButtonModel::measure(
            &mut t,
            LABEL,
            a.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let h = if self.open() {
            trigger.height + Spacing::Tight.value() + self.editor.measure(t, a).size.height
        } else {
            trigger.height
        };
        MeasuredChild::new(Size::new(a.width, h))
    }
    fn arrange(&mut self, mut t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let h = ButtonModel::measure(
            &mut t,
            LABEL,
            f.size.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        )
        .height;
        if self.open() {
            let y = f.min_y() + h + Spacing::Tight.value();
            self.editor_frame = Rect::new(f.min_x(), y, f.size.width, (f.max_y() - y).max(0.));
            self.editor.arrange(t, self.editor_frame, c);
        }
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        mut t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let h = ButtonModel::measure(
            &mut t,
            LABEL,
            f.size.width,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        )
        .height;
        self.view.render_trigger(
            s,
            &mut t,
            &mut self.model,
            LABEL,
            Rect::new(f.min_x(), f.min_y(), f.size.width, h),
            Some(c),
            theme.palette,
            proposal::TextRole::UiBody,
        );
        if self.open() {
            self.editor.paint(s, t, self.editor_frame, c, theme);
        }
    }
    fn semantic_frame(&self, f: Rect) -> Rect {
        if self.open() {
            self.editor.semantic_frame(f)
        } else {
            self.view.trigger_layout().map(|l| l.frame).unwrap_or(f)
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = if self.open() {
            self.editor.semantics()
        } else {
            ContentSemantics::new(ContentRole::Button, LABEL)
        };
        s.expanded = Some(self.open());
        s
    }
    fn paint_overlay(
        &mut self,
        s: &mut Scene,
        mut t: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(
            s,
            &mut t,
            &mut self.model,
            self.editor.surface.outer,
            c,
            Some(c),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.view.options.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn overlay_open(&self) -> bool {
        self.open()
    }
    fn overlay_presented(&self) -> bool {
        self.open() || self.view.options.is_presented()
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        if !self.open() || self.composing() {
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
                self.keyboard = true;
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
            ContentEvent::PointerDown(p)
                if !self.editor.surface.contains_visible(*p)
                    && !self
                        .view
                        .trigger_layout()
                        .is_some_and(|l| l.visible.is_some_and(|r| r.contains(*p))) =>
            {
                self.view.cancel(&mut self.model);
                self.closed(None);
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) => {
                let selected = self.view.pointer_up(*p, &mut self.model);
                if selected.is_some() {
                    self.keyboard = false;
                    self.closed(selected);
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
        if matches!(e, ContentEvent::Cancel) {
            self.editor.event(e);
            self.view.cancel(&mut self.model);
            self.closed(None);
            self.trigger_press = false;
            return ContentResponse::consumed(true);
        }
        if self.composing() {
            let r = self.editor.event(e);
            self.sync_query();
            return r;
        }
        let before = self.open();
        match e {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p, &mut self.model);
                let r = if before {
                    self.editor.event(e)
                } else {
                    ContentResponse::default()
                };
                ContentResponse { changed: true, ..r }
            }
            ContentEvent::PointerDown(p)
                if self
                    .view
                    .trigger_layout()
                    .is_some_and(|l| l.visible.is_some_and(|f| f.contains(*p))) =>
            {
                self.keyboard = false;
                self.trigger_press = self.view.pointer_down_trigger(*p);
                ContentResponse {
                    consumed: self.trigger_press,
                    capture_pointer: self.trigger_press,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) if self.trigger_press => {
                self.trigger_press = false;
                let opened = self.view.pointer_up_trigger(*p, &mut self.model);
                if opened {
                    self.opened();
                }
                ContentResponse {
                    consumed: true,
                    changed: opened,
                    ..Default::default()
                }
            }
            ContentEvent::Key { key, down } if !before => {
                self.keyboard = true;
                let r = self.view.trigger_key(*key, *down, &mut self.model);
                if self.open() {
                    self.opened();
                }
                ContentResponse {
                    consumed: r.consumed,
                    changed: r.changed,
                    ..Default::default()
                }
            }
            ContentEvent::Key { key, down }
                if matches!(
                    key,
                    ControlKey::Enter | ControlKey::Escape | ControlKey::Up | ControlKey::Down
                ) =>
            {
                self.menu_key(*key, *down)
            }
            _ if before => {
                let r = self.editor.event(e);
                self.sync_query();
                r
            }
            _ => ContentResponse::default(),
        }
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.open() {
            return Vec::new();
        }
        let live = self.model.combo.filtered();
        self.view
            .options
            .layout()
            .map(|l| {
                l.items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        let original = self.view.options.original_index(index)?;
                        if live.get(index) != Some(&original) {
                            return None;
                        }
                        let mut semantics = ContentSemantics::new(
                            ContentRole::MenuItem,
                            self.model.combo.items.get(original)?,
                        );
                        semantics.selected = self.model.combo.selected == Some(original);
                        Some(SemanticPart {
                            index: original,
                            frame: item.frame,
                            focused: self.model.combo.active == Some(original),
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
        if self.composing() {
            return if part.is_none() {
                self.editor.accessibility_action(part, action)
            } else {
                ContentResponse::default()
            };
        }
        if let Some(part) = part {
            if !self.open() || !matches!(action, AccessibilityAction::Press) {
                return ContentResponse::default();
            }
            if let Some(original) = self.view.activate_option(part, &mut self.model) {
                self.keyboard = true;
                self.closed(Some(original));
                return ContentResponse {
                    consumed: true,
                    changed: true,
                    activated: true,
                    ..Default::default()
                };
            }
            return ContentResponse::default();
        }
        if !self.open() && matches!(action, AccessibilityAction::Press) {
            self.view.open(&mut self.model);
            self.opened();
            return ContentResponse::consumed(self.open());
        }
        if self.open() {
            let r = self.editor.accessibility_action(part, action);
            self.sync_query();
            r
        } else {
            ContentResponse::default()
        }
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
        let mut view = CommandPaletteView::default();
        view.options.set_reduced_motion(true);
        let commands = Commands {
            editor: TextFieldContent::new("Command query", ""),
            model: CommandPalette {
                combo: ComboBox {
                    items,
                    ..Default::default()
                },
            },
            view,
            focused: false,
            keyboard: false,
            trigger_press: false,
            editor_frame: Rect::default(),
            last_executed: None,
        };
        Ok(Self {
            host: ContentHost::new(Content::new(
                ROOT,
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: Spacing::Regular.value(),
                    },
                    children: vec![
                        Content::new(FEEDBACK, Feedback::new(status.clone())),
                        Content::new(COMMAND, commands),
                    ],
                    flexible_child: None,
                },
            ))?,
            status,
        })
    }
    fn commands(&self) -> &Commands {
        self.host
            .get(COMMAND)
            .unwrap()
            .component::<Commands>()
            .unwrap()
    }
    fn commands_mut(&mut self) -> &mut Commands {
        self.host
            .get_mut(COMMAND)
            .unwrap()
            .component_mut::<Commands>()
            .unwrap()
    }
    /// The existing owned presentation may outlive logical menu dismissal.
    pub fn is_presented(&self) -> bool {
        self.commands().overlay_presented()
    }
    pub fn set_status(&mut self, status: Status) {
        self.status.set(status);
        self.host.get_mut(FEEDBACK);
    }
    pub fn status(&self) -> Status {
        self.status.get()
    }
    pub fn open(&self) -> bool {
        self.commands().open()
    }
    pub fn query(&self) -> &str {
        &self.commands().editor.surface.input.text
    }
    pub fn last_executed(&self) -> Option<&str> {
        self.commands().last_executed.as_deref()
    }
    pub fn active(&self) -> Option<usize> {
        self.commands().model.combo.active
    }
    pub fn trigger_frame(&self) -> Option<Rect> {
        self.commands().view.trigger_layout().map(|l| l.frame)
    }
    pub fn query_frame(&self) -> Option<Rect> {
        self.open().then_some(self.commands().editor.surface.outer)
    }
    pub fn set_items(&mut self, items: Vec<String>) -> bool {
        let c = self.commands_mut();
        if c.composing()
            || items
                .iter()
                .enumerate()
                .any(|(i, v)| items[..i].contains(v))
        {
            return false;
        }
        let active = c
            .model
            .combo
            .active
            .and_then(|i| c.model.combo.items.get(i))
            .cloned();
        let selected = c
            .model
            .combo
            .selected
            .and_then(|i| c.model.combo.items.get(i))
            .cloned();
        if c.model.combo.items != items {
            c.view.options.cancel();
        }
        c.model.combo.items = items;
        c.model.combo.selected =
            selected.and_then(|v| c.model.combo.items.iter().position(|x| x == &v));
        let live = c.model.combo.filtered();
        c.model.combo.active = active
            .and_then(|v| c.model.combo.items.iter().position(|x| x == &v))
            .filter(|i| live.contains(i))
            .or_else(|| live.first().copied());
        true
    }
    pub fn measure(&mut self, t: &mut dyn ButtonTextRenderer, width: f32) -> Size {
        self.host
            .get_mut(ROOT)
            .unwrap()
            .measure(t, Size::new(width, 4096.))
            .size
    }
    pub fn render(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        p: Palette,
    ) -> Result<(), CompositionError> {
        self.host.render(s, t, f, c, p)
    }
}
