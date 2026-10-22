//! Real public ContentHost consumer: no pre-click to seed a toolbar focus index.
use tekes_ui_controls::{composition::*, *};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::new(s.len() as f32 * 8., 16.)
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
fn render(host: &mut ContentHost, width: f32) {
    let frame = Rect::new(0., 0., width, 48.);
    host.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn event(host: &mut ContentHost, key: ControlKey, down: bool) -> ContentResponse {
    host.event(&ContentEvent::KeyInput(ContentKeyEvent {
        key: Some(key),
        down,
        repeat: false,
        modifiers: Default::default(),
        native_code: 0,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
    }))
}
fn host() -> ContentHost {
    host_profile(false)
}
fn host_profile(window: bool) -> ContentHost {
    let mut toolbar = if window {
        ToolbarControl::window()
    } else {
        ToolbarControl::default()
    };
    toolbar.set_viewport(Rect::new(0., 0., 600., 400.));
    toolbar.set_actions(vec![
        ToolbarAction {
            id: 1,
            title: "First".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 2,
            title: "Disabled".into(),
            enabled: false,
        },
        ToolbarAction {
            id: 3,
            title: "Last".into(),
            enabled: true,
        },
    ]);
    if window {
        toolbar.set_overflow_icon(Some(tekes_ui_scene::ImageId(8)));
        toolbar.set_action_icon(2, Some(tekes_ui_scene::ImageId(7)));
    }
    let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
    host.set_action_events(true);
    render(&mut host, 600.);
    host
}
fn take(host: &mut ContentHost) -> Option<u64> {
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .take_action()
}
#[test]
fn tab_entry_space_and_roving_keys_work_without_pointer_and_skip_disabled() {
    let mut host = host();
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::Right, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(3));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::Home, true);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
    event(&mut host, ControlKey::End, true);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(3));
    assert_eq!(host.drain_actions().len(), 1);
}
#[test]
fn changing_focus_cancels_old_space_arm_and_disabled_registration_is_not_activated() {
    let mut host = host();
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Right, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), None);
    assert!(host.drain_actions().is_empty());
    let toolbar = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap();
    toolbar.set_actions(vec![
        ToolbarAction {
            id: 1,
            title: "First".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 2,
            title: "Disabled".into(),
            enabled: false,
        },
        ToolbarAction {
            id: 3,
            title: "Last".into(),
            enabled: false,
        },
    ]);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
}
#[test]
fn narrow_overflow_is_keyboard_reachable_and_opening_is_not_an_action() {
    let mut host = host();
    render(&mut host, 60.);
    event(&mut host, ControlKey::Tab { reverse: false }, true);
    event(&mut host, ControlKey::Space, true);
    event(&mut host, ControlKey::Space, false);
    assert!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap()
            .is_open()
    );
    assert!(host.drain_actions().is_empty());
    assert_eq!(take(&mut host), None);
    event(&mut host, ControlKey::Escape, true);
    assert!(
        !host
            .get(ComponentId(1))
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap()
            .is_open()
    );
    render(&mut host, 600.);
    event(&mut host, ControlKey::Enter, true);
    assert_eq!(take(&mut host), Some(1));
    assert_eq!(host.drain_actions().len(), 1);
}

#[cfg(feature = "token-audit")]
#[test]
fn original_toolbar_component_states_observe_real_token_dependencies() {
    let mut cases = Vec::new();
    for window in [false, true] {
        for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
            for brand in 0..3 {
                for state in ["rest", "hover", "pressed", "focus", "disabled", "overflow"] {
                    let mut host = host_profile(window);
                    let width = if state == "overflow" { 60. } else { 600. };
                    render(&mut host, width);
                    match state {
                        "focus" => {
                            event(&mut host, ControlKey::Tab { reverse: false }, true);
                        }
                        "pressed" => {
                            event(&mut host, ControlKey::Tab { reverse: false }, true);
                            assert!(event(&mut host, ControlKey::Space, true).consumed);
                        }
                        "hover" => {
                            host.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
                                24., 24.,
                            )));
                        }
                        "disabled" => {
                            let t = host
                                .get_mut(ComponentId(1))
                                .unwrap()
                                .component_mut::<ToolbarControl>()
                                .unwrap();
                            t.set_actions(vec![
                                ToolbarAction {
                                    id: 1,
                                    title: "First".into(),
                                    enabled: false,
                                },
                                ToolbarAction {
                                    id: 2,
                                    title: "Disabled".into(),
                                    enabled: false,
                                },
                                ToolbarAction {
                                    id: 3,
                                    title: "Last".into(),
                                    enabled: false,
                                },
                            ]);
                            assert!(!t.focusable());
                        }
                        "overflow" => {
                            event(&mut host, ControlKey::Tab { reverse: false }, true);
                            event(&mut host, ControlKey::Space, true);
                            event(&mut host, ControlKey::Space, false);
                            assert!(
                                host.get(ComponentId(1))
                                    .unwrap()
                                    .component::<ToolbarControl>()
                                    .unwrap()
                                    .is_open()
                            );
                        }
                        _ => {}
                    }
                    let frame = Rect::new(0., 0., width, 48.);
                    let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                        host.render(
                            &mut Scene::new(),
                            &mut Text,
                            frame,
                            frame,
                            tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                        )
                        .unwrap()
                    });
                    assert!(!reads.is_empty());
                    if state == "pressed" {
                        event(&mut host, ControlKey::Space, false);
                        assert_eq!(take(&mut host), Some(1));
                        assert_eq!(host.drain_actions().len(), 1);
                    } else {
                        assert!(host.drain_actions().is_empty());
                    }
                    cases.push(serde_json::json!({"component":"tui.toolbar","placement":if window {"window"} else {"inline"},"action_state":state,"state_asserted":true,"theme":theme_index,"brand":brand,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 144);
    if let Some(path) = std::env::var_os("TEKESUI_TOOLBAR_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"cases":cases,"scope":"Actual ToolbarControl in original ContentHost: inline/window profiles, six states, four themes, three existing brands. Reuses ButtonContent and Menu; legacy ToolbarView is not exercised by this probe.","excluded":["legacy ToolbarView runtime","arbitrary caller Content slots","font backend and icon texture pixels","complete state combinations or dependency extraction","native/VoiceOver/Gold7 experience","performance"]})).unwrap()).unwrap();
    }
}

#[test]
fn window_constructor_uses_shared_design_role_through_real_content_host() {
    struct Roles(Vec<proposal::TextRole>);
    impl ButtonTextRenderer for Roles {
        fn measure(&mut self, text: &str, role: proposal::TextRole, _: f32) -> Size {
            Size::new(
                text.len() as f32 * role.style().size * 0.5,
                role.style().line_height,
            )
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            text: &str,
            role: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
            if text == "Read" {
                self.0.push(role);
            }
        }
    }
    for (mut toolbar, role) in [
        (ToolbarControl::window(), proposal::TextRole::UiBody),
        (ToolbarControl::default(), proposal::TextRole::UiBody),
    ] {
        toolbar.set_actions(vec![ToolbarAction {
            id: 1,
            title: "Read".into(),
            enabled: true,
        }]);
        let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
        let mut text = Roles(vec![]);
        let frame = Rect::new(0., 0., 500., 52.);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert_eq!(text.0, vec![role]);
        host.focus(Some(ComponentId(1)), true);
        assert!(event(&mut host, ControlKey::Space, true).consumed);
        assert!(event(&mut host, ControlKey::Space, false).activated);
    }
}

fn parts(host: &ContentHost) -> Vec<SemanticPart> {
    host.semantics()
        .into_iter()
        .find(|n| n.id == ComponentId(1))
        .unwrap()
        .parts
}
fn part(host: &ContentHost, name: &str) -> usize {
    parts(host)
        .into_iter()
        .find(|p| p.semantics.name == name)
        .unwrap()
        .index
}
#[test]
fn ax_parts_reuse_original_button_and_host_focus_without_duplicate_delivery() {
    for window in [false, true] {
        let mut h = host_profile(window);
        let first = part(&h, "First");
        let last = part(&h, "Last");
        let disabled = part(&h, "Disabled");
        assert!(
            !h.accessibility_action(ComponentId(1), Some(disabled), AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(h.focused(), None);
        assert!(
            h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Focus)
                .consumed
        );
        assert_eq!(h.focused(), Some(ComponentId(1)));
        assert!(parts(&h).iter().any(|p| p.index == last && p.focused));
        assert_eq!(take(&mut h), None);
        assert!(h.drain_actions().is_empty());
        assert!(
            h.accessibility_action(ComponentId(1), Some(first), AccessibilityAction::Press)
                .activated
        );
        assert_eq!(take(&mut h), Some(1));
        assert_eq!(h.drain_actions().len(), 1);
        event(&mut h, ControlKey::Space, false);
        assert_eq!(take(&mut h), None);
    }
}
#[test]
fn semantic_identity_survives_reorder_but_not_removal_and_reintroduction() {
    let mut h = host();
    let first = part(&h, "First");
    let last = part(&h, "Last");
    let actions = vec![
        ToolbarAction {
            id: 3,
            title: "Last renamed".into(),
            enabled: true,
        },
        ToolbarAction {
            id: 1,
            title: "First".into(),
            enabled: true,
        },
    ];
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_actions(actions.clone());
    assert!(
        !h.accessibility_action(ComponentId(1), Some(first), AccessibilityAction::Press)
            .consumed,
        "reordered geometry is not mounted until paint"
    );
    render(&mut h, 600.);
    assert_eq!(part(&h, "Last renamed"), last);
    assert_eq!(part(&h, "First"), first);
    assert!(
        h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(take(&mut h), Some(3));
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_actions(vec![actions[0].clone()]);
    render(&mut h, 600.);
    h.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_actions(actions);
    render(&mut h, 600.);
    assert_ne!(part(&h, "First"), first);
    assert!(
        !h.accessibility_action(ComponentId(1), Some(first), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(take(&mut h), None);
}
#[test]
fn ax_overflow_reuses_original_menu_and_rejects_closed_or_disabled_parts() {
    let mut h = host();
    render(&mut h, 64.);
    let more = part(&h, "More options");
    assert_eq!(
        parts(&h)
            .iter()
            .find(|p| p.index == more)
            .unwrap()
            .semantics
            .role,
        ContentRole::MenuButton
    );
    let open = h.accessibility_action(ComponentId(1), Some(more), AccessibilityAction::Press);
    assert!(open.consumed && !open.activated);
    assert!(h.drain_actions().is_empty());
    render(&mut h, 64.);
    let disabled = part(&h, "Disabled");
    let last = part(&h, "Last");
    assert!(
        parts(&h)
            .iter()
            .all(|p| p.index == more || p.semantics.role == ContentRole::MenuItem)
    );
    assert_eq!(
        parts(&h)
            .iter()
            .find(|p| p.index == more)
            .unwrap()
            .semantics
            .expanded,
        Some(true)
    );
    assert_eq!(h.semantic_part_parent(ComponentId(1), last), Some(more));
    assert!(
        !h.accessibility_action(ComponentId(1), Some(disabled), AccessibilityAction::Focus)
            .consumed
    );
    assert!(
        h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(ComponentId(1)));
    render(&mut h, 64.);
    assert!(parts(&h).iter().any(|p| p.index == last && p.focused));
    assert!(
        h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(take(&mut h), Some(3));
    assert_eq!(h.drain_actions().len(), 1);
    assert!(
        !h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(take(&mut h), None);
}
#[test]
fn ax_focus_cancels_old_space_arm_and_preserves_fresh_activation() {
    let mut h = host();
    event(&mut h, ControlKey::Tab { reverse: false }, true);
    event(&mut h, ControlKey::Space, true);
    let last = part(&h, "Last");
    assert!(
        h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Focus)
            .consumed
    );
    event(&mut h, ControlKey::Space, false);
    assert_eq!(take(&mut h), None);
    assert!(h.drain_actions().is_empty());
    event(&mut h, ControlKey::Space, true);
    event(&mut h, ControlKey::Space, false);
    assert_eq!(take(&mut h), Some(3));
    assert_eq!(h.drain_actions().len(), 1);
}
#[test]
fn toolbar_parts_use_actual_ancestor_clip_and_reject_unpainted_targets() {
    let mut h = host();
    let last = part(&h, "Last");
    let frame = Rect::new(0., 0., 600., 48.);
    let clip = Rect::new(0., 0., 20., 48.);
    h.render(
        &mut Scene::new(),
        &mut Text,
        frame,
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(parts(&h).iter().all(|p| p.frame.max_x() <= 20.));
    assert!(
        !h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Press)
            .consumed
    );
    assert!(h.drain_actions().is_empty());
    render(&mut h, 600.);
    assert_eq!(part(&h, "Last"), last);
    assert!(
        h.accessibility_action(ComponentId(1), Some(last), AccessibilityAction::Press)
            .activated
    );
}

#[test]
fn native_function_and_caps_flags_do_not_block_plain_toolbar_roving() {
    for modifiers in [
        InputModifiers {
            function: true,
            ..Default::default()
        },
        InputModifiers {
            function: true,
            caps_lock: true,
            ..Default::default()
        },
    ] {
        let mut h = host();
        event(&mut h, ControlKey::Tab { reverse: false }, true);
        h.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Right),
            down: true,
            repeat: false,
            modifiers,
            native_code: 124,
            characters: "\u{f703}".into(),
            characters_ignoring_modifiers: "\u{f703}".into(),
        }));
        event(&mut h, ControlKey::Enter, true);
        assert_eq!(take(&mut h), Some(3));
        assert_eq!(h.drain_actions().len(), 1);
    }
    for modifiers in [
        InputModifiers {
            command: true,
            ..Default::default()
        },
        InputModifiers {
            alt: true,
            ..Default::default()
        },
        InputModifiers {
            control: true,
            ..Default::default()
        },
        InputModifiers {
            shift: true,
            ..Default::default()
        },
    ] {
        let mut h = host();
        event(&mut h, ControlKey::Tab { reverse: false }, true);
        let r = h.event(&ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Right),
            down: true,
            repeat: false,
            modifiers,
            native_code: 124,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        }));
        assert!(!r.consumed);
        event(&mut h, ControlKey::Enter, true);
        assert_eq!(take(&mut h), Some(1));
    }
}

#[test]
fn overflow_remains_a_focusable_inspection_menu_when_commands_are_unavailable() {
    let mut h = host();
    let toolbar = h
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<ToolbarControl>()
        .unwrap();
    toolbar.set_actions(vec![ToolbarAction {
        id: 8,
        title: "Unavailable long command".into(),
        enabled: false,
    }]);
    render(&mut h, 64.);
    let more = part(&h, "More options");
    assert!(
        h.accessibility_action(ComponentId(1), Some(more), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(ComponentId(1)));
    assert!(
        h.accessibility_action(ComponentId(1), Some(more), AccessibilityAction::Press)
            .consumed
    );
    render(&mut h, 64.);
    let item = part(&h, "Unavailable long command");
    assert!(
        !h.accessibility_action(ComponentId(1), Some(item), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(take(&mut h), None);
    assert!(h.drain_actions().is_empty());
}

#[test]
fn toolbar_geometry_automatically_follows_resolved_title_size_including_zoom() {
    struct Scaled(f32);
    impl ButtonTextRenderer for Scaled {
        fn resolved_font_size(&mut self, _: proposal::TextRole) -> f32 {
            self.0
        }
        fn measure(&mut self, s: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(s.len() as f32 * self.0 * 0.5, 16. * self.0 / 13.)
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
    for title_size in [9.75, 13., 20., 26.] {
        for window in [false, true] {
            let mut toolbar = if window {
                ToolbarControl::window()
            } else {
                ToolbarControl::default()
            };
            toolbar.set_icon_actions(vec![ToolbarIconAction {
                id: 9,
                image: tekes_ui_scene::ImageId(1),
                label: "Icon".into(),
                enabled: true,
            }]);
            let mut text = Scaled(title_size);
            let ratio = title_size / 13.;
            assert!((ToolbarControl::window().row_height(&mut text) - 42. * ratio).abs() < 0.001);
            let natural = toolbar.measure(&mut text);
            assert!((natural.height - 42. * ratio).abs() < 0.001);
            assert!((natural.width - 42. * ratio).abs() < 0.001);
            let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
            let frame = Rect::new(0., 0., 200., natural.height);
            let mut scene = Scene::new();
            host.render(&mut scene, &mut text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            let image = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Image(i) => Some(i),
                    _ => None,
                })
                .unwrap();
            assert!((image.rect.size.width - 16. * ratio).abs() < 0.001);
            assert!((image.rect.size.height - 16. * ratio).abs() < 0.001);
            // Pixel-align raster artwork only; original action geometry below
            // remains at the exact typography-derived center.
            assert!((image.rect.min_y() - (13. * ratio).round()).abs() < 0.001);
            assert!((image.rect.center().y - 21. * ratio).abs() <= 0.5001);
            let icon = host
                .semantics()
                .into_iter()
                .find(|n| n.id == ComponentId(1))
                .unwrap()
                .parts
                .into_iter()
                .find(|p| p.semantics.name == "Icon")
                .unwrap();
            assert!((icon.frame.size.width - 28. * ratio).abs() < 0.001);
            assert!((icon.frame.size.height - 28. * ratio).abs() < 0.001);
            assert!((icon.frame.min_y() - 7. * ratio).abs() < 0.001);
            assert!((icon.frame.center().y - 21. * ratio).abs() < 0.001);
        }
    }
}

#[test]
fn natural_icon_width_survives_repeated_font_zoom_and_translated_hosts() {
    struct Zoom(f32);
    impl ButtonTextRenderer for Zoom {
        fn resolved_font_size(&mut self, _: proposal::TextRole) -> f32 {
            self.0
        }
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(self.0, self.0)
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
    let base = proposal::TextRole::UiBody.style().size;
    let mut effective = base;
    for _ in 0..24 {
        for count in [1, 2, 3, 6] {
            for x in [0., 69., 967.125] {
                let mut toolbar = ToolbarControl::window();
                toolbar.set_icon_actions(
                    (0..count)
                        .map(|i| ToolbarIconAction {
                            id: i as u64,
                            image: tekes_ui_scene::ImageId(i as u32),
                            label: format!("Action {i}"),
                            enabled: i != 0,
                        })
                        .collect(),
                );
                let mut text = Zoom(effective);
                let size = toolbar.measure(&mut text);
                let frame = Rect::new(x, 0., size.width, size.height);
                let viewport = Rect::new(0., 0., 4000., 400.);
                toolbar.set_viewport(viewport);
                let mut host = ContentHost::new(Content::new(ComponentId(1), toolbar)).unwrap();
                host.render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    viewport,
                    Theme::LIGHT.into(),
                )
                .unwrap();
                let parts = host
                    .semantics()
                    .into_iter()
                    .find(|n| n.id == ComponentId(1))
                    .unwrap()
                    .parts;
                assert_eq!(
                    parts.len(),
                    count,
                    "natural width must fit: font={effective} count={count} x={x}: {parts:?}"
                );
                assert!(
                    parts.iter().all(|p| p.semantics.name != "More options"),
                    "false overflow: font={effective} count={count} x={x}"
                );
                // A real width shortage must still overflow; no pixel epsilon.
                let narrow = Rect::new(x, 0., size.width - 0.25, size.height);
                host.render(
                    &mut Scene::new(),
                    &mut text,
                    narrow,
                    viewport,
                    Theme::LIGHT.into(),
                )
                .unwrap();
                assert!(
                    host.semantics()
                        .into_iter()
                        .flat_map(|n| n.parts)
                        .any(|p| p.semantics.name == "More options")
                );
            }
        }
        // Same arithmetic as the real AppHost Command-plus context update.
        effective = base * ((effective + 1.) / base);
    }
}
