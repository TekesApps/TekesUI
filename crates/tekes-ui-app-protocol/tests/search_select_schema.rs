//! Actual merged PR200 a883d00 production24 decoder, captured before Search/Select.
//! The whole Reply/Input decoder is compiled; no role stripping or reconstructed peer.
#[allow(dead_code)]
#[path = "old_search_select_schema/content.rs"]
mod old_content;
use tekes_ui_app_protocol::component::{VERSION, content as wire};

const PREFIX_ROLES: &[&str] = &[
    "Group", "ProgressIndicator", "Text", "Image", "Button", "Link", "TextInput",
    "ComboBox", "Slider", "Splitter", "Switch", "Checkbox", "RadioGroup",
    "RadioButton", "SegmentedGroup", "Segment", "List", "ListItem", "Dialog",
    "MenuButton", "Menu", "MenuItem", "TabList", "Tab",
];
fn node(role: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "target":{"id":id,"mount":id+100},"parent":null,
        "frame":[0.,0.,100.,30.],"visible":[0.,0.,100.,30.],"focused":false,
        "semantics":{"role":role,"name":"Query","value":null,
            "disabled":false,"selected":false,"expanded":null,"invalid":false,
            "required":false,"description":null},
        "parts":[],"value_range":null,"text":null
    })
}
fn reply(role: &str) -> serde_json::Value {
    serde_json::json!({
        "view":1,"viewport":1,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,
        "focus_targets":[],"composing":false,"text":null,
        "semantics":[node("TextInput",1),node(role,2)]
    })
}
#[test]
fn actual_prefix24_rejects_entire_search_select_root_and_part_snapshots() {
    assert_eq!(VERSION, 2);
    for role in ["SearchInput", "Select"] {
        let value = reply(role);
        let bytes = serde_json::to_vec(&value).unwrap();
        let new: wire::Reply = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::from_slice::<wire::Reply>(
            &serde_json::to_vec(&new).unwrap()).unwrap(), new);
        let error = serde_json::from_slice::<old_content::Reply>(&bytes)
            .unwrap_err().to_string();
        assert!(error.contains(&format!("unknown variant `{role}`")), "{error}");
        let semantics = serde_json::to_vec(new.semantics.as_ref().unwrap()).unwrap();
        assert!(serde_json::from_slice::<Vec<old_content::SemanticNode>>(&semantics).is_err());
        let mut part = reply("Group");
        part["semantics"][1]["parts"] = serde_json::json!([{
            "index":7,"parent":null,"frame":[0.,0.,100.,30.],"focused":false,
            "semantics":node(role,3)["semantics"].clone()
        }]);
        assert!(serde_json::from_value::<wire::Reply>(part.clone()).is_ok());
        assert!(serde_json::from_value::<old_content::Reply>(part).is_err());
    }
}
#[test]
fn all_actual_prefix24_roles_and_existing_expansion_action_remain_compatible() {
    assert_eq!(PREFIX_ROLES.len(), 24);
    for role in PREFIX_ROLES {
        let bytes = serde_json::to_vec(&reply(role)).unwrap();
        let new: wire::Reply = serde_json::from_slice(&bytes).unwrap();
        let old: old_content::Reply = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_value(new).unwrap(), serde_json::to_value(old).unwrap(), "{role}");
    }
    for expanded in [false, true] {
        let input = wire::Input::Accessibility {
            target: wire::Target { id: 41, mount: 92 },
            part: None, binding: None,
            action: wire::AccessibilityAction::SetExpanded(expanded),
        };
        let bytes = serde_json::to_vec(&input).unwrap();
        let new: wire::Input = serde_json::from_slice(&bytes).unwrap();
        let old: old_content::Input = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_value(new).unwrap(), serde_json::to_value(old).unwrap());
    }
}
