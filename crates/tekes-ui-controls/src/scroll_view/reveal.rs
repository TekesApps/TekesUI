//! Explicit navigation shares the original viewport, input ownership and motion.
use super::*;
use crate::{Insets, control_motion::ProgressTransition};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
static NEXT_REVEAL: AtomicU64 = AtomicU64::new(1);
use tekes_ui_motion::{Motion, MotionSpec, TimingCurve};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollRevealPlacement {
    #[default]
    Nearest,
    Centered,
    /// Align only the selected axis to the comfort-zone start, even when
    /// already visible. The other document offset is retained.
    Start(crate::extended::Axis),
}
/// Document-local target placement. Margins define the viewport's comfort zone;
/// they never change content layout or the legacy focus-ring reveal contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollRevealOptions {
    pub placement: ScrollRevealPlacement,
    pub margins: Insets,
    /// Uses the existing JSON geometry duration and standard reveal easing.
    /// Reduced Motion always completes immediately. The host supplies tick().
    pub animated: bool,
}
impl Default for ScrollRevealOptions {
    fn default() -> Self {
        Self {
            placement: ScrollRevealPlacement::Nearest,
            margins: Insets::ZERO,
            animated: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollRevealError {
    InvalidRect,
    InvalidMargins,
    UnlaidOut,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollRevealResult {
    pub changed: bool,
    pub clamped: bool,
    pub animated: bool,
}
#[derive(Clone, Debug)]
pub(super) struct RevealTransition {
    from: Point,
    to: Point,
    elapsed: Duration,
    progress: ProgressTransition,
}
impl RevealTransition {
    fn new(from: Point, to: Point) -> Self {
        let [x1, y1, x2, y2] = proposal::control_easing("disclosure_reveal")
            .expect("existing standard reveal easing binding");
        let motion = Motion::new(MotionSpec::Tween {
            duration: Duration::from_secs_f64(f64::from(
                proposal::duration("geometry").expect("existing geometry duration"),
            )),
            curve: TimingCurve::cubic_bezier(
                f64::from(x1),
                f64::from(y1),
                f64::from(x2),
                f64::from(y2),
            )
            .expect("validated standard easing"),
        });
        let mut progress = ProgressTransition::default();
        progress
            .retarget(1., motion, Duration::ZERO, false)
            .expect("new monotonic transition");
        Self {
            from,
            to,
            elapsed: Duration::ZERO,
            progress,
        }
    }
    pub(super) fn sample(&mut self, dt: f32) -> (Point, bool) {
        let elapsed = Duration::try_from_secs_f64(f64::from(dt))
            .ok()
            .and_then(|delta| self.elapsed.checked_add(delta));
        let Some(elapsed) = elapsed else {
            return (self.to, true);
        };
        self.elapsed = elapsed;
        let fraction = self
            .progress
            .advance(elapsed)
            .expect("monotonic host elapsed time");
        let mix = |from: f32, to: f32| {
            (f64::from(from) + (f64::from(to) - f64::from(from)) * fraction) as f32
        };
        (
            Point::new(mix(self.from.x, self.to.x), mix(self.from.y, self.to.y)),
            !self.progress.needs_animation(),
        )
    }
}
impl ScrollView {
    /// Explicit navigation, without focusing or editing any content. Old
    /// momentum is fenced until a fresh non-momentum gesture. Fresh wheel input
    /// wins over animation; another reveal retargets from the current position.
    /// Invalid requests have no side effects. Layout changes and cancel stop it.
    pub fn reveal_with(
        &mut self,
        rect: Rect,
        options: ScrollRevealOptions,
    ) -> Result<ScrollRevealResult, ScrollRevealError> {
        let (target, clamped) = self.reveal_target(rect, options)?;
        self.reveal_generation = NEXT_REVEAL
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("scroll navigation identity exhausted");
        let changed = target != self.offset || self.overscroll != Point::default();
        let from = self.offset;
        self.cancel();
        self.ignore_momentum = true;
        let animated = options.animated && !self.reduced_motion && target != from;
        if animated {
            self.reveal_animation = Some(RevealTransition::new(from, target));
        } else {
            self.scroll_by(target.x - self.offset.x, target.y - self.offset.y);
        }
        Ok(ScrollRevealResult {
            changed,
            clamped,
            animated,
        })
    }
    pub(crate) fn reveal_target(
        &self,
        rect: Rect,
        options: ScrollRevealOptions,
    ) -> Result<(Point, bool), ScrollRevealError> {
        let valid = [
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ]
        .into_iter()
        .all(f32::is_finite);
        if !valid || rect.size.width < 0. || rect.size.height < 0. {
            return Err(ScrollRevealError::InvalidRect);
        }
        if self.viewport.width <= 0. || self.viewport.height <= 0. {
            return Err(ScrollRevealError::UnlaidOut);
        }
        let m = options.margins;
        if [m.top, m.right, m.bottom, m.left]
            .into_iter()
            .any(|v| !v.is_finite() || v < 0.)
            || f64::from(m.left) + f64::from(m.right) >= f64::from(self.viewport.width)
            || f64::from(m.top) + f64::from(m.bottom) >= f64::from(self.viewport.height)
        {
            return Err(ScrollRevealError::InvalidMargins);
        }
        let axis = |start: f32,
                    length: f32,
                    offset: f32,
                    view: f32,
                    before: f32,
                    after: f32,
                    limit: f32| {
            let (start, end, offset, view, before, after, limit) = (
                f64::from(start),
                f64::from(start) + f64::from(length),
                f64::from(offset),
                f64::from(view),
                f64::from(before),
                f64::from(after),
                f64::from(limit),
            );
            let near = offset + before;
            let far = offset + view - after;
            let wanted = match options.placement {
                ScrollRevealPlacement::Start(_) => start - before,
                ScrollRevealPlacement::Centered => (start + end - view - before + after) * 0.5,
                ScrollRevealPlacement::Nearest
                    if (start >= near && end <= far) || (start <= near && end >= far) =>
                {
                    offset
                }
                ScrollRevealPlacement::Nearest => {
                    let leading = start - before;
                    let trailing = end - view + after;
                    if (leading - offset).abs() < (trailing - offset).abs() {
                        leading
                    } else {
                        trailing
                    }
                }
            };
            let applied = wanted.clamp(0., limit);
            (applied as f32, (wanted - applied).abs() > 0.001)
        };
        let limit = self.limit();
        let (mut x, mut cx) = axis(
            rect.origin.x,
            rect.size.width,
            self.offset.x,
            self.viewport.width,
            m.left,
            m.right,
            limit.x,
        );
        let (mut y, mut cy) = axis(
            rect.origin.y,
            rect.size.height,
            self.offset.y,
            self.viewport.height,
            m.top,
            m.bottom,
            limit.y,
        );
        match options.placement {
            ScrollRevealPlacement::Start(crate::extended::Axis::Vertical) => {
                x = self.offset.x;
                cx = false;
            }
            ScrollRevealPlacement::Start(crate::extended::Axis::Horizontal) => {
                y = self.offset.y;
                cy = false;
            }
            _ => {}
        }
        Ok((Point::new(x, y), cx || cy))
    }
    pub(crate) fn reveal_identity(&self) -> u64 {
        self.reveal_generation
    }
    pub(crate) fn reveal_in_flight(&self) -> bool {
        self.reveal_animation.is_some()
    }
    pub(crate) fn cancel_reveal(&mut self, generation: u64) {
        if self.reveal_generation == generation {
            self.reveal_animation = None;
        }
    }
    pub(super) fn finish_reveal(&mut self) -> bool {
        let Some(animation) = self.reveal_animation.take() else {
            return false;
        };
        self.scroll_by(
            animation.to.x - self.offset.x,
            animation.to.y - self.offset.y,
        )
    }
}
