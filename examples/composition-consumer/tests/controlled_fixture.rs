use tekes_ui_composition_consumer::controlled_editor::*;
use tekes_ui_controls::{ControlKey, ExternalValueResult as R, composition::ContentEvent as E};
fn activate(f: &mut ControlledEditorFixture, id: tekes_ui_controls::composition::ComponentId) {
    assert!(f.host.focus(Some(id), true));
    f.event(&E::Key {
        key: ControlKey::Enter,
        down: true,
    });
}
#[test]
fn actual_buttons_drive_versioned_clear_undo_and_document_isolation() {
    let mut f = ControlledEditorFixture::new();
    let original = f.editor().input.text.clone();
    activate(&mut f, CLEAR);
    assert!(f.editor().input.text.is_empty());
    f.event(&E::TextCommand("undo:".into()));
    assert_eq!(f.editor().input.text, original);
    activate(&mut f, SWITCH);
    assert_eq!(f.authority().identity, "B");
    assert_eq!(f.editor().input.text, "B 草稿");
    assert!(!f.event(&E::TextCommand("undo:".into())).changed);
    activate(&mut f, SWITCH);
    assert_eq!(f.authority().identity, "A");
    assert_eq!(f.editor().input.text, original);
}
#[test]
fn actual_delayed_authority_response_cannot_overwrite_local_edits_or_another_document() {
    let mut f = ControlledEditorFixture::new();
    activate(&mut f, CAPTURE);
    let captured = f.captured_value().unwrap().clone();
    f.event(&E::Commit {
        text: "new".into(),
        replacement: None,
    });
    let local = f.editor().input.text.clone();
    assert!(f.authority().version > captured.version);
    activate(&mut f, APPLY);
    assert!(matches!(f.last_result, Some(R::Conflict(_))));
    assert_eq!(f.editor().input.text, local);
    activate(&mut f, SWITCH);
    let b = f.editor().input.text.clone();
    activate(&mut f, APPLY);
    assert!(matches!(f.last_result, Some(R::Conflict(_))));
    assert_eq!(f.editor().input.text, b);
}

#[test]
fn actual_increment_button_maps_the_existing_utf16_selection_without_resetting_history() {
    let mut f = ControlledEditorFixture::new();
    let original = f.editor().input.text.clone();
    f.event(&E::TextCommand("selectAll:".into()));
    let before = f.editor().input.selection();
    activate(&mut f, PREFIX);
    assert_eq!(f.editor().input.text, format!("前{original}"));
    assert_eq!(
        f.editor().input.selection(),
        before.start + 1..before.end + 1
    );
    assert_eq!(f.editor().input.selected_text(), original);
}
