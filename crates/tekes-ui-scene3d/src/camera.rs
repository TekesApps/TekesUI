use crate::{Error, err};
use glam::{EulerRot, Mat4, Quat, Vec3};

/// Platform-independent orbit camera and model orientation. Angles are radians.
/// Hosts map mouse/keyboard/scroll events into this state; no timers or platform
/// event dependencies live in the component.
#[derive(Clone, Debug, PartialEq)]
pub struct Camera3D {
    yaw: f32,
    pitch: f32,
    zoom: f32,
    rotation: Quat,
    orthographic: bool,
    pan: [f32; 2],
}
impl Default for Camera3D {
    fn default() -> Self {
        Self {
            yaw: 0.,
            pitch: 0.35_f32.atan(),
            zoom: 1.,
            rotation: Quat::IDENTITY,
            orthographic: false,
            pan: [0.; 2],
        }
    }
}
impl Camera3D {
    /// Parallel projection for architectural/isometric scenes. Zoom still scales
    /// the visible area; perspective remains the default for existing consumers.
    pub fn set_orthographic(&mut self, enabled: bool) {
        self.orthographic = enabled;
    }
    pub fn yaw(&self) -> f32 {
        self.yaw
    }
    pub fn pitch(&self) -> f32 {
        self.pitch
    }
    /// Camera distance multiplier; smaller values enlarge the object.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    pub fn model_rotation(&self) -> [f32; 4] {
        self.rotation.to_array()
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) -> Result<(), Error> {
        if !yaw_delta.is_finite() || !pitch_delta.is_finite() {
            return Err(err("orbit angles must be finite"));
        }
        self.yaw = ((self.yaw as f64 + yaw_delta as f64).rem_euclid(std::f64::consts::TAU)) as f32;
        self.pitch = (self.pitch as f64 + pitch_delta as f64).clamp(-1.55, 1.55) as f32;
        Ok(())
    }
    /// Positive multiplicative distance change, bounded to avoid singular views.
    pub fn zoom_by(&mut self, factor: f32) -> Result<(), Error> {
        if !factor.is_finite() || factor <= 0. {
            return Err(err("zoom factor must be finite and positive"));
        }
        self.zoom = (self.zoom as f64 * factor as f64).clamp(0.04, 8.) as f32;
        Ok(())
    }
    /// Move the displayed scene by fractions of the viewport (right/up positive).
    /// This screen-space camera offset preserves orbit, model orientation and zoom.
    pub fn pan_by(&mut self, horizontal: f32, vertical: f32) -> Result<(), Error> {
        if !horizontal.is_finite() || !vertical.is_finite() {
            return Err(err("pan offsets must be finite"));
        }
        self.pan[0] = (self.pan[0] as f64 + horizontal as f64).clamp(-16., 16.) as f32;
        self.pan[1] = (self.pan[1] as f64 + vertical as f64).clamp(-16., 16.) as f32;
        Ok(())
    }
    /// Rotate the entire model around its fitted center, independently of orbit.
    pub fn rotate_model(&mut self, yaw: f32, pitch: f32, roll: f32) -> Result<(), Error> {
        if [yaw, pitch, roll].iter().any(|v| !v.is_finite()) {
            return Err(err("model angles must be finite"));
        }
        let turn = |v: f32| v.rem_euclid(std::f32::consts::TAU);
        self.rotation = (Quat::from_euler(EulerRot::YXZ, turn(yaw), turn(pitch), turn(roll))
            * self.rotation)
            .normalize();
        Ok(())
    }
    pub(crate) fn legacy(yaw: f32, zoom: f32) -> Result<Self, Error> {
        if !yaw.is_finite() || !zoom.is_finite() || zoom <= 0. {
            return Err(err("invalid orbit camera"));
        }
        Ok(Self {
            yaw,
            zoom,
            ..Self::default()
        })
    }
    pub(crate) fn matrices(
        &self,
        center: Vec3,
        radius: f32,
        aspect: f32,
    ) -> Result<[[[f32; 4]; 4]; 2], Error> {
        let distance =
            radius * 3.2 * if self.orthographic { 1. } else { self.zoom } / aspect.min(1.);
        let eye = center
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            ) * distance;
        // Reversed-Z preserves floating-point depth precision when distant
        // surfaces are close together (e.g. stickers above a cube face).
        let projection = if self.orthographic {
            let half_height = radius * 1.3 * self.zoom / aspect.min(1.);
            Mat4::orthographic_rh(
                -half_height * aspect,
                half_height * aspect,
                -half_height,
                half_height,
                radius * 16.,
                (radius * 0.001).max(0.00001),
            )
        } else {
            Mat4::perspective_infinite_reverse_rh(
                45_f32.to_radians(),
                aspect,
                (radius * 0.001).max(0.00001),
            )
        };
        let screen_offset =
            Mat4::from_translation(Vec3::new(self.pan[0] * 2., self.pan[1] * 2., 0.));
        let view_projection = screen_offset * projection * Mat4::look_at_rh(eye, center, Vec3::Y);
        let model = Mat4::from_translation(center)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_translation(-center);
        if !view_projection.is_finite() || !model.is_finite() {
            return Err(err("camera exceeds finite coordinate range"));
        }
        Ok([view_projection.to_cols_array_2d(), model.to_cols_array_2d()])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_are_bounded_finite_and_resettable() {
        let mut c = Camera3D::default();
        let original = c.clone();
        c.orbit(1e20, 1e20).unwrap();
        c.zoom_by(1e20).unwrap();
        assert!(c.pitch() < std::f32::consts::FRAC_PI_2);
        assert_eq!(c.zoom(), 8.);
        c.zoom_by(1e-20).unwrap();
        assert_eq!(c.zoom(), 0.04);
        c.rotate_model(0.5, 0.2, 0.1).unwrap();
        assert!((Quat::from_array(c.model_rotation()).length() - 1.).abs() < 1e-6);
        let before = c.clone();
        assert!(c.orbit(f32::NAN, 0.).is_err());
        assert_eq!(before, c);
        assert!(c.zoom_by(0.).is_err());
        assert!(c.rotate_model(0., f32::INFINITY, 0.).is_err());
        c.reset();
        assert_eq!(c, original);
    }
    #[test]
    fn orthographic_zoom_preserves_depth_and_parallel_lines() {
        let mut c = Camera3D::default();
        c.set_orthographic(true);
        let m = Mat4::from_cols_array_2d(&c.matrices(Vec3::ZERO, 2., 1.5).unwrap()[0]);
        let a = m * Vec3::new(0., 0., 1.).extend(1.);
        let b = m * Vec3::new(0., 0., -1.).extend(1.);
        assert_eq!(a.w, b.w);
        c.zoom_by(0.2).unwrap();
        let zoomed = Mat4::from_cols_array_2d(&c.matrices(Vec3::ZERO, 2., 1.5).unwrap()[0]);
        assert!((m.z_axis.z - zoomed.z_axis.z).abs() < 1e-6);
        assert!((m.w_axis.z - zoomed.w_axis.z).abs() < 1e-6);
        c.reset();
        assert!(!c.orthographic);
    }
    #[test]
    fn pan_tracks_viewport_fraction_without_changing_depth_or_orientation() {
        for orthographic in [false, true] {
            for aspect in [0.7, 1.8] {
                let mut c = Camera3D::default();
                c.set_orthographic(orthographic);
                c.orbit(0.7, 0.2).unwrap();
                c.zoom_by(0.35).unwrap();
                c.rotate_model(0.2, 0.1, 0.).unwrap();
                let original = c.clone();
                let before = c.matrices(Vec3::ZERO, 2., aspect).unwrap();
                c.pan_by(0.25, -0.1).unwrap();
                let after = c.matrices(Vec3::ZERO, 2., aspect).unwrap();
                for point in [Vec3::ZERO, Vec3::new(0.2, -0.3, 0.1)] {
                    let a = Mat4::from_cols_array_2d(&before[0]).project_point3(point);
                    let b = Mat4::from_cols_array_2d(&after[0]).project_point3(point);
                    assert!((b - a).abs_diff_eq(Vec3::new(0.5, -0.2, 0.), 1e-5));
                }
                assert_eq!(before[1], after[1]);
                assert_eq!(c.zoom(), original.zoom());
                assert_eq!(c.yaw(), original.yaw());
                let valid = c.clone();
                assert!(c.pan_by(f32::NAN, 0.).is_err());
                assert_eq!(c, valid);
                c.reset();
                assert_eq!(c, Camera3D::default());
            }
        }
    }
    #[test]
    fn model_rotation_preserves_the_fitted_center() {
        let mut c = Camera3D::default();
        c.rotate_model(0.8, -0.4, 0.3).unwrap();
        let center = Vec3::new(8., -3., 4.);
        let matrices = c.matrices(center, 2., 1.5).unwrap();
        let model = Mat4::from_cols_array_2d(&matrices[1]);
        assert!(model.transform_point3(center).abs_diff_eq(center, 1e-5));
    }
}
