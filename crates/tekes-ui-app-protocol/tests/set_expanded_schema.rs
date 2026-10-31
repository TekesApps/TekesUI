//! Complete request compatibility for the original disclosure action.
#[path = "old_combo_schema/content.rs"]
mod old_content;
use tekes_ui_app_protocol::component::{VERSION, content as wire};

#[test]
fn set_expanded_complete_input_roundtrips_and_old_decoder_rejects_it() {
    assert_eq!(VERSION, 2);
    for expanded in [true, false] {
        let request = wire::Input::Accessibility {
            target: wire::Target { id: 41, mount: 92 },
            part: Some(7),
            binding: Some(vec![9, 42]),
            action: wire::AccessibilityAction::SetExpanded(expanded),
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: wire::Input = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, request);
        assert!(wire::AccessibilityAction::SetExpanded(expanded).is_valid());
        let error = serde_json::from_slice::<old_content::Input>(&bytes)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown variant `SetExpanded`"), "{error}");
    }
}
