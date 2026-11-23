//! Presentation runtimes for the existing collection and document models.
//!
//! These views own component geometry, semantic painting and transient input
//! state. Hosts provide the existing text shaper and forward translated events.
#![allow(clippy::too_many_arguments)]
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey,
    ControlState, Insets,
    chart::{BarChart, ChartGeometry},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

pub(super) fn valid_rect(rect: Rect) -> bool {
    [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ]
    .into_iter()
    .all(f32::is_finite)
        && rect.max_x().is_finite()
        && rect.max_y().is_finite()
        && rect.size.width >= 0.0
        && rect.size.height >= 0.0
}
pub(super) fn visible(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
    let clip = clip?;
    if !valid_rect(frame) || !valid_rect(clip) {
        return None;
    }
    let x0 = frame.min_x().max(clip.min_x());
    let y0 = frame.min_y().max(clip.min_y());
    let x1 = frame.max_x().min(clip.max_x());
    let y1 = frame.max_y().min(clip.max_y());
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}
pub(super) fn own_clip(frame: Rect, ancestor: Option<Rect>) -> Option<Rect> {
    if !valid_rect(frame) || frame.size.width <= 0.0 || frame.size.height <= 0.0 {
        return None;
    }
    if let Some(ancestor) = ancestor
        && (!valid_rect(ancestor) || ancestor.size.width <= 0.0 || ancestor.size.height <= 0.0)
    {
        return None;
    }
    match ancestor {
        Some(ancestor) => visible(frame, Some(ancestor)),
        None => Some(frame),
    }
}
// A finite mounted frame can be outside its ancestor viewport. Retain its
// logical row frames for keyboard reveal without treating a missing paint clip
// as permission to draw. Zero/invalid constraints represent unavailable layout.
pub(super) fn mounted_bounds(frame: Rect, ancestor: Option<Rect>) -> bool {
    let positive = |rect: Rect| valid_rect(rect) && rect.size.width > 0. && rect.size.height > 0.;
    positive(frame) && ancestor.is_none_or(positive)
}
pub(super) fn render_quiet_button(
    button: &mut ButtonModel,
    scene: &mut Scene,
    text: &mut impl ButtonTextRenderer,
    label: &str,
    frame: Rect,
    clip: Option<Rect>,
    palette: Palette,
) -> crate::ButtonLayout {
    button.render(
        scene,
        text,
        label,
        frame,
        clip,
        palette,
        ButtonVariant::Quiet,
        ButtonSize::Body,
        proposal::TextRole::UiBody,
        ButtonTextAlign::Leading,
    )
}
pub(super) fn row_height() -> f32 {
    proposal::metric("size", "size-row-height").expect("list row height")
}
pub(super) fn compact_row_height() -> f32 {
    proposal::metric("size", "size-compact-row").expect("compact row height")
}
pub(super) fn row_height_for(compact: bool) -> f32 {
    if compact {
        compact_row_height()
    } else {
        row_height()
    }
}
pub(super) fn row_gap() -> f32 {
    tekes_ui_tokens::Spacing::Tight.value()
}
pub(super) fn control_state(
    selected: bool,
    hovered: bool,
    focused: bool,
    disabled: bool,
    pressed: bool,
) -> ControlState {
    ControlState {
        selected,
        hovered,
        focused,
        // The collection paints keyboard focus after all sibling chrome.
        focus_visible: false,
        disabled,
        pressed,
        ..Default::default()
    }
}

/// One item's complete frame and clipped hit/paint area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollectionItemLayout {
    pub id: usize,
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub depth: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub items: Vec<CollectionItemLayout>,
    pub content_height: f32,
}

/// Result of a row pointer or keyboard action. The model remains authoritative.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollectionEvent {
    #[default]
    Ignored,
    Handled,
    Changed,
    Activated(usize),
    Toggled(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct CollectionInteraction {
    hovered: Option<usize>,
    focused: Option<usize>,
    focus_visible: bool,
    captured: Option<usize>,
    window_active: bool,
}
impl CollectionInteraction {
    fn cancel_capture(&mut self) -> bool {
        self.captured.take().is_some()
    }
    fn focus(&mut self, id: usize, visible: bool) {
        self.focused = Some(id);
        self.focus_visible = visible;
    }
    fn blur(&mut self) {
        self.focused = None;
        self.focus_visible = false;
        self.captured = None;
        self.hovered = None;
    }
    fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.blur();
        }
    }
    fn retain_ids(&mut self, ids: &[usize]) {
        if self.focused.is_some_and(|id| !ids.contains(&id)) {
            self.focused = None;
            self.focus_visible = false;
        }
        if self.captured.is_some_and(|id| !ids.contains(&id)) {
            self.captured = None;
        }
        if self.hovered.is_some_and(|id| !ids.contains(&id)) {
            self.hovered = None;
        }
    }
}

// Keep logical row hit/content clipping separate from keyboard focus ink.
// Complete rows may expose their canonical outset beyond the control frame;
// real ancestors still clip it. Overflow-cropped rows retain the viewport clip.
pub(super) fn paint_collection_focus(
    scene: &mut Scene,
    item: &CollectionItemLayout,
    frame: Rect,
    ancestor_clip: Option<Rect>,
    palette: Palette,
) {
    if item.visible.is_none() {
        return;
    }
    let container = ContainerLayout::new(frame, Insets::ZERO, Insets::ZERO, ancestor_clip);
    let fits = item.frame.min_x() >= frame.min_x()
        && item.frame.min_y() >= frame.min_y()
        && item.frame.max_x() <= frame.max_x()
        && item.frame.max_y() <= frame.max_y();
    let clip = if fits {
        container.background_paint_clip()
    } else {
        container.visible_clip()
    };
    if let Some(clip) = clip {
        scene.push_clip(clip);
    }
    crate::paint::focus_ring(scene, item.frame, palette, ButtonSize::Body.radius());
    if clip.is_some() {
        scene.pop_clip();
    }
}

pub(super) fn row_key_target(key: ControlKey, current: usize, ids: &[usize]) -> Option<usize> {
    let position = ids.iter().position(|id| *id == current)?;
    match key {
        ControlKey::Home => ids.first().copied(),
        ControlKey::End => ids.last().copied(),
        ControlKey::Up => ids.get(position.saturating_sub(1)).copied(),
        ControlKey::Down => ids.get((position + 1).min(ids.len() - 1)).copied(),
        _ => None,
    }
}

pub(crate) enum ListInput {
    Down(Point),
    Up(Point),
    Key(ControlKey, bool),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CollectionAction {
    pub id: usize,
    pub header: bool,
    pub keyboard: bool,
    pub capture: bool,
    pub select: bool,
    pub sort: bool,
    pub next_sort: Option<(usize, crate::extended::SortDirection)>,
    pub event: CollectionEvent,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ListActionSnapshot {
    layout: CollectionLayout,
    state: CollectionInteraction,
    disabled: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TableActionSnapshot {
    layout: DataTableLayout,
    state: CollectionInteraction,
    disabled: bool,
    header: Option<usize>,
}

mod chart_and_docs;
mod chart_owner;
mod list;
mod table;
mod tree;

pub use chart_and_docs::{
    ChartView, ChartViewLayout, CodeBlockView, DiffViewRuntime, DocumentLayout, MarkdownView,
    PassiveCollectionView,
};
pub use chart_owner::{ChartOwnerCapacity, ChartOwnerFocusSpace, ChartOwnerMeasurement};
pub use list::ListView;
pub use table::{DataTableLayout, DataTableView};
pub(crate) use table::{TableContentMetrics, TableInput};
pub use tree::TreeView;

#[cfg(test)]
mod tests_1;
#[cfg(test)]
mod tests_2;
