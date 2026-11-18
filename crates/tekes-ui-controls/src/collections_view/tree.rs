#![allow(clippy::too_many_arguments)]
use super::{
    CollectionEvent, CollectionInteraction, CollectionItemLayout, CollectionLayout, control_state,
    mounted_bounds, own_clip, paint_collection_focus, render_quiet_button, row_gap, row_key_target,
};
use crate::{
    ButtonModel, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::Tree,
    layout_view::{IconRenderer, IconView},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Palette, proposal};

/// Runtime for the existing hierarchical [`Tree`] model.
#[derive(Clone, Debug)]
pub struct TreeView {
    pub(super) layout: CollectionLayout,
    pub(super) rows: Vec<TreeContentRow>,
    pub(super) unavailable_layout: CollectionLayout,
    pub(super) accepted_identity: Option<std::sync::Arc<()>>,
    pub(super) accepted_revision: Option<u64>,
    pub(super) layout_revision: Option<u64>,
    accepted_nodes: Vec<TreeViewIdentity>,
    pub(super) keyed: bool,
    pub(super) state: CollectionInteraction,
    pub(super) disabled: bool,
    pub(super) buttons: Vec<ButtonModel>,
    pub(super) icons: Vec<IconView>,
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
        if key == ControlKey::Enter {
            self.state.focus_visible = true;
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
            return CollectionEvent::Ignored;
        }
        self.state.focus_visible = true;
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
