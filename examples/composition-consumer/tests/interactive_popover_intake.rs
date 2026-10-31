//! Issue49 intake: original popup Content must win over a later document sibling.
//! No second host, manual input forwarding, copied focus engine or GPU needed.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::Popover,
    feedback_view::{OverlayEvent, PopoverView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(width), 18.)
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
#[derive(Debug)]
struct Popup {
    model: Popover,
    view: PopoverView,
    children: Vec<Content>,
    painted: bool,
}
impl Component for Popup {
    // The shared scope declaration itself retires closed input and AX; no
    // consumer copy of an active/open predicate is needed.
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn overlay_scope(&self) -> Option<OverlayScope> {
        Some(OverlayScope::default())
    }
    fn overlay_presented(&self) -> bool {
        self.model.open || self.view.is_presented()
    }
    fn overlay_open(&self) -> bool {
        self.model.open
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, mut text: &mut dyn ButtonTextRenderer, anchor: Rect, clip: Rect) {
        let layout = self.view.arrange(
            &mut text,
            &self.model,
            anchor,
            Size::new(360., 220.),
            clip,
            Some(clip),
            proposal::TextRole::UiBody,
        );
        self.children[0].arrange(text, layout.content, clip);
    }
    fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        anchor: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let children = &mut self.children;
        let painted = &mut self.painted;
        self.view.render_with_text_content(
            scene,
            &mut text,
            &self.model,
            anchor,
            Size::new(360., 220.),
            clip,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            "Original popup child",
            |scene, text, _| {
                children[0].paint(scene, text, theme);
                *painted = true;
            },
        );
    }
    fn overlay_event(&mut self, e: &ContentEvent) -> ContentResponse {
        match e {
            ContentEvent::PointerDown(p) if self.view.pointer_down(&mut self.model, *p) => {
                ContentResponse::consumed(false)
            }
            ContentEvent::PointerUp(p)
                if matches!(
                    self.view.pointer_up(&mut self.model, *p),
                    OverlayEvent::Dismissed { .. }
                ) =>
            {
                ContentResponse::consumed(true)
            }
            ContentEvent::Key { key, down }
                if matches!(
                    self.view.key_event(&mut self.model, *key, *down),
                    OverlayEvent::Dismissed { .. }
                ) =>
            {
                ContentResponse::consumed(true)
            }
            _ => ContentResponse::default(),
        }
    }
}
#[derive(Debug)]
struct Overlap {
    children: Vec<Content>,
}
impl Component for Overlap {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        self.children[0].arrange(text, Rect::new(20., 20., 80., 30.), clip);
        let content = self.children[0]
            .component::<Popup>()
            .unwrap()
            .view
            .layout()
            .content;
        for child in &mut self.children[1..] {
            child.arrange(text, content, clip);
        }
    }
}
fn fixture(open: bool) -> ContentHost {
    let popup = ComponentId(51001);
    let child = ComponentId(51002);
    let background = ComponentId(51003);
    let mut model = Popover::default();
    if open {
        model.show(1);
    }
    let mut host = ContentHost::new(Content::new(
        ComponentId(51000),
        Overlap {
            children: vec![
                Content::new(
                    popup,
                    Popup {
                        model,
                        view: PopoverView::default(),
                        children: vec![Content::new(child, Button::new("Popup Save"))],
                        painted: false,
                    },
                ),
                Content::new(background, Button::new("Background Action")),
            ],
        },
    ))
    .unwrap();
    host.set_action_events(true);
    let frame = Rect::new(0., 0., 600., 500.);
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

#[test]
fn painted_popup_original_child_must_win_over_later_background_sibling() {
    let popup = ComponentId(51001);
    let child = ComponentId(51002);
    let background = ComponentId(51003);
    let mut host = fixture(true);
    let point = host.get(child).unwrap().visible().unwrap().center();
    assert!(
        host.get(popup)
            .unwrap()
            .component::<Popup>()
            .unwrap()
            .painted
    );
    assert!(
        host.get(background)
            .unwrap()
            .visible()
            .unwrap()
            .contains(point)
    );
    host.event(&ContentEvent::PointerDown(point));
    host.event(&ContentEvent::PointerUp(point));
    let targets = host
        .drain_actions()
        .into_iter()
        .map(|a| a.target)
        .collect::<Vec<_>>();
    assert_eq!(
        targets,
        vec![child],
        "original overlay action must not fall through to underlying document"
    );
}

#[test]
fn tab_must_remain_in_the_visible_popup_original_subtree() {
    let mut host = fixture(true);
    for _ in 0..2 {
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Tab { reverse: false },
            down: true,
        });
        assert_eq!(
            host.focused(),
            Some(ComponentId(51002)),
            "background sibling must not enter popup traversal"
        );
    }
}
#[test]
fn background_ax_press_cannot_activate_through_an_open_popup() {
    let mut host = fixture(true);
    let response = host.accessibility_action(ComponentId(51003), None, AccessibilityAction::Press);
    assert!(
        !response.consumed,
        "background AX action must be rejected while the transient scope is active"
    );
    assert!(host.drain_actions().is_empty());
}
#[test]
fn capture_acquired_before_popup_open_keeps_the_original_owner() {
    let mut host = fixture(false);
    let target = host
        .get(ComponentId(51003))
        .unwrap()
        .visible()
        .unwrap()
        .center();
    let down = host.event(&ContentEvent::PointerDown(target));
    assert!(down.capture_pointer);
    host.get_mut(ComponentId(51001))
        .unwrap()
        .component_mut::<Popup>()
        .unwrap()
        .model
        .show(1);
    let frame = Rect::new(0., 0., 600., 500.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    host.event(&ContentEvent::PointerUp(target));
    assert_eq!(
        host.drain_actions()
            .into_iter()
            .map(|a| a.target)
            .collect::<Vec<_>>(),
        vec![ComponentId(51003)],
        "new overlay must not steal capture acquired before it opened"
    );
}

fn render(host: &mut ContentHost) {
    let frame = Rect::new(0., 0., 600., 500.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn set_open(host: &mut ContentHost, open: bool) {
    let popup = host
        .get_mut(ComponentId(51001))
        .unwrap()
        .component_mut::<Popup>()
        .unwrap();
    if open {
        popup.model.show(1);
    } else {
        popup.model.close();
    }
}
#[test]
fn declared_scope_wraps_even_when_outer_host_yields_at_tab_boundary() {
    let mut host = fixture(true);
    host.set_focus_wrap(false);
    for reverse in [false, true, false] {
        assert!(
            host.event(&ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Tab { reverse },
                down: true
            })
            .consumed
        );
        assert!(!host.event_bubbled());
        assert_eq!(host.focused(), Some(ComponentId(51002)));
    }
}
#[test]
fn close_restores_original_mount_and_immediately_retires_popup_ax() {
    let mut host = fixture(false);
    host.focus(Some(ComponentId(51003)), true);
    let original_mount = host.semantic_mount(ComponentId(51003)).unwrap();
    set_open(&mut host, true);
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(51002)));
    assert_eq!(
        host.semantics().iter().map(|n| n.id).collect::<Vec<_>>(),
        vec![ComponentId(51001), ComponentId(51002)]
    );
    set_open(&mut host, false);
    assert!(
        !host
            .accessibility_action(ComponentId(51002), None, AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(host.focused(), Some(ComponentId(51003)));
    assert_eq!(
        host.semantic_mount(ComponentId(51003)),
        Some(original_mount)
    );
    let restored_epoch = host.focus_epoch();
    assert!(
        !host
            .accessibility_action(ComponentId(51002), None, AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(host.focused(), Some(ComponentId(51003)));
    assert_eq!(host.focus_epoch(), restored_epoch);
    for id in [ComponentId(51002), ComponentId(51003)] {
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            0
        );
    }
    assert!(host.drain_actions().is_empty());
}
#[test]
fn same_numeric_invoker_replacement_cannot_inherit_return_focus() {
    let mut host = fixture(false);
    host.focus(Some(ComponentId(51003)), true);
    let old_mount = host.semantic_mount(ComponentId(51003)).unwrap();
    set_open(&mut host, true);
    render(&mut host);
    host.remove(ComponentId(51003)).unwrap();
    host.insert(
        ComponentId(51000),
        Content::new(ComponentId(51003), Button::new("Replacement")),
    )
    .unwrap();
    set_open(&mut host, false);
    render(&mut host);
    assert_ne!(host.semantic_mount(ComponentId(51003)), Some(old_mount));
    assert_eq!(host.focused(), None);
}
#[test]
fn unhandled_inside_or_outside_input_cannot_bubble_through_scope() {
    let mut host = fixture(true);
    let response = host.event(&ContentEvent::Scroll {
        position: tekes_ui_scene::Point::new(599., 499.),
        delta: tekes_ui_scene::Point::new(0., 10.),
        precise: true,
        begin: true,
        end: true,
        momentum: false,
        time: 1.,
    });
    assert!(response.consumed);
    assert!(!host.event_bubbled());
    assert!(host.drain_actions().is_empty());
}
#[test]
fn outside_click_dismisses_once_without_activating_background() {
    let mut host = fixture(true);
    let point = tekes_ui_scene::Point::new(599., 499.);
    assert!(host.event(&ContentEvent::PointerDown(point)).consumed);
    assert!(host.event(&ContentEvent::PointerUp(point)).consumed);
    assert!(
        !host
            .get(ComponentId(51001))
            .unwrap()
            .component::<Popup>()
            .unwrap()
            .model
            .open
    );
    assert!(host.drain_actions().is_empty());
}

#[derive(Debug)]
struct Scope {
    open: bool,
    disabled: bool,
    children: Vec<Content>,
}
impl Component for Scope {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn overlay_open(&self) -> bool {
        self.open
    }
    fn overlay_scope(&self) -> Option<OverlayScope> {
        Some(OverlayScope::default())
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::Group, "Scoped content");
        s.disabled = self.disabled;
        s
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            child.arrange(text, frame, clip);
        }
    }
}
// A test owner with two original Buttons records which existing handler
// receives release. Popover remains the original open/close model.
#[derive(Debug)]
struct HandlerCapture {
    body: Button,
    overlay: Button,
    popup: Popover,
    body_ups: usize,
    overlay_ups: usize,
}
impl Component for HandlerCapture {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.body.arrange(text, frame, clip);
        self.overlay.arrange(text, frame, clip);
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.body.paint(scene, text, frame, clip, theme);
        if self.popup.open {
            self.overlay.paint(scene, text, frame, clip, theme);
        }
    }
    fn overlay_open(&self) -> bool {
        self.popup.open
    }
    fn focusable(&self) -> bool {
        self.body.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        self.body.set_focus(focused, visible);
    }
    fn semantics(&self) -> ContentSemantics {
        self.body.semantics()
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::PointerUp(_)) {
            self.body_ups += 1;
        }
        let response = self.body.event(event);
        if matches!(event, ContentEvent::PointerDown(_)) && response.capture_pointer {
            self.popup.show(1);
        }
        response
    }
    fn overlay_event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::PointerUp(_)) {
            self.overlay_ups += 1;
        }
        let response = self.overlay.event(event);
        if response.activated {
            self.popup.close();
        }
        response
    }
}
fn handler_capture(open: bool, scoped: bool) -> (ContentHost, ComponentId) {
    let id = ComponentId(51101);
    let mut body = Button::new("Original body");
    let mut overlay = Button::new("Original overlay");
    body.button.frame_policy = tekes_ui_controls::ButtonFramePolicy::Exact;
    overlay.button.frame_policy = tekes_ui_controls::ButtonFramePolicy::Exact;
    let mut popup = Popover::default();
    if open {
        popup.show(1);
    }
    let owner = Content::new(
        id,
        HandlerCapture {
            body,
            overlay,
            popup,
            body_ups: 0,
            overlay_ups: 0,
        },
    );
    let root = if scoped {
        Content::new(
            ComponentId(51100),
            Scope {
                open: true,
                disabled: false,
                children: vec![owner],
            },
        )
    } else {
        owner
    };
    let mut host = ContentHost::new(root).unwrap();
    host.set_action_events(true);
    render(&mut host);
    (host, id)
}
#[test]
fn generic_overlay_capture_releases_to_its_acquiring_handler_once() {
    for scoped in [false, true] {
        let (mut host, id) = handler_capture(true, scoped);
        let point = host.get(id).unwrap().visible().unwrap().center();
        assert!(
            host.event(&ContentEvent::PointerDown(point))
                .capture_pointer
        );
        assert_eq!(host.pointer_capture().map(|(id, _)| id), Some(id));
        assert!(host.event(&ContentEvent::PointerUp(point)).activated);
        let owner = host.get(id).unwrap().component::<HandlerCapture>().unwrap();
        assert_eq!((owner.body_ups, owner.overlay_ups), (0, 1));
        assert_eq!((owner.body.activations, owner.overlay.activations), (0, 1));
        assert!(!owner.popup.open);
        assert!(host.pointer_capture().is_none());
        assert_eq!(host.drain_actions().len(), 1);
    }
}
#[test]
fn scoped_body_capture_keeps_its_handler_when_same_owner_opens_overlay() {
    let (mut host, id) = handler_capture(false, true);
    let point = host.get(id).unwrap().visible().unwrap().center();
    assert!(
        host.event(&ContentEvent::PointerDown(point))
            .capture_pointer
    );
    assert!(
        host.get(id)
            .unwrap()
            .component::<HandlerCapture>()
            .unwrap()
            .popup
            .open
    );
    assert!(host.event(&ContentEvent::PointerUp(point)).activated);
    let owner = host.get(id).unwrap().component::<HandlerCapture>().unwrap();
    assert_eq!((owner.body_ups, owner.overlay_ups), (1, 0));
    assert_eq!((owner.body.activations, owner.overlay.activations), (1, 0));
    assert!(owner.popup.open);
    assert!(host.pointer_capture().is_none());
    assert_eq!(host.drain_actions().len(), 1);
}

#[test]
fn nested_scope_retires_to_outer_original_focus_and_disabled_subtree_is_not_published() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Scope {
            open: true,
            disabled: false,
            children: vec![
                Content::new(ComponentId(2), Button::new("Outer Save")),
                Content::new(
                    ComponentId(3),
                    Scope {
                        open: false,
                        disabled: false,
                        children: vec![Content::new(
                            ComponentId(4),
                            Button::new("Inner Save"),
                        )],
                    },
                ),
            ],
        },
    ))
    .unwrap();
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(2)));
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<Scope>()
        .unwrap()
        .open = true;
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(4)));
    assert!(
        !host
            .accessibility_action(ComponentId(2), None, AccessibilityAction::Press)
            .consumed
    );
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<Scope>()
        .unwrap()
        .open = false;
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(2)));
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<Scope>()
        .unwrap()
        .open = true;
    render(&mut host);
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<Scope>()
        .unwrap()
        .disabled = true;
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(2)));
    assert!(!host.semantics().iter().any(|n| n.id == ComponentId(4)));
}
#[test]
fn cancellation_discards_return_intent_and_does_not_refocus_on_a_later_frame() {
    let mut host = fixture(false);
    host.focus(Some(ComponentId(51003)), true);
    set_open(&mut host, true);
    render(&mut host);
    host.event(&ContentEvent::Cancel);
    render(&mut host);
    assert_eq!(host.focused(), None);
    set_open(&mut host, false);
    render(&mut host);
    assert_eq!(host.focused(), None);
}

#[test]
fn explicit_focus_after_close_is_not_overridden_by_delayed_return_intent() {
    let mut host = fixture(false);
    host.insert(
        ComponentId(51000),
        Content::new(
            ComponentId(51004),
            Button::new("Navigation destination"),
        ),
    )
    .unwrap();
    render(&mut host);
    host.focus(Some(ComponentId(51003)), true);
    set_open(&mut host, true);
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(51002)));
    set_open(&mut host, false);
    assert!(host.focus(Some(ComponentId(51004)), true));
    render(&mut host);
    assert_eq!(host.focused(), Some(ComponentId(51004)));
}
