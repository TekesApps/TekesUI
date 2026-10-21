//! Public event-boundary availability regression; no paint required to observe disable.
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
use tekes_ui_controls::{
    ControlKey, RadioGroupView, SelectionEvent, SliderView, SwitchView,
    extended::{RadioGroup, Slider, Switch},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, _: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new(w, 24.)
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
fn slider_observed_disabled_events_release_old_drag_without_waiting_for_paint() {
    for event in 0..6 {
        let mut model = Slider::new(50., 0., 100., 5.).unwrap();
        let mut view = SliderView::new();
        let layout = view.render(
            &mut Scene::new(),
            model,
            Rect::new(10., 10., 220., 30.),
            None,
            Theme::LIGHT.into(),
        );
        assert!(view.pointer_down(layout.thumb.center(), &mut model));
        let before = model.value();
        model.disabled = true;
        match event {
            0 => {
                view.pointer_down(layout.thumb.center(), &mut model);
            }
            1 => {
                view.pointer_down(Point::new(-100., -100.), &mut model);
            }
            2 => {
                view.key(ControlKey::Right, true, &mut model);
            }
            3 => {
                view.key(ControlKey::Right, false, &mut model);
            }
            4 => {
                view.pointer_move(layout.thumb.center(), &mut model);
            }
            _ => {
                view.pointer_up(layout.thumb.center(), &model);
            }
        }
        assert!(
            !view.is_captured(),
            "observed disable event {event} must release capture"
        );
        model.disabled = false;
        view.pointer_move(
            Point::new(layout.track.max_x(), layout.track.center().y),
            &mut model,
        );
        assert_eq!(
            model.value(),
            before,
            "reenable cannot revive old drag {event}"
        );
    }
}
#[test]
fn switch_observed_disabled_pointer_cannot_revive_either_activation_owner() {
    for pointer in [false, true] {
        for event in 0..3 {
            let mut model = Switch::default();
            let mut view = SwitchView::new();
            let point = Point::new(20., 20.);
            view.render(
                &mut Scene::new(),
                model,
                Rect::new(10., 10., 48., 26.),
                None,
                Theme::LIGHT.into(),
            );
            view.focus(true);
            if pointer {
                assert!(view.pointer_down(point, &model));
            } else {
                view.key(ControlKey::Space, true, &mut model);
            }
            model.disabled = true;
            match event {
                0 => {
                    view.pointer_down(point, &model);
                }
                1 => {
                    view.pointer_down(Point::new(-100., -100.), &model);
                }
                _ => {
                    view.pointer_up(point, &mut model);
                }
            }
            model.disabled = false;
            assert_ne!(view.pointer_up(point, &mut model), SelectionEvent::Changed);
            assert_ne!(
                view.key(ControlKey::Space, false, &mut model),
                SelectionEvent::Changed
            );
            assert!(
                !model.checked,
                "disabled observation must cancel pointer={pointer},event={event}"
            );
        }
    }
}
#[test]
fn radio_observed_disabled_row_cancels_its_old_capture_on_pointer_and_key_boundaries() {
    for event in 0..4 {
        let mut model = RadioGroup::new(2);
        let mut view = RadioGroupView::new();
        let layout = view.render(
            &mut Scene::new(),
            &mut Text,
            &model,
            Rect::new(10., 10., 220., 0.),
            None,
            Palette::from(Theme::LIGHT),
            &["One", "Two"],
        );
        let point = layout.rows[1].frame().center();
        assert_eq!(view.pointer_down(point, &model), SelectionEvent::Handled);
        model.set_enabled(1, false);
        match event {
            0 => {
                view.pointer_down(point, &model);
            }
            1 => {
                view.pointer_down(Point::new(-100., -100.), &model);
            }
            2 => {
                view.key(ControlKey::Space, true, &mut model);
            }
            _ => {
                view.key(ControlKey::Space, false, &mut model);
            }
        }
        model.set_enabled(1, true);
        assert_ne!(
            view.pointer_up(point, &mut model),
            SelectionEvent::Changed,
            "old row capture event {event}"
        );
        assert_eq!(model.selected, Some(0));
    }
}

#[test]
fn switch_unrelated_key_observes_disabled_without_consuming_parent_keys() {
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
                let mut model = Switch::default();
                let mut view = SwitchView::new();
                let frame = Rect::new(10., 10., 48., 26.);
                view.render(&mut Scene::new(), model, frame, None, Theme::LIGHT.into());
                view.focus(true);
                if pointer {
                    assert!(view.pointer_down(frame.center(), &model));
                } else {
                    assert_eq!(
                        view.key(ControlKey::Space, true, &mut model),
                        SelectionEvent::Handled
                    );
                }
                model.disabled = true;
                assert_eq!(
                    view.key(key, down, &mut model),
                    SelectionEvent::Ignored,
                    "unrelated key remains available to the parent"
                );
                model.disabled = false;
                assert_ne!(
                    view.pointer_up(frame.center(), &mut model),
                    SelectionEvent::Changed,
                    "old pointer release must not revive after {key:?}/{down}/{pointer}"
                );
                assert_ne!(
                    view.key(ControlKey::Space, false, &mut model),
                    SelectionEvent::Changed,
                    "old Space release must not revive after {key:?}/{down}/{pointer}"
                );
                assert!(!model.checked);
                assert_eq!(
                    view.key(ControlKey::Space, true, &mut model),
                    SelectionEvent::Handled
                );
                assert_eq!(
                    view.key(ControlKey::Space, false, &mut model),
                    SelectionEvent::Changed
                );
                assert!(model.checked);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 20);
}

#[test]
fn radio_group_disabled_rejects_late_focus_requests_without_changing_selection() {
    for selected in [None, Some(0), Some(1)] {
        for requested in [None, Some(0), Some(1), Some(2), Some(99)] {
            for visible in [false, true] {
                let mut model = RadioGroup::new(3);
                model.selected = selected;
                model.set_enabled(2, false);
                let mut view = RadioGroupView::new();
                view.set_disabled(true);
                view.focus(&model, requested, visible);
                assert_eq!(
                    view.focused(),
                    None,
                    "disabled focus: selected={selected:?},requested={requested:?},visible={visible}"
                );
                assert_eq!(model.selected, selected);
                view.set_disabled(false);
                assert_eq!(
                    view.focused(),
                    None,
                    "reenable never revives a rejected focus request"
                );
            }
        }
    }
}
#[test]
fn radio_late_disabled_focus_does_not_enable_keyboard_selection_after_restore() {
    let mut model = RadioGroup::new(2);
    let mut view = RadioGroupView::new();
    let layout = view.render(
        &mut Scene::new(),
        &mut Text,
        &model,
        Rect::new(10., 10., 220., 0.),
        None,
        Theme::LIGHT.into(),
        &["One", "Two"],
    );
    view.focus(&model, Some(0), true);
    view.pointer_down(layout.rows[1].frame().center(), &model);
    view.set_disabled(true);
    view.focus(&model, Some(1), true);
    view.set_disabled(false);
    assert_eq!(
        view.pointer_up(layout.rows[1].frame().center(), &mut model),
        SelectionEvent::Ignored
    );
    assert_eq!(
        view.key(ControlKey::Space, true, &mut model),
        SelectionEvent::Ignored,
        "late disabled focus must not revive keyboard selection"
    );
    assert_eq!(model.selected, Some(0));
    view.focus(&model, Some(1), true);
    assert_eq!(
        view.key(ControlKey::Space, true, &mut model),
        SelectionEvent::Changed
    );
    assert_eq!(model.selected, Some(1));
}
