//! Whole-reply compatibility contract for the new Splitter wire role.
#[path = "support/pre_splitter_content.rs"]
mod old_content;
use tekes_ui_app_protocol::component::{VERSION, content as wire};

fn node(role: &str, id: u64) -> serde_json::Value {
    serde_json::json!({
        "target":{"id":id,"mount":id+100},"parent":null,
        "frame":[0.,0.,100.,30.],"visible":[0.,0.,100.,30.],"focused":false,
        "semantics":{"role":role,"name":"Split view divider","value":"0.5",
        "disabled":false,"selected":false,"expanded":null,"invalid":false,
        "required":false,"description":null},"parts":[],
        "value_range":if role == "Splitter" { serde_json::json!([0.1,0.9]) } else { serde_json::Value::Null },
        "text_binding":null,"text":null,"text_layout":null
    })
}
#[test]
fn old_complete_reply_decoder_rejects_splitter_after_valid_prefix() {
    assert_eq!(VERSION, 2);
    let reply = serde_json::json!({
        "view":1,"viewport":1,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,
        "focus_targets":[],"composing":false,"text":null,
        "semantics":[node("Text",1),node("Splitter",2)]
    });
    let bytes = serde_json::to_vec(&reply).unwrap();
    let decoded: wire::Reply = serde_json::from_slice(&bytes).unwrap();
    let semantics = decoded.semantics.as_ref().unwrap();
    assert_eq!(semantics.len(), 2);
    assert_eq!(semantics[1].semantics.role, wire::SemanticRole::Splitter);
    let encoded = serde_json::to_vec(&decoded).unwrap();
    assert_eq!(
        serde_json::to_string(&wire::SemanticRole::Splitter).unwrap(),
        "\"Splitter\""
    );
    let error = serde_json::from_slice::<old_content::Reply>(&encoded)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant `Splitter`"), "{error}");
}

#[test]
fn complete_pre_splitter_decoder_still_accepts_prior_image_role() {
    let mut image = node("Image", 2);
    image["semantics"]["name"] = "Informative image".into();
    image["semantics"]["value"] = serde_json::Value::Null;
    let reply = serde_json::json!({
        "view":1,"viewport":1,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,
        "focus_targets":[],"composing":false,"text":null,
        "semantics":[node("Text",1), image]
    });
    let current: wire::Reply = serde_json::from_value(reply).unwrap();
    let encoded = serde_json::to_vec(&current).unwrap();
    let old: old_content::Reply = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        old.semantics.unwrap()[1].semantics.role,
        old_content::SemanticRole::Image
    );
}
