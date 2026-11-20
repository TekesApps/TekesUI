//! Static passive Content over the original SkeletonView.
use super::*;
use crate::feedback_view::{SkeletonView, ViewLayout};

/// Passive skeleton placeholder with token-based intrinsic height; no animation clock.
/// Decorative by default. A caller may opt into a named Group; loading/result
/// messages remain ordinary host-controlled siblings.
#[derive(Clone, Debug, Default)]
pub struct SkeletonContent {
    view: SkeletonView,
    name: Option<String>,
    description: Option<String>,
    height: Option<f32>,
    max_width: Option<f32>,
}
impl SkeletonContent {
    pub const DEFAULT_HEIGHT: f32 = tekes_ui_tokens::Spacing::Loose.value();

    /// Override the intrinsic height when an embedding layout needs a custom placeholder height.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
    /// Cap the intrinsic measured width while preserving parent shrinking bounds.
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }
    pub fn layout(&self) -> ViewLayout {
        self.view.layout()
    }
    /// Atomically switch decoration/Group semantics. Invalid names preserve
    /// the previous name, description and mounted component identity.
    pub fn set_accessibility(
        &mut self,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<(), CompositionError> {
        if name.as_ref().is_some_and(|name| name.trim().is_empty()) {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.name = name;
        self.description = description;
        Ok(())
    }
}
impl Component for SkeletonContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn semantic_hidden(&self) -> bool {
        self.name.is_none()
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
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
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Surface");
        self.view.render(scene, frame, Some(clip), theme.palette);
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics =
            ContentSemantics::new(ContentRole::Group, self.name.clone().unwrap_or_default());
        semantics.description = self.description.clone();
        semantics
    }
    // Default routes own no action, focus, text service or range.
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn skeleton_has_token_intrinsic_height_under_unbounded_scroll_budget() {
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
        let mut skel = SkeletonContent::default();
        let mut text = NoText;
        assert_eq!(
            skel.measure(&mut text, Size::new(240., 1e8)).size,
            Size::new(240., SkeletonContent::DEFAULT_HEIGHT)
        );
        assert_eq!(
            SkeletonContent::default()
                .with_max_width(120.)
                .with_height(20.)
                .measure(&mut text, Size::new(240., 1e8))
                .size,
            Size::new(120., 20.)
        );
    }
}
