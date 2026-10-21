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
}
enum Event {
    Client(ClientMessage),
    Eof,
    Failure(String),
    Rendered(GlyphAtlas),
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
    let supported = ViewCapabilities {
        primary_pointer: true,
        independent_toolbar: true,
        ..Default::default()
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
                    let frame = renderer.render(
                        &job.input,
                        &job.scene,
                        &mut job.atlas,
                        job.state,
                        &target.endpoint,
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
            let mut atlas = atlas.take().unwrap_or_default();
            atlas.begin_frame();
            let mut scene = Scene::new();
            let state = module
                .view()
                .ok_or_else(|| anyhow::anyhow!("View disappeared"))?
                .update(&input, &mut scene, &mut atlas)?;
            jobs_tx.send(Job {
                id,
                input,
                scene,
                atlas,
                state,
            })?;
            rendering = true;
        }
    }
    Ok(true)
}
