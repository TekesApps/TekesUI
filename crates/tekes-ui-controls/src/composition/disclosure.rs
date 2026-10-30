//! Controlled composition over the original Disclosure model/runtime/reveal.
use super::*;
use crate::{extended::Disclosure, layout_view::DisclosureView};
use std::sync::atomic::{AtomicU64, Ordering};
use tekes_ui_scene::ImageQuad;

static NEXT_INTENT: AtomicU64 = AtomicU64::new(1);
/// Owned event-time request. Unique generations distinguish callbacks even
/// across replacement mounts. Only the latest request at its confirmed revision
/// can be settled; this is unrelated to consumer persistence/session identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureIntent {
    expanded: bool,
    generation: u64,
    revision: u64,
}
impl DisclosureIntent {
    pub fn expanded(&self) -> bool {
        self.expanded
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Debug)]
struct Trigger {
    model: Disclosure,
    view: DisclosureView,
    title: Vec<Content>,
    name: String,
    rtl: bool,
    size: Size,
    pending: Option<DisclosureIntent>,
    last_intent: Option<DisclosureIntent>,
    revision: u64,
    image: Option<IconImage>,
    icon_error: Option<IconRenderError>,
    window_active: bool,
}
impl Trigger {
    fn request(&mut self, activated: bool) -> ContentResponse {
        self.last_intent = activated.then(|| DisclosureIntent {
            expanded: !self.model.open,
            generation: NEXT_INTENT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("Disclosure intent generation exhausted"),
            revision: self.revision,
        });
        if activated {
            self.pending = self.last_intent.clone();
        }
        ContentResponse {
            consumed: activated,
            changed: activated,
            activated,
            ..Default::default()
        }
    }
    fn revoke(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("Disclosure revision exhausted");
        self.pending = None;
        self.last_intent = None;
    }
    fn title_measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> Size {
        let (inset, side, row) = DisclosureView::content_insets(&mut TextAdapter(text));
        let width = (available.width - inset * 3. - side).max(0.);
        self.size = self.title[0]
            .measure_intrinsic(text, Size::new(width, available.height))
            .size;
        Size::new(available.width, row.max(self.size.height + inset))
    }
}
impl Component for Trigger {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.title.len() != 1 {
            return Err(CompositionError::InvalidSlotCount);
        }
        if self.name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        validate_title(&self.title[0])
    }
    fn children(&self) -> &[Content] {
        &self.title
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.title)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn requires_icon_services(&self) -> bool {
        true
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.icon_error
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.icon_error = None;
        MeasuredChild::new(self.title_measure(text, available))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.title_measure(text, frame.size);
        // Actual title frame is resolved by the original row during paint.
        self.title[0].arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.icon_error = None;
        self.view
            .set_surface(theme.surface())
            .expect("validated Disclosure Surface");
        let layout = self.view.render_content(
            scene,
            &mut TextAdapter(text),
            &self.model,
            self.size,
            frame,
            clip,
            theme.palette,
            self.rtl,
        );
        let (ink, icon_ink) = self.view.content_inks(theme.palette, theme.surface());
        self.title[0].arrange(
            text,
            layout.content,
            layout.content.intersection(&clip).unwrap_or_default(),
        );
        self.title[0].paint(
            scene,
            text,
            ContentTheme {
                foreground: Some(ink),
                icon_foreground: Some(icon_ink),
                ..theme
            },
        );
        let backing = text.backing_scale();
        let Some(service) = text.icon_renderer() else {
            self.icon_error = Some(IconRenderError::UnsupportedRenderer);
            self.image = None;
            return;
        };
        struct Prepared<'a> {
            service: &'a mut dyn crate::layout_view::IconRenderer,
            image: &'a mut Option<IconImage>,
            error: &'a mut Option<IconRenderError>,
            backing: f32,
        }
        impl crate::layout_view::IconRenderer for Prepared<'_> {
            fn draw_icon(&mut self, scene: &mut Scene, name: &str, frame: Rect, tint: Rgba) {
                match self
                    .service
                    .prepare_icon(name, frame.size.width, self.backing, tint)
                {
                    Ok(image) => {
                        scene.push_image(ImageQuad {
                            rect: frame,
                            image: image.id(),
                            opacity: 1.,
                        });
                        *self.image = Some(image);
                    }
                    Err(e) => {
                        *self.image = None;
                        *self.error = Some(e);
                    }
                }
            }
        }
        self.view.render_content_indicator(
            scene,
            &mut Prepared {
                service,
                image: &mut self.image,
                error: &mut self.icon_error,
                backing,
            },
            &self.model,
            self.rtl,
            clip,
            (theme.palette, icon_ink),
        );
    }
    fn window_active_changed(&mut self, active: bool) {
        self.window_active = active;
        self.view.set_window_active(active);
        if !active {
            self.revoke();
        }
    }
    fn focus_decoration_above_siblings(&self) -> bool {
        self.view.focus_decoration_above_siblings(&self.model)
    }
    fn focusable(&self) -> bool {
        !self.model.disabled && self.window_active
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            self.view.focus(visible);
        } else {
            self.view.blur();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Button, self.name.clone());
        s.disabled = self.model.disabled;
        s.expanded = Some(self.model.open);
        s
    }
    fn action_payload(&self, kind: ContentActionKind) -> Option<ContentActionPayload> {
        (kind == ContentActionKind::Activate)
            .then(|| self.last_intent.clone())
            .flatten()
            .map(ContentActionPayload::DisclosureExpansion)
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        self.last_intent = None;
        if part.is_some() {
            return ContentResponse::default();
        }
        match action {
            AccessibilityAction::Press => {
                let a = self.view.request_activate(&self.model);
                self.request(a)
            }
            AccessibilityAction::Focus if self.focusable() => {
                self.view.focus(true);
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if key.down && key.repeat && matches!(key.key, Some(ControlKey::Enter | ControlKey::Space))
        {
            self.last_intent = None;
            return ContentResponse::consumed(false);
        }
        key.key
            .map(|key_code| {
                self.event(&ContentEvent::Key {
                    key: key_code,
                    down: key.down,
                })
            })
            .unwrap_or_default()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_intent = None;
        match event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p);
                ContentResponse::consumed(true)
            }
            ContentEvent::PointerDown(p) => {
                let a = self.view.request_pointer_down(*p, &self.model);
                ContentResponse {
                    consumed: a,
                    changed: a,
                    capture_pointer: a,
                    ..Default::default()
                }
            }
            ContentEvent::PointerUp(p) => {
                let a = self.view.request_pointer_up(*p, &self.model);
                self.request(a)
            }
            ContentEvent::Key { key, down } => {
                let e = self.view.request_key_event(*key, *down, &self.model);
                let mut r = self.request(e.changed);
                r.consumed = e.consumed;
                r
            }
            ContentEvent::TextCommand(command)
                if matches!(command.as_str(), "insertNewline:" | "insertLineBreak:") =>
            {
                let e = self
                    .view
                    .request_key_event(ControlKey::Enter, true, &self.model);
                self.request(e.changed)
            }
            ContentEvent::Cancel => {
                let changed = self.view.request_cancel();
                self.revoke();
                ContentResponse::consumed(changed)
            }
            _ => ContentResponse::default(),
        }
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.revoke();
            self.view.cancel();
            self.image = None;
        }
    }
}

/// One passive trigger title and an independently mounted body. The explicit
/// trigger ID is the sole action/focus/AX owner; the container itself is a Group.
/// Expansion is confirmed, never optimistically toggled by input delivery.
#[derive(Debug)]
pub struct DisclosureContent {
    children: Vec<Content>,
    header_height: f32,
    body_height: f32,
    body_layout: crate::layout_view::DisclosureBodyLayout,
}
impl DisclosureContent {
    pub fn new(trigger: ComponentId, title: Content, body: Content, expanded: bool) -> Self {
        fn name(c: &Content) -> String {
            let own = c.component.semantics().name;
            if own.trim().is_empty() {
                c.component
                    .children()
                    .iter()
                    .map(name)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                own
            }
        }
        let name = name(&title);
        let mut this = Self {
            children: vec![
                Content::new(
                    trigger,
                    Trigger {
                        model: Disclosure {
                            open: expanded,
                            disabled: false,
                        },
                        view: DisclosureView::default(),
                        title: vec![title],
                        name,
                        rtl: false,
                        size: Size::default(),
                        pending: None,
                        last_intent: None,
                        revision: 0,
                        image: None,
                        icon_error: None,
                        window_active: true,
                    },
                ),
                body,
            ],
            header_height: 0.,
            body_height: 0.,
            body_layout: Default::default(),
        };
        this.sync_body();
        this
    }
    fn trigger(&self) -> &Trigger {
        self.children[0]
            .component::<Trigger>()
            .expect("fixed Disclosure trigger")
    }
    fn trigger_mut(&mut self) -> &mut Trigger {
        self.children[0]
            .component_mut::<Trigger>()
            .expect("fixed Disclosure trigger")
    }
    fn sync_body(&mut self) {
        let open = self.expanded();
        self.children[1].set_composition_active(open);
    }
    pub fn trigger_id(&self) -> ComponentId {
        self.children[0].id()
    }
    pub fn expanded(&self) -> bool {
        self.trigger().model.open
    }
    pub fn disabled(&self) -> bool {
        self.trigger().model.disabled
    }
    /// A valid rejection settles the request without changing the confirmed
    /// value. Accepting a collapse cannot hide marked text, just like Tabs.
    /// Stale, reordered or already settled requests return false without effect.
    pub fn confirm(&mut self, intent: &DisclosureIntent, accepted: bool) -> bool {
        if self.trigger().pending.as_ref() != Some(intent)
            || self.trigger().revision != intent.revision
        {
            return false;
        }
        if accepted && !intent.expanded && subtree_composing(&self.children[1]) {
            return false;
        }
        let trigger = self.trigger_mut();
        if accepted {
            trigger.model.open = intent.expanded;
        }
        trigger.revoke();
        self.sync_body();
        true
    }
    /// Apply an authoritative external update and retire outstanding intents.
    /// False means the body is composing; it and its owner remain untouched.
    pub fn set_expanded(&mut self, expanded: bool) -> bool {
        if !expanded && subtree_composing(&self.children[1]) {
            return false;
        }
        self.trigger_mut().model.open = expanded;
        self.trigger_mut().revoke();
        self.sync_body();
        true
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        let t = self.trigger_mut();
        if t.model.disabled != disabled {
            t.model.disabled = disabled;
            t.revoke();
            if disabled {
                t.view.cancel();
            }
        }
    }
    pub fn set_rtl(&mut self, rtl: bool) {
        self.trigger_mut().rtl = rtl;
    }
    pub fn set_accessible_name(&mut self, name: impl Into<String>) -> Result<(), CompositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyAccessibleName);
        }
        self.trigger_mut().name = name;
        Ok(())
    }
    pub fn set_window_active(&mut self, active: bool) {
        let t = self.trigger_mut();
        t.window_active = active;
        t.view.set_window_active(active);
        if !active {
            t.revoke();
        }
    }
    /// Original reveal block, including fixed header-focus clearance. The body
    /// Content's frame starts after that inset. Occupied height/clip never exceed
    /// the allocated reveal portion; a fully closed block occupies zero height.
    pub fn body_layout(&self) -> crate::layout_view::DisclosureBodyLayout {
        self.body_layout
    }
    fn focus_clearance(header: Rect, backing: f32) -> f32 {
        (crate::extended::ControlFrame::new(header, None)
            .focus_bounds(backing)
            .max_y()
            - header.max_y())
        .max(0.)
    }
    fn reveal(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        block: Rect,
        clip: Rect,
        allocated: f32,
    ) {
        let allocated = allocated.max(0.);
        let viewport = Rect::new(block.min_x(), block.min_y(), block.size.width, allocated);
        let clip = viewport.intersection(&clip).unwrap_or_default();
        let t = self.trigger_mut();
        let model = t.model;
        self.body_layout = t
            .view
            .layout_body(&mut TextAdapter(text), &model, block, Some(clip));
        self.body_layout.occupied_height = self.body_layout.occupied_height.min(allocated);
        self.sync_body();
        let retained = !self.expanded() && self.body_layout.visible_clip.is_some();
        self.children[1].retain_body_paint(retained);
    }
}
impl Component for DisclosureContent {
    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() == 2 && self.children[0].component::<Trigger>().is_some() {
            Ok(())
        } else {
            Err(CompositionError::InvalidSlotCount)
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.header_height = self.children[0].measure(text, available).size.height;
        let header = Rect::new(0., 0., available.width, self.header_height);
        let clearance = Self::focus_clearance(header, text.backing_scale());
        let allocated = (available.height - self.header_height).max(0.);
        self.body_height = self.children[1]
            .measure(
                text,
                Size::new(available.width, (allocated - clearance).max(0.)),
            )
            .size
            .height;
        let block_height = if self.body_height > 0. {
            clearance + self.body_height
        } else {
            0.
        };
        self.reveal(
            text,
            Rect::new(0., self.header_height, available.width, block_height),
            Rect::new(0., 0., available.width, available.height),
            allocated,
        );
        MeasuredChild::new(Size::new(
            available.width,
            self.header_height + self.body_layout.occupied_height,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let header = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width,
            self.header_height.min(frame.size.height),
        );
        self.children[0].arrange(text, header, clip);
        let clearance = if self.body_height > 0. {
            Self::focus_clearance(header, text.backing_scale())
        } else {
            0.
        };
        let block = Rect::new(
            frame.min_x(),
            header.max_y(),
            frame.size.width,
            clearance + self.body_height,
        );
        self.reveal(text, block, clip, (frame.max_y() - header.max_y()).max(0.));
        let child = Rect::new(
            block.min_x(),
            block.min_y() + clearance,
            block.size.width,
            self.body_height,
        );
        // The clip remains the actual allocated ancestor reveal viewport. The
        // child's own frame controls hit/AX geometry, while its original focus
        // ring may use the already allocated clearance above that frame.
        let child_clip = self.body_layout.visible_clip.unwrap_or_default();
        self.children[1].arrange(text, child, child_clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        // Same original Core predicate/order, including focused descendants in
        // the body. This custom container never swaps slots or replays paint.
        for raised in [false, true] {
            for child in self
                .children
                .iter_mut()
                .filter(|child| child.has_raised_focus_decoration() == raised)
            {
                child.paint(scene, text, theme);
            }
        }
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        // Composition-inactive closing bodies never traverse this overlay path.
        self.children[1].paint_overlays(scene, text, theme);
    }
    fn overlay_presented(&self) -> bool {
        self.children[1].active()
    }
}

fn subtree_composing(content: &Content) -> bool {
    content.component.composing()
        || content
            .component
            .text_surface()
            .is_some_and(|s| s.input.marked.is_some())
        || content.component.children().iter().any(subtree_composing)
}
fn validate_title(content: &Content) -> Result<(), CompositionError> {
    if let Some(child) = content.interactive_id() {
        return Err(CompositionError::NestedInteractive {
            action: content.id(),
            child,
        });
    }
    fn owns_overlay(content: &Content) -> bool {
        content.component.overlay_scope().is_some()
            || content.component.overlay_open()
            || content.component.overlay_presented()
            || content.component.children().iter().any(owns_overlay)
    }
    if owns_overlay(content) {
        Err(CompositionError::InvalidSlotCount)
    } else {
        Ok(())
    }
}
