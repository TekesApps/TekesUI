//! Static passive Content over the original SkeletonView.
use super::*;
use crate::feedback_view::{SkeletonView, ViewLayout};

/// Parent-owned finite allocation; no intrinsic height or animation clock.
/// Decorative by default. A caller may opt into a named Group; loading/result
/// messages remain ordinary host-controlled siblings.
#[derive(Clone, Debug, Default)]
pub struct SkeletonContent {
    view: SkeletonView,
    name: Option<String>,
    description: Option<String>,
}
impl SkeletonContent {
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
    fn semantic_hidden(&self) -> bool {
        self.name.is_none()
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        // ContentHost supplies its existing cleaned allocation. An embedding
        // host must validate finite nonnegative dimensions before submitting it.
        MeasuredChild::new(clean_size(available))
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
