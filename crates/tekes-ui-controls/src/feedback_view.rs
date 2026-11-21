//! Complete presentation adapters for the existing surface, field, overlay and
//! feedback models. These views own geometry, clipping, hit targets and input
//! state; hosts supply shaped text and forward elapsed fixture time.
pub use crate::composite::{
    DialogEvent, DialogLayout, DialogView, MenuEvent, MenuLayout, MenuView, MenuWheelEvent,
    MenuWheelInput,
};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey,
    Insets, Toolbar, paint,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, proposal};

pub(super) fn finite_rect(r: Rect) -> bool {
    r.min_x().is_finite()
        && r.min_y().is_finite()
        && r.size.width.is_finite()
        && r.size.height.is_finite()
        && r.max_x().is_finite()
        && r.max_y().is_finite()
        && r.size.width >= 0.
        && r.size.height >= 0.
}
pub(super) fn clipped(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
    if !finite_rect(frame) || clip.is_some_and(|c| !finite_rect(c)) {
        return None;
    }
    ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, clip).visible_clip()
}
pub(super) fn valid_logical_target(frame: Rect, clip: Option<Rect>) -> bool {
    finite_rect(frame)
        && frame.size.width > 0.0
        && frame.size.height > 0.0
        && clip
            .is_none_or(|clip| finite_rect(clip) && clip.size.width > 0.0 && clip.size.height > 0.0)
}
pub(super) fn paint_surface_background(
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
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_notice_background(
    scene: &mut Scene,
    frame: Rect,
    ancestor_clip: Option<Rect>,
    visible: Option<Rect>,
    palette: Palette,
    kind: paint::SurfaceKind,
    backing_scale: f32,
    colors: tekes_ui_tokens::surface_palette::SurfaceColors,
) {
    if visible.is_none() {
        return;
    }
    let layout = ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, ancestor_clip);
    if let Some(clip) = layout.background_paint_clip() {
        scene.push_clip(clip);
    }
    paint::surface_kind_with_elevation_colors_at_scale(
        scene,
        frame,
        palette,
        kind,
        backing_scale,
        "panel",
        colors,
    );
    if layout.background_paint_clip().is_some() {
        scene.pop_clip();
    }
}
pub(super) fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    if !finite_rect(a) || !finite_rect(b) {
        return None;
    }
    a.intersection(&b).filter(|r| !r.is_empty())
}
pub(super) fn effective_viewport(viewport: Rect, clip: Option<Rect>) -> Rect {
    match clip {
        Some(clip) => intersect(viewport, clip).unwrap_or_default(),
        None if finite_rect(viewport) => viewport,
        None => Rect::default(),
    }
}
pub(super) fn safe_inset(r: Rect, x: f32, y: f32) -> Rect {
    let x = x.max(0.).min(r.size.width * 0.5);
    let y = y.max(0.).min(r.size.height * 0.5);
    Rect::new(
        r.min_x() + x,
        r.min_y() + y,
        (r.size.width - 2. * x).max(0.),
        (r.size.height - 2. * y).max(0.),
    )
}
pub(super) fn draw_text(
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
/// Align notice markers to the first shaped line, including wrapped messages
/// and host text scaling. Keep multiline text top-aligned; the marker must not
/// drift toward the paragraph center or use an unrelated fixed Y offset.
pub(super) fn notice_marker_frame(
    text: &mut impl ButtonTextRenderer,
    message: &str,
    role: proposal::TextRole,
    message_frame: Rect,
    x: f32,
    diameter: f32,
) -> Rect {
    let first_line = text
        .first_line_height(message, role, message_frame.size.width.max(0.))
        .max(0.)
        .min(message_frame.size.height.max(0.));
    let diameter = diameter.min(first_line);
    Rect::new(
        x,
        message_frame.min_y() + (first_line - diameter) * 0.5,
        diameter,
        diameter,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_centered_text_with_rgba(
    scene: &mut Scene,
    renderer: &mut impl ButtonTextRenderer,
    text: &str,
    role: proposal::TextRole,
    rect: Rect,
    align: ButtonTextAlign,
    ink: tekes_ui_tokens::Rgba,
) {
    if text.is_empty() || rect.is_empty() {
        return;
    }
    renderer.draw(scene, text, role, rect, align, ink);
}
pub(super) fn button_text_align(policy: proposal::layout::AlignmentPolicy) -> ButtonTextAlign {
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
    pub fn paint(scene: &mut Scene, rect: Rect, theme: impl Into<Palette>, panel: bool) {
        paint::surface(scene, rect, theme, panel);
    }
    pub fn paint_at_scale(
        scene: &mut Scene,
        rect: Rect,
        theme: impl Into<Palette>,
        panel: bool,
        backing_scale: f32,
    ) {
        paint::surface_at_scale(scene, rect, theme, panel, backing_scale);
    }
    pub fn paint_kind_at_scale(
        scene: &mut Scene,
        rect: Rect,
        theme: impl Into<Palette>,
        kind: crate::SurfaceKind,
        backing_scale: f32,
    ) {
        paint::surface_kind_at_scale(scene, rect, theme, kind, backing_scale);
    }
    pub fn paint_panel_on_surface_at_scale(
        scene: &mut Scene,
        rect: Rect,
        palette: Palette,
        surface: &str,
        backing_scale: f32,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        paint::panel_on_surface_at_scale(scene, rect, palette, surface, backing_scale)
    }
    pub fn decorative_border(theme: impl Into<Palette>, backing_scale: f32) -> (f32, Rgba) {
        paint::decorative_border(theme, backing_scale)
    }
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

/// Toolbar owns the equal-width action layout and each production ButtonModel.
#[derive(Clone, Debug, Default)]
pub struct ToolbarView {
    buttons: Vec<ButtonModel>,
    layout: Vec<crate::ButtonLayout>,
    focus: Option<usize>,
    disabled: bool,
    gap: f32,
}
impl ToolbarView {
    pub fn new(count: usize, gap: f32) -> Self {
        Self {
            buttons: vec![ButtonModel::default(); count],
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
        self.buttons.resize_with(labels.len(), ButtonModel::default);
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
                ButtonTextAlign::default(),
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

mod fields;
mod notices;
mod overlays;
mod status_and_empty;

pub use fields::{
    FieldLayout, FieldPresentation, FieldView, NumberFieldEvent, NumberFieldLayout, NumberFieldView,
};
pub use notices::{BannerView, ToastView};
pub(crate) use overlays::place_overlay;
pub use overlays::{
    OverlayEvent, OverlayLayout, PopoverPlacement, PopoverView, TooltipEvent, TooltipView,
};
pub use status_and_empty::{EmptyStateView, ProgressView, SkeletonView, StatusBadgeView};

#[cfg(test)]
mod tests_1;
#[cfg(test)]
mod tests_2;
