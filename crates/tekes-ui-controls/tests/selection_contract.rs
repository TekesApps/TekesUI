//! Independent public API probe: production Views, real event methods and Scene
//! primitives. Optional token observations are scoped lookups, not native UAT.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, CheckState, Checkbox, ControlKey, RadioGroupView,
    SelectionEvent, SwitchView,
    extended::{RadioGroup, Switch},
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Labels;
impl ButtonTextRenderer for Labels {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 8.).min(width),
            role.style().line_height,
        )
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
fn observe<T>(render: impl FnOnce() -> T) -> (T, Vec<(String, String)>) {
    #[cfg(feature = "token-audit")]
    {
        tekes_ui_tokens::audit::capture(render)
    }
    #[cfg(not(feature = "token-audit"))]
    {
        (render(), Vec::new())
    }
}
#[test]
fn selection_validation_preserves_geometry_value_focus_and_existing_disabled_roles() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in [0, 1, 2] {
            let palette = Palette { theme, brand };
            for component in ["tui.checkbox", "tui.radio-group", "tui.switch"] {
                for value in 0..if component == "tui.checkbox" { 3 } else { 2 } {
                    for error in [false, true] {
                        for disabled in [false, true] {
                            for focused in [false, true] {
                                for phase in 0..3 {
                                    let bounds = Rect::new(16., 18., 240., 44.);
                                    let ((scene, frame, expected), reads) = observe(|| {
                                        let mut scene = Scene::new();
                                        let mut text = Labels;
                                        let expected;
                                        let frame;
                                        match component {
                                            "tui.checkbox" => {
                                                let mut control = Checkbox {
                                                    checked: [
                                                        CheckState::Unchecked,
                                                        CheckState::Checked,
                                                        CheckState::Mixed,
                                                    ][value],
                                                    ..Default::default()
                                                };
                                                let original = control.checked;
                                                control.button.state.disabled = disabled;
                                                let baseline = control.render(
                                                    &mut Scene::new(),
                                                    &mut text,
                                                    "Choice",
                                                    bounds,
                                                    None,
                                                    palette,
                                                    proposal::TextRole::UiBody,
                                                );
                                                control.button.state.error = error;
                                                control.button.state.disabled = disabled;
                                                control.button.state.hovered = phase >= 1;
                                                control.button.state.pressed = phase == 2;
                                                control.button.state.focused = focused;
                                                control.button.state.focus_visible = focused;
                                                let layout = control.render(
                                                    &mut scene,
                                                    &mut text,
                                                    "Choice",
                                                    bounds,
                                                    None,
                                                    palette,
                                                    proposal::TextRole::UiBody,
                                                );
                                                assert_eq!(baseline.frame, layout.frame);
                                                assert_eq!(baseline.control, layout.control);
                                                frame = layout.control;
                                                expected =
                                                    if error { "status-failed" } else { "border" };
                                                assert_eq!(control.checked, original);
                                            }
                                            "tui.radio-group" => {
                                                let mut model = RadioGroup::new(1);
                                                model.selected = (value == 1).then_some(0);
                                                let original = model.selected;
                                                let mut view = RadioGroupView::new();
                                                view.set_disabled(disabled);
                                                let layout = view.render(
                                                    &mut Scene::new(),
                                                    &mut text,
                                                    &model,
                                                    bounds,
                                                    None,
                                                    palette,
                                                    &["Choice"],
                                                );
                                                view.set_error(error);
                                                let point = layout.rows[0].frame().center();
                                                if phase >= 1 {
                                                    view.pointer_move(point);
                                                }
                                                if phase == 2 {
                                                    view.pointer_down(point, &model);
                                                }
                                                if focused {
                                                    view.focus(&model, Some(0), true);
                                                }
                                                let next = view.render(
                                                    &mut scene,
                                                    &mut text,
                                                    &model,
                                                    bounds,
                                                    None,
                                                    palette,
                                                    &["Choice"],
                                                );
                                                assert_eq!(layout, next);
                                                frame = next.indicators[0];
                                                expected = if error {
                                                    "status-failed"
                                                } else if disabled {
                                                    "disabled-text"
                                                } else {
                                                    "border"
                                                };
                                                assert_eq!(model.selected, original);
                                                assert_eq!(view.error(), error);
                                            }
                                            _ => {
                                                let model = Switch {
                                                    checked: value == 1,
                                                    disabled,
                                                };
                                                let mut view = SwitchView::new();
                                                let layout = view.render_labeled(
                                                    &mut Scene::new(),
                                                    &mut text,
                                                    model,
                                                    bounds,
                                                    None,
                                                    palette,
                                                    "Choice",
                                                );
                                                view.set_error(error);
                                                let point = layout.toggle.frame().center();
                                                if phase >= 1 {
                                                    view.pointer_move(point);
                                                }
                                                if phase == 2 {
                                                    view.pointer_down(point, &model);
                                                }
                                                if focused {
                                                    view.focus(true);
                                                }
                                                let next = view.render_labeled(
                                                    &mut scene, &mut text, model, bounds, None,
                                                    palette, "Choice",
                                                );
                                                assert_eq!(layout, next);
                                                frame = next.toggle.frame();
                                                expected = if error {
                                                    "status-failed"
                                                } else if disabled {
                                                    "disabled-text"
                                                } else {
                                                    "border"
                                                };
                                                assert_eq!(view.error(), error);
                                            }
                                        }
                                        (scene, frame, expected)
                                    });
                                    let quads: Vec<_> = scene
                                        .primitives()
                                        .iter()
                                        .filter_map(|p| {
                                            if let Primitive::Quad(q) = p {
                                                Some(q)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    let boundary = quads
                                        .iter()
                                        .find(|q| q.rect == frame)
                                        .expect("production control boundary");
                                    assert_eq!(
                                        boundary.border_color,
                                        palette.color(expected).unwrap(),
                                        "{component} / {theme:?}"
                                    );
                                    let rings = quads
                                        .iter()
                                        .filter(|q| {
                                            q.fill.a == 0.
                                                && q.border_color
                                                    == palette.color("border-focused").unwrap()
                                        })
                                        .count();
                                    assert_eq!(
                                        rings,
                                        usize::from(focused && !disabled),
                                        "{component} focus remains independent of validation"
                                    );
                                    cases.push(serde_json::json!({"component":component,"theme":format!("{theme:?}"),"brand":brand,"value":value,"error":error,"disabled":disabled,"focus":focused,"phase":phase,"lookups":reads}));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 2016);
    if let Some(path) = std::env::var_os("TEKESUI_SELECTION_AUDIT_OUT") {
        #[cfg(not(feature = "token-audit"))]
        {
            let _ = path;
            panic!("explicit token-audit feature required");
        }
        #[cfg(feature = "token-audit")]
        std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","scope":"actual production selection Views rendered through public APIs; logical geometry; four themes, three existing brand presets; isolated per component probe including baseline and final render","complete":false,"excluded":"constants, baked alias expansion, motion/layout accessors, host font backend, and unexecuted branches are not complete dependency coverage","cases":cases})).unwrap()).unwrap();
    }
}
#[test]
fn validation_does_not_block_activation_or_erase_on_blur_disable_and_clear() {
    let mut view = SwitchView::new();
    let mut model = Switch {
        checked: false,
        disabled: false,
    };
    view.set_error(true);
    view.render(
        &mut Scene::new(),
        model,
        Rect::new(0., 0., 48., 26.),
        None,
        Theme::LIGHT.into(),
    );
    view.focus(true);
    view.key(ControlKey::Space, true, &mut model);
    assert_eq!(
        view.key(ControlKey::Space, false, &mut model),
        SelectionEvent::Changed
    );
    assert!(model.checked && view.error());
    view.blur();
    assert!(view.error());
    model.disabled = true;
    view.render(
        &mut Scene::new(),
        model,
        Rect::new(0., 0., 48., 26.),
        None,
        Theme::LIGHT.into(),
    );
    assert!(!view.pointer_down(tekes_ui_scene::Point::new(10., 10.), &model));
    assert!(model.checked && view.error());
    view.set_error(false);
    assert!(!view.error());
    let mut radio = RadioGroup::new(2);
    let mut view = RadioGroupView::new();
    view.set_error(true);
    view.render(
        &mut Scene::new(),
        &mut Labels,
        &radio,
        Rect::new(0., 0., 200., 100.),
        None,
        Theme::LIGHT.into(),
        &["A", "B"],
    );
    view.focus(&radio, Some(0), true);
    view.key(ControlKey::Right, true, &mut radio);
    assert_eq!(radio.selected, Some(1));
    view.blur();
    view.set_disabled(true);
    assert!(view.error());
    view.set_error(false);
    assert_eq!(radio.selected, Some(1));
}
