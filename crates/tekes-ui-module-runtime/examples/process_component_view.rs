//! Independent negotiated View consumer; service and toolbar do not depend on frame credit.
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Result, ensure};
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};
    use tekes_ui_app_protocol::{InputKind, Request, ToolbarItem};
    use tekes_ui_module_runtime::{component::*, *};
    use tekes_ui_scene::{GlyphAtlas, Quad, Rect, Scene};
    fn invalidation_peer() -> Result<()> {
        use std::io::Write;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let ClientMessage::StartView {
            session,
            required,
            attach,
            ..
        } = serde_json::from_str(&line)?
        else {
            anyhow::bail!("missing View start")
        };
        let send = |serial, output| -> Result<()> {
            println!(
                "{}",
                serde_json::to_string(&ServerMessage {
                    version: VERSION,
                    session: session.clone(),
                    serial,
                    output
                })?
            );
            std::io::stdout().flush()?;
            Ok(())
        };
        send(
            1,
            Output::ReadyView {
                pid: std::process::id(),
                capabilities: required,
                window: attach.window,
                view: attach.view,
                native: attach.required,
            },
        )?;
        send(
            2,
            Output::Event(ComponentEvent::ViewInvalidated {
                view: attach.view,
                revision: 2,
            }),
        )?;
        send(
            3,
            Output::Event(ComponentEvent::ViewInvalidated {
                view: attach.view,
                revision: 1,
            }),
        )?;
        loop {
            std::thread::park();
        }
    }
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--tekesui-component")
        && args.get(2).map(String::as_str) == Some("fault-invalidation")
    {
        return invalidation_peer();
    }
    struct Counter(u64);
    impl PanelModule for Counter {
        fn update(
            &mut self,
            input: &Request,
            scene: &mut Scene,
            _: &mut GlyphAtlas,
        ) -> Result<ModuleState> {
            scene.push_quad(Quad::new(
                Rect::new(0., 0., input.width as f32, input.height as f32),
                tekes_ui_tokens::Rgba::gray(if self.0 == 0 { 1. } else { 0. }),
            ));
            Ok(ModuleState {
                revision: self.0,
                toolbar: vec![],
            })
        }
    }
    impl ProcessModule for Counter {
        fn view(&mut self) -> Option<&mut dyn PanelModule> {
            Some(self)
        }
        fn view_capabilities(&self) -> ViewCapabilities {
            ViewCapabilities {
                primary_pointer: true,
                independent_toolbar: true,
                ..Default::default()
            }
        }
        fn started(&mut self, e: &ComponentEmitter) -> Result<()> {
            e.emit(ComponentEvent::Toolbar {
                revision: 1,
                scope: ToolbarScope::Window {
                    window: e.window_identity().unwrap_or(1),
                },
                items: vec![ToolbarAction {
                    command: ToolbarItem {
                        id: 7,
                        title: "Increment".into(),
                        enabled: true,
                        geometry_dependent: false,
                    },
                    icon: None,
                    accessible_name: None,
                    tooltip: None,
                }],
            })
        }
        fn handle(&mut self, body: RequestBody, e: &ComponentEmitter) -> Result<Value> {
            if let RequestBody::Message { topic, .. } = &body {
                if topic == "local-toolbar" || topic == "window-toolbar" {
                    e.emit(ComponentEvent::Toolbar {
                        revision: if topic == "local-toolbar" { 2 } else { 3 },
                        scope: if topic == "local-toolbar" {
                            ToolbarScope::View
                        } else {
                            ToolbarScope::Window {
                                window: e.window_identity().unwrap(),
                            }
                        },
                        items: vec![ToolbarAction {
                            command: ToolbarItem {
                                id: 7,
                                title: "Increment".into(),
                                enabled: true,
                                geometry_dependent: false,
                            },
                            icon: None,
                            accessible_name: None,
                            tooltip: None,
                        }],
                    })?;
                    return Ok(json!(self.0));
                }
                if topic == "redraw-burst" {
                    for _ in 0..130 {
                        self.0 += 1;
                        e.invalidate_view(self.0)?;
                    }
                    return Ok(json!(self.0));
                }
            }
            self.0 += 1;
            e.invalidate_view(self.0)?;
            Ok(json!(self.0))
        }
    }
    fn create() -> Box<dyn ProcessModule> {
        Box::new(Counter(0))
    }
    let caps = Capabilities {
        view: true,
        ..Capabilities::HEADLESS
    };
    if run_component_worker_from_args(
        &[
            ComponentDefinition {
                id: "view",
                capabilities: caps,
                create,
            },
            ComponentDefinition {
                id: "headless",
                capabilities: caps,
                create,
            },
        ],
        &std::env::args().collect::<Vec<_>>(),
    )? {
        return Ok(());
    }
    fn until(mut test: impl FnMut() -> bool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !test() {
            ensure!(Instant::now() < deadline, "correctness deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn ready(p: &ProcessComponent) -> Result<()> {
        until(|| {
            matches!(
                p.status(),
                ProcessStatus::Ready { .. } | ProcessStatus::Failed { .. }
            )
        })?;
        ensure!(
            matches!(p.status(), ProcessStatus::Ready { .. }),
            "{:?}",
            p.status()
        );
        let mut toolbar = false;
        until(|| {
            for e in p.drain_events() {
                toolbar |= matches!(e.output, Output::Event(ComponentEvent::Toolbar { .. }));
            }
            toolbar
        })
    }
    fn reply(p: &ProcessComponent, id: u64) -> Result<Value> {
        let mut value = None;
        until(|| {
            for e in p.drain_events() {
                if let Output::Response { request, result } = e.output
                    && request == id
                {
                    value = Some(result);
                }
            }
            value.is_some()
        })?;
        value.unwrap().map_err(anyhow::Error::msg)
    }
    let headless = ProcessComponent::spawn(ProcessComponentOptions::new("headless", 10), || {})?;
    ready(&headless)?;
    ensure!(headless.view_capabilities().is_none());
    ensure!(reply(&headless, headless.invoke(7)?)? == json!(1));
    let native = ViewCapabilities {
        primary_pointer: true,
        independent_toolbar: true,
        ..Default::default()
    };
    let p = ProcessComponent::spawn(
        ProcessComponentOptions::new("view", 10).with_view(1, 2, native),
        || {},
    )?;
    ready(&p)?;
    ensure!(p.view_capabilities() == Some(native));
    ensure!(
        p.outstanding_view_frames() == 0,
        "handshake/toolbar allocated a frame"
    );
    let request = |seq| Request {
        version: tekes_ui_app_protocol::VERSION,
        seq,
        viewport: 1,
        kind: InputKind::Layout,
        command: None,
        x: 0.,
        y: 0.,
        width: 32,
        height: 24,
        identity: 8,
        scale: 1.,
    };
    p.enqueue_view(request(1))?;
    let mut frames = Vec::new();
    until(|| {
        frames.extend(p.drain_view_frames());
        frames.len() == 1
    })?;
    p.enqueue_view(request(2))?;
    until(|| {
        frames.extend(p.drain_view_frames());
        frames.len() == 2
    })?;
    ensure!(p.outstanding_view_frames() == 2);
    // Hold both surfaces: actor control remains usable while rendering is backpressured.
    ensure!(reply(&p, p.invoke(7)?)? == json!(1));
    let old = p.toolbar_registration().unwrap();
    ensure!(
        reply(
            &p,
            p.request(RequestBody::Message {
                topic: "local-toolbar".into(),
                payload: Value::Null
            })?
        )? == json!(1)
    );
    let local = p.toolbar_registration().unwrap();
    ensure!(matches!(
        local.output,
        Output::Event(ComponentEvent::Toolbar {
            scope: ToolbarScope::View,
            revision: 2,
            ..
        })
    ));
    ensure!(
        p.request_from(
            &old.session,
            RequestBody::Invoke {
                command: 7,
                registration: 1,
                window: 1
            }
        )
        .is_err()
    );
    ensure!(
        reply(
            &p,
            p.request_from(
                &local.session,
                RequestBody::Invoke {
                    command: 7,
                    registration: 2,
                    window: 1
                }
            )?
        )? == json!(2)
    );
    ensure!(
        reply(
            &p,
            p.request(RequestBody::Message {
                topic: "window-toolbar".into(),
                payload: Value::Null
            })?
        )? == json!(2)
    );
    ensure!(matches!(
        p.toolbar_registration().unwrap().output,
        Output::Event(ComponentEvent::Toolbar {
            scope: ToolbarScope::Window { window: 1 },
            revision: 3,
            ..
        })
    ));
    ensure!(
        p.outstanding_view_frames() == 2,
        "toolbar placement consumed frame credit"
    );
    ensure!(p.take_view_invalidated());
    ensure!(!p.take_view_invalidated());
    let mut renderer = tekes_ui_render::Renderer::new_headless()?;
    let gpu = renderer.gpu_context();
    for frame in frames.drain(..) {
        drop(frame.import(&gpu)?);
    }
    until(|| p.outstanding_view_frames() == 0)?;
    // Exercise immediate return/reuse repeatedly; catches actor credit ordering.
    for seq in 3..19 {
        p.enqueue_view(request(seq))?;
        until(|| {
            frames.extend(p.drain_view_frames());
            !frames.is_empty()
        })?;
        let image = frames.pop().unwrap().import(&gpu)?;
        ensure!(image.metadata().revision == 2);
        let id = renderer.register_gpu_image(image.image())?;
        let mut scene = Scene::new();
        scene.push_image(tekes_ui_scene::ImageQuad {
            rect: Rect::new(0., 0., 32., 24.),
            image: id,
            opacity: 1.,
        });
        let pixels = renderer.render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (32, 24),
            1.,
            tekes_ui_tokens::Rgba::gray(1.),
        )?;
        ensure!(
            pixels.chunks_exact(4).all(|p| p == [0, 0, 0, 255]),
            "actual consumer pixels"
        );
        renderer.unregister_image(id);
        drop(image);
        until(|| p.outstanding_view_frames() == 0)?;
    }
    // No service drain until a later real frame proves the actor/receiver have
    // processed all130 invalidations. Visual signals do not fill the64event lane.
    let burst = p.request(RequestBody::Message {
        topic: "redraw-burst".into(),
        payload: Value::Null,
    })?;
    p.enqueue_view(request(19))?;
    until(|| {
        frames.extend(p.drain_view_frames());
        !frames.is_empty()
    })?;
    ensure!(frames[0].metadata().revision == 132);
    let events = p.drain_events();
    ensure!(events.iter().any(|e|matches!(&e.output,Output::Response{request,result:Ok(v)} if *request==burst && *v==json!(132))));
    ensure!(!events.iter().any(|e| matches!(
        e.output,
        Output::Event(ComponentEvent::ViewInvalidated { .. })
    )));
    reply(&p, p.request(RequestBody::Stop)?)?;
    until(|| matches!(p.status(), ProcessStatus::Closed))?;
    drop(frames);
    let other = ProcessComponent::spawn(
        ProcessComponentOptions::new("view", 10).with_view(91, 73, native),
        || {},
    )?;
    ready(&other)?;
    ensure!(reply(&other, other.invoke(7)?)? == json!(1));
    ensure!(
        other.take_view_invalidated(),
        "factory hardcoded the first View ID"
    );
    ensure!(matches!(other.status(), ProcessStatus::Ready { .. }));
    reply(&other, other.request(RequestBody::Stop)?)?;
    until(|| matches!(other.status(), ProcessStatus::Closed))?;
    let remounted = ProcessComponent::spawn(
        ProcessComponentOptions::new("view", 10).with_view(1, 2, native),
        || {},
    )?;
    ready(&remounted)?;
    ensure!(
        remounted
            .request_from(
                &old.session,
                RequestBody::Invoke {
                    command: 7,
                    registration: 1,
                    window: 1
                }
            )
            .is_err()
    );
    ensure!(reply(&remounted, remounted.invoke(7)?)? == json!(1));
    reply(&remounted, remounted.request(RequestBody::Stop)?)?;
    until(|| matches!(remounted.status(), ProcessStatus::Closed))?;
    let unnegotiated = ProcessComponent::spawn(
        ProcessComponentOptions::new("view", 10).with_view(
            15,
            16,
            ViewCapabilities {
                primary_pointer: true,
                ..Default::default()
            },
        ),
        || {},
    )?;
    ready(&unnegotiated)?;
    let rejected = unnegotiated.request(RequestBody::Message {
        topic: "local-toolbar".into(),
        payload: Value::Null,
    })?;
    ensure!(reply(&unnegotiated, rejected).is_err());
    ensure!(matches!(
        unnegotiated.toolbar_registration().unwrap().output,
        Output::Event(ComponentEvent::Toolbar {
            scope: ToolbarScope::Window { window: 15 },
            revision: 1,
            ..
        })
    ));
    ensure!(reply(&unnegotiated, unnegotiated.invoke(7)?)? == json!(1));
    reply(&unnegotiated, unnegotiated.request(RequestBody::Stop)?)?;
    until(|| matches!(unnegotiated.status(), ProcessStatus::Closed))?;
    let forged = ProcessComponent::spawn(
        ProcessComponentOptions::new("fault-invalidation", 10).with_view(1, 2, native),
        || {},
    )?;
    until(|| matches!(forged.status(), ProcessStatus::Failed { .. }))?;
    let unsupported = ProcessComponent::spawn(
        ProcessComponentOptions::new("view", 10).with_view(
            1,
            2,
            ViewCapabilities {
                text_ime: true,
                ..Default::default()
            },
        ),
        || {},
    )?;
    until(|| matches!(unsupported.status(), ProcessStatus::Failed { .. }))?;
    headless.close();
    until(|| matches!(headless.status(), ProcessStatus::Closed))?;
    // Exercise the real shared Component mount/arrange/prepare path as a
    // second consumer, rather than another handwritten frame state machine.
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer,
        composition::{Component, Lifecycle},
    };
    use tekes_ui_scene::Size;
    use tekes_ui_tokens::{Rgba, proposal};
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(1., 1.)
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
    let mut container = ProcessContainer::new_view(
        ProcessComponentOptions::new("view", 11).with_view(1, 2, native),
        || {},
    )?;
    ensure!(container.status().is_none(), "spawn before mount");
    container.lifecycle(Lifecycle::Mounted);
    let frame = Rect::new(0., 0., 32., 24.);
    for revision in 0..=1 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            container.prepare(&mut renderer)?;
            container.arrange(&mut Text, frame, frame);
            if container.revision() == Some(revision) {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "Component invalidation did not repaint"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        if revision == 0 {
            ensure!(container.command(7)?);
        }
    }
    let declaration = container.toolbar_registration().unwrap();
    container.set_visible(false)?;
    ensure!(
        container
            .request_from(
                &declaration.session,
                RequestBody::Invoke {
                    command: 7,
                    registration: 1,
                    window: 1
                }
            )
            .is_err()
    );
    let id = container.request(RequestBody::Message {
        topic: "background".into(),
        payload: Value::Null,
    })?;
    let mut got_reply = false;
    until(|| {
        for packet in container.drain_events() {
            if let Output::Response {
                request,
                result: Ok(value),
            } = packet.output
                && request == id
            {
                got_reply = value == json!(2);
            }
        }
        got_reply
    })?;
    ensure!(matches!(
        container.status(),
        Some(ProcessStatus::Ready { .. })
    ));
    container.lifecycle(Lifecycle::Unmounted);
    renderer.collect_retired_images();
    ensure!(container.status().is_none());
    println!(
        "{}",
        json!({"result":"V2_VIEW_PASSED","headless_same_definition":true,"toolbar_before_frame":true,"services_with_two_unretired_frames":true,"gpu_pixel_cases":16,"coalesced_invalidation":true,"stop_with_unretired_frame":true,"unsupported_ime_rejected":true,"standard_component_mount_and_invalidation":true,"window_and_view_scopes":true,"generation_and_remount_owner_rejected":true,"redraw_burst_coalesced":130,"hidden_component_services":true,"unnegotiated_local_scope_rejected":true,"stale_wire_invalidation_rejected":true,"native_desktop":"pending","performance_test":false})
    );
    Ok(())
}
