//! Optional GPU instances and analytic motion. No city, UI, clock or window policy.
use crate::{Error, err};
use bytemuck::{Pod, Zeroable};

/// Closed polyline. The renderer appends the first point and computes arc lengths once.
#[derive(Clone, Debug)]
pub struct Route3D {
    pub points: Vec<[f32; 3]>,
}
#[derive(Clone, Copy, Debug, Default)]
pub enum Deformation3D {
    #[default]
    None,
    /// Root-anchored quadratic bending, with analytically transformed normals.
    Wind {
        amplitude: f32,
        frequency: f32,
        phase: f32,
        root_y: f32,
    },
    /// Two traveling waves with analytic surface normals.
    Waves {
        amplitude: f32,
        frequency: f32,
        phase: f32,
    },
    /// Reveal a structure from the ground up, preserving its original dimensions.
    BuildUp {
        floor_height: f32,
        seconds_per_floor: f32,
        levels: f32,
        phase: f32,
    },
    /// Grow in fixed-size steps, hold, then remove steps from the top down.
    StagedReveal {
        step_height: f32,
        seconds_per_step: f32,
        steps: f32,
        initial_steps: f32,
        hold_seconds: f32,
        seconds_per_removal_step: f32,
    },
    /// Rigid oscillation about a pivot (e.g. an independently instanced limb).
    Swing {
        amplitude: f32,
        frequency: f32,
        phase: f32,
        pivot: [f32; 3],
        axis: [f32; 3],
    },
}
#[derive(Clone, Copy, Debug)]
pub struct FollowRoute3D {
    pub route: usize,
    pub distance: f32,
    pub speed: f32,
    pub speed_variation: f32,
    pub phase: f32,
    pub lateral_offset: f32,
}
#[derive(Clone, Debug)]
pub struct Instance3D {
    pub translation: [f32; 3],
    pub yaw: f32,
    pub scale: [f32; 3],
    pub tint: [f32; 4],
    pub route: Option<FollowRoute3D>,
    pub deformation: Deformation3D,
}
impl Default for Instance3D {
    fn default() -> Self {
        Self {
            translation: [0.; 3],
            yaw: 0.,
            scale: [1.; 3],
            tint: [1.; 4],
            route: None,
            deformation: Deformation3D::None,
        }
    }
}
/// All primitives under this named glTF node use the same instance list.
#[derive(Clone, Debug)]
pub struct InstanceBatch3D {
    pub node: String,
    pub instances: Vec<Instance3D>,
}
#[derive(Clone, Debug, Default)]
pub struct InstanceScene3D {
    pub routes: Vec<Route3D>,
    pub batches: Vec<InstanceBatch3D>,
}
/// Host-owned time; freezing this value freezes environment and route motion.
#[derive(Clone, Copy, Debug)]
pub struct Environment3D {
    pub time_seconds: f32,
    pub wind_direction: [f32; 2],
    pub wind_strength: f32,
}
impl Default for Environment3D {
    fn default() -> Self {
        Self {
            time_seconds: 0.,
            wind_direction: [0.8, 0.4],
            wind_strength: 1.,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GpuInstance {
    pub position_yaw: [f32; 4],
    pub scale: [f32; 4],
    pub tint: [f32; 4],
    pub route: [f32; 4],
    pub effect: [f32; 4],
    pub pivot: [f32; 4],
    pub axis: [f32; 4],
}
impl GpuInstance {
    pub fn identity() -> Self {
        Self {
            position_yaw: [0.; 4],
            scale: [1., 1., 1., 0.],
            tint: [1.; 4],
            route: [0.; 4],
            effect: [0.; 4],
            pivot: [0.; 4],
            axis: [1., 0., 0., 0.],
        }
    }
    pub fn from_instance(i: &Instance3D, routes: &[(u32, u32)]) -> Result<Self, Error> {
        let mut g = Self::identity();
        g.position_yaw = [i.translation[0], i.translation[1], i.translation[2], i.yaw];
        g.scale = [i.scale[0], i.scale[1], i.scale[2], 0.];
        g.tint = i.tint;
        if i.scale.iter().any(|s| *s <= 0.) || i.tint.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err(err("instance requires positive scale and tint in 0..1"));
        }
        if let Some(r) = i.route {
            let &(offset, count) = routes
                .get(r.route)
                .ok_or_else(|| err("instance route index out of range"))?;
            if r.speed < 0. || r.speed_variation < 0. || r.speed_variation > r.speed {
                return Err(err("route speed must be nonnegative"));
            }
            g.route = [offset as f32, count as f32, r.distance, r.speed];
            g.scale[3] = r.speed_variation;
            g.axis[3] = r.phase;
            g.pivot[3] = r.lateral_offset;
        }
        match i.deformation {
            Deformation3D::None => {}
            Deformation3D::BuildUp {
                floor_height,
                seconds_per_floor,
                levels,
                phase,
            } => {
                if floor_height <= 0.
                    || seconds_per_floor <= 0.
                    || levels <= 0.
                    || !seconds_per_floor.is_finite()
                {
                    return Err(err(
                        "build-up requires positive floor height, duration and levels",
                    ));
                }
                g.effect = [4., floor_height, 1. / seconds_per_floor, phase];
                g.pivot[1] = levels;
            }
            Deformation3D::StagedReveal {
                step_height,
                seconds_per_step,
                steps,
                initial_steps,
                hold_seconds,
                seconds_per_removal_step,
            } => {
                if [
                    step_height,
                    seconds_per_step,
                    steps,
                    seconds_per_removal_step,
                ]
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.)
                    || !initial_steps.is_finite()
                    || initial_steps < 0.
                    || !hold_seconds.is_finite()
                    || hold_seconds < 0.
                {
                    return Err(err(
                        "staged reveal requires positive steps/durations and nonnegative hold/initial steps",
                    ));
                }
                g.effect = [5., step_height, 1. / seconds_per_step, initial_steps];
                g.pivot[0] = hold_seconds;
                g.pivot[1] = steps;
                g.axis[0] = 1. / seconds_per_removal_step;
            }
            Deformation3D::Wind {
                amplitude,
                frequency,
                phase,
                root_y,
            } => {
                g.effect = [1., amplitude, frequency, phase];
                g.pivot[1] = root_y;
            }
            Deformation3D::Waves {
                amplitude,
                frequency,
                phase,
            } => g.effect = [2., amplitude, frequency, phase],
            Deformation3D::Swing {
                amplitude,
                frequency,
                phase,
                pivot,
                axis,
            } => {
                let length = axis.iter().map(|v| v * v).sum::<f32>().sqrt();
                if !length.is_finite() || length <= 0. {
                    return Err(err("swing axis must be finite and nonzero"));
                }
                g.effect = [3., amplitude, frequency, phase];
                g.pivot = [pivot[0], pivot[1], pivot[2], g.pivot[3]];
                g.axis = [
                    axis[0] / length,
                    axis[1] / length,
                    axis[2] / length,
                    g.axis[3],
                ];
            }
        }
        if bytemuck::cast_slice::<GpuInstance, f32>(&[g])
            .iter()
            .any(|f| !f.is_finite())
            || g.effect[1] < 0.
            || g.effect[2] < 0.
        {
            return Err(err(
                "instance motion must be finite with nonnegative amplitude/frequency",
            ));
        }
        Ok(g)
    }
}
pub(crate) type PackedRoutes = (Vec<[f32; 4]>, Vec<(u32, u32)>);
pub(crate) fn pack_routes(routes: &[Route3D]) -> Result<PackedRoutes, Error> {
    let mut points = Vec::new();
    let mut spans = Vec::new();
    for route in routes {
        if route.points.len() < 3 || route.points.len() > 256 {
            return Err(err("closed routes require 3..256 points"));
        }
        let offset = points.len() as u32;
        let mut distance = 0.;
        for index in 0..=route.points.len() {
            let p = route.points[index % route.points.len()];
            if p.iter().any(|v| !v.is_finite()) {
                return Err(err("route coordinates must be finite"));
            }
            if index > 0 {
                let last = route.points[(index - 1) % route.points.len()];
                let length = p
                    .iter()
                    .zip(last)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>()
                    .sqrt();
                if !length.is_finite() || length < 0.00001 {
                    return Err(err("route segments must have nonzero finite length"));
                }
                distance += length;
            }
            if !distance.is_finite() {
                return Err(err("route length overflow"));
            }
            points.push([p[0], p[1], p[2], distance]);
        }
        spans.push((offset, (route.points.len() + 1) as u32));
    }
    if points.len() > 1_000_000 {
        return Err(err("too many route points"));
    }
    if points.is_empty() {
        points.push([0.; 4]);
    }
    Ok((points, spans))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_reveal_rejects_invalid_lifecycle_without_expanding_instance_storage() {
        let instance = |hold_seconds, seconds_per_removal_step, initial_steps| Instance3D {
            deformation: Deformation3D::StagedReveal {
                step_height: 0.78,
                seconds_per_step: 8.,
                steps: 5.3,
                initial_steps,
                hold_seconds,
                seconds_per_removal_step,
            },
            ..Default::default()
        };
        assert!(GpuInstance::from_instance(&instance(6., 2.5, 1.4), &[]).is_ok());
        for (hold, removal, initial) in [
            (f32::NAN, 2.5, 1.4),
            (-1., 2.5, 1.4),
            (6., 0., 1.4),
            (6., f32::INFINITY, 1.4),
            (6., 2.5, -1.),
        ] {
            assert!(GpuInstance::from_instance(&instance(hold, removal, initial), &[]).is_err());
        }
        assert_eq!(std::mem::size_of::<GpuInstance>(), 112);
    }
    #[test]
    fn routes_reject_degenerate_data_and_pack_closed_arc_lengths() {
        let r = Route3D {
            points: vec![[0., 0., 0.], [3., 0., 0.], [3., 0., 4.]],
        };
        let (p, s) = pack_routes(&[r]).unwrap();
        assert_eq!(s, [(0, 4)]);
        assert_eq!(p[3][3], 12.);
        assert!(
            pack_routes(&[Route3D {
                points: vec![[0.; 3]; 3]
            }])
            .is_err()
        );
        let bad = Instance3D {
            scale: [0., 1., 1.],
            ..Default::default()
        };
        assert!(GpuInstance::from_instance(&bad, &[]).is_err());
        assert!(
            GpuInstance::from_instance(
                &Instance3D {
                    route: Some(FollowRoute3D {
                        route: 0,
                        distance: 0.,
                        speed: 1.,
                        speed_variation: 0.,
                        phase: 0.,
                        lateral_offset: 0.
                    }),
                    ..Default::default()
                },
                &[]
            )
            .is_err()
        );
    }
}
