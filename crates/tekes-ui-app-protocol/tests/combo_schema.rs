//! The decoder below is a capture of the actual current pre-ComboBox schema (source hashes in handoff).
//! Its content/geometry/input structs are compiled, not a hand-written role imitation.
#[path = "old_combo_schema/content.rs"]
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
fn real_old_reply_decoder_rejects_entire_new_combo_snapshot() {
    assert_eq!(VERSION, 2);
    let encoded = serde_json::to_vec(&reply("ComboBox")).unwrap();
    let new: wire::Reply = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        new.semantics.as_ref().unwrap()[1].semantics.role,
        wire::SemanticRole::ComboBox
    );
    assert_eq!(
        serde_json::from_slice::<wire::Reply>(&serde_json::to_vec(&new).unwrap()).unwrap(),
        new
    );
    let error = serde_json::from_slice::<old_content::Reply>(&encoded)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant `ComboBox`"), "{error}");
    // The receiver never receives a partial Vec/Reply even with a valid first node.
    let nodes = serde_json::to_vec(new.semantics.as_ref().unwrap()).unwrap();
    assert!(serde_json::from_slice::<Vec<old_content::SemanticNode>>(&nodes).is_err());
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
fn complete_actual_old_decoder_rejects_new_part_after_valid_prefix() {
    let mut value = reply("TextInput");
    let mut part = node("ComboBox", 9);
    let object = part.as_object_mut().unwrap();
    for key in ["target", "parent", "visible", "text", "value_range"] {
        object.remove(key);
    }
    // SemanticPart has its own exact schema, not the root's fields.
    value["semantics"][1]["parts"] = serde_json::json!([{
        "index":7,"parent":null,"frame":[0.,0.,100.,30.],"focused":false,
        "semantics":part["semantics"],"preserves_activation_focus":false
    }]);
    let bytes = serde_json::to_vec(&value).unwrap();
    let new: wire::Reply = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        new.semantics.unwrap()[1].parts[0].semantics.role,
        wire::SemanticRole::ComboBox
    );
    let error = serde_json::from_slice::<old_content::Reply>(&bytes)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant `ComboBox`"), "{error}");
}
#[test]
fn all_existing_roles_keep_exact_json_and_both_complete_decoders() {
    use wire::SemanticRole as R;
    for role in [
        R::Group,
        R::ProgressIndicator,
        R::Text,
        R::Image,
        R::Button,
        R::Link,
        R::TextInput,
        R::Slider,
        R::Splitter,
        R::Switch,
        R::List,
        R::ListItem,
        R::Dialog,
        R::MenuButton,
        R::Menu,
        R::MenuItem,
    ] {
        let mut value = reply("TextInput");
        value["semantics"][1]["semantics"]["role"] = serde_json::to_value(role).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        let new: wire::Reply = serde_json::from_slice(&bytes).unwrap();
        let old: old_content::Reply = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(new).unwrap(),
            serde_json::to_value(old).unwrap(),
            "{role:?}"
        );
    }
}
