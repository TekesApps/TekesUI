#![allow(clippy::too_many_arguments)]
use super::{
    CollectionAction, CollectionEvent, CollectionInteraction, CollectionItemLayout,
    CollectionLayout, ListActionSnapshot, ListInput, control_state, mounted_bounds, own_clip,
    paint_collection_focus, row_gap, row_height, row_height_for, row_key_target, visible,
};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, extended::List,
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

/// Runtime for the existing single-selection [`List`] model.
/// Runtime for the existing single-selection [`List`] model.
#[derive(Clone, Debug)]
struct ListViewIdentity {
    instance: std::sync::Arc<()>,
    revision: u64,
    arranged: bool,
    items: Vec<(crate::extended::ListItemKey, u64)>,
}
#[derive(Clone, Debug)]
pub struct ListView {
    pub(super) layout: CollectionLayout,
    pub(super) state: CollectionInteraction,
    pub(super) disabled: bool,
    pub(super) compact: bool,
    pub(super) navigation_selection_fill_only: bool,
    pub(super) navigation_layout: bool,
    pub(super) buttons: Vec<ButtonModel>,
    keyed: Option<ListViewIdentity>,
    pub(super) row_placements: Vec<crate::ButtonLayout>,
    pub(super) row_custom: Vec<bool>,
    pub(super) list_ancestor_clip: Option<Rect>,
    pub(super) unavailable_layout: CollectionLayout,
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
            compact: false,
            navigation_selection_fill_only: false,
            navigation_layout: false,
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
    /// Opt a navigation list into a filled selection without an enclosing
    /// row stroke. The default List appearance and independent keyboard focus
    /// remain unchanged; this does not change row state, layout or hit targets.
    pub fn set_navigation_selection_fill_only(&mut self, fill_only: bool) {
        self.navigation_selection_fill_only = fill_only;
    }
    /// Use the existing Gallery sidebar row-height and gap tokens explicitly.
    /// Ordinary List metrics remain unchanged until this navigation opt-in.
    pub fn set_navigation_layout(&mut self, navigation: bool) {
        if self.navigation_layout != navigation {
            self.cancel();
            self.navigation_layout = navigation;
        }
    }
    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
    pub fn set_compact(&mut self, compact: bool) {
        if self.compact != compact {
            self.cancel();
            self.compact = compact;
        }
    }
    pub fn compact(&self) -> bool {
        self.compact
    }
    fn metrics(&self) -> (f32, f32) {
        if self.navigation_layout {
            (
                tekes_ui_tokens::ControlSize::RowHeight.value(),
                tekes_ui_tokens::Spacing::Micro.value(),
            )
        } else {
            (row_height_for(self.compact), row_gap())
        }
    }
    pub fn measure_rows(
        &self,
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        width: f32,
    ) -> tekes_ui_scene::Size {
        let (height, gap) = self.metrics();
        Self::measure_content_rows_with_metrics(
            text,
            labels,
            &(0..labels.len()).collect::<Vec<_>>(),
            width,
            &[],
            height,
            gap,
        )
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
        Self::measure_content_rows_with_metrics(
            text,
            labels,
            ids,
            width,
            sizes,
            row_height(),
            row_gap(),
        )
    }
    /// Measure keyed rows with the same configured metrics as arrangement.
    pub(crate) fn measure_current_content_rows(
        &self,
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        ids: &[usize],
        width: f32,
        sizes: &[Option<tekes_ui_scene::Size>],
    ) -> tekes_ui_scene::Size {
        let (height, gap) = self.metrics();
        Self::measure_content_rows_with_metrics(text, labels, ids, width, sizes, height, gap)
    }
    fn measure_content_rows_with_metrics(
        text: &mut impl ButtonTextRenderer,
        labels: &[String],
        ids: &[usize],
        width: f32,
        sizes: &[Option<tekes_ui_scene::Size>],
        minimum: f32,
        gap: f32,
    ) -> tekes_ui_scene::Size {
        let h = ids
            .iter()
            .map(|i| {
                Self::row_size(text, &labels[*i], sizes.get(*i).copied().flatten(), width)
                    .height
                    .max(minimum)
            })
            .sum::<f32>()
            + gap * ids.len().saturating_sub(1) as f32;
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
        let (minimum, gap) = self.metrics();
        let rh = minimum
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
            content_height: heights.iter().sum::<f32>() + gap * ids.len().saturating_sub(1) as f32,
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
            y += heights[ordinal] + gap;
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
            button.set_selection_fill_only(self.navigation_selection_fill_only);
            button.state = control_state(
                model.selected == Some(row.id),
                self.state.hovered == Some(row.id),
                self.state.focused == Some(row.id) && self.state.window_active,
                self.disabled,
                self.state.captured == Some(row.id),
            );
            // A List row is content on its parent, not a separate Quiet-button
            // card. Use the same Surface states for default and declared hosts.
            button.set_surface(surface.unwrap_or("canvas"));
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
                // List backgrounds belong only to selection and current hover.
                // Exit motion may continue for ink, but must not paint old rows.
                !button.state.selected && !button.state.hovered,
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
        if key == ControlKey::Enter {
            self.state.focus_visible = true;
            model.select(id);
            return CollectionEvent::Activated(id);
        }
        if !matches!(
            key,
            ControlKey::Up | ControlKey::Down | ControlKey::Home | ControlKey::End
        ) {
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
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
