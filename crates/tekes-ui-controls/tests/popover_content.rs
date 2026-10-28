use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Debug)]
struct Screen {
    children: Vec<Content>,
    anchor: Rect,
    popup_clip: Option<Rect>,
    active: bool,
}
impl Component for Screen {
    fn active(&self) -> bool {
        self.active
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        for child in &mut self.children {
            let (f, c) = match child.id().0 {
                2 => (self.anchor, clip),
                6 => (
                    Rect::new(0., 64., frame.size.width, (frame.size.height - 64.).max(0.)),
                    clip,
                ),
                _ => (
                    frame,
                    self.popup_clip
                        .map(|c| c.intersection(&clip).unwrap_or_default())
                        .unwrap_or(clip),
                ),
            };
            child.arrange(text, f, c);
        }
    }
}
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
fn popup(owner: u64, child: u64) -> Content {
    let mut popup = PopoverContent::new(
        "Original form",
        Content::new(ComponentId(child), Button::new("Save")),
    );
    popup.initial_focus = Some(ComponentId(child));
    popup.set_dismiss_visible(false);
    Content::new(ComponentId(owner), popup)
}
fn fixture() -> ContentHost {
    let mut host = ContentHost::new(Content::new(
        ComponentId(1),
        Screen {
            children: vec![
                popup(3, 4),
                Content::new(ComponentId(2), Button::new("Open form")),
                Content::new(ComponentId(6), Button::new("Background action")),
            ],
            anchor: Rect::new(20., 20., 120., 40.),
            popup_clip: None,
            active: true,
        },
    ))
    .unwrap();
    host.set_action_events(true);
    render(&mut host, 600., 500.);
    host
}
fn render(host: &mut ContentHost, w: f32, h: f32) -> Scene {
    let f = Rect::new(0., 0., w, h);
    let mut scene = Scene::new();
    host.render(&mut scene, &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
    scene
}
fn open(host: &mut ContentHost) -> PopoverSession {
    let session = host.show_popover(ComponentId(3), ComponentId(2)).unwrap();
    render(host, 600., 500.);
    session
}
fn state(host: &ContentHost) -> &PopoverContent {
    host.get(ComponentId(3))
        .unwrap()
        .component::<PopoverContent>()
        .unwrap()
}
fn closed(host: &mut ContentHost) -> Vec<PopoverClosed> {
    host.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<PopoverContent>()
        .unwrap()
        .take_closed()
}
#[test]
fn standard_entry_uses_original_anchor_and_subtree_above_later_background() {
    let mut h = fixture();
    let session = open(&mut h);
    assert_eq!(state(&h).layout().anchor, Rect::new(20., 20., 120., 40.));
    assert_eq!(h.focused(), Some(ComponentId(4)));
    let point = h.get(ComponentId(4)).unwrap().visible().unwrap().center();
    assert!(
        h.get(ComponentId(6))
            .unwrap()
            .visible()
            .unwrap()
            .contains(point)
    );
    h.event(&ContentEvent::PointerDown(point));
    h.event(&ContentEvent::PointerUp(point));
    assert_eq!(
        h.drain_actions()
            .iter()
            .map(|a| a.target)
            .collect::<Vec<_>>(),
        vec![ComponentId(4)]
    );
    assert!(
        !h.accessibility_action(ComponentId(6), None, AccessibilityAction::Press)
            .consumed
    );
    assert!(h.close_popover(session));
    assert_eq!(h.focused(), Some(ComponentId(2)));
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session,
            reason: PopoverCloseReason::Requested
        }]
    );
}
#[test]
fn stale_opening_and_replaced_owner_handles_cannot_close_current_form() {
    let mut h = fixture();
    let first = open(&mut h);
    assert!(h.close_popover(first));
    let second = open(&mut h);
    assert_ne!(first, second);
    assert!(!h.close_popover(first));
    assert!(state(&h).is_open());
    h.remove(ComponentId(3)).unwrap();
    h.insert(ComponentId(1), popup(3, 4)).unwrap();
    let third = open(&mut h);
    assert_ne!(second, third);
    assert!(!h.close_popover(second));
    assert!(state(&h).is_open());
}
#[test]
fn original_anchor_reorder_keeps_mount_and_resizes_use_sdk_clip() {
    let mut h = fixture();
    let session = open(&mut h);
    let mount = h.semantic_mount(ComponentId(4));
    h.reorder(
        ComponentId(1),
        &[ComponentId(6), ComponentId(3), ComponentId(2)],
    )
    .unwrap();
    for width in [180., 360., 720.] {
        render(&mut h, width, 300.);
        assert_eq!(state(&h).current_session(), Some(session));
        assert_eq!(h.semantic_mount(ComponentId(4)), mount);
        let f = state(&h).layout().frame;
        assert!(f.min_x() >= 0. && f.max_x() <= width && f.max_y() <= 300.);
    }
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Screen>()
        .unwrap()
        .popup_clip = Some(Rect::new(20., 70., 120., 180.));
    render(&mut h, 600., 500.);
    let l = state(&h).layout();
    assert!(
        l.frame.min_x() >= 20.
            && l.frame.max_x() <= 140.
            && l.frame.min_y() >= 70.
            && l.frame.max_y() <= 250.
    );
}
#[test]
fn original_anchor_hidden_unmounted_or_replaced_closes_without_return_to_replacement() {
    for reason in [
        PopoverCloseReason::AnchorHidden,
        PopoverCloseReason::AnchorUnmounted,
        PopoverCloseReason::AnchorReplaced,
    ] {
        let mut h = fixture();
        let session = open(&mut h);
        match reason {
            PopoverCloseReason::AnchorHidden => {
                h.get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<Screen>()
                    .unwrap()
                    .anchor = Rect::new(-800., 20., 120., 40.)
            }
            PopoverCloseReason::AnchorUnmounted => {
                h.remove(ComponentId(2)).unwrap();
            }
            _ => {
                let screen = h
                    .get_mut(ComponentId(1))
                    .unwrap()
                    .component_mut::<Screen>()
                    .unwrap();
                let i = screen
                    .children
                    .iter()
                    .position(|c| c.id() == ComponentId(2))
                    .unwrap();
                screen.children[i] =
                    Content::new(ComponentId(2), Button::new("Replacement anchor"));
            }
        }
        render(&mut h, 600., 500.);
        assert!(!state(&h).is_open());
        assert_eq!(h.focused(), None);
        assert_eq!(closed(&mut h), vec![PopoverClosed { session, reason }]);
        assert!(!h.close_popover(session));
        assert!(!h.semantics().iter().any(|node| node.id == ComponentId(4)));
    }
}
#[test]
fn disabled_and_window_loss_retire_arms_and_never_reopen_on_activation() {
    let mut h = fixture();
    let session = open(&mut h);
    let point = h.get(ComponentId(4)).unwrap().visible().unwrap().center();
    h.event(&ContentEvent::PointerDown(point));
    h.set_window_active(false);
    assert!(!state(&h).is_open());
    assert_eq!(h.focused(), None);
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session,
            reason: PopoverCloseReason::WindowDeactivated
        }]
    );
    assert!(h.show_popover(ComponentId(3), ComponentId(2)).is_err());
    h.set_window_active(true);
    h.event(&ContentEvent::PointerUp(point));
    assert!(h.drain_actions().is_empty());
    assert!(!state(&h).is_open());
    let next = open(&mut h);
    h.get_mut(ComponentId(3))
        .unwrap()
        .component_mut::<PopoverContent>()
        .unwrap()
        .set_enabled(false);
    h.event(&ContentEvent::PointerMove(point));
    assert_eq!(h.focused(), None);
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session: next,
            reason: PopoverCloseReason::Disabled
        }]
    );
}
#[test]
fn second_original_popup_opens_above_earlier_popup_regardless_document_order() {
    let mut h = fixture();
    let first = open(&mut h);
    h.insert(ComponentId(1), popup(7, 8)).unwrap();
    h.reorder(
        ComponentId(1),
        &[
            ComponentId(7),
            ComponentId(3),
            ComponentId(2),
            ComponentId(6),
        ],
    )
    .unwrap();
    let nested = h.show_popover(ComponentId(7), ComponentId(4)).unwrap();
    render(&mut h, 600., 500.);
    assert_eq!(h.focused(), Some(ComponentId(8)));
    assert!(
        !h.accessibility_action(ComponentId(4), None, AccessibilityAction::Press)
            .consumed
    );
    assert!(h.close_popover(nested));
    assert_eq!(h.focused(), Some(ComponentId(4)));
    assert_eq!(state(&h).current_session(), Some(first));
}
#[test]
fn outside_and_escape_are_once_only_close_reasons_and_tab_is_scoped() {
    let mut h = fixture();
    let session = open(&mut h);
    for reverse in [false, true] {
        h.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse },
            down: true,
        });
        assert_eq!(h.focused(), Some(ComponentId(4)));
    }
    h.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session,
            reason: PopoverCloseReason::Escape
        }]
    );
    let session = open(&mut h);
    let outside = Point::new(599., 499.);
    assert!(h.event(&ContentEvent::PointerDown(outside)).consumed);
    assert!(h.event(&ContentEvent::PointerUp(outside)).consumed);
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session,
            reason: PopoverCloseReason::OutsidePointer
        }]
    );
    assert!(h.drain_actions().is_empty());
}

#[test]
fn opening_standard_popover_does_not_cancel_an_existing_original_capture() {
    let mut h = fixture();
    let point = Point::new(300., 300.);
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    let session = open(&mut h);
    h.event(&ContentEvent::PointerUp(point));
    assert_eq!(
        h.drain_actions()
            .iter()
            .map(|a| a.target)
            .collect::<Vec<_>>(),
        vec![ComponentId(6)]
    );
    assert_eq!(h.focused(), Some(ComponentId(4)));
    assert!(h.close_popover(session));
}
#[test]
fn closing_outer_owner_retires_nested_anchor_and_returns_to_original_invoker() {
    let mut h = fixture();
    let outer = open(&mut h);
    h.insert(ComponentId(1), popup(7, 8)).unwrap();
    let inner = h.show_popover(ComponentId(7), ComponentId(4)).unwrap();
    render(&mut h, 600., 500.);
    assert!(h.close_popover(outer));
    assert!(!h.close_popover(inner));
    assert_eq!(h.focused(), Some(ComponentId(2)));
    let events = h
        .get_mut(ComponentId(7))
        .unwrap()
        .component_mut::<PopoverContent>()
        .unwrap()
        .take_closed();
    assert_eq!(
        events,
        vec![PopoverClosed {
            session: inner,
            reason: PopoverCloseReason::AnchorHidden
        }]
    );
}
#[test]
fn clipped_owner_retires_original_input_and_ax_without_replaying_exit_on_return() {
    let mut h = fixture();
    let session = open(&mut h);
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Screen>()
        .unwrap()
        .popup_clip = Some(Rect::new(900., 900., 10., 10.));
    render(&mut h, 600., 500.);
    assert!(!state(&h).is_open());
    assert!(!state(&h).is_presented());
    assert_eq!(h.focused(), None);
    assert_eq!(
        closed(&mut h),
        vec![PopoverClosed {
            session,
            reason: PopoverCloseReason::OwnerHidden
        }]
    );
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<Screen>()
        .unwrap()
        .popup_clip = None;
    render(&mut h, 600., 500.);
    assert!(!state(&h).is_open());
    assert!(!h.semantics().iter().any(|n| n.id == ComponentId(4)));
}

#[test]
fn inactive_host_rejects_late_background_input_and_ax_before_reactivation() {
    let mut h = fixture();
    open(&mut h);
    h.set_window_active(false);
    assert!(
        !h.accessibility_action(ComponentId(6), None, AccessibilityAction::Press)
            .consumed
    );
    assert!(!h.focus(Some(ComponentId(6)), true));
    h.event(&ContentEvent::PointerDown(Point::new(300., 300.)));
    h.event(&ContentEvent::PointerUp(Point::new(300., 300.)));
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert!(h.drain_actions().is_empty());
    assert_eq!(h.focused(), None);
    h.set_window_active(true);
    assert!(!state(&h).is_open());
    assert!(
        h.accessibility_action(ComponentId(6), None, AccessibilityAction::Press)
            .activated
    );
}
#[test]
fn registered_shells_paint_once_in_opening_order_without_sibling_reordering() {
    let mut h = fixture();
    open(&mut h);
    h.insert(ComponentId(1), popup(7, 8)).unwrap();
    h.reorder(
        ComponentId(1),
        &[
            ComponentId(7),
            ComponentId(3),
            ComponentId(2),
            ComponentId(6),
        ],
    )
    .unwrap();
    h.show_popover(ComponentId(7), ComponentId(4)).unwrap();
    let scene = render(&mut h, 600., 500.);
    let frames = [
        state(&h).layout().frame,
        h.get(ComponentId(7))
            .unwrap()
            .component::<PopoverContent>()
            .unwrap()
            .layout()
            .frame,
    ];
    let color = tekes_ui_tokens::Palette::from(Theme::LIGHT)
        .color("surface-popover")
        .unwrap();
    let positions: Vec<_> = scene
        .primitives()
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p {
            tekes_ui_scene::Primitive::Quad(q) if q.fill == color && frames.contains(&q.rect) => {
                Some((i, q.rect))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        positions.len(),
        2,
        "each original shell is painted exactly once"
    );
    assert_eq!(positions[0].1, frames[0]);
    assert_eq!(positions[1].1, frames[1]);
}

#[test]
fn opening_dialog_retires_external_popup_paint_and_keeps_dialog_popup_visible() {
    let mut h = fixture();
    h.insert(
        ComponentId(1),
        Content::new(
            ComponentId(20),
            DialogContent::new(
                "Modal",
                Content::new(ComponentId(21), TextContent::new("Header")),
                Content::new(ComponentId(22), Button::new("Dialog action")),
                popup(23, 24),
            ),
        ),
    )
    .unwrap();
    open(&mut h);
    h.show_dialog(ComponentId(20)).unwrap();
    let scene = render(&mut h, 600., 500.);
    assert!(!state(&h).is_open());
    let color = tekes_ui_tokens::Palette::from(Theme::LIGHT)
        .color("surface-popover")
        .unwrap();
    let count = |scene: &Scene, frame: Rect| {
        scene
            .primitives()
            .iter()
            .filter(|p| {
                matches!(p,
            tekes_ui_scene::Primitive::Quad(q) if q.fill == color && q.rect == frame)
            })
            .count()
    };
    assert_eq!(
        count(&scene, state(&h).layout().frame),
        0,
        "retiring background popup cannot cover modal"
    );
    assert!(!h.semantics().iter().any(|n| n.id == ComponentId(4)));
    h.show_popover(ComponentId(23), ComponentId(22)).unwrap();
    let scene = render(&mut h, 600., 500.);
    assert_eq!(
        count(
            &scene,
            h.get(ComponentId(23))
                .unwrap()
                .component::<PopoverContent>()
                .unwrap()
                .layout()
                .frame
        ),
        1,
        "original popup inside modal paints exactly once"
    );
    assert_eq!(h.focused(), Some(ComponentId(24)));
}

#[test]
fn projected_input_overlay_identity_is_original_mount_and_excludes_exit_paint() {
    let mut host = fixture();
    assert_eq!(host.input_overlay_owner(), None);
    let session = open(&mut host);
    assert_eq!(
        host.input_overlay_owner(),
        Some((ComponentId(3), host.semantic_mount(ComponentId(3)).unwrap()))
    );
    assert!(host.close_popover(session));
    assert_eq!(
        host.input_overlay_owner(),
        None,
        "retained closing paint is not an input scope"
    );
    render(&mut host, 600., 500.);
    let next = open(&mut host);
    host.cancel();
    assert_eq!(host.input_overlay_owner(), None);
    assert!(
        !host.close_popover(next),
        "old scope completion remained executable"
    );
}

#[test]
fn original_frame_demand_crosses_renderer_adapter_and_retires_after_motion() {
    struct Clock {
        now: std::time::Duration,
        reduced: bool,
        forwarded: bool,
    }
    impl ButtonTextRenderer for Clock {
        fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
            Some(tekes_ui_controls::ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced,
            })
        }
        fn request_animation_frame(&mut self) {
            self.forwarded = true;
        }
        fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
            Text.measure(s, role, width)
        }
        fn draw(
            &mut self,
            scene: &mut Scene,
            s: &str,
            role: proposal::TextRole,
            frame: Rect,
            align: ButtonTextAlign,
            color: Rgba,
        ) {
            Text.draw(scene, s, role, frame, align, color);
        }
    }
    let mut host = fixture();
    let frame = Rect::new(0., 0., 600., 500.);
    let mut clock = Clock {
        now: std::time::Duration::ZERO,
        reduced: false,
        forwarded: false,
    };
    let session = host.show_popover(ComponentId(3), ComponentId(2)).unwrap();
    host.render(
        &mut Scene::new(),
        &mut clock,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(clock.forwarded && host.animation_frame_requested());
    // Observing the demand does not acknowledge it or generate a new request.
    assert!(host.animation_frame_requested());
    clock.now = std::time::Duration::from_secs(2);
    clock.forwarded = false;
    host.render(
        &mut Scene::new(),
        &mut clock,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(!host.animation_frame_requested() && !clock.forwarded);
    assert!(host.close_popover(session));
    host.render(
        &mut Scene::new(),
        &mut clock,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(host.animation_frame_requested());
    clock.reduced = true;
    clock.forwarded = false;
    host.render(
        &mut Scene::new(),
        &mut clock,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(!host.animation_frame_requested() && !clock.forwarded);
    host.set_window_active(false);
    assert!(!host.animation_frame_requested());
}
