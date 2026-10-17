//! Shared page viewport; clipping belongs to the renderer, not to geometry mutation.
use super::layout;
use std::ops::{Deref, DerefMut};
use tekes_ui_controls::ScrollArea;
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, ControlSize, Spacing, Theme};
#[derive(Debug)]
pub struct Viewport {
    size: Size,
    fill_window: bool,
    document: Size,
    scroll: ScrollArea,
}
impl Deref for Viewport {
    type Target = ScrollArea;
    fn deref(&self) -> &ScrollArea {
        &self.scroll
    }
}
impl DerefMut for Viewport {
    fn deref_mut(&mut self) -> &mut ScrollArea {
        &mut self.scroll
    }
}
impl Default for Viewport {
    fn default() -> Self {
        let mut v = Self {
            size: Size::new(layout::FRAME_W, layout::FRAME_H),
            fill_window: false,
            document: layout::PAGE_CONTENT.size,
            scroll: ScrollArea::default(),
        };
        v.resize(layout::FRAME_W, layout::FRAME_H);
        v
    }
}
impl Viewport {
    pub fn set_fill_window(&mut self, fill: bool) {
        self.fill_window = fill;
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            self.size = Size::new(width, height);
            self.scroll.layout(self.bounds().size, self.document);
        }
    }
    pub fn document_height(&self) -> f32 {
        self.document.height
    }
    pub fn set_document_extent(&mut self, size: Size) {
        self.document = size;
        self.scroll.layout(self.bounds().size, size);
    }
    pub fn bounds(&self) -> Rect {
        let c = layout::PAGE_CONTENT;
        let width = (self.size.width - c.min_x() - Spacing::Section.value()).max(0.0);
        let height = (self.size.height - c.min_y() - Spacing::Section.value()).max(0.0);
        Rect::new(
            c.min_x(),
            c.min_y(),
            if self.fill_window {
                width
            } else {
                c.size.width.min(width)
            },
            if self.fill_window {
                height
            } else {
                c.size.height.min(height)
            },
        )
    }
    pub fn move_by(&mut self, x: f32, y: f32) -> bool {
        self.scroll.scroll_by(x, y)
    }
    /// Reveals a focused document-space control while preserving this page's
    /// viewport as the clipping and scrolling boundary.
    pub fn reveal_focus(&mut self, document_bounds: Rect, backing_scale: f32) -> bool {
        self.scroll.reveal_focus(document_bounds, backing_scale)
    }
    pub fn content(&self) -> Rect {
        let geometry = self.geometry();
        self.content_with_geometry(&geometry)
    }
    pub fn content_with_geometry(&self, geometry: &tekes_ui_controls::ScrollGeometry) -> Rect {
        let mut content = layout::PAGE_CONTENT;
        if self.fill_window {
            content.size = self.bounds().size;
        }
        content.origin = geometry.content_origin();
        content
    }
    pub fn point_in_content(&self, p: Point) -> Option<Point> {
        let geometry = self.geometry();
        self.point_in_content_with_geometry(p, &geometry)
    }
    pub fn point_in_content_with_geometry(
        &self,
        p: Point,
        geometry: &tekes_ui_controls::ScrollGeometry,
    ) -> Option<Point> {
        let local = geometry.viewport_to_document(p)?;
        let origin = self.bounds().origin;
        Some(Point::new(origin.x + local.x, origin.y + local.y))
    }
    /// The shared viewport/document mapping for this page, optionally clipped
    /// by an ancestor container. Rendering and input code should retain and use
    /// the same snapshot for one pass.
    pub fn geometry(&self) -> tekes_ui_controls::ScrollGeometry {
        self.scroll.geometry(self.bounds(), None)
    }
    pub fn geometry_with_ancestor_clip(
        &self,
        ancestor_clip: Rect,
    ) -> tekes_ui_controls::ScrollGeometry {
        self.scroll.geometry(self.bounds(), Some(ancestor_clip))
    }
    pub fn track(&self, vertical: bool) -> Rect {
        let c = self.bounds();
        let w = tekes_ui_tokens::proposal::SCROLLBAR_WIDTH;
        if vertical {
            Rect::new(
                c.max_x() + Spacing::Regular.value(),
                c.min_y(),
                w,
                c.size.height,
            )
        } else {
            Rect::new(
                c.min_x(),
                c.max_y() + Spacing::Regular.value(),
                c.size.width,
                w,
            )
        }
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        for vertical in [true, false] {
            if self.scroll.scrollbar_visible(vertical) {
                let track = self.track(vertical).inset(-Spacing::Micro.value());
                if self
                    .scroll
                    .pointer_down(p, vertical, track, ControlSize::RowHeight.value())
                {
                    return true;
                }
            }
        }
        false
    }
    pub fn pointer_drag(&mut self, p: Point) -> bool {
        if let Some(axis) = self.scroll.dragging_axis() {
            let track = self.track(axis).inset(-Spacing::Micro.value());
            self.scroll
                .pointer_drag(p, track, ControlSize::RowHeight.value())
        } else {
            false
        }
    }
    pub fn append_visible_with_geometry(
        &self,
        source: &Scene,
        target: &mut Scene,
        theme: Theme,
        geometry: &tekes_ui_controls::ScrollGeometry,
    ) {
        let Some(clip) = geometry.visible_viewport_rect() else {
            return;
        };
        target.push_clip(clip);
        for p in source.primitives() {
            target.push(p.clone());
        }
        target.pop_clip();
        for vertical in [true, false] {
            if self.scroll.scrollbar_visible(vertical) {
                let track = self.track(vertical);
                let thumb = self
                    .scroll
                    .thumb(vertical, track, ControlSize::RowHeight.value());
                target.push_quad(
                    Quad::new(thumb, ColorRole::SecondaryText.resolve(theme))
                        .with_corner_radius(Spacing::Tight.value() * 0.5),
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_short_window_can_reach_the_last_row_and_inverse_hit_coordinates() {
        let mut view = Viewport::default();
        view.resize(1440.0, 827.0);
        assert_eq!(view.limit().y, 133.0);
        assert!(view.move_by(0.0, 99999.0));
        assert_eq!(view.offset.y, 133.0);
        assert!(!view.move_by(0.0, 1.0));
        let p = view.bounds().center();
        assert_eq!(view.point_in_content(p), Some(Point::new(p.x, p.y + 133.0)));
        assert!(view.point_in_content(Point::new(10.0, 10.0)).is_none());
        assert!(!view.move_by(f32::NAN, 0.0));
        view.resize(layout::FRAME_W, layout::FRAME_H);
        assert_eq!(view.offset, Point::default());
    }
}

#[cfg(test)]
mod regression {
    use super::*;
    #[test]
    fn resize_must_not_reset_a_long_document_before_clamping_its_offset() {
        let mut viewport = Viewport::default();
        viewport.resize(1440.0, 827.0);
        viewport.set_document_extent(Size::new(layout::PAGE_CONTENT.size.width, 7000.0));
        assert!(viewport.move_by(0.0, 4000.0));
        viewport.resize(1440.0, 827.0);
        assert_eq!(viewport.offset.y, 4000.0);
        assert!(viewport.limit().y > 6000.0);
        viewport.set_document_extent(layout::PAGE_CONTENT.size);
        assert_eq!(viewport.offset.y, 133.0);
    }

    #[test]
    fn one_snapshot_drives_gallery_content_transform_and_ancestor_hit_clip() {
        let mut viewport = Viewport::default();
        viewport.resize(1440.0, 827.0);
        viewport.set_document_extent(Size::new(1200.0, 1200.0));
        viewport.move_by(40.0, 90.0);
        let bounds = viewport.bounds();
        let ancestor = Rect::new(bounds.min_x() + 20.0, bounds.min_y() + 10.0, 500.0, 400.0);
        let geometry = viewport.geometry_with_ancestor_clip(ancestor);
        let content = viewport.content_with_geometry(&geometry);
        assert_eq!(content.origin, geometry.content_origin());
        let hit = Point::new(ancestor.min_x() + 4.0, ancestor.min_y() + 5.0);
        let document = viewport
            .point_in_content_with_geometry(hit, &geometry)
            .unwrap();
        assert_eq!(
            document,
            Point::new(
                layout::PAGE_CONTENT.min_x() + 24.0 + 40.0,
                layout::PAGE_CONTENT.min_y() + 15.0 + 90.0
            )
        );
        assert!(
            viewport
                .point_in_content_with_geometry(
                    Point::new(bounds.min_x() + 2.0, bounds.min_y() + 2.0),
                    &geometry
                )
                .is_none()
        );
    }
}
