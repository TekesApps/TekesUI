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
        Component, ContentEvent, ContentResponse, ContentRole, ContentSemantics, ContentTheme,
        Lifecycle,
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
    pub fn status(&self) -> Option<ProcessStatus> {
        self.process.as_ref().map(ProcessComponent::status)
    }
    pub fn session(&self) -> Option<&crate::component::Session> {
        self.process.as_ref().map(ProcessComponent::session)
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
            (self.wake)();
        }
    }
    pub fn set_window_active(&mut self, active: bool) {
        if self.window_active != active {
            self.cancel();
            self.window_active = active;
            (self.wake)();
        }
    }
    pub fn set_visible(&mut self, visible: bool) -> Result<()> {
        if self.visible == visible {
            return Ok(());
        }
        self.pressed = false;
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
            x: point.x - self.frame.min_x(),
            y: point.y - self.frame.min_y(),
            width: self.frame.size.width as u32,
            height: self.frame.size.height as u32,
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
        self.visible && self.enabled && self.opaque_pointer && self.window_active
    }
    fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let changed = (self.frame.size.width as u32, self.frame.size.height as u32)
            != (frame.size.width as u32, frame.size.height as u32);
        if changed {
            self.cancel();
        }
        self.frame = frame;
        self.clip = clip;
        if self.process.is_some()
            && self.visible
            && frame.size.width >= 1.
            && frame.size.height >= 1.
            && (self.geometry_dirty || changed)
        {
            self.viewport += 1;
            self.geometry_dirty = false;
            if let Err(error) = self.enqueue(
                InputKind::Layout,
                Point::new(frame.min_x(), frame.min_y()),
                None,
            ) {
                self.error = Some(error.to_string());
            }
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        tekes_ui_controls::paint::surface_at_scale(scene, frame, theme.palette, true, self.scale);
        if let (Some(registration), Some(image)) = (&self.registration, &self.image)
            && let Some(clip) = frame.intersection(&clip)
        {
            scene.push_clip(clip);
            scene.push_image(ImageQuad {
                rect: crate::panel_image_rect(
                    frame.origin,
                    (image.metadata().pixel_width, image.metadata().pixel_height),
                    self.scale,
                ),
                image: registration.id(),
                opacity: 1.,
            });
            scene.pop_clip();
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::Cancel) {
            self.cancel();
            return ContentResponse::default();
        }
        if !self.opaque_pointer || !self.input_ready() {
            return ContentResponse::default();
        }
        match event {
            ContentEvent::PointerDown(point)
                if self
                    .frame
                    .intersection(&self.clip)
                    .is_some_and(|r| r.contains(*point)) =>
            {
                self.pressed = true;
                self.send_event(InputKind::PointerDown, *point, true)
            }
            ContentEvent::PointerMove(point) if self.pressed => {
                self.send_event(InputKind::PointerMove, *point, true)
            }
            ContentEvent::PointerUp(point) if self.pressed => {
                self.pressed = false;
                self.send_event(InputKind::PointerUp, *point, false)
            }
            _ => ContentResponse::default(),
        }
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
