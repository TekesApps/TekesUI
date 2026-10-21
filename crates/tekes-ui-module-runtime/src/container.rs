//! Standard Content adapter for the explicit legacy View profile.
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

pub struct ProcessContainer {
    module: String,
    host_epoch: u64,
    native: Option<NativeViewOptions>,
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
    visible: bool,
    enabled: bool,
    window_active: bool,
    opaque_pointer: bool,
    pressed: bool,
    error: Option<String>,
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
            visible: true,
            enabled: true,
            window_active: true,
            opaque_pointer: false,
            pressed: false,
            error: None,
        })
    }
    /// Mount a negotiated V2 View through the same Content lifecycle. Native
    /// keyboard/IME/AX remain unavailable and must not be requested here.
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
            !matches!(body, crate::component::RequestBody::Invoke { .. })
                || self.visible && self.enabled && self.window_active,
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
            self.cancel();
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
        self.toolbar.control.cancel();
        self.pointer_focus = None;
        self.pressed = false;
        if let Some(process) = &self.process {
            match process.cancel_view_input() {
                Ok(Some(seq)) => self.seq = self.seq.max(seq),
                Ok(None) => (),
                Err(error) => self.error = Some(error.to_string()),
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
            if was_local || self.toolbar.active {
                self.cancel();
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
            self.error = Some(message.clone());
            anyhow::bail!("{message}");
        }
        if process.take_view_invalidated() {
            self.geometry_dirty = true;
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
            && (self.toolbar.active || self.opaque_pointer)
    }
    fn focusable(&self) -> bool {
        self.visible
            && self.enabled
            && self.window_active
            && self.toolbar.active
            && self.toolbar.control.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.toolbar_focused = focused && self.pointer_focus.take().unwrap_or(true);
        self.toolbar
            .control
            .set_focus(self.toolbar_focused, visible);
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if !self.toolbar_focused || !self.available() {
            return ContentResponse::default();
        }
        let response = self.toolbar.key(key);
        self.finish_toolbar(response)
    }
    fn overlay_open(&self) -> bool {
        self.visible && self.toolbar.active && self.toolbar.control.is_open()
    }
    fn overlay_key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        self.key_event(key)
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        if !self.available() {
            return ContentResponse::default();
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
        let changed = (
            self.view_frame.size.width as u32,
            self.view_frame.size.height as u32,
        ) != (view.size.width as u32, view.size.height as u32);
        if changed || self.clip != clip {
            self.cancel();
        }
        if changed {
            self.geometry_dirty = true;
        }
        self.frame = frame;
        self.view_frame = view;
        self.clip = clip;
        if self.process.is_some()
            && self.visible
            && view.size.width >= 1.
            && view.size.height >= 1.
            && (self.geometry_dirty || changed)
        {
            self.viewport += 1;
            self.geometry_dirty = false;
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
        tekes_ui_controls::paint::surface_at_scale(scene, frame, theme.palette, true, self.scale);
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
        match event {
            ContentEvent::Cancel => {
                self.cancel();
                ContentResponse::default()
            }
            ContentEvent::Pointer(pointer) => self.pointer_event(pointer),
            ContentEvent::PointerDown(point)
            | ContentEvent::PointerMove(point)
            | ContentEvent::PointerUp(point) => self.pointer_event(&PointerEvent {
                position: *point,
                phase: match event {
                    ContentEvent::PointerDown(_) => PointerPhase::Down,
                    ContentEvent::PointerUp(_) => PointerPhase::Up,
                    _ => PointerPhase::Move,
                },
                button: Some(PointerButton::Primary),
                modifiers: Default::default(),
                click_count: 1,
            }),
            _ => {
                if !self.available() {
                    return ContentResponse::default();
                }
                let response = self.toolbar.event(event);
                self.finish_toolbar(response)
            }
        }
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if !self.available() {
            return ContentResponse::default();
        }
        let local = self.toolbar.pointer(pointer);
        if local.consumed {
            if pointer.phase == PointerPhase::Down {
                self.pointer_focus = Some(true);
            }
            return self.finish_toolbar(local);
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
                        Ok(process) => self.process = Some(process),
                        Err(error) => self.error = Some(error.to_string()),
                    };
                    self.geometry_dirty = true;
                }
            }
            Lifecycle::Unmounted => {
                self.pressed = false;
                self.toolbar.reset();
                self.toolbar_focused = false;
                self.pointer_focus = None;
                self.registration = None;
                self.image = None;
                self.process = None;
                (self.wake)();
            }
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, &self.module);
        semantics.disabled = !self.enabled;
        semantics
    }
}
