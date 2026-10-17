//! Platform-independent interaction contracts. Rendering and native event translation
//! are adapters; these models are shared by applications and Gallery stories.
use crate::control_motion::{ProgressTransition, motion_for};
use crate::{ContainerLayout, Insets};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, proposal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlMotionContext {
    pub now: Duration,
    pub reduced_motion: bool,
}

/// Button size roles resolve through the JSON Design System token table.
/// `Body` preserves the existing 24pt/12pt application toolbar action metric.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
    Body,
}

impl ButtonSize {
    fn radius(self) -> f32 {
        let token = match self {
            Self::Xs => "c-button-xs-radius",
            Self::Sm => "c-button-sm-radius",
            Self::Md | Self::Body => "c-button-md-radius",
            Self::Lg => "c-button-lg-radius",
        };
        proposal::metric("radius", token).expect("Button component radius token")
    }
    fn minimum_height(self) -> f32 {
        match self {
            Self::Xs => proposal::metric("size", "size-button-xs"),
            Self::Sm => proposal::metric("size", "size-button-sm"),
            Self::Md => proposal::metric("size", "size-button-md"),
            Self::Lg => proposal::metric("size", "size-button-lg"),
            Self::Body => proposal::metric("size", "size-action-button"),
        }
        .expect("button minimum size token")
    }

    fn inset(self) -> f32 {
        let name = match self {
            Self::Xs => "space-button-xs-inset",
            Self::Sm => "space-button-sm-inset",
            Self::Md => "space-button-md-inset",
            Self::Lg => "space-button-lg-inset",
            Self::Body => "space-relaxed",
        };
        proposal::metric("spacing", name).expect("button inset token")
    }

    fn vertical_inset(self, role: proposal::TextRole) -> f32 {
        ((self.minimum_height() - role.style().line_height) * 0.5).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonTextAlign {
    Leading,
    Center,
}
impl Default for ButtonTextAlign {
    fn default() -> Self {
        match proposal::layout::BUTTON_CONTENT.text_alignment {
            proposal::layout::TextAlignment::Leading => Self::Leading,
            proposal::layout::TextAlignment::Center => Self::Center,
            proposal::layout::TextAlignment::Trailing => unreachable!(
                "Design compiler rejects unsupported trailing Button content alignment"
            ),
        }
    }
}

/// Text shaping and glyph output are supplied by the host; layout and paint
/// remain owned by the production Button component.
pub trait ButtonTextRenderer {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size;
    /// Optional host clock and motion preference for animated production controls.
    fn motion_context(&self) -> Option<ControlMotionContext> {
        None
    }
    /// Ask the host for another frame while a component-owned transition is active.
    fn request_animation_frame(&mut self) {}
    /// Backing scale used for scale-aware decorative geometry. Text adapters
    /// that do not know the renderer scale keep the compatible 1x default.
    fn backing_scale(&self) -> f32 {
        1.0
    }
    /// Height of the first shaped line. Existing adapters may rely on the
    /// fallback; text engines with shaping data should return the actual first
    /// line metric for the requested label and width.
    fn first_line_height(&mut self, _text: &str, role: proposal::TextRole, max_width: f32) -> f32 {
        self.measure("M", role, max_width).height
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    );
}

/// Resolved geometry shared by button paint, text placement and hit testing.
/// `content` is the constrained text slot; it is deliberately the same width
/// used for shaping so the host cannot accidentally wrap at a second width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonLayout {
    pub frame: Rect,
    pub content: Rect,
    /// Visible hit and text clip after intersecting the ancestor clip.
    pub visible: Option<Rect>,
    /// Shaped extent returned by the host text backend.
    pub label_size: Size,
}

impl ButtonLayout {
    fn resolve(
        frame: Rect,
        clip: Option<Rect>,
        label_size: Size,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) -> Self {
        let horizontal_inset = size.inset();
        let vertical_inset = size.vertical_inset(role);
        let label_size = Size::new(
            finite_nonnegative(label_size.width),
            finite_nonnegative(label_size.height),
        );
        let label_height = finite_nonnegative(label_size.height);
        let min_height = size.minimum_height();
        let height = frame
            .size
            .height
            .max(min_height)
            .max(label_height + vertical_inset * 2.0);
        let frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, height);
        let content_width = (frame.size.width - horizontal_inset * 2.0).max(0.0);
        let content_height = label_height.min((frame.size.height - vertical_inset * 2.0).max(0.0));
        let content_x = match align {
            ButtonTextAlign::Leading | ButtonTextAlign::Center => frame.min_x() + horizontal_inset,
        };
        let content = Rect::new(
            content_x,
            frame.center().y - content_height * 0.5,
            content_width,
            content_height,
        );
        let visible = clip.map_or(Some(frame), |clip| frame.intersection(&clip));
        Self {
            frame,
            content,
            visible,
            label_size,
        }
    }

    fn empty() -> Self {
        Self {
            frame: Rect::default(),
            content: Rect::default(),
            visible: None,
            label_size: Size::default(),
        }
    }

    pub fn hit_test(&self, point: Point) -> bool {
        self.visible.is_some_and(|rect| rect.contains(point))
    }

    /// Bounds including the Design System focus ring, for host focus reveal.
    pub fn focus_bounds(&self) -> Rect {
        expand_for_focus(self.frame)
    }
}

fn expand_for_focus(rect: Rect) -> Rect {
    let bleed = proposal::metric("stroke", "c-focus-width")
        .unwrap_or(0.0)
        .max(0.0)
        + proposal::metric("spacing", "c-focus-offset")
            .unwrap_or(0.0)
            .max(0.0);
    Rect::new(
        rect.min_x() - bleed,
        rect.min_y() - bleed,
        rect.size.width + bleed * 2.0,
        rect.size.height + bleed * 2.0,
    )
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn valid_frame(frame: Rect) -> bool {
    frame.min_x().is_finite()
        && frame.min_y().is_finite()
        && frame.size.width.is_finite()
        && frame.size.height.is_finite()
        && frame.max_x().is_finite()
        && frame.max_y().is_finite()
        && frame.size.width > 0.0
        && frame.size.height >= 0.0
}

fn valid_clip(clip: Rect) -> bool {
    clip.min_x().is_finite()
        && clip.min_y().is_finite()
        && clip.size.width.is_finite()
        && clip.size.height.is_finite()
        && clip.max_x().is_finite()
        && clip.max_y().is_finite()
        && clip.size.width > 0.0
        && clip.size.height > 0.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Enter,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    Tab { reverse: bool },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub focus_visible: bool,
    pub disabled: bool,
    pub loading: bool,
    pub selected: bool,
    pub error: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Button {
    pub state: ControlState,
    arm: ButtonArm,
    placement: Option<ButtonLayout>,
    last_pointer: Option<Point>,
    hover_transition: ProgressTransition,
    press_transition: ProgressTransition,
    motion_palette: Option<(tekes_ui_tokens::Theme, usize)>,
    motion_now: Duration,
    motion_suspended: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ButtonArm {
    #[default]
    None,
    Pointer,
    Space,
}
impl Button {
    fn synchronize_activation(&mut self) {
        if self.state.disabled
            || self.state.loading
            || (!self.state.focused && self.arm == ButtonArm::Space)
        {
            self.cancel();
        }
    }

    /// Measure the natural outer button size under an outer width constraint.
    /// The result includes token-driven horizontal and vertical insets and the
    /// button size role's minimum height.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        label: &str,
        max_width: f32,
        size: ButtonSize,
        role: proposal::TextRole,
    ) -> Size {
        let inset = size.inset();
        let vertical_inset = size.vertical_inset(role);
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
            size.minimum_height()
                .max(finite_nonnegative(label_size.height) + vertical_inset * 2.0),
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
        let context = text.motion_context();
        self.render_with_motion_context(
            scene, text, label, frame, clip, palette, variant, size, role, align, context,
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
    ) -> ButtonLayout {
        // Pointer capture survives focus changes; only a keyboard Space arm
        // is canceled when focus leaves between key-down and key-up.
        self.synchronize_activation();
        if !valid_frame(frame) || clip.is_some_and(|clip| !valid_clip(clip)) {
            self.placement = Some(ButtonLayout::empty());
            self.blur();
            return ButtonLayout::empty();
        }
        let palette = palette.into();
        let inset = size.inset();
        let max_width = (frame.size.width - inset * 2.0).max(0.0);
        let label_size = if label.is_empty() {
            Size::default()
        } else {
            text.measure(label, role, max_width)
        };
        let layout = ButtonLayout::resolve(frame, clip, label_size, size, role, align);
        self.placement = Some(layout);
        if layout.visible.is_none() {
            self.blur();
        }
        self.refresh_pointer_state();

        let transition_progress = if let Some(context) = context {
            self.motion_now = context.now;
            let palette_key = (palette.theme, palette.brand);
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
            let ink = if let Some((hover, pressed)) = transition_progress {
                crate::paint::button_variant_with_interaction(
                    scene,
                    layout.frame,
                    palette,
                    self.state,
                    variant,
                    size.radius(),
                    hover,
                    pressed,
                )
            } else {
                crate::paint::button_variant_with_radius(
                    scene,
                    layout.frame,
                    palette,
                    self.state,
                    variant,
                    size.radius(),
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

    pub fn layout(&self) -> Option<ButtonLayout> {
        self.placement
    }

    pub fn clear_layout(&mut self) {
        self.placement = None;
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

    fn refresh_pointer_state(&mut self) {
        let Some(point) = self.last_pointer else {
            return;
        };
        let inside = self.placement.is_some_and(|layout| layout.hit_test(point));
        self.state.hovered = inside && !self.state.disabled && !self.state.loading;
        if self.arm == ButtonArm::Pointer {
            self.state.pressed = self.state.hovered;
        }
    }

    pub fn pointer_down_at(&mut self, point: Point) -> bool {
        if !self.placement.is_some_and(|layout| layout.hit_test(point)) {
            self.pointer_move_at(point);
            return false;
        }
        self.pointer_move_at(point);
        self.pointer_down();
        self.arm == ButtonArm::Pointer
    }

    pub fn pointer_up_at(&mut self, point: Point) -> bool {
        self.last_pointer = Some(point);
        let inside = self.placement.is_some_and(|layout| layout.hit_test(point));
        let activated = self.pointer_up(inside);
        self.state.hovered = inside && !self.state.disabled && !self.state.loading;
        activated
    }

    pub fn pointer_down(&mut self) {
        if self.arm != ButtonArm::None {
            return;
        }
        let armed = !self.state.disabled && !self.state.loading;
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
        if self.arm != ButtonArm::Pointer {
            return false;
        }
        let activated = inside && !self.state.disabled && !self.state.loading;
        self.cancel();
        activated
    }
    pub fn cancel(&mut self) {
        self.arm = ButtonArm::None;
        self.state.pressed = false;
    }

    /// Cancel only an active pointer press, preserving a keyboard Space press.
    pub fn cancel_pointer(&mut self) {
        if self.arm == ButtonArm::Pointer {
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
        if self.state.disabled || self.state.loading {
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    Mixed,
}
#[derive(Clone, Debug, Default)]
pub struct Checkbox {
    pub button: Button,
    pub checked: CheckState,
}
impl Checkbox {
    /// Measure the complete checkbox row. The visual box is 16pt; the label
    /// wraps inside the space remaining after that box and the 6pt label gap.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        label: &str,
        max_width: f32,
        role: proposal::TextRole,
    ) -> Size {
        let width = finite_nonnegative(max_width);
        let control_size = checkbox_control_size();
        let label_gap = checkbox_label_gap();
        let label_width = (width - control_size - label_gap).max(0.0);
        let (label_size, first_line_height) = if label.is_empty() || label_width == 0.0 {
            (Size::default(), 0.0)
        } else {
            let first_line_height =
                finite_nonnegative(text.first_line_height(label, role, label_width));
            let label_size = text.measure(label, role, label_width);
            (label_size, first_line_height)
        };
        let (content_height, _, _) = checkbox_content_geometry(
            first_line_height,
            finite_nonnegative(label_size.height),
            !label.is_empty() && label_width > 0.0,
        );
        Size::new(width, checkbox_row_min_height().max(content_height + 12.0))
    }

    /// Paint and place the complete checkbox row. Its outer placement is also
    /// installed on the shared Button interaction model, so pointer and
    /// keyboard behavior use the same clipped geometry as drawing.
    // Like Button::render, this adapts layout, state, drawing, and text shaping.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: impl Into<Palette>,
        role: proposal::TextRole,
    ) -> CheckboxLayout {
        if !valid_frame(frame) || clip.is_some_and(|clip| !valid_clip(clip)) {
            self.button.placement = Some(ButtonLayout::empty());
            self.button.blur();
            return CheckboxLayout::default();
        }
        let frame_width = finite_nonnegative(frame.size.width);
        let control_size = checkbox_control_size();
        let label_gap = checkbox_label_gap();
        let label_width = (frame_width - control_size - label_gap).max(0.0);
        let has_label = !label.is_empty() && label_width > 0.0;
        let first_line_height = if has_label {
            finite_nonnegative(text.first_line_height(label, role, label_width))
        } else {
            0.0
        };
        let measured = if !has_label {
            Size::default()
        } else {
            text.measure(label, role, label_width)
        };
        let measured = Size::new(
            finite_nonnegative(measured.width),
            finite_nonnegative(measured.height),
        );
        let (content_height, control_content_top, label_content_top) =
            checkbox_content_geometry(first_line_height, measured.height, has_label);
        let height = finite_nonnegative(frame.size.height)
            .max(checkbox_row_min_height().max(content_height + 12.0));
        let content_top = (height - content_height).max(0.0) * 0.5;
        let container = ContainerLayout::new(
            Rect::new(frame.min_x(), frame.min_y(), frame_width, height),
            Insets::ZERO,
            Insets::ZERO,
            clip,
        );
        let frame = container.frame();
        let control = Rect::new(
            frame.min_x(),
            frame.min_y() + content_top + control_content_top,
            control_size,
            control_size,
        );
        let label_slot = if has_label {
            Rect::new(
                frame.min_x() + control_size + label_gap,
                frame.min_y() + content_top + label_content_top,
                label_width,
                measured.height,
            )
        } else {
            Rect::new(
                frame.min_x() + control_size + label_gap,
                frame.min_y(),
                0.0,
                0.0,
            )
        };
        let layout = CheckboxLayout {
            frame,
            control,
            label: label_slot,
            visible: container.visible_clip(),
        };
        self.button.placement = Some(ButtonLayout {
            frame,
            content: label_slot,
            visible: layout.visible,
            label_size: measured,
        });
        self.button.synchronize_activation();
        self.button.refresh_pointer_state();

        if let Some(visible) = layout.visible {
            let palette = palette.into();
            let control_fits_row = control.min_x() >= frame.min_x()
                && control.min_y() >= frame.min_y()
                && control.max_x() <= frame.max_x()
                && control.max_y() <= frame.max_y();
            let control_clip = if control_fits_row {
                container.background_paint_clip()
            } else {
                // Preserve the legacy row boundary when a narrow row cannot
                // contain the fixed-size indicator; only fitting indicators
                // may expose their outer AA fringe beyond the row frame.
                container.visible_clip()
            };
            if let Some(control_clip) = control_clip {
                scene.push_clip(control_clip);
            }
            let _mark_ink =
                crate::paint::checkbox(scene, control, palette, self.button.state, self.checked);
            if control_clip.is_some() {
                scene.pop_clip();
            }
            let label_ink = crate::paint::color(
                palette,
                if self.button.state.disabled {
                    "tertiary-text"
                } else {
                    "primary-text"
                },
            );
            if has_label && !label_slot.is_empty() {
                scene.push_clip(visible);
                text.draw(
                    scene,
                    label,
                    role,
                    label_slot,
                    ButtonTextAlign::Leading,
                    label_ink,
                );
                scene.pop_clip();
            }
        } else {
            self.button.blur();
        }
        layout
    }

    pub fn layout(&self) -> Option<ButtonLayout> {
        self.button.layout()
    }

    pub fn clear_layout(&mut self) {
        self.button.clear_layout();
    }

    pub fn pointer_move_at(&mut self, point: Point) {
        self.button.pointer_move_at(point);
    }

    pub fn pointer_down_at(&mut self, point: Point) -> bool {
        self.button.pointer_down_at(point)
    }

    pub fn pointer_up_at(&mut self, point: Point) -> bool {
        let activated = self.button.pointer_up_at(point);
        if activated {
            self.toggle();
        }
        activated
    }

    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        if key == ControlKey::Space && self.button.key(key, down) {
            self.toggle()
        } else {
            false
        }
    }

    pub fn toggle(&mut self) -> bool {
        if self.button.state.disabled || self.button.state.loading {
            return false;
        }
        self.checked = if self.checked == CheckState::Checked {
            CheckState::Unchecked
        } else {
            CheckState::Checked
        };
        true
    }
}

fn checkbox_control_size() -> f32 {
    proposal::metric("size", "c-checkbox-size").expect("checkbox visual size")
}

fn checkbox_label_gap() -> f32 {
    proposal::metric("spacing", "c-checkbox-label-gap").expect("checkbox label gap")
}

fn checkbox_row_min_height() -> f32 {
    proposal::metric("size", "c-checkbox-row-min").expect("checkbox hit row")
}

fn checkbox_content_geometry(
    first_line_height: f32,
    label_height: f32,
    has_label: bool,
) -> (f32, f32, f32) {
    let control_size = checkbox_control_size();
    if !has_label {
        return (control_size, 0.0, 0.0);
    }
    let first_line_height = finite_nonnegative(first_line_height);
    let label_height = finite_nonnegative(label_height);
    let label_top = (control_size - first_line_height).max(0.0) * 0.5;
    let control_top = (first_line_height - control_size).max(0.0) * 0.5;
    let content_height = (label_top + label_height).max(control_top + control_size);
    (content_height, control_top, label_top)
}

/// Geometry for a checkbox's full hit row, visual mark, label, and visible clip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CheckboxLayout {
    pub frame: Rect,
    pub control: Rect,
    pub label: Rect,
    pub visible: Option<Rect>,
}

impl CheckboxLayout {
    pub fn hit_test(&self, point: Point) -> bool {
        self.visible.is_some_and(|rect| rect.contains(point))
    }

    /// Bounds including the Design System focus ring, for host focus reveal.
    pub fn focus_bounds(&self) -> Rect {
        expand_for_focus(self.frame)
    }
}
#[derive(Clone, Debug)]
pub struct Tabs {
    pub selected: usize,
    pub count: usize,
}
impl Tabs {
    pub fn new(count: usize) -> Self {
        Self { selected: 0, count }
    }
    pub fn select(&mut self, i: usize) -> bool {
        if i >= self.count || i == self.selected {
            return false;
        }
        self.selected = i;
        true
    }
    pub fn key(&mut self, key: ControlKey) -> bool {
        if self.count == 0 {
            return false;
        }
        let n = match key {
            ControlKey::Left => (self.selected + self.count - 1) % self.count,
            ControlKey::Right => (self.selected + 1) % self.count,
            ControlKey::Home => 0,
            ControlKey::End => self.count - 1,
            _ => return false,
        };
        self.select(n)
    }
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub open: bool,
    pub items: Tabs,
    pub last_action: Option<usize>,
}
impl Menu {
    pub fn new(count: usize) -> Self {
        Self {
            open: false,
            items: Tabs::new(count),
            last_action: None,
        }
    }
    pub fn show(&mut self) {
        self.open = true;
        self.items.selected = 0;
    }
    pub fn key(&mut self, key: ControlKey) -> Option<usize> {
        if !self.open {
            return None;
        }
        match key {
            ControlKey::Escape => self.open = false,
            ControlKey::Enter | ControlKey::Space if self.items.count > 0 => {
                let i = self.items.selected;
                self.last_action = Some(i);
                self.open = false;
                return Some(i);
            }
            ControlKey::Up => {
                self.items.key(ControlKey::Left);
            }
            ControlKey::Down => {
                self.items.key(ControlKey::Right);
            }
            ControlKey::Home | ControlKey::End => {
                self.items.key(key);
            }
            _ => {}
        }
        None
    }
}
#[derive(Clone, Debug, Default)]
pub struct Dialog {
    pub open: bool,
    pub focus: usize,
    return_focus: Option<usize>,
}
impl Dialog {
    pub fn show(&mut self, invoker: usize) {
        self.open = true;
        self.focus = 0;
        self.return_focus = Some(invoker);
    }
    pub fn close(&mut self) -> Option<usize> {
        self.open = false;
        self.return_focus.take()
    }
    pub fn tab(&mut self, reverse: bool, count: usize) {
        if self.open && count > 0 {
            self.focus = if reverse {
                (self.focus + count - 1) % count
            } else {
                (self.focus + 1) % count
            };
        }
    }
}
#[derive(Clone, Debug)]
pub struct Select {
    pub menu: Menu,
    pub selected: usize,
}
impl Select {
    pub fn new(count: usize) -> Self {
        Self {
            menu: Menu::new(count),
            selected: 0,
        }
    }
    pub fn choose(&mut self, i: usize) -> bool {
        if i >= self.menu.items.count {
            return false;
        }
        self.selected = i;
        self.menu.open = false;
        true
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NumberField {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
}
impl NumberField {
    pub fn new(value: f64, min: f64, max: f64, step: f64) -> Option<Self> {
        if ![value, min, max, step].iter().all(|v| v.is_finite()) || min > max || step <= 0.0 {
            return None;
        }
        Some(Self {
            value: value.clamp(min, max),
            min,
            max,
            step,
        })
    }
    pub fn value(self) -> f64 {
        self.value
    }
    /// Whether one configured step can change the value in the requested direction.
    pub fn can_increment(self, up: bool) -> bool {
        if up {
            self.value < self.max
        } else {
            self.value > self.min
        }
    }
    pub fn set(&mut self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let next = value.clamp(self.min, self.max);
        let changed = next != self.value;
        self.value = next;
        changed
    }
    pub fn increment(&mut self, up: bool) -> bool {
        self.set(self.value + if up { self.step } else { -self.step })
    }
}
/// Semantic field metadata, separate from the input's editing model.
#[derive(Clone, Debug, Default)]
pub struct Field {
    pub label: String,
    pub help: String,
    pub error: Option<String>,
    pub required: bool,
}
impl Field {
    pub fn message(&self) -> &str {
        self.error.as_deref().unwrap_or(&self.help)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::{Theme, proposal};

    struct FixedText {
        measured: Size,
        drawn: Option<(Rect, ButtonTextAlign)>,
    }

    struct CheckboxText {
        measured: Size,
        first_line: f32,
        backing_scale: f32,
        drawn: Option<(Rect, Rgba)>,
    }

    impl ButtonTextRenderer for CheckboxText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(self.measured.width.min(max_width), self.measured.height)
        }

        fn backing_scale(&self) -> f32 {
            self.backing_scale
        }

        fn first_line_height(
            &mut self,
            _text: &str,
            _role: proposal::TextRole,
            _max_width: f32,
        ) -> f32 {
            self.first_line
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            rect: Rect,
            _align: ButtonTextAlign,
            ink: Rgba,
        ) {
            self.drawn = Some((rect, ink));
        }
    }

    impl ButtonTextRenderer for FixedText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(self.measured.width.min(max_width), self.measured.height)
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            rect: Rect,
            align: ButtonTextAlign,
            _ink: Rgba,
        ) {
            self.drawn = Some((rect, align));
        }
    }

    #[derive(Default)]
    struct MotionText {
        context: Option<ControlMotionContext>,
        requested_frames: usize,
    }

    impl ButtonTextRenderer for MotionText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, _max_width: f32) -> Size {
            Size::new(48., 20.)
        }

        fn motion_context(&self) -> Option<ControlMotionContext> {
            self.context
        }

        fn request_animation_frame(&mut self) {
            self.requested_frames += 1;
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

    #[test]
    fn button_component_transitions_are_interactive_interruptible_and_reduced_motion_aware() {
        let palette = Palette {
            theme: Theme::LIGHT,
            brand: 0,
        };
        let frame = Rect::new(0., 0., 120., 40.);
        let point = Point::new(60., 20.);
        let mut button = Button::default();
        let mut text = MotionText {
            context: Some(ControlMotionContext {
                now: Duration::ZERO,
                reduced_motion: false,
            }),
            ..Default::default()
        };
        let mut scene = Scene::new();
        button.render(
            &mut scene,
            &mut text,
            "Run",
            frame,
            None,
            palette,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        button.pointer_move_at(point);
        text.context = Some(ControlMotionContext {
            now: Duration::from_millis(1),
            reduced_motion: false,
        });
        scene.clear();
        button.render(
            &mut scene,
            &mut text,
            "Run",
            frame,
            None,
            palette,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        assert!(button.needs_animation());
        assert!(text.requested_frames > 0);
        button.tick(Duration::from_millis(100));
        let hover_midflight = button.hover_transition.value();
        assert!(hover_midflight > 0.0 && hover_midflight < 1.0);

        assert!(button.pointer_down_at(point));
        text.context = Some(ControlMotionContext {
            now: Duration::from_millis(100),
            reduced_motion: false,
        });
        scene.clear();
        button.render(
            &mut scene,
            &mut text,
            "Run",
            frame,
            None,
            palette,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        button.tick(Duration::from_millis(130));
        let press_midflight = button.press_transition.value();
        assert!(press_midflight > 0.0 && press_midflight < 1.0);
        assert!(
            button.pointer_up_at(point),
            "activation stays immediate while paint is transitioning"
        );

        button.set_reduced_motion(true, Duration::from_millis(130));
        assert!(!button.needs_animation());
        assert_eq!(button.press_transition.value(), 0.0);

        button.state.focused = true;
        button.state.focus_visible = true;
        assert!(button.pointer_down_at(point));
        button.render(
            &mut scene,
            &mut text,
            "Run",
            frame,
            Some(Rect::new(500., 500., 1., 1.)),
            palette,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        assert!(
            !button.state.focused,
            "fully clipped controls release focus"
        );
        assert!(
            !button.pointer_up_at(point),
            "a hidden press capture cannot activate"
        );
    }

    #[test]
    fn invalid_button_layout_cannot_keep_keyboard_activation_or_motion() {
        let mut button = Button::default();
        button.state.focused = true;
        button.state.focus_visible = true;
        let mut text = FixedText {
            measured: Size::new(20., 16.),
            drawn: None,
        };
        button.render(
            &mut Scene::new(),
            &mut text,
            "Hidden",
            Rect::new(f32::NAN, 0., 80., 32.),
            None,
            tekes_ui_tokens::Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
        );
        assert!(!button.state.focused);
        assert!(!button.needs_animation());
        assert!(!button.key(ControlKey::Enter, true));
        assert!(!button.key(ControlKey::Space, true));
        assert!(!button.key(ControlKey::Space, false));
    }

    #[test]
    fn button_sizes_consume_their_own_radius_roles_for_base_and_focus() {
        for (size, expected) in [
            (ButtonSize::Xs, 6.),
            (ButtonSize::Sm, 8.),
            (ButtonSize::Md, 8.),
            (ButtonSize::Lg, 12.),
            (ButtonSize::Body, 8.),
        ] {
            let mut text = FixedText {
                measured: Size::new(20., 16.),
                drawn: None,
            };
            let mut scene = Scene::new();
            let mut button = Button::default();
            button.state.focused = true;
            button.state.focus_visible = true;
            let layout = button.render(
                &mut scene,
                &mut text,
                "Label",
                Rect::new(20., 20., 180., 48.),
                None,
                tekes_ui_tokens::Theme::LIGHT,
                crate::paint::ButtonVariant::Quiet,
                size,
                proposal::TextRole::UiBody,
                ButtonTextAlign::default(),
            );
            let quads = scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    tekes_ui_scene::Primitive::Quad(q) => Some(q),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(quads[0].rect, layout.frame);
            assert_eq!(
                quads[0].corner_radii,
                tekes_ui_scene::CornerRadii::uniform(expected)
            );
            assert_eq!(
                quads[1].corner_radii,
                tekes_ui_scene::CornerRadii::uniform(expected + 4.)
            );
            assert!(layout.hit_test(layout.frame.center()));
        }
    }

    #[test]
    fn rendered_button_owns_large_text_layout_clipping_and_hit_geometry() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(88.0, 54.0),
            drawn: None,
        };
        let mut scene = Scene::new();
        let layout = button.render(
            &mut scene,
            &mut text,
            "Large label",
            Rect::new(10.0, 20.0, 140.0, 32.0),
            Some(Rect::new(0.0, 30.0, 300.0, 100.0)),
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );

        assert!(
            layout.frame.size.height
                >= 54.0 + ButtonSize::Md.vertical_inset(proposal::TextRole::UiBody) * 2.0
        );
        assert_eq!(button.layout(), Some(layout));
        assert_eq!(
            layout.visible,
            Some(Rect::new(
                10.0,
                30.0,
                140.0,
                layout.frame.size.height - 10.0
            ))
        );
        assert!(layout.hit_test(Point::new(20.0, 31.0)));
        assert!(!layout.hit_test(Point::new(20.0, 29.0)));
        assert_eq!(text.drawn, Some((layout.content, ButtonTextAlign::Center)));
        assert!(
            scene
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive, Primitive::PushClip(_)))
        );
    }

    #[test]
    fn fractional_button_background_uses_only_the_ancestor_clip() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
            drawn: None,
        };
        let parent = Rect::new(10.0, 10.0, 120.0, 80.0);
        let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
        let mut scene = Scene::new();
        let layout = button.render(
            &mut scene,
            &mut text,
            "Action",
            frame,
            Some(parent),
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
        );

        assert_eq!(layout.frame, frame);
        assert_eq!(layout.visible, Some(frame));
        assert!(layout.hit_test(frame.center()));
        assert!(!layout.hit_test(Point::new(frame.min_x() - 0.01, frame.center().y)));
        assert_eq!(text.drawn, Some((layout.content, ButtonTextAlign::Leading)));
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::PushClip(clip) => Some(*clip),
                _ => None,
            })
            .collect();
        assert_eq!(clips, [parent, frame]);
        assert!(matches!(
            scene.primitives().first(),
            Some(Primitive::PushClip(clip)) if *clip == parent
        ));
        assert!(matches!(
            scene.primitives().get(1),
            Some(Primitive::Quad(quad)) if quad.rect == frame
        ));
    }

    #[test]
    fn button_background_remains_clipped_by_a_real_parent_viewport_edge() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
            drawn: None,
        };
        let parent = Rect::new(30.0, 30.0, 50.0, 50.0);
        let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
        let mut scene = Scene::new();
        let layout = button.render(
            &mut scene,
            &mut text,
            "Action",
            frame,
            Some(parent),
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Md,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
        );

        assert_eq!(layout.visible, Some(Rect::new(30.0, 30.0, 50.0, 30.3125)));
        assert!(!layout.hit_test(Point::new(29.99, 40.0)));
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::PushClip(clip) => Some(*clip),
                _ => None,
            })
            .collect();
        assert_eq!(clips, [parent, layout.visible.unwrap()]);
        assert!(matches!(
            scene.primitives().first(),
            Some(Primitive::PushClip(clip)) if *clip == parent
        ));
        assert!(matches!(
            scene.primitives().get(1),
            Some(Primitive::Quad(quad)) if quad.rect == frame
        ));
    }

    #[test]
    fn checkbox_indicator_uses_parent_clip_while_label_and_hits_keep_row_clip() {
        let mut checkbox = Checkbox::default();
        let mut text = CheckboxText {
            measured: Size::new(40.0, proposal::TextRole::UiBody.style().line_height),
            first_line: proposal::TextRole::UiBody.style().line_height,
            backing_scale: 1.0,
            drawn: None,
        };
        let parent = Rect::new(10.0, 10.0, 120.0, 80.0);
        let frame = Rect::new(24.1875, 24.3125, 80.0, 36.0);
        let mut scene = Scene::new();
        let layout = checkbox.render(
            &mut scene,
            &mut text,
            "Option",
            frame,
            Some(parent),
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );

        assert_eq!(layout.frame, frame);
        assert_eq!(layout.visible, Some(frame));
        assert!(layout.hit_test(frame.center()));
        assert!(!layout.hit_test(Point::new(frame.min_x() - 0.01, frame.center().y)));
        assert!(text.drawn.is_some_and(|(rect, _)| rect == layout.label));
        let clips: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::PushClip(clip) => Some(*clip),
                _ => None,
            })
            .collect();
        assert_eq!(clips, [parent, frame]);
        assert!(matches!(
            scene.primitives().first(),
            Some(Primitive::PushClip(clip)) if *clip == parent
        ));
        assert!(matches!(
            scene.primitives().get(1),
            Some(Primitive::Quad(quad)) if quad.rect == layout.control
        ));
    }

    #[test]
    fn checkbox_indicator_clip_preserves_narrow_rows_with_or_without_ancestor() {
        for width in [0.0, 8.0, 16.0, 220.0] {
            for ancestor in [None, Some(Rect::new(10.0, 10.0, 300.0, 100.0))] {
                let frame = Rect::new(24.1875, 24.3125, width, 36.0);
                let mut checkbox = Checkbox::default();
                let mut text = CheckboxText {
                    measured: Size::new(80.0, proposal::TextRole::UiBody.style().line_height),
                    first_line: proposal::TextRole::UiBody.style().line_height,
                    backing_scale: 1.0,
                    drawn: None,
                };
                let mut scene = Scene::new();
                let layout = checkbox.render(
                    &mut scene,
                    &mut text,
                    "Option",
                    frame,
                    ancestor,
                    Theme::LIGHT,
                    proposal::TextRole::UiBody,
                );

                if width == 0.0 {
                    assert_eq!(layout.visible, None);
                    assert!(scene.is_empty());
                    assert_eq!(text.drawn, None);
                    continue;
                }
                assert_eq!(layout.visible, Some(frame));
                assert!(layout.hit_test(frame.center()));
                let clips: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|primitive| match primitive {
                        Primitive::PushClip(clip) => Some(*clip),
                        _ => None,
                    })
                    .collect();

                if width < 16.0 {
                    // The fixed-size mark overflows a narrow row, so paint is
                    // intentionally contained by the legacy visible row.
                    assert!(matches!(
                        scene.primitives().first(),
                        Some(Primitive::PushClip(clip)) if *clip == frame
                    ));
                    assert_eq!(clips.first(), Some(&frame));
                } else {
                    let expected_mark_clip = ancestor;
                    if let Some(parent) = expected_mark_clip {
                        assert!(matches!(
                            scene.primitives().first(),
                            Some(Primitive::PushClip(clip)) if *clip == parent
                        ));
                        assert_eq!(clips.first(), Some(&parent));
                    } else {
                        assert!(matches!(
                            scene.primitives().first(),
                            Some(Primitive::Quad(_))
                        ));
                        assert_eq!(clips.first(), (width > 16.0).then_some(&frame));
                    }
                    if width == 16.0 {
                        assert_eq!(text.drawn, None);
                    } else {
                        assert!(text.drawn.is_some_and(|(rect, _)| rect == layout.label));
                        assert_eq!(clips.last(), Some(&frame));
                    }
                }
            }
        }
    }

    #[test]
    fn button_measure_keeps_legacy_height_at_default_text_and_expands_for_large_text() {
        let mut text = FixedText {
            measured: Size::new(80.0, proposal::TextRole::ButtonXs.style().line_height),
            drawn: None,
        };
        let default = Button::measure(
            &mut text,
            "Motion action",
            160.0,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
        );
        assert_eq!(
            default.height,
            ButtonSize::Xs.minimum_height().max(
                proposal::TextRole::ButtonXs.style().line_height
                    + ButtonSize::Xs.vertical_inset(proposal::TextRole::ButtonXs) * 2.0
            )
        );
        text.measured.height = proposal::TextRole::ButtonXs.style().line_height * 1.5;
        let large = Button::measure(
            &mut text,
            "Motion action",
            160.0,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
        );
        assert!(large.height > default.height);
        assert!(large.width <= 160.0);
    }

    #[test]
    fn invalid_geometry_and_empty_labels_never_emit_invalid_or_unclipped_text() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(f32::NAN, f32::INFINITY),
            drawn: None,
        };
        let mut scene = Scene::new();
        let invalid = button.render(
            &mut scene,
            &mut text,
            "Invalid",
            Rect::new(f32::NAN, 0.0, 0.0, 20.0),
            None,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
        assert_eq!(invalid.visible, None);
        assert!(scene.is_empty());

        let empty = button.render(
            &mut scene,
            &mut text,
            "",
            Rect::new(10.0, 10.0, 60.0, 26.0),
            None,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
        assert!(empty.frame.min_x().is_finite());
        assert!(empty.frame.size.width.is_finite());
        assert!(text.drawn.is_none());
    }

    #[test]
    fn button_point_interaction_uses_clipped_placement_and_cancels_outside_release() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        button.render(
            &mut Scene::new(),
            &mut text,
            "Action",
            Rect::new(10.0, 20.0, 100.0, 32.0),
            Some(Rect::new(10.0, 20.0, 50.0, 32.0)),
            Theme::LIGHT,
            crate::paint::ButtonVariant::Panel,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Leading,
        );
        assert!(!button.pointer_down_at(Point::new(70.0, 30.0)));
        assert!(button.pointer_down_at(Point::new(20.0, 30.0)));
        assert!(button.state.pressed);
        button.pointer_move_at(Point::new(70.0, 30.0));
        assert!(!button.state.pressed);
        assert!(!button.pointer_up_at(Point::new(70.0, 30.0)));
        assert!(!button.state.pressed);
        assert!(!button.pointer_up_at(Point::new(20.0, 30.0)));
        button.clear_layout();
        assert_eq!(button.layout(), None);
    }

    #[test]
    fn stationary_pointer_rehits_after_layout_translation_and_restoration() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        let mut scene = Scene::new();
        let render = |button: &mut Button,
                      frame: Rect,
                      clip: Option<Rect>,
                      text: &mut FixedText,
                      scene: &mut Scene| {
            button.render(
                scene,
                text,
                "Animated",
                frame,
                clip,
                Theme::LIGHT,
                crate::paint::ButtonVariant::Quiet,
                ButtonSize::Xs,
                proposal::TextRole::ButtonXs,
                ButtonTextAlign::Center,
            );
        };

        render(
            &mut button,
            Rect::new(10.0, 10.0, 100.0, 30.0),
            None,
            &mut text,
            &mut scene,
        );
        assert!(button.pointer_down_at(Point::new(20.0, 20.0)));
        assert!(button.state.hovered && button.state.pressed);

        render(
            &mut button,
            Rect::new(120.0, 10.0, 100.0, 30.0),
            None,
            &mut text,
            &mut scene,
        );
        assert!(!button.state.hovered && !button.state.pressed);

        render(
            &mut button,
            Rect::new(10.0, 10.0, 100.0, 30.0),
            None,
            &mut text,
            &mut scene,
        );
        assert!(button.state.hovered && button.state.pressed);
        assert!(button.pointer_up_at(Point::new(20.0, 20.0)));
    }

    #[test]
    fn stationary_pointer_hover_tracks_new_ancestor_clip() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        let mut scene = Scene::new();
        let frame = Rect::new(10.0, 10.0, 100.0, 30.0);
        let render = |button: &mut Button, clip, text: &mut FixedText, scene: &mut Scene| {
            button.render(
                scene,
                text,
                "Clipped",
                frame,
                clip,
                Theme::LIGHT,
                crate::paint::ButtonVariant::Quiet,
                ButtonSize::Xs,
                proposal::TextRole::ButtonXs,
                ButtonTextAlign::Center,
            );
        };
        render(&mut button, None, &mut text, &mut scene);
        button.pointer_move_at(Point::new(20.0, 20.0));
        assert!(button.state.hovered);
        render(
            &mut button,
            Some(Rect::new(50.0, 10.0, 60.0, 30.0)),
            &mut text,
            &mut scene,
        );
        assert!(!button.state.hovered);
        render(
            &mut button,
            Some(Rect::new(10.0, 10.0, 100.0, 30.0)),
            &mut text,
            &mut scene,
        );
        assert!(button.state.hovered);
    }

    #[test]
    fn leading_and_centered_label_positions_are_component_owned() {
        let mut leading = Button::default();
        let mut centered = Button::default();
        for (button, align) in [
            (&mut leading, ButtonTextAlign::Leading),
            (&mut centered, ButtonTextAlign::Center),
        ] {
            let mut text = FixedText {
                measured: Size::new(40.0, 16.0),
                drawn: None,
            };
            button.render(
                &mut Scene::new(),
                &mut text,
                "Label",
                Rect::new(10.0, 20.0, 140.0, 32.0),
                None,
                Theme::LIGHT,
                crate::paint::ButtonVariant::Panel,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
                align,
            );
            let (drawn, actual_align) = text.drawn.unwrap();
            assert_eq!(actual_align, align);
            assert_eq!(drawn.min_x(), 22.0);
            assert_eq!(drawn.size.width, 140.0 - ButtonSize::Body.inset() * 2.0);
        }
    }

    #[test]
    fn button_release_contract_and_disabled_loading() {
        let mut b = Button::default();
        b.pointer_down();
        assert!(!b.pointer_up(false));
        b.pointer_down();
        assert!(b.pointer_up(true));
        assert!(!b.pointer_up(true));
        b.state.focused = true;
        assert!(!b.key(ControlKey::Space, true));
        assert!(b.key(ControlKey::Space, false));
        for loading in [false, true] {
            b.state.loading = loading;
            b.state.disabled = !loading;
            b.pointer_down();
            assert!(!b.pointer_up(true));
            assert!(!b.key(ControlKey::Enter, true));
        }
    }

    #[test]
    fn keyboard_and_pointer_arms_cannot_release_or_activate_each_other() {
        let mut button = Button::default();
        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        assert!(!button.key(ControlKey::Space, true)); // key repeat does not re-arm
        assert!(button.state.pressed);
        assert!(!button.pointer_up(true));
        assert!(button.state.pressed);
        assert!(button.key(ControlKey::Space, false));
        assert!(!button.key(ControlKey::Space, false)); // one activation per press

        button.pointer_down();
        assert!(!button.key(ControlKey::Space, false));
        assert!(button.state.pressed);
        assert!(button.pointer_up(true));

        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        button.blur();
        assert!(!button.key(ControlKey::Space, false));
        assert!(!button.state.pressed);

        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        button.state.disabled = true;
        assert!(!button.key(ControlKey::Space, false));
        assert!(!button.state.pressed);

        button.state.disabled = false;
        button.pointer_down();
        button.state.loading = true;
        assert!(!button.pointer_up(true));
        assert!(!button.state.pressed);
    }

    #[test]
    fn pointer_cancellation_preserves_keyboard_space_arm() {
        let mut button = Button::default();
        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        button.cancel_pointer();
        assert!(button.state.pressed);
        assert!(button.key(ControlKey::Space, false));

        button.pointer_down();
        assert!(button.state.pressed);
        button.cancel_pointer();
        assert!(!button.state.pressed);
        assert!(!button.pointer_up(true));
    }

    #[test]
    fn render_cancels_pointer_and_space_arms_when_disabled_or_loading() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        let frame = Rect::new(10.0, 10.0, 100.0, 30.0);
        let render = |button: &mut Button, text: &mut FixedText| {
            button.render(
                &mut Scene::new(),
                text,
                "Action",
                frame,
                None,
                Theme::LIGHT,
                crate::paint::ButtonVariant::Quiet,
                ButtonSize::Xs,
                proposal::TextRole::ButtonXs,
                ButtonTextAlign::Center,
            );
        };

        render(&mut button, &mut text);
        for loading in [false, true] {
            button.state.disabled = false;
            button.state.loading = false;
            render(&mut button, &mut text);
            assert!(button.pointer_down_at(Point::new(20.0, 20.0)));
            button.state.disabled = !loading;
            button.state.loading = loading;
            render(&mut button, &mut text);
            button.state.disabled = false;
            button.state.loading = false;
            render(&mut button, &mut text);
            assert!(!button.pointer_up_at(Point::new(20.0, 20.0)));

            button.state.focused = true;
            assert!(!button.key(ControlKey::Space, true));
            button.state.disabled = !loading;
            button.state.loading = loading;
            render(&mut button, &mut text);
            button.state.disabled = false;
            button.state.loading = false;
            render(&mut button, &mut text);
            assert!(!button.key(ControlKey::Space, false));
            assert!(!button.state.pressed);
        }
    }

    #[test]
    fn render_cancels_space_arm_when_controlled_focus_is_lost() {
        let mut button = Button::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        let render = |button: &mut Button, text: &mut FixedText| {
            button.render(
                &mut Scene::new(),
                text,
                "Action",
                Rect::new(10.0, 10.0, 100.0, 30.0),
                None,
                Theme::LIGHT,
                crate::paint::ButtonVariant::Quiet,
                ButtonSize::Xs,
                proposal::TextRole::ButtonXs,
                ButtonTextAlign::Center,
            );
        };
        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        button.state.focused = false;
        render(&mut button, &mut text);
        button.state.focused = true;
        render(&mut button, &mut text);
        assert!(!button.key(ControlKey::Space, false));
        assert!(!button.state.pressed);
    }
    #[test]
    fn mixed_checkbox_and_disabled_combination() {
        let mut c = Checkbox {
            checked: CheckState::Mixed,
            ..Default::default()
        };
        assert!(c.toggle());
        assert_eq!(c.checked, CheckState::Checked);
        c.button.state.disabled = true;
        assert!(!c.toggle());
        assert_eq!(c.checked, CheckState::Checked);
    }

    #[test]
    fn checkbox_renders_and_hits_the_same_clipped_row_with_centered_default_text() {
        let mut checkbox = Checkbox::default();
        let mut text = FixedText {
            measured: Size::new(172.0, 18.0),
            drawn: None,
        };
        let mut scene = Scene::new();
        let layout = checkbox.render(
            &mut scene,
            &mut text,
            "Receive project notifications",
            Rect::new(10.0, 20.0, 400.0, 36.0),
            Some(Rect::new(10.0, 20.0, 220.0, 36.0)),
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert_eq!(layout.frame, Rect::new(10.0, 20.0, 400.0, 36.0));
        assert_eq!(layout.control, Rect::new(10.0, 30.0, 16.0, 16.0));
        assert_eq!(layout.label, Rect::new(32.0, 29.0, 378.0, 18.0));
        assert_eq!(layout.visible, Some(Rect::new(10.0, 20.0, 220.0, 36.0)));
        assert_eq!(text.drawn, Some((layout.label, ButtonTextAlign::Leading)));
        assert!(checkbox.pointer_down_at(Point::new(200.0, 30.0)));
        assert!(checkbox.button.state.pressed);
        assert!(!checkbox.pointer_up_at(Point::new(230.0, 30.0)));
        assert_eq!(checkbox.checked, CheckState::Unchecked);
        assert!(checkbox.pointer_down_at(Point::new(200.0, 30.0)));
        assert!(checkbox.pointer_up_at(Point::new(200.0, 30.0)));
        assert_eq!(checkbox.checked, CheckState::Checked);
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::PushClip(_)))
        );
    }

    #[test]
    fn checkbox_expands_for_large_text_and_cancels_stale_hit_when_disabled_or_cleared() {
        let mut checkbox = Checkbox::default();
        let mut text = FixedText {
            measured: Size::new(180.0, 48.0),
            drawn: None,
        };
        let layout = checkbox.render(
            &mut Scene::new(),
            &mut text,
            "Large project notification label",
            Rect::new(8.0, 12.0, 400.0, 36.0),
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert_eq!(layout.frame.size.height, 60.0);
        assert_eq!(layout.control, Rect::new(8.0, 34.0, 16.0, 16.0));
        assert_eq!(layout.label, Rect::new(30.0, 18.0, 378.0, 48.0));
        assert!(checkbox.pointer_down_at(Point::new(100.0, 20.0)));
        checkbox.button.state.disabled = true;
        checkbox.render(
            &mut Scene::new(),
            &mut text,
            "Large project notification label",
            Rect::new(8.0, 12.0, 400.0, 36.0),
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert!(!checkbox.button.state.pressed);
        assert!(!checkbox.pointer_down_at(Point::new(100.0, 20.0)));
        checkbox.clear_layout();
        assert_eq!(checkbox.layout(), None);
        assert!(!checkbox.pointer_up_at(Point::new(100.0, 20.0)));
    }

    #[test]
    fn checkbox_first_line_center_label_gap_and_row_height_scale_naturally() {
        for backing_scale in [1.0, 2.0] {
            for (type_scale, first_line, label_height, expected_row_height) in [
                (1.0, 18.0, 18.0, 36.0),
                (1.5, 27.0, 54.0, 66.0),
                (2.0, 36.0, 108.0, 120.0),
            ] {
                let mut checkbox = Checkbox::default();
                let mut text = CheckboxText {
                    measured: Size::new(320.0, label_height),
                    first_line,
                    backing_scale,
                    drawn: None,
                };
                let frame = Rect::new(12.0, 20.0, 220.0, 36.0);
                let layout = checkbox.render(
                    &mut Scene::new(),
                    &mut text,
                    "Notifications with wrapped supporting text",
                    frame,
                    None,
                    Theme::LIGHT,
                    proposal::TextRole::UiBody,
                );

                assert_eq!(layout.control.size, Size::new(16.0, 16.0));
                assert_eq!(layout.label.min_x(), 34.0, "16pt mark + 6pt gap");
                assert_eq!(layout.frame.size.height, expected_row_height);
                assert!(
                    (layout.control.center().y - (layout.label.min_y() + first_line * 0.5)).abs()
                        < 0.001,
                    "first line and visual box must share a center at {type_scale}x / {backing_scale}x"
                );
                assert!(
                    layout.hit_test(Point::new(frame.min_x() + 2.0, layout.frame.center().y)),
                    "the entire row remains the hit target"
                );
                assert_eq!(text.drawn.unwrap().0, layout.label);
            }
        }
    }

    #[test]
    fn checkbox_label_uses_semantic_text_ink_for_all_mark_and_control_states() {
        let palettes = [
            0,
            tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.id == "example-blue")
                .expect("blue example preset")
                .palette_index,
            tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.id == "example-clay")
                .expect("clay example preset")
                .palette_index,
        ];
        let states = [
            (CheckState::Unchecked, ControlState::default()),
            (
                CheckState::Checked,
                ControlState {
                    hovered: true,
                    ..Default::default()
                },
            ),
            (
                CheckState::Checked,
                ControlState {
                    pressed: true,
                    ..Default::default()
                },
            ),
            (
                CheckState::Mixed,
                ControlState {
                    focused: true,
                    focus_visible: true,
                    ..Default::default()
                },
            ),
            (
                CheckState::Unchecked,
                ControlState {
                    disabled: true,
                    ..Default::default()
                },
            ),
            (
                CheckState::Checked,
                ControlState {
                    disabled: true,
                    hovered: true,
                    ..Default::default()
                },
            ),
            (
                CheckState::Mixed,
                ControlState {
                    disabled: true,
                    focused: true,
                    focus_visible: true,
                    ..Default::default()
                },
            ),
        ];

        for theme in Theme::ALL {
            for brand in palettes {
                let palette = tekes_ui_tokens::Palette { theme, brand };
                for (checked, state) in states {
                    let mut checkbox = Checkbox {
                        button: Button {
                            state,
                            ..Default::default()
                        },
                        checked,
                    };
                    let mut text = CheckboxText {
                        measured: Size::new(120.0, 18.0),
                        first_line: 18.0,
                        backing_scale: 1.0,
                        drawn: None,
                    };
                    checkbox.render(
                        &mut Scene::new(),
                        &mut text,
                        "Notifications",
                        Rect::new(0.0, 0.0, 200.0, 36.0),
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                    );
                    let actual = text.drawn.expect("checkbox label is drawn").1;
                    let role = if state.disabled {
                        "tertiary-text"
                    } else {
                        "primary-text"
                    };
                    assert_eq!(
                        actual,
                        palette.color(role).expect("semantic label role"),
                        "label ink must remain semantic for {checked:?}, {state:?}, brand {brand}, theme {theme:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn checkbox_keyboard_activation_uses_the_shared_button_space_contract() {
        let mut checkbox = Checkbox::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 18.0),
            drawn: None,
        };
        checkbox.render(
            &mut Scene::new(),
            &mut text,
            "Notifications",
            Rect::new(0.0, 0.0, 200.0, 36.0),
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert!(!checkbox.key(ControlKey::Enter, true));
        checkbox.button.state.focused = true;
        checkbox.button.state.focus_visible = true;
        assert!(!checkbox.key(ControlKey::Space, true));
        assert!(checkbox.button.state.pressed);
        assert!(checkbox.key(ControlKey::Space, false));
        assert_eq!(checkbox.checked, CheckState::Checked);
        checkbox.button.state.disabled = true;
        assert!(!checkbox.key(ControlKey::Space, true));
        assert_eq!(checkbox.checked, CheckState::Checked);
    }

    #[test]
    fn checkbox_pointer_capture_survives_blur_but_keyboard_space_arm_does_not() {
        let mut checkbox = Checkbox::default();
        let mut text = FixedText {
            measured: Size::new(40.0, 18.0),
            drawn: None,
        };
        let mut scene = Scene::new();
        let frame = Rect::new(0.0, 0.0, 200.0, 36.0);
        checkbox.render(
            &mut scene,
            &mut text,
            "Notifications",
            frame,
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        let point = Point::new(120.0, 18.0);
        assert!(checkbox.pointer_down_at(point));
        checkbox.button.state.focused = false;
        checkbox.render(
            &mut scene,
            &mut text,
            "Notifications",
            frame,
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert!(checkbox.button.state.pressed);
        assert!(checkbox.pointer_up_at(point));
        assert_eq!(checkbox.checked, CheckState::Checked);

        checkbox.button.state.focused = true;
        assert!(!checkbox.key(ControlKey::Space, true));
        assert!(checkbox.button.state.pressed);
        checkbox.button.state.focused = false;
        checkbox.render(
            &mut scene,
            &mut text,
            "Notifications",
            frame,
            None,
            Theme::LIGHT,
            proposal::TextRole::UiBody,
        );
        assert!(!checkbox.button.state.pressed);
        checkbox.button.state.focused = true;
        assert!(!checkbox.key(ControlKey::Space, false));
        assert_eq!(checkbox.checked, CheckState::Checked);
    }
    #[test]
    fn tab_menu_boundaries_and_escape() {
        let mut t = Tabs::new(3);
        t.key(ControlKey::Left);
        assert_eq!(t.selected, 2);
        t.key(ControlKey::Home);
        assert_eq!(t.selected, 0);
        let mut m = Menu::new(3);
        m.show();
        m.key(ControlKey::Down);
        assert_eq!(m.key(ControlKey::Enter), Some(1));
        assert!(!m.open);
        m.show();
        m.key(ControlKey::Escape);
        assert!(!m.open);
        assert!(!Tabs::new(0).key(ControlKey::End));
    }
    #[test]
    fn modal_traps_focus_and_restores_invoker() {
        let mut d = Dialog::default();
        d.show(7);
        d.tab(true, 2);
        assert_eq!(d.focus, 1);
        d.tab(false, 2);
        assert_eq!(d.focus, 0);
        assert_eq!(d.close(), Some(7));
        assert_eq!(d.close(), None);
    }
    #[test]
    fn numeric_clamps_and_rejects_invalid_values() {
        assert!(NumberField::new(0., 1., 0., 1.).is_none());
        let mut n = NumberField::new(99., 0., 10., 1.).unwrap();
        assert_eq!(n.value(), 10.);
        assert!(!n.increment(true));
        assert!(n.increment(false));
        assert!(!n.set(f64::NAN));
        assert_eq!(n.value(), 9.);
    }
}
/// Toolbar geometry is shared by clients; narrow bounds wrap rather than overflow.
#[derive(Clone, Copy, Debug, Default)]
pub struct Toolbar;
impl Toolbar {
    pub fn layout(
        bounds: tekes_ui_scene::Rect,
        count: usize,
        gap: f32,
    ) -> Vec<tekes_ui_scene::Rect> {
        if count == 0 {
            return Vec::new();
        }
        let width = ((bounds.size.width - gap * (count - 1) as f32) / count as f32).max(0.);
        (0..count)
            .map(|i| {
                tekes_ui_scene::Rect::new(
                    bounds.min_x() + i as f32 * (width + gap),
                    bounds.min_y(),
                    width,
                    bounds.size.height,
                )
            })
            .collect()
    }
}
