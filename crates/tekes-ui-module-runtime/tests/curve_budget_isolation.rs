//! Issue #428: Isolate Curve read-only query transport budget errors so the
//! worker remains alive and subsequent valid queries in the same session succeed.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, Result, bail, ensure};
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};
    use tekes_ui_app_protocol::Request;
    use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
    use tekes_ui_module_runtime::{component::*, *};
    use tekes_ui_scene::{GlyphAtlas, Rect, Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    const TEXT_ID: ComponentId = ComponentId(10);
    const CAPS: ViewCapabilities = ViewCapabilities {
        accessibility: true,
        primary_pointer: false,
        content_pointer: false,
        keyboard: false,
        scroll: false,
        text_ime: false,
        independent_toolbar: false,
    };

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 8.).min(w), 20.)
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

    struct Child {
        host: ContentHost,
        text: Text,
        revision: u64,
    }
    impl PanelModule for Child {
        fn update(
            &mut self,
            r: &Request,
            s: &mut Scene,
            _: &mut GlyphAtlas,
        ) -> Result<ModuleState> {
            let f = Rect::new(0., 0., r.width as f32, r.height as f32);
            self.host
                .render(s, &mut self.text, f, f, Theme::LIGHT.into())?;
            Ok(ModuleState {
                revision: self.revision,
                toolbar: vec![],
            })
        }
    }
    impl ProcessModule for Child {
        fn view(&mut self) -> Option<&mut dyn PanelModule> {
            Some(self)
        }
        fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
            Some((&mut self.host, &mut self.text))
        }
        fn view_capabilities(&self) -> ViewCapabilities {
            CAPS
        }
        fn handle(&mut self, r: RequestBody, e: &ComponentEmitter) -> Result<Value> {
            let RequestBody::Message { topic, .. } = r else {
                bail!("unsupported")
            };
            match topic.as_str() {
                "inspect" => Ok(json!({ "pid": std::process::id() })),
                "oversize" => {
                    self.host
                        .get_mut(TEXT_ID)
                        .and_then(|n| n.component_mut::<TextContent>())
                        .context("missing TextContent")?
                        .set_text("x".repeat(600_000));
                    self.revision += 1;
                    e.invalidate_view(self.revision)?;
                    Ok(json!("oversized"))
                }
                "shrink" => {
                    self.host
                        .get_mut(TEXT_ID)
                        .and_then(|n| n.component_mut::<TextContent>())
                        .context("missing TextContent")?
                        .set_text("recovered");
                    self.revision += 1;
                    e.invalidate_view(self.revision)?;
                    Ok(json!("shrunk"))
                }
                other => bail!("unknown topic {other}"),
            }
        }
    }

    fn create_worker() -> Box<dyn ProcessModule> {
        let host = ContentHost::new(Content::new(TEXT_ID, TextContent::new("initial-small")))
            .expect("host");
        Box::new(Child {
            host,
            text: Text,
            revision: 1,
        })
    }

    let args = std::env::args().collect::<Vec<_>>();
    let caps = Capabilities {
        view: true,
        ..Capabilities::HEADLESS
    };
    let defs = [ComponentDefinition {
        id: "curve-budget",
        capabilities: caps,
        create: create_worker,
    }];
    if run_component_worker_from_args(&defs, &args)? {
        return Ok(());
    }

    fn container(h: &ContentHost) -> &ProcessContainer {
        h.get(ComponentId(50))
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
    }
    fn container_mut(h: &mut ContentHost) -> &mut ProcessContainer {
        h.get_mut(ComponentId(50))
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
    }
    fn message(c: &ProcessContainer, topic: &str) -> Result<Value> {
        let id = c.request(RequestBody::Message {
            topic: topic.into(),
            payload: Value::Null,
        })?;
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            for p in c.drain_events() {
                if let Output::Response { request, result } = p.output
                    && request == id
                {
                    return result.map_err(anyhow::Error::msg);
                }
            }
            ensure!(Instant::now() < end, "message deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    let mut container_init = ProcessContainer::new_view(
        ProcessComponentOptions::new("curve-budget", 75).with_view(75, 1, CAPS),
        || {},
    )?;
    container_init.begin_curve_preflight()?;
    let end = Instant::now() + Duration::from_secs(15);
    while !container_init.curve_preflight_ready()? {
        ensure!(Instant::now() < end, "curve preflight deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
    let panel = PanelContent::new(Content::new(ComponentId(50), container_init))?;
    let mut host = ContentHost::new(Content::new(ComponentId(100), panel))?;

    let frame = Rect::new(0., 0., 240., 160.);
    let mut need_layout = true;
    loop {
        let events = container(&host).drain_curve_host_events();
        let host_update = !events.is_empty() || container(&host).curve_requires_host_update();
        if host_update {
            container_mut(&mut host).prepare_curve_host_update()?;
        }
        if need_layout || host_update || host.needs_render() {
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            need_layout = false;
        }
        let prepared = container(&host)
            .frame_resource_handle()?
            .prepare_current_frame(&mut gpu)?;
        if prepared {
            host.repaint_current(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
        }
        if let Some(err) = container(&host).error() {
            bail!("container error before ready: {err}");
        }
        if container(&host).input_ready() {
            break;
        }
        ensure!(Instant::now() < end, "curve ready deadline");
        std::thread::sleep(Duration::from_millis(5));
    }

    let pid_before = message(container(&host), "inspect")?["pid"]
        .as_u64()
        .context("missing pid")?;
    ensure!(
        pid_before != std::process::id() as u64,
        "expected out-of-process worker"
    );
    ensure!(
        matches!(
            container(&host).status(),
            Some(ProcessStatus::Ready { pid, .. }) if pid as u64 == pid_before
        ),
        "unexpected worker status: {:?}",
        container(&host).status()
    );

    let initial_snap = container(&host)
        .native_content_bridge(Duration::from_secs(2))?
        .semantics()?;
    let initial_stamp = initial_snap
        .curve_stamp()
        .cloned()
        .context("expected active Curve QueryStamp pairing frame and semantics")?;
    ensure!(
        initial_snap
            .reply()
            .semantics
            .as_ref()
            .and_then(|nodes| nodes.first())
            .is_some_and(|n| n.semantics.name == "initial-small"),
        "unexpected initial semantics"
    );

    // Capability auth remains enforced: scroll/reading is unnegotiated.
    let auth_err = container(&host)
        .native_content_bridge(Duration::from_secs(2))?
        .reading_state(&initial_snap, TEXT_ID.0)
        .expect_err("unnegotiated reading_state must be rejected");
    ensure!(
        auth_err.to_string().contains("not negotiated")
            || auth_err.to_string().contains("CapabilityNotNegotiated")
            || auth_err.to_string().contains("without scroll"),
        "unexpected auth rejection: {auth_err:#}"
    );

    // Trigger an over-budget semantic snapshot (> MAX_PACKET / 2) on the worker,
    // pump the next Curve frame, and verify it isolates cleanly to LimitExceeded.
    ensure!(message(container(&host), "oversize")? == json!("oversized"));
    let oversize_deadline = Instant::now() + Duration::from_secs(15);
    let mut oversize_need_layout = false;
    let oversize_msg = loop {
        let events = container(&host).drain_curve_host_events();
        let host_update = !events.is_empty() || container(&host).curve_requires_host_update();
        if host_update && container_mut(&mut host).prepare_curve_host_update()? {
            oversize_need_layout = true;
        }
        if oversize_need_layout || host.needs_render() {
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            oversize_need_layout = false;
        }
        if let Ok(handle) = container(&host).frame_resource_handle() {
            match handle.prepare_current_frame(&mut gpu) {
                Ok(true) => bail!("unexpected success on oversized Curve ContentQuery"),
                Ok(false) => (),
                Err(err) => break format!("{err:#}"),
            }
        }
        ensure!(
            Instant::now() < oversize_deadline,
            "oversized Curve query deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    ensure!(
        oversize_msg.contains("LimitExceeded"),
        "expected CurveErrorCode::LimitExceeded rejection, got: {oversize_msg}"
    );

    // The exact same worker process and session remain alive and ready.
    ensure!(
        container(&host).error().is_none(),
        "over-budget query error contaminated ProcessContainer error state: {:?}",
        container(&host).error()
    );
    ensure!(
        matches!(
            container(&host).status(),
            Some(ProcessStatus::Ready { pid, .. }) if pid as u64 == pid_before
        ),
        "worker terminated or changed status: {:?}",
        container(&host).status()
    );
    let pid_after_err = message(container(&host), "inspect")?["pid"]
        .as_u64()
        .context("missing pid after error")?;
    ensure!(pid_after_err == pid_before);

    // Shrink the semantic payload back inside budget on the same worker/session,
    // pump the next frame + paired semantics, and confirm recovery on the same PID.
    ensure!(message(container(&host), "shrink")? == json!("shrunk"));
    let recover_deadline = Instant::now() + Duration::from_secs(15);
    let mut recover_need_layout = false;
    loop {
        let events = container(&host).drain_curve_host_events();
        let host_update = !events.is_empty() || container(&host).curve_requires_host_update();
        if host_update && container_mut(&mut host).prepare_curve_host_update()? {
            recover_need_layout = true;
        }
        if recover_need_layout || host.needs_render() {
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            recover_need_layout = false;
        }
        let prepared = container(&host)
            .frame_resource_handle()?
            .prepare_current_frame(&mut gpu)?;
        if prepared {
            host.repaint_current(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            if container(&host)
                .semantic_parts()
                .iter()
                .any(|p| p.semantics.name == "recovered")
            {
                break;
            }
        }
        if let Some(err) = container(&host).error() {
            bail!("container error during recovery: {err}");
        }
        ensure!(Instant::now() < recover_deadline, "curve recovery deadline");
        std::thread::sleep(Duration::from_millis(5));
    }

    let recovered_snap = container(&host)
        .native_content_bridge(Duration::from_secs(2))?
        .semantics()?;
    let recovered_stamp = recovered_snap
        .curve_stamp()
        .cloned()
        .context("missing Curve QueryStamp after recovery")?;
    ensure!(recovered_stamp.published_frame_seq > initial_stamp.published_frame_seq);
    let recovered_name = recovered_snap
        .reply()
        .semantics
        .as_ref()
        .and_then(|nodes| nodes.first())
        .map(|n| n.semantics.name.as_str());
    ensure!(recovered_name == Some("recovered"));
    let pid_final = message(container(&host), "inspect")?["pid"]
        .as_u64()
        .context("missing pid after recovery")?;
    ensure!(pid_final == pid_before);

    println!(
        "CURVE_BUDGET_ISOLATION worker_pid={} auth_err={:?} oversize_err={:?} recovered_label={:?} initial_stamp={:?} recovered_stamp={:?}",
        pid_before,
        auth_err.to_string(),
        oversize_msg,
        recovered_name,
        initial_stamp,
        recovered_stamp
    );
    Ok(())
}
