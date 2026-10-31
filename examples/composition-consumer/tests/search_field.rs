//! Independent public API regression. These tests require #101's SearchInput
//! closure; no fallback TextInput or test-only fake Search role is permitted.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::search_field::*;
use tekes_ui_controls::{
    ControlKey, DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, ScrollArea,
    composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Theme;

fn render(f: &mut SearchFixture) {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let rect = Rect::new(20., 30., 360., 300.);
    f.host
        .render(
            &mut Scene::new(),
            &mut text,
            rect,
            rect,
            Theme::LIGHT.into(),
        )
        .unwrap();
}
fn clear_point(f: &SearchFixture) -> Point {
    f.host
        .semantics()
        .into_iter()
        .find(|n| n.id == SEARCH)
        .unwrap()
        .parts[0]
        .frame
        .center()
}
fn key(repeat: bool) -> ContentKeyEvent {
    ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat,
        modifiers: InputModifiers::default(),
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    }
}
fn commit(f: &mut SearchFixture, text: &str) {
    f.host.event(&ContentEvent::Commit {
        text: text.into(),
        replacement: None,
    });
}
fn action(f: &SearchFixture, kind: SearchCommandKind, query: &str, source: ContentActionSource) {
    let record = f.delivered.last().unwrap();
    assert_eq!(record.action.target, SEARCH);
    assert_eq!(record.action.source, source);
    assert_eq!(
        record.action.kind,
        if kind == SearchCommandKind::Submit {
            ContentActionKind::Submit
        } else {
            ContentActionKind::Clear
        }
    );
    let Some(ContentActionPayload::Search(a)) = &record.payload else {
        panic!("original Search payload missing")
    };
    assert_eq!(a.kind, kind);
    assert_eq!(a.query, query);
}
#[derive(Debug, PartialEq, Eq)]
struct InputState {
    text: String,
    selection: std::ops::Range<usize>,
    marked: Option<std::ops::Range<usize>>,
    focused: Option<ComponentId>,
    binding: Option<Vec<u8>>,
}
fn state(f: &SearchFixture) -> InputState {
    let input = f.search().input();
    InputState {
        text: input.text.clone(),
        selection: input.selection(),
        marked: input.marked.clone(),
        focused: f.host.focused(),
        binding: f
            .host
            .semantic_text_input_snapshot(SEARCH)
            .map(|s| s.binding),
    }
}

#[test]
fn keyboard_submit_and_pointer_clear_are_once_only_and_keep_the_original_owner() {
    let mut f = SearchFixture::new("A😀B");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let binding = f.host.semantic_text_input_snapshot(SEARCH).unwrap().binding;
    f.host.event(&ContentEvent::KeyInput(key(false)));
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    action(
        &f,
        SearchCommandKind::Submit,
        "A😀B",
        ContentActionSource::Keyboard,
    );
    render(&mut f);
    let point = clear_point(&f);
    f.host.event(&ContentEvent::PointerDown(point));
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    f.host.event(&ContentEvent::PointerUp(point));
    f.collect();
    assert_eq!(f.delivered.len(), 2);
    action(
        &f,
        SearchCommandKind::Clear,
        "A😀B",
        ContentActionSource::Pointer,
    );
    assert_eq!(f.search().query(), "");
    assert_eq!(f.search().input().selection(), 0..0);
    assert_eq!(f.host.focused(), Some(SEARCH));
    assert_eq!(
        f.host
            .semantic_text_input_snapshot(SEARCH)
            .map(|s| s.binding),
        Some(binding)
    );
    assert!(f.host.drain_actions().is_empty());
    f.host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(f.search().query(), "A😀B");
    f.host.event(&ContentEvent::TextCommand("redo:".into()));
    assert_eq!(f.search().query(), "");
    f.collect();
    assert_eq!(f.delivered.len(), 2);
}

#[test]
fn clear_focus_policy_does_not_leak_to_the_next_editor_click() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    f.host.focus(Some(OTHER), true);
    let point = clear_point(&f);
    f.host.event(&ContentEvent::PointerDown(point));
    f.host.event(&ContentEvent::PointerUp(point));
    assert_eq!(f.host.focused(), Some(OTHER));
    render(&mut f);
    let editor = f.host.text_surface(SEARCH).unwrap().inner.center();
    f.host.event(&ContentEvent::PointerDown(editor));
    assert_eq!(
        f.host.focused(),
        Some(SEARCH),
        "previous clear must not preserve OTHER for an editor click"
    );
    f.host.event(&ContentEvent::PointerUp(editor));
}

#[test]
fn typed_after_accepted_submit_keeps_immutable_delivery_query_and_mount() {
    let mut f = SearchFixture::new("before");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    f.host.event(&ContentEvent::KeyInput(key(false)));
    commit(&mut f, "after");
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    action(
        &f,
        SearchCommandKind::Submit,
        "before",
        ContentActionSource::Keyboard,
    );
}

#[test]
fn ax_and_programmatic_clear_keep_other_focus_and_have_real_sources() {
    for source in [
        ContentActionSource::Accessibility,
        ContentActionSource::Programmatic,
    ] {
        let mut f = SearchFixture::new("original");
        render(&mut f);
        f.host.focus(Some(OTHER), true);
        if source == ContentActionSource::Accessibility {
            assert!(
                f.host
                    .accessibility_action(SEARCH, Some(0), AccessibilityAction::Press)
                    .activated
            );
        } else {
            let command = f
                .host
                .capture_search_command(SEARCH, SearchCommandKind::Clear)
                .unwrap();
            assert!(f.host.execute_search_command(command).unwrap().activated);
        }
        f.collect();
        assert_eq!(f.delivered.len(), 1);
        action(&f, SearchCommandKind::Clear, "original", source);
        assert_eq!(f.host.focused(), Some(OTHER));
        assert_eq!(f.search().query(), "");
        let command = f
            .host
            .capture_search_command(SEARCH, SearchCommandKind::Clear)
            .unwrap();
        assert_eq!(
            f.host.execute_search_command(command),
            Err(SearchCommandError::EmptyQuery)
        );
        assert!(f.host.drain_actions().is_empty());
    }
}

#[test]
fn another_marked_editor_blocks_all_clear_sources_without_touching_either_owner() {
    for source in 0..3 {
        let mut f = SearchFixture::new("keep");
        render(&mut f);
        let point = clear_point(&f);
        f.host.focus(Some(OTHER), true);
        f.host.event(&ContentEvent::Preedit {
            text: "输入".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        });
        let before = state(&f);
        let other = f.host.semantic_text_input_snapshot(OTHER).unwrap();
        match source {
            0 => {
                f.host.event(&ContentEvent::PointerDown(point));
                f.host.event(&ContentEvent::PointerUp(point));
            }
            1 => {
                assert!(
                    !f.host
                        .accessibility_action(SEARCH, Some(0), AccessibilityAction::Press)
                        .activated
                );
            }
            _ => {
                let c = f
                    .host
                    .capture_search_command(SEARCH, SearchCommandKind::Clear)
                    .unwrap();
                assert_eq!(
                    f.host.execute_search_command(c),
                    Err(SearchCommandError::Composition)
                );
            }
        }
        assert_eq!(state(&f), before);
        assert_eq!(f.host.semantic_text_input_snapshot(OTHER).unwrap(), other);
        assert!(f.host.drain_actions().is_empty());
    }
}

#[test]
fn submit_none_command_enter_repeat_and_raw_key_keep_original_gate() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    f.host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    f.host.event(&ContentEvent::KeyInput(key(true)));
    f.search_mut().set_submit_policy(TextSubmitPolicy::None);
    f.host.event(&ContentEvent::KeyInput(key(false)));
    assert!(f.host.drain_actions().is_empty());
    f.search_mut()
        .set_submit_policy(TextSubmitPolicy::CommandEnter);
    f.host.event(&ContentEvent::KeyInput(key(false)));
    assert!(f.host.drain_actions().is_empty());
    let mut command = key(false);
    command.modifiers.command = true;
    f.host.event(&ContentEvent::KeyInput(command));
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    action(
        &f,
        SearchCommandKind::Submit,
        "draft",
        ContentActionSource::Keyboard,
    );
}

#[test]
fn native_start_end_consumption_and_marked_enter_do_not_submit_twice() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    f.host.event(&ContentEvent::TextKeyStart {
        id: 1,
        key: key(false),
        composing: false,
    });
    f.host
        .event(&ContentEvent::TextCommand("insertNewline:".into()));
    f.host.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    f.host.event(&ContentEvent::TextKeyEnd {
        id: 1,
        input_context_consumed: false,
    });
    f.host.event(&ContentEvent::TextKeyStart {
        id: 2,
        key: key(false),
        composing: false,
    });
    f.host.event(&ContentEvent::TextKeyEnd {
        id: 2,
        input_context_consumed: true,
    });
    f.host.event(&ContentEvent::Preedit {
        text: "拼".into(),
        selection: 1..1,
        clauses: std::iter::once(0..1).collect(),
        replacement: None,
    });
    let before = state(&f);
    f.host.event(&ContentEvent::KeyInput(key(false)));
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    assert_eq!(
        f.host.execute_search_command(command),
        Err(SearchCommandError::Composition)
    );
    assert_eq!(state(&f), before);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn stale_baseline_including_edit_undo_aba_is_atomic() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    commit(&mut f, "new");
    f.host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(f.search().query(), "draft");
    let before = state(&f);
    assert_eq!(
        f.host.execute_search_command(command),
        Err(SearchCommandError::StaleBaseline)
    );
    assert_eq!(state(&f), before);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn remove_reinsert_and_replace_same_id_revoke_old_commands_and_pending_actions() {
    for replace in [false, true] {
        let mut f = SearchFixture::new("old");
        render(&mut f);
        let command = f
            .host
            .capture_search_command(SEARCH, SearchCommandKind::Clear)
            .unwrap();
        let submit = f
            .host
            .capture_search_command(SEARCH, SearchCommandKind::Submit)
            .unwrap();
        f.host.execute_search_command(submit).unwrap();
        let removed = f.host.remove(SEARCH).unwrap();
        let new = if replace {
            Content::new(SEARCH, SearchFieldContent::new("replacement", "fresh"))
        } else {
            removed
        };
        f.host.insert(ROOT, new).unwrap();
        let before = state(&f);
        assert_eq!(
            f.host.execute_search_command(command),
            Err(SearchCommandError::StaleMount)
        );
        assert_eq!(state(&f), before);
        assert!(f.host.drain_actions().is_empty());
    }
}

#[test]
fn reorder_preserves_command_and_current_availability_rejects_pending_submit() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Submit)
        .unwrap();
    f.host.reorder(ROOT, &[HISTORY, OTHER, SEARCH]).unwrap();
    f.host.execute_search_command(command).unwrap();
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Submit)
        .unwrap();
    f.host.execute_search_command(command).unwrap();
    f.search_mut().set_availability(true, false);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn readonly_disabled_external_conflicts_and_new_document_binding_are_preserved() {
    for (readonly, disabled) in [(true, false), (false, true)] {
        let mut f = SearchFixture::new("keep");
        render(&mut f);
        f.host.focus(Some(SEARCH), true);
        f.search_mut().set_availability(readonly, disabled);
        let command = f
            .host
            .capture_search_command(SEARCH, SearchCommandKind::Clear)
            .unwrap();
        let before = state(&f);
        assert_eq!(
            f.host.execute_search_command(command),
            Err(SearchCommandError::Unavailable)
        );
        assert_eq!(state(&f), before);
        let baseline = f.search().input().external_baseline();
        let revision = DraftRevision {
            identity: "query".into(),
            version: 1,
            payload: "external".into(),
        };
        assert!(matches!(
            f.search_mut().apply_query(
                &baseline,
                &revision,
                ExternalSelection::Reset,
                ExternalUndo::Reset
            ),
            ExternalValueResult::Applied { .. }
        ));
        let before = state(&f);
        let invalid = DraftRevision {
            identity: "".into(),
            version: 0,
            payload: "bad".into(),
        };
        assert!(matches!(
            f.search_mut().apply_query(
                &baseline,
                &invalid,
                ExternalSelection::Reset,
                ExternalUndo::Reset
            ),
            ExternalValueResult::Conflict(_)
        ));
        assert_eq!(state(&f), before);
        assert!(f.host.drain_actions().is_empty());
    }
    let mut f = SearchFixture::new("keep");
    render(&mut f);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    let baseline = f.search().input().external_baseline();
    f.search_mut().apply_query(
        &baseline,
        &DraftRevision {
            identity: "query".into(),
            version: 1,
            payload: "new document".into(),
        },
        ExternalSelection::Reset,
        ExternalUndo::Reset,
    );
    let before = state(&f);
    assert_eq!(
        f.host.execute_search_command(command),
        Err(SearchCommandError::StaleEditor)
    );
    assert_eq!(state(&f), before);
}

#[test]
fn selection_backspace_and_reading_sibling_keep_original_state() {
    let mut f = SearchFixture::new("A😀B");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    assert!(
        f.host
            .accessibility_action(SEARCH, None, AccessibilityAction::SetSelection(1..3))
            .consumed
    );
    f.host
        .event(&ContentEvent::TextCommand("deleteBackward:".into()));
    assert_eq!(f.search().query(), "AB");
    {
        let history = f
            .host
            .get_mut(HISTORY)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        history.surface.input.select_range(3..8);
        history.surface.scroll.scroll_by(0., 120.);
        history
            .reading
            .as_mut()
            .unwrap()
            .stop_following(&mut history.surface);
    }

    assert!(
        f.host
            .get(HISTORY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .scroll
            .offset
            .y
            > 0.
    );
    let history_before = f.host.semantic_text_input_snapshot(HISTORY).unwrap();
    let scroll = f
        .host
        .get(HISTORY)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .scroll
        .offset;
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    f.host.execute_search_command(command).unwrap();
    assert_eq!(
        f.host.semantic_text_input_snapshot(HISTORY).unwrap(),
        history_before
    );
    assert_eq!(
        f.host
            .get(HISTORY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .scroll
            .offset,
        scroll
    );
    assert!(
        f.host
            .get(HISTORY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .reading
            .is_some()
    );
}

#[test]
fn clip_empty_clear_and_invalid_metadata_do_not_activate() {
    let mut f = SearchFixture::new("");
    render(&mut f);
    assert!(
        f.host
            .semantics()
            .into_iter()
            .find(|n| n.id == SEARCH)
            .unwrap()
            .parts
            .is_empty()
    );
    assert!(
        !f.host
            .accessibility_action(SEARCH, Some(0), AccessibilityAction::Press)
            .activated
    );
    let before = state(&f);
    assert_eq!(
        f.search_mut().set_clear_label("   "),
        Err(CompositionError::EmptyAccessibleName)
    );
    assert_eq!(state(&f), before);
    for width in [0., 8., 32.] {
        let mut text = coretext::Text::new(1.);
        text.services = true;
        let frame = Rect::new(20., 30., width, 300.);
        let clip = Rect::new(20., 30., width / 2., 300.);
        f.host
            .render(
                &mut Scene::new(),
                &mut text,
                frame,
                clip,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(f.host.drain_actions().is_empty());
    }
    assert_eq!(f.search().query(), "");
}

#[test]
fn inactive_window_rejects_captured_request_before_side_effects() {
    let mut f = SearchFixture::new("keep");
    render(&mut f);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    f.host.set_window_active(false);
    let query = f.search().query().to_owned();
    let baseline = f.search().input().external_baseline();
    let focus = f.host.focused();
    assert_eq!(
        f.host.execute_search_command(command),
        Err(SearchCommandError::Unavailable)
    );
    assert_eq!(f.search().query(), query);
    assert_eq!(f.search().input().external_baseline(), baseline);
    assert_eq!(f.host.focused(), focus);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn modal_scope_rejects_background_commands_and_undrained_submit() {
    let mut f = SearchFixture::new("keep");
    render(&mut f);
    let clear = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    let submit = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Submit)
        .unwrap();
    f.host.execute_search_command(submit).unwrap();
    let dialog = ComponentId(8890);
    f.host
        .insert(
            ROOT,
            Content::new(
                dialog,
                DialogContent::new(
                    "Modal",
                    Content::new(ComponentId(8891), TextContent::new("Header")),
                    Content::new(ComponentId(8892), TextContent::new("Body")),
                    Content::new(ComponentId(8893), Button::new("Close")),
                ),
            ),
        )
        .unwrap();
    f.host.show_dialog(dialog).unwrap();
    let baseline = f.search().input().external_baseline();
    let focus = f.host.focused();
    assert_eq!(
        f.host.execute_search_command(clear),
        Err(SearchCommandError::Unavailable)
    );
    assert_eq!(f.search().input().external_baseline(), baseline);
    assert_eq!(f.host.focused(), focus);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn current_search_semantics_and_text_queries_are_the_same_original_owner() {
    let mut f = SearchFixture::new("A😀B");
    render(&mut f);
    let node = f
        .host
        .semantics()
        .into_iter()
        .find(|n| n.id == SEARCH)
        .unwrap();
    assert_eq!(node.semantics.role, ContentRole::SearchInput);
    assert_eq!(node.semantics.value.as_deref(), Some("A😀B"));
    let input = f.host.semantic_text_input_snapshot(SEARCH).unwrap();
    let layout = f.host.semantic_text_layout_snapshot(SEARCH, None).unwrap();
    assert_eq!(input.binding, layout.input.binding);
    assert_eq!(input.text, f.search().input().text);
    assert_eq!(layout.input.selection, f.search().input().selection());
    assert!(
        node.parts
            .iter()
            .all(|p| p.semantics.role == ContentRole::Button)
    );
}

#[test]
fn controlled_query_refuses_active_preedit_without_changing_selection_or_focus() {
    let mut f = SearchFixture::new("A😀B");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let baseline = f.search().input().external_baseline();
    f.host.event(&ContentEvent::Preedit {
        text: "输入".into(),
        selection: 1..1,
        clauses: vec![0..1, 1..2],
        replacement: Some(1..3),
    });
    let before = state(&f);
    let revision = DraftRevision {
        identity: "query".into(),
        version: 1,
        payload: "late external".into(),
    };
    assert_eq!(
        f.search_mut().apply_query(
            &baseline,
            &revision,
            ExternalSelection::Reset,
            ExternalUndo::Transaction
        ),
        ExternalValueResult::Conflict(tekes_ui_controls::ExternalValueConflict::Composition)
    );
    assert_eq!(state(&f), before);
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn local_command_cannot_clear_between_original_native_key_start_and_end() {
    let mut f = SearchFixture::new("draft");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let command = f
        .host
        .capture_search_command(SEARCH, SearchCommandKind::Clear)
        .unwrap();
    f.host.event(&ContentEvent::TextKeyStart {
        id: 71,
        key: key(false),
        composing: false,
    });
    let before = state(&f);
    assert_eq!(
        f.host.execute_search_command(command),
        Err(SearchCommandError::InputInProgress)
    );
    assert_eq!(state(&f), before);
    assert!(f.host.drain_actions().is_empty());
    f.host
        .event(&ContentEvent::TextCommand("insertNewline:".into()));
    f.host.event(&ContentEvent::TextKeyEnd {
        id: 71,
        input_context_consumed: false,
    });
    f.collect();
    assert_eq!(f.delivered.len(), 1);
    action(
        &f,
        SearchCommandKind::Submit,
        "draft",
        ContentActionSource::Keyboard,
    );
}

#[test]
fn actual_search_adapter_projects_native_search_text_and_clear_action() {
    use tekes_ui_platform_macos::{
        AccessibilityAction as NativeAction, AccessibilityRole, content_accessibility::*,
    };
    let mut f = SearchFixture::new("A😀B");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let key = content_key(&f.host, SEARCH, None).unwrap();
    let nodes = content_nodes(&f.host);
    let node = nodes.iter().find(|n| n.key == key).unwrap();
    assert_eq!(node.role, AccessibilityRole::SearchField);
    assert_eq!(node.value.as_deref(), Some("A😀B"));
    assert!(node.focused);
    let layouts = content_text_layouts(&f.host);
    assert_eq!(layouts[&key].state.text, "A😀B");
    assert!(content_action(&mut f.host, key, NativeAction::SetSelection(1..3)).consumed);
    assert_eq!(f.search().input().selection(), 1..3);
    let clear = content_key(&f.host, SEARCH, Some(0)).unwrap();
    assert!(content_action(&mut f.host, clear, NativeAction::Press).activated);
    f.collect();
    action(
        &f,
        SearchCommandKind::Clear,
        "A😀B",
        ContentActionSource::Accessibility,
    );
    assert_eq!(f.host.focused(), Some(SEARCH));
    assert!(!content_action(&mut f.host, clear, NativeAction::Press).activated);
    let removed = f.host.remove(SEARCH).unwrap();
    f.host.insert(ROOT, removed).unwrap();
    assert!(!content_action(&mut f.host, key, NativeAction::SetValue("stale".into())).consumed);
    assert_eq!(f.search().query(), "");
    f.search_mut().set_availability(true, false);
    render(&mut f);
    let node = content_nodes(&f.host)
        .into_iter()
        .find(|n| n.key == content_key(&f.host, SEARCH, None).unwrap())
        .unwrap();
    assert!(node.read_only);
    assert_eq!(node.role, AccessibilityRole::SearchField);
    f.search_mut().set_availability(false, true);
    render(&mut f);
    let node = content_nodes(&f.host)
        .into_iter()
        .find(|n| n.key == content_key(&f.host, SEARCH, None).unwrap())
        .unwrap();
    assert!(!node.enabled);
    assert_eq!(node.role, AccessibilityRole::SearchField);
}

#[test]
fn accepted_external_publication_revokes_old_clear_press_without_stealing_focus() {
    let mut f = SearchFixture::new("old document");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let point = clear_point(&f);
    f.host.event(&ContentEvent::PointerDown(point));
    let baseline = f.search().input().external_baseline();
    let publication = DraftRevision {
        identity: "replacement".into(),
        version: 1,
        payload: "new document".into(),
    };
    assert!(matches!(
        f.search_mut().apply_query(
            &baseline,
            &publication,
            ExternalSelection::Reset,
            ExternalUndo::Reset
        ),
        ExternalValueResult::Applied { .. }
    ));
    f.host.event(&ContentEvent::PointerUp(point));
    assert_eq!(f.search().query(), "new document");
    assert_eq!(f.host.focused(), Some(SEARCH));
    assert!(f.host.drain_actions().is_empty());
}

#[test]
fn refused_external_publication_does_not_cancel_existing_clear_gesture() {
    let mut f = SearchFixture::new("original");
    render(&mut f);
    f.host.focus(Some(SEARCH), true);
    let point = clear_point(&f);
    f.host.event(&ContentEvent::PointerDown(point));
    let before = state(&f);
    let capture = f.host.pointer_capture();
    let baseline = f.search().input().external_baseline();
    let invalid = DraftRevision {
        identity: "".into(),
        version: 0,
        payload: "illegal".into(),
    };
    assert!(matches!(
        f.search_mut().apply_query(
            &baseline,
            &invalid,
            ExternalSelection::Reset,
            ExternalUndo::Reset
        ),
        ExternalValueResult::Conflict(_)
    ));
    assert_eq!(state(&f), before);
    assert_eq!(f.host.pointer_capture(), capture);
    f.host.event(&ContentEvent::PointerUp(point));
    f.collect();
    action(
        &f,
        SearchCommandKind::Clear,
        "original",
        ContentActionSource::Pointer,
    );
}

const STRICT_ROOT: ComponentId = ComponentId(8810);
const STRICT_STACK: ComponentId = ComponentId(8812);
const STRICT_SPACER: ComponentId = ComponentId(8813);
fn strict_search_host() -> ContentHost {
    let mut search = SearchFieldContent::new("Search documents", "needle");
    search.set_submit_policy(TextSubmitPolicy::Enter);
    let mut stack = StackContent {
        model: tekes_ui_controls::extended::Stack {
            axis: tekes_ui_controls::extended::Axis::Vertical,
            gap: 8.,
        },
        flexible_child: None,
        children: vec![Content::new(
            OTHER,
            TextFieldContent::new("Composer", "draft 😀"),
        )],
    };
    stack.children.extend((0..18).map(|i| {
        Content::new(
            ComponentId(8820 + i),
            TextContent::new(format!("scroll row {i}")),
        )
    }));
    stack
        .children
        .push(Content::new(STRICT_SPACER, TextContent::new("spacer")));
    stack.children.push(Content::new(SEARCH, search));
    let mut scroll = ScrollContent::new(ScrollArea::default(), Content::new(STRICT_STACK, stack));
    scroll.viewport_height = Some(120.);
    let mut host = ContentHost::new(Content::new(STRICT_ROOT, scroll)).unwrap();
    host.set_action_events(true);
    host
}
fn render_strict(host: &mut ContentHost) {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(20., 30., 360., 120.);
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn strict_scroll_offset(host: &ContentHost) -> Point {
    host.get(STRICT_ROOT)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
}

#[test]
fn strict_scroll_search_admits_original_editor_actions_and_clear_once() {
    let mut host = strict_search_host();
    render_strict(&mut host);
    assert_eq!(strict_scroll_offset(&host), Point::new(0., 0.));
    assert!(host.focus(Some(OTHER), true));
    assert!(
        host.accessibility_action(OTHER, None, AccessibilityAction::SetSelection(1..4))
            .consumed
    );
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "拼音".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        })
        .consumed
    );
    let composer_before = host.semantic_text_input_snapshot(OTHER).unwrap();
    let search_before = host.semantic_text_input_snapshot(SEARCH).unwrap();
    let offset_before = strict_scroll_offset(&host);
    let refused = host.accessibility_action(SEARCH, None, AccessibilityAction::SetSelection(0..3));
    assert!(!refused.consumed && !refused.changed);
    assert_eq!(host.focused(), Some(OTHER));
    assert_eq!(
        host.semantic_text_input_snapshot(OTHER).unwrap(),
        composer_before
    );
    assert_eq!(
        host.semantic_text_input_snapshot(SEARCH).unwrap(),
        search_before
    );
    assert_eq!(strict_scroll_offset(&host), offset_before);
    assert!(host.drain_action_records().is_empty());

    assert!(
        host.event(&ContentEvent::Commit {
            text: "拼音".into(),
            replacement: None,
        })
        .consumed
    );
    assert!(
        host.accessibility_action(OTHER, None, AccessibilityAction::SetSelection(1..3))
            .consumed
    );
    let composer_selection = host.text_surface(OTHER).unwrap().input.selection();
    let before_reveal = strict_scroll_offset(&host);
    let selected = host.accessibility_action(SEARCH, None, AccessibilityAction::SetSelection(1..4));
    assert!(selected.consumed && selected.changed);
    assert_eq!(host.focused(), Some(SEARCH));
    assert_ne!(strict_scroll_offset(&host), before_reveal);
    assert_eq!(
        host.text_surface(OTHER).unwrap().input.selection(),
        composer_selection
    );
    assert!(host.drain_action_records().is_empty());

    assert!(host.focus(None, false));
    render_strict(&mut host);
    let point = host
        .semantics()
        .into_iter()
        .find(|node| node.id == SEARCH)
        .unwrap()
        .parts[0]
        .frame
        .center();
    host.event(&ContentEvent::PointerDown(point));
    assert_eq!(
        host.pointer_capture().map(|capture| capture.0),
        Some(SEARCH)
    );
    assert_eq!(host.focused(), None);
    assert!(host.drain_action_records().is_empty());
    host.event(&ContentEvent::PointerUp(point));
    assert!(host.pointer_capture().is_none());
    assert_eq!(host.focused(), None);
    let delivered = host.drain_action_records();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].action.target, SEARCH);
    assert_eq!(delivered[0].action.kind, ContentActionKind::Clear);
    assert_eq!(delivered[0].action.source, ContentActionSource::Pointer);
    let Some(ContentActionPayload::Search(action)) = &delivered[0].payload else {
        panic!("original Search clear payload missing")
    };
    assert_eq!(action.kind, SearchCommandKind::Clear);
    assert_eq!(action.query, "needle");
    assert_eq!(
        host.get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
            .query(),
        ""
    );
    let after_clear = host.semantic_text_input_snapshot(SEARCH).unwrap();
    let refused_clear = host.accessibility_action(SEARCH, Some(0), AccessibilityAction::Press);
    assert!(!refused_clear.consumed && !refused_clear.changed && !refused_clear.activated);
    assert_eq!(
        host.semantic_text_input_snapshot(SEARCH).unwrap(),
        after_clear
    );
    assert!(host.drain_action_records().is_empty());
}

#[test]
fn strict_scroll_clear_sources_respect_another_composers_mark_and_then_clear_once() {
    for clear_source in [
        ContentActionSource::Accessibility,
        ContentActionSource::Pointer,
    ] {
        let mut host = strict_search_host();
        render_strict(&mut host);
        assert!(host.focus(Some(OTHER), true));
        assert!(
            host.event(&ContentEvent::Preedit {
                text: "拼音".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: None,
            })
            .consumed
        );
        host.get_mut(STRICT_ROOT)
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y = 10_000.;
        render_strict(&mut host);
        let point = host
            .semantics()
            .into_iter()
            .find(|node| node.id == SEARCH)
            .unwrap()
            .parts[0]
            .frame
            .center();
        let composer_before = host.semantic_text_input_snapshot(OTHER).unwrap();
        let search_before = host.semantic_text_input_snapshot(SEARCH).unwrap();
        let offset_before = strict_scroll_offset(&host);
        let refused = host.accessibility_action(SEARCH, Some(0), AccessibilityAction::Press);
        assert!(!refused.consumed && !refused.changed && !refused.activated);
        host.event(&ContentEvent::PointerDown(point));
        host.event(&ContentEvent::PointerUp(point));
        assert_eq!(host.focused(), Some(OTHER));
        assert_eq!(
            host.semantic_text_input_snapshot(OTHER).unwrap(),
            composer_before
        );
        assert_eq!(
            host.semantic_text_input_snapshot(SEARCH).unwrap(),
            search_before
        );
        assert_eq!(strict_scroll_offset(&host), offset_before);
        assert!(host.drain_action_records().is_empty());

        assert!(
            host.event(&ContentEvent::Commit {
                text: "拼音".into(),
                replacement: None,
            })
            .consumed
        );
        let composer_selection = host.text_surface(OTHER).unwrap().input.selection();
        match clear_source {
            ContentActionSource::Accessibility => {
                assert!(
                    host.accessibility_action(SEARCH, Some(0), AccessibilityAction::Press)
                        .activated
                );
            }
            ContentActionSource::Pointer => {
                host.event(&ContentEvent::PointerDown(point));
                assert_eq!(
                    host.pointer_capture().map(|capture| capture.0),
                    Some(SEARCH)
                );
                host.event(&ContentEvent::PointerUp(point));
            }
            _ => unreachable!(),
        }
        assert_eq!(host.focused(), Some(OTHER));
        assert_eq!(
            host.text_surface(OTHER).unwrap().input.selection(),
            composer_selection
        );
        assert_eq!(
            host.get(SEARCH)
                .unwrap()
                .component::<SearchFieldContent>()
                .unwrap()
                .query(),
            ""
        );
        let records = host.drain_action_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action.target, SEARCH);
        assert_eq!(records[0].action.kind, ContentActionKind::Clear);
        assert_eq!(records[0].action.source, clear_source);
        let Some(ContentActionPayload::Search(action)) = &records[0].payload else {
            panic!("original Search clear payload missing")
        };
        assert_eq!(action.kind, SearchCommandKind::Clear);
        assert_eq!(action.query, "needle");
        assert!(host.drain_action_records().is_empty());
    }
}

#[test]
fn strict_scroll_editor_drag_captured_by_search_cannot_clear_when_crossing_button() {
    let mut host = strict_search_host();
    render_strict(&mut host);
    assert!(host.focus(Some(OTHER), true));
    host.get_mut(STRICT_ROOT)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y = 10_000.;
    render_strict(&mut host);
    let editor = host.text_surface(SEARCH).unwrap().inner;
    let down = Point::new(editor.min_x() + 3., editor.center().y);
    let clear = host
        .semantics()
        .into_iter()
        .find(|node| node.id == SEARCH)
        .unwrap()
        .parts[0]
        .frame
        .center();
    let before = host
        .get(SEARCH)
        .unwrap()
        .component::<SearchFieldContent>()
        .unwrap()
        .input()
        .selection();
    assert!(host.event(&ContentEvent::PointerDown(down)).capture_pointer);
    assert_eq!(host.focused(), Some(SEARCH));
    let lease = host.pointer_capture_lease();
    assert!(lease.is_some_and(|(id, _, _)| id == SEARCH));
    host.event(&ContentEvent::PointerMove(clear));
    host.event(&ContentEvent::PointerUp(clear));
    assert!(host.pointer_capture_lease().is_none());
    let selection_after_up = host
        .get(SEARCH)
        .unwrap()
        .component::<SearchFieldContent>()
        .unwrap()
        .input()
        .selection();
    assert!(selection_after_up.start < selection_after_up.end);
    assert_ne!(selection_after_up, before);
    host.event(&ContentEvent::PointerMove(clear));
    let search = host
        .get(SEARCH)
        .unwrap()
        .component::<SearchFieldContent>()
        .unwrap();
    assert_eq!(
        search.input().selection(),
        selection_after_up,
        "released drag no longer updates editor selection"
    );
    assert_eq!(search.query(), "needle");
    assert!(host.drain_action_records().is_empty());
}
