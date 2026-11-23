use super::{button_text_align, clipped, draw_centered_text_with_rgba};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, Field, NumberField,
    paint,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

/// Field metadata and input slot geometry; the actual input model remains the
/// shared TextInput/TextSurface host adapter.
#[derive(Clone, Debug, Default)]
pub struct FieldView {
    layout: FieldLayout,
    surface: Option<&'static str>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FieldLayout {
    pub frame: Rect,
    pub label: Rect,
    pub required_mark: Option<Rect>,
    pub slot: Rect,
    pub message: Rect,
    pub has_error: bool,
    pub visible: Option<Rect>,
}
/// Optional typography and spacing for embedding a Field in a composed form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldPresentation {
    pub label_role: proposal::TextRole,
    pub label_min_height: f32,
    pub message_role: Option<proposal::TextRole>,
    pub label_to_slot_gap: f32,
    pub slot_to_message_gap: f32,
    pub slot_min_height: f32,
    /// Exact viewport height for an explicitly constrained composed field.
    /// None retains the existing intrinsic single-line/minimum-height behavior.
    pub slot_height: Option<f32>,
    pub message_min_height: f32,
}
impl FieldPresentation {
    /// Emphasized heading label, caption helper and shared row-height form slots.
    pub fn emphasized_form() -> Self {
        Self {
            label_role: proposal::TextRole::UiHeading,
            label_min_height: 0.,
            message_role: Some(proposal::TextRole::UiCaption),
            label_to_slot_gap: proposal::metric("spacing", "space-tight").unwrap_or(4.),
            slot_to_message_gap: proposal::metric("spacing", "space-snug").unwrap_or(6.),
            slot_min_height: proposal::metric("size", "size-row-height").unwrap_or(32.),
            slot_height: None,
            message_min_height: 0.,
        }
    }
}
impl Default for FieldPresentation {
    fn default() -> Self {
        Self {
            label_role: proposal::TextRole::UiBody,
            label_min_height: 0.,
            message_role: None,
            label_to_slot_gap: proposal::metric("spacing", "space-tight").unwrap_or(4.),
            slot_to_message_gap: proposal::metric("spacing", "space-tight").unwrap_or(4.),
            slot_min_height: 0.,
            slot_height: None,
            message_min_height: 0.,
        }
    }
}
impl FieldView {
    pub fn surface(&self) -> &'static str {
        self.surface.unwrap_or("canvas")
    }

    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(
            surface,
        )?);
        Ok(())
    }

    pub fn measure_layout(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Field,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
    ) -> FieldLayout {
        self.measure_with_presentation(text, model, frame, clip, role, FieldPresentation::default())
    }

    pub fn measure_with_presentation(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Field,
        frame: Rect,
        clip: Option<Rect>,
        role: proposal::TextRole,
        presentation: FieldPresentation,
    ) -> FieldLayout {
        // Reserve the required indicator before shaping a wrapping label. Its
        // width must not depend on whatever room the full-width label leaves.
        let width = frame.size.width.max(0.);
        let mark_w = if model.required {
            text.measure("*", presentation.label_role, f32::MAX)
                .width
                .max(0.)
                .min(width)
        } else {
            0.
        };
        let label_width = (width - mark_w).max(0.);
        let label_size = text.measure(&model.label, presentation.label_role, label_width);
        let label_h = label_size
            .height
            .max(presentation.label_role.style().line_height)
            .max(presentation.label_min_height.max(0.));
        let message = model.message();
        let message_role = presentation.message_role.unwrap_or(role);
        let message_h = if message.is_empty() {
            0.
        } else {
            text.measure(message, message_role, frame.size.width.max(0.))
                .height
                .max(message_role.style().line_height)
                .max(presentation.message_min_height.max(0.))
        };
        let measured_line = text.measure("M", role, f32::MAX).height.max(0.);
        let input_h = proposal::metric("size", "size-button-md")
            .unwrap_or(32.)
            .max(measured_line + 2. * proposal::SCROLL_SINGLE_LINE_PADDING_Y)
            .max(presentation.slot_min_height.max(0.));
        let input_h = presentation
            .slot_height
            .filter(|height| height.is_finite())
            .map_or(input_h, |height| height.max(0.));
        let slot_y = frame.min_y() + label_h + presentation.label_to_slot_gap.max(0.);
        let slot = Rect::new(
            frame.min_x(),
            slot_y,
            frame.size.width.max(0.),
            input_h.max(0.),
        );
        let msg_gap = if message.is_empty() {
            0.
        } else {
            presentation.slot_to_message_gap.max(0.)
        };
        let msg_y = slot.max_y() + msg_gap;
        let message_rect = Rect::new(frame.min_x(), msg_y, frame.size.width.max(0.), message_h);
        let visible = clipped(frame, clip);
        let label_rect = Rect::new(frame.min_x(), frame.min_y(), label_width, label_h);
        let required_mark = if model.required {
            Some(Rect::new(
                label_rect.min_x() + label_size.width.max(0.).min(label_width),
                label_rect.min_y(),
                mark_w,
                label_h,
            ))
        } else {
            None
        };
        self.layout = FieldLayout {
            frame,
            label: label_rect,
            required_mark,
            slot,
            message: message_rect,
            has_error: model.error.is_some(),
            visible,
        };
        self.layout
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Field,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> FieldLayout {
        self.render_with_presentation(
            scene,
            text,
            model,
            frame,
            clip,
            palette,
            role,
            FieldPresentation::default(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_presentation(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Field,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        presentation: FieldPresentation,
    ) -> FieldLayout {
        let layout = self.measure_with_presentation(text, model, frame, clip, role, presentation);
        let message = model.message();
        let message_role = presentation.message_role.unwrap_or(role);
        if let Some(v) = layout.visible {
            let surface = self.surface();
            let label_ink = tekes_ui_tokens::surface_palette::resolve_surface(
                palette, surface, "content", "rest",
            )
            .expect("Field label Surface packet")
            .text;
            let draw_top_aligned = |scene: &mut Scene,
                                    text: &mut dyn ButtonTextRenderer,
                                    value: &str,
                                    role: proposal::TextRole,
                                    rect: Rect| {
                if value.is_empty() || rect.is_empty() {
                    return;
                }
                let measured = text.measure(value, role, rect.size.width.max(0.));
                let draw_rect = Rect::new(
                    rect.min_x(),
                    rect.min_y(),
                    rect.size.width,
                    rect.size.height.min(measured.height.max(0.)),
                );
                if !draw_rect.is_empty() {
                    text.draw(
                        scene,
                        value,
                        role,
                        draw_rect,
                        ButtonTextAlign::Leading,
                        label_ink,
                    );
                }
            };
            scene.push_clip(v);
            draw_top_aligned(
                scene,
                text,
                &model.label,
                presentation.label_role,
                layout.label,
            );
            if let Some(r) = layout.required_mark {
                draw_top_aligned(scene, text, "*", presentation.label_role, r);
            }
            if !message.is_empty() {
                let packet = tekes_ui_tokens::surface_palette::resolve_surface_with_options(
                    palette,
                    surface,
                    "input",
                    "rest",
                    tekes_ui_tokens::surface_palette::SurfaceOptions {
                        error: model.error.is_some(),
                        read_only: false,
                    },
                )
                .expect("Field helper Surface packet");
                draw_centered_text_with_rgba(
                    scene,
                    text,
                    message,
                    message_role,
                    layout.message,
                    ButtonTextAlign::Leading,
                    packet.helper.expect("Field helper ink"),
                );
            }
            scene.pop_clip();
        }
        layout
    }
    pub fn layout(&self) -> FieldLayout {
        self.layout
    }
}

/// Existing bounded stepper presentation; it intentionally does not expose text
/// editing or locale parsing.
#[derive(Clone, Debug, Default)]
pub struct NumberFieldView {
    minus: ButtonModel,
    plus: ButtonModel,
    layout: NumberFieldLayout,
    disabled: bool,
    focused: Option<bool>,
    surface: Option<&'static str>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberFieldLayout {
    pub frame: Rect,
    pub decrement: crate::ButtonLayout,
    pub value: Rect,
    pub increment: crate::ButtonLayout,
    pub visible: Option<Rect>,
}
impl Default for NumberFieldLayout {
    fn default() -> Self {
        Self {
            frame: Rect::default(),
            decrement: crate::ButtonLayout {
                frame: Rect::default(),
                content: Rect::default(),
                visible: None,
                label_size: Size::default(),
            },
            value: Rect::default(),
            increment: crate::ButtonLayout {
                frame: Rect::default(),
                content: Rect::default(),
                visible: None,
                label_size: Size::default(),
            },
            visible: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberFieldEvent {
    None,
    Changed,
}
impl NumberFieldView {
    pub fn surface(&self) -> &'static str {
        self.surface.unwrap_or("canvas")
    }
    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = Some(tekes_ui_tokens::surface_palette::canonical_surface(
            surface,
        )?);
        Ok(())
    }
    pub(crate) fn slot_frames(frame: Rect) -> (Rect, Rect, Rect) {
        let nominal_gap = tekes_ui_tokens::Spacing::Regular.value();
        let button_nominal =
            proposal::metric("size", "size-row-height").expect("stepper button width");
        let value_min = button_nominal;
        let nominal = button_nominal * 2. + nominal_gap * 2. + value_min;
        let width = frame.size.width.max(0.);
        let (button_w, gap, value_w) = if width >= nominal {
            (
                button_nominal,
                nominal_gap,
                (width - button_nominal * 2. - nominal_gap * 2.).max(0.),
            )
        } else {
            let scale = width / nominal;
            (
                button_nominal * scale,
                nominal_gap * scale,
                value_min * scale,
            )
        };
        let height = frame.size.height.max(0.);
        let minus = Rect::new(frame.min_x(), frame.min_y(), button_w, height);
        let value = Rect::new(minus.max_x() + gap, frame.min_y(), value_w, height);
        let plus = Rect::new(value.max_x() + gap, frame.min_y(), button_w, height);
        (minus, value, plus)
    }
    fn sync_model(&mut self, model: &NumberField) {
        for (button, increment) in [(&mut self.minus, false), (&mut self.plus, true)] {
            button.state.disabled = self.disabled || !model.can_increment(increment);
            if button.state.disabled {
                button.cancel();
            }
        }
    }
    pub fn set_disabled(&mut self, d: bool) {
        self.disabled = d;
        if d {
            self.blur();
        }
    }
    /// Focus one stepper child (`increment = false` selects decrement).
    pub fn focus(&mut self, increment: bool, visible: bool) {
        if self.disabled {
            self.blur();
            return;
        }
        self.focused = Some(increment);
        self.minus.state.focused = !increment;
        self.minus.state.focus_visible = !increment && visible;
        self.plus.state.focused = increment;
        self.plus.state.focus_visible = increment && visible;
        if increment {
            self.minus.cancel();
        } else {
            self.plus.cancel();
        }
    }
    pub fn focused_index(&self) -> Option<bool> {
        self.focused
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.minus.blur();
        self.plus.blur();
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &NumberField,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> NumberFieldLayout {
        let (minus, value, plus) = Self::slot_frames(frame);
        let surface = self.surface();
        self.sync_model(model);
        self.minus.set_surface(surface);
        self.plus.set_surface(surface);
        self.minus.state.focused = self.focused == Some(false);
        self.plus.state.focused = self.focused == Some(true);
        // The stepper owns these allocated hit slots, including narrow widths.
        // Do not apply ButtonModel's minimum/insets again after typography scales.
        self.minus.frame_policy = crate::ButtonFramePolicy::Exact;
        self.plus.frame_policy = crate::ButtonFramePolicy::Exact;
        let d = self.minus.render_transparent_label(
            scene,
            text,
            "−",
            minus,
            clip,
            palette,
            paint::ButtonVariant::Quiet,
            ButtonSize::Body,
            role,
            button_text_align(proposal::layout::BUTTON_CONTENT),
        );
        let i = self.plus.render_transparent_label(
            scene,
            text,
            "+",
            plus,
            clip,
            palette,
            paint::ButtonVariant::Quiet,
            ButtonSize::Body,
            role,
            button_text_align(proposal::layout::BUTTON_CONTENT),
        );
        let visible = clipped(frame, clip);
        if let Some(v) = visible {
            let value_ink = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                surface,
                "content",
                if self.disabled { "disabled" } else { "rest" },
            )
            .expect("NumberField value Surface packet")
            .text;
            scene.push_clip(v);
            draw_centered_text_with_rgba(
                scene,
                &mut *text,
                &model.value().to_string(),
                proposal::TextRole::UiHeading,
                value,
                ButtonTextAlign::Center,
                value_ink,
            );
            scene.pop_clip();
        }
        self.layout = NumberFieldLayout {
            frame,
            decrement: d,
            value,
            increment: i,
            visible,
        };
        self.layout
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.minus.pointer_move_at(p);
        self.plus.pointer_move_at(p)
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<bool> {
        if self.layout.decrement.hit_test(p) && self.minus.pointer_down_at(p) {
            Some(false)
        } else if self.layout.increment.hit_test(p) && self.plus.pointer_down_at(p) {
            Some(true)
        } else {
            None
        }
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut NumberField) -> NumberFieldEvent {
        self.sync_model(model);
        let increment = if self.minus.pointer_up_at(p) {
            Some(false)
        } else if self.plus.pointer_up_at(p) {
            Some(true)
        } else {
            None
        };
        if let Some(increment) = increment
            && !self.disabled
            && model.can_increment(increment)
            && model.increment(increment)
        {
            self.sync_model(model);
            NumberFieldEvent::Changed
        } else {
            NumberFieldEvent::None
        }
    }
    pub fn key(&mut self, model: &mut NumberField, key: ControlKey) -> NumberFieldEvent {
        self.sync_model(model);
        let up = matches!(key, ControlKey::Up | ControlKey::Right);
        let down = matches!(key, ControlKey::Down | ControlKey::Left);
        if !self.disabled && (up || down) && model.increment(up) {
            self.sync_model(model);
            NumberFieldEvent::Changed
        } else {
            NumberFieldEvent::None
        }
    }
    pub fn key_event(
        &mut self,
        model: &mut NumberField,
        key: ControlKey,
        down: bool,
    ) -> NumberFieldEvent {
        self.sync_model(model);
        if self.disabled {
            self.cancel();
            return NumberFieldEvent::None;
        }
        let Some(increment) = self.focused else {
            return NumberFieldEvent::None;
        };
        let button = if increment {
            &mut self.plus
        } else {
            &mut self.minus
        };
        if button.key(key, down) && model.can_increment(increment) && model.increment(increment) {
            self.sync_model(model);
            NumberFieldEvent::Changed
        } else {
            NumberFieldEvent::None
        }
    }
    pub fn cancel(&mut self) {
        self.minus.cancel();
        self.plus.cancel();
    }
    pub fn layout(&self) -> NumberFieldLayout {
        self.layout
    }
}
