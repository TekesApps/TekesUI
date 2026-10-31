//! Actual6ec7a53 prior26 complete decoder versus Collections33.
//! Keeps actualNav/Search/Combo/Selection26 and SetExpanded; only new7 roles reject.
#[allow(dead_code)]
#[path = "old_collections26_schema/content.rs"]
mod old_content;

use tekes_ui_app_protocol::component::{VERSION, content as wire};

const BASELINE_AND_PEER_ROLES: &[&str] = &[
    "Group",
    "ProgressIndicator",
    "Text",
    "Image",
    "Button",
    "Link",
    "TextInput",
    "SearchInput",
    "ComboBox",
    "Slider",
    "Splitter",
    "Switch",
    "Checkbox",
    "RadioGroup",
    "RadioButton",
    "SegmentedGroup",
    "Segment",
    "List",
    "ListItem",
    "Dialog",
    "MenuButton",
    "Select",
    "Menu",
    "MenuItem",
    "TabList",
    "Tab",
];

const NEW_COLLECTION_ROLES: &[&str] = &[
    "Tree",
    "TreeItem",
    "Table",
    "TableColumn",
    "TableColumnHeader",
    "TableRow",
    "TableCell",
];

fn node_with_role(role: serde_json::Value, id: u64) -> serde_json::Value {
    serde_json::json!({
        "target":{"id":id,"mount":id+100},"parent":null,
        "frame":[0.,0.,100.,30.],"visible":[0.,0.,100.,30.],"focused":false,
        "semantics":{"role":role,"name":"Query","value":"A😀B",
            "disabled":false,"selected":false,"expanded":null,"invalid":false,
            "required":false,"description":null},
        "parts":[],"value_range":null,"text":null
    })
}

fn node(role: &str, id: u64) -> serde_json::Value {
    node_with_role(serde_json::json!(role), id)
}

fn role_value(role: &str) -> serde_json::Value {
    match role {
        "Tree" | "Table" => serde_json::json!(role),
        "TreeItem" => serde_json::json!({"TreeItem":{"level":0}}),
        "TableColumn" => serde_json::json!({"TableColumn":{"index":0}}),
        "TableColumnHeader" => serde_json::json!({
            "TableColumnHeader":{"index":0,"sort":"Descending"}
        }),
        "TableRow" => serde_json::json!({"TableRow":{"index":0}}),
        "TableCell" => serde_json::json!({"TableCell":{"row":0,"column":0}}),
        _ => serde_json::json!(role),
    }
}

fn reply_with_role(role: &str) -> serde_json::Value {
    let mut value = serde_json::json!({
        "view":1,"viewport":1,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,
        "focus_targets":[],"composing":false,"text":null,
        "semantics":[node("TextInput",1),node("TextInput",2)]
    });
    value["semantics"][1]["semantics"]["role"] = role_value(role);
    value
}

fn reply_with_part(role: &str) -> serde_json::Value {
    let mut value = reply_with_role("TextInput");
    value["semantics"][1]["parts"] = serde_json::json!([{
        "index":7,"parent":null,"frame":[0.,0.,100.,30.],"focused":false,
        "semantics":{
            "role":role_value(role),"name":"Query","value":null,"disabled":false,
            "selected":false,"expanded":null,"invalid":false,"required":false,
            "description":null
        },
        "preserves_activation_focus":false
    }]);
    value
}

#[test]
fn protocol_version_stays_two_and_complete_reply_roundtrips() {
    assert_eq!(VERSION, 2);
    let original: wire::Reply = serde_json::from_value(reply_with_role("TextInput")).unwrap();
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded: wire::Reply = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn old_and_current_decoders_preserve_baseline_and_final_peer_roles() {
    for &role in BASELINE_AND_PEER_ROLES {
        let value = serde_json::to_value(role).unwrap();
        let current: wire::SemanticRole = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("current role {role} missing: {error}"));
        let old: old_content::SemanticRole = serde_json::from_value(value)
            .unwrap_or_else(|error| panic!("old role {role} missing: {error}"));
        assert_eq!(
            serde_json::to_value(current).unwrap(),
            serde_json::to_value(old).unwrap()
        );
    }
}

#[test]
fn complete_old_reply_decoder_rejects_every_new_root_role_after_valid_prefix() {
    for &role in NEW_COLLECTION_ROLES {
        let bytes = serde_json::to_vec(&reply_with_role(role)).unwrap();
        let current: wire::Reply = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("current Reply rejected root {role}: {error}"));
        assert_eq!(
            current.semantics.as_ref().unwrap()[1].semantics.role,
            serde_json::from_value::<wire::SemanticRole>(role_value(role)).unwrap()
        );
        let error = serde_json::from_slice::<old_content::Reply>(&bytes)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("unknown variant `{role}`")),
            "{role}: {error}"
        );
    }
}

#[test]
fn complete_old_reply_decoder_rejects_every_new_virtual_part_role() {
    for &role in NEW_COLLECTION_ROLES {
        let bytes = serde_json::to_vec(&reply_with_part(role)).unwrap();
        let current: wire::Reply = serde_json::from_slice(&bytes)
            .unwrap_or_else(|error| panic!("current Reply rejected part {role}: {error}"));
        assert_eq!(
            current.semantics.as_ref().unwrap()[1].parts[0]
                .semantics
                .role,
            serde_json::from_value::<wire::SemanticRole>(role_value(role)).unwrap()
        );
        let error = serde_json::from_slice::<old_content::Reply>(&bytes)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("unknown variant `{role}`")),
            "{role}: {error}"
        );
    }
}

#[test]
fn tree_depth_is_unsigned_and_table_sort_uses_the_declared_tokens() {
    let mut tree_item = reply_with_role("TreeItem");
    tree_item["semantics"][1]["semantics"]["role"] = serde_json::json!({"TreeItem":{"level":-1}});
    assert!(serde_json::from_value::<wire::Reply>(tree_item).is_err());

    let mut table_header = reply_with_role("TableColumnHeader");
    table_header["semantics"][1]["semantics"]["role"]["TableColumnHeader"]["sort"] =
        serde_json::json!("Sideways");
    assert!(serde_json::from_value::<wire::Reply>(table_header).is_err());
    let valid = reply_with_role("TableColumnHeader");
    assert!(serde_json::from_value::<wire::Reply>(valid).is_ok());
}

#[test]
fn actual26_input_decoder_keeps_existing_set_expanded_and_original_identity() {
    let input = serde_json::json!({
        "kind":"Accessibility",
        "data":{
            "target":{"id":41,"mount":92},"part":null,"binding":[9,42],
            "action":{"SetExpanded":true}
        }
    });
    let bytes = serde_json::to_vec(&input).unwrap();
    let current: wire::Input = serde_json::from_slice(&bytes).unwrap();
    let actual26: old_content::Input = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_value(current).unwrap(), input);
    assert_eq!(serde_json::to_value(actual26).unwrap(), input);
}
