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
    ControlKey, ToolbarAction, ToolbarControl,
    composition::{
        ComponentId, Content, ContentEvent, ContentHost, ContentKeyEvent, InputModifiers,
        PointerButton, PointerEvent, PointerModifiers, PointerPhase,
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
    toolbar_host: ContentHost,
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
        let inbox = Arc::new(Inbox {
            queue: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            closed: std::sync::atomic::AtomicBool::new(false),
        });
        let process = tekes_ui_module_runtime::ProcessControl::new();
        let (tx, rx) = mpsc::sync_channel(1);
        Self {
            id,
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
            toolbar_host: {
                let mut host =
                    ContentHost::new(Content::new(ComponentId(1), ToolbarControl::default()))
                        .expect("one toolbar action has a unique identity");
                host.set_action_events(true);
                host
            },
            toolbar_height: 0.,
            toolbar: Vec::new(),
            seq: 0,
            failed: None,
            pid: None,
        }
    }
    fn start(&mut self, wake: WindowWake) {
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
    fn toolbar_event(&mut self, event: &ContentEvent) -> Result<bool> {
        self.toolbar_host.event(event);
        let bubbles = self.toolbar_host.event_bubbled();
        let command = self
            .toolbar_host
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap()
            .take_action();
        for action in self.toolbar_host.drain_actions() {
            if action.kind == tekes_ui_controls::composition::ContentActionKind::Activate
                && let Some(id) = command
            {
                self.command(id)?;
            }
        }
        Ok(bubbles)
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
pub struct Host {
    leading_visible: bool,
    trailing_visible: bool,
    visibility_bars: [crate::window_toolbar::VisibilityToolbar; 2],
    navigation_options: Option<NavigationOptions>,
    navigation: Option<WindowNavigation>,
    navigation_ack: Option<Receiver<()>>,
    navigation_order: Option<Vec<usize>>,
    navigation_noticed: bool,
    navigation_title: Option<String>,
    module_ids: Vec<String>,
    panels: Vec<Panel>,
    renderer: Option<Renderer>,
    text: Text,
    outer: SplitView,
    inner: SplitView,
    outer_view: SplitViewView,
    inner_view: SplitViewView,
    drag_split: Option<usize>,
    pressed: Option<usize>,
    toolbar_pressed: Option<usize>,
    focused: usize,
    completion: Option<SyncSender<Vec<SyncSender<()>>>>,
    order: Vec<usize>,
    acceptance: bool,
    step: u32,
}
impl Host {
    fn new(
        module_ids: Vec<String>,
        acceptance: bool,
        navigation_options: Option<NavigationOptions>,
    ) -> Self {
        let n = module_ids.len();
        Self {
            leading_visible: n != 3,
            trailing_visible: true,
            visibility_bars: [
                crate::window_toolbar::VisibilityToolbar::new(0x5601, "tekes-side-bar"),
                crate::window_toolbar::VisibilityToolbar::new(0x5602, "tekes-side-panel"),
            ],
            navigation_options,
            navigation: None,
            navigation_ack: None,
            navigation_order: None,
            navigation_noticed: false,
            navigation_title: None,
            module_ids,
            panels: Vec::new(),
            renderer: None,
            text: Text::new(),
            outer: SplitView::new(1. / 3.),
            inner: SplitView::new(2. / 3.),
            outer_view: SplitViewView::new(Axis::Horizontal),
            inner_view: SplitViewView::new(Axis::Horizontal),
            drag_split: None,
            pressed: None,
            toolbar_pressed: None,
            focused: if n == 3 { 1 } else { 0 },
            completion: None,
            order: (0..n).collect(),
            acceptance,
            step: 0,
        }
    }
    fn init(&mut self, w: &Window) -> Result<()> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let renderer = unsafe {
            Renderer::new_for_metal_layer(w.metal_layer(), w.drawable_size(), w.scale() as f32)
        }?;
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
            self.navigation = Some(WindowNavigation::new(options, w.wake_proxy())?);
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
                self.toolbar_pressed = None;
                for p in &mut self.panels {
                    p.request(InputKind::Cancel, 0., 0.)?;
                    p.toolbar_event(&ContentEvent::Cancel)?;
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
        let size = w.size();
        let whole = Rect::new(0., 0., size.width as f32, size.height as f32);
        let (native_height, leading) = w.titlebar_safe_area();
        // Workspace panes are flat layout regions, not decorated Panel cards.
        // Keep component defaults intact; the consumer chooses no frame/shadow.
        let palette = Palette::from(Theme::LIGHT);
        scene.push_quad(Quad::new(whole, palette.color("surface-1").unwrap()));
        let mut frames = if self.panels.len() == 1 {
            vec![whole]
        } else {
            let mut hidden_splitters = Scene::new();
            let a = self.outer_view.render_at_scale(
                if self.leading_visible {
                    &mut scene
                } else {
                    &mut hidden_splitters
                },
                &mut self.outer,
                whole,
                None,
                Theme::LIGHT.into(),
                w.scale() as f32,
            );
            let b = self.inner_view.render_at_scale(
                if self.trailing_visible {
                    &mut scene
                } else {
                    &mut hidden_splitters
                },
                &mut self.inner,
                whole,
                None,
                Theme::LIGHT.into(),
                w.scale() as f32,
            );
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
        if self.panels.len() == 3 {
            frames = visible_frames(&frames, whole, self.leading_visible, self.trailing_visible);
        }
        let targets = if self.panels.len() > 1 {
            vec![
                if self.leading_visible {
                    self.outer_view.resize_target(&self.outer)
                } else {
                    None
                },
                if self.trailing_visible {
                    self.inner_view.resize_target(&self.inner)
                } else {
                    None
                },
            ]
        } else {
            Vec::new()
        };
        w.set_cursor_regions(
            targets
                .into_iter()
                .flatten()
                .map(|(r, axis)| tekes_ui_platform_macos::CursorRegion {
                    origin: tekes_ui_platform_macos::Point {
                        x: r.origin.x as f64,
                        y: r.origin.y as f64,
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
        let mut toolbar_height = native_height as f32;
        if self.panels.len() == 3 {
            toolbar_height = toolbar_height.max(self.visibility_bars[0].prepare(
                renderer,
                w.scale() as f32,
                whole,
                &mut self.text,
                self.leading_visible,
                if self.leading_visible {
                    "Hide leading panel"
                } else {
                    "Show leading panel"
                },
            )?);
            self.visibility_bars[1].set_secondary(
                0x5603,
                "tekes-swap-horizontal",
                "Swap center and trailing",
                self.trailing_visible && self.navigation.as_ref().is_none_or(|n| !n.busy),
            );
            toolbar_height = toolbar_height.max(self.visibility_bars[1].prepare(
                renderer,
                w.scale() as f32,
                whole,
                &mut self.text,
                self.trailing_visible,
                if self.trailing_visible {
                    "Hide trailing panel"
                } else {
                    "Show trailing panel"
                },
            )?);
        }
        if let Some(nav) = &mut self.navigation {
            toolbar_height = toolbar_height.max(nav.prepare(
                renderer,
                w.scale() as f32,
                whole,
                &mut self.text,
            )?);
        }
        for p in &mut self.panels {
            let toolbar = p
                .toolbar_host
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap();
            toolbar.set_viewport(whole);
            toolbar.set_actions(
                p.toolbar
                    .iter()
                    .map(|item| ToolbarAction {
                        id: item.id,
                        title: item.title.clone(),
                        enabled: p.failed.is_none()
                            && item.enabled
                            && (!item.geometry_dependent || p.frame_viewport == p.viewport),
                    })
                    .collect(),
            );
            toolbar_height = toolbar_height.max(toolbar.measure(&mut self.text).height);
        }
        let scale = w.scale() as f32;
        toolbar_height = (toolbar_height * scale).ceil() / scale;
        if self
            .panels
            .iter()
            .any(|p| p.toolbar_height != toolbar_height)
        {
            eprintln!(
                "TOOLBAR_HEIGHT height={} native={} leading={}",
                toolbar_height, native_height, leading
            );
        }
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
            let reserved = if slot == 0 { leading as f32 } else { 0. };
            let start = (bounds.min_x() + reserved).max(if self.navigation.is_some() {
                leading as f32 + 160.
            } else if frames.len() == 3 {
                leading as f32 + 48.
            } else {
                0.
            });
            let end = bounds.max_x().min(if frames.len() == 3 {
                whole.max_x() - 96.
            } else {
                whole.max_x()
            });
            let bar = Rect::new(
                start,
                bounds.min_y(),
                (end - start).max(0.),
                p.toolbar_height,
            );
            p.toolbar_host
                .render(&mut scene, &mut self.text, bar, whole, Theme::LIGHT.into())
                .map_err(|e| anyhow::anyhow!("toolbar layout: {e:?}"))?;
        }
        if let Some(nav) = &mut self.navigation {
            nav.render(
                &mut scene,
                &mut self.text,
                Rect::new(
                    leading as f32 + if frames.len() == 3 { 48. } else { 0. },
                    0.,
                    112.,
                    toolbar_height,
                ),
                whole,
            )?;
        }
        if frames.len() == 3 {
            self.visibility_bars[0].render(
                &mut scene,
                &mut self.text,
                Rect::new(leading as f32, 0., 48., toolbar_height),
                whole,
            )?;
            self.visibility_bars[1].render(
                &mut scene,
                &mut self.text,
                Rect::new(whole.max_x() - 96., 0., 96., toolbar_height),
                whole,
            )?;
        }
        // Ordinary content is composed first; toolbar-owned popups overlay it.
        for p in &mut self.panels {
            if p.bounds.size.width <= 0. {
                continue;
            }
            p.toolbar_host
                .get_mut(ComponentId(1))
                .unwrap()
                .paint_overlays(&mut scene, &mut self.text, palette.into());
        }
        if let Some(nav) = &mut self.navigation {
            nav.overlay(&mut scene, &mut self.text);
        }
        renderer.resize(w.drawable_size(), w.scale() as f32);
        renderer.render(&scene, &mut self.text.atlas, Rgba::gray(1.))?;
        if self.navigation_order.is_some()
            && self.navigation_ack.is_none()
            && self.panels.iter().all(|p| {
                p.bounds.size.width <= 0.
                    || (p.image.is_some()
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
                    nav.rect
                );
                self.navigation_noticed = true;
            }
        }
        self.accept(w)?;
        Ok(())
    }
    fn swap_center_trailing(&mut self) -> Result<()> {
        if self.panels.len() != 3
            || !self.trailing_visible
            || self.navigation.as_ref().is_some_and(|n| n.busy)
        {
            return Ok(());
        }
        if let Some(i) = self.pressed.take() {
            self.panels[i].request(InputKind::Cancel, 0., 0.)?;
        }
        self.toolbar_pressed = None;
        self.drag_split = None;
        self.outer_view.cancel(&mut self.outer);
        self.inner_view.cancel(&mut self.inner);
        for panel in &mut self.panels {
            panel.toolbar_event(&ContentEvent::Cancel)?;
        }
        self.order.swap(1, 2);
        self.focused = self.order[1];
        if let Some(nav) = &self.navigation {
            nav.trace(serde_json::json!({"event":"panel-swap","order":self.order,"pids":self.panels.iter().map(|p|p.pid).collect::<Vec<_>>()}));
        }
        Ok(())
    }
    fn toggle_visibility(&mut self, side: usize) -> Result<()> {
        if side == 0 {
            self.leading_visible = !self.leading_visible;
        } else {
            self.trailing_visible = !self.trailing_visible;
        }
        if let Some(i) = self.pressed.take() {
            self.panels[i].request(InputKind::Cancel, 0., 0.)?;
        }
        self.toolbar_pressed = None;
        self.drag_split = None;
        self.outer_view.cancel(&mut self.outer);
        self.inner_view.cancel(&mut self.inner);
        for panel in &mut self.panels {
            panel.toolbar_event(&ContentEvent::Cancel)?;
        }
        let slot = if side == 0 { 0 } else { 2 };
        let hidden = if side == 0 {
            !self.leading_visible
        } else {
            !self.trailing_visible
        };
        if hidden {
            let i = self.order[slot];
            self.panels[i].request(InputKind::Cancel, 0., 0.)?;
            self.panels[i].bounds = Rect::default();
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
        if self
            .panels
            .iter()
            .any(|p| p.bounds.size.width > 0. && (p.pid.is_none() || p.image.is_none()))
        {
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
        match event {
            Event::Created | Event::Redraw | Event::Wake => self.draw(w)?,
            Event::Resized { .. } => {
                w.request_redraw();
            }
            Event::Mouse(m) if w.is_active() => {
                let content_event = toolbar_pointer(m);
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                match m.kind {
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if self.panels.len() == 3
                            && let Some(i) =
                                self.visibility_bars.iter().position(|b| b.rect.contains(p))
                        {
                            self.visibility_bars[i].pressed = true;
                            self.visibility_bars[i].event(&content_event);
                        } else if let Some(nav) = &mut self.navigation
                            && (nav.rect.contains(p) || nav.is_open())
                        {
                            nav.pressed = true;
                            if nav.event(&content_event)? {
                                nav.pressed = false;
                                nav.event(&ContentEvent::Cancel)?;
                                w.try_begin_titlebar_drag();
                            }
                        } else if let Some(i) = self.panels.iter().position(|panel| {
                            panel.toolbar_host.get(ComponentId(1)).is_some_and(|c| {
                                c.component::<ToolbarControl>().is_some_and(|t| t.is_open())
                            })
                        }) {
                            self.toolbar_pressed = Some(i);
                            self.panels[i].toolbar_event(&content_event)?;
                        } else if self.panels.len() == 3
                            && self.leading_visible
                            && self.outer_view.pointer_down(p, &mut self.outer)
                        {
                            self.drag_split = Some(0);
                        } else if self.panels.len() == 3
                            && self.trailing_visible
                            && self.inner_view.pointer_down(p, &mut self.inner)
                        {
                            self.drag_split = Some(1);
                        } else if let Some(i) =
                            self.panels.iter().position(|v| v.bounds.contains(p))
                        {
                            self.focused = i;
                            if p.y < self.panels[i].bounds.min_y() + self.panels[i].toolbar_height {
                                self.toolbar_pressed = Some(i);
                                if self.panels[i].toolbar_event(&content_event)? {
                                    self.toolbar_pressed = None;
                                    self.panels[i].toolbar_event(&ContentEvent::Cancel)?;
                                    let begun = w.try_begin_titlebar_drag();
                                    eprintln!(
                                        "TITLEBAR_HANDOFF panel={i} phase=down started={begun} original_arm_cancelled=true"
                                    );
                                }
                            } else if self
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
                                let b = self.panels[i].bounds;
                                let toolbar_height = self.panels[i].toolbar_height;
                                self.panels[i].request(
                                    InputKind::PointerDown,
                                    content_point(b, toolbar_height, p).x,
                                    content_point(b, toolbar_height, p).y,
                                )?;
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
                        for i in 0..2 {
                            if self.visibility_bars[i].pressed {
                                self.visibility_bars[i].pressed = false;
                                if let Some(action) = self.visibility_bars[i].event(&content_event)
                                {
                                    if action == 0x5603 {
                                        self.swap_center_trailing()?;
                                    } else {
                                        self.toggle_visibility(i)?;
                                    }
                                }
                            }
                        }
                        if let Some(nav) = &mut self.navigation
                            && nav.pressed
                        {
                            nav.pressed = false;
                            nav.event(&content_event)?;
                        }
                        if let Some(i) = self.toolbar_pressed.take() {
                            self.panels[i].toolbar_event(&content_event)?;
                        }
                        if let Some(i) = self.pressed.take() {
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
                            let second_x = self.inner.layout(whole, Axis::Horizontal).1.min_x();
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
                            if let Some(i) = self.visibility_bars.iter().position(|b| b.pressed) {
                                self.visibility_bars[i].event(&content_event);
                            } else if let Some(nav) = &mut self.navigation
                                && nav.pressed
                            {
                                if nav.event(&content_event)? {
                                    nav.pressed = false;
                                    nav.event(&ContentEvent::Cancel)?;
                                    w.try_begin_titlebar_drag();
                                }
                            } else if let Some(i) = self.toolbar_pressed {
                                if self.panels[i].toolbar_event(&content_event)? {
                                    // Propagation is not capture transfer. Explicitly
                                    // cancel the old arm before native handoff.
                                    self.toolbar_pressed = None;
                                    self.panels[i].toolbar_event(&ContentEvent::Cancel)?;
                                    let begun = w.try_begin_titlebar_drag();
                                    eprintln!(
                                        "TITLEBAR_HANDOFF panel={i} phase=drag started={begun} original_arm_cancelled=true"
                                    );
                                }
                            } else if let Some(i) = self.pressed {
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
                        for bar in &mut self.visibility_bars {
                            bar.event(&content_event);
                        }
                        if let Some(nav) = &mut self.navigation {
                            nav.event(&content_event)?;
                        }
                        for panel in &mut self.panels {
                            panel.toolbar_event(&content_event)?;
                        }
                    }
                    _ => {}
                }
                w.request_redraw();
            }
            Event::Key(k) if w.is_active() => {
                if let Some(nav) = &mut self.navigation
                    && nav.is_open()
                {
                    nav.event(&ContentEvent::KeyInput(toolbar_key(k)))?;
                } else if let Some(i) = self.panels.iter().position(|p| {
                    p.toolbar_host.get(ComponentId(1)).is_some_and(|c| {
                        c.component::<ToolbarControl>().is_some_and(|t| t.is_open())
                    })
                }) {
                    let event = ContentEvent::KeyInput(toolbar_key(k));
                    self.panels[i].toolbar_event(&event)?;
                } else if k.kind == KeyEventKind::Down
                    && !k.is_repeat
                    && k.characters == "r"
                    && self.panels.len() == 3
                {
                    self.swap_center_trailing()?;
                    eprintln!("PANEL_ORDER {:?}", self.order);
                } else if !self.panels.is_empty() {
                    if k.key_code == 53 {
                        self.toolbar_pressed = None;
                        self.panels[self.focused].toolbar_event(&ContentEvent::Cancel)?;
                    } else {
                        let event = ContentEvent::KeyInput(toolbar_key(k));
                        self.panels[self.focused].toolbar_event(&event)?;
                    }
                }
                w.request_redraw();
            }
            Event::Focus(true) => {
                self.outer_view.set_window_active(true, &mut self.outer);
                self.inner_view.set_window_active(true, &mut self.inner);
                w.request_redraw();
            }
            Event::Focus(false) => {
                for bar in &mut self.visibility_bars {
                    bar.cancel();
                }
                if let Some(nav) = &mut self.navigation {
                    nav.pressed = false;
                    nav.event(&ContentEvent::Cancel)?;
                }
                if let Some(i) = self.pressed.take() {
                    self.panels[i].request(InputKind::Cancel, 0., 0.)?;
                }
                self.toolbar_pressed = None;
                for panel in &mut self.panels {
                    panel.toolbar_event(&ContentEvent::Cancel)?;
                }
                self.outer_view.set_window_active(false, &mut self.outer);
                self.inner_view.set_window_active(false, &mut self.inner);
                w.set_cursor_regions(Vec::new());
                self.drag_split = None;
            }
            Event::CloseRequested => w.close(),
            _ => {}
        }
        Ok(())
    }
}
impl WindowHandler for Host {
    fn handle(&mut self, event: Event, w: &mut Window) {
        if let Err(e) = self.event(event, w) {
            eprintln!("HOST_ERROR {e:#}");
            w.set_title(&format!("AppHost error: {e}"));
        }
    }
}
pub fn run(
    module_ids: Vec<String>,
    acceptance: bool,
    navigation: Option<NavigationOptions>,
) -> Result<()> {
    ensure!(
        module_ids.len() == 1 || module_ids.len() == 3,
        "expected one or three preselected modules"
    );
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
        MenuBar::default(),
        Host::new(module_ids, acceptance, navigation),
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
