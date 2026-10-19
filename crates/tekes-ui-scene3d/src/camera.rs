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
}
impl Default for Camera3D {
    fn default() -> Self {
        Self {
            yaw: 0.,
            pitch: 0.35_f32.atan(),
            zoom: 1.,
            rotation: Quat::IDENTITY,
        }
    }
}
impl Camera3D {
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
        self.zoom = (self.zoom as f64 * factor as f64).clamp(0.2, 8.) as f32;
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
        let distance = radius * 3.2 * self.zoom / aspect.min(1.);
        let eye = center
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            ) * distance;
        // Reversed-Z preserves floating-point depth precision when distant
        // surfaces are close together (e.g. stickers above a cube face).
        let view_projection = Mat4::perspective_infinite_reverse_rh(
            45_f32.to_radians(),
            aspect,
            (radius * 0.001).max(0.00001),
        ) * Mat4::look_at_rh(eye, center, Vec3::Y);
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
        assert_eq!(c.zoom(), 0.2);
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
    fn model_rotation_preserves_the_fitted_center() {
        let mut c = Camera3D::default();
        c.rotate_model(0.8, -0.4, 0.3).unwrap();
        let center = Vec3::new(8., -3., 4.);
        let matrices = c.matrices(center, 2., 1.5).unwrap();
        let model = Mat4::from_cols_array_2d(&matrices[1]);
        assert!(model.transform_point3(center).abs_diff_eq(center, 1e-5));
    }
}
