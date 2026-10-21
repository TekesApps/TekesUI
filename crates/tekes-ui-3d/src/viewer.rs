use crate::district::DistrictDemo;
use std::{collections::HashMap, path::PathBuf, time::Instant};
use tekes_ui_platform_macos::*;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, ImageId, Point as ScenePoint, Rect, Scene};
use tekes_ui_scene3d::{Asset3D, Camera3D, Lighting3D, Renderer3D, Scene3D};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    push::push_paragraph,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{ColorRole, Rgba, Theme};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Schedule autonomous animation without reducing input/rotation responsiveness.
/// Uses display target timestamps; scene clocks still receive every host delta.
struct AnimationCadence {
    next: Option<f64>,
    /// Zero follows every native display-link callback (including screen changes).
    hz: u32,
}
impl Default for AnimationCadence {
    fn default() -> Self {
        Self { next: None, hz: 60 }
    }
}
impl AnimationCadence {
    fn due(&mut self, target: f64) -> bool {
        if self.hz == 0 {
            return true;
        }
        let period = 1. / f64::from(self.hz);
        let Some(next) = self.next else {
            self.next = Some(target + period);
            return true;
        };
        if target + 1e-6 < next {
            return false;
        }
        // Preserve phase under display jitter and skip missed deadlines.
        let steps = ((target - next + 1e-6) / period).floor().max(0.) + 1.;
        self.next = Some(next + steps * period);
        true
    }
    fn set_hz(&mut self, hz: u32) {
        if self.hz != hz {
            self.hz = hz;
            self.reset();
        }
    }
    fn reset(&mut self) {
        self.next = None;
    }
}

/// Counts submitted drawable frames, never animation callbacks or occluded draws.
/// The half-second label cadence avoids rebuilding UI text on every GPU frame.
#[derive(Default)]
struct FrameRate {
    start: Option<Instant>,
    frames: u32,
    fps: Option<f64>,
}
impl FrameRate {
    fn refresh(&mut self, now: Instant) -> bool {
        let Some(start) = self.start else {
            self.start = Some(now);
            return false;
        };
        let seconds = now.duration_since(start).as_secs_f64();
        if seconds < 0.5 {
            return false;
        }
        self.fps = (seconds < 1.5).then_some(f64::from(self.frames) / seconds);
        self.frames = 0;
        self.start = Some(now);
        true
    }
    fn clear(&mut self) -> bool {
        let changed = self.start.is_some() || self.fps.is_some();
        *self = Self::default();
        changed
    }
    fn label(&self) -> String {
        match self.fps {
            Some(fps) if fps > 0. => format!("FPS {fps:.1} · {:.1} ms / 帧", 1000. / fps),
            _ => "FPS — · 静止 / 采样中".to_owned(),
        }
    }
}

fn home_camera() -> Camera3D {
    let mut camera = Camera3D::default();
    camera.orbit(0.65, 0.16).unwrap();
    camera.zoom_by(0.85).unwrap();
    camera
}
fn camera_for_asset(name: &str) -> Camera3D {
    if name == "manhattan.glb" {
        let mut camera = Camera3D::default();
        camera.orbit(-0.45, 0.29).unwrap();
        camera.zoom_by(0.56).unwrap();
        camera.set_orthographic(true);
        camera
    } else if name == "city-block.glb" {
        let mut camera = Camera3D::default();
        camera.orbit(0.55, 0.24).unwrap();
        camera.zoom_by(0.56).unwrap();
        camera.set_orthographic(true);
        camera
    } else if name == "health-cabin.glb" {
        let mut camera = Camera3D::default();
        camera.orbit(0.52, -0.08).unwrap();
        camera.zoom_by(0.75).unwrap();
        camera
    } else {
        home_camera()
    }
}
fn demo_asset(name: &str) -> PathBuf {
    let filename = format!("{name}.glb");
    let bundled = std::env::current_exe().ok().and_then(|p| {
        p.parent()?
            .parent()
            .map(|p| p.join("Resources").join(&filename))
    });
    bundled.filter(|p| p.exists()).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(&filename)
    })
}
fn default_asset() -> PathBuf {
    demo_asset("rubiks-cube")
}

pub fn run() -> Result<()> {
    let executable = std::env::current_exe().ok();
    let mut demo = match executable
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
    {
        Some("tekes-ui-manhattan") => "manhattan",
        Some("tekes-ui-health-cabin") => "health-cabin",
        Some("tekes-ui-city-block") => "city-block",
        _ => "cube",
    }
    .to_owned();
    let mut input = None;
    let mut snapshot = None;
    let mut auto_rotate = false;
    let mut benchmark_background = false;
    let mut benchmark_120hz = false;
    let mut play = false;
    let mut night = false;
    let mut scene_time = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--demo" {
            demo = args
                .next()
                .ok_or("--demo requires cube, manhattan, health-cabin or city-block")?
                .to_string_lossy()
                .into_owned();
            if !["cube", "manhattan", "health-cabin", "city-block"].contains(&demo.as_str()) {
                return Err("--demo requires cube, manhattan, health-cabin or city-block".into());
            }
        } else if arg == "--snapshot" {
            snapshot = Some(PathBuf::from(
                args.next().ok_or("--snapshot requires a PNG path")?,
            ));
        } else if arg == "--time" {
            scene_time = Some(
                args.next()
                    .ok_or("--time requires seconds")?
                    .to_string_lossy()
                    .parse::<f64>()?,
            );
        } else if arg == "--benchmark-120hz" {
            benchmark_120hz = true;
        } else if arg == "--benchmark-background" {
            benchmark_background = true;
        } else if arg == "--auto-rotate" {
            auto_rotate = true;
        } else if arg == "--night" {
            night = true;
        } else if arg == "--play" {
            play = true;
        } else if input.replace(PathBuf::from(arg)).is_some() {
            return Err("usage: tekes-ui-3d [model.glb|model.gltf] [--demo cube|manhattan|health-cabin|city-block] [--snapshot output.png] [--auto-rotate] [--play] [--night] [--benchmark-background] [--benchmark-120hz]".into());
        }
    }
    let path = input.unwrap_or_else(|| {
        if demo != "cube" {
            demo_asset(&demo)
        } else {
            default_asset()
        }
    });
    let mut viewer = Viewer::new(
        Asset3D::load(&path)?,
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    )?;
    viewer.night = night;
    viewer.benchmark_background = benchmark_background;
    if benchmark_120hz {
        viewer.cadence.set_hz(120);
    }
    if let Some(d) = &mut viewer.district {
        d.set_night(night);
    }
    if auto_rotate {
        viewer.command(10)?;
    }
    if play && !viewer.playing {
        viewer.command(9)?;
    }
    if let Some(t) = scene_time {
        if let Some(district) = &mut viewer.district {
            district.clock.seek(t)?;
        } else {
            viewer.view.seek(t as f32)?;
        }
    }
    if let Some(path) = snapshot {
        viewer.init(Renderer::new_headless()?, (960, 720), 1.)?;
        let scene = viewer.scene()?;
        let clear = viewer.clear_color();
        let bytes = viewer.ui.as_mut().unwrap().render_to_rgba8(
            &scene,
            &mut viewer.atlas,
            (960, 720),
            1.,
            clear,
        )?;
        let mut encoder = png::Encoder::new(std::fs::File::create(path)?, 960, 720);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&bytes)?;
        return Ok(());
    }
    App::new(AppConfig {
        name: Some("TekesUI3D".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: format!("TekesUI3D — {}", viewer.name),
            size: Size {
                width: 960.,
                height: 720.,
            },
            min_size: Some(Size {
                width: 640.,
                height: 480.,
            }),
            ..Default::default()
        },
        menus(),
        viewer,
    );
    Ok(())
}
fn menus() -> MenuBar {
    let item = |name, id| MenuItem::Command(MenuCommand::new(name, CommandId(id)));
    MenuBar {
        menus: vec![
            Menu::new(
                "Demo",
                vec![
                    item("Rubik’s Cube", 11),
                    item("Manhattan Skyline", 12),
                    item("AI 健康小屋", 13),
                    item("Waterfront City Block", 14),
                ],
            ),
            Menu::new(
                "View",
                vec![
                    item("Reset view (R)", 1),
                    item("Zoom in (+)", 2),
                    item("Zoom out (−)", 3),
                    MenuItem::Separator,
                    item("Orbit left", 4),
                    item("Orbit right", 5),
                    item("Orbit up", 6),
                    item("Orbit down", 7),
                    item("Rotate model (right drag)", 8),
                    MenuItem::Separator,
                    item("Pan left (Shift+←)", 20),
                    item("Pan right (Shift+→)", 21),
                    item("Pan up (Shift+↑)", 22),
                    item("Pan down (Shift+↓)", 23),
                ],
            ),
            Menu::new(
                "Lighting",
                vec![
                    item("Day / 白天", 15),
                    item("Night / 夜晚", 16),
                    item("Toggle day / night (N)", 17),
                ],
            ),
            Menu::new(
                "Animation",
                vec![
                    item("Play / pause animation (Space)", 9),
                    item("Auto rotate (A)", 10),
                    item("Background benchmark (B)", 18),
                    item("Limit to 60 FPS (H)", 19),
                ],
            ),
        ],
    }
}
#[derive(Clone, Copy)]
enum DragMode {
    Orbit,
    Rotate,
    Pan,
}
struct Viewer {
    view: Scene3D,
    district: Option<DistrictDemo>,
    camera: Camera3D,
    home: Camera3D,
    name: String,
    ui: Option<Renderer>,
    gpu: Option<Renderer3D>,
    image: Option<ImageId>,
    size: (f32, f32),
    scale: f32,
    atlas: GlyphAtlas,
    shaper: CoreTextShaper,
    raster: CoreTextRasterizer,
    text: HashMap<String, Paragraph>,
    theme: Theme,
    drag: Option<(MouseButton, Point, DragMode)>,
    active: bool,
    benchmark_background: bool,
    reduced: bool,
    playing: bool,
    spin: bool,
    night: bool,
    night_since: f32,
    cadence: AnimationCadence,
    frame_rate: FrameRate,
    last: Option<f64>,
    pending_redraw: bool,
    menu_state: Option<(bool, bool, bool, bool, bool)>,
    cached_scene: Option<Scene>,
}
impl Viewer {
    fn new(asset: Asset3D, name: String) -> Result<Self> {
        let mut view = Scene3D::new(asset);
        if view.asset().clips().len() > 0 {
            view.select_clip(0)?;
        }
        let district = if name == "city-block.glb" {
            Some(DistrictDemo::load(
                &demo_asset("city-block").with_extension("json"),
            )?)
        } else {
            None
        };
        let playing = district.is_some();
        let home = camera_for_asset(&name);
        let fonts = CoreTextFonts::new();
        Ok(Self {
            view,
            district,
            camera: home.clone(),
            home,
            name,
            ui: None,
            gpu: None,
            image: None,
            size: (960., 720.),
            scale: 1.,
            atlas: GlyphAtlas::default(),
            shaper: CoreTextShaper::new(fonts.clone()),
            raster: CoreTextRasterizer::new(fonts),
            text: HashMap::new(),
            theme: Theme::LIGHT,
            drag: None,
            active: true,
            benchmark_background: false,
            reduced: false,
            playing,
            spin: false,
            night: false,
            night_since: 0.,
            cadence: AnimationCadence::default(),
            frame_rate: FrameRate::default(),
            last: None,
            pending_redraw: true,
            menu_state: None,
            cached_scene: None,
        })
    }
    fn stage(&self) -> Rect {
        Rect::new(
            16.,
            76.,
            (self.size.0 - 32.).max(1.),
            (self.size.1 - 152.).max(1.),
        )
    }
    fn pixels(&self) -> (u32, u32) {
        let r = self.stage();
        (
            (r.size.width * self.scale).round().max(1.) as u32,
            (r.size.height * self.scale).round().max(1.) as u32,
        )
    }
    fn init(&mut self, mut ui: Renderer, pixels: (u32, u32), scale: f32) -> Result<()> {
        self.scale = scale;
        self.size = (pixels.0 as f32 / scale, pixels.1 as f32 / scale);
        let mut gpu = if ["manhattan.glb", "health-cabin.glb", "city-block.glb"]
            .contains(&self.name.as_str())
        {
            Renderer3D::new_antialiased(&ui.gpu_context(), &self.view, self.pixels())?
        } else {
            Renderer3D::new(&ui.gpu_context(), &self.view, self.pixels())?
        };
        if let Some(district) = &self.district {
            district.configure(&mut gpu, &self.view)?;
        }
        self.image = Some(ui.register_gpu_image(gpu.output())?);
        self.gpu = Some(gpu);
        self.ui = Some(ui);
        Ok(())
    }
    fn lighting(&self) -> Lighting3D {
        if self.night {
            Lighting3D {
                ambient: [0.065, 0.09, 0.15],
                direction: [-0.5, 0.8, -0.3],
                directional: [0.08, 0.10, 0.17],
                emissive_strength: 1.,
                effect_time_origin: self.night_since,
            }
        } else {
            Lighting3D {
                emissive_strength: if self.district.is_some() { 0. } else { 1. },
                ..Default::default()
            }
        }
    }
    fn clear_color(&self) -> Rgba {
        if self.night {
            Rgba::new(0.012, 0.025, 0.055, 1.)
        } else {
            ColorRole::DocumentCanvas.resolve(self.theme)
        }
    }
    fn scene(&mut self) -> Result<Scene> {
        let lighting = self.lighting();
        self.gpu
            .as_mut()
            .ok_or("GPU not initialized")?
            .set_lighting(lighting)?;
        if let Some(district) = &self.district {
            self.gpu
                .as_mut()
                .ok_or("GPU not initialized")?
                .set_environment(district.environment())?;
            self.gpu
                .as_mut()
                .unwrap()
                .set_motion_clocks(district.motion_clocks())?;
        }
        self.gpu
            .as_mut()
            .ok_or("GPU not initialized")?
            .render_camera(&self.view, &self.camera)?;
        self.ui_scene()
    }
    fn ui_scene(&mut self) -> Result<Scene> {
        if let Some(scene) = &self.cached_scene {
            return Ok(scene.clone());
        }
        self.atlas.begin_frame();
        let mut scene = Scene::new();
        self.view
            .paint(&mut scene, self.stage(), self.image.unwrap());
        let labels = [
            (
                if self.name == "city-block.glb" {
                    format!(
                        "WATERFRONT · Living City Block · {}",
                        if self.night { "夜晚" } else { "白天" }
                    )
                } else if self.name == "health-cabin.glb" {
                    "AI 健康小屋 · 带窗版".to_owned()
                } else if self.name == "manhattan.glb" {
                    "MANHATTAN · Skyline".to_owned()
                } else {
                    "TekesUI3D".to_owned()
                },
                24.,
                20.,
                20.,
            ),
            (format!("{} · glTF 2.0 · wgpu", self.name), 24., 48., 12.),
            (
                "拖动：视角  ·  Shift / 中键拖动：平移  ·  右键拖动：旋转  ·  滚轮 / + −：缩放"
                    .to_owned(),
                24.,
                self.size.1 - 62.,
                13.,
            ),
            (
                format!(
                    "方向键：视角  ·  Shift 方向键：平移  ·  R：复位  ·  Space：{}  ·  A：自动旋转  ·  N：昼夜",
                    if self.district.is_some() {
                        if self.playing {
                            "暂停街区动态"
                        } else {
                            "播放街区动态"
                        }
                    } else if self.view.asset().clips().len() == 0 {
                        "此场景为静态建筑"
                    } else if self.playing {
                        "暂停动画"
                    } else {
                        "播放动画"
                    }
                ),
                24.,
                self.size.1 - 36.,
                13.,
            ),
        ];
        let fps_label = self.frame_rate.label();
        for (label, x, y, size) in labels.into_iter().chain(std::iter::once((
            fps_label.clone(),
            (self.size.0 - 264.).max(24.),
            48.,
            12.,
        ))) {
            let fps_paragraph;
            let paragraph = if label == fps_label {
                fps_paragraph = self.shaper.shape(
                    &label,
                    &TextStyle::system(size),
                    &ShapeOptions {
                        width: Some(240.),
                        line_spacing: 0.,
                    },
                );
                &fps_paragraph
            } else {
                self.text.entry(label.clone()).or_insert_with(|| {
                    self.shaper.shape(
                        &label,
                        &TextStyle::system(size),
                        &ShapeOptions {
                            width: Some((self.size.0 - 48.) as f64),
                            line_spacing: 0.,
                        },
                    )
                })
            };
            push_paragraph(
                &mut scene,
                &mut self.atlas,
                &mut self.raster,
                paragraph,
                ScenePoint::new(x, y),
                self.scale,
                if self.night {
                    Rgba::new(0.87, 0.90, 0.96, 1.)
                } else {
                    ColorRole::PrimaryText.resolve(self.theme)
                },
            );
        }
        self.cached_scene = Some(scene.clone());
        Ok(scene)
    }
    fn draw(&mut self) -> Result<()> {
        if self.last.is_some() && self.frame_rate.refresh(Instant::now()) {
            self.cached_scene = None;
        }
        let lighting = self.lighting();
        let clear = self.clear_color();
        if self.cached_scene.is_none() {
            self.ui_scene()?;
        }
        let scene = self.cached_scene.as_ref().unwrap();
        let gpu = self.gpu.as_mut().ok_or("GPU not initialized")?;
        gpu.set_lighting(lighting)?;
        let view = &self.view;
        let camera = &self.camera;
        let environment = self
            .district
            .as_ref()
            .map(|d| (d.environment(), d.motion_clocks()));
        let mut acquired = false;
        self.ui
            .as_mut()
            .unwrap()
            .render_with_gpu(scene, &mut self.atlas, clear, |encoder| {
                acquired = true;
                if let Some((environment, clocks)) = environment {
                    gpu.set_environment_and_clocks(environment, clocks)
                        .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))?;
                }
                gpu.encode_camera(view, camera, encoder)
                    .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))
            })?;
        if acquired && self.last.is_some() {
            self.frame_rate.frames += 1;
        }
        Ok(())
    }
    fn select_demo(&mut self, demo: &str) -> Result<()> {
        let path = demo_asset(demo);
        let mut view = Scene3D::new(Asset3D::load(&path)?);
        if view.asset().clips().len() > 0 {
            view.select_clip(0)?;
        }
        let context = self.ui.as_ref().ok_or("GPU not initialized")?.gpu_context();
        let mut gpu = if demo != "rubiks-cube" {
            Renderer3D::new_antialiased(&context, &view, self.pixels())?
        } else {
            Renderer3D::new(&context, &view, self.pixels())?
        };
        let district = if demo == "city-block" {
            Some(DistrictDemo::load(
                &demo_asset("city-block").with_extension("json"),
            )?)
        } else {
            None
        };
        if let Some(d) = &district {
            d.configure(&mut gpu, &view)?;
        }
        self.ui
            .as_mut()
            .unwrap()
            .replace_gpu_image(self.image.unwrap(), gpu.output())?;
        self.playing = district.is_some();
        self.district = district;
        if let Some(d) = &mut self.district {
            d.set_night(self.night);
        }
        self.night_since = 0.;
        self.view = view;
        self.gpu = Some(gpu);
        self.name = format!("{demo}.glb");
        self.home = camera_for_asset(&self.name);
        self.camera = self.home.clone();
        self.spin = false;
        self.drag = None;
        self.last = None;
        self.text.clear();
        self.cached_scene = None;
        Ok(())
    }
    fn command(&mut self, id: u32) -> Result<()> {
        match id {
            1 => {
                self.camera = self.home.clone();
                self.view.seek(0.)?;
                if let Some(district) = &mut self.district {
                    district.reset();
                    self.night_since = 0.;
                }
            }
            2 => self.camera.zoom_by(0.85)?,
            3 => self.camera.zoom_by(1. / 0.85)?,
            4 => self.camera.orbit(-0.18, 0.)?,
            5 => self.camera.orbit(0.18, 0.)?,
            6 => self.camera.orbit(0., 0.15)?,
            7 => self.camera.orbit(0., -0.15)?,
            20 => self.camera.pan_by(-0.06, 0.)?,
            21 => self.camera.pan_by(0.06, 0.)?,
            22 => self.camera.pan_by(0., 0.06)?,
            23 => self.camera.pan_by(0., -0.06)?,
            8 => self.camera.rotate_model(0.25, 0.15, 0.)?,
            9 if self.district.is_some() => {
                self.playing = !self.playing;
                self.district
                    .as_mut()
                    .unwrap()
                    .clock
                    .set_playing(self.playing);
                self.cached_scene = None;
            }
            9 if self.view.asset().clips().len() > 0 => {
                self.cached_scene = None;
                self.playing = !self.playing;
                if self.playing {
                    self.view.play();
                } else {
                    self.view.pause();
                }
            }
            10 => self.spin = !self.spin,
            18 => self.benchmark_background = !self.benchmark_background,
            19 => self
                .cadence
                .set_hz(if self.cadence.hz == 60 { 0 } else { 60 }),
            15..=17 => {
                let was_night = self.night;
                self.night = match id {
                    15 => false,
                    16 => true,
                    _ => !self.night,
                };
                if self.night && !was_night {
                    self.night_since = self
                        .district
                        .as_ref()
                        .map_or(0., |d| d.clock.seconds() as f32);
                }
                if let Some(d) = &mut self.district {
                    d.set_night(self.night);
                }
                self.cached_scene = None;
            }
            11 => self.select_demo("rubiks-cube")?,
            12 => self.select_demo("manhattan")?,
            13 => self.select_demo("health-cabin")?,
            14 => self.select_demo("city-block")?,
            _ => {}
        }
        Ok(())
    }
    fn event(&mut self, event: Event, window: &mut Window) -> Result<()> {
        let mut redraw = true;
        match event {
            Event::Created => {
                // The platform retains this layer/window for the handler's lifetime;
                // construction and rendering stay on its main thread.
                let ui = unsafe {
                    Renderer::new_for_metal_layer(
                        window.metal_layer(),
                        window.drawable_size(),
                        window.scale() as f32,
                    )
                }?;
                self.init(ui, window.drawable_size(), window.scale() as f32)?;
            }
            Event::Resized {
                size,
                scale,
                drawable_size,
            } => {
                self.size = (size.width as f32, size.height as f32);
                self.scale = scale as f32;
                self.text.clear();
                self.cached_scene = None;
                if let Some(ui) = &mut self.ui {
                    ui.resize(drawable_size, self.scale);
                }
                let pixels = self.pixels();
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(pixels)?;
                    self.ui
                        .as_mut()
                        .unwrap()
                        .replace_gpu_image(self.image.unwrap(), gpu.output())?;
                }
            }
            Event::Redraw => {
                if window.is_animating() && !window.is_live_resizing() {
                    self.pending_redraw = true;
                } else if self.ui.is_some() {
                    self.draw()?;
                    self.pending_redraw = false;
                }
                redraw = false;
            }
            Event::AnimationFrame(frame) => {
                let dt = self
                    .last
                    .map_or(frame.refresh_interval, |t| frame.target_timestamp - t)
                    .clamp(0., 0.1) as f32;
                self.last = Some(frame.target_timestamp);
                let mut changed = false;
                if (self.active || self.benchmark_background) && !self.reduced {
                    changed = self.view.advance(dt)?;
                    if let Some(district) = &mut self.district {
                        changed |= district.clock.advance(dt, true)?;
                    }
                    if self.spin {
                        self.camera.rotate_model(dt * 0.45, 0., 0.)?;
                        changed = true;
                    }
                }
                let autonomous_city =
                    self.district.is_some() && self.drag.is_none() && !self.pending_redraw;
                // Automatic rotation shares the city's selected presentation
                // budget. Keep motion time continuous; direct manipulation and
                // explicit redraws still bypass the autonomous cadence.
                let due = if autonomous_city {
                    self.cadence.due(frame.target_timestamp)
                } else {
                    self.cadence.reset();
                    true
                };
                if (changed || self.pending_redraw) && due {
                    self.draw()?;
                    self.pending_redraw = false;
                }
                redraw = false;
            }
            Event::Mouse(e) => match e.kind {
                MouseEventKind::Down { button, .. }
                    if self
                        .stage()
                        .contains(ScenePoint::new(e.position.x as f32, e.position.y as f32)) =>
                {
                    let mode = if button == MouseButton::Middle
                        || (button == MouseButton::Left && (e.modifiers.shift || e.modifiers.alt))
                    {
                        DragMode::Pan
                    } else if button == MouseButton::Right {
                        DragMode::Rotate
                    } else {
                        DragMode::Orbit
                    };
                    self.drag = Some((button, e.position, mode))
                }
                MouseEventKind::Dragged { button } => {
                    if let Some((held, previous, mode)) = self.drag
                        && held == button
                    {
                        let x = (e.position.x - previous.x) as f32 * 0.008;
                        let y = (e.position.y - previous.y) as f32 * 0.008;
                        match mode {
                            DragMode::Pan => {
                                let stage = self.stage();
                                self.camera.pan_by(
                                    (e.position.x - previous.x) as f32 / stage.size.width.max(1.),
                                    -(e.position.y - previous.y) as f32 / stage.size.height.max(1.),
                                )?;
                            }
                            DragMode::Rotate => self.camera.rotate_model(x, y, 0.)?,
                            DragMode::Orbit if button == MouseButton::Left => {
                                self.camera.orbit(-x, y)?
                            }
                            _ => {}
                        }
                        self.drag = Some((held, e.position, mode));
                    }
                }
                MouseEventKind::Up { .. } => self.drag = None,
                _ => redraw = false,
            },
            Event::Scroll(e) => {
                let amount =
                    (e.delta.y as f32 * if e.precise { 0.01 } else { 0.08 }).clamp(-1., 1.);
                self.camera.zoom_by(amount.exp())?;
            }
            Event::Key(e) if e.kind == KeyEventKind::Down && !e.modifiers.command => {
                let direction = match e.key_code {
                    123 => Some((-0.15, 0.)),
                    124 => Some((0.15, 0.)),
                    125 => Some((0., -0.15)),
                    126 => Some((0., 0.15)),
                    _ => None,
                };
                if let Some((x, y)) = direction {
                    if e.modifiers.shift || e.modifiers.alt {
                        self.camera.pan_by(x * 0.4, y * 0.4)?;
                    } else {
                        self.camera.orbit(x, y)?;
                    }
                } else {
                    let id = match e.characters.to_lowercase().as_str() {
                        "r" => 1,
                        "+" | "=" => 2,
                        "-" => 3,
                        " " => 9,
                        "a" => 10,
                        "b" => 18,
                        "h" => 19,
                        "n" => 17,
                        _ => 0,
                    };
                    self.command(id)?;
                }
            }
            Event::Command(id) => {
                self.command(id.0)?;
                window.set_title(&format!("TekesUI3D — {}", self.name));
            }
            Event::Focus(active) => {
                self.active = active;
                self.drag = None;
                self.last = None;
                self.cadence.reset();
            }
            Event::SystemMotionChanged(reduced) => {
                self.reduced = reduced;
                self.view.reduced_motion = reduced;
                self.last = None;
            }
            Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.cached_scene = None;
                self.theme = match (dark, increase_contrast) {
                    (true, true) => Theme::DARK_INCREASED,
                    (true, false) => Theme::DARK,
                    (false, true) => Theme::LIGHT_INCREASED,
                    _ => Theme::LIGHT,
                };
            }
            Event::CloseRequested => window.close(),
            _ => redraw = false,
        }
        let animate = (self.active || self.benchmark_background)
            && !self.reduced
            && (self.view.needs_frame()
                || self.district.as_ref().is_some_and(|d| d.clock.playing())
                || self.spin
                || self.drag.is_some());
        window.set_animating(animate);
        if !animate {
            self.last = None;
            if self.frame_rate.clear() {
                self.cached_scene = None;
                redraw = true;
            }
        }
        let menu_state = (
            self.playing,
            self.spin,
            self.night,
            self.benchmark_background,
            self.cadence.hz == 60,
        );
        if self.menu_state != Some(menu_state) {
            window.set_command_checked(CommandId(9), self.playing);
            window.set_command_checked(CommandId(10), self.spin);
            window.set_command_checked(CommandId(18), self.benchmark_background);
            window.set_command_checked(CommandId(19), self.cadence.hz == 60);
            window.set_command_checked(CommandId(15), !self.night);
            window.set_command_checked(CommandId(16), self.night);
            self.menu_state = Some(menu_state);
        }
        if redraw {
            self.pending_redraw = true;
            window.request_redraw();
        }
        Ok(())
    }
}
impl WindowHandler for Viewer {
    fn handle(&mut self, event: Event, window: &mut Window) {
        if let Err(error) = self.event(event, window) {
            eprintln!("TekesUI3D: {error}");
            window.close();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_frame_matches_standalone_and_keeps_cached_draws_valid() {
        for name in ["rubiks-cube", "city-block", "health-cabin"] {
            let mut direct = Viewer::new(
                Asset3D::load(demo_asset(name)).unwrap(),
                format!("{name}.glb"),
            )
            .unwrap();
            let mut combined = Viewer::new(
                Asset3D::load(demo_asset(name)).unwrap(),
                format!("{name}.glb"),
            )
            .unwrap();
            for viewer in [&mut direct, &mut combined] {
                viewer
                    .init(Renderer::new_headless().unwrap(), (320, 240), 1.)
                    .unwrap();
            }
            for time in [0., 1.2, 24., 46., 55., 64., 66., 70., 80., 0.] {
                for viewer in [&mut direct, &mut combined] {
                    if let Some(d) = &mut viewer.district {
                        d.clock.seek(time).unwrap();
                    } else if viewer.view.asset().clips().len() > 0 {
                        viewer.view.seek(time as f32).unwrap();
                    }
                }
                let scene = direct.scene().unwrap();
                let expected = direct
                    .ui
                    .as_mut()
                    .unwrap()
                    .render_to_rgba8(
                        &scene,
                        &mut direct.atlas,
                        (320, 240),
                        1.,
                        ColorRole::DocumentCanvas.resolve(direct.theme),
                    )
                    .unwrap();
                let scene = combined.ui_scene().unwrap();
                let environment = combined
                    .district
                    .as_ref()
                    .map(|d| (d.environment(), d.motion_clocks()));
                let lighting = combined.lighting();
                let gpu = combined.gpu.as_mut().unwrap();
                gpu.set_lighting(lighting).unwrap();
                let camera = &combined.camera;
                let view = &combined.view;
                let actual = combined
                    .ui
                    .as_mut()
                    .unwrap()
                    .render_to_rgba8_with_gpu(
                        &scene,
                        &mut combined.atlas,
                        (320, 240),
                        1.,
                        ColorRole::DocumentCanvas.resolve(combined.theme),
                        |encoder| {
                            if let Some((environment, clocks)) = environment {
                                gpu.set_environment(environment).unwrap();
                                gpu.set_motion_clocks(clocks).unwrap();
                            }
                            gpu.encode_camera(view, camera, encoder)
                                .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))
                        },
                    )
                    .unwrap();
                assert!(
                    actual == expected,
                    "{name} time {time}: shared submission must match standalone"
                );
                assert_eq!(combined.gpu.as_ref().unwrap().draw_bundle_builds(), 1);
                assert_eq!(combined.gpu.as_ref().unwrap().camera_upload_bytes(), 128);
            }
            if let Some(d) = &combined.district {
                d.configure(combined.gpu.as_mut().unwrap(), &combined.view)
                    .unwrap();
                combined.scene().unwrap();
                assert_eq!(combined.gpu.as_ref().unwrap().draw_bundle_builds(), 2);
            }
            combined.camera.orbit(0.1, 0.1).unwrap();
            combined.scene().unwrap();
            assert_eq!(combined.gpu.as_ref().unwrap().camera_upload_bytes(), 256);
            let builds = combined.gpu.as_ref().unwrap().draw_bundle_builds();
            combined.gpu.as_mut().unwrap().resize((400, 300)).unwrap();
            combined.scene().unwrap();
            assert_eq!(combined.gpu.as_ref().unwrap().draw_bundle_builds(), builds);
        }
    }
    #[test]
    fn district_gpu_motion_is_visible_independent_and_does_not_upload_geometry() {
        // Isolate geometry/actor lifecycle checks from independent material
        // animation, which has its own night/interior pixel regression below.
        let mut bytes = std::fs::read(demo_asset("city-block")).unwrap();
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let mut json: serde_json::Value =
            serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        for material in json["materials"].as_array_mut().unwrap() {
            if let Some(extras) = material.get_mut("extras").and_then(|v| v.as_object_mut()) {
                extras.remove("tekes_interior");
            }
        }
        let encoded = serde_json::to_vec(&json).unwrap();
        assert!(encoded.len() <= json_len);
        bytes[20..20 + json_len].fill(b' ');
        bytes[20..20 + encoded.len()].copy_from_slice(&encoded);
        let mut viewer = Viewer::new(
            Asset3D::from_slice(&bytes).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let capture = |v: &mut Viewer| {
            let scene = v.scene().unwrap();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &scene,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    ColorRole::DocumentCanvas.resolve(v.theme),
                )
                .unwrap()
        };
        let first = capture(&mut viewer);
        let (upload, geometry, allocations) = {
            let gpu = viewer.gpu.as_ref().unwrap();
            (
                gpu.instance_upload_bytes(),
                gpu.geometry_resident_bytes(),
                gpu.vertex_buffer_allocations(),
            )
        };
        assert!(upload > 0);
        // Isolate each feature by disabling all other dynamic batches.
        let mut delta = std::collections::BTreeMap::new();
        for feature in [
            "car",
            "car-police",
            "car-ambulance",
            "car-fire",
            "car-garbage",
            "activity-jogging",
            "activity-listening",
            "activity-strolling",
            "person",
            "tree",
            "water",
            "construction-frame",
            "construction-facade",
            "construction-scaffold",
            "construction-dust-screen",
            "construction-hoarding",
            "crane-boom",
            "boat",
            "boat-wake",
            "crane-retirement",
        ] {
            let district = viewer.district.as_ref().unwrap();
            district
                .configure(viewer.gpu.as_mut().unwrap(), &viewer.view)
                .unwrap();
            let mut config = crate::district::instance_recipe_for_test(district);
            for batch in &mut config.batches {
                let selected =
                    if feature.starts_with("activity-") && batch.node.starts_with("person-") {
                        let activity = match feature {
                            "activity-jogging" => tekes_ui_city::Activity::Jogging,
                            "activity-listening" => tekes_ui_city::Activity::Listening,
                            _ => tekes_ui_city::Activity::Strolling,
                        };
                        batch.instances.retain(|i| {
                            i.route.is_some_and(|r| {
                                district.config.pedestrians.iter().any(|a| {
                                    a.activity == activity
                                        && a.route == r.route
                                        && a.distance == r.distance
                                        && a.phase == r.phase
                                })
                            })
                        });
                        true
                    } else if feature == "car" {
                        batch.node.starts_with("car") || batch.node.starts_with("road-wheel-")
                    } else if feature == "crane-retirement" {
                        batch.node.starts_with("crane-")
                    } else if feature == "person" {
                        batch.node.starts_with("person-")
                    } else if feature == "tree" {
                        batch.node.starts_with("tree-")
                    } else if feature == "water" {
                        batch.node == "water" || batch.node == "pond"
                    } else {
                        batch.node == feature
                    };
                if batch.node == "construction-dust-screen" && feature != "construction-dust-screen"
                {
                    batch.instances.clear();
                }
                if feature == "construction-frame" && batch.node == "construction-facade" {
                    batch.instances.clear();
                }
                if batch.node == "sky-cloud" && feature != "sky-cloud" {
                    batch.instances.clear();
                }
                if !selected {
                    for instance in &mut batch.instances {
                        instance.removal = None;
                        instance.route = None;
                        instance.route_schedule = None;
                        instance.rolling = None;
                        instance.deformation = tekes_ui_scene3d::Deformation3D::None;
                    }
                }
            }
            viewer
                .gpu
                .as_mut()
                .unwrap()
                .configure_instances(&viewer.view, &config)
                .unwrap();
            viewer.district.as_mut().unwrap().clock.seek(0.).unwrap();
            let a = capture(&mut viewer);
            viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .seek(if feature == "construction-hoarding" {
                    65.
                } else if feature.starts_with("car-") {
                    4.5
                } else if feature.starts_with("construction-") {
                    20.
                } else {
                    1.2
                })
                .unwrap();
            let b = capture(&mut viewer);
            let mut changed = a
                .chunks_exact(4)
                .zip(b.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            // A service vehicle can begin behind a building. Observe its whole route,
            // while every other instance remains frozen, rather than mistaking occlusion for no motion.
            if feature.starts_with("car-") {
                for seconds in [10., 20., 30., 40.] {
                    if changed > 10 {
                        break;
                    }
                    viewer
                        .district
                        .as_mut()
                        .unwrap()
                        .clock
                        .seek(seconds)
                        .unwrap();
                    let frame = capture(&mut viewer);
                    changed = a
                        .chunks_exact(4)
                        .zip(frame.chunks_exact(4))
                        .filter(|(a, b)| a != b)
                        .count();
                }
            }
            assert!(
                changed > 10,
                "feature must change actual pixels: {feature} ({changed})"
            );
            if feature == "construction-frame" {
                viewer.district.as_mut().unwrap().clock.seek(120.).unwrap();
                let completed = capture(&mut viewer);
                viewer.district.as_mut().unwrap().clock.seek(240.).unwrap();
                assert!(
                    completed == capture(&mut viewer),
                    "completed building must stop growing"
                );
            }
            if feature == "crane-retirement" {
                viewer.district.as_mut().unwrap().clock.seek(66.).unwrap();
                let removing = capture(&mut viewer);
                viewer.district.as_mut().unwrap().clock.seek(70.).unwrap();
                let removed = capture(&mut viewer);
                assert!(removing != removed, "crane must retire progressively");
                for batch in &mut config.batches {
                    if batch.node.starts_with("crane-") {
                        batch.instances.clear();
                    }
                }
                viewer
                    .gpu
                    .as_mut()
                    .unwrap()
                    .configure_instances(&viewer.view, &config)
                    .unwrap();
                assert!(
                    removed == capture(&mut viewer),
                    "retired crane must leave no pixels"
                );
            }
            if matches!(
                feature,
                "construction-scaffold" | "construction-hoarding" | "construction-dust-screen"
            ) {
                viewer.district.as_mut().unwrap().clock.seek(55.).unwrap();
                let removing = capture(&mut viewer);
                viewer.district.as_mut().unwrap().clock.seek(80.).unwrap();
                let removed = capture(&mut viewer);
                assert!(
                    removing != removed,
                    "temporary site structures must dismantle after completion"
                );
                config
                    .batches
                    .iter_mut()
                    .find(|b| b.node == feature)
                    .unwrap()
                    .instances
                    .clear();
                viewer
                    .gpu
                    .as_mut()
                    .unwrap()
                    .configure_instances(&viewer.view, &config)
                    .unwrap();
                assert!(
                    removed == capture(&mut viewer),
                    "finished removal must leave no temporary site pixels"
                );
            }
            delta.insert(feature, changed);
        }
        viewer
            .district
            .as_ref()
            .unwrap()
            .configure(viewer.gpu.as_mut().unwrap(), &viewer.view)
            .unwrap();
        viewer.district.as_mut().unwrap().clock.seek(0.).unwrap();
        assert!(
            first == capture(&mut viewer),
            "reset must reproduce the original scene"
        );
        let bytes = viewer.gpu.as_ref().unwrap().instance_upload_bytes();
        for i in 1..=120 {
            viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .seek(f64::from(i) / 60.)
                .unwrap();
            viewer.scene().unwrap();
            let gpu = viewer.gpu.as_ref().unwrap();
            assert_eq!(gpu.instance_upload_bytes(), bytes);
            assert_eq!(gpu.geometry_resident_bytes(), geometry);
            assert_eq!(gpu.vertex_buffer_allocations(), allocations);
            assert_eq!(gpu.last_frame_pose_upload_bytes(), 0);
            assert_eq!(gpu.last_frame_deformed_meshes(), 0);
        }
        assert!(first != capture(&mut viewer), "animated scene must differ");
        viewer.command(9).unwrap();
        assert!(!viewer.playing);
        let frozen = capture(&mut viewer);
        assert!(
            !viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .advance(10., true)
                .unwrap()
        );
        assert!(
            frozen == capture(&mut viewer),
            "paused scene must be unchanged"
        );
        viewer.command(9).unwrap();
        let enabled_frozen = capture(&mut viewer);
        assert!(
            !viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .advance(10., false)
                .unwrap()
        );
        assert!(
            enabled_frozen == capture(&mut viewer),
            "reduced/hidden motion must freeze scene"
        );
        viewer.command(1).unwrap();
        assert!(
            first == capture(&mut viewer),
            "reset must reproduce the original scene"
        );
        let image = viewer.image;
        let count = viewer.ui.as_ref().unwrap().image_count();
        for command in [11, 12, 13, 14] {
            viewer.command(command).unwrap();
            viewer.scene().unwrap();
            assert_eq!(viewer.image, image);
            assert_eq!(viewer.ui.as_ref().unwrap().image_count(), count);
        }
        eprintln!(
            "CITY_BLOCK_GPU_CHECK: feature_changed_pixels={delta:?}, geometry_bytes={geometry}, instance_route_upload_bytes={bytes}; 120 frames: vertex/pose/instance uploads=0"
        );
    }
    #[test]
    fn district_day_night_changes_light_without_rebuilding_geometry_or_instances() {
        let mut viewer = Viewer::new(
            Asset3D::load(demo_asset("city-block")).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let capture = |v: &mut Viewer| {
            let scene = v.scene().unwrap();
            let clear = v.clear_color();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(&scene, &mut v.atlas, (960, 720), 1., clear)
                .unwrap()
        };
        let day = capture(&mut viewer);
        let gpu = viewer.gpu.as_ref().unwrap();
        let before = (
            gpu.geometry_upload_bytes(),
            gpu.geometry_resident_bytes(),
            gpu.instance_upload_bytes(),
            gpu.draw_bundle_builds(),
        );
        let uniforms = gpu.environment_upload_bytes();
        viewer.command(16).unwrap();
        let night = capture(&mut viewer);
        assert!(viewer.night);
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(
            before,
            (
                gpu.geometry_upload_bytes(),
                gpu.geometry_resident_bytes(),
                gpu.instance_upload_bytes(),
                gpu.draw_bundle_builds()
            )
        );
        assert_eq!(gpu.environment_upload_bytes() - uniforms, 48);
        let lit = day
            .chunks_exact(4)
            .zip(night.chunks_exact(4))
            .filter(|(d, n)| n[0] as i32 > d[0] as i32 + 24 && n[0] as i32 > n[2] as i32 + 12)
            .count();
        assert!(
            lit > 100,
            "night must have real emissive windows, pixels={lit}"
        );
        let luminance = |data: &[u8]| {
            data.chunks_exact(4)
                .map(|p| p[0] as u64 + p[1] as u64 + p[2] as u64)
                .sum::<u64>()
        };
        assert!(luminance(&night) < luminance(&day));
        let count = gpu.environment_upload_bytes();
        capture(&mut viewer);
        assert_eq!(
            viewer.gpu.as_ref().unwrap().environment_upload_bytes(),
            count
        );
        viewer.command(17).unwrap();
        assert_eq!(capture(&mut viewer), day);
        let gpu = viewer.gpu.as_mut().unwrap();
        assert!(
            gpu.set_lighting(Lighting3D {
                direction: [0.; 3],
                ..Default::default()
            })
            .is_err()
        );
    }
    #[test]
    fn completed_construction_joins_shared_window_life_without_reuploading() {
        use tekes_ui_scene3d::{Deformation3D, Environment3D, LocalLighting3D};
        let mut viewer = Viewer::new(
            Asset3D::load(demo_asset("city-block")).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let mut recipe =
            crate::district::instance_recipe_for_test(viewer.district.as_ref().unwrap());
        for batch in &mut recipe.batches {
            if batch.node == "construction-facade" {
                // Keep the facade fully visible to isolate occupancy activation
                // from construction visibility and retiring work lights.
                for i in &mut batch.instances {
                    i.deformation = Deformation3D::None;
                }
            } else {
                batch.instances.clear();
            }
        }
        let gpu = viewer.gpu.as_mut().unwrap();
        gpu.configure_instances(&viewer.view, &recipe).unwrap();
        gpu.configure_local_lighting(&LocalLighting3D::default())
            .unwrap();
        gpu.set_lighting(Lighting3D {
            ambient: [0.065, 0.09, 0.15],
            directional: [0.08, 0.10, 0.17],
            ..Default::default()
        })
        .unwrap();
        gpu.set_environment(Environment3D {
            time_seconds: 10.,
            ..Default::default()
        })
        .unwrap();
        let capture = |v: &mut Viewer, t: f32| {
            let gpu = v.gpu.as_mut().unwrap();
            gpu.set_motion_clocks([t, 0., 0., 0.]).unwrap();
            gpu.render_camera(&v.view, &v.camera).unwrap();
            let mut scene = Scene::new();
            v.view
                .paint(&mut scene, Rect::new(0., 0., 960., 720.), v.image.unwrap());
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &scene,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    Rgba::new(0., 0., 0., 1.),
                )
                .unwrap()
        };
        let empty = capture(&mut viewer, 30.);
        assert_eq!(
            empty,
            capture(&mut viewer, 40.),
            "unfinished facade must stay unoccupied and unlit"
        );
        let occupied = capture(&mut viewer, 80.);
        let lit = empty
            .chunks_exact(4)
            .zip(occupied.chunks_exact(4))
            .filter(|(a, b)| b[0] as i32 > a[0] as i32 + 15 && b[1] as i32 > a[1] as i32 + 8)
            .count();
        assert!(
            lit > 30,
            "completed facade must acquire shared interior lighting, pixels={lit}"
        );
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(
            gpu.interior_style_evaluations(),
            32,
            "handover reuses the existing 32 styles"
        );
        let bytes = (
            gpu.geometry_upload_bytes(),
            gpu.instance_upload_bytes(),
            gpu.draw_bundle_builds(),
        );
        assert_eq!(occupied, capture(&mut viewer, 100.));
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(
            bytes,
            (
                gpu.geometry_upload_bytes(),
                gpu.instance_upload_bytes(),
                gpu.draw_bundle_builds()
            )
        );
    }
    #[test]
    fn construction_holds_built_height_across_night_and_resumes_without_rewinding() {
        let mut viewer = Viewer::new(
            Asset3D::load(demo_asset("city-block")).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let mut recipe =
            crate::district::instance_recipe_for_test(viewer.district.as_ref().unwrap());
        for batch in &mut recipe.batches {
            if !batch.node.starts_with("construction-") && !batch.node.starts_with("crane-") {
                batch.instances.clear();
            }
        }
        viewer
            .gpu
            .as_mut()
            .unwrap()
            .configure_instances(&viewer.view, &recipe)
            .unwrap();
        // Hold lighting and other clocks fixed to compare construction geometry,
        // rather than the intentionally different day/night surface colors.
        let capture = |v: &mut Viewer| {
            let clocks = v.district.as_ref().unwrap().motion_clocks();
            let gpu = v.gpu.as_mut().unwrap();
            gpu.set_lighting(Lighting3D {
                emissive_strength: 0.,
                ..Default::default()
            })
            .unwrap();
            gpu.set_motion_clocks(clocks).unwrap();
            gpu.set_environment(Default::default()).unwrap();
            gpu.render_camera(&v.view, &v.camera).unwrap();
            let mut scene = Scene::new();
            v.view
                .paint(&mut scene, Rect::new(0., 0., 960., 720.), v.image.unwrap());
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &scene,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    Rgba::new(0., 0., 0., 1.),
                )
                .unwrap()
        };
        for start in [18., 39., 57., 70.] {
            viewer.command(15).unwrap();
            viewer.command(1).unwrap();
            viewer.district.as_mut().unwrap().clock.seek(start).unwrap();
            let built = capture(&mut viewer);
            viewer.command(16).unwrap();
            assert_eq!(
                built,
                capture(&mut viewer),
                "night entry must preserve construction at {start}"
            );
            viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .seek(start + 300.)
                .unwrap();
            viewer.command(16).unwrap(); // Repeated Night selection must not restart it.
            assert_eq!(
                built,
                capture(&mut viewer),
                "whole night must hold construction at {start}"
            );
            viewer.command(15).unwrap();
            assert_eq!(
                built,
                capture(&mut viewer),
                "day must resume at the same height"
            );
            viewer
                .district
                .as_mut()
                .unwrap()
                .clock
                .seek(start + 315.)
                .unwrap();
            if start < 70. {
                assert_ne!(
                    built,
                    capture(&mut viewer),
                    "day must resume building/dismantling"
                );
            } else {
                assert_eq!(
                    built,
                    capture(&mut viewer),
                    "retired site structures must stay removed"
                );
            }
        }
    }
    #[test]
    fn night_interiors_sleep_and_local_lights_cast_shadows_without_frame_uploads() {
        use tekes_ui_scene3d::{Environment3D, LocalLighting3D};
        let mut viewer = Viewer::new(
            Asset3D::load(demo_asset("city-block")).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        // At this physical resolution, room actors require a close camera.
        viewer.camera.zoom_by(0.35).unwrap();
        viewer.command(16).unwrap();
        let district = viewer.district.as_ref().unwrap();
        let mut recipe = crate::district::instance_recipe_for_test(district);
        for batch in &mut recipe.batches {
            batch.instances.clear();
        }
        let lighting = viewer.lighting();
        let gpu = viewer.gpu.as_mut().unwrap();
        gpu.configure_instances(&viewer.view, &recipe).unwrap();
        gpu.configure_local_lighting(&LocalLighting3D::default())
            .unwrap();
        gpu.set_lighting(lighting).unwrap();
        let capture = |v: &mut Viewer, time: f32| {
            let gpu = v.gpu.as_mut().unwrap();
            gpu.set_motion_clocks([0., time, 0., 0.]).unwrap();
            gpu.set_environment(Environment3D {
                time_seconds: time,
                ..Default::default()
            })
            .unwrap();
            gpu.render_camera(&v.view, &v.camera).unwrap();
            let scene = v.ui_scene().unwrap();
            let clear = v.clear_color();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(&scene, &mut v.atlas, (960, 720), 1., clear)
                .unwrap()
        };
        let evening = capture(&mut viewer, 10.);
        let walking = capture(&mut viewer, 18.);
        let midnight = capture(&mut viewer, 300.);
        let bright = |p: &[u8]| {
            p.chunks_exact(4)
                .filter(|c| c[0] > 140 && c[1] > 100 && c[0] as i32 > c[2] as i32 + 15)
                .count()
        };
        let a = bright(&evening);
        let b = bright(&midnight);
        assert!(
            a > 500 && b > 20 && b * 2 < a,
            "lit windows must decline but night owls remain: {a} -> {b}"
        );
        assert!(
            evening
                .chunks_exact(4)
                .zip(walking.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
                > 50,
            "window silhouettes should move with frozen exterior actors"
        );
        let config = DistrictDemo::load(&demo_asset("city-block").with_extension("json")).unwrap();
        config
            .configure(viewer.gpu.as_mut().unwrap(), &viewer.view)
            .unwrap();
        viewer.scene().unwrap();
        let gpu = viewer.gpu.as_ref().unwrap();
        let baseline = (
            gpu.geometry_upload_bytes(),
            gpu.instance_upload_bytes(),
            gpu.draw_bundle_builds(),
        );
        let with_shadows = capture(&mut viewer, 12.);
        let mut lights = crate::district::lighting_recipe_for_test(&config);
        lights.shadows.clear();
        viewer
            .gpu
            .as_mut()
            .unwrap()
            .configure_local_lighting(&lights)
            .unwrap();
        let without_shadows = capture(&mut viewer, 12.);
        let shadow_pixels = with_shadows
            .chunks_exact(4)
            .zip(without_shadows.chunks_exact(4))
            .filter(|(a, b)| b[0] as i32 > a[0] as i32 + 3)
            .count();
        assert!(
            shadow_pixels > 40,
            "local lights must have real receiver occlusion, pixels={shadow_pixels}"
        );
        viewer
            .gpu
            .as_mut()
            .unwrap()
            .configure_local_lighting(&LocalLighting3D::default())
            .unwrap();
        let no_lights = capture(&mut viewer, 12.);
        assert!(
            without_shadows
                .chunks_exact(4)
                .zip(no_lights.chunks_exact(4))
                .filter(|(a, b)| a[0] as i32 > b[0] as i32 + 10)
                .count()
                > 500,
            "street and head lights must illuminate surfaces"
        );
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(
            baseline,
            (
                gpu.geometry_upload_bytes(),
                gpu.instance_upload_bytes(),
                gpu.draw_bundle_builds()
            )
        );
        eprintln!(
            "NIGHT_GPU_CHECK: lit pixels evening={a}, midnight={b}; shadow pixels={shadow_pixels}; geometry/instance uploads and bundles unchanged"
        );
    }
    #[test]
    fn gpu_departure_timeline_matches_removing_instances_and_reuses_geometry() {
        use tekes_ui_scene3d::Environment3D;
        let mut viewer = Viewer::new(
            Asset3D::load(demo_asset("city-block")).unwrap(),
            "city-block.glb".into(),
        )
        .unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let lighting = viewer.lighting();
        viewer.gpu.as_mut().unwrap().set_lighting(lighting).unwrap();
        let capture = |v: &mut Viewer, clock: f32| {
            let gpu = v.gpu.as_mut().unwrap();
            gpu.set_environment(Environment3D {
                time_seconds: 12.,
                ..Default::default()
            })
            .unwrap();
            gpu.set_motion_clocks([0., clock, 0., 0.]).unwrap();
            gpu.render_camera(&v.view, &v.camera).unwrap();
            let scene = v.ui_scene().unwrap();
            let clear = v.clear_color();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(&scene, &mut v.atlas, (960, 720), 1., clear)
                .unwrap()
        };
        let evening = capture(&mut viewer, 0.);
        let gpu = viewer.gpu.as_ref().unwrap();
        let before = (
            gpu.geometry_upload_bytes(),
            gpu.instance_upload_bytes(),
            gpu.draw_bundle_builds(),
        );
        let late = capture(&mut viewer, 300.);
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(
            before,
            (
                gpu.geometry_upload_bytes(),
                gpu.instance_upload_bytes(),
                gpu.draw_bundle_builds()
            )
        );
        assert!(
            evening
                .chunks_exact(4)
                .zip(late.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
                > 100
        );
        let mut recipe =
            crate::district::instance_recipe_for_test(viewer.district.as_ref().unwrap());
        for batch in &mut recipe.batches {
            batch.instances.retain(|i| i.visibility.is_none());
        }
        viewer
            .gpu
            .as_mut()
            .unwrap()
            .configure_instances(&viewer.view, &recipe)
            .unwrap();
        viewer
            .gpu
            .as_mut()
            .unwrap()
            .configure_screen_detail(&viewer.view, &crate::district::screen_detail(&recipe))
            .unwrap();
        assert_eq!(
            late,
            capture(&mut viewer, 300.),
            "GPU hidden actors must match omitted draws, leaving only late walkers"
        );
    }
    #[test]
    fn health_cabin_imports_glass_and_animates_hinges_on_gpu() {
        let asset = Asset3D::load(demo_asset("health-cabin")).unwrap();
        assert_eq!(asset.clips().len(), 1);
        let mut viewer = Viewer::new(asset, "health-cabin.glb".into()).unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let capture = |v: &mut Viewer| {
            let scene = v.scene().unwrap();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &scene,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    ColorRole::DocumentCanvas.resolve(v.theme),
                )
                .unwrap()
        };
        let closed = capture(&mut viewer);
        let allocations = viewer.gpu.as_ref().unwrap().vertex_buffer_allocations();
        for id in [2, 4, 6, 8] {
            viewer.command(id).unwrap();
            assert_ne!(closed, capture(&mut viewer));
            assert_eq!(
                viewer.gpu.as_ref().unwrap().last_frame_pose_upload_bytes(),
                0
            );
            assert_eq!(viewer.gpu.as_ref().unwrap().last_frame_deformed_meshes(), 0);
            viewer.command(1).unwrap();
            assert_eq!(closed, capture(&mut viewer));
        }
        viewer.view.seek(2.).unwrap();
        assert_ne!(closed, capture(&mut viewer));
        let dirty = viewer.gpu.as_ref().unwrap().last_frame_deformed_meshes();
        assert!(
            dirty > 0 && dirty < allocations / 2,
            "hinges must only deform the door/window primitives"
        );
        viewer.command(1).unwrap();
        assert_eq!(closed, capture(&mut viewer));
        viewer.command(9).unwrap();
        assert!(viewer.playing);
        viewer.view.reduced_motion = true;
        assert!(!viewer.view.advance(0.5).unwrap());
        let image = viewer.image;
        let count = viewer.ui.as_ref().unwrap().image_count();
        for command in [11, 12, 13] {
            viewer.command(command).unwrap();
            viewer.scene().unwrap();
            assert_eq!(viewer.image, image);
            assert_eq!(viewer.ui.as_ref().unwrap().image_count(), count);
        }
    }
    #[test]
    fn manhattan_demo_keeps_static_geometry_on_gpu_and_switches_without_image_leaks() {
        let asset = Asset3D::load(demo_asset("manhattan")).unwrap();
        assert_eq!(asset.clips().len(), 0);
        let mut viewer = Viewer::new(asset, "manhattan.glb".into()).unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        viewer.scene().unwrap();
        let image = viewer.image;
        let image_count = viewer.ui.as_ref().unwrap().image_count();
        let gpu = viewer.gpu.as_ref().unwrap();
        assert_eq!(gpu.vertex_buffer_allocations(), 4);
        let bytes = gpu.geometry_resident_bytes();
        assert!(bytes > 0);
        let capture = |v: &mut Viewer| {
            let scene = v.scene().unwrap();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &scene,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    ColorRole::DocumentCanvas.resolve(v.theme),
                )
                .unwrap()
        };
        let first = capture(&mut viewer);
        for id in [2, 4, 6, 8] {
            viewer.command(id).unwrap();
            assert_ne!(first, capture(&mut viewer));
            viewer.command(1).unwrap();
            assert_eq!(first, capture(&mut viewer));
        }
        for _ in 0..120 {
            viewer.camera.rotate_model(0.01, 0., 0.).unwrap();
            viewer.scene().unwrap();
            let gpu = viewer.gpu.as_ref().unwrap();
            assert_eq!(gpu.last_frame_pose_upload_bytes(), 0);
            assert_eq!(gpu.last_frame_deformed_meshes(), 0);
            assert_eq!(gpu.vertex_buffer_allocations(), 4);
            assert_eq!(gpu.geometry_resident_bytes(), bytes);
        }
        let original_pixels = viewer.pixels();
        for size in [(480, 320), original_pixels] {
            let gpu = viewer.gpu.as_mut().unwrap();
            gpu.resize(size).unwrap();
            viewer
                .ui
                .as_mut()
                .unwrap()
                .replace_gpu_image(image.unwrap(), gpu.output())
                .unwrap();
            viewer.scene().unwrap();
            assert_eq!(viewer.image, image);
            assert_eq!(
                viewer.gpu.as_ref().unwrap().last_frame_pose_upload_bytes(),
                0
            );
        }
        viewer.command(9).unwrap();
        assert!(!viewer.playing);
        for command in [11, 12, 11, 12] {
            viewer.command(command).unwrap();
            viewer.scene().unwrap();
            assert_eq!(viewer.image, image);
            assert_eq!(viewer.ui.as_ref().unwrap().image_count(), image_count);
        }
    }
    #[test]
    fn demo_imports_and_controls_render_on_gpu() {
        menus().validate().unwrap();
        let asset = Asset3D::load(default_asset()).unwrap();
        assert_eq!(asset.clips().len(), 1);
        let mut viewer = Viewer::new(asset, "cube".into()).unwrap();
        viewer
            .init(Renderer::new_headless().unwrap(), (960, 720), 1.)
            .unwrap();
        let capture = |v: &mut Viewer| {
            let s = v.scene().unwrap();
            v.ui.as_mut()
                .unwrap()
                .render_to_rgba8(
                    &s,
                    &mut v.atlas,
                    (960, 720),
                    1.,
                    ColorRole::DocumentCanvas.resolve(v.theme),
                )
                .unwrap()
        };
        let first = capture(&mut viewer);
        for id in [2, 4, 6, 8] {
            viewer.command(id).unwrap();
            assert_ne!(first, capture(&mut viewer));
            viewer.command(1).unwrap();
            assert_eq!(first, capture(&mut viewer));
        }
        assert_eq!(
            viewer.gpu.as_ref().unwrap().last_frame_pose_upload_bytes(),
            0
        );
        assert_eq!(viewer.gpu.as_ref().unwrap().last_frame_deformed_meshes(), 0);
        viewer.view.seek(1.5).unwrap();
        assert_ne!(first, capture(&mut viewer));
        assert_eq!(viewer.gpu.as_ref().unwrap().last_frame_deformed_meshes(), 9);
        assert_eq!(
            viewer.gpu.as_ref().unwrap().last_frame_pose_upload_bytes(),
            9 * 80
        );
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::AnimationCadence;
    #[test]
    fn auto_follows_native_callbacks_and_screen_refresh_changes() {
        let mut cadence = AnimationCadence::default();
        assert_eq!(cadence.hz, 60);
        cadence.set_hz(0);
        let mut timestamp = 0.;
        for hz in [120., 60., 90., 120.] {
            for _ in 0..hz as usize {
                timestamp += 1. / hz;
                assert!(cadence.due(timestamp));
            }
        }
        cadence.set_hz(60);
        assert!(cadence.due(timestamp));
        assert!(!cadence.due(timestamp + 1. / 120.));
        cadence.set_hz(0);
        assert!(cadence.due(timestamp + 1. / 120.));
    }
    #[test]
    fn fps_samples_drawables_and_resets_after_idle() {
        use super::FrameRate;
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let mut rate = FrameRate::default();
        assert!(!rate.refresh(start));
        rate.frames = 60;
        assert!(!rate.refresh(start + Duration::from_millis(499)));
        assert!(rate.refresh(start + Duration::from_millis(500)));
        assert_eq!(rate.fps, Some(120.));
        assert_eq!(rate.frames, 0);
        assert!(rate.label().contains("120.0"));
        assert!(rate.clear());
        assert_eq!(rate.fps, None);
        assert!(!rate.clear());
        rate.refresh(start);
        rate.frames = 1;
        rate.refresh(start + Duration::from_secs(10));
        assert_eq!(rate.fps, None, "idle gaps must not display a stale rate");
    }
    #[test]
    fn sixty_hz_on_high_refresh_and_no_half_rate_on_sixty_hz() {
        for budget in [60, 120] {
            for hz in [30., 60., 90., 120., 144.] {
                let mut cadence = AnimationCadence::default();
                cadence.set_hz(budget);
                let count = (0..hz as usize)
                    .filter(|i| cadence.due(*i as f64 / hz))
                    .count();
                assert_eq!(
                    count,
                    (hz as usize).min(budget as usize),
                    "host refresh {hz}, budget {budget}"
                );
            }
        }
    }
    #[test]
    fn jitter_missed_deadlines_and_interaction_reset_do_not_catch_up() {
        let mut cadence = AnimationCadence::default();
        cadence.set_hz(60);
        assert!(cadence.due(0.));
        assert!(!cadence.due(1. / 120.));
        assert!(cadence.due(1. / 60. - 1e-7));
        assert!(!cadence.due(1. / 40.));
        assert!(cadence.due(10.));
        assert!(!cadence.due(10. + 1. / 120.));
        cadence.reset();
        assert!(cadence.due(0.));
    }
}
