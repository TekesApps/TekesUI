//! Passive Progress over the original scalar model and presentation View.
use super::*;
use crate::{
    extended::Progress,
    feedback_view::{ProgressView, ViewLayout},
};

/// A visible, noninteractive Group. The parent must allocate a finite bar
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
        let mut semantics = ContentSemantics::new(ContentRole::Group, &self.name);
        semantics.value = Some(if self.model.indeterminate {
            "进度未知".into()
        } else {
            format!("{}%", self.model.value() * 100.)
        });
        semantics.description = self.description.clone();
        semantics
    }
    // Default routing, focus, text services and value_range remain passive.
}
