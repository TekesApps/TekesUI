//! 平台层与渲染层的接线：窗口事件 → Renderer，主题跟随系统外观。
//!
//! Gallery 的内容不在这里，见 `gallery.rs`。

use std::time::Instant;

use tekes_ui_platform_macos::{Event, Window, WindowHandler};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Scene, Size};
use tekes_ui_tokens::{Appearance, Contrast, Theme};

use crate::gallery::Gallery;

pub struct Host {
    gallery: Gallery,
    scene: Scene,
    theme: Theme,
    renderer: Option<Renderer>,
    /// `TEKES_LOG_EVENTS` 设了就把事件和重绘次数打到 stderr，用来核对空闲时不重绘。
    log: bool,
    start: Instant,
    redraws: u64,
}

impl Host {
    pub fn new() -> Host {
        Host {
            gallery: Gallery::new(),
            scene: Scene::new(),
            theme: Theme::default(),
            renderer: None,
            log: std::env::var("TEKES_LOG_EVENTS").is_ok()
                || std::env::var("TEKES_SELFTEST").is_ok(),
            start: Instant::now(),
            redraws: 0,
        }
    }

    fn log(&self, what: std::fmt::Arguments<'_>) {
        if self.log {
            eprintln!("[{:>8.3}] {what}", self.start.elapsed().as_secs_f64());
        }
    }

    fn redraw(&mut self, window: &Window) {
        self.redraws += 1;
        let size = window.size();
        let size = Size::new(size.width as f32, size.height as f32);
        self.scene.clear();
        self.gallery.build_scene(size, self.theme, &mut self.scene);
        let clear = self.gallery.background(self.theme);
        self.log(format_args!(
            "Redraw #{} size {}x{} drawable {:?} quads {}",
            self.redraws,
            size.width,
            size.height,
            window.drawable_size(),
            self.scene.len()
        ));
        if let Some(r) = &mut self.renderer
            && let Err(e) = r.render(&self.scene, clear)
        {
            eprintln!("渲染失败：{e}");
        }
    }
}

impl WindowHandler for Host {
    fn handle(&mut self, event: Event, window: &mut Window) {
        match &event {
            Event::Created => {
                // SAFETY: 层由视图持有，窗口存活期间有效；渲染器随 Host 一起在窗口关闭后丢弃。
                // 在主线程创建（平台层保证）。
                let r = unsafe {
                    Renderer::new_for_metal_layer(
                        window.metal_layer(),
                        window.drawable_size(),
                        window.scale() as f32,
                    )
                };
                match r {
                    Ok(r) => self.renderer = Some(r),
                    Err(e) => eprintln!("创建渲染器失败：{e}"),
                }
                self.log(format_args!(
                    "Created: size {:?} scale {} drawable {:?} windowNumber {}",
                    window.size(),
                    window.scale(),
                    window.drawable_size(),
                    window.native_window_number()
                ));
                if let Some(r) = window.placement_report() {
                    self.log(format_args!(
                        "placement: {:?} builtin={} scale={} max_fps={}；{}",
                        r.screen.name, r.screen.builtin, r.screen.scale, r.screen.max_fps, r.reason
                    ));
                }
            }
            Event::Resized {
                size,
                scale,
                drawable_size,
            } => {
                self.log(format_args!(
                    "Resized {}x{} scale {scale} drawable {drawable_size:?}",
                    size.width, size.height
                ));
                if let Some(r) = &mut self.renderer {
                    r.resize(*drawable_size, *scale as f32);
                }
            }
            Event::Redraw => self.redraw(window),
            Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.theme = Theme {
                    appearance: if *dark {
                        Appearance::Dark
                    } else {
                        Appearance::Light
                    },
                    contrast: if *increase_contrast {
                        Contrast::Increased
                    } else {
                        Contrast::Standard
                    },
                };
                self.log(format_args!("theme {:?}", self.theme));
                window.request_redraw();
            }
            Event::CloseRequested => {
                self.log(format_args!(
                    "CloseRequested -> close；Redraw 共 {} 次",
                    self.redraws
                ));
                window.close();
            }
            // handle_input 返回是否需要重绘。
            Event::Mouse(_) | Event::Scroll(_) | Event::Key(_)
                if self.gallery.handle_input(&event) =>
            {
                window.request_redraw();
            }
            _ => {}
        }
    }
}
