//! Independent public consumer: semantic edits must use the retained view's availability.
use tekes_ui_controls::{
    composition::{AccessibilityAction as A, Component, SliderContent},
    extended::Slider,
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn semantic_range_updates_require_a_current_visible_active_mount() {
    let mut control = SliderContent::new("Volume", Slider::new(50., 0., 100., 5.).unwrap());
    assert!(!control.accessibility_action(None, &A::Increment).consumed);
    assert_eq!(control.model.value(), 50.);
    let frame = Rect::new(10., 10., 240., 36.);
    control.view.render(
        &mut Scene::new(),
        control.model,
        frame,
        Some(frame),
        Palette::from(Theme::LIGHT),
    );
    assert!(control.accessibility_action(None, &A::Increment).consumed);
    assert_eq!(control.model.value(), 55.);
    control.view.set_window_active(false);
    assert!(
        !control
            .accessibility_action(None, &A::SetValue("80".into()))
            .consumed
    );
    assert_eq!(control.model.value(), 55.);
    control.view.set_window_active(true);
    assert!(control.accessibility_action(None, &A::Decrement).consumed);
    control.view.render(
        &mut Scene::new(),
        control.model,
        frame,
        Some(Rect::new(500., 500., 20., 20.)),
        Palette::from(Theme::LIGHT),
    );
    assert!(!control.accessibility_action(None, &A::Increment).consumed);
    assert_eq!(control.model.value(), 50.);
}

#[test]
fn retained_semantic_updates_share_steps_bounds_and_cancel_only_valid_gestures() {
    let mut model = Slider::new(50., 0., 100., 5.).unwrap();
    let mut view = tekes_ui_controls::SliderView::new();
    let frame = Rect::new(10., 10., 240., 36.);
    let layout = view.render(
        &mut Scene::new(),
        model,
        frame,
        Some(frame),
        Palette::from(Theme::LIGHT),
    );
    assert!(view.pointer_down(layout.thumb.center(), &mut model));
    assert_eq!(view.set_value(f64::NAN, &mut model), None);
    assert!(view.is_captured());
    assert_eq!(view.set_value(63., &mut model), Some(true));
    assert_eq!(model.value(), 65.);
    assert!(!view.is_captured());
    assert!(!view.pointer_move(tekes_ui_scene::Point::new(900., 28.), &mut model));
    assert_eq!(model.value(), 65.);
    assert_eq!(view.set_value(999., &mut model), Some(true));
    assert_eq!(view.adjust_value(true, &mut model), Some(false));
    assert_eq!(model.value(), 100.);
    model.disabled = true;
    assert_eq!(view.adjust_value(false, &mut model), None);
    assert!(!view.is_focused());
    assert_eq!(model.value(), 100.);
}
