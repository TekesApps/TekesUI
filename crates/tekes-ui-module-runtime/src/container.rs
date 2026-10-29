//! Standard Content adapter for negotiated original Content and legacy Views.
//! GPU attachment uses one prepare call; consumers do not own frame state machines.
use crate::{
    NativeViewOptions, ProcessComponent, ProcessComponentOptions, ProcessStatus, ViewImage,
};
use anyhow::{Context, Result, bail, ensure};
use std::sync::Arc;
use tekes_ui_app_protocol::{InputKind, Request, VERSION};
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{
        Component, ContentEvent, ContentKeyEvent, ContentResponse, ContentRole, ContentSemantics,
        ContentTheme, Lifecycle, PointerButton, PointerEvent, PointerPhase,
    },
    layout::MeasuredChild,
};
use tekes_ui_render::{Renderer, gpu::GpuImageRegistration};
use tekes_ui_scene::{ImageQuad, Point, Rect, Scene, Size};

fn trace_layer(args: std::fmt::Arguments<'_>) {
    if std::env::var_os("TEKESUI_PROCESS_LAYER_TRACE").is_some() {
        eprintln!(
            "PROCESS_LAYER_TRACE pid={} at={:?} {args}",
            std::process::id(),
            std::time::Instant::now()
        );
    }
}

#[derive(Debug)]
struct PendingContentInput {
    session: crate::component::Session,
    view: u64,
    owner: Option<crate::component::content::Target>,
    focus_epoch: u64,
    text_binding: Option<Vec<u8>>,
    capture: Option<crate::component::content::Capture>,
    input: crate::component::content::Input,
    original_key_binding: Option<Vec<u8>>,
    bytes: usize,
}

pub struct ProcessContainer {
    module: String,
    host_epoch: u64,
    native: Option<NativeViewOptions>,
    view_context: Option<crate::component::ViewContext>,
    wake: Arc<dyn Fn() + Send + Sync>,
    process: Option<Arc<ProcessComponent>>,
    frame_resources: Arc<std::sync::Mutex<FrameResources>>,
    frame: Rect,
    view_frame: Rect,
    toolbar: crate::container_toolbar::ContainerToolbar,
    pointer_focus: Option<bool>,
    toolbar_focused: bool,
    clip: Rect,
    scale: f32,
    viewport: u64,
    seq: u64,
    allocation_seq: u64,
    layout_seq: u64,
    geometry_dirty: bool,
    repaint_dirty: bool,
    visible: bool,
    enabled: bool,
    window_active: bool,
    opaque_pointer: bool,
    pressed: bool,
    error: Option<String>,
    // Read-only child authority. No local editor, press or focus-target model.
    content_snapshot: Option<crate::ProcessContentSnapshot>,
    semantic_projection: crate::container_semantics::RemoteSemantics,
    accessibility_focus: bool,
    content_focused: bool,
    content_bubbles: Option<bool>,
    content_preserves_focus: bool,
    content_reading_scroll: bool,
    geometry_input_pending: bool,
    pending_content: std::collections::VecDeque<PendingContentInput>,
    pending_content_bytes: usize,
    // One original latest native-finish receipt, not another input/focus model.
    deferred_key_result: Option<(ContentResponse, bool, Option<Vec<u8>>)>,
    curve_preflight: CurvePreflight,
    curve_request: Option<(u64, u64)>,
    curve_transaction: u64,
    curve_grant_request: Option<(u64, u64, crate::component::curve::GrantProposal)>,
    curve_geometry: Option<tekes_ui_controls::composition::CurveGeometryContext>,
}
#[derive(Default)]
struct FrameResources {
    registration: Option<GpuImageRegistration>,
    image: Option<ViewImage>,
    expected_viewport: u64,
    expected_seq: u64,
    expected_size: (u32, u32),
    expected_scale: f32,
    layer_layout: Option<crate::layer_raster::LayerRasterLayout>,
    visible: bool,
    view: u64,
    parent_generation: u64,
    child_generation: u64,
    grant_serial: u64,
    allocation_seq: u64,
    layout_seq: u64,
    grant_acknowledged: bool,
    deferred_curve_grant: bool,
    pending_revokes: std::collections::VecDeque<(u64, u64, u64)>,
    pending_responses: std::collections::VecDeque<u64>,
    pending_curve_frame: Option<PendingCurveFrame>,
    pending_curve_publication: Option<PendingCurvePublication>,
    retired_curve_query: Option<u64>,
    geometry_cancel_confirmed: bool,
}
struct PendingCurveFrame {
    image: ViewImage,
    stamp: crate::component::curve::FrameStamp,
    query_stamp: crate::component::curve::QueryStamp,
    query_request: u64,
}
struct PendingCurvePublication {
    image: ViewImage,
    registration: GpuImageRegistration,
    stamp: crate::component::curve::FrameStamp,
    query_stamp: crate::component::curve::QueryStamp,
    snapshot: crate::ProcessContentSnapshot,
}
fn curve_query_stamp(
    frame: &crate::component::curve::FrameStamp,
) -> crate::component::curve::QueryStamp {
    crate::component::curve::QueryStamp {
        grant_serial: frame.grant_serial,
        gp: frame.gp,
        gc: frame.gc,
        view: frame.view,
        viewport: frame.viewport,
        allocation_seq: frame.allocation_seq,
        layout_seq: frame.layout_seq,
        published_frame_seq: frame.frame_seq,
    }
}
/// A non-owning route for the exact display resource only. It cannot outlive
/// the ProcessContainer or retain its child process authority.
pub struct ProcessContainerFrameHandle {
    process: std::sync::Weak<ProcessComponent>,
    resources: std::sync::Weak<std::sync::Mutex<FrameResources>>,
    session: crate::component::Session,
    view: u64,
    request: u64,
    transaction: u64,
    proposal: crate::component::curve::GrantProposal,
    layer: crate::layer_raster::LayerRasterLayout,
    frame: Rect,
    clip: Rect,
}
impl ProcessContainerFrameHandle {
    /// Receive only a current immutable child frame. UI events, toolbar changes
    /// and semantic scopes remain on the ordinary mutable Host path.
    #[cfg(target_os = "macos")]
    pub fn prepare_current_frame(&self, renderer: &mut Renderer) -> Result<bool> {
        let Some(process) = self.process.upgrade() else {
            return Ok(false);
        };
        ensure!(
            process.session() == &self.session,
            "stale frame handle session"
        );
        ensure!(
            matches!(process.status(), ProcessStatus::Ready { .. }),
            "frame owner unavailable"
        );
        let Some(resources) = self.resources.upgrade() else {
            return Ok(false);
        };
        let mut resources = resources.lock().unwrap();
        if let Some(request) = resources.retired_curve_query {
            if process.take_curve_reply(request).is_some() {
                resources.retired_curve_query = None;
            } else {
                return Ok(false);
            }
        }
        if process.has_pending_curve_host_updates() {
            if let Some(pending) = resources.pending_curve_frame.take() {
                resources.retired_curve_query = Some(pending.query_request);
            }
            resources.pending_curve_publication = None;
            return Ok(false);
        }
        while let Some(request) = resources.pending_responses.front().copied() {
            let Some(result) = process.take_response(request) else {
                return Ok(false);
            };
            result?;
            resources.pending_responses.pop_front();
        }
        while let Some((request, transaction, grant_serial)) =
            resources.pending_revokes.front().copied()
        {
            let Some(packet) = process.take_curve_reply(request) else {
                break;
            };
            let (view, reply) = match packet.output {
                crate::component::Output::CurveReplyV1 { view, reply, .. } => (view, reply),
                _ => unreachable!("take_curve_reply filters response type"),
            };
            ensure!(
                view == self.view,
                "curve revoke acknowledgement View mismatch"
            );
            match reply {
                crate::component::curve::CurveReply::GrantRevoked {
                    transaction_id,
                    grant_serial: actual,
                } => ensure!(
                    transaction_id == transaction && actual == grant_serial,
                    "curve revoke acknowledgement changed its grant"
                ),
                crate::component::curve::CurveReply::Rejected { code, .. } => {
                    bail!("curve revoke rejected: {code:?}")
                }
                _ => bail!("unexpected reply for curve revoke"),
            }
            resources.pending_revokes.pop_front();
            resources.geometry_cancel_confirmed = true;
            trace_layer(format_args!(
                "event=revoke_ack view={} grant={} transaction={}",
                self.view, grant_serial, transaction
            ));
        }
        if resources.layer_layout.as_ref() != Some(&self.layer)
            || resources.grant_serial != self.proposal.proposal_id
            || resources.parent_generation != self.proposal.parent_geometry_generation
            || resources.expected_viewport != self.proposal.viewport
            || resources.allocation_seq != self.proposal.allocation_seq
            || resources.layout_seq != self.proposal.layout_seq
            || !resources.visible
            || resources.expected_viewport == 0
            || resources.expected_seq == 0
        {
            return Ok(false);
        }
        if !resources.grant_acknowledged {
            let Some(packet) = process.take_curve_reply(self.request) else {
                return Ok(false);
            };
            let (view, reply) = match packet.output {
                crate::component::Output::CurveReplyV1 { view, reply, .. } => (view, reply),
                _ => unreachable!("take_curve_reply filters response type"),
            };
            ensure!(
                view == self.view,
                "curve grant acknowledgement View mismatch"
            );
            let child_generation = match reply {
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
                    ensure!(
                        transaction_id == self.transaction
                            && proposal_id == self.proposal.proposal_id
                            && grant_serial == self.proposal.proposal_id
                            && parent_geometry_generation
                                == self.proposal.parent_geometry_generation
                            && child_geometry_generation != 0
                            && viewport == self.proposal.viewport
                            && allocation_seq == self.proposal.allocation_seq
                            && layout_seq == self.proposal.layout_seq,
                        "curve grant acknowledgement changed its source geometry"
                    );
                    child_geometry_generation
                }
                crate::component::curve::CurveReply::Rejected { code, .. } => {
                    bail!("curve grant rejected: {code:?}")
                }
                crate::component::curve::CurveReply::Negotiated { .. }
                | crate::component::curve::CurveReply::LayerNegotiated { .. } => {
                    bail!("unexpected negotiation reply for curve grant")
                }
                crate::component::curve::CurveReply::GrantRevoked { .. } => {
                    bail!("unexpected revoke acknowledgement for curve grant")
                }
                crate::component::curve::CurveReply::ContentProjection { .. } => {
                    bail!("unexpected content projection for curve grant")
                }
                crate::component::curve::CurveReply::ContentInput { .. } => {
                    bail!("unexpected Content input reply for curve grant")
                }
                crate::component::curve::CurveReply::FramePublished { .. }
                | crate::component::curve::CurveReply::LayerFramePublished { .. } => {
                    bail!("unexpected frame publication reply for curve grant")
                }
            };
            resources.grant_acknowledged = true;
            resources.child_generation = child_generation;
            trace_layer(format_args!(
                "event=grant_ack view={} request={} proposal={} gp={} gc={} deferred={} expected_seq={}",
                self.view,
                self.request,
                self.proposal.proposal_id,
                self.proposal.parent_geometry_generation,
                child_generation,
                resources.deferred_curve_grant,
                resources.expected_seq
            ));
        }
        if resources.deferred_curve_grant {
            // GrantPrepared follows this job's frame output. Retire its frame
            // credit, but never publish pixels/queries for a superseded intent.
            drop(process.drain_view_frames());
            return Ok(false);
        }
        if resources.pending_curve_publication.is_some() {
            return Ok(true);
        }
        if let Some(pending) = resources.pending_curve_frame.take() {
            let Some(packet) = process.take_curve_reply(pending.query_request) else {
                resources.pending_curve_frame = Some(pending);
                return Ok(false);
            };
            let (view, reply) = match packet.output {
                crate::component::Output::CurveReplyV1 { view, reply, .. } => (view, reply),
                _ => unreachable!("take_curve_reply filters response type"),
            };
            ensure!(view == self.view, "curve projection View mismatch");
            let reply = match reply {
                crate::component::curve::CurveReply::ContentProjection {
                    transaction_id,
                    stamp,
                    reply,
                } => {
                    ensure!(
                        transaction_id == 0 && stamp == pending.query_stamp,
                        "curve projection changed its frame identity"
                    );
                    *reply
                }
                crate::component::curve::CurveReply::Rejected { code, .. } => {
                    bail!("curve read-only projection rejected: {code:?}")
                }
                _ => bail!("unexpected reply for curve read-only projection"),
            };
            let snapshot = crate::ProcessContentSnapshot::from_curve(
                &process,
                self.view,
                pending.stamp.viewport,
                self.frame,
                self.clip,
                pending.query_stamp.clone(),
                reply,
            )?;
            let semantics = snapshot
                .reply()
                .semantics
                .as_deref()
                .context("curve projection omitted original semantics")?;
            crate::container_semantics::validate(semantics)?;
            let registration = renderer.register_owned_gpu_image(pending.image.image())?;
            resources.pending_curve_publication = Some(PendingCurvePublication {
                image: pending.image,
                registration,
                stamp: pending.stamp,
                query_stamp: pending.query_stamp,
                snapshot,
            });
            return Ok(true);
        }
        if !resources.visible || resources.expected_viewport == 0 || resources.expected_seq == 0 {
            return Ok(false);
        }
        let old = resources
            .image
            .as_ref()
            .map(|image| (image.metadata().viewport, image.metadata().seq))
            .unwrap_or((0, 0));
        let mut selected = None;
        for frame in process.drain_view_frames() {
            let metadata = frame.metadata();
            let Some(stamp) = frame.curve_stamp() else {
                continue;
            };
            if metadata.viewport != resources.expected_viewport
                || metadata.seq < resources.expected_seq
                || metadata.viewport < old.0
                || metadata.seq <= old.1
                || (metadata.pixel_width, metadata.pixel_height) != resources.expected_size
                || frame.layer_layout() != Some(&self.layer)
                || self.layer.logical_size() != [metadata.width, metadata.height]
                || self.layer.scale() != resources.expected_scale
                || stamp.view != self.view
                || stamp.viewport != resources.expected_viewport
                || stamp.gp != resources.parent_generation
                || stamp.grant_serial != resources.grant_serial
                || stamp.allocation_seq != resources.allocation_seq
                || stamp.layout_seq != resources.layout_seq
                || stamp.frame_seq != metadata.seq
                || stamp.gc == 0
                || stamp.gc != resources.child_generation
            {
                continue;
            }
            selected = Some(frame);
        }
        let Some(frame) = selected else {
            return Ok(false);
        };
        let stamp = frame.curve_stamp().cloned().expect("validated Curve frame");
        let query_stamp = curve_query_stamp(&stamp);
        let envelope = crate::component::curve::Envelope {
            protocol_major: 1,
            wire_revision: 1,
            view: self.view,
            transaction_id: 0,
            ttl_ms: 0,
            body: crate::component::curve::RequestBodyV1::ContentQuery {
                query: crate::component::curve::ReadOnlyQueryWire::SemanticTree {
                    stamp: query_stamp.clone(),
                },
            },
        };
        let query_request = process.request(crate::component::RequestBody::CurveV1(envelope))?;
        let image = frame.import(&renderer.gpu_context())?;
        resources.pending_curve_frame = Some(PendingCurveFrame {
            image,
            stamp,
            query_stamp,
            query_request,
        });
        Ok(false)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum CurvePreflight {
    Unrequested,
    Starting,
    Negotiating,
    Negotiated {
        session: crate::component::Session,
        view: u64,
    },
    Rejected(crate::component::curve::CurveErrorCode),
    Failed(String),
}
fn curve_negotiation_envelope(view: u64) -> crate::component::curve::Envelope {
    use crate::component::curve::{
        CapabilityWire, Envelope, LimitsWire, REQUIRED_FEATURES, RequestBodyV1,
    };
    Envelope {
        protocol_major: 1,
        wire_revision: 1,
        view,
        transaction_id: 0,
        ttl_ms: 0,
        body: RequestBodyV1::NegotiateLayer {
            offer: CapabilityWire {
                protocol_major: 1,
                wire_revision: 1,
                clip_geometry_revision: 1,
                coverage_aa_revision: 1,
                features: REQUIRED_FEATURES,
                limits: LimitsWire {
                    packet_bytes_exclusive: 1_048_575,
                    records: 4096,
                    depth: 256,
                    validation_steps: 12_288,
                    target_axis_px: 4096,
                    ttl_ms: 1000,
                },
            },
        },
    }
}
impl std::fmt::Debug for ProcessContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessContainer")
            .field("module", &self.module)
            .field("viewport", &self.viewport)
            .field("mounted", &self.process.is_some())
            .finish_non_exhaustive()
    }
}
impl ProcessContainer {
    fn next_curve_transaction(&mut self) -> Result<u64> {
        self.curve_transaction = self
            .curve_transaction
            .checked_add(1)
            .context("curve transaction identity exhausted")?;
        Ok(self.curve_transaction)
    }
    fn grant_proposal(
        &self,
        proposal_id: u64,
        context: &tekes_ui_controls::composition::CurveGeometryContext,
    ) -> Result<(
        crate::component::curve::GrantProposal,
        crate::layer_raster::LayerRasterLayout,
    )> {
        use crate::component::curve::{BoundedRecords, GrantProposal, ShapeRecord};
        ensure!(proposal_id != 0, "invalid curve proposal identity");
        ensure!(
            !context.shapes().is_empty() && context.shapes().len() <= 256,
            "curve chain outside supported depth"
        );
        ensure!(
            self.scale.is_finite() && self.scale > 0.,
            "invalid curve backing scale"
        );
        let width = self.view_frame.size.width as u32;
        let height = self.view_frame.size.height as u32;
        let layer = crate::layer_raster::LayerRasterLayout::new(
            [width, height],
            self.scale,
            [
                self.view_frame.origin.x * self.scale,
                self.view_frame.origin.y * self.scale,
            ],
        )?;
        let target_px = layer.pixel_size();
        ensure!(
            target_px.0 > 0 && target_px.1 > 0 && target_px.0 <= 4096 && target_px.1 <= 4096,
            "curve target outside supported pixel dimensions"
        );
        let scale = self.scale;
        let records = context
            .shapes()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, shape)| {
                let rect = shape.rect();
                let radii = shape.radii();
                let index = u16::try_from(index).context("curve shape index overflow")?;
                Ok(ShapeRecord {
                    index,
                    parent_index: index.checked_sub(1),
                    depth: index.checked_add(1).context("curve depth overflow")?,
                    rect_px: [
                        rect.origin.x * scale,
                        rect.origin.y * scale,
                        rect.size.width * scale,
                        rect.size.height * scale,
                    ],
                    radii_px: [
                        radii.top_left * scale,
                        radii.top_right * scale,
                        radii.bottom_right * scale,
                        radii.bottom_left * scale,
                    ],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let chain_depth = u16::try_from(records.len()).context("curve chain depth overflow")?;
        let proposal = GrantProposal {
            proposal_id,
            viewport: self.viewport,
            allocation_seq: self.allocation_seq,
            layout_seq: self.layout_seq.saturating_add(1),
            parent_geometry_generation: context.generation(),
            target_px: [target_px.0, target_px.1],
            physical_origin_px: [
                self.view_frame.origin.x * scale,
                self.view_frame.origin.y * scale,
            ],
            backing_scale: scale,
            clear_rgba: [0., 0., 0., 0.],
            sample_count: 1,
            coordinate_mapping: "IndependentRasterLayerV1".into(),
            chain_head: chain_depth - 1,
            chain_depth,
            records: BoundedRecords(records),
        };
        Ok((proposal, layer))
    }
    /// Adapt a compiled V1 PanelModule. V1 has no per-event disposition reply;
    /// primary input is disabled until explicitly opting into an opaque sink.
    /// This profile does not support remote keyboard, scroll, editing/IME or AX.
    pub fn new_v1(
        module: impl Into<String>,
        host_epoch: u64,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let module = module.into();
        ensure!(
            !module.is_empty() && host_epoch != 0,
            "invalid container identity"
        );
        Ok(Self {
            module,
            host_epoch,
            native: None,
            view_context: None,
            wake: Arc::new(wake),
            process: None,
            frame_resources: Arc::new(std::sync::Mutex::new(FrameResources {
                visible: true,
                ..FrameResources::default()
            })),
            frame: Rect::default(),
            view_frame: Rect::default(),
            toolbar: Default::default(),
            pointer_focus: None,
            toolbar_focused: false,
            clip: Rect::default(),
            scale: 1.,
            viewport: 0,
            seq: 0,
            allocation_seq: 0,
            layout_seq: 0,
            geometry_dirty: true,
            repaint_dirty: false,
            visible: true,
            enabled: true,
            window_active: true,
            opaque_pointer: false,
            pressed: false,
            error: None,
            content_snapshot: None,
            semantic_projection: Default::default(),
            accessibility_focus: false,
            content_focused: false,
            content_bubbles: None,
            content_preserves_focus: false,
            content_reading_scroll: false,
            geometry_input_pending: false,
            pending_content: Default::default(),
            pending_content_bytes: 0,
            deferred_key_result: None,
            curve_preflight: CurvePreflight::Unrequested,
            curve_request: None,
            curve_transaction: 0,
            curve_grant_request: None,
            curve_geometry: None,
        })
    }
    /// Mount a negotiated View through the original Content lifecycle. Typed
    /// pointer, capture, focus and control keys route through this Component;
    /// legacy Views retain their explicit opaque-pointer opt-in. Native window
    /// text-query publication and AX remain separate contracts; negotiated Scroll
    /// uses the original child wheel/reading route and its exact disposition.
    pub fn new_view(
        options: ProcessComponentOptions,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self> {
        let native = options
            .view
            .ok_or_else(|| anyhow::anyhow!("View attachment required"))?;
        ensure!(
            native.window != 0 && native.view != 0,
            "invalid View identity"
        );
        ensure!(
            options.required
                == crate::component::Capabilities {
                    view: true,
                    ..crate::component::Capabilities::HEADLESS
                },
            "container requires service, toolbar and View capabilities"
        );
        let mut container = Self::new_v1(options.module, options.host_epoch, wake)?;
        container.native = Some(native);
        container.frame_resources.lock().unwrap().view = container.native.as_ref().unwrap().view;
        Ok(container)
    }
    /// Start this container's single process owner and begin curve negotiation
    /// without waiting. Call `curve_preflight_ready` from the existing wake
    /// loop until it returns `Negotiated`, then compose it under PanelContent.
    pub fn begin_curve_preflight(&mut self) -> Result<()> {
        ensure!(self.native.is_some(), "curve preflight requires a V2 View");
        ensure!(
            self.curve_preflight == CurvePreflight::Unrequested
                || matches!(
                    &self.curve_preflight,
                    CurvePreflight::Starting
                        | CurvePreflight::Negotiating
                        | CurvePreflight::Negotiated { .. }
                ),
            "curve preflight is terminal"
        );
        if self.curve_preflight == CurvePreflight::Unrequested {
            self.ensure_process_started()?;
            self.curve_preflight = CurvePreflight::Starting;
            (self.wake)();
        }
        Ok(())
    }
    /// Nonblocking progress query for the explicit pre-mount curve handshake.
    pub fn curve_preflight_ready(&mut self) -> Result<bool> {
        let Some(status) = self.process.as_ref().map(|process| process.status()) else {
            ensure!(
                !matches!(self.curve_preflight, CurvePreflight::Unrequested),
                "begin_curve_preflight must be called first"
            );
            bail!("curve preflight process is unavailable");
        };
        match status {
            ProcessStatus::Starting => return Ok(false),
            ProcessStatus::Failed { message } => {
                self.curve_preflight = CurvePreflight::Failed(message.clone());
                bail!("curve preflight failed: {message}");
            }
            ProcessStatus::Closed | ProcessStatus::Closing => {
                let message = "curve preflight owner closed".to_owned();
                self.curve_preflight = CurvePreflight::Failed(message.clone());
                bail!("{message}");
            }
            ProcessStatus::Ready { .. } => (),
        }
        match self.curve_preflight.clone() {
            CurvePreflight::Unrequested => bail!("begin_curve_preflight must be called first"),
            CurvePreflight::Starting => {
                let view = self.native.as_ref().expect("preflight requires View").view;
                let process = self.process.as_ref().expect("ready process exists").clone();
                ensure!(
                    process.view_capabilities().is_some(),
                    "View handshake missing"
                );
                let envelope = curve_negotiation_envelope(view);
                let request = process.request(crate::component::RequestBody::CurveV1(envelope))?;
                self.curve_request = Some((request, 0));
                self.curve_preflight = CurvePreflight::Negotiating;
                Ok(false)
            }
            CurvePreflight::Negotiating => {
                let (request, transaction) = self.curve_request.context("curve request missing")?;
                let Some(packet) = self
                    .process
                    .as_ref()
                    .and_then(|p| p.take_curve_reply(request))
                else {
                    return Ok(false);
                };
                let (view, reply) = match packet.output {
                    crate::component::Output::CurveReplyV1 { view, reply, .. } => (view, reply),
                    _ => unreachable!("take_curve_reply filters response type"),
                };
                let native_view = self.native.as_ref().context("curve View disappeared")?.view;
                ensure!(view == native_view, "curve negotiation View mismatch");
                match reply {
                    crate::component::curve::CurveReply::LayerNegotiated { capability } => {
                        ensure!(
                            capability.features == crate::component::curve::REQUIRED_FEATURES,
                            "peer omitted a required curve feature"
                        );
                        let session = self.process.as_ref().unwrap().session().clone();
                        self.curve_preflight = CurvePreflight::Negotiated { session, view };
                        Ok(true)
                    }
                    crate::component::curve::CurveReply::Negotiated { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "peer acknowledged a different Curve composition contract".into(),
                        );
                        bail!("peer did not acknowledge independent layer composition");
                    }
                    crate::component::curve::CurveReply::GrantPrepared { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "grant acknowledgement arrived during capability negotiation".into(),
                        );
                        bail!("unexpected grant acknowledgement during negotiation");
                    }
                    crate::component::curve::CurveReply::GrantRevoked { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "revoke acknowledgement arrived during negotiation".into(),
                        );
                        bail!("unexpected revoke acknowledgement during negotiation");
                    }
                    crate::component::curve::CurveReply::ContentProjection { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "content projection arrived during negotiation".into(),
                        );
                        bail!("unexpected content projection during negotiation");
                    }
                    crate::component::curve::CurveReply::ContentInput { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "Content input arrived during negotiation".into(),
                        );
                        bail!("unexpected Content input during negotiation");
                    }
                    crate::component::curve::CurveReply::FramePublished { .. }
                    | crate::component::curve::CurveReply::LayerFramePublished { .. } => {
                        self.curve_preflight = CurvePreflight::Failed(
                            "frame publication arrived during negotiation".into(),
                        );
                        bail!("unexpected frame publication during negotiation");
                    }
                    crate::component::curve::CurveReply::Rejected {
                        transaction_id,
                        code,
                    } => {
                        ensure!(transaction_id == transaction, "curve transaction mismatch");
                        self.curve_preflight = CurvePreflight::Rejected(code);
                        bail!("curve capability rejected: {code:?}");
                    }
                }
            }
            CurvePreflight::Negotiated { session, view } => {
                ensure!(
                    self.process.as_ref().is_some_and(|process| {
                        process.session() == &session
                            && matches!(process.status(), ProcessStatus::Ready { .. })
                    }) && self
                        .native
                        .as_ref()
                        .is_some_and(|native| native.view == view),
                    "curve negotiation owner changed"
                );
                Ok(true)
            }
            CurvePreflight::Rejected(code) => bail!("curve capability rejected: {code:?}"),
            CurvePreflight::Failed(message) => bail!("{message}"),
        }
    }
    fn ensure_process_started(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }
        let wake = self.wake.clone();
        let process = if let Some(native) = &self.native {
            ProcessComponent::spawn(
                ProcessComponentOptions::new(self.module.clone(), self.host_epoch).with_view(
                    native.window,
                    native.view,
                    native.required,
                ),
                move || wake(),
            )?
        } else {
            ProcessComponent::spawn_v1_view(self.module.clone(), self.host_epoch, move || wake())?
        };
        if let (Some(native), Some(context)) = (&self.native, self.view_context.as_ref()) {
            match process.request(crate::component::RequestBody::SetViewContext {
                view: native.view,
                context: context.clone(),
            }) {
                Ok(request) => self
                    .frame_resources
                    .lock()
                    .unwrap()
                    .pending_responses
                    .push_back(request),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        self.process = Some(Arc::new(process));
        self.geometry_dirty = true;
        Ok(())
    }
    /// Host-owned icon images, as for IconButtonView. Names resolve the module's
    /// declarations; the container never unregisters these borrowed resources.
    pub fn set_toolbar_icon_images(&mut self, images: Vec<(String, tekes_ui_scene::ImageId)>) {
        self.toolbar.set_images(images);
        (self.wake)();
    }
    /// Inherit the window environment into the original negotiated View actor.
    /// Reuses the service queue and original geometry/capture cancellation.
    /// Fonts/theme are applied by the module's original backend; unsupported
    /// modules fail explicitly through the existing process status/error path.
    pub fn set_view_context(&mut self, context: crate::component::ViewContext) -> Result<bool> {
        crate::view_context::view_context_palette(&context)?;
        let native = self
            .native
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("View context requires a V2 View"))?;
        let view = native.view;
        ensure!(
            self.view_context
                .as_ref()
                .is_none_or(|old| context.elapsed_nanos >= old.elapsed_nanos),
            "View clock went backwards"
        );
        if self.view_context.as_ref() == Some(&context) {
            return Ok(false);
        }
        let geometry_changed = self
            .view_context
            .as_ref()
            .is_none_or(|old| old.type_scale != context.type_scale);
        if geometry_changed {
            self.cancel_geometry_input();
        }
        if let Some(process) = &self.process {
            let request = process.request(crate::component::RequestBody::SetViewContext {
                view,
                context: context.clone(),
            })?;
            self.frame_resources
                .lock()
                .unwrap()
                .pending_responses
                .push_back(request);
        }
        self.view_context = Some(context);
        if geometry_changed {
            self.geometry_dirty = true;
        }
        self.repaint_dirty = true;
        (self.wake)();
        Ok(true)
    }
    pub fn local_toolbar_frame(&self) -> Option<Rect> {
        self.toolbar.active.then_some(self.toolbar.frame)
    }
    pub fn status(&self) -> Option<ProcessStatus> {
        self.process.as_ref().map(|process| process.status())
    }
    /// Return a weak, resource-only receiver for the pre-negotiated same
    /// process owner. It can import current frames but cannot keep the owner alive.
    #[cfg(target_os = "macos")]
    pub fn frame_resource_handle(&self) -> Result<ProcessContainerFrameHandle> {
        let process = self
            .process
            .as_ref()
            .context("ProcessContainer must be prestarted before taking a frame handle")?;
        let native = self
            .native
            .as_ref()
            .context("frame handle requires a V2 View")?;
        ensure!(
            matches!(process.status(), ProcessStatus::Ready { .. })
                && self.frame_resources.lock().unwrap().view == native.view
                && self.curve_subtree_capability()
                    == tekes_ui_controls::composition::CurveSubtreeCapability::Recursive,
            "frame handle requires current curve negotiation"
        );
        let (request, transaction, proposal) = self
            .curve_grant_request
            .clone()
            .context("current external curve grant is unavailable")?;
        Ok(ProcessContainerFrameHandle {
            process: Arc::downgrade(process),
            resources: Arc::downgrade(&self.frame_resources),
            session: process.session().clone(),
            view: native.view,
            request,
            transaction,
            proposal,
            layer: self
                .frame_resources
                .lock()
                .unwrap()
                .layer_layout
                .clone()
                .context("current layer raster layout unavailable")?,
            frame: self.view_frame,
            clip: self.clip,
        })
    }
    pub fn session(&self) -> Option<&crate::component::Session> {
        self.process.as_ref().map(|process| process.session())
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// Service output stays in the existing ProcessComponent queue. Call on the
    /// supplied wake even when hidden; preparing a frame never discards replies.
    pub fn drain_events(&self) -> Vec<crate::component::ServerMessage> {
        self.process
            .as_ref()
            .map_or_else(Vec::new, |process| process.drain_events())
    }
    /// Consume only the generic response to this owner's exact service request,
    /// leaving Curve publication replies and unrelated events queued.
    pub fn take_response(&self, request: u64) -> Option<Result<serde_json::Value>> {
        self.process.as_ref()?.take_response(request)
    }
    /// Drain ordinary Curve-host events while keeping protocol replies in the
    /// shared queue for their exact request owner.
    #[cfg(target_os = "macos")]
    pub fn drain_curve_host_events(&self) -> Vec<crate::component::ServerMessage> {
        self.process
            .as_ref()
            .map_or_else(Vec::new, |process| process.drain_curve_host_events())
    }
    pub fn toolbar_registration(&self) -> Option<crate::component::ServerMessage> {
        self.process.as_ref()?.toolbar_registration()
    }
    /// Background service work may continue while hidden. UI Invoke obeys the
    /// same availability boundary as command(), and retains explicit generation.
    pub fn request(&self, body: crate::component::RequestBody) -> Result<u64> {
        let origin = self
            .process
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container is unmounted"))?
            .session()
            .clone();
        self.request_from(&origin, body)
    }
    pub fn request_from(
        &self,
        origin: &crate::component::Session,
        body: crate::component::RequestBody,
    ) -> Result<u64> {
        ensure!(
            !matches!(
                body,
                crate::component::RequestBody::Invoke { .. }
                    | crate::component::RequestBody::Content(_)
                    | crate::component::RequestBody::SetViewContext { .. }
            ) || self.visible && self.enabled && self.window_active,
            "container unavailable"
        );
        self.process
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container is unmounted"))?
            .request_from(origin, body)
    }
    pub fn revision(&self) -> Option<u64> {
        self.frame_resources
            .lock()
            .unwrap()
            .image
            .as_ref()
            .map(|image| image.metadata().revision)
    }
    pub fn set_scale(&mut self, scale: f32) -> Result<()> {
        ensure!(
            scale.is_finite() && (0.5..=4.).contains(&scale),
            "invalid backing scale"
        );
        if self.scale != scale {
            self.cancel_geometry_input();
            self.scale = scale;
            self.geometry_dirty = true;
        }
        (self.wake)();
        Ok(())
    }
    pub fn set_opaque_primary_input(&mut self, allow: bool) {
        if self.opaque_pointer != allow {
            self.cancel();
            self.opaque_pointer = allow;
        }
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.cancel();
            self.enabled = enabled;
            self.toolbar.dirty();
            (self.wake)();
        }
    }
    pub fn set_window_active(&mut self, active: bool) {
        if self.window_active != active {
            self.cancel();
            self.window_active = active;
            self.toolbar.dirty();
            (self.wake)();
        }
    }
    pub fn set_visible(&mut self, visible: bool) -> Result<()> {
        if self.visible == visible {
            return Ok(());
        }
        self.cancel();
        if let Some(process) = &self.process
            && let Some(seq) = process.set_view_visible(visible)?
        {
            self.seq = self.seq.max(seq);
        }
        self.visible = visible;
        self.geometry_dirty = true;
        if !visible {
            let mut resources = self.frame_resources.lock().unwrap();
            resources.registration = None;
            resources.image = None;
            resources.visible = false;
        } else {
            self.frame_resources.lock().unwrap().visible = true;
        }
        (self.wake)();
        Ok(())
    }
    fn cancel(&mut self) {
        self.semantic_projection.revoke();
        self.accessibility_focus = false;
        // Lifecycle cancellation must finish in the original Content actor
        // before a later release can be delivered. A render-only Cancel can be
        // pending behind image work even though the pipe already accepted it.
        let typed = self.content_pointer_enabled()
            || self.content_keyboard_enabled()
            || self.content_scroll_enabled()
            || self.content_accessibility_enabled();
        let acknowledged = if typed && self.input_ready() {
            let result = (|| -> Result<_> {
                let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
                // Cancellation revokes the current lifecycle, not a retained
                // input callback. Query its current authority; never replay input.
                let current = self.fresh_content_snapshot(&bridge)?;
                bridge.request(&current, crate::component::content::Input::Cancel)
            })();
            match result {
                Ok(_) => true,
                Err(error) => {
                    self.error = Some(error.to_string());
                    false
                }
            }
        } else {
            false
        };
        self.toolbar.control.cancel();
        self.pointer_focus = None;
        self.pressed = false;
        self.content_snapshot = None;
        self.content_bubbles = None;
        self.content_preserves_focus = false;
        self.geometry_input_pending = false;
        self.pending_content.clear();
        self.pending_content_bytes = 0;
        self.deferred_key_result = None;
        if !acknowledged && let Some(process) = &self.process {
            match process.cancel_view_input() {
                Ok(Some(seq)) => self.seq = self.seq.max(seq),
                Ok(None) => (),
                Err(error) => self.error = Some(error.to_string()),
            }
            // Typed fallback stays unavailable until the ordered Cancel/Layout
            // has produced an acknowledged new allocation. Never race an old Up
            // against a cancellation merely queued behind GPU work.
            if typed {
                self.geometry_dirty = true;
            }
        }
    }
    fn enqueue(&mut self, kind: InputKind, point: Point, command: Option<u64>) -> Result<()> {
        let process = self
            .process
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container is unmounted"))?;
        self.seq = self
            .seq
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("request exhausted"))?;
        if kind == InputKind::Layout {
            self.layout_seq = self.layout_seq.saturating_add(1);
            let mut resources = self.frame_resources.lock().unwrap();
            resources.expected_viewport = self.viewport;
            resources.expected_seq = self.seq;
            resources.expected_size = resources.layer_layout.as_ref().map_or_else(
                || {
                    (
                        ((self.view_frame.size.width as u32 as f32) * self.scale).round() as u32,
                        ((self.view_frame.size.height as u32 as f32) * self.scale).round() as u32,
                    )
                },
                |layer| layer.pixel_size(),
            );
            resources.expected_scale = self.scale;
            resources.visible = self.visible;
            resources.allocation_seq = self.allocation_seq;
            resources.layout_seq = self.layout_seq;
        }
        process.enqueue_view(Request {
            version: VERSION,
            seq: self.seq,
            viewport: self.viewport,
            kind,
            command,
            x: point.x - self.view_frame.min_x(),
            y: point.y - self.view_frame.min_y(),
            width: self.view_frame.size.width as u32,
            height: self.view_frame.size.height as u32,
            identity: 0,
            scale: self.scale,
        })
    }
    pub fn input_ready(&self) -> bool {
        self.geometry_ready() && !self.geometry_input_pending
    }
    fn geometry_ready(&self) -> bool {
        let resources = self.frame_resources.lock().unwrap();
        let frame_current = resources.image.as_ref().is_some_and(|image| {
            if self.curve_geometry.is_none() {
                image.metadata().viewport == self.viewport
            } else {
                resources.grant_acknowledged
                    && !resources.deferred_curve_grant
                    && image.curve_stamp().is_some_and(|stamp| {
                        stamp.grant_serial == resources.grant_serial
                            && stamp.gp == resources.parent_generation
                            && stamp.gc == resources.child_generation
                            && stamp.view == resources.view
                            && stamp.viewport == resources.expected_viewport
                            && stamp.allocation_seq == resources.allocation_seq
                            && stamp.layout_seq == resources.layout_seq
                            && stamp.frame_seq == image.metadata().seq
                    })
                    && image.curve_stamp().is_some_and(|stamp| {
                        self.content_snapshot
                            .as_ref()
                            .and_then(crate::ProcessContentSnapshot::curve_stamp)
                            == Some(&curve_query_stamp(stamp))
                    })
            }
        });
        !self.geometry_dirty
            && !self.repaint_dirty
            && !self
                .process
                .as_ref()
                .is_some_and(|process| process.has_pending_curve_host_updates())
            && self.visible
            && self.enabled
            && self.window_active
            && self
                .process
                .as_ref()
                .is_some_and(|p| matches!(p.status(), ProcessStatus::Ready { .. }))
            && frame_current
    }
    /// Original child text/query adapter for this exact acknowledged allocation.
    /// Old snapshots cannot be used after resize, move, hide or restart. This
    /// never opts the whole component into an opaque pointer sink.
    pub fn native_content_bridge(
        &self,
        timeout: std::time::Duration,
    ) -> Result<crate::ProcessContentBridge<'_>> {
        ensure!(self.input_ready(), "container Content geometry unavailable");
        self.geometry_content_bridge(timeout)
    }
    fn geometry_content_bridge(
        &self,
        timeout: std::time::Duration,
    ) -> Result<crate::ProcessContentBridge<'_>> {
        ensure!(self.geometry_ready(), "container geometry not acknowledged");
        crate::ProcessContentBridge::new_with_curve(
            self.process
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("container unmounted"))?,
            self.native
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("legacy Content unavailable"))?,
            self.viewport,
            self.view_frame,
            self.clip,
            self.content_snapshot
                .as_ref()
                .and_then(crate::ProcessContentSnapshot::curve_stamp)
                .cloned(),
            timeout,
        )
    }
    /// Attach received GPU resources to the host Renderer before Content paint.
    /// This handles all selection/import/replacement/retirement, not the consumer.
    pub fn prepare(&mut self, renderer: &mut Renderer) -> Result<bool> {
        ensure!(
            self.curve_preflight == CurvePreflight::Unrequested,
            "preflighted curve frames require the restricted frame-resource handle"
        );
        let was_local = self.toolbar.active;
        let change = if let Some(process) = &self.process {
            self.toolbar
                .sync(process, self.enabled && self.window_active)
        } else {
            self.toolbar.reset();
            Ok(false)
        };
        let changed = match change {
            Ok(changed) => changed,
            Err(error) => {
                self.error = Some(error.to_string());
                return Err(error);
            }
        };
        if changed {
            if was_local != self.toolbar.active {
                self.cancel_geometry_input();
                self.geometry_dirty = true;
            }
            self.error = None;
            (self.wake)();
        }
        let Some(process) = &self.process else {
            renderer.collect_retired_images();
            return Ok(false);
        };
        if let ProcessStatus::Failed { message } = process.status() {
            self.semantic_projection.revoke();
            self.error = Some(message.clone());
            anyhow::bail!("{message}");
        }
        let invalidated = process.take_view_invalidated();
        if invalidated {
            self.repaint_dirty = true;
            (self.wake)();
        }
        let old = self
            .frame_resources
            .lock()
            .unwrap()
            .image
            .as_ref()
            .map(|i| (i.metadata().viewport, i.metadata().seq))
            .unwrap_or((0, 0));
        let mut selected = None;
        for frame in process.drain_view_frames() {
            let meta = frame.metadata();
            if !self.visible
                || meta.viewport > self.viewport
                || meta.viewport < old.0
                || meta.seq <= old.1
            {
                continue;
            }
            selected = Some(frame);
        }
        let Some(frame) = selected else {
            // Business updates may open/close original overlays before a new
            // image is presented. Publish input authority on the same wake.
            if (invalidated || self.content_accessibility_enabled()) && self.input_ready() {
                self.refresh_content_scope()?;
            }
            renderer.collect_retired_images();
            return Ok(false);
        };
        let image = frame.import(&renderer.gpu_context())?;
        let mut resources = self.frame_resources.lock().unwrap();
        if let Some(registration) = &resources.registration {
            renderer.replace_owned_gpu_image(registration, image.image())?;
        } else {
            resources.registration = Some(renderer.register_owned_gpu_image(image.image())?);
        }
        resources.image = Some(image);
        drop(resources);
        if self.geometry_input_pending && self.geometry_ready() {
            self.acknowledge_geometry_cancel()?;
        }
        if self.input_ready() {
            self.flush_pending_content()?;
            self.refresh_content_scope()?;
        }
        (self.wake)();
        Ok(true)
    }
    /// Apply only the original non-frame service updates for a preflighted
    /// curve child. The Host must then perform its ordinary render/layout; this
    /// method never imports a frame or mutates the current display resource.
    #[cfg(target_os = "macos")]
    pub fn prepare_curve_host_update(&mut self) -> Result<bool> {
        ensure!(
            self.curve_preflight != CurvePreflight::Unrequested,
            "curve update requires preflight"
        );
        let was_local = self.toolbar.active;
        let changed = if let Some(process) = &self.process {
            self.toolbar
                .sync(process, self.enabled && self.window_active)?
        } else {
            false
        };
        if changed {
            if was_local != self.toolbar.active {
                self.cancel_geometry_input();
                self.geometry_dirty = true;
            }
            self.error = None;
        }
        let invalidated = self
            .process
            .as_ref()
            .is_some_and(|process| process.take_view_invalidated());
        if invalidated {
            self.repaint_dirty = true;
            (self.wake)();
        }
        Ok(changed || invalidated)
    }
    #[cfg(target_os = "macos")]
    pub fn curve_requires_host_update(&self) -> bool {
        self.process
            .as_ref()
            .is_some_and(|process| process.has_pending_curve_host_updates())
    }
    /// Commit one fully prepared frame and original Host projection together
    /// at the existing paint point. A rejected projection leaves the currently
    /// displayed resource and semantic publication untouched.
    fn commit_pending_curve_publication(&mut self) {
        let shared = self.frame_resources.clone();
        let publication = shared.lock().unwrap().pending_curve_publication.take();
        let Some(publication) = publication else {
            return;
        };
        let Some(process) = self.process.as_ref() else {
            self.error = Some("curve publication owner disappeared".into());
            self.geometry_dirty = true;
            return;
        };
        let current = {
            let resources = shared.lock().unwrap();
            matches!(process.status(), ProcessStatus::Ready { .. })
                && resources.visible
                && resources.grant_acknowledged
                && resources.grant_serial == publication.stamp.grant_serial
                && resources.parent_generation == publication.stamp.gp
                && resources.child_generation == publication.stamp.gc
                && resources.expected_viewport == publication.stamp.viewport
                && resources.allocation_seq == publication.stamp.allocation_seq
                && resources.layout_seq == publication.stamp.layout_seq
                && resources.expected_seq <= publication.stamp.frame_seq
                && resources.pending_revokes.is_empty()
                && self
                    .curve_geometry
                    .as_ref()
                    .is_some_and(|context| context.generation() == publication.stamp.gp)
                && publication.image.layer_layout() == resources.layer_layout.as_ref()
                && publication.image.curve_stamp() == Some(&publication.stamp)
                && publication.query_stamp == curve_query_stamp(&publication.stamp)
                && publication.snapshot.curve_stamp() == Some(&publication.query_stamp)
                && publication
                    .snapshot
                    .belongs_to(process.session(), publication.stamp.view)
                && !process.has_pending_curve_host_updates()
                && (!self.geometry_input_pending || resources.geometry_cancel_confirmed)
        };
        if !current {
            trace_layer(format_args!(
                "event=publication_rejected stamp={:?} geometry_dirty={} repaint_dirty={} cancel_pending={}",
                publication.stamp,
                self.geometry_dirty,
                self.repaint_dirty,
                self.geometry_input_pending
            ));
            self.error = Some("stale or incomplete curve publication".into());
            self.geometry_dirty = true;
            return;
        }
        let mut semantics = self.semantic_projection.clone();
        if let Err(error) = semantics.update(publication.snapshot.clone()) {
            self.error = Some(error.to_string());
            self.geometry_dirty = true;
            return;
        }
        trace_layer(format_args!(
            "event=publication_commit stamp={:?} capture={:?} queued_callbacks={}",
            publication.stamp,
            publication.snapshot.reply().captured,
            self.pending_content.len()
        ));
        self.semantic_projection = semantics;
        self.content_snapshot = Some(publication.snapshot);
        self.geometry_input_pending = false;
        self.geometry_dirty = false;
        self.repaint_dirty = false;
        self.error = None;
        let mut resources = shared.lock().unwrap();
        resources.registration = Some(publication.registration);
        resources.image = Some(publication.image);
        resources.geometry_cancel_confirmed = false;
        drop(resources);
        if !self.pending_content.is_empty()
            && let Err(error) = self.flush_pending_content()
        {
            self.error = Some(error.to_string());
        }
    }
    pub fn command(&mut self, command: u64) -> Result<bool> {
        if !self.visible || !self.enabled || !self.window_active {
            return Ok(false);
        }
        if self.native.is_some() {
            let Some(process) = &self.process else {
                return Ok(false);
            };
            // The shared service owner validates registration generation and
            // enabled state. No frame or renderer is required for toolbar RPC.
            process.invoke(command)?;
            return Ok(true);
        }
        let input_ready = self.input_ready();
        let resources = self.frame_resources.lock().unwrap();
        let Some(image) = &resources.image else {
            return Ok(false);
        };
        let Some(item) = image
            .metadata()
            .toolbar
            .iter()
            .find(|i| i.id == command && i.enabled)
        else {
            return Ok(false);
        };
        if item.geometry_dependent && !input_ready {
            return Ok(false);
        }
        drop(resources);
        self.enqueue(
            InputKind::Command,
            Point::new(self.frame.min_x(), self.frame.min_y()),
            Some(command),
        )?;
        Ok(true)
    }
    fn available(&self) -> bool {
        self.visible
            && self.enabled
            && self.window_active
            && self
                .process
                .as_ref()
                .is_some_and(|process| matches!(process.status(), ProcessStatus::Ready { .. }))
    }
    fn finish_toolbar(&mut self, mut response: ContentResponse) -> ContentResponse {
        if response.activated {
            let result = self.toolbar.take_request().and_then(|request| {
                if let Some((owner, body)) = request {
                    self.request_from(&owner, body).map(|_| ())
                } else {
                    Ok(())
                }
            });
            if let Err(error) = result {
                self.error = Some(error.to_string());
            }
            // The module receives exactly one Invoke. The containing Host must
            // not also synthesize a second business action from this group.
            response.activated = false;
        }
        response
    }
    fn pointer_value(event: &ContentEvent) -> Option<PointerEvent> {
        if let ContentEvent::Pointer(pointer) = event {
            return Some(*pointer);
        }
        let (position, phase) = match event {
            ContentEvent::PointerDown(point) => (*point, PointerPhase::Down),
            ContentEvent::PointerMove(point) => (*point, PointerPhase::Move),
            ContentEvent::PointerUp(point) => (*point, PointerPhase::Up),
            _ => return None,
        };
        Some(PointerEvent {
            position,
            phase,
            button: Some(PointerButton::Primary),
            modifiers: Default::default(),
            click_count: 1,
        })
    }
    fn remote_overlay(&self) -> bool {
        self.available()
            && self.content_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.reply().overlay.is_some()
                    && self.process.as_ref().zip(self.native.as_ref()).is_some_and(
                        |(process, native)| snapshot.belongs_to(process.session(), native.view),
                    )
            })
    }
    fn refresh_content_scope(&mut self) -> Result<()> {
        if self.curve_geometry.is_some() {
            ensure!(
                self.content_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.curve_stamp().is_some()),
                "current verified Curve projection unavailable"
            );
            return Ok(());
        }
        if !self.content_pointer_enabled()
            && !self.content_keyboard_enabled()
            && !self.content_scroll_enabled()
            && !self.content_accessibility_enabled()
        {
            return Ok(());
        }
        let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
        let result = if self.content_accessibility_enabled() {
            bridge.semantics()
        } else {
            bridge.snapshot()
        };
        match result {
            Ok(snapshot) => {
                self.accept_content(snapshot);
            }
            // Keep the last authoritative barrier while existing layout/frame
            // acknowledgement completes. Never replay or retarget input here.
            Err(error)
                if error.root_cause().to_string()
                    == tekes_ui_app_protocol::component::content::GEOMETRY_PENDING =>
            {
                self.semantic_projection.dirty();
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
    fn fresh_content_snapshot(
        &self,
        bridge: &crate::ProcessContentBridge<'_>,
    ) -> Result<crate::ProcessContentSnapshot> {
        if self.curve_geometry.is_some() {
            ensure!(
                self.input_ready(),
                "verified Curve input projection unavailable"
            );
            self.content_snapshot
                .clone()
                .context("verified Curve content projection missing")
        } else {
            bridge.snapshot()
        }
    }
    fn query_content(&self) -> Option<crate::ProcessContentSnapshot> {
        if !(self.content_focused || self.remote_overlay()) || !self.available() {
            return None;
        }
        let current = if self.curve_geometry.is_some() {
            // Read-only logical text and binding outlive a frame regrant.
            // text_input_snapshot separately withdraws stale caret geometry;
            // mutable callbacks still use exact current admission or the
            // existing same-owner/binding deferred-input validation.
            self.content_snapshot.clone()?
        } else if self.input_ready() {
            self.native_content_bridge(std::time::Duration::from_secs(1))
                .ok()?
                .snapshot()
                .ok()?
        } else {
            self.content_snapshot.clone()?
        };
        if !current.belongs_to(self.process.as_ref()?.session(), self.native.as_ref()?.view) {
            return None;
        }
        Some(current)
    }
    fn content_capabilities(&self) -> Option<crate::component::ViewCapabilities> {
        self.process.as_ref()?.view_capabilities()
    }
    fn content_pointer_enabled(&self) -> bool {
        self.content_capabilities()
            .is_some_and(|c| c.content_pointer)
    }
    fn content_keyboard_enabled(&self) -> bool {
        self.content_capabilities().is_some_and(|c| c.keyboard)
    }
    fn content_accessibility_enabled(&self) -> bool {
        self.content_capabilities().is_some_and(|c| c.accessibility)
    }
    fn semantic_ready(&self) -> bool {
        self.input_ready()
            && self.content_accessibility_enabled()
            && !self.semantic_projection.needs_refresh()
    }
    fn content_scroll_enabled(&self) -> bool {
        self.content_capabilities().is_some_and(|c| c.scroll)
    }
    fn content_captured(&self) -> bool {
        self.content_snapshot
            .as_ref()
            .is_some_and(|s| s.reply().captured.is_some())
    }
    fn accept_content(&mut self, next: crate::ProcessContentSnapshot) -> ContentResponse {
        self.error = None;
        let reply = next.reply();
        self.content_reading_scroll = reply.consumed && reply.reading_scroll_source.is_some();
        self.content_bubbles = Some(reply.bubbles);
        self.content_preserves_focus = !reply.pointer_focus_requested;
        let response = ContentResponse {
            consumed: reply.consumed,
            changed: reply.changed,
            capture_pointer: reply.captured.is_some(),
            // Child actions remain in the original child. Never emit a second
            // parent business action for the same activation.
            activated: false,
            text_key: reply
                .key
                .as_ref()
                .map(|k| tekes_ui_controls::composition::TextKeyOutcome {
                    id: k.id,
                    target: None,
                    disposition: match k.disposition {
                        crate::component::content::KeyDisposition::Deferred => {
                            tekes_ui_controls::composition::TextKeyDisposition::Deferred
                        }
                        crate::component::content::KeyDisposition::InputConsumed => {
                            tekes_ui_controls::composition::TextKeyDisposition::InputConsumed
                        }
                        crate::component::content::KeyDisposition::EditHandled => {
                            tekes_ui_controls::composition::TextKeyDisposition::EditHandled
                        }
                        crate::component::content::KeyDisposition::Submitted => {
                            tekes_ui_controls::composition::TextKeyDisposition::Submitted
                        }
                        crate::component::content::KeyDisposition::Navigated => {
                            tekes_ui_controls::composition::TextKeyDisposition::Navigated
                        }
                        crate::component::content::KeyDisposition::Unhandled => {
                            tekes_ui_controls::composition::TextKeyDisposition::Unhandled
                        }
                        crate::component::content::KeyDisposition::Cancelled => {
                            tekes_ui_controls::composition::TextKeyDisposition::Cancelled
                        }
                    },
                }),
        };
        if reply.semantics.is_some() {
            if let Err(error) = self.semantic_projection.update(next.clone()) {
                self.error = Some(error.to_string());
            }
        } else {
            self.semantic_projection.dirty();
        }
        if response.changed
            || self
                .content_snapshot
                .as_ref()
                .and_then(|old| old.reply().captured)
                != reply.captured
        {
            trace_layer(format_args!(
                "event=content_reply stamp={:?} changed={} activated={} capture={:?} owner={:?} focus_epoch={}",
                next.curve_stamp(),
                reply.changed,
                reply.activated,
                reply.captured,
                reply.focused,
                reply.focus_epoch
            ));
        }
        self.content_snapshot = Some(next);
        if response.changed && self.curve_geometry.is_some() {
            // Withdraw input immediately, before the asynchronous invalidation
            // packet arrives. Established capture continuations may queue once.
            self.repaint_dirty = true;
        }
        if response.changed {
            (self.wake)();
        }
        response
    }
    fn content_pointer(&mut self, pointer: &PointerEvent) -> ContentResponse {
        let result = (|| -> Result<_> {
            let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
            let previous = match &self.content_snapshot {
                Some(snapshot) => bridge.refresh_allocation(snapshot)?,
                None => self.fresh_content_snapshot(&bridge)?,
            };
            bridge.pointer(&previous, pointer)
        })();
        match result {
            Ok(next) => {
                if pointer.phase == PointerPhase::Down && next.reply().pointer_focus_requested {
                    self.pointer_focus = Some(false);
                }
                self.accept_content(next)
            }
            Err(error) => self.content_failure(error),
        }
    }
    fn cancel_geometry_input(&mut self) -> ContentResponse {
        self.semantic_projection.dirty();
        self.accessibility_focus = false;
        self.content_reading_scroll = false;
        if !self.content_pointer_enabled()
            && !self.content_keyboard_enabled()
            && !self.content_scroll_enabled()
            && !self.content_accessibility_enabled()
        {
            self.cancel();
            return ContentResponse::default();
        }
        self.toolbar.control.cancel();
        self.pointer_focus = None;
        self.pressed = false;
        self.geometry_input_pending = true;
        if self.geometry_ready()
            && let Err(error) = self.acknowledge_geometry_cancel()
        {
            self.error = Some(error.to_string());
            self.geometry_dirty = true;
        }
        self.content_bubbles = Some(false);
        ContentResponse::consumed(true)
    }
    fn acknowledge_geometry_cancel(&mut self) -> Result<()> {
        let next = {
            let bridge = self.geometry_content_bridge(std::time::Duration::from_secs(1))?;
            let current = self.fresh_content_snapshot(&bridge)?;
            bridge.request(
                &current,
                crate::component::content::Input::CancelPointerCapture,
            )?
        };
        self.accept_content(next);
        self.geometry_input_pending = false;
        Ok(())
    }
    fn queue_content(&mut self, input: crate::component::content::Input) -> ContentResponse {
        self.content_bubbles = Some(false);
        let Some(snapshot) = &self.content_snapshot else {
            return ContentResponse::default();
        };
        let Some(process) = &self.process else {
            return ContentResponse::default();
        };
        let original_key_binding =
            matches!(input, crate::component::content::Input::TextKeyEnd { .. })
                .then(|| snapshot.native_binding())
                .flatten();
        // Include retained original identity bytes in the existing pipe budget.
        let size = serde_json::to_vec(&input).map(|v| {
            v.len()
                .saturating_add(std::mem::size_of::<PendingContentInput>())
                .saturating_add(snapshot.reply().text_binding.as_ref().map_or(0, Vec::len))
                .saturating_add(original_key_binding.as_ref().map_or(0, Vec::len))
                .saturating_add(process.session().module.len())
        });
        let finishing = if let crate::component::content::Input::TextKeyEnd { id, .. } = &input {
            Some(*id)
        } else {
            None
        };
        let bytes = match size {
            Ok(bytes)
                if self.pending_content_bytes.saturating_add(bytes) <= crate::pipes::MAX_PACKET =>
            {
                bytes
            }
            rejected => {
                // Exhaustion must revoke the original span, not silently drop
                // its End and let an old frame consume a later editor callback.
                self.cancel();
                self.error = Some(
                    if rejected.is_err() {
                        "invalid deferred Content input"
                    } else {
                        "deferred Content input exceeds transport budget"
                    }
                    .into(),
                );
                let mut response = ContentResponse::consumed(false);
                response.text_key =
                    finishing.map(|id| tekes_ui_controls::composition::TextKeyOutcome {
                        id,
                        target: None,
                        disposition: tekes_ui_controls::composition::TextKeyDisposition::Cancelled,
                    });
                return response;
            }
        };
        trace_layer(format_args!(
            "event=input_queue view={} pointer_phase={:?} capture={:?} stamp={:?} queue_len={}",
            snapshot.reply().view,
            match &input {
                crate::component::content::Input::Pointer(pointer) => Some(pointer.phase),
                _ => None,
            },
            snapshot.reply().captured,
            snapshot.curve_stamp(),
            self.pending_content.len()
        ));
        self.pending_content.push_back(PendingContentInput {
            session: process.session().clone(),
            view: snapshot.reply().view,
            owner: snapshot.reply().focused,
            focus_epoch: snapshot.reply().focus_epoch,
            text_binding: snapshot.reply().text_binding.clone(),
            capture: snapshot.reply().captured,
            original_key_binding,
            input,
            bytes,
        });
        self.pending_content_bytes += bytes;
        let mut response = ContentResponse::consumed(false);
        response.text_key = finishing.map(|id| tekes_ui_controls::composition::TextKeyOutcome {
            id,
            target: None,
            disposition: tekes_ui_controls::composition::TextKeyDisposition::Deferred,
        });
        response
    }
    fn flush_pending_content(&mut self) -> Result<()> {
        let mut failure = None;
        while let Some(pending) = self.pending_content.pop_front() {
            self.pending_content_bytes -= pending.bytes;
            // Rejected edits must never migrate to a fresh owner. Original
            // key-span finishes still need to reach that same session for cleanup.
            if failure.is_some()
                && !matches!(
                    pending.input,
                    crate::component::content::Input::TextKeyEnd { .. }
                )
            {
                continue;
            }
            let finishing_id =
                if let crate::component::content::Input::TextKeyEnd { id, .. } = &pending.input {
                    Some(*id)
                } else {
                    None
                };
            let result = (|| -> Result<_> {
                let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
                let finishing = matches!(
                    pending.input,
                    crate::component::content::Input::TextKeyEnd { .. }
                );
                let current = if finishing {
                    self.fresh_content_snapshot(&bridge)?
                } else {
                    match &self.content_snapshot {
                        Some(snapshot) => bridge.refresh_allocation(snapshot)?,
                        None => self.fresh_content_snapshot(&bridge)?,
                    }
                };
                trace_layer(format_args!(
                    "event=deferred_delivery view={} old_capture={:?} current_capture={:?} old_owner={:?} current_owner={:?} stamp={:?}",
                    pending.view,
                    pending.capture,
                    current.reply().captured,
                    pending.owner,
                    current.reply().focused,
                    current.curve_stamp()
                ));
                ensure!(
                    self.process
                        .as_ref()
                        .is_some_and(|p| p.session() == &pending.session)
                        && current.reply().view == pending.view
                        && (finishing
                            || if matches!(
                                pending.input,
                                crate::component::content::Input::Pointer(_)
                            ) {
                                pending.capture.is_some()
                                    && current.reply().captured == pending.capture
                            } else {
                                current.reply().focused == pending.owner
                                    && current.reply().focus_epoch == pending.focus_epoch
                                    && current.reply().text_binding == pending.text_binding
                            }),
                    "stale deferred Content callback"
                );
                bridge.request(&current, pending.input)
            })();
            match result {
                Ok(next) => {
                    let mut response = self.accept_content(next);
                    let needs_publication = self.curve_geometry.is_some() && response.changed;
                    if let Some(id) = finishing_id {
                        response.text_key.get_or_insert(
                            tekes_ui_controls::composition::TextKeyOutcome {
                                id,
                                target: None,
                                disposition:
                                    tekes_ui_controls::composition::TextKeyDisposition::Cancelled,
                            },
                        );
                        self.deferred_key_result = Some((
                            response,
                            self.content_bubbles == Some(true),
                            pending.original_key_binding,
                        ));
                    }
                    if needs_publication {
                        break;
                    }
                }
                Err(error) => {
                    trace_layer(format_args!(
                        "event=deferred_error view={} capture={:?} error={error:#}",
                        pending.view, pending.capture
                    ));
                    // Observe the current owner for future fresh intent, without
                    // applying any of the rejected old callbacks to that owner.
                    if let Ok(bridge) =
                        self.native_content_bridge(std::time::Duration::from_secs(1))
                        && let Ok(current) = self.fresh_content_snapshot(&bridge)
                    {
                        self.accept_content(current);
                    }
                    if let Some(id) = finishing_id {
                        let mut response = ContentResponse::consumed(false);
                        response.text_key = Some(tekes_ui_controls::composition::TextKeyOutcome {
                            id,
                            target: None,
                            disposition:
                                tekes_ui_controls::composition::TextKeyDisposition::Cancelled,
                        });
                        self.deferred_key_result =
                            Some((response, false, pending.original_key_binding));
                    }
                    failure.get_or_insert(error);
                }
            }
        }
        if let Some(error) = failure {
            self.error = Some(error.to_string());
            self.content_bubbles = Some(false);
            return Err(error);
        }
        Ok(())
    }
    fn content_input(&mut self, input: crate::component::content::Input) -> ContentResponse {
        if !self.input_ready() {
            if self.available()
                && (self.content_focused
                    || matches!(input, crate::component::content::Input::TextKeyEnd { .. }))
            {
                return self.queue_content(input);
            }
            self.content_bubbles = Some(false);
            return ContentResponse::default();
        }
        if let Err(error) = self.flush_pending_content() {
            self.error = Some(error.to_string());
            return ContentResponse::default();
        }
        self.deliver_content_input(input)
    }
    fn deliver_content_input(
        &mut self,
        input: crate::component::content::Input,
    ) -> ContentResponse {
        let result = (|| -> Result<_> {
            let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
            // Started is fresh native intent, not a retained editing callback.
            // Capture the current original child lease; subsequent callbacks
            // remain fenced by the parent frame and this retained snapshot.
            let previous = if matches!(
                input,
                crate::component::content::Input::TextKeyEnd { .. }
                    | crate::component::content::Input::TextKeyStart { .. }
            ) {
                self.fresh_content_snapshot(&bridge)?
            } else {
                match &self.content_snapshot {
                    Some(snapshot) => bridge.refresh_allocation(snapshot)?,
                    None => self.fresh_content_snapshot(&bridge)?,
                }
            };
            match input {
                crate::component::content::Input::Scroll(scroll) => {
                    bridge.scroll(&previous, scroll)
                }
                input => bridge.request(&previous, input),
            }
        })();
        match result {
            Ok(next) => self.accept_content(next),
            Err(error) => self.content_failure(error),
        }
    }
    fn content_failure(&mut self, error: anyhow::Error) -> ContentResponse {
        // A failed delivery is cancelled, never resnapshotted and replayed.
        self.cancel_geometry_input();
        self.error = Some(error.to_string());
        self.content_bubbles = Some(false);
        ContentResponse::default()
    }
    fn focus_content(&mut self, focused: bool, visible: bool) {
        let result = (|| -> Result<_> {
            let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
            // This is a new explicit focus/blur intent, not a retained native
            // callback. Layout may have applied the original overlay's pending
            // focus since the cached reply; capture its current authority once.
            let previous = self.fresh_content_snapshot(&bridge)?;
            let target = focused
                .then(|| {
                    previous
                        .reply()
                        .focused
                        .or_else(|| previous.reply().focus_targets.first().copied())
                })
                .flatten();
            // Opening/layout already applies the original child's own overlay
            // focus intent. Do not send a redundant Focus across that pending
            // layout, or turn an empty pending scope into an explicit blur.
            if focused && previous.reply().overlay.is_some() || previous.reply().focused == target {
                return Ok(previous);
            }
            bridge.focus(&previous, target, visible)
        })();
        match result {
            Ok(next) => {
                self.accept_content(next);
                if self.content_accessibility_enabled()
                    && self.input_ready()
                    && let Err(error) = self.refresh_content_scope()
                {
                    self.error = Some(error.to_string());
                }
            }
            Err(error) => {
                self.content_failure(error);
            }
        }
    }
    fn enter_content(&mut self, reverse: bool) -> bool {
        if !self.content_keyboard_enabled() || !self.input_ready() {
            return false;
        }
        self.content_input(crate::component::content::Input::EnterFocus { reverse });
        let focused = self
            .content_snapshot
            .as_ref()
            .is_some_and(|s| s.reply().focused.is_some());
        if focused {
            self.toolbar.control.set_focus(false, false);
            self.toolbar_focused = false;
            self.content_focused = true;
        } else if self.content_snapshot.is_some() {
            // An empty original focus scope is not a traversal barrier. Failed
            // transport still retains content_failure's cancellation fence.
            self.content_bubbles = None;
        }
        focused
    }
    fn enter_toolbar(&mut self, reverse: bool) -> bool {
        if !self.toolbar.active || !self.toolbar.control.focusable() {
            return false;
        }
        if self.content_focused {
            self.focus_content(false, false);
        }
        self.content_focused = false;
        self.toolbar_focused = true;
        self.toolbar.control.set_focus_from_traversal(reverse);
        true
    }
    fn send_event(&mut self, kind: InputKind, point: Point, capture: bool) -> ContentResponse {
        match self.enqueue(kind, point, None) {
            Ok(()) => ContentResponse {
                consumed: true,
                changed: false,
                capture_pointer: capture,
                ..Default::default()
            },
            Err(error) => {
                self.error = Some(error.to_string());
                self.pressed = false;
                ContentResponse::default()
            }
        }
    }
}
impl ProcessContainer {
    fn begin_curve_grant(
        &mut self,
        context: tekes_ui_controls::composition::CurveGeometryContext,
    ) -> Result<()> {
        let (session, view) = match &self.curve_preflight {
            CurvePreflight::Negotiated { session, view } => (session.clone(), *view),
            _ => bail!("curve geometry arrived before current negotiation"),
        };
        ensure!(
            self.process.as_ref().is_some_and(|process| {
                process.session() == &session
                    && matches!(process.status(), ProcessStatus::Ready { .. })
            }) && self
                .native
                .as_ref()
                .is_some_and(|native| native.view == view),
            "curve grant owner changed"
        );
        let transaction = self.next_curve_transaction()?;
        let (proposal, layer) = self.grant_proposal(transaction, &context)?;
        let envelope = crate::component::curve::Envelope {
            protocol_major: 1,
            wire_revision: 1,
            view,
            transaction_id: transaction,
            ttl_ms: 1000,
            body: crate::component::curve::RequestBodyV1::PrepareLayerGrant {
                proposal: Box::new(crate::component::curve::LayerGrantProposal {
                    geometry: Box::new(proposal.clone()),
                    logical_size: layer.logical_size(),
                }),
            },
        };
        let request = self
            .process
            .as_ref()
            .expect("negotiated process exists")
            .request(crate::component::RequestBody::CurveV1(envelope))?;
        trace_layer(format_args!(
            "event=grant_send view={} request={} proposal={} gp={} viewport={} allocation={} layout={} capture={:?}",
            view,
            request,
            proposal.proposal_id,
            proposal.parent_geometry_generation,
            proposal.viewport,
            proposal.allocation_seq,
            proposal.layout_seq,
            self.content_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.reply().captured)
        ));
        self.curve_grant_request = Some((request, transaction, proposal.clone()));
        {
            let mut resources = self.frame_resources.lock().unwrap();
            resources.parent_generation = proposal.parent_geometry_generation;
            resources.grant_serial = proposal.proposal_id;
            resources.child_generation = 0;
            resources.allocation_seq = proposal.allocation_seq;
            resources.layout_seq = proposal.layout_seq;
            resources.grant_acknowledged = false;
            resources.deferred_curve_grant = false;
            resources.layer_layout = Some(layer);
        }
        self.geometry_dirty = false;
        self.repaint_dirty = false;
        self.enqueue(InputKind::Layout, self.view_frame.origin, None)
    }
}
impl Component for ProcessContainer {
    fn curve_subtree_capability(&self) -> tekes_ui_controls::composition::CurveSubtreeCapability {
        use tekes_ui_controls::composition::CurveSubtreeCapability;
        let negotiated = match &self.curve_preflight {
            CurvePreflight::Negotiated { session, view } => {
                self.process.as_ref().is_some_and(|process| {
                    process.session() == session
                        && matches!(process.status(), ProcessStatus::Ready { .. })
                }) && self
                    .native
                    .as_ref()
                    .is_some_and(|native| native.view == *view)
            }
            _ => false,
        };
        if negotiated {
            CurveSubtreeCapability::Recursive
        } else {
            CurveSubtreeCapability::Unsupported
        }
    }
    fn set_curve_geometry_context(
        &mut self,
        context: Option<tekes_ui_controls::composition::CurveGeometryContext>,
    ) {
        let (old_grant, independent_layer, awaiting_grant, deferred_ready) = {
            let resources = self.frame_resources.lock().unwrap();
            (
                resources.grant_serial,
                resources.layer_layout.is_some(),
                self.curve_grant_request.is_some() && !resources.grant_acknowledged,
                resources.deferred_curve_grant && resources.grant_acknowledged,
            )
        };
        if deferred_ready
            || self
                .curve_geometry
                .as_ref()
                .map(|v| (v.generation(), v.shapes()))
                != context.as_ref().map(|v| (v.generation(), v.shapes()))
        {
            trace_layer(format_args!(
                "event=context view={} grant={} old_gp={:?} next_gp={:?} same_shapes={} awaiting={} deferred_ready={} geometry_dirty={} repaint_dirty={} cancel_pending={} capture={:?}",
                self.native.as_ref().map_or(0, |native| native.view),
                old_grant,
                self.curve_geometry.as_ref().map(|value| value.generation()),
                context.as_ref().map(|value| value.generation()),
                self.curve_geometry
                    .as_ref()
                    .zip(context.as_ref())
                    .is_some_and(|(old, new)| old.shapes() == new.shapes()),
                awaiting_grant,
                deferred_ready,
                self.geometry_dirty,
                self.repaint_dirty,
                self.geometry_input_pending,
                self.content_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.reply().captured)
            ));
            if independent_layer && awaiting_grant && context.is_some() {
                // One Prepare/Layout remains outstanding until its exact
                // GrantPrepared is consumed. Coalesce only unsent geometry
                // intent, without reserving an outer request identity.
                // Waiting for cancellation is an admission fence, not another
                // spatial change. Keep that fence until the current frame commits.
                let same_space = !self.geometry_dirty
                    && self
                        .curve_geometry
                        .as_ref()
                        .zip(context.as_ref())
                        .is_some_and(|(old, new)| old.shapes() == new.shapes());
                self.curve_geometry = context;
                self.repaint_dirty = true;
                if !same_space {
                    self.geometry_dirty = true;
                    self.geometry_input_pending = true;
                }
                let mut resources = self.frame_resources.lock().unwrap();
                resources.deferred_curve_grant = true;
                if let Some(pending) = resources.pending_curve_frame.take() {
                    resources.retired_curve_query = Some(pending.query_request);
                }
                resources.pending_curve_publication = None;
                drop(resources);
                (self.wake)();
                return;
            }
            // A sibling update can also advance the parent Host clock. An
            // unchanged typed spatial layer still withdraws old frame authority,
            // but must retain its original gesture regardless of which child
            // requested repaint. Real geometry/cancellation changes still revoke.
            let same_spatial_layer = independent_layer
                && !self.geometry_dirty
                && self
                    .curve_geometry
                    .as_ref()
                    .zip(context.as_ref())
                    .is_some_and(|(old, new)| old.shapes() == new.shapes());
            let parent_generation = context.as_ref().map_or(0, |value| value.generation());
            // Regrant is not cancellation acknowledgement. Only a current
            // frame/query commit may clear an outstanding admission fence.
            self.geometry_input_pending =
                old_grant != 0 && (self.geometry_input_pending || !same_spatial_layer);
            self.geometry_dirty = true;
            self.curve_geometry = context;
            let mut resources = self.frame_resources.lock().unwrap();
            if let Some(pending) = resources.pending_curve_frame.take() {
                resources.retired_curve_query = Some(pending.query_request);
            }
            resources.pending_curve_publication = None;
            if !same_spatial_layer {
                resources.geometry_cancel_confirmed = false;
            }
            resources.parent_generation = parent_generation;
            resources.grant_serial = 0;
            resources.child_generation = 0;
            resources.expected_viewport = 0;
            resources.expected_seq = 0;
            resources.allocation_seq = 0;
            resources.layout_seq = 0;
            resources.grant_acknowledged = false;
            resources.deferred_curve_grant = false;
            resources.layer_layout = None;
            drop(resources);
            if old_grant != 0 && !same_spatial_layer {
                trace_layer(format_args!(
                    "event=revoke_send view={} grant={} next_gp={} capture={:?}",
                    self.native.as_ref().map_or(0, |native| native.view),
                    old_grant,
                    parent_generation,
                    self.content_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.reply().captured)
                ));
                let revoke = crate::component::curve::Envelope {
                    protocol_major: 1,
                    wire_revision: 1,
                    view: self.native.as_ref().map_or(0, |native| native.view),
                    transaction_id: 0,
                    ttl_ms: 0,
                    body: crate::component::curve::RequestBodyV1::RevokeAdmission {
                        grant_serial: old_grant,
                        reason: crate::component::curve::RevokeReasonWire::ParentGeometryChanged,
                    },
                };
                match self
                    .process
                    .as_ref()
                    .map(|process| process.request(crate::component::RequestBody::CurveV1(revoke)))
                {
                    Some(Ok(request)) => self
                        .frame_resources
                        .lock()
                        .unwrap()
                        .pending_revokes
                        .push_back((request, 0, old_grant)),
                    Some(Err(error)) => self.error = Some(error.to_string()),
                    None => self.error = Some("curve owner unavailable for revoke".into()),
                }
            }
            if let Some(context) = self.curve_geometry.clone()
                && let Err(error) = self.begin_curve_grant(context)
            {
                self.error = Some(error.to_string());
                self.geometry_input_pending = true;
            }
            (self.wake)();
        }
    }
    fn delegates_curve_focus(&self) -> bool {
        if self.curve_geometry.is_none()
            || !self.available()
            || self.geometry_dirty
            || self.geometry_input_pending
        {
            return false;
        }
        let resources = self.frame_resources.lock().unwrap();
        resources.layer_layout.is_some()
            && resources.image.as_ref().and_then(ViewImage::layer_layout)
                == resources.layer_layout.as_ref()
    }
    fn rectangular_input_geometry(&self) -> bool {
        self.input_ready()
            && self.content_capabilities().is_some_and(|c| {
                c.content_pointer || c.keyboard || c.text_ime || c.scroll || c.accessibility
            })
    }
    fn reading_scroll_source(&self) -> bool {
        self.content_reading_scroll
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn active(&self) -> bool {
        self.visible
    }
    fn interactive(&self) -> bool {
        self.visible
            && self.enabled
            && self.window_active
            && (self.toolbar.active
                || self.opaque_pointer
                || self.content_pointer_enabled()
                || self.content_scroll_enabled())
    }
    fn focusable(&self) -> bool {
        self.visible
            && self.enabled
            && self.window_active
            && ((self.toolbar.active && self.toolbar.control.focusable())
                || (self.content_keyboard_enabled() || self.content_accessibility_enabled())
                    && (self.input_ready()
                        || self.content_focused && self.available()
                        || self.pointer_focus == Some(false) && self.delegates_curve_focus()))
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let pointer_focus = self.pointer_focus.take();
        let acknowledged_child_focus = pointer_focus == Some(false)
            && self.content_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.reply().pointer_focus_requested
                    || snapshot.reply().accessibility_focus_requested
            });
        let toolbar = !self.remote_overlay()
            && pointer_focus.unwrap_or(self.toolbar.active && !self.content_focused);
        self.toolbar_focused = focused && toolbar;
        self.toolbar
            .control
            .set_focus(self.toolbar_focused, visible);
        let was_content = self.content_focused;
        self.content_focused = focused && !toolbar;
        // Hidden/inactive lifecycle already cancelled the original actor. A
        // parent scope restore must not send another blur through unavailable
        // geometry and poison the next show/activation.
        if self.available()
            && (self.content_keyboard_enabled() || self.content_accessibility_enabled())
            && (self.content_focused || was_content)
            && !(self.content_focused && acknowledged_child_focus)
        {
            self.focus_content(self.content_focused, visible);
        }
    }
    fn set_focus_from_traversal(&mut self, reverse: bool) {
        self.pointer_focus = None;
        if reverse && self.enter_content(true) {
            return;
        }
        if self.enter_toolbar(reverse) {
            return;
        }
        self.toolbar.control.set_focus(false, false);
        self.toolbar_focused = false;
        if !self.enter_content(reverse) {
            self.content_focused = false;
        }
    }
    fn take_text_key_result(&mut self, id: u64, binding: Option<&[u8]>) -> Option<ContentResponse> {
        use tekes_ui_controls::composition::TextKeyDisposition;
        if !self.input_ready() {
            return None;
        }
        let (mut response, mut bubbles, original_binding) = self
            .deferred_key_result
            .clone()
            .filter(|(response, _, _)| response.text_key.is_some_and(|key| key.id == id))?;
        if original_binding.as_deref() != binding {
            self.deferred_key_result = None;
            self.content_bubbles = Some(false);
            response.text_key.as_mut().unwrap().disposition = TextKeyDisposition::Cancelled;
            return Some(response);
        }
        if response
            .text_key
            .is_some_and(|key| key.disposition == TextKeyDisposition::Deferred)
        {
            let result = (|| -> Result<_> {
                let bridge = self.native_content_bridge(std::time::Duration::from_secs(1))?;
                let current = self.fresh_content_snapshot(&bridge)?;
                bridge.request(
                    &current,
                    crate::component::content::Input::TextKeyResult { id },
                )
            })();
            match result {
                Ok(next) => {
                    response = self.accept_content(next);
                    bubbles = self.content_bubbles == Some(true);
                }
                Err(error) => {
                    self.deferred_key_result = None;
                    response = self.content_failure(error);
                    response.text_key = Some(tekes_ui_controls::composition::TextKeyOutcome {
                        id,
                        target: None,
                        disposition: TextKeyDisposition::Cancelled,
                    });
                    return Some(response);
                }
            }
        }
        self.content_bubbles = Some(bubbles);
        self.deferred_key_result = if response
            .text_key
            .is_some_and(|key| key.disposition == TextKeyDisposition::Deferred)
        {
            Some((response, bubbles, original_binding))
        } else {
            None
        };
        Some(response)
    }
    fn focus_boundary(&mut self, reverse: bool) -> ContentResponse {
        if self.content_focused && reverse && self.enter_toolbar(true) {
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn delegates_text_keys(&self) -> bool {
        (self.content_focused || self.remote_overlay())
            && self.available()
            && self.content_keyboard_enabled()
    }
    fn native_key_binding(&self) -> Option<Vec<u8>> {
        self.query_content()?.native_binding()
    }
    fn text_input_snapshot(&self) -> Option<tekes_ui_controls::composition::ContentTextInput> {
        let query = self.query_content()?;
        let state = query.text_input_state()?;
        let ready = self.input_ready();
        let rect = |point: tekes_ui_platform_macos::Point, size: tekes_ui_platform_macos::Size| {
            Rect::new(
                point.x as f32,
                point.y as f32,
                size.width as f32,
                size.height as f32,
            )
        };
        let clip = if ready {
            query.reply().text.as_ref().and_then(|t| {
                let r = Rect::new(t.clip[0], t.clip[1], t.clip[2], t.clip[3])
                    .offset(self.view_frame.min_x(), self.view_frame.min_y());
                r.intersection(&self.view_frame)
                    .and_then(|r| r.intersection(&self.clip))
            })
        } else {
            None
        };
        Some(tekes_ui_controls::composition::ContentTextInput {
            binding: query.native_binding()?,
            text: state.text.clone(),
            caret_index: state.caret_index,
            selection: state.selection.clone(),
            marked: state.marked.clone(),
            read_only: state.read_only,
            caret: (ready && state.caret_size.height > 0.)
                .then(|| rect(state.caret_origin, state.caret_size)),
            clip,
            carets: if ready {
                state
                    .carets
                    .iter()
                    .map(|(index, p, s)| (*index, rect(*p, *s)))
                    .collect()
            } else {
                Vec::new()
            },
        })
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        self.cancel_geometry_input()
    }
    fn composing(&self) -> bool {
        (self.content_focused || self.remote_overlay())
            && self
                .content_snapshot
                .as_ref()
                .is_some_and(|s| s.reply().composing)
    }
    fn preserves_activation_focus(&self) -> bool {
        self.content_preserves_focus
    }
    fn blocks_event_bubbling(&self, _: &ContentEvent) -> bool {
        self.content_bubbles == Some(false)
    }
    fn bubble_handled_event(&self, _: &ContentEvent) -> bool {
        self.content_bubbles == Some(true)
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.content_bubbles = None;
        if !self.available() {
            return ContentResponse::default();
        }
        if self.remote_overlay() {
            return self.content_input(crate::component::content::Input::Key(
                crate::native_content::content_key(key),
            ));
        }
        let tab = if key.down {
            key.key.and_then(|k| match k {
                tekes_ui_controls::ControlKey::Tab { reverse } => Some(reverse),
                _ => None,
            })
        } else {
            None
        };
        if self.toolbar_focused {
            let response = self.toolbar.key(key);
            if !response.consumed && tab == Some(false) && self.enter_content(false) {
                return ContentResponse::consumed(true);
            }
            return self.finish_toolbar(response);
        }
        if self.content_focused && self.content_keyboard_enabled() {
            let response = self.content_input(crate::component::content::Input::Key(
                crate::native_content::content_key(key),
            ));
            if !response.consumed && self.content_bubbles == Some(true) && tab == Some(true) {
                let boundary = self.focus_boundary(true);
                if boundary.consumed {
                    return boundary;
                }
            }
            return response;
        }
        ContentResponse::default()
    }
    fn overlay_open(&self) -> bool {
        self.remote_overlay()
            || self.visible && self.toolbar.active && self.toolbar.control.is_open()
    }
    fn overlay_scope(&self) -> Option<tekes_ui_controls::composition::OverlayScope> {
        self.remote_overlay().then_some(Default::default())
    }
    fn overlay_pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if self.remote_overlay() {
            return if self.input_ready() {
                self.content_pointer(pointer)
            } else {
                ContentResponse::consumed(false)
            };
        }
        if pointer.is_primary() {
            self.overlay_event(&ContentEvent::Pointer(*pointer))
        } else {
            ContentResponse::default()
        }
    }
    fn overlay_key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.key_event(key)
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.content_reading_scroll = false;
        self.content_bubbles = None;
        if !self.available() {
            return ContentResponse::default();
        }
        if self.remote_overlay() {
            if matches!(event, ContentEvent::Scroll { .. }) && !self.content_scroll_enabled() {
                self.content_bubbles = Some(false);
                return ContentResponse::consumed(false);
            }
            if let Some(pointer) = Self::pointer_value(event) {
                return self.overlay_pointer_event(&pointer);
            }
            if !self.input_ready() {
                self.content_bubbles = Some(false);
                return ContentResponse::consumed(false);
            }
            if let Some(input) = crate::native_content::content_input(event) {
                return self.content_input(input);
            }
            if matches!(event, ContentEvent::Scroll { .. }) {
                self.content_bubbles = Some(false);
                return ContentResponse::consumed(false);
            }
        }
        let response = self.toolbar.event(event);
        self.finish_toolbar(response)
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.toolbar.paint_overlay(scene, text, theme);
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let view = self.toolbar.arrange(text, frame, clip);
        let moved = self.view_frame.origin != view.origin;
        let changed = (
            self.view_frame.size.width as u32,
            self.view_frame.size.height as u32,
        ) != (view.size.width as u32, view.size.height as u32);
        let allocation_changed = changed || moved || self.clip != clip;
        if allocation_changed {
            self.cancel_geometry_input();
            self.geometry_dirty = true;
            self.allocation_seq = self.allocation_seq.saturating_add(1);
        }
        self.frame = frame;
        self.view_frame = view;
        self.clip = clip;
        if self.process.is_some()
            && self.visible
            && view.size.width >= 1.
            && view.size.height >= 1.
            && (self.geometry_dirty || changed || self.repaint_dirty)
        {
            if self.geometry_dirty || changed {
                self.viewport += 1;
            }
            // begin_curve_preflight is an explicit promise that this same owner
            // will be admitted under a curved Panel. Its first and subsequent
            // layout input must follow the Host's current external-chain grant.
            // The freeze callback below queues grant then Layout in that order.
            let curve_preflight = matches!(
                self.curve_preflight,
                CurvePreflight::Starting
                    | CurvePreflight::Negotiating
                    | CurvePreflight::Negotiated { .. }
            );
            if !curve_preflight {
                self.geometry_dirty = false;
                self.repaint_dirty = false;
                if let Err(error) = self.enqueue(InputKind::Layout, view.origin, None) {
                    self.error = Some(error.to_string());
                }
            }
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let deferred_ready = {
            let resources = self.frame_resources.lock().unwrap();
            resources.deferred_curve_grant && resources.grant_acknowledged
        };
        if deferred_ready {
            // Resource completion resumes the latest already-frozen parent
            // context. It must not invalidate the Host geometry clock again.
            // The original setter retains exact revoke/cancel/owner checks.
            self.set_curve_geometry_context(self.curve_geometry.clone());
        }
        self.commit_pending_curve_publication();
        let (palette, surface) = if let Some(context) = &self.view_context {
            (
                crate::view_context_palette(context).expect("validated original View context"),
                context.surface.as_str(),
            )
        } else {
            (theme.palette, theme.surface())
        };
        tekes_ui_controls::paint::panel_on_surface_at_scale(
            scene, frame, palette, surface, self.scale,
        )
        .expect("validated original process frame Surface");
        if self.available()
            && self.content_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.reply().animation_frame_requested
                    && self.process.as_ref().zip(self.native.as_ref()).is_some_and(
                        |(process, view)| snapshot.belongs_to(process.session(), view.view),
                    )
            })
        {
            text.request_animation_frame();
        }
        let resources = self.frame_resources.lock().unwrap();
        if let (Some(registration), Some(image)) = (&resources.registration, &resources.image)
            && let Some(clip) = self.view_frame.intersection(&clip)
        {
            scene.push_clip(clip);
            scene.push_image(ImageQuad {
                rect: image.layer_layout().map_or_else(
                    || {
                        crate::panel_image_rect(
                            self.view_frame.origin,
                            (image.metadata().pixel_width, image.metadata().pixel_height),
                            self.scale,
                        )
                    },
                    |layer| layer.image_rect(),
                ),
                image: registration.id(),
                opacity: 1.,
            });
            scene.pop_clip();
        }
        self.toolbar.paint(scene, text, clip, theme);
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.content_reading_scroll = false;
        self.content_bubbles = None;
        match event {
            ContentEvent::Scroll { position, .. } if self.content_scroll_enabled() => {
                if !self.available() {
                    return ContentResponse::default();
                }
                if !self.input_ready() {
                    // Geometry is a barrier, not permission to apply the wheel
                    // to stale child coordinates or leak it to background readers.
                    self.content_bubbles = Some(false);
                    return ContentResponse::consumed(false);
                }
                let inside = self
                    .view_frame
                    .intersection(&self.clip)
                    .is_some_and(|r| r.contains(*position));
                if !inside && !self.remote_overlay() {
                    return ContentResponse::default();
                }
                match crate::native_content::content_input(event) {
                    Some(input) => self.content_input(input),
                    None => {
                        self.content_bubbles = Some(false);
                        ContentResponse::consumed(false)
                    }
                }
            }
            ContentEvent::Scroll { .. } => {
                // An unsupported child wheel remains available to the parent;
                // never send it through the focused editor/text fallback lane.
                let response = self.toolbar.event(event);
                self.finish_toolbar(response)
            }
            ContentEvent::TextKeyEnd { .. }
                if self.available() && self.content_keyboard_enabled() =>
            {
                self.content_input(
                    crate::native_content::content_input(event).expect("key finish mapping"),
                )
            }
            ContentEvent::Key { key, down } => self.key_event(&ContentKeyEvent {
                key: Some(*key),
                down: *down,
                repeat: false,
                modifiers: Default::default(),
                native_code: 0,
                characters: String::new(),
                characters_ignoring_modifiers: String::new(),
            }),
            ContentEvent::Cancel => {
                self.cancel();
                ContentResponse::default()
            }
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            ContentEvent::PointerDown(_)
            | ContentEvent::PointerMove(_)
            | ContentEvent::PointerUp(_) => {
                self.pointer_event(&Self::pointer_value(event).expect("pointer pattern"))
            }
            _ => {
                if !self.available() {
                    return ContentResponse::default();
                }
                if self.content_focused
                    && let Some(input) = crate::native_content::content_input(event)
                {
                    return self.content_input(input);
                }
                let response = self.toolbar.event(event);
                self.finish_toolbar(response)
            }
        }
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        self.content_bubbles = None;
        self.content_preserves_focus = false;
        if !self.available() {
            return ContentResponse::default();
        }
        // Existing child capture precedes local chrome hit testing, even when
        // the pointer crosses its toolbar. The outer host owns capture routing.
        if self.content_pointer_enabled() && self.content_captured() && self.input_ready() {
            return self.content_pointer(pointer);
        }
        let local = self.toolbar.pointer(pointer);
        if local.consumed {
            if pointer.phase == PointerPhase::Down {
                self.pointer_focus = Some(true);
            }
            return self.finish_toolbar(local);
        }
        if self.content_pointer_enabled() && !self.input_ready() {
            if self.content_captured()
                && pointer.is_primary()
                && self.delegates_curve_focus()
                && matches!(
                    pointer.phase,
                    PointerPhase::Up | PointerPhase::Move | PointerPhase::Drag
                )
            {
                let input = match crate::native_content::pointer_input(pointer, self.view_frame) {
                    Ok(input) => input,
                    Err(error) => {
                        self.error = Some(error.to_string());
                        self.content_bubbles = Some(false);
                        return ContentResponse::default();
                    }
                };
                return self.queue_content(input);
            }
            self.content_bubbles = Some(false);
            return ContentResponse::default();
        }
        if self.content_pointer_enabled() && self.input_ready() {
            let mut response = self.content_pointer(pointer);
            response.changed |= local.changed;
            return response;
        }
        if !pointer.is_primary() || !self.opaque_pointer || !self.input_ready() {
            return local;
        }
        let point = pointer.position;
        let mut response = match pointer.phase {
            PointerPhase::Down
                if self
                    .view_frame
                    .intersection(&self.clip)
                    .is_some_and(|r| r.contains(point)) =>
            {
                self.pressed = true;
                self.pointer_focus = Some(false);
                self.toolbar_focused = false;
                self.toolbar.control.set_focus(false, false);
                self.send_event(InputKind::PointerDown, point, true)
            }
            PointerPhase::Move | PointerPhase::Drag if self.pressed => {
                self.send_event(InputKind::PointerMove, point, true)
            }
            PointerPhase::Up if self.pressed => {
                self.pressed = false;
                self.send_event(InputKind::PointerUp, point, false)
            }
            _ => ContentResponse::default(),
        };
        response.changed |= local.changed;
        response
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        match event {
            Lifecycle::Mounted => {
                if let Err(error) = self.ensure_process_started() {
                    self.error = Some(error.to_string());
                }
            }
            Lifecycle::Unmounted => {
                self.semantic_projection.revoke();
                self.accessibility_focus = false;
                self.pressed = false;
                self.toolbar.reset();
                self.toolbar_focused = false;
                self.pointer_focus = None;
                let mut resources = self.frame_resources.lock().unwrap();
                resources.registration = None;
                resources.image = None;
                resources.expected_viewport = 0;
                resources.expected_seq = 0;
                resources.visible = false;
                self.process = None;
                self.curve_preflight = CurvePreflight::Unrequested;
                self.curve_request = None;
                self.curve_geometry = None;
                self.content_snapshot = None;
                self.content_focused = false;
                self.content_bubbles = None;
                self.geometry_input_pending = false;
                self.pending_content.clear();
                self.pending_content_bytes = 0;
                (self.wake)();
            }
        }
    }
    fn semantic_parts(&self) -> Vec<tekes_ui_controls::composition::SemanticPart> {
        if !self.semantic_ready() {
            return Vec::new();
        }
        self.semantic_projection.parts(
            self.view_frame,
            self.clip,
            self.content_focused || self.remote_overlay(),
        )
    }
    fn semantic_part_text_layout(
        &self,
        part: usize,
    ) -> Option<tekes_ui_controls::composition::ContentTextLayout> {
        self.semantic_ready()
            .then(|| {
                self.semantic_projection
                    .text_layout(part, self.view_frame, self.clip)
            })
            .flatten()
    }
    fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
        self.semantic_ready()
            .then(|| self.semantic_projection.value_range(part))
            .flatten()
    }
    fn semantic_part_parent(&self, part: usize) -> Option<usize> {
        self.semantic_ready()
            .then(|| self.semantic_projection.parent(part))
            .flatten()
    }
    fn semantic_part_preserves_activation_focus(&self, part: usize) -> bool {
        self.semantic_ready() && self.semantic_projection.preserves_focus(part)
    }
    fn accessibility_focus_requested(&self) -> bool {
        self.accessibility_focus
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &tekes_ui_controls::composition::AccessibilityAction,
    ) -> ContentResponse {
        self.accessibility_focus = false;
        if !self.semantic_ready() {
            return ContentResponse::default();
        }
        let Some((snapshot, target, child_part)) =
            part.and_then(|part| self.semantic_projection.callback(part))
        else {
            return ContentResponse::default();
        };
        use crate::component::content::AccessibilityAction as W;
        use tekes_ui_controls::composition::AccessibilityAction as A;
        let action = match action {
            A::Increment => W::Increment,
            A::Decrement => W::Decrement,
            A::Focus => W::Focus,
            A::Press => W::Press,
            A::SetValue(v) => W::SetValue(v.clone()),
            A::SetSelection(v) => W::SetSelection(v.clone()),
            A::ReplaceSelectedText(v) => W::ReplaceSelectedText(v.clone()),
        };
        let result = (|| -> Result<_> {
            self.native_content_bridge(std::time::Duration::from_secs(1))?
                .accessibility_action(&snapshot, target, child_part, action)
        })();
        match result {
            Ok(next) => {
                let requested = next.reply().accessibility_focus_requested;
                let response = self.accept_content(next);
                self.accessibility_focus = requested;
                if requested {
                    self.pointer_focus = Some(false);
                }
                if self.input_ready()
                    && let Err(error) = self.refresh_content_scope()
                {
                    self.semantic_projection.dirty();
                    self.error = Some(error.to_string());
                }
                response
            }
            Err(error) => {
                // A stale AX callback must not cancel a newer editor/gesture.
                self.semantic_projection.dirty();
                self.error = Some(error.to_string());
                ContentResponse::default()
            }
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, &self.module);
        semantics.disabled = !self.enabled;
        semantics
    }
}

#[cfg(test)]
mod curve_packet_contract_tests {
    #[test]
    fn host_negotiation_matches_existing_receiver_operation_contract() {
        let packet = super::curve_negotiation_envelope(23);
        assert_eq!(
            crate::component::curve::validate_for_attached_view(&packet, Some(23)),
            Ok(())
        );
    }
}
