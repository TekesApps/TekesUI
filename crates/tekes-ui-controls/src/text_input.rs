mod external;
pub use external::{
    ExternalSelection, ExternalUndo, ExternalValueConflict, ExternalValueResult, TextEditBaseline,
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    text: String,
    anchor: usize,
    caret: usize,
}
/// Selection and replacement ranges use UTF-16, matching native text services.
/// Edits and arrow/delete movement preserve extended grapheme clusters.
#[derive(Debug)]
pub struct TextInput {
    pub text: String,
    pub anchor: usize,
    pub caret: usize,
    pub marked: Option<Range<usize>>,
    pub multiline: bool,
    pub read_only: bool,
    pub disabled: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    composition: Option<Snapshot>,
    editor_id: u64,
    edit_epoch: u64,
    binding_epoch: u64,
    external: Option<crate::DraftRevision>,
}
impl TextInput {
    pub fn new(text: &str, multiline: bool) -> Self {
        Self {
            text: text.into(),
            anchor: 0,
            caret: 0,
            marked: None,
            multiline,
            read_only: false,
            disabled: false,
            undo: Vec::new(),
            redo: Vec::new(),
            composition: None,
            editor_id: external::next_editor(),
            edit_epoch: 0,
            binding_epoch: 0,
            external: None,
        }
    }
    pub fn len(&self) -> usize {
        self.text.encode_utf16().count()
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }
    fn advance_edit_epoch(&mut self) {
        self.edit_epoch = self
            .edit_epoch
            .checked_add(1)
            .expect("editor mutation epoch exhausted");
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            anchor: self.anchor,
            caret: self.caret,
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.advance_edit_epoch();
        self.text = s.text;
        self.anchor = s.anchor;
        self.caret = s.caret;
        self.marked = None;
        self.composition = None;
    }
    fn remember(&mut self, s: Snapshot) {
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(s);
        self.redo.clear();
    }
    pub fn boundaries(&self) -> Vec<usize> {
        Self::boundaries_for(&self.text)
    }
    pub(crate) fn boundaries_for(text: &str) -> Vec<usize> {
        let mut n = 0;
        let mut out = vec![0];
        for g in text.graphemes(true) {
            n += g.encode_utf16().count();
            out.push(n);
        }
        out
    }
    /// History document replacement is not an editor undo transaction. Only
    /// ReadingController's checked, read-only path calls this helper.
    pub(crate) fn clear_reading_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
    fn boundary(&self, index: usize) -> usize {
        self.boundaries()
            .into_iter()
            .take_while(|n| *n <= index)
            .last()
            .unwrap_or(0)
    }
    fn byte(&self, index: usize) -> usize {
        let mut n = 0;
        for (b, c) in self.text.char_indices() {
            if n >= index {
                return b;
            }
            n += c.len_utf16();
        }
        self.text.len()
    }
    fn normalized(&self, range: Range<usize>) -> Range<usize> {
        let start = self.boundary(range.start);
        let end = if range.is_empty() {
            start
        } else {
            self.boundaries()
                .into_iter()
                .find(|n| *n >= range.end)
                .unwrap_or(self.len())
        };
        start..end.max(start)
    }
    pub fn select(&mut self, index: usize, extend: bool) {
        self.advance_edit_epoch();
        self.caret = self.boundary(index.min(self.len()));
        if !extend {
            self.anchor = self.caret;
        }
    }
    pub(crate) fn edit_revision(&self) -> (u64, u64) {
        (self.editor_id, self.edit_epoch)
    }
    pub fn select_all(&mut self) {
        self.advance_edit_epoch();
        self.anchor = 0;
        self.caret = self.len();
    }
    /// Native/assistive selection uses UTF-16 and the same grapheme boundaries
    /// as pointer and keyboard selection. Composition keeps exclusive ownership.
    /// Returns whether the request was accepted, even if selection is unchanged.
    pub fn select_range(&mut self, range: Range<usize>) -> bool {
        if self.disabled
            || self.marked.is_some()
            || range.start > range.end
            || range.end > self.len()
        {
            return false;
        }
        self.advance_edit_epoch();
        let range = self.normalized(range);
        self.anchor = range.start;
        self.caret = range.end;
        true
    }
    pub fn selected_text(&self) -> String {
        let r = self.selection();
        self.text[self.byte(r.start)..self.byte(r.end)].into()
    }
    fn normalize_value(&self, value: &str) -> String {
        value
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|c| !c.is_control() || (self.multiline && (*c == '\n' || *c == '\t')))
            .collect()
    }
    fn replace_raw(&mut self, range: Range<usize>, value: &str) -> Range<usize> {
        let range = self.normalized(range);
        let value = self.normalize_value(value);
        self.text
            .replace_range(self.byte(range.start)..self.byte(range.end), &value);
        let end = range.start + value.encode_utf16().count();
        self.anchor = end;
        self.caret = end;
        range.start..end
    }
    pub fn insert(&mut self, value: &str, replacement: Option<Range<usize>>) -> bool {
        if self.read_only || self.disabled {
            return false;
        }
        self.advance_edit_epoch();
        let before = self.snapshot();
        let original = self.composition.take().unwrap_or_else(|| before.clone());
        let range = replacement
            .or_else(|| self.marked.take())
            .unwrap_or_else(|| self.selection());
        self.replace_raw(range, value);
        self.marked = None;
        let changed = self.text != before.text || self.caret != before.caret || before != original;
        if self.snapshot() != original {
            self.remember(original);
        }
        changed
    }
    pub fn preedit(
        &mut self,
        value: &str,
        selection: Range<usize>,
        replacement: Option<Range<usize>>,
    ) -> bool {
        if self.read_only || self.disabled {
            return false;
        }
        // Native IMEs cancel conversion with an empty marked-text update.
        // Restore the original selection/draft rather than retaining a phantom
        // empty composition that traps Tab or blocks accessibility edits.
        if value.is_empty() {
            return match replacement {
                Some(range) => self.insert("", Some(range)),
                None => self.cancel_composition(),
            };
        }
        self.advance_edit_epoch();
        if self.composition.is_none() {
            self.composition = Some(self.snapshot());
        }
        let range = replacement
            .or_else(|| self.marked.take())
            .unwrap_or_else(|| self.selection());
        let range = self.replace_raw(range, value);
        self.anchor = self.boundary(range.start.saturating_add(selection.start).min(range.end));
        self.caret = self.boundary(range.start.saturating_add(selection.end).min(range.end));
        self.marked = Some(range);
        true
    }
    pub fn unmark(&mut self) {
        if self.marked.is_some() || self.composition.is_some() {
            self.advance_edit_epoch();
        }
        self.marked = None;
        if let Some(s) = self.composition.take()
            && s != self.snapshot()
        {
            self.remember(s);
        }
    }
    pub fn cancel_composition(&mut self) -> bool {
        if let Some(s) = self.composition.take() {
            self.restore(s);
            true
        } else {
            false
        }
    }
    fn word_boundaries(&self) -> Vec<usize> {
        let mut offset = 0;
        let mut boundaries = vec![0];
        for part in self.text.split_word_bounds() {
            offset += part.encode_utf16().count();
            boundaries.push(offset);
        }
        boundaries
    }
    /// Same Unicode word segmentation as keyboard word movement, selecting the
    /// segment containing the visual grapheme rather than the word before a caret.
    pub(crate) fn select_word_at(&mut self, index: usize) {
        if let Some(pair) = self
            .word_boundaries()
            .windows(2)
            .find(|pair| pair[0] <= index && index < pair[1])
        {
            self.select_range(pair[0]..pair[1]);
        } else {
            self.select(index, false);
        }
    }
    /// Logical paragraph, including its terminating break. CRLF is one grapheme;
    /// wrapped visual lines do not create paragraphs. A trailing empty paragraph
    /// has an empty selection, rather than selecting the preceding paragraph.
    pub(crate) fn select_paragraph_at(&mut self, index: usize) {
        let mut start = 0;
        let mut end = 0;
        let mut selected = None;
        for grapheme in self.text.graphemes(true) {
            end += grapheme.encode_utf16().count();
            if matches!(grapheme, "\r\n" | "\r" | "\n" | "\u{2028}" | "\u{2029}") {
                if index < end {
                    selected = Some(start..end);
                    break;
                }
                start = end;
            }
        }
        self.select_range(selected.unwrap_or(start..self.len()));
    }
    pub fn move_by(&mut self, direction: isize, extend: bool, word: bool) {
        self.advance_edit_epoch();
        if !extend && !self.selection().is_empty() {
            self.caret = if direction < 0 {
                self.selection().start
            } else {
                self.selection().end
            };
            self.anchor = self.caret;
            return;
        }
        let positions = if word {
            self.word_boundaries()
        } else {
            self.boundaries()
        };
        let next = if direction < 0 {
            positions
                .into_iter()
                .rfind(|n| *n < self.caret)
                .unwrap_or(0)
        } else {
            positions
                .into_iter()
                .find(|n| *n > self.caret)
                .unwrap_or(self.len())
        };
        self.select(next, extend);
    }
    pub fn delete(&mut self, backward: bool, word: bool) -> bool {
        if self.read_only || self.disabled {
            return false;
        }
        if self.marked.is_some() {
            self.unmark();
        }
        let original = self.snapshot();
        if self.selection().is_empty() {
            self.move_by(if backward { -1 } else { 1 }, true, word);
        }
        if self.selection().is_empty() {
            return false;
        }
        let changed = self.insert("", None);
        if changed && let Some(last) = self.undo.last_mut() {
            *last = original;
        }
        changed
    }
    pub fn undo(&mut self) -> bool {
        if self.read_only || self.disabled {
            return false;
        }
        if self.cancel_composition() {
            return true;
        }
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if self.read_only || self.disabled {
            return false;
        }
        if self.cancel_composition() {
            return true;
        }
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(s);
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_native_marked_update_cancels_without_deleting_the_original_selection() {
        let mut t = TextInput::new("A😀B", false);
        t.select_range(1..3);
        assert!(!t.preedit("", 0..0, None));
        assert_eq!(t.text, "A😀B");
        t.preedit("ni", 2..2, None);
        assert!(t.preedit("", 0..0, None));
        assert_eq!(t.text, "A😀B");
        assert_eq!(t.selection(), 1..3);
        assert!(t.marked.is_none());
        assert!(!t.undo());
        assert!(t.preedit("", 0..0, Some(1..3)));
        assert_eq!(t.text, "AB");
        assert!(t.marked.is_none());
        assert!(t.undo());
        assert_eq!(t.text, "A😀B");
    }
    #[test]
    fn undo_and_redo_cancel_only_the_active_composition_before_edit_history() {
        let mut t = TextInput::new("initial", false);
        t.insert("A😀中文", Some(0..7));
        t.select_range(1..3);
        t.preedit("x", 1..1, None);
        assert!(t.undo());
        assert_eq!(t.text, "A😀中文");
        assert_eq!(t.selection(), 1..3);
        assert!(t.marked.is_none());
        assert!(t.undo());
        assert_eq!(t.text, "initial");
        t.preedit("pin", 3..3, None);
        assert!(t.redo());
        assert_eq!(t.text, "initial");
        assert!(t.marked.is_none());
        assert!(t.redo());
        assert_eq!(t.text, "A😀中文");
    }
    #[test]
    fn assistive_selection_preserves_graphemes_and_read_only_composition() {
        let mut t = TextInput::new("A😀e\u{301}中", false);
        assert!(t.select_range(2..4));
        assert_eq!(t.selection(), 1..5);
        assert_eq!(t.selected_text(), "😀e\u{301}");
        assert!(!t.select_range(0..99));
        t.read_only = true;
        assert!(t.select_range(5..6));
        assert!(!t.insert("x", None));
        t.read_only = false;
        t.preedit("pin", 3..3, None);
        let before = t.selection();
        assert!(!t.select_range(0..0));
        assert_eq!(t.selection(), before);
        t.cancel_composition();
        t.disabled = true;
        assert!(!t.select_range(0..0));
    }
    #[test]
    fn grapheme_delete_and_selection_never_split_utf16_or_combining_sequences() {
        let mut t = TextInput::new("A👨‍👩‍👧‍👦e\u{301}中", false);
        t.select(t.len(), false);
        t.delete(true, false);
        assert_eq!(t.text, "A👨‍👩‍👧‍👦e\u{301}");
        t.delete(true, false);
        assert_eq!(t.text, "A👨‍👩‍👧‍👦");
        t.delete(true, false);
        assert_eq!(t.text, "A");
        assert!(t.undo());
        assert_eq!(t.text, "A👨‍👩‍👧‍👦");
        assert!(t.redo());
        assert_eq!(t.text, "A");
    }
    #[test]
    fn preedit_replaces_marked_range_and_is_one_undo_transaction() {
        let mut t = TextInput::new("项目", false);
        t.select(t.len(), false);
        t.preedit("qing", 4..4, None);
        t.preedit("请", 1..1, None);
        assert_eq!(t.text, "项目请");
        t.insert("清晰", None);
        assert_eq!(t.text, "项目清晰");
        assert!(t.undo());
        assert_eq!(t.text, "项目");
        t.preedit("取消", 2..2, None);
        t.cancel_composition();
        assert_eq!(t.text, "项目");
    }
    #[test]
    fn read_only_and_newlines_and_replacement() {
        let mut t = TextInput::new("abc", false);
        t.insert("中\n文", Some(1..2));
        assert_eq!(t.text, "a中文c");
        t.select_all();
        assert_eq!(t.selected_text(), "a中文c");
        t.read_only = true;
        assert!(!t.insert("x", None));
        assert!(!t.delete(true, false));
        assert!(!t.undo());
        let mut m = TextInput::new("", true);
        m.insert("a\r\nb", None);
        assert_eq!(m.text, "a\nb");
    }
}
