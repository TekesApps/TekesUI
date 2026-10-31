//! Presentation runtimes for the existing collection and document models.
//!
//! These views own component geometry, semantic painting and transient input
//! state. Hosts provide the existing text shaper and forward translated events.
#![allow(clippy::too_many_arguments)]
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ContainerLayout, ControlKey,
    ControlState, Insets,
    chart::{BarChart, ChartGeometry, paint_bar_chart_with_label_metrics},
    extended::{CodeBlock, DataTable, DiffKind, DiffView, List, Markdown, MarkdownBlock, Tree},
    layout_view::{IconRenderer, IconView},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

fn valid_rect(rect: Rect) -> bool {
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
fn visible(frame: Rect, clip: Option<Rect>) -> Option<Rect> {
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
fn own_clip(frame: Rect, ancestor: Option<Rect>) -> Option<Rect> {
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
fn mounted_bounds(frame: Rect, ancestor: Option<Rect>) -> bool {
    let positive = |rect: Rect| valid_rect(rect) && rect.size.width > 0. && rect.size.height > 0.;
    positive(frame) && ancestor.is_none_or(positive)
}
fn render_quiet_button(
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
fn row_height() -> f32 {
    proposal::metric("spacing", "space-loose")
        .unwrap_or(24.0)
        .max(24.0)
        + 12.0
}
fn row_gap() -> f32 {
    // Existing Gallery list/tree/table fixtures use 4pt row gaps. Preserve
    // those default pitches while allowing wrapped content to grow naturally.
    4.0
}
fn control_state(
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
struct CollectionInteraction {
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
fn paint_collection_focus(
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

fn row_key_target(key: ControlKey, current: usize, ids: &[usize]) -> Option<usize> {
    let position = ids.iter().position(|id| *id == current)?;
    match key {
        ControlKey::Home => ids.first().copied(),
        ControlKey::End => ids.last().copied(),
        ControlKey::Up => ids.get(position.saturating_sub(1)).copied(),
        ControlKey::Down => ids.get((position + 1).min(ids.len() - 1)).copied(),
        _ => None,
    }
}

/// Runtime for the existing single-selection [`List`] model.
/// Runtime for the existing single-selection [`List`] model.
#[derive(Clone, Debug)]
struct ListViewIdentity {
    instance: std::sync::Arc<()>,
    revision: u64,
    arranged: bool,
    items: Vec<(crate::extended::ListItemKey, u64)>,
}
#[derive(Clone, Copy, Debug)]
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
/// Runtime for the existing single-selection [`List`] model.
#[derive(Clone, Debug)]
pub struct ListView {
    layout: CollectionLayout,
    state: CollectionInteraction,
    disabled: bool,
    buttons: Vec<ButtonModel>,
    keyed: Option<ListViewIdentity>,
    row_placements: Vec<crate::ButtonLayout>,
    row_custom: Vec<bool>,
    list_ancestor_clip: Option<Rect>,
    unavailable_layout: CollectionLayout,
}
impl Default for ListView {
    fn default() -> Self {
        Self {
            layout: CollectionLayout::default(),
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            disabled: false,
            buttons: Vec::new(),
            keyed: None,
            row_placements: Vec::new(),
            row_custom: Vec::new(),
            list_ancestor_clip: None,
            unavailable_layout: CollectionLayout::default(),
        }
    }
}
impl ListView {
    pub fn new() -> Self {
        Self {
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    /// Intrinsic height using the same default row metrics as painting.
    pub fn measure(
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        width: f32,
    ) -> tekes_ui_scene::Size {
        Self::measure_content_rows(
            text,
            labels,
            &(0..labels.len()).collect::<Vec<_>>(),
            width,
            &[],
        )
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.state.blur();
        }
    }
    pub fn focus(&mut self, index: usize, keyboard: bool) {
        if self.keyed.is_some() || self.disabled || !self.state.window_active {
            return;
        }
        self.state.focus(index, keyboard);
    }
    pub fn blur(&mut self) {
        self.state.blur();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.state.set_window_active(active);
    }
    pub fn layout(&self) -> &CollectionLayout {
        if self.keyed.is_some() {
            &self.unavailable_layout
        } else {
            &self.layout
        }
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &List,
        labels: &[String],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> CollectionLayout {
        self.leave_keyed();
        let ids = (0..model.count.min(labels.len())).collect::<Vec<_>>();
        self.render_rows(scene, text, model, labels, &ids, frame, clip, palette, 1.0)
    }
    pub fn render_scaled(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &List,
        labels: &[String],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        scale: f32,
    ) -> CollectionLayout {
        self.leave_keyed();
        let ids = (0..model.count.min(labels.len())).collect::<Vec<_>>();
        self.render_rows(
            scene, text, model, labels, &ids, frame, clip, palette, scale,
        )
    }
    fn render_rows(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &List,
        labels: &[String],
        ids: &[usize],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        scale: f32,
    ) -> CollectionLayout {
        self.arrange_rows(text, labels, ids, frame, clip, scale, &[]);
        self.paint_rows(scene, text, model, labels, palette, None, frame, clip);
        self.layout.clone()
    }
    fn row_size(
        text: &mut impl ButtonTextRenderer,
        label: &str,
        custom: Option<tekes_ui_scene::Size>,
        width: f32,
    ) -> tekes_ui_scene::Size {
        match custom {
            Some(size) => ButtonModel::measure_content(
                text,
                size,
                width,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
            ),
            None => ButtonModel::measure(
                text,
                label,
                width,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
            ),
        }
    }
    pub(crate) fn content_width(text: &mut impl ButtonTextRenderer, width: f32) -> f32 {
        crate::ButtonLayout::resolve_with_text(
            text,
            Rect::new(0., 0., width, 0.),
            None,
            tekes_ui_scene::Size::default(),
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Leading,
            crate::ButtonFramePolicy::Minimum,
        )
        .content
        .size
        .width
    }
    pub(crate) fn measure_content_rows(
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        ids: &[usize],
        width: f32,
        sizes: &[Option<tekes_ui_scene::Size>],
    ) -> tekes_ui_scene::Size {
        let h = ids
            .iter()
            .map(|i| {
                Self::row_size(text, &labels[*i], sizes.get(*i).copied().flatten(), width)
                    .height
                    .max(row_height())
            })
            .sum::<f32>()
            + row_gap() * ids.len().saturating_sub(1) as f32;
        tekes_ui_scene::Size::new(width, h)
    }
    fn arrange_rows(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        ids: &[usize],
        frame: Rect,
        clip: Option<Rect>,
        scale: f32,
        sizes: &[Option<tekes_ui_scene::Size>],
    ) {
        self.list_ancestor_clip = clip;
        self.state.retain_ids(ids);
        self.row_placements.clear();
        self.row_custom.clear();
        let rh = row_height()
            * if scale.is_finite() && scale > 0. {
                scale
            } else {
                1.
            };
        let heights = ids
            .iter()
            .map(|i| {
                Self::row_size(
                    text,
                    &labels[*i],
                    sizes.get(*i).copied().flatten(),
                    frame.size.width,
                )
                .height
                .max(rh)
            })
            .collect::<Vec<_>>();
        self.layout = CollectionLayout {
            frame,
            visible: own_clip(frame, clip),
            content_height: heights.iter().sum::<f32>()
                + row_gap() * ids.len().saturating_sub(1) as f32,
            items: Vec::with_capacity(ids.len()),
        };
        if !mounted_bounds(frame, clip) {
            self.state.blur();
            return;
        }
        if self.layout.visible.is_none() {
            self.state.captured = None;
            self.state.hovered = None;
        }
        self.buttons.resize_with(ids.len(), ButtonModel::default);
        self.buttons.truncate(ids.len());
        let width = Self::content_width(text, frame.size.width);
        let mut y = frame.min_y();
        for (ordinal, &id) in ids.iter().enumerate() {
            let custom = sizes.get(id).copied().flatten();
            let extent = custom.unwrap_or_else(|| {
                if labels[id].is_empty() {
                    tekes_ui_scene::Size::default()
                } else {
                    text.measure(&labels[id], proposal::TextRole::UiBody, width)
                }
            });
            let r = Rect::new(frame.min_x(), y, frame.size.width, heights[ordinal]);
            let mut layout = crate::ButtonLayout::resolve_with_text(
                text,
                r,
                self.layout.visible,
                extent,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Leading,
                crate::ButtonFramePolicy::Minimum,
            );
            // None is absence of viewport authority, not an unbounded clip.
            layout.visible = visible(layout.frame, self.layout.visible);
            self.layout.items.push(CollectionItemLayout {
                id,
                frame: r,
                visible: layout.visible,
                depth: 0,
            });
            self.row_placements.push(layout);
            self.row_custom.push(custom.is_some());
            y += heights[ordinal] + row_gap();
        }
    }
    fn paint_rows(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &List,
        labels: &[String],
        palette: Palette,
        surface: Option<&'static str>,
        frame: Rect,
        clip: Option<Rect>,
    ) {
        let Some(own) = self.layout.visible else {
            return;
        };
        scene.push_clip(own);
        for (ordinal, row) in self.layout.items.iter().enumerate() {
            let button = &mut self.buttons[ordinal];
            button.state = control_state(
                model.selected == Some(row.id),
                self.state.hovered == Some(row.id),
                self.state.focused == Some(row.id) && self.state.window_active,
                self.disabled,
                self.state.captured == Some(row.id),
            );
            if let Some(surface) = surface {
                button.set_surface(surface);
            } else {
                button.clear_surface();
            }
            button.paint_prepared_segment(
                scene,
                text,
                if self.row_custom[ordinal] {
                    ""
                } else {
                    &labels[row.id]
                },
                self.row_placements[ordinal],
                Some(own),
                palette,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
            );
        }
        scene.pop_clip();
        if !self.disabled
            && self.state.window_active
            && self.state.focus_visible
            && let Some(row) = self
                .layout
                .items
                .iter()
                .find(|i| Some(i.id) == self.state.focused)
        {
            paint_collection_focus(scene, row, frame, clip, palette);
        }
    }
    pub(crate) fn invalidate_list_layout(&mut self, model: &crate::extended::KeyedList) {
        self.guard_list(model);
        self.cancel();
        self.layout = CollectionLayout::default();
        self.row_placements.clear();
        if let Some(k) = &mut self.keyed {
            k.arranged = false;
        }
    }
    pub(crate) fn arrange_content_keyed(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedList,
        frame: Rect,
        clip: Option<Rect>,
        sizes: &[Option<tekes_ui_scene::Size>],
    ) {
        self.guard_list(model);
        let labels = model
            .snapshot()
            .items
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>();
        self.arrange_rows(
            text,
            &labels,
            model.display_indices(),
            frame,
            clip,
            1.,
            sizes,
        );
        self.keyed.as_mut().expect("guarded list").arranged = true;
    }
    pub(crate) fn paint_content_keyed(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedList,
        palette: Palette,
        surface: Option<&'static str>,
    ) {
        if !self.list_current(model) {
            return;
        }
        let labels = model
            .snapshot()
            .items
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>();
        self.paint_rows(
            scene,
            text,
            model.original(),
            &labels,
            palette,
            surface,
            self.layout.frame,
            self.list_ancestor_clip,
        );
    }
    pub(crate) fn list_content_frame(&self, source: usize) -> Option<Rect> {
        self.layout
            .items
            .iter()
            .position(|r| r.id == source)
            .and_then(|j| self.row_placements.get(j))
            .map(|p| p.content)
    }
    pub(crate) fn list_row_ink(
        &self,
        source: usize,
        palette: Palette,
        surface: Option<&str>,
    ) -> Option<(tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba)> {
        let j = self.layout.items.iter().position(|r| r.id == source)?;
        let b = self.buttons.get(j)?;
        let c = b.surface_colors(palette, ButtonVariant::Quiet, surface.unwrap_or("canvas"));
        Some((c.text, c.icon))
    }
    pub fn pointer_move(&mut self, point: Point) -> bool {
        if self.keyed.is_some() {
            return false;
        }
        self.pointer_move_impl(point)
    }
    fn pointer_move_impl(&mut self, point: Point) -> bool {
        if self.disabled || !self.state.window_active {
            let changed = self.state.hovered.take().is_some();
            return changed;
        }
        let next = self
            .layout
            .items
            .iter()
            .find(|i| i.visible.is_some_and(|r| r.contains(point)))
            .map(|i| i.id);
        let changed = self.state.hovered != next;
        self.state.hovered = next;
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &mut List) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.pointer_down_impl(point, model)
    }
    fn decide_list_down(&self, point: Point, model: &List) -> Option<CollectionAction> {
        if self.disabled || !self.state.window_active {
            return None;
        }
        let id = self
            .layout
            .items
            .iter()
            .find(|r| r.visible.is_some_and(|r| r.contains(point)))?
            .id;
        if id >= model.count {
            return None;
        }
        Some(CollectionAction {
            id,
            header: false,
            keyboard: false,
            capture: true,
            select: true,
            sort: false,
            next_sort: None,
            event: if model.selected != Some(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Handled
            },
        })
    }
    fn pointer_down_impl(&mut self, point: Point, model: &mut List) -> CollectionEvent {
        let Some(p) = self.decide_list_down(point, model) else {
            return CollectionEvent::Ignored;
        };
        let event = p.event;
        self.apply_list_action(p, model);
        event
    }
    pub(crate) fn list_action_snapshot(&self) -> ListActionSnapshot {
        ListActionSnapshot {
            layout: self.layout.clone(),
            state: self.state.clone(),
            disabled: self.disabled,
        }
    }
    pub(crate) fn prepare_list_down(
        &self,
        point: Point,
        model: &crate::extended::KeyedList,
    ) -> Option<CollectionAction> {
        if !self.list_current(model) {
            return None;
        }
        self.decide_list_down(point, model.original())
    }
    pub(crate) fn prepare_list_part(
        &self,
        key: crate::extended::ListItemKey,
        press: bool,
        model: &crate::extended::KeyedList,
    ) -> Option<CollectionAction> {
        if !self.list_current(model) || self.disabled || !self.state.window_active {
            return None;
        }
        let id = model
            .item_index(key)
            .filter(|id| self.layout.items.iter().any(|r| r.id == *id))?;
        Some(CollectionAction {
            id,
            header: false,
            keyboard: true,
            capture: false,
            select: press,
            sort: false,
            next_sort: None,
            event: if press {
                CollectionEvent::Activated(id)
            } else {
                CollectionEvent::Changed
            },
        })
    }
    pub(crate) fn apply_list_action(&mut self, p: CollectionAction, model: &mut List) {
        self.state.focus(p.id, p.keyboard);
        if p.capture {
            self.state.captured = Some(p.id);
        }
        if p.select {
            model.selected = Some(p.id);
        }
    }
    pub fn pointer_up(&mut self, point: Point, model: &mut List) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.pointer_up_impl(point, model)
    }
    fn pointer_up_impl(&mut self, point: Point, model: &mut List) -> CollectionEvent {
        let Some(id) = self.state.captured.take() else {
            return CollectionEvent::Ignored;
        };
        if !self.state.window_active || self.disabled || id >= model.count {
            return CollectionEvent::Ignored;
        }
        if self
            .layout
            .items
            .iter()
            .any(|i| i.id == id && i.visible.is_some_and(|r| r.contains(point)))
        {
            if model.select(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Handled
            }
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut List) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.key_impl(key, down, model)
    }
    fn key_impl(&mut self, key: ControlKey, down: bool, model: &mut List) -> CollectionEvent {
        if !down || self.disabled || !self.state.window_active {
            return CollectionEvent::Ignored;
        }
        // Cancellation belongs to the held input, even if its source became
        // unavailable. An unowned Escape remains available to the parent.
        if key == ControlKey::Escape {
            return if self.state.cancel_capture() {
                CollectionEvent::Handled
            } else {
                CollectionEvent::Ignored
            };
        }
        let Some(id) = self.state.focused else {
            return CollectionEvent::Ignored;
        };
        if !self.layout.items.iter().any(|item| item.id == id) || id >= model.count {
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
        if key == ControlKey::Enter {
            model.select(id);
            return CollectionEvent::Activated(id);
        }
        if !matches!(
            key,
            ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End
        ) {
            return CollectionEvent::Handled;
        }
        // Ancestor clipping limits pointer hits, not logical keyboard targets.
        // Hosts use the focused row frame to reveal content after navigation.
        let ids = self
            .layout
            .items
            .iter()
            .filter(|item| item.id < model.count)
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let target = row_key_target(key, id, &ids).unwrap_or(id);
        self.state.focus(target, true);
        if model.select(target) {
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn cancel(&mut self) {
        self.state.cancel_capture();
    }
    fn list_current(&self, model: &crate::extended::KeyedList) -> bool {
        self.keyed.as_ref().is_some_and(|k| {
            k.arranged
                && std::sync::Arc::ptr_eq(&k.instance, &model.view_identity())
                && k.revision == model.revision()
        })
    }
    fn leave_keyed(&mut self) {
        if self.keyed.take().is_some() {
            self.state.blur();
            self.layout = CollectionLayout::default();
            self.buttons.clear();
        }
    }
    fn guard_list(&mut self, model: &crate::extended::KeyedList) {
        let instance = model.view_identity();
        if self.keyed.as_ref().is_some_and(|k| {
            std::sync::Arc::ptr_eq(&k.instance, &instance) && k.revision == model.revision()
        }) {
            return;
        }
        let items = model
            .snapshot()
            .items
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.key,
                    model.item_incarnation(i).expect("validated list item"),
                )
            })
            .collect();
        self.accept_list_identity(instance, model.revision(), items, model.display_indices());
    }
    fn accept_list_identity(
        &mut self,
        instance: std::sync::Arc<()>,
        revision: u64,
        items: Vec<(crate::extended::ListItemKey, u64)>,
        displayed: &[usize],
    ) {
        let focused = self
            .keyed
            .as_ref()
            .filter(|k| std::sync::Arc::ptr_eq(&k.instance, &instance))
            .and_then(|k| self.state.focused.and_then(|i| k.items.get(i)))
            .and_then(|old| items.iter().position(|new| new == old))
            .filter(|i| displayed.contains(i));
        self.state.focused = focused;
        if focused.is_none() {
            self.state.focus_visible = false;
        }
        self.state.captured = None;
        self.state.hovered = None;
        self.layout = CollectionLayout::default();
        self.row_placements.clear();
        for b in &mut self.buttons {
            b.clear_layout();
        }
        self.keyed = Some(ListViewIdentity {
            instance,
            revision,
            arranged: false,
            items,
        });
    }
    /// Prepare focus mapping/capture cancellation and all identity storage before
    /// the original model commits. No second model or selection authority.
    pub(crate) fn prepare_list_update(
        &self,
        model: &crate::extended::KeyedList,
        stage: &crate::extended::ListStage,
    ) -> Self {
        let mut view = self.clone();
        let items = stage
            .snapshot
            .items
            .iter()
            .zip(&stage.incarnations)
            .map(|(r, n)| (r.key, *n))
            .collect();
        view.accept_list_identity(
            model.view_identity(),
            stage.reconcile.revision,
            items,
            &stage.display_indices,
        );
        view
    }
    /// Pure query: no reconciliation or geometry publication on stale input.
    pub fn layout_keyed(&self, model: &crate::extended::KeyedList) -> Option<&CollectionLayout> {
        self.list_current(model).then_some(&self.layout)
    }
    pub fn focused_index_keyed(&self, model: &crate::extended::KeyedList) -> Option<usize> {
        self.list_current(model)
            .then_some(self.state.focused)
            .flatten()
    }
    pub fn focused_keyed(
        &self,
        model: &crate::extended::KeyedList,
    ) -> Option<crate::extended::ListItemKey> {
        self.focused_index_keyed(model)
            .and_then(|i| model.item_key(i))
    }
    /// The same original metrics/chrome, arranged in logical display order.
    pub fn render_keyed(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedList,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> CollectionLayout {
        self.render_keyed_scaled(scene, text, model, frame, clip, palette, 1.0)
    }
    pub fn render_keyed_scaled(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedList,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        scale: f32,
    ) -> CollectionLayout {
        self.guard_list(model);
        let labels = model
            .snapshot()
            .items
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>();
        let layout = self.render_rows(
            scene,
            text,
            model.original(),
            &labels,
            model.display_indices(),
            frame,
            clip,
            palette,
            scale,
        );
        self.keyed.as_mut().expect("guarded list").arranged = true;
        layout
    }
    pub fn focus_keyed(&mut self, keyboard: bool, model: &crate::extended::KeyedList) -> bool {
        self.guard_list(model);
        let Some(key) = model
            .original()
            .selected
            .or_else(|| model.display_indices().first().copied())
            .and_then(|i| model.item_key(i))
        else {
            return false;
        };
        self.focus_item_keyed(key, keyboard, model)
    }
    pub fn focus_item_keyed(
        &mut self,
        key: crate::extended::ListItemKey,
        keyboard: bool,
        model: &crate::extended::KeyedList,
    ) -> bool {
        self.guard_list(model);
        if !self.list_current(model) || self.disabled || !self.state.window_active {
            return false;
        }
        let Some(i) = model
            .item_index(key)
            .filter(|i| self.layout.items.iter().any(|r| r.id == *i))
        else {
            return false;
        };
        self.state.focus(i, keyboard);
        true
    }
    pub fn pointer_move_keyed(&mut self, point: Point, model: &crate::extended::KeyedList) -> bool {
        self.guard_list(model);
        self.list_current(model) && self.pointer_move_impl(point)
    }
    fn input_keyed(
        &mut self,
        input: ListInput,
        model: &mut crate::extended::KeyedList,
    ) -> CollectionEvent {
        self.guard_list(model);
        if !self.list_current(model) {
            return CollectionEvent::Ignored;
        }
        model.view_input(self, input)
    }
    pub fn pointer_down_keyed(
        &mut self,
        point: Point,
        model: &mut crate::extended::KeyedList,
    ) -> CollectionEvent {
        self.input_keyed(ListInput::Down(point), model)
    }
    pub fn pointer_up_keyed(
        &mut self,
        point: Point,
        model: &mut crate::extended::KeyedList,
    ) -> CollectionEvent {
        self.input_keyed(ListInput::Up(point), model)
    }
    pub fn key_keyed(
        &mut self,
        key: ControlKey,
        down: bool,
        model: &mut crate::extended::KeyedList,
    ) -> CollectionEvent {
        self.input_keyed(ListInput::Key(key, down), model)
    }
    pub(crate) fn reduce_list(&mut self, input: ListInput, model: &mut List) -> CollectionEvent {
        match input {
            ListInput::Down(point) => self.pointer_down_impl(point, model),
            ListInput::Up(point) => self.pointer_up_impl(point, model),
            ListInput::Key(key, down) => self.key_impl(key, down, model),
        }
    }
}

/// Runtime for the existing hierarchical [`Tree`] model.
#[derive(Clone, Debug)]
pub struct TreeView {
    layout: CollectionLayout,
    rows: Vec<TreeContentRow>,
    unavailable_layout: CollectionLayout,
    accepted_identity: Option<std::sync::Arc<()>>,
    accepted_revision: Option<u64>,
    layout_revision: Option<u64>,
    accepted_nodes: Vec<TreeViewIdentity>,
    keyed: bool,
    state: CollectionInteraction,
    disabled: bool,
    buttons: Vec<ButtonModel>,
    icons: Vec<IconView>,
    icon_slots: Vec<TreeIconSlot>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeContentRow {
    pub(crate) id: usize,
    pub(crate) content: Rect,
    icon: Rect,
    size: tekes_ui_scene::Size,
    custom: bool,
}
#[derive(Clone, Debug)]
struct TreeViewIdentity {
    key: Option<crate::extended::TreeNodeKey>,
    incarnation: Option<u64>,
    parent: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
struct TreeIconSlot {
    name: &'static str,
    frame: Rect,
    visible: Option<Rect>,
    tint: tekes_ui_tokens::Rgba,
}
impl Default for TreeView {
    fn default() -> Self {
        Self {
            layout: CollectionLayout::default(),
            rows: Vec::new(),
            unavailable_layout: CollectionLayout::default(),
            accepted_identity: None,
            accepted_revision: None,
            layout_revision: None,
            accepted_nodes: Vec::new(),
            keyed: false,
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            disabled: false,
            buttons: Vec::new(),
            icons: Vec::new(),
            icon_slots: Vec::new(),
        }
    }
}
impl TreeView {
    fn model_current(&self, model: &Tree) -> bool {
        self.accepted_identity
            .as_ref()
            .is_some_and(|id| std::sync::Arc::ptr_eq(id, &model.model_identity()))
            && self.accepted_revision == Some(model.model_revision())
            && model.selection_projection_current()
    }
    fn invalidate_layout(&mut self) {
        self.layout = CollectionLayout::default();
        self.rows.clear();
        self.layout_revision = None;
        self.state.captured = None;
        self.state.hovered = None;
        self.buttons.clear();
        self.icons.clear();
        self.icon_slots.clear();
    }
    /// Automatic identity reconciliation; callers never need to forward deltas.
    pub fn reconcile_model(&mut self, model: &Tree) {
        if self.model_current(model) {
            return;
        }
        let same_model = self
            .accepted_identity
            .as_ref()
            .is_some_and(|id| std::sync::Arc::ptr_eq(id, &model.model_identity()));
        if model.is_keyed() || self.keyed {
            let visible = model.visible();
            let old_focus = self.state.focused;
            let mut focus = None;
            if same_model && model.is_keyed() {
                let map = |old: usize| {
                    let identity = self.accepted_nodes.get(old)?;
                    let i = model.node_index(identity.key?)?;
                    (model.incarnation(i) == identity.incarnation
                        && visible.contains(&i)
                        && !model.nodes()[i].disabled)
                        .then_some(i)
                };
                focus = old_focus.and_then(map);
                if focus.is_none()
                    && let Some(old) = old_focus
                    && let Some(identity) = self.accepted_nodes.get(old)
                {
                    // Disabled current entities clear focus without ancestor recovery.
                    let current = identity.key.and_then(|key| model.node_index(key));
                    let revoked_or_disabled =
                        current.is_some_and(|i| model.nodes()[i].disabled || visible.contains(&i));
                    if !revoked_or_disabled {
                        let mut ancestor = identity.parent;
                        while let Some(i) = ancestor {
                            if let Some(next) = map(i) {
                                focus = Some(next);
                                break;
                            }
                            ancestor = self.accepted_nodes.get(i).and_then(|node| node.parent);
                        }
                    }
                }
            }
            self.state.focused = focus;
            if focus.is_none() {
                self.state.focus_visible = false;
            }
            self.invalidate_layout();
        }
        self.keyed = model.is_keyed();
        self.accepted_identity = Some(model.model_identity());
        self.accepted_revision = Some(model.model_revision());
        self.accepted_nodes = model
            .nodes()
            .iter()
            .enumerate()
            .map(|(i, node)| TreeViewIdentity {
                key: model.node_key(i),
                incarnation: model.incarnation(i),
                parent: node.parent,
            })
            .collect();
    }
    fn input_current(&mut self, model: &mut Tree) -> bool {
        let tampered = model.restore_selection_projection();
        if tampered {
            self.invalidate_layout();
        }
        self.reconcile_model(model);
        !model.is_keyed() || (!tampered && self.layout_revision == Some(model.model_revision()))
    }
    /// Pure query: stale geometry is unavailable and never reconciles state.
    pub fn layout_for(&self, model: &Tree) -> Option<&CollectionLayout> {
        (!(self.keyed || model.is_keyed())
            || (self.model_current(model) && self.layout_revision == Some(model.model_revision())))
        .then_some(&self.layout)
    }
    pub fn focused_index(&self, model: &Tree) -> Option<usize> {
        if self.disabled || !self.state.window_active {
            return None;
        }
        self.layout_for(model)?;
        self.state.focused.filter(|i| {
            model.nodes().get(*i).is_some_and(|n| !n.disabled) && model.visible().contains(i)
        })
    }
    pub fn focus_model(&mut self, index: usize, keyboard: bool, model: &mut Tree) -> bool {
        if !self.input_current(model)
            || self.disabled
            || !self.state.window_active
            || model.nodes().get(index).is_none_or(|n| n.disabled)
            || !model.visible().contains(&index)
        {
            return false;
        }
        self.state.focus(index, keyboard);
        true
    }
    pub fn pointer_move_model(&mut self, point: Point, model: &mut Tree) -> bool {
        if !self.input_current(model) {
            return false;
        }
        self.pointer_move_impl(point)
    }
    pub fn render_icons_model(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        model: &mut Tree,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) {
        if self.input_current(model) {
            self.render_icons_impl(scene, renderer, clip, palette, icon_size);
        }
    }
    /// Mutable paint entry also repairs the compatibility selection projection.
    pub fn render_model(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &mut Tree,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> CollectionLayout {
        if model.restore_selection_projection() {
            self.invalidate_layout();
        }
        self.render(scene, text, model, frame, clip, palette)
    }

    pub fn new() -> Self {
        Self {
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.state.blur();
        }
    }
    pub fn focus(&mut self, index: usize, keyboard: bool) {
        if self.keyed || self.disabled || !self.state.window_active {
            return;
        }
        self.state.focus(index, keyboard);
    }
    pub fn blur(&mut self) {
        self.state.blur();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.state.set_window_active(active);
    }
    pub fn layout(&self) -> &CollectionLayout {
        if self.keyed {
            &self.unavailable_layout
        } else {
            &self.layout
        }
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Tree,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> CollectionLayout {
        let metrics = Self::measure_rows(model, frame.size.width, text, |text, id, width, _| {
            (
                text.measure(&model.nodes()[id].label, proposal::TextRole::UiBody, width),
                false,
            )
        });
        self.arrange_rows(text, model, frame, clip, &metrics);
        self.paint_rows(scene, text, model, clip, palette, "base");
        self.layout.clone()
    }
    /// One row geometry resolver shared by legacy text and mounted Content.
    pub(crate) fn measure_rows(
        model: &Tree,
        width: f32,
        text: &mut dyn ButtonTextRenderer,
        mut measure: impl FnMut(
            &mut dyn ButtonTextRenderer,
            usize,
            f32,
            f32,
        ) -> (tekes_ui_scene::Size, bool),
    ) -> Vec<(usize, tekes_ui_scene::Size, bool)> {
        let slot =
            proposal::metric("size", "size-inline-action").expect("Tree expand action slot token");
        let inset = proposal::metric("spacing", "space-relaxed").unwrap_or(12.);
        model
            .visible()
            .into_iter()
            .map(|id| {
                let row_width = (width - depth_indent(model, id)).max(0.);
                let width = (row_width - inset * 2. - slot).max(0.);
                let resolved = crate::ButtonLayout::resolve_with_text(
                    text,
                    Rect::new(0., 0., row_width, 34.),
                    None,
                    tekes_ui_scene::Size::default(),
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::Leading,
                    crate::ButtonFramePolicy::Minimum,
                );
                let custom_width = (resolved.content.size.width - slot).max(0.);
                let (size, custom) = measure(text, id, width, custom_width);
                (id, size, custom)
            })
            .collect()
    }
    fn resolved_row_height(
        text: &mut dyn ButtonTextRenderer,
        size: tekes_ui_scene::Size,
        custom: bool,
        width: f32,
    ) -> f32 {
        if custom {
            ButtonModel::measure_content(
                text,
                size,
                width,
                ButtonSize::Body,
                proposal::TextRole::UiBody,
            )
            .height
            .max(34.)
        } else {
            size.height.max(34.)
        }
    }
    pub(crate) fn rows_height(
        text: &mut dyn ButtonTextRenderer,
        width: f32,
        metrics: &[(usize, tekes_ui_scene::Size, bool)],
    ) -> f32 {
        metrics
            .iter()
            .map(|(_, size, custom)| Self::resolved_row_height(text, *size, *custom, width))
            .sum::<f32>()
            + row_gap() * metrics.len().saturating_sub(1) as f32
    }
    pub(crate) fn arrange_rows(
        &mut self,
        text: &mut impl ButtonTextRenderer,
        model: &Tree,
        frame: Rect,
        clip: Option<Rect>,
        metrics: &[(usize, tekes_ui_scene::Size, bool)],
    ) {
        self.reconcile_model(model);
        if !model.selection_projection_current() {
            self.invalidate_layout();
            return;
        }
        self.layout_revision = Some(model.model_revision());
        self.rows.clear();
        self.icon_slots.clear();
        let ids = model.visible();
        self.state.retain_ids(&ids);
        let own = own_clip(frame, clip);
        self.layout = CollectionLayout {
            frame,
            visible: own,
            content_height: Self::rows_height(text, frame.size.width, metrics),
            items: Vec::new(),
        };
        if own.is_none() {
            self.state.captured = None;
            self.state.hovered = None;
            for b in &mut self.buttons {
                b.clear_layout();
            }
            if !mounted_bounds(frame, clip) {
                self.state.blur();
                return;
            }
        }
        let slot =
            proposal::metric("size", "size-inline-action").expect("Tree expand action slot token");
        let mut y = frame.min_y();
        for &(id, size, custom) in metrics {
            let indent = depth_indent(model, id);
            let width = (frame.size.width - indent).max(0.);
            let height = Self::resolved_row_height(text, size, custom, width);
            let r = Rect::new(frame.min_x() + indent, y, width, height);
            let button = crate::ButtonLayout::resolve_with_text(
                text,
                r,
                own,
                if custom {
                    size
                } else {
                    tekes_ui_scene::Size::default()
                },
                ButtonSize::Body,
                proposal::TextRole::UiBody,
                ButtonTextAlign::Leading,
                crate::ButtonFramePolicy::Minimum,
            );
            let content = button.content;
            let side = slot.min(content.size.width);
            let text_width = (content.size.width - side).max(0.);
            let label_size = if custom {
                size
            } else {
                text.measure(
                    &model.nodes()[id].label,
                    proposal::TextRole::UiBody,
                    text_width,
                )
            };
            let label_height = label_size.height.min(r.size.height);
            let display = Rect::new(
                content.min_x() + side,
                r.center().y - label_height * 0.5,
                text_width,
                label_height,
            );
            self.layout.items.push(CollectionItemLayout {
                id,
                frame: r,
                visible: if own.is_some() { button.visible } else { None },
                depth: tree_depth(model, id),
            });
            self.rows.push(TreeContentRow {
                id,
                content: display,
                icon: Rect::new(content.min_x(), r.min_y(), side, r.size.height),
                size,
                custom,
            });
            y += height + row_gap();
        }
        self.layout.content_height =
            (y - frame.min_y() - if metrics.is_empty() { 0. } else { row_gap() }).max(0.);
    }
    pub(crate) fn content_rows(&self, model: &Tree) -> Option<&[TreeContentRow]> {
        self.layout_for(model)?;
        Some(&self.rows)
    }
    pub(crate) fn invalidate_content_layout(&mut self) {
        self.invalidate_layout();
    }
    pub(crate) fn paint_rows(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &Tree,
        clip: Option<Rect>,
        palette: Palette,
        surface: &'static str,
    ) {
        if self.layout_for(model).is_none() {
            return;
        }
        self.icon_slots.clear();
        let Some(own) = self.layout.visible else {
            return;
        };
        self.buttons
            .resize_with(model.nodes().len(), ButtonModel::default);
        self.buttons.truncate(model.nodes().len());
        scene.push_clip(own);
        for (item, row) in self.layout.items.iter().zip(&self.rows) {
            let id = item.id;
            let node = &model.nodes()[id];
            let focused = self.state.focused == Some(id) && self.state.window_active;
            let button = &mut self.buttons[id];
            button.state = control_state(
                model.selected == Some(id),
                self.state.hovered == Some(id),
                focused,
                self.disabled || node.disabled,
                self.state.captured == Some(id),
            );
            if surface == "base" {
                button.clear_surface();
            } else {
                button.set_surface(surface);
            }
            if row.custom {
                button.render_content_frame(
                    scene,
                    text,
                    row.size,
                    item.frame,
                    Some(own),
                    palette,
                    ButtonVariant::Quiet,
                );
            } else {
                render_quiet_button(button, scene, text, "", item.frame, Some(own), palette);
            }
            let colors = button.surface_colors(
                palette,
                ButtonVariant::Quiet,
                if surface == "base" { "canvas" } else { surface },
            );
            // The original string View and mounted Content share the Quiet
            // Surface packet, including rest/selected/disabled text and icons.
            let tint = colors.text;
            if !row.custom
                && item.visible.is_some()
                && !node.label.is_empty()
                && !row.content.is_empty()
            {
                text.draw(
                    scene,
                    &node.label,
                    proposal::TextRole::UiBody,
                    row.content,
                    ButtonTextAlign::Leading,
                    tint,
                );
            }
            if model.nodes().iter().any(|n| n.parent == Some(id)) {
                self.icon_slots.push(TreeIconSlot {
                    name: if node.expanded {
                        "tekes-expand"
                    } else {
                        "tekes-forward"
                    },
                    frame: row.icon,
                    visible: item.visible,
                    tint: colors.icon,
                });
            }
        }
        scene.pop_clip();
        if !self.disabled
            && self.state.window_active
            && self.state.focus_visible
            && let Some(item) = self
                .layout
                .items
                .iter()
                .find(|i| Some(i.id) == self.state.focused)
            && model.nodes().get(item.id).is_some_and(|n| !n.disabled)
        {
            paint_collection_focus(scene, item, self.layout.frame, clip, palette);
        }
    }
    /// Draw the branch affordances after releasing the host text-shaper borrow.
    /// Every row reserves the same leading slot, including leaf rows.
    pub fn render_icons(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) {
        if self.keyed {
            return;
        }
        self.render_icons_impl(scene, renderer, clip, palette, icon_size);
    }
    fn render_icons_impl(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
        clip: Option<Rect>,
        palette: Palette,
        icon_size: f32,
    ) {
        self.icons
            .resize_with(self.icon_slots.len(), IconView::default);
        for (view, slot) in self.icons.iter_mut().zip(&self.icon_slots) {
            let effective_clip = match (slot.visible, clip) {
                (Some(visible), Some(clip)) => visible.intersection(&clip),
                (Some(visible), None) => Some(visible),
                (None, _) => None,
            };
            if effective_clip.is_none() {
                continue;
            }
            view.render_tinted(
                scene,
                renderer,
                slot.name,
                slot.frame,
                icon_size,
                effective_clip,
                palette,
                slot.tint,
            );
        }
    }
    pub fn pointer_move(&mut self, point: Point) -> bool {
        if self.keyed {
            return false;
        }
        self.pointer_move_impl(point)
    }
    fn pointer_move_impl(&mut self, point: Point) -> bool {
        if self.disabled || !self.state.window_active {
            let changed = self.state.hovered.take().is_some();
            return changed;
        }
        let next = self
            .layout
            .items
            .iter()
            .find(|i| i.visible.is_some_and(|r| r.contains(point)))
            .map(|i| i.id);
        let changed = self.state.hovered != next;
        self.state.hovered = next;
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &mut Tree) -> CollectionEvent {
        if !self.input_current(model) {
            return CollectionEvent::Ignored;
        }
        if self.disabled || !self.state.window_active {
            return CollectionEvent::Ignored;
        }
        let Some(id) = self
            .layout
            .items
            .iter()
            .find(|i| i.visible.is_some_and(|r| r.contains(point)))
            .map(|i| i.id)
        else {
            return CollectionEvent::Ignored;
        };
        if model.nodes().get(id).is_none_or(|n| n.disabled) || !model.visible().contains(&id) {
            return CollectionEvent::Ignored;
        }
        self.state.focus(id, false);
        self.state.captured = Some(id);
        if model.select(id) {
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn pointer_up(&mut self, point: Point, model: &mut Tree) -> CollectionEvent {
        if !self.input_current(model) {
            return CollectionEvent::Ignored;
        }
        let Some(id) = self.state.captured.take() else {
            return CollectionEvent::Ignored;
        };
        if !self.state.window_active
            || self.disabled
            || model.nodes().get(id).is_none_or(|n| n.disabled)
            || !model.visible().contains(&id)
        {
            return CollectionEvent::Ignored;
        }
        if !self
            .layout
            .items
            .iter()
            .any(|i| i.id == id && i.visible.is_some_and(|r| r.contains(point)))
        {
            return CollectionEvent::Handled;
        }
        if model.nodes().iter().any(|n| n.parent == Some(id)) {
            self.set_expanded(id, !model.nodes()[id].expanded, model)
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut Tree) -> CollectionEvent {
        if !self.input_current(model) {
            return CollectionEvent::Ignored;
        }
        if !down || self.disabled || !self.state.window_active {
            return CollectionEvent::Ignored;
        }
        if key == ControlKey::Escape {
            return if self.state.cancel_capture() {
                CollectionEvent::Handled
            } else {
                CollectionEvent::Ignored
            };
        }
        let Some(id) = self.state.focused else {
            return CollectionEvent::Ignored;
        };
        if model.nodes().get(id).is_none_or(|node| node.disabled)
            || !model.visible().contains(&id)
            || !self.layout.items.iter().any(|item| item.id == id)
        {
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
        if key == ControlKey::Enter {
            model.select(id);
            return CollectionEvent::Activated(id);
        }
        if !matches!(
            key,
            ControlKey::Up
                | ControlKey::Down
                | ControlKey::Home
                | ControlKey::End
                | ControlKey::Left
                | ControlKey::Right
        ) {
            return CollectionEvent::Handled;
        }
        if matches!(
            key,
            ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End
        ) {
            let ids = model
                .visible()
                .into_iter()
                .filter(|id| !model.nodes()[*id].disabled)
                .collect::<Vec<_>>();
            let target = row_key_target(key, id, &ids).unwrap_or(id);
            self.state.focus(target, true);
            return if model.select(target) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Handled
            };
        }
        if (key == ControlKey::Left && model.nodes()[id].expanded)
            || (key == ControlKey::Right && !model.nodes()[id].expanded)
        {
            return self.set_expanded(id, key == ControlKey::Right, model);
        }
        let selected = model.select(id);
        let moved = model.key(key);
        if selected || moved {
            self.state.focus(model.selected.unwrap_or(id), true);
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
    /// Set a mounted branch's disclosure state without reinterpreting Enter.
    /// Preserve selection/focus unless collapse conceals the current owner.
    pub fn set_expanded(
        &mut self,
        index: usize,
        expanded: bool,
        model: &mut Tree,
    ) -> CollectionEvent {
        if !self.input_current(model) {
            return CollectionEvent::Ignored;
        }
        if self.disabled
            || !self.state.window_active
            || model.nodes().get(index).is_none_or(|n| n.disabled)
            || !model.visible().contains(&index)
            || !self.layout.items.iter().any(|item| item.id == index)
            || !model.nodes().iter().any(|n| n.parent == Some(index))
        {
            return CollectionEvent::Ignored;
        }
        if model.nodes()[index].expanded == expanded {
            return CollectionEvent::Handled;
        }
        if !model.toggle(index) {
            return CollectionEvent::Ignored;
        }
        let visible = model.visible();
        if model.selected.is_some_and(|id| !visible.contains(&id)) {
            model.select(index);
        }
        if self.state.focused.is_some_and(|id| !visible.contains(&id)) {
            self.state.focus(index, true);
        }
        self.state.captured = None;
        self.reconcile_model(model);
        CollectionEvent::Toggled(index)
    }
    pub fn cancel(&mut self) {
        self.state.captured = None;
    }
}

fn tree_depth(tree: &Tree, mut id: usize) -> usize {
    let mut depth = 0;
    while let Some(parent) = tree.nodes().get(id).and_then(|n| n.parent) {
        depth += 1;
        id = parent;
        if depth > tree.nodes().len() {
            break;
        }
    }
    depth
}
fn depth_indent(tree: &Tree, id: usize) -> f32 {
    tree_depth(tree, id) as f32 * 20.0
}

/// Data table row input uses original model indices, so sorting never retargets selection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataTableLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub headers: Vec<CollectionItemLayout>,
    pub rows: Vec<CollectionItemLayout>,
    pub columns: Vec<Rect>,
    pub content_height: f32,
}
// Optional passive measurements consumed by the original table layout service.
#[derive(Clone, Debug, Default)]
pub(crate) struct TableContentMetrics {
    pub surface: Option<&'static str>,
    pub headers: Vec<Option<tekes_ui_scene::Size>>,
    pub cells: Vec<Vec<(Option<tekes_ui_scene::Size>, Option<tekes_ui_scene::Size>)>>,
}
// Accepted identity at the last model-aware call; no selection/sort copy.
#[derive(Clone, Debug)]
struct TableViewIdentity {
    instance: std::sync::Arc<()>,
    revision: u64,
    rows: Vec<(crate::extended::TableRowKey, u64)>,
    columns: Vec<(crate::extended::TableColumnKey, u64)>,
    arranged: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum TableInput {
    Down(Point),
    Up(Point),
    Key(ControlKey, bool),
}
#[derive(Clone, Debug)]
pub struct DataTableView {
    layout: DataTableLayout,
    state: CollectionInteraction,
    disabled: bool,
    header_buttons: Vec<ButtonModel>,
    row_buttons: Vec<ButtonModel>,
    focused_header: Option<usize>,
    keyed: Option<TableViewIdentity>,
    unavailable_layout: DataTableLayout,
    content_metrics: Option<TableContentMetrics>,
}
impl Default for DataTableView {
    fn default() -> Self {
        Self {
            layout: DataTableLayout::default(),
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            disabled: false,
            header_buttons: Vec::new(),
            row_buttons: Vec::new(),
            focused_header: None,
            keyed: None,
            unavailable_layout: DataTableLayout::default(),
            content_metrics: None,
        }
    }
}
impl DataTableView {
    pub fn new() -> Self {
        Self {
            state: CollectionInteraction {
                window_active: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.focused_header = None;
            self.state.blur();
        }
    }
    pub fn focus_row(&mut self, row: usize, keyboard: bool) {
        if self.keyed.is_some() {
            return;
        }
        self.focus_row_impl(row, keyboard);
    }
    fn focus_row_impl(&mut self, row: usize, keyboard: bool) {
        if self.disabled || !self.state.window_active {
            return;
        }
        self.focused_header = None;
        self.state.focus(row, keyboard);
    }
    pub fn focus_header(&mut self, column: usize, keyboard: bool) {
        if self.keyed.is_some() {
            return;
        }
        self.focus_header_impl(column, keyboard);
    }
    fn focus_header_impl(&mut self, column: usize, keyboard: bool) {
        if self.disabled || !self.state.window_active {
            return;
        }
        self.focused_header = Some(column);
        self.state.focused = None;
        self.state.focus_visible = keyboard;
    }
    pub fn blur(&mut self) {
        self.focused_header = None;
        self.state.blur();
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.state.set_window_active(active);
        if !active {
            self.focused_header = None;
        }
    }
    pub fn layout(&self) -> &DataTableLayout {
        if self.keyed.is_some() {
            &self.unavailable_layout
        } else {
            &self.layout
        }
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &DataTable,
        headers: &[String],
        rows: &[Vec<String>],
        row_order: &[usize],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> DataTableLayout {
        if self.keyed.take().is_some() {
            self.blur();
            self.layout = DataTableLayout::default();
        }
        self.render_impl(
            scene, text, model, headers, rows, row_order, frame, clip, palette,
        )
    }
    fn render_impl(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &DataTable,
        headers: &[String],
        rows: &[Vec<String>],
        row_order: &[usize],
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> DataTableLayout {
        let width = frame.size.width.max(0.0);
        let columns = model.columns.max(1);
        let cw = width / columns as f32;
        // Preserve the Gallery's existing table contract: 34pt header and
        // rows, first row at 40pt, then 38pt pitch.
        // Header/row frames must contain the same original Button chrome at
        // the renderer's resolved type scale; otherwise visible/frame diverge.
        let minimum =
            ButtonModel::measure(text, "", cw, ButtonSize::Body, proposal::TextRole::UiBody)
                .height
                .max(34.0);
        let header_h = minimum;
        let rh = minimum;
        let gap = row_gap();
        let own_clip = own_clip(frame, clip);
        let valid_order = row_order
            .iter()
            .copied()
            .filter(|id| *id < model.rows && *id < rows.len())
            .collect::<Vec<_>>();
        self.state.retain_ids(&valid_order);
        if self
            .focused_header
            .is_some_and(|column| column >= model.columns)
        {
            self.focused_header = None;
        }
        let header_labels = (0..model.columns)
            .map(|col| {
                let title = headers.get(col).cloned().unwrap_or_default();
                if model.sort.is_some_and(|(sorted, _)| sorted == col) {
                    format!("{title}  ↕")
                } else {
                    title
                }
            })
            .collect::<Vec<_>>();
        let header_heights = header_labels
            .iter()
            .enumerate()
            .map(|(col, title)| {
                if let Some(size) = self
                    .content_metrics
                    .as_ref()
                    .and_then(|m| m.headers.get(col))
                    .copied()
                    .flatten()
                {
                    size.height.max(header_h)
                } else {
                    ButtonModel::measure(
                        text,
                        title,
                        cw,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                    )
                    .height
                    .max(header_h)
                }
            })
            .collect::<Vec<_>>();
        let header_band = header_heights.iter().copied().fold(header_h, f32::max);
        let row_start = header_band + 6.0;
        let row_heights = valid_order
            .iter()
            .map(|id| {
                (0..model.columns)
                    .map(|col| {
                        let (custom, accessory) = self
                            .content_metrics
                            .as_ref()
                            .and_then(|m| m.cells.get(*id))
                            .and_then(|r| r.get(col))
                            .copied()
                            .unwrap_or_default();
                        let aw = accessory.map_or(0., |a| a.width.min((cw - 16.).max(0.)));
                        custom
                            .unwrap_or_else(|| {
                                text.measure(
                                    rows[*id].get(col).map(String::as_str).unwrap_or(""),
                                    proposal::TextRole::UiBody,
                                    (cw - 16. - aw).max(0.),
                                )
                            })
                            .height
                            .max(accessory.map_or(0., |a| a.height))
                    })
                    .fold(rh, f32::max)
            })
            .collect::<Vec<_>>();
        let content_height = if valid_order.is_empty() {
            header_band
        } else {
            row_start
                + row_heights.iter().sum::<f32>()
                + gap * valid_order.len().saturating_sub(1) as f32
        };
        let mut result = DataTableLayout {
            frame,
            visible: own_clip,
            content_height,
            ..Default::default()
        };
        result.columns = (0..model.columns)
            .map(|i| {
                Rect::new(
                    frame.min_x() + i as f32 * cw,
                    frame.min_y(),
                    cw,
                    frame.size.height,
                )
            })
            .collect();
        if own_clip.is_none() {
            if mounted_bounds(frame, clip) {
                self.state.captured = None;
                self.state.hovered = None;
                result.headers = (0..model.columns)
                    .map(|id| CollectionItemLayout {
                        id,
                        frame: Rect::new(
                            frame.min_x() + id as f32 * cw,
                            frame.min_y(),
                            cw,
                            header_band,
                        ),
                        visible: None,
                        depth: 0,
                    })
                    .collect();
                let mut y = frame.min_y() + row_start;
                for (display, id) in valid_order.iter().copied().enumerate() {
                    result.rows.push(CollectionItemLayout {
                        id,
                        frame: Rect::new(frame.min_x(), y, width, row_heights[display]),
                        visible: None,
                        depth: 0,
                    });
                    y += row_heights[display] + gap;
                }
            } else {
                self.state.blur();
                self.focused_header = None;
            }
            for button in self
                .header_buttons
                .iter_mut()
                .chain(self.row_buttons.iter_mut())
            {
                button.clear_layout();
            }
            self.layout = result.clone();
            return result;
        }
        scene.push_clip(own_clip.expect("checked table clip"));
        self.header_buttons
            .resize_with(model.columns, ButtonModel::default);
        self.row_buttons
            .resize_with(model.rows, ButtonModel::default);
        for (col, _header_height) in header_heights.iter().copied().enumerate() {
            let r = Rect::new(
                frame.min_x() + col as f32 * cw,
                frame.min_y(),
                cw,
                header_band,
            );
            let custom = self
                .content_metrics
                .as_ref()
                .and_then(|m| m.headers.get(col))
                .is_some_and(Option::is_some);
            let title = if custom { "" } else { &header_labels[col] };
            let surface = self
                .content_metrics
                .as_ref()
                .and_then(|m| m.surface)
                .filter(|s| *s != "base");
            let button = &mut self.header_buttons[col];
            if let Some(surface) = surface {
                button.set_surface(surface);
            } else {
                button.clear_surface();
            }
            let header_focused = self.focused_header == Some(col) && self.state.window_active;
            button.state = control_state(false, false, header_focused, self.disabled, false);
            let button_layout =
                render_quiet_button(button, scene, text, title, r, own_clip, palette);
            result.headers.push(CollectionItemLayout {
                id: col,
                frame: r,
                visible: button_layout.visible,
                depth: 0,
            });
        }
        let mut y = frame.min_y() + row_start;
        for (display, id) in valid_order.iter().copied().enumerate() {
            let row_height = row_heights[display];
            let r = Rect::new(frame.min_x(), y, width, row_height);
            let focused = self.state.focused == Some(id) && self.state.window_active;
            let surface = self
                .content_metrics
                .as_ref()
                .and_then(|m| m.surface)
                .filter(|s| *s != "base")
                .unwrap_or("canvas");
            let button = &mut self.row_buttons[id];
            button.set_surface(surface);
            button.state = control_state(
                model.selected_row == Some(id),
                self.state.hovered == Some(id),
                focused,
                self.disabled,
                self.state.captured == Some(id),
            );
            let row_layout = render_quiet_button(button, scene, text, "", r, own_clip, palette);
            for col in 0..model.columns {
                let (custom, accessory) = self
                    .content_metrics
                    .as_ref()
                    .and_then(|m| m.cells.get(id))
                    .and_then(|r| r.get(col))
                    .copied()
                    .unwrap_or_default();
                if custom.is_some() {
                    continue;
                }
                let aw = accessory.map_or(0., |a| a.width.min((cw - 16.).max(0.)));
                let cell = Rect::new(
                    frame.min_x() + col as f32 * cw + 8.,
                    y,
                    (cw - 16. - aw).max(0.),
                    row_height,
                );
                if let Some(v) = visible(cell, row_layout.visible) {
                    scene.push_clip(v);
                    text.draw(
                        scene,
                        rows[id].get(col).map(String::as_str).unwrap_or(""),
                        proposal::TextRole::UiBody,
                        cell,
                        ButtonTextAlign::Leading,
                        Self::row_foreground(button, palette, surface).0,
                    );
                    scene.pop_clip();
                }
            }
            result.rows.push(CollectionItemLayout {
                id,
                frame: r,
                visible: row_layout.visible,
                depth: 0,
            });
            y += row_height + gap;
        }
        scene.pop_clip();
        if !self.disabled && self.state.window_active && self.state.focus_visible {
            let focused = if let Some(column) = self.focused_header {
                result.headers.iter().find(|i| i.id == column)
            } else {
                result
                    .rows
                    .iter()
                    .find(|i| Some(i.id) == self.state.focused)
            };
            if let Some(item) = focused {
                paint_collection_focus(scene, item, frame, clip, palette);
            }
        }
        self.layout = result.clone();
        result
    }
    pub fn pointer_move(&mut self, point: Point) -> bool {
        if self.keyed.is_some() {
            return false;
        }
        self.pointer_move_impl(point)
    }
    fn pointer_move_impl(&mut self, point: Point) -> bool {
        if self.disabled || !self.state.window_active {
            let changed = self.state.hovered.take().is_some();
            return changed;
        }
        let next = self
            .layout
            .rows
            .iter()
            .find(|i| i.visible.is_some_and(|r| r.contains(point)))
            .map(|i| i.id);
        let changed = self.state.hovered != next;
        self.state.hovered = next;
        changed
    }
    pub fn pointer_down(&mut self, point: Point, model: &mut DataTable) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.pointer_down_impl(point, model)
    }
    fn decide_table_down(&self, point: Point, model: &DataTable) -> Option<CollectionAction> {
        if self.disabled || !self.state.window_active {
            return None;
        }
        if let Some(id) = self
            .layout
            .headers
            .iter()
            .find(|r| r.visible.is_some_and(|r| r.contains(point)))
            .map(|r| r.id)
        {
            if id >= model.columns {
                return None;
            }
            return Some(CollectionAction {
                id,
                header: true,
                keyboard: false,
                capture: false,
                select: false,
                sort: true,
                next_sort: model.decided_sort(id),
                event: CollectionEvent::Changed,
            });
        }
        let id = self
            .layout
            .rows
            .iter()
            .find(|r| r.visible.is_some_and(|r| r.contains(point)))?
            .id;
        if id >= model.rows {
            return None;
        }
        Some(CollectionAction {
            id,
            header: false,
            keyboard: false,
            capture: true,
            select: true,
            sort: false,
            next_sort: None,
            event: if model.selected_row != Some(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Handled
            },
        })
    }
    fn pointer_down_impl(&mut self, point: Point, model: &mut DataTable) -> CollectionEvent {
        let Some(p) = self.decide_table_down(point, model) else {
            return CollectionEvent::Ignored;
        };
        let event = p.event;
        self.apply_table_action(p, model);
        event
    }
    pub(crate) fn table_action_snapshot(&self) -> TableActionSnapshot {
        TableActionSnapshot {
            layout: self.layout.clone(),
            state: self.state.clone(),
            disabled: self.disabled,
            header: self.focused_header,
        }
    }
    pub(crate) fn prepare_table_down(
        &self,
        point: Point,
        model: &crate::extended::KeyedDataTable,
    ) -> Option<CollectionAction> {
        if !self.table_current(model) || model.revision() == u64::MAX {
            return None;
        }
        self.decide_table_down(point, model.original())
    }
    pub(crate) fn prepare_table_part(
        &self,
        id: usize,
        header: bool,
        press: bool,
        model: &crate::extended::KeyedDataTable,
    ) -> Option<CollectionAction> {
        if !self.table_current(model)
            || self.disabled
            || !self.state.window_active
            || (press && model.revision() == u64::MAX)
        {
            return None;
        }
        if header {
            if id >= model.original().columns {
                return None;
            }
        } else if id >= model.original().rows || !model.row_displayed(id) {
            return None;
        }
        Some(CollectionAction {
            id,
            header,
            keyboard: true,
            capture: false,
            select: press && !header,
            sort: press && header,
            next_sort: if press && header {
                model.original().decided_sort(id)
            } else {
                None
            },
            event: if !press || header || model.original().selected_row != Some(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Activated(id)
            },
        })
    }
    pub(crate) fn apply_table_action(&mut self, p: CollectionAction, model: &mut DataTable) {
        if p.header {
            self.focus_header_impl(p.id, p.keyboard);
        } else {
            self.focused_header = None;
            self.state.focus(p.id, p.keyboard);
        }
        if p.capture {
            self.state.captured = Some(p.id);
        }
        if p.select {
            model.selected_row = Some(p.id);
        }
        if p.sort {
            model.sort = p.next_sort;
        }
    }
    pub fn pointer_up(&mut self, point: Point, model: &mut DataTable) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.pointer_up_impl(point, model)
    }
    fn pointer_up_impl(&mut self, point: Point, model: &mut DataTable) -> CollectionEvent {
        let Some(id) = self.state.captured.take() else {
            return CollectionEvent::Ignored;
        };
        if !self.state.window_active || self.disabled || id >= model.rows {
            return CollectionEvent::Ignored;
        }
        if self
            .layout
            .rows
            .iter()
            .any(|i| i.id == id && i.visible.is_some_and(|r| r.contains(point)))
        {
            if model.select_row(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Handled
            }
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool, model: &mut DataTable) -> CollectionEvent {
        if self.keyed.is_some() {
            return CollectionEvent::Ignored;
        }
        self.key_impl(key, down, model)
    }
    fn key_impl(&mut self, key: ControlKey, down: bool, model: &mut DataTable) -> CollectionEvent {
        if !down || self.disabled || !self.state.window_active {
            return CollectionEvent::Ignored;
        }
        if key == ControlKey::Escape {
            return if self.state.cancel_capture() {
                CollectionEvent::Handled
            } else {
                CollectionEvent::Ignored
            };
        }
        if self.state.focused.is_none() && self.focused_header.is_none() {
            return CollectionEvent::Ignored;
        }
        // Source membership alone is insufficient: the current consumer may
        // not have mounted this header/row. Clipped mounted rows remain valid.
        let mounted = if let Some(column) = self.focused_header {
            column < model.columns && self.layout.headers.iter().any(|h| h.id == column)
        } else {
            self.state
                .focused
                .is_some_and(|id| id < model.rows && self.layout.rows.iter().any(|r| r.id == id))
        };
        if !mounted {
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
        if let Some(column) = self.focused_header {
            if key == ControlKey::Enter {
                return if model.sort_by(column) {
                    CollectionEvent::Changed
                } else {
                    CollectionEvent::Handled
                };
            }
            if matches!(key, ControlKey::Left | ControlKey::Right) && model.columns > 0 {
                // Use the same mounted/live intersection as rows: source
                // growth does not publish geometry for new columns yet.
                let ids = self
                    .layout
                    .headers
                    .iter()
                    .filter(|header| header.id < model.columns)
                    .map(|header| header.id)
                    .collect::<Vec<_>>();
                let direction = if key == ControlKey::Right {
                    ControlKey::Down
                } else {
                    ControlKey::Up
                };
                let next = row_key_target(direction, column, &ids).unwrap_or(column);
                self.focus_header_impl(next, true);
                return CollectionEvent::Changed;
            }
            return CollectionEvent::Handled;
        }
        if key == ControlKey::Enter {
            let Some(id) = self.state.focused else {
                return CollectionEvent::Ignored;
            };
            if !self.layout.rows.iter().any(|row| row.id == id) {
                return CollectionEvent::Ignored;
            }
            return if model.select_row(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Activated(id)
            };
        }
        if self.layout.rows.is_empty() {
            return CollectionEvent::Ignored;
        }
        let visible_rows = self
            .layout
            .rows
            .iter()
            .filter(|row| row.id < model.rows)
            .map(|row| row.id)
            .collect::<Vec<_>>();
        if visible_rows.is_empty() {
            return CollectionEvent::Handled;
        }
        let target = match key {
            ControlKey::Home => visible_rows.first().copied(),
            ControlKey::End => visible_rows.last().copied(),
            ControlKey::Up | ControlKey::Down => {
                let origin = self.state.focused.or(model.selected_row);
                let position = origin.and_then(|id| visible_rows.iter().position(|row| *row == id));
                let position = match (key, position) {
                    (ControlKey::Down, Some(position)) => {
                        (position + 1).min(visible_rows.len() - 1)
                    }
                    (ControlKey::Up, Some(position)) => position.saturating_sub(1),
                    (ControlKey::Down, None) => 0,
                    (ControlKey::Up, None) => visible_rows.len() - 1,
                    _ => unreachable!("only vertical navigation reaches this branch"),
                };
                visible_rows.get(position).copied()
            }
            _ => None,
        };
        let Some(target) = target else {
            return CollectionEvent::Handled;
        };
        self.state.focus(target, true);
        if model.select_row(target) {
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        }
    }
    pub fn cancel(&mut self) {
        self.state.cancel_capture();
    }
    fn table_current(&self, model: &crate::extended::KeyedDataTable) -> bool {
        self.keyed.as_ref().is_some_and(|k| {
            k.arranged
                && std::sync::Arc::ptr_eq(&k.instance, &model.view_identity())
                && k.revision == model.revision()
        })
    }
    // Reconcile against accepted identities, not merely the most recent update.
    // Multiple replacements between calls therefore cannot hide an ABA.
    pub(crate) fn guard_table(&mut self, model: &crate::extended::KeyedDataTable) {
        let instance = model.view_identity();
        if self.keyed.as_ref().is_some_and(|k| {
            std::sync::Arc::ptr_eq(&k.instance, &instance) && k.revision == model.revision()
        }) {
            return;
        }
        let same = self
            .keyed
            .as_ref()
            .is_some_and(|k| std::sync::Arc::ptr_eq(&k.instance, &instance));
        let row = same
            .then(|| {
                self.state
                    .focused
                    .and_then(|i| self.keyed.as_ref()?.rows.get(i).copied())
            })
            .flatten()
            .and_then(|(key, incarnation)| {
                model.row_index(key).filter(|i| {
                    model.row_displayed(*i) && model.row_incarnation(*i) == Some(incarnation)
                })
            });
        let column = same
            .then(|| {
                self.focused_header
                    .and_then(|i| self.keyed.as_ref()?.columns.get(i).copied())
            })
            .flatten()
            .and_then(|(key, incarnation)| {
                model
                    .column_index(key)
                    .filter(|i| model.column_incarnation(*i) == Some(incarnation))
            });
        self.state.focused = row;
        self.focused_header = column;
        if row.is_none() && column.is_none() {
            self.state.focus_visible = false;
        }
        self.state.captured = None;
        self.state.hovered = None;
        self.layout = DataTableLayout::default();
        self.keyed = Some(TableViewIdentity {
            instance,
            revision: model.revision(),
            arranged: false,
            rows: model
                .snapshot()
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| (r.key, model.row_incarnation(i).expect("validated row")))
                .collect(),
            columns: model
                .snapshot()
                .columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    (
                        c.key,
                        model.column_incarnation(i).expect("validated column"),
                    )
                })
                .collect(),
        });
    }
    /// Arrange and paint through the original string-table layout service.
    /// Calling this is the only way to publish fresh keyed geometry.
    pub fn render_keyed(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedDataTable,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> DataTableLayout {
        self.guard_table(model);
        let headers = model
            .snapshot()
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        let rows = model
            .snapshot()
            .rows
            .iter()
            .map(|r| r.cells.clone())
            .collect::<Vec<_>>();
        let layout = self.render_impl(
            scene,
            text,
            model.original(),
            &headers,
            &rows,
            model.display_indices(),
            frame,
            clip,
            palette,
        );
        self.keyed.as_mut().expect("guarded table").arranged = true;
        layout
    }
    pub(crate) fn render_content_keyed(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        model: &crate::extended::KeyedDataTable,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        metrics: TableContentMetrics,
    ) -> DataTableLayout {
        self.content_metrics = Some(metrics);
        let layout = self.render_keyed(scene, text, model, frame, clip, palette);
        self.content_metrics = None;
        layout
    }
    pub(crate) fn invalidate_content_layout(&mut self) {
        self.layout = DataTableLayout::default();
        if let Some(k) = &mut self.keyed {
            k.arranged = false;
        }
        self.cancel();
    }
    pub(crate) fn content_retained_cost(&self) -> Option<(usize, usize)> {
        let mut units = 0usize;
        let mut bytes = 0usize;
        for (n, size) in [
            (
                self.header_buttons.capacity(),
                std::mem::size_of::<ButtonModel>(),
            ),
            (
                self.row_buttons.capacity(),
                std::mem::size_of::<ButtonModel>(),
            ),
            (
                self.layout.headers.capacity(),
                std::mem::size_of::<CollectionItemLayout>(),
            ),
            (
                self.layout.rows.capacity(),
                std::mem::size_of::<CollectionItemLayout>(),
            ),
            (self.layout.columns.capacity(), std::mem::size_of::<Rect>()),
        ] {
            units = units.checked_add(n)?;
            bytes = bytes.checked_add(n.checked_mul(size)?)?;
        }
        if let Some(k) = &self.keyed {
            for n in [k.rows.capacity(), k.columns.capacity()] {
                units = units.checked_add(n)?;
                bytes = bytes.checked_add(n.checked_mul(24)?)?;
            }
        }
        Some((units, bytes))
    }
    // Legacy strings and Content row child-theme share the original row
    // Button/state foreground. This approved legacy ink change leaves layout,
    // hit testing, focus and selection policies in their existing owners.
    fn row_foreground(
        button: &ButtonModel,
        palette: Palette,
        surface: &str,
    ) -> (tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba) {
        let surface = if surface == "base" { "canvas" } else { surface };
        let colors = button.surface_colors(palette, ButtonVariant::Quiet, surface);
        (colors.text, colors.icon)
    }
    pub(crate) fn content_row_ink(
        &self,
        index: usize,
        palette: Palette,
        surface: &str,
    ) -> Option<(tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba)> {
        self.row_buttons
            .get(index)
            .map(|button| Self::row_foreground(button, palette, surface))
    }
    /// Pure query: stale identity/geometry is unavailable and does not reconcile.
    pub fn layout_keyed(
        &self,
        model: &crate::extended::KeyedDataTable,
    ) -> Option<&DataTableLayout> {
        self.table_current(model).then_some(&self.layout)
    }
    pub fn focused_row_keyed(
        &self,
        model: &crate::extended::KeyedDataTable,
    ) -> Option<crate::extended::TableRowKey> {
        self.table_current(model)
            .then(|| self.state.focused.and_then(|i| model.row_key(i)))
            .flatten()
    }
    pub fn focused_header_keyed(
        &self,
        model: &crate::extended::KeyedDataTable,
    ) -> Option<crate::extended::TableColumnKey> {
        self.table_current(model)
            .then(|| self.focused_header.and_then(|i| model.column_key(i)))
            .flatten()
    }
    /// Internal focus entry for a host that has already passed its focus barrier.
    pub fn focus_keyed(&mut self, keyboard: bool, model: &crate::extended::KeyedDataTable) -> bool {
        self.guard_table(model);
        if !self.table_current(model) || self.disabled || !self.state.window_active {
            return false;
        }
        let row = model
            .original()
            .selected_row
            .or_else(|| model.display_indices().first().copied());
        let Some(row) = row else {
            return false;
        };
        self.focus_row_impl(row, keyboard);
        true
    }
    pub fn focus_row_keyed(
        &mut self,
        key: crate::extended::TableRowKey,
        keyboard: bool,
        model: &crate::extended::KeyedDataTable,
    ) -> bool {
        self.guard_table(model);
        if !self.table_current(model) || self.disabled || !self.state.window_active {
            return false;
        }
        let Some(i) = model.row_index(key).filter(|i| model.row_displayed(*i)) else {
            return false;
        };
        self.focus_row_impl(i, keyboard);
        true
    }
    pub fn focus_header_keyed(
        &mut self,
        key: crate::extended::TableColumnKey,
        keyboard: bool,
        model: &crate::extended::KeyedDataTable,
    ) -> bool {
        self.guard_table(model);
        if !self.table_current(model) || self.disabled || !self.state.window_active {
            return false;
        }
        let Some(i) = model.column_index(key) else {
            return false;
        };
        self.focus_header_impl(i, keyboard);
        true
    }
    pub fn pointer_move_keyed(
        &mut self,
        point: Point,
        model: &crate::extended::KeyedDataTable,
    ) -> bool {
        self.guard_table(model);
        self.table_current(model) && self.pointer_move_impl(point)
    }
    fn input_keyed(
        &mut self,
        input: TableInput,
        model: &mut crate::extended::KeyedDataTable,
    ) -> CollectionEvent {
        self.guard_table(model);
        if !self.table_current(model) {
            return CollectionEvent::Ignored;
        }
        let event = model.view_input(self, input);
        self.guard_table(model);
        event
    }
    pub fn pointer_down_keyed(
        &mut self,
        point: Point,
        model: &mut crate::extended::KeyedDataTable,
    ) -> CollectionEvent {
        self.input_keyed(TableInput::Down(point), model)
    }
    pub fn pointer_up_keyed(
        &mut self,
        point: Point,
        model: &mut crate::extended::KeyedDataTable,
    ) -> CollectionEvent {
        self.input_keyed(TableInput::Up(point), model)
    }
    pub fn key_keyed(
        &mut self,
        key: ControlKey,
        down: bool,
        model: &mut crate::extended::KeyedDataTable,
    ) -> CollectionEvent {
        self.input_keyed(TableInput::Key(key, down), model)
    }
    pub(crate) fn reduce_table(
        &mut self,
        input: TableInput,
        model: &mut DataTable,
    ) -> CollectionEvent {
        match input {
            TableInput::Down(point) => self.pointer_down_impl(point, model),
            TableInput::Up(point) => self.pointer_up_impl(point, model),
            TableInput::Key(key, down) => self.key_impl(key, down, model),
        }
    }
}

mod chart_owner;
pub use chart_owner::{ChartOwnerCapacity, ChartOwnerFocusSpace, ChartOwnerMeasurement};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartViewLayout {
    pub frame: Rect,
    pub visible: Option<Rect>,
    pub geometry: ChartGeometry,
}
#[derive(Clone, Debug)]
pub struct ChartView {
    layout: ChartViewLayout,
    owner_band: Option<chart_owner::OwnerBand>,
    hovered: Option<usize>,
    focused: bool,
    focus_visible: bool,
    window_active: bool,
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

fn chart_label_metrics(
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
    draw_chart_labels_with_inks(scene, text, geometry, |category| {
        if category {
            palette
                .color("secondary-text")
                .expect("DS secondary text token")
        } else {
            palette
                .color("primary-text")
                .expect("DS primary text token")
        }
    });
}

fn draw_chart_labels_with_inks(
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

fn chart_text_frame(
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
            let width = text
                .measure(value, proposal::TextRole::UiCaption, slot.size.width)
                .width
                .clamp(0.0, slot.size.width);
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
    layout: DocumentLayout,
}
impl MarkdownView {
    pub fn new() -> Self {
        Self::default()
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
        // The original Gallery specimen started four points below its story
        // body origin; preserve that breathing room while owning its layout.
        let mut y = frame.min_y() + 4.0;
        let mut blocks = Vec::with_capacity(model.blocks.len());
        for (index, block) in model.blocks.iter().enumerate() {
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
            let rendered = if matches!(block, MarkdownBlock::Bullet(_)) {
                format!("• {value}")
            } else {
                value.to_owned()
            };
            let measured = text.measure(&rendered, role, max_width);
            // `measure` reflects the active host scale; only the token floor is
            // scaled here so the gallery does not apply its text scale twice.
            let h = measured.height.max(min_h * scale).max(24.);
            let rect = Rect::new(frame.min_x() + indent, y, max_width, h);
            let draw_rect = Rect::new(rect.min_x(), rect.min_y(), rect.size.width, measured.height);
            let v = visible(rect, own_clip);
            if let Some(v) = v {
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
}
impl CodeBlockView {
    pub fn new() -> Self {
        Self::default()
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
        let inset = proposal::metric("spacing", "space-button-md-inset").unwrap_or(14.);
        let mut blocks = Vec::new();
        let mut y = frame.min_y() + inset;
        let x = frame.min_x() + inset;
        // The code surface owns an inset on both sides. Measuring only from
        // the left inset made narrow samples measure wider than their visible
        // area: the final glyphs were clipped at the right edge instead of
        // wrapping inside the code block. Preserve the 650pt wide specimen
        // slot while keeping narrow lines inside the same 14pt inset.
        let w = (frame.size.width - inset * 2.).clamp(0., 650.);
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
    layout: DocumentLayout,
}
impl DiffViewRuntime {
    pub fn new() -> Self {
        Self::default()
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
        let line_floor = 28. * scale;
        let mut blocks = Vec::with_capacity(model.lines.len());
        let inset = 8.0;
        let line_gap = 2.0 * scale;
        let mut y = frame.min_y();
        for (index, line) in model.lines.iter().enumerate() {
            let (prefix, color) = match line.kind {
                DiffKind::Equal => ("  ", "primary-text"),
                DiffKind::Added => ("+ ", "status-succeeded"),
                DiffKind::Removed => ("− ", "status-failed"),
            };
            let rendered = format!("{prefix}{}", line.text);
            // Diff text starts after an 8pt leading inset, so reserve the same
            // amount at the trailing edge. This keeps wrapped prefixes and
            // the corresponding line geometry inside the component frame.
            let content_width = (frame.size.width - inset * 2.).clamp(0., 700.);
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

/// Compatibility spelling for hosts that prefer the component name.
pub type DiffViewPresenter = DiffViewRuntime;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{Primitive, Quad, Size};
    use tekes_ui_tokens::{Rgba, Theme};
    struct FixedText {
        height: f32,
    }

    struct ScaledText(f32);

    impl ButtonTextRenderer for ScaledText {
        fn backing_scale(&self) -> f32 {
            self.0
        }

        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(max_width.min(100.0), 14.0)
        }

        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }
    impl ButtonTextRenderer for FixedText {
        fn measure(&mut self, _text: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(max_width.min(100.), self.height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            _text: &str,
            _role: proposal::TextRole,
            _rect: Rect,
            _align: ButtonTextAlign,
            _ink: Rgba,
        ) {
        }
    }
    #[derive(Default)]
    struct RecordingText {
        measured: Vec<(String, f32)>,
        drawn: Vec<String>,
        draw_rects: Vec<Rect>,
        alignments: Vec<ButtonTextAlign>,
        ink_origins: Vec<Point>,
        ink_widths: Vec<f32>,
    }
    impl ButtonTextRenderer for RecordingText {
        fn measure(&mut self, value: &str, _role: proposal::TextRole, max_width: f32) -> Size {
            self.measured.push((value.to_owned(), max_width));
            let height = if value.chars().count() > 12 { 72. } else { 14. };
            Size::new(max_width.min(100.), height)
        }
        fn draw(
            &mut self,
            _scene: &mut Scene,
            value: &str,
            role: proposal::TextRole,
            rect: Rect,
            align: ButtonTextAlign,
            _ink: Rgba,
        ) {
            // Mirror the Gallery text engine's `label_origin` contract so
            // tests verify the requested ink origin as well as the slot.
            let ink_width = self.measure(value, role, rect.size.width).width;
            let ink_x = match align {
                ButtonTextAlign::Leading => rect.min_x(),
                ButtonTextAlign::Center => rect.center().x - ink_width * 0.5,
            };
            self.drawn.push(value.to_owned());
            self.draw_rects.push(rect);
            self.alignments.push(align);
            self.ink_origins.push(Point::new(ink_x, rect.min_y()));
            self.ink_widths.push(ink_width);
        }
    }
    #[derive(Default)]
    struct RecordingIcon(Vec<(String, Rect, Rgba)>);
    impl IconRenderer for RecordingIcon {
        fn draw_icon(&mut self, scene: &mut Scene, canonical_name: &str, frame: Rect, tint: Rgba) {
            self.0.push((canonical_name.to_owned(), frame, tint));
            scene.push_quad(Quad::new(frame, tint));
        }
    }
    fn rect() -> Rect {
        Rect::new(10., 20., 240., 180.)
    }
    #[test]
    fn wholly_offscreen_mounted_collections_keep_keyboard_geometry_but_no_paint_or_hits() {
        let palette = Palette::from(Theme::LIGHT);
        let onscreen = Rect::new(0., 0., 240., 240.);
        let offscreen = Rect::new(0., 400., 240., 240.);
        let clip = Some(Rect::new(0., 0., 240., 240.));
        let labels: [String; 4] = ["one".into(), "two".into(), "three".into(), "four".into()];
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        let mut list = List::new(4);
        let mut list_view = ListView::new();
        list_view.render(
            &mut scene, &mut text, &list, &labels, onscreen, clip, palette,
        );
        list_view.focus(0, true);
        scene = Scene::new();
        text.drawn.clear();
        let layout = list_view.render(
            &mut scene, &mut text, &list, &labels, offscreen, clip, palette,
        );
        assert_eq!(layout.items.len(), 4);
        assert_eq!(layout.visible, None);
        assert!(layout.items.iter().all(|row| row.visible.is_none()));
        assert!(scene.primitives().is_empty());
        assert!(text.drawn.is_empty());
        assert_eq!(
            list_view.key(ControlKey::End, true, &mut list),
            CollectionEvent::Changed
        );
        assert_eq!(list.selected, Some(3));
        assert_eq!(
            list_view.pointer_down(layout.items[3].frame.center(), &mut list),
            CollectionEvent::Ignored
        );
        assert_eq!(
            list_view.key(ControlKey::Home, true, &mut list),
            CollectionEvent::Changed
        );
        assert_eq!(list.selected, Some(0));
        list_view.render(
            &mut scene,
            &mut text,
            &list,
            &labels,
            offscreen,
            Some(Rect::default()),
            palette,
        );
        assert!(list_view.layout().items.is_empty());
        assert_eq!(
            list_view.key(ControlKey::End, true, &mut list),
            CollectionEvent::Ignored
        );

        let mut tree = Tree::default();
        for label in ["one", "two", "three", "four"] {
            tree.add(label, None);
        }
        let mut tree_view = TreeView::new();
        tree_view.render(&mut scene, &mut text, &tree, onscreen, clip, palette);
        tree_view.focus(0, true);
        scene = Scene::new();
        text.drawn.clear();
        let layout = tree_view.render(&mut scene, &mut text, &tree, offscreen, clip, palette);
        assert_eq!(layout.items.len(), 4);
        assert!(layout.items.iter().all(|row| row.visible.is_none()));
        assert!(scene.primitives().is_empty());
        assert!(text.drawn.is_empty());
        tree_view.key(ControlKey::End, true, &mut tree);
        assert_eq!(tree.selected, Some(3));
        assert_eq!(
            tree_view.pointer_down(layout.items[3].frame.center(), &mut tree),
            CollectionEvent::Ignored
        );
        tree_view.key(ControlKey::Home, true, &mut tree);
        assert_eq!(tree.selected, Some(0));
        tree_view.render(
            &mut scene,
            &mut text,
            &tree,
            offscreen,
            Some(Rect::default()),
            palette,
        );
        assert!(tree_view.layout().items.is_empty());
        assert_eq!(
            tree_view.key(ControlKey::End, true, &mut tree),
            CollectionEvent::Ignored
        );

        let rows = labels
            .iter()
            .map(|label| vec![label.clone()])
            .collect::<Vec<_>>();
        let headers = ["Name".into()];
        let order = [0, 1, 2, 3];
        let mut table = DataTable::new(4, 1);
        let mut table_view = DataTableView::new();
        table_view.render(
            &mut scene, &mut text, &table, &headers, &rows, &order, onscreen, clip, palette,
        );
        table_view.focus_row(0, true);
        scene = Scene::new();
        text.drawn.clear();
        let layout = table_view.render(
            &mut scene, &mut text, &table, &headers, &rows, &order, offscreen, clip, palette,
        );
        assert_eq!(layout.rows.len(), 4);
        assert_eq!(layout.headers.len(), 1);
        assert!(layout.rows.iter().all(|row| row.visible.is_none()));
        assert!(scene.primitives().is_empty());
        assert!(text.drawn.is_empty());
        table_view.key(ControlKey::End, true, &mut table);
        assert_eq!(table.selected_row, Some(3));
        assert_eq!(
            table_view.pointer_down(layout.rows[3].frame.center(), &mut table),
            CollectionEvent::Ignored
        );
        table_view.key(ControlKey::Home, true, &mut table);
        assert_eq!(table.selected_row, Some(0));
        table_view.render(
            &mut scene,
            &mut text,
            &table,
            &headers,
            &rows,
            &order,
            offscreen,
            Some(Rect::default()),
            palette,
        );
        assert!(table_view.layout().rows.is_empty());
        assert_eq!(
            table_view.key(ControlKey::End, true, &mut table),
            CollectionEvent::Ignored
        );
        let mut chart = BarChart::new(vec![
            crate::ChartDatum::new("first", 1.),
            crate::ChartDatum::new("last", 2.),
        ]);
        let mut chart_view = ChartView::new();
        chart_view.render(
            &mut scene, &mut text, &mut chart, onscreen, clip, palette, 1.,
        );
        chart_view.focus(true, &mut chart);
        scene = Scene::new();
        text.drawn.clear();
        let layout = chart_view.render(
            &mut scene, &mut text, &mut chart, offscreen, clip, palette, 1.,
        );
        assert_eq!(layout.geometry.bars.len(), 2);
        assert_eq!(layout.visible, None);
        assert!(scene.primitives().is_empty());
        assert!(text.drawn.is_empty());
        assert_eq!(
            chart_view.key(ControlKey::End, true, &mut chart),
            CollectionEvent::Changed
        );
        assert_eq!(chart.selected(), Some(1));
        assert_eq!(
            chart_view.pointer_down(layout.geometry.bars[1].rect.center(), &mut chart),
            CollectionEvent::Ignored
        );
        chart_view.key(ControlKey::Home, true, &mut chart);
        assert_eq!(chart.selected(), Some(0));
        chart_view.render(
            &mut scene,
            &mut text,
            &mut chart,
            offscreen,
            Some(Rect::default()),
            palette,
            1.,
        );
        assert!(chart_view.layout().geometry.bars.is_empty());
        assert_eq!(
            chart_view.key(ControlKey::End, true, &mut chart),
            CollectionEvent::Ignored
        );
    }

    #[test]
    fn offscreen_collection_rows_remain_keyboard_targets_without_pointer_hits() {
        let mut scene = Scene::new();
        let mut text = FixedText { height: 14. };
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(0., 0., 240., 240.);
        let clip = Some(Rect::new(0., 0., 240., 78.));
        let mut list = List::new(4);
        let mut list_view = ListView::new();
        list_view.render(
            &mut scene,
            &mut text,
            &list,
            &["one".into(), "two".into(), "three".into(), "four".into()],
            frame,
            clip,
            palette,
        );
        list_view.focus(0, true);
        list_view.key(ControlKey::End, true, &mut list);
        assert_eq!(list.selected, Some(3));
        assert!(list_view.layout().items[3].visible.is_none());
        assert_eq!(
            list_view.pointer_down(list_view.layout().items[3].frame.center(), &mut list),
            CollectionEvent::Ignored
        );
        list_view.focus(3, true);
        assert_eq!(
            list_view.key(ControlKey::Enter, true, &mut list),
            CollectionEvent::Activated(3)
        );
        assert_eq!(list.selected, Some(3));

        let mut tree = Tree::default();
        for label in ["one", "two", "three", "four"] {
            tree.add(label, None);
        }
        tree.set_disabled(1, true);
        let mut tree_view = TreeView::new();
        tree_view.render(&mut scene, &mut text, &tree, frame, clip, palette);
        tree_view.focus(0, true);
        tree_view.key(ControlKey::End, true, &mut tree);
        assert_eq!(tree.selected, Some(3));
        assert!(tree_view.layout().items[3].visible.is_none());
        assert_eq!(
            tree_view.pointer_down(tree_view.layout().items[3].frame.center(), &mut tree),
            CollectionEvent::Ignored
        );
        tree_view.focus(3, true);
        assert_eq!(
            tree_view.key(ControlKey::Enter, true, &mut tree),
            CollectionEvent::Activated(3)
        );

        let mut table = DataTable::new(4, 1);
        let mut table_view = DataTableView::new();
        table_view.render(
            &mut scene,
            &mut text,
            &table,
            &["header".into()],
            &[
                vec!["one".into()],
                vec!["two".into()],
                vec!["three".into()],
                vec!["four".into()],
            ],
            &[0, 1, 2, 3],
            frame,
            clip,
            palette,
        );
        table_view.focus_row(3, true);
        assert_eq!(
            table_view.key(ControlKey::Enter, true, &mut table),
            CollectionEvent::Changed
        );
        assert_eq!(table.selected_row, Some(3));
        assert!(table_view.layout().rows[3].visible.is_none());
        assert_eq!(
            table_view.pointer_down(table_view.layout().rows[3].frame.center(), &mut table),
            CollectionEvent::Ignored
        );
    }

    #[test]
    fn collection_keys_follow_restored_focus_and_reject_disabled_tree_focus() {
        let mut scene = Scene::new();
        let mut text = FixedText { height: 14. };
        let palette = Palette::from(Theme::LIGHT);
        let frame = Rect::new(0., 0., 240., 240.);
        let mut list = List::new(4);
        list.select(0);
        let mut view = ListView::new();
        view.render(
            &mut scene,
            &mut text,
            &list,
            &["one".into(), "two".into(), "three".into(), "four".into()],
            frame,
            None,
            palette,
        );
        view.focus(2, true);
        assert_eq!(
            view.key(ControlKey::Down, true, &mut list),
            CollectionEvent::Changed
        );
        assert_eq!(list.selected, Some(3));
        view.focus(1, true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut list),
            CollectionEvent::Activated(1)
        );
        assert_eq!(list.selected, Some(1));

        let mut tree = Tree::default();
        tree.add("one", None);
        tree.add("two", None);
        tree.add("three", None);
        tree.select(0);
        let mut view = TreeView::new();
        view.render(&mut scene, &mut text, &tree, frame, None, palette);
        view.focus(1, true);
        assert_eq!(
            view.key(ControlKey::Down, true, &mut tree),
            CollectionEvent::Changed
        );
        assert_eq!(tree.selected, Some(2));
        view.focus(1, true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut tree),
            CollectionEvent::Activated(1)
        );
        tree.set_disabled(1, true);
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut tree),
            CollectionEvent::Ignored
        );
        assert_eq!(tree.selected, None);
    }

    #[test]
    fn list_uses_owned_clip_geometry_and_clips_wide_text() {
        let mut view = ListView::new();
        let model = List::new(4);
        let labels = (0..4).map(|i| format!("row {i}")).collect::<Vec<_>>();
        let mut text = FixedText { height: 60. };
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            &labels,
            rect(),
            Some(Rect::new(10., 20., 240., 30.)),
            Theme::LIGHT.into(),
        );
        assert_eq!(layout.items.len(), 4);
        assert!(layout.items[0].visible.is_some());
        assert!(layout.items[1].visible.is_none());
        assert!(layout.items[0].frame.size.height >= 60.);
    }

    #[test]
    fn list_tree_and_table_keep_gallery_default_row_geometry() {
        let mut scene = Scene::new();
        let mut text = FixedText { height: 14. };

        let mut list = ListView::new();
        let list_model = List::new(2);
        let list_layout = list.render(
            &mut scene,
            &mut text,
            &list_model,
            &["first".into(), "second".into()],
            Rect::new(0., 0., 200., 100.),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(list_layout.items[0].frame.size.height, 36.);
        assert_eq!(list_layout.items[1].frame.min_y(), 40.);

        let mut tree = Tree::default();
        let root = tree.add("root", None).unwrap();
        let child = tree.add("child", Some(root)).unwrap();
        tree.toggle(root);
        let mut tree_view = TreeView::new();
        let tree_layout = tree_view.render(
            &mut scene,
            &mut text,
            &tree,
            Rect::new(0., 0., 200., 100.),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(tree_layout.items[0].frame.size.height, 34.);
        assert_eq!(tree_layout.items[1].frame.min_y(), 38.);
        assert_eq!(
            tree_layout.items[1].frame.min_x() - tree_layout.items[0].frame.min_x(),
            20.
        );
        assert_eq!(tree_layout.items[1].id, child);

        let table_model = DataTable::new(2, 1);
        let mut table_view = DataTableView::new();
        let table_layout = table_view.render(
            &mut scene,
            &mut text,
            &table_model,
            &["Name".into()],
            &[vec!["first".into()], vec!["second".into()]],
            &[0, 1],
            Rect::new(0., 0., 200., 120.),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(table_layout.headers[0].frame.size.height, 34.);
        assert_eq!(table_layout.rows[0].frame.min_y(), 40.);
        assert_eq!(table_layout.rows[0].frame.size.height, 34.);
        assert_eq!(
            table_layout.rows[1].frame.min_y() - table_layout.rows[0].frame.min_y(),
            38.
        );
    }

    #[test]
    fn tree_uses_fixed_real_icon_slot_and_clips_branch_icons() {
        let mut tree = Tree::default();
        let root = tree.add("Workspace", None).unwrap();
        let branch = tree.add("Documents", Some(root)).unwrap();
        tree.add("Report", Some(branch)).unwrap();
        let leaf = tree.add("Settings", None).unwrap();
        tree.toggle(root);
        let mut view = TreeView::new();
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        let frame = Rect::new(20.0, 30.0, 360.0, 120.0);
        let full_clip = frame;
        let layout = view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            Some(full_clip),
            Theme::LIGHT.into(),
        );
        assert_eq!(
            layout.items.iter().map(|row| row.id).collect::<Vec<_>>(),
            [root, branch, leaf]
        );
        assert_eq!(text.drawn, ["Workspace", "Documents", "Settings"]);
        assert_eq!(
            text.ink_origins[0].x, text.ink_origins[2].x,
            "branch and leaf at the same depth keep the same label origin"
        );
        assert_eq!(text.draw_rects[0].min_x(), text.draw_rects[2].min_x());

        let mut icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut icons,
            Some(full_clip),
            Theme::LIGHT.into(),
            16.0,
        );
        assert_eq!(
            icons
                .0
                .iter()
                .map(|item| item.0.as_str())
                .collect::<Vec<_>>(),
            ["tekes-expand", "tekes-forward"]
        );
        assert!(
            icons
                .0
                .iter()
                .all(|(_, rect, _)| rect.size.width == 16.0 && rect.size.height == 16.0)
        );
        let expected_ink = Palette::from(Theme::LIGHT).color("secondary-text").unwrap();
        assert!(
            icons.0.iter().all(|(_, _, tint)| *tint == expected_ink),
            "Quiet/rest icon tint: actual={:?}, expected secondary-text={:?}",
            icons.0,
            expected_ink
        );

        view.set_disabled(true);
        view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            Some(full_clip),
            Theme::LIGHT.into(),
        );
        let mut disabled_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut disabled_icons,
            Some(full_clip),
            Theme::LIGHT.into(),
            16.0,
        );
        let disabled_ink = Palette::from(Theme::LIGHT).color("tertiary-text").unwrap();
        assert!(
            disabled_icons
                .0
                .iter()
                .all(|(_, _, tint)| *tint == disabled_ink),
            "Quiet/disabled icon tint: actual={:?}, expected tertiary-text={:?}",
            disabled_icons.0,
            disabled_ink
        );
        view.set_disabled(false);

        let clipped = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 34.0);
        view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            Some(clipped),
            Theme::LIGHT.into(),
        );
        let mut clipped_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut clipped_icons,
            Some(clipped),
            Theme::LIGHT.into(),
            16.0,
        );
        assert_eq!(
            clipped_icons
                .0
                .iter()
                .map(|item| item.0.as_str())
                .collect::<Vec<_>>(),
            ["tekes-expand"]
        );
        assert!(clipped_icons.0[0].1.intersection(&clipped).is_some());

        view.render(
            &mut scene,
            &mut text,
            &tree,
            Rect::new(frame.min_x(), frame.min_y(), 0.0, frame.size.height),
            Some(full_clip),
            Theme::LIGHT.into(),
        );
        let mut hidden_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut hidden_icons,
            None,
            Theme::LIGHT.into(),
            16.0,
        );
        assert!(
            hidden_icons.0.is_empty(),
            "hidden render clears old icon slots"
        );
    }

    #[test]
    fn tree_icon_render_reuses_full_and_partial_row_clips() {
        let mut tree = Tree::default();
        let root = tree.add("Workspace", None).unwrap();
        let branch = tree.add("Documents", Some(root)).unwrap();
        tree.add("Report", Some(branch)).unwrap();
        tree.toggle(root);
        let mut view = TreeView::new();
        let mut scene = Scene::new();
        let mut text = FixedText { height: 14.0 };
        let frame = Rect::new(0.0, 0.0, 300.0, 120.0);
        view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            None,
            Theme::LIGHT.into(),
        );
        scene.clear();
        let mut visible_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut visible_icons,
            None,
            Theme::LIGHT.into(),
            16.0,
        );
        assert_eq!(
            visible_icons
                .0
                .iter()
                .map(|icon| icon.0.as_str())
                .collect::<Vec<_>>(),
            ["tekes-expand", "tekes-forward"]
        );

        let full_clip = Rect::new(0.0, 0.0, frame.size.width, 34.0);
        view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            Some(full_clip),
            Theme::LIGHT.into(),
        );
        scene.clear();
        let mut row_clipped_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut row_clipped_icons,
            None,
            Theme::LIGHT.into(),
            16.0,
        );
        assert_eq!(
            row_clipped_icons
                .0
                .iter()
                .map(|icon| icon.0.as_str())
                .collect::<Vec<_>>(),
            ["tekes-expand"]
        );

        let partial_clip = Rect::new(0.0, 42.0, frame.size.width, 10.0);
        let partial_layout = view.render(
            &mut scene,
            &mut text,
            &tree,
            frame,
            Some(partial_clip),
            Theme::LIGHT.into(),
        );
        scene.clear();
        let mut partial_icons = RecordingIcon::default();
        view.render_icons(
            &mut scene,
            &mut partial_icons,
            None,
            Theme::LIGHT.into(),
            16.0,
        );
        assert_eq!(
            partial_icons
                .0
                .iter()
                .map(|icon| icon.0.as_str())
                .collect::<Vec<_>>(),
            ["tekes-forward"]
        );
        let clips = scene
            .primitives()
            .iter()
            .filter_map(|primitive| {
                if let Primitive::PushClip(rect) = primitive {
                    Some(*rect)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let icon_slot = Rect::new(
            partial_layout.items[1].frame.min_x()
                + proposal::metric("spacing", "space-relaxed").unwrap_or(12.0),
            partial_layout.items[1].frame.min_y(),
            proposal::metric("size", "size-inline-action").expect("Tree expand action slot token"),
            partial_layout.items[1].frame.size.height,
        );
        assert_eq!(
            clips,
            [icon_slot.intersection(&partial_clip).unwrap()],
            "icon pass must retain the rendered row clip and remain inside its reserved slot"
        );
    }

    #[test]
    fn data_table_measures_final_sorted_header_label_before_positioning_rows() {
        let mut model = DataTable::new(1, 2);
        model.sort_by(0);
        let headers = ["name field".to_owned(), "Kind".to_owned()];
        let rows = vec![vec!["A".to_owned(), "Text".to_owned()]];
        let mut view = DataTableView::new();
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        let frame = Rect::new(10.0, 20.0, 400.0, 180.0);
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            &headers,
            &rows,
            &[0],
            frame,
            Some(frame),
            Theme::LIGHT.into(),
        );
        assert!(
            text.measured
                .iter()
                .any(|(label, _)| label == "name field  ↕")
        );
        assert_eq!(layout.headers[0].frame.size.height, 80.0);
        assert_eq!(layout.rows[0].frame.min_y(), frame.min_y() + 86.0);
        assert_eq!(layout.content_height, 120.0);
    }

    #[test]
    fn list_down_and_up_without_render_keeps_capture_and_updates_model() {
        let mut view = ListView::new();
        let mut model = List::new(2);
        let labels = vec!["a".into(), "b".into()];
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            &model,
            &labels,
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let point = Point::new(20., view.layout.items[1].frame.min_y() + 2.);
        assert_eq!(
            view.pointer_down(point, &mut model),
            CollectionEvent::Changed
        );
        assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Handled);
        assert_eq!(model.selected, Some(1));
    }
    #[test]
    fn tree_expansion_rebuild_changes_visible_row_ids_not_node_identity() {
        let mut tree = Tree::default();
        let root = tree.add("root", None).unwrap();
        let child = tree.add("child", Some(root)).unwrap();
        let mut view = TreeView::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            &tree,
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(
            view.layout.items.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec![root]
        );
        tree.toggle(root);
        view.render(
            &mut scene,
            &mut text,
            &tree,
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(
            view.layout.items.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec![root, child]
        );
        assert_eq!(view.layout.items[1].depth, 1);
    }
    #[test]
    fn data_table_sorted_layout_routes_to_stable_source_row() {
        let mut table = DataTable::new(2, 1);
        let rows = vec![vec!["z".into()], vec!["a".into()]];
        let mut view = DataTableView::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            &table,
            &["Name".into()],
            &rows,
            &[1, 0],
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let point = Point::new(20., view.layout.rows[0].frame.min_y() + 2.);
        assert_eq!(
            view.pointer_down(point, &mut table),
            CollectionEvent::Changed
        );
        assert_eq!(table.selected_row, Some(1));
    }
    #[test]
    fn window_deactivation_cancels_captured_list_row() {
        let mut view = ListView::new();
        let mut model = List::new(1);
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["a".into()],
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let point = Point::new(20., 25.);
        view.pointer_down(point, &mut model);
        view.set_window_active(false);
        assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Ignored);
    }

    #[test]
    fn table_skips_clipped_rows_and_disabled_table_cannot_sort() {
        let mut model = DataTable::new(2, 1);
        let rows = vec![vec!["first".into()], vec!["second".into()]];
        let mut view = DataTableView::new();
        let mut text = FixedText { height: 56. };
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &model,
            &["Name".into()],
            &rows,
            &[0, 1],
            rect(),
            Some(Rect::new(10., 20., 90., 32.)),
            Theme::LIGHT.into(),
        );
        assert!(layout.rows.iter().all(|row| row.visible.is_none()));
        view.set_disabled(true);
        assert_eq!(
            view.pointer_down(Point::new(20., 25.), &mut model),
            CollectionEvent::Ignored
        );
        assert_eq!(model.sort, None);
    }

    #[test]
    fn chart_routes_only_visible_real_bar_geometry() {
        let mut model = BarChart::new(vec![
            crate::ChartDatum::new("visible", 3.),
            crate::ChartDatum::new("clipped", 7.),
        ]);
        let mut view = ChartView::new();
        let mut text = FixedText { height: 12. };
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &mut model,
            Rect::new(0., 0., 300., 140.),
            Some(Rect::new(0., 0., 60., 140.)),
            Theme::LIGHT.into(),
            1.,
        );
        assert!(layout.visible.is_some());
        assert!(
            layout
                .geometry
                .bars
                .iter()
                .all(|bar| bar.rect.min_x() >= 0.)
        );
        assert!(!view.pointer_move(Point::new(280., 80.), &mut model));
    }

    #[test]
    fn markdown_and_code_layout_expand_to_host_measure_and_clip() {
        let markdown = Markdown::parse("# Heading\n\nA long paragraph");
        let mut view = MarkdownView::new();
        let mut text = FixedText { height: 64. };
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &markdown,
            rect(),
            Some(Rect::new(10., 20., 240., 40.)),
            Theme::LIGHT.into(),
            1.,
        );
        assert!(layout.blocks[0].frame.size.height >= 64.);
        assert!(layout.blocks[0].visible.is_some());
        assert!(layout.blocks[1].visible.is_none());

        let mut code_view = CodeBlockView::new();
        let code_layout = code_view.render(
            &mut scene,
            &mut text,
            &CodeBlock {
                language: "rs".into(),
                source: "line one\nline two".into(),
            },
            Rect::new(0., 0., 80., 30.),
            None,
            Theme::LIGHT.into(),
            1.5,
        );
        assert!(code_layout.content_height > 30.);
        assert_eq!(
            code_layout
                .blocks
                .iter()
                .filter(|line| line.visible.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn diff_presentation_keeps_stable_line_ids_when_clipped() {
        let diff = DiffView::between("same\nold", "same\nnew");
        let mut view = DiffViewRuntime::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        let layout = view.render(
            &mut scene,
            &mut text,
            &diff,
            Rect::new(0., 0., 200., 100.),
            Some(Rect::new(0., 0., 200., 31.)),
            Theme::LIGHT.into(),
            1.,
        );
        assert_eq!(
            layout.blocks.iter().map(|line| line.id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            layout
                .blocks
                .iter()
                .filter(|line| line.visible.is_some())
                .count(),
            2
        );
    }

    #[test]
    fn data_table_header_keyboard_focus_routes_to_sorting() {
        let mut model = DataTable::new(0, 2);
        let mut view = DataTableView::new();
        view.render(
            &mut Scene::new(),
            &mut FixedText { height: 14. },
            &model,
            &["Name".into(), "Kind".into()],
            &[],
            &[],
            Rect::new(0., 0., 300., 120.),
            None,
            Theme::LIGHT.into(),
        );
        view.focus_header(0, true);
        assert_eq!(
            view.key(ControlKey::Right, true, &mut model),
            CollectionEvent::Changed
        );
        assert_eq!(
            view.key(ControlKey::Enter, true, &mut model),
            CollectionEvent::Changed
        );
        assert_eq!(
            model.sort,
            Some((1, crate::extended::SortDirection::Ascending))
        );
    }

    #[test]
    fn sorted_data_table_keys_follow_visual_order_without_intermediate_render() {
        let mut model = DataTable::new(5, 1);
        let mut view = DataTableView::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        let rows = (0..5)
            .map(|index| vec![index.to_string()])
            .collect::<Vec<_>>();
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["Name".into()],
            &rows,
            &[3, 2, 1, 0, 4],
            Rect::new(0., 0., 300., 260.),
            None,
            Theme::LIGHT.into(),
        );
        model.select_row(3);
        view.focus_row(3, true);

        for (key, expected) in [
            (ControlKey::Down, 2),
            (ControlKey::Down, 1),
            (ControlKey::Up, 2),
            (ControlKey::Home, 3),
            (ControlKey::End, 4),
        ] {
            assert_eq!(
                view.key(key, true, &mut model),
                CollectionEvent::Changed,
                "{key:?} should move to source row {expected} in visual order"
            );
            assert_eq!(model.selected_row, Some(expected));
            assert_eq!(view.state.focused, Some(expected));
        }
    }

    #[test]
    fn sorted_data_table_keys_traverse_logical_rows_beyond_the_visible_clip() {
        let mut model = DataTable::new(5, 1);
        let mut view = DataTableView::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        let rows = (0..5)
            .map(|index| vec![index.to_string()])
            .collect::<Vec<_>>();
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["Name".into()],
            &rows,
            &[3, 2, 1, 0, 4],
            Rect::new(0., 0., 300., 260.),
            Some(Rect::new(0., 40., 300., 34.)),
            Theme::LIGHT.into(),
        );
        model.select_row(3);
        view.focus_row(3, true);
        assert_eq!(
            view.key(ControlKey::Down, true, &mut model),
            CollectionEvent::Changed
        );
        assert_eq!(model.selected_row, Some(2));
        assert_eq!(view.state.focused, Some(2));
    }

    #[test]
    fn list_hidden_or_disabled_state_does_not_keep_old_capture_or_hover() {
        let mut view = ListView::new();
        let mut model = List::new(1);
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["row".into()],
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let point = Point::new(20., 25.);
        view.pointer_down(point, &mut model);
        view.pointer_move(point);
        view.render(
            &mut scene,
            &mut text,
            &model,
            &["row".into()],
            Rect::new(0., 0., 0., 0.),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(view.pointer_up(point, &mut model), CollectionEvent::Ignored);
        assert!(!view.pointer_move(point));

        view.set_disabled(true);
        assert!(!view.pointer_move(point));
    }

    #[test]
    fn tree_and_table_hidden_or_disabled_views_cancel_old_press() {
        let mut tree = Tree::default();
        tree.add("root", None).unwrap();
        let mut tree_view = TreeView::new();
        let mut text = FixedText { height: 14. };
        let mut scene = Scene::new();
        tree_view.render(
            &mut scene,
            &mut text,
            &tree,
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let tree_point = Point::new(20., 25.);
        assert_eq!(
            tree_view.pointer_down(tree_point, &mut tree),
            CollectionEvent::Changed
        );
        tree_view.set_disabled(true);
        assert_eq!(
            tree_view.pointer_up(tree_point, &mut tree),
            CollectionEvent::Ignored
        );

        let mut table = DataTable::new(1, 1);
        let mut table_view = DataTableView::new();
        table_view.render(
            &mut scene,
            &mut text,
            &table,
            &["Name".into()],
            &[vec!["row".into()]],
            &[0],
            rect(),
            None,
            Theme::LIGHT.into(),
        );
        let row_point = Point::new(20., 62.);
        assert_eq!(
            table_view.pointer_down(row_point, &mut table),
            CollectionEvent::Changed
        );
        table_view.render(
            &mut scene,
            &mut text,
            &table,
            &["Name".into()],
            &[vec!["row".into()]],
            &[0],
            Rect::new(0., 0., 0., 0.),
            None,
            Theme::LIGHT.into(),
        );
        assert_eq!(
            table_view.pointer_up(row_point, &mut table),
            CollectionEvent::Ignored
        );
    }

    #[test]
    fn markdown_measures_bullet_prefix_and_diff_measures_wrapped_prefixed_lines() {
        let mut markdown = Markdown::default();
        markdown.blocks.push(MarkdownBlock::Bullet("item".into()));
        let mut markdown_view = MarkdownView::new();
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        markdown_view.render(
            &mut scene,
            &mut text,
            &markdown,
            Rect::new(0., 0., 80., 120.),
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert!(text.measured.iter().any(|(value, _)| value == "• item"));
        assert_eq!(text.drawn, ["• item"]);
        assert_eq!(markdown_view.layout.blocks[0].frame.min_y(), 4.);
        assert_eq!(markdown_view.layout.blocks[0].frame.min_x(), 0.);
        assert_eq!(markdown_view.layout.blocks[0].frame.size.height, 24.);
        assert_eq!(
            text.draw_rects[0].min_y(),
            markdown_view.layout.blocks[0].frame.min_y()
        );
        assert_eq!(text.draw_rects[0].size.height, 14.);

        markdown
            .blocks
            .push(MarkdownBlock::Paragraph("next".into()));
        markdown_view.render(
            &mut scene,
            &mut text,
            &markdown,
            Rect::new(0., 0., 80., 120.),
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert_eq!(markdown_view.layout.blocks[1].frame.min_y(), 40.);

        let diff = DiffView {
            lines: vec![crate::extended::DiffLine {
                kind: DiffKind::Added,
                text: "a sufficiently long line".into(),
            }],
        };
        let mut diff_view = DiffViewRuntime::new();
        let mut text = RecordingText::default();
        let diff_layout = diff_view.render(
            &mut scene,
            &mut text,
            &diff,
            Rect::new(0., 0., 80., 120.),
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert!(
            text.measured
                .iter()
                .any(|(value, width)| { value == "+ a sufficiently long line" && *width == 64. })
        );
        assert_eq!(text.drawn, ["+ a sufficiently long line"]);
        assert!(diff_layout.blocks[0].frame.size.height >= 72.);
        assert_eq!(diff_layout.blocks[0].frame.min_x(), 8.);
        assert_eq!(diff_layout.blocks[0].frame.size.width, 64.);
    }

    #[test]
    fn code_diff_and_chart_keep_gallery_text_insets_and_alignment() {
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        let mut code_view = CodeBlockView::new();
        let code = CodeBlock {
            language: "rs".into(),
            source: "let value = 1;".into(),
        };
        code_view.render(
            &mut scene,
            &mut text,
            &code,
            Rect::new(0., 0., 680., 100.),
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert_eq!(text.draw_rects[0].min_x(), 14.);
        assert_eq!(text.draw_rects[0].min_y(), 14.);
        assert_eq!(text.draw_rects[0].size.width, 650.);

        let mut diff_view = DiffViewRuntime::new();
        let diff = DiffView::between("same", "same");
        diff_view.render(
            &mut scene,
            &mut text,
            &diff,
            Rect::new(0., 0., 800., 100.),
            None,
            Theme::LIGHT.into(),
            1.,
        );
        assert_eq!(text.draw_rects[1].min_x(), 8.);
        assert_eq!(text.draw_rects[1].size.width, 700.);
        assert_eq!(text.draw_rects[1].size.height, 28.);
        assert_eq!(diff_view.layout.blocks[0].frame.size.height, 28.);

        for (scale, label, value) in [(1., "A", 4.), (2., "Long release label", 123456.)] {
            let mut chart_view = ChartView::new();
            let mut chart = BarChart::new(vec![crate::chart::ChartDatum::new(label, value)]);
            let mut chart_text = RecordingText::default();
            let chart_layout = chart_view.render(
                &mut scene,
                &mut chart_text,
                &mut chart,
                Rect::new(0., 0., 320., 220.),
                None,
                Theme::LIGHT.into(),
                scale,
            );
            let bar = &chart_layout.geometry.bars[0];
            assert_eq!(chart_text.alignments, [ButtonTextAlign::Center; 2]);
            assert_eq!(chart_text.draw_rects[0], bar.value_label);
            assert_eq!(chart_text.draw_rects[1], bar.category_label);
            for slot in [bar.value_label, bar.category_label] {
                assert!(slot.min_y() >= chart_layout.geometry.bounds.min_y());
                assert!(slot.max_y() <= chart_layout.geometry.bounds.max_y());
            }
            assert!(chart_layout.geometry.plot.min_y() > chart_layout.geometry.bounds.min_y());
            for ((origin, ink_width), slot) in chart_text
                .ink_origins
                .iter()
                .zip(&chart_text.ink_widths)
                .zip([bar.value_label, bar.category_label])
            {
                assert!((origin.x + ink_width * 0.5 - slot.center().x).abs() < 0.001);
            }
        }
    }

    #[test]
    fn code_and_diff_wrapped_lines_reserve_both_horizontal_insets() {
        let mut scene = Scene::new();
        let mut text = RecordingText::default();
        let code_frame = Rect::new(20., 10., 120., 240.);
        let mut code_view = CodeBlockView::new();
        let code_layout = code_view.render(
            &mut scene,
            &mut text,
            &CodeBlock {
                language: "rs".into(),
                source: "a deliberately long line that wraps in a narrow code block".into(),
            },
            code_frame,
            None,
            Theme::LIGHT.into(),
            1.5,
        );
        assert!(text.measured.iter().any(|(_, width)| *width == 92.));
        assert_eq!(text.draw_rects[0].min_x(), code_frame.min_x() + 14.);
        assert_eq!(text.draw_rects[0].max_x(), code_frame.max_x() - 14.);
        assert!(code_layout.blocks[0].frame.max_x() <= code_frame.max_x() - 14.);

        text.measured.clear();
        text.draw_rects.clear();
        let diff_frame = Rect::new(30., 5., 100., 240.);
        let mut diff_view = DiffViewRuntime::new();
        let diff_layout = diff_view.render(
            &mut scene,
            &mut text,
            &DiffView::between(
                "a deliberately long existing line which wraps",
                "a deliberately long changed line which wraps",
            ),
            diff_frame,
            None,
            Theme::LIGHT.into(),
            1.5,
        );
        assert!(text.measured.iter().any(|(_, width)| *width == 84.));
        assert!(diff_layout.blocks.iter().all(|line| {
            line.frame.min_x() == diff_frame.min_x() + 8.
                && line.frame.max_x() == diff_frame.max_x() - 8.
        }));
    }

    #[test]
    fn code_block_surface_uses_code_role_and_text_backend_backing_scale() {
        for theme in Theme::ALL {
            for backing_scale in [1.0, 2.0] {
                let palette = Palette::from(theme);
                let frame = Rect::new(14.0, 18.0, 220.0, 80.0);
                let mut text = ScaledText(backing_scale);
                let mut view = CodeBlockView::new();
                let mut scene = Scene::new();
                let layout = view.render(
                    &mut scene,
                    &mut text,
                    &CodeBlock {
                        language: "rs".into(),
                        source: "let value = 1;".into(),
                    },
                    frame,
                    Some(Rect::new(0.0, 0.0, 400.0, 200.0)),
                    palette,
                    1.5,
                );
                assert_eq!(layout.frame, frame);
                let code_quad = scene
                    .primitives()
                    .iter()
                    .find_map(|primitive| match primitive {
                        Primitive::Quad(q) if q.rect == frame => Some(q),
                        _ => None,
                    })
                    .expect("code block surface quad");
                assert_eq!(code_quad.fill, palette.color("code-surface").unwrap());
                assert_eq!(
                    code_quad.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(
                        proposal::metric("radius", "c-code-radius").unwrap()
                    )
                );
                assert_eq!(
                    code_quad.border_width,
                    crate::paint::decorative_border(palette, backing_scale).0
                );
            }
        }
    }

    #[test]
    fn code_block_background_uses_parent_clip_and_text_keeps_frame_clip() {
        let frame = Rect::new(14., 18., 220., 80.);
        for ancestor in [
            Rect::new(-20., -20., 400., 240.),
            Rect::new(60., 30., 140., 50.),
        ] {
            let mut text = ScaledText(2.0);
            let mut view = CodeBlockView::new();
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut text,
                &CodeBlock {
                    language: "rs".into(),
                    source: "let value = 1;".into(),
                },
                frame,
                Some(ancestor),
                Theme::LIGHT.into(),
                1.0,
            );
            let visible = frame.intersection(&ancestor).expect("visible code block");
            assert_eq!(layout.visible, Some(visible));
            let primitives = scene.primitives();
            assert!(matches!(primitives.first(), Some(Primitive::PushClip(r)) if *r == ancestor));
            assert!(matches!(primitives.get(1), Some(Primitive::Quad(q)) if q.rect == frame));
            assert!(matches!(primitives.get(2), Some(Primitive::PopClip)));
            assert!(matches!(primitives.get(3), Some(Primitive::PushClip(r)) if *r == visible));
        }
    }

    #[test]
    fn malformed_and_empty_code_block_bounds_emit_no_surface_or_text() {
        let model = CodeBlock {
            language: "rs".into(),
            source: "line".into(),
        };
        let mut view = CodeBlockView::new();
        let mut text = RecordingText::default();
        let mut scene = Scene::new();
        for frame in [
            Rect::new(0., 0., 0., 40.),
            Rect::new(f32::NAN, 0., 40., 40.),
            Rect::new(f32::MAX, 0., f32::MAX, 40.),
        ] {
            scene.clear();
            text.drawn.clear();
            view.render(
                &mut scene,
                &mut text,
                &model,
                frame,
                None,
                Theme::LIGHT.into(),
                1.,
            );
            assert!(scene.is_empty());
            assert!(text.drawn.is_empty());
        }
    }
}
