#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, composition::*, extended::SegmentedControl, layout_view::SegmentedControlView,
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
const ID: ComponentId = ComponentId(110);
fn labels() -> Vec<String> {
    vec!["首段 First".into(), "".into(), "首段 First".into()]
}
fn host() -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ID,
        SegmentedControlContent::new("Segmented", labels()),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn c(h: &ContentHost) -> &SegmentedControlContent {
    h.get(ID).unwrap().component().unwrap()
}
fn cm(h: &mut ContentHost) -> &mut SegmentedControlContent {
    h.get_mut(ID).unwrap().component_mut().unwrap()
}
fn frame() -> Rect {
    Rect::new(10., 10., 330., 48.)
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, clip: Rect) -> Scene {
    let mut s = Scene::new();
    h.render(&mut s, t, frame(), clip, Theme::LIGHT.into())
        .unwrap();
    s
}
fn key(h: &mut ContentHost, k: ControlKey, down: bool) -> ContentResponse {
    h.event(&ContentEvent::Key { key: k, down })
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    c(h).semantic_parts()
}
fn payloads(h: &mut ContentHost) -> Vec<usize> {
    h.drain_action_records()
        .into_iter()
        .map(|r| {
            assert_eq!(r.action.kind, ContentActionKind::Activate);
            match r.payload.unwrap() {
                ContentActionPayload::SegmentSelection(i) => i.index(),
                _ => panic!("wrong payload"),
            }
        })
        .collect()
}
#[test]
fn original_pointer_keyboard_and_focus_are_retained() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, frame());
    let p = parts(&h)[1].frame.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    assert_eq!(c(&h).model().selected, 0);
    assert_eq!(c(&h).view().focused(), Some(1));
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(payloads(&mut h), vec![1]);
    assert!(payloads(&mut h).is_empty());
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    let token = parts(&h)[2].index;
    assert!(
        h.accessibility_action(ID, Some(token), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(c(&h).model().selected, 1);
    assert_eq!(c(&h).view().focused(), Some(2));
    assert!(key(&mut h, ControlKey::Space, true).consumed);
    assert!(key(&mut h, ControlKey::Space, true).consumed);
    assert_eq!(c(&h).model().selected, 1);
    assert!(key(&mut h, ControlKey::Space, false).activated);
    assert_eq!(payloads(&mut h), vec![2]);
    assert!(!key(&mut h, ControlKey::Space, false).activated);
    assert!(key(&mut h, ControlKey::Left, true).activated);
    assert_eq!(payloads(&mut h), vec![1]);
    assert!(key(&mut h, ControlKey::Home, true).consumed);
    assert_eq!(payloads(&mut h), vec![0]);
    assert!(key(&mut h, ControlKey::Left, true).consumed);
    assert!(payloads(&mut h).is_empty());
    cm(&mut h).set_item_enabled(2, false).unwrap();
    render(&mut h, &mut t, frame());
    assert!(key(&mut h, ControlKey::End, true).activated);
    assert_eq!(payloads(&mut h), vec![1]);
}
#[test]
fn controlled_updates_preflight_then_cancel_and_keep_focus() {
    for pointer in [false, true] {
        let mut h = host();
        let mut t = coretext::Text::new(1.);
        render(&mut h, &mut t, frame());
        let p = parts(&h)[1].frame.center();
        if pointer {
            h.event(&ContentEvent::PointerDown(p));
        } else {
            let token = parts(&h)[1].index;
            h.accessibility_action(ID, Some(token), AccessibilityAction::Focus);
            key(&mut h, ControlKey::Space, true);
        }
        cm(&mut h).set_selected(2).unwrap();
        assert_eq!(c(&h).view().focused(), Some(1));
        let r = if pointer {
            h.event(&ContentEvent::PointerUp(p))
        } else {
            key(&mut h, ControlKey::Space, false)
        };
        assert!(!r.activated);
        assert_eq!(c(&h).model().selected, 2);
        assert!(payloads(&mut h).is_empty());
    }
}
#[test]
fn invalid_and_same_updates_do_not_cancel_or_mutate() {
    for invalid in [0, 1, 2] {
        let mut h = host();
        let mut t = coretext::Text::new(1.);
        cm(&mut h).set_item_enabled(2, false).unwrap();
        render(&mut h, &mut t, frame());
        let p = parts(&h)[1].frame.center();
        h.event(&ContentEvent::PointerDown(p));
        let before = format!("{:?}", c(&h));
        match invalid {
            0 => cm(&mut h).set_selected(0).unwrap(),
            1 => assert!(cm(&mut h).set_selected(99).is_err()),
            _ => assert!(cm(&mut h).set_selected(2).is_err()),
        }
        assert_eq!(format!("{:?}", c(&h)), before);
        assert!(h.event(&ContentEvent::PointerUp(p)).activated);
        assert_eq!(payloads(&mut h), vec![1]);
    }
}
#[test]
fn delivery_payload_survives_setter_cancel_and_unrelated_sibling() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, frame());
    let tokens = parts(&h);
    h.accessibility_action(ID, Some(tokens[1].index), AccessibilityAction::Press);
    cm(&mut h).set_selected(2).unwrap();
    h.event(&ContentEvent::Cancel);
    cm(&mut h).set_item_enabled(0, false).unwrap();
    render(&mut h, &mut t, frame());
    assert_eq!(parts(&h)[0].index, tokens[1].index);
    assert_eq!(payloads(&mut h), vec![1]);
    assert!(payloads(&mut h).is_empty());
}
#[test]
fn disabled_inactive_and_clip_aba_reject_old_parts_and_intents() {
    for lane in 0..4 {
        let mut h = host();
        let mut t = coretext::Text::new(1.);
        render(&mut h, &mut t, frame());
        let old = parts(&h)[2].index;
        h.accessibility_action(ID, Some(old), AccessibilityAction::Press);
        match lane {
            0 => {
                cm(&mut h).set_item_enabled(2, false).unwrap();
                cm(&mut h).set_item_enabled(2, true).unwrap();
            }
            1 => {
                cm(&mut h).set_disabled(true);
                cm(&mut h).set_disabled(false);
            }
            2 => {
                h.set_window_active(false);
                h.set_window_active(true);
            }
            _ => {
                render(&mut h, &mut t, Rect::new(10., 10., 110., 48.));
            }
        }
        render(&mut h, &mut t, frame());
        assert!(payloads(&mut h).is_empty());
        let before = c(&h).view().focused();
        for a in [AccessibilityAction::Focus, AccessibilityAction::Press] {
            assert!(!h.accessibility_action(ID, Some(old), a).consumed);
            assert_eq!(c(&h).view().focused(), before);
        }
        let new = parts(&h).last().unwrap().index;
        assert_ne!(new, old);
        cm(&mut h).set_selected(0).unwrap();
        assert!(
            h.accessibility_action(ID, Some(new), AccessibilityAction::Press)
                .activated
        );
        assert_eq!(payloads(&mut h), vec![2]);
    }
}
#[test]
fn empty_duplicate_labels_all_disabled_and_invalid_constructor_are_finite() {
    assert!(SegmentedControlContent::from_model("bad", SegmentedControl::new(1), vec![]).is_err());
    let mut m = SegmentedControl::new(2);
    m.selected = 8;
    assert!(SegmentedControlContent::from_model("bad", m, vec!["".into(), "".into()]).is_err());
    let mut t = coretext::Text::new(1.);
    for n in [0, 3] {
        let mut c = SegmentedControlContent::new("", vec!["".into(); n]);
        for i in 0..n {
            c.set_item_enabled(i, false).unwrap();
        }
        let mut h = ContentHost::new(Content::new(ID, c)).unwrap();
        render(&mut h, &mut t, frame());
        assert!(parts(&h).is_empty());
        assert!(!c_for_focus(&h));
        assert!(h.focus_targets().is_empty());
    }
}
fn c_for_focus(h: &ContentHost) -> bool {
    c(h).focusable()
}
#[test]
fn original_view_geometry_and_paint_match_at_focus_and_clip() {
    for theme in Theme::ALL {
        for compact in [false, true] {
            for focus in [0, 1, 2] {
                for width in [70., 330.] {
                    let f = Rect::new(10., 10., width, 60.);
                    let clip = Rect::new(4., 4., width + 12., 55.);
                    let mut h = host();
                    cm(&mut h).set_compact(compact);
                    let mut t = coretext::Text::new(1.5);
                    let mut first = Scene::new();
                    h.render(&mut first, &mut t, f, clip, theme.into()).unwrap();
                    let token = parts(&h)[focus].index;
                    h.accessibility_action(ID, Some(token), AccessibilityAction::Focus);
                    let mut actual = Scene::new();
                    h.render(&mut actual, &mut t, f, clip, theme.into())
                        .unwrap();
                    // Compare the original View at the frame allocated by the
                    // real Host, which may expand a wrapping intrinsic height.
                    let f = h.get(ID).unwrap().frame();
                    let mut model = SegmentedControl::new(3);
                    let mut view = SegmentedControlView::new(3);
                    // Match the Surface inherited by the real Host, as well as its palette.
                    view.set_surface(ContentTheme::from(tekes_ui_tokens::Palette::from(theme)).surface())
                        .unwrap();
                    let owned = labels();
                    let ls = owned.iter().map(String::as_str).collect::<Vec<_>>();
                    let mut ot = coretext::Text::new(1.5);
                    view.render(
                        &mut Scene::new(),
                        &mut ot,
                        &mut model,
                        &ls,
                        f,
                        Some(clip),
                        theme.into(),
                        proposal::TextRole::UiBody,
                        compact,
                    );
                    view.focus(focus);
                    let mut expected = Scene::new();
                    expected.push_clip(clip);
                    let layout = view.render(
                        &mut expected,
                        &mut ot,
                        &mut model,
                        &ls,
                        f,
                        Some(clip),
                        theme.into(),
                        proposal::TextRole::UiBody,
                        compact,
                    );
                    expected.pop_clip();
                    assert_eq!(*c(&h).view().layout(), layout);
                    assert_eq!(
                        format!("{:?}", actual),
                        format!("{:?}", expected),
                        "{theme:?} {compact} {focus} {width}"
                    );
                    assert!(payloads(&mut h).is_empty());
                }
            }
        }
    }
}
#[test]
fn real_composer_marked_selection_and_binding_gate_before_original_view() {
    let editor = ComponentId(111);
    let mut field = TextFieldContent::new("Composer", "draft 原文");
    field.surface = tekes_ui_controls::TextSurface::new("draft 原文", true, false);
    let content = Content::new(
        ComponentId(1),
        StackContent {
            model: tekes_ui_controls::extended::Stack {
                axis: tekes_ui_controls::extended::Axis::Vertical,
                gap: 4.,
            },
            flexible_child: None,
            children: vec![
                Content::new(editor, field),
                Content::new(ID, SegmentedControlContent::new("seg", labels())),
            ],
        },
    );
    let mut h = ContentHost::new(content).unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    let mut s = Scene::new();
    let f = Rect::new(0., 0., 400., 240.);
    h.render(&mut s, &mut t, f, f, Theme::LIGHT.into()).unwrap();
    h.focus(Some(editor), true);
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    let binding = h.focused_text_binding();
    let before = format!("{:?}", h.focused_text().unwrap());
    let view = format!("{:?}", c(&h).view());
    let token = parts(&h)[1].index;
    for a in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        assert!(!h.accessibility_action(ID, Some(token), a).consumed);
        assert_eq!(format!("{:?}", c(&h).view()), view);
    }
    cm(&mut h).set_selected(2).unwrap();
    assert_eq!(h.focused(), Some(editor));
    assert_eq!(h.focused_text_binding(), binding);
    assert_eq!(format!("{:?}", h.focused_text().unwrap()), before);
    assert!(payloads(&mut h).is_empty());
}

#[test]
fn real_native_projection_routes_tokens_and_rejects_stale_mounts() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as A, AccessibilityRole as R, content_accessibility as ax,
    };
    let parent = ComponentId(1);
    let mut h = ContentHost::new(Content::new(
        parent,
        ListContent::new(
            "root",
            vec![Content::new(
                ID,
                SegmentedControlContent::new("seg", labels()),
            )],
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, frame());
    let token = parts(&h)[1].index;
    let old = ax::content_key(&h, ID, Some(token)).unwrap();
    let nodes = ax::content_nodes(&h);
    let root = nodes
        .iter()
        .find(|n| n.key == ax::content_key(&h, ID, None).unwrap())
        .unwrap();
    assert_eq!(root.role, R::RadioGroup);
    let node = nodes.iter().find(|n| n.key == old).unwrap();
    assert_eq!(node.role, R::RadioButton);
    assert!(!node.selected);
    assert_eq!(node.parent, Some(root.key));
    assert!(ax::content_text_layouts(&h).is_empty());
    assert!(!ax::content_action(&mut h, root.key, A::Press).consumed);
    for a in [
        A::SetValue("1".into()),
        A::SetSelection(0..1),
        A::ReplaceSelectedText("x".into()),
    ] {
        assert!(!ax::content_action(&mut h, old, a).consumed);
    }
    assert!(ax::content_action(&mut h, old, A::Focus).consumed);
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(c(&h).model().selected, 0);
    assert!(ax::content_action(&mut h, old, A::Press).activated);
    assert_eq!(payloads(&mut h), vec![1]);
    assert!(
        ax::content_nodes(&h)
            .iter()
            .find(|n| n.key == old)
            .unwrap()
            .selected
    );
    let removed = h.remove(ID).unwrap();
    h.insert(parent, removed).unwrap();
    render(&mut h, &mut t, frame());
    assert!(!ax::content_action(&mut h, old, A::Focus).consumed);
    assert!(!ax::content_action(&mut h, old, A::Press).consumed);
    assert!(payloads(&mut h).is_empty());
    assert_ne!(
        ax::content_key(&h, ID, None).unwrap().component,
        old.component
    );
}

#[test]
fn paint_does_not_measure_again_or_advance_an_extra_frame() {
    use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
    use tekes_ui_scene::Size;
    use tekes_ui_tokens::Rgba;
    struct Count {
        inner: coretext::Text,
        measures: usize,
        draws: usize,
        requests: usize,
    }
    impl ButtonTextRenderer for Count {
        fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
            self.measures += 1;
            self.inner.measure(s, r, w)
        }
        fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
            self.inner.resolved_font_size(r)
        }
        fn draw(
            &mut self,
            s: &mut Scene,
            t: &str,
            r: proposal::TextRole,
            f: Rect,
            a: ButtonTextAlign,
            i: Rgba,
        ) {
            self.draws += 1;
            self.inner.draw(s, t, r, f, a, i)
        }
        fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
            self.inner.motion_context()
        }
        fn request_animation_frame(&mut self) {
            self.requests += 1;
        }
    }
    let mut c = SegmentedControlContent::new("s", labels());
    c.lifecycle(Lifecycle::Mounted);
    let mut text = Count {
        inner: coretext::Text::new(1.),
        measures: 0,
        draws: 0,
        requests: 0,
    };
    text.inner.services = true;
    c.arrange(&mut text, frame(), frame());
    c.presentation_runnable_changed(true);
    let measured = text.measures;
    assert_eq!(measured, 2);
    assert_eq!(text.draws, 0);
    assert_eq!(text.requests, 0);
    let before = format!("{:?}", c.view());
    c.paint(
        &mut Scene::new(),
        &mut text,
        frame(),
        frame(),
        tekes_ui_tokens::Palette::from(Theme::LIGHT).into(),
    );
    assert_eq!(text.measures, measured);
    assert_eq!(text.draws, 2);
    assert_eq!(text.requests, 0);
    // At rest, painting preserves original focused/selected/layout and input arms.
    assert_eq!(c.view().focused(), None);
    assert_eq!(c.model().selected, 0);
    assert!(before.contains("space_armed: None"));
}

#[derive(Debug)]
struct Gate {
    active: bool,
    children: Vec<Content>,
}
impl Component for Gate {
    fn active(&self) -> bool {
        self.active
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        a: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.children[0].measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn tekes_ui_controls::ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
}
#[test]
fn actual_ancestor_scope_withdrawal_does_not_resurrect_token_or_intent() {
    let gate = ComponentId(1);
    let mut h = ContentHost::new(Content::new(
        gate,
        Gate {
            active: true,
            children: vec![Content::new(
                ID,
                SegmentedControlContent::new("seg", labels()),
            )],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, frame());
    let old = parts(&h)[1].index;
    h.accessibility_action(ID, Some(old), AccessibilityAction::Press);
    h.get_mut(gate)
        .unwrap()
        .component_mut::<Gate>()
        .unwrap()
        .active = false;
    render(&mut h, &mut t, frame());
    assert!(
        !h.accessibility_action(ID, Some(old), AccessibilityAction::Focus)
            .consumed
    );
    h.get_mut(gate)
        .unwrap()
        .component_mut::<Gate>()
        .unwrap()
        .active = true;
    render(&mut h, &mut t, frame());
    assert!(payloads(&mut h).is_empty());
    assert_ne!(parts(&h)[1].index, old);
    assert!(
        !h.accessibility_action(ID, Some(old), AccessibilityAction::Press)
            .consumed
    );
}

#[test]
fn focus_transfer_retires_old_space_arm_even_after_focus_aba() {
    for pointer in [false, true] {
        let mut h = host();
        let mut t = coretext::Text::new(1.);
        render(&mut h, &mut t, frame());
        let ps = parts(&h);
        h.accessibility_action(ID, Some(ps[1].index), AccessibilityAction::Focus);
        key(&mut h, ControlKey::Space, true);
        if pointer {
            h.event(&ContentEvent::PointerDown(ps[2].frame.center()));
            h.event(&ContentEvent::Cancel);
        } else {
            h.accessibility_action(ID, Some(ps[2].index), AccessibilityAction::Focus);
        }
        h.accessibility_action(ID, Some(ps[1].index), AccessibilityAction::Focus);
        assert!(!key(&mut h, ControlKey::Space, false).activated);
        assert_eq!(c(&h).model().selected, 0);
        assert!(payloads(&mut h).is_empty());
    }
}

#[test]
fn reading_item_identity_and_real_composer_survive_controlled_theme_updates() {
    use tekes_ui_composition_consumer::scroll_updates::{COMPOSER, ITEMS, SCROLLER, Scenario};
    let mut s = Scenario::reading_document().unwrap();
    s.host
        .insert(
            ITEMS,
            Content::new(
                ID,
                SegmentedControlContent::new("reading segment", labels()),
            ),
        )
        .unwrap();
    let version = s.attach_reading("segment-reading", 1).unwrap();
    let mut t = coretext::Text::new(1.);
    t.services = true;
    let f = Rect::new(0., 0., 600., 420.);
    s.host
        .render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
        .unwrap();
    s.host.focus(Some(COMPOSER), true);
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    // Publish the actual original editor's new marked-text geometry before
    // comparing a later Theme-only render; the preedit event itself does not shape.
    s.host
        .render(&mut Scene::new(), &mut t, f, f, Theme::LIGHT.into())
        .unwrap();
    let mount = s.host.semantic_mount(ID);
    let binding = s.host.focused_text_binding();
    let input = s.host.text_input_snapshot().unwrap();
    let keys = s.item_keys();
    let offset = s
        .host
        .get(SCROLLER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    s.host
        .get_mut(ID)
        .unwrap()
        .component_mut::<SegmentedControlContent>()
        .unwrap()
        .set_selected(2)
        .unwrap();
    s.host
        .render(&mut Scene::new(), &mut t, f, f, Theme::DARK.into())
        .unwrap();
    let scroll = s
        .host
        .get(SCROLLER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert_eq!(scroll.reading_document_version(), Some(&version));
    assert_eq!(scroll.scroll.offset, offset);
    assert_eq!(s.item_keys(), keys);
    assert_eq!(s.host.semantic_mount(ID), mount);
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert_eq!(s.host.focused_text_binding(), binding);
    assert_eq!(s.host.text_input_snapshot(), Some(input));
    assert!(s.host.drain_action_records().is_empty());
}

#[test]
fn pointer_and_space_arms_retire_on_original_host_lifecycles() {
    for pointer in [false, true] {
        for reason in 0..6 {
            let parent = ComponentId(1);
            let mut h = ContentHost::new(Content::new(
                parent,
                ListContent::new(
                    "root",
                    vec![Content::new(
                        ID,
                        SegmentedControlContent::new("seg", labels()),
                    )],
                ),
            ))
            .unwrap();
            h.set_action_events(true);
            let mut t = coretext::Text::new(1.);
            render(&mut h, &mut t, frame());
            let ps = parts(&h);
            let p = ps[1].frame.center();
            if pointer {
                assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
            } else {
                h.accessibility_action(ID, Some(ps[1].index), AccessibilityAction::Focus);
                key(&mut h, ControlKey::Space, true);
            }
            match reason {
                0 => {
                    h.event(&ContentEvent::Cancel);
                }
                1 => {
                    cm(&mut h).set_disabled(true);
                    cm(&mut h).set_disabled(false);
                }
                2 => {
                    h.set_window_active(false);
                    h.set_window_active(true);
                }
                3 => {
                    render(&mut h, &mut t, Rect::new(0., 0., 1., 1.));
                }
                4 => {
                    cm(&mut h).set_item_enabled(1, false).unwrap();
                    cm(&mut h).set_item_enabled(1, true).unwrap();
                }
                _ => {
                    let old = h.remove(ID).unwrap();
                    h.insert(parent, old).unwrap();
                }
            }
            render(&mut h, &mut t, frame());
            let r = if pointer {
                h.event(&ContentEvent::PointerUp(p))
            } else {
                key(&mut h, ControlKey::Space, false)
            };
            assert!(!r.activated, "{pointer} {reason}");
            assert_eq!(c(&h).model().selected, 0);
            assert!(payloads(&mut h).is_empty());
        }
    }
}
