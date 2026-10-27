use crate::{
    module_process::ModuleProcess,
    navigation_window::{NavigationOptions, WindowNavigation},
    text::Text,
};
use anyhow::{Result, ensure};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Receiver, SyncSender},
    },
};
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
use tekes_ui_platform_macos::{
    App, AppConfig, Event, KeyEvent, KeyEventKind, MenuBar, MouseButton, MouseEvent,
    MouseEventKind, Size as WindowSize, Window, WindowConfig, WindowHandler, WindowWake,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, ImageQuad, Point, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme};
const MIN_COLUMN: f32 = 120.;
#[path = "window_accessibility.rs"]
mod window_accessibility;
#[path = "window_settings.rs"]
mod window_settings;
#[path = "window_columns.rs"]
mod window_columns;
#[path = "window_toolbar_mount.rs"]
mod window_toolbar_mount;
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
fn toolbar_divider_frame(divider: Rect, row_y: f32) -> Rect {
    let top = divider.min_y().max(row_y);
    Rect::new(
        divider.min_x(),
        top,
        divider.size.width,
        (divider.max_y() - top).max(0.),
    )
}

fn content_geometry(bounds: Rect, toolbar_height: f32) -> Rect {
    Rect::new(
        bounds.min_x(),
        bounds.min_y() + toolbar_height,
        bounds.size.width,
        (bounds.size.height - toolbar_height).max(1.),
    )
}
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
struct Inbox {
    queue: Mutex<VecDeque<Request>>,
    cv: Condvar,
    closed: std::sync::atomic::AtomicBool,
}
struct Pending {
    frame: tekes_ui_module_runtime::ReceivedFrame,
    ack: SyncSender<()>,
}
struct Panel {
    id: u32,
    instance: u64,
    local: Option<crate::Container>,
    inbox: Arc<Inbox>,
    process: tekes_ui_module_runtime::ProcessControl,
    rx: Receiver<Result<Pending, String>>,
    image: Option<ImageId>,
    frame_size: (u32, u32),
    frame_pixels: (u32, u32),
    viewport: u64,
    frame_viewport: u64,
    frame_seq: u64,
    scale: f32,
    bounds: Rect,
    toolbar_generation: u64,
    toolbar_height: f32,
    toolbar: Vec<ToolbarItem>,
    seq: u64,
    failed: Option<String>,
    pid: Option<u32>,
    module_id: String,
    launch_tx: Option<SyncSender<Result<Pending, String>>>,
}
impl Panel {
    fn new(id: u32, module_id: String) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let instance = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let inbox = Arc::new(Inbox {
            queue: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let process = tekes_ui_module_runtime::ProcessControl::new();
        let (tx, rx) = mpsc::sync_channel(1);
        Self {
            id,
            instance,
            local: None,
            module_id,
            launch_tx: Some(tx),
            inbox,
            process,
            rx,
            image: None,
            frame_size: (0, 0),
            frame_pixels: (0, 0),
            viewport: 0,
            frame_viewport: 0,
            frame_seq: 0,
            scale: 1.,
            bounds: Rect::default(),
            toolbar_generation: 0,
            toolbar_height: 0.,
            toolbar: Vec::new(),
            seq: 0,
            failed: None,
            pid: None,
        }
    }
    fn start(&mut self, wake: WindowWake) {
        if self.local.is_some() {
            return;
        }
        let Some(tx) = self.launch_tx.take() else {
            return;
        };
        let work = self.inbox.clone();
        let control = self.process.clone();
        let module_id = self.module_id.clone();
        std::thread::spawn(move || {
            let run = || -> Result<()> {
                let mut c = ModuleProcess::spawn_with_control(&module_id, control)?;
                // At most two immutable frames may be outstanding. Prepare the
                // next while the previous frame is being imported/presented;
                // completion acknowledgements still bound GPU resource lifetime.
                let mut outstanding = VecDeque::new();
                loop {
                    if outstanding.len() == 2 {
                        let done: Receiver<()> = outstanding.pop_front().unwrap();
                        loop {
                            match done.recv_timeout(std::time::Duration::from_millis(20)) {
                                Ok(()) => break,
                                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                                Err(mpsc::RecvTimeoutError::Timeout) => {
                                    if work.closed.load(std::sync::atomic::Ordering::Acquire) {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                    let request = {
                        let mut q = work.queue.lock().unwrap();
                        while q.is_empty()
                            && !work.closed.load(std::sync::atomic::Ordering::Acquire)
                        {
                            q = work.cv.wait(q).unwrap();
                        }
                        if work.closed.load(std::sync::atomic::Ordering::Acquire) {
                            break;
                        }
                        q.pop_front().unwrap()
                    };
                    let frame = c.request(&request)?;
                    let (at, ar) = mpsc::sync_channel(1);
                    if tx.send(Ok(Pending { frame, ack: at })).is_err() {
                        break;
                    }
                    outstanding.push_back(ar);
                    wake.wake();
                }
                Ok(())
            };
            if let Err(e) = run() {
                let _ = tx.send(Err(format!("{e:#}")));
                wake.wake();
            }
        });
    }
    fn request(&mut self, kind: InputKind, x: f32, y: f32) -> Result<()> {
        if self.failed.is_some() || self.launch_tx.is_some() {
            return Ok(());
        }
        if kind == InputKind::Command {
            if let Some(item) = self.toolbar.first() {
                return self.command(item.id);
            }
            return Ok(());
        }
        self.enqueue(kind, x, y, None)
    }
    fn command(&mut self, id: u64) -> Result<()> {
        let Some(item) = self.toolbar.iter().find(|item| item.id == id) else {
            return Ok(());
        };
        if self.failed.is_some()
            || !item.enabled
            || (item.geometry_dependent && self.frame_viewport != self.viewport)
        {
            return Ok(());
        }
        self.enqueue(InputKind::Command, 0., 0., Some(id))
    }
    fn enqueue(&mut self, kind: InputKind, x: f32, y: f32, command: Option<u64>) -> Result<()> {
        if let Some(local) = &mut self.local {
            if let Some(command) = command {
                local.command(command)?;
            }
            if kind == InputKind::Cancel {
                local.cancel();
            }
            self.seq += 1;
            return Ok(());
        }
        self.seq += 1;
        let r = Request {
            version: VERSION,
            command,
            seq: self.seq,
            viewport: self.viewport,
            kind,
            x,
            y,
            width: self.bounds.size.width.max(1.) as u32,
            height: (self.bounds.size.height - self.toolbar_height).max(1.) as u32,
            identity: self.id,
            scale: self.scale,
        };
        let mut q = self.inbox.queue.lock().unwrap();
        if kind == InputKind::Layout
            && let Some(last) = q.back_mut()
            && last.kind == InputKind::Layout
        {
            *last = r;
            self.inbox.cv.notify_one();
            return Ok(());
        }
        ensure!(q.len() < 256, "input queue full");
        q.push_back(r);
        self.inbox.cv.notify_one();
        Ok(())
    }
    fn geometry(&mut self, bounds: Rect, scale: f32, toolbar_height: f32) -> Result<()> {
        // The wire viewport currently uses whole logical points. Don't render
        // another identical texture for fractional pointer motion within one pt.
        let changed = (
            self.bounds.size.width as u32,
            self.bounds.size.height as u32,
        ) != (bounds.size.width as u32, bounds.size.height as u32)
            || self.scale != scale
            || self.toolbar_height != toolbar_height;
        self.bounds = bounds;
        self.scale = scale;
        self.toolbar_height = toolbar_height;
        if changed {
            // A height-only content resize can leave the toolbar paint frame
            // unchanged. Revoke old arms before advancing viewport ownership.

            self.viewport += 1;
            eprintln!(
                "GEOMETRY panel={} viewport={} width={:.3} height={:.3} toolbar_height={:.3} scale={:.3}",
                self.id,
                self.viewport,
                bounds.size.width,
                bounds.size.height,
                toolbar_height,
                scale
            );
            self.request(InputKind::Layout, 0., 0.)?;
        }
        Ok(())
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        self.process.cancel();
        self.inbox
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
        self.inbox.cv.notify_all();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarTarget {
    Visibility(usize),
    Navigation,
    Panel(usize),
}

pub struct Host {
    settings: Option<std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>>,
    settings_back: bool,
    settings_options: Option<crate::settings::SettingsOptions>,
    settings_child: Option<tekes_ui_platform_macos::WindowHandle>,
    is_settings_window: bool,
    settings_frames: Vec<Rect>,
    settings_top: f32,
    settings_pointer: Option<bool>,
    settings_provider: Option<tekes_ui_settings::FileProvider>,
    settings_loading: Option<Receiver<Result<tekes_ui_settings::Snapshot, String>>>,
    leading_visible: bool,
    sidebar_motion: crate::sidebar_motion::SidebarMotion,
    motion_epoch: std::time::Instant,
    reduced_motion: bool,
    trailing_visible: bool,
    toolbar: crate::window_toolbar::MountedToolbar,
    toolbar_ax: Vec<(
        tekes_ui_platform_macos::AccessibilityKey,
        tekes_ui_controls::composition::ToolbarAccessibilityTarget,
    )>,
    toolbar_ax_next: u64,
    toolbar_focus: Option<ToolbarTarget>,
    toolbar_focus_visible: bool,
    toolbar_focus_lease: Option<(u64, crate::window_toolbar::Source)>,
    navigation_options: Option<NavigationOptions>,
    navigation: Option<WindowNavigation>,
    navigation_ack: Option<Receiver<()>>,
    navigation_order: Option<Vec<usize>>,
    navigation_noticed: bool,
    navigation_title: Option<String>,
    module_ids: Vec<String>,
    column_definitions: Option<Vec<crate::Column>>,
    panels: Vec<Panel>,
    renderer: Option<Renderer>,
    text: Text,
    toolbar_images: std::collections::HashMap<(String, u32), tekes_ui_scene::ImageId>,
    outer: SplitView,
    inner: SplitView,
    outer_view: SplitViewView,
    inner_view: SplitViewView,
    drag_split: Option<usize>,
    pressed: Option<usize>,
    focused: usize,
    completion: Option<SyncSender<Vec<SyncSender<()>>>>,
    order: Vec<usize>,
    acceptance: bool,
    step: u32,
}
impl Host {
    pub(crate) fn new(
        module_ids: Vec<String>,
        acceptance: bool,
        navigation_options: Option<NavigationOptions>,
    ) -> Self {
        let n = module_ids.len();
        Self {
            settings: None,
            settings_back: false,
            settings_options: None,
            settings_child: None,
            is_settings_window: false,
            settings_frames: Vec::new(),
            settings_top: 0.,
            settings_pointer: None,
            settings_provider: None,
            settings_loading: None,
            leading_visible: n != 3,
            sidebar_motion: crate::sidebar_motion::SidebarMotion::new(n != 3, true),
            motion_epoch: std::time::Instant::now(),
            reduced_motion: false,
            trailing_visible: true,
            toolbar: crate::window_toolbar::MountedToolbar::new(),
            toolbar_ax: vec![],
            toolbar_ax_next: 1 << 63,
            toolbar_focus: None,
            toolbar_focus_visible: false,
            toolbar_focus_lease: None,
            navigation_options,
            navigation: None,
            navigation_ack: None,
            navigation_order: None,
            navigation_noticed: false,
            navigation_title: None,
            module_ids,
            column_definitions: None,
            panels: Vec::new(),
            renderer: None,
            text: Text::new(),
            toolbar_images: std::collections::HashMap::new(),
            outer: SplitView::new(1. / 3.),
            inner: SplitView::new(2. / 3.),
            outer_view: SplitViewView::new(Axis::Horizontal),
            inner_view: SplitViewView::new(Axis::Horizontal),
            drag_split: None,
            pressed: None,
            focused: if n >= 2 { 1 } else { 0 },
            completion: None,
            order: (0..n).collect(),
            acceptance,
            step: 0,
        }
    }
    pub(crate) fn from_columns(
        columns: Vec<crate::Column>,
        acceptance: bool,
        navigation: Option<NavigationOptions>,
    ) -> Self {
        let ids = columns.iter().map(|c| c.id().to_owned()).collect();
        let mut host = Self::new(ids, acceptance, navigation);
        host.column_definitions = Some(columns);
        host
    }
    pub(super) fn mount_columns(&mut self) -> bool {
        if let Some(columns) = self.column_definitions.take() {
            self.panels = columns
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut panel = Panel::new(i as u32, c.id().to_owned());
                    if let crate::Column::Container(container) = c {
                        panel.toolbar = container.toolbar.clone();
                        panel.local = Some(container);
                        panel.launch_tx = None;
                    }
                    panel
                })
                .collect();
            return true;
        }
        false
    }
    fn init(&mut self, w: &Window) -> Result<()> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let renderer = unsafe {
            Renderer::new_for_metal_layer(w.metal_layer(), w.drawable_size(), w.scale() as f32)
        }?;
        w.set_text_key_events(true);
        let device = renderer.gpu_context().device().clone();
        let (tx, rx) = mpsc::sync_channel::<Vec<SyncSender<()>>>(16);
        let completion_wake = w.wake_proxy();
        std::thread::spawn(move || {
            while let Ok(acks) = rx.recv() {
                if device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
                    break;
                }
                for ack in acks {
                    let _ = ack.send(());
                }
                completion_wake.wake();
            }
        });
        self.completion = Some(tx);
        self.renderer = Some(renderer);
        if let Some(options) = self.navigation_options.take() {
            self.navigation = Some(WindowNavigation::new(
                options,
                w.wake_proxy(),
                self.toolbar.window,
            )?);
        }
        if self.mount_columns() {
            return Ok(());
        }
        self.panels = self
            .module_ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, p)| Panel::new(i as u32, p))
            .collect();
        Ok(())
    }
    fn draw(&mut self, w: &Window) -> Result<()> {
        self.init(w)?;
        if !self.is_settings_window { self.poll_settings_open(w)?; }
        self.text
            .begin_frame(self.motion_epoch.elapsed(), self.reduced_motion);
        if self.is_settings_window { return self.draw_settings(w); }
        let progress = self.sidebar_motion.sample(
            self.motion_epoch.elapsed(),
            self.reduced_motion,
            w.is_active(),
        )?;
        let transitioning = self.sidebar_motion.needs_frame();
        if let Some(nav) = &mut self.navigation {
            if self
                .navigation_ack
                .as_ref()
                .is_some_and(|ack| ack.try_recv().is_ok())
            {
                self.navigation_ack = None;
                self.navigation_order = None;
                nav.published()?;
            }
            if let Err(error) = nav.poll() {
                nav.error = Some(error.to_string());
                nav.host.input_open = false;
                eprintln!("NAV_ERROR {error:#}");
                w.set_title(&format!("Navigation: {error}"));
            }
            if self.navigation_order.is_none()
                && let Some(label) = nav.publication_label()?
            {
                self.pressed = None;
                self.toolbar.host.event(&ContentEvent::Cancel);
                for p in &mut self.panels {
                    p.request(InputKind::Cancel, 0., 0.)?;
                }
                let title = format!("TekesUI AppHost · {label}");
                w.set_title(&title);
                self.navigation_title = Some(title);
                self.navigation_order = Some(self.order.clone());

            }
        }
        self.outer.min_pane = MIN_COLUMN;
        self.inner.min_pane = MIN_COLUMN;
        let mut scene = Scene::new();
        let mut toolbar_scene = Scene::new();
        let size = w.size();
        let whole = Rect::new(0., 0., size.width as f32, size.height as f32);
        let row_height = tekes_ui_controls::ToolbarControl::window().row_height(&mut self.text);

        // Workspace panes are flat layout regions, not decorated Panel cards.
        // Keep component defaults intact; the consumer chooses no frame/shadow.
        let palette = Palette::from(Theme::LIGHT);
        scene.push_quad(Quad::new(whole, palette.color("surface-1").unwrap()));
        let mut frames = if self.panels.len() == 1 {
            vec![whole]
        } else if self.panels.len() == 2 {
            let layout = self.outer_view.arrange(&mut self.outer, whole, None);
            vec![layout.leading.frame(), layout.trailing.frame()]
        } else {
            let a = self.outer_view.arrange(&mut self.outer, whole, None);
            let b = self.inner_view.arrange(&mut self.inner, whole, None);
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
        if self.panels.len() >= 2 {
            frames = presented_frames(&frames, whole, progress, w.scale() as f32);
        }
        self.text.scale = w.scale() as f32;
        let mut acks = Vec::new();
        let renderer = self.renderer.as_mut().unwrap();
        for p in &mut self.panels {
            while let Ok(result) = p.rx.try_recv() {
                match result {
                    Err(e) => {
                        eprintln!("PANEL_FAILED id={} {e}", p.id);
                        p.failed = Some(e);
                    }
                    Ok(received) => {
                        let mut f = received.frame;
                        // A continuous resize may always be one viewport ahead
                        // of the producer. Present monotonic progress instead of
                        // starving every completed frame until pointer release.
                        // Geometry-dependent input remains gated below.
                        if f.viewport < p.frame_viewport
                            || f.viewport > p.viewport
                            || f.seq <= p.frame_seq
                        {
                            drop(f);
                            let _ = received.ack.send(());
                            continue;
                        }
                        let texture =
                            tekes_ui_module_runtime::import_frame(&renderer.gpu_context(), &mut f)?;
                        let f = f.into_metadata();
                        let image = renderer.gpu_context().import_image(texture)?;
                        let id = renderer.register_gpu_image(&image)?;
                        if let Some(old) = p.image.replace(id) {
                            renderer.unregister_image(old);
                        }
                        p.frame_size = (f.width, f.height);
                        p.frame_pixels = (f.pixel_width, f.pixel_height);
                        p.frame_viewport = f.viewport;
                        p.frame_seq = f.seq;
                        p.pid = Some(f.pid);
                        if serde_json::to_value(&p.toolbar)? != serde_json::to_value(&f.toolbar)? {
                            p.toolbar_generation += 1;
                        }
                        p.toolbar = f.toolbar;
                        eprintln!(
                            "FRAME panel={} pid={} seq={} viewport={} revision={}",
                            p.id, f.pid, f.seq, f.viewport, f.revision
                        );
                        acks.push(received.ack);
                    }
                }
            }
        }
        let _ = renderer;
        let scale = w.scale() as f32;
        let native_center = w.set_toolbar_row_height(f64::from(row_height));
        let (native_height, leading) = w.titlebar_safe_area();
        let (toolbar_height, toolbar_y, toolbar_paint_height) = window_toolbar_geometry(
            native_height as f32,
            row_height,
            native_center.map(|y| y as f32),
            scale,
        );
        let panel_input_open = self
            .navigation
            .as_ref()
            .is_none_or(|nav| nav.host.input_open && !nav.busy);
        let title = self
            .navigation
            .as_ref()
            .and_then(|n| n.title())
            .unwrap_or("")
            .to_owned();
        for (slot, bounds) in frames.iter().enumerate() {
            if bounds.size.width > 0. {
                self.panel_geometry(self.order[slot], *bounds, scale, toolbar_height)?;
            }
        }
        let renderer = self.renderer.as_mut().unwrap();
        for (slot, bounds) in frames.iter().enumerate() {
            let index = self.order[slot];
            if bounds.size.width <= 0. {
                self.panels[index].bounds = Rect::default();
                if self.focused == index {
                    self.focused = self.order[1];
                }
                continue;
            }
            let p = &mut self.panels[index];
            p.start(w.wake_proxy());
            p.geometry(*bounds, scale, toolbar_height)?;
            let content = content_geometry(*bounds, p.toolbar_height);
            scene.push_quad(Quad::new(content, palette.color("surface-panel").unwrap()));
            if let Some(local) = &mut p.local {
                local.render(&mut scene, &mut self.text, content, content, palette)?;
                p.frame_seq = p.seq;
                p.frame_viewport = p.viewport;
                p.frame_size = (content.size.width as u32, content.size.height as u32);
            }
            if let Some(id) = p.image {
                scene.push_clip(content);
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
        }
        let _ = renderer;
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
        self.toolbar
            .host
            .render(&mut toolbar_scene, &mut self.text, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("toolbar paint: {e:?}"))?;
        self.reconcile_toolbar_focus(w.is_active());
        if self.toolbar.host.needs_render() {
            // Focus restoration is admitted only after current shared geometry
            // exists; paint that accepted state in this same native frame.
            toolbar_scene = Scene::new();
            self.toolbar.host.render(&mut toolbar_scene, &mut self.text, Theme::LIGHT.into())
                .map_err(|e| anyhow::anyhow!("toolbar restored focus paint: {e:?}"))?;
        }
        let mut toolbar_controls = self.toolbar.host.content_host().pointer_regions();
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
        self.outer_view
            .set_toolbar_region(band, &toolbar_controls, &mut self.text);
        self.inner_view
            .set_toolbar_region(band, &toolbar_controls, &mut self.text);
        if frames.len() >= 2 && self.leading_visible {
            if transitioning && frames.len() == 3 {
                let gap = Rect::new(
                    frames[0].max_x(),
                    0.,
                    (frames[1].min_x() - frames[0].max_x()).max(0.),
                    whole.size.height,
                );
                self.outer_view.paint_presented_divider(
                    &mut scene,
                    &self.outer,
                    toolbar_divider_frame(gap, toolbar_y),
                    palette,
                    w.scale() as f32,
                );
            } else if !transitioning {
                self.outer_view.paint_presented_divider(
                    &mut scene,
                    &self.outer,
                    toolbar_divider_frame(self.outer_view.layout().divider.frame(), toolbar_y),
                    palette,
                    scale,
                );
            }
        }
        if frames.len() == 3 && self.trailing_visible {
            if transitioning {
                let gap = Rect::new(
                    frames[1].max_x(),
                    0.,
                    (frames[2].min_x() - frames[1].max_x()).max(0.),
                    whole.size.height,
                );
                self.inner_view.paint_presented_divider(
                    &mut scene,
                    &self.inner,
                    toolbar_divider_frame(gap, toolbar_y),
                    palette,
                    w.scale() as f32,
                );
            } else {
                self.inner_view.paint_presented_divider(
                    &mut scene,
                    &self.inner,
                    toolbar_divider_frame(self.inner_view.layout().divider.frame(), toolbar_y),
                    palette,
                    scale,
                );
            }
        }
        let mut resize_regions = Vec::new();
        if frames.len() >= 2 && self.leading_visible && !transitioning {
            resize_regions.extend(self.outer_view.resize_regions(&self.outer));
        }
        if frames.len() == 3 && self.trailing_visible && !transitioning {
            resize_regions.extend(self.inner_view.resize_regions(&self.inner));
        }
        w.set_cursor_regions(
            resize_regions
                .into_iter()
                .map(|(r, axis)| tekes_ui_platform_macos::CursorRegion {
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
                })
                .collect(),
        );
        for primitive in toolbar_scene.primitives() {
            scene.push(primitive.clone());
        }
        let renderer = self.renderer.as_mut().unwrap();
        renderer.resize(w.drawable_size(), w.scale() as f32);
        renderer.render(&scene, &mut self.text.atlas, Rgba::gray(1.))?;
        if let Some(settings) = &self.settings
            && settings.borrow().open
        {
            use tekes_ui_platform_macos::content_accessibility::{ContentAccessibilitySnapshot,ContentAccessibilityOwner,WindowAccessibilityPublication};
            let publication=if w.is_active(){ContentAccessibilitySnapshot::from_host(&settings.borrow().form.host)
                .and_then(|snapshot|WindowAccessibilityPublication::prepare(vec![(ContentAccessibilityOwner(u64::MAX),snapshot)]))}
                else{WindowAccessibilityPublication::prepare(Vec::new())};
            w.set_content_accessibility_publication(publication);
            w.set_text_input_state(if w.is_active() {
                settings.borrow().text_state()
            } else {
                None
            });
        } else {
            w.set_content_accessibility_publication(
                self.content_accessibility_publication(w.is_active() && panel_input_open),
            );
            w.set_text_input_state(self.local_text_state(w.is_active() && panel_input_open));
        }
        if cfg!(debug_assertions)
            && (transitioning || w.is_animating())
            && let Some(nav) = &self.navigation
        {
            nav.trace(serde_json::json!({
                "event":"sidebar-layout-sample", "progress":progress,
                "active":w.is_active(), "animating":transitioning,
                "display_link_frames":w.animation_frame_count(),
                "bounds":frames.iter().map(|r| [r.min_x(),r.min_y(),r.size.width,r.size.height]).collect::<Vec<_>>()
            }));
        }
        if self.navigation_order.is_some()
            && self.navigation_ack.is_none()
            && self.panels.iter().all(|p| {
                p.bounds.size.width <= 0.
                    || ((p.local.is_some() || p.image.is_some())
                        && p.frame_viewport == p.viewport
                        && p.frame_size
                            == (
                                p.bounds.size.width as u32,
                                (p.bounds.size.height - p.toolbar_height).max(1.) as u32,
                            ))
            })
        {
            let (tx, rx) = mpsc::sync_channel(1);
            acks.push(tx);
            self.navigation_ack = Some(rx);
        }
        if !acks.is_empty() {
            self.completion
                .as_ref()
                .unwrap()
                .try_send(acks)
                .map_err(|_| anyhow::anyhow!("GPU completion backlog"))?;
        }
        if let Some(nav) = &self.navigation
            && nav.host.input_open
            && !nav.busy
        {
            let name = nav
                .host
                .target
                .as_ref()
                .map(|e| e.id.as_str())
                .unwrap_or("empty");
            let title = format!("TekesUI AppHost · {name}");
            if self.navigation_title.as_ref() != Some(&title) {
                w.set_title(&title);
                self.navigation_title = Some(title);
            }
            if !self.navigation_noticed {
                eprintln!(
                    "NAV_WINDOW_READY host_pid={} nav_status={:?} view_pids={:?} order={:?} toolbar={:?}",
                    std::process::id(),
                    nav.process.status(),
                    self.panels.iter().map(|p| p.pid).collect::<Vec<_>>(),
                    self.order,
                    self.toolbar.host.content_host().pointer_regions()
                );
                self.navigation_noticed = true;
            }
        }
        self.accept(w)?;
        let local_needs_frame = self.panels.iter_mut().any(|p| {
            let mut needs = false;
            if p.bounds.size.width > 0.
                && let Some(local) = &mut p.local
            {
                local.with_host(&mut |h| needs = h.needs_render());
            }
            needs
        });
        let animate = (self.toolbar.host.needs_render()
            || local_needs_frame
            || self.sidebar_motion.needs_frame()
            || self.text.animation_requested())
            && w.is_active();
        if !w.set_animating(animate) && animate {
            w.request_redraw();
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
            if let Some(nav) = &self.navigation {
                nav.trace(serde_json::json!({"event":"window-typography","body_size":next,"type_scale":self.text.type_scale()}));
            }
        }
        true
    }
    fn toggle_visibility(&mut self, side: usize) -> Result<()> {
        if self.is_settings_window
            && side == 0
            && let Some(settings) = &mut self.settings
        {
            let mut settings = settings.borrow_mut();
            settings.sidebar.host.cancel();
            settings.sidebar.host.focus(None, false);
        }

        if side == 0 {
            self.leading_visible = !self.leading_visible;
        } else {
            self.trailing_visible = !self.trailing_visible;
        }
        self.sidebar_motion.retarget(
            side,
            if side == 0 {
                self.leading_visible
            } else {
                self.trailing_visible
            },
            self.motion_epoch.elapsed(),
        )?;
        if let Some(i) = self.pressed.take() {
            self.panels[i].request(InputKind::Cancel, 0., 0.)?;
        }
        self.cancel_toolbar_input();
        self.drag_split = None;
        self.outer_view.cancel(&mut self.outer);
        self.inner_view.cancel(&mut self.inner);
        for panel in &mut self.panels {
            if let Some(local) = &mut panel.local {
                local.cancel();
            }
        }
        let slot = if side == 0 { 0 } else { 2 };
        let hidden = if side == 0 {
            !self.leading_visible
        } else {
            !self.trailing_visible
        };
        if hidden && !self.is_settings_window {
            let i = self.order[slot];
            self.panels[i].request(InputKind::Cancel, 0., 0.)?;
            // Keep paint alive until the reveal reaches zero; input is gated
            // below immediately and the worker remains mounted for reopening.
            if self.focused == i {
                self.focused = self.order[1];
            }
        }
        if let Some(nav) = &self.navigation {
            nav.trace(serde_json::json!({"event":"panel-visibility","leading":self.leading_visible,"trailing":self.trailing_visible,"pids":self.panels.iter().map(|p|p.pid).collect::<Vec<_>>()}));
        }
        Ok(())
    }
    fn accept(&mut self, w: &Window) -> Result<()> {
        if !self.acceptance {
            return Ok(());
        }
        if self.panels.iter().any(|p| {
            p.bounds.size.width > 0.
                && (p.local.is_none() && (p.pid.is_none() || p.image.is_none()))
        }) {
            return Ok(());
        }
        // This exercises the same Host dispatch/render path but is deliberately
        // reported as automated native-window acceptance, not physical input.
        if self.step == 0 {
            self.step = 1;
            self.panels[self.focused].request(InputKind::Command, 0., 0.)?;
        } else if self.step < 22 {
            self.step += 1;
            // Drive a resize burst faster than workers can produce frames,
            // exercising coalescing and the bounded two-frame pipeline.
            w.set_content_size(WindowSize {
                width: 1000. + ((self.step * 37) % 200) as f64,
                height: 600. + ((self.step * 11) % 60) as f64,
            });
        } else if self.step == 22 {
            self.step = 23;
            w.set_content_size(WindowSize {
                width: 1100.,
                height: 640.,
            });
        } else if self.step == 23 && self.panels.iter().all(|p| p.frame_viewport == p.viewport) {
            self.step = 24;
            w.set_title("TekesUI AppHost — native window ready");
            eprintln!(
                "NATIVE_WINDOW_READY panels={} pids={:?} resize_burst=21 max_inflight_per_panel=2 physical_input=false",
                self.panels.len(),
                self.panels.iter().map(|p| p.pid).collect::<Vec<_>>()
            );
        }
        Ok(())
    }
    fn event(&mut self, event: Event, w: &mut Window) -> Result<()> {
        if let Event::Accessibility{target,..}=&event
            && w.content_accessibility_publication().and_then(|p|p.owner(*target)).is_none(){return Ok(());}

        if matches!(
            &event,
            Event::Command(tekes_ui_platform_macos::CommandId(0x534554))
        ) || matches!(&event,Event::Key(k) if k.kind==KeyEventKind::Down && k.modifiers.command && k.characters_ignoring_modifiers==",")
        {
            self.cancel_toolbar_input();
            if let Some(i) = self.pressed.take() {
                self.panels[i].request(InputKind::Cancel, 0., 0.)?;
            }
            self.drag_split = None;
            if self.is_settings_window {
                w.request_redraw();
                return Ok(());
            }
            self.open_settings(w)?;
            w.request_redraw();
            return Ok(());
        }
        if self.settings_navigation_focus_guard(&event, w.content_accessibility_publication().as_ref()) {
            w.request_redraw();
            return Ok(());
        }
        if let Event::Key(key) = &event
            && self.typography_key_input(key)
        {
            self.draw(w)?;
            return Ok(());
        }
        if self.route_toolbar_event(&event, w)? {
            // Publish current geometry before the next native input/AX callback.
            self.draw(w)?;
            return Ok(());
        }

        if self.settings.as_ref().is_some_and(|s| s.borrow().open)
            && self.settings_event(&event, w)? { return Ok(()); }
        if !self.is_settings_window && self.local_content_event(&event, w)? {
            w.request_redraw();
            return Ok(());
        }
        if let Event::Mouse(m) = &event
            && cfg!(debug_assertions)
            && matches!(
                m.kind,
                MouseEventKind::Down { .. } | MouseEventKind::Up { .. }
            )
            && let Some(nav) = &self.navigation
        {
            nav.trace(serde_json::json!({"event":"mouse","kind":format!("{:?}",m.kind),"x":m.position.x,"y":m.position.y,"active":w.is_active(),"key_window":w.is_key_window()}));
        }
        if cfg!(debug_assertions)
            && let Event::Key(key) = &event
            && let Some(nav) = &self.navigation
        {
            nav.trace(serde_json::json!({"event":"window-toolbar-key","code":key.key_code,"kind":format!("{:?}",key.kind),"active":w.is_active(),"owner":format!("{:?}",self.toolbar.host.content_host().focused())}));
        }
        match event {
            Event::Created | Event::Redraw | Event::Wake | Event::AnimationFrame(_) => {
                self.draw(w)?
            }
            Event::Resized { .. } => {
                self.cancel_toolbar_input();
                self.draw(w)?;
            }
            Event::Mouse(m) if w.is_active() => {
                let content_event = toolbar_pointer(m);
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                match m.kind {
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count,
                    } => {
                        if self.panels.len() >= 2
                            && self.leading_visible
                            && !self.sidebar_motion.needs_frame()
                            && self.outer_view.pointer_down(p, &mut self.outer)
                        {
                            self.cancel_toolbar_input();
                            self.drag_split = Some(0);
                        } else if self.panels.len() == 3
                            && self.trailing_visible
                            && !self.sidebar_motion.needs_frame()
                            && self.inner_view.pointer_down(p, &mut self.inner)
                        {
                            self.cancel_toolbar_input();
                            self.drag_split = Some(1);
                        } else if let Some(i) = self.panels.iter().enumerate().position(|(i, v)| {
                            !self.sidebar_motion.needs_frame()
                                && (self.leading_visible || i != self.order[0])
                                && (self.panels.len() < 3
                                    || self.trailing_visible
                                    || i != self.order[2])
                                && content_geometry(v.bounds, v.toolbar_height).contains(p)
                        }) {
                            for (other, panel) in self.panels.iter_mut().enumerate() {
                                if other != i
                                    && let Some(local) = &mut panel.local
                                {
                                    local.cancel();
                                }
                            }
                            self.focused = i;
                            self.cancel_toolbar_input();
                            if self
                                .navigation
                                .as_ref()
                                .is_none_or(|nav| nav.host.input_open && !nav.busy)
                                && self.panels[i].frame_viewport == self.panels[i].viewport
                                && self.panels[i].frame_size
                                    == (
                                        self.panels[i].bounds.size.width as u32,
                                        (self.panels[i].bounds.size.height
                                            - self.panels[i].toolbar_height)
                                            as u32,
                                    )
                            {
                                self.pressed = Some(i);
                                if let Some(local) = &mut self.panels[i].local {
                                    local.event(&content_event, &mut self.text)?;
                                }
                                let b = self.panels[i].bounds;
                                let toolbar_height = self.panels[i].toolbar_height;
                                self.panels[i].request(
                                    InputKind::PointerDown,
                                    content_point(b, toolbar_height, p).x,
                                    content_point(b, toolbar_height, p).y,
                                )?;
                            }
                        } else if p.y >= 0.
                            && self.panels.iter().any(|panel| p.y < panel.toolbar_height)
                        {
                            // Empty segmented toolbar chrome is not child
                            // content. Splitter seams were offered capture
                            // above; remaining blank chrome belongs to Window.
                            if click_count == 2 {
                                w.try_titlebar_double_click();
                            } else {
                                w.try_begin_titlebar_drag();
                            }
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        self.outer_view.pointer_up(&mut self.outer);
                        self.inner_view.pointer_up(&mut self.inner);
                        self.drag_split = None;
                        if let Some(i) = self.pressed.take() {
                            if let Some(local) = &mut self.panels[i].local {
                                local.event(&content_event, &mut self.text)?;
                            }
                            let b = self.panels[i].bounds;
                            let toolbar_height = self.panels[i].toolbar_height;
                            self.panels[i].request(
                                InputKind::PointerUp,
                                content_point(b, toolbar_height, p).x,
                                content_point(b, toolbar_height, p).y,
                            )?;
                        }
                    }
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } => match self.drag_split {
                        Some(0) => {
                            self.outer_view.pointer_move(p, &mut self.outer);
                            let whole = self.outer_view.layout().frame.frame();
                            let second_x = if self.panels.len() == 2 {
                                whole.max_x()
                            } else {
                                self.inner.layout(whole, Axis::Horizontal).1.min_x()
                            };
                            let limit = (second_x - self.outer.divider - MIN_COLUMN)
                                / (whole.size.width - self.outer.divider);
                            self.outer.set_ratio(self.outer.ratio().min(limit));
                        }
                        Some(1) => {
                            self.inner_view.pointer_move(p, &mut self.inner);
                            let whole = self.inner_view.layout().frame.frame();
                            let first_x = self.outer.layout(whole, Axis::Horizontal).1.max_x();
                            let limit =
                                (first_x + MIN_COLUMN) / (whole.size.width - self.inner.divider);
                            self.inner.set_ratio(self.inner.ratio().max(limit));
                        }
                        _ => {
                            if let Some(i) = self.pressed {
                                if let Some(local) = &mut self.panels[i].local {
                                    local.event(&content_event, &mut self.text)?;
                                }
                                let b = self.panels[i].bounds;
                                let toolbar_height = self.panels[i].toolbar_height;
                                self.panels[i].request(
                                    InputKind::PointerMove,
                                    content_point(b, toolbar_height, p).x,
                                    content_point(b, toolbar_height, p).y,
                                )?;
                            }
                        }
                    },
                    MouseEventKind::Moved | MouseEventKind::Entered | MouseEventKind::Exited => {
                        let local_input_open = !self.sidebar_motion.needs_frame()
                            && self.open_toolbar().is_none()
                            && self
                                .navigation
                                .as_ref()
                                .is_none_or(|n| n.host.input_open && !n.busy);
                        for panel in &mut self.panels {
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
                self.reduced_motion = reduced;
                w.request_redraw();
            }
            Event::Focus(true) => {
                eprintln!(
                    "TOOLBAR_WINDOW_ACTIVE window={} active=true",
                    self.toolbar.window
                );
                self.toolbar.host.set_window_active(true);
                self.outer_view.set_window_active(true, &mut self.outer);
                self.inner_view.set_window_active(true, &mut self.inner);
                w.request_redraw();
            }
            Event::Focus(false) => {
                eprintln!(
                    "TOOLBAR_WINDOW_ACTIVE window={} active=false",
                    self.toolbar.window
                );
                self.remember_toolbar_focus(None);
                self.toolbar.host.set_window_active(false);
                self.sidebar_motion.sample(
                    self.motion_epoch.elapsed(),
                    self.reduced_motion,
                    false,
                )?;
                w.set_animating(false);
                if let Some(i) = self.pressed.take() {
                    self.panels[i].request(InputKind::Cancel, 0., 0.)?;
                }
                for panel in &mut self.panels {
                    if let Some(local) = &mut panel.local {
                        local.cancel();
                    }
                }
                self.outer_view.set_window_active(false, &mut self.outer);
                self.inner_view.set_window_active(false, &mut self.inner);
                w.set_cursor_regions(Vec::new());
                self.drag_split = None;
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
impl WindowHandler for Host {
    fn allows_app_quit(&self) -> bool {
        self.settings.as_ref().is_none_or(|s| {
            let s = s.borrow();
            s.clean_for_close()
        })
    }

    fn handle(&mut self, event: Event, w: &mut Window) {
        if let Err(e) = self.event(event, w) {
            eprintln!("HOST_ERROR {e:#}");
            w.set_title(&format!("AppHost error: {e}"));
        }
    }
}
pub fn run(
    module_ids: Vec<String>,
    columns: Option<Vec<crate::Column>>,
    acceptance: bool,
    navigation: Option<NavigationOptions>,
    settings: Option<crate::settings::SettingsOptions>,
) -> Result<()> {
    ensure!(
        (1..=3).contains(&columns.as_ref().map_or(module_ids.len(), Vec::len)),
        "expected one, two or three preselected modules"
    );
    let mut host = if let Some(columns) = columns {
        Host::from_columns(columns, acceptance, navigation)
    } else {
        Host::new(module_ids, acceptance, navigation)
    };
    let has_settings = settings.is_some();
    host.settings_provider = settings
        .as_ref()
        .map(|s| tekes_ui_settings::FileProvider::open(s.file.clone(), s.schema.namespace.clone()))
        .transpose()?;
    host.settings_options = settings;
    let menus = if has_settings {
        MenuBar {
            menus: vec![tekes_ui_platform_macos::Menu::new(
                "Settings",
                vec![tekes_ui_platform_macos::MenuItem::Command(
                    tekes_ui_platform_macos::MenuCommand::new(
                        "Settings…",
                        tekes_ui_platform_macos::CommandId(0x534554),
                    )
                    .key(tekes_ui_platform_macos::KeyEquivalent::command(',')),
                )],
            )],
        }
    } else {
        MenuBar::default()
    };
    App::new(AppConfig {
        name: Some("TekesUI AppHost Prototype".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "TekesUI AppHost Prototype".into(),
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
    fn dormant_panel_has_no_process_and_drops_pre_mount_input() {
        let mut p = Panel::new(0, "demo-first".into());
        assert!(p.process.pid().is_none());
        assert!(p.launch_tx.is_some());
        p.request(InputKind::Cancel, 0., 0.).unwrap();
        p.request(InputKind::PointerDown, 20., 20.).unwrap();
        assert!(p.inbox.queue.lock().unwrap().is_empty());
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
    use tekes_ui_controls::{ToolbarAction, composition::{ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion}};
    fn render(h: &mut Host) {
        h.toolbar.host.render(&mut Scene::new(), &mut h.text, Theme::LIGHT.into()).unwrap();
    }
    fn refresh(h: &mut Host, narrow: Option<usize>) {
        let viewport = Rect::new(0., 0., 1000., 400.);
        let mut regions = Vec::new();
        let mut declarations = Vec::new();
        for (order, owner) in h.toolbar_groups().into_iter().enumerate() {
            let (rect, actions, source) = match owner {
                ToolbarTarget::Visibility(side) => (
                    Rect::new(if side == 0 { 0. } else { 820. }, 0., 48., 52.),
                    vec![ToolbarAction { id: if side == 0 { 0x5601 } else { 0x5602 }, title: "Show".into(), enabled: true }],
                    Source::Visibility(side),
                ),
                ToolbarTarget::Panel(i) => {
                    let p = &h.panels[i];
                    (Rect::new(p.bounds.min_x(), 0., if narrow == Some(i) { 60. } else { 240. }, 52.),
                    p.toolbar.iter().map(|a| ToolbarAction {id:a.id,title:a.title.clone(),enabled:a.enabled && (!a.geometry_dependent || p.frame_viewport == p.viewport)}).collect(),
                    Source::Panel {instance:p.instance,index:i,module:p.module_id.clone(),pid:p.pid,generation:p.toolbar_generation})
                }
                ToolbarTarget::Navigation => unreachable!("fixture has no navigation process"),
            };
            let group = Host::toolbar_group(owner);
            let region = ComponentId(93000 + order as u64);
            regions.push(ToolbarRegion::new(region, rect));
            declarations.push(Declaration { id:group,region,placement:ToolbarPlacement::Navigation,order:0,
                contents:Contents::Commands(actions.into_iter().map(|a|ToolbarButtonDeclaration {id:a.id,icon:None,text:a.title,hint:None,enabled:a.enabled}).collect(),ToolbarGroupOverflow::Split),source });
        }
        h.remember_toolbar_focus(None);
        h.toolbar.sync(viewport, regions, declarations).unwrap();
        render(h);
    }
    fn fixture() -> Host {
        let ids = (0..3).map(|i| format!("window-test-{i}")).collect();
        let mut h = Host::new(ids, false, None);
        h.panels = (0..3).map(|i|Panel::new(i,format!("window-test-{i}"))).collect();
        h.leading_visible = true;
        for (i,p) in h.panels.iter_mut().enumerate() {
            p.bounds=Rect::new(80.+i as f32*240.,0.,240.,300.);
            p.toolbar_height=66.;
            p.toolbar=vec![ToolbarItem{id:7,title:"Increment".into(),enabled:true,geometry_dependent:false},ToolbarItem{id:8,title:"Reset".into(),enabled:true,geometry_dependent:false}];
        }
        refresh(&mut h,None);
        h
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
        h.panels
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                p.inbox
                    .queue
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|r| r.kind == InputKind::Command)
                    .map(|r| (i, r.command.unwrap()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
    fn focused_groups(h: &Host) -> Vec<ToolbarTarget> {
        h.actual_toolbar_owner().into_iter().collect()
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
            assert_eq!(h.toolbar_focus, Some(*target));
            assert_eq!(focused_groups(&h), vec![*target]);
        }
        key(&mut h, ControlKey::Tab { reverse: true }, true);
        assert_eq!(h.toolbar_focus, Some(ToolbarTarget::Visibility(1)));
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
        h.leading_visible = false;
        h.reconcile_toolbar_focus(true);
        key(&mut h, ControlKey::Space, false);
        assert_eq!(h.toolbar_focus, None);
        assert!(commands(&h).is_empty());
        assert!(!h.toolbar_focus_targets().contains(&ToolbarTarget::Panel(0)));
        h.leading_visible = true;
        h.order = vec![2, 0, 1];
        refresh(&mut h, None);
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(0)), true);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        assert_eq!(h.toolbar_focus, Some(ToolbarTarget::Panel(1)));
        h.panels[1].toolbar = vec![ToolbarItem { id:7,title:"Disabled".into(),enabled:false,geometry_dependent:false }];
        refresh(&mut h, None);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_focus, None);
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
        assert_eq!(h.toolbar_focus, Some(owner));
        let p = Point::new(24., 26.);
        assert_eq!(h.toolbar_target_at(p), Some(owner));
        h.dispatch_toolbar(owner, &ContentEvent::PointerDown(p))
            .unwrap();
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(p))
            .unwrap();
        assert!(h.leading_visible);
        assert!(h.trailing_visible);
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
        assert_eq!(h.toolbar.host.content_host().pointer_capture().map(|(id, _)| id), Some(Host::toolbar_control(owner)));
        key(&mut h, ControlKey::Escape, true);
        h.dispatch_toolbar(owner, &ContentEvent::PointerUp(p))
            .unwrap();
        assert!(commands(&h).is_empty());
        assert_eq!(h.toolbar.host.content_host().pointer_capture(), None);
        let mut h = Host::new(vec!["empty".into()], false, None);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        key(&mut h, ControlKey::Tab { reverse: true }, true);
        assert_eq!(h.toolbar_focus, None);
    }
    #[test]
    fn temporary_input_barrier_restores_logical_owner_but_never_old_arm_or_stolen_focus() {
        let mut h = fixture();
        let owner = ToolbarTarget::Panel(1);
        h.set_toolbar_focus(Some(owner), true);
        key(&mut h, ControlKey::Space, true);
        h.sidebar_motion
            .retarget(0, true, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(h.sidebar_motion.needs_frame());
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_focus, Some(owner));
        assert!(focused_groups(&h).is_empty());
        key(&mut h, ControlKey::Escape, true);
        assert_eq!(h.toolbar_focus, Some(owner));
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        h.sidebar_motion
            .sample(std::time::Duration::from_secs(2), false, true)
            .unwrap();
        h.reconcile_toolbar_focus(true);
        assert_eq!(focused_groups(&h), vec![owner]);
        key(&mut h, ControlKey::Space, false);
        assert!(commands(&h).is_empty());
        h.sidebar_motion
            .retarget(0, false, std::time::Duration::from_secs(3))
            .unwrap();
        h.reconcile_toolbar_focus(true);
        key(&mut h, ControlKey::Tab { reverse: false }, true);
        assert_eq!(h.toolbar_focus, Some(ToolbarTarget::Visibility(1)));
        h.sidebar_motion
            .sample(std::time::Duration::from_secs(4), false, true)
            .unwrap();
        h.reconcile_toolbar_focus(true);
        assert_eq!(focused_groups(&h), vec![ToolbarTarget::Visibility(1)]);
    }
    #[test]
    fn modified_system_tab_does_not_enter_or_restyle_a_toolbar_focus_scope() {
        let mut h = fixture();
        h.set_toolbar_focus(Some(ToolbarTarget::Panel(1)), false);
        render(&mut h);
        assert!(!h.toolbar.host.needs_render());
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
        assert_eq!(h.toolbar_focus, Some(ToolbarTarget::Panel(1)));
        assert!(!h.toolbar.host.needs_render());
        assert!(commands(&h).is_empty());
    }
    #[test]
    fn unavailable_toolbar_rejects_action_but_leaves_native_chrome_drag_available() {
        let mut h = fixture();
        h.leading_visible = false;
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
        h.panels[1].viewport = 1;
        for item in &mut h.panels[1].toolbar {
            item.geometry_dependent = true;
        }
        refresh(&mut h, None);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_focus, Some(owner));
        assert!(focused_groups(&h).is_empty());
        h.panels[1].frame_viewport = 1;
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
        h.panel_geometry(1, Rect::new(320., 0., 240., 320.), 1., 66.)
            .unwrap();
        assert_eq!(h.toolbar.host.content_host().focused(), None);
        render(&mut h);
        h.reconcile_toolbar_focus(true);
        assert_eq!(h.toolbar_focus, Some(owner));
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
        assert_eq!(super::window_toolbar_geometry(66.,48.,Some(26.),2.),(66.,2.,48.));
        assert_eq!(super::window_toolbar_geometry(40.,32.,Some(20.),1.),(40.,4.,32.));
        assert_eq!(super::window_toolbar_geometry(0.,48.,None,2.),(48.,0.,48.));
        // Oversize retains complete content rather than a negative, clipped origin.
        assert_eq!(super::window_toolbar_geometry(66.,80.,Some(26.),2.),(80.,0.,80.));
        assert_eq!(super::window_toolbar_geometry(65.75,48.,Some(25.75),2.),(66.,1.75,48.));
    }
}

#[cfg(test)]
mod settings_layout_tests {
    use super::*;
    #[test]
    fn settings_layout_reuses_only_leading_visibility_toolbar_and_can_block_quit() {
        let mut h = Host::new(vec![], false, None);
        h.is_settings_window = true;
        assert_eq!(h.panels.len(), 0);
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
            include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml"),
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
        host.is_settings_window = true;
        host.settings_back = surface.exclusive;
        host.settings = Some(std::rc::Rc::new(std::cell::RefCell::new(surface)));
        assert!(host.allows_app_quit());
        {
            let mut state = host.settings.as_ref().unwrap().borrow_mut();
            let s = &mut *state;
            s.draft.edit(&s.schema.settings[0], "Changed".into());
        }
        assert!(!host.allows_app_quit());
        host.settings.as_ref().unwrap().borrow_mut().draft.cancel();
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
    use tekes_ui_controls::composition::{ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement, ToolbarRegion};
    #[test]
    fn actual_config_group_focus_retires_old_panel_return_intent() {
        let mut host = Host::new(vec!["panel".into()], false, None);
        host.panels = vec![Panel::new(0, "panel".into())];
        host.panels[0].bounds = Rect::new(0.,0.,240.,300.);
        let source = ComponentSource {
            session: tekes_ui_module_runtime::component::Session {module:crate::settings_undo::MODULE_ID.into(),instance:1,host_epoch:1},
            registration:1,window:host.toolbar.window,
        };
        let button = |id, name:&str| Contents::Commands(vec![ToolbarButtonDeclaration {id,icon:None,text:name.into(),hint:None,enabled:true}],ToolbarGroupOverflow::Together);
        host.toolbar.sync(Rect::new(0.,0.,480.,300.),vec![ToolbarRegion::new(ComponentId(10),Rect::new(0.,0.,240.,52.)),ToolbarRegion::new(ComponentId(20),Rect::new(240.,0.,240.,52.))],vec![
            Declaration {id:Host::toolbar_group(ToolbarTarget::Panel(0)),region:ComponentId(10),placement:ToolbarPlacement::Navigation,order:0,contents:button(7,"Panel"),source:Source::Panel{instance:host.panels[0].instance,index:0,module:"panel".into(),pid:None,generation:0}},
            Declaration {id:ComponentId(92300),region:ComponentId(20),placement:ToolbarPlacement::SecondaryAction,order:0,contents:button(crate::settings_undo::UNDO,"Undo"),source:Source::Undo(source)},
        ]).unwrap();
        host.toolbar.host.render(&mut Scene::new(),&mut host.text,Theme::LIGHT.into()).unwrap();
        host.set_toolbar_focus(Some(ToolbarTarget::Panel(0)),true);
        assert_eq!(host.toolbar_focus,Some(ToolbarTarget::Panel(0)));
        let undo = host.toolbar.host.semantics().into_iter().find(|n|n.node.id==ComponentId(92301)).unwrap().target;
        assert!(host.toolbar.host.restore_focus(undo,true));
        host.remember_toolbar_focus(Some(true));
        host.reconcile_toolbar_focus(true);
        assert_eq!(host.toolbar_focus,None);
        assert!(host.toolbar_focus_lease.is_none());
        assert_eq!(host.toolbar.host.content_host().focused(),Some(ComponentId(92301)));
    }
}

#[cfg(test)]
mod settings_close_input_tests {
    use super::*;
    use crate::settings::{SettingsSurface, SettingsWindow};
    use tekes_ui_controls::composition::NumberFieldContent;
    use tekes_ui_settings::{FileProvider, Snapshot};
    fn fixture(schema: &str) -> (Host, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("settings-close-input-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())).join("settings.toml");
        let options=SettingsWindow::from_toml(schema,path.clone()).unwrap();
        let provider=FileProvider::open(path.clone(),options.schema.namespace.clone()).unwrap();
        let mut surface=SettingsSurface::new(options,provider,Snapshot::default()).unwrap();
        surface.show();
        // Ordinary focus/reveal requires current real layout, as in the Window.
        let frame=Rect::new(0.,0.,620.,400.);
        surface.form.render(&mut Scene::new(),&mut Text::new(),frame,Theme::LIGHT.into()).unwrap();
        for (index,field) in surface.view_schema.settings.iter().enumerate() {
            let id=ComponentId(100+index as u64);
            let refusal=surface.form.host.plan_host_reveal(id,Default::default(),&[],Default::default()).err();
            eprintln!("SETTINGS_FIXTURE selected={} field={} kind={:?} component={} focus_targets={:?} reveal_refusal={:?}",surface.selected,field.id,field.kind,surface.form.host.get(id).is_some(),surface.form.host.focus_targets(),refusal);
        }
        let mut host=Host::new(vec![],false,None);
        host.settings=Some(std::rc::Rc::new(std::cell::RefCell::new(surface)));
        (host,path)
    }
    #[test]
    fn raw_number_draft_blocks_quit_and_close_commits_only_valid_input() {
        for text in ["999%","125%"] {
            let (host,path)=fixture(include_str!("../../../examples/app-host-demo/settings.schema.toml"));
            let state=host.settings.as_ref().unwrap();
            {
                let mut s=state.borrow_mut();
                let form=&mut s.form.host;
                assert!(form.focus(Some(ComponentId(101)),true));
                form.event(&ContentEvent::TextCommand("selectAll:".into()));
                form.event(&ContentEvent::Commit{text:text.into(),replacement:None});
                assert!(s.form.has_number_draft());
                assert!(!s.draft.dirty(),"raw editor input is not a saved Draft value");
            }
            assert!(!host.allows_app_quit());
            {
                let mut s=state.borrow_mut();
                assert!(!s.close_requested());
                assert!(s.open);
                if text=="999%" {
                    assert!(!s.draft.dirty());
                    assert!(s.form.has_number_draft());
                    assert_eq!(s.form.host.get(ComponentId(101)).unwrap().component::<NumberFieldContent>().unwrap().edit_state().text,"999%");
                } else {
                    assert!(s.draft.dirty(),"valid raw input must await its persistence receipt");
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
        let (host,path)=fixture(include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml"));
        let state=host.settings.as_ref().unwrap();
        {
            let mut s=state.borrow_mut();
            assert!(s.form.host.focus(Some(ComponentId(100)),true));
            s.form.host.event(&ContentEvent::Preedit{text:"中文".into(),selection:0..2,clauses:std::iter::once(0..2).collect(),replacement:None});
            assert!(s.form.host.is_composing());
            assert!(!s.draft.dirty());
        }
        assert!(!host.allows_app_quit());
        {
            let mut s=state.borrow_mut();
            let before=s.form.host.focused_text().unwrap().input.text.clone();
            let marked=s.form.host.focused_text().unwrap().input.marked.clone();
            assert!(!s.close_requested());
            assert!(s.open);
            assert_eq!(s.form.host.focused_text().unwrap().input.text,before);
            assert_eq!(s.form.host.focused_text().unwrap().input.marked,marked);
            assert!(!s.draft.dirty());
        }
        assert!(!host.allows_app_quit());
        assert!(!path.exists());
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn original_settings_row_focus_succeeds_but_unknown_scroll_layout_stays_rejected() {
        use tekes_ui_controls::{ButtonTextRenderer,ScrollArea,composition::{Component,Content,ContentHost,HostRevealError,ScrollContent,TextFieldContent},layout::MeasuredChild};
        use tekes_ui_scene::Size;
        let (host,path)=fixture(include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml"));
        {
            let mut state=host.settings.as_ref().unwrap().borrow_mut();
            assert!(state.form.host.plan_host_reveal(ComponentId(100),Default::default(),&[],Default::default()).is_ok());
            assert!(state.form.host.focus(Some(ComponentId(100)),true));
            assert_eq!(state.form.host.focused(),Some(ComponentId(100)));
        }
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        #[derive(Debug)]
        struct Unknown { children:Vec<Content> }
        impl Component for Unknown {
            fn children(&self)->&[Content]{&self.children}
            fn children_mut(&mut self)->Option<&mut Vec<Content>>{Some(&mut self.children)}
            fn measure(&mut self,text:&mut dyn ButtonTextRenderer,available:Size)->MeasuredChild{self.children[0].measure(text,available)}
            fn arrange(&mut self,text:&mut dyn ButtonTextRenderer,frame:Rect,clip:Rect){self.children[0].arrange(text,frame,clip);}
        }
        let mut unknown=ContentHost::new(Content::new(ComponentId(1),ScrollContent::new(ScrollArea::default(),Content::new(ComponentId(2),Unknown{children:vec![Content::new(ComponentId(3),TextFieldContent::new("Unknown","draft"))]})))).unwrap();
        let frame=Rect::new(0.,0.,620.,400.);
        unknown.render(&mut Scene::new(),&mut Text::new(),frame,frame,Theme::LIGHT.into()).unwrap();
        assert!(matches!(unknown.plan_host_reveal(ComponentId(3),Default::default(),&[],Default::default()),Err(HostRevealError::UnsupportedLayout)));
        assert!(!unknown.focus(Some(ComponentId(3)),true));
        assert_eq!(unknown.focused(),None);
    }

}
