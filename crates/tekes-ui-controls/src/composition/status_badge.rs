//! Passive solid badge; the original View is the sole painting/layout authority.
use super::*;
use crate::{
    extended::StatusBadge,
    feedback_view::{StatusBadgeView, ViewLayout},
};
#[derive(Clone, Debug)]
pub struct StatusBadgeContent {
    model: StatusBadge,
    label: String,
    name: Option<String>,
    description: Option<String>,
    decorative: bool,
    view: StatusBadgeView,
}
impl StatusBadgeContent {
    pub fn new(model: StatusBadge, label: impl Into<String>) -> Result<Self, CompositionError> {
        let next = Self {
            model,
            label: label.into(),
            name: None,
            description: None,
            decorative: false,
            view: StatusBadgeView::default(),
        };
        next.validate()?;
        Ok(next)
    }
    pub fn decorative(model: StatusBadge, label: impl Into<String>) -> Self {
        Self {
            model,
            label: label.into(),
            name: None,
            description: None,
            decorative: true,
            view: StatusBadgeView::default(),
        }
    }
    pub fn model(&self) -> StatusBadge {
        self.model
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn layout(&self) -> ViewLayout {
        self.view.layout()
    }
    /// Validate the complete proposed state before replacing any retained field.
    pub fn update(
        &mut self,
        model: StatusBadge,
        label: impl Into<String>,
        name: Option<String>,
        description: Option<String>,
        decorative: bool,
    ) -> Result<(), CompositionError> {
        let next = Self {
            model,
            label: label.into(),
            name,
            description,
            decorative,
            view: self.view,
        };
        next.validate()?;
        *self = next;
        Ok(())
    }
}
impl Component for StatusBadgeContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if (!self.decorative && self.label.trim().is_empty())
            || self.name.as_ref().is_some_and(|s| s.trim().is_empty())
        {
            return Err(CompositionError::EmptyAccessibleName);
        }
        Ok(())
    }
    fn semantic_hidden(&self) -> bool {
        self.decorative
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        // Host finite-parent precondition; standalone callers can use the
        // fallible original View measure/render entry points before mounting.
        let _ = StatusBadgeView::measure_label(text, &self.label, available);
        MeasuredChild::new(available)
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
        self.view
            .render_labelled(
                scene,
                text,
                self.model,
                &self.label,
                frame,
                frame,
                Some(clip),
                theme.palette,
            )
            .expect("finite parent allocation");
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(
            ContentRole::Text,
            self.name.as_deref().unwrap_or(&self.label),
        );
        s.description = self.description.clone();
        s
    }
    // Default events, focus, editor, range and animation remain passive.
}
