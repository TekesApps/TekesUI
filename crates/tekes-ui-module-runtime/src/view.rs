//! V1 View transport adapter. Uses the same child owner as ProcessComponent.
//! This is not remote keyboard/IME/AX support or the final Content adapter.
use crate::{
    ModuleProcess, ProcessControl, ProcessStatus, ReceivedFrame, component::Capabilities,
    import_frame,
};
use anyhow::{Result, ensure};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tekes_ui_app_protocol::{Frame, InputKind, Request, VERSION};
use tekes_ui_render::gpu::{GpuContext, GpuImage};
const INPUT_CAPACITY: usize = 256;
const FRAME_CAPACITY: usize = 2;
struct State {
    queue: VecDeque<Request>,
    frames: VecDeque<ViewFrame>,
    status: ProcessStatus,
    closed: bool,
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
}
impl Core {
    fn notify(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.wake)()));
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
}
impl ViewFrame {
    pub fn metadata(&self) -> &Frame {
        &self.frame
    }
    pub fn import(self, context: &GpuContext) -> Result<ViewImage> {
        let Self { mut frame, lease } = self;
        let owner = lease
            .0
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("View frame owner no longer exists"))?;
        ensure!(
            !owner.control.is_cancelled(),
            "View frame belongs to a closed instance"
        );
        let texture = import_frame(context, &mut frame)?;
        let image = context.import_image(texture)?;
        Ok(ViewImage {
            metadata: frame.into_metadata(),
            image: Some(image),
            context: context.clone(),
            lease: Some(lease),
        })
    }
}
/// Keep this guard while the image can be included in future submissions.
/// Unregister its Renderer image before dropping/replacing it. Drop registers
/// a fence after all submissions already made; no caller-thread GPU wait.
pub struct ViewImage {
    metadata: Frame,
    image: Option<GpuImage>,
    context: GpuContext,
    lease: Option<Lease>,
}
impl ViewImage {
    pub fn metadata(&self) -> &Frame {
        &self.metadata
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
            _lease: lease,
            context: self.context.clone(),
            done,
        });
    }
}
struct Retired {
    _image: GpuImage,
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
pub(crate) struct ViewConnection {
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
    pub(crate) fn complete(&self, frame: ReceivedFrame, lease: Lease) {
        let mut state = self.core.state.lock().unwrap();
        if state.closed || state.suspended {
            drop(state);
            drop(frame);
            drop(lease);
            return;
        }
        state.frames.push_back(ViewFrame { frame, lease });
        drop(state);
        self.core.notify();
    }
    pub(crate) fn remote_status(&self, status: ProcessStatus) {
        self.core.state.lock().unwrap().status = status;
        self.core.notify();
    }
    pub(crate) fn invalidate(&self) {
        self.core.state.lock().unwrap().invalidated = true;
        self.core.notify();
    }
    pub(crate) fn take_invalidated(&self) -> bool {
        std::mem::take(&mut self.core.state.lock().unwrap().invalidated)
    }
    pub fn spawn(
        module: impl Into<String>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let module = module.into();
        ensure!(!module.is_empty(), "empty module identity");
        let core = Self::core(ProcessControl::new(), Arc::new(wake), None);
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
fn view_loop(core: &Arc<Core>, module: &str) -> Result<()> {
    let mut process = ModuleProcess::spawn_with_control(module, core.control.clone())?;
    {
        let mut state = core.state.lock().unwrap();
        if state.closed {
            return Ok(());
        }
        state.status = ProcessStatus::Ready {
            pid: process.pid(),
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
            while !state.closed && (state.queue.is_empty() || state.outstanding == FRAME_CAPACITY) {
                state = core
                    .changed
                    .wait_timeout(state, Duration::from_millis(50))
                    .unwrap()
                    .0;
                if !state.closed {
                    ensure!(
                        !core.control.exited()?,
                        "View worker exited while idle or backpressured"
                    );
                }
            }
            if state.closed {
                return Ok(());
            }
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
        state.frames.push_back(ViewFrame { frame, lease });
        drop(state);
        core.notify();
    }
}
