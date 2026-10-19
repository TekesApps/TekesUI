//! Minimal semantic adapters. State and presentation stay in production controls.
use super::*;
use crate::extended::Switch;
use crate::feedback_view::FieldView;
use crate::{
    Dialog, DialogView, Field, MenuEvent, MenuView, MenuWheelInput, ReadingController,
    SelectionEvent, SwitchView, TextSurface, TextSurfacePresentation,
};
use tekes_ui_tokens::Spacing;

/// General content organization, deliberately without ListBox selection semantics.
/// Children keep their own actions; adding an input never implicitly selects a row.
#[derive(Debug)]
pub struct ListContent {
    pub name: String,
    pub rows: StackContent,
}
impl ListContent {
    pub fn new(name: impl Into<String>, children: Vec<Content>) -> Self {
        Self {
            name: name.into(),
            rows: StackContent {
                flexible_child: None,
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                children,
            },
        }
    }
}
impl Component for ListContent {
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.rows.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.rows.arrange(t, f, c)
    }
    fn children(&self) -> &[Content] {
        &self.rows.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.rows.children)
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::List, &self.name)
    }
}

#[derive(Debug)]
pub struct SwitchContent {
    pub model: Switch,
    pub view: SwitchView,
    pub label: String,
}
impl SwitchContent {
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            model: Switch {
                checked,
                disabled: false,
            },
            view: SwitchView::default(),
            label: label.into(),
        }
    }
}
impl Component for SwitchContent {
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || self.model.disabled {
            return ContentResponse::default();
        }
        let changed = self.model.toggle();
        ContentResponse {
            consumed: true,
            changed,
            activated: changed,
            ..Default::default()
        }
    }

    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let s = t.measure(
            &self.label,
            proposal::TextRole::UiBody,
            (a.width - 62.).max(0.),
        );
        MeasuredChild::new(Size::new(a.width, 32_f32.max(s.height + 8.)))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render_labeled(
            s,
            &mut TextAdapter(t),
            self.model,
            f,
            Some(c),
            theme.palette,
            &self.label,
        );
        if self.view.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn focusable(&self) -> bool {
        !self.model.disabled
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        if f {
            self.view.focus(v)
        } else {
            self.view.blur()
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Switch, &self.label);
        s.selected = self.model.checked;
        s.disabled = self.model.disabled;
        s.invalid = self.view.error();
        s
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let mut r = ContentResponse::default();
        match e {
            ContentEvent::PointerMove(p) => r.changed = self.view.pointer_move(*p),
            ContentEvent::PointerDown(p) => {
                r.consumed = self.view.pointer_down(*p, &self.model);
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
            ContentEvent::PointerUp(p) => {
                let e = self.view.pointer_up(*p, &mut self.model);
                r.consumed = e != SelectionEvent::Ignored;
                r.changed = e == SelectionEvent::Changed;
            }
            ContentEvent::Key { key, down } => {
                let e = self.view.key(*key, *down, &mut self.model);
                r.consumed = e != SelectionEvent::Ignored;
                r.changed = r.consumed;
            }
            ContentEvent::Cancel => {
                r.changed = self.view.cancel();
            }
            _ => {}
        }
        r
    }
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
    focused: bool,
}
impl TextFieldContent {
    pub fn new(label: impl Into<String>, value: &str) -> Self {
        Self {
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
            focused: false,
        }
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
    fn field_layout(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
    ) -> crate::feedback_view::FieldLayout {
        if self.surface.presentation == TextSurfacePresentation::Content {
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
            let base = FieldView::default().render_with_presentation(
                &mut Scene::new(),
                &mut TextAdapter(t),
                &self.field,
                f,
                Some(c),
                Palette::from(tekes_ui_tokens::Theme::LIGHT),
                proposal::TextRole::UiBody,
                presentation,
            );
            let metadata = base.slot.min_y() - f.min_y() + base.message.max_y() - base.slot.max_y();
            let minimum = presentation.slot_min_height.max(base.slot.size.height);
            let height = self.viewport_height(t, f, metadata, minimum);
            presentation.slot_height = Some(height);
        }
        self.view.render_with_presentation(
            &mut Scene::new(),
            &mut TextAdapter(t),
            &self.field,
            f,
            Some(c),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
            proposal::TextRole::UiBody,
            presentation,
        )
    }
}
impl Component for TextFieldContent {
    fn action_kinds(&self, result: &ContentResponse) -> Vec<ContentActionKind> {
        if result.activated {
            vec![ContentActionKind::Submit]
        } else {
            Vec::new()
        }
    }

    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.keyboard(key, true)
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.surface.outer
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || self.surface.input.disabled || self.surface.input.marked.is_some() {
            return ContentResponse::default();
        }
        if let AccessibilityAction::SetSelection(range) = action {
            let before = self.surface.input.selection();
            return if self.surface.input.select_range(range.clone()) {
                let changed = before != self.surface.input.selection();
                if changed {
                    self.surface.reveal();
                }
                ContentResponse::consumed(changed)
            } else {
                ContentResponse::default()
            };
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

    fn requires_text_services(&self) -> bool {
        true
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        Some(&self.surface)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let l = self.field_layout(
            t,
            Rect::new(0., 0., a.width, a.height),
            Rect::new(0., 0., a.width, a.height),
        );
        MeasuredChild::new(Size::new(a.width, l.slot.max_y().max(l.message.max_y())))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let l = self.field_layout(t, f, c);
        self.surface.error = self.field.error.is_some();
        self.layout_surface(t, l.slot, c);
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        if !self.surface.outer.is_empty() {
            self.layout_surface(t, self.surface.outer, clip);
        }
    }

    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        // Arrangement owns the exact viewport. Paint must not shape/resize a
        // second time or derive a different slot from an unrelated height.
        if self.surface.presentation == TextSurfacePresentation::Field {
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
                .draw(host, s, theme.palette, self.focused, caret_visible);
        }
    }
    fn focusable(&self) -> bool {
        !self.surface.input.disabled
    }
    fn set_focus(&mut self, f: bool, _: bool) {
        if self.focused && !f {
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
    fn composing(&self) -> bool {
        self.surface.input.marked.is_some()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::TextInput, &self.field.label);
        s.value = Some(self.surface.input.text.clone());
        s.disabled = self.surface.input.disabled;
        s.invalid = self.field.error.is_some();
        s.description = (!self.field.message().is_empty()).then(|| self.field.message().to_owned());
        s
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.input_event(e, None)
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        self.input_event(&pointer.routing_event(), Some(pointer))
    }
}

impl TextFieldContent {
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
                    plain && self.surface.command("insertNewline:").unwrap_or(false),
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
                    Some(ControlKey::Left) if modifiers.command => "moveToBeginningOfLine",
                    Some(ControlKey::Right) if modifiers.command => "moveToEndOfLine",
                    Some(ControlKey::Left) if modifiers.alt => "moveWordLeft",
                    Some(ControlKey::Right) if modifiers.alt => "moveWordRight",
                    Some(ControlKey::Left) => "moveLeft",
                    Some(ControlKey::Right) => "moveRight",
                    Some(ControlKey::Up) if modifiers.command => "moveToBeginningOfDocument",
                    Some(ControlKey::Down) if modifiers.command => "moveToEndOfDocument",
                    Some(ControlKey::Up | ControlKey::Down)
                        if modifiers.alt || modifiers.control =>
                    {
                        return ContentResponse::default();
                    }
                    Some(ControlKey::Up) => "moveUp",
                    Some(ControlKey::Down) => "moveDown",
                    _ => unreachable!(),
                };
                let command = format!(
                    "{command}{}:",
                    if modifiers.shift {
                        "AndModifySelection"
                    } else {
                        ""
                    }
                );
                let changed = self.surface.command(&command);
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
    fn input_event(&mut self, e: &ContentEvent, pointer: Option<&PointerEvent>) -> ContentResponse {
        let mut r = ContentResponse::default();
        if self.surface.input.disabled && !matches!(e, ContentEvent::Cancel) {
            return r;
        }
        match e {
            ContentEvent::PointerDown(p) => {
                if let Some(reading) = &mut self.reading {
                    reading.stop_following(&mut self.surface);
                }
                r.consumed = self
                    .surface
                    .pointer_down(
                        *p,
                        pointer.is_some_and(|p| p.modifiers.shift),
                        pointer.map_or(1, |p| p.click_count),
                        24.,
                    )
                    .is_some();
                r.capture_pointer = r.consumed;
                r.changed = r.consumed;
            }
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
                if (c.starts_with("move") || c == "selectAll:")
                    && let Some(reading) = &mut self.reading
                {
                    reading.stop_following(&mut self.surface);
                }
                let changed = self.surface.command(c);
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
                    ControlKey::Left => "moveLeft:",
                    ControlKey::Right => "moveRight:",
                    ControlKey::Up => "moveUp:",
                    ControlKey::Down => "moveDown:",
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
                let result = self.surface.wheel_at(
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

/// A semantic menu remains a leaf with its existing MenuView item contracts.
/// Its popup is painted/routed in the host overlay pass, above later siblings.
#[derive(Debug)]
pub struct MenuContent {
    pub trigger: ButtonContent,
    pub menu: MenuView,
    pub labels: Vec<String>,
    pub selected: Option<usize>,
    pub popup: Option<crate::MenuLayout>,
}
impl MenuContent {
    pub fn new(label: impl Into<String>, labels: Vec<String>) -> Self {
        Self {
            trigger: ButtonContent::new(label),
            menu: MenuView::new(labels.len()),
            labels,
            selected: None,
            popup: None,
        }
    }
    fn handle_menu(&mut self, e: &ContentEvent) -> ContentResponse {
        let mut wheel = false;
        let result = match e {
            ContentEvent::PointerMove(p) => {
                self.menu.pointer_move_at(*p);
                MenuEvent::None
            }
            ContentEvent::PointerDown(p) => self.menu.pointer_down_at(*p),
            ContentEvent::PointerUp(p) => self.menu.pointer_up_at(*p),
            ContentEvent::Key { key, down } => self.menu.key(*key, *down),
            ContentEvent::Scroll {
                position,
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
            } => {
                wheel = matches!(
                    self.menu.wheel_event(MenuWheelInput {
                        point: *position,
                        delta: *delta,
                        precise: *precise,
                        begin: *begin,
                        end: *end,
                        momentum: *momentum,
                        time: *time
                    }),
                    crate::MenuWheelEvent::Consumed { .. }
                );
                MenuEvent::None
            }
            ContentEvent::Cancel => self.menu.deactivate(),
            _ => MenuEvent::None,
        };
        if let MenuEvent::Activated { index, .. } = result {
            self.selected = Some(index);
        }
        ContentResponse {
            text_key: None,
            consumed: if matches!(e, ContentEvent::Scroll { .. }) {
                wheel
            } else {
                !matches!(
                    e,
                    ContentEvent::Key {
                        key: ControlKey::Tab { .. },
                        ..
                    }
                )
            },
            changed: !matches!(e, ContentEvent::Scroll { .. }) || wheel,
            activated: matches!(result, MenuEvent::Activated { .. }),
            capture_pointer: false,
        }
    }
}
impl Component for MenuContent {
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.menu.is_open() {
            return Vec::new();
        }
        self.popup.as_ref().map_or_else(Vec::new, |p| {
            p.items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let label = self.labels.get(index)?;
                    Some({
                        let frame = item.frame;
                        let mut semantics = ContentSemantics::new(ContentRole::MenuItem, label);
                        semantics.selected = self.selected == Some(index);
                        SemanticPart {
                            index,
                            frame,
                            focused: p.active_item == index,
                            semantics,
                        }
                    })
                })
                .collect()
        })
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if self.trigger.button.state.disabled {
            return ContentResponse::default();
        }
        if let Some(index) = part {
            if matches!(action, AccessibilityAction::Focus) {
                return if self.menu.focus_item(index) {
                    ContentResponse::consumed(true)
                } else {
                    ContentResponse::default()
                };
            }
            if !matches!(action, AccessibilityAction::Press) {
                return ContentResponse::default();
            }
            if let MenuEvent::Activated { index, .. } = self.menu.activate_item(index) {
                self.selected = Some(index);
                return ContentResponse {
                    consumed: true,
                    changed: true,
                    activated: true,
                    ..Default::default()
                };
            }
            return ContentResponse::default();
        }
        if matches!(action, AccessibilityAction::Press) {
            self.menu.show(0);
            return ContentResponse::consumed(true);
        }
        ContentResponse::default()
    }

    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        self.trigger.measure(t, a)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.trigger.paint(s, t, f, c, theme);
    }
    fn paint_overlay(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.popup = self.menu.render(
            s,
            &mut TextAdapter(t),
            &labels,
            f,
            c,
            Some(c),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        if self.menu.needs_animation() {
            t.request_animation_frame();
        }
    }
    fn overlay_open(&self) -> bool {
        self.menu.is_open()
    }
    fn overlay_presented(&self) -> bool {
        self.menu.is_open() || self.menu.is_presented()
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.handle_menu(e)
    }
    fn focusable(&self) -> bool {
        self.trigger.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.trigger.set_focus(f, v);
        if !f {
            self.menu.deactivate();
        }
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if matches!(e, ContentEvent::Cancel) {
            self.menu.deactivate();
        }
        let r = self.trigger.event(e);
        if r.activated {
            self.menu.show(0);
        }
        r
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::MenuButton, &self.trigger.label);
        s.expanded = Some(self.menu.is_open());
        s.value = self.selected.and_then(|i| self.labels.get(i).cloned());
        s
    }
}

/// Three fixed semantic slots using the same Content type. The host owns modal
/// focus/restore; the existing Dialog model owns open/close. Body overflow is
/// expressed with the existing ScrollContent, not an additional scroll model.
#[derive(Debug)]
pub struct DialogContent {
    pub(super) model: Dialog,
    presence: crate::control_motion::OverlayPresence,
    pub name: String,
    slots: Vec<Content>,
    viewport: Rect,
    modal: Rect,
}
impl DialogContent {
    pub fn new(name: impl Into<String>, header: Content, body: Content, footer: Content) -> Self {
        Self {
            model: Dialog::default(),
            presence: Default::default(),
            name: name.into(),
            slots: vec![header, body, footer],
            viewport: Rect::default(),
            modal: Rect::default(),
        }
    }
    pub fn modal_frame(&self) -> Rect {
        self.modal
    }
    pub fn is_open(&self) -> bool {
        self.model.open
    }
    /// Paint may outlive semantic dismissal; it never keeps input or focus.
    pub fn is_presented(&self) -> bool {
        self.model.open || self.presence.is_presented()
    }
    pub fn needs_animation(&self) -> bool {
        self.presence.needs_animation()
    }
    /// Call before replacing the host atlas/text/image resources.
    pub fn clear_presentation(&mut self) {
        self.presence.clear();
    }
    pub(super) fn present(
        &mut self,
        destination: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
        current: Option<Scene>,
    ) {
        let context = format!("{frame:?}{clip:?}{palette:?}{}", text.backing_scale());
        self.presence.paint(
            destination,
            &mut TextAdapter(text),
            self.model.open,
            context,
            current,
        );
    }
}
impl Component for DialogContent {
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.modal
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.clear_presentation();
        }
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.slots.len() != 3 {
            Err(CompositionError::InvalidSlotCount)
        } else if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn active(&self) -> bool {
        self.model.open
    }
    fn children(&self) -> &[Content] {
        &self.slots
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.slots)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.viewport = f;
        if !self.is_presented() {
            return;
        }
        let pad = Spacing::Relaxed.value();
        let gap = Spacing::Regular.value();
        let w = (f.size.width - 16.).clamp(0., 460.);
        let inner = (w - pad * 2.).max(0.);
        let header = self.slots[0]
            .measure(t, Size::new(inner, f.size.height))
            .size
            .height;
        let footer = self.slots[2]
            .measure(t, Size::new(inner, f.size.height))
            .size
            .height;
        let body_available = (f.size.height - 2. * pad - 2. * gap - header - footer).max(0.);
        let body = self.slots[1]
            .measure(t, Size::new(inner, body_available))
            .size
            .height
            .min(body_available);
        let h = (header + footer + body + 2. * pad + 2. * gap).min(f.size.height);
        self.modal = Rect::new(
            f.min_x() + (f.size.width - w) * 0.5,
            f.min_y() + (f.size.height - h) * 0.5,
            w,
            h,
        );
        let clip = self.modal.intersection(&c).unwrap_or_default();
        let x = self.modal.min_x() + pad;
        let y = self.modal.min_y() + pad;
        self.slots[0].arrange(t, Rect::new(x, y, inner, header), clip);
        self.slots[1].arrange(t, Rect::new(x, y + header + gap, inner, body), clip);
        self.slots[2].arrange(
            t,
            Rect::new(x, self.modal.max_y() - pad - footer, inner, footer),
            clip,
        );
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        DialogView::paint_content_frame(
            s,
            self.viewport,
            self.modal,
            c,
            theme.palette,
            t.backing_scale(),
        );
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Dialog, &self.name)
    }
}

/// Range adapter for the existing Slider model/View. The name is semantic;
/// callers compose visible labels/captions with the existing content containers.
#[derive(Debug)]
pub struct SliderContent {
    pub model: crate::extended::Slider,
    pub view: crate::SliderView,
    pub name: String,
}
impl SliderContent {
    pub fn new(name: impl Into<String>, model: crate::extended::Slider) -> Self {
        Self {
            model,
            view: crate::SliderView::new(),
            name: name.into(),
        }
    }
}
impl Component for SliderContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(
            a.width,
            proposal::metric("size", "size-row-height").expect("shared row size"),
        ))
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(s, self.model, f, Some(c), theme.palette);
    }
    fn focusable(&self) -> bool {
        !self.model.disabled
    }
    fn set_focus(&mut self, f: bool, visible: bool) {
        if f {
            self.view.focus(visible)
        } else {
            self.view.blur()
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Slider, &self.name);
        s.value = Some(self.model.value().to_string());
        s.disabled = self.model.disabled;
        s
    }
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        Some(self.model.range())
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let before = self.model.value();
        let consumed = match e {
            ContentEvent::PointerDown(p) => {
                let began = self.view.pointer_down(*p, &mut self.model);
                return ContentResponse {
                    consumed: began,
                    changed: began,
                    capture_pointer: began,
                    ..Default::default()
                };
            }
            ContentEvent::PointerMove(p) => {
                let changed = self.view.pointer_move(*p, &mut self.model);
                return ContentResponse {
                    changed,
                    ..Default::default()
                };
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up(*p, &self.model),
            ContentEvent::Key { key, down }
                if matches!(
                    key,
                    ControlKey::Left
                        | ControlKey::Right
                        | ControlKey::Up
                        | ControlKey::Down
                        | ControlKey::Home
                        | ControlKey::End
                ) =>
            {
                let owned = self.view.is_focused() && !self.model.disabled;
                self.view.key(*key, *down, &mut self.model);
                owned
            }
            ContentEvent::Cancel => {
                let changed = self.view.cancel();
                return ContentResponse {
                    changed,
                    ..Default::default()
                };
            }
            _ => false,
        };
        ContentResponse {
            consumed,
            changed: self.model.value() != before,
            ..Default::default()
        }
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        a: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || self.model.disabled {
            return ContentResponse::default();
        }
        let changed = match a {
            AccessibilityAction::Increment => self.view.adjust_value(true, &mut self.model),
            AccessibilityAction::Decrement => self.view.adjust_value(false, &mut self.model),
            AccessibilityAction::SetValue(v) => {
                let Ok(value) = v.parse::<f64>() else {
                    return ContentResponse::default();
                };
                if !value.is_finite() {
                    return ContentResponse::default();
                }
                self.view.set_value(value, &mut self.model)
            }
            _ => return ContentResponse::default(),
        };
        changed.map_or_else(ContentResponse::default, ContentResponse::consumed)
    }
}
