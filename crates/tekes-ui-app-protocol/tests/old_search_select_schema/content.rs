//! Data-only original Content input/query profile. Session and request identity
//! come from the outer component envelope; geometry is acknowledged separately.
use serde::{Deserialize, Serialize};
use std::ops::Range;
#[path = "accessibility.rs"]
mod accessibility;
pub use accessibility::*;

/// A query cannot observe geometry until the original layout is acknowledged.
pub const GEOMETRY_PENDING: &str = "stale/unapplied Content geometry";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub id: u64,
    pub mount: u64,
}
/// One acquisition of original capture; a mount alone does not distinguish a
/// cancelled gesture from a subsequent press on that same owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub target: Target,
    pub serial: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
    pub caps_lock: bool,
    pub function: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    Enter,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    Tab { reverse: bool },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyEvent {
    pub key: Option<Key>,
    pub down: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
    pub native_code: u16,
    pub characters: String,
    pub characters_ignoring_modifiers: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerPhase {
    Down,
    Up,
    Move,
    Drag,
    Enter,
    Exit,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointerEvent {
    /// Original child logical coordinates. Capture may continue outside its clip.
    pub position: [f32; 2],
    pub phase: PointerPhase,
    pub button: Option<PointerButton>,
    pub modifiers: Modifiers,
    pub click_count: u32,
}
/// Original document-space wheel input. Position is in acknowledged child
/// logical coordinates; scale is not multiplied into position or delta.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollEvent {
    pub position: [f32; 2],
    pub delta: [f32; 2],
    pub precise: bool,
    pub begin: bool,
    pub end: bool,
    pub momentum: bool,
    pub time: f64,
}
impl ScrollEvent {
    pub fn is_valid(&self) -> bool {
        self.position
            .iter()
            .chain(self.delta.iter())
            .all(|v| v.is_finite())
            && self.time.is_finite()
            && self.time >= 0.
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum Input {
    Snapshot,
    /// Read only the original active semantic tree; never requests a repaint.
    Semantics,
    Accessibility {
        target: Target,
        part: Option<usize>,
        /// Target editor/document provenance, independent of the focused owner.
        #[serde(default)]
        binding: Option<Vec<u8>>,
        action: AccessibilityAction,
    },
    /// Read current original reading identity without changing focus or paint.
    ReadingState {
        reader: u64,
    },
    /// Execute the existing child ContentHost item reveal transaction.
    RevealReadingItem {
        reader: Target,
        document: ReadingVersion,
        item: Target,
        options: RevealOptions,
    },
    Pointer(PointerEvent),
    Scroll(ScrollEvent),
    Focus {
        target: Option<Target>,
        visible: bool,
    },
    /// Enter the original child focus scope using its own traversal policy.
    EnterFocus {
        reverse: bool,
    },
    Key(KeyEvent),
    TextKeyStart {
        id: u64,
        key: KeyEvent,
        composing: bool,
    },
    TextKeyEnd {
        id: u64,
        input_context_consumed: bool,
    },
    /// Read/drain the original deferred native key result; never replay input.
    TextKeyResult {
        id: u64,
    },
    Commit {
        text: String,
        replacement: Option<Range<usize>>,
    },
    Preedit {
        text: String,
        selection: Range<usize>,
        clauses: Vec<Range<usize>>,
        replacement: Option<Range<usize>>,
    },
    Unmark,
    TextCommand(String),
    Cancel,
    /// Revoke original pointer capture without ending composition or focus.
    CancelPointerCapture,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub view: u64,
    pub viewport: u64,
    /// Event-time focused owner, or the original captured owner for a pointer
    /// delivery. Never substitute a later mount for a retained callback.
    /// Snapshot queries alone may omit this fence.
    pub owner: Option<Target>,
    /// Existing child focus generation and editor/document binding fence.
    pub focus_epoch: u64,
    pub text_binding: Option<Vec<u8>>,
    /// Original pointer gesture identity, independent of focus ownership.
    pub capture: Option<Capture>,
    pub input: Input,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextState {
    pub owner: Target,
    pub text: String,
    pub caret_index: usize,
    pub selection: Range<usize>,
    pub marked: Option<Range<usize>>,
    pub read_only: bool,
    /// Original shaped caret rectangles in child content-view coordinates.
    /// The parent applies the acknowledged container transform, never reshapes.
    pub caret: [f32; 4],
    /// Original editor visible inner clip in the same child logical coordinates.
    pub clip: [f32; 4],
    pub carets: Vec<(usize, [f32; 4])>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDisposition {
    Deferred,
    InputConsumed,
    EditHandled,
    Submitted,
    Navigated,
    Unhandled,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyOutcome {
    pub id: u64,
    pub target: Option<u64>,
    pub disposition: KeyDisposition,
}
/// Read-only projection; only the original child can issue attachment identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingVersion {
    pub key: String,
    pub revision: u64,
    pub attachment: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevealPlacement {
    Nearest,
    Centered,
    StartVertical,
    StartHorizontal,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealOptions {
    pub placement: RevealPlacement,
    /// Top, right, bottom, left, in original logical points.
    pub margins: [f32; 4],
    pub animated: bool,
}
impl RevealOptions {
    pub fn is_valid(&self) -> bool {
        self.margins
            .iter()
            .all(|value| value.is_finite() && *value >= 0.)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevealStatus {
    Animating,
    Completed,
    Interrupted,
    Rejected(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingNavigation {
    pub item: u64,
    pub status: RevealStatus,
    pub changed: bool,
    pub clamped: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingState {
    pub reader: Target,
    pub document: ReadingVersion,
    /// Only currently mounted items in the original active input scope.
    pub items: Vec<Target>,
    pub following_latest: bool,
    pub navigation: Option<ReadingNavigation>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub view: u64,
    pub viewport: u64,
    pub consumed: bool,
    pub changed: bool,
    /// Last original render's frame demand; not read-only query invalidation.
    #[serde(default)]
    pub animation_frame_requested: bool,
    pub capture_pointer: bool,
    pub activated: bool,
    pub bubbles: bool,
    /// Original activation focus intent; independent of current focus equality.
    pub pointer_focus_requested: bool,
    /// Original AX action acquired its semantic focus owner. Query replies omit intent.
    #[serde(default)]
    pub accessibility_focus_requested: bool,
    pub key: Option<KeyOutcome>,
    /// Original active input overlay owner; passive/closing paint is excluded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<Target>,
    /// Accepted original reading wheel source, not a second wheel dispatch.
    /// Only the exact input reply may carry it; Snapshot queries omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_scroll_source: Option<Target>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<ReadingState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantics: Option<Vec<SemanticNode>>,
    pub focused: Option<Target>,
    pub focus_epoch: u64,
    pub text_binding: Option<Vec<u8>>,
    /// Current original pointer owner, including captures outside an overlay's
    /// current semantic scope. This is not inferred from focus or hit position.
    pub captured: Option<Capture>,
    pub focus_targets: Vec<Target>,
    pub composing: bool,
    pub text: Option<TextState>,
}

#[cfg(test)]
mod scroll_tests {
    use super::*;
    #[test]
    fn scroll_roundtrip_preserves_original_coordinates_units_and_phases() {
        let event = ScrollEvent {
            position: [-12.5, 40.],
            delta: [2., -17.25],
            precise: true,
            begin: true,
            end: false,
            momentum: true,
            time: 123.25,
        };
        assert!(event.is_valid());
        let value = serde_json::to_value(Input::Scroll(event)).unwrap();
        assert_eq!(
            serde_json::from_value::<Input>(value.clone()).unwrap(),
            Input::Scroll(event)
        );
        let mut unknown = value;
        unknown["data"]["offset"] = 3.into();
        assert!(serde_json::from_value::<Input>(unknown).is_err());
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                !ScrollEvent {
                    delta: [0., invalid],
                    ..event
                }
                .is_valid()
            );
            assert!(
                !ScrollEvent {
                    position: [invalid, 0.],
                    ..event
                }
                .is_valid()
            );
        }
        for time in [f64::NAN, f64::INFINITY, -1.] {
            assert!(!ScrollEvent { time, ..event }.is_valid());
        }
    }
}
