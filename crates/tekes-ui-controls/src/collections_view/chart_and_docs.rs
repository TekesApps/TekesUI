#![allow(clippy::too_many_arguments)]
use super::{
    CollectionEvent, CollectionItemLayout,
    chart_owner::{self, ChartOwnerFocusSpace},
    mounted_bounds, own_clip, valid_rect, visible,
};
use crate::{
    ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey, Insets,
    chart::{BarChart, ChartGeometry, paint_bar_chart_with_label_metrics},
    extended::{CodeBlock, DiffKind, DiffView, Markdown, MarkdownBlock},
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartViewLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub geometry: ChartGeometry,
}
#[derive(Clone, Debug)]
pub struct ChartView {
    pub(super) layout: ChartViewLayout,
    pub(super) owner_band: Option<chart_owner::OwnerBand>,
    pub(super) hovered: Option<usize>,
    pub(super) focused: bool,
    pub(super) focus_visible: bool,
    pub(super) window_active: bool,
}
impl Default for ChartView {
    fn default() -> Self {
        Self {
            layout: ChartViewLayout::default(),
            owner_band: None,
            hovered: None,
            focused: false,
            focus_visible: false,
            window_active: true,
        }
    }
}
impl ChartView {
    pub fn new() -> Self {
        Self {
            window_active: true,
            ..Default::default()
        }
    }
    pub fn focus(&mut self, keyboard: bool, model: &mut BarChart) {
        if !self.window_active {
            return;
        }
        self.focused = true;
        self.focus_visible = keyboard;
        model.set_focused(true, keyboard);
    }
    pub fn blur(&mut self, model: &mut BarChart) {
        self.focused = false;
        self.focus_visible = false;
        model.set_focused(false, false);
    }
    pub fn set_window_active(&mut self, active: bool, model: &mut BarChart) {
        self.window_active = active;
        if !active {
            self.clear_hover(model);
            self.blur(model);
        }
    }
    pub fn layout(&self) -> &ChartViewLayout {
        &self.layout
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut BarChart,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        text_scale: f32,
    ) -> ChartViewLayout {
        self.owner_band = None;
        let own_clip = own_clip(frame, clip);
        if own_clip.is_none() && !mounted_bounds(frame, clip) {
            self.focused = false;
            self.focus_visible = false;
            self.hovered = None;
            model.set_focused(false, false);
            model.set_hovered(None);
            self.layout = ChartViewLayout {
                frame,
                visible: None,
                geometry: ChartGeometry::default(),
            };
            return self.layout.clone();
        }
        if own_clip.is_none() {
            self.hovered = None;
        }
        model.set_focused(self.focused && self.window_active, self.focus_visible);
        model.set_hovered(self.hovered);
        let mut local = Scene::new();
        let _ = text_scale; // Label heights come from the active text shaper.
        let mut value_line_height: f32 = 16.0;
        let mut category_line_height: f32 = 16.0;
        if let Some((value_width, category_width)) = model.label_widths(frame) {
            for datum in model.data().iter().filter(|datum| datum.value.is_finite()) {
                value_line_height = value_line_height.max(
                    text.measure(
                        &datum.value.to_string(),
                        proposal::TextRole::UiCaption,
                        value_width,
                    )
                    .height,
                );
                category_line_height = category_line_height.max(
                    text.measure(&datum.label, proposal::TextRole::UiCaption, category_width)
                        .height,
                );
            }
        }
        let geometry = if own_clip.is_some() {
            paint_bar_chart_with_label_metrics(
                &mut local,
                frame,
                palette,
                model,
                value_line_height,
                category_line_height,
            )
        } else {
            model.layout_with_label_metrics(frame, value_line_height, category_line_height)
        };
        if let Some(clip) = own_clip {
            scene.push_clip(clip);
            for primitive in local.primitives() {
                scene.push(primitive.clone());
            }
            draw_chart_labels(scene, text, &geometry, palette);
            scene.pop_clip();
        }
        self.layout = ChartViewLayout {
            frame,
            visible: own_clip,
            geometry,
        };
        self.layout.clone()
    }
    /// Retire the original View's hover together with its model on cancellation.
    pub(crate) fn clear_hover(&mut self, model: &mut BarChart) -> bool {
        let changed = self.hovered.take().is_some() || model.hovered().is_some();
        model.set_hovered(None);
        changed
    }
    pub(crate) fn pointer_move_in_region(
        &mut self,
        point: Point,
        model: &mut BarChart,
        region_contains: bool,
    ) -> bool {
        if region_contains {
            return self.pointer_move(point, model);
        }
        if !self.owner_data_current(model) {
            return false;
        }
        self.clear_hover(model)
    }
    pub fn pointer_move(&mut self, point: Point, model: &mut BarChart) -> bool {
        if !self.owner_data_current(model) {
            return false;
        }
        if !self.window_active || self.layout.visible.is_none() {
            let changed = self.hovered.take().is_some();
            model.set_hovered(None);
            return changed;
        }
        let next = self
            .layout
            .geometry
            .bars
            .iter()
            .find(|b| {
                visible(b.rect.inset(-6.0), self.layout.visible).is_some_and(|r| r.contains(point))
            })
            .map(|b| b.index);
        let changed = next != self.hovered;
        self.hovered = next;
        model.set_hovered(next);
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &mut BarChart) -> CollectionEvent {
        if !self.owner_data_current(model) {
            return CollectionEvent::Ignored;
        }
        if self.owner_band.is_some()
            && !self.focused
            && !matches!(
                self.owner_focus_space(model),
                ChartOwnerFocusSpace::Visible { .. }
            )
        {
            return CollectionEvent::Ignored;
        }
        if !self.window_active || self.layout.visible.is_none_or(|r| !r.contains(point)) {
            return CollectionEvent::Ignored;
        }
        let Some(index) = self
            .layout
            .geometry
            .bars
            .iter()
            .find(|b| {
                visible(b.rect.inset(-6.0), self.layout.visible).is_some_and(|r| r.contains(point))
            })
            .map(|b| b.index)
        else {
            return CollectionEvent::Ignored;
        };
        self.focus(false, model);
        if model.select(index) {
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut BarChart) -> CollectionEvent {
        if !down || !self.focused || !self.window_active {
            return CollectionEvent::Ignored;
        }
        self.focus_visible = true;
        if model.key(key) {
            model.set_focused(true, true);
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
}

pub(super) fn chart_label_metrics(
    text: &mut (impl ButtonTextRenderer + ?Sized),
    model: &BarChart,
    frame: Rect,
) -> (f32, f32, bool) {
    let mut measured = true;
    let mut value_line_height: f32 = 16.0;
    let mut category_line_height: f32 = 16.0;
    if let Some((value_width, category_width)) = model.label_widths(frame) {
        for datum in model.data().iter().filter(|datum| datum.value.is_finite()) {
            let value_size = text.measure(
                &datum.value.to_string(),
                proposal::TextRole::UiCaption,
                value_width,
            );
            let category_size =
                text.measure(&datum.label, proposal::TextRole::UiCaption, category_width);
            measured &= [
                value_size.width,
                value_size.height,
                category_size.width,
                category_size.height,
            ]
            .into_iter()
            .all(|extent| extent.is_finite() && extent >= 0.0);
            value_line_height = value_line_height.max(value_size.height);
            category_line_height = category_line_height.max(category_size.height);
        }
    }
    let widths_available = model.data().is_empty() || model.label_widths(frame).is_some();
    (
        value_line_height,
        category_line_height,
        measured && widths_available,
    )
}

fn draw_chart_labels(
    scene: &mut Scene,
    text: &mut impl ButtonTextRenderer,
    geometry: &ChartGeometry,
    palette: Palette,
) {
    let plot =
        tekes_ui_tokens::surface_palette::resolve_surface(palette, "canvas", "data-plot", "rest")
            .expect("chart plot Surface packet");
    let category_packet = tekes_ui_tokens::surface_palette::resolve_surface(
        palette,
        "canvas",
        "data-category-label",
        "rest",
    )
    .expect("chart category Surface packet");
    draw_chart_labels_with_inks(scene, text, geometry, |category| {
        if category {
            category_packet.text
        } else {
            plot.text
        }
    });
}

pub(super) fn draw_chart_labels_with_inks(
    scene: &mut Scene,
    text: &mut (impl ButtonTextRenderer + ?Sized),
    geometry: &ChartGeometry,
    mut ink: impl FnMut(bool) -> tekes_ui_tokens::Rgba,
) {
    let text_alignment = tekes_ui_tokens::proposal::layout::CHART_LABEL.text_alignment;
    for bar in &geometry.bars {
        let value = bar.value.to_string();
        let (value_rect, value_align) =
            chart_text_frame(text, &value, bar.value_label, text_alignment);
        text.draw(
            scene,
            &value,
            proposal::TextRole::UiCaption,
            value_rect,
            value_align,
            ink(false),
        );
        let (category_rect, category_align) =
            chart_text_frame(text, &bar.label, bar.category_label, text_alignment);
        text.draw(
            scene,
            &bar.label,
            proposal::TextRole::UiCaption,
            category_rect,
            category_align,
            ink(true),
        );
    }
}

pub(super) fn chart_text_frame(
    text: &mut (impl ButtonTextRenderer + ?Sized),
    value: &str,
    slot: Rect,
    alignment: tekes_ui_tokens::proposal::layout::TextAlignment,
) -> (Rect, ButtonTextAlign) {
    use tekes_ui_tokens::proposal::layout::TextAlignment;
    match alignment {
        TextAlignment::Center => (slot, ButtonTextAlign::Center),
        TextAlignment::Leading => (slot, ButtonTextAlign::Leading),
        TextAlignment::Trailing => {
            let max_width = if slot.size.width.is_finite() {
                slot.size.width.max(0.0)
            } else {
                0.0
            };
            let measured = text
                .measure(value, proposal::TextRole::UiCaption, max_width)
                .width;
            let width = if measured.is_finite() {
                measured.clamp(0.0, max_width)
            } else {
                0.0
            };
            (
                Rect::new(slot.max_x() - width, slot.min_y(), width, slot.size.height),
                ButtonTextAlign::Leading,
            )
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub blocks: Vec<CollectionItemLayout>,
    pub content_height: f32,
}

/// Layout and paint runtime for the repository's intentionally bounded Markdown subset.
#[derive(Clone, Debug, Default)]
pub struct MarkdownView {
    pub(super) layout: DocumentLayout,
    pub(super) top_inset: f32,
}
impl MarkdownView {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_top_inset(mut self, top_inset: f32) -> Self {
        self.top_inset = top_inset;
        self
    }
    pub fn set_top_inset(&mut self, top_inset: f32) {
        self.top_inset = top_inset;
    }
    pub fn layout(&self) -> &DocumentLayout {
        &self.layout
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Markdown,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        text_scale: f32,
    ) -> DocumentLayout {
        let scale = if text_scale.is_finite() && text_scale > 0. {
            text_scale
        } else {
            1.
        };
        // Keep the historical 12pt paragraph rhythm at the default scale;
        // shaped multiline height remains content-driven.
        let gap = proposal::metric("spacing", "space-relaxed").unwrap_or(12.);
        let width = frame.size.width.max(0.);
        let own_clip = own_clip(frame, clip);
        let top_inset = if self.top_inset.is_finite() {
            self.top_inset.max(0.)
        } else {
            0.
        };
        let mut y = frame.min_y() + top_inset;
        let mut blocks = Vec::with_capacity(model.blocks.len());
        let min_row = proposal::metric("size", "size-compact-row").expect("markdown row floor");
        let code_pad = tekes_ui_tokens::Spacing::Relaxed.value();
        for (index, block) in model.blocks.iter().enumerate() {
            let is_code = matches!(block, MarkdownBlock::Code { .. });
            let (value, role, indent, min_h) = match block {
                MarkdownBlock::Heading { level: _, text } => (
                    text.as_str(),
                    proposal::TextRole::UiHeading,
                    0.,
                    proposal::TextRole::UiHeading.style().line_height,
                ),
                MarkdownBlock::Paragraph(value) => (
                    value.as_str(),
                    proposal::TextRole::UiBody,
                    0.,
                    proposal::TextRole::UiBody.style().line_height,
                ),
                MarkdownBlock::Bullet(value) => (
                    value.as_str(),
                    proposal::TextRole::UiBody,
                    0.,
                    proposal::TextRole::UiBody.style().line_height,
                ),
                MarkdownBlock::Code { source, .. } => (
                    source.as_str(),
                    proposal::TextRole::Code,
                    0.,
                    proposal::TextRole::Code.style().line_height,
                ),
            };
            let max_width = (width - indent).max(0.);
            let pad = if is_code { code_pad } else { 0. };
            let text_width = (max_width - pad * 2.).max(0.);
            let rendered = if matches!(block, MarkdownBlock::Bullet(_)) {
                format!("• {value}")
            } else {
                value.to_owned()
            };
            let measured = text.measure(&rendered, role, text_width);
            // `measure` reflects the active host scale; only the token floor is
            // scaled here so the gallery does not apply its text scale twice.
            let h = (measured.height.max(min_h * scale) + pad * 2.).max(min_row);
            let rect = Rect::new(frame.min_x() + indent, y, max_width, h);
            let draw_rect = Rect::new(
                rect.min_x() + pad,
                rect.min_y() + pad,
                text_width,
                measured.height,
            );
            let v = visible(rect, own_clip);
            if let Some(v) = v {
                if is_code {
                    let container =
                        ContainerLayout::new(rect, Insets::ZERO, Insets::ZERO, own_clip);
                    if let Some(bg_clip) = container.background_paint_clip() {
                        scene.push_clip(bg_clip);
                        crate::paint::surface_kind_at_scale(
                            scene,
                            rect,
                            palette,
                            crate::paint::SurfaceKind::Code,
                            text.backing_scale(),
                        );
                        scene.pop_clip();
                    }
                }
                scene.push_clip(v);
                let ink = palette
                    .color("primary-text")
                    .expect("DS primary text token");
                text.draw(
                    scene,
                    &rendered,
                    role,
                    draw_rect,
                    ButtonTextAlign::Leading,
                    ink,
                );
                scene.pop_clip();
            }
            blocks.push(CollectionItemLayout {
                id: index,
                frame: rect,
                visible: v,
                depth: 0,
            });
            y += h + gap;
        }
        self.layout = DocumentLayout {
            frame,
            visible: own_clip,
            blocks,
            content_height: (y - frame.min_y() - if model.blocks.is_empty() { 0. } else { gap })
                .max(0.),
        };
        self.layout.clone()
    }
}

/// Paint runtime for plain source text. It does not claim syntax highlighting or editing.
#[derive(Clone, Debug, Default)]
pub struct CodeBlockView {
    layout: DocumentLayout,
    max_width: Option<f32>,
}
impl CodeBlockView {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_width(mut self, max_width: Option<f32>) -> Self {
        self.max_width = max_width;
        self
    }
    pub fn set_max_width(&mut self, max_width: Option<f32>) {
        self.max_width = max_width;
    }
    pub fn layout(&self) -> &DocumentLayout {
        &self.layout
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &CodeBlock,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        text_scale: f32,
    ) -> DocumentLayout {
        if !valid_rect(frame) || clip.is_some_and(|r| !valid_rect(r)) {
            self.layout = DocumentLayout {
                frame,
                ..Default::default()
            };
            return self.layout.clone();
        }
        let container = ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, clip);
        let own_clip = container.visible_clip();
        if let Some(own) = own_clip {
            if let Some(background_clip) = container.background_paint_clip() {
                scene.push_clip(background_clip);
            }
            crate::paint::surface_kind_at_scale(
                scene,
                frame,
                palette,
                crate::paint::SurfaceKind::Code,
                text.backing_scale(),
            );
            if container.background_paint_clip().is_some() {
                scene.pop_clip();
            }
            scene.push_clip(own);
        }
        let scale = if text_scale.is_finite() && text_scale > 0. {
            text_scale
        } else {
            1.
        };
        let role = proposal::TextRole::Code;
        let lh = role.style().line_height * scale;
        // The original Gallery code specimen uses a 14pt text inset from its
        // surface edge. Keep that visual contract while the view owns drawing.
        let inset = proposal::metric("spacing", "space-relaxed").unwrap_or(12.);
        let mut blocks = Vec::new();
        let mut y = frame.min_y() + inset;
        let x = frame.min_x() + inset;
        let raw_w = (frame.size.width - inset * 2.).max(0.);
        let w = self
            .max_width
            .filter(|limit| limit.is_finite())
            .map_or(raw_w, |limit| raw_w.min(limit.max(0.)));
        for (index, source) in model.lines().enumerate() {
            let measured = text.measure(source, role, w);
            let h = measured.height.max(lh);
            let rect = Rect::new(x, y, w, h);
            let v = visible(rect, own_clip);
            if let Some(v) = v {
                scene.push_clip(v);
                text.draw(
                    scene,
                    source,
                    role,
                    rect,
                    ButtonTextAlign::Leading,
                    palette
                        .color("primary-text")
                        .expect("DS primary text token"),
                );
                scene.pop_clip();
            }
            blocks.push(CollectionItemLayout {
                id: index,
                frame: rect,
                visible: v,
                depth: 0,
            });
            y += h;
        }
        if own_clip.is_some() {
            scene.pop_clip();
        }
        self.layout = DocumentLayout {
            frame,
            visible: own_clip,
            blocks,
            content_height: y - frame.min_y() + inset,
        };
        self.layout.clone()
    }
}

/// Paint runtime for the existing line-oriented LCS diff model.
#[derive(Clone, Debug, Default)]
pub struct DiffViewRuntime {
    pub(super) layout: DocumentLayout,
    pub(super) max_width: Option<f32>,
}
impl DiffViewRuntime {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_width(mut self, max_width: Option<f32>) -> Self {
        self.max_width = max_width;
        self
    }
    pub fn set_max_width(&mut self, max_width: Option<f32>) {
        self.max_width = max_width;
    }
    pub fn layout(&self) -> &DocumentLayout {
        &self.layout
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &DiffView,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        text_scale: f32,
    ) -> DocumentLayout {
        if !valid_rect(frame) || clip.is_some_and(|r| !valid_rect(r)) {
            self.layout = DocumentLayout {
                frame,
                ..Default::default()
            };
            return self.layout.clone();
        }
        let own_clip = own_clip(frame, clip);
        if let Some(own) = own_clip {
            scene.push_clip(own);
        }
        let scale = if text_scale.is_finite() && text_scale > 0. {
            text_scale
        } else {
            1.
        };
        let role = proposal::TextRole::Code;
        let line_floor =
            proposal::metric("size", "size-compact-row").expect("diff row height") * scale;
        let mut blocks = Vec::with_capacity(model.lines.len());
        let inset = tekes_ui_tokens::Spacing::Regular.value();
        let line_gap = tekes_ui_tokens::Spacing::Micro.value() * scale;
        let mut y = frame.min_y();
        for (index, line) in model.lines.iter().enumerate() {
            let (prefix, color) = match line.kind {
                DiffKind::Equal => ("  ", "primary-text"),
                DiffKind::Added => ("+ ", "status-succeeded"),
                DiffKind::Removed => ("− ", "status-failed"),
            };
            let rendered = format!("{prefix}{}", line.text);
            let raw_w = (frame.size.width - inset * 2.).max(0.);
            let content_width = self
                .max_width
                .filter(|limit| limit.is_finite())
                .map_or(raw_w, |limit| raw_w.min(limit.max(0.)));
            let measured = text.measure(&rendered, role, content_width);
            let h = measured.height.max(line_floor);
            let rect = Rect::new(frame.min_x() + inset, y, content_width, h);
            let v = visible(rect, own_clip);
            if let Some(v) = v {
                scene.push_clip(v);
                text.draw(
                    scene,
                    &rendered,
                    role,
                    rect,
                    ButtonTextAlign::Leading,
                    palette
                        .color(color)
                        .or_else(|| palette.color("primary-text"))
                        .expect("DS text token"),
                );
                scene.pop_clip();
            }
            blocks.push(CollectionItemLayout {
                id: index,
                frame: rect,
                visible: v,
                depth: 0,
            });
            y += h + line_gap;
        }
        if own_clip.is_some() {
            scene.pop_clip();
        }
        self.layout = DocumentLayout {
            frame,
            visible: own_clip,
            blocks,
            content_height: (y
                - frame.min_y()
                - if model.lines.is_empty() { 0. } else { line_gap })
            .max(0.),
        };
        self.layout.clone()
    }
}

/// Read-only list/table/chart/document views do not own keyboard editing state.
pub trait PassiveCollectionView {
    fn cancel(&mut self);
}
impl PassiveCollectionView for MarkdownView {
    fn cancel(&mut self) {}
}
impl PassiveCollectionView for CodeBlockView {
    fn cancel(&mut self) {}
}
impl PassiveCollectionView for DiffViewRuntime {
    fn cancel(&mut self) {}
}
