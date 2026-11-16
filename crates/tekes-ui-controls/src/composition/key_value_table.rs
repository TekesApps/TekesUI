//! Paired ordinary Content cells with shared property-table layout and chrome.
use super::*;
use crate::layout_view::SeparatorView;
use tekes_ui_tokens::Spacing;

/// A finite key/value table. Keys occupy one third of the usable width and values
/// the remainder. Children keep their own text, focus and accessibility owners.
/// Group headings and scrolling belong to the enclosing composition.
#[derive(Debug)]
pub struct KeyValueTableContent {
    cells: Vec<Content>,
    arranged: bool,
}
impl KeyValueTableContent {
    pub fn new(rows: impl IntoIterator<Item = (Content, Content)>) -> Self {
        Self {
            cells: rows
                .into_iter()
                .flat_map(|(key, value)| [key, value])
                .collect(),
            arranged: false,
        }
    }
    /// Alternating key/value children in row order, including their stable IDs.
    pub fn cells(&self) -> &[Content] {
        &self.cells
    }
    fn columns(width: f32) -> (f32, f32, f32) {
        let padding = Spacing::Micro.value().min(width / 4.);
        let usable = (width - 4. * padding).max(0.);
        (padding, usable / 3., usable * 2. / 3.)
    }
    fn row_heights(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> Vec<f32> {
        let (padding, key, value) = Self::columns(available.width);
        self.cells
            .chunks_exact_mut(2)
            .map(|row| {
                let key_height = row[0]
                    .measure(text, Size::new(key, available.height))
                    .size
                    .height;
                let value_height = row[1]
                    .measure(text, Size::new(value, available.height))
                    .size
                    .height;
                key_height.max(value_height) + 2. * padding
            })
            .collect()
    }
}
impl Component for KeyValueTableContent {
    fn host_reveal_passthrough(&self) -> bool {
        // After arrangement, ancestor scrolling only translates these ordinary
        // child frames. No local clip, overlay or alternate text owner is added.
        // This opt-in lets Host prepare focus/reveal for the original cells.
        self.arranged
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(
        &self,
        hooks: &[PreparedHook],
    ) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, hooks))
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn validate(&self) -> Result<(), CompositionError> {
        // Component::children_mut is public: reject a consumer removing only one
        // cell, instead of silently dropping its unpaired sibling during layout.
        if !self.cells.len().is_multiple_of(2) {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }
    fn children(&self) -> &[Content] {
        &self.cells
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.cells)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        let available = clean_size(available);
        MeasuredChild::new(Size::new(
            available.width,
            self.row_heights(text, available).iter().sum(),
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let (padding, key, value) = Self::columns(frame.size.width);
        let heights = self.row_heights(text, frame.size);
        let mut y = frame.min_y();
        for (row, height) in self.cells.chunks_exact_mut(2).zip(heights) {
            let h = height - 2. * padding;
            row[0].arrange(
                text,
                Rect::new(frame.min_x() + padding, y + padding, key, h),
                clip,
            );
            row[1].arrange(
                text,
                Rect::new(frame.min_x() + key + 3. * padding, y + padding, value, h),
                clip,
            );
            y += height;
        }
        self.arranged = !frame.is_empty();
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let (padding, key, _) = Self::columns(frame.size.width);
        let extent =
            proposal::metric("stroke", "stroke-rule").expect("Separator logical extent token");
        let mut separator = SeparatorView::default();
        for row in self.cells.chunks_exact(2) {
            let bottom = row[0].frame().max_y() + padding;
            separator.render_at_scale(
                scene,
                Rect::new(frame.min_x(), bottom - extent, frame.size.width, extent),
                Axis::Horizontal,
                Some(clip),
                theme.palette,
                text.backing_scale(),
            );
        }
        separator.render_at_scale(
            scene,
            Rect::new(
                frame.min_x() + key + 2. * padding,
                frame.min_y(),
                extent,
                frame.size.height,
            ),
            Axis::Vertical,
            Some(clip),
            theme.palette,
            text.backing_scale(),
        );
    }
}
