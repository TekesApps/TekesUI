//! Compatibility adapter for the original string-backed single-selection list.
use super::*;

/// Composition adapter for the production single-selection List and ListView.
/// All selection ink and interaction remain owned by the standard control.
#[derive(Debug)]
pub struct SelectableListContent {
    pub model: crate::extended::List,
    pub view: crate::collections_view::ListView,
    pub labels: Vec<String>,
    pub name: String,
}
impl SelectableListContent {
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self {
        Self {
            model: crate::extended::List::new(labels.len()),
            view: crate::collections_view::ListView::new(),
            labels,
            name: name.into(),
        }
    }
}
impl Component for SelectableListContent {
    // Interaction ownership is independent of the current empty state.
    fn interactive(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(crate::collections_view::ListView::measure(
            &mut TextAdapter(text),
            &self.labels,
            available.width,
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view.render(
            scene,
            &mut TextAdapter(text),
            &self.model,
            &self.labels,
            frame,
            Some(clip),
            theme.palette,
        );
    }
    fn focusable(&self) -> bool {
        self.model.count > 0
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(self.model.selected.unwrap_or(0), visible);
        } else {
            self.view.blur();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::List, &self.name)
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.view
            .layout()
            .items
            .iter()
            .filter_map(|row| {
                let label = self.labels.get(row.id)?;
                let mut semantics = ContentSemantics::new(ContentRole::ListItem, label);
                semantics.selected = self.model.selected == Some(row.id);
                Some(SemanticPart {
                    index: row.id,
                    frame: row.frame,
                    focused: false,
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
        if matches!(action, AccessibilityAction::Press)
            && let Some(index) = part
            && index < self.model.count
        {
            let changed = self.model.select(index);
            self.view.focus(index, false);
            return ContentResponse {
                consumed: true,
                changed,
                activated: true,
                ..Default::default()
            };
        }
        ContentResponse::default()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        use crate::collections_view::CollectionEvent;
        let mut response = ContentResponse::default();
        let result = match event {
            ContentEvent::PointerMove(point) => {
                response.changed = self.view.pointer_move(*point);
                return response;
            }
            ContentEvent::PointerDown(point) => {
                let result = self.view.pointer_down(*point, &mut self.model);
                response.capture_pointer = result != CollectionEvent::Ignored;
                result
            }
            ContentEvent::PointerUp(point) => self.view.pointer_up(*point, &mut self.model),
            ContentEvent::Key { key, down } => self.view.key(*key, *down, &mut self.model),
            ContentEvent::Cancel => {
                self.view.cancel();
                return ContentResponse::consumed(true);
            }
            _ => return response,
        };
        response.consumed = result != CollectionEvent::Ignored;
        response.changed = matches!(
            result,
            CollectionEvent::Changed | CollectionEvent::Activated(_)
        );
        response.activated = response.changed;
        response
    }
}
