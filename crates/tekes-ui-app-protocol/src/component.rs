//! ProcessComponent service messages. No View/Panel/GPU identity is required.
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub mod content;
pub mod curve;
mod view_context;
pub use view_context::ViewContext;
pub const VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub services: bool,
    pub toolbar: bool,
    pub view: bool,
}
impl Capabilities {
    pub const HEADLESS: Self = Self {
        services: true,
        toolbar: true,
        view: false,
    };
    pub fn supports(self, required: Self) -> bool {
        (!required.services || self.services)
            && (!required.toolbar || self.toolbar)
            && (!required.view || self.view)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub module: String,
    pub instance: u64,
    pub host_epoch: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum RequestBody {
    /// Curve V1 wire envelope. The current receiver is reject-only; it never grants capabilities.
    CurveV1(curve::Envelope),
    /// Original Content callbacks. Requires a negotiated keyboard/text View;
    /// legacy/headless workers must reject it rather than dispatch as Message.
    Content(content::Request),
    /// Window-owned environment for the original View/font backend. The actor
    /// invalidates applied geometry until its next original Layout.
    SetViewContext {
        view: u64,
        context: ViewContext,
    },
    Message {
        topic: String,
        payload: Value,
    },
    Invoke {
        command: u64,
        registration: u64,
        window: u64,
    },
    /// V2 session/correlation with the explicitly negotiated Panel render profile.
    Render {
        view: u64,
        input: crate::Request,
    },
    Stop,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ClientMessage {
    StartView {
        version: u32,
        session: Session,
        required: Capabilities,
        attach: ViewAttach,
    },
    Start {
        version: u32,
        session: Session,
        required: Capabilities,
    },
    Request {
        version: u32,
        session: Session,
        id: u64,
        body: RequestBody,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolbarScope {
    Window { window: u64 },
    View,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolbarAction {
    pub command: crate::ToolbarItem,
    pub icon: Option<String>,
    pub accessible_name: Option<String>,
    pub tooltip: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ComponentEvent {
    ViewInvalidated {
        view: u64,
        revision: u64,
    },
    Toolbar {
        revision: u64,
        scope: ToolbarScope,
        items: Vec<ToolbarAction>,
    },
    Message {
        topic: String,
        payload: Value,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Output {
    /// Typed fail-closed reply, correlated to the original service request and attached View.
    CurveReplyV1 {
        request: u64,
        view: u64,
        reply: curve::CurveReply,
    },
    ReadyView {
        pid: u32,
        capabilities: Capabilities,
        window: u64,
        view: u64,
        native: ViewCapabilities,
    },
    Frame {
        request: u64,
        view: u64,
        frame: crate::Frame,
    },
    Ready {
        pid: u32,
        capabilities: Capabilities,
    },
    Event(ComponentEvent),
    Response {
        request: u64,
        result: Result<Value, String>,
    },
    Fatal {
        message: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerMessage {
    pub version: u32,
    pub session: Session,
    pub serial: u64,
    pub output: Output,
}

/// Native View features, separate from the existence of a rendered surface.
/// V1 is an explicit compatibility profile, not a V2 handshake result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCapabilities {
    pub primary_pointer: bool,
    /// Original Content routing with per-event disposition and capture identity.
    /// Distinct from the legacy opaque primary-pointer render stream.
    #[serde(default, skip_serializing_if = "is_false")]
    pub content_pointer: bool,
    pub keyboard: bool,
    pub scroll: bool,
    pub text_ime: bool,
    pub accessibility: bool,
    pub independent_toolbar: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
impl ViewCapabilities {
    pub const V1: Self = Self {
        primary_pointer: true,
        content_pointer: false,
        keyboard: false,
        scroll: false,
        text_ime: false,
        accessibility: false,
        independent_toolbar: false,
    };
    pub fn supports(self, required: Self) -> bool {
        (!required.primary_pointer || self.primary_pointer)
            && (!required.content_pointer || self.content_pointer)
            && (!required.keyboard || self.keyboard)
            && (!required.scroll || self.scroll)
            && (!required.text_ime || self.text_ime)
            && (!required.accessibility || self.accessibility)
            && (!required.independent_toolbar || self.independent_toolbar)
    }
}

/// Real host View identity and receive endpoint. Absent for a headless Start.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewAttach {
    pub window: u64,
    pub view: u64,
    pub endpoint: String,
    pub required: ViewCapabilities,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_pointer_disposition_is_not_the_legacy_opaque_stream() {
        let required = ViewCapabilities {
            content_pointer: true,
            ..Default::default()
        };
        assert!(!ViewCapabilities::V1.supports(required));
        assert!(
            !ViewCapabilities {
                keyboard: true,
                text_ime: true,
                ..Default::default()
            }
            .supports(required)
        );
        assert!(required.supports(required));
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        struct V1Capabilities {
            primary_pointer: bool,
            keyboard: bool,
            scroll: bool,
            text_ime: bool,
            accessibility: bool,
            independent_toolbar: bool,
        }
        let old = serde_json::json!({"primary_pointer":true,"keyboard":false,"scroll":false,"text_ime":false,"accessibility":false,"independent_toolbar":false});
        let decoded: ViewCapabilities = serde_json::from_value(old.clone()).unwrap();
        assert!(!decoded.content_pointer);
        assert_eq!(serde_json::to_value(decoded).unwrap(), old);
        let legacy: V1Capabilities =
            serde_json::from_value(serde_json::to_value(decoded).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(legacy).unwrap(), old);
        assert!(
            serde_json::from_value::<V1Capabilities>(serde_json::to_value(required).unwrap())
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(required).unwrap()["content_pointer"],
            true
        );
    }
    #[test]
    fn legacy_start_wire_and_exhaustive_client_patterns_remain_unchanged() {
        fn version(message: ClientMessage) -> u32 {
            match message {
                ClientMessage::Start { version, .. } => version,
                ClientMessage::StartView { version, .. } => version,
                ClientMessage::Request { version, .. } => version,
            }
        }
        let session = Session {
            module: "counter".into(),
            instance: 1,
            host_epoch: 42,
        };
        let legacy = ClientMessage::Start {
            version: VERSION,
            session,
            required: Capabilities::HEADLESS,
        };
        let value = serde_json::to_value(&legacy).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"kind":"Start","data":{"version":2,"session":{"module":"counter","instance":1,"host_epoch":42},"required":{"services":true,"toolbar":true,"view":false}}})
        );
        assert_eq!(version(legacy), VERSION);
    }
}
