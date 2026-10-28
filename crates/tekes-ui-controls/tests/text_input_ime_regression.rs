use tekes_ui_controls::TextInput;

#[test]
fn original_repro_repeated_combining_preedit_preserves_base_and_neighbors() {
    let mut input = TextInput::new("AeB", false);
    input.select(2, false);
    assert!(input.preedit("\u{301}", 1..1, None));
    assert!(input.preedit("\u{300}", 1..1, None));
    assert_eq!(input.text, "Ae\u{300}B");
}
