//! Existing bounded stepper state must follow the current numeric model.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, NumberField,
    feedback_view::{NumberFieldEvent, NumberFieldView},
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

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
fn paint(view: &mut NumberFieldView, model: &NumberField) {
    view.render(
        &mut Scene::new(),
        &mut Labels,
        model,
        Rect::new(0., 0., 198., 36.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
}

#[test]
fn model_moving_away_from_a_bound_enables_keyboard_without_another_paint() {
    for increment in [false, true] {
        for key in [ControlKey::Enter, ControlKey::Space] {
            let mut model = NumberField::new(if increment { 4. } else { 0. }, 0., 4., 1.).unwrap();
            let mut view = NumberFieldView::default();
            paint(&mut view, &model);
            view.focus(increment, true);
            assert!(model.set(2.));
            let down = view.key_event(&mut model, key, true);
            let result = if key == ControlKey::Space {
                assert_eq!(down, NumberFieldEvent::None);
                view.key_event(&mut model, key, false)
            } else {
                down
            };
            assert_eq!(
                result,
                NumberFieldEvent::Changed,
                "latest bound, direction={increment}, key={key:?}"
            );
            assert_eq!(model.value(), if increment { 3. } else { 1. });
        }
    }
}

#[test]
fn crossing_a_model_bound_cancels_space_before_the_model_is_reenabled() {
    for increment in [false, true] {
        let mut model = NumberField::new(2., 0., 4., 1.).unwrap();
        let mut view = NumberFieldView::default();
        paint(&mut view, &model);
        view.focus(increment, true);
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, true),
            NumberFieldEvent::None
        );
        assert!(model.set(if increment { 4. } else { 0. }));
        // An observed repeat while the requested direction is unavailable
        // must invalidate the old arm, even without a render between events.
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, true),
            NumberFieldEvent::None
        );
        assert!(model.set(2.));
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, false),
            NumberFieldEvent::None
        );
        assert_eq!(model.value(), 2.);
    }
}

#[test]
fn overall_disabled_stepper_rejects_new_focus_and_keyboard() {
    let mut model = NumberField::new(2., 0., 4., 1.).unwrap();
    let mut view = NumberFieldView::default();
    paint(&mut view, &model);
    view.set_disabled(true);
    for increment in [false, true] {
        view.focus(increment, true);
        assert_eq!(view.focused_index(), None);
        assert_eq!(
            view.key_event(&mut model, ControlKey::Enter, true),
            NumberFieldEvent::None
        );
        assert_eq!(model.value(), 2.);
    }
}

#[test]
fn an_own_activation_reaching_a_bound_also_invalidates_an_older_space_arm() {
    for arrow in [false, true] {
        let mut model = NumberField::new(3., 0., 4., 1.).unwrap();
        let mut view = NumberFieldView::default();
        paint(&mut view, &model);
        view.focus(true, true);
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, true),
            NumberFieldEvent::None
        );
        let changed = if arrow {
            view.key(&mut model, ControlKey::Up)
        } else {
            view.key_event(&mut model, ControlKey::Enter, true)
        };
        assert_eq!(changed, NumberFieldEvent::Changed);
        assert_eq!(model.value(), 4.);
        assert!(model.set(2.));
        assert_eq!(
            view.key_event(&mut model, ControlKey::Space, false),
            NumberFieldEvent::None
        );
        assert_eq!(model.value(), 2.);
    }
}

#[test]
fn number_field_existing_states_use_real_view_tokens_without_changing_the_model() {
    let mut cases = Vec::new();
    for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
        for brand in 0..3 {
            for state in ["rest", "hover", "pressed", "focus", "disabled"] {
                let mut model = NumberField::new(2., 0., 4., 1.).unwrap();
                let mut view = NumberFieldView::default();
                let frame = Rect::new(16., 16., 198., 40.);
                let palette = tekes_ui_tokens::Palette::new(theme, brand).unwrap();
                let layout = view.render(
                    &mut Scene::new(),
                    &mut Labels,
                    &model,
                    frame,
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                );
                let p = layout.increment.frame.center();
                match state {
                    "hover" => view.pointer_move(p),
                    "pressed" => assert_eq!(view.pointer_down(p), Some(true)),
                    "focus" => view.focus(true, true),
                    "disabled" => view.set_disabled(true),
                    _ => {}
                }
                let mut scene = Scene::new();
                #[cfg(feature = "token-audit")]
                let (layout, reads) = tekes_ui_tokens::audit::capture(|| {
                    view.render(
                        &mut scene,
                        &mut Labels,
                        &model,
                        frame,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                    )
                });
                #[cfg(not(feature = "token-audit"))]
                let (layout, reads) = (
                    view.render(
                        &mut scene,
                        &mut Labels,
                        &model,
                        frame,
                        None,
                        palette,
                        proposal::TextRole::UiBody,
                    ),
                    Vec::<(String, String)>::new(),
                );
                assert_eq!(layout.frame, frame);
                assert_eq!(model.value(), 2.);
                let quads = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| {
                        if let tekes_ui_scene::Primitive::Quad(q) = p {
                            Some(q)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let increment = quads
                    .iter()
                    .find(|q| q.rect == layout.increment.frame)
                    .unwrap();
                assert_eq!(
                    increment.fill,
                    palette
                        .color(match state {
                            "hover" => "button-hover",
                            "pressed" => "button-pressed",
                            _ => "surface-2",
                        })
                        .unwrap()
                );
                if state == "focus" {
                    assert!(quads.len() > 2, "independent focus ink");
                }
                if state == "disabled" {
                    assert_eq!(view.pointer_down(p), None);
                }
                assert_eq!(
                    view.focused_index(),
                    if state == "focus" { Some(true) } else { None }
                );
                view.cancel();
                assert_eq!(view.pointer_up(p, &mut model), NumberFieldEvent::None);
                assert_eq!(model.value(), 2.);
                cases.push(serde_json::json!({"component":"tui.number-field","action_state":state,"state_asserted":true,"theme":theme_index,"brand":brand,"lookups":reads}));
            }
        }
    }
    assert_eq!(cases.len(), 60);
    if let Some(path) = std::env::var_os("TEKESUI_NUMBER_STATE_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"complete":false,"cases":cases,"excluded":["font backend","compiled constants","unexecuted paths","native and VoiceOver acceptance"]})).unwrap()).unwrap();
    }
}
