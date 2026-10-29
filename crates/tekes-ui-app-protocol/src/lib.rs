//! Data-only process contract. No Rust pointers, closures, UI or GPU objects.
//! Experimental: the full geometry/capture/IME design is not implemented yet.
use serde::{Deserialize, Serialize};
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputKind {
    Layout,
    PointerDown,
    PointerUp,
    Command,
    PointerMove,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    pub seq: u64,
    pub viewport: u64,
    pub kind: InputKind,
    pub command: Option<u64>,
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub identity: u32,
    pub scale: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolbarItem {
    pub id: u64,
    pub title: String,
    pub enabled: bool,
    pub geometry_dependent: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub version: u32,
    pub seq: u64,
    pub viewport: u64,
    pub pid: u32,
    pub registry: u64,
    pub width: u32,
    pub height: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Receiver-local Mach right, carried out of band, never serialized as an ID.
    #[serde(skip)]
    pub port: u32,
    pub revision: u64,
    pub toolbar: Vec<ToolbarItem>,
}

/// Versioned service protocol; V1 View messages above remain unchanged.
pub mod component;
