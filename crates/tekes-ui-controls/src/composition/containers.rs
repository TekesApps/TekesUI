use super::*;
use crate::extended::{Axis, Grid, Stack};
use crate::layout::{AlignmentOverride, MeasuredChild, layout_measured_children};
use crate::{ButtonTextAlign, ButtonTextRenderer, ScrollArea};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::proposal;

#[derive(Debug)]
/// Intrinsic stack: children keep measured main-axis sizes. Like existing
/// layout primitives it does not introduce a viewport; use ScrollContent or
/// the host clip to bound overflow. No implicit flex/shrink or per-child frames.
pub struct StackContent {
    pub model: Stack,
    pub children: Vec<Content>,
    /// One child is remeasured in the main-axis space left by fixed siblings.
    /// This expresses e.g. wrapping text beside an intrinsic icon and badge.
    pub flexible_child: Option<ComponentId>,
}
impl StackContent {
    /// Canonical inter-section gap in scrolled document pages (`Spacing::Section` = 24 pt).
    pub const DOCUMENT_SECTION_GAP: f32 = tekes_ui_tokens::Spacing::Section.value();
    /// Canonical inter-group gap in side panels and Inspector (`Spacing::Loose` = 16 pt).
    pub const PANEL_SECTION_GAP: f32 = tekes_ui_tokens::Spacing::Loose.value();
    /// Canonical gap between a section heading and its section body (`Spacing::Regular` = 8 pt).
    pub const SECTION_HEADER_GAP: f32 = tekes_ui_tokens::Spacing::Regular.value();
    /// Canonical gap between a specimen caption label and its control (`Spacing::Tight` = 4 pt).
    pub const SPECIMEN_LABEL_GAP: f32 = tekes_ui_tokens::Spacing::Tight.value();

    /// Compose document sections separated by `DOCUMENT_SECTION_GAP` (24 pt).
    pub fn document_sections(id: ComponentId, sections: Vec<Content>) -> Content {
        Content::new(
            id,
            Self {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Self::DOCUMENT_SECTION_GAP,
                },
                children: sections,
                flexible_child: None,
            },
        )
    }

    /// Compose panel/Inspector sections separated by `PANEL_SECTION_GAP` (16 pt).
    pub fn panel_sections(id: ComponentId, sections: Vec<Content>) -> Content {
        Content::new(
            id,
            Self {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Self::PANEL_SECTION_GAP,
                },
                children: sections,
                flexible_child: None,
            },
        )
    }

    /// Group a `UiHeading` title with its section body using `SECTION_HEADER_GAP` (8 pt).
    pub fn titled_section(
        group_id: ComponentId,
        heading_id: ComponentId,
        title: impl Into<String>,
        body: Content,
    ) -> Content {
        Content::new(
            group_id,
            Self {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Self::SECTION_HEADER_GAP,
                },
                children: vec![Content::new(heading_id, TextContent::heading(title)), body],
                flexible_child: None,
            },
        )
    }

    /// Group a `UiCaption` label with its specimen control using `SPECIMEN_LABEL_GAP` (4 pt).
    pub fn labeled_specimen(
        group_id: ComponentId,
        label_id: ComponentId,
        label: impl Into<String>,
        specimen: Content,
    ) -> Content {
        Content::new(
            group_id,
            Self {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Self::SPECIMEN_LABEL_GAP,
                },
                children: vec![
                    Content::new(label_id, TextContent::caption(label)),
                    specimen,
                ],
                flexible_child: None,
            },
        )
    }

    /// Inset this Stack's original children without introducing a viewport or
    /// changing their mount identities. Bare StackContent literals keep zero padding.
    pub fn into_content_with_padding(
        self,
        id: ComponentId,
        padding: crate::layout::Insets,
    ) -> Content {
        let mut content = Content::new(id, self);
        content.stack_padding = Some(padding);
        content
    }
    /// Declare a Stack that occupies the available main axis and distributes
    /// its children within it. Intrinsic measurement keeps the natural size.
    /// The original Stack component, children and mount identities are retained.
    pub fn into_content_with_distribution(
        self,
        id: ComponentId,
        distribution: proposal::layout::MainAxisDistribution,
    ) -> Content {
        let mut content = Content::new(id, self);
        content.stack_distribution = Some(distribution);
        content
    }

    pub(super) fn arrange_with_distribution(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        distribution: Option<proposal::layout::MainAxisDistribution>,
    ) {
        let measured = self.measurements(text, frame.size, false);
        let placements = layout_measured_children(
            frame,
            self.model.axis,
            &measured,
            self.model.gap,
            if self.model.axis == Axis::Horizontal {
                proposal::layout::ROW
            } else {
                proposal::layout::COLUMN
            },
            AlignmentOverride {
                main_axis: distribution,
                ..AlignmentOverride::default()
            },
        );
        for (c, p) in self.children.iter_mut().zip(placements) {
            c.arrange(text, p.frame, clip)
        }
    }

    fn measurements(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
        intrinsic: bool,
    ) -> Vec<MeasuredChild> {
        let mut measured: Vec<_> = self
            .children
            .iter_mut()
            .map(|c| {
                if intrinsic {
                    c.measure_intrinsic(text, available)
                } else {
                    c.measure(text, available)
                }
            })
            .collect();
        if let Some(i) = self
            .flexible_child
            .and_then(|id| self.children.iter().position(|c| c.id == id))
        {
            let horizontal = self.model.axis == Axis::Horizontal;
            let fixed = measured
                .iter()
                .enumerate()
                .filter(|(n, _)| *n != i)
                .map(|(_, m)| {
                    if horizontal {
                        m.size.width
                    } else {
                        m.size.height
                    }
                })
                .sum::<f32>();
            let left = ((if horizontal {
                available.width
            } else {
                available.height
            }) - fixed
                - finite(self.model.gap) * self.children.len().saturating_sub(1) as f32)
                .max(0.);
            let constrained = if horizontal {
                Size::new(left, available.height)
            } else {
                Size::new(available.width, left)
            };
            measured[i] = if intrinsic {
                self.children[i].measure_intrinsic(text, constrained)
            } else {
                self.children[i].measure(text, constrained)
            };
        }
        measured
    }
    fn measure_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
        intrinsic: bool,
    ) -> MeasuredChild {
        let horizontal = self.model.axis == Axis::Horizontal;
        let mut main = 0_f32;
        let mut cross = 0_f32;
        for m in self.measurements(text, available, intrinsic) {
            let m = m.size;
            main += (if horizontal { m.width } else { m.height }).min(1e8);
            cross = cross.max(if horizontal { m.height } else { m.width });
        }
        main += finite(self.model.gap) * self.children.len().saturating_sub(1) as f32;
        MeasuredChild::new(if horizontal {
            Size::new(main, cross)
        } else {
            Size::new(cross, main)
        })
    }
}
impl Component for StackContent {
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
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.measure_content(text, available, false)
    }
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.measure_content(text, available, true)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arrange_with_distribution(text, frame, clip, None);
    }
}
#[derive(Debug)]
pub struct GridContent {
    pub model: Grid,
    pub children: Vec<Content>,
}
impl Component for GridContent {
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
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(
            Rect::new(0., 0., available.width, available.height),
            self.children.len(),
            0.,
        );
        let mut rows = vec![0_f32; self.children.len().div_ceil(columns)];
        for (i, (c, cell)) in self.children.iter_mut().zip(cells).enumerate() {
            rows[i / columns] = rows[i / columns].max(
                c.measure(text, Size::new(cell.size.width, available.height))
                    .size
                    .height,
            )
        }
        MeasuredChild::new(Size::new(
            available.width,
            rows.iter().sum::<f32>()
                + finite(self.model.gap_y) * rows.len().saturating_sub(1) as f32,
        ))
    }
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(
            Rect::new(0., 0., available.width, available.height),
            self.children.len(),
            0.,
        );
        let width = self
            .children
            .iter_mut()
            .zip(cells)
            .map(|(child, cell)| {
                child
                    .measure_intrinsic(text, Size::new(cell.size.width, available.height))
                    .size
                    .width
            })
            .fold(0_f32, f32::max);
        let gap =
            finite(self.model.gap_x).min(available.width / columns.saturating_sub(1).max(1) as f32);
        let width =
            (width * columns as f32 + gap * columns.saturating_sub(1) as f32).min(available.width);
        // Preserve the original equal-column allocation and remeasure its rows
        // at the receiving width. No alternative Grid arrangement is introduced.
        self.measure(text, Size::new(width, available.height))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let columns = self.model.columns.max(1);
        let cells = self.model.layout(frame, self.children.len(), 0.);
        let mut heights = vec![0_f32; self.children.len().div_ceil(columns)];
        for (i, (c, cell)) in self.children.iter_mut().zip(&cells).enumerate() {
            heights[i / columns] = heights[i / columns].max(
                c.measure(text, Size::new(cell.size.width, frame.size.height))
                    .size
                    .height,
            )
        }
        let mut y = frame.min_y();
        for (i, (c, cell)) in self.children.iter_mut().zip(cells).enumerate() {
            if i > 0 && i % columns == 0 {
                y += heights[i / columns - 1] + finite(self.model.gap_y)
            }
            c.arrange(
                text,
                Rect::new(cell.min_x(), y, cell.size.width, heights[i / columns]),
                clip,
            )
        }
    }
}
#[derive(Debug)]
pub struct ScrollContent {
    pub(super) toolbar_tabs: bool,
    pub(super) horizontal: bool,
    pub(super) reveal_horizontal: Option<Rect>,
    pub scroll: ScrollArea,
    pub viewport_height: Option<f32>,
    pub(super) backing_scale: f32,
    pub(super) children: Vec<Content>,
    pub(super) viewport: Rect,
    pub(super) clip: Rect,
    pub(super) scrollbar_pointer: Option<bool>,
    pub(super) pointer_hovered: bool,
    pub(super) content_insets: crate::layout::Insets,
    pub(super) last_frame: Option<std::time::Duration>,
    pub(super) reading: Option<content_reading::ContentReadingState>,
    pub(super) reading_mounted: bool,
}
impl ScrollContent {
    pub(super) fn scroll_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        let base = PreparedEffects::local_work(n)?;
        base.checked_add(match &self.reading {
            Some(r) => content_reading::reader_preparation_work(r, n)
                .ok_or(PreparedEffectsError::CounterCapacity)?,
            None => 0,
        })
        .ok_or(PreparedEffectsError::CounterCapacity)
    }
    pub(super) fn prepare_scroll_action(
        &self,
        request: ComponentActionRequest<'_>,
    ) -> ActionPreparation {
        let prepared = action_preparation::prepare_original(
            self,
            request,
            Self::decide_scroll_pointer,
            Self::apply_scroll_pointer,
            |_| {
                (
                    ContentResponse {
                        consumed: true,
                        changed: true,
                        capture_pointer: true,
                        ..Default::default()
                    },
                    PreparedActionFocus::Preserve,
                )
            },
        );
        match prepared {
            ActionPreparation::Unsupported => action_preparation::ignored_pointer(request),
            ready => ready,
        }
    }
    pub fn new(mut scroll: ScrollArea, child: Content) -> Self {
        // Preserve the existing adapter's effective no-bar presentation.
        // The standalone ScrollArea default is unchanged; Lists opt in below.
        scroll.set_scrollbar_visibility(false, false);
        Self {
            scroll,
            horizontal: false,
            toolbar_tabs: false,
            reveal_horizontal: None,
            viewport_height: None,
            backing_scale: 1.,
            children: vec![child],
            viewport: Rect::default(),
            clip: Rect::default(),
            scrollbar_pointer: None,
            pointer_hovered: false,
            content_insets: crate::layout::Insets::ZERO,
            last_frame: None,
            reading: None,
            reading_mounted: false,
        }
    }
    /// Standard scrollable single-line strip (finite intrinsic cross-axis height
    /// and a bottom horizontal scrollbar) for section/tab navigation bars.
    pub fn horizontal_strip(child: Content) -> Self {
        Self::new(ScrollArea::default(), child)
            .horizontal(true)
            .scrollbar(true)
    }
    /// Standard vertically scrolled document owning a single `Spacing::Loose` (16 pt)
    /// padding contract and a flush edge scrollbar.
    pub fn document(child: Content) -> Self {
        Self::new(ScrollArea::default(), child)
            .scrollbar(true)
            .content_padding(tekes_ui_tokens::Spacing::Loose.value())
    }
    /// Use natural document width and disable the vertical scrolling axis.
    /// The default remains the existing vertically flowing viewport.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self.scroll.set_scroll_policy(
            crate::ScrollPolicy::Auto,
            if horizontal {
                crate::ScrollPolicy::Disabled
            } else {
                crate::ScrollPolicy::Auto
            },
        );
        self
    }
    /// Reveal a document-local horizontal range on the next layout.
    pub fn reveal_horizontal(&mut self, rect: Rect) {
        self.reveal_horizontal = Some(rect);
    }
    /// Show overflow bars without changing wheel, axis policy or reveal range.
    pub fn scrollbar(mut self, visible: bool) -> Self {
        self.scroll.set_scrollbar_visibility(visible, visible);
        if !visible {
            self.scrollbar_pointer = None;
            self.pointer_hovered = false;
        }
        self
    }
    /// Symmetric document padding; the original focus bleed remains a minimum.
    pub fn content_padding(mut self, padding: f32) -> Self {
        let padding = finite(padding);
        self.content_insets = crate::layout::Insets::new(padding, padding, padding, padding);
        self
    }
    /// Asymmetric four-edge document insets; the original focus bleed remains a minimum.
    pub fn content_insets(mut self, insets: crate::layout::Insets) -> Self {
        self.content_insets = crate::layout::Insets::new(
            finite(insets.top),
            finite(insets.right),
            finite(insets.bottom),
            finite(insets.left),
        );
        self
    }
    fn document_gutter(&self, text: &mut dyn ButtonTextRenderer) -> (f32, crate::layout::Insets) {
        let focus = crate::extended::ControlFrame::new(Rect::default(), None)
            .focus_bounds(text.backing_scale());
        let bleed = (-focus.min_x()).max(0.);
        let floor = if self.toolbar_tabs {
            bleed.max(crate::toolbar::tab_geometry(text).0)
        } else {
            bleed
        };
        (
            bleed,
            crate::layout::Insets::new(
                floor.max(self.content_insets.top),
                floor.max(self.content_insets.right),
                floor.max(self.content_insets.bottom),
                floor.max(self.content_insets.left),
            ),
        )
    }
    fn overflow_gutter(&self, bleed: f32, right_gutter: f32) -> f32 {
        if self.scroll.scrollbar_requested(true)
            && self.scroll.scroll_policy().1 != crate::ScrollPolicy::Disabled
        {
            let band = proposal::SCROLLBAR_WIDTH
                + proposal::SCROLLBAR_INSET
                + tekes_ui_tokens::Spacing::Micro.value();
            (band + bleed - right_gutter).max(0.)
        } else {
            0.
        }
    }
    /// Outer width needed for a document's minimum content width. The caller
    /// supplies whether that document overflows its allocated vertical extent;
    /// padding, focus bleed and requested scrollbar space remain owner policy.
    pub fn outer_width_for_content(
        &self,
        text: &mut dyn ButtonTextRenderer,
        width: f32,
        vertical_overflow: bool,
    ) -> f32 {
        let (bleed, insets) = self.document_gutter(text);
        finite(width)
            + insets.left
            + insets.right
            + if vertical_overflow {
                self.overflow_gutter(bleed, insets.right)
            } else {
                0.
            }
    }
    pub(super) fn scrollbar_track(&self, vertical: bool) -> Rect {
        if vertical {
            Rect::new(
                self.viewport.max_x() - proposal::SCROLLBAR_WIDTH - proposal::SCROLLBAR_INSET,
                self.viewport.min_y(),
                proposal::SCROLLBAR_WIDTH,
                self.viewport.size.height,
            )
        } else {
            Rect::new(
                self.viewport.min_x(),
                self.viewport.max_y() - proposal::SCROLLBAR_WIDTH - proposal::SCROLLBAR_INSET,
                self.viewport.size.width,
                proposal::SCROLLBAR_WIDTH,
            )
        }
    }
    fn scrollbar_hit_track(&self, vertical: bool) -> Rect {
        self.scrollbar_track(vertical)
            .inset(-tekes_ui_tokens::Spacing::Micro.value())
            .intersection(&self.clip)
            .unwrap_or_default()
    }
    fn scrollbar_presentation_visible(&self, vertical: bool) -> bool {
        self.scroll.scrollbar_visible(vertical)
            && (self.pointer_hovered || self.scrollbar_pointer == Some(vertical))
    }
    fn update_pointer_hover(&mut self, point: Point, in_curve: bool) -> bool {
        let inside = in_curve && self.viewport.contains(point) && self.clip.contains(point);
        let changed = self.pointer_hovered != inside;
        self.pointer_hovered = inside;
        changed
    }
    fn pointer_move(&mut self, point: Point, in_curve: bool) -> ContentResponse {
        let hover_changed = self.update_pointer_hover(point, in_curve);
        if let Some(vertical) = self.scrollbar_pointer {
            let track = self.scrollbar_hit_track(vertical);
            let moved = self
                .scroll
                .pointer_drag(point, track, Self::minimum_thumb());
            ContentResponse {
                consumed: true,
                changed: moved || hover_changed,
                ..Default::default()
            }
        } else {
            ContentResponse {
                changed: hover_changed,
                ..Default::default()
            }
        }
    }
    pub(super) fn minimum_thumb() -> f32 {
        proposal::metric("size", "size-row-height").expect("existing scrollbar minimum")
    }
    fn decide_scroll_pointer(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedScrollContentPointer> {
        let action_preparation::OwnedActionRequest::Pointer(event) = request else {
            return None;
        };
        let routed = event.routing_event();
        let ContentEvent::PointerDown(point) = routed.as_ref() else {
            return None;
        };
        if !event.is_primary_pointer() || !self.pointer_hovered || !self.clip.contains(*point) {
            return None;
        }
        [true, false].into_iter().find_map(|vertical| {
            self.scroll
                .prepare_pointer_down(
                    *point,
                    vertical,
                    self.scrollbar_hit_track(vertical),
                    Self::minimum_thumb(),
                )
                .map(|scroll| PreparedScrollContentPointer {
                    scroll,
                    vertical,
                    viewport: self.viewport,
                    clip: self.clip,
                })
        })
    }
    fn apply_scroll_pointer(&mut self, decision: PreparedScrollContentPointer) {
        self.scroll.apply_pointer_down(decision.scroll);
        self.scrollbar_pointer = Some(decision.vertical);
        if let Some(reading) = &mut self.reading {
            reading.observe_manual_history();
        }
    }
    fn content_clip(&self, frame: Rect, clip: Rect) -> Rect {
        let mut child_clip = self
            .scroll
            .geometry(frame, Some(clip))
            .visible_viewport_rect()
            .unwrap_or_default();
        if self.scroll.scrollbar_visible(true) {
            let edge = self
                .scrollbar_track(true)
                .inset(-tekes_ui_tokens::Spacing::Micro.value())
                .min_x();
            child_clip.size.width = (child_clip.max_x().min(edge) - child_clip.min_x()).max(0.);
        }
        child_clip
    }
    fn arrange_content(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        insets: crate::layout::Insets,
        width: f32,
        heights: &[f32],
    ) -> Rect {
        let origin = self.scroll.geometry(frame, Some(clip)).content_origin();
        let child_clip = self.content_clip(frame, clip);
        let mut y = origin.y + insets.top;
        for (c, &h) in self.children.iter_mut().zip(heights) {
            c.arrange(
                text,
                Rect::new(origin.x + insets.left, y, width, h),
                child_clip,
            );
            y += h;
        }
        child_clip
    }
}
#[derive(Debug, PartialEq)]
struct PreparedScrollContentPointer {
    scroll: crate::scroll_view::ScrollPointerDecision,
    vertical: bool,
    viewport: Rect,
    clip: Rect,
}
impl action_preparation::OriginalActionDecision for PreparedScrollContentPointer {}
impl Component for ScrollContent {
    fn host_reveal_local_clip(&self) -> Option<Rect> {
        // Publish only the local boundary; the planner intersects the moving
        // ancestor clip, just as arrange does. Freezing that ancestor here would
        // prevent an initially clipped nested viewport from being revealed.
        Some(self.content_clip(self.viewport, self.viewport))
    }
    fn pointer_event_in_region(&mut self, event: &ContentEvent, inside: bool) -> ContentResponse {
        if !event.is_primary_pointer() {
            return event.deliver(self);
        }
        match event.routing_event().as_ref() {
            ContentEvent::PointerMove(point) => self.pointer_move(*point, inside),
            ContentEvent::PointerUp(point) => {
                let hover_changed = self.update_pointer_hover(*point, inside);
                let mut response = event.deliver(self);
                response.changed |= hover_changed;
                response
            }
            _ => event.deliver(self),
        }
    }
    fn reading_scroll_source(&self) -> bool {
        true
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
    fn interactive(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        // A horizontal document uses its natural width (and natural cross-axis
        // height when vertical scrolling is disabled and no explicit height is pinned).
        let mut child_cross = 0_f32;
        let width = if self.horizontal {
            let (_, insets) = self.document_gutter(text);
            let mut document = 0_f32;
            for child in &mut self.children {
                let measured = child
                    .measure_intrinsic(text, Size::new(1e8, available.height))
                    .size;
                document = document.max(measured.width);
                child_cross += measured.height;
            }
            available.width.min(document + insets.left + insets.right)
        } else {
            available.width
        };
        let height = if let Some(h) = self.viewport_height {
            finite(h).min(available.height)
        } else if self.horizontal && self.scroll.scroll_policy().1 == crate::ScrollPolicy::Disabled
        {
            let (_, insets) = self.document_gutter(text);
            let scrollbar_band = if self.scroll.scrollbar_requested(false) {
                proposal::SCROLLBAR_WIDTH + proposal::SCROLLBAR_INSET
            } else {
                0.
            };
            (insets.top + insets.bottom + child_cross + scrollbar_band).min(available.height)
        } else {
            available.height
        };
        MeasuredChild::new(Size::new(width, height))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        if self.viewport != frame || self.clip != frame.intersection(&clip).unwrap_or_default() {
            self.scroll.pointer_up();
            self.scrollbar_pointer = None;
            self.pointer_hovered = false;
        }
        self.backing_scale = text.backing_scale();
        if let Some(m) = text.motion_context() {
            self.scroll.set_reduced_motion(m.reduced_motion);
            if let Some(previous) = self.last_frame {
                self.scroll
                    .tick(m.now.saturating_sub(previous).as_secs_f32());
            }
            self.last_frame = Some(m.now);
            if self.scroll.needs_animation() {
                text.request_animation_frame();
            }
        }
        let reading_anchor = self.reading.as_ref().and_then(|s| s.capture(&self.scroll));
        self.viewport = frame;
        self.clip = frame.intersection(&clip).unwrap_or_default();
        // Scroll is a viewport: reserve the existing ControlFrame focus bleed
        // in document geometry, rather than relaxing the ancestor clip.
        let (bleed, insets) = self.document_gutter(text);
        let mut width = (frame.size.width - insets.left - insets.right).max(0.0);
        if self.horizontal {
            width = self
                .children
                .iter_mut()
                .map(|child| {
                    child
                        .measure_intrinsic(text, Size::new(1e8, frame.size.height))
                        .size
                        .width
                })
                .fold(width, f32::max);
        }
        let available = Size::new(width, 1e8);
        let mut child_heights: Vec<f32> = self
            .children
            .iter_mut()
            .map(|c| c.measure(text, available).size.height)
            .collect();
        let mut height = insets.top + insets.bottom + child_heights.iter().sum::<f32>();
        if self.scroll.scrollbar_requested(true)
            && self.scroll.scroll_policy().1 != crate::ScrollPolicy::Disabled
            && height > frame.size.height
        {
            let extra = self.overflow_gutter(bleed, insets.right);
            width = (width - extra).max(0.);
            let available = Size::new(width, 1e8);
            child_heights = self
                .children
                .iter_mut()
                .map(|child| child.measure(text, available).size.height)
                .collect();
            height = insets.top + insets.bottom + child_heights.iter().sum::<f32>();
        }
        let document = Size::new(
            if self.horizontal {
                width + insets.left + insets.right
            } else {
                frame.size.width
            },
            height,
        );
        if document != self.scroll.document_size() {
            self.scrollbar_pointer = None;
        }
        self.scroll.layout(frame.size, document);
        if let Some(rect) = self.reveal_horizontal.take() {
            let left = rect.min_x() + if self.toolbar_tabs { 0. } else { insets.left };
            let right = rect.max_x()
                + if self.toolbar_tabs {
                    insets.left + insets.right
                } else {
                    insets.right
                };
            let target = if left < self.scroll.offset.x {
                left
            } else if right > self.scroll.offset.x + frame.size.width {
                right - frame.size.width
            } else {
                self.scroll.offset.x
            };
            self.scroll.scroll_by(target - self.scroll.offset.x, 0.);
        }
        if self
            .scrollbar_pointer
            .is_some_and(|vertical| !self.scroll.scrollbar_visible(vertical))
        {
            self.scroll.pointer_up();
            self.scrollbar_pointer = None;
        }
        if let Some(reading) = &mut self.reading {
            reading.prepare(&mut self.scroll);
        }
        let visible = self.arrange_content(text, frame, clip, insets, width, &child_heights);
        let restored = self.reading.as_mut().is_some_and(|reading| {
            reading.restore(&mut self.scroll, &self.children, visible, reading_anchor)
        });
        if restored {
            self.arrange_content(text, frame, clip, insets, width, &child_heights);
        }
        if let Some(reading) = &mut self.reading {
            reading.record(&self.scroll, &self.children, visible);
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        for vertical in [true, false] {
            if self.scrollbar_presentation_visible(vertical) {
                let thumb = self.scroll.thumb(
                    vertical,
                    self.scrollbar_track(vertical),
                    Self::minimum_thumb(),
                );
                if let Some(thumb) = thumb.intersection(&self.clip) {
                    scene.push_quad(
                        tekes_ui_scene::Quad::new(
                            thumb,
                            theme
                                .palette
                                .color("scrollbar-thumb")
                                .expect("scrollbar Theme ink"),
                        )
                        .with_corner_radius(proposal::SCROLLBAR_WIDTH * 0.5),
                    );
                }
            }
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::PointerDown(_) => {
                if let Some(decision) = self.decide_scroll_pointer(
                    &action_preparation::OwnedActionRequest::Pointer(event.clone()),
                ) {
                    self.apply_scroll_pointer(decision);
                    ContentResponse {
                        consumed: true,
                        changed: true,
                        capture_pointer: true,
                        ..Default::default()
                    }
                } else {
                    ContentResponse::default()
                }
            }
            ContentEvent::PointerMove(point) => self.pointer_move(*point, true),
            ContentEvent::PointerUp(_) if self.scrollbar_pointer.is_some() => {
                self.scroll.pointer_up();
                self.scrollbar_pointer = None;
                ContentResponse::consumed(true)
            }
            ContentEvent::Scroll {
                delta,
                precise,
                begin,
                end,
                momentum,
                time,
                ..
            } => {
                // Match the existing TextSurface wheel contract at this input
                // boundary. ScrollArea itself consumes logical point offsets.
                let step = ScrollArea::wheel_step(*precise);
                let delta = Point::new(delta.x * step, delta.y * step);
                if self
                    .reading
                    .as_mut()
                    .is_some_and(|reading| !reading.observe_scroll(delta, *begin, *momentum))
                {
                    return ContentResponse::default();
                }
                let changed = self
                    .scroll
                    .wheel_at(delta, *precise, *begin, *end, *momentum, *time);
                ContentResponse {
                    consumed: changed,
                    changed,
                    ..Default::default()
                }
            }
            ContentEvent::Cancel => {
                self.scroll.cancel();
                self.scrollbar_pointer = None;
                self.pointer_hovered = false;
                self.last_frame = None;
                if let Some(reading) = &mut self.reading {
                    reading.fence_momentum();
                }
                ContentResponse::default()
            }
            _ => ContentResponse::default(),
        }
    }
    fn reveal(&mut self, descendant: Rect) -> Rect {
        let g = self.scroll.geometry(self.viewport, None);
        let origin = g.content_origin();
        let old = g.visual_offset();
        self.scroll
            .reveal_focus(descendant.offset(-origin.x, -origin.y), self.backing_scale);
        let new = self.scroll.visual_offset();
        if new != old {
            self.stop_following();
        }
        descendant.offset(old.x - new.x, old.y - new.y)
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.reading_mounted = event == Lifecycle::Mounted;
        if event == Lifecycle::Unmounted {
            self.last_frame = None;
            self.reading_unmounted();
        }
    }
}

/// The forwarding value keeps existing sized renderer APIs source-compatible.
pub(crate) struct FrameTextAdapter<'a>(
    pub(crate) &'a mut dyn ButtonTextRenderer,
    pub(crate) Option<&'a mut bool>,
    pub(crate) bool,
);
impl ButtonTextRenderer for FrameTextAdapter<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.0.text_surface_host()
    }
    fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
        self.0.measure(t, r, w)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        t: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        i: Rgba,
    ) {
        self.0.draw(s, t, r, f, a, i)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.0.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.2 && self.0.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        if !self.animation_frames_allowed() {
            return;
        }
        if let Some(requested) = &mut self.1 {
            **requested = true;
        }
        self.0.request_animation_frame()
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, t: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.0.first_line_height(t, r, w)
    }
}

pub(crate) struct TextAdapter<'a>(pub(crate) &'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for TextAdapter<'_> {
    fn icon_renderer(&mut self) -> Option<&mut dyn crate::layout_view::IconRenderer> {
        self.0.icon_renderer()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.0.resolved_font_size(role)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn crate::TextSurfaceHost> {
        self.0.text_surface_host()
    }
    fn measure(&mut self, t: &str, r: proposal::TextRole, w: f32) -> Size {
        self.0.measure(t, r, w)
    }
    fn draw(
        &mut self,
        s: &mut Scene,
        t: &str,
        r: proposal::TextRole,
        f: Rect,
        a: ButtonTextAlign,
        i: Rgba,
    ) {
        self.0.draw(s, t, r, f, a, i)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.0.motion_context()
    }
    fn animation_frames_allowed(&self) -> bool {
        self.0.animation_frames_allowed()
    }
    fn request_animation_frame(&mut self) {
        self.0.request_animation_frame()
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn first_line_height(&mut self, t: &str, r: proposal::TextRole, w: f32) -> f32 {
        self.0.first_line_height(t, r, w)
    }
}
#[derive(Debug)]
pub struct TextContent {
    pub text: String,
    pub role: proposal::TextRole,
}
impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            role: proposal::TextRole::UiBody,
        }
    }
    /// Override the typographic role on this text node.
    pub fn with_role(mut self, role: proposal::TextRole) -> Self {
        self.role = role;
        self
    }
    /// Construct heading text (`TextRole::UiHeading`).
    pub fn heading(text: impl Into<String>) -> Self {
        Self::new(text).with_role(proposal::TextRole::UiHeading)
    }
    /// Construct label text (`TextRole::UiLabel`).
    pub fn label(text: impl Into<String>) -> Self {
        Self::new(text).with_role(proposal::TextRole::UiLabel)
    }
    /// Construct secondary explanatory text (`TextRole::UiSecondary`).
    pub fn secondary(text: impl Into<String>) -> Self {
        Self::new(text).with_role(proposal::TextRole::UiSecondary)
    }
    /// Construct caption/helper text (`TextRole::UiCaption`).
    pub fn caption(text: impl Into<String>) -> Self {
        Self::new(text).with_role(proposal::TextRole::UiCaption)
    }
    pub(super) fn text_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)?
            .checked_add(
                self.text
                    .len()
                    .checked_mul(n.max(1))
                    .ok_or(PreparedEffectsError::CounterCapacity)?,
            )
            .ok_or(PreparedEffectsError::CounterCapacity)
    }
}
impl Component for TextContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(text.measure(&self.text, self.role, available.width))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        let ink = theme.foreground.unwrap_or_else(|| {
            let usage = match self.role {
                proposal::TextRole::UiSecondary | proposal::TextRole::UiCaption => {
                    "data-category-label"
                }
                _ => "content",
            };
            tekes_ui_tokens::surface_palette::resolve_surface(
                theme.palette,
                theme.surface(),
                usage,
                "rest",
            )
            .expect("validated TextContent Surface packet")
            .text
        });
        text.draw(
            scene,
            &self.text,
            self.role,
            frame,
            ButtonTextAlign::Leading,
            ink,
        )
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Text, &self.text)
    }
}

#[cfg(test)]
mod section_and_insets_tests {
    use super::*;

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (value.chars().count() as f32 * 7.).min(width),
                role.style().line_height,
            )
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

    #[test]
    fn scroll_content_insets_and_flush_scrollbar_preserve_four_edge_geometry() {
        let mut scroll = ScrollContent::new(
            ScrollArea::default(),
            Content::new(ComponentId(2), TextContent::new("Line\n\n\n\n\n\nBottom")),
        )
        .scrollbar(true)
        .content_insets(crate::layout::Insets::new(8., 16., 16., 16.));
        let viewport = Rect::new(10., 20., 300., 80.);
        scroll.arrange(&mut Text, viewport, viewport);
        let child = scroll.children()[0].frame();
        assert_eq!(child.min_x() - viewport.min_x(), 16.);
        assert_eq!(viewport.max_x() - child.max_x(), 16.);
        assert_eq!(child.min_y() - viewport.min_y(), 8.);
        let track = scroll.scrollbar_track(true);
        assert_eq!(track.max_x(), viewport.max_x() - proposal::SCROLLBAR_INSET);
        assert_eq!(track.min_y(), viewport.min_y());
    }

    #[test]
    fn stack_gestalt_sections_and_labeled_specimens_preserve_hierarchy() {
        const {
            assert!(StackContent::DOCUMENT_SECTION_GAP > StackContent::PANEL_SECTION_GAP);
            assert!(StackContent::PANEL_SECTION_GAP > StackContent::SECTION_HEADER_GAP);
            assert!(StackContent::SECTION_HEADER_GAP > StackContent::SPECIMEN_LABEL_GAP);
        }
        let mut host = ContentHost::new(StackContent::document_sections(
            ComponentId(1),
            vec![
                StackContent::titled_section(
                    ComponentId(2),
                    ComponentId(3),
                    "First",
                    StackContent::labeled_specimen(
                        ComponentId(4),
                        ComponentId(5),
                        "rest",
                        Content::new(ComponentId(6), TextContent::new("Sample 1")),
                    ),
                ),
                StackContent::titled_section(
                    ComponentId(7),
                    ComponentId(8),
                    "Second",
                    Content::new(ComponentId(9), TextContent::new("Sample 2")),
                ),
            ],
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 400., 300.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let h1 = host.get(ComponentId(3)).unwrap().frame();
        let label = host.get(ComponentId(5)).unwrap().frame();
        let sample1 = host.get(ComponentId(6)).unwrap().frame();
        let h2 = host.get(ComponentId(8)).unwrap().frame();
        assert_eq!(label.min_y() - h1.max_y(), StackContent::SECTION_HEADER_GAP);
        assert_eq!(
            sample1.min_y() - label.max_y(),
            StackContent::SPECIMEN_LABEL_GAP
        );
        assert_eq!(
            h2.min_y() - sample1.max_y(),
            StackContent::DOCUMENT_SECTION_GAP
        );
        assert_eq!(
            host.get(ComponentId(3))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .role,
            proposal::TextRole::UiHeading
        );
        assert_eq!(
            host.get(ComponentId(5))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .role,
            proposal::TextRole::UiCaption
        );
    }

    #[test]
    fn horizontal_strip_measures_finite_cross_axis_height() {
        use tekes_ui_tokens::Spacing;
        let row = Content::new(
            ComponentId(20),
            StackContent {
                model: Stack {
                    axis: Axis::Horizontal,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children: vec![
                    Content::new(ComponentId(21), Button::new("Docs")),
                    Content::new(ComponentId(22), Button::new("Playground")),
                    Content::new(ComponentId(23), Button::new("Reset")),
                ],
            },
        );
        let mut host = ContentHost::new(Content::new(
            ComponentId(19),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: Spacing::Regular.value(),
                },
                flexible_child: None,
                children: vec![
                    Content::new(ComponentId(24), ScrollContent::horizontal_strip(row)),
                    Content::new(ComponentId(25), TextContent::new("Below strip")),
                ],
            },
        ))
        .unwrap();
        let viewport = Rect::new(0., 0., 220., 600.);
        host.render(
            &mut Scene::new(),
            &mut Text,
            viewport,
            viewport,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let strip = host.get(ComponentId(24)).unwrap().frame();
        let below = host.get(ComponentId(25)).unwrap().frame();
        assert!(strip.size.height > 0. && strip.size.height < 100.);
        assert_eq!(below.min_y() - strip.max_y(), Spacing::Regular.value());
        assert_eq!(
            host.get(ComponentId(21)).unwrap().frame().min_y(),
            host.get(ComponentId(23)).unwrap().frame().min_y()
        );
    }

    #[test]
    fn responsive_grid_max_width_and_theme_scope_behave_as_containers() {
        let mut grid = ResponsiveGridContent::new(
            1,
            8.,
            12.,
            vec![
                Content::new(ComponentId(31), TextContent::new("A")),
                Content::new(ComponentId(32), TextContent::new("B")),
                Content::new(ComponentId(33), TextContent::new("C")),
                Content::new(ComponentId(34), TextContent::new("D")),
            ],
        )
        .with_breakpoints(vec![(600., 4), (260., 2)]);
        grid.measure(&mut Text, Size::new(700., 400.));
        assert_eq!(grid.columns(), 4);
        grid.measure(&mut Text, Size::new(300., 400.));
        assert_eq!(grid.columns(), 2);
        grid.measure(&mut Text, Size::new(200., 400.));
        assert_eq!(grid.columns(), 1);

        let scoped = Content::new(
            ComponentId(40),
            MaxWidthContent::new(
                240.,
                Content::new(
                    ComponentId(41),
                    ThemeScopeContent::new(
                        tekes_ui_tokens::Theme::DARK.into(),
                        Content::new(ComponentId(42), Button::new("Scoped")),
                    ),
                ),
            ),
        );
        let mut host = ContentHost::new(scoped).unwrap();
        let viewport = Rect::new(0., 0., 500., 300.);
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut Text,
            viewport,
            viewport,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(host.get(ComponentId(41)).unwrap().frame().size.width, 240.);
        assert!(host.get(ComponentId(41)).unwrap().frame().size.height < 300.);
        assert!(
            host.get(ComponentId(40))
                .unwrap()
                .component::<MaxWidthContent>()
                .unwrap()
                .host_reveal_passthrough()
        );
        assert!(
            host.get(ComponentId(41))
                .unwrap()
                .component::<ThemeScopeContent>()
                .unwrap()
                .host_reveal_passthrough()
        );
    }
}

/// Breakpoint-driven responsive wrapper over [`GridContent`].
///
/// Updates the underlying `Grid` column count from `available.width` using descending
/// `(min_width, columns)` breakpoints, preserving container invariants and focus-reveal passthrough.
#[derive(Debug)]
pub struct ResponsiveGridContent {
    grid: GridContent,
    default_columns: usize,
    breakpoints: Vec<(f32, usize)>,
    arranged: bool,
}

impl ResponsiveGridContent {
    pub fn new(default_columns: usize, gap_x: f32, gap_y: f32, children: Vec<Content>) -> Self {
        let default_columns = default_columns.max(1);
        let mut model = Grid::new(default_columns);
        model.gap_x = finite(gap_x);
        model.gap_y = finite(gap_y);
        Self {
            grid: GridContent { model, children },
            default_columns,
            breakpoints: Vec::new(),
            arranged: false,
        }
    }

    /// Configure `(min_width, columns)` breakpoints evaluated from highest `min_width` down.
    pub fn with_breakpoints(mut self, breakpoints: Vec<(f32, usize)>) -> Self {
        self.breakpoints = breakpoints
            .into_iter()
            .map(|(min_width, cols)| (finite(min_width), cols.max(1)))
            .collect();
        self.breakpoints.sort_by(|a, b| b.0.total_cmp(&a.0));
        self
    }

    pub fn columns(&self) -> usize {
        self.grid.model.columns
    }

    pub fn model(&self) -> &Grid {
        &self.grid.model
    }

    fn resolve_columns(&self, width: f32) -> usize {
        self.breakpoints
            .iter()
            .find(|(min_width, _)| width >= *min_width)
            .map_or(self.default_columns, |(_, cols)| *cols)
            .max(1)
    }
}

impl Component for ResponsiveGridContent {
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
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

    fn children(&self) -> &[Content] {
        &self.grid.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.grid.children)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        self.grid.model.columns = self.resolve_columns(available.width);
        self.grid.measure(text, available)
    }

    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.arranged = false;
        self.grid.model.columns = self.resolve_columns(available.width);
        self.grid.measure_intrinsic(text, available)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.grid.model.columns = self.resolve_columns(frame.size.width);
        self.grid.arrange(text, frame, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}

/// Single-child container capping measured and arranged width to `max_width`.
#[derive(Debug)]
pub struct MaxWidthContent {
    max_width: f32,
    children: Vec<Content>,
    arranged_child: Option<(ComponentId, u64)>,
}

impl MaxWidthContent {
    pub fn new(max_width: f32, child: Content) -> Self {
        Self {
            max_width: finite(max_width),
            children: vec![child],
            arranged_child: None,
        }
    }

    pub fn max_width(&self) -> f32 {
        self.max_width
    }
}

impl Component for MaxWidthContent {
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged_child = None;
        }
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.children.len() == 1
            && self.arranged_child.is_some_and(|identity| {
                let child = &self.children[0];
                (child.id(), child.mount_identity()) == identity
            })
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != 1 {
            Err(CompositionError::InvalidSlotCount)
        } else {
            Ok(())
        }
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

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged_child = None;
        self.children[0].measure(
            text,
            Size::new(available.width.min(self.max_width), available.height),
        )
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let width = frame.size.width.min(self.max_width);
        let measured_height = self.children[0]
            .measure(text, Size::new(width, frame.size.height))
            .size
            .height;
        let height = if measured_height.is_finite() {
            measured_height.min(frame.size.height).max(0.)
        } else {
            frame.size.height
        };
        let capped = Rect::new(frame.min_x(), frame.min_y(), width, height);
        self.children[0].arrange(text, capped, clip);
        self.arranged_child = self
            .children
            .first()
            .filter(|child| !child.frame().is_empty())
            .map(|child| (child.id(), child.mount_identity()));
    }
}

enum PaletteScopeResolver {
    Fixed(Palette),
    Dynamic(Box<dyn Fn(Palette) -> Palette>),
}

impl std::fmt::Debug for PaletteScopeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(palette) => f.debug_tuple("Fixed").field(palette).finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(..)"),
        }
    }
}

impl PaletteScopeResolver {
    fn apply(&self, inherited: Palette) -> Palette {
        match self {
            Self::Fixed(palette) => *palette,
            Self::Dynamic(resolve) => resolve(inherited),
        }
    }
}

/// Single-child container projecting an explicit or derived `Palette` onto its subtree
/// while inheriting host layout and surface declarations.
#[derive(Debug)]
pub struct ThemeScopeContent {
    children: Vec<Content>,
    resolver: PaletteScopeResolver,
    arranged: bool,
}

impl ThemeScopeContent {
    pub fn new(palette: Palette, child: Content) -> Self {
        Self {
            children: vec![child],
            resolver: PaletteScopeResolver::Fixed(palette),
            arranged: false,
        }
    }

    pub fn with_resolver(resolve: impl Fn(Palette) -> Palette + 'static, child: Content) -> Self {
        Self {
            children: vec![child],
            resolver: PaletteScopeResolver::Dynamic(Box::new(resolve)),
            arranged: false,
        }
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.resolver = PaletteScopeResolver::Fixed(palette);
    }
}

impl Component for ThemeScopeContent {
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != 1 {
            Err(CompositionError::InvalidSlotCount)
        } else {
            Ok(())
        }
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

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn child_theme(&self, mut theme: ContentTheme) -> ContentTheme {
        theme.palette = self.resolver.apply(theme.palette);
        theme
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        self.children[0].measure(text, available)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.children[0].arrange(text, frame, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}
