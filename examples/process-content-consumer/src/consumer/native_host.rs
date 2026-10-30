//! Original fixture with real glyph rendering and one native window adapter.
use super::*;
use tekes_ui_controls::{TextSurfaceHost, TextSurfaceTextRenderer};
use tekes_ui_platform_macos as native;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Point, Size};
#[path = "native_fonts.rs"]
mod fonts;

const CAPS: ViewCapabilities = ViewCapabilities {
    accessibility: true,
    ..super::CAPS
};
pub(super) const MODULE: &str = "original-content-native-glyphs";
pub(super) struct NativeText {
    fonts: fonts::Fonts,
    pub(super) atlas: GlyphAtlas,
    pub(super) scale: f32,
    pub(super) context: ViewContext,
    animation_requested: bool,
}
impl NativeText {
    pub(super) fn new() -> Self {
        Self {
            fonts: fonts::Fonts::new(),
            atlas: GlyphAtlas::default(),
            scale: 1.,
            context: ViewContext::default(),
            animation_requested: false,
        }
    }
    pub(super) fn apply_context(&mut self, context: &ViewContext) -> Result<()> {
        tekes_ui_module_runtime::view_context_palette(context)?;
        self.fonts.set_type_scale(context.type_scale);
        self.context = context.clone();
        Ok(())
    }
    fn adapter(&mut self) -> fonts::TextRenderer<'_> {
        fonts::TextRenderer {
            fonts: &mut self.fonts,
            atlas: &mut self.atlas,
            scale: self.scale,
        }
    }
}
impl ButtonTextRenderer for NativeText {
    fn request_animation_frame(&mut self) {
        self.animation_requested = true;
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: Duration::from_nanos(self.context.elapsed_nanos),
            reduced_motion: self.context.reduced_motion,
        })
    }

    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        self.adapter().measure(s, role, width)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.adapter().draw(scene, s, role, rect, align, ink);
    }
}
impl TextSurfaceTextRenderer for NativeText {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        p: &tekes_ui_text::Paragraph,
        origin: Point,
        ink: Rgba,
        clip: Rect,
    ) {
        self.adapter().push_visible(scene, p, origin, ink, clip);
    }
}
impl TextSurfaceHost for NativeText {
    fn caret_visible(&self) -> bool {
        self.context.caret_visible
    }

    fn type_scale(&self) -> f32 {
        self.fonts.type_scale
    }
    fn shape_surface(
        &mut self,
        s: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        self.adapter().shape_surface(s, role, width)
    }
}
struct NativeModule {
    original: Module,
    text: NativeText,
}
pub(super) fn create() -> Box<dyn ProcessModule> {
    Box::new(NativeModule {
        original: accessibility_consumer::original_semantic_module(),
        text: NativeText::new(),
    })
}
impl ProcessModule for NativeModule {
    fn set_view_context(&mut self, context: &ViewContext) -> Result<()> {
        self.text.apply_context(context)?;
        self.original.revision += 1;
        Ok(())
    }

    fn content(&mut self) -> Option<(&mut ContentHost, &mut dyn ButtonTextRenderer)> {
        Some((&mut self.original.host, &mut self.text))
    }
    fn view(&mut self) -> Option<&mut dyn PanelModule> {
        Some(self)
    }
    fn view_capabilities(&self) -> ViewCapabilities {
        CAPS
    }
    fn started(&mut self, emitter: &ComponentEmitter) -> Result<()> {
        self.original.started(emitter)
    }
    fn handle(
        &mut self,
        request: RequestBody,
        emitter: &ComponentEmitter,
    ) -> Result<serde_json::Value> {
        if let RequestBody::Message { topic, .. } = &request
            && topic == "view-context-inspect"
        {
            return Ok(serde_json::json!({
                "context":self.text.context,
                "render_revision":self.original.revision,
                "resolved_type_scale":self.text.type_scale(),
                "caret_visible":self.text.caret_visible(),
                "motion_reduced":self.text.motion_context().unwrap().reduced_motion,
                "motion_nanos":self.text.motion_context().unwrap().now.as_nanos(),
                "surface":Some(tekes_ui_module_runtime::view_context_background(&self.text.context)?).map(|c|[c.r,c.g,c.b,c.a]),
                "inherited_surface":self.original.host.inherited_surface(),
                "pid":std::process::id(),
            }));
        }
        self.original.handle(request, emitter)
    }
}
impl PanelModule for NativeModule {
    fn update(
        &mut self,
        input: &Request,
        scene: &mut Scene,
        atlas: &mut GlyphAtlas,
    ) -> Result<ModuleState> {
        self.text.scale = input.scale;
        // The worker owns its original persistent atlas; the adapter only
        // borrows it during this same original Content render.
        std::mem::swap(atlas, &mut self.text.atlas);
        let frame = Rect::new(0., 0., input.width as f32, input.height as f32);
        let palette = tekes_ui_module_runtime::view_context_palette(&self.text.context)?;
        let rendered = self
            .original
            .host
            .render(scene, &mut self.text, frame, frame, palette);
        std::mem::swap(atlas, &mut self.text.atlas);
        rendered?;
        self.original.revision += 1;
        Ok(ModuleState {
            revision: self.original.revision,
            toolbar: vec![],
        })
    }
}
struct Host {
    module: &'static str,
    capabilities: ViewCapabilities,
    tree: Option<ContentHost>,
    renderer: Option<Renderer>,
    text: NativeText,
    ids: Vec<ComponentId>,
    type_scale: f32,
    logged_ready: bool,
    context: ViewContext,
    started: Instant,
    overlay_review: bool,
    review_directory: Option<std::path::PathBuf>,
    review_started: bool,
    manual_review: bool,
    manual_snapshot_due: bool,
    manual_mutations_armed: bool,
    manual_mutations_pending: bool,
    curve_mode: bool,
    curve_outside_inset: f32,
    pending_curve: Vec<(ComponentId, tekes_ui_module_runtime::ProcessContainer)>,
    b2: B2SelfTest,
}
// Clock ticks use the existing child-frame backpressure. The parent clock
// still advances; a child receives the latest time once its prior frame commits.
fn sync_process_contexts(
    host: &mut ContentHost,
    ids: &[ComponentId],
    context: &ViewContext,
    clock_only: bool,
) -> Result<usize> {
    // The children share one parent geometry clock. Advancing it for an early
    // sibling must not retire another sibling's still-publishing frame.
    if clock_only
        && ids.iter().any(|id| {
            !host
                .get(*id)
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .input_ready()
        })
    {
        return Ok(0);
    }
    let mut forwarded = 0;
    for id in ids {
        forwarded += usize::from(
            host.get_mut(*id)
                .unwrap()
                .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .set_view_context(context.clone())?,
        );
    }
    Ok(forwarded)
}

// Both original native and no-window consumers use this one resource pipeline:
// finish all Host updates, freeze current geometry, then progress every child's
// exact acknowledgements/frame/query and commit without changing that geometry.
fn original_layer_scene(
    host: &mut ContentHost,
    renderer: &mut Renderer,
    text: &mut NativeText,
    ids: &[ComponentId],
    frame: Rect,
    context: &ViewContext,
) -> Result<(Scene, bool)> {
    let mut host_update = false;
    for id in ids {
        let child = host
            .get(*id)
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        let events = child.drain_curve_host_events();
        let update = !events.is_empty() || child.curve_requires_host_update();
        for event in events {
            eprintln!("PROCESS_NATIVE_SERVICE component={} {event:?}", id.0);
        }
        if update {
            host.get_mut(*id)
                .unwrap()
                .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .prepare_curve_host_update()?;
            host_update = true;
        }
    }
    let palette = tekes_ui_module_runtime::view_context_palette(context)?;
    if host_update || host.needs_render() {
        let mut layout_scene = Scene::new();
        host.render(&mut layout_scene, text, frame, frame, palette)?;
    }
    let mut prepared = false;
    for id in ids {
        prepared |= host
            .get(*id)
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap()
            .frame_resource_handle()?
            .prepare_current_frame(renderer)?;
    }
    // Always repaint against the current frozen geometry after receipt polling:
    // an exact retired-grant acknowledgement may resume unsent resource intent
    // even when no new image has been prepared in this pass.
    let mut scene = Scene::new();
    text.animation_requested = false;
    host.repaint_current(&mut scene, text, frame, frame, palette)?;
    if prepared {
        eprintln!("PROCESS_NATIVE_B2_LAYER_FRAME_RECEIVED");
    }
    let next_host_update = ids.iter().any(|id| {
        host.get(*id)
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap()
            .curve_requires_host_update()
    });
    Ok((scene, next_host_update))
}

// Optional B2-only diagnostics share the Render trace's process-monotonic
// Instant time points. Existing presentation receipts remain the evidence.
fn log_b2_timing(w: &native::Window, phase: &str, began: Instant) {
    let presented = w.presented_frames();
    eprintln!(
        "PROCESS_NATIVE_B2_TIMING phase={phase} at={:?} elapsed_us={} acquisition={:?} serial={} reentrant_queued={} presented_count={} last_presented={:?}",
        Instant::now(),
        began.elapsed().as_micros(),
        w.drawable_acquisition_counts(),
        w.drawable_serial(),
        w.metal_reentrant_queued(),
        presented.len(),
        presented
            .last()
            .map(|frame| (frame.serial, frame.acquired_at, frame.presented_at)),
    );
}
/// The same original editor setup drives native acceptance and the real IPC
/// regression. Each mutable action runs once, then waits for its new child
/// frame/projection before the next action uses that geometry lease.
#[derive(Default, Debug)]
enum B2EditorSetupStep {
    #[default]
    Focus,
    Capture,
    Reveal,
    Release,
    Complete,
}
#[derive(Default, Debug)]
struct B2EditorSetup {
    step: B2EditorSetupStep,
    previous: Option<tekes_ui_module_runtime::ProcessContentSnapshot>,
    wait_after_revision: Option<u64>,
    capture_point: [f32; 2],
    capture: Option<wire::Capture>,
    reveal_target: Option<wire::Target>,
}
impl B2EditorSetup {
    fn advance(
        &mut self,
        container: &tekes_ui_module_runtime::ProcessContainer,
    ) -> Result<Option<tekes_ui_module_runtime::ProcessContentSnapshot>> {
        if !container.input_ready()
            || self
                .wait_after_revision
                .is_some_and(|revision| container.revision() == Some(revision))
        {
            return Ok(None);
        }
        let revision = container
            .revision()
            .context("B2 current child frame revision missing")?;
        let bridge = container.native_content_bridge(Duration::from_secs(2))?;
        let snapshot = bridge.snapshot()?;
        if let Some(previous) = &self.previous {
            ensure!(
                snapshot.reply().focused == previous.reply().focused
                    && snapshot.reply().focus_epoch == previous.reply().focus_epoch
                    && snapshot.reply().text_binding == previous.reply().text_binding
                    && snapshot.reply().captured == previous.reply().captured
                    && snapshot.reply().text == previous.reply().text,
                "B2 new frame did not preserve the original editor/capture/text continuation: step={:?} prior_focus={:?}/{} current_focus={:?}/{} prior_capture={:?} current_capture={:?} binding_equal={} prior_text={:?} current_text={:?}",
                self.step,
                previous.reply().focused,
                previous.reply().focus_epoch,
                snapshot.reply().focused,
                snapshot.reply().focus_epoch,
                previous.reply().captured,
                snapshot.reply().captured,
                previous.reply().text_binding == snapshot.reply().text_binding,
                previous.reply().text,
                snapshot.reply().text
            );
        }
        let (next, next_step) = match self.step {
            B2EditorSetupStep::Focus => {
                let editor_target = snapshot
                    .reply()
                    .semantics
                    .as_ref()
                    .and_then(|nodes| nodes.iter().find(|node| node.semantics.name == "Message"))
                    .map(|node| node.target)
                    .context("B2 original editor focus target missing")?;
                let focused = bridge.focus(&snapshot, Some(editor_target), true)?;
                ensure!(
                    focused.reply().focused == Some(editor_target)
                        && focused
                            .reply()
                            .text
                            .as_ref()
                            .is_some_and(|text| text.owner == editor_target),
                    "B2 explicit original editor focus did not publish its text owner"
                );
                (focused, B2EditorSetupStep::Capture)
            }
            B2EditorSetupStep::Capture => {
                let original_text = snapshot
                    .reply()
                    .text
                    .as_ref()
                    .context("B2 original text projection missing")?;
                self.capture_point = [original_text.caret[0], original_text.caret[1]];
                let captured = bridge.request(
                    &snapshot,
                    wire::Input::Pointer(wire::PointerEvent {
                        position: self.capture_point,
                        phase: wire::PointerPhase::Down,
                        button: Some(wire::PointerButton::Primary),
                        modifiers: Default::default(),
                        click_count: 1,
                    }),
                )?;
                self.capture = Some(
                    captured
                        .reply()
                        .captured
                        .context("B2 real editor pointer did not acquire capture")?,
                );
                (captured, B2EditorSetupStep::Reveal)
            }
            B2EditorSetupStep::Reveal => {
                let capture = self.capture.context("B2 original capture missing")?;
                let reading = bridge.reading_state(&snapshot, 20)?;
                let reading_state = reading
                    .reply()
                    .reading
                    .as_ref()
                    .context("B2 real child did not return its reading state")?;
                let reveal_target = *reading_state
                    .items
                    .last()
                    .context("B2 reading fixture has no declared reveal item")?;
                let revealed = bridge.reveal_reading_item(
                    &reading,
                    reveal_target,
                    wire::RevealOptions {
                        placement: wire::RevealPlacement::StartVertical,
                        margins: [0.; 4],
                        animated: false,
                    },
                )?;
                ensure!(
                    revealed.reply().captured == Some(capture)
                        && revealed.reply().focused == snapshot.reply().focused
                        && revealed.reply().focus_epoch == snapshot.reply().focus_epoch
                        && revealed.reply().text == snapshot.reply().text
                        && matches!(
                            revealed
                                .reply()
                                .reading
                                .as_ref()
                                .and_then(|state| state.navigation.as_ref())
                                .map(|navigation| &navigation.status),
                            Some(wire::RevealStatus::Completed)
                        ),
                    "B2 reveal did not preserve the captured editor owner and text continuation"
                );
                self.reveal_target = Some(reveal_target);
                (revealed, B2EditorSetupStep::Release)
            }
            B2EditorSetupStep::Release => {
                let released = bridge.request(
                    &snapshot,
                    wire::Input::Pointer(wire::PointerEvent {
                        position: self.capture_point,
                        phase: wire::PointerPhase::Up,
                        button: Some(wire::PointerButton::Primary),
                        modifiers: Default::default(),
                        click_count: 1,
                    }),
                )?;
                ensure!(
                    released.reply().captured.is_none()
                        && released.reply().focused == snapshot.reply().focused
                        && released.reply().text == snapshot.reply().text,
                    "B2 original capture release did not close the same editor gesture"
                );
                (released, B2EditorSetupStep::Complete)
            }
            B2EditorSetupStep::Complete => return Ok(Some(snapshot)),
        };
        self.wait_after_revision = next.reply().changed.then_some(revision);
        self.previous = Some(next);
        self.step = next_step;
        Ok(None)
    }
}
fn b2_auxiliary_state(host: &ContentHost) -> Result<(u64, bool)> {
    let container = host
        .get(ComponentId(200))
        .and_then(|node| node.component::<tekes_ui_module_runtime::ProcessContainer>())
        .context("B2 original child missing")?;
    let state = inspect_container(container)?;
    let result = (
        state["aux_activations"]
            .as_u64()
            .context("original Button count missing")?,
        state["aux_pressed"]
            .as_bool()
            .context("original Button press state missing")?,
    );
    if std::env::var_os("TEKESUI_PROCESS_LAYER_TRACE").is_some() {
        eprintln!(
            "PROCESS_LAYER_TRACE pid={} at={:?} event=business_state session={:?} revision={:?} count={} pressed={}",
            std::process::id(),
            Instant::now(),
            container.session(),
            container.revision(),
            result.0,
            result.1
        );
    }
    Ok(result)
}
#[derive(Default, Debug)]
struct B2SelfTest {
    started: bool,
    editor_setup: B2EditorSetup,
    button_center: Option<(f64, f64)>,
    editor_center: Option<(f64, f64)>,
    button_key: Option<native::AccessibilityKey>,
    rounded_corner: Option<(f64, f64)>,
    old_stamp: Option<tekes_ui_module_runtime::component::curve::QueryStamp>,
    old_input: Option<wire::Request>,
    pointer_activated: bool,
    pointer_released: bool,
    ax_requested: bool,
    auxiliary_start: Option<u64>,
    ax_activated: bool,
    rounded_pointer_rejected: bool,
    text_query_due: bool,
    text_projection_verified: bool,
    pixels_verified: bool,
    resize_seen: bool,
    stale_requests: Option<(u64, u64)>,
    stale_input_rejected: bool,
    stale_query_rejected: bool,
    stale_replies_verified: bool,
    reveal_capture_verified: bool,
}
impl Host {
    fn created(&mut self, w: &native::Window) -> Result<()> {
        if self.curve_mode && std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_TIMING").is_some() {
            w.set_presentation_timing(true);
        }
        ensure!(
            w.set_text_key_events(true),
            "native ordered key markers unavailable"
        );
        self.renderer = Some(unsafe {
            Renderer::new_for_metal_layer(w.metal_layer(), w.drawable_size(), w.scale() as f32)?
        });
        let wake = w.wake_proxy();
        let window = u64::try_from(w.native_window_number())?;
        let mut containers = Vec::new();
        for index in 0..3 {
            let id = ComponentId(200 + index);
            self.ids.push(id);
            let wake = wake.clone();
            let mut container = tekes_ui_module_runtime::ProcessContainer::new_view(
                ProcessComponentOptions::new(self.module, 81).with_view(
                    window,
                    10 + index,
                    self.capabilities,
                ),
                move || {
                    wake.wake();
                },
            )?;
            container.set_scale(w.scale() as f32)?;
            container.set_view_context(self.context.clone())?;
            if self.curve_mode {
                container.begin_curve_preflight()?;
            }
            containers.push((id, container));
        }
        if self.curve_mode {
            self.pending_curve = containers;
            eprintln!("PROCESS_NATIVE_B2_PREFLIGHT_STARTED children=3");
            return self.redraw(w);
        }
        let children = containers
            .into_iter()
            .map(|(id, container)| Content::new(id, container))
            .collect::<Vec<_>>();
        self.tree = Some(ContentHost::new(Content::new(
            ComponentId(100),
            GridContent {
                model: tekes_ui_controls::extended::Grid::new(3),
                children,
            },
        ))?);
        self.tree.as_mut().unwrap().set_focus_wrap(false);
        self.tree.as_mut().unwrap().set_window_active(w.is_active());
        eprintln!(
            "PROCESS_NATIVE_CREATED parent_pid={} window={} screen={:?}",
            std::process::id(),
            window,
            w.placement_report()
        );
        self.redraw(w)
    }
    fn publish(&mut self, w: &native::Window) {
        // Keep the original Host geometry lease with its complete semantic tree.
        // A clipped child may have zero visible bounds; raw node publication
        // deliberately cannot authorize those nodes and rejects the whole batch.
        let publication = self.tree.as_ref().and_then(|tree| {
            use native::content_accessibility::{
                ContentAccessibilityOwner, ContentAccessibilitySnapshot,
                WindowAccessibilityPublication,
            };
            ContentAccessibilitySnapshot::from_host(tree).and_then(|snapshot| {
                WindowAccessibilityPublication::prepare(vec![(
                    ContentAccessibilityOwner(0),
                    snapshot,
                )])
            })
        });
        w.set_content_accessibility_publication(publication);
        let native_text = self
            .tree
            .as_ref()
            .and_then(tekes_ui_module_runtime::content_text_input_state);
        if self.curve_mode && std::env::var_os("TEKESUI_PROCESS_LAYER_TRACE").is_some() {
            eprintln!(
                "PROCESS_NATIVE_TEXT_PUBLICATION focused={:?} state={native_text:?}",
                self.tree.as_ref().and_then(ContentHost::focused)
            );
        }
        w.set_text_input_state(native_text);
    }
    fn redraw(&mut self, w: &native::Window) -> Result<()> {
        if self.curve_mode && self.tree.is_none() {
            let mut ready = true;
            for (_, container) in &mut self.pending_curve {
                ready &= container.curve_preflight_ready()?;
            }
            if !ready {
                return Ok(());
            }
            let children = self
                .pending_curve
                .drain(..)
                .enumerate()
                .map(|(index, (id, container))| {
                    let mut panel = tekes_ui_controls::composition::PanelContent::new(
                        Content::new(id, container),
                    )?;
                    if self.curve_mode {
                        // The B2 witness intentionally puts the real child
                        // image behind the parent's rounded corner so the
                        // final-pixel assertion detects a missing clip.
                        panel.set_padding(Some(tekes_ui_controls::Insets {
                            top: 0.,
                            right: 0.,
                            bottom: 0.,
                            left: 0.,
                        }))?;
                    }
                    Ok(Content::new(ComponentId(300 + index as u64), panel))
                })
                .collect::<Result<Vec<_>>>()?;
            self.tree = Some(ContentHost::new(Content::new(
                ComponentId(100),
                GridContent {
                    model: tekes_ui_controls::extended::Grid::new(3),
                    children,
                },
            ))?);
            self.tree.as_mut().unwrap().set_focus_wrap(false);
            self.tree.as_mut().unwrap().set_action_events(true);
            self.tree.as_mut().unwrap().set_window_active(w.is_active());
            eprintln!("PROCESS_NATIVE_B2_PREFLIGHT_READY children=3");
        }
        let (Some(tree), Some(renderer)) = (&mut self.tree, &mut self.renderer) else {
            return Ok(());
        };
        tree.set_window_active(w.is_active());
        if !self.curve_mode {
            for id in &self.ids {
                for event in tree
                    .get(*id)
                    .unwrap()
                    .component::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    // Leave exact request replies to their SDK owners, notably
                    // FrameResources' pending SetViewContext acknowledgements.
                    .drain_curve_host_events()
                {
                    eprintln!("PROCESS_NATIVE_SERVICE component={} {event:?}", id.0);
                }
                tree.get_mut(*id)
                    .unwrap()
                    .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .prepare(renderer)?;
            }
        }
        self.text.scale = w.scale() as f32;
        let size = w.size();
        // Keep the original Host viewport size while placing its curved
        // outside-corner probe beyond NSWindow's resize chrome. The B2 test
        // window grows by the same application-owned inset on both sides.
        let inset = self.curve_outside_inset;
        let frame = Rect::new(
            inset,
            inset,
            (size.width as f32 - 2. * inset).max(0.),
            (size.height as f32 - 2. * inset).max(0.),
        );
        self.text.animation_requested = false;
        tree.set_inherited_surface(&self.context.surface)
            .expect("validated window Surface");
        let scene = if self.curve_mode {
            let (scene, next_update) = original_layer_scene(
                tree,
                renderer,
                &mut self.text,
                &self.ids,
                frame,
                &self.context,
            )?;
            if next_update {
                w.request_redraw();
            }
            scene
        } else {
            let mut scene = Scene::new();
            let palette = tekes_ui_module_runtime::view_context_palette(&self.context)?;
            tree.render(&mut scene, &mut self.text, frame, frame, palette)?;
            scene
        };
        let clear = tekes_ui_module_runtime::view_context_background(&self.context)?;
        let render_timing = (self.curve_mode
            && std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_TIMING").is_some())
        .then(Instant::now);
        if let Some(began) = render_timing {
            log_b2_timing(w, "render_begin", began);
        }
        let rendered = renderer.render(&scene, &mut self.text.atlas, clear);
        if let Some(began) = render_timing {
            log_b2_timing(
                w,
                if rendered.is_ok() {
                    "render_end"
                } else {
                    "render_error"
                },
                began,
            );
        }
        rendered?;
        w.set_animating(w.is_active() && self.text.animation_requested);
        if !self.logged_ready
            && self.ids.iter().all(|id| {
                tree.get(*id)
                    .unwrap()
                    .component::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .input_ready()
            })
        {
            self.logged_ready = true;
            let children:Vec<_>=self.ids.iter().map(|id| {
                let c=tree.get(*id).unwrap().component::<tekes_ui_module_runtime::ProcessContainer>().unwrap();
                serde_json::json!({"id":id.0,"session":c.session(),"status":format!("{:?}",c.status())})
            }).collect();
            eprintln!("PROCESS_NATIVE_READY {}", serde_json::to_string(&children)?);
        }
        self.publish(w);
        if self.curve_mode {
            // Finish the field borrow above before invoking Host-wide checks,
            // and restore the same GPU owner even when a check returns Err.
            let mut renderer = self.renderer.take().context("B2 renderer unavailable")?;
            let progress = self.progress_b2(w, &mut renderer, &scene, clear);
            self.renderer = Some(renderer);
            progress?;
        }
        if self.manual_review && self.manual_mutations_pending {
            let tree = self.tree.as_ref().unwrap();
            let first = tree
                .get(self.ids[0])
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap();
            if first.input_ready()
                && tree.focused() == Some(self.ids[0])
                && tekes_ui_module_runtime::content_text_input_state(tree)
                    .is_some_and(|state| state.marked.is_some())
            {
                self.native_review_snapshot(w, "manual_before_update")?;
                scroll_consumer::manual_reveal_tail(first)?;
                let update = container_message(first, "reading-append")?;
                eprintln!("PROCESS_MANUAL_UPDATE append={update}");
                self.manual_mutations_pending = false;
                self.manual_snapshot_due = true;
                w.request_redraw();
            }
        }
        if self.manual_review
            && self.manual_snapshot_due
            && self.logged_ready
            && self.ids.iter().all(|id| {
                self.tree
                    .as_ref()
                    .unwrap()
                    .get(*id)
                    .unwrap()
                    .component::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .input_ready()
            })
        {
            self.native_review_snapshot(w, "manual_frame")?;
            self.manual_snapshot_due = false;
        }
        self.start_native_review()?;
        if self.curve_mode && self.logged_ready && !self.b2.started {
            // Drive setup after its once-only input without replaying it while
            // the original child publishes a replacement frame/projection.
            w.request_redraw();
        }
        Ok(())
    }
    fn verify_b2_parent_pixels(
        &mut self,
        w: &native::Window,
        renderer: &mut Renderer,
        scene: &Scene,
        clear: Rgba,
    ) -> Result<()> {
        if self.b2.pixels_verified {
            return Ok(());
        }
        let tree = self.tree.as_ref().context("B2 parent Host missing")?;
        let panel = tree
            .get(ComponentId(300))
            .and_then(|node| node.component::<tekes_ui_controls::composition::PanelContent>())
            .and_then(tekes_ui_controls::composition::PanelContent::geometry)
            .context("B2 parent Host has no arranged PanelGeometry")?;
        let accessibility =
            native::content_accessibility::ContentAccessibilitySnapshot::from_host(tree)
                .context("B2 current parent accessibility snapshot unavailable")?;
        let nodes = accessibility.nodes();
        let button = nodes
            .iter()
            .find(|node| node.label == "Auxiliary")
            .context("B2 original child Button is absent from the parent Host")?;
        let backing = w.scale() as f32;
        let (width, height) = w.drawable_size();
        let rgba = renderer.render_to_rgba8(
            scene,
            &mut self.text.atlas,
            (width, height),
            backing,
            clear,
        )?;
        let at = |x: u32, y: u32| -> [u8; 4] {
            let offset = (y as usize * width as usize + x as usize) * 4;
            rgba[offset..offset + 4].try_into().unwrap()
        };
        let clear8 = [clear.r, clear.g, clear.b, clear.a]
            .map(|value| (value.clamp(0., 1.) * 255.).round() as u8);
        let shape_rect = panel.shape.rect();
        let mut clipped = None;
        for dy in 0..64 {
            for dx in 0..64 {
                let px = shape_rect.origin.x + (dx as f32 + 0.5) / backing;
                let py = shape_rect.origin.y + (dy as f32 + 0.5) / backing;
                if !panel
                    .shape
                    .contains([f64::from(px), f64::from(py)])
                    .map_err(|error| anyhow::anyhow!("B2 parent geometry: {error:?}"))?
                {
                    let x = (px * backing).floor() as u32;
                    let y = (py * backing).floor() as u32;
                    if x < width && y < height {
                        clipped = Some((x, y));
                        break;
                    }
                }
            }
            if clipped.is_some() {
                break;
            }
        }
        let (corner_x, corner_y) =
            clipped.context("no device pixel lies outside the rounded parent")?;
        ensure!(
            at(corner_x, corner_y) == clear8,
            "parent rounded clip leaked the real child image at ({corner_x},{corner_y})"
        );
        let backing64 = f64::from(backing);
        let x0 = (button.frame.origin.x.max(0.) * backing64).floor() as u32;
        let y0 = (button.frame.origin.y.max(0.) * backing64).floor() as u32;
        let x1 = ((button.frame.origin.x + button.frame.size.width) * backing64)
            .ceil()
            .min(width as f64) as u32;
        let y1 = ((button.frame.origin.y + button.frame.size.height) * backing64)
            .ceil()
            .min(height as f64) as u32;
        let reference = at(x1.min(width - 1), ((y0 + y1) / 2).min(height - 1));
        let nonuniform = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|(x, y)| at(*x, *y) != reference)
            .count();
        ensure!(
            nonuniform > 0,
            "parent final image contains no actual Button pixels inside its semantic frame"
        );
        self.b2.pixels_verified = true;
        eprintln!(
            "PROCESS_NATIVE_B2_PIXEL_ASSERT passed clipped_corner=({corner_x},{corner_y}) child_button_pixels={nonuniform} source=parent-Host-offscreen-readback"
        );
        Ok(())
    }
    fn start_b2_selftest(&mut self) -> Result<()> {
        if self.b2.started {
            return Ok(());
        }
        let tree = self.tree.as_ref().context("B2 parent Host missing")?;
        let container = tree
            .get(ComponentId(200))
            .and_then(|node| node.component::<tekes_ui_module_runtime::ProcessContainer>())
            .context("B2 child ProcessContainer missing")?;
        let Some(snapshot) = self.b2.editor_setup.advance(container)? else {
            return Ok(());
        };
        let accessibility =
            native::content_accessibility::ContentAccessibilitySnapshot::from_host(tree)
                .context("B2 current parent accessibility snapshot unavailable")?;
        let nodes = accessibility.nodes();
        let button = nodes
            .iter()
            .find(|node| node.label == "Auxiliary" && node.frame.size.width > 0.)
            .context("B2 real child Button is not published")?;
        let editor = nodes
            .iter()
            .find(|node| node.label == "Message" && node.frame.size.width > 0.)
            .context("B2 real child editor is not published")?;
        let center = |node: &native::accessibility::AccessibilityNode| {
            (
                node.frame.origin.x + node.frame.size.width * 0.5,
                node.frame.origin.y + node.frame.size.height * 0.5,
            )
        };
        let panel = tree
            .get(ComponentId(300))
            .and_then(|node| node.component::<tekes_ui_controls::composition::PanelContent>())
            .and_then(tekes_ui_controls::composition::PanelContent::geometry)
            .context("B2 parent Panel geometry missing")?;
        let mut corner = None;
        let shape_rect = panel.shape.rect();
        let scale = self.text.scale;
        for dy in 0..64 {
            for dx in 0..64 {
                let x = shape_rect.origin.x + (dx as f32 + 0.5) / scale;
                let y = shape_rect.origin.y + (dy as f32 + 0.5) / scale;
                if !panel
                    .shape
                    .contains([f64::from(x), f64::from(y)])
                    .map_err(|error| anyhow::anyhow!("B2 parent geometry: {error:?}"))?
                {
                    corner = Some((f64::from(x), f64::from(y)));
                    break;
                }
            }
            if corner.is_some() {
                break;
            }
        }
        let corner = corner.context("B2 parent rounded corner has no outside pointer witness")?;
        self.b2.reveal_capture_verified = true;
        eprintln!(
            "PROCESS_NATIVE_B2_REVEAL_CAPTURE_ASSERT passed capture={:?} reveal_item={} released=true",
            self.b2.editor_setup.capture,
            self.b2
                .editor_setup
                .reveal_target
                .context("B2 original reveal target missing")?
                .id
        );
        let stamp = snapshot
            .curve_stamp()
            .cloned()
            .context("B2 initial semantic query is not bound to an independent layer frame")?;
        let target = snapshot
            .reply()
            .semantics
            .as_ref()
            .context("B2 original child semantic tree missing")?
            .iter()
            .find(|node| node.semantics.name == "Auxiliary")
            .map(|node| node.target)
            .context("B2 original child action owner missing")?;
        self.b2.button_center = Some(center(button));
        self.b2.button_key = Some(button.key);
        self.b2.rounded_corner = Some(corner);
        self.b2.old_stamp = Some(stamp.clone());
        self.b2.old_input = Some(wire::Request {
            view: stamp.view,
            viewport: stamp.viewport,
            owner: snapshot.reply().focused,
            focus_epoch: snapshot.reply().focus_epoch,
            text_binding: snapshot.reply().text_binding.clone(),
            capture: snapshot.reply().captured,
            input: wire::Input::Accessibility {
                target,
                part: None,
                binding: None,
                action: wire::AccessibilityAction::Press,
            },
        });
        self.b2.auxiliary_start = Some(b2_auxiliary_state(tree)?.0);
        let original_text_len = snapshot
            .reply()
            .text
            .as_ref()
            .context("B2 original text projection missing")?
            .text
            .encode_utf16()
            .count();
        self.b2.started = true;
        self.review_started = true;
        use native::selftest::Action;
        let (button_x, button_y) = center(button);
        let (editor_x, editor_y) = center(editor);
        self.b2.editor_center = Some((editor_x, editor_y));
        eprintln!("PROCESS_NATIVE_B2_ASSERTIONS_STARTED frame_stamp={stamp:?} corner={corner:?}");
        native::selftest::run_script_published_state(
            vec![
                (
                    200,
                    Action::Click {
                        x: button_x,
                        y: button_y,
                    },
                ),
                (700, Action::AXPressKey { key: button.key }),
                (
                    1_100,
                    Action::Click {
                        x: corner.0,
                        y: corner.1,
                    },
                ),
                (
                    1_500,
                    Action::Click {
                        x: editor_x,
                        y: editor_y,
                    },
                ),
                (
                    1_900,
                    Action::TextRange {
                        text: "B2 text".into(),
                        replacement: Some(0..original_text_len),
                    },
                ),
                (
                    2_300,
                    Action::ExpectText {
                        text: "B2 text".into(),
                        marked: false,
                    },
                ),
                (
                    2_800,
                    Action::Resize {
                        w: 1000. + f64::from(2. * self.curve_outside_inset),
                        h: 530. + f64::from(2. * self.curve_outside_inset),
                    },
                ),
                (10_000, Action::Close),
            ],
            Some((1_500, editor.key)),
        );
        Ok(())
    }
    fn progress_b2(
        &mut self,
        w: &native::Window,
        renderer: &mut Renderer,
        scene: &Scene,
        clear: Rgba,
    ) -> Result<()> {
        if !self.logged_ready || !self.b2.started {
            return Ok(());
        }
        self.verify_b2_parent_pixels(w, renderer, scene, clear)?;
        if self.b2.pointer_released && !self.b2.pointer_activated
            || self.b2.ax_requested && !self.b2.ax_activated
        {
            let tree = self.tree.as_ref().context("B2 parent Host missing")?;
            let ready = tree
                .get(ComponentId(200))
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .input_ready();
            if ready {
                let (count, pressed) = b2_auxiliary_state(tree)?;
                let initial = self
                    .b2
                    .auxiliary_start
                    .context("B2 initial original Button count missing")?;
                let expected = initial + if self.b2.ax_requested { 2 } else { 1 };
                ensure!(count <= expected, "original Button action was duplicated");
                if count == expected {
                    ensure!(
                        !pressed,
                        "original Button remains pressed after completed action"
                    );
                    ensure!(
                        self.tree.as_mut().unwrap().drain_actions().is_empty(),
                        "original child action emitted a duplicate parent business action"
                    );
                    if self.b2.ax_requested {
                        self.b2.ax_activated = true;
                        eprintln!("PROCESS_NATIVE_B2_AX_ASSERT passed original_count={count}");
                    } else {
                        self.b2.pointer_activated = true;
                        eprintln!("PROCESS_NATIVE_B2_POINTER_ASSERT passed original_count={count}");
                    }
                }
            }
        }
        if !self.b2.text_projection_verified
            && self.b2.text_query_due
            && let Some(container) = self.tree.as_ref().and_then(|tree| {
                tree.get(ComponentId(200))
                    .and_then(|node| node.component::<tekes_ui_module_runtime::ProcessContainer>())
            })
            && container.input_ready()
        {
            let snapshot = container
                .native_content_bridge(Duration::from_secs(2))?
                .snapshot()?;
            if let Some(text) = snapshot.reply().text.as_ref()
                && text.text == "B2 text"
            {
                let labels = snapshot
                    .reply()
                    .semantics
                    .as_ref()
                    .context("B2 combined frame omitted semantics")?;
                ensure!(
                    labels.iter().any(|node| node.semantics.name == "Message")
                        && labels.iter().any(|node| node.semantics.name == "Auxiliary")
                        && snapshot
                            .text_input_state()
                            .is_some_and(|state| state.text == text.text),
                    "B2 text and accessibility projection are not from one actual child reply"
                );
                self.b2.text_projection_verified = true;
                eprintln!(
                    "PROCESS_NATIVE_B2_PROJECTION_ASSERT passed stamp={:?} text={} semantics={}",
                    snapshot.curve_stamp(),
                    text.text,
                    labels.len()
                );
            }
        }
        if self.b2.resize_seen && self.b2.stale_requests.is_none() {
            let Some(container) = self.tree.as_ref().and_then(|tree| {
                tree.get(ComponentId(200))
                    .and_then(|node| node.component::<tekes_ui_module_runtime::ProcessContainer>())
            }) else {
                return Ok(());
            };
            if !container.input_ready() {
                return Ok(());
            }
            let old = self
                .b2
                .old_stamp
                .clone()
                .context("B2 stale stamp missing")?;
            let current = container
                .native_content_bridge(Duration::from_secs(2))?
                .snapshot()?;
            let now = current
                .curve_stamp()
                .cloned()
                .context("B2 resized projection lost its independent layer frame stamp")?;
            ensure!(
                old != now
                    && old.gp != now.gp
                    && old.viewport != now.viewport
                    && old.published_frame_seq != now.published_frame_seq,
                "B2 real Host resize did not replace the old frame identity"
            );
            let old_input = self
                .b2
                .old_input
                .clone()
                .context("B2 stale input missing")?;
            let input =
                container.request(tekes_ui_module_runtime::component::RequestBody::CurveV1(
                    tekes_ui_module_runtime::component::curve::Envelope {
                        protocol_major: 1,
                        wire_revision: 1,
                        view: old.view,
                        transaction_id: 0,
                        ttl_ms: 0,
                        body:
                            tekes_ui_module_runtime::component::curve::RequestBodyV1::ContentInput {
                                stamp: old.clone(),
                                request: Box::new(old_input),
                            },
                    },
                ))?;
            let query = container.request(
                tekes_ui_module_runtime::component::RequestBody::CurveV1(
                    tekes_ui_module_runtime::component::curve::Envelope {
                        protocol_major: 1,
                        wire_revision: 1,
                        view: old.view,
                        transaction_id: 0,
                        ttl_ms: 0,
                        body: tekes_ui_module_runtime::component::curve::RequestBodyV1::ContentQuery {
                            query: tekes_ui_module_runtime::component::curve::ReadOnlyQueryWire::SemanticTree {
                                stamp: old,
                            },
                        },
                    },
                ),
            )?;
            self.b2.stale_requests = Some((input, query));
            eprintln!(
                "PROCESS_NATIVE_B2_STALE_CHECK_SENT input_request={input} query_request={query} new_stamp={now:?}"
            );
        }
        if !self.b2.stale_replies_verified
            && let Some((input, query)) = self.b2.stale_requests
            && let Some(container) = self.tree.as_ref().and_then(|tree| {
                tree.get(ComponentId(200))
                    .and_then(|node| node.component::<tekes_ui_module_runtime::ProcessContainer>())
            })
        {
            for packet in container.drain_events() {
                if let tekes_ui_module_runtime::component::Output::CurveReplyV1 {
                    request,
                    reply:
                        tekes_ui_module_runtime::component::curve::CurveReply::Rejected {
                            code:
                                tekes_ui_module_runtime::component::curve::CurveErrorCode::NotReady,
                            ..
                        },
                    ..
                } = packet.output
                {
                    self.b2.stale_input_rejected |= request == input;
                    self.b2.stale_query_rejected |= request == query;
                } else if let tekes_ui_module_runtime::component::Output::CurveReplyV1 {
                    request,
                    ..
                } = packet.output
                {
                    ensure!(
                        request != input && request != query,
                        "old-frame input/query produced a non-NotReady reply"
                    );
                }
            }
            if self.b2.stale_input_rejected && self.b2.stale_query_rejected {
                self.b2.stale_replies_verified = true;
                eprintln!(
                    "PROCESS_NATIVE_B2_STALE_ASSERT passed old_frame_input=NotReady old_frame_query=NotReady no_replay=true"
                );
            }
        }
        Ok(())
    }
    fn start_native_review(&mut self) -> Result<()> {
        if self.manual_review || self.review_started || !self.logged_ready {
            return Ok(());
        }
        if self.curve_mode {
            return self.start_b2_selftest();
        }
        if self.review_directory.is_none() {
            return Ok(());
        }
        let nodes = native::content_accessibility::content_nodes(self.tree.as_ref().unwrap());
        let named = |label: &str| {
            let mut found: Vec<_> = nodes
                .iter()
                .filter(|n| n.label == label && n.frame.size.width > 0. && n.frame.size.height > 0.)
                .collect();
            found.sort_by(|a, b| a.frame.origin.x.total_cmp(&b.frame.origin.x));
            found
        };
        let editors = named("Message");
        let others = named("Other");
        let auxiliary = named("Auxiliary");
        let history = named("Original history");
        if [editors.len(), others.len(), auxiliary.len(), history.len()] != [3, 3, 3, 3] {
            return Ok(());
        }
        let center = |n: &native::accessibility::AccessibilityNode| {
            (
                n.frame.origin.x + n.frame.size.width * 0.5,
                n.frame.origin.y + n.frame.size.height * 0.5,
            )
        };
        let editor = center(editors[0]);
        let other = center(others[0]);
        let auxiliary_point = center(auxiliary[0]);
        let reader = center(history[0]);
        let third = center(editors[2]);
        let foreign_auxiliary = auxiliary[1].key;
        use native::selftest::Action;
        if std::env::var_os("TEKESUI_PROCESS_NATIVE_READING_SELFTEST").is_some() {
            self.review_started = true;
            native::selftest::run_script(vec![
                (200, Action::MenuCommand(48_001)),
                (
                    400,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: 4,
                    },
                ),
                (1_000, Action::MenuCommand(48_008)),
                (
                    1_200,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: -4,
                    },
                ),
                (1_800, Action::MenuCommand(48_009)),
                (
                    2_000,
                    Action::ScrollWheel {
                        x: reader.0,
                        y: reader.1,
                        delta_x: 0,
                        delta_y: -4,
                    },
                ),
                (2_600, Action::MenuCommand(48_010)),
                (3_000, Action::Close),
            ]);
            return Ok(());
        }
        let marked = || Action::ExpectTextState {
            text: "Keep A😀B拼音".into(),
            selection: 11..11,
            marked: Some(9..11),
            query: 9..11,
            actual: 9..11,
        };
        let third_marked = || Action::ExpectTextState {
            text: "draft拼音".into(),
            selection: 7..7,
            marked: Some(5..7),
            query: 5..7,
            actual: 5..7,
        };
        self.review_started = true;
        native::selftest::run_script(vec![
            (200, Action::MenuCommand(48_001)),
            (
                400,
                Action::Click {
                    x: editor.0,
                    y: editor.1,
                },
            ),
            (
                800,
                Action::TextRange {
                    text: "Keep A😀B".into(),
                    replacement: Some(0..5),
                },
            ),
            (
                1_200,
                Action::PreeditRange {
                    text: "拼音".into(),
                    selection: 2..2,
                    replacement: None,
                },
            ),
            (1_700, marked()),
            (
                1_900,
                Action::Click {
                    x: auxiliary_point.0,
                    y: auxiliary_point.1,
                },
            ),
            (2_300, marked()),
            (2_500, Action::MenuCommand(48_002)),
            (3_000, marked()),
            (
                3_200,
                Action::ScrollWheel {
                    x: reader.0,
                    y: reader.1,
                    delta_x: 0,
                    delta_y: 4,
                },
            ),
            (3_600, marked()),
            (
                3_800,
                Action::AXPressKey {
                    key: foreign_auxiliary,
                },
            ),
            (4_300, marked()),
            (4_500, Action::MenuCommand(48_003)),
            (4_700, Action::Unmark),
            (
                5_100,
                Action::ExpectText {
                    text: "Keep A😀B拼音".into(),
                    marked: false,
                },
            ),
            (
                5_300,
                Action::Click {
                    x: other.0,
                    y: other.1,
                },
            ),
            (
                5_650,
                Action::KeyCharacters {
                    key_code: 48,
                    chars: "\t",
                },
            ),
            (
                5_950,
                Action::KeyCharacters {
                    key_code: 48,
                    chars: "\t",
                },
            ),
            (6_400, Action::MenuCommand(48_004)),
            (
                6_600,
                Action::Click {
                    x: third.0,
                    y: third.1,
                },
            ),
            (
                7_000,
                Action::PreeditRange {
                    text: "拼音".into(),
                    selection: 2..2,
                    replacement: None,
                },
            ),
            (7_500, third_marked()),
            (7_700, Action::Resize { w: 1000., h: 360. }),
            (8_400, third_marked()),
            (8_600, Action::MenuCommand(48_005)),
            (8_800, Action::Miniaturize),
            (9_600, Action::Deminiaturize),
            (10_400, Action::MenuCommand(48_006)),
            // Window deactivation deliberately cancels focus. Re-enter with
            // actual user input; do not invent automatic focus restoration.
            (10_600, Action::Resize { w: 1000., h: 500. }),
            (
                11_000,
                Action::Click {
                    x: third.0,
                    y: third.1,
                },
            ),
            (11_400, Action::Unmark),
            (
                11_800,
                Action::ExpectText {
                    text: "draft拼音".into(),
                    marked: false,
                },
            ),
            (12_100, Action::MenuCommand(48_007)),
            (12_400, Action::Close),
        ]);
        Ok(())
    }
    fn native_review_snapshot(&self, w: &native::Window, phase: &str) -> Result<()> {
        use std::io::Write;
        let directory = self
            .review_directory
            .as_ref()
            .context("native review directory")?;
        let tree = self.tree.as_ref().context("native mixed tree")?;
        let mut children = Vec::new();
        for id in &self.ids {
            let c = tree
                .get(*id)
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap();
            children.push(serde_json::json!({
                "id": id.0, "session": c.session(), "status": format!("{:?}",c.status()),
                "input_ready": c.input_ready(), "error":c.error(),
                "mount": tree.semantic_mount(*id),
                "reading": container_message(c, "reading-inspect")?,
                "actions": container_message(c, "inspect")?
            }));
        }
        let text = tekes_ui_module_runtime::content_text_input_state(tree).map(|s| {
            serde_json::json!({
                "text": s.text, "selection":[s.selection.start,s.selection.end],
                "marked":s.marked.map(|r|[r.start,r.end]),
                "caret_origin":[s.caret_origin.x,s.caret_origin.y],
                "caret_size":[s.caret_size.width,s.caret_size.height],"read_only":s.read_only
            })
        });
        let nodes = native::content_accessibility::content_nodes(tree);
        let snapshot = serde_json::json!({"phase":phase,"parent_pid":std::process::id(),
            "focused_container":tree.focused().map(|id|id.0),"window_active":w.is_active(),
            "window_size":[w.size().width,w.size().height],"placement":format!("{:?}",w.placement_report()),
            "physical_ime":"operator_evidence_required","manual_mutations_pending":self.manual_mutations_pending,
            "native_text":text,"children":children,
            "nodes":nodes.iter().map(|n|serde_json::json!({"component":n.key.component,"part":n.key.part,
                "role":format!("{:?}",n.role),"label":n.label,"focused":n.focused,"value":n.value,
                "frame":[n.frame.origin.x,n.frame.origin.y,n.frame.size.width,n.frame.size.height]})).collect::<Vec<_>>()});
        std::fs::create_dir_all(directory)?;
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("native-mixed-history.jsonl"))?,
            "{snapshot}"
        )?;
        if phase == "after_tab" {
            ensure!(
                tree.focused() == Some(self.ids[1])
                    && children[1]["actions"]["focused"] == EDIT.0
                    && text.as_ref().is_some_and(|s| s["text"] == "draft"),
                "native Tab did not enter the next Reader and advance from history to its editor"
            );
            ensure!(
                children[0]["actions"]["edit_text"] == "Keep A😀B拼音"
                    && children[0]["actions"]["submits"] == 0,
                "native Tab changed or submitted the old editor"
            );
        }
        if phase == "after_reactivate" {
            ensure!(
                tree.focused().is_none() && text.is_none(),
                "inactive focus was not revoked"
            );
            ensure!(
                children[2]["actions"]["edit_text"] == "draft拼音"
                    && children[2]["actions"]["submits"] == 0,
                "window lifecycle lost or submitted the original text"
            );
        }
        eprintln!("PROCESS_NATIVE_REVIEW phase={phase}");
        Ok(())
    }
    fn zoom(&mut self, w: &native::Window, delta: f32) -> Result<()> {
        self.type_scale = (self.type_scale + delta).clamp(0.5, 4.);
        self.text.fonts.set_type_scale(self.type_scale);
        self.context.type_scale = self.type_scale;
        self.sync_context(false)?;
        eprintln!("PROCESS_NATIVE_FONT {}", self.type_scale);
        w.request_redraw();
        Ok(())
    }
    fn sync_context(&mut self, clock_only: bool) -> Result<()> {
        self.context.elapsed_nanos = u64::try_from(self.started.elapsed().as_nanos())?;
        self.text.context = self.context.clone();
        if let Some(tree) = &mut self.tree {
            sync_process_contexts(
                tree,
                &self.ids,
                &self.context,
                clock_only && self.curve_mode,
            )?;
        }
        eprintln!("PROCESS_NATIVE_CONTEXT {:?}", self.context);
        Ok(())
    }
    fn handle_event(&mut self, event: native::Event, w: &mut native::Window) -> Result<()> {
        let manual_preedit =
            self.manual_review && matches!(&event, native::Event::TextPreedit { .. });
        if self.manual_review
            && matches!(
                &event,
                native::Event::TextPreedit { .. }
                    | native::Event::TextCommit { .. }
                    | native::Event::TextUnmark
                    | native::Event::Resized { .. }
                    | native::Event::Focus(_)
                    | native::Event::Mouse(_)
                    | native::Event::Scroll(_)
                    | native::Event::Key(_)
                    | native::Event::TextKey(_)
                    | native::Event::Accessibility { .. }
            )
        {
            self.manual_snapshot_due = true;
        }
        if self.curve_mode
            && matches!(
                &event,
                native::Event::TextCommit { .. }
                    | native::Event::TextPreedit { .. }
                    | native::Event::TextUnmark
            )
        {
            self.b2.text_query_due = true;
        }
        // Sample the original clock at real input, not at arbitrary IPC wakes.
        // Otherwise a later hover/open starts from an old caret-blink instant.
        if !self.curve_mode
            && w.is_active()
            && matches!(
                &event,
                native::Event::Mouse(_)
                    | native::Event::Scroll(_)
                    | native::Event::Key(_)
                    | native::Event::TextKey(_)
                    | native::Event::Accessibility { .. }
            )
        {
            self.sync_context(false)?;
        }
        match event {
            native::Event::Created => self.created(w)?,
            native::Event::AnimationFrame(_) => {
                if w.is_active() {
                    self.sync_context(true)?;
                    self.redraw(w)?;
                } else {
                    w.set_animating(false);
                }
            }
            native::Event::CaretBlink => {
                self.context.caret_visible = !self.context.caret_visible;
                self.sync_context(false)?;
                w.request_redraw();
            }
            native::Event::SystemAppearanceChanged {
                dark,
                increase_contrast,
            } => {
                self.context.dark = dark;
                self.context.increased_contrast = increase_contrast;
                self.sync_context(false)?;
                w.request_redraw();
            }
            native::Event::SystemMotionChanged(reduced) => {
                self.context.reduced_motion = reduced;
                self.sync_context(false)?;
                w.request_redraw();
            }
            native::Event::Redraw | native::Event::Wake => self.redraw(w)?,
            native::Event::Resized {
                drawable_size,
                scale,
                ..
            } => {
                if self.curve_mode && self.b2.started {
                    self.b2.resize_seen = true;
                }
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(drawable_size, scale as f32);
                }
                if let Some(tree) = &mut self.tree {
                    for id in &self.ids {
                        tree.get_mut(*id)
                            .unwrap()
                            .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                            .unwrap()
                            .set_scale(scale as f32)?;
                    }
                }
                self.redraw(w)?;
            }
            native::Event::Focus(active) => {
                if let Some(tree) = &mut self.tree {
                    tree.set_window_active(active && w.is_active());
                }
                eprintln!(
                    "PROCESS_NATIVE_FOCUS active={active} actual={}",
                    w.is_active()
                );
                self.publish(w);
                w.request_redraw();
            }
            native::Event::Command(native::CommandId(command))
                if self.review_directory.is_some()
                    && ((48_001..=48_010).contains(&command)
                        || self.manual_review && (48_011..=48_013).contains(&command)) =>
            {
                if self.manual_review && command == 48_013 {
                    self.manual_mutations_armed = true;
                    eprintln!(
                        "PROCESS_MANUAL_ARMED reveal+append on next real Preedit; no injected text"
                    );
                } else if self.manual_review && command == 48_011 {
                    self.native_review_snapshot(w, "manual_checkpoint")?;
                } else if self.manual_review && command == 48_012 {
                    let c = self
                        .tree
                        .as_ref()
                        .unwrap()
                        .get(self.ids[0])
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap();
                    scroll_consumer::manual_reveal_tail(c)?;
                    self.manual_snapshot_due = true;
                    w.request_redraw();
                } else if command == 48_002 {
                    let tree = self.tree.as_ref().context("native mixed tree")?;
                    let c = tree
                        .get(self.ids[0])
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap();
                    let _ = container_message(c, "reading-append")?;
                    self.manual_snapshot_due |= self.manual_review;
                    w.request_redraw();
                } else {
                    let phase = match command {
                        48_001 => "ready",
                        48_003 => "after_aux_append_scroll",
                        48_004 => "after_tab",
                        48_005 => "after_resize",
                        48_006 => "after_reactivate",
                        48_007 => "final",
                        48_008 => "reading_inner",
                        48_009 => "reading_restored",
                        48_010 => "reading_boundary",
                        _ => unreachable!(),
                    };
                    self.native_review_snapshot(w, phase)?;
                }
            }
            native::Event::CloseRequested => {
                if self.manual_review && self.manual_mutations_pending {
                    eprintln!(
                        "PROCESS_MANUAL_INCOMPLETE real marked input was not admitted; no mutation replay"
                    );
                }
                if self.curve_mode {
                    ensure!(
                        self.b2.pixels_verified
                            && self.b2.pointer_activated
                            && self.b2.ax_activated
                            && self.b2.rounded_pointer_rejected
                            && self.b2.text_projection_verified
                            && self.b2.resize_seen
                            && self.b2.stale_replies_verified
                            && self.b2.reveal_capture_verified,
                        "B2 selftest closed before every real frame/input/AX/geometry assertion passed: {:#?}",
                        self.b2
                    );
                    eprintln!("PROCESS_NATIVE_B2_SELFTEST_PASSED");
                }
                if (self.review_started || self.manual_review) && self.review_directory.is_some() {
                    self.native_review_snapshot(w, "before_close")?;
                }
                w.set_animating(false);
                self.tree.take();
                w.set_text_input_state(None);
                w.set_content_accessibility_publication(None);
                w.close();
            }
            native::Event::Key(ref key)
                if self.overlay_review
                    && key.kind == native::KeyEventKind::Down
                    && key.modifiers.command
                    && key.characters_ignoring_modifiers == "o" =>
            {
                if w.is_active()
                    && let Some(tree) = &self.tree
                {
                    tree.get(self.ids[0])
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap()
                        .request(RequestBody::Message {
                            topic: "open_pointer_scope".into(),
                            payload: serde_json::Value::Null,
                        })?;
                    eprintln!("PROCESS_NATIVE_OPEN_REMOTE_POPOVER");
                }
                w.request_redraw();
            }
            native::Event::Key(ref key)
                if key.kind == native::KeyEventKind::Down
                    && key.modifiers.command
                    && matches!(key.characters_ignoring_modifiers.as_str(), "+" | "=" | "-") =>
            {
                if w.is_active() {
                    self.zoom(
                        w,
                        if key.characters_ignoring_modifiers == "-" {
                            -0.1
                        } else {
                            0.1
                        },
                    )?;
                }
            }
            other => {
                if self.curve_mode
                    && let native::Event::Accessibility { target, .. } = &other
                    && self.b2.button_key == Some(*target)
                {
                    ensure!(
                        self.b2.pointer_activated,
                        "B2 AX started before the original pointer action completed"
                    );
                }
                let response = self.tree.as_mut().and_then(|tree| {
                    tekes_ui_module_runtime::dispatch_content_window_event(tree, &other, w)
                });
                if self.curve_mode {
                    let near = |point: (f64, f64), x: f64, y: f64| {
                        (point.0 - x).abs() < 1. && (point.1 - y).abs() < 1.
                    };
                    if let native::Event::Mouse(native::MouseEvent {
                        kind:
                            native::MouseEventKind::Down {
                                button: native::MouseButton::Left,
                                ..
                            },
                        position,
                        ..
                    }) = &other
                        && self
                            .b2
                            .editor_center
                            .is_some_and(|p| near(p, position.x, position.y))
                    {
                        ensure!(
                            response
                                .as_ref()
                                .is_some_and(|r| r.consumed && r.capture_pointer)
                                && self.tree.as_ref().unwrap().focused() == Some(ComponentId(200)),
                            "original B2 editor Down did not acquire its actual parent owner"
                        );
                        eprintln!("PROCESS_NATIVE_B2_EDITOR_DOWN_ASSERT passed");
                    }
                    if matches!(&other, native::Event::TextCommit { .. }) {
                        ensure!(
                            response.as_ref().is_some_and(|r| r.consumed)
                                && self.tree.as_ref().unwrap().focused() == Some(ComponentId(200)),
                            "original B2 TextCommit was not admitted to its editor owner"
                        );
                        eprintln!("PROCESS_NATIVE_B2_TEXT_COMMIT_ASSERT passed");
                    }
                    if let native::Event::Mouse(native::MouseEvent {
                        kind:
                            native::MouseEventKind::Down {
                                button: native::MouseButton::Left,
                                ..
                            },
                        position,
                        ..
                    }) = &other
                        && self
                            .b2
                            .button_center
                            .is_some_and(|p| near(p, position.x, position.y))
                    {
                        ensure!(
                            response
                                .as_ref()
                                .is_some_and(|r| r.consumed && r.capture_pointer && !r.activated),
                            "real original Button Down was not captured without premature activation"
                        );
                    }
                    if let native::Event::Mouse(native::MouseEvent {
                        kind:
                            native::MouseEventKind::Up {
                                button: native::MouseButton::Left,
                                ..
                            },
                        position,
                        ..
                    }) = &other
                    {
                        if self
                            .b2
                            .button_center
                            .is_some_and(|p| near(p, position.x, position.y))
                        {
                            ensure!(
                                response
                                    .as_ref()
                                    .is_some_and(|r| r.consumed && !r.capture_pointer),
                                "real child Button Up was not admitted through parent/child Hosts"
                            );
                            ensure!(
                                !self.b2.pointer_released,
                                "original Button Up was duplicated"
                            );
                            self.b2.pointer_released = true;
                        }
                        if self
                            .b2
                            .rounded_corner
                            .is_some_and(|p| near(p, position.x, position.y))
                        {
                            ensure!(
                                response
                                    .as_ref()
                                    .is_none_or(|r| { !r.activated && !r.capture_pointer }),
                                "rounded parent corner reached an action or captured pointer"
                            );
                            self.b2.rounded_pointer_rejected = true;
                            eprintln!("PROCESS_NATIVE_B2_ROUNDED_POINTER_ASSERT passed");
                        }
                    }
                    if let native::Event::Accessibility { target, .. } = &other
                        && self.b2.button_key == Some(*target)
                    {
                        ensure!(
                            response.as_ref().is_some_and(|r| r.consumed),
                            "real child Button AX action was not admitted through parent/child Hosts"
                        );
                        ensure!(
                            !self.b2.ax_requested,
                            "original Button AX action was duplicated"
                        );
                        self.b2.ax_requested = true;
                    }
                }
                if let Some(response) = response {
                    eprintln!(
                        "PROCESS_NATIVE_INPUT native={other:?} response={response:?} focused={:?}",
                        self.tree.as_ref().and_then(ContentHost::focused)
                    );
                    self.publish(w);
                    if manual_preedit && self.manual_mutations_armed {
                        self.manual_mutations_armed = false;
                        self.manual_mutations_pending = true;
                    }
                    w.request_redraw();
                }
            }
        }
        Ok(())
    }
}
impl native::WindowHandler for Host {
    fn handle(&mut self, event: native::Event, w: &mut native::Window) {
        let event_timing = (self.curve_mode
            && std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_TIMING").is_some())
        .then(|| (Instant::now(), format!("{event:?}")));
        if let Some((began, name)) = &event_timing {
            log_b2_timing(w, &format!("event_begin {name}"), *began);
        }
        let handled = self.handle_event(event, w);
        if let Some((began, name)) = &event_timing {
            log_b2_timing(
                w,
                &format!("event_end {name} ok={}", handled.is_ok()),
                *began,
            );
        }
        if let Err(error) = handled {
            eprintln!("PROCESS_NATIVE_ERROR {error:#}");
            if self.curve_mode
                && let Some(tree) = &self.tree
            {
                for id in &self.ids {
                    let child = tree
                        .get(*id)
                        .unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>()
                        .unwrap();
                    eprintln!(
                        "PROCESS_NATIVE_ERROR_OWNER id={} session={:?} status={:?} revision={:?} input_ready={} error={:?}",
                        id.0,
                        child.session(),
                        child.status(),
                        child.revision(),
                        child.input_ready(),
                        child.error()
                    );
                }
            }
            if self.review_directory.is_some() || self.curve_mode {
                panic!("native mixed acceptance failed: {error:#}");
            }
            w.set_text_input_state(None);
        }
    }
}
pub(super) fn run() -> Result<()> {
    run_mode(false, MODULE, CAPS)
}
pub(super) fn run_overlay() -> Result<()> {
    run_mode(true, MODULE, CAPS)
}
pub(super) fn run_scroll() -> Result<()> {
    run_mode(
        false,
        scroll_consumer::NATIVE_MODULE,
        ViewCapabilities {
            scroll: true,
            ..CAPS
        },
    )
}
fn run_mode(
    overlay_review: bool,
    module: &'static str,
    capabilities: ViewCapabilities,
) -> Result<()> {
    let manual_directory = std::env::var_os("TEKESUI_PROCESS_NATIVE_MANUAL_EVIDENCE");
    let selftest_directory = std::env::var_os("TEKESUI_PROCESS_NATIVE_SELFTEST_EVIDENCE");
    ensure!(
        manual_directory.is_none()
            || (selftest_directory.is_none()
                && std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_SELFTEST").is_none()
                && std::env::var_os("TEKESUI_PROCESS_NATIVE_READING_SELFTEST").is_none()),
        "manual evidence cannot be combined with programmatic selftests"
    );
    let manual_review = manual_directory.is_some();
    let review_directory = manual_directory
        .or(selftest_directory)
        .map(std::path::PathBuf::from);
    let curve_mode = std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_SELFTEST").is_some();
    let curve_outside_inset = if curve_mode {
        proposal::metric("spacing", "space-loose").context("B2 outside inset token missing")?
    } else {
        0.
    };
    let (module, capabilities) = if curve_mode {
        (
            scroll_consumer::NATIVE_MODULE,
            ViewCapabilities {
                scroll: true,
                accessibility: true,
                ..CAPS
            },
        )
    } else {
        (module, capabilities)
    };
    ensure!(
        review_directory.is_none() || module == scroll_consumer::NATIVE_MODULE,
        "native mixed selftest requires the real mixed-reader module"
    );
    ensure!(
        std::env::var_os("TEKESUI_PROCESS_NATIVE_READING_SELFTEST").is_none()
            || review_directory.is_some(),
        "native reading selftest requires its evidence directory"
    );
    let menu = if review_directory.is_some() {
        native::MenuBar {
            menus: vec![native::Menu::new(
                "Acceptance",
                (48_001..=if manual_review { 48_013 } else { 48_010 })
                    .map(|id| {
                        native::MenuItem::Command(native::MenuCommand::new(
                            match id {
                                48_011 => "Manual checkpoint".to_owned(),
                                48_012 => "Manual reveal original tail".to_owned(),
                                48_013 => "Arm reveal + append on next real preedit".to_owned(),
                                _ => format!("Acceptance {id}"),
                            },
                            native::CommandId(id),
                        ))
                    })
                    .collect(),
            )],
        }
    } else {
        native::MenuBar::default()
    };
    native::App::new(native::AppConfig {
        name: Some("TekesUI Process Content Review".into()),
        ..Default::default()
    })
    .run(
        native::WindowConfig {
            title: "TekesUI Process Content Review".into(),
            size: native::Size {
                width: 1000. + f64::from(2. * curve_outside_inset),
                height: 500. + f64::from(2. * curve_outside_inset),
            },
            placement: native::WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            ..Default::default()
        },
        menu,
        Host {
            module,
            capabilities,
            tree: None,
            renderer: None,
            text: NativeText::new(),
            ids: Vec::new(),
            type_scale: 1.,
            logged_ready: false,
            context: ViewContext::default(),
            started: Instant::now(),
            overlay_review,
            review_directory,
            review_started: false,
            manual_review,
            manual_snapshot_due: manual_review,
            manual_mutations_armed: false,
            manual_mutations_pending: false,
            curve_mode,
            curve_outside_inset,
            pending_curve: Vec::new(),
            b2: B2SelfTest::default(),
        },
    );
    Ok(())
}

/// Original three Reader children and real layer IPC/GPU, without opening a window.
pub(super) fn independent_layer_roundtrip() -> Result<serde_json::Value> {
    ensure!(
        std::env::var_os("TEKESUI_PROCESS_NATIVE_B2_SELFTEST").is_some(),
        "layer integration requires original B2 fixture mode"
    );
    let ids = [ComponentId(200), ComponentId(201), ComponentId(202)];
    let scale = 2.;
    let mut context = ViewContext::default();
    let mut children = Vec::new();
    let caps = ViewCapabilities {
        scroll: true,
        accessibility: true,
        ..CAPS
    };
    for (index, id) in ids.into_iter().enumerate() {
        let mut child = tekes_ui_module_runtime::ProcessContainer::new_view(
            ProcessComponentOptions::new(scroll_consumer::NATIVE_MODULE, 81).with_view(
                91,
                10 + index as u64,
                caps,
            ),
            || {},
        )?;
        child.set_scale(scale)?;
        child.set_view_context(context.clone())?;
        child.begin_curve_preflight()?;
        children.push((id, child));
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let mut ready = true;
        for (_, child) in &mut children {
            ready &= child.curve_preflight_ready()?;
        }
        if ready {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "three real layer preflights did not complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let children = children
        .into_iter()
        .enumerate()
        .map(|(index, (id, child))| {
            let mut panel =
                tekes_ui_controls::composition::PanelContent::new(Content::new(id, child))?;
            panel.set_padding(Some(tekes_ui_controls::Insets {
                top: 0.,
                right: 0.,
                bottom: 0.,
                left: 0.,
            }))?;
            Ok(Content::new(ComponentId(300 + index as u64), panel))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut host = ContentHost::new(Content::new(
        ComponentId(100),
        GridContent {
            model: tekes_ui_controls::extended::Grid::new(3),
            children,
        },
    ))?;
    host.set_focus_wrap(false);
    host.set_action_events(true);
    host.set_inherited_surface(&context.surface)
        .map_err(|error| anyhow::anyhow!("original layer host Surface: {error:?}"))?;
    let inset = proposal::metric("spacing", "space-loose").context("B2 inset token missing")?;
    let frame = Rect::new(inset, inset, 1000., 500.);
    let mut renderer = Renderer::new_headless()?;
    let mut text = NativeText::new();
    text.scale = scale;
    fn assert_stale_layer_callbacks(
        child: &tekes_ui_module_runtime::ProcessContainer,
        snapshot: &tekes_ui_module_runtime::ProcessContentSnapshot,
        target_button: wire::Target,
    ) -> Result<()> {
        let old = snapshot
            .curve_stamp()
            .cloned()
            .context("old curve stamp missing")?;
        let old_input = wire::Request {
            view: old.view,
            viewport: old.viewport,
            owner: snapshot.reply().focused,
            focus_epoch: snapshot.reply().focus_epoch,
            text_binding: snapshot.reply().text_binding.clone(),
            capture: snapshot.reply().captured,
            input: wire::Input::Accessibility {
                target: target_button,
                part: None,
                binding: None,
                action: wire::AccessibilityAction::Press,
            },
        };
        let mut pending = Vec::new();
        for body in [
            tekes_ui_module_runtime::component::curve::RequestBodyV1::ContentInput {
                stamp: old.clone(),
                request: Box::new(old_input),
            },
            tekes_ui_module_runtime::component::curve::RequestBodyV1::ContentQuery {
                query: tekes_ui_module_runtime::component::curve::ReadOnlyQueryWire::SemanticTree {
                    stamp: old.clone(),
                },
            },
        ] {
            pending.push(child.request(RequestBody::CurveV1(
                tekes_ui_module_runtime::component::curve::Envelope {
                    protocol_major: 1,
                    wire_revision: 1,
                    view: old.view,
                    transaction_id: 0,
                    ttl_ms: 0,
                    body,
                },
            ))?);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pending.is_empty() {
            for packet in child.drain_events() {
                if let Output::CurveReplyV1 { request, reply, .. } = packet.output
                    && let Some(index) = pending.iter().position(|id| *id == request)
                {
                    ensure!(
                        matches!(
                        reply,
                        tekes_ui_module_runtime::component::curve::CurveReply::Rejected {
                            code:
                                tekes_ui_module_runtime::component::curve::CurveErrorCode::NotReady,
                            ..
                        }
                    ),
                        "old independent-layer input/query was not refused"
                    );
                    pending.remove(index);
                }
            }
            ensure!(
                Instant::now() < deadline,
                "stale independent-layer replies missing"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn settled_scene(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut NativeText,
        ids: &[ComponentId],
        frame: Rect,
        context: &ViewContext,
    ) -> Result<Scene> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let (scene, _) = original_layer_scene(host, renderer, text, ids, frame, context)?;
            let ready = ids.iter().all(|id| {
                host.get(*id)
                    .unwrap()
                    .component::<tekes_ui_module_runtime::ProcessContainer>()
                    .unwrap()
                    .input_ready()
            });
            if ready {
                return Ok(scene);
            }
            if Instant::now() >= deadline {
                let states: Vec<_> = ids.iter().map(|id| {
                    let child = host.get(*id).unwrap()
                        .component::<tekes_ui_module_runtime::ProcessContainer>().unwrap();
                    serde_json::json!({"id":id.0,"session":child.session(),
                        "status":format!("{:?}",child.status()),"revision":child.revision(),
                        "input_ready":child.input_ready(),"host_update":child.curve_requires_host_update(),
                        "delegated":child.delegates_curve_focus(),"error":child.error()})
                }).collect();
                anyhow::bail!("original three-child independent layers did not settle: {states:?}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let scene = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let target = (
        ((1000. + 2. * inset) * scale).round() as u32,
        ((500. + 2. * inset) * scale).round() as u32,
    );
    let clear = tekes_ui_module_runtime::view_context_background(&context)?;
    let rgba = renderer.render_to_rgba8(&scene, &mut text.atlas, target, scale, clear)?;
    let clear8 = [clear.r, clear.g, clear.b, clear.a].map(|v| (v * 255.).round() as u8);
    for index in 0..3 {
        let panel = host
            .get(ComponentId(300 + index))
            .unwrap()
            .component::<tekes_ui_controls::composition::PanelContent>()
            .unwrap()
            .geometry()
            .unwrap();
        let x = (panel.outer.min_x() * scale).ceil() as u32;
        let y = (panel.outer.min_y() * scale).ceil() as u32;
        let offset = ((y * target.0 + x) * 4) as usize;
        ensure!(
            rgba[offset..offset + 4] == clear8,
            "independent child leaked the rounded parent corner"
        );
    }
    let blue = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[..3] == [17, 93, 161])
        .count();
    ensure!(
        blue > 0,
        "original child witness pixels were never imported"
    );
    let child = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap();
    let snapshot = child
        .native_content_bridge(Duration::from_secs(2))?
        .snapshot()?;
    ensure!(
        snapshot.reply().focused.is_none() && snapshot.reply().text.is_none(),
        "fresh Reader unexpectedly acquired an editor from read-only publication"
    );
    let mut setup = B2EditorSetup::default();
    let deadline = Instant::now() + Duration::from_secs(15);
    let snapshot = loop {
        let child = host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        if let Some(snapshot) = setup.advance(child)? {
            break snapshot;
        }
        let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
        ensure!(
            Instant::now() < deadline,
            "original editor focus/capture/reveal publication deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    ensure!(
        snapshot
            .reply()
            .text
            .as_ref()
            .is_some_and(|text| text.text == "draft")
            && snapshot
                .text_input_state()
                .is_some_and(|text| text.text == "draft"),
        "settled original editor text projection differs from its real input state"
    );
    let parent_accessibility =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("settled actual parent Host accessibility publication missing")?;
    let container_mount = host
        .semantic_mount(ids[0])
        .context("original parent container mount missing")?;
    let container_frame = host.get(ids[0]).unwrap().frame();
    for label in ["Message", "Auxiliary"] {
        let projected = parent_accessibility
            .nodes()
            .iter()
            .find(|node| node.key.component == container_mount && node.label == label)
            .with_context(|| format!("settled parent Host omitted original {label}"))?;
        let original = snapshot
            .reply()
            .semantics
            .as_ref()
            .unwrap()
            .iter()
            .find(|node| node.semantics.name == label)
            .with_context(|| format!("original child omitted {label}"))?;
        let visible = original
            .visible
            .context("original target has no visible geometry")?;
        let expected = Rect::new(visible[0], visible[1], visible[2], visible[3])
            .offset(container_frame.min_x(), container_frame.min_y());
        ensure!(
            projected.frame.origin.x == f64::from(expected.min_x())
                && projected.frame.origin.y == f64::from(expected.min_y())
                && projected.frame.size.width == f64::from(expected.size.width)
                && projected.frame.size.height == f64::from(expected.size.height)
                && projected.frame.size.width > 0.
                && projected.frame.size.height > 0.,
            "settled parent Host {label} projection differs from the real original child geometry"
        );
    }
    let editor_node = parent_accessibility
        .nodes()
        .iter()
        .find(|node| node.key.component == container_mount && node.label == "Message")
        .unwrap();
    let editor_point = Point::new(
        (editor_node.frame.origin.x + editor_node.frame.size.width / 2.) as f32,
        (editor_node.frame.origin.y + editor_node.frame.size.height / 2.) as f32,
    );
    let editor_key = editor_node.key;
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "editor click witness did not start the current clock publication"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let unready_down = host.event(&ContentEvent::PointerDown(editor_point));
    let unready_up = host.event(&ContentEvent::PointerUp(editor_point));
    ensure!(
        !unready_down.consumed
            && !unready_down.capture_pointer
            && !unready_up.consumed
            && host.focused().is_none()
            && tekes_ui_module_runtime::content_text_input_state(&host).is_none(),
        "unpublished editor point acquired parent focus or native text authority"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let recovered_editor =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("current editor target publication missing")?;
    ensure!(
        recovered_editor
            .nodes()
            .iter()
            .any(|node| node.key == editor_key && node.label == "Message"),
        "original editor native key did not recover before its click"
    );
    let down = host.event(&ContentEvent::PointerDown(editor_point));
    ensure!(
        down.consumed && down.capture_pointer,
        "actual parent did not admit original editor Down"
    );
    let up = host.event(&ContentEvent::PointerUp(editor_point));
    ensure!(
        up.consumed,
        "actual parent did not retain original editor Up through repaint"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        host.focused() == Some(ids[0]) && host.text_input_snapshot().is_some(),
        "acknowledged child pointer focus did not become original parent keyboard ownership"
    );
    let preedit = host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    ensure!(
        preedit.consumed,
        "actual parent did not deliver original preedit"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        host.is_composing(),
        "original child did not retain marked text through its parent"
    );
    let other_mount = host.semantic_mount(ids[1]).unwrap();
    let current_parent =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("marked parent publication missing")?;
    let other_node = current_parent
        .nodes()
        .iter()
        .find(|node| node.key.component == other_mount && node.label == "Message")
        .context("other original parent editor missing")?;
    let other_editor = other_node.key;
    let other_point = Point::new(
        (other_node.frame.origin.x + other_node.frame.size.width / 2.) as f32,
        (other_node.frame.origin.y + other_node.frame.size.height / 2.) as f32,
    );
    let original_marked = host.text_input_snapshot().unwrap();
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "marked original editor clock witness did not start"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let retained_marked = host
        .text_input_snapshot()
        .context("clock publication removed marked original editor")?;
    let retained_native = tekes_ui_module_runtime::content_text_input_state(&host)
        .context("clock publication removed native marked state")?;
    ensure!(
        host.is_composing()
            && retained_marked.binding == original_marked.binding
            && retained_marked.text == original_marked.text
            && retained_marked.selection == original_marked.selection
            && retained_marked.marked == original_marked.marked
            && retained_native.marked == original_marked.marked
            && retained_native.caret_size.height == 0.
            && retained_native.carets.is_empty(),
        "clock publication replaced IME owner/state or exposed stale native caret"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;

    let other_before = inspect_container(
        host.get(ids[1])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap(),
    )?;
    let rejected = native::content_accessibility::content_action(
        &mut host,
        other_editor,
        native::AccessibilityAction::Focus,
    );
    ensure!(
        !rejected.consumed && host.focused() == Some(ids[0]) && host.is_composing(),
        "cross-owner AX focus bypassed the original IME barrier"
    );
    let rejected_pointer = host.event(&ContentEvent::PointerDown(other_point));
    ensure!(
        !rejected_pointer.consumed
            && !rejected_pointer.capture_pointer
            && host.focused() == Some(ids[0])
            && host.is_composing()
            && host
                .text_input_snapshot()
                .is_some_and(|text| text.text == original_marked.text
                    && text.selection == original_marked.selection
                    && text.marked == original_marked.marked),
        "cross-owner pointer focus bypassed the delegated original IME owner"
    );
    let other_after = inspect_container(
        host.get(ids[1])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap(),
    )?;
    ensure!(
        other_after["focused"] == other_before["focused"]
            && other_after["edit_text"] == other_before["edit_text"],
        "refused cross-owner input mutated the other original child"
    );
    let original_len = host
        .text_input_snapshot()
        .context("marked original parent text missing")?
        .text
        .encode_utf16()
        .count();
    ensure!(
        host.event(&ContentEvent::Commit {
            text: "B2 route".into(),
            replacement: Some(0..original_len)
        })
        .consumed,
        "actual parent did not deliver original commit"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        host.text_input_snapshot()
            .is_some_and(|text| text.text == "B2 route" && text.marked.is_none()),
        "actual parent text callback did not preserve the original editor value/binding"
    );
    // A clock publication withdraws spatial authority, not the original
    // editor/binding or its logical text. Exercise the native TextCommit
    // conversion while that exact same owner must queue its callback.
    let original_binding = host.text_input_snapshot().unwrap().binding;
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "text witness did not start a real original clock regrant"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    ensure!(
        !host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap()
            .input_ready(),
        "text witness unexpectedly retained spatial input admission"
    );
    let retained = tekes_ui_module_runtime::content_text_input_state(&host)
        .context("clock publication removed the original native logical editor")?;
    ensure!(
        retained.text == "B2 route"
            && retained.marked.is_none()
            && retained.caret_size.height == 0.
            && retained.carets.is_empty()
            && host.text_input_snapshot().unwrap().binding == original_binding,
        "clock publication changed native logical text/binding or leaked old geometry"
    );
    let native_commit = native::Event::TextCommit {
        text: "B2 text".into(),
        replacement: Some(0..retained.text.encode_utf16().count()),
    };
    let response = host.event(
        &tekes_ui_module_runtime::content_event(&native_commit)
            .context("native TextCommit conversion missing")?,
    );
    ensure!(
        response.consumed && !response.changed,
        "native TextCommit did not queue behind the current publication"
    );
    // The native-r14 CaretBlink arrived before the commit frame published.
    // Forward that actual context change immediately, including its latest
    // clock, and require the coalesced original spatial publication to recover.
    context.caret_visible = !context.caret_visible;
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, false)? == 3,
        "CaretBlink did not reach the pending original text publication"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let committed = tekes_ui_module_runtime::content_text_input_state(&host)
        .context("acknowledged native text publication missing")?;
    ensure!(
        committed.text == "B2 text"
            && committed.marked.is_none()
            && committed.caret_size.height > 0.
            && committed.caret_origin.x.is_finite()
            && committed.caret_origin.y.is_finite()
            && !committed.carets.is_empty()
            && host.text_input_snapshot().unwrap().binding == original_binding,
        "queued native commit lost original editor text/binding"
    );
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "post-commit native text witness did not advance clocks"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let republishing = tekes_ui_module_runtime::content_text_input_state(&host)
        .context("post-commit clock withdrew original native text")?;
    ensure!(
        republishing.text == "B2 text"
            && republishing.marked.is_none()
            && republishing.caret_size.height == 0.
            && republishing.carets.is_empty()
            && host.text_input_snapshot().unwrap().binding == original_binding,
        "post-commit clock changed original native text/binding or retained stale caret"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let recovered_caret = tekes_ui_module_runtime::content_text_input_state(&host)
        .context("settled native B2 text geometry missing")?;
    ensure!(
        recovered_caret.text == "B2 text"
            && recovered_caret.marked.is_none()
            && recovered_caret.caret_size.height > 0.
            && recovered_caret.caret_origin.x.is_finite()
            && recovered_caret.caret_origin.y.is_finite()
            && !recovered_caret.carets.is_empty()
            && host.text_input_snapshot().unwrap().binding == original_binding,
        "current native B2 caret did not recover after clock publication"
    );
    let current_parent =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("current parent button publication missing")?;
    let button_node = current_parent
        .nodes()
        .iter()
        .find(|node| node.key.component == container_mount && node.label == "Auxiliary")
        .context("actual parent original Button missing")?;
    let button_point = Point::new(
        (button_node.frame.origin.x + button_node.frame.size.width / 2.) as f32,
        (button_node.frame.origin.y + button_node.frame.size.height / 2.) as f32,
    );
    let auxiliary_start = b2_auxiliary_state(&host)?.0;
    let down = host.event(&ContentEvent::PointerDown(button_point));
    ensure!(
        down.consumed && down.capture_pointer && !down.activated,
        "actual parent Button Down was not captured without premature activation"
    );
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start, true),
        "original Button did not enter its actual Down state exactly once"
    );
    let up = host.event(&ContentEvent::PointerUp(button_point));
    ensure!(
        up.consumed && !up.capture_pointer && !up.activated,
        "actual parent fast Button Up was dropped or emitted a second parent action"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 1, false),
        "original Button did not activate exactly once after actual parent fast Up"
    );
    ensure!(
        host.drain_actions().is_empty(),
        "original pointer activation duplicated a parent business action"
    );
    // Start a real same-position grant before Up, but deliberately do not
    // prepare/import its frame. This is an observed parent-publication phase,
    // not a delay chosen to guess when the GPU will finish.
    let before_inflight = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .revision();
    let down = host.event(&ContentEvent::PointerDown(button_point));
    ensure!(
        down.consumed && down.capture_pointer,
        "inflight witness Down was not captured"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    // Coalesce another current parent context while the first Prepare/Layout
    // is still outstanding. Neither render may create an orphan Prepare.
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let inflight = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap();
    ensure!(
        !inflight.input_ready()
            && inflight.revision() == before_inflight
            && inflight.delegates_curve_focus(),
        "same-position grant in flight did not retain only the original gesture continuation"
    );
    let up = host.event(&ContentEvent::PointerUp(button_point));
    ensure!(
        up.consumed && !up.capture_pointer && !up.activated,
        "Up after grant start was lost or converted into a parent action"
    );
    let legacy_clock_probe = std::env::var_os("TEKESUI_PROCESS_LAYER_LEGACY_CLOCK_PROBE").is_some();
    for tick in 1..=65 {
        context.elapsed_nanos = tick * 33_333_333;
        text.context = context.clone();
        let forwarded = sync_process_contexts(&mut host, &ids, &context, !legacy_clock_probe)
            .with_context(|| format!("pending original clock tick {tick}"))?;
        if !legacy_clock_probe {
            ensure!(
                forwarded == 0,
                "clock-only tick bypassed original frame backpressure"
            );
        }
    }
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "latest deferred clock did not reach all original ready owners"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let after_inflight = b2_auxiliary_state(&host)?;
    ensure!(
        after_inflight == (auxiliary_start + 2, false),
        "original Up after grant start did not activate exactly once: actual={after_inflight:?} expected_count={}",
        auxiliary_start + 2
    );
    ensure!(
        host.drain_actions().is_empty(),
        "inflight original action duplicated parent business action"
    );
    // After the capture's own repaint has fully published, change only another
    // child's original View context. A global Host clock refresh must not revoke
    // this still-current spatial gesture merely because its own repaint is done.
    let down = host.event(&ContentEvent::PointerDown(button_point));
    ensure!(
        down.consumed && down.capture_pointer && !down.activated,
        "sibling-interleave original Button Down was not captured"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let capture_before_sibling = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .native_content_bridge(Duration::from_secs(2))?
        .snapshot()?
        .reply()
        .captured
        .context("settled original Button lost capture before sibling update")?;
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 2, true),
        "original Button activated before sibling-interleave Up"
    );
    host.get_mut(ids[1])
        .unwrap()
        .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .set_view_context(ViewContext {
            elapsed_nanos: context.elapsed_nanos + 1,
            ..context.clone()
        })?;
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let own_child = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap();
    ensure!(
        !own_child.input_ready() && own_child.delegates_curve_focus(),
        "sibling clock refresh changed the original typed spatial continuation"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let capture_after_sibling = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .native_content_bridge(Duration::from_secs(2))?
        .snapshot()?
        .reply()
        .captured;
    ensure!(
        capture_after_sibling == Some(capture_before_sibling)
            && b2_auxiliary_state(&host)? == (auxiliary_start + 2, true),
        "sibling clock refresh revoked or retargeted the original captured Button"
    );
    let up = host.event(&ContentEvent::PointerUp(button_point));
    ensure!(
        up.consumed && !up.capture_pointer && !up.activated,
        "sibling-interleave Up was lost or duplicated into parent action"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 3, false)
            && host.drain_actions().is_empty(),
        "sibling-interleave original Button did not activate exactly once"
    );
    let current_parent =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("post-click parent publication missing")?;
    let button_key = current_parent
        .nodes()
        .iter()
        .find(|node| node.key.component == container_mount && node.label == "Auxiliary")
        .unwrap()
        .key;
    // The original native AX key is absent during a real clock regrant. It
    // must recover with exactly the same identity before the original action.
    context.elapsed_nanos += 33_333_333;
    text.context = context.clone();
    ensure!(
        sync_process_contexts(&mut host, &ids, &context, true)? == 3,
        "AX transition witness did not advance original ready clocks"
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let transitional =
        native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
            .context("AX transition parent groups missing")?;
    ensure!(
        !transitional
            .nodes()
            .iter()
            .any(|node| node.key == button_key),
        "unpublished clock frame retained mutating child AX authority"
    );
    let refused = native::content_accessibility::content_action(
        &mut host,
        button_key,
        native::AccessibilityAction::Press,
    );
    ensure!(
        !refused.consumed
            && !refused.activated
            && b2_auxiliary_state(&host)? == (auxiliary_start + 3, false),
        "unpublished original AX key changed business state"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    let recovered = native::content_accessibility::ContentAccessibilitySnapshot::from_host(&host)
        .context("AX ready parent snapshot missing")?;
    ensure!(
        recovered
            .nodes()
            .iter()
            .any(|node| node.key == button_key && node.label == "Auxiliary"),
        "original AX key did not recover after actual clock publication"
    );
    let ax = native::content_accessibility::content_action(
        &mut host,
        button_key,
        native::AccessibilityAction::Press,
    );
    ensure!(
        ax.consumed && !ax.activated,
        "actual parent AX did not admit only the original child action"
    );
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 4, false),
        "original Button AX action did not activate exactly once"
    );
    ensure!(
        host.drain_actions().is_empty(),
        "original AX activation duplicated a parent business action"
    );
    let corner = Point::new(inset + 0.25, inset + 0.25);
    let corner_down = host.event(&ContentEvent::PointerDown(corner));
    let corner_up = host.event(&ContentEvent::PointerUp(corner));
    ensure!(
        !corner_down.capture_pointer
            && !corner_up.capture_pointer
            && !corner_down.activated
            && !corner_up.activated
            && b2_auxiliary_state(&host)? == (auxiliary_start + 4, false),
        "rounded parent corner reached an original child action"
    );
    // Match native corner Down/Up -> Redraw with no intervening clock or
    // mutable child access. Refused input still retires the Host input lease.
    ensure!(
        host.needs_render(),
        "refused corner input did not request current Host geometry"
    );
    let corner_scene =
        original_layer_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?.0;
    let _ = renderer.render_to_rgba8(&corner_scene, &mut text.atlas, target, scale, clear)?;
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, frame, &context)?;
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 4, false)
            && host.pointer_capture().is_none()
            && host.drain_actions().is_empty(),
        "corner redraw changed original action/capture state"
    );
    let snapshot = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .native_content_bridge(Duration::from_secs(2))?
        .snapshot()?;
    let old = snapshot
        .curve_stamp()
        .cloned()
        .context("independent layer has no geometry stamp")?;
    ensure!(
        snapshot
            .reply()
            .semantics
            .as_ref()
            .is_some_and(|nodes| nodes.iter().any(|n| n.semantics.name == "Message")
                && nodes.iter().any(|n| n.semantics.name == "Auxiliary")),
        "original Reader semantics missing"
    );
    let target_button = snapshot
        .reply()
        .semantics
        .as_ref()
        .unwrap()
        .iter()
        .find(|n| n.semantics.name == "Auxiliary")
        .unwrap()
        .target;
    // Revoke a real queued gesture before its replacement frame. It must not
    // acquire permission from the new allocation or activate a later owner.
    let down = host.event(&ContentEvent::PointerDown(button_point));
    ensure!(
        down.consumed && down.capture_pointer,
        "resize witness Down was not captured"
    );
    let before_revoked = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap()
        .revision();
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let inflight = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap();
    ensure!(
        !inflight.input_ready()
            && inflight.revision() == before_revoked
            && inflight.delegates_curve_focus(),
        "resize negative did not begin in the real same-position grant phase"
    );
    let up = host.event(&ContentEvent::PointerUp(button_point));
    ensure!(
        up.consumed && !up.capture_pointer,
        "resize witness Up was not retained"
    );
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 4, true),
        "queued original Up ran before its required publication"
    );
    let resized = Rect::new(inset, inset, 1000., 530.);
    host.render(
        &mut Scene::new(),
        &mut text,
        resized,
        resized,
        tekes_ui_module_runtime::view_context_palette(&context)?,
    )?;
    let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, resized, &context)?;
    let child = host
        .get(ids[0])
        .unwrap()
        .component::<tekes_ui_module_runtime::ProcessContainer>()
        .unwrap();
    let current = child
        .native_content_bridge(Duration::from_secs(2))?
        .snapshot()?;
    let new = current
        .curve_stamp()
        .context("resized independent layer has no geometry stamp")?;
    ensure!(
        current.reply().captured.is_none(),
        "real resize retained an old-geometry capture"
    );
    ensure!(
        b2_auxiliary_state(&host)? == (auxiliary_start + 4, false),
        "revoked queued Up activated after real resize or left original Button pressed"
    );
    ensure!(
        child
            .error()
            .is_some_and(|error| error.contains("stale deferred Content callback")),
        "revoked queued gesture did not report its original identity rejection"
    );
    ensure!(
        old != *new
            && old.gp != new.gp
            && old.viewport != new.viewport
            && old.published_frame_seq != new.published_frame_seq,
        "resize did not replace independent layer geometry identity"
    );
    assert_stale_layer_callbacks(child, &snapshot, target_button)?;
    ensure!(
        host.drain_actions().is_empty(),
        "revoked gesture emitted a parent business action"
    );
    // Lifecycle candidates remain preserved and explicitly selected; the
    // scoped main-chain runner selects main; the original default keeps both.
    let lifecycle_case =
        std::env::var("TEKESUI_PROCESS_LAYER_LIFECYCLE_CASE").unwrap_or_else(|_| "both".into());
    ensure!(
        matches!(
            lifecycle_case.as_str(),
            "main" | "none" | "remount" | "both"
        ),
        "unknown independent layer lifecycle case"
    );
    let mut lifecycle_checks = Vec::new();
    if matches!(lifecycle_case.as_str(), "none" | "both") {
        let child = host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        let prior_session = child
            .session()
            .cloned()
            .context("lifecycle original session missing")?;
        let prior = child
            .native_content_bridge(Duration::from_secs(2))?
            .snapshot()?;
        let prior_target = prior
            .reply()
            .semantics
            .as_ref()
            .unwrap()
            .iter()
            .find(|node| node.semantics.name == "Auxiliary")
            .unwrap()
            .target;
        let prior_count = b2_auxiliary_state(&host)?.0;
        host.render(
            &mut Scene::new(),
            &mut text,
            resized,
            resized,
            tekes_ui_module_runtime::view_context_palette(&context)?,
        )?;
        ensure!(
            !host
                .get(ids[0])
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .input_ready(),
            "context-removal witness did not start before grant acknowledgement"
        );
        eprintln!("LAYER_LIFECYCLE pending_context_none begin");
        // This public Host boundary really calls root.clear_curve_geometry_context.
        let external =
            tekes_ui_scene::ClipShape::normalize(resized, Default::default(), Point::new(0., 0.))
                .map_err(|error| anyhow::anyhow!("lifecycle external shape: {error:?}"))?;
        host.set_external_curve_chain(Some(81_001), std::sync::Arc::from([external]))?;
        host.set_external_curve_chain(None, std::sync::Arc::from([]))?;
        ensure!(
            !host
                .get(ids[0])
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .input_ready(),
            "removed context retained input authority"
        );
        let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, resized, &context)
            .context("pending context removal did not recover original owner")?;
        let child = host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        ensure!(
            child.session() == Some(&prior_session),
            "context removal replaced process owner"
        );
        assert_stale_layer_callbacks(child, &prior, prior_target)?;
        ensure!(
            b2_auxiliary_state(&host)? == (prior_count, false) && host.drain_actions().is_empty(),
            "context removal activated or captured a control"
        );
        lifecycle_checks.push("pending_context_none_recovery_and_stale_refusal");
        eprintln!("LAYER_LIFECYCLE pending_context_none passed");
    }
    if matches!(lifecycle_case.as_str(), "remount" | "both") {
        let child = host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        let prior_session = child
            .session()
            .cloned()
            .context("remount original session missing")?;
        let prior = child
            .native_content_bridge(Duration::from_secs(2))?
            .snapshot()?;
        let prior_target = prior
            .reply()
            .semantics
            .as_ref()
            .unwrap()
            .iter()
            .find(|node| node.semantics.name == "Auxiliary")
            .unwrap()
            .target;
        let stale_handle = child.frame_resource_handle()?;
        host.render(
            &mut Scene::new(),
            &mut text,
            resized,
            resized,
            tekes_ui_module_runtime::view_context_palette(&context)?,
        )?;
        ensure!(
            !host
                .get(ids[0])
                .unwrap()
                .component::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap()
                .input_ready(),
            "remount witness did not start before grant acknowledgement"
        );
        eprintln!("LAYER_LIFECYCLE pending_unmount_remount begin");
        let mut retained = host.remove(ComponentId(300))?;
        {
            let panel = retained
                .component_mut::<tekes_ui_controls::composition::PanelContent>()
                .unwrap();
            let child = tekes_ui_controls::composition::Component::children_mut(panel).unwrap()[0]
                .component_mut::<tekes_ui_module_runtime::ProcessContainer>()
                .unwrap();
            ensure!(
                child.session().is_none() && !child.input_ready(),
                "unmounted original instance retained process/input authority"
            );
            child.begin_curve_preflight()?;
            let deadline = Instant::now() + Duration::from_secs(15);
            while !child.curve_preflight_ready()? {
                ensure!(
                    Instant::now() < deadline,
                    "same-instance remount preflight did not complete"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            ensure!(
                child.session() != Some(&prior_session),
                "remount reused old process session"
            );
        }
        host.insert(ComponentId(100), retained)?;
        let _ = settled_scene(&mut host, &mut renderer, &mut text, &ids, resized, &context)
            .context("same original instance remount did not publish a fresh grant")?;
        let child = host
            .get(ids[0])
            .unwrap()
            .component::<tekes_ui_module_runtime::ProcessContainer>()
            .unwrap();
        ensure!(
            !stale_handle.prepare_current_frame(&mut renderer)?,
            "old frame handle published into remounted owner"
        );
        ensure!(
            child
                .native_content_bridge(Duration::from_secs(2))?
                .request(&prior, wire::Input::Unmark)
                .is_err(),
            "old session snapshot entered remounted owner"
        );
        assert_stale_layer_callbacks(child, &prior, prior_target)?;
        ensure!(
            b2_auxiliary_state(&host)? == (0, false) && host.drain_actions().is_empty(),
            "remounted original Button retained old actions/capture"
        );
        lifecycle_checks.push("same_instance_pending_remount_new_session_and_stale_refusal");
        eprintln!("LAYER_LIFECYCLE pending_unmount_remount passed");
    }
    Ok(
        serde_json::json!({"result":"INDEPENDENT_LAYER_ORIGINAL_READER_OK", "children":3,
        "logical_request":[333,500], "parent_allocation_width":1000_f32/3.,
        "backing_scale":scale,"actual_witness_pixels":blue,"old_stamp":old,"new_stamp":new,
        "parent_corner_refused":true,"resize_stale_input_query_refused":true,
        "initial_read_only_text_absent":true,"explicit_original_focus_text_projection":true,
        "original_capture_reveal_release_after_publication":true,
        "resize_cancels_original_capture":true,
        "settled_parent_host_original_targets_and_coordinates":true,
        "actual_parent_fast_pointer_and_ax_exact_activation":true,"no_duplicate_parent_business_action":true,
        "actual_parent_text_and_cross_owner_ime_barrier":true,"queued_up_revoked_on_real_resize":true,
        "up_after_reraster_grant_before_publication":true,"same_spatial_capture_survives_sibling_context_update":true,"clock_ticks_respect_frame_backpressure_and_catch_up":true,"same_native_ax_key_withdraws_then_recovers_before_press":true,"refused_corner_then_shared_redraw_preserves_freshness":true,"native_logical_text_and_ime_binding_survive_clock_publication":true,"queued_native_text_commit_preserves_original_editor":true,"current_native_caret_recovers_after_commit_and_caret_blink":true,"unpublished_editor_click_refused_then_same_key_focus_admitted":true,"lifecycle_checks":lifecycle_checks,
        "native_window":"not_opened","physical_ime":"not_run","voiceover":"not_run"}),
    )
}

/// Real isolated worker + original Content/backend; no window or physical IME claim.
pub(super) fn context_roundtrip() -> Result<serde_json::Value> {
    use tekes_ui_module_runtime::ProcessContainer;
    let id = ComponentId(410);
    let mut child = ProcessContainer::new_view(
        ProcessComponentOptions::new(MODULE, 91).with_view(9, 81, CAPS),
        || {},
    )?;
    let initial = ViewContext::default();
    ensure!(
        child.set_view_context(initial.clone())?,
        "first context ignored"
    );
    ensure!(
        !child.set_view_context(initial.clone())?,
        "identical context creates work"
    );
    let mut host = ContentHost::new(Content::new(id, child))?;
    let mut renderer = Renderer::new_headless()?;
    let mut text = Text;
    let frame = Rect::new(0., 0., 320., 500.);
    fn ready(
        host: &mut ContentHost,
        renderer: &mut Renderer,
        text: &mut Text,
        frame: Rect,
    ) -> Result<()> {
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            host.get_mut(ComponentId(410))
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare(renderer)?;
            host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())?;
            let c = host
                .get(ComponentId(410))
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(c.error().is_none(), "context worker failed {:?}", c.error());
            if c.input_ready() {
                return Ok(());
            }
            ensure!(
                Instant::now() < end,
                "context layout acknowledgement deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let first = container_message(c, "view-context-inspect")?;
    let pid = first["pid"].as_u64().context("worker PID")?;
    let session = c.session().unwrap().clone();
    let (owner, original_binding) = {
        let bridge = c.native_content_bridge(Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let editor = *snapshot
            .reply()
            .focus_targets
            .iter()
            .find(|t| t.id == EDIT.0)
            .unwrap();
        let focused = bridge.focus(&snapshot, Some(editor), true)?;
        let marked = bridge.request(
            &focused,
            wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3),
            },
        )?;
        ensure!(
            marked.text_input_state().unwrap().marked.is_some(),
            "original preedit absent"
        );
        (editor, marked.reply().text_binding.clone())
    };
    let mut contexts = Vec::new();
    let mut revision = 0;
    for surface in tekes_ui_tokens::surface_palette::surface_names() {
        for type_scale in [1., 1.5, 2.] {
            for dark in [false, true] {
                for increased_contrast in [false, true] {
                    revision += 1;
                    let context = ViewContext {
                        type_scale,
                        dark,
                        increased_contrast,
                        brand: 0,
                        surface: (*surface).into(),
                        elapsed_nanos: revision,
                        caret_visible: revision % 2 == 0,
                        reduced_motion: revision % 2 != 0,
                    };
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<ProcessContainer>()
                        .unwrap()
                        .set_view_context(context.clone())?;
                    ready(&mut host, &mut renderer, &mut text, frame)?;
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    let mut report = container_message(c, "view-context-inspect")?;
                    ensure!(
                        report["inherited_surface"] == *surface,
                        "SDK did not inherit Surface into original ContentHost"
                    );
                    ensure!(
                        report["context"] == serde_json::to_value(&context)?,
                        "environment changed in transit"
                    );
                    ensure!(
                        report["resolved_type_scale"].as_f64() == Some(f64::from(type_scale)),
                        "original font backend did not scale"
                    );
                    ensure!(
                        report["caret_visible"].as_bool() == Some(context.caret_visible)
                            && report["motion_reduced"].as_bool() == Some(context.reduced_motion),
                        "original backend dropped caret/motion"
                    );
                    let color = tekes_ui_module_runtime::view_context_background(&context)?;
                    ensure!(
                        report["surface"]
                            == serde_json::json!([color.r, color.g, color.b, color.a]),
                        "palette bypasses compiled JSON"
                    );
                    let revision = report["render_revision"]
                        .as_u64()
                        .context("context revision")?;
                    let end = Instant::now() + Duration::from_secs(15);
                    loop {
                        host.get_mut(id)
                            .unwrap()
                            .component_mut::<ProcessContainer>()
                            .unwrap()
                            .prepare(&mut renderer)?;
                        let mut scene = Scene::new();
                        host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())?;
                        if host
                            .get(id)
                            .unwrap()
                            .component::<ProcessContainer>()
                            .unwrap()
                            .revision()
                            .is_some_and(|v| v >= revision)
                        {
                            let pixels = renderer.render_to_rgba8(
                                &scene,
                                &mut GlyphAtlas::default(),
                                (320, 500),
                                1.,
                                Rgba::gray(1.),
                            )?;
                            let offset = (480 * 320 + 300) * 4;
                            let expected = [color.r, color.g, color.b, color.a]
                                .map(|v| (v * 255.).round() as u8);
                            ensure!(
                                pixels[offset..offset + 4]
                                    .iter()
                                    .zip(expected)
                                    .all(|(a, b)| a.abs_diff(b) <= 1),
                                "remote surface background bypasses View context: actual {:?}, expected {:?}",
                                &pixels[offset..offset + 4],
                                expected
                            );
                            report["gpu_corner"] = serde_json::json!(&pixels[offset..offset + 4]);
                            report["expected_corner"] = serde_json::json!(expected);
                            break;
                        }
                        ensure!(Instant::now() < end, "context GPU frame deadline");
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    let snapshot = c
                        .native_content_bridge(Duration::from_secs(5))?
                        .snapshot()?;
                    let state = snapshot
                        .text_input_state()
                        .context("original editor after context change")?;
                    ensure!(
                        state.marked.is_some() && state.text.contains("ni"),
                        "context update interrupted original preedit"
                    );
                    ensure!(
                        snapshot.reply().focused == Some(owner)
                            && snapshot.reply().text_binding == original_binding,
                        "environment changed original editor authority"
                    );
                    contexts.push(report);
                }
            }
        }
    }
    let c = host
        .get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap();
    ensure!(
        c.set_view_context(ViewContext {
            type_scale: f32::NAN,
            ..initial.clone()
        })
        .is_err(),
        "NaN scale accepted"
    );
    ensure!(
        c.set_view_context(initial.clone()).is_err(),
        "backward clock accepted"
    );
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    ensure!(
        c.session() == Some(&session),
        "environment replaced original process"
    );
    let bridge = c.native_content_bridge(Duration::from_secs(5))?;
    let snapshot = bridge.snapshot()?;
    // Bypass the convenience setter to exercise the original actor's own
    // validation boundary. A rejected request must not terminate or mutate it.
    let before_invalid = container_message(c, "view-context-inspect")?;
    let bad = c.request(RequestBody::SetViewContext {
        view: 81,
        context: ViewContext {
            surface: "unknown".into(),
            type_scale: 4.,
            elapsed_nanos: revision + 1,
            ..Default::default()
        },
    })?;
    let mut rejected = None;
    wait(|| {
        for packet in c.drain_events() {
            if let Output::Response { request, result } = packet.output
                && request == bad
            {
                rejected = Some(result);
            }
        }
        rejected.is_some()
    })?;
    ensure!(rejected.unwrap().is_err(), "invalid actor Surface accepted");
    let after_invalid = container_message(c, "view-context-inspect")?;
    ensure!(
        before_invalid == after_invalid,
        "invalid actor context changed original backend"
    );
    let preserved = bridge.snapshot()?;
    ensure!(
        preserved.reply().text_binding == snapshot.reply().text_binding
            && preserved.text_input_state() == snapshot.text_input_state(),
        "invalid context changed original IME authority"
    );
    let confirmed = bridge.request(
        &snapshot,
        wire::Input::Commit {
            text: "你".into(),
            replacement: None,
        },
    )?;
    ensure!(
        confirmed.text_input_state().unwrap().marked.is_none()
            && confirmed.text_input_state().unwrap().text.contains('你'),
        "original preedit cannot confirm after environment changes"
    );
    let point = confirmed.reply().text.as_ref().unwrap().caret;
    let pointer = |phase| PointerEvent {
        position: tekes_ui_scene::Point::new(point[0], point[1] + 1.),
        phase,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 1,
    };
    let held = bridge.pointer(&confirmed, &pointer(PointerPhase::Down))?;
    let capture = held.reply().captured;
    ensure!(capture.is_some(), "original text pointer did not capture");
    let visual = ViewContext {
        type_scale: 2.,
        elapsed_nanos: revision + 1,
        caret_visible: false,
        reduced_motion: true,
        ..Default::default()
    };
    host.get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_view_context(visual.clone())?;
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let report = container_message(c, "view-context-inspect")?;
    ensure!(
        report["context"] == serde_json::to_value(&visual)?,
        "visual context not applied"
    );
    let retained = c
        .native_content_bridge(Duration::from_secs(5))?
        .snapshot()?;
    ensure!(
        retained.reply().captured == capture,
        "visual-only environment cancelled held pointer"
    );
    let zoom = ViewContext {
        type_scale: 1.,
        elapsed_nanos: revision + 2,
        ..visual.clone()
    };
    host.get_mut(id)
        .unwrap()
        .component_mut::<ProcessContainer>()
        .unwrap()
        .set_view_context(zoom.clone())?;
    ready(&mut host, &mut renderer, &mut text, frame)?;
    let c = host
        .get(id)
        .unwrap()
        .component::<ProcessContainer>()
        .unwrap();
    let after = c
        .native_content_bridge(Duration::from_secs(5))?
        .snapshot()?;
    ensure!(
        after.reply().captured.is_none(),
        "typography change retained old pointer geometry"
    );
    ensure!(
        after.text_input_state().unwrap().text.contains('你'),
        "geometry cancellation lost original editor value"
    );
    let mut motion_checks = Vec::new();
    for (phase, nanos, reduced, topic, expected) in [
        (
            "enter",
            1_000_000_000,
            false,
            Some("open_pointer_scope"),
            true,
        ),
        ("settled", 3_000_000_000, false, None, false),
        (
            "exit",
            3_000_000_001,
            false,
            Some("close_pointer_scope"),
            true,
        ),
        ("reduced", 3_000_000_002, true, None, false),
    ] {
        let old_revision = host
            .get(id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap()
            .revision();
        let c = host
            .get_mut(id)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap();
        c.set_view_context(ViewContext {
            elapsed_nanos: nanos,
            reduced_motion: reduced,
            ..zoom.clone()
        })?;
        if let Some(topic) = topic {
            container_message(c, topic)?;
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            host.get_mut(id)
                .unwrap()
                .component_mut::<ProcessContainer>()
                .unwrap()
                .prepare(&mut renderer)?;
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )?;
            let c = host
                .get(id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(c.error().is_none(), "motion worker error {:?}", c.error());
            if c.input_ready() && c.revision() > old_revision {
                let observed = c
                    .native_content_bridge(Duration::from_secs(5))?
                    .snapshot()?;
                ensure!(
                    !observed.reply().changed,
                    "motion query created render work"
                );
                if observed.reply().animation_frame_requested == expected
                    && host.animation_frame_requested() == expected
                {
                    motion_checks.push(
                        serde_json::json!({"phase":phase,"requested":expected,"clock":nanos}),
                    );
                    break;
                }
            }
            ensure!(
                Instant::now() < deadline,
                "original remote {phase} frame demand failed"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    drop(host);
    renderer.collect_retired_images();
    Ok(
        serde_json::json!({"result":"ORIGINAL_VIEW_CONTEXT_OK","pid":pid,"session":session,"contexts":contexts,"original_preedit_and_binding_preserved":true,"confirmed_after_updates":true,"visual_context_preserves_capture":true,"typography_cancels_only_capture":true,"invalid_input_rejected":true,"invalid_actor_context_keeps_original_backend_alive":true,"original_frame_demand":motion_checks,"scope":"real IPC + original CoreText/GlyphAtlas backend and editor; no physical IME/native/AX/Gold7 acceptance"}),
    )
}

/// Three original SDK trees: independent process/session context, not three
/// copies of a consumer input/focus implementation. No desktop window is opened.
pub(super) fn three_surface_roundtrip() -> Result<serde_json::Value> {
    use tekes_ui_module_runtime::ProcessContainer;
    let ids = [ComponentId(510), ComponentId(511), ComponentId(512)];
    let names = ["sidebar", "panel", "popover"];
    let mut children = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let mut c = ProcessContainer::new_view(
            ProcessComponentOptions::new(MODULE, 101).with_view(9, 91 + index as u64, CAPS),
            || {},
        )?;
        c.set_view_context(ViewContext {
            surface: names[index].into(),
            ..Default::default()
        })?;
        children.push(Content::new(*id, c));
    }
    let mut host = ContentHost::new(Content::new(
        ComponentId(500),
        GridContent {
            model: tekes_ui_controls::extended::Grid::new(3),
            children,
        },
    ))?;
    let frame = Rect::new(0., 0., 960., 500.);
    let mut renderer = Renderer::new_headless()?;
    let mut text = Text;
    let prepare =
        |host: &mut ContentHost, renderer: &mut Renderer, text: &mut Text| -> Result<Scene> {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                for id in ids {
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<ProcessContainer>()
                        .unwrap()
                        .prepare(renderer)?;
                }
                let mut scene = Scene::new();
                host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())?;
                let mut ready = true;
                for id in ids {
                    let c = host
                        .get(id)
                        .unwrap()
                        .component::<ProcessContainer>()
                        .unwrap();
                    ensure!(
                        c.error().is_none(),
                        "three-context worker failed {:?}",
                        c.error()
                    );
                    ready &= c.input_ready();
                }
                if ready {
                    return Ok(scene);
                }
                ensure!(
                    Instant::now() < deadline,
                    "three-context layout acknowledgement deadline"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        };
    prepare(&mut host, &mut renderer, &mut text)?;
    let mut before = Vec::new();
    let mut bindings = Vec::new();
    let mut sessions = Vec::new();
    for id in ids {
        let c = host
            .get(id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let report = container_message(c, "view-context-inspect")?;
        before.push(report);
        sessions.push(c.session().unwrap().clone());
        let bridge = c.native_content_bridge(Duration::from_secs(5))?;
        let snapshot = bridge.snapshot()?;
        let owner = *snapshot
            .reply()
            .focus_targets
            .iter()
            .find(|target| target.id == EDIT.0)
            .context("original editor")?;
        let focused = bridge.focus(&snapshot, Some(owner), true)?;
        let marked = bridge.request(
            &focused,
            wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3),
            },
        )?;
        ensure!(
            marked.text_input_state().unwrap().marked.is_some(),
            "original preedit missing"
        );
        bindings.push(marked.reply().text_binding.clone());
    }
    let pids: std::collections::BTreeSet<_> =
        before.iter().map(|v| v["pid"].as_u64().unwrap()).collect();
    ensure!(
        pids.len() == 3 && !pids.contains(&u64::from(std::process::id())),
        "process isolation missing"
    );
    let mut reports = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let next = ViewContext {
            surface: names[(index + 1) % names.len()].into(),
            dark: index % 2 == 0,
            elapsed_nanos: 1,
            ..Default::default()
        };
        host.get_mut(*id)
            .unwrap()
            .component_mut::<ProcessContainer>()
            .unwrap()
            .set_view_context(next.clone())?;
        prepare(&mut host, &mut renderer, &mut text)?;
        let c = host
            .get(*id)
            .unwrap()
            .component::<ProcessContainer>()
            .unwrap();
        let report = container_message(c, "view-context-inspect")?;
        ensure!(
            report["context"] == serde_json::to_value(&next)?
                && report["inherited_surface"] == next.surface,
            "original SDK tree lost its own Surface"
        );
        ensure!(
            report["pid"] == before[index]["pid"] && c.session() == Some(&sessions[index]),
            "Surface replaced process"
        );
        let snapshot = c
            .native_content_bridge(Duration::from_secs(5))?
            .snapshot()?;
        ensure!(
            snapshot.reply().text_binding == bindings[index]
                && snapshot.text_input_state().unwrap().marked.is_some(),
            "Surface update interrupted original preedit/binding"
        );
        for (other, other_id) in ids.iter().enumerate().skip(index + 1) {
            let untouched = host
                .get(*other_id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            ensure!(
                container_message(untouched, "view-context-inspect")?["context"]
                    == before[other]["context"],
                "context was misrouted to another process"
            );
        }
        reports.push(report);
    }
    // Each worker's real shared image must use its own generated background.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut pixels_checked;
    loop {
        let scene = prepare(&mut host, &mut renderer, &mut text)?;
        let pixels = renderer.render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (960, 500),
            1.,
            Rgba::gray(1.),
        )?;
        pixels_checked = 0;
        for (index, id) in ids.iter().enumerate() {
            let rect = host.get(*id).unwrap().frame();
            let x = (rect.max_x() - 20.).floor() as usize;
            let y = (rect.max_y() - 20.).floor() as usize;
            let offset = (y * 960 + x) * 4;
            let expected = reports[index]["surface"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v.as_f64().unwrap() * 255.).round() as u8)
                .collect::<Vec<_>>();
            let c = host
                .get(*id)
                .unwrap()
                .component::<ProcessContainer>()
                .unwrap();
            let revision = reports[index]["render_revision"]
                .as_u64()
                .context("original context revision")?;
            if c.revision().is_some_and(|current| current >= revision)
                && pixels[offset..offset + 4]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1)
            {
                pixels_checked += 1;
            }
        }
        if pixels_checked == 3 {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "three original GPU Surface backgrounds did not settle"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(host);
    renderer.collect_retired_images();
    Ok(
        serde_json::json!({"result":"THREE_ORIGINAL_SURFACES_OK","processes":reports,"sessions":sessions,
        "gpu_backgrounds_checked":pixels_checked,"preedit_and_bindings_preserved":true,"context_misrouting":false,
        "scope":"three real child PIDs/sessions + shared GPU images; no native/physical IME/AX/scroll/Gold7 acceptance"}),
    )
}
