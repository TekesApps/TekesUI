//! Road wheel placement only. Scene3D rolls vertices/normals from the exact body route distance.
use tekes_ui_city::Actor;
use tekes_ui_scene3d::{Instance3D, InstanceBatch3D, Rolling3D};
pub fn batches(actors: &[Actor], bodies: &[Instance3D]) -> Vec<InstanceBatch3D> {
    [
        ("sedan-rear", 0.21 * 0.78, 0.17 * 0.78, -0.49),
        ("sedan-front", 0.21 * 0.78, 0.17 * 0.78, 0.51),
        ("heavy-front", 0.22, 0.19, 0.69),
        ("ambulance-rear", 0.22, 0.19, -0.72),
        ("heavy-middle", 0.22, 0.19, -0.55),
        ("heavy-rear", 0.22, 0.19, -1.08),
    ]
    .into_iter()
    .map(|(part, axle_y, radius, axle_z)| InstanceBatch3D {
        node: format!("road-wheel-{part}"),
        instances: actors
            .iter()
            .zip(bodies)
            .filter(|(a, _)| match part {
                "sedan-rear" | "sedan-front" => a.appearance <= 1,
                "heavy-front" => a.appearance >= 2,
                "ambulance-rear" => a.appearance == 2,
                _ => a.appearance >= 3,
            })
            .map(|(_, body)| {
                let mut wheel = body.clone();
                wheel.tint = [1.; 4];
                wheel.rolling = Some(Rolling3D {
                    radius,
                    axle_y,
                    axle_z,
                });
                wheel
            })
            .collect(),
    })
    .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene3d::FollowRoute3D;
    #[test]
    fn every_vehicle_has_correct_axles_and_identical_body_speed_and_phase() {
        let mut config =
            tekes_ui_city::DistrictConfig::from_json(include_bytes!("../assets/city-block.json"))
                .unwrap();
        // Explicitly include a parked vehicle; no independent wheel speed exists.
        config.vehicles[0].speed = 0.;
        config.vehicles[0].speed_variation = 0.;
        let bodies: Vec<_> = config
            .vehicles
            .iter()
            .map(|a| Instance3D {
                scale: [a.scale; 3],
                route: Some(FollowRoute3D {
                    route: a.route,
                    distance: a.distance,
                    speed: a.speed,
                    speed_variation: a.speed_variation,
                    phase: a.phase,
                    lateral_offset: a.lateral_offset,
                }),
                ..Default::default()
            })
            .collect();
        let all = batches(&config.vehicles, &bodies);
        for (a, body) in config.vehicles.iter().zip(&bodies) {
            let r = body.route.unwrap();
            let wheels: Vec<_> = all
                .iter()
                .flat_map(|b| &b.instances)
                .filter(|w| {
                    let v = w.route.unwrap();
                    v.route == r.route && v.distance == r.distance && v.phase == r.phase
                })
                .collect();
            assert_eq!(wheels.len(), if a.appearance >= 3 { 3 } else { 2 });
            for w in wheels {
                let v = w.route.unwrap();
                assert_eq!(v.speed, r.speed);
                assert_eq!(v.speed_variation, r.speed_variation);
                assert_eq!(w.scale, body.scale);
                assert_eq!(w.tint, [1.; 4]);
                assert!(w.rolling.unwrap().radius > 0.);
            }
        }
    }
}
