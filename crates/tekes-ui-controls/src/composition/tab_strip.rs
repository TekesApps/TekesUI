//! Header-only Tabs adapter. The host owns destination content and commits intents.
use super::*;
use crate::{ButtonSize, TabsView};

/// Maximum natural title width; close chrome and control insets are additional.
pub const TAB_TITLE_MAX_WIDTH: f32 = 160.;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabStripIntent {
    pub key: ComponentId,
    pub close: bool,
}
#[derive(Debug)]
pub struct TabStripItem {
    pub label: Content,
    pub name: String,
    pub closable: bool,
}
#[derive(Debug)]
pub struct TabStripContent {
    children: Vec<Content>,
    names: Vec<String>,
    closable: Vec<bool>,
    view: TabsView,
    sizes: Vec<Size>,
    lanes: Vec<f32>,
    height: f32,
    intent: Option<TabStripIntent>,
    pub title_max_width: f32,
}
impl TabStripContent {
    pub fn new(items: Vec<TabStripItem>, selected: usize) -> Self {
        let mut view = TabsView::new(items.len());
        let mut children = Vec::new();
        let mut names = Vec::new();
        let mut closable = Vec::new();
        for (index, mut item) in items.into_iter().enumerate() {
            view.set_closable(index, item.closable);
            item.label.suppress_semantics();
            item.label.toolbar_label_layouts();
            children.push(item.label);
            names.push(item.name);
            closable.push(item.closable);
        }
        view.select(selected);
        Self {
            children,
            names,
            closable,
            view,
            sizes: vec![],
            lanes: vec![],
            height: 0.,
            intent: None,
            title_max_width: TAB_TITLE_MAX_WIDTH,
        }
    }
    pub fn select(&mut self, index: usize) {
        self.view.select(index);
    }
    pub fn selected(&self) -> usize {
        self.view.selected()
    }
    /// Document-local target, usable by an enclosing horizontal viewport.
    pub fn selected_rect(&self) -> Rect {
        let index = self.selected();
        Rect::new(
            self.lanes.iter().take(index).sum(),
            0.,
            self.lanes.get(index).copied().unwrap_or(0.),
            self.height,
        )
    }
    fn measure_labels(&mut self, text: &mut dyn ButtonTextRenderer) {
        let gap = crate::composite::focus_reveal_outset(text.backing_scale()).max(4.);
        self.sizes = self
            .children
            .iter_mut()
            .map(|child| {
                child
                    .measure_intrinsic(text, Size::new(self.title_max_width, 1e8))
                    .size
            })
            .collect();
        let close = TabsView::close_width(text);
        self.lanes = self
            .sizes
            .iter()
            .zip(&self.closable)
            .map(|(size, closable)| {
                crate::ButtonModel::measure_content(
                    text,
                    *size,
                    f32::MAX,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .width
                    + gap
                    + if *closable { close } else { 0. }
            })
            .collect();
        self.height = TabsView::content_height(text, &self.sizes, &self.lanes, false);
    }
    fn snapshot(&mut self, selected: Option<usize>) -> ContentResponse {
        let close = self.view.take_close_request();
        self.intent = close.or(selected).map(|index| TabStripIntent {
            key: self.children[index].id(),
            close: close.is_some(),
        });
        ContentResponse {
            consumed: self.intent.is_some(),
            changed: self.intent.is_some(),
            activated: self.intent.is_some(),
            ..Default::default()
        }
    }
}
impl Component for TabStripContent {
    fn rectangular_input_geometry(&self) -> bool {
        !self.view.placement().items.is_empty()
    }

    fn requires_icon_services(&self) -> bool {
        self.closable.iter().any(|v| *v)
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.view.close_icon_error()
    }
    fn validate(&self) -> Result<(), CompositionError> {
        for (child, name) in self.children.iter().zip(&self.names) {
            if name.trim().is_empty() {
                return Err(CompositionError::EmptyAccessibleName);
            }
            super::navigation::validate_display_slot(child)?;
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
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        self.measure_labels(text);
        MeasuredChild::new(Size::new(self.lanes.iter().sum(), self.height))
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
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            false,
        );
        for (index, child) in self.children.iter_mut().enumerate() {
            if let Some(item) = layout.items.get(index) {
                child.arrange(text, item.content, item.visible.unwrap_or_default());
                let mut label_theme = theme;
                if let Some((foreground, icon)) = self.view.content_theme(index, theme.palette) {
                    label_theme.foreground = Some(foreground);
                    label_theme.icon_foreground = Some(icon);
                }
                child.paint(scene, text, label_theme);
            }
        }
    }
    fn focusable(&self) -> bool {
        !self.children.is_empty()
    }
    fn interactive(&self) -> bool {
        self.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            // PointerDown already focused the pressed title without selecting
            // it. Restoring the old selected (possibly offscreen) tab here
            // makes the next paint clear focus and cancel the live press.
            if self.view.pointer_press_index().is_some() {
                return;
            }
            if let Some(index) = self.view.close_focused_index() {
                self.view.focus_close(index, visible);
            } else {
                self.view.focus(self.selected(), visible);
            }
        } else {
            self.view.clear_focus();
        }
    }
    fn set_focus_from_traversal(&mut self, reverse: bool) {
        let selected = self.selected();
        if !reverse || !self.view.focus_close(selected, true) {
            self.view.focus(selected, true);
        }
    }
    fn focus_boundary(&mut self, reverse: bool) -> ContentResponse {
        let changed = if reverse {
            self.view
                .close_focused_index()
                .is_some_and(|index| self.view.focus(index, true))
        } else {
            self.view
                .focused_index()
                .is_some_and(|index| self.view.focus_close(index, true))
        };
        ContentResponse {
            consumed: changed,
            changed,
            ..Default::default()
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::TabList, "Tabs")
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let mut parts: Vec<_> = self
            .view
            .placement()
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let frame = item.visible.filter(|f| !f.is_empty())?;
                let mut semantics = ContentSemantics::new(ContentRole::Tab, &self.names[index]);
                semantics.selected = self.selected() == index;
                Some(SemanticPart {
                    index,
                    frame,
                    focused: self.view.focused_index() == Some(index),
                    semantics,
                })
            })
            .collect();
        for (index, layout) in self.view.placement().close_buttons.iter().enumerate() {
            if let Some(frame) = layout
                .and_then(|layout| layout.visible)
                .filter(|frame| !frame.is_empty())
            {
                parts.push(SemanticPart {
                    index: self.names.len() + index,
                    frame,
                    focused: self.view.close_focused_index() == Some(index),
                    semantics: ContentSemantics::new(
                        ContentRole::Button,
                        format!("Close {}", self.names[index]),
                    ),
                });
            }
        }
        parts
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        part.checked_sub(self.names.len())
            .filter(|index| self.closable.get(*index).copied().unwrap_or(false))
    }
    fn semantic_part_is_auxiliary_action(&self, part: usize) -> bool {
        self.semantic_part_parent(part).is_some()
    }
    fn semantic_part_preserves_activation_focus(&self, part: usize) -> bool {
        self.semantic_part_parent(part).is_some()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.intent = None;
        let Some(part) = part.filter(|part| self.semantic_parts().iter().any(|p| p.index == *part))
        else {
            return ContentResponse::default();
        };
        if let Some(index) = self.semantic_part_parent(part) {
            return match action {
                AccessibilityAction::Press => {
                    self.intent = Some(TabStripIntent {
                        key: self.children[index].id(),
                        close: true,
                    });
                    ContentResponse {
                        consumed: true,
                        changed: true,
                        activated: true,
                        ..Default::default()
                    }
                }
                AccessibilityAction::Focus => {
                    self.view.focus_close(index, true);
                    ContentResponse::consumed(true)
                }
                _ => ContentResponse::default(),
            };
        }
        match action {
            AccessibilityAction::Press => {
                self.view.focus(part, false);
                self.snapshot(Some(part))
            }
            AccessibilityAction::Focus => {
                self.view.focus(part, true);
                self.snapshot(Some(part))
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        self.view
            .placement()
            .items
            .iter()
            .filter_map(|item| item.visible)
            .collect()
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| self.intent.clone().map(ContentActionPayload::TabStrip))
            .flatten()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.intent = None;
        match event {
            ContentEvent::PointerMove(point) => {
                self.view.pointer_move_at(*point);
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(point) => {
                let pressed = self.view.pointer_down_at(*point).is_some();
                ContentResponse {
                    consumed: pressed,
                    changed: pressed,
                    capture_pointer: pressed,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(point) => {
                let selected = self.view.pointer_up_at(*point);
                self.snapshot(selected)
            }
            ContentEvent::Key { key, down } => {
                let selected = self.view.key(*key, *down);
                let mut response = self.snapshot(selected);
                if matches!(key, ControlKey::Space | ControlKey::Enter)
                    && (self.view.focused_index().is_some()
                        || self.view.close_focused_index().is_some())
                {
                    response.consumed = true;
                    response.changed = true;
                }
                response
            }
            ContentEvent::Cancel => {
                self.view.cancel();
                ContentResponse::default()
            }
            _ => ContentResponse::default(),
        }
    }
}
