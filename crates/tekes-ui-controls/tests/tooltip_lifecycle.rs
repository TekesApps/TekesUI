use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::Tooltip,
    feedback_view::{TooltipEvent, TooltipView},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
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
fn anchor() -> Rect {
    Rect::new(20., 20., 100., 30.)
}
fn model(visible: bool) -> Tooltip {
    Tooltip {
        visible,
        text: "Help · 说明".into(),
    }
}
fn draw(view: &mut TooltipView, model: &Tooltip) -> bool {
    view.render(
        &mut Scene::new(),
        &mut Text,
        model,
        anchor(),
        Size::new(150., 48.),
        Rect::new(0., 0., 240., 160.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiCaption,
    )
    .visible
    .is_some()
}

#[test]
fn inactive_hover_and_focus_never_rearm_a_tooltip_on_reactivation() {
    for focus in [false, true] {
        let mut view = TooltipView::default();
        let model = model(false);
        assert!(!draw(&mut view, &model));
        view.set_window_active(false);
        if focus {
            view.set_focus(true, true);
        } else {
            view.pointer_move(anchor().center());
        }
        assert!(!draw(&mut view, &model));
        view.set_window_active(true);
        assert!(
            !draw(&mut view, &model),
            "inactive input must not become future visibility"
        );
        if focus {
            view.set_focus(true, true);
        } else {
            view.pointer_move(anchor().center());
        }
        assert!(
            draw(&mut view, &model),
            "fresh active input must remain usable"
        );
    }
}

#[test]
fn inactive_escape_does_not_suppress_an_explicit_controlled_visibility_request() {
    let mut view = TooltipView::default();
    let mut model = model(true);
    assert!(draw(&mut view, &model));
    view.set_window_active(false);
    assert_eq!(
        view.key(ControlKey::Escape),
        TooltipEvent::None,
        "inactive keys cannot dismiss a controlled owner"
    );
    view.set_window_active(true);
    assert!(
        draw(&mut view, &model),
        "the explicit model request survives without stale Escape suppression"
    );
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
    assert!(!draw(&mut view, &model));
    model.visible = false;
    assert!(!draw(&mut view, &model));
    model.visible = true;
    assert!(draw(&mut view, &model));
}

#[test]
fn deactivation_immediately_invalidates_overlay_and_hover_geometry_before_repaint() {
    let mut view = TooltipView::default();
    let model = model(false);
    draw(&mut view, &model);
    view.pointer_move(anchor().center());
    assert!(draw(&mut view, &model));
    let old = view.layout().frame.center();
    view.set_window_active(false);
    assert!(
        view.layout().visible.is_none(),
        "published hit/paint availability must end immediately"
    );
    assert!(!view.layout().hit_test(old));
    view.set_window_active(true);
    view.pointer_move(old);
    assert!(
        !draw(&mut view, &model),
        "stale tooltip/anchor geometry cannot manufacture a fresh hover"
    );
    view.pointer_move(Point::new(-10., -10.));
    view.pointer_move(anchor().center());
    assert!(draw(&mut view, &model));
}
