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
use crate::perf::{FrameCtx, Perf};

#[derive(Debug)]
pub struct Host {
    #[cfg(feature = "composition-review")]
    wake_acceptance: Option<crate::wake_acceptance::WakeAcceptance>,
    gallery: Gallery,
    scene: Scene,
    renderer: Option<Renderer>,
    /// `TEKES_LOG_EVENTS` 设了就把事件和重绘次数打到 stderr，用来核对空闲时不重绘。
    log: bool,
    /// `TEKES_LOG_EVENTS=all`：连没有处理的事件（鼠标、键盘、焦点……）也记，用来查空闲时是谁在唤醒进程。
    log_all: bool,
    start: Instant,
    redraws: u64,
    /// `TEKES_PERF` 开着时的性能 harness（见 `perf.rs`）。
    perf: Option<Perf>,
    pending_redraw: bool,
    last_animation_target: Option<f64>,
    window_active: bool,
    accessibility_events: Vec<serde_json::Value>,
}

impl Host {
    pub fn new(gallery: Gallery) -> Host {
        Host {
            #[cfg(feature = "composition-review")]
            wake_acceptance: crate::wake_acceptance::WakeAcceptance::from_env(),
            gallery,
            scene: Scene::new(),
            renderer: None,
            log: std::env::var("TEKES_LOG_EVENTS").is_ok()
                || std::env::var("TEKES_SELFTEST").is_ok(),
            log_all: std::env::var("TEKES_LOG_EVENTS").is_ok_and(|v| v == "all"),
            start: Instant::now(),
            redraws: 0,
            perf: None,
            pending_redraw: false,
            last_animation_target: None,
            window_active: true,
            accessibility_events: Vec::new(),
        }
    }

    /// 挂上性能 harness：窗口创建后它接管动画和渲染，直到写完报告。
    pub fn with_perf(mut self, perf: Option<Perf>) -> Host {
        self.perf = perf;
        self
    }

    fn log(&self, what: std::fmt::Arguments<'_>) {
        if self.log {
            eprintln!("[{:>8.3}] {what}", self.start.elapsed().as_secs_f64());
        }
    }

    /// 窗口标题与菜单勾选跟随 Gallery 状态。
    fn sync_chrome(&self, window: &Window) {
        window.set_title(&self.gallery.title());
        window.update_menu_titles(&self.gallery.menu_bar());
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

    fn sync_animation(&mut self, window: &Window, animate: bool) {
        // A performance run owns its display link, including its stopped cooldown.
        // System preference and mouse events must never restart or stop that link.
        if self.perf.is_some() {
            return;
        }
        let animate = animate && self.window_active;
        if !animate || !window.is_animating() {
            self.last_animation_target = None;
        }
        window.set_animating(animate);
    }

    fn sync_accessibility(&self, window: &Window) {
        window.set_accessibility_nodes(self.gallery.accessibility_nodes());
        if let Some(out) = std::env::var_os("TEKESUI_GALLERY_AX_OUT") {
            let nodes:Vec<_>=window.accessibility_nodes().into_iter().map(|n|serde_json::json!({
                "component":n.key.component,"part":n.key.part,"parent":n.parent.map(|p|serde_json::json!({"component":p.component,"part":p.part})),"role":format!("{:?}",n.role),"description":n.description,"expanded":n.expanded,"invalid":n.invalid,"label":n.label,"value":n.value,"enabled":n.enabled,"focused":n.focused,"selected":n.selected,"read_only":n.read_only,"selection":n.text_selection,"frame":{"x":n.frame.origin.x,"y":n.frame.origin.y,"width":n.frame.size.width,"height":n.frame.size.height}
            })).collect();
            let out = std::path::PathBuf::from(out);
            std::fs::create_dir_all(&out).expect("AX audit output");
            std::fs::write(out.join("native-tree.json"),serde_json::to_vec_pretty(&serde_json::json!({"nodes":nodes,"screen":format!("{:?}",window.screen_info()),"backing_scale":window.scale(),"host_window_active":self.window_active,"window_key":window.is_key_window(),"window_active":window.is_active(),"scope":"accepted native publication readback; VoiceOver acceptance separate","actions":self.accessibility_events})).unwrap()).expect("AX snapshot");
        }
    }

    fn redraw(&mut self, window: &Window) {
        self.redraws += 1;
        let size = window.size();
        self.gallery
            .set_view_size(size.width as f32, size.height as f32);
        let theme = self.gallery.theme();
        // 字形 atlas 归 Gallery（文字引擎）所有：prepare 记下倍率并备好静态文字页的参照图，
        // build_scene 里 begin_frame 并整个重编。
        if let Some(r) = &mut self.renderer {
            self.gallery.prepare(r, window.scale() as f32);
        }
        self.scene.clear();
        self.gallery.build_scene(&mut self.scene);
        window.set_cursor_regions(self.gallery.cursor_regions());
        window.set_text_input_state(self.gallery.text_input_state());
        self.sync_accessibility(window);
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
        if self.log && window.is_live_resizing() {
            let presented = window
                .presented_frames()
                .iter()
                .filter(|frame| frame.presented_at > 0.0)
                .count();
            self.log(format_args!(
                "live-resize rendered; positive presented={presented}"
            ));
        }
        // Components discover their transition targets while building the
        // scene. Start the display link after that build as well as after
        // input, otherwise a first hover/press can remain on its initial frame.
        self.sync_animation(window, self.gallery.needs_animation());
    }
}

impl WindowHandler for Host {
    fn handle(&mut self, event: Event, window: &mut Window) {
        #[cfg(feature = "composition-review")]
        if let Some(audit) = &mut self.wake_acceptance {
            audit.observe(&event, window);
        }
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
                if let (Some(p), Some(r)) = (self.perf.as_mut(), self.renderer.as_mut()) {
                    p.begin(window, r);
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
                self.gallery
                    .set_view_size(size.width as f32, size.height as f32);
                if let Some(perf) = &mut self.perf {
                    perf.resize(size.width as f32, size.height as f32);
                }
                self.log(format_args!(
                    "Resized {}x{} scale {scale} drawable {drawable_size:?}",
                    size.width, size.height
                ));
                if let Some(r) = &mut self.renderer {
                    r.resize(*drawable_size, *scale as f32);
                }
            }
            Event::Redraw if self.perf.is_some() => {
                if let (Some(perf), Some(renderer)) = (self.perf.as_mut(), self.renderer.as_mut()) {
                    perf.poll_presentation_drain(&mut FrameCtx {
                        scene: &mut self.scene,
                        renderer,
                        window: &mut *window,
                        theme: self.gallery.theme(),
                    });
                }
            }
            Event::Redraw if window.is_animating() && !window.is_live_resizing() => {
                self.pending_redraw = true;
            }
            Event::Redraw => self.redraw(window),
            Event::AnimationFrame(f) => {
                if let (Some(p), Some(r)) = (self.perf.as_mut(), self.renderer.as_mut()) {
                    let mut ctx = FrameCtx {
                        scene: &mut self.scene,
                        renderer: r,
                        window: &mut *window,
                        theme: self.gallery.theme(),
                    };
                    p.frame(*f, &mut ctx);
                } else {
                    let dt = self
                        .last_animation_target
                        .map_or(f.refresh_interval, |previous| {
                            (f.target_timestamp - previous).max(0.0)
                        });
                    self.last_animation_target = Some(f.target_timestamp);
                    let animated = self.gallery.animate(dt as f32);
                    let pending = std::mem::take(&mut self.pending_redraw);
                    if animated || pending {
                        self.redraw(window);
                    }
                    let animate = self.gallery.needs_animation();
                    if !animate {
                        self.last_animation_target = None;
                    }
                    self.sync_animation(window, animate);
                }
            }
            Event::SystemMotionChanged(reduced) => {
                if let Some(perf) = &mut self.perf {
                    perf.set_reduced_motion(*reduced);
                }
                let change = self.gallery.set_reduced_motion(*reduced);
                self.apply_change(change, window);
                self.sync_animation(window, self.gallery.needs_animation());
            }
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
                if let Some(perf) = &mut self.perf {
                    perf.set_system_theme(system);
                }
                self.log(format_args!("system theme {system:?}"));
                let change = self.gallery.set_system_theme(system);
                self.apply_change(change, window);
            }
            Event::Command(id) => {
                if let Some(command) = Command::from_id(*id) {
                    self.log(format_args!("command {command:?}"));
                    let change = self.gallery.apply(command);
                    self.apply_change(change, window);
                    window.set_text_input_state(self.gallery.text_input_state());
                    self.sync_animation(window, self.gallery.needs_animation());
                }
            }
            Event::CloseRequested if self.perf.as_ref().is_some_and(Perf::is_active) => {
                if let (Some(p), Some(r)) = (self.perf.as_mut(), self.renderer.as_mut()) {
                    p.on_close_requested(window, r);
                } else {
                    window.close();
                }
            }
            Event::Accessibility { target, action } => {
                let change = self.gallery.accessibility_action(*target, action);
                if std::env::var_os("TEKESUI_GALLERY_AX_OUT").is_some()
                    && self.accessibility_events.len() < 128
                {
                    self.accessibility_events.push(serde_json::json!({"component":target.component,"part":target.part,"action":format!("{action:?}"),"changed":change.redraw}));
                }
                window.set_text_input_state(self.gallery.text_input_state());
                self.apply_change(change, window);
                self.sync_accessibility(window);
                self.sync_animation(window, self.gallery.needs_animation());
            }
            Event::CloseRequested => {
                self.log(format_args!(
                    "CloseRequested -> close；Redraw 共 {} 次",
                    self.redraws
                ));
                window.close();
                if let Some(out) = std::env::var_os("TEKESUI_GALLERY_AX_OUT") {
                    std::fs::write(std::path::PathBuf::from(out).join("close-readback.json"),serde_json::to_vec_pretty(&serde_json::json!({"accepted_nodes_after_close":window.accessibility_nodes().len()})).unwrap()).expect("closed AX readback");
                }
            }
            Event::TextCommand(command)
                if matches!(command.as_str(), "copy:" | "paste:" | "cut:") =>
            {
                let clipboard = window.clipboard_text();
                let (change, copied) = self.gallery.text_command(command, clipboard.as_deref());
                if let Some(text) = copied {
                    window.set_clipboard_text(&text);
                }
                window.set_text_input_state(self.gallery.text_input_state());
                self.apply_change(change, window);
            }
            Event::Key(k)
                if k.kind == tekes_ui_platform_macos::KeyEventKind::Down
                    && k.modifiers.command
                    && matches!(k.key_code, 7..=9)
                    && self.gallery.text_input_state().is_some() =>
            {
                let command = match k.key_code {
                    7 => "cut:",
                    8 => "copy:",
                    _ => "paste:",
                };
                let clipboard = window.clipboard_text();
                let (change, copied) = self.gallery.text_command(command, clipboard.as_deref());
                if let Some(text) = copied {
                    window.set_clipboard_text(&text);
                }
                window.set_text_input_state(self.gallery.text_input_state());
                self.apply_change(change, window);
            }
            Event::Focus(active) => {
                self.window_active = *active;
                self.last_animation_target = None;
                let change = self.gallery.handle_input(&event);
                self.apply_change(change, window);
                window.set_text_input_state(self.gallery.text_input_state());
                window.set_cursor_regions(self.gallery.cursor_regions());
                // Focus changes must reach assistive clients even when an
                // occluded window cannot produce another rendered frame.
                self.sync_accessibility(window);
                self.sync_animation(window, self.gallery.needs_animation());
            }
            // handle_input 返回哪些状态变了；没变就什么都不做（不请求重绘）。
            Event::Mouse(_)
            | Event::Key(_)
            | Event::Scroll(_)
            | Event::TextCommit { .. }
            | Event::TextPreedit { .. }
            | Event::CaretBlink
            | Event::TextUnmark
            | Event::TextCommand(_) => {
                let hover = self.gallery.hover();
                let change = self.gallery.handle_input(&event);
                if self.gallery.hover() != hover {
                    self.log(format_args!("hover {:?}", self.gallery.hover()));
                }
                window.set_text_input_state(self.gallery.text_input_state());
                if matches!(event, Event::Scroll(_)) {
                    self.pending_redraw |= change.redraw;
                    self.sync_animation(
                        window,
                        self.gallery.needs_animation() || self.pending_redraw,
                    );
                    return;
                }
                let animate = self.gallery.needs_animation();
                {
                    self.apply_change(change, window);
                    if std::mem::take(&mut self.pending_redraw) {
                        window.request_redraw();
                    }
                }
                if !animate || !window.is_animating() {
                    self.last_animation_target = None;
                }
                self.sync_animation(window, animate);
            }
            _ => {}
        }
    }
}
