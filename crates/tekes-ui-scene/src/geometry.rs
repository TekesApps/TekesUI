//! 几何类型：逻辑点，原点在左上角，y 向下。

/// 点。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

/// 尺寸。宽高允许为 0，不应为负。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Size {
        Size { width, height }
    }
}

/// 矩形，由左上角原点和尺寸给出。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub const fn from_origin_size(origin: Point, size: Size) -> Rect {
        Rect { origin, size }
    }

    pub const fn min_x(&self) -> f32 {
        self.origin.x
    }

    pub const fn min_y(&self) -> f32 {
        self.origin.y
    }

    pub fn max_x(&self) -> f32 {
        self.origin.x + self.size.width
    }

    pub fn max_y(&self) -> f32 {
        self.origin.y + self.size.height
    }

    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width * 0.5,
            self.origin.y + self.size.height * 0.5,
        )
    }

    /// 四边各向内收 `d`。`d` 为负时向外扩。
    /// 收到宽或高小于 0 时，该方向的尺寸钳到 0，中心保持不动。
    pub fn inset(&self, d: f32) -> Rect {
        let w = (self.size.width - 2.0 * d).max(0.0);
        let h = (self.size.height - 2.0 * d).max(0.0);
        let c = self.center();
        Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h)
    }

    /// 点是否在矩形内。左、上边界包含，右、下边界不包含，
    /// 这样相邻的矩形不会重复命中同一个点。
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.x < self.max_x() && p.y >= self.min_y() && p.y < self.max_y()
    }

    pub fn is_empty(&self) -> bool {
        self.size.width <= 0.0 || self.size.height <= 0.0
    }

    /// 平移。
    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.origin.x + dx,
            self.origin.y + dy,
            self.size.width,
            self.size.height,
        )
    }

    /// 与另一个矩形的交集；不相交时返回 `None`。
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.min_x().max(other.min_x());
        let y0 = self.min_y().max(other.min_y());
        let x1 = self.max_x().min(other.max_x());
        let y1 = self.max_y().min(other.max_y());
        if x1 > x0 && y1 > y0 {
            Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_center() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(r.min_x(), 10.0);
        assert_eq!(r.min_y(), 20.0);
        assert_eq!(r.max_x(), 40.0);
        assert_eq!(r.max_y(), 60.0);
        assert_eq!(r.center(), Point::new(25.0, 40.0));
    }

    #[test]
    fn inset_shrinks_and_grows() {
        let r = Rect::new(10.0, 10.0, 20.0, 10.0);
        assert_eq!(r.inset(2.0), Rect::new(12.0, 12.0, 16.0, 6.0));
        assert_eq!(r.inset(-1.0), Rect::new(9.0, 9.0, 22.0, 12.0));
    }

    #[test]
    fn inset_clamps_to_zero_around_center() {
        let r = Rect::new(0.0, 0.0, 10.0, 4.0);
        let s = r.inset(3.0);
        assert_eq!(s.size, Size::new(4.0, 0.0));
        assert_eq!(s.center(), r.center());
        assert!(s.is_empty());
    }

    #[test]
    fn contains_is_half_open() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(Point::new(0.0, 0.0)));
        assert!(r.contains(Point::new(9.999, 9.999)));
        assert!(!r.contains(Point::new(10.0, 5.0)));
        assert!(!r.contains(Point::new(5.0, 10.0)));
        assert!(!r.contains(Point::new(-0.001, 5.0)));
    }

    #[test]
    fn adjacent_rects_do_not_double_hit() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 10.0, 10.0);
        let p = Point::new(10.0, 5.0);
        assert!(!a.contains(p));
        assert!(b.contains(p));
    }

    #[test]
    fn empty_rect() {
        assert!(Rect::new(0.0, 0.0, 0.0, 5.0).is_empty());
        assert!(Rect::new(0.0, 0.0, 5.0, 0.0).is_empty());
        assert!(!Rect::new(0.0, 0.0, 1.0, 1.0).is_empty());
        assert!(!Rect::new(0.0, 0.0, 0.0, 5.0).contains(Point::new(0.0, 1.0)));
    }

    #[test]
    fn offset_moves_origin_only() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0).offset(10.0, -2.0);
        assert_eq!(r, Rect::new(11.0, 0.0, 3.0, 4.0));
    }

    #[test]
    fn intersection() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 6.0, 10.0, 10.0);
        assert_eq!(a.intersection(&b), Some(Rect::new(5.0, 6.0, 5.0, 4.0)));
        // 只共享边的矩形不算相交。
        let c = Rect::new(10.0, 0.0, 5.0, 5.0);
        assert_eq!(a.intersection(&c), None);
        let d = Rect::new(20.0, 20.0, 1.0, 1.0);
        assert_eq!(a.intersection(&d), None);
    }
}
