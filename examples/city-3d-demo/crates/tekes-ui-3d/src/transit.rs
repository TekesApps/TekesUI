//! Scene recipe adapter only. Motion/door timing lives in generic Scene3D GPU code.
use tekes_ui_city::Actor;
use tekes_ui_scene3d::{Deformation3D, FollowRoute3D, Instance3D, InstanceBatch3D, Rolling3D};
pub const COACH_PITCH: f32 = 4.70;

pub fn batches(trains: &[Actor]) -> Vec<InstanceBatch3D> {
    [
        ("transit-coach", 0.),
        ("transit-cab-front", 0.),
        ("transit-cab-rear", 0.),
        ("transit-interior", 0.),
        ("transit-door-a", -0.18),
        ("transit-door-b", 0.18),
        ("transit-wheel-0", 0.),
        ("transit-wheel-1", 0.),
        ("transit-wheel-2", 0.),
        ("transit-wheel-3", 0.),
    ]
    .into_iter()
    .map(|(node, slide)| InstanceBatch3D {
        node: node.into(),
        instances: trains
            .iter()
            .flat_map(|a| {
                [-COACH_PITCH, 0., COACH_PITCH]
                    .into_iter()
                    .filter(move |&coach| match node {
                        "transit-coach" => coach == 0.,
                        "transit-cab-front" => coach > 0.,
                        "transit-cab-rear" => coach < 0.,
                        _ => true,
                    })
                    .map(move |coach| Instance3D {
                        scale: [a.scale; 3],
                        tint: [a.color[0], a.color[1], a.color[2], 1.],
                        route: Some(FollowRoute3D {
                            route: a.route,
                            distance: a.distance + coach,
                            speed: a.speed,
                            speed_variation: 0.,
                            phase: 0.,
                            lateral_offset: 0.,
                        }),
                        rolling: node.strip_prefix("transit-wheel-").map(|axle| Rolling3D {
                            radius: 0.09,
                            axle_y: 0.09,
                            axle_z: [-1.20, -0.92, 0.92, 1.20][axle.parse::<usize>().unwrap()]
                                * 1.4,
                        }),
                        deformation: Deformation3D::Transit {
                            dwell_seconds: 7.,
                            acceleration_seconds: 2.5,
                            phase_seconds: a.phase,
                            door_slide: slide,
                            route_window: Some([16., 40.]),
                        },
                        ..Default::default()
                    })
            })
            .collect(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn through_trains_share_door_schedule_and_use_opposing_ground_tracks() {
        let mut config =
            tekes_ui_city::DistrictConfig::from_json(include_bytes!("../assets/city-block.json"))
                .unwrap();
        assert_eq!(config.trains.len(), 2);
        let a = &config.routes[config.trains[0].route].points;
        let b = &config.routes[config.trains[1].route].points;
        assert!((a[1][2] - a[0][2]) * (b[1][2] - b[0][2]) < 0.);
        assert!((a[0][0] - b[0][0]).abs() > 1.7);
        assert_eq!(a[0][1], 0.1);
        assert_eq!(b[0][1], 0.1);
        let all = batches(&config.trains);
        assert_eq!(all.len(), 10);
        assert_eq!(
            all.iter().map(|b| b.instances.len()).collect::<Vec<_>>(),
            vec![2, 2, 2, 6, 6, 6, 6, 6, 6, 6]
        );
        for batch in &all {
            for instance in &batch.instances {
                let route = instance.route.unwrap();
                let train = config
                    .trains
                    .iter()
                    .find(|a| a.route == route.route)
                    .unwrap();
                assert!(
                    [
                        train.distance - COACH_PITCH,
                        train.distance,
                        train.distance + COACH_PITCH
                    ]
                    .iter()
                    .any(|d| (route.distance - d).abs() < 0.0001)
                );
                let Deformation3D::Transit {
                    route_window,
                    phase_seconds,
                    ..
                } = instance.deformation
                else {
                    panic!("missing GPU transit schedule")
                };
                assert_eq!(route_window, Some([16., 40.]));
                assert_eq!(phase_seconds, train.phase);
            }
        }
        config.trains[0].route = config.routes.len();
        assert!(config.validate().is_err());
    }
}
