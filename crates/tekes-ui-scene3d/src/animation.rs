use crate::{Error, err};
use glam::{Quat, Vec4};
use gltf::animation::Interpolation;

#[derive(Debug)]
pub(crate) struct Channel {
    pub node: usize,
    pub property: gltf::animation::Property,
    pub interpolation: Interpolation,
    pub times: Vec<f32>,
    pub values: Vec<f32>,
    pub width: usize,
}
impl Channel {
    pub fn validate(&self) -> Result<(), Error> {
        let multiplier = if self.interpolation == Interpolation::CubicSpline {
            3
        } else {
            1
        };
        if self.width == 0
            || self.times.is_empty()
            || self.times.iter().any(|t| !t.is_finite() || *t < 0.)
            || self.times.windows(2).any(|w| w[0] >= w[1])
            || self.values.len() != self.times.len() * self.width * multiplier
            || self.values.iter().any(|v| !v.is_finite())
        {
            return Err(err("invalid animation keyframes"));
        }
        if self.property == gltf::animation::Property::Rotation {
            let stride = self.width * multiplier;
            let offset = if multiplier == 3 { 4 } else { 0 };
            if self
                .values
                .chunks_exact(stride)
                .any(|key| Vec4::from_slice(&key[offset..offset + 4]).length_squared() < 1e-12)
            {
                return Err(err("zero animation quaternion"));
            }
        }
        Ok(())
    }
    pub fn sample(&self, t: f32) -> Vec<f32> {
        let cubic = self.interpolation == Interpolation::CubicSpline;
        let stride = self.width * if cubic { 3 } else { 1 };
        let value =
            |key: usize, slot: usize, c: usize| self.values[key * stride + slot * self.width + c];
        let right = self.times.partition_point(|key| *key <= t);
        let a = right.saturating_sub(1).min(self.times.len() - 1);
        let b = right.min(self.times.len() - 1);
        let dt = self.times[b] - self.times[a];
        let u = if dt > 0. {
            ((t - self.times[a]) / dt).clamp(0., 1.)
        } else {
            0.
        };
        if !cubic
            && self.property == gltf::animation::Property::Rotation
            && self.interpolation == Interpolation::Linear
        {
            let q = |key| Quat::from_array(std::array::from_fn(|c| value(key, 0, c))).normalize();
            return q(a).slerp(q(b), u).to_array().to_vec();
        }
        let mut out: Vec<f32> = (0..self.width)
            .map(|c| {
                let v0 = value(a, usize::from(cubic), c);
                let v1 = value(b, usize::from(cubic), c);
                match self.interpolation {
                    Interpolation::Step => v0,
                    Interpolation::Linear => v0 + (v1 - v0) * u,
                    Interpolation::CubicSpline => {
                        let u2 = u * u;
                        let u3 = u2 * u;
                        (2. * u3 - 3. * u2 + 1.) * v0
                            + (u3 - 2. * u2 + u) * dt * value(a, 2, c)
                            + (-2. * u3 + 3. * u2) * v1
                            + (u3 - u2) * dt * value(b, 0, c)
                    }
                }
            })
            .collect();
        if self.property == gltf::animation::Property::Rotation {
            let q = Vec4::from_slice(&out);
            let q = if q.length_squared() > 1e-12 {
                q.normalize()
            } else {
                Vec4::W
            };
            out.copy_from_slice(&q.to_array());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn channel(mode: Interpolation, values: Vec<f32>) -> Channel {
        Channel {
            node: 0,
            property: gltf::animation::Property::MorphTargetWeights,
            interpolation: mode,
            times: vec![0., 2.],
            values,
            width: 1,
        }
    }
    #[test]
    fn interpolation_and_endpoints() {
        let c = channel(Interpolation::Linear, vec![2., 6.]);
        c.validate().unwrap();
        assert_eq!(c.sample(-1.), vec![2.]);
        assert_eq!(c.sample(1.), vec![4.]);
        assert_eq!(c.sample(9.), vec![6.]);
        // Hermite tangents scale by the two-second key interval.
        let c = channel(Interpolation::CubicSpline, vec![0., 0., 2., 0., 0., 0.]);
        assert_eq!(c.sample(1.), vec![0.5]);
        assert_eq!(
            channel(Interpolation::Step, vec![2., 6.]).sample(1.),
            vec![2.]
        );
        let c = channel(Interpolation::CubicSpline, vec![0., 2., 1., 1., 6., 0.]);
        assert_eq!(c.sample(1.), vec![4.]);
    }
    #[test]
    fn rejects_duplicate_times_and_bad_counts() {
        let mut c = channel(Interpolation::Linear, vec![2., 6.]);
        c.times[1] = 0.;
        assert!(c.validate().is_err());
        assert!(
            channel(Interpolation::CubicSpline, vec![0., 1.])
                .validate()
                .is_err()
        );
    }
}
