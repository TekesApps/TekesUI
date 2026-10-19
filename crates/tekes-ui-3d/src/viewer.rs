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
fn default_asset() -> PathBuf {
    let bundled = std::env::current_exe().ok().and_then(|p| {
        p.parent()?
            .parent()
            .map(|p| p.join("Resources/rubiks-cube.glb"))
    });
    bundled
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/rubiks-cube.glb"))
}
pub fn run() -> Result<()> {
    let mut input = None;
    let mut snapshot = None;
    let mut auto_rotate = false;
    let mut play = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--snapshot" {
            snapshot = Some(PathBuf::from(
                args.next().ok_or("--snapshot requires a PNG path")?,
            ));
        } else if arg == "--auto-rotate" {
            auto_rotate = true;
        } else if arg == "--play" {
            play = true;
        } else if input.replace(PathBuf::from(arg)).is_some() {
            return Err("usage: tekes-ui-3d [model.glb|model.gltf] [--snapshot output.png] [--auto-rotate] [--play]".into());
        }
    }
    let path = input.unwrap_or_else(default_asset);
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
    if play {
        viewer.command(9)?;
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
                    item("Play / pause layer (Space)", 9),
                    item("Auto rotate (A)", 10),
                ],
            ),
        ],
    }
}
struct Viewer {
    view: Scene3D,
    camera: Camera3D,
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
        let fonts = CoreTextFonts::new();
        Ok(Self {
            view,
            camera: home_camera(),
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
            playing: false,
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
        let gpu = Renderer3D::new(&ui.gpu_context(), &self.view, self.pixels())?;
        self.image = Some(ui.register_gpu_image(gpu.output())?);
        self.gpu = Some(gpu);
        self.ui = Some(ui);
        Ok(())
    }
    fn scene(&mut self) -> Result<Scene> {
        self.gpu
            .as_mut()
            .ok_or("GPU not initialized")?
            .render_camera(&self.view, &self.camera)?;
        if let Some(scene) = &self.cached_scene {
            return Ok(scene.clone());
        }
        self.atlas.begin_frame();
        let mut scene = Scene::new();
        self.view
            .paint(&mut scene, self.stage(), self.image.unwrap());
        let labels = [
            ("TekesUI3D".to_owned(), 24., 20., 20.),
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
                    if self.playing {
                        "暂停顶层动画"
                    } else {
                        "播放顶层动画"
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
        let scene = self.scene()?;
        self.ui.as_mut().unwrap().render(
            &scene,
            &mut self.atlas,
            ColorRole::DocumentCanvas.resolve(self.theme),
        )?;
        Ok(())
    }
    fn command(&mut self, id: u32) -> Result<()> {
        match id {
            1 => {
                self.camera = home_camera();
                self.view.seek(0.)?;
            }
            2 => self.camera.zoom_by(0.85)?,
            3 => self.camera.zoom_by(1. / 0.85)?,
            4 => self.camera.orbit(-0.18, 0.)?,
            5 => self.camera.orbit(0.18, 0.)?,
            6 => self.camera.orbit(0., 0.15)?,
            7 => self.camera.orbit(0., -0.15)?,
            8 => self.camera.rotate_model(0.25, 0.15, 0.)?,
            9 => {
                self.cached_scene = None;
                self.playing = !self.playing;
                if self.playing {
                    self.view.play();
                } else {
                    self.view.pause();
                }
            }
            10 => self.spin = !self.spin,
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
            Event::Command(id) => self.command(id.0)?,
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
            && (self.view.needs_frame() || self.spin || self.drag.is_some());
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
