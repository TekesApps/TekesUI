//! One module actor owns mutable state; GPU work cannot block its service lane.
use crate::{
    ComponentEmitter, ProcessModule,
    component::*,
    pipes,
    process::{FrameRenderer, validate_view_request},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::VecDeque,
    io::BufRead,
    sync::{Arc, mpsc},
};
use tekes_ui_app_protocol::{InputKind, Request};
use tekes_ui_scene::{GlyphAtlas, Scene};
struct Job {
    id: u64,
    input: Request,
    scene: Scene,
    atlas: GlyphAtlas,
    state: crate::ModuleState,
    clear: tekes_ui_tokens::Rgba,
    curve: Option<CurveRenderContext>,
}
struct CurveRenderContext {
    request: u64,
    transaction: u64,
    proposal: tekes_ui_app_protocol::component::curve::GrantProposal,
    parent_shapes: Arc<[tekes_ui_scene::ClipShape]>,
    child_generation: u64,
    layer: Option<crate::layer_raster::LayerRasterLayout>,
    stamp: tekes_ui_app_protocol::component::curve::FrameStamp,
}
struct PendingCurveGrant {
    request: u64,
    transaction: u64,
    proposal: tekes_ui_app_protocol::component::curve::GrantProposal,
    parent_shapes: Arc<[tekes_ui_scene::ClipShape]>,
    layer: Option<crate::layer_raster::LayerRasterLayout>,
}
fn child_curve_shapes(
    proposal: &tekes_ui_app_protocol::component::curve::GrantProposal,
) -> Result<Arc<[tekes_ui_scene::ClipShape]>> {
    use tekes_ui_scene::{ClipShape, CornerRadii, Point, Rect};
    let scale = proposal.backing_scale;
    let [origin_x, origin_y] = proposal.physical_origin_px;
    let mut shapes = Vec::with_capacity(proposal.records.0.len());
    for record in &proposal.records.0 {
        let [x, y, width, height] = record.rect_px;
        let [top_left, top_right, bottom_right, bottom_left] = record.radii_px;
        let shape = ClipShape::normalize(
            Rect::new(
                (x - origin_x) / scale,
                (y - origin_y) / scale,
                width / scale,
                height / scale,
            ),
            CornerRadii {
                top_left: top_left / scale,
                top_right: top_right / scale,
                bottom_right: bottom_right / scale,
                bottom_left: bottom_left / scale,
            },
            Point::default(),
        )
        .map_err(|error| anyhow::anyhow!("invalid Curve clip shape: {error:?}"))?;
        let rect = shape.rect();
        let radii = shape.radii();
        ensure!(
            rect.origin.x * scale + origin_x == x
                && rect.origin.y * scale + origin_y == y
                && rect.size.width * scale == width
                && rect.size.height * scale == height
                && radii.top_left * scale == top_left
                && radii.top_right * scale == top_right
                && radii.bottom_right * scale == bottom_right
                && radii.bottom_left * scale == bottom_left,
            "curve shape changed during child-coordinate mapping"
        );
        shapes.push(shape);
    }
    Ok(Arc::from(shapes))
}
fn content_input_allowed(input: &content::Input, capabilities: &ViewCapabilities) -> bool {
    match input {
        content::Input::Semantics | content::Input::Accessibility { .. } => {
            capabilities.accessibility
        }
        content::Input::Pointer(_) => capabilities.content_pointer,
        content::Input::Scroll(_)
        | content::Input::ReadingState { .. }
        | content::Input::RevealReadingItem { .. } => capabilities.scroll,
        content::Input::Key(_)
        | content::Input::TextKeyStart { .. }
        | content::Input::TextKeyEnd { .. }
        | content::Input::TextKeyResult { .. } => capabilities.keyboard,
        content::Input::Commit { .. }
        | content::Input::Preedit { .. }
        | content::Input::Unmark
        | content::Input::TextCommand(_) => capabilities.text_ime,
        _ => {
            capabilities.keyboard
                || capabilities.text_ime
                || capabilities.content_pointer
                || capabilities.scroll
                || capabilities.accessibility
        }
    }
}
enum Event {
    Client(ClientMessage),
    Eof,
    Failure(String),
    Rendered(
        GlyphAtlas,
        Option<tekes_ui_app_protocol::component::curve::FrameStamp>,
    ),
}
pub(crate) fn run(
    mut module: Box<dyn ProcessModule>,
    emitter: ComponentEmitter,
    mut input: impl BufRead + Send + 'static,
    attach: ViewAttach,
    required: Capabilities,
) -> Result<bool> {
    ensure!(
        attach.window != 0
            && attach.view != 0
            && !attach.endpoint.is_empty()
            && attach.endpoint.len() < 256
            && !attach.endpoint.contains('\0'),
        "invalid View attachment"
    );
    let (original_content, text_services) = module.content().map_or((false, false), |(_, text)| {
        (true, text.text_surface_host().is_some())
    });
    let supported = ViewCapabilities {
        primary_pointer: true,
        independent_toolbar: true,
        keyboard: original_content,
        scroll: original_content,
        content_pointer: original_content,
        accessibility: original_content,
        text_ime: text_services,
    };
    ensure!(
        supported.supports(attach.required) && module.view_capabilities().supports(attach.required),
        "unsupported View capabilities"
    );
    ensure!(
        module.view().is_some(),
        "module advertised View without an implementation"
    );
    emitter.output(Output::ReadyView {
        pid: std::process::id(),
        capabilities: required,
        window: attach.window,
        view: attach.view,
        native: attach.required,
    })?;
    module.started(&emitter)?;
    let (tx, rx) = mpsc::sync_channel::<Event>(64);
    let reader = tx.clone();
    std::thread::Builder::new()
        .name("component-input".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                while let Some(bytes) = pipes::read_packet(&mut input)? {
                    let packet = serde_json::from_slice(&bytes)?;
                    if reader.send(Event::Client(packet)).is_err() {
                        return Ok(());
                    }
                }
                let _ = reader.send(Event::Eof);
                Ok(())
            })();
            if let Err(error) = result {
                let _ = reader.send(Event::Failure(error.to_string()));
            }
        })?;
    let (jobs_tx, jobs) = mpsc::sync_channel::<Job>(1);
    let done = tx.clone();
    let output = emitter.clone();
    let target = attach.clone();
    let published_curve_output = Arc::new(std::sync::Mutex::new(
        None::<tekes_ui_app_protocol::component::curve::FrameStamp>,
    ));
    let gpu_published_curve_output = published_curve_output.clone();
    std::thread::Builder::new()
        .name("component-gpu".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                // No Renderer/device/Scene/Atlas is manufactured for handshake or
                // toolbar. The first actual render request is the allocation boundary.
                let mut renderer = None;
                while let Ok(mut job) = jobs.recv() {
                    let renderer = match &mut renderer {
                        Some(renderer) => renderer,
                        None => renderer.insert(FrameRenderer::new()?),
                    };
                    let (frame, curve_stamp, curve_ack) = if let Some(curve) = job.curve.as_ref() {
                        #[cfg(target_os = "macos")]
                        {
                            let frame = if let Some(layer) = &curve.layer {
                                renderer.render_layer(&job.input, &job.scene, &mut job.atlas,
                                    job.state, &target.endpoint, layer)?
                            } else {
                                renderer.render_with_parent_boundary_proof(
                                    &job.input, &job.scene, &mut job.atlas, job.state,
                                    &target.endpoint, curve.parent_shapes.clone())?
                            };
                            let ack = Output::CurveReplyV1 {
                                request: curve.request,
                                view: target.view,
                                reply: tekes_ui_app_protocol::component::curve::CurveReply::GrantPrepared {
                                    transaction_id: curve.transaction,
                                    proposal_id: curve.proposal.proposal_id,
                                    grant_serial: curve.proposal.proposal_id,
                                    parent_geometry_generation: curve.proposal.parent_geometry_generation,
                                    child_geometry_generation: curve.child_generation,
                                    viewport: curve.proposal.viewport,
                                    allocation_seq: curve.proposal.allocation_seq,
                                    layout_seq: curve.proposal.layout_seq,
                                },
                            };
                            (frame, Some(curve.stamp.clone()), Some(ack))
                        }
                    } else {
                        (
                            renderer.render_with_clear(
                                &job.input,
                                &job.scene,
                                &mut job.atlas,
                                job.state,
                                &target.endpoint,
                                job.clear,
                            )?,
                            None,
                            None,
                        )
                    };
                    let committed_stamp = curve_stamp.clone();
                    if let Some(stamp) = curve_stamp {
                        output.output(Output::CurveReplyV1 {
                            request: job.id,
                            view: target.view,
                            reply: if let Some(layer) = job.curve.as_ref().and_then(|curve| curve.layer.as_ref()) {
                                tekes_ui_app_protocol::component::curve::CurveReply::LayerFramePublished {
                                    stamp, frame: Box::new(frame), physical_origin_px: layer.physical_origin_px(),
                                }
                            } else {
                                tekes_ui_app_protocol::component::curve::CurveReply::FramePublished { stamp, frame: Box::new(frame) }
                            },
                        })?;
                    } else {
                        output.output(Output::Frame {
                            request: job.id,
                            view: target.view,
                            frame,
                        })?;
                    }
                    if let Some(ack) = curve_ack {
                        output.output(ack)?;
                    }
                    if let Some(stamp) = committed_stamp {
                        *gpu_published_curve_output.lock().unwrap() = Some(stamp);
                    }
                    // Return the exact dispatched job after publishing its
                    // frame and acknowledgement. Prepare/Revoke may already
                    // have retired its grant; current admission is checked separately.
                    let published_stamp = job.curve.as_ref().map(|curve| curve.stamp.clone());
                    if done.send(Event::Rendered(job.atlas, published_stamp)).is_err() {
                        return Ok(());
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                let _ = done.send(Event::Failure(error.to_string()));
            }
        })?;
    let mut last = 0;
    let mut sequence = None;
    let mut viewport = 0;
    let mut identity = None;
    let mut applied_viewport = None;
    let mut view_context: Option<ViewContext> = None;
    let mut atlas = None;
    let mut rendering = false;
    let mut rendering_curve_stamp = None;
    let mut pending = VecDeque::new();
    let mut negotiated_curve: Option<tekes_ui_app_protocol::component::curve::CapabilityWire> =
        None;
    let mut curve_grant: Option<PendingCurveGrant> = None;
    let mut active_curve_grant: Option<(u64, u64, u64)> = None;
    let mut independent_layer = false;
    while let Ok(event) = rx.recv() {
        match event {
            Event::Client(ClientMessage::Request {
                version,
                session,
                id,
                body,
            }) => {
                ensure!(
                    version == VERSION && session == *emitter.session() && id > last,
                    "stale View session/request"
                );
                last = id;
                match body {
                    RequestBody::CurveV1(envelope) => {
                        use tekes_ui_app_protocol::component::curve::{
                            CurveErrorCode, CurveReply, RequestBodyV1,
                        };
                        let transaction_id = envelope.transaction_id;
                        let reject = |code| {
                            emitter.output(Output::CurveReplyV1 {
                                request: id,
                                view: attach.view,
                                reply: CurveReply::Rejected {
                                    transaction_id,
                                    code,
                                },
                            })
                        };
                        if let Err(code) =
                            tekes_ui_app_protocol::component::curve::validate_for_attached_view(
                                &envelope,
                                Some(attach.view),
                            )
                        {
                            reject(code)?;
                            continue;
                        }
                        let layer_negotiation =
                            matches!(&envelope.body, RequestBodyV1::NegotiateLayer { .. });
                        let mut proposed_layer = None;
                        let body = match envelope.body {
                            RequestBodyV1::NegotiateLayer { offer } => {
                                RequestBodyV1::Negotiate { offer }
                            }
                            RequestBodyV1::PrepareLayerGrant { proposal } => {
                                if !independent_layer {
                                    reject(CurveErrorCode::CapabilityNotNegotiated)?;
                                    continue;
                                }
                                let layout = match crate::layer_raster::LayerRasterLayout::new(
                                    proposal.logical_size,
                                    proposal.geometry.backing_scale,
                                    proposal.geometry.physical_origin_px,
                                ) {
                                    Ok(layout) => layout,
                                    Err(_) => {
                                        reject(CurveErrorCode::MalformedMessage)?;
                                        continue;
                                    }
                                };
                                if layout.pixel_size()
                                    != (
                                        proposal.geometry.target_px[0],
                                        proposal.geometry.target_px[1],
                                    )
                                {
                                    reject(CurveErrorCode::MalformedMessage)?;
                                    continue;
                                }
                                proposed_layer = Some(layout);
                                RequestBodyV1::PrepareGrant {
                                    proposal: proposal.geometry,
                                }
                            }
                            body => body,
                        };
                        match body {
                            RequestBodyV1::Negotiate { offer } => {
                                ensure!(original_content, "curve needs the original ContentHost");
                                ensure!(
                                    module.content().is_some() && module.view().is_some(),
                                    "curve needs the original View and ContentHost"
                                );
                                if negotiated_curve.is_some()
                                    && independent_layer != layer_negotiation
                                {
                                    reject(CurveErrorCode::UnsupportedFeature)?;
                                    continue;
                                }
                                independent_layer = layer_negotiation;
                                negotiated_curve = Some(offer.clone());
                                emitter.output(Output::CurveReplyV1 {
                                    request: id,
                                    view: attach.view,
                                    reply: if independent_layer {
                                        CurveReply::LayerNegotiated { capability: offer }
                                    } else {
                                        CurveReply::Negotiated { capability: offer }
                                    },
                                })?;
                            }
                            RequestBodyV1::PrepareGrant { proposal } => {
                                ensure!(
                                    negotiated_curve.is_some(),
                                    "curve grant before capability negotiation"
                                );
                                if independent_layer != proposed_layer.is_some() {
                                    reject(CurveErrorCode::UnsupportedFeature)?;
                                    continue;
                                }
                                ensure!(
                                    curve_grant.is_none(),
                                    "curve grant already pending: view={} existing={:?} incoming={:?} rendering={} queued_layouts={}",
                                    attach.view,
                                    curve_grant.as_ref().map(|grant| (
                                        grant.request,
                                        grant.proposal.proposal_id,
                                        grant.proposal.viewport,
                                        grant.proposal.layout_seq,
                                    )),
                                    (
                                        id,
                                        proposal.proposal_id,
                                        proposal.viewport,
                                        proposal.layout_seq
                                    ),
                                    rendering,
                                    pending.len(),
                                );
                                let parent_shapes = child_curve_shapes(&proposal)?;
                                let (host, _) = module.content().ok_or_else(|| {
                                    anyhow::anyhow!("original ContentHost unavailable")
                                })?;
                                host.set_external_curve_chain(
                                    Some(proposal.parent_geometry_generation),
                                    parent_shapes.clone(),
                                )?;
                                active_curve_grant = None;
                                *published_curve_output.lock().unwrap() = None;
                                curve_grant = Some(PendingCurveGrant {
                                    request: id,
                                    transaction: envelope.transaction_id,
                                    proposal: *proposal,
                                    parent_shapes,
                                    layer: proposed_layer,
                                });
                            }
                            RequestBodyV1::NegotiateLayer { .. }
                            | RequestBodyV1::PrepareLayerGrant { .. } => {
                                unreachable!("layer operations are normalized before dispatch")
                            }
                            RequestBodyV1::RevokeAdmission { grant_serial, .. } => {
                                ensure!(
                                    negotiated_curve.is_some()
                                        && (curve_grant.as_ref().is_some_and(|grant| {
                                            grant.proposal.proposal_id == grant_serial
                                        }) || active_curve_grant
                                            .is_some_and(|(serial, _, _)| serial == grant_serial)),
                                    "revoke does not name the current curve grant"
                                );
                                let (host, _) = module.content().ok_or_else(|| {
                                    anyhow::anyhow!("original ContentHost unavailable")
                                })?;
                                host.cancel_pointer_capture();
                                host.set_external_curve_chain(None, Arc::from([]))?;
                                curve_grant = None;
                                active_curve_grant = None;
                                *published_curve_output.lock().unwrap() = None;
                                emitter.output(Output::CurveReplyV1 {
                                    request: id,
                                    view: attach.view,
                                    reply: CurveReply::GrantRevoked {
                                        transaction_id: envelope.transaction_id,
                                        grant_serial,
                                    },
                                })?;
                            }
                            RequestBodyV1::ContentInput { stamp, request } => {
                                use tekes_ui_app_protocol::component::curve::{
                                    ContentInputOutcome, CurveErrorCode, CurveReply, QueryStamp,
                                };
                                if !content_input_allowed(&request.input, &attach.required) {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::CapabilityNotNegotiated,
                                        },
                                    })?;
                                    continue;
                                }
                                let published = published_curve_output.lock().unwrap().clone();
                                let Some(frame_stamp) = published.as_ref() else {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                };
                                let Some(grant) = active_curve_grant else {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                };
                                let expected = QueryStamp {
                                    grant_serial: frame_stamp.grant_serial,
                                    gp: frame_stamp.gp,
                                    gc: frame_stamp.gc,
                                    view: frame_stamp.view,
                                    viewport: frame_stamp.viewport,
                                    allocation_seq: frame_stamp.allocation_seq,
                                    layout_seq: frame_stamp.layout_seq,
                                    published_frame_seq: frame_stamp.frame_seq,
                                };
                                if !(stamp == expected
                                    && request.view == expected.view
                                    && request.viewport == expected.viewport
                                    && (frame_stamp.grant_serial, frame_stamp.gp, frame_stamp.gc)
                                        == grant)
                                {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                }
                                let (host, text) = module
                                    .content()
                                    .context("original ContentHost unavailable")?;
                                if host.external_curve_ack()
                                    != Some((frame_stamp.gp, frame_stamp.gc))
                                {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                }
                                let outcome =
                                    match crate::content_input::dispatch(host, text, *request) {
                                        Ok(reply) => ContentInputOutcome::Applied {
                                            reply: Box::new(reply),
                                        },
                                        Err(error) => ContentInputOutcome::Failed {
                                            message: error.to_string(),
                                        },
                                    };
                                let geometry_changed = host.external_curve_ack()
                                    != Some((frame_stamp.gp, frame_stamp.gc));
                                emitter.output(Output::CurveReplyV1 {
                                    request: id,
                                    view: attach.view,
                                    reply: CurveReply::ContentInput {
                                        transaction_id: envelope.transaction_id,
                                        stamp,
                                        outcome,
                                    },
                                })?;
                                if geometry_changed {
                                    emitter.invalidate_view_next()?;
                                }
                            }
                            RequestBodyV1::ContentQuery { query } => {
                                use tekes_ui_app_protocol::component::curve::{
                                    CurveErrorCode, CurveReply, QueryStamp, ReadOnlyQueryWire,
                                };
                                let query_allowed = match &query {
                                    ReadOnlyQueryWire::SemanticTree { .. } => {
                                        attach.required.accessibility
                                    }
                                    ReadOnlyQueryWire::ReadingState { .. } => {
                                        attach.required.scroll
                                    }
                                };
                                if !query_allowed {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::CapabilityNotNegotiated,
                                        },
                                    })?;
                                    continue;
                                }
                                let published = published_curve_output.lock().unwrap().clone();
                                let Some(frame_stamp) = published.as_ref() else {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                };
                                let Some(grant) = active_curve_grant else {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                };
                                if (frame_stamp.grant_serial, frame_stamp.gp, frame_stamp.gc)
                                    != grant
                                {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                }
                                let expected = QueryStamp {
                                    grant_serial: frame_stamp.grant_serial,
                                    gp: frame_stamp.gp,
                                    gc: frame_stamp.gc,
                                    view: frame_stamp.view,
                                    viewport: frame_stamp.viewport,
                                    allocation_seq: frame_stamp.allocation_seq,
                                    layout_seq: frame_stamp.layout_seq,
                                    published_frame_seq: frame_stamp.frame_seq,
                                };
                                let stamp = match &query {
                                    ReadOnlyQueryWire::SemanticTree { stamp }
                                    | ReadOnlyQueryWire::ReadingState { stamp, .. } => stamp,
                                };
                                if stamp != &expected {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                }
                                let (host, _) = module
                                    .content()
                                    .context("original ContentHost unavailable")?;
                                if host.external_curve_ack()
                                    != Some((frame_stamp.gp, frame_stamp.gc))
                                {
                                    emitter.output(Output::CurveReplyV1 {
                                        request: id,
                                        view: attach.view,
                                        reply: CurveReply::Rejected {
                                            transaction_id: envelope.transaction_id,
                                            code: CurveErrorCode::NotReady,
                                        },
                                    })?;
                                    continue;
                                }
                                let reading = match &query {
                                    ReadOnlyQueryWire::SemanticTree { .. } => None,
                                    ReadOnlyQueryWire::ReadingState {
                                        reader_id,
                                        reader_mount,
                                        ..
                                    } => {
                                        let reading = crate::content_reading::read_only_state(
                                            host, *reader_id,
                                        )?;
                                        ensure!(
                                            reading.reader.mount == *reader_mount,
                                            "Curve reading query names a stale reader"
                                        );
                                        Some(reading)
                                    }
                                };
                                let reply = crate::content_input::read_only_projection(
                                    host,
                                    attach.view,
                                    frame_stamp.viewport,
                                    reading,
                                )?;
                                emitter.output(Output::CurveReplyV1 {
                                    request: id,
                                    view: attach.view,
                                    reply: CurveReply::ContentProjection {
                                        transaction_id: envelope.transaction_id,
                                        stamp: expected,
                                        reply: Box::new(reply),
                                    },
                                })?;
                            }
                            RequestBodyV1::TransactionStatus => {
                                emitter.output(Output::CurveReplyV1 {
                                    request: id,
                                    view: attach.view,
                                    reply: CurveReply::Rejected {
                                        transaction_id: envelope.transaction_id,
                                        code: CurveErrorCode::CapabilityNotNegotiated,
                                    },
                                })?;
                            }
                        }
                    }
                    RequestBody::Content(request) => {
                        // A queued or still-unapplied layout cannot authorize
                        // native queries against the previous shaped geometry.
                        let result = (|| -> Result<serde_json::Value> {
                            ensure!(
                                request.view == attach.view
                                    && request.viewport == viewport
                                    && applied_viewport == Some(viewport),
                                content::GEOMETRY_PENDING
                            );
                            ensure!(
                                negotiated_curve.is_none(),
                                "Curve Content input requires the stamped Curve lane"
                            );
                            ensure!(
                                content_input_allowed(&request.input, &attach.required),
                                "Content input capability not negotiated"
                            );
                            let (host, text) = module
                                .content()
                                .ok_or_else(|| anyhow::anyhow!("original Content unavailable"))?;
                            if negotiated_curve.is_some() {
                                let (_, gp, gc) = active_curve_grant
                                    .ok_or_else(|| anyhow::anyhow!(content::GEOMETRY_PENDING))?;
                                ensure!(
                                    host.external_curve_ack()
                                        .is_some_and(|current| { current == (gp, gc) }),
                                    content::GEOMETRY_PENDING
                                );
                            }
                            serde_json::to_value(crate::content_input::dispatch(
                                host, text, request,
                            )?)
                            .map_err(Into::into)
                        })()
                        .map_err(|e| e.to_string());
                        emitter.output(Output::Response {
                            request: id,
                            result,
                        })?;
                    }
                    RequestBody::SetViewContext { view, context } => {
                        let result = (|| -> Result<serde_json::Value> {
                            ensure!(view == attach.view, "wrong View context owner");
                            let geometry_changed = crate::view_context::apply(
                                &mut *module,
                                view_context.as_ref(),
                                &context,
                            )?;
                            view_context = Some(context);
                            // Queries cannot observe old shapes under new fonts.
                            if geometry_changed {
                                applied_viewport = None;
                                *published_curve_output.lock().unwrap() = None;
                            }
                            Ok(serde_json::Value::Null)
                        })()
                        .map_err(|error| error.to_string());
                        emitter.output(Output::Response {
                            request: id,
                            result,
                        })?;
                    }
                    RequestBody::Stop => {
                        emitter.output(Output::Response {
                            request: id,
                            result: Ok(serde_json::Value::Null),
                        })?;
                        return Ok(true);
                    }
                    RequestBody::Render { view, input } => {
                        ensure!(
                            required.view && view == attach.view,
                            "wrong/unnegotiated View"
                        );
                        validate_view_request(&input)?;
                        ensure!(
                            sequence.is_none_or(|old| input.seq > old)
                                && input.viewport >= viewport
                                && identity.is_none_or(|old| input.identity == old),
                            "stale View input"
                        );
                        ensure!(
                            matches!(input.kind, InputKind::Layout | InputKind::Cancel)
                                || attach.required.primary_pointer,
                            "primary View input not negotiated"
                        );
                        // Toolbar commands use the generation-bound service Invoke.
                        ensure!(
                            input.kind != InputKind::Command,
                            "View command requires registered Invoke"
                        );
                        ensure!(
                            pending.len() + usize::from(rendering) < 2,
                            "View render backlog full"
                        );
                        sequence = Some(input.seq);
                        viewport = input.viewport;
                        identity = Some(input.identity);
                        pending.push_back((id, input));
                    }
                    body => {
                        ensure!(
                            match &body {
                                RequestBody::Message { .. } => required.services,
                                RequestBody::Invoke { .. } => required.toolbar,
                                _ => false,
                            },
                            "service capability not negotiated"
                        );
                        let result = if crate::services::valid_invoke(&body, &emitter) {
                            module.handle(body, &emitter).map_err(|e| e.to_string())
                        } else {
                            Err("stale or disabled toolbar action".into())
                        };
                        emitter.output(Output::Response {
                            request: id,
                            result,
                        })?;
                    }
                }
            }
            Event::Client(_) => bail!("duplicate View Start"),
            Event::Rendered(returned, frame_stamp) => {
                // A real Prepare/Revoke may withdraw admission while the one
                // dispatched GPU job finishes. Validate that immutable job's
                // full identity, not a newer grant; completion grants no input.
                ensure!(
                    rendering && frame_stamp == rendering_curve_stamp.take(),
                    "rendered Curve frame does not match the dispatched job"
                );
                atlas = Some(returned);
                rendering = false;
            }
            Event::Eof => return Ok(true),
            Event::Failure(message) => bail!("{message}"),
        }
        if !rendering && let Some((id, input)) = pending.pop_front() {
            // The actor owns original Content cancellation even when the
            // module's legacy render callback only paints. Use the ordered
            // existing Cancel lane, not another focus/capture state machine.
            if input.kind == InputKind::Cancel
                && let Some((host, _)) = module.content()
            {
                host.cancel();
            }
            let mut atlas = atlas.take().unwrap_or_default();
            atlas.begin_frame();
            let mut scene = Scene::new();
            let current_curve_grant = if negotiated_curve.is_some() {
                let grant = curve_grant
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("curve render has no current parent grant"))?;
                ensure!(
                    input.kind == InputKind::Layout
                        && input.viewport == grant.proposal.viewport
                        && input.width > 0
                        && input.height > 0
                        && input.scale == grant.proposal.backing_scale
                        && grant
                            .layer
                            .as_ref()
                            .is_none_or(|layer| layer.logical_size() == [input.width, input.height])
                        && grant.layer.as_ref().map_or_else(
                            || (
                                (input.width as f32 * input.scale).round() as u32,
                                (input.height as f32 * input.scale).round() as u32,
                            ),
                            |layer| layer.pixel_size()
                        ) == (grant.proposal.target_px[0], grant.proposal.target_px[1]),
                    "curve grant does not match the actual child layout input"
                );
                Some(grant)
            } else {
                None
            };
            let state = module
                .view()
                .ok_or_else(|| anyhow::anyhow!("View disappeared"))?
                .update(&input, &mut scene, &mut atlas)?;
            let curve = if let Some(grant) = current_curve_grant {
                let (host, _) = module
                    .content()
                    .ok_or_else(|| anyhow::anyhow!("original ContentHost disappeared"))?;
                let (parent_generation, child_generation) =
                    host.external_curve_ack().ok_or_else(|| {
                        anyhow::anyhow!("child Host did not acknowledge current geometry")
                    })?;
                ensure!(
                    parent_generation == grant.proposal.parent_geometry_generation
                        && child_generation != 0,
                    "child Host acknowledged a different geometry lease"
                );
                let stamp = tekes_ui_app_protocol::component::curve::FrameStamp {
                    grant_serial: grant.proposal.proposal_id,
                    gp: parent_generation,
                    gc: child_generation,
                    view: attach.view,
                    viewport: grant.proposal.viewport,
                    allocation_seq: grant.proposal.allocation_seq,
                    layout_seq: grant.proposal.layout_seq,
                    frame_seq: input.seq,
                };
                active_curve_grant = Some((
                    grant.proposal.proposal_id,
                    parent_generation,
                    child_generation,
                ));
                Some(CurveRenderContext {
                    request: grant.request,
                    transaction: grant.transaction,
                    proposal: grant.proposal,
                    parent_shapes: grant.parent_shapes,
                    child_generation,
                    layer: grant.layer,
                    stamp,
                })
            } else {
                None
            };
            if input.kind == InputKind::Layout {
                applied_viewport = Some(input.viewport);
            }
            rendering_curve_stamp = curve.as_ref().map(|curve| curve.stamp.clone());
            jobs_tx.send(Job {
                id,
                input,
                scene,
                atlas,
                state,
                clear: if curve.is_some() {
                    tekes_ui_tokens::Rgba::new(0., 0., 0., 0.)
                } else {
                    match &view_context {
                        Some(context) => crate::view_context::view_context_background(context)?,
                        None => {
                            crate::view_context::view_context_background(&ViewContext::default())?
                        }
                    }
                },
                curve,
            })?;
            rendering = true;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod curve_mapping_regressions {
    #[test]
    fn actual_three_column_grid_mapping_retains_fractional_parent_and_refuses_witness() {
        use crate::component::curve::{BoundedRecords, GrantProposal, ShapeRecord};
        use tekes_ui_scene::Rect;
        let cell = tekes_ui_controls::extended::Grid::new(3).layout(
            Rect::new(0., 0., 1000., 500.),
            3,
            500.,
        )[1];
        let scale = 2.;
        let width_px = (cell.size.width as u32 as f32 * scale).round() as u32;
        let proposal = GrantProposal {
            proposal_id: 1,
            viewport: 1,
            allocation_seq: 1,
            layout_seq: 1,
            parent_geometry_generation: 1,
            target_px: [width_px, 1000],
            physical_origin_px: [cell.origin.x * scale, cell.origin.y * scale],
            backing_scale: scale,
            clear_rgba: [0.; 4],
            sample_count: 1,
            coordinate_mapping: "SamePhysicalCoordinates".into(),
            chain_head: 0,
            chain_depth: 1,
            records: BoundedRecords(vec![ShapeRecord {
                index: 0,
                parent_index: None,
                depth: 1,
                rect_px: [
                    cell.origin.x * scale,
                    cell.origin.y * scale,
                    cell.size.width * scale,
                    cell.size.height * scale,
                ],
                radii_px: [0.; 4],
            }]),
        };
        let mapped = super::child_curve_shapes(&proposal).unwrap();
        assert_eq!(
            mapped[0].rect(),
            Rect::new(0., 0., cell.size.width, cell.size.height)
        );
        assert_eq!(width_px, 666);
        assert_ne!((cell.size.width * scale * 256.).fract(), 0.);
        assert!(
            mapped[0]
                .full_coverage_inner_rect_px(scale, (width_px, 1000))
                .unwrap()
                .is_none()
        );
    }
}
