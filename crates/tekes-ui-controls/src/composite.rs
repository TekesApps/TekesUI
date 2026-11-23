//! Complete production presentations for controls composed from Buttons.
//!
//! These runtimes keep the old public `Tabs`, `Menu`, and `Dialog` models while
//! centralizing their layout, clipping, paint, pointer capture, keyboard focus,
//! and activation behavior. A runtime is bound either to its owned model or to
//! one external model for its lifetime; do not alternate between them.
use crate::{
    ButtonLayout, ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout,
    ControlKey, Dialog, Insets, Menu, ScrollArea, ScrollPolicy, Tabs, paint,
};
use tekes_ui_scene::{Rect, Size};
use tekes_ui_tokens::proposal;

pub(crate) const MENU_MAX_WIDTH: f32 = 280.0;

pub(super) fn menu_max_width_limit(max_width: f32, ceiling: f32) -> f32 {
    if max_width.is_nan() || ceiling.is_nan() {
        0.0
    } else {
        max_width.clamp(0.0, ceiling.max(0.0))
    }
}

pub(super) fn menu_width_limit(max_width: f32) -> f32 {
    menu_max_width_limit(max_width, MENU_MAX_WIDTH)
}

pub(crate) fn command_palette_max_width() -> f32 {
    proposal::metric("size", "c-settings-content-max-width")
        .expect("canonical command palette max width")
}

pub(super) fn clip_rect(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
    ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, clip).visible_clip()
}

pub(super) fn safe_rect(rect: Rect) -> Rect {
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

pub(super) fn explicit_visible_clip(visible: Option<Rect>) -> Option<Rect> {
    Some(visible.unwrap_or_default())
}

pub(crate) fn focus_reveal_outset(backing_scale: f32) -> f32 {
    let ring_width = proposal::metric("stroke", "c-focus-width").expect("focus width token");
    let ring_gap = proposal::metric("spacing", "c-focus-offset").expect("focus offset token");
    let scale = if backing_scale.is_finite() && backing_scale > 0.0 {
        backing_scale
    } else {
        1.0
    };
    ring_width + ring_gap + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS / scale
}

pub(super) fn set_button_state(
    button: &mut ButtonModel,
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

pub(super) fn size_role(compact: bool) -> ButtonSize {
    let _ = compact;
    // Compact density changes the outer row metric. Body is retained so
    // typography and horizontal inset stay equal to existing Gallery actions.
    ButtonSize::Body
}

pub(super) fn row_height(compact: bool) -> f32 {
    proposal::metric(
        "size",
        if compact {
            "size-button-sm"
        } else {
            "size-row-height"
        },
    )
    .expect("tab row height")
}

pub(super) fn menu_size_role(compact: bool) -> ButtonSize {
    if compact {
        ButtonSize::Body
    } else {
        ButtonSize::Sm
    }
}

pub(super) fn menu_min_row_height(compact: bool) -> f32 {
    if compact {
        proposal::metric("size", "size-compact-row").expect("canonical compact menu row height")
    } else {
        proposal::metric("size", "size-button-sm").expect("canonical menu row height")
    }
}

pub(crate) fn menu_popup_inset() -> f32 {
    proposal::metric("spacing", "space-snug").expect("canonical menu popup inset")
}

pub(super) fn menu_row_gap() -> f32 {
    proposal::metric("spacing", "space-none").expect("canonical menu row gap")
}

pub(super) fn menu_header_gap() -> f32 {
    proposal::metric("spacing", "space-tight").expect("canonical menu header gap")
}

pub(super) fn menu_separator_height() -> f32 {
    proposal::metric("spacing", "space-regular").expect("canonical menu separator spacing")
        + proposal::metric("stroke", "stroke-rule").expect("canonical menu separator rule")
}

pub(super) fn menu_row_radius() -> f32 {
    tekes_ui_tokens::metrics::CornerRadius::concentric(
        proposal::metric("radius", "radius-floating").expect("canonical floating surface radius"),
        menu_popup_inset(),
    )
}

pub(super) fn menu_typography_scale(
    text: &mut impl ButtonTextRenderer,
    role: proposal::TextRole,
) -> f32 {
    let resolved = text.resolved_font_size(role);
    let base = role.style().size;
    if resolved.is_finite() && resolved > 0.0 && base.is_finite() && base > 0.0 {
        resolved / base
    } else {
        1.0
    }
}

pub(super) fn menu_row_horizontal_inset(
    text: &mut impl ButtonTextRenderer,
    role: proposal::TextRole,
) -> f32 {
    proposal::metric("spacing", "space-regular").expect("canonical menu row horizontal inset")
        * menu_typography_scale(text, role)
}

pub(super) fn empty_button_layout() -> ButtonLayout {
    ButtonLayout {
        frame: Rect::default(),
        content: Rect::default(),
        visible: None,
        label_size: Size::default(),
    }
}

mod dialog;
mod menu;
mod menu_runtime;
mod tabs;

pub use dialog::{DialogEvent, DialogLayout, DialogView};
pub use menu::{MenuEvent, MenuLayout, MenuView, MenuWheelEvent, MenuWheelInput};
pub(crate) use menu::{
    MenuHeader, MenuPreparedAction, MenuPreparedKind, MenuRenderContext, MenuRowStyle,
};
pub use tabs::{TabsLayout, TabsView};

#[cfg(test)]
mod tests_1;
#[cfg(test)]
mod tests_2;
