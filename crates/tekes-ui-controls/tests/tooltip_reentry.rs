use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey,
    extended::Tooltip,
    feedback_view::{TooltipEvent, TooltipView},
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (text.chars().count() as f32 * 7.).min(width),
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
fn anchor() -> Rect {
    Rect::new(20., 20., 100., 30.)
}
fn draw(view: &mut TooltipView) -> bool {
    draw_controlled(view, false)
}
fn draw_controlled(view: &mut TooltipView, visible: bool) -> bool {
    view.render(
        &mut Scene::new(),
        &mut Text,
        &Tooltip {
            text: "Help · 说明".into(),
            visible,
        },
        anchor(),
        Size::new(160., 48.),
        Rect::new(0., 0., 300., 180.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiCaption,
    )
    .visible
    .is_some()
}
fn visible_both_sources() -> TooltipView {
    let mut view = TooltipView::default();
    draw(&mut view);
    view.pointer_move(anchor().center());
    view.set_focus(true, true);
    assert!(draw(&mut view));
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
    assert!(!draw(&mut view));
    view.pointer_move(anchor().center());
    view.set_focus(true, true);
    assert!(
        !draw(&mut view),
        "duplicate active owner notifications cannot defeat Escape"
    );
    view
}
#[test]
fn fresh_pointer_entry_recovers_without_requiring_the_keyboard_owner_to_leave() {
    let mut view = visible_both_sources();
    view.pointer_move(Point::new(-10., -10.));
    assert!(
        !draw(&mut view),
        "old retained keyboard focus must not revive the tooltip on pointer exit"
    );
    view.pointer_move(anchor().center());
    assert!(
        draw(&mut view),
        "fresh pointer entry is a new display intent even with old keyboard focus retained"
    );
}
#[test]
fn fresh_focus_entry_recovers_without_requiring_the_pointer_owner_to_leave() {
    let mut view = visible_both_sources();
    view.set_focus(false, false);
    assert!(
        !draw(&mut view),
        "old retained hover must not revive the tooltip on blur"
    );
    view.set_focus(true, true);
    assert!(
        draw(&mut view),
        "fresh keyboard focus is a new display intent even with old hover retained"
    );
}
#[test]
fn inactive_reentry_attempts_never_become_a_future_display_intent() {
    let mut view = visible_both_sources();
    view.set_window_active(false);
    view.pointer_move(Point::new(-10., -10.));
    view.pointer_move(anchor().center());
    view.set_focus(false, false);
    view.set_focus(true, true);
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::None);
    view.set_window_active(true);
    assert!(!draw(&mut view));
    view.set_focus(true, true);
    assert!(draw(&mut view));
}

#[test]
fn observed_caller_hide_show_recovers_without_requiring_old_input_owners_to_leave() {
    let mut view = TooltipView::default();
    assert!(draw_controlled(&mut view, true));
    view.pointer_move(anchor().center());
    view.set_focus(true, true);
    assert!(draw_controlled(&mut view, true));
    assert_eq!(view.key(ControlKey::Escape), TooltipEvent::Dismissed);
    assert!(
        !draw_controlled(&mut view, true),
        "unchanged controlled true cannot override Escape"
    );
    assert!(
        !draw_controlled(&mut view, false),
        "old input owners cannot revive on caller hide"
    );
    assert!(
        draw_controlled(&mut view, true),
        "observed caller hide/show is a new independent request"
    );
}
