use super::button_model::ButtonArm;
use super::tests_1::{CheckboxText, FixedText};
use super::*;
use tekes_ui_scene::Primitive;
use tekes_ui_tokens::Palette;
use tekes_ui_tokens::{Theme, proposal};

#[test]
fn stationary_pointer_rehits_after_layout_translation_and_restoration() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 16.0),
        drawn: None,
    };
    let mut scene = Scene::new();
    let render = |button: &mut ButtonModel,
                  frame: Rect,
                  clip: Option<Rect>,
                  text: &mut FixedText,
                  scene: &mut Scene| {
        button.render(
            scene,
            text,
            "Animated",
            frame,
            clip,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
    };

    render(
        &mut button,
        Rect::new(10.0, 10.0, 100.0, 30.0),
        None,
        &mut text,
        &mut scene,
    );
    assert!(button.pointer_down_at(Point::new(20.0, 20.0)));
    assert!(button.state.hovered && button.state.pressed);

    render(
        &mut button,
        Rect::new(120.0, 10.0, 100.0, 30.0),
        None,
        &mut text,
        &mut scene,
    );
    assert!(!button.state.hovered && !button.state.pressed);

    render(
        &mut button,
        Rect::new(10.0, 10.0, 100.0, 30.0),
        None,
        &mut text,
        &mut scene,
    );
    assert!(button.state.hovered && button.state.pressed);
    assert!(button.pointer_up_at(Point::new(20.0, 20.0)));
}

#[test]
fn stationary_pointer_hover_tracks_new_ancestor_clip() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 16.0),
        drawn: None,
    };
    let mut scene = Scene::new();
    let frame = Rect::new(10.0, 10.0, 100.0, 30.0);
    let render = |button: &mut ButtonModel, clip, text: &mut FixedText, scene: &mut Scene| {
        button.render(
            scene,
            text,
            "Clipped",
            frame,
            clip,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
    };
    render(&mut button, None, &mut text, &mut scene);
    button.pointer_move_at(Point::new(20.0, 20.0));
    assert!(button.state.hovered);
    render(
        &mut button,
        Some(Rect::new(50.0, 10.0, 60.0, 30.0)),
        &mut text,
        &mut scene,
    );
    assert!(!button.state.hovered);
    render(
        &mut button,
        Some(Rect::new(10.0, 10.0, 100.0, 30.0)),
        &mut text,
        &mut scene,
    );
    assert!(button.state.hovered);
}

#[test]
fn leading_and_centered_label_positions_are_component_owned() {
    let mut leading = ButtonModel::default();
    let mut centered = ButtonModel::default();
    for (button, align) in [
        (&mut leading, ButtonTextAlign::Leading),
        (&mut centered, ButtonTextAlign::Center),
    ] {
        let mut text = FixedText {
            measured: Size::new(40.0, 16.0),
            drawn: None,
        };
        button.render(
            &mut Scene::new(),
            &mut text,
            "Label",
            Rect::new(10.0, 20.0, 140.0, 32.0),
            None,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Panel,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            align,
        );
        let (drawn, actual_align) = text.drawn.unwrap();
        assert_eq!(actual_align, align);
        assert_eq!(drawn.min_x(), 22.0);
        assert_eq!(drawn.size.width, 140.0 - ButtonSize::Body.inset() * 2.0);
    }
}

#[test]
fn button_release_contract_and_disabled_loading() {
    let mut b = ButtonModel::default();
    b.pointer_down();
    assert!(!b.pointer_up(false));
    b.pointer_down();
    assert!(b.pointer_up(true));
    assert!(!b.pointer_up(true));
    b.state.focused = true;
    assert!(!b.key(ControlKey::Space, true));
    assert!(b.key(ControlKey::Space, false));
    for loading in [false, true] {
        b.state.loading = loading;
        b.state.disabled = !loading;
        b.pointer_down();
        assert!(!b.pointer_up(true));
        assert!(!b.key(ControlKey::Enter, true));
    }
}

#[test]
fn keyboard_and_pointer_arms_cannot_release_or_activate_each_other() {
    let mut button = ButtonModel::default();
    button.state.focused = true;
    assert!(!button.key(ControlKey::Space, true));
    assert!(!button.key(ControlKey::Space, true)); // key repeat does not re-arm
    assert!(button.state.pressed);
    assert!(!button.pointer_up(true));
    assert!(button.state.pressed);
    assert!(button.key(ControlKey::Space, false));
    assert!(!button.key(ControlKey::Space, false)); // one activation per press

    button.pointer_down();
    assert!(!button.key(ControlKey::Space, false));
    assert!(button.state.pressed);
    assert!(button.pointer_up(true));

    button.state.focused = true;
    assert!(!button.key(ControlKey::Space, true));
    button.blur();
    assert!(!button.key(ControlKey::Space, false));
    assert!(!button.state.pressed);

    button.state.focused = true;
    assert!(!button.key(ControlKey::Space, true));
    button.state.disabled = true;
    assert!(!button.key(ControlKey::Space, false));
    assert!(!button.state.pressed);

    button.state.disabled = false;
    button.pointer_down();
    button.state.loading = true;
    assert!(!button.pointer_up(true));
    assert!(!button.state.pressed);
}

#[test]
fn checked_button_release_matches_original_clip_and_once_predicate() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40., 16.),
        drawn: None,
    };
    button.render(
        &mut Scene::new(),
        &mut text,
        "Action",
        Rect::new(0., 0., 120., 36.),
        None,
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
        ButtonTextAlign::Center,
    );
    let point = Point::new(20., 20.);
    assert!(!button.pointer_release_would_activate(point));
    assert!(button.pointer_down_at(point));
    let state = button.state;
    assert!(button.pointer_armed());
    assert!(button.pointer_release_would_activate(point));
    assert!(button.pointer_release_would_activate(point));
    assert_eq!(button.state, state);
    button.state.disabled = true;
    assert!(!button.pointer_release_would_activate(point));
    button.state.disabled = false;
    button.state.loading = true;
    assert!(!button.pointer_release_would_activate(point));
    button.state.loading = false;
    let shape = tekes_ui_scene::ClipShape::normalize(
        Rect::new(0., 0., 10., 36.),
        Default::default(),
        Point::new(0., 0.),
    )
    .unwrap();
    button.set_clip_region(Some(
        tekes_ui_scene::ClipRegion::new(
            &[shape],
            tekes_ui_scene::ClipBudget {
                max_shapes: 1,
                max_work: 4096,
            },
        )
        .unwrap(),
    ));
    assert!(!button.pointer_release_would_activate(point));
    assert!(!button.pointer_up_at(point));
    assert!(!button.pointer_armed());
    button.set_clip_region(None);
    assert!(button.pointer_down_at(point));
    assert!(button.pointer_release_would_activate(point));
    assert!(button.pointer_up_at(point));
    assert!(!button.pointer_release_would_activate(point));
    assert!(!button.pointer_up_at(point));
    button.state.focused = true;
    button.key(ControlKey::Space, true);
    assert!(!button.pointer_armed());
    assert!(!button.pointer_release_would_activate(point));
    assert!(button.key(ControlKey::Space, false));
}

#[test]
fn pointer_cancellation_preserves_keyboard_space_arm() {
    let mut button = ButtonModel::default();
    button.state.focused = true;
    assert!(!button.key(ControlKey::Space, true));
    button.cancel_pointer();
    assert!(button.state.pressed);
    assert!(button.key(ControlKey::Space, false));

    button.pointer_down();
    assert!(button.state.pressed);
    button.cancel_pointer();
    assert!(!button.state.pressed);
    assert!(!button.pointer_up(true));
}

#[test]
fn inactive_window_rejects_new_button_and_checkbox_activation() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40., 16.),
        drawn: None,
    };
    let mut scene = Scene::new();
    button.render(
        &mut scene,
        &mut text,
        "Action",
        Rect::new(0., 0., 120., 36.),
        None,
        Theme::LIGHT,
        crate::paint::ButtonVariant::Quiet,
        ButtonSize::Xs,
        proposal::TextRole::ButtonXs,
        ButtonTextAlign::Center,
    );
    button.set_window_active(false, Duration::ZERO);
    let inside = Point::new(20., 20.);
    button.pointer_move_at(inside);
    assert!(!button.state.hovered);
    assert!(!button.pointer_down_at(inside));
    assert!(!button.pointer_up_at(inside));
    button.state.focused = true;
    assert!(!button.key(ControlKey::Enter, true));
    assert!(!button.key(ControlKey::Space, true));
    assert!(!button.key(ControlKey::Space, false));
    button.set_window_active(true, Duration::ZERO);
    assert!(button.pointer_down_at(inside));
    assert!(button.pointer_up_at(inside));

    let mut checkbox = Checkbox::default();
    checkbox.render(
        &mut scene,
        &mut text,
        "Option",
        Rect::new(0., 0., 120., 36.),
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    checkbox.button.set_window_active(false, Duration::ZERO);
    checkbox.pointer_move_at(inside);
    assert!(!checkbox.button.state.hovered);
    assert!(!checkbox.pointer_down_at(inside));
    assert!(!checkbox.pointer_up_at(inside));
    assert_eq!(checkbox.checked, CheckState::Unchecked);
    checkbox.button.set_window_active(true, Duration::ZERO);
    assert!(checkbox.pointer_down_at(inside));
    assert!(checkbox.pointer_up_at(inside));
    assert_eq!(checkbox.checked, CheckState::Checked);
}

#[test]
fn render_cancels_pointer_and_space_arms_when_disabled_or_loading() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 16.0),
        drawn: None,
    };
    let frame = Rect::new(10.0, 10.0, 100.0, 30.0);
    let render = |button: &mut ButtonModel, text: &mut FixedText| {
        button.render(
            &mut Scene::new(),
            text,
            "Action",
            frame,
            None,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
    };

    render(&mut button, &mut text);
    for loading in [false, true] {
        button.state.disabled = false;
        button.state.loading = false;
        render(&mut button, &mut text);
        assert!(button.pointer_down_at(Point::new(20.0, 20.0)));
        button.state.disabled = !loading;
        button.state.loading = loading;
        render(&mut button, &mut text);
        button.state.disabled = false;
        button.state.loading = false;
        render(&mut button, &mut text);
        assert!(!button.pointer_up_at(Point::new(20.0, 20.0)));

        button.state.focused = true;
        assert!(!button.key(ControlKey::Space, true));
        button.state.disabled = !loading;
        button.state.loading = loading;
        render(&mut button, &mut text);
        button.state.disabled = false;
        button.state.loading = false;
        render(&mut button, &mut text);
        assert!(!button.key(ControlKey::Space, false));
        assert!(!button.state.pressed);
    }
}

#[test]
fn render_cancels_space_arm_when_controlled_focus_is_lost() {
    let mut button = ButtonModel::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 16.0),
        drawn: None,
    };
    let render = |button: &mut ButtonModel, text: &mut FixedText| {
        button.render(
            &mut Scene::new(),
            text,
            "Action",
            Rect::new(10.0, 10.0, 100.0, 30.0),
            None,
            Theme::LIGHT,
            crate::paint::ButtonVariant::Quiet,
            ButtonSize::Xs,
            proposal::TextRole::ButtonXs,
            ButtonTextAlign::Center,
        );
    };
    button.state.focused = true;
    assert!(!button.key(ControlKey::Space, true));
    button.state.focused = false;
    render(&mut button, &mut text);
    button.state.focused = true;
    render(&mut button, &mut text);
    assert!(!button.key(ControlKey::Space, false));
    assert!(!button.state.pressed);
}
#[test]
fn mixed_checkbox_and_disabled_combination() {
    let mut c = Checkbox {
        checked: CheckState::Mixed,
        ..Default::default()
    };
    assert!(c.toggle());
    assert_eq!(c.checked, CheckState::Checked);
    c.button.state.disabled = true;
    assert!(!c.toggle());
    assert_eq!(c.checked, CheckState::Checked);
}

#[test]
fn checkbox_renders_and_hits_the_same_clipped_row_with_centered_default_text() {
    let mut checkbox = Checkbox::default();
    let mut text = FixedText {
        measured: Size::new(172.0, 18.0),
        drawn: None,
    };
    let mut scene = Scene::new();
    let layout = checkbox.render(
        &mut scene,
        &mut text,
        "Receive project notifications",
        Rect::new(10.0, 20.0, 400.0, 36.0),
        Some(Rect::new(10.0, 20.0, 220.0, 36.0)),
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert_eq!(layout.frame, Rect::new(10.0, 20.0, 400.0, 36.0));
    assert_eq!(layout.control, Rect::new(10.0, 30.0, 16.0, 16.0));
    assert_eq!(layout.label, Rect::new(32.0, 29.0, 378.0, 18.0));
    assert_eq!(layout.visible, Some(Rect::new(10.0, 20.0, 220.0, 36.0)));
    assert_eq!(text.drawn, Some((layout.label, ButtonTextAlign::Leading)));
    assert!(checkbox.pointer_down_at(Point::new(200.0, 30.0)));
    assert!(checkbox.button.state.pressed);
    assert!(!checkbox.pointer_up_at(Point::new(230.0, 30.0)));
    assert_eq!(checkbox.checked, CheckState::Unchecked);
    assert!(checkbox.pointer_down_at(Point::new(200.0, 30.0)));
    assert!(checkbox.pointer_up_at(Point::new(200.0, 30.0)));
    assert_eq!(checkbox.checked, CheckState::Checked);
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::PushClip(_)))
    );
}

#[test]
fn checkbox_expands_for_large_text_and_cancels_stale_hit_when_disabled_or_cleared() {
    let mut checkbox = Checkbox::default();
    let mut text = FixedText {
        measured: Size::new(180.0, 48.0),
        drawn: None,
    };
    let layout = checkbox.render(
        &mut Scene::new(),
        &mut text,
        "Large project notification label",
        Rect::new(8.0, 12.0, 400.0, 32.0),
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert_eq!(layout.frame.size.height, 56.0);
    assert_eq!(layout.control, Rect::new(8.0, 32.0, 16.0, 16.0));
    assert_eq!(layout.label, Rect::new(30.0, 16.0, 378.0, 48.0));
    assert!(checkbox.pointer_down_at(Point::new(100.0, 20.0)));
    checkbox.button.state.disabled = true;
    checkbox.render(
        &mut Scene::new(),
        &mut text,
        "Large project notification label",
        Rect::new(8.0, 12.0, 400.0, 32.0),
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert!(!checkbox.button.state.pressed);
    assert!(!checkbox.pointer_down_at(Point::new(100.0, 20.0)));
    checkbox.clear_layout();
    assert_eq!(checkbox.layout(), None);
    assert!(!checkbox.pointer_up_at(Point::new(100.0, 20.0)));
}

#[test]
fn checkbox_first_line_center_label_gap_and_row_height_scale_naturally() {
    for backing_scale in [1.0, 2.0] {
        for (type_scale, first_line, label_height, expected_row_height) in [
            (1.0, 18.0, 18.0, 32.0),
            (1.5, 27.0, 54.0, 62.0),
            (2.0, 36.0, 108.0, 116.0),
        ] {
            let mut checkbox = Checkbox::default();
            let mut text = CheckboxText {
                measured: Size::new(320.0, label_height),
                first_line,
                backing_scale,
                drawn: None,
            };
            let frame = Rect::new(12.0, 20.0, 220.0, 32.0);
            let layout = checkbox.render(
                &mut Scene::new(),
                &mut text,
                "Notifications with wrapped supporting text",
                frame,
                None,
                Theme::LIGHT,
                proposal::TextRole::UiBody,
            );

            assert_eq!(layout.control.size, Size::new(16.0, 16.0));
            assert_eq!(layout.label.min_x(), 34.0, "16pt mark + 6pt gap");
            assert_eq!(layout.frame.size.height, expected_row_height);
            assert!(
                (layout.control.center().y - (layout.label.min_y() + first_line * 0.5)).abs()
                    < 0.001,
                "first line and visual box must share a center at {type_scale}x / {backing_scale}x"
            );
            assert!(
                layout.hit_test(Point::new(frame.min_x() + 2.0, layout.frame.center().y)),
                "the entire row remains the hit target"
            );
            assert_eq!(text.drawn.unwrap().0, layout.label);
        }
    }
}

#[test]
fn checkbox_label_uses_semantic_text_ink_for_all_mark_and_control_states() {
    let palettes = [0, 1, 2];
    let states = [
        (CheckState::Unchecked, ControlState::default()),
        (
            CheckState::Checked,
            ControlState {
                hovered: true,
                ..Default::default()
            },
        ),
        (
            CheckState::Checked,
            ControlState {
                pressed: true,
                ..Default::default()
            },
        ),
        (
            CheckState::Mixed,
            ControlState {
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
        ),
        (
            CheckState::Unchecked,
            ControlState {
                disabled: true,
                ..Default::default()
            },
        ),
        (
            CheckState::Checked,
            ControlState {
                disabled: true,
                hovered: true,
                ..Default::default()
            },
        ),
        (
            CheckState::Mixed,
            ControlState {
                disabled: true,
                focused: true,
                focus_visible: true,
                ..Default::default()
            },
        ),
    ];

    for theme in Theme::ALL {
        for brand in palettes {
            let palette =
                tekes_ui_tokens::Palette::new(theme, brand).expect("registered Gallery brand");
            for (checked, state) in states {
                let mut checkbox = Checkbox {
                    button: ButtonModel {
                        state,
                        ..Default::default()
                    },
                    checked,
                };
                let mut text = CheckboxText {
                    measured: Size::new(120.0, 18.0),
                    first_line: 18.0,
                    backing_scale: 1.0,
                    drawn: None,
                };
                checkbox.render(
                    &mut Scene::new(),
                    &mut text,
                    "Notifications",
                    Rect::new(0.0, 0.0, 200.0, 36.0),
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
                let actual = text.drawn.expect("checkbox label is drawn").1;
                let role = if state.disabled {
                    "tertiary-text"
                } else {
                    "primary-text"
                };
                assert_eq!(
                    actual,
                    palette.color(role).expect("semantic label role"),
                    "label ink must remain semantic for {checked:?}, {state:?}, brand {brand}, theme {theme:?}"
                );
            }
        }
    }
}

#[test]
fn checkbox_keyboard_activation_uses_the_shared_button_space_contract() {
    let mut checkbox = Checkbox::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 18.0),
        drawn: None,
    };
    checkbox.render(
        &mut Scene::new(),
        &mut text,
        "Notifications",
        Rect::new(0.0, 0.0, 200.0, 36.0),
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert!(!checkbox.key(ControlKey::Enter, true));
    checkbox.button.state.focused = true;
    checkbox.button.state.focus_visible = true;
    assert!(!checkbox.key(ControlKey::Space, true));
    assert!(checkbox.button.state.pressed);
    assert!(checkbox.key(ControlKey::Space, false));
    assert_eq!(checkbox.checked, CheckState::Checked);
    checkbox.button.state.disabled = true;
    assert!(!checkbox.key(ControlKey::Space, true));
    assert_eq!(checkbox.checked, CheckState::Checked);
}

#[test]
fn checkbox_pointer_capture_survives_blur_but_keyboard_space_arm_does_not() {
    let mut checkbox = Checkbox::default();
    let mut text = FixedText {
        measured: Size::new(40.0, 18.0),
        drawn: None,
    };
    let mut scene = Scene::new();
    let frame = Rect::new(0.0, 0.0, 200.0, 36.0);
    checkbox.render(
        &mut scene,
        &mut text,
        "Notifications",
        frame,
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    let point = Point::new(120.0, 18.0);
    assert!(checkbox.pointer_down_at(point));
    checkbox.button.state.focused = false;
    checkbox.render(
        &mut scene,
        &mut text,
        "Notifications",
        frame,
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert!(checkbox.button.state.pressed);
    assert!(checkbox.pointer_up_at(point));
    assert_eq!(checkbox.checked, CheckState::Checked);

    checkbox.button.state.focused = true;
    assert!(!checkbox.key(ControlKey::Space, true));
    assert!(checkbox.button.state.pressed);
    checkbox.button.state.focused = false;
    checkbox.render(
        &mut scene,
        &mut text,
        "Notifications",
        frame,
        None,
        Theme::LIGHT,
        proposal::TextRole::UiBody,
    );
    assert!(!checkbox.button.state.pressed);
    checkbox.button.state.focused = true;
    assert!(!checkbox.key(ControlKey::Space, false));
    assert_eq!(checkbox.checked, CheckState::Checked);
}
#[test]
fn tab_menu_boundaries_and_escape() {
    let mut t = Tabs::new(3);
    t.key(ControlKey::Left);
    assert_eq!(t.selected, 2);
    t.key(ControlKey::Home);
    assert_eq!(t.selected, 0);
    let mut m = Menu::new(3);
    m.show();
    m.key(ControlKey::Down);
    assert_eq!(m.key(ControlKey::Enter), Some(1));
    assert!(!m.open);
    m.show();
    m.key(ControlKey::Escape);
    assert!(!m.open);
    assert!(!Tabs::new(0).key(ControlKey::End));
}
#[test]
fn modal_traps_focus_and_restores_invoker() {
    let mut d = Dialog::default();
    d.show(7);
    d.tab(true, 2);
    assert_eq!(d.focus, 1);
    d.tab(false, 2);
    assert_eq!(d.focus, 0);
    assert_eq!(d.close(), Some(7));
    assert_eq!(d.close(), None);
}
#[test]
fn numeric_clamps_and_rejects_invalid_values() {
    assert!(NumberField::new(0., 1., 0., 1.).is_none());
    let mut n = NumberField::new(99., 0., 10., 1.).unwrap();
    assert_eq!(n.value(), 10.);
    assert!(!n.increment(true));
    assert!(n.increment(false));
    assert!(!n.set(f64::NAN));
    assert_eq!(n.value(), 9.);
}

#[cfg(test)]
mod empty_state_surface_tests {
    use super::*;
    use tekes_ui_tokens::{
        Theme,
        surface_palette::{resolve_surface, surface_names},
    };
    #[derive(Default)]
    struct Text {
        measures: usize,
        inks: Vec<Rgba>,
    }
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            self.measures += 1;
            Size::new(50., 18.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            ink: Rgba,
        ) {
            self.inks.push(ink);
        }
    }
    #[test]
    fn empty_state_prepared_action_consumes_surface_packets_and_keeps_geometry() {
        for theme in Theme::ALL {
            let palettes = (0..3)
                .map(|brand| Palette::new(theme, brand).unwrap())
                .chain(["blue", "blue-inputs"].into_iter().map(|id| {
                    Palette::for_theme_definition(theme, id).expect("required Theme definition")
                }));
            for palette in palettes {
                for surface in surface_names() {
                    for state in ["rest", "hover", "pressed", "disabled", "focus"] {
                        let mut button = ButtonModel::default();
                        let mut text = Text::default();
                        let frame = Rect::new(10.25, 12.5, 140., 40.);
                        let clip = Rect::new(0., 0., 220., 160.);
                        let layout = button.prepare_label(
                            &mut text,
                            "Create",
                            frame,
                            Some(clip),
                            proposal::TextRole::UiBody,
                        );
                        button.set_surface(surface);
                        match state {
                            "hover" => button.pointer_move_at(frame.center()),
                            "pressed" => {
                                assert!(button.pointer_down_at(frame.center()));
                            }
                            "disabled" => button.state.disabled = true,
                            "focus" => {
                                button.state.focused = true;
                                button.state.focus_visible = true;
                            }
                            _ => {}
                        }
                        let expected = resolve_surface(
                            palette,
                            surface,
                            "action-primary",
                            if state == "focus" { "rest" } else { state },
                        )
                        .unwrap();
                        let measures = text.measures;
                        let mut scene = Scene::new();
                        button.paint_prepared_label(
                            &mut scene,
                            &mut text,
                            "Create",
                            layout,
                            Some(clip),
                            palette,
                            proposal::TextRole::UiBody,
                        );
                        assert_eq!(
                            text.measures, measures,
                            "paint must retain prepared geometry"
                        );
                        assert_eq!(button.layout(), Some(layout));
                        assert_eq!(
                            text.inks,
                            vec![expected.text],
                            "{theme:?}/{surface}/{state}"
                        );
                        let quads: Vec<_> = scene
                            .primitives()
                            .iter()
                            .filter_map(|p| match p {
                                tekes_ui_scene::Primitive::Quad(quad) => Some(quad),
                                _ => None,
                            })
                            .collect();
                        assert_eq!(
                            quads[0].fill, expected.background,
                            "{theme:?}/{surface}/{state}"
                        );
                        if let Some(boundary) = expected.boundary {
                            assert_eq!(quads[0].border_color, boundary);
                            assert_eq!(
                                quads[0].border_width,
                                proposal::metric("stroke", "stroke-rule").unwrap()
                            );
                        } else {
                            assert_eq!(quads[0].border_width, 0.);
                        }
                        if state == "focus" {
                            assert_eq!(quads.len(), 2);
                            assert_eq!(
                                quads[1].border_color,
                                expected.focus.expect("action focus packet")
                            );
                        } else {
                            assert_eq!(quads.len(), 1);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn notice_prepared_dismiss_consumes_quiet_and_primary_state_packets() {
        for theme in Theme::ALL {
            let palette = Palette::for_theme_definition(theme, "blue-inputs")
                .expect("required Theme definition");
            for surface in surface_names() {
                for (variant, usage) in [
                    (crate::paint::ButtonVariant::Quiet, "action-quiet"),
                    (crate::paint::ButtonVariant::Primary, "action-primary"),
                ] {
                    for state in ["rest", "hover", "pressed", "disabled", "focus"] {
                        let mut button = ButtonModel::default();
                        let mut text = Text::default();
                        let frame = Rect::new(10.25, 12.5, 140., 40.);
                        let clip = Rect::new(0., 0., 220., 160.);
                        let layout = button.prepare_label(
                            &mut text,
                            "Dismiss",
                            frame,
                            Some(clip),
                            proposal::TextRole::UiBody,
                        );
                        button.set_surface(surface);
                        match state {
                            "hover" => button.pointer_move_at(frame.center()),
                            "pressed" => {
                                assert!(button.pointer_down_at(frame.center()));
                            }
                            "disabled" => button.state.disabled = true,
                            "focus" => {
                                button.state.focused = true;
                                button.state.focus_visible = true;
                            }
                            _ => {}
                        }
                        let expected = resolve_surface(
                            palette,
                            surface,
                            usage,
                            if state == "focus" { "rest" } else { state },
                        )
                        .unwrap();
                        let measures = text.measures;
                        let mut scene = Scene::new();
                        button.paint_prepared_label_variant(
                            &mut scene,
                            &mut text,
                            "Dismiss",
                            layout,
                            Some(clip),
                            palette,
                            proposal::TextRole::UiBody,
                            variant,
                        );
                        assert_eq!(text.measures, measures);
                        assert_eq!(button.layout(), Some(layout));
                        assert_eq!(text.inks, vec![expected.text]);
                        let quads: Vec<_> = scene
                            .primitives()
                            .iter()
                            .filter_map(|p| match p {
                                tekes_ui_scene::Primitive::Quad(q) => Some(q),
                                _ => None,
                            })
                            .collect();
                        assert_eq!(quads[0].fill, expected.background);
                        if let Some(boundary) = expected.boundary {
                            assert_eq!(quads[0].border_color, boundary);
                            assert_eq!(
                                quads[0].border_width,
                                proposal::metric("stroke", "stroke-rule").unwrap()
                            );
                        } else {
                            assert_eq!(quads[0].border_width, 0.);
                        }
                        if state == "focus" {
                            assert_eq!(quads.len(), 2);
                            assert_eq!(quads[1].border_color, expected.focus.unwrap());
                        } else {
                            assert_eq!(quads.len(), 1);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod select_snapshot_tests {
    use super::*;
    #[test]
    fn prepared_content_press_delegates_current_accessibility_decision_without_effects() {
        let mut button = ButtonModel::default();
        for (disabled, loading, suspended) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            button.state.disabled = disabled;
            button.state.loading = loading;
            button.motion_suspended = suspended;
            let state = button.state;
            let arm = button.arm;
            assert_eq!(
                button.prepare_content_press(),
                button.prepare_accessibility_press()
            );
            assert_eq!(button.state, state);
            assert_eq!(button.arm, arm);
            assert_eq!(
                button.prepare_content_press().activated,
                !disabled && !loading && !suspended
            );
        }
        button.motion_suspended = false;
        button.state.focused = true;
        button.key(ControlKey::Space, true);
        let decision = button.prepare_content_press();
        assert!(!decision.activated);
        assert_eq!(button.arm, ButtonArm::Space);
        button.apply_content_action(decision);
        assert!(button.key(ControlKey::Space, false));
    }
}
