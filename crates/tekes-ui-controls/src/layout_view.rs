//! Component-owned presentation runtimes for layout and navigation controls.
//!
//! Existing model structs stay source compatible. These views bind their model
//! to a single resolved frame, paint through the shared token painters, and own
//! pointer/key arming so Gallery only supplies placement, text shaping, and
//! event translation.
pub use crate::composite::{TabsLayout, TabsView};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
pub use crate::selection::{
    RadioGroupLayout, RadioGroupView, SelectionEvent, SliderLayout, SliderView, SwitchLayout,
    SwitchView,
};
use crate::{
    ButtonTextRenderer, ControlKey,
    extended::{Axis, ControlFrame, Grid, SplitView, Stack},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

pub(super) fn safe_clip(scene: &mut Scene, clip: Option<Rect>) -> bool {
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
pub(super) fn valid(frame: Rect, clip: Option<Rect>) -> bool {
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

pub(super) fn has_area(rect: Rect) -> bool {
    rect.size.width.is_finite()
        && rect.size.height.is_finite()
        && rect.size.width > 0.
        && rect.size.height > 0.
        && rect.max_x().is_finite()
        && rect.max_y().is_finite()
}

pub(super) fn icon_foreground(
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
    /// Resource namespace for retained image leases. Backends supporting bitmap
    /// registration must return an identity that changes with the renderer.
    fn resource_identity(&self) -> Option<u64> {
        None
    }
    /// Register an immutable premultiplied RGBA8 icon through the current
    /// renderer. The returned lease keeps it alive through scene submission.
    fn register_icon_bitmap(
        &mut self,
        _rgba8_premul: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<crate::composition::IconImage, crate::composition::IconRenderError> {
        Err(crate::composition::IconRenderError::UnsupportedRenderer)
    }
    /// Leading edge of the painted outline (including strokes/transforms),
    /// normalized to the canonical artboard width. Used only by explicit
    /// ink-leading Content; ordinary icons keep their centered artboard.
    fn leading_ink_inset(
        &mut self,
        _name: &str,
    ) -> Result<f32, crate::composition::IconRenderError> {
        Err(crate::composition::IconRenderError::UnsupportedRenderer)
    }
    /// Checked resource seam for ordinary Content. Existing drawing backends
    /// remain valid for direct View callers; unsupported Content use is explicit.
    fn prepare_icon(
        &mut self,
        _name: &str,
        _size: f32,
        _backing_scale: f32,
        _tint: tekes_ui_tokens::Rgba,
    ) -> Result<crate::composition::IconImage, crate::composition::IconRenderError> {
        Err(crate::composition::IconRenderError::UnsupportedRenderer)
    }
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
#[derive(Clone, Copy, Debug)]
pub struct IconView {
    layout: IconLayout,
    surface: &'static str,
    leading_ink_inset: Option<f32>,
}
impl Default for IconView {
    fn default() -> Self {
        Self {
            layout: IconLayout::default(),
            surface: "canvas",
            leading_ink_inset: None,
        }
    }
}
impl IconView {
    pub(crate) fn set_leading_ink_inset(&mut self, inset: Option<f32>) {
        self.leading_ink_inset = inset;
    }
    /// Select the actual parent Surface; explicit render_tinted stays an override.
    pub fn set_surface(
        &mut self,
        name: &str,
    ) -> Result<(), tekes_ui_tokens::surface_palette::SurfaceError> {
        self.surface = tekes_ui_tokens::surface_palette::canonical_surface(name)?;
        Ok(())
    }
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
        let tint = tekes_ui_tokens::surface_palette::resolve_surface(
            palette,
            self.surface,
            "content",
            "rest",
        )
        .expect("validated Icon Surface")
        .icon;
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
            proposal::metric("size", "size-icon-standard").expect("Icon natural artboard token")
        };
        let side = natural
            .min(frame.size.width.max(0.))
            .min(frame.size.height.max(0.));
        let mut artboard = layout_measured_children(
            frame,
            Axis::Horizontal,
            &[MeasuredChild::new(Size::new(side, side))],
            0.,
            proposal::layout::alignment(proposal::layout::AlignmentPreset::IconArtboard),
            AlignmentOverride::default(),
        )[0]
        .frame;
        if let Some(inset) = self.leading_ink_inset {
            artboard.origin.x = frame.min_x() - inset * side;
        }
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
        if !valid(bounds, None) || clip.is_some_and(|c| !valid(c, None)) {
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
            divider_hit: ControlFrame::new(
                divider.inset(-tekes_ui_tokens::Spacing::Tight.value()),
                clip,
            )
            .visible_frame(),
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

mod combo_command;
mod disclosure_link;
mod menus_and_pickers;
mod segmented_breadcrumb;

pub(crate) use combo_command::PreparedComboConfirmation;
pub use combo_command::{ComboBoxView, CommandPaletteView};
pub use disclosure_link::{ButtonComponentLayout, DisclosureBodyLayout, DisclosureView, LinkView};
pub use menus_and_pickers::{FilteredMenuView, SelectLayout, SelectView};
pub(crate) use menus_and_pickers::{
    SelectPreparedAction, SelectPreparedInput, SelectPreparedState,
};
pub use segmented_breadcrumb::{
    BreadcrumbLayout, BreadcrumbView, SegmentedControlView, SegmentedLayout,
};
pub(crate) use segmented_breadcrumb::{SegmentedActionSnapshot, SegmentedViewDecision};

#[cfg(test)]
mod tests;
