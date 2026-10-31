use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, ControlKey, composition::*};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), 20.)
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
fn select() -> SelectContent {
    SelectContent::new("Mode", vec!["One".into(), "Two".into()])
}
fn render(h: &mut ContentHost) {
    let f = Rect::new(0., 0., 320., 260.);
    h.render(&mut Scene::new(), &mut Text, f, f, palette())
        .unwrap();
}
fn host() -> ContentHost {
    let mut h = ContentHost::new(Content::new(ComponentId(89), select())).unwrap();
    h.set_action_events(true);
    render(&mut h);
    assert!(h.focus(Some(ComponentId(89)), true));
    h
}
fn open(h: &mut ContentHost) {
    assert!(
        h.accessibility_action(ComponentId(89), None, AccessibilityAction::Press)
            .consumed
    );
    render(h);
}
fn part(h: &ContentHost, index: usize) -> usize {
    h.get(ComponentId(89))
        .unwrap()
        .component::<SelectContent>()
        .unwrap()
        .semantic_parts()[index + 1]
        .index
}
fn owner(h: &mut ContentHost) -> &mut SelectContent {
    h.get_mut(ComponentId(89))
        .unwrap()
        .component_mut::<SelectContent>()
        .unwrap()
}
#[test]
fn public_fixed_labels_raw_selection_clamp_empty_duplicates_and_no_user_setter_action() {
    let mut h = host();
    assert!(owner(&mut h).set_selected(1));
    assert_eq!(owner(&mut h).set_labels(vec!["".into()]), Ok(true));
    assert_eq!(owner(&mut h).model().selected, 1);
    assert_eq!(owner(&mut h).semantics().value.as_deref(), Some(""));
    assert!(!owner(&mut h).set_selected(1));
    assert_eq!(
        owner(&mut h).set_labels(vec!["".into(), "".into()]),
        Ok(true)
    );
    assert_eq!(
        owner(&mut h).set_labels(vec!["".into(), "".into()]),
        Ok(false)
    );
    assert_eq!(owner(&mut h).set_labels(vec![]), Ok(true));
    assert!(!owner(&mut h).focusable());
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn public_original_choice_once_same_value_and_payload_survives_close() {
    let mut h = host();
    for _ in 0..2 {
        open(&mut h);
        let token = part(&h, 0);
        assert!(
            h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press)
                .activated
        );
        let a = h.drain_action_records();
        assert_eq!(a.len(), 1);
        let Some(ContentActionPayload::MenuSelection(s)) = &a[0].payload else {
            panic!()
        };
        assert_eq!((s.index(), s.label(), s.value_path()), (0, "One", None));
        assert!(h.drain_actions().is_empty());
        assert!(
            !h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press)
                .consumed
        );
    }
}
#[test]
fn public_reopen_and_aba_revoke_old_parts_new_input_valid() {
    let mut h = host();
    open(&mut h);
    let old = part(&h, 1);
    h.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    open(&mut h);
    assert_ne!(old, part(&h, 1));
    assert!(
        !h.accessibility_action(ComponentId(89), Some(old), AccessibilityAction::Press)
            .consumed
    );
    let second = part(&h, 1);
    owner(&mut h)
        .set_labels(vec!["B".into(), "C".into()])
        .unwrap();
    owner(&mut h)
        .set_labels(vec!["One".into(), "Two".into()])
        .unwrap();
    render(&mut h);
    open(&mut h);
    assert_ne!(part(&h, 1), second);
    assert!(
        !h.accessibility_action(ComponentId(89), Some(second), AccessibilityAction::Focus)
            .consumed
    );
    let current = part(&h, 1);
    assert!(
        h.accessibility_action(ComponentId(89), Some(current), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(owner(&mut h).model().selected, 1);
}
#[test]
fn public_changed_declaration_discards_queued_payload_equal_keeps_it() {
    let mut h = host();
    open(&mut h);
    let token = part(&h, 1);
    h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press);
    assert_eq!(
        owner(&mut h).set_labels(vec!["One".into(), "Two".into()]),
        Ok(false)
    );
    assert_eq!(h.drain_action_records().len(), 1);
    open(&mut h);
    let token = part(&h, 1);
    h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press);
    owner(&mut h).set_labels(vec!["New".into()]).unwrap();
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn public_late_release_after_labels_update_cannot_open_trigger() {
    let mut h = host();
    let p = Point::new(20., 20.);
    h.event(&ContentEvent::PointerDown(p));
    owner(&mut h).set_labels(vec!["New".into()]).unwrap();
    render(&mut h);
    h.event(&ContentEvent::PointerUp(p));
    assert!(!owner(&mut h).model().menu.open);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn public_disabled_and_error_project_and_old_pending_action_is_revoked() {
    let mut h = host();
    open(&mut h);
    let token = part(&h, 0);
    h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press);
    owner(&mut h).set_error(true);
    owner(&mut h).set_disabled(true);
    assert!(h.drain_action_records().is_empty());
    assert!(owner(&mut h).semantics().invalid);
    assert!(
        !h.accessibility_action(ComponentId(89), None, AccessibilityAction::Press)
            .consumed
    );
    owner(&mut h).set_disabled(false);
    render(&mut h);
    open(&mut h);
    assert!(owner(&mut h).model().menu.open);
}
use tekes_ui_controls::layout::MeasuredChild;
#[derive(Debug)]
struct Overlap {
    children: Vec<Content>,
}
impl Component for Overlap {
    // This actual fixture only arranges its original children; it has no
    // mutable focus hooks or reentry and declares the accepted98 contract.
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }

    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, _: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(320., 260.))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, _: Rect, c: Rect) {
        self.children[0].arrange(t, Rect::new(20., 20., 200., 40.), c);
        self.children[1].arrange(t, Rect::new(0., 65., 310., 160.), c);
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "")
    }
}
fn overlap() -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        ComponentId(90),
        Overlap {
            children: vec![
                Content::new(ComponentId(89), select()),
                Content::new(ComponentId(91), Button::new("Behind popup")),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h);
    h.focus(Some(ComponentId(89)), true);
    h
}
#[test]
fn public_overlay_inside_click_is_single_original_choice_and_outside_down_up_is_barrier() {
    let mut h = overlap();
    open(&mut h);
    let row = owner(&mut h).layout().popup.as_ref().unwrap().items[1]
        .frame
        .center();
    h.event(&ContentEvent::PointerDown(row));
    assert_eq!(h.pointer_capture().unwrap().0, ComponentId(89));
    h.event(&ContentEvent::PointerUp(row));
    let actions = h.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, ComponentId(89));
    assert_eq!(owner(&mut h).model().selected, 1);
    open(&mut h);
    let outside = Point::new(305., 210.);
    assert!(
        !owner(&mut h)
            .layout()
            .popup
            .as_ref()
            .unwrap()
            .popup
            .contains(outside)
    );
    let before = format!("{:?}", owner(&mut h));
    let response = h.event(&ContentEvent::PointerDown(outside));
    assert!(
        !owner(&mut h).model().menu.open,
        "before={before} response={response:?} after={:?}",
        owner(&mut h)
    );
    assert_eq!(h.pointer_capture().unwrap().0, ComponentId(89));
    h.event(&ContentEvent::PointerUp(outside));
    assert!(h.drain_actions().is_empty());
    assert!(h.pointer_capture().is_none());
}
#[test]
fn public_tab_closes_popup_and_reaches_original_next_focus_owner() {
    let mut h = overlap();
    open(&mut h);
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert!(!owner(&mut h).model().menu.open);
    assert_eq!(h.focused(), Some(ComponentId(91)));
    assert!(h.drain_actions().is_empty());
}
struct Motion(u64);
impl ButtonTextRenderer for Motion {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Text.measure(s, r, w)
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
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        Some(tekes_ui_controls::ControlMotionContext {
            now: std::time::Duration::from_millis(self.0),
            reduced_motion: false,
        })
    }
}
#[test]
fn public_outgoing_presence_only_paints_and_does_not_keep_parts_or_input_barrier() {
    let mut h = overlap();
    let mut t = Motion(0);
    let f = Rect::new(0., 0., 320., 260.);
    let paint = |h: &mut ContentHost, t: &mut Motion| {
        h.render(&mut Scene::new(), t, f, f, palette()).unwrap()
    };
    paint(&mut h, &mut t);
    open(&mut h);
    paint(&mut h, &mut t);
    t.0 = 500;
    paint(&mut h, &mut t);
    let token = part(&h, 0);
    assert!(
        h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press)
            .activated
    );
    t.0 = 516;
    paint(&mut h, &mut t);
    assert!(!owner(&mut h).overlay_open());
    assert!(owner(&mut h).overlay_presented());
    assert!(owner(&mut h).semantic_parts().is_empty());
    assert!(
        !h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press)
            .consumed
    );
    assert_eq!(h.drain_action_records().len(), 1);
    assert!(h.drain_actions().is_empty());
    let p = Point::new(305., 210.);
    h.event(&ContentEvent::PointerDown(p));
    h.event(&ContentEvent::PointerUp(p));
    let actions = h.drain_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].target, ComponentId(91));
}

#[test]
fn original_mount_reinsertion_hidden_and_inactive_revoke_old_popup_parts_and_queue() {
    let mut h = overlap();
    open(&mut h);
    let old = part(&h, 1);
    let mount = h.semantic_mount(ComponentId(89));
    let retained = h.remove(ComponentId(89)).unwrap();
    h.insert(ComponentId(90), retained).unwrap();
    render(&mut h);
    assert_ne!(h.semantic_mount(ComponentId(89)), mount);
    assert!(
        !h.accessibility_action(ComponentId(89), Some(old), AccessibilityAction::Press)
            .consumed
    );
    h.focus(Some(ComponentId(89)), true);
    open(&mut h);
    let new = part(&h, 1);
    assert_ne!(old, new);
    let f = Rect::new(0., 0., 320., 260.);
    h.render(&mut Scene::new(), &mut Text, f, Rect::default(), palette())
        .unwrap();
    assert!(!owner(&mut h).model().menu.open);
    assert!(
        !h.accessibility_action(ComponentId(89), Some(new), AccessibilityAction::Press)
            .consumed
    );
    render(&mut h);
    h.focus(Some(ComponentId(89)), true);
    open(&mut h);
    h.set_window_active(false);
    assert!(!owner(&mut h).model().menu.open);
    assert!(
        !h.accessibility_action(ComponentId(89), Some(new), AccessibilityAction::Press)
            .consumed
    );
    h.set_window_active(true);
    render(&mut h);
    open(&mut h);
    let token = part(&h, 0);
    assert!(
        h.accessibility_action(ComponentId(89), Some(token), AccessibilityAction::Press)
            .activated
    );
    h.remove(ComponentId(89)).unwrap();
    assert!(h.drain_action_records().is_empty());
}

fn palette() -> tekes_ui_tokens::Palette {
    if std::env::var_os("TEKESUI_SELECT89_EXTERNAL").is_some() {
        tekes_ui_tokens::Palette::for_theme_definition(Theme::LIGHT, "external-inputs")
            .expect("compiled external inputs")
    } else {
        Theme::LIGHT.into()
    }
}

#[test]
fn public_blur_refocus_reopens_without_a_render_and_retires_old_parts() {
    let mut h = host();
    let id = ComponentId(89);
    let mut previous = None;
    for _ in 0..2 {
        assert!(h.focus(None, false));
        assert!(
            !h.get(id)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model()
                .menu
                .open
        );
        assert!(
            h.get(id)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .semantic_parts()
                .is_empty()
        );
        // A valid rendered allocation is retained. No render occurs between
        // leaving focus, returning to Select, and the checked root AX open.
        assert!(h.focus(Some(id), true));
        assert!(
            h.accessibility_action(id, None, AccessibilityAction::Press)
                .consumed
        );
        assert!(
            h.get(id)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model()
                .menu
                .open
        );
        assert!(h.drain_action_records().is_empty());
        render(&mut h);
        let current = part(&h, 1);
        if let Some(old) = previous.replace(current) {
            assert_ne!(current, old);
            assert!(
                !h.accessibility_action(id, Some(old), AccessibilityAction::Press)
                    .consumed
            );
        }
    }
    let current = previous.unwrap();
    assert!(
        h.accessibility_action(id, Some(current), AccessibilityAction::Press)
            .activated
    );
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    let Some(ContentActionPayload::MenuSelection(selection)) = &records[0].payload else {
        panic!("one original Select confirmation");
    };
    assert_eq!((selection.index(), selection.label()), (1, "Two"));
    assert!(h.drain_action_records().is_empty());
    h.set_window_active(false);
    assert!(
        !h.accessibility_action(id, None, AccessibilityAction::Press)
            .consumed
    );
    assert!(
        h.get(id)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .semantic_parts()
            .is_empty()
    );
    assert!(h.drain_action_records().is_empty());
}
