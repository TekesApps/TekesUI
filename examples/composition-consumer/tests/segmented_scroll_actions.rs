#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{composition::*, extended::{Axis, Stack}, ControlKey, ScrollArea};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
const ID: ComponentId = ComponentId(110);
const EDITOR: ComponentId = ComponentId(11001);
const SCROLL: ComponentId = ComponentId(11002);
const AFTER: ComponentId = ComponentId(11003);
fn owner(h: &ContentHost) -> &SegmentedControlContent { h.get(ID).unwrap().component().unwrap() }
fn owner_mut(h: &mut ContentHost) -> &mut SegmentedControlContent { h.get_mut(ID).unwrap().component_mut().unwrap() }
fn render(h: &mut ContentHost, text: &mut coretext::Text) {
    let frame = Rect::new(0., 0., 480., 320.);
    h.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into()).unwrap();
    assert_eq!(owner(h).semantic_parts().len(), 3, "actual visible Scroll descendant");
}
fn host() -> (ContentHost, coretext::Text) {
    let mut scroll = ScrollContent::new(ScrollArea::default(), Content::new(ID,
        SegmentedControlContent::new("Segments", vec!["Day".into(), "Week".into(), "Month".into()])));
    scroll.viewport_height = Some(100.);
    let mut h = ContentHost::new(Content::new(ComponentId(11000), StackContent {
        model: Stack { axis: Axis::Vertical, gap: 4. }, flexible_child: None,
        children: vec![Content::new(EDITOR, TextFieldContent::new("Composer", "draft 原文")),
            Content::new(SCROLL, scroll), Content::new(AFTER, Button::new("After"))],
    })).unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.); text.services = true;
    render(&mut h, &mut text);
    assert!(h.focus(Some(EDITOR), true));
    (h, text)
}
fn token(h: &ContentHost, index: usize) -> usize { owner(h).part_token(index).unwrap() }
fn records(h: &mut ContentHost) -> Vec<usize> {
    h.drain_action_records().into_iter().map(|r| {
        assert_eq!(r.action.kind, ContentActionKind::Activate);
        match r.payload.unwrap() { ContentActionPayload::SegmentSelection(i) => i.index(), _ => panic!("original intent") }
    }).collect()
}
fn key(h: &mut ContentHost, key: ControlKey, down: bool) -> ContentResponse { h.event(&ContentEvent::Key { key, down }) }
#[test]
fn visible_scroll_pointer_and_semantic_focus_press_use_original_once_actions() {
    let (mut h, mut text) = host();
    let mount = h.semantic_mount(ID); let initial = token(&h, 1);
    let point = owner(&h).semantic_parts()[1].frame.center();
    let down = h.event(&ContentEvent::PointerDown(point));
    assert!(down.consumed && down.capture_pointer && !down.activated);
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(owner(&h).model().selected, 0);
    assert_eq!(owner(&h).view().focused(), Some(1));
    assert!(records(&mut h).is_empty());
    assert!(h.event(&ContentEvent::PointerUp(point)).activated);
    assert_eq!(records(&mut h), vec![1]);
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    assert!(h.focus(Some(EDITOR), true));
    let next = token(&h, 2);
    assert!(h.accessibility_action(ID, Some(next), AccessibilityAction::Focus).consumed);
    assert_eq!(owner(&h).view().focused(), Some(2));
    assert_eq!(owner(&h).model().selected, 1);
    assert!(records(&mut h).is_empty());
    assert!(key(&mut h, ControlKey::Space, true).consumed);
    assert!(key(&mut h, ControlKey::Space, true).consumed);
    assert!(key(&mut h, ControlKey::Space, false).activated);
    assert_eq!(records(&mut h), vec![2]);
    assert!(!key(&mut h, ControlKey::Space, false).activated);
    assert!(h.focus(Some(EDITOR), true));
    assert!(h.accessibility_action(ID, Some(initial), AccessibilityAction::Press).activated);
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(records(&mut h), vec![1]);
    assert!(!h.accessibility_action(ID, Some(initial), AccessibilityAction::Press).activated);
    assert!(records(&mut h).is_empty());
    render(&mut h, &mut text);
    assert_eq!(h.semantic_mount(ID), mount);
    assert_eq!(token(&h, 1), initial);
}
#[test]
fn visible_scroll_host_focus_and_tab_transfer_use_stable_original_hooks() {
    let (mut h, _) = host();
    assert!(h.focus(Some(ID), true));
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(owner(&h).view().focused(), Some(0));
    assert!(h.focus(Some(EDITOR), true));
    assert!(key(&mut h, ControlKey::Tab { reverse: false }, true).consumed);
    assert_eq!(h.focused(), Some(ID));
    assert_eq!(owner(&h).view().focused(), Some(0));
    let t = token(&h, 1);
    assert!(h.accessibility_action(ID, Some(t), AccessibilityAction::Focus).consumed);
    assert!(key(&mut h, ControlKey::Space, true).consumed);
    assert_eq!(owner(&h).model().selected, 0);
    assert!(key(&mut h, ControlKey::Tab { reverse: false }, true).consumed);
    assert_eq!(h.focused(), Some(AFTER));
    assert_eq!(owner(&h).view().focused(), None);
    assert!(key(&mut h, ControlKey::Tab { reverse: true }, true).consumed);
    assert_eq!(h.focused(), Some(ID));
    assert!(!key(&mut h, ControlKey::Space, false).activated);
    assert_eq!(owner(&h).model().selected, 0);
    assert!(records(&mut h).is_empty());
}
#[test]
fn marked_composer_rejects_scroll_strict_ax_tab_but_original_pointer_commits() {
    let (mut h, mut text) = host();
    assert!(h.event(&ContentEvent::Preedit { text: "ni".into(), selection: 2..2,
        clauses: std::iter::once(0..2).collect(), replacement: None }).consumed);
    render(&mut h, &mut text);
    assert!(h.is_composing());
    let binding = h.focused_text_binding(); let snapshot = h.text_input_snapshot();
    let before = format!("{:?}", owner(&h)); let epoch = h.focus_epoch();
    let t = token(&h, 1);
    for action in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        assert!(!h.accessibility_action(ID, Some(t), action).consumed);
        assert_eq!(format!("{:?}", owner(&h)), before);
        assert_eq!(h.focused(), Some(EDITOR)); assert_eq!(h.focus_epoch(), epoch);
        assert_eq!(h.focused_text_binding(), binding); assert_eq!(h.text_input_snapshot(), snapshot);
    }
    key(&mut h, ControlKey::Tab { reverse: false }, true);
    assert_eq!(h.focused(), Some(EDITOR)); assert_eq!(h.focus_epoch(), epoch);
    assert_eq!(format!("{:?}", owner(&h)), before);
    assert_eq!(h.focused_text_binding(), binding); assert_eq!(h.text_input_snapshot(), snapshot);
    assert!(records(&mut h).is_empty());
    let input = h.get(EDITOR).unwrap().component::<TextFieldContent>().unwrap().surface.input.text.clone();
    let point = owner(&h).semantic_parts()[1].frame.center();
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert_eq!(h.focused(), Some(ID)); assert!(!h.is_composing());
    let final_input = &h.get(EDITOR).unwrap().component::<TextFieldContent>().unwrap().surface.input;
    assert!(final_input.marked.is_none()); assert_eq!(final_input.text, input);
    assert_eq!(owner(&h).model().selected, 0); assert_eq!(owner(&h).view().focused(), Some(1));
    assert!(records(&mut h).is_empty());
    assert!(h.event(&ContentEvent::PointerUp(point)).activated);
    assert_eq!(records(&mut h), vec![1]); assert!(records(&mut h).is_empty());
}
#[test]
fn visible_scroll_controlled_cancel_and_exposure_aba_keep_original_intent_contract() {
    let (mut h, mut text) = host();
    let old = token(&h, 1); let p = owner(&h).semantic_parts()[1].frame.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    owner_mut(&mut h).set_selected(2).unwrap();
    assert_eq!(owner(&h).view().focused(), Some(1));
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(records(&mut h).is_empty());
    // get_mut invalidates the Host input/reveal proof. Publish the normal next
    // layout before a new semantic transaction; continuous authority is retained.
    render(&mut h, &mut text);
    assert_eq!(token(&h, 1), old);
    assert!(h.accessibility_action(ID, Some(old), AccessibilityAction::Press).activated);
    let intents = h.drain_action_records(); assert_eq!(intents.len(), 1);
    let Some(ContentActionPayload::SegmentSelection(intent)) = &intents[0].payload else { panic!("intent") };
    owner_mut(&mut h).set_selected(0).unwrap();
    assert!(owner(&h).intent_is_current(intent));
    owner_mut(&mut h).set_item_enabled(1, false).unwrap();
    owner_mut(&mut h).set_item_enabled(1, true).unwrap();
    render(&mut h, &mut text);
    assert_ne!(token(&h, 1), old); assert!(!owner(&h).intent_is_current(intent));
    assert!(!h.accessibility_action(ID, Some(old), AccessibilityAction::Focus).consumed);
    assert!(!h.accessibility_action(ID, Some(old), AccessibilityAction::Press).activated);
    assert!(records(&mut h).is_empty());
}
