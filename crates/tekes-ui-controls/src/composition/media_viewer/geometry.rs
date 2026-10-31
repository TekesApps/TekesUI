use super::{MediaViewerError, MediaViewerMode, MediaViewerTransform};
use tekes_ui_scene::{Point, Rect, Size};

const MIN_RELATIVE_SCALE: f64 = 1.0;
const MAX_RELATIVE_SCALE: f64 = 8.0;

/// Viewport-local image geometry. Calculations stay in f64 until producing the
/// scene's f32 points and rectangles.
#[derive(Debug)]
pub(super) struct Geometry {
    source: Size,
    viewport: Option<Size>,
    active: bool,
    relative_scale: f64,
    translation: (f64, f64),
    mode: MediaViewerMode,
}

impl Geometry {
    pub(super) fn new(source: Size) -> Result<Self, MediaViewerError> {
        if !valid_size(source) {
            return Err(MediaViewerError::InvalidImageSize);
        }
        Ok(Self {
            source,
            viewport: None,
            active: false,
            relative_scale: MIN_RELATIVE_SCALE,
            translation: (0.0, 0.0),
            mode: MediaViewerMode::Fit,
        })
    }

    /// Accept a newly ready source and reset its view, including when the new
    /// resource has the same intrinsic dimensions.
    pub(super) fn set_source(&mut self, source: Size) -> Result<bool, MediaViewerError> {
        if !valid_size(source) {
            return Err(MediaViewerError::InvalidImageSize);
        }
        let old = self.transform();
        let old_source = self.source;
        let old_q = self.relative_scale;
        let old_mode = self.mode;
        self.source = source;
        self.relative_scale = MIN_RELATIVE_SCALE;
        self.mode = MediaViewerMode::Fit;
        self.translation = self
            .viewport
            .map(|viewport| fit_translation(source, viewport))
            .unwrap_or((0.0, 0.0));
        Ok(old_source != source
            || old_q != self.relative_scale
            || old_mode != self.mode
            || old != self.transform())
    }

    /// Invalid or empty allocations suspend drawing and input while retaining
    /// the last valid geometry for a later resume.
    pub(super) fn set_viewport(&mut self, viewport: Size) -> bool {
        if !valid_size(viewport) {
            let changed = self.active;
            self.active = false;
            return changed;
        }

        let was_active = self.active;
        if self.viewport == Some(viewport) {
            self.active = true;
            return !was_active;
        }

        if let Some(old_viewport) = self.viewport {
            if self.mode == MediaViewerMode::Fit {
                self.translation = fit_translation(self.source, viewport);
            } else {
                let old_scale = self.scale_for(old_viewport);
                let new_scale =
                    scale_from_fit(fit_scale(self.source, viewport), self.relative_scale);
                let old_center_point = (
                    (f64::from(old_viewport.width) * 0.5 - self.translation.0) / old_scale,
                    (f64::from(old_viewport.height) * 0.5 - self.translation.1) / old_scale,
                );
                self.translation = clamp_translation(
                    (
                        f64::from(viewport.width) * 0.5 - new_scale * old_center_point.0,
                        f64::from(viewport.height) * 0.5 - new_scale * old_center_point.1,
                    ),
                    self.source,
                    viewport,
                    new_scale,
                );
            }
        } else {
            self.relative_scale = MIN_RELATIVE_SCALE;
            self.mode = MediaViewerMode::Fit;
            self.translation = fit_translation(self.source, viewport);
        }
        self.viewport = Some(viewport);
        self.active = true;
        true
    }

    pub(super) fn transform(&self) -> Option<MediaViewerTransform> {
        if !self.active {
            return None;
        }
        let viewport = self.viewport?;
        let scale = self.scale_for(viewport);
        if !valid_extent(self.source, scale) {
            return None;
        }
        Some(MediaViewerTransform {
            relative_scale: self.relative_scale,
            scale,
            translation: Point::new(as_f32(self.translation.0)?, as_f32(self.translation.1)?),
            mode: self.mode,
        })
    }

    /// `relative_scale` is the requested Fit-relative q; finite requests are
    /// clamped before solving the anchor and applying image-bound constraints.
    pub(super) fn zoom(
        &mut self,
        relative_scale: f64,
        anchor: Point,
    ) -> Result<bool, MediaViewerError> {
        let viewport = self
            .viewport
            .filter(|_| self.active)
            .ok_or(MediaViewerError::InvalidTransform)?;
        if !relative_scale.is_finite() || !point_in_viewport(anchor, viewport) {
            return Err(MediaViewerError::InvalidTransform);
        }
        if !valid_extent(self.source, self.scale_for(viewport)) {
            return Err(MediaViewerError::InvalidTransform);
        }
        let next_q = relative_scale.clamp(MIN_RELATIVE_SCALE, MAX_RELATIVE_SCALE);
        let old_scale = self.scale_for(viewport);
        let next_scale = scale_from_fit(fit_scale(self.source, viewport), next_q);
        if !valid_extent(self.source, next_scale) {
            return Err(MediaViewerError::InvalidTransform);
        }
        let ax = f64::from(anchor.x);
        let ay = f64::from(anchor.y);
        let next_translation = clamp_translation(
            (
                ax - (ax - self.translation.0) * next_scale / old_scale,
                ay - (ay - self.translation.1) * next_scale / old_scale,
            ),
            self.source,
            viewport,
            next_scale,
        );
        ensure_translation_representable(next_translation)?;
        let changed = next_q != self.relative_scale || next_translation != self.translation;
        if changed {
            self.relative_scale = next_q;
            self.translation = next_translation;
            self.mode = MediaViewerMode::Manual;
        }
        Ok(changed)
    }

    pub(super) fn pan(&mut self, delta: Point) -> Result<bool, MediaViewerError> {
        let viewport = self
            .viewport
            .filter(|_| self.active)
            .ok_or(MediaViewerError::InvalidTransform)?;
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return Err(MediaViewerError::InvalidTransform);
        }
        let scale = self.scale_for(viewport);
        if !valid_extent(self.source, scale) {
            return Err(MediaViewerError::InvalidTransform);
        }
        let next = clamp_translation(
            (
                self.translation.0 + f64::from(delta.x),
                self.translation.1 + f64::from(delta.y),
            ),
            self.source,
            viewport,
            scale,
        );
        ensure_translation_representable(next)?;
        let changed = next != self.translation;
        if changed {
            self.translation = next;
            self.mode = MediaViewerMode::Manual;
        }
        Ok(changed)
    }

    pub(super) fn fit(&mut self) -> bool {
        let old = self.transform();
        self.relative_scale = MIN_RELATIVE_SCALE;
        self.mode = MediaViewerMode::Fit;
        self.translation = self
            .viewport
            .map(|viewport| fit_translation(self.source, viewport))
            .unwrap_or((0.0, 0.0));
        old != self.transform()
    }

    pub(super) fn actual(&mut self) -> bool {
        if !self.can_actual() {
            return false;
        }
        let viewport = self.viewport.expect("can_actual requires a viewport");
        let old = self.transform();
        let next_q = 1.0 / fit_scale(self.source, viewport);
        let old_scale = self.scale_for(viewport);
        let center = (
            f64::from(viewport.width) * 0.5,
            f64::from(viewport.height) * 0.5,
        );
        self.translation = clamp_translation(
            (
                center.0 - (center.0 - self.translation.0) / old_scale,
                center.1 - (center.1 - self.translation.1) / old_scale,
            ),
            self.source,
            viewport,
            1.0,
        );
        self.relative_scale = next_q;
        self.mode = MediaViewerMode::Manual;
        old != self.transform()
    }

    pub(super) fn can_actual(&self) -> bool {
        let Some(viewport) = self.viewport.filter(|_| self.active) else {
            return false;
        };
        let q = 1.0 / fit_scale(self.source, viewport);
        (MIN_RELATIVE_SCALE..=MAX_RELATIVE_SCALE).contains(&q)
    }

    pub(super) fn can_pan(&self) -> bool {
        let Some(viewport) = self.viewport.filter(|_| self.active) else {
            return false;
        };
        let scale = self.scale_for(viewport);
        if !valid_extent(self.source, scale) {
            return false;
        }
        f64::from(self.source.width) * scale > f64::from(viewport.width)
            || f64::from(self.source.height) * scale > f64::from(viewport.height)
    }

    pub(super) fn image_rect(&self, origin: Point) -> Option<Rect> {
        let viewport = self.viewport.filter(|_| self.active)?;
        if !origin.x.is_finite() || !origin.y.is_finite() {
            return None;
        }
        let scale = self.scale_for(viewport);
        let x = f64::from(origin.x) + self.translation.0;
        let y = f64::from(origin.y) + self.translation.1;
        let width = f64::from(self.source.width) * scale;
        let height = f64::from(self.source.height) * scale;
        if !valid_extent(self.source, scale)
            || as_f32(x + width).is_none()
            || as_f32(y + height).is_none()
        {
            return None;
        }
        Some(Rect::new(
            as_f32(x)?,
            as_f32(y)?,
            as_positive_f32(width)?,
            as_positive_f32(height)?,
        ))
    }

    fn scale_for(&self, viewport: Size) -> f64 {
        scale_from_fit(fit_scale(self.source, viewport), self.relative_scale)
    }
}

fn valid_size(size: Size) -> bool {
    size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0
}

fn fit_scale(source: Size, viewport: Size) -> f64 {
    (f64::from(viewport.width) / f64::from(source.width))
        .min(f64::from(viewport.height) / f64::from(source.height))
}

fn scale_from_fit(fit: f64, relative_scale: f64) -> f64 {
    if relative_scale == 1.0 / fit {
        1.0
    } else {
        fit * relative_scale
    }
}

fn fit_translation(source: Size, viewport: Size) -> (f64, f64) {
    let scale = fit_scale(source, viewport);
    (
        (f64::from(viewport.width) - f64::from(source.width) * scale) * 0.5,
        (f64::from(viewport.height) - f64::from(source.height) * scale) * 0.5,
    )
}

fn clamp_translation(
    translation: (f64, f64),
    source: Size,
    viewport: Size,
    scale: f64,
) -> (f64, f64) {
    (
        clamp_axis(
            translation.0,
            f64::from(source.width) * scale,
            f64::from(viewport.width),
        ),
        clamp_axis(
            translation.1,
            f64::from(source.height) * scale,
            f64::from(viewport.height),
        ),
    )
}

fn clamp_axis(translation: f64, image_length: f64, viewport_length: f64) -> f64 {
    if image_length <= viewport_length {
        (viewport_length - image_length) * 0.5
    } else {
        translation.clamp(viewport_length - image_length, 0.0)
    }
}

fn point_in_viewport(point: Point, viewport: Size) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x >= 0.0
        && point.y >= 0.0
        && point.x < viewport.width
        && point.y < viewport.height
}

fn as_f32(value: f64) -> Option<f32> {
    if value.is_finite() && value >= f64::from(f32::MIN) && value <= f64::from(f32::MAX) {
        Some(value as f32)
    } else {
        None
    }
}

fn as_positive_f32(value: f64) -> Option<f32> {
    let value = as_f32(value)?;
    (value.is_finite() && value > 0.0).then_some(value)
}

fn valid_extent(source: Size, scale: f64) -> bool {
    scale.is_finite()
        && as_positive_f32(f64::from(source.width) * scale).is_some()
        && as_positive_f32(f64::from(source.height) * scale).is_some()
}

fn ensure_translation_representable(translation: (f64, f64)) -> Result<(), MediaViewerError> {
    if as_f32(translation.0).is_some() && as_f32(translation.1).is_some() {
        Ok(())
    } else {
        Err(MediaViewerError::InvalidTransform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: f32, height: f32) -> Size {
        Size::new(width, height)
    }

    fn geometry(source: Size, viewport: Size) -> Geometry {
        let mut geometry = Geometry::new(source).unwrap();
        geometry.set_viewport(viewport);
        geometry
    }

    fn transform(geometry: &Geometry) -> MediaViewerTransform {
        geometry.transform().unwrap()
    }

    #[test]
    fn fit_centers_short_axis_and_clamps_zoomed_axes() {
        let mut wide = geometry(size(200.0, 100.0), size(100.0, 100.0));
        assert_eq!(transform(&wide).translation, Point::new(0.0, 25.0));
        wide.zoom(8.0, Point::new(50.0, 50.0)).unwrap();
        assert_eq!(transform(&wide).translation, Point::new(-350.0, -150.0));

        let mut small = geometry(size(20.0, 10.0), size(100.0, 80.0));
        assert_eq!(transform(&small).translation, Point::new(0.0, 15.0));
        small.zoom(1.0, Point::new(50.0, 40.0)).unwrap();
        assert_eq!(transform(&small).translation, Point::new(0.0, 15.0));
    }

    #[test]
    fn zoom_solves_anchor_before_clamping() {
        let mut geometry = geometry(size(200.0, 100.0), size(100.0, 100.0));
        let anchor = Point::new(20.0, 50.0);
        let old = transform(&geometry);
        geometry.zoom(2.0, anchor).unwrap();
        let new = transform(&geometry);
        assert_eq!(new.translation, Point::new(-20.0, 0.0));
        let before_x = (f64::from(anchor.x) - f64::from(old.translation.x)) / old.scale;
        let after_x = (f64::from(anchor.x) - f64::from(new.translation.x)) / new.scale;
        let before_y = (f64::from(anchor.y) - f64::from(old.translation.y)) / old.scale;
        let after_y = (f64::from(anchor.y) - f64::from(new.translation.y)) / new.scale;
        assert_eq!(before_x, after_x);
        assert_eq!(before_y, after_y);
    }

    #[test]
    fn fractional_zoom_anchor_drift_is_bounded_in_viewport_points() {
        let mut geometry = geometry(size(317.0, 211.0), size(241.0, 163.0));
        let anchor = Point::new(103.25, 76.75);
        geometry.zoom(2.3, anchor).unwrap();
        let before = transform(&geometry);
        let content = (
            (f64::from(anchor.x) - f64::from(before.translation.x)) / before.scale,
            (f64::from(anchor.y) - f64::from(before.translation.y)) / before.scale,
        );
        geometry.zoom(3.7, anchor).unwrap();
        let after = transform(&geometry);
        let projected = (
            f64::from(after.translation.x) + after.scale * content.0,
            f64::from(after.translation.y) + after.scale * content.1,
        );
        assert!((projected.0 - f64::from(anchor.x)).abs() <= 2.0);
        assert!((projected.1 - f64::from(anchor.y)).abs() <= 2.0);
        // Both axes are strictly inside their bounds: this is an unclamped anchor.
        let image = geometry.image_rect(Point::new(0.0, 0.0)).unwrap();
        assert!(image.min_x() < 0.0 && image.max_x() > 241.0);
        assert!(image.min_y() < 0.0 && image.max_y() > 163.0);
    }

    #[test]
    fn zoom_and_pan_clamp_at_both_limits() {
        let mut geometry = geometry(size(100.0, 100.0), size(100.0, 100.0));
        geometry.zoom(100.0, Point::new(50.0, 50.0)).unwrap();
        assert_eq!(transform(&geometry).relative_scale, 8.0);
        assert_eq!(transform(&geometry).translation, Point::new(-350.0, -350.0));
        assert!(geometry.pan(Point::new(-1000.0, -1000.0)).unwrap());
        assert_eq!(transform(&geometry).translation, Point::new(-700.0, -700.0));
        assert!(!geometry.pan(Point::new(-10.0, -10.0)).unwrap());
        assert!(geometry.pan(Point::new(1000.0, 1000.0)).unwrap());
        assert_eq!(transform(&geometry).translation, Point::new(0.0, 0.0));
        assert!(geometry.can_pan());
    }

    #[test]
    fn resize_preserves_manual_q_and_center_content_point() {
        let mut geometry = geometry(size(200.0, 100.0), size(100.0, 100.0));
        geometry.zoom(2.0, Point::new(40.0, 50.0)).unwrap();
        let old = transform(&geometry);
        let old_center_content = (
            (50.0 - f64::from(old.translation.x)) / old.scale,
            (50.0 - f64::from(old.translation.y)) / old.scale,
        );
        assert_eq!(old.translation, Point::new(-40.0, 0.0));
        assert_eq!(old_center_content, (90.0, 50.0));
        geometry.set_viewport(size(200.0, 100.0));
        let resized = transform(&geometry);
        assert_eq!(resized.relative_scale, 2.0);
        assert_eq!(resized.translation, Point::new(-80.0, -50.0));
        let new_center_content = (
            (100.0 - f64::from(resized.translation.x)) / resized.scale,
            (50.0 - f64::from(resized.translation.y)) / resized.scale,
        );
        assert_eq!(old_center_content, new_center_content);

        geometry.set_viewport(size(20.0, 10.0));
        assert_eq!(transform(&geometry).relative_scale, 2.0);
        assert_eq!(transform(&geometry).translation, Point::new(-8.0, -5.0));
    }

    #[test]
    fn resize_clamps_center_anchor_only_when_new_bounds_require_it() {
        let mut geometry = geometry(size(200.0, 100.0), size(100.0, 100.0));
        geometry.zoom(8.0, Point::new(0.0, 0.0)).unwrap();
        assert_eq!(transform(&geometry).translation, Point::new(0.0, 0.0));
        geometry.set_viewport(size(100.0, 10.0));
        assert_eq!(transform(&geometry).relative_scale, 8.0);
        // The computed x anchor would be +40pt; the oversized image must stay
        // within [viewport_width - image_width, 0].
        assert_eq!(transform(&geometry).translation, Point::new(0.0, -5.0));
    }

    #[test]
    fn empty_viewport_suspends_and_resumes_from_last_valid_geometry() {
        let mut geometry = geometry(size(100.0, 100.0), size(80.0, 60.0));
        geometry.zoom(3.0, Point::new(40.0, 30.0)).unwrap();
        let old = transform(&geometry);
        assert!(geometry.set_viewport(size(0.0, 60.0)));
        assert!(geometry.transform().is_none());
        assert!(!geometry.can_pan());
        assert!(geometry.set_viewport(size(80.0, 60.0)));
        assert_eq!(transform(&geometry), old);
    }

    #[test]
    fn invalid_inputs_leave_the_accepted_state_unchanged() {
        assert!(matches!(
            Geometry::new(size(0.0, 4.0)),
            Err(MediaViewerError::InvalidImageSize)
        ));
        let mut geometry = geometry(size(100.0, 100.0), size(80.0, 60.0));
        let old = transform(&geometry);
        assert!(geometry.zoom(f64::NAN, Point::new(10.0, 10.0)).is_err());
        assert!(geometry.zoom(2.0, Point::new(80.0, 10.0)).is_err());
        assert!(geometry.pan(Point::new(f32::INFINITY, 0.0)).is_err());
        assert_eq!(transform(&geometry), old);
        assert!(geometry.set_source(size(f32::NAN, 3.0)).is_err());
        assert_eq!(transform(&geometry), old);
    }

    #[test]
    fn actual_logical_size_obeys_relative_scale_range() {
        let mut allowed = geometry(size(100.0, 100.0), size(100.0, 100.0));
        assert!(allowed.can_actual());
        assert!(allowed.actual());
        assert_eq!(transform(&allowed).scale, 1.0);
        assert_eq!(transform(&allowed).mode, MediaViewerMode::Manual);

        let disallowed = geometry(size(10.0, 10.0), size(100.0, 100.0));
        assert!(!disallowed.can_actual());

        let mut anchored = geometry(size(200.0, 100.0), size(100.0, 100.0));
        anchored.zoom(4.0, Point::new(75.0, 50.0)).unwrap();
        assert!(anchored.actual());
        assert_eq!(transform(&anchored).scale, 1.0);
        assert_eq!(transform(&anchored).translation, Point::new(-87.5, 0.0));
    }

    #[test]
    fn accepting_same_sized_source_resets_manual_view_to_fit() {
        let mut geometry = geometry(size(100.0, 50.0), size(100.0, 100.0));
        geometry.zoom(2.0, Point::new(50.0, 50.0)).unwrap();
        assert_eq!(transform(&geometry).mode, MediaViewerMode::Manual);
        assert!(geometry.set_source(size(100.0, 50.0)).unwrap());
        assert_eq!(transform(&geometry).mode, MediaViewerMode::Fit);
        assert_eq!(transform(&geometry).relative_scale, 1.0);
    }

    #[test]
    fn image_rect_applies_origin_once_and_rejects_unrepresentable_output() {
        let normal = geometry(size(100.0, 50.0), size(100.0, 100.0));
        assert_eq!(
            normal.image_rect(Point::new(10.0, 20.0)),
            Some(Rect::new(10.0, 45.0, 100.0, 50.0))
        );
        let huge_view = geometry(size(100.0, 100.0), size(f32::MAX / 2.0, f32::MAX / 2.0));
        assert_eq!(huge_view.image_rect(Point::new(f32::MAX, 0.0)), None);

        let mut underflow = Geometry::new(size(f32::MAX, f32::MIN_POSITIVE)).unwrap();
        underflow.set_viewport(size(f32::MIN_POSITIVE, f32::MAX));
        assert!(underflow.transform().is_none());
        assert!(!underflow.can_pan());
        assert_eq!(underflow.image_rect(Point::new(0.0, 0.0)), None);
    }
}
