use crate::district::DistrictDemo;
use std::{collections::HashMap, path::PathBuf};
use tekes_ui_platform_macos::*;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, ImageId, Point as ScenePoint, Rect, Scene};
use tekes_ui_scene3d::{Asset3D, Camera3D, Renderer3D, Scene3D};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    push::push_paragraph,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{ColorRole, Theme};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
        Some("tekes-ui-city-block") => "city-block",
        _ => "cube",
    }
    .to_owned();
    let mut input = None;
    let mut snapshot = None;
    let mut auto_rotate = false;
    let mut play = false;
    let mut scene_time = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--demo" {
            demo = args
                .next()
                .ok_or("--demo requires cube, manhattan or city-block")?
                .to_string_lossy()
                .into_owned();
            if !["cube", "manhattan", "city-block"].contains(&demo.as_str()) {
                return Err("--demo requires cube, manhattan or city-block".into());
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
        } else if arg == "--auto-rotate" {
            auto_rotate = true;
        } else if arg == "--play" {
            play = true;
        } else if input.replace(PathBuf::from(arg)).is_some() {
            return Err("usage: tekes-ui-3d [model.glb|model.gltf] [--demo cube|manhattan|city-block] [--snapshot output.png] [--auto-rotate] [--play]".into());
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
        let bytes = viewer.ui.as_mut().unwrap().render_to_rgba8(
            &scene,
            &mut viewer.atlas,
            (960, 720),
            1.,
            ColorRole::DocumentCanvas.resolve(viewer.theme),
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
                    item("Rotate model", 8),
                ],
            ),
            Menu::new(
                "Animation",
                vec![
                    item("Play / pause animation (Space)", 9),
                    item("Auto rotate (A)", 10),
                ],
            ),
        ],
    }
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
    drag: Option<(MouseButton, Point, bool)>,
    active: bool,
    reduced: bool,
    playing: bool,
    spin: bool,
    last: Option<f64>,
    pending_redraw: bool,
    menu_state: Option<(bool, bool)>,
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
            reduced: false,
            playing,
            spin: false,
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
        let mut gpu = if ["manhattan.glb", "city-block.glb"].contains(&self.name.as_str()) {
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
    fn scene(&mut self) -> Result<Scene> {
        if let Some(district) = &self.district {
            self.gpu
                .as_mut()
                .ok_or("GPU not initialized")?
                .set_environment(district.environment())?;
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
                    "WATERFRONT · Living City Block".to_owned()
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
                "拖动：视角  ·  右键 / Shift 拖动：旋转模型  ·  滚轮 / + −：缩放".to_owned(),
                24.,
                self.size.1 - 62.,
                13.,
            ),
            (
                format!(
                    "方向键：视角  ·  R：复位  ·  Space：{}  ·  A：自动旋转",
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
        for (label, x, y, size) in labels {
            let paragraph = self.text.entry(label.clone()).or_insert_with(|| {
                self.shaper.shape(
                    &label,
                    &TextStyle::system(size),
                    &ShapeOptions {
                        width: Some((self.size.0 - 48.) as f64),
                        line_spacing: 0.,
                    },
                )
            });
            push_paragraph(
                &mut scene,
                &mut self.atlas,
                &mut self.raster,
                paragraph,
                ScenePoint::new(x, y),
                self.scale,
                ColorRole::PrimaryText.resolve(self.theme),
            );
        }
        self.cached_scene = Some(scene.clone());
        Ok(scene)
    }
    fn draw(&mut self) -> Result<()> {
        let scene = self.ui_scene()?;
        let gpu = self.gpu.as_mut().ok_or("GPU not initialized")?;
        let view = &self.view;
        let camera = &self.camera;
        let environment = self.district.as_ref().map(DistrictDemo::environment);
        self.ui.as_mut().unwrap().render_with_gpu(
            &scene,
            &mut self.atlas,
            ColorRole::DocumentCanvas.resolve(self.theme),
            |encoder| {
                if let Some(environment) = environment {
                    gpu.set_environment(environment)
                        .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))?;
                }
                gpu.encode_camera(view, camera, encoder)
                    .map_err(|e| tekes_ui_render::RenderError::Surface(e.to_string()))
            },
        )?;
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
                    district.clock.reset();
                }
            }
            2 => self.camera.zoom_by(0.85)?,
            3 => self.camera.zoom_by(1. / 0.85)?,
            4 => self.camera.orbit(-0.18, 0.)?,
            5 => self.camera.orbit(0.18, 0.)?,
            6 => self.camera.orbit(0., 0.15)?,
            7 => self.camera.orbit(0., -0.15)?,
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
            11 => self.select_demo("rubiks-cube")?,
            12 => self.select_demo("manhattan")?,
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
                if self.active && !self.reduced {
                    changed = self.view.advance(dt)?;
                    if let Some(district) = &mut self.district {
                        changed |= district.clock.advance(dt, true)?;
                    }
                    if self.spin {
                        self.camera.rotate_model(dt * 0.45, 0., 0.)?;
                        changed = true;
                    }
                }
                if changed || self.pending_redraw {
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
                    self.drag = Some((button, e.position, e.modifiers.shift))
                }
                MouseEventKind::Dragged { button } => {
                    if let Some((held, previous, rotate)) = self.drag
                        && held == button
                    {
                        let x = (e.position.x - previous.x) as f32 * 0.008;
                        let y = (e.position.y - previous.y) as f32 * 0.008;
                        if button == MouseButton::Right || rotate {
                            self.camera.rotate_model(x, y, 0.)?;
                        } else if button == MouseButton::Left {
                            self.camera.orbit(-x, y)?;
                        }
                        self.drag = Some((held, e.position, rotate));
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
                    if e.modifiers.shift {
                        self.camera.rotate_model(x, y, 0.)?;
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
        let animate = self.active
            && !self.reduced
            && (self.view.needs_frame()
                || self.district.as_ref().is_some_and(|d| d.clock.playing())
                || self.spin
                || self.drag.is_some());
        window.set_animating(animate);
        if !animate {
            self.last = None;
        }
        let menu_state = (self.playing, self.spin);
        if self.menu_state != Some(menu_state) {
            window.set_command_checked(CommandId(9), self.playing);
            window.set_command_checked(CommandId(10), self.spin);
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
        for name in ["rubiks-cube", "city-block"] {
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
            for time in [0., 1.2, 24., 46., 55., 80., 0.] {
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
                let environment = combined.district.as_ref().map(DistrictDemo::environment);
                let gpu = combined.gpu.as_mut().unwrap();
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
                            if let Some(environment) = environment {
                                gpu.set_environment(environment).unwrap();
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
            "person",
            "tree",
            "water",
            "construction-frame",
            "construction-facade",
            "construction-scaffold",
            "crane-boom",
        ] {
            let district = viewer.district.as_ref().unwrap();
            district
                .configure(viewer.gpu.as_mut().unwrap(), &viewer.view)
                .unwrap();
            let mut config = crate::district::instance_recipe_for_test(district);
            for batch in &mut config.batches {
                let selected = if feature == "person" {
                    batch.node.starts_with("person-")
                } else if feature == "tree" {
                    batch.node.starts_with("tree-")
                } else if feature == "water" {
                    batch.node == "water" || batch.node == "pond"
                } else {
                    batch.node == feature
                };
                if feature == "construction-frame" && batch.node == "construction-facade" {
                    batch.instances.clear();
                }
                if !selected {
                    for instance in &mut batch.instances {
                        instance.route = None;
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
                .seek(if feature.starts_with("construction-") {
                    20.
                } else {
                    1.2
                })
                .unwrap();
            let b = capture(&mut viewer);
            let changed = a
                .chunks_exact(4)
                .zip(b.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
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
            if feature == "construction-scaffold" {
                viewer.district.as_mut().unwrap().clock.seek(55.).unwrap();
                let removing = capture(&mut viewer);
                viewer.district.as_mut().unwrap().clock.seek(80.).unwrap();
                let removed = capture(&mut viewer);
                assert!(
                    removing != removed,
                    "scaffold must dismantle after completion"
                );
                config
                    .batches
                    .iter_mut()
                    .find(|b| b.node == "construction-scaffold")
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
                    "finished removal must leave no scaffold pixels"
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
        for command in [11, 12, 14] {
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
