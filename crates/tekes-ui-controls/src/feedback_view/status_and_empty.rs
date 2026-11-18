use super::{ViewLayout, clipped, finite_rect, valid_logical_target};
use crate::extended::{EmptyState, Progress, StatusBadge};
use crate::{ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, paint};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Copy, Debug, Default)]
pub struct StatusBadgeView {
    layout: ViewLayout,
    surface: Option<&'static str>,
}
impl StatusBadgeView {
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn surface(&self) -> &'static str {
        self.surface.unwrap_or("canvas")
    }
    /// UiCaption uses the caller's original text service; there is no intrinsic
    /// badge height/padding. The parent owns the finite allocation.
    pub fn measure_label(
        text: &mut dyn ButtonTextRenderer,
        label: &str,
        available: Size,
    ) -> Result<Size, tekes_ui_tokens::surface_palette::SurfaceError> {
        if !finite_rect(Rect::new(0., 0., available.width, available.height)) {
            return Err(tekes_ui_tokens::surface_palette::SurfaceError::UnknownCombination);
        }
        let size = text.measure(label, proposal::TextRole::UiCaption, available.width);
        Ok(Size::new(
            size.width.min(available.width).max(0.),
            size.height.min(available.height).max(0.),
        ))
    }
    /// Both original View and Content delegate text geometry to this entry.
    /// label_frame is an explicit host allocation (Gallery retains its inset).
    #[allow(clippy::too_many_arguments)]
    pub fn render_labelled(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        model: StatusBadge,
        label: &str,
        frame: Rect,
        label_frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> Result<ViewLayout, tekes_ui_tokens::surface_palette::SurfaceError> {
        self.render_labelled_aligned(
            scene,
            text,
            model,
            label,
            frame,
            label_frame,
            clip,
            palette,
            ButtonTextAlign::Center,
        )
    }
    /// Explicit existing text alignment for original hosts such as Gallery;
    /// this does not add padding, intrinsic height, or a separate text layout.
    #[allow(clippy::too_many_arguments)]
    pub fn render_labelled_aligned(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        model: StatusBadge,
        label: &str,
        frame: Rect,
        label_frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        align: ButtonTextAlign,
    ) -> Result<ViewLayout, tekes_ui_tokens::surface_palette::SurfaceError> {
        if !finite_rect(frame) || !finite_rect(label_frame) || clip.is_some_and(|r| !finite_rect(r))
        {
            return Err(tekes_ui_tokens::surface_palette::SurfaceError::UnknownCombination);
        }
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            self.surface(),
            crate::extended::status_badge_usage(model),
            "rest",
        )?;
        let layout = ViewLayout {
            frame,
            content: label_frame,
            visible: clipped(frame, clip),
        };
        // Validate the entire request before committing retained layout or Scene.
        Self::measure_label(text, label, label_frame.size)?;
        if let Some(visible) = layout.visible {
            scene.push_clip(visible);
            crate::extended::status_badge_on_surface(scene, frame, palette, model, self.surface())?;
            if !label.is_empty() && !label_frame.is_empty() {
                text.draw(
                    scene,
                    label,
                    proposal::TextRole::UiCaption,
                    label_frame,
                    align,
                    colors.text,
                );
            }
            scene.pop_clip();
        }
        self.layout = layout;
        Ok(layout)
    }

    /// Foreground ink for labels placed over the solid status badge surface.
    /// This neutral semantic role is intentionally independent of brand action colors.
    pub fn label_color(palette: Palette) -> tekes_ui_tokens::Rgba {
        palette
            .color("c-status-badge-foreground")
            .expect("Design System status badge foreground token")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        model: StatusBadge,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        self.layout = ViewLayout {
            frame,
            content: frame,
            visible,
        };
        if let Some(v) = visible {
            scene.push_clip(v);
            crate::extended::status_badge_on_surface(scene, frame, palette, model, self.surface())
                .expect("validated StatusBadge Surface");
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressView {
    layout: ViewLayout,
    surface: Option<&'static str>,
}
impl ProgressView {
    /// Reject unknown context before replacing the retained Surface.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn surface(&self) -> &'static str {
        self.surface.unwrap_or("canvas")
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        model: Progress,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        self.layout = ViewLayout {
            frame,
            content: frame,
            visible,
        };
        if let Some(v) = visible {
            scene.push_clip(v);
            crate::extended::progress_on_surface(scene, frame, palette, model, self.surface())
                .expect("generated Progress Surface packet");
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
}
#[derive(Clone, Debug, Default)]
pub struct EmptyStateView {
    pub(super) layout: ViewLayout,
    pub(super) action: ButtonModel,
    pub(super) action_layout: Option<crate::ButtonLayout>,
    pub(super) logical_action_frame: Option<Rect>,
    pub(super) disabled: bool,
    pub(super) inactive: bool,
    pub(super) surface: Option<&'static str>,
    prepared: Option<EmptyStatePrepared>,
    pub(super) max_detail_width: Option<f32>,
}
/// Geometry owned by the original View; only prepare can replace it.
#[derive(Clone, Copy, Debug)]
struct EmptyStatePrepared {
    title: Rect,
    detail: Rect,
    title_draw: Rect,
    detail_draw: Rect,
    role: proposal::TextRole,
}
impl EmptyStateView {
    pub fn with_max_detail_width(mut self, max_width: Option<f32>) -> Self {
        self.max_detail_width = max_width;
        self
    }

    pub fn set_max_detail_width(&mut self, max_width: Option<f32>) {
        self.max_detail_width = max_width;
    }

    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn cancel_action(&mut self) {
        self.action.cancel();
    }
    pub fn invalidate_body_geometry(&mut self) {
        self.prepared = None;
    }
    pub fn invalidate_geometry(&mut self) {
        self.invalidate_body_geometry();
        self.action_layout = None;
        self.logical_action_frame = None;
        self.action.clear_layout();
    }
    pub fn activate_action(&mut self) -> bool {
        if self.focus_target().is_none() {
            return false;
        }
        self.action.pointer_down();
        self.action.pointer_up(true)
    }

    /// Natural content-slot size, including wrapped title, detail and action.
    /// Parent surface padding is separate. Supply the same width and text
    /// renderer to `render`; a deliberately smaller frame still clips content.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        width: f32,
        role: proposal::TextRole,
    ) -> Size {
        Self::measure_with_max_detail_width(text, model, width, role, None)
    }

    pub fn measure_with_max_detail_width(
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        width: f32,
        role: proposal::TextRole,
        max_detail_width: Option<f32>,
    ) -> Size {
        let (title, detail, action) =
            Self::measured_content_with_limit(text, model, width, role, max_detail_width);
        Size::new(
            width.max(0.),
            action
                .map_or(detail.max_y(), |r| r.max_y())
                .max(title.max_y()),
        )
    }

    fn measured_content_with_limit(
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        width: f32,
        role: proposal::TextRole,
        max_detail_width: Option<f32>,
    ) -> (Rect, Rect, Option<Rect>) {
        let content_width = (width - 8.).max(0.);
        let title_width = content_width.min(420.);
        let title_h = text
            .measure(&model.title, proposal::TextRole::UiHeading, title_width)
            .height
            .max(34.);
        let title = Rect::new(4., 6., title_width, title_h);
        let detail_y = 44f32.max(title.max_y() + 4.);
        let detail_width = max_detail_width
            .filter(|w| w.is_finite())
            .map_or(content_width, |w| content_width.min(w.max(0.)));
        let detail_h = text
            .measure(&model.detail, role, detail_width)
            .height
            .max(54.);
        let detail = Rect::new(4., detail_y, detail_width, detail_h);
        let action = model.action.as_ref().map(|label| {
            let width = content_width.min(150.);
            let height = ButtonModel::measure(text, label, width, ButtonSize::Body, role)
                .height
                .max(34.);
            Rect::new(4., detail.max_y().max(100.), width, height)
        });
        (title, detail, action)
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.action.cancel();
            self.action.state.focused = false;
            self.action.state.focus_visible = false;
        }
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.inactive = !active;
        if !active {
            self.action.blur();
        }
    }
    pub fn focus_action(&mut self, focused: bool, focus_visible: bool) {
        let enabled =
            focused && !self.disabled && !self.inactive && self.logical_action_frame.is_some();
        self.action.state.focused = enabled;
        self.action.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.action.cancel();
        }
    }
    pub fn prepare(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        // Keep the Gallery's established left-aligned empty-state composition.
        // `frame` is the padded content slot inside its parent surface.
        let x = frame.min_x() + 4.;
        let title_y = frame.min_y() + 6.;
        let content_width = (frame.size.width - 8.).max(0.);
        let (title, detail, action) = Self::measured_content_with_limit(
            text,
            model,
            frame.size.width,
            role,
            self.max_detail_width,
        );
        let title_width = title.size.width;
        let title_h = title.size.height;
        let detail_y = frame.min_y() + detail.min_y();
        let detail_h = detail.size.height;
        let action_w = action.map_or(0., |r| r.size.width);
        let action_h = action.map_or(0., |r| r.size.height);
        let action_y = if let Some(action) = action {
            let desired_y = frame.min_y() + action.min_y();
            let last_fitting_y = frame.max_y() - action_h;
            if last_fitting_y >= detail_y + detail_h {
                desired_y.min(last_fitting_y)
            } else {
                // Explicitly constrained hosts may clip, without overlapping text.
                desired_y
            }
        } else {
            detail_y + detail_h
        };
        let total_h = action_y + action_h - title_y;
        let content = Rect::new(x, title_y, content_width, total_h.max(0.));
        self.layout = ViewLayout {
            frame,
            content,
            visible,
        };
        let logical_action_frame = Rect::new(x, action_y, action_w, action_h);
        self.logical_action_frame = model.action.as_ref().and_then(|_| {
            (!self.disabled && !self.inactive && valid_logical_target(logical_action_frame, clip))
                .then_some(logical_action_frame)
        });
        let title_frame = Rect::new(x, title_y, title_width, title_h);
        let detail_frame = Rect::new(x, detail_y, detail.size.width, detail_h);
        let title_draw_h = text
            .measure(&model.title, proposal::TextRole::UiHeading, title_width)
            .height
            .max(0.)
            .min(title_h);
        let detail_draw_h = text
            .measure(&model.detail, role, detail_frame.size.width)
            .height
            .max(0.)
            .min(detail_h);
        self.prepared = Some(EmptyStatePrepared {
            title: title_frame,
            detail: detail_frame,
            title_draw: Rect::new(x, title_y, title_width, title_draw_h),
            detail_draw: Rect::new(x, detail_y, detail_frame.size.width, detail_draw_h),
            role,
        });
        self.action.state.disabled = self.disabled || self.inactive;
        self.action_layout = match (&model.action, visible) {
            (Some(label), Some(v)) => {
                Some(
                    self.action
                        .prepare_label(text, label, logical_action_frame, Some(v), role),
                )
            }
            _ => {
                self.action.clear_layout();
                self.action.blur();
                None
            }
        };
        self.layout
    }
    /// Consume the arranged geometry. No measure call or hit-frame mutation.
    pub fn paint_prepared(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        palette: Palette,
    ) {
        let Some(prepared) = self.prepared else {
            return;
        };
        let Some(visible) = self.layout.visible else {
            return;
        };
        let surface = self.surface.unwrap_or("canvas");
        let body =
            tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, "content", "rest")
                .expect("generated EmptyState content packet");
        // Resolve both complete packets before adding any ink or clip.
        self.action.set_surface(surface);
        let _action_packet =
            self.action
                .surface_colors(palette, paint::ButtonVariant::Primary, surface);
        scene.push_clip(visible);
        if !model.title.is_empty() && !prepared.title.is_empty() {
            text.draw(
                scene,
                &model.title,
                proposal::TextRole::UiHeading,
                prepared.title_draw,
                ButtonTextAlign::Leading,
                body.text,
            );
        }
        if !model.detail.is_empty() && !prepared.detail.is_empty() {
            text.draw(
                scene,
                &model.detail,
                prepared.role,
                prepared.detail_draw,
                ButtonTextAlign::Leading,
                body.text,
            );
        }
        if let (Some(label), Some(layout)) = (&model.action, self.action_layout) {
            self.action.paint_prepared_label(
                scene,
                text,
                label,
                layout,
                self.layout.visible,
                palette,
                prepared.role,
            );
        }
        scene.pop_clip();
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &EmptyState,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.prepare(text, model, frame, clip, role);
        self.paint_prepared(scene, text, model, palette);
        self.layout
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        !self.disabled
            && !self.inactive
            && self.action_layout.is_some_and(|layout| layout.hit_test(p))
            && self.action.pointer_down_at(p)
    }
    pub fn pointer_move(&mut self, p: Point) {
        if self.disabled
            || self.inactive
            || self
                .action_layout
                .is_none_or(|layout| layout.visible.is_none())
        {
            self.action.blur();
            return;
        }
        self.action.pointer_move_at(p);
    }
    pub fn pointer_up(&mut self, p: Point) -> bool {
        // Let the child clear its pointer arm for releases outside the action.
        let activated = self.action.pointer_up_at(p);
        if self.disabled || self.inactive {
            self.action.blur();
            return false;
        }
        activated && self.action_layout.is_some_and(|l| l.hit_test(p))
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        !self.disabled && !self.inactive && self.action.key(key, down)
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
    /// Clipped focus target for the visible EmptyState action.
    pub fn focus_target(&self) -> Option<Rect> {
        (!self.disabled && !self.inactive)
            .then_some(self.action_layout?)
            .and_then(|layout| layout.visible)
    }
    /// Full mounted action frame for keyboard reveal; use `focus_target` for
    /// pointer visibility after ancestor clipping.
    pub fn logical_focus_target(&self) -> Option<Rect> {
        (!self.disabled && !self.inactive).then_some(self.logical_action_frame?)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SkeletonView {
    layout: ViewLayout,
    surface: Option<&'static str>,
}
impl SkeletonView {
    pub fn paint(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>) {
        paint::skeleton(scene, rect, theme);
    }
    pub fn paint_on_surface(
        scene: &mut Scene,
        rect: Rect,
        palette: Palette,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        paint::skeleton_on_surface(scene, rect, palette, surface)
    }
    /// Validate before replacing the retained context.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(name)?);
        Ok(())
    }
    pub fn surface(&self) -> &'static str {
        self.surface.unwrap_or("canvas")
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        self.layout = ViewLayout {
            frame,
            content: frame,
            visible,
        };
        if visible.is_some() {
            // Geometry already bounds its own ink. Quantizing the leaf frame
            // into a second scissor can erase fractional narrow allocations.
            // Keep the ancestor clip as the sole rectangular clip owner.
            if let Some(clip) = clip {
                scene.push_clip(clip);
            }
            let _ = paint::skeleton_on_surface(scene, frame, palette, self.surface());
            if clip.is_some() {
                scene.pop_clip();
            }
        }
        self.layout
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
}
