//! Station visual demand and immutable passenger recipes, independent of rendering.
//! The host estimates demand once from occupied residents/jobs; GPU schedules
//! all walking, waiting, boarding, alighting and night-time departures.
use tekes_ui_city::DistrictConfig;
use tekes_ui_scene3d::{
    Deformation3D, FollowRoute3D, Instance3D, InstanceBatch3D, Route3D, RouteSchedule3D,
    Visibility3D,
};
fn per_train(config: &DistrictConfig) -> usize {
    config.station_demand.as_ref().map_or(0, |d| {
        ((d.occupied_residents * 0.12 + d.occupied_jobs * 0.15) * d.rail_share)
            .round()
            .clamp(0., 13.) as usize
    })
}
pub fn passenger_count(config: &DistrictConfig) -> usize {
    per_train(config) * config.trains.len()
}
fn length(points: &[[f32; 3]]) -> f32 {
    points.windows(2).map(|p| distance(p[0], p[1])).sum()
}
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}
pub fn extend(config: &DistrictConfig, batches: &mut [InstanceBatch3D]) -> Vec<Route3D> {
    let mut routes = Vec::new();
    let count = per_train(config);
    let mut people = Vec::with_capacity(passenger_count(config));
    for (train_index, train) in config.trains.iter().enumerate() {
        let points = &config.routes[train.route].points;
        let total = length(points) + distance(*points.last().unwrap(), points[0]);
        let period = total / train.speed + (total / train.speed).min(2.5) + 7.;
        let direction = (points[1][2] - points[0][2]).signum();
        let x = points[0][0];
        let side = if train_index == 0 { 1. } else { -1. };
        let boarding = (count * 6 / 10).max(1).min(count);
        let alighting = (count - boarding).min(3);
        for j in 0..count {
            let door = j % 4;
            let coach = j / 4;
            let z = direction
                * ((coach as f32 - 1.) * crate::transit::COACH_PITCH
                    + [-1.20, -0.40, 0.40, 1.20][door] * 1.4);
            let queue = [x + side * 0.50, 0.34, z];
            let cabin = [x + side * 0.15, 0.375, z];
            let mut instance = Instance3D {
                scale: [0.38; 3],
                visibility: if j % 6 == 0 {
                    None
                } else {
                    Some(Visibility3D {
                        clock: 1,
                        start_seconds: 65. + j as f32 * 11.,
                        fade_seconds: 8.,
                    })
                },
                ..Default::default()
            };
            if j < boarding {
                let duration = 2.2;
                let speed = distance(queue, cabin) / duration;
                let route = config.routes.len() + routes.len();
                routes.push(Route3D {
                    points: vec![queue, cabin, [cabin[0], -1., cabin[2]]],
                });
                instance.route = Some(FollowRoute3D {
                    route,
                    distance: 0.,
                    speed,
                    speed_variation: 0.,
                    phase: 0.,
                    lateral_offset: 0.,
                });
                instance.route_schedule = Some(RouteSchedule3D {
                    period_seconds: period,
                    phase_seconds: train.phase,
                    start_seconds: 3.15 + (j % 2) as f32 * 0.35,
                    duration_seconds: duration,
                    before_visibility: 1.,
                    after_visibility: 0.,
                    fade_seconds: 0.25,
                });
            } else if j < boarding + alighting {
                let exit = [10.3, 0.18, if z >= 0. { 8.9 } else { -8.9 }];
                // Keep feet on the platform until the stairs. A direct queue-to-exit
                // segment sinks through the deck as it interpolates toward ground level.
                let end = if z >= 0. { 1. } else { -1. };
                let path = vec![
                    cabin,
                    queue,
                    [10.10, 0.34, z],
                    [10.10, 0.34, end * 8.02],
                    [10.30, 0.34, end * 8.02],
                    [10.30, 0.34, end * 8.20],
                    [10.30, 0.29, end * 8.40],
                    [10.30, 0.24, end * 8.60],
                    [10.30, 0.19, end * 8.80],
                    exit,
                ];
                let duration = length(&path) / 0.30;
                let route = config.routes.len() + routes.len();
                let mut closed = path;
                closed.push([exit[0], -1., exit[2]]);
                routes.push(Route3D { points: closed });
                instance.route = Some(FollowRoute3D {
                    route,
                    distance: 0.,
                    speed: 0.30,
                    speed_variation: 0.,
                    phase: 0.,
                    lateral_offset: 0.,
                });
                instance.route_schedule = Some(RouteSchedule3D {
                    period_seconds: period,
                    phase_seconds: train.phase,
                    start_seconds: 1.05 + (j % 2) as f32 * 0.25,
                    duration_seconds: duration,
                    before_visibility: 0.,
                    after_visibility: 0.,
                    fade_seconds: 0.25,
                });
            } else {
                instance.translation = [10.3 + side * 0.17, 0.34, z];
                instance.yaw = side * std::f32::consts::FRAC_PI_2;
            }
            people.push((j % 8, instance));
        }
    }
    for batch in batches {
        for appearance in 0..8 {
            let prefix = format!("person-{appearance}-");
            let Some(part) = batch.node.strip_prefix(&prefix) else {
                continue;
            };
            if part == "headphones" || part.ends_with("-run") {
                continue;
            }
            for (_, base) in people.iter().filter(|(a, _)| *a == appearance) {
                let mut instance = base.clone();
                let limb = match part {
                    "leg-left" => Some((-0.075, 0.50, 0.)),
                    "leg-right" => Some((0.075, 0.50, std::f32::consts::PI)),
                    "arm-left" => Some((-0.18, 0.88, std::f32::consts::PI)),
                    "arm-right" => Some((0.18, 0.88, 0.)),
                    _ => None,
                };
                if let Some((x, y, phase)) = limb {
                    instance.deformation = Deformation3D::Swing {
                        amplitude: if base.route_schedule.is_some() {
                            0.4
                        } else {
                            0.
                        },
                        frequency: 4.2,
                        phase,
                        pivot: [x, y, 0.],
                        axis: [1., 0., 0.],
                    };
                }
                batch.instances.push(instance);
            }
        }
    }
    routes
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn departing_passengers_stay_above_platform_until_stairs() {
        let config =
            DistrictConfig::from_json(include_bytes!("../assets/city-block.json")).unwrap();
        let mut batches = Vec::new();
        let routes = extend(&config, &mut batches);
        let departures: Vec<_> = routes.iter().filter(|r| r.points.len() > 4).collect();
        assert!(!departures.is_empty());
        for route in departures {
            for pair in route.points[..route.points.len() - 1].windows(2) {
                // Sample entire segments, including the middle of the platform.
                for step in 0..=100 {
                    let t = step as f32 / 100.;
                    let y = pair[0][1] + (pair[1][1] - pair[0][1]) * t;
                    let z = pair[0][2] + (pair[1][2] - pair[0][2]) * t;
                    if z.abs() <= 8.05 {
                        assert!(y >= 0.34 - 0.00001);
                    }
                }
            }
        }
    }

    #[test]
    fn demand_is_bounded_and_follows_occupied_population() {
        let mut config =
            DistrictConfig::from_json(include_bytes!("../assets/city-block.json")).unwrap();
        let n = passenger_count(&config);
        assert!(n > 0 && n <= 26);
        let d = config.station_demand.as_mut().unwrap();
        d.occupied_residents = 0.;
        d.occupied_jobs = 0.;
        assert_eq!(passenger_count(&config), 0);
        config.station_demand.as_mut().unwrap().occupied_jobs = 100000.;
        assert_eq!(passenger_count(&config), 26);
    }
}
