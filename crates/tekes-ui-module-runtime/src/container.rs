//! Standard Content adapter for negotiated original Content and legacy Views.
//! GPU attachment uses one prepare call; consumers do not own frame state machines.
use crate::{
    NativeViewOptions, ProcessComponent, ProcessComponentOptions, ProcessStatus, ViewImage,
};
use anyhow::{Result, ensure};
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

#[derive(Debug)]
struct PendingContentInput {
    session: crate::component::Session,
    owner: Option<crate::component::content::Target>,
    focus_epoch: u64,
    text_binding: Option<Vec<u8>>,
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
    process: Option<ProcessComponent>,
    registration: Option<GpuImageRegistration>,
    image: Option<ViewImage>,
    frame: Rect,
    view_frame: Rect,
    toolbar: crate::container_toolbar::ContainerToolbar,
    pointer_focus: Option<bool>,
    toolbar_focused: bool,
    clip: Rect,
    scale: f32,
    viewport: u64,
    seq: u64,
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
            registration: None,
            image: None,
            frame: Rect::default(),
            view_frame: Rect::default(),
            toolbar: Default::default(),
            pointer_focus: None,
            toolbar_focused: false,
            clip: Rect::default(),
            scale: 1.,
            viewport: 0,
            seq: 0,
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
        Ok(container)
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
            process.request(crate::component::RequestBody::SetViewContext {
                view,
                context: context.clone(),
            })?;
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
        self.process.as_ref().map(ProcessComponent::status)
    }
    pub fn session(&self) -> Option<&crate::component::Session> {
        self.process.as_ref().map(ProcessComponent::session)
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// Service output stays in the existing ProcessComponent queue. Call on the
    /// supplied wake even when hidden; preparing a frame never discards replies.
    pub fn drain_events(&self) -> Vec<crate::component::ServerMessage> {
        self.process
            .as_ref()
            .map_or_else(Vec::new, ProcessComponent::drain_events)
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
        self.image.as_ref().map(|i| i.metadata().revision)
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
            self.registration = None;
            self.image = None;
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
                let current = bridge.snapshot()?;
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
        !self.geometry_dirty
            && self.visible
            && self.enabled
            && self.window_active
            && self
                .process
                .as_ref()
                .is_some_and(|p| matches!(p.status(), ProcessStatus::Ready { .. }))
            && self
                .image
                .as_ref()
                .is_some_and(|i| i.metadata().viewport == self.viewport)
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
        crate::ProcessContentBridge::new(
            self.process
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("container unmounted"))?,
            self.native
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("legacy Content unavailable"))?,
            self.viewport,
            self.view_frame,
            self.clip,
            timeout,
        )
    }
    /// Attach received GPU resources to the host Renderer before Content paint.
    /// This handles all selection/import/replacement/retirement, not the consumer.
    pub fn prepare(&mut self, renderer: &mut Renderer) -> Result<bool> {
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
        if let Some(registration) = &self.registration {
            renderer.replace_owned_gpu_image(registration, image.image())?;
        } else {
            self.registration = Some(renderer.register_owned_gpu_image(image.image())?);
        }
        self.image = Some(image);
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
        let Some(image) = &self.image else {
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
        if item.geometry_dependent && !self.input_ready() {
            return Ok(false);
        }
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
    fn query_content(&self) -> Option<crate::ProcessContentSnapshot> {
        if !(self.content_focused || self.remote_overlay()) || !self.available() {
            return None;
        }
        let current = if self.input_ready() {
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
        self.content_snapshot = Some(next);
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
                None => bridge.snapshot()?,
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
            let current = bridge.snapshot()?;
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
        self.pending_content.push_back(PendingContentInput {
            session: process.session().clone(),
            owner: snapshot.reply().focused,
            focus_epoch: snapshot.reply().focus_epoch,
            text_binding: snapshot.reply().text_binding.clone(),
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
                    bridge.snapshot()?
                } else {
                    match &self.content_snapshot {
                        Some(snapshot) => bridge.refresh_allocation(snapshot)?,
                        None => bridge.snapshot()?,
                    }
                };
                ensure!(
                    self.process
                        .as_ref()
                        .is_some_and(|p| p.session() == &pending.session)
                        && (finishing
                            || current.reply().focused == pending.owner
                                && current.reply().focus_epoch == pending.focus_epoch
                                && current.reply().text_binding == pending.text_binding),
                    "stale deferred Content callback"
                );
                bridge.request(&current, pending.input)
            })();
            match result {
                Ok(next) => {
                    let mut response = self.accept_content(next);
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
                }
                Err(error) => {
                    // Observe the current owner for future fresh intent, without
                    // applying any of the rejected old callbacks to that owner.
                    if let Ok(bridge) =
                        self.native_content_bridge(std::time::Duration::from_secs(1))
                        && let Ok(current) = bridge.snapshot()
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
                bridge.snapshot()?
            } else {
                match &self.content_snapshot {
                    Some(snapshot) => bridge.refresh_allocation(snapshot)?,
                    None => bridge.snapshot()?,
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
            let previous = bridge.snapshot()?;
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
impl Component for ProcessContainer {
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
                    && (self.input_ready() || self.content_focused && self.available()))
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let toolbar = !self.remote_overlay()
            && self
                .pointer_focus
                .take()
                .unwrap_or(self.toolbar.active && !self.content_focused);
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
                let current = bridge.snapshot()?;
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
            self.geometry_dirty = false;
            self.repaint_dirty = false;
            if let Err(error) = self.enqueue(InputKind::Layout, view.origin, None) {
                self.error = Some(error.to_string());
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
        if let (Some(registration), Some(image)) = (&self.registration, &self.image)
            && let Some(clip) = self.view_frame.intersection(&clip)
        {
            scene.push_clip(clip);
            scene.push_image(ImageQuad {
                rect: crate::panel_image_rect(
                    self.view_frame.origin,
                    (image.metadata().pixel_width, image.metadata().pixel_height),
                    self.scale,
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
                if self.process.is_none() {
                    let wake = self.wake.clone();
                    let process = if let Some(native) = &self.native {
                        ProcessComponent::spawn(
                            ProcessComponentOptions::new(self.module.clone(), self.host_epoch)
                                .with_view(native.window, native.view, native.required),
                            move || wake(),
                        )
                    } else {
                        ProcessComponent::spawn_v1_view(
                            self.module.clone(),
                            self.host_epoch,
                            move || wake(),
                        )
                    };
                    match process {
                        Ok(process) => {
                            if let (Some(native), Some(context)) =
                                (&self.native, self.view_context.as_ref())
                                && let Err(error) =
                                    process.request(crate::component::RequestBody::SetViewContext {
                                        view: native.view,
                                        context: context.clone(),
                                    })
                            {
                                self.error = Some(error.to_string());
                            }
                            self.process = Some(process);
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    };
                    self.geometry_dirty = true;
                }
            }
            Lifecycle::Unmounted => {
                self.semantic_projection.revoke();
                self.accessibility_focus = false;
                self.pressed = false;
                self.toolbar.reset();
                self.toolbar_focused = false;
                self.pointer_focus = None;
                self.registration = None;
                self.image = None;
                self.process = None;
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
