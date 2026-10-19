//! Public navigation contracts: latest caller flags and component-owned input.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, extended::Disclosure,
    layout_view::DisclosureView,
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
fn disclosure_latest_disabled_key_down_cannot_arm_a_later_enabled_release() {
    let mut model = Disclosure::default();
    let mut view = DisclosureView::default();
    view.render(
        &mut Scene::new(),
        &mut Labels,
        &model,
        "Details",
        Rect::new(0., 0., 240., 40.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    view.focus(true);
    model.disabled = true;
    assert!(
        !view.key_event(ControlKey::Space, true, &mut model).consumed,
        "latest caller availability must be consulted before arming"
    );
    model.disabled = false;
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    assert!(!model.open);
}
#[test]
fn disclosure_observed_disabled_interval_cancels_prior_space_arm() {
    let mut model = Disclosure::default();
    let mut view = DisclosureView::default();
    view.render(
        &mut Scene::new(),
        &mut Labels,
        &model,
        "Details",
        Rect::new(0., 0., 240., 40.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    view.focus(true);
    assert!(view.key_event(ControlKey::Space, true, &mut model).consumed);
    model.disabled = true;
    view.key_event(ControlKey::Escape, true, &mut model);
    model.disabled = false;
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    assert!(!model.open);
}
#[test]
fn disclosure_chrome_uses_its_existing_row_radius_token() {
    let mut view = DisclosureView::default();
    let model = Disclosure::default();
    let mut scene = Scene::new();
    let frame = Rect::new(0., 0., 240., 40.);
    view.render(
        &mut scene,
        &mut Labels,
        &model,
        "Details",
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    let radius = proposal::metric("radius", "radius-row").unwrap();
    assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==frame && q.corner_radii==tekes_ui_scene::CornerRadii::uniform(radius))));
}

#[test]
fn breadcrumb_group_availability_and_public_index_boundaries_preserve_the_path() {
    use tekes_ui_controls::{extended::Breadcrumb, layout_view::BreadcrumbView};
    let model = Breadcrumb {
        items: vec!["Home".into(), "Library".into(), "Current".into()],
    };
    let mut view = BreadcrumbView::default();
    let frame = Rect::new(0., 0., 480., 40.);
    view.render(
        &mut Scene::new(),
        &mut Labels,
        &model,
        &["Home", "Library", "Current"],
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    let point = view.layout().items[0].frame().center();
    assert_eq!(view.pointer_down(point), Some(0));
    view.set_disabled(true);
    assert_eq!(view.pointer_up(point, &model), None);
    assert_eq!(view.activate(0, &model), None);
    view.set_disabled(false);
    view.render(
        &mut Scene::new(),
        &mut Labels,
        &model,
        &["Home", "Library", "Current"],
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    assert_eq!(view.pointer_up(point, &model), None);
    view.focus(0);
    assert_eq!(
        view.key_event(ControlKey::Space, true, &model),
        (false, None)
    );
    assert_eq!(
        view.key_event(ControlKey::Enter, true, &model),
        (true, Some(0))
    );
    assert_eq!(view.activate(usize::MAX, &model), None);
    view.focus(usize::MAX);
    assert_eq!(model.activate(usize::MAX), None);
    assert_eq!(model.items.len(), 3);
}

#[test]
fn navigation_header_states_use_public_events_and_existing_painters() {
    use tekes_ui_controls::{TabsView, extended::Breadcrumb, layout_view::BreadcrumbView};
    let registry:serde_json::Value=serde_json::from_str(include_str!("../../../design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/design/registry/components.json")).unwrap();
    let mut cases = Vec::new();
    let mut deferred = Vec::new();
    for component in ["tui.disclosure", "tui.breadcrumb", "tui.tabs"] {
        let spec = registry["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == component)
            .unwrap();
        let names = spec["states"]
            .as_array()
            .unwrap()
            .iter()
            .chain(spec["state_combinations"].as_array().unwrap().iter());
        for name in names {
            let state = name.as_str().unwrap();
            if state
                .split('+')
                .any(|s| matches!(s, "loading" | "error" | "empty"))
            {
                deferred.push(serde_json::json!({"component":component,"state":state,"reason":"content status is not a header flag; body/consumer contracts remain unverified"}));
                continue;
            }
            let has = |value: &str| state.split('+').any(|s| s == value);
            for theme in Theme::ALL {
                for brand in 0..3 {
                    let palette = Palette::new(theme, brand).unwrap();
                    let mut scene = Scene::new();
                    let expected_quiet_ground = palette.color("surface-2").unwrap();
                    let run = || {
                        let mut text = Labels;
                        let mut baseline_scene = Scene::new();
                        match component {
                            "tui.disclosure" => {
                                let mut model = Disclosure {
                                    open: has("expanded"),
                                    disabled: has("disabled"),
                                };
                                let mut view = DisclosureView::default();
                                let frame = Rect::new(8., 8., 240., 40.);
                                let baseline = view.render(
                                    &mut baseline_scene,
                                    &mut text,
                                    &model,
                                    "Details",
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                if has("hover") {
                                    view.pointer_move(frame.center());
                                }
                                if has("pressed") {
                                    assert!(view.pointer_down(frame.center()));
                                }
                                if has("focus") {
                                    view.focus(true);
                                }
                                let layout = view.render(
                                    &mut scene,
                                    &mut text,
                                    &model,
                                    "Details",
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                if state == "rest" {
                                    assert!(scene.primitives().iter().any(|p|matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==layout.frame && q.fill==expected_quiet_ground && q.border_width==0.)),
                                        "Disclosure is a quiet expansion action, not primary action chrome");
                                }

                                let body = view.layout_body(
                                    &mut text,
                                    &model,
                                    Rect::new(8., 48., 240., 80.),
                                    None,
                                );
                                assert_eq!(body.interactive, has("expanded"));
                                assert_eq!(
                                    body.occupied_height,
                                    if has("expanded") { 80. } else { 0. }
                                );
                                if has("disabled") {
                                    assert!(!view.pointer_down(frame.center()));
                                    assert!(
                                        !view
                                            .key_event(ControlKey::Enter, true, &mut model)
                                            .changed
                                    );
                                }
                                assert_eq!(model.open, has("expanded"));
                            }
                            "tui.breadcrumb" => {
                                let model = Breadcrumb {
                                    items: vec!["Home".into(), "Library".into(), "Current".into()],
                                };
                                let mut view = BreadcrumbView::default();
                                let frame = Rect::new(8., 8., 480., 40.);
                                let labels = ["Home", "Library", "Current"];
                                view.set_disabled(has("disabled"));
                                let baseline = view.render(
                                    &mut baseline_scene,
                                    &mut text,
                                    &model,
                                    &labels,
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                let point = baseline.items[0].frame().center();
                                if has("hover") {
                                    view.pointer_move(point);
                                }
                                if has("pressed") {
                                    assert_eq!(view.pointer_down(point), Some(0));
                                }
                                if has("focus") {
                                    view.focus(0);
                                }
                                let layout = view.render(
                                    &mut scene,
                                    &mut text,
                                    &model,
                                    &labels,
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                assert!(
                                    view.pointer_down(layout.items[2].frame().center())
                                        .is_none()
                                );
                                assert!(view.activate(2, &model).is_none());
                                if has("disabled") {
                                    assert!(view.pointer_down(point).is_none());
                                    assert_eq!(
                                        view.key_event(ControlKey::Enter, true, &model),
                                        (false, None)
                                    );
                                }
                                assert_eq!(model.items[2], "Current");
                            }
                            "tui.tabs" => {
                                let mut view = TabsView::new(3);
                                let labels = ["General", "Advanced", "Security"];
                                let frame = Rect::new(8., 8., 480., 40.);
                                if has("selected") {
                                    assert!(view.select(1));
                                }
                                view.set_disabled(has("disabled"));
                                let baseline = view.render(
                                    &mut baseline_scene,
                                    &mut text,
                                    &labels,
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                let selected = view.selected();
                                let point = baseline.items[selected].frame.center();
                                if has("hover") {
                                    view.pointer_move_at(point);
                                }
                                if has("pressed") {
                                    assert_eq!(view.pointer_down_at(point), Some(selected));
                                }
                                if has("focus") {
                                    assert!(view.focus(selected, true));
                                }
                                let layout = view.render(
                                    &mut scene,
                                    &mut text,
                                    &labels,
                                    frame,
                                    None,
                                    palette,
                                    proposal::TextRole::UiBody,
                                    false,
                                );
                                assert_eq!(layout.frame, baseline.frame);
                                assert_eq!(view.selected(), selected);
                                if has("disabled") {
                                    assert!(view.pointer_down_at(point).is_none());
                                    assert!(view.key(ControlKey::Right, true).is_none());
                                }
                            }
                            _ => unreachable!(),
                        }
                        assert!(!scene.is_empty());
                        if has("hover") || has("pressed") || has("focus") {
                            assert_ne!(
                                format!("{:?}", scene.primitives()),
                                format!("{:?}", baseline_scene.primitives()),
                                "{component}/{state} must change actual production paint after public input"
                            );
                        }
                    };
                    #[cfg(feature = "token-audit")]
                    let (_, reads) = tekes_ui_tokens::audit::capture(run);
                    #[cfg(not(feature = "token-audit"))]
                    let reads: Vec<(String, String)> = {
                        let mut run = run;
                        run();
                        Vec::new()
                    };
                    #[cfg(feature = "token-audit")]
                    if component == "tui.disclosure" {
                        assert!(
                            reads
                                .iter()
                                .any(|(f, t)| f == "radius" && t == "radius-row")
                        );
                        assert!(
                            !reads
                                .iter()
                                .any(|(f, t)| f == "radius" && t == "c-button-md-radius")
                        );
                    }
                    cases.push(serde_json::json!({"component":component,"navigation_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 288);
    assert_eq!(deferred.len(), 10);
    if let Ok(path) = std::env::var("TEKESUI_NAVIGATION_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Production Disclosure/Breadcrumb/Tabs header states:17 required states and7 applicable combinations; four themes and three existing brands.10 content-status requirements/combinations remain deferred, not reinterpreted as header color flags.","excluded":["content loading/error/empty contracts","native input/AX","compiled aliases","font backend internals","unexecuted paths"],"deferred":deferred,"cases":cases})).unwrap()).unwrap();
    }
}
