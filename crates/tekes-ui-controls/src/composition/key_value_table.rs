//! Paired ordinary Content cells with shared property-table layout and chrome.
use super::*;
use crate::layout_view::SeparatorView;
use tekes_ui_tokens::Spacing;

#[derive(Debug)]
struct MeasuredRows {
    width: f32,
    signature: Vec<(ComponentId, u64)>,
    heights: Vec<f32>,
}

/// A finite key/value table. Keys occupy one third of the usable width and values
/// the remainder. Children keep their own text, focus and accessibility owners.
/// Group headings and scrolling belong to the enclosing composition.
#[derive(Debug)]
pub struct KeyValueTableContent {
    cells: Vec<Content>,
    arranged_signature: Option<Vec<(ComponentId, u64)>>,
    measured: Option<MeasuredRows>,
}
impl KeyValueTableContent {
    pub fn new(rows: impl IntoIterator<Item = (Content, Content)>) -> Self {
        Self {
            cells: rows
                .into_iter()
                .flat_map(|(key, value)| [key, value])
                .collect(),
            arranged_signature: None,
            measured: None,
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
        // Verify cell identities/mounts still match the arranged layout so any
        // external mutation through `children_mut` invalidates passthrough cleanly.
        self.arranged_signature.as_ref().is_some_and(|sig| {
            sig.len() == self.cells.len()
                && !self.cells.is_empty()
                && self.cells.iter().zip(sig).all(|(cell, &(id, mount))| {
                    cell.id == id && cell.mount == mount && !cell.frame().is_empty()
                })
        })
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged_signature = None;
        self.measured = None;
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn ignores_pointer_actions(&self) -> bool {
        true
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
        self.arranged_signature = None;
        let available = clean_size(available);
        let heights = self.row_heights(text, available);
        let total = heights.iter().sum();
        let signature = self
            .cells
            .iter()
            .map(|cell| (cell.id, cell.mount))
            .collect();
        self.measured = Some(MeasuredRows {
            width: available.width,
            signature,
            heights,
        });
        MeasuredChild::new(Size::new(available.width, total))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let (padding, key, value) = Self::columns(frame.size.width);
        let heights = match self.measured.take() {
            Some(m)
                if (m.width - frame.size.width).abs() <= f32::EPSILON
                    && m.signature.len() == self.cells.len()
                    && self
                        .cells
                        .iter()
                        .zip(&m.signature)
                        .all(|(cell, &(id, mount))| cell.id == id && cell.mount == mount) =>
            {
                m.heights
            }
            _ => self.row_heights(text, frame.size),
        };
        let child_clip = frame.intersection(&clip).unwrap_or_default();
        let mut y = frame.min_y();
        for (row, height) in self.cells.chunks_exact_mut(2).zip(heights) {
            let h = height - 2. * padding;
            row[0].arrange(
                text,
                Rect::new(frame.min_x() + padding, y + padding, key, h),
                child_clip,
            );
            row[1].arrange(
                text,
                Rect::new(frame.min_x() + key + 3. * padding, y + padding, value, h),
                child_clip,
            );
            y += height;
        }
        self.arranged_signature = (!frame.is_empty()).then(|| {
            self.cells
                .iter()
                .map(|cell| (cell.id, cell.mount))
                .collect()
        });
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let Some(paint_clip) = frame.intersection(&clip) else {
            return;
        };
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
                Some(paint_clip),
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
            Some(paint_clip),
            theme.palette,
            text.backing_scale(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Debug)]
    struct CountingCell {
        measures: Rc<Cell<usize>>,
        last_clip: Rc<Cell<Option<Rect>>>,
    }
    impl Component for CountingCell {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
            self.measures.set(self.measures.get() + 1);
            MeasuredChild::new(Size::new(a.width, 20.))
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
            self.last_clip.set(Some(clip));
        }
        fn paint(
            &mut self,
            _: &mut Scene,
            _: &mut dyn ButtonTextRenderer,
            _: Rect,
            _: Rect,
            _: ContentTheme,
        ) {
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Text, "cell")
        }
    }

    struct DummyText;
    impl ButtonTextRenderer for DummyText {
        fn measure(&mut self, s: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(s.len() as f32 * 8., 16.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }

    #[test]
    fn key_value_table_reuses_measured_heights_and_invalidates_arranged_on_children_mut() {
        let measures = Rc::new(Cell::new(0));
        let last_clip = Rc::new(Cell::new(None));
        let mut table = KeyValueTableContent::new([(
            Content::new(
                ComponentId(1),
                CountingCell {
                    measures: measures.clone(),
                    last_clip: last_clip.clone(),
                },
            ),
            Content::new(
                ComponentId(2),
                CountingCell {
                    measures: measures.clone(),
                    last_clip: Rc::new(Cell::new(None)),
                },
            ),
        )]);
        let size = Size::new(200., 100.);
        table.measure(&mut DummyText, size);
        assert_eq!(measures.get(), 2);
        let frame = Rect::new(0., 0., 200., 40.);
        table.arrange(&mut DummyText, frame, Rect::new(0., 0., 500., 500.));
        assert_eq!(measures.get(), 2, "arrange must reuse measure heights");
        assert!(table.host_reveal_passthrough());
        assert_eq!(last_clip.get(), Some(frame));
        // Non-structural traversal via `children_mut()` preserves passthrough.
        let _ = table.children_mut().unwrap()[0].component::<CountingCell>();
        assert!(table.host_reveal_passthrough());
        // Replacing a cell invalidates passthrough and forces re-measurement.
        table.children_mut().unwrap()[0] = Content::new(
            ComponentId(3),
            CountingCell {
                measures: measures.clone(),
                last_clip: last_clip.clone(),
            },
        );
        assert!(!table.host_reveal_passthrough());
    }
}
