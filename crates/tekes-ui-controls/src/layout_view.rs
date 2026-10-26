//! Component-owned presentation runtimes for layout and navigation controls.
//!
//! Existing model structs stay source compatible. These views bind their model
//! to a single resolved frame, paint through the shared token painters, and own
//! pointer/key arming so Gallery only supplies placement, text shaping, and
//! event translation.
use crate::control_motion::{ProgressTransition, motion_for};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
use crate::{
    Button, ButtonLayout, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    composite::{MenuEvent, MenuLayout, MenuView},
    extended::{
        Axis, Breadcrumb, ComboBox, CommandPalette, ControlFrame, Disclosure, Grid, Link,
        SegmentedControl, SplitView, Stack,
    },
    paint::ButtonVariant,
};
use std::time::Duration;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

fn safe_clip(scene: &mut Scene, clip: Option<Rect>) -> bool {
    if let Some(clip) = clip {
        if clip.origin.x.is_finite()
            && clip.origin.y.is_finite()
            && clip.size.width.is_finite()
            && clip.size.height.is_finite()
            && clip.max_x().is_finite()
            && clip.max_y().is_finite()
            && clip.size.width > 0.
            && clip.size.height > 0.
        {
            scene.push_clip(clip);
            true
        } else {
            false
        }
    } else {
        false
    }
}
fn valid(frame: Rect, clip: Option<Rect>) -> bool {
    frame.origin.x.is_finite()
        && frame.origin.y.is_finite()
        && frame.size.width.is_finite()
        && frame.size.height.is_finite()
        && frame.max_x().is_finite()
        && frame.max_y().is_finite()
        && frame.size.width >= 0.
        && frame.size.height >= 0.
        && clip.is_none_or(|c| {
            c.origin.x.is_finite()
                && c.origin.y.is_finite()
                && c.size.width.is_finite()
                && c.size.height.is_finite()
                && c.max_x().is_finite()
                && c.max_y().is_finite()
                && c.size.width > 0.
                && c.size.height > 0.
        })
}

fn has_area(rect: Rect) -> bool {
    rect.size.width.is_finite()
        && rect.size.height.is_finite()
        && rect.size.width > 0.
        && rect.size.height > 0.
        && rect.max_x().is_finite()
        && rect.max_y().is_finite()
}

fn icon_foreground(
    palette: Palette,
    state: crate::ControlState,
    variant: ButtonVariant,
) -> tekes_ui_tokens::Rgba {
    let role = if state.disabled {
        "tertiary-text"
    } else if variant == ButtonVariant::Primary {
        "action-primary-foreground"
    } else {
        "primary-text"
    };
    palette
        .color(role)
        .expect("button foreground semantic token")
}

/// Separates event ownership from a model value changing. Hosts use `consumed`
/// for routing and `changed` for invalidation or event logging.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComponentEvent {
    pub consumed: bool,
    pub changed: bool,
}

/// Platform asset backend for one canonical production icon. The backend may
/// resolve/cache SVG raster data or an existing image handle, while IconView
/// owns fitted size, semantic currentColor tint, and the same clip used by hit
/// geometry. No Gallery-only icon artwork belongs in this trait.
pub trait IconRenderer {
    fn draw_icon(
        &mut self,
        scene: &mut Scene,
        canonical_name: &str,
        frame: Rect,
        tint: tekes_ui_tokens::Rgba,
    );
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IconLayout {
    pub frame: Rect,
    pub artboard: Rect,
    pub visible: Option<Rect>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct IconView {
    layout: IconLayout,
}
impl IconView {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        canonical_name: &str,
        frame: Rect,
        requested_size: f32,
        clip: Option<Rect>,
        palette: Palette,
    ) -> IconLayout {
        let tint = palette
            .color("primary-text")
            .expect("icon semantic ink token");
        self.render_tinted(
            scene,
            renderer,
            canonical_name,
            frame,
            requested_size,
            clip,
            palette,
            tint,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_tinted(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        canonical_name: &str,
        frame: Rect,
        requested_size: f32,
        clip: Option<Rect>,
        _palette: Palette,
        tint: tekes_ui_tokens::Rgba,
    ) -> IconLayout {
        if canonical_name.trim().is_empty() || !valid(frame, clip) {
            self.layout = IconLayout::default();
            return self.layout;
        }
        let visible = ControlFrame::new(frame, clip).visible_frame();
        let natural = if requested_size.is_finite() && requested_size > 0. {
            requested_size
        } else {
            proposal::metric("size", "size-icon-preview").expect("Icon natural artboard token")
        };
        let side = natural
            .min(frame.size.width.max(0.))
            .min(frame.size.height.max(0.));
        let artboard = layout_measured_children(
            frame,
            Axis::Horizontal,
            &[MeasuredChild::new(Size::new(side, side))],
            0.,
            proposal::layout::alignment(proposal::layout::AlignmentPreset::IconArtboard),
            AlignmentOverride::default(),
        )[0]
        .frame;
        self.layout = IconLayout {
            frame,
            artboard,
            visible,
        };
        if let Some(v) = visible {
            scene.push_clip(v);
            renderer.draw_icon(scene, canonical_name, artboard, tint);
            scene.pop_clip();
        }
        self.layout
    }
    pub fn layout(&self) -> IconLayout {
        self.layout
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutItems {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub items: Vec<ControlFrame>,
}

/// Stack owns arrangement and clipping; child painting remains with children.
#[derive(Clone, Debug, Default)]
pub struct StackView {
    model: Stack,
    layout: LayoutItems,
}
impl StackView {
    pub fn new(model: Stack) -> Self {
        Self {
            model,
            layout: LayoutItems::default(),
        }
    }
    pub fn model(&self) -> &Stack {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut Stack {
        &mut self.model
    }
    pub fn render(&mut self, bounds: Rect, count: usize, clip: Option<Rect>) -> &LayoutItems {
        self.layout = if valid(bounds, clip) {
            let frame = ControlFrame::new(bounds, clip).frame();
            LayoutItems {
                frame,
                visible: ControlFrame::new(bounds, clip).visible_frame(),
                items: self
                    .model
                    .layout(frame, count)
                    .into_iter()
                    .map(|r| ControlFrame::new(r, clip))
                    .collect(),
            }
        } else {
            LayoutItems::default()
        };
        &self.layout
    }
    pub fn layout(&self) -> &LayoutItems {
        &self.layout
    }
}

/// A mounted grid requires nonzero bounds; the model may still measure natural
/// rows in zero-height constraints. Positive grids retain caller-clipped overflow.
#[derive(Clone, Debug)]
pub struct GridView {
    model: Grid,
    layout: LayoutItems,
}
impl GridView {
    pub fn new(model: Grid) -> Self {
        Self {
            model,
            layout: LayoutItems::default(),
        }
    }
    pub fn model(&self) -> &Grid {
        &self.model
    }
    pub fn model_mut(&mut self) -> &mut Grid {
        &mut self.model
    }
    pub fn render(
        &mut self,
        bounds: Rect,
        count: usize,
        row_height: f32,
        clip: Option<Rect>,
    ) -> &LayoutItems {
        self.layout = if valid(bounds, clip) && has_area(bounds) {
            let frame = ControlFrame::new(bounds, clip).frame();
            LayoutItems {
                frame,
                visible: ControlFrame::new(bounds, clip).visible_frame(),
                items: self
                    .model
                    .layout(frame, count, row_height)
                    .into_iter()
                    .map(|r| ControlFrame::new(r, clip))
                    .collect(),
            }
        } else {
            LayoutItems::default()
        };
        &self.layout
    }
    pub fn layout(&self) -> &LayoutItems {
        &self.layout
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SeparatorLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SeparatorView {
    layout: SeparatorLayout,
}
impl SeparatorView {
    pub fn render(
        &mut self,
        scene: &mut Scene,
        frame: Rect,
        axis: Axis,
        clip: Option<Rect>,
        palette: Palette,
    ) -> SeparatorLayout {
        self.render_at_scale(scene, frame, axis, clip, palette, 1.0)
    }
    /// As `render`, with the host's backing scale for decorative ink only.
    /// Logical layout and ancestor clipping retain the same contract.
    pub fn render_at_scale(
        &mut self,
        scene: &mut Scene,
        frame: Rect,
        axis: Axis,
        clip: Option<Rect>,
        palette: Palette,
        backing_scale: f32,
    ) -> SeparatorLayout {
        self.layout = if valid(frame, clip) {
            let f = ControlFrame::new(frame, clip);
            if let Some(v) = f.visible_frame() {
                let clipped = safe_clip(scene, clip);
                super::extended::separator_at_scale(scene, f.frame(), palette, axis, backing_scale);
                if clipped {
                    scene.pop_clip();
                }
                let _ = v;
            }
            SeparatorLayout {
                frame: f.frame(),
                visible: f.visible_frame(),
            }
        } else {
            SeparatorLayout::default()
        };
        self.layout
    }
    pub fn layout(&self) -> SeparatorLayout {
        self.layout
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SplitViewLayout {
    pub frame: ControlFrame,
    pub leading: ControlFrame,
    pub divider: ControlFrame,
    pub trailing: ControlFrame,
    pub divider_hit: Option<Rect>,
}
#[derive(Clone, Debug)]
pub struct SplitViewView {
    layout: SplitViewLayout,
    axis: Axis,
    dragging: bool,
    window_active: bool,
    toolbar: Option<Rect>,
    toolbar_metrics: Option<crate::toolbar::ToolbarMetrics>,
    toolbar_controls: Vec<Rect>,
}
impl SplitViewView {
    pub fn new(axis: Axis) -> Self {
        Self {
            layout: SplitViewLayout::default(),
            axis,
            dragging: false,
            window_active: true,
            toolbar: None,
            toolbar_metrics: None,
            toolbar_controls: Vec::new(),
        }
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        model: &mut SplitView,
        bounds: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> SplitViewLayout {
        self.render_at_scale(scene, model, bounds, clip, palette, 1.)
    }
    /// Preserve logical pane/divider/hit geometry; paint decorative ink at the
    /// actual host scale through the existing shared separator policy.
    #[allow(clippy::too_many_arguments)]
    pub fn render_at_scale(
        &mut self,
        scene: &mut Scene,
        model: &mut SplitView,
        bounds: Rect,
        clip: Option<Rect>,
        palette: Palette,
        backing_scale: f32,
    ) -> SplitViewLayout {
        let layout = self.arrange(model, bounds, clip);
        self.paint(scene, model, palette, backing_scale);
        layout
    }
    /// Resolve the same pane/divider/capture geometry without painting. A
    /// composition owner can then lay out toolbars and publish their real hits.
    pub fn arrange(
        &mut self,
        model: &mut SplitView,
        bounds: Rect,
        clip: Option<Rect>,
    ) -> SplitViewLayout {
        if !valid(bounds, None) || clip.is_some_and(|c| !valid(c,None)) {
            self.cancel(model);
            self.layout = SplitViewLayout::default();
            return self.layout;
        }
        let f = ControlFrame::new(bounds, clip);
        if !f.visible_frame().is_some_and(has_area) || model.is_constrained(f.frame(), self.axis) {
            self.cancel(model);
        }
        let (leading, divider, trailing) = model.layout(f.frame(), self.axis);
        self.layout = SplitViewLayout {
            frame: f,
            leading: ControlFrame::new(leading, clip),
            divider: ControlFrame::new(divider, clip),
            trailing: ControlFrame::new(trailing, clip),
            divider_hit: ControlFrame::new(divider.inset(-4.), clip).visible_frame(),
        };
        self.layout
    }
    /// Replace the current toolbar band and original control hit regions after
    /// layout/overflow. An empty band removes the chrome policy. Existing drag
    /// capture is never cancelled merely because a control moves beneath it.
    pub fn set_toolbar_region(
        &mut self,
        toolbar: Option<Rect>,
        controls: &[Rect],
        text: &mut dyn ButtonTextRenderer,
    ) {
        self.toolbar = toolbar.filter(|r| valid(*r, None) && has_area(*r));
        self.toolbar_metrics = self
            .toolbar
            .map(|_| crate::toolbar::metrics(text, proposal::TextRole::UiBody));
        self.toolbar_controls = controls
            .iter()
            .copied()
            .filter(|r| valid(*r, None) && has_area(*r))
            .collect();
    }
    pub fn paint(
        &self,
        scene: &mut Scene,
        model: &SplitView,
        palette: Palette,
        backing_scale: f32,
    ) {
        self.paint_presented_divider(
            scene,
            model,
            self.layout.divider.frame(),
            palette,
            backing_scale,
        );
    }
    /// For a shared-layout presentation (such as sidebar visibility motion).
    /// Hosts disable new input during that transition and pass its actual gap;
    /// separator/toolbar masking and the marker still belong to this component.
    pub fn paint_presented_divider(
        &self,
        scene: &mut Scene,
        model: &SplitView,
        divider: Rect,
        palette: Palette,
        backing_scale: f32,
    ) {
        if !model.is_constrained(self.layout.frame.frame(), self.axis)
            && self.layout.divider.visible_frame().is_some_and(has_area)
            && valid(divider, None)
            && has_area(divider)
        {
            let clipped = safe_clip(scene, self.layout.frame.visible_frame());
            let paint_line = |scene: &mut Scene, rect| {
                super::extended::separator_at_scale(
                    scene,
                    rect,
                    palette,
                    match self.axis {
                        Axis::Horizontal => Axis::Vertical,
                        Axis::Vertical => Axis::Horizontal,
                    },
                    backing_scale,
                )
            };
            if let Some((band, metrics)) =
                self.toolbar.zip(self.toolbar_metrics).filter(|(r, _)| {
                    self.axis == Axis::Horizontal && r.intersection(&divider).is_some_and(has_area)
                })
            {
                let top = band.min_y().clamp(divider.min_y(), divider.max_y());
                let bottom = band.max_y().clamp(divider.min_y(), divider.max_y());
                if top > divider.min_y() {
                    paint_line(
                        scene,
                        Rect::new(
                            divider.min_x(),
                            divider.min_y(),
                            divider.size.width,
                            top - divider.min_y(),
                        ),
                    );
                }
                if bottom < divider.max_y() {
                    paint_line(
                        scene,
                        Rect::new(
                            divider.min_x(),
                            bottom,
                            divider.size.width,
                            divider.max_y() - bottom,
                        ),
                    );
                }
                {
                    let height = metrics.box_size.min(bottom - top);
                    let row = metrics.row.min(bottom - top);
                    let (width, color) = crate::paint::decorative_border(palette, backing_scale);
                    let marker = Rect::new(
                        divider.center().x - width * 0.5,
                        top + (row - height) * 0.5,
                        width,
                        height,
                    );
                    if !self
                        .toolbar_controls
                        .iter()
                        .any(|r| r.intersection(&marker).is_some_and(has_area))
                    {
                        scene.push_quad(
                            tekes_ui_scene::Quad::new(marker, color).with_corner_radius(
                                proposal::metric("radius", "radius-pill")
                                    .expect("pill radius token"),
                            ),
                        );
                    }
                }
            } else {
                paint_line(scene, divider);
            }
            if clipped {
                scene.pop_clip();
            }
        }
    }
    pub fn layout(&self) -> SplitViewLayout {
        self.layout
    }
    /// Clipped resize target and movement axis for a host's native hover cursor.
    /// Constrained, hidden, and inactive splitters do not advertise resizing.
    pub fn resize_target(&self, model: &SplitView) -> Option<(Rect, Axis)> {
        self.resize_regions(model)
            .into_iter()
            .max_by(|(a, _), (b, _)| {
                (a.size.width * a.size.height).total_cmp(&(b.size.width * b.size.height))
            })
    }
    /// Disjoint resize cursor/hit regions from the same divider geometry and
    /// original toolbar controls used by paint. Disabled controls also shield
    /// their full hover/hit box; blank toolbar padding remains draggable.
    pub fn resize_regions(&self, model: &SplitView) -> Vec<(Rect, Axis)> {
        if !self.window_active
            || !self.layout.frame.visible_frame().is_some_and(has_area)
            || !self.layout.divider.visible_frame().is_some_and(has_area)
            || model.is_constrained(self.layout.frame.frame(), self.axis)
        {
            return Vec::new();
        }
        let mut regions: Vec<_> = self
            .layout
            .divider_hit
            .filter(|r| has_area(*r))
            .into_iter()
            .collect();
        if let Some(band) = self.toolbar {
            for control in &self.toolbar_controls {
                if let Some(cut) = control.intersection(&band) {
                    regions = regions
                        .into_iter()
                        .flat_map(|r| {
                            let Some(c) = r.intersection(&cut).filter(|r| has_area(*r)) else {
                                return vec![r];
                            };
                            [
                                Rect::new(
                                    r.min_x(),
                                    r.min_y(),
                                    r.size.width,
                                    c.min_y() - r.min_y(),
                                ),
                                Rect::new(
                                    r.min_x(),
                                    c.max_y(),
                                    r.size.width,
                                    r.max_y() - c.max_y(),
                                ),
                                Rect::new(
                                    r.min_x(),
                                    c.min_y(),
                                    c.min_x() - r.min_x(),
                                    c.size.height,
                                ),
                                Rect::new(
                                    c.max_x(),
                                    c.min_y(),
                                    r.max_x() - c.max_x(),
                                    c.size.height,
                                ),
                            ]
                            .into_iter()
                            .filter(|r| has_area(*r))
                            .collect()
                        })
                        .collect();
                }
            }
        }
        regions.into_iter().map(|r| (r, self.axis)).collect()
    }
    pub fn pointer_down(&mut self, p: Point, model: &mut SplitView) -> bool {
        if !self
            .resize_regions(model)
            .iter()
            .any(|(r, _)| r.contains(p))
        {
            return false;
        }
        let axis = match self.axis {
            Axis::Horizontal => p.x,
            Axis::Vertical => p.y,
        };
        self.dragging = model.begin_drag(axis, self.layout.frame.frame(), self.axis);
        self.dragging
    }
    /// A double click resets without starting another captured gesture.
    pub fn double_click(&mut self, p: Point, model: &mut SplitView) -> bool {
        if !self
            .resize_regions(model)
            .iter()
            .any(|(r, _)| r.contains(p))
            || model.is_constrained(self.layout.frame.frame(), self.axis)
        {
            return false;
        }
        self.dragging = false;
        model.reset();
        true
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut SplitView) -> bool {
        if !self.window_active || !self.dragging {
            return false;
        }
        model.drag_to(
            match self.axis {
                Axis::Horizontal => p.x,
                Axis::Vertical => p.y,
            },
            self.layout.frame.frame(),
            self.axis,
        )
    }
    pub fn pointer_up(&mut self, model: &mut SplitView) -> bool {
        let was = std::mem::replace(&mut self.dragging, false);
        model.end_drag();
        was
    }
    pub fn key(&mut self, key: ControlKey, model: &mut SplitView) -> bool {
        if !self.window_active || !self.layout.frame.visible_frame().is_some_and(has_area) {
            return false;
        }
        if key == ControlKey::Escape {
            return self.cancel(model);
        }
        if matches!(key, ControlKey::Enter | ControlKey::Space) {
            self.dragging = false;
        }
        model.key_in(key, self.layout.frame.frame(), self.axis)
    }
    pub fn cancel(&mut self, model: &mut SplitView) -> bool {
        let was = std::mem::replace(&mut self.dragging, false);
        model.cancel_drag() || was
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut SplitView) {
        self.window_active = active;
        if !active {
            self.cancel(model);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonComponentLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub content: Rect,
}
#[derive(Clone, Debug)]
struct ButtonRuntime {
    button: Button,
    layout: ButtonComponentLayout,
    focused: bool,
    window_active: bool,
    press_target: bool,
}
impl Default for ButtonRuntime {
    fn default() -> Self {
        Self {
            button: Button::default(),
            layout: ButtonComponentLayout::default(),
            focused: false,
            window_active: true,
            press_target: false,
        }
    }
}
impl ButtonRuntime {
    #[allow(clippy::too_many_arguments)]
    fn render(
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
    fn render_icon_label(
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
                let ink = icon_foreground(palette, self.button.state, variant);
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
    fn pointer_move(&mut self, p: Point) {
        if self.window_active {
            self.button.pointer_move_at(p);
        }
    }
    fn pointer_down(&mut self, p: Point) -> bool {
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
    fn pointer_up(&mut self, p: Point) -> bool {
        if !self.window_active {
            self.blur();
            return false;
        }
        let was = self.press_target;
        self.press_target = false;
        self.button.pointer_up_at(p) && was
    }
    fn key(&mut self, key: ControlKey, down: bool) -> bool {
        self.window_active
            && self.focused
            && !self.button.state.disabled
            && self.button.key(key, down)
    }
    fn focus(&mut self, visible: bool) {
        self.focused = true;
        self.button.state.focused = true;
        self.button.state.focus_visible = visible;
    }
    fn blur(&mut self) {
        self.focused = false;
        self.press_target = false;
        self.button.blur();
    }
    fn cancel(&mut self) -> bool {
        let had = self.press_target || self.button.state.pressed;
        self.press_target = false;
        self.button.cancel();
        had
    }
    fn active(&mut self, v: bool) {
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

#[derive(Clone, Debug, Default)]
pub struct DisclosureView {
    runtime: ButtonRuntime,
    icon: IconView,
    icon_frame: Rect,
    reveal: ProgressTransition,
    reveal_initialized: bool,
    reveal_suspended: bool,
    motion_now: Duration,
}
impl DisclosureView {
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
        self.icon.render_tinted(
            scene,
            renderer,
            if model.open {
                "chevron-down"
            } else {
                "chevron-right"
            },
            self.icon_frame,
            icon_size,
            clip,
            palette,
            icon_foreground(palette, self.runtime.button.state, ButtonVariant::Quiet),
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
pub struct IconButtonView {
    runtime: ButtonRuntime,
    icon: IconView,
}
impl IconButtonView {
    fn sync_model(&mut self, model: &crate::extended::IconButton) {
        self.runtime.button.state.disabled = model.button.state.disabled;
        self.runtime.button.state.loading = model.button.state.loading;
        self.runtime.button.state.selected = model.button.state.selected;
        if model.button.state.disabled || model.button.state.loading {
            self.runtime.cancel();
        }
    }
    pub fn new() -> Self {
        Self {
            runtime: ButtonRuntime::default(),
            icon: IconView::default(),
        }
    }
    /// Natural square hit size from the existing IconButton contract. Hosts
    /// may use a larger explicit frame; render never silently resizes it.
    pub fn measure(compact: bool) -> Size {
        let token = if compact {
            "size-icon-button-compact"
        } else {
            "size-icon-button"
        };
        let side = proposal::metric("size", token).expect("IconButton hit size token");
        Size::new(side, side)
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::extended::IconButton,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ButtonComponentLayout {
        self.sync_model(model);
        let l = self.runtime.render(
            scene,
            text,
            "",
            frame,
            clip,
            palette,
            proposal::TextRole::UiBody,
            ButtonVariant::Quiet,
            Some(proposal::metric("radius", "radius-control").expect("IconButton radius token")),
        );
        model.button.state = self.runtime.button.state;
        l
    }
    /// Render the icon as part of the component's resolved frame. The Gallery
    /// supplies only its existing asset backend; centering and clipping stay
    /// with this view.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_icon(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        renderer: &mut impl IconRenderer,
        canonical_name: &str,
        model: &mut crate::extended::IconButton,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) -> ButtonComponentLayout {
        let layout = self.render(scene, text, model, frame, clip, palette);
        self.render_icon(scene, renderer, canonical_name, clip, palette, icon_size);
        layout
    }
    /// Paint the icon after the host has released its text adapter borrow.
    /// Geometry still comes from the latest component-owned button layout.
    /// A non-positive/non-finite icon_size uses size-icon-standard; a positive
    /// explicit size remains supported and is fitted by the existing IconView.
    pub fn render_icon(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        canonical_name: &str,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) -> IconLayout {
        let icon_size = if icon_size.is_finite() && icon_size > 0. {
            icon_size
        } else {
            proposal::metric("size", "size-icon-standard").expect("IconButton icon size token")
        };
        self.icon.render_tinted(
            scene,
            renderer,
            canonical_name,
            self.runtime.layout.frame,
            icon_size,
            clip,
            palette,
            icon_foreground(palette, self.runtime.button.state, ButtonVariant::Quiet),
        )
    }
    pub fn icon_layout(&self) -> IconLayout {
        self.icon.layout()
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.runtime.pointer_move(p)
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        self.runtime.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut crate::extended::IconButton) -> bool {
        self.sync_model(model);
        let a = self.runtime.pointer_up(p);
        model.button.state = self.runtime.button.state;
        a
    }
    pub fn key(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut crate::extended::IconButton,
    ) -> bool {
        self.key_event(k, down, model).changed
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut crate::extended::IconButton,
    ) -> ComponentEvent {
        self.sync_model(model);
        let consumed = matches!(k, ControlKey::Enter | ControlKey::Space)
            && self.runtime.window_active
            && self.runtime.focused
            && !model.button.state.disabled
            && !model.button.state.loading;
        let changed = self.runtime.key(k, down);
        model.button.state = self.runtime.button.state;
        ComponentEvent { consumed, changed }
    }
    pub fn activate(&mut self, model: &mut crate::extended::IconButton) -> bool {
        self.sync_model(model);
        if model.button.state.disabled || model.button.state.loading {
            return false;
        }
        self.runtime.focus(false);
        self.key(ControlKey::Enter, true, model)
    }
    pub fn focus(&mut self, v: bool) {
        self.runtime.focus(v)
    }
    pub fn blur(&mut self) {
        self.runtime.blur()
    }
    pub fn layout(&self) -> ButtonComponentLayout {
        self.runtime.layout
    }
    pub fn cancel(&mut self) -> bool {
        self.runtime.cancel()
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.runtime.active(v)
    }
}
impl Default for IconButtonView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct LinkView {
    button: Button,
    layout: ButtonComponentLayout,
    focused: bool,
    focus_visible: bool,
    hovered: bool,
    pressed: bool,
    window_active: bool,
    visible: Option<Rect>,
}
impl LinkView {
    fn sync_model(&mut self, model: &Link) {
        self.button.state.disabled = model.button.state.disabled;
        if model.button.state.disabled {
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
        if !visible.is_some_and(has_area) || model.button.state.disabled {
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
            let ink = crate::extended::link_underline(scene, frame, palette, self.button.state);
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
    pub fn pointer_move(&mut self, p: Point) {
        self.hovered = self.window_active && self.visible.is_some_and(|v| v.contains(p));
        if self.pressed && !self.hovered {
            self.button.state.pressed = false;
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        if !self.window_active
            || self.button.state.disabled
            || !self.visible.is_some_and(|v| v.contains(p))
        {
            return false;
        }
        self.pressed = true;
        self.hovered = true;
        self.focused = true;
        self.focus_visible = false;
        self.button.pointer_down();
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
        if model.button.state.disabled {
            return false;
        }
        if !self.focused {
            self.focus(false);
        }
        self.key(ControlKey::Enter, true, model).changed
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
            button: Button::default(),
            layout: ButtonComponentLayout::default(),
            focused: false,
            focus_visible: false,
            hovered: false,
            pressed: false,
            window_active: true,
            visible: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SegmentedLayout {
    pub frame: Rect,
    pub items: Vec<ButtonLayout>,
}
#[derive(Clone, Debug)]
pub struct SegmentedControlView {
    buttons: Vec<Button>,
    layout: SegmentedLayout,
    focused: Option<usize>,
    focus_visible: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    space_armed: Option<usize>,
    window_active: bool,
    disabled: bool,
}
impl SegmentedControlView {
    pub fn new(count: usize) -> Self {
        Self {
            buttons: vec![Button::default(); count],
            layout: SegmentedLayout::default(),
            focused: None,
            focus_visible: false,
            hovered: None,
            pressed: None,
            space_armed: None,
            window_active: true,
            disabled: false,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut SegmentedControl,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> SegmentedLayout {
        if !valid(frame, clip) {
            self.cancel();
            self.focused = None;
            self.hovered = None;
            self.layout = SegmentedLayout::default();
            return self.layout.clone();
        }
        let count = model.len().min(labels.len()).min(self.buttons.len());
        if self
            .space_armed
            .is_some_and(|i| i >= count || !model.is_enabled(i))
            || self
                .pressed
                .is_some_and(|i| i >= count || !model.is_enabled(i))
        {
            self.cancel();
        }
        if self
            .focused
            .is_some_and(|i| i >= count || !model.is_enabled(i))
        {
            self.focused = (0..count).find(|i| model.is_enabled(*i));
        }
        let cells = Stack::new(Axis::Horizontal).layout(frame, count);
        let mut items = vec![None; count];
        let clipped = safe_clip(scene, clip);
        let focus_last = self.focused.filter(|&index| {
            index < count
                && self.focus_visible
                && self.window_active
                && !self.disabled
                && model.is_enabled(index)
        });
        // Keep hit/layout slots in model order, while keyboard focus paints
        // above adjacent opaque segments using the existing Button painter.
        for i in (0..count)
            .filter(|i| Some(*i) != focus_last)
            .chain(focus_last)
        {
            let button = &mut self.buttons[i];
            let disabled = self.disabled || !model.is_enabled(i);
            button.state.disabled = disabled;
            button.state.selected = model.selected == i;
            button.state.focused = self.window_active && self.focused == Some(i);
            button.state.focus_visible = button.state.focused && self.focus_visible;
            button.state.hovered = self.window_active && self.hovered == Some(i);
            button.state.pressed = self.window_active && self.pressed == Some(i);
            let size = if compact {
                ButtonSize::Sm
            } else {
                ButtonSize::Md
            };
            let l = button.render(
                scene,
                text,
                labels[i],
                cells[i],
                clip,
                palette,
                ButtonVariant::Quiet,
                size,
                role,
                ButtonTextAlign::Leading,
            );
            items[i] = Some(l);
        }
        let items = items
            .into_iter()
            .map(|item| item.expect("every segment retains its original layout slot"))
            .collect();
        if clipped {
            scene.pop_clip();
        }
        self.layout = SegmentedLayout { frame, items };
        if !self
            .layout
            .items
            .iter()
            .any(|item| item.visible.is_some_and(has_area))
        {
            self.cancel();
            self.focused = None;
            self.hovered = None;
        }
        self.layout.clone()
    }
    pub fn pointer_move(&mut self, p: Point) {
        if self.disabled {
            self.hovered = None;
            return;
        }
        self.hovered = self.layout.items.iter().position(|l| l.hit_test(p));
        for (i, b) in self.buttons.iter_mut().enumerate() {
            b.pointer_move_at(p);
            if self.pressed == Some(i) {
                b.state.pressed = self.hovered == Some(i)
            }
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<usize> {
        if !self.window_active || self.disabled {
            return None;
        }
        let i = self.layout.items.iter().position(|l| l.hit_test(p))?;
        if self.buttons[i].state.disabled {
            return None;
        }
        self.buttons[i].pointer_down_at(p);
        self.pressed = Some(i);
        self.focused = Some(i);
        self.focus_visible = false;
        Some(i)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut SegmentedControl) -> Option<usize> {
        let i = self.pressed.take()?;
        let ok = self.buttons.get_mut(i).is_some_and(|b| b.pointer_up_at(p));
        if ok && self.window_active && !self.disabled && model.select(i) {
            Some(i)
        } else {
            None
        }
    }
    pub fn key_event(
        &mut self,
        key: ControlKey,
        down: bool,
        model: &mut SegmentedControl,
    ) -> ComponentEvent {
        let Some(index) = self
            .focused
            .filter(|_| self.window_active && !self.disabled)
        else {
            return ComponentEvent::default();
        };
        if !model.is_enabled(index) {
            self.cancel();
            return ComponentEvent::default();
        }
        if key == ControlKey::Space {
            if down {
                if self.space_armed.is_none() {
                    self.space_armed = Some(index);
                }
                return ComponentEvent {
                    consumed: true,
                    changed: false,
                };
            }
            let armed = self.space_armed.take() == Some(index);
            let changed = armed && model.select(index);
            return ComponentEvent {
                consumed: true,
                changed,
            };
        }
        if !down {
            return ComponentEvent::default();
        }
        let supported = matches!(
            key,
            ControlKey::Left
                | ControlKey::Right
                | ControlKey::Up
                | ControlKey::Down
                | ControlKey::Home
                | ControlKey::End
        );
        if !supported {
            return ComponentEvent::default();
        }
        self.focus_visible = true;
        let changed = model.key_from(key, index);
        if self.focused != Some(model.selected) {
            self.cancel();
        }
        self.focused = Some(model.selected);
        ComponentEvent {
            consumed: true,
            changed,
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut SegmentedControl) -> bool {
        self.key_event(key, down, model).changed
    }
    pub fn select(&mut self, index: usize, model: &mut SegmentedControl) -> bool {
        let changed = !self.disabled && model.select(index);
        if changed {
            self.focused = Some(index);
            self.focus_visible = false;
            self.cancel();
        }
        changed
    }
    pub fn focus(&mut self, i: usize) {
        self.focus_with_visibility(i, true)
    }
    pub fn focus_with_visibility(&mut self, i: usize, visible: bool) {
        if !self.disabled
            && i < self.buttons.len()
            && self.buttons.get(i).is_some_and(|b| !b.state.disabled)
        {
            self.focused = Some(i);
            self.focus_visible = visible;
        }
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        let pointer = self.pressed.take().is_some();
        let keyboard = self.space_armed.take().is_some();
        for b in &mut self.buttons {
            b.cancel();
        }
        pointer || keyboard
    }
    /// Temporarily disable the whole view without changing the controlled
    /// selection or any per-item availability in the model. Disabling cancels
    /// pending pointer/Space activation and clears focus; re-enabling does not
    /// restore an old press or automatically move focus.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.blur();
            self.hovered = None;
        }
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
        }
    }
    /// The current roving-focus item, independent of controlled selection.
    /// Hosts can project the actual focused part without mirroring its state.
    pub fn focused(&self) -> Option<usize> {
        self.focused
    }
    pub fn layout(&self) -> &SegmentedLayout {
        &self.layout
    }
}
impl Default for SegmentedControlView {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BreadcrumbLayout {
    pub frame: Rect,
    pub items: Vec<ControlFrame>,
}
#[derive(Clone, Debug)]
pub struct BreadcrumbView {
    layout: BreadcrumbLayout,
    buttons: Vec<Button>,
    focused: Option<usize>,
    focus_visible: bool,
    hovered: Option<usize>,
    pressed: Option<usize>,
    window_active: bool,
    disabled: bool,
}
impl BreadcrumbView {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Breadcrumb,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> BreadcrumbLayout {
        if !valid(frame, clip) {
            self.cancel();
            self.focused = None;
            self.hovered = None;
            self.layout = BreadcrumbLayout::default();
            return self.layout.clone();
        }
        let n = model.items.len().min(labels.len());
        if self.buttons.len() != n {
            self.cancel();
        }
        if self.focused.is_some_and(|i| i + 1 >= n) {
            self.focused = None;
            self.focus_visible = false;
        }
        if self.hovered.is_some_and(|i| i + 1 >= n) {
            self.hovered = None;
        }
        self.buttons.resize_with(n, Button::default);
        let mut cells = Vec::new();
        let mut x = frame.min_x();
        for (i, label) in labels.iter().take(n).enumerate() {
            let size = Button::measure(
                text,
                label,
                (frame.max_x() - x).max(0.),
                ButtonSize::Body,
                role,
            );
            let w = if i + 1 == n {
                (frame.max_x() - x).max(0.)
            } else {
                (size.width + 22.).min((frame.max_x() - x).max(0.))
            };
            cells.push(Rect::new(x, frame.min_y(), w, frame.size.height));
            x += w;
            if i + 1 < n {
                x += 8.;
            }
        }
        let clipped = safe_clip(scene, clip);
        let mut items = Vec::new();
        for i in 0..n {
            let b = &mut self.buttons[i];
            b.state.disabled = self.disabled || i + 1 == n;
            b.state.selected = i + 1 == n;
            b.state.focused = self.window_active && self.focused == Some(i);
            b.state.focus_visible = b.state.focused && self.focus_visible;
            b.state.hovered = self.window_active && self.hovered == Some(i);
            b.state.pressed = self.window_active && self.pressed == Some(i);
            let l = b.render(
                scene,
                text,
                labels[i],
                cells[i],
                clip,
                palette,
                ButtonVariant::Quiet,
                ButtonSize::Body,
                role,
                ButtonTextAlign::Leading,
            );
            if i + 1 < n {
                let separator = Rect::new(
                    (cells[i].max_x() - 13.0).max(cells[i].min_x()),
                    frame.min_y(),
                    (cells[i].max_x() - 5.0).max(cells[i].min_x())
                        - (cells[i].max_x() - 13.0).max(cells[i].min_x()),
                    frame.size.height,
                );
                text.draw(
                    scene,
                    "/",
                    role,
                    separator,
                    ButtonTextAlign::Center,
                    palette.color("primary-text").expect("primary text token"),
                );
            }
            items.push(ControlFrame::new(l.frame, clip));
        }
        if clipped {
            scene.pop_clip();
        }
        self.layout = BreadcrumbLayout { frame, items };
        if !self
            .layout
            .items
            .iter()
            .any(|item| item.visible_frame().is_some_and(has_area))
            || self.focused.is_some_and(|index| {
                self.layout
                    .items
                    .get(index)
                    .is_none_or(|item| !item.visible_frame().is_some_and(has_area))
            })
        {
            self.cancel();
            self.focused = None;
            self.hovered = None;
        }
        self.layout.clone()
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.hovered = self
            .window_active
            .then(|| self.layout.items.iter().position(|l| l.contains(p)))
            .flatten();
        for b in &mut self.buttons {
            b.pointer_move_at(p);
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> Option<usize> {
        if !self.window_active || self.disabled {
            return None;
        }
        let i = self.layout.items.iter().position(|l| l.contains(p))?;
        if i + 1 == self.layout.items.len() || self.buttons[i].state.disabled {
            return None;
        }
        self.buttons[i].pointer_down_at(p);
        self.pressed = Some(i);
        self.focused = Some(i);
        self.focus_visible = false;
        Some(i)
    }
    pub fn pointer_up(&mut self, p: Point, model: &Breadcrumb) -> Option<usize> {
        let i = self.pressed.take()?;
        if self.window_active
            && !self.disabled
            && self
                .buttons
                .get_mut(i)
                .is_some_and(|button| button.pointer_up_at(p))
        {
            model.activate(i)
        } else {
            None
        }
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &Breadcrumb,
    ) -> (bool, Option<usize>) {
        let Some(i) = self
            .focused
            .filter(|_| self.window_active && !self.disabled)
        else {
            return (false, None);
        };
        // Breadcrumb ancestors are links; Space stays available for page scroll.
        if k == ControlKey::Enter {
            if down && k == ControlKey::Enter {
                return (true, model.activate(i));
            }
            return (true, None);
        }
        (false, None)
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &Breadcrumb) -> Option<usize> {
        self.key_event(k, down, model).1
    }
    pub fn activate(&mut self, index: usize, model: &Breadcrumb) -> Option<usize> {
        if self.disabled || !self.window_active {
            return None;
        }
        let index = model.activate(index)?;
        self.focused = Some(index);
        self.focus_visible = false;
        Some(index)
    }
    pub fn focus(&mut self, i: usize) {
        self.focus_with_visibility(i, true)
    }
    pub fn focus_with_visibility(&mut self, i: usize, visible: bool) {
        if !self.disabled
            && i < self.buttons.len().saturating_sub(1)
            && !self.buttons[i].state.disabled
        {
            self.focused = Some(i);
            self.focus_visible = visible;
        }
    }
    pub fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        let had = self.pressed.take().is_some();
        for b in &mut self.buttons {
            b.cancel();
        }
        had
    }
    /// Disable all navigable ancestors without changing the breadcrumb path.
    /// The current page remains noninteractive regardless of this gate.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.blur();
            self.hovered = None;
        }
    }
    pub fn set_window_active(&mut self, v: bool) {
        self.window_active = v;
        if !v {
            self.blur();
        }
    }
    pub fn layout(&self) -> &BreadcrumbLayout {
        &self.layout
    }
}
impl Default for BreadcrumbView {
    fn default() -> Self {
        Self {
            layout: BreadcrumbLayout::default(),
            buttons: Vec::new(),
            focused: None,
            focus_visible: false,
            hovered: None,
            pressed: None,
            window_active: true,
            disabled: false,
        }
    }
}

/// Shared popup adapter for ComboBox and CommandPalette. The editable text
/// surface remains a host bridge; option layout, clipping, hover, keyboard
/// navigation, and popup dismissal are owned here.
#[derive(Debug)]
pub struct FilteredMenuView {
    menu: crate::Menu,
    menu_view: MenuView,
    mapping: Vec<usize>,
    labels: Vec<String>,
    layout: Option<MenuLayout>,
}
impl FilteredMenuView {
    pub fn is_presented(&self) -> bool {
        self.menu_view.is_presented()
    }

    /// Forward the owned popup scroll lifecycle to the host's frame clock.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.menu_view.set_reduced_motion(reduced)
    }
    pub fn needs_animation(&self) -> bool {
        self.menu_view.needs_animation()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        self.menu_view.tick(dt)
    }
    /// Routes a wheel gesture through the same clipped runtime that paints this popup.
    pub fn wheel_event(&mut self, input: crate::MenuWheelInput) -> crate::MenuWheelEvent {
        self.menu_view.wheel_event_with_model(&self.menu, input)
    }
    pub fn new() -> Self {
        Self {
            menu: crate::Menu::new(0),
            menu_view: MenuView::new_external(),
            mapping: Vec::new(),
            labels: Vec::new(),
            layout: None,
        }
    }
    /// `items` and `mapping` are parallel rendered-candidate snapshots. Each
    /// mapping entry is an index in the original source; `active` is a filtered
    /// row position. Semantic focus/activation accept original indices instead.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        items: &[String],
        mapping: &[usize],
        active: Option<usize>,
        open: bool,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        // A press belongs to the options actually presented at pointer-down.
        // Updating labels/mapping must not retarget its eventual release.
        if self.mapping != mapping || self.labels != items {
            self.cancel();
        }
        self.mapping = mapping.to_vec();
        self.labels = items.to_vec();
        // An open query with no candidates belongs to the caller's empty
        // feedback. No phantom option surface may paint or intercept input.
        if items.is_empty() {
            self.cancel();
            return None;
        }
        self.menu.items.count = items.len();
        self.menu.items.selected = active.unwrap_or(0).min(items.len().saturating_sub(1));
        self.menu.open = open;
        if open {
            let _ = self
                .menu_view
                .set_window_active_with_model(&mut self.menu, true);
        }
        let labels: Vec<&str> = items.iter().map(String::as_str).collect();
        self.layout = None;
        let l = self.menu_view.render_with_model(
            &mut self.menu,
            scene,
            text,
            &labels,
            anchor,
            visible,
            clip,
            palette,
            role,
            compact,
        )?;
        self.layout = Some(l.clone());
        Some(l)
    }
    /// Whether this candidate snapshot still describes the live source model.
    /// Hosts can use the same guard when publishing semantic nodes.
    pub fn is_current(&self, model: &ComboBox) -> bool {
        !model.disabled
            && model.open
            && self.menu.open
            && self.layout.is_some()
            && model.filtered() == self.mapping
            && self.source_matches(model)
    }
    fn source_matches(&self, model: &ComboBox) -> bool {
        self.mapping.len() == self.labels.len()
            && self
                .mapping
                .iter()
                .zip(&self.labels)
                .all(|(index, label)| model.items.get(*index) == Some(label))
    }
    // Confirmation changes the query, not the outgoing rows. Rebuild those
    // existing labels in the current atlas until exit finishes. Actual source
    // replacement still invalidates the snapshot; no alternate model is added.
    fn mapping_for_model(&mut self, model: &ComboBox) -> Vec<usize> {
        if !model.open && self.is_presented() {
            if self.source_matches(model) {
                return self.mapping.clone();
            }
            self.cancel();
        }
        model.filtered()
    }
    fn accepts_model(&mut self, model: &ComboBox) -> bool {
        let current = self.is_current(model);
        if !current {
            if !model.open && !model.disabled && self.source_matches(model) {
                self.dismiss();
            } else {
                self.cancel();
            }
        }
        current
    }
    /// Focus a currently rendered candidate by its original source index.
    /// Rejects closed, disabled and stale query/item snapshots. Focus does not
    /// commit a value; the editable surface keeps ownership of text/IME focus.
    pub fn focus_item(&mut self, original: usize, model: &mut ComboBox) -> bool {
        if !self.accepts_model(model) {
            return false;
        }
        let Some(filtered) = self.mapping.iter().position(|i| *i == original) else {
            return false;
        };
        if !self
            .menu_view
            .focus_item_with_model(&mut self.menu, filtered)
        {
            return false;
        }
        model.active = Some(original);
        true
    }
    /// Confirm a currently rendered candidate and return its original index.
    /// Shares MenuRuntime's mounted guard and the model's normal Enter path.
    pub fn activate_item(&mut self, original: usize, model: &mut ComboBox) -> Option<usize> {
        if !self.accepts_model(model) {
            return None;
        }
        let filtered = self.mapping.iter().position(|i| *i == original)?;
        if !matches!(
            self.menu_view
                .activate_item_with_model(&mut self.menu, filtered),
            MenuEvent::Activated { .. }
        ) {
            return None;
        }
        model.active = Some(original);
        let selected = model.key(ControlKey::Enter);
        self.dismiss();
        selected
    }
    pub fn pointer_move(&mut self, p: Point) -> bool {
        let before = self.menu.items.selected;
        self.menu_view.pointer_move_with_model(&mut self.menu, p);
        before != self.menu.items.selected
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.menu_view.pointer_down_with_model(&mut self.menu, p)
    }
    pub fn pointer_up(&mut self, p: Point) -> MenuEvent {
        self.menu_view.pointer_up_with_model(&mut self.menu, p)
    }
    pub fn key(&mut self, k: ControlKey, down: bool) -> MenuEvent {
        self.menu_view.key_with_model(&mut self.menu, k, down)
    }
    pub fn active_index(&self) -> Option<usize> {
        self.mapping.get(self.menu.items.selected).copied()
    }
    pub fn original_index(&self, filtered_index: usize) -> Option<usize> {
        self.mapping.get(filtered_index).copied()
    }
    pub fn is_visible(&self) -> bool {
        self.layout
            .as_ref()
            .is_some_and(|layout| layout.visible.is_some_and(has_area))
    }
    pub fn dismiss(&mut self) {
        let _ = self.menu_view.deactivate_with_model(&mut self.menu);
        self.layout = None;
        self.menu.open = false;
    }
    pub fn cancel(&mut self) {
        self.menu_view.clear_presentation();
        self.layout = None;
        let _ = self
            .menu_view
            .set_window_active_with_model(&mut self.menu, false);
        self.menu.open = false;
    }
    pub fn layout(&self) -> Option<&MenuLayout> {
        self.layout.as_ref()
    }
}
impl Default for FilteredMenuView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectLayout {
    pub trigger: ButtonLayout,
    pub popup: Option<MenuLayout>,
}
impl Default for SelectLayout {
    fn default() -> Self {
        Self {
            trigger: ButtonLayout {
                frame: Rect::default(),
                content: Rect::default(),
                visible: None,
                label_size: tekes_ui_scene::Size::default(),
            },
            popup: None,
        }
    }
}
#[derive(Debug)]
pub struct SelectView {
    trigger: Button,
    menu: MenuView,
    layout: SelectLayout,
    window_active: bool,
    disabled: bool,
}
impl SelectView {
    /// Logical close may retain outgoing paint before the next frame starts exit.
    pub fn is_presented(&self) -> bool {
        self.menu.is_presented()
    }
    /// Forward the owned popup scroll lifecycle to the host's frame clock.
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.menu.set_reduced_motion(reduced)
    }
    pub fn needs_animation(&self) -> bool {
        self.menu.needs_animation()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        self.menu.tick(dt)
    }
    pub fn new(_count: usize) -> Self {
        Self {
            trigger: Button::default(),
            menu: MenuView::new_external(),
            layout: SelectLayout::default(),
            window_active: true,
            disabled: false,
        }
    }
    pub fn set_disabled(&mut self, disabled: bool, model: &mut crate::Select) {
        self.disabled = disabled;
        self.trigger.state.disabled = disabled;
        self.menu.set_disabled_with_model(&mut model.menu, disabled);
        if disabled {
            self.cancel(model);
        }
    }
    /// Validation belongs to the caller; this projects it through the existing
    /// shared error boundary without disabling selection or clearing its value.
    pub fn set_error(&mut self, error: bool) {
        self.trigger.state.error = error;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        frame: Rect,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> SelectLayout {
        self.render_trigger(scene, text, model, labels, frame, clip, palette, role);
        self.render_options(
            scene, text, model, labels, viewport, clip, palette, role, compact,
        );
        self.layout.clone()
    }
    /// Paint and lay out only the trigger. Call render_options in the host's
    /// overlay pass after base content; render remains the combined convenience.
    #[allow(clippy::too_many_arguments)]
    pub fn render_trigger(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        if !valid(frame, clip) {
            self.cancel(model);
            self.trigger.blur();
            self.trigger.clear_layout();
            self.layout = SelectLayout::default();
            return self.layout.trigger;
        }
        if labels.is_empty() {
            self.cancel(model);
        }
        self.trigger.state.disabled = self.disabled || labels.is_empty();
        self.trigger.state.focused =
            self.trigger.state.focused && self.window_active && !self.trigger.state.disabled;
        let selected = model.selected.min(labels.len().saturating_sub(1));
        let title = labels.get(selected).copied().unwrap_or("");
        let trigger = self.trigger.render(
            scene,
            text,
            title,
            frame,
            clip,
            palette,
            ButtonVariant::Primary,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
        );
        model.menu.items.count = labels.len();
        if !model.menu.open {
            model.menu.items.selected = model.selected.min(labels.len().saturating_sub(1));
        } else if model.menu.items.selected >= labels.len() {
            model.menu.items.selected = labels.len().saturating_sub(1);
        }
        self.layout = SelectLayout {
            trigger,
            popup: None,
        };
        if !trigger.visible.is_some_and(has_area) {
            self.cancel(model);
            self.trigger.blur();
        }
        trigger
    }
    /// Paint the owned menu above the host's base content, using the trigger
    /// layout from this frame. The existing menu still owns its state/lifecycle.
    #[allow(clippy::too_many_arguments)]
    pub fn render_options(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut crate::Select,
        labels: &[&str],
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        model.menu.items.count = labels.len();
        if labels.is_empty() || self.disabled || !self.window_active {
            self.cancel(model);
            self.layout.popup = None;
            return None;
        }
        let trigger = self.layout.trigger;
        if model.menu.open {
            let _ = self
                .menu
                .set_window_active_with_model(&mut model.menu, self.window_active);
        }
        let popup = if model.menu.open || self.menu.is_presented() {
            self.menu.render_with_model(
                &mut model.menu,
                scene,
                text,
                labels,
                trigger.frame,
                viewport,
                clip,
                palette,
                role,
                compact,
            )
        } else {
            None
        };
        self.layout.popup = popup;
        if !self.layout.trigger.visible.is_some_and(has_area)
            || model.menu.open
                && self
                    .layout
                    .popup
                    .as_ref()
                    .is_none_or(|layout| !layout.visible.is_some_and(has_area))
        {
            self.cancel(model);
            self.trigger.blur();
        }
        self.layout.popup.clone()
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut crate::Select) {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return;
        }
        if !self.window_active {
            return;
        }
        if self.disabled {
            // Preserve the latest pointer position while disabled so re-enabling
            // cannot revive hover from the location observed before disabling.
            self.trigger.pointer_move_at(p);
            return;
        }
        self.trigger.pointer_move_at(p);
        if model.menu.open {
            self.menu.pointer_move_with_model(&mut model.menu, p);
        }
    }
    pub fn pointer_down(&mut self, p: Point, model: &mut crate::Select) -> bool {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return false;
        }
        if !self.window_active || self.disabled {
            return false;
        }
        let mut dismissed = false;
        if model.menu.open {
            if self
                .layout
                .popup
                .as_ref()
                .is_some_and(|l| l.visible.is_some_and(|v| v.contains(p)))
            {
                let _ = self.menu.pointer_down_with_model(&mut model.menu, p);
                return true;
            }
            dismissed = matches!(
                self.menu.pointer_down_with_model(&mut model.menu, p),
                MenuEvent::Dismissed { .. }
            );
        }
        if dismissed {
            // The outside press already dismissed this open selection menu.
            // Do not arm its trigger again and reopen on the same release.
            self.trigger.cancel();
            return true;
        }
        self.trigger.pointer_down_at(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut crate::Select) -> MenuEvent {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return MenuEvent::None;
        }
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        if model.menu.open {
            let event = self.menu.pointer_up_with_model(&mut model.menu, p);
            if let MenuEvent::Activated { index, .. } = event {
                let _ = model.choose(index);
            }
            return event;
        }
        if self.trigger.pointer_up_at(p) {
            if model.menu.open {
                model.menu.open = false;
            } else {
                self.menu.show_with_model(&mut model.menu, 0);
                model.menu.items.selected =
                    model.selected.min(model.menu.items.count.saturating_sub(1));
            }
        } else if model.menu.open {
            model.menu.open = false;
        }
        MenuEvent::None
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut crate::Select) -> MenuEvent {
        if model.menu.items.count == 0 {
            self.cancel(model);
            return MenuEvent::None;
        }
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        if model.menu.open {
            let event = self.menu.key_with_model(&mut model.menu, key, down);
            if let MenuEvent::Activated { index, .. } = event {
                let _ = model.choose(index);
            }
            // The owned trigger regains keyboard focus presentation when its
            // menu closes by keyboard, even if a pointer originally opened it.
            if down
                && matches!(
                    event,
                    MenuEvent::Activated { .. } | MenuEvent::Dismissed { .. }
                )
            {
                self.focus(true);
            }
            event
        } else if matches!(key, ControlKey::Enter | ControlKey::Space)
            && self.trigger.key(key, down)
        {
            self.menu.show_with_model(&mut model.menu, 0);
            model.menu.items.selected =
                model.selected.min(model.menu.items.count.saturating_sub(1));
            MenuEvent::None
        } else {
            MenuEvent::None
        }
    }
    /// Focus a mounted popup option without committing the selected value.
    pub fn focus_option(&mut self, index: usize, model: &mut crate::Select) -> bool {
        if !self.window_active || self.disabled {
            return false;
        }
        self.menu.focus_item_with_model(&mut model.menu, index)
    }
    /// Commit through the retained menu, then restore the original trigger focus.
    pub fn activate_option(&mut self, index: usize, model: &mut crate::Select) -> MenuEvent {
        if !self.window_active || self.disabled {
            return MenuEvent::None;
        }
        let event = self.menu.activate_item_with_model(&mut model.menu, index);
        if let MenuEvent::Activated { index, .. } = event {
            let _ = model.choose(index);
            self.focus(true);
        }
        event
    }
    pub fn wheel_event(
        &mut self,
        input: crate::MenuWheelInput,
        model: &crate::Select,
    ) -> crate::MenuWheelEvent {
        if !self.window_active || !model.menu.open || model.menu.items.count == 0 {
            return crate::MenuWheelEvent::Ignored;
        }
        self.menu.wheel_event_with_model(&model.menu, input)
    }
    pub fn focus(&mut self, visible: bool) {
        self.trigger.state.focused = true;
        self.trigger.state.focus_visible = visible;
    }
    pub fn blur(&mut self, model: &mut crate::Select) {
        self.menu.clear_presentation();
        self.trigger.blur();
        model.menu.open = false;
        let _ = self
            .menu
            .set_window_active_with_model(&mut model.menu, false);
    }
    pub fn cancel(&mut self, model: &mut crate::Select) {
        self.menu.clear_presentation();
        self.trigger.cancel();
        model.menu.open = false;
        let _ = self
            .menu
            .set_window_active_with_model(&mut model.menu, false);
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut crate::Select) {
        self.window_active = active;
        let _ = self
            .menu
            .set_window_active_with_model(&mut model.menu, active);
        if !active {
            self.trigger.blur();
            self.cancel(model);
        }
    }
    pub fn layout(&self) -> &SelectLayout {
        &self.layout
    }
}
impl Default for SelectView {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug, Default)]
pub struct ComboBoxView {
    pub options: FilteredMenuView,
}
impl ComboBoxView {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut ComboBox,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        if model.disabled {
            model.open = false;
            self.options.cancel();
        }
        let map = self.options.mapping_for_model(model);
        let filtered: Vec<String> = map.iter().map(|i| model.items[*i].clone()).collect();
        let active = model.active.and_then(|i| map.iter().position(|x| *x == i));
        self.options.render(
            scene, text, &filtered, &map, active, model.open, anchor, visible, clip, palette, role,
            compact,
        )
    }
    pub fn focus_option(&mut self, original: usize, model: &mut ComboBox) -> bool {
        self.options.focus_item(original, model)
    }
    pub fn activate_option(&mut self, original: usize, model: &mut ComboBox) -> Option<usize> {
        self.options.activate_item(original, model)
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut ComboBox) {
        if self.options.accepts_model(model) && self.options.pointer_move(p) {
            model.active = self.options.active_index();
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.options.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut ComboBox) -> Option<usize> {
        if !self.options.accepts_model(model) {
            return None;
        }
        if let MenuEvent::Activated { index, .. } = self.options.pointer_up(p) {
            let original = self.options.original_index(index)?;
            model.active = Some(original);
            let selected = model.key(ControlKey::Enter);
            self.options.dismiss();
            selected
        } else {
            None
        }
    }
    pub fn key_event(&mut self, k: ControlKey, down: bool, model: &mut ComboBox) -> ComponentEvent {
        if model.disabled {
            self.options.cancel();
            return ComponentEvent::default();
        }
        let supported = matches!(
            k,
            ControlKey::Escape | ControlKey::Enter | ControlKey::Up | ControlKey::Down
        );
        let can_open = !model.open && matches!(k, ControlKey::Enter | ControlKey::Down);
        if !supported || (!model.open && !can_open) {
            return ComponentEvent::default();
        }
        let before = (
            model.open,
            model.active,
            model.selected,
            model.query.clone(),
        );
        if down {
            // The model owns the current filtered mapping and active item. A
            // text command can arrive before another render, so routing keys
            // through the last rendered menu would use stale query results.
            let _ = model.key(k);
            if !model.open {
                self.options.dismiss();
            }
        }
        ComponentEvent {
            consumed: true,
            changed: before
                != (
                    model.open,
                    model.active,
                    model.selected,
                    model.query.clone(),
                ),
        }
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut ComboBox) -> Option<usize> {
        let event = self.key_event(k, down, model);
        // Confirmation is an event, including confirmation of the same value.
        // Query/candidate movement, cancellation and key-up are not selections.
        if down && k == ControlKey::Enter && event.changed && !model.open {
            model.selected
        } else {
            None
        }
    }
}
#[derive(Debug)]
pub struct CommandPaletteView {
    pub options: FilteredMenuView,
    trigger: Button,
    trigger_layout: Option<ButtonLayout>,
    window_active: bool,
    return_focus: Option<usize>,
}
impl CommandPaletteView {
    fn sync_trigger_availability(&mut self, model: &mut CommandPalette) -> bool {
        self.trigger.state.disabled = model.combo.disabled;
        if model.combo.disabled || !self.window_active {
            self.cancel(model);
            false
        } else {
            true
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_trigger(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut CommandPalette,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ButtonLayout {
        self.sync_trigger_availability(model);
        self.trigger_layout = Some(self.trigger.render(
            scene,
            text,
            label,
            frame,
            clip,
            palette,
            ButtonVariant::Primary,
            ButtonSize::Body,
            role,
            ButtonTextAlign::Leading,
        ));
        self.trigger_layout.unwrap_or_else(|| ButtonLayout {
            frame: Rect::default(),
            content: Rect::default(),
            visible: None,
            label_size: tekes_ui_scene::Size::default(),
        })
    }
    pub fn pointer_down_trigger(&mut self, p: Point) -> bool {
        if !self.window_active || self.trigger.state.disabled {
            return false;
        }
        self.trigger.pointer_down_at(p)
    }
    pub fn pointer_up_trigger(&mut self, p: Point, model: &mut CommandPalette) -> bool {
        if !self.sync_trigger_availability(model) {
            return false;
        }
        let activated = self.trigger.pointer_up_at(p);
        if activated && self.window_active {
            self.open(model);
            true
        } else {
            false
        }
    }
    pub fn trigger_key(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut CommandPalette,
    ) -> ComponentEvent {
        if !self.sync_trigger_availability(model)
            || !self.trigger.state.focused
            || !matches!(k, ControlKey::Enter | ControlKey::Space)
        {
            return ComponentEvent::default();
        }
        let changed = self.trigger.key(k, down);
        if changed {
            self.open(model);
        }
        ComponentEvent {
            consumed: true,
            changed,
        }
    }
    pub fn open(&mut self, model: &mut CommandPalette) {
        if self.sync_trigger_availability(model) {
            self.return_focus = None;
            model.open();
        }
    }
    /// Open from a caller and request transfer to the host-owned query input.
    /// Returns false when unavailable or already opened from a caller; repeated
    /// activation must not reset the live query or replace the original caller.
    /// This also adopts a palette just opened by the trigger methods.
    pub fn open_from(&mut self, model: &mut CommandPalette, invoker: usize) -> bool {
        if !self.sync_trigger_availability(model)
            || (model.combo.open && self.return_focus.is_some())
        {
            return false;
        }
        if !model.combo.open {
            model.open();
        }
        self.return_focus = Some(invoker);
        self.blur_trigger();
        true
    }
    /// Take the caller once after any close path. The host must validate that
    /// this ID is still mounted and enabled before restoring focus. Scope exit
    /// (`blur` or window deactivation) discards the handoff instead.
    pub fn take_restore_focus(&mut self, model: &CommandPalette) -> Option<usize> {
        if model.combo.open {
            None
        } else {
            self.return_focus.take()
        }
    }
    pub fn focus_trigger(&mut self, visible: bool) {
        self.trigger.state.focused = true;
        self.trigger.state.focus_visible = visible;
    }
    /// Transfer focus to the query or another host target without dismissing
    /// the palette. `blur` continues to release the entire component.
    pub fn blur_trigger(&mut self) {
        self.trigger.blur();
    }
    pub fn blur(&mut self, model: &mut CommandPalette) {
        self.blur_trigger();
        self.cancel(model);
        self.return_focus = None;
    }
    pub fn cancel(&mut self, model: &mut CommandPalette) {
        model.combo.open = false;
        self.trigger.cancel();
        self.options.cancel();
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut CommandPalette) {
        self.window_active = active;
        if !active {
            self.blur(model);
        }
    }
    pub fn trigger_layout(&self) -> Option<ButtonLayout> {
        self.trigger_layout
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut CommandPalette,
        anchor: Rect,
        visible: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        compact: bool,
    ) -> Option<MenuLayout> {
        self.sync_trigger_availability(model);
        if model.combo.open {
            let _ = self
                .options
                .menu_view
                .set_window_active_with_model(&mut self.options.menu, true);
        }
        let map = self.options.mapping_for_model(&model.combo);
        let items: Vec<String> = map.iter().map(|i| model.combo.items[*i].clone()).collect();
        let active = model
            .combo
            .active
            .and_then(|i| map.iter().position(|x| *x == i));
        self.options.render(
            scene,
            text,
            &items,
            &map,
            active,
            model.combo.open,
            anchor,
            visible,
            clip,
            palette,
            role,
            compact,
        )
    }
    pub fn key_event(
        &mut self,
        k: ControlKey,
        down: bool,
        model: &mut CommandPalette,
    ) -> ComponentEvent {
        if model.combo.disabled || !self.window_active {
            self.options.cancel();
            return ComponentEvent::default();
        }
        if !model.combo.open
            || !matches!(
                k,
                ControlKey::Escape | ControlKey::Up | ControlKey::Down | ControlKey::Enter
            )
        {
            return ComponentEvent::default();
        }
        let before = (
            model.combo.open,
            model.combo.active,
            model.combo.selected,
            model.combo.query.clone(),
        );
        if down {
            // CommandPalette shares the ComboBox model contract. Resolve from
            // its live query rather than a menu mapping populated by render.
            let _ = model.combo.key(k);
            if !model.combo.open {
                self.options.dismiss();
            }
        }
        ComponentEvent {
            consumed: true,
            changed: before
                != (
                    model.combo.open,
                    model.combo.active,
                    model.combo.selected,
                    model.combo.query.clone(),
                ),
        }
    }
    pub fn key(&mut self, k: ControlKey, down: bool, model: &mut CommandPalette) -> Option<usize> {
        let event = self.key_event(k, down, model);
        // Executing the same command again must still notify the consumer.
        if down && k == ControlKey::Enter && event.changed && !model.combo.open {
            model.combo.selected
        } else {
            None
        }
    }
    pub fn focus_option(&mut self, original: usize, model: &mut CommandPalette) -> bool {
        self.sync_trigger_availability(model) && self.options.focus_item(original, &mut model.combo)
    }
    pub fn activate_option(
        &mut self,
        original: usize,
        model: &mut CommandPalette,
    ) -> Option<usize> {
        if !self.sync_trigger_availability(model) {
            return None;
        }
        self.options.activate_item(original, &mut model.combo)
    }
    pub fn pointer_move(&mut self, p: Point, model: &mut CommandPalette) {
        if !self.sync_trigger_availability(model) {
            if self.window_active {
                // sync_trigger_availability has applied the live disabled
                // state; update the saved pointer before the next paint.
                self.trigger.pointer_move_at(p);
            } else {
                self.trigger.blur();
            }
            return;
        }
        self.trigger.pointer_move_at(p);
        if self.options.accepts_model(&model.combo) && self.options.pointer_move(p) {
            model.combo.active = self.options.active_index();
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> MenuEvent {
        self.options.pointer_down(p)
    }
    pub fn pointer_up(&mut self, p: Point, model: &mut CommandPalette) -> Option<usize> {
        if !self.window_active {
            self.options.cancel();
            return None;
        }
        if !self.options.accepts_model(&model.combo) {
            return None;
        }
        if let MenuEvent::Activated { index, .. } = self.options.pointer_up(p) {
            let original = self.options.original_index(index)?;
            model.combo.active = Some(original);
            let selected = model.combo.key(ControlKey::Enter);
            self.options.dismiss();
            selected
        } else {
            None
        }
    }
}
impl Default for CommandPaletteView {
    fn default() -> Self {
        Self {
            options: FilteredMenuView::default(),
            trigger: Button::default(),
            trigger_layout: None,
            window_active: true,
            return_focus: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::ControlMotionContext;
    use std::time::Duration;
    use tekes_ui_scene::Size;
    use tekes_ui_tokens::Theme;

    struct FixedText;
    impl ButtonTextRenderer for FixedText {
        fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (text.chars().count() as f32 * 8.).min(max_width),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }

    #[derive(Default)]
    struct DisclosureMotionText {
        now: Duration,
        reduced: bool,
        frames: usize,
    }
    impl ButtonTextRenderer for DisclosureMotionText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(max_width, 20.)
        }
        fn motion_context(&self) -> Option<ControlMotionContext> {
            Some(ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.frames += 1;
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

    #[test]
    fn shared_button_runtime_deactivation_clears_owned_motion_and_stale_hover() {
        let mut runtime = ButtonRuntime::default();
        let mut text = DisclosureMotionText::default();
        let mut scene = Scene::new();
        let frame = Rect::new(20., 20., 160., 40.);
        let render =
            |runtime: &mut ButtonRuntime, text: &mut DisclosureMotionText, scene: &mut Scene| {
                runtime.render(
                    scene,
                    text,
                    "Action",
                    frame,
                    None,
                    Palette::from(Theme::LIGHT),
                    proposal::TextRole::UiBody,
                    ButtonVariant::Quiet,
                    None,
                );
            };
        render(&mut runtime, &mut text, &mut scene);
        runtime.pointer_move(frame.center());
        render(&mut runtime, &mut text, &mut scene);
        assert!(runtime.button.needs_animation());
        text.now = Duration::from_millis(50);
        render(&mut runtime, &mut text, &mut scene);
        runtime.active(false);
        assert!(!runtime.button.needs_animation());
        assert!(!runtime.button.state.hovered);
        runtime.pointer_move(frame.center());
        assert!(!runtime.pointer_up(frame.center()));
        runtime.active(true);
        render(&mut runtime, &mut text, &mut scene);
        assert!(!runtime.button.needs_animation());
        assert!(!runtime.button.state.hovered);
    }

    #[test]
    fn disclosure_body_layout_animates_reversibly_and_respects_motion_and_clip() {
        let mut view = DisclosureView::default();
        let mut model = Disclosure::default();
        let mut text = DisclosureMotionText::default();
        let body = Rect::new(10., 50., 180., 100.);
        let clip = Some(Rect::new(0., 0., 200., 80.));

        let closed = view.layout_body(&mut text, &model, body, clip);
        assert_eq!(closed.occupied_height, 0.);
        assert!(!closed.interactive);

        text.now = Duration::from_millis(1);
        model.open = true;
        let start = view.layout_body(&mut text, &model, body, clip);
        assert_eq!(start.occupied_height, 0.);
        assert!(!start.interactive);

        text.now = Duration::from_millis(151);
        let middle = view.layout_body(&mut text, &model, body, clip);
        assert!(middle.progress > 0. && middle.progress < 1.);
        assert!(middle.interactive);
        assert_eq!(middle.body_rect, body);
        assert_eq!(
            middle.visible_clip.unwrap().size.height,
            middle.occupied_height.min(30.)
        );
        assert!(
            text.frames > 0,
            "active reveal should request another host frame"
        );

        model.open = false;
        text.now = Duration::from_millis(152);
        let reversed = view.layout_body(&mut text, &model, body, clip);
        assert!(reversed.progress > 0.);
        assert!(
            !reversed.interactive,
            "closing content becomes inert immediately"
        );
        text.now = Duration::from_millis(180);
        let collapsing = view.layout_body(&mut text, &model, body, clip);
        assert!(collapsing.progress < reversed.progress);

        model.open = true;
        text.reduced = true;
        text.now = Duration::from_millis(181);
        let reduced = view.layout_body(&mut text, &model, body, clip);
        assert_eq!(reduced.progress, 1.);

        let hidden_clip = Some(Rect::new(400., 400., 20., 20.));
        model.open = false;
        text.reduced = false;
        text.now = Duration::from_millis(182);
        let hidden = view.layout_body(&mut text, &model, body, hidden_clip);
        assert_eq!(hidden.progress, 0.);
        assert!(hidden.visible_clip.is_none());
        assert!(!hidden.interactive);
    }

    #[test]
    fn icon_artboard_uses_json_center_alignment_without_changing_fitted_size() {
        #[derive(Default)]
        struct Capture(Option<Rect>);
        impl IconRenderer for Capture {
            fn draw_icon(&mut self, _: &mut Scene, _: &str, frame: Rect, _: tekes_ui_tokens::Rgba) {
                self.0 = Some(frame);
            }
        }

        let bounds = Rect::new(10., 20., 80., 64.);
        let side = 24.;
        let expected = layout_measured_children(
            bounds,
            Axis::Horizontal,
            &[MeasuredChild::new(Size::new(side, side))],
            0.,
            proposal::layout::alignment(proposal::layout::AlignmentPreset::IconArtboard),
            AlignmentOverride::default(),
        )[0]
        .frame;
        let mut renderer = Capture::default();
        let mut scene = Scene::default();
        let layout = IconView::default().render(
            &mut scene,
            &mut renderer,
            "plus",
            bounds,
            side,
            None,
            Palette::from(Theme::LIGHT),
        );
        assert_eq!(layout.artboard, expected);
        assert_eq!(layout.artboard.size, Size::new(side, side));
        assert_eq!(layout.artboard.center(), bounds.center());
        assert_eq!(renderer.0, Some(expected));
    }

    #[test]
    fn stack_and_grid_views_preserve_frames_and_reject_overflow_geometry() {
        let clip = Rect::new(10., 10., 20., 20.);
        let mut stack = StackView::new(Stack {
            axis: Axis::Horizontal,
            gap: 4.,
        });
        let l = stack
            .render(Rect::new(0., 0., 100., 30.), 2, Some(clip))
            .clone();
        assert_eq!(l.items[0].frame(), Rect::new(0., 0., 48., 30.));
        assert_eq!(
            l.items[0].visible_frame(),
            Some(Rect::new(10., 10., 20., 20.))
        );
        let mut grid = GridView::new(Grid::new(2));
        assert_eq!(
            grid.render(Rect::new(0., 0., 80., 80.), 3, 20., None)
                .items
                .len(),
            3
        );
        assert!(
            stack
                .render(Rect::new(f32::MAX, 0., f32::MAX, 30.), 2, None)
                .items
                .is_empty()
        );
        assert!(
            grid.render(
                Rect::new(0., 0., 80., 80.),
                3,
                20.,
                Some(Rect::new(f32::MAX, 0., f32::MAX, 20.))
            )
            .visible
            .is_none()
        );
    }

    #[test]
    fn disclosure_space_down_is_consumed_and_release_toggles_once() {
        let mut view = DisclosureView::default();
        let mut model = Disclosure::default();
        view.focus(true);
        let down = view.key_event(ControlKey::Space, true, &mut model);
        assert!(down.consumed);
        assert!(!down.changed);
        assert!(!model.open);
        let up = view.key_event(ControlKey::Space, false, &mut model);
        assert!(up.consumed);
        assert!(up.changed);
        assert!(model.open);
        assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    }

    #[test]
    fn disclosure_pointer_activation_requires_visible_matching_press() {
        let mut scene = Scene::default();
        let mut view = DisclosureView::default();
        let mut model = Disclosure::default();
        let frame = Rect::new(12., 20., 260., 40.);
        let palette = Palette::from(Theme::LIGHT);
        let full_clip = Some(Rect::new(0., 0., 400., 300.));
        let mut text = FixedText;
        view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            full_clip,
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
        assert!(!model.open);
        assert!(view.pointer_down(Point::new(30., 30.)));
        assert!(view.pointer_up(Point::new(30., 30.), &mut model));
        assert!(model.open);

        assert!(view.pointer_down(Point::new(30., 30.)));
        let hidden = view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            Some(Rect::new(500., 500., 20., 20.)),
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(hidden.visible.is_none());
        assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
        assert!(model.open);

        model.disabled = true;
        view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            full_clip,
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(!view.pointer_down(Point::new(30., 30.)));
        assert!(!view.pointer_up(Point::new(30., 30.), &mut model));
        assert!(model.open);
    }

    #[test]
    fn disclosure_icon_slot_keeps_label_geometry_stable_across_state_and_text_scale() {
        struct ScaledText(f32);
        impl ButtonTextRenderer for ScaledText {
            fn measure(&mut self, text: &str, _: proposal::TextRole, max_width: f32) -> Size {
                Size::new(
                    (text.chars().count() as f32 * 8. * self.0).min(max_width),
                    18. * self.0,
                )
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: tekes_ui_tokens::Rgba,
            ) {
            }
        }
        #[derive(Default)]
        struct CaptureIcon {
            name: String,
            frame: Rect,
            tint: Option<tekes_ui_tokens::Rgba>,
        }
        impl IconRenderer for CaptureIcon {
            fn draw_icon(
                &mut self,
                _: &mut Scene,
                canonical_name: &str,
                frame: Rect,
                tint: tekes_ui_tokens::Rgba,
            ) {
                self.name = canonical_name.to_owned();
                self.frame = frame;
                self.tint = Some(tint);
            }
        }

        let mut scene = Scene::default();
        let mut view = DisclosureView::default();
        let mut model = Disclosure::default();
        let frame = Rect::new(12., 20., 260., 40.);
        let clip = Some(Rect::new(0., 0., 400., 300.));
        for scale in [1.0, 1.25, 1.5] {
            let mut text = ScaledText(scale);
            let closed = view.render(
                &mut scene,
                &mut text,
                &model,
                "Details",
                frame,
                clip,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
            );
            let closed_icon_frame = view.icon_frame();
            let mut icon = CaptureIcon::default();
            view.render_icon(
                &mut scene,
                &mut icon,
                &model,
                clip,
                Palette::from(Theme::LIGHT),
                16.,
            );
            assert_eq!(icon.name, "chevron-right");
            assert_eq!(icon.frame.center(), closed_icon_frame.center());
            assert_eq!(icon.frame.center().y, closed.frame.center().y);
            assert_eq!(icon.tint, Palette::from(Theme::LIGHT).color("primary-text"));

            model.open = true;
            let opened = view.render(
                &mut scene,
                &mut text,
                &model,
                "Details",
                frame,
                clip,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
            );
            let opened_icon_frame = view.icon_frame();
            view.render_icon(
                &mut scene,
                &mut icon,
                &model,
                clip,
                Palette::from(Theme::LIGHT),
                16.,
            );
            assert_eq!(icon.name, "chevron-down");
            assert_eq!(opened.content.min_x(), closed.content.min_x());
            assert_eq!(opened_icon_frame, closed_icon_frame);
            assert_eq!(icon.frame.center(), opened_icon_frame.center());
            assert_eq!(opened.content.center().y, opened.frame.center().y);
            assert_eq!(closed.content.center().y, closed.frame.center().y);
            model.open = false;
        }
        model.disabled = true;
        let mut text = FixedText;
        view.render(
            &mut scene,
            &mut text,
            &model,
            "Details",
            frame,
            clip,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
        let mut icon = CaptureIcon::default();
        view.render_icon(
            &mut scene,
            &mut icon,
            &model,
            clip,
            Palette::from(Theme::LIGHT),
            16.,
        );
        assert_eq!(
            icon.tint,
            Palette::from(Theme::LIGHT).color("tertiary-text")
        );
    }

    #[test]
    fn combo_box_preserves_closed_open_and_disabled_model_semantics() {
        let mut view = ComboBoxView::default();
        let mut model = ComboBox {
            items: vec!["A".into(), "B".into()],
            active: Some(0),
            ..Default::default()
        };
        let opened = view.key_event(ControlKey::Enter, true, &mut model);
        assert!(opened.consumed && opened.changed);
        assert!(model.open);
        assert_eq!(model.selected, None);
        model.disabled = true;
        let before = (
            model.open,
            model.active,
            model.selected,
            model.query.clone(),
        );
        assert!(!view.key_event(ControlKey::Enter, true, &mut model).consumed);
        assert_eq!(
            before,
            (model.open, model.active, model.selected, model.query)
        );
    }

    #[test]
    fn combo_box_keys_use_live_query_without_an_intervening_render() {
        let mut view = ComboBoxView::default();
        let mut model = ComboBox {
            items: vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
            ..Default::default()
        };
        model.set_query("Com");

        let down = view.key_event(ControlKey::Down, true, &mut model);
        assert!(down.consumed);
        assert_eq!(model.active, Some(2));
        let enter = view.key_event(ControlKey::Enter, true, &mut model);
        assert!(enter.consumed && enter.changed);
        assert_eq!(model.selected, Some(2));
        assert_eq!(model.query, "Comfortable");
        assert!(!model.open);
    }

    #[test]
    fn command_palette_returns_original_invoker_once_for_all_close_paths() {
        let mut view = CommandPaletteView::default();
        let mut model = CommandPalette {
            combo: ComboBox {
                items: vec!["One".into(), "Two".into()],
                ..Default::default()
            },
        };
        for key in [ControlKey::Escape, ControlKey::Enter] {
            assert!(view.open_from(&mut model, 42));
            model.combo.set_query("T");
            assert!(!view.open_from(&mut model, 99));
            assert_eq!(model.combo.query, "T");
            assert_eq!(view.take_restore_focus(&model), None);
            assert!(view.key_event(key, true, &mut model).consumed);
            assert!(!model.combo.open);
            assert_eq!(view.take_restore_focus(&model), Some(42));
            assert_eq!(view.take_restore_focus(&model), None);
        }
        assert!(view.open_from(&mut model, 17));
        view.cancel(&mut model);
        assert_eq!(view.take_restore_focus(&model), Some(17));
    }

    #[test]
    fn command_palette_does_not_replay_focus_after_scope_exit_or_legacy_reopen() {
        let mut view = CommandPaletteView::default();
        let mut model = CommandPalette::default();
        assert!(view.open_from(&mut model, 42));
        view.blur(&mut model);
        assert_eq!(view.take_restore_focus(&model), None);
        assert!(view.open_from(&mut model, 42));
        view.set_window_active(false, &mut model);
        assert_eq!(view.take_restore_focus(&model), None);
        assert!(!view.open_from(&mut model, 99));
        view.set_window_active(true, &mut model);
        assert!(view.open_from(&mut model, 42));
        view.cancel(&mut model);
        view.open(&mut model);
        view.cancel(&mut model);
        assert_eq!(view.take_restore_focus(&model), None);
        model.combo.disabled = true;
        assert!(!view.open_from(&mut model, 99));
        assert_eq!(view.take_restore_focus(&model), None);
    }

    #[test]
    fn command_palette_keys_use_live_query_without_an_intervening_render() {
        let mut view = CommandPaletteView::default();
        let mut model = CommandPalette {
            combo: ComboBox {
                items: vec!["Automatic".into(), "Compact".into(), "Comfortable".into()],
                ..Default::default()
            },
        };
        model.open();
        model.combo.set_query("Com");

        let down = view.key_event(ControlKey::Down, true, &mut model);
        assert!(down.consumed);
        assert_eq!(model.combo.active, Some(2));
        let enter = view.key_event(ControlKey::Enter, true, &mut model);
        assert!(enter.consumed && enter.changed);
        assert_eq!(model.combo.selected, Some(2));
        assert_eq!(model.combo.query, "Comfortable");
        assert!(!model.combo.open);
    }

    #[test]
    fn select_keeps_menu_cursor_across_render_close_and_reopen() {
        let mut view = SelectView::new(3);
        let mut model = crate::Select::new(3);
        let labels = ["One", "Two", "Three"];
        let mut scene = Scene::default();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 180., 40.);
        let viewport = Rect::new(0., 0., 240., 220.);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            MenuEvent::None
        );
        let _ = view.key(ControlKey::Enter, false, &mut model);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let _ = view.key(ControlKey::Down, true, &mut model);
        assert_eq!(model.menu.items.selected, 1, "navigation before render");
        let _ = view.key(ControlKey::Down, false, &mut model);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(model.menu.items.selected, 1, "navigation after render");
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            MenuEvent::Activated {
                index: 1,
                restore_focus: Some(0)
            }
        );
        assert_eq!(model.selected, 1);
        let _ = view.key(ControlKey::Enter, false, &mut model);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let _ = view.key(ControlKey::Enter, true, &mut model);
        assert!(model.menu.open, "menu reopens after prior selection");
        let _ = view.key(ControlKey::Enter, false, &mut model);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let _ = view.key(ControlKey::Down, true, &mut model);
        assert_eq!(
            model.menu.items.selected, 2,
            "reopened navigation before render"
        );
        let _ = view.key(ControlKey::Down, false, &mut model);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(
            model.menu.items.selected, 2,
            "reopened navigation after render"
        );
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            MenuEvent::Activated {
                index: 2,
                restore_focus: Some(0)
            }
        );
        assert_eq!(model.selected, 2);
        let _ = view.key(ControlKey::Enter, false, &mut model);
    }

    #[test]
    fn select_pointer_release_requires_a_matching_down_and_outside_down_dismisses() {
        let mut view = SelectView::new(3);
        let mut model = crate::Select::new(3);
        let labels = ["One", "Two", "Three"];
        let mut scene = Scene::default();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 180., 40.);
        let viewport = Rect::new(0., 0., 240., 220.);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(true);
        let _ = view.key(ControlKey::Enter, true, &mut model);
        let _ = view.key(ControlKey::Enter, false, &mut model);
        let layout = view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let item = layout.popup.as_ref().unwrap().items[1].frame.center();

        assert_eq!(view.pointer_up(item, &mut model), MenuEvent::None);
        assert!(model.menu.open, "an unpaired release must not dismiss");
        assert_eq!(model.selected, 0);

        let _ = view.pointer_down(item, &mut model);
        let outside = Point::new(230., 210.);
        view.pointer_move(outside, &mut model);
        assert_eq!(view.pointer_up(outside, &mut model), MenuEvent::None);
        assert!(model.menu.open, "dragging out does not commit or dismiss");
        assert_eq!(model.selected, 0);

        let _ = view.pointer_down(item, &mut model);
        assert!(matches!(
            view.pointer_up(item, &mut model),
            MenuEvent::Activated { index: 1, .. }
        ));
        assert_eq!(model.selected, 1);
        assert!(!model.menu.open);

        view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        let _ = view.key(ControlKey::Enter, true, &mut model);
        let _ = view.key(ControlKey::Enter, false, &mut model);
        assert!(
            model.menu.open,
            "selection can reopen after pointer capture"
        );
        let _ = view.pointer_down(outside, &mut model);
        assert!(!model.menu.open, "outside pointer-down dismisses the menu");
    }

    #[test]
    fn select_render_does_not_reactivate_an_inactive_menu_runtime() {
        let mut view = SelectView::new(2);
        let mut model = crate::Select::new(2);
        let labels = ["One", "Two"];
        let mut scene = Scene::default();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 180., 40.);
        let viewport = Rect::new(0., 0., 240., 220.);
        view.set_window_active(false, &mut model);
        model.menu.open = true;
        let _ = view.render(
            &mut scene,
            &mut text,
            &mut model,
            &labels,
            frame,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
        assert_eq!(
            view.key(ControlKey::Down, true, &mut model),
            MenuEvent::None
        );
        assert!(!model.menu.open, "inactive rendering cancels the menu");
    }

    #[test]
    fn link_focus_ring_uses_ancestor_clip_while_text_keeps_its_frame_clip() {
        for theme in Theme::ALL {
            let mut view = LinkView::default();
            let mut model = Link::default();
            let frame = Rect::new(20.25, 20.5, 120., 36.);
            let ancestor = Rect::new(4., 4., 180., 90.);
            view.focus(true);
            let mut scene = Scene::new();
            view.render(
                &mut scene,
                &mut FixedText,
                &mut model,
                "Open",
                frame,
                Some(ancestor),
                Palette::from(theme),
                proposal::TextRole::UiBody,
            );
            let mut clips = Vec::new();
            let mut found = false;
            for primitive in scene.primitives() {
                match primitive {
                    tekes_ui_scene::Primitive::PushClip(rect) => clips.push(*rect),
                    tekes_ui_scene::Primitive::PopClip => {
                        clips.pop();
                    }
                    tekes_ui_scene::Primitive::Quad(q)
                        if q.fill.a == 0. && q.border_width == 2. =>
                    {
                        assert_eq!(clips, vec![ancestor]);
                        assert_eq!(q.rect, frame.inset(-4.));
                        assert!(ancestor.contains(q.rect.origin));
                        found = true;
                    }
                    _ => {}
                }
            }
            assert!(found, "keyboard focus must survive the Link's own frame");
            assert_eq!(view.layout.frame, frame);
            assert_eq!(view.layout.visible, Some(frame));
        }
    }

    #[test]
    fn link_hidden_by_clip_cancels_old_pointer_arm() {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let mut scene = Scene::default();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(0., 0., 120., 32.);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            "Open",
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(view.pointer_down(Point::new(20., 16.)));
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            "Open",
            frame,
            Some(Rect::new(300., 300., 30., 30.)),
            palette,
            proposal::TextRole::UiBody,
        );
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            "Open",
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        assert!(!view.pointer_up(Point::new(20., 16.), &mut model));
    }

    #[test]
    fn link_needs_down_and_cancels_when_released_outside() {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let mut scene = Scene::default();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(0., 0., 120., 32.);
        view.render(
            &mut scene,
            &mut text,
            &mut model,
            "Open",
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        view.pointer_move(Point::new(20., 16.));
        assert!(!view.pointer_up(Point::new(20., 16.), &mut model));
        assert!(view.pointer_down(Point::new(20., 16.)));
        view.pointer_move(Point::new(200., 16.));
        assert!(!view.pointer_up(Point::new(200., 16.), &mut model));
    }

    #[test]
    fn segmented_space_is_consumed_and_selects_once_on_release() {
        let mut view = SegmentedControlView::new(2);
        let mut model = SegmentedControl::new(2);
        view.focus_with_visibility(1, true);
        let down = view.key_event(ControlKey::Space, true, &mut model);
        assert!(down.consumed);
        assert!(!down.changed);
        let up = view.key_event(ControlKey::Space, false, &mut model);
        assert!(up.consumed);
        assert!(up.changed);
        assert_eq!(model.selected, 1);
        assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    }

    #[test]
    fn split_resize_target_follows_axis_clip_constraints_and_activation() {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let mut view = SplitViewView::new(axis);
            let mut model = SplitView::new(0.5);
            let bounds = Rect::new(0., 0., 400., 400.);
            let clip = Rect::new(40., 40., 300., 300.);
            assert_eq!(view.resize_target(&model), None);
            let layout = view.render(
                &mut Scene::default(),
                &mut model,
                bounds,
                Some(clip),
                Theme::LIGHT.into(),
            );
            assert_eq!(
                view.resize_target(&model),
                layout.divider_hit.map(|r| (r, axis))
            );
            let (target, _) = view.resize_target(&model).unwrap();
            assert!(target.min_x() >= clip.min_x() && target.max_x() <= clip.max_x());
            assert!(target.min_y() >= clip.min_y() && target.max_y() <= clip.max_y());
            view.set_window_active(false, &mut model);
            assert_eq!(view.resize_target(&model), None);
            view.set_window_active(true, &mut model);
            assert!(view.resize_target(&model).is_some());
            model.min_pane = 300.;
            assert_eq!(view.resize_target(&model), None);
            model.min_pane = 0.;
            view.render(
                &mut Scene::default(),
                &mut model,
                Rect::default(),
                None,
                Theme::LIGHT.into(),
            );
            assert_eq!(view.resize_target(&model), None);
        }
    }

    #[test]
    fn split_view_runtime_cancel_restores_ratio_for_escape_and_invalid_layout() {
        let mut v = SplitViewView::new(Axis::Horizontal);
        let mut model = SplitView::new(0.5);
        let bounds = Rect::new(0., 0., 200., 100.);
        v.render(
            &mut Scene::default(),
            &mut model,
            bounds,
            None,
            Palette::from(Theme::LIGHT),
        );
        assert!(v.pointer_down(Point::new(100., 50.), &mut model));
        assert!(v.pointer_move(Point::new(140., 50.), &mut model));
        assert_ne!(model.ratio(), 0.5);
        assert!(v.key(ControlKey::Escape, &mut model));
        assert_eq!(model.ratio(), 0.5);
        assert!(!v.pointer_move(Point::new(140., 50.), &mut model));
        assert!(!v.pointer_up(&mut model));

        assert!(v.pointer_down(Point::new(100., 50.), &mut model));
        assert!(v.pointer_move(Point::new(140., 50.), &mut model));
        assert_ne!(model.ratio(), 0.5);
        v.render(
            &mut Scene::default(),
            &mut model,
            bounds,
            Some(Rect::new(300., 300., 10., 10.)),
            Palette::from(Theme::LIGHT),
        );
        assert_eq!(
            model.ratio(),
            0.5,
            "a fully clipped split cancels and rolls back"
        );
        assert!(!v.pointer_move(Point::new(150., 50.), &mut model));
        assert!(!v.pointer_up(&mut model));

        v.render(
            &mut Scene::default(),
            &mut model,
            bounds,
            None,
            Palette::from(Theme::LIGHT),
        );
        assert!(v.pointer_down(Point::new(100., 50.), &mut model));
        assert!(v.pointer_move(Point::new(140., 50.), &mut model));
        assert_ne!(model.ratio(), 0.5);
        v.render(
            &mut Scene::default(),
            &mut model,
            Rect::new(f32::NAN, 0., 200., 100.),
            None,
            Palette::from(Theme::LIGHT),
        );
        assert_eq!(model.ratio(), 0.5, "invalid bounds cancel and roll back");
        assert!(!v.pointer_move(Point::new(150., 50.), &mut model));
        assert!(!v.pointer_up(&mut model));
    }

    #[test]
    fn split_double_click_reset_requires_live_divider_and_releases_drag() {
        let mut view = SplitViewView::new(Axis::Horizontal);
        let mut model = SplitView::new(0.7);
        view.render(
            &mut Scene::new(),
            &mut model,
            Rect::new(0., 0., 200., 100.),
            None,
            Palette::from(Theme::LIGHT),
        );
        let divider = view.layout().divider_hit.unwrap().center();
        assert!(!view.double_click(Point::new(10., 10.), &mut model));
        assert_eq!(model.ratio(), 0.7);
        assert!(view.pointer_down(divider, &mut model));
        assert!(view.pointer_move(Point::new(divider.x + 20., divider.y), &mut model));
        assert!(view.double_click(divider, &mut model));
        assert_eq!(model.ratio(), 0.5);
        assert!(!view.pointer_move(divider, &mut model));
        assert!(!view.pointer_up(&mut model));
        model.set_ratio(0.7);
        view.set_window_active(false, &mut model);
        assert!(!view.double_click(divider, &mut model));
        assert_eq!(model.ratio(), 0.7);
    }

    #[test]
    fn split_view_runtime_focus_loss_cancels_and_releases_drag() {
        let mut v = SplitViewView::new(Axis::Horizontal);
        let mut model = SplitView::new(0.5);
        let bounds = Rect::new(0., 0., 200., 100.);
        v.render(
            &mut Scene::default(),
            &mut model,
            bounds,
            None,
            Palette::from(Theme::LIGHT),
        );
        assert!(v.pointer_down(Point::new(100., 50.), &mut model));
        assert!(v.pointer_move(Point::new(140., 50.), &mut model));
        assert_ne!(model.ratio(), 0.5);
        v.set_window_active(false, &mut model);
        assert_eq!(model.ratio(), 0.5);
        assert!(!v.pointer_move(Point::new(140., 50.), &mut model));
        assert!(!v.pointer_up(&mut model));
    }

    #[test]
    fn navigation_invalid_layout_drops_old_button_hit_targets() {
        let mut scene = Scene::new();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(20., 20., 160., 40.);
        let invalid = Rect::new(f32::NAN, 20., 160., 40.);
        let mut icon = IconButtonView::new();
        let mut icon_model = crate::extended::IconButton::new("Action").unwrap();
        icon.render(&mut scene, &mut text, &mut icon_model, frame, None, palette);
        assert!(icon.pointer_down(frame.center()));
        icon.render(
            &mut scene,
            &mut text,
            &mut icon_model,
            invalid,
            None,
            palette,
        );
        assert!(!icon.pointer_up(frame.center(), &mut icon_model));
        assert!(!icon.pointer_down(frame.center()));
        icon.focus(true);
        // A hidden invalidated button must not retain an old pointer placement.
        assert!(icon.runtime.button.layout().is_none());

        let mut select = SelectView::new(2);
        let mut model = crate::Select::new(2);
        for bounds in [frame, invalid] {
            select.render(
                &mut scene,
                &mut text,
                &mut model,
                &["A", "B"],
                bounds,
                Rect::new(0., 0., 400., 300.),
                None,
                palette,
                proposal::TextRole::UiBody,
                false,
            );
            if bounds == frame {
                select.focus(true);
            }
        }
        assert!(!select.pointer_down(frame.center(), &mut model));
        select.key(ControlKey::Enter, true, &mut model);
        assert!(!model.menu.open);
    }

    #[test]
    fn breadcrumb_shrink_releases_press_and_invalid_roving_focus() {
        let mut view = BreadcrumbView::default();
        let mut model = Breadcrumb {
            items: vec!["A".into(), "B".into(), "C".into()],
        };
        let mut scene = Scene::new();
        let mut text = FixedText;
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(0., 0., 320., 40.);
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["A", "B", "C"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        let point = view.layout.items[1].frame().center();
        assert_eq!(view.pointer_down(point), Some(1));
        model.items.truncate(1);
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["A"],
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        assert_eq!(view.pointer_up(point, &model), None);
        assert_eq!(
            view.key_event(ControlKey::Enter, true, &model),
            (false, None)
        );
        assert!(!view.buttons[0].state.focused);
    }

    #[test]
    fn split_hidden_divider_does_not_accept_keyboard_reset() {
        let mut view = SplitViewView::new(Axis::Horizontal);
        let mut model = SplitView::new(0.7);
        view.render(
            &mut Scene::new(),
            &mut model,
            Rect::new(0., 0., 200., 100.),
            Some(Rect::new(300., 300., 20., 20.)),
            Palette::from(Theme::LIGHT),
        );
        assert!(!view.key(ControlKey::Enter, &mut model));
        assert_eq!(model.ratio(), 0.7);
    }

    #[test]
    fn icon_button_preserves_model_loading_and_selected_state() {
        let mut view = IconButtonView::new();
        let mut model = crate::extended::IconButton::new("Action").unwrap();
        model.button.state.loading = true;
        model.button.state.selected = true;
        let frame = Rect::new(0., 0., 40., 40.);
        view.render(
            &mut Scene::new(),
            &mut FixedText,
            &mut model,
            frame,
            None,
            Palette::from(Theme::LIGHT),
        );
        assert!(model.button.state.loading && model.button.state.selected);
        assert!(!view.pointer_down(frame.center()));
        view.focus(true);
        assert_eq!(
            view.key_event(ControlKey::Enter, true, &mut model),
            ComponentEvent::default()
        );
    }

    #[test]
    fn link_deactivation_clears_separately_owned_hover() {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let frame = Rect::new(0., 0., 160., 40.);
        let render = |view: &mut LinkView, model: &mut Link| {
            view.render(
                &mut Scene::new(),
                &mut FixedText,
                model,
                "Link",
                frame,
                None,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
            );
        };
        render(&mut view, &mut model);
        view.pointer_move(frame.center());
        render(&mut view, &mut model);
        assert!(model.button.state.hovered);
        view.set_window_active(false);
        view.set_window_active(true);
        render(&mut view, &mut model);
        assert!(!model.button.state.hovered);
    }

    #[test]
    fn select_reopens_with_selected_item_revealed_in_unchanged_popup() {
        let mut view = SelectView::new(10);
        let mut model = crate::Select::new(10);
        let labels = ["A"; 10];
        let mut scene = Scene::new();
        let mut text = FixedText;
        let render = |view: &mut SelectView,
                      model: &mut crate::Select,
                      scene: &mut Scene,
                      text: &mut FixedText| {
            view.render(
                scene,
                text,
                model,
                &labels,
                Rect::new(20., 20., 160., 40.),
                Rect::new(0., 0., 300., 180.),
                None,
                Palette::from(Theme::LIGHT),
                proposal::TextRole::UiBody,
                false,
            )
        };
        render(&mut view, &mut model, &mut scene, &mut text);
        view.focus(true);
        view.key(ControlKey::Enter, true, &mut model);
        render(&mut view, &mut model, &mut scene, &mut text);
        view.key(ControlKey::Escape, true, &mut model);
        model.selected = 9;
        render(&mut view, &mut model, &mut scene, &mut text);
        view.key(ControlKey::Enter, true, &mut model);
        let popup = render(&mut view, &mut model, &mut scene, &mut text)
            .popup
            .unwrap();
        assert!(popup.scroll_offset > 0.);
        assert!(popup.items[9].visible.is_some_and(has_area));
    }
}
