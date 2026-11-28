//! Native window selftest and performance harness running on the default AppHost Gallery.
use super::apphost_workbench::{
    SEARCH_ROUTE, SharedWorkbench, assemble_for_route_with_state, resolve_route,
};
use super::theme::{AppearanceChoice, ContrastChoice, ThemeState};
use super::workbench;
use super::{CONTENT_HEIGHT, CONTENT_WIDTH};
use tekes_ui_app_host::HostedWindow;
use tekes_ui_platform_macos::selftest;
use tekes_ui_platform_macos::{
    App, AppConfig, CommandId, Event, Menu, MenuBar, MenuCommand, MenuItem, Size, Window,
    WindowConfig, WindowHandler, WindowPlacement,
};
use tekes_ui_scene::Scene;
use tekes_ui_tokens::{Appearance, Contrast, Theme};

const CMD_APPEARANCE_SYSTEM: u32 = 200;
const CMD_APPEARANCE_LIGHT: u32 = 201;
const CMD_APPEARANCE_DARK: u32 = 202;
const CMD_CONTRAST_SYSTEM: u32 = 300;
const CMD_CONTRAST_ON: u32 = 301;
const CMD_CONTRAST_OFF: u32 = 302;
const CMD_SCALE_100: u32 = 540;
const CMD_SCALE_125: u32 = 541;
const CMD_SCALE_150: u32 = 542;
const CMD_ROUTE_BASE: u32 = 1000;

pub(crate) fn run_harness() -> anyhow::Result<()> {
    let initial_route =
        std::env::var("TEKESUI_GALLERY_PAGE").unwrap_or_else(|_| SEARCH_ROUTE.to_owned());
    let mode = std::env::var("TEKES_SELFTEST").ok();
    let perf_config = crate::perf::PerfConfig::from_env();
    let (need_120hz, size_check) = (perf_config.is_some(), mode.as_deref() == Some("1"));
    let (app, state, session_path) = assemble_for_route_with_state(&initial_route)?;
    let _session_dir = super::apphost::SessionTempDir::new(session_path);
    let (mut menu_bar, hosted) = app.into_window_handler()?;
    append_diagnostic_menus(&mut menu_bar)?;
    if let Some(mode) = mode.as_deref() {
        selftest::run_script(crate::selftest_scripts::script_for_mode(mode));
    }
    let window_config = WindowConfig {
        title: "TekesUI Gallery".into(),
        unified_titlebar: true,
        size: Size {
            width: f64::from(CONTENT_WIDTH),
            height: f64::from(CONTENT_HEIGHT),
        },
        min_size: Some(Size {
            width: 720.0,
            height: 480.0,
        }),
        placement: WindowPlacement::Auto {
            need_120hz,
            size_check,
        },
        frame_autosave_name: None,
    };
    let perf = perf_config.map(crate::perf::Perf::new);
    let has_perf = perf.is_some();
    let handler = AppHostHarnessHandler {
        hosted,
        state,
        perf,
        scene: Scene::new(),
        theme_state: ThemeState::default(),
    };
    App::new(AppConfig {
        name: Some("TekesUI Gallery".into()),
        quit_after_last_window_closed: true,
    })
    .run(window_config, menu_bar, handler);
    if has_perf {
        std::process::exit(crate::perf::exit_code());
    }
    Ok(())
}

fn append_diagnostic_menus(menu_bar: &mut MenuBar) -> anyhow::Result<()> {
    let view_items = vec![
        MenuItem::Command(MenuCommand::new(
            "Appearance: Follow System",
            CommandId(CMD_APPEARANCE_SYSTEM),
        )),
        MenuItem::Command(MenuCommand::new(
            "Appearance: Light",
            CommandId(CMD_APPEARANCE_LIGHT),
        )),
        MenuItem::Command(MenuCommand::new(
            "Appearance: Dark",
            CommandId(CMD_APPEARANCE_DARK),
        )),
        MenuItem::Separator,
        MenuItem::Command(MenuCommand::new(
            "Contrast: Follow System",
            CommandId(CMD_CONTRAST_SYSTEM),
        )),
        MenuItem::Command(MenuCommand::new(
            "Contrast: Increase",
            CommandId(CMD_CONTRAST_ON),
        )),
        MenuItem::Command(MenuCommand::new(
            "Contrast: Standard",
            CommandId(CMD_CONTRAST_OFF),
        )),
        MenuItem::Separator,
        MenuItem::Command(MenuCommand::new(
            "Text Size: 100%",
            CommandId(CMD_SCALE_100),
        )),
        MenuItem::Command(MenuCommand::new(
            "Text Size: 125%",
            CommandId(CMD_SCALE_125),
        )),
        MenuItem::Command(MenuCommand::new(
            "Text Size: 150%",
            CommandId(CMD_SCALE_150),
        )),
    ];
    menu_bar
        .menus
        .push(Menu::new("Diagnostics View", view_items));

    let route_items = workbench::entries()
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            MenuItem::Command(MenuCommand::new(
                entry.title(),
                CommandId(CMD_ROUTE_BASE + i as u32),
            ))
        })
        .collect();
    menu_bar
        .menus
        .push(Menu::new("Diagnostics Routes", route_items));
    menu_bar.validate()?;
    Ok(())
}

struct AppHostHarnessHandler {
    hosted: HostedWindow,
    state: SharedWorkbench,
    perf: Option<crate::perf::Perf>,
    scene: Scene,
    theme_state: ThemeState,
}

impl AppHostHarnessHandler {
    fn sync_effective_theme(&mut self) {
        let effective = self.theme_state.effective();
        self.hosted.set_theme(effective);
        let current = self.state.borrow().palette.get().with_theme(effective);
        self.state.borrow().palette.set(current);
        if let Some(perf) = &mut self.perf {
            perf.set_system_theme(effective);
        }
    }

    fn sync_perf_animation(&self, window: &mut Window) {
        if let Some(perf) = &self.perf {
            let _ = window.set_animating(perf.wants_animation());
        }
    }

    fn handle_diagnostic_command(&mut self, id: u32, window: &mut Window) -> bool {
        match id {
            CMD_APPEARANCE_SYSTEM => self.theme_state.appearance = AppearanceChoice::System,
            CMD_APPEARANCE_LIGHT => self.theme_state.appearance = AppearanceChoice::Light,
            CMD_APPEARANCE_DARK => self.theme_state.appearance = AppearanceChoice::Dark,
            CMD_CONTRAST_SYSTEM => self.theme_state.contrast = ContrastChoice::System,
            CMD_CONTRAST_ON => self.theme_state.contrast = ContrastChoice::On,
            CMD_CONTRAST_OFF => self.theme_state.contrast = ContrastChoice::Off,
            CMD_SCALE_100 => self.hosted.set_type_scale(1.0),
            CMD_SCALE_125 => self.hosted.set_type_scale(1.25),
            CMD_SCALE_150 => self.hosted.set_type_scale(1.5),
            _ if id >= CMD_ROUTE_BASE => {
                let index = (id - CMD_ROUTE_BASE) as usize;
                let Some(entry) = workbench::entries().get(index) else {
                    return false;
                };
                if let Ok((canonical, _)) = resolve_route(entry.slug()) {
                    let _ = self.state.borrow_mut().switch_to_route(&canonical);
                }
                window.request_redraw();
                return true;
            }
            _ => return false,
        }
        self.sync_effective_theme();
        window.request_redraw();
        true
    }
}

impl WindowHandler for AppHostHarnessHandler {
    fn allows_app_quit(&self) -> bool {
        self.hosted.allows_app_quit()
    }

    fn close_menu_title(&self) -> Option<&'static str> {
        self.hosted.close_menu_title()
    }

    fn handle_close_shortcut(&mut self, window: &mut Window) -> bool {
        self.hosted.handle_close_shortcut(window)
    }

    fn handle(&mut self, event: Event, window: &mut Window) {
        match &event {
            Event::Command(CommandId(id)) if self.handle_diagnostic_command(*id, window) => {}
            Event::Created => {
                self.hosted.handle(event, window);
                if let (Some(perf), Some(renderer)) =
                    (self.perf.as_mut(), self.hosted.renderer_mut())
                {
                    perf.begin(window, renderer);
                }
            }
            Event::Resized { size, .. } => {
                if let Some(perf) = &mut self.perf {
                    perf.resize(size.width as f32, size.height as f32);
                }
                self.hosted.handle(event, window);
                self.sync_perf_animation(window);
            }
            Event::Redraw if self.perf.is_some() => {
                let theme = self.hosted.theme();
                if let (Some(perf), Some(renderer)) =
                    (self.perf.as_mut(), self.hosted.renderer_mut())
                {
                    perf.poll_presentation_drain(&mut crate::perf::FrameCtx {
                        scene: &mut self.scene,
                        renderer,
                        window,
                        theme,
                    });
                }
            }
            Event::AnimationFrame(frame) if self.perf.is_some() => {
                let frame = *frame;
                let theme = self.hosted.theme();
                if let (Some(perf), Some(renderer)) =
                    (self.perf.as_mut(), self.hosted.renderer_mut())
                {
                    perf.frame(
                        frame,
                        &mut crate::perf::FrameCtx {
                            scene: &mut self.scene,
                            renderer,
                            window,
                            theme,
                        },
                    );
                }
            }
            Event::SystemMotionChanged(reduced) => {
                if let Some(perf) = &mut self.perf {
                    perf.set_reduced_motion(*reduced);
                }
                self.hosted.handle(event, window);
                self.sync_perf_animation(window);
            }
            Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.theme_state.system = Theme {
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
                self.hosted.handle(event, window);
                self.sync_effective_theme();
                self.sync_perf_animation(window);
            }
            Event::CloseRequested
                if self.perf.as_ref().is_some_and(crate::perf::Perf::is_active) =>
            {
                if let (Some(perf), Some(renderer)) =
                    (self.perf.as_mut(), self.hosted.renderer_mut())
                {
                    perf.on_close_requested(window, renderer);
                } else {
                    window.close();
                }
            }
            _ => {
                self.hosted.handle(event, window);
                self.sync_perf_animation(window);
            }
        }
    }
}
