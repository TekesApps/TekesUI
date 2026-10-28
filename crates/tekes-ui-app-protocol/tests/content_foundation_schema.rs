//! A matched receiver accepts Image; the complete original decoder fails closed.
#[path = "support/pre_image_content.rs"]
mod pre_image_content;
use tekes_ui_app_protocol::component::content::{Reply, SemanticRole};

#[test]
fn image_reply_uses_matched_schema_and_old_decoder_rejects_the_complete_snapshot() {
    let node = |id, role, name| {
        serde_json::json!({
            "target":{"id":id,"mount":id+100},"parent":null,
            "frame":[0,0,20,10],"visible":[0,0,20,10],"focused":false,
            "semantics":{"role":role,"name":name,"value":null,"disabled":false,
                "selected":false,"expanded":null,"invalid":false,"required":false,"description":null},
            "parts":[],"value_range":null,"text":null
        })
    };
    let mut value = serde_json::json!({
        "view":1,"viewport":2,"consumed":false,"changed":false,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,"focus_targets":[],
        "composing":false,"text":null,
        "semantics":[node(1,"Text","valid prefix"),node(2,"Image","Informative image")]
    });
    let reply: Reply = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        reply.semantics.as_ref().unwrap()[1].semantics.role,
        SemanticRole::Image
    );
    let encoded = serde_json::to_vec(&reply).unwrap();
    assert_eq!(serde_json::from_slice::<Reply>(&encoded).unwrap(), reply);
    assert!(serde_json::from_slice::<pre_image_content::Reply>(&encoded).is_err());
    value["semantics"][1]["semantics"]["role"] = "UnknownFutureRole".into();
    assert!(serde_json::from_value::<Reply>(value.clone()).is_err());
    value["semantics"][1]["semantics"]["role"] = "Text".into();
    assert!(serde_json::from_value::<pre_image_content::Reply>(value).is_ok());
}
