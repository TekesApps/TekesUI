//! 平台层与渲染层的接线：窗口事件 → Renderer，主题跟随系统外观。
//!
//! Gallery 的内容与状态不在这里，见 `gallery/`。
//! 这里只做三件事：把平台层事件交给 `Gallery`，按它返回的 `Change` 请求重绘、同步窗口标题与菜单勾选，
//! 以及把编好的场景交给渲染器。

use std::time::Instant;

use tekes_ui_platform_macos::{Event, Window, WindowHandler};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::{Appearance, Contrast, Theme};

use crate::gallery::{Change, Command, Gallery};

#[derive(Debug)]
pub struct Host {
    gallery: Gallery,
    scene: Scene,
    renderer: Option<Renderer>,
    /// `TEKES_LOG_EVENTS` 设了就把事件和重绘次数打到 stderr，用来核对空闲时不重绘。
    log: bool,
    /// `TEKES_LOG_EVENTS=all`：连没有处理的事件（鼠标、键盘、焦点……）也记，用来查空闲时是谁在唤醒进程。
    log_all: bool,
    start: Instant,
    redraws: u64,
}

impl Host {
    pub fn new(gallery: Gallery) -> Host {
        Host {
            gallery,
            scene: Scene::new(),
            renderer: None,
            log: std::env::var("TEKES_LOG_EVENTS").is_ok()
                || std::env::var("TEKES_SELFTEST").is_ok(),
            log_all: std::env::var("TEKES_LOG_EVENTS").is_ok_and(|v| v == "all"),
            start: Instant::now(),
            redraws: 0,
        }
    }

    fn log(&self, what: std::fmt::Arguments<'_>) {
        if self.log {
            eprintln!("[{:>8.3}] {what}", self.start.elapsed().as_secs_f64());
        }
    }

    /// 窗口标题与菜单勾选跟随 Gallery 状态。
    fn sync_chrome(&self, window: &Window) {
        window.set_title(&self.gallery.title());
        for c in Command::all() {
            window.set_command_checked(c.id(), self.gallery.is_checked(c));
        }
    }

    fn apply_change(&self, change: Change, window: &Window) {
        if change.chrome {
            self.sync_chrome(window);
            self.log(format_args!("{}", self.gallery.title()));
        }
        if change.redraw {
            window.request_redraw();
        }
    }

    fn redraw(&mut self, window: &Window) {
        self.redraws += 1;
        let size = window.size();
        let theme = self.gallery.theme();
        // 字形 atlas 归 Gallery（文字引擎）所有：prepare 记下倍率并备好静态文字页的参照图，
        // build_scene 里 begin_frame 并整个重编。
        if let Some(r) = &mut self.renderer {
            self.gallery.prepare(r, window.scale() as f32);
        }
        self.scene.clear();
        self.gallery.build_scene(&mut self.scene);
        let clear = Gallery::background(theme);
        self.log(format_args!(
            "Redraw #{} size {}x{} drawable {:?} quads {}",
            self.redraws,
            size.width,
            size.height,
            window.drawable_size(),
            self.scene.len()
        ));
        if let Some(r) = &mut self.renderer
            && let Err(e) = r.render(&self.scene, self.gallery.atlas_mut(), clear)
        {
            eprintln!("渲染失败：{e}");
        }
    }
}

impl WindowHandler for Host {
    fn handle(&mut self, event: Event, window: &mut Window) {
        if self.log_all
            && !matches!(
                event,
                Event::Created | Event::Resized { .. } | Event::Redraw
            )
        {
            self.log(format_args!("{event:?}"));
        }
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
                self.sync_chrome(window);
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
                let system = Theme {
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
                self.log(format_args!("system theme {system:?}"));
                let change = self.gallery.set_system_theme(system);
                self.apply_change(change, window);
            }
            Event::Command(id) => {
                if let Some(command) = Command::from_id(*id) {
                    self.log(format_args!("command {command:?}"));
                    let change = self.gallery.apply(command);
                    self.apply_change(change, window);
                }
            }
            Event::CloseRequested => {
                self.log(format_args!(
                    "CloseRequested -> close；Redraw 共 {} 次",
                    self.redraws
                ));
                window.close();
            }
            // handle_input 返回哪些状态变了；没变就什么都不做（不请求重绘）。
            Event::Mouse(_) | Event::Key(_) => {
                let hover = self.gallery.hover();
                let change = self.gallery.handle_input(&event);
                if self.gallery.hover() != hover {
                    self.log(format_args!("hover {:?}", self.gallery.hover()));
                }
                self.apply_change(change, window);
            }
            _ => {}
        }
    }
}
