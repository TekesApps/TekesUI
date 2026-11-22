use super::{
    ButtonFramePolicy, ButtonLayout, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    ControlMotionContext, ControlState, button_typography_scale, finite_nonnegative, valid_clip,
    valid_frame,
};
use crate::ContainerLayout;
use crate::control_motion::{ProgressTransition, motion_for};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, Default)]
pub struct ButtonModel {
    pub state: ControlState,
    pub frame_policy: ButtonFramePolicy,
    /// Optional compact decorative height, independent of layout/hit geometry.
    /// Larger measured labels expand this proportionally; text is never clipped
    /// to the decorative bounds. Ordinary controls retain their original frame.
    pub chrome_height: Option<f32>,
    pub(super) chrome_bounds: Option<Rect>,
    pub(super) arm: ButtonArm,
    pub(super) placement: Option<ButtonLayout>,
    pub(super) clip_region: Option<tekes_ui_scene::ClipRegion>,
    pub(super) last_pointer: Option<Point>,
    pub(super) hover_transition: ProgressTransition,
    pub(super) press_transition: ProgressTransition,
    pub(super) motion_palette: Option<(
        tekes_ui_tokens::Theme,
        tekes_ui_tokens::palette::PaletteSource,
    )>,
    pub(super) motion_now: Duration,
    pub(super) motion_suspended: bool,
    // Effective parent Surface supplied by the original feedback View.
    pub(super) surface: Option<&'static str>,
    // Menu/Tab selection uses fill; ordinary actions retain their own boundary.
    pub(super) selection_fill_only: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ButtonArm {
    #[default]
    None,
    Pointer,
    PointerActivated,
    Space,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ButtonActionDecision {
    expected: (
        ControlState,
        ButtonArm,
        Option<ButtonLayout>,
        Option<Point>,
        bool,
    ),
    next_state: ControlState,
    next_arm: ButtonArm,
    point: Option<Point>,
    pub consumed: bool,
    pub activated: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ButtonActionSnapshot {
    state: ControlState,
    arm: ButtonArm,
    placement: Option<ButtonLayout>,
    pointer: Option<Point>,
    suspended: bool,
}
impl ButtonModel {
    /// Select and Menu snapshots use the existing Button press decision.
    pub(crate) fn prepare_content_press(&self) -> ButtonActionDecision {
        self.prepare_accessibility_press()
    }
    pub(crate) fn action_snapshot(&self) -> ButtonActionSnapshot {
        ButtonActionSnapshot {
            state: self.state,
            arm: self.arm,
            placement: self.placement,
            pointer: self.last_pointer,
            suspended: self.motion_suspended,
        }
    }
    pub(crate) fn set_surface(&mut self, surface: &'static str) {
        self.surface = Some(surface);
    }
    pub(crate) fn surface(&self) -> Option<&'static str> {
        self.surface
    }
    pub(crate) fn clear_surface(&mut self) {
        self.surface = None;
    }
    /// Component-owned selection appearance; preserves the original state,
    /// motion, geometry, error indicator and independent keyboard focus.
    pub(crate) fn set_selection_fill_only(&mut self, fill_only: bool) {
        self.selection_fill_only = fill_only;
    }
    pub(crate) fn surface_colors(
        &self,
        palette: Palette,
        variant: crate::paint::ButtonVariant,
        surface: &str,
    ) -> tekes_ui_tokens::surface_palette::SurfaceColors {
        crate::paint::button_surface_colors(
            palette,
            surface,
            self.state,
            variant,
            self.hover_transition.value() as f32,
            self.press_transition.value() as f32,
        )
    }
    pub(crate) fn synchronize_activation(&mut self) {
        if self.state.disabled
            || self.state.loading
            || (!self.state.focused && self.arm == ButtonArm::Space)
        {
            self.cancel();
        }
    }

    /// Measure the natural outer button size under an outer width constraint.
    /// The result includes token-driven horizontal and vertical insets and the
    /// button size role's minimum height, scaled by the resolved text role.
    /// Glyph height alone is not a scale input: wrapping must not inflate insets.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        label: &str,
        max_width: f32,
        size: ButtonSize,
        role: proposal::TextRole,
    ) -> Size {
        let scale = button_typography_scale(text, role);
        let inset = size.inset() * scale;
        let vertical_inset = size.vertical_inset(role) * scale;
        let max_width = if max_width.is_finite() {
            max_width.max(0.0)
        } else {
            f32::MAX
        };
        let label_size = if label.is_empty() {
            Size::default()
        } else {
            text.measure(label, role, (max_width - inset * 2.0).max(0.0))
        };
        Size::new(
            if label.is_empty() {
                (inset * 2.0).min(max_width)
            } else {
                (finite_nonnegative(label_size.width) + inset * 2.0).min(max_width)
            },
            (size.minimum_height() * scale)
                .max(finite_nonnegative(label_size.height) + vertical_inset * 2.0),
        )
    }

    pub(crate) fn measure_content(
        text: &mut dyn ButtonTextRenderer,
        content: Size,
        max_width: f32,
        size: ButtonSize,
        role: proposal::TextRole,
    ) -> Size {
        let scale = button_typography_scale(text, role);
        Size::new(
            (finite_nonnegative(content.width) + size.inset() * scale * 2.).min(max_width),
            (size.minimum_height() * scale)
                .max(finite_nonnegative(content.height) + size.vertical_inset(role) * scale * 2.),
        )
    }

    /// Paint the existing action chrome around content measured through the
    /// public composition protocol. ContentHost owns child traversal; this
    /// method retains ButtonModel's original press, motion, focus and hit geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn render_content_frame(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        content_size: Size,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
    ) -> ButtonLayout {
        self.render_content_frame_with_role(
            scene,
            text,
            content_size,
            frame,
            clip,
            palette,
            variant,
            proposal::TextRole::UiBody,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_content_frame_with_role(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        content_size: Size,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        let context = text.motion_context();
        self.render_with_motion_context(
            scene,
            text,
            "",
            frame,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
            context,
            Some(content_size),
            None,
            None,
            false,
        )
    }

    // Tabs close retains the original geometry, motion and focus path while
    // its declared Surface paints only semantic foreground and keyboard focus.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_transparent_content_frame(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        content_size: Size,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
    ) -> ButtonLayout {
        let context = text.motion_context();
        self.render_with_motion_context(
            scene,
            text,
            "",
            frame,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
            context,
            Some(content_size),
            None,
            None,
            true,
        )
    }

    /// Lay out and paint the complete button, including its label. The host
    /// supplies text shaping and glyph output through `ButtonTextRenderer`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) -> ButtonLayout {
        self.render_with_radius(
            scene, text, label, frame, clip, palette, variant, size, role, align, None,
        )
    }

    /// Symbol actions keep the original hit, focus and motion paths but paint
    /// only the foreground from the existing Surface state tokens.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_transparent_label(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) -> ButtonLayout {
        let surface = self.surface;
        self.surface = Some(surface.unwrap_or("canvas"));
        let context = text.motion_context();
        let layout = self.render_with_motion_context(
            scene, text, label, frame, clip, palette, variant, size, role, align, context, None,
            None, None, true,
        );
        self.surface = surface;
        layout
    }

    // Icon-only controls share ButtonModel's interaction/motion/clip machinery, but
    // resolve their own semantic radius. This is internal geometry plumbing;
    // public ButtonModel callers continue to use the size-specific ButtonModel token.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_with_radius(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        radius: Option<f32>,
    ) -> ButtonLayout {
        let context = text.motion_context();
        self.render_with_motion_context(
            scene, text, label, frame, clip, palette, variant, size, role, align, context, None,
            radius, None, false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_animated(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        now: Duration,
        reduced_motion: bool,
    ) -> ButtonLayout {
        self.render_with_motion_context(
            scene,
            text,
            label,
            frame,
            clip,
            palette,
            variant,
            size,
            role,
            align,
            Some(ControlMotionContext {
                now,
                reduced_motion,
            }),
            None,
            None,
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_with_motion_context(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        variant: crate::paint::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        context: Option<ControlMotionContext>,
        measured_content: Option<Size>,
        radius: Option<f32>,
        prepared: Option<ButtonLayout>,
        transparent_background: bool,
    ) -> ButtonLayout {
        // Pointer capture survives focus changes; only a keyboard Space arm
        // is canceled when focus leaves between key-down and key-up.
        self.synchronize_activation();
        if !valid_frame(frame) || clip.is_some_and(|clip| !valid_clip(clip)) {
            self.placement = Some(ButtonLayout::empty());
            self.chrome_bounds = None;
            self.blur();
            return ButtonLayout::empty();
        }
        let palette = palette.into();
        let scale = button_typography_scale(text, role);
        let inset = if self.frame_policy == ButtonFramePolicy::Exact {
            0.
        } else {
            size.inset() * scale
        };
        let max_width = (frame.size.width - inset * 2.0).max(0.0);
        let layout = prepared.unwrap_or_else(|| {
            let label_size = measured_content.unwrap_or_else(|| {
                if label.is_empty() {
                    Size::default()
                } else {
                    text.measure(label, role, max_width)
                }
            });
            ButtonLayout::resolve_scaled(
                frame,
                clip,
                label_size,
                size,
                role,
                align,
                self.frame_policy,
                scale,
            )
        });
        self.placement = Some(layout);
        let chrome = self
            .chrome_height
            .filter(|h| h.is_finite() && *h > 0.)
            .map_or(layout.frame, |height| {
                let ratio = (layout.label_size.height / role.style().line_height.max(1.)).max(1.);
                let height = (height * ratio).min(layout.frame.size.height);
                Rect::new(
                    layout.frame.min_x(),
                    layout.frame.center().y - height * 0.5,
                    layout.frame.size.width,
                    height,
                )
            });
        self.chrome_bounds = Some(chrome);
        if layout.visible.is_none() {
            self.blur();
        }
        self.refresh_pointer_state();

        let transition_progress = if let Some(context) = context {
            self.motion_now = context.now;
            let palette_key = (palette.theme, palette.source());
            let palette_changed = self
                .motion_palette
                .is_some_and(|previous| previous != palette_key);
            self.motion_palette = Some(palette_key);
            let interaction_enabled =
                !self.motion_suspended && !self.state.disabled && !self.state.loading;
            let hover_target = if interaction_enabled && self.state.hovered {
                1.0
            } else {
                0.0
            };
            let press_target = if interaction_enabled && self.state.pressed {
                1.0
            } else {
                0.0
            };
            let snap = palette_changed || !interaction_enabled;
            let hover = if snap {
                self.hover_transition.snap(hover_target, context.now)
            } else {
                self.hover_transition.retarget(
                    hover_target,
                    motion_for("button_hover"),
                    context.now,
                    context.reduced_motion,
                )
            }
            .unwrap_or_else(|_| self.hover_transition.value());
            let pressed = if snap {
                self.press_transition.snap(press_target, context.now)
            } else {
                self.press_transition.retarget(
                    press_target,
                    motion_for("button_press"),
                    context.now,
                    context.reduced_motion,
                )
            }
            .unwrap_or_else(|_| self.press_transition.value());
            if self.needs_animation() {
                text.request_animation_frame();
            }
            Some((hover as f32, pressed as f32))
        } else {
            let _ = self.hover_transition.snap(
                if self.state.hovered && !self.state.disabled && !self.state.loading {
                    1.0
                } else {
                    0.0
                },
                self.motion_now,
            );
            let _ = self.press_transition.snap(
                if self.state.pressed && !self.state.disabled && !self.state.loading {
                    1.0
                } else {
                    0.0
                },
                self.motion_now,
            );
            None
        };

        if let Some(visible) = layout.visible {
            let background_clip =
                ContainerLayout::new(layout.frame, crate::Insets::ZERO, crate::Insets::ZERO, clip)
                    .background_paint_clip();
            if let Some(background_clip) = background_clip {
                scene.push_clip(background_clip);
            }
            let ink = if let Some(surface) = self
                .surface
                .or((variant == crate::paint::ButtonVariant::Toolbar).then_some("toolbar"))
            {
                let (hover, pressed) = transition_progress.unwrap_or((
                    self.hover_transition.value() as f32,
                    self.press_transition.value() as f32,
                ));
                crate::paint::button_variant_on_surface(
                    scene,
                    chrome,
                    palette,
                    self.state,
                    variant,
                    radius.unwrap_or_else(|| size.radius() * scale),
                    hover,
                    pressed,
                    surface,
                    self.selection_fill_only,
                    transparent_background,
                )
                .text
            } else {
                let (hover, pressed) = transition_progress.unwrap_or((
                    if self.state.hovered { 1. } else { 0. },
                    if self.state.pressed { 1. } else { 0. },
                ));
                crate::paint::button_variant_with_selection_fill(
                    scene,
                    chrome,
                    palette,
                    self.state,
                    variant,
                    radius.unwrap_or_else(|| size.radius() * scale),
                    hover,
                    pressed,
                    self.selection_fill_only,
                )
            };
            if background_clip.is_some() {
                scene.pop_clip();
            }
            if !label.is_empty() && !layout.content.is_empty() {
                scene.push_clip(visible);
                text.draw(scene, label, role, layout.content, align, ink);
                scene.pop_clip();
            }
        }
        layout
    }

    // Geometry and activation reconciliation shared by original render and the
    // retained Segmented View. No Scene, motion tick or frame request occurs here.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_layout(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
        measured_content: Option<Size>,
    ) -> ButtonLayout {
        // Pointer capture survives focus changes; only a keyboard Space arm
        // is canceled when focus leaves between key-down and key-up.
        self.synchronize_activation();
        if !valid_frame(frame) || clip.is_some_and(|clip| !valid_clip(clip)) {
            self.placement = Some(ButtonLayout::empty());
            self.chrome_bounds = None;
            self.blur();
            return ButtonLayout::empty();
        }
        let scale = button_typography_scale(text, role);
        let inset = if self.frame_policy == ButtonFramePolicy::Exact {
            0.
        } else {
            size.inset() * scale
        };
        let max_width = (frame.size.width - inset * 2.0).max(0.0);
        let layout = {
            let label_size = measured_content.unwrap_or_else(|| {
                if label.is_empty() {
                    Size::default()
                } else {
                    text.measure(label, role, max_width)
                }
            });
            ButtonLayout::resolve_scaled(
                frame,
                clip,
                label_size,
                size,
                role,
                align,
                self.frame_policy,
                scale,
            )
        };
        self.prepare_published_layout(layout, role);

        layout
    }
    fn prepare_published_layout(&mut self, layout: ButtonLayout, role: proposal::TextRole) {
        self.placement = Some(layout);
        let chrome = self
            .chrome_height
            .filter(|h| h.is_finite() && *h > 0.)
            .map_or(layout.frame, |height| {
                let ratio = (layout.label_size.height / role.style().line_height.max(1.)).max(1.);
                let height = (height * ratio).min(layout.frame.size.height);
                Rect::new(
                    layout.frame.min_x(),
                    layout.frame.center().y - height * 0.5,
                    layout.frame.size.width,
                    height,
                )
            });
        self.chrome_bounds = Some(chrome);
        if layout.visible.is_none() {
            self.blur();
        }
        self.refresh_pointer_state();
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_prepared_segment(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        layout: ButtonLayout,
        clip: Option<Rect>,
        palette: Palette,
        size: ButtonSize,
        role: proposal::TextRole,
        transparent_background: bool,
    ) {
        let context = text.motion_context();
        self.render_with_motion_context(
            scene,
            text,
            label,
            layout.frame,
            clip,
            palette,
            crate::paint::ButtonVariant::Quiet,
            size,
            role,
            ButtonTextAlign::Leading,
            context,
            None,
            None,
            Some(layout),
            transparent_background,
        );
    }

    /// Arrange through the original resolver without painting or arming input.
    pub(crate) fn prepare_label(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        let scale = button_typography_scale(text, role);
        let width = (frame.size.width - ButtonSize::Body.inset() * scale * 2.).max(0.);
        let measured = text.measure(label, role, width);
        let layout = ButtonLayout::resolve_scaled(
            frame,
            clip,
            measured,
            ButtonSize::Body,
            role,
            ButtonTextAlign::default(),
            self.frame_policy,
            scale,
        );
        self.placement = Some(layout);
        layout
    }
    // Keep the original arranged geometry, clip and text role explicit.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_prepared_label(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        layout: ButtonLayout,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) {
        self.paint_prepared_label_variant(
            scene,
            text,
            label,
            layout,
            clip,
            palette,
            role,
            crate::paint::ButtonVariant::Primary,
        );
    }
    // Preserve the existing explicit renderer/layout/clip/Theme contract;
    // the variant is the original Toast Quiet versus Banner Primary choice.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_prepared_label_variant(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        layout: ButtonLayout,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        variant: crate::paint::ButtonVariant,
    ) {
        let context = text.motion_context();
        self.render_with_motion_context(
            scene,
            text,
            label,
            layout.frame,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            role,
            ButtonTextAlign::default(),
            context,
            None,
            None,
            Some(layout),
            false,
        );
    }

    pub fn layout(&self) -> Option<ButtonLayout> {
        self.placement
    }

    /// Decorative bounds from the last render. Hit testing continues to use
    /// `layout().visible`, including the larger accessible action target.
    pub fn chrome_bounds(&self) -> Option<Rect> {
        self.chrome_bounds
    }

    pub fn clear_layout(&mut self) {
        self.placement = None;
        self.chrome_bounds = None;
        self.blur();
    }

    /// Whether this component currently needs a host-driven animation frame.
    pub fn needs_animation(&self) -> bool {
        self.hover_transition.needs_animation() || self.press_transition.needs_animation()
    }

    /// Advance component-owned presentation state using the host monotonic clock.
    /// Returns true when the sampled presentation changed.
    pub fn tick(&mut self, now: Duration) -> bool {
        self.motion_now = now;
        let before = (self.hover_transition.value(), self.press_transition.value());
        let _ = self.hover_transition.advance(now);
        let _ = self.press_transition.advance(now);
        before != (self.hover_transition.value(), self.press_transition.value())
    }

    /// Apply the host reduced-motion preference immediately.
    pub fn set_reduced_motion(&mut self, reduced: bool, now: Duration) -> bool {
        self.motion_now = now;
        let before = (self.hover_transition.value(), self.press_transition.value());
        let _ = self.hover_transition.set_reduced_motion(reduced, now);
        let _ = self.press_transition.set_reduced_motion(reduced, now);
        if reduced {
            let _ = self.hover_transition.snap(
                if self.state.hovered && !self.state.disabled && !self.state.loading {
                    1.0
                } else {
                    0.0
                },
                now,
            );
            let _ = self.press_transition.snap(
                if self.state.pressed && !self.state.disabled && !self.state.loading {
                    1.0
                } else {
                    0.0
                },
                now,
            );
        }
        before != (self.hover_transition.value(), self.press_transition.value())
    }

    /// Suspend visual interaction while the owning window is inactive.
    pub fn set_window_active(&mut self, active: bool, now: Duration) {
        self.motion_now = now;
        self.motion_suspended = !active;
        if !active {
            self.blur();
        }
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        self.last_pointer = Some(point);
        self.refresh_pointer_state();
    }

    pub(crate) fn set_clip_region(&mut self, region: Option<tekes_ui_scene::ClipRegion>) {
        self.clip_region = region;
        self.refresh_pointer_state();
    }

    fn hit_test(&self, point: Point) -> bool {
        self.placement.is_some_and(|layout| layout.hit_test(point))
            && self
                .clip_region
                .as_ref()
                .is_none_or(|region| region.contains([point.x as f64, point.y as f64]) == Ok(true))
    }

    fn decided_pointer_arm(&self, state: ControlState) -> (ControlState, ButtonArm) {
        self.decided_pointer_arm_from(state, self.arm)
    }
    fn decided_pointer_arm_from(
        &self,
        mut state: ControlState,
        arm: ButtonArm,
    ) -> (ControlState, ButtonArm) {
        if arm != ButtonArm::None {
            return (state, arm);
        }
        let armed = !self.motion_suspended && !state.disabled && !state.loading;
        state.pressed = armed;
        state.focused = armed;
        state.focus_visible = false;
        (
            state,
            if armed {
                ButtonArm::Pointer
            } else {
                ButtonArm::None
            },
        )
    }

    pub(crate) fn prepare_content_down(
        &self,
        point: Point,
        activate_on_press: bool,
    ) -> ButtonActionDecision {
        self.prepare_content_down_after_cancel(point, activate_on_press, false)
    }
    pub(crate) fn prepare_content_down_after_cancel(
        &self,
        point: Point,
        activate_on_press: bool,
        cancel: bool,
    ) -> ButtonActionDecision {
        let mut state = self.state;
        let mut arm = self.arm;
        if cancel {
            state.pressed = false;
            arm = ButtonArm::None;
        }
        let inside = self.hit_test(point);
        state.hovered = inside && !self.motion_suspended && !state.disabled && !state.loading;
        if matches!(arm, ButtonArm::Pointer | ButtonArm::PointerActivated) {
            state.pressed = state.hovered;
        }
        if inside {
            (state, arm) = self.decided_pointer_arm_from(state, arm);
        }
        let consumed = inside && arm == ButtonArm::Pointer;
        let activated = consumed
            && activate_on_press
            && !self.motion_suspended
            && !state.disabled
            && !state.loading;
        if activated {
            arm = ButtonArm::PointerActivated;
        }
        ButtonActionDecision {
            expected: (
                self.state,
                self.arm,
                self.placement,
                self.last_pointer,
                self.motion_suspended,
            ),
            next_state: state,
            next_arm: arm,
            point: Some(point),
            consumed,
            activated,
        }
    }

    pub(crate) fn prepare_accessibility_press(&self) -> ButtonActionDecision {
        let (mut state, mut arm) = self.decided_pointer_arm(self.state);
        let activated = arm == ButtonArm::Pointer && !state.disabled && !state.loading;
        if matches!(arm, ButtonArm::Pointer | ButtonArm::PointerActivated) {
            arm = ButtonArm::None;
            state.pressed = false;
        }
        ButtonActionDecision {
            expected: (
                self.state,
                self.arm,
                self.placement,
                self.last_pointer,
                self.motion_suspended,
            ),
            next_state: state,
            next_arm: arm,
            point: self.last_pointer,
            consumed: activated,
            activated,
        }
    }

    pub(crate) fn prepare_content_cancel(&self) -> ButtonActionDecision {
        let mut state = self.state;
        state.pressed = false;
        ButtonActionDecision {
            expected: (
                self.state,
                self.arm,
                self.placement,
                self.last_pointer,
                self.motion_suspended,
            ),
            next_state: state,
            next_arm: ButtonArm::None,
            point: self.last_pointer,
            consumed: false,
            activated: false,
        }
    }
    pub(crate) fn apply_content_action(&mut self, decision: ButtonActionDecision) {
        self.state = decision.next_state;
        self.arm = decision.next_arm;
        self.last_pointer = decision.point;
    }

    pub(super) fn refresh_pointer_state(&mut self) {
        let Some(point) = self.last_pointer else {
            return;
        };
        let inside = self.hit_test(point);
        self.state.hovered =
            inside && !self.motion_suspended && !self.state.disabled && !self.state.loading;
        if matches!(self.arm, ButtonArm::Pointer | ButtonArm::PointerActivated) {
            self.state.pressed = self.state.hovered;
        }
    }

    pub fn pointer_down_at(&mut self, point: Point) -> bool {
        if !self.hit_test(point) {
            self.pointer_move_at(point);
            return false;
        }
        self.pointer_move_at(point);
        self.pointer_down();
        self.arm == ButtonArm::Pointer
    }

    /// Read the original release decision without consuming its pointer arm.
    pub(crate) fn pointer_release_would_activate(&self, point: Point) -> bool {
        self.arm == ButtonArm::Pointer
            && !self.state.disabled
            && !self.state.loading
            && self.hit_test(point)
    }

    pub(crate) fn pointer_armed(&self) -> bool {
        matches!(self.arm, ButtonArm::Pointer | ButtonArm::PointerActivated)
    }

    pub fn pointer_up_at(&mut self, point: Point) -> bool {
        self.last_pointer = Some(point);
        let inside = self.hit_test(point);
        let activated = self.pointer_up(inside);
        self.state.hovered =
            inside && !self.motion_suspended && !self.state.disabled && !self.state.loading;
        activated
    }

    pub fn pointer_down(&mut self) {
        if self.arm != ButtonArm::None {
            return;
        }
        let armed = !self.motion_suspended && !self.state.disabled && !self.state.loading;
        self.arm = if armed {
            ButtonArm::Pointer
        } else {
            ButtonArm::None
        };
        self.state.pressed = armed;
        self.state.focused = armed;
        self.state.focus_visible = false;
    }
    pub fn pointer_up(&mut self, inside: bool) -> bool {
        if !matches!(self.arm, ButtonArm::Pointer | ButtonArm::PointerActivated) {
            return false;
        }
        let activated =
            self.arm == ButtonArm::Pointer && inside && !self.state.disabled && !self.state.loading;
        self.cancel();
        activated
    }
    pub fn cancel(&mut self) {
        self.arm = ButtonArm::None;
        self.state.pressed = false;
    }

    /// Cancel only an active pointer press, preserving a keyboard Space press.
    pub fn cancel_pointer(&mut self) {
        if matches!(self.arm, ButtonArm::Pointer | ButtonArm::PointerActivated) {
            self.cancel();
        }
    }

    /// Cancel active activation when the owning focus scope loses focus.
    pub fn blur(&mut self) {
        self.cancel();
        self.last_pointer = None;
        self.state.focused = false;
        self.state.focus_visible = false;
        self.state.hovered = false;
        let _ = self.hover_transition.snap(0.0, self.motion_now);
        let _ = self.press_transition.snap(0.0, self.motion_now);
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        if self.motion_suspended || self.state.disabled || self.state.loading {
            self.cancel();
            return false;
        }
        if !self.state.focused {
            if self.arm == ButtonArm::Space {
                self.cancel();
            }
            return false;
        }
        match key {
            ControlKey::Enter => down,
            ControlKey::Space => {
                if down {
                    if self.arm != ButtonArm::None {
                        return false;
                    }
                    self.arm = ButtonArm::Space;
                    self.state.pressed = true;
                    false
                } else {
                    if self.arm != ButtonArm::Space {
                        return false;
                    }
                    self.cancel();
                    true
                }
            }
            _ => false,
        }
    }
}
