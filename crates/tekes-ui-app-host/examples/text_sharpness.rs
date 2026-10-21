//! Native, same-font comparison fixture against the original Swift renderer.
use tekes_ui_platform_macos::placement::{ScreenChoice, WindowPlacement};
use tekes_ui_platform_macos::{
    App, AppConfig, Event, MenuBar, Size, Window, WindowConfig, WindowHandler,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Scene};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    push_paragraph,
};
use tekes_ui_tokens::Rgba;
macro_rules! diag { ($($arg:tt)*) => {{ use std::io::Write; let mut f=std::fs::OpenOptions::new().create(true).append(true).open("/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI/target/text-window-compare/rust-runtime.log").unwrap(); writeln!(f,$($arg)*).unwrap(); }}; }
struct Probe {
    renderer: Option<Renderer>,
    fonts: CoreTextFonts,
    atlas: GlyphAtlas,
    targets: Vec<f64>,
    composed: bool,
    raw: bool,
}
impl Probe {
    fn draw(&mut self, w: &Window) {
        let r = self.renderer.as_mut().unwrap();
        let scale = w.scale() as f32;
        r.resize(w.drawable_size(), scale);
        self.atlas.begin_frame();
        let mut scene = Scene::new();
        let mut shaper = CoreTextShaper::new(self.fonts.clone());
        let mut raster = CoreTextRasterizer::new(self.fonts.clone());
        let samples = [
            "System UI 13 pt regular — Rust TekesUI",
            "English prose · A small observatory 0123456789",
            "中文长文 · 清晨，街角的面包店打开了木门。",
            "混排 Unicode café / café · 中文 English 123",
            "System UI 17 pt regular — Rust TekesUI",
            "English prose · A small observatory 0123456789",
            "中文长文 · 清晨，街角的面包店打开了木门。",
            "混排 Unicode café / café · 中文 English 123",
            "Menlo 13 pt regular · illl 111 mmm @# 012345",
            "Fractional origin · 中文 English 123",
        ];
        for (i, s) in samples.iter().enumerate() {
            let size = if (4..=7).contains(&i) { 17. } else { 13. };
            let style = if i == 8 {
                TextStyle::named("Menlo", size)
            } else {
                TextStyle::system(size)
            };
            let p = shaper.shape(s, &style, &ShapeOptions::default());
            let y = 24. + i as f32 * 48. + ((32. - p.height as f32) / 2.).max(0.).floor();
            push_paragraph(
                &mut scene,
                &mut self.atlas,
                &mut raster,
                &p,
                Point::new(if i == 9 { 24.25 } else { 24. }, y),
                scale,
                Rgba::new(0.12, 0.12, 0.12, 1.),
            );
            if self.targets.is_empty() {
                for run in &p.lines[0].runs {
                    diag!("FONT row={i} {:?}", self.fonts.info(run.font));
                }
            }
        }
        if self.composed {
            let context = r.gpu_context();
            let size = w.drawable_size();
            let texture = context.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("native text composition probe"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            r.render_to_texture(&scene, &mut self.atlas, &texture, scale, Rgba::gray(1.))
                .unwrap();
            let image = context.import_image(texture).unwrap();
            let id = r.register_gpu_image(&image).unwrap();
            let origin = Point::new(3.75, 0.);
            let rect = if self.raw {
                tekes_ui_scene::Rect::new(
                    origin.x,
                    origin.y,
                    size.0 as f32 / scale,
                    size.1 as f32 / scale,
                )
            } else {
                tekes_ui_module_runtime::panel_image_rect(origin, size, scale)
            };
            let mut dest = Scene::new();
            dest.push_image(tekes_ui_scene::ImageQuad {
                rect,
                image: id,
                opacity: 1.,
            });
            r.render(&dest, &mut self.atlas, Rgba::gray(1.)).unwrap();
            r.unregister_image(id);
        } else {
            r.render(&scene, &mut self.atlas, Rgba::gray(1.)).unwrap();
        }
    }
}
impl WindowHandler for Probe {
    fn handle(&mut self, event: Event, w: &mut Window) {
        match event {
            Event::Created => {
                diag!("SCREEN {:?} scale={}", w.screen_info(), w.scale());
                self.renderer = Some(
                    unsafe {
                        Renderer::new_for_metal_layer(
                            w.metal_layer(),
                            w.drawable_size(),
                            w.scale() as f32,
                        )
                    }
                    .unwrap(),
                );
                self.draw(w);
                w.set_animating(true);
            }
            Event::Redraw | Event::Resized { .. } if self.renderer.is_some() => self.draw(w),
            Event::AnimationFrame(f) => {
                self.targets.push(f.target_timestamp);
                self.draw(w);
                if self.targets.len() == 241 {
                    let mut gaps: Vec<_> = self
                        .targets
                        .windows(2)
                        .map(|v| (v[1] - v[0]) * 1000.)
                        .collect();
                    gaps.sort_by(f64::total_cmp);
                    diag!(
                        "DISPLAY_LINK samples=240 p50_ms={} p95_ms={}",
                        gaps[120],
                        gaps[228]
                    );
                    w.set_animating(false);
                }
            }
            Event::Key(k) if k.kind == tekes_ui_platform_macos::KeyEventKind::Down => {
                {
                    let key = k.characters.as_str();
                    match key {
                        "c" => {
                            self.composed = true;
                            self.raw = false;
                        }
                        "o" => {
                            self.composed = true;
                            self.raw = true;
                        }
                        "d" => {
                            self.composed = false;
                            self.raw = false;
                        }
                        _ => return,
                    }
                }
                w.set_title(if self.raw {
                    "Rust TekesUI · old fractional composition"
                } else if self.composed {
                    "Rust TekesUI · corrected composition"
                } else {
                    "Rust TekesUI · matched fonts"
                });
                self.draw(w);
            }
            Event::CloseRequested => w.close(),
            _ => {}
        }
    }
}
fn main() {
    App::new(AppConfig {
        name: Some("Rust Text Compare".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "Rust TekesUI · matched fonts".into(),
            size: Size {
                width: 620.,
                height: 570.,
            },
            placement: WindowPlacement::RightHalf(ScreenChoice::Builtin),
            ..Default::default()
        },
        MenuBar::default(),
        Probe {
            renderer: None,
            fonts: CoreTextFonts::new(),
            atlas: GlyphAtlas::default(),
            targets: Vec::new(),
            composed: false,
            raw: false,
        },
    );
}
