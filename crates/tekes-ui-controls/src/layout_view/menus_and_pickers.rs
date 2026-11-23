use super::{has_area, valid};
use crate::composite::{MenuEvent, MenuHeader, MenuLayout, MenuRenderContext, MenuView};
use crate::{
    ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::ComboBox, paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

/// Shared popup adapter for ComboBox and CommandPalette. The editable text
/// surface remains a host bridge; option layout, clipping, hover, keyboard
/// navigation, and popup dismissal are owned here.
#[derive(Debug)]
pub struct FilteredMenuView {
    pub(super) menu: crate::Menu,
    pub(super) menu_view: MenuView,
    pub(super) mapping: Vec<usize>,
    pub(super) labels: Vec<String>,
    pub(super) layout: Option<MenuLayout>,
}
impl FilteredMenuView {
    pub fn is_presented(&self) -> bool {
        self.menu_view.is_presented()
    }

    /// Forward the owned popup scroll lifecycle to the host's frame clock.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.menu_view.set_reduced_motion(reduced)
    }
    pub fn needs_animation(&self) -> bool {
        self.menu_view.needs_animation()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        self.menu_view.tick(dt)
    }
    /// Routes a wheel gesture through the same clipped runtime that paints this popup.
    pub fn wheel_event(
        &mut self,
        input: crate::feedback_view::MenuWheelInput,
    ) -> crate::feedback_view::MenuWheelEvent {
        self.menu_view.wheel_event_with_model(&self.menu, input)
    }
    pub fn new() -> Self {
        Self {
            menu: crate::Menu::new(0),
            menu_view: MenuView::new_external(),
            mapping: Vec::new(),
            labels: Vec::new(),
            layout: None,
        }
    }
    /// `items` and `mapping` are parallel rendered-candidate snapshots. Each
    /// mapping entry is an index in the original source; `active` is a filtered
    /// row position. Semantic focus/activation accept original indices instead.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        items: &[String],
        mapping: &[usize],
        active: Option<usize>,
        open: bool,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        self.render_inner(
            MenuRenderContext {
                scene,
                text,
                anchor,
                visible,
                clip,
                palette,
                role,
                compact,
            },
            items,
            mapping,
            active,
            open,
            None,
        )
    }
    pub(super) fn render_inner(
        &mut self,
        context: MenuRenderContext<'_, impl ButtonTextRenderer>,
        items: &[String],
        mapping: &[usize],
        active: Option<usize>,
        open: bool,
        header: Option<MenuHeader<'_>>,
    ) -> Option<MenuLayout> {
        let MenuRenderContext {
            scene,
            text,
            anchor,
            visible,
            clip,
            palette,
            role,
            compact,
        } = context;
        // A press belongs to the options actually presented at pointer-down.
        // Updating labels/mapping must not retarget its eventual release.
        if self.mapping != mapping || self.labels != items {
            self.cancel();
        }
        self.mapping = mapping.to_vec();
        self.labels = items.to_vec();
        // An open query with no candidates belongs to the caller's empty
        // feedback. No phantom option surface may paint or intercept input.
        if items.is_empty() && header.is_none() {
            self.cancel();
            return None;
        }
        self.menu.items.count = items.len();
        self.menu.items.selected = active.unwrap_or(0).min(items.len().saturating_sub(1));
        self.menu.open = open;
        if open {
            let _ = self
                .menu_view
                .set_window_active_with_model(&mut self.menu, true);
        }
        let labels: Vec<&str> = items.iter().map(String::as_str).collect();
        self.layout = None;
        let l = if let Some((header, paint)) = header {
            self.menu_view.render_with_header(
                &mut self.menu,
                &labels,
                MenuRenderContext {
                    scene,
                    text,
                    anchor,
                    visible,
                    clip,
                    palette,
                    role,
                    compact,
                },
                header,
                paint,
            )
        } else {
            self.menu_view.render_with_model(
                &mut self.menu,
                scene,
                text,
                &labels,
                anchor,
                visible,
                clip,
                palette,
                role,
                compact,
            )
        }?;
        self.layout = Some(l.clone());
        Some(l)
    }
    /// Whether this candidate snapshot still describes the live source model.
    /// Hosts can use the same guard when publishing semantic nodes.
    pub fn is_current(&self, model: &ComboBox) -> bool {
        !model.disabled
            && model.open
            && self.menu.open
            && self.layout.is_some()
            && model.filtered() == self.mapping
            && self.source_matches(model)
    }
    fn source_matches(&self, model: &ComboBox) -> bool {
        self.mapping.len() == self.labels.len()
            && self
                .mapping
                .iter()
                .zip(&self.labels)
                .all(|(index, label)| model.items.get(*index) == Some(label))
    }
    // Confirmation changes the query, not the outgoing rows. Rebuild those
    // existing labels in the current atlas until exit finishes. Actual source
    // replacement still invalidates the snapshot; no alternate model is added.
    pub(super) fn mapping_for_model(&mut self, model: &ComboBox) -> Vec<usize> {
        if !model.open && self.is_presented() {
            if self.source_matches(model) {
                return self.mapping.clone();
            }
            self.cancel();
        }
        model.filtered()
    }
    pub(super) fn accepts_model(&mut self, model: &ComboBox) -> bool {
        let current = self.is_current(model);
        if !current {
            if !model.open && !model.disabled && self.source_matches(model) {
                self.dismiss();
            } else {
                self.cancel();
            }
        }
        current
    }
    /// Focus a currently rendered candidate by its original source index.
    /// Rejects closed, disabled and stale query/item snapshots. Focus does not
    /// commit a value; the editable surface keeps ownership of text/IME focus.
    pub fn focus_item(&mut self, original: usize, model: &mut ComboBox) -> bool {
        if !self.accepts_model(model) {
            return false;
        }
        let Some(filtered) = self.mapping.iter().position(|i| *i == original) else {
            return false;
        };
        if !self
            .menu_view
            .focus_item_with_model(&mut self.menu, filtered)
        {
            return false;
        }
        model.active = Some(original);
        true
    }
    /// Confirm a currently rendered candidate and return its original index.
    /// Shares MenuRuntime's mounted guard and the model's normal Enter path.
    pub fn activate_item(&mut self, original: usize, model: &mut ComboBox) -> Option<usize> {
        if !self.accepts_model(model) {
            return None;
        }
        let filtered = self.mapping.iter().position(|i| *i == original)?;
        if !matches!(
            self.menu_view
                .activate_item_with_model(&mut self.menu, filtered),
            MenuEvent::Activated { .. }
        ) {
            return None;
        }
        model.active = Some(original);
        let selected = model.key(ControlKey::Enter);
        self.dismiss();
        selected
    }
    pub fn pointer_move(&mut self, p: Point) -> bool {
        let before = self.menu.items.selected;
        self.menu_view.pointer_move_with_model(&mut self.menu, p);
        before != self.menu.items.selected
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.menu_view.pointer_down_with_model(&mut self.menu, p)
    }
    pub fn pointer_up(&mut self, p: Point) -> MenuEvent {
        self.menu_view.pointer_up_with_model(&mut self.menu, p)
    }
    pub fn key(&mut self, k: ControlKey, down: bool) -> MenuEvent {
        self.menu_view.key_with_model(&mut self.menu, k, down)
    }
    pub fn active_index(&self) -> Option<usize> {
        self.mapping.get(self.menu.items.selected).copied()
    }
    pub fn original_index(&self, filtered_index: usize) -> Option<usize> {
        self.mapping.get(filtered_index).copied()
    }
    pub fn is_visible(&self) -> bool {
        self.layout
            .as_ref()
            .is_some_and(|layout| layout.visible.is_some_and(has_area))
    }
    pub(crate) fn cancel_pointer_arm(&mut self) {
        self.menu_view.cancel_pointer_arm();
    }
    pub fn dismiss(&mut self) {
        let _ = self.menu_view.deactivate_with_model(&mut self.menu);
        self.layout = None;
        self.menu.open = false;
    }
    pub fn cancel(&mut self) {
        self.menu_view.clear_presentation();
        self.layout = None;
        let _ = self
            .menu_view
            .set_window_active_with_model(&mut self.menu, false);
        self.menu.open = false;
    }
    pub fn layout(&self) -> Option<&MenuLayout> {
        self.layout.as_ref()
    }
}
impl Default for FilteredMenuView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectLayout {
    pub trigger: ButtonLayout,
    pub popup: Option<MenuLayout>,
}
impl Default for SelectLayout {
    fn default() -> Self {
        Self {
            trigger: ButtonLayout {
                frame: Rect::default(),
                content: Rect::default(),
                visible: None,
                label_size: tekes_ui_scene::Size::default(),
            },
            popup: None,
        }
    }
}
#[derive(Debug)]
pub struct SelectView {
    trigger: ButtonModel,
    menu: MenuView,
    layout: SelectLayout,
    surface: &'static str,
    window_active: bool,
    disabled: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SelectPreparedInput {
    PointerDown(Point),
    Open,
    Focus(usize),
    Activate(usize),
    Close,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectPreparedAction {
    expected: SelectPreparedState,
    trigger: Option<crate::widgets::ButtonActionDecision>,
    menu: Option<crate::composite::MenuPreparedAction>,
    choice: Option<crate::widgets::SelectChoice>,
    pub event: MenuEvent,
    pub consumed: bool,
    pub reserve_open: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectPreparedState {
    trigger: crate::widgets::ButtonActionDecision,
    layout: SelectLayout,
    availability: (bool, bool),
    model: (usize, bool, usize, usize, Option<usize>),
}
impl SelectView {
    /// Logical close may retain outgoing paint before the next frame starts exit.
    pub fn is_presented(&self) -> bool {
        self.menu.is_presented()
    }
    /// Forward the owned popup scroll lifecycle to the host's frame clock.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.menu.set_reduced_motion(reduced)
    }
    pub fn needs_animation(&self) -> bool {
        self.menu.needs_animation()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        self.menu.tick(dt)
    }
    pub fn new(_count: usize) -> Self {
        Self {
            trigger: ButtonModel::default(),
            menu: MenuView::new_external(),
            layout: SelectLayout::default(),
            surface: "canvas",
            window_active: true,
            disabled: false,
        }
    }
    pub(crate) fn prepared_state(&self, model: &crate::Select) -> SelectPreparedState {
        SelectPreparedState {
            trigger: self.trigger.prepare_content_press(),
            layout: self.layout.clone(),
            availability: (self.window_active, self.disabled),
            model: (
                model.selected,
                model.menu.open,
                model.menu.items.count,
                model.menu.items.selected,
                model.menu.last_action,
            ),
        }
    }
    pub(crate) fn prepare_select_action(
        &self,
        model: &crate::Select,
        input: SelectPreparedInput,
    ) -> Option<SelectPreparedAction> {
        use crate::composite::MenuPreparedKind as K;
        if self.disabled || !self.window_active || model.menu.items.count == 0 {
            return None;
        }
        let mut trigger = None;
        let mut menu = None;
        let mut choice = None;
        let mut event = MenuEvent::None;
        let mut reserve_open = false;
        let consumed = match input {
            SelectPreparedInput::PointerDown(point) => {
                if model.menu.open {
                    return None;
                }
                self.next_open_generation()?;
                let d = self.trigger.prepare_content_down(point, false);
                let consumed = d.consumed;
                reserve_open = consumed;
                trigger = Some(d);
                consumed
            }
            input => {
                let kind = match input {
                    SelectPreparedInput::Open => {
                        reserve_open = !model.menu.open;
                        K::Open {
                            generation: 0,
                            selected: model.selected,
                        }
                    }
                    SelectPreparedInput::Focus(index) => K::Focus { index, offset: 0. },
                    SelectPreparedInput::Activate(index) => {
                        choice = Some(model.prepare_choice(index)?);
                        K::Activate { index }
                    }
                    SelectPreparedInput::Close => K::Close,
                    SelectPreparedInput::PointerDown(_) => unreachable!(),
                };
                let d = self.menu.prepare_select_action(&model.menu, kind)?;
                event = d.event;
                menu = Some(d);
                true
            }
        };
        Some(SelectPreparedAction {
            expected: self.prepared_state(model),
            trigger,
            menu,
            choice,
            event,
            consumed,
            reserve_open,
        })
    }
    pub(crate) fn apply_select_action(
        &mut self,
        model: &mut crate::Select,
        action: SelectPreparedAction,
    ) {
        if let Some(d) = action.trigger {
            self.trigger.apply_content_action(d);
        }
        if action.menu.is_some() {
            self.trigger.cancel();
        }
        if let Some(d) = action.menu {
            self.menu.apply_select_action(&mut model.menu, d);
        }
        if let Some(choice) = action.choice {
            model.apply_choice(choice);
            self.focus(true);
        }
    }
    #[cfg(test)]
    pub(crate) fn select_test_generation(&mut self, generation: u64) {
        self.menu.select_test_generation(generation);
    }
    pub(crate) fn next_open_generation(&self) -> Option<u64> {
        self.menu.next_open_generation()
    }
    pub(crate) fn open_generation(&self) -> u64 {
        self.menu.open_generation()
    }
    /// The trigger inherits its parent Surface; the popup keeps its Menu Surface.
    pub fn surface(&self) -> &'static str {
        self.surface
    }
    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)?;
        self.surface = surface;
        self.trigger.set_surface(surface);
        Ok(())
    }
    pub(crate) fn replace_options(&mut self, model: &mut crate::Select, count: usize) {
        self.trigger.cancel();
        self.trigger.clear_layout();
        self.menu.replace_items_with_model(&mut model.menu, count);
        self.layout = SelectLayout::default();
    }
    pub fn set_disabled(&mut self, disabled: bool, model: &mut crate::Select) {
        self.disabled = disabled;
        self.trigger.state.disabled = disabled;
        self.menu.set_disabled_with_model(&mut model.menu, disabled);
        if disabled {
            self.cancel(model);
        }
    }
    /// Validation belongs to the caller; this projects it through the existing
    /// shared error boundary without disabling selection or clearing its value.
    pub fn set_error(&mut self, error: bool) {
        self.trigger.state.error = error;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        frame: Rect,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> SelectLayout {
        self.render_trigger(scene, text, model, labels, frame, clip, palette, role);
        self.render_options(
            scene, text, model, labels, viewport, clip, palette, role, compact,
        );
        self.layout.clone()
    }
    /// Paint and lay out only the trigger. Call render_options in the host's
    /// overlay pass after base content; render remains the combined convenience.
    #[allow(clippy::too_many_arguments)]
    pub fn render_trigger(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        if !valid(frame, clip) {
            self.cancel(model);
            self.trigger.blur();
            self.trigger.clear_layout();
            self.layout = SelectLayout::default();
            return self.layout.trigger;
        }
        if labels.is_empty() {
            self.cancel(model);
        }
        self.trigger.state.disabled = self.disabled || labels.is_empty();
        self.trigger.state.focused =
            self.trigger.state.focused && self.window_active && !self.trigger.state.disabled;
        let selected = model.selected.min(labels.len().saturating_sub(1));
        let title = labels.get(selected).copied().unwrap_or("");
        let icon_side = proposal::metric("size", "size-icon-standard")
            .expect("select indicator size")
            * text.resolved_font_size(role)
            / role.style().size;
        let gap = proposal::metric("spacing", "space-tight").expect("select indicator gap")
            * text.resolved_font_size(role)
            / role.style().size;
        let insets = ButtonModel::measure_content(
            text,
            Size::default(),
            frame.size.width,
            ButtonSize::Body,
            role,
        )
        .width;
        let label_width = (frame.size.width - insets - icon_side - gap).max(0.);
        let label_size = text.measure(title, role, label_width);
        let mut trigger = self.trigger.render_content_frame_with_role(
            scene,
            text,
            Size::new(
                label_size.width + icon_side + gap,
                label_size.height.max(icon_side),
            ),
            frame,
            clip,
            palette,
            ButtonVariant::Quiet,
            role,
        );
        let icon_width = icon_side.min(trigger.content.size.width);
        let icon_frame = Rect::new(
            trigger.content.max_x() - icon_width,
            trigger.content.center().y - icon_width * 0.5,
            icon_width,
            icon_width,
        );
        trigger.content.size.width = (trigger.content.size.width - icon_width - gap).max(0.);
        if let Some(visible) = trigger.visible {
            let ink = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                self.surface,
                if self.trigger.state.disabled {
                    "action-quiet"
                } else {
                    "content"
                },
                if self.trigger.state.disabled {
                    "disabled"
                } else {
                    "rest"
                },
            )
            .expect("select Surface ink")
            .text;
            scene.push_clip(visible);
            // Explicit text clipping keeps long values out of the popup indicator slot.
            scene.push_clip(trigger.content);
            text.draw(
                scene,
                title,
                role,
                trigger.content,
                ButtonTextAlign::Leading,
                ink,
            );
            scene.pop_clip();
            if let Some(renderer) = text.icon_renderer() {
                renderer.draw_icon(scene, "tekes-expand", icon_frame, ink);
            }
            scene.pop_clip();
        }
        model.menu.items.count = labels.len();
        if !model.menu.open {
            model.menu.items.selected = model.selected.min(labels.len().saturating_sub(1));
        } else if model.menu.items.selected >= labels.len() {
            model.menu.items.selected = labels.len().saturating_sub(1);
        }
        self.layout = SelectLayout {
            trigger,
            popup: None,
        };
        if !trigger.visible.is_some_and(has_area) {
            self.cancel(model);
            self.trigger.blur();
        } else if self.window_active && !self.disabled && !labels.is_empty() {
            // Cancellation retires the popup runtime. A current visible trigger
            // restores only its owning window's availability before a later
            // checked AX open; it does not open or reserve anything here.
            let _ = self
                .menu
                .set_window_active_with_model(&mut model.menu, true);
        }
        trigger
    }
    /// Paint the owned menu above the host's base content, using the trigger
    /// layout from this frame. The existing menu still owns its state/lifecycle.
    #[allow(clippy::too_many_arguments)]
    pub fn render_options(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        model.menu.items.count = labels.len();
        if labels.is_empty() || self.disabled || !self.window_active {
            self.cancel(model);
            self.layout.popup = None;
            return None;
        }
        let trigger = self.layout.trigger;
        self.menu.set_popup_width_bounds(
            Some(trigger.frame.size.width),
            Some(trigger.frame.size.width.max(crate::composite::MENU_MAX_WIDTH)),
        );
        if model.menu.open {
            let _ = self
                .menu
                .set_window_active_with_model(&mut model.menu, self.window_active);
        }
        let popup = if model.menu.open || self.menu.is_presented() {
            self.menu.render_with_model(
                &mut model.menu,
                scene,
                text,
                labels,
                trigger.frame,
                viewport,
                clip,
                palette,
                role,
                compact,
            )
        } else {
            None
        };
        self.layout.popup = popup;
        if !self.layout.trigger.visible.is_some_and(has_area)
            || model.menu.open
                && self
                    .layout
                    .popup
                    .as_ref()
                    .is_none_or(|layout| !layout.visible.is_some_and(has_area))
        {
            self.cancel(model);
            self.trigger.blur();
        }
        self.layout.popup.clone()
    }
    pub(crate) fn pointer_move_changed(&mut self, point: Point, model: &mut crate::Select) -> bool {
        let before = (self.trigger.state, model.menu.items.selected);
        self.pointer_move(point, model);
        before != (self.trigger.state, model.menu.items.selected)
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut crate::Select) {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return;
        }
        if !self.window_active {
            return;
        }
        if self.disabled {
            // Preserve the latest pointer position while disabled so re-enabling
            // cannot revive hover from the location observed before disabling.
            self.trigger.pointer_move_at(p);
            return;
        }
        self.trigger.pointer_move_at(p);
        if model.menu.open {
            self.menu.pointer_move_with_model(&mut model.menu, p);
        }
    }
    pub fn pointer_down(&mut self, p: Point, model: &mut crate::Select) -> bool {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return false;
        }
        if !self.window_active || self.disabled {
            return false;
        }
        let mut dismissed = false;
        if model.menu.open {
            if self
                .layout
                .popup
                .as_ref()
                .is_some_and(|l| l.visible.is_some_and(|v| v.contains(p)))
            {
                let _ = self.menu.pointer_down_with_model(&mut model.menu, p);
                return true;
            }
            dismissed = matches!(
                self.menu.pointer_down_with_model(&mut model.menu, p),
                MenuEvent::Dismissed { .. }
            );
        }
        if dismissed {
            // The outside press already dismissed this open selection menu.
            // Do not arm its trigger again and reopen on the same release.
            self.trigger.cancel();
            return true;
        }
        self.trigger.pointer_down_at(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut crate::Select) -> MenuEvent {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return MenuEvent::None;
        }
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        if model.menu.open {
            let event = self.menu.pointer_up_with_model(&mut model.menu, p);
            if let MenuEvent::Activated { index, .. } = event {
                let _ = model.choose(index);
            }
            return event;
        }
        if self.trigger.pointer_up_at(p) {
            if model.menu.open {
                model.menu.open = false;
            } else {
                self.menu.show_with_model(&mut model.menu, 0);
                model.menu.items.selected =
                    model.selected.min(model.menu.items.count.saturating_sub(1));
            }
        } else if model.menu.open {
            model.menu.open = false;
        }
        MenuEvent::None
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut crate::Select) -> MenuEvent {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return MenuEvent::None;
        }
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        if model.menu.open {
            let event = self.menu.key_with_model(&mut model.menu, key, down);
            if let MenuEvent::Activated { index, .. } = event {
                let _ = model.choose(index);
            }
            // The owned trigger regains keyboard focus presentation when its
            // menu closes by keyboard, even if a pointer originally opened it.
            if down
                && matches!(
                    event,
                    MenuEvent::Activated { .. } | MenuEvent::Dismissed { .. }
                )
            {
                self.focus(true);
            }
            event
        } else if matches!(key, ControlKey::Enter | ControlKey::Space)
            && self.trigger.key(key, down)
        {
            self.menu.show_with_model(&mut model.menu, 0);
            model.menu.items.selected =
                model.selected.min(model.menu.items.count.saturating_sub(1));
            MenuEvent::None
        } else {
            MenuEvent::None
        }
    }
    /// Focus a mounted popup option without committing the selected value.
    pub fn focus_option(&mut self, index: usize, model: &mut crate::Select) -> bool {
        if !self.window_active || self.disabled {
            return false;
        }
        self.menu.focus_item_with_model(&mut model.menu, index)
    }
    /// Commit through the retained menu, then restore the original trigger focus.
    pub fn activate_option(&mut self, index: usize, model: &mut crate::Select) -> MenuEvent {
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        let event = self.menu.activate_item_with_model(&mut model.menu, index);
        if let MenuEvent::Activated { index, .. } = event {
            let _ = model.choose(index);
            self.focus(true);
        }
        event
    }
    pub fn wheel_event(
        &mut self,
        input: crate::feedback_view::MenuWheelInput,
        model: &crate::Select,
    ) -> crate::feedback_view::MenuWheelEvent {
        if !self.window_active || !model.menu.open || model.menu.items.count == 0 {
            return crate::feedback_view::MenuWheelEvent::Ignored;
        }
        self.menu.wheel_event_with_model(&model.menu, input)
    }
    pub fn focus(&mut self, visible: bool) {
        self.trigger.state.focused = true;
        self.trigger.state.focus_visible = visible;
    }
    pub fn blur(&mut self, model: &mut crate::Select) {
        self.menu.clear_presentation();
        self.trigger.blur();
        // Focus departure retires the interaction, not its actual window.
        // True window deactivation remains owned by set_window_active(false).
        let _ = self.menu.deactivate_with_model(&mut model.menu);
    }
    pub fn cancel(&mut self, model: &mut crate::Select) {
        self.menu.clear_presentation();
        self.trigger.cancel();
        model.menu.open = false;
        let _ = self
            .menu
            .set_window_active_with_model(&mut model.menu, false);
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut crate::Select) {
        self.window_active = active;
        let _ = self
            .menu
            .set_window_active_with_model(&mut model.menu, active);
        if !active {
            self.trigger.blur();
            self.cancel(model);
        }
    }
    pub fn layout(&self) -> &SelectLayout {
        &self.layout
    }
}
impl Default for SelectView {
    fn default() -> Self {
        Self::new(0)
    }
}
