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
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
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
        MeasuredChild::new(
            StatusBadgeView::measure_capsule(text, &self.label, clean_size(available))
                .unwrap_or_default(),
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
        let label_frame = StatusBadgeView::capsule_label_frame(frame);
        self.view
            .render_labelled(
                scene,
                text,
                self.model,
                &self.label,
                frame,
                label_frame,
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

/// Passive neutral badge sharing `UiCaption` text metrics and inherited Surface frame tokens.
#[derive(Clone, Debug)]
pub struct BadgeContent {
    label: String,
    name: Option<String>,
    description: Option<String>,
    decorative: bool,
}

impl BadgeContent {
    pub fn new(label: impl Into<String>) -> Result<Self, CompositionError> {
        let next = Self {
            label: label.into(),
            name: None,
            description: None,
            decorative: false,
        };
        next.validate()?;
        Ok(next)
    }

    pub fn decorative(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            name: None,
            description: None,
            decorative: true,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// Validate the complete proposed state before replacing any retained field.
    pub fn update(
        &mut self,
        label: impl Into<String>,
        name: Option<String>,
        description: Option<String>,
        decorative: bool,
    ) -> Result<(), CompositionError> {
        let next = Self {
            label: label.into(),
            name,
            description,
            decorative,
        };
        next.validate()?;
        *self = next;
        Ok(())
    }
}

impl Component for BadgeContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
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
        let available = clean_size(available);
        let pad_x = tekes_ui_tokens::Spacing::Tight.value();
        let pad_y = tekes_ui_tokens::Spacing::Micro.value();
        let measured = text.measure(
            &self.label,
            proposal::TextRole::UiCaption,
            (available.width - pad_x * 2.).max(0.),
        );
        MeasuredChild::new(Size::new(
            (measured.width + pad_x * 2.).min(available.width).max(0.),
            (measured.height + pad_y * 2.).min(available.height).max(0.),
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
        let Some(visible) = frame.intersection(&clip).filter(|r| !r.is_empty()) else {
            return;
        };
        let colors = tekes_ui_tokens::surface_palette::resolve_surface(
            theme.palette,
            theme.surface(),
            "surface-frame",
            "rest",
        )
        .expect("validated BadgeContent Surface packet");
        let radius = proposal::metric("radius", "radius-badge")
            .unwrap_or(4.)
            .min(frame.size.width.min(frame.size.height) * 0.5);
        let pad_x = tekes_ui_tokens::Spacing::Tight.value();
        let pad_y = tekes_ui_tokens::Spacing::Micro.value();
        let ix = pad_x.min(frame.size.width.max(0.) * 0.5);
        let iy = pad_y.min(frame.size.height.max(0.) * 0.5);
        let label_frame = Rect::new(
            frame.min_x() + ix,
            frame.min_y() + iy,
            (frame.size.width - ix * 2.).max(0.),
            (frame.size.height - iy * 2.).max(0.),
        );
        scene.push_clip(visible);
        let mut quad =
            tekes_ui_scene::Quad::new(frame, colors.background).with_corner_radius(radius);
        if let Some(boundary) = colors.boundary {
            let (width, _) = crate::paint::decorative_border(theme.palette, text.backing_scale());
            quad = quad.with_border(width, boundary);
        }
        scene.push_quad(quad);
        if !self.label.is_empty() && !label_frame.is_empty() {
            text.draw(
                scene,
                &self.label,
                proposal::TextRole::UiCaption,
                label_frame,
                ButtonTextAlign::Center,
                theme.foreground.unwrap_or(colors.text),
            );
        }
        scene.pop_clip();
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(
            ContentRole::Text,
            self.name.as_deref().unwrap_or(&self.label),
        );
        s.description = self.description.clone();
        s
    }
}
