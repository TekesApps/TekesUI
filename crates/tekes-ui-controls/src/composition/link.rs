//! Original Link model/view mounted in the shared input, focus and AX tree.
use super::*;
use crate::{extended::Link, layout_view::LinkView};

/// A single link action. Enter activates; Space remains available to scrolling.
/// Use `Content::preserving_focus` for auxiliary actions beside an editor.
#[derive(Debug)]
pub struct LinkContent {
    pub model: Link,
    pub view: LinkView,
    pub label: String,
}
impl LinkContent {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            model: Link::default(),
            view: LinkView::default(),
            label: label.into(),
        }
    }
}
impl Component for LinkContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.label.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let inset = proposal::metric("spacing", "space-regular")
            .expect("Link inset token")
            .max(0.);
        let size = text.measure(
            &self.label,
            proposal::TextRole::UiBody,
            (available.width - 2. * inset).max(0.),
        );
        // Reserve the original content inset and the largest underline stroke,
        // including keyboard focus, without a focus-dependent layout jump.
        let stroke = proposal::metric("stroke", "stroke-emphasis")
            .expect("Link underline stroke")
            .max(0.);
        MeasuredChild::new(Size::new(
            (size.width + 2. * inset).min(available.width.max(0.)),
            size.height + 2. * (inset + stroke),
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
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Surface");
        self.view.render(
            scene,
            &mut FrameTextAdapter(text, None, true),
            &mut self.model,
            &self.label,
            frame,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
        );
    }
    fn focusable(&self) -> bool {
        !self.model.button.state.disabled && !self.model.button.state.loading
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(visible);
        } else {
            self.view.blur();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut result = ContentSemantics::new(ContentRole::Link, &self.label);
        result.disabled = !self.focusable();
        result
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.view.set_window_active(event == Lifecycle::Mounted);
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if part.is_some() || !matches!(action, AccessibilityAction::Press) || !self.focusable() {
            return ContentResponse::default();
        }
        let activated = self.view.routed_activate(&mut self.model);
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        let mut result = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(point) => {
                result.changed = self.view.routed_pointer_move(*point);
            }
            ContentEvent::PointerDown(point) => {
                result.consumed = self.view.routed_pointer_down(*point, &self.model);
                result.capture_pointer = result.consumed;
                result.changed = result.consumed;
            }
            ContentEvent::PointerUp(point) => {
                result.activated = self.view.pointer_up(*point, &mut self.model);
                result.changed = true;
                result.consumed = result.activated;
            }
            ContentEvent::Key { key, down } => {
                let response = self.view.key(*key, *down, &mut self.model);
                result.consumed = response.consumed;
                result.changed = response.changed;
                result.activated = response.changed;
            }
            ContentEvent::Cancel => result.changed = self.view.routed_cancel(),
            _ => {}
        }
        result
    }
}
