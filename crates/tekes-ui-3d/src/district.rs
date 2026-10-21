//! Host adapter only: translate the data-only city recipe into generic GPU motion.
//! No window events, rendering shaders, or city rules live here.
use tekes_ui_city::{CityClock, DistrictConfig};
use tekes_ui_scene3d::{
    Deformation3D, Environment3D, FollowRoute3D, Instance3D, InstanceBatch3D, InstanceScene3D,
    Renderer3D, Route3D, Scene3D,
};

pub struct DistrictDemo {
    pub clock: CityClock,
    pub config: DistrictConfig,
}
impl DistrictDemo {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config =
            DistrictConfig::from_json(&std::fs::read(path)?).map_err(std::io::Error::other)?;
        Ok(Self {
            clock: CityClock::default(),
            config,
        })
    }
    pub fn configure(
        &self,
        gpu: &mut Renderer3D,
        view: &Scene3D,
    ) -> Result<(), tekes_ui_scene3d::Error> {
        let recipe = self.instance_recipe();
        gpu.configure_instances(view, &recipe)?;
        gpu.configure_screen_detail(view, &screen_detail(&recipe))
    }
    fn instance_recipe(&self) -> InstanceScene3D {
        let mut batches = Vec::new();
        let actors = |people: bool| -> Vec<Instance3D> {
            let list = if people {
                &self.config.pedestrians
            } else {
                &self.config.vehicles
            };
            list.iter()
                .map(|a| Instance3D {
                    scale: [a.scale; 3],
                    tint: [a.color[0], a.color[1], a.color[2], 1.],
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
                .collect()
        };
        batches.push(InstanceBatch3D {
            node: "car".into(),
            instances: actors(false),
        });
        batches.push(InstanceBatch3D {
            node: "person-body".into(),
            instances: actors(true),
        });
        for (name, x, y, amplitude, phase) in [
            ("person-leg-left", -0.075, 0.50, 0.48, 0.),
            ("person-leg-right", 0.075, 0.50, 0.48, std::f32::consts::PI),
            ("person-arm-left", -0.18, 0.88, 0.38, std::f32::consts::PI),
            ("person-arm-right", 0.18, 0.88, 0.38, 0.),
        ] {
            let instances = actors(true)
                .into_iter()
                .zip(&self.config.pedestrians)
                .map(|(mut i, a)| {
                    i.deformation = Deformation3D::Swing {
                        amplitude,
                        frequency: 7.8 * a.speed,
                        phase: phase + a.phase,
                        pivot: [x, y, 0.],
                        axis: [1., 0., 0.],
                    };
                    i
                })
                .collect();
            batches.push(InstanceBatch3D {
                node: name.into(),
                instances,
            });
        }
        for (species, node) in ["tree-broadleaf", "tree-fanleaf", "tree-slender"]
            .iter()
            .enumerate()
        {
            batches.push(InstanceBatch3D {
                node: (*node).into(),
                instances: self
                    .config
                    .trees
                    .iter()
                    .filter(|t| t.species == species)
                    .map(|t| Instance3D {
                        translation: t.position,
                        scale: [t.scale; 3],
                        deformation: Deformation3D::Wind {
                            amplitude: self.config.wind_amplitude,
                            frequency: 1.5,
                            phase: t.phase,
                            root_y: 0.,
                        },
                        ..Default::default()
                    })
                    .collect(),
            });
        }
        if let Some(c) = &self.config.construction {
            batches.push(InstanceBatch3D {
                node: "construction-frame".into(),
                instances: vec![Instance3D {
                    translation: c.position,
                    deformation: Deformation3D::BuildUp {
                        floor_height: c.floor_height,
                        seconds_per_floor: c.seconds_per_floor,
                        levels: c.levels,
                        phase: c.phase,
                    },
                    ..Default::default()
                }],
            });
            batches.push(InstanceBatch3D {
                node: "construction-facade".into(),
                instances: vec![Instance3D {
                    translation: c.position,
                    deformation: Deformation3D::BuildUp {
                        floor_height: c.floor_height,
                        seconds_per_floor: c.seconds_per_floor,
                        levels: c.levels,
                        phase: (c.phase - c.facade_delay_floors).max(0.),
                    },
                    ..Default::default()
                }],
            });
            batches.push(InstanceBatch3D {
                node: "construction-scaffold".into(),
                instances: vec![Instance3D {
                    translation: c.position,
                    deformation: Deformation3D::StagedReveal {
                        step_height: c.floor_height,
                        seconds_per_step: c.seconds_per_floor,
                        steps: c.levels,
                        initial_steps: c.phase,
                        hold_seconds: c.scaffold_hold_seconds
                            + c.facade_delay_floors * c.seconds_per_floor,
                        seconds_per_removal_step: c.scaffold_removal_seconds_per_floor,
                    },
                    ..Default::default()
                }],
            });
            for node in ["crane-tower", "crane-boom"] {
                batches.push(InstanceBatch3D {
                    node: node.into(),
                    instances: vec![Instance3D {
                        translation: c.crane_position,
                        deformation: if node == "crane-boom" {
                            Deformation3D::Swing {
                                amplitude: 0.75,
                                frequency: 0.17,
                                phase: 0.,
                                pivot: [0., 6., 0.],
                                axis: [0., 1., 0.],
                            }
                        } else {
                            Deformation3D::None
                        },
                        ..Default::default()
                    }],
                });
            }
        }
        for (node, amplitude, frequency) in [
            ("water", self.config.wave_amplitude, 1.25),
            ("pond", self.config.wave_amplitude * 0.3, 2.1),
        ] {
            batches.push(InstanceBatch3D {
                node: node.into(),
                instances: vec![Instance3D {
                    deformation: Deformation3D::Waves {
                        amplitude,
                        frequency,
                        phase: 0.,
                    },
                    ..Default::default()
                }],
            });
        }
        InstanceScene3D {
            routes: self
                .config
                .routes
                .iter()
                .map(|r| Route3D {
                    points: r.points.clone(),
                })
                .collect(),
            batches,
        }
    }
    pub fn environment(&self) -> Environment3D {
        Environment3D {
            time_seconds: self.clock.seconds() as f32,
            ..Default::default()
        }
    }
}

pub(crate) fn screen_detail(recipe: &InstanceScene3D) -> Vec<tekes_ui_scene3d::ScreenDetail3D> {
    recipe
        .batches
        .iter()
        .filter_map(|b| {
            let (reference, fade, motion, padding) = if b.node.starts_with("person-") {
                (
                    1.2,
                    if b.node.ends_with("headphones") {
                        [8., 12.]
                    } else {
                        [3., 6.]
                    },
                    [6., 12.],
                    0.35,
                )
            } else if b.node.starts_with("car") {
                (1.8, [4., 8.], [0., 0.], 0.)
            } else if b.node.starts_with("tree-") {
                (0., [4., 8.], [8., 12.], 1.5)
            } else {
                return None;
            };
            Some(tekes_ui_scene3d::ScreenDetail3D {
                node: b.node.clone(),
                reference_size: reference,
                fade_pixels: fade,
                motion_pixels: motion,
                motion_padding: padding,
            })
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
pub(crate) fn instance_recipe_for_test(district: &DistrictDemo) -> InstanceScene3D {
    district.instance_recipe()
}

#[cfg(test)]
mod recipe_tests {
    use super::*;
    const RECIPE: &[u8] = include_bytes!("../assets/city-block.json");
    #[test]
    fn district_routes_are_continuous_and_actor_spacing_is_safe() {
        let config = DistrictConfig::from_json(RECIPE).unwrap();
        assert_eq!(
            (
                config.vehicles.len(),
                config.pedestrians.len(),
                config.trees.len()
            ),
            (10, 28, 18)
        );
        for (route, r) in config.routes.iter().enumerate() {
            let length = (0..r.points.len())
                .map(|i| {
                    r.points[i]
                        .iter()
                        .zip(r.points[(i + 1) % r.points.len()])
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f32>()
                        .sqrt()
                })
                .sum::<f32>();
            assert_eq!(
                config.position_on_route(route, 0.),
                config.position_on_route(route, length)
            );
            for time in [0., 1., 8., 29., 120.] {
                let actors: Vec<_> = config
                    .vehicles
                    .iter()
                    .filter(|a| a.route == route)
                    .collect();
                for (i, a) in actors.iter().enumerate() {
                    let p = config
                        .position_on_route(
                            route,
                            a.distance
                                + time * a.speed
                                + a.speed_variation * ((time + a.phase).sin() - a.phase.sin()),
                        )
                        .unwrap();
                    for b in &actors[i + 1..] {
                        let q = config
                            .position_on_route(
                                route,
                                b.distance
                                    + time * b.speed
                                    + b.speed_variation * ((time + b.phase).sin() - b.phase.sin()),
                            )
                            .unwrap();
                        let distance = p
                            .iter()
                            .zip(q)
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f32>()
                            .sqrt();
                        assert!(distance > 2., "cars on the same lane must remain separated");
                    }
                }
            }
        }
        assert!(
            config
                .pedestrians
                .iter()
                .any(|a| (a.speed - config.pedestrians[0].speed).abs() > 0.1)
        );
        for species in 0..3 {
            assert!(config.trees.iter().any(|t| t.species == species));
        }
        assert!(
            config
                .vehicles
                .iter()
                .all(|a| a.scale < 0.75 && a.speed_variation > 0.)
        );
        assert!(config.pedestrians.iter().all(|a| a.scale < 0.7));
        let mut invalid = config.clone();
        invalid.construction.as_mut().unwrap().seconds_per_floor = 0.;
        assert!(invalid.validate().is_err());
        invalid = config.clone();
        invalid.trees[0].species = 3;
        assert!(invalid.validate().is_err());
        let mut bad = config.clone();
        bad.vehicles[0].route = 99;
        assert!(bad.validate().is_err());
        bad = config.clone();
        bad.routes[0].points[1] = bad.routes[0].points[0];
        assert!(bad.validate().is_err());
    }
}
