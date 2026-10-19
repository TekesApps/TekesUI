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
