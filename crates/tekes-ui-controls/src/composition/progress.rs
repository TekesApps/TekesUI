//! Passive Progress over the original scalar model and presentation View.
use super::*;
use crate::{
    extended::Progress,
    feedback_view::{ProgressView, ViewLayout},
};

/// A visible, noninteractive progress indicator with token-based intrinsic bar height.
/// Label and host loading/error/success text remain ordinary siblings.
/// Unknown progress is explicit and never reports the retained scalar as 0%.
#[derive(Clone, Debug)]
pub struct ProgressContent {
    model: Progress,
    name: String,
    description: Option<String>,
    view: ProgressView,
    height: Option<f32>,
    max_width: Option<f32>,
    indeterminate_label: Option<String>,
}
impl ProgressContent {
    pub const DEFAULT_HEIGHT: f32 = tekes_ui_tokens::Spacing::Relaxed.value();

    pub fn new(name: impl Into<String>, value: f32) -> Self {
        Self {
            model: Progress::new(value),
            name: name.into(),
            description: None,
            view: ProgressView::default(),
            height: None,
            max_width: None,
            indeterminate_label: None,
        }
    }
    pub fn value(&self) -> f32 {
        self.model.value()
    }
    pub fn set_value(&mut self, value: f32) -> bool {
        self.model.set(value)
    }
    pub fn is_indeterminate(&self) -> bool {
        self.model.indeterminate
    }
    pub fn with_indeterminate(mut self, indeterminate: bool) -> Self {
        self.model.indeterminate = indeterminate;
        self
    }
    pub fn set_indeterminate(&mut self, indeterminate: bool) {
        self.model.indeterminate = indeterminate;
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }
    pub fn model(&self) -> Progress {
        self.model
    }
    /// Override the bar height when an enclosing layout needs a non-default height.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
    /// Cap the intrinsic measured width while preserving parent shrinking bounds.
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }
    /// Customize the accessible value reported when indeterminate is true.
    pub fn with_indeterminate_label(mut self, label: impl Into<String>) -> Self {
        self.indeterminate_label = Some(label.into());
        self
    }
    pub fn layout(&self) -> ViewLayout {
        self.view.layout()
    }
}
impl Component for ProgressContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    #[cfg(feature = "inspection")]
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", "ProgressContent"),
            InspectionProperty::new("name", &self.name),
            InspectionProperty::new("value", self.model.value().to_string()),
            InspectionProperty::new("indeterminate", self.model.indeterminate.to_string()),
        ])
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let avail = clean_size(available);
        let width = self
            .max_width
            .filter(|w| w.is_finite() && *w >= 0.)
            .map_or(avail.width, |w| avail.width.min(w));
        let target_h = self
            .height
            .filter(|h| h.is_finite() && *h >= 0.)
            .unwrap_or(Self::DEFAULT_HEIGHT);
        let height = if avail.height > 0. {
            target_h.min(avail.height)
        } else {
            0.
        };
        MeasuredChild::new(Size::new(width, height))
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
            self.indeterminate_label
                .clone()
                .unwrap_or_else(|| "Indeterminate".into())
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
                Some(if unknown { "Indeterminate" } else { "45%" })
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
        let custom = progress.with_indeterminate_label("Pending");
        assert_eq!(custom.semantics().value.as_deref(), Some("45%"));
        let mut custom_unknown = custom;
        custom_unknown.set_indeterminate(true);
        assert_eq!(custom_unknown.semantics().value.as_deref(), Some("Pending"));
    }

    #[test]
    fn progress_has_token_intrinsic_height_under_unbounded_scroll_budget() {
        struct NoText;
        impl ButtonTextRenderer for NoText {
            fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
                Size::default()
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
        let mut bar = ProgressContent::new("Download", 0.5);
        let mut text = NoText;
        let measured = bar.measure(&mut text, Size::new(240., 1e8));
        assert_eq!(
            measured.size,
            Size::new(240., ProgressContent::DEFAULT_HEIGHT)
        );
        assert_eq!(
            ProgressContent::new("Narrow", 0.5)
                .with_max_width(120.)
                .with_height(18.)
                .measure(&mut text, Size::new(240., 1e8))
                .size,
            Size::new(120., 18.)
        );
    }
}
