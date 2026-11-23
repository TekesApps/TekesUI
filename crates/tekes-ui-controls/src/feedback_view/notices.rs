use super::{
    ViewLayout, clipped, notice_marker_frame, paint_notice_background, safe_inset,
    valid_logical_target,
};
use crate::extended::{Banner, NoticeTone, Toast};
use crate::{ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, paint};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

/// Toast rendering keeps duration progression deterministic in its model.
#[derive(Clone, Copy, Debug)]
struct NoticePrepared {
    message: Rect,
    marker: Rect,
    role: proposal::TextRole,
    clip: Option<Rect>,
}
fn notice_usage(tone: NoticeTone, toast: bool) -> &'static str {
    match tone {
        NoticeTone::Info => "notice-marker-info",
        NoticeTone::Success => "notice-marker-success",
        NoticeTone::Error => "notice-marker-error",
        NoticeTone::Warning if toast => "notice-marker-toast-warning",
        NoticeTone::Warning => "notice-marker-banner-warning",
    }
}
const NOTICE_STACK_WIDTH: f32 = 280.0;

fn notice_marker_diameter() -> f32 {
    proposal::metric("size", "size-dot").expect("notice marker dot size token")
}

fn notice_min_row_height() -> f32 {
    proposal::metric("size", "size-row-height").expect("notice minimum row height token")
}

fn notice_dismiss_metrics(
    text: &mut impl ButtonTextRenderer,
    available_width: f32,
    role: proposal::TextRole,
    min_row: f32,
) -> (f32, f32) {
    let width = ButtonModel::measure(text, "Dismiss", f32::MAX, ButtonSize::Body, role)
        .width
        .min(available_width);
    let height = ButtonModel::measure(text, "Dismiss", width, ButtonSize::Body, role)
        .height
        .max(min_row);
    (width, height)
}

#[derive(Clone, Debug, Default)]
pub struct ToastView {
    pub(super) layout: ViewLayout,
    pub(super) dismiss: ButtonModel,
    pub(super) dismiss_layout: Option<crate::ButtonLayout>,
    exit_dismiss_layout: Option<crate::ButtonLayout>,
    pub(super) logical_dismiss_frame: Option<Rect>,
    pub(super) inactive: bool,
    pub(super) surface: Option<&'static str>,
    prepared: Option<NoticePrepared>,
    presence: crate::control_motion::OverlayPresence,
}
impl ToastView {
    fn stacked(frame: Rect) -> bool {
        frame.size.width < NOTICE_STACK_WIDTH
    }
    /// Measures the toast's content and dismiss action using the same text backend
    /// and width constraint used by `render`. Hosts can size the outer frame before
    /// drawing so large text does not clip the action or message.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        message: &str,
        width: f32,
        role: proposal::TextRole,
    ) -> Size {
        let width = width.max(0.0);
        let stacked = width < NOTICE_STACK_WIDTH;
        let pad = tekes_ui_tokens::Spacing::Relaxed.value();
        let gap = tekes_ui_tokens::Spacing::Regular.value();
        let marker_diameter = notice_marker_diameter();
        let min_row = notice_min_row_height();
        let leading_inset = pad + marker_diameter + gap;
        let available_action_width = if stacked {
            (width - pad * 2.0).max(0.0)
        } else {
            (width - leading_inset - gap - pad).max(0.0)
        };
        let (action_width, action_height) =
            notice_dismiss_metrics(text, available_action_width, role, min_row);
        let content_width = if stacked {
            (width - leading_inset - pad).max(0.0)
        } else {
            (width - leading_inset - gap - action_width - pad).max(0.0)
        };
        let content_height = text
            .measure(message, role, content_width)
            .height
            .max(min_row);
        let height = if stacked {
            pad + content_height + gap + action_height + pad
        } else {
            pad + content_height.max(action_height) + pad
        };
        Size::new(width, height)
    }

    pub fn prepare(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Toast,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        let stacked = Self::stacked(frame);
        let pad = tekes_ui_tokens::Spacing::Relaxed.value();
        let gap = tekes_ui_tokens::Spacing::Regular.value();
        let marker_diameter = notice_marker_diameter();
        let min_row = notice_min_row_height();
        let leading_inset = pad + marker_diameter + gap;
        let available_action_width = if stacked {
            (frame.size.width - pad * 2.0).max(0.0)
        } else {
            (frame.size.width - leading_inset - gap - pad).max(0.0)
        };
        let (dismiss_w, raw_dismiss_h) =
            notice_dismiss_metrics(text, available_action_width, role, min_row);
        let dismiss_h = raw_dismiss_h.min(frame.size.height.max(0.));
        let dismiss_x = if stacked {
            frame.min_x() + (frame.size.width - dismiss_w) * 0.5
        } else {
            frame.min_x() + (frame.size.width - pad - dismiss_w).max(0.0)
        };
        let content_width = if stacked {
            (frame.size.width - leading_inset - pad).max(0.0)
        } else {
            (dismiss_x - gap - (frame.min_x() + leading_inset)).max(0.0)
        };
        let content_height = text
            .measure(&model.message, role, content_width)
            .height
            .max(min_row);
        self.layout = ViewLayout {
            frame,
            content: Rect::new(
                frame.min_x() + leading_inset,
                frame.min_y() + pad,
                content_width,
                content_height,
            ),
            visible: model.visible.then_some(visible).flatten(),
        };
        self.dismiss_layout = None;
        let dismiss_y = if stacked {
            self.layout.content.max_y() + gap
        } else {
            frame.min_y() + (frame.size.height - dismiss_h).max(0.0) * 0.5
        };
        let logical_dismiss_frame = Rect::new(dismiss_x, dismiss_y, dismiss_w, dismiss_h);
        self.logical_dismiss_frame =
            (model.visible && !self.inactive && valid_logical_target(logical_dismiss_frame, clip))
                .then_some(logical_dismiss_frame);

        let message = self.layout.content;
        let marker = notice_marker_frame(
            text,
            &model.message,
            role,
            message,
            frame.min_x() + pad,
            marker_diameter,
        );
        let h = text
            .measure(&model.message, role, message.size.width)
            .height
            .max(0.)
            .min(message.size.height);
        let draw = Rect::new(message.min_x(), message.min_y(), message.size.width, h);
        self.prepared = Some(NoticePrepared {
            message: draw,
            marker,
            role,
            clip,
        });
        self.dismiss_layout = if model.visible {
            visible.map(|v| {
                self.dismiss
                    .prepare_label(text, "Dismiss", logical_dismiss_frame, Some(v), role)
            })
        } else {
            None
        };
        if model.visible {
            self.exit_dismiss_layout = self.dismiss_layout;
        } else if !self.presence.is_presented() {
            self.exit_dismiss_layout = None;
        }
        if self.dismiss_layout.is_none() {
            self.dismiss.clear_layout();
        }
        self.layout
    }
    pub fn is_presented(&self) -> bool {
        self.presence.is_presented()
    }
    pub fn needs_animation(&self) -> bool {
        self.presence.needs_animation()
    }
    pub fn clear_presentation(&mut self) {
        self.presence.clear();
        self.exit_dismiss_layout = None;
    }
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn invalidate_geometry(&mut self) {
        self.prepared = None;
        self.dismiss_layout = None;
        self.logical_dismiss_frame = None;
        self.dismiss.clear_layout();
    }
    pub fn activate_dismiss(&mut self, model: &mut Toast) -> bool {
        if !model.visible || self.focus_target().is_none() {
            return false;
        }
        self.dismiss.pointer_down();
        let activated = self.dismiss.pointer_up(true);
        if activated {
            model.dismiss();
            self.dismiss.blur();
        }
        activated
    }
    pub fn pointer_move_button(&mut self, p: Point) {
        self.dismiss.pointer_move_at(p);
    }
    pub fn paint_prepared(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Toast,
        palette: Palette,
    ) {
        let open = model.visible && !self.inactive;
        if !open && !self.presence.is_presented() {
            self.exit_dismiss_layout = None;
            return;
        }
        let surface = self.surface.unwrap_or("panel");
        let paint_context = format!(
            "{:?}{:?}{:?}{:?}{}{:?}{}",
            self.layout.frame,
            self.prepared.and_then(|p| p.clip),
            palette,
            surface,
            text.backing_scale(),
            model.tone,
            model.message
        );
        let paint_frame = if open {
            match (self.prepared, self.layout.visible) {
                (Some(p), Some(v)) => Some((p, v, self.dismiss_layout)),
                _ => None,
            }
        } else if let Some(p) = self.prepared
            && let Some(v) = clipped(self.layout.frame, p.clip)
        {
            Some((p, v, self.exit_dismiss_layout))
        } else {
            None
        };
        let overlay = paint_frame.map(|(p, v, dismiss_layout)| {
            let mut overlay_scene = scene.fragment();
            let usage = notice_usage(model.tone, true);
            let marker =
                tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, usage, "rest")
                    .expect("validated notice marker packet");
            let body = tekes_ui_tokens::surface_palette::resolve_surface(
                palette, surface, "content", "rest",
            )
            .expect("validated notice content packet");
            let frame_colors = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                surface,
                "surface-frame",
                "rest",
            )
            .expect("validated notice frame packet");
            self.dismiss.set_surface(surface);
            let _action =
                self.dismiss
                    .surface_colors(palette, paint::ButtonVariant::Quiet, surface);
            paint_notice_background(
                &mut overlay_scene,
                self.layout.frame,
                p.clip,
                Some(v),
                palette,
                paint::SurfaceKind::Floating,
                text.backing_scale(),
                frame_colors,
            );
            overlay_scene.push_clip(v);
            overlay_scene.push_quad(
                Quad::new(p.marker, marker.marker.expect("notice marker"))
                    .with_corner_radius(p.marker.size.height * 0.5),
            );
            if !p.message.is_empty() && !model.message.is_empty() {
                text.draw(
                    &mut overlay_scene,
                    &model.message,
                    p.role,
                    p.message,
                    ButtonTextAlign::Leading,
                    body.text,
                );
            }
            if let Some(layout) = dismiss_layout {
                self.dismiss.paint_prepared_label_variant(
                    &mut overlay_scene,
                    text,
                    "Dismiss",
                    layout,
                    Some(v),
                    palette,
                    p.role,
                    paint::ButtonVariant::Quiet,
                );
            }
            overlay_scene.pop_clip();
            overlay_scene
        });
        if open && overlay.is_none() {
            return;
        }
        self.presence
            .paint(scene, text, open, paint_context, overlay);
        if !open && !self.presence.is_presented() {
            self.exit_dismiss_layout = None;
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Toast,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.prepare(text, model, frame, clip, role);
        self.paint_prepared(scene, text, model, palette);
        self.layout
    }
    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled = focused && self.logical_dismiss_frame.is_some() && !self.inactive;
        self.dismiss.state.focused = enabled;
        self.dismiss.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.dismiss.cancel();
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        !self.inactive
            && self.dismiss_layout.is_some_and(|l| l.hit_test(p))
            && self.dismiss.pointer_down_at(p)
    }
    pub fn pointer_up(&mut self, model: &mut Toast, p: Point) -> bool {
        // Always deliver pointer-up to the child so it can release a press
        // that began on the button and ended outside it.
        let activated = self.dismiss.pointer_up_at(p);
        if self.inactive || !model.visible {
            self.dismiss.blur();
            return false;
        }
        if activated && self.dismiss_layout.is_some_and(|l| l.hit_test(p)) {
            model.dismiss();
            self.dismiss.blur();
            true
        } else {
            false
        }
    }
    pub fn key(&mut self, model: &mut Toast, key: ControlKey, down: bool) -> bool {
        if !self.inactive
            && model.visible
            && self.dismiss.state.focused
            && self.dismiss.key(key, down)
        {
            model.dismiss();
            self.dismiss.blur();
            true
        } else {
            false
        }
    }
    pub fn cancel(&mut self) {
        self.dismiss.cancel();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.inactive = !active;
        if !active {
            self.cancel();
            self.dismiss.blur();
            self.clear_presentation();
        }
    }
    pub fn pointer_move(&mut self, model: &mut Toast, p: Point) {
        model.paused = model.visible && self.layout.hit_test(p);
        self.dismiss.pointer_move_at(p);
    }
    pub fn advance(&mut self, model: &mut Toast, elapsed_ms: u32) -> bool {
        model.advance(elapsed_ms)
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
    /// Clipped focus target for the visible Dismiss button.
    pub fn focus_target(&self) -> Option<Rect> {
        (!self.inactive)
            .then_some(self.dismiss_layout?)
            .and_then(|layout| layout.visible)
    }
    /// Full mounted Dismiss action frame, even when its ancestor clip is
    /// currently disjoint. Pointer handling still uses the clipped target.
    pub fn logical_focus_target(&self) -> Option<Rect> {
        (!self.inactive).then_some(self.logical_dismiss_frame?)
    }
}
/// Banner presentation owns its message and dismiss hit target.
#[derive(Clone, Debug, Default)]
pub struct BannerView {
    pub(super) layout: ViewLayout,
    pub(super) dismiss_button: ButtonModel,
    pub(super) dismiss_layout: Option<crate::ButtonLayout>,
    pub(super) logical_dismiss_frame: Option<Rect>,
    pub(super) inactive: bool,
    pub(super) surface: Option<&'static str>,
    prepared: Option<NoticePrepared>,
}
impl BannerView {
    fn stacked(width: f32, dismissible: bool) -> bool {
        dismissible && width < NOTICE_STACK_WIDTH
    }

    /// Returns the banner's natural frame size for the supplied message and
    /// width. Narrow dismissible banners place the action after the wrapped
    /// message so the two never compete for the same horizontal slot.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        message: &str,
        width: f32,
        dismissible: bool,
        role: proposal::TextRole,
    ) -> Size {
        let width = width.max(0.0);
        let pad = tekes_ui_tokens::Spacing::Relaxed.value();
        let gap = tekes_ui_tokens::Spacing::Regular.value();
        let marker_diameter = notice_marker_diameter();
        let min_row = notice_min_row_height();
        let leading_inset = pad + marker_diameter + gap;
        if Self::stacked(width, dismissible) {
            let message_width = (width - leading_inset - pad).max(0.0);
            let message_height = text
                .measure(message, role, message_width)
                .height
                .max(min_row);
            let available_action_width = (width - pad * 2.0).max(0.0);
            let (_, action_height) =
                notice_dismiss_metrics(text, available_action_width, role, min_row);
            Size::new(width, pad + message_height + gap + action_height + pad)
        } else {
            let action_size = if dismissible {
                let available = (width - leading_inset - gap - pad).max(0.0);
                let (action_w, action_h) = notice_dismiss_metrics(text, available, role, min_row);
                Size::new(action_w, action_h)
            } else {
                Size::default()
            };
            let message_width = if dismissible {
                (width - leading_inset - gap - action_size.width - pad).max(0.0)
            } else {
                (width - leading_inset - pad).max(0.0)
            };
            let message_height = text
                .measure(message, role, message_width)
                .height
                .max(min_row);
            Size::new(width, pad + message_height.max(action_size.height) + pad)
        }
    }

    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled = focused && self.logical_dismiss_frame.is_some() && !self.inactive;
        self.dismiss_button.state.focused = enabled;
        self.dismiss_button.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.dismiss_button.cancel();
        }
    }
    pub fn prepare(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Banner,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        let pad = tekes_ui_tokens::Spacing::Relaxed.value();
        let gap = tekes_ui_tokens::Spacing::Regular.value();
        let marker_diameter = notice_marker_diameter();
        let min_row = notice_min_row_height();
        let leading_inset = pad + marker_diameter + gap;
        self.layout = ViewLayout {
            frame,
            content: safe_inset(frame, pad, pad),
            visible: model.visible.then_some(visible).flatten(),
        };
        let stacked = Self::stacked(frame.size.width, model.dismissible);
        let available_action_width = if stacked {
            (frame.size.width - pad * 2.0).max(0.0)
        } else {
            (frame.size.width - leading_inset - gap - pad).max(0.0)
        };
        let (dismiss_w, dismiss_h) = if model.dismissible {
            let (w, h) = notice_dismiss_metrics(text, available_action_width, role, min_row);
            (w, h.min(frame.size.height.max(0.0)))
        } else {
            (0.0, 0.0)
        };
        let dismiss_x = if stacked {
            frame.min_x() + (frame.size.width - dismiss_w) * 0.5
        } else {
            frame.min_x() + (frame.size.width - pad - dismiss_w).max(0.0)
        };
        let msg_width = if stacked {
            (frame.size.width - leading_inset - pad).max(0.0)
        } else if model.dismissible {
            (dismiss_x - gap - (frame.min_x() + leading_inset)).max(0.0)
        } else {
            (frame.size.width - leading_inset - pad).max(0.0)
        };
        let height = text
            .measure(&model.message, role, msg_width)
            .height
            .max(min_row)
            .min((frame.size.height - pad * 2.0).max(0.0));
        let message = Rect::new(
            frame.min_x() + leading_inset,
            frame.min_y() + pad,
            msg_width,
            height,
        );
        let dismiss_y = if stacked {
            message.max_y() + gap
        } else {
            frame.min_y() + (frame.size.height - dismiss_h).max(0.0) * 0.5
        };
        let logical_dismiss_frame = Rect::new(dismiss_x, dismiss_y, dismiss_w, dismiss_h);
        self.logical_dismiss_frame = (model.visible
            && model.dismissible
            && !self.inactive
            && valid_logical_target(logical_dismiss_frame, clip))
        .then_some(logical_dismiss_frame);
        self.dismiss_layout = None;
        let marker = notice_marker_frame(
            text,
            &model.message,
            role,
            message,
            frame.min_x() + pad,
            marker_diameter,
        );
        let h = text
            .measure(&model.message, role, message.size.width)
            .height
            .max(0.)
            .min(message.size.height);
        let draw = Rect::new(message.min_x(), message.min_y(), message.size.width, h);
        self.prepared = Some(NoticePrepared {
            message: draw,
            marker,
            role,
            clip,
        });
        self.dismiss_layout = if model.visible && model.dismissible {
            visible.map(|v| {
                self.dismiss_button.prepare_label(
                    text,
                    "Dismiss",
                    logical_dismiss_frame,
                    Some(v),
                    role,
                )
            })
        } else {
            None
        };
        if self.dismiss_layout.is_none() {
            self.dismiss_button.clear_layout();
        }
        self.layout
    }
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn invalidate_geometry(&mut self) {
        self.prepared = None;
        self.dismiss_layout = None;
        self.logical_dismiss_frame = None;
        self.dismiss_button.clear_layout();
    }
    pub fn activate_dismiss(&mut self, model: &mut Banner) -> bool {
        if !model.visible || !model.dismissible || self.focus_target().is_none() {
            return false;
        }
        self.dismiss_button.pointer_down();
        let activated = self.dismiss_button.pointer_up(true);
        if activated {
            model.dismiss();
            self.dismiss_button.blur();
        }
        activated
    }
    pub fn pointer_move_button(&mut self, p: Point) {
        self.dismiss_button.pointer_move_at(p);
    }
    pub fn paint_prepared(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Banner,
        palette: Palette,
    ) {
        if !model.visible {
            return;
        }
        let (Some(p), Some(v)) = (self.prepared, self.layout.visible) else {
            return;
        };
        let surface = self.surface.unwrap_or("panel");
        let usage = notice_usage(model.tone, false);
        let marker =
            tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, usage, "rest")
                .expect("validated notice marker packet");
        let body =
            tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, "content", "rest")
                .expect("validated notice content packet");
        let frame_colors = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            surface,
            "surface-frame",
            "rest",
        )
        .expect("validated notice frame packet");
        self.dismiss_button.set_surface(surface);
        let _action =
            self.dismiss_button
                .surface_colors(palette, paint::ButtonVariant::Quiet, surface);
        paint_notice_background(
            scene,
            self.layout.frame,
            p.clip,
            Some(v),
            palette,
            paint::SurfaceKind::Panel,
            text.backing_scale(),
            frame_colors,
        );
        scene.push_clip(v);
        scene.push_quad(
            Quad::new(p.marker, marker.marker.expect("notice marker"))
                .with_corner_radius(p.marker.size.height * 0.5),
        );
        if !p.message.is_empty() && !model.message.is_empty() {
            text.draw(
                scene,
                &model.message,
                p.role,
                p.message,
                ButtonTextAlign::Leading,
                body.text,
            );
        }
        if let Some(layout) = self.dismiss_layout {
            self.dismiss_button.paint_prepared_label_variant(
                scene,
                text,
                "Dismiss",
                layout,
                Some(v),
                palette,
                p.role,
                paint::ButtonVariant::Quiet,
            );
        }
        scene.pop_clip();
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Banner,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.prepare(text, model, frame, clip, role);
        self.paint_prepared(scene, text, model, palette);
        self.layout
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.dismiss_button.pointer_move_at(p);
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        !self.inactive
            && self.dismiss_layout.is_some_and(|l| l.hit_test(p))
            && self.dismiss_button.pointer_down_at(p)
    }
    pub fn pointer_up(&mut self, model: &mut Banner, p: Point) -> bool {
        // Release the owned button even when the pointer left its bounds;
        // otherwise its pressed state can stick and activate on a later up.
        let activated = self.dismiss_button.pointer_up_at(p);
        if self.inactive || !model.visible || !model.dismissible {
            self.dismiss_button.blur();
            return false;
        }
        let dismissed =
            activated && self.dismiss_layout.is_some_and(|l| l.hit_test(p)) && model.dismiss();
        if dismissed {
            self.dismiss_button.blur();
        }
        dismissed
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.inactive = !active;
        if !active {
            self.cancel();
            self.dismiss_button.blur();
        }
    }
    pub fn key(&mut self, model: &mut Banner, key: ControlKey, down: bool) -> bool {
        let dismissed = !self.inactive
            && model.visible
            && model.dismissible
            && self.dismiss_button.key(key, down)
            && model.dismiss();
        if dismissed {
            self.dismiss_button.blur();
        }
        dismissed
    }
    pub fn cancel(&mut self) {
        self.dismiss_button.cancel();
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
    /// Clipped focus target for the visible Dismiss button.
    pub fn focus_target(&self) -> Option<Rect> {
        (!self.inactive)
            .then_some(self.dismiss_layout?)
            .and_then(|layout| layout.visible)
    }
    /// Full mounted Dismiss action frame in document coordinates. This stays
    /// available for keyboard reveal while pointer visibility remains clipped.
    pub fn logical_focus_target(&self) -> Option<Rect> {
        (!self.inactive).then_some(self.logical_dismiss_frame?)
    }
}
