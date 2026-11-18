use super::{
    clipped, draw_text, effective_viewport, finite_rect, paint_surface_background, safe_inset,
    valid_logical_target,
};
use crate::control_motion::OverlayPresence;
use crate::extended::{Popover, Tooltip};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey,
    Insets, paint,
};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

/// Tooltip positioning is anchored and clamped to the visible preview bounds.
#[derive(Clone, Debug, Default)]
pub struct TooltipView {
    layout: OverlayLayout,
    hovered: bool,
    focused: bool,
    focus_visible: bool,
    dismissed_until_leave: bool,
    controlled_suppressed: bool,
    controlled_visible: bool,
    anchor_visible: Option<Rect>,
    inactive: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayLayout {
    pub frame: Rect,
    pub content: Rect,
    pub visible: Option<Rect>,
    pub anchor: Rect,
    pub requested_size: Size,
}
impl OverlayLayout {
    pub fn hit_test(self, point: Point) -> bool {
        self.visible.is_some_and(|rect| rect.contains(point))
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipEvent {
    #[default]
    None,
    Dismissed,
}
impl TooltipView {
    // One inset definition for the original fixed-size and fitted help paths.
    const INSET_X: f32 = 12.;
    const INSET_Y: f32 = 8.;
    pub(crate) fn fitted_size(
        text: &mut dyn ButtonTextRenderer,
        model: &Tooltip,
        viewport: Rect,
        role: proposal::TextRole,
    ) -> Size {
        let width = (viewport.size.width - 2. * Self::INSET_X).max(0.);
        let measured = text.measure(&model.text, role, width);
        Size::new(
            (measured.width + 2. * Self::INSET_X).min(viewport.size.width),
            (measured.height + 2. * Self::INSET_Y).min(viewport.size.height),
        )
    }
    pub(crate) fn content_size(viewport: Rect) -> Size {
        Size::new(
            (viewport.size.width - 2. * Self::INSET_X).max(0.),
            (viewport.size.height - 2. * Self::INSET_Y).max(0.),
        )
    }
    pub(crate) fn fitting_lines(viewport: Rect, ends: impl Iterator<Item = f64>) -> usize {
        let available = f64::from(Self::content_size(viewport).height);
        ends.take_while(|end| end.is_finite() && *end <= available)
            .count()
    }
    pub fn pointer_move(&mut self, point: Point) {
        if self.inactive {
            return;
        }
        let inside = self.anchor_visible.is_some_and(|r| r.contains(point))
            || self.layout.visible.is_some_and(|r| r.contains(point));
        // A new active hover is a new request, even if an old focus owner
        // remains. Repeated movement within the same region stays suppressed.
        if (inside && !self.hovered) || (self.hovered && !inside && !self.focused) {
            self.dismissed_until_leave = false;
        }
        self.hovered = inside;
    }
    pub fn set_focus(&mut self, focused: bool, focus_visible: bool) {
        if self.inactive {
            return;
        }
        // A false-to-true focus transition is independent of retained hover;
        // duplicate focus notifications must not undo an Escape dismissal.
        if (focused && !self.focused) || (self.focused && !focused && !self.hovered) {
            self.dismissed_until_leave = false;
        }
        self.focused = focused;
        self.focus_visible = focused && focus_visible;
    }
    pub fn key(&mut self, key: ControlKey) -> TooltipEvent {
        if self.inactive {
            return TooltipEvent::None;
        }
        if key == ControlKey::Escape && (self.hovered || self.focused || self.controlled_visible) {
            // A controlled-only request has no hover/focus owner to leave.
            // Its existing suppression resets when the caller hides it.
            self.dismissed_until_leave = self.hovered || self.focused;
            self.controlled_suppressed = self.controlled_visible;
            TooltipEvent::Dismissed
        } else {
            TooltipEvent::None
        }
    }
    pub fn blur(&mut self) {
        self.focus_visible = false;
        self.focused = false;
        if !self.hovered {
            self.dismissed_until_leave = false;
        }
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.inactive = !active;
        if !active {
            self.blur();
            self.hovered = false;
            self.dismissed_until_leave = false;
            // Availability ends at deactivation, before any later redraw or
            // input. Old anchor/overlay geometry cannot re-arm after restore.
            self.anchor_visible = None;
            self.layout.visible = None;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Tooltip,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> OverlayLayout {
        // Observed caller hide/show is a fresh request, independent of old
        // hover/focus owners. Repeated visible=true remains suppressed.
        if model.visible && !self.controlled_visible {
            self.dismissed_until_leave = false;
        }
        if !model.visible {
            self.controlled_suppressed = false;
        }
        self.controlled_visible = model.visible;
        let showing = !self.inactive
            && ((model.visible && !self.controlled_suppressed) || self.hovered || self.focused);
        let viewport = effective_viewport(viewport, clip);
        let r = place_overlay(anchor, size, viewport, 8.);
        let content = safe_inset(r, Self::INSET_X, Self::INSET_Y);
        self.anchor_visible = clipped(anchor, clip);
        self.layout = OverlayLayout {
            frame: r,
            content,
            visible: (showing && !self.dismissed_until_leave)
                .then(|| clipped(r, Some(viewport)))
                .flatten(),
            anchor,
            requested_size: size,
        };
        if showing
            && !self.dismissed_until_leave
            && let Some(v) = self.layout.visible
        {
            scene.push_clip(v);
            scene.push_quad(
                Quad::new(
                    r,
                    palette
                        .color("surface-3")
                        .unwrap_or_else(|| palette.color("surface-2").expect("surface token")),
                )
                .with_corner_radius(proposal::metric("radius", "radius-floating").unwrap_or(12.)),
            );
            draw_text(
                scene,
                text,
                &model.text,
                role,
                content,
                palette,
                ButtonTextAlign::Leading,
            );
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> OverlayLayout {
        self.layout
    }
}
/// Non-modal popover owns viewport-aware placement, outside dismissal and Escape.
#[derive(Clone, Debug)]
pub struct PopoverView {
    pub(super) surface: Option<&'static str>,
    pub(super) dismiss_visible: bool,
    pub(super) placement: PopoverPlacement,
    pub(super) presence: OverlayPresence,
    pub(super) layout: OverlayLayout,
    pub(super) disabled: bool,
    pub(super) dismiss: ButtonModel,
    pub(super) dismiss_layout: Option<crate::ButtonLayout>,
    pub(super) logical_dismiss_frame: Option<Rect>,
    pub(super) outside_armed: bool,
    pub(super) active: bool,
}
impl Default for PopoverView {
    fn default() -> Self {
        Self {
            surface: None,
            dismiss_visible: true,
            placement: PopoverPlacement::Below,
            presence: OverlayPresence::default(),
            layout: OverlayLayout::default(),
            disabled: false,
            dismiss: ButtonModel::default(),
            dismiss_layout: None,
            logical_dismiss_frame: None,
            outside_armed: false,
            active: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayEvent {
    None,
    Dismissed { restore_focus: Option<usize> },
}
#[derive(Clone, Copy)]
struct PopoverSpacing {
    content_inset_x: f32,
    content_inset_y: f32,
    content_to_action_gap: f32,
    action_inset_x: f32,
    action_width: f32,
}
fn popover_spacing() -> PopoverSpacing {
    let snug = proposal::metric("spacing", "space-snug").unwrap_or(6.);
    let regular = proposal::metric("spacing", "space-regular").unwrap_or(8.);
    let tight = proposal::metric("spacing", "space-tight").unwrap_or(4.);
    PopoverSpacing {
        // Keep the established 14 / 10 / 8 geometry while expressing every
        // inset through the JSON-backed spacing scale.
        content_inset_x: snug + regular,
        content_inset_y: snug + tight,
        content_to_action_gap: regular,
        action_inset_x: snug + tight,
        action_width: 120.,
    }
}
/// Preferred placement in the existing viewport-clamped Popover shell. Right
/// placements flip to the left when it fits, then clamp to the actual viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverPlacement {
    #[default]
    Below,
    RightStart,
    RightCenter,
    RightEnd,
}
impl PopoverView {
    pub fn set_placement(&mut self, placement: PopoverPlacement) {
        self.placement = placement;
    }
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        self.surface = Some(surface);
        self.dismiss.set_surface(surface);
        Ok(())
    }
    /// Forms with original Cancel/Save actions may omit the built-in Dismiss.
    /// Default is true. Hiding it retires its focus/hit geometry and any arm;
    /// outside dismissal, Escape and original content remain unchanged.
    pub fn set_dismiss_visible(&mut self, visible: bool) {
        if self.dismiss_visible != visible {
            self.dismiss_visible = visible;
            self.dismiss.blur();
            self.dismiss_layout = None;
            self.logical_dismiss_frame = None;
        }
    }
    pub fn dismiss_visible(&self) -> bool {
        self.dismiss_visible
    }
    /// Measure this configured shell around original content. The legacy static
    /// `measure` retains its default built-in action for existing consumers.
    pub fn measure_content(
        &self,
        text: &mut impl ButtonTextRenderer,
        content: Size,
        role: proposal::TextRole,
    ) -> Size {
        if self.dismiss_visible {
            Self::measure(text, content, role)
        } else {
            let spacing = popover_spacing();
            Size::new(
                content.width + spacing.content_inset_x * 2.,
                content.height + spacing.content_inset_y * 2.,
            )
        }
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

    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        content: Size,
        role: proposal::TextRole,
    ) -> Size {
        let spacing = popover_spacing();
        let dismiss =
            ButtonModel::measure(text, "Dismiss", spacing.action_width, ButtonSize::Md, role);
        Size::new(
            (content.width + spacing.content_inset_x * 2.)
                .max(spacing.action_width + spacing.action_inset_x * 2.)
                .max(dismiss.width + spacing.action_inset_x * 2.),
            content.height
                + spacing.content_inset_y * 2.
                + spacing.content_to_action_gap
                + dismiss.height,
        )
    }
    /// Arrange the existing popup/content/dismiss geometry without drawing or
    /// sampling its transition. Owned Component children can use these bounds
    /// during their ordinary layout pass, then paint through the content callback.
    #[allow(clippy::too_many_arguments)]
    pub fn arrange(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> OverlayLayout {
        self.arrange_geometry(text, model, anchor, size, viewport, clip, role)
            .0
    }
    #[allow(clippy::too_many_arguments)]
    fn arrange_geometry(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> (OverlayLayout, Rect) {
        let effective = effective_viewport(viewport, clip);
        // Preserve document-space child geometry when a valid ancestor clip is
        // temporarily disjoint (for example, while the open popover is scrolled
        // away). Painting and pointer visibility still use the ancestor clip.
        let viewport = if effective.size.width > 0.0 && effective.size.height > 0.0 {
            effective
        } else if finite_rect(viewport) && viewport.size.width > 0.0 && viewport.size.height > 0.0 {
            viewport
        } else {
            effective
        };
        let frame = place_popover(anchor, size, viewport, 6., self.placement);
        let visible = clipped(frame, clip);
        let spacing = popover_spacing();
        let dismiss_w = spacing
            .action_width
            .min((frame.size.width - spacing.action_inset_x * 2.).max(0.));
        let dismiss_h = if self.dismiss_visible {
            ButtonModel::measure(text, "Dismiss", dismiss_w, ButtonSize::Md, role)
                .height
                .min((frame.size.height - spacing.content_inset_y).max(0.))
        } else {
            0.
        };
        let dismiss_top = frame.max_y() - spacing.content_inset_y - dismiss_h;
        let dismiss_frame = Rect::new(
            frame.min_x() + spacing.action_inset_x,
            dismiss_top.max(frame.min_y()),
            dismiss_w,
            dismiss_h,
        );
        self.logical_dismiss_frame = (self.dismiss_visible
            && model.open
            && self.active
            && !self.disabled
            && valid_logical_target(dismiss_frame, clip))
        .then_some(dismiss_frame);
        let content_bottom = if self.dismiss_visible {
            dismiss_top - spacing.content_to_action_gap
        } else {
            frame.max_y() - spacing.content_inset_y
        };
        let content = Rect::new(
            frame.min_x() + spacing.content_inset_x,
            frame.min_y() + spacing.content_inset_y,
            (frame.size.width - spacing.content_inset_x * 2.).max(0.),
            (content_bottom - frame.min_y() - spacing.content_inset_y).max(0.),
        );
        self.layout = OverlayLayout {
            frame,
            content,
            visible,
            anchor,
            requested_size: size,
        };
        self.dismiss_layout = None;
        (self.layout, dismiss_frame)
    }
    pub fn show(&mut self, model: &mut Popover, invoker: usize) {
        if !self.disabled && self.active {
            model.show(invoker)
        }
    }
    pub fn close(&mut self, model: &mut Popover) -> OverlayEvent {
        self.dismiss.blur();
        self.outside_armed = false;
        self.dismiss_layout = None;
        self.logical_dismiss_frame = None;
        self.layout.visible = None;
        if model.open {
            OverlayEvent::Dismissed {
                restore_focus: model.close(),
            }
        } else {
            OverlayEvent::None
        }
    }
    pub fn set_disabled(&mut self, d: bool, model: &mut Popover) -> OverlayEvent {
        self.disabled = d;
        if d {
            if model.open {
                self.close(model)
            } else {
                self.cancel();
                OverlayEvent::None
            }
        } else {
            OverlayEvent::None
        }
    }
    /// Deactivation closes the transient overlay and clears any held pointer/key arm.
    pub fn set_window_active(&mut self, active: bool, model: &mut Popover) -> OverlayEvent {
        self.active = active;
        if active {
            OverlayEvent::None
        } else {
            self.presence.clear();
            self.close(model)
        }
    }
    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled =
            focused && self.logical_dismiss_frame.is_some() && self.active && !self.disabled;
        self.dismiss.state.focused = enabled;
        self.dismiss.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.dismiss.cancel();
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> OverlayLayout {
        self.render_with_content(
            scene,
            text,
            model,
            anchor,
            size,
            viewport,
            clip,
            palette,
            role,
            |_, _| {},
        )
    }
    /// Render a caller-owned content slot inside the production popover frame.
    /// The callback receives clipped content bounds and cannot change hit geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_content(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        draw_content: impl FnOnce(&mut Scene, Rect),
    ) -> OverlayLayout {
        self.render_with_text_content(
            scene,
            text,
            model,
            anchor,
            size,
            viewport,
            clip,
            palette,
            role,
            "",
            |scene, _, bounds| draw_content(scene, bounds),
        )
    }
    /// Text-aware content slot included in retained enter/exit paint. Change
    /// `content_identity` when caller-owned content or resource identities change.
    #[allow(clippy::too_many_arguments)]
    /// Compatible layout-returning entry. On composition failure the Scene is
    /// explicitly non-submittable; the returned layout is not paint success.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_text_content<T: ButtonTextRenderer>(
        &mut self,
        scene: &mut Scene,
        text: &mut T,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        content_identity: &str,
        draw_content: impl FnOnce(&mut Scene, &mut T, Rect),
    ) -> OverlayLayout {
        match self.try_render_with_text_content(
            scene,
            text,
            model,
            anchor,
            size,
            viewport,
            clip,
            palette,
            role,
            content_identity,
            draw_content,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                scene.record_composition_error(error);
                self.layout
            }
        }
    }
    /// Fallible overlay composition. Rejected paint is never appended in part.
    #[allow(clippy::too_many_arguments)]
    pub fn try_render_with_text_content<T: ButtonTextRenderer>(
        &mut self,
        scene: &mut Scene,
        text: &mut T,
        model: &Popover,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        content_identity: &str,
        draw_content: impl FnOnce(&mut Scene, &mut T, Rect),
    ) -> Result<OverlayLayout, tekes_ui_scene::FragmentError> {
        if let Some(error) = scene.composition_error() {
            return Err(error);
        }
        let paint_context = format!(
            "{anchor:?}{size:?}{viewport:?}{clip:?}{palette:?}{role:?}{content_identity}{}{}{:?}",
            text.backing_scale(),
            self.dismiss_visible,
            (self.surface, self.placement)
        );
        if !model.open && !self.presence.is_presented() {
            self.dismiss.blur();
            self.dismiss_layout = None;
            self.logical_dismiss_frame = None;
            self.layout.visible = None;
            self.outside_armed = false;
            self.presence
                .try_paint(scene, text, false, paint_context, None)?;
            return Ok(self.layout);
        }
        let destination = scene;
        let mut overlay_scene = destination.fragment();
        let scene = &mut overlay_scene;
        let (layout, dismiss_frame) =
            self.arrange_geometry(text, model, anchor, size, viewport, clip, role);
        let OverlayLayout {
            frame,
            content,
            visible,
            ..
        } = layout;
        let viewport = effective_viewport(viewport, clip);
        if (model.open || self.presence.is_presented()) && self.active && !self.disabled {
            if let Some(v) = visible {
                if let Some(surface) = self.surface {
                    let colors = tekes_ui_tokens::surface_palette::resolve_surface(
                        palette,
                        surface,
                        "surface-frame",
                        "rest",
                    )
                    .expect("validated Popover frame");
                    let layout =
                        ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, Some(viewport));
                    if let Some(clip) = layout.background_paint_clip() {
                        scene.push_clip(clip);
                    }
                    paint::surface_kind_with_elevation_colors_at_scale(
                        scene,
                        frame,
                        palette,
                        paint::SurfaceKind::Floating,
                        text.backing_scale(),
                        "popover",
                        colors,
                    );
                    if layout.background_paint_clip().is_some() {
                        scene.pop_clip();
                    }
                } else {
                    paint_surface_background(
                        scene,
                        frame,
                        Some(viewport),
                        visible,
                        palette,
                        paint::SurfaceKind::Floating,
                        text.backing_scale(),
                    );
                }
                scene.push_clip(v);
                draw_content(scene, text, content);
                if self.dismiss_visible {
                    self.dismiss_layout = Some(self.dismiss.render(
                        scene,
                        text,
                        "Dismiss",
                        dismiss_frame,
                        Some(v),
                        palette,
                        paint::ButtonVariant::Quiet,
                        ButtonSize::Md,
                        role,
                        ButtonTextAlign::default(),
                    ));
                }
                scene.pop_clip();
            } else {
                self.dismiss.blur();
                self.outside_armed = false;
            }
        } else {
            self.dismiss.blur();
            self.outside_armed = false;
        }
        self.presence.try_paint(
            destination,
            text,
            model.open,
            paint_context,
            Some(overlay_scene),
        )?;
        if !model.open {
            self.layout.visible = None;
            self.dismiss_layout = None;
            self.dismiss.blur();
        }
        Ok(self.layout)
    }
    pub fn pointer_move(&mut self, point: Point) {
        self.dismiss.pointer_move_at(point);
    }
    pub fn pointer_down(&mut self, model: &mut Popover, p: Point) -> bool {
        if !model.open || self.disabled || !self.active || self.layout.visible.is_none() {
            return false;
        }
        self.cancel();
        if self.dismiss_layout.is_some_and(|l| l.hit_test(p)) {
            self.dismiss.pointer_down_at(p)
        } else if !self.layout.visible.is_some_and(|r| r.contains(p)) {
            self.outside_armed = true;
            true
        } else {
            self.outside_armed = false;
            false
        }
    }
    pub fn pointer_up(&mut self, model: &mut Popover, p: Point) -> OverlayEvent {
        if !model.open || self.disabled || !self.active {
            self.cancel();
            return OverlayEvent::None;
        }
        if self.dismiss.pointer_up_at(p) {
            self.close(model)
        } else if self.outside_armed && !self.layout.visible.is_some_and(|r| r.contains(p)) {
            self.outside_armed = false;
            self.close(model)
        } else {
            self.outside_armed = false;
            OverlayEvent::None
        }
    }
    pub fn key(&mut self, model: &mut Popover, key: ControlKey) -> OverlayEvent {
        self.key_event(model, key, true)
    }
    pub fn key_event(&mut self, model: &mut Popover, key: ControlKey, down: bool) -> OverlayEvent {
        if !self.active || !model.open || self.disabled {
            return OverlayEvent::None;
        }
        if (key == ControlKey::Escape && down)
            || (self.dismiss.state.focused && self.dismiss.key(key, down))
        {
            self.close(model)
        } else {
            OverlayEvent::None
        }
    }
    pub fn cancel(&mut self) {
        self.dismiss.cancel();
        self.outside_armed = false;
    }
    pub fn layout(&self) -> OverlayLayout {
        self.layout
    }
    /// Clipped focus target for the component-owned dismiss action.
    pub fn focus_target(&self) -> Option<Rect> {
        (self.active && self.layout.visible.is_some())
            .then_some(self.dismiss_layout?)
            .and_then(|layout| layout.visible)
    }
    /// Full mounted action geometry in document coordinates, independent of
    /// ancestor clipping. Hosts use this for offscreen keyboard reveal; pointer
    /// hit testing must continue to use `focus_target`/`layout.visible`.
    pub fn logical_focus_target(&self) -> Option<Rect> {
        (self.active && !self.disabled).then_some(self.logical_dismiss_frame?)
    }
}
pub(crate) fn place_overlay(anchor: Rect, size: Size, viewport: Rect, gap: f32) -> Rect {
    if !finite_rect(anchor)
        || !finite_rect(viewport)
        || !size.width.is_finite()
        || !size.height.is_finite()
    {
        return Rect::default();
    }
    let w = if size.width.is_finite() {
        size.width.max(0.).min(viewport.size.width.max(0.))
    } else {
        0.
    };
    let h = if size.height.is_finite() {
        size.height.max(0.).min(viewport.size.height.max(0.))
    } else {
        0.
    };
    let below = anchor.max_y() + gap;
    let above = anchor.min_y() - gap - h;
    let y = if below + h <= viewport.max_y() {
        below
    } else if above >= viewport.min_y() {
        above
    } else {
        below.min((viewport.max_y() - h).max(viewport.min_y()))
    };
    let x = anchor.min_x().clamp(
        viewport.min_x(),
        (viewport.max_x() - w).max(viewport.min_x()),
    );
    let y = y.clamp(
        viewport.min_y(),
        (viewport.max_y() - h).max(viewport.min_y()),
    );
    Rect::new(x, y, w, h)
}

fn place_popover(
    anchor: Rect,
    size: Size,
    viewport: Rect,
    gap: f32,
    placement: PopoverPlacement,
) -> Rect {
    let below = place_overlay(anchor, size, viewport, gap);
    if placement == PopoverPlacement::Below || below.is_empty() {
        return below;
    }
    let right = anchor.max_x() + gap;
    let left = anchor.min_x() - gap - below.size.width;
    let x = if right + below.size.width <= viewport.max_x() {
        right
    } else if left >= viewport.min_x() {
        left
    } else {
        right
    };
    let y = match placement {
        PopoverPlacement::RightStart => anchor.min_y(),
        PopoverPlacement::RightCenter => {
            anchor.min_y() + (anchor.size.height - below.size.height) / 2.
        }
        PopoverPlacement::RightEnd => anchor.max_y() - below.size.height,
        PopoverPlacement::Below => unreachable!(),
    };
    Rect::new(
        x.clamp(
            viewport.min_x(),
            (viewport.max_x() - below.size.width).max(viewport.min_x()),
        ),
        y.clamp(
            viewport.min_y(),
            (viewport.max_y() - below.size.height).max(viewport.min_y()),
        ),
        below.size.width,
        below.size.height,
    )
}
