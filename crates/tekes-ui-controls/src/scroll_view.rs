use crate::ScrollGeometry;
use tekes_ui_scene::{Point, Rect, Size};
use tekes_ui_tokens::proposal;
include!(concat!(env!("OUT_DIR"), "/scroll_tokens.rs"));
pub(crate) mod reveal;
pub use reveal::{
    ScrollRevealError, ScrollRevealOptions, ScrollRevealPlacement, ScrollRevealResult,
};
#[derive(Clone, Debug)]
struct Rebound {
    initial: Point,
    velocity: Point,
    elapsed: f64,
}

/// Per-axis scrolling policy, independent of scrollbar appearance.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPolicy {
    /// Accept input and allow elastic movement even when the document fits.
    Enabled,
    /// Reject input, elastic movement, and programmatic scrolling on this axis.
    Disabled,
    /// Accept input only while the document exceeds the viewport on this axis.
    #[default]
    Auto,
}

type ScrollPointerInputs = (
    Point,
    Point,
    Size,
    Size,
    (ScrollPolicy, ScrollPolicy),
    (bool, bool),
    u64,
);
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScrollPointerDecision {
    expected: ScrollPointerInputs,
    drag: Option<(bool, f32, f32)>,
    offset: Point,
}
impl ScrollPointerDecision {
    pub(crate) fn visual_offset(&self) -> Point {
        self.offset
    }
}

/// Two-axis document viewport. Momentum deltas come from the operating system;
/// elastic boundary return and explicit reveal navigation animate locally using
/// the host clock. No synthetic in-bounds momentum is introduced.
#[derive(Clone, Debug, Default)]
pub struct ScrollView {
    pub offset: Point,
    pub overscroll: Point,
    horizontal: ScrollPolicy,
    vertical: ScrollPolicy,
    hidden_scrollbars: (bool, bool),
    viewport: Size,
    document: Size,
    gesture: bool,
    clock: f64,
    last_input: f64,
    last_motion: f64,
    last_velocity: Point,
    rebound: Option<Rebound>,
    reveal_animation: Option<reveal::RevealTransition>,
    reveal_generation: u64,
    ignore_momentum: bool,
    entry_x: bool,
    entry_y: bool,
    momentum: bool,
    reduced_motion: bool,
    axis: Option<bool>,
    drag: Option<(bool, f32, f32)>,
}
impl ScrollView {
    /// Configure each direction independently. Changing policy ends any active
    /// gesture and clamps offsets immediately, including a disabled direction.
    pub fn set_scroll_policy(&mut self, horizontal: ScrollPolicy, vertical: ScrollPolicy) {
        if (self.horizontal, self.vertical) != (horizontal, vertical) {
            self.cancel();
            self.horizontal = horizontal;
            self.vertical = vertical;
            self.scroll_by(0.0, 0.0);
        }
    }
    pub fn scroll_policy(&self) -> (ScrollPolicy, ScrollPolicy) {
        (self.horizontal, self.vertical)
    }
    /// Show overflow indicators independently of scrolling permissions. Both are
    /// shown by default; hiding one removes its pointer target, not its range.
    pub fn set_scrollbar_visibility(&mut self, horizontal: bool, vertical: bool) {
        self.hidden_scrollbars = (!horizontal, !vertical);
        if self
            .drag
            .is_some_and(|(axis, _, _)| !self.scrollbar_visible(axis))
        {
            self.drag = None;
        }
    }
    pub fn scrollbar_visible(&self, vertical: bool) -> bool {
        let (hidden, limit) = if vertical {
            (self.hidden_scrollbars.1, self.limit().y)
        } else {
            (self.hidden_scrollbars.0, self.limit().x)
        };
        !hidden && limit > 0.0
    }
    pub fn axis_enabled(&self, vertical: bool) -> bool {
        let (policy, document, viewport) = if vertical {
            (self.vertical, self.document.height, self.viewport.height)
        } else {
            (self.horizontal, self.document.width, self.viewport.width)
        };
        match policy {
            ScrollPolicy::Enabled => true,
            ScrollPolicy::Disabled => false,
            ScrollPolicy::Auto => document > viewport,
        }
    }

    /// Update geometry without ending a still-valid native scroll sequence.
    /// Streaming/reflow is not a new input owner. A thumb drag uses obsolete
    /// track geometry and is released; unavailable directions and illegal
    /// elastic displacement are cleared. Explicit lifecycle/policy changes
    /// continue to use `cancel`.
    pub fn layout(&mut self, viewport: Size, document: Size) {
        let valid = |value: f32| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        let viewport = Size::new(valid(viewport.width), valid(viewport.height));
        let document = Size::new(valid(document.width), valid(document.height));
        let changed = viewport != self.viewport || document != self.document;
        let previous_offset = self.offset;
        let previous_stretch = self.overscroll;
        let previous_limit = self.limit();
        self.viewport = viewport;
        self.document = document;
        self.scroll_by(0.0, 0.0);
        if !changed {
            return;
        }
        self.drag = None;
        self.reveal_animation = None;
        let enabled = (self.axis_enabled(false), self.axis_enabled(true));
        if !enabled.0 && !enabled.1 {
            self.cancel();
            return;
        }
        let limit = self.limit();
        let valid_stretch =
            |stretch: f32, offset: f32, previous: f32, limit: f32, extent: f32, enabled: bool| {
                if !enabled
                    || offset != previous
                    || (stretch < 0.0 && offset > 0.0)
                    || (stretch > 0.0 && offset < limit)
                {
                    0.0
                } else {
                    let maximum = extent * MAXIMUM_STRETCH_FRACTION as f32;
                    stretch.clamp(-maximum, maximum)
                }
            };
        self.overscroll = Point::new(
            valid_stretch(
                previous_stretch.x,
                self.offset.x,
                previous_offset.x,
                limit.x,
                viewport.width,
                enabled.0,
            ),
            valid_stretch(
                previous_stretch.y,
                self.offset.y,
                previous_offset.y,
                limit.y,
                viewport.height,
                enabled.1,
            ),
        );
        // Keep an existing return's phase/trajectory when it is still legal.
        // If reflow clamps one direction, remove only that contribution.
        if let Some(rebound) = &mut self.rebound {
            let ratio = |before: f32, after: f32| {
                if before == 0.0 { 0.0 } else { after / before }
            };
            let x = ratio(previous_stretch.x, self.overscroll.x);
            let y = ratio(previous_stretch.y, self.overscroll.y);
            rebound.initial.x *= x;
            rebound.velocity.x *= x;
            rebound.initial.y *= y;
            rebound.velocity.y *= y;
            if rebound.initial == Point::default() {
                self.rebound = None;
            }
        }
        let edge_became_interior = |before: f32, after: f32, offset: f32| {
            after > before && offset >= before && offset > 0.0 && offset < after
        };
        if self.rebound.is_none()
            && (edge_became_interior(previous_limit.x, limit.x, self.offset.x)
                || edge_became_interior(previous_limit.y, limit.y, self.offset.y))
        {
            // A former bottom/right edge may now be ordinary document. The
            // old elastic return no longer owns incoming native momentum.
            self.ignore_momentum = false;
        }
        if !enabled.0 {
            self.last_velocity.x = 0.0;
            self.entry_x = false;
            if self.axis == Some(false) {
                self.axis = None;
            }
        }
        if !enabled.1 {
            self.last_velocity.y = 0.0;
            self.entry_y = false;
            if self.axis == Some(true) {
                self.axis = None;
            }
        }
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        self.reduced_motion = reduced;
        if reduced {
            let reveal_changed = self.finish_reveal();
            let dirty = self.overscroll != Point::default() || reveal_changed;
            self.overscroll = Point::default();
            self.rebound = None;
            dirty
        } else {
            false
        }
    }
    pub fn limit(&self) -> Point {
        Point::new(
            if self.axis_enabled(false) {
                (self.document.width - self.viewport.width).max(0.0)
            } else {
                0.0
            },
            if self.axis_enabled(true) {
                (self.document.height - self.viewport.height).max(0.0)
            } else {
                0.0
            },
        )
    }
    pub fn visual_offset(&self) -> Point {
        Point::new(
            self.offset.x + self.overscroll.x,
            self.offset.y + self.overscroll.y,
        )
    }
    /// Capture the current scroll transform for a single layout/render/input
    /// pass. Consumers should reuse this value for content placement, clipping,
    /// and hit conversion so those paths cannot drift apart.
    pub fn geometry(&self, viewport: Rect, ancestor_clip: Option<Rect>) -> ScrollGeometry {
        ScrollGeometry::new(
            viewport,
            self.document,
            self.offset,
            self.overscroll,
            ancestor_clip,
        )
    }

    pub fn viewport_size(&self) -> Size {
        self.viewport
    }

    pub fn document_size(&self) -> Size {
        self.document
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
        x: f32,
        y: f32,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
    ) -> bool {
        self.wheel_at(Point::new(x, y), precise, begin, end, momentum, self.clock)
    }
    /// Same input delta and timestamp domain as the operating system. No
    /// synthetic in-bounds inertia: incoming momentum is already integrated.
    pub fn wheel_at(
        &mut self,
        delta: Point,
        precise: bool,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
    ) -> bool {
        let (mut x, mut y) = (delta.x, delta.y);
        if !x.is_finite() || !y.is_finite() || !time.is_finite() {
            return false;
        }
        if self.drag.is_some() {
            return false;
        }
        if !self.axis_enabled(false) {
            x = 0.0;
        }
        if !self.axis_enabled(true) {
            y = 0.0;
        }
        if momentum && self.ignore_momentum && self.reveal_animation.is_some() {
            return false;
        }
        if !momentum && (begin || x != 0.0 || y != 0.0) {
            self.reveal_animation = None;
            self.ignore_momentum = false;
        }
        let before = (self.offset, self.overscroll);
        let elapsed = (time - self.last_motion).max(0.0);
        self.clock = self.clock.max(time);
        if begin {
            let limit = self.limit();
            self.entry_x = self.offset.x > 0.0 && self.offset.x < limit.x;
            self.entry_y = self.offset.y > 0.0 && self.offset.y < limit.y;
            self.momentum = false;
            self.gesture = true;
            self.axis = None;
            self.rebound = None;
            self.ignore_momentum = false;
            self.last_velocity = Point::default();
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
        if x != 0.0 || y != 0.0 {
            let dt = if elapsed > 0.0001 && elapsed < VELOCITY_TIMEOUT_S {
                elapsed
            } else {
                1.0 / NOMINAL_HZ
            };
            self.last_velocity = Point::new((f64::from(x) / dt) as f32, (f64::from(y) / dt) as f32);
        } else if elapsed > VELOCITY_TIMEOUT_S {
            self.last_velocity = Point::default();
        }
        if x != 0.0 || y != 0.0 || begin {
            self.last_motion = time;
        }
        self.last_input = time;
        if momentum && self.rebound.is_some() {
            self.ignore_momentum = true;
        }
        if momentum && !end {
            self.momentum = true;
        }
        if !(momentum && self.ignore_momentum) {
            if precise && !momentum {
                fn consume(stretch: &mut f32, delta: &mut f32) {
                    if *stretch * *delta < 0.0 {
                        let pull =
                            delta.abs().min(stretch.abs() * STIFFNESS as f32) * delta.signum();
                        *stretch += pull / STIFFNESS as f32;
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
            if precise && (self.gesture || momentum) && !self.reduced_motion {
                let rubber = |value: f32, delta: f32, extent: f32, entry: &mut bool| {
                    // Only momentum carries its first interior delta across the
                    // edge. Finger input is resisted from the first excess:
                    // displaying it 1:1 but unwinding at 1/STIFFNESS would turn
                    // a 10pt excess into 200pt of reverse travel.
                    let gain = if *entry && value == 0.0 && delta != 0.0 {
                        *entry = false;
                        if momentum {
                            1.0
                        } else {
                            1.0 / STIFFNESS as f32
                        }
                    } else {
                        1.0 / STIFFNESS as f32
                    };
                    (value + delta * gain).clamp(
                        -extent * MAXIMUM_STRETCH_FRACTION as f32,
                        extent * MAXIMUM_STRETCH_FRACTION as f32,
                    )
                };
                self.overscroll.x = rubber(
                    self.overscroll.x,
                    wanted.x - self.offset.x,
                    self.viewport.width,
                    &mut self.entry_x,
                );
                self.overscroll.y = rubber(
                    self.overscroll.y,
                    wanted.y - self.offset.y,
                    self.viewport.height,
                    &mut self.entry_y,
                );
                if momentum && self.overscroll != Point::default() {
                    self.ignore_momentum = true;
                    self.start_rebound();
                }
            }
        }
        let limit = self.limit();
        if self.overscroll.x == 0.0 && self.offset.x > 0.0 && self.offset.x < limit.x {
            self.entry_x = true;
        }
        if self.overscroll.y == 0.0 && self.offset.y > 0.0 && self.offset.y < limit.y {
            self.entry_y = true;
        }
        if end {
            self.gesture = false;
            self.momentum = false;
            self.axis = None;
            if self.rebound.is_none() {
                self.start_rebound();
            }
        }
        before != (self.offset, self.overscroll)
    }
    fn start_rebound(&mut self) {
        if self.clock - self.last_motion > VELOCITY_TIMEOUT_S {
            self.last_velocity = Point::default();
        }
        if self.overscroll != Point::default() && !self.reduced_motion {
            self.rebound = Some(Rebound {
                initial: self.overscroll,
                velocity: self.last_velocity,
                elapsed: 0.0,
            });
        }
    }
    pub fn needs_animation(&self) -> bool {
        self.reveal_animation.is_some()
            || self.rebound.is_some()
            || ((self.gesture || self.momentum)
                && (self.clock - self.last_input) < VELOCITY_TIMEOUT_S)
    }
    pub fn tick(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        self.clock += f64::from(dt);
        if let Some(animation) = &mut self.reveal_animation {
            let (target, finished) = animation.sample(dt);
            if finished {
                self.reveal_animation = None;
            }
            return self.scroll_by(target.x - self.offset.x, target.y - self.offset.y);
        }
        let Some(rebound) = &mut self.rebound else {
            return false;
        };
        rebound.elapsed += f64::from(dt);
        let t = rebound.elapsed;
        let decay = (-STIFFNESS * t / RELEASE_PERIOD).exp();
        let axis = |initial: f32, velocity: f32| {
            let p =
                ((f64::from(initial) + f64::from(velocity) * RELEASE_AMPLITUDE * t) * decay) as f32;
            if p * initial <= 0.0 || p.abs() < SETTLE_POSITION_PT as f32 {
                0.0
            } else {
                p
            }
        };
        self.overscroll = Point::new(
            axis(rebound.initial.x, rebound.velocity.x),
            axis(rebound.initial.y, rebound.velocity.y),
        );
        if self.overscroll == Point::default() {
            self.rebound = None;
        }
        true
    }
    /// End input ownership without snapping an elastic return already in flight.
    pub fn end_gesture(&mut self) {
        self.gesture = false;
        self.momentum = false;
        self.axis = None;
        if self.rebound.is_none() {
            self.start_rebound();
        }
    }
    pub fn cancel(&mut self) {
        self.reveal_animation = None;
        self.gesture = false;
        self.momentum = false;
        self.axis = None;
        self.drag = None;
        self.overscroll = Point::default();
        self.rebound = None;
        self.last_velocity = Point::default();
        self.ignore_momentum = false;
    }
    pub fn reveal(&mut self, rect: Rect) -> bool {
        self.reveal_animation = None;
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

    /// Scrolls enough to expose the complete keyboard focus ring, including a
    /// one-physical-pixel antialiasing guard, while respecting this real
    /// viewport's bounds. `rect` uses document coordinates.
    pub fn reveal_focus(&mut self, rect: Rect, backing_scale: f32) -> bool {
        if !rect.origin.x.is_finite()
            || !rect.origin.y.is_finite()
            || !rect.size.width.is_finite()
            || !rect.size.height.is_finite()
            || rect.size.width < 0.0
            || rect.size.height < 0.0
        {
            return false;
        }
        let ring_width = proposal::metric("stroke", "c-focus-width").expect("focus width token");
        let ring_gap = proposal::metric("spacing", "c-focus-offset").expect("focus offset token");
        let scale = if backing_scale.is_finite() && backing_scale > 0.0 {
            backing_scale
        } else {
            1.0
        };
        let outset =
            ring_width + ring_gap + proposal::FOCUS_ANTIALIAS_GUARD_PHYSICAL_PIXELS / scale;
        let bounds = Rect::new(
            rect.min_x() - outset,
            rect.min_y() - outset,
            rect.size.width + outset * 2.0,
            rect.size.height + outset * 2.0,
        );
        self.reveal(bounds)
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
        let stretch = if vertical {
            self.overscroll.y
        } else {
            self.overscroll.x
        };
        // Native NSScroller uses (viewport - elastic excess) / document.
        // Apply the same visible fraction to minimum-sized thumbs as well;
        // otherwise long documents would show no feedback at their boundaries.
        let visible_fraction = if view > 0.0 {
            ((view - stretch.abs()) / view).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let size = if doc > 0.0 {
            (length * view / doc).max(minimum).min(length) * visible_fraction
        } else {
            length * visible_fraction
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
    pub(crate) fn prepare_pointer_down(
        &self,
        p: Point,
        vertical: bool,
        track: Rect,
        minimum: f32,
    ) -> Option<ScrollPointerDecision> {
        if !self.scrollbar_visible(vertical) || !track.contains(p) {
            return None;
        }
        // `pointer_down` cancels elastic return before measuring the thumb.
        // Compute that same decision on a clone so preparation stays pure.
        let mut after_cancel = self.clone();
        after_cancel.cancel();
        let thumb = after_cancel.thumb(vertical, track, minimum);
        let coordinate = if vertical { p.y } else { p.x };
        let mut offset = after_cancel.offset;
        let drag = if thumb.contains(p) {
            Some((
                vertical,
                coordinate,
                if vertical {
                    after_cancel.offset.y
                } else {
                    after_cancel.offset.x
                },
            ))
        } else {
            let forward = coordinate
                > if vertical {
                    thumb.max_y()
                } else {
                    thumb.max_x()
                };
            let delta = if forward { 0.9 } else { -0.9 };
            if vertical {
                after_cancel.scroll_by(0.0, after_cancel.viewport.height * delta);
            } else {
                after_cancel.scroll_by(after_cancel.viewport.width * delta, 0.0);
            }
            offset = after_cancel.offset;
            None
        };
        Some(ScrollPointerDecision {
            expected: (
                self.offset,
                self.overscroll,
                self.viewport,
                self.document,
                self.scroll_policy(),
                self.hidden_scrollbars,
                self.reveal_generation,
            ),
            drag,
            offset,
        })
    }
    pub(crate) fn apply_pointer_down(&mut self, decision: ScrollPointerDecision) {
        debug_assert_eq!(
            decision.expected,
            (
                self.offset,
                self.overscroll,
                self.viewport,
                self.document,
                self.scroll_policy(),
                self.hidden_scrollbars,
                self.reveal_generation,
            )
        );
        self.cancel();
        self.drag = decision.drag;
        self.offset = decision.offset;
    }
    pub fn pointer_down(&mut self, p: Point, vertical: bool, track: Rect, minimum: f32) -> bool {
        let Some(decision) = self.prepare_pointer_down(p, vertical, track, minimum) else {
            return false;
        };
        self.apply_pointer_down(decision);
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
    fn hidden_scrollbars_preserve_range_and_input_but_cannot_capture() {
        let mut view = ScrollView::default();
        view.layout(Size::new(100., 100.), Size::new(400., 400.));
        let track = Rect::new(95., 0., 5., 100.);
        assert!(view.scrollbar_visible(true));
        view.set_scrollbar_visibility(true, false);
        assert!(view.scrollbar_visible(false));
        assert!(!view.scrollbar_visible(true));
        assert!(!view.pointer_down(Point::new(97., 10.), true, track, 20.));
        assert!(view.wheel(0., 20., true, true, true, false));
        assert!(view.offset.y > 0.);
        view.scroll_by(20., 30.);
        assert_eq!(view.limit(), Point::new(300., 300.));
        let offset = view.offset;
        view.set_scrollbar_visibility(true, true);
        assert_eq!(view.offset, offset);
        assert!(view.pointer_down(
            Point::new(97., view.thumb(true, track, 20.).center().y),
            true,
            track,
            20.
        ));
        view.set_scrollbar_visibility(false, false);
        assert_eq!(view.dragging_axis(), None);
    }

    #[test]
    fn prepared_scrollbar_pointer_decision_is_pure_fenced_and_matches_thumb_and_page_actions() {
        let track = Rect::new(95.0, 0.0, 5.0, 100.0);
        for (point, expected_drag, expected_offset_y) in [
            (Point::new(97.0, 40.0), true, 120.0),
            (Point::new(97.0, 80.0), false, 210.0),
        ] {
            let mut prepared_view = ScrollView::default();
            prepared_view.layout(Size::new(100.0, 100.0), Size::new(400.0, 400.0));
            prepared_view.scroll_by(0.0, 120.0);
            prepared_view.overscroll.y = -8.0;
            let original_offset = prepared_view.offset;
            let original_stretch = prepared_view.overscroll;
            let decision = prepared_view
                .prepare_pointer_down(point, true, track, 20.0)
                .unwrap();
            assert_eq!(prepared_view.offset, original_offset);
            assert_eq!(prepared_view.overscroll, original_stretch);
            assert_eq!(decision.drag.is_some(), expected_drag);
            assert_eq!(decision.visual_offset(), Point::new(0.0, expected_offset_y));

            let mut immediate_view = prepared_view.clone();
            assert!(immediate_view.pointer_down(point, true, track, 20.0));
            let mut applied_view = prepared_view.clone();
            applied_view.apply_pointer_down(decision);
            assert_eq!(applied_view.offset, immediate_view.offset);
            assert_eq!(applied_view.offset, Point::new(0.0, expected_offset_y));
            assert_eq!(applied_view.overscroll, immediate_view.overscroll);
            assert_eq!(applied_view.overscroll, Point::default());
            assert_eq!(applied_view.dragging_axis(), immediate_view.dragging_axis());
            assert_eq!(applied_view.dragging_axis(), expected_drag.then_some(true));
            assert!(!applied_view.needs_animation());
        }

        let mut view = ScrollView::default();
        view.layout(Size::new(100.0, 100.0), Size::new(400.0, 400.0));
        let decision = view
            .prepare_pointer_down(Point::new(97.0, 10.0), true, track, 20.0)
            .unwrap();
        view.scroll_by(0.0, 1.0);
        let current = view
            .prepare_pointer_down(Point::new(97.0, 10.0), true, track, 20.0)
            .unwrap();
        assert_ne!(decision, current);
    }

    #[test]
    fn replay_measured_native_top_and_bottom_momentum_traces() {
        for (source, tolerance) in [
            (
                include_str!("../../../design/reviews/scroll-repair/native-top.json"),
                2.0,
            ),
            (
                include_str!("../../../design/reviews/scroll-repair/native-momentum.json"),
                6.0,
            ),
        ] {
            let data: serde_json::Value = serde_json::from_str(source).unwrap();
            let inputs = data["inputs"].as_array().unwrap();
            let samples = data["samples"].as_array().unwrap();
            let mut model = ScrollView::default();
            model.layout(Size::new(600.0, 400.0), Size::new(600.0, 3000.0));
            model.scroll_by(0.0, data["start"].as_f64().unwrap() as f32);
            let mut input_index = 0;
            let mut target = samples[0][1].as_f64().unwrap() - 1.0 / 120.0;
            let mut errors = Vec::new();
            for row in samples {
                let time = row[0].as_f64().unwrap();
                while input_index < inputs.len() && inputs[input_index][0].as_f64().unwrap() <= time
                {
                    let event = &inputs[input_index];
                    let phase = event[3].as_u64().unwrap();
                    let momentum = event[4].as_u64().unwrap();
                    model.wheel_at(
                        Point::new(
                            -event[1].as_f64().unwrap() as f32,
                            -event[2].as_f64().unwrap() as f32,
                        ),
                        true,
                        phase == 1,
                        phase == 8 || momentum == 8,
                        momentum != 0,
                        event[0].as_f64().unwrap(),
                    );
                    input_index += 1;
                }
                let next = row[1].as_f64().unwrap();
                model.tick((next - target) as f32);
                target = next;
                if time >= 0.0 {
                    errors.push(f64::from(model.visual_offset().y) - row[2].as_f64().unwrap());
                }
            }
            let rmse = (errors.iter().map(|x| x * x).sum::<f64>() / errors.len() as f64).sqrt();
            assert!(
                rmse < tolerance,
                "native trace RMSE {rmse} pt exceeds {tolerance}"
            );
        }
    }

    #[test]
    fn elastic_thumb_matches_native_visible_proportion_and_stays_edge_anchored() {
        let mut view = ScrollView::default();
        view.layout(Size::new(600.0, 400.0), Size::new(600.0, 3000.0));
        let track = Rect::new(590.0, 0.0, 4.0, 400.0);
        let rest = view.thumb(true, track, 20.0);
        view.overscroll.y = -31.0; // native capture: knobProportion = 0.123
        let top = view.thumb(true, track, 20.0);
        assert!((top.size.height / track.size.height - 0.123).abs() < 0.00001);
        assert_eq!(top.min_y(), track.min_y());
        assert!(top.size.height < rest.size.height);
        view.offset.y = view.limit().y;
        view.overscroll.y = 31.0;
        let bottom = view.thumb(true, track, 20.0);
        assert_eq!(bottom.size.height, top.size.height);
        assert!((bottom.max_y() - track.max_y()).abs() < 0.0001);
        view.end_gesture();
        for _ in 0..240 {
            view.tick(1.0 / 120.0);
        }
        assert_eq!(view.thumb(true, track, 20.0).size.height, rest.size.height);
        view.layout(Size::new(600.0, 400.0), Size::new(600.0, 30000.0));
        view.overscroll.y = -31.0;
        assert!(view.thumb(true, track, 32.0).size.height < 32.0);
    }

    #[test]
    fn elastic_return_is_refresh_independent_and_stationary_release_loses_velocity() {
        fn sample(hz: u32, stationary: bool) -> f32 {
            let mut view = ScrollView::default();
            view.layout(Size::new(100.0, 400.0), Size::new(100.0, 3000.0));
            view.wheel_at(Point::new(0.0, -400.0), true, true, false, false, 1.0);
            if stationary {
                view.tick(0.2);
            }
            view.end_gesture();
            for _ in 0..hz / 5 {
                view.tick(1.0 / hz as f32);
            }
            view.visual_offset().y
        }
        assert!((sample(60, false) - sample(120, false)).abs() < 0.001);
        assert!(sample(120, true).abs() < sample(120, false).abs());
    }

    #[test]
    fn policies_are_independent_for_all_nine_axis_combinations() {
        for horizontal in [
            ScrollPolicy::Enabled,
            ScrollPolicy::Disabled,
            ScrollPolicy::Auto,
        ] {
            for vertical in [
                ScrollPolicy::Enabled,
                ScrollPolicy::Disabled,
                ScrollPolicy::Auto,
            ] {
                let mut view = ScrollView::default();
                view.set_scroll_policy(horizontal, vertical);
                view.layout(Size::new(100.0, 100.0), Size::new(300.0, 300.0));
                view.wheel(30.0, 30.0, true, true, false, false);
                assert_eq!(
                    view.offset.x,
                    if horizontal == ScrollPolicy::Disabled {
                        0.0
                    } else {
                        30.0
                    }
                );
                assert_eq!(
                    view.offset.y,
                    if vertical == ScrollPolicy::Disabled {
                        0.0
                    } else {
                        30.0
                    }
                );
                view.wheel(-500.0, -500.0, true, false, true, false);
                assert_eq!(
                    view.overscroll.x != 0.0,
                    horizontal != ScrollPolicy::Disabled
                );
                assert_eq!(view.overscroll.y != 0.0, vertical != ScrollPolicy::Disabled);
            }
        }
    }

    #[test]
    fn auto_tracks_overflow_and_enabled_allows_elasticity_on_fitting_content() {
        let mut view = ScrollView::default();
        view.layout(Size::new(100.0, 100.0), Size::new(100.0, 100.0));
        assert!(!view.wheel(-50.0, -50.0, true, true, true, false));
        assert_eq!(view.visual_offset(), Point::default());
        view.set_scroll_policy(ScrollPolicy::Enabled, ScrollPolicy::Auto);
        view.wheel(-50.0, -50.0, true, true, true, false);
        assert!(view.overscroll.x < 0.0);
        assert_eq!(view.overscroll.y, 0.0);
        view.layout(Size::new(50.0, 50.0), Size::new(100.0, 100.0));
        view.scroll_by(30.0, 30.0);
        assert_eq!(view.offset, Point::new(30.0, 30.0));
        view.layout(Size::new(200.0, 200.0), Size::new(100.0, 100.0));
        assert_eq!(view.visual_offset(), Point::default());
        assert!(!view.axis_enabled(true));
    }

    #[test]
    fn disabling_axis_clears_rebound_and_blocks_reveal_and_scrollbar_capture() {
        let mut view = ScrollView::default();
        view.layout(Size::new(100.0, 100.0), Size::new(400.0, 400.0));
        view.wheel(-100.0, -100.0, true, true, true, false);
        assert!(view.needs_animation());
        view.set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
        assert_eq!(view.visual_offset(), Point::default());
        view.reveal(Rect::new(200.0, 200.0, 10.0, 10.0));
        assert_eq!(view.offset.x, 0.0);
        assert_eq!(view.offset.y, 110.0);
        assert!(!view.pointer_down(
            Point::new(10.0, 97.0),
            false,
            Rect::new(0.0, 95.0, 100.0, 5.0),
            20.0
        ));
        view.wheel(100.0, 10.0, true, true, false, false);
        assert_eq!(view.offset.x, 0.0);
        assert_eq!(view.offset.y, 120.0); // Filter disabled X before dominant-axis locking.
    }

    #[test]
    fn finger_edge_reversal_matches_native_active_phase() {
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../../design/reviews/scroll-repair/native-edge-reverse.json"
        ))
        .unwrap();
        let mut s = ScrollView::default();
        s.layout(Size::new(600.0, 400.0), Size::new(600.0, 3000.0));
        s.scroll_by(0.0, data["start"].as_f64().unwrap() as f32);
        let inputs = data["inputs"].as_array().unwrap();
        let mut index = 0;
        let mut previous = s.visual_offset().y;
        for row in data["samples"].as_array().unwrap() {
            let time = row[0].as_f64().unwrap();
            while index < inputs.len() && inputs[index][0].as_f64().unwrap() <= time {
                let e = &inputs[index];
                s.wheel_at(
                    Point::new(0.0, -e[2].as_f64().unwrap() as f32),
                    true,
                    e[3] == 1,
                    e[3] == 8,
                    false,
                    e[0].as_f64().unwrap(),
                );
                index += 1;
            }
            // Native bounds publication trails input by one captured display
            // sample. This comparison excludes all post-release coasting.
            assert!((f64::from(previous) - row[2].as_f64().unwrap()).abs() <= 1.0);
            previous = s.visual_offset().y;
        }
    }

    #[test]
    fn finger_reversal_at_either_edge_does_not_accumulate_reverse_travel() {
        for bottom in [false, true] {
            let mut s = ScrollView::default();
            s.layout(Size::new(100.0, 400.0), Size::new(100.0, 3000.0));
            let initial = if bottom { s.limit().y - 20.0 } else { 20.0 };
            let outward = if bottom { 30.0 } else { -30.0 };
            s.scroll_by(0.0, initial);
            for gesture in 0..100 {
                let time = gesture as f64;
                s.wheel_at(Point::default(), true, true, false, false, time);
                s.wheel_at(
                    Point::new(0.0, outward),
                    true,
                    false,
                    false,
                    false,
                    time + 0.01,
                );
                assert!((s.overscroll.y.abs() - 0.5).abs() < 0.001);
                s.wheel_at(
                    Point::new(0.0, -outward),
                    true,
                    false,
                    false,
                    false,
                    time + 0.02,
                );
                assert_eq!(s.overscroll.y, 0.0);
                assert!((s.offset.y - initial).abs() < 0.001, "gesture {gesture}");
                s.wheel_at(Point::default(), true, false, true, false, time + 0.03);
                assert!(!s.needs_animation());
            }
            // Reversing repeatedly without lifting the fingers must also keep
            // the same response after every crossing of the boundary.
            s.wheel_at(Point::default(), true, true, false, false, 101.0);
            for step in 0..100 {
                let time = 101.0 + step as f64 * 0.02;
                s.wheel_at(Point::new(0.0, outward), true, false, false, false, time);
                s.wheel_at(
                    Point::new(0.0, -outward),
                    true,
                    false,
                    false,
                    false,
                    time + 0.01,
                );
                assert_eq!(s.overscroll.y, 0.0);
                assert!((s.offset.y - initial).abs() < 0.001);
            }
        }
    }

    #[test]
    fn reverse_gesture_consumes_elastic_stretch_before_scrolling_document() {
        let mut s = ScrollView::default();
        s.layout(Size::new(100.0, 100.0), Size::new(100.0, 500.0));
        s.wheel(0.0, -100.0, true, true, false, false);
        assert_eq!(s.offset.y, 0.0);
        assert_eq!(s.overscroll.y, -5.0);
        s.wheel(0.0, 30.0, true, false, false, false);
        assert_eq!(s.offset.y, 0.0);
        assert!((s.overscroll.y + 3.5).abs() < 0.001);
        s.wheel(0.0, 80.0, true, false, false, false);
        assert_eq!(s.overscroll.y, 0.0);
        assert!((s.offset.y - 10.0).abs() < 0.001);
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

    #[test]
    fn reveal_focus_keeps_ring_gap_and_one_physical_pixel_inside_viewport() {
        let ring_outset = proposal::metric("spacing", "c-focus-offset").unwrap()
            + proposal::metric("stroke", "c-focus-width").unwrap();
        for scale in [1.0_f32, 2.0] {
            let mut view = ScrollView::default();
            view.layout(Size::new(100.0, 100.0), Size::new(100.0, 500.0));
            let control = Rect::new(10.0, 94.0, 30.0, 20.0);
            assert!(view.reveal_focus(control, scale));
            let guard = 1.0 / scale;
            let expected = control.max_y() + ring_outset + guard - 100.0;
            assert!((view.offset.y - expected).abs() < 0.001);
            assert!(!view.reveal_focus(Rect::new(f32::NAN, 0.0, 1.0, 1.0), scale));
        }
    }

    #[test]
    fn geometry_snapshot_is_shared_by_content_clip_and_hit_coordinates() {
        let mut scroll = ScrollView::default();
        scroll.layout(Size::new(100.0, 80.0), Size::new(300.0, 400.0));
        scroll.scroll_by(40.0, 50.0);
        scroll.overscroll.y = -10.0;
        let snapshot = scroll.geometry(
            Rect::new(20.0, 30.0, 100.0, 80.0),
            Some(Rect::new(30.0, 40.0, 70.0, 50.0)),
        );
        assert_eq!(snapshot.content_origin(), Point::new(-20.0, -10.0));
        assert_eq!(
            snapshot.visible_viewport_rect(),
            Some(Rect::new(30.0, 40.0, 70.0, 50.0))
        );
        assert_eq!(
            snapshot.viewport_to_document(Point::new(40.0, 50.0)),
            Some(Point::new(60.0, 60.0))
        );
        assert_eq!(
            snapshot.document_to_viewport(Rect::new(60.0, 60.0, 5.0, 5.0)),
            Some(Rect::new(40.0, 50.0, 5.0, 5.0))
        );
    }
}
