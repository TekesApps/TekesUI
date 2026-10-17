//! Shared, immutable container and scroll geometry snapshots.
//!
//! These helpers keep layout, clipping, and pointer conversion on the same
//! logical-pixel rectangles. They describe geometry only; content realization
//! and scheduling remain the responsibility of their consumers.
use tekes_ui_scene::{Point, Rect, Size};
use tekes_ui_tokens::proposal::layout::{
    AlignmentPolicy, CrossAxisAlignment, MainAxisDistribution,
};

/// A child's measured size and optional baseline from its top edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasuredChild {
    pub size: Size,
    pub first_baseline: Option<f32>,
}

impl MeasuredChild {
    pub const fn new(size: Size) -> Self {
        Self {
            size,
            first_baseline: None,
        }
    }

    pub const fn with_first_baseline(mut self, baseline: f32) -> Self {
        self.first_baseline = Some(baseline);
        self
    }
}

/// Per-call geometry overrides. Text alignment remains part of the policy but
/// is intentionally not used to place measured rectangles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlignmentOverride {
    pub main_axis: Option<MainAxisDistribution>,
    pub cross_axis: Option<CrossAxisAlignment>,
}

/// The full child frame and the part visible inside the parent bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlignedChild {
    pub frame: Rect,
    pub visible: Option<Rect>,
}

/// Positions measured children on a shared axis using Design System policy.
/// Intrinsic dimensions are retained; `visible` is clipped to `bounds` and is
/// the rectangle consumers should use for painting and hit testing. Text must
/// be measured at the available width by the caller before passing its size.
/// If any derived edge or position exceeds the finite `f32` geometry range,
/// the function returns one empty frame per child with no visible region.
pub fn layout_measured_children(
    bounds: Rect,
    axis: crate::extended::Axis,
    children: &[MeasuredChild],
    gap: f32,
    policy: AlignmentPolicy,
    overrides: AlignmentOverride,
) -> Vec<AlignedChild> {
    let bounds = normalized_rect(bounds);
    if children.is_empty() {
        return Vec::new();
    }
    let main_alignment = overrides.main_axis.unwrap_or(policy.main_axis);
    let cross_alignment = overrides.cross_axis.unwrap_or(policy.cross_axis);
    let horizontal = matches!(axis, crate::extended::Axis::Horizontal);
    let main_length = if horizontal {
        bounds.size.width
    } else {
        bounds.size.height
    };
    let cross_length = if horizontal {
        bounds.size.height
    } else {
        bounds.size.width
    };
    let sizes: Vec<Size> = children
        .iter()
        .map(|child| {
            Size::new(
                finite_nonnegative(child.size.width),
                finite_nonnegative(child.size.height),
            )
        })
        .collect();
    let main_sizes: Vec<f32> = sizes
        .iter()
        .map(|size| if horizontal { size.width } else { size.height })
        .collect();
    let total_main: f32 = main_sizes.iter().sum();
    let requested_gap = finite_nonnegative(gap);
    let gaps = children.len().saturating_sub(1);
    let gap = if gaps == 0 {
        0.0
    } else {
        requested_gap.min(main_length / gaps as f32)
    };
    let natural = total_main + gap * gaps as f32;
    let extra = (main_length - natural).max(0.0);
    let (mut cursor, distributed_gap) = match main_alignment {
        MainAxisDistribution::Start => (0.0, gap),
        MainAxisDistribution::Center => (extra * 0.5, gap),
        MainAxisDistribution::End => (extra, gap),
        MainAxisDistribution::SpaceBetween if gaps > 0 => (0.0, gap + extra / gaps as f32),
        MainAxisDistribution::SpaceBetween => (0.0, gap),
    };
    let baseline = if horizontal && cross_alignment == CrossAxisAlignment::FirstBaseline {
        children
            .iter()
            .zip(&sizes)
            .filter_map(|(child, size)| {
                child
                    .first_baseline
                    .filter(|value| value.is_finite())
                    .map(|value| value.clamp(0.0, size.height))
            })
            .reduce(f32::max)
    } else {
        None
    };
    let frames: Vec<Rect> = children
        .iter()
        .zip(sizes)
        .zip(main_sizes)
        .map(|((child, size), main_size)| {
            let cross_size = if horizontal { size.height } else { size.width };
            let cross_offset = match cross_alignment {
                CrossAxisAlignment::Leading => 0.0,
                CrossAxisAlignment::Center => (cross_length - cross_size) * 0.5,
                CrossAxisAlignment::Trailing => cross_length - cross_size,
                CrossAxisAlignment::FirstBaseline if horizontal => {
                    let baseline = baseline.unwrap_or(cross_length * 0.5);
                    let child_baseline = child
                        .first_baseline
                        .filter(|value| value.is_finite())
                        .map(|value| value.clamp(0.0, size.height))
                        .unwrap_or(cross_size * 0.5);
                    baseline - child_baseline
                }
                CrossAxisAlignment::FirstBaseline => 0.0,
            };
            let frame = if horizontal {
                Rect::new(
                    bounds.origin.x + cursor,
                    bounds.origin.y + cross_offset,
                    size.width,
                    size.height,
                )
            } else {
                Rect::new(
                    bounds.origin.x + cross_offset,
                    bounds.origin.y + cursor,
                    size.width,
                    size.height,
                )
            };
            cursor += main_size + distributed_gap;
            frame
        })
        .collect();
    if !rect_edges_are_finite(bounds) || frames.iter().any(|frame| !rect_edges_are_finite(*frame)) {
        return vec![
            AlignedChild {
                frame: Rect::default(),
                visible: None,
            };
            children.len()
        ];
    }
    frames
        .into_iter()
        .map(|frame| AlignedChild {
            frame,
            visible: frame.intersection(&bounds),
        })
        .collect()
}

/// Insets applied to each edge of a container, in logical points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    fn normalized(self) -> Self {
        Self {
            top: finite_nonnegative(self.top),
            right: finite_nonnegative(self.right),
            bottom: finite_nonnegative(self.bottom),
            left: finite_nonnegative(self.left),
        }
    }
}

/// A normalized container frame, its inner content rectangle, and its
/// effective ancestor-clipped region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerLayout {
    frame: Rect,
    content: Rect,
    ancestor_clip: Option<Rect>,
    visible_clip: Option<Rect>,
}

impl ContainerLayout {
    /// Insets are applied in order: frame border first, then content padding.
    /// Each side consumes only the space remaining on that side, so excessive
    /// or non-finite values collapse safely to an empty rectangle.
    pub fn new(frame: Rect, border: Insets, content: Insets, ancestor_clip: Option<Rect>) -> Self {
        let frame = normalized_rect(frame);
        let border = border.normalized();
        let content_insets = content.normalized();
        let ancestor_clip = ancestor_clip.map(normalized_rect);
        let bordered = inset_edges(frame, border);
        let content = inset_edges(bordered, content_insets);
        let visible_clip = match ancestor_clip {
            Some(clip) => content.intersection(&clip),
            None if !content.is_empty() => Some(content),
            None => None,
        };
        Self {
            frame,
            content,
            ancestor_clip,
            visible_clip,
        }
    }

    pub const fn frame(&self) -> Rect {
        self.frame
    }

    pub const fn content(&self) -> Rect {
        self.content
    }

    pub const fn ancestor_clip(&self) -> Option<Rect> {
        self.ancestor_clip
    }

    pub const fn visible_clip(&self) -> Option<Rect> {
        self.visible_clip
    }

    /// Clip for painting a container's background. Unlike `visible_clip`,
    /// this is only the true ancestor boundary: the container's own frame and
    /// insets do not trim the outer antialiasing fringe of its background.
    /// Callers must still check `visible_clip()` before painting so an empty
    /// or fully clipped container emits no geometry. `None` means there is no
    /// additional scene clip to push.
    pub const fn background_paint_clip(&self) -> Option<Rect> {
        self.ancestor_clip
    }
}

/// A coherent scroll transform for one layout pass.
///
/// `viewport` and `ancestor_clip` are in the same parent coordinate space;
/// document coordinates start at `(0, 0)`. Normal offset is clamped to the
/// document's scrollable range, while elastic displacement remains separate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollGeometry {
    viewport: Rect,
    document_bounds: Rect,
    normal_offset: Point,
    overscroll: Point,
    visible_viewport_rect: Option<Rect>,
    visible_document_rect: Option<Rect>,
}

impl ScrollGeometry {
    pub fn new(
        viewport: Rect,
        document_extent: Size,
        offset: Point,
        overscroll: Point,
        ancestor_clip: Option<Rect>,
    ) -> Self {
        let viewport = normalized_rect(viewport);
        let document = Size::new(
            finite_nonnegative(document_extent.width),
            finite_nonnegative(document_extent.height),
        );
        let document_bounds = Rect::new(0.0, 0.0, document.width, document.height);
        let normal_offset = Point::new(
            finite_or_zero(offset.x).clamp(0.0, (document.width - viewport.size.width).max(0.0)),
            finite_or_zero(offset.y).clamp(0.0, (document.height - viewport.size.height).max(0.0)),
        );
        let overscroll = Point::new(finite_or_zero(overscroll.x), finite_or_zero(overscroll.y));
        let effective_clip = match ancestor_clip.map(normalized_rect) {
            Some(clip) => viewport.intersection(&clip),
            None if !viewport.is_empty() => Some(viewport),
            None => None,
        };
        let visible_document_rect = effective_clip.and_then(|visible| {
            let origin = Point::new(
                viewport.origin.x - normal_offset.x - overscroll.x,
                viewport.origin.y - normal_offset.y - overscroll.y,
            );
            let visible_in_document = Rect::new(
                visible.origin.x - origin.x,
                visible.origin.y - origin.y,
                visible.size.width,
                visible.size.height,
            );
            visible_in_document.intersection(&document_bounds)
        });
        Self {
            viewport,
            document_bounds,
            normal_offset,
            overscroll,
            visible_viewport_rect: effective_clip,
            visible_document_rect,
        }
    }

    pub const fn viewport(&self) -> Rect {
        self.viewport
    }

    pub const fn document_bounds(&self) -> Rect {
        self.document_bounds
    }

    pub const fn normal_offset(&self) -> Point {
        self.normal_offset
    }

    pub const fn overscroll(&self) -> Point {
        self.overscroll
    }

    pub const fn visual_offset(&self) -> Point {
        Point::new(
            self.normal_offset.x + self.overscroll.x,
            self.normal_offset.y + self.overscroll.y,
        )
    }

    /// Document origin in the viewport's parent coordinate space.
    pub const fn content_origin(&self) -> Point {
        Point::new(
            self.viewport.origin.x - self.visual_offset().x,
            self.viewport.origin.y - self.visual_offset().y,
        )
    }

    pub const fn visible_viewport_rect(&self) -> Option<Rect> {
        self.visible_viewport_rect
    }

    /// Visible document content. Elastic blank space is excluded by
    /// intersecting the transformed viewport with the finite document bounds.
    pub const fn visible_document_rect(&self) -> Option<Rect> {
        self.visible_document_rect
    }

    /// Convert a point in parent coordinates into document coordinates.
    /// Points outside the effective viewport, in ancestor-clipped regions, or
    /// in elastic blank space do not hit document content.
    pub fn viewport_to_document(&self, point: Point) -> Option<Point> {
        let visible = self.visible_viewport_rect?;
        if !point_is_finite(point) || !visible.contains(point) {
            return None;
        }
        let origin = self.content_origin();
        let document_point = Point::new(point.x - origin.x, point.y - origin.y);
        self.document_bounds
            .contains(document_point)
            .then_some(document_point)
    }

    /// Convert a document rectangle into parent coordinates and clip it to
    /// the effective viewport. Empty or fully clipped content returns `None`.
    pub fn document_to_viewport(&self, rect: Rect) -> Option<Rect> {
        let rect = normalized_rect(rect);
        let rect = rect.intersection(&self.document_bounds)?;
        let origin = self.content_origin();
        let transformed = rect.offset(origin.x, origin.y);
        transformed.intersection(&self.visible_viewport_rect?)
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_nonnegative(value: f32) -> f32 {
    finite_or_zero(value).max(0.0)
}

fn normalized_rect(rect: Rect) -> Rect {
    if !rect.origin.x.is_finite() || !rect.origin.y.is_finite() {
        return Rect::default();
    }
    Rect::new(
        rect.origin.x,
        rect.origin.y,
        finite_nonnegative(rect.size.width),
        finite_nonnegative(rect.size.height),
    )
}

fn rect_edges_are_finite(rect: Rect) -> bool {
    let max = f32::MAX as f64;
    let right = rect.origin.x as f64 + rect.size.width as f64;
    let bottom = rect.origin.y as f64 + rect.size.height as f64;
    right.is_finite()
        && bottom.is_finite()
        && right <= max
        && bottom <= max
        && rect.origin.x.is_finite()
        && rect.origin.y.is_finite()
        && rect.size.width.is_finite()
        && rect.size.height.is_finite()
}

fn inset_edges(rect: Rect, insets: Insets) -> Rect {
    let left = insets.left.min(rect.size.width);
    let right = insets.right.min(rect.size.width - left);
    let top = insets.top.min(rect.size.height);
    let bottom = insets.bottom.min(rect.size.height - top);
    Rect::new(
        rect.origin.x + left,
        rect.origin.y + top,
        rect.size.width - left - right,
        rect.size.height - top - bottom,
    )
}

fn point_is_finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_tokens::proposal::layout::{self, AlignmentPreset};

    #[test]
    fn container_insets_are_applied_once_and_clip_to_ancestor() {
        let layout = ContainerLayout::new(
            Rect::new(10.0, 20.0, 200.0, 100.0),
            Insets::new(1.0, 2.0, 3.0, 4.0),
            Insets::new(5.0, 6.0, 7.0, 8.0),
            Some(Rect::new(30.0, 30.0, 150.0, 60.0)),
        );
        assert_eq!(layout.frame(), Rect::new(10.0, 20.0, 200.0, 100.0));
        assert_eq!(layout.content(), Rect::new(22.0, 26.0, 180.0, 84.0));
        assert_eq!(
            layout.ancestor_clip(),
            Some(Rect::new(30.0, 30.0, 150.0, 60.0))
        );
        assert_eq!(
            layout.visible_clip(),
            Some(Rect::new(30.0, 30.0, 150.0, 60.0))
        );
    }

    #[test]
    fn background_paint_clip_is_ancestor_only_and_empty_visibility_stays_empty() {
        let ancestor = Rect::new(0.0, 0.0, 200.0, 120.0);
        let partly_clipped = ContainerLayout::new(
            Rect::new(30.25, 20.75, 80.0, 40.0),
            Insets::ZERO,
            Insets::ZERO,
            Some(ancestor),
        );
        assert_eq!(partly_clipped.visible_clip(), Some(partly_clipped.frame()));
        assert_eq!(partly_clipped.background_paint_clip(), Some(ancestor));

        let unbounded = ContainerLayout::new(
            Rect::new(30.25, 20.75, 80.0, 40.0),
            Insets::ZERO,
            Insets::ZERO,
            None,
        );
        assert_eq!(unbounded.visible_clip(), Some(unbounded.frame()));
        assert_eq!(unbounded.background_paint_clip(), None);

        let empty_parent = ContainerLayout::new(
            Rect::new(30.25, 20.75, 80.0, 40.0),
            Insets::ZERO,
            Insets::ZERO,
            Some(Rect::new(500.0, 500.0, 0.0, 0.0)),
        );
        assert_eq!(empty_parent.visible_clip(), None);
        // A renderer must skip an empty visible clip before using this value.
        assert_eq!(
            empty_parent.background_paint_clip(),
            Some(Rect::new(500.0, 500.0, 0.0, 0.0))
        );
    }

    #[test]
    fn nested_clip_and_bounce_share_the_same_forward_and_reverse_transform() {
        let geometry = ScrollGeometry::new(
            Rect::new(20.0, 30.0, 100.0, 80.0),
            Size::new(200.0, 240.0),
            Point::new(40.0, 50.0),
            Point::new(0.0, -12.0),
            Some(Rect::new(30.0, 40.0, 70.0, 55.0)),
        );
        assert_eq!(geometry.content_origin(), Point::new(-20.0, -8.0));
        assert_eq!(
            geometry.visible_viewport_rect(),
            Some(Rect::new(30.0, 40.0, 70.0, 55.0))
        );
        assert_eq!(
            geometry.visible_document_rect(),
            Some(Rect::new(50.0, 48.0, 70.0, 55.0))
        );
        let point = Point::new(42.0, 52.0);
        let doc = geometry.viewport_to_document(point).unwrap();
        assert_eq!(doc, Point::new(62.0, 60.0));
        assert_eq!(
            geometry.document_to_viewport(Rect::new(doc.x, doc.y, 4.0, 4.0)),
            Some(Rect::new(point.x, point.y, 4.0, 4.0))
        );
    }

    #[test]
    fn normal_offset_clamps_after_resize_and_bounce_blank_has_no_hit() {
        let geometry = ScrollGeometry::new(
            Rect::new(0.0, 0.0, 100.0, 80.0),
            Size::new(90.0, 120.0),
            Point::new(30.0, 50.0),
            Point::new(0.0, 14.0),
            None,
        );
        assert_eq!(geometry.normal_offset(), Point::new(0.0, 40.0));
        assert_eq!(
            geometry.visible_document_rect(),
            Some(Rect::new(0.0, 54.0, 90.0, 66.0))
        );
        assert_eq!(geometry.viewport_to_document(Point::new(50.0, 79.0)), None);
    }

    #[test]
    fn zero_nonfinite_and_empty_geometry_is_safe() {
        let container = ContainerLayout::new(
            Rect::new(f32::NAN, 2.0, -4.0, f32::INFINITY),
            Insets::new(f32::NAN, -1.0, 0.0, 0.0),
            Insets::ZERO,
            None,
        );
        assert_eq!(container.frame(), Rect::default());
        assert_eq!(container.visible_clip(), None);
        let geometry = ScrollGeometry::new(
            Rect::new(4.0, 5.0, 0.0, 10.0),
            Size::new(f32::NAN, 40.0),
            Point::new(f32::INFINITY, 8.0),
            Point::new(0.0, f32::NAN),
            None,
        );
        assert_eq!(geometry.normal_offset(), Point::new(0.0, 8.0));
        assert_eq!(geometry.visible_viewport_rect(), None);
        assert_eq!(geometry.visible_document_rect(), None);
        assert_eq!(geometry.viewport_to_document(Point::new(4.0, 5.0)), None);
        let invalid_origin = ContainerLayout::new(
            Rect::new(f32::INFINITY, 5.0, 20.0, 20.0),
            Insets::ZERO,
            Insets::ZERO,
            None,
        );
        assert_eq!(invalid_origin.content(), Rect::default());
        assert_eq!(invalid_origin.visible_clip(), None);
    }

    #[test]
    fn document_render_mapping_rejects_content_outside_extent() {
        let geometry = ScrollGeometry::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(60.0, 80.0),
            Point::default(),
            Point::default(),
            None,
        );
        assert_eq!(
            geometry.document_to_viewport(Rect::new(60.0, 10.0, 10.0, 10.0)),
            None
        );
        assert_eq!(
            geometry.document_to_viewport(Rect::new(55.0, 10.0, 10.0, 10.0)),
            Some(Rect::new(55.0, 10.0, 5.0, 10.0))
        );
    }

    #[test]
    fn measured_row_centers_children_and_clips_fractional_overflow() {
        let result = layout_measured_children(
            Rect::new(0.25, 0.5, 31.0, 12.0),
            crate::extended::Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(15.0, 8.0)),
                MeasuredChild::new(Size::new(20.0, 6.0)),
            ],
            4.0,
            layout::ROW,
            AlignmentOverride::default(),
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].frame, Rect::new(0.25, 2.5, 15.0, 8.0));
        assert_eq!(result[1].frame, Rect::new(19.25, 3.5, 20.0, 6.0));
        assert_eq!(result[1].visible, Some(Rect::new(19.25, 3.5, 12.0, 6.0)));
    }

    #[test]
    fn column_defaults_to_leading_and_first_baseline_aligns_text_row() {
        let column = layout_measured_children(
            Rect::new(2.0, 3.0, 20.0, 50.0),
            crate::extended::Axis::Vertical,
            &[MeasuredChild::new(Size::new(12.0, 10.0))],
            2.0,
            layout::COLUMN,
            AlignmentOverride::default(),
        );
        assert_eq!(column[0].frame, Rect::new(2.0, 3.0, 12.0, 10.0));

        let row = layout_measured_children(
            Rect::new(0.0, 0.0, 120.0, 44.0),
            crate::extended::Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(20.0, 20.0)).with_first_baseline(10.0),
                MeasuredChild::new(Size::new(80.0, 40.0)).with_first_baseline(30.0),
                MeasuredChild::new(Size::new(15.0, 16.0)),
            ],
            4.0,
            layout::alignment(AlignmentPreset::TextRow),
            AlignmentOverride::default(),
        );
        assert_eq!(row[0].frame.origin.y + 10.0, 30.0);
        assert_eq!(row[1].frame.origin.y + 30.0, 30.0);
        assert_eq!(row[2].frame.origin.y + 8.0, 30.0);
    }

    #[test]
    fn narrow_long_measured_content_keeps_intrinsic_size_and_safe_visible_clip() {
        let result = layout_measured_children(
            Rect::new(1.5, 2.5, 24.0, 18.0),
            crate::extended::Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(24.0, 42.0)).with_first_baseline(15.5),
                MeasuredChild::new(Size::new(8.0, 8.0)),
            ],
            8.0,
            layout::TEXT_ROW,
            AlignmentOverride::default(),
        );
        assert_eq!(result[0].frame.size, Size::new(24.0, 42.0));
        assert_eq!(result[0].visible, Some(Rect::new(1.5, 2.5, 24.0, 18.0)));
        assert_eq!(result[1].visible, None);
        assert_eq!(
            result[0].frame.origin.y + 15.5,
            result[1].frame.origin.y + 4.0
        );
    }

    #[test]
    fn malformed_measurements_and_space_between_overrides_remain_finite() {
        let result = layout_measured_children(
            Rect::new(0.0, 0.0, 40.0, 20.0),
            crate::extended::Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(f32::NAN, -2.0)),
                MeasuredChild::new(Size::new(10.0, 8.0)),
            ],
            f32::INFINITY,
            tekes_ui_tokens::proposal::layout::ROW,
            AlignmentOverride {
                main_axis: Some(MainAxisDistribution::SpaceBetween),
                cross_axis: None,
            },
        );
        assert_eq!(result[0].frame, Rect::new(0.0, 10.0, 0.0, 0.0));
        assert_eq!(result[1].frame, Rect::new(30.0, 6.0, 10.0, 8.0));
        assert!(result.iter().all(|item| {
            item.frame.origin.x.is_finite()
                && item.frame.origin.y.is_finite()
                && item.frame.size.width.is_finite()
                && item.frame.size.height.is_finite()
        }));
    }

    #[test]
    fn unrepresentable_bounds_or_accumulated_children_return_empty_visible_geometry() {
        let overflowing_bounds = layout_measured_children(
            Rect::new(f32::MAX, 0.0, f32::MAX, 10.0),
            crate::extended::Axis::Horizontal,
            &[MeasuredChild::new(Size::new(1.0, 1.0))],
            0.0,
            tekes_ui_tokens::proposal::layout::ROW,
            AlignmentOverride::default(),
        );
        assert_eq!(overflowing_bounds.len(), 1);
        assert_eq!(overflowing_bounds[0].frame, Rect::default());
        assert_eq!(overflowing_bounds[0].visible, None);

        let overflowing_children = layout_measured_children(
            Rect::new(0.0, 0.0, f32::MAX, 10.0),
            crate::extended::Axis::Horizontal,
            &[
                MeasuredChild::new(Size::new(f32::MAX, 1.0)),
                MeasuredChild::new(Size::new(f32::MAX, 1.0)),
                MeasuredChild::new(Size::new(f32::MAX, 1.0)),
            ],
            1.0,
            tekes_ui_tokens::proposal::layout::ROW,
            AlignmentOverride::default(),
        );
        assert_eq!(overflowing_children.len(), 3);
        assert!(
            overflowing_children
                .iter()
                .all(|item| { item.frame == Rect::default() && item.visible.is_none() })
        );
    }
}
