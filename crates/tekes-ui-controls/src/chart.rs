//! A controlled, single-series bar chart model and token-driven scene adapter.
//!
//! This deliberately implements a bar chart only. It accepts an arbitrary bounded
//! number of categories supplied by the consumer; it does not claim to implement
//! line, scatter, multi-series, or time-series charts. Text shaping remains the
//! responsibility of the host, using the label bounds returned by [`ChartGeometry`].

use crate::ControlKey;
use tekes_ui_scene::{Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

/// One named category and its numeric value.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartDatum {
    pub label: String,
    pub value: f64,
}

impl ChartDatum {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// Atomic replacement failure for an updated finite chart data set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChartDataUpdateError {
    pub index: usize,
}

/// Controlled state for a single-series vertical bar chart.
///
/// The data is bounded by the consumer before construction. Non-finite values are
/// retained for diagnostics, omitted from layout and skipped by keyboard selection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarChart {
    data: Vec<ChartDatum>,
    selected: Option<usize>,
    hovered: Option<usize>,
    focused: bool,
    focus_visible: bool,
}

impl BarChart {
    pub fn new(data: Vec<ChartDatum>) -> Self {
        let mut chart = Self {
            data,
            ..Self::default()
        };
        chart.selected = chart.next_valid(0, 1, true);
        chart
    }

    pub fn data(&self) -> &[ChartDatum] {
        &self.data
    }

    /// Replace the finite data set without replacing this model or its owner.
    /// Category labels, including blank and duplicate labels, remain literal data.
    /// Existing selection is retained by slot when that slot remains finite;
    /// otherwise selection advances to the next finite slot. Hover is cleared.
    /// Validation completes before any model state changes.
    pub fn try_update_data(&mut self, data: Vec<ChartDatum>) -> Result<(), ChartDataUpdateError> {
        if let Some((index, _)) = data.iter().enumerate().find(|(_, d)| !d.value.is_finite()) {
            return Err(ChartDataUpdateError { index });
        }
        let previous = self.selected;
        self.data = data;
        self.selected = previous
            .filter(|&index| self.is_valid(index))
            .or_else(|| {
                previous.and_then(|index| self.next_valid(index.min(self.data.len()), 1, true))
            })
            .or_else(|| self.next_valid(0, 1, true));
        self.hovered = None;
        Ok(())
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub(crate) fn selectable(&self, index: usize) -> bool {
        self.is_valid(index)
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn focus_visible(&self) -> bool {
        self.focus_visible
    }

    pub fn set_focused(&mut self, focused: bool, focus_visible: bool) {
        self.focused = focused;
        self.focus_visible = focused && focus_visible;
    }

    pub fn set_hovered(&mut self, index: Option<usize>) {
        self.hovered = index.filter(|&i| self.is_valid(i));
    }

    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Select a finite data point by its stable index in the supplied data.
    pub fn select(&mut self, index: usize) -> bool {
        if !self.is_valid(index) || self.selected == Some(index) {
            return false;
        }
        self.selected = Some(index);
        true
    }

    /// Move selection with chart semantics: left/down move backward, right/up
    /// move forward, and Home/End choose the first/last finite datum.
    ///
    /// A successful key move also establishes keyboard-visible focus. The return
    /// value indicates whether the selection changed.
    pub fn key(&mut self, key: ControlKey) -> bool {
        let direction = match key {
            ControlKey::Left | ControlKey::Down => -1,
            ControlKey::Right | ControlKey::Up => 1,
            ControlKey::Home => {
                let next = self.next_valid(0, 1, true);
                return self.set_from_key(next);
            }
            ControlKey::End => {
                let start = self.data.len().saturating_sub(1);
                let next = self.next_valid(start, -1, true);
                return self.set_from_key(next);
            }
            _ => return false,
        };
        let start = self.selected.map_or_else(
            || {
                if direction > 0 {
                    0
                } else {
                    self.data.len().saturating_sub(1)
                }
            },
            |i| i.saturating_add_signed(direction),
        );
        let next = self.next_valid(start, direction, false);
        self.set_from_key(next)
    }

    /// Original horizontal slots, independent of whether vertical label rows fit.
    pub(crate) fn label_widths(&self, bounds: Rect) -> Option<(f32, f32)> {
        let bounds = finite_bounds(bounds)?;
        if self.data.is_empty() {
            return None;
        }
        let outer = token_metric("spacing", "space-regular");
        let width = (bounds.size.width - outer * 2.0).max(0.0);
        if width <= 0.0 {
            return None;
        }
        let slot_width = width / self.data.len() as f32;
        let gap = token_metric("spacing", "space-tight");
        let bar_width = (slot_width - gap).max(0.0).min(slot_width * 0.72);
        Some((bar_width, slot_width))
    }

    /// Calculate bar and label geometry from the actual data and current bounds.
    /// Empty, non-finite, and degenerate bounds produce safe empty geometry.
    pub fn layout(&self, bounds: Rect) -> ChartGeometry {
        self.layout_with_text_scale(bounds, 1.0)
    }

    /// Calculate layout while reserving label space for the active text scale.
    ///
    /// The label region grows with the supplied scale but is capped to 45% of the
    /// viewport height. If the remaining plot becomes too small, geometry is empty
    /// instead of escaping the viewport. Invalid scales fall back to the default.
    pub fn layout_with_text_scale(&self, bounds: Rect, text_scale: f32) -> ChartGeometry {
        let Some(bounds) = finite_bounds(bounds) else {
            return ChartGeometry::default();
        };
        if self.data.is_empty() || bounds.is_empty() {
            return ChartGeometry {
                bounds,
                ..ChartGeometry::default()
            };
        }

        let outer = token_metric("spacing", "space-regular");
        let label_gap = token_metric("spacing", "space-tight");
        let scale = if text_scale.is_finite() && text_scale > 0.0 {
            text_scale
        } else {
            1.0
        };
        let label_height =
            (token_metric("spacing", "space-loose") * scale).min(bounds.size.height * 0.45);
        let inset = Rect::new(
            bounds.min_x() + outer,
            bounds.min_y() + outer,
            (bounds.size.width - outer * 2.0).max(0.0),
            (bounds.size.height - outer * 2.0).max(0.0),
        );
        let plot_height = (inset.size.height - label_height - label_gap).max(0.0);
        let plot = Rect::new(inset.min_x(), inset.min_y(), inset.size.width, plot_height);
        if plot.is_empty() {
            return ChartGeometry {
                bounds,
                plot,
                ..ChartGeometry::default()
            };
        }

        // Normalize before finding the range so finite values near +/- f64::MAX
        // cannot overflow while calculating the domain span.
        let max_abs = self
            .data
            .iter()
            .map(|datum| datum.value.abs())
            .filter(|value| value.is_finite())
            .fold(0.0_f64, f64::max);
        if max_abs == 0.0 {
            return self.layout_zero_values(bounds, plot, inset, label_height, label_gap);
        }
        let mut min_value = 0.0_f64;
        let mut max_value = 0.0_f64;
        for datum in &self.data {
            if datum.value.is_finite() {
                let normalized = datum.value / max_abs;
                min_value = min_value.min(normalized);
                max_value = max_value.max(normalized);
            }
        }
        let span = (max_value - min_value).max(f64::EPSILON);
        let baseline = (plot.min_y() as f64 + (max_value / span) * plot.size.height as f64) as f32;

        let (bar_width, slot_width) = self.label_widths(bounds).expect("nonempty chart plot");
        let value_label_height = label_height * 0.5;
        let category_label_height = label_height - value_label_height;
        let mut bars = Vec::with_capacity(self.data.len());
        for (index, datum) in self.data.iter().enumerate() {
            if !datum.value.is_finite() {
                continue;
            }
            let normalized = datum.value / max_abs;
            let value_y = (baseline as f64 - normalized / span * plot.size.height as f64) as f32;
            let x = plot.min_x() + slot_width * index as f32 + (slot_width - bar_width) * 0.5;
            let y = value_y.min(baseline);
            let height = (baseline - value_y).abs();
            let value_label = if value_y < baseline {
                Rect::new(
                    x,
                    (value_y - value_label_height).max(bounds.min_y()),
                    bar_width,
                    value_label_height,
                )
            } else {
                Rect::new(x, value_y.min(plot.max_y()), bar_width, value_label_height)
            };
            let category_label = Rect::new(
                plot.min_x() + slot_width * index as f32,
                plot.max_y() + label_gap,
                slot_width,
                category_label_height,
            );
            bars.push(BarGeometry {
                index,
                label: datum.label.clone(),
                value: datum.value,
                rect: Rect::new(x, y, bar_width, height),
                value_label,
                category_label,
                selected: self.selected == Some(index),
                hovered: self.hovered == Some(index),
                focused: self.focused && self.selected == Some(index),
                focus_visible: self.focus_visible && self.selected == Some(index),
            });
        }
        finalize_category_row(
            ChartGeometry {
                bounds,
                plot,
                baseline,
                bars,
                invalid_count: self
                    .data
                    .iter()
                    .filter(|datum| !datum.value.is_finite())
                    .count(),
            },
            label_gap,
        )
    }

    /// Layout using the host text shaper's actual caption line heights.
    ///
    /// This is the production path for views that shape labels before painting:
    /// value labels are given a row above/below the plot endpoint, and category
    /// labels receive their own row after all lower value labels. Unused upper
    /// space is omitted for nonpositive data. The older scale-only API is
    /// retained for consumers that do not provide shaping metrics.
    pub fn layout_with_label_metrics(
        &self,
        bounds: Rect,
        value_line_height: f32,
        category_line_height: f32,
    ) -> ChartGeometry {
        self.layout_with_label_budget(bounds, value_line_height, category_line_height)
            .0
    }

    /// Original #115 geometry plus its reserved label-only vertical budget.
    /// The chart coordinates and calculations below are unchanged.
    pub(crate) fn layout_with_label_budget(
        &self,
        bounds: Rect,
        value_line_height: f32,
        category_line_height: f32,
    ) -> (ChartGeometry, Option<f32>) {
        let Some(bounds) = finite_bounds(bounds) else {
            return (ChartGeometry::default(), None);
        };
        if self.data.is_empty() || bounds.is_empty() {
            return (
                ChartGeometry {
                    bounds,
                    ..ChartGeometry::default()
                },
                Some(0.0),
            );
        }
        let outer = token_metric("spacing", "space-regular");
        let gap = token_metric("spacing", "space-tight");
        let value_height = sane_line_height(value_line_height);
        let category_height = sane_line_height(category_line_height);
        let inset = Rect::new(
            bounds.min_x() + outer,
            bounds.min_y() + outer,
            (bounds.size.width - outer * 2.0).max(0.0),
            (bounds.size.height - outer * 2.0).max(0.0),
        );
        let has_positive = self
            .data
            .iter()
            .any(|d| d.value.is_finite() && d.value > 0.0);
        let has_nonpositive = self
            .data
            .iter()
            .any(|d| d.value.is_finite() && d.value <= 0.0);
        let top = if has_positive || !has_nonpositive {
            value_height + gap
        } else {
            0.0
        };
        let mut bottom = if has_nonpositive { value_height } else { 0.0 };
        // One monotonic promotion handles positive endpoints rounded onto baseline.
        for _ in 0..2 {
            let reserved = if bottom == 0.0 && top > 0.0 {
                value_height + category_height + gap * 2.0
            } else {
                top + bottom + category_height + gap
            };
            let plot_height = (inset.size.height - reserved).max(0.0);
            let plot = Rect::new(
                inset.min_x(),
                inset.min_y() + top,
                inset.size.width,
                plot_height,
            );
            if plot.is_empty() {
                return (
                    ChartGeometry {
                        bounds,
                        plot,
                        ..ChartGeometry::default()
                    },
                    Some(outer * 2.0 + reserved),
                );
            }
            let max_abs = self
                .data
                .iter()
                .map(|d| d.value.abs())
                .filter(|v| v.is_finite())
                .fold(0.0_f64, f64::max);
            let (min_value, max_value) = if max_abs == 0.0 {
                (0.0, 0.0)
            } else {
                self.data.iter().filter(|d| d.value.is_finite()).fold(
                    (0.0_f64, 0.0_f64),
                    |(min, max), d| {
                        let value = d.value / max_abs;
                        (min.min(value), max.max(value))
                    },
                )
            };
            let span = (max_value - min_value).max(f64::EPSILON);
            let baseline =
                (plot.min_y() as f64 + (max_value / span) * plot.size.height as f64) as f32;
            let (bar_width, slot_width) = self.label_widths(bounds).expect("nonempty chart plot");
            let category_y = plot.max_y() + gap;
            let bars: Vec<BarGeometry> = self
                .data
                .iter()
                .enumerate()
                .filter(|(_, d)| d.value.is_finite())
                .map(|(index, datum)| {
                    let normalized = if max_abs == 0.0 {
                        0.0
                    } else {
                        datum.value / max_abs
                    };
                    let value_y =
                        (baseline as f64 - normalized / span * plot.size.height as f64) as f32;
                    let x =
                        plot.min_x() + slot_width * index as f32 + (slot_width - bar_width) * 0.5;
                    let value_label_y = if value_y < baseline {
                        value_y - value_height
                    } else {
                        value_y
                    };
                    BarGeometry {
                        index,
                        label: datum.label.clone(),
                        value: datum.value,
                        rect: Rect::new(
                            x,
                            value_y.min(baseline),
                            bar_width,
                            (baseline - value_y).abs(),
                        ),
                        value_label: Rect::new(x, value_label_y, bar_width, value_height),
                        category_label: Rect::new(
                            plot.min_x() + slot_width * index as f32,
                            category_y,
                            slot_width,
                            category_height,
                        ),
                        selected: self.selected == Some(index),
                        hovered: self.hovered == Some(index),
                        focused: self.focused && self.selected == Some(index),
                        focus_visible: self.focus_visible && self.selected == Some(index),
                    }
                })
                .collect();
            if bottom == 0.0
                && bars
                    .iter()
                    .any(|bar| bar.value_label.max_y() > plot.max_y())
            {
                bottom = value_height;
                continue;
            }
            return (
                finalize_category_row(
                    ChartGeometry {
                        bounds,
                        plot,
                        baseline,
                        bars,
                        invalid_count: self.data.iter().filter(|d| !d.value.is_finite()).count(),
                    },
                    gap,
                ),
                Some(outer * 2.0 + reserved),
            );
        }
        unreachable!("one bottom-row promotion is sufficient")
    }

    fn layout_zero_values(
        &self,
        bounds: Rect,
        plot: Rect,
        _inset: Rect,
        label_height: f32,
        label_gap: f32,
    ) -> ChartGeometry {
        let (bar_width, slot_width) = self.label_widths(bounds).expect("nonempty chart plot");
        let value_label_height = label_height * 0.5;
        let category_label_height = label_height - value_label_height;
        let bars = self
            .data
            .iter()
            .enumerate()
            .filter(|(_, datum)| datum.value.is_finite())
            .map(|(index, datum)| {
                let x = plot.min_x() + slot_width * index as f32 + (slot_width - bar_width) * 0.5;
                BarGeometry {
                    index,
                    label: datum.label.clone(),
                    value: datum.value,
                    rect: Rect::new(x, plot.center().y, bar_width, 0.0),
                    value_label: Rect::new(x, plot.center().y, bar_width, value_label_height),
                    category_label: Rect::new(
                        plot.min_x() + slot_width * index as f32,
                        plot.max_y() + label_gap,
                        slot_width,
                        category_label_height,
                    ),
                    selected: self.selected == Some(index),
                    hovered: self.hovered == Some(index),
                    focused: self.focused && self.selected == Some(index),
                    focus_visible: self.focus_visible && self.selected == Some(index),
                }
            })
            .collect();
        finalize_category_row(
            ChartGeometry {
                bounds,
                plot,
                baseline: plot.center().y,
                bars,
                invalid_count: self
                    .data
                    .iter()
                    .filter(|datum| !datum.value.is_finite())
                    .count(),
            },
            label_gap,
        )
    }

    fn is_valid(&self, index: usize) -> bool {
        self.data
            .get(index)
            .is_some_and(|datum| datum.value.is_finite())
    }

    fn next_valid(&self, start: usize, direction: isize, wrap: bool) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }
        let len = self.data.len();
        let mut index = start.min(len - 1);
        for _ in 0..len {
            if self.is_valid(index) {
                return Some(index);
            }
            if wrap {
                index = if direction < 0 {
                    (index + len - 1) % len
                } else {
                    (index + 1) % len
                };
            } else if direction < 0 {
                if index == 0 {
                    return None;
                }
                index -= 1;
            } else {
                if index + 1 >= len {
                    return None;
                }
                index += 1;
            }
        }
        None
    }

    fn set_from_key(&mut self, next: Option<usize>) -> bool {
        if next.is_some() {
            self.focused = true;
            self.focus_visible = true;
        }
        let changed = next.is_some() && next != self.selected;
        if changed {
            self.selected = next;
        }
        changed
    }
}

/// Per-point geometry and semantic state. Text is shaped by the consumer inside
/// `value_label` and `category_label` so this crate stays renderer/platform neutral.
#[derive(Clone, Debug, PartialEq)]
pub struct BarGeometry {
    pub index: usize,
    pub label: String,
    pub value: f64,
    pub rect: Rect,
    pub value_label: Rect,
    pub category_label: Rect,
    pub selected: bool,
    pub hovered: bool,
    pub focused: bool,
    pub focus_visible: bool,
}

/// Layout produced from real chart data. `bars` omits non-finite values, while
/// retaining original stable indices for selection and accessibility.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartGeometry {
    pub bounds: Rect,
    pub plot: Rect,
    pub baseline: f32,
    pub bars: Vec<BarGeometry>,
    pub invalid_count: usize,
}

/// Draw the bar series and its zero baseline using JSON Design System semantic
/// tokens. Consumers draw shaped category and value labels using returned layout.
pub fn paint_bar_chart(
    scene: &mut Scene,
    bounds: Rect,
    theme: impl Into<Palette>,
    chart: &BarChart,
) -> ChartGeometry {
    paint_bar_chart_with_text_scale(scene, bounds, theme, chart, 1.0)
}

/// Draw the bar series using the active gallery/application text scale when
/// reserving category and value label space. Returned bar and label geometry is
/// calculated with the same scale used for painting.
pub fn paint_bar_chart_with_text_scale(
    scene: &mut Scene,
    bounds: Rect,
    theme: impl Into<Palette>,
    chart: &BarChart,
    text_scale: f32,
) -> ChartGeometry {
    let theme = theme.into();
    let geometry = chart.layout_with_text_scale(bounds, text_scale);
    paint_chart_geometry(scene, theme, chart, geometry)
}

/// Paint chart bars from text-shaper-measured label geometry.
pub fn paint_bar_chart_with_label_metrics(
    scene: &mut Scene,
    bounds: Rect,
    theme: impl Into<Palette>,
    chart: &BarChart,
    value_line_height: f32,
    category_line_height: f32,
) -> ChartGeometry {
    let geometry = chart.layout_with_label_metrics(bounds, value_line_height, category_line_height);
    paint_chart_geometry(scene, theme.into(), chart, geometry)
}

fn paint_chart_geometry(
    scene: &mut Scene,
    theme: Palette,
    chart: &BarChart,
    geometry: ChartGeometry,
) -> ChartGeometry {
    if geometry.plot.is_empty() || geometry.bars.is_empty() {
        return geometry;
    }
    let inks = ChartInk {
        baseline: theme.color("separator").expect("chart baseline semantic token"),
        series: theme.color("c-chart-series-1").expect("chart series semantic token"),
        selection: theme.color("row-selection").expect("chart selection semantic token"),
        focus: theme.color("border-focused").expect("chart focus semantic token"),
    };
    paint_chart_geometry_with_inks(scene, chart, geometry, true, inks)
}

#[derive(Clone, Copy)]
struct ChartInk {
    baseline: tekes_ui_tokens::Rgba,
    series: tekes_ui_tokens::Rgba,
    selection: tekes_ui_tokens::Rgba,
    focus: tekes_ui_tokens::Rgba,
}

pub(crate) fn paint_chart_geometry_on_surface(
    scene: &mut Scene,
    chart: &BarChart,
    geometry: ChartGeometry,
    colors: tekes_ui_tokens::surface_palette::DataPlotSurfaceColors,
) -> ChartGeometry {
    paint_chart_geometry_with_inks(
        scene,
        chart,
        geometry,
        false,
        ChartInk {
            baseline: colors.plot.boundary.expect("validated data boundary"),
            series: colors.plot.marker.expect("validated data marker"),
            selection: colors.selection.background,
            focus: colors.plot.focus.expect("validated owner focus"),
        },
    )
}

fn paint_chart_geometry_with_inks(
    scene: &mut Scene,
    chart: &BarChart,
    geometry: ChartGeometry,
    bar_focus: bool,
    inks: ChartInk,
) -> ChartGeometry {
    if geometry.plot.is_empty() || geometry.bars.is_empty() {
        return geometry;
    }
    let border_width = proposal::metric("stroke", "stroke-rule").expect("chart stroke token");
    for bar in &geometry.bars {
        let slot = Rect::new(
            bar.category_label.min_x(),
            geometry.plot.min_y(),
            bar.category_label.size.width,
            geometry.plot.size.height,
        );
        if bar.selected {
            scene.push_quad(Quad::new(slot, inks.selection));
        }
        if !bar.rect.is_empty() {
            let mut quad = Quad::new(bar.rect, inks.series);
            if bar.focus_visible {
                if bar_focus {
                    quad = quad.with_border(border_width.max(1.0), inks.focus);
                }
            } else if bar.hovered || (bar.focused && chart.is_focused()) {
                quad = quad.with_border(border_width, inks.baseline);
            }
            scene.push_quad(quad);
        }
    }
    let baseline = Rect::new(
        geometry.plot.min_x(),
        geometry.baseline - border_width * 0.5,
        geometry.plot.size.width,
        border_width,
    );
    scene.push_quad(Quad::new(baseline, inks.baseline));
    geometry
}

// The original geometry owner places categories after actual lower value ink slots.
// Raw layout already reserved V below categories; measured layout reserves each side.
fn finalize_category_row(mut geometry: ChartGeometry, gap: f32) -> ChartGeometry {
    let lower_edge = geometry
        .bars
        .iter()
        .fold(geometry.plot.max_y(), |edge, bar| {
            edge.max(bar.value_label.max_y())
        });
    for bar in &mut geometry.bars {
        bar.category_label.origin.y = lower_edge + gap;
    }
    geometry
}

fn finite_bounds(bounds: Rect) -> Option<Rect> {
    let values = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ];
    if !values.iter().all(|value| value.is_finite())
        || bounds.size.width <= 0.0
        || bounds.size.height <= 0.0
    {
        return None;
    }
    Some(bounds)
}

fn token_metric(family: &str, name: &str) -> f32 {
    proposal::metric(family, name).expect("chart layout semantic token")
}

fn sane_line_height(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        16.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::{Theme, proposal};

    fn chart(values: &[f64]) -> BarChart {
        BarChart::new(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| ChartDatum::new(format!("item {index}"), *value))
                .collect(),
        )
    }

    #[test]
    fn layout_uses_every_data_value_and_preserves_category_indices() {
        let model = chart(&[2.0, 8.0, 4.0, 6.0]);
        let geometry = model.layout(Rect::new(0.0, 0.0, 400.0, 240.0));
        assert_eq!(geometry.bars.len(), 4);
        assert!(geometry.bars[1].rect.size.height > geometry.bars[0].rect.size.height);
        assert!(geometry.bars[1].rect.size.height > geometry.bars[2].rect.size.height);
        assert_eq!(
            geometry
                .bars
                .iter()
                .map(|bar| bar.index)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert!(
            geometry
                .bars
                .iter()
                .all(|bar| bar.category_label.size.width > 0.0)
        );
    }

    #[test]
    fn negative_values_extend_below_zero_baseline() {
        let geometry = chart(&[-4.0, 8.0]).layout(Rect::new(0.0, 0.0, 240.0, 180.0));
        assert_eq!(geometry.bars.len(), 2);
        let negative = &geometry.bars[0];
        let positive = &geometry.bars[1];
        assert!(negative.rect.min_y() >= geometry.baseline);
        assert!(positive.rect.max_y() <= geometry.baseline + f32::EPSILON);
        assert!(negative.rect.size.height > 0.0);
        assert!(positive.rect.size.height > 0.0);
    }

    #[test]
    fn non_finite_values_are_counted_omitted_and_unselectable() {
        let mut model = chart(&[1.0, f64::NAN, f64::INFINITY, -2.0]);
        let geometry = model.layout(Rect::new(0.0, 0.0, 320.0, 200.0));
        assert_eq!(geometry.invalid_count, 2);
        assert_eq!(
            geometry
                .bars
                .iter()
                .map(|bar| bar.index)
                .collect::<Vec<_>>(),
            [0, 3]
        );
        assert_eq!(model.selected(), Some(0));
        assert!(!model.select(1));
        assert!(model.key(ControlKey::Right));
        assert_eq!(model.selected(), Some(3));
    }

    #[test]
    fn all_invalid_and_empty_inputs_are_safe() {
        let mut model = chart(&[f64::NAN, f64::NEG_INFINITY]);
        assert_eq!(model.selected(), None);
        assert!(!model.key(ControlKey::Right));
        let geometry = model.layout(Rect::new(0.0, 0.0, 200.0, 120.0));
        assert_eq!(geometry.invalid_count, 2);
        assert!(geometry.bars.is_empty());
        assert!(
            chart(&[])
                .layout(Rect::new(0.0, 0.0, 200.0, 120.0))
                .bars
                .is_empty()
        );
    }

    #[test]
    fn keyboard_selection_skips_invalid_values_and_supports_boundaries() {
        let mut model = chart(&[1.0, f64::NAN, 2.0, 3.0]);
        assert!(model.key(ControlKey::Right));
        assert_eq!(model.selected(), Some(2));
        assert!(model.is_focused());
        assert!(model.focus_visible());
        assert!(model.key(ControlKey::End));
        assert_eq!(model.selected(), Some(3));
        assert!(model.key(ControlKey::Home));
        assert_eq!(model.selected(), Some(0));
        assert!(model.key(ControlKey::Up));
        assert_eq!(model.selected(), Some(2));
        assert!(model.key(ControlKey::Down));
        assert_eq!(model.selected(), Some(0));
        assert!(!model.key(ControlKey::Escape));
    }

    #[test]
    fn extreme_finite_values_do_not_overflow_geometry() {
        let geometry = chart(&[f64::MAX, -f64::MAX]).layout(Rect::new(0.0, 0.0, 300.0, 200.0));
        assert_eq!(geometry.bars.len(), 2);
        assert!(geometry.baseline.is_finite());
        assert!(geometry.bars.iter().all(|bar| {
            bar.rect.origin.x.is_finite()
                && bar.rect.origin.y.is_finite()
                && bar.rect.size.width.is_finite()
                && bar.rect.size.height.is_finite()
        }));
    }

    #[test]
    fn painting_uses_chart_semantic_tokens_and_draws_dynamic_bars() {
        for theme in Theme::ALL {
            let model = chart(&[1.0, 3.0, -2.0]);
            let mut scene = Scene::new();
            let geometry =
                paint_bar_chart(&mut scene, Rect::new(0.0, 0.0, 320.0, 180.0), theme, &model);
            assert_eq!(geometry.bars.len(), 3);
            let series = proposal::render_color("c-chart-series-1", theme).unwrap();
            let quads = scene
                .primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Quad(quad) => Some(quad),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(quads.iter().filter(|quad| quad.fill == series).count(), 3);
            assert!(
                quads
                    .iter()
                    .any(|quad| quad.fill == proposal::render_color("separator", theme).unwrap())
            );
        }
    }

    #[test]
    fn scaled_paint_returns_the_geometry_used_for_its_scene_and_labels() {
        let model = chart(&[2.0, 8.0, 4.0]);
        let bounds = Rect::new(12.0, 18.0, 360.0, 260.0);
        let mut default_scene = Scene::new();
        let default_geometry = paint_bar_chart(&mut default_scene, bounds, Theme::LIGHT, &model);
        assert_eq!(default_geometry, model.layout(bounds));

        let mut scaled_scene = Scene::new();
        let scaled_geometry =
            paint_bar_chart_with_text_scale(&mut scaled_scene, bounds, Theme::LIGHT, &model, 1.5);
        assert!(scaled_geometry.plot.size.height < default_geometry.plot.size.height);
        assert!(
            scaled_geometry.bars[0].category_label.size.height
                > default_geometry.bars[0].category_label.size.height
        );
        assert!(
            scaled_geometry.bars[0].value_label.size.height
                > default_geometry.bars[0].value_label.size.height
        );

        let series = proposal::render_color("c-chart-series-1", Theme::LIGHT).unwrap();
        let painted_bars = scaled_scene
            .primitives()
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Quad(quad) if quad.fill == series => Some(quad.rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            painted_bars,
            scaled_geometry
                .bars
                .iter()
                .map(|bar| bar.rect)
                .collect::<Vec<_>>()
        );
        assert!(scaled_geometry.bars.iter().all(|bar| {
            bar.category_label.min_y() >= bounds.min_y()
                && bar.category_label.max_y() <= bounds.max_y()
                && bar.value_label.min_y() >= bounds.min_y()
                && bar.value_label.max_y() <= bounds.max_y()
        }));
    }

    #[test]
    fn selected_and_focused_state_is_reported_per_bar() {
        let mut model = chart(&[2.0, 4.0]);
        model.select(1);
        model.set_focused(true, true);
        model.set_hovered(Some(1));
        let geometry = model.layout(Rect::new(0.0, 0.0, 240.0, 160.0));
        assert!(!geometry.bars[0].selected);
        assert!(geometry.bars[1].selected);
        assert!(geometry.bars[1].hovered);
        assert!(geometry.bars[1].focused);
        assert!(geometry.bars[1].focus_visible);
    }

    #[test]
    fn invalid_or_degenerate_bounds_never_emit_geometry() {
        let model = chart(&[1.0, 2.0]);
        assert!(
            model
                .layout(Rect::new(0.0, 0.0, f32::NAN, 100.0))
                .bars
                .is_empty()
        );
        assert!(
            model
                .layout(Rect::new(0.0, 0.0, 0.0, 100.0))
                .bars
                .is_empty()
        );
        assert!(
            model
                .layout(Rect::new(0.0, 0.0, 100.0, f32::INFINITY))
                .bars
                .is_empty()
        );
    }

    #[test]
    fn bounds_narrower_than_token_insets_produce_no_paintable_geometry() {
        let model = chart(&[1.0, 2.0, -1.0]);
        let mut scene = Scene::new();
        let geometry = paint_bar_chart(
            &mut scene,
            Rect::new(0.0, 0.0, 12.0, 120.0),
            Theme::LIGHT,
            &model,
        );
        assert!(geometry.plot.is_empty());
        assert!(geometry.bars.is_empty());
        assert!(scene.is_empty());

        let geometry = model.layout(Rect::new(0.0, 0.0, 120.0, 12.0));
        assert!(geometry.plot.is_empty());
        assert!(geometry.bars.is_empty());
    }

    #[test]
    fn scaled_label_regions_stay_inside_viewport_or_collapse_plot_safely() {
        let model = chart(&[2.0, 8.0, -3.0]);
        let bounds = Rect::new(10.0, 20.0, 320.0, 240.0);
        let geometry = model.layout_with_text_scale(bounds, 8.0);
        assert!(!geometry.plot.is_empty());
        for bar in &geometry.bars {
            for label in [bar.value_label, bar.category_label] {
                assert!(label.min_x() >= bounds.min_x());
                assert!(label.max_x() <= bounds.max_x());
                assert!(label.min_y() >= bounds.min_y());
                assert!(label.max_y() <= bounds.max_y());
            }
        }

        let tiny = Rect::new(0.0, 0.0, 80.0, 32.0);
        let collapsed = model.layout_with_text_scale(tiny, f32::MAX);
        assert!(collapsed.plot.is_empty());
        assert!(collapsed.bars.is_empty());
    }

    #[test]
    fn measured_label_rows_keep_positive_negative_and_category_text_in_viewport() {
        let model = BarChart::new(vec![
            ChartDatum::new("short", 9.0),
            ChartDatum::new("中日混排 category", -8.0),
            ChartDatum::new("long category label", 2.0),
        ]);
        for (bounds, value_height, category_height) in [
            (Rect::new(13.0, 17.0, 420.0, 260.0), 16.0, 16.0),
            (Rect::new(13.0, 17.0, 320.0, 240.0), 24.0, 30.0),
            (Rect::new(0.0, 0.0, 150.0, 180.0), 32.0, 48.0),
        ] {
            let geometry = model.layout_with_label_metrics(bounds, value_height, category_height);
            assert!(!geometry.plot.is_empty());
            for bar in &geometry.bars {
                for label in [bar.value_label, bar.category_label] {
                    assert!(label.min_x() >= bounds.min_x());
                    assert!(label.max_x() <= bounds.max_x());
                    assert!(label.min_y() >= bounds.min_y());
                    assert!(label.max_y() <= bounds.max_y());
                }
                assert_eq!(bar.value_label.size.height, value_height);
                assert_eq!(bar.category_label.size.height, category_height);
                if bar.value > 0.0 {
                    assert!((bar.value_label.max_y() - bar.rect.min_y()).abs() < 0.001);
                } else if bar.value < 0.0 {
                    assert!((bar.value_label.min_y() - bar.rect.max_y()).abs() < 0.001);
                }
            }
            assert!(geometry.plot.min_y() >= bounds.min_y() + value_height);
            assert!(geometry.plot.max_y() <= bounds.max_y() - category_height);
        }
    }
    #[test]
    fn finite_update_is_atomic_and_keeps_legal_category_labels() {
        let mut chart = BarChart::new(vec![
            ChartDatum::new("", 1.0),
            ChartDatum::new("same", 2.0),
            ChartDatum::new("same", 3.0),
        ]);
        chart.select(1);
        chart.set_hovered(Some(2));
        assert_eq!(
            chart.try_update_data(vec![ChartDatum::new("", 4.0), ChartDatum::new("", 5.0)]),
            Ok(())
        );
        assert_eq!(chart.selected(), Some(1));
        assert_eq!(chart.hovered(), None);
        let before = chart.clone();
        assert_eq!(
            chart.try_update_data(vec![
                ChartDatum::new("x", 8.0),
                ChartDatum::new("bad", f64::INFINITY)
            ]),
            Err(ChartDataUpdateError { index: 1 })
        );
        assert_eq!(chart, before);
        assert_eq!(chart.try_update_data(Vec::new()), Ok(()));
        assert_eq!(chart.selected(), None);
    }
}
