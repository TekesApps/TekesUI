//! Data-only projection of original Content semantics, not another control tree.
use super::{Target, TextState};
use serde::{Deserialize, Serialize};
use std::ops::Range;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticRole {
    Group,
    /// Read-only progress metadata; unknown progress has no numeric range.
    ProgressIndicator,
    Text,
    /// Matched-schema informative/decorative image semantics; no resource IDs.
    Image,
    Button,
    Link,
    TextInput,
    /// Matched-schema editable single-line owner; no extra popup editor.
    ComboBox,
    Slider,
    /// Original split divider; requires a matching receiver.
    Splitter,
    Switch,
    Checkbox,
    RadioGroup,
    RadioButton,
    SegmentedGroup,
    Segment,
    List,
    ListItem,
    Dialog,
    MenuButton,
    Menu,
    MenuItem,
    TabList,
    Tab,
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
    /// Set the original owner disclosure state; never toggle via Press.
    SetExpanded(bool),
    SetValue(String),
    SetSelection(Range<usize>),
    ReplaceSelectedText(String),
}
impl AccessibilityAction {
    pub fn is_valid(&self) -> bool {
        !matches!(self, Self::SetSelection(r) if r.start > r.end)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_text_layout_rejects_gaps_surrogates_and_nonfinite_geometry() {
        let target = Target { id: 1, mount: 2 };
        let mut t = TextLayout {
            binding: vec![7],
            input: TextState {
                owner: target,
                text: "A😀B\nC".into(),
                caret_index: 1,
                selection: 1..3,
                marked: None,
                read_only: true,
                caret: [0., 0., 1., 20.],
                clip: [0., 0., 40., 20.],
                carets: vec![(0, [0., 0., 1., 20.])],
            },
            multiline: true,
            lines: vec![0..5, 5..6],
            carets: vec![
                (0, [0., 0., 1., 20.]),
                (1, [8., 0., 1., 20.]),
                (3, [16., 0., 1., 20.]),
                (4, [24., 0., 1., 20.]),
                (5, [0., 20., 1., 20.]),
                (6, [8., 20., 1., 20.]),
            ],
        };
        assert!(t.is_valid());
        let original = t.clone();
        t.lines[1] = 4..6;
        assert!(!t.is_valid());
        t = original.clone();
        t.input.selection = 1..2;
        assert!(!t.is_valid());
        t = original.clone();
        t.carets[0].1[0] = f32::INFINITY;
        assert!(!t.is_valid());
        t = original.clone();
        t.carets.pop();
        assert!(!t.is_valid());
        let encoded = serde_json::to_vec(&original).unwrap();
        assert_eq!(
            serde_json::from_slice::<TextLayout>(&encoded).unwrap(),
            original
        );
    }
    #[test]
    fn semantic_callback_roundtrip_retains_mount_and_utf16_selection() {
        let input = super::super::Input::Accessibility {
            target: Target { id: 41, mount: 92 },
            part: Some(17),
            binding: Some(vec![9, 42]),
            action: AccessibilityAction::SetSelection(1..3),
        };
        let value = serde_json::to_value(&input).unwrap();
        assert_eq!(
            serde_json::from_value::<super::super::Input>(value.clone()).unwrap(),
            input
        );
        let mut unknown = value;
        unknown["data"]["generation"] = 1.into();
        assert!(serde_json::from_value::<super::super::Input>(unknown).is_err());
        assert!(AccessibilityAction::SetSelection(1..3).is_valid());
        assert!(
            !AccessibilityAction::SetSelection(std::ops::Range { start: 3, end: 1 }).is_valid()
        );
    }
}

#[cfg(test)]
mod navigation_role_contract_tests {
    use super::SemanticRole;

    #[test]
    fn tab_roles_roundtrip_and_future_roles_fail_closed() {
        for (role, encoded) in [
            (SemanticRole::TabList, r#""TabList""#),
            (SemanticRole::Tab, r#""Tab""#),
        ] {
            assert_eq!(serde_json::to_string(&role).unwrap(), encoded);
            assert_eq!(serde_json::from_str::<SemanticRole>(encoded).unwrap(), role);
        }
        assert!(serde_json::from_str::<SemanticRole>(r#""FutureRole""#).is_err());
    }
}
