//! Curve V1 protocol wire codec and reject-only first integration slice.
//! Full grant publication, input/query forwarding, final clipping and reveal are not implemented here.
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, IgnoredAny, SeqAccess, Visitor},
};
use std::fmt;

pub const HARD_PACKET_EXCLUSIVE: usize = 1_048_576;
pub const HARD_RECORDS: usize = 4096;
pub const HARD_DEPTH: u16 = 256;
pub const HARD_STEPS: u32 = 12_288;
pub const HARD_TARGET_AXIS: u32 = 4096;
pub const HARD_TTL_MS: u16 = 1000;
pub const REQUIRED_FEATURES: u64 = 0x01ff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub packet_bytes_exclusive: usize,
    pub records: usize,
    pub depth: u16,
    pub validation_steps: u32,
    pub target_axis_px: u32,
    pub ttl_ms: u16,
}
impl Limits {
    pub fn validate(self) -> Result<Self, WireError> {
        if self.packet_bytes_exclusive == 0
            || self.packet_bytes_exclusive > HARD_PACKET_EXCLUSIVE
            || self.records == 0
            || self.records > HARD_RECORDS
            || self.depth == 0
            || self.depth > HARD_DEPTH
            || self.validation_steps == 0
            || self.validation_steps > HARD_STEPS
            || self.target_axis_px == 0
            || self.target_axis_px > HARD_TARGET_AXIS
            || self.ttl_ms == 0
            || self.ttl_ms > HARD_TTL_MS
        {
            return Err(WireError::LimitRange);
        }
        Ok(self)
    }
    pub fn validate_ack(self, offered: Self) -> Result<Self, WireError> {
        let value = self.validate()?;
        let offer = offered.validate()?;
        if value.packet_bytes_exclusive > offer.packet_bytes_exclusive
            || value.records > offer.records
            || value.depth > offer.depth
            || value.validation_steps > offer.validation_steps
            || value.target_axis_px > offer.target_axis_px
            || value.ttl_ms > offer.ttl_ms
        {
            return Err(WireError::AckRaisedLimit);
        }
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum CurveErrorCode {
    UnsupportedVersion,
    UnsupportedFeature,
    MalformedMessage,
    LimitExceeded,
    CapabilityNotNegotiated,
    NotReady,
    StaleEpoch,
    ClassAProofUnavailable,
    DeadlineExpired,
    DuplicateFinalize,
    UnknownTransaction,
    OwnerMismatch,
    StaleFrame,
    RevealUnsupported,
    InternalFailure,
}
impl From<CurveErrorCode> for u16 {
    fn from(value: CurveErrorCode) -> Self {
        match value {
            CurveErrorCode::UnsupportedVersion => 1,
            CurveErrorCode::UnsupportedFeature => 2,
            CurveErrorCode::MalformedMessage => 3,
            CurveErrorCode::LimitExceeded => 4,
            CurveErrorCode::CapabilityNotNegotiated => 5,
            CurveErrorCode::NotReady => 6,
            CurveErrorCode::StaleEpoch => 7,
            CurveErrorCode::ClassAProofUnavailable => 8,
            CurveErrorCode::DeadlineExpired => 9,
            CurveErrorCode::DuplicateFinalize => 10,
            CurveErrorCode::UnknownTransaction => 11,
            CurveErrorCode::OwnerMismatch => 12,
            CurveErrorCode::StaleFrame => 13,
            CurveErrorCode::RevealUnsupported => 14,
            CurveErrorCode::InternalFailure => 15,
        }
    }
}
impl TryFrom<u16> for CurveErrorCode {
    type Error = WireError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::UnsupportedVersion,
            2 => Self::UnsupportedFeature,
            3 => Self::MalformedMessage,
            4 => Self::LimitExceeded,
            5 => Self::CapabilityNotNegotiated,
            6 => Self::NotReady,
            7 => Self::StaleEpoch,
            8 => Self::ClassAProofUnavailable,
            9 => Self::DeadlineExpired,
            10 => Self::DuplicateFinalize,
            11 => Self::UnknownTransaction,
            12 => Self::OwnerMismatch,
            13 => Self::StaleFrame,
            14 => Self::RevealUnsupported,
            15 => Self::InternalFailure,
            _ => return Err(WireError::UnknownErrorCode(value)),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    LimitRange,
    AckRaisedLimit,
    UnknownErrorCode(u16),
}
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for WireError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeRecord {
    pub index: u16,
    pub parent_index: Option<u16>,
    pub depth: u16,
    pub rect_px: [f32; 4],
    pub radii_px: [f32; 4],
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoundedRecords(pub Vec<ShapeRecord>);
impl Serialize for BoundedRecords {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error, SerializeSeq};
        if self.0.len() > HARD_RECORDS {
            return Err(S::Error::custom("record hard cap exceeded"));
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for item in &self.0 {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}
impl<'de> Deserialize<'de> for BoundedRecords {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RecordsVisitor;
        impl<'de> Visitor<'de> for RecordsVisitor {
            type Value = BoundedRecords;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "at most {HARD_RECORDS} fixed shape records")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut records =
                    Vec::with_capacity(seq.size_hint().unwrap_or(0).min(HARD_RECORDS));
                loop {
                    if records.len() == HARD_RECORDS {
                        if seq.next_element::<IgnoredAny>()?.is_some() {
                            return Err(de::Error::custom(
                                "record hard cap exceeded before allocation",
                            ));
                        }
                        break;
                    }
                    match seq.next_element::<ShapeRecord>()? {
                        Some(record) => records.push(record),
                        None => break,
                    }
                }
                Ok(BoundedRecords(records))
            }
        }
        deserializer.deserialize_seq(RecordsVisitor)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantProposal {
    pub proposal_id: u64,
    pub viewport: u64,
    pub allocation_seq: u64,
    pub layout_seq: u64,
    pub parent_geometry_generation: u64,
    pub target_px: [u32; 2],
    pub physical_origin_px: [f32; 2],
    pub backing_scale: f32,
    pub clear_rgba: [f32; 4],
    pub sample_count: u8,
    pub coordinate_mapping: String,
    pub chain_head: u16,
    pub chain_depth: u16,
    pub records: BoundedRecords,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol_major: u16,
    pub wire_revision: u16,
    pub view: u64,
    pub transaction_id: u64,
    pub ttl_ms: u16,
    pub body: RequestBodyV1,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "payload", deny_unknown_fields)]
pub enum RequestBodyV1 {
    Negotiate {
        offer: CapabilityWire,
    },
    PrepareGrant {
        proposal: GrantProposal,
    },
    ContentQuery {
        query: ReadOnlyQueryWire,
    },
    TransactionStatus,
    RevokeAdmission {
        grant_serial: u64,
        reason: RevokeReasonWire,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsWire {
    pub packet_bytes_exclusive: u32,
    pub records: u32,
    pub depth: u16,
    pub validation_steps: u32,
    pub target_axis_px: u32,
    pub ttl_ms: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityWire {
    pub protocol_major: u16,
    pub wire_revision: u16,
    pub clip_geometry_revision: u32,
    pub coverage_aa_revision: u16,
    pub features: u64,
    pub limits: LimitsWire,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RevokeReasonWire {
    ParentGeometryChanged,
    ChildGeometryChanged,
    ViewportChanged,
    ContextGeometryChanged,
    ProtocolFailure,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum CurveReply {
    Negotiated {
        capability: CapabilityWire,
    },
    Rejected {
        transaction_id: u64,
        code: CurveErrorCode,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum ReadOnlyQueryWire {
    SemanticTree {
        stamp: QueryStamp,
    },
    ReadingState {
        stamp: QueryStamp,
        reader_id: u64,
        reader_mount: u64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryStamp {
    pub grant_serial: u64,
    pub gp: u64,
    pub gc: u64,
    pub view: u64,
    pub viewport: u64,
    pub allocation_seq: u64,
    pub layout_seq: u64,
    pub published_frame_seq: u64,
}

/// First production routing slice is reject-only. It validates protocol and owner
/// identity before choosing a refusal; no request can produce Negotiated here.
fn wire_limits_valid(value: &LimitsWire) -> bool {
    Limits {
        packet_bytes_exclusive: value.packet_bytes_exclusive as usize,
        records: value.records as usize,
        depth: value.depth,
        validation_steps: value.validation_steps,
        target_axis_px: value.target_axis_px,
        ttl_ms: value.ttl_ms,
    }
    .validate()
    .is_ok()
}
fn proposal_within_hard_limits(proposal: &GrantProposal) -> bool {
    let records = proposal.records.0.len();
    records > 0
        && records <= HARD_RECORDS
        && proposal.chain_depth > 0
        && proposal.chain_depth <= HARD_DEPTH
        && proposal
            .records
            .0
            .iter()
            .all(|record| record.depth > 0 && record.depth <= HARD_DEPTH)
        && proposal
            .target_px
            .iter()
            .all(|axis| *axis > 0 && *axis <= HARD_TARGET_AXIS)
        && (records as u32).saturating_mul(3) <= HARD_STEPS
}
fn operation_contract(envelope: &Envelope) -> Result<(), CurveErrorCode> {
    match &envelope.body {
        RequestBodyV1::Negotiate { .. }
        | RequestBodyV1::ContentQuery { .. }
        | RequestBodyV1::RevokeAdmission { .. } => {
            if envelope.transaction_id != 0 || envelope.ttl_ms != 0 {
                return Err(CurveErrorCode::MalformedMessage);
            }
        }
        RequestBodyV1::TransactionStatus => {
            if envelope.transaction_id == 0 || envelope.ttl_ms != 0 {
                return Err(CurveErrorCode::MalformedMessage);
            }
        }
        RequestBodyV1::PrepareGrant { .. } => {
            if envelope.transaction_id == 0 || envelope.ttl_ms == 0 {
                return Err(CurveErrorCode::MalformedMessage);
            }
            if envelope.ttl_ms > HARD_TTL_MS {
                return Err(CurveErrorCode::LimitExceeded);
            }
        }
    }
    Ok(())
}
pub fn reject_for_attached_view(envelope: &Envelope, attached_view: Option<u64>) -> CurveErrorCode {
    if (envelope.protocol_major, envelope.wire_revision) != (1, 1) {
        return CurveErrorCode::UnsupportedVersion;
    }
    if envelope.view == 0 {
        return CurveErrorCode::MalformedMessage;
    }
    if let Err(code) = operation_contract(envelope) {
        return code;
    }
    let Some(attached_view) = attached_view else {
        return CurveErrorCode::NotReady;
    };
    if attached_view != envelope.view {
        return CurveErrorCode::OwnerMismatch;
    }
    match &envelope.body {
        RequestBodyV1::Negotiate { offer } => {
            if (
                offer.protocol_major,
                offer.wire_revision,
                offer.clip_geometry_revision,
                offer.coverage_aa_revision,
            ) != (1, 1, 1, 1)
            {
                return CurveErrorCode::UnsupportedVersion;
            }
            if !wire_limits_valid(&offer.limits) {
                return CurveErrorCode::LimitExceeded;
            }
            if offer.features != REQUIRED_FEATURES {
                return CurveErrorCode::UnsupportedFeature;
            }
            CurveErrorCode::UnsupportedFeature
        }
        RequestBodyV1::PrepareGrant { proposal } => {
            if !proposal_within_hard_limits(proposal) {
                return CurveErrorCode::LimitExceeded;
            }
            CurveErrorCode::CapabilityNotNegotiated
        }
        RequestBodyV1::ContentQuery { .. }
        | RequestBodyV1::TransactionStatus
        | RequestBodyV1::RevokeAdmission { .. } => CurveErrorCode::CapabilityNotNegotiated,
    }
}

pub fn rejection_output(
    request: u64,
    envelope: &Envelope,
    attached_view: Option<u64>,
) -> crate::component::Output {
    let view = attached_view.unwrap_or(0);
    let code = reject_for_attached_view(envelope, attached_view);
    crate::component::Output::CurveReplyV1 {
        request,
        view,
        reply: CurveReply::Rejected {
            transaction_id: envelope.transaction_id,
            code,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{ClientMessage, RequestBody};

    fn envelope(tx: u64, ttl: u16, body: RequestBodyV1) -> Envelope {
        Envelope {
            protocol_major: 1,
            wire_revision: 1,
            view: 23,
            transaction_id: tx,
            ttl_ms: ttl,
            body,
        }
    }
    fn offer() -> CapabilityWire {
        CapabilityWire {
            protocol_major: 1,
            wire_revision: 1,
            clip_geometry_revision: 1,
            coverage_aa_revision: 1,
            features: REQUIRED_FEATURES,
            limits: LimitsWire {
                packet_bytes_exclusive: 65536,
                records: 512,
                depth: 64,
                validation_steps: 1000,
                target_axis_px: 1024,
                ttl_ms: 100,
            },
        }
    }
    fn proposal(records: Vec<ShapeRecord>) -> GrantProposal {
        GrantProposal {
            proposal_id: 1,
            viewport: 2,
            allocation_seq: 3,
            layout_seq: 4,
            parent_geometry_generation: 5,
            target_px: [64, 48],
            physical_origin_px: [0., 0.],
            backing_scale: 1.,
            clear_rgba: [0.; 4],
            sample_count: 1,
            coordinate_mapping: "SamePhysicalCoordinates".into(),
            chain_head: 0,
            chain_depth: 1,
            records: BoundedRecords(records),
        }
    }
    fn root_record() -> ShapeRecord {
        ShapeRecord {
            index: 0,
            parent_index: None,
            depth: 1,
            rect_px: [0., 0., 64., 48.],
            radii_px: [0.; 4],
        }
    }
    #[test]
    fn operation_identity_and_ttl_contracts_are_distinct() {
        let attached = Some(23);
        assert_eq!(
            reject_for_attached_view(
                &envelope(0, 0, RequestBodyV1::Negotiate { offer: offer() }),
                attached
            ),
            CurveErrorCode::UnsupportedFeature
        );
        assert_eq!(
            reject_for_attached_view(
                &envelope(1, 0, RequestBodyV1::Negotiate { offer: offer() }),
                attached
            ),
            CurveErrorCode::MalformedMessage
        );
        assert_eq!(
            reject_for_attached_view(
                &envelope(0, 1, RequestBodyV1::Negotiate { offer: offer() }),
                attached
            ),
            CurveErrorCode::MalformedMessage
        );

        let stamp = QueryStamp {
            grant_serial: 1,
            gp: 2,
            gc: 3,
            view: 23,
            viewport: 4,
            allocation_seq: 5,
            layout_seq: 6,
            published_frame_seq: 7,
        };
        let query = || RequestBodyV1::ContentQuery {
            query: ReadOnlyQueryWire::SemanticTree {
                stamp: stamp.clone(),
            },
        };
        assert_eq!(
            reject_for_attached_view(&envelope(0, 0, query()), attached),
            CurveErrorCode::CapabilityNotNegotiated
        );
        assert_eq!(
            reject_for_attached_view(&envelope(1, 0, query()), attached),
            CurveErrorCode::MalformedMessage
        );
        assert_eq!(
            reject_for_attached_view(&envelope(0, 1, query()), attached),
            CurveErrorCode::MalformedMessage
        );

        assert_eq!(
            reject_for_attached_view(
                &envelope(
                    0,
                    0,
                    RequestBodyV1::RevokeAdmission {
                        grant_serial: 3,
                        reason: RevokeReasonWire::ProtocolFailure
                    }
                ),
                attached
            ),
            CurveErrorCode::CapabilityNotNegotiated
        );
        assert_eq!(
            reject_for_attached_view(
                &envelope(
                    2,
                    0,
                    RequestBodyV1::RevokeAdmission {
                        grant_serial: 3,
                        reason: RevokeReasonWire::ProtocolFailure
                    }
                ),
                attached
            ),
            CurveErrorCode::MalformedMessage
        );

        assert_eq!(
            reject_for_attached_view(&envelope(29, 0, RequestBodyV1::TransactionStatus), attached),
            CurveErrorCode::CapabilityNotNegotiated
        );
        assert_eq!(
            reject_for_attached_view(&envelope(0, 0, RequestBodyV1::TransactionStatus), attached),
            CurveErrorCode::MalformedMessage
        );
        assert_eq!(
            reject_for_attached_view(&envelope(29, 1, RequestBodyV1::TransactionStatus), attached),
            CurveErrorCode::MalformedMessage
        );

        let valid_grant = RequestBodyV1::PrepareGrant {
            proposal: proposal(vec![root_record()]),
        };
        assert_eq!(
            reject_for_attached_view(&envelope(29, 50, valid_grant.clone()), attached),
            CurveErrorCode::CapabilityNotNegotiated
        );
        assert_eq!(
            reject_for_attached_view(&envelope(0, 50, valid_grant.clone()), attached),
            CurveErrorCode::MalformedMessage
        );
        assert_eq!(
            reject_for_attached_view(&envelope(29, 0, valid_grant.clone()), attached),
            CurveErrorCode::MalformedMessage
        );
        assert_eq!(
            reject_for_attached_view(&envelope(29, HARD_TTL_MS + 1, valid_grant), attached),
            CurveErrorCode::LimitExceeded
        );

        let mut oversized = proposal(vec![root_record()]);
        oversized.chain_depth = HARD_DEPTH + 1;
        assert_eq!(
            reject_for_attached_view(
                &envelope(
                    29,
                    1,
                    RequestBodyV1::PrepareGrant {
                        proposal: oversized
                    }
                ),
                attached
            ),
            CurveErrorCode::LimitExceeded
        );
    }
    #[test]
    fn owner_and_protocol_failures_are_separate_from_op_contract_errors() {
        let valid = envelope(29, 0, RequestBodyV1::TransactionStatus);
        assert_eq!(
            reject_for_attached_view(&valid, Some(24)),
            CurveErrorCode::OwnerMismatch
        );
        assert_eq!(
            reject_for_attached_view(&valid, None),
            CurveErrorCode::NotReady
        );
        let unsupported = Envelope {
            wire_revision: 2,
            ..valid
        };
        assert_eq!(
            reject_for_attached_view(&unsupported, Some(23)),
            CurveErrorCode::UnsupportedVersion
        );
    }
    #[test]
    fn valid_full_offer_is_rejected_without_partial_or_full_ack() {
        assert_eq!(
            reject_for_attached_view(
                &envelope(0, 0, RequestBodyV1::Negotiate { offer: offer() }),
                Some(23)
            ),
            CurveErrorCode::UnsupportedFeature
        );
        assert_eq!(
            reject_for_attached_view(
                &envelope(
                    0,
                    0,
                    RequestBodyV1::Negotiate {
                        offer: CapabilityWire {
                            limits: LimitsWire {
                                records: (HARD_RECORDS as u32) + 1,
                                ..offer().limits
                            },
                            ..offer()
                        }
                    }
                ),
                Some(23)
            ),
            CurveErrorCode::LimitExceeded
        );
    }
    #[test]
    fn bounded_record_wire_deserializer_caps_count_and_rejects_unknown_codes() {
        let one = serde_json::json!({"index":0,"parent_index":null,"depth":1,"rect_px":[0.0,0.0,1.0,1.0],"radii_px":[0.0,0.0,0.0,0.0]});
        let within = serde_json::Value::Array((0..HARD_RECORDS).map(|_| one.clone()).collect());
        let over = serde_json::Value::Array((0..=HARD_RECORDS).map(|_| one.clone()).collect());
        assert_eq!(
            serde_json::from_value::<BoundedRecords>(within)
                .unwrap()
                .0
                .len(),
            HARD_RECORDS
        );
        assert!(
            serde_json::from_value::<BoundedRecords>(over)
                .unwrap_err()
                .to_string()
                .contains("record hard cap")
        );
        assert!(serde_json::from_value::<CurveErrorCode>(serde_json::json!(16)).is_err());
    }
    fn client_wire_with_record_count(count: usize) -> Vec<u8> {
        let record = serde_json::json!({"index":0,"parent_index":null,"depth":1,"rect_px":[0.0,0.0,64.0,48.0],"radii_px":[0.0,0.0,0.0,0.0]});
        let records = serde_json::Value::Array((0..count).map(|_| record.clone()).collect());
        serde_json::to_vec(&serde_json::json!({
            "kind":"Request","data":{"version":2,"session":{"module":"fixture","instance":1,"host_epoch":1},"id":1,
            "body":{"kind":"CurveV1","data":{"protocol_major":1,"wire_revision":1,"view":23,"transaction_id":29,"ttl_ms":1,
                "body":{"op":"PrepareGrant","payload":{"proposal":{"proposal_id":1,"viewport":2,"allocation_seq":3,"layout_seq":4,"parent_geometry_generation":5,
                    "target_px":[64,48],"physical_origin_px":[0.0,0.0],"backing_scale":1.0,"clear_rgba":[0.0,0.0,0.0,0.0],"sample_count":1,
                    "coordinate_mapping":"SamePhysicalCoordinates","chain_head":0,"chain_depth":1,"records":records}}}}}}
        })).unwrap()
    }
    #[test]
    fn actual_json_bytes_accept_4096_records_and_refuse_4097() {
        let within = client_wire_with_record_count(HARD_RECORDS);
        assert!(
            within.len() < HARD_PACKET_EXCLUSIVE,
            "fixture must remain within the independent packet cap"
        );
        let decoded: ClientMessage = serde_json::from_slice(&within).unwrap();
        assert!(
            matches!(decoded, ClientMessage::Request { body: RequestBody::CurveV1(Envelope { body: RequestBodyV1::PrepareGrant { proposal: GrantProposal { records: BoundedRecords(ref records), .. } }, .. }), .. } if records.len() == HARD_RECORDS)
        );
        let over = client_wire_with_record_count(HARD_RECORDS + 1);
        assert!(
            over.len() < HARD_PACKET_EXCLUSIVE,
            "record-count case must remain below the independent packet cap"
        );
        assert!(
            serde_json::from_slice::<ClientMessage>(&over)
                .unwrap_err()
                .to_string()
                .contains("record hard cap")
        );
    }
    #[test]
    fn actual_request_bytes_accept_out_of_order_curve_tag() {
        let wire = br#"{"kind":"Request","data":{"version":2,"session":{"module":"fixture","instance":1,"host_epoch":1},"id":2,"body":{"data":{"protocol_major":1,"wire_revision":1,"view":23,"transaction_id":29,"ttl_ms":0,"body":{"op":"TransactionStatus"}},"kind":"CurveV1"}}}"#;
        let decoded: ClientMessage = serde_json::from_slice(wire).unwrap();
        assert!(matches!(
            decoded,
            ClientMessage::Request {
                id: 2,
                body: RequestBody::CurveV1(Envelope {
                    body: RequestBodyV1::TransactionStatus,
                    ..
                }),
                ..
            }
        ));
    }
}
