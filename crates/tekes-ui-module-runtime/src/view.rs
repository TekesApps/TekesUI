//! V1 View transport adapter. Uses the same child owner as ProcessComponent.
//! This is not remote keyboard/IME/AX support or the final Content adapter.
use crate::{
    ModuleProcess, ProcessControl, ProcessStatus, ReceivedFrame, component::Capabilities,
    parent::import_frame_with_guard,
};
use anyhow::{Result, ensure};
use std::{
    collections::VecDeque,
    os::fd::{AsRawFd, OwnedFd},
    sync::{
        Arc, Condvar, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tekes_ui_app_protocol::{Frame, InputKind, Request, VERSION};
use tekes_ui_render::{
    gpu::{GpuContext, GpuImage},
    shared_surface_macos::SharedSurfaceUseGuard,
};
const INPUT_CAPACITY: usize = 256;
const FRAME_CAPACITY: usize = 2;

mod kqueue_sys {
    use std::os::fd::{FromRawFd, OwnedFd, RawFd};

    pub(super) const EVFILT_PROC: i16 = -5;
    pub(super) const EVFILT_USER: i16 = -10;
    pub(super) const EV_ADD: u16 = 0x0001;
    pub(super) const EV_ONESHOT: u16 = 0x0010;
    pub(super) const EV_CLEAR: u16 = 0x0020;
    pub(super) const EV_ERROR: u16 = 0x4000;
    pub(super) const NOTE_EXIT: u32 = 0x8000_0000;
    pub(super) const NOTE_TRIGGER: u32 = 0x0100_0000;
    pub(super) const ESRCH: i32 = 3;
    pub(super) const EINTR: i32 = 4;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub(super) struct Kevent64 {
        pub ident: u64,
        pub filter: i16,
        pub flags: u16,
        pub fflags: u32,
        pub data: i64,
        pub udata: u64,
        pub ext: [u64; 2],
    }

    #[repr(C)]
    pub(super) struct Timespec {
        pub tv_sec: isize,
        pub tv_nsec: isize,
    }

    unsafe extern "C" {
        pub(super) fn kqueue() -> RawFd;
        pub(super) fn kevent64(
            kq: RawFd,
            changelist: *const Kevent64,
            nchanges: i32,
            eventlist: *mut Kevent64,
            nevents: i32,
            flags: u32,
            timeout: *const Timespec,
        ) -> i32;
    }

    pub(super) fn create() -> std::io::Result<OwnedFd> {
        // SAFETY: `kqueue()` allocates a fresh kernel event queue descriptor with no pointers.
        let fd = unsafe { kqueue() };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `fd` is a valid owned kqueue descriptor just returned by `kqueue()`.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

fn trigger_user_kq(kq: &OwnedFd) {
    let wake = kqueue_sys::Kevent64 {
        ident: 1,
        filter: kqueue_sys::EVFILT_USER,
        flags: 0,
        fflags: kqueue_sys::NOTE_TRIGGER,
        ..Default::default()
    };
    // SAFETY: `kq` is a live kqueue fd and `wake` points to one initialized stack `Kevent64`.
    let _ = unsafe {
        kqueue_sys::kevent64(
            kq.as_raw_fd(),
            &wake,
            1,
            std::ptr::null_mut(),
            0,
            0,
            std::ptr::null(),
        )
    };
}

struct State {
    queue: VecDeque<Request>,
    frames: VecDeque<ViewFrame>,
    status: ProcessStatus,
    closed: bool,
    worker_exited: bool,
    outstanding: usize,
    last_seq: Option<u64>,
    identity: Option<u32>,
    viewport: u64,
    latest: Option<Request>,
    suspended: bool,
    invalidated: bool,
}
struct Core {
    state: Mutex<State>,
    changed: Condvar,
    control: ProcessControl,
    wake: Arc<dyn Fn() + Send + Sync>,
    work: Option<Arc<dyn Fn() + Send + Sync>>,
    retirement: Mutex<Option<mpsc::SyncSender<Retired>>>,
    exit_kq: Mutex<Option<Arc<OwnedFd>>>,
}
impl Core {
    fn notify(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.wake)()));
    }
    fn mark_worker_exited(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.worker_exited = true;
        }
        self.changed.notify_all();
    }
    fn terminate(&self, failure: Option<String>) {
        let frames = {
            let mut state = self.state.lock().unwrap();
            if state.closed {
                return;
            }
            state.closed = true;
            state.queue.clear();
            state.status = match failure {
                Some(message) => ProcessStatus::Failed { message },
                None => ProcessStatus::Closing,
            };
            std::mem::take(&mut state.frames)
        };
        // Leases lock state on drop; never drop pending frames under that lock.
        drop(frames);
        self.control.cancel();
        self.retirement.lock().unwrap().take();
        if let Some(kq) = self.exit_kq.lock().unwrap().take() {
            trigger_user_kq(&kq);
        }
        self.changed.notify_all();
        self.notify();
    }
    fn retire(self: &Arc<Self>, retired: Retired) {
        if self.control.is_cancelled() {
            return;
        }
        let mut sender = self.retirement.lock().unwrap();
        if sender.is_none() {
            let (tx, rx) = mpsc::sync_channel(FRAME_CAPACITY);
            let weak = Arc::downgrade(self);
            match std::thread::Builder::new()
                .name("view-gpu-retirement".into())
                .spawn(move || retire_loop(weak, rx))
            {
                Ok(_) => *sender = Some(tx),
                Err(error) => {
                    drop(sender);
                    self.terminate(Some(error.to_string()));
                    return;
                }
            }
        }
        let result = sender.as_ref().unwrap().try_send(retired);
        drop(sender);
        if result.is_err() {
            self.terminate(Some("GPU retirement queue unavailable".into()));
        }
    }
}
pub(crate) struct Lease(Weak<Core>);
impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(core) = self.0.upgrade() {
            {
                let mut state = core.state.lock().unwrap();
                state.outstanding -= 1;
            }
            core.changed.notify_all();
            if let Some(work) = &core.work {
                work();
            }
        }
    }
}
/// A received immutable frame consumes one of two slots until discarded or its
/// imported image is retired after actual consumer GPU completion.
pub struct ViewFrame {
    frame: ReceivedFrame,
    lease: Lease,
    curve: Option<crate::component::curve::FrameStamp>,
    layer: Option<crate::layer_raster::LayerRasterLayout>,
}
impl ViewFrame {
    pub fn metadata(&self) -> &Frame {
        &self.frame
    }
    pub fn curve_stamp(&self) -> Option<&crate::component::curve::FrameStamp> {
        self.curve.as_ref()
    }
    pub(crate) fn layer_layout(&self) -> Option<&crate::layer_raster::LayerRasterLayout> {
        self.layer.as_ref()
    }
    pub fn import(self, context: &GpuContext) -> Result<ViewImage> {
        let Self {
            mut frame,
            lease,
            curve,
            layer,
        } = self;
        let owner = lease
            .0
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("View frame owner no longer exists"))?;
        ensure!(
            !owner.control.is_cancelled(),
            "View frame belongs to a closed instance"
        );
        let (texture, guard) = import_frame_with_guard(context, &mut frame)?;
        let image = context.import_image(texture)?;
        Ok(ViewImage {
            metadata: frame.into_metadata(),
            image: Some(image),
            surface_guard: Some(guard),
            context: context.clone(),
            lease: Some(lease),
            curve,
            layer,
        })
    }
}
/// Keep this guard while the image can be included in future submissions.
/// Unregister its Renderer image before dropping/replacing it. Drop registers
/// a fence after all submissions already made; no caller-thread GPU wait.
pub struct ViewImage {
    metadata: Frame,
    image: Option<GpuImage>,
    surface_guard: Option<SharedSurfaceUseGuard>,
    context: GpuContext,
    lease: Option<Lease>,
    curve: Option<crate::component::curve::FrameStamp>,
    layer: Option<crate::layer_raster::LayerRasterLayout>,
}
impl ViewImage {
    pub fn metadata(&self) -> &Frame {
        &self.metadata
    }
    pub fn curve_stamp(&self) -> Option<&crate::component::curve::FrameStamp> {
        self.curve.as_ref()
    }
    pub(crate) fn layer_layout(&self) -> Option<&crate::layer_raster::LayerRasterLayout> {
        self.layer.as_ref()
    }
    pub fn image(&self) -> &GpuImage {
        self.image.as_ref().unwrap()
    }
}
impl Drop for ViewImage {
    fn drop(&mut self) {
        let lease = self.lease.take().unwrap();
        let Some(core) = lease.0.upgrade() else {
            return;
        };
        let done = Arc::new(AtomicBool::new(false));
        let callback = done.clone();
        self.context
            .queue()
            .on_submitted_work_done(move || callback.store(true, Ordering::Release));
        core.retire(Retired {
            _image: self.image.take().unwrap(),
            // Keep `_surface_guard` strictly after `_image` so Metal textures drop
            // before decrementing the kernel IOSurface use count.
            _surface_guard: self.surface_guard.take(),
            _lease: lease,
            context: self.context.clone(),
            done,
        });
    }
}
struct Retired {
    _image: GpuImage,
    _surface_guard: Option<SharedSurfaceUseGuard>,
    _lease: Lease,
    context: GpuContext,
    done: Arc<AtomicBool>,
}
fn retire_loop(core: Weak<Core>, rx: mpsc::Receiver<Retired>) {
    let mut pending: Vec<Retired> = Vec::new();
    loop {
        let Some(shared) = core.upgrade() else {
            return;
        };
        if shared.control.is_cancelled() {
            return;
        }
        // Poll never waits for a hung GPU. Cancellation remains observable.
        let received = if pending.is_empty() {
            // No polling/timer while idle. Close drops the sender and wakes recv.
            rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        } else {
            rx.recv_timeout(Duration::from_millis(5))
        };
        match received {
            Ok(frame) => pending.push(frame),
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => (),
        }
        while let Ok(frame) = rx.try_recv() {
            pending.push(frame);
        }
        for frame in &pending {
            if let Err(error) = frame.context.device().poll(wgpu::PollType::Poll) {
                shared.terminate(Some(format!("GPU completion: {error}")));
                return;
            }
        }
        pending.retain(|frame| !frame.done.load(Ordering::Acquire));
    }
}
/// Nonblocking queued V1 View connection. Protocol capabilities are limited to
/// Layout, primary pointer, Cancel and toolbar Command. Services and toolbar
/// changes independent of a rendered frame still require the V2 View adapter.
pub struct ViewConnection {
    core: Arc<Core>,
}
impl ViewConnection {
    fn core(
        control: ProcessControl,
        wake: Arc<dyn Fn() + Send + Sync>,
        work: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Arc<Core> {
        Arc::new(Core {
            state: Mutex::new(State {
                queue: VecDeque::new(),
                frames: VecDeque::new(),
                status: ProcessStatus::Starting,
                closed: false,
                worker_exited: false,
                outstanding: 0,
                last_seq: None,
                identity: None,
                viewport: 0,
                latest: None,
                suspended: false,
                invalidated: false,
            }),
            changed: Condvar::new(),
            control,
            wake,
            work,
            retirement: Mutex::new(None),
            exit_kq: Mutex::new(None),
        })
    }
    pub(crate) fn remote(
        control: ProcessControl,
        wake: Arc<dyn Fn() + Send + Sync>,
        work: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            core: Self::core(control, wake, Some(work)),
        }
    }
    pub(crate) fn try_start(&self) -> Option<(Request, Lease)> {
        let mut state = self.core.state.lock().unwrap();
        if state.closed || state.outstanding >= FRAME_CAPACITY {
            return None;
        }
        let request = state.queue.pop_front()?;
        state.outstanding += 1;
        Some((request, Lease(Arc::downgrade(&self.core))))
    }
    pub(crate) fn complete(
        &self,
        frame: ReceivedFrame,
        lease: Lease,
        curve: Option<crate::component::curve::FrameStamp>,
        layer: Option<crate::layer_raster::LayerRasterLayout>,
    ) {
        let mut state = self.core.state.lock().unwrap();
        if state.closed || state.suspended {
            drop(state);
            drop(frame);
            drop(lease);
            return;
        }
        state.frames.push_back(ViewFrame {
            frame,
            lease,
            curve,
            layer,
        });
        drop(state);
        self.core.notify();
    }
    pub(crate) fn remote_status(&self, status: ProcessStatus) {
        let mut state = self.core.state.lock().unwrap();
        let changed = state.status != status;
        state.status = status;
        drop(state);
        if changed {
            self.core.notify();
        }
    }
    pub(crate) fn invalidate(&self) {
        let mut state = self.core.state.lock().unwrap();
        if state.closed || state.invalidated {
            return;
        }
        state.invalidated = true;
        drop(state);
        self.core.notify();
    }
    pub(crate) fn take_invalidated(&self) -> bool {
        std::mem::take(&mut self.core.state.lock().unwrap().invalidated)
    }
    pub(crate) fn has_pending_invalidation(&self) -> bool {
        self.core.state.lock().unwrap().invalidated
    }
    pub fn spawn(
        module: impl Into<String>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        Self::spawn_with_control(module, ProcessControl::new(), wake)
    }
    pub fn spawn_with_control(
        module: impl Into<String>,
        control: ProcessControl,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let module = module.into();
        ensure!(!module.is_empty(), "empty module identity");
        let core = Self::core(control, Arc::new(wake), None);
        let work = core.clone();
        std::thread::Builder::new()
            .name("view-launch-request".into())
            .spawn(move || {
                let result = view_loop(&work, &module);
                // view_loop's ModuleProcess drops/reaps before publishing terminal state.
                if let Err(error) = result {
                    work.terminate(Some(format!("{error:#}")));
                }
                {
                    let mut state = work.state.lock().unwrap();
                    if !matches!(state.status, ProcessStatus::Failed { .. }) {
                        state.status = ProcessStatus::Closed;
                    }
                    state.closed = true;
                }
                work.changed.notify_all();
                work.notify();
            })?;
        Ok(Self { core })
    }
    #[doc(hidden)]
    pub fn new_inert() -> Self {
        Self {
            core: Self::core(ProcessControl::new(), Arc::new(|| {}), None),
        }
    }
    #[doc(hidden)]
    pub fn queued_requests(&self) -> Vec<Request> {
        self.core
            .state
            .lock()
            .unwrap()
            .queue
            .iter()
            .cloned()
            .collect()
    }
    pub fn control(&self) -> &ProcessControl {
        &self.core.control
    }
    pub fn status(&self) -> ProcessStatus {
        self.core.state.lock().unwrap().status.clone()
    }
    pub fn enqueue(&self, request: Request) -> Result<()> {
        ensure!(
            request.version == VERSION
                && request.x.is_finite()
                && request.y.is_finite()
                && request.scale.is_finite()
                && (0.5..=4.).contains(&request.scale)
                && (1..=4096).contains(&request.width)
                && (1..=4096).contains(&request.height)
                && (request.width as f32 * request.scale).round() <= 8192.
                && (request.height as f32 * request.scale).round() <= 8192.,
            "invalid View request"
        );
        let mut state = self.core.state.lock().unwrap();
        ensure!(!state.closed, "View connection closed");
        ensure!(
            !state.suspended || request.kind == InputKind::Cancel,
            "View is hidden"
        );
        ensure!(
            state.last_seq.is_none_or(|seq| request.seq > seq)
                && state.identity.is_none_or(|id| id == request.identity)
                && request.viewport >= state.viewport,
            "stale View request identity"
        );
        let coalesce = request.kind == InputKind::Layout
            && state
                .queue
                .back()
                .is_some_and(|r| r.kind == InputKind::Layout);
        ensure!(
            coalesce || state.queue.len() < INPUT_CAPACITY,
            "View input queue full"
        );
        state.last_seq = Some(request.seq);
        state.identity = Some(request.identity);
        state.viewport = request.viewport;
        state.latest = Some(request.clone());
        if coalesce {
            *state.queue.back_mut().unwrap() = request;
        } else {
            state.queue.push_back(request);
        }
        drop(state);
        self.core.changed.notify_all();
        if let Some(work) = &self.core.work {
            work();
        }
        Ok(())
    }
    pub fn cancel_input(&self, visible: Option<bool>) -> Result<Option<u64>> {
        let (frames, sequence) = {
            let mut state = self.core.state.lock().unwrap();
            ensure!(!state.closed, "View connection closed");
            if let Some(visible) = visible {
                state.suspended = !visible;
            }
            if visible == Some(true) {
                return Ok(None);
            }
            state.queue.clear();
            let sequence = if let Some(mut request) = state.latest.clone() {
                request.seq = request
                    .seq
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("request exhausted"))?;
                request.kind = InputKind::Cancel;
                request.command = None;
                request.x = 0.;
                request.y = 0.;
                state.last_seq = Some(request.seq);
                state.latest = Some(request.clone());
                let seq = request.seq;
                state.queue.push_back(request);
                Some(seq)
            } else {
                None
            };
            (std::mem::take(&mut state.frames), sequence)
        };
        drop(frames);
        self.core.changed.notify_all();
        if let Some(work) = &self.core.work {
            work();
        }
        self.core.notify();
        Ok(sequence)
    }
    pub fn drain_frames(&self) -> Vec<ViewFrame> {
        self.core.state.lock().unwrap().frames.drain(..).collect()
    }
    pub fn outstanding_frames(&self) -> usize {
        self.core.state.lock().unwrap().outstanding
    }
    pub fn close(&self) {
        self.core.terminate(None);
    }
}
impl Drop for ViewConnection {
    fn drop(&mut self) {
        self.close();
    }
}
struct ExitWatch {
    kq: Arc<OwnedFd>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ExitWatch {
    fn start(core: &Arc<Core>, pid: u32) -> Result<Self> {
        let kq = Arc::new(kqueue_sys::create()?);
        let user_add = kqueue_sys::Kevent64 {
            ident: 1,
            filter: kqueue_sys::EVFILT_USER,
            flags: kqueue_sys::EV_ADD | kqueue_sys::EV_CLEAR,
            ..Default::default()
        };
        // SAFETY: `kq` is valid and `user_add` is an initialized stack `Kevent64`.
        let rc = unsafe {
            kqueue_sys::kevent64(
                kq.as_raw_fd(),
                &user_add,
                1,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null(),
            )
        };
        ensure!(
            rc >= 0,
            "register kqueue wake failed: {}",
            std::io::Error::last_os_error()
        );
        let proc_add = kqueue_sys::Kevent64 {
            ident: u64::from(pid),
            filter: kqueue_sys::EVFILT_PROC,
            flags: kqueue_sys::EV_ADD | kqueue_sys::EV_ONESHOT,
            fflags: kqueue_sys::NOTE_EXIT,
            ..Default::default()
        };
        // SAFETY: `kq` is valid and `proc_add` registers `EVFILT_PROC` for child `pid`.
        let proc_rc = unsafe {
            kqueue_sys::kevent64(
                kq.as_raw_fd(),
                &proc_add,
                1,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null(),
            )
        };
        if proc_rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(kqueue_sys::ESRCH) {
                core.mark_worker_exited();
                return Ok(Self { kq, handle: None });
            }
            anyhow::bail!("register kqueue proc exit failed: {err}");
        }
        *core.exit_kq.lock().unwrap() = Some(Arc::clone(&kq));
        if core.state.lock().unwrap().closed {
            core.exit_kq.lock().unwrap().take();
            return Ok(Self { kq, handle: None });
        }
        let watch_core = Arc::downgrade(core);
        let watch_kq = Arc::clone(&kq);
        let handle = std::thread::Builder::new()
            .name("view-exit-watch".into())
            .spawn(move || {
                let mut event = kqueue_sys::Kevent64::default();
                loop {
                    // SAFETY: blocks on valid `watch_kq` writing at most 1 event to stack `event`.
                    let rc = unsafe {
                        kqueue_sys::kevent64(
                            watch_kq.as_raw_fd(),
                            std::ptr::null(),
                            0,
                            &mut event,
                            1,
                            0,
                            std::ptr::null(),
                        )
                    };
                    if rc < 0 {
                        let err = std::io::Error::last_os_error();
                        if err.raw_os_error() == Some(kqueue_sys::EINTR) {
                            continue;
                        }
                        break;
                    }
                    if rc == 0 {
                        continue;
                    }
                    if event.filter == kqueue_sys::EVFILT_PROC
                        || (event.flags & kqueue_sys::EV_ERROR != 0)
                    {
                        if let Some(core) = watch_core.upgrade() {
                            core.mark_worker_exited();
                        }
                        break;
                    }
                    if event.filter == kqueue_sys::EVFILT_USER {
                        break;
                    }
                }
            })?;
        Ok(Self {
            kq,
            handle: Some(handle),
        })
    }
}

impl Drop for ExitWatch {
    fn drop(&mut self) {
        trigger_user_kq(&self.kq);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn view_loop(core: &Arc<Core>, module: &str) -> Result<()> {
    let mut process = ModuleProcess::spawn_with_control(module, core.control.clone())?;
    let pid = process.pid();
    let _exit_watch = ExitWatch::start(core, pid)?;
    {
        let mut state = core.state.lock().unwrap();
        if state.closed {
            return Ok(());
        }
        state.status = ProcessStatus::Ready {
            pid,
            capabilities: Capabilities {
                services: false,
                toolbar: true,
                view: true,
            },
        };
    }
    core.notify();
    loop {
        let request = {
            let mut state = core.state.lock().unwrap();
            while !state.closed
                && !state.worker_exited
                && (state.queue.is_empty() || state.outstanding == FRAME_CAPACITY)
            {
                state = core.changed.wait(state).unwrap();
            }
            if state.closed {
                return Ok(());
            }
            ensure!(
                !state.worker_exited && !core.control.exited()?,
                "View worker exited while idle or backpressured"
            );
            let request = state.queue.pop_front().unwrap();
            state.outstanding += 1;
            request
        };
        let lease = Lease(Arc::downgrade(core));
        let frame = process.request(&request)?;
        let mut state = core.state.lock().unwrap();
        if state.closed || state.suspended {
            let closed = state.closed;
            drop(state);
            drop(frame);
            drop(lease);
            if closed {
                return Ok(());
            } else {
                continue;
            }
        }
        state.frames.push_back(ViewFrame {
            frame,
            lease,
            curve: None,
            layer: None,
        });
        drop(state);
        core.notify();
    }
}
