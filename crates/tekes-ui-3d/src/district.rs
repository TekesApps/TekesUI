//! Host adapter only: translate the data-only city recipe into generic GPU motion.
//! No window events, rendering shaders, or city rules live here.
use tekes_ui_city::{Activity, CityClock, DistrictConfig};
use tekes_ui_scene3d::{
    Deformation3D, Dimming3D, Environment3D, FollowRoute3D, Instance3D, InstanceBatch3D,
    InstanceScene3D, LocalLight3D, LocalLighting3D, Removal3D, Renderer3D, Route3D, Scene3D,
    ShadowProxy3D, Visibility3D,
};

pub struct DistrictDemo {
    pub clock: CityClock,
    pub config: DistrictConfig,
    night_start: Option<f64>,
    construction_pause: f64,
    construction_frozen_at: Option<f64>,
}
impl DistrictDemo {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config =
            DistrictConfig::from_json(&std::fs::read(path)?).map_err(std::io::Error::other)?;
        Ok(Self {
            clock: CityClock::default(),
            config,
            night_start: None,
            construction_pause: 0.,
            construction_frozen_at: None,
        })
    }
    pub fn set_night(&mut self, night: bool) {
        if night && self.night_start.is_none() {
            // Capture the already-built state, not night-relative time. Every
            // construction attachment shares this exact held GPU clock value.
            self.construction_frozen_at =
                Some((self.clock.seconds() - self.construction_pause).max(0.));
            self.night_start = Some(self.clock.seconds());
        } else if !night && let Some(start) = self.night_start.take() {
            self.construction_pause += (self.clock.seconds() - start).max(0.);
            self.construction_frozen_at = None;
        }
    }
    pub fn reset(&mut self) {
        self.clock.reset();
        self.construction_pause = 0.;
        if self.night_start.is_some() {
            self.night_start = Some(0.);
            self.construction_frozen_at = Some(0.);
        }
    }
    pub fn motion_clocks(&self) -> [f32; 4] {
        let now = self.clock.seconds();
        [
            self.construction_frozen_at
                .unwrap_or_else(|| (now - self.construction_pause).max(0.)) as f32,
            self.night_start
                .map_or(0., |start| (now - start).max(0.) as f32),
            0.,
            0.,
        ]
    }
    pub fn configure(
        &self,
        gpu: &mut Renderer3D,
        view: &Scene3D,
    ) -> Result<(), tekes_ui_scene3d::Error> {
        let recipe = self.instance_recipe();
        gpu.set_motion_clocks(self.motion_clocks())?;
        gpu.configure_instances(view, &recipe)?;
        let policies = screen_detail(&recipe);
        gpu.configure_screen_detail(view, &policies)?;
        gpu.configure_local_lighting(&self.lighting_recipe(&recipe))
    }
    fn lighting_recipe(&self, instances: &InstanceScene3D) -> LocalLighting3D {
        let mut recipe = LocalLighting3D {
            routes: instances.routes.clone(),
            ..Default::default()
        };
        for l in &self.config.lights {
            let mut attachment = Instance3D::default();
            if l.construction {
                attachment.clock = Some(0);
                if let Some(c) = &self.config.construction {
                    attachment.removal = Some(Removal3D {
                        start_seconds: c.scaffold_removed_at(),
                        duration_seconds: 2.,
                        top_y: 3.0,
                    });
                }
            }
            recipe.lights.push(LocalLight3D {
                attachment,
                position: l.position,
                direction: l.direction,
                color: l.color,
                radius: l.radius,
                outer_cos: l.outer_cos,
                dimming: l.dimming.map(|p| Dimming3D {
                    clock: 1,
                    start_seconds: p[0],
                    end_seconds: p[1],
                    minimum: p[2],
                }),
            });
        }
        for batch in &instances.batches {
            if batch.node.starts_with("car") {
                let truck = matches!(
                    batch.node.as_str(),
                    "car-ambulance" | "car-fire" | "car-garbage"
                );
                for attachment in &batch.instances {
                    for side in [-1., 1.] {
                        recipe.lights.push(LocalLight3D {
                            attachment: attachment.clone(),
                            position: [
                                side * 0.25,
                                if truck { 0.41 } else { 0.365 * 0.78 },
                                if truck { 1.07 } else { 0.82 },
                            ],
                            direction: [side * 0.045, -0.18, 1.],
                            color: [6.0, 5.7, 4.7],
                            radius: 3.7,
                            outer_cos: 0.80,
                            dimming: None,
                        });
                    }
                    recipe.shadows.push(ShadowProxy3D {
                        attachment: attachment.clone(),
                        center: [0., 0.40, 0.],
                        radii: if truck {
                            [0.46, 0.42, 1.0]
                        } else {
                            [0.36, 0.28, 0.73]
                        },
                    });
                }
            } else if matches!(
                batch.node.as_str(),
                "transit-coach" | "transit-cab-front" | "transit-cab-rear"
            ) {
                for attachment in &batch.instances {
                    recipe.shadows.push(ShadowProxy3D {
                        attachment: attachment.clone(),
                        center: [0., 0.5, 0.],
                        radii: [0.369, 0.46, 2.41],
                    });
                    if batch.node == "transit-cab-front" {
                        recipe.lights.push(LocalLight3D {
                            attachment: attachment.clone(),
                            position: [0., 0.945, 2.43],
                            direction: [0., -0.16, 1.],
                            color: [5.0, 4.8, 4.1],
                            radius: 3.7,
                            outer_cos: 0.80,
                            dimming: None,
                        });
                    }
                }
            } else if batch.node.ends_with("-body") {
                for attachment in &batch.instances {
                    recipe.shadows.push(ShadowProxy3D {
                        attachment: attachment.clone(),
                        center: [0., 0.64, 0.],
                        radii: [0.20, 0.58, 0.16],
                    });
                }
            } else if batch.node.starts_with("tree-") {
                for attachment in &batch.instances {
                    recipe.shadows.push(ShadowProxy3D {
                        attachment: attachment.clone(),
                        center: [0., 1.3, 0.],
                        radii: [0.65, 0.82, 0.65],
                    });
                }
            }
        }
        // Block footprints cast broad, conservative proxy shadows onto streets.
        for (center, radii) in [
            ([-3.3, 3.5, -2.65], [1.60, 3.35, 1.30]),
            ([-0.125, 4.4, -2.65], [1.575, 4.25, 1.30]),
            ([3.2, 3.7, -2.65], [1.50, 3.55, 1.30]),
            ([3.2, 2.8, 1.25], [1.50, 2.65, 2.25]),
        ] {
            recipe.shadows.push(ShadowProxy3D {
                attachment: Instance3D::default(),
                center,
                radii,
            });
        }
        recipe
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
                .enumerate()
                .map(|(index, a)| Instance3D {
                    visibility: if people && index % 7 != 0 {
                        Some(Visibility3D {
                            clock: 1,
                            start_seconds: 25. + ((a.phase * 7.93).sin() * 0.5 + 0.5) * 220.,
                            fade_seconds: 5.,
                        })
                    } else {
                        None
                    },
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
        let vehicle_instances = actors(false);
        batches.extend(crate::road_wheels::batches(
            &self.config.vehicles,
            &vehicle_instances,
        ));
        for (appearance, node) in [
            "car",
            "car-police",
            "car-ambulance",
            "car-fire",
            "car-garbage",
        ]
        .iter()
        .enumerate()
        {
            batches.push(InstanceBatch3D {
                node: (*node).into(),
                instances: vehicle_instances
                    .iter()
                    .cloned()
                    .zip(&self.config.vehicles)
                    .filter(|(_, a)| a.appearance == appearance)
                    .map(|(i, _)| i)
                    .collect(),
            });
        }
        for appearance in 0..8 {
            let people: Vec<_> = actors(true)
                .into_iter()
                .zip(&self.config.pedestrians)
                .filter(|(_, a)| a.appearance == appearance)
                .collect();
            batches.push(InstanceBatch3D {
                node: format!("person-{appearance}-body"),
                instances: people
                    .iter()
                    .map(|(instance, a)| {
                        let mut i = instance.clone();
                        if a.activity == Activity::Jogging {
                            i.deformation = Deformation3D::Swing {
                                amplitude: 0.10,
                                frequency: 0.,
                                phase: std::f32::consts::FRAC_PI_2,
                                pivot: [0., 0.5, 0.],
                                axis: [1., 0., 0.],
                            };
                        }
                        i
                    })
                    .collect(),
            });
            batches.push(InstanceBatch3D {
                node: format!("person-{appearance}-headphones"),
                instances: people
                    .iter()
                    .filter(|(_, a)| a.activity == Activity::Listening)
                    .map(|(i, _)| i.clone())
                    .collect(),
            });
            for (part, x, y, phase) in [
                ("leg-left", -0.075, 0.50, 0.),
                ("leg-right", 0.075, 0.50, std::f32::consts::PI),
                ("arm-left", -0.18, 0.88, std::f32::consts::PI),
                ("arm-right", 0.18, 0.88, 0.),
                ("arm-left-run", -0.18, 0.88, std::f32::consts::PI),
                ("arm-right-run", 0.18, 0.88, 0.),
            ] {
                let instances = people
                    .iter()
                    .filter(|(_, a)| {
                        !part.starts_with("arm")
                            || part.ends_with("run") == (a.activity == Activity::Jogging)
                    })
                    .map(|(instance, a)| {
                        let mut i = instance.clone();
                        let leg = part.starts_with("leg");
                        let amplitude = match (a.activity, leg) {
                            (Activity::Jogging, true) => 0.75,
                            (Activity::Jogging, false) => 0.55,
                            (Activity::Strolling, true) => 0.30,
                            (Activity::Strolling, false) => 0.20,
                            (Activity::Listening, false) => 0.16,
                            (_, true) => 0.48,
                            (_, false) => 0.38,
                        };
                        i.deformation = Deformation3D::Swing {
                            amplitude,
                            frequency: 5.0 * a.speed / a.scale,
                            phase: phase + a.phase,
                            pivot: [x, y, 0.],
                            axis: [1., 0., 0.],
                        };
                        i
                    })
                    .collect();
                batches.push(InstanceBatch3D {
                    node: format!("person-{appearance}-{part}"),
                    instances,
                });
            }
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
                node: "construction-worklights".into(),
                instances: vec![Instance3D {
                    removal: Some(Removal3D {
                        start_seconds: c.scaffold_removed_at(),
                        duration_seconds: 2.,
                        top_y: 3.0,
                    }),
                    ..Default::default()
                }],
            });
            batches.push(InstanceBatch3D {
                node: "construction-hoarding".into(),
                instances: vec![Instance3D {
                    removal: Some(Removal3D {
                        start_seconds: c.scaffold_removed_at(),
                        duration_seconds: 2.,
                        top_y: 0.70,
                    }),
                    ..Default::default()
                }],
            });
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
            let mut screen = batches
                .iter()
                .find(|b| b.node == "construction-scaffold")
                .unwrap()
                .clone();
            screen.node = "construction-dust-screen".into();
            batches.push(screen);
            for node in ["crane-tower", "crane-boom"] {
                batches.push(InstanceBatch3D {
                    node: node.into(),
                    instances: vec![Instance3D {
                        translation: c.crane_position,
                        removal: Some(Removal3D {
                            start_seconds: c.scaffold_removed_at() + 1.,
                            duration_seconds: 3.,
                            top_y: 6.6,
                        }),
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
        for node in ["boat", "boat-wake"] {
            batches.push(InstanceBatch3D {
                node: node.into(),
                instances: self
                    .config
                    .boats
                    .iter()
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
                        deformation: if node == "boat" {
                            Deformation3D::Float {
                                amplitude: self.config.wave_amplitude,
                                frequency: 1.25,
                                phase: 0.,
                            }
                        } else {
                            Deformation3D::SurfaceTrail {
                                amplitude: self.config.wave_amplitude,
                                frequency: 1.25,
                                phase: 0.,
                            }
                        },
                        ..Default::default()
                    })
                    .collect(),
            });
        }
        batches.extend(crate::transit::batches(&self.config.trains));
        let mut station_routes = crate::station::extend(&self.config, &mut batches);

        let (cloud_route, cloud_batch) =
            crate::clouds::recipe(self.config.routes.len() + station_routes.len());
        station_routes.push(cloud_route);
        batches.push(cloud_batch);
        for batch in &mut batches {
            if batch.node.starts_with("construction-") || batch.node.starts_with("crane-") {
                for instance in &mut batch.instances {
                    instance.clock = Some(0);
                }
            }
        }
        InstanceScene3D {
            routes: self
                .config
                .routes
                .iter()
                .map(|r| Route3D {
                    points: r.points.clone(),
                })
                .chain(station_routes)
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
            let (reference, fade, motion, padding) = if b.node == "sky-cloud" {
                (1.8, [45., 25.], [0., 0.], 0.)
            } else if b.node.starts_with("person-") {
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
            } else if b.node.starts_with("car") || b.node.starts_with("road-wheel-") {
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
pub(crate) fn lighting_recipe_for_test(district: &DistrictDemo) -> LocalLighting3D {
    district.lighting_recipe(&district.instance_recipe())
}

#[cfg(test)]
mod recipe_tests {
    use super::*;
    const RECIPE: &[u8] = include_bytes!("../assets/city-block.json");
    #[test]
    fn instance_timelines_pause_only_construction_and_keep_departure_ranks_stable() {
        let mut district = DistrictDemo::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/city-block.json"),
        )
        .unwrap();
        district.clock.seek(30.).unwrap();
        district.set_night(true);
        district.clock.seek(270.).unwrap();
        assert_eq!(district.motion_clocks(), [30., 240., 0., 0.]);
        let recipe = district.instance_recipe();
        assert!(
            recipe
                .batches
                .iter()
                .filter(|b| b.node.starts_with("construction-") || b.node.starts_with("crane-"))
                .flat_map(|b| &b.instances)
                .all(|i| i.clock == Some(0))
        );
        let people = &recipe
            .batches
            .iter()
            .find(|b| b.node == "person-0-body")
            .unwrap()
            .instances;
        assert!(people.iter().any(|i| i.visibility.is_some()));
        let all_people: Vec<_> = recipe
            .batches
            .iter()
            .filter(|b| b.node.ends_with("-body"))
            .flat_map(|b| &b.instances)
            .collect();
        let owls = all_people.iter().filter(|i| i.visibility.is_none()).count();
        assert!(owls > 0 && owls < all_people.len() / 3);
        assert!(
            all_people
                .iter()
                .filter_map(|i| i.visibility)
                .all(|v| v.clock == 1 && v.start_seconds < 245.)
        );
        district.set_night(false);
        district.clock.seek(280.).unwrap();
        assert_eq!(district.motion_clocks(), [40., 0., 0., 0.]);
        district.set_night(true);
        district.clock.seek(300.).unwrap();
        assert_eq!(district.motion_clocks(), [40., 20., 0., 0.]);
        district.reset();
        assert_eq!(district.motion_clocks(), [0.; 4]);
        let lights = district.lighting_recipe(&recipe);
        assert_eq!(lights.lights.len(), 46);
        assert_eq!(
            lights.shadows.len(),
            70 + crate::station::passenger_count(&district.config)
        );
        assert!(lights.lights.iter().any(|l| l.attachment.clock == Some(0)));
        assert!(
            lights
                .lights
                .iter()
                .filter_map(|l| l.dimming)
                .any(|c| c.minimum == 0.)
        );
        let mut invalid = district.config.clone();
        invalid.lights[0].direction = [0.; 3];
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn boat_route_stays_in_river_and_crane_waits_for_scaffold() {
        let config = DistrictConfig::from_json(RECIPE).unwrap();
        assert_eq!(config.boats.len(), 1);
        let boat = &config.boats[0];
        for step in 0..2000 {
            let p = config
                .position_on_route(boat.route, step as f32 * 0.05)
                .unwrap();
            // Account for the hull length at both turns and the widening wake.
            assert!(p[0] - 0.95 > 12.6 && p[0] + 0.95 < 16.3);
            assert!(p[2].abs() + 1.0 < 8.9);
        }
        let c = config.construction.as_ref().unwrap();
        assert!((c.scaffold_removed_at() - 63.8).abs() < 0.001);
        let mut bad = config.clone();
        bad.boats[0].route = bad.routes.len();
        assert!(bad.validate().is_err());
        let mut bad = config.clone();
        bad.boats[0].speed = f32::NAN;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn people_have_varied_appearances_sizes_and_independent_speeds() {
        let config = DistrictConfig::from_json(RECIPE).unwrap();
        for appearance in 0..8 {
            assert!(
                config
                    .pedestrians
                    .iter()
                    .any(|a| a.appearance == appearance)
            );
        }
        let average_speed = |elder: bool| {
            let actors: Vec<_> = config
                .pedestrians
                .iter()
                .filter(|a| matches!(a.appearance, 4 | 5) == elder)
                .collect();
            actors.iter().map(|a| a.speed).sum::<f32>() / actors.len() as f32
        };
        assert!(average_speed(true) < average_speed(false));
        assert!(
            config
                .pedestrians
                .iter()
                .all(|a| a.speed_variation > 0. && a.speed_variation < a.speed)
        );
        let mut invalid = config;
        invalid.pedestrians[0].appearance = 8;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn district_routes_are_continuous_and_actor_spacing_is_safe() {
        let config = DistrictConfig::from_json(RECIPE).unwrap();
        assert_eq!(
            (
                config.vehicles.len(),
                config.pedestrians.len(),
                config.trees.len()
            ),
            (10, 32, 18)
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
