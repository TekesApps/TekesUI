//! Public navigation components retain their original models and mounted Content.
use super::*;
mod breadcrumb;
mod command;
use crate::TabsView;
pub use breadcrumb::BreadcrumbContent;
pub use command::CommandPaletteContent;

pub(super) fn validate_display_slot(content: &Content) -> Result<(), CompositionError> {
    if let Some(child) = content.interactive_id() {
        return Err(CompositionError::NestedInteractive {
            action: content.id(),
            child,
        });
    }
    fn owns_overlay(content: &Content) -> bool {
        content.component.overlay_scope().is_some()
            || content.component.overlay_open()
            || content.component.overlay_presented()
            || content.component.children().iter().any(owns_overlay)
    }
    if owns_overlay(content) {
        Err(CompositionError::InvalidSlotCount)
    } else {
        Ok(())
    }
}

/// A single decorative label and its original, independently mounted panel.
#[derive(Debug)]
pub struct TabItem {
    label: Content,
    panel: Content,
    name: String,
    disabled: bool,
}
impl TabItem {
    pub fn new(label: Content, panel: Content) -> Self {
        fn name(content: &Content) -> String {
            let own = content.component.semantics().name;
            if !own.trim().is_empty() {
                own
            } else {
                content
                    .component
                    .children()
                    .iter()
                    .map(name)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
        Self {
            name: name(&label),
            label,
            panel,
            disabled: false,
        }
    }
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Original Tabs roving interaction with arbitrary passive Content labels and
/// real retained panel subtrees. Selection cannot hide an ongoing composition.
#[derive(Debug)]
pub struct TabsContent {
    children: Vec<Content>,
    names: Vec<String>,
    disabled_items: Vec<bool>,
    view: TabsView,
    pub compact: bool,
    disabled: bool,
    window_active: bool,
    sizes: Vec<Size>,
    lanes: Vec<f32>,
    header_height: f32,
    last_selection: Option<ContentActionPayload>,
}
impl TabsContent {
    pub fn new(items: Vec<TabItem>, selected: usize) -> Self {
        let count = items.len();
        let mut this = Self {
            children: Vec::with_capacity(count * 2),
            names: Vec::new(),
            disabled_items: Vec::new(),
            view: TabsView::new(count),
            compact: false,
            disabled: false,
            window_active: true,
            sizes: Vec::new(),
            lanes: Vec::new(),
            header_height: 0.,
            last_selection: None,
        };
        for mut item in items {
            item.label.suppress_semantics();
            this.names.push(item.name);
            this.disabled_items.push(item.disabled);
            this.children.push(item.label);
            this.children.push(item.panel);
        }
        for (i, disabled) in this.disabled_items.iter().copied().enumerate() {
            this.view.set_item_disabled(i, disabled);
        }
        let initial = selected.min(count.saturating_sub(1));
        let initial = (!this.disabled_items.get(initial).copied().unwrap_or(true))
            .then_some(initial)
            .or_else(|| this.disabled_items.iter().position(|d| !d));
        if let Some(initial) = initial {
            this.view.select(initial);
        }
        this.sync_panels();
        this
    }
    pub fn selected(&self) -> usize {
        self.view.selected()
    }
    fn sync_panels(&mut self) {
        let selected = self.selected();
        for (index, pair) in self.children.chunks_exact_mut(2).enumerate() {
            pair[1].set_composition_active(index == selected);
        }
    }
    pub(super) fn subtree_composing(content: &Content) -> bool {
        content.component.composing()
            || content
                .component
                .text_surface()
                .is_some_and(|s| s.input.marked.is_some())
            || content
                .component
                .children()
                .iter()
                .any(Self::subtree_composing)
    }
    fn may_select(&self, index: usize) -> bool {
        !self.disabled
            && self.window_active
            && index < self.names.len()
            && !self.disabled_items[index]
            && (index == self.selected()
                || !Self::subtree_composing(&self.children[self.selected() * 2 + 1]))
    }
    pub fn select(&mut self, index: usize) -> bool {
        if !self.may_select(index) {
            return false;
        }
        let changed = self.view.select(index);
        self.sync_panels();
        changed
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.view.set_disabled(disabled);
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        self.view.set_window_active(active);
    }
    pub fn set_tab_disabled(&mut self, index: usize, disabled: bool) -> bool {
        let Some(item) = self.disabled_items.get_mut(index) else {
            return false;
        };
        let changed = *item != disabled;
        *item = disabled;
        self.view.set_item_disabled(index, disabled);
        changed
    }
    /// Reorder complete pairs, preserving Content mounts and selected panel.
    /// An outstanding gesture is revoked; no Content lifecycle is replayed.
    pub fn reorder(&mut self, order: &[usize]) -> bool {
        if order.len() != self.names.len()
            || order.iter().copied().collect::<BTreeSet<_>>() != (0..self.names.len()).collect()
        {
            return false;
        }
        let selected = order
            .iter()
            .position(|i| *i == self.selected())
            .unwrap_or(0);
        let focused = self.view.focused_index().is_some();
        let mut children: Vec<_> = std::mem::take(&mut self.children)
            .into_iter()
            .map(Some)
            .collect();
        let names = std::mem::take(&mut self.names);
        let disabled = std::mem::take(&mut self.disabled_items);
        for &index in order {
            self.children.push(children[index * 2].take().unwrap());
            self.children.push(children[index * 2 + 1].take().unwrap());
            self.names.push(names[index].clone());
            self.disabled_items.push(disabled[index]);
        }
        self.view = TabsView::new(order.len());
        self.view.select(selected);
        for (i, disabled) in self.disabled_items.iter().copied().enumerate() {
            self.view.set_item_disabled(i, disabled);
        }
        self.view.set_disabled(self.disabled);
        self.view.set_window_active(self.window_active);
        if focused {
            self.view.focus(selected, true);
        }
        self.sync_panels();
        self.sizes.clear();
        true
    }
    fn snapshot(&mut self, index: Option<usize>) -> ContentResponse {
        self.last_selection = index.map(|index| ContentActionPayload::TabSelection {
            index,
            label: self.children[index * 2].id(),
            panel: self.children[index * 2 + 1].id(),
        });
        self.sync_panels();
        ContentResponse {
            consumed: index.is_some(),
            changed: index.is_some(),
            activated: index.is_some(),
            ..Default::default()
        }
    }
    fn measure_labels(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) {
        let natural: Vec<_> = self
            .children
            .chunks_exact_mut(2)
            .map(|pair| pair[0].measure_intrinsic(text, available).size)
            .collect();
        self.lanes = TabsView::content_lanes(
            text,
            &natural,
            available.width,
            proposal::TextRole::UiBody,
            self.compact,
        );
        self.sizes = self
            .children
            .chunks_exact_mut(2)
            .zip(&self.lanes)
            .map(|(pair, lane)| {
                let lane = TabsView::content_lane_width(text, *lane);
                let insets = crate::ButtonModel::measure_content(
                    text,
                    Size::default(),
                    lane,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .width;
                pair[0]
                    .measure_intrinsic(text, Size::new((lane - insets).max(0.), available.height))
                    .size
            })
            .collect();
        self.header_height = TabsView::content_height(text, &self.sizes, &self.lanes, self.compact);
    }
}
impl Component for TabsContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != self.names.len() * 2 {
            return Err(CompositionError::InvalidSlotCount);
        }
        for (index, pair) in self.children.chunks_exact(2).enumerate() {
            if self.names[index].trim().is_empty() {
                return Err(CompositionError::EmptyAccessibleName);
            }
            validate_display_slot(&pair[0])?;
        }
        Ok(())
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.measure_labels(text, available);
        let panel = self
            .children
            .get_mut(self.view.selected() * 2 + 1)
            .map(|panel| {
                panel
                    .measure(
                        text,
                        Size::new(
                            available.width,
                            (available.height - self.header_height).max(0.),
                        ),
                    )
                    .size
            })
            .unwrap_or_default();
        MeasuredChild::new(Size::new(
            available.width,
            self.header_height + panel.height,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.measure_labels(text, frame.size);
        let height = self.header_height.min(frame.size.height);
        let panel_frame = Rect::new(
            frame.min_x(),
            frame.min_y() + height,
            frame.size.width,
            (frame.size.height - height).max(0.),
        );
        for pair in self.children.chunks_exact_mut(2) {
            pair[1].arrange(text, panel_frame, clip);
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated Tabs Surface");
        let layout = self.view.render_content(
            scene,
            &mut FrameTextAdapter(text, None, true),
            &self.sizes,
            &self.lanes,
            Rect::new(
                frame.min_x(),
                frame.min_y(),
                frame.size.width,
                self.header_height.min(frame.size.height),
            ),
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            self.compact,
        );
        for (index, pair) in self.children.chunks_exact_mut(2).enumerate() {
            if let Some(item) = layout.items.get(index) {
                pair[0].arrange(text, item.content, item.visible.unwrap_or_default());
                let mut label_theme = theme;
                if let Some((foreground, icon)) = self.view.content_theme(index, theme.palette) {
                    label_theme.foreground = Some(foreground);
                    label_theme.icon_foreground = Some(icon);
                }
                pair[0].paint(scene, text, label_theme);
            }
            pair[1].paint(scene, text, theme);
        }
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        for pair in self.children.chunks_exact_mut(2) {
            pair[1].paint_overlays(scene, text, theme);
        }
    }
    // A child may own a nonstandard overlay; forward its original paint without
    // declaring a second input scope on the tab list itself.
    fn overlay_presented(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        !self.disabled && self.window_active && self.disabled_items.iter().any(|d| !d)
    }
    fn interactive(&self) -> bool {
        self.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            let index = self.view.focused_index().unwrap_or(self.selected());
            let index = if !self.disabled_items.get(index).copied().unwrap_or(true) {
                Some(index)
            } else {
                self.disabled_items.iter().position(|d| !d)
            };
            if let Some(index) = index
                && self.may_select(index)
            {
                self.view.focus(index, visible);
                self.sync_panels();
            }
        } else {
            self.view.clear_focus();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::TabList, "Tabs");
        semantics.disabled = self.disabled || !self.window_active;
        semantics
    }
    fn semantic_frame(&self, frame: Rect) -> Rect {
        Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            self.header_height,
        )
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.view
            .placement()
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let frame = item.visible.filter(|f| !f.is_empty())?;
                let mut semantics = ContentSemantics::new(ContentRole::Tab, self.names.get(index)?);
                semantics.disabled =
                    self.disabled || !self.window_active || self.disabled_items[index];
                semantics.selected = self.selected() == index;
                Some(SemanticPart {
                    index,
                    frame,
                    focused: self.view.focused_index() == Some(index),
                    semantics,
                })
            })
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last_selection = None;
        let Some(index) = part.filter(|index| {
            self.view
                .placement()
                .items
                .get(*index)
                .is_some_and(|item| item.visible.is_some_and(|f| !f.is_empty()))
        }) else {
            return ContentResponse::default();
        };
        if !self.may_select(index) {
            return ContentResponse::default();
        }
        match action {
            AccessibilityAction::Press => {
                self.view.focus(index, false);
                self.snapshot(Some(index))
            }
            AccessibilityAction::Focus => {
                self.view.focus(index, true);
                self.sync_panels();
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        if !self.focusable() {
            return Vec::new();
        }
        self.view
            .placement()
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.disabled_items[*i])
            .filter_map(|(_, item)| item.visible)
            .collect()
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| self.last_selection.clone())
            .flatten()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_selection = None;
        match event {
            ContentEvent::PointerMove(point) => {
                self.view.pointer_move_at(*point);
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(point) => {
                if self
                    .view
                    .placement()
                    .items
                    .iter()
                    .position(|item| item.hit_test(*point))
                    .is_some_and(|i| !self.may_select(i))
                {
                    return ContentResponse::consumed(false);
                }
                let pressed = self.view.pointer_down_at(*point).is_some();
                ContentResponse {
                    consumed: pressed,
                    changed: pressed,
                    capture_pointer: pressed,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(point) => {
                if self
                    .view
                    .placement()
                    .items
                    .iter()
                    .position(|item| item.hit_test(*point))
                    .is_some_and(|i| !self.may_select(i))
                {
                    self.view.cancel();
                    return ContentResponse::consumed(false);
                }
                let index = self.view.pointer_up_at(*point);
                self.snapshot(index)
            }
            ContentEvent::Key { key, down } => {
                if self
                    .children
                    .get(self.selected() * 2 + 1)
                    .is_some_and(Self::subtree_composing)
                    && matches!(
                        key,
                        ControlKey::Left | ControlKey::Right | ControlKey::Home | ControlKey::End
                    )
                {
                    return ContentResponse::consumed(false);
                }
                if matches!(key, ControlKey::Enter | ControlKey::Space)
                    && self
                        .view
                        .focused_index()
                        .is_some_and(|index| !self.may_select(index))
                {
                    self.view.cancel();
                    return ContentResponse::consumed(false);
                }
                let index = self.view.key(*key, *down);
                self.snapshot(index)
            }
            ContentEvent::Cancel => {
                self.view.cancel();
                ContentResponse::default()
            }
            _ => ContentResponse::default(),
        }
    }
}
