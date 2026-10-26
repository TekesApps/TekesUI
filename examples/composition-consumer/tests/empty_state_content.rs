#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::{
    empty_state_content::{self as fixture, ACTION, MESSAGE},
    scroll_updates::{COMPOSER, LATEST, ROOT, SCROLLER},
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*, extended::EmptyState,
    feedback_view::EmptyStateView,
};
use tekes_ui_platform_macos::{
    AccessibilityAction as NativeAction,
    content_accessibility::{content_action, content_key},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Rgba, Theme, proposal,
    surface_palette::{resolve_surface, surface_names},
};
fn single(action: Option<&str>) -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        MESSAGE,
        EmptyStateContent::new("Documents", fixture::model(action), ACTION).unwrap(),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, p: Palette) -> Scene {
    t.services = true;
    t.atlas.begin_frame();
    let mut s = Scene::new();
    let f = Rect::new(0.25, 0.5, 360., 720.);
    h.render(&mut s, t, f, f, p).unwrap();
    s
}
fn action_frame(h: &ContentHost) -> Rect {
    h.semantics()
        .into_iter()
        .find(|n| n.id == ACTION)
        .unwrap()
        .frame
}
#[test]
fn original_view_content_geometry_packets_and_action_owner() {
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            for surface in surface_names() {
                for (w, clip) in [
                    (360., Rect::new(0., 0., 500., 500.)),
                    (82.5, Rect::new(10., 20., 65., 180.)),
                    (0., Rect::new(0., 0., 500., 500.)),
                    (360., Rect::new(500., 500., 10., 10.)),
                ] {
                    let palette = Palette::new(theme, brand).unwrap();
                    let mut h = single(Some("Create 文档"));
                    h.set_inherited_surface(surface).unwrap();
                    let mut t = coretext::Text::new(1.);
                    let f = Rect::new(4.25, 8.5, w, 260.);
                    t.atlas.begin_frame();
                    let mut actual = Scene::new();
                    h.render(&mut actual, &mut t, f, clip, palette).unwrap();
                    let actual_frame = h.get(MESSAGE).unwrap().frame();
                    let mut v = EmptyStateView::default();
                    v.set_surface(surface).unwrap();
                    let mut expected = Scene::new();
                    v.render(
                        &mut expected,
                        &mut t,
                        &fixture::model(Some("Create 文档")),
                        actual_frame,
                        Some(clip),
                        palette,
                        proposal::TextRole::UiBody,
                    );
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
                    if let Some(target) = v.logical_focus_target() {
                        assert_eq!(h.get(ACTION).unwrap().frame(), target);
                    }
                    assert!(!h.focus(Some(MESSAGE), true));
                    assert!(
                        !h.accessibility_action(MESSAGE, None, AccessibilityAction::Press)
                            .activated
                    );
                    let packet = resolve_surface(palette, surface, "content", "rest").unwrap();
                    for p in actual.primitives() {
                        if let Primitive::Glyphs(run) = p {
                            assert!(
                                run.color == packet.text
                                    || run.color
                                        == resolve_surface(
                                            palette,
                                            surface,
                                            "action-primary",
                                            "rest"
                                        )
                                        .unwrap()
                                        .text
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn typed_updates_invalidate_old_native_keys_and_queued_actions_once() {
    let mut h = single(Some("Create"));
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let root = h.semantic_mount(MESSAGE);
    let old = content_key(&h, ACTION, None).unwrap();
    let p = action_frame(&h).center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.replace_empty_state(MESSAGE, fixture::model(Some("Open")))
        .unwrap();
    assert!(h.pointer_capture().is_none());
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(!content_action(&mut h, old, NativeAction::Press).activated);
    assert!(h.semantic_mount(ACTION).is_none());
    render(&mut h, &mut t, Theme::LIGHT.into());
    let new = content_key(&h, ACTION, None).unwrap();
    assert_ne!(new, old);
    assert_eq!(h.semantic_mount(MESSAGE), root);
    assert!(!content_action(&mut h, old, NativeAction::Press).activated);
    assert!(content_action(&mut h, new, NativeAction::Press).activated);
    h.replace_empty_state(MESSAGE, fixture::model(Some("Run")))
        .unwrap();
    assert!(h.drain_actions().is_empty());
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert!(h.focus(Some(ACTION), true));
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: true
        })
        .activated
    );
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Space,
            down: false
        })
        .activated
    );
    let a = h.drain_actions();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].target, ACTION);
    assert_eq!(a[0].kind, ContentActionKind::Activate);
    assert!(h.drain_actions().is_empty());
    let current = content_key(&h, ACTION, None).unwrap();
    h.set_window_active(false);
    h.set_window_active(true);
    assert!(!content_action(&mut h, current, NativeAction::Press).activated);
    assert!(!h.focus(Some(ACTION), true));
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert_ne!(content_key(&h, ACTION, None).unwrap(), current);
    assert!(!content_action(&mut h, current, NativeAction::Press).activated);
    h.set_empty_state_disabled(MESSAGE, true).unwrap();
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert!(h.semantic_mount(ACTION).is_none());
    h.set_empty_state_disabled(MESSAGE, false).unwrap();
    assert!(h.semantic_mount(ACTION).is_none());
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert!(content_key(&h, ACTION, None).is_some());
    h.replace_empty_state(MESSAGE, fixture::model(None))
        .unwrap();
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.semantics.role == ContentRole::Button)
            .count(),
        0
    );
    h.replace_empty_state(MESSAGE, fixture::model(Some("Create")))
        .unwrap();
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert_eq!(
        h.semantics()
            .iter()
            .filter(|n| n.semantics.role == ContentRole::Button)
            .count(),
        1
    );
    for a in [
        AccessibilityAction::SetValue("1".into()),
        AccessibilityAction::Increment,
        AccessibilityAction::Decrement,
    ] {
        assert_eq!(
            h.accessibility_action(ACTION, None, a),
            ContentResponse::default()
        );
    }
    assert_eq!(
        h.accessibility_action(ACTION, Some(1), AccessibilityAction::Press),
        ContentResponse::default()
    );
}
#[test]
fn metadata_surface_allocation_and_model_preflight_are_atomic() {
    let mut h = single(Some("Create"));
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let old = content_key(&h, ACTION, None).unwrap();
    let p = action_frame(&h).center();
    h.event(&ContentEvent::PointerDown(p));
    let capture = h.pointer_capture();
    let model = h
        .get(MESSAGE)
        .unwrap()
        .component::<EmptyStateContent>()
        .unwrap()
        .model()
        .clone();
    assert!(
        h.replace_empty_state(MESSAGE, fixture::model(Some("  ")))
            .is_err()
    );
    assert!(
        h.replace_empty_state(ComponentId(0), model.clone())
            .is_err()
    );
    assert!(h.replace_empty_state(ACTION, model.clone()).is_err());
    assert_eq!(
        *h.get(MESSAGE)
            .unwrap()
            .component::<EmptyStateContent>()
            .unwrap()
            .model(),
        model
    );
    assert_eq!(h.pointer_capture(), capture);
    assert_eq!(content_key(&h, ACTION, None), Some(old));
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    h.drain_actions();
    let e = h
        .get_mut(MESSAGE)
        .unwrap()
        .component_mut::<EmptyStateContent>()
        .unwrap();
    assert!(e.set_metadata(" ", Some("valid".into())).is_err());
    assert!(e.set_metadata("Valid", Some(" ".into())).is_err());
    assert!(
        h.get_mut(MESSAGE)
            .unwrap()
            .set_surface(Some("unknown"))
            .is_err()
    );
    assert!(h.set_inherited_surface("unknown").is_err());
    assert_eq!(h.inherited_surface(), "canvas");
    let f = Rect::new(0., 0., 100., 200.);
    let mut allocation = (f, f);
    for (frame, clip) in [
        (Rect::new(f32::NAN, 0., 1., 1.), f),
        (Rect::new(0., 0., -1., 1.), f),
        (f, Rect::new(0., 0., f32::INFINITY, 1.)),
    ] {
        assert!(!fixture::set_allocation(&mut allocation, frame, clip));
        assert_eq!(allocation, (f, f));
    }
    assert!(fixture::set_allocation(
        &mut allocation,
        Rect::default(),
        Rect::default()
    ));
    assert!(EmptyStateContent::new(" ", fixture::model(None), ACTION).is_err());
    assert!(
        EmptyStateContent::new(
            "Valid",
            EmptyState {
                title: "".into(),
                detail: "".into(),
                action: None
            },
            ACTION
        )
        .is_ok()
    );
    assert!(
        ContentHost::new(Content::new(
            ACTION,
            EmptyStateContent::new("Valid", fixture::model(None), ACTION).unwrap()
        ))
        .is_err()
    );
}
#[test]
fn children_mut_damage_is_fail_closed_without_panics_or_actions() {
    for kind in 0..4 {
        let mut h = single(Some("Create"));
        let mut t = coretext::Text::new(1.);
        render(&mut h, &mut t, Theme::LIGHT.into());
        let key = content_key(&h, ACTION, None).unwrap();
        let c = h
            .get_mut(MESSAGE)
            .unwrap()
            .component_mut::<EmptyStateContent>()
            .unwrap()
            .children_mut()
            .unwrap();
        match kind {
            0 => c.clear(),
            1 => c.push(Content::new(ComponentId(99), ButtonContent::new("Foreign"))),
            2 => c[0] = Content::new(ACTION, ButtonContent::new("Replaced")),
            _ => {
                let mut other =
                    EmptyStateContent::new("Other", fixture::model(Some("Other")), ACTION).unwrap();
                c[0] = other.children_mut().unwrap().remove(0);
            }
        }
        assert!(!h.focus(Some(ACTION), true));
        assert!(!content_action(&mut h, key, NativeAction::Press).activated);
        assert!(
            !h.event(&ContentEvent::PointerDown(Point::new(30., 120.)))
                .activated
        );
        assert!(
            h.replace_empty_state(MESSAGE, fixture::model(Some("Next")))
                .is_err()
        );
        let mut scene = Scene::new();
        let f = Rect::new(0., 0., 300., 300.);
        assert!(
            h.render(&mut scene, &mut t, f, f, Theme::LIGHT.into())
                .is_err()
        );
        assert!(scene.is_empty());
        assert!(h.drain_actions().is_empty());
    }
}
#[test]
fn paint_cannot_remeasure_or_move_the_prepared_hit_frame() {
    struct Unstable {
        count: usize,
        forbid: bool,
    }
    impl ButtonTextRenderer for Unstable {
        fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
            assert!(!self.forbid, "paint remeasured");
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
    let mut text = Unstable {
        count: 0,
        forbid: false,
    };
    let model = fixture::model(Some("Create"));
    let mut view = EmptyStateView::default();
    let f = Rect::new(0.25, 0.5, 320., 240.);
    view.prepare(&mut text, &model, f, Some(f), proposal::TextRole::UiBody);
    let full = view.logical_focus_target();
    let visible = view.focus_target();
    let count = text.count;
    text.forbid = true;
    view.paint_prepared(&mut Scene::new(), &mut text, &model, Theme::LIGHT.into());
    assert_eq!(text.count, count);
    assert_eq!(view.logical_focus_target(), full);
    assert_eq!(view.focus_target(), visible);
    assert!(view.pointer_down(full.unwrap().center()));
    assert!(view.pointer_up(full.unwrap().center()));
}
#[test]
fn mixed_original_host_preserves_composer_and_reading_through_updates() {
    mixed(false);
}
#[test]
#[ignore = "explicit independently locked external Theme lane"]
fn external_mixed_original_host() {
    mixed(true);
}
fn mixed(external: bool) {
    let (mut s, mut version) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    let editor = s.host.semantic_mount(COMPOSER);
    let root = s.host.semantic_mount(MESSAGE);
    let keys = s.item_keys();
    let items: Vec<_> = keys.iter().map(|id| s.host.semantic_mount(*id)).collect();
    let input = format!("{:?}", s.host.focused_text().unwrap().input);
    s.host.drain_actions();
    for theme in Theme::ALL {
        for surface in surface_names() {
            let palettes = if external {
                vec![
                    Palette::for_theme_definition(theme, "blue").unwrap(),
                    Palette::for_theme_definition(theme, "blue-inputs").unwrap(),
                ]
            } else {
                vec![Palette::new(theme, 2).unwrap()]
            };
            for palette in palettes {
                for action in [Some("Create"), None, Some("Open 文档")] {
                    let (v, ()) = s
                        .host
                        .update_scroll_reading_document(
                            SCROLLER,
                            &version,
                            version.revision() + 1,
                            keys.clone(),
                            |h| {
                                h.replace_empty_state(MESSAGE, fixture::model(action))
                                    .unwrap();
                                h.set_inherited_surface(surface).unwrap();
                            },
                        )
                        .unwrap();
                    version = v;
                    render(&mut s.host, &mut t, palette);
                    let after = s.host.text_input_snapshot().unwrap();
                    assert_eq!(after.binding, before.binding);
                    assert_eq!(after.text, before.text);
                    assert_eq!(after.selection, before.selection);
                    assert_eq!(after.marked, before.marked);
                    assert_eq!(s.host.focused(), Some(COMPOSER));
                    assert_eq!(s.host.focused_text_binding(), binding);
                    assert_eq!(s.host.focus_epoch(), epoch);
                    assert_eq!(s.host.semantic_mount(COMPOSER), editor);
                    assert_eq!(s.host.semantic_mount(MESSAGE), root);
                    assert_eq!(
                        keys.iter()
                            .map(|id| s.host.semantic_mount(*id))
                            .collect::<Vec<_>>(),
                        items
                    );
                    assert_eq!(format!("{:?}", s.host.focused_text().unwrap().input), input);
                    s.host.event(&ContentEvent::Key {
                        key: ControlKey::Enter,
                        down: true,
                    });
                    assert!(s.host.drain_actions().is_empty());
                }
            }
        }
    }
    // Retained instances reorder without inventing a remount or editor update.
    s.host
        .reorder(ROOT, &[SCROLLER, LATEST, MESSAGE, COMPOSER])
        .unwrap();
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert_eq!(s.host.focused_text_binding(), binding);
    assert_eq!(s.host.semantic_mount(COMPOSER), editor);
}

#[test]
fn passive_body_and_theme_keep_original_space_arm_focus_and_mount() {
    let mut h = single(Some("Create"));
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let old = content_key(&h, ACTION, None).unwrap();
    h.focus(Some(ACTION), true);
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    let mut m = fixture::model(Some("Create"));
    m.title = "Changed title".into();
    h.replace_empty_state(MESSAGE, m).unwrap();
    h.set_inherited_surface("panel").unwrap();
    render(&mut h, &mut t, Theme::DARK.into());
    assert_eq!(content_key(&h, ACTION, None), Some(old));
    assert_eq!(h.focused(), Some(ACTION));
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
fn mixed_narrow_clip_and_explicit_action_focus_policies() {
    let (mut s, _) = fixture::build().unwrap();
    let mut t = coretext::Text::new(1.);
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut t, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let keys = s.item_keys();
    let mounts: Vec<_> = keys.iter().map(|id| s.host.semantic_mount(*id)).collect();
    for (frame, clip) in [
        (
            Rect::new(0.25, 0.5, 240., 520.),
            Rect::new(30., 40., 180., 430.),
        ),
        (
            Rect::new(0.25, 0.5, 240., 520.),
            Rect::new(800., 800., 10., 10.),
        ),
        (
            Rect::new(0.25, 0.5, 360., 720.),
            Rect::new(0., 0., 400., 740.),
        ),
    ] {
        let mut scene = Scene::new();
        t.atlas.begin_frame();
        s.host
            .render(&mut scene, &mut t, frame, clip, Theme::DARK.into())
            .unwrap();
        let after = s.host.text_input_snapshot().unwrap();
        assert_eq!(after.text, before.text);
        assert_eq!(after.selection, before.selection);
        assert_eq!(after.marked, before.marked);
        assert_eq!(after.binding, before.binding);
        assert_eq!(s.host.focused(), Some(COMPOSER));
        assert_eq!(s.host.focused_text_binding(), binding);
        assert_eq!(
            keys.iter()
                .map(|id| s.host.semantic_mount(*id))
                .collect::<Vec<_>>(),
            mounts
        );
    }
    // The original transfer barrier protects marked text for an ordinary action.
    assert!(
        !s.host
            .accessibility_action(ACTION, None, AccessibilityAction::Press)
            .activated
    );
    // Configure the original proxy Content policy without replacing its owner.
    let empty = s
        .host
        .get_mut(MESSAGE)
        .unwrap()
        .component_mut::<EmptyStateContent>()
        .unwrap();
    let child = empty.children_mut().unwrap().remove(0).preserving_focus();
    empty.children_mut().unwrap().push(child);
    let current = content_key(&s.host, ACTION, None).unwrap();
    assert!(content_action(&mut s.host, current, NativeAction::Press).activated);
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert_eq!(s.host.text_input_snapshot().unwrap().marked, before.marked);
    assert_eq!(s.host.drain_actions().len(), 1);
    s.host.event(&ContentEvent::Unmark);
    assert!(s.host.focus(Some(ACTION), true));
    assert_eq!(s.host.focused(), Some(ACTION));
}
#[test]
fn pointer_cancel_clipped_restore_and_reorder_preserve_original_delivery_contract() {
    let mut h = single(Some("Create"));
    let mut t = coretext::Text::new(1.);
    render(&mut h, &mut t, Theme::LIGHT.into());
    let key = content_key(&h, ACTION, None).unwrap();
    let p = action_frame(&h).center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::Cancel);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_actions().is_empty());
    let f = Rect::new(0.25, 0.5, 360., 720.);
    h.render(
        &mut Scene::new(),
        &mut t,
        f,
        Rect::new(800., 800., 10., 10.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(h.semantic_mount(ACTION).is_none());
    render(&mut h, &mut t, Theme::LIGHT.into());
    assert_ne!(content_key(&h, ACTION, None), Some(key));
    assert!(!content_action(&mut h, key, NativeAction::Press).activated);
    let key = content_key(&h, ACTION, None).unwrap();
    let p = action_frame(&h).center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.set_inherited_surface("toolbar").unwrap();
    render(&mut h, &mut t, Theme::DARK.into());
    assert_eq!(content_key(&h, ACTION, None), Some(key));
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(h.drain_actions().len(), 1);
}

#[test]
fn damaged_empty_state_layout_rejects_before_cancelling_neighbor_composer() {
    let (mut s, _) = fixture::build().unwrap();
    let mut text = coretext::Text::new(1.);
    render(&mut s.host, &mut text, Theme::LIGHT.into());
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    render(&mut s.host, &mut text, Theme::LIGHT.into());
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    s.host
        .get_mut(MESSAGE)
        .unwrap()
        .component_mut::<EmptyStateContent>()
        .unwrap()
        .children_mut()
        .unwrap()
        .clear();
    assert!(
        s.host
            .replace_empty_state(MESSAGE, fixture::model(Some("Open")))
            .is_err()
    );
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert_eq!(s.host.focused_text_binding(), binding);
    let f = Rect::new(0.25, 0.5, 360., 720.);
    let mut scene = Scene::new();
    assert!(
        s.host
            .render(&mut scene, &mut text, f, f, Theme::LIGHT.into())
            .is_err()
    );
    assert!(scene.is_empty());
    assert_eq!(s.host.focused(), Some(COMPOSER));
    assert_eq!(s.host.focus_epoch(), epoch);
    assert_eq!(s.host.focused_text_binding(), binding);
    let after = s.host.text_input_snapshot().unwrap();
    assert_eq!(after.marked, before.marked);
    assert_eq!(after.selection, before.selection);
    assert_eq!(after.binding, before.binding);
}

#[test]
fn reorder_and_theme_preserve_an_already_queued_current_action() {
    let (mut s, _) = fixture::build().unwrap();
    let mut text = coretext::Text::new(1.);
    render(&mut s.host, &mut text, Theme::LIGHT.into());
    let key = content_key(&s.host, ACTION, None).unwrap();
    assert!(content_action(&mut s.host, key, NativeAction::Press).activated);
    s.host.reorder(ROOT, &[SCROLLER, LATEST, MESSAGE, COMPOSER]).unwrap();
    s.host.set_inherited_surface("panel").unwrap();
    render(&mut s.host, &mut text, Theme::DARK.into());
    assert_eq!(content_key(&s.host, ACTION, None), Some(key));
    let actions = s.host.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, ACTION);
    assert!(s.host.drain_actions().is_empty());
}
