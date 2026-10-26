//! ProcessComponent service messages. No View/Panel/GPU identity is required.
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub mod curve;
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
    pub keyboard: bool,
    pub scroll: bool,
    pub text_ime: bool,
    pub accessibility: bool,
    pub independent_toolbar: bool,
}
impl ViewCapabilities {
    pub const V1: Self = Self {
        primary_pointer: true,
        keyboard: false,
        scroll: false,
        text_ime: false,
        accessibility: false,
        independent_toolbar: false,
    };
    pub fn supports(self, required: Self) -> bool {
        (!required.primary_pointer || self.primary_pointer)
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
