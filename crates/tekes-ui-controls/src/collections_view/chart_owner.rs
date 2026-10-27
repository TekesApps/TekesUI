//! Explicit original Chart owner layout. No Host, reveal, selection or theme engine.
use super::{
    BarChart, ButtonTextRenderer, ChartGeometry, ChartView, ChartViewLayout, Palette, Rect, Scene,
    chart_label_metrics, draw_chart_labels_with_inks, own_clip, valid_rect,
};
use crate::extended::FocusRingMetrics;
use tekes_ui_scene::Size;

/// Original measured body capacity, independent of clipping and keyboard state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartOwnerCapacity {
    Unmeasured,
    InsufficientAllocation,
    InsufficientBody,
    Sufficient,
}

/// Current original-owner layout result, not a GPU/accessibility receipt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChartOwnerFocusSpace {
    Unmeasured,
    NotRequired,
    Excluded,
    Inactive,
    Insufficient(ChartOwnerCapacity),
    NoVisibleWitness,
    Visible { witness: Rect },
}

/// Measured labels at the actual reserved-body width. This is not a guaranteed
/// minimum usable size: it excludes plot space and does not impose a plot floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartOwnerMeasurement {
    pub capacity: ChartOwnerCapacity,
    pub measured_label_extent: Option<Size>,
}

#[derive(Clone, Debug)]
pub(super) struct OwnerBand {
    allocation: Rect,
    target: Rect,
    focus_clip: Option<Rect>,
    metrics: FocusRingMetrics,
    capacity: ChartOwnerCapacity,
    label_extent: Option<Size>,
    // Exact layout provenance only, never another mutable data/selection model.
    data: Vec<(String, u64)>,
    evidence: std::sync::Arc<()>,
}
impl OwnerBand {
    fn matches(&self, model: &BarChart) -> bool {
        self.data.len() == model.data().len()
            && self
                .data
                .iter()
                .zip(model.data())
                .all(|((label, bits), datum)| {
                    label == &datum.label && *bits == datum.value.to_bits()
                })
    }

    fn visible_witness(&self) -> Option<Rect> {
        self.focus_clip.and_then(|clip| self.witness_in(clip))
    }

    fn required_witness(&self) -> Option<Rect> {
        self.witness_in(self.allocation)
    }

    fn witness_in(&self, clip: Rect) -> Option<Rect> {
        let m = self.metrics;
        let outer = self.target.inset(-m.outset());
        let inner_clip = clip.inset(m.antialias_guard);
        if !positive(inner_clip) {
            return None;
        }
        let radius = m.outset();
        if outer.size.width < 2.0 * radius || outer.size.height < 2.0 * radius {
            return None;
        }
        // Each patch is a full-thickness straight segment of the actual shared
        // ring. Its AA guard also remains in that straight band and real clip.
        let horizontal = |y: f32| {
            if y < inner_clip.min_y() || y + m.width > inner_clip.max_y() {
                return None;
            }
            let left = (outer.min_x() + radius + m.antialias_guard).max(inner_clip.min_x());
            let right = (outer.max_x() - radius - m.antialias_guard).min(inner_clip.max_x());
            if right - left < 2.0 * m.width {
                return None;
            }
            Some(
                Rect::new(
                    left + (right - left - 2.0 * m.width) * 0.5,
                    y,
                    2.0 * m.width,
                    m.width,
                )
                .inset(-m.antialias_guard),
            )
        };
        let vertical = |x: f32| {
            if x < inner_clip.min_x() || x + m.width > inner_clip.max_x() {
                return None;
            }
            let top = (outer.min_y() + radius + m.antialias_guard).max(inner_clip.min_y());
            let bottom = (outer.max_y() - radius - m.antialias_guard).min(inner_clip.max_y());
            if bottom - top < 2.0 * m.width {
                return None;
            }
            Some(
                Rect::new(
                    x,
                    top + (bottom - top - 2.0 * m.width) * 0.5,
                    m.width,
                    2.0 * m.width,
                )
                .inset(-m.antialias_guard),
            )
        };
        horizontal(outer.max_y() - m.width)
            .or_else(|| horizontal(outer.min_y()))
            .or_else(|| vertical(outer.min_x()))
            .or_else(|| vertical(outer.max_x() - m.width))
    }
}

fn positive(rect: Rect) -> bool {
    valid_rect(rect) && rect.size.width > 0.0 && rect.size.height > 0.0
}
fn contains(outer: Rect, inner: Rect) -> bool {
    valid_rect(inner)
        && inner.min_x() >= outer.min_x()
        && inner.min_y() >= outer.min_y()
        && inner.max_x() <= outer.max_x()
        && inner.max_y() <= outer.max_y()
}

fn prepare_owner(
    text: &mut (impl ButtonTextRenderer + ?Sized),
    model: &BarChart,
    allocation: Rect,
    clip: Option<Rect>,
) -> (OwnerBand, ChartViewLayout) {
    let scale = text.backing_scale();
    let metrics = FocusRingMetrics::new(scale);
    let mut owner = OwnerBand {
        allocation,
        target: Rect::default(),
        focus_clip: own_clip(allocation, clip),
        metrics,
        capacity: ChartOwnerCapacity::InsufficientAllocation,
        label_extent: None,
        data: model
            .data()
            .iter()
            .map(|d| (d.label.clone(), d.value.to_bits()))
            .collect(),
        evidence: std::sync::Arc::new(()),
    };
    let mut layout = ChartViewLayout::default();
    if !positive(allocation) {
        return (owner, layout);
    }
    let target = allocation.inset(metrics.reserved_band());
    if !positive(target) {
        return (owner, layout);
    }
    owner.target = target;
    layout.frame = target;
    if !scale.is_finite() || scale <= 0.0 || clip.is_some_and(|r| !valid_rect(r)) {
        owner.capacity = ChartOwnerCapacity::Unmeasured;
        return (owner, layout);
    }
    let (value, category, measured) = chart_label_metrics(text, model, target);
    let (geometry, label_height) = model.layout_with_label_budget(target, value, category);
    owner.label_extent = label_height.filter(|_| measured).map(|height| {
        Size::new(
            allocation.size.width,
            height + 2.0 * metrics.reserved_band(),
        )
    });
    owner.capacity = if !measured {
        ChartOwnerCapacity::Unmeasured
    } else if model.selected().is_none()
        || (positive(geometry.plot)
            && geometry.bars.iter().all(|bar| {
                contains(target, bar.value_label) && contains(target, bar.category_label)
            }))
    {
        ChartOwnerCapacity::Sufficient
    } else {
        ChartOwnerCapacity::InsufficientBody
    };
    if owner.capacity == ChartOwnerCapacity::Sufficient {
        layout.visible = own_clip(target, clip);
    }
    layout.geometry = geometry;
    (owner, layout)
}

impl ChartView {
    /// Pure finite measurement using the original Chart's actual label widths,
    /// text measurement and budget. No geometry or focus is published here.
    pub fn measure_owner(
        &self,
        text: &mut (impl ButtonTextRenderer + ?Sized),
        model: &BarChart,
        width: f32,
        preferred_height: f32,
    ) -> ChartOwnerMeasurement {
        let (owner, _) = prepare_owner(
            text,
            model,
            Rect::new(0.0, 0.0, width, preferred_height),
            None,
        );
        ChartOwnerMeasurement {
            capacity: owner.capacity,
            measured_label_extent: owner.label_extent,
        }
    }

    /// Explicit C allocation: reserve the shared band persistently, including
    /// nonfocus, and arrange original body geometry once. Insufficiency never
    /// blurs or changes selection. Returned layout() is the original body/hit
    /// geometry; owner allocation/paint internals remain private.
    pub fn arrange_owner(
        &mut self,
        text: &mut (impl ButtonTextRenderer + ?Sized),
        model: &mut BarChart,
        allocation: Rect,
        clip: Option<Rect>,
    ) -> ChartOwnerCapacity {
        let (mut owner, layout) = prepare_owner(text, model, allocation, clip);
        owner.evidence = std::sync::Arc::new(());
        if layout.visible.is_none() {
            self.hovered = None;
            model.set_hovered(None);
        }
        model.set_focused(self.focused && self.window_active, self.focus_visible);
        let capacity = owner.capacity;
        self.layout = layout;
        self.owner_band = Some(owner);
        capacity
    }

    /// Pure current-layout/sufficiency/one-witness query for later Host use.
    /// A missing plot with legal data/selection is not "no target".
    pub fn window_active(&self) -> bool {
        self.window_active
    }

    pub fn owner_focus_space(&self, model: &BarChart) -> ChartOwnerFocusSpace {
        let Some(owner) = &self.owner_band else {
            return ChartOwnerFocusSpace::Unmeasured;
        };
        if !owner.matches(model) {
            return ChartOwnerFocusSpace::Unmeasured;
        }
        if model.selected().is_none() {
            return ChartOwnerFocusSpace::NotRequired;
        }
        if owner.capacity == ChartOwnerCapacity::Unmeasured {
            return ChartOwnerFocusSpace::Unmeasured;
        }
        if positive(owner.allocation) && owner.focus_clip.is_none() {
            return ChartOwnerFocusSpace::Excluded;
        }
        match owner.capacity {
            ChartOwnerCapacity::Unmeasured => ChartOwnerFocusSpace::Unmeasured,
            ChartOwnerCapacity::Sufficient => owner
                .visible_witness()
                .map_or(ChartOwnerFocusSpace::NoVisibleWitness, |witness| {
                    ChartOwnerFocusSpace::Visible { witness }
                }),
            capacity => ChartOwnerFocusSpace::Insufficient(capacity),
        }
    }

    pub(crate) fn owner_focus_evidence(&self, model: &BarChart) -> Option<std::sync::Arc<()>> {
        let owner = self.owner_band.as_ref()?;
        (owner.matches(model) && owner.capacity == ChartOwnerCapacity::Sufficient)
            .then(|| owner.evidence.clone())
    }

    /// Current original-owner requirement, independent of parent clipping. This
    /// is the deterministic target for the original Host reveal planner; actual
    /// visibility remains owner_focus_space() after the parent has laid out again.
    pub fn owner_required_focus_space(&self, model: &BarChart) -> ChartOwnerFocusSpace {
        let Some(owner) = &self.owner_band else {
            return ChartOwnerFocusSpace::Unmeasured;
        };
        if !owner.matches(model) {
            return ChartOwnerFocusSpace::Unmeasured;
        }
        if model.selected().is_none() {
            return ChartOwnerFocusSpace::NotRequired;
        }
        match owner.capacity {
            ChartOwnerCapacity::Sufficient => {
                let witness = match self.owner_focus_space(model) {
                    ChartOwnerFocusSpace::Visible { witness } => Some(witness),
                    _ => owner.required_witness(),
                };
                witness.map_or(ChartOwnerFocusSpace::NoVisibleWitness, |witness| {
                    ChartOwnerFocusSpace::Visible { witness }
                })
            }
            ChartOwnerCapacity::Unmeasured => ChartOwnerFocusSpace::Unmeasured,
            capacity => ChartOwnerFocusSpace::Insufficient(capacity),
        }
    }

    /// Checked standalone C entry. A failed admission has no focus/selection
    /// effects. Legacy focus() remains available under the old bare contract.
    pub fn try_focus_owner(
        &mut self,
        keyboard: bool,
        model: &mut BarChart,
    ) -> Result<(), ChartOwnerFocusSpace> {
        if !self.window_active {
            return Err(ChartOwnerFocusSpace::Inactive);
        }
        let space = self.owner_focus_space(model);
        if !matches!(space, ChartOwnerFocusSpace::Visible { .. }) {
            return Err(space);
        }
        self.focus(keyboard, model);
        Ok(())
    }

    /// Explicit invalidation for a successful same-byte data replacement or a
    /// host boundary change. Data, selection and original focus are retained.
    pub fn invalidate_owner_layout(&mut self) {
        self.owner_band = None;
        self.hovered = None;
        self.layout = ChartViewLayout::default();
    }

    /// Paint only the current C arrangement; never remeasure a second hit tree.
    /// Insufficient/fully excluded frames return their synchronous typed result
    /// without pretending to paint a positive focus or mutating focus state.
    pub fn paint_owner(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut BarChart,
        palette: Palette,
    ) -> ChartOwnerFocusSpace {
        self.paint_owner_on_surface(scene, text, model, palette, "canvas")
            .expect("validated default Chart Surface")
    }

    /// Resolve the entire original shared packet before any scene/model effects.
    /// The parent owns the real Surface ground; this never paints a substitute.
    pub fn paint_owner_on_surface(
        &mut self,
        scene: &mut Scene,
        text: &mut (impl ButtonTextRenderer + ?Sized),
        model: &mut BarChart,
        palette: Palette,
        surface: &str,
    ) -> Result<ChartOwnerFocusSpace, tekes_ui_tokens::surface_palette::SurfaceError> {
        let colors = tekes_ui_tokens::surface_palette::resolve_data_plot_surface(palette, surface)?;
        Ok(self.paint_owner_resolved(scene, text, model, colors))
    }

    fn paint_owner_resolved(
        &mut self,
        scene: &mut Scene,
        text: &mut (impl ButtonTextRenderer + ?Sized),
        model: &mut BarChart,
        colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors,
    ) -> ChartOwnerFocusSpace {
        let space = self.owner_focus_space(model);
        let witness_visible = matches!(space, ChartOwnerFocusSpace::Visible { .. });
        let Some(owner) = self.owner_band.as_ref() else {
            return ChartOwnerFocusSpace::Unmeasured;
        };
        if !owner.matches(model) {
            return ChartOwnerFocusSpace::Unmeasured;
        }
        // Missing focus space must not hide otherwise legal visible body ink.
        // The required-focus query stays insufficient; no ring is fabricated.
        let body_only = matches!(space, ChartOwnerFocusSpace::NoVisibleWitness)
            && owner.capacity == ChartOwnerCapacity::Sufficient
            && self.layout.visible.is_some();
        if !witness_visible && !body_only {
            return space;
        }
        let scale = text.backing_scale();
        if !scale.is_finite() || scale <= 0.0 || FocusRingMetrics::new(scale) != owner.metrics {
            return ChartOwnerFocusSpace::Unmeasured;
        }
        let target = owner.target;
        let focus_clip = if witness_visible {
            owner.focus_clip
        } else {
            None
        };
        model.set_focused(self.focused && self.window_active, self.focus_visible);
        model.set_hovered(self.hovered);
        let mut geometry: ChartGeometry = self.layout.geometry.clone();
        for bar in &mut geometry.bars {
            bar.selected = model.selected() == Some(bar.index);
            bar.hovered = model.hovered() == Some(bar.index);
            bar.focused = model.is_focused() && bar.selected;
            bar.focus_visible = model.focus_visible() && bar.selected;
        }
        if let Some(clip) = self.layout.visible {
            scene.push_clip(clip);
            geometry =
                crate::chart::paint_chart_geometry_on_surface(scene, model, geometry, colors);
            draw_chart_labels_with_inks(scene, text, &geometry, |category| {
                if category {
                    colors.category.text
                } else {
                    colors.plot.text
                }
            });
            scene.pop_clip();
        }
        if model.focus_visible()
            && let Some(focus_clip) = focus_clip
        {
            scene.push_clip(focus_clip);
            crate::paint::focus_ring_with_color(
                scene,
                target,
                colors.plot.focus.expect("validated owner focus"),
                0.0,
            );
            scene.pop_clip();
        }
        self.layout.geometry = geometry;
        space
    }

    pub(super) fn owner_data_current(&self, model: &BarChart) -> bool {
        self.owner_band
            .as_ref()
            .is_none_or(|owner| owner.matches(model))
    }
}
