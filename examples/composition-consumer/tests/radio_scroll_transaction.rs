#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, ScrollArea,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
const RADIO: ComponentId = ComponentId(10920);
const EDITOR: ComponentId = ComponentId(10921);
fn fixture() -> (ContentHost, coretext::Text) {
    let mut scroll = ScrollContent::new(
        ScrollArea::default(),
        Content::new(
            RADIO,
            RadioGroupContent::new("Mode", vec!["One".into(), "Two".into(), "Three".into()]),
        ),
    );
    scroll.viewport_height = Some(110.);
    let mut h = ContentHost::new(Content::new(
        ComponentId(10900),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(EDITOR, TextFieldContent::new("Composer", "Draft A😀B")),
                Content::new(ComponentId(10910), scroll),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    t.services = true;
    h.render(
        &mut Scene::new(),
        &mut t,
        Rect::new(0., 0., 300., 240.),
        Rect::new(0., 0., 300., 240.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    (h, t)
}
fn get(h: &ContentHost) -> &RadioGroupContent {
    h.get(RADIO).unwrap().component().unwrap()
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == RADIO)
        .unwrap()
        .parts
}
fn once(h: &mut ContentHost, index: usize) {
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].action.kind, ContentActionKind::Activate);
    let Some(ContentActionPayload::RadioSelection(i)) = &r[0].payload else {
        panic!("typed intent")
    };
    assert_eq!(i.index(), index);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn visible_scroll_pointer_focus_and_release_select_once() {
    let (mut h, _) = fixture();
    let p = parts(&h)[1].frame.center();
    let d = h.event(&ContentEvent::PointerDown(p));
    assert!(d.consumed && d.capture_pointer);
    assert_eq!(h.focused(), Some(RADIO));
    assert_eq!(get(&h).view().focused(), Some(1));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    once(&mut h, 1);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn visible_scroll_ax_focus_differs_from_selected_then_press_once() {
    let (mut h, _) = fixture();
    let p = parts(&h)[1].index;
    assert!(
        h.accessibility_action(RADIO, Some(p), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(h.focused(), Some(RADIO));
    assert_eq!(get(&h).view().focused(), Some(1));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    assert!(
        h.accessibility_action(RADIO, Some(p), AccessibilityAction::Press)
            .activated
    );
    once(&mut h, 1);
    assert!(
        !h.accessibility_action(RADIO, Some(p), AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn visible_scroll_direct_focus_tab_and_original_key_once() {
    for tab in [false, true] {
        let (mut h, mut t) = fixture();
        if tab {
            assert!(h.focus(Some(EDITOR), true));
            h.event_with_text(
                &ContentEvent::Key {
                    key: ControlKey::Tab { reverse: false },
                    down: true,
                },
                &mut t,
            )
            .unwrap();
        } else {
            assert!(h.focus(Some(RADIO), true));
        }
        assert_eq!(h.focused(), Some(RADIO));
        assert_eq!(get(&h).model().selected, Some(0));
        assert!(h.drain_action_records().is_empty());
        assert!(
            h.event(&ContentEvent::Key {
                key: ControlKey::Right,
                down: true
            })
            .activated
        );
        once(&mut h, 1);
    }
}
fn mark(h: &mut ContentHost, t: &mut coretext::Text) {
    assert!(h.focus(Some(EDITOR), true));
    h.event_with_text(
        &ContentEvent::Preedit {
            text: "ni".into(),
            selection: 0..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        },
        t,
    )
    .unwrap();
    assert!(h.is_composing());
}
#[test]
fn marked_composer_strict_ax_and_tab_keep_original_owner_and_no_actions() {
    for path in 0..3 {
        let (mut h, mut t) = fixture();
        mark(&mut h, &mut t);
        let before = h.text_input_snapshot().unwrap();
        let binding = h.focused_text_binding();
        let epoch = h.focus_epoch();
        let token = parts(&h)[1].index;
        if path < 2 {
            let a = if path == 0 {
                AccessibilityAction::Focus
            } else {
                AccessibilityAction::Press
            };
            assert!(!h.accessibility_action(RADIO, Some(token), a).consumed);
        } else {
            h.event_with_text(
                &ContentEvent::Key {
                    key: ControlKey::Tab { reverse: false },
                    down: true,
                },
                &mut t,
            )
            .unwrap();
        }
        assert_eq!(h.focused(), Some(EDITOR));
        assert_eq!(h.focused_text_binding(), binding);
        assert_eq!(h.focus_epoch(), epoch);
        let after = h.text_input_snapshot().unwrap();
        assert_eq!(
            (after.text, after.marked, after.selection),
            (before.text, before.marked, before.selection)
        );
        assert_eq!(get(&h).model().selected, Some(0));
        assert!(h.pointer_capture().is_none());
        assert!(h.drain_action_records().is_empty());
    }
}
#[test]
fn marked_composer_ordinary_pointer_unmarks_then_selects_once() {
    let (mut h, mut t) = fixture();
    mark(&mut h, &mut t);
    let p = parts(&h)[1].frame.center();
    let d = h
        .event_with_text(&ContentEvent::PointerDown(p), &mut t)
        .unwrap();
    assert!(d.consumed && d.capture_pointer);
    assert!(!h.is_composing());
    assert_eq!(h.focused(), Some(RADIO));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    assert!(
        h.event_with_text(&ContentEvent::PointerUp(p), &mut t)
            .unwrap()
            .activated
    );
    once(&mut h, 1);
}

#[test]
fn clipped_scroll_ax_focus_reveals_then_layout_authorizes_press() {
    let (mut h, mut t) = fixture();
    let token = parts(&h)[2].index;
    assert!(
        h.accessibility_action(RADIO, Some(token), AccessibilityAction::Focus)
            .consumed
    );
    assert_eq!(get(&h).view().focused(), Some(2));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    // Focus ring reveal changes the viewport. The existing Host policy requires
    // publication of that layout before another strict geometry transaction.
    assert!(
        !h.accessibility_action(RADIO, Some(token), AccessibilityAction::Press)
            .consumed
    );
    assert!(h.drain_action_records().is_empty());
    h.render(
        &mut Scene::new(),
        &mut t,
        Rect::new(0., 0., 300., 240.),
        Rect::new(0., 0., 300., 240.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    let current = get(&h).part_token(2).unwrap();
    assert!(
        h.accessibility_action(RADIO, Some(current), AccessibilityAction::Press)
            .activated
    );
    once(&mut h, 2);
}
#[test]
fn visible_scroll_controlled_cancel_preserves_focus_and_prevents_late_release() {
    let (mut h, _) = fixture();
    let p = parts(&h)[1].frame.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
    h.get_mut(RADIO)
        .unwrap()
        .component_mut::<RadioGroupContent>()
        .unwrap()
        .set_selected(None)
        .unwrap();
    assert_eq!(get(&h).view().focused(), Some(1));
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(get(&h).model().selected, None);
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn disabled_scroll_part_keeps_host_owner_focus_and_original_disabled_skip() {
    let (mut h, mut t) = fixture();
    h.get_mut(RADIO)
        .unwrap()
        .component_mut::<RadioGroupContent>()
        .unwrap()
        .set_item_enabled(1, false)
        .unwrap();
    h.render(
        &mut Scene::new(),
        &mut t,
        Rect::new(0., 0., 300., 240.),
        Rect::new(0., 0., 300., 240.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    let rows = parts(&h);
    assert!(rows[1].semantics.disabled);
    let p = rows[1].frame.center();
    let down = h.event(&ContentEvent::PointerDown(p));
    assert!(down.consumed && !down.capture_pointer);
    assert_eq!(h.focused(), Some(RADIO));
    assert_eq!(get(&h).view().focused(), Some(0));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert!(h.drain_action_records().is_empty());
    assert!(
        h.event(&ContentEvent::Key {
            key: ControlKey::Right,
            down: true
        })
        .activated
    );
    once(&mut h, 2);
}
