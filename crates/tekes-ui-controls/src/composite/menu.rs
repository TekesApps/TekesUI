use super::menu_runtime::MenuRuntime;
use super::{
    ButtonLayout, ButtonModel, ButtonTextAlign, ButtonTextRenderer, ControlKey, Menu, ScrollArea,
    ScrollPolicy, focus_reveal_outset, menu_width_limit, size_role,
};
use crate::control_motion::OverlayPresence;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

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
    pub(super) model: Menu,
    pub(super) bound_external: bool,
    pub(super) runtime: MenuRuntime,
    // Optional adapter-owned declaration, within the already private original
    // runtime storage. MenuContent's legacy public struct shape stays intact.
    pub(crate) content_state: Option<crate::composition::MenuContentState>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MenuRowStyle {
    pub disabled: bool,
    pub checked: Option<bool>,
    pub separator: bool,
    pub submenu: bool,
}

// Original ButtonModel still owns chrome, ink, motion, hit layout and clipping.
// This label adapter reserves shared icon columns without another painter.
pub(super) struct MenuRowText<'a, T> {
    pub(super) text: &'a mut T,
    pub(super) leading: f32,
    pub(super) trailing: f32,
    pub(super) gap: f32,
    pub(super) style: MenuRowStyle,
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

// Popup height follows measured rows and surface insets. Placement bounds
// this intrinsic height to the available viewport and retains scrolling.
pub(super) fn menu_surface_height(content: f32, inset: f32) -> f32 {
    content + inset * 2.0
}

pub(super) fn menu_surface_height_for_rows(rows: &[f32], inset: f32) -> f32 {
    menu_surface_height(
        rows.iter().sum::<f32>() + rows.len().saturating_sub(1) as f32 * inset.max(4.0),
        inset,
    )
}

// Optional header geometry belongs to the existing popup and shares its inset,
// placement, clipping, and scrolling viewport.
pub(crate) type MenuHeader<'a> = (
    Size,
    &'a mut dyn FnMut(&mut Scene, &mut dyn ButtonTextRenderer, Rect),
);

pub(crate) struct MenuRenderContext<'a, T: ButtonTextRenderer> {
    pub scene: &'a mut Scene,
    pub text: &'a mut T,
    pub anchor: Rect,
    pub visible: Rect,
    pub clip: Option<Rect>,
    pub palette: Palette,
    pub role: proposal::TextRole,
    pub compact: bool,
}

pub(crate) fn menu_popup_geometry(
    anchor: Rect,
    natural: Size,
    viewport: Rect,
    inset: f32,
    row_heights: &[f32],
    beside: bool,
    header: Option<Size>,
) -> (Rect, Rect, Rect) {
    let gap = proposal::metric("spacing", "space-micro").expect("canonical popup spacing");
    let header_height = header.map_or(0.0, |size| size.height.max(0.0));
    let header_gap = if header.is_some() && !row_heights.is_empty() {
        inset.max(4.0)
    } else {
        0.0
    };
    let rows_height = if header.is_some() && row_heights.is_empty() {
        inset * 2.0
    } else {
        natural.height
    };
    let size = Size::new(
        natural.width,
        (rows_height + header_height + header_gap).max(0.0),
    );
    let viewport_height = if viewport.size.height.is_finite() {
        viewport.size.height.max(0.0)
    } else {
        0.0
    };
    let below = (viewport.max_y() - anchor.max_y() - gap).clamp(0.0, viewport_height);
    let above = (anchor.min_y() - gap - viewport.min_y()).clamp(0.0, viewport_height);
    let available = below.max(above);
    let first = row_heights.first().copied().unwrap_or(0.0);
    let minimum = first + inset * 2.0 + header_height + header_gap;
    let limit = if beside || available < minimum {
        viewport.size.height
    } else {
        available
    };
    let size = Size::new(size.width, size.height.min(limit));
    let popup = if beside {
        let transpose = |r: Rect| Rect::new(r.min_y(), r.min_x(), r.size.height, r.size.width);
        transpose(crate::feedback_view::place_overlay(
            transpose(anchor),
            Size::new(size.height, size.width),
            transpose(viewport),
            gap,
        ))
    } else {
        crate::feedback_view::place_overlay(anchor, size, viewport, gap)
    };
    let inner = Rect::new(
        popup.min_x() + inset,
        popup.min_y() + inset,
        (popup.size.width - inset * 2.0).max(0.0),
        (popup.size.height - inset * 2.0).max(0.0),
    );
    let header_frame = Rect::new(
        inner.min_x(),
        inner.min_y(),
        inner.size.width,
        header_height.min(inner.size.height),
    );
    let rows_top = (header_frame.max_y() + header_gap).min(inner.max_y());
    let rows = Rect::new(
        inner.min_x(),
        rows_top,
        inner.size.width,
        (inner.max_y() - rows_top).max(0.0),
    );
    (popup, header_frame, rows)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MenuPreparedKind {
    Open { generation: u64, selected: usize },
    Focus { index: usize, offset: f32 },
    Activate { index: usize },
    Close,
    Noop,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MenuPreparedAction {
    pub(super) expected: MenuPreparedState,
    pub(super) kind: MenuPreparedKind,
    pub event: MenuEvent,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MenuPreparedState {
    pub(super) model: (bool, usize, usize, Option<usize>),
    pub(super) generation: u64,
    pub(super) availability: (bool, bool, bool),
    pub(super) return_focus: Option<usize>,
    pub(super) layout: MenuLayout,
    pub(super) row_styles: Vec<MenuRowStyle>,
    pub(super) row_heights: Vec<f32>,
    pub(super) viewport: Rect,
    pub(super) offset: Point,
    pub(super) limit: Point,
    pub(super) focus_outset: f32,
    pub(super) gesture: bool,
    pub(super) buttons: Vec<crate::widgets::ButtonActionDecision>,
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
    pub(super) fn measured_rows(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        max_width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> (Size, Vec<f32>) {
        let inset = focus_reveal_outset(text.backing_scale());
        let width = Self::preferred_menu_width(text, labels, max_width, role, compact);
        let inner_width = (width - inset * 2.0).max(0.0);
        let heights = Self::measure_row_heights(text, labels, inner_width, role, compact);
        (
            Size::new(width, menu_surface_height_for_rows(&heights, inset)),
            heights,
        )
    }
    fn preferred_menu_width(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        max_width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> f32 {
        let max_width = menu_width_limit(max_width);
        let inset = focus_reveal_outset(text.backing_scale());
        if labels.is_empty() {
            return max_width;
        }
        let max_inner_width = (max_width - inset * 2.0).max(0.0);
        let preferred_width = labels
            .iter()
            .map(|label| {
                ButtonModel::measure(text, label, max_inner_width, size_role(compact), role).width
            })
            .fold(0.0, f32::max);
        (preferred_width + inset * 2.0).min(max_width)
    }
    pub(super) fn measure_row_heights(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        inner_width: f32,
        role: proposal::TextRole,
        compact: bool,
    ) -> Vec<f32> {
        labels
            .iter()
            .map(|label| {
                ButtonModel::measure(text, label, inner_width, size_role(compact), role)
                    .height
                    .max(36.0)
            })
            .collect()
    }
    pub(crate) fn header_content_width(text: &mut impl ButtonTextRenderer, width: f32) -> f32 {
        (menu_width_limit(width) - focus_reveal_outset(text.backing_scale()) * 2.0).max(0.0)
    }

    pub(crate) fn header_frame(
        text: &mut impl ButtonTextRenderer,
        labels: &[&str],
        header: Size,
        anchor: Rect,
        viewport: Rect,
        role: proposal::TextRole,
        compact: bool,
    ) -> Rect {
        let inset = focus_reveal_outset(text.backing_scale());
        let width = Self::preferred_menu_width(text, labels, viewport.size.width, role, compact)
            .max((header.width + inset * 2.0).min(menu_width_limit(viewport.size.width)));
        let rows =
            Self::measure_row_heights(text, labels, (width - inset * 2.0).max(0.0), role, compact);
        let natural = Size::new(width, menu_surface_height_for_rows(&rows, inset));
        menu_popup_geometry(anchor, natural, viewport, inset, &rows, false, Some(header)).1
    }

    pub(crate) fn render_with_header(
        &mut self,
        model: &mut Menu,
        labels: &[&str],
        context: MenuRenderContext<'_, impl ButtonTextRenderer>,
        header: Size,
        paint_header: &mut dyn FnMut(&mut Scene, &mut dyn ButtonTextRenderer, Rect),
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
        self.bound_external = true;
        self.runtime.render_model(
            model,
            scene,
            text,
            labels,
            anchor,
            visible,
            clip,
            palette,
            role,
            compact,
            Some((header, paint_header)),
        )
    }

    pub(crate) fn cancel_pointer_arm(&mut self) {
        for button in &mut self.runtime.buttons {
            button.cancel_pointer();
        }
        self.runtime.scroll.cancel();
        self.runtime.scroll_gesture_captured = false;
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
                buttons: vec![ButtonModel::default(); count],
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
        self.runtime.replace_items_model(&mut self.model, count);
    }

    #[cfg(test)]
    pub(crate) fn select_test_generation(&mut self, generation: u64) {
        self.runtime.open_generation = generation;
    }
    pub(crate) fn prepare_select_action(
        &self,
        model: &Menu,
        kind: MenuPreparedKind,
    ) -> Option<MenuPreparedAction> {
        let kind = match kind {
            MenuPreparedKind::Open { selected, .. } => {
                if !self.runtime.window_active || self.runtime.disabled || model.items.count == 0 {
                    return None;
                }
                if model.open {
                    MenuPreparedKind::Noop
                } else {
                    MenuPreparedKind::Open {
                        generation: self.next_open_generation()?,
                        selected: selected.min(model.items.count - 1),
                    }
                }
            }
            MenuPreparedKind::Focus { index, .. } => {
                if !self.runtime.semantic_item_mounted(model, index) {
                    return None;
                }
                MenuPreparedKind::Focus {
                    index,
                    offset: self.runtime.reveal_offset(index),
                }
            }
            MenuPreparedKind::Activate { index } => {
                if !self.runtime.semantic_item_mounted(model, index)
                    || self.runtime.row_style(index).submenu
                {
                    return None;
                }
                MenuPreparedKind::Activate { index }
            }
            MenuPreparedKind::Close => MenuPreparedKind::Close,
            MenuPreparedKind::Noop => MenuPreparedKind::Noop,
        };
        let event = match kind {
            MenuPreparedKind::Activate { index } => MenuEvent::Activated {
                index,
                restore_focus: self.runtime.return_focus,
            },
            MenuPreparedKind::Close if model.open => MenuEvent::Dismissed {
                restore_focus: self.runtime.return_focus,
            },
            _ => MenuEvent::None,
        };
        Some(MenuPreparedAction {
            expected: self.runtime.select_prepared_state(model),
            kind,
            event,
        })
    }
    pub(crate) fn apply_select_action(&mut self, model: &mut Menu, action: MenuPreparedAction) {
        self.bound_external = true;
        match action.kind {
            MenuPreparedKind::Open {
                generation,
                selected,
            } => {
                self.runtime.window_active = true;
                self.runtime.open_generation = generation;
                model.open = true;
                model.items.selected = selected;
                self.runtime.return_focus = Some(0);
                self.runtime.focus_visible = false;
                self.runtime.scroll.offset.y = 0.;
                self.runtime.clear_layout();
                self.runtime.sync_focus(selected);
            }
            MenuPreparedKind::Focus { index, offset } => {
                model.items.selected = index;
                self.runtime.focus_visible = true;
                self.runtime.scroll.offset.y = offset;
                self.runtime.sync_focus(index);
            }
            MenuPreparedKind::Activate { index } => {
                model.items.selected = index;
                model.last_action = Some(index);
                model.open = false;
                self.runtime.clear_layout();
                self.runtime.clear_button_pointer_positions();
                self.runtime.return_focus = None;
            }
            MenuPreparedKind::Close => {
                model.open = false;
                self.runtime.clear_layout();
                self.runtime.clear_button_pointer_positions();
                self.runtime.return_focus = None;
            }
            MenuPreparedKind::Noop => {}
        }
    }
    pub(crate) fn replace_items_with_model(&mut self, model: &mut Menu, count: usize) {
        self.bound_external = true;
        self.runtime.replace_items_model(model, count);
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
    pub(crate) fn next_open_generation(&self) -> Option<u64> {
        self.runtime.open_generation.checked_add(1)
    }
    pub(crate) fn apply_open_generation(
        &mut self,
        model: &mut Menu,
        invoker: usize,
        generation: u64,
    ) {
        debug_assert_eq!(self.next_open_generation(), Some(generation));
        self.bound_external = true;
        self.runtime.open_generation = generation;
        model.show();
        self.runtime.return_focus = Some(invoker);
        self.runtime.focus_visible = false;
        self.runtime
            .scroll
            .scroll_by(0.0, -self.runtime.scroll.offset.y);
        self.runtime.clear_layout();
        self.runtime.sync_focus(model.items.selected);
    }
    pub(crate) fn pointer_armed(&self) -> bool {
        self.runtime.buttons.iter().any(ButtonModel::pointer_armed)
    }
    pub(crate) fn pointer_release_index(&self, model: &Menu, point: Point) -> Option<usize> {
        if !model.open || !self.runtime.window_active {
            return None;
        }
        self.runtime
            .buttons
            .iter()
            .enumerate()
            .find_map(|(index, button)| {
                (button.pointer_release_would_activate(point) && self.runtime.item_enabled(index))
                    .then_some(index)
            })
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
            .next_open_generation()
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
        let generation = self
            .next_open_generation()
            .expect("menu popup generation exhausted");
        self.apply_open_generation(model, invoker, generation);
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
            None,
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
            None,
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
