//! Breadcrumb composition adapter over the original multi-link BreadcrumbView.
use super::*;
use crate::{extended::Breadcrumb, layout_view::BreadcrumbView};

/// A breadcrumb path with one semantic link per ancestor and a selected current page.
///
/// The model's item strings are also the displayed labels. Pointer, keyboard and
/// accessibility actions are handled by the original `BreadcrumbView` owner.
#[derive(Debug)]
pub struct BreadcrumbContent {
    pub model: Breadcrumb,
    pub view: BreadcrumbView,
    disabled: bool,
    window_active: bool,
    last_navigation: Option<(usize, Vec<String>)>,
    /// The path whose frames were published by the most recent render. This is
    /// used only to avoid publishing stale semantic parts before the next paint;
    /// gesture validity remains owned by `BreadcrumbView`.
    rendered_items: Option<Vec<String>>,
}

impl BreadcrumbContent {
    pub fn new(model: Breadcrumb) -> Self {
        Self {
            model,
            view: BreadcrumbView::default(),
            disabled: false,
            window_active: true,
            last_navigation: None,
            rendered_items: None,
        }
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.view.set_disabled(disabled);
    }

    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        self.view.set_window_active(active);
    }

    fn navigable(&self, index: usize) -> bool {
        self.window_active
            && !self.disabled
            && self.rendered_items.as_deref() == Some(self.model.items.as_slice())
            && index < self.model.items.len().saturating_sub(1)
            && self
                .view
                .layout()
                .items
                .get(index)
                .and_then(|item| item.visible_frame())
                .is_some_and(|frame| !frame.is_empty())
    }

    fn capture_navigation(&mut self, index: Option<usize>) {
        self.last_navigation = index.map(|index| (index, self.model.items.clone()));
    }

    fn pointer_response(&mut self, phase: PointerPhase, position: Point) -> ContentResponse {
        self.last_navigation = None;
        match phase {
            PointerPhase::Move | PointerPhase::Drag | PointerPhase::Enter => {
                self.view.pointer_move(position);
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            PointerPhase::Exit => {
                self.view.pointer_move(Point::new(-1.0, -1.0));
                ContentResponse {
                    changed: true,
                    ..Default::default()
                }
            }
            PointerPhase::Down => {
                let pressed = self.view.pointer_down(position).is_some();
                ContentResponse {
                    consumed: pressed,
                    changed: pressed,
                    capture_pointer: pressed,
                    ..Default::default()
                }
            }
            PointerPhase::Up => {
                let index = self.view.pointer_up(position, &self.model);
                self.capture_navigation(index);
                ContentResponse {
                    consumed: index.is_some(),
                    changed: index.is_some(),
                    activated: index.is_some(),
                    ..Default::default()
                }
            }
        }
    }
}

impl Component for BreadcrumbContent {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(
            self.view.measure(
                &mut FrameTextAdapter(text, None, true),
                &self.model,
                &self
                    .model
                    .items
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                available.width,
                proposal::TextRole::UiBody,
            ),
        )
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
            .expect("validated inherited Surface");
        let labels = self
            .model
            .items
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        self.view.render(
            scene,
            &mut FrameTextAdapter(text, None, true),
            &self.model,
            &labels,
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
        );
        self.rendered_items = Some(self.model.items.clone());
    }

    fn focusable(&self) -> bool {
        !self.disabled && self.window_active && self.model.items.len() > 1
    }

    fn set_focus(&mut self, focused: bool, visible: bool) {
        if !focused {
            self.view.blur();
        } else if self.view.focused_index().is_none() {
            let count = self.model.items.len().saturating_sub(1);
            let index = if visible {
                (count > 0).then_some(0)
            } else {
                count.checked_sub(1)
            };
            if let Some(index) = index {
                self.view.focus_with_visibility(index, visible);
            }
        }
    }

    fn set_focus_from_traversal(&mut self, reverse: bool) {
        if self.view.focused_index().is_some() {
            return;
        }
        if reverse {
            if let Some(index) = self.model.items.len().checked_sub(2) {
                self.view.focus_with_visibility(index, true);
            }
        } else {
            self.set_focus(true, true);
        }
    }

    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, "Breadcrumb");
        semantics.disabled = !self.focusable();
        semantics
    }

    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if self.rendered_items.as_deref() != Some(self.model.items.as_slice()) {
            return Vec::new();
        }
        let layout = self.view.layout();
        layout
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let label = self.model.items.get(index)?;
                let frame = item.visible_frame().filter(|frame| !frame.is_empty())?;
                let current = index + 1 == self.model.items.len();
                let mut semantics = ContentSemantics::new(
                    if current {
                        ContentRole::Text
                    } else {
                        ContentRole::Link
                    },
                    label,
                );
                semantics.disabled = current || self.disabled || !self.window_active;
                semantics.selected = current;
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
        self.last_navigation = None;
        let Some(index) = part else {
            return ContentResponse::default();
        };
        match action {
            AccessibilityAction::Focus if self.navigable(index) => {
                self.view.focus_with_visibility(index, true);
                ContentResponse::consumed(true)
            }
            AccessibilityAction::Press => {
                let activated = self.view.activate(index, &self.model);
                self.capture_navigation(activated);
                ContentResponse {
                    consumed: activated.is_some(),
                    changed: activated.is_some(),
                    activated: activated.is_some(),
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }

    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then_some(self.last_navigation.as_ref())
            .flatten()
            .map(|(index, path)| ContentActionPayload::BreadcrumbNavigation {
                index: *index,
                path: path.clone(),
            })
    }

    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !pointer.is_primary() {
            self.last_navigation = None;
            return ContentResponse::default();
        }
        self.pointer_response(pointer.phase, pointer.position)
    }

    fn pointer_regions(&self, _frame: Rect, _clip: Rect) -> Vec<Rect> {
        self.view
            .layout()
            .items
            .iter()
            .take(self.model.items.len().saturating_sub(1))
            .filter_map(|item| item.visible_frame())
            .filter(|frame| !frame.is_empty())
            .collect()
    }

    fn focus_boundary(&mut self, reverse: bool) -> ContentResponse {
        if !self.focusable() {
            return ContentResponse::default();
        }
        let last = self.model.items.len().saturating_sub(2);
        let next = match (self.view.focused_index(), reverse) {
            (Some(index), true) if index > 0 => Some(index - 1),
            (Some(index), false) if index < last => Some(index + 1),
            _ => None,
        };
        let Some(next) = next.filter(|index| self.navigable(*index)) else {
            return ContentResponse::default();
        };
        self.view.focus_with_visibility(next, true);
        ContentResponse::consumed(true)
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::PointerMove(point) => self.pointer_response(PointerPhase::Move, *point),
            ContentEvent::PointerDown(point) => self.pointer_response(PointerPhase::Down, *point),
            ContentEvent::PointerUp(point) => self.pointer_response(PointerPhase::Up, *point),
            ContentEvent::Key { key, down } => {
                self.last_navigation = None;
                let (consumed, index) = self.view.key_event(*key, *down, &self.model);
                self.capture_navigation(index);
                ContentResponse {
                    consumed,
                    changed: index.is_some(),
                    activated: index.is_some(),
                    ..Default::default()
                }
            }
            ContentEvent::Cancel => {
                self.last_navigation = None;
                let cancelled = self.view.cancel();
                ContentResponse {
                    consumed: cancelled,
                    changed: cancelled,
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }

    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        key.key.map_or_else(ContentResponse::default, |key_code| {
            self.event(&ContentEvent::Key {
                key: key_code,
                down: key.down,
            })
        })
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        self.set_window_active(event == Lifecycle::Mounted);
    }
}
