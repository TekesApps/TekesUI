use std::time::Duration;
use tekes_ui_controls::layout_view::SelectView;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlMotionContext, Menu, MenuEvent, MenuView, Select,
};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
#[derive(Default)]
struct Text {
    millis: u64,
    reduced: bool,
}
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.millis),
            reduced_motion: self.reduced,
        })
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        r: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        scene.push_quad(Quad::new(r, c));
    }
}
fn paint_menu(view: &mut MenuView, model: &mut Menu, text: &mut Text) -> Scene {
    let mut scene = Scene::new();
    view.render_with_model(
        model,
        &mut scene,
        text,
        &["Open", "Duplicate", "Archive"],
        Rect::new(10., 10., 160., 36.),
        Rect::new(0., 0., 600., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    scene
}
#[test]
fn external_menu_semantics_use_original_model_restore_once_and_keep_only_exit_paint() {
    let mut model = Menu::new(3);
    let mut view = MenuView::new_external();
    let mut text = Text::default();
    view.show_with_model(&mut model, 17);
    assert!(!view.focus_item_with_model(&mut model, 1), "no layout yet");
    paint_menu(&mut view, &mut model, &mut text);
    text.millis = 200;
    paint_menu(&mut view, &mut model, &mut text);
    assert!(view.focus_item_with_model(&mut model, 2));
    assert_eq!(model.items.selected, 2);
    assert!(
        !view.focus_item(0),
        "bound runtime cannot mutate hidden internal model"
    );
    assert_eq!(
        view.activate_item_with_model(&mut model, 2),
        MenuEvent::Activated {
            index: 2,
            restore_focus: Some(17)
        }
    );
    assert_eq!(model.last_action, Some(2));
    assert!(!model.open);
    assert!(
        !paint_menu(&mut view, &mut model, &mut text).is_empty(),
        "original exit snapshot remains drawable"
    );
    assert!(view.is_presented());
    assert!(!view.focus_item_with_model(&mut model, 0));
    assert_eq!(
        view.activate_item_with_model(&mut model, 0),
        MenuEvent::None
    );
    text.millis = 500;
    paint_menu(&mut view, &mut model, &mut text);
    assert!(!view.is_presented());
}
#[test]
fn external_semantics_reject_disabled_inactive_and_stale_model_count() {
    let mut model = Menu::new(3);
    let mut view = MenuView::new_external();
    let mut text = Text::default();
    view.show_with_model(&mut model, 8);
    paint_menu(&mut view, &mut model, &mut text);
    model.items.count = 1;
    assert!(!view.focus_item_with_model(&mut model, 2));
    assert_eq!(
        view.activate_item_with_model(&mut model, 2),
        MenuEvent::None
    );
    view.set_disabled_with_model(&mut model, true);
    assert!(!view.focus_item_with_model(&mut model, 0));
    assert_eq!(
        view.activate_item_with_model(&mut model, 0),
        MenuEvent::None
    );
    view.set_disabled_with_model(&mut model, false);
    let _ = view.set_window_active_with_model(&mut model, false);
    assert!(!view.focus_item_with_model(&mut model, 0));
    assert_eq!(
        view.activate_item_with_model(&mut model, 0),
        MenuEvent::None
    );
}
#[test]
fn public_select_focus_is_not_commit_and_activation_uses_retained_menu() {
    let mut model = Select::new(3);
    let mut view = SelectView::new(3);
    let mut text = Text::default();
    model.menu.show();
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        &["Auto", "Compact", "Comfortable"],
        Rect::new(10., 10., 200., 36.),
        Rect::new(0., 0., 600., 500.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert!(view.focus_option(2, &mut model));
    assert_eq!(model.selected, 0);
    assert_eq!(model.menu.items.selected, 2);
    assert!(matches!(
        view.activate_option(2, &mut model),
        MenuEvent::Activated { index: 2, .. }
    ));
    assert_eq!(model.selected, 2);
    assert!(!model.menu.open);
    assert_eq!(view.activate_option(2, &mut model), MenuEvent::None);
    assert!(!view.focus_option(2, &mut model));
}

#[test]
fn external_semantic_close_uses_existing_reduced_motion_presence() {
    let mut model = Menu::new(3);
    let mut view = MenuView::new_external();
    let mut text = Text {
        reduced: true,
        ..Text::default()
    };
    view.set_reduced_motion(true);
    view.show_with_model(&mut model, 6);
    assert!(!paint_menu(&mut view, &mut model, &mut text).is_empty());
    assert!(matches!(
        view.activate_item_with_model(&mut model, 1),
        MenuEvent::Activated { index: 1, .. }
    ));
    assert!(paint_menu(&mut view, &mut model, &mut text).is_empty());
    assert!(!view.is_presented());
    assert!(!view.needs_animation());
}
