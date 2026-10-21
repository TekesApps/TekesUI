//! Headless service lifecycle. All pipe IO and process reaping run off the caller thread.
use crate::{component::*, pipes};
use anyhow::{Result, bail, ensure};
use serde_json::Value;
#[cfg(target_os = "macos")]
use std::collections::HashMap;
use std::{
    collections::{HashSet, VecDeque},
    io::{BufReader, Write},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
const CAPACITY: usize = 64;
static INSTANCE: AtomicU64 = AtomicU64::new(1);

pub trait ProcessModule {
    #[cfg(target_os = "macos")]
    fn view(&mut self) -> Option<&mut dyn crate::PanelModule> {
        None
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        ViewCapabilities::default()
    }
    fn started(&mut self, _: &ComponentEmitter) -> Result<()> {
        Ok(())
    }
    fn handle(&mut self, request: RequestBody, emitter: &ComponentEmitter) -> Result<Value>;
}
#[derive(Clone, Copy)]
pub struct ComponentDefinition {
    pub id: &'static str,
    pub capabilities: Capabilities,
    pub create: fn() -> Box<dyn ProcessModule>,
}
struct Sink {
    writer: Box<dyn Write + Send>,
    serial: u64,
    toolbar: Option<(u64, u64, Vec<ToolbarAction>)>,
    view_revision: u64,
}
#[derive(Clone)]
pub struct ComponentEmitter {
    session: Session,
    capabilities: Capabilities,
    view: Option<NativeViewOptions>,
    window: Option<u64>,
    sink: Arc<Mutex<Sink>>,
}
impl ComponentEmitter {
    pub(crate) fn output(&self, output: Output) -> Result<()> {
        let mut sink = self.sink.lock().unwrap();
        sink.serial = sink
            .serial
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("serial exhausted"))?;
        let packet = ServerMessage {
            version: VERSION,
            session: self.session.clone(),
            serial: sink.serial,
            output,
        };
        pipes::write_packet(&mut sink.writer, &serde_json::to_vec(&packet)?)
    }
    /// Actual logical window/View IDs negotiated by this attachment. Headless
    /// modules receive None; querying this does not allocate native/GPU objects.
    /// Logical host window binding, including service-only attachments.
    pub fn window_identity(&self) -> Option<u64> {
        self.window
    }
    pub fn view_identity(&self) -> Option<(u64, u64)> {
        self.view.as_ref().map(|view| (view.window, view.view))
    }
    /// Notify the attached View using its actual identity. A headless consumer
    /// remains headless and returns false without manufacturing an event.
    pub fn invalidate_view(&self, revision: u64) -> Result<bool> {
        let Some((_, view)) = self.view_identity() else {
            return Ok(false);
        };
        self.emit(ComponentEvent::ViewInvalidated { view, revision })?;
        Ok(true)
    }
    pub fn emit(&self, event: ComponentEvent) -> Result<()> {
        validate_event(&event, self.capabilities)?;
        if let ComponentEvent::ViewInvalidated { view, .. } = &event {
            ensure!(
                self.view
                    .as_ref()
                    .is_some_and(|attached| attached.view == *view),
                "invalidation does not belong to the attached View"
            );
        }
        // Validate and write under one lock so concurrent emitters cannot reorder registrations.
        let mut sink = self.sink.lock().unwrap();
        if let ComponentEvent::ViewInvalidated { revision, .. } = &event {
            ensure!(*revision > sink.view_revision, "stale View invalidation");
            sink.view_revision = *revision;
        }
        if let ComponentEvent::Toolbar {
            revision,
            scope,
            items,
        } = &event
        {
            ensure!(
                sink.toolbar
                    .as_ref()
                    .is_none_or(|(old, _, _)| revision > old),
                "stale toolbar registration"
            );
            let window = toolbar_window(scope, self.window, self.view.as_ref())?;
            sink.toolbar = Some((*revision, window, items.clone()));
        }
        sink.serial = sink
            .serial
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("serial exhausted"))?;
        let packet = ServerMessage {
            version: VERSION,
            session: self.session.clone(),
            serial: sink.serial,
            output: Output::Event(event),
        };
        pipes::write_packet(&mut sink.writer, &serde_json::to_vec(&packet)?)
    }
    pub fn session(&self) -> &Session {
        &self.session
    }
}
fn validate_event(event: &ComponentEvent, caps: Capabilities) -> Result<()> {
    match event {
        ComponentEvent::ViewInvalidated { view, revision } => ensure!(
            caps.view && *view != 0 && *revision != 0,
            "invalid/unnegotiated View invalidation"
        ),
        ComponentEvent::Message { topic, .. } => ensure!(
            caps.services && !topic.is_empty(),
            "service not negotiated or empty topic"
        ),
        ComponentEvent::Toolbar {
            revision,
            scope,
            items,
        } => {
            ensure!(
                caps.toolbar && *revision > 0,
                "toolbar not negotiated or invalid generation"
            );
            ensure!(
                match scope {
                    ToolbarScope::Window { window } => *window != 0,
                    ToolbarScope::View => caps.view,
                },
                "invalid or unnegotiated toolbar scope"
            );
            let mut ids = HashSet::new();
            for item in items {
                ensure!(ids.insert(item.command.id), "duplicate toolbar command");
                ensure!(
                    !item.command.geometry_dependent,
                    "headless command cannot require View geometry"
                );
                ensure!(
                    !item.command.title.is_empty()
                        || item.accessible_name.as_ref().is_some_and(|n| !n.is_empty()),
                    "toolbar action needs accessible text"
                );
            }
        }
    }
    Ok(())
}
// The same scope validation governs producer and receiver. A service-only
// window binding is data, not a native Window/View allocation. Legacy unbound
// Start remains compatible; hosts requiring isolation use spawn_in_window.
fn toolbar_window(
    scope: &ToolbarScope,
    window: Option<u64>,
    view: Option<&NativeViewOptions>,
) -> Result<u64> {
    match scope {
        ToolbarScope::Window { window: target } => {
            ensure!(
                *target != 0 && window.is_none_or(|bound| bound == *target),
                "toolbar belongs to another window"
            );
            Ok(*target)
        }
        ToolbarScope::View => {
            let view =
                view.ok_or_else(|| anyhow::anyhow!("View toolbar requires an attached View"))?;
            ensure!(
                view.required.independent_toolbar,
                "View toolbar capability not negotiated"
            );
            Ok(view.window)
        }
    }
}
/// Call before constructing any UI/GPU object. Returns false for the ordinary host.
pub fn run_component_worker_from_args(
    definitions: &[ComponentDefinition],
    args: &[String],
) -> Result<bool> {
    if args.get(1).map(String::as_str) != Some("--tekesui-component") {
        return Ok(false);
    }
    let cli_window = match args.len() {
        3 => None,
        5 if args[3] == "--window" => {
            let window = args[4].parse::<u64>()?;
            ensure!(window != 0, "invalid bound window");
            Some(window)
        }
        _ => bail!("expected one component identifier and optional --window ID"),
    };
    let found: Vec<_> = definitions.iter().filter(|d| d.id == args[2]).collect();
    ensure!(found.len() == 1, "unknown or ambiguous component");
    let definition = found[0];
    let mut input = BufReader::new(std::io::stdin());
    let bytes =
        pipes::read_packet(&mut input)?.ok_or_else(|| anyhow::anyhow!("missing handshake"))?;
    let (version, session, required, attach) = match serde_json::from_slice(&bytes)? {
        ClientMessage::Start {
            version,
            session,
            required,
        } => (version, session, required, None),
        ClientMessage::StartView {
            version,
            session,
            required,
            attach,
        } => (version, session, required, Some(attach)),
        _ => bail!("Start required"),
    };
    ensure!(
        version == VERSION
            && session.module == definition.id
            && session.instance != 0
            && session.host_epoch != 0,
        "invalid handshake identity/version"
    );
    ensure!(
        required.view == attach.is_some() && definition.capabilities.supports(required),
        "unsupported capabilities"
    );
    let attached_window = attach.as_ref().map(|view| view.window);
    ensure!(
        cli_window.zip(attached_window).is_none_or(|(a, b)| a == b),
        "conflicting window binding"
    );
    let window = attached_window.or(cli_window);
    let emitter = ComponentEmitter {
        session: session.clone(),
        capabilities: required,
        view: attach.as_ref().map(|view| NativeViewOptions {
            window: view.window,
            view: view.view,
            required: view.required,
        }),
        window,
        sink: Arc::new(Mutex::new(Sink {
            writer: Box::new(std::io::stdout()),
            serial: 0,
            toolbar: None,
            view_revision: 0,
        })),
    };
    if let Some(attach) = attach {
        #[cfg(target_os = "macos")]
        return crate::process_view::run((definition.create)(), emitter, input, attach, required);
        #[cfg(not(target_os = "macos"))]
        bail!("native View transport requires macOS");
    }
    emitter.output(Output::Ready {
        pid: std::process::id(),
        capabilities: required,
    })?;
    let mut module = (definition.create)();
    module.started(&emitter)?;
    let mut last = 0;
    while let Some(bytes) = pipes::read_packet(&mut input)? {
        let ClientMessage::Request {
            version,
            session: incoming,
            id,
            body,
        } = serde_json::from_slice(&bytes)?
        else {
            bail!("duplicate Start");
        };
        ensure!(
            version == VERSION && incoming == session && id > last,
            "stale request identity"
        );
        last = id;
        if matches!(body, RequestBody::Stop) {
            emitter.output(Output::Response {
                request: id,
                result: Ok(Value::Null),
            })?;
            return Ok(true);
        }
        ensure!(
            !matches!(body, RequestBody::Render { .. }),
            "View not negotiated"
        );
        ensure!(
            required.services || matches!(body, RequestBody::Invoke { .. }),
            "services not negotiated"
        );
        ensure!(
            required.toolbar || !matches!(body, RequestBody::Invoke { .. }),
            "toolbar not negotiated"
        );
        let valid = valid_invoke(&body, &emitter);
        let result = if valid {
            module.handle(body, &emitter).map_err(|e| e.to_string())
        } else {
            Err("stale or disabled toolbar action".into())
        };
        emitter.output(Output::Response {
            request: id,
            result,
        })?;
    }
    Ok(true)
}
pub(crate) fn valid_invoke(body: &RequestBody, emitter: &ComponentEmitter) -> bool {
    match body {
        RequestBody::Invoke {
            command,
            registration,
            window,
        } => emitter.sink.lock().unwrap().toolbar.as_ref().is_some_and(
            |(generation, owner_window, items)| {
                generation == registration
                    && owner_window == window
                    && items
                        .iter()
                        .any(|i| i.command.id == *command && i.command.enabled)
            },
        ),
        _ => true,
    }
}
#[derive(Clone, Debug)]
pub struct ProcessComponentOptions {
    pub module: String,
    pub host_epoch: u64,
    pub required: Capabilities,
    pub view: Option<NativeViewOptions>,
}
#[derive(Clone, Debug)]
pub struct NativeViewOptions {
    pub window: u64,
    pub view: u64,
    pub required: ViewCapabilities,
}
impl ProcessComponentOptions {
    pub fn with_view(mut self, window: u64, view: u64, required: ViewCapabilities) -> Self {
        self.required.view = true;
        self.view = Some(NativeViewOptions {
            window,
            view,
            required,
        });
        self
    }
    pub fn new(module: impl Into<String>, host_epoch: u64) -> Self {
        Self {
            module: module.into(),
            host_epoch,
            required: Capabilities::HEADLESS,
            view: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    Starting,
    Ready {
        pid: u32,
        capabilities: Capabilities,
    },
    Closing,
    Closed,
    Failed {
        message: String,
    },
}
struct State {
    status: ProcessStatus,
    abort: bool,
    stopping: bool,
    next: u64,
    outgoing: VecDeque<(u64, RequestBody)>,
    incoming: VecDeque<ServerMessage>,
    pending: HashSet<u64>,
    toolbar: Vec<ToolbarAction>,
    toolbar_revision: u64,
    toolbar_window: u64,
    toolbar_declaration: Option<ServerMessage>,
    view_revision: u64,
    native: Option<ViewCapabilities>,
    stop_id: Option<u64>,
    #[cfg(target_os = "macos")]
    renders: HashMap<u64, (tekes_ui_app_protocol::Request, crate::view::Lease)>,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    child: crate::ProcessControl,
    #[cfg(target_os = "macos")]
    view: std::sync::OnceLock<crate::ViewConnection>,
    wake: Arc<dyn Fn() + Send + Sync>,
}
impl Shared {
    fn notify(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.wake)()));
    }
    fn kill(&self) {
        self.child.cancel();
    }
    fn fail(&self, message: String) {
        {
            let mut s = self.state.lock().unwrap();
            if !s.abort {
                s.status = ProcessStatus::Failed { message };
            }
            s.abort = true;
            s.outgoing.clear();
            s.incoming.clear();
            s.pending.clear();
            s.toolbar.clear();
            s.toolbar_declaration = None;
            #[cfg(target_os = "macos")]
            s.renders.clear();
        }
        self.changed.notify_all();
        self.kill();
        #[cfg(target_os = "macos")]
        if let Some(view) = self.view.get() {
            view.close();
        }
        self.notify();
    }
}
/// One public component lifecycle with a service or an explicit legacy View transport.
/// Dropping/closing never waits for pipe IO, GPU completion or child exit.
pub struct ProcessComponent {
    session: Session,
    window: Option<u64>,
    backend: Backend,
}
enum Backend {
    Service(Arc<Shared>),
    #[cfg(target_os = "macos")]
    View(crate::ViewConnection),
}
impl ProcessComponent {
    pub fn spawn(
        options: ProcessComponentOptions,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let window = options.view.as_ref().map(|view| view.window);
        Self::spawn_bound(options, window, wake)
    }
    /// Bind a headless or View component's Window toolbar to this host window.
    /// Existing unbound spawn remains a compatibility entry.
    pub fn spawn_in_window(
        options: ProcessComponentOptions,
        window: u64,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        ensure!(
            window != 0
                && options
                    .view
                    .as_ref()
                    .is_none_or(|view| view.window == window),
            "invalid/mismatched window binding"
        );
        Self::spawn_bound(options, Some(window), wake)
    }
    fn spawn_bound(
        options: ProcessComponentOptions,
        window: Option<u64>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        ensure!(
            !options.module.is_empty()
                && options.host_epoch != 0
                && options.required.view == options.view.is_some()
                && options
                    .view
                    .as_ref()
                    .is_none_or(|v| v.window != 0 && v.view != 0),
            "invalid headless component options"
        );
        #[cfg(not(target_os = "macos"))]
        ensure!(options.view.is_none(), "native View requires macOS");
        let instance = INSTANCE.fetch_add(1, Ordering::Relaxed);
        ensure!(instance != 0, "instance exhausted");
        let session = Session {
            module: options.module,
            instance,
            host_epoch: options.host_epoch,
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                status: ProcessStatus::Starting,
                abort: false,
                stopping: false,
                next: 1,
                outgoing: VecDeque::new(),
                incoming: VecDeque::new(),
                pending: HashSet::new(),
                toolbar: Vec::new(),
                toolbar_revision: 0,
                toolbar_window: 0,
                toolbar_declaration: None,
                view_revision: 0,
                native: None,
                stop_id: None,
                #[cfg(target_os = "macos")]
                renders: HashMap::new(),
            }),
            changed: Condvar::new(),
            child: crate::ProcessControl::new(),
            #[cfg(target_os = "macos")]
            view: std::sync::OnceLock::new(),
            wake: Arc::new(wake),
        });
        #[cfg(target_os = "macos")]
        if options.view.is_some() {
            let weak = Arc::downgrade(&shared);
            let work = Arc::new(move || {
                if let Some(shared) = weak.upgrade() {
                    shared.changed.notify_all();
                }
            });
            shared
                .view
                .set(crate::ViewConnection::remote(
                    shared.child.clone(),
                    shared.wake.clone(),
                    work,
                ))
                .ok()
                .expect("new View slot");
        }
        let worker = shared.clone();
        let identity = session.clone();
        std::thread::Builder::new()
            .name("component-launch-read".into())
            .spawn(move || {
                let result = connection(&worker, &identity, options.required, options.view, window);
                if let Err(error) = result {
                    worker.fail(error.to_string());
                }
                // Take ownership before waiting: no caller-thread lock can cover wait().
                worker.child.reap();
                {
                    let mut state = worker.state.lock().unwrap();
                    if !matches!(state.status, ProcessStatus::Failed { .. }) {
                        state.status = ProcessStatus::Closed;
                    }
                    state.abort = true;
                    state.toolbar.clear();
                    state.toolbar_declaration = None;
                    state.pending.clear();
                    #[cfg(target_os = "macos")]
                    state.renders.clear();
                }
                #[cfg(target_os = "macos")]
                if let Some(view) = worker.view.get() {
                    view.close();
                    let status = worker.state.lock().unwrap().status.clone();
                    view.remote_status(status);
                }
                worker.changed.notify_all();
                worker.notify();
            })?;
        Ok(Self {
            session,
            window,
            backend: Backend::Service(shared),
        })
    }
    #[cfg(target_os = "macos")]
    /// Adapt a fixed V1 PanelModule. Does not negotiate V2 services or claim
    /// independent toolbar, keyboard, scrolling, text/IME or remote AX support.
    pub fn spawn_v1_view(
        module: impl Into<String>,
        host_epoch: u64,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let module = module.into();
        ensure!(
            !module.is_empty() && host_epoch != 0,
            "invalid View component identity"
        );
        let instance = INSTANCE.fetch_add(1, Ordering::Relaxed);
        ensure!(instance != 0, "instance exhausted");
        let session = Session {
            module: module.clone(),
            instance,
            host_epoch,
        };
        Ok(Self {
            session,
            window: None,
            backend: Backend::View(crate::ViewConnection::spawn(module, wake)?),
        })
    }
    fn service(&self) -> Result<&Arc<Shared>> {
        match &self.backend {
            Backend::Service(shared) => Ok(shared),
            #[cfg(target_os = "macos")]
            Backend::View(_) => bail!("V2 services were not negotiated for the V1 View adapter"),
        }
    }
    #[cfg(target_os = "macos")]
    fn view(&self) -> Result<&crate::ViewConnection> {
        match &self.backend {
            Backend::View(view) => Ok(view),
            Backend::Service(shared) => shared
                .view
                .get()
                .ok_or_else(|| anyhow::anyhow!("component has no View")),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn enqueue_view(&self, request: tekes_ui_app_protocol::Request) -> Result<()> {
        self.view()?.enqueue(request)
    }
    #[cfg(target_os = "macos")]
    pub fn set_view_visible(&self, visible: bool) -> Result<Option<u64>> {
        self.view()?.cancel_input(Some(visible))
    }
    #[cfg(target_os = "macos")]
    pub fn cancel_view_input(&self) -> Result<Option<u64>> {
        self.view()?.cancel_input(None)
    }
    #[cfg(target_os = "macos")]
    pub fn drain_view_frames(&self) -> Vec<crate::ViewFrame> {
        match self.view() {
            Ok(view) => view.drain_frames(),
            Err(_) => Vec::new(),
        }
    }
    /// Take the module's coalesced repaint notification without manufacturing
    /// a render request on the I/O thread.
    #[cfg(target_os = "macos")]
    pub fn take_view_invalidated(&self) -> bool {
        self.view().is_ok_and(|view| view.take_invalidated())
    }
    #[cfg(target_os = "macos")]
    pub fn outstanding_view_frames(&self) -> usize {
        self.view().map_or(0, |v| v.outstanding_frames())
    }
    pub fn view_capabilities(&self) -> Option<ViewCapabilities> {
        match &self.backend {
            Backend::Service(shared) => shared.state.lock().unwrap().native,
            #[cfg(target_os = "macos")]
            Backend::View(_) => Some(ViewCapabilities::V1),
        }
    }
    pub fn require_view_capabilities(&self, required: ViewCapabilities) -> Result<()> {
        ensure!(
            self.view_capabilities()
                .is_some_and(|actual| actual.supports(required)),
            "required native View capabilities are unavailable"
        );
        Ok(())
    }
    /// Logical immutable window binding. None identifies an unbound legacy
    /// profile, which is not an isolated Window toolbar contribution source.
    pub fn window_identity(&self) -> Option<u64> {
        self.window
    }
    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn status(&self) -> ProcessStatus {
        match &self.backend {
            Backend::Service(shared) => shared.state.lock().unwrap().status.clone(),
            #[cfg(target_os = "macos")]
            Backend::View(view) => view.status(),
        }
    }
    /// Latest accepted owner/session/scope/generation snapshot. Retain this
    /// envelope with displayed actions; use request_from for delayed UI input.
    pub fn toolbar_registration(&self) -> Option<ServerMessage> {
        let state = self.service().ok()?.state.lock().unwrap();
        if !matches!(state.status, ProcessStatus::Ready { .. }) {
            return None;
        }
        state.toolbar_declaration.clone()
    }
    pub fn drain_events(&self) -> Vec<ServerMessage> {
        let Ok(shared) = self.service() else {
            return Vec::new();
        };
        shared.state.lock().unwrap().incoming.drain(..).collect()
    }
    pub fn request(&self, body: RequestBody) -> Result<u64> {
        self.request_from(&self.session, body)
    }
    /// Keep the event-time owner when callbacks outlive a mount or a process.
    /// Latest-session convenience calls must not retarget a retained UI action.
    pub fn request_from(&self, origin: &Session, body: RequestBody) -> Result<u64> {
        ensure!(*origin == self.session, "stale component action owner");
        ensure!(
            !matches!(body, RequestBody::Render { .. }),
            "use the bounded enqueue_view lane"
        );
        let shared = self.service()?;
        let mut state = shared.state.lock().unwrap();
        ensure!(!state.abort && !state.stopping, "component closing");
        ensure!(
            state.pending.len() < CAPACITY,
            "component request queue full"
        );
        if let RequestBody::Invoke {
            command,
            registration,
            window,
        } = &body
        {
            ensure!(
                *registration == state.toolbar_revision && *window == state.toolbar_window,
                "stale toolbar action"
            );
            ensure!(
                state
                    .toolbar
                    .iter()
                    .any(|i| i.command.id == *command && i.command.enabled),
                "unregistered or disabled command"
            );
        }
        if matches!(body, RequestBody::Stop) {
            state.stopping = true;
            state.status = ProcessStatus::Closing;
        }
        let id = state.next;
        state.next = id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("request exhausted"))?;
        if matches!(body, RequestBody::Stop) {
            state.stop_id = Some(id);
        }
        state.pending.insert(id);
        state.outgoing.push_back((id, body));
        drop(state);
        shared.changed.notify_all();
        Ok(id)
    }
    pub fn invoke(&self, command: u64) -> Result<u64> {
        let (registration, window) = {
            let state = self.service()?.state.lock().unwrap();
            (state.toolbar_revision, state.toolbar_window)
        };
        self.request(RequestBody::Invoke {
            command,
            registration,
            window,
        })
    }
    pub fn close(&self) {
        #[cfg(target_os = "macos")]
        if let Backend::View(view) = &self.backend {
            view.close();
            return;
        }
        let Ok(shared) = self.service() else { return };
        {
            let mut state = shared.state.lock().unwrap();
            if matches!(
                state.status,
                ProcessStatus::Closed | ProcessStatus::Failed { .. }
            ) {
                return;
            }
            state.abort = true;
            state.status = ProcessStatus::Closing;
            state.outgoing.clear();
            state.incoming.clear();
            state.pending.clear();
            state.toolbar.clear();
            state.toolbar_declaration = None;
        }
        shared.changed.notify_all();
        shared.kill();
        #[cfg(target_os = "macos")]
        if let Some(view) = shared.view.get() {
            view.close();
        }
    }
}
impl Drop for ProcessComponent {
    fn drop(&mut self) {
        self.close();
    }
}
fn connection(
    shared: &Arc<Shared>,
    session: &Session,
    required: Capabilities,
    native: Option<NativeViewOptions>,
    window: Option<u64>,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    let receiver = if native.is_some() {
        Some(crate::transport::Receiver::new()?)
    } else {
        None
    };
    let start = if let Some(native) = native.clone() {
        #[cfg(target_os = "macos")]
        {
            ClientMessage::StartView {
                version: VERSION,
                session: session.clone(),
                required,
                attach: ViewAttach {
                    window: native.window,
                    view: native.view,
                    endpoint: receiver.as_ref().unwrap().name.clone(),
                    required: native.required,
                },
            }
        }
        #[cfg(not(target_os = "macos"))]
        bail!("View transport requires macOS")
    } else {
        ClientMessage::Start {
            version: VERSION,
            session: session.clone(),
            required,
        }
    };
    let pipes::Pipes {
        child,
        mut input,
        mut output,
    } = {
        let binding = window.map(|window| window.to_string());
        let mut arguments = vec!["--tekesui-component", session.module.as_str()];
        if native.is_none()
            && let Some(binding) = &binding
        {
            arguments.extend(["--window", binding.as_str()]);
        }
        pipes::spawn(&arguments)?
    };
    let pid = child.id();
    shared.child.install(child)?;
    if shared.state.lock().unwrap().abort {
        shared.kill();
        return Ok(());
    }
    let writer_shared = shared.clone();
    let identity = session.clone();
    let writer_native = native.clone();
    std::thread::Builder::new()
        .name("component-write".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                pipes::write_packet(&mut input, &serde_json::to_vec(&start)?)?;
                loop {
                    let (id, body) = {
                        let mut state = writer_shared.state.lock().unwrap();
                        loop {
                            if state.abort {
                                return Ok(());
                            }
                            if !matches!(state.status, ProcessStatus::Starting) {
                                if let Some(request) = state.outgoing.pop_front() {
                                    break request;
                                }
                                #[cfg(target_os = "macos")]
                                if !state.stopping
                                    && state.pending.len() < CAPACITY
                                    && let Some(native) = &writer_native
                                    && let Some((request, lease)) =
                                        writer_shared.view.get().unwrap().try_start()
                                {
                                    let id = state.next;
                                    state.next = id
                                        .checked_add(1)
                                        .ok_or_else(|| anyhow::anyhow!("request exhausted"))?;
                                    state.pending.insert(id);
                                    state.renders.insert(id, (request.clone(), lease));
                                    break (
                                        id,
                                        RequestBody::Render {
                                            view: native.view,
                                            input: request,
                                        },
                                    );
                                }
                            }
                            state = writer_shared.changed.wait(state).unwrap();
                        }
                    };
                    let packet = ClientMessage::Request {
                        version: VERSION,
                        session: identity.clone(),
                        id,
                        body,
                    };
                    pipes::write_packet(&mut input, &serde_json::to_vec(&packet)?)?;
                }
            })();
            if let Err(error) = result {
                writer_shared.fail(error.to_string());
            }
        })?;
    let mut serial = 0;
    let mut ready = false;
    let mut stopped = false;
    while let Some(bytes) = pipes::read_packet(&mut output)? {
        let packet: ServerMessage = serde_json::from_slice(&bytes)?;
        ensure!(
            packet.version == VERSION && packet.session == *session && packet.serial > serial,
            "stale server identity/serial"
        );
        serial = packet.serial;
        #[cfg(target_os = "macos")]
        if let Output::Frame {
            request,
            view,
            frame,
        } = &packet.output
        {
            let receiver = receiver
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unnegotiated frame"))?;
            // Mach IO never holds the public status/request mutex.
            let right = receiver.receive_owned_cancellable(|| shared.child.is_cancelled())?;
            let mut state = shared.state.lock().unwrap();
            if state.abort || stopped {
                drop(state);
                drop(right);
                continue;
            }
            ensure!(
                ready && native.as_ref().is_some_and(|n| n.view == *view),
                "wrong frame View"
            );
            let (expected, lease) = state
                .renders
                .remove(request)
                .ok_or_else(|| anyhow::anyhow!("unknown render response"))?;
            ensure!(state.pending.remove(request), "duplicate render response");
            ensure!(
                frame.pid == pid
                    && frame.version == tekes_ui_app_protocol::VERSION
                    && frame.seq == expected.seq
                    && frame.viewport == expected.viewport
                    && frame.width == expected.width
                    && frame.height == expected.height
                    && frame.pixel_width == (expected.width as f32 * expected.scale).round() as u32
                    && frame.pixel_height
                        == (expected.height as f32 * expected.scale).round() as u32,
                "wrong render identity/geometry"
            );
            drop(state);
            shared
                .view
                .get()
                .unwrap()
                .complete(crate::ReceivedFrame::received(frame.clone(), right), lease);
            shared.changed.notify_all();
            continue;
        }
        let mut state = shared.state.lock().unwrap();
        if state.abort {
            return Ok(());
        }
        let mut invalidated = false;
        match &packet.output {
            Output::Ready {
                pid: actual,
                capabilities,
            } => {
                ensure!(
                    !ready && native.is_none() && *actual == pid && *capabilities == required,
                    "invalid Ready"
                );
                ready = true;
                if !state.stopping {
                    state.status = ProcessStatus::Ready {
                        pid,
                        capabilities: *capabilities,
                    };
                }
            }
            Output::ReadyView {
                pid: actual,
                capabilities,
                window,
                view,
                native: negotiated,
            } => {
                let expected = native
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("unnegotiated View Ready"))?;
                ensure!(
                    !ready
                        && *actual == pid
                        && *capabilities == required
                        && *window == expected.window
                        && *view == expected.view
                        && *negotiated == expected.required,
                    "invalid View Ready"
                );
                ready = true;
                state.native = Some(*negotiated);
                if !state.stopping {
                    state.status = ProcessStatus::Ready {
                        pid,
                        capabilities: *capabilities,
                    };
                }
            }
            other => {
                ensure!(ready, "output before Ready");
                match other {
                    Output::Event(event) => {
                        validate_event(event, required)?;
                        match event {
                            ComponentEvent::Toolbar {
                                revision,
                                items,
                                scope,
                            } => {
                                ensure!(
                                    *revision > state.toolbar_revision,
                                    "stale toolbar generation"
                                );
                                state.toolbar_window =
                                    toolbar_window(scope, window, native.as_ref())?;
                                state.toolbar_declaration = Some(packet.clone());
                                state.toolbar_revision = *revision;
                                state.toolbar = items.clone();
                            }
                            ComponentEvent::ViewInvalidated { view, revision } => {
                                ensure!(
                                    native.as_ref().is_some_and(|n| n.view == *view),
                                    "wrong invalidation View"
                                );
                                ensure!(*revision > state.view_revision, "stale View invalidation");
                                state.view_revision = *revision;
                                invalidated = true;
                            }
                            _ => (),
                        }
                    }
                    Output::Response { request, result } => {
                        ensure!(state.pending.remove(request), "unknown response");
                        #[cfg(target_os = "macos")]
                        ensure!(
                            !state.renders.contains_key(request),
                            "render returned a service response"
                        );
                        if state.stop_id == Some(*request) {
                            ensure!(result.is_ok(), "Stop rejected");
                            stopped = true;
                        }
                    }
                    Output::Fatal { message } => bail!("worker fatal: {message}"),
                    _ => bail!("unnegotiated output"),
                }
            }
        }
        // View invalidation is a coalesced redraw signal, not business output.
        // It must not fill the service-event queue while the View is hidden.
        if !invalidated {
            ensure!(
                state.incoming.len() < CAPACITY,
                "component output queue full"
            );
            state.incoming.push_back(packet);
        }
        #[cfg(target_os = "macos")]
        let status = state.status.clone();
        drop(state);
        #[cfg(target_os = "macos")]
        if let Some(view) = shared.view.get() {
            if invalidated {
                view.invalidate();
            } else {
                view.remote_status(status);
            }
        }
        shared.changed.notify_all();
        if !invalidated {
            shared.notify();
        }
    }
    ensure!(
        shared.state.lock().unwrap().abort || stopped,
        "worker exited without Stop acknowledgement"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action(id: u64) -> ToolbarAction {
        ToolbarAction {
            command: tekes_ui_app_protocol::ToolbarItem {
                id,
                title: String::new(),
                enabled: true,
                geometry_dependent: false,
            },
            icon: Some("back".into()),
            accessible_name: Some("Back".into()),
            tooltip: None,
        }
    }
    fn toolbar(revision: u64, window: u64, items: Vec<ToolbarAction>) -> ComponentEvent {
        ComponentEvent::Toolbar {
            revision,
            scope: ToolbarScope::Window { window },
            items,
        }
    }
    #[test]
    fn headless_toolbar_requires_real_scope_unique_identity_and_accessible_text() {
        assert!(validate_event(&toolbar(1, 4, vec![action(7)]), Capabilities::HEADLESS).is_ok());
        assert!(validate_event(&toolbar(1, 0, vec![action(7)]), Capabilities::HEADLESS).is_err());
        assert!(validate_event(&toolbar(0, 4, vec![]), Capabilities::HEADLESS).is_err());
        assert!(
            validate_event(
                &toolbar(1, 4, vec![action(7), action(7)]),
                Capabilities::HEADLESS
            )
            .is_err()
        );
        let mut icon_only = action(7);
        icon_only.accessible_name = None;
        assert!(validate_event(&toolbar(1, 4, vec![icon_only]), Capabilities::HEADLESS).is_err());
        let mut view = action(7);
        view.command.geometry_dependent = true;
        assert!(validate_event(&toolbar(1, 4, vec![view]), Capabilities::HEADLESS).is_err());
        assert!(
            validate_event(
                &ComponentEvent::Toolbar {
                    revision: 1,
                    scope: ToolbarScope::View,
                    items: vec![]
                },
                Capabilities::HEADLESS
            )
            .is_err()
        );
        assert!(validate_event(&toolbar(1, 4, vec![]), Capabilities::default()).is_err());
    }
    #[test]
    fn concurrent_emitter_registry_cannot_regress_generation() {
        let emitter = ComponentEmitter {
            session: Session {
                module: "navigation".into(),
                instance: 1,
                host_epoch: 1,
            },
            capabilities: Capabilities::HEADLESS,
            view: None,
            window: None,
            sink: Arc::new(Mutex::new(Sink {
                writer: Box::new(Vec::<u8>::new()),
                serial: 0,
                toolbar: None,
                view_revision: 0,
            })),
        };
        emitter.emit(toolbar(2, 4, vec![action(7)])).unwrap();
        assert!(emitter.clone().emit(toolbar(1, 4, vec![])).is_err());
        assert!(emitter.emit(toolbar(2, 4, vec![])).is_err());
        emitter.emit(toolbar(3, 4, vec![])).unwrap();
        assert_eq!(emitter.sink.lock().unwrap().serial, 2);
    }
    #[test]
    fn view_context_rejects_foreign_or_stale_invalidation_and_headless_stays_empty() {
        let mut emitter = ComponentEmitter {
            session: Session {
                module: "view".into(),
                instance: 3,
                host_epoch: 8,
            },
            capabilities: Capabilities {
                view: true,
                ..Capabilities::HEADLESS
            },
            view: Some(NativeViewOptions {
                window: 21,
                view: 34,
                required: ViewCapabilities {
                    independent_toolbar: true,
                    ..Default::default()
                },
            }),
            window: Some(21),
            sink: Arc::new(Mutex::new(Sink {
                writer: Box::new(Vec::<u8>::new()),
                serial: 0,
                toolbar: None,
                view_revision: 0,
            })),
        };
        assert_eq!(emitter.view_identity(), Some((21, 34)));
        assert!(
            emitter
                .emit(ComponentEvent::ViewInvalidated {
                    view: 2,
                    revision: 1
                })
                .is_err()
        );
        assert!(emitter.invalidate_view(0).is_err());
        assert!(emitter.invalidate_view(1).unwrap());
        assert!(emitter.clone().invalidate_view(1).is_err());
        assert!(emitter.invalidate_view(2).unwrap());
        assert_eq!(emitter.sink.lock().unwrap().serial, 2);
        emitter.view = None;
        emitter.capabilities = Capabilities::HEADLESS;
        assert!(!emitter.invalidate_view(3).unwrap());
        assert_eq!(emitter.view_identity(), None);
        assert_eq!(emitter.sink.lock().unwrap().serial, 2);
    }
    #[test]
    fn toolbar_scope_binding_preserves_prior_registration_after_foreign_clear() {
        let emitter = ComponentEmitter {
            session: Session {
                module: "scope".into(),
                instance: 3,
                host_epoch: 8,
            },
            capabilities: Capabilities {
                view: true,
                ..Capabilities::HEADLESS
            },
            window: Some(21),
            view: Some(NativeViewOptions {
                window: 21,
                view: 34,
                required: ViewCapabilities {
                    independent_toolbar: true,
                    ..Default::default()
                },
            }),
            sink: Arc::new(Mutex::new(Sink {
                writer: Box::new(Vec::<u8>::new()),
                serial: 0,
                toolbar: None,
                view_revision: 0,
            })),
        };
        emitter.emit(toolbar(1, 21, vec![action(7)])).unwrap();
        assert!(emitter.emit(toolbar(2, 22, vec![])).is_err());
        assert_eq!(emitter.sink.lock().unwrap().serial, 1);
        assert!(valid_invoke(
            &RequestBody::Invoke {
                command: 7,
                registration: 1,
                window: 21
            },
            &emitter
        ));
        emitter
            .emit(ComponentEvent::Toolbar {
                revision: 2,
                scope: ToolbarScope::View,
                items: vec![action(7)],
            })
            .unwrap();
        assert!(!valid_invoke(
            &RequestBody::Invoke {
                command: 7,
                registration: 1,
                window: 21
            },
            &emitter
        ));
        assert!(valid_invoke(
            &RequestBody::Invoke {
                command: 7,
                registration: 2,
                window: 21
            },
            &emitter
        ));
        assert!(!valid_invoke(
            &RequestBody::Invoke {
                command: 7,
                registration: 2,
                window: 22
            },
            &emitter
        ));
        let missing_cap = NativeViewOptions {
            window: 21,
            view: 34,
            required: ViewCapabilities::default(),
        };
        assert!(toolbar_window(&ToolbarScope::View, Some(21), Some(&missing_cap)).is_err());
        assert!(toolbar_window(&ToolbarScope::View, Some(21), None).is_err());
        emitter.emit(toolbar(3, 21, vec![])).unwrap();
        assert!(!valid_invoke(
            &RequestBody::Invoke {
                command: 7,
                registration: 3,
                window: 21
            },
            &emitter
        ));
    }
    #[test]
    fn ordinary_entry_never_invokes_factory_and_invalid_cli_is_rejected() {
        fn forbidden() -> Box<dyn ProcessModule> {
            panic!("must not instantiate");
        }
        let definition = ComponentDefinition {
            id: "navigation",
            capabilities: Capabilities::HEADLESS,
            create: forbidden,
        };
        assert!(!run_component_worker_from_args(&[definition], &["host".into()]).unwrap());
        for arguments in [
            vec!["host", "--tekesui-component"],
            vec!["host", "--tekesui-component", "unknown"],
            vec!["host", "--tekesui-component", "navigation", "extra"],
            vec!["host", "--tekesui-component", "navigation", "--window", "0"],
            vec![
                "host",
                "--tekesui-component",
                "navigation",
                "--window",
                "bad",
            ],
            vec!["host", "--tekesui-component", "navigation", "--fake", "12"],
        ] {
            assert!(
                run_component_worker_from_args(
                    &[definition],
                    &arguments.into_iter().map(String::from).collect::<Vec<_>>()
                )
                .is_err()
            );
        }
        assert!(
            run_component_worker_from_args(
                &[definition, definition],
                &[
                    "host".into(),
                    "--tekesui-component".into(),
                    "navigation".into()
                ]
            )
            .is_err()
        );
    }
}
