//! Independent 3D animation component. No window, clock, thread or product state.
//! glTF 2.0 (`.glb`, `.gltf`) assets; platform rendering through wgpu.
//! See the crate README for the intentionally bounded preview material model.
mod animation;
mod asset;
mod camera;
mod deform;
mod gpu;
mod instances;
mod interiors;
mod visibility;
pub use visibility::ScreenDetail3D;
mod local_lighting;
pub use local_lighting::{Dimming3D, LocalLight3D, LocalLighting3D, ShadowProxy3D};

pub use asset::{Asset3D, ClipInfo};
pub use camera::Camera3D;
pub use gpu::Renderer3D;
pub use instances::{
    Deformation3D, Environment3D, FollowRoute3D, Instance3D, InstanceBatch3D, InstanceScene3D,
    Lighting3D, Removal3D, Rolling3D, Route3D, RouteSchedule3D, Visibility3D,
};
use tekes_ui_scene::{ImageId, ImageQuad, Rect, Scene};

/// Import, unsupported-feature, argument or GPU error, with actionable context.
#[derive(Debug, Clone)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
pub(crate) fn err(message: impl Into<String>) -> Error {
    Error(message.into())
}

/// Host-controlled animation policy. Reduced motion and visibility freeze time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Once,
    Loop,
}

/// Reusable component state. Hosts own asset selection, layout and frame scheduling.
#[derive(Debug)]
pub struct Scene3D {
    pub(crate) asset: Asset3D,
    pub(crate) clip: Option<usize>,
    pub(crate) time: f32,
    playing: bool,
    pub repeat: Repeat,
    pub visible: bool,
    pub reduced_motion: bool,
    speed: f32,
}
impl Scene3D {
    pub fn new(asset: Asset3D) -> Self {
        Self {
            asset,
            clip: None,
            time: 0.,
            playing: false,
            repeat: Repeat::Loop,
            visible: true,
            reduced_motion: false,
            speed: 1.,
        }
    }
    pub fn asset(&self) -> &Asset3D {
        &self.asset
    }
    /// Selecting a clip resets to its first frame and pauses playback.
    pub fn select_clip(&mut self, index: usize) -> Result<(), Error> {
        if index >= self.asset.clips.len() {
            return Err(err("animation index out of range"));
        }
        self.clip = Some(index);
        self.time = 0.;
        self.playing = false;
        Ok(())
    }
    pub fn play(&mut self) {
        self.playing = self.clip.is_some();
    }
    pub fn pause(&mut self) {
        self.playing = false;
    }
    pub fn time(&self) -> f32 {
        self.time
    }
    pub fn set_speed(&mut self, speed: f32) -> Result<(), Error> {
        if !speed.is_finite() || speed <= 0. {
            return Err(err("speed must be finite and positive"));
        }
        self.speed = speed;
        Ok(())
    }
    pub fn seek(&mut self, seconds: f32) -> Result<(), Error> {
        if !seconds.is_finite() || seconds < 0. {
            return Err(err("time must be finite and nonnegative"));
        }
        let duration = self.clip.map_or(0., |i| self.asset.clips[i].info.duration);
        self.time = seconds.min(duration);
        Ok(())
    }
    pub fn needs_frame(&self) -> bool {
        self.playing
            && self.visible
            && !self.reduced_motion
            && self
                .clip
                .is_some_and(|i| self.asset.clips[i].info.duration > 0.)
    }
    /// Advance by the host's display-frame delta; hidden time is never accumulated.
    pub fn advance(&mut self, delta_seconds: f32) -> Result<bool, Error> {
        if !delta_seconds.is_finite() || delta_seconds < 0. {
            return Err(err("invalid frame delta"));
        }
        if !self.needs_frame() || delta_seconds == 0. {
            return Ok(false);
        }
        let duration = self.asset.clips[self.clip.unwrap()].info.duration;
        let t = self.time as f64 + delta_seconds as f64 * self.speed as f64;
        self.time = match self.repeat {
            Repeat::Loop => t.rem_euclid(duration as f64) as f32,
            Repeat::Once => {
                if t >= duration as f64 {
                    self.playing = false;
                }
                t.min(duration as f64) as f32
            }
        };
        Ok(true)
    }
    /// Compose a registered render result into the ordinary UI scene. Host retains
    /// ImageId ownership; replace GPU backing storage on resize, unregister on teardown.
    pub fn paint(&self, scene: &mut Scene, bounds: Rect, image: ImageId) {
        if self.visible {
            scene.push_image(ImageQuad {
                rect: bounds,
                image,
                opacity: 1.,
            });
        }
    }
}
