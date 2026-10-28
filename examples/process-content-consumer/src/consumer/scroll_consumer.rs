//! Original nested readers in three real processes. SDK owns all input routing.
use super::*;
use tekes_ui_controls::{
    ReadingController, ReadingDocumentVersion, ScrollArea, ScrollPolicy,
    extended::{Axis, Stack},
};
use tekes_ui_module_runtime::ProcessContainer;
use tekes_ui_scene::Point;
pub(super) const MODULE: &str = "original-content-scroll";
pub(super) const NATIVE_MODULE: &str = "original-content-native-scroll";
const PARENT: ComponentId = ComponentId(20);
const INNER: ComponentId = ComponentId(21);
const BODY: ComponentId = ComponentId(22);
const TAIL: ComponentId = ComponentId(23);
const IDS: [ComponentId; 3] = [ComponentId(481), ComponentId(482), ComponentId(483)];
const CAPS: ViewCapabilities = ViewCapabilities {
    scroll: true,
    ..super::CAPS
};
struct Reader {
    native_accessibility: bool,
    module: Module,
    text: native_host::NativeText,
    version: ReadingDocumentVersion,
    items: Vec<ComponentId>,
}
pub(super) fn create() -> Box<dyn ProcessModule> {
    create_with_accessibility(false)
}
pub(super) fn create_native() -> Box<dyn ProcessModule> {
    create_with_accessibility(true)
}
fn create_with_accessibility(native_accessibility: bool) -> Box<dyn ProcessModule> {
    let mut module = original_module(false);
    let value = (0..60)
        .map(|n| format!("History line {n} 中文\n"))
        .collect::<String>();
    let mut inner =
        TextFieldContent::read_only_content("Original history", &value, proposal::TextRole::UiBody);
    inner.viewport = TextViewport::Grow {
        min: 120.,
        max: 120.,
    };
    inner
        .surface
        .scroll
        .set_scroll_policy(ScrollPolicy::Disabled, ScrollPolicy::Auto);
    let mut reading = ReadingController::default();
    reading
        .set_document(&mut inner.surface, "remote-inner", 1, &value)
        .unwrap();
    reading.follow_latest(&mut inner.surface);
    inner.reading = Some(reading);
    let mut editor = TextFieldContent::new("Message", "draft");
    editor.submit_policy = TextSubmitPolicy::Enter;
    module.host = ContentHost::new(Content::new(
        ComponentId(1),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.,
            },
            flexible_child: Some(PARENT),
            children: vec![
                Content::new(
                    PARENT,
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(
                            BODY,
                            StackContent {
                                model: Stack {
                                    axis: Axis::Vertical,
                                    gap: 4.,
                                },
                                flexible_child: None,
                                children: vec![
                                    Content::new(INNER, inner),
                                    Content::new(TAIL, TextContent::new("After history")),
                                ],
                            },
                        ),
                    ),
                ),
                Content::new(EDIT, editor),
                Content::new(ComponentId(3), Button::new("Auxiliary")).preserving_focus(),
                Content::new(OTHER, TextFieldContent::new("Other", "other")),
                Content::new(
                    ComponentId(5),
                    Popup::new(
                        "Pointer scope",
                        Content::new(ComponentId(6), Button::new("Overlay action")),
                    ),
                ),
            ],
        },
    ))
    .unwrap();
    module.host.set_action_events(true);
    if native_accessibility {
        // Cross-container traversal belongs to the receiving native host.
        module.host.set_focus_wrap(false);
    }
    let items = vec![INNER, TAIL];
    let (version, ()) = module
        .host
        .set_scroll_reading_document(PARENT, "remote-parent", 1, items.clone(), |_| ())
        .unwrap();
    module
        .host
        .get_mut(PARENT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    Box::new(Reader {
        native_accessibility,
        module,
        text: native_host::NativeText::new(),
        version,
        items,
    })
}
impl ProcessModule for Reader {
    fn set_view_context(&mut self, context: &ViewContext) -> Result<()> {
        self.text.apply_context(context)?;
        self.module.revision += 1;
        Ok(())
    }
    fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
        Some((&mut self.module.host, &mut self.text))
    }
    fn view(&mut self) -> Option<&mut dyn PanelModule> {
        Some(self)
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        ViewCapabilities {
            accessibility: self.native_accessibility,
            ..CAPS
        }
    }
    fn started(&mut self, e: &ComponentEmitter) -> Result<()> {
        self.module.started(e)
    }
    fn handle(&mut self, input: RequestBody, e: &ComponentEmitter) -> Result<serde_json::Value> {
        match input {
            RequestBody::Message { topic, .. } if topic == "reading-inspect" => {
                let history = self
                    .module
                    .host
                    .get(INNER)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap();
                let parent = self
                    .module
                    .host
                    .get(PARENT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap();
                let frame = history.surface.inner;
                let edit = self.module.host.get(EDIT).unwrap().frame();
                Ok(
                    serde_json::json!({"pid":std::process::id(),"history_frame":[frame.min_x(),frame.min_y(),frame.size.width,frame.size.height],"edit_frame":[edit.min_x(),edit.min_y(),edit.size.width,edit.size.height],"inner_offset":history.surface.scroll.offset.y,"parent_offset":parent.scroll.offset.y,"following":parent.is_following_latest(),"revision":self.module.revision,"text":self.module.host.get(EDIT).unwrap().component::<TextFieldContent>().unwrap().surface.input.text,"marked":self.module.host.get(EDIT).unwrap().component::<TextFieldContent>().unwrap().surface.input.marked.clone()}),
                )
            }
            RequestBody::Message { topic, .. } if topic == "reading-append" => {
                let id = ComponentId(100 + self.version.revision());
                let mut items = self.items.clone();
                items.push(id);
                let (version, ()) = self
                    .module
                    .host
                    .update_scroll_reading_document(
                        PARENT,
                        &self.version,
                        self.version.revision() + 1,
                        items.clone(),
                        |h| {
                            h.insert(
                                BODY,
                                Content::new(
                                    id,
                                    TextContent::new("Appended original content\n".repeat(40)),
                                ),
                            )
                            .unwrap();
                        },
                    )
                    .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                self.version = version;
                self.items = items;
                self.module.revision += 1;
                e.invalidate_view(self.module.revision)?;
                Ok(serde_json::json!({"revision":self.module.revision}))
            }
            input => self.module.handle(input, e),
        }
    }
}
impl PanelModule for Reader {
    fn update(
        &mut self,
        input: &Request,
        scene: &mut Scene,
        atlas: &mut GlyphAtlas,
    ) -> Result<ModuleState> {
        self.text.scale = input.scale;
        std::mem::swap(atlas, &mut self.text.atlas);
        let frame = Rect::new(0., 0., input.width as f32, input.height as f32);
        let palette = tekes_ui_module_runtime::view_context_palette(&self.text.context)?;
        let rendered = self
            .module
            .host
            .render(scene, &mut self.text, frame, frame, palette);
        std::mem::swap(atlas, &mut self.text.atlas);
        rendered?;
        ensure!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, tekes_ui_scene::Primitive::Glyphs(_))),
            "original child glyphs missing"
        );
        self.module.revision += 1;
        Ok(ModuleState {
            revision: self.module.revision,
            toolbar: vec![],
        })
    }
}
#[derive(Debug)]
struct ReviewRow {
    grid: GridContent,
    arranged: Option<tekes_ui_controls::extended::Grid>,
}
impl Component for ReviewRow {
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(available.width, 400.))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged = None;
        self.grid.arrange(text, frame, clip);
        // This fixed one-row fixture contains only ProcessContainers, whose
        // measure returns the available size. Delegate the real Grid layout,
        // then admit only its finite current cells. Positive translation keeps
        // these relative frames and the inherited clip unchanged.
        let cells = self.grid.model.layout(frame, self.grid.children.len(), 0.);
        let valid = self.grid.model.columns == IDS.len()
            && self.grid.children.len() == IDS.len()
            && [frame, clip].into_iter().all(|r| {
                [
                    r.min_x(),
                    r.min_y(),
                    r.max_x(),
                    r.max_y(),
                    r.size.width,
                    r.size.height,
                ]
                .into_iter()
                .all(f32::is_finite)
                    && r.size.width > 0.
                    && r.size.height > 0.
            })
            && self.grid.children.iter().zip(cells).all(|(child, cell)| {
                child.component::<ProcessContainer>().is_some()
                    && child.frame()
                        == Rect::new(
                            cell.min_x(),
                            frame.min_y(),
                            cell.size.width,
                            frame.size.height,
                        )
            });
        if valid {
            self.arranged = Some(self.grid.model);
        }
    }
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged == Some(self.grid.model)
            && self.grid.children.len() == IDS.len()
            && self
                .grid
                .children
                .iter()
                .all(|child| child.component::<ProcessContainer>().is_some())
    }
    fn children(&self) -> &[Content] {
        self.grid.children()
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.grid.children_mut()
    }
}
fn container(h: &ContentHost, id: ComponentId) -> &ProcessContainer {
    h.get(id).unwrap().component::<ProcessContainer>().unwrap()
}
fn ready(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer) -> Result<()> {
    ready_at(h, gpu, Rect::new(0., 0., 960., 420.))
}
fn ready_at(h: &mut ContentHost, gpu: &mut tekes_ui_render::Renderer, frame: Rect) -> Result<()> {
    ready_revision(h, gpu, frame, None)
}
fn ready_revision(
    h: &mut ContentHost,
    gpu: &mut tekes_ui_render::Renderer,
    frame: Rect,
    min: Option<(ComponentId, u64)>,
) -> Result<()> {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        for id in IDS {
            h.get_mut(id)
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare(gpu)?;
        }
        h.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )?;
        for id in IDS {
            ensure!(
                container(h, id).error().is_none(),
                "scroll worker error {:?}",
                container(h, id).error()
            );
        }
        if IDS.iter().all(|id| container(h, *id).input_ready())
            && min.is_none_or(|(id, revision)| {
                container(h, id)
                    .revision()
                    .is_some_and(|actual| actual >= revision)
            })
        {
            return Ok(());
        }
        ensure!(
            Instant::now() < end,
            "scroll layout acknowledgement deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn point(value: &serde_json::Value, field: &str, origin: Point) -> Point {
    let r = &value[field];
    Point::new(
        origin.x + r[0].as_f64().unwrap() as f32 + r[2].as_f64().unwrap() as f32 * 0.5,
        origin.y + r[1].as_f64().unwrap() as f32 + r[3].as_f64().unwrap() as f32 * 0.5,
    )
}
fn rejects_unnegotiated(gpu: &mut tekes_ui_render::Renderer) -> Result<serde_json::Value> {
    let p = ProcessComponent::spawn(
        ProcessComponentOptions::new(super::MODULE, 482).with_view(48, 999, super::CAPS),
        || {},
    )?;
    wait(|| matches!(p.status(), ProcessStatus::Ready { .. }))?;
    layout(&p, 1, 1, 300)?;
    let frame = Rect::new(0., 0., 300., 300.);
    let options = tekes_ui_module_runtime::NativeViewOptions {
        window: 48,
        view: 999,
        required: super::CAPS,
    };
    let bridge = tekes_ui_module_runtime::ProcessContentBridge::new(
        &p,
        &options,
        1,
        frame,
        frame,
        Duration::from_secs(1),
    )?;
    let before = bridge.snapshot()?;
    let error = bridge
        .request(
            &before,
            wire::Input::Scroll(wire::ScrollEvent {
                position: [40., 40.],
                delta: [0., -12.],
                precise: true,
                begin: true,
                end: false,
                momentum: false,
                time: 1.,
            }),
        )
        .err()
        .context("unnegotiated Scroll accepted")?;
    ensure!(
        error
            .to_string()
            .contains("scroll capability not negotiated")
    );
    let after = bridge.snapshot()?;
    ensure!(
        before.reply().focused == after.reply().focused
            && before.reply().focus_epoch == after.reply().focus_epoch
    );
    p.close();
    wait(|| matches!(p.status(), ProcessStatus::Closed))?;
    let id = ComponentId(990);
    let mut host = ContentHost::new(Content::new(
        ComponentId(989),
        ListContent::new(
            "unsupported wheel",
            vec![Content::new(
                id,
                ProcessContainer::new_view(
                    ProcessComponentOptions::new(super::MODULE, 483).with_view(
                        48,
                        990,
                        super::CAPS,
                    ),
                    || {},
                )?,
            )],
        ),
    ))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        host.get_mut(id)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .prepare(gpu)?;
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )?;
        if container(&host, id).input_ready() {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "unsupported fixture layout deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid = match container(&host, id).status() {
        Some(ProcessStatus::Ready { pid, .. }) => pid,
        _ => anyhow::bail!("unsupported fixture not ready"),
    };
    ensure!(host.focus(Some(id), true));
    host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let text = host.text_input_snapshot();
    let response = host.event(&ContentEvent::Scroll {
        position: Point::new(40., 40.),
        delta: Point::new(0., -12.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    ensure!(
        !response.consumed && host.event_bubbled(),
        "unsupported wheel leaked into focused text fallback"
    );
    ensure!(
        host.text_input_snapshot() == text && container(&host, id).error().is_none(),
        "unsupported wheel cancelled editor or failed actor"
    );
    host.remove(id)?;
    wait(|| {
        !std::process::Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })?;
    Ok(
        serde_json::json!({"unnegotiated_scroll_rejected":true,"actor_survived":true,"process_closed":true,"unsupported_container_wheel_bubbled_without_text_rpc_or_composition_cancel":true}),
    )
}
fn capture(
    h: &mut ContentHost,
    gpu: &mut tekes_ui_render::Renderer,
    out: Option<&std::path::Path>,
    file: &str,
) -> Result<()> {
    let frame = Rect::new(0., 0., 960., 420.);
    let mut scene = Scene::new();
    h.render(&mut scene, &mut Text, frame, frame, Theme::LIGHT.into())?;
    let pixels = gpu.render_to_rgba8(
        &scene,
        &mut GlyphAtlas::default(),
        (960, 420),
        1.,
        Rgba::gray(1.),
    )?;
    ensure!(
        pixels.chunks_exact(4).any(|p| p != &pixels[..4]),
        "shared original child image pixels missing"
    );
    if let Some(out) = out {
        let target = out.join(file);
        assert!(!target.exists());
        let mut ppm = b"P6\n960 420\n255\n".to_vec();
        for pixel in pixels.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(target, ppm)?;
    }
    Ok(())
}
pub(super) fn roundtrip() -> Result<serde_json::Value> {
    let children = IDS
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut c = ProcessContainer::new_view(
                ProcessComponentOptions::new(MODULE, 481).with_view(48, 481 + i as u64, CAPS),
                || {},
            )?;
            c.set_view_context(ViewContext {
                reduced_motion: true,
                caret_visible: false,
                ..Default::default()
            })?;
            Ok(Content::new(*id, c))
        })
        .collect::<Result<Vec<_>>>()?;
    let row = Content::new(
        ComponentId(480),
        ReviewRow {
            arranged: None,
            grid: GridContent {
                model: tekes_ui_controls::extended::Grid::new(3),
                children,
            },
        },
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(479),
        ScrollContent::new(ScrollArea::default(), row),
    ))?;
    host.set_scroll_reading_document(
        ComponentId(479),
        "outer-reader",
        1,
        vec![ComponentId(480)],
        |_| (),
    )
    .unwrap();
    host.get_mut(ComponentId(479))
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    let mut gpu = tekes_ui_render::Renderer::new_headless()?;
    ready(&mut host, &mut gpu)?;
    let output = std::env::var_os("TEKESUI_REMOTE_SCROLL_GPU_OUT").map(std::path::PathBuf::from);
    if let Some(out) = &output {
        ensure!(!out.exists(), "fresh evidence directory required");
        std::fs::create_dir_all(out)?;
    }
    let mut results = vec![];
    for id in IDS {
        let before = container_message(container(&host, id), "reading-inspect")?;
        let origin = host.get(id).unwrap().frame().origin;
        let edit = point(&before, "edit_frame", origin);
        ensure!(host.event(&ContentEvent::PointerDown(edit)).capture_pointer);
        host.event(&ContentEvent::PointerUp(edit));
        host.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        });
        ready(&mut host, &mut gpu)?;
        let old = host.text_input_snapshot();
        let epoch = host.focus_epoch();
        let focus = host.focused();
        let states: Vec<_> = IDS
            .iter()
            .map(|id| container_message(container(&host, *id), "reading-inspect"))
            .collect::<Result<_>>()?;
        let before = container_message(container(&host, id), "reading-inspect")?;
        let p = point(&before, "history_frame", origin);
        let native = tekes_ui_platform_macos::Event::Scroll(tekes_ui_platform_macos::ScrollEvent {
            position: tekes_ui_platform_macos::Point {
                x: p.x.into(),
                y: p.y.into(),
            },
            delta: tekes_ui_platform_macos::Vec2 { x: 0., y: 16. },
            precise: true,
            phase: tekes_ui_platform_macos::ScrollPhase::Began,
            momentum_phase: tekes_ui_platform_macos::ScrollPhase::None,
            inverted: true,
            modifiers: tekes_ui_platform_macos::Modifiers::NONE,
            timestamp: 1.,
        });
        let event = tekes_ui_module_runtime::content_event(&native)
            .context("native scroll was not projected")?;
        let outer_offset = host
            .get(ComponentId(479))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset;
        let response = host.event(&event);
        ensure!(
            response.consumed && !host.event_bubbled(),
            "remote child wheel disposition lost"
        );
        let outer = host
            .get(ComponentId(479))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap();
        ensure!(
            !outer.is_following_latest() && outer.scroll.offset == outer_offset,
            "remote original reading intent failed to reach outer reader or double-scrolled it"
        );
        ensure!(
            host.reading_scroll_source()
                .is_some_and(|(source, _)| source == id),
            "source observation not bound to original mounted container"
        );
        ensure!(
            host.focused() == focus
                && host.focus_epoch() == epoch
                && host.text_input_snapshot() == old,
            "wheel changed original composer lease/preedit"
        );
        let mut after = container_message(container(&host, id), "reading-inspect")?;
        ensure!(
            before["inner_offset"].as_f64().unwrap() - after["inner_offset"].as_f64().unwrap()
                == 16.,
            "wheel not delivered exactly once: before={before}, after={after}"
        );
        ensure!(
            after["following"] == false && after["parent_offset"] == before["parent_offset"],
            "original child nested reading notification or parent offset wrong"
        );
        for (other, state) in IDS.iter().zip(&states) {
            if *other != id {
                ensure!(
                    container_message(container(&host, *other), "reading-inspect")?["inner_offset"]
                        == state["inner_offset"],
                    "wheel reached sibling process"
                );
            }
        }
        let prior_momentum = after["inner_offset"].as_f64().unwrap();
        ensure!(
            host.event(&ContentEvent::Scroll {
                position: p,
                delta: Point::new(0., -4.),
                precise: true,
                begin: false,
                end: false,
                momentum: true,
                time: 1.1
            })
            .consumed
        );
        after = container_message(container(&host, id), "reading-inspect")?;
        ensure!(
            prior_momentum - after["inner_offset"].as_f64().unwrap() == 4.,
            "momentum phase lost original delta/owner"
        );
        ensure!(host.focused() == focus && host.text_input_snapshot() == old);
        let append_result = container_message(container(&host, id), "reading-append")?;
        ready_revision(
            &mut host,
            &mut gpu,
            Rect::new(0., 0., 960., 420.),
            Some((id, append_result["revision"].as_u64().unwrap())),
        )?;
        let appended = container_message(container(&host, id), "reading-inspect")?;
        ensure!(
            appended["following"] == false
                && appended["inner_offset"] == after["inner_offset"]
                && appended["parent_offset"] == after["parent_offset"],
            "append stole reading anchor"
        );
        ensure!(
            host.text_input_snapshot() == old,
            "background append interrupted composition"
        );
        capture(
            &mut host,
            &mut gpu,
            output.as_deref(),
            &format!("reader-{}-preedit-append.ppm", id.0),
        )?;
        let neighbor = IDS[(IDS.iter().position(|v| *v == id).unwrap() + 1) % IDS.len()];
        let neighbor_before = container_message(container(&host, neighbor), "reading-inspect")?;
        let neighbor_point = point(
            &neighbor_before,
            "history_frame",
            host.get(neighbor).unwrap().frame().origin,
        );
        ensure!(
            host.event(&ContentEvent::Scroll {
                position: neighbor_point,
                delta: Point::new(0., -8.),
                precise: true,
                begin: true,
                end: false,
                momentum: false,
                time: 2.
            })
            .consumed
        );
        let neighbor_after = container_message(container(&host, neighbor), "reading-inspect")?;
        ensure!(
            neighbor_before["inner_offset"].as_f64().unwrap()
                - neighbor_after["inner_offset"].as_f64().unwrap()
                == 8.,
            "unfocused container wheel was not delivered once"
        );
        ensure!(
            host.focused() == focus
                && host.focus_epoch() == epoch
                && host.text_input_snapshot() == old,
            "scrolling another process interrupted composer"
        );
        let reveal_producer_revision = container(&host, id).revision().unwrap() + 1;
        let c = container(&host, id);
        let bridge = c.native_content_bridge(Duration::from_secs(1))?;
        let snapshot = bridge.snapshot()?;
        ensure!(
            snapshot.reply().reading_scroll_source.is_none(),
            "Snapshot replayed old reading notification"
        );
        // Public SDK reading/reveal calls, not business Message commands or offset math.
        let query = bridge.reading_state(&snapshot, PARENT.0)?;
        ensure!(
            !query.reply().changed,
            "reading query created paint invalidation"
        );
        let reading = query
            .reply()
            .reading
            .as_ref()
            .context("original reading identity missing")?;
        let target = *reading
            .items
            .iter()
            .find(|item| item.id == TAIL.0)
            .context("declared tail missing")?;
        let options = wire::RevealOptions {
            placement: wire::RevealPlacement::StartVertical,
            margins: [0.; 4],
            animated: false,
        };
        let reveal = bridge.reveal_reading_item(&query, target, options)?;
        ensure!(
            reveal.reply().text == query.reply().text
                && reveal.reply().focused == query.reply().focused
                && reveal.reply().focus_epoch == query.reply().focus_epoch,
            "reveal interrupted original composer"
        );
        ensure!(
            matches!(
                reveal
                    .reply()
                    .reading
                    .as_ref()
                    .and_then(|r| r.navigation.as_ref())
                    .map(|n| &n.status),
                Some(wire::RevealStatus::Completed)
            ),
            "original reveal did not complete"
        );
        let revealed_state = container_message(c, "reading-inspect")?;
        ensure!(
            revealed_state["parent_offset"].as_f64().unwrap()
                > appended["parent_offset"].as_f64().unwrap(),
            "reveal fixture did not actually scroll original parent"
        );
        ensure!(
            revealed_state["inner_offset"] == appended["inner_offset"],
            "reveal changed original detail offset"
        );
        let mut stale = reading.clone();
        stale.document.attachment += 1;
        ensure!(
            bridge
                .request(
                    &reveal,
                    wire::Input::RevealReadingItem {
                        reader: stale.reader,
                        document: stale.document.clone(),
                        item: target,
                        options
                    }
                )
                .is_err(),
            "stale reading attachment accepted"
        );
        let mut replaced = target;
        replaced.mount += 1;
        ensure!(
            bridge
                .reveal_reading_item(&reveal, replaced, options)
                .is_err(),
            "replacement item accepted"
        );
        ensure!(
            bridge
                .reveal_reading_item(
                    &reveal,
                    target,
                    wire::RevealOptions {
                        margins: [f32::NAN; 4],
                        ..options
                    }
                )
                .is_err(),
            "nonfinite margins accepted"
        );
        let other_bridge =
            container(&host, neighbor).native_content_bridge(Duration::from_secs(1))?;
        let other_snapshot = other_bridge.snapshot()?;
        let other_reading = other_bridge.reading_state(&other_snapshot, PARENT.0)?;
        ensure!(
            bridge
                .reveal_reading_item(&other_reading, target, options)
                .is_err(),
            "sibling process reading snapshot accepted"
        );
        ensure!(
            container_message(c, "reading-inspect")?["parent_offset"]
                == revealed_state["parent_offset"],
            "rejected reveal changed original offset"
        );
        let queried = bridge.reading_state(&reveal, PARENT.0)?;
        ensure!(
            queried.reply().text == reveal.reply().text,
            "rejected reveal cancelled preedit"
        );
        let raw = wire::Request {
            view: snapshot.reply().view,
            viewport: snapshot.reply().viewport,
            owner: snapshot.reply().focused,
            focus_epoch: snapshot.reply().focus_epoch,
            text_binding: snapshot.reply().text_binding.clone(),
            capture: None,
            input: wire::Input::Scroll(wire::ScrollEvent {
                position: [20., 20.],
                delta: [0., f32::NAN],
                precise: true,
                begin: true,
                end: false,
                momentum: false,
                time: 1.,
            }),
        };
        ensure!(
            c.request(RequestBody::Content(raw)).is_err(),
            "nonfinite scroll entered JSON/worker lane"
        );
        ensure!(
            bridge.snapshot()?.reply().focus_epoch == snapshot.reply().focus_epoch,
            "invalid packet disturbed actor"
        );
        ready_revision(
            &mut host,
            &mut gpu,
            Rect::new(0., 0., 960., 420.),
            Some((id, reveal_producer_revision)),
        )?;
        capture(
            &mut host,
            &mut gpu,
            output.as_deref(),
            &format!("reader-{}-public-reveal.ppm", id.0),
        )?;
        host.event(&ContentEvent::Commit {
            text: "你".into(),
            replacement: None,
        });
        host.event(&ContentEvent::Unmark);
        let committed = container_message(container(&host, id), "reading-inspect")?;
        ensure!(
            committed["marked"].is_null()
                && committed["text"].as_str().is_some_and(|s| s.contains('你'))
        );
        ensure!(
            inspect_container(container(&host, id))?["submits"] == 0,
            "scroll/append submitted business action"
        );
        results.push(serde_json::json!({"container":id.0,"session":container(&host,id).session(),"before":before,"after":after,"appended":appended,"revealed":revealed_state,"public_reveal_preserved_preedit":true,"reveal_rejected_stale_attachment_replacement_sibling_snapshot_and_nonfinite_margin":true}));
        // Restore the next independent wheel scenario through the original
        // item API, never by assigning the SDK's offset in a consumer.
        let restore_revision = container(&host, id).revision().unwrap() + 1;
        {
            let original = container(&host, id).native_content_bridge(Duration::from_secs(1))?;
            let current = original.snapshot()?;
            let reading = original.reading_state(&current, PARENT.0)?;
            let first = *reading
                .reply()
                .reading
                .as_ref()
                .unwrap()
                .items
                .iter()
                .find(|item| item.id == INNER.0)
                .unwrap();
            original.reveal_reading_item(&reading, first, options)?;
        }
        ready_revision(
            &mut host,
            &mut gpu,
            Rect::new(0., 0., 960., 420.),
            Some((id, restore_revision)),
        )?;
    }
    let pids: std::collections::BTreeSet<_> = results
        .iter()
        .map(|r| r["after"]["pid"].as_u64().unwrap())
        .collect();
    ensure!(pids.len() == 3);
    let retired = container(&host, IDS[0])
        .native_content_bridge(Duration::from_secs(1))?
        .snapshot()?;
    host.get_mut(ComponentId(479))
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    let resized = Rect::new(0., 0., 900., 420.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        resized,
        resized,
        Theme::LIGHT.into(),
    )?;
    let p = host.get(IDS[0]).unwrap().frame().center();
    ensure!(
        host.event(&ContentEvent::Scroll {
            position: p,
            delta: Point::new(0., -12.),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time: 4.
        })
        .consumed
    );
    ensure!(
        host.reading_scroll_source().is_none(),
        "pending geometry reused old accepted reading observation"
    );
    ensure!(
        host.get(ComponentId(479))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .is_following_latest(),
        "geometry barrier paused original parent"
    );
    ready_at(&mut host, &mut gpu, resized)?;
    let fresh = container(&host, IDS[0]).native_content_bridge(Duration::from_secs(1))?;
    ensure!(
        fresh
            .request(
                &retired,
                wire::Input::Scroll(wire::ScrollEvent {
                    position: [20., 20.],
                    delta: [0., 12.],
                    precise: true,
                    begin: true,
                    end: false,
                    momentum: false,
                    time: 5.
                })
            )
            .is_err(),
        "old viewport scroll survived resize"
    );
    let negative = rejects_unnegotiated(&mut gpu)?;
    for id in IDS {
        host.remove(id)?;
    }
    wait(|| {
        pids.iter().all(|pid| {
            !std::process::Command::new("/bin/kill")
                .args(["-0", &pid.to_string()])
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        })
    })?;
    Ok(
        serde_json::json!({"result":"REMOTE_ORIGINAL_SCROLL_OK","unnegotiated":negative,"unfocused_other_container_kept_composer":true,"stale_resize_callback_rejected":true,"invalid_packet_rejected_before_serialization":true,"geometry_barrier_did_not_pause_reader":true,"all_original_children_reaped":true,"real_glyph_gpu_frames":6,"public_reading_query_and_actual_item_reveal":true,"original_momentum_preserved":true,"producer_revision_acknowledged":true,"runs":results,"real_child_pids":pids,"scope":"original mixed Content, composer preedit and nested reader in three actual IPC workers and GPU imported images; not physical IME, native window, child AX, Gold7","outer_original_reading_intent":true,"outer_offset_not_redispatched":true,"native":false,"physical_ime":false,"voiceover":false,"gold7":false}),
    )
}

#[cfg(test)]
mod native_fixture_tests {
    use super::*;

    #[test]
    fn native_mixed_reader_uses_original_components_and_explicit_capabilities() {
        let original = create();
        assert!(original.view_capabilities().scroll);
        assert!(!original.view_capabilities().accessibility);
        let mut native = create_native();
        let caps = native.view_capabilities();
        assert!(
            caps.keyboard
                && caps.text_ime
                && caps.content_pointer
                && caps.scroll
                && caps.accessibility
        );
        assert!(
            !caps.primary_pointer,
            "mixed native acceptance must not require an opaque pointer sink"
        );
        let (host, text) = native
            .content()
            .expect("original Content and native font backend");
        let frame = Rect::new(0., 0., 320., 500.);
        host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        assert!(
            host.get(PARENT)
                .unwrap()
                .component::<ScrollContent>()
                .is_some()
        );
        let history = host
            .get(INNER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert!(history.reading.is_some());
        assert!(history.surface.input.text.contains("History line 59 中文"));
        for id in [EDIT, OTHER] {
            assert!(
                host.get(id)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .is_some()
            );
        }
        assert!(
            host.get(ComponentId(3))
                .unwrap()
                .component::<Button>()
                .is_some()
        );
    }
}
