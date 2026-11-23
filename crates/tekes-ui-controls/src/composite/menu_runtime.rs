use super::menu::{
    MenuEvent, MenuHeader, MenuLayout, MenuPreparedState, MenuRowStyle, MenuRowText, MenuView,
    MenuWheelEvent, MenuWheelInput, menu_popup_geometry, menu_surface_height,
    menu_surface_height_for_rows,
};
use super::{
    ButtonLayout, ButtonModel, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey,
    Insets, MENU_MAX_WIDTH, Menu, ScrollArea, clip_rect, explicit_visible_clip,
    menu_max_width_limit, menu_min_row_height, menu_popup_inset, menu_row_gap,
    menu_row_horizontal_inset, menu_row_radius, menu_separator_height, menu_size_role,
    menu_typography_scale, paint, safe_rect, set_button_state,
};
use crate::control_motion::OverlayPresence;
use tekes_ui_scene::{CornerRadii, Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Debug)]
pub(super) struct MenuRuntime {
    pub(super) presence: OverlayPresence,
    pub(super) buttons: Vec<ButtonModel>,
    pub(super) scroll: ScrollArea,
    pub(super) placement: MenuLayout,
    pub(super) return_focus: Option<usize>,
    pub(super) window_active: bool,
    pub(super) disabled: bool,
    pub(super) focus_visible: bool,
    pub(super) compact: bool,
    pub(super) row_heights: Vec<f32>,
    pub(super) scroll_viewport: Rect,
    pub(super) visible_bounds: Rect,
    pub(super) ancestor_clip: Option<Rect>,
    pub(super) palette: Palette,
    pub(super) role: proposal::TextRole,
    pub(super) scroll_gesture_captured: bool,
    pub(super) focus_outset: f32,
    pub(super) row_styles: Vec<MenuRowStyle>,
    pub(super) submenu_request: Option<usize>,
    pub(super) beside_anchor: bool,
    pub(super) open_generation: u64,
    pub(super) popup_min_width: Option<f32>,
    pub(super) popup_max_width: Option<f32>,
}

impl MenuRuntime {
    pub(super) fn replace_items_model(&mut self, model: &mut Menu, count: usize) {
        self.deactivate_model(model);
        self.scroll.cancel();
        self.scroll_gesture_captured = false;
        self.presence.clear();
        self.clear_layout();
        self.row_styles.clear();
        *model = Menu::new(count);
    }
    pub(super) fn select_prepared_state(&self, model: &Menu) -> MenuPreparedState {
        MenuPreparedState {
            model: (
                model.open,
                model.items.count,
                model.items.selected,
                model.last_action,
            ),
            generation: self.open_generation,
            availability: (self.window_active, self.disabled, self.focus_visible),
            return_focus: self.return_focus,
            layout: self.placement.clone(),
            row_styles: self.row_styles.clone(),
            row_heights: self.row_heights.clone(),
            viewport: self.scroll_viewport,
            offset: self.scroll.offset,
            limit: self.scroll.limit(),
            focus_outset: self.focus_outset,
            gesture: self.scroll_gesture_captured,
            buttons: self
                .buttons
                .iter()
                .map(ButtonModel::prepare_content_press)
                .collect(),
        }
    }
    pub(super) fn reveal_offset(&self, index: usize) -> f32 {
        if self.row_heights.is_empty() {
            return self.scroll.offset.y;
        }
        let index = index.min(self.row_heights.len() - 1);
        let row_gap = menu_row_gap();
        let top = self
            .row_heights
            .iter()
            .take(index)
            .map(|h| h + row_gap)
            .sum::<f32>();
        let bottom = top + self.row_heights[index];
        let offset = self.scroll.offset.y;
        let height = self.scroll_viewport.size.height.max(0.);
        let target = if top < offset {
            top
        } else if bottom > offset + height {
            bottom - height
        } else {
            offset
        };
        target.clamp(0., self.scroll.limit().y)
    }
    pub(super) fn row_style(&self, index: usize) -> MenuRowStyle {
        self.row_styles.get(index).copied().unwrap_or_default()
    }
    pub(super) fn item_enabled(&self, index: usize) -> bool {
        let row = self.row_style(index);
        !row.disabled && !row.separator
    }
    pub(super) fn first_enabled(&self, count: usize) -> Option<usize> {
        (0..count).find(|&index| self.item_enabled(index))
    }
    pub(super) fn move_enabled(
        &self,
        count: usize,
        before: usize,
        key: ControlKey,
    ) -> Option<usize> {
        match key {
            ControlKey::Home => self.first_enabled(count),
            ControlKey::End => (0..count).rev().find(|&index| self.item_enabled(index)),
            ControlKey::Up | ControlKey::Down if count > 0 => (1..=count)
                .map(|n| {
                    if key == ControlKey::Up {
                        (before % count + count - n) % count
                    } else {
                        (before % count + n) % count
                    }
                })
                .find(|&index| self.item_enabled(index)),
            _ => None,
        }
    }
    pub(super) fn semantic_item_mounted(&self, model: &Menu, index: usize) -> bool {
        !self.disabled
            && self.window_active
            && model.open
            && index < model.items.count
            && self.item_enabled(index)
            && self
                .placement
                .items
                .get(index)
                .is_some_and(|item| !item.frame.is_empty())
    }
    pub(super) fn focus_item_model(&mut self, model: &mut Menu, index: usize) -> bool {
        if !self.semantic_item_mounted(model, index) {
            return false;
        }
        model.items.selected = index;
        self.focus_visible = true;
        self.reveal_active(model);
        self.sync_focus(index);
        true
    }
    pub(super) fn activate_item_model(&mut self, model: &mut Menu, index: usize) -> MenuEvent {
        if !self.semantic_item_mounted(model, index) {
            return MenuEvent::None;
        }
        self.activate_model(model, index)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_model(
        &mut self,
        model: &mut Menu,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        anchor: Rect,
        visible_bounds: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
        mut header: Option<MenuHeader<'_>>,
    ) -> Option<MenuLayout> {
        if safe_rect(anchor) != anchor
            || safe_rect(visible_bounds) != visible_bounds
            || visible_bounds.size.width <= 0.0
            || visible_bounds.size.height <= 0.0
            || ancestor_clip.is_some_and(|clip| safe_rect(clip) != clip)
        {
            self.presence.clear();
            self.clear_layout();
            return None;
        }
        let header_size = header.as_ref().map(|(size, _)| *size);
        let paint_context = format!(
            "{header_size:?}{anchor:?}{visible_bounds:?}{ancestor_clip:?}{palette:?}{role:?}{compact:?}{}{labels:?}{:?}{:?}{:?}",
            text.backing_scale(),
            self.row_styles,
            self.popup_min_width,
            self.popup_max_width
        );
        if !model.open && !self.presence.is_presented() {
            self.clear_layout();
            self.presence.paint(scene, text, false, paint_context, None);
            return None;
        }
        let destination = scene;
        let mut overlay_scene = destination.fragment();
        let scene = &mut overlay_scene;
        self.compact = compact;
        self.palette = palette;
        self.role = role;
        let popup_inset = menu_popup_inset();
        let row_gap = menu_row_gap();
        let role_size = menu_size_role(compact);
        let min_row_height = menu_min_row_height(compact);
        let row_radius = menu_row_radius();
        let horiz_inset = menu_row_horizontal_inset(text, role);
        self.focus_outset = popup_inset;
        let anchor = safe_rect(anchor);
        let visible_bounds = safe_rect(visible_bounds);
        let old_scroll_viewport = self.scroll_viewport;
        let old_row_heights = self.row_heights.clone();
        self.visible_bounds = visible_bounds;
        self.ancestor_clip = ancestor_clip;
        let count = model.items.count.min(labels.len());
        self.buttons
            .resize_with(model.items.count, ButtonModel::default);
        let has_check = self.row_styles.iter().any(|style| style.checked.is_some());
        let has_submenu = self.row_styles.iter().any(|style| style.submenu);
        let type_scale = menu_typography_scale(text, role);
        let indicator_gap = if has_check || has_submenu {
            proposal::metric("spacing", "space-tight")
                .expect("canonical menu indicator-caption spacing")
                * type_scale
        } else {
            0.0
        };
        let icon = if has_check || has_submenu {
            proposal::metric("size", "size-icon-small").expect("canonical menu indicator size")
                * type_scale
        } else {
            0.0
        };
        let leading = if has_check {
            icon.max(text.measure("✓", role, f32::MAX).width) + indicator_gap
        } else {
            0.0
        };
        let trailing = if has_submenu {
            icon.max(text.measure("›", role, f32::MAX).width) + indicator_gap
        } else {
            0.0
        };
        let (mut natural_size, mut heights) = MenuView::measured_rows_bounded(
            &mut MenuRowText {
                text,
                leading,
                trailing,
                gap: indicator_gap,
                style: MenuRowStyle::default(),
            },
            &labels[..count],
            visible_bounds.size.width,
            role,
            compact,
            self.popup_min_width,
            self.popup_max_width,
        );
        if let Some(header) = header_size {
            let max_limit = menu_max_width_limit(
                visible_bounds.size.width,
                self.popup_max_width.unwrap_or(MENU_MAX_WIDTH),
            );
            let header_width = natural_size
                .width
                .max((header.width + popup_inset * 2.0).min(max_limit));
            if header_width > natural_size.width {
                natural_size.width = header_width;
                let mut row_text = MenuRowText {
                    text,
                    leading,
                    trailing,
                    gap: indicator_gap,
                    style: MenuRowStyle::default(),
                };
                heights = MenuView::measure_row_heights(
                    &mut row_text,
                    &labels[..count],
                    (natural_size.width - popup_inset * 2.0).max(0.0),
                    role,
                    compact,
                );
                natural_size.height = menu_surface_height_for_rows(&heights, popup_inset);
            }
        }
        let separator_height = menu_separator_height();
        for (index, height) in heights.iter_mut().enumerate() {
            if self.row_style(index).separator {
                *height = separator_height;
            }
        }
        if !self.row_styles.is_empty() {
            natural_size.height = menu_surface_height(
                heights.iter().sum::<f32>() + count.saturating_sub(1) as f32 * row_gap,
                popup_inset,
            );
        }
        heights.resize(model.items.count, min_row_height);
        self.row_heights = heights;
        let natural_content =
            self.row_heights.iter().take(count).sum::<f32>() + count.saturating_sub(1) as f32 * row_gap;
        let (popup, header_frame, inner_viewport) = menu_popup_geometry(
            anchor,
            natural_size,
            visible_bounds,
            popup_inset,
            &self.row_heights[..count],
            self.beside_anchor,
            header_size,
        );
        let popup_container =
            ContainerLayout::new(popup, Insets::ZERO, Insets::ZERO, ancestor_clip);
        let visible = popup_container.visible_clip();
        let document_height = natural_content.max(inner_viewport.size.height);
        self.scroll.layout(
            inner_viewport.size,
            Size::new(inner_viewport.size.width, document_height),
        );
        self.scroll_viewport = inner_viewport;
        if old_scroll_viewport != inner_viewport
            || old_row_heights != self.row_heights
            || self.placement.items.is_empty()
            || self.placement.active_item != model.items.selected
        {
            self.reveal_active(model);
        }
        let scroll_geometry = self.scroll.geometry(inner_viewport, visible);
        let offset = scroll_geometry.visual_offset().y;
        if let Some(visible) = visible {
            // The popup surface's AA edge must not be clipped by its own
            // frame. Keep it inside the real ancestor viewport, then restore
            // the established frame∩ancestor clip for rows and hit content.
            if let Some(background_clip) = popup_container.background_paint_clip() {
                scene.push_clip(background_clip);
            }
            paint::surface_kind_with_elevation_at_scale(
                scene,
                popup,
                palette,
                paint::SurfaceKind::Floating,
                text.backing_scale(),
                "menu",
            );
            if popup_container.background_paint_clip().is_some() {
                scene.pop_clip();
            }
            scene.push_clip(visible);
        }
        if let Some((_, paint_header)) = &mut header
            && visible.is_some()
        {
            paint_header(scene, text, header_frame);
        }
        let rows_visible = if header_size.is_some() {
            let top = (inner_viewport.min_y() - popup_inset).max(header_frame.max_y());
            let gutter = Rect::new(
                popup.min_x(),
                top,
                popup.size.width,
                (popup.max_y() - top).max(0.0),
            );
            visible.and_then(|clip| gutter.intersection(&clip))
        } else {
            visible
        };
        let mut items = Vec::with_capacity(model.items.count);
        let mut deferred_focus: Option<(Rect, Option<Rect>, tekes_ui_tokens::Rgba)> = None;
        let mut y = inner_viewport.min_y() - offset;
        for index in 0..model.items.count {
            let height = self.row_heights[index];
            let item = Rect::new(inner_viewport.min_x(), y, inner_viewport.size.width, height);
            let row_clip = explicit_visible_clip(rows_visible);
            let style = self.row_style(index);
            if style.separator {
                if let Some(clip) = rows_visible {
                    scene.push_clip(clip);
                }
                crate::extended::separator_at_scale(
                    scene,
                    item,
                    palette,
                    crate::extended::Axis::Horizontal,
                    text.backing_scale(),
                );
                if rows_visible.is_some() {
                    scene.pop_clip();
                }
                items.push(ButtonLayout {
                    frame: item,
                    content: item,
                    visible: clip_rect(item, rows_visible),
                    label_size: Size::default(),
                });
                y += height + row_gap;
                continue;
            }
            let button = &mut self.buttons[index];
            button.set_surface("popover");
            button.set_selection_fill_only(true);
            button.set_corner_radii(Some(CornerRadii::uniform(row_radius)));
            button.set_horizontal_inset_override(Some((horiz_inset, horiz_inset)));
            button.set_defer_focus_ring(true);
            set_button_state(
                button,
                self.window_active && model.open && model.items.selected == index,
                self.window_active
                    && model.open
                    && self.focus_visible
                    && model.items.selected == index,
                self.disabled || style.disabled,
                model.items.selected == index && !style.disabled,
            );
            let label = labels.get(index).copied().unwrap_or("");
            let layout = button.render(
                scene,
                &mut MenuRowText {
                    text,
                    leading,
                    trailing,
                    gap: indicator_gap,
                    style,
                },
                label,
                item,
                row_clip,
                palette,
                paint::ButtonVariant::Quiet,
                role_size,
                role,
                ButtonTextAlign::Leading,
            );
            if layout.visible.is_some()
                && button.state.focused
                && button.state.focus_visible
                && !button.state.disabled
                && let Some(focus_color) = paint::button_surface_colors_with_tone_and_selection_progress(
                    palette,
                    "popover",
                    button.state,
                    paint::ButtonVariant::Quiet,
                    0.0,
                    0.0,
                    button.tone,
                    button.transparent_rest,
                    button.selection_progress_override,
                )
                .focus
            {
                deferred_focus = Some((item, rows_visible, focus_color));
            }
            items.push(layout);
            y += height + row_gap;
        }
        if let Some((focus_rect, focus_clip, focus_color)) = deferred_focus {
            let pushed = if let Some(clip) = focus_clip {
                scene.push_clip(clip);
                true
            } else {
                false
            };
            paint::focus_ring_with_radii(
                scene,
                focus_rect,
                focus_color,
                CornerRadii::uniform(row_radius),
            );
            if pushed {
                scene.pop_clip();
            }
        }
        if visible.is_some() {
            scene.pop_clip();
        }
        self.placement = MenuLayout {
            anchor,
            popup,
            visible,
            items,
            active_item: model.items.selected,
            scroll_offset: offset,
        };
        self.presence.paint(
            destination,
            text,
            model.open,
            paint_context,
            Some(overlay_scene),
        );
        if !model.open {
            self.clear_layout();
            return None;
        }
        Some(self.placement.clone())
    }

    pub(super) fn pointer_move_model(&mut self, model: &mut Menu, point: Point) {
        if !model.open || !self.window_active || self.disabled {
            return;
        }
        let hovered = self
            .placement
            .items
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                (self.item_enabled(index) && item.hit_test(point)).then_some(index)
            });
        if let Some(index) = hovered {
            model.items.selected = index;
            self.focus_visible = false;
            self.sync_focus(index);
            if self.row_style(index).submenu {
                self.submenu_request = Some(index);
            }
        }
        for button in &mut self.buttons {
            button.pointer_move_at(point);
        }
    }

    pub(super) fn pointer_down_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
        if !model.open || !self.window_active {
            return MenuEvent::None;
        }
        let Some(event_bounds) = clip_rect(self.visible_bounds, self.ancestor_clip) else {
            return MenuEvent::None;
        };
        if !event_bounds.contains(point) {
            return MenuEvent::None;
        }
        if !self.placement.popup.contains(point) {
            return self.dismiss_model(model);
        }
        if self.disabled {
            return MenuEvent::None;
        }
        if let Some(index) = self
            .placement
            .items
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                (self.item_enabled(index) && item.hit_test(point)).then_some(index)
            })
        {
            model.items.selected = index;
            self.focus_visible = false;
            self.sync_focus(index);
            self.buttons[index].pointer_down_at(point);
        }
        MenuEvent::None
    }

    pub(super) fn pointer_up_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
        if !model.open || !self.window_active {
            return MenuEvent::None;
        }
        for index in 0..self.buttons.len() {
            if self.buttons[index].pointer_up_at(point) && self.item_enabled(index) {
                return self.activate_model(model, index);
            }
        }
        MenuEvent::None
    }

    pub(super) fn key_model(&mut self, model: &mut Menu, key: ControlKey, down: bool) -> MenuEvent {
        if !model.open || !self.window_active {
            return MenuEvent::None;
        }
        if down && key == ControlKey::Escape {
            return self.dismiss_model(model);
        }
        if self.disabled {
            return MenuEvent::None;
        }
        if !down {
            let index = model.items.selected;
            if self
                .buttons
                .get_mut(index)
                .is_some_and(|button| button.key(key, false))
            {
                return self.activate_model(model, index);
            }
            return MenuEvent::None;
        }
        match key {
            ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End => {
                self.focus_visible = true;
                let before = model.items.selected;
                if let Some(index) = self.move_enabled(model.items.count, before, key) {
                    model.items.selected = index;
                }
                self.reveal_active(model);
                self.sync_focus(model.items.selected);
                MenuEvent::None
            }
            ControlKey::Enter | ControlKey::Space => {
                self.focus_visible = true;
                let index = model.items.selected;
                if self
                    .placement
                    .items
                    .get(index)
                    .is_none_or(|item| item.visible.is_none())
                {
                    return MenuEvent::None;
                }
                self.sync_focus(index);
                self.activate_model(model, index)
            }
            ControlKey::Right if self.row_style(model.items.selected).submenu => {
                let index = model.items.selected;
                if self.semantic_item_mounted(model, index) {
                    self.submenu_request = Some(index);
                }
                MenuEvent::None
            }
            _ => MenuEvent::None,
        }
    }

    pub(super) fn wheel_event(&mut self, model: &Menu, input: MenuWheelInput) -> MenuWheelEvent {
        if !model.open || !self.window_active {
            return MenuWheelEvent::Ignored;
        }
        if input.begin {
            self.scroll_gesture_captured = input.precise
                && self
                    .placement
                    .visible
                    .is_some_and(|visible| visible.contains(input.point))
                && self.placement.popup.contains(input.point);
        }
        let captured = self.scroll_gesture_captured;
        let inside_popup = self
            .placement
            .visible
            .is_some_and(|visible| visible.contains(input.point))
            && self.placement.popup.contains(input.point);
        if !captured && !inside_popup {
            return MenuWheelEvent::Ignored;
        }
        let scrolled = self.scroll.wheel_at(
            input.delta,
            input.precise,
            input.begin,
            input.end,
            input.momentum,
            input.time,
        );
        if input.end {
            self.scroll_gesture_captured = false;
        }
        MenuWheelEvent::Consumed { scrolled }
    }

    pub(super) fn deactivate_model(&mut self, model: &mut Menu) -> MenuEvent {
        self.cancel_buttons();
        self.clear_button_pointer_positions();
        if model.open {
            self.dismiss_model(model)
        } else {
            MenuEvent::None
        }
    }

    pub(super) fn activate_model(&mut self, model: &mut Menu, index: usize) -> MenuEvent {
        if index >= model.items.count || !self.item_enabled(index) {
            return MenuEvent::None;
        }
        if self.row_style(index).submenu {
            model.items.selected = index;
            self.reveal_active(model);
            self.sync_focus(index);
            self.submenu_request = Some(index);
            return MenuEvent::None;
        }
        model.items.selected = index;
        model.last_action = Some(index);
        model.open = false;
        self.clear_layout();
        MenuEvent::Activated {
            index,
            restore_focus: self.return_focus.take(),
        }
    }

    pub(super) fn dismiss_model(&mut self, model: &mut Menu) -> MenuEvent {
        if !model.open {
            return MenuEvent::None;
        }
        model.open = false;
        self.clear_layout();
        MenuEvent::Dismissed {
            restore_focus: self.return_focus.take(),
        }
    }

    pub(super) fn reveal_active(&mut self, model: &Menu) {
        let index = model
            .items
            .selected
            .min(self.row_heights.len().saturating_sub(1));
        if self.row_heights.is_empty() {
            return;
        }
        let row_gap = menu_row_gap();
        let top = self
            .row_heights
            .iter()
            .take(index)
            .map(|h| h + row_gap)
            .sum::<f32>();
        let bottom = top + self.row_heights[index];
        let viewport_height = self.scroll_viewport.size.height.max(0.0);
        let offset = self.scroll.offset.y;
        if top < offset {
            self.scroll.scroll_by(0.0, top - offset);
        } else if bottom > offset + viewport_height {
            self.scroll
                .scroll_by(0.0, bottom - (offset + viewport_height));
        }
    }

    pub(super) fn cancel_buttons(&mut self) {
        for button in &mut self.buttons {
            button.cancel();
        }
        self.scroll.cancel();
        self.scroll_gesture_captured = false;
    }

    pub(super) fn sync_focus(&mut self, selected: usize) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            let focused = index == selected && self.window_active;
            if button.state.focused && !focused {
                button.state.focused = false;
                button.state.focus_visible = false;
                let _ = button.key(ControlKey::Space, false);
            }
            if focused {
                button.state.focused = true;
                button.state.focus_visible = self.focus_visible;
            }
        }
    }

    pub(super) fn clear_layout(&mut self) {
        self.submenu_request = None;
        self.cancel_buttons();
        self.placement = MenuLayout::default();
        self.focus_visible = false;
    }

    /// A deactivated menu no longer owns the pointer's previous location.
    pub(super) fn clear_button_pointer_positions(&mut self) {
        for button in &mut self.buttons {
            button.blur();
        }
    }
}
