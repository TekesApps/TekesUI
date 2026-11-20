use tekes_ui_controls::{TextSurface, composition::*};
use tekes_ui_tokens::proposal;
#[test]
fn ordinary_semantics_default_to_optional_without_creating_validation() {
    assert!(!ContentSemantics::new(ContentRole::TextInput, "Field").required);
    let field = TextFieldContent::new("Name", "").with_required(true);
    let semantics = field.semantics();
    assert!(semantics.required);
    assert!(!semantics.invalid);
    assert_eq!(semantics.value.as_deref(), Some(""));
    assert!(!Button::new("Submit").semantics().required);
}
#[test]
fn required_is_independent_of_readonly_disabled_error_and_multiline_policies() {
    let mut cases = 0;
    for multiline in [false, true] {
        for required in [false, true] {
            for readonly in [false, true] {
                for disabled in [false, true] {
                    for invalid in [false, true] {
                        let field = TextFieldContent::from_surface(
                            "Name",
                            TextSurface::new("Existing", multiline, readonly),
                        )
                        .with_disabled(disabled)
                        .with_required(required)
                        .with_error(invalid.then(|| "Existing validation".into()));
                        let s = field.semantics();
                        assert_eq!(s.required, required);
                        assert_eq!(s.disabled, disabled);
                        assert_eq!(s.invalid, invalid);
                        assert_eq!(s.name, "Name");
                        assert_eq!(s.value.as_deref(), Some("Existing"));
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 32);
}
#[test]
fn content_presentation_keeps_explicit_metadata_without_reintroducing_form_decoration() {
    let mut field =
        TextFieldContent::read_only_content("Document", "Existing", proposal::TextRole::UiBody);
    assert!(!field.semantics().required);
    field.set_required(true);
    assert!(field.semantics().required);
    assert_eq!(
        field.surface_presentation(),
        tekes_ui_controls::TextSurfacePresentation::Content
    );
    assert!(field.is_read_only());
}
