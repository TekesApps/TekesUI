use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const MENU: ComponentId = ComponentId(18001);
#[derive(Default)]
struct Text {
    draws: Vec<(String, Rect, Rgba)>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, role: proposal::TextRole, width: f32) -> Size {
        Size::new(
            (s.chars().count() as f32 * 8.).min(width),
            role.style().line_height,
        )
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        r: Rect,
        _: ButtonTextAlign,
        c: Rgba,
    ) {
        self.draws.push((s.into(), r, c));
    }
}
fn menu(h: &mut ContentHost) -> &mut MenuContent {
    h.get_mut(MENU)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
}
fn paint(h: &mut ContentHost, theme: Theme) -> Text {
    let mut t = Text::default();
    let f = Rect::new(0., 0., 600., 500.);
    h.render(&mut Scene::new(), &mut t, f, f, theme.into())
        .unwrap();
    t
}
fn host() -> ContentHost {
    let mut m = MenuContent::new(
        "Options",
        ["Disabled", "Checked", "separator", "Next", "Last disabled"]
            .map(String::from)
            .to_vec(),
    );
    m.set_values(
        ["d", "checked", "separator", "next", "last"]
            .map(String::from)
            .to_vec(),
    )
    .unwrap();
    m.set_item_disabled(0, true);
    m.set_item_disabled(4, true);
    m.set_item_separator(2, true);
    m.set_item_checked(1, true);
    m.set_item_checked(3, false);
    let mut h = ContentHost::new(Content::new(MENU, m)).unwrap();
    h.set_action_events(true);
    h
}
fn open(h: &mut ContentHost, theme: Theme) -> Text {
    paint(h, theme);
    assert!(
        h.accessibility_action(MENU, None, AccessibilityAction::Press)
            .consumed
    );
    paint(h, theme)
}
#[test]
fn keyboard_skips_disabled_separator_and_keeps_checked_independent() {
    let mut h = host();
    open(&mut h, Theme::LIGHT);
    assert_eq!(menu(&mut h).popup.as_ref().unwrap().active_item, 1);
    h.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    paint(&mut h, Theme::LIGHT);
    let n = &h.semantics()[0];
    assert!(n.parts.iter().find(|p| p.index == 3).unwrap().focused);
    let checked = n.parts.iter().find(|p| p.index == 1).unwrap();
    assert!(checked.semantics.selected);
    assert!(!checked.focused);
    assert_eq!(checked.semantics.value.as_deref(), Some("on"));
    assert_eq!(
        n.parts
            .iter()
            .find(|p| p.index == 3)
            .unwrap()
            .semantics
            .value
            .as_deref(),
        Some("off")
    );
    assert!(!n.parts.iter().any(|p| p.index == 2));
    h.event(&ContentEvent::Key {
        key: ControlKey::End,
        down: true,
    });
    assert_eq!(menu(&mut h).menu.menu().items.selected, 3);
    h.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    assert_eq!(menu(&mut h).menu.menu().items.selected, 1);
    h.event(&ContentEvent::Key {
        key: ControlKey::Up,
        down: true,
    });
    assert_eq!(menu(&mut h).menu.menu().items.selected, 3);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn disabled_and_separator_reject_ax_pointer_and_programmatic_item_activation() {
    let mut h = host();
    open(&mut h, Theme::LIGHT);
    for i in [0, 2, 4] {
        assert!(
            !h.accessibility_action(MENU, Some(i), AccessibilityAction::Focus)
                .consumed
        );
        assert!(
            !h.accessibility_action(MENU, Some(i), AccessibilityAction::Press)
                .activated
        );
        let p = menu(&mut h).popup.as_ref().unwrap().items[i].frame.center();
        h.event(&ContentEvent::PointerDown(p));
        assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
        assert_eq!(
            menu(&mut h).menu.activate_item(i),
            tekes_ui_controls::MenuEvent::None
        );
    }
    assert!(h.drain_action_records().is_empty());
    assert!(menu(&mut h).menu.is_open());
}
#[test]
fn checked_is_controlled_not_toggled_by_original_selection() {
    let mut h = host();
    open(&mut h, Theme::LIGHT);
    assert!(
        h.accessibility_action(MENU, Some(3), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_action_records().len(), 1);
    open(&mut h, Theme::LIGHT);
    let n = &h.semantics()[0];
    assert!(
        !n.parts
            .iter()
            .find(|p| p.index == 3)
            .unwrap()
            .semantics
            .selected
    );
    assert!(
        n.parts
            .iter()
            .find(|p| p.index == 1)
            .unwrap()
            .semantics
            .selected
    );
}
#[test]
fn declaration_update_cancels_old_popup_and_held_input_without_rebinding_queued_value() {
    let mut h = host();
    open(&mut h, Theme::LIGHT);
    assert!(
        h.accessibility_action(MENU, Some(1), AccessibilityAction::Press)
            .activated
    );
    open(&mut h, Theme::LIGHT);
    let p = menu(&mut h).popup.as_ref().unwrap().items[3].frame.center();
    h.event(&ContentEvent::PointerDown(p));
    let before = menu(&mut h).model_revision();
    assert!(menu(&mut h).set_item_disabled(3, true));
    assert_ne!(menu(&mut h).model_revision(), before);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(!menu(&mut h).menu.is_open());
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    let ContentActionPayload::MenuSelection(s) = r[0].payload.as_ref().unwrap();
    assert_eq!(s.value_path().unwrap(), ["checked"]);
    assert!(!menu(&mut h).set_item_disabled(3, true));
    assert!(!menu(&mut h).set_item_separator(99, true));
}
#[test]
fn check_marker_and_separator_reuse_original_palette_and_label_columns() {
    for theme in Theme::ALL {
        let mut h = host();
        let t = open(&mut h, theme);
        assert!(!t.draws.iter().any(|(s, _, _)| s == "separator"));
        let check = t.draws.iter().find(|(s, _, _)| s == "✓").unwrap();
        let label = t.draws.iter().find(|(s, _, _)| s == "Checked").unwrap();
        let next = t.draws.iter().find(|(s, _, _)| s == "Next").unwrap();
        assert_eq!(check.2, label.2);
        assert!(check.1.max_x() <= label.1.min_x());
        assert!(
            label.1.min_x() - check.1.max_x()
                >= proposal::metric("spacing", "space-tight").unwrap() - 0.001,
            "indicator and caption need the shared gap even when large glyph fills its column"
        );
        assert_eq!(label.1.min_x(), next.1.min_x());
        let disabled = t.draws.iter().find(|(s, _, _)| s == "Disabled").unwrap();
        let palette: tekes_ui_tokens::Palette = theme.into();
        assert_eq!(disabled.2, palette.color("tertiary-text").unwrap());
    }
}
#[test]
fn an_all_unavailable_menu_has_no_selection_or_fake_focused_row() {
    let mut h = host();
    menu(&mut h).set_item_disabled(1, true);
    menu(&mut h).set_item_disabled(3, true);
    open(&mut h, Theme::LIGHT);
    for key in [
        ControlKey::Home,
        ControlKey::Down,
        ControlKey::End,
        ControlKey::Enter,
        ControlKey::Space,
    ] {
        h.event(&ContentEvent::Key { key, down: true });
        h.event(&ContentEvent::Key { key, down: false });
    }
    paint(&mut h, Theme::LIGHT);
    assert!(h.semantics()[0].parts.iter().all(|p| !p.focused));
    assert!(h.drain_action_records().is_empty());
    assert!(menu(&mut h).menu.is_open());
}

#[test]
fn adding_short_item_state_does_not_change_original_popup_minimum_geometry() {
    let mut m = MenuContent::new("Options", vec!["A".into(), "B".into()]);
    m.set_values(vec!["a".into(), "b".into()]).unwrap();
    let mut h = ContentHost::new(Content::new(MENU, m)).unwrap();
    open(&mut h, Theme::LIGHT);
    let before = menu(&mut h).popup.as_ref().unwrap().popup;
    menu(&mut h).set_item_checked(0, false);
    open(&mut h, Theme::LIGHT);
    assert_eq!(
        menu(&mut h).popup.as_ref().unwrap().popup,
        before,
        "item state must reuse the same original popup minimum/placement rule"
    );
}
