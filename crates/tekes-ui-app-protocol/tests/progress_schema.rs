//! Matched-schema progress role: known range versus explicit unknown.
#[path = "support/pre_splitter_content.rs"]
mod old_content;
use tekes_ui_app_protocol::component::content as wire;

#[test]
fn progress_whole_reply_roundtrip_and_old_receiver_rejection() {
    for (value, range) in [
        ("45%", serde_json::json!([0., 100.])),
        ("进度未知", serde_json::Value::Null),
    ] {
        let expected_range = if range.is_null() {
            None
        } else {
            Some([0., 100.])
        };
        let reply = serde_json::json!({
            "view":1,"viewport":1,"consumed":false,"changed":false,
            "capture_pointer":false,"activated":false,"bubbles":true,
            "pointer_focus_requested":false,"key":null,"focused":null,
            "focus_epoch":0,"text_binding":null,"captured":null,
            "focus_targets":[],"composing":false,"text":null,
            "semantics":[{
                "target":{"id":1,"mount":101},"parent":null,
                "frame":[0.,0.,100.,12.],"visible":[0.,0.,100.,12.],"focused":false,
                "semantics":{"role":"ProgressIndicator","name":"Transfer progress","value":value,
                    "disabled":false,"selected":false,"expanded":null,"invalid":false,
                    "required":false,"description":null},"parts":[],"value_range":range,
                "text_binding":null,"text":null,"text_layout":null
            }]
        });
        let decoded: wire::Reply = serde_json::from_value(reply).unwrap();
        let node = &decoded.semantics.as_ref().unwrap()[0];
        assert_eq!(node.semantics.role, wire::SemanticRole::ProgressIndicator);
        assert_eq!(node.semantics.value.as_deref(), Some(value));
        assert_eq!(node.value_range, expected_range);
        let encoded = serde_json::to_vec(&decoded).unwrap();
        let roundtrip: wire::Reply = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            roundtrip.semantics.unwrap()[0].value_range,
            node.value_range
        );
        let error = serde_json::from_slice::<old_content::Reply>(&encoded)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("unknown variant `ProgressIndicator`"),
            "{error}"
        );
    }
}
