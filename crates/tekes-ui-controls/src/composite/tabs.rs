use super::{
    ButtonLayout, ButtonModel, ButtonTextAlign, ButtonTextRenderer, ControlKey, Tabs, clip_rect,
    empty_button_layout, focus_reveal_outset, paint, row_height, safe_rect, set_button_state,
    size_role,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabsLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    /// One entry per tab, in model order. Fully clipped tabs have `visible=None`.
    pub items: Vec<ButtonLayout>,
    /// Independent close targets, indexed by the original tab identity.
    pub close_buttons: Vec<Option<ButtonLayout>>,
}

/// ButtonModel-backed, roving-tabstop presentation for the existing `Tabs` model.
#[derive(Clone, Debug)]
pub struct TabsView {
    pub(super) toolbar: bool,
    pub(super) model: Tabs,
    pub(super) buttons: Vec<ButtonModel>,
    pub(super) close_buttons: Vec<ButtonModel>,
    pub(super) close_resources: Vec<Option<crate::composition::IconImage>>,
    pub(super) close_icon_error: Option<crate::composition::IconRenderError>,
    pub(super) closable: Vec<bool>,
    pub(super) visible_items: Vec<bool>,
    pub(super) close_press: Option<usize>,
    pub(super) close_focused: Option<usize>,
    pub(super) close_request: Option<usize>,
    pub(super) placement: TabsLayout,
    pub(super) focus_visible: bool,
    pub(super) focus_index: usize,
    pub(super) focused: Option<usize>,
    pub(super) window_active: bool,
    pub(super) disabled: bool,
    pub(super) disabled_items: Vec<bool>,
    pub(super) compact: bool,
    pub(super) press_target: Option<usize>,
    pub(super) surface: &'static str,
}

impl TabsView {
    /// Preserve the original roving stop and arm while adopting a declared parent.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        self.surface = surface;
        for button in &mut self.buttons {
            button.set_surface(surface);
        }
        for button in &mut self.close_buttons {
            button.set_surface(surface);
        }
        Ok(())
    }
    pub fn new(count: usize) -> Self {
        Self {
            toolbar: false,
            model: Tabs::new(count),
            buttons: vec![ButtonModel::default(); count],
            close_buttons: vec![ButtonModel::default(); count],
            close_resources: vec![None; count],
            close_icon_error: None,
            closable: vec![false; count],
            visible_items: vec![true; count],
            close_press: None,
            close_focused: None,
            close_request: None,
            placement: TabsLayout::default(),
            focus_visible: false,
            focus_index: 0,
            focused: None,
            window_active: true,
            disabled: false,
            disabled_items: vec![false; count],
            compact: false,
            press_target: None,
            surface: "canvas",
        }
    }

    pub(crate) fn set_toolbar_presentation(&mut self, toolbar: bool) {
        self.toolbar = toolbar;
    }

    pub fn selected(&self) -> usize {
        self.model.selected
    }

    pub fn select(&mut self, index: usize) -> bool {
        if self.disabled_items.get(index).copied().unwrap_or(true)
            || !self.visible_items.get(index).copied().unwrap_or(false)
        {
            return false;
        }
        let changed = self.model.select(index);
        if changed {
            self.cancel_arms();
        }
        changed
    }

    /// Moves the selected/roving stop as a host restores keyboard focus.
    pub fn focus(&mut self, index: usize, focus_visible: bool) -> bool {
        if index >= self.model.count || self.disabled_items[index] || !self.visible_items[index] {
            return false;
        }
        self.model.selected = index;
        self.focus_index = index;
        self.focused = Some(index);
        self.close_focused = None;
        self.focus_visible = focus_visible;
        self.sync_focus_buttons();
        true
    }

    /// Closing is an intent, separate from selection and owned by the host.
    pub fn set_closable(&mut self, index: usize, closable: bool) {
        if let Some(value) = self.closable.get_mut(index) {
            *value = closable;
            if !closable {
                self.close_buttons[index].blur();
                self.close_resources[index] = None;
                self.cancel_arms();
                if self.close_focused == Some(index) {
                    self.close_focused = None;
                }
            }
        }
    }
    pub fn take_close_request(&mut self) -> Option<usize> {
        self.close_request.take()
    }
    /// Retain the backing tab and revoke its old input geometry while hidden.
    pub fn set_item_visible(&mut self, index: usize, visible: bool) {
        let Some(value) = self.visible_items.get_mut(index) else {
            return;
        };
        if *value == visible {
            return;
        }
        *value = visible;
        self.cancel_arms();
        self.buttons[index].clear_layout();
        self.close_buttons[index].clear_layout();
        if !visible {
            self.close_resources[index] = None;
        }
        if let Some(item) = self.placement.items.get_mut(index) {
            *item = empty_button_layout();
        }
        if let Some(item) = self.placement.close_buttons.get_mut(index) {
            *item = None;
        }
        if self.focused == Some(index) || self.close_focused == Some(index) {
            self.clear_focus();
        }
        if !self
            .visible_items
            .get(self.model.selected)
            .copied()
            .unwrap_or(false)
        {
            let candidate = (index + 1..self.model.count)
                .chain((0..index).rev())
                .find(|&i| self.visible_items[i] && !self.disabled_items[i]);
            if let Some(candidate) = candidate {
                self.model.selected = candidate;
            } else if visible && !self.disabled_items[index] {
                self.model.selected = index;
            }
        }
    }
    pub(crate) fn close_focused_index(&self) -> Option<usize> {
        self.close_focused
    }
    pub(crate) fn close_press_index(&self) -> Option<usize> {
        self.close_press
    }
    pub(crate) fn pointer_press_index(&self) -> Option<usize> {
        self.press_target
    }
    /// Latest explicit close artwork failure for direct View callers.
    pub fn close_icon_error(&self) -> Option<crate::composition::IconRenderError> {
        self.close_icon_error
    }
    pub(crate) fn focus_close(&mut self, index: usize, visible: bool) -> bool {
        if !self.close_available(index) {
            return false;
        }
        if self.close_focused != Some(index) {
            self.cancel_arms();
        }
        self.focused = None;
        self.close_focused = Some(index);
        self.focus_visible = visible;
        self.sync_focus_buttons();
        true
    }
    fn close_available(&self, index: usize) -> bool {
        !self.disabled
            && self.window_active
            && self.closable.get(index).copied().unwrap_or(false)
            && self.visible_items[index]
            && !self.disabled_items[index]
    }

    pub(crate) fn set_item_disabled(&mut self, index: usize, disabled: bool) {
        if let Some(item) = self.disabled_items.get_mut(index) {
            *item = disabled;
        }
        if let Some(button) = self.buttons.get_mut(index) {
            button.state.disabled = self.disabled || disabled;
            if disabled {
                button.cancel();
            }
        }
        if disabled && self.focused == Some(index) {
            self.clear_focus();
        }
        if disabled && self.close_focused == Some(index) {
            self.clear_focus();
        }
        if let Some(button) = self.close_buttons.get_mut(index) {
            button.state.disabled = self.disabled || disabled;
            if disabled {
                button.blur();
            }
        }
        self.cancel_arms();
    }
    pub(crate) fn focused_index(&self) -> Option<usize> {
        self.focused
    }
    pub(crate) fn placement(&self) -> &TabsLayout {
        &self.placement
    }
    pub(crate) fn content_theme(
        &self,
        index: usize,
        palette: Palette,
    ) -> Option<(tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba)> {
        self.buttons.get(index).map(|button| {
            let colors = button.surface_colors(palette, paint::ButtonVariant::Quiet, self.surface);
            (colors.text, colors.icon)
        })
    }
    // Natural original Content sizes decide only whether the existing lanes
    // can be rebalanced. When equal lanes fit, or natural demands cannot fit
    // within the original budget/cap, retain the established equal geometry.
    pub(crate) fn content_lanes(
        text: &mut dyn ButtonTextRenderer,
        natural: &[Size],
        width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> Vec<f32> {
        if natural.is_empty() {
            return Vec::new();
        }
        let count = natural.len();
        let equal = (width / count as f32).min(160.);
        let gap = focus_reveal_outset(text.backing_scale()).max(4.);
        let wanted: Vec<_> = natural
            .iter()
            .map(|size| {
                ButtonModel::measure_content(text, *size, f32::MAX, size_role(compact), role).width
                    + gap
            })
            .collect();
        let budget = width.min(160. * count as f32);
        let total: f32 = wanted.iter().sum();
        if wanted.iter().all(|need| *need <= equal)
            || wanted.iter().any(|need| *need > 160.)
            || total > budget
        {
            return vec![equal; count];
        }
        let mut lanes = wanted;
        let mut remaining = (budget - total).max(0.);
        while remaining > 0.0001 {
            let open = lanes.iter().filter(|lane| **lane < 160.).count();
            if open == 0 {
                break;
            }
            let share = remaining / open as f32;
            let before = remaining;
            for lane in &mut lanes {
                let extra = share.min(160. - *lane);
                *lane += extra;
                remaining = (remaining - extra).max(0.);
            }
            if remaining >= before {
                break;
            }
        }
        lanes
    }
    pub(crate) fn content_lane_width(text: &mut dyn ButtonTextRenderer, advance: f32) -> f32 {
        (advance - focus_reveal_outset(text.backing_scale()).max(4.)).max(0.)
    }
    pub(crate) fn close_width(text: &mut dyn ButtonTextRenderer) -> f32 {
        let role = proposal::TextRole::UiBody;
        24. * (text.resolved_font_size(role) / role.style().size).max(1.)
    }
    pub(crate) fn content_height(
        text: &mut dyn ButtonTextRenderer,
        sizes: &[Size],
        lanes: &[f32],
        compact: bool,
    ) -> f32 {
        let gap = focus_reveal_outset(text.backing_scale()).max(4.);
        sizes
            .iter()
            .zip(lanes)
            .fold(row_height(compact), |height, (size, lane)| {
                height.max(
                    ButtonModel::measure_content(
                        text,
                        *size,
                        (*lane - gap).max(0.),
                        size_role(compact),
                        proposal::TextRole::UiBody,
                    )
                    .height,
                )
            })
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.focus_visible = false;
            self.focused = None;
            self.close_focused = None;
            self.cancel_arms();
            for button in &mut self.buttons {
                button.blur();
            }
            for button in &mut self.close_buttons {
                button.blur();
            }
            self.sync_focus_buttons();
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.cancel_arms();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> TabsLayout {
        self.render_inner(
            scene, text, labels, None, None, frame, clip, palette, role, compact,
        )
    }

    /// Render the same Tabs interaction chrome around caller-owned decorative
    /// Content labels. The labels remain mounted by the composition host; this
    /// view only measures their content and supplies the original ButtonModel
    /// frames, focus, hover, press and hit geometry.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_content(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        content_sizes: &[Size],
        lanes: &[f32],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> TabsLayout {
        self.render_inner(
            scene,
            text,
            &[],
            Some(content_sizes),
            Some(lanes),
            frame,
            clip,
            palette,
            role,
            compact,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_inner(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        content_sizes: Option<&[Size]>,
        lane_advances: Option<&[f32]>,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> TabsLayout {
        self.compact = compact;
        self.close_icon_error = None;
        if safe_rect(frame) != frame
            || frame.size.width <= 0.0
            || frame.size.height <= 0.0
            || clip.is_some_and(|clip| safe_rect(clip) != clip)
        {
            self.clear_focus();
            self.cancel_arms();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            for button in &mut self.close_buttons {
                button.clear_layout();
            }
            self.close_resources.fill(None);
            self.placement = TabsLayout::default();
            return self.placement.clone();
        }
        let frame = safe_rect(frame);
        self.buttons
            .resize_with(self.model.count, ButtonModel::default);
        let lanes = if let Some(lanes) = lane_advances {
            lanes.to_vec()
        } else {
            let close_width = Self::close_width(text);
            let natural: Vec<_> = (0..self.model.count)
                .filter(|&i| self.visible_items[i])
                .map(|index| {
                    let mut size = text.measure(
                        labels.get(index).copied().unwrap_or(""),
                        role,
                        frame.size.width,
                    );
                    if self.closable[index] {
                        size.width += close_width;
                    }
                    size
                })
                .collect();
            let visible_lanes =
                Self::content_lanes(text, &natural, frame.size.width, role, compact);
            let mut next = visible_lanes.into_iter();
            self.visible_items
                .iter()
                .map(|visible| {
                    if *visible {
                        next.next().unwrap_or(0.)
                    } else {
                        0.
                    }
                })
                .collect()
        };
        let toolbar_geometry = self.toolbar.then(|| crate::toolbar::tab_geometry(text));
        let lane_gap = toolbar_geometry.map_or_else(
            || focus_reveal_outset(text.backing_scale()).max(4.),
            |(padding, _)| padding,
        );
        let body = proposal::TextRole::UiBody;
        let toolbar_scale = text.resolved_font_size(body) / body.style().size;
        let close_width = if self.toolbar {
            24. * toolbar_scale
        } else {
            Self::close_width(text)
        };
        let base_height = row_height(compact);
        let mut measured_heights = Vec::with_capacity(self.model.count);
        for (index, lane) in lanes.iter().copied().enumerate().take(self.model.count) {
            if !self.visible_items[index] {
                measured_heights.push(0.);
                continue;
            }
            let lane_width = (lane
                - lane_gap
                - if self.closable[index] {
                    close_width
                } else {
                    0.
                })
            .max(0.);
            let label = labels.get(index).copied().unwrap_or("");
            let measured = content_sizes
                .and_then(|sizes| sizes.get(index).copied())
                .map(|size| {
                    ButtonModel::measure_content(text, size, lane_width, size_role(compact), role)
                        .height
                })
                .unwrap_or_else(|| {
                    ButtonModel::measure(text, label, lane_width, size_role(compact), role).height
                });
            measured_heights
                .push(toolbar_geometry.map_or(base_height.max(measured), |(_, height)| height));
        }
        let height = measured_heights.iter().copied().fold(
            toolbar_geometry.map_or(base_height, |(_, height)| height),
            f32::max,
        );
        let component_frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, height);
        let visible = clip_rect(component_frame, clip);
        let mut items = Vec::with_capacity(self.model.count);
        let mut close_layouts = Vec::with_capacity(self.model.count);
        let mut x = frame.min_x();
        for (index, measured_height) in measured_heights.iter().copied().enumerate() {
            if !self.visible_items[index] {
                self.buttons[index].clear_layout();
                self.close_buttons[index].clear_layout();
                self.close_resources[index] = None;
                items.push(empty_button_layout());
                close_layouts.push(None);
                continue;
            }
            let label = labels.get(index).copied().unwrap_or("");
            let item = Rect::new(
                x,
                frame.min_y(),
                (lanes[index] - lane_gap).max(0.),
                measured_height,
            );
            // The Tabs frame is an internal layout boundary, not a clipping
            // viewport. Keep focus-ring AA inside the true ancestor clip.
            let item_clip = clip;
            let button = &mut self.buttons[index];
            button.set_surface(self.surface);
            button.frame_policy = if self.toolbar {
                crate::ButtonFramePolicy::Exact
            } else {
                crate::ButtonFramePolicy::Minimum
            };
            button.set_selection_fill_only(true);
            set_button_state(
                button,
                self.window_active && self.focused == Some(index),
                self.window_active && self.focus_visible && self.focused == Some(index),
                self.disabled || self.disabled_items[index],
                self.model.selected == index,
            );
            let mut layout = if let Some(size) = content_sizes.and_then(|sizes| sizes.get(index)) {
                button.render_content_frame(
                    scene,
                    text,
                    *size,
                    item,
                    item_clip,
                    palette,
                    paint::ButtonVariant::Quiet,
                )
            } else if self.closable[index] {
                let insets = ButtonModel::measure_content(
                    text,
                    Size::default(),
                    item.size.width,
                    size_role(compact),
                    role,
                )
                .width;
                let width = (item.size.width - close_width - insets).max(0.);
                let size = text.measure(label, role, width);
                button.render_content_frame(
                    scene,
                    text,
                    size,
                    item,
                    item_clip,
                    palette,
                    paint::ButtonVariant::Quiet,
                )
            } else {
                button.render(
                    scene,
                    text,
                    label,
                    item,
                    item_clip,
                    palette,
                    paint::ButtonVariant::Quiet,
                    size_role(compact),
                    role,
                    ButtonTextAlign::Leading,
                )
            };
            if self.toolbar {
                let inset = proposal::metric("spacing", "space-relaxed").expect("tab title inset")
                    * toolbar_scale;
                let size = content_sizes
                    .and_then(|sizes| sizes.get(index))
                    .copied()
                    .unwrap_or(layout.label_size);
                layout.content = Rect::new(
                    item.min_x() + inset,
                    item.center().y - size.height * 0.5,
                    (item.size.width - 2. * inset).max(0.),
                    size.height,
                );
            }
            let close_layout = if self.closable[index] {
                let width = close_width.min((item.size.width * 0.5).max(0.));
                let close_frame =
                    Rect::new(item.max_x() - width, item.min_y(), width, item.size.height);
                layout.content.size.width = (close_frame.min_x() - layout.content.min_x()).max(0.);
                let close = &mut self.close_buttons[index];
                close.set_surface(self.surface);
                close.frame_policy = crate::ButtonFramePolicy::Exact;
                set_button_state(
                    close,
                    self.window_active && self.close_focused == Some(index),
                    self.window_active && self.focus_visible && self.close_focused == Some(index),
                    self.disabled || self.disabled_items[index],
                    false,
                );
                let close_layout = close.render_transparent_content_frame(
                    scene,
                    text,
                    Size::default(),
                    close_frame,
                    item_clip,
                    palette,
                    paint::ButtonVariant::Quiet,
                );
                if let Some(visible) = close_layout.visible {
                    let body = proposal::TextRole::UiBody;
                    let scale = if self.toolbar {
                        toolbar_scale
                    } else {
                        (text.resolved_font_size(body) / body.style().size).max(1.)
                    };
                    let side = (proposal::metric("size", "size-icon-standard")
                        .expect("Tabs close icon size token")
                        * scale)
                        .min(close_frame.size.width)
                        .min(close_frame.size.height);
                    let artboard = Rect::new(
                        close_frame.center().x - side * 0.5,
                        close_frame.center().y - side * 0.5,
                        side,
                        side,
                    );
                    let tint = close
                        .surface_colors(palette, paint::ButtonVariant::Quiet, self.surface)
                        .icon;
                    let backing = text.backing_scale();
                    let image = text
                        .icon_renderer()
                        .ok_or(crate::composition::IconRenderError::UnsupportedRenderer)
                        .and_then(|icons| {
                            icons.prepare_icon("tekes-remove-one", side, backing, tint)
                        });
                    match image {
                        Ok(image) => {
                            scene.push_clip(visible);
                            scene.push_image(tekes_ui_scene::ImageQuad {
                                rect: artboard,
                                image: image.id(),
                                opacity: 1.,
                            });
                            scene.pop_clip();
                            self.close_resources[index] = Some(image);
                        }
                        Err(error) => {
                            self.close_resources[index] = None;
                            self.close_icon_error = Some(error);
                        }
                    }
                } else {
                    self.close_resources[index] = None;
                }
                Some(close_layout)
            } else {
                self.close_buttons[index].clear_layout();
                self.close_resources[index] = None;
                None
            };
            if content_sizes.is_none() && self.closable[index] {
                let colors =
                    button.surface_colors(palette, paint::ButtonVariant::Quiet, self.surface);
                text.draw(
                    scene,
                    label,
                    role,
                    layout.content,
                    ButtonTextAlign::Leading,
                    colors.text,
                );
            }
            items.push(layout);
            close_layouts.push(close_layout);
            x += lanes[index];
        }
        self.placement = TabsLayout {
            frame: component_frame,
            visible,
            items,
            close_buttons: close_layouts,
        };
        if !visible.is_some_and(|rect| rect.size.width > 0.0 && rect.size.height > 0.0)
            || self.focused.is_some_and(|index| {
                self.placement
                    .items
                    .get(index)
                    .is_none_or(|item| item.visible.is_none())
            })
            || self.close_focused.is_some_and(|index| {
                self.placement
                    .close_buttons
                    .get(index)
                    .is_none_or(|item| item.is_none_or(|item| item.visible.is_none()))
            })
        {
            self.clear_focus();
            self.cancel_arms();
        }
        self.placement.clone()
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        if !self.window_active || self.disabled {
            return;
        }
        for button in &mut self.buttons {
            button.pointer_move_at(point);
        }
        for button in &mut self.close_buttons {
            button.pointer_move_at(point);
        }
    }

    pub(crate) fn close_target_at(&self, point: Point) -> Option<usize> {
        self.placement
            .close_buttons
            .iter()
            .position(|layout| layout.is_some_and(|layout| layout.hit_test(point)))
    }

    pub fn pointer_down_at(&mut self, point: Point) -> Option<usize> {
        if !self.window_active || self.disabled {
            return None;
        }
        self.close_request = None;
        if let Some(index) = self.close_target_at(point) {
            self.cancel_arms();
            if !self.close_available(index) {
                return None;
            }
            self.focused = None;
            self.close_focused = Some(index);
            self.focus_visible = false;
            self.sync_focus_buttons();
            self.close_press = self.close_buttons[index]
                .pointer_down_at(point)
                .then_some(index);
            return self.close_press;
        }
        let target = self
            .placement
            .items
            .iter()
            .position(|item| item.hit_test(point))?;
        if target >= self.model.count || self.disabled_items[target] || !self.visible_items[target]
        {
            return None;
        }
        self.focus_visible = false;
        self.focus_index = target;
        self.focused = Some(target);
        self.close_focused = None;
        self.sync_focus_buttons();
        self.press_target = self.buttons[target]
            .pointer_down_at(point)
            .then_some(target);
        Some(target)
    }

    pub fn pointer_up_at(&mut self, point: Point) -> Option<usize> {
        if !self.window_active {
            self.cancel_arms();
            return None;
        }
        if let Some(index) = self.close_press.take() {
            let activated =
                self.close_available(index) && self.close_buttons[index].pointer_up_at(point);
            self.cancel_arms();
            if activated {
                self.close_request = Some(index);
            }
            return None;
        }
        let target = self.press_target.take()?;
        let activated = self.buttons[target].pointer_up_at(point);
        self.cancel_other_arms(target);
        if activated {
            self.model.select(target);
            self.focus_index = target;
            self.sync_focus_buttons();
            Some(target)
        } else {
            self.focus_index = self.model.selected;
            self.focused = Some(self.model.selected);
            self.sync_focus_buttons();
            None
        }
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> Option<usize> {
        if !self.window_active || self.disabled {
            self.cancel_arms();
            return None;
        }
        if let Some(index) = self.close_focused {
            if self.close_available(index) && self.close_buttons[index].key(key, down) {
                self.close_request = Some(index);
            }
            return None;
        }
        let focused = self.focused?;
        if !down {
            let selected = focused;
            let activated = self
                .buttons
                .get_mut(selected)
                .is_some_and(|button| button.key(key, false));
            if activated {
                self.model.select(selected);
            }
            return activated.then_some(selected);
        }
        if matches!(
            key,
            ControlKey::Left | ControlKey::Right | ControlKey::Home | ControlKey::End
        ) {
            let previous = self.model.selected;
            self.focus_visible = true;
            if self.model.key(key) {
                for _ in 0..self.model.count {
                    if !self.disabled_items[self.model.selected]
                        && self.visible_items[self.model.selected]
                    {
                        break;
                    }
                    self.model.key(if key == ControlKey::Home {
                        ControlKey::Right
                    } else if key == ControlKey::End {
                        ControlKey::Left
                    } else {
                        key
                    });
                }
                self.focus_index = self.model.selected;
                self.focused = Some(self.model.selected);
                self.cancel_arms();
                self.sync_focus_buttons();
                return Some(self.model.selected);
            }
            return (previous != self.model.selected).then_some(self.model.selected);
        }
        let selected = focused;
        if key == ControlKey::Enter {
            self.focus_visible = true;
            self.sync_focus_buttons();
        }
        let activated = self
            .buttons
            .get_mut(selected)
            .is_some_and(|button| button.key(key, true));
        if activated {
            self.model.select(selected);
        }
        activated.then_some(selected)
    }

    pub fn cancel(&mut self) {
        self.cancel_arms();
        self.focus_visible = false;
        self.focus_index = self.model.selected;
        self.sync_focus_buttons();
    }

    pub fn clear_focus(&mut self) {
        self.focused = None;
        self.close_focused = None;
        self.focus_visible = false;
        self.cancel_arms();
        self.sync_focus_buttons();
    }

    fn sync_focus_buttons(&mut self) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if button.state.focused && self.focused != Some(index) {
                button.state.focused = false;
                button.state.focus_visible = false;
                let _ = button.key(ControlKey::Space, false);
            }
        }
        if let Some(index) = self.focused
            && let Some(button) = self.buttons.get_mut(index)
        {
            button.state.focused = true;
            button.state.focus_visible = self.focus_visible;
        }
        for (index, button) in self.close_buttons.iter_mut().enumerate() {
            if self.close_focused == Some(index) {
                button.state.focused = true;
                button.state.focus_visible = self.focus_visible;
            } else if button.state.focused {
                button.blur();
            }
        }
    }

    fn cancel_arms(&mut self) {
        self.press_target = None;
        self.close_press = None;
        self.close_request = None;
        for button in &mut self.buttons {
            button.cancel();
        }
        for button in &mut self.close_buttons {
            button.cancel();
        }
    }

    fn cancel_other_arms(&mut self, except: usize) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if index != except {
                button.cancel();
            }
        }
    }
}
