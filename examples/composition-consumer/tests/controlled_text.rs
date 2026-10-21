use tekes_ui_controls::{
    ControlKey, DraftRevision, ExternalSelection as S, ExternalUndo as U,
    ExternalValueConflict as C, ExternalValueResult as R, TextInput, TextSurface, composition::*,
};
fn value(key: &str, version: u64, text: &str) -> DraftRevision {
    DraftRevision {
        identity: key.into(),
        version,
        payload: text.into(),
    }
}
fn bind(input: &mut TextInput, key: &str, text: &str) {
    let baseline = input.external_baseline();
    assert!(matches!(
        input.apply_external_value(&baseline, &value(key, 1, text), S::Reset, U::Reset),
        R::Applied { .. }
    ));
}
#[test]
fn repeated_revision_does_not_reset_new_local_text_selection_history_or_preedit() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "old");
    let original = input.external_baseline();
    input.select(3, false);
    input.insert("!", None);
    input.preedit("ni", 2..2, None);
    let before = input.external_baseline();
    assert_eq!(
        input.apply_external_value(&original, &value("A", 1, "old"), S::Reset, U::Reset),
        R::Unchanged
    );
    assert_eq!(input.external_baseline(), before);
    assert!(input.marked.is_some());
    input.cancel_composition();
    assert_eq!(input.text, "old!");
    assert!(input.undo());
    assert_eq!(input.text, "old");
}
#[test]
fn an_edit_undo_aba_still_invalidates_the_original_expected_baseline() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "text");
    let baseline = input.external_baseline();
    input.insert("x", None);
    assert!(input.undo());
    assert_eq!(input.text, "text");
    let before = input.external_baseline();
    assert_eq!(
        input.apply_external_value(
            &baseline,
            &value("A", 2, "stale"),
            S::Preserve,
            U::Transaction
        ),
        R::Conflict(C::StaleBaseline)
    );
    assert_eq!(input.external_baseline(), before);
}
#[test]
fn foreign_instances_and_legacy_field_changes_cannot_use_an_old_baseline() {
    let mut a = TextInput::new("same", false);
    let mut b = TextInput::new("same", false);
    let baseline = a.external_baseline();
    assert_eq!(
        b.apply_external_value(&baseline, &value("A", 1, "other"), S::Reset, U::Reset),
        R::Conflict(C::ForeignEditor)
    );
    a.text.push('x');
    assert_eq!(
        a.apply_external_value(&baseline, &value("A", 1, "other"), S::Reset, U::Reset),
        R::Conflict(C::StaleBaseline)
    );
    assert_eq!(a.text, "samex");
    assert_eq!(b.text, "same");
}
#[test]
fn preserve_maps_directed_utf16_endpoints_through_retained_grapheme_text() {
    let mut input = TextInput::new("", true);
    let original = "中文😀e\u{301} tail";
    bind(&mut input, "A", original);
    input.select_range(2..6);
    let baseline = input.external_baseline();
    assert!(matches!(
        input.apply_external_value(
            &baseline,
            &value("A", 2, &format!("新{original}")),
            S::Preserve,
            U::Preserve
        ),
        R::Applied { .. }
    ));
    assert_eq!(input.selection(), 3..7);
    assert_eq!(input.selected_text(), "😀e\u{301}");
    input.anchor = 7;
    input.caret = 3;
    let baseline = input.external_baseline();
    input.apply_external_value(
        &baseline,
        &value("A", 3, &format!("新新{original}")),
        S::Preserve,
        U::Preserve,
    );
    assert_eq!((input.anchor, input.caret), (8, 4));
    assert_eq!(input.selected_text(), "😀e\u{301}");
}
#[test]
fn specified_selection_normalizes_surrogates_and_combining_clusters_atomically() {
    let mut input = TextInput::new("", true);
    let baseline = input.external_baseline();
    input.apply_external_value(
        &baseline,
        &value("A", 1, "😀e\u{301}"),
        S::Set(1..3),
        U::Reset,
    );
    assert_eq!(input.selection(), 0..4);
    assert_eq!(input.selected_text(), "😀e\u{301}");
    let baseline = input.external_baseline();
    let result = input.apply_external_value(
        &baseline,
        &value("A", 2, "😀e\u{301}"),
        S::Set(1..1),
        U::Transaction,
    );
    assert!(matches!(result, R::Applied { .. }));
    assert_eq!(input.selection(), 0..0);
    let before = input.external_baseline();
    assert_eq!(
        input.apply_external_value(&before, &value("A", 3, "x"), S::Set(0..2), U::Reset),
        R::Conflict(C::Selection)
    );
    assert_eq!(input.external_baseline(), before);
}
#[test]
fn confirmed_clear_is_one_transaction_and_a_duplicate_ack_does_not_erase_user_undo() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "hello");
    input.select(5, false);
    input.insert("!", None);
    let baseline = input.external_baseline();
    let clear = value("A", 2, "");
    assert!(matches!(
        input.apply_external_value(&baseline, &clear, S::Reset, U::Transaction),
        R::Applied { .. }
    ));
    assert_eq!(input.text, "");
    assert!(input.undo());
    assert_eq!(input.text, "hello!");
    assert_eq!(input.selection(), 6..6);
    assert_eq!(
        input.apply_external_value(&baseline, &clear, S::Reset, U::Transaction),
        R::Unchanged
    );
    assert_eq!(input.text, "hello!");
    assert!(input.redo());
    assert_eq!(input.text, "");
}
#[test]
fn explicit_document_reset_is_required_and_a_b_a_does_not_reuse_old_history_or_receipts() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "a");
    input.insert("local", None);
    let old_a = input.external_baseline();
    assert_eq!(
        input.apply_external_value(&old_a, &value("B", 1, "b"), S::Reset, U::Preserve),
        R::Conflict(C::DocumentResetRequired)
    );
    assert!(matches!(
        input.apply_external_value(&old_a, &value("B", 1, "b"), S::Reset, U::Reset),
        R::Applied {
            document_changed: true,
            ..
        }
    ));
    assert!(!input.undo());
    assert!(!input.redo());
    assert_eq!(
        input.apply_external_value(&old_a, &value("A", 2, "late"), S::Reset, U::Reset),
        R::Conflict(C::StaleBaseline)
    );
    assert_eq!(input.text, "b");
    let b = input.external_baseline();
    input.apply_external_value(&b, &value("A", 3, "new A"), S::Reset, U::Reset);
    assert!(!input.undo());
    let current = input.external_baseline();
    assert_eq!(
        input.apply_external_value(
            &current,
            &value("A", 2, "old sent text"),
            S::Reset,
            U::Reset
        ),
        R::Conflict(C::Revision)
    );
    assert_eq!(input.text, "new A");
}
#[test]
fn preserve_history_is_explicit_and_never_adds_an_external_undo_step() {
    let mut input = TextInput::new("", true);
    input.insert("a", None);
    input.insert("b", None);
    let baseline = input.external_baseline();
    input.apply_external_value(&baseline, &value("A", 1, "ab"), S::Preserve, U::Preserve);
    assert!(input.undo());
    assert_eq!(input.text, "a");
    assert!(input.redo());
    assert_eq!(input.text, "ab");
    let baseline = input.external_baseline();
    input.apply_external_value(&baseline, &value("A", 2, "external"), S::Reset, U::Preserve);
    assert!(input.undo());
    assert_eq!(input.text, "a");
}
#[test]
fn composition_conflict_is_not_queued_and_retrying_its_old_baseline_cannot_overwrite_local_input() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "old");
    let baseline = input.external_baseline();
    input.preedit("ni", 2..2, None);
    let before = input.external_baseline();
    let update = value("A", 2, "remote");
    assert_eq!(
        input.apply_external_value(&baseline, &update, S::Reset, U::Reset),
        R::Conflict(C::Composition)
    );
    assert_eq!(input.external_baseline(), before);
    input.cancel_composition();
    input.insert("new", None);
    let local = input.text.clone();
    assert_eq!(
        input.apply_external_value(&baseline, &update, S::Reset, U::Transaction),
        R::Conflict(C::StaleBaseline)
    );
    assert_eq!(input.text, local);
    assert_eq!(input.external_value().unwrap().version, 1);
}
#[test]
fn normalization_reuses_native_edit_rules_and_revision_identity_remains_exact() {
    for multiline in [false, true] {
        let mut input = TextInput::new("", multiline);
        let baseline = input.external_baseline();
        let raw = value("A", 1, "中\r\n😀\t");
        input.apply_external_value(&baseline, &raw, S::Reset, U::Reset);
        assert_eq!(input.text, if multiline { "中\n😀\t" } else { "中😀" });
        assert_eq!(
            input.apply_external_value(&baseline, &raw, S::Set(1..2), U::Transaction),
            R::Unchanged
        );
        let canonical = value("A", 1, &input.text);
        let current = input.external_baseline();
        assert_eq!(
            input.apply_external_value(&current, &canonical, S::Preserve, U::Preserve),
            R::Conflict(C::Revision)
        );
    }
}
#[test]
fn unchanged_and_conflicting_surface_updates_preserve_all_derived_state() {
    let mut surface = TextSurface::new("", true, false);
    let baseline = surface.input.external_baseline();
    surface.apply_external_value(&baseline, &value("A", 1, "original"), S::Reset, U::Reset);
    surface.input.preedit("ni", 2..2, None);
    surface.clauses = std::iter::once(0..2).collect();
    let before = format!("{surface:?}");
    assert_eq!(
        surface.apply_external_value(&baseline, &value("A", 1, "original"), S::Reset, U::Reset),
        R::Unchanged
    );
    assert_eq!(format!("{surface:?}"), before);
    let current = surface.input.external_baseline();
    assert_eq!(
        surface.apply_external_value(&current, &value("A", 2, "remote"), S::Reset, U::Reset),
        R::Conflict(C::Composition)
    );
    assert_eq!(format!("{surface:?}"), before);
}
#[test]
fn a_controlled_revision_or_document_change_fences_the_native_key_even_without_a_focus_change() {
    for key in ["A", "B"] {
        let id = ComponentId(700);
        let mut field = TextFieldContent::new("editor", "old");
        field.submit_policy = TextSubmitPolicy::Enter;
        let baseline = field.surface.input.external_baseline();
        field
            .surface
            .apply_external_value(&baseline, &value("A", 1, "old"), S::Reset, U::Reset);
        let mut host = ContentHost::new(Content::new(id, field)).unwrap();
        host.focus(Some(id), true);
        let input = ContentKeyEvent {
            key: Some(ControlKey::Enter),
            down: true,
            repeat: false,
            modifiers: InputModifiers::default(),
            native_code: 36,
            characters: "\r".into(),
            characters_ignoring_modifiers: "\r".into(),
        };
        host.event(&ContentEvent::TextKeyStart {
            id: 1,
            key: input,
            composing: false,
        });
        let f = host
            .get_mut(id)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap();
        let expected = f.surface.input.external_baseline();
        f.surface
            .apply_external_value(&expected, &value(key, 2, "old"), S::Reset, U::Reset);
        host.event(&ContentEvent::Commit {
            text: "late".into(),
            replacement: None,
        });
        host.event(&ContentEvent::TextCommand("insertNewline:".into()));
        let response = host.event(&ContentEvent::TextKeyEnd {
            id: 1,
            input_context_consumed: false,
        });
        assert!(!response.activated);
        assert_eq!(
            response.text_key.unwrap().disposition,
            TextKeyDisposition::Cancelled
        );
        assert_eq!(host.focused_text().unwrap().input.text, "old");
    }
}

#[test]
fn presentation_disabling_does_not_block_an_authorized_external_update() {
    let mut input = TextInput::new("", true);
    bind(&mut input, "A", "old");
    let baseline = input.external_baseline();
    input.disabled = true;
    input.read_only = true;
    assert!(matches!(
        input.apply_external_value(
            &baseline,
            &value("A", 2, "authoritative"),
            S::Reset,
            U::Transaction
        ),
        R::Applied { .. }
    ));
    assert_eq!(input.text, "authoritative");
    assert!(input.disabled);
    assert!(input.read_only);
}
