//! Platform-neutral accessibility snapshots published by the host.
//!
//! Coordinates in `AccessibilityNode::frame` are local to the content view,
//! in points, with the same top-left origin as input and painting.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccessibilityKey {
    pub component: u64,
    pub part: Option<usize>,
}

/// Finite slider domain. Stored in the role-specific descriptor so existing
/// AccessibilityNode struct literals do not gain a required field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibilityRange {
    minimum: f64,
    maximum: f64,
}
// Constructors preserve finite values (no NaN), so equality is reflexive.
impl Eq for AccessibilityRange {}
impl AccessibilityRange {
    pub fn new(minimum: f64, maximum: f64) -> Option<Self> {
        (minimum.is_finite() && maximum.is_finite() && minimum <= maximum)
            .then_some(Self { minimum, maximum })
    }
    pub fn minimum(self) -> f64 {
        self.minimum
    }
    pub fn maximum(self) -> f64 {
        self.maximum
    }
    fn contains(self, value: f64) -> bool {
        value.is_finite() && (self.minimum..=self.maximum).contains(&value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilitySortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    Slider(AccessibilityRange),
    Group,
    StaticText,
    Button,
    Link,
    Switch,
    /// Checkbox value is `selected`; `value == Some("mixed")` represents indeterminate.
    Checkbox,
    RadioButton,
    RadioGroup,
    TextField,
    TextArea,
    List,
    ListItem,
    Tree,
    TreeItem {
        level: usize,
    },
    Table,
    TableColumn {
        index: usize,
    },
    TableColumnHeader {
        index: usize,
        sort: Option<AccessibilitySortDirection>,
    },
    TableRow {
        index: usize,
    },
    TableCell {
        row: usize,
        column: usize,
    },
    Dialog,
    Menu,
    MenuButton,
    MenuItem,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityNode {
    pub key: AccessibilityKey,
    pub parent: Option<AccessibilityKey>,
    pub role: AccessibilityRole,
    pub frame: crate::placement::Rect,
    pub label: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub invalid: bool,
    /// Whether this element must contain a value for form submission.
    pub required: bool,
    /// UTF-16 selection for text fields; absent for non-text semantic owners.
    pub text_selection: Option<std::ops::Range<usize>>,
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccessibilityAction {
    Increment,
    Decrement,
    Focus,
    Press,
    SetValue(String),
    SetSelection(std::ops::Range<usize>),
    ReplaceSelectedText(String),
    SetExpanded(bool),
}

/// Complete shaped text layout for one semantic owner, including unfocused and
/// read-only editors. Reuses the text-services geometry; unlike a visible IME
/// snapshot, `state.carets` and `lines` must cover the whole document. Line ranges
/// come from the shaper (including soft wraps and explicit newline characters),
/// never from splitting the string. Coordinates remain content-view points.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityTextLayout {
    pub state: crate::event::TextInputState,
    pub lines: Vec<std::ops::Range<usize>>,
}

impl AccessibilityTextLayout {
    /// Reject partial, stale or malformed layouts rather than expose invented
    /// line numbers. Empty documents still have one shaped empty line.
    pub fn is_valid(&self) -> bool {
        let units: Vec<_> = self.state.text.encode_utf16().collect();
        let len = units.len();
        let boundary = |index| {
            index <= len
                && !units
                    .get(index)
                    .is_some_and(|u| (0xDC00..=0xDFFF).contains(u))
        };
        let mut end = 0;
        if self.lines.is_empty() {
            return false;
        }
        for (i, line) in self.lines.iter().enumerate() {
            if line.start != end
                || line.end > len
                || line.end < line.start
                || (line.is_empty() && i + 1 != self.lines.len())
                || !boundary(line.start)
                || !boundary(line.end)
            {
                return false;
            }
            end = line.end;
        }
        end == len
            && self.state.carets.iter().all(|(index, p, size)| {
                *index <= len
                    && boundary(*index)
                    && p.x.is_finite()
                    && p.y.is_finite()
                    && size.width.is_finite()
                    && size.height.is_finite()
                    && size.height > 0.
            })
    }

    pub fn line_for_index(&self, index: usize) -> Option<usize> {
        if !self.is_valid() || text_in_range(&self.state.text, index..index).is_none() {
            return None;
        }
        // A soft-wrap boundary belongs to the following line; end-of-document
        // belongs to the last line, including a trailing empty line.
        self.lines.iter().rposition(|line| line.start <= index)
    }

    pub fn range_for_line(&self, line: usize) -> Option<std::ops::Range<usize>> {
        self.is_valid()
            .then(|| self.lines.get(line).cloned())
            .flatten()
    }

    /// The shaped grapheme containing a UTF-16 index, including an index inside
    /// a surrogate pair. Carets already come from CoreText's composed boundaries.
    pub fn range_for_index(&self, index: usize) -> Option<std::ops::Range<usize>> {
        if !self.is_valid() {
            return None;
        }
        let len = self.state.text.encode_utf16().count();
        if index > len {
            return None;
        }
        let mut boundaries: Vec<_> = self.state.carets.iter().map(|c| c.0).collect();
        boundaries.sort_unstable();
        boundaries.dedup();
        if boundaries.first() != Some(&0) || boundaries.last() != Some(&len) {
            return None;
        }
        if index == len {
            return Some(len..len);
        }
        boundaries
            .windows(2)
            .find(|p| p[0] <= index && index < p[1])
            .map(|p| p[0]..p[1])
    }

    /// Entire shaped lines intersecting the viewport vertically, including
    /// horizontally clipped characters, as required by AppKit's text contract.
    pub fn visible_range(
        &self,
        viewport: crate::placement::Rect,
    ) -> Option<std::ops::Range<usize>> {
        if !self.is_valid()
            || !viewport.origin.y.is_finite()
            || !viewport.size.height.is_finite()
            || viewport.size.height <= 0.
        {
            return None;
        }
        let mut visible: Option<std::ops::Range<usize>> = None;
        for line in &self.lines {
            let (y, height) = self.line_geometry(line.start)?;
            if y + height > viewport.origin.y && y < viewport.origin.y + viewport.size.height {
                visible = Some(visible.map_or(line.clone(), |r| r.start..line.end));
            }
        }
        visible
    }

    /// Hit-test the real visual grapheme intervals on a shaped line. No
    /// character-width approximation and no fallback to another focused editor.
    pub fn range_for_position(&self, point: crate::event::Point) -> Option<std::ops::Range<usize>> {
        if !self.is_valid() || !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        for line in &self.lines {
            let (y, height) = self.line_geometry(line.start)?;
            if point.y < y || point.y >= y + height {
                continue;
            }
            let mut carets: Vec<_> = self
                .state
                .carets
                .iter()
                .filter(|c| (c.1.y - y).abs() < 0.01 && line.start <= c.0 && c.0 <= line.end)
                .collect();
            carets.sort_by_key(|c| c.0);
            for pair in carets.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if a.0 < b.0 && point.x >= a.1.x.min(b.1.x) && point.x < a.1.x.max(b.1.x) {
                    return Some(a.0..b.0);
                }
            }
            return None;
        }
        None
    }

    fn line_geometry(&self, start: usize) -> Option<(f64, f64)> {
        self.state
            .carets
            .iter()
            .filter(|c| c.0 == start)
            .max_by(|a, b| a.1.y.total_cmp(&b.1.y))
            .map(|c| (c.1.y, c.2.height))
    }

    /// Bounding frame of the requested range across all shaped lines. No
    /// focused editor fallback, string-based line estimation, or screen scaling.
    pub fn frame_for_range(&self, range: std::ops::Range<usize>) -> Option<crate::placement::Rect> {
        self.line_for_index(range.start)?;
        text_in_range(&self.state.text, range.clone())?;
        let first_line = self.line_for_index(range.start)?;
        let mut bounds: Option<crate::placement::Rect> = None;
        for (index, line) in self.lines.iter().enumerate().skip(first_line) {
            if index > first_line && line.start >= range.end {
                break;
            }
            // A soft-wrap boundary has two carets: choose the downstream line.
            let y = self
                .state
                .carets
                .iter()
                .filter(|c| c.0 == line.start)
                .map(|c| c.1.y)
                .max_by(f64::total_cmp)?;
            let mut state = self.state.clone();
            state.carets.retain(|c| (c.1.y - y).abs() < 0.01);
            state.caret_index = usize::MAX;
            let (_, origin, size) =
                state.first_rect_for_range(range.start.max(line.start)..range.end.min(line.end))?;
            let frame = crate::placement::Rect { origin, size };
            bounds = Some(match bounds {
                None => frame,
                Some(old) => {
                    let x = old.origin.x.min(origin.x);
                    let y = old.origin.y.min(origin.y);
                    crate::placement::Rect {
                        origin: crate::event::Point { x, y },
                        size: crate::event::Size {
                            width: (old.origin.x + old.size.width).max(origin.x + size.width) - x,
                            height: (old.origin.y + old.size.height).max(origin.y + size.height)
                                - y,
                        },
                    }
                }
            });
        }
        bounds
    }
}

fn text_in_range(text: &str, range: std::ops::Range<usize>) -> Option<String> {
    let units: Vec<_> = text.encode_utf16().collect();
    for index in [range.start, range.end] {
        if units
            .get(index)
            .is_some_and(|u| (0xDC00..=0xDFFF).contains(u))
        {
            return None;
        }
    }
    String::from_utf16(units.get(range)?).ok()
}

/// Reject malformed host snapshots before they reach AppKit. Keeping this
/// validator platform independent makes tree integrity testable without AX.
pub(crate) fn validate_accessibility_nodes(nodes: &[AccessibilityNode]) -> bool {
    use std::collections::{HashMap, HashSet};

    let mut parents = HashMap::with_capacity(nodes.len());
    for node in nodes {
        let frame = node.frame;
        if !frame.origin.x.is_finite()
            || !frame.origin.y.is_finite()
            || !frame.size.width.is_finite()
            || !frame.size.height.is_finite()
            || frame.size.width <= 0.0
            || frame.size.height <= 0.0
            || !(frame.origin.x + frame.size.width).is_finite()
            || !(frame.origin.y + frame.size.height).is_finite()
            || parents.insert(node.key, node.parent).is_some()
            || matches!(node.role, AccessibilityRole::Slider(domain)
                if node.value.as_ref().and_then(|v|v.parse::<f64>().ok()).is_none_or(|v|!domain.contains(v)))
            || node.text_selection.as_ref().is_some_and(|range| {
                !matches!(
                    node.role,
                    AccessibilityRole::TextField | AccessibilityRole::TextArea
                ) || text_in_range(node.value.as_deref().unwrap_or(""), range.clone()).is_none()
            })
        {
            return false;
        }
    }

    if parents
        .values()
        .flatten()
        .any(|parent| !parents.contains_key(parent))
    {
        return false;
    }

    let mut complete = HashSet::with_capacity(nodes.len());
    for start in parents.keys().copied() {
        let mut path = HashSet::new();
        let mut cursor = Some(start);
        while let Some(key) = cursor {
            if complete.contains(&key) {
                break;
            }
            if !path.insert(key) {
                return false;
            }
            cursor = parents[&key];
        }
        complete.extend(path);
    }
    true
}

#[cfg(target_os = "macos")]
pub(crate) mod native {
    use std::rc::{Rc, Weak};

    use objc2::rc::Retained;
    use objc2::{AnyThread, DefinedClass, MainThreadOnly, Message, define_class, msg_send, sel};
    use objc2_app_kit::{NSAccessibility, NSAccessibilityElement, NSAccessibilityRole};
    use objc2_foundation::{
        MainThreadMarker, NSArray, NSAttributedString, NSObjectProtocol, NSPoint, NSRange, NSRect,
        NSSize, NSString, NSValue,
    };

    use super::{AccessibilityAction, AccessibilityKey, AccessibilityNode, AccessibilityRole};
    use crate::event::Event;
    use crate::shared::Shared;

    #[derive(Debug)]
    pub(crate) struct ElementIvars {
        shared: Weak<Shared>,
        key: AccessibilityKey,
    }

    define_class!(
        #[unsafe(super = NSAccessibilityElement)]
        #[thread_kind = MainThreadOnly]
        #[ivars = ElementIvars]
        pub(crate) struct NativeAccessibilityElement;

        unsafe impl NSObjectProtocol for NativeAccessibilityElement {}
        unsafe impl NSAccessibility for NativeAccessibilityElement {}

        #[allow(non_snake_case)]
        impl NativeAccessibilityElement {
            #[unsafe(method(isAccessibilityElement))]
            fn isAccessibilityElement(&self) -> bool {
                self.node().is_some()
            }

            #[unsafe(method_id(accessibilityRole))]
            fn accessibilityRole(&self) -> Retained<NSAccessibilityRole> {
                let role = unsafe {
                    match self
                        .node()
                        .map(|n| n.role)
                        .unwrap_or(AccessibilityRole::Group)
                    {
                        AccessibilityRole::Slider(_) => objc2_app_kit::NSAccessibilitySliderRole,
                        AccessibilityRole::Group => objc2_app_kit::NSAccessibilityGroupRole,
                        AccessibilityRole::StaticText => {
                            objc2_app_kit::NSAccessibilityStaticTextRole
                        }
                        AccessibilityRole::Button => objc2_app_kit::NSAccessibilityButtonRole,
                        AccessibilityRole::Link => objc2_app_kit::NSAccessibilityLinkRole,
                        AccessibilityRole::Switch | AccessibilityRole::Checkbox => {
                            objc2_app_kit::NSAccessibilityCheckBoxRole
                        }
                        AccessibilityRole::RadioButton => {
                            objc2_app_kit::NSAccessibilityRadioButtonRole
                        }
                        AccessibilityRole::RadioGroup => {
                            objc2_app_kit::NSAccessibilityRadioGroupRole
                        }
                        AccessibilityRole::TextField => objc2_app_kit::NSAccessibilityTextFieldRole,
                        AccessibilityRole::TextArea => objc2_app_kit::NSAccessibilityTextAreaRole,
                        AccessibilityRole::List => objc2_app_kit::NSAccessibilityListRole,
                        AccessibilityRole::ListItem
                        | AccessibilityRole::TreeItem { .. }
                        | AccessibilityRole::TableRow { .. } => {
                            objc2_app_kit::NSAccessibilityRowRole
                        }
                        AccessibilityRole::Tree => objc2_app_kit::NSAccessibilityOutlineRole,
                        AccessibilityRole::Table => objc2_app_kit::NSAccessibilityTableRole,
                        AccessibilityRole::TableColumn { .. } => {
                            objc2_app_kit::NSAccessibilityColumnRole
                        }
                        AccessibilityRole::TableColumnHeader { .. } => {
                            objc2_app_kit::NSAccessibilityButtonRole
                        }
                        AccessibilityRole::TableCell { .. } => {
                            objc2_app_kit::NSAccessibilityCellRole
                        }
                        AccessibilityRole::Dialog => objc2_app_kit::NSAccessibilityGroupRole,
                        AccessibilityRole::Menu => objc2_app_kit::NSAccessibilityMenuRole,
                        AccessibilityRole::MenuButton => {
                            objc2_app_kit::NSAccessibilityMenuButtonRole
                        }
                        AccessibilityRole::MenuItem => objc2_app_kit::NSAccessibilityMenuItemRole,
                    }
                };
                role.retain()
            }

            #[unsafe(method_id(accessibilitySubrole))]
            fn accessibilitySubrole(&self) -> Retained<NSString> {
                if self
                    .node()
                    .is_some_and(|n| n.role == AccessibilityRole::Dialog)
                {
                    unsafe { objc2_app_kit::NSAccessibilityDialogSubrole.retain() }
                } else if self
                    .node()
                    .is_some_and(|n| matches!(n.role, AccessibilityRole::TreeItem { .. }))
                {
                    unsafe { objc2_app_kit::NSAccessibilityOutlineRowSubrole.retain() }
                } else {
                    NSString::from_str("")
                }
            }

            #[unsafe(method_id(accessibilityLabel))]
            fn accessibilityLabel(&self) -> Retained<NSString> {
                NSString::from_str(&self.node().map(|n| n.label).unwrap_or_default())
            }

            #[unsafe(method_id(accessibilityValue))]
            fn accessibilityValue(&self) -> Retained<objc2::runtime::AnyObject> {
                match self.node() {
                    Some(node)
                        if matches!(
                            node.role,
                            AccessibilityRole::Switch
                                | AccessibilityRole::Checkbox
                                | AccessibilityRole::RadioButton
                        ) =>
                    {
                        objc2_foundation::NSNumber::new_i32(
                            if node.role == AccessibilityRole::Checkbox
                                && node.value.as_deref() == Some("mixed")
                            {
                                -1
                            } else {
                                i32::from(node.selected)
                            },
                        )
                        .into()
                    }
                    Some(node) if matches!(node.role, AccessibilityRole::Slider(_)) => {
                        objc2_foundation::NSNumber::new_f64(
                            node.value.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.),
                        )
                        .into()
                    }
                    Some(node) => NSString::from_str(&node.value.unwrap_or_default()).into(),
                    None => NSString::from_str("").into(),
                }
            }

            #[unsafe(method_id(accessibilityHelp))]
            fn accessibilityHelp(&self) -> Retained<NSString> {
                NSString::from_str(&self.node().and_then(|n| n.description).unwrap_or_default())
            }

            #[unsafe(method(accessibilityNumberOfCharacters))]
            fn accessibilityNumberOfCharacters(&self) -> isize {
                self.node()
                    .and_then(|n| n.value)
                    .map_or(0, |v| v.encode_utf16().count() as isize)
            }

            #[unsafe(method(accessibilitySelectedTextRange))]
            fn accessibilitySelectedTextRange(&self) -> NSRange {
                self.node()
                    .and_then(|n| n.text_selection)
                    .map_or(NSRange::new(isize::MAX as usize, 0), |r| {
                        NSRange::new(r.start, r.end - r.start)
                    })
            }

            #[unsafe(method_id(accessibilitySelectedText))]
            fn accessibilitySelectedText(&self) -> Option<Retained<NSString>> {
                (|| {
                    let node = self.node()?;
                    let value = super::text_in_range(node.value.as_deref()?, node.text_selection?)?;
                    Some(NSString::from_str(&value))
                })()
            }

            #[unsafe(method_id(accessibilityStringForRange:))]
            fn accessibilityStringForRange(&self, range: NSRange) -> Option<Retained<NSString>> {
                (|| {
                    let node = self.node()?;
                    if !matches!(
                        node.role,
                        AccessibilityRole::TextField | AccessibilityRole::TextArea
                    ) {
                        return None;
                    }
                    let end = range.location.checked_add(range.length)?;
                    super::text_in_range(node.value.as_deref()?, range.location..end)
                        .map(|v| NSString::from_str(&v))
                })()
            }

            #[unsafe(method_id(accessibilityAttributedStringForRange:))]
            fn accessibilityAttributedStringForRange(
                &self,
                range: NSRange,
            ) -> Option<Retained<NSAttributedString>> {
                self.trace_text(&format!("attributed-string {range:?}"));
                NSAccessibility::accessibilityStringForRange(self, range).map(|text| {
                    NSAttributedString::initWithString(NSAttributedString::alloc(), &text)
                })
            }

            #[unsafe(method_id(accessibilitySelectedTextRanges))]
            fn accessibilitySelectedTextRanges(&self) -> Option<Retained<NSArray<NSValue>>> {
                (|| {
                    let node = self.node()?;
                    if !matches!(
                        node.role,
                        AccessibilityRole::TextField | AccessibilityRole::TextArea
                    ) {
                        return None;
                    }
                    let r = node.text_selection?;
                    let value = unsafe { NSValue::valueWithRange(NSRange::new(r.start, r.len())) };
                    Some(NSArray::from_retained_slice(&[value]))
                })()
            }

            #[unsafe(method(setAccessibilitySelectedTextRanges:))]
            fn setAccessibilitySelectedTextRanges(&self, ranges: Option<&NSArray>) {
                self.trace_text("set-selection-ranges");
                let Some(ranges) = ranges.filter(|r| r.count() == 1) else {
                    return;
                };
                let Some(value) = ranges.firstObject() else {
                    return;
                };
                let Some(value) = value.downcast_ref::<NSValue>() else {
                    return;
                };
                let template = unsafe { NSValue::valueWithRange(NSRange::new(0, 0)) };
                // AX clients are external: an arbitrary NSValue is not a range.
                let correct_type = unsafe {
                    std::ffi::CStr::from_ptr(value.objCType().as_ptr())
                        == std::ffi::CStr::from_ptr(template.objCType().as_ptr())
                };
                if correct_type {
                    NSAccessibility::setAccessibilitySelectedTextRange(self, unsafe {
                        value.rangeValue()
                    });
                }
            }

            #[unsafe(method(accessibilityVisibleCharacterRange))]
            fn accessibilityVisibleCharacterRange(&self) -> NSRange {
                self.trace_text("visible-character-range");
                let range = self
                    .node()
                    .and_then(|n| self.text_layout()?.visible_range(n.frame));
                Self::native_range(range)
            }

            #[unsafe(method(accessibilityRangeForIndex:))]
            fn accessibilityRangeForIndex(&self, index: isize) -> NSRange {
                self.trace_text(&format!("range-for-index {index}"));
                Self::native_range(
                    usize::try_from(index)
                        .ok()
                        .and_then(|i| self.text_layout()?.range_for_index(i)),
                )
            }

            #[unsafe(method(accessibilityRangeForPosition:))]
            fn accessibilityRangeForPosition(&self, point: NSPoint) -> NSRange {
                self.trace_text(&format!("range-for-position {point:?}"));
                let Some((shared, node)) = self.shared_and_node() else {
                    return Self::native_range(None);
                };
                let Some(objects) = shared.objects.get() else {
                    return Self::native_range(None);
                };
                let local = objects
                    .view
                    .convertPoint_fromView(objects.window.convertPointFromScreen(point), None);
                let f = node.frame;
                if local.x < f.origin.x
                    || local.y < f.origin.y
                    || local.x >= f.origin.x + f.size.width
                    || local.y >= f.origin.y + f.size.height
                {
                    return Self::native_range(None);
                }
                Self::native_range(self.text_layout().and_then(|l| {
                    l.range_for_position(crate::event::Point {
                        x: local.x,
                        y: local.y,
                    })
                }))
            }

            #[unsafe(method(accessibilityLineForIndex:))]
            fn accessibilityLineForIndex(&self, index: isize) -> isize {
                self.trace_text(&format!("line-for-index {index}"));
                usize::try_from(index)
                    .ok()
                    .and_then(|i| self.text_layout()?.line_for_index(i))
                    .and_then(|i| isize::try_from(i).ok())
                    .unwrap_or(isize::MAX)
            }

            #[unsafe(method(accessibilityRangeForLine:))]
            fn accessibilityRangeForLine(&self, line: isize) -> NSRange {
                self.trace_text(&format!("range-for-line {line}"));
                usize::try_from(line)
                    .ok()
                    .and_then(|i| self.text_layout()?.range_for_line(i))
                    .map_or(NSRange::new(isize::MAX as usize, 0), |r| {
                        NSRange::new(r.start, r.len())
                    })
            }

            #[unsafe(method(accessibilityInsertionPointLineNumber))]
            fn accessibilityInsertionPointLineNumber(&self) -> isize {
                self.text_layout()
                    .and_then(|l| l.line_for_index(l.state.caret_index))
                    .and_then(|i| isize::try_from(i).ok())
                    .unwrap_or(isize::MAX)
            }

            #[unsafe(method(accessibilityFrameForRange:))]
            fn accessibilityFrameForRange(&self, range: NSRange) -> NSRect {
                self.trace_text(&format!("frame-for-range {range:?}"));
                let Some(end) = range.location.checked_add(range.length) else {
                    return NSRect::ZERO;
                };
                let Some(frame) = self
                    .text_layout()
                    .and_then(|l| l.frame_for_range(range.location..end))
                else {
                    return NSRect::ZERO;
                };
                let Some(shared) = self.ivars().shared.upgrade() else {
                    return NSRect::ZERO;
                };
                let Some(objects) = shared.objects.get() else {
                    return NSRect::ZERO;
                };
                let local = NSRect::new(
                    NSPoint::new(frame.origin.x, frame.origin.y),
                    NSSize::new(frame.size.width, frame.size.height),
                );
                let window_rect = objects.view.convertRect_toView(local, None);
                objects.window.convertRectToScreen(window_rect)
            }

            #[unsafe(method(setAccessibilitySelectedTextRange:))]
            fn setAccessibilitySelectedTextRange(&self, range: NSRange) {
                self.trace_text(&format!("set-selection {range:?}"));
                let Some(end) = range.location.checked_add(range.length) else {
                    return;
                };
                if self.node().is_some_and(|n| {
                    matches!(
                        n.role,
                        AccessibilityRole::TextField | AccessibilityRole::TextArea
                    ) && n.text_selection.is_some()
                }) {
                    self.dispatch(AccessibilityAction::SetSelection(range.location..end));
                }
            }

            #[unsafe(method(setAccessibilitySelectedText:))]
            unsafe fn setAccessibilitySelectedText(
                &self,
                value: Option<&objc2::runtime::AnyObject>,
            ) {
                if self.node().is_some_and(|n| {
                    matches!(
                        n.role,
                        AccessibilityRole::TextField | AccessibilityRole::TextArea
                    ) && !n.read_only
                }) && let Some(value) = value.and_then(|v| v.downcast_ref::<NSString>())
                {
                    self.dispatch(AccessibilityAction::ReplaceSelectedText(value.to_string()));
                }
            }

            #[unsafe(method(accessibilityFrame))]
            fn accessibilityFrame(&self) -> NSRect {
                let Some((shared, node)) = self.shared_and_node() else {
                    return NSRect::ZERO;
                };
                let local = NSRect::new(
                    NSPoint::new(node.frame.origin.x, node.frame.origin.y),
                    NSSize::new(node.frame.size.width, node.frame.size.height),
                );
                let Some(objects) = shared.objects.get() else {
                    return NSRect::ZERO;
                };
                let window_rect = objects.view.convertRect_toView(local, None);
                objects.window.convertRectToScreen(window_rect)
            }

            #[unsafe(method_id(accessibilityParent))]
            fn accessibilityParent(&self) -> Retained<objc2::runtime::AnyObject> {
                let object: Retained<objc2::runtime::AnyObject> = match self.shared_and_node() {
                    Some((shared, node)) => match node.parent {
                        Some(parent) => shared
                            .accessibility_elements
                            .borrow()
                            .get(&parent)
                            .map(|e| e.retain().into())
                            .unwrap_or_else(|| objc2_foundation::NSNull::null().into()),
                        None => shared
                            .objects
                            .get()
                            .map(|o| o.view.retain().into())
                            .unwrap_or_else(|| objc2_foundation::NSNull::null().into()),
                    },
                    None => objc2_foundation::NSNull::null().into(),
                };
                object
            }

            #[unsafe(method_id(accessibilityChildren))]
            fn accessibilityChildren(&self) -> Retained<NSArray> {
                let parent = self.shared_and_node().map(|(_, n)| n.key);
                self.elements_for_parent(parent)
            }

            #[unsafe(method(isAccessibilityEnabled))]
            fn accessibilityEnabled(&self) -> bool {
                self.node().is_some_and(|n| n.enabled)
            }

            #[unsafe(method(isAccessibilityRequired))]
            fn accessibilityRequired(&self) -> bool {
                self.node().is_some_and(|n| n.required)
            }

            #[unsafe(method(isAccessibilityFocused))]
            fn isAccessibilityFocused(&self) -> bool {
                self.node().is_some_and(|n| n.focused)
            }

            #[unsafe(method(isAccessibilitySelected))]
            fn accessibilitySelected(&self) -> bool {
                self.node().is_some_and(|n| n.selected)
            }

            #[unsafe(method(isAccessibilityExpanded))]
            fn accessibilityExpanded(&self) -> bool {
                self.node().and_then(|n| n.expanded).unwrap_or(false)
            }

            #[unsafe(method(setAccessibilityExpanded:))]
            fn accessibilitySetExpanded(&self, expanded: bool) {
                if self.node().is_some_and(|n| {
                    matches!(n.role, AccessibilityRole::TreeItem { .. }) && n.expanded.is_some()
                }) {
                    self.dispatch(AccessibilityAction::SetExpanded(expanded));
                }
            }
            #[unsafe(method(isAccessibilityDisclosed))]
            fn accessibilityDisclosed(&self) -> bool {
                self.node().and_then(|n| n.expanded).unwrap_or(false)
            }
            #[unsafe(method(setAccessibilityDisclosed:))]
            fn accessibilitySetDisclosed(&self, expanded: bool) {
                if self.node().is_some_and(|n| {
                    matches!(n.role, AccessibilityRole::TreeItem { .. }) && n.expanded.is_some()
                }) {
                    self.dispatch(AccessibilityAction::SetExpanded(expanded));
                }
            }
            #[unsafe(method(accessibilityDisclosureLevel))]
            fn accessibilityDisclosureLevel(&self) -> isize {
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::TreeItem { level }) => {
                        level.try_into().unwrap_or(isize::MAX)
                    }
                    _ => 0,
                }
            }
            #[unsafe(method(accessibilityIndex))]
            fn accessibilityIndex(&self) -> isize {
                match self.node().map(|n| n.role) {
                    Some(
                        AccessibilityRole::TableRow { index }
                        | AccessibilityRole::TableColumn { index }
                        | AccessibilityRole::TableColumnHeader { index, .. },
                    ) => index.try_into().unwrap_or(isize::MAX),
                    _ => 0,
                }
            }
            #[unsafe(method(accessibilityRowIndexRange))]
            fn accessibilityRowIndexRange(&self) -> NSRange {
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::TableCell { row, .. }) => NSRange::new(row, 1),
                    _ => NSRange::new(0, 0),
                }
            }
            #[unsafe(method(accessibilityColumnIndexRange))]
            fn accessibilityColumnIndexRange(&self) -> NSRange {
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::TableCell { column, .. }) => NSRange::new(column, 1),
                    _ => NSRange::new(0, 0),
                }
            }
            #[unsafe(method(accessibilitySortDirection))]
            fn accessibilitySortDirection(&self) -> objc2_app_kit::NSAccessibilitySortDirection {
                use super::AccessibilitySortDirection as D;
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::TableColumnHeader {
                        sort: Some(D::Ascending),
                        ..
                    }) => objc2_app_kit::NSAccessibilitySortDirection::Ascending,
                    Some(AccessibilityRole::TableColumnHeader {
                        sort: Some(D::Descending),
                        ..
                    }) => objc2_app_kit::NSAccessibilitySortDirection::Descending,
                    _ => objc2_app_kit::NSAccessibilitySortDirection::Unknown,
                }
            }
            #[unsafe(method_id(accessibilityRows))]
            fn accessibilityRows(&self) -> Retained<NSArray> {
                self.collection_rows()
            }
            #[unsafe(method_id(accessibilityVisibleRows))]
            fn accessibilityVisibleRows(&self) -> Retained<NSArray> {
                self.collection_rows()
            }
            #[unsafe(method_id(accessibilitySelectedRows))]
            fn accessibilitySelectedRows(&self) -> Retained<NSArray> {
                self.collection_elements(|n| {
                    n.selected
                        && matches!(
                            n.role,
                            AccessibilityRole::TreeItem { .. } | AccessibilityRole::TableRow { .. }
                        )
                })
            }
            #[unsafe(method_id(accessibilityColumns))]
            fn accessibilityColumns(&self) -> Retained<NSArray> {
                self.collection_elements(|n| {
                    matches!(n.role, AccessibilityRole::TableColumn { .. })
                })
            }
            #[unsafe(method_id(accessibilityVisibleColumns))]
            fn accessibilityVisibleColumns(&self) -> Retained<NSArray> {
                self.collection_elements(|n| {
                    matches!(n.role, AccessibilityRole::TableColumn { .. })
                })
            }
            #[unsafe(method_id(accessibilityColumnHeaderUIElements))]
            fn accessibilityColumnHeaderUIElements(&self) -> Retained<NSArray> {
                self.collection_elements(|n| {
                    matches!(n.role, AccessibilityRole::TableColumnHeader { .. })
                })
            }
            #[unsafe(method_id(accessibilityDisclosedRows))]
            fn accessibilityDisclosedRows(&self) -> Retained<NSArray> {
                self.elements_for_parent(Some(self.ivars().key))
            }
            #[unsafe(method_id(accessibilityDisclosedByRow))]
            fn accessibilityDisclosedByRow(&self) -> Option<Retained<objc2::runtime::AnyObject>> {
                self.disclosed_by_row()
            }

            #[unsafe(method_id(accessibilityMinValue))]
            fn accessibilityMinValue(&self) -> Option<Retained<objc2_foundation::NSNumber>> {
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::Slider(range)) => {
                        Some(objc2_foundation::NSNumber::new_f64(range.minimum()))
                    }
                    _ => None,
                }
            }
            #[unsafe(method_id(accessibilityMaxValue))]
            fn accessibilityMaxValue(&self) -> Option<Retained<objc2_foundation::NSNumber>> {
                match self.node().map(|n| n.role) {
                    Some(AccessibilityRole::Slider(range)) => {
                        Some(objc2_foundation::NSNumber::new_f64(range.maximum()))
                    }
                    _ => None,
                }
            }
            #[unsafe(method(accessibilityPerformIncrement))]
            fn accessibilityPerformIncrement(&self) -> bool {
                self.node()
                    .is_some_and(|n| matches!(n.role, AccessibilityRole::Slider(_)) && !n.read_only)
                    && self.dispatch(AccessibilityAction::Increment)
            }
            #[unsafe(method(accessibilityPerformDecrement))]
            fn accessibilityPerformDecrement(&self) -> bool {
                self.node()
                    .is_some_and(|n| matches!(n.role, AccessibilityRole::Slider(_)) && !n.read_only)
                    && self.dispatch(AccessibilityAction::Decrement)
            }

            #[unsafe(method(accessibilityPerformPress))]
            fn accessibilityPerformPress(&self) -> bool {
                self.node()
                    .is_some_and(|node| Self::is_pressable(node.role))
                    && self.dispatch(AccessibilityAction::Press)
            }

            #[unsafe(method(setAccessibilityValue:))]
            unsafe fn accessibilitySetValue(&self, value: Option<&objc2::runtime::AnyObject>) {
                let Some((_, node)) = self.shared_and_node() else {
                    return;
                };
                if !node.enabled || node.read_only {
                    return;
                }
                if matches!(node.role, AccessibilityRole::Slider(_)) {
                    if let Some(value) = value
                        .and_then(|v| v.downcast_ref::<objc2_foundation::NSNumber>())
                        .map(|v| v.as_f64())
                        .filter(|v| v.is_finite())
                    {
                        self.dispatch(AccessibilityAction::SetValue(value.to_string()));
                    }
                    return;
                }
                if !matches!(
                    node.role,
                    AccessibilityRole::TextField | AccessibilityRole::TextArea
                ) {
                    return;
                }
                if let Some(value) = value.and_then(|value| value.downcast_ref::<NSString>()) {
                    self.dispatch(AccessibilityAction::SetValue(value.to_string()));
                }
            }

            #[unsafe(method(setAccessibilityFocused:))]
            fn accessibilitySetFocused(&self, focused: bool) {
                self.trace_text(&format!("set-focus {focused}"));
                if focused
                    && self
                        .node()
                        .is_some_and(|node| Self::is_focusable(node.role))
                {
                    self.dispatch(AccessibilityAction::Focus);
                }
            }

            #[unsafe(method(isAccessibilitySelectorAllowed:))]
            fn accessibility_is_selector_allowed(
                &self,
                selector: objc2::runtime::Sel,
            ) -> objc2::runtime::Bool {
                let Some(node) = self.node() else {
                    return objc2::runtime::Bool::NO;
                };
                if matches!(
                    node.role,
                    AccessibilityRole::TextField | AccessibilityRole::TextArea
                ) {
                    self.trace_text(&format!(
                        "allowed-query {}",
                        selector.name().to_string_lossy()
                    ));
                }
                if selector == sel!(isAccessibilityRequired) {
                    return objc2::runtime::Bool::YES;
                }
                if selector == sel!(setAccessibilityRequired:) {
                    return objc2::runtime::Bool::NO;
                }
                let disclosure = matches!(node.role, AccessibilityRole::TreeItem { .. });
                let attribute = if selector == sel!(accessibilityLineForIndex:)
                    || selector == sel!(accessibilityRangeForLine:)
                    || selector == sel!(accessibilityInsertionPointLineNumber)
                    || selector == sel!(accessibilityFrameForRange:)
                    || selector == sel!(accessibilityRangeForIndex:)
                    || selector == sel!(accessibilityRangeForPosition:)
                    || selector == sel!(accessibilityVisibleCharacterRange)
                {
                    Some(self.text_layout().is_some())
                } else if selector == sel!(accessibilityAttributedStringForRange:)
                    || selector == sel!(accessibilitySelectedTextRanges)
                {
                    Some(
                        matches!(
                            node.role,
                            AccessibilityRole::TextField | AccessibilityRole::TextArea
                        ) && if selector == sel!(accessibilityAttributedStringForRange:) {
                            node.value.is_some()
                        } else {
                            node.text_selection.is_some()
                        },
                    )
                } else if selector == sel!(isAccessibilityExpanded) {
                    Some(node.expanded.is_some())
                } else if selector == sel!(isAccessibilityDisclosed) {
                    Some(disclosure && node.expanded.is_some())
                } else if selector == sel!(accessibilityDisclosureLevel)
                    || selector == sel!(accessibilityDisclosedByRow)
                    || selector == sel!(accessibilityDisclosedRows)
                {
                    Some(disclosure)
                } else if selector == sel!(accessibilityRows)
                    || selector == sel!(accessibilityVisibleRows)
                {
                    Some(matches!(
                        node.role,
                        AccessibilityRole::Tree
                            | AccessibilityRole::Table
                            | AccessibilityRole::TableColumn { .. }
                    ))
                } else if selector == sel!(accessibilityColumns)
                    || selector == sel!(accessibilityVisibleColumns)
                    || selector == sel!(accessibilityColumnHeaderUIElements)
                    || selector == sel!(accessibilitySelectedRows)
                {
                    Some(matches!(
                        node.role,
                        AccessibilityRole::Tree
                            | AccessibilityRole::Table
                            | AccessibilityRole::TableColumn { .. }
                    ))
                } else if selector == sel!(accessibilityIndex) {
                    Some(matches!(
                        node.role,
                        AccessibilityRole::TableColumn { .. }
                            | AccessibilityRole::TableColumnHeader { .. }
                            | AccessibilityRole::TableRow { .. }
                    ))
                } else if selector == sel!(accessibilitySortDirection) {
                    Some(matches!(
                        node.role,
                        AccessibilityRole::TableColumnHeader { .. }
                    ))
                } else if selector == sel!(accessibilityRowIndexRange)
                    || selector == sel!(accessibilityColumnIndexRange)
                {
                    Some(matches!(node.role, AccessibilityRole::TableCell { .. }))
                } else {
                    None
                };
                if let Some(allowed) = attribute {
                    return objc2::runtime::Bool::new(allowed);
                }
                let mutation = selector == sel!(setAccessibilityExpanded:)
                    || selector == sel!(setAccessibilityDisclosed:)
                    || selector == sel!(accessibilityPerformPress)
                    || selector == sel!(setAccessibilityValue:)
                    || selector == sel!(setAccessibilitySelectedText:)
                    || selector == sel!(setAccessibilitySelectedTextRange:)
                    || selector == sel!(setAccessibilitySelectedTextRanges:)
                    || selector == sel!(setAccessibilityFocused:)
                    || selector == sel!(accessibilityPerformIncrement)
                    || selector == sel!(accessibilityPerformDecrement);
                if !mutation {
                    return unsafe {
                        msg_send![super(self), isAccessibilitySelectorAllowed: selector]
                    };
                }
                let allowed = self.node().is_some_and(|node| {
                    node.enabled
                        && (((selector == sel!(setAccessibilityExpanded:)
                            || selector == sel!(setAccessibilityDisclosed:))
                            && matches!(node.role, AccessibilityRole::TreeItem { .. })
                            && node.expanded.is_some())
                            || (selector == sel!(accessibilityPerformPress)
                                && Self::is_pressable(node.role))
                            || (selector == sel!(setAccessibilityValue:)
                                && matches!(
                                    node.role,
                                    AccessibilityRole::TextField
                                        | AccessibilityRole::TextArea
                                        | AccessibilityRole::Slider(_)
                                )
                                && !node.read_only)
                            || (selector == sel!(setAccessibilitySelectedText:)
                                && matches!(
                                    node.role,
                                    AccessibilityRole::TextField | AccessibilityRole::TextArea
                                )
                                && !node.read_only)
                            || ((selector == sel!(accessibilityPerformIncrement)
                                || selector == sel!(accessibilityPerformDecrement))
                                && matches!(node.role, AccessibilityRole::Slider(_))
                                && !node.read_only)
                            || ((selector == sel!(setAccessibilitySelectedTextRange:)
                                || selector == sel!(setAccessibilitySelectedTextRanges:))
                                && matches!(
                                    node.role,
                                    AccessibilityRole::TextField | AccessibilityRole::TextArea
                                )
                                && node.text_selection.is_some())
                            || (selector == sel!(setAccessibilityFocused:)
                                && Self::is_focusable(node.role)))
                });
                objc2::runtime::Bool::new(allowed)
            }
        }
    );

    impl NativeAccessibilityElement {
        fn native_range(range: Option<std::ops::Range<usize>>) -> NSRange {
            range.map_or(NSRange::new(isize::MAX as usize, 0), |r| {
                NSRange::new(r.start, r.len())
            })
        }

        // Explicit opt-in native diagnostic; no event synthesis or polling.
        fn trace_text(&self, detail: &str) {
            if std::env::var_os("TEKESUI_TRACE_AX").is_some() {
                eprintln!(
                    "TEKESUI_AX component={} part={:?} {detail}",
                    self.ivars().key.component,
                    self.ivars().key.part
                );
            }
        }

        fn is_pressable(role: AccessibilityRole) -> bool {
            matches!(
                role,
                AccessibilityRole::Button
                    | AccessibilityRole::Link
                    | AccessibilityRole::Switch
                    | AccessibilityRole::Checkbox
                    | AccessibilityRole::RadioButton
                    | AccessibilityRole::MenuButton
                    | AccessibilityRole::MenuItem
                    | AccessibilityRole::ListItem
                    | AccessibilityRole::TreeItem { .. }
                    | AccessibilityRole::TableRow { .. }
                    | AccessibilityRole::TableColumnHeader { .. }
            )
        }

        fn is_focusable(role: AccessibilityRole) -> bool {
            matches!(
                role,
                AccessibilityRole::Slider(_)
                    | AccessibilityRole::Button
                    | AccessibilityRole::Link
                    | AccessibilityRole::Switch
                    | AccessibilityRole::Checkbox
                    | AccessibilityRole::RadioButton
                    | AccessibilityRole::TextField
                    | AccessibilityRole::TextArea
                    | AccessibilityRole::MenuButton
                    | AccessibilityRole::MenuItem
                    | AccessibilityRole::ListItem
                    | AccessibilityRole::TreeItem { .. }
                    | AccessibilityRole::TableRow { .. }
                    | AccessibilityRole::TableColumnHeader { .. }
                    | AccessibilityRole::Dialog
            )
        }

        fn shared_and_node(&self) -> Option<(Rc<Shared>, AccessibilityNode)> {
            let shared = self.ivars().shared.upgrade()?;
            if shared.closed.get() {
                return None;
            }
            let current = shared
                .accessibility_elements
                .borrow()
                .get(&self.ivars().key)
                .cloned()?;
            if !std::ptr::eq(self, current.as_ref()) {
                return None;
            }
            let node = shared
                .accessibility_nodes
                .borrow()
                .iter()
                .find(|n| n.key == self.ivars().key)?
                .clone();
            Some((shared, node))
        }

        fn text_layout(&self) -> Option<super::AccessibilityTextLayout> {
            let (shared, node) = self.shared_and_node()?;
            if !matches!(
                node.role,
                AccessibilityRole::TextField | AccessibilityRole::TextArea
            ) {
                return None;
            }
            let layout = shared
                .accessibility_text_layouts
                .borrow()
                .get(&node.key)?
                .clone();
            (node.value.as_deref() == Some(layout.state.text.as_str()) && layout.is_valid())
                .then_some(layout)
        }

        fn node(&self) -> Option<AccessibilityNode> {
            self.shared_and_node().map(|(_, n)| n)
        }

        fn dispatch(&self, action: AccessibilityAction) -> bool {
            let Some((shared, node)) = self.shared_and_node() else {
                return false;
            };
            if shared.closed.get() || !node.enabled {
                return false;
            }
            shared.dispatch(Event::Accessibility {
                target: node.key,
                action,
            });
            true
        }

        fn collection_rows(&self) -> Retained<NSArray> {
            let Some(owner) = self.node() else {
                return NSArray::new();
            };
            let column = match owner.role {
                AccessibilityRole::TableColumn { index } => Some(index),
                _ => None,
            };
            self.collection_elements(|n| {
                if let Some(col) = column {
                    matches!(n.role, AccessibilityRole::TableCell { column, .. } if column == col)
                } else {
                    matches!(
                        n.role,
                        AccessibilityRole::TreeItem { .. } | AccessibilityRole::TableRow { .. }
                    )
                }
            })
        }
        fn disclosed_by_row(&self) -> Option<Retained<objc2::runtime::AnyObject>> {
            let (shared, node) = self.shared_and_node()?;
            let parent = node.parent?;
            if !shared
                .accessibility_nodes
                .borrow()
                .iter()
                .any(|n| n.key == parent && matches!(n.role, AccessibilityRole::TreeItem { .. }))
            {
                return None;
            }
            shared
                .accessibility_elements
                .borrow()
                .get(&parent)
                .cloned()
                .map(Into::into)
        }

        // All table/outline relationships use the currently accepted snapshot
        // and cached native objects, retaining the existing stale-object fence.
        fn collection_elements(
            &self,
            predicate: impl Fn(&AccessibilityNode) -> bool,
        ) -> Retained<NSArray> {
            let Some((shared, owner)) = self.shared_and_node() else {
                return NSArray::new();
            };
            let nodes = shared.accessibility_nodes.borrow();
            let root = |mut key: AccessibilityKey| {
                for _ in 0..nodes.len() {
                    let n = nodes.iter().find(|n| n.key == key)?;
                    if matches!(n.role, AccessibilityRole::Table | AccessibilityRole::Tree) {
                        return Some(key);
                    }
                    key = n.parent?;
                }
                None
            };
            let Some(owner_root) = root(owner.key) else {
                return NSArray::new();
            };
            let cache = shared.accessibility_elements.borrow();
            let elements: Vec<Retained<objc2::runtime::AnyObject>> = nodes
                .iter()
                .filter(|n| predicate(n) && root(n.key) == Some(owner_root))
                .filter_map(|n| cache.get(&n.key).cloned().map(Into::into))
                .collect();
            NSArray::from_retained_slice(&elements)
        }

        fn elements_for_parent(&self, parent: Option<AccessibilityKey>) -> Retained<NSArray> {
            let Some((shared, _)) = self.shared_and_node() else {
                return NSArray::new();
            };
            elements_for_parent(&shared, parent)
        }

        pub(crate) fn new(
            mtm: MainThreadMarker,
            shared: &Rc<Shared>,
            key: AccessibilityKey,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(ElementIvars {
                shared: Rc::downgrade(shared),
                key,
            });
            // SAFETY: NSAccessibilityElement inherits NSObject's init initializer.
            unsafe { msg_send![super(this), init] }
        }
    }

    pub(crate) fn elements_for_parent(
        shared: &Rc<Shared>,
        parent: Option<AccessibilityKey>,
    ) -> Retained<NSArray> {
        if shared.closed.get() {
            return NSArray::new();
        }
        let cache = shared.accessibility_elements.borrow();
        let keys: Vec<_> = shared
            .accessibility_nodes
            .borrow()
            .iter()
            .filter(|n| n.parent == parent)
            .map(|n| n.key)
            .collect();
        let elements: Vec<Retained<objc2::runtime::AnyObject>> = keys
            .iter()
            .filter_map(|k| cache.get(k).cloned().map(Into::into))
            .collect();
        NSArray::from_retained_slice(&elements)
    }

    pub(crate) use NativeAccessibilityElement as Element;
}

#[cfg(target_os = "macos")]
pub(crate) use native::Element as NativeAccessibilityElement;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Point, Size};
    use crate::placement::Rect;

    #[test]
    fn text_selection_snapshots_reject_split_surrogates_and_non_text_owners() {
        for role in [AccessibilityRole::TextField, AccessibilityRole::TextArea] {
            let mut n = node(1, None, None);
            n.role = role;
            n.value = Some("A😀中".into());
            n.text_selection = Some(1..3);
            assert!(validate_accessibility_nodes(&[n.clone()]));
            assert_eq!(text_in_range("A😀中", 1..3).as_deref(), Some("😀"));
            assert_eq!(text_in_range("A😀中", 1..2), None);
            assert_eq!(text_in_range("A😀中", 2..2), None);
            n.text_selection = Some(2..3);
            assert!(!validate_accessibility_nodes(&[n.clone()]));
            n.text_selection = Some(1..3);
            n.role = AccessibilityRole::Button;
            assert!(!validate_accessibility_nodes(&[n.clone()]));
            n.role = role;
            n.text_selection = Some(0..usize::MAX);
            assert!(!validate_accessibility_nodes(&[n]));
        }
    }

    #[test]
    fn slider_domains_and_numeric_snapshot_values_are_finite_and_bounded() {
        for (min, max) in [(f64::NAN, 100.), (0., f64::INFINITY), (10., 0.)] {
            assert!(AccessibilityRange::new(min, max).is_none());
        }
        let domain = AccessibilityRange::new(0., 100.).unwrap();
        assert_eq!(domain, domain);
        let mut n = node(1, None, None);
        n.role = AccessibilityRole::Slider(domain);
        for value in ["0", "50", "100"] {
            n.value = Some(value.into());
            assert!(validate_accessibility_nodes(&[n.clone()]));
        }
        for value in ["NaN", "inf", "-1", "101", "not numeric"] {
            n.value = Some(value.into());
            assert!(!validate_accessibility_nodes(&[n.clone()]));
        }
        n.value = None;
        assert!(!validate_accessibility_nodes(&[n]));
        assert!(AccessibilityRange::new(5., 5.).is_some());
    }

    fn node(
        component: u64,
        part: Option<usize>,
        parent: Option<AccessibilityKey>,
    ) -> AccessibilityNode {
        AccessibilityNode {
            key: AccessibilityKey { component, part },
            parent,
            role: AccessibilityRole::Group,
            frame: Rect {
                origin: Point::default(),
                size: Size {
                    width: 10.0,
                    height: 10.0,
                },
            },
            label: String::new(),
            value: None,
            description: None,
            enabled: true,
            focused: false,
            selected: false,
            expanded: None,
            invalid: false,
            required: false,
            text_selection: None,
            read_only: false,
        }
    }

    fn shaped_text() -> AccessibilityTextLayout {
        use crate::event::{Point, Size, TextInputState};
        AccessibilityTextLayout {
            state: TextInputState {
                text: "A😀中\nZ".into(),
                caret_index: 5,
                selection: 5..5,
                carets: vec![
                    (0, 10., 20.),
                    (1, 20., 20.),
                    (3, 40., 20.),
                    (3, 10., 40.),
                    (4, 20., 40.),
                    (5, 20., 40.),
                    (5, 10., 60.),
                    (6, 20., 60.),
                ]
                .into_iter()
                .map(|(i, x, y)| {
                    (
                        i,
                        Point { x, y },
                        Size {
                            width: 1.,
                            height: 20.,
                        },
                    )
                })
                .collect(),
                ..Default::default()
            },
            lines: vec![0..3, 3..5, 5..6],
        }
    }

    #[test]
    fn shaped_ax_lines_include_soft_wrap_and_newline_in_utf16() {
        let l = shaped_text();
        assert!(l.is_valid());
        assert_eq!(l.line_for_index(0), Some(0));
        assert_eq!(l.line_for_index(2), None, "inside surrogate pair");
        assert_eq!(l.line_for_index(3), Some(1), "downstream soft wrap");
        assert_eq!(l.range_for_line(1), Some(3..5));
        assert_eq!(l.line_for_index(6), Some(2));
        assert_eq!(l.line_for_index(7), None);
        assert_eq!(l.range_for_line(3), None);
    }

    #[test]
    fn shaped_ax_geometry_uses_downstream_line_and_multiline_bounds() {
        let l = shaped_text();
        let r = l.frame_for_range(3..6).unwrap();
        assert_eq!(r.origin, crate::event::Point { x: 10., y: 40. });
        assert_eq!(r.size.width, 10.);
        assert_eq!(r.size.height, 40.);
        assert_eq!(l.frame_for_range(3..3).unwrap().origin.y, 40.);
        assert!(l.frame_for_range(2..3).is_none());
        assert!(l.frame_for_range(3..7).is_none());
    }

    #[test]
    fn shaped_ax_rejects_partial_or_malformed_layouts() {
        let mut l = shaped_text();
        l.lines.remove(0);
        assert!(!l.is_valid());
        l = shaped_text();
        l.lines[1] = 2..5;
        assert!(!l.is_valid());
        l = shaped_text();
        l.state.carets[0].1.x = f64::NAN;
        assert!(!l.is_valid());
        l = shaped_text();
        l.lines.clear();
        assert!(!l.is_valid());
    }

    #[test]
    fn ax_character_queries_reuse_shaped_graphemes_including_surrogate_indices() {
        let layout = shaped_text();
        assert_eq!(layout.range_for_index(2), Some(1..3));
        assert_eq!(layout.range_for_index(3), Some(3..4));
        assert_eq!(layout.range_for_index(6), Some(6..6));
        assert_eq!(layout.range_for_index(7), None);
        let mut partial = layout;
        partial.state.carets.clear();
        assert_eq!(partial.range_for_index(0), None);
    }

    #[test]
    fn ax_visible_range_uses_actual_lines_not_the_focused_caret_or_horizontal_clip() {
        use crate::{Point, Rect, Size};
        let layout = shaped_text();
        let viewport = Rect {
            origin: Point { x: 999., y: 40. },
            size: Size {
                width: 1.,
                height: 20.,
            },
        };
        assert_eq!(layout.visible_range(viewport), Some(3..5));
        assert_eq!(
            layout.visible_range(Rect {
                origin: Point { x: 0., y: 39. },
                ..viewport
            }),
            Some(0..5)
        );
        assert_eq!(
            layout.visible_range(Rect {
                origin: Point { x: 0., y: 80. },
                ..viewport
            }),
            None
        );
    }

    #[test]
    fn ax_point_queries_use_real_visual_intervals_and_reject_absent_glyphs() {
        use crate::Point;
        let layout = shaped_text();
        assert_eq!(
            layout.range_for_position(Point { x: 25., y: 25. }),
            Some(1..3)
        );
        assert_eq!(
            layout.range_for_position(Point { x: 15., y: 45. }),
            Some(3..4)
        );
        assert_eq!(layout.range_for_position(Point { x: 100., y: 45. }), None);
        assert_eq!(
            layout.range_for_position(Point {
                x: f64::NAN,
                y: 45.
            }),
            None
        );
        let mut rtl = layout;
        for caret in &mut rtl.state.carets {
            caret.1.x = 100. - caret.1.x;
        }
        assert_eq!(rtl.range_for_position(Point { x: 75., y: 25. }), Some(1..3));
    }

    #[test]
    fn shaped_ax_empty_and_trailing_empty_lines_are_addressable() {
        let mut l = shaped_text();
        l.lines.push(6..6);
        assert_eq!(l.line_for_index(6), Some(3));
        l.state.text.clear();
        l.state.carets.clear();
        l.lines.clear();
        l.lines.push(0..0);
        assert!(l.is_valid());
        assert_eq!(l.line_for_index(0), Some(0));
        assert_eq!(l.range_for_line(0), Some(0..0));
        assert!(l.frame_for_range(0..0).is_none(), "no invented geometry");
    }

    #[test]
    fn snapshot_validator_accepts_forest_and_rejects_bad_links() {
        let root = AccessibilityKey {
            component: 1,
            part: None,
        };
        assert!(validate_accessibility_nodes(&[
            node(1, None, None),
            node(2, Some(0), Some(root))
        ]));
        assert!(!validate_accessibility_nodes(&[
            node(1, None, None),
            node(1, None, None)
        ]));
        assert!(!validate_accessibility_nodes(&[node(
            1,
            None,
            Some(AccessibilityKey {
                component: 9,
                part: None
            })
        )]));
    }

    #[test]
    fn snapshot_validator_rejects_cycles_and_invalid_frames() {
        let a = AccessibilityKey {
            component: 1,
            part: None,
        };
        let b = AccessibilityKey {
            component: 2,
            part: None,
        };
        assert!(!validate_accessibility_nodes(&[
            node(1, None, Some(b)),
            node(2, None, Some(a))
        ]));
        let mut bad_frame = node(1, None, None);
        bad_frame.frame.size.width = f64::INFINITY;
        assert!(!validate_accessibility_nodes(&[bad_frame]));
        let mut zero_frame = node(1, None, None);
        zero_frame.frame.size.height = 0.0;
        assert!(!validate_accessibility_nodes(&[zero_frame]));
    }
}
