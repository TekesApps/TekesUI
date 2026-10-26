//! Editable numeric Content. TextFieldContent owns all text editing and IME;
//! NumberField owns bounded arithmetic; ordinary ButtonContent owns activation.
use super::*;
use crate::{NumberField, TextSurface, TextSurfaceAlignment, TextSurfacePresentation};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum NumberParseError {
    Incomplete,
    Invalid,
}
/// A paired codec. Every formatted finite value must parse back to that value.
/// Parsing never strips arbitrary punctuation or changes numeric units implicitly.
pub trait NumberFormat: Debug {
    fn format(&self, value: f64) -> String;
    fn parse(&self, text: &str) -> Result<f64, NumberParseError>;
    /// Format a sizing-only digit run, preserving the normal affixes/precision.
    /// Override for formats whose zero has no ASCII `0`, or nontrivial numeric
    /// fields. This sample is never inserted into the editor or parsed as input.
    fn width_sample(&self, digits: &str) -> String {
        self.format(0.).replacen('0', digits, 1)
    }
}
/// Shortest round-tripping decimal, with an optional consumer-chosen suffix.
/// Input accepts ASCII decimal digits, a decimal point, an optional leading sign,
/// and the declared suffix. Exponents, localized digits and words are not parsed.
/// The suffix is optional on input. A percentage uses suffix "%" and values
/// in percentage points; the codec performs no multiplication or division.
#[derive(Clone, Debug, Default)]
pub struct DecimalNumberFormat {
    pub suffix: String,
}
impl NumberFormat for DecimalNumberFormat {
    fn width_sample(&self, digits: &str) -> String {
        format!("{digits}{}", self.suffix)
    }
    fn format(&self, value: f64) -> String {
        format!("{value}{}", self.suffix)
    }
    fn parse(&self, text: &str) -> Result<f64, NumberParseError> {
        let text = text.trim();
        let text = if self.suffix.is_empty() {
            text
        } else {
            text.strip_suffix(&self.suffix).unwrap_or(text).trim()
        };
        if text.is_empty() || matches!(text, "+" | "-" | "." | "+." | "-.") {
            return Err(NumberParseError::Incomplete);
        }
        if !text.bytes().enumerate().all(|(i, byte)| {
            byte.is_ascii_digit() || byte == b'.' || (i == 0 && matches!(byte, b'+' | b'-'))
        }) {
            return Err(NumberParseError::Invalid);
        }
        text.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(NumberParseError::Invalid)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum NumberValidation {
    Valid(f64),
    Incomplete,
    Invalid,
    OutOfRange,
    /// Consumer codec cannot represent this value without changing it.
    FormatMismatch,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberValueError {
    Editing,
    InvalidValue,
    FormatMismatch,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberCommitReason {
    Submit,
    Blur,
    Step,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NumberValueChange {
    pub previous: f64,
    pub value: f64,
    pub reason: NumberCommitReason,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NumberEditState {
    pub text: String,
    pub focused: bool,
    pub composing: bool,
    pub dirty: bool,
    pub validation: NumberValidation,
    pub committed_value: f64,
}
/// One keyboard stop: Tab/Backtab navigate; Up/Down step; Enter commits;
/// Escape restores the last committed value. Left/Right retain text navigation.
/// Invalid/incomplete drafts survive submit, blur and step. During IME no numeric
/// commit, cancellation or step occurs (Escape is delegated to the text editor).
#[derive(Debug)]
pub struct NumberFieldContent {
    editor: TextFieldContent,
    name: String,
    input_digits: Option<usize>,
    number: NumberField,
    min: f64,
    max: f64,
    format: Box<dyn NumberFormat>,
    buttons: [ButtonContent; 2],
    frames: [Rect; 2],
    editor_frame: Rect,
    captured: Option<usize>,
    editor_captured: bool,
    focused: bool,
    changes: VecDeque<NumberValueChange>,
    pub description: Option<String>,
    /// Accessible names and hover hints for the two actions.
    pub decrement_label: String,
    pub increment_label: String,
}
impl NumberFieldContent {
    /// Rejects invalid configuration and out-of-range initial values; never clamps input.
    pub fn new(
        name: impl Into<String>,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        format: impl NumberFormat + 'static,
    ) -> Option<Self> {
        let number = NumberField::new(value, min, max, step)?;
        if value < min || value > max || format.parse(&format.format(value)) != Ok(value) {
            return None;
        }
        let name = name.into();
        let mut editor = TextFieldContent::new("", &format.format(value));
        editor.submit_policy = TextSubmitPolicy::Enter;
        editor.show_metadata = false;

        let mut buttons = [ButtonContent::new("−"), ButtonContent::new("+")];
        for button in &mut buttons {
            button.button.frame_policy = crate::ButtonFramePolicy::Exact;
        }
        Some(Self {
            editor,
            name,
            input_digits: None,
            number,
            min,
            max,
            format: Box::new(format),
            buttons,
            frames: [Rect::default(); 2],
            editor_frame: Rect::default(),
            captured: None,
            editor_captured: false,
            focused: false,
            changes: VecDeque::new(),
            description: None,
            decrement_label: "Decrease".into(),
            increment_label: "Increase".into(),
        })
    }
    /// Reserve a fixed input-box width for this many digits plus format symbols.
    /// Accepts 1..=1024; invalid requests leave the previous setting unchanged.
    /// The width follows the active font/scale, never the current draft length.
    /// This is layout capacity, not a character limit or a numeric constraint.
    pub fn set_input_digits(&mut self, digits: usize) -> bool {
        if !(1..=1024).contains(&digits) {
            return false;
        }
        self.input_digits = Some(digits);
        true
    }
    pub fn input_digits(&self) -> Option<usize> {
        self.input_digits
    }
    /// Restore the default width that follows the current editor content.
    pub fn clear_input_digits(&mut self) {
        self.input_digits = None;
    }
    fn input_width(&self, text: &mut dyn ButtonTextRenderer) -> f32 {
        let role = self.editor.surface.role;
        let mut width = 0_f32;
        // Measure the formatter with every digit, rather than assuming that a
        // particular digit or a fixed point size is widest in the active font.
        if let Some(digits) = self.input_digits {
            for digit in '0'..='9' {
                let run = digit.to_string().repeat(digits);
                let sample = self.format.width_sample(&run);
                width = width.max(text.measure(&sample, role, f32::MAX).width);
                if self.min < 0. {
                    let sample = self.format.width_sample(&format!("-{run}"));
                    width = width.max(text.measure(&sample, role, f32::MAX).width);
                }
            }
        } else {
            let minimum = self.format.width_sample("0");
            width = text.measure(&minimum, role, f32::MAX).width.max(
                text.measure(&self.editor.surface.input.text, role, f32::MAX)
                    .width,
            );
        }
        let padding = if self.editor.surface.presentation == TextSurfacePresentation::Field {
            2. * proposal::SCROLL_CONTENT_PADDING
        } else {
            0.
        };
        let width = width + padding + tekes_ui_tokens::Stroke::Rule.value();
        let scale = text.backing_scale().max(1.);
        (width * scale).ceil() / scale
    }
    pub fn set_alignment(&mut self, alignment: TextSurfaceAlignment) {
        self.editor.surface.alignment = alignment;
    }
    /// Uses TextSurface's existing field chrome, or its undecorated Content presentation.
    pub fn set_borderless(&mut self, borderless: bool) {
        self.editor.surface.presentation = if borderless {
            TextSurfacePresentation::Content
        } else {
            TextSurfacePresentation::Field
        };
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        self.editor.surface.input.disabled = disabled;
        if disabled {
            self.captured = None;
            self.editor_captured = false;
        }
        self.sync_buttons();
    }
    pub fn value(&self) -> f64 {
        self.number.value()
    }
    /// Apply consumer-owned state (including configuration Undo/Redo).
    /// Refuses to overwrite a dirty draft or marked text. Call cancel_edit first
    /// when discarding a non-IME draft is an explicit consumer decision.
    /// This synchronization creates no numeric change notification.
    pub fn set_value(&mut self, value: f64) -> Result<bool, NumberValueError> {
        if !value.is_finite() || value < self.min || value > self.max {
            return Err(NumberValueError::InvalidValue);
        }
        if self.format.parse(&self.format.format(value)) != Ok(value) {
            return Err(NumberValueError::FormatMismatch);
        }
        if value == self.value() {
            return Ok(false);
        }
        if self.composing() || self.edit_state().dirty {
            return Err(NumberValueError::Editing);
        }
        let changed = self.number.set(value);
        self.replace_display();
        self.sync_buttons();
        Ok(changed)
    }
    pub fn validation(&self) -> NumberValidation {
        match self.format.parse(&self.editor.surface.input.text) {
            Err(NumberParseError::Incomplete) => NumberValidation::Incomplete,
            Err(NumberParseError::Invalid) => NumberValidation::Invalid,
            Ok(v) if !v.is_finite() => NumberValidation::Invalid,
            Ok(v) if v < self.min || v > self.max => NumberValidation::OutOfRange,
            Ok(v) if self.format.parse(&self.format.format(v)) != Ok(v) => {
                NumberValidation::FormatMismatch
            }
            Ok(v) => NumberValidation::Valid(v),
        }
    }
    pub fn edit_state(&self) -> NumberEditState {
        NumberEditState {
            text: self.editor.surface.input.text.clone(),
            focused: self.focused,
            composing: self.composing(),
            dirty: self.editor.surface.input.text != self.format.format(self.value()),
            validation: self.validation(),
            committed_value: self.value(),
        }
    }
    /// Includes blur commits, which cannot be represented by activation alone.
    /// Drain after every host delivery/focus change; only changed committed values enter this queue.
    pub fn drain_value_changes(&mut self) -> impl Iterator<Item = NumberValueChange> + '_ {
        self.changes.drain(..)
    }
    fn replace_display(&mut self) {
        let display = self.format.format(self.value());
        if display != self.editor.surface.input.text {
            let len = self.editor.surface.input.len();
            self.editor.surface.input.insert(&display, Some(0..len));
            self.editor.surface.reveal();
        }
    }
    fn publish(&mut self, value: f64, reason: NumberCommitReason) -> bool {
        let previous = self.value();
        let changed = self.number.set(value);
        self.replace_display();
        if changed {
            self.changes.push_back(NumberValueChange {
                previous,
                value: self.value(),
                reason,
            });
        }
        self.sync_buttons();
        changed
    }
    pub fn commit(&mut self, reason: NumberCommitReason) -> bool {
        if self.composing() || !self.focusable() {
            return false;
        }
        let NumberValidation::Valid(value) = self.validation() else {
            return false;
        };
        self.publish(value, reason)
    }
    pub fn cancel_edit(&mut self) -> bool {
        if self.composing() || !self.focusable() {
            return false;
        }
        let changed = self.edit_state().dirty;
        self.replace_display();
        self.sync_buttons();
        changed
    }
    pub fn can_step(&self, increment: bool) -> bool {
        if self.composing() || !self.focusable() {
            return false;
        }
        let NumberValidation::Valid(value) = self.validation() else {
            return false;
        };
        let mut candidate = self.number;
        candidate.set(value);
        if !candidate.can_increment(increment) {
            return false;
        }
        if !candidate.increment(increment) {
            return false;
        }
        self.format.parse(&self.format.format(candidate.value())) == Ok(candidate.value())
    }
    pub fn step(&mut self, increment: bool) -> bool {
        if !self.can_step(increment) {
            return false;
        }
        let NumberValidation::Valid(value) = self.validation() else {
            return false;
        };
        let mut candidate = self.number;
        candidate.set(value);
        candidate.increment(increment);
        self.publish(candidate.value(), NumberCommitReason::Step)
    }
    fn sync_buttons(&mut self) {
        for i in 0..2 {
            let disabled = !self.can_step(i == 1);
            self.buttons[i].button.state.disabled = disabled;
            if disabled {
                self.buttons[i].button.cancel();
            }
        }
        self.editor.surface.error = !matches!(self.validation(), NumberValidation::Valid(_));
    }
    fn finish_edit_event(&mut self, mut result: ContentResponse) -> ContentResponse {
        if result.activated {
            result.activated = self.commit(NumberCommitReason::Submit);
            result.changed = true;
        }
        self.sync_buttons();
        result
    }
}
impl Component for NumberFieldContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability { CurveSubtreeCapability::Local }

    fn requires_text_services(&self) -> bool {
        true
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.editor.text_surface()
    }
    fn composing(&self) -> bool {
        self.editor.composing()
    }
    fn focusable(&self) -> bool {
        self.editor.focusable()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let was_composing = self.composing();
        if self.focused && !focused && !was_composing {
            self.commit(NumberCommitReason::Blur);
        }
        self.editor.set_focus(focused, visible);
        self.focused = focused;
        if !focused {
            self.captured = None;
            self.editor_captured = false;
            for b in &mut self.buttons {
                b.button.cancel();
            }
        }
        self.sync_buttons();
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.editor.set_focus(false, false);
            self.focused = false;
            self.captured = None;
            self.editor_captured = false;
            for b in &mut self.buttons {
                b.button.cancel();
            }
        }
    }
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        Some(self.min..=self.max)
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = self.editor.semantics();
        s.invalid = !matches!(self.validation(), NumberValidation::Valid(_));
        s.name = self.name.clone();
        s.description = Some(format!(
            "{}{}Range {} to {}",
            self.description.as_deref().unwrap_or(""),
            if self.description.is_some() { ". " } else { "" },
            self.format.format(self.min),
            self.format.format(self.max)
        ));
        s
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.editor.surface.outer
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        (0..2)
            .map(|i| {
                let mut semantics = ContentSemantics::new(
                    ContentRole::Button,
                    if i == 0 {
                        &self.decrement_label
                    } else {
                        &self.increment_label
                    },
                );
                semantics.disabled = !self.can_step(i == 1);
                SemanticPart {
                    index: i,
                    frame: self.frames[i],
                    focused: false,
                    semantics,
                }
            })
            .collect()
    }
    fn semantic_part_help(&self, part: usize) -> Option<(&str, u64)> {
        match part {
            0 => Some((&self.decrement_label, 0)),
            1 => Some((&self.increment_label, 0)),
            _ => None,
        }
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let direction = match (part, action) {
            (Some(0), AccessibilityAction::Press) | (None, AccessibilityAction::Decrement) => {
                Some(false)
            }
            (Some(1), AccessibilityAction::Press) | (None, AccessibilityAction::Increment) => {
                Some(true)
            }
            _ => None,
        };
        if let Some(up) = direction {
            let changed = self.step(up);
            return ContentResponse {
                activated: changed,
                ..ContentResponse::consumed(changed)
            };
        }
        let r = self.editor.accessibility_action(part, action);
        self.finish_edit_event(r)
    }
    fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let minus = self.buttons[0].measure(t, a).size;
        let plus = self.buttons[1].measure(t, a).size;
        let field = self
            .editor
            .measure(
                t,
                Size::new((a.width - minus.width - plus.width).max(0.), a.height),
            )
            .size;
        MeasuredChild::new(Size::new(
            (self.input_width(t) + minus.width + plus.width).min(a.width),
            field
                .height
                .max(minus.height)
                .max(plus.height)
                .max(proposal::metric("size", "size-button-md").expect("JSON input minimum")),
        ))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        let mut f = f;
        let width = self.input_width(t);
        let minus = self.buttons[0].measure(t, f.size).size.width;
        let plus = self.buttons[1].measure(t, f.size).size.width;
        f.size.width = (width + minus + plus).min(f.size.width);
        let w0 = self.buttons[0]
            .measure(t, f.size)
            .size
            .width
            .min(f.size.width / 3.);
        let w1 = self.buttons[1]
            .measure(t, f.size)
            .size
            .width
            .min(f.size.width / 3.);
        self.frames = [
            Rect::new(f.min_x(), f.min_y(), w0, f.size.height),
            Rect::new(f.max_x() - w1, f.min_y(), w1, f.size.height),
        ];
        self.editor_frame = Rect::new(
            f.min_x() + w0,
            f.min_y(),
            (f.size.width - w0 - w1).max(0.),
            f.size.height,
        );
        let height = self.editor.measure(t, self.editor_frame.size).size.height;
        self.editor_frame.origin.y += (f.size.height - height).max(0.) * 0.5;
        self.editor_frame.size.height = height;
        self.editor.arrange(t, self.editor_frame, c);
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, _: Rect, c: Rect) {
        self.editor.prepare_text_input(t, self.editor_frame, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        _: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.sync_buttons();
        self.editor.paint(s, t, self.editor_frame, c, theme);
        for i in 0..2 {
            self.buttons[i].paint(s, t, self.frames[i], c, theme);
        }
    }
    fn pointer_regions(&self, _: Rect, clip: Rect) -> Vec<Rect> {
        self.frames
            .iter()
            .copied()
            .chain(std::iter::once(self.editor.surface.outer))
            .filter_map(|frame| frame.intersection(&clip))
            .filter(|frame| !frame.is_empty())
            .collect()
    }
    fn pointer_event(&mut self, p: &PointerEvent) -> ContentResponse {
        if !p.is_primary() {
            return ContentResponse::default();
        }
        self.sync_buttons();
        let event = p.routing_event();
        let position = match event {
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerMove(p)
            | ContentEvent::PointerUp(p) => p,
            _ => return ContentResponse::default(),
        };
        if matches!(event, ContentEvent::PointerMove(_)) && self.captured.is_none() {
            for button in &mut self.buttons {
                button.pointer_event(p);
            }
        }
        if self.editor_captured {
            let r = self.editor.pointer_event(p);
            if matches!(event, ContentEvent::PointerUp(_)) {
                self.editor_captured = false;
            }
            return self.finish_edit_event(r);
        }
        let hit = self
            .captured
            .or_else(|| self.frames.iter().position(|f| f.contains(position)));
        if let Some(i) = hit {
            let mut r = self.buttons[i].pointer_event(p);
            if r.capture_pointer {
                self.captured = Some(i);
            }
            if matches!(event, ContentEvent::PointerUp(_)) {
                self.captured = None;
                self.editor_captured = false;
            }
            if r.activated {
                r.activated = self.step(i == 1);
            }
            return r;
        }
        let r = self.editor.pointer_event(p);
        self.editor_captured = r.capture_pointer;
        self.finish_edit_event(r)
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if key.down
            && !key.modifiers.command
            && !key.modifiers.alt
            && !key.modifiers.control
            && !key.modifiers.shift
        {
            if matches!(key.key, Some(ControlKey::Up | ControlKey::Down)) {
                let changed = self.step(key.key == Some(ControlKey::Up));
                return ContentResponse {
                    activated: changed,
                    ..ContentResponse::consumed(changed)
                };
            }
            if key.key == Some(ControlKey::Escape) && !self.composing() {
                return ContentResponse::consumed(self.cancel_edit());
            }
        }
        let r = self.editor.key_event(key);
        self.finish_edit_event(r)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        match e {
            ContentEvent::Pointer(p) => return self.pointer_event(p),
            ContentEvent::PointerDown(position)
            | ContentEvent::PointerUp(position)
            | ContentEvent::PointerMove(position) => {
                return self.pointer_event(&PointerEvent {
                    position: *position,
                    phase: match e {
                        ContentEvent::PointerDown(_) => PointerPhase::Down,
                        ContentEvent::PointerUp(_) => PointerPhase::Up,
                        _ => PointerPhase::Move,
                    },
                    button: Some(PointerButton::Primary),
                    modifiers: InputModifiers::default(),
                    click_count: 1,
                });
            }
            ContentEvent::Key { key, down } => {
                return self.key_event(&ContentKeyEvent {
                    key: Some(*key),
                    down: *down,
                    repeat: false,
                    modifiers: InputModifiers::default(),
                    native_code: 0,
                    characters: String::new(),
                    characters_ignoring_modifiers: String::new(),
                });
            }
            ContentEvent::TextCommand(command) if !self.composing() => match command.as_str() {
                "moveUp:" | "moveDown:" => {
                    let changed = self.step(command == "moveUp:");
                    return ContentResponse {
                        activated: changed,
                        ..ContentResponse::consumed(changed)
                    };
                }
                "insertNewline:" | "insertLineBreak:" => {
                    let changed = self.commit(NumberCommitReason::Submit);
                    return ContentResponse {
                        activated: changed,
                        ..ContentResponse::consumed(true)
                    };
                }
                "cancelOperation:" => return ContentResponse::consumed(self.cancel_edit()),
                _ => {}
            },
            ContentEvent::Cancel => {
                self.captured = None;
                self.editor_captured = false;
                for b in &mut self.buttons {
                    b.button.cancel();
                }
            }
            _ => {}
        }
        let r = self.editor.event(e);
        self.finish_edit_event(r)
    }
}
