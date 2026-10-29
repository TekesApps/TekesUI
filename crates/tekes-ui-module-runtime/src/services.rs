//! Headless service lifecycle. All pipe IO and process reaping run off the caller thread.
use crate::{component::*, pipes};
use anyhow::{Result, bail, ensure};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::{BufReader, Write},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
const CAPACITY: usize = 64;
static INSTANCE: AtomicU64 = AtomicU64::new(1);

pub trait ProcessModule {
    /// Apply window environment to the SAME original font backend and View.
    /// Unsupported modules fail explicitly; no silently ignored zoom/theme.
    fn set_view_context(&mut self, _: &ViewContext) -> Result<()> {
        bail!("View context is not supported by this module")
    }
    /// Original child tree and its existing text backend. Returning this opts
    /// into the typed Content callback lane; the module retains both owners.
    /// Render/layout still use this same tree through view(), never a replica.
    #[cfg(target_os = "macos")]
    fn content(
        &mut self,
    ) -> Option<(
        &mut tekes_ui_controls::composition::ContentHost,
        &mut dyn tekes_ui_controls::ButtonTextRenderer,
    )> {
        None
    }
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
    /// Allocate the next revision from the original event sink and emit one
    /// ordinary ViewInvalidated event. Used when a stamped Content input
    /// changes the child Host geometry after the input has already completed.
    pub(crate) fn invalidate_view_next(&self) -> Result<u64> {
        let (_, view) = self
            .view_identity()
            .ok_or_else(|| anyhow::anyhow!("View invalidation requires an attached View"))?;
        let mut sink = self.sink.lock().unwrap();
        let revision = sink
            .view_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("View invalidation revision exhausted"))?;
        let event = ComponentEvent::ViewInvalidated { view, revision };
        validate_event(&event, self.capabilities)?;
        ensure!(
            self.view
                .as_ref()
                .is_some_and(|attached| attached.view == view),
            "invalidation does not belong to the attached View"
        );
        sink.view_revision = revision;
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
        pipes::write_packet(&mut sink.writer, &serde_json::to_vec(&packet)?)?;
        Ok(revision)
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
        if let RequestBody::CurveV1(envelope) = &body {
            emitter.output(crate::component::curve::rejection_output(
                id, envelope, None,
            ))?;
            continue;
        }
        if matches!(body, RequestBody::SetViewContext { .. }) {
            emitter.output(Output::Response {
                request: id,
                result: Err("View context requires a negotiated View".into()),
            })?;
            continue;
        }
        ensure!(
            !matches!(body, RequestBody::Render { .. }),
            "View not negotiated"
        );
        ensure!(
            !matches!(body, RequestBody::Content(_)),
            "Content input requires a negotiated View"
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
#[derive(Clone)]
struct CurvePending {
    layer_contract: bool,
    attached_view: u64,
    transaction_id: u64,
    offer: Option<curve::CapabilityWire>,
    proposal: Option<curve::GrantProposal>,
    revoke_serial: Option<u64>,
    query: Option<curve::QueryStamp>,
    content_input: bool,
    discard_reply: bool,
}
fn validate_server_identity(
    packet: &ServerMessage,
    session: &Session,
    previous_serial: u64,
) -> Result<()> {
    ensure!(
        packet.version == VERSION && packet.session == *session && packet.serial > previous_serial,
        "stale server identity/serial"
    );
    Ok(())
}
fn consume_generic_response_pending(
    pending: &mut HashSet<u64>,
    curve_pending: &HashMap<u64, CurvePending>,
    request: u64,
) -> Result<()> {
    ensure!(
        !curve_pending.contains_key(&request),
        "Curve request returned an untyped response"
    );
    ensure!(pending.remove(&request), "unknown response");
    Ok(())
}
fn consume_curve_rejection(
    pending: &mut HashSet<u64>,
    curve_pending: &mut HashMap<u64, CurvePending>,
    request: u64,
    view: u64,
    reply: &crate::component::curve::CurveReply,
) -> Result<bool> {
    let expected = curve_pending
        .get(&request)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Curve reply for non-Curve request"))?;
    ensure!(expected.attached_view == view, "Curve reply View mismatch");
    match reply {
        crate::component::curve::CurveReply::Rejected { transaction_id, .. } => {
            ensure!(
                expected.transaction_id == *transaction_id,
                "Curve reply transaction mismatch"
            );
        }
        crate::component::curve::CurveReply::Negotiated { capability }
        | crate::component::curve::CurveReply::LayerNegotiated { capability } => {
            ensure!(
                matches!(reply, curve::CurveReply::LayerNegotiated { .. })
                    == expected.layer_contract,
                "Curve acknowledgement changed the composition contract"
            );
            let offered = expected
                .offer
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unexpected Curve negotiation acknowledgement"))?;
            ensure!(
                capability.protocol_major == offered.protocol_major
                    && capability.wire_revision == offered.wire_revision
                    && capability.clip_geometry_revision == offered.clip_geometry_revision
                    && capability.coverage_aa_revision == offered.coverage_aa_revision
                    && capability.features == offered.features,
                "Curve acknowledgement changed the offered contract"
            );
            let limits = |value: &curve::LimitsWire| curve::Limits {
                packet_bytes_exclusive: value.packet_bytes_exclusive as usize,
                records: value.records as usize,
                depth: value.depth,
                validation_steps: value.validation_steps,
                target_axis_px: value.target_axis_px,
                ttl_ms: value.ttl_ms,
            };
            limits(&capability.limits).validate_ack(limits(&offered.limits))?;
        }
        crate::component::curve::CurveReply::FramePublished { .. }
        | crate::component::curve::CurveReply::LayerFramePublished { .. } => {
            bail!("Curve frame publication must be consumed by the View transport")
        }
        crate::component::curve::CurveReply::GrantPrepared {
            transaction_id,
            proposal_id,
            grant_serial,
            parent_geometry_generation,
            child_geometry_generation,
            viewport,
            allocation_seq,
            layout_seq,
        } => {
            let proposal = expected
                .proposal
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unexpected Curve grant acknowledgement"))?;
            ensure!(
                expected.transaction_id == *transaction_id
                    && proposal.proposal_id == *proposal_id
                    && *grant_serial == proposal.proposal_id
                    && proposal.parent_geometry_generation == *parent_geometry_generation
                    && proposal.viewport == *viewport
                    && proposal.allocation_seq == *allocation_seq
                    && proposal.layout_seq == *layout_seq
                    && *child_geometry_generation != 0,
                "Curve grant acknowledgement changed its geometry lease"
            );
        }
        crate::component::curve::CurveReply::GrantRevoked {
            transaction_id,
            grant_serial,
        } => {
            ensure!(
                expected.transaction_id == *transaction_id
                    && expected.revoke_serial == Some(*grant_serial),
                "Curve revoke acknowledgement changed its grant"
            );
        }
        crate::component::curve::CurveReply::ContentProjection {
            transaction_id,
            stamp,
            reply,
        } => {
            let query = expected
                .query
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unexpected Curve content projection"))?;
            ensure!(
                !expected.content_input
                    && expected.transaction_id == *transaction_id
                    && query == stamp,
                "Curve content projection changed its query identity"
            );
            ensure!(
                reply.view == stamp.view && reply.viewport == stamp.viewport,
                "Curve content projection changed its View allocation"
            );
        }
        crate::component::curve::CurveReply::ContentInput {
            transaction_id,
            stamp,
            outcome,
        } => {
            let query = expected
                .query
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unexpected Curve content input reply"))?;
            ensure!(
                expected.content_input
                    && expected.transaction_id == *transaction_id
                    && query == stamp,
                "Curve content input changed its frame identity"
            );
            match outcome {
                crate::component::curve::ContentInputOutcome::Applied { reply } => ensure!(
                    reply.view == stamp.view && reply.viewport == stamp.viewport,
                    "Curve content input changed its View allocation"
                ),
                crate::component::curve::ContentInputOutcome::Failed { message } => ensure!(
                    !message.trim().is_empty(),
                    "Curve content input failure omitted its diagnostic"
                ),
            }
        }
    }
    ensure!(
        pending.contains(&request),
        "Curve reply has no pending outer request"
    );
    pending.remove(&request);
    curve_pending.remove(&request);
    Ok(expected.discard_reply)
}
struct State {
    status: ProcessStatus,
    abort: bool,
    stopping: bool,
    next: u64,
    outgoing: VecDeque<(u64, RequestBody)>,
    incoming: VecDeque<ServerMessage>,
    pending: HashSet<u64>,
    curve_pending: HashMap<u64, CurvePending>,
    // Existing RPC reply routing: these callers wait on changed, not the UI.
    synchronous_content: HashSet<u64>,
    toolbar: Vec<ToolbarAction>,
    toolbar_revision: u64,
    toolbar_window: u64,
    toolbar_declaration: Option<ServerMessage>,
    view_revision: u64,
    native: Option<ViewCapabilities>,
    expected_view: Option<u64>,
    layer_view: Option<u64>,
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
    #[cfg(target_os = "macos")]
    fn wake_view_writer(&self) {
        // View work has its own queue lock. Serialize its notification with
        // the service writer's predicate-check -> wait transition as well.
        let _state = self.state.lock().unwrap();
        self.changed.notify_all();
    }
    fn notify(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.wake)()));
    }
    fn kill(&self) {
        self.child.cancel();
    }
    fn fail(&self, message: String) {
        #[cfg(target_os = "macos")]
        let renders;
        {
            let mut s = self.state.lock().unwrap();
            if !s.abort {
                s.status = ProcessStatus::Failed { message };
            }
            s.abort = true;
            s.outgoing.clear();
            s.incoming.clear();
            s.pending.clear();
            s.curve_pending.clear();
            s.synchronous_content.clear();
            s.toolbar.clear();
            s.toolbar_declaration = None;
            #[cfg(target_os = "macos")]
            {
                renders = std::mem::take(&mut s.renders);
            }
        }
        // Frame leases wake this writer; never drop them under its state lock.
        #[cfg(target_os = "macos")]
        drop(renders);
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
        let expected_view = options.view.as_ref().map(|view| view.view);
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                status: ProcessStatus::Starting,
                abort: false,
                stopping: false,
                next: 1,
                outgoing: VecDeque::new(),
                incoming: VecDeque::new(),
                pending: HashSet::new(),
                curve_pending: HashMap::new(),
                synchronous_content: HashSet::new(),
                toolbar: Vec::new(),
                toolbar_revision: 0,
                toolbar_window: 0,
                toolbar_declaration: None,
                view_revision: 0,
                native: None,
                expected_view,
                layer_view: None,
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
                    shared.wake_view_writer();
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
                #[cfg(target_os = "macos")]
                let renders;
                {
                    let mut state = worker.state.lock().unwrap();
                    if !matches!(state.status, ProcessStatus::Failed { .. }) {
                        state.status = ProcessStatus::Closed;
                    }
                    state.abort = true;
                    state.toolbar.clear();
                    state.toolbar_declaration = None;
                    state.pending.clear();
                    state.curve_pending.clear();
                    state.synchronous_content.clear();
                    #[cfg(target_os = "macos")]
                    {
                        renders = std::mem::take(&mut state.renders);
                    }
                }
                #[cfg(target_os = "macos")]
                drop(renders);
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
    pub fn has_pending_view_updates(&self) -> bool {
        let incoming = self
            .service()
            .is_ok_and(|shared| !shared.state.lock().unwrap().incoming.is_empty());
        incoming
            || self
                .view()
                .is_ok_and(|view| view.has_pending_invalidation())
    }
    /// Curve frame receipt may proceed only when no host-visible service or
    /// View invalidation is waiting. Exact Curve and generic responses remain
    /// consumable by their owners and do not force a Host relayout by themselves.
    #[cfg(target_os = "macos")]
    pub fn has_pending_curve_host_updates(&self) -> bool {
        let incoming = self.service().is_ok_and(|shared| {
            shared.state.lock().unwrap().incoming.iter().any(|packet| {
                !matches!(
                    packet.output,
                    Output::CurveReplyV1 { .. } | Output::Response { .. }
                )
            })
        });
        incoming
            || self
                .view()
                .is_ok_and(|view| view.has_pending_invalidation())
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
    /// Remove only one already identity-validated Curve response, leaving
    /// unrelated service and toolbar events in their original queue order.
    #[cfg(target_os = "macos")]
    pub fn take_curve_reply(&self, request: u64) -> Option<ServerMessage> {
        let shared = self.service().ok()?;
        let mut state = shared.state.lock().unwrap();
        let index = state.incoming.iter().position(|packet| {
            matches!(packet.output, Output::CurveReplyV1 { request: actual, .. } if actual == request)
        })?;
        state.incoming.remove(index)
    }
    /// Consume only one identity-validated generic response owned by the
    /// caller, leaving Curve and business events in their original queue.
    #[cfg(target_os = "macos")]
    pub fn take_response(&self, request: u64) -> Option<Result<serde_json::Value>> {
        let shared = self.service().ok()?;
        let mut state = shared.state.lock().unwrap();
        let index = state.incoming.iter().position(|packet| {
            matches!(packet.output, Output::Response { request: actual, .. } if actual == request)
        })?;
        let packet = state.incoming.remove(index)?;
        let Output::Response { result, .. } = packet.output else {
            unreachable!("response queue search filters response type")
        };
        Some(result.map_err(anyhow::Error::msg))
    }
    /// Drain ordinary host events while preserving request/reply packets for
    /// their exact pending owners.
    #[cfg(target_os = "macos")]
    pub fn drain_curve_host_events(&self) -> Vec<ServerMessage> {
        let Ok(shared) = self.service() else {
            return Vec::new();
        };
        let mut state = shared.state.lock().unwrap();
        let mut events = Vec::new();
        let count = state.incoming.len();
        for _ in 0..count {
            let Some(packet) = state.incoming.pop_front() else {
                break;
            };
            if matches!(
                packet.output,
                Output::CurveReplyV1 { .. } | Output::Response { .. }
            ) {
                state.incoming.push_back(packet);
            } else {
                events.push(packet);
            }
        }
        events
    }
    pub fn request(&self, body: RequestBody) -> Result<u64> {
        self.request_from(&self.session, body)
    }
    /// Ordered native callback/query round trip. This consumes only its own
    /// response, leaving business events and other replies in the public queue.
    /// Timeout is a failed query, not process termination or permission to use
    /// stale text geometry. The caller must not restart/replay the input.
    #[cfg(target_os = "macos")]
    pub fn content_request(
        &self,
        origin: &Session,
        request: content::Request,
        timeout: std::time::Duration,
    ) -> Result<content::Reply> {
        self.content_request_with_curve(origin, request, None, timeout)
    }
    /// Ordered mutable callback on the explicit Curve lane. The original
    /// Content request type remains source-compatible; its frame stamp lives
    /// only in the Curve envelope.
    #[cfg(target_os = "macos")]
    pub fn curve_content_request(
        &self,
        origin: &Session,
        stamp: curve::QueryStamp,
        request: content::Request,
        timeout: std::time::Duration,
    ) -> Result<content::Reply> {
        ensure!(
            (request.view, request.viewport) == (stamp.view, stamp.viewport),
            "Curve Content input does not match its frame stamp"
        );
        self.content_request_with_curve(origin, request, Some(stamp), timeout)
    }
    /// Read-only semantic or reading query on its own exact-frame lane.
    #[cfg(target_os = "macos")]
    pub fn curve_content_query(
        &self,
        origin: &Session,
        stamp: curve::QueryStamp,
        query: curve::ReadOnlyQueryWire,
        timeout: std::time::Duration,
    ) -> Result<content::Reply> {
        ensure!(
            !timeout.is_zero() && timeout <= std::time::Duration::from_secs(10),
            "invalid Curve Content query timeout"
        );
        let query_stamp = match &query {
            curve::ReadOnlyQueryWire::SemanticTree { stamp }
            | curve::ReadOnlyQueryWire::ReadingState { stamp, .. } => stamp,
        };
        ensure!(
            query_stamp == &stamp,
            "Curve Content query changed its frame stamp"
        );
        let id = self.enqueue_request(
            origin,
            RequestBody::CurveV1(curve::Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: stamp.view,
                transaction_id: 0,
                ttl_ms: 0,
                body: curve::RequestBodyV1::ContentQuery { query },
            }),
            true,
        )?;
        let shared = self.service()?;
        let deadline = std::time::Instant::now() + timeout;
        let mut state = shared.state.lock().unwrap();
        loop {
            ensure!(
                !state.abort,
                "Content process unavailable: {:?}",
                state.status
            );
            if let Some(index) = state.incoming.iter().position(|packet| {
                matches!(packet.output, Output::CurveReplyV1 { request, .. } if request == id)
            }) {
                let packet = state.incoming.remove(index).unwrap();
                let Output::CurveReplyV1 { view, reply, .. } = packet.output else {
                    unreachable!("query reply search filters Curve responses")
                };
                ensure!(view == stamp.view, "mismatched Curve Content query View");
                let reply = match reply {
                    curve::CurveReply::ContentProjection {
                        transaction_id,
                        stamp: actual,
                        reply,
                    } => {
                        ensure!(
                            transaction_id == 0 && actual == stamp,
                            "mismatched Curve Content query identity"
                        );
                        *reply
                    }
                    curve::CurveReply::Rejected { code, .. } => {
                        anyhow::bail!("Curve Content query rejected: {code:?}")
                    }
                    _ => anyhow::bail!("unexpected Curve Content query reply"),
                };
                ensure!(
                    (reply.view, reply.viewport) == (stamp.view, stamp.viewport),
                    "mismatched Curve Content query geometry"
                );
                return Ok(reply);
            }
            let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) else {
                if let Some(pending) = state.curve_pending.get_mut(&id) {
                    pending.discard_reply = true;
                }
                state.synchronous_content.remove(&id);
                anyhow::bail!("Curve Content query timed out");
            };
            let (next, _) = shared.changed.wait_timeout(state, remaining).unwrap();
            state = next;
        }
    }
    #[cfg(target_os = "macos")]
    fn content_request_with_curve(
        &self,
        origin: &Session,
        request: content::Request,
        curve_stamp: Option<curve::QueryStamp>,
        timeout: std::time::Duration,
    ) -> Result<content::Reply> {
        ensure!(
            !timeout.is_zero() && timeout <= std::time::Duration::from_secs(10),
            "invalid Content query timeout"
        );
        let expected = (request.view, request.viewport);
        let expected_curve = curve_stamp.clone();
        let body = if let Some(stamp) = curve_stamp {
            RequestBody::CurveV1(curve::Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: stamp.view,
                transaction_id: 0,
                ttl_ms: 0,
                body: curve::RequestBodyV1::ContentInput {
                    stamp,
                    request: Box::new(request),
                },
            })
        } else {
            RequestBody::Content(request)
        };
        let id = self.enqueue_request(origin, body, true)?;
        let shared = self.service()?;
        let deadline = std::time::Instant::now() + timeout;
        let mut state = shared.state.lock().unwrap();
        loop {
            ensure!(
                !state.abort,
                "Content process unavailable: {:?}",
                state.status
            );
            if let Some(index) = state.incoming.iter().position(|p| match &p.output {
                Output::Response { request, .. } => *request == id,
                Output::CurveReplyV1 { request, .. } => *request == id,
                _ => false,
            }) {
                let packet = state.incoming.remove(index).unwrap();
                let reply = match packet.output {
                    Output::Response { result, .. } if expected_curve.is_none() => {
                        let reply: content::Reply =
                            serde_json::from_value(result.map_err(anyhow::Error::msg)?)?;
                        reply
                    }
                    Output::CurveReplyV1 { view, reply, .. }
                        if let Some(stamp) = &expected_curve =>
                    {
                        ensure!(view == stamp.view, "mismatched Curve Content View");
                        match reply {
                            curve::CurveReply::ContentInput {
                                transaction_id,
                                stamp: actual,
                                outcome,
                            } => {
                                ensure!(
                                    transaction_id == 0 && &actual == stamp,
                                    "mismatched Curve Content input identity"
                                );
                                match outcome {
                                    curve::ContentInputOutcome::Applied { reply } => *reply,
                                    curve::ContentInputOutcome::Failed { message } => {
                                        anyhow::bail!(
                                            "Curve Content input failed after dispatch: {message}"
                                        )
                                    }
                                }
                            }
                            curve::CurveReply::Rejected { code, .. } => {
                                anyhow::bail!("Curve Content input rejected: {code:?}")
                            }
                            _ => anyhow::bail!("unexpected Curve Content input reply"),
                        }
                    }
                    _ => anyhow::bail!("wrong Content reply lane"),
                };
                ensure!(
                    (reply.view, reply.viewport) == expected,
                    "mismatched Content reply geometry"
                );
                drop(state);
                if expected_curve.is_none()
                    && reply.changed
                    && let Some(view) = shared.view.get()
                {
                    view.invalidate();
                }
                return Ok(reply);
            }
            let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) else {
                // A later reply belongs to the ordinary public drain after this
                // caller stops waiting. Never replay its original input.
                if expected_curve.is_some()
                    && let Some(pending) = state.curve_pending.get_mut(&id)
                {
                    pending.discard_reply = true;
                }
                state.synchronous_content.remove(&id);
                anyhow::bail!("Content query timed out; input not replayed");
            };
            let (next, _) = shared.changed.wait_timeout(state, remaining).unwrap();
            state = next;
        }
    }
    /// Keep the event-time owner when callbacks outlive a mount or a process.
    /// Latest-session convenience calls must not retarget a retained UI action.
    pub fn request_from(&self, origin: &Session, body: RequestBody) -> Result<u64> {
        self.enqueue_request(origin, body, false)
    }
    fn enqueue_request(
        &self,
        origin: &Session,
        body: RequestBody,
        synchronous_content: bool,
    ) -> Result<u64> {
        let content_input = match &body {
            RequestBody::Content(request) => Some(&request.input),
            RequestBody::CurveV1(curve::Envelope {
                body: curve::RequestBodyV1::ContentInput { request, .. },
                ..
            }) => Some(&request.input),
            _ => None,
        };
        ensure!(
            !synchronous_content
                || matches!(body, RequestBody::Content(_))
                || matches!(
                    body,
                    RequestBody::CurveV1(curve::Envelope {
                        body: curve::RequestBodyV1::ContentInput { .. }
                            | curve::RequestBodyV1::ContentQuery { .. },
                        ..
                    })
                ),
            "wrong synchronous reply lane"
        );
        ensure!(*origin == self.session, "stale component action owner");
        if let Some(input) = content_input {
            match input {
                content::Input::Pointer(pointer) => ensure!(
                    pointer.position.iter().all(|p| p.is_finite()),
                    "invalid Content pointer position"
                ),
                content::Input::Scroll(scroll) => {
                    ensure!(scroll.is_valid(), "invalid Content scroll input");
                }
                content::Input::RevealReadingItem { options, .. } => {
                    ensure!(options.is_valid(), "invalid Content reveal options");
                }
                content::Input::Accessibility { action, .. } => {
                    ensure!(action.is_valid(), "invalid Content accessibility action");
                }
                _ => (),
            }
        }
        ensure!(
            !matches!(body, RequestBody::Render { .. }),
            "use the bounded enqueue_view lane"
        );
        let shared = self.service()?;
        let mut state = shared.state.lock().unwrap();
        ensure!(
            !state.abort && !state.stopping,
            "component closing: {:?}",
            state.status
        );
        if let RequestBody::CurveV1(curve::Envelope {
            body: curve::RequestBodyV1::ContentQuery { query, .. },
            ..
        }) = &body
        {
            ensure!(
                match query {
                    curve::ReadOnlyQueryWire::SemanticTree { .. } => {
                        state.native.is_some_and(|caps| caps.accessibility)
                    }
                    curve::ReadOnlyQueryWire::ReadingState { .. } => {
                        state.native.is_some_and(|caps| caps.scroll)
                    }
                },
                "Curve Content query capability not negotiated"
            );
        }
        if matches!(body, RequestBody::SetViewContext { .. }) {
            // The configured original View lane exists before ReadyView, so
            // initial context can be queued without racing the handshake.
            #[cfg(target_os = "macos")]
            let configured = shared.view.get().is_some();
            #[cfg(not(target_os = "macos"))]
            let configured = false;
            ensure!(configured, "View context requires a configured V2 View");
        }
        if content_input.is_some() {
            ensure!(
                state.native.is_some_and(|n| n.keyboard
                    || n.text_ime
                    || n.content_pointer
                    || n.scroll
                    || n.accessibility),
                "Content input not negotiated"
            );
        }
        if content_input.is_some_and(|input| {
            matches!(
                input,
                content::Input::Scroll(_)
                    | content::Input::ReadingState { .. }
                    | content::Input::RevealReadingItem { .. }
            )
        }) {
            ensure!(
                state.native.is_some_and(|n| n.scroll),
                "Content scroll capability not negotiated"
            );
        }
        if content_input.is_some_and(|input| {
            matches!(
                input,
                content::Input::Semantics | content::Input::Accessibility { .. }
            )
        }) {
            ensure!(
                state.native.is_some_and(|n| n.accessibility),
                "Content accessibility capability not negotiated"
            );
        }
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
        if let RequestBody::CurveV1(envelope) = &body {
            let attached_view = state.expected_view.unwrap_or(0);
            state.curve_pending.insert(
                id,
                CurvePending {
                    layer_contract: matches!(
                        &envelope.body,
                        curve::RequestBodyV1::NegotiateLayer { .. }
                            | curve::RequestBodyV1::PrepareLayerGrant { .. }
                    ),
                    attached_view,
                    transaction_id: envelope.transaction_id,
                    offer: match &envelope.body {
                        curve::RequestBodyV1::Negotiate { offer }
                        | curve::RequestBodyV1::NegotiateLayer { offer } => Some(offer.clone()),
                        _ => None,
                    },
                    proposal: match &envelope.body {
                        curve::RequestBodyV1::PrepareGrant { proposal } => {
                            Some((**proposal).clone())
                        }
                        curve::RequestBodyV1::PrepareLayerGrant { proposal } => {
                            Some((*proposal.geometry).clone())
                        }
                        _ => None,
                    },
                    revoke_serial: match &envelope.body {
                        curve::RequestBodyV1::RevokeAdmission { grant_serial, .. } => {
                            Some(*grant_serial)
                        }
                        _ => None,
                    },
                    query: match &envelope.body {
                        curve::RequestBodyV1::ContentQuery { query } => Some(match query {
                            curve::ReadOnlyQueryWire::SemanticTree { stamp }
                            | curve::ReadOnlyQueryWire::ReadingState { stamp, .. } => stamp.clone(),
                        }),
                        curve::RequestBodyV1::ContentInput { stamp, .. } => Some(stamp.clone()),
                        _ => None,
                    },
                    content_input: matches!(
                        &envelope.body,
                        curve::RequestBodyV1::ContentInput { .. }
                    ),
                    discard_reply: false,
                },
            );
        }
        if synchronous_content {
            state.synchronous_content.insert(id);
        }
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
            state.curve_pending.clear();
            state.synchronous_content.clear();
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
        validate_server_identity(&packet, session, serial)?;
        serial = packet.serial;
        #[cfg(target_os = "macos")]
        let frame_publication = match &packet.output {
            Output::Frame {
                request,
                view,
                frame,
            } => Some((*request, *view, frame, None, None)),
            Output::CurveReplyV1 {
                request,
                view,
                reply: crate::component::curve::CurveReply::FramePublished { stamp, frame },
            } => Some((*request, *view, frame.as_ref(), Some(stamp), None)),
            Output::CurveReplyV1 {
                request,
                view,
                reply:
                    crate::component::curve::CurveReply::LayerFramePublished {
                        stamp,
                        frame,
                        physical_origin_px,
                    },
            } => Some((
                *request,
                *view,
                frame.as_ref(),
                Some(stamp),
                Some(*physical_origin_px),
            )),
            _ => None,
        };
        #[cfg(target_os = "macos")]
        if let Some((request, view, frame, curve, layer_origin)) = frame_publication {
            let receiver = receiver
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("unnegotiated frame"))?;
            // Mach IO never holds the public status/request mutex.
            let right = receiver.receive_owned_for_process(&shared.child)?;
            let mut state = shared.state.lock().unwrap();
            if state.abort || stopped {
                drop(state);
                drop(right);
                continue;
            }
            ensure!(
                ready && native.as_ref().is_some_and(|n| n.view == view),
                "wrong frame View"
            );
            let (expected, _) = state
                .renders
                .get(&request)
                .ok_or_else(|| anyhow::anyhow!("unknown render response"))?;
            ensure!(
                state.pending.contains(&request),
                "duplicate render response"
            );
            ensure!(
                frame.pid == pid
                    && frame.version == tekes_ui_app_protocol::VERSION
                    && frame.seq == expected.seq
                    && frame.viewport == expected.viewport
                    && frame.width == expected.width
                    && frame.height == expected.height,
                "wrong render identity/geometry"
            );
            let layer = if let Some(origin) = layer_origin {
                ensure!(
                    state.layer_view == Some(view),
                    "independent layer was not negotiated"
                );
                let layout = crate::layer_raster::LayerRasterLayout::new(
                    [expected.width, expected.height],
                    expected.scale,
                    origin,
                )?;
                ensure!(
                    layout.pixel_size() == (frame.pixel_width, frame.pixel_height),
                    "wrong layer raster extent/phase"
                );
                Some(layout)
            } else {
                ensure!(
                    state.layer_view != Some(view),
                    "layer View published an untyped raster"
                );
                ensure!(
                    frame.pixel_width == (expected.width as f32 * expected.scale).round() as u32
                        && frame.pixel_height
                            == (expected.height as f32 * expected.scale).round() as u32,
                    "wrong legacy render extent"
                );
                None
            };
            if let Some(stamp) = curve {
                ensure!(
                    stamp.view == view
                        && stamp.viewport == frame.viewport
                        && stamp.frame_seq == frame.seq
                        && stamp.grant_serial != 0
                        && stamp.gp != 0
                        && stamp.gc != 0,
                    "wrong Curve frame publication identity"
                );
            }
            // Validate before taking the lease so every error drops the lock
            // before failure cleanup releases and notifies frame credit.
            let (_, lease) = state.renders.remove(&request).expect("validated render");
            state.pending.remove(&request);
            drop(state);
            shared.view.get().unwrap().complete(
                crate::ReceivedFrame::received(frame.clone(), right),
                lease,
                curve.cloned(),
                layer,
            );
            shared.changed.notify_all();
            continue;
        }
        let mut state = shared.state.lock().unwrap();
        if state.abort {
            return Ok(());
        }
        let mut invalidated = false;
        let mut synchronous_content = false;
        let mut content_changed = false;
        let mut discard_curve_reply = false;
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
                        let State {
                            pending,
                            curve_pending,
                            ..
                        } = &mut *state;
                        consume_generic_response_pending(pending, curve_pending, *request)?;
                        synchronous_content = state.synchronous_content.remove(request);
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
                    Output::CurveReplyV1 {
                        request,
                        view,
                        reply,
                    } => {
                        let State {
                            pending,
                            curve_pending,
                            ..
                        } = &mut *state;
                        discard_curve_reply = consume_curve_rejection(
                            pending,
                            curve_pending,
                            *request,
                            *view,
                            reply,
                        )?;
                        if matches!(
                            reply,
                            crate::component::curve::CurveReply::LayerNegotiated { .. }
                        ) {
                            state.layer_view = Some(*view);
                        } else if matches!(
                            reply,
                            crate::component::curve::CurveReply::Negotiated { .. }
                        ) {
                            state.layer_view = None;
                        }
                        content_changed = matches!(
                            reply,
                            crate::component::curve::CurveReply::ContentInput {
                                outcome:
                                    crate::component::curve::ContentInputOutcome::Applied {
                                        reply,
                                    },
                                ..
                            } if reply.changed
                        );
                        synchronous_content = state.synchronous_content.remove(request);
                    }
                    Output::Fatal { message } => bail!("worker fatal: {message}"),
                    _ => bail!("unnegotiated output"),
                }
            }
        }
        // View invalidation is a coalesced redraw signal, not business output.
        // It must not fill the service-event queue while the View is hidden.
        if !invalidated && !discard_curve_reply {
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
            if invalidated || content_changed {
                view.invalidate();
            } else if !synchronous_content {
                view.remote_status(status);
            }
        }
        // Synchronous query/input replies wake their actual waiting caller.
        // A changed result still uses view.invalidate() at that caller; a
        // read-only native query must not schedule a new UI query in a loop.
        shared.changed.notify_all();
        if !invalidated && !synchronous_content {
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
    use crate::component::curve::{CurveErrorCode, CurveReply};

    #[cfg(target_os = "macos")]
    #[test]
    fn view_work_notification_cannot_pass_the_writer_predicate_lock() {
        use std::{sync::mpsc, time::Duration};
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                status: ProcessStatus::Starting,
                abort: false,
                stopping: false,
                next: 1,
                outgoing: VecDeque::new(),
                incoming: VecDeque::new(),
                pending: HashSet::new(),
                curve_pending: HashMap::new(),
                synchronous_content: HashSet::new(),
                toolbar: Vec::new(),
                toolbar_revision: 0,
                toolbar_window: 0,
                toolbar_declaration: None,
                view_revision: 0,
                native: None,
                expected_view: Some(1),
                layer_view: None,
                stop_id: None,
                renders: HashMap::new(),
            }),
            changed: Condvar::new(),
            child: crate::ProcessControl::new(),
            view: std::sync::OnceLock::new(),
            wake: Arc::new(|| {}),
        });
        let (queued_tx, queued_rx) = mpsc::channel();
        let (notified_tx, notified_rx) = mpsc::channel();
        let weak = Arc::downgrade(&shared);
        let view = Arc::new(crate::ViewConnection::remote(
            shared.child.clone(),
            Arc::new(|| {}),
            Arc::new(move || {
                queued_tx.send(()).unwrap();
                weak.upgrade().unwrap().wake_view_writer();
                notified_tx.send(()).unwrap();
            }),
        ));
        // This is the real writer's predicate-check -> Condvar::wait window.
        let guard = shared.state.lock().unwrap();
        assert!(view.try_start().is_none());
        let producer = view.clone();
        let thread = std::thread::spawn(move || {
            producer.enqueue(tekes_ui_app_protocol::Request {
                version: tekes_ui_app_protocol::VERSION,
                seq: 1,
                viewport: 1,
                kind: tekes_ui_app_protocol::InputKind::Layout,
                command: None,
                x: 0.,
                y: 0.,
                width: 1,
                height: 1,
                identity: 1,
                scale: 1.,
            })
        });
        queued_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let notified_before_wait = notified_rx.recv_timeout(Duration::from_millis(100)).is_ok();
        drop(guard);
        thread.join().unwrap().unwrap();
        assert!(
            !notified_before_wait,
            "View work can lose its wake before the writer waits"
        );
        let (request, lease) = view.try_start().unwrap();
        assert_eq!(request.seq, 1);
        drop(lease);
        let mut next = request;
        next.seq += 1;
        view.enqueue(next).unwrap();
        let (request, lease) = view.try_start().unwrap();
        shared
            .state
            .lock()
            .unwrap()
            .renders
            .insert(1, (request, lease));
        let failed = shared.clone();
        let (failed_tx, failed_rx) = mpsc::channel();
        let cleanup = std::thread::spawn(move || {
            failed.fail("controlled terminal cleanup".into());
            failed_tx.send(()).unwrap();
        });
        failed_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("cleanup held the writer lock while releasing View credit");
        cleanup.join().unwrap();
        assert_eq!(view.outstanding_frames(), 0);
    }

    fn rejected(tx: u64) -> CurveReply {
        CurveReply::Rejected {
            transaction_id: tx,
            code: CurveErrorCode::UnsupportedFeature,
        }
    }
    #[test]
    fn actual_curve_json_bytes_use_the_existing_exclusive_pipe_packet_limit() {
        use std::io::Cursor;
        let packet = ClientMessage::Request {
            version: VERSION,
            session: Session {
                module: "fixture".into(),
                instance: 7,
                host_epoch: 9,
            },
            id: 11,
            body: RequestBody::CurveV1(crate::component::curve::Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: 29,
                ttl_ms: 0,
                body: crate::component::curve::RequestBodyV1::TransactionStatus,
            }),
        };
        let mut actual = serde_json::to_vec(&packet).unwrap();
        assert!(actual.len() < crate::pipes::MAX_PACKET - 1);
        actual.resize(crate::pipes::MAX_PACKET - 1, b' ');
        let mut framed = Vec::new();
        crate::pipes::write_packet(&mut framed, &actual).unwrap();
        let mut reader = Cursor::new(framed);
        let read = crate::pipes::read_packet(&mut reader).unwrap().unwrap();
        assert_eq!(read.len(), crate::pipes::MAX_PACKET - 1);
        assert!(matches!(
            serde_json::from_slice::<ClientMessage>(&read).unwrap(),
            ClientMessage::Request {
                body: RequestBody::CurveV1(_),
                ..
            }
        ));

        actual.push(b' ');
        assert_eq!(actual.len(), crate::pipes::MAX_PACKET);
        assert!(crate::pipes::write_packet(&mut Vec::new(), &actual).is_err());
        let mut oversized_frame = actual;
        oversized_frame.push(b'\n');
        assert!(crate::pipes::read_packet(&mut Cursor::new(oversized_frame)).is_err());
    }
    #[test]
    fn outer_session_and_serial_guard_precedes_curve_reply_routing() {
        let expected = Session {
            module: "fixture".into(),
            instance: 7,
            host_epoch: 9,
        };
        let packet = ServerMessage {
            version: VERSION,
            session: expected.clone(),
            serial: 3,
            output: Output::CurveReplyV1 {
                request: 7,
                view: 23,
                reply: rejected(29),
            },
        };
        assert!(validate_server_identity(&packet, &expected, 2).is_ok());
        assert!(validate_server_identity(&packet, &expected, 3).is_err());
        let other = Session {
            host_epoch: 10,
            ..expected.clone()
        };
        assert!(validate_server_identity(&packet, &other, 2).is_err());
        let wrong_version = ServerMessage {
            version: VERSION + 1,
            ..packet
        };
        assert!(validate_server_identity(&wrong_version, &expected, 2).is_err());
    }
    #[test]
    fn curve_reply_consumes_only_the_exact_existing_request_slot() {
        let mut pending = HashSet::from([7, 8]);
        let mut curve_pending = HashMap::from([(
            7,
            CurvePending {
                layer_contract: false,
                attached_view: 23,
                transaction_id: 29,
                offer: None,
                proposal: None,
                revoke_serial: None,
                query: None,
                content_input: false,
                discard_reply: false,
            },
        )]);
        assert!(
            consume_curve_rejection(&mut pending, &mut curve_pending, 7, 23, &rejected(29)).is_ok()
        );
        assert!(!pending.contains(&7));
        assert!(!curve_pending.contains_key(&7));
        assert!(pending.contains(&8));
        assert!(
            consume_curve_rejection(&mut pending, &mut curve_pending, 7, 23, &rejected(29))
                .is_err()
        );
        assert!(pending.contains(&8));
    }
    #[test]
    fn curve_content_input_reply_preserves_the_exact_frame_stamp() {
        let stamp = crate::component::curve::QueryStamp {
            grant_serial: 1,
            gp: 2,
            gc: 3,
            view: 23,
            viewport: 4,
            allocation_seq: 5,
            layout_seq: 6,
            published_frame_seq: 7,
        };
        let pending_for = || {
            (
                HashSet::from([7]),
                HashMap::from([(
                    7,
                    CurvePending {
                        layer_contract: false,
                        attached_view: 23,
                        transaction_id: 0,
                        offer: None,
                        proposal: None,
                        revoke_serial: None,
                        query: Some(stamp.clone()),
                        content_input: true,
                        discard_reply: false,
                    },
                )]),
            )
        };
        let reply = |stamp| CurveReply::ContentInput {
            transaction_id: 0,
            stamp,
            outcome: crate::component::curve::ContentInputOutcome::Applied {
                reply: Box::new(crate::component::content::Reply {
                    view: 23,
                    viewport: 4,
                    consumed: false,
                    changed: false,
                    animation_frame_requested: false,
                    capture_pointer: false,
                    activated: false,
                    bubbles: false,
                    pointer_focus_requested: false,
                    accessibility_focus_requested: false,
                    key: None,
                    overlay: None,
                    reading_scroll_source: None,
                    reading: None,
                    semantics: None,
                    focused: None,
                    focus_epoch: 0,
                    text_binding: None,
                    captured: None,
                    focus_targets: Vec::new(),
                    composing: false,
                    text: None,
                }),
            },
        };
        let (mut pending, mut curve_pending) = pending_for();
        assert!(
            consume_curve_rejection(
                &mut pending,
                &mut curve_pending,
                7,
                23,
                &reply(stamp.clone())
            )
            .is_ok()
        );
        assert!(!pending.contains(&7) && !curve_pending.contains_key(&7));

        let (mut pending, mut curve_pending) = pending_for();
        let stale = crate::component::curve::QueryStamp {
            published_frame_seq: 8,
            ..stamp
        };
        assert!(
            consume_curve_rejection(&mut pending, &mut curve_pending, 7, 23, &reply(stale))
                .is_err()
        );
        assert!(pending.contains(&7) && curve_pending.contains_key(&7));
    }
    #[test]
    fn curve_mismatches_and_negotiated_reply_fail_without_consuming_an_unrelated_pending() {
        let capability = crate::component::curve::CapabilityWire {
            protocol_major: 1,
            wire_revision: 1,
            clip_geometry_revision: 1,
            coverage_aa_revision: 1,
            features: crate::component::curve::REQUIRED_FEATURES,
            limits: crate::component::curve::LimitsWire {
                packet_bytes_exclusive: 4096,
                records: 4,
                depth: 2,
                validation_steps: 12,
                target_axis_px: 64,
                ttl_ms: 10,
            },
        };
        let mut generic_pending = HashSet::from([7, 8]);
        let generic_curve_pending = HashMap::from([(
            7,
            CurvePending {
                layer_contract: false,
                attached_view: 23,
                transaction_id: 29,
                offer: None,
                proposal: None,
                revoke_serial: None,
                query: None,
                content_input: false,
                discard_reply: false,
            },
        )]);
        assert!(
            consume_generic_response_pending(&mut generic_pending, &generic_curve_pending, 7)
                .is_err()
        );
        assert!(generic_pending.contains(&7));
        assert!(
            consume_generic_response_pending(&mut generic_pending, &generic_curve_pending, 8)
                .is_ok()
        );
        assert!(!generic_pending.contains(&8));
        for (request, view, reply) in [
            (9, 23, rejected(29)),
            (8, 23, rejected(29)),
            (7, 24, rejected(29)),
            (7, 23, rejected(30)),
            (
                7,
                23,
                CurveReply::Negotiated {
                    capability: capability.clone(),
                },
            ),
        ] {
            let mut pending = HashSet::from([7, 8]);
            let mut curve_pending = HashMap::from([(
                7,
                CurvePending {
                    layer_contract: false,
                    attached_view: 23,
                    transaction_id: 29,
                    offer: None,
                    proposal: None,
                    revoke_serial: None,
                    query: None,
                    content_input: false,
                    discard_reply: false,
                },
            )]);
            assert!(
                consume_curve_rejection(&mut pending, &mut curve_pending, request, view, &reply)
                    .is_err()
            );
            assert!(pending.contains(&7) && pending.contains(&8));
            assert!(curve_pending.contains_key(&7));
        }
    }
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
        // Two accepted registry events, as required by the original test.
        // A headless toolbar has no View to invalidate.
        assert_eq!(emitter.sink.lock().unwrap().serial, 2);
        assert_eq!(emitter.sink.lock().unwrap().view_revision, 0);
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
        assert_eq!(emitter.invalidate_view_next().unwrap(), 3);
        assert_eq!(emitter.sink.lock().unwrap().serial, 3);
        emitter.view = None;
        emitter.capabilities = Capabilities::HEADLESS;
        assert!(!emitter.invalidate_view(3).unwrap());
        assert_eq!(emitter.view_identity(), None);
        assert_eq!(emitter.sink.lock().unwrap().serial, 3);
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

    #[cfg(target_os = "macos")]
    mod curve_actor_regressions {
        use super::*;
        use crate::component::curve::*;
        use tekes_ui_controls::{
            Button, ButtonTextAlign, ButtonTextRenderer,
            composition::{ComponentId, Content, ContentHost},
        };
        use tekes_ui_scene::{GlyphAtlas, Rect, Scene, Size};
        use tekes_ui_tokens::{Rgba, Theme, proposal};

        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
                Size::new(32., 16.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: proposal::TextRole,
                _: Rect,
                _: ButtonTextAlign,
                _: Rgba,
            ) {
            }
        }
        struct CapturedBytes(Arc<Mutex<Vec<u8>>>);
        impl Write for CapturedBytes {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        struct OriginalHost {
            host: ContentHost,
            text: Text,
            snapshots: Arc<Mutex<Vec<Value>>>,
        }
        impl OriginalHost {
            fn snapshot(&self) -> Value {
                serde_json::json!({
                    "geometry": self.host.geometry_snapshot().map(|s| (s.generation, s.is_current())),
                    "mount": self.host.semantic_mount(ComponentId(1)),
                    "focused": self.host.focused().map(|id| id.0),
                    "focus_epoch": self.host.focus_epoch(),
                    "requires_curve": self.host.requires_curve_geometry(),
                    "external_ack": self.host.external_curve_ack(),
                    "needs_render": self.host.needs_render(),
                })
            }
        }
        impl Drop for OriginalHost {
            fn drop(&mut self) {
                self.snapshots.lock().unwrap().push(self.snapshot());
            }
        }
        impl crate::PanelModule for OriginalHost {
            fn update(
                &mut self,
                _: &tekes_ui_app_protocol::Request,
                _: &mut Scene,
                _: &mut GlyphAtlas,
            ) -> Result<crate::ModuleState> {
                anyhow::bail!("refusal-only actor regression must not request a GPU frame")
            }
        }
        impl ProcessModule for OriginalHost {
            fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
                Some((&mut self.host, &mut self.text))
            }
            fn view(&mut self) -> Option<&mut dyn crate::PanelModule> {
                Some(self)
            }
            fn view_capabilities(&self) -> ViewCapabilities {
                ViewCapabilities::default()
            }
            fn handle(&mut self, body: RequestBody, _: &ComponentEmitter) -> Result<Value> {
                ensure!(
                    matches!(body, RequestBody::Message { .. }),
                    "unexpected service request"
                );
                Ok(self.snapshot())
            }
        }
        fn capability() -> CapabilityWire {
            CapabilityWire {
                protocol_major: 1,
                wire_revision: 1,
                clip_geometry_revision: 1,
                coverage_aa_revision: 1,
                features: REQUIRED_FEATURES,
                limits: LimitsWire {
                    packet_bytes_exclusive: 65536,
                    records: 8,
                    depth: 8,
                    validation_steps: 24,
                    target_axis_px: 64,
                    ttl_ms: 100,
                },
            }
        }
        fn grant(serial: u64) -> Envelope {
            Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: serial,
                ttl_ms: 50,
                body: RequestBodyV1::PrepareGrant {
                    proposal: Box::new(GrantProposal {
                        proposal_id: serial,
                        viewport: 1,
                        allocation_seq: 1,
                        layout_seq: 1,
                        parent_geometry_generation: 5,
                        target_px: [64, 48],
                        physical_origin_px: [0., 0.],
                        backing_scale: 1.,
                        clear_rgba: [0.; 4],
                        sample_count: 1,
                        coordinate_mapping: "SamePhysicalCoordinates".into(),
                        chain_head: 0,
                        chain_depth: 1,
                        records: BoundedRecords(vec![ShapeRecord {
                            index: 0,
                            parent_index: None,
                            depth: 1,
                            rect_px: [0., 0., 64., 48.],
                            radii_px: [0.; 4],
                        }]),
                    }),
                },
            }
        }
        fn negotiate() -> RequestBody {
            RequestBody::CurveV1(Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: 0,
                ttl_ms: 0,
                body: RequestBodyV1::Negotiate {
                    offer: capability(),
                },
            })
        }
        fn snapshot_request() -> RequestBody {
            RequestBody::Message {
                topic: "observe-original-host".into(),
                payload: Value::Null,
            }
        }
        struct ActorResult {
            result: Result<bool>,
            output: Vec<ServerMessage>,
            lifetime_snapshots: Vec<Value>,
        }
        fn run_actor(bodies: Vec<RequestBody>) -> ActorResult {
            let session = Session {
                module: "actual-curve-actor".into(),
                instance: 1,
                host_epoch: 81,
            };
            let caps = Capabilities {
                view: true,
                ..Capabilities::HEADLESS
            };
            let attach = ViewAttach {
                window: 7,
                view: 23,
                endpoint: "refusal-only-no-render".into(),
                required: ViewCapabilities::default(),
            };
            let captured = Arc::new(Mutex::new(Vec::new()));
            let snapshots = Arc::new(Mutex::new(Vec::new()));
            let mut host =
                ContentHost::new(Content::new(ComponentId(1), Button::new("Original"))).unwrap();
            let frame = Rect::new(0., 0., 64., 48.);
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            let module = OriginalHost {
                host,
                text: Text,
                snapshots: snapshots.clone(),
            };
            snapshots.lock().unwrap().push(module.snapshot());
            let emitter = ComponentEmitter {
                session: session.clone(),
                capabilities: caps,
                view: Some(NativeViewOptions {
                    window: attach.window,
                    view: attach.view,
                    required: attach.required,
                }),
                window: Some(attach.window),
                sink: Arc::new(Mutex::new(Sink {
                    writer: Box::new(CapturedBytes(captured.clone())),
                    serial: 0,
                    toolbar: None,
                    view_revision: 0,
                })),
            };
            let mut bytes = Vec::new();
            for (index, body) in bodies.into_iter().enumerate() {
                let packet = ClientMessage::Request {
                    version: VERSION,
                    session: session.clone(),
                    id: index as u64 + 1,
                    body,
                };
                pipes::write_packet(&mut bytes, &serde_json::to_vec(&packet).unwrap()).unwrap();
            }
            let result = crate::process_view::run(
                Box::new(module),
                emitter,
                std::io::Cursor::new(bytes),
                attach,
                caps,
            );
            let output = captured
                .lock()
                .unwrap()
                .split(|b| *b == b'\n')
                .filter(|line| !line.is_empty())
                .map(|line| serde_json::from_slice::<ServerMessage>(line).unwrap())
                .collect();
            let lifetime_snapshots = snapshots.lock().unwrap().clone();
            ActorResult {
                result,
                output,
                lifetime_snapshots,
            }
        }
        fn response(run: &ActorResult, id: u64) -> &Value {
            run.output
                .iter()
                .find_map(|packet| match &packet.output {
                    Output::Response {
                        request,
                        result: Ok(value),
                    } if *request == id => Some(value),
                    _ => None,
                })
                .expect("actor did not answer its original service lane")
        }
        #[test]
        fn malformed_grants_preserve_original_host_and_pending_grant_in_actual_actor() {
            let mut malformed = grant(101);
            let RequestBodyV1::PrepareGrant { proposal } = &mut malformed.body else {
                unreachable!()
            };
            proposal.records.0[0].parent_index = Some(0);
            let revoke = RequestBody::CurveV1(Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: 0,
                ttl_ms: 0,
                body: RequestBodyV1::RevokeAdmission {
                    grant_serial: 100,
                    reason: RevokeReasonWire::ProtocolFailure,
                },
            });
            let run = run_actor(vec![
                snapshot_request(),
                negotiate(),
                RequestBody::CurveV1(malformed.clone()),
                snapshot_request(),
                RequestBody::CurveV1(grant(100)),
                snapshot_request(),
                RequestBody::CurveV1(malformed),
                snapshot_request(),
                revoke,
                RequestBody::Stop,
            ]);
            assert!(matches!(run.result, Ok(true)), "{:?}", run.result);
            assert!(response(&run, 1)["geometry"].is_array());
            assert_eq!(
                response(&run, 1),
                response(&run, 4),
                "malformed grant mutated original Host"
            );
            assert_eq!(
                response(&run, 6),
                response(&run, 8),
                "malformed replacement mutated pending Host"
            );
            assert_eq!(response(&run, 6)["requires_curve"], true);
            for request_id in [3, 7] {
                assert!(run.output.iter().any(|packet| matches!(packet.output,
                    Output::CurveReplyV1 { request, reply: CurveReply::Rejected { code: CurveErrorCode::MalformedMessage, .. }, .. }
                    if request == request_id)));
            }
            assert!(run.output.iter().any(|packet| matches!(
                packet.output,
                Output::CurveReplyV1 {
                    request: 9,
                    reply: CurveReply::GrantRevoked {
                        grant_serial: 100,
                        ..
                    },
                    ..
                }
            )));
            assert!(!run.output.iter().any(|packet| matches!(
                packet.output,
                Output::Frame { .. }
                    | Output::CurveReplyV1 {
                        reply: CurveReply::FramePublished { .. } | CurveReply::GrantPrepared { .. },
                        ..
                    }
            )));
        }
        #[test]
        fn independent_layer_target_mismatch_refuses_atomically_and_fractional_grant_survives() {
            let layer_grant = |serial, pixel_width| {
                let mut original = grant(serial);
                let RequestBodyV1::PrepareGrant { mut proposal } = original.body else {
                    unreachable!()
                };
                proposal.physical_origin_px = [666.6667, 0.];
                proposal.backing_scale = 2.;
                proposal.target_px = [pixel_width, 1000];
                proposal.coordinate_mapping = "IndependentRasterLayerV1".into();
                proposal.records.0[0].rect_px = [666.6667, 0., 666.6667, 1000.];
                original.body = RequestBodyV1::PrepareLayerGrant {
                    proposal: Box::new(LayerGrantProposal {
                        geometry: proposal,
                        logical_size: [333, 500],
                    }),
                };
                RequestBody::CurveV1(original)
            };
            let negotiate_layer = RequestBody::CurveV1(Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: 0,
                ttl_ms: 0,
                body: RequestBodyV1::NegotiateLayer {
                    offer: CapabilityWire {
                        limits: LimitsWire {
                            target_axis_px: 4096,
                            ..capability().limits
                        },
                        ..capability()
                    },
                },
            });
            let revoke = RequestBody::CurveV1(Envelope {
                protocol_major: 1,
                wire_revision: 1,
                view: 23,
                transaction_id: 0,
                ttl_ms: 0,
                body: RequestBodyV1::RevokeAdmission {
                    grant_serial: 100,
                    reason: RevokeReasonWire::ProtocolFailure,
                },
            });
            let run = run_actor(vec![
                snapshot_request(),
                negotiate_layer,
                layer_grant(101, 666),
                snapshot_request(),
                layer_grant(100, 667),
                snapshot_request(),
                layer_grant(102, 666),
                snapshot_request(),
                revoke,
                RequestBody::Stop,
            ]);
            assert!(matches!(run.result, Ok(true)), "{:?}", run.result);
            assert_eq!(response(&run, 1), response(&run, 4));
            assert_eq!(response(&run, 6), response(&run, 8));
            for request_id in [3, 7] {
                assert!(run.output.iter().any(|packet| matches!(packet.output,
                    Output::CurveReplyV1 { request, reply: CurveReply::Rejected { code: CurveErrorCode::MalformedMessage, .. }, .. }
                    if request == request_id)));
            }
            assert!(run.output.iter().any(|packet| matches!(
                packet.output,
                Output::CurveReplyV1 {
                    request: 2,
                    reply: CurveReply::LayerNegotiated { .. },
                    ..
                }
            )));
            assert!(run.output.iter().any(|packet| matches!(
                packet.output,
                Output::CurveReplyV1 {
                    request: 9,
                    reply: CurveReply::GrantRevoked {
                        grant_serial: 100,
                        ..
                    },
                    ..
                }
            )));
            assert!(!run.output.iter().any(|packet| matches!(
                packet.output,
                Output::Frame { .. }
                    | Output::CurveReplyV1 {
                        reply: CurveReply::FramePublished { .. }
                            | CurveReply::LayerFramePublished { .. }
                            | CurveReply::GrantPrepared { .. },
                        ..
                    }
            )));
        }
        #[test]
        fn layer_negotiation_does_not_accept_a_legacy_proof_acknowledgement() {
            let make = |layer_contract| {
                (
                    HashSet::from([7]),
                    HashMap::from([(
                        7,
                        CurvePending {
                            layer_contract,
                            attached_view: 23,
                            transaction_id: 0,
                            offer: Some(capability()),
                            proposal: None,
                            revoke_serial: None,
                            query: None,
                            content_input: false,
                            discard_reply: false,
                        },
                    )]),
                )
            };
            let (mut pending, mut requests) = make(true);
            assert!(
                consume_curve_rejection(
                    &mut pending,
                    &mut requests,
                    7,
                    23,
                    &CurveReply::Negotiated {
                        capability: capability()
                    }
                )
                .is_err()
            );
            assert!(pending.contains(&7) && requests.contains_key(&7));
            assert!(
                consume_curve_rejection(
                    &mut pending,
                    &mut requests,
                    7,
                    23,
                    &CurveReply::LayerNegotiated {
                        capability: capability()
                    }
                )
                .is_ok()
            );
            assert!(pending.is_empty() && requests.is_empty());
            let (mut pending, mut requests) = make(false);
            assert!(
                consume_curve_rejection(
                    &mut pending,
                    &mut requests,
                    7,
                    23,
                    &CurveReply::LayerNegotiated {
                        capability: capability()
                    }
                )
                .is_err()
            );
            assert!(pending.contains(&7) && requests.contains_key(&7));
        }
        #[test]
        fn failed_child_shape_conversion_preserves_original_host_without_publication() {
            let mut invalid_shape = grant(100);
            let RequestBodyV1::PrepareGrant { proposal } = &mut invalid_shape.body else {
                unreachable!()
            };
            proposal.records.0[0].rect_px = [f32::MAX, 0., f32::MAX, 48.];
            assert_eq!(validate_for_attached_view(&invalid_shape, Some(23)), Ok(()));
            let run = run_actor(vec![negotiate(), RequestBody::CurveV1(invalid_shape)]);
            let error = run.result.as_ref().unwrap_err().to_string();
            assert!(error.contains("invalid Curve clip shape"), "{error}");
            assert_eq!(run.lifetime_snapshots.len(), 2);
            assert!(run.lifetime_snapshots[0]["geometry"].is_array());
            assert_eq!(
                run.lifetime_snapshots[0], run.lifetime_snapshots[1],
                "failed conversion mutated Host before returning failure"
            );
            assert_eq!(
                run.output.len(),
                2,
                "failure published more than ReadyView and Negotiated"
            );
            assert!(matches!(run.output[0].output, Output::ReadyView { .. }));
            assert!(matches!(
                run.output[1].output,
                Output::CurveReplyV1 {
                    reply: CurveReply::Negotiated { .. },
                    ..
                }
            ));
        }
    }
}
