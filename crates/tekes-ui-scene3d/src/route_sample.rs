//! Small CPU attachment sampler for a camera/audio listener following one GPU actor.
//! Routes are packed once. This never updates actor or vertex buffers.
use crate::instances::{GpuInstance, pack_routes};
use crate::{Error, Instance3D, Route3D, err};
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct RoutePose3D {
    pub position: [f32; 3],
    pub yaw: f32,
    pub distance: f32,
}
impl RoutePose3D {
    /// Transform an already-scaled local attachment offset into world space.
    pub fn transform_point(&self, local: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.yaw.sin_cos();
        [
            self.position[0] + c * local[0] + s * local[2],
            self.position[1] + local[1],
            self.position[2] - s * local[0] + c * local[2],
        ]
    }
}

/// Validated, cached attachment sampling with the same f32 route integration as
/// instance_motion.wgsl. Pass the instance's selected environment clock channel.
/// Samples root translation/yaw only, excluding visibility and vertex deformation.
#[derive(Debug)]
pub struct RouteSampler3D {
    points: Vec<[f32; 4]>,
    instance: GpuInstance,
}
impl RouteSampler3D {
    pub fn new(route: &Route3D, instance: &Instance3D) -> Result<Self, Error> {
        if instance.route.is_none() {
            return Err(err("route attachment requires a routed instance"));
        }
        let (points, spans) = pack_routes(std::slice::from_ref(route))?;
        let mut local = instance.clone();
        local.route.as_mut().unwrap().route = 0;
        Ok(Self {
            points,
            instance: GpuInstance::from_instance(&local, &spans)?,
        })
    }
    pub fn length(&self) -> f32 {
        self.points.last().unwrap()[3]
    }
    pub fn sample(&self, seconds: f32) -> Result<RoutePose3D, Error> {
        if !seconds.is_finite() {
            return Err(err("attachment time must be finite"));
        }
        let i = &self.instance;
        let mut travel =
            seconds * i.route[3] + i.scale[3] * ((seconds + i.axis[3]).sin() - i.axis[3].sin());
        if i.schedule[0] > 0. {
            let q = (seconds + i.schedule[3]).rem_euclid(i.schedule[0]);
            travel = (q - i.schedule[1]).clamp(0., i.schedule[2]) * i.route[3];
        }
        if i.effect[0] == 8. {
            let speed = i.route[3];
            let ramp = i.effect[2].min(self.length() / speed);
            let travel_time = self.length() / speed + ramp;
            let q = (seconds + i.effect[3]).rem_euclid(travel_time + i.effect[1]);
            let u = (q - i.effect[1]).clamp(0., travel_time);
            let d = if u < ramp {
                u * u / (2. * ramp)
            } else if u > travel_time - ramp {
                self.length() / speed - (travel_time - u) * (travel_time - u) / (2. * ramp)
            } else {
                u - ramp * 0.5
            };
            travel = d * speed;
        }
        self.at_distance(i.route[2] + travel)
    }
    pub fn at_distance(&self, distance: f32) -> Result<RoutePose3D, Error> {
        if !distance.is_finite() {
            return Err(err("attachment distance must be finite"));
        }
        let distance = distance.rem_euclid(self.length());
        let i = &self.instance;
        for segment in self.points.windows(2) {
            let a = segment[0];
            let b = segment[1];
            if distance <= b[3] {
                let a3 = Vec3::new(a[0], a[1], a[2]);
                let b3 = Vec3::new(b[0], b[1], b[2]);
                let tangent = (b3 - a3).normalize();
                let p = a3.lerp(b3, (distance - a[3]) / (b[3] - a[3]))
                    + Vec3::new(tangent.z, 0., -tangent.x) * i.pivot[3]
                    + Vec3::new(i.position_yaw[0], i.position_yaw[1], i.position_yaw[2]);
                return Ok(RoutePose3D {
                    position: p.to_array(),
                    yaw: i.position_yaw[3] + tangent.x.atan2(tangent.z),
                    distance,
                });
            }
        }
        Err(err("route attachment distance out of range"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Deformation3D, FollowRoute3D};
    fn follower(transit: bool) -> RouteSampler3D {
        let route = Route3D {
            points: vec![[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 0., 0.]],
        };
        let instance = Instance3D {
            route: Some(FollowRoute3D {
                route: 42,
                distance: 2.,
                speed: 2.,
                speed_variation: if transit { 0. } else { 0.3 },
                phase: 0.7,
                lateral_offset: 0.1,
            }),
            deformation: if transit {
                Deformation3D::Transit {
                    dwell_seconds: 7.,
                    acceleration_seconds: 2.5,
                    phase_seconds: 0.,
                    door_slide: 0.,
                    route_window: None,
                }
            } else {
                Deformation3D::None
            },
            ..Default::default()
        };
        RouteSampler3D::new(&route, &instance).unwrap()
    }
    #[test]
    fn ordinary_motion_matches_gpu_integral_and_wraps() {
        let f = follower(false);
        for t in [0., 0.1, 1., 12., 60., 120.] {
            let pose = f.sample(t).unwrap();
            let expected = (2. + t * 2. + 0.3 * ((t + 0.7).sin() - 0.7_f32.sin())).rem_euclid(40.);
            assert!((pose.distance - expected).abs() < 1e-5);
            assert!(pose.position.iter().all(|v| v.is_finite()));
        }
        let pose = f.at_distance(5.).unwrap();
        assert_eq!(pose.position, [0.1, 0., 5.]);
        assert_eq!(pose.transform_point([0., 1.6, 1.]), [0.1, 1.6, 6.]);
        assert_eq!(f.at_distance(-35.).unwrap().position, pose.position);
        assert!(f.sample(f32::NAN).is_err());
    }
    #[test]
    fn transit_dwell_accelerate_cruise_brake_and_loop_match_gpu() {
        let f = follower(true);
        for (t, d) in [
            (0., 2.),
            (7., 2.),
            (8., 2.4),
            (9.5, 4.5),
            (17., 19.5),
            (28.5, 1.6),
            (29.5, 2.),
            (36.5, 2.),
        ] {
            assert!((f.sample(t).unwrap().distance - d).abs() < 1e-4, "t={t}");
        }
        assert!(
            (f.sample(7.001).unwrap().distance - f.sample(6.999).unwrap().distance).abs() < 1e-4
        );
    }
}
