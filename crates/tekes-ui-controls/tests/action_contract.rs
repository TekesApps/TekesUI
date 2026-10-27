//! Public action-state contracts; model flags stay authoritative between frames.
use tekes_ui_controls::{
    Button, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    composition::{Component, ContentEvent},
    extended::{Link, SegmentedControl},
    layout_view::{LinkView, SegmentedControlView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
#[derive(Default)]
struct Labels;
impl ButtonTextRenderer for Labels {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(width), role.style().line_height)
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
#[test]
fn latest_icon_model_availability_blocks_pointer_release_without_a_render() {
    for loading in [false, true] {
        let mut model = Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Settings");
        let frame = Rect::new(0., 0., 40., 40.);
        model.arrange(&mut Labels, frame, frame);
        model.paint(
            &mut Scene::new(),
            &mut Labels,
            frame,
            frame,
            Palette::from(Theme::LIGHT).into(),
        );
        assert!(
            model
                .event(&ContentEvent::PointerDown(frame.center()))
                .consumed
        );
        model.button.state.loading = loading;
        model.button.state.disabled = !loading;
        assert!(
            !model
                .event(&ContentEvent::PointerUp(frame.center()))
                .activated,
            "latest disabled/loading must prevent activation before another paint"
        );
        assert_eq!(model.button.state.loading, loading);
        assert_eq!(model.button.state.disabled, !loading);
    }
}
#[test]
fn latest_link_model_disable_blocks_keyboard_and_pointer_without_a_render() {
    for keyboard in [false, true] {
        let mut view = LinkView::default();
        let mut model = Link::default();
        let frame = Rect::new(0., 0., 160., 40.);
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            "Read documentation",
            frame,
            None,
            Palette::from(Theme::LIGHT),
            proposal::TextRole::UiBody,
        );
        view.focus(true);
        if !keyboard {
            assert!(view.pointer_down(frame.center()));
        }
        model.button.state.disabled = true;
        let activated = if keyboard {
            view.key(ControlKey::Enter, true, &mut model).changed
        } else {
            view.pointer_up(frame.center(), &mut model)
        };
        assert!(
            !activated,
            "current model disabled state must be checked at activation"
        );
        assert!(model.button.state.disabled);
    }
}

#[test]
fn direct_link_non_enter_keys_observe_disabled_and_never_claim_parent_keys() {
    for key in [
        ControlKey::Space,
        ControlKey::Escape,
        ControlKey::Tab { reverse: false },
        ControlKey::Tab { reverse: true },
        ControlKey::Left,
        ControlKey::Right,
        ControlKey::Home,
        ControlKey::End,
    ] {
        for down in [false, true] {
            let mut link = Link::default();
            link.pointer_down();
            assert!(link.button.state.pressed);
            link.button.state.disabled = true;
            assert!(!link.key(key, down));
            assert!(!link.button.state.pressed);
            link.button.state.disabled = false;
            assert!(
                !link.pointer_up(true),
                "disabled observation must invalidate the old pointer press"
            );
            link.pointer_down();
            assert!(link.pointer_up(true), "new pointer input remains available");
            link.pointer_down();
            assert!(!link.key(key, down));
            assert!(
                link.pointer_up(true),
                "unrelated enabled keys do not cancel valid pointer ownership"
            );
        }
    }
}
#[test]
fn segmented_disabled_release_or_paint_cancels_space_arm_before_reenable() {
    for paint in [false, true] {
        let mut model = SegmentedControl::new(3);
        let mut view = SegmentedControlView::new(3);
        let frame = Rect::new(0., 0., 300., 40.);
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            &["One", "Two", "Three"],
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(1);
        assert!(view.key_event(ControlKey::Space, true, &mut model).consumed);
        model.set_enabled(1, false);
        if paint {
            view.render(
                &mut Scene::new(),
                &mut Labels,
                &mut model,
                &["One", "Two", "Three"],
                frame,
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            );
        } else {
            assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
        }
        model.set_enabled(1, true);
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            &["One", "Two", "Three"],
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(1);
        assert!(
            !view.key_event(ControlKey::Space, false, &mut model).changed,
            "an observed disabled interval must not revive an old Space press"
        );
        assert_eq!(model.selected, 0);
    }
}

fn observe<T>(f: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    #[cfg(feature = "token-audit")]
    {
        tekes_ui_tokens::audit::capture(f)
    }
    #[cfg(not(feature = "token-audit"))]
    {
        (f(), Vec::new())
    }
}
#[test]
fn action_family_required_states_use_real_public_events_and_token_painters() {
    use tekes_ui_controls::{ButtonModel, ButtonSize, CheckState, Checkbox, paint::ButtonVariant};
    let registry:serde_json::Value=serde_json::from_str(include_str!("../../../design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json")).unwrap();
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            let palette = Palette::new(theme, brand).expect("registered Gallery brand");
            for component in [
                "tui.button",
                "tui.icon-button",
                "tui.link",
                "tui.checkbox",
                "tui.segmented-control",
            ] {
                let entry = registry["components"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["id"] == component)
                    .unwrap();
                let states: Vec<_> = entry["states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(entry["state_combinations"].as_array().unwrap())
                    .map(|v| v.as_str().unwrap())
                    .collect();
                for state in states {
                    let flags: Vec<_> = state.split('+').collect();
                    let has = |s| flags.contains(&s);
                    let ((), reads) = observe(|| {
                        let bounds = Rect::new(
                            12.,
                            12.,
                            if component == "tui.segmented-control" {
                                360.
                            } else {
                                240.
                            },
                            44.,
                        );
                        let point = bounds.center();
                        let mut scene = Scene::new();
                        let mut text = Labels;
                        match component {
                            "tui.button" => {
                                let mut control = ButtonModel::default();
                                control.state.disabled = has("disabled");
                                control.state.loading = has("loading");
                                let baseline = control.render(
                                    &mut Scene::new(),
                                    &mut text,
                                    "Apply settings",
                                    bounds,
                                    None,
                                    palette,
                                    ButtonVariant::Primary,
                                    ButtonSize::Body,
                                    proposal::TextRole::UiBody,
                                    ButtonTextAlign::Leading,
                                );
                                if has("hover") {
                                    control.pointer_move_at(point);
                                }
                                if has("pressed") {
                                    assert!(control.pointer_down_at(point));
                                }
                                if has("focus") {
                                    control.state.focused = true;
                                    control.state.focus_visible = true;
                                }
                                let layout = control.render(
                                    &mut scene,
                                    &mut text,
                                    "Apply settings",
                                    bounds,
                                    None,
                                    palette,
                                    ButtonVariant::Primary,
                                    ButtonSize::Body,
                                    proposal::TextRole::UiBody,
                                    ButtonTextAlign::Leading,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                if has("pressed") {
                                    assert!(control.state.pressed);
                                }
                                if has("hover") {
                                    assert!(control.state.hovered);
                                }
                                if has("focus") {
                                    assert!(control.state.focused && control.state.focus_visible);
                                }
                                if has("disabled") || has("loading") {
                                    assert!(!control.pointer_down_at(point));
                                    assert!(!control.key(ControlKey::Enter, true));
                                }
                            }
                            "tui.icon-button" => {
                                let mut model = Button::new(tekes_ui_scene::ImageId(1))
                                    .with_accessible_name("Create item");
                                let natural =
                                    model.measure(&mut text, Size::new(f32::MAX, f32::MAX)).size;
                                let frame = Rect::new(12., 12., natural.width, natural.height);
                                let p = frame.center();
                                model.button.state.disabled = has("disabled");
                                model.button.state.loading = has("loading");
                                model.arrange(&mut text, frame, frame);
                                model.paint(
                                    &mut Scene::new(),
                                    &mut text,
                                    frame,
                                    frame,
                                    palette.into(),
                                );
                                let baseline = model.button.layout().unwrap();
                                if has("hover") {
                                    model.event(&ContentEvent::PointerMove(p));
                                }
                                if has("pressed") {
                                    assert!(model.event(&ContentEvent::PointerDown(p)).consumed);
                                }
                                if has("focus") {
                                    model.set_focus(true, true);
                                }
                                model.paint(&mut scene, &mut text, frame, frame, palette.into());
                                let layout = model.button.layout().unwrap();
                                assert_eq!(layout.frame, baseline.frame);
                                assert_eq!(model.accessible_name(), "Create item");
                                if has("pressed") {
                                    assert!(model.button.state.pressed);
                                }
                                if has("hover") {
                                    assert!(model.button.state.hovered);
                                }
                                if has("focus") {
                                    assert!(model.button.state.focus_visible);
                                }
                                if has("disabled") || has("loading") {
                                    assert!(!model.event(&ContentEvent::PointerDown(p)).consumed);
                                    assert!(
                                        !model
                                            .event(&ContentEvent::Key {
                                                key: ControlKey::Enter,
                                                down: true
                                            })
                                            .activated
                                    );
                                    assert!(!model.accessibility_action(None, &tekes_ui_controls::composition::AccessibilityAction::Press).activated);
                                }
                            }
                            "tui.link" => {
                                let mut model = Link::default();
                                let mut view = LinkView::default();
                                model.button.state.disabled = has("disabled");
                                let baseline = view.render(
                                    &mut Scene::new(),
                                    &mut text,
                                    &mut model,
                                    "Read documentation",
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                if has("hover") {
                                    view.pointer_move(point);
                                }
                                if has("pressed") {
                                    assert!(view.pointer_down(point));
                                }
                                if has("focus") {
                                    view.focus(true);
                                }
                                let layout = view.render(
                                    &mut scene,
                                    &mut text,
                                    &mut model,
                                    "Read documentation",
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                if has("pressed") {
                                    assert!(model.button.state.pressed);
                                }
                                if has("hover") {
                                    assert!(model.button.state.hovered);
                                }
                                if has("focus") {
                                    assert!(model.button.state.focus_visible);
                                }
                                assert!(
                                    !view.key(ControlKey::Space, true, &mut model).consumed,
                                    "Space belongs to page scrolling"
                                );
                                if has("disabled") {
                                    assert!(!view.activate(&mut model));
                                    assert!(model.button.state.disabled);
                                }
                            }
                            "tui.checkbox" => {
                                let mut control = Checkbox {
                                    checked: if has("mixed") {
                                        CheckState::Mixed
                                    } else if has("checked") {
                                        CheckState::Checked
                                    } else {
                                        CheckState::Unchecked
                                    },
                                    ..Default::default()
                                };
                                let before = control.checked;
                                control.button.state.disabled = has("disabled");
                                control.button.state.error = has("error");
                                let baseline = control.render(
                                    &mut Scene::new(),
                                    &mut text,
                                    "Keep local settings",
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                if has("hover") {
                                    control.pointer_move_at(point);
                                }
                                if has("pressed") {
                                    assert!(control.pointer_down_at(point));
                                }
                                if has("focus") {
                                    control.button.state.focused = true;
                                    control.button.state.focus_visible = true;
                                }
                                let layout = control.render(
                                    &mut scene,
                                    &mut text,
                                    "Keep local settings",
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                assert_eq!(control.checked, before);
                                assert_eq!(control.button.state.error, has("error"));
                                if has("pressed") {
                                    assert!(control.button.state.pressed);
                                }
                                if has("hover") {
                                    assert!(control.button.state.hovered);
                                }
                                if has("focus") {
                                    assert!(control.button.state.focus_visible);
                                }
                                if has("disabled") {
                                    assert!(!control.pointer_down_at(point));
                                    assert!(!control.key(ControlKey::Space, true));
                                    assert!(!control.key(ControlKey::Space, false));
                                    assert_eq!(control.checked, before);
                                }
                            }
                            "tui.segmented-control" => {
                                let mut model = SegmentedControl::new(3);
                                let mut view = SegmentedControlView::new(3);
                                let labels = ["General", "Advanced", "Security"];
                                if has("selected") {
                                    assert!(model.select(1));
                                }
                                let value = model.selected;
                                if has("disabled") {
                                    for i in (0..3).filter(|i| *i != value) {
                                        model.set_enabled(i, false);
                                    }
                                    model.set_enabled(value, false);
                                }
                                let baseline = view.render(
                                    &mut Scene::new(),
                                    &mut text,
                                    &mut model,
                                    &labels,
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                let p = baseline.items[if has("selected") { value } else { 1 }]
                                    .frame
                                    .center();
                                if has("hover") {
                                    view.pointer_move(p);
                                }
                                if has("pressed") {
                                    assert_eq!(view.pointer_down(p), Some(1));
                                }
                                if has("focus") {
                                    view.focus(value);
                                }
                                let layout = view.render(
                                    &mut scene,
                                    &mut text,
                                    &mut model,
                                    &labels,
                                    bounds,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                assert_eq!(layout.items.len(), 3);
                                assert_eq!(model.selected, value);
                                if has("focus") {
                                    assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.border_width>0.)));
                                }
                                if has("disabled") {
                                    assert!((0..3).all(|i| !model.is_enabled(i)));
                                    assert!(view.pointer_down(p).is_none());
                                    assert!(
                                        !view
                                            .key_event(ControlKey::Space, false, &mut model)
                                            .changed
                                    );
                                    assert_eq!(model.selected, value);
                                }
                            }
                            _ => unreachable!(),
                        }
                        assert!(!scene.is_empty());
                    });
                    #[cfg(feature = "token-audit")]
                    if component == "tui.icon-button" {
                        for (family, token) in [
                            ("radius", "radius-control"),
                            ("size", "size-icon-button"),
                            ("size", "size-icon-standard"),
                        ] {
                            assert!(reads.iter().any(|(f, t)| f == family && t == token));
                        }
                        assert!(
                            !reads
                                .iter()
                                .any(|(f, t)| f == "radius" && t == "c-button-md-radius")
                        );
                    }
                    cases.push(serde_json::json!({"component":component,"theme":format!("{theme:?}"),"brand":brand,"action_state":state,"state_asserted":true,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 576);
    if let Ok(path) = std::env::var("TEKESUI_ACTION_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Production Button/IconButton/Link/Checkbox/SegmentedControl through public render/event APIs: all31 required states plus17 registered combinations, four themes, three existing brands. Label metrics are deterministic test host; real CoreText/GPU and native acceptance separate.","excluded":["compiled constants/aliases","baked text style accessors","font/icon backend internals","unexecuted paths","native input/AX"],"cases":cases})).unwrap()).unwrap();
    }
}

#[test]
fn segment_group_disable_preserves_model_and_cancels_all_armed_input() {
    for keyboard in [false, true] {
        let mut model = SegmentedControl::new(3);
        model.set_enabled(2, false);
        let mut view = SegmentedControlView::new(3);
        let frame = Rect::new(0., 0., 300., 40.);
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            &["One", "Two", "Three"],
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
        let point = view.layout().items[1].frame.center();
        if keyboard {
            view.focus(1);
            assert!(view.key_event(ControlKey::Space, true, &mut model).consumed);
        } else {
            assert_eq!(view.pointer_down(point), Some(1));
        }
        view.set_disabled(true);
        assert_eq!(model.selected, 0);
        assert!(!model.is_enabled(2));
        assert!(view.pointer_down(point).is_none());
        assert!(!view.select(1, &mut model));
        assert!(!view.key_event(ControlKey::Right, true, &mut model).consumed);
        assert!(view.pointer_up(point, &mut model).is_none());
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            &["One", "Two", "Three"],
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
        view.set_disabled(false);
        view.render(
            &mut Scene::new(),
            &mut Labels,
            &mut model,
            &["One", "Two", "Three"],
            frame,
            None,
            Theme::LIGHT.into(),
            proposal::TextRole::UiBody,
            false,
        );
        view.focus(1);
        assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
        assert!(view.pointer_up(point, &mut model).is_none());
        assert_eq!(model.selected, 0);
        assert!(!model.is_enabled(2));
        assert_eq!(view.pointer_down(point), Some(1));
        assert_eq!(view.pointer_up(point, &mut model), Some(1));
    }
}

#[test]
fn icon_chrome_uses_semantic_control_radius_and_default_icon_size() {
    let mut model = Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Create item");
    let frame = Rect::new(12., 12., 40., 40.);
    model.arrange(&mut Labels, frame, frame);
    model.paint(
        &mut Scene::new(),
        &mut Labels,
        frame,
        frame,
        Palette::from(Theme::LIGHT).into(),
    );
    model.event(&ContentEvent::PointerMove(frame.center()));
    let mut scene = Scene::new();
    model.paint(
        &mut scene,
        &mut Labels,
        frame,
        frame,
        Palette::from(Theme::LIGHT).into(),
    );
    let expected = proposal::metric("radius", "radius-control").unwrap();
    assert!(scene.primitives().iter().any(|p| matches!(p,
        tekes_ui_scene::Primitive::Quad(q) if q.rect==frame && q.corner_radii==tekes_ui_scene::CornerRadii::uniform(expected))),
        "IconButton chrome must use its declared radius-control token");
    let icon = scene
        .primitives()
        .iter()
        .find_map(|p| match p {
            tekes_ui_scene::Primitive::Image(image)
                if image.image == tekes_ui_scene::ImageId(1) =>
            {
                Some(image.rect)
            }
            _ => None,
        })
        .expect("real image Button must emit its original artwork");
    assert_eq!(icon.size, Size::new(16., 16.));
}

#[test]
fn icon_natural_sizes_are_token_owned_and_explicit_frames_remain_supported() {
    struct Typography(f32);
    impl ButtonTextRenderer for Typography {
        fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
            role.style().size * self.0
        }
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Labels.measure(value, role, width)
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
    // The approved unified Button rule replaces standalone compact/default
    // presets. Both natural size and artwork follow the same body typography.
    for scale in [1., 1.5] {
        let mut text = Typography(scale);
        let mut model = Button::new(tekes_ui_scene::ImageId(1)).with_accessible_name("Create item");
        let natural = model.measure(&mut text, Size::new(f32::MAX, f32::MAX)).size;
        let side = proposal::metric("size", "size-icon-standard").unwrap() * scale;
        let inset = proposal::metric("spacing", "space-relaxed").unwrap() * scale;
        let minimum = proposal::metric("size", "size-action-button").unwrap() * scale;
        assert_eq!(natural, Size::new(side + 2. * inset, minimum));
        for frame in [
            Rect::new(4., 6., natural.width, natural.height),
            Rect::new(12., 12., 56. * scale, 48. * scale),
        ] {
            model.arrange(&mut text, frame, frame);
            let mut scene = Scene::new();
            model.paint(
                &mut scene,
                &mut text,
                frame,
                frame,
                Palette::from(Theme::LIGHT).into(),
            );
            assert_eq!(model.button.layout().unwrap().frame, frame);
            let icon = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    tekes_ui_scene::Primitive::Image(image)
                        if image.image == tekes_ui_scene::ImageId(1) =>
                    {
                        Some(image.rect)
                    }
                    _ => None,
                })
                .expect("real unified Image Button must emit artwork");
            assert_eq!(icon.size, Size::new(side, side));
            assert_eq!(icon.center(), frame.center());
            assert!(
                model
                    .event(&ContentEvent::PointerDown(frame.center()))
                    .consumed
            );
            assert!(
                model
                    .event(&ContentEvent::PointerUp(frame.center()))
                    .activated
            );
        }
    }
}

#[test]
fn checkbox_unrelated_key_observes_unavailability_without_reviving_old_press() {
    use tekes_ui_controls::{CheckState, Checkbox};
    let keys = [
        ControlKey::Enter,
        ControlKey::Escape,
        ControlKey::Left,
        ControlKey::Tab { reverse: false },
        ControlKey::Tab { reverse: true },
    ];
    let mut cases = 0;
    for key in keys {
        for down in [false, true] {
            for pointer in [false, true] {
                for loading in [false, true] {
                    let mut checkbox = Checkbox::default();
                    checkbox.button.state.focused = true;
                    if pointer {
                        checkbox.button.pointer_down();
                    } else {
                        assert!(!checkbox.key(ControlKey::Space, true));
                    }
                    assert!(checkbox.button.state.pressed);
                    checkbox.button.state.disabled = !loading;
                    checkbox.button.state.loading = loading;
                    assert!(!checkbox.key(key, down));
                    assert!(
                        !checkbox.button.state.pressed,
                        "availability must be observed before key filtering: {key:?}/{down}/{pointer}/{loading}"
                    );
                    checkbox.button.state.disabled = false;
                    checkbox.button.state.loading = false;
                    assert!(!checkbox.button.pointer_up(true));
                    assert!(!checkbox.key(ControlKey::Space, false));
                    assert_eq!(checkbox.checked, CheckState::Unchecked);
                    assert!(!checkbox.key(ControlKey::Space, true));
                    assert!(checkbox.key(ControlKey::Space, false));
                    assert_eq!(checkbox.checked, CheckState::Checked);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 40);
}

fn assert_segmented_enabled_boundary(through_view: bool) {
    for count in 1..=5 {
        for mask in 1..(1_usize << count) {
            for start in 0..count {
                if mask & (1 << start) == 0 {
                    continue;
                }
                for key in [ControlKey::Home, ControlKey::End] {
                    let expected = if key == ControlKey::Home {
                        (0..count).find(|i| mask & (1 << i) != 0).unwrap()
                    } else {
                        (0..count).rev().find(|i| mask & (1 << i) != 0).unwrap()
                    };
                    let mut model = SegmentedControl::new(count);
                    for i in 0..count {
                        model.set_enabled(i, mask & (1 << i) != 0);
                    }
                    model.selected = start;
                    let changed = if through_view {
                        let mut view = SegmentedControlView::new(count);
                        let labels = vec!["Choice"; count];
                        view.render(
                            &mut Scene::new(),
                            &mut Labels,
                            &mut model,
                            &labels,
                            Rect::new(0., 0., 400., 40.),
                            None,
                            Theme::LIGHT.into(),
                            proposal::TextRole::UiBody,
                            false,
                        );
                        view.focus(start);
                        let event = view.key_event(key, true, &mut model);
                        assert!(event.consumed);
                        event.changed
                    } else {
                        model.key(key)
                    };
                    assert_eq!(
                        model.selected, expected,
                        "count={count},mask={mask:b},start={start},key={key:?},view={through_view}"
                    );
                    assert_eq!(changed, start != expected);
                    assert!(model.is_enabled(model.selected));
                }
            }
        }
    }
}

#[test]
fn segmented_home_end_choose_enabled_boundary_in_public_model() {
    assert_segmented_enabled_boundary(false);
}
#[test]
fn segmented_home_end_choose_enabled_boundary_in_actual_view() {
    assert_segmented_enabled_boundary(true);
}

#[test]
fn segmented_navigation_uses_actual_roving_focus_even_when_selection_is_unchanged() {
    for selected in 0..3 {
        for focused in 0..3 {
            for key in [
                ControlKey::Left,
                ControlKey::Right,
                ControlKey::Up,
                ControlKey::Down,
                ControlKey::Home,
                ControlKey::End,
            ] {
                let mut model = SegmentedControl::new(3);
                model.selected = selected;
                let mut view = SegmentedControlView::new(3);
                view.render(
                    &mut Scene::new(),
                    &mut Labels,
                    &mut model,
                    &["One", "Two", "Three"],
                    Rect::new(0., 0., 600., 48.),
                    None,
                    Theme::LIGHT.into(),
                    proposal::TextRole::UiBody,
                    false,
                );
                view.focus(focused);
                assert_eq!(view.focused(), Some(focused));
                assert_eq!(model.selected, selected);
                let expected = match key {
                    ControlKey::Left | ControlKey::Up => focused.saturating_sub(1),
                    ControlKey::Right | ControlKey::Down => (focused + 1).min(2),
                    ControlKey::Home => 0,
                    ControlKey::End => 2,
                    _ => unreachable!(),
                };
                let event = view.key_event(key, true, &mut model);
                assert!(event.consumed);
                assert_eq!(event.changed, selected != expected);
                assert_eq!(model.selected, expected);
                assert_eq!(view.focused(), Some(expected));
                view.set_disabled(true);
                assert_eq!(view.focused(), None);
                view.set_disabled(false);
                assert_eq!(view.focused(), None);
                view.focus(expected);
                view.set_window_active(false);
                assert_eq!(view.focused(), None);
                view.set_window_active(true);
                assert_eq!(view.focused(), None);
            }
        }
    }
}
