//! Actual ce69 Combo17 whole decoder, captured byte-for-byte before Selection22.
//! See old_selection_schema/manifest.json for immutable source/fixture identities.
//! Its content/geometry/input structs are compiled, not a hand-written role imitation.
#[allow(dead_code)]
#[path = "old_selection_schema/content.rs"]
mod old_content;
use tekes_ui_app_protocol::component::{VERSION, content as wire};

fn reply(role: &str) -> serde_json::Value {
    serde_json::json!({
        "view":1,"viewport":1,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,
        "focus_targets":[],"composing":false,"text":null,
        "semantics":[node("TextInput", 1), node(role, 2)]
    })
}
fn node(role: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "target":{"id":id,"mount":id+100},"parent":null,
        "frame":[0.,0.,100.,30.],"visible":[0.,0.,100.,30.],"focused":false,
        "semantics":{"role":role,"name":"Query","value":"A😀B",
        "disabled":false,"selected":false,"expanded":null,"invalid":false,
        "required":false,"description":null},"parts":[],"value_range":null,"text":null
    })
}
#[test]
fn real_old_reply_decoder_rejects_entire_selection_snapshot() {
    for role in [
        "Checkbox",
        "RadioGroup",
        "RadioButton",
        "SegmentedGroup",
        "Segment",
    ] {
        assert_eq!(VERSION, 2);
        let encoded = serde_json::to_vec(&reply(role)).unwrap();
        let new: wire::Reply = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            new.semantics.as_ref().unwrap()[1].semantics.role,
            serde_json::from_value(serde_json::json!(role)).unwrap()
        );
        assert_eq!(
            serde_json::from_slice::<wire::Reply>(&serde_json::to_vec(&new).unwrap()).unwrap(),
            new
        );
        let error = serde_json::from_slice::<old_content::Reply>(&encoded)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("unknown variant `{role}`")),
            "{error}"
        );
        // The receiver never receives a partial Vec/Reply even with a valid first node.
        let mut part_reply = reply("Group");
        part_reply["semantics"][1]["parts"] = serde_json::json!([{
            "index":0,"parent":null,"frame":[0.,0.,20.,20.],"focused":false,
            "semantics":node(role,3)["semantics"].clone()
        }]);
        assert!(serde_json::from_value::<wire::Reply>(part_reply.clone()).is_ok());
        assert!(serde_json::from_value::<old_content::Reply>(part_reply).is_err());
        let nodes = serde_json::to_vec(new.semantics.as_ref().unwrap()).unwrap();
        assert!(serde_json::from_slice::<Vec<old_content::SemanticNode>>(&nodes).is_err());
    }
}
#[test]
fn old_text_input_encoding_and_both_decoders_are_unchanged() {
    assert_eq!(
        serde_json::to_string(&wire::SemanticRole::TextInput).unwrap(),
        "\"TextInput\""
    );
    let encoded = serde_json::to_vec(&reply("TextInput")).unwrap();
    let new: wire::Reply = serde_json::from_slice(&encoded).unwrap();
    let old: old_content::Reply = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&new).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
}

#[test]
fn actual_combo17_roles_and_set_expanded_remain_compatible() {
    // These are the actual pre-Selection peer roles, not a simulated role decoder.
    for role in [
        "Group",
        "ProgressIndicator",
        "Text",
        "Image",
        "Button",
        "Link",
        "TextInput",
        "ComboBox",
        "Slider",
        "Splitter",
        "Switch",
        "List",
        "ListItem",
        "Dialog",
        "MenuButton",
        "Menu",
        "MenuItem",
    ] {
        let encoded = serde_json::to_vec(&reply(role)).unwrap();
        let new: wire::Reply = serde_json::from_slice(&encoded).unwrap();
        let old: old_content::Reply = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            serde_json::to_value(new).unwrap(),
            serde_json::to_value(old).unwrap(),
            "{role}"
        );
    }
    let request = wire::Input::Accessibility {
        target: wire::Target { id: 41, mount: 92 },
        part: None,
        binding: None,
        action: wire::AccessibilityAction::SetExpanded(true),
    };
    let encoded = serde_json::to_vec(&request).unwrap();
    let new: wire::Input = serde_json::from_slice(&encoded).unwrap();
    let old: old_content::Input = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(new).unwrap(),
        serde_json::to_value(old).unwrap()
    );
}
