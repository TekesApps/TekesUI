#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::checkbox_content::{CHECKBOX as CHECK, build};
use tekes_ui_controls::{
    CheckState, Checkbox, ControlKey, ScrollArea, ScrollRevealOptions,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_platform_macos::{
    AccessibilityAction as Native,
    content_accessibility::{content_action, content_key},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, surface_palette::surface_names};
fn render(h: &mut ContentHost, t: &mut coretext::Text) {
    paint(
        h,
        t,
        Rect::new(0., 0., 360., 100.),
        Rect::new(0., 0., 500., 400.),
        Theme::LIGHT.into(),
    );
}
fn paint(h: &mut ContentHost, t: &mut coretext::Text, f: Rect, c: Rect, p: Palette) -> Scene {
    t.atlas.begin_frame();
    let mut s = Scene::new();
    h.render(&mut s, t, f, c, p).unwrap();
    s
}
fn check(h: &ContentHost) -> &CheckboxContent {
    h.get(CHECK)
        .unwrap()
        .component::<CheckboxContent>()
        .unwrap()
}
fn check_mut(h: &mut ContentHost) -> &mut CheckboxContent {
    h.get_mut(CHECK)
        .unwrap()
        .component_mut::<CheckboxContent>()
        .unwrap()
}
fn setup(state: CheckState) -> (ContentHost, coretext::Text) {
    let mut h = build("Checkbox · 复选", state).unwrap();
    let mut t = coretext::Text::new(1.);
    t.services = true;
    render(&mut h, &mut t);
    (h, t)
}
fn key(h: &mut ContentHost, k: ControlKey, down: bool) -> ContentResponse {
    h.event(&ContentEvent::Key { key: k, down })
}
fn click(h: &mut ContentHost) -> ContentResponse {
    let p = check(h).layout().label.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.event(&ContentEvent::PointerUp(p))
}
#[test]
fn three_states_original_pointer_space_enter_and_immutable_delivery() {
    for initial in [
        CheckState::Mixed,
        CheckState::Unchecked,
        CheckState::Checked,
    ] {
        let (mut h, mut t) = setup(initial);
        let mut original = Checkbox {
            checked: initial,
            ..Default::default()
        };
        let f = h.get(CHECK).unwrap().frame();
        original.render(
            &mut Scene::new(),
            &mut t,
            "Checkbox · 复选",
            f,
            Some(Rect::new(0., 0., 500., 400.)),
            Theme::LIGHT,
            tekes_ui_tokens::proposal::TextRole::UiBody,
        );
        let p = check(&h).layout().label.center();
        assert_eq!(
            original.pointer_down_at(p),
            h.event(&ContentEvent::PointerDown(p)).consumed
        );
        assert_eq!(
            original.pointer_up_at(p),
            h.event(&ContentEvent::PointerUp(p)).activated
        );
        assert_eq!(check(&h).model().checked, original.checked);
        let delivered = original.checked;
        check_mut(&mut h).set_checked(initial);
        h.event(&ContentEvent::Cancel);
        let records = h.drain_action_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action.source, ContentActionSource::Pointer);
        assert!(
            matches!(&records[0].payload,Some(ContentActionPayload::CheckboxState(i)) if i.checked()==delivered)
        );
        assert!(h.drain_actions().is_empty());
        h.focus(Some(CHECK), true);
        original.checked = initial;
        original.button.state.focused = true;
        for (k, down) in [
            (ControlKey::Enter, true),
            (ControlKey::Enter, false),
            (ControlKey::Space, true),
            (ControlKey::Space, true),
            (ControlKey::Space, false),
            (ControlKey::Space, false),
        ] {
            let before = check(&h).model().checked;
            let actual = key(&mut h, k, down);
            let expected = original.key(k, down);
            // Restore the controlled value in the independent original owner before comparing next activation.
            if expected {
                assert_ne!(before, check(&h).model().checked);
            }
            assert_eq!(actual.activated, expected);
            assert_eq!(
                check(&h).model().checked,
                original.checked,
                "Space retains the original three-state result"
            );
        }
        assert_eq!(h.drain_actions().len(), 1);
    }
}
#[test]
fn setters_cancel_only_changed_value_noop_keeps_arm_and_cancel_keeps_committed_intent() {
    for space in [false, true] {
        let (mut h, _) = setup(CheckState::Mixed);
        let p = check(&h).layout().label.center();
        h.focus(Some(CHECK), true);
        if space {
            key(&mut h, ControlKey::Space, true);
        } else {
            h.event(&ContentEvent::PointerDown(p));
        }
        check_mut(&mut h).set_checked(CheckState::Mixed);
        let r = if space {
            key(&mut h, ControlKey::Space, false)
        } else {
            h.event(&ContentEvent::PointerUp(p))
        };
        assert!(r.activated);
        check_mut(&mut h).set_checked(CheckState::Mixed);
        if space {
            key(&mut h, ControlKey::Space, true);
        } else {
            h.event(&ContentEvent::PointerDown(p));
        }
        check_mut(&mut h).set_checked(CheckState::Unchecked);
        let r = if space {
            key(&mut h, ControlKey::Space, false)
        } else {
            h.event(&ContentEvent::PointerUp(p))
        };
        assert!(!r.activated);
        assert_eq!(check(&h).model().checked, CheckState::Unchecked);
        h.event(&ContentEvent::Cancel);
        let records = h.drain_action_records();
        assert_eq!(records.len(), 1);
        assert!(
            matches!(&records[0].payload,Some(ContentActionPayload::CheckboxState(i)) if i.checked()==CheckState::Checked)
        );
    }
}
#[test]
fn exposure_renewal_rejects_old_native_capture_focus_return_and_intent_without_replacing_owner() {
    for mode in ["disabled", "loading", "inactive", "clip"] {
        let (mut h, mut t) = setup(CheckState::Mixed);
        let old = content_key(&h, CHECK, None).unwrap();
        let ptr = check(&h).model() as *const Checkbox;
        h.focus(Some(CHECK), true);
        let ret = h.capture_focus();
        assert!(click(&mut h).activated);
        let p = check(&h).layout().label.center();
        h.event(&ContentEvent::PointerDown(p));
        match mode {
            "disabled" => check_mut(&mut h).set_availability(true, false),
            "loading" => check_mut(&mut h).set_availability(false, true),
            "inactive" => h.set_window_active(false),
            _ => {
                paint(
                    &mut h,
                    &mut t,
                    Rect::new(0., 0., 360., 100.),
                    Rect::new(450., 200., 10., 10.),
                    Theme::LIGHT.into(),
                );
            }
        }
        assert!(!content_action(&mut h, old, Native::Press).activated);
        assert!(h.pointer_capture().is_none());
        match mode {
            "disabled" | "loading" => check_mut(&mut h).set_availability(false, false),
            "inactive" => h.set_window_active(true),
            _ => {}
        }
        assert!(h.pointer_capture().is_none(), "restoring availability cannot republish a revoked capture lease before arrangement");
        render(&mut h, &mut t);
        let new = content_key(&h, CHECK, None).unwrap();
        assert_ne!(old, new);
        assert_eq!(check(&h).model() as *const Checkbox, ptr);
        assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
        assert!(!content_action(&mut h, old, Native::Press).activated);
        h.restore_focus(ret);
        assert_ne!(h.focused(), Some(CHECK));
        assert!(h.drain_actions().is_empty());
        assert!(content_action(&mut h, new, Native::Press).activated);
        assert_eq!(h.drain_actions().len(), 1);
    }
}
#[test]
fn continuous_value_theme_move_and_invalid_surface_preserve_original_identity() {
    let (mut h, mut t) = setup(CheckState::Mixed);
    let old = content_key(&h, CHECK, None).unwrap();
    let p = check(&h).layout().label.center();
    h.event(&ContentEvent::PointerDown(p));
    let before = format!("{:?}", check(&h));
    assert!(
        h.get_mut(CHECK)
            .unwrap()
            .set_surface(Some("invalid-surface"))
            .is_err()
    );
    assert_eq!(format!("{:?}", check(&h)), before);
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    let delivered = check(&h).model().checked;
    for theme in Theme::ALL {
        check_mut(&mut h).set_checked(CheckState::Mixed);
        paint(
            &mut h,
            &mut t,
            Rect::new(12., 20., 240., 80.),
            Rect::new(0., 0., 500., 400.),
            theme.into(),
        );
        assert_eq!(content_key(&h, CHECK, None).unwrap(), old);
    }
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    assert!(
        matches!(&r[0].payload,Some(ContentActionPayload::CheckboxState(i)) if i.checked()==delivered)
    );
}
#[test]
fn original_measure_arrange_and_paint_match_all_states_surfaces_scales_and_clips() {
    for theme in Theme::ALL {
        for surface in surface_names() {
            for scale in [1., 1.5, 2.] {
                for state in [
                    CheckState::Unchecked,
                    CheckState::Checked,
                    CheckState::Mixed,
                ] {
                    for label in ["", "Long label 第一行对齐 and wrapping"] {
                        for width in [8., 82.5, 360.] {
                            let mut h = build(label, state).unwrap();
                            h.set_inherited_surface(surface).unwrap();
                            let mut t = coretext::Text::new(scale);
                            t.services = true;
                            let clip = Rect::new(10., 0., 320., 300.);
                            let actual = paint(
                                &mut h,
                                &mut t,
                                Rect::new(0., 0., width, 20.),
                                clip,
                                theme.into(),
                            );
                            let mut original = Checkbox {
                                checked: state,
                                ..Default::default()
                            };
                            let mut expected = Scene::new();
                            let layout = original
                                .render_on_surface(
                                    &mut expected,
                                    &mut t,
                                    label,
                                    h.get(CHECK).unwrap().frame(),
                                    Some(clip),
                                    theme,
                                    tekes_ui_tokens::proposal::TextRole::UiBody,
                                    surface,
                                )
                                .unwrap();
                            assert_eq!(check(&h).layout(), layout);
                            assert_eq!(check(&h).model().layout(), original.layout());
                            let quads = |s: &Scene| {
                                s.primitives()
                                    .iter()
                                    .filter_map(|p| {
                                        if let Primitive::Quad(q) = p {
                                            Some(*q)
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            };
                            assert_eq!(quads(&actual), quads(&expected));
                            let glyphs = |s: &Scene| {
                                s.primitives()
                                    .iter()
                                    .filter_map(|p| {
                                        if let Primitive::Glyphs(g) = p {
                                            Some((g.color, g.glyphs.len()))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            };
                            assert_eq!(glyphs(&actual), glyphs(&expected));
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn unmount_same_id_remount_and_original_constructor_retire_gesture_and_payload() {
    let mut h = ContentHost::new(Content::new(
        ComponentId(10800),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![Content::new(
                CHECK,
                CheckboxContent::new("Check", CheckState::Mixed),
            )],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    render(&mut h, &mut t);
    let old = content_key(&h, CHECK, None).unwrap();
    assert!(click(&mut h).activated);
    let removed = h.remove(CHECK).unwrap();
    h.insert(ComponentId(10800), removed).unwrap();
    render(&mut h, &mut t);
    assert_ne!(content_key(&h, CHECK, None).unwrap(), old);
    assert!(!content_action(&mut h, old, Native::Press).activated);
    assert!(h.drain_actions().is_empty());
    let mut original = Checkbox::default();
    original.render(
        &mut Scene::new(),
        &mut t,
        "old",
        Rect::new(0., 0., 300., 50.),
        None,
        Theme::LIGHT,
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    original.pointer_down_at(Point::new(4., 4.));
    let content = CheckboxContent::from_original("", original);
    assert!(content.model().layout().is_none());
    assert!(!content.model().button.state.pressed);
}
const ROOT: ComponentId = ComponentId(10820);
const SCROLL: ComponentId = ComponentId(10821);
const EDITOR: ComponentId = ComponentId(10822);
const ITEMS: ComponentId = ComponentId(10823);
fn mixed() -> (
    ContentHost,
    coretext::Text,
    tekes_ui_controls::ReadingDocumentVersion,
) {
    let rows = Content::new(
        ITEMS,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: (0..2)
                .map(|i| Content::new(ComponentId(10830 + i), TextContent::new(format!("row {i}"))))
                .chain([Content::new(
                    CHECK,
                    CheckboxContent::new("Check", CheckState::Mixed),
                )])
                .chain((2..12).map(|i| {
                    Content::new(ComponentId(10830 + i), TextContent::new(format!("row {i}")))
                }))
                .collect(),
        },
    );
    let mut scroll = ScrollContent::new(ScrollArea::default(), rows);
    scroll.viewport_height = Some(170.);
    let mut h = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(SCROLL, scroll),
                Content::new(EDITOR, TextFieldContent::new("Composer", "kept draft 😀")),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    let (v, ()) = h
        .set_scroll_reading_document(SCROLL, "checkbox-reader", 1, vec![CHECK], |_| {})
        .unwrap();
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    h.reveal_scroll_reading_item(
        SCROLL,
        &v,
        CHECK,
        ScrollRevealOptions {
            animated: false,
            ..Default::default()
        },
        &mut t,
    )
    .unwrap();
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    (h, t, v)
}
#[test]
fn registered_checkbox_requires_original_caller_revision_after_real_exposure_renewal() {
    let (mut h, mut t, v) = mixed();
    let position = h.get(SCROLL).unwrap().frame().center();
    assert!(
        h.event(&ContentEvent::Scroll {
            position,
            delta: Point::new(0., 15.25),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    assert!(matches!(
        h.get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_result()
            .unwrap()
            .anchor,
        ContentReadingAnchorUpdate::Preserved(CHECK)
    ));
    let old = content_key(&h, CHECK, None).unwrap();
    let stable_offset = h
        .get(SCROLL)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    check_mut(&mut h).set_checked(CheckState::Checked);
    paint(
        &mut h,
        &mut t,
        Rect::new(12., 16., 400., 400.),
        Rect::new(0., 0., 500., 500.),
        Theme::DARK.into(),
    );
    assert_eq!(content_key(&h, CHECK, None), Some(old));
    assert!(
        h.reveal_scroll_reading_item(SCROLL, &v, CHECK, Default::default(), &mut t)
            .is_ok(),
        "continuous value/Theme/position changes retain the real registered item"
    );
    assert_eq!(
        h.get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        stable_offset
    );
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    assert_eq!(content_key(&h, CHECK, None), Some(old));

    let scroll = |h: &ContentHost| {
        h.get(SCROLL)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
    };
    let offset = scroll(&h);
    let plan = h
        .plan_host_reveal(CHECK, Default::default(), &[], HostRevealBudget::default())
        .unwrap();
    h.set_window_active(false);
    h.set_window_active(true);
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    assert_ne!(content_key(&h, CHECK, None).unwrap(), old);
    assert!(!content_action(&mut h, old, Native::Press).activated);
    assert!(h.commit_host_reveal(plan).is_err());
    assert_eq!(
        h.reveal_scroll_reading_item(SCROLL, &v, CHECK, Default::default(), &mut t),
        Err(ContentRevealError::TargetReplaced)
    );
    assert_eq!(scroll(&h), offset);
    let (next, ()) = h
        .update_scroll_reading_document(SCROLL, &v, 2, vec![CHECK], |_| {})
        .unwrap();
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    assert_eq!(scroll(&h), offset);
    assert!(
        h.reveal_scroll_reading_item(SCROLL, &next, CHECK, Default::default(), &mut t)
            .is_ok()
    );
    assert!(
        h.reveal_scroll_reading_item(SCROLL, &v, CHECK, Default::default(), &mut t)
            .is_err()
    );
}
#[test]
fn actual_composer_marked_binding_selection_barrier_and_local_updates() {
    let (mut h, mut t, _) = mixed();
    assert!(h.focus(Some(EDITOR), true));
    h.event_with_text(&ContentEvent::TextCommand("selectAll:".into()), &mut t)
        .unwrap();
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        },
        &mut t,
    )
    .unwrap();
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::LIGHT.into(),
    );
    let original_input = format!("{:?}", h.text_surface(EDITOR).unwrap().input);
    let snap = h.text_input_snapshot().unwrap();
    let binding = h.focused_text_binding();
    let check_before = format!("{:?}", check(&h));
    let target = content_key(&h, CHECK, None).unwrap();
    assert!(!content_action(&mut h, target, Native::Press).activated);
    assert!(!content_action(&mut h, target, Native::Focus).consumed);
    assert_eq!(format!("{:?}", check(&h)), check_before);
    check_mut(&mut h).set_checked(CheckState::Checked);
    check_mut(&mut h).set_availability(true, false);
    check_mut(&mut h).set_availability(false, false);
    paint(
        &mut h,
        &mut t,
        Rect::new(0., 0., 400., 400.),
        Rect::new(0., 0., 400., 400.),
        Theme::DARK.into(),
    );
    assert_eq!(
        format!("{:?}", h.text_surface(EDITOR).unwrap().input),
        original_input,
        "local Checkbox updates preserve original undo/draft/marked/selection owner"
    );
    assert_eq!(h.text_input_snapshot(), Some(snap));
    assert_eq!(h.focused_text_binding(), binding);
    assert_eq!(h.focused(), Some(EDITOR));
}

#[test]
fn original_drag_cancel_noop_availability_and_multiple_delivery_values() {
    let (mut h, mut t) = setup(CheckState::Unchecked);
    let p = check(&h).layout().label.center();
    h.event(&ContentEvent::PointerDown(p));
    check_mut(&mut h).set_availability(false, false);
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    h.drain_actions();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerMove(Point::new(-40., -40.)));
    assert!(
        !h.event(&ContentEvent::PointerUp(Point::new(-40., -40.)))
            .activated
    );
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::Cancel);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_actions().is_empty());
    h.focus(Some(CHECK), true);
    key(&mut h, ControlKey::Space, true);
    h.focus(None, false);
    assert!(!key(&mut h, ControlKey::Space, false).activated);
    check_mut(&mut h).set_checked(CheckState::Unchecked);
    render(&mut h, &mut t);
    let target = content_key(&h, CHECK, None).unwrap();
    assert!(content_action(&mut h, target, Native::Press).activated);
    assert!(content_action(&mut h, target, Native::Press).activated);
    check_mut(&mut h).set_checked(CheckState::Mixed);
    let records = h.drain_action_records();
    let values = records
        .iter()
        .map(|r| match r.payload.as_ref().unwrap() {
            ContentActionPayload::CheckboxState(i) => i.checked(),
            _ => panic!("Checkbox payload"),
        })
        .collect::<Vec<_>>();
    assert_eq!(values, vec![CheckState::Checked, CheckState::Unchecked]);
}

#[test]
fn checkbox_modal_scope_open_close_without_paint_retires_old_exposure_and_queue() {
    let mut h = ContentHost::new(Content::new(
        ComponentId(10800),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(CHECK, CheckboxContent::new("Check", CheckState::Mixed)),
                Content::new(
                    ComponentId(10890),
                    DialogContent::new(
                        "Dialog",
                        Content::new(ComponentId(10891), TextContent::new("Header")),
                        Content::new(ComponentId(10892), Button::new("Body")),
                        Content::new(ComponentId(10893), Button::new("Close")),
                    ),
                ),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    render(&mut h, &mut t);
    let old = content_key(&h, CHECK, None).unwrap();
    h.focus(Some(CHECK), true);
    let ret = h.capture_focus();
    assert!(content_action(&mut h, old, Native::Press).activated);
    h.show_dialog(ComponentId(10890)).unwrap();
    h.close_dialog();
    assert!(
        !content_action(&mut h, old, Native::Press).activated,
        "closing a modal cannot revive the old root exposure even without an intermediate layout"
    );
    render(&mut h, &mut t);
    assert_ne!(content_key(&h, CHECK, None).unwrap(), old);
    assert!(h.drain_actions().is_empty());
    h.restore_focus(ret);
    assert_ne!(h.focused(), Some(CHECK));
}

#[test]
fn empty_duplicate_labels_keep_sibling_identity_and_unrelated_delivered_intent() {
    let other = ComponentId(10811);
    let mut h = ContentHost::new(Content::new(
        ComponentId(10800),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(CHECK, CheckboxContent::new("", CheckState::Mixed)),
                Content::new(other, CheckboxContent::new("", CheckState::Unchecked)),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    render(&mut h, &mut t);
    let a = content_key(&h, CHECK, None).unwrap();
    let b = content_key(&h, other, None).unwrap();
    assert_ne!(a, b);
    assert!(content_action(&mut h, a, Native::Press).activated);
    assert!(content_action(&mut h, b, Native::Press).activated);
    check_mut(&mut h).set_availability(true, false);
    check_mut(&mut h).set_availability(false, false);
    render(&mut h, &mut t);
    assert_ne!(content_key(&h, CHECK, None).unwrap(), a);
    assert_eq!(content_key(&h, other, None).unwrap(), b);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, other);
    assert!(
        matches!(&records[0].payload,Some(ContentActionPayload::CheckboxState(i)) if i.checked()==CheckState::Checked)
    );
}
#[test]
fn checkbox_original_popover_scope_retires_background_exposure_without_intermediate_layout() {
    for paint_open in [false, true] {
        let popup = ComponentId(10894);
        let mut h = ContentHost::new(Content::new(
            ComponentId(10800),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![
                    Content::new(CHECK, CheckboxContent::new("Anchor", CheckState::Mixed)),
                    Content::new(
                        popup,
                        PopoverContent::new(
                            "Original popup",
                            Content::new(ComponentId(10895), Button::new("Save")),
                        ),
                    ),
                ],
            },
        ))
        .unwrap();
        let mut t = coretext::Text::new(1.);
        t.services = true;
        render(&mut h, &mut t);
        let old = content_key(&h, CHECK, None).unwrap();
        let session = h.show_popover(popup, CHECK).unwrap();
        if paint_open {
            render(&mut h, &mut t);
            assert!(
                h.get(popup)
                    .unwrap()
                    .component::<PopoverContent>()
                    .unwrap()
                    .is_open(),
                "retiring Checkbox callbacks must not disable the original Popover anchor"
            );
        }
        assert!(h.close_popover(session));
        assert!(!content_action(&mut h, old, Native::Press).activated);
        render(&mut h, &mut t);
        assert_ne!(content_key(&h, CHECK, None).unwrap(), old);
    }
}

#[test]
fn checkbox_capture_availability_aba_without_host_observation_rejects_original_lease() {
    for loading in [false, true] {
        let (mut h, mut t) = setup(CheckState::Mixed);
        let old = content_key(&h, CHECK, None).unwrap();
        let p = check(&h).layout().label.center();
        assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
        check_mut(&mut h).set_availability(!loading, loading);
        check_mut(&mut h).set_availability(false, false);
        assert!(
            h.pointer_capture().is_none(),
            "a revoked root exposure must not report its old capture after immediate availability restoration"
        );
        assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
        render(&mut h, &mut t);
        assert_ne!(content_key(&h, CHECK, None).unwrap(), old);
        assert!(!content_action(&mut h, old, Native::Press).activated);
    }
}
