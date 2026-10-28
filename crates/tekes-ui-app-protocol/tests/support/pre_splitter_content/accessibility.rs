// Frozen complete pre-Splitter schema from Main 277958f90f09cc96158b795595e8852ec2be8771.
#![allow(dead_code)]
// Data-only projection of original Content semantics, not another control tree.
use super::{Target, TextState};
use serde::{Deserialize, Serialize};
use std::ops::Range;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticRole {
    Group,
    Text,
    /// Matched-schema informative/decorative image semantics; no resource IDs.
    Image,
    Button,
    Link,
    TextInput,
    Slider,
    Switch,
    List,
    ListItem,
    Dialog,
    MenuButton,
    Menu,
    MenuItem,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Semantics {
    pub role: SemanticRole,
    pub name: String,
    pub value: Option<String>,
    pub disabled: bool,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub invalid: bool,
    pub required: bool,
    pub description: Option<String>,
}
/// Complete original shaper metadata, independent of the visible IME snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextLayout {
    pub binding: Vec<u8>,
    pub input: TextState,
    pub multiline: bool,
    pub lines: Vec<Range<usize>>,
    pub carets: Vec<(usize, [f32; 4])>,
}
impl TextLayout {
    /// Reject incomplete/malformed query geometry, never invent line boundaries.
    pub fn is_valid(&self) -> bool {
        let units: Vec<_> = self.input.text.encode_utf16().collect();
        let boundary = |i: usize| {
            i <= units.len() && !units.get(i).is_some_and(|u| (0xDC00..=0xDFFF).contains(u))
        };
        let range = |r: &Range<usize>| r.start <= r.end && boundary(r.start) && boundary(r.end);
        let rect = |r: &[f32; 4]| {
            r.iter().all(|v| v.is_finite())
                && r[2] >= 0.
                && r[3] >= 0.
                && (r[0] + r[2]).is_finite()
                && (r[1] + r[3]).is_finite()
        };
        let mut end = 0;
        !self.lines.is_empty()
            && self.lines.iter().enumerate().all(|(i, r)| {
                let valid =
                    range(r) && r.start == end && (!r.is_empty() || i + 1 == self.lines.len());
                end = r.end;
                valid
            })
            && end == units.len()
            && boundary(self.input.caret_index)
            && range(&self.input.selection)
            && self.input.marked.as_ref().is_none_or(range)
            && rect(&self.input.caret)
            && rect(&self.input.clip)
            && self
                .input
                .carets
                .iter()
                .chain(&self.carets)
                .all(|(i, r)| boundary(*i) && rect(r) && r[3] > 0.)
            && self.carets.iter().any(|(i, _)| *i == 0)
            && self.carets.iter().any(|(i, _)| *i == units.len())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticPart {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_layout: Option<TextLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_range: Option<[f64; 2]>,
    pub index: usize,
    pub parent: Option<usize>,
    pub frame: [f32; 4],
    pub focused: bool,
    pub semantics: Semantics,
    #[serde(default)]
    pub preserves_activation_focus: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_layout: Option<TextLayout>,
    pub target: Target,
    pub parent: Option<Target>,
    pub frame: [f32; 4],
    pub visible: Option<[f32; 4]>,
    pub focused: bool,
    pub semantics: Semantics,
    #[serde(default)]
    pub preserves_activation_focus: bool,
    pub parts: Vec<SemanticPart>,
    pub value_range: Option<[f64; 2]>,
    /// Original shaped text, also for unfocused fields in the active scope.
    /// Original editor/document binding; ordinary edits/selection do not change it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_binding: Option<Vec<u8>>,
    pub text: Option<TextState>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessibilityAction {
    Increment,
    Decrement,
    Focus,
    Press,
    SetValue(String),
    SetSelection(Range<usize>),
    ReplaceSelectedText(String),
}
impl AccessibilityAction {
    pub fn is_valid(&self) -> bool {
        !matches!(self, Self::SetSelection(r) if r.start > r.end)
    }
}
