//! Shared page viewport; clipping belongs to the renderer, not to geometry mutation.
use super::layout;
use std::ops::{Deref, DerefMut};
use tekes_ui_controls::ScrollArea;
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, ControlSize, Spacing, Theme};
#[derive(Debug)]
pub struct Viewport {
    size: Size,
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
            document: layout::PAGE_CONTENT.size,
            scroll: ScrollArea::default(),
        };
        v.resize(layout::FRAME_W, layout::FRAME_H);
        v
    }
}
impl Viewport {
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
        Rect::new(
            c.min_x(),
            c.min_y(),
            c.size
                .width
                .min((self.size.width - c.min_x() - Spacing::Section.value()).max(0.0)),
            c.size
                .height
                .min((self.size.height - c.min_y() - Spacing::Section.value()).max(0.0)),
        )
    }
    pub fn move_by(&mut self, x: f32, y: f32) -> bool {
        self.scroll.scroll_by(x, y)
    }
    pub fn content(&self) -> Rect {
        let p = self.scroll.visual_offset();
        layout::PAGE_CONTENT.offset(-p.x, -p.y)
    }
    pub fn point_in_content(&self, p: Point) -> Option<Point> {
        let o = self.scroll.visual_offset();
        self.bounds()
            .contains(p)
            .then_some(Point::new(p.x + o.x, p.y + o.y))
    }
    pub fn track(&self, vertical: bool) -> Rect {
        let c = self.bounds();
        let w = Spacing::Tight.value();
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
            let limit = self.limit();
            if (if vertical { limit.y } else { limit.x }) > 0.0 {
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
    pub fn append_visible(&self, source: &Scene, target: &mut Scene, _scale: f32, theme: Theme) {
        target.push_clip(self.bounds());
        for p in source.primitives() {
            target.push(p.clone());
        }
        target.pop_clip();
        let limit = self.limit();
        for vertical in [true, false] {
            if (if vertical { limit.y } else { limit.x }) > 0.0 {
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
}
