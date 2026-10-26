//! Contract for the exact pure Output builder called by the ProcessView Curve route.
//! It has no Module, ContentHost, Renderer, render queue, or grant-publication input.
use tekes_ui_module_runtime::component::Output;
use tekes_ui_module_runtime::component::curve::{
    CurveErrorCode, Envelope, RequestBodyV1, rejection_output,
};

#[test]
fn process_view_curve_route_can_only_emit_exact_typed_rejection() {
    let request = Envelope {
        protocol_major: 1,
        wire_revision: 1,
        view: 41,
        transaction_id: 43,
        ttl_ms: 0,
        body: RequestBodyV1::TransactionStatus,
    };
    let Output::CurveReplyV1 {
        request: id,
        view,
        reply,
    } = rejection_output(47, &request, Some(41))
    else {
        panic!("Curve refusal builder emitted a non-reply output");
    };
    assert_eq!(id, 47);
    assert_eq!(view, 41);
    assert!(matches!(
        reply,
        tekes_ui_module_runtime::component::curve::CurveReply::Rejected {
            transaction_id: 43,
            code: CurveErrorCode::CapabilityNotNegotiated,
        }
    ));

    let Output::CurveReplyV1 { view, reply, .. } = rejection_output(48, &request, None) else {
        panic!("headless Curve refusal builder emitted a non-reply output");
    };
    assert_eq!(view, 0);
    assert!(matches!(
        reply,
        tekes_ui_module_runtime::component::curve::CurveReply::Rejected {
            transaction_id: 43,
            code: CurveErrorCode::NotReady,
        }
    ));
}
