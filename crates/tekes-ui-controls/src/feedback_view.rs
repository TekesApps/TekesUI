//! Complete presentation adapters for the existing surface, field, overlay and
//! feedback models. These views own geometry, clipping, hit targets and input
//! state; hosts supply shaped text and forward elapsed fixture time.
use crate::extended::{
    Banner, EmptyState, NoticeTone, Popover, Progress, StatusBadge, Toast, Tooltip,
};
use crate::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey, Field,
    Insets, NumberField, Toolbar, paint,
};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

fn finite_rect(r: Rect) -> bool {
    r.min_x().is_finite()
        && r.min_y().is_finite()
        && r.size.width.is_finite()
        && r.size.height.is_finite()
        && r.max_x().is_finite()
        && r.max_y().is_finite()
        && r.size.width >= 0.
        && r.size.height >= 0.
}
fn clipped(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
    if !finite_rect(frame) || clip.is_some_and(|c| !finite_rect(c)) {
        return None;
    }
    ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, clip).visible_clip()
}
fn paint_surface_background(
    scene: &mut Scene,
    frame: Rect,
    ancestor_clip: Option<Rect>,
    visible: Option<Rect>,
    palette: Palette,
    kind: paint::SurfaceKind,
    backing_scale: f32,
) {
    if visible.is_none() {
        return;
    }
    let layout = ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, ancestor_clip);
    if let Some(clip) = layout.background_paint_clip() {
        scene.push_clip(clip);
    }
    paint::surface_kind_at_scale(scene, frame, palette, kind, backing_scale);
    if layout.background_paint_clip().is_some() {
        scene.pop_clip();
    }
}
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    if !finite_rect(a) || !finite_rect(b) {
        return None;
    }
    a.intersection(&b).filter(|r| !r.is_empty())
}
fn effective_viewport(viewport: Rect, clip: Option<Rect>) -> Rect {
    match clip {
        Some(clip) => intersect(viewport, clip).unwrap_or_default(),
        None if finite_rect(viewport) => viewport,
        None => Rect::default(),
    }
}
fn safe_inset(r: Rect, x: f32, y: f32) -> Rect {
    let x = x.max(0.).min(r.size.width * 0.5);
    let y = y.max(0.).min(r.size.height * 0.5);
    Rect::new(
        r.min_x() + x,
        r.min_y() + y,
        (r.size.width - 2. * x).max(0.),
        (r.size.height - 2. * y).max(0.),
    )
}
fn draw_text(
    scene: &mut Scene,
    renderer: &mut impl ButtonTextRenderer,
    text: &str,
    role: proposal::TextRole,
    rect: Rect,
    palette: Palette,
    align: ButtonTextAlign,
) {
    if text.is_empty() || rect.is_empty() {
        return;
    }
    // `ButtonTextRenderer::draw` vertically centers within its rectangle,
    // while the former Gallery label helper aligned wrapped text to the top.
    // Give it the measured text height so component slots retain that contract.
    let measured = renderer.measure(text, role, rect.size.width.max(0.));
    let draw_rect = Rect::new(
        rect.min_x(),
        rect.min_y(),
        rect.size.width,
        rect.size.height.min(measured.height.max(0.)),
    );
    if draw_rect.is_empty() {
        return;
    }
    let ink = palette.color("primary-text").expect("semantic text token");
    renderer.draw(scene, text, role, draw_rect, align, ink);
}
#[allow(clippy::too_many_arguments)]
fn draw_centered_text_with_ink(
    scene: &mut Scene,
    renderer: &mut impl ButtonTextRenderer,
    text: &str,
    role: proposal::TextRole,
    rect: Rect,
    palette: Palette,
    align: ButtonTextAlign,
    ink_role: &str,
) {
    if text.is_empty() || rect.is_empty() {
        return;
    }
    let ink = palette.color(ink_role).expect("semantic text token");
    renderer.draw(scene, text, role, rect, align, ink);
}
fn button_text_align(policy: proposal::layout::AlignmentPolicy) -> ButtonTextAlign {
    match policy.text_alignment {
        proposal::layout::TextAlignment::Center => ButtonTextAlign::Center,
        // ButtonTextAlign currently has no trailing variant.
        proposal::layout::TextAlignment::Leading | proposal::layout::TextAlignment::Trailing => {
            ButtonTextAlign::Leading
        }
    }
}
#[allow(clippy::too_many_arguments)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewLayout {
    pub frame: Rect,
    pub content: Rect,
    pub visible: Option<Rect>,
}
impl ViewLayout {
    pub fn hit_test(self, p: Point) -> bool {
        self.visible.is_some_and(|r| r.contains(p))
    }
}

/// Semantic Surface/Panel presentation. The `panel` flag keeps the existing
/// Surface (no border) and Panel (hairline boundary) distinction.
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfaceView {
    layout: ViewLayout,
}
impl SurfaceView {
    pub fn render(
        &mut self,
        scene: &mut Scene,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        panel: bool,
    ) -> ViewLayout {
        self.render_at_scale(scene, frame, clip, palette, panel, 1.0)
    }

    /// As `render`, but resolves decorative borders using the renderer's
    /// backing scale. Component geometry and hit regions are identical.
    pub fn render_at_scale(
        &mut self,
        scene: &mut Scene,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        panel: bool,
        backing_scale: f32,
    ) -> ViewLayout {
        let visible = clipped(frame, clip);
        let inset = proposal::metric("spacing", "space-loose").unwrap_or(16.);
        self.layout = ViewLayout {
            frame,
            content: safe_inset(frame, inset, inset),
            visible,
        };
        paint_surface_background(
            scene,
            frame,
            clip,
            visible,
            palette,
            if panel {
                paint::SurfaceKind::Panel
            } else {
                paint::SurfaceKind::Canvas
            },
            backing_scale,
        );
        self.layout
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
}

/// Toolbar owns the equal-width action layout and each production Button.
#[derive(Clone, Debug, Default)]
pub struct ToolbarView {
    buttons: Vec<Button>,
    layout: Vec<crate::ButtonLayout>,
    focus: Option<usize>,
    disabled: bool,
    gap: f32,
}
impl ToolbarView {
    pub fn new(count: usize, gap: f32) -> Self {
        Self {
            buttons: vec![Button::default(); count],
            layout: Vec::new(),
            focus: None,
            disabled: false,
            gap: gap.max(0.),
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.focus = None;
            for button in &mut self.buttons {
                button.blur();
            }
        }
    }
    pub fn focus(&mut self, index: Option<usize>, visible: bool) {
        self.focus = index.filter(|i| *i < self.buttons.len());
        for (i, b) in self.buttons.iter_mut().enumerate() {
            b.state.focused = self.focus == Some(i);
            b.state.focus_visible = visible && self.focus == Some(i);
            if !b.state.focused {
                b.cancel();
            }
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
    ) -> Vec<crate::ButtonLayout> {
        self.buttons.resize_with(labels.len(), Button::default);
        self.focus = self.focus.filter(|i| *i < labels.len());
        let rects = Toolbar::layout(frame, labels.len(), self.gap);
        self.layout.clear();
        for (i, (b, r)) in self.buttons.iter_mut().zip(rects).enumerate() {
            b.state.disabled = self.disabled;
            b.state.focused = self.focus == Some(i);
            let l = b.render(
                scene,
                text,
                labels[i],
                r,
                clip,
                palette,
                if i == 0 {
                    paint::ButtonVariant::Primary
                } else {
                    paint::ButtonVariant::Quiet
                },
                ButtonSize::Body,
                role,
                ButtonTextAlign::Leading,
            );
            self.layout.push(l);
        }
        self.layout.clone()
    }
    pub fn pointer_move(&mut self, p: Point) {
        for b in &mut self.buttons {
            b.pointer_move_at(p);
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<usize> {
        if self.disabled {
            return None;
        }
        let i = self.layout.iter().position(|l| l.hit_test(p))?;
        self.buttons[i].pointer_down_at(p).then_some(i)
    }
    pub fn pointer_up(&mut self, p: Point) -> Option<usize> {
        let hit = self.layout.iter().position(|l| l.hit_test(p));
        self.buttons
            .iter_mut()
            .enumerate()
            .find_map(|(i, b)| b.pointer_up_at(p).then_some(i))
            .filter(|i| hit == Some(*i))
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> Option<usize> {
        if self.disabled {
            return None;
        }
        let i = self.focus?;
        self.buttons.get_mut(i)?.key(key, down).then_some(i)
    }
    pub fn cancel(&mut self) {
        for b in &mut self.buttons {
            b.cancel();
        }
    }
    pub fn layouts(&self) -> &[crate::ButtonLayout] {
        &self.layout
    }
}

/// Field metadata and input slot geometry; the actual input model remains the
/// shared TextInput/TextSurface host adapter.
#[derive(Clone, Debug, Default)]
pub struct FieldView {
    layout: FieldLayout,
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
    pub message_min_height: f32,
}
impl FieldPresentation {
    /// Emphasized label, caption helper and legacy 36px form slots.
    pub fn emphasized_form() -> Self {
        Self {
            label_role: proposal::TextRole::UiHeading,
            label_min_height: 36.,
            message_role: Some(proposal::TextRole::UiCaption),
            label_to_slot_gap: 4.,
            slot_to_message_gap: 6.,
            slot_min_height: 36.,
            message_min_height: 36.,
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
            message_min_height: 0.,
        }
    }
}
impl FieldView {
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
        let label_h = text
            .measure(
                &model.label,
                presentation.label_role,
                frame.size.width.max(0.),
            )
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
            .unwrap_or(36.)
            .max(measured_line + 2. * proposal::SCROLL_SINGLE_LINE_PADDING_Y)
            .max(presentation.slot_min_height.max(0.));
        let slot_y = frame.min_y() + label_h + presentation.label_to_slot_gap.max(0.);
        let slot = Rect::new(
            frame.min_x(),
            slot_y,
            frame.size.width.max(0.),
            input_h.max(0.),
        );
        let msg_y = slot.max_y() + presentation.slot_to_message_gap.max(0.);
        let message_rect = Rect::new(frame.min_x(), msg_y, frame.size.width.max(0.), message_h);
        let visible = clipped(frame, clip);
        let label_rect = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width.max(0.),
            label_h,
        );
        let required_mark = if model.required {
            let label_w = text
                .measure(
                    &model.label,
                    presentation.label_role,
                    frame.size.width.max(0.),
                )
                .width;
            let mark_w = text
                .measure("*", presentation.label_role, frame.size.width.max(0.))
                .width;
            Some(Rect::new(
                (label_rect.min_x() + label_w).min(label_rect.max_x()),
                label_rect.min_y(),
                mark_w.min((label_rect.max_x() - label_rect.min_x() - label_w).max(0.)),
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
        if let Some(v) = visible {
            scene.push_clip(v);
            draw_text(
                scene,
                text,
                &model.label,
                presentation.label_role,
                self.layout.label,
                palette,
                ButtonTextAlign::Leading,
            );
            if let Some(r) = required_mark {
                draw_text(scene, text, "*", role, r, palette, ButtonTextAlign::Leading);
            }
            if !message.is_empty() {
                draw_centered_text_with_ink(
                    scene,
                    text,
                    message,
                    message_role,
                    message_rect,
                    palette,
                    ButtonTextAlign::Leading,
                    if model.error.is_some() {
                        "status-failed"
                    } else {
                        "secondary-text"
                    },
                );
            }
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> FieldLayout {
        self.layout
    }
}

/// Existing bounded stepper presentation; it intentionally does not expose text
/// editing or locale parsing.
#[derive(Clone, Debug, Default)]
pub struct NumberFieldView {
    minus: Button,
    plus: Button,
    layout: NumberFieldLayout,
    disabled: bool,
    focused: Option<bool>,
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
    pub fn set_disabled(&mut self, d: bool) {
        self.disabled = d;
        if d {
            self.blur();
        }
    }
    /// Focus one stepper child (`increment = false` selects decrement).
    pub fn focus(&mut self, increment: bool, visible: bool) {
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
        // Preserve the established 48 / 12 / 80 / 10 / 48 stepper measure;
        // compress the same slots proportionally only in a narrower host.
        let nominal = 48. + 12. + 80. + 10. + 48.;
        let scale = (frame.size.width.max(0.) / nominal).min(1.);
        let button_w = 48. * scale;
        let first_gap = 12. * scale;
        let value_w = 80. * scale;
        let second_gap = 10. * scale;
        let minus = Rect::new(
            frame.min_x(),
            frame.min_y(),
            button_w,
            frame.size.height.max(0.),
        );
        let value = Rect::new(
            minus.max_x() + first_gap,
            frame.min_y(),
            value_w,
            frame.size.height.max(0.),
        );
        let plus = Rect::new(
            value.max_x() + second_gap,
            frame.min_y(),
            button_w,
            frame.size.height.max(0.),
        );
        self.minus.state.disabled = self.disabled || !model.can_increment(false);
        self.plus.state.disabled = self.disabled || !model.can_increment(true);
        self.minus.state.focused = self.focused == Some(false);
        self.plus.state.focused = self.focused == Some(true);
        let d = self.minus.render(
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
        let i = self.plus.render(
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
            scene.push_clip(v);
            draw_text(
                scene,
                &mut *text,
                &model.value().to_string(),
                proposal::TextRole::UiHeading,
                value,
                palette,
                ButtonTextAlign::Leading,
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
        if self.minus.pointer_up_at(p) && !self.disabled && model.can_increment(false) {
            return if model.increment(false) {
                NumberFieldEvent::Changed
            } else {
                NumberFieldEvent::None
            };
        }
        if self.plus.pointer_up_at(p) {
            return if !self.disabled && model.can_increment(true) && model.increment(true) {
                NumberFieldEvent::Changed
            } else {
                NumberFieldEvent::None
            };
        }
        NumberFieldEvent::None
    }
    pub fn key(&mut self, model: &mut NumberField, key: ControlKey) -> NumberFieldEvent {
        let up = matches!(key, ControlKey::Up | ControlKey::Right);
        let down = matches!(key, ControlKey::Down | ControlKey::Left);
        if !self.disabled && (up || down) && model.increment(up) {
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
        if button.key(key, down) {
            if model.can_increment(increment) && model.increment(increment) {
                NumberFieldEvent::Changed
            } else {
                NumberFieldEvent::None
            }
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
    pub fn pointer_move(&mut self, point: Point) {
        let inside = self.anchor_visible.is_some_and(|r| r.contains(point))
            || self.layout.visible.is_some_and(|r| r.contains(point));
        if self.hovered && !inside && !self.focused {
            self.dismissed_until_leave = false;
        }
        self.hovered = inside;
    }
    pub fn set_focus(&mut self, focused: bool, focus_visible: bool) {
        if self.focused && !focused && !self.hovered {
            self.dismissed_until_leave = false;
        }
        self.focused = focused;
        self.focus_visible = focused && focus_visible;
    }
    pub fn key(&mut self, key: ControlKey) -> TooltipEvent {
        if key == ControlKey::Escape && (self.hovered || self.focused || self.controlled_visible) {
            self.dismissed_until_leave = true;
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
        if !model.visible {
            self.controlled_suppressed = false;
        }
        self.controlled_visible = model.visible;
        let showing = !self.inactive
            && ((model.visible && !self.controlled_suppressed) || self.hovered || self.focused);
        let viewport = effective_viewport(viewport, clip);
        let r = place_overlay(anchor, size, viewport, 8.);
        let content = safe_inset(r, 12., 8.);
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
    layout: OverlayLayout,
    disabled: bool,
    dismiss: Button,
    dismiss_layout: Option<crate::ButtonLayout>,
    outside_armed: bool,
    active: bool,
}
impl Default for PopoverView {
    fn default() -> Self {
        Self {
            layout: OverlayLayout::default(),
            disabled: false,
            dismiss: Button::default(),
            dismiss_layout: None,
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
impl PopoverView {
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        content: Size,
        role: proposal::TextRole,
    ) -> Size {
        let dismiss = Button::measure(text, "Dismiss", f32::MAX, ButtonSize::Md, role);
        let spacing = popover_spacing();
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
    pub fn show(&mut self, model: &mut Popover, invoker: usize) {
        if !self.disabled && self.active {
            model.show(invoker)
        }
    }
    pub fn close(&mut self, model: &mut Popover) -> OverlayEvent {
        self.dismiss.blur();
        self.outside_armed = false;
        self.dismiss_layout = None;
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
            self.close(model)
        }
    }
    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled = focused && self.layout.visible.is_some() && self.active && !self.disabled;
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
        let viewport = effective_viewport(viewport, clip);
        let frame = place_overlay(anchor, size, viewport, 6.);
        let visible = model.open.then(|| clipped(frame, Some(viewport))).flatten();
        let spacing = popover_spacing();
        let dismiss_h = Button::measure(text, "Dismiss", f32::MAX, ButtonSize::Md, role)
            .height
            .min(frame.size.height);
        let dismiss_top = frame.max_y() - spacing.content_inset_y - dismiss_h;
        let content = Rect::new(
            frame.min_x() + spacing.content_inset_x,
            frame.min_y() + spacing.content_inset_y,
            (frame.size.width - spacing.content_inset_x * 2.).max(0.),
            (dismiss_top - spacing.content_to_action_gap - frame.min_y() - spacing.content_inset_y)
                .max(0.),
        );
        self.layout = OverlayLayout {
            frame,
            content,
            visible,
            anchor,
            requested_size: size,
        };
        self.dismiss_layout = None;
        if model.open && self.active && !self.disabled {
            if let Some(v) = visible {
                paint_surface_background(
                    scene,
                    frame,
                    Some(viewport),
                    visible,
                    palette,
                    paint::SurfaceKind::Floating,
                    text.backing_scale(),
                );
                scene.push_clip(v);
                draw_content(scene, content);
                let dismiss_w = spacing
                    .action_width
                    .min((frame.size.width - spacing.action_inset_x * 2.).max(0.));
                let dismiss_frame = Rect::new(
                    frame.min_x() + spacing.action_inset_x,
                    dismiss_top.max(frame.min_y()),
                    dismiss_w,
                    dismiss_h,
                );
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
                    ButtonTextAlign::Center,
                ));
                scene.pop_clip();
            } else {
                self.dismiss.blur();
                self.outside_armed = false;
            }
        } else {
            self.dismiss.blur();
            self.outside_armed = false;
        }
        self.layout
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
        if !self.active {
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
}
fn place_overlay(anchor: Rect, size: Size, viewport: Rect, gap: f32) -> Rect {
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
    Rect::new(x, y, w, h)
}

/// Toast rendering keeps duration progression deterministic in its model.
#[derive(Clone, Debug, Default)]
pub struct ToastView {
    layout: ViewLayout,
    dismiss: Button,
    dismiss_layout: Option<crate::ButtonLayout>,
    inactive: bool,
}
impl ToastView {
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
        let right_inset = 50.0;
        let leading_inset = 36.0;
        let gap = 4.0;
        let available_action_width = (width - leading_inset - gap - right_inset).max(0.0);
        let natural_action = Button::measure(
            text,
            "Dismiss",
            available_action_width,
            ButtonSize::Body,
            role,
        );
        let action_width = natural_action.width.max(90.0).min(available_action_width);
        let action_height = Button::measure(text, "Dismiss", action_width, ButtonSize::Body, role)
            .height
            .max(34.0);
        let content_width = (width - leading_inset - gap - action_width - right_inset).max(0.0);
        let content_height = text.measure(message, role, content_width).height.max(34.0);
        Size::new(
            width,
            62.0_f32
                .max(12.0 + content_height + 16.0)
                .max(10.0 + action_height + 10.0),
        )
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
        let visible = clipped(frame, clip);
        let right_inset = 50.0;
        let leading_inset = 36.0;
        let gap = 4.0;
        let available_action_width =
            (frame.size.width - leading_inset - gap - right_inset).max(0.0);
        let natural_action = Button::measure(
            text,
            "Dismiss",
            available_action_width,
            ButtonSize::Body,
            role,
        );
        let dismiss_w = natural_action.width.max(90.0).min(available_action_width);
        let dismiss_h = Button::measure(text, "Dismiss", dismiss_w, ButtonSize::Body, role)
            .height
            .max(34.0);
        let dismiss_x = frame.min_x() + (frame.size.width - right_inset - dismiss_w).max(0.0);
        let content_width = (dismiss_x - gap - (frame.min_x() + leading_inset)).max(0.0);
        let content_height = text
            .measure(&model.message, role, content_width)
            .height
            .max(34.0);
        self.layout = ViewLayout {
            frame,
            content: Rect::new(
                frame.min_x() + leading_inset,
                frame.min_y() + 12.,
                content_width,
                content_height,
            ),
            visible: model.visible.then_some(visible).flatten(),
        };
        self.dismiss_layout = None;
        if model.visible {
            if let Some(v) = visible {
                paint_surface_background(
                    scene,
                    frame,
                    clip,
                    visible,
                    palette,
                    paint::SurfaceKind::Panel,
                    text.backing_scale(),
                );
                scene.push_clip(v);
                let status = match model.tone {
                    NoticeTone::Info => StatusBadge::Info,
                    NoticeTone::Success => StatusBadge::Succeeded,
                    NoticeTone::Warning | NoticeTone::Error => StatusBadge::Failed,
                };
                crate::extended::status_badge(
                    scene,
                    Rect::new(frame.min_x() + 14., frame.min_y() + 18., 10., 10.),
                    palette,
                    status,
                );
                draw_text(
                    scene,
                    text,
                    &model.message,
                    role,
                    self.layout.content,
                    palette,
                    ButtonTextAlign::Leading,
                );
                self.dismiss_layout = Some(self.dismiss.render(
                    scene,
                    text,
                    "Dismiss",
                    Rect::new(
                        dismiss_x,
                        frame.min_y() + 10.,
                        dismiss_w,
                        dismiss_h.min(frame.size.height.max(0.)),
                    ),
                    Some(v),
                    palette,
                    paint::ButtonVariant::Quiet,
                    ButtonSize::Body,
                    role,
                    ButtonTextAlign::Leading,
                ));
                scene.pop_clip();
            } else {
                self.dismiss.blur();
            }
        } else {
            self.dismiss.blur();
        }
        self.layout
    }
    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled = focused && self.layout.visible.is_some() && !self.inactive;
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
}
/// Banner presentation owns its message and dismiss hit target.
#[derive(Clone, Debug, Default)]
pub struct BannerView {
    layout: ViewLayout,
    dismiss_button: Button,
    dismiss_layout: Option<crate::ButtonLayout>,
    inactive: bool,
}
impl BannerView {
    pub fn focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        let enabled = focused && self.layout.visible.is_some() && !self.inactive;
        self.dismiss_button.state.focused = enabled;
        self.dismiss_button.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.dismiss_button.cancel();
        }
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
        let visible = clipped(frame, clip);
        self.layout = ViewLayout {
            frame,
            content: safe_inset(frame, 12., 8.),
            visible: model.visible.then_some(visible).flatten(),
        };
        self.dismiss_layout = None;
        if model.visible {
            if let Some(v) = visible {
                paint_surface_background(
                    scene,
                    frame,
                    clip,
                    visible,
                    palette,
                    paint::SurfaceKind::Panel,
                    text.backing_scale(),
                );
                scene.push_clip(v);
                // Preserve the established Gallery banner specimen: bordered
                // panel, tone badge, two-line message slot and visible action.
                let status = match model.tone {
                    NoticeTone::Info => StatusBadge::Info,
                    NoticeTone::Success => StatusBadge::Succeeded,
                    NoticeTone::Warning => StatusBadge::Running,
                    NoticeTone::Error => StatusBadge::Failed,
                };
                crate::extended::status_badge(
                    scene,
                    Rect::new(frame.min_x() + 12., frame.min_y() + 12., 12., 12.),
                    palette,
                    status,
                );
                let dismiss_x = frame.min_x() + (frame.size.width - 170.).max(0.);
                let msg_width = if model.dismissible {
                    (dismiss_x - frame.min_x() - 50.).max(0.)
                } else {
                    (frame.size.width - 40.).max(0.)
                };
                let message_top = frame.min_y() + 6.;
                let message_height = 54f32.min((frame.size.height - 12.).max(0.));
                let msg = Rect::new(frame.min_x() + 34., message_top, msg_width, message_height);
                draw_text(
                    scene,
                    text,
                    &model.message,
                    role,
                    msg,
                    palette,
                    ButtonTextAlign::Leading,
                );
                if model.dismissible {
                    let dismiss = Rect::new(
                        dismiss_x,
                        frame.min_y() + 16.,
                        frame.size.width.min(92.),
                        34f32.min(frame.size.height.max(0.)),
                    );
                    self.dismiss_layout = Some(self.dismiss_button.render(
                        scene,
                        text,
                        "Dismiss",
                        dismiss,
                        Some(v),
                        palette,
                        paint::ButtonVariant::Primary,
                        ButtonSize::Body,
                        role,
                        ButtonTextAlign::Leading,
                    ));
                } else {
                    self.dismiss_button.blur();
                }
                scene.pop_clip();
            } else {
                self.dismiss_button.blur();
            }
        } else {
            self.dismiss_button.blur();
        }
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
}
#[derive(Clone, Copy, Debug, Default)]
pub struct StatusBadgeView {
    layout: ViewLayout,
}
impl StatusBadgeView {
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
            crate::extended::status_badge(scene, frame, palette, model);
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
}
impl ProgressView {
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
            crate::extended::progress(scene, frame, palette, model);
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
    layout: ViewLayout,
    action: Button,
    action_layout: Option<crate::ButtonLayout>,
    disabled: bool,
    inactive: bool,
}
impl EmptyStateView {
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
        let enabled = focused
            && !self.disabled
            && !self.inactive
            && self.layout.visible.is_some()
            && self.action_layout.is_some();
        self.action.state.focused = enabled;
        self.action.state.focus_visible = enabled && focus_visible;
        if !enabled {
            self.action.cancel();
        }
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
        let visible = clipped(frame, clip);
        // Keep the Gallery's established left-aligned empty-state composition.
        // `frame` is the padded content slot inside its parent surface.
        let x = frame.min_x() + 4.;
        let title_y = frame.min_y() + 6.;
        let content_width = (frame.size.width - 8.).max(0.);
        let title_width = content_width.min(420.);
        let title_h = text
            .measure(&model.title, proposal::TextRole::UiHeading, title_width)
            .height
            .max(34.);
        let detail_y = frame.min_y() + 44f32.max(title_y - frame.min_y() + title_h + 4.);
        let detail_h = text
            .measure(&model.detail, role, content_width.min(460.))
            .height
            .max(54.);
        let action_w = content_width.min(150.);
        let action_size = model
            .action
            .as_ref()
            .map(|label| Button::measure(text, label, action_w, ButtonSize::Body, role));
        let action_h = action_size.map_or(0., |size| size.height.max(34.));
        let action_y = detail_y + detail_h;
        let action_y = if action_size.is_some() {
            let desired_y = action_y.max(frame.min_y() + 100.);
            let last_fitting_y = frame.max_y() - action_h;
            if last_fitting_y >= detail_y + detail_h {
                desired_y.min(last_fitting_y)
            } else {
                // Preserve the text/action order in a constrained host. The
                // parent clip may trim the action, but it must not cover text.
                desired_y
            }
        } else {
            action_y
        };
        let total_h = action_y + action_h - title_y;
        let content = Rect::new(x, title_y, content_width, total_h.max(0.));
        self.layout = ViewLayout {
            frame,
            content,
            visible,
        };
        if let Some(v) = visible {
            scene.push_clip(v);
            draw_text(
                scene,
                text,
                &model.title,
                proposal::TextRole::UiHeading,
                Rect::new(x, title_y, title_width, title_h),
                palette,
                ButtonTextAlign::Leading,
            );
            draw_text(
                scene,
                text,
                &model.detail,
                role,
                Rect::new(x, detail_y, content_width.min(460.), detail_h),
                palette,
                ButtonTextAlign::Leading,
            );
            if let Some(label) = &model.action {
                let r = Rect::new(x, action_y, action_w, action_h);
                self.action.state.disabled = self.disabled || self.inactive;
                self.action_layout = Some(self.action.render(
                    scene,
                    text,
                    label,
                    r,
                    Some(v),
                    palette,
                    paint::ButtonVariant::Primary,
                    ButtonSize::Body,
                    role,
                    ButtonTextAlign::Leading,
                ));
            } else {
                self.action_layout = None;
                self.action.blur();
            }
            scene.pop_clip();
        } else {
            self.action_layout = None;
            self.action.blur();
        }
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
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SkeletonView {
    layout: ViewLayout,
}
impl SkeletonView {
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
        if let Some(v) = visible {
            scene.push_clip(v);
            paint::skeleton(scene, frame, palette);
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> ViewLayout {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::Primitive;

    struct FixedText;
    struct ScaledText(f32);

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
            _ink: tekes_ui_tokens::Rgba,
        ) {
        }
    }

    impl ButtonTextRenderer for FixedText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            let width = (text.chars().count() as f32 * 8.).min(max_width.max(0.));
            Size::new(width, role.style().line_height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    struct TallText;
    impl ButtonTextRenderer for TallText {
        fn measure(&mut self, text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
                48.,
            )
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    #[derive(Default)]
    struct RecordingText(Vec<(String, proposal::TextRole, Rect, ButtonTextAlign)>);
    impl ButtonTextRenderer for RecordingText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8.).min(max_width.max(0.)),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            text: &str,
            role: proposal::TextRole,
            rect: Rect,
            align: ButtonTextAlign,
            _ink: tekes_ui_tokens::Rgba,
        ) {
            self.0.push((text.to_owned(), role, rect, align));
        }
    }
    #[derive(Default)]
    struct WrappingText {
        measures: Vec<(String, proposal::TextRole, f32)>,
        draws: Vec<(String, proposal::TextRole, Rect, ButtonTextAlign)>,
    }
    impl ButtonTextRenderer for WrappingText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            self.measures.push((text.to_owned(), role, max_width));
            let line_height = role.style().line_height.max(1.);
            let width = (text.chars().count() as f32 * 8.).min(max_width.max(0.));
            let lines = if max_width > 0. {
                ((text.chars().count() as f32 * 8.) / max_width)
                    .ceil()
                    .max(1.)
            } else {
                1.
            };
            Size::new(width, lines * line_height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            text: &str,
            role: proposal::TextRole,
            rect: Rect,
            align: ButtonTextAlign,
            _ink: tekes_ui_tokens::Rgba,
        ) {
            self.draws.push((text.to_owned(), role, rect, align));
        }
    }
    fn palette() -> Palette {
        Palette::from(tekes_ui_tokens::Theme::LIGHT)
    }
    fn role() -> proposal::TextRole {
        proposal::TextRole::UiBody
    }

    fn assert_background_clip(
        scene: &Scene,
        frame: Rect,
        ancestor: Rect,
        visible: Rect,
        has_content: bool,
    ) {
        let primitives = scene.primitives();
        assert!(matches!(primitives.first(), Some(Primitive::PushClip(r)) if *r == ancestor));
        let background = primitives
            .iter()
            .position(|p| matches!(p, Primitive::Quad(q) if q.rect == frame))
            .expect("surface quad");
        assert!(
            primitives[1..background]
                .iter()
                .all(|p| matches!(p, Primitive::Shadow(_)))
        );
        assert!(matches!(
            primitives.get(background + 1),
            Some(Primitive::PopClip)
        ));
        if has_content {
            assert!(
                primitives
                    .iter()
                    .any(|p| matches!(p, Primitive::PushClip(r) if *r == visible))
            );
        }
    }

    #[test]
    fn surface_panel_clips_and_fully_clipped_frame_has_no_visible_geometry() {
        let mut view = SurfaceView::default();
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            Rect::new(0., 0., 100., 60.),
            Some(Rect::new(120., 0., 40., 40.)),
            palette(),
            true,
        );
        assert_eq!(layout.visible, None);
        assert!(scene.is_empty());
        assert!(!layout.hit_test(Point::new(1., 1.)));
    }

    #[test]
    fn surface_background_uses_parent_clip_but_keeps_visible_geometry() {
        let frame = Rect::new(40., 30., 120., 70.);
        for (ancestor, visible) in [
            (Rect::new(0., 0., 320., 240.), frame),
            (
                Rect::new(80., 40., 120., 90.),
                Rect::new(80., 40., 80., 60.),
            ),
        ] {
            let mut view = SurfaceView::default();
            let mut scene = Scene::new();
            let layout = view.render(&mut scene, frame, Some(ancestor), palette(), true);
            assert_eq!(layout.visible, Some(visible));
            assert_background_clip(&scene, frame, ancestor, visible, false);
        }
    }

    #[test]
    fn surface_render_scale_changes_only_decorative_ink_not_geometry() {
        let frame = Rect::new(10.0, 20.0, 180.0, 88.0);
        let clip = Rect::new(24.0, 10.0, 140.0, 70.0);
        for theme in tekes_ui_tokens::Theme::ALL {
            let palette = Palette::from(theme);
            let mut layouts = Vec::new();
            for scale in [1.0, 2.0] {
                let mut view = SurfaceView::default();
                let mut scene = Scene::new();
                let layout =
                    view.render_at_scale(&mut scene, frame, Some(clip), palette, true, scale);
                assert_eq!(layout.frame, frame);
                assert_eq!(layout.content, safe_inset(frame, 16.0, 16.0));
                assert_eq!(layout.visible, frame.intersection(&clip));
                let panel = scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| match primitive {
                        tekes_ui_scene::Primitive::Quad(q) if q.rect == frame => Some(q),
                        _ => None,
                    })
                    .expect("panel quad");
                assert_eq!(panel.rect, frame);
                assert_eq!(panel.fill, palette.color("surface-panel").unwrap());
                assert_eq!(
                    panel.border_color,
                    paint::decorative_border(palette, scale).1
                );
                layouts.push(layout);
            }
            assert_eq!(layouts[0], layouts[1]);
        }
    }
    #[test]
    fn field_provides_nonoverlapping_label_slot_and_error_message_geometry() {
        let mut view = FieldView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let field = Field {
            label: "Workspace".into(),
            help: "Use a short name".into(),
            error: Some("Already exists".into()),
            required: true,
        };
        let l = view.render(
            &mut scene,
            &mut text,
            &field,
            Rect::new(10., 20., 240., 120.),
            None,
            palette(),
            role(),
        );
        assert!(l.slot.min_y() >= l.label.max_y());
        assert!(l.message.min_y() >= l.slot.max_y());
        assert!(l.has_error);
        assert!(l.required_mark.is_some());
    }
    #[test]
    fn field_slot_grows_for_scaled_text_surface_line_metrics() {
        let mut view = FieldView::default();
        let mut text = TallText;
        let mut scene = Scene::new();
        let l = view.render(
            &mut scene,
            &mut text,
            &Field {
                label: "Name".into(),
                ..Field::default()
            },
            Rect::new(0., 0., 240., 120.),
            None,
            palette(),
            role(),
        );
        assert!(l.slot.size.height >= 48. + 2. * proposal::SCROLL_SINGLE_LINE_PADDING_Y);
    }
    #[test]
    fn field_emphasized_form_presentation_restores_label_and_spacing() {
        let mut view = FieldView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let frame = Rect::new(10., 20., 320., 140.);
        let layout = view.render_with_presentation(
            &mut scene,
            &mut text,
            &Field {
                label: "Workspace name".into(),
                help: "Use a clear name for your workspace.".into(),
                required: false,
                ..Field::default()
            },
            frame,
            None,
            palette(),
            role(),
            FieldPresentation::emphasized_form(),
        );
        assert_eq!(layout.label.min_y(), frame.min_y());
        assert_eq!(layout.label.size.height, 36.);
        assert_eq!(layout.slot.min_y(), layout.label.max_y() + 4.);
        assert_eq!(layout.message.min_y(), layout.slot.max_y() + 6.);
        assert!(layout.slot.size.height >= 36.);
        assert_eq!(layout.message.size.height, 36.);
        assert!(layout.required_mark.is_none());
    }
    #[test]
    fn toolbar_pointer_and_keyboard_activation_are_owned_by_child_buttons() {
        let mut view = ToolbarView::new(2, 4.);
        view.focus(Some(1), true);
        let mut scene = Scene::new();
        let mut text = FixedText;
        let r = Rect::new(0., 0., 180., 36.);
        let layouts = view.render(
            &mut scene,
            &mut text,
            &["Add", "Remove"],
            r,
            None,
            palette(),
            role(),
        );
        assert_eq!(layouts.len(), 2);
        assert_eq!(view.key(ControlKey::Enter, true), Some(1));
        let point = Point::new(layouts[0].frame.center().x, layouts[0].frame.center().y);
        assert_eq!(view.pointer_down(point), Some(0));
        assert_eq!(view.pointer_up(point), Some(0));
        view.set_disabled(true);
        assert_eq!(view.key(ControlKey::Enter, true), None);
    }
    #[test]
    fn toolbar_restores_primary_presentation_for_first_action() {
        let mut view = ToolbarView::new(3, 4.);
        let mut text = FixedText;
        let mut scene = Scene::new();
        let layouts = view.render(
            &mut scene,
            &mut text,
            &["Create", "Rename", "Remove"],
            Rect::new(0., 0., 528., 36.),
            None,
            palette(),
            role(),
        );
        let primary_fill = palette().color("action-primary-ground").unwrap();
        assert!(scene.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Quad(q) if q.rect == layouts[0].frame && q.fill == primary_fill)
        }));
    }
    #[test]
    fn number_field_restores_leading_emphasized_value_presentation() {
        let mut view = NumberFieldView::default();
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        let model = NumberField::new(8., 1., 32., 1.).unwrap();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(10., 20., 198., 36.),
            None,
            palette(),
            role(),
        );
        assert!(text.0.iter().any(|(drawn, text_role, rect, align)| {
            drawn == "8"
                && *text_role == proposal::TextRole::UiHeading
                && *rect
                    == Rect::new(
                        layout.value.min_x(),
                        layout.value.min_y(),
                        layout.value.size.width,
                        proposal::TextRole::UiHeading.style().line_height,
                    )
                && *align == ButtonTextAlign::Leading
        }));
        for stepper in ["−", "+"] {
            assert!(text.0.iter().any(|(drawn, _, _, align)| {
                drawn == stepper && *align == button_text_align(proposal::layout::BUTTON_CONTENT)
            }));
        }
    }
    #[test]
    fn number_field_respects_model_limits_and_cancels_press_when_disabled() {
        let mut model = NumberField::new(2., 1., 3., 1.).unwrap();
        let mut view = NumberFieldView::default();
        let mut scene = Scene::new();
        let mut text = FixedText;
        let l = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 140., 36.),
            None,
            palette(),
            role(),
        );
        assert!(
            view.pointer_down(Point::new(
                l.increment.frame.center().x,
                l.increment.frame.center().y
            ))
            .is_some()
        );
        view.set_disabled(true);
        assert_eq!(
            view.pointer_up(
                Point::new(l.increment.frame.center().x, l.increment.frame.center().y),
                &mut model
            ),
            NumberFieldEvent::None
        );
        assert_eq!(model.value(), 2.);
        view.set_disabled(false);
        let l = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 140., 36.),
            None,
            palette(),
            role(),
        );
        let p = Point::new(l.increment.frame.center().x, l.increment.frame.center().y);
        assert!(view.pointer_down(p).is_some());
        assert_eq!(view.pointer_up(p, &mut model), NumberFieldEvent::Changed);
        assert_eq!(model.value(), 3.);
        assert!(!model.can_increment(true));
    }

    #[test]
    fn number_field_focus_routes_enter_and_space_to_the_focused_stepper_button() {
        let mut model = NumberField::new(2., 0., 4., 1.).unwrap();
        let mut view = NumberFieldView::default();
        view.focus(false, true);
        assert_eq!(view.focused_index(), Some(false));
        assert_eq!(
            view.key_event(&mut model, ControlKey::Enter, true),
            NumberFieldEvent::Changed
        );
        assert_eq!(model.value(), 1.);
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, true),
            NumberFieldEvent::None
        );
        view.blur();
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, false),
            NumberFieldEvent::None
        );
        assert_eq!(view.focused_index(), None);
        assert_eq!(model.value(), 1.);
    }
    #[test]
    fn tooltip_and_popover_clamp_to_viewport_and_outside_escape_restore_focus() {
        let viewport = Rect::new(0., 0., 240., 160.);
        let anchor = Rect::new(210., 135., 20., 20.);
        let mut text = FixedText;
        let mut scene = Scene::new();
        let mut tooltip = TooltipView::default();
        let model = Tooltip {
            visible: true,
            text: "Helpful detail".into(),
        };
        let tip = tooltip.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(120., 42.),
            viewport,
            None,
            palette(),
            role(),
        );
        assert!(tip.frame.max_x() <= viewport.max_x());
        assert!(tip.frame.max_y() <= viewport.max_y());
        let mut popover = Popover::default();
        let mut view = PopoverView::default();
        view.show(&mut popover, 7);
        let p = view.render(
            &mut scene,
            &mut text,
            &popover,
            anchor,
            Size::new(150., 90.),
            viewport,
            None,
            palette(),
            role(),
        );
        assert!(p.frame.max_x() <= viewport.max_x());
        assert!(p.frame.max_y() <= viewport.max_y());
        assert_eq!(
            view.key(&mut popover, ControlKey::Escape),
            OverlayEvent::Dismissed {
                restore_focus: Some(7)
            }
        );
        view.show(&mut popover, 8);
        view.render(
            &mut scene,
            &mut text,
            &popover,
            anchor,
            Size::new(150., 90.),
            viewport,
            None,
            palette(),
            role(),
        );
        let outside = Point::new(3., 3.);
        assert!(view.pointer_down(&mut popover, outside));
        assert_eq!(
            view.pointer_up(&mut popover, outside),
            OverlayEvent::Dismissed {
                restore_focus: Some(8)
            }
        );
    }
    #[test]
    fn toast_fixture_time_banner_dismiss_and_progress_are_clipped() {
        let mut toast = Toast::new("Saved", NoticeTone::Success, 30);
        let mut view = ToastView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let r = Rect::new(10., 10., 180., 48.);
        view.render(
            &mut scene,
            &mut text,
            &toast,
            r,
            Some(Rect::new(0., 0., 100., 60.)),
            palette(),
            role(),
        );
        let toast_dismiss = view.dismiss_layout.unwrap().frame.center();
        assert_eq!(view.focus_target(), view.dismiss_layout.unwrap().visible);
        assert!(view.pointer_down(toast_dismiss));
        assert!(!view.pointer_up(&mut toast, Point::new(300., 300.)));
        assert!(!view.dismiss.state.pressed);
        assert!(toast.visible);
        assert!(view.pointer_down(toast_dismiss));
        assert!(view.pointer_up(&mut toast, toast_dismiss));
        assert!(!toast.visible);
        // Duration fixture tests use a fresh toast after the action test.
        let mut toast = Toast::new("Saved", NoticeTone::Success, 30);
        view.render(
            &mut scene,
            &mut text,
            &toast,
            r,
            Some(Rect::new(0., 0., 100., 60.)),
            palette(),
            role(),
        );
        assert!(!view.advance(&mut toast, 10));
        assert_eq!(toast.remaining_ms(), 20);
        assert!(view.advance(&mut toast, 20));
        let mut banner = Banner {
            message: "Offline".into(),
            tone: NoticeTone::Warning,
            dismissible: true,
            visible: true,
        };
        let mut b = BannerView::default();
        b.render(&mut scene, &mut text, &banner, r, None, palette(), role());
        let dismiss = b.dismiss_layout.unwrap().frame.center();
        assert_eq!(b.focus_target(), b.dismiss_layout.unwrap().visible);
        assert!(!b.pointer_up(&mut banner, dismiss));
        assert!(b.pointer_down(dismiss));
        assert!(!b.pointer_up(&mut banner, Point::new(300., 300.)));
        assert!(!b.dismiss_button.state.pressed);
        assert!(banner.visible);
        assert!(b.pointer_down(dismiss));
        assert!(b.pointer_up(&mut banner, dismiss));
        assert!(!banner.visible);
        let mut progress = ProgressView::default();
        progress.render(
            &mut scene,
            Progress::new(0.5),
            r,
            Some(Rect::new(500., 500., 10., 10.)),
            palette(),
        );
        assert!(progress.layout().visible.is_none());
        let mut badge = StatusBadgeView::default();
        badge.render(&mut scene, StatusBadge::Running, r, None, palette());
        let mut skeleton = SkeletonView::default();
        skeleton.render(&mut scene, r, None, palette());
        let mut empty = EmptyStateView::default();
        let state = EmptyState {
            title: "No items".into(),
            detail: "Try another query".into(),
            action: Some("Clear".into()),
        };
        empty.render(&mut scene, &mut text, &state, r, None, palette(), role());
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Quad(_)))
        );
    }
    #[test]
    fn banner_restores_legacy_message_slot_and_primary_dismiss_button() {
        let mut view = BannerView::default();
        let model = Banner {
            message: "Your workspace has unsaved changes.".into(),
            tone: NoticeTone::Info,
            dismissible: true,
            visible: true,
        };
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        let frame = Rect::new(10., 12., 640., 74.);
        view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        let dismiss = view.dismiss_layout.unwrap();
        assert_eq!(dismiss.frame, Rect::new(480., 28., 92., 34.));
        let message = text
            .0
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.message)
            .unwrap();
        assert_eq!(message.2.min_y(), frame.min_y() + 6.);
        assert_eq!(message.2.size.height, role().style().line_height);
        assert!(scene.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Quad(q) if q.rect == Rect::new(frame.min_x() + 12., frame.min_y() + 12., 12., 12.))
        }));
        let primary_fill = palette().color("action-primary-ground").unwrap();
        assert!(scene.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Quad(q) if q.rect == dismiss.frame && q.fill == primary_fill)
        }));
    }

    #[test]
    fn toast_uses_legacy_top_aligned_message_slot() {
        let mut view = ToastView::default();
        let model = Toast::new("Changes saved", NoticeTone::Success, 5000);
        let frame = Rect::new(40., 30., 520., 62.);
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        assert_eq!(layout.content, Rect::new(76., 42., 340., 34.));
        let message = text
            .0
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.message)
            .unwrap();
        assert_eq!(
            message.2,
            Rect::new(76., 42., 340., role().style().line_height)
        );
        assert_eq!(
            view.dismiss_layout.unwrap().frame,
            Rect::new(frame.min_x() + 380., frame.min_y() + 10., 90., 34.)
        );
        assert!(text.0.iter().any(|(drawn, _, _, align)| {
            drawn == "Dismiss" && *align == ButtonTextAlign::Leading
        }));
        assert!(scene.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Quad(q) if q.rect == Rect::new(frame.min_x() + 14., frame.min_y() + 18., 10., 10.))
        }));
    }

    #[test]
    fn toast_measure_matches_wrapped_message_and_dismiss_slot() {
        let mut view = ToastView::default();
        let model = Toast::new(
            "A long toast message that wraps in a narrow viewport.",
            NoticeTone::Info,
            5000,
        );
        let mut text = TallText;
        let measured = ToastView::measure(&mut text, &model.message, 196.0, role());
        assert!(measured.height > 62.0);
        let frame = Rect::new(20.0, 30.0, measured.width, measured.height);
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        let dismiss = view.dismiss_layout.unwrap();
        assert_eq!(layout.frame.size, measured);
        assert_eq!(layout.content.size.height, 48.0);
        assert!(dismiss.frame.max_y() <= frame.max_y());
        assert!(dismiss.frame.size.height >= 34.0);
        assert!(layout.content.max_y() <= frame.max_y());
        assert!(layout.content.max_x() + 4.0 <= dismiss.frame.min_x());
    }

    #[test]
    fn empty_state_text_uses_top_aligned_legacy_slots() {
        let mut view = EmptyStateView::default();
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: Some("Create document".into()),
        };
        let frame = Rect::new(36., 28., 488., 164.);
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        let title = text
            .0
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.title)
            .unwrap();
        let detail = text
            .0
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.detail)
            .unwrap();
        assert_eq!(title.2.min_y(), frame.min_y() + 6.);
        assert_eq!(
            title.2.size.height,
            proposal::TextRole::UiHeading.style().line_height
        );
        assert_eq!(detail.2.min_y(), frame.min_y() + 44.);
        assert_eq!(detail.2.size.height, role().style().line_height);
    }

    #[test]
    fn empty_state_action_stays_inside_tight_frame() {
        let mut view = EmptyStateView::default();
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: Some("Create document".into()),
        };
        let frame = Rect::new(36., 28., 488., 132.);
        let mut scene = Scene::new();
        let mut text = FixedText;
        view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        let action = view.action_layout.unwrap();
        assert_eq!(action.frame.max_y(), frame.max_y());
        assert_eq!(action.visible.unwrap().max_y(), frame.max_y());
    }

    #[test]
    fn empty_state_wrap_measure_matches_draw_and_action_stays_after_text() {
        let mut view = EmptyStateView::default();
        let model = EmptyState {
            title: "A".repeat(110),
            detail: "Details".into(),
            action: Some("Create document".into()),
        };
        let frame = Rect::new(20., 16., 500., 132.);
        let title_width = (frame.size.width - 8.).min(420.);
        let mut scene = Scene::new();
        let mut text = WrappingText::default();
        view.render(
            &mut scene,
            &mut text,
            &model,
            frame,
            None,
            palette(),
            role(),
        );
        let title_measurements: Vec<_> = text
            .measures
            .iter()
            .filter(|(measured, text_role, _)| {
                measured == &model.title && *text_role == proposal::TextRole::UiHeading
            })
            .collect();
        assert!(!title_measurements.is_empty());
        assert!(
            title_measurements
                .iter()
                .all(|(_, _, width)| *width == title_width)
        );
        let title_draw = text
            .draws
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.title)
            .unwrap();
        assert_eq!(title_draw.2.size.width, title_width);
        assert_eq!(
            title_draw.2.size.height,
            3. * proposal::TextRole::UiHeading.style().line_height
        );
        let detail_draw = text
            .draws
            .iter()
            .find(|(drawn, _, _, _)| drawn == &model.detail)
            .unwrap();
        let action = view.action_layout.unwrap();
        assert!(action.frame.min_y() >= detail_draw.2.min_y() + 54.);
        let detail_slot = Rect::new(detail_draw.2.min_x(), detail_draw.2.min_y(), 460., 54.);
        assert!(intersect(action.frame, detail_slot).is_none());
    }

    #[test]
    fn tooltip_owns_hover_focus_escape_and_leave_state() {
        let anchor = Rect::new(20., 20., 120., 36.);
        let viewport = Rect::new(0., 0., 240., 160.);
        let mut view = TooltipView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let model = Tooltip {
            visible: false,
            text: "Helpful detail".into(),
        };
        assert!(
            view.render(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(150., 48.),
                viewport,
                None,
                palette(),
                role(),
            )
            .visible
            .is_none()
        );
        view.pointer_move(anchor.center());
        assert!(
            view.render(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(150., 48.),
                viewport,
                None,
                palette(),
                role(),
            )
            .visible
            .is_some()
        );
        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
        assert!(
            view.render(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(150., 48.),
                viewport,
                None,
                palette(),
                role(),
            )
            .visible
            .is_none()
        );
        view.pointer_move(Point::new(230., 150.));
        view.render(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(150., 48.),
            viewport,
            None,
            palette(),
            role(),
        );
        view.pointer_move(anchor.center());
        assert!(
            view.render(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(150., 48.),
                viewport,
                None,
                palette(),
                role(),
            )
            .visible
            .is_some()
        );
        view.set_focus(true, true);
        assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
        view.pointer_move(Point::new(230., 150.));
        view.set_focus(false, false);
        assert!(
            view.render(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(150., 48.),
                viewport,
                None,
                palette(),
                role(),
            )
            .visible
            .is_none()
        );
    }

    #[test]
    fn popover_content_slot_uses_component_bounds_and_deactivation_restores_focus() {
        let viewport = Rect::new(0., 0., 240., 160.);
        let anchor = Rect::new(210., 135., 20., 20.);
        let mut model = Popover::default();
        model.show(12);
        let mut view = PopoverView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let mut slot = None;
        let layout = view.render_with_content(
            &mut scene,
            &mut text,
            &model,
            anchor,
            Size::new(180., 96.),
            viewport,
            Some(Rect::new(0., 0., 230., 150.)),
            palette(),
            role(),
            |scene, content| {
                slot = Some(content);
                scene.push_quad(Quad::new(content, palette().color("surface-2").unwrap()));
            },
        );
        let content = slot.expect("open popover calls its content slot");
        assert_eq!(content, layout.content);
        assert!(
            layout
                .visible
                .is_some_and(|visible| visible.contains(content.center()))
        );
        assert!(layout.frame.max_x() <= 230.);
        assert!(layout.frame.max_y() <= 150.);
        let dismiss = view.dismiss_layout.unwrap().frame.center();
        assert!(view.pointer_down(&mut model, dismiss));
        assert_eq!(
            view.pointer_up(&mut model, dismiss),
            OverlayEvent::Dismissed {
                restore_focus: Some(12)
            }
        );
        view.show(&mut model, 13);
        assert!(model.open);
        assert_eq!(
            view.set_window_active(false, &mut model),
            OverlayEvent::Dismissed {
                restore_focus: Some(13)
            }
        );
        assert!(!model.open);
    }

    #[test]
    fn popover_surface_uses_floating_role_and_text_backend_scale() {
        for theme in tekes_ui_tokens::Theme::ALL {
            for scale in [1.0, 2.0] {
                let palette = Palette::from(theme);
                let mut model = Popover::default();
                let mut view = PopoverView::default();
                view.show(&mut model, 7);
                let mut text = ScaledText(scale);
                let mut scene = Scene::new();
                let layout = view.render_with_content(
                    &mut scene,
                    &mut text,
                    &model,
                    Rect::new(30.0, 24.0, 80.0, 30.0),
                    Size::new(180.0, 96.0),
                    Rect::new(0.0, 0.0, 320.0, 240.0),
                    None,
                    palette,
                    role(),
                    |_, _| {},
                );
                let popover_quad = scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| match primitive {
                        Primitive::Quad(q) if q.rect == layout.frame => Some(q),
                        _ => None,
                    })
                    .expect("popover surface quad");
                assert_eq!(popover_quad.fill, palette.color("surface-popover").unwrap());
                assert_eq!(
                    popover_quad.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(
                        proposal::metric("radius", "radius-floating").unwrap(),
                    )
                );
                assert_eq!(
                    popover_quad.border_width,
                    paint::decorative_border(palette, scale).0
                );
            }
        }
    }

    #[test]
    fn popover_background_uses_effective_parent_viewport_before_content_clip() {
        let anchor = Rect::new(90., 70., 30., 24.);
        let viewport = Rect::new(0., 0., 320., 240.);
        for (clip, parent) in [
            (Rect::new(-20., -20., 360., 280.), viewport),
            (
                Rect::new(20., 10., 240., 200.),
                Rect::new(20., 10., 240., 200.),
            ),
        ] {
            let mut model = Popover::default();
            model.show(1);
            let mut view = PopoverView::default();
            let mut text = FixedText;
            let mut scene = Scene::new();
            let layout = view.render_with_content(
                &mut scene,
                &mut text,
                &model,
                anchor,
                Size::new(160., 100.),
                viewport,
                Some(clip),
                palette(),
                role(),
                |_, _| {},
            );
            let visible = layout.visible.expect("popover visible inside parent");
            assert_background_clip(&scene, layout.frame, parent, visible, true);
        }
    }

    #[test]
    fn toast_and_banner_backgrounds_use_parent_clip_before_content_clip() {
        let frame = Rect::new(40., 30., 220., 74.);
        for ancestor in [
            Rect::new(0., 0., 400., 300.),
            Rect::new(80., 40., 140., 60.),
        ] {
            let expected_visible = frame.intersection(&ancestor).unwrap();
            let toast = Toast::new("Saved", NoticeTone::Success, 1_000);
            let mut toast_view = ToastView::default();
            let mut toast_text = FixedText;
            let mut toast_scene = Scene::new();
            let toast_layout = toast_view.render(
                &mut toast_scene,
                &mut toast_text,
                &toast,
                frame,
                Some(ancestor),
                palette(),
                role(),
            );
            assert_eq!(toast_layout.visible, Some(expected_visible));
            assert_background_clip(&toast_scene, frame, ancestor, expected_visible, true);

            let banner = Banner {
                message: "Offline".into(),
                tone: NoticeTone::Warning,
                dismissible: true,
                visible: true,
            };
            let mut banner_view = BannerView::default();
            let mut banner_text = FixedText;
            let mut banner_scene = Scene::new();
            let banner_layout = banner_view.render(
                &mut banner_scene,
                &mut banner_text,
                &banner,
                frame,
                Some(ancestor),
                palette(),
                role(),
            );
            assert_eq!(banner_layout.visible, Some(expected_visible));
            assert_background_clip(&banner_scene, frame, ancestor, expected_visible, true);
        }
    }

    #[test]
    fn popover_measure_matches_rendered_content_slot() {
        let mut model = Popover::default();
        model.show(4);
        let mut view = PopoverView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let requested = Size::new(160., 100.);
        let size = PopoverView::measure(&mut text, requested, role());
        let layout = view.render_with_content(
            &mut scene,
            &mut text,
            &model,
            Rect::new(100., 100., 20., 20.),
            size,
            Rect::new(0., 0., 600., 600.),
            None,
            palette(),
            role(),
            |_, _| {},
        );
        assert_eq!(layout.content.size, requested);
    }

    #[test]
    fn popover_fully_clipped_geometry_never_receives_input() {
        let mut model = Popover::default();
        model.show(3);
        let mut view = PopoverView::default();
        let mut text = FixedText;
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(10., 10., 20., 20.),
            Size::new(120., 80.),
            Rect::new(0., 0., 240., 160.),
            Some(Rect::new(300., 300., 20., 20.)),
            palette(),
            role(),
        );
        assert_eq!(layout.visible, None);
        assert!(!layout.hit_test(Point::new(10., 10.)));
        assert!(!view.pointer_down(&mut model, Point::new(10., 10.)));
        assert_eq!(
            view.pointer_up(&mut model, Point::new(10., 10.)),
            OverlayEvent::None
        );
        assert!(model.open);
    }

    #[test]
    fn empty_state_action_requires_a_real_visible_pointer_press() {
        let mut view = EmptyStateView::default();
        let model = EmptyState {
            title: "No documents".into(),
            detail: "Create a document to get started.".into(),
            action: Some("Create document".into()),
        };
        let mut scene = Scene::new();
        let mut text = FixedText;
        let outer = Rect::new(10., 20., 520., 164.);
        let mut surface = SurfaceView::default();
        let content = surface
            .render(&mut scene, outer, None, palette(), true)
            .content;
        view.render(
            &mut scene,
            &mut text,
            &model,
            content,
            None,
            palette(),
            role(),
        );
        let title = view.layout.content;
        assert_eq!(title.min_x(), outer.min_x() + 20.);
        assert_eq!(title.min_y(), outer.min_y() + 22.);
        let point = view.action_layout.unwrap().frame.center();
        assert_eq!(
            view.action_layout.unwrap().frame.min_x(),
            outer.min_x() + 20.
        );
        assert_eq!(
            view.action_layout.unwrap().frame.min_y(),
            outer.min_y() + 114.
        );
        let primary_fill = palette().color("action-primary-ground").unwrap();
        assert!(scene.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Quad(q) if q.rect == view.action_layout.unwrap().frame && q.fill == primary_fill)
        }));
        view.pointer_move(point);
        assert!(view.action.state.hovered);
        assert_eq!(view.focus_target(), view.action_layout.unwrap().visible);
        assert!(!view.pointer_up(point));
        assert!(view.pointer_down(point));
        assert!(view.action.state.pressed);
        let outside = Point::new(350., 220.);
        view.pointer_move(outside);
        assert!(!view.action.state.pressed);
        assert!(!view.pointer_up(outside));
        assert!(!view.action.state.pressed);
        view.pointer_move(point);
        assert!(view.action.state.hovered);
        assert!(view.pointer_down(point));
        assert!(view.pointer_up(point));

        view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 260., 180.),
            Some(Rect::new(400., 400., 20., 20.)),
            palette(),
            role(),
        );
        assert!(view.action_layout.is_none());
        assert!(!view.pointer_down(point));
        assert!(!view.pointer_up(point));

        view.render(
            &mut scene,
            &mut text,
            &model,
            Rect::new(0., 0., 260., 180.),
            None,
            palette(),
            role(),
        );
        let point = view.action_layout.unwrap().frame.center();
        view.pointer_move(point);
        assert!(view.action.state.hovered);
        view.set_disabled(true);
        view.pointer_move(point);
        assert!(!view.action.state.hovered);
        assert!(!view.pointer_down(point));
        assert!(!view.pointer_up(point));
    }
}
