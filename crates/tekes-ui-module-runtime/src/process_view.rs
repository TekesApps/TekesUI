//! One module actor owns mutable state; GPU work cannot block its service lane.
use crate::{
    ComponentEmitter, ProcessModule,
    component::*,
    pipes,
    process::{FrameRenderer, validate_view_request},
};
use anyhow::{Result, bail, ensure};
use std::{collections::VecDeque, io::BufRead, sync::mpsc};
use tekes_ui_app_protocol::{InputKind, Request};
use tekes_ui_scene::{GlyphAtlas, Scene};
struct Job {
    id: u64,
    input: Request,
    scene: Scene,
    atlas: GlyphAtlas,
    state: crate::ModuleState,
    clear: tekes_ui_tokens::Rgba,
}
enum Event {
    Client(ClientMessage),
    Eof,
    Failure(String),
    Rendered(GlyphAtlas),
}
fn emit_curve_refusal(
    request: u64,
    envelope: &tekes_ui_app_protocol::component::curve::Envelope,
    attached_view: Option<u64>,
    emit: impl FnOnce(Output) -> Result<()>,
) -> Result<()> {
    // No module, ContentHost, renderer, frame queue, or grant owner is in this seam.
    emit(tekes_ui_app_protocol::component::curve::rejection_output(
        request,
        envelope,
        attached_view,
    ))
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
                    let frame = renderer.render_with_clear(
                        &job.input,
                        &job.scene,
                        &mut job.atlas,
                        job.state,
                        &target.endpoint,
                        job.clear,
                    )?;
                    // Return actor ownership before publishing the frame. The
                    // parent can immediately return frame credit and send work.
                    if done.send(Event::Rendered(job.atlas)).is_err() {
                        return Ok(());
                    }
                    output.output(Output::Frame {
                        request: job.id,
                        view: target.view,
                        frame,
                    })?;
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
    let mut pending = VecDeque::new();
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
                        emit_curve_refusal(id, &envelope, Some(attach.view), |output| {
                            emitter.output(output)
                        })?;
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
                            let allowed = match &request.input {
                                content::Input::Semantics
                                | content::Input::Accessibility { .. } => {
                                    attach.required.accessibility
                                }
                                content::Input::Pointer(_) => attach.required.content_pointer,
                                content::Input::Scroll(_)
                                | content::Input::ReadingState { .. }
                                | content::Input::RevealReadingItem { .. } => {
                                    attach.required.scroll
                                }
                                content::Input::Key(_)
                                | content::Input::TextKeyStart { .. }
                                | content::Input::TextKeyEnd { .. }
                                | content::Input::TextKeyResult { .. } => attach.required.keyboard,
                                content::Input::Commit { .. }
                                | content::Input::Preedit { .. }
                                | content::Input::Unmark
                                | content::Input::TextCommand(_) => attach.required.text_ime,
                                _ => {
                                    attach.required.keyboard
                                        || attach.required.text_ime
                                        || attach.required.content_pointer
                                        || attach.required.scroll
                                        || attach.required.accessibility
                                }
                            };
                            ensure!(allowed, "Content input capability not negotiated");
                            let (host, text) = module
                                .content()
                                .ok_or_else(|| anyhow::anyhow!("original Content unavailable"))?;
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
            Event::Rendered(returned) => {
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
            let state = module
                .view()
                .ok_or_else(|| anyhow::anyhow!("View disappeared"))?
                .update(&input, &mut scene, &mut atlas)?;
            if input.kind == InputKind::Layout {
                applied_viewport = Some(input.viewport);
            }
            jobs_tx.send(Job {
                id,
                input,
                scene,
                atlas,
                state,
                clear: match &view_context {
                    Some(context) => crate::view_context::view_context_background(context)?,
                    None => crate::view_context::view_context_background(&ViewContext::default())?,
                },
            })?;
            rendering = true;
        }
    }
    Ok(true)
}
