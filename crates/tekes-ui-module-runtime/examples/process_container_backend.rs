//! Independent public backend consumer; no AppHost/Gallery or desktop window.
//! This verifies extraction, not the pending nonblocking UI container contract.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native module backend requires macOS");
}
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    if blocked_view_fixture()? {
        return Ok(());
    }
    use anyhow::ensure;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tekes_ui_app_protocol::{InputKind, Request, ToolbarItem, VERSION};
    use tekes_ui_module_runtime::{
        ModuleDefinition, ModuleProcess, ModuleState, PanelModule, import_frame,
        run_worker_from_args,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, ImageQuad, Quad, Rect, Scene};
    use tekes_ui_tokens::{Palette, Rgba, Theme};
    static CREATED: AtomicBool = AtomicBool::new(false);
    struct Module(u64, bool);
    impl PanelModule for Module {
        fn update(
            &mut self,
            input: &Request,
            scene: &mut Scene,
            _: &mut GlyphAtlas,
        ) -> anyhow::Result<ModuleState> {
            if input.kind == InputKind::Command && input.command == Some(7) {
                self.0 += 1;
            }
            match input.kind {
                InputKind::PointerDown => self.1 = true,
                InputKind::PointerUp if self.1 => {
                    self.0 += 1;
                    self.1 = false;
                }
                InputKind::Cancel => self.1 = false,
                _ => (),
            }
            let palette = Palette::from(Theme::LIGHT);
            let color = palette
                .color(if self.0 == 0 {
                    "surface-panel"
                } else {
                    "primary-text"
                })
                .unwrap();
            scene.push_quad(Quad::new(
                Rect::new(0., 0., input.width as f32, input.height as f32),
                color,
            ));
            Ok(ModuleState {
                revision: self.0,
                toolbar: vec![ToolbarItem {
                    id: 7,
                    title: "Increment".into(),
                    enabled: true,
                    geometry_dependent: true,
                }],
            })
        }
    }
    fn create() -> Box<dyn PanelModule> {
        CREATED.store(true, Ordering::SeqCst);
        Box::new(Module(0, false))
    }
    let definitions = [ModuleDefinition {
        id: "independent-container-backend",
        create,
    }];
    let args = std::env::args().collect::<Vec<_>>();
    if run_worker_from_args(&definitions, &args)? {
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--queue-review") {
        return verify_queued_view(definitions[0].id);
    }
    if args.get(1).map(String::as_str) == Some("--component-review") {
        return verify_component_view(definitions[0].id, args.get(2).map(String::as_str));
    }
    let cancelled = verify_blocked_view_cancel()?;
    let mut renderer = Renderer::new_headless()?;
    let context = renderer.gpu_context();
    let mut atlas = GlyphAtlas::default();
    let mut left = ModuleProcess::spawn(definitions[0].id)?;
    let mut right = ModuleProcess::spawn(definitions[0].id)?;
    ensure!(
        left.pid() != right.pid(),
        "separate container instances require distinct workers"
    );
    let pids = [left.pid(), right.pid()];
    let mut evidence = vec![];
    for (slot, process) in [&mut left, &mut right].into_iter().enumerate() {
        for seq in 0..3 {
            let scale = if seq == 2 { 2. } else { 1. };
            let command = (slot == 0 && seq == 1).then_some(7);
            let request = Request {
                version: VERSION,
                seq: seq + 1,
                viewport: if seq == 2 { 2 } else { 1 },
                kind: if command.is_some() {
                    InputKind::Command
                } else {
                    InputKind::Layout
                },
                command,
                x: 0.,
                y: 0.,
                width: 80,
                height: 64,
                identity: slot as u32 + 100,
                scale,
            };
            let mut frame = process.request(&request)?;
            let expected_revision = u64::from(slot == 0 && seq > 0);
            ensure!(
                frame.revision == expected_revision,
                "container business state crossed instance boundary"
            );
            ensure!(
                frame.toolbar.len() == 1 && frame.toolbar[0].id == 7,
                "original module command identity"
            );
            let size = (
                (request.width as f32 * scale) as u32,
                (request.height as f32 * scale) as u32,
            );
            ensure!(
                (frame.pixel_width, frame.pixel_height) == size,
                "current native geometry"
            );
            let texture = import_frame(&context, &mut frame)?;
            ensure!(!frame.has_surface(), "import consumes the received right");
            ensure!(
                import_frame(&context, &mut frame).is_err(),
                "a consumed right cannot be imported twice"
            );
            let image = context.import_image(texture)?;
            let image_id = renderer.register_gpu_image(&image)?;
            let rect = Rect::new(0., 0., request.width as f32, request.height as f32);
            let mut scene = Scene::new();
            scene.push_image(ImageQuad {
                rect,
                image: image_id,
                opacity: 1.,
            });
            let actual =
                renderer.render_to_rgba8(&scene, &mut atlas, size, scale, Rgba::gray(1.))?;
            let mut reference = Scene::new();
            let palette = Palette::from(Theme::LIGHT);
            reference.push_quad(Quad::new(
                rect,
                palette
                    .color(if expected_revision == 0 {
                        "surface-panel"
                    } else {
                        "primary-text"
                    })
                    .unwrap(),
            ));
            let expected =
                renderer.render_to_rgba8(&reference, &mut atlas, size, scale, Rgba::gray(1.))?;
            let max_delta = actual
                .iter()
                .zip(&expected)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            ensure!(
                max_delta <= 1,
                "imported GPU frame changed pixels: {max_delta}"
            );
            renderer.unregister_image(image_id);
            evidence.push(serde_json::json!({"slot":slot,"pid":frame.pid,"revision":frame.revision,"seq":frame.seq,"viewport":frame.viewport,"scale":scale,"max_gpu_delta":max_delta,"right_consumed":true}));
        }
    }
    ensure!(
        !CREATED.load(Ordering::SeqCst),
        "module factory ran in parent"
    );
    drop(left);
    drop(right);
    println!(
        "{}",
        serde_json::json!({"scope":"public worker/transport/import extraction; no AppHost/Gallery dependency; not completed ProcessContainer lifecycle or UI acceptance","parent_pid":std::process::id(),"worker_pids":pids,"cases":evidence,"cancellation": cancelled, "parent_factory_called":false,"result":"INDEPENDENT_BACKEND_GPU_OK"})
    );
    Ok(())
}

/// Deliberately broken View worker: receives a request then never returns metadata.
#[cfg(target_os = "macos")]
fn blocked_view_fixture() -> anyhow::Result<bool> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--worker")
        && args.get(2).map(String::as_str) == Some("probe-exit-idle")
    {
        return Ok(true);
    }
    if args.get(1).map(String::as_str) != Some("--worker")
        || args.get(2).map(String::as_str) != Some("probe-blocked-output")
    {
        return Ok(false);
    }
    let mut request = String::new();
    std::io::stdin().read_line(&mut request)?;
    let request: tekes_ui_app_protocol::Request = serde_json::from_str(&request)?;
    let marker =
        std::env::temp_dir().join(format!("tekesui-view-cancel-{}.ready", std::process::id()));
    // Publish readiness atomically; the parent must never parse a partial file.
    let temporary = marker.with_extension("pending");
    std::fs::write(
        &temporary,
        serde_json::to_vec(
            &serde_json::json!({"pid":std::process::id(),"seq":request.seq,"request_received":true}),
        )?,
    )?;
    std::fs::rename(temporary, marker)?;
    loop {
        std::thread::park();
    }
}
#[cfg(target_os = "macos")]
fn verify_blocked_view_cancel() -> anyhow::Result<serde_json::Value> {
    use anyhow::ensure;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    use tekes_ui_module_runtime::{ModuleProcess, ProcessControl};
    let control = ProcessControl::new();
    let worker = control.clone();
    let (sent, done) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let result = (|| -> anyhow::Result<()> {
            let mut process = ModuleProcess::spawn_with_control("probe-blocked-output", worker)?;
            process.request(&tekes_ui_app_protocol::Request {
                version: 1,
                seq: 12345,
                viewport: 1,
                kind: tekes_ui_app_protocol::InputKind::Layout,
                command: None,
                x: 0.,
                y: 0.,
                width: 16,
                height: 16,
                identity: 9,
                scale: 1.,
            })?;
            Ok(())
        })();
        sent.send(result.map_err(|e| e.to_string())).unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let (pid, marker) = loop {
        if let Some(pid) = control.pid() {
            let marker = std::env::temp_dir().join(format!("tekesui-view-cancel-{pid}.ready"));
            if marker.exists() {
                break (pid, marker);
            }
        }
        ensure!(
            Instant::now() < deadline,
            "blocked worker did not receive request"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let evidence: serde_json::Value = serde_json::from_slice(&std::fs::read(&marker)?)?;
    ensure!(evidence["seq"] == 12345 && evidence["pid"] == pid);
    control.cancel();
    ensure!(
        done.recv_timeout(Duration::from_secs(3))?.is_err(),
        "cancelled read unexpectedly succeeded"
    );
    thread.join().unwrap();
    ensure!(control.pid().is_none(), "child not reaped");
    std::fs::remove_file(marker)?;
    let cancelled = ProcessControl::new();
    cancelled.cancel();
    ensure!(
        ModuleProcess::spawn_with_control("probe-blocked-output", cancelled).is_err(),
        "cancel before spawn launched a process"
    );
    Ok(
        serde_json::json!({"blocked_pipe_read":evidence,"cancel_unblocked_owner":true,"reaped":true,"cancel_before_launch":true}),
    )
}

#[cfg(target_os = "macos")]
fn verify_queued_view(module: &str) -> anyhow::Result<()> {
    use anyhow::ensure;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };
    use tekes_ui_app_protocol::{InputKind, Request, VERSION};
    use tekes_ui_module_runtime::{ProcessComponent, ProcessStatus};
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, ImageQuad, Quad, Rect, Scene};
    use tekes_ui_tokens::{Palette, Rgba, Theme};
    fn until(mut predicate: impl FnMut() -> bool) -> anyhow::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !predicate() {
            ensure!(
                Instant::now() < deadline,
                "queued View correctness deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn request(seq: u64, viewport: u64, kind: InputKind) -> Request {
        Request {
            version: VERSION,
            seq,
            viewport,
            kind,
            command: (kind == InputKind::Command).then_some(7),
            x: 0.,
            y: 0.,
            width: 80,
            height: 64,
            identity: 5,
            scale: if viewport == 1 { 1. } else { 2. },
        }
    }
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake = wakes.clone();
    let connection = ProcessComponent::spawn_v1_view(module, 1, move || {
        wake.fetch_add(1, Ordering::Relaxed);
    })?;
    connection.require_view_capabilities(tekes_ui_module_runtime::component::ViewCapabilities {
        primary_pointer: true,
        ..Default::default()
    })?;
    ensure!(
        connection
            .require_view_capabilities(tekes_ui_module_runtime::component::ViewCapabilities {
                text_ime: true,
                ..Default::default()
            })
            .is_err()
    );
    ensure!(
        connection
            .request(tekes_ui_module_runtime::component::RequestBody::Message {
                topic: "unsupported".into(),
                payload: serde_json::Value::Null
            })
            .is_err()
    );
    connection.enqueue_view(request(1, 1, InputKind::Layout))?;
    connection.enqueue_view(request(2, 1, InputKind::Command))?;
    let mut first = Vec::new();
    until(|| {
        first.extend(connection.drain_view_frames());
        first.len() == 2
    })?;
    ensure!(connection.outstanding_view_frames() == 2);
    // Neither layout coalescing nor producer progress may cross this budget.
    for seq in 3..=80 {
        connection.enqueue_view(request(seq, seq, InputKind::Layout))?;
    }
    connection.enqueue_view(request(81, 80, InputKind::Command))?;
    ensure!(
        connection
            .enqueue_view(request(80, 80, InputKind::Cancel))
            .is_err(),
        "stale sequence accepted"
    );
    let mut wrong = request(82, 80, InputKind::Cancel);
    wrong.identity = 6;
    ensure!(
        connection.enqueue_view(wrong).is_err(),
        "identity changed within one instance"
    );
    ensure!(connection.drain_view_frames().is_empty());
    let mut renderer = Renderer::new_headless()?;
    let context = renderer.gpu_context();
    let mut atlas = GlyphAtlas::default();
    let mut cases = Vec::new();
    {
        for frame in first {
            let imported = frame.import(&context)?;
            let metadata = imported.metadata();
            ensure!(
                metadata.seq != 1 || connection.outstanding_view_frames() == 2,
                "CPU import prematurely returned frame budget"
            );
            let image_id = renderer.register_gpu_image(imported.image())?;
            let scale = if metadata.viewport == 1 { 1. } else { 2. };
            let rect = Rect::new(0., 0., 80., 64.);
            let size = ((80. * scale) as u32, (64. * scale) as u32);
            let mut scene = Scene::new();
            scene.push_image(ImageQuad {
                rect,
                image: image_id,
                opacity: 1.,
            });
            let pixels =
                renderer.render_to_rgba8(&scene, &mut atlas, size, scale, Rgba::gray(1.))?;
            let palette = Palette::from(Theme::LIGHT);
            let mut reference = Scene::new();
            reference.push_quad(Quad::new(
                rect,
                palette
                    .color(if metadata.revision == 0 {
                        "surface-panel"
                    } else {
                        "primary-text"
                    })
                    .unwrap(),
            ));
            let expected =
                renderer.render_to_rgba8(&reference, &mut atlas, size, scale, Rgba::gray(1.))?;
            let delta = pixels
                .iter()
                .zip(expected)
                .map(|(a, b)| a.abs_diff(b))
                .max()
                .unwrap();
            ensure!(delta <= 1);
            cases.push(serde_json::json!({"seq":metadata.seq,"viewport":metadata.viewport,"revision":metadata.revision,"max_gpu_delta":delta}));
            renderer.unregister_image(image_id);
            drop(imported);
        }
    }
    let mut next = Vec::new();
    until(|| {
        next.extend(connection.drain_view_frames());
        next.len() == 2
    })?;
    ensure!(
        next[0].metadata().seq == 80 && next[1].metadata().seq == 81,
        "coalescing reordered or removed an action"
    );
    ensure!(next[0].metadata().revision == 1 && next[1].metadata().revision == 2);
    for frame in next {
        let imported = frame.import(&context)?;
        let metadata = imported.metadata();
        let id = renderer.register_gpu_image(imported.image())?;
        let mut scene = Scene::new();
        scene.push_image(ImageQuad {
            rect: Rect::new(0., 0., 80., 64.),
            image: id,
            opacity: 1.,
        });
        let pixels =
            renderer.render_to_rgba8(&scene, &mut atlas, (160, 128), 2., Rgba::gray(1.))?;
        let palette = Palette::from(Theme::LIGHT);
        let mut reference = Scene::new();
        reference.push_quad(Quad::new(
            Rect::new(0., 0., 80., 64.),
            palette.color("primary-text").unwrap(),
        ));
        let expected =
            renderer.render_to_rgba8(&reference, &mut atlas, (160, 128), 2., Rgba::gray(1.))?;
        let delta = pixels
            .iter()
            .zip(expected)
            .map(|(a, b)| a.abs_diff(b))
            .max()
            .unwrap();
        ensure!(delta <= 1);
        cases.push(serde_json::json!({"seq":metadata.seq,"viewport":metadata.viewport,"revision":metadata.revision,"pixel_size":[160,128],"max_gpu_delta":delta}));
        renderer.unregister_image(id);
        drop(imported);
    }
    until(|| connection.outstanding_view_frames() == 0)?;
    connection.close();
    until(|| connection.status() == ProcessStatus::Closed)?;
    let parked = ProcessComponent::spawn_v1_view(module, 1, || {})?;
    parked.enqueue_view(request(1, 1, InputKind::Layout))?;
    parked.enqueue_view(request(2, 1, InputKind::Command))?;
    let mut held = Vec::new();
    until(|| {
        held.extend(parked.drain_view_frames());
        held.len() == 2
    })?;
    for seq in 3..259 {
        parked.enqueue_view(request(seq, 1, InputKind::Command))?;
    }
    ensure!(
        parked
            .enqueue_view(request(259, 1, InputKind::Command))
            .is_err(),
        "unbounded input queue"
    );
    parked.close();
    until(|| parked.status() == ProcessStatus::Closed)?;
    for frame in held {
        ensure!(
            frame.import(&context).is_err(),
            "late frame imported after close"
        );
    }
    let blocked = ProcessComponent::spawn_v1_view("probe-blocked-output", 1, || {})?;
    blocked.enqueue_view(request(12345, 1, InputKind::Layout))?;
    let mut fixture = None;
    until(|| {
        if let ProcessStatus::Ready { pid, .. } = blocked.status() {
            let marker = std::env::temp_dir().join(format!("tekesui-view-cancel-{pid}.ready"));
            if marker.exists() {
                fixture = Some(marker);
                return true;
            }
        }
        false
    })?;
    let marker = fixture.unwrap();
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(&marker)?)?;
    blocked.close();
    until(|| blocked.status() == ProcessStatus::Closed)?;
    std::fs::remove_file(marker)?;
    let idle = ProcessComponent::spawn_v1_view("probe-exit-idle", 2, || {})?;
    until(|| matches!(idle.status(), ProcessStatus::Failed { .. }))?;
    println!(
        "{}",
        serde_json::json!({"result":"QUEUED_VIEW_GPU_OK","unexpected_idle_exit_detected":true,"unsupported_service_and_ime_rejected":true,"cases":cases,"two_frame_budget":true,"cpu_import_does_not_ack":true,"actual_gpu_retirement_releases_budget":true,"layout_coalescing_preserves_command":true,"input_queue_bound":256,"close_with_unretired_frames":true,"late_frame_after_close_rejected":true,"close_blocked_output":fixture,"framework_wakes":wakes.load(Ordering::Relaxed)>0,"scope":"shared V1 queued transport and GPU completion; standard Content/optional View negotiation/native desktop pending","performance_test":false})
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_component_view(module: &str, output: Option<&str>) -> anyhow::Result<()> {
    use anyhow::ensure;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };
    use tekes_ui_controls::{
        ButtonTextAlign, ButtonTextRenderer,
        composition::*,
        extended::{Axis, Stack},
    };
    use tekes_ui_module_runtime::{ProcessContainer, ProcessStatus};
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{GlyphAtlas, Point, Rect, Scene, Size};
    use tekes_ui_tokens::{Rgba, Theme, proposal};
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
    fn container(host: &mut ContentHost) -> &mut ProcessContainer {
        host.get_mut(ComponentId(2))
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
    }
    fn tick(host: &mut ContentHost, renderer: &mut Renderer) -> anyhow::Result<Scene> {
        container(host).prepare(renderer)?;
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut Text,
            Rect::new(10., 12., 80., 64.),
            Rect::new(10., 12., 40., 64.),
            Theme::LIGHT.into(),
        )
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(scene)
    }
    fn until(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        mut predicate: impl FnMut(&mut ProcessContainer) -> bool,
    ) -> anyhow::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            tick(host, renderer)?;
            if predicate(container(host)) {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "component correctness deadline: {:?}",
                container(host)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn screenshot(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        scale: f32,
        output: Option<&str>,
        name: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let scene = tick(host, renderer)?;
        let size = ((100. * scale) as u32, (100. * scale) as u32);
        let pixels = renderer.render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            size,
            scale,
            Rgba::gray(1.),
        )?;
        if let Some(output) = output {
            std::fs::create_dir_all(output)?;
            let path = std::path::Path::new(output).join(format!("{name}.png"));
            let file = std::io::BufWriter::new(std::fs::File::create(path)?);
            let mut encoder = png::Encoder::new(file, size.0, size.1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header()?.write_image_data(&pixels)?;
        }
        Ok(pixels)
    }
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake = wakes.clone();
    let component = ProcessContainer::new_v1(module, 7, move || {
        wake.fetch_add(1, Ordering::Relaxed);
    })?;
    ensure!(
        component.status().is_none(),
        "constructor spawned before mount"
    );
    let root = Content::new(
        ComponentId(1),
        StackContent {
            model: Stack::new(Axis::Vertical),
            children: vec![Content::new(ComponentId(2), component)],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(root).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut renderer = Renderer::new_headless()?;
    until(&mut host, &mut renderer, |c| c.input_ready())?;
    let session = container(&mut host).session().unwrap().clone();
    let initial_pid = match container(&mut host).status().unwrap() {
        ProcessStatus::Ready { pid, .. } => pid,
        _ => anyhow::bail!("not ready"),
    };
    let before = screenshot(&mut host, &mut renderer, 1., output, "before")?;
    ensure!(renderer.image_count() == 1);
    // Real Content routing/primary capture and asynchronous module state update.
    let point = Point::new(20., 20.);
    ensure!(
        !host.event(&ContentEvent::PointerDown(point)).consumed && host.event_bubbled(),
        "legacy preview intercepted default pointer input"
    );
    container(&mut host).set_opaque_primary_input(true);
    let down = host.event(&ContentEvent::PointerDown(point));
    ensure!(
        down.consumed && down.capture_pointer && !host.event_bubbled(),
        "handled input must stop by default"
    );
    ensure!(host.event(&ContentEvent::PointerUp(point)).consumed);
    until(&mut host, &mut renderer, |c| c.revision() == Some(1))?;
    let after = screenshot(&mut host, &mut renderer, 1., output, "after-primary")?;
    ensure!(before != after, "real module did not change pixels");
    let inside = (20 * 100 + 20) * 4;
    let outside = (20 * 100 + 70) * 4;
    ensure!(
        after[inside..inside + 3] != [255, 255, 255],
        "image not drawn inside clip"
    );
    ensure!(
        after[outside..outside + 4] == [255, 255, 255, 255],
        "image escaped Content clip"
    );
    container(&mut host).set_scale(2.)?;
    ensure!(
        !container(&mut host).input_ready(),
        "scale change left old geometry interactive"
    );
    ensure!(!host.event(&ContentEvent::PointerDown(point)).consumed);
    until(&mut host, &mut renderer, |c| c.input_ready())?;
    screenshot(&mut host, &mut renderer, 2., output, "scale-2x")?;
    // Hide preserves the exact process/session/state; removes registrations and
    // cancels an armed gesture before it can complete.
    host.event(&ContentEvent::PointerDown(point));
    container(&mut host).set_visible(false)?;
    tick(&mut host, &mut renderer)?;
    ensure!(!host.event(&ContentEvent::PointerUp(point)).consumed);
    renderer.collect_retired_images();
    ensure!(renderer.image_count() == 0);
    container(&mut host).set_visible(true)?;
    until(&mut host, &mut renderer, |c| c.input_ready())?;
    ensure!(container(&mut host).session() == Some(&session));
    ensure!(
        matches!(container(&mut host).status(),Some(ProcessStatus::Ready{pid,..}) if pid==initial_pid),
        "hide replaced child process"
    );
    ensure!(container(&mut host).revision() == Some(1));
    // Disabled and inactive input cannot change the child state; public command
    // IDs still work after re-enabling, using the same process.
    container(&mut host).set_enabled(false);
    tick(&mut host, &mut renderer)?;
    ensure!(!host.event(&ContentEvent::PointerDown(point)).consumed);
    ensure!(!container(&mut host).command(7)?);
    container(&mut host).set_enabled(true);
    container(&mut host).set_window_active(false);
    host.cancel();
    ensure!(!host.event(&ContentEvent::PointerDown(point)).consumed);
    container(&mut host).set_window_active(true);
    ensure!(container(&mut host).command(7)?);
    until(&mut host, &mut renderer, |c| c.revision() == Some(2))?;
    let pid = match container(&mut host).status().unwrap() {
        ProcessStatus::Ready { pid, .. } => pid,
        _ => anyhow::bail!("worker not ready"),
    };
    let removed = host
        .remove(ComponentId(2))
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let pixels = renderer.render_to_rgba8(
        &Scene::new(),
        &mut GlyphAtlas::default(),
        (100, 100),
        1.,
        Rgba::gray(1.),
    )?;
    ensure!(
        pixels.iter().all(|c| *c == 255) && renderer.image_count() == 0,
        "unmount leaked registration"
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if !std::process::Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "pid="])
            .output()?
            .status
            .success()
        {
            break;
        }
        ensure!(Instant::now() < deadline, "unmounted child not reaped");
        std::thread::sleep(Duration::from_millis(5));
    }
    host.insert(ComponentId(1), removed.bubbling_handled_events(true))
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    until(&mut host, &mut renderer, |c| c.input_ready())?;
    ensure!(container(&mut host).session() != Some(&session));
    ensure!(
        container(&mut host).revision() == Some(0),
        "remount reused closed instance"
    );
    let remount_pid = match container(&mut host).status().unwrap() {
        ProcessStatus::Ready { pid, .. } => pid,
        _ => anyhow::bail!("not ready"),
    };
    ensure!(remount_pid != initial_pid);
    ensure!(
        host.event(&ContentEvent::PointerDown(point)).consumed && host.event_bubbled(),
        "handled bubbling opt-in failed"
    );
    host.cancel();
    drop(host);
    renderer.collect_retired_images();
    ensure!(renderer.image_count() == 0);
    println!(
        "{}",
        serde_json::json!({"result":"STANDARD_CONTENT_V1_CONTAINER_OK","initial_pid":initial_pid,"remount_pid":remount_pid,"session":session,"pid":pid,"mount_only_spawn":true,"default_unhandled_and_configurable_handled_bubbling":true,"original_content_pointer_and_capture":true,"real_pixels_changed":true,"clip_enforced":true,"scale_stale_input_rejected":true,"hide_preserved_process_and_state":true,"hide_cancelled_press":true,"disabled_and_inactive_isolated":true,"original_command_id":7,"unmount_reaped":true,"unmount_and_drop_unregistered":true,"remount_fresh_instance":true,"framework_wakes":wakes.load(Ordering::Relaxed)>0,"scope":"standard Content lifecycle + explicit opaque V1 primary input; V2 consumption/keyboard/IME/AX and native desktop pending","performance_test":false})
    );
    Ok(())
}
