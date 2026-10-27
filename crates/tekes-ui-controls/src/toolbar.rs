//! Module-declared single-row toolbar. Measurement, overflow and menu input
//! belong to this control; window hosts only provide its allocated frame.
use crate::composite::MenuRowStyle;
use crate::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, MenuEvent, MenuView,
};
use tekes_ui_scene::{ImageId, Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

/// One internal geometry calculation for both original Toolbar entry points.
/// JSON visible-ink proportions scaled only by effective title size.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ToolbarMetrics {
    pub row: f32,
    pub box_size: f32,
    pub inset: f32,
}
pub(crate) fn metrics(
    text: &mut (impl ButtonTextRenderer + ?Sized),
    role: proposal::TextRole,
) -> ToolbarMetrics {
    let size = text.resolved_font_size(role);
    let ratio = if size.is_finite() && size > 0. {
        size / proposal::TOOLBAR_REFERENCE_TITLE_SIZE
    } else {
        1.
    };
    let metric =
        |group, name| proposal::metric(group, name).expect("toolbar JSON geometry") * ratio;
    let icon = metric("size", "size-icon-standard");
    let ink = icon * proposal::REGULAR_ICON_VISIBLE_HEIGHT_RATIO;
    ToolbarMetrics {
        row: ink * proposal::TOOLBAR_ROW_TO_VISIBLE_ICON,
        box_size: ink * proposal::TOOLBAR_BOX_TO_VISIBLE_ICON,
        inset: ink * proposal::TOOLBAR_INSET_TO_VISIBLE_ICON,

    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarAction {
    pub id: u64,
    pub title: String,
    pub enabled: bool,
}
/// Resolved toolkit icon artwork; labels also name overflow actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarIconAction {
    pub id: u64,
    pub image: ImageId,
    pub label: String,
    pub enabled: bool,
}
#[derive(Debug)]
pub struct ToolbarControl {
    keep_group_together: bool,
    embedded: bool,
    actions: Vec<ToolbarAction>,
    semantic_ids: std::collections::HashMap<u64, usize>,
    next_semantic_id: usize,
    popup: Option<crate::MenuLayout>,
    text_role: proposal::TextRole,
    icons: std::collections::HashMap<u64, ImageId>,
    selected: std::collections::HashSet<u64>,
    action_help: std::collections::HashMap<u64, (String, u64)>,
    help_revision: u64,
    buttons: Vec<crate::composition::Button>,
    visible: Vec<usize>,
    hidden: Vec<usize>,
    trigger: crate::composition::Button,
    overflow_icon: Option<ImageId>,
    viewport: Rect,
    held: Option<usize>,
    focused: Option<usize>,
    has_focus: bool,
    focus_visible: bool,
    last_action: Option<u64>,
    trigger_rect: Rect,
    menu: MenuView,
    frame: Rect,
    clip: Rect,
}
impl Default for ToolbarControl {
    fn default() -> Self {
        Self {
            keep_group_together: false,
            embedded: false,
            actions: Vec::new(),
            semantic_ids: Default::default(),
            next_semantic_id: 1,
            popup: None,
            text_role: proposal::TextRole::UiBody,
            icons: Default::default(),
            selected: Default::default(),
            action_help: Default::default(),
            help_revision: 0,
            buttons: Vec::new(),
            visible: Vec::new(),
            hidden: Vec::new(),
            trigger: crate::composition::Button::new("…"),
            overflow_icon: None,
            viewport: Rect::default(),
            held: None,
            focused: None,
            has_focus: false,
            focus_visible: false,
            last_action: None,
            trigger_rect: Rect::default(),
            menu: MenuView::new(0),
            frame: Rect::default(),
            clip: Rect::default(),
        }
    }
}
impl ToolbarControl {
    pub(crate) fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }
    fn edge_inset(&self, m: ToolbarMetrics) -> f32 {
        if self.embedded { 0. } else { m.inset }
    }
    /// Native-window contribution styling is owned by this shared control and
    /// JSON Design System. Inline toolbar defaults remain unchanged.
    pub fn window() -> Self {
        Self {
            text_role: proposal::TextRole::UiBody,
            ..Self::default()
        }
    }
    /// Borrow a host-resolved canonical More image. Small ink stays centered
    /// in the same standard icon slot; resource ownership remains with caller.
    pub fn set_overflow_icon(&mut self, image: Option<ImageId>) {
        if self.overflow_icon != image {
            self.cancel();
            self.overflow_icon = image;
        }
    }
    fn overflow_size(&self, text: &mut impl ButtonTextRenderer) -> Size {
        let m = metrics(text, self.text_role);
        Size::new(m.box_size, m.box_size)
    }
    fn item_size(&self, text: &mut impl ButtonTextRenderer, action: &ToolbarAction) -> Size {
        let m = metrics(text, self.text_role);
        let width = if self.icons.contains_key(&action.id) {
            m.box_size
        } else {
            text.measure(&action.title, self.text_role, f32::MAX).width + m.inset * 2.
        };
        Size::new(width, m.box_size)
    }
    /// Explain an existing action through the original ContentHost tooltip.
    /// Empty/None revokes help; this never changes command, focus or selection.
    /// Returns false for unknown/ambiguous IDs or an unchanged declaration.
    pub fn set_action_help(&mut self, id: u64, text: Option<String>) -> bool {
        if self.actions.iter().filter(|a| a.id == id).count() != 1 {
            return false;
        }
        let text = text.filter(|s| !s.trim().is_empty());
        if self.action_help.get(&id).map(|(text, _)| text) == text.as_ref() {
            return false;
        }
        self.help_revision = self
            .help_revision
            .checked_add(1)
            .expect("toolbar help revision exhausted");
        match text {
            Some(text) => {
                self.action_help.insert(id, (text, self.help_revision));
            }
            None => {
                self.action_help.remove(&id);
            }
        }
        true
    }
    pub fn set_selected_ids(&mut self, ids: &[u64]) {
        self.selected = ids.iter().copied().collect();
    }
    pub fn set_icon_actions(&mut self, actions: Vec<ToolbarIconAction>) {
        self.icons = actions.iter().map(|a| (a.id, a.image)).collect();
        self.update_actions(
            actions
                .into_iter()
                .map(|a| ToolbarAction {
                    id: a.id,
                    title: a.label,
                    enabled: a.enabled,
                })
                .collect(),
        );
    }
    /// Resolve one declared icon through the host's existing image capability;
    /// other actions remain text. Geometry changes cancel an old input arm.
    pub fn set_action_icon(&mut self, id: u64, image: Option<ImageId>) -> bool {
        if !self.actions.iter().any(|action| action.id == id)
            || self.icons.get(&id).copied() == image
        {
            return false;
        }
        self.cancel();
        if let Some(image) = image {
            self.icons.insert(id, image);
        } else {
            self.icons.remove(&id);
        }
        true
    }
    /// Keep a logical command group in one presentation: all buttons or one
    /// original More menu. The same menu/action identities are retained.
    pub fn set_group_together(&mut self, together: bool) {
        if self.keep_group_together != together {
            self.cancel();
            self.keep_group_together = together;
        }
    }
    pub(crate) fn minimum_width(&self, text: &mut dyn ButtonTextRenderer) -> f32 {
        if self.actions.is_empty() { return 0.; }
        {
            let m = metrics(text, self.text_role);
            self.overflow_size(&mut ToolbarText(text)).width + self.edge_inset(m) * 2.
        }
    }
    pub fn set_actions(&mut self, actions: Vec<ToolbarAction>) {
        self.icons.clear();
        self.update_actions(actions);
    }
    fn update_actions(&mut self, mut actions: Vec<ToolbarAction>) {
        // A label never introduces a second toolbar row. Popup labels reuse
        // these same normalized action names.
        for action in &mut actions {
            action.title = action
                .title
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
        if self.actions != actions {
            let focused_id = self.focused.and_then(|i| self.actions.get(i)).map(|a| a.id);
            let overflow_focused = self.focused == Some(self.actions.len());
            let same_ids = self.actions.len() == actions.len()
                && self
                    .actions
                    .iter()
                    .zip(&actions)
                    .all(|(old, new)| old.id == new.id);
            self.cancel();
            self.buttons
                .resize_with(actions.len(), || crate::composition::Button::new(""));
            // Keep logical action identity across reorder/label updates. A
            // removed and later reintroduced action receives a fresh part key.
            self.semantic_ids
                .retain(|id, _| actions.iter().any(|a| a.id == *id));
            self.action_help
                .retain(|id, _| actions.iter().any(|a| a.id == *id));
            for action in &actions {
                if !self.semantic_ids.contains_key(&action.id) {
                    let part = self.next_semantic_id;
                    self.next_semantic_id = part
                        .checked_add(1)
                        .expect("toolbar semantic identity exhausted");
                    self.semantic_ids.insert(action.id, part);
                }
            }
            self.actions = actions;
            self.focused = focused_id
                .and_then(|id| self.actions.iter().position(|a| a.id == id))
                .or_else(|| overflow_focused.then_some(self.actions.len()));
            for (button, action) in self.buttons.iter_mut().zip(&self.actions) {
                button.button.state.disabled = !action.enabled;
            }
            if same_ids {
                crate::composition::Component::set_focus(self, self.has_focus, self.focus_visible);
            } else {
                self.visible.clear();
                self.hidden.clear();
                self.trigger_rect = Rect::default();
                self.menu.replace_items(0);
                for button in &mut self.buttons {
                    button.button.clear_layout();
                }
                self.trigger.button.clear_layout();
            }
        }
    }
    /// Typography-derived chrome height, including an empty window toolbar.
    /// Window adapters use this same value for native controls and content.
    pub fn row_height(&self, text: &mut impl ButtonTextRenderer) -> f32 {
        metrics(text, self.text_role).row
    }
    pub fn measure(&self, text: &mut impl ButtonTextRenderer) -> Size {
        if self.actions.is_empty() {
            return Size::default();
        }
        let m = metrics(text, self.text_role);
        let pad = m.inset;
        let mut size = self.overflow_size(text);
        size.width = 0.;
        for action in &self.actions {
            let item = self.item_size(text, action);
            size.width += item.width;
            size.height = size.height.max(item.height);
        }
        Size::new(size.width + pad * self.actions.len().saturating_sub(1) as f32 + self.edge_inset(m) * 2., m.row)
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
    ) {
        self.render_clipped(scene, text, frame, palette, None)
    }
    /// Preserve full row geometry while clipping both painting and hit targets
    /// to the containing component's visible area.
    pub fn render_clipped(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        frame: Rect,
        palette: Palette,
        clip: Option<Rect>,
    ) {
        let m = metrics(text, self.text_role);
        let allocated = frame;
        let frame = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, m.row);
        let clip = clip
            .or(Some(allocated))
            .and_then(|clip| frame.intersection(&clip))
            .unwrap_or_else(|| {
                if clip.is_some() {
                    Rect::default()
                } else {
                    frame
                }
            });
        if !self.embedded {
            crate::paint::toolbar_background(scene, allocated, Some(clip), m.row, palette);
        }
        let pad = m.inset;
        let edge = self.edge_inset(m);
        let inner = Rect::new(
            frame.min_x() + edge,
            frame.min_y(),
            (frame.size.width - edge * 2.).max(0.),
            frame.size.height,
        );
        let sizes: Vec<_> = self
            .actions
            .iter()
            .map(|a| self.item_size(text, a))
            .collect();
        // Match measure() in outer-frame space. Subtracting the insets first
        // loses low bits under typography zoom and can invent overflow even
        // at the exact natural width returned to a host.
        let natural_width =
            sizes.iter().map(|s| s.width).sum::<f32>() + pad * sizes.len().saturating_sub(1) as f32 + edge * 2.;
        let needs_overflow = natural_width > frame.size.width;
        let mut overflow = self.overflow_size(text);
        overflow.height = overflow
            .height
            .max(sizes.iter().map(|s| s.height).fold(0., f32::max));
        let mut visible = Vec::new();
        let mut used = 0.;
        for (i, s) in sizes.iter().enumerate() {
            if needs_overflow && self.keep_group_together { break; }
            let next = used + s.width;
            let with_overflow = next + overflow.width + pad * (visible.len() + 1) as f32 + edge * 2.;
            if needs_overflow && with_overflow > frame.size.width {
                break;
            }
            visible.push(i);
            used = next;
        }
        let hidden: Vec<_> = (visible.len()..self.actions.len()).collect();
        if self.frame != frame || self.clip != clip || self.hidden != hidden {
            self.cancel();
        }
        if self.hidden != hidden {
            self.menu.replace_items(hidden.len());
        }
        self.frame = frame;
        self.clip = clip;
        self.visible = visible;
        self.hidden = hidden;
        if self.has_focus {
            crate::composition::Component::set_focus(self, true, self.focus_visible);
        }
        let styles = self
            .hidden
            .iter()
            .map(|&i| MenuRowStyle {
                disabled: !self.actions[i].enabled,
                ..Default::default()
            })
            .collect();
        self.menu.set_row_styles(styles);
        let mut x = inner.min_x();
        for &i in &self.visible {
            let size = Size::new(
                sizes[i].width,
                sizes.iter().map(|s| s.height).fold(0., f32::max),
            );
            let rect = Rect::new(
                x,
                frame.min_y() + (frame.size.height - size.height) * 0.5,
                size.width,
                size.height,
            );
            self.buttons[i].set_toolbar_presentation(true);
            self.buttons[i].button.frame_policy = crate::ButtonFramePolicy::Exact;
            self.buttons[i].button.chrome_height = None;
            self.buttons[i].button.state.disabled = !self.actions[i].enabled;
            self.buttons[i].button.state.selected = self.selected.contains(&self.actions[i].id);
            let content = match self.icons.get(&self.actions[i].id) {
                Some(&image) => crate::composition::ButtonContents::Image(image),
                None => crate::composition::ButtonContents::Text(self.actions[i].title.clone()),
            };
            self.buttons[i].set_contents(content);
            self.buttons[i].set_accessible_name(Some(self.actions[i].title.clone()));
            crate::composition::Component::arrange(&mut self.buttons[i], text, rect, clip);
            crate::composition::Component::paint(&mut self.buttons[i], scene, text, rect, clip, palette.into());
            x += size.width + pad;
        }
        self.trigger_rect = Rect::default();
        if !self.hidden.is_empty() && inner.size.width > 0. {
            self.trigger_rect = Rect::new(
                (inner.max_x() - overflow.width).max(inner.min_x()),
                frame.min_y() + (frame.size.height - overflow.height) * 0.5,
                overflow.width.min(inner.size.width),
                overflow.height,
            );
            self.trigger.set_toolbar_presentation(true);
            self.trigger.button.frame_policy = crate::ButtonFramePolicy::Exact;
            self.trigger.button.chrome_height = None;
            self.trigger.set_contents(match self.overflow_icon {
                Some(image) => crate::composition::ButtonContents::Image(image),
                None => crate::composition::ButtonContents::Text("…".into()),
            });
            self.trigger.set_accessible_name(Some("More actions".into()));
            crate::composition::Component::arrange(&mut self.trigger, text, self.trigger_rect, clip);
            crate::composition::Component::paint(&mut self.trigger, scene, text, self.trigger_rect, clip, palette.into());
        }
    }
    /// Called after all ordinary content so the popup can overlay sibling panes.
    pub fn render_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut impl ButtonTextRenderer,
        viewport: Rect,
        palette: Palette,
    ) {
        let labels: Vec<_> = self
            .hidden
            .iter()
            .map(|&i| self.actions[i].title.as_str())
            .collect();
        self.popup = self.menu.render(
            scene,
            text,
            &labels,
            self.trigger_rect,
            viewport,
            None,
            palette,
            proposal::TextRole::UiBody,
            false,
        );
    }
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }
    fn focus_stops(&self) -> Vec<usize> {
        let mut stops: Vec<_> = self
            .visible
            .iter()
            .copied()
            .filter(|&i| self.actions.get(i).is_some_and(|action| action.enabled))
            .collect();
        if !self.hidden.is_empty() && !self.trigger_rect.is_empty() {
            stops.push(self.actions.len());
        }
        stops
    }
    pub fn take_action(&mut self) -> Option<u64> {
        self.last_action.take()
    }
    pub fn is_open(&self) -> bool {
        self.menu.is_open()
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        if self.menu.is_open() {
            self.menu.pointer_down_at(p);
            return true;
        }
        if !self.hidden.is_empty() && self.trigger.button.pointer_down_at(p) {
            return true;
        }
        self.visible
            .iter()
            .any(|&i| self.buttons[i].button.pointer_down_at(p))
    }
    pub fn pointer_move(&mut self, p: Point) {
        if self.menu.is_open() {
            self.menu.pointer_move_at(p);
        }
        self.trigger.button.pointer_move_at(p);
        for b in &mut self.buttons {
            b.button.pointer_move_at(p);
        }
    }
    pub fn pointer_up(&mut self, p: Point) -> Option<u64> {
        if self.menu.is_open() {
            let event = self.menu.pointer_up_at(p);
            return self.action_for(event);
        }
        if !self.hidden.is_empty() && self.trigger.button.pointer_up_at(p) {
            self.menu.show(0);
            return None;
        }
        let mut action = None;
        for &i in &self.visible {
            if self.buttons[i].button.pointer_up_at(p) && self.actions[i].enabled {
                action = Some(self.actions[i].id);
            }
        }
        action
    }
    fn action_for(&self, event: MenuEvent) -> Option<u64> {
        if let MenuEvent::Activated { index, .. } = event {
            let action = self.actions.get(*self.hidden.get(index)?)?;
            return action.enabled.then_some(action.id);
        }
        None
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> Option<u64> {
        let event = self.menu.key(key, down);
        self.action_for(event)
    }
    pub fn cancel(&mut self) {
        self.cancel_interaction();
        self.last_action = None;
    }
    fn cancel_interaction(&mut self) {
        self.popup = None;
        for b in &mut self.buttons {
            b.button.cancel();
        }
        self.trigger.button.cancel();
        self.held = None;
        self.menu.key(ControlKey::Escape, true);
        self.menu.clear_presentation();
    }
    pub fn overflow_ids(&self) -> Vec<u64> {
        self.hidden.iter().map(|&i| self.actions[i].id).collect()
    }
}

// The composition wrapper keeps the real Button input contract,
// including explicit drag propagation, modifiers and primary-button ownership.
impl crate::composition::Component for ToolbarControl {
    fn validate(&self) -> Result<(), crate::composition::CompositionError> {
        let mut ids = std::collections::BTreeSet::new();
        for action in &self.actions {
            if !ids.insert(action.id) { return Err(crate::composition::CompositionError::DuplicateId(crate::composition::ComponentId(action.id))); }
            if action.title.trim().is_empty() { return Err(crate::composition::CompositionError::EmptyAccessibleName); }
        }
        Ok(())
    }
    fn action_payload(&self, kind: crate::composition::ContentActionKind) -> Option<crate::composition::ContentActionPayload> {
        (kind == crate::composition::ContentActionKind::Activate)
            .then_some(self.last_action).flatten()
            .map(crate::composition::ContentActionPayload::ToolbarCommand)
    }
    fn pointer_regions(&self, _: Rect, clip: Rect) -> Vec<Rect> {
        self.visible
            .iter()
            .filter_map(|&i| self.buttons[i].button.layout())
            .chain(
                (!self.hidden.is_empty())
                    .then(|| self.trigger.button.layout())
                    .flatten(),
            )
            .filter_map(|l| l.visible.and_then(|r| r.intersection(&clip)))
            .filter(|r| !r.is_empty())
            .collect()
    }
    fn measure(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> crate::layout::MeasuredChild {
        let mut size = ToolbarControl::measure(self, &mut ToolbarText(text));
        // A toolbar owns overflow within its allocation; it must not ask a
        // measured parent to preserve the width of all hidden commands.
        size.width = size.width.min(available.width.max(0.));
        crate::layout::MeasuredChild::new(size)
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: crate::composition::ContentTheme,
    ) {
        self.render_clipped(
            scene,
            &mut ToolbarText(text),
            frame,
            theme.palette,
            Some(clip),
        );
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: crate::composition::ContentTheme,
    ) {
        self.render_overlay(scene, &mut ToolbarText(text), self.viewport, theme.palette);
    }
    fn overlay_open(&self) -> bool {
        self.is_open()
    }
    fn overlay_scope(&self) -> Option<crate::composition::OverlayScope> {
        // Closed toolbars stay ordinary active Content. Only the open original
        // menu acquires the shared input/Tab/AX barrier.
        self.is_open().then_some(Default::default())
    }
    fn overlay_event(
        &mut self,
        event: &crate::composition::ContentEvent,
    ) -> crate::composition::ContentResponse {
        self.event(event)
    }
    fn lifecycle(&mut self, event: crate::composition::Lifecycle) {
        if event == crate::composition::Lifecycle::Unmounted {
            self.action_help.clear();
            self.help_revision = self
                .help_revision
                .checked_add(1)
                .expect("toolbar help revision exhausted");
            self.cancel();
        }
    }
    fn focusable(&self) -> bool {
        self.actions.iter().any(|a| a.enabled)
            || !self.hidden.is_empty()
                && self
                    .trigger
                    .button
                    .layout()
                    .is_some_and(|layout| layout.visible.is_some())
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let stops = self.focus_stops();
        let next = if focused && !self.focused.is_some_and(|i| stops.contains(&i)) {
            stops.first().copied()
        } else {
            self.focused
        };
        if (!focused && self.has_focus) || next != self.focused {
            self.cancel_interaction();
        }
        self.focused = next;
        self.has_focus = focused;
        self.focus_visible = visible;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            button.set_focus(focused && self.focused == Some(i), visible);
        }
        self.trigger
            .set_focus(focused && self.focused == Some(self.actions.len()), visible);
    }
    fn set_focus_from_traversal(&mut self, reverse: bool) {
        self.cancel();
        let stops = self.focus_stops();
        self.focused = if reverse { stops.last() } else { stops.first() }.copied();
        self.set_focus(true, true);
    }
    fn key_event(
        &mut self,
        key: &crate::composition::ContentKeyEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::ContentResponse;
        self.last_action = None;
        if self.is_open() {
            if let Some(k) = key.key {
                self.last_action = self.key(k, key.down);
            }
            return ContentResponse {
                consumed: true,
                activated: self.last_action.is_some(),
                changed: true,
                ..Default::default()
            };
        }
        if !self.has_focus || self.visible.is_empty() && self.hidden.is_empty() {
            return ContentResponse::default();
        }
        if key.down
            && !key.modifiers.shift
            && !key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.command
            && matches!(
                key.key,
                Some(ControlKey::Left | ControlKey::Right | ControlKey::Home | ControlKey::End)
            )
        {
            let stops = self.focus_stops();
            if let Some(index) = self
                .focused
                .and_then(|i| stops.iter().position(|&s| s == i))
            {
                let target = match key.key {
                    Some(ControlKey::Home) => 0,
                    Some(ControlKey::End) => stops.len() - 1,
                    Some(ControlKey::Left) => index.saturating_sub(1),
                    _ => (index + 1).min(stops.len() - 1),
                };
                self.cancel();
                self.focused = Some(stops[target]);
                self.set_focus(true, true);
                return ContentResponse::consumed(true);
            }
        }
        let Some(i) = self.focused else {
            return ContentResponse::default();
        };
        let result = if i == self.actions.len() {
            self.trigger.key_event(key)
        } else if let Some(button) = self.buttons.get_mut(i) {
            button.key_event(key)
        } else {
            return ContentResponse::default();
        };
        if result.activated {
            if i == self.actions.len() {
                self.menu.show(0);
            } else if self.actions[i].enabled {
                self.last_action = Some(self.actions[i].id);
            }
        }
        ContentResponse {
            activated: self.last_action.is_some(),
            ..result
        }
    }
    fn overlay_key_event(
        &mut self,
        key: &crate::composition::ContentKeyEvent,
    ) -> crate::composition::ContentResponse {
        self.key_event(key)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn semantics(&self) -> crate::composition::ContentSemantics {
        crate::composition::ContentSemantics::new(crate::composition::ContentRole::Group, "Toolbar")
    }
    fn semantic_part_help(&self, part: usize) -> Option<(&str, u64)> {
        let id = self.semantic_ids.iter().find(|(_, key)| **key == part)?.0;
        if self.actions.iter().filter(|a| a.id == *id).count() != 1 {
            return None;
        }
        let (text, revision) = self.action_help.get(id)?;
        Some((text.as_str(), *revision))
    }
    fn semantic_parts(&self) -> Vec<crate::composition::SemanticPart> {
        use crate::composition::{ContentRole, ContentSemantics, SemanticPart};
        let mut parts = Vec::new();
        let open = self.menu.is_open();
        let candidates = if open { &self.hidden } else { &self.visible };
        for (slot, &i) in candidates.iter().enumerate() {
            let action = &self.actions[i];
            // Duplicate command IDs have no unambiguous public action owner.
            if self.actions.iter().filter(|a| a.id == action.id).count() != 1 {
                continue;
            }
            let layout = if open {
                self.popup.as_ref().and_then(|p| p.items.get(slot)).copied()
            } else {
                self.buttons[i].button.layout()
            };
            let Some(frame) = layout.and_then(|l| l.visible).filter(|r| !r.is_empty()) else {
                continue;
            };
            let mut semantics = ContentSemantics::new(
                if open {
                    ContentRole::MenuItem
                } else {
                    ContentRole::Button
                },
                &action.title,
            );
            semantics.disabled = !action.enabled;
            semantics.selected = self.selected.contains(&action.id);
            semantics.description = self
                .action_help
                .get(&action.id)
                .map(|(text, _)| text.clone());
            parts.push(SemanticPart {
                index: self.semantic_ids[&action.id],
                frame,
                focused: self.has_focus
                    && action.enabled
                    && if open {
                        self.popup.as_ref().is_some_and(|p| p.active_item == slot)
                    } else {
                        self.focused == Some(i)
                    },
                semantics,
            });
        }
        if !self.hidden.is_empty()
            && let Some(frame) = self
                .trigger
                .button
                .layout()
                .and_then(|l| l.visible)
                .filter(|r| !r.is_empty())
        {
            let mut semantics = ContentSemantics::new(ContentRole::MenuButton, "More options");
            semantics.expanded = Some(open);
            parts.push(SemanticPart {
                index: 0,
                frame,
                focused: !open && self.has_focus && self.focused == Some(self.actions.len()),
                semantics,
            });
        }
        parts
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        (part != 0
            && self.menu.is_open()
            && self
                .hidden
                .iter()
                .any(|&i| self.semantic_ids[&self.actions[i].id] == part))
        .then_some(0)
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &crate::composition::AccessibilityAction,
    ) -> crate::composition::ContentResponse {
        use crate::composition::{AccessibilityAction, ContentResponse};
        self.last_action = None;
        let Some(part) = part else {
            return ContentResponse::default();
        };
        if !matches!(
            action,
            AccessibilityAction::Focus | AccessibilityAction::Press
        ) || !self
            .semantic_parts()
            .iter()
            .any(|p| p.index == part && !p.semantics.disabled)
        {
            return ContentResponse::default();
        }
        if part == 0 && self.menu.is_open() {
            self.cancel();
            self.focused = Some(self.actions.len());
            let focus = matches!(action, AccessibilityAction::Focus);
            self.set_focus(self.has_focus || focus, self.focus_visible || focus);
            return ContentResponse::consumed(true);
        }
        if self.menu.is_open() {
            let Some(slot) = self
                .hidden
                .iter()
                .position(|&i| self.semantic_ids[&self.actions[i].id] == part)
            else {
                return ContentResponse::default();
            };
            if matches!(action, AccessibilityAction::Focus) {
                if !self.menu.focus_item(slot) {
                    return ContentResponse::default();
                }
                self.focused = Some(self.actions.len());
                self.set_focus(true, true);
                return ContentResponse::consumed(true);
            }
            let event = self.menu.activate_item(slot);
            self.last_action = self.action_for(event);
            return ContentResponse {
                consumed: self.last_action.is_some(),
                changed: self.last_action.is_some(),
                activated: self.last_action.is_some(),
                ..Default::default()
            };
        }
        let target = if part == 0 {
            self.actions.len()
        } else {
            let Some(i) = self
                .visible
                .iter()
                .copied()
                .find(|&i| self.semantic_ids[&self.actions[i].id] == part)
            else {
                return ContentResponse::default();
            };
            i
        };
        self.cancel();
        self.focused = Some(target);
        let focus = matches!(action, AccessibilityAction::Focus);
        self.set_focus(self.has_focus || focus, self.focus_visible || focus);
        if focus {
            return ContentResponse::consumed(true);
        }
        if target == self.actions.len() {
            let result = self.trigger.accessibility_action(None, action);
            if result.activated {
                self.menu.show(0);
            }
            ContentResponse {
                activated: false,
                ..result
            }
        } else {
            let result = self.buttons[target].accessibility_action(None, action);
            if result.activated {
                self.last_action = Some(self.actions[target].id);
            }
            result
        }
    }
    fn event(
        &mut self,
        event: &crate::composition::ContentEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::{ContentEvent, ContentResponse};
        match event {
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerMove(p)
            | ContentEvent::PointerUp(p) => {
                use crate::composition::{PointerButton, PointerEvent, PointerPhase};
                let phase = match event {
                    ContentEvent::PointerDown(_) => PointerPhase::Down,
                    ContentEvent::PointerUp(_) => PointerPhase::Up,
                    _ => PointerPhase::Move,
                };
                self.pointer_event(&PointerEvent {
                    position: *p,
                    phase,
                    button: Some(PointerButton::Primary),
                    modifiers: Default::default(),
                    click_count: 1,
                })
            }
            ContentEvent::Key { key, down } => {
                self.key_event(&crate::composition::ContentKeyEvent {
                    key: Some(*key),
                    down: *down,
                    repeat: false,
                    modifiers: Default::default(),
                    native_code: 0,
                    characters: String::new(),
                    characters_ignoring_modifiers: String::new(),
                })
            }
            ContentEvent::Cancel => {
                self.cancel();
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_event(
        &mut self,
        pointer: &crate::composition::PointerEvent,
    ) -> crate::composition::ContentResponse {
        use crate::composition::{ContentResponse, PointerPhase};
        if !pointer.is_primary() {
            return ContentResponse::default();
        }
        self.last_action = None;
        if self.is_open() {
            let p = pointer.position;
            match pointer.phase {
                PointerPhase::Down => {
                    self.menu.pointer_down_at(p);
                }
                PointerPhase::Up => {
                    let event = self.menu.pointer_up_at(p);
                    self.last_action = self.action_for(event);
                }
                _ => self.menu.pointer_move_at(p),
            }
            return ContentResponse {
                consumed: true,
                capture_pointer: pointer.phase == PointerPhase::Down,
                activated: self.last_action.is_some(),
                changed: true,
                ..Default::default()
            };
        }
        if pointer.phase == PointerPhase::Down {
            self.held = if !self.hidden.is_empty() && self.trigger_rect.contains(pointer.position) {
                Some(self.actions.len())
            } else {
                self.visible.iter().copied().find(|&i| {
                    self.buttons[i]
                        .button
                        .layout()
                        .is_some_and(|l| l.hit_test(pointer.position))
                })
            };
        }
        if pointer.phase == PointerPhase::Down {
            self.focused = self.held;
            self.set_focus(self.has_focus, self.focus_visible);
        }
        let Some(i) = self.held else {
            let before = self
                .buttons
                .iter()
                .map(|button| button.button.state.hovered)
                .collect::<Vec<_>>();
            let trigger_before = self.trigger.button.state.hovered;
            self.pointer_move(pointer.position);
            return ContentResponse {
                changed: trigger_before != self.trigger.button.state.hovered
                    || before
                        .iter()
                        .zip(&self.buttons)
                        .any(|(old, button)| *old != button.button.state.hovered),
                ..Default::default()
            };
        };
        let result = if i == self.actions.len() {
            self.trigger.pointer_event(pointer)
        } else {
            self.buttons[i].pointer_event(pointer)
        };
        if result.activated {
            if i == self.actions.len() {
                self.menu.show(0);
            } else if self.actions[i].enabled {
                self.last_action = Some(self.actions[i].id);
            }
        }
        if pointer.phase == PointerPhase::Up {
            self.held = None;
        }
        ContentResponse {
            activated: self.last_action.is_some(),
            ..result
        }
    }
}
struct ToolbarText<'a>(&'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for ToolbarText<'_> {
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        self.0.measure(s, role, width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        self.0.draw(scene, s, role, rect, align, ink);
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{
        Component, PointerButton, PointerEvent, PointerModifiers, PointerPhase,
    };
    struct Text {
        height: f32,
    }
    impl ButtonTextRenderer for Text {
        fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
            role.style().size * self.height / role.style().line_height
        }
        fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
            assert!(!text.contains('\n'));
            Size::new(text.chars().count() as f32 * 8., self.height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            text: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
            assert!(!text.contains('\n'));
        }
    }
    fn toolbar() -> ToolbarControl {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_actions(vec![
            ToolbarAction {
                id: 11,
                title: "First\nAction".into(),
                enabled: true,
            },
            ToolbarAction {
                id: 22,
                title: "Second action".into(),
                enabled: false,
            },
            ToolbarAction {
                id: 33,
                title: "Third action".into(),
                enabled: true,
            },
        ]);
        toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
        toolbar
    }
    fn pointer(p: Point, phase: PointerPhase) -> PointerEvent {
        PointerEvent {
            position: p,
            phase,
            button: Some(PointerButton::Primary),
            modifiers: PointerModifiers::default(),
            click_count: 1,
        }
    }
    #[test]
    fn mixed_window_actions_share_height_center_and_hover_geometry() {
        let mut toolbar = ToolbarControl::window();
        toolbar.set_actions(vec![
            ToolbarAction {
                id: 1,
                title: "Window label".into(),
                enabled: true,
            },
            ToolbarAction {
                id: 2,
                title: "Icon".into(),
                enabled: true,
            },
        ]);
        toolbar.set_action_icon(2, Some(ImageId(7)));
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., height),
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        let first = toolbar.buttons[0].button.layout().unwrap().frame;
        let second = toolbar.buttons[1].button.layout().unwrap().frame;
        assert_eq!(first.size.height, second.size.height);
        assert_eq!(first.center().y, second.center().y);
        let chrome_a = toolbar.buttons[0].button.chrome_bounds().unwrap();
        let chrome_b = toolbar.buttons[1].button.chrome_bounds().unwrap();
        assert_eq!(chrome_a.size.height, chrome_b.size.height);
        assert_eq!(chrome_a.size.height, 28.);
        assert_eq!(chrome_a.center().y, first.center().y);
        let margin = Point::new(first.center().x, first.min_y() + 1.);
        assert!(chrome_a.contains(margin));
        toolbar.pointer_down(margin);
        assert_eq!(toolbar.pointer_up(margin), Some(1));
        assert_eq!(first.size.height, 28.);
        assert_eq!(toolbar.text_role, proposal::TextRole::UiBody);
        assert_eq!(
            ToolbarControl::default().text_role,
            proposal::TextRole::UiBody
        );
    }
    #[test]
    fn replacing_or_clearing_registration_rejects_old_geometry_without_panicking() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., 48.),
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        let point = toolbar.buttons[0].button.layout().unwrap().frame.center();
        toolbar.pointer_event(&pointer(point, PointerPhase::Down));
        toolbar.set_focus(true, true);
        toolbar.set_actions(vec![ToolbarAction {
            id: 99,
            title: "Replacement".into(),
            enabled: true,
        }]);
        assert!(
            !toolbar
                .pointer_event(&pointer(point, PointerPhase::Up))
                .activated
        );
        assert!(
            !toolbar
                .pointer_event(&pointer(point, PointerPhase::Down))
                .consumed
        );
        assert_eq!(toolbar.take_action(), None);
        toolbar.set_actions(vec![]);
        toolbar.set_focus(false, false);
        toolbar.render_overlay(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., 400.),
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        assert!(!toolbar.focusable());
    }
    #[test]
    fn unheld_hover_reports_only_real_redraw_and_mixed_icons_keep_the_action_ids() {
        let mut toolbar = toolbar();
        assert!(toolbar.set_action_icon(11, Some(ImageId(1))));
        assert!(!toolbar.set_action_icon(999, Some(ImageId(2))));
        let mut scene = Scene::new();
        let mut text = Text { height: 16. };
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 600., 48.),
            tekes_ui_tokens::Theme::LIGHT.into(),
        );
        let point = toolbar.buttons[0].button.layout().unwrap().frame.center();
        let hovered = toolbar.pointer_event(&pointer(point, PointerPhase::Move));
        assert!(hovered.changed && !hovered.consumed);
        assert!(
            !toolbar
                .pointer_event(&pointer(point, PointerPhase::Move))
                .changed
        );
        let exited = toolbar.pointer_event(&pointer(Point::new(900., 900.), PointerPhase::Move));
        assert!(exited.changed && !exited.consumed);
        toolbar.pointer_event(&pointer(point, PointerPhase::Down));
        assert!(
            toolbar
                .pointer_event(&pointer(point, PointerPhase::Up))
                .activated
        );
        assert_eq!(toolbar.take_action(), Some(11));
    }
    #[test]
    fn pure_icon_toolbar_uses_square_hits_original_ids_and_named_overflow() {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_icon_actions(vec![
            ToolbarIconAction {
                id: 101,
                image: ImageId(1),
                label: "Back".into(),
                enabled: true,
            },
            ToolbarIconAction {
                id: 102,
                image: ImageId(2),
                label: "Forward".into(),
                enabled: false,
            },
        ]);
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        let mut scene = Scene::new();
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 240., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let first = toolbar.buttons[0].button.layout().unwrap().frame;
        assert_eq!(first.size.width, 28.);
        assert!(toolbar.pointer_down(first.center()));
        assert_eq!(toolbar.pointer_up(first.center()), Some(101));
        let second = toolbar.buttons[1].button.layout().unwrap().frame;
        toolbar.pointer_down(second.center());
        assert_eq!(toolbar.pointer_up(second.center()), None);
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 40., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(toolbar.measure(&mut text).height, height);
        assert_eq!(toolbar.overflow_ids(), vec![101, 102]);
        assert_eq!(toolbar.actions[0].title, "Back");
    }

    #[test]
    fn narrow_toolbar_overflows_without_growing_and_measures_font_changes() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        let mut scene = Scene::new();
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert!(toolbar.overflow_ids().is_empty());
        toolbar.render(
            &mut scene,
            &mut text,
            Rect::new(0., 0., 80., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(toolbar.overflow_ids(), vec![11, 22, 33]);
        assert_eq!(toolbar.measure(&mut text).height, height);
        text.height = 60.;
        assert!(toolbar.measure(&mut text).height > height);
    }
    #[test]
    fn overflow_activates_registered_id_and_disables_unavailable_actions() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 80., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let p = Point::new(
            toolbar.trigger_rect.min_x() + 4.,
            toolbar.trigger_rect.min_y() + 4.,
        );
        assert!(
            toolbar
                .pointer_event(&pointer(p, PointerPhase::Down))
                .capture_pointer
        );
        toolbar.pointer_event(&pointer(p, PointerPhase::Up));
        assert!(toolbar.is_open());
        toolbar.render_overlay(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., 400.),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        // Disabled rows cannot activate even through a direct menu action.
        assert_eq!(toolbar.menu.activate_item(1), MenuEvent::None);
        let event = toolbar.menu.activate_item(2);
        assert_eq!(toolbar.action_for(event), Some(33));
        // A geometry change revokes a held trigger so a late up cannot reopen it.
        toolbar.pointer_event(&pointer(p, PointerPhase::Down));
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        toolbar.pointer_event(&pointer(p, PointerPhase::Up));
        assert!(!toolbar.is_open());
    }
    #[test]
    fn real_button_drag_can_bubble_without_a_phantom_activation() {
        let mut toolbar = toolbar();
        let mut text = Text { height: 16. };
        let height = toolbar.measure(&mut text).height;
        toolbar.render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 600., height),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let p = Point::new(20., height * 0.5);
        assert!(
            toolbar
                .pointer_event(&pointer(p, PointerPhase::Down))
                .capture_pointer
        );
        assert!(
            !toolbar
                .pointer_event(&pointer(Point::new(220., height * 0.5), PointerPhase::Drag))
                .consumed
        );
        toolbar.cancel();
        assert!(
            !toolbar
                .pointer_event(&pointer(p, PointerPhase::Up))
                .activated
        );
        assert_eq!(toolbar.take_action(), None);
    }
}

#[cfg(test)]
mod composition_tests {
    use super::*;
    use crate::composition::*;
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, text: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(text.len() as f32 * 8., 16.)
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
    #[test]
    fn component_host_retains_command_identity_and_focus_across_registration_updates() {
        let mut toolbar = ToolbarControl::default();
        toolbar.set_actions(vec![ToolbarAction {
            id: 7,
            title: "Increment 0".into(),
            enabled: true,
        }]);
        toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
        let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
        host.set_action_events(true);
        let frame = Rect::new(0., 0., 600., 48.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        )
        .unwrap();
        let p = Point::new(24., 24.);
        host.event(&ContentEvent::PointerDown(p));
        host.event(&ContentEvent::PointerUp(p));
        assert_eq!(host.drain_actions().len(), 1);
        let toolbar = host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        assert_eq!(toolbar.take_action(), Some(7));
        toolbar.set_actions(vec![ToolbarAction {
            id: 7,
            title: "Increment 1".into(),
            enabled: true,
        }]);
        let key = |down| {
            ContentEvent::KeyInput(ContentKeyEvent {
                key: Some(ControlKey::Space),
                down,
                repeat: false,
                modifiers: Default::default(),
                native_code: 49,
                characters: " ".into(),
                characters_ignoring_modifiers: " ".into(),
            })
        };
        host.event(&key(true));
        host.event(&key(false));
        assert_eq!(host.drain_actions().len(), 1);
        assert_eq!(
            host.get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap()
                .take_action(),
            Some(7)
        );
    }
}
