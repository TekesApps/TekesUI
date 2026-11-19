//! Independent sky recipe; motion and projected-size fading stay on the GPU.
use tekes_ui_scene3d::{FollowRoute3D, Instance3D, InstanceBatch3D, Route3D};
pub fn recipe(route: usize) -> (Route3D, InstanceBatch3D) {
    (
        Route3D {
            points: vec![
                [-16., 9.5, -10.],
                [20., 9.5, -10.],
                [20., 9.5, 10.],
                [-16., 9.5, 10.],
            ],
        },
        InstanceBatch3D {
            node: "sky-cloud".into(),
            instances: [(2., 1.), (17., 0.8), (43., 1.2), (65., 0.9), (84., 1.1)]
                .into_iter()
                .map(|(distance, scale)| Instance3D {
                    scale: [scale; 3],
                    route: Some(FollowRoute3D {
                        route,
                        distance,
                        speed: 0.045,
                        speed_variation: 0.,
                        phase: 0.,
                        lateral_offset: 0.,
                    }),
                    ..Default::default()
                })
                .collect(),
        },
    )
}
