use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext,
    composition::*,
    extended::{Banner, NoticeTone, Stack, Toast},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const T: ComponentId = ComponentId(790);
const A: ComponentId = ComponentId(791);
const T2: ComponentId = ComponentId(792);
const A2: ComponentId = ComponentId(793);
#[derive(Default)]
struct Text {
    now: Option<Duration>,
    requests: usize,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.now.map(|now| ControlMotionContext {
            now,
            reduced_motion: true,
        })
    }
    fn request_animation_frame(&mut self) {
        self.requests += 1;
    }
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new(w.min(s.len() as f32 * 8.), 18.)
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
fn toast(ms: u32) -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        T,
        ToastContent::new("Notice", Toast::new("Hello", NoticeTone::Warning, ms), A).unwrap(),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn frame() -> Rect {
    Rect::new(0., 0., 360., 200.)
}
fn paint(h: &mut ContentHost, t: &mut Text, us: u64) -> Result<(), CompositionError> {
    t.now = Some(Duration::from_micros(us));
    h.render(&mut Scene::new(), t, frame(), frame(), Theme::LIGHT.into())
}
fn remaining(h: &ContentHost, id: ComponentId) -> u32 {
    h.get(id)
        .unwrap()
        .component::<ToastContent>()
        .unwrap()
        .model()
        .remaining_ms()
}
#[test]
fn mounted_clock_baseline_fraction_pause_show_hide_and_zero() {
    let mut h = toast(1000);
    let mut text = Text::default();
    paint(&mut h, &mut text, 1_000_000).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    assert!(h.animation_frame_requested());
    paint(&mut h, &mut text, 1_000_500).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    paint(&mut h, &mut text, 1_001_250).unwrap();
    assert_eq!(remaining(&h, T), 999);
    h.set_toast_paused(T, true).unwrap();
    paint(&mut h, &mut text, 4_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    assert!(!h.animation_frame_requested());
    h.set_toast_paused(T, false).unwrap();
    paint(&mut h, &mut text, 5_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    h.set_toast_visible(T, false).unwrap();
    paint(&mut h, &mut text, 6_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    assert!(!h.animation_frame_requested());
    h.set_toast_visible(T, true).unwrap();
    paint(&mut h, &mut text, 7_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    h.show_toast(T, "Hello", NoticeTone::Warning, 0).unwrap();
    paint(&mut h, &mut text, 8_000_000).unwrap();
    assert!(
        !h.get(T)
            .unwrap()
            .component::<ToastContent>()
            .unwrap()
            .model()
            .visible
    );
    assert!(!h.animation_frame_requested());
    assert!(h.drain_actions().is_empty());
    h.show_toast(T, "Hello", NoticeTone::Error, u32::MAX)
        .unwrap();
    paint(&mut h, &mut text, 9_000_000).unwrap();
    paint(&mut h, &mut text, u64::MAX).unwrap();
    assert_eq!(remaining(&h, T), 0);
}
#[test]
fn clock_error_all_owners_zero_advance_and_next_valid_baseline() {
    let root = StackContent {
        model: Stack::new(tekes_ui_controls::extended::Axis::Vertical),
        children: vec![
            Content::new(
                T,
                ToastContent::new("One", Toast::new("One", NoticeTone::Info, 1000), A).unwrap(),
            ),
            Content::new(
                T2,
                ToastContent::new("Two", Toast::new("Two", NoticeTone::Success, 1000), A2).unwrap(),
            ),
        ],
        flexible_child: None,
    };
    let mut h = ContentHost::new(Content::new(ComponentId(1), root)).unwrap();
    let mut text = Text::default();
    paint(&mut h, &mut text, 1000).unwrap();
    // Give only the second owner a later baseline, so the first clock is valid
    // when the second is rejected; no earlier sibling may have advanced.
    h.set_toast_paused(T, true).unwrap();
    paint(&mut h, &mut text, 10_000).unwrap();
    h.set_toast_paused(T, false).unwrap();
    assert!(matches!(
        paint(&mut h, &mut text, 5000),
        Err(CompositionError::NoticeClockWentBackwards(_))
    ));
    assert_eq!(remaining(&h, T), 1000);
    assert_eq!(remaining(&h, T2), 991);
    assert!(!h.animation_frame_requested());
    assert!(!h.needs_render());
    paint(&mut h, &mut text, 30_000).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    assert_eq!(remaining(&h, T2), 991);
    text.now = None;
    assert!(matches!(
        h.render(
            &mut Scene::new(),
            &mut text,
            frame(),
            frame(),
            Theme::LIGHT.into()
        ),
        Err(CompositionError::MissingNoticeClock(_))
    ));
    paint(&mut h, &mut text, 100_000).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    assert_eq!(remaining(&h, T2), 991);
}
#[test]
fn actual_pointer_capture_hover_exit_and_once_only_dismiss() {
    let mut h = toast(1000);
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    let p = h.get(A).unwrap().frame().center();
    h.event(&ContentEvent::PointerMove(p));
    assert!(
        h.get(T)
            .unwrap()
            .component::<ToastContent>()
            .unwrap()
            .model()
            .paused
    );
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.event(&ContentEvent::PointerMove(Point::new(500., 500.)));
    assert!(
        !h.get(T)
            .unwrap()
            .component::<ToastContent>()
            .unwrap()
            .model()
            .paused
    );
    h.event(&ContentEvent::PointerMove(p));
    assert!(
        h.get(T)
            .unwrap()
            .component::<ToastContent>()
            .unwrap()
            .model()
            .paused
    );
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    let actions = h.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, A);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn window_clip_and_old_native_mount_fail_closed() {
    let mut h = toast(1000);
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    let old = h.semantic_mount(A).unwrap();
    h.set_window_active(false);
    paint(&mut h, &mut t, 1_000_000).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    assert!(!h.animation_frame_requested());
    h.set_window_active(true);
    assert!(h.semantic_mount(A).is_none());
    paint(&mut h, &mut t, 2_000_000).unwrap();
    assert_ne!(old, h.semantic_mount(A).unwrap());
    assert_eq!(remaining(&h, T), 1000);
    t.now = Some(Duration::from_secs(3));
    h.render(
        &mut Scene::new(),
        &mut t,
        frame(),
        Rect::new(900., 900., 1., 1.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(!h.animation_frame_requested());
    paint(&mut h, &mut t, 4_000_000).unwrap();
    assert_eq!(remaining(&h, T), 1000);
    h.show_toast(T, "Hello", NoticeTone::Warning, 1000).unwrap();
    assert!(h.semantic_mount(A).is_none());
    paint(&mut h, &mut t, 5_000_000).unwrap();
    assert_ne!(old, h.semantic_mount(A).unwrap());
}
#[test]
fn banner_has_no_timer_dismissible_and_original_space_action() {
    let mut h = ContentHost::new(Content::new(
        T,
        BannerContent::new(
            "Announcement",
            Banner {
                message: "Banner".into(),
                tone: NoticeTone::Warning,
                dismissible: false,
                visible: true,
            },
            A,
        )
        .unwrap(),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = Text::default();
    h.render(
        &mut Scene::new(),
        &mut t,
        frame(),
        frame(),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(h.semantic_mount(A).is_none());
    assert!(!h.animation_frame_requested());
    h.replace_banner(
        T,
        Banner {
            message: "Banner".into(),
            tone: NoticeTone::Warning,
            dismissible: true,
            visible: true,
        },
    )
    .unwrap();
    h.render(
        &mut Scene::new(),
        &mut t,
        frame(),
        frame(),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(h.focus(Some(A), true));
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    assert_eq!(h.drain_actions().len(), 1);
}

#[test]
fn prepared_view_paint_never_remeasures() {
    struct Unstable {
        count: usize,
        forbid: bool,
    }
    impl ButtonTextRenderer for Unstable {
        fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
            assert!(!self.forbid);
            self.count += 1;
            Size::new(w, 20. + self.count as f32)
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
    let mut t = Unstable {
        count: 0,
        forbid: false,
    };
    let tm = Toast::new("Saved", NoticeTone::Info, 1000);
    let bm = Banner {
        message: "Banner".into(),
        tone: NoticeTone::Warning,
        dismissible: true,
        visible: true,
    };
    let mut tv = tekes_ui_controls::feedback_view::ToastView::default();
    let mut bv = tekes_ui_controls::feedback_view::BannerView::default();
    tv.prepare(
        &mut t,
        &tm,
        frame(),
        Some(frame()),
        proposal::TextRole::UiBody,
    );
    bv.prepare(
        &mut t,
        &bm,
        frame(),
        Some(frame()),
        proposal::TextRole::UiBody,
    );
    let tf = tv.logical_focus_target();
    let bf = bv.logical_focus_target();
    let count = t.count;
    t.forbid = true;
    tv.paint_prepared(&mut Scene::new(), &mut t, &tm, Theme::LIGHT.into());
    bv.paint_prepared(&mut Scene::new(), &mut t, &bm, Theme::LIGHT.into());
    assert_eq!(count, t.count);
    assert_eq!(tf, tv.logical_focus_target());
    assert_eq!(bf, bv.logical_focus_target());
}
#[test]
fn malformed_slots_metadata_and_surface_fail_closed() {
    for kind in 0..4 {
        let mut h = toast(1000);
        let mut t = Text::default();
        paint(&mut h, &mut t, 0).unwrap();
        let c = h
            .get_mut(T)
            .unwrap()
            .component_mut::<ToastContent>()
            .unwrap();
        assert!(c.set_metadata(" ").is_err());
        let children = c.children_mut().unwrap();
        match kind {
            0 => children.clear(),
            1 => children.push(Content::new(ComponentId(99), Button::new("Foreign"))),
            2 => children[0] = Content::new(A, Button::new("Wrong type")),
            _ => {
                let mut other =
                    ToastContent::new("Other", Toast::new("Other", NoticeTone::Info, 1000), A)
                        .unwrap();
                children[0] = other.children_mut().unwrap().remove(0);
            }
        }
        assert!(h.show_toast(T, "Changed", NoticeTone::Error, 2000).is_err());
        assert!(paint(&mut h, &mut t, 1000).is_err());
        assert!(
            !h.accessibility_action(A, None, AccessibilityAction::Press)
                .activated
        );
        assert!(h.drain_actions().is_empty());
    }
    let mut h = toast(1000);
    assert!(h.set_inherited_surface("unknown").is_err());
    assert!(h.get_mut(T).unwrap().set_surface(Some("unknown")).is_err());
    assert_eq!(remaining(&h, T), 1000);
}
#[derive(Debug)]
struct Overflow {
    children: Vec<Content>,
    active: std::rc::Rc<std::cell::Cell<bool>>,
}
impl Component for Overflow {
    fn active(&self) -> bool {
        self.active.get()
    }
    fn paints_children(&self) -> bool {
        true
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        _: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(360., 200.))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, _: Rect, clip: Rect) {
        self.children[0].arrange(text, frame(), clip);
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
}
#[test]
fn offscreen_parent_overflow_child_and_inactive_ancestry() {
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let root = Overflow {
        children: vec![Content::new(
            T,
            ToastContent::new("Toast", Toast::new("Saved", NoticeTone::Info, 1000), A).unwrap(),
        )],
        active: active.clone(),
    };
    let mut h = ContentHost::new(Content::new(ComponentId(1), root)).unwrap();
    let mut t = Text::default();
    for us in [0, 1000] {
        t.now = Some(Duration::from_micros(us));
        h.render(
            &mut Scene::new(),
            &mut t,
            Rect::new(900., 900., 360., 200.),
            frame(),
            Theme::LIGHT.into(),
        )
        .unwrap();
    }
    assert_eq!(remaining(&h, T), 999);
    active.set(false);
    paint(&mut h, &mut t, 1_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    assert!(!h.animation_frame_requested());
    active.set(true);
    paint(&mut h, &mut t, 2_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
}

#[test]
fn original_unmount_retains_model_but_resumes_with_new_baseline_and_mount() {
    let root = ComponentId(1);
    let node = Content::new(
        T,
        ToastContent::new("Toast", Toast::new("Saved", NoticeTone::Info, 1000), A).unwrap(),
    );
    let mut h = ContentHost::new(Content::new(
        root,
        StackContent {
            model: Stack::new(tekes_ui_controls::extended::Axis::Vertical),
            children: vec![node],
            flexible_child: None,
        },
    ))
    .unwrap();
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    paint(&mut h, &mut t, 1000).unwrap();
    let old = h.semantic_mount(A);
    let node = h.remove(T).unwrap();
    paint(&mut h, &mut t, 4_000_000).unwrap();
    assert!(!h.animation_frame_requested());
    h.insert(root, node).unwrap();
    paint(&mut h, &mut t, 5_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    assert_ne!(old, h.semantic_mount(A));
}
#[test]
fn pure_theme_resize_and_surface_preserve_original_space_arm() {
    let mut h = toast(1000);
    h.set_action_events(true);
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    let old = h.semantic_mount(A);
    assert!(h.focus(Some(A), true));
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    h.set_inherited_surface("panel").unwrap();
    t.now = Some(Duration::ZERO);
    let f = Rect::new(0.25, 0.5, 240., 220.);
    h.render(&mut Scene::new(), &mut t, f, f, Theme::DARK.into())
        .unwrap();
    assert_eq!(old, h.semantic_mount(A));
    assert_eq!(h.focused(), Some(A));
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    assert_eq!(h.drain_actions().len(), 1);
}
#[test]
fn passive_root_unknown_actions_and_finite_allocation_contract() {
    let mut h = toast(1000);
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    assert!(!h.focus(Some(T), true));
    assert!(
        !h.accessibility_action(T, None, AccessibilityAction::Press)
            .activated
    );
    for a in [
        AccessibilityAction::SetValue("1".into()),
        AccessibilityAction::Increment,
        AccessibilityAction::Decrement,
    ] {
        assert!(!h.accessibility_action(A, None, a).activated);
    }
    assert!(
        !h.accessibility_action(A, Some(1), AccessibilityAction::Press)
            .activated
    );
    let f = frame();
    let mut allocation = (f, f);
    for (frame, clip) in [
        (Rect::new(f32::NAN, 0., 1., 1.), f),
        (Rect::new(0., 0., -1., 1.), f),
        (f, Rect::new(0., 0., f32::INFINITY, 1.)),
    ] {
        assert!(
            !tekes_ui_composition_consumer::empty_state_content::set_allocation(
                &mut allocation,
                frame,
                clip
            )
        );
        assert_eq!(allocation, (f, f));
    }
    assert_eq!(remaining(&h, T), 1000);
    assert!(h.drain_actions().is_empty());
}

#[test]
fn observed_inactive_ancestor_clears_sample_without_an_intervening_layout() {
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let root = Overflow {
        children: vec![Content::new(
            T,
            ToastContent::new("Toast", Toast::new("Saved", NoticeTone::Info, 1000), A).unwrap(),
        )],
        active: active.clone(),
    };
    let mut h = ContentHost::new(Content::new(ComponentId(1), root)).unwrap();
    let mut t = Text::default();
    paint(&mut h, &mut t, 0).unwrap();
    paint(&mut h, &mut t, 1000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    let old = h.semantic_mount(A);
    active.set(false);
    h.event(&ContentEvent::PointerMove(Point::new(0., 0.)));
    active.set(true);
    paint(&mut h, &mut t, 5_000_000).unwrap();
    assert_eq!(remaining(&h, T), 999);
    assert_ne!(old, h.semantic_mount(A));
}

#[test]
fn completed_dismiss_stays_once_only_across_its_hidden_repaint() {
    let mut h=toast(1000);let mut t=Text::default();paint(&mut h,&mut t,0).unwrap();
    assert!(h.accessibility_action(A,None,AccessibilityAction::Press).activated);
    paint(&mut h,&mut t,1_000_000).unwrap();
    assert!(!h.accessibility_action(A,None,AccessibilityAction::Press).activated);
    let actions=h.drain_actions();assert_eq!(actions.len(),1);assert_eq!(actions[0].target,A);assert!(h.drain_actions().is_empty());
}

#[test]
fn completed_banner_dismiss_stays_once_only_across_its_hidden_repaint() {
    let mut h=ContentHost::new(Content::new(T,BannerContent::new("Banner",Banner {
        message:"Saved".into(),tone:NoticeTone::Info,dismissible:true,visible:true,
    },A).unwrap())).unwrap();
    h.set_action_events(true);let mut t=Text::default();paint(&mut h,&mut t,0).unwrap();
    assert!(h.accessibility_action(A,None,AccessibilityAction::Press).activated);
    paint(&mut h,&mut t,1_000_000).unwrap();
    assert!(h.semantic_mount(A).is_none());
    assert!(!h.accessibility_action(A,None,AccessibilityAction::Press).activated);
    let actions=h.drain_actions();assert_eq!(actions.len(),1);assert_eq!(actions[0].target,A);assert!(h.drain_actions().is_empty());
}
#[test]
fn completed_notice_dismiss_is_revoked_by_observed_ancestor_or_window_aba_before_drain() {
    for banner in [false,true] {
        for window in [false,true] {
            let active=std::rc::Rc::new(std::cell::Cell::new(true));
            let notice=if banner {Content::new(T,BannerContent::new("Banner",Banner {
                message:"Saved".into(),tone:NoticeTone::Info,dismissible:true,visible:true,
            },A).unwrap())}else{Content::new(T,ToastContent::new("Toast",Toast::new("Saved",NoticeTone::Info,1000),A).unwrap())};
            let mut h=ContentHost::new(Content::new(ComponentId(1),Overflow {
                active:active.clone(),children:vec![notice],
            })).unwrap();
            h.set_action_events(true);let mut t=Text::default();paint(&mut h,&mut t,0).unwrap();
            assert!(h.accessibility_action(A,None,AccessibilityAction::Press).activated);
            if window {
                h.set_window_active(false);h.set_window_active(true);
            }else{
                active.set(false);h.event(&ContentEvent::PointerMove(Point::new(0.,0.)));
                active.set(true);h.event(&ContentEvent::PointerMove(Point::new(0.,0.)));
            }
            paint(&mut h,&mut t,1_000_000).unwrap();
            assert!(h.drain_actions().is_empty(),"banner={banner} window={window}");
            assert!(h.drain_actions().is_empty());
            assert!(!h.accessibility_action(A,None,AccessibilityAction::Press).activated);
        }
    }
}

#[test]
fn ordinary_component_frame_requests_survive_the_notice_adapter_and_settle() {
    #[derive(Debug)]
    struct FrameRequester(bool);
    impl Component for FrameRequester {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(available)
        }
        fn paint(&mut self, _: &mut Scene, text: &mut dyn ButtonTextRenderer, _: Rect, _: Rect, _: ContentTheme) {
            if self.0 { text.request_animation_frame(); }
        }
    }
    let id = ComponentId(799);
    let mut h = ContentHost::new(Content::new(id, FrameRequester(true))).unwrap();
    let mut text = Text::default();
    paint(&mut h, &mut text, 0).unwrap();
    assert_eq!(text.requests, 1);
    assert!(h.animation_frame_requested());
    h.get_mut(id).unwrap().component_mut::<FrameRequester>().unwrap().0 = false;
    paint(&mut h, &mut text, 1000).unwrap();
    assert_eq!(text.requests, 1);
    assert!(!h.animation_frame_requested());
}
