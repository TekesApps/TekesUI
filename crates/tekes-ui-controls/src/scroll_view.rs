use tekes_ui_scene::{Point, Rect, Size};

/// Two-axis document viewport. Momentum deltas come from the operating system;
/// only the elastic boundary return is animated locally.
#[derive(Debug, Default)]
pub struct ScrollView {
    pub offset: Point,
    pub overscroll: Point,
    viewport: Size,
    document: Size,
    gesture: bool,
    momentum: bool,
    reduced_motion: bool,
    axis: Option<bool>,
    drag: Option<(bool, f32, f32)>,
}
impl ScrollView {
    pub fn layout(&mut self, viewport: Size, document: Size) {
        let valid = |value: f32| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        self.viewport = Size::new(valid(viewport.width), valid(viewport.height));
        self.document = Size::new(valid(document.width), valid(document.height));
        self.scroll_by(0.0, 0.0);
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.reduced_motion = reduced;
        if reduced {
            let dirty = self.overscroll != Point::default();
            self.overscroll = Point::default();
            dirty
        } else {
            false
        }
    }
    pub fn limit(&self) -> Point {
        Point::new(
            (self.document.width - self.viewport.width).max(0.0),
            (self.document.height - self.viewport.height).max(0.0),
        )
    }
    pub fn visual_offset(&self) -> Point {
        Point::new(
            self.offset.x + self.overscroll.x,
            self.offset.y + self.overscroll.y,
        )
    }
    pub fn scroll_by(&mut self, x: f32, y: f32) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let before = self.offset;
        let l = self.limit();
        self.offset.x = (self.offset.x + x).clamp(0.0, l.x);
        self.offset.y = (self.offset.y + y).clamp(0.0, l.y);
        before != self.offset
    }
    pub fn wheel(
        &mut self,
        mut x: f32,
        mut y: f32,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
    ) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let before = (self.offset, self.overscroll);
        if begin {
            self.momentum = false;
            self.gesture = true;
            self.axis = None;
        }
        if precise && !momentum && self.gesture {
            if self.axis.is_none() && x.abs() + y.abs() > 2.0 {
                if y.abs() > x.abs() * 2.0 {
                    self.axis = Some(true)
                } else if x.abs() > y.abs() * 2.0 {
                    self.axis = Some(false)
                }
            }
            match self.axis {
                Some(true) => x = 0.0,
                Some(false) => y = 0.0,
                None => {}
            }
        }
        if momentum && !end {
            self.momentum = true;
        }
        // Reverse movement first pulls the stretched paper back to its boundary;
        // only the remaining delta enters the document. Never move both at once.
        if precise && !momentum {
            fn consume(stretch: &mut f32, delta: &mut f32) {
                if *stretch * *delta < 0.0 {
                    let pull = delta.abs().min(stretch.abs() / 0.35) * delta.signum();
                    *stretch += pull * 0.35;
                    *delta -= pull;
                    if stretch.abs() < 0.001 {
                        *stretch = 0.0;
                    }
                }
            }
            consume(&mut self.overscroll.x, &mut x);
            consume(&mut self.overscroll.y, &mut y);
        }
        let wanted = Point::new(self.offset.x + x, self.offset.y + y);
        self.scroll_by(x, y);
        if precise && self.gesture && !momentum && !self.reduced_motion {
            let rubber = |value: f32, delta: f32, extent: f32| {
                (value + delta * 0.35).clamp(-extent * 0.25, extent * 0.25)
            };
            self.overscroll.x = rubber(
                self.overscroll.x,
                wanted.x - self.offset.x,
                self.viewport.width,
            );
            self.overscroll.y = rubber(
                self.overscroll.y,
                wanted.y - self.offset.y,
                self.viewport.height,
            );
        }
        if end {
            self.gesture = false;
            self.momentum = false;
            self.axis = None;
        }
        before != (self.offset, self.overscroll)
    }
    pub fn needs_animation(&self) -> bool {
        self.gesture || self.momentum || self.overscroll != Point::default()
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        if !dt.is_finite()
            || dt <= 0.0
            || self.gesture
            || self.momentum
            || self.overscroll == Point::default()
        {
            return false;
        }
        let factor = (-22.0 * dt.clamp(0.0, 0.05)).exp();
        for value in [&mut self.overscroll.x, &mut self.overscroll.y] {
            *value *= factor;
            if value.abs() < 0.1 {
                *value = 0.0;
            }
        }
        true
    }
    /// End input ownership without snapping an elastic return already in flight.
    pub fn end_gesture(&mut self) {
        self.gesture = false;
        self.momentum = false;
        self.axis = None;
    }
    pub fn cancel(&mut self) {
        self.gesture = false;
        self.momentum = false;
        self.axis = None;
        self.drag = None;
        self.overscroll = Point::default();
    }
    pub fn reveal(&mut self, rect: Rect) -> bool {
        let dx = if rect.min_x() < self.offset.x {
            rect.min_x() - self.offset.x
        } else {
            (rect.max_x() - self.offset.x - self.viewport.width).max(0.0)
        };
        let dy = if rect.min_y() < self.offset.y {
            rect.min_y() - self.offset.y
        } else {
            (rect.max_y() - self.offset.y - self.viewport.height).max(0.0)
        };
        self.scroll_by(dx, dy)
    }
    pub fn thumb(&self, vertical: bool, track: Rect, minimum: f32) -> Rect {
        let (view, doc, offset, limit, length) = if vertical {
            (
                self.viewport.height,
                self.document.height,
                self.offset.y,
                self.limit().y,
                track.size.height,
            )
        } else {
            (
                self.viewport.width,
                self.document.width,
                self.offset.x,
                self.limit().x,
                track.size.width,
            )
        };
        let size = if doc > 0.0 {
            (length * view / doc).max(minimum).min(length)
        } else {
            length
        };
        let pos = if limit > 0.0 {
            (length - size) * offset / limit
        } else {
            0.0
        };
        if vertical {
            Rect::new(track.min_x(), track.min_y() + pos, track.size.width, size)
        } else {
            Rect::new(track.min_x() + pos, track.min_y(), size, track.size.height)
        }
    }
    pub fn pointer_down(&mut self, p: Point, vertical: bool, track: Rect, minimum: f32) -> bool {
        if !track.contains(p) {
            return false;
        }
        let thumb = self.thumb(vertical, track, minimum);
        let coordinate = if vertical { p.y } else { p.x };
        if thumb.contains(p) {
            self.drag = Some((
                vertical,
                coordinate,
                if vertical {
                    self.offset.y
                } else {
                    self.offset.x
                },
            ));
        } else {
            let forward = coordinate
                > if vertical {
                    thumb.max_y()
                } else {
                    thumb.max_x()
                };
            let delta = if forward { 0.9 } else { -0.9 };
            if vertical {
                self.scroll_by(0.0, self.viewport.height * delta);
            } else {
                self.scroll_by(self.viewport.width * delta, 0.0);
            }
        }
        true
    }
    pub fn pointer_drag(&mut self, p: Point, track: Rect, minimum: f32) -> bool {
        let Some((vertical, start, offset)) = self.drag else {
            return false;
        };
        let thumb = self.thumb(vertical, track, minimum);
        let (delta, travel, limit) = if vertical {
            (
                p.y - start,
                track.size.height - thumb.size.height,
                self.limit().y,
            )
        } else {
            (
                p.x - start,
                track.size.width - thumb.size.width,
                self.limit().x,
            )
        };
        if travel <= 0.0 {
            return false;
        }
        let target = offset + delta / travel * limit;
        if vertical {
            self.scroll_by(0.0, target - self.offset.y)
        } else {
            self.scroll_by(target - self.offset.x, 0.0)
        }
    }
    pub fn dragging_axis(&self) -> Option<bool> {
        self.drag.map(|v| v.0)
    }
    pub fn pointer_up(&mut self) {
        self.drag = None;
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn reverse_gesture_consumes_elastic_stretch_before_scrolling_document() {
        let mut s = ScrollView::default();
        s.layout(Size::new(100.0, 100.0), Size::new(100.0, 500.0));
        s.wheel(0.0, -100.0, true, true, false, false);
        assert_eq!(s.offset.y, 0.0);
        assert_eq!(s.overscroll.y, -25.0);
        s.wheel(0.0, 30.0, true, false, false, false);
        assert_eq!(s.offset.y, 0.0);
        assert!((s.overscroll.y + 14.5).abs() < 0.001);
        s.wheel(0.0, 60.0, true, false, false, false);
        assert_eq!(s.overscroll.y, 0.0);
        assert!((s.offset.y - 18.57143).abs() < 0.001);
    }

    use super::*;
    #[test]
    fn wheel_momentum_elasticity_and_resize() {
        let mut v = ScrollView::default();
        v.layout(Size::new(100., 100.), Size::new(400., 500.));
        v.wheel(1., 50., true, true, false, false);
        assert_eq!(v.offset, Point::new(0., 50.));
        v.wheel(0., 40., true, false, true, false);
        v.wheel(0., 30., true, false, false, true);
        assert_eq!(v.offset.y, 120.);
        v.scroll_by(0., -999.);
        v.wheel(0., -90., true, true, true, false);
        assert!(v.overscroll.y < 0.);
        for _ in 0..120 {
            v.tick(1. / 120.);
        }
        assert_eq!(v.overscroll, Point::default());
        v.layout(Size::new(500., 600.), Size::new(400., 500.));
        assert_eq!(v.offset, Point::default());
        assert!(!v.scroll_by(f32::NAN, 0.));
    }
    #[test]
    fn thumb_capture_drag_and_reveal() {
        let mut v = ScrollView::default();
        v.layout(Size::new(100., 100.), Size::new(100., 1000.));
        let track = Rect::new(95., 0., 5., 100.);
        assert!(v.pointer_down(Point::new(97., 10.), true, track, 20.));
        v.pointer_drag(Point::new(97., 90.), track, 20.);
        assert_eq!(v.offset.y, 900.);
        v.pointer_up();
        v.reveal(Rect::new(0., 20., 50., 10.));
        assert_eq!(v.offset.y, 20.);
    }
}
