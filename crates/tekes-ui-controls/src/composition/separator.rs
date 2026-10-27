//! Passive ordinary Content adapter over the original SeparatorView painter.
use super::*;
use crate::{
    extended::Separator,
    layout_view::{SeparatorLayout, SeparatorView},
};

/// Decorative separator. Parent constraints own logical extent/inset; the
/// original View owns theme ink and backing-scale hairline policy. This leaf
/// never participates in actions, focus, accessibility or text/scroll ownership.
#[derive(Clone, Copy, Debug)]
pub struct SeparatorContent {
    pub model: Separator,
    view: SeparatorView,
}
impl SeparatorContent {
    pub fn new(axis: Axis) -> Self {
        Self {
            model: Separator::new(axis),
            view: SeparatorView::default(),
        }
    }
    pub fn layout(&self) -> SeparatorLayout {
        self.view.layout()
    }
}
impl Default for SeparatorContent {
    fn default() -> Self {
        Self::new(Axis::Horizontal)
    }
}
impl Component for SeparatorContent {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, calls: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(calls)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }

    fn semantic_hidden(&self) -> bool {
        true
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let available = clean_size(available);
        let extent =
            proposal::metric("stroke", "stroke-rule").expect("Separator logical extent token");
        MeasuredChild::new(match self.model.axis {
            Axis::Horizontal => Size::new(available.width, extent.min(available.height)),
            Axis::Vertical => Size::new(extent.min(available.width), available.height),
        })
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        // Preserve the original View's ancestor clip/renderer quantization for
        // ordinary allocations. A fractional one-point leaf clip can quantize
        // to zero pixels even when the original line fits its logical frame.
        // Only overflowing decorative ink needs the additional allocation clip;
        // its width remains the exact original painter policy.
        let (ink_width, _) = crate::paint::decorative_border(theme.palette, text.backing_scale());
        let extent = match self.model.axis {
            Axis::Horizontal => frame.size.height,
            Axis::Vertical => frame.size.width,
        };
        let clip_overflow = ink_width > extent;
        if clip_overflow {
            scene.push_clip(clean_rect(frame));
        }
        self.view.render_at_scale(
            scene,
            frame,
            self.model.axis,
            Some(clip),
            theme.palette,
            text.backing_scale(),
        );
        if clip_overflow {
            scene.pop_clip();
        }
    }
}
