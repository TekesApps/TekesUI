//! Complete production presentations for controls composed from Buttons.
//!
//! These runtimes keep the old public `Tabs`, `Menu`, and `Dialog` models while
//! centralizing their layout, clipping, paint, pointer capture, keyboard focus,
//! and activation behavior. A runtime is bound either to its owned model or to
//! one external model for its lifetime; do not alternate between them.
use crate::control_motion::OverlayPresence;
use crate::{
    Button, ButtonLayout, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout,
    ControlKey, Dialog, Insets, Menu, ScrollArea, ScrollPolicy, Tabs, paint,
};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

fn clip_rect(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
    ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, clip).visible_clip()
}

fn safe_rect(rect: Rect) -> Rect {
    if !rect.origin.x.is_finite() || !rect.origin.y.is_finite() {
        return Rect::default();
    }
    let valid = |n: f32| if n.is_finite() { n } else { 0.0 };
    Rect::new(
        valid(rect.origin.x),
        valid(rect.origin.y),
        valid(rect.size.width).max(0.0),
        valid(rect.size.height).max(0.0),
    )
}

fn explicit_visible_clip(visible: Option<Rect>) -> Option<Rect> {
    Some(visible.unwrap_or_default())
}

fn focus_reveal_outset(backing_scale: f32) -> f32 {
    let ring_width = proposal::metric("stroke", "c-focus-width").expect("focus width token");
    let ring_gap = proposal::metric("spacing", "c-focus-offset").expect("focus offset token");
    let scale = if backing_scale.is_finite() && backing_scale > 0.0 {
        backing_scale
    } else {
        1.0
    };
    ring_width + ring_gap + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS / scale
}

fn set_button_state(
    button: &mut Button,
    focused: bool,
    focus_visible: bool,
    disabled: bool,
    selected: bool,
) {
    button.state.focused = focused;
    button.state.focus_visible = focused && focus_visible;
    button.state.disabled = disabled;
    button.state.selected = selected;
    button.state.loading = false;
}

fn size_role(compact: bool) -> ButtonSize {
    let _ = compact;
    // Compact density changes the outer row metric. Body is retained so
    // typography and horizontal inset stay equal to existing Gallery actions.
    ButtonSize::Body
}

fn row_height(compact: bool) -> f32 {
    if compact { 30.0 } else { 34.0 }
}

fn empty_button_layout() -> ButtonLayout {
    ButtonLayout {
        frame: Rect::default(),
        content: Rect::default(),
        visible: None,
        label_size: Size::default(),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabsLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    /// One entry per tab, in model order. Fully clipped tabs have `visible=None`.
    pub items: Vec<ButtonLayout>,
}

/// Button-backed, roving-tabstop presentation for the existing `Tabs` model.
#[derive(Clone, Debug)]
pub struct TabsView {
    model: Tabs,
    buttons: Vec<Button>,
    placement: TabsLayout,
    focus_visible: bool,
    focus_index: usize,
    focused: Option<usize>,
    window_active: bool,
    disabled: bool,
    compact: bool,
    press_target: Option<usize>,
}

impl TabsView {
    pub fn new(count: usize) -> Self {
        Self {
            model: Tabs::new(count),
            buttons: vec![Button::default(); count],
            placement: TabsLayout::default(),
            focus_visible: false,
            focus_index: 0,
            focused: None,
            window_active: true,
            disabled: false,
            compact: false,
            press_target: None,
        }
    }

    pub fn selected(&self) -> usize {
        self.model.selected
    }

    pub fn select(&mut self, index: usize) -> bool {
        let changed = self.model.select(index);
        if changed {
            self.cancel_arms();
        }
        changed
    }

    /// Moves the selected/roving stop as a host restores keyboard focus.
    pub fn focus(&mut self, index: usize, focus_visible: bool) -> bool {
        if index >= self.model.count {
            return false;
        }
        self.model.selected = index;
        self.focus_index = index;
        self.focused = Some(index);
        self.focus_visible = focus_visible;
        self.sync_focus_buttons();
        true
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.focus_visible = false;
            self.focused = None;
            self.cancel_arms();
            for button in &mut self.buttons {
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
        self.compact = compact;
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
            self.placement = TabsLayout::default();
            return self.placement.clone();
        }
        let frame = safe_rect(frame);
        self.buttons.resize_with(self.model.count, Button::default);
        let lane = if self.model.count == 0 {
            0.0
        } else {
            (frame.size.width / self.model.count as f32).min(160.0)
        };
        let lane_gap = focus_reveal_outset(text.backing_scale()).max(4.0);
        let lane_width = (lane - lane_gap).max(0.0);
        let base_height = row_height(compact);
        let mut measured_heights = Vec::with_capacity(self.model.count);
        for index in 0..self.model.count {
            let label = labels.get(index).copied().unwrap_or("");
            let measured =
                Button::measure(text, label, lane_width, size_role(compact), role).height;
            measured_heights.push(base_height.max(measured));
        }
        let height = measured_heights.iter().copied().fold(base_height, f32::max);
        let component_frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, height);
        let visible = clip_rect(component_frame, clip);
        let mut items = Vec::with_capacity(self.model.count);
        for (index, measured_height) in measured_heights.iter().copied().enumerate() {
            let label = labels.get(index).copied().unwrap_or("");
            let item = Rect::new(
                frame.min_x() + index as f32 * lane,
                frame.min_y(),
                lane_width,
                measured_height,
            );
            // The Tabs frame is an internal layout boundary, not a clipping
            // viewport. Keep focus-ring AA inside the true ancestor clip.
            let item_clip = clip;
            let button = &mut self.buttons[index];
            set_button_state(
                button,
                self.window_active && self.focused == Some(index),
                self.window_active && self.focus_visible && self.focused == Some(index),
                self.disabled,
                self.model.selected == index,
            );
            let layout = button.render(
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
            );
            items.push(layout);
        }
        self.placement = TabsLayout {
            frame: component_frame,
            visible,
            items,
        };
        if !visible.is_some_and(|rect| rect.size.width > 0.0 && rect.size.height > 0.0)
            || self.focused.is_some_and(|index| {
                self.placement
                    .items
                    .get(index)
                    .is_none_or(|item| item.visible.is_none())
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
    }

    pub fn pointer_down_at(&mut self, point: Point) -> Option<usize> {
        if !self.window_active || self.disabled {
            return None;
        }
        let target = self
            .placement
            .items
            .iter()
            .position(|item| item.hit_test(point))?;
        if target >= self.model.count {
            return None;
        }
        self.focus_visible = false;
        self.focus_index = target;
        self.focused = Some(target);
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
        let focused = self.focused?;
        if !down {
            let selected = focused;
            return self
                .buttons
                .get_mut(selected)
                .and_then(|button| button.key(key, false).then_some(selected));
        }
        if matches!(
            key,
            ControlKey::Left | ControlKey::Right | ControlKey::Home | ControlKey::End
        ) {
            let previous = self.model.selected;
            self.focus_visible = true;
            if self.model.key(key) {
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
        self.buttons
            .get_mut(selected)
            .and_then(|button| button.key(key, true).then_some(selected))
    }

    pub fn cancel(&mut self) {
        self.cancel_arms();
        self.focus_visible = false;
        self.focus_index = self.model.selected;
        self.sync_focus_buttons();
    }

    pub fn clear_focus(&mut self) {
        self.focused = None;
        self.focus_visible = false;
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
    }

    fn cancel_arms(&mut self) {
        self.press_target = None;
        for button in &mut self.buttons {
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuLayout {
    pub anchor: Rect,
    pub popup: Rect,
    pub visible: Option<Rect>,
    /// One entry per menu action, in model order.
    pub items: Vec<ButtonLayout>,
    pub active_item: usize,
    pub scroll_offset: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuEvent {
    #[default]
    None,
    Activated {
        index: usize,
        restore_focus: Option<usize>,
    },
    Dismissed {
        restore_focus: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuWheelInput {
    pub point: Point,
    pub delta: Point,
    pub precise: bool,
    pub begin: bool,
    pub end: bool,
    pub momentum: bool,
    pub time: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuWheelEvent {
    #[default]
    Ignored,
    Consumed {
        scrolled: bool,
    },
}

/// Runtime for the existing `Menu` model. It may own the model or bind to an
/// external one (for example `Select::menu`) using the `_with_model` methods.
#[derive(Debug)]
pub struct MenuView {
    model: Menu,
    bound_external: bool,
    runtime: MenuRuntime,
    // Optional adapter-owned declaration, within the already private original
    // runtime storage. MenuContent's legacy public struct shape stays intact.
    pub(crate) content_state: Option<crate::composition::MenuContentState>,
}

#[derive(Debug)]
struct MenuRuntime {
    presence: OverlayPresence,
    buttons: Vec<Button>,
    scroll: ScrollArea,
    placement: MenuLayout,
    return_focus: Option<usize>,
    window_active: bool,
    disabled: bool,
    focus_visible: bool,
    compact: bool,
    row_heights: Vec<f32>,
    scroll_viewport: Rect,
    visible_bounds: Rect,
    ancestor_clip: Option<Rect>,
    palette: Palette,
    role: proposal::TextRole,
    scroll_gesture_captured: bool,
    focus_outset: f32,
    row_styles: Vec<MenuRowStyle>,
    submenu_request: Option<usize>,
    beside_anchor: bool,
    open_generation: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MenuRowStyle {
    pub disabled: bool,
    pub checked: Option<bool>,
    pub separator: bool,
    pub submenu: bool,
}

// Original Button still owns chrome, ink, motion, hit layout and clipping.
// This label adapter reserves shared icon columns without another painter.
struct MenuRowText<'a, T> {
    text: &'a mut T,
    leading: f32,
    trailing: f32,
    gap: f32,
    style: MenuRowStyle,
}
impl<T: ButtonTextRenderer> ButtonTextRenderer for MenuRowText<'_, T> {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.text.resolved_font_size(role)
    }
    fn measure(&mut self, label: &str, role: proposal::TextRole, width: f32) -> Size {
        if self.leading == 0.0 && self.trailing == 0.0 {
            return self.text.measure(label, role, width);
        }
        let gutters = self.leading + self.trailing;
        let measured = self.text.measure(label, role, (width - gutters).max(0.0));
        Size::new(
            (measured.width + gutters).min(width.max(0.0)),
            measured.height,
        )
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        label: &str,
        role: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        let leading = self.leading.min(frame.size.width);
        let trailing = self.trailing.min((frame.size.width - leading).max(0.0));
        self.text.draw(
            scene,
            label,
            role,
            Rect::new(
                frame.min_x() + leading,
                frame.min_y(),
                (frame.size.width - leading - trailing).max(0.0),
                frame.size.height,
            ),
            align,
            ink,
        );
        let mark = (leading - self.gap).max(0.0);
        if self.style.checked == Some(true) && mark > 0.0 {
            self.text.draw(
                scene,
                "✓",
                role,
                Rect::new(frame.min_x(), frame.min_y(), mark, frame.size.height),
                ButtonTextAlign::Center,
                ink,
            );
        }
        let arrow = (trailing - self.gap).max(0.0);
        if self.style.submenu && arrow > 0.0 {
            self.text.draw(
                scene,
                "›",
                role,
                Rect::new(
                    frame.max_x() - arrow,
                    frame.min_y(),
                    arrow,
                    frame.size.height,
                ),
                ButtonTextAlign::Center,
                ink,
            );
        }
    }
    fn motion_context(&self) -> Option<crate::ControlMotionContext> {
        self.text.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.text.request_animation_frame();
    }
    fn backing_scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.text.text_surface_host()
    }
    fn first_line_height(&mut self, label: &str, role: proposal::TextRole, width: f32) -> f32 {
        self.text
            .first_line_height(label, role, (width - self.leading - self.trailing).max(0.0))
    }
}

// Retain the existing legacy surface minimum in one production rule. Item
// metadata changes row content; it must not silently change this minimum.
fn menu_surface_height(content: f32, inset: f32) -> f32 {
    (content + inset * 2.0).max(132.0)
}

impl MenuView {
    /// Legacy flat-label popup size before viewport placement/scroll clamping.
    /// Controlled declaration metadata is measured by this same runtime.
    /// Shares production row measurement, focus gutter and minimum surface
    /// height, so callers can reserve space without painting a probe.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        max_width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> Size {
        Self::measured_rows(text, labels, max_width, role, compact).0
    }
    fn measured_rows(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        max_width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> (Size, Vec<f32>) {
        let width = if max_width.is_nan() {
            0.0
        } else {
            max_width.clamp(0.0, 280.0)
        };
        let inset = focus_reveal_outset(text.backing_scale());
        let inner_width = (width - inset * 2.0).max(0.0);
        let heights: Vec<f32> = labels
            .iter()
            .map(|label| {
                Button::measure(text, label, inner_width, size_role(compact), role)
                    .height
                    .max(36.0)
            })
            .collect();
        let content =
            heights.iter().sum::<f32>() + labels.len().saturating_sub(1) as f32 * inset.max(4.0);
        (
            Size::new(width, menu_surface_height(content, inset)),
            heights,
        )
    }
    pub fn new(count: usize) -> Self {
        let mut scroll = ScrollArea::default();
        scroll.set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
        scroll.set_scrollbar_visibility(false, false);
        Self {
            model: Menu::new(count),
            bound_external: false,
            content_state: None,
            runtime: MenuRuntime {
                presence: OverlayPresence::default(),
                buttons: vec![Button::default(); count],
                scroll,
                placement: MenuLayout::default(),
                return_focus: None,
                window_active: true,
                disabled: false,
                focus_visible: false,
                compact: false,
                row_heights: vec![36.0; count],
                scroll_viewport: Rect::default(),
                visible_bounds: Rect::default(),
                ancestor_clip: None,
                palette: Palette::from(tekes_ui_tokens::Theme::default()),
                role: proposal::TextRole::UiBody,
                scroll_gesture_captured: false,
                focus_outset: focus_reveal_outset(1.0),
                row_styles: Vec::new(),
                submenu_request: None,
                beside_anchor: false,
                open_generation: 0,
            },
        }
    }

    /// Creates a presentation runtime that must always receive an external
    /// `Menu` through the `_with_model` methods. Its zero-item internal model
    /// is inert and is never synchronized with or used as an alternate source.
    pub fn new_external() -> Self {
        let mut view = Self::new(0);
        view.bound_external = true;
        view
    }

    pub fn for_model() -> Self {
        Self::new_external()
    }

    /// Paint can outlive semantic open during dismissal.
    pub fn is_presented(&self) -> bool {
        self.runtime.presence.is_presented()
    }

    /// Call on unmount or text/image resource replacement.
    pub fn clear_presentation(&mut self) {
        self.runtime.presence.clear();
    }

    /// Move semantic focus, revealing a mounted logical item before redraw.
    pub fn focus_item(&mut self, index: usize) -> bool {
        !self.bound_external && self.runtime.focus_item_model(&mut self.model, index)
    }
    /// The same semantic focus contract for a caller-owned menu model.
    pub fn focus_item_with_model(&mut self, model: &mut Menu, index: usize) -> bool {
        self.bound_external = true;
        self.runtime.focus_item_model(model, index)
    }
    /// Semantic activation shares the production state and dismissal path.
    pub fn activate_item(&mut self, index: usize) -> MenuEvent {
        if self.bound_external {
            return MenuEvent::None;
        }
        self.runtime.activate_item_model(&mut self.model, index)
    }
    /// The same semantic activation contract for a caller-owned menu model.
    pub fn activate_item_with_model(&mut self, model: &mut Menu, index: usize) -> MenuEvent {
        self.bound_external = true;
        self.runtime.activate_item_model(model, index)
    }
    pub fn menu(&self) -> &Menu {
        &self.model
    }

    /// Replace the owned flat model, retaining runtime window/motion settings.
    /// Old popup geometry and held input are revoked.
    pub(crate) fn replace_items(&mut self, count: usize) {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.deactivate_model(&mut self.model);
        self.runtime.presence.clear();
        self.runtime.clear_layout();
        self.model = Menu::new(count);
        self.runtime.row_styles.clear();
    }

    pub(crate) fn set_row_styles(&mut self, styles: Vec<MenuRowStyle>) {
        if self.runtime.row_styles != styles {
            self.runtime.cancel_buttons();
            self.runtime.row_styles = styles;
            if !self.runtime.item_enabled(self.model.items.selected) {
                self.model.items.selected = self
                    .runtime
                    .first_enabled(self.model.items.count)
                    .unwrap_or(0);
            }
        }
    }
    pub(crate) fn take_submenu_request(&mut self) -> Option<usize> {
        self.runtime.submenu_request.take()
    }
    pub(crate) fn scroll_gesture_captured(&self) -> bool {
        self.runtime.scroll_gesture_captured
    }
    pub(crate) fn cancel_scroll_gesture(&mut self) {
        self.runtime.scroll_gesture_captured = false;
        self.runtime.scroll.cancel();
    }
    pub(crate) fn set_submenu_placement(&mut self) {
        self.runtime.beside_anchor = true;
    }
    pub(crate) fn open_generation(&self) -> u64 {
        self.runtime.open_generation
    }
    pub(crate) fn set_keyboard_focus(&mut self, visible: bool) {
        self.runtime.focus_visible = visible;
        self.runtime.sync_focus(self.model.items.selected);
    }
    pub(crate) fn item_style(&self, index: usize) -> Option<MenuRowStyle> {
        self.runtime.row_styles.get(index).copied()
    }

    pub fn is_open(&self) -> bool {
        self.model.open
    }

    pub fn is_open_with_model(&self, model: &Menu) -> bool {
        model.open
    }

    pub fn show(&mut self, invoker: usize) {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.open_generation = self
            .runtime
            .open_generation
            .checked_add(1)
            .expect("menu popup generation exhausted");
        self.model.show();
        self.model.items.selected = self
            .runtime
            .first_enabled(self.model.items.count)
            .unwrap_or(0);
        self.runtime.return_focus = Some(invoker);
        self.runtime.focus_visible = false;
        self.runtime
            .scroll
            .scroll_by(0.0, -self.runtime.scroll.offset.y);
        self.runtime.clear_layout();
        self.runtime.sync_focus(self.model.items.selected);
    }

    pub fn show_with_model(&mut self, model: &mut Menu, invoker: usize) {
        self.bound_external = true;
        self.runtime.open_generation = self
            .runtime
            .open_generation
            .checked_add(1)
            .expect("menu popup generation exhausted");
        model.show();
        self.runtime.return_focus = Some(invoker);
        self.runtime.focus_visible = false;
        self.runtime
            .scroll
            .scroll_by(0.0, -self.runtime.scroll.offset.y);
        self.runtime.clear_layout();
        self.runtime.sync_focus(model.items.selected);
    }

    pub fn set_window_active(&mut self, active: bool) -> MenuEvent {
        self.runtime.window_active = active;
        if !active {
            self.runtime.presence.clear();
            self.runtime.deactivate_model(&mut self.model)
        } else {
            MenuEvent::None
        }
    }

    pub fn set_window_active_with_model(&mut self, model: &mut Menu, active: bool) -> MenuEvent {
        self.bound_external = true;
        self.runtime.window_active = active;
        if !active {
            self.runtime.presence.clear();
            self.runtime.deactivate_model(model)
        } else {
            MenuEvent::None
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.runtime.disabled = disabled;
        if disabled {
            self.runtime.cancel_buttons();
        }
    }

    pub fn set_disabled_with_model(&mut self, _model: &mut Menu, disabled: bool) {
        self.bound_external = true;
        self.runtime.disabled = disabled;
        if disabled {
            self.runtime.cancel_buttons();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        anchor: Rect,
        visible_bounds: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.render_model(
            &mut self.model,
            scene,
            text,
            labels,
            anchor,
            visible_bounds,
            ancestor_clip,
            palette,
            role,
            compact,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_with_model(
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
    ) -> Option<MenuLayout> {
        self.bound_external = true;
        self.runtime.render_model(
            model,
            scene,
            text,
            labels,
            anchor,
            visible_bounds,
            ancestor_clip,
            palette,
            role,
            compact,
        )
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.pointer_move_model(&mut self.model, point);
    }

    pub fn pointer_move_with_model(&mut self, model: &mut Menu, point: Point) {
        self.bound_external = true;
        self.runtime.pointer_move_model(model, point);
    }

    pub fn pointer_down_at(&mut self, point: Point) -> MenuEvent {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.pointer_down_model(&mut self.model, point)
    }

    pub fn pointer_down_with_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
        self.bound_external = true;
        self.runtime.pointer_down_model(model, point)
    }

    pub fn pointer_up_at(&mut self, point: Point) -> MenuEvent {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.pointer_up_model(&mut self.model, point)
    }

    pub fn pointer_up_with_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
        self.bound_external = true;
        self.runtime.pointer_up_model(model, point)
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> MenuEvent {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.key_model(&mut self.model, key, down)
    }

    pub fn key_with_model(&mut self, model: &mut Menu, key: ControlKey, down: bool) -> MenuEvent {
        self.bound_external = true;
        self.runtime.key_model(model, key, down)
    }

    pub fn wheel_event(&mut self, input: MenuWheelInput) -> MenuWheelEvent {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.wheel_event(&self.model, input)
    }

    pub fn wheel_event_with_model(
        &mut self,
        model: &Menu,
        input: MenuWheelInput,
    ) -> MenuWheelEvent {
        self.bound_external = true;
        self.runtime.wheel_event(model, input)
    }

    /// Applies the global reduced-motion preference to this menu's scroll
    /// physics. Enabling it immediately removes elastic displacement and ends
    /// any pending wheel animation without changing the normal scroll offset.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        let changed = self.runtime.scroll.set_reduced_motion(reduced);
        if reduced {
            let was_animating = self.runtime.scroll.needs_animation();
            self.runtime.scroll.cancel();
            changed || was_animating
        } else {
            changed
        }
    }

    /// Whether the host should keep scheduling frames for this menu's scroll
    /// gesture timeout or elastic rebound.
    pub fn needs_animation(&self) -> bool {
        self.runtime.scroll.needs_animation() || self.runtime.presence.needs_animation()
    }

    /// Advances the owned scroll clock by elapsed seconds and returns whether
    /// the visible scroll transform changed.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.runtime.scroll.tick(dt)
    }

    pub fn deactivate(&mut self) -> MenuEvent {
        assert!(
            !self.bound_external,
            "MenuView is bound to an external Menu model"
        );
        self.runtime.deactivate_model(&mut self.model)
    }

    pub fn deactivate_with_model(&mut self, model: &mut Menu) -> MenuEvent {
        self.bound_external = true;
        self.runtime.deactivate_model(model)
    }
}

impl MenuRuntime {
    fn row_style(&self, index: usize) -> MenuRowStyle {
        self.row_styles.get(index).copied().unwrap_or_default()
    }
    fn item_enabled(&self, index: usize) -> bool {
        let row = self.row_style(index);
        !row.disabled && !row.separator
    }
    fn first_enabled(&self, count: usize) -> Option<usize> {
        (0..count).find(|&index| self.item_enabled(index))
    }
    fn move_enabled(&self, count: usize, before: usize, key: ControlKey) -> Option<usize> {
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
    fn semantic_item_mounted(&self, model: &Menu, index: usize) -> bool {
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
    fn focus_item_model(&mut self, model: &mut Menu, index: usize) -> bool {
        if !self.semantic_item_mounted(model, index) {
            return false;
        }
        model.items.selected = index;
        self.focus_visible = true;
        self.reveal_active(model);
        self.sync_focus(index);
        true
    }
    fn activate_item_model(&mut self, model: &mut Menu, index: usize) -> MenuEvent {
        if !self.semantic_item_mounted(model, index) {
            return MenuEvent::None;
        }
        self.activate_model(model, index)
    }
    #[allow(clippy::too_many_arguments)]
    fn render_model(
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
        let paint_context = format!(
            "{anchor:?}{visible_bounds:?}{ancestor_clip:?}{palette:?}{role:?}{compact:?}{}{labels:?}{:?}",
            text.backing_scale(),
            self.row_styles
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
        self.focus_outset = focus_reveal_outset(text.backing_scale());
        let anchor = safe_rect(anchor);
        let visible_bounds = safe_rect(visible_bounds);
        let old_scroll_viewport = self.scroll_viewport;
        let old_row_heights = self.row_heights.clone();
        self.visible_bounds = visible_bounds;
        self.ancestor_clip = ancestor_clip;
        let count = model.items.count.min(labels.len());
        self.buttons.resize_with(model.items.count, Button::default);
        let has_check = self.row_styles.iter().any(|style| style.checked.is_some());
        let has_submenu = self.row_styles.iter().any(|style| style.submenu);
        let indicator_gap = if has_check || has_submenu {
            proposal::metric("spacing", "space-tight")
                .expect("canonical menu indicator-caption spacing")
        } else {
            0.0
        };
        let icon = if has_check || has_submenu {
            proposal::metric("size", "size-icon-standard").expect("canonical menu indicator size")
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
        let (mut natural_size, mut heights) = MenuView::measured_rows(
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
        );
        for (index, height) in heights.iter_mut().enumerate() {
            if self.row_style(index).separator {
                *height = proposal::metric("spacing", "space-regular")
                    .expect("canonical separator row spacing");
            }
        }
        if !self.row_styles.is_empty() {
            natural_size.height = menu_surface_height(
                heights.iter().sum::<f32>()
                    + count.saturating_sub(1) as f32 * self.focus_outset.max(4.0),
                self.focus_outset,
            );
        }
        heights.resize(model.items.count, 36.0);
        self.row_heights = heights;
        let width = natural_size.width;
        let popup_inset = self.focus_outset;
        let row_gap = popup_inset.max(4.0);
        let role_size = size_role(compact);
        let natural_content = self.row_heights.iter().take(count).sum::<f32>()
            + count.saturating_sub(1) as f32 * row_gap;
        let gap =
            proposal::metric("spacing", "space-micro").expect("canonical popup-to-anchor spacing");
        let below =
            (visible_bounds.max_y() - anchor.max_y() - gap).clamp(0.0, visible_bounds.size.height);
        let above =
            (anchor.min_y() - gap - visible_bounds.min_y()).clamp(0.0, visible_bounds.size.height);
        let available = below.max(above);
        let minimum_row = self.row_heights.first().copied().unwrap_or(0.0) + popup_inset * 2.0;
        // Scroll inside an available side rather than covering a measured
        // anchor. If neither side can show one full row, retain the existing
        // viewport-bounded overlay fallback instead of an unusable thin strip.
        let height_limit = if self.beside_anchor {
            visible_bounds.size.height
        } else if available >= minimum_row {
            available
        } else {
            visible_bounds.size.height
        };
        let size = Size::new(width, natural_size.height.min(height_limit));
        let popup = if self.beside_anchor {
            // The same original placement rule, with axes transposed: right,
            // then left, clamped to the real viewport. No host screen math.
            let transpose = |r: Rect| Rect::new(r.min_y(), r.min_x(), r.size.height, r.size.width);
            transpose(crate::feedback_view::place_overlay(
                transpose(anchor),
                Size::new(size.height, size.width),
                transpose(visible_bounds),
                gap,
            ))
        } else {
            crate::feedback_view::place_overlay(anchor, size, visible_bounds, gap)
        };
        let popup_container =
            ContainerLayout::new(popup, Insets::ZERO, Insets::ZERO, ancestor_clip);
        let visible = popup_container.visible_clip();
        let inner_viewport = Rect::new(
            popup.min_x() + popup_inset,
            popup.min_y() + popup_inset,
            (popup.size.width - popup_inset * 2.0).max(0.0),
            (popup.size.height - popup_inset * 2.0).max(0.0),
        );
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
        let mut items = Vec::with_capacity(model.items.count);
        let mut y = inner_viewport.min_y() - offset;
        for index in 0..model.items.count {
            let height = self.row_heights[index];
            let item = Rect::new(inner_viewport.min_x(), y, inner_viewport.size.width, height);
            let row_clip = explicit_visible_clip(visible);
            let style = self.row_style(index);
            if style.separator {
                if let Some(clip) = visible {
                    scene.push_clip(clip);
                }
                crate::extended::separator_at_scale(
                    scene,
                    item,
                    palette,
                    crate::extended::Axis::Horizontal,
                    text.backing_scale(),
                );
                if visible.is_some() {
                    scene.pop_clip();
                }
                items.push(ButtonLayout {
                    frame: item,
                    content: item,
                    visible: clip_rect(item, visible),
                    label_size: Size::default(),
                });
                y += height + row_gap;
                continue;
            }
            let button = &mut self.buttons[index];
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
            items.push(layout);
            y += height + row_gap;
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

    fn pointer_move_model(&mut self, model: &mut Menu, point: Point) {
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

    fn pointer_down_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
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

    fn pointer_up_model(&mut self, model: &mut Menu, point: Point) -> MenuEvent {
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

    fn key_model(&mut self, model: &mut Menu, key: ControlKey, down: bool) -> MenuEvent {
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

    fn wheel_event(&mut self, model: &Menu, input: MenuWheelInput) -> MenuWheelEvent {
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

    fn deactivate_model(&mut self, model: &mut Menu) -> MenuEvent {
        self.cancel_buttons();
        self.clear_button_pointer_positions();
        if model.open {
            self.dismiss_model(model)
        } else {
            MenuEvent::None
        }
    }

    fn activate_model(&mut self, model: &mut Menu, index: usize) -> MenuEvent {
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

    fn dismiss_model(&mut self, model: &mut Menu) -> MenuEvent {
        if !model.open {
            return MenuEvent::None;
        }
        model.open = false;
        self.clear_layout();
        MenuEvent::Dismissed {
            restore_focus: self.return_focus.take(),
        }
    }

    fn reveal_active(&mut self, model: &Menu) {
        let index = model
            .items
            .selected
            .min(self.row_heights.len().saturating_sub(1));
        if self.row_heights.is_empty() {
            return;
        }
        let top = self
            .row_heights
            .iter()
            .take(index)
            .map(|h| h + self.focus_outset.max(4.0))
            .sum::<f32>()
            - self.focus_outset;
        let bottom = top + self.row_heights[index] + self.focus_outset * 2.0;
        let viewport_height = self.scroll_viewport.size.height.max(0.0);
        let offset = self.scroll.offset.y;
        if top < offset {
            self.scroll.scroll_by(0.0, top - offset);
        } else if bottom > offset + viewport_height {
            self.scroll
                .scroll_by(0.0, bottom - (offset + viewport_height));
        }
    }

    fn cancel_buttons(&mut self) {
        for button in &mut self.buttons {
            button.cancel();
        }
        self.scroll.cancel();
        self.scroll_gesture_captured = false;
    }

    fn sync_focus(&mut self, selected: usize) {
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

    fn clear_layout(&mut self) {
        self.submenu_request = None;
        self.cancel_buttons();
        self.placement = MenuLayout::default();
        self.focus_visible = false;
    }

    /// A deactivated menu no longer owns the pointer's previous location.
    fn clear_button_pointer_positions(&mut self) {
        for button in &mut self.buttons {
            button.blur();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialogLayout {
    pub preview: Rect,
    pub visible: Option<Rect>,
    pub modal: Rect,
    pub title: Rect,
    pub buttons: [ButtonLayout; 2],
    pub focus: usize,
}

impl Default for DialogLayout {
    fn default() -> Self {
        Self {
            preview: Rect::default(),
            visible: None,
            modal: Rect::default(),
            title: Rect::default(),
            buttons: [empty_button_layout(); 2],
            focus: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogEvent {
    #[default]
    None,
    Blocked,
    Cancelled {
        restore_focus: Option<usize>,
    },
    Confirmed {
        restore_focus: Option<usize>,
    },
}

/// Modal presentation for the existing `Dialog` model.
#[derive(Clone, Debug)]
pub struct DialogView {
    presence: OverlayPresence,
    model: Dialog,
    buttons: [Button; 2],
    placement: DialogLayout,
    return_focus: Option<usize>,
    window_active: bool,
    disabled: bool,
    focus_visible: bool,
    press_target: Option<usize>,
}

impl Default for DialogView {
    fn default() -> Self {
        Self {
            presence: OverlayPresence::default(),
            model: Dialog::default(),
            buttons: [Button::default(), Button::default()],
            placement: DialogLayout::default(),
            return_focus: None,
            window_active: true,
            disabled: false,
            focus_visible: false,
            press_target: None,
        }
    }
}

impl DialogView {
    /// Shared modal surface for default actions and public composed content.
    pub fn paint_content_frame(
        scene: &mut Scene,
        viewport: Rect,
        modal: Rect,
        clip: Rect,
        palette: Palette,
        scale: f32,
    ) {
        scene.push_clip(clip);
        scene.push_quad(Quad::new(
            viewport,
            palette.color("scrim").expect("scrim token"),
        ));
        paint::surface_kind_at_scale(scene, modal, palette, paint::SurfaceKind::Dialog, scale);
        scene.pop_clip();
    }

    /// Paint can outlive semantic open during dismissal.
    pub fn is_presented(&self) -> bool {
        self.presence.is_presented()
    }

    pub fn needs_animation(&self) -> bool {
        self.presence.needs_animation()
    }

    /// Call on unmount or text/image resource replacement.
    pub fn clear_presentation(&mut self) {
        self.presence.clear();
    }

    /// Labels are shared with measurement and painting, including native AX adapters.
    pub fn button_label(index: usize) -> Option<&'static str> {
        ["Cancel", "Confirm"].get(index).copied()
    }

    /// Direct semantic focus of an existing visible action; no synthetic key.
    pub fn focus_button(&mut self, index: usize) -> bool {
        if index >= 2
            || !self.model.open
            || !self.window_active
            || self.disabled
            || self.buttons[index]
                .layout()
                .is_none_or(|l| l.visible.is_none())
        {
            return false;
        }
        let changed = self.model.focus != index || !self.focus_visible;
        self.model.focus = index;
        self.focus_visible = true;
        self.cancel_button_arms_except(index);
        self.sync_focus_buttons();
        changed
    }

    /// Activate the same retained Button and finish path used by pointer/keyboard.
    /// Closing/hidden/disabled actions cannot produce a second dismissal.
    pub fn activate_button(&mut self, index: usize) -> DialogEvent {
        if index >= 2
            || !self.model.open
            || !self.window_active
            || self.disabled
            || self.buttons[index]
                .layout()
                .is_none_or(|l| l.visible.is_none())
        {
            return DialogEvent::Blocked;
        }
        self.buttons[index].pointer_down();
        if !self.buttons[index].pointer_up(true) {
            return DialogEvent::Blocked;
        }
        self.finish(index == 1)
    }

    pub fn dialog(&self) -> &Dialog {
        &self.model
    }

    pub fn is_open(&self) -> bool {
        self.model.open
    }

    pub fn show(&mut self, invoker: usize) {
        self.model.show(invoker);
        self.return_focus = Some(invoker);
        self.window_active = true;
        self.disabled = false;
        self.focus_visible = false;
        self.cancel_buttons();
        for button in &mut self.buttons {
            button.clear_layout();
        }
        self.sync_focus_buttons();
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.focus_visible = false;
            self.cancel_buttons();
            self.presence.settle(self.model.open);
            for button in &mut self.buttons {
                button.blur();
            }
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.cancel_buttons();
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Compatible entry; a returned layout does not imply successful paint.
    /// Renderer rejects a Scene carrying any composition error.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        title: &str,
        preview: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<DialogLayout> {
        match self.try_render(
            scene,
            text,
            title,
            preview,
            ancestor_clip,
            palette,
            role,
            compact,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                scene.record_composition_error(error);
                None
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn try_render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        title: &str,
        preview: Rect,
        ancestor_clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Result<Option<DialogLayout>, tekes_ui_scene::FragmentError> {
        if let Some(error) = scene.composition_error() {
            return Err(error);
        }
        if safe_rect(preview) != preview
            || preview.size.width <= 0.0
            || preview.size.height <= 0.0
            || ancestor_clip.is_some_and(|clip| safe_rect(clip) != clip)
        {
            self.presence.clear();
            self.placement = DialogLayout::default();
            self.cancel_buttons();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        let paint_context = format!(
            "{title}{preview:?}{ancestor_clip:?}{palette:?}{role:?}{compact:?}{}",
            text.backing_scale()
        );
        if !self.model.open && !self.presence.is_presented() {
            self.placement = DialogLayout::default();
            self.presence
                .try_paint(scene, text, false, paint_context, None)?;
            return Ok(None);
        }
        let destination = scene;
        let mut overlay_scene = destination.fragment();
        let scene = &mut overlay_scene;
        let preview = safe_rect(preview);
        let visible = clip_rect(preview, ancestor_clip);
        if visible.is_none() {
            self.presence.clear();
            self.placement = DialogLayout::default();
            self.cancel_buttons();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        let modal_width = (preview.size.width - 16.0).clamp(0.0, 460.0);
        let modal_height = preview.size.height.clamp(0.0, 200.0);
        let modal = Rect::new(
            preview.min_x() + (preview.size.width - modal_width) * 0.5,
            preview.min_y() + (preview.size.height - modal_height) * 0.5,
            modal_width,
            modal_height,
        );
        let action_height: f32 = if compact { 30.0 } else { 36.0 };
        let margin = 20.0_f32.min((modal.size.width * 0.5).max(0.0));
        let action_width = (modal.size.width - margin * 2.0).max(0.0);
        let focus_gap = focus_reveal_outset(text.backing_scale()) * 2.0;
        let cancel_natural = Button::measure(
            text,
            Self::button_label(0).unwrap(),
            f32::MAX,
            ButtonSize::Body,
            role,
        );
        let confirm_natural = Button::measure(
            text,
            Self::button_label(1).unwrap(),
            f32::MAX,
            ButtonSize::Body,
            role,
        );
        let inline_width = ((action_width - focus_gap).max(0.0)) * 0.5;
        let mut stacked = inline_width < cancel_natural.width.max(confirm_natural.width);
        let vertical_margin = 20.0_f32.min((modal.size.height * 0.5).max(0.0));
        let stacked_natural_height = Button::measure(
            text,
            Self::button_label(0).unwrap(),
            action_width,
            ButtonSize::Body,
            role,
        )
        .height
        .max(
            Button::measure(
                text,
                Self::button_label(1).unwrap(),
                action_width,
                ButtonSize::Body,
                role,
            )
            .height,
        );
        if stacked
            && (modal.size.height - vertical_margin * 2.0).max(0.0)
                < stacked_natural_height * 2.0 + focus_gap
        {
            stacked = false;
        }
        let stack_gap = if stacked {
            focus_gap.min((modal.size.height / 4.0).max(0.0))
        } else {
            0.0
        };
        let button_width = if stacked { action_width } else { inline_width };
        let measured_button_height = Button::measure(
            text,
            Self::button_label(0).unwrap(),
            button_width,
            ButtonSize::Body,
            role,
        )
        .height
        .max(
            Button::measure(
                text,
                Self::button_label(1).unwrap(),
                button_width,
                ButtonSize::Body,
                role,
            )
            .height,
        );
        let available_action_height = (modal.size.height - vertical_margin * 2.0).max(0.0);
        let maximum_button_height = if stacked {
            (available_action_height - stack_gap).max(0.0) * 0.5
        } else {
            available_action_height
        };
        let action_h = action_height
            .max(measured_button_height)
            .min(maximum_button_height);
        let actions_total = if stacked {
            action_h * 2.0 + stack_gap
        } else {
            action_h
        };
        let actions_top =
            (modal.max_y() - vertical_margin - actions_total).max(modal.min_y() + vertical_margin);
        let button_frame = |index: usize| {
            if stacked {
                Rect::new(
                    modal.min_x() + margin,
                    actions_top + index as f32 * (action_h + stack_gap),
                    action_width,
                    action_h,
                )
            } else {
                Rect::new(
                    modal.min_x() + margin + index as f32 * (button_width + focus_gap),
                    actions_top,
                    button_width,
                    action_h,
                )
            }
        };
        let visible_modal = visible.and_then(|clip| modal.intersection(&clip));
        if let Some(visible) = visible {
            Self::paint_content_frame(
                scene,
                preview,
                modal,
                visible,
                palette,
                text.backing_scale(),
            );
        }
        let title_clip = Rect::new(
            modal.min_x() + margin,
            modal.min_y() + 20.0_f32.min(modal.size.height),
            (modal.size.width - margin * 2.0).max(0.0),
            (actions_top - modal.min_y() - 32.0).max(0.0),
        );
        let title_size = text.measure(title, proposal::TextRole::UiHeading, title_clip.size.width);
        let title_rect = Rect::new(
            title_clip.min_x(),
            title_clip.min_y(),
            title_clip.size.width,
            title_size.height,
        );
        if let Some(visible_modal) = visible_modal
            && let Some(clip) = title_clip.intersection(&visible_modal)
        {
            scene.push_clip(clip);
            text.draw(
                scene,
                title,
                proposal::TextRole::UiHeading,
                title_rect,
                ButtonTextAlign::Leading,
                palette.color("primary-text").expect("primary text token"),
            );
            scene.pop_clip();
        }
        let labels = [
            Self::button_label(0).unwrap(),
            Self::button_label(1).unwrap(),
        ];
        let mut button_layouts = [empty_button_layout(); 2];
        for index in 0..2 {
            let frame = button_frame(index);
            set_button_state(
                &mut self.buttons[index],
                self.window_active && self.model.open && self.model.focus == index,
                self.window_active
                    && self.model.open
                    && self.focus_visible
                    && self.model.focus == index,
                self.disabled,
                false,
            );
            button_layouts[index] = self.buttons[index].render(
                scene,
                text,
                labels[index],
                frame,
                // The action bounds are not a clip. Keep ring antialiasing
                // inside the actual modal and ancestor viewport instead.
                visible_modal,
                palette,
                paint::ButtonVariant::Quiet,
                ButtonSize::Body,
                role,
                ButtonTextAlign::default(),
            );
        }
        self.placement = DialogLayout {
            preview,
            visible,
            modal,
            title: title_rect,
            buttons: button_layouts,
            focus: self.model.focus,
        };
        self.presence.try_paint(
            destination,
            text,
            self.model.open,
            paint_context,
            Some(overlay_scene),
        )?;
        if !self.window_active {
            self.presence.settle(self.model.open);
        }
        if !self.model.open {
            self.placement = DialogLayout::default();
            for button in &mut self.buttons {
                button.clear_layout();
            }
            return Ok(None);
        }
        Ok(Some(self.placement.clone()))
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        if !self.window_active || self.disabled || !self.model.open {
            return;
        }
        for button in &mut self.buttons {
            button.pointer_move_at(point);
        }
    }

    pub fn pointer_down_at(&mut self, point: Point) -> DialogEvent {
        if !self.window_active || self.disabled || !self.model.open {
            return DialogEvent::None;
        }
        if !self
            .placement
            .visible
            .is_some_and(|visible| visible.contains(point))
        {
            return DialogEvent::None;
        }
        if !self.placement.modal.contains(point) {
            self.cancel_buttons();
            return DialogEvent::Blocked;
        }
        if let Some(index) = self
            .placement
            .buttons
            .iter()
            .position(|button| button.hit_test(point))
        {
            self.model.focus = index;
            self.focus_visible = false;
            self.sync_focus_buttons();
            if self.buttons[index].pointer_down_at(point) {
                self.press_target = Some(index);
            }
        } else {
            self.cancel_buttons();
        }
        DialogEvent::Blocked
    }

    pub fn pointer_up_at(&mut self, point: Point) -> DialogEvent {
        if !self.window_active || !self.model.open {
            return DialogEvent::None;
        }
        let target = self.press_target.take();
        let Some(target) = target else {
            return if self
                .placement
                .visible
                .is_some_and(|visible| visible.contains(point))
            {
                DialogEvent::Blocked
            } else {
                DialogEvent::None
            };
        };
        let activated = self.buttons[target].pointer_up_at(point);
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if index != target {
                button.cancel_pointer();
            }
        }
        if activated {
            self.finish(target == 1)
        } else if self
            .placement
            .visible
            .is_some_and(|visible| visible.contains(point))
        {
            DialogEvent::Blocked
        } else {
            DialogEvent::None
        }
    }

    /// A visible modal consumes scrolling anywhere inside its preview so the
    /// event cannot reach the page behind it.
    pub fn wheel_at(&self, point: Point) -> DialogEvent {
        if self.model.open
            && self.window_active
            && self
                .placement
                .visible
                .is_some_and(|visible| visible.contains(point))
        {
            DialogEvent::Blocked
        } else {
            DialogEvent::None
        }
    }

    /// Reports whether this open, active modal blocks background scrolling in
    /// the host that owns its modal scope. The host should call this only after
    /// routing an event belonging to that same window/document.
    pub fn blocks_background(&self) -> bool {
        self.model.open && self.window_active
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> DialogEvent {
        if !self.window_active || !self.model.open {
            return DialogEvent::None;
        }
        if down && key == ControlKey::Escape {
            return self.finish(false);
        }
        if self.disabled {
            return DialogEvent::None;
        }
        if !down {
            let index = self.model.focus.min(1);
            if self.buttons[index].key(key, false) {
                return self.finish(index == 1);
            }
            return DialogEvent::None;
        }
        match key {
            ControlKey::Tab { reverse } => {
                self.model.tab(reverse, 2);
                self.focus_visible = true;
                self.cancel_button_arms_except(self.model.focus);
                self.sync_focus_buttons();
                DialogEvent::Blocked
            }
            ControlKey::Enter | ControlKey::Space => {
                self.focus_visible = true;
                let index = self.model.focus.min(1);
                if self.buttons[index]
                    .layout()
                    .is_none_or(|layout| layout.visible.is_none())
                {
                    return DialogEvent::Blocked;
                }
                self.sync_focus_buttons();
                if self.buttons[index].key(key, true) {
                    self.finish(index == 1)
                } else {
                    DialogEvent::Blocked
                }
            }
            _ => DialogEvent::Blocked,
        }
    }

    pub fn deactivate(&mut self) {
        // A modal stays open across window deactivation, but cannot keep an
        // activation arm or react to input while the owning window is inactive.
        self.set_window_active(false);
    }

    /// Explicitly closes the modal, including during a page/window teardown.
    /// Unlike `deactivate`, this reports focus restoration and closes the model.
    pub fn close(&mut self) -> DialogEvent {
        self.cancel()
    }

    pub fn cancel(&mut self) -> DialogEvent {
        if self.model.open {
            self.finish(false)
        } else {
            DialogEvent::None
        }
    }

    fn finish(&mut self, confirmed: bool) -> DialogEvent {
        let model_restore_focus = self.model.close();
        let restore_focus = self.return_focus.take().or(model_restore_focus);
        self.model.open = false;
        self.cancel_buttons();
        for button in &mut self.buttons {
            button.clear_layout();
        }
        self.placement = DialogLayout::default();
        if confirmed {
            DialogEvent::Confirmed { restore_focus }
        } else {
            DialogEvent::Cancelled { restore_focus }
        }
    }

    fn cancel_buttons(&mut self) {
        self.press_target = None;
        for button in &mut self.buttons {
            button.cancel();
        }
    }

    fn sync_focus_buttons(&mut self) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            let focused = self.window_active && self.model.open && index == self.model.focus;
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

    fn cancel_button_arms_except(&mut self, except: usize) {
        for (index, button) in self.buttons.iter_mut().enumerate() {
            if index != except {
                button.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    struct FixedText;

    struct TallText;

    struct ScaledText(f32);

    struct WrappingText {
        wraps_below: f32,
    }

    impl ButtonTextRenderer for WrappingText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            let line_count = if max_width < self.wraps_below {
                2.0
            } else {
                1.0
            };
            Size::new(max_width.max(0.0), line_count * 20.0)
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    #[derive(Default)]
    struct RecordingText {
        draws: Vec<(String, Rect, ButtonTextAlign)>,
        scale: f32,
    }

    impl ButtonTextRenderer for RecordingText {
        fn backing_scale(&self) -> f32 {
            if self.scale > 0.0 { self.scale } else { 1.0 }
        }

        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
                18.0,
            )
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            text: &str,
            _role: proposal::TextRole,
            rect: Rect,
            align: ButtonTextAlign,
            _ink: Rgba,
        ) {
            self.draws.push((text.to_owned(), rect, align));
        }
    }

    impl ButtonTextRenderer for ScaledText {
        fn backing_scale(&self) -> f32 {
            self.0
        }

        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
                18.0,
            )
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    impl ButtonTextRenderer for TallText {
        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 7.0).min(max_width.max(0.0)),
                100.0,
            )
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    impl ButtonTextRenderer for FixedText {
        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            let width = (text.chars().count() as f32 * 7.0).min(max_width.max(0.0));
            Size::new(width, 18.0)
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }

    fn palette() -> Palette {
        Palette::from(Theme::default())
    }

    fn render_menu(view: &mut MenuView, labels: &[&str]) -> MenuLayout {
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut FixedText,
            labels,
            Rect::new(12.0, 12.0, 120.0, 34.0),
            Rect::new(0.0, 0.0, 320.0, 260.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        )
        .expect("open menu layout")
    }

    #[test]
    fn menu_surface_uses_ancestor_clip_and_rows_keep_popup_clip() {
        for ancestor in [
            Rect::new(-20., -20., 400., 300.),
            Rect::new(30., 20., 90., 100.),
        ] {
            let mut view = MenuView::new(2);
            view.show(0);
            let mut scene = Scene::new();
            let layout = view
                .render(
                    &mut scene,
                    &mut FixedText,
                    &["First", "Second"],
                    Rect::new(20., 20., 100., 32.),
                    Rect::new(0., 0., 320., 240.),
                    Some(ancestor),
                    palette(),
                    proposal::TextRole::UiBody,
                    false,
                )
                .expect("menu layout");
            let visible = layout.visible.expect("menu visible");
            let primitives = scene.primitives();
            assert!(
                matches!(primitives.first(), Some(tekes_ui_scene::Primitive::PushClip(r)) if *r == ancestor)
            );
            let surface_index = primitives
                .iter()
                .position(|primitive| {
                    matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == layout.popup)
                })
                .expect("popup surface remains present after shadow layers");
            let pop_index = primitives[surface_index + 1..]
                .iter()
                .position(|primitive| matches!(primitive, tekes_ui_scene::Primitive::PopClip))
                .map(|offset| surface_index + 1 + offset)
                .expect("popup surface clip is closed");
            assert!(primitives[pop_index + 1..].iter().any(
                |primitive| matches!(primitive, tekes_ui_scene::Primitive::PushClip(r) if *r == visible)
            ));
        }
    }

    #[test]
    fn keyboard_menu_reveals_focus_ring_inside_popup_padding() {
        let labels = [
            "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
        ];
        let mut view = MenuView::new(labels.len());
        view.show(1);
        let mut scene = Scene::new();
        let _ = render_menu(&mut view, &labels);
        assert_eq!(view.key(ControlKey::End, true), MenuEvent::None);
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12.0, 12.0, 120.0, 34.0),
                Rect::new(0.0, 0.0, 120.0, 260.0),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open keyboard menu layout");
        let focused = layout.items.last().expect("last row");
        let ring_width = proposal::metric("stroke", "c-focus-width").unwrap();
        let ring_gap = proposal::metric("spacing", "c-focus-offset").unwrap();
        let ring_outset = ring_width + ring_gap;
        let minimum_row_gap = focus_reveal_outset(1.0).max(4.0);
        assert!(layout.items.windows(2).all(|rows| {
            rows[1].frame.min_y() - rows[0].frame.max_y() + f32::EPSILON >= minimum_row_gap
        }));
        assert!(layout.scroll_offset > 0.0);
        assert!(focused.focus_bounds().min_y() >= layout.popup.min_y() + 1.0);
        assert!(focused.focus_bounds().max_y() <= layout.popup.max_y() - 1.0);
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(q)
                if (q.rect.min_y() - (focused.frame.min_y() - ring_outset)).abs() < 0.01
                    && q.rect.size.width > focused.frame.size.width)
            }),
            "keyboard-selected menu row paints its focus ring"
        );
    }

    #[test]
    fn menu_row_measurement_uses_actual_focus_safe_inner_width() {
        let inset = proposal::metric("spacing", "space-relaxed").unwrap();
        let current_label_width = 120.0 - focus_reveal_outset(1.0) * 2.0 - inset * 2.0;
        let mut view = MenuView::new(1);
        view.show(1);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut WrappingText {
                    wraps_below: current_label_width + 0.5,
                },
                &["abcdefghijklmnopq"],
                Rect::new(12.0, 12.0, 120.0, 34.0),
                Rect::new(0.0, 0.0, 120.0, 260.0),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open menu layout");
        assert_eq!(layout.items[0].frame.size.height, 48.0);
        assert_eq!(layout.items[0].content.size.width, current_label_width);
    }

    #[test]
    fn tabs_focus_ring_uses_ancestor_clip_not_tabs_frame() {
        let mut tabs = TabsView::new(1);
        assert!(tabs.focus(0, true));
        let ancestor = Rect::new(-10.0, 0.0, 140.0, 80.0);
        let mut scene = Scene::new();
        let layout = tabs.render(
            &mut scene,
            &mut FixedText,
            &["Tab"],
            Rect::new(10.0, 20.0, 100.0, 30.0),
            Some(ancestor),
            palette(),
            proposal::TextRole::UiBody,
            false,
        );
        let item = layout.items.first().unwrap();
        let outset = proposal::metric("stroke", "c-focus-width").unwrap()
            + proposal::metric("spacing", "c-focus-offset").unwrap();
        let expected = item.frame.inset(-outset);
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == expected)
            }),
            "ring extends beyond the tab's own frame"
        );
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::PushClip(clip) if *clip == ancestor)
            }),
            "only the real ancestor viewport clips the ring"
        );
    }

    #[test]
    fn dialog_actions_use_design_system_centered_content_alignment() {
        let mut dialog = DialogView::default();
        dialog.show(11);
        assert_eq!(
            dialog.key(ControlKey::Tab { reverse: false }, true),
            DialogEvent::Blocked
        );
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        let layout = dialog
            .render(
                &mut scene,
                &mut text,
                "Delete this item?",
                Rect::new(0.0, 0.0, 600.0, 420.0),
                Some(Rect::new(0.0, 0.0, 600.0, 420.0)),
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open dialog layout");
        assert_eq!(layout.focus, 1);
        let focus_outset = focus_reveal_outset(text.backing_scale());
        let action_gap = layout.buttons[1].frame.min_x() - layout.buttons[0].frame.max_x();
        assert!(
            action_gap + f32::EPSILON >= 2.0 * focus_outset,
            "dialog action focus rings need non-overlapping clearance"
        );
        for action in ["Cancel", "Confirm"] {
            assert_eq!(
                text.draws
                    .iter()
                    .find(|(label, _, _)| label == action)
                    .unwrap()
                    .2,
                ButtonTextAlign::default(),
                "dialog action {action} follows ButtonContent token alignment"
            );
        }
        let focused = layout.buttons[1];
        let outset = proposal::metric("stroke", "c-focus-width").unwrap()
            + proposal::metric("spacing", "c-focus-offset").unwrap();
        let ring = focused.frame.inset(-outset);
        assert!(
            scene.primitives().iter().any(|primitive| {
                matches!(primitive, tekes_ui_scene::Primitive::Quad(q) if q.rect == ring)
            }),
            "dialog button focus ring is drawn outside the action frame"
        );
    }

    #[test]
    fn tabs_focus_arrow_and_space_use_the_new_focus_without_a_render() {
        let mut view = TabsView::new(3);
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut FixedText,
            &["One", "Two", "Three"],
            Rect::new(0.0, 0.0, 300.0, 34.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        );
        let lane_gap = layout.items[1].frame.min_x() - layout.items[0].frame.max_x();
        assert!(lane_gap + f32::EPSILON >= focus_reveal_outset(1.0).max(4.0));
        assert!(view.focus(0, true));
        assert_eq!(view.key(ControlKey::Right, true), Some(1));
        assert_eq!(view.selected(), 1);
        assert_eq!(view.key(ControlKey::Space, true), None);
        assert_eq!(view.key(ControlKey::Space, false), Some(1));
        assert!(layout.items[1].visible.is_some());
    }

    #[test]
    fn tabs_cancel_space_when_focus_moves_before_key_up() {
        let mut view = TabsView::new(2);
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut FixedText,
            &["One", "Two"],
            Rect::new(0.0, 0.0, 200.0, 34.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(0, true);
        view.key(ControlKey::Space, true);
        view.key(ControlKey::Right, true);
        assert_eq!(view.key(ControlKey::Space, false), None);
        assert_eq!(view.selected(), 1);

        view.key(ControlKey::Space, true);
        view.clear_focus();
        assert_eq!(view.key(ControlKey::Space, false), None);
        assert_eq!(view.selected(), 1);
    }

    #[test]
    fn menu_keyboard_focus_syncs_before_enter_and_escape_works_before_render() {
        let mut view = MenuView::new(3);
        view.show(7);
        let layout = render_menu(&mut view, &["One", "Two", "Three"]);
        assert_eq!(view.key(ControlKey::Down, true), MenuEvent::None);
        assert_eq!(
            view.key(ControlKey::Enter, true),
            MenuEvent::Activated {
                index: 1,
                restore_focus: Some(7),
            }
        );
        assert!(layout.items[1].visible.is_some());

        let mut view = MenuView::new(2);
        view.show(4);
        assert_eq!(
            view.key(ControlKey::Escape, true),
            MenuEvent::Dismissed {
                restore_focus: Some(4),
            }
        );
        assert!(!view.is_open());

        let mut view = MenuView::new(2);
        view.show(6);
        render_menu(&mut view, &["One", "Two"]);
        assert_eq!(
            view.key(ControlKey::Space, true),
            MenuEvent::Activated {
                index: 0,
                restore_focus: Some(6),
            }
        );
        assert_eq!(view.key(ControlKey::Space, false), MenuEvent::None);
        assert_eq!(view.menu().last_action, Some(0));
    }

    #[test]
    fn menu_keyboard_active_row_uses_button_focus_ring() {
        let mut view = MenuView::new(3);
        view.show(0);
        render_menu(&mut view, &["One", "Two", "Three"]);
        assert_eq!(view.key(ControlKey::Down, true), MenuEvent::None);
        render_menu(&mut view, &["One", "Two", "Three"]);

        let active = &view.runtime.buttons[1].state;
        assert!(active.selected);
        assert!(active.focused);
        assert!(active.focus_visible);
        assert!(!view.runtime.buttons[0].state.focus_visible);
    }

    #[test]
    fn disabled_menu_still_closes_with_escape_or_outside_pointer() {
        let mut view = MenuView::new(2);
        view.show(3);
        render_menu(&mut view, &["One", "Two"]);
        view.set_disabled(true);
        assert_eq!(view.key(ControlKey::Enter, true), MenuEvent::None);
        assert_eq!(
            view.key(ControlKey::Escape, true),
            MenuEvent::Dismissed {
                restore_focus: Some(3),
            }
        );

        let mut view = MenuView::new(2);
        view.show(4);
        render_menu(&mut view, &["One", "Two"]);
        view.set_disabled(true);
        assert_eq!(
            view.pointer_down_at(Point::new(1.0, 1.0)),
            MenuEvent::Dismissed {
                restore_focus: Some(4),
            }
        );
    }

    #[test]
    fn menu_hover_changes_active_action_and_precise_scroll_capture_survives_outside_point() {
        let mut view = MenuView::new(8);
        view.show(2);
        let layout = render_menu(
            &mut view,
            &[
                "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
            ],
        );
        let point = Point::new(
            layout.items[2].frame.center().x,
            layout.items[2].frame.center().y,
        );
        view.pointer_move_at(point);
        assert_eq!(view.menu().items.selected, 2);

        let inside = Point::new(layout.popup.center().x, layout.popup.center().y);
        assert_eq!(
            view.wheel_event(MenuWheelInput {
                point: inside,
                delta: Point::new(0.0, -18.0),
                precise: true,
                begin: true,
                time: 1.0,
                ..MenuWheelInput::default()
            }),
            MenuWheelEvent::Consumed { scrolled: true }
        );
        assert_eq!(
            view.wheel_event(MenuWheelInput {
                point: Point::new(400.0, 400.0),
                delta: Point::new(0.0, -12.0),
                precise: true,
                end: true,
                momentum: true,
                time: 1.02,
                ..MenuWheelInput::default()
            }),
            MenuWheelEvent::Consumed { scrolled: true }
        );
        assert_eq!(
            view.wheel_event(MenuWheelInput {
                point: Point::new(400.0, 400.0),
                precise: true,
                time: 1.04,
                ..MenuWheelInput::default()
            }),
            MenuWheelEvent::Ignored
        );
    }

    #[test]
    fn menu_wheel_edge_rebound_advances_and_settles_through_owned_clock() {
        let labels = [
            "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
            "Eleven", "Twelve",
        ];
        for delta in [-400.0, 400.0] {
            let mut view = MenuView::new(labels.len());
            view.show(1);
            let mut scene = Scene::new();
            let layout = view
                .render(
                    &mut scene,
                    &mut FixedText,
                    &labels,
                    Rect::new(12., 12., 120., 34.),
                    Rect::new(0., 0., 320., 180.),
                    None,
                    palette(),
                    proposal::TextRole::UiBody,
                    false,
                )
                .unwrap();
            let inside = layout.popup.center();
            assert_eq!(
                view.wheel_event(MenuWheelInput {
                    point: inside,
                    delta: Point::new(0., delta),
                    precise: true,
                    begin: true,
                    time: 1.0,
                    ..MenuWheelInput::default()
                }),
                MenuWheelEvent::Consumed { scrolled: true }
            );
            assert_eq!(
                view.wheel_event(MenuWheelInput {
                    point: Point::new(400., 400.),
                    precise: true,
                    end: true,
                    momentum: true,
                    time: 1.02,
                    ..MenuWheelInput::default()
                }),
                MenuWheelEvent::Consumed { scrolled: false }
            );
            assert!(view.needs_animation(), "edge {delta} starts rebound");
            let bouncing = view
                .render(
                    &mut scene,
                    &mut FixedText,
                    &labels,
                    Rect::new(12., 12., 120., 34.),
                    Rect::new(0., 0., 320., 180.),
                    None,
                    palette(),
                    proposal::TextRole::UiBody,
                    false,
                )
                .unwrap();
            assert_ne!(bouncing.scroll_offset, 0.0, "edge {delta} visibly bounces");

            for _ in 0..240 {
                if !view.needs_animation() {
                    break;
                }
                view.tick(1.0 / 120.0);
            }
            assert!(!view.needs_animation(), "edge {delta} settles");
            let settled = view
                .render(
                    &mut scene,
                    &mut FixedText,
                    &labels,
                    Rect::new(12., 12., 120., 34.),
                    Rect::new(0., 0., 320., 180.),
                    None,
                    palette(),
                    proposal::TextRole::UiBody,
                    false,
                )
                .unwrap();
            let popup_inset = focus_reveal_outset(1.0);
            let document_height = labels.len() as f32 * 36.0
                + labels.len().saturating_sub(1) as f32 * popup_inset.max(4.0);
            let expected_rest = if delta < 0.0 {
                0.0
            } else {
                (document_height - (layout.popup.size.height - popup_inset * 2.0)).max(0.0)
            };
            assert!(
                (settled.scroll_offset - expected_rest).abs() < 0.01,
                "edge {delta} settles at scroll limit {expected_rest}, got {}",
                settled.scroll_offset
            );
        }
    }

    #[test]
    fn menu_reduced_motion_clears_open_menu_bounce_and_stops_frames() {
        let labels = [
            "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
        ];
        let mut view = MenuView::new(labels.len());
        view.show(1);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        let inside = layout.popup.center();
        view.wheel_event(MenuWheelInput {
            point: inside,
            delta: Point::new(0., -300.),
            precise: true,
            begin: true,
            time: 1.0,
            ..MenuWheelInput::default()
        });
        view.wheel_event(MenuWheelInput {
            point: Point::new(400., 400.),
            precise: true,
            end: true,
            momentum: true,
            time: 1.02,
            ..MenuWheelInput::default()
        });
        assert!(view.needs_animation());
        assert!(view.set_reduced_motion(true));
        assert!(!view.needs_animation());
        let cleared = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_eq!(cleared.scroll_offset, 0.0);

        // The preference remains active for a later edge gesture while the
        // menu stays open: wheel input is consumed, but no elastic frame runs.
        view.wheel_event(MenuWheelInput {
            point: cleared.popup.center(),
            delta: Point::new(0., -300.),
            precise: true,
            begin: true,
            time: 2.0,
            ..MenuWheelInput::default()
        });
        view.wheel_event(MenuWheelInput {
            point: Point::new(400., 400.),
            precise: true,
            end: true,
            momentum: true,
            time: 2.02,
            ..MenuWheelInput::default()
        });
        assert!(!view.needs_animation());
        let still = view
            .render(
                &mut scene,
                &mut FixedText,
                &labels,
                Rect::new(12., 12., 120., 34.),
                Rect::new(0., 0., 320., 180.),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_eq!(still.scroll_offset, 0.0);
    }

    #[test]
    fn menu_hover_target_is_the_action_enter_activates() {
        let mut view = MenuView::new(3);
        view.show(1);
        let layout = render_menu(&mut view, &["One", "Two", "Three"]);
        let row = layout.items[2].frame;
        view.pointer_move_at(Point::new(row.center().x, row.center().y));
        assert_eq!(
            view.key(ControlKey::Enter, true),
            MenuEvent::Activated {
                index: 2,
                restore_focus: Some(1),
            }
        );
    }

    #[test]
    fn dialog_tab_then_enter_confirms_without_render_and_escape_closes_unrendered_dialog() {
        let mut view = DialogView::default();
        view.show(9);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                "Confirm action?",
                Rect::new(0.0, 0.0, 600.0, 400.0),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open dialog layout");
        assert_eq!(
            view.key(ControlKey::Tab { reverse: false }, true),
            DialogEvent::Blocked
        );
        assert_eq!(
            view.key(ControlKey::Enter, true),
            DialogEvent::Confirmed {
                restore_focus: Some(9),
            }
        );
        assert!(layout.buttons.iter().all(|button| button.visible.is_some()));

        let mut view = DialogView::default();
        view.show(5);
        assert_eq!(
            view.key(ControlKey::Escape, true),
            DialogEvent::Cancelled {
                restore_focus: Some(5),
            }
        );
        assert!(!view.is_open());
    }

    #[test]
    fn menu_and_dialog_surfaces_use_semantic_roles_and_renderer_scale() {
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                let palette = Palette::from(theme);
                let mut menu = MenuView::new(2);
                menu.show(1);
                let mut text = ScaledText(scale);
                let mut scene = Scene::new();
                let menu_layout = menu
                    .render(
                        &mut scene,
                        &mut text,
                        &["First", "Second"],
                        Rect::new(20.0, 20.0, 100.0, 32.0),
                        Rect::new(0.0, 0.0, 320.0, 240.0),
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    )
                    .expect("open menu layout");
                let menu_quad = scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == menu_layout.popup => {
                            Some(q)
                        }
                        _ => None,
                    })
                    .expect("menu surface quad");
                assert_eq!(menu_quad.fill, palette.color("surface-popover").unwrap());
                assert_eq!(
                    menu_quad.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(
                        proposal::metric("radius", "radius-floating").unwrap(),
                    )
                );
                assert_eq!(
                    menu_quad.border_width,
                    paint::decorative_border(palette, scale).0
                );

                let mut dialog = DialogView::default();
                dialog.show(2);
                let mut dialog_scene = Scene::new();
                let dialog_layout = dialog
                    .render(
                        &mut dialog_scene,
                        &mut text,
                        "Confirm?",
                        Rect::new(0.0, 0.0, 600.0, 420.0),
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                        false,
                    )
                    .expect("open dialog layout");
                let dialog_quad = dialog_scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == dialog_layout.modal => {
                            Some(q)
                        }
                        _ => None,
                    })
                    .expect("dialog surface quad");
                assert_eq!(dialog_quad.fill, palette.color("surface-popover").unwrap());
                assert_eq!(
                    dialog_quad.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(
                        proposal::metric("radius", "c-dialog-radius").unwrap(),
                    )
                );
                assert_eq!(
                    dialog_quad.border_width,
                    paint::decorative_border(palette, scale).0
                );
            }
        }
    }

    #[test]
    fn dialog_inactive_input_is_ignored_but_explicit_cancel_closes() {
        let mut view = DialogView::default();
        view.show(3);
        assert!(view.blocks_background());
        view.deactivate();
        assert!(!view.blocks_background());
        assert_eq!(view.key(ControlKey::Escape, true), DialogEvent::None);
        assert_eq!(
            view.cancel(),
            DialogEvent::Cancelled {
                restore_focus: Some(3),
            }
        );
        assert!(!view.is_open());
    }

    #[test]
    fn disabled_dialog_still_closes_with_escape_but_rejects_activation() {
        let mut view = DialogView::default();
        view.show(8);
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut FixedText,
            "Confirm?",
            Rect::new(0.0, 0.0, 600.0, 400.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        );
        view.set_disabled(true);
        assert_eq!(view.key(ControlKey::Enter, true), DialogEvent::None);
        assert!(view.is_open());
        assert_eq!(
            view.key(ControlKey::Escape, true),
            DialogEvent::Cancelled {
                restore_focus: Some(8),
            }
        );
        assert!(!view.is_open());
    }

    #[test]
    fn open_dialog_blocks_host_background_scroll_outside_modal_preview() {
        let mut view = DialogView::default();
        view.show(3);
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut FixedText,
            "Confirm?",
            Rect::new(100.0, 100.0, 400.0, 300.0),
            None,
            palette(),
            proposal::TextRole::UiBody,
            false,
        );

        let header_point = Point::new(20.0, 24.0);
        assert_eq!(view.wheel_at(header_point), DialogEvent::None);
        assert!(view.blocks_background());
        assert_eq!(
            view.wheel_at(Point::new(120.0, 120.0)),
            DialogEvent::Blocked
        );
    }

    #[test]
    fn short_dialog_stacks_actions_without_overlapping_or_leaving_modal() {
        let mut view = DialogView::default();
        view.show(0);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut FixedText,
                "Confirm?",
                Rect::new(0.0, 0.0, 180.0, 120.0),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open dialog layout");
        let [cancel, confirm] = layout.buttons;
        assert!(cancel.frame.max_y() <= confirm.frame.min_y());
        assert!(cancel.frame.min_y() >= layout.modal.min_y());
        assert!(confirm.frame.max_y() <= layout.modal.max_y());
    }

    #[test]
    fn large_dialog_text_keeps_its_measure_and_actions_clip_to_separate_slots() {
        let mut view = DialogView::default();
        view.show(0);
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut TallText,
                "A very long title that needs several lines",
                Rect::new(0.0, 0.0, 80.0, 120.0),
                None,
                palette(),
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open dialog layout");

        assert_eq!(layout.title.size.height, 100.0);
        let [cancel, confirm] = layout.buttons;
        let cancel_visible = cancel.visible.expect("cancel has a visible slot");
        let confirm_visible = confirm.visible.expect("confirm has a visible slot");
        assert!(
            cancel_visible.intersection(&confirm_visible).is_none(),
            "cancel={cancel_visible:?} confirm={confirm_visible:?}"
        );
        assert!(!cancel.hit_test(confirm_visible.center()));
        assert!(!confirm.hit_test(cancel_visible.center()));
    }
}

#[cfg(test)]
mod overlay_presence_tests {
    use super::*;
    use crate::{
        ControlMotionContext,
        extended::Popover,
        feedback_view::{OverlayEvent, PopoverView},
    };
    use std::time::Duration;
    use tekes_ui_tokens::{Rgba, Theme};

    struct ClockText {
        millis: u64,
        reduced: bool,
        requested: bool,
    }
    impl ClockText {
        fn new() -> Self {
            Self {
                millis: 0,
                reduced: false,
                requested: false,
            }
        }
    }
    impl ButtonTextRenderer for ClockText {
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: Duration::from_millis(self.millis),
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.requested = true;
        }
        fn measure(&mut self, label: &str, _: proposal::TextRole, max: f32) -> Size {
            Size::new((label.len() as f32 * 7.0).min(max), 18.0)
        }
        fn draw(
            &mut self,
            scene: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            frame: Rect,
            _: ButtonTextAlign,
            color: Rgba,
        ) {
            scene.push_quad(Quad::new(frame, color));
        }
    }
    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 600.0, 400.0)
    }
    fn menu(view: &mut MenuView, text: &mut ClockText, scene: &mut Scene) -> Option<MenuLayout> {
        view.render(
            scene,
            text,
            &["Open", "Copy"],
            Rect::new(20.0, 20.0, 120.0, 36.0),
            viewport(),
            Some(viewport()),
            Palette::from(Theme::default()),
            proposal::TextRole::UiBody,
            false,
        )
    }
    fn dialog(
        view: &mut DialogView,
        text: &mut ClockText,
        scene: &mut Scene,
        clip: Option<Rect>,
    ) -> Option<DialogLayout> {
        view.render(
            scene,
            text,
            "Confirm changes?",
            viewport(),
            clip,
            Palette::from(Theme::default()),
            proposal::TextRole::UiBody,
            false,
        )
    }
    fn popover(
        view: &mut PopoverView,
        model: &Popover,
        text: &mut ClockText,
        scene: &mut Scene,
        palette: Palette,
    ) {
        view.render_with_text_content(
            scene,
            text,
            model,
            Rect::new(20.0, 20.0, 120.0, 36.0),
            Size::new(220.0, 120.0),
            viewport(),
            Some(viewport()),
            palette,
            proposal::TextRole::UiBody,
            "content-v1",
            |scene, _, bounds| scene.push_quad(Quad::new(bounds, Rgba::new(1.0, 0.0, 0.0, 1.0))),
        );
    }
    #[test]
    fn window_deactivation_clears_popover_presence_without_replaying_exit() {
        for closing in [false, true] {
            let mut view = PopoverView::default();
            let mut model = Popover::default();
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(&mut model, 7);
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            text.millis = 40;
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            if closing {
                view.close(&mut model);
            }
            assert!(view.needs_animation());
            view.set_window_active(false, &mut model);
            assert!(!view.needs_animation());
            assert!(!view.is_presented());
            assert!(!model.open);
            text.requested = false;
            scene.clear();
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            assert!(scene.is_empty());
            assert!(!text.requested);
            view.set_window_active(true, &mut model);
            text.millis = 60;
            popover(
                &mut view,
                &model,
                &mut text,
                &mut scene,
                Theme::LIGHT.into(),
            );
            assert!(scene.is_empty());
            assert!(!text.requested);
        }
    }
    #[test]
    fn filtered_menu_dismissal_keeps_exit_but_cancel_discards_presentation() {
        let mut view = crate::layout_view::FilteredMenuView::new();
        let mut text = ClockText::new();
        let mut scene = Scene::new();
        let items = vec!["Open".to_owned(), "Copy".to_owned()];
        let paint = |view: &mut crate::layout_view::FilteredMenuView,
                     text: &mut ClockText,
                     scene: &mut Scene,
                     open| {
            view.render(
                scene,
                text,
                &items,
                &[7, 8],
                Some(0),
                open,
                Rect::new(20., 20., 120., 36.),
                viewport(),
                Some(viewport()),
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            );
        };
        paint(&mut view, &mut text, &mut scene, true);
        text.millis = 40;
        paint(&mut view, &mut text, &mut scene, true);
        view.dismiss();
        assert!(!view.is_visible());
        assert!(view.layout().is_none());
        assert!(
            view.is_presented(),
            "ordinary filtered dismissal must not mean window deactivation"
        );
        scene.clear();
        paint(&mut view, &mut text, &mut scene, false);
        assert!(!scene.is_empty());
        assert_eq!(view.key(ControlKey::Enter, true), MenuEvent::None);
        view.cancel();
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn ordinary_menu_deactivation_keeps_exit_until_window_is_inactive() {
        let mut view = MenuView::new(2);
        let mut text = ClockText::new();
        let mut scene = Scene::new();
        view.show(7);
        menu(&mut view, &mut text, &mut scene);
        text.millis = 40;
        menu(&mut view, &mut text, &mut scene);
        view.deactivate();
        assert!(!view.is_open());
        assert!(
            view.is_presented(),
            "ordinary focus return retains exit paint"
        );
        assert!(view.needs_animation());
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert!(!scene.is_empty());
        view.set_window_active(false);
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn window_deactivation_stops_menu_presence_without_render_or_replay() {
        for closing in [false, true] {
            let mut view = MenuView::new(2);
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(7);
            menu(&mut view, &mut text, &mut scene);
            text.millis = 40;
            menu(&mut view, &mut text, &mut scene);
            if closing {
                view.key(ControlKey::Escape, true);
            }
            assert!(view.needs_animation());
            view.set_window_active(false);
            assert!(
                !view.needs_animation(),
                "inactive menu stops pending presence, closing={closing}"
            );
            assert!(!view.is_presented());
            text.requested = false;
            scene.clear();
            menu(&mut view, &mut text, &mut scene);
            assert!(scene.is_empty());
            assert!(!text.requested);
            view.set_window_active(true);
            text.millis = 60;
            menu(&mut view, &mut text, &mut scene);
            assert!(!view.needs_animation());
            assert!(scene.is_empty());
        }
    }
    #[test]
    fn window_deactivation_settles_dialog_presence_to_its_logical_state() {
        for closing in [false, true] {
            let mut view = DialogView::default();
            let mut text = ClockText::new();
            let mut scene = Scene::new();
            view.show(7);
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            text.millis = 40;
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            if closing {
                view.close();
            }
            assert!(view.needs_animation());
            view.set_window_active(false);
            assert!(
                !view.needs_animation(),
                "inactive dialog stops pending presence, closing={closing}"
            );
            assert_eq!(view.dialog().open, !closing);
            text.requested = false;
            scene.clear();
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            assert_eq!(scene.is_empty(), closing);
            assert!(!text.requested);
            view.set_window_active(true);
            text.millis = 60;
            text.requested = false;
            dialog(&mut view, &mut text, &mut scene, Some(viewport()));
            assert!(!view.needs_animation());
            assert!(!text.requested);
        }
    }
    #[test]
    fn menu_exit_keeps_only_paint_and_reopen_continues_current_progress() {
        let mut text = ClockText::new();
        let mut view = MenuView::new(2);
        let mut scene = Scene::new();
        view.show(8);
        assert!(menu(&mut view, &mut text, &mut scene).is_some());
        assert!(text.requested && view.needs_animation());
        text.millis = 200;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        let resting = scene.clone();
        assert!(!resting.is_empty());
        assert!(matches!(
            view.key(ControlKey::Escape, true),
            MenuEvent::Dismissed {
                restore_focus: Some(8)
            }
        ));
        assert!(!view.is_open());
        scene.clear();
        assert!(menu(&mut view, &mut text, &mut scene).is_none());
        assert_eq!(scene, resting);
        assert!(view.is_presented());
        assert_eq!(
            view.pointer_down_at(Point::new(30.0, 80.0)),
            MenuEvent::None
        );
        text.millis = 240;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        let exiting = scene.clone();
        assert!(!exiting.is_empty());
        assert_ne!(exiting, resting);
        view.show(9);
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert_eq!(scene, exiting);
        text.millis = 500;
        scene.clear();
        menu(&mut view, &mut text, &mut scene);
        assert_eq!(scene, resting);
        view.clear_presentation();
        assert!(!view.is_presented());
    }
    #[test]
    fn dialog_dismissal_restores_focus_once_and_reduced_motion_finishes_exit() {
        let mut text = ClockText::new();
        let mut view = DialogView::default();
        let mut scene = Scene::new();
        view.show(3);
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        text.millis = 200;
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        assert!(matches!(
            view.close(),
            DialogEvent::Cancelled {
                restore_focus: Some(3)
            }
        ));
        assert_eq!(view.close(), DialogEvent::None);
        assert!(!view.blocks_background());
        assert_eq!(view.key(ControlKey::Enter, true), DialogEvent::None);
        scene.clear();
        assert!(dialog(&mut view, &mut text, &mut scene, Some(viewport())).is_none());
        assert!(!scene.is_empty());
        assert!(view.needs_animation());
        text.reduced = true;
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        assert!(scene.is_empty());
        assert!(!view.is_presented());
        assert!(!view.needs_animation());
    }
    #[test]
    fn dialog_exit_is_cancelled_when_current_ancestor_clip_changes() {
        let mut text = ClockText::new();
        let mut view = DialogView::default();
        let mut scene = Scene::new();
        view.show(3);
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        text.millis = 200;
        dialog(&mut view, &mut text, &mut scene, Some(viewport()));
        view.close();
        scene.clear();
        dialog(&mut view, &mut text, &mut scene, Some(Rect::default()));
        assert!(scene.is_empty());
        assert!(!view.is_presented());
    }
    #[test]
    fn popover_retains_content_but_no_focus_or_hits_and_cancels_stale_brand() {
        let mut text = ClockText::new();
        let mut view = PopoverView::default();
        let mut model = Popover::default();
        let mut scene = Scene::new();
        let palette = Palette::from(Theme::default());
        view.show(&mut model, 4);
        popover(&mut view, &model, &mut text, &mut scene, palette);
        text.millis = 200;
        scene.clear();
        popover(&mut view, &model, &mut text, &mut scene, palette);
        let resting = scene.clone();
        assert!(matches!(
            view.close(&mut model),
            OverlayEvent::Dismissed {
                restore_focus: Some(4)
            }
        ));
        assert!(view.focus_target().is_none());
        assert!(!view.pointer_down(&mut model, Point::new(30.0, 80.0)));
        scene.clear();
        popover(&mut view, &model, &mut text, &mut scene, palette);
        assert_eq!(scene, resting);
        assert!(view.is_presented());
        scene.clear();
        popover(
            &mut view,
            &model,
            &mut text,
            &mut scene,
            Palette::new(palette.theme, 1).expect("registered Gallery brand"),
        );
        assert!(scene.is_empty());
        assert!(!view.is_presented());
    }

    #[test]
    fn tabs_invalid_and_hidden_layout_cancel_focus_and_capture() {
        let mut view = TabsView::new(2);
        let mut scene = Scene::new();
        let mut text = ClockText::new();
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 240., 40.);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let point = view.placement.items[0].frame.center();
        assert_eq!(view.pointer_down_at(point), Some(0));
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            Rect::new(f32::NAN, 0., 240., 40.),
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(view.pointer_up_at(point), None);
        assert_eq!(view.pointer_down_at(point), None);
        assert_eq!(view.key(ControlKey::Enter, true), None);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(0, true);
        view.key(ControlKey::Space, true);
        view.render(
            &mut scene,
            &mut text,
            &["One", "Two"],
            frame,
            Some(Rect::new(500., 500., 20., 20.)),
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(view.key(ControlKey::Space, false), None);
        assert_eq!(view.key(ControlKey::Right, true), None);
        assert_eq!(view.selected(), 0);
    }
}
