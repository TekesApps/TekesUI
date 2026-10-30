//! Passive Progress over the original scalar model and presentation View.
use super::*;
use crate::{
    extended::Progress,
    feedback_view::{ProgressView, ViewLayout},
};

/// A visible, noninteractive progress indicator. The parent must allocate a finite bar
/// extent; label and host loading/error/success text remain ordinary siblings.
/// Unknown progress is explicit and never reports the retained scalar as 0%.
#[derive(Clone, Debug)]
pub struct ProgressContent {
    pub model: Progress,
    pub name: String,
    pub description: Option<String>,
    view: ProgressView,
}
impl ProgressContent {
    pub fn new(name: impl Into<String>, value: f32) -> Self {
        Self {
            model: Progress::new(value),
            name: name.into(),
            description: None,
            view: ProgressView::default(),
        }
    }
    pub fn layout(&self) -> ViewLayout {
        self.view.layout()
    }
}
impl Component for ProgressContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(clean_size(available))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Surface");
        self.view
            .render(scene, self.model, frame, Some(clip), theme.palette);
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::ProgressIndicator, &self.name);
        semantics.value = Some(if self.model.indeterminate {
            "进度未知".into()
        } else {
            format!("{}%", self.model.value() * 100.)
        });
        semantics.description = self.description.clone();
        semantics
    }
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        (!self.model.indeterminate).then_some(0.0..=100.0)
    }
    // Numeric metadata is read-only. Routing, focus and text services stay passive.
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;

    #[test]
    fn progress_metadata_is_passive_across_known_unknown_known() {
        let mut progress = ProgressContent::new("Transfer progress", 0.45);
        progress.description = Some("Download".into());
        for unknown in [false, true, false] {
            progress.model.indeterminate = unknown;
            let semantics = progress.semantics();
            assert_eq!(semantics.role, ContentRole::ProgressIndicator);
            assert_eq!(semantics.name, "Transfer progress");
            assert_eq!(semantics.description.as_deref(), Some("Download"));
            assert_eq!(
                semantics.value.as_deref(),
                Some(if unknown { "进度未知" } else { "45%" })
            );
            assert_eq!(progress.value_range(), (!unknown).then_some(0.0..=100.0));
            assert!(!progress.focusable());
            assert!(!progress.interactive());
            assert!(!progress.requires_text_services());
            assert!(progress.text_surface().is_none());
            for action in [
                AccessibilityAction::Press,
                AccessibilityAction::Focus,
                AccessibilityAction::SetValue("50".into()),
                AccessibilityAction::Increment,
                AccessibilityAction::Decrement,
            ] {
                assert_eq!(
                    progress.accessibility_action(None, &action),
                    ContentResponse::default()
                );
            }
        }
    }
}
