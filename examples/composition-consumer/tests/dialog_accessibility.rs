use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, DialogEvent, DialogView};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * 8.).min(w),
            r.style().line_height,
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
fn render(v: &mut DialogView, clip: Rect) {
    v.render(
        &mut Scene::new(),
        &mut Text,
        "Real public dialog",
        Rect::new(25., 35., 650., 500.),
        Some(clip),
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
}
#[test]
fn direct_public_semantic_actions_use_the_retained_buttons_and_restore_once() {
    let mut v = DialogView::default();
    v.show(17);
    render(&mut v, Rect::new(0., 0., 800., 700.));
    assert_eq!(DialogView::button_label(0), Some("Cancel"));
    assert_eq!(DialogView::button_label(1), Some("Confirm"));
    assert_eq!(DialogView::button_label(2), None);
    assert!(v.focus_button(1));
    assert_eq!(v.dialog().focus, 1);
    assert_eq!(
        v.activate_button(1),
        DialogEvent::Confirmed {
            restore_focus: Some(17)
        }
    );
    assert!(!v.is_open());
    assert_eq!(v.activate_button(1), DialogEvent::Blocked);
    v.show(4);
    render(&mut v, Rect::new(0., 0., 800., 700.));
    assert_eq!(
        v.activate_button(0),
        DialogEvent::Cancelled {
            restore_focus: Some(4)
        }
    );
}
#[test]
fn hidden_disabled_invalid_and_inactive_dialog_actions_preserve_the_existing_scope() {
    let mut v = DialogView::default();
    v.show(9);
    assert_eq!(v.activate_button(0), DialogEvent::Blocked);
    assert!(!v.focus_button(0));
    render(&mut v, Rect::new(0., 0., 800., 700.));
    v.set_disabled(true);
    assert_eq!(v.activate_button(1), DialogEvent::Blocked);
    assert!(!v.focus_button(1));
    assert!(v.is_open());
    v.set_disabled(false);
    assert_eq!(v.activate_button(2), DialogEvent::Blocked);
    v.set_window_active(false);
    assert_eq!(v.activate_button(0), DialogEvent::Blocked);
    v.set_window_active(true);
    render(&mut v, Rect::new(0., 0., 5., 5.));
    assert_eq!(v.activate_button(0), DialogEvent::Blocked);
    assert!(v.is_open());
}

#[cfg(feature = "token-audit")]
#[test]
fn direct_semantic_dialog_actions_reuse_actual_paint_tokens_in_all_themes_and_brands() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for index in 0..2 {
                let ((), lookups) = tekes_ui_tokens::audit::capture(|| {
                    let mut v = DialogView::default();
                    v.show(42);
                    let draw = |v: &mut DialogView| {
                        let mut scene = Scene::new();
                        v.render(
                            &mut scene,
                            &mut Text,
                            "Public action",
                            Rect::new(25., 35., 650., 500.),
                            Some(Rect::new(0., 0., 800., 700.)),
                            tekes_ui_tokens::Palette::new(theme, brand).unwrap(),
                            proposal::TextRole::UiBody,
                            false,
                        );
                        scene
                    };
                    assert!(!draw(&mut v).primitives().is_empty());
                    assert!(v.focus_button(index));
                    assert_eq!(v.dialog().focus, index);
                    assert!(!draw(&mut v).primitives().is_empty());
                    assert_eq!(
                        v.activate_button(index),
                        if index == 0 {
                            DialogEvent::Cancelled {
                                restore_focus: Some(42),
                            }
                        } else {
                            DialogEvent::Confirmed {
                                restore_focus: Some(42),
                            }
                        }
                    );
                    draw(&mut v);
                    assert_eq!(v.activate_button(index), DialogEvent::Blocked);
                });
                assert!(lookups.iter().any(|(family, _)| family == "color"));
                cases.push(serde_json::json!({"component":"tui.dialog","theme":format!("{theme:?}"),"brand":brand,"action_index":index,"state_asserted":true,"lookups":lookups}));
            }
        }
    }
    assert_eq!(cases.len(), 24);
    if let Some(path) = std::env::var_os("TEKESUI_DIALOG_AX_TOKEN_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"public direct semantic DialogView focus/activate and original retained Button painting,4themes3brands,Cancel/Confirm","excluded":["complete dependency extraction","physicalIME/VoiceOver","native GPU pixels"],"cases":cases})).unwrap()).unwrap();
    }
}
