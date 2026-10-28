use tekes_ui_controls::TextInput;

#[test]
fn repeated_combining_preedit_replaces_only_owned_text_and_commit_is_one_undo() {
    let mut input = TextInput::new("AeB", false);
    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    assert_eq!(input.text, "Ae\u{301}B");
    assert!(input.preedit("\u{300}", 1..1, None));
    assert_eq!(input.text, "Ae\u{300}B");

    assert!(input.insert("\u{300}", None));
    assert_eq!(input.text, "Ae\u{300}B");
    assert!(input.undo());
    assert_eq!(input.text, "AeB");
    assert!(input.redo());
    assert_eq!(input.text, "Ae\u{300}B");
}

#[test]
fn first_combining_preedit_can_commit_without_replacing_its_base_character() {
    let mut input = TextInput::new("AeB", false);
    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    assert!(input.insert("\u{301}", None));
    assert_eq!(input.text, "Ae\u{301}B");
    assert!(input.undo());
    assert_eq!(input.text, "AeB");
    assert!(input.redo());
    assert_eq!(input.text, "Ae\u{301}B");
}

#[test]
fn cancel_after_attaching_mark_preserves_original_selection_and_neighbors() {
    let mut input = TextInput::new("AeB", false);
    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    assert!(input.preedit("\u{300}", 1..1, None));
    assert!(input.cancel_composition());
    assert_eq!(input.text, "AeB");
    assert_eq!(input.selection(), 2..2);
    assert!(input.marked.is_none());
    assert!(!input.undo());
}

#[test]
fn variation_selector_and_nonzero_surrogate_selection_keep_surrounding_text() {
    let mut variation = TextInput::new("A✈B", false);
    variation.select(2, false);
    assert!(variation.preedit("\u{fe0f}", 1..1, None));
    assert!(variation.preedit("\u{fe0e}", 1..1, None));
    assert_eq!(variation.text, "A✈\u{fe0e}B");
    assert!(variation.cancel_composition());
    assert_eq!(variation.text, "A✈B");

    let mut selected = TextInput::new("A😀B", false);
    assert!(selected.select_range(1..3));
    assert!(selected.preedit("👩", 2..2, None));
    assert!(selected.preedit("👩\u{fe0f}", 3..3, None));
    assert_eq!(selected.text, "A👩\u{fe0f}B");
    assert!(selected.cancel_composition());
    assert_eq!(selected.text, "A😀B");
    assert_eq!(selected.selection(), 1..3);
}

#[test]
fn zwj_composition_leaves_following_insert_and_delete_on_grapheme_edges() {
    let mut input = TextInput::new("👩👧", false);
    input.select(2, false);
    assert!(input.preedit("\u{200d}", 1..1, None));
    assert!(input.preedit("\u{200d}", 1..1, None));
    assert!(input.insert("\u{200d}", None));
    assert_eq!(input.text, "👩‍👧");
    assert!(input.insert("X", None));
    assert_eq!(input.text, "👩‍👧X");
    assert!(input.delete(true, false));
    assert_eq!(input.text, "👩‍👧");
    assert!(input.delete(true, false));
    assert_eq!(input.text, "");
}

#[test]
fn composition_respects_disabled_and_read_only_and_plain_edit_undo_direction() {
    for read_only in [false, true] {
        let mut input = TextInput::new("A😀e\u{301}B", false);
        input.read_only = read_only;
        input.disabled = !read_only;
        assert!(!input.preedit("\u{300}", 1..1, None));
        assert_eq!(input.text, "A😀e\u{301}B");
    }

    let mut input = TextInput::new("A😀e\u{301}B", false);
    input.select(1, false);
    assert!(input.insert("X", None));
    assert_eq!(input.text, "AX😀e\u{301}B");
    assert!(input.undo());
    assert_eq!(input.text, "A😀e\u{301}B");
    assert!(input.redo());
    assert_eq!(input.text, "AX😀e\u{301}B");
    input.select(2, false);
    assert!(input.delete(false, false));
    assert_eq!(input.text, "AXe\u{301}B");
}

#[test]
fn explicit_native_owned_ranges_are_exact_and_unrelated_replacements_keep_their_scope() {
    let mut input = TextInput::new("AeB", false);
    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    let owned = input.marked.clone().unwrap();
    assert!(input.preedit("\u{300}", 1..1, Some(owned)));
    assert_eq!(input.text, "Ae\u{300}B");
    let owned = input.marked.clone().unwrap();
    assert!(input.insert("\u{301}", Some(owned)));
    assert_eq!(input.text, "Ae\u{301}B");
    assert!(input.undo());
    assert_eq!(input.text, "AeB");

    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    assert!(input.insert("X", Some(0..1)));
    assert_eq!(input.text, "Xe\u{301}B");
    assert!(input.undo());
    assert_eq!(input.text, "AeB");
}

#[test]
fn ordinary_zwj_insert_maps_the_next_edit_and_history_to_grapheme_boundaries() {
    let mut input = TextInput::new("👩👧", false);
    input.select(2, false);
    assert!(input.insert("\u{200d}", None));
    assert_eq!(input.text, "👩‍👧");
    assert_eq!(input.selection(), 5..5);
    assert!(input.insert("X", None));
    assert_eq!(input.text, "👩‍👧X");
    assert!(input.undo());
    assert_eq!(input.text, "👩‍👧");
    assert_eq!(input.selection(), 5..5);
    assert!(input.undo());
    assert_eq!(input.text, "👩👧");
    assert!(input.redo());
    assert_eq!(input.text, "👩‍👧");
    assert_eq!(input.selection(), 5..5);
}
