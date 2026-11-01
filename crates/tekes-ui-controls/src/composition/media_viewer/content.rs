//! Content adapter over the single MediaViewer geometry and ImageContent owners.
use super::*;
use tekes_ui_scene::{ClipError, ClipShape};

// Rectangles and canonical rounded rectangles are convex: checking all four
// corners certifies this entire painted focus boundary, not a sampled point.
fn focus_boundary_inside(rect: Rect, shape: ClipShape) -> Result<bool, ClipError> {
    for point in [
        [rect.min_x(), rect.min_y()],
        [rect.max_x(), rect.min_y()],
        [rect.min_x(), rect.max_y()],
        [rect.max_x(), rect.max_y()],
    ] {
        if !shape.contains([f64::from(point[0]), f64::from(point[1])])? {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    pointer: Point,
    translation: Point,
}

/// A local, optional image viewport. Dialog, Close, Retry and toolbar Buttons
/// compose around this Content through the original Host. No I/O or decoding.
///
/// Use begin_load/complete on this owner, rather than mutating its ImageContent
/// child directly: the accepted source change and geometry reset are one update.
/// After direct public commands or completions, request the host's normal redraw.
#[derive(Debug)]
pub struct MediaViewerContent {
    children: Vec<Content>,
    image_id: ComponentId,
    name: String,
    geometry: Geometry,
    viewport: Rect,
    drag: Option<Drag>,
    disabled: bool,
    window_active: bool,
    runnable: bool,
    focused: bool,
    focus_visible: bool,
    backing_scale: f32,
    focus_layout: Option<(Rect, Rect)>,
    focus_context: Option<CurveGeometryContext>,
    focus_geometry_pending: bool,
    focus_space: RequiredFocusSpace,
    changes: Vec<MediaViewerTransform>,
}
impl MediaViewerContent {
    pub fn new(
        image_id: ComponentId,
        image: ImageContent,
        name: impl Into<String>,
    ) -> Result<Self, MediaViewerError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MediaViewerError::InvalidName);
        }
        let geometry = Geometry::new(image.logical_size())?;
        Ok(Self {
            children: vec![Content::new(image_id, image.with_fit(ImageFit::Contain))],
            image_id,
            name,
            geometry,
            viewport: Rect::default(),
            drag: None,
            disabled: false,
            window_active: true,
            runnable: false,
            focused: false,
            focus_visible: false,
            backing_scale: 1.0,
            focus_layout: None,
            focus_context: None,
            focus_geometry_pending: false,
            focus_space: RequiredFocusSpace::Unmeasured,
            changes: Vec::new(),
        })
    }
    /// The whole visible reading-viewport boundary is the keyboard focus
    /// region. Its conservative flat-center fit is both painted and certified;
    /// it never substitutes a tiny internal point for the viewport.
    fn refresh_focus_space(&mut self) {
        if self.focus_geometry_pending
            || self.focus_layout.is_none()
            || !self.backing_scale.is_finite()
            || self.backing_scale <= 0.0
        {
            self.focus_space = RequiredFocusSpace::Unmeasured;
            return;
        }
        self.focus_space = RequiredFocusSpace::Insufficient;
        if self.disabled || !self.window_active || !self.runnable {
            return;
        }
        let (frame, clip) = self.focus_layout.expect("arranged focus allocation");
        let Some(mut boundary) = frame.intersection(&clip) else {
            return;
        };
        if let Some(context) = &self.focus_context {
            for shape in context.shapes() {
                let Some(visible) = boundary.intersection(&shape.rect()) else {
                    return;
                };
                boundary = visible;
                match focus_boundary_inside(boundary, *shape) {
                    Ok(true) => {}
                    Ok(false) => {
                        // The flat center is a conservative logical rectangle
                        // of this actual shape. No physical-pixel/dyadic proof
                        // API or assumed backing scale is substituted here.
                        let radii = shape.radii();
                        let radius = radii
                            .top_left
                            .max(radii.top_right)
                            .max(radii.bottom_left)
                            .max(radii.bottom_right);
                        let Some(inner) = boundary.intersection(&shape.rect().inset(radius)) else {
                            return;
                        };
                        boundary = inner;
                    }
                    Err(_) => {
                        // The Component focus contract reports an uncertified
                        // numeric query as Unmeasured; it must never grant focus.
                        self.focus_space = RequiredFocusSpace::Unmeasured;
                        return;
                    }
                }
            }
        }
        let metrics = crate::extended::FocusRingMetrics::new(self.backing_scale);
        // Reserve the current backing-pixel AA guard inside the allocation.
        // The witness includes that guard as well as the original ring stroke/gap.
        boundary = boundary.inset(metrics.antialias_guard);
        if boundary.size.width <= 2.0 * metrics.reserved_band()
            || boundary.size.height <= 2.0 * metrics.reserved_band()
        {
            return;
        }
        if let Some(context) = &self.focus_context {
            for shape in context.shapes() {
                match focus_boundary_inside(boundary, *shape) {
                    Ok(true) => {}
                    Ok(false) => return,
                    Err(_) => {
                        self.focus_space = RequiredFocusSpace::Unmeasured;
                        return;
                    }
                }
            }
        }
        self.focus_space = RequiredFocusSpace::Sufficient {
            witness: boundary,
            evidence: RequiredFocusEvidence::new(),
        };
    }
    fn image(&self) -> Option<&ImageContent> {
        self.children
            .first()
            .filter(|child| self.children.len() == 1 && child.id() == self.image_id)?
            .component()
    }
    fn image_mut(&mut self) -> Option<&mut ImageContent> {
        if self.children.len() != 1 || self.children[0].id() != self.image_id {
            return None;
        }
        self.children[0].component_mut()
    }
    fn input_ready(&self) -> bool {
        !self.disabled
            && self.window_active
            && self.runnable
            && self.geometry.transform().is_some()
            && self.image().is_some_and(|image| {
                image.status() != ImageLoadStatus::Loading
                    && image.displayed_logical_size().is_some()
            })
    }
    fn record_change(&mut self, before: Option<MediaViewerTransform>) {
        if let Some(after) = self.geometry.transform()
            && Some(after) != before
        {
            self.changes.push(after);
        }
    }
    /// Changes are drained once; reads and no-op commands never enqueue a change.
    pub fn drain_transform_changes(&mut self) -> Vec<MediaViewerTransform> {
        std::mem::take(&mut self.changes)
    }
    pub fn transform(&self) -> Option<MediaViewerTransform> {
        self.geometry.transform()
    }
    pub fn image_status(&self) -> Option<ImageLoadStatus> {
        self.image().map(ImageContent::status)
    }
    pub fn failure_message(&self) -> Option<&str> {
        self.image().and_then(ImageContent::failure_message)
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        if disabled {
            self.drag = None;
        }
        self.refresh_focus_space();
    }
    /// Availability for original Button disabled semantics and AX Press routing.
    pub fn can_execute(&self, command: MediaViewerCommand) -> bool {
        if !self.input_ready() {
            return false;
        }
        let transform = self.geometry.transform().expect("input-ready geometry");
        match command {
            MediaViewerCommand::ZoomIn => transform.relative_scale < 8.0,
            MediaViewerCommand::ZoomOut => transform.relative_scale > 1.0,
            MediaViewerCommand::ActualLogicalSize => self.geometry.can_actual(),
            MediaViewerCommand::PanBy { delta } => {
                delta.x.is_finite() && delta.y.is_finite() && self.geometry.can_pan()
            }
            MediaViewerCommand::ZoomAt {
                relative_scale,
                anchor,
            } => {
                relative_scale.is_finite()
                    && anchor.x.is_finite()
                    && anchor.y.is_finite()
                    && anchor.x >= 0.0
                    && anchor.y >= 0.0
                    && anchor.x < self.viewport.size.width
                    && anchor.y < self.viewport.size.height
            }
            MediaViewerCommand::Fit => true,
        }
    }
    /// Invalid inputs return an error without cancelling an active gesture.
    /// Unavailable but valid commands are explicit no-ops.
    pub fn execute(&mut self, command: MediaViewerCommand) -> Result<bool, MediaViewerError> {
        match command {
            MediaViewerCommand::ZoomAt {
                relative_scale,
                anchor,
            } if !relative_scale.is_finite()
                || !anchor.x.is_finite()
                || !anchor.y.is_finite()
                || anchor.x < 0.0
                || anchor.y < 0.0
                || anchor.x >= self.viewport.size.width
                || anchor.y >= self.viewport.size.height =>
            {
                return Err(MediaViewerError::InvalidTransform);
            }
            MediaViewerCommand::PanBy { delta } if !delta.x.is_finite() || !delta.y.is_finite() => {
                return Err(MediaViewerError::InvalidTransform);
            }
            _ => {}
        }
        if !self.input_ready() {
            return Ok(false);
        }
        let before = self.geometry.transform();
        let center = Point::new(
            self.viewport.size.width / 2.0,
            self.viewport.size.height / 2.0,
        );
        let changed = match command {
            MediaViewerCommand::Fit => self.geometry.fit(),
            MediaViewerCommand::ActualLogicalSize => self.geometry.actual(),
            MediaViewerCommand::ZoomIn => self
                .geometry
                .zoom(before.unwrap().relative_scale * 1.25, center)?,
            MediaViewerCommand::ZoomOut => self
                .geometry
                .zoom(before.unwrap().relative_scale / 1.25, center)?,
            MediaViewerCommand::ZoomAt {
                relative_scale,
                anchor,
            } => self.geometry.zoom(relative_scale, anchor)?,
            MediaViewerCommand::PanBy { delta } => self.geometry.pan(delta)?,
        };
        self.drag = None;
        self.record_change(before);
        Ok(changed)
    }
    pub fn begin_load(&mut self) -> Result<ImageLoadRequest, MediaViewerError> {
        let request = self
            .image_mut()
            .ok_or(MediaViewerError::InvalidImageChild)?
            .begin_load()?;
        self.drag = None;
        Ok(request)
    }
    /// Exactly the original ImageContent capability admits a host completion.
    /// Stale/cross-owner results have no Viewer side effects.
    pub fn complete(
        &mut self,
        request: ImageLoadRequest,
        outcome: ImageLoadOutcome,
    ) -> Result<bool, MediaViewerError> {
        let ready = matches!(outcome, ImageLoadOutcome::Ready { .. });
        let accepted = self
            .image_mut()
            .ok_or(MediaViewerError::InvalidImageChild)?
            .complete(request, outcome)?;
        if accepted {
            self.drag = None;
            if ready {
                let size = self.image().unwrap().logical_size();
                let before = self.geometry.transform();
                self.geometry
                    .set_source(size)
                    .expect("ImageContent accepted logical size");
                self.record_change(before);
            }
        }
        Ok(accepted)
    }
}
impl Component for MediaViewerContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.image().is_none() {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let before = self.geometry.transform();
        if self.viewport != frame {
            self.drag = None;
        }
        self.viewport = frame;
        self.backing_scale = text.backing_scale();
        self.focus_layout = Some((frame, clip));
        self.focus_context = None;
        self.focus_geometry_pending = false;
        self.refresh_focus_space();
        self.geometry.set_viewport(frame.size);
        self.record_change(before);
        let allocation = self.geometry.image_rect(frame.origin).unwrap_or_default();
        let visible = frame.intersection(&clip).unwrap_or_default();
        if let Some(child) = self.children.first_mut() {
            child.arrange(text, allocation, visible);
        }
    }
    fn set_clip_region(&mut self, _: Option<tekes_ui_scene::ClipRegion>) {
        // Host publishes the matching context immediately after this callback.
        // Intermediate/cleared geometry cannot retain the prior witness token.
        self.focus_geometry_pending = true;
        self.focus_space = RequiredFocusSpace::Unmeasured;
    }
    fn set_curve_geometry_context(&mut self, context: Option<CurveGeometryContext>) {
        if self.focus_context.is_some() && context.is_none() {
            // Revocation outside a new arrange must not mint unconstrained
            // authority for the old curved allocation.
            self.focus_layout = None;
        }
        self.focus_context = context;
        self.focus_geometry_pending = false;
        self.refresh_focus_space();
    }
    fn requires_focus_reveal(&self) -> bool {
        true
    }
    fn required_focus_space(&self) -> RequiredFocusSpace {
        self.focus_space.clone()
    }
    fn required_focus_result(&self) -> Option<RequiredFocusResult> {
        Some(match &self.focus_space {
            RequiredFocusSpace::Sufficient { witness, .. } => {
                RequiredFocusResult::Visible { witness: *witness }
            }
            RequiredFocusSpace::Insufficient => RequiredFocusResult::Insufficient,
            _ => RequiredFocusResult::Unmeasured,
        })
    }
    fn focus_boundary_visible(&self) -> bool {
        self.focused
            && self.focus_visible
            && matches!(self.focus_space, RequiredFocusSpace::Sufficient { .. })
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        if text.backing_scale() != self.backing_scale {
            // A paint using a different renderer scale needs a new arrange;
            // do not publish or paint the old coverage evidence at that scale.
            self.focus_space = RequiredFocusSpace::Unmeasured;
            self.focus_geometry_pending = true;
        }
        // Paint the retained original ImageContent once, then its viewport
        // focus boundary above the bitmap. The ordinary tree still owns input,
        // layout, resource leases, semantics and lifecycle.
        if let Some(image) = self.children.first_mut() {
            image.paint(scene, text, theme);
        }
        if self.focus_boundary_visible()
            && let RequiredFocusSpace::Sufficient { witness, .. } = &self.focus_space
        {
            let metrics = crate::extended::FocusRingMetrics::new(self.backing_scale);
            // The original painter expands by width+gap; the remaining guard
            // is reserved inside this same required coverage witness.
            crate::paint::focus_ring(
                scene,
                witness.inset(metrics.reserved_band()),
                theme.palette,
                0.0,
            );
        }
    }
    // Pure projection: cancellation clears drag; only an admitted primary Down
    // can create it again. Querying never reacquires capture or emits a change.
    fn pointer_capture_current(&self) -> bool {
        self.drag.is_some() && self.input_ready()
    }
    fn focusable(&self) -> bool {
        !self.disabled
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.focused = focused;
        self.focus_visible = focused && visible;
        if !focused {
            self.drag = None;
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.window_active = active;
        if !active {
            self.drag = None;
        }
        self.refresh_focus_space();
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.runnable = runnable;
        if !runnable {
            self.drag = None;
        }
        self.refresh_focus_space();
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.drag = None;
            self.runnable = false;
            self.focused = false;
            self.focus_visible = false;
            self.focus_layout = None;
            self.focus_context = None;
            self.focus_geometry_pending = false;
            self.focus_space = RequiredFocusSpace::Unmeasured;
            self.geometry.fit();
            self.changes.clear();
        }
    }
    fn help(&self) -> Option<(&str, u64)> {
        Some((
            "Use arrow keys to pan. Use the zoom and Fit buttons to change the image view. Escape cancels a drag; a modal host may close the viewer.",
            0,
        ))
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, self.name.clone());
        semantics.disabled = self.disabled;
        semantics.value = self
            .transform()
            .map(|transform| format!("{:.0}% logical size", transform.scale * 100.0));
        semantics
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Cancel => {
                let changed = self.drag.take().is_some();
                return ContentResponse::consumed(changed);
            }
            ContentEvent::Key {
                key: ControlKey::Escape,
                down: true,
            } if self.drag.is_some() => {
                self.drag = None;
                return ContentResponse::consumed(true);
            }
            _ => {}
        }
        if !self.input_ready() {
            return ContentResponse::default();
        }
        match event {
            ContentEvent::PointerDown(point) if self.geometry.can_pan() => {
                self.drag = Some(Drag {
                    pointer: *point,
                    translation: self.transform().unwrap().translation,
                });
                ContentResponse {
                    consumed: true,
                    capture_pointer: true,
                    ..Default::default()
                }
            }
            ContentEvent::PointerDown(_) => ContentResponse::consumed(false),
            ContentEvent::PointerMove(point) => {
                let Some(drag) = self.drag else {
                    return ContentResponse::default();
                };
                let current = self.transform().unwrap().translation;
                let delta = Point::new(
                    drag.translation.x + point.x - drag.pointer.x - current.x,
                    drag.translation.y + point.y - drag.pointer.y - current.y,
                );
                let before = self.transform();
                match self.geometry.pan(delta) {
                    Ok(changed) => {
                        self.record_change(before);
                        ContentResponse::consumed(changed)
                    }
                    Err(_) => ContentResponse::default(), // Reject invalid pointer coordinates without changing the session.
                }
            }
            ContentEvent::PointerUp(_) => ContentResponse::consumed(self.drag.take().is_some()),
            ContentEvent::Key { key, down: true } if self.focused => {
                let delta = match key {
                    ControlKey::Left => Point::new(32.0, 0.0),
                    ControlKey::Right => Point::new(-32.0, 0.0),
                    ControlKey::Up => Point::new(0.0, 32.0),
                    ControlKey::Down => Point::new(0.0, -32.0),
                    _ => return ContentResponse::default(),
                };
                ContentResponse::consumed(
                    self.execute(MediaViewerCommand::PanBy { delta })
                        .expect("finite key delta"),
                )
            }
            _ => ContentResponse::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonTextAlign;
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(0.0, 0.0)
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
    fn loading() -> MediaViewerContent {
        MediaViewerContent::new(
            ComponentId(7202),
            ImageContent::loading(
                Size::new(200.0, 100.0),
                ImageSemantics::Alternative("Diagram".into()),
            )
            .unwrap(),
            "Diagram viewer",
        )
        .unwrap()
    }
    fn layout(viewer: &mut MediaViewerContent, frame: Rect, clip: Rect) {
        viewer.arrange(&mut Text, frame, clip);
        viewer.presentation_runnable_changed(true);
    }
    #[test]
    fn viewport_and_ancestor_clip_arrange_the_original_image_once() {
        let mut viewer = loading();
        layout(
            &mut viewer,
            Rect::new(10.0, 20.0, 100.0, 100.0),
            Rect::new(40.0, 10.0, 30.0, 100.0),
        );
        assert_eq!(
            viewer.children[0].frame(),
            Rect::new(10.0, 45.0, 100.0, 50.0)
        );
        assert_eq!(
            viewer.children[0].visible(),
            Some(Rect::new(40.0, 45.0, 30.0, 50.0))
        );
        assert_eq!(
            viewer.children[0]
                .component::<ImageContent>()
                .unwrap()
                .semantics(),
            ContentSemantics::image_alt("Diagram").unwrap()
        );
        assert_eq!(viewer.children.len(), 1);
        layout(
            &mut viewer,
            Rect::new(10.0, 20.0, 0.0, 100.0),
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );
        assert!(viewer.transform().is_none());
        assert!(viewer.children[0].visible().is_none());
    }
    #[test]
    fn stale_foreign_and_replayed_completions_leave_viewer_unchanged() {
        let mut viewer = loading();
        let mut other = loading();
        let old = viewer.begin_load().unwrap();
        let current = viewer.begin_load().unwrap();
        let before = viewer.transform();
        assert!(!viewer.complete(old, ImageLoadOutcome::Missing).unwrap());
        assert!(
            !other
                .complete(current.clone(), ImageLoadOutcome::Missing)
                .unwrap()
        );
        assert_eq!(viewer.transform(), before);
        assert!(
            viewer
                .complete(
                    current.clone(),
                    ImageLoadOutcome::Failed("unavailable".into())
                )
                .unwrap()
        );
        assert_eq!(viewer.failure_message(), Some("unavailable"));
        assert!(!viewer.complete(current, ImageLoadOutcome::Missing).unwrap());
        assert_eq!(viewer.image_status(), Some(ImageLoadStatus::Failed));
        assert!(viewer.drain_transform_changes().is_empty());
    }
    #[test]
    fn original_host_unmount_invalidates_load_capability() {
        let mut viewer = loading();
        let request = viewer.begin_load().unwrap();
        let id = ComponentId(7201);
        let mut content = Content::new(id, viewer);
        content.lifecycle(Lifecycle::Unmounted);
        let mut host = ContentHost::new(content).unwrap();
        let viewer = host
            .get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        assert!(!viewer.complete(request, ImageLoadOutcome::Missing).unwrap());
        assert!(viewer.drain_transform_changes().is_empty());
    }
    #[test]
    fn damaged_child_is_a_source_error_instead_of_a_panic() {
        let mut viewer = loading();
        let request = viewer.begin_load().unwrap();
        viewer.children.clear();
        assert!(matches!(
            viewer.begin_load(),
            Err(MediaViewerError::InvalidImageChild)
        ));
        assert!(matches!(
            viewer.complete(request, ImageLoadOutcome::Missing),
            Err(MediaViewerError::InvalidImageChild)
        ));
        assert!(viewer.validate().is_err());
    }
    #[test]
    fn loading_missing_and_disabled_commands_have_no_side_effects() {
        let mut viewer = loading();
        let frame = Rect::new(0.0, 0.0, 100.0, 100.0);
        layout(&mut viewer, frame, frame);
        viewer.drain_transform_changes();
        let before = viewer.transform();
        assert!(!viewer.execute(MediaViewerCommand::ZoomIn).unwrap());
        let request = viewer.begin_load().unwrap();
        viewer.complete(request, ImageLoadOutcome::Missing).unwrap();
        assert!(!viewer.can_execute(MediaViewerCommand::Fit));
        assert!(
            !viewer
                .event(&ContentEvent::PointerDown(Point::new(20.0, 20.0)))
                .capture_pointer
        );
        viewer.set_disabled(true);
        assert!(!viewer.focusable());
        assert!(viewer.semantics().disabled);
        assert_eq!(viewer.transform(), before);
        assert!(viewer.drain_transform_changes().is_empty());
    }

    /// A CPU-only interactive Component exercising the real Host contract.
    /// It owns no ImageContent and proves neither bitmap readiness nor GPU input admission.
    #[derive(Debug)]
    struct CaptureProbe {
        live: bool,
        frame: Rect,
        last: Point,
        translation: Point,
        effective_cancels: usize,
    }
    impl CaptureProbe {
        fn cancel(&mut self) {
            if std::mem::take(&mut self.live) {
                self.effective_cancels += 1;
            }
        }
    }
    impl Component for CaptureProbe {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(available)
        }
        fn focusable(&self) -> bool {
            true
        }
        fn rectangular_input_geometry(&self) -> bool {
            true
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Group, "CPU capture contract probe")
        }
        fn pointer_capture_current(&self) -> bool {
            self.live
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
            if self.frame != frame {
                self.cancel();
            }
            self.frame = frame;
        }
        fn set_focus(&mut self, focused: bool, _: bool) {
            if !focused {
                self.cancel();
            }
        }
        fn event(&mut self, event: &ContentEvent) -> ContentResponse {
            match event {
                ContentEvent::PointerDown(point) => {
                    self.live = true;
                    self.last = *point;
                    ContentResponse {
                        consumed: true,
                        capture_pointer: true,
                        ..Default::default()
                    }
                }
                ContentEvent::PointerMove(point) if self.live => {
                    self.translation.x += point.x - self.last.x;
                    self.translation.y += point.y - self.last.y;
                    self.last = *point;
                    ContentResponse::consumed(true)
                }
                ContentEvent::PointerUp(_) if self.live => {
                    self.live = false;
                    ContentResponse::consumed(false)
                }
                ContentEvent::Cancel
                | ContentEvent::Key {
                    key: ControlKey::Escape,
                    down: true,
                } => {
                    self.cancel();
                    ContentResponse::consumed(false)
                }
                _ => ContentResponse::default(),
            }
        }
    }
    fn capture_host() -> ContentHost {
        let mut host = ContentHost::new(Content::new(
            ComponentId(7401),
            CaptureProbe {
                live: false,
                frame: Rect::default(),
                last: Point::new(0.0, 0.0),
                translation: Point::new(0.0, 0.0),
                effective_cancels: 0,
            },
        ))
        .unwrap();
        let frame = Rect::new(0.0, 0.0, 100.0, 100.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        host
    }
    fn probe(host: &ContentHost) -> &CaptureProbe {
        host.get(ComponentId(7401))
            .unwrap()
            .component::<CaptureProbe>()
            .unwrap()
    }
    fn assert_retired_and_reacquire(host: &mut ContentHost) {
        let before = probe(host).translation;
        let cancels = probe(host).effective_cancels;
        // Repeated publication queries are pure. Reconciliation may deliver an
        // idempotent Cancel, but cannot cancel the already-retired gesture again.
        assert!(host.pointer_capture().is_none());
        assert!(host.pointer_capture_lease().is_none());
        assert!(host.pointer_capture().is_none());
        assert_eq!(probe(host).effective_cancels, cancels);
        host.event(&ContentEvent::PointerMove(Point::new(70.0, 70.0)));
        host.event(&ContentEvent::PointerUp(Point::new(70.0, 70.0)));
        host.event(&ContentEvent::PointerMove(Point::new(80.0, 80.0)));
        assert_eq!(probe(host).translation, before);
        assert_eq!(probe(host).effective_cancels, cancels);
        host.event(&ContentEvent::PointerDown(Point::new(20.0, 20.0)));
        assert!(host.pointer_capture_lease().is_some());
        host.event(&ContentEvent::PointerMove(Point::new(30.0, 35.0)));
        assert_eq!(
            probe(host).translation,
            Point::new(before.x + 10.0, before.y + 15.0)
        );
    }
    #[test]
    fn cpu_host_capture_query_and_reconcile_do_not_duplicate_cancellation() {
        let mut host = capture_host();
        host.event(&ContentEvent::PointerDown(Point::new(20.0, 20.0)));
        let first = host.pointer_capture_lease().unwrap();
        assert_eq!(host.pointer_capture_lease(), Some(first));
        assert_eq!(probe(&host).effective_cancels, 0);
        host.get_mut(ComponentId(7401))
            .unwrap()
            .component_mut::<CaptureProbe>()
            .unwrap()
            .cancel();
        assert_retired_and_reacquire(&mut host);
        assert_ne!(host.pointer_capture_lease().unwrap().2, first.2);
    }
    #[test]
    fn cpu_host_resize_retires_capture_before_stale_move_up_and_allows_new_down() {
        let mut host = capture_host();
        host.event(&ContentEvent::PointerDown(Point::new(20.0, 20.0)));
        assert!(host.pointer_capture_lease().is_some());
        let frame = Rect::new(10.0, 10.0, 120.0, 100.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(probe(&host).effective_cancels, 1);
        assert_retired_and_reacquire(&mut host);
    }
    #[test]
    fn cpu_host_blur_and_escape_revoke_capture_without_moving_content() {
        for escape in [false, true] {
            let mut host = capture_host();
            host.event(&ContentEvent::PointerDown(Point::new(20.0, 20.0)));
            assert!(host.pointer_capture_lease().is_some());
            if escape {
                host.event(&ContentEvent::Key {
                    key: ControlKey::Escape,
                    down: true,
                });
            } else {
                assert!(host.focus(None, false));
            }
            assert_eq!(probe(&host).effective_cancels, 1);
            assert_retired_and_reacquire(&mut host);
        }
    }
    #[test]
    fn cpu_viewer_cancellation_entries_retire_existing_drag_without_transform_changes() {
        for reason in [
            "resize",
            "begin_load",
            "accepted_missing",
            "accepted_failure",
            "blur",
            "escape",
        ] {
            let mut viewer = loading();
            let frame = Rect::new(0.0, 0.0, 100.0, 100.0);
            layout(&mut viewer, frame, frame);
            viewer.geometry.zoom(4.0, Point::new(50.0, 50.0)).unwrap();
            let request = viewer.begin_load().unwrap();
            // Seed only an existing reducer session. No fake Ready state or
            // image lease: actual ready Down admission remains in the GPU tests.
            viewer.drag = Some(Drag {
                pointer: Point::new(20.0, 20.0),
                translation: viewer.transform().unwrap().translation,
            });
            let before = viewer.transform();
            match reason {
                "resize" => layout(&mut viewer, Rect::new(10.0, 0.0, 100.0, 100.0), frame),
                "begin_load" => {
                    viewer.begin_load().unwrap();
                }
                "accepted_missing" => {
                    assert!(viewer.complete(request, ImageLoadOutcome::Missing).unwrap());
                }
                "accepted_failure" => {
                    assert!(
                        viewer
                            .complete(request, ImageLoadOutcome::Failed("offline".into()))
                            .unwrap()
                    );
                }
                "blur" => viewer.set_focus(false, false),
                "escape" => {
                    viewer.event(&ContentEvent::Key {
                        key: ControlKey::Escape,
                        down: true,
                    });
                }
                _ => unreachable!(),
            }
            assert!(viewer.drag.is_none(), "{reason}");
            assert!(!viewer.pointer_capture_current(), "{reason}");
            viewer.event(&ContentEvent::PointerMove(Point::new(70.0, 70.0)));
            viewer.event(&ContentEvent::PointerUp(Point::new(70.0, 70.0)));
            assert_eq!(viewer.transform(), before, "{reason}");
        }
    }

    // Actual failing bounds, with a real original Retry Button before the
    // original Viewer. Only layout is a CPU fixture; no fake Ready or GPU lease.
    #[derive(Debug)]
    struct ObservedNativeBounds {
        children: Vec<Content>,
        arranged: bool,
    }
    impl Component for ObservedNativeBounds {
        fn lifecycle(&mut self, event: Lifecycle) {
            if event == Lifecycle::Unmounted {
                self.arranged = false;
            }
        }
        fn fixed_slots(&self) -> bool {
            true
        }
        fn host_reveal_passthrough(&self) -> bool {
            // Two retained slots have fixed relative offsets, sizes and clips
            // under pure ancestor translation after this fixture's arrange.
            self.arranged
        }
        fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
            CurveSubtreeCapability::Recursive
        }
        fn children(&self) -> &[Content] {
            &self.children
        }
        fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
            Some(&mut self.children)
        }
        fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            self.children[0].measure(text, Size::new(available.width, 24.0));
            self.children[1].measure(
                text,
                Size::new(available.width, (available.height - 188.0).max(0.0)),
            );
            MeasuredChild::new(available)
        }
        fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
            self.arranged = true;
            self.children[0].arrange(
                text,
                Rect::new(frame.min_x(), frame.min_y() + 140.0, frame.size.width, 24.0),
                clip,
            );
            self.children[1].arrange(
                text,
                Rect::new(
                    frame.min_x(),
                    frame.min_y() + 188.0,
                    frame.size.width,
                    (frame.size.height - 188.0).max(0.0),
                ),
                clip,
            );
        }
    }
    const FOCUS_VIEW: ComponentId = ComponentId(7601);
    const FOCUS_RETRY: ComponentId = ComponentId(7603);
    fn observed_focus_host() -> ContentHost {
        let body = Content::new(
            ComponentId(7604),
            ObservedNativeBounds {
                arranged: false,
                children: vec![
                    Content::new(FOCUS_RETRY, Button::new("Retry")),
                    Content::new(FOCUS_VIEW, loading()),
                ],
            },
        );
        let mut panel = PanelContent::new(body).unwrap();
        panel.set_padding(Some(crate::Insets::ZERO)).unwrap();
        ContentHost::new(Content::new(ComponentId(7600), panel)).unwrap()
    }
    struct ScaledText(f32);
    impl ButtonTextRenderer for ScaledText {
        fn backing_scale(&self) -> f32 {
            self.0
        }
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(0.0, 0.0)
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
    fn focus_scene_with_scale(host: &mut ContentHost, frame: Rect, scale: f32) -> Scene {
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut ScaledText(scale),
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        scene
    }
    fn focus_scene(host: &mut ContentHost, frame: Rect) -> Scene {
        let mut scene = Scene::new();
        host.render(&mut scene, &mut Text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        scene
    }
    fn focus_space(host: &ContentHost) -> RequiredFocusSpace {
        host.get(FOCUS_VIEW)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .required_focus_space()
    }
    #[test]
    fn cpu_focus_native_bounds_tab_retry_to_view_uses_same_painted_coverage() {
        let mut host = observed_focus_host();
        let frame = Rect::new(0.0, 0.0, 480.0, 720.0);
        focus_scene(&mut host, frame);
        let RequiredFocusSpace::Sufficient { witness, evidence } = focus_space(&host) else {
            panic!("visible viewport must have a real focus boundary");
        };
        assert_eq!(
            host.get(FOCUS_VIEW).unwrap().frame(),
            Rect::new(0.0, 188.0, 480.0, 532.0)
        );
        // Panel's 10pt curve leaves the full meaningful flat-center rectangle,
        // not a tiny central focus target. At 1x the AA coverage guard is 1pt.
        assert_eq!(witness, Rect::new(11.0, 189.0, 458.0, 520.0));
        assert_eq!(
            focus_space(&host),
            RequiredFocusSpace::Sufficient { witness, evidence }
        );
        let tab = ContentEvent::Key {
            key: ControlKey::Tab { reverse: false },
            down: true,
        };
        host.event(&tab);
        assert_eq!(host.focused(), Some(FOCUS_RETRY));
        host.event(&tab);
        assert_eq!(
            host.focused(),
            Some(FOCUS_VIEW),
            "real Host traversal must admit the viewport after Retry"
        );
        let scene = focus_scene(&mut host, frame);
        let RequiredFocusSpace::Sufficient { witness, .. } = focus_space(&host) else {
            panic!("focused coverage remains current");
        };
        let metrics = crate::extended::FocusRingMetrics::new(1.0);
        let target = witness.inset(metrics.reserved_band());
        assert_eq!(
            metrics.bounds(target),
            witness,
            "required witness is the full AA coverage domain"
        );
        assert!(scene.primitives().iter().any(|p| matches!(p, tekes_ui_scene::Primitive::Quad(q)
            if q.rect == witness.inset(metrics.antialias_guard) && q.border_width == metrics.width)),
            "the original ring's actual quad must use this same coverage witness");
    }
    #[test]
    fn cpu_focus_disable_scale_and_resize_retire_evidence_without_transform_changes() {
        let mut host = observed_focus_host();
        let frame = Rect::new(0.0, 0.0, 480.0, 720.0);
        focus_scene(&mut host, frame);
        let original = focus_space(&host);
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        let transform = viewer.transform();
        viewer.set_disabled(true);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Insufficient
        );
        viewer.set_disabled(false);
        assert_ne!(viewer.required_focus_space(), original);
        let restored = viewer.required_focus_space();
        focus_scene_with_scale(&mut host, frame, 2.0);
        let scaled = focus_space(&host);
        assert_ne!(scaled, restored);
        assert_eq!(
            focus_space(&host),
            scaled,
            "repeated reads retain the same arranged Arc"
        );
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        assert_eq!(
            viewer.transform(),
            transform,
            "backing scale never changes logical image geometry"
        );
        viewer.arrange(&mut ScaledText(f32::NAN), viewer.viewport, frame);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured,
            "invalid renderer scale cannot use fallback authority"
        );
        focus_scene_with_scale(&mut host, frame, 2.0);
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        viewer.paint(
            &mut Scene::new(),
            &mut ScaledText(1.0),
            frame,
            frame,
            tekes_ui_tokens::Palette::from(Theme::LIGHT).into(),
        );
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured,
            "paint scale mismatch revokes arranged coverage"
        );
        focus_scene(&mut host, Rect::new(0.0, 0.0, 500.0, 740.0));
        assert_ne!(focus_space(&host), scaled);
        assert!(matches!(
            focus_space(&host),
            RequiredFocusSpace::Sufficient { .. }
        ));
    }
    #[test]
    fn cpu_focus_clip_context_and_unmount_revoke_then_remeasure_evidence() {
        let mut host = observed_focus_host();
        let frame = Rect::new(0.0, 0.0, 480.0, 720.0);
        focus_scene(&mut host, frame);
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        let old = viewer.required_focus_space();
        let context = viewer.focus_context.clone().unwrap();
        viewer.set_clip_region(None);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured
        );
        viewer.set_curve_geometry_context(Some(context));
        assert_ne!(viewer.required_focus_space(), old);
        viewer.set_curve_geometry_context(None);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured
        );
        viewer.set_disabled(false);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured,
            "availability cannot revive revoked layout evidence"
        );
        focus_scene(&mut host, frame);
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        assert!(matches!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Sufficient { .. }
        ));
        viewer.lifecycle(Lifecycle::Unmounted);
        assert_eq!(
            viewer.required_focus_space(),
            RequiredFocusSpace::Unmeasured
        );
    }
    #[test]
    fn cpu_focus_too_small_viewport_has_no_sufficient_or_painted_boundary() {
        let mut host = observed_focus_host();
        focus_scene(&mut host, Rect::new(0.0, 0.0, 6.0, 720.0));
        assert_eq!(focus_space(&host), RequiredFocusSpace::Insufficient);
        let viewer = host
            .get_mut(FOCUS_VIEW)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap();
        viewer.set_focus(true, true);
        assert!(!viewer.focus_boundary_visible());
        let mut scene = Scene::new();
        viewer.paint(
            &mut scene,
            &mut Text,
            Rect::default(),
            Rect::default(),
            tekes_ui_tokens::Palette::from(Theme::LIGHT).into(),
        );
        assert!(
            !scene
                .primitives()
                .iter()
                .any(|p| matches!(p, tekes_ui_scene::Primitive::Quad(_)))
        );
    }

    // Run only in an allocated GPU window: these use real renderer leases.
    #[test]
    #[ignore = "requires an allocated GPU window"]
    fn gpu_ready_replacement_failure_and_cancellation_keep_one_owner() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let lease = renderer
            .register_bitmap_image(&[255, 0, 0, 255].repeat(4), 2, 2)
            .unwrap();
        let mut viewer = loading();
        let request = viewer.begin_load().unwrap();
        viewer
            .complete(
                request,
                ImageLoadOutcome::Ready {
                    lease: lease.clone(),
                    logical_size: Size::new(200.0, 100.0),
                },
            )
            .unwrap();
        let frame = Rect::new(10.0, 20.0, 100.0, 100.0);
        layout(&mut viewer, frame, frame);
        viewer.drain_transform_changes();
        assert!(
            viewer
                .execute(MediaViewerCommand::ZoomAt {
                    relative_scale: 4.0,
                    anchor: Point::new(50.0, 50.0)
                })
                .unwrap()
        );
        assert_eq!(viewer.drain_transform_changes().len(), 1);
        assert!(
            viewer
                .event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)))
                .capture_pointer
        );
        assert!(
            viewer
                .event(&ContentEvent::PointerMove(Point::new(70.0, 80.0)))
                .changed
        );
        let moved = viewer.transform();
        viewer.event(&ContentEvent::Cancel);
        viewer.event(&ContentEvent::PointerMove(Point::new(90.0, 90.0)));
        viewer.event(&ContentEvent::PointerUp(Point::new(90.0, 90.0)));
        assert_eq!(viewer.transform(), moved);
        let request = viewer.begin_load().unwrap();
        assert!(!viewer.execute(MediaViewerCommand::ZoomIn).unwrap());
        viewer
            .complete(request, ImageLoadOutcome::Failed("keep image".into()))
            .unwrap();
        assert!(viewer.can_execute(MediaViewerCommand::ZoomIn));
        assert_eq!(viewer.transform(), moved);
        let request = viewer.begin_load().unwrap();
        viewer
            .complete(
                request,
                ImageLoadOutcome::Ready {
                    lease,
                    logical_size: Size::new(200.0, 100.0),
                },
            )
            .unwrap();
        assert_eq!(viewer.transform().unwrap().mode, MediaViewerMode::Fit);
        assert_eq!(viewer.transform().unwrap().relative_scale, 1.0);
        // Rendering still goes through the original ImageContent child.
        let mut host = ContentHost::new(Content::new(ComponentId(7201), viewer)).unwrap();
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }
    #[test]
    #[ignore = "requires an allocated GPU window"]
    fn gpu_original_host_releases_cancelled_capture_and_routes_arrow_keys() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let lease = renderer
            .register_bitmap_image(&[255, 0, 0, 255].repeat(4), 2, 2)
            .unwrap();
        let viewer = MediaViewerContent::new(
            ComponentId(7302),
            ImageContent::ready(
                lease.clone(),
                Size::new(200.0, 100.0),
                ImageSemantics::Decorative,
            )
            .unwrap(),
            "Viewer",
        )
        .unwrap();
        let id = ComponentId(7301);
        let mut host = ContentHost::new(Content::new(id, viewer)).unwrap();
        let mut frame = Rect::new(10.0, 20.0, 100.0, 100.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .execute(MediaViewerCommand::ZoomAt {
                relative_scale: 4.0,
                anchor: Point::new(50.0, 50.0),
            })
            .unwrap();
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        assert!(host.pointer_capture().is_some());
        let before = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        host.event(&ContentEvent::Key {
            key: ControlKey::Right,
            down: true,
        });
        let after = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        assert_eq!(
            after.unwrap().translation.x,
            before.unwrap().translation.x - 32.0
        );
        assert!(host.pointer_capture().is_none());
        // A new original gesture, then resize: stale move/up cannot pan the new allocation.
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        assert!(host.pointer_capture().is_some());
        frame = Rect::new(20.0, 30.0, 120.0, 100.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(host.pointer_capture().is_none());
        let resized = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        host.event(&ContentEvent::PointerMove(Point::new(90.0, 90.0)));
        host.event(&ContentEvent::PointerUp(Point::new(90.0, 90.0)));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            resized
        );
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        assert!(host.pointer_capture().is_some());
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .begin_load()
            .unwrap();
        assert!(host.pointer_capture().is_none());
        host.event(&ContentEvent::PointerMove(Point::new(95.0, 95.0)));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            resized
        );
        let request = host
            .get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .begin_load()
            .unwrap();
        assert!(
            host.get_mut(id)
                .unwrap()
                .component_mut::<MediaViewerContent>()
                .unwrap()
                .complete(
                    request,
                    ImageLoadOutcome::Ready {
                        lease,
                        logical_size: Size::new(200.0, 100.0)
                    }
                )
                .unwrap()
        );
        let replaced = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        assert_eq!(replaced.unwrap().mode, MediaViewerMode::Fit);
        assert!(host.pointer_capture_lease().is_none());
        host.event(&ContentEvent::PointerMove(Point::new(90.0, 90.0)));
        host.event(&ContentEvent::PointerUp(Point::new(90.0, 90.0)));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            replaced
        );
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .execute(MediaViewerCommand::ZoomAt {
                relative_scale: 4.0,
                anchor: Point::new(50.0, 50.0),
            })
            .unwrap();
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        for escape in [false, true] {
            host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
            assert!(host.pointer_capture_lease().is_some());
            let before = host
                .get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform();
            if escape {
                host.event(&ContentEvent::Key {
                    key: ControlKey::Escape,
                    down: true,
                });
            } else {
                assert!(host.focus(None, false));
            }
            assert!(host.pointer_capture_lease().is_none());
            host.event(&ContentEvent::PointerMove(Point::new(95.0, 95.0)));
            host.event(&ContentEvent::PointerUp(Point::new(95.0, 95.0)));
            assert_eq!(
                host.get(id)
                    .unwrap()
                    .component::<MediaViewerContent>()
                    .unwrap()
                    .transform(),
                before
            );
        }
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        assert!(host.pointer_capture_lease().is_some());
    }
    #[test]
    #[ignore = "requires an allocated real Renderer/GPU window"]
    fn gpu_active_drag_ready_replay_and_replacement_follow_original_load_contract() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let first = renderer
            .register_bitmap_image(&[255, 0, 0, 255].repeat(4), 2, 2)
            .unwrap();
        let replacement = renderer
            .register_bitmap_image(&[0, 255, 0, 255].repeat(4), 2, 2)
            .unwrap();
        let mut content = loading();
        let consumed = content.begin_load().unwrap();
        assert!(
            content
                .complete(
                    consumed.clone(),
                    ImageLoadOutcome::Ready {
                        lease: first,
                        logical_size: Size::new(200.0, 100.0),
                    }
                )
                .unwrap()
        );
        let id = ComponentId(7501);
        let mut host = ContentHost::new(Content::new(id, content)).unwrap();
        let frame = Rect::new(10.0, 20.0, 100.0, 100.0);
        // get_mut conservatively invalidates layout, even for a queue drain or
        // rejected completion. Publish those owner updates through the normal
        // same-frame render before testing input against the current layout.
        let acknowledge_layout = |host: &mut ContentHost| {
            host.render(
                &mut Scene::new(),
                &mut Text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        };
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .execute(MediaViewerCommand::ZoomAt {
                relative_scale: 4.0,
                anchor: Point::new(50.0, 50.0),
            })
            .unwrap();
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .drain_transform_changes();
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let start = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        let original_capture = host
            .pointer_capture_lease()
            .expect("actual Ready Down owns capture");
        assert!(
            host.event(&ContentEvent::PointerMove(Point::new(70.0, 75.0)))
                .changed
        );
        let active_transform = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        assert_ne!(active_transform, start);
        assert_eq!(host.pointer_capture_lease(), Some(original_capture));
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .drain_transform_changes();
        acknowledge_layout(&mut host);

        // A Ready result can arrive during an active gesture, but a consumed
        // request is rejected by the original ImageContent before Viewer effects.
        assert!(
            !host
                .get_mut(id)
                .unwrap()
                .component_mut::<MediaViewerContent>()
                .unwrap()
                .complete(
                    consumed,
                    ImageLoadOutcome::Ready {
                        lease: replacement.clone(),
                        logical_size: Size::new(200.0, 100.0),
                    }
                )
                .unwrap()
        );
        acknowledge_layout(&mut host);
        assert_eq!(host.pointer_capture_lease(), Some(original_capture));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            active_transform
        );
        assert!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .changes
                .is_empty()
        );
        assert!(
            host.event(&ContentEvent::PointerMove(Point::new(75.0, 80.0)))
                .changed
        );
        let last_dragged = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        assert_ne!(last_dragged, active_transform);
        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .drain_transform_changes();
        acknowledge_layout(&mut host);

        // The supported source transaction revokes the active gesture at
        // begin_load. Loading rejects Down; Ready must not revive old capture.
        let request = host
            .get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .begin_load()
            .unwrap();
        acknowledge_layout(&mut host);
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .image_status(),
            Some(ImageLoadStatus::Loading)
        );
        assert!(host.pointer_capture_lease().is_none());
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            last_dragged
        );
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        assert!(host.pointer_capture_lease().is_none());
        assert!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .changes
                .is_empty()
        );
        assert!(
            host.get_mut(id)
                .unwrap()
                .component_mut::<MediaViewerContent>()
                .unwrap()
                .complete(
                    request,
                    ImageLoadOutcome::Ready {
                        lease: replacement,
                        logical_size: Size::new(200.0, 100.0),
                    }
                )
                .unwrap()
        );
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .image_status(),
            Some(ImageLoadStatus::Ready)
        );
        let replaced = host
            .get(id)
            .unwrap()
            .component::<MediaViewerContent>()
            .unwrap()
            .transform();
        assert_eq!(replaced.unwrap().mode, MediaViewerMode::Fit);
        assert_eq!(replaced.unwrap().relative_scale, 1.0);
        assert_eq!(
            host.get_mut(id)
                .unwrap()
                .component_mut::<MediaViewerContent>()
                .unwrap()
                .drain_transform_changes()
                .len(),
            1
        );
        acknowledge_layout(&mut host);
        assert!(host.pointer_capture_lease().is_none());
        host.event(&ContentEvent::PointerMove(Point::new(90.0, 90.0)));
        host.event(&ContentEvent::PointerUp(Point::new(90.0, 90.0)));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            replaced
        );
        assert!(
            host.get(id)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .changes
                .is_empty()
        );

        host.get_mut(id)
            .unwrap()
            .component_mut::<MediaViewerContent>()
            .unwrap()
            .execute(MediaViewerCommand::ZoomAt {
                relative_scale: 4.0,
                anchor: Point::new(50.0, 50.0),
            })
            .unwrap();
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        host.event(&ContentEvent::PointerDown(Point::new(60.0, 70.0)));
        let fresh_capture = host
            .pointer_capture_lease()
            .expect("new Ready Down obtains new capture");
        assert_ne!(fresh_capture.2, original_capture.2);
        assert!(
            host.event(&ContentEvent::PointerMove(Point::new(65.0, 75.0)))
                .changed
        );
        host.event(&ContentEvent::PointerUp(Point::new(65.0, 75.0)));
        assert!(host.pointer_capture_lease().is_none());
    }
    #[test]
    #[ignore = "requires allocated real Renderer/GPU window; focused pixels need separate image review"]
    fn gpu_focus_boundary_paints_above_one_original_image_at_four_themes_two_scales() {
        let output = std::path::PathBuf::from(
            std::env::var_os("MEDIA_VIEWER_FOCUS_OUTPUT")
                .expect("set an allocated evidence directory for new focused images"),
        );
        std::fs::create_dir_all(&output).unwrap();
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let lease = renderer
            .register_bitmap_image(
                &[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                ],
                2,
                2,
            )
            .unwrap();
        let frame = Rect::new(0.0, 0.0, 480.0, 720.0);
        for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
            for scale in [1.0, 2.0] {
                let mut host = observed_focus_host();
                let viewer = host
                    .get_mut(FOCUS_VIEW)
                    .unwrap()
                    .component_mut::<MediaViewerContent>()
                    .unwrap();
                let request = viewer.begin_load().unwrap();
                viewer
                    .complete(
                        request,
                        ImageLoadOutcome::Ready {
                            lease: lease.clone(),
                            logical_size: Size::new(200.0, 100.0),
                        },
                    )
                    .unwrap();
                let mut scene = Scene::new();
                host.render(
                    &mut scene,
                    &mut ScaledText(scale),
                    frame,
                    frame,
                    theme.into(),
                )
                .unwrap();
                host.get_mut(FOCUS_VIEW)
                    .unwrap()
                    .component_mut::<MediaViewerContent>()
                    .unwrap()
                    .execute(MediaViewerCommand::ZoomAt {
                        relative_scale: 4.0,
                        anchor: Point::new(240.0, 266.0),
                    })
                    .unwrap();
                scene.clear();
                host.render(
                    &mut scene,
                    &mut ScaledText(scale),
                    frame,
                    frame,
                    theme.into(),
                )
                .unwrap();
                let size = ((480.0 * scale) as u32, (720.0 * scale) as u32);
                let before = renderer
                    .render_to_rgba8(
                        &scene,
                        &mut tekes_ui_scene::GlyphAtlas::default(),
                        size,
                        scale,
                        Rgba::gray(0.0),
                    )
                    .unwrap();
                let tab = ContentEvent::Key {
                    key: ControlKey::Tab { reverse: false },
                    down: true,
                };
                host.event(&tab);
                assert_eq!(host.focused(), Some(FOCUS_RETRY));
                host.event(&tab);
                assert_eq!(host.focused(), Some(FOCUS_VIEW));
                scene.clear();
                host.render(
                    &mut scene,
                    &mut ScaledText(scale),
                    frame,
                    frame,
                    theme.into(),
                )
                .unwrap();
                let RequiredFocusSpace::Sufficient { witness, .. } = focus_space(&host) else {
                    panic!("actual Ready focus has a coverage witness");
                };
                let metrics = crate::extended::FocusRingMetrics::new(scale);
                let images: Vec<_> = scene
                    .primitives()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        matches!(
                            p,
                            tekes_ui_scene::Primitive::ControlledTransformedImage { .. }
                        )
                        .then_some(i)
                    })
                    .collect();
                let ring = scene.primitives().iter().position(|p| matches!(p, tekes_ui_scene::Primitive::Quad(q)
                    if q.rect == witness.inset(metrics.antialias_guard) && q.border_width == metrics.width));
                let after = renderer
                    .render_to_rgba8(
                        &scene,
                        &mut tekes_ui_scene::GlyphAtlas::default(),
                        size,
                        scale,
                        Rgba::gray(0.0),
                    )
                    .unwrap();
                let prefix = format!("theme-{theme_index}-scale-{scale}");
                let rect = |r: Rect| [r.min_x(), r.min_y(), r.size.width, r.size.height];
                let metadata = serde_json::to_vec_pretty(&serde_json::json!({
                    "theme_index": theme_index,
                    "theme": format!("{theme:?}"),
                    "backing_scale": scale,
                    "width_px": size.0, "height_px": size.1,
                    "logical_frame": rect(frame),
                    "viewport": rect(host.get(FOCUS_VIEW).unwrap().frame()),
                    "viewer_transform": format!("{:?}", host.get(FOCUS_VIEW).unwrap().component::<MediaViewerContent>().unwrap().transform()),
                    "keyboard_focus": format!("{:?}", host.focused()),
                    "focus_coverage_witness": rect(witness),
                    "focus_draw_target": rect(witness.inset(metrics.reserved_band())),
                    "focus_quad_bounds": rect(witness.inset(metrics.antialias_guard)),
                    "focus_width": metrics.width, "focus_gap": metrics.gap,
                    "focus_aa_guard": metrics.antialias_guard,
                    "controlled_image_count": images.len(),
                    "image_primitive_index": images.first().copied(), "ring_primitive_index": ring,
                    "unfocused_rgba": format!("{prefix}-unfocused.rgba"),
                    "focused_rgba": format!("{prefix}-focused.rgba"),
                    "format": "RGBA8, row-major; preserve originals and convert with existing image-evidence tools",
                    "visual_review": "NOT_REVIEWED; actual rendered files are not visual acceptance"
                })).unwrap();
                for (name, bytes) in [
                    (format!("{prefix}-unfocused.rgba"), before.as_slice()),
                    (format!("{prefix}-focused.rgba"), after.as_slice()),
                    (format!("{prefix}.json"), metadata.as_slice()),
                ] {
                    // Evidence is append-only. A rerun needs a new allocated
                    // directory rather than replacing the earlier failure.
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(output.join(name))
                        .unwrap();
                    std::io::Write::write_all(&mut file, bytes).unwrap();
                }
                assert_eq!(
                    images.len(),
                    1,
                    "original child image is painted exactly once"
                );
                assert!(
                    ring.expect("same witness drives the focused quad") > images[0],
                    "focus ring must not be covered by the bitmap"
                );
                assert_ne!(before, after, "actual focused rendering must change pixels");
            }
        }
    }
}
