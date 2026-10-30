#![cfg(target_os = "macos")]
//! Real original Checkbox/Scroll/Composer input owners; no GPU or native window claim.
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{CheckState, Checkbox, ControlKey, ScrollArea, composition::*, extended::{Axis, Stack}};
use tekes_ui_platform_macos::{AccessibilityAction as Native, content_accessibility::{content_action, content_key}};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Theme, proposal::TextRole};
const CHECK: ComponentId = ComponentId(10861);
const EDITOR: ComponentId = ComponentId(10862);
const SCROLL: ComponentId = ComponentId(10863);
fn setup(value: CheckState) -> (ContentHost, coretext::Text) {
    let mut scroll = ScrollContent::new(ScrollArea::default(), Content::new(CHECK, CheckboxContent::new("Original Checkbox · 完整标签", value)));
    scroll.viewport_height = Some(100.);
    let mut host = ContentHost::new(Content::new(ComponentId(10860), StackContent {
        model: Stack { axis: Axis::Vertical, gap: 0. }, flexible_child: None,
        children: vec![Content::new(SCROLL, scroll), Content::new(EDITOR, TextFieldContent::new("Composer", "kept draft 😀"))],
    })).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    render(&mut host, &mut text);
    assert!(host.focus(Some(EDITOR), true));
    render(&mut host, &mut text);
    host.set_action_events(true);
    (host, text)
}
fn render(h: &mut ContentHost, t: &mut coretext::Text) {
    let f = Rect::new(0., 0., 360., 220.);
    h.render(&mut Scene::new(), t, f, f, Theme::LIGHT.into()).unwrap();
}
fn check(h: &ContentHost) -> &CheckboxContent { h.get(CHECK).unwrap().component::<CheckboxContent>().unwrap() }
fn check_mut(h: &mut ContentHost) -> &mut CheckboxContent { h.get_mut(CHECK).unwrap().component_mut::<CheckboxContent>().unwrap() }
fn key(h: &mut ContentHost, t: &mut coretext::Text, key: ControlKey, down: bool) -> ContentResponse {
    h.event_with_text(&ContentEvent::Key { key, down }, t).unwrap()
}
fn once(h: &mut ContentHost, value: CheckState, source: ContentActionSource) {
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.source, source);
    assert!(matches!(&records[0].payload, Some(ContentActionPayload::CheckboxState(intent)) if intent.checked()==value));
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn scroll_visible_pointer_uses_original_decision_and_once_only_release() {
    for initial in [CheckState::Unchecked, CheckState::Checked, CheckState::Mixed] {
        let (mut h, mut t) = setup(initial);
        let frame = h.get(CHECK).unwrap().frame();
        let viewport = h.get(SCROLL).unwrap().frame();
        let mut original = Checkbox { checked: initial, ..Default::default() };
        original.render(&mut Scene::new(), &mut t, "Original Checkbox · 完整标签", frame, Some(viewport), Theme::LIGHT, TextRole::UiBody);
        let p = check(&h).layout().label.center();
        assert!(viewport.contains(p));
        let down = h.event_with_text(&ContentEvent::PointerDown(p), &mut t).unwrap();
        assert_eq!(down.consumed, original.pointer_down_at(p));
        assert!(down.consumed && down.capture_pointer && !down.activated);
        assert_eq!(h.focused(), Some(CHECK));
        assert!(check(&h).model().button.state.focused);
        assert!(!check(&h).model().button.state.focus_visible);
        assert_eq!(check(&h).model().checked, initial);
        assert!(h.drain_action_records().is_empty());
        let up = h.event_with_text(&ContentEvent::PointerUp(p), &mut t).unwrap();
        assert_eq!(up.activated, original.pointer_up_at(p));
        assert!(up.activated);
        assert_eq!(check(&h).model().checked, original.checked);
        once(&mut h, original.checked, ContentActionSource::Pointer);
        assert!(!h.event_with_text(&ContentEvent::PointerUp(p), &mut t).unwrap().activated);
        assert!(h.drain_action_records().is_empty());
    }
}
#[test]
fn scroll_ax_press_focus_and_bidirectional_tab_retain_original_focus_hooks() {
    let (mut h, mut t) = setup(CheckState::Mixed);
    let target = content_key(&h, CHECK, None).unwrap();
    assert!(content_action(&mut h, target, Native::Press).activated);
    assert_eq!(h.focused(), Some(CHECK));
    assert!(check(&h).model().button.state.focused);
    assert!(check(&h).model().button.state.focus_visible, "consuming AX apply must preserve the already-admitted focus hook");
    once(&mut h, CheckState::Checked, ContentActionSource::Accessibility);
    render(&mut h, &mut t);
    assert!(h.focus(Some(EDITOR), true));
    render(&mut h, &mut t);
    let target = content_key(&h, CHECK, None).unwrap();
    assert!(content_action(&mut h, target, Native::Focus).consumed);
    assert!(check(&h).model().button.state.focus_visible);
    assert!(h.drain_action_records().is_empty());
    for reverse in [false, true] {
        assert!(h.focus(Some(EDITOR), true)); render(&mut h, &mut t);
        key(&mut h, &mut t, ControlKey::Tab { reverse }, true);
        assert_eq!(h.focused(), Some(CHECK));
        assert!(check(&h).model().button.state.focused && check(&h).model().button.state.focus_visible);
        assert!(h.drain_action_records().is_empty());
    }
    assert!(!key(&mut h, &mut t, ControlKey::Enter, true).activated);
    key(&mut h, &mut t, ControlKey::Space, true);
    key(&mut h, &mut t, ControlKey::Space, true);
    assert!(key(&mut h, &mut t, ControlKey::Space, false).activated);
    once(&mut h, CheckState::Unchecked, ContentActionSource::Keyboard);
}
#[test]
fn scroll_controlled_change_cancels_original_pointer_and_space_but_noop_keeps_them() {
    for space in [false, true] { for changed in [false, true] {
        let (mut h, mut t) = setup(CheckState::Mixed);
        let p = check(&h).layout().label.center();
        if space {
            assert!(h.focus(Some(CHECK), true)); render(&mut h, &mut t);
            key(&mut h, &mut t, ControlKey::Space, true);
        } else { assert!(h.event_with_text(&ContentEvent::PointerDown(p), &mut t).unwrap().capture_pointer); }
        check_mut(&mut h).set_checked(if changed { CheckState::Unchecked } else { CheckState::Mixed });
        let response = if space { key(&mut h, &mut t, ControlKey::Space, false) }
            else { h.event_with_text(&ContentEvent::PointerUp(p), &mut t).unwrap() };
        assert_eq!(response.activated, !changed);
        if changed { assert_eq!(check(&h).model().checked, CheckState::Unchecked); assert!(h.drain_action_records().is_empty()); }
        else { once(&mut h, CheckState::Checked, if space {ContentActionSource::Keyboard} else {ContentActionSource::Pointer}); }
    }}
}
fn preedit(h: &mut ContentHost, t: &mut coretext::Text) {
    h.event_with_text(&ContentEvent::Preedit {text:"ni".into(),selection:1..2,clauses:vec![],replacement:None}, t).unwrap();
    assert!(h.is_composing()); render(h, t);
}
#[test]
fn scroll_marked_composer_refuses_strict_ax_and_tab_before_checkbox_or_focus_effects() {
    let (mut h, mut t) = setup(CheckState::Mixed); preedit(&mut h, &mut t);
    let before = format!("{:?}", check(&h));
    let editor = h.semantic_text_input_snapshot(EDITOR).unwrap();
    let binding = h.focused_text_binding(); let epoch = h.focus_epoch();
    let offset = h.get(SCROLL).unwrap().component::<ScrollContent>().unwrap().scroll.offset;
    for action in [Native::Press, Native::Focus, Native::SetValue("on".into())] {
        let target = content_key(&h, CHECK, None).unwrap();
        let result = content_action(&mut h, target, action);
        assert!(!result.consumed && !result.activated);
    }
    key(&mut h, &mut t, ControlKey::Tab { reverse:false }, true);
    assert_eq!(format!("{:?}", check(&h)), before);
    assert_eq!((h.focused(),h.focus_epoch(),h.focused_text_binding()), (Some(EDITOR),epoch,binding));
    let after = h.semantic_text_input_snapshot(EDITOR).unwrap();
    assert_eq!((after.text,after.selection,after.marked),(editor.text,editor.selection,editor.marked));
    assert_eq!(h.get(SCROLL).unwrap().component::<ScrollContent>().unwrap().scroll.offset,offset);
    assert!(h.pointer_capture().is_none()); assert!(h.drain_action_records().is_empty());
}
#[test]
fn scroll_marked_composer_pointer_blurs_original_editor_then_releases_checkbox_once() {
    let (mut h, mut t) = setup(CheckState::Mixed); preedit(&mut h, &mut t);
    let before = h.semantic_text_input_snapshot(EDITOR).unwrap();
    let p = check(&h).layout().label.center();
    assert!(h.event_with_text(&ContentEvent::PointerDown(p), &mut t).unwrap().capture_pointer);
    assert_eq!(h.focused(),Some(CHECK)); assert!(!h.is_composing());
    let after = h.semantic_text_input_snapshot(EDITOR).unwrap();
    assert_eq!((after.text,after.selection),(before.text,before.selection));
    assert!(after.marked.is_none());
    assert!(h.drain_action_records().is_empty(),"original blur never submits the Composer");
    assert!(h.event_with_text(&ContentEvent::PointerUp(p), &mut t).unwrap().activated);
    once(&mut h, CheckState::Checked, ContentActionSource::Pointer);
}
#[test]
fn scroll_stale_layout_rejects_before_checkbox_capture_queue_and_editor_mutation() {
    for pointer in [false,true] {
        let (mut h, mut t) = setup(CheckState::Mixed);
        let p = check(&h).layout().label.center(); let target = content_key(&h,CHECK,None).unwrap();
        let before = format!("{:?}",check(&h)); let binding = h.focused_text_binding();
        h.get_mut(SCROLL).unwrap();
        let r = if pointer {h.event_with_text(&ContentEvent::PointerDown(p),&mut t).unwrap()}
            else {content_action(&mut h,target,Native::Press)};
        assert!(!r.consumed && !r.activated);
        assert_eq!(format!("{:?}",check(&h)),before);
        assert_eq!(h.focused(),Some(EDITOR)); assert_eq!(h.focused_text_binding(),binding);
        assert!(h.pointer_capture().is_none()); assert!(h.drain_action_records().is_empty());
    }
}

#[test]
fn scroll_ax_press_cancels_original_space_arm_without_duplicate_release() {
    let (mut h, mut t) = setup(CheckState::Mixed);
    assert!(h.focus(Some(CHECK), true)); render(&mut h, &mut t);
    key(&mut h, &mut t, ControlKey::Space, true);
    let target = content_key(&h, CHECK, None).unwrap();
    assert!(content_action(&mut h, target, Native::Press).activated);
    assert!(check(&h).model().button.state.focused && check(&h).model().button.state.focus_visible);
    once(&mut h, CheckState::Checked, ContentActionSource::Accessibility);
    assert!(!key(&mut h, &mut t, ControlKey::Space, false).activated);
    assert!(h.drain_action_records().is_empty());
}
