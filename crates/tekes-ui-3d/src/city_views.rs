//! City-specific camera seats. Rendering and route integration stay in Scene3D.
use tekes_ui_city::{Actor, DistrictConfig};
use tekes_ui_scene3d::{Camera3D, FollowRoute3D, Instance3D, Route3D, RoutePose3D, RouteSampler3D};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CityView {
    #[default]
    Overview,
    Pedestrian,
    Driver,
    TrainCab,
    Passenger,
}
impl CityView {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Pedestrian,
        Self::Driver,
        Self::TrainCab,
        Self::Passenger,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "1 总览",
            Self::Pedestrian => "2 行人",
            Self::Driver => "3 汽车驾驶",
            Self::TrainCab => "4 电车驾驶",
            Self::Passenger => "5 电车乘客",
        }
    }
    pub fn argument(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Pedestrian => "pedestrian",
            Self::Driver => "driver",
            Self::TrainCab => "train-cab",
            Self::Passenger => "passenger",
        }
    }
}
struct Seat {
    route: RouteSampler3D,
    eye: [f32; 3],
    yaw: f32,
    alternate: Option<RouteSampler3D>,
}
pub struct CityViews {
    pub selected: CityView,
    seats: Vec<Seat>,
    look: [f32; 2],
}
impl CityViews {
    pub fn new(config: &DistrictConfig) -> Result<Self, tekes_ui_scene3d::Error> {
        let actor = |a: &Actor| Instance3D {
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
        };
        let person = config
            .pedestrians
            .first()
            .ok_or_else(|| tekes_ui_scene3d::Error("city views require a pedestrian".into()))?;
        if config.vehicles.is_empty() || config.trains.len() < 2 {
            return Err(tekes_ui_scene3d::Error(
                "city views require vehicles and two trains".into(),
            ));
        }
        let car = config
            .vehicles
            .iter()
            .find(|a| a.appearance == 0)
            .unwrap_or(&config.vehicles[0]);
        let mut cab_instances = crate::transit::batches(&config.trains)
            .into_iter()
            .find(|b| b.node == "transit-cab-front")
            .unwrap()
            .instances;
        let train = cab_instances.remove(0);
        let second_train = cab_instances.remove(0);
        let mut passenger = train.clone();
        passenger.route.as_mut().unwrap().distance -= crate::transit::COACH_PITCH;
        let mut seats = Vec::new();
        for (instance, eye, yaw, train) in [
            (
                actor(person),
                [0., 1.04 * person.scale, 0.13 * person.scale],
                0.,
                false,
            ),
            (
                actor(car),
                [0.11 * car.scale, 0.53 * car.scale, 0.05 * car.scale],
                0.,
                false,
            ),
            (train, [0.10, 0.73, 2.12], 0.18, true),
            (
                passenger,
                [0.10, 0.69, 0.25],
                std::f32::consts::FRAC_PI_2,
                true,
            ),
        ] {
            let route = &config.routes[instance.route.unwrap().route];
            seats.push(Seat {
                route: RouteSampler3D::new(
                    &Route3D {
                        points: route.points.clone(),
                    },
                    &instance,
                )?,
                eye,
                yaw,
                alternate: if train {
                    let mut other = second_train.clone();
                    if eye[2] < 1. {
                        other.route.as_mut().unwrap().distance -= crate::transit::COACH_PITCH;
                    }
                    Some(RouteSampler3D::new(
                        &Route3D {
                            points: config.routes[other.route.unwrap().route].points.clone(),
                        },
                        &other,
                    )?)
                } else {
                    None
                },
            });
        }
        Ok(Self {
            selected: CityView::Overview,
            seats,
            look: [0.; 2],
        })
    }
    pub fn select(&mut self, view: CityView) {
        self.selected = view;
        self.look = [0.; 2];
    }
    pub fn turn(&mut self, x: f32, y: f32) {
        self.look[0] = (self.look[0] + x).clamp(-2.7, 2.7);
        self.look[1] = (self.look[1] + y).clamp(-1.1, 1.1);
    }
    fn pose(
        &self,
        seat: &Seat,
        seconds: f32,
    ) -> Result<Option<(RoutePose3D, bool)>, tekes_ui_scene3d::Error> {
        let pose = seat.route.sample(seconds)?;
        let Some(alternate) = &seat.alternate else {
            return Ok(Some((pose, false)));
        };
        let safe = if self.selected == CityView::TrainCab {
            18.0..=35.5
        } else {
            18.0..=38.0
        };
        if safe.contains(&pose.distance) {
            return Ok(Some((pose, false)));
        }
        let pose = alternate.sample(seconds)?;
        Ok(safe.contains(&pose.distance).then_some((pose, true)))
    }
    pub fn waiting(&self, seconds: f32) -> bool {
        self.selected != CityView::Overview
            && self
                .pose(&self.seats[self.selected as usize - 1], seconds)
                .is_ok_and(|p| p.is_none())
    }
    pub fn update(
        &self,
        camera: &mut Camera3D,
        seconds: f32,
    ) -> Result<(), tekes_ui_scene3d::Error> {
        if self.selected == CityView::Overview {
            return Ok(());
        }
        let seat = &self.seats[self.selected as usize - 1];
        let Some((pose, opposite)) = self.pose(seat, seconds)? else {
            // This demo models one finite block. Wait on the island platform
            // while both trains are outside it; never follow the hidden return.
            let eye = [10.3, 0.86, 1.6];
            let yaw = -2.55 + self.look[0];
            let pitch = self.look[1] - 0.06;
            return camera.look_at(
                eye,
                [
                    eye[0] + yaw.sin() * pitch.cos(),
                    eye[1] + pitch.sin(),
                    eye[2] + yaw.cos() * pitch.cos(),
                ],
                70_f32.to_radians(),
            );
        };
        let eye = pose.transform_point(seat.eye);
        let yaw = pose.yaw + if opposite { -seat.yaw } else { seat.yaw } + self.look[0];
        let pitch = self.look[1];
        let target = [
            eye[0] + yaw.sin() * pitch.cos(),
            eye[1] + pitch.sin(),
            eye[2] + yaw.cos() * pitch.cos(),
        ];
        camera.look_at(eye, target, 70_f32.to_radians())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_seats_share_scene_clock_and_stay_finite_through_train_return() {
        let config =
            DistrictConfig::from_json(include_bytes!("../assets/city-block.json")).unwrap();
        let mut views = CityViews::new(&config).unwrap();
        for view in CityView::ALL.into_iter().skip(1) {
            views.select(view);
            let mut camera = Camera3D::default();
            for t in [0., 7., 12., 30., 60., 120.] {
                views.update(&mut camera, t).unwrap();
            }
            let paused = camera.clone();
            views.update(&mut camera, 120.).unwrap();
            assert_eq!(camera, paused);
            views.turn(0.3, 0.1);
            views.update(&mut camera, 120.).unwrap();
            assert_ne!(camera, paused);
        }
    }
}
