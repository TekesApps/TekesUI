use super::{ComponentEvent, IconLayout, IconRenderer, IconView, has_area, icon_foreground, valid};
use crate::control_motion::{ProgressTransition, motion_for};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::{Axis, ControlFrame, Disclosure, Link},
    paint::ButtonVariant,
};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonComponentLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub content: Rect,
}
#[derive(Clone, Debug)]
pub(super) struct ButtonRuntime {
    pub(super) button: ButtonModel,
    pub(super) layout: ButtonComponentLayout,
    pub(super) focused: bool,
    pub(super) window_active: bool,
    pub(super) press_target: bool,
}
impl Default for ButtonRuntime {
    fn default() -> Self {
        Self {
            button: ButtonModel::default(),
            layout: ButtonComponentLayout::default(),
            focused: false,
            window_active: true,
            press_target: false,
        }
    }
}
impl ButtonRuntime {
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        variant: ButtonVariant,
        radius: Option<f32>,
    ) -> ButtonComponentLayout {
        if !valid(frame, clip) {
            self.blur();
            self.button.clear_layout();
            self.layout = ButtonComponentLayout::default();
            return self.layout;
        }
        self.button.state.focused = self.focused && self.window_active;
        let l = self.button.render_with_radius(
            scene,
            text,
            label,
            frame,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
            radius,
        );
        self.layout = ButtonComponentLayout {
            frame: l.frame,
            visible: l.visible,
            content: l.content,
        };
        if !l.visible.is_some_and(has_area) || self.button.state.disabled {
            self.blur();
        }
        self.layout
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_icon_label(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        variant: ButtonVariant,
        icon_slot_width: f32,
        radius: Option<f32>,
    ) -> (ButtonComponentLayout, Rect) {
        if !valid(frame, clip) {
            self.blur();
            self.button.clear_layout();
            self.layout = ButtonComponentLayout::default();
            return (self.layout, Rect::default());
        }
        let inset = proposal::metric("spacing", "space-relaxed").unwrap_or(12.);
        let available = (frame.size.width - inset * 2.).max(0.);
        let slot_width = icon_slot_width.max(0.).min(available);
        let text_width = (available - slot_width).max(0.);
        let label_size = if label.is_empty() {
            Size::default()
        } else {
            text.measure(label, role, text_width)
        };
        let vertical_inset = ((proposal::metric("size", "size-action-button").unwrap_or(40.)
            - role.style().line_height)
            * 0.5)
            .max(0.);
        let button_frame = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            frame
                .size
                .height
                .max(label_size.height + vertical_inset * 2.),
        );
        self.button.state.focused = self.focused && self.window_active;
        let button_layout = self.button.render_with_radius(
            scene,
            text,
            "",
            button_frame,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
            radius,
        );
        let icon_side = proposal::metric("size", "size-icon-preview")
            .expect("Disclosure icon artboard token")
            .min(slot_width);
        let row_bounds = Rect::new(
            button_layout.content.min_x(),
            button_layout.frame.min_y(),
            button_layout.content.size.width,
            button_layout.frame.size.height,
        );
        let row = layout_measured_children(
            row_bounds,
            Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(icon_side, icon_side)),
                MeasuredChild::new(Size::new(text_width, label_size.height)),
            ],
            (slot_width - icon_side).max(0.),
            proposal::layout::alignment(proposal::layout::AlignmentPreset::Row),
            AlignmentOverride::default(),
        );
        let icon_frame = row[0].frame;
        let content = row[1].frame;
        self.layout = ButtonComponentLayout {
            frame: button_layout.frame,
            visible: button_layout.visible,
            content,
        };
        if let Some(visible) = button_layout.visible.filter(|v| has_area(*v)) {
            if !label.is_empty() && has_area(content) {
                scene.push_clip(visible);
                let ink = if let Some(surface) = self.button.surface() {
                    self.button.surface_colors(palette, variant, surface).text
                } else {
                    icon_foreground(palette, self.button.state, variant)
                };
                text.draw(scene, label, role, content, ButtonTextAlign::Leading, ink);
                scene.pop_clip();
            }
        } else {
            self.cancel();
            self.layout = ButtonComponentLayout::default();
            return (self.layout, Rect::default());
        }
        (self.layout, icon_frame)
    }
    pub(super) fn pointer_move(&mut self, p: Point) {
        if self.window_active {
            self.button.pointer_move_at(p);
        }
    }
    pub(super) fn pointer_down(&mut self, p: Point) -> bool {
        if !self.window_active || self.button.state.disabled {
            return false;
        }
        let armed = self.button.pointer_down_at(p);
        self.press_target = armed;
        if armed {
            self.focused = true;
            self.button.state.focused = true;
        }
        armed
    }
    pub(super) fn pointer_up(&mut self, p: Point) -> bool {
        if !self.window_active {
            self.blur();
            return false;
        }
        let was = self.press_target;
        self.press_target = false;
        self.button.pointer_up_at(p) && was
    }
    pub(super) fn key(&mut self, key: ControlKey, down: bool) -> bool {
        self.window_active
            && self.focused
            && !self.button.state.disabled
            && self.button.key(key, down)
    }
    pub(super) fn focus(&mut self, visible: bool) {
        self.focused = true;
        self.button.state.focused = true;
        self.button.state.focus_visible = visible;
    }
    pub(super) fn blur(&mut self) {
        self.focused = false;
        self.press_target = false;
        self.button.blur();
    }
    pub(super) fn cancel(&mut self) -> bool {
        let had = self.press_target || self.button.state.pressed;
        self.press_target = false;
        self.button.cancel();
        had
    }
    pub(super) fn active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
        }
    }
}

/// Geometry for a Disclosure's animated body. `body_rect` is the stable full
/// content frame; `visible_clip` is the portion currently revealed and safe to
/// draw. Hosts must use that clip for body painting and hit testing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DisclosureBodyLayout {
    pub body_rect: Rect,
    pub visible_clip: Option<Rect>,
    pub occupied_height: f32,
    pub progress: f32,
    /// False as soon as the Disclosure starts closing, including during its
    /// visual collapse, so hidden content cannot receive input.
    pub interactive: bool,
}

#[derive(Clone, Debug)]
pub struct DisclosureView {
    runtime: ButtonRuntime,
    icon: IconView,
    icon_frame: Rect,
    reveal: ProgressTransition,
    reveal_initialized: bool,
    reveal_suspended: bool,
    motion_now: Duration,
}
impl Default for DisclosureView {
    fn default() -> Self {
        let mut runtime = ButtonRuntime::default();
        runtime.button.set_surface("panel");
        Self {
            runtime,
            icon: IconView::default(),
            icon_frame: Rect::default(),
            reveal: ProgressTransition::default(),
            reveal_initialized: false,
            reveal_suspended: false,
            motion_now: Duration::default(),
        }
    }
}
impl DisclosureView {
    /// Request-only delivery uses the original ButtonRuntime, without changing
    /// the confirmed model. `changed` reports an activation intent, not a toggle.
    pub fn request_pointer_down(&mut self, p: Point, model: &Disclosure) -> bool {
        self.sync_model(model);
        self.runtime.pointer_down(p)
    }
    pub fn request_pointer_up(&mut self, p: Point, model: &Disclosure) -> bool {
        self.sync_model(model);
        self.runtime.pointer_up(p)
    }
    pub fn request_key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &Disclosure,
    ) -> ComponentEvent {
        self.sync_model(model);
        let consumed = matches!(k, ControlKey::Enter | ControlKey::Space)
            && self.runtime.window_active
            && self.runtime.focused
            && !model.disabled;
        let changed = self.runtime.key(k, down);
        ComponentEvent { consumed, changed }
    }
    /// Retire a controlled gesture using the original runtime and suspend the
    /// original reveal until its next layout; no second motion state is created.
    pub fn request_cancel(&mut self) -> bool {
        self.reveal_suspended = true;
        self.runtime.cancel()
    }
    pub fn request_activate(&mut self, model: &Disclosure) -> bool {
        self.sync_model(model);
        self.runtime.window_active && !model.disabled
    }
    pub fn set_surface(
        &mut self,
        surface: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        let surface = tekes_ui_tokens::surface_palette::canonical_surface(surface)?;
        self.runtime.button.set_surface(surface);
        Ok(())
    }
    pub(crate) fn focus_decoration_above_siblings(&self, model: &Disclosure) -> bool {
        // Disclosure's original row always emits an outsetting ring, including
        // its default Minimum geometry. Do not apply compact Button Exact-only.
        self.runtime.window_active
            && self.runtime.button.state.focused
            && self.runtime.button.state.focus_visible
            && !model.disabled
            && !self.runtime.button.state.loading
    }
    pub(crate) fn content_inks(
        &self,
        palette: Palette,
        surface: &str,
    ) -> (tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba) {
        let colors = self
            .runtime
            .button
            .surface_colors(palette, ButtonVariant::Quiet, surface);
        (colors.text, colors.icon)
    }
    pub(crate) fn content_insets(text: &mut impl ButtonTextRenderer) -> (f32, f32, f32) {
        let role = proposal::TextRole::UiBody;
        let scale = text.resolved_font_size(role) / role.style().size;
        let scale = if scale.is_finite() && scale > 0. {
            scale
        } else {
            1.
        };
        (
            proposal::metric("spacing", "space-relaxed").expect("Disclosure inset") * scale,
            proposal::metric("size", "size-icon-standard").expect("Disclosure icon") * scale,
            proposal::metric("size", "size-action-button").expect("Disclosure row") * scale,
        )
    }
    /// Shared arranged/painted geometry for the original passive title slot.
    pub(crate) fn content_frames(
        text: &mut impl ButtonTextRenderer,
        size: Size,
        frame: Rect,
        rtl: bool,
        section_header: bool,
    ) -> (Rect, Rect) {
        let (inset, side, _) = Self::content_insets(text);
        let inset = if section_header { 16. } else { inset };
        let width = (frame.size.width - inset * 2.).max(0.);
        let side = if section_header {
            0.
        } else {
            side.min(width).min(frame.size.height)
        };
        let slot = if section_header {
            0.
        } else {
            (side + inset).min(width)
        };
        let icon_x = if rtl {
            frame.max_x() - inset - side
        } else {
            frame.min_x() + inset
        };
        let title_x = frame.min_x() + inset + if rtl { 0. } else { slot };
        (
            Rect::new(
                title_x,
                frame.center().y - size.height.min(frame.size.height) * 0.5,
                (width - slot).max(0.),
                size.height.min(frame.size.height),
            ),
            Rect::new(icon_x, frame.center().y - side * 0.5, side, side),
        )
    }
    /// Original row chrome/runtime with a passive Content slot. Direction is
    /// local to this component; it is not a global text/layout RTL policy.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_content(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Disclosure,
        size: Size,
        frame: Rect,
        clip: Rect,
        palette: Palette,
        rtl: bool,
        section_header: bool,
    ) -> ButtonComponentLayout {
        self.sync_model(model);
        self.runtime.button.state.focused = self.runtime.focused && self.runtime.window_active;
        let l = self.runtime.button.render_with_radius(
            scene,
            text,
            "",
            frame,
            Some(clip),
            palette,
            ButtonVariant::Quiet,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
            Some(proposal::metric("radius", "radius-row").expect("Disclosure radius token")),
        );
        let (content, icon) = Self::content_frames(text, size, frame, rtl, section_header);
        self.icon_frame = icon;
        self.runtime.layout = ButtonComponentLayout {
            frame: l.frame,
            visible: l.visible,
            content,
        };
        if self.runtime.layout.visible.is_none() {
            self.runtime.cancel();
        }
        self.runtime.layout
    }
    pub(crate) fn render_content_indicator(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        model: &Disclosure,
        rtl: bool,
        clip: Rect,
        colors: (Palette, tekes_ui_tokens::Rgba),
    ) {
        self.icon.render_tinted(
            scene,
            renderer,
            if model.open {
                "tekes-expand"
            } else if rtl {
                "tekes-back"
            } else {
                "tekes-forward"
            },
            self.icon_frame,
            self.icon_frame.size.width,
            Some(clip),
            colors.0,
            colors.1,
        );
    }

    fn sync_model(&mut self, model: &Disclosure) {
        self.runtime.button.state.disabled = model.disabled;
        if model.disabled {
            self.runtime.cancel();
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Disclosure,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonComponentLayout {
        self.sync_model(model);
        let (layout, icon_frame) = self.runtime.render_icon_label(
            scene,
            text,
            label,
            frame,
            clip,
            palette,
            role,
            ButtonVariant::Quiet,
            24.,
            Some(proposal::metric("radius", "radius-row").expect("Disclosure radius token")),
        );
        self.icon_frame = icon_frame;
        layout
    }
    /// Paints the state icon in the fixed leading slot established by `render`.
    /// Hosts can call this after releasing their text-engine borrow.
    pub fn render_icon(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        model: &Disclosure,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) -> IconLayout {
        let tint = self
            .runtime
            .button
            .surface_colors(
                palette,
                ButtonVariant::Quiet,
                self.runtime.button.surface().unwrap_or("panel"),
            )
            .icon;
        self.icon.render_tinted(
            scene,
            renderer,
            if model.open {
                "tekes-expand"
            } else {
                "tekes-forward"
            },
            self.icon_frame,
            icon_size,
            clip,
            palette,
            tint,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_icon(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        renderer: &mut impl IconRenderer,
        model: &Disclosure,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        icon_size: f32,
    ) -> ButtonComponentLayout {
        let layout = self.render(scene, text, model, label, frame, clip, palette, role);
        self.render_icon(scene, renderer, model, clip, palette, icon_size);
        layout
    }
    pub fn icon_frame(&self) -> Rect {
        self.icon_frame
    }
    /// Resolves the body reveal from the host clock and Design System motion
    /// token. The body frame remains stable while `visible_clip` grows or
    /// shrinks, allowing the host to lay out content once and clip it here.
    pub fn layout_body(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Disclosure,
        body_rect: Rect,
        ancestor_clip: Option<Rect>,
    ) -> DisclosureBodyLayout {
        let context = text.motion_context();
        if let Some(context) = context
            && context.now >= self.motion_now
        {
            self.motion_now = context.now;
        }
        let now = self.motion_now;
        let target = if model.open { 1.0 } else { 0.0 };
        let geometry_valid = valid(body_rect, ancestor_clip);
        let ancestor_visible = geometry_valid
            .then(|| ControlFrame::new(body_rect, ancestor_clip).visible_frame())
            .flatten();
        let can_animate = self.runtime.window_active
            && !self.reveal_suspended
            && !model.disabled
            && has_area(body_rect)
            && ancestor_visible.is_some();
        let value = if !self.reveal_initialized || !can_animate {
            self.reveal.snap(target, now).unwrap_or(target)
        } else if let Some(context) = context {
            self.reveal
                .retarget(
                    target,
                    motion_for("disclosure_reveal"),
                    now,
                    context.reduced_motion,
                )
                .unwrap_or(target)
        } else {
            self.reveal.snap(target, now).unwrap_or(target)
        };
        self.reveal_initialized = true;
        self.reveal_suspended = false;
        let progress = value.clamp(0.0, 1.0) as f32;
        let occupied_height = if geometry_valid {
            (body_rect.size.height * progress).clamp(0.0, body_rect.size.height)
        } else {
            0.0
        };
        let revealed = Rect::new(
            body_rect.origin.x,
            body_rect.origin.y,
            body_rect.size.width,
            occupied_height,
        );
        let visible_clip = if geometry_valid && occupied_height > 0.0 {
            ControlFrame::new(revealed, ancestor_clip).visible_frame()
        } else {
            None
        };
        if can_animate && self.reveal.needs_animation() {
            text.request_animation_frame();
        }
        DisclosureBodyLayout {
            body_rect,
            visible_clip,
            occupied_height,
            progress,
            interactive: model.open && visible_clip.is_some(),
        }
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.runtime.pointer_move(p)
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        self.runtime.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut Disclosure) -> bool {
        self.sync_model(model);
        if self.runtime.pointer_up(p) {
            model.toggle()
        } else {
            false
        }
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut Disclosure,
    ) -> ComponentEvent {
        self.sync_model(model);
        if !matches!(k, ControlKey::Enter | ControlKey::Space) {
            return ComponentEvent::default();
        }
        let consumed = self.runtime.window_active
            && self.runtime.focused
            && !self.runtime.button.state.disabled;
        let activated = self.runtime.key(k, down);
        let changed = activated && model.toggle();
        ComponentEvent { consumed, changed }
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut Disclosure) -> bool {
        self.key_event(k, down, model).changed
    }
    pub fn focus(&mut self, v: bool) {
        self.runtime.focus(v)
    }
    pub fn blur(&mut self) {
        self.runtime.blur()
    }
    pub fn cancel(&mut self) -> bool {
        self.runtime.cancel()
    }
    pub fn layout(&self) -> ButtonComponentLayout {
        self.runtime.layout
    }
    pub fn activate(&mut self, model: &mut Disclosure) -> bool {
        self.sync_model(model);
        model.toggle()
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.runtime.active(active);
        if !active {
            self.reveal_suspended = true;
        }
    }
}

#[derive(Clone, Debug)]
pub struct LinkView {
    pub(super) button: ButtonModel,
    pub(super) layout: ButtonComponentLayout,
    pub(super) focused: bool,
    pub(super) focus_visible: bool,
    pub(super) hovered: bool,
    pub(super) pressed: bool,
    pub(super) window_active: bool,
    pub(super) visible: Option<Rect>,
    pub(super) surface: &'static str,
}
impl LinkView {
    /// Validate the parent identity without changing the original arm/focus/model.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        Ok(())
    }
    fn sync_model(&mut self, model: &Link) {
        self.button.state.disabled = model.button.state.disabled;
        self.button.state.loading = model.button.state.loading;
        if model.button.state.disabled || model.button.state.loading {
            self.cancel();
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut Link,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonComponentLayout {
        self.sync_model(model);
        let visible = ControlFrame::new(frame, clip).visible_frame();
        if !visible.is_some_and(has_area)
            || model.button.state.disabled
            || model.button.state.loading
        {
            self.blur();
        }
        let inset = proposal::metric("spacing", "space-regular")
            .expect("Link content inset token")
            .max(0.);
        let content = Rect::new(
            frame.min_x() + inset,
            frame.min_y(),
            (frame.size.width - inset * 2.).max(0.),
            frame.size.height.max(0.),
        );
        self.layout = ButtonComponentLayout {
            frame,
            visible,
            content,
        };
        self.visible = visible;
        self.button.state.hovered = self.hovered && visible.is_some() && self.window_active;
        self.button.state.pressed = self.pressed && self.hovered && self.window_active;
        self.button.state.focused = self.focused && self.window_active;
        self.button.state.focus_visible = self.focused && self.focus_visible && self.window_active;
        if let Some(v) = visible {
            // Decoration/focus may extend beyond the Link frame, while text
            // and hit testing remain constrained to its visible content.
            if let Some(ancestor) = clip {
                scene.push_clip(ancestor);
            }
            let measured = text.measure(label, role, content.size.width);
            let text_bounds = Rect::new(
                content.min_x(),
                content.center().y - measured.height * 0.5,
                measured.width.max(0.),
                measured.height.max(0.),
            );
            let ink = crate::extended::link_underline_with_content_on_surface(
                scene,
                frame,
                palette,
                self.button.state,
                self.surface,
                Some(text_bounds),
            )
            .expect("validated Link Surface");
            if clip.is_some() {
                scene.pop_clip();
            }
            scene.push_clip(v);
            text.draw(scene, label, role, content, ButtonTextAlign::Leading, ink);
            scene.pop_clip();
        }
        model.button.state = self.button.state;
        self.layout
    }
    pub(crate) fn routed_pointer_move(&mut self, p: Point) -> bool {
        let before = (self.hovered, self.button.state.pressed);
        self.pointer_move(p);
        before != (self.hovered, self.button.state.pressed)
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.hovered = self.window_active && self.visible.is_some_and(|v| v.contains(p));
        if self.pressed && !self.hovered {
            self.button.state.pressed = false;
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        self.pointer_down_with_focus(p, true)
    }
    pub(crate) fn routed_pointer_down(&mut self, p: Point, model: &Link) -> bool {
        self.sync_model(model);
        self.pointer_down_with_focus(p, false)
    }
    fn pointer_down_with_focus(&mut self, p: Point, request_focus: bool) -> bool {
        if !self.window_active
            || self.button.state.disabled
            || self.button.state.loading
            || !self.visible.is_some_and(|v| v.contains(p))
        {
            return false;
        }
        self.pressed = true;
        self.hovered = true;
        if request_focus {
            self.focused = true;
            self.focus_visible = false;
        }
        self.button.pointer_down();
        self.button.state.focused = self.focused;
        self.button.state.focus_visible = self.focus_visible;
        true
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut Link) -> bool {
        self.sync_model(model);
        let armed = std::mem::replace(&mut self.pressed, false);
        let inside = self.window_active && self.visible.is_some_and(|v| v.contains(p));
        let activated = self.button.pointer_up(inside) && armed && self.window_active;
        model.button.state = self.button.state;
        activated
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut Link) -> ComponentEvent {
        self.sync_model(model);
        if k != ControlKey::Enter
            || !self.focused
            || !self.window_active
            || self.button.state.disabled
            || self.button.state.loading
        {
            return ComponentEvent::default();
        }
        self.button.state.focused = true;
        let changed = self.button.key(k, down);
        model.button.state = self.button.state;
        ComponentEvent {
            consumed: true,
            changed,
        }
    }
    pub fn activate(&mut self, model: &mut Link) -> bool {
        self.sync_model(model);
        if model.button.state.disabled || model.button.state.loading {
            return false;
        }
        if !self.focused {
            self.focus(false);
        }
        self.key(ControlKey::Enter, true, model).changed
    }
    // ContentHost owns activation focus; an auxiliary AX action must not create
    // a second focused Link while a composing editor keeps the original focus.
    pub(crate) fn routed_activate(&mut self, model: &mut Link) -> bool {
        self.sync_model(model);
        if !self.window_active || self.button.state.disabled || self.button.state.loading {
            return false;
        }
        self.button.pointer_down();
        let activated = self.button.pointer_up(true);
        self.button.state.focused = self.focused;
        self.button.state.focus_visible = self.focus_visible;
        model.button.state = self.button.state;
        activated
    }
    pub(crate) fn routed_cancel(&mut self) -> bool {
        let changed = self.cancel() || self.hovered;
        self.hovered = false;
        self.button.state.hovered = false;
        changed
    }
    pub fn focus(&mut self, visible: bool) {
        self.focused = true;
        self.focus_visible = visible;
        self.button.state.focused = true;
        self.button.state.focus_visible = visible;
    }
    pub fn blur(&mut self) {
        self.focused = false;
        self.focus_visible = false;
        self.hovered = false;
        self.button.blur();
        self.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        let had = self.pressed || self.button.state.pressed;
        self.pressed = false;
        self.button.cancel();
        had
    }
    pub fn layout(&self) -> ButtonComponentLayout {
        self.layout
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
        }
    }
}
impl Default for LinkView {
    fn default() -> Self {
        Self {
            button: ButtonModel::default(),
            layout: ButtonComponentLayout::default(),
            focused: false,
            focus_visible: false,
            hovered: false,
            pressed: false,
            window_active: true,
            visible: None,
            surface: "canvas",
        }
    }
}
