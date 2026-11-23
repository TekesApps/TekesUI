//! Original Link model/view mounted in the shared input, focus and AX tree.
use super::*;
use crate::{extended::Link, layout_view::LinkView};

/// A single link action. Enter activates; Space remains available to scrolling.
/// Use `Content::preserving_focus` for auxiliary actions beside an editor.
#[derive(Debug)]
pub struct LinkContent {
    model: Link,
    view: LinkView,
    label: String,
}
impl LinkContent {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            model: Link::default(),
            view: LinkView::default(),
            label: label.into(),
        }
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.model.button.state.disabled = disabled;
        if disabled {
            self.view.routed_cancel();
            self.view.blur();
        }
    }
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.set_loading(loading);
        self
    }
    pub fn set_loading(&mut self, loading: bool) {
        self.model.button.state.loading = loading;
        if loading {
            self.view.routed_cancel();
            self.view.blur();
        }
    }
    pub fn is_disabled(&self) -> bool {
        self.model.button.state.disabled
    }
    pub fn is_loading(&self) -> bool {
        self.model.button.state.loading
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }
    pub fn model(&self) -> &Link {
        &self.model
    }
    pub fn view(&self) -> &LinkView {
        &self.view
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
        let role = proposal::TextRole::UiBody;
        let scale = crate::button_typography_scale(text, role);
        let inset = proposal::metric("spacing", "space-tight")
            .expect("Link inset token")
            .max(0.)
            * scale;
        let size = text.measure(&self.label, role, (available.width - 2. * inset).max(0.));
        let min_h =
            proposal::metric("size", "size-action-button").expect("Link min height") * scale;
        MeasuredChild::new(Size::new(
            (size.width + 2. * inset).min(available.width.max(0.)),
            (size.height + 2. * inset).max(min_h),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonTextAlign;
    use tekes_ui_tokens::Rgba;

    struct DummyText;
    impl ButtonTextRenderer for DummyText {
        fn measure(&mut self, s: &str, role: proposal::TextRole, max_width: f32) -> Size {
            Size::new(
                (s.chars().count() as f32 * 7.).min(max_width.max(0.)),
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }

    #[test]
    fn link_content_measures_at_compact_24pt_single_line_height() {
        let mut link = LinkContent::new("Details");
        let measured = link.measure(&mut DummyText, Size::new(200., 100.)).size;
        assert_eq!(measured.height, 24.0);
    }
}
