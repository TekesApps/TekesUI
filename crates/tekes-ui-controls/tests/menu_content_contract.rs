use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, ControlMotionContext,
    composition::{
        AccessibilityAction, Component, ContentEvent, ContentRole, ContentTheme, Lifecycle,
        MenuContent,
    },
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};

#[derive(Default)]
struct Text(u64);
impl ButtonTextRenderer for Text {
    fn motion_context(&self) -> Option<ControlMotionContext> {
        Some(ControlMotionContext {
            now: Duration::from_millis(self.0),
            reduced_motion: false,
        })
    }
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(width), role.style().line_height)
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
fn menu() -> MenuContent {
    MenuContent::new("Actions", vec!["Open".into(), "Archive".into()])
}
fn paint(menu: &mut MenuContent, text: &mut Text) {
    menu.paint_overlay(
        &mut Scene::new(),
        text,
        Rect::new(20., 20., 180., 36.),
        Rect::new(0., 0., 600., 500.),
        ContentTheme::from(tekes_ui_tokens::Palette::from(Theme::LIGHT)),
    );
}
fn unavailable(menu: &mut MenuContent, loading: bool) {
    menu.trigger.button.state.disabled = !loading;
    menu.trigger.button.state.loading = loading;
}

#[test]
fn menu_trigger_reuses_button_unavailable_semantics_and_rejects_ax_open() {
    for loading in [false, true] {
        let mut menu = menu();
        unavailable(&mut menu, loading);
        let semantics = menu.semantics();
        assert_eq!(semantics.role, ContentRole::MenuButton);
        assert!(
            semantics.disabled,
            "disabled/loading trigger must publish unavailable"
        );
        assert!(
            !menu
                .accessibility_action(None, &AccessibilityAction::Press)
                .activated
        );
        assert!(
            !menu.menu.is_open(),
            "unavailable AX Press must not open the menu"
        );
    }
}

#[test]
fn open_menu_rejects_latest_unavailable_trigger_before_another_paint() {
    for loading in [false, true] {
        for action in [AccessibilityAction::Press, AccessibilityAction::Focus] {
            let mut menu = menu();
            menu.menu.show(0);
            paint(&mut menu, &mut Text::default());
            unavailable(&mut menu, loading);
            assert!(menu.semantic_parts().is_empty());
            assert!(!menu.overlay_open());
            assert_eq!(menu.semantics().expanded, Some(false));
            let response = menu.accessibility_action(Some(1), &action);
            assert!(!response.activated && !response.consumed);
            assert_eq!(menu.selected, None);
            assert!(!menu.menu.is_open());
        }
    }
}

#[test]
fn held_menu_pointer_is_cancelled_when_trigger_becomes_unavailable() {
    for loading in [false, true] {
        let mut menu = menu();
        menu.menu.show(0);
        let mut text = Text::default();
        paint(&mut menu, &mut text);
        text.0 = 200;
        paint(&mut menu, &mut text);
        let point = menu.popup.as_ref().unwrap().items[1].frame.center();
        menu.overlay_event(&ContentEvent::PointerDown(point));
        unavailable(&mut menu, loading);
        assert!(
            !menu
                .overlay_event(&ContentEvent::PointerUp(point))
                .activated
        );
        assert_eq!(menu.selected, None);
        assert!(!menu.menu.is_open());
        menu.trigger.button.state.disabled = false;
        menu.trigger.button.state.loading = false;
        assert!(
            !menu
                .overlay_event(&ContentEvent::PointerUp(point))
                .activated
        );
        assert!(
            !menu
                .overlay_event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .activated
        );
        assert!(
            !menu.menu.is_open(),
            "re-enabling cannot revive an old open/press intent"
        );
        menu.accessibility_action(None, &AccessibilityAction::Press);
        paint(&mut menu, &mut text);
        assert!(
            menu.accessibility_action(Some(1), &AccessibilityAction::Press)
                .activated
        );
        assert_eq!(menu.selected, Some(1));
    }
}

#[test]
fn menu_unmount_discards_retained_paint_and_hit_layout() {
    let mut menu = menu();
    let mut text = Text::default();
    menu.menu.show(0);
    paint(&mut menu, &mut text);
    text.0 = 200;
    paint(&mut menu, &mut text);
    assert!(menu.overlay_presented());
    menu.event(&ContentEvent::Cancel);
    assert!(
        menu.overlay_presented(),
        "ordinary dismissal retains exit paint"
    );
    menu.lifecycle(Lifecycle::Unmounted);
    assert!(
        !menu.overlay_presented(),
        "unmounted content must not replay old exit paint"
    );
    assert!(menu.popup.is_none());
    menu.lifecycle(Lifecycle::Mounted);
    assert!(!menu.overlay_presented());
    assert!(menu.semantic_parts().is_empty());
}
