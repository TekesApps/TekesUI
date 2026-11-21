//! Independent public consumer: semantic edits must use the retained view's availability.
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{AccessibilityAction as A, ComponentId, Content, ContentHost, SliderContent},
    extended::Slider,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct NoText;
impl ButtonTextRenderer for NoText {
    fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
        Size::default()
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
fn semantic_range_updates_require_a_current_visible_active_mount() {
    const SLIDER: ComponentId = ComponentId(1);
    let mut host = ContentHost::new(Content::new(
        SLIDER,
        SliderContent::new("Volume", Slider::new(50., 0., 100., 5.).unwrap()),
    ))
    .unwrap();
    let value = |h: &ContentHost| {
        h.get(SLIDER)
            .unwrap()
            .component::<SliderContent>()
            .unwrap()
            .value()
    };
    assert!(
        !host
            .accessibility_action(SLIDER, None, A::Increment)
            .consumed
    );
    assert_eq!(value(&host), 50.);
    let frame = Rect::new(10., 10., 240., 36.);
    let mut text = NoText;
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(
        host.accessibility_action(SLIDER, None, A::Increment)
            .consumed
    );
    assert_eq!(value(&host), 55.);
    host.set_window_active(false);
    assert!(
        !host
            .accessibility_action(SLIDER, None, A::SetValue("80".into()))
            .consumed
    );
    assert_eq!(value(&host), 55.);
    host.set_window_active(true);
    assert!(
        host.accessibility_action(SLIDER, None, A::Decrement)
            .consumed
    );
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        Rect::new(500., 500., 20., 20.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(
        !host
            .accessibility_action(SLIDER, None, A::Increment)
            .consumed
    );
    assert_eq!(value(&host), 50.);
}

#[test]
fn retained_semantic_updates_share_steps_bounds_and_cancel_only_valid_gestures() {
    let mut model = Slider::new(50., 0., 100., 5.).unwrap();
    let mut view = tekes_ui_controls::layout_view::SliderView::new();
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
