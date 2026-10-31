#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
const EDITOR: ComponentId = ComponentId(1);
const SELECT: ComponentId = ComponentId(2);
fn host() -> (ContentHost, coretext::Text) {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    let mut composer = TextFieldContent::new("Composer", "A😀B");
    composer.submit_policy = TextSubmitPolicy::Enter;
    let mut h = ContentHost::new(Content::new(
        ComponentId(3),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(EDITOR, composer),
                Content::new(
                    SELECT,
                    SelectContent::new("Mode", vec!["One".into(), "Two".into()]),
                ),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, &mut text);
    h.focus(Some(EDITOR), true);
    h.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 0..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    assert!(h.is_composing());
    render(&mut h, &mut text);
    (h, text)
}
fn render(h: &mut ContentHost, t: &mut coretext::Text) {
    let f = Rect::new(0., 0., 320., 200.);
    h.render(&mut Scene::new(), t, f, f, palette()).unwrap();
}
#[test]
fn actual_composer_marked_ax_and_traversal_rejection_passive_updates_preserve_original_owner() {
    let (mut h, mut t) = host();
    let before = h.text_input_snapshot().unwrap();
    let focus = (h.focused(), h.focus_epoch(), h.focused_text_binding());
    for action in [
        AccessibilityAction::Focus,
        AccessibilityAction::SetExpanded(true),
        AccessibilityAction::Press,
    ] {
        assert!(!h.accessibility_action(SELECT, None, action).consumed);
        assert_eq!(h.text_input_snapshot().unwrap(), before);
        assert_eq!(
            (h.focused(), h.focus_epoch(), h.focused_text_binding()),
            focus
        );
        assert!(
            !h.get(SELECT)
                .unwrap()
                .component::<SelectContent>()
                .unwrap()
                .model()
                .menu
                .open
        );
    }
    assert!(!h.enter_focus(false));
    h.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    assert_eq!(h.text_input_snapshot().unwrap(), before);
    h.get_mut(SELECT)
        .unwrap()
        .component_mut::<SelectContent>()
        .unwrap()
        .set_labels(vec!["New".into()])
        .unwrap();
    h.set_inherited_surface("sidebar").unwrap();
    render(&mut h, &mut t);
    assert_eq!(h.text_input_snapshot().unwrap(), before);
    assert_eq!(
        (h.focused(), h.focus_epoch(), h.focused_text_binding()),
        focus
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn cross_owner_set_expanded_rejects_marked_composer_then_opens_and_focuses_once() {
    let (mut h, _text) = host();
    let marked = h.text_input_snapshot().unwrap();
    let initial_focus = (h.focused(), h.focus_epoch(), h.focused_text_binding());

    assert!(
        !h.accessibility_action(SELECT, None, AccessibilityAction::SetExpanded(true))
            .consumed
    );
    assert_eq!(h.text_input_snapshot().unwrap(), marked);
    assert_eq!(
        (h.focused(), h.focus_epoch(), h.focused_text_binding()),
        initial_focus
    );
    assert!(
        !h.get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .model()
            .menu
            .open
    );

    h.event(&ContentEvent::Unmark);
    let unmarked = h.text_input_snapshot().unwrap();
    let before_open_epoch = h.focus_epoch();
    assert!(
        h.accessibility_action(SELECT, None, AccessibilityAction::SetExpanded(true))
            .consumed
    );
    assert!(
        h.get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .model()
            .menu
            .open
    );
    assert_eq!(h.focused(), Some(SELECT));
    let opened_epoch = h.focus_epoch();
    assert!(opened_epoch > before_open_epoch);
    // Focus now belongs to the nontext Select. The focused native text query
    // must be empty; inspect the still-mounted original editor for preservation.
    assert!(h.text_input_snapshot().is_none());
    assert_eq!(
        h.get(EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .text_input_snapshot()
            .unwrap(),
        unmarked
    );

    assert!(
        h.accessibility_action(SELECT, None, AccessibilityAction::SetExpanded(true))
            .consumed
    );
    assert_eq!(h.focused(), Some(SELECT));
    assert_eq!(h.focus_epoch(), opened_epoch);
    assert!(
        h.get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .model()
            .menu
            .open
    );
    // Focus now belongs to the nontext Select. The focused native text query
    // must be empty; inspect the still-mounted original editor for preservation.
    assert!(h.text_input_snapshot().is_none());
    assert_eq!(
        h.get(EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .text_input_snapshot()
            .unwrap(),
        unmarked
    );
    assert!(h.drain_action_records().is_empty());
    assert!(h.drain_actions().is_empty());
}

#[test]
fn actual_native_select_adapter_role_closed_open_same_owner_and_option_once() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as Action, AccessibilityRole, content_accessibility as native,
    };
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t.reduced_motion = true;
    let mut h = ContentHost::new(Content::new(
        SELECT,
        SelectContent::new("Mode", vec!["One".into(), "Two".into()]),
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, &mut t);
    let nodes = native::content_nodes(&h);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].role, AccessibilityRole::PopUpButton);
    assert_eq!(nodes[0].expanded, Some(false));
    h.focus(Some(SELECT), true);
    assert!(!h.focus(Some(SELECT), true));
    assert!(native::content_action(&mut h, nodes[0].key, Action::Press).consumed);
    render(&mut h, &mut t);
    assert!(h.drain_actions().is_empty());
    let nodes = native::content_nodes(&h);
    assert_eq!(nodes[0].expanded, Some(true));
    let option = nodes.iter().find(|n| n.label == "Two").unwrap().key;
    assert!(native::content_action(&mut h, option, Action::Press).activated);
    let actions = h.drain_action_records();
    assert_eq!(actions.len(), 1);
    assert!(h.drain_actions().is_empty());
    assert!(!native::content_action(&mut h, option, Action::Press).consumed);
    assert_eq!(native::content_nodes(&h)[0].value.as_deref(), Some("Two"));
    assert!(native::content_text_layouts(&h).is_empty());
}
#[test]
fn actual_original_reading_fixture_menu_wheel_does_not_record_background_history() {
    use tekes_ui_composition_consumer::nested_reading as reader;
    use tekes_ui_scene::Point;
    let (mut h, _state) = reader::build().unwrap();
    h.get_mut(reader::ROOT)
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .children
        .insert(
            1,
            Content::new(
                SELECT,
                SelectContent::new("Mode", (0..50).map(|n| format!("Option {n}")).collect()),
            ),
        );
    let mut t = coretext::Text::new(1.);
    t.services = true;
    t.reduced_motion = true;
    let frame = Rect::new(0., 0., 600., 440.);
    let paint = |h: &mut ContentHost, t: &mut coretext::Text| {
        h.render(&mut Scene::new(), t, frame, frame, palette())
            .unwrap()
    };
    paint(&mut h, &mut t);
    assert!(h.focus(Some(SELECT), true));
    assert!(
        h.accessibility_action(SELECT, None, AccessibilityAction::Press)
            .consumed
    );
    paint(&mut h, &mut t);
    let point = h
        .get(SELECT)
        .unwrap()
        .component::<SelectContent>()
        .unwrap()
        .layout()
        .popup
        .as_ref()
        .unwrap()
        .visible
        .unwrap()
        .center();
    let outer = format!(
        "{:?}",
        h.get(reader::PARENT)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
    );
    let inner = format!(
        "{:?}",
        h.get(reader::INNER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
    );
    for (i, (precise, begin, end, momentum)) in [
        (false, true, true, false),
        (true, true, false, false),
        (true, false, true, false),
        (true, false, false, true),
    ]
    .into_iter()
    .enumerate()
    {
        h.event(&ContentEvent::Scroll {
            position: point,
            delta: Point::new(0., -30.),
            precise,
            begin,
            end,
            momentum,
            time: i as f64 + 1.,
        });
        assert_eq!(
            format!(
                "{:?}",
                h.get(reader::PARENT)
                    .unwrap()
                    .component::<ScrollContent>()
                    .unwrap()
            ),
            outer
        );
        assert_eq!(
            format!(
                "{:?}",
                h.get(reader::INNER)
                    .unwrap()
                    .component::<TextFieldContent>()
                    .unwrap()
                    .surface
            ),
            inner
        );
    }
    assert!(h.drain_actions().is_empty());
}
#[test]
fn accepted_original_pointer_transfers_marked_composer_and_opens_once() {
    use tekes_ui_scene::Point;
    let (mut h, _t) = host();
    let before = h.text_input_snapshot().unwrap();
    let point = h.get(SELECT).unwrap().frame().center();
    h.event(&ContentEvent::PointerDown(Point::new(point.x, point.y)));
    h.event(&ContentEvent::PointerUp(point));
    assert_eq!(h.focused(), Some(SELECT));
    let editor = h
        .get(EDITOR)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(editor.surface.input.text, before.text);
    assert!(editor.surface.input.marked.is_none());
    assert!(
        h.get(SELECT)
            .unwrap()
            .component::<SelectContent>()
            .unwrap()
            .model()
            .menu
            .open,
        "accepted ordinary pointer must open original Select after transfer"
    );
}

#[test]
fn original_composer_send_gate_and_single_undo_survive_select_confirmation() {
    let enter = || {
        ContentEvent::KeyInput(ContentKeyEvent {
            key: Some(ControlKey::Enter),
            down: true,
            repeat: false,
            modifiers: InputModifiers::default(),
            native_code: 36,
            characters: "\r".into(),
            characters_ignoring_modifiers: "\r".into(),
        })
    };
    let (mut h, mut _text) = host();
    let before = h.text_input_snapshot().unwrap();
    h.event(&enter());
    assert!(h.drain_actions().is_empty());
    assert_eq!(h.text_input_snapshot().unwrap(), before);
    assert!(h.focus(Some(SELECT), true));
    let editor = h
        .get(EDITOR)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(editor.surface.input.text, before.text);
    assert!(editor.surface.input.marked.is_none());
    assert!(
        h.accessibility_action(SELECT, None, AccessibilityAction::Press)
            .consumed
    );
    render(&mut h, &mut _text);
    assert!(h.event(&enter()).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, SELECT);
    assert!(matches!(
        records[0].payload,
        Some(ContentActionPayload::MenuSelection(_))
    ));
    assert!(
        records
            .iter()
            .all(|r| r.action.kind != ContentActionKind::Submit)
    );
    let editor = h
        .get_mut(EDITOR)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    assert!(editor.surface.input.undo());
    assert_eq!(editor.surface.input.text, "A😀B");
    assert!(!editor.surface.input.undo());
    assert!(h.focus(Some(EDITOR), true));
    assert!(h.event(&enter()).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.kind, ContentActionKind::Submit);
}
#[test]
fn original_menu_unknown_strict_action_is_not_default_authorized() {
    use tekes_ui_controls::ScrollArea;
    let id = ComponentId(400);
    let root = ComponentId(401);
    let mut h = ContentHost::new(Content::new(
        root,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(id, MenuContent::new("Menu", vec!["One".into()])),
        ),
    ))
    .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    render(&mut h, &mut text);
    h.set_action_events(true);
    assert!(
        !h.accessibility_action(id, None, AccessibilityAction::Press)
            .consumed
    );
    assert!(
        !h.get(id)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    assert!(h.drain_actions().is_empty());
    assert_eq!(h.focused(), None);
}

#[test]
fn native_empty_and_duplicate_labels_remain_legal_original_select_options() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as Action, AccessibilityRole, content_accessibility as native,
    };
    let mut h = ContentHost::new(Content::new(
        SELECT,
        SelectContent::new("Mode", vec!["".into(), "".into()]),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    render(&mut h, &mut text);
    let root = native::content_nodes(&h)[0].key;
    assert!(native::content_action(&mut h, root, Action::Press).consumed);
    render(&mut h, &mut text);
    let nodes = native::content_nodes(&h);
    let items: Vec<_> = nodes
        .iter()
        .filter(|n| n.role == AccessibilityRole::MenuItem)
        .collect();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|n| n.label.is_empty()));
    assert_ne!(items[0].key, items[1].key);
    assert!(native::content_action(&mut h, items[1].key, Action::Press).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    let Some(ContentActionPayload::MenuSelection(s)) = &records[0].payload else {
        panic!()
    };
    assert_eq!(s.index(), 1);
    assert_eq!(s.label(), "");
}

fn palette() -> tekes_ui_tokens::Palette {
    if std::env::var_os("TEKESUI_SELECT89_EXTERNAL").is_some() {
        tekes_ui_tokens::Palette::for_theme_definition(Theme::LIGHT, "external-inputs")
            .expect("compiled external inputs")
    } else {
        Theme::LIGHT.into()
    }
}
