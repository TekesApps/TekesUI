//! Data-only district recipes and a deterministic clock. No renderer, window or UI.
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub name: String,
    pub points: Vec<[f32; 3]>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub route: usize,
    pub distance: f32,
    pub speed: f32,
    #[serde(default)]
    pub speed_variation: f32,
    #[serde(default)]
    pub lateral_offset: f32,
    pub color: [f32; 3],
    pub scale: f32,
    pub phase: f32,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tree {
    #[serde(default)]
    pub species: usize,
    pub position: [f32; 3],
    pub scale: f32,
    pub phase: f32,
}
fn default_scaffold_removal_seconds() -> f32 {
    2.5
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Construction {
    pub position: [f32; 3],
    pub crane_position: [f32; 3],
    #[serde(default)]
    pub facade_delay_floors: f32,
    #[serde(default)]
    pub scaffold_hold_seconds: f32,
    #[serde(default = "default_scaffold_removal_seconds")]
    pub scaffold_removal_seconds_per_floor: f32,
    pub floor_height: f32,
    pub seconds_per_floor: f32,
    pub levels: f32,
    pub phase: f32,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistrictConfig {
    pub asset: String,
    #[serde(default)]
    pub construction: Option<Construction>,
    pub routes: Vec<Route>,
    pub vehicles: Vec<Actor>,
    pub pedestrians: Vec<Actor>,
    pub trees: Vec<Tree>,
    pub wind_amplitude: f32,
    pub wave_amplitude: f32,
}
impl DistrictConfig {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let c: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        c.validate()?;
        Ok(c)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.asset.ends_with(".glb") || self.asset.contains(['/', '\\']) {
            return Err("district asset must name a local GLB file".into());
        }
        if self.routes.is_empty()
            || self.routes.len() > 64
            || self.vehicles.len() > 4096
            || self.pedestrians.len() > 4096
            || self.trees.len() > 4096
        {
            return Err("district count exceeds supported recipe limits".into());
        }
        for r in &self.routes {
            if r.points.len() < 3 || r.points.len() > 256 || r.name.is_empty() {
                return Err("route requires a name and 3..256 points".into());
            }
            for i in 0..r.points.len() {
                let a = r.points[i];
                let b = r.points[(i + 1) % r.points.len()];
                let length = a
                    .iter()
                    .zip(b)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>()
                    .sqrt();
                if !length.is_finite() || length < 0.001 {
                    return Err("route has nonfinite or zero-length segment".into());
                }
            }
        }
        for a in self.vehicles.iter().chain(&self.pedestrians) {
            if a.route >= self.routes.len()
                || !a.distance.is_finite()
                || !a.phase.is_finite()
                || !a.speed_variation.is_finite()
                || a.speed_variation < 0.
                || a.speed_variation > a.speed
                || !a.lateral_offset.is_finite()
                || a.lateral_offset.abs() > 0.3
                || !a.speed.is_finite()
                || a.speed <= 0.
                || !a.scale.is_finite()
                || a.scale <= 0.
                || a.color
                    .iter()
                    .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
            {
                return Err("invalid actor route, speed, scale or color".into());
            }
        }
        for t in &self.trees {
            if t.species > 2
                || t.position.iter().any(|p| !p.is_finite())
                || !t.phase.is_finite()
                || !t.scale.is_finite()
                || t.scale <= 0.
            {
                return Err("invalid tree placement".into());
            }
        }
        if let Some(c) = &self.construction
            && (c
                .position
                .iter()
                .chain(&c.crane_position)
                .any(|v| !v.is_finite())
                || !c.phase.is_finite()
                || c.phase < 0.
                || [c.floor_height, c.seconds_per_floor, c.levels]
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.)
                || c.levels > 64.
                || !c.facade_delay_floors.is_finite()
                || !(0.0..=c.levels).contains(&c.facade_delay_floors)
                || !c.scaffold_hold_seconds.is_finite()
                || c.scaffold_hold_seconds < 0.
                || !c.scaffold_removal_seconds_per_floor.is_finite()
                || c.scaffold_removal_seconds_per_floor <= 0.)
        {
            return Err("invalid construction parameters".into());
        }
        if [self.wind_amplitude, self.wave_amplitude]
            .iter()
            .any(|a| !a.is_finite() || !(0.0..=0.1).contains(a))
        {
            return Err("environment amplitude must be finite in 0..0.1".into());
        }
        Ok(())
    }
    /// CPU reference for tools/tests. Interactive actors are evaluated on the GPU.
    pub fn position_on_route(&self, route: usize, distance: f32) -> Option<[f32; 3]> {
        if !distance.is_finite() {
            return None;
        }
        let r = self.routes.get(route)?;
        let lengths: Vec<_> = (0..r.points.len())
            .map(|i| {
                r.points[i]
                    .iter()
                    .zip(r.points[(i + 1) % r.points.len()])
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>()
                    .sqrt()
            })
            .collect();
        let mut d = distance.rem_euclid(lengths.iter().sum());
        for (i, l) in lengths.into_iter().enumerate() {
            if d <= l {
                let a = r.points[i];
                let b = r.points[(i + 1) % r.points.len()];
                return Some(std::array::from_fn(|j| a[j] + (b[j] - a[j]) * d / l));
            }
            d -= l;
        }
        None
    }
}
#[derive(Clone, Debug)]
pub struct CityClock {
    seconds: f64,
    playing: bool,
}
impl Default for CityClock {
    fn default() -> Self {
        Self {
            seconds: 0.,
            playing: true,
        }
    }
}
impl CityClock {
    pub fn seconds(&self) -> f64 {
        self.seconds
    }
    pub fn playing(&self) -> bool {
        self.playing
    }
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }
    pub fn seek(&mut self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0. || seconds > f64::from(f32::MAX) {
            return Err("invalid city time".into());
        }
        self.seconds = seconds;
        Ok(())
    }
    pub fn reset(&mut self) {
        self.seconds = 0.;
    }
    /// Host supplies active/reduced-motion policy; hidden time is never caught up.
    pub fn advance(&mut self, delta: f32, enabled: bool) -> Result<bool, String> {
        if !delta.is_finite() || delta < 0. {
            return Err("invalid city frame delta".into());
        }
        if !self.playing || !enabled || delta == 0. {
            return Ok(false);
        }
        let next = self.seconds + f64::from(delta);
        if !next.is_finite() || next > f64::from(f32::MAX) {
            return Err("city clock overflow".into());
        }
        self.seconds = next;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_freezes_without_catchup_and_validates_delta() {
        let mut clock = CityClock::default();
        assert!(clock.advance(1., true).unwrap());
        assert!(!clock.advance(100., false).unwrap());
        assert_eq!(clock.seconds(), 1.);
        clock.set_playing(false);
        assert!(!clock.advance(10., true).unwrap());
        clock.set_playing(true);
        clock.advance(0.5, true).unwrap();
        assert_eq!(clock.seconds(), 1.5);
        assert!(clock.advance(f32::NAN, true).is_err());
        clock.reset();
        assert_eq!(clock.seconds(), 0.);
    }
}
