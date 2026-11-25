use crate::{
    navigation_window::{NavigationOptions, WindowNavigation},
    text::Text,
};
use anyhow::{Result, ensure};
use std::sync::mpsc::{self, Receiver, SyncSender};
use tekes_ui_app_protocol::{InputKind, Request, ToolbarItem, VERSION};
use tekes_ui_controls::{
    ControlKey, ToolbarControl,
    composition::{
        ComponentId, ContentEvent, ContentHost, ContentKeyEvent, InputModifiers, PointerButton,
        PointerEvent, PointerModifiers, PointerPhase,
    },
    extended::{Axis, SplitView},
    layout_view::SplitViewView,
};
use tekes_ui_macos_bridge::WindowContentAccessibilityExt;
use tekes_ui_platform_macos::{
    App, AppConfig, Event, KeyEvent, KeyEventKind, MenuBar, MouseButton, MouseEvent,
    MouseEventKind, Size as WindowSize, Window, WindowConfig, WindowHandler, WindowWake,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, ImageQuad, Point, Quad, Rect, Scene};
#[cfg(test)]
use tekes_ui_tokens::Rgba;
use tekes_ui_tokens::{Palette, Theme};
const MIN_COLUMN: f32 = 120.;
#[cfg(feature = "window-acceptance")]
#[path = "window_acceptance.rs"]
mod window_acceptance;
#[path = "window_accessibility.rs"]
mod window_accessibility;
#[path = "window_columns.rs"]
mod window_columns;
#[path = "window_container_geometry.rs"]
mod window_container_geometry;
#[path = "window_creation.rs"]
mod window_creation;
#[path = "window_layout.rs"]
mod window_layout;
#[path = "window_panels.rs"]
mod window_panels;
#[path = "window_presentation.rs"]
mod window_presentation;
#[path = "window_settings.rs"]
mod window_settings;
#[path = "window_switcher.rs"]
mod window_switcher;
#[path = "window_toolbar_mount.rs"]
mod window_toolbar_mount;
use window_panels::Panel;
/// Same allocation used by the accepted public toolbar-split consumer:
/// shared typography measures the row; the platform supplies its native center.
/// Only the content boundary rounds outward to the drawable pixel grid.
fn window_toolbar_geometry(
    native_height: f32,
    measured: f32,
    native_center: Option<f32>,
    scale: f32,
) -> (f32, f32, f32) {
    // Oversized content keeps the existing complete, nonnegative allocation;
    // its center cannot simultaneously remain at the smaller native center.
    let row_y = native_center.map_or(0., |center| (center - measured * 0.5).max(0.));
    let allocation = (native_height.max(measured).max(row_y + measured) * scale).ceil() / scale;
    (allocation, row_y, measured)
}
// Top rows share their measured, control-safe boundary, not AppKit's potentially
// larger titlebar allocation. Toolbar-free containers only avoid actual controls.
// Settings retains its existing native-top chrome contract. General container
// layout uses ContainerGeometry and never derives four-edge content from this.
fn presentation_content_inset(
    measured: f32,
    row_boundary: f32,
    controls_exclusion: f32,
    native_row: bool,
    intersects_controls: bool,
) -> f32 {
    if measured > 0. {
        if native_row {
            row_boundary.max(if intersects_controls {
                controls_exclusion
            } else {
                0.
            })
        } else if intersects_controls {
            measured.max(controls_exclusion)
        } else {
            measured
        }
    } else if intersects_controls {
        controls_exclusion
    } else {
        0.
    }
}
fn toolbar_divider_frame(divider: Rect, row_y: f32) -> Rect {
    let top = divider.min_y().max(row_y);
    Rect::new(
        divider.min_x(),
        top,
        divider.size.width,
        (divider.max_y() - top).max(0.),
    )
}

// Top-only allocation for the existing Settings drawing path and legacy tests.
// Local/Process containers use Panel::content_frame's canonical four-edge result.
fn content_geometry(bounds: Rect, toolbar_height: f32) -> Rect {
    Rect::new(
        bounds.min_x(),
        bounds.min_y() + toolbar_height,
        bounds.size.width,
        (bounds.size.height - toolbar_height).max(1.),
    )
}

// Each original ContentHost keeps its frame leases until its next render.
// The shared SVG renderer owns uploads and renderer-scoped image retirement.
pub(super) fn surface_ground(palette: Palette, surface: &str) -> tekes_ui_tokens::Rgba {
    tekes_ui_tokens::surface_palette::resolve_surface(palette, surface, "content", "rest")
        .expect("valid semantic surface ground")
        .background
}

fn with_content_icons<R>(
    renderer: &mut Renderer,
    icons: &mut tekes_ui_controls::composition::SvgIconCache,
    text: &mut Text,
    render: impl FnOnce(&mut dyn tekes_ui_controls::ButtonTextRenderer) -> Result<R>,
) -> Result<R> {
    let mut service = icons.renderer(renderer, text.scale);
    render(&mut tekes_ui_controls::composition::WithIconRenderer::new(
        text,
        &mut service,
    ))
}
// Resource preparation never overlaps the SVG service's Renderer borrow. The
// original host still owns the single layout/paint and all input/AX publication.
fn render_local_content(
    local: &mut crate::LocalContainer,
    icons: &mut tekes_ui_controls::composition::SvgIconCache,
    text: &mut Text,
    scene: &mut Scene,
    mut context: crate::ContainerRenderContext<'_>,
) -> Result<()> {
    local.prepare_render();
    local.prepare_resources(&mut context)?;
    with_content_icons(context.renderer, icons, text, |text| {
        local.render_prepared(scene, text, context.frame, context.clip, context.palette)
    })?;
    local.finish_resources(&mut context)
}
#[cfg(test)]
fn content_size(bounds: Rect, toolbar_height: f32) -> (u32, u32) {
    let content = content_geometry(bounds, toolbar_height);
    (
        content.size.width.max(1.) as u32,
        content.size.height.max(1.) as u32,
    )
}
fn take_local_navigation_request(
    panels: &mut [Panel],
    input_open: bool,
) -> Option<crate::navigation::Entry> {
    input_open
        .then(|| {
            panels
                .iter_mut()
                .filter_map(|panel| panel.local.as_mut())
                .find_map(|local| local.take_navigation_request())
        })
        .flatten()
}
#[cfg(test)]
fn content_point(bounds: Rect, toolbar_height: f32, point: Point) -> Point {
    let content = content_geometry(bounds, toolbar_height);
    Point::new(point.x - content.min_x(), point.y - content.min_y())
}
fn visible_frames(frames: &[Rect], whole: Rect, leading: bool, trailing: bool) -> Vec<Rect> {
    if frames.len() == 2 {
        return vec![
            if leading { frames[0] } else { Rect::default() },
            if leading { frames[1] } else { whole },
        ];
    }
    let mut result = frames.to_vec();
    let start = if leading {
        frames[1].min_x()
    } else {
        whole.min_x()
    };
    let end = if trailing {
        frames[1].max_x()
    } else {
        whole.max_x()
    };
    if !leading {
        result[0] = Rect::default();
    }
    if !trailing {
        result[2] = Rect::default();
    }
    result[1] = Rect::new(
        start,
        whole.min_y(),
        (end - start).max(0.),
        whole.size.height,
    );
    result
}
// Pixel-snapped presentation and input share this geometry. The opposite side
// remains fixed when only one visibility channel changes.
fn presented_frames(frames: &[Rect], whole: Rect, progress: [f32; 2], scale: f32) -> Vec<Rect> {
    if progress.iter().all(|p| *p == 0. || *p == 1.) {
        return visible_frames(frames, whole, progress[0] == 1., progress[1] == 1.);
    }
    let snap = |x: f32| (x * scale).round() / scale;
    let left_end = if progress[0] == 1. {
        frames[0].max_x()
    } else {
        snap(whole.min_x() + frames[0].size.width * progress[0])
    };
    let center_start = if progress[0] == 1. {
        frames[1].min_x()
    } else {
        snap(whole.min_x() + (frames[1].min_x() - whole.min_x()) * progress[0])
    };
    let center_end = if progress[1] == 1. {
        frames[1].max_x()
    } else {
        snap(whole.max_x() - (whole.max_x() - frames[1].max_x()) * progress[1])
    };
    if frames.len() == 2 {
        return vec![
            Rect::new(
                whole.min_x(),
                whole.min_y(),
                (left_end - whole.min_x()).max(0.),
                whole.size.height,
            ),
            Rect::new(
                center_start,
                whole.min_y(),
                (whole.max_x() - center_start).max(0.),
                whole.size.height,
            ),
        ];
    }
    let right_start = if progress[1] == 1. {
        frames[2].min_x()
    } else {
        snap(whole.max_x() - frames[2].size.width * progress[1])
    };
    let mut result = vec![
        Rect::new(
            whole.min_x(),
            whole.min_y(),
            (left_end - whole.min_x()).max(0.),
            whole.size.height,
        ),
        Rect::new(
            center_start,
            whole.min_y(),
            (center_end - center_start).max(0.),
            whole.size.height,
        ),
        Rect::new(
            right_start,
            whole.min_y(),
            (whole.max_x() - right_start).max(0.),
            whole.size.height,
        ),
    ];
    if progress[0] == 1. {
        result[0] = frames[0];
    }
    if progress[1] == 1. {
        result[2] = frames[2];
    }
    result
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarTarget {
    Visibility(usize),
    Navigation,
    Panel(usize),
    Retained {
        owner: usize,
        presenter: usize,
    },
    Contribution {
        presenter: usize,
        group: ComponentId,
    },
}

struct NavigationCoordinator {
    navigation_options: Option<NavigationOptions>,
    navigation: Option<WindowNavigation>,
    navigation_ack: Option<Receiver<()>>,
    navigation_order: Option<Vec<usize>>,
    navigation_content_seq: Vec<(usize, u64)>,
    navigation_noticed: bool,
    navigation_title: Option<String>,
}

impl NavigationCoordinator {
    fn new(navigation_options: Option<NavigationOptions>) -> Self {
        Self {
            navigation_options,
            navigation: None,
            navigation_ack: None,
            navigation_order: None,
            navigation_content_seq: Vec::new(),
            navigation_noticed: false,
            navigation_title: None,
        }
    }
}

struct HostDiagnostics {
    draw_diagnostic: bool,
    diagnostic_event: u64,
    diagnostic_draw: u64,
    #[cfg(test)]
    pub(super) column_layout_calls: u64,
    #[cfg(test)]
    pub(super) presentation_layout_calls: u64,
    #[cfg(feature = "window-acceptance")]
    acceptance: bool,
    #[cfg(feature = "window-acceptance")]
    acceptance_shared: Option<window_acceptance::Shared>,
    #[cfg(feature = "window-acceptance")]
    presentation_acceptance: Option<window_acceptance::PresentationAcceptance>,
    #[cfg(feature = "window-acceptance")]
    acceptance_draws: u64,
    #[cfg(feature = "window-acceptance")]
    step: u32,
}

impl HostDiagnostics {
    fn new(_acceptance: bool) -> Self {
        Self {
            draw_diagnostic: std::env::var("TEKESUI_APPHOST_DRAW_DIAGNOSTIC").as_deref() == Ok("1"),
            diagnostic_event: 0,
            diagnostic_draw: 0,
            #[cfg(test)]
            column_layout_calls: 0,
            #[cfg(test)]
            presentation_layout_calls: 0,
            #[cfg(feature = "window-acceptance")]
            acceptance: _acceptance,
            #[cfg(feature = "window-acceptance")]
            acceptance_shared: None,
            #[cfg(feature = "window-acceptance")]
            presentation_acceptance: None,
            #[cfg(feature = "window-acceptance")]
            acceptance_draws: 0,
            #[cfg(feature = "window-acceptance")]
            step: 0,
        }
    }
}

pub struct Host {
    pub(crate) theme: Theme,
    palette_source: Option<std::rc::Rc<std::cell::Cell<Palette>>>,
    window_title: Option<String>,
    pub(super) extension_icons: Vec<tekes_ui_icons::Glyph>,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) text: Text,
    completion: Option<SyncSender<SyncSender<()>>>,
    layout: window_layout::WindowLayoutState,
    panel_mgr: window_panels::PanelManagerState,
    toolbar_mgr: window_toolbar_mount::ToolbarMountState,
    settings_mgr: window_settings::SettingsHostState,
    nav_mgr: NavigationCoordinator,
    diagnostics: HostDiagnostics,
}
impl Host {
    fn visible_text_cursor_regions(&mut self) -> Vec<Rect> {
        if let Some(tabs) = &self.layout.presentation_tabs
            && tabs.content_host().input_overlay_owner().is_some()
        {
            return tabs.content_host().text_cursor_regions();
        }
        if let Some(grid) = &self.layout.presentation_creation_grid
            && grid.host.input_overlay_owner().is_some()
        {
            return grid.host.text_cursor_regions();
        }
        if let Some(target) = self.open_toolbar()
            && let Some(index) = self.toolbar_container(target)
        {
            return self.panel_mgr.panels[index]
                .mounted_toolbar
                .host
                .content_host()
                .text_cursor_regions();
        }
        let mut regions = Vec::new();
        for panel in &mut self.panel_mgr.panels {
            if !panel.bounds.is_empty() {
                let clip = if self.layout.presentation.is_some() {
                    panel
                        .content_frame()
                        .intersection(&panel.clip)
                        .unwrap_or_default()
                } else {
                    panel.content_frame()
                };
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |host| {
                        regions.extend(
                            host.text_cursor_regions()
                                .into_iter()
                                .filter_map(|region| region.intersection(&clip)),
                        );
                    });
                }
            }
            // The original Focus presenter also owns Window controls when its
            // body is hidden by expanded Others. Use mounted control geometry.
            regions.extend(
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .text_cursor_regions(),
            );
        }
        regions
    }

    pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
    pub(crate) fn with_palette_source(
        mut self,
        source: Option<std::rc::Rc<std::cell::Cell<Palette>>>,
    ) -> Self {
        self.palette_source = source;
        self
    }
    pub(crate) fn with_window_title(mut self, title: Option<String>) -> Self {
        self.window_title = title;
        self
    }
    pub(crate) fn with_extension_icons(mut self, icons: Vec<tekes_ui_icons::Glyph>) -> Self {
        self.set_extension_icons(icons);
        self
    }
    pub(super) fn set_extension_icons(&mut self, icons: Vec<tekes_ui_icons::Glyph>) {
        self.extension_icons = icons.clone();
        self.layout.presentation_icons.set_extensions(icons.clone());
        for panel in &mut self.panel_mgr.panels {
            panel.set_extension_icons(icons.clone());
        }
    }
    #[cfg(test)]
    pub(crate) fn window_title(&self) -> Option<&str> {
        self.window_title.as_deref()
    }
    fn palette(&self) -> Palette {
        self.palette_source.as_ref().map_or_else(
            || self.theme.into(),
            |source| source.get().with_theme(self.theme),
        )
    }
    pub(crate) fn new(
        module_ids: Vec<String>,
        acceptance: bool,
        navigation_options: Option<NavigationOptions>,
    ) -> Self {
        let n = module_ids.len();
        Self {
            theme: Theme::LIGHT,
            palette_source: None,
            window_title: None,
            extension_icons: Vec::new(),
            renderer: None,
            text: Text::new(),
            completion: None,
            layout: window_layout::WindowLayoutState::new(n),
            panel_mgr: window_panels::PanelManagerState::new(module_ids),
            toolbar_mgr: window_toolbar_mount::ToolbarMountState::new(),
            settings_mgr: window_settings::SettingsHostState::new(),
            nav_mgr: NavigationCoordinator::new(navigation_options),
            diagnostics: HostDiagnostics::new(acceptance),
        }
    }
    pub(crate) fn from_columns(
        columns: Vec<crate::Column>,
        acceptance: bool,
        navigation: Option<NavigationOptions>,
    ) -> Self {
        let ids = columns.iter().map(|c| c.id().to_owned()).collect();
        let mut host = Self::new(ids, acceptance, navigation);
        host.panel_mgr.column_definitions = Some(columns);
        host
    }
    #[cfg(test)]
    pub(crate) fn from_containers(
        containers: Vec<crate::Container>,
        acceptance: bool,
        navigation: Option<NavigationOptions>,
        presentation: Option<crate::PresentationOptions>,
    ) -> Self {
        Self::from_containers_with_icons(
            containers,
            acceptance,
            navigation,
            presentation,
            Vec::new(),
        )
    }
    pub(crate) fn from_containers_with_icons(
        containers: Vec<crate::Container>,
        acceptance: bool,
        navigation: Option<NavigationOptions>,
        presentation: Option<crate::PresentationOptions>,
        extension_icons: Vec<tekes_ui_icons::Glyph>,
    ) -> Self {
        let columns: Vec<_> = containers.into_iter().map(crate::Column::from).collect();
        let state = presentation.map(|options| {
            crate::PresentationState::from_columns_with_icons(&columns, options, &extension_icons)
                .expect("AppWindow validates presentation before mounting")
        });
        let mut host = Self::from_columns(columns, acceptance, navigation);
        host.set_extension_icons(extension_icons);
        if let Some(state) = &state {
            host.layout.focused = host
                .panel_mgr
                .module_ids
                .iter()
                .position(|id| id == state.focus_id())
                .unwrap();
            host.layout.leading_visible = true;
        }
        if state.is_some() {
            host.layout.inner_view = SplitViewView::new(Axis::Horizontal);
        }
        host.layout.presentation = state;
        host
    }
    fn mount_column(&self, index: usize, column: crate::Column) -> Panel {
        let mut panel = Panel::from_column(index, column, self.layout.presentation.as_ref());
        panel.set_extension_icons(self.extension_icons.clone());
        panel
    }
    pub(super) fn mount_columns(&mut self) -> bool {
        if let Some(columns) = self.panel_mgr.column_definitions.take() {
            self.panel_mgr.panels = columns
                .into_iter()
                .enumerate()
                .map(|(i, column)| self.mount_column(i, column))
                .collect();
            return true;
        }
        false
    }
    /// Mount original container records once, before any service binds their
    /// Toolbar identities. GPU/window initialization must not recreate them.
    pub(crate) fn mount_initial_containers(&mut self) {
        if !self.panel_mgr.panels.is_empty() {
            return;
        }
        if !self.mount_columns() {
            self.panel_mgr.panels = self
                .panel_mgr
                .module_ids
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, id)| {
                    let mut panel = Panel::new(index as u32, id);
                    panel.set_extension_icons(self.extension_icons.clone());
                    panel
                })
                .collect();
        }
    }
    fn init(&mut self, w: &Window) -> Result<()> {
        if self.renderer.is_some() {
            return Ok(());
        }
        if self.diagnostics.draw_diagnostic {
            w.set_presentation_timing(true);
        }
        let renderer = unsafe {
            Renderer::new_for_metal_layer(w.metal_layer(), w.drawable_size(), w.scale() as f32)
        }?;
        w.set_text_key_events(true);
        let device = renderer.gpu_context().device().clone();
        let (tx, rx) = mpsc::sync_channel::<SyncSender<()>>(16);
        let completion_wake = w.wake_proxy();
        std::thread::spawn(move || {
            while let Ok(ack) = rx.recv() {
                if device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
                    break;
                }
                let _ = ack.send(());
                completion_wake.wake();
            }
        });
        self.completion = Some(tx);
        self.renderer = Some(renderer);
        self.mount_initial_containers();
        if let Some(options) = self.nav_mgr.navigation_options.take() {
            let index = self
                .layout
                .presentation
                .as_ref()
                .and_then(|state| state.navigation_toolbar_id())
                .and_then(|id| {
                    self.panel_mgr
                        .module_ids
                        .iter()
                        .position(|candidate| candidate == id)
                })
                .unwrap_or(0);
            self.nav_mgr.navigation = Some(WindowNavigation::new(
                options,
                w.wake_proxy(),
                self.panel_mgr.panels[index].mounted_toolbar.window,
            )?);
        }
        Ok(())
    }
    fn retire_hidden_content(&mut self, index: usize, bounds: Rect) {
        // A hidden presentation frame can retain an origin and height. Keep
        // that canonical allocation: replacing it with default would make the
        // next identical layout cancel the still-visible Window toolbar again.
        self.panel_mgr.panels[index].bounds = bounds;
        if let Some(local) = &mut self.panel_mgr.panels[index].local {
            local.set_active(false);
        }
        if self.layout.focused == index && !self.panel_mgr.panels[index].logically_visible {
            self.layout.focused = self
                .layout
                .presentation
                .as_ref()
                .and_then(|state| {
                    self.panel_mgr
                        .module_ids
                        .iter()
                        .position(|id| id == state.focus_id())
                })
                .unwrap_or_else(|| {
                    self.layout
                        .order
                        .get(usize::from(self.layout.order.len() >= 2))
                        .copied()
                        .or_else(|| self.layout.order.first().copied())
                        .unwrap_or(0)
                });
        }
    }
    fn draw(&mut self, w: &Window) -> Result<()> {
        let redraw_generation = w.redraw_generation();
        // One opt-in summary at the end avoids synchronous stage logging inside
        // the measured layout path. These are CPU boundaries, not presentation.
        let diagnostic = self.diagnostics.draw_diagnostic.then(|| {
            self.diagnostics.diagnostic_draw += 1;
            (
                std::time::Instant::now(),
                self.diagnostics.diagnostic_event,
                self.diagnostics.diagnostic_draw,
            )
        });
        self.init(w)?;
        self.sync_settings_display(w)?;
        if !self.settings_mgr.is_settings_window {
            self.poll_settings_open(w)?;
            if w.is_active()
                && !self.layout.sidebar_motion.needs_frame()
                && self
                    .nav_mgr
                    .navigation
                    .as_ref()
                    .is_none_or(|nav| !nav.busy && nav.host.input_open && nav.publication.is_none())
            {
                for panel in &mut self.panel_mgr.panels {
                    if let Some(local) = &mut panel.local
                        && let Some(request) = local.take_window_open_request(self.theme)?
                    {
                        request.app.open(w, request.config)?;
                        break;
                    }
                }
            }
        }
        self.text.begin_frame(
            self.layout.motion_epoch.elapsed(),
            self.layout.reduced_motion,
        );
        if self.settings_mgr.is_settings_window {
            // This probe covers the ordinary AppHost draw below, not Settings'
            // separate renderer. Event timing still bounds the delegated call.
            return self.draw_settings(w);
        }
        let initialized = diagnostic.map(|_| std::time::Instant::now());
        let progress = self.layout.sidebar_motion.sample(
            self.layout.motion_epoch.elapsed(),
            self.layout.reduced_motion,
            w.is_active(),
        )?;
        let transitioning = self.layout.sidebar_motion.needs_frame();
        if let Some(nav) = &mut self.nav_mgr.navigation {
            if self
                .nav_mgr
                .navigation_ack
                .as_ref()
                .is_some_and(|ack| ack.try_recv().is_ok())
            {
                self.nav_mgr.navigation_ack = None;
                self.nav_mgr.navigation_order = None;
                self.nav_mgr.navigation_content_seq.clear();

                nav.published()?;
            }
            if let Err(error) = nav.poll() {
                nav.error = Some(error.to_string());
                nav.host.input_open = false;
                host_diag!("NAV_ERROR {error:#}");
                w.set_title(&format!("Navigation: {error}"));
            }
            if let Some(entry) = take_local_navigation_request(
                &mut self.panel_mgr.panels,
                !nav.busy && nav.host.input_open && nav.publication.is_none(),
            ) {
                nav.send(crate::navigation::Command::Navigate(
                    crate::navigation::Action::Push(entry),
                ))?;
            }
            if self.nav_mgr.navigation_order.is_none()
                && let Some(label) = nav.publication_label()?
            {
                self.layout.pressed = None;
                self.panel_mgr.panels[self.toolbar_mgr.toolbar_index]
                    .mounted_toolbar
                    .host
                    .event(&ContentEvent::Cancel);
                for p in &mut self.panel_mgr.panels {
                    p.request(InputKind::Cancel, 0., 0.)?;
                }
                let base = self.window_title.as_deref().unwrap_or("TekesUI AppHost");
                let title = format!("{base} · {label}");
                w.set_title(&title);
                self.nav_mgr.navigation_title = Some(title);
                self.nav_mgr.navigation_order = Some(self.layout.order.clone());
                if let Some(command) = nav.publication_view_command()? {
                    let targets: Vec<usize> = if let Some(state) = &self.layout.presentation {
                        state
                            .navigation_targets()
                            .iter()
                            .filter_map(|id| {
                                self.panel_mgr
                                    .module_ids
                                    .iter()
                                    .position(|candidate| candidate == id)
                            })
                            .collect()
                    } else {
                        vec![self.layout.order[usize::from(self.panel_mgr.panels.len() >= 2)]]
                    };
                    for index in targets {
                        let panel = &mut self.panel_mgr.panels[index];
                        panel.start(w.wake_proxy());
                        panel.enqueue(InputKind::Command, 0., 0., Some(command))?;
                        // Local commands finish synchronously. Hidden locals have
                        // no frame obligation; remote commands await their real reply.
                        if panel.local.is_none()
                            || panel.bounds.size.width > 0. && panel.bounds.size.height > 0.
                        {
                            self.nav_mgr.navigation_content_seq.push((index, panel.seq));
                        }
                        nav.trace(serde_json::json!({"event":"navigation-view-command","panel":index,"command":command,"seq":panel.seq}));
                    }
                }
            }
        }
        self.layout.outer.min_pane = MIN_COLUMN;
        self.layout.inner.min_pane = MIN_COLUMN;
        let mut scene = Scene::new();
        let mut toolbar_scene = Scene::new();
        let mut toolbar_overlays = Scene::new();
        let mut presentation_scene = Scene::new();
        let size = w.size();
        let whole = Rect::new(0., 0., size.width as f32, size.height as f32);
        let row_height = tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text);

        let scale = w.scale() as f32;
        let native_center = w.set_toolbar_row_height(f64::from(row_height));
        let (native_height, leading) = w.titlebar_safe_area();
        let (toolbar_height, toolbar_y, toolbar_paint_height) = window_toolbar_geometry(
            native_height as f32,
            row_height,
            native_center.map(|y| y as f32),
            scale,
        );
        // The platform aligned genuine controls around this center, with their
        // own height as the floor. AppKit's full titlebar allocation can grow
        // during resize and must not become extra padding on the leading pane.
        let controls_floor = native_center.map_or(0., |center| 2. * center as f32);
        let content_boundary = (toolbar_y + toolbar_paint_height).max(controls_floor);
        self.layout.container_window_boundary = Some(content_boundary);

        let native_geometry = (
            native_height as f32,
            toolbar_y,
            toolbar_paint_height,
            toolbar_height,
        );
        if self.layout.presentation.is_some()
            && self.layout.native_toolbar_geometry != Some(native_geometry)
        {
            self.layout.native_toolbar_geometry = Some(native_geometry);
            host_diag!(
                "APPHOST_TOOLBAR_GEOMETRY native_height={} row_y={} paint_height={} allocation={} content_boundary={}",
                native_geometry.0,
                native_geometry.1,
                native_geometry.2,
                native_geometry.3,
                content_boundary
            );
        }
        // Workspace panes are flat layout regions, not decorated Panel cards.
        // Keep component defaults intact; the consumer chooses no frame/shadow.
        let palette = self.palette();
        scene.push_quad(Quad::new(whole, surface_ground(palette, "canvas")));
        let frames = if self.layout.presentation.is_some() {
            self.presentation_frames(
                whole,
                w,
                toolbar_y + toolbar_paint_height,
                window_toolbar_mount::ToolbarBand {
                    y: toolbar_y,
                    height: toolbar_paint_height,
                    native_leading: leading as f32,
                },
            )?
        } else {
            let mut frames = self.arrange_column_frames(whole);
            if self.panel_mgr.panels.len() >= 2 {
                frames = presented_frames(&frames, whole, progress, w.scale() as f32);
            }
            frames
        };
        self.text.scale = w.scale() as f32;
        let renderer = self.renderer.as_mut().unwrap();
        for p in &mut self.panel_mgr.panels {
            let mut latest = None;
            for frame in p.poll_status_and_frames() {
                let meta = frame.metadata();
                // A continuous resize may always be one viewport ahead of the
                // producer. Present monotonic progress instead of starving
                // every completed frame until pointer release. Geometry-
                // dependent input remains gated below.
                if meta.viewport >= p.frame_viewport
                    && meta.viewport <= p.viewport
                    && meta.seq > p.frame_seq
                {
                    latest = Some(frame);
                }
            }
            if let Some(frame) = latest {
                let f = frame.metadata().clone();
                let view_image = frame.import(&renderer.gpu_context())?;
                let id = renderer.register_gpu_image(view_image.image())?;
                if let Some(old) = p.image.replace(id) {
                    renderer.unregister_image(old);
                }
                p.view_image = Some(view_image);
                p.frame_size = (f.width, f.height);
                p.frame_pixels = (f.pixel_width, f.pixel_height);
                p.frame_viewport = f.viewport;
                p.frame_seq = f.seq;
                p.pid = Some(f.pid);
                if p.toolbar != f.toolbar {
                    p.toolbar_generation += 1;
                }
                p.toolbar = f.toolbar;
                host_diag!(
                    "FRAME panel={} pid={} seq={} viewport={} revision={}",
                    p.id,
                    f.pid,
                    f.seq,
                    f.viewport,
                    f.revision
                );
            }
        }
        let _ = renderer;
        let panel_input_open = self
            .nav_mgr
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy);
        let title = self
            .nav_mgr
            .navigation
            .as_ref()
            .and_then(|n| n.title())
            .unwrap_or("")
            .to_owned();
        for (slot, bounds) in frames.iter().enumerate() {
            if bounds.size.width > 0. {
                let index = self.layout.order[slot];
                self.panel_mgr.panels[index].start(w.wake_proxy());
                let geometry = self.container_geometry(index, *bounds, content_boundary)?;
                self.panel_geometry_layout(index, *bounds, scale, geometry)?;
            }
        }
        let geometry_ready = diagnostic.map(|_| std::time::Instant::now());
        for (slot, bounds) in frames.iter().enumerate() {
            let index = self.layout.order[slot];
            if bounds.size.width <= 0. {
                self.retire_hidden_content(index, *bounds);
                continue;
            }
            let mut renderer = self.renderer.take().unwrap();
            let painted = self.paint_panel_body(
                index,
                w.is_active(),
                scale,
                palette,
                &mut scene,
                &mut renderer,
            );
            self.renderer = Some(renderer);
            painted?;
        }
        let body_scene_ready = diagnostic.map(|_| std::time::Instant::now());
        if self.layout.presentation.is_some() {
            scene.push_quad(Quad::new(
                Rect::new(
                    whole.min_x(),
                    toolbar_y,
                    whole.size.width,
                    toolbar_paint_height,
                ),
                surface_ground(palette, "toolbar"),
            ));
        }
        self.sync_toolbar(
            w,
            whole,
            &frames,
            window_toolbar_mount::ToolbarBand {
                y: toolbar_y,
                height: toolbar_paint_height,
                native_leading: leading as f32,
            },
            title,
        )?;
        let mut renderer = self.renderer.take().unwrap();
        let rendered = self.render_toolbar_hosts(
            w.is_active(),
            palette,
            &mut renderer,
            &mut toolbar_scene,
            &mut toolbar_overlays,
        );
        self.renderer = Some(renderer);
        rendered?;
        let toolbar_scene_ready = diagnostic.map(|_| std::time::Instant::now());
        let mut toolbar_controls = Vec::new();
        for panel in &self.panel_mgr.panels {
            toolbar_controls.extend(panel.mounted_toolbar.host.content_host().pointer_regions());
        }
        if let Some(tabs) = self.layout.presentation_layout.tabs {
            toolbar_controls.push(tabs);
        }
        let band = Some(Rect::new(
            whole.min_x(),
            toolbar_y,
            whole.size.width,
            toolbar_paint_height,
        ));
        if leading > 0. {
            toolbar_controls.push(Rect::new(
                whole.min_x(),
                toolbar_y,
                leading as f32,
                toolbar_paint_height,
            ));
        }
        self.layout
            .outer_view
            .set_toolbar_region(band, &toolbar_controls, &mut self.text);
        self.layout
            .inner_view
            .set_toolbar_region(band, &toolbar_controls, &mut self.text);
        if self.layout.presentation.is_none() && frames.len() >= 2 && self.layout.leading_visible {
            if transitioning && frames.len() == 3 {
                let gap = Rect::new(
                    frames[0].max_x(),
                    0.,
                    (frames[1].min_x() - frames[0].max_x()).max(0.),
                    whole.size.height,
                );
                self.layout.outer_view.paint_presented_divider(
                    &mut scene,
                    &self.layout.outer,
                    toolbar_divider_frame(gap, toolbar_y),
                    palette,
                    w.scale() as f32,
                );
            } else if !transitioning {
                self.layout.outer_view.paint_presented_divider(
                    &mut scene,
                    &self.layout.outer,
                    toolbar_divider_frame(
                        self.layout.outer_view.layout().divider.frame(),
                        toolbar_y,
                    ),
                    palette,
                    scale,
                );
            }
        }
        if self.layout.presentation.is_none() && frames.len() == 3 && self.layout.trailing_visible {
            if transitioning {
                let gap = Rect::new(
                    frames[1].max_x(),
                    0.,
                    (frames[2].min_x() - frames[1].max_x()).max(0.),
                    whole.size.height,
                );
                self.layout.inner_view.paint_presented_divider(
                    &mut scene,
                    &self.layout.inner,
                    toolbar_divider_frame(gap, toolbar_y),
                    palette,
                    w.scale() as f32,
                );
            } else {
                self.layout.inner_view.paint_presented_divider(
                    &mut scene,
                    &self.layout.inner,
                    toolbar_divider_frame(
                        self.layout.inner_view.layout().divider.frame(),
                        toolbar_y,
                    ),
                    palette,
                    scale,
                );
            }
        }
        if self.layout.presentation.is_some() {
            self.paint_presentation(&mut presentation_scene, whole, palette)?;
        }
        let mut resize_regions = Vec::new();
        if self.layout.presentation.is_none()
            && frames.len() >= 2
            && self.layout.leading_visible
            && !transitioning
        {
            resize_regions.extend(self.layout.outer_view.resize_regions(&self.layout.outer));
        }
        if self.layout.presentation.is_none()
            && frames.len() == 3
            && self.layout.trailing_visible
            && !transitioning
        {
            resize_regions.extend(self.layout.inner_view.resize_regions(&self.layout.inner));
        }
        if self.layout.presentation.is_some() {
            resize_regions.extend(self.presentation_resize_regions());
        }
        let mut cursor_regions: Vec<_> = self
            .visible_text_cursor_regions()
            .into_iter()
            .map(|r| tekes_ui_platform_macos::CursorRegion {
                origin: tekes_ui_platform_macos::Point {
                    x: r.min_x() as f64,
                    y: r.min_y() as f64,
                },
                size: tekes_ui_platform_macos::Size {
                    width: r.size.width as f64,
                    height: r.size.height as f64,
                },
                icon: tekes_ui_platform_macos::CursorIcon::IBeam,
            })
            .collect();
        // Native lookup is back-to-front: splitters retain priority over text.
        cursor_regions.extend(resize_regions.into_iter().map(|(r, axis)| {
            tekes_ui_platform_macos::CursorRegion {
                origin: tekes_ui_platform_macos::Point {
                    x: r.min_x() as f64,
                    y: r.min_y() as f64,
                },
                size: tekes_ui_platform_macos::Size {
                    width: r.size.width as f64,
                    height: r.size.height as f64,
                },
                icon: match axis {
                    Axis::Horizontal => tekes_ui_platform_macos::CursorIcon::ResizeHorizontal,
                    Axis::Vertical => tekes_ui_platform_macos::CursorIcon::ResizeVertical,
                },
            }
        }));
        w.set_cursor_regions(cursor_regions);
        self.append_toolbar_layers(
            &mut scene,
            &toolbar_scene,
            &presentation_scene,
            &toolbar_overlays,
        );
        let renderer_begin = diagnostic.map(|_| std::time::Instant::now());
        let drawable_before = diagnostic.map(|_| w.drawable_serial());
        let renderer = self.renderer.as_mut().unwrap();
        renderer.resize(w.drawable_size(), w.scale() as f32);
        renderer.render(
            &scene,
            &mut self.text.atlas,
            surface_ground(palette, "canvas"),
        )?;
        self.layout.presentation_scroll_pending = false;
        w.satisfy_pending_redraw(redraw_generation);
        let renderer_return = diagnostic.map(|_| std::time::Instant::now());
        let drawable_after = diagnostic.map(|_| w.drawable_serial());
        #[cfg(feature = "window-acceptance")]
        if self.diagnostics.acceptance {
            self.diagnostics.acceptance_draws += 1;
        }
        w.set_content_accessibility_publication(
            self.content_accessibility_publication(w.is_active() && panel_input_open),
        );
        w.set_text_input_state(self.local_text_state(w.is_active() && panel_input_open));
        if cfg!(debug_assertions)
            && (transitioning || w.is_animating())
            && let Some(nav) = &self.nav_mgr.navigation
        {
            nav.trace(serde_json::json!({
                "event":"sidebar-layout-sample", "progress":progress,
                "active":w.is_active(), "animating":transitioning,
                "display_link_frames":w.animation_frame_count(),
                "bounds":frames.iter().map(|r| [r.min_x(),r.min_y(),r.size.width,r.size.height]).collect::<Vec<_>>()
            }));
        }
        if self.nav_mgr.navigation_order.is_some()
            && self.nav_mgr.navigation_ack.is_none()
            && self
                .nav_mgr
                .navigation_content_seq
                .iter()
                .all(|&(index, seq)| self.panel_mgr.panels[index].frame_seq >= seq)
            && self.panel_mgr.panels.iter().all(|p| {
                p.bounds.size.width <= 0.
                    || ((p.local.is_some() || p.image.is_some())
                        && p.frame_viewport == p.viewport
                        && p.frame_size == p.content_size())
            })
        {
            let (tx, rx) = mpsc::sync_channel(1);
            self.completion
                .as_ref()
                .unwrap()
                .try_send(tx)
                .map_err(|_| anyhow::anyhow!("GPU completion backlog"))?;
            self.nav_mgr.navigation_ack = Some(rx);
        }
        if let Some(nav) = &self.nav_mgr.navigation
            && nav.host.input_open
            && !nav.busy
        {
            let name = nav
                .host
                .target
                .as_ref()
                .map(|e| e.id.as_str())
                .unwrap_or("empty");
            let base = self.window_title.as_deref().unwrap_or("TekesUI AppHost");
            let title = format!("{base} · {name}");
            if self.nav_mgr.navigation_title.as_ref() != Some(&title) {
                w.set_title(&title);
                self.nav_mgr.navigation_title = Some(title);
            }
            if !self.nav_mgr.navigation_noticed {
                host_diag!(
                    "NAV_WINDOW_READY host_pid={} nav_status={:?} view_pids={:?} order={:?} toolbar={:?}",
                    std::process::id(),
                    nav.status(),
                    self.panel_mgr
                        .panels
                        .iter()
                        .map(|p| p.pid)
                        .collect::<Vec<_>>(),
                    self.layout.order,
                    self.panel_mgr.panels[self.toolbar_mgr.toolbar_index]
                        .mounted_toolbar
                        .host
                        .content_host()
                        .pointer_regions()
                );
                self.nav_mgr.navigation_noticed = true;
            }
        }
        let local_needs_frame = self.panel_mgr.panels.iter_mut().any(|p| {
            let mut needs = false;
            if p.bounds.size.width > 0.
                && let Some(local) = &mut p.local
            {
                local.with_host(&mut |h| needs = h.needs_render());
            }
            needs
        });
        let animate = (self
            .panel_mgr
            .panels
            .iter()
            .any(|p| p.mounted_toolbar.host.needs_render())
            || local_needs_frame
            || self.layout.sidebar_motion.needs_frame()
            || self.text.animation_requested())
            && w.is_active();
        if !w.set_animating(animate) && animate {
            w.request_redraw();
        }
        if let Some((begin, event, draw)) = diagnostic {
            let end = std::time::Instant::now();
            let offset = |boundary: Option<std::time::Instant>| {
                boundary.map(|at| at.duration_since(begin).as_nanos())
            };
            eprintln!(
                "APPHOST_DRAW_DIAGNOSTIC pid={} window={} event={} draw={} begin_monotonic={begin:?} end_monotonic={end:?} initialized_ns={:?} geometry_ready_ns={:?} body_scene_ready_ns={:?} toolbar_scene_ready_ns={:?} renderer_begin_ns={:?} renderer_return_ns={:?} total_ns={} cpu_only=true drawable_before={drawable_before:?} drawable_after={drawable_after:?} paragraph_layout={:?}",
                std::process::id(),
                w.native_window_number(),
                event,
                draw,
                offset(initialized),
                offset(geometry_ready),
                offset(body_scene_ready),
                offset(toolbar_scene_ready),
                offset(renderer_begin),
                offset(renderer_return),
                end.duration_since(begin).as_nanos(),
                self.text.paragraph_layout_diagnostic(),
            );
        }
        Ok(())
    }

    fn typography_key_input(&mut self, key: &KeyEvent) -> bool {
        if key.kind != KeyEventKind::Down
            || !key.modifiers.command
            || key.modifiers.control
            || key.modifiers.alt
        {
            return false;
        }
        let baseline = tekes_ui_tokens::proposal::TextRole::UiBody.style().size;
        let current = baseline * self.text.type_scale();
        let next = match key.characters_ignoring_modifiers.as_str() {
            "+" | "=" => current + 1.,
            "-" => (current - 1.).max(1.),
            "0" => baseline,
            _ => return false,
        };
        if self.text.set_body_size(next) {
            // Layout changes invalidate old pointer/Space geometry, but keep
            // the original focused component and worker processes.
            self.cancel_toolbar_input();
            if let Some(nav) = &self.nav_mgr.navigation {
                nav.trace(serde_json::json!({"event":"window-typography","body_size":next,"type_scale":self.text.type_scale()}));
            }
        }
        true
    }
    fn toggle_visibility(&mut self, side: usize) -> Result<()> {
        if self.layout.presentation.is_some() {
            if self.panel_mgr.panels.iter_mut().any(|panel| {
                let mut marked = false;
                if let Some(local) = &mut panel.local {
                    local.with_host(&mut |host| {
                        marked = host
                            .focused_text()
                            .is_some_and(|surface| surface.marked().is_some())
                    });
                }
                marked
            }) {
                return Ok(());
            }
            let state = self.layout.presentation.as_mut().unwrap();
            if side == 0 {
                state.toggle_nav_bar()?;
            } else {
                state.toggle_others()?;
            }
            self.cancel_toolbar_input();
            self.layout.drag_split = None;
            self.layout.outer_view.cancel(&mut self.layout.outer);
            self.layout.inner_view.cancel(&mut self.layout.inner);
            return Ok(());
        }

        if self.settings_mgr.is_settings_window
            && side == 0
            && let Some(settings) = &mut self.settings_mgr.settings
        {
            let mut settings = settings.borrow_mut();
            settings.sidebar.cancel_host();
            settings.sidebar.focus(None, false);
        }

        if side == 0 {
            self.layout.leading_visible = !self.layout.leading_visible;
        } else {
            self.layout.trailing_visible = !self.layout.trailing_visible;
        }
        self.layout.sidebar_motion.retarget(
            side,
            if side == 0 {
                self.layout.leading_visible
            } else {
                self.layout.trailing_visible
            },
            self.layout.motion_epoch.elapsed(),
        )?;
        if let Some(i) = self.layout.pressed.take() {
            self.panel_mgr.panels[i].request(InputKind::Cancel, 0., 0.)?;
        }
        self.cancel_toolbar_input();
        self.layout.drag_split = None;
        self.layout.outer_view.cancel(&mut self.layout.outer);
        self.layout.inner_view.cancel(&mut self.layout.inner);
        for panel in &mut self.panel_mgr.panels {
            if let Some(local) = &mut panel.local {
                local.cancel();
            }
        }
        let slot = if side == 0 { 0 } else { 2 };
        let hidden = if side == 0 {
            !self.layout.leading_visible
        } else {
            !self.layout.trailing_visible
        };
        if hidden && !self.settings_mgr.is_settings_window {
            let i = self.layout.order[slot];
            self.panel_mgr.panels[i].request(InputKind::Cancel, 0., 0.)?;
            // Keep paint alive until the reveal reaches zero; input is gated
            // below immediately and the worker remains mounted for reopening.
            if self.layout.focused == i {
                self.layout.focused = self.layout.order[1];
            }
        }
        if let Some(nav) = &self.nav_mgr.navigation {
            nav.trace(serde_json::json!({"event":"panel-visibility","leading":self.layout.leading_visible,"trailing":self.layout.trailing_visible,"pids":self.panel_mgr.panels.iter().map(|p|p.pid).collect::<Vec<_>>()}));
        }
        Ok(())
    }
    #[cfg(feature = "window-acceptance")]
    fn accept_settings_window_parent(
        &mut self,
        w: &Window,
        shared: &window_acceptance::Shared,
    ) -> Result<()> {
        match shared.stage() {
            window_acceptance::Stage::Main => {
                if !w.is_active() || !w.is_key_window() || w.is_occluded() {
                    w.request_redraw();
                    return Ok(());
                }
                if let Ok(mut result) = shared.0.lock() {
                    result.parent_window = w.native_window_number();
                }
                shared.transition(window_acceptance::Stage::Opening);
                eprintln!(
                    "SETTINGS_ACCEPTANCE_SCOPE scope=settings-window-only env=1 main-resize=not_run panel-frame=not_run parent_window={}",
                    w.native_window_number()
                );
                w.request_redraw();
            }
            window_acceptance::Stage::Closing
                if self
                    .settings_mgr
                    .settings_child
                    .as_ref()
                    .is_some_and(|child| child.is_closed())
                    && w.is_active()
                    && w.is_key_window()
                    && !w.is_occluded() =>
            {
                shared.transition(window_acceptance::Stage::Done);
                eprintln!(
                    "SETTINGS_ACCEPTANCE_PASS parent_window={} child_window={:?} mode={:?} shown_active_key_non_occluded=true saved=true undo_restored=true closed=true parent_restored=true physical_input=false",
                    w.native_window_number(),
                    shared.0.lock().ok().and_then(|s| s.child_window),
                    shared.0.lock().ok().and_then(|s| s.mode)
                );
                w.close();
            }
            window_acceptance::Stage::Failed
                if self
                    .settings_mgr
                    .settings_child
                    .as_ref()
                    .is_none_or(|child| child.is_closed()) =>
            {
                eprintln!(
                    "SETTINGS_ACCEPTANCE_FAIL child_closed=true parent_active={} parent_key={} error={:?}",
                    w.is_active(),
                    w.is_key_window(),
                    shared.0.lock().ok().and_then(|s| s.error.clone())
                );
                w.close();
            }
            _ => {}
        }
        Ok(())
    }
    #[cfg(feature = "window-acceptance")]
    fn accept(&mut self, w: &mut Window) -> Result<()> {
        if !self.diagnostics.acceptance {
            return Ok(());
        }
        if let Some(mut probe) = self.diagnostics.presentation_acceptance.take() {
            let result = self.accept_presentation(w, &mut probe);
            if let Err(error) = &result {
                probe.shared.fail(format!("{error:#}"));
            }
            self.diagnostics.presentation_acceptance = Some(probe);
            return result;
        }
        if let Some(shared) = self.diagnostics.acceptance_shared.clone() {
            return self.accept_settings_window_parent(w, &shared);
        }
        if self.diagnostics.step >= 24
            && self.panel_mgr.panels.len() == 3
            && std::env::var("TEKESUI_COLUMNS_WINDOW_ACCEPTANCE").as_deref() == Ok("1")
        {
            return self.accept_columns(w);
        }
        if self.panel_mgr.panels.iter().any(|p| {
            p.bounds.size.width > 0.
                && (p.local.is_none() && (p.pid.is_none() || p.image.is_none()))
        }) {
            return Ok(());
        }
        // This exercises the same Host dispatch/render path but is deliberately
        // reported as automated native-window acceptance, not physical input.
        if self.diagnostics.step == 0 {
            self.diagnostics.step = 1;
            self.panel_mgr.panels[self.layout.focused].request(InputKind::Command, 0., 0.)?;
        } else if self.diagnostics.step < 22 {
            self.diagnostics.step += 1;
            // Drive a resize burst faster than workers can produce frames,
            // exercising coalescing and the bounded two-frame pipeline.
            w.set_content_size(WindowSize {
                width: 1000. + ((self.diagnostics.step * 37) % 200) as f64,
                height: 600. + ((self.diagnostics.step * 11) % 60) as f64,
            });
        } else if self.diagnostics.step == 22 {
            self.diagnostics.step = 23;
            w.set_content_size(WindowSize {
                width: 1100.,
                height: 640.,
            });
        } else if self.diagnostics.step == 23
            && self
                .panel_mgr
                .panels
                .iter()
                .all(|p| p.frame_viewport == p.viewport)
        {
            self.diagnostics.step = 24;
            w.set_title("TekesUI AppHost — native window ready");
            eprintln!(
                "NATIVE_WINDOW_READY panels={} pids={:?} resize_burst=21 max_inflight_per_panel=2 physical_input=false native_window={}",
                self.panel_mgr.panels.len(),
                self.panel_mgr
                    .panels
                    .iter()
                    .map(|p| p.pid)
                    .collect::<Vec<_>>(),
                w.native_window_number()
            );
        }
        Ok(())
    }
    #[cfg(not(feature = "window-acceptance"))]
    fn accept(&mut self, _w: &mut Window) -> Result<()> {
        Ok(())
    }
    /// Append the original hosts in their final native renderer painter order.
    fn append_toolbar_layers(
        &self,
        scene: &mut Scene,
        toolbar: &Scene,
        presentation: &Scene,
        overlays: &Scene,
    ) {
        // Region ground stays behind the selector even while a business menu is
        // open. Only the original host overlay pass is promoted above it.
        for layer in [toolbar, presentation, overlays] {
            for primitive in layer.primitives() {
                scene.push(primitive.clone());
            }
        }
    }
    /// Blank toolbar chrome falls through to the native window only after
    /// original controls, splitters and body input have had their turn.
    pub(super) fn toolbar_chrome_at(&self, p: Point) -> bool {
        p.y >= 0.
            && (self.layout.presentation.is_some()
                && self
                    .layout
                    .container_window_boundary
                    .is_some_and(|boundary| p.y < boundary)
                || self.panel_mgr.panels.iter().any(|panel| {
                    if self.layout.presentation.is_some() {
                        panel
                            .geometry
                            .toolbar
                            .is_some_and(|frame| frame.contains(p))
                            || panel.geometry.close.is_some_and(|frame| frame.contains(p))
                    } else {
                        p.y < panel.geometry.insets.top
                    }
                }))
    }
    /// Consume only uncaptured drag samples belonging to our accepted native
    /// window handoff; all other events continue through their original routes.
    fn owns_native_titlebar_drag_event(&mut self, event: &Event) -> bool {
        // performWindowDragWithEvent returns immediately. Do not offer its
        // subsequent drag samples to ContentHosts (even refused input retires
        // their geometry lease), or schedule a full body paint for window moves.
        // A new down or ordinary move also ends the scope when AppKit omitted up.
        match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down { .. } | MouseEventKind::Up { .. } | MouseEventKind::Moved => {
                    self.layout.native_titlebar_drag = false
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } if self.layout.native_titlebar_drag
                    && self.layout.drag_split.is_none()
                    && self.layout.pressed.is_none()
                    && self.settings_mgr.settings_pointer.is_none()
                    && self.panel_mgr.panels.iter().all(|panel| {
                        panel
                            .mounted_toolbar
                            .host
                            .content_host()
                            .pointer_capture()
                            .is_none()
                    })
                    && self
                        .layout
                        .presentation_tabs
                        .as_ref()
                        .is_none_or(|tabs| tabs.content_host().pointer_capture().is_none())
                    && self
                        .layout
                        .presentation_creation_grid
                        .as_ref()
                        .is_none_or(|grid| grid.host.pointer_capture().is_none()) =>
                {
                    return true;
                }
                _ => {}
            },
            Event::Focus(false) | Event::CloseRequested => self.layout.native_titlebar_drag = false,
            _ => {}
        }
        false
    }
    fn event(&mut self, event: Event, w: &mut Window) -> Result<()> {
        // Apply before Settings or any input handler borrows its current host.
        // Do not depend on redraw: resign-active need not produce another frame.
        let active = match &event {
            Event::Focus(active) => *active,
            Event::CloseRequested => false,
            _ => w.is_active(),
        };
        for panel in &mut self.panel_mgr.panels {
            if let Some(local) = &mut panel.local {
                local.set_active(active && panel.logically_visible && panel.bounds.size.width > 0.);
            }
        }
        if let Event::SystemAppearanceChanged {
            dark,
            increase_contrast,
        } = &event
            && let Some(binding) = &mut self.settings_mgr.settings_display
        {
            binding.system_theme = Theme {
                appearance: if *dark {
                    tekes_ui_tokens::Appearance::Dark
                } else {
                    tekes_ui_tokens::Appearance::Light
                },
                contrast: if *increase_contrast {
                    tekes_ui_tokens::Contrast::Increased
                } else {
                    tekes_ui_tokens::Contrast::Standard
                },
            };
            self.sync_settings_display(w)?;
            w.request_redraw();
            return Ok(());
        }
        if self.owns_native_titlebar_drag_event(&event) {
            return Ok(());
        }
        #[cfg(feature = "window-acceptance")]
        if !self.settings_mgr.is_settings_window
            && matches!(&event, Event::Created)
            && let Some((shared, scope)) = self
                .diagnostics
                .presentation_acceptance
                .as_ref()
                .map(|probe| (probe.shared.clone(), "presentation"))
                .or_else(|| {
                    self.diagnostics
                        .acceptance_shared
                        .clone()
                        .map(|shared| (shared, "Settings-only"))
                })
        {
            let start_watchdog = if let Ok(mut result) = shared.0.lock() {
                result.parent_window = w.native_window_number();
                result.parent_wake = Some(w.wake_proxy());
                result.started = std::time::Instant::now();
                !std::mem::replace(&mut result.watchdog_started, true)
            } else {
                false
            };
            if start_watchdog {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    if shared.stage() != window_acceptance::Stage::Done {
                        shared.fail(format!("30s total {scope} acceptance timeout"));
                        if let Ok(result) = shared.0.lock() {
                            if let Some(wake) = &result.child_wake {
                                let _ = wake.wake();
                            }
                            if let Some(wake) = &result.parent_wake {
                                let _ = wake.wake();
                            }
                        }
                    }
                });
            }
        }
        #[cfg(feature = "window-acceptance")]
        if self
            .diagnostics
            .acceptance_shared
            .as_ref()
            .is_some_and(|s| s.stage() == window_acceptance::Stage::Opening)
            && !self.settings_mgr.is_settings_window
            && matches!(&event, Event::Redraw | Event::Wake)
        {
            self.event(
                Event::Command(tekes_ui_platform_macos::CommandId(0x534554)),
                w,
            )?;
            if let Some(shared) = &self.diagnostics.acceptance_shared {
                shared.transition(window_acceptance::Stage::AwaitingShown);
            }
            w.request_redraw();
            return Ok(());
        }
        #[cfg(feature = "window-acceptance")]
        if self.settings_mgr.is_settings_window
            && self.diagnostics.acceptance_shared.is_some()
            && matches!(
                &event,
                Event::Redraw | Event::Wake | Event::AnimationFrame(_)
            )
        {
            self.drive_settings_acceptance(w)?;
        }
        // Wheel input retires the published geometry lease. Refresh a pending
        // header before checking AX ownership, then apply the same rejection
        // to the newly published identities (including retired targets).
        if self.layout.presentation_scroll_pending && matches!(&event, Event::Accessibility { .. })
        {
            if std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some() {
                eprintln!(
                    "OTHERS_SCROLL_FLUSH input=AX before_draw={}",
                    self.diagnostics.diagnostic_draw
                );
            }
            self.draw(w)?;
            self.trace_presentation_scroll("flushed-before-input");
        }
        if let Event::Accessibility { target, .. } = &event
            && w.content_accessibility_publication()
                .and_then(|p| p.owner(*target))
                .is_none()
        {
            return Ok(());
        }

        if matches!(
            &event,
            Event::Command(tekes_ui_platform_macos::CommandId(0x534554))
        ) || matches!(&event,Event::Key(k) if k.kind==KeyEventKind::Down && k.modifiers.command && k.characters_ignoring_modifiers==",")
        {
            self.cancel_toolbar_input();
            if let Some(i) = self.layout.pressed.take() {
                self.panel_mgr.panels[i].request(InputKind::Cancel, 0., 0.)?;
            }
            self.layout.drag_split = None;
            if self.settings_mgr.is_settings_window {
                w.request_redraw();
                return Ok(());
            }
            self.open_settings(w)?;
            w.request_redraw();
            return Ok(());
        }
        if self
            .settings_navigation_focus_guard(&event, w.content_accessibility_publication().as_ref())
        {
            w.request_redraw();
            return Ok(());
        }
        if w.is_active() && Self::is_search_shortcut(&event) && self.reveal_registered_search()? {
            // Refresh current layout before formal focus admission; publish the
            // accepted Search editor to native text services before more input.
            self.draw(w)?;
            if self.focus_registered_search()? {
                self.draw(w)?;
                return Ok(());
            }
        }
        if let Event::Key(key) = &event
            && self.typography_key_input(key)
        {
            self.draw(w)?;
            return Ok(());
        }
        // A queued wheel paint can be overtaken by a click or key action.
        // Publish the moved tab geometry before those inputs choose a target.
        if self.layout.presentation_scroll_pending
            && matches!(&event, Event::Mouse(_) | Event::Key(_) | Event::TextKey(_))
        {
            if std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some() {
                let input = match &event {
                    Event::Mouse(_) => "Mouse",
                    _ => "Key",
                };
                eprintln!(
                    "OTHERS_SCROLL_FLUSH input={input} before_draw={}",
                    self.diagnostics.diagnostic_draw
                );
            }
            self.draw(w)?;
            self.trace_presentation_scroll("flushed-before-input");
        }
        if w.is_active() && self.local_secondary_pointer(&event)? {
            w.request_redraw();
            return Ok(());
        }
        if self.presentation_event(&event, w)? {
            if matches!(&event, Event::Scroll(scroll) if self.layout.presentation_layout.tabs.is_some_and(|frame| frame.contains(Point::new(scroll.position.x as f32, scroll.position.y as f32))))
            {
                // Apply every native delta immediately, but coalesce painting on
                // the existing runloop redraw path. A header wheel must not
                // synchronously render all containers and submit a GPU frame
                // for every input sample. Pointer/selection geometry stays on
                // the synchronous path below.
                self.layout.presentation_scroll_pending = true;
                if std::env::var_os("TEKESUI_OTHERS_SCROLL_TRACE").is_some() {
                    eprintln!(
                        "OTHERS_SCROLL_QUEUED draw={}",
                        self.diagnostics.diagnostic_draw
                    );
                }
                w.request_redraw();
            } else {
                self.draw(w)?;
            }
            return Ok(());
        }
        if self.route_toolbar_event(&event, w)? {
            // Publish current geometry before the next native input/AX callback.
            self.draw(w)?;
            return Ok(());
        }

        if self
            .settings_mgr
            .settings
            .as_ref()
            .is_some_and(|s| s.borrow().open)
            && self.settings_event(&event, w)?
        {
            return Ok(());
        }
        if !self.settings_mgr.is_settings_window && self.local_content_event(&event, w)? {
            w.request_redraw();
            return Ok(());
        }
        if let Event::Mouse(m) = &event
            && cfg!(debug_assertions)
            && matches!(
                m.kind,
                MouseEventKind::Down { .. } | MouseEventKind::Up { .. }
            )
            && let Some(nav) = &self.nav_mgr.navigation
        {
            nav.trace(serde_json::json!({"event":"mouse","kind":format!("{:?}",m.kind),"x":m.position.x,"y":m.position.y,"active":w.is_active(),"key_window":w.is_key_window()}));
        }
        if cfg!(debug_assertions)
            && let Event::Key(key) = &event
            && let Some(nav) = &self.nav_mgr.navigation
        {
            nav.trace(serde_json::json!({"event":"window-toolbar-key","code":key.key_code,"kind":format!("{:?}",key.kind),"active":w.is_active(),"owner":format!("{:?}",self.panel_mgr.panels[self.toolbar_mgr.toolbar_index].mounted_toolbar.host.content_host().focused())}));
        }
        match event {
            Event::Created | Event::Redraw | Event::Wake | Event::AnimationFrame(_) => {
                self.draw(w)?
            }
            Event::Resized {
                drawable_size,
                scale,
                ..
            } => {
                if let Some(header) = &mut self.layout.presentation_tabs {
                    header.event(&ContentEvent::Cancel);
                }
                if let Some(grid) = &mut self.layout.presentation_creation_grid {
                    grid.host.event(&ContentEvent::Cancel);
                }
                self.cancel_toolbar_input();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(drawable_size, scale as f32);
                }
            }
            Event::Mouse(m) if w.is_active() => {
                let content_event = toolbar_pointer(m);
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                match m.kind {
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count,
                    } => {
                        if self.layout.presentation.is_none()
                            && self.panel_mgr.panels.len() >= 2
                            && self.layout.leading_visible
                            && !self.layout.sidebar_motion.needs_frame()
                            && self
                                .layout
                                .outer_view
                                .pointer_down(p, &mut self.layout.outer)
                        {
                            self.cancel_toolbar_input();
                            self.layout.drag_split = Some(0);
                        } else if self.layout.presentation.is_none()
                            && self.panel_mgr.panels.len() == 3
                            && self.layout.trailing_visible
                            && !self.layout.sidebar_motion.needs_frame()
                            && self
                                .layout
                                .inner_view
                                .pointer_down(p, &mut self.layout.inner)
                        {
                            self.cancel_toolbar_input();
                            self.layout.drag_split = Some(1);
                        } else if let Some(i) =
                            self.panel_mgr.panels.iter().enumerate().position(|(i, v)| {
                                !self.layout.sidebar_motion.needs_frame()
                                    && (self.layout.presentation.is_some()
                                        || (self.layout.leading_visible
                                            || i != self.layout.order[0])
                                            && (self.panel_mgr.panels.len() < 3
                                                || self.layout.trailing_visible
                                                || i != self.layout.order[2]))
                                    && (self.layout.presentation.is_none() || v.clip.contains(p))
                                    && v.content_frame().contains(p)
                            })
                        {
                            for (other, panel) in self.panel_mgr.panels.iter_mut().enumerate() {
                                if other != i
                                    && let Some(local) = &mut panel.local
                                {
                                    local.cancel();
                                }
                            }
                            self.layout.focused = i;
                            if let Some(tabs) = &mut self.layout.presentation_tabs {
                                tabs.blur();
                            }
                            if let Some(grid) = &mut self.layout.presentation_creation_grid {
                                grid.host.focus(None, false);
                            }
                            self.set_toolbar_focus(None, false);
                            if self
                                .nav_mgr
                                .navigation
                                .as_ref()
                                .is_none_or(|nav| nav.host.input_open && !nav.busy)
                                && self.panel_mgr.panels[i].frame_viewport
                                    == self.panel_mgr.panels[i].viewport
                                && self.panel_mgr.panels[i].frame_size
                                    == self.panel_mgr.panels[i].content_size()
                            {
                                self.layout.pressed = Some(i);
                                if let Some(local) = &mut self.panel_mgr.panels[i].local {
                                    local.event(&content_event, &mut self.text)?;
                                }
                                let point = self.panel_mgr.panels[i].content_point(p);
                                self.panel_mgr.panels[i].request(
                                    InputKind::PointerDown,
                                    point.x,
                                    point.y,
                                )?;
                            }
                        } else if self.toolbar_chrome_at(p) {
                            // Empty segmented toolbar chrome is not child
                            // content. Splitter seams were offered capture
                            // above; remaining blank chrome belongs to Window.
                            if click_count == 2 {
                                w.try_titlebar_double_click();
                            } else {
                                // Only a successful native handoff owns future
                                // drag samples. Keep this down's existing paint
                                // so focus/hover cancellation is acknowledged.
                                self.layout.native_titlebar_drag = w.try_begin_titlebar_drag();
                            }
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        self.layout.outer_view.pointer_up(&mut self.layout.outer);
                        self.layout.inner_view.pointer_up(&mut self.layout.inner);
                        self.layout.drag_split = None;
                        if let Some(i) = self.layout.pressed.take() {
                            if let Some(local) = &mut self.panel_mgr.panels[i].local {
                                local.event(&content_event, &mut self.text)?;
                            }
                            let point = self.panel_mgr.panels[i].content_point(p);
                            self.panel_mgr.panels[i].request(
                                InputKind::PointerUp,
                                point.x,
                                point.y,
                            )?;
                        }
                    }
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => match self.layout.drag_split {
                        Some(0) => {
                            self.layout
                                .outer_view
                                .pointer_move(p, &mut self.layout.outer);
                            let whole = self.layout.outer_view.layout().frame.frame();
                            let second_x = if self.panel_mgr.panels.len() == 2 {
                                whole.max_x()
                            } else {
                                self.layout.inner.layout(whole, Axis::Horizontal).1.min_x()
                            };
                            let limit = (second_x - self.layout.outer.divider - MIN_COLUMN)
                                / (whole.size.width - self.layout.outer.divider);
                            self.layout
                                .outer
                                .set_ratio(self.layout.outer.ratio().min(limit));
                        }
                        Some(1) => {
                            self.layout
                                .inner_view
                                .pointer_move(p, &mut self.layout.inner);
                            let whole = self.layout.inner_view.layout().frame.frame();
                            let first_x =
                                self.layout.outer.layout(whole, Axis::Horizontal).1.max_x();
                            let limit = (first_x + MIN_COLUMN)
                                / (whole.size.width - self.layout.inner.divider);
                            self.layout
                                .inner
                                .set_ratio(self.layout.inner.ratio().max(limit));
                        }
                        _ => {
                            if let Some(i) = self.layout.pressed {
                                if let Some(local) = &mut self.panel_mgr.panels[i].local {
                                    local.event(&content_event, &mut self.text)?;
                                }
                                let point = self.panel_mgr.panels[i].content_point(p);
                                self.panel_mgr.panels[i].request(
                                    InputKind::PointerMove,
                                    point.x,
                                    point.y,
                                )?;
                            }
                        }
                    },
                    MouseEventKind::Moved | MouseEventKind::Entered | MouseEventKind::Exited => {
                        let local_input_open = !self.layout.sidebar_motion.needs_frame()
                            && self.open_toolbar().is_none()
                            && self
                                .nav_mgr
                                .navigation
                                .as_ref()
                                .is_none_or(|n| n.host.input_open && !n.busy);
                        for panel in &mut self.panel_mgr.panels {
                            if panel.bounds.size.width > 0.
                                && local_input_open
                                && let Some(local) = &mut panel.local
                            {
                                local.event(&content_event, &mut self.text)?;
                            }
                        }
                    }
                    _ => {}
                }
                w.request_redraw();
            }
            Event::Key(k) if w.is_active() => {
                let _ = k;
                w.request_redraw();
            }
            Event::Accessibility { .. } => {
                w.request_redraw();
            }
            Event::SystemMotionChanged(reduced) => {
                self.layout.reduced_motion = reduced;
                w.request_redraw();
            }
            Event::Focus(true) => {
                if let Some(header) = &mut self.layout.presentation_tabs {
                    header.set_window_active(true);
                }
                if let Some(grid) = &mut self.layout.presentation_creation_grid {
                    grid.host.set_window_active(true);
                }
                w.set_menu_bar(&self.menu_bar())
                    .map_err(|error| anyhow::anyhow!("AppHost menu: {error}"))?;
                if let Some(panel) = self.panel_mgr.panels.get(self.toolbar_mgr.toolbar_index) {
                    host_diag!(
                        "TOOLBAR_WINDOW_ACTIVE window={} active=true",
                        panel.mounted_toolbar.window
                    );
                }
                for panel in &mut self.panel_mgr.panels {
                    panel.mounted_toolbar.host.set_window_active(true);
                }
                self.layout
                    .outer_view
                    .set_window_active(true, &mut self.layout.outer);
                self.layout
                    .inner_view
                    .set_window_active(true, &mut self.layout.inner);
                w.request_redraw();
            }
            Event::Focus(false) => {
                if let Some(header) = &mut self.layout.presentation_tabs {
                    header.set_window_active(false);
                }
                if let Some(grid) = &mut self.layout.presentation_creation_grid {
                    grid.host.set_window_active(false);
                }
                if let Some(panel) = self.panel_mgr.panels.get(self.toolbar_mgr.toolbar_index) {
                    host_diag!(
                        "TOOLBAR_WINDOW_ACTIVE window={} active=false",
                        panel.mounted_toolbar.window
                    );
                }
                self.remember_toolbar_focus(None);
                for panel in &mut self.panel_mgr.panels {
                    panel.mounted_toolbar.host.set_window_active(false);
                }
                self.layout.sidebar_motion.sample(
                    self.layout.motion_epoch.elapsed(),
                    self.layout.reduced_motion,
                    false,
                )?;
                w.set_animating(false);
                if let Some(i) = self.layout.pressed.take() {
                    self.panel_mgr.panels[i].request(InputKind::Cancel, 0., 0.)?;
                }
                for panel in &mut self.panel_mgr.panels {
                    if let Some(local) = &mut panel.local {
                        local.cancel();
                    }
                }
                self.layout
                    .outer_view
                    .set_window_active(false, &mut self.layout.outer);
                self.layout
                    .inner_view
                    .set_window_active(false, &mut self.layout.inner);
                w.set_cursor_regions(Vec::new());
                self.layout.drag_split = None;
                // Resign-active does not schedule a paint. Revoke the native
                // snapshot now without restarting rendering or animations.
                w.set_content_accessibility_publication(None);
            }
            Event::CloseRequested => w.close(),
            _ => {}
        }
        Ok(())
    }
}
impl Host {
    pub(crate) fn closable_active_tab(&self) -> Option<&str> {
        if self.settings_mgr.is_settings_window {
            return None;
        }
        let state = self.layout.presentation.as_ref()?;
        if !state.others_visible() {
            return None;
        }
        match state.selected_tab() {
            Some(crate::OthersTab::Container(id))
                if id != state.focus_id()
                    && state.others_ids().iter().any(|candidate| candidate == id) =>
            {
                Some(id.as_str())
            }
            _ => None,
        }
    }

    pub(crate) fn close_menu_title(&self) -> Option<&'static str> {
        self.closable_active_tab().map(|_| "Close Tab")
    }

    pub(crate) fn close_active_tab(&mut self) -> Result<bool> {
        let Some(id) = self.closable_active_tab().map(str::to_owned) else {
            return Ok(false);
        };
        if self.composition_in_progress() {
            host_diag!("PRESENTATION_REJECTED composition-in-progress");
            return Ok(true);
        }
        if let Some(header) = &mut self.layout.presentation_tabs {
            header.event(&ContentEvent::Cancel);
        }
        if let Some(grid) = &mut self.layout.presentation_creation_grid {
            grid.host.event(&ContentEvent::Cancel);
        }
        self.cancel_toolbar_input();
        self.layout.drag_split = None;
        self.layout.outer_view.cancel(&mut self.layout.outer);
        self.layout.inner_view.cancel(&mut self.layout.inner);
        self.close_container(&id)?;
        if let Some(state) = self.layout.presentation.as_ref() {
            host_diag!(
                "PRESENTATION_CHANGED focus={} others={:?}",
                state.focus_id(),
                state.others()
            );
        }
        Ok(true)
    }
}

impl WindowHandler for Host {
    fn allows_app_quit(&self) -> bool {
        self.settings_mgr.settings.as_ref().is_none_or(|s| {
            let s = s.borrow();
            s.clean_for_close() || s.failed_close_prompted
        })
    }

    fn close_menu_title(&self) -> Option<&'static str> {
        Host::close_menu_title(self)
    }

    fn handle_close_shortcut(&mut self, w: &mut Window) -> bool {
        match self.close_active_tab() {
            Ok(true) => {
                if let Err(e) = self.draw(w) {
                    host_diag!("HOST_ERROR {e:#}");
                    w.set_title(&format!("AppHost error: {e}"));
                }
                true
            }
            Ok(false) => false,
            Err(e) => {
                host_diag!("HOST_ERROR {e:#}");
                w.set_title(&format!("AppHost error: {e}"));
                true
            }
        }
    }

    fn handle(&mut self, event: Event, w: &mut Window) {
        // The existing observer reports actual drawable presentation on a
        // separate CA clock. Join by window/serial; do not subtract Instant.
        // Flush outside a drag so copying receipts cannot perturb its timing.
        if self.diagnostics.draw_diagnostic
            && matches!(&event, Event::CloseRequested | Event::Focus(false))
        {
            eprintln!(
                "APPHOST_SCREEN_DIAGNOSTIC pid={} window={} native_frame={:?} screen_id={:?} screen_info={:?} scale={} size={:?}",
                std::process::id(),
                w.native_window_number(),
                w.native_window_frame(),
                w.native_screen_id(),
                w.screen_info(),
                w.scale(),
                w.size(),
            );
            eprintln!(
                "APPHOST_PRESENT_DIAGNOSTIC pid={} window={} clock=CACurrentMediaTime drawable_serial={} acquisition_counts={:?} frames={:?}",
                std::process::id(),
                w.native_window_number(),
                w.drawable_serial(),
                w.drawable_acquisition_counts(),
                w.presented_frames(),
            );
        }
        let diagnostic = self.diagnostics.draw_diagnostic.then(|| {
            self.diagnostics.diagnostic_event += 1;
            let begin = std::time::Instant::now();
            // MouseEvent does not expose NSEvent.timestamp. This marks receipt
            // at the Host handler, after the platform's event queue.
            let kind = match &event {
                Event::Mouse(mouse) => format!(
                    "Mouse::{:?}@{},{}",
                    mouse.kind, mouse.position.x, mouse.position.y
                ),
                Event::Scroll(_) => "Scroll".into(),
                Event::Key(_) | Event::TextKey(_) => "Key".into(),
                Event::Accessibility { .. } => "AX".into(),
                Event::Created => "Created".into(),
                Event::Resized { .. } => "Resized".into(),
                Event::Redraw => "Redraw".into(),
                Event::Wake => "Wake".into(),
                Event::AnimationFrame(_) => "AnimationFrame".into(),
                _ => "Other".into(),
            };
            (
                begin,
                self.diagnostics.diagnostic_event,
                self.diagnostics.diagnostic_draw,
                kind,
            )
        });
        let acceptance_tick = matches!(
            &event,
            Event::Created | Event::Redraw | Event::Wake | Event::AnimationFrame(_)
        );
        let result = self.event(event, w).and_then(|()| {
            if acceptance_tick && self.renderer.is_some() {
                self.accept(w)
            } else {
                Ok(())
            }
        });
        if let Some((begin, event, draws_before, kind)) = diagnostic {
            let end = std::time::Instant::now();
            eprintln!(
                "APPHOST_EVENT_DIAGNOSTIC pid={} window={} event={} kind={kind:?} begin_monotonic={begin:?} end_monotonic={end:?} draws_before={} draws_after={} elapsed_ns={} ok={} native_input_timestamp=unavailable",
                std::process::id(),
                w.native_window_number(),
                event,
                draws_before,
                self.diagnostics.diagnostic_draw,
                end.duration_since(begin).as_nanos(),
                result.is_ok(),
            );
        }
        if let Err(e) = result {
            host_diag!("HOST_ERROR {e:#}");
            #[cfg(feature = "window-acceptance")]
            if let Some(shared) = &self.diagnostics.acceptance_shared {
                shared.fail(format!("HOST_ERROR {e:#}"));
                if let Ok(result) = shared.0.lock()
                    && let Some(wake) = &result.parent_wake
                {
                    let _ = wake.wake();
                }
            }
            w.set_title(&format!("AppHost error: {e}"));
            #[cfg(feature = "window-acceptance")]
            if let Some(probe) = &self.diagnostics.presentation_acceptance {
                probe.shared.fail(format!("{e:#}"));
                eprintln!(
                    "PRESENTATION_ACCEPTANCE_FAIL step={} draws={} automated_native=true physical_input=false error={e:#}",
                    self.diagnostics.step, self.diagnostics.acceptance_draws
                );
            }
            #[cfg(feature = "window-acceptance")]
            if self.diagnostics.acceptance_shared.is_some()
                || self.diagnostics.presentation_acceptance.is_some()
            {
                w.close();
            }
        }
    }
}
impl Host {
    pub(crate) fn configure_settings(
        &mut self,
        settings: Option<crate::settings::SettingsConfiguration>,
    ) -> Result<()> {
        let owner = match settings {
            Some(crate::settings::SettingsConfiguration::Window(options)) => {
                Some(crate::settings::SettingsOwner::new(options)?)
            }
            Some(crate::settings::SettingsConfiguration::Shared(owner)) => Some(owner),
            None => None,
        };
        self.settings_mgr.settings_options =
            owner.as_ref().map(|owner| owner.0.borrow().options.clone());
        self.settings_mgr.settings_display = owner
            .as_ref()
            .and_then(|owner| window_settings::SettingsDisplayBinding::new(owner, self.theme));
        self.settings_mgr.settings_owner = owner;
        Ok(())
    }
    pub(crate) fn menu_bar(&self) -> MenuBar {
        let mut menus = MenuBar::default();
        if self.settings_mgr.settings_options.is_some() || self.settings_mgr.is_settings_window {
            menus.menus.push(
                tekes_ui_platform_macos::Menu::new(
                    "Settings",
                    vec![tekes_ui_platform_macos::MenuItem::Command(
                        tekes_ui_platform_macos::MenuCommand::new(
                            "Settings…",
                            tekes_ui_platform_macos::CommandId(0x534554),
                        )
                        .key(tekes_ui_platform_macos::KeyEquivalent::command(',')),
                    )],
                )
                .in_application_menu(),
            );
        }
        if self.layout.presentation.is_some() {
            menus.menus.push(self.presentation_menu());
        }
        menus
    }
    pub(crate) fn open(
        mut self,
        parent: &Window,
        config: WindowConfig,
        settings: Option<crate::settings::SettingsConfiguration>,
    ) -> Result<tekes_ui_platform_macos::WindowHandle> {
        self.mount_initial_containers();
        self.configure_settings(settings)?;
        self.menu_bar().validate()?;
        Ok(parent.open_window(config, self))
    }

    fn arrange_column_frames(&mut self, whole: Rect) -> Vec<Rect> {
        #[cfg(test)]
        {
            self.diagnostics.column_layout_calls += 1;
        }
        let frames = if self.panel_mgr.panels.len() == 1 {
            self.layout
                .outer_view
                .arrange(&mut self.layout.outer, Rect::default(), None);
            self.layout
                .inner_view
                .arrange(&mut self.layout.inner, Rect::default(), None);
            vec![whole]
        } else if self.panel_mgr.panels.len() == 2 {
            let layout = self
                .layout
                .outer_view
                .arrange(&mut self.layout.outer, whole, None);
            self.layout
                .inner_view
                .arrange(&mut self.layout.inner, Rect::default(), None);
            vec![layout.leading.frame(), layout.trailing.frame()]
        } else {
            let a = self
                .layout
                .outer_view
                .arrange(&mut self.layout.outer, whole, None);
            let b = self
                .layout
                .inner_view
                .arrange(&mut self.layout.inner, whole, None);
            vec![
                a.leading.frame(),
                Rect::new(
                    a.trailing.frame().min_x(),
                    0.,
                    (b.leading.frame().max_x() - a.trailing.frame().min_x()).max(0.),
                    whole.size.height,
                ),
                b.trailing.frame(),
            ]
        };
        if (self.layout.drag_split == Some(0) && !self.layout.outer_view.is_dragging())
            || (self.layout.drag_split == Some(1) && !self.layout.inner_view.is_dragging())
        {
            self.layout.drag_split = None;
        }
        frames
    }

    fn paint_panel_body(
        &mut self,
        index: usize,
        active: bool,
        scale: f32,
        palette: Palette,
        scene: &mut Scene,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let is_nav = self.layout.presentation.as_ref().is_some_and(|state| {
            state.nav_bar_id() == Some(self.panel_mgr.panels[index].module_id.as_str())
        });
        let body_surface = if is_nav { "sidebar" } else { "panel" };
        let p = &mut self.panel_mgr.panels[index];
        let content = p.content_frame();
        if self.layout.presentation.is_some() {
            scene.push_clip(p.clip);
        }
        scene.push_quad(Quad::new(
            p.geometry.content,
            surface_ground(palette, body_surface),
        ));
        if self.layout.presentation.is_some() {
            scene.pop_clip();
        }
        if let Some(local) = &mut p.local {
            let clip = if self.layout.presentation.is_some() {
                content.intersection(&p.clip).unwrap_or_default()
            } else {
                content
            };
            local.set_active(active);
            let context = crate::ContainerRenderContext {
                renderer,
                frame: content,
                clip,
                scale: self.text.scale,
                motion: tekes_ui_controls::ButtonTextRenderer::motion_context(&self.text),
                palette,
            };
            render_local_content(local, &mut p.content_icons, &mut self.text, scene, context)?;
            p.frame_seq = p.seq;
            p.frame_viewport = p.viewport;
            p.frame_size = p.content_size();
        }
        if let Some(id) = p.image {
            scene.push_clip(if self.layout.presentation.is_some() {
                content.intersection(&p.clip).unwrap_or_default()
            } else {
                content
            });
            scene.push_image(ImageQuad {
                rect: tekes_ui_module_runtime::panel_image_rect(
                    content.origin,
                    p.frame_pixels,
                    scale,
                ),
                image: id,
                opacity: 1.,
            });
            scene.pop_clip();
        }
        Ok(())
    }

    fn render_toolbar_hosts(
        &mut self,
        active: bool,
        palette: Palette,
        renderer: &mut Renderer,
        toolbar_scene: &mut Scene,
        toolbar_overlays: &mut Scene,
    ) -> Result<()> {
        let toolbar_order: Vec<_> = if let Some(state) = &self.layout.presentation {
            let focus = self
                .panel_mgr
                .panels
                .iter()
                .position(|panel| panel.module_id == state.focus_id())
                .unwrap();
            (0..self.panel_mgr.panels.len())
                .filter(|index| *index != focus)
                .chain(std::iter::once(focus))
                .collect()
        } else {
            (0..self.panel_mgr.panels.len()).collect()
        };
        for pass in 0..2 {
            if pass == 1
                && !self
                    .panel_mgr
                    .panels
                    .iter()
                    .any(|panel| panel.mounted_toolbar.host.needs_render())
            {
                break;
            }
            if pass == 1 {
                *toolbar_scene = Scene::new();
                *toolbar_overlays = Scene::new();
            }
            for &index in &toolbar_order {
                let panel = &mut self.panel_mgr.panels[index];
                with_content_icons(
                    renderer,
                    &mut panel.mounted_toolbar.icons,
                    &mut self.text,
                    |text| {
                        panel
                            .mounted_toolbar
                            .host
                            .render_layers(toolbar_scene, toolbar_overlays, text, palette)
                            .map_err(|e| anyhow::anyhow!("container toolbar paint: {e:?}"))
                    },
                )?;
            }
            if pass == 0 {
                self.reconcile_toolbar_focus(active);
            }
        }
        Ok(())
    }

    pub(crate) fn set_reduced_motion(&mut self, reduced: bool) {
        self.layout.reduced_motion = reduced;
    }

    pub(crate) fn set_headless_motion_elapsed(&mut self, elapsed: std::time::Duration) {
        self.layout.headless_motion_elapsed = Some(elapsed);
    }

    pub(crate) fn build_headless_scene(
        &mut self,
        renderer: &mut Renderer,
        whole: Rect,
        scale: f32,
        active: bool,
    ) -> Result<(Scene, tekes_ui_tokens::Rgba)> {
        self.mount_initial_containers();
        ensure!(
            self.panel_mgr.panels.iter().all(|p| p.local.is_some()),
            "headless rendering requires local containers only"
        );
        if let Some(view_command) = self
            .nav_mgr
            .navigation_options
            .as_ref()
            .and_then(|options| options.view_command)
        {
            while let Some(entry) = take_local_navigation_request(&mut self.panel_mgr.panels, true)
            {
                let command = view_command(&entry)?;
                let targets: Vec<usize> = if let Some(state) = &self.layout.presentation {
                    state
                        .navigation_targets()
                        .iter()
                        .filter_map(|id| {
                            self.panel_mgr
                                .module_ids
                                .iter()
                                .position(|candidate| candidate == id)
                        })
                        .collect()
                } else {
                    vec![self.layout.order[usize::from(self.panel_mgr.panels.len() >= 2)]]
                };
                for index in targets {
                    self.panel_mgr.panels[index].enqueue(
                        InputKind::Command,
                        0.,
                        0.,
                        Some(command),
                    )?;
                }
            }
        }
        let now = self
            .layout
            .headless_motion_elapsed
            .unwrap_or_else(|| self.layout.motion_epoch.elapsed());
        self.text.begin_frame(now, self.layout.reduced_motion);
        self.text.scale = scale;
        self.layout.outer.min_pane = MIN_COLUMN;
        self.layout.inner.min_pane = MIN_COLUMN;
        let row_height = tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text);
        let (_, toolbar_y, toolbar_paint_height) =
            window_toolbar_geometry(0., row_height, None, scale);
        let content_boundary = toolbar_y + toolbar_paint_height;
        self.layout.container_window_boundary = Some(content_boundary);

        let palette = self.palette();
        let clear = surface_ground(palette, "canvas");
        let mut scene = Scene::new();
        scene.push_quad(Quad::new(whole, clear));
        let frames = if self.layout.presentation.is_some() {
            self.presentation_frames_with_safe_area(
                whole,
                content_boundary,
                window_toolbar_mount::ToolbarBand {
                    y: toolbar_y,
                    height: toolbar_paint_height,
                    native_leading: 0.,
                },
                0.,
                0.,
            )?
        } else {
            self.arrange_column_frames(whole)
        };
        for (slot, bounds) in frames.iter().enumerate() {
            if bounds.size.width > 0. {
                let index = self.layout.order[slot];
                let geometry =
                    self.container_geometry_with_icons(index, *bounds, content_boundary, true)?;
                self.panel_geometry_layout(index, *bounds, scale, geometry)?;
            }
        }
        for (slot, bounds) in frames.iter().enumerate() {
            let index = self.layout.order[slot];
            if bounds.size.width <= 0. {
                self.retire_hidden_content(index, *bounds);
                continue;
            }
            self.paint_panel_body(index, active, scale, palette, &mut scene, renderer)?;
        }
        if self.layout.presentation.is_some() {
            scene.push_quad(Quad::new(
                Rect::new(
                    whole.min_x(),
                    toolbar_y,
                    whole.size.width,
                    toolbar_paint_height,
                ),
                surface_ground(palette, "toolbar"),
            ));
            self.sync_container_toolbars_with_renderer(
                active,
                whole,
                window_toolbar_mount::ToolbarBand {
                    y: toolbar_y,
                    height: toolbar_paint_height,
                    native_leading: 0.,
                },
                0.,
                Some(renderer),
            )?;
        }
        let mut toolbar_scene = Scene::new();
        let mut toolbar_overlays = Scene::new();
        let mut presentation_scene = Scene::new();
        self.render_toolbar_hosts(
            active,
            palette,
            renderer,
            &mut toolbar_scene,
            &mut toolbar_overlays,
        )?;
        if self.layout.presentation.is_some() {
            self.paint_presentation_with_renderer(
                &mut presentation_scene,
                whole,
                palette,
                Some(renderer),
            )?;
        }
        self.append_toolbar_layers(
            &mut scene,
            &toolbar_scene,
            &presentation_scene,
            &toolbar_overlays,
        );
        self.layout.presentation_scroll_pending = false;
        Ok((scene, clear))
    }

    pub(crate) fn render_headless_rgba8(
        &mut self,
        renderer: &mut Renderer,
        size_pt: (u32, u32),
        scale: u32,
    ) -> Result<Vec<u8>> {
        let whole = Rect::new(0., 0., size_pt.0 as f32, size_pt.1 as f32);
        let (scene, clear) = self.build_headless_scene(renderer, whole, scale as f32, true)?;
        let size_px = (size_pt.0 * scale, size_pt.1 * scale);
        Ok(renderer.render_to_rgba8(&scene, &mut self.text.atlas, size_px, scale as f32, clear)?)
    }
}

pub(crate) fn run(mut app: crate::AppWindow, acceptance: bool) -> Result<()> {
    let app_name = app
        .app_name
        .clone()
        .unwrap_or_else(|| "TekesUI AppHost Prototype".into());
    let window_title = app
        .window_title
        .clone()
        .or_else(|| app.app_name.clone())
        .unwrap_or_else(|| "TekesUI AppHost Prototype".into());
    let quit_after_last_window_closed = app.quit_after_last_window_closed.unwrap_or(acceptance);
    let settings = app.settings.take();
    let mut host = app.into_host(acceptance);
    ensure!(
        host.layout.presentation.is_some() || (1..=3).contains(&host.panel_mgr.module_ids.len()),
        "expected one, two or three preselected modules"
    );
    host.mount_initial_containers();
    #[cfg(feature = "window-acceptance")]
    {
        let has_settings = settings.is_some();
        let settings_window_acceptance = acceptance
            && has_settings
            && std::env::var("TEKESUI_SETTINGS_WINDOW_ACCEPTANCE").as_deref() == Ok("1");
        if settings_window_acceptance {
            host.diagnostics.acceptance_shared = Some(window_acceptance::Shared::new());
        }
        let presentation_acceptance = acceptance
            && std::env::var("TEKESUI_PRESENTATION_WINDOW_ACCEPTANCE").as_deref() == Ok("1");
        ensure!(
            !presentation_acceptance || host.layout.presentation.is_some(),
            "presentation acceptance requires from_containers"
        );
        ensure!(
            !(presentation_acceptance && settings_window_acceptance),
            "choose one finite window acceptance scope"
        );
        if presentation_acceptance {
            host.diagnostics.presentation_acceptance =
                Some(window_acceptance::PresentationAcceptance::new());
        }
    }
    #[cfg(not(feature = "window-acceptance"))]
    ensure!(
        !acceptance,
        "window acceptance requires building tekes-ui-app-host with feature \"window-acceptance\""
    );
    host.configure_settings(settings)?;
    let menus = host.menu_bar();
    App::new(AppConfig {
        name: Some(app_name),
        quit_after_last_window_closed,
    })
    .run(
        WindowConfig {
            title: window_title,
            unified_titlebar: true,
            placement: tekes_ui_platform_macos::WindowPlacement::Auto {
                need_120hz: false,
                size_check: true,
            },
            size: WindowSize {
                width: 1100.,
                height: 640.,
            },
            min_size: Some(WindowSize {
                width: 600.,
                height: 320.,
            }),
            ..Default::default()
        },
        menus,
        host,
    );
    Ok(())
}

fn toolbar_pointer(m: MouseEvent) -> ContentEvent {
    let button = |b| match b {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Other(n) => PointerButton::Other(n),
    };
    let (phase, pressed, click_count) = match m.kind {
        MouseEventKind::Down {
            button: b,
            click_count,
        } => (PointerPhase::Down, Some(button(b)), click_count),
        MouseEventKind::Up {
            button: b,
            click_count,
        } => (PointerPhase::Up, Some(button(b)), click_count),
        MouseEventKind::Dragged { button: b } => (PointerPhase::Drag, Some(button(b)), 0),
        MouseEventKind::Moved => (PointerPhase::Move, None, 0),
        MouseEventKind::Entered => (PointerPhase::Enter, None, 0),
        MouseEventKind::Exited => (PointerPhase::Exit, None, 0),
    };
    ContentEvent::Pointer(PointerEvent {
        position: tekes_ui_scene::Point::new(m.position.x as f32, m.position.y as f32),
        phase,
        button: pressed,
        click_count,
        modifiers: PointerModifiers {
            shift: m.modifiers.shift,
            control: m.modifiers.control,
            alt: m.modifiers.alt,
            command: m.modifiers.command,
            caps_lock: m.modifiers.caps_lock,
            function: m.modifiers.function,
        },
    })
}

fn toolbar_key(k: KeyEvent) -> ContentKeyEvent {
    ContentKeyEvent {
        key: match k.key_code {
            36 | 76 => Some(ControlKey::Enter),
            49 => Some(ControlKey::Space),
            48 => Some(ControlKey::Tab {
                reverse: k.modifiers.shift,
            }),
            53 => Some(ControlKey::Escape),
            123 => Some(ControlKey::Left),
            124 => Some(ControlKey::Right),
            125 => Some(ControlKey::Down),
            126 => Some(ControlKey::Up),
            115 => Some(ControlKey::Home),
            119 => Some(ControlKey::End),
            _ => None,
        },
        down: k.kind == KeyEventKind::Down,
        repeat: k.is_repeat,
        modifiers: InputModifiers {
            shift: k.modifiers.shift,
            control: k.modifiers.control,
            alt: k.modifiers.alt,
            command: k.modifiers.command,
            caps_lock: k.modifiers.caps_lock,
            function: k.modifiers.function,
        },
        native_code: k.key_code,
        characters: k.characters,
        characters_ignoring_modifiers: k.characters_ignoring_modifiers,
    }
}

#[cfg(test)]
mod toolbar_geometry_tests {
    use super::*;
    #[test]
    fn explicit_zero_container_padding_leaves_one_real_content_padding_owner() {
        use tekes_ui_controls::composition::{Content, ListContent, TextContent};
        let padding = tekes_ui_tokens::Spacing::Loose.value();
        let local = crate::LocalContainer::new(
            "pane",
            Content::new(
                ComponentId(40),
                ListContent::scrollable(
                    "Body",
                    ComponentId(41),
                    vec![Content::new(
                        ComponentId(42),
                        TextContent::new("Actual body"),
                    )],
                )
                .content_padding(padding),
            ),
        )
        .unwrap()
        .content_padding(0.);
        let mut host = crate::AppWindow::from_containers(
            vec![local.into()],
            crate::PresentationOptions::new("pane"),
        )
        .unwrap()
        .into_host(false);
        host.mount_columns();
        assert_eq!(host.panel_mgr.panels[0].content_padding, 0.);
        let bounds = Rect::new(0., 0., 300., 260.);
        host.panel_geometry(0, bounds, 1., 48., 0.).unwrap();
        let frame = host.panel_mgr.panels[0].content_frame();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                let items = content.get(ComponentId(41)).unwrap().frame();
                assert_eq!(items.min_x() - bounds.min_x(), padding);
                assert_eq!(bounds.max_x() - items.max_x(), padding);
                assert_eq!(items.min_y() - frame.min_y(), padding);
                let actual = content.get(ComponentId(42)).unwrap().frame();
                assert_eq!(actual.min_x(), items.min_x());
                assert_eq!(actual.min_y(), items.min_y());
                assert!(actual.max_x() <= items.max_x());
            });
    }
    #[test]
    fn default_container_padding_is_zero_so_scroll_list_owns_single_document_inset() {
        use tekes_ui_controls::composition::{Content, ListContent, TextContent};
        let padding = tekes_ui_tokens::Spacing::Loose.value();
        let local = crate::LocalContainer::new(
            "pane",
            Content::new(
                ComponentId(40),
                ListContent::scrollable(
                    "Body",
                    ComponentId(41),
                    vec![Content::new(
                        ComponentId(42),
                        TextContent::new("Actual body"),
                    )],
                ),
            ),
        )
        .unwrap();
        let mut host = crate::AppWindow::from_containers(
            vec![local.into()],
            crate::PresentationOptions::new("pane"),
        )
        .unwrap()
        .into_host(false);
        host.mount_columns();
        assert_eq!(host.panel_mgr.panels[0].content_padding, 0.);
        let bounds = Rect::new(0., 0., 300., 260.);
        host.panel_geometry(0, bounds, 1., 48., 0.).unwrap();
        let frame = host.panel_mgr.panels[0].content_frame();
        assert_eq!(frame.min_x(), bounds.min_x());
        assert_eq!(frame.max_x(), bounds.max_x());
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut host.text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        host.panel_mgr.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |content| {
                let items = content.get(ComponentId(41)).unwrap().frame();
                assert_eq!(items.min_x() - bounds.min_x(), padding);
                assert_eq!(bounds.max_x() - items.max_x(), padding);
                assert_eq!(items.min_y() - frame.min_y(), padding);
                let actual = content.get(ComponentId(42)).unwrap().frame();
                assert_eq!(actual.min_x(), items.min_x());
                assert_eq!(actual.min_y(), items.min_y());
                assert!(actual.max_x() <= items.max_x());
            });
    }
    #[test]
    fn ordinary_others_uses_one_row_and_one_padding_for_local_and_wire_geometry() {
        let mut panel = Panel::new(0, "hidden-worker".into());
        panel.content_padding = tekes_ui_tokens::Spacing::Loose.value();
        let bounds = Rect::new(734., 0., 366., 640.);
        panel.geometry_with_header(bounds, 1., 52., 0.).unwrap();
        let generation = panel.viewport;
        assert_eq!(panel.content_inset(), 52.);
        let content = panel.content_frame();
        assert_eq!(content, Rect::new(750., 68., 334., 556.));
        assert_eq!(content.min_x() - bounds.min_x(), 16.);
        assert_eq!(bounds.max_x() - content.max_x(), 16.);
        assert_eq!(
            content.min_y() - (bounds.min_y() + panel.content_inset()),
            16.
        );
        assert_eq!(bounds.max_y() - content.max_y(), 16.);
        assert_eq!(panel.content_size(), (334, 556));
        assert_eq!(panel.content_point(content.origin), Point::default());
        assert_eq!(
            panel.content_point(Point::new(content.max_x(), content.max_y())),
            Point::new(334., 556.)
        );
        // No worker is launched by this CPU fixture. Verify the exact wire
        // serialization at the production queue boundary, not request()'s
        // deliberately unopened launch gate.
        panel.enqueue(InputKind::Layout, 0., 0., None).unwrap();
        let queued = panel.queued_requests();
        let request = queued.last().unwrap();
        assert_eq!((request.width, request.height), panel.content_size());
        panel.local = Some(
            crate::LocalContainer::new(
                "hidden-worker",
                tekes_ui_controls::composition::Content::new(
                    ComponentId(1),
                    tekes_ui_controls::composition::TextContent::new("Last line"),
                ),
            )
            .unwrap(),
        );
        panel
            .local
            .as_mut()
            .unwrap()
            .render(
                &mut Scene::new(),
                &mut Text::new(),
                content,
                content,
                Theme::LIGHT.into(),
            )
            .unwrap();
        panel
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| assert_eq!(host.get(ComponentId(1)).unwrap().frame(), content));
        panel.geometry_with_header(bounds, 1., 52., 0.).unwrap();
        assert_eq!(panel.viewport, generation);
        panel.geometry_with_header(bounds, 1., 66., 0.).unwrap();
        assert!(panel.viewport > generation);
        assert_eq!(content_size(Rect::default(), 0.), (1, 1));
        let legacy = Panel::new(1, "legacy".into());
        assert_eq!(legacy.content_padding, 0.);
    }
    #[test]
    fn native_safety_allocation_does_not_add_blank_space_below_any_top_toolbar() {
        let (allocation, y, paint) = window_toolbar_geometry(66., 42., Some(24.), 2.);
        assert_eq!(allocation, 66.);
        assert_eq!(y + paint, 45.);
        let controls_floor: f32 = 2. * 24.;
        let row_boundary = (y + paint).max(controls_floor);
        let boundary = presentation_content_inset(42., row_boundary, controls_floor, true, false);
        assert_eq!(boundary, 48.);
        assert_eq!(
            presentation_content_inset(42., row_boundary, controls_floor, true, true),
            boundary
        );
        let mut panel = Panel::new(0, "focus".into());
        panel.bounds = Rect::new(300., 0., 400., 640.);
        panel.geometry =
            window_container_geometry::ContainerGeometry::legacy(panel.bounds, boundary);
        panel.content_padding = tekes_ui_tokens::Spacing::Loose.value();
        assert_eq!(panel.content_frame().min_y() - row_boundary, 16.);
        // Reproduce the observed 120% typography while AppKit changes only its
        // total allocation: Nav, Focus and Others keep the same content origin.
        for native_height in [51., 62., 66.] {
            let (allocation, y, paint) =
                window_toolbar_geometry(native_height, 50.4, Some(25.2), 2.);
            assert_eq!(allocation, native_height);
            let boundary = (y + paint).max(2. * 25.2);
            for intersects in [false, true] {
                assert_eq!(
                    presentation_content_inset(50.4, boundary, 50.4, true, intersects),
                    50.4
                );
            }
        }
        assert_eq!(presentation_content_inset(0., 48., 48., true, true), 48.);
        assert_eq!(presentation_content_inset(0., 48., 48., true, false), 0.);
        assert_eq!(presentation_content_inset(0., 0., 0., true, false), 0.);
    }
    #[test]
    fn navigation_gate_preserves_rapid_consumer_requests_and_original_wire_geometry() {
        use std::{cell::RefCell, collections::VecDeque, rc::Rc};
        struct Pending {
            host: ContentHost,
            entries: Rc<RefCell<VecDeque<crate::navigation::Entry>>>,
        }
        impl crate::ContainerContent for Pending {
            fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
                visit(&mut self.host);
            }
            fn take_navigation_request(&mut self) -> Option<crate::navigation::Entry> {
                self.entries.borrow_mut().pop_front()
            }
        }
        let entries = Rc::new(RefCell::new(
            (0..2)
                .map(|index| crate::navigation::Entry {
                    id: format!("doc{index}"),
                    route: "doc".into(),
                    params: serde_json::json!({"index":index}),
                })
                .collect::<VecDeque<_>>(),
        ));
        let source = Pending {
            host: ContentHost::new(tekes_ui_controls::composition::Content::new(
                ComponentId(1),
                tekes_ui_controls::composition::TextContent::new("Nav"),
            ))
            .unwrap(),
            entries: entries.clone(),
        };
        let mut host = Host::from_columns(
            vec![crate::LocalContainer::from_source("nav", source).into()],
            false,
            None,
        );
        host.mount_columns();
        assert!(take_local_navigation_request(&mut host.panel_mgr.panels, false).is_none());
        assert_eq!(entries.borrow().len(), 2);
        assert_eq!(
            take_local_navigation_request(&mut host.panel_mgr.panels, true)
                .unwrap()
                .id,
            "doc0"
        );
        assert!(take_local_navigation_request(&mut host.panel_mgr.panels, false).is_none());
        assert_eq!(
            take_local_navigation_request(&mut host.panel_mgr.panels, true)
                .unwrap()
                .id,
            "doc1"
        );
        assert!(take_local_navigation_request(&mut host.panel_mgr.panels, true).is_none());
        let outer = Rect::new(20., 30., 400., 300.);
        let inner = content_geometry(outer, 48.);
        assert_eq!(inner, Rect::new(20., 78., 400., 252.));
        assert_eq!(content_size(outer, 48.), (400, 252));
        assert_eq!(content_point(outer, 48., inner.origin), Point::default());
        assert_eq!(
            content_point(outer, 48., Point::new(inner.max_x(), inner.max_y())),
            Point::new(400., 252.)
        );
    }
    #[test]
    fn compact_presentation_reserves_native_controls_once_and_no_empty_toolbar_rows() {
        assert_eq!(presentation_content_inset(48., 56., 40., true, true), 56.);
        assert_eq!(presentation_content_inset(48., 56., 40., true, false), 56.);
        assert_eq!(presentation_content_inset(0., 56., 40., true, true), 40.);
        assert_eq!(presentation_content_inset(0., 56., 40., true, false), 0.);
        assert_eq!(presentation_content_inset(0., 56., 40., false, false), 0.);
        assert_eq!(
            presentation_content_inset(48., 56., 40., false, false),
            48.,
            "List/internal rows use ordinary measured height"
        );
    }
    #[test]
    fn dormant_panel_has_no_process_and_drops_pre_mount_input() {
        let mut p = Panel::new(0, "demo-first".into());
        assert!(p.process.pid().is_none());
        assert!(p.needs_launch());
        p.request(InputKind::Cancel, 0., 0.).unwrap();
        p.request(InputKind::PointerDown, 20., 20.).unwrap();
        assert!(p.queued_requests().is_empty());
        assert_eq!(p.seq, 0);
        assert!(p.process.pid().is_none());
    }
    #[test]
    fn side_visibility_expands_center_and_keeps_unrelated_column_fixed() {
        let whole = Rect::new(0., 0., 1100., 640.);
        let original = vec![
            Rect::new(0., 0., 360., 640.),
            Rect::new(364., 0., 372., 640.),
            Rect::new(740., 0., 360., 640.),
        ];
        let left_off = visible_frames(&original, whole, false, true);
        assert_eq!(left_off[0], Rect::default());
        assert_eq!(left_off[2], original[2]);
        assert_eq!(left_off[1], Rect::new(0., 0., 736., 640.));
        let right_off = visible_frames(&original, whole, true, false);
        assert_eq!(right_off[0], original[0]);
        assert_eq!(right_off[2], Rect::default());
        assert_eq!(right_off[1], Rect::new(364., 0., 736., 640.));
        assert_eq!(visible_frames(&original, whole, false, false)[1], whole);
        assert_eq!(visible_frames(&original, whole, true, true), original);
    }
    #[test]
    fn animated_side_geometry_keeps_opposite_fixed_and_closes_without_gap() {
        let whole = Rect::new(0., 0., 1100., 640.);
        let base = vec![
            Rect::new(0., 0., 366.3333, 640.),
            Rect::new(367.3333, 0., 365.3334, 640.),
            Rect::new(733.6667, 0., 366.3333, 640.),
        ];
        for progress in [0., 0.1, 0.5, 0.9, 1.] {
            let left = presented_frames(&base, whole, [progress, 1.], 2.);
            assert_eq!(left[2], base[2]);
            let right = presented_frames(&base, whole, [1., progress], 2.);
            assert_eq!(right[0], base[0]);
            for frames in [left, right] {
                assert!(frames[0].max_x() <= frames[1].min_x());
                assert!(frames[1].max_x() <= frames[2].min_x() || frames[2] == Rect::default());
                for r in frames {
                    assert!(r.size.width >= 0.);
                }
            }
        }
        assert_eq!(presented_frames(&base, whole, [0., 0.], 2.)[1], whole);
    }
    #[test]
    fn unified_height_changes_content_viewport_and_all_pointer_coordinates_together() {
        for x in [0., 366., 734.] {
            let panel = Rect::new(x, 0., 365., 640.);
            for bar in [38., 52., 66., 88.] {
                let content = content_geometry(panel, bar);
                assert_eq!(content.size.height, 640. - bar);
                assert_eq!(content.size.width, 365.);
                let point = Point::new(x + 40., bar + 32.);
                assert_eq!(content_point(panel, bar, point), Point::new(40., 32.));
                assert!(content.contains(point));
            }
        }
    }
}

#[cfg(test)]
mod native_toolbar_alignment_tests {
    #[test]
    fn shared_row_uses_native_center_and_content_keeps_native_floor() {
        assert_eq!(
            super::window_toolbar_geometry(66., 48., Some(40.), 2.),
            (66., 16., 48.)
        );
        assert_eq!(
            super::window_toolbar_geometry(40., 32., Some(20.), 1.),
            (40., 4., 32.)
        );
        assert_eq!(
            super::window_toolbar_geometry(0., 48., None, 2.),
            (48., 0., 48.)
        );
        assert_eq!(
            super::window_toolbar_geometry(66., 80., Some(40.), 2.),
            (80., 0., 80.)
        );
        assert_eq!(
            super::window_toolbar_geometry(65.75, 12.25, Some(10.), 2.),
            (66., 3.875, 12.25)
        );
    }
    #[test]
    fn row_and_divider_remain_aligned_at_fractional_font_scales() {
        use tekes_ui_scene::Rect;
        for row in [12.25, 27.5, 40., 61.25] {
            let center = 8_f32.max(row * 0.5);
            let (top, y, h) = super::window_toolbar_geometry(16., row, Some(center), 2.);
            assert_eq!(y + h * 0.5, center);
            assert!(top >= y + h);
            let divider = super::toolbar_divider_frame(Rect::new(200., 0., 1., 640.), y);
            assert_eq!(divider.min_y(), y);
            assert_eq!(divider.max_y(), 640.);
        }
    }
}

#[cfg(test)]
mod two_column_geometry_tests {
    use super::*;
    #[test]
    fn leading_center_has_two_real_slots_and_visibility_keeps_right_edge_fixed() {
        let whole = Rect::new(0., 0., 900., 640.);
        let frames = [
            Rect::new(0., 0., 224., 640.),
            Rect::new(225., 0., 675., 640.),
        ];
        for p in [0., 0.25, 0.5, 1.] {
            let displayed = presented_frames(&frames, whole, [p, 0.], 2.);
            assert_eq!(displayed.len(), 2);
            assert_eq!(displayed[1].max_x(), whole.max_x());
            assert_eq!(displayed[1].min_x(), (225. * p * 2.).round() / 2.);
        }
        let hidden = visible_frames(&frames, whole, false, false);
        assert_eq!(hidden[1], whole);
        assert_eq!(hidden[0], Rect::default());
    }
}

#[cfg(test)]
mod window_focus_tests {
    use super::*;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use tekes_ui_controls::{
        ToolbarAction,
        composition::{
            ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion,
        },
    };
    fn render(h: &mut Host) {
        let palette = h.palette();
        for panel in &mut h.panel_mgr.panels {
            panel
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut h.text, palette)
                .unwrap();
        }
    }
    fn refresh(h: &mut Host, narrow: Option<usize>) {
        let viewport = Rect::new(0., 0., 1000., 400.);
        let mut regions = Vec::new();
        let mut declarations = Vec::new();
        for (order, owner) in h.toolbar_groups().into_iter().enumerate() {
            let (rect, actions, source) = match owner {
                ToolbarTarget::Visibility(side) => (
                    Rect::new(if side == 0 { 0. } else { 820. }, 0., 48., 52.),
                    vec![ToolbarAction {
                        id: if side == 0 { 0x5601 } else { 0x5602 },
                        title: "Show".into(),
                        enabled: true,
                    }],
                    Source::Visibility(side),
                ),
                ToolbarTarget::Panel(i) => {
                    let p = &h.panel_mgr.panels[i];
                    (
                        Rect::new(
                            p.bounds.min_x(),
                            0.,
                            if narrow == Some(i) { 60. } else { 240. },
                            52.,
                        ),
                        p.toolbar
                            .iter()
                            .map(|a| ToolbarAction {
                                id: a.id,
                                title: a.title.clone(),
                                enabled: a.enabled
                                    && (!a.geometry_dependent || p.frame_viewport == p.viewport),
                            })
                            .collect(),
                        Source::Panel {
                            instance: p.instance,
                            index: i,
                            module: p.module_id.clone(),
                            pid: p.pid,
                            generation: p.toolbar_generation,
                        },
                    )
                }
                ToolbarTarget::Navigation
                | ToolbarTarget::Retained { .. }
                | ToolbarTarget::Contribution { .. } => {
                    unreachable!("fixture has no navigation or retained contribution")
                }
            };
            let group = Host::toolbar_group(owner);
            let region = ComponentId(93000 + order as u64);
            regions.push(ToolbarRegion::new(region, rect));
            declarations.push(Declaration {
                id: group,
                region,
                placement: ToolbarPlacement::Navigation,
                order: 0,
                contents: Contents::Commands(
                    actions
                        .into_iter()
                        .map(|a| ToolbarButtonDeclaration {
                            id: a.id,
                            icon: None,
                            text: a.title,
                            hint: None,
                            enabled: a.enabled,
                        })
                        .collect(),
                    ToolbarGroupOverflow::Split,
                ),
                source,
            });
        }
        h.remember_toolbar_focus(None);
        for (index, panel) in h.panel_mgr.panels.iter_mut().enumerate() {
            let owned: Vec<_> = declarations
                .iter()
                .filter(|declaration| match declaration.source {
                    Source::Panel { index: owner, .. } => owner == index,
                    Source::Visibility(0) => index == 0,
                    Source::Visibility(_) => index == 2,
                    _ => index == 0,
                })
                .cloned()
                .collect();
            let regions = regions
                .iter()
                .filter(|region| owned.iter().any(|d| d.region == region.id))
                .cloned()
                .collect();
            panel
                .mounted_toolbar
                .sync(viewport, regions, owned)
                .unwrap();
        }
        render(h);
    }
    fn fixture() -> Host {
        let ids = (0..3).map(|i| format!("window-test-{i}")).collect();
        let mut h = Host::new(ids, false, None);
        h.panel_mgr.panels = (0..3)
            .map(|i| Panel::new(i, format!("window-test-{i}")))
            .collect();
        h.layout.leading_visible = true;
        for (i, p) in h.panel_mgr.panels.iter_mut().enumerate() {
            p.bounds = Rect::new(80. + i as f32 * 240., 0., 240., 300.);
            p.geometry = window_container_geometry::ContainerGeometry::legacy(p.bounds, 66.);
            p.toolbar = vec![
                ToolbarItem {
                    id: 7,
                    title: "Increment".into(),
                    enabled: true,
                    geometry_dependent: false,
                },
                ToolbarItem {
                    id: 8,
                    title: "Reset".into(),
                    enabled: true,
                    geometry_dependent: false,
                },
            ];
        }
        refresh(&mut h, None);
        h
    }
    #[test]
    fn appwindow_theme_reaches_local_body_and_original_toolbar_paint() {
        use tekes_ui_controls::composition::{Content, TextContent};
        use tekes_ui_scene::Primitive;
        let default = crate::AppWindow::new(crate::AppProfile { modules: vec![] }).into_host(false);
        assert_eq!(default.palette(), Theme::LIGHT.into());
        for theme in Theme::ALL {
            let local = crate::LocalContainer::new(
                "theme-body",
                Content::new(ComponentId(1), TextContent::new("Themed body")),
            )
            .unwrap();
            let mut host = crate::AppWindow::from_columns(vec![local.into()])
                .unwrap()
                .theme(theme)
                .into_host(false);
            let palette = host.palette();
            assert_eq!(palette, Palette::from(theme));
            host.mount_columns();
            let mut body = Scene::new();
            let frame = Rect::new(0., 0., 300., 100.);
            host.panel_mgr.panels[0]
                .local
                .as_mut()
                .unwrap()
                .render(&mut body, &mut host.text, frame, frame, palette)
                .unwrap();
            assert!(
                body.primitives().iter().any(|p| matches!(
                    p, Primitive::Glyphs(run) if run.color == palette.color("primary-text").unwrap()
                )),
                "real local content uses {theme:?}"
            );
            let mut toolbar = fixture().with_theme(theme);
            let mut scene = Scene::new();
            for panel in &mut toolbar.panel_mgr.panels {
                panel
                    .mounted_toolbar
                    .host
                    .render(&mut scene, &mut toolbar.text, palette)
                    .unwrap();
            }
            let expected = tekes_ui_tokens::surface_palette::resolve_surface(
                palette,
                "toolbar",
                "action-quiet",
                "rest",
            )
            .unwrap();
            assert!(
                scene.primitives().iter().any(|p| matches!(
                    p, Primitive::Glyphs(run) if run.color == expected.text
                )),
                "real ToolbarHost uses {theme:?}"
            );
        }
    }
    fn key(h: &mut Host, key: ControlKey, down: bool) {
        h.toolbar_key_input(ContentKeyEvent {
            key: Some(key),
            down,
            repeat: false,
            modifiers: Default::default(),
            native_code: 0,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        })
        .unwrap();
    }
    fn commands(h: &Host) -> Vec<(usize, u64)> {
        h.panel_mgr
            .panels
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                p.queued_requests()
                    .into_iter()
                    .filter(|r| r.kind == InputKind::Command)
                    .map(|r| (i, r.command.unwrap()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
    fn focused_groups(h: &Host) -> Vec<ToolbarTarget> {
        h.actual_toolbar_owner().into_iter().collect()
    }
    fn native_drag_mouse(kind: MouseEventKind) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            position: tekes_ui_platform_macos::Point { x: 110., y: 14. },
            modifiers: Default::default(),
        })
    }
    #[test]
    fn native_titlebar_drag_preserves_clean_toolbar_geometry_without_routing_input() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        h.set_toolbar_focus(Some(owner), false);
        render(&mut h);
        let snapshots: Vec<_> = h
            .panel_mgr
            .panels
            .iter()
            .map(|p| {
                assert!(!p.mounted_toolbar.host.needs_render());
                p.mounted_toolbar
                    .host
                    .content_host()
                    .geometry_snapshot()
                    .unwrap()
            })
            .collect();
        let drag = native_drag_mouse(MouseEventKind::Dragged {
            button: MouseButton::Left,
        });
        assert!(!h.owns_native_titlebar_drag_event(&drag));
        // Production assigns this only from try_begin_titlebar_drag's result.
        h.layout.native_titlebar_drag = true;
        for _ in 0..7 {
            assert!(h.owns_native_titlebar_drag_event(&drag));
        }
        for (panel, before) in h.panel_mgr.panels.iter().zip(snapshots) {
            assert!(!panel.mounted_toolbar.host.needs_render());
            assert!(before.is_current());
            assert_eq!(
                panel
                    .mounted_toolbar
                    .host
                    .content_host()
                    .geometry_snapshot()
                    .unwrap()
                    .generation,
                before.generation
            );
        }
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn native_titlebar_drag_releases_without_up_and_preserves_non_drag_events() {
        let mut h = fixture();
        let drag = native_drag_mouse(MouseEventKind::Dragged {
            button: MouseButton::Left,
        });
        for end in [
            native_drag_mouse(MouseEventKind::Moved),
            native_drag_mouse(MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            }),
            native_drag_mouse(MouseEventKind::Up {
                button: MouseButton::Left,
                click_count: 1,
            }),
            Event::Focus(false),
        ] {
            h.layout.native_titlebar_drag = true;
            assert!(!h.owns_native_titlebar_drag_event(&end));
            assert!(!h.owns_native_titlebar_drag_event(&drag));
        }
        h.layout.native_titlebar_drag = true;
        for event in [
            Event::Resized {
                size: WindowSize {
                    width: 900.,
                    height: 500.,
                },
                scale: 2.,
                drawable_size: (1800, 1000),
            },
            Event::Redraw,
            native_drag_mouse(MouseEventKind::Exited),
            native_drag_mouse(MouseEventKind::Entered),
        ] {
            assert!(!h.owns_native_titlebar_drag_event(&event));
        }
        assert!(h.owns_native_titlebar_drag_event(&drag));
    }
    #[test]
    fn native_titlebar_drag_does_not_steal_existing_toolbar_split_or_body_capture() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(0);
        h.set_toolbar_focus(Some(owner), false);
        let point = Point::new(110., 26.);
        h.dispatch_toolbar(owner, &ContentEvent::PointerDown(point))
            .unwrap();
        let index = h.toolbar_mgr.toolbar_index;
        let capture = h.panel_mgr.panels[index]
            .mounted_toolbar
            .host
            .content_host()
            .pointer_capture();
        assert!(capture.is_some());
        h.layout.native_titlebar_drag = true;
        let drag = native_drag_mouse(MouseEventKind::Dragged {
            button: MouseButton::Left,
        });
        assert!(!h.owns_native_titlebar_drag_event(&drag));
        assert_eq!(
            h.panel_mgr.panels[index]
                .mounted_toolbar
                .host
                .content_host()
                .pointer_capture(),
            capture
        );
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(point))
            .unwrap();
        assert_eq!(commands(&h), vec![(0, 7)]);

        let whole = Rect::new(0., 0., 1000., 400.);
        h.layout
            .outer_view
            .arrange(&mut h.layout.outer, whole, Some(whole));
        let divider = h.layout.outer_view.layout().divider.frame();
        let start = Point::new(divider.min_x() + divider.size.width / 2., 200.);
        assert!(h.layout.outer_view.pointer_down(start, &mut h.layout.outer));
        h.layout.drag_split = Some(0);
        assert!(!h.owns_native_titlebar_drag_event(&drag));
        assert!(
            h.layout
                .outer_view
                .pointer_move(Point::new(start.x + 30., start.y), &mut h.layout.outer)
        );
        assert!(h.layout.outer_view.pointer_up(&mut h.layout.outer));
        h.layout.drag_split = None;
        h.layout.pressed = Some(1);
        assert!(!h.owns_native_titlebar_drag_event(&drag));
        assert_eq!(h.layout.pressed, Some(1));
        h.layout.pressed = None;
        assert!(h.owns_native_titlebar_drag_event(&drag));
    }
    #[test]
    fn whole_window_tabs_traverse_original_hosts_and_reverse_without_preclick() {
        let mut h = fixture();
        let targets = vec![
            ToolbarTarget::Visibility(0),
            ToolbarTarget::Panel(0),
            ToolbarTarget::Panel(1),
            ToolbarTarget::Panel(2),
            ToolbarTarget::Visibility(1),
        ];
        assert_eq!(h.toolbar_focus_targets(), targets);
        for target in targets.iter().chain(targets.iter().take(1)) {
            key(&mut h, ControlKey::Tab { reverse: false }, true);
            assert_eq!(h.toolbar_mgr.toolbar_focus, Some(*target));
            assert_eq!(focused_groups(&h), vec![*target]);
        }
        key(&mut h, ControlKey::Tab { reverse: true }, true);
        assert_eq!(
            h.toolbar_mgr.toolbar_focus,
            Some(ToolbarTarget::Visibility(1))
        );
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn arrow_roving_and_same_ids_remain_with_original_module_owner() {
        let mut h = fixture();
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(1)), true);
        key(&mut h, ControlKey::Right, true);
        key(&mut h, ControlKey::Enter, true);
        key(&mut h, ControlKey::Enter, false);
        assert_eq!(commands(&h), vec![(1, 8)]);
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(commands(&h), vec![(0, 7), (1, 8)]);
    }
    #[test]
    fn tab_escape_and_deactivation_cancel_late_space_without_losing_next_activation() {
        let mut h = fixture();
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Escape, true);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        key(&mut h, ControlKey::Space, true);
        h.cancel_toolbar_input();
        h.reconcile_toolbar_focus(false);
        assert!(focused_groups(&h).is_empty());
        h.reconcile_toolbar_focus(true);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        assert_eq!(focused_groups(&h), vec![ToolbarTarget::Panel(1)]);
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(commands(&h), vec![(1, 7)]);
    }
    #[test]
    fn hidden_disabled_and_reordered_groups_reconcile_existing_focus() {
        let mut h = fixture();
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        key(&mut h, ControlKey::Space, true);
        h.layout.leading_visible = false;
        h.reconcile_toolbar_focus(true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(h.toolbar_mgr.toolbar_focus, None);
        assert!(commands(&h).is_empty());
        assert!(!h.toolbar_focus_targets().contains(&ToolbarTarget::Panel(0)));
        h.layout.leading_visible = true;
        h.layout.order = vec![2, 0, 1];
        refresh(&mut h, None);
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(ToolbarTarget::Panel(1)));
        h.panel_mgr.panels[1].toolbar = vec![ToolbarItem {
            id: 7,
            title: "Disabled".into(),
            enabled: false,
            geometry_dependent: false,
        }];
        refresh(&mut h, None);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, None);
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn overlay_owns_tab_and_outside_pointer_before_visibility_controls() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        refresh(&mut h, Some(1));
        h.set_toolbar_focus(Some(owner), true);
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(h.open_toolbar(), Some(owner));
        refresh(&mut h, Some(1));
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        let p = Point::new(24., 26.);
        assert_eq!(h.toolbar_target_at(p), Some(owner));
        h.dispatch_toolbar(owner, &ContentEvent::PointerDown(p))
            .unwrap();
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(p))
            .unwrap();
        assert!(h.layout.leading_visible);
        assert!(h.layout.trailing_visible);
        assert!(commands(&h).is_empty());
        assert_eq!(h.open_toolbar(), None);
    }
    #[test]
    fn escape_cancels_original_pointer_arm_and_empty_window_tab_is_safe() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(0);
        h.set_toolbar_focus(Some(owner), false);
        let p = Point::new(110., 26.);
        h.dispatch_toolbar(owner, &ContentEvent::PointerDown(p))
            .unwrap();
        assert_eq!(
            h.panel_mgr.panels[h.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .pointer_capture()
                .map(|(id, _)| id),
            Some(Host::toolbar_control(owner))
        );
        key(&mut h, ControlKey::Escape, true);
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(p))
            .unwrap();
        assert!(commands(&h).is_empty());
        assert_eq!(
            h.panel_mgr.panels[h.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .pointer_capture(),
            None
        );
        let mut h = Host::new(vec!["empty".into()], false, None);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        key(&mut h, ControlKey::Tab { reverse: true }, true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, None);
    }
    #[test]
    fn temporary_input_barrier_restores_logical_owner_but_never_old_arm_or_stolen_focus() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        h.set_toolbar_focus(Some(owner), true);
        key(&mut h, ControlKey::Space, true);
        h.layout
            .sidebar_motion
            .retarget(0, true, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(h.layout.sidebar_motion.needs_frame());
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        assert!(focused_groups(&h).is_empty());
        key(&mut h, ControlKey::Escape, true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        h.layout
            .sidebar_motion
            .sample(std::time::Duration::from_secs(2), false, true)
            .unwrap();
        h.reconcile_toolbar_focus(true);
        assert_eq!(focused_groups(&h), vec![owner]);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        h.layout
            .sidebar_motion
            .retarget(0, false, std::time::Duration::from_secs(3))
            .unwrap();
        h.reconcile_toolbar_focus(true);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        assert_eq!(
            h.toolbar_mgr.toolbar_focus,
            Some(ToolbarTarget::Visibility(1))
        );
        h.layout
            .sidebar_motion
            .sample(std::time::Duration::from_secs(4), false, true)
            .unwrap();
        h.reconcile_toolbar_focus(true);
        assert_eq!(focused_groups(&h), vec![ToolbarTarget::Visibility(1)]);
    }
    fn native_tab(reverse: bool) -> ContentKeyEvent {
        ContentKeyEvent {
            key: Some(ControlKey::Tab { reverse }),
            down: true,
            repeat: false,
            modifiers: Default::default(),
            native_code: 48,
            characters: "\t".into(),
            characters_ignoring_modifiers: "\t".into(),
        }
    }
    #[test]
    fn native_toolbar_tab_consumption_does_not_reuse_previous_pointer_bubbling() {
        for reverse in [false, true] {
            let mut h = fixture();
            h.set_toolbar_focus(None, false);
            for panel in &mut h.panel_mgr.panels {
                panel
                    .mounted_toolbar
                    .host
                    .event_with_text(
                        &ContentEvent::PointerMove(Point::new(-100., -100.)),
                        &mut h.text,
                    )
                    .unwrap();
                assert!(
                    panel.mounted_toolbar.host.event_bubbled(),
                    "real prior pointer event must bubble"
                );
            }
            // This is the same consumption result used by route_toolbar_event
            // before the original Settings/body fallback receives this key.
            let consumed = h.toolbar_key_input(native_tab(reverse)).unwrap();
            assert_eq!(
                focused_groups(&h).len(),
                1,
                "Tab really focused one Toolbar"
            );
            assert!(consumed, "that same Tab must not also enter the body Form");
            let focused = focused_groups(&h);
            let mut modified = native_tab(reverse);
            modified.modifiers.command = true;
            assert!(!h.toolbar_key_input(modified).unwrap());
            assert_eq!(focused_groups(&h), focused);
        }
    }
    #[test]
    fn native_toolbar_tab_without_an_actual_focus_target_bubbles_to_body() {
        let local = crate::LocalContainer::new(
            "body-only",
            tekes_ui_controls::composition::Content::new(
                ComponentId(1),
                tekes_ui_controls::composition::TextContent::new("Body without toolbar actions"),
            ),
        )
        .unwrap();
        let mut h = crate::AppWindow::from_columns(vec![local.into()])
            .unwrap()
            .into_host(false);
        h.mount_initial_containers();
        assert_eq!(
            h.panel_mgr.panels.len(),
            1,
            "legal public local Container is mounted"
        );
        assert!(h.toolbar_focus_targets().is_empty());
        assert!(!h.toolbar_key_input(native_tab(false)).unwrap());
        assert!(focused_groups(&h).is_empty());
    }
    #[test]
    fn modified_system_tab_does_not_enter_or_restyle_a_toolbar_focus_scope() {
        let mut h = fixture();
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(1)), false);
        render(&mut h);
        assert!(
            !h.panel_mgr.panels[h.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .needs_render()
        );
        h.toolbar_key_input(ContentKeyEvent {
            key: Some(ControlKey::Tab { reverse: false }),
            down: true,
            repeat: false,
            modifiers: InputModifiers {
                command: true,
                ..Default::default()
            },
            native_code: 48,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        })
        .unwrap();
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(ToolbarTarget::Panel(1)));
        assert!(
            !h.panel_mgr.panels[h.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .needs_render()
        );
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn unavailable_toolbar_rejects_action_but_leaves_native_chrome_drag_available() {
        let mut h = fixture();
        h.layout.leading_visible = false;
        let owner = ToolbarTarget::Panel(0);
        assert!(
            h.dispatch_toolbar(owner, &ContentEvent::PointerDown(Point::new(110., 26.)))
                .unwrap()
        );
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(Point::new(110., 26.)))
            .unwrap();
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn awaiting_current_geometry_preserves_owner_without_replaying_old_space() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        h.set_toolbar_focus(Some(owner), true);
        key(&mut h, ControlKey::Space, true);
        h.panel_mgr.panels[1].viewport = 1;
        for item in &mut h.panel_mgr.panels[1].toolbar {
            item.geometry_dependent = true;
        }
        refresh(&mut h, None);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        assert!(focused_groups(&h).is_empty());
        h.panel_mgr.panels[1].frame_viewport = 1;
        refresh(&mut h, None);
        h.reconcile_toolbar_focus(true);
        assert_eq!(focused_groups(&h), vec![owner]);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(commands(&h), vec![(1, 7)]);
    }
    #[test]
    fn height_only_viewport_change_revokes_arm_even_when_toolbar_frame_does_not_move() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        h.set_toolbar_focus(Some(owner), true);
        key(&mut h, ControlKey::Space, true);
        h.panel_geometry(1, Rect::new(320., 0., 240., 320.), 1., 66., 0.)
            .unwrap();
        assert_eq!(
            h.panel_mgr.panels[h.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused(),
            None
        );
        render(&mut h);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_mgr.toolbar_focus, Some(owner));
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        key(&mut h, ControlKey::Space, true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(commands(&h), vec![(1, 7)]);
    }
}

#[cfg(test)]
mod original_native_allocation_tests {
    #[test]
    fn native_toolbar_alignment_preserves_full_allocation_without_centering_in_it() {
        assert_eq!(
            super::window_toolbar_geometry(66., 48., Some(26.), 2.),
            (66., 2., 48.)
        );
        assert_eq!(
            super::window_toolbar_geometry(40., 32., Some(20.), 1.),
            (40., 4., 32.)
        );
        assert_eq!(
            super::window_toolbar_geometry(0., 48., None, 2.),
            (48., 0., 48.)
        );
        // Oversize retains complete content rather than a negative, clipped origin.
        assert_eq!(
            super::window_toolbar_geometry(66., 80., Some(26.), 2.),
            (80., 0., 80.)
        );
        assert_eq!(
            super::window_toolbar_geometry(65.75, 48., Some(25.75), 2.),
            (66., 1.75, 48.)
        );
    }
}

#[cfg(test)]
mod settings_layout_tests {
    use super::*;
    #[test]
    fn settings_layout_reuses_only_leading_visibility_toolbar_and_can_block_quit() {
        let mut h = Host::new(vec![], false, None);
        h.settings_mgr.is_settings_window = true;
        assert_eq!(h.panel_mgr.panels.len(), 0);
    }
}

#[cfg(test)]
mod settings_guard_tests {
    use super::*;
    use crate::settings::{SettingsDisplayMode, SettingsSurface, SettingsWindow};
    use tekes_ui_platform_macos::WindowHandler;
    use tekes_ui_settings::{FileProvider, Snapshot};
    #[test]
    fn dirty_settings_blocks_app_quit_until_explicit_cancel_and_modes_are_window_policy() {
        let path = std::env::temp_dir()
            .join(format!(
                "apphost-settings-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("test_fixtures/layout.schema.toml"),
            path.clone(),
        )
        .unwrap();
        assert_eq!(options.display_mode, SettingsDisplayMode::Exclusive);
        assert_eq!(
            options
                .clone()
                .display_mode(SettingsDisplayMode::Independent)
                .display_mode,
            SettingsDisplayMode::Independent
        );
        let provider = FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
        let surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        let first = surface.entry("workspace/general").unwrap();
        let repeat = surface.entry("workspace/general").unwrap();
        assert_ne!(first.id, repeat.id);
        assert_eq!(first.params, repeat.params);
        let mut host = Host::new(vec![], false, None);
        host.settings_mgr.is_settings_window = true;
        host.settings_mgr.settings_back = surface.exclusive;
        host.settings_mgr.settings = Some(std::rc::Rc::new(std::cell::RefCell::new(surface)));
        assert!(host.allows_app_quit());
        {
            let mut state = host.settings_mgr.settings.as_ref().unwrap().borrow_mut();
            let s = &mut *state;
            s.draft.edit(&s.schema.settings[0], "Changed".into());
        }
        assert!(!host.allows_app_quit());
        host.settings_mgr
            .settings
            .as_ref()
            .unwrap()
            .borrow_mut()
            .draft
            .cancel();
        assert!(host.allows_app_quit());
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[cfg(test)]
mod settings_toolbar_focus_tests {
    use super::*;
    use crate::toolbar_source::ComponentSource;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use tekes_ui_controls::composition::{
        ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion,
    };
    #[test]
    fn actual_config_group_focus_retires_old_panel_return_intent() {
        let mut host = Host::new(vec!["panel".into()], false, None);
        host.panel_mgr.panels = vec![Panel::new(0, "panel".into())];
        host.panel_mgr.panels[0].bounds = Rect::new(0., 0., 240., 300.);
        let source = ComponentSource {
            session: tekes_ui_module_runtime::component::Session {
                module: crate::settings_undo::MODULE_ID.into(),
                instance: 1,
                host_epoch: 1,
            },
            registration: 1,
            window: host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .window,
        };
        let button = |id, name: &str| {
            Contents::Commands(
                vec![ToolbarButtonDeclaration {
                    id,
                    icon: None,
                    text: name.into(),
                    hint: None,
                    enabled: true,
                }],
                ToolbarGroupOverflow::Together,
            )
        };
        let instance = host.panel_mgr.panels[0].instance;
        host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .sync(
                Rect::new(0., 0., 480., 300.),
                vec![
                    ToolbarRegion::new(ComponentId(10), Rect::new(0., 0., 240., 52.)),
                    ToolbarRegion::new(ComponentId(20), Rect::new(240., 0., 240., 52.)),
                ],
                vec![
                    Declaration {
                        id: Host::toolbar_group(ToolbarTarget::Panel(0)),
                        region: ComponentId(10),
                        placement: ToolbarPlacement::Navigation,
                        order: 0,
                        contents: button(7, "Panel"),
                        source: Source::Panel {
                            instance,
                            index: 0,
                            module: "panel".into(),
                            pid: None,
                            generation: 0,
                        },
                    },
                    Declaration {
                        id: ComponentId(92300),
                        region: ComponentId(20),
                        placement: ToolbarPlacement::SecondaryAction,
                        order: 0,
                        contents: button(crate::settings_undo::UNDO, "Undo"),
                        source: Source::Undo(source),
                    },
                ],
            )
            .unwrap();
        host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
        host.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        assert_eq!(
            host.toolbar_mgr.toolbar_focus,
            Some(ToolbarTarget::Panel(0))
        );
        let undo = host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .find(|n| n.node.id == ComponentId(92301))
            .unwrap()
            .target;
        assert!(
            host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .restore_focus(undo, true)
        );
        host.remember_toolbar_focus(Some(true));
        host.reconcile_toolbar_focus(true);
        assert_eq!(host.toolbar_mgr.toolbar_focus, None);
        assert!(host.toolbar_mgr.toolbar_focus_lease.is_none());
        assert_eq!(
            host.panel_mgr.panels[host.toolbar_mgr.toolbar_index]
                .mounted_toolbar
                .host
                .content_host()
                .focused(),
            Some(ComponentId(92301))
        );
    }
}

#[cfg(test)]
mod settings_close_input_tests {
    use super::*;
    use crate::settings::{SettingsSurface, SettingsWindow};
    use tekes_ui_controls::composition::NumberFieldContent;
    use tekes_ui_settings::{FileProvider, Snapshot};
    fn fixture(schema: &str) -> (Host, std::path::PathBuf) {
        let path = std::env::temp_dir()
            .join(format!(
                "settings-close-input-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(schema, path.clone()).unwrap();
        let provider = FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
        let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        surface.show();
        // Ordinary focus/reveal requires current real layout, as in the Window.
        let frame = Rect::new(0., 0., 620., 400.);
        surface
            .form
            .render(
                &mut Scene::new(),
                &mut Text::new(),
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        for (index, field) in surface.view_schema.settings.iter().enumerate() {
            let id = ComponentId(100 + index as u64);
            let refusal = surface
                .form
                .with_host_mut(|host| {
                    host.plan_host_reveal(id, Default::default(), &[], Default::default())
                })
                .err();
            eprintln!(
                "SETTINGS_FIXTURE selected={} field={} kind={:?} component={} focus_targets={:?} reveal_refusal={:?}",
                surface.selected,
                field.id,
                field.kind,
                surface.form.host().get(id).is_some(),
                surface.form.host().focus_targets(),
                refusal
            );
        }
        let mut host = Host::new(vec![], false, None);
        host.settings_mgr.settings = Some(std::rc::Rc::new(std::cell::RefCell::new(surface)));
        (host, path)
    }
    #[test]
    fn raw_number_draft_blocks_quit_and_close_commits_only_valid_input() {
        for text in ["999%", "125%"] {
            let (host, path) = fixture(include_str!(
                "test_fixtures/app-host-demo.settings.schema.toml"
            ));
            let state = host.settings_mgr.settings.as_ref().unwrap();
            {
                let mut s = state.borrow_mut();
                s.form.with_host_mut(|form| {
                    assert!(form.focus(Some(ComponentId(101)), true));
                    form.event(&ContentEvent::TextCommand(
                        tekes_ui_controls::TextEditCommand::SelectAll,
                    ));
                    form.event(&ContentEvent::Commit {
                        text: text.into(),
                        replacement: None,
                    });
                });
                assert!(s.form.has_number_draft());
                assert!(
                    !s.draft.dirty(),
                    "raw editor input is not a saved Draft value"
                );
            }
            assert!(!host.allows_app_quit());
            {
                let mut s = state.borrow_mut();
                assert!(!s.close_requested());
                assert!(s.open);
                if text == "999%" {
                    assert!(!s.draft.dirty());
                    assert!(s.form.has_number_draft());
                    assert_eq!(
                        s.form
                            .host()
                            .get(ComponentId(101))
                            .unwrap()
                            .component::<NumberFieldContent>()
                            .unwrap()
                            .edit_state()
                            .text,
                        "999%"
                    );
                } else {
                    assert!(
                        s.draft.dirty(),
                        "valid raw input must await its persistence receipt"
                    );
                    assert!(!s.form.has_number_draft());
                }
            }
            assert!(!host.allows_app_quit());
            assert!(!path.exists());
            drop(host);
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }
    #[test]
    fn marked_text_with_clean_draft_is_retained_when_close_or_quit_is_requested() {
        let (host, path) = fixture(include_str!("test_fixtures/layout.schema.toml"));
        let state = host.settings_mgr.settings.as_ref().unwrap();
        {
            let mut s = state.borrow_mut();
            assert!(s.form.focus(Some(ComponentId(100)), true));
            s.form.with_host_mut(|host| {
                host.event(&ContentEvent::Preedit {
                    text: "中文".into(),
                    selection: 0..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: None,
                });
            });
            assert!(s.form.host().is_composing());
            assert!(!s.draft.dirty());
        }
        assert!(!host.allows_app_quit());
        {
            let mut s = state.borrow_mut();
            let before = s.form.host().focused_text().unwrap().text().to_owned();
            let marked = s.form.host().focused_text().unwrap().marked();
            assert!(!s.close_requested());
            assert!(s.open);
            assert_eq!(s.form.host().focused_text().unwrap().text(), before);
            assert_eq!(s.form.host().focused_text().unwrap().marked(), marked);
            assert!(!s.draft.dirty());
        }
        assert!(!host.allows_app_quit());
        assert!(!path.exists());
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn original_settings_row_focus_succeeds_but_unknown_scroll_layout_stays_rejected() {
        use tekes_ui_controls::{
            ButtonTextRenderer, ScrollArea,
            composition::{
                Component, Content, ContentHost, HostRevealError, ScrollContent, TextFieldContent,
            },
            layout::MeasuredChild,
        };
        use tekes_ui_scene::Size;
        let (host, path) = fixture(include_str!("test_fixtures/layout.schema.toml"));
        {
            let mut state = host.settings_mgr.settings.as_ref().unwrap().borrow_mut();
            assert!(
                state
                    .form
                    .with_host_mut(|host| {
                        host.plan_host_reveal(
                            ComponentId(100),
                            Default::default(),
                            &[],
                            Default::default(),
                        )
                    })
                    .is_ok()
            );
            assert!(state.form.focus(Some(ComponentId(100)), true));
            assert_eq!(state.form.host().focused(), Some(ComponentId(100)));
        }
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        #[derive(Debug)]
        struct Unknown {
            children: Vec<Content>,
        }
        impl Component for Unknown {
            fn children(&self) -> &[Content] {
                &self.children
            }
            fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
                Some(&mut self.children)
            }
            fn measure(
                &mut self,
                text: &mut dyn ButtonTextRenderer,
                available: Size,
            ) -> MeasuredChild {
                self.children[0].measure(text, available)
            }
            fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
                self.children[0].arrange(text, frame, clip);
            }
        }
        let mut unknown = ContentHost::new(Content::new(
            ComponentId(1),
            ScrollContent::new(
                ScrollArea::default(),
                Content::new(
                    ComponentId(2),
                    Unknown {
                        children: vec![Content::new(
                            ComponentId(3),
                            TextFieldContent::new("Unknown", "draft"),
                        )],
                    },
                ),
            ),
        ))
        .unwrap();
        let frame = Rect::new(0., 0., 620., 400.);
        unknown
            .render(
                &mut Scene::new(),
                &mut Text::new(),
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(matches!(
            unknown.plan_host_reveal(ComponentId(3), Default::default(), &[], Default::default()),
            Err(HostRevealError::UnsupportedLayout)
        ));
        assert!(!unknown.focus(Some(ComponentId(3)), true));
        assert_eq!(unknown.focused(), None);
    }
}

#[cfg(test)]
mod shared_settings_owner_tests {
    use super::*;
    use crate::settings::{
        SettingsConfiguration, SettingsDisplayMode, SettingsOwner, SettingsWindow,
    };
    use std::rc::Rc;
    use tekes_ui_settings::Draft;

    #[test]
    fn shared_owner_keeps_one_writer_and_releases_abandoned_host_load() {
        let path = std::env::temp_dir()
            .join(format!(
                "shared-settings-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("test_fixtures/layout.schema.toml"),
            path.clone(),
        )
        .unwrap()
        .display_mode(SettingsDisplayMode::Independent);
        let owner = SettingsOwner::new(options.clone()).unwrap();
        let mut first = Host::new(vec![], false, None);
        let mut second = Host::new(vec![], false, None);
        first
            .configure_settings(Some(SettingsConfiguration::Shared(owner.clone())))
            .unwrap();
        second
            .configure_settings(Some(SettingsConfiguration::Shared(owner.clone())))
            .unwrap();
        assert!(Rc::ptr_eq(
            &first.settings_mgr.settings_owner.as_ref().unwrap().0,
            &second.settings_mgr.settings_owner.as_ref().unwrap().0
        ));
        assert!(
            SettingsOwner::new(options.clone()).is_err(),
            "separate owners must respect the existing writer lock"
        );

        let ticket = first
            .settings_mgr
            .settings_owner
            .as_ref()
            .unwrap()
            .begin_load()
            .unwrap();
        let (_sender, receiver) = mpsc::channel();
        first.settings_mgr.settings_loading = Some(window_settings::SettingsLoad {
            _ticket: ticket,
            receiver,
        });
        assert!(
            second
                .settings_mgr
                .settings_owner
                .as_ref()
                .unwrap()
                .begin_load()
                .is_none(),
            "only one live opener may load"
        );
        drop(first); // Equivalent owner/receiver release to native Shared::finish.
        let next = second
            .settings_mgr
            .settings_owner
            .as_ref()
            .unwrap()
            .begin_load()
            .expect("a closed requester must not strand pending state");
        drop(next);
        let saved = {
            let state = owner.0.borrow();
            let mut draft = Draft::new(state.provider.load().unwrap());
            draft.edit(&state.options.schema.settings[0], "Shared sample".into());
            state
                .provider
                .commit(
                    &draft
                        .submit(&state.options.schema, "shared-save".into())
                        .unwrap(),
                )
                .unwrap()
        };
        drop(second);
        drop(owner);
        let reopened = SettingsOwner::new(options).unwrap();
        assert_eq!(reopened.0.borrow().provider.load().unwrap(), saved);
        drop(reopened);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn settings_child_keeps_standard_application_settings_command_without_owner_cycle() {
        let mut child = Host::new(vec![], false, None);
        child.settings_mgr.is_settings_window = true;
        assert!(child.settings_mgr.settings_owner.is_none());
        let bar = child.menu_bar();
        assert_eq!(bar.menus.len(), 1);
        assert!(bar.menus[0].application_menu);
        let command = bar.commands().next().unwrap();
        assert_eq!(
            command.command,
            tekes_ui_platform_macos::CommandId(0x534554)
        );
        assert_eq!(
            command.key_equivalent,
            Some(tekes_ui_platform_macos::KeyEquivalent::command(','))
        );
    }

    #[test]
    fn empty_order_retire_hidden_content_and_failed_close_prompted_allows_app_quit() {
        let mut host = Host::new(vec!["panel".into()], false, None);
        host.panel_mgr.panels = vec![Panel::new(0, "panel".into())];
        host.panel_mgr.panels[0].logically_visible = false;
        host.layout.order.clear();
        host.layout.focused = 0;
        host.retire_hidden_content(0, Rect::default());
        assert_eq!(host.layout.focused, 0);

        let path = std::env::temp_dir()
            .join(format!(
                "settings-failed-close-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("test_fixtures/layout.schema.toml"),
            path.clone(),
        )
        .unwrap();
        let provider =
            tekes_ui_settings::FileProvider::open(path.clone(), options.schema.namespace.clone())
                .unwrap();
        let mut surface = crate::settings::SettingsSurface::new(
            options,
            provider,
            tekes_ui_settings::Snapshot::default(),
        )
        .unwrap();
        surface.show();
        surface
            .draft
            .edit(&surface.schema.settings[0], "Changed".into());
        host.settings_mgr.is_settings_window = true;
        host.settings_mgr.settings = Some(Rc::new(std::cell::RefCell::new(surface)));

        assert!(!host.allows_app_quit());
        host.settings_mgr
            .settings
            .as_ref()
            .unwrap()
            .borrow_mut()
            .failed_close_prompted = true;
        assert!(host.allows_app_quit());

        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn close_shortcut_targets_closable_others_tabs_before_window() {
        use crate::{ContainerOptions, LocalContainer, OthersOptions, PresentationOptions};
        use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};

        let make_local = |id: &str, list_height: Option<f32>| {
            let mut local = LocalContainer::new(
                id,
                Content::new(ComponentId(1), TextFieldContent::new("Field", id)),
            )
            .unwrap();
            if let Some(height) = list_height {
                local = local.options(ContainerOptions {
                    list_height: Some(height),
                    ..ContainerOptions::default()
                });
            }
            local.into()
        };

        let mut host = crate::AppWindow::from_containers(
            vec![
                make_local("focus", None),
                make_local("tab_a", None),
                make_local("tab_b", None),
                make_local("list_item", Some(120.)),
            ],
            PresentationOptions::new("focus").others(OthersOptions::new(vec![
                "tab_a".into(),
                "tab_b".into(),
                "list_item".into(),
            ])),
        )
        .unwrap()
        .into_host(false);
        host.mount_initial_containers();

        assert_eq!(host.close_menu_title(), Some("Close Tab"));
        assert_eq!(host.closable_active_tab(), Some("tab_a"));
        assert!(host.close_active_tab().unwrap());
        assert_eq!(
            host.layout.presentation.as_ref().unwrap().others_ids(),
            &["tab_b".to_owned(), "list_item".to_owned()]
        );

        // Selecting List makes the active tab non-closable by container close.
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(crate::OthersTab::List)
            .unwrap();
        assert_eq!(host.close_menu_title(), None);
        assert!(!host.close_active_tab().unwrap());

        // Switching back to container tab restores "Close Tab", while hiding Others yields None.
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .select_tab(crate::OthersTab::Container("tab_b".into()))
            .unwrap();
        assert_eq!(host.close_menu_title(), Some("Close Tab"));
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
        assert_eq!(host.close_menu_title(), None);
        assert!(!host.close_active_tab().unwrap());

        // Restoring Others and closing tab_b leaves only List -> next shortcut closes window.
        host.layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_others()
            .unwrap();
        assert!(host.close_active_tab().unwrap());
        assert_eq!(host.close_menu_title(), None);
        assert!(!host.close_active_tab().unwrap());

        // Settings auxiliary windows never treat ⌘W as a tab close.
        let mut settings_host = crate::AppWindow::from_containers(
            vec![make_local("focus", None), make_local("tab", None)],
            PresentationOptions::new("focus").others(OthersOptions::new(vec!["tab".into()])),
        )
        .unwrap()
        .into_host(false);
        settings_host.settings_mgr.is_settings_window = true;
        assert_eq!(settings_host.close_menu_title(), None);
        assert!(!settings_host.close_active_tab().unwrap());
    }

    #[test]
    fn presentation_layout_skips_columns_and_cancels_constrained_or_hidden_split_drag() {
        use crate::{LocalContainer, OthersOptions, PresentationOptions};
        use tekes_ui_controls::composition::{ComponentId, Content, TextContent};

        let make_local = |id: &str| -> crate::Container {
            LocalContainer::new(id, Content::new(ComponentId(1), TextContent::new(id)))
                .unwrap()
                .into()
        };

        let mut pres_host = crate::AppWindow::from_containers(
            vec![
                make_local("focus"),
                make_local("navbar"),
                make_local("tab_a"),
                make_local("tab_b"),
            ],
            PresentationOptions::new("focus")
                .nav_bar("navbar")
                .others(OthersOptions::new(vec!["tab_a".into(), "tab_b".into()])),
        )
        .unwrap()
        .into_host(false);
        pres_host.mount_initial_containers();
        assert!(
            pres_host
                .layout
                .presentation
                .as_ref()
                .unwrap()
                .others_visible()
        );

        // Wide layout: NavBar + Focus + Others dividers visible and draggable.
        let wide = Rect::new(0., 0., 1200., 600.);
        let frames_wide = pres_host
            .presentation_frames_with_safe_area(
                wide,
                42.,
                window_toolbar_mount::ToolbarBand {
                    y: 0.,
                    height: 42.,
                    native_leading: 0.,
                },
                0.,
                0.,
            )
            .unwrap();
        pres_host
            .paint_presentation(&mut Scene::new(), wide, Theme::LIGHT.into())
            .unwrap();
        assert_eq!(pres_host.diagnostics.presentation_layout_calls, 1);
        assert_eq!(pres_host.diagnostics.column_layout_calls, 0);
        assert_eq!(frames_wide.len(), 4);
        assert!(frames_wide[0].size.width > 0.);
        assert_eq!(pres_host.presentation_resize_regions().len(), 2);

        // Start dragging Focus/Others divider (inner_view -> drag_split = Some(3)), then narrow
        // to constrained width so SplitView::is_constrained cancels drag capture.
        let inner_down = pres_host
            .layout
            .inner_view
            .layout()
            .divider
            .frame()
            .center();
        assert!(
            pres_host
                .layout
                .inner_view
                .pointer_down(inner_down, &mut pres_host.layout.inner)
        );
        pres_host.layout.drag_split = Some(3);
        assert!(pres_host.layout.inner_view.is_dragging());

        let narrow = Rect::new(0., 0., 460., 420.);
        pres_host
            .presentation_frames_with_safe_area(
                narrow,
                42.,
                window_toolbar_mount::ToolbarBand {
                    y: 0.,
                    height: 42.,
                    native_leading: 0.,
                },
                0.,
                0.,
            )
            .unwrap();
        pres_host
            .paint_presentation(&mut Scene::new(), narrow, Theme::LIGHT.into())
            .unwrap();
        assert!(!pres_host.layout.inner_view.is_dragging());
        assert_eq!(pres_host.layout.drag_split, None);
        assert_eq!(pres_host.diagnostics.column_layout_calls, 0);
        for panel in &pres_host.panel_mgr.panels {
            assert!(panel.bounds.size.width >= 0. && panel.clip.size.width >= 0.);
            assert!(panel.bounds.max_x() <= narrow.max_x() + 0.01);
        }

        // Re-widen, start dragging NavBar divider (outer_view -> drag_split = Some(2)),
        // then hide NavBar: else branch cancels outer_view & drag_split and removes its resize hit.
        pres_host
            .presentation_frames_with_safe_area(
                wide,
                42.,
                window_toolbar_mount::ToolbarBand {
                    y: 0.,
                    height: 42.,
                    native_leading: 0.,
                },
                0.,
                0.,
            )
            .unwrap();
        pres_host
            .paint_presentation(&mut Scene::new(), wide, Theme::LIGHT.into())
            .unwrap();
        let outer_down = pres_host
            .layout
            .outer_view
            .layout()
            .divider
            .frame()
            .center();
        assert!(
            pres_host
                .layout
                .outer_view
                .pointer_down(outer_down, &mut pres_host.layout.outer)
        );
        pres_host.layout.drag_split = Some(2);
        pres_host
            .layout
            .presentation
            .as_mut()
            .unwrap()
            .toggle_nav_bar()
            .unwrap();
        pres_host
            .paint_presentation(&mut Scene::new(), wide, Theme::LIGHT.into())
            .unwrap();
        assert!(!pres_host.layout.outer_view.is_dragging());
        assert_eq!(pres_host.layout.drag_split, None);
        assert_eq!(pres_host.presentation_resize_regions().len(), 1);

        // When headless GPU renderer is available, verify full scene build keeps branches exclusive.
        if let Ok(mut renderer) = tekes_ui_render::Renderer::new_headless() {
            let calls_before = pres_host.diagnostics.presentation_layout_calls;
            pres_host
                .build_headless_scene(&mut renderer, wide, 2.0, true)
                .unwrap();
            assert_eq!(
                pres_host.diagnostics.presentation_layout_calls,
                calls_before + 1
            );
            assert_eq!(pres_host.diagnostics.column_layout_calls, 0);

            let mut col_host = crate::AppWindow::from_columns(vec![
                make_local("a").into(),
                make_local("b").into(),
            ])
            .unwrap()
            .into_host(false);
            col_host.mount_initial_containers();
            col_host
                .build_headless_scene(&mut renderer, wide, 2.0, true)
                .unwrap();
            assert_eq!(col_host.diagnostics.column_layout_calls, 1);
            assert_eq!(col_host.diagnostics.presentation_layout_calls, 0);
        }
    }
}
