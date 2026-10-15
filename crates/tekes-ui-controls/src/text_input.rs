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
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    composition: Option<Snapshot>,
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
            undo: Vec::new(),
            redo: Vec::new(),
            composition: None,
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
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            anchor: self.anchor,
            caret: self.caret,
        }
    }
    fn restore(&mut self, s: Snapshot) {
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
        let mut n = 0;
        let mut out = vec![0];
        for g in self.text.graphemes(true) {
            n += g.encode_utf16().count();
            out.push(n);
        }
        out
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
        self.caret = self.boundary(index.min(self.len()));
        if !extend {
            self.anchor = self.caret;
        }
    }
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.len();
    }
    pub fn selected_text(&self) -> String {
        let r = self.selection();
        self.text[self.byte(r.start)..self.byte(r.end)].into()
    }
    fn replace_raw(&mut self, range: Range<usize>, value: &str) -> Range<usize> {
        let range = self.normalized(range);
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        let value: String = value
            .chars()
            .filter(|c| !c.is_control() || (self.multiline && (*c == '\n' || *c == '\t')))
            .collect();
        self.text
            .replace_range(self.byte(range.start)..self.byte(range.end), &value);
        let end = range.start + value.encode_utf16().count();
        self.anchor = end;
        self.caret = end;
        range.start..end
    }
    pub fn insert(&mut self, value: &str, replacement: Option<Range<usize>>) -> bool {
        if self.read_only {
            return false;
        }
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
        if self.read_only {
            return false;
        }
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
    pub fn move_by(&mut self, direction: isize, extend: bool, word: bool) {
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
            let mut n = 0;
            let mut v = vec![0];
            for part in self.text.split_word_bounds() {
                n += part.encode_utf16().count();
                v.push(n);
            }
            v
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
        if self.read_only {
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
        if self.read_only {
            return false;
        }
        self.cancel_composition();
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if self.read_only {
            return false;
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
