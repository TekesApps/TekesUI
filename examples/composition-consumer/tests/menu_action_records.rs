use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
const ROOT: ComponentId = ComponentId(17000);
const MENU: ComponentId = ComponentId(17001);
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
fn host() -> ContentHost {
    let mut menu = MenuContent::new("Session options", vec!["A".into(), "B".into(), "C".into()]);
    menu.set_values(vec!["value-a".into(), "value-b".into(), "value-c".into()])
        .unwrap();
    let mut h = ContentHost::new(Content::new(
        ROOT,
        ListContent::new("Options", vec![Content::new(MENU, menu)]),
    ))
    .unwrap();
    h.set_action_events(true);
    paint(&mut h);
    h
}
fn menu(h: &mut ContentHost) -> &mut MenuContent {
    h.get_mut(MENU)
        .unwrap()
        .component_mut::<MenuContent>()
        .unwrap()
}
fn paint(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 600., 500.);
    h.render(&mut Scene::new(), &mut Text, f, f, Theme::LIGHT.into())
        .unwrap();
}
fn open(h: &mut ContentHost) {
    assert!(
        h.accessibility_action(MENU, None, AccessibilityAction::Press)
            .consumed
    );
    paint(h);
}
fn choose(h: &mut ContentHost, i: usize) {
    open(h);
    assert!(
        h.accessibility_action(MENU, Some(i), AccessibilityAction::Press)
            .activated
    );
}
fn selection(r: &ContentActionRecord) -> &MenuSelection {
    match r.payload.as_ref().unwrap() {
        ContentActionPayload::MenuSelection(s) => s,
    }
}
#[test]
fn delayed_records_keep_a_b_values_after_selection_and_model_change() {
    let mut h = host();
    let rev = menu(&mut h).model_revision().unwrap();
    choose(&mut h, 0);
    choose(&mut h, 1);
    menu(&mut h).selected = Some(2);
    menu(&mut h)
        .set_values(vec!["new-a".into(), "new-b".into(), "new-c".into()])
        .unwrap();
    let a = h.drain_action_records();
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].action.target, MENU);
    assert_eq!(a[0].action.source, ContentActionSource::Accessibility);
    assert_eq!(a[0].mount, a[1].mount);
    for (r, i, value) in [(&a[0], 0, "value-a"), (&a[1], 1, "value-b")] {
        let s = selection(r);
        assert_eq!(s.index(), i);
        assert_eq!(s.model_revision(), rev);
        assert_eq!(s.value_path().unwrap(), [value]);
    }
    assert!(h.drain_action_records().is_empty());
    assert!(h.drain_actions().is_empty());
}
#[test]
fn opening_keyboard_pointer_ax_dismissal_focus_paint_never_select() {
    let mut h = host();
    h.focus(Some(MENU), true);
    for _ in 0..2 {
        paint(&mut h);
    }
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    paint(&mut h);
    assert!(menu(&mut h).menu.is_open());
    assert!(h.drain_action_records().is_empty());
    h.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(h.drain_actions().is_empty());
    let p = h.get(MENU).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    paint(&mut h);
    assert!(h.drain_action_records().is_empty());
    h.event(&ContentEvent::Cancel);
    open(&mut h);
    h.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn label_mutation_invalidates_old_popup_before_another_paint() {
    let mut h = host();
    open(&mut h);
    let old = menu(&mut h).model_revision().unwrap();
    menu(&mut h).labels.swap(0, 1);
    assert_eq!(menu(&mut h).model_revision(), None);
    assert!(
        !h.accessibility_action(MENU, Some(0), AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
    assert!(!menu(&mut h).menu.is_open());
    assert_ne!(menu(&mut h).model_revision(), Some(old));
    open(&mut h);
    assert!(
        h.accessibility_action(MENU, Some(0), AccessibilityAction::Press)
            .activated
    );
    let a = h.drain_action_records();
    let s = selection(&a[0]);
    assert_eq!(s.label(), "B");
    assert_eq!(
        s.value_path(),
        None,
        "old positional value map cannot survive label reorder"
    );
}
#[test]
fn unmount_and_same_id_remount_discard_undrained_actions() {
    let mut h = host();
    choose(&mut h, 0);
    let node = h.remove(MENU).unwrap();
    h.insert(ROOT, node).unwrap();
    paint(&mut h);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.accessibility_action(MENU, Some(0), AccessibilityAction::Press)
            .activated
    );
    choose(&mut h, 1);
    assert_eq!(h.drain_actions().len(), 1);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn failed_value_binding_does_not_change_current_model_or_close_popup() {
    let mut h = host();
    open(&mut h);
    let before = menu(&mut h).model_revision();
    for values in [
        vec!["one".into()],
        vec!["a".into(), "a".into(), "c".into()],
        vec!["a".into(), " ".into(), "c".into()],
    ] {
        assert!(menu(&mut h).set_values(values).is_err());
        assert_eq!(menu(&mut h).model_revision(), before);
        assert!(menu(&mut h).menu.is_open());
    }
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn pointer_and_keyboard_capture_values_at_original_delivery() {
    let mut h = host();
    open(&mut h);
    let point = menu(&mut h).popup.as_ref().unwrap().items[1].frame.center();
    h.event(&ContentEvent::PointerDown(point));
    h.event(&ContentEvent::PointerUp(point));
    open(&mut h);
    h.event(&ContentEvent::Key {
        key: ControlKey::Home,
        down: true,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    h.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    let a = h.drain_action_records();
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].action.source, ContentActionSource::Pointer);
    assert_eq!(selection(&a[0]).value_path().unwrap(), ["value-b"]);
    assert_eq!(a[1].action.source, ContentActionSource::Keyboard);
    assert_eq!(selection(&a[1]).value_path().unwrap(), ["value-a"]);
}
#[test]
fn rebinding_model_cancels_held_pointer_and_identical_binding_keeps_revision() {
    let mut h = host();
    open(&mut h);
    let before = menu(&mut h).model_revision();
    menu(&mut h)
        .set_values(vec!["value-a".into(), "value-b".into(), "value-c".into()])
        .unwrap();
    assert_eq!(menu(&mut h).model_revision(), before);
    assert!(menu(&mut h).menu.is_open());
    let point = menu(&mut h).popup.as_ref().unwrap().items[0].frame.center();
    h.event(&ContentEvent::PointerDown(point));
    menu(&mut h)
        .set_values(vec!["new-a".into(), "new-b".into(), "new-c".into()])
        .unwrap();
    h.event(&ContentEvent::PointerUp(point));
    assert!(h.drain_action_records().is_empty());
    assert!(!menu(&mut h).menu.is_open());
    assert!(menu(&mut h).popup.is_none());
    assert_ne!(menu(&mut h).model_revision(), before);
}

#[test]
fn legacy_public_menu_literal_still_compiles_and_selects_original_item() {
    let original = MenuContent {
        trigger: Button::new("Legacy"),
        menu: tekes_ui_controls::MenuView::new(2),
        labels: vec!["Open".into(), "Archive".into()],
        selected: None,
        popup: None,
    };
    let mut h = ContentHost::new(Content::new(MENU, original)).unwrap();
    h.set_action_events(true);
    paint(&mut h);
    choose(&mut h, 1);
    let a = h.drain_action_records();
    assert_eq!(a.len(), 1);
    assert_eq!(selection(&a[0]).label(), "Archive");
    assert_eq!(selection(&a[0]).value_path(), None);
}
