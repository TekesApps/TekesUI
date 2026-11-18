#![allow(clippy::too_many_arguments)]
use super::{
    CollectionAction, CollectionEvent, CollectionInteraction, CollectionItemLayout,
    TableActionSnapshot, control_state, mounted_bounds, own_clip, paint_collection_focus,
    render_quiet_button, row_gap, row_key_target, visible,
};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, extended::DataTable,
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

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
    pub(super) layout: DataTableLayout,
    pub(super) state: CollectionInteraction,
    pub(super) disabled: bool,
    pub(super) header_buttons: Vec<ButtonModel>,
    pub(super) row_buttons: Vec<ButtonModel>,
    pub(super) focused_header: Option<usize>,
    keyed: Option<TableViewIdentity>,
    pub(super) unavailable_layout: DataTableLayout,
    pub(super) content_metrics: Option<TableContentMetrics>,
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
                .filter(|s| *s != "base");
            let button = &mut self.row_buttons[id];
            if let Some(surface) = surface {
                button.set_surface(surface);
            } else {
                button.clear_surface();
            }
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
                        Self::row_foreground(button, palette, surface.unwrap_or("canvas")).0,
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
        if let Some(column) = self.focused_header {
            if key == ControlKey::Enter {
                self.state.focus_visible = true;
                return if model.sort_by(column) {
                    CollectionEvent::Changed
                } else {
                    CollectionEvent::Handled
                };
            }
            if matches!(key, ControlKey::Left | ControlKey::Right) && model.columns > 0 {
                self.state.focus_visible = true;
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
            return CollectionEvent::Ignored;
        }
        if key == ControlKey::Enter {
            let Some(id) = self.state.focused else {
                return CollectionEvent::Ignored;
            };
            if !self.layout.rows.iter().any(|row| row.id == id) {
                return CollectionEvent::Ignored;
            }
            self.state.focus_visible = true;
            return if model.select_row(id) {
                CollectionEvent::Changed
            } else {
                CollectionEvent::Activated(id)
            };
        }
        if !matches!(
            key,
            ControlKey::Home | ControlKey::End | ControlKey::Up | ControlKey::Down
        ) {
            return CollectionEvent::Ignored;
        }
        if self.layout.rows.is_empty() {
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
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
    // Row labels are readonly information, so disabling row actions does not
    // dim their text or icon with the disabled operation-label role.
    fn row_foreground(
        button: &ButtonModel,
        palette: Palette,
        surface: &str,
    ) -> (tekes_ui_tokens::Rgba, tekes_ui_tokens::Rgba) {
        if button.state.disabled {
            let ink = palette
                .color("primary-text")
                .expect("DS primary text token");
            return (ink, ink);
        }
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
