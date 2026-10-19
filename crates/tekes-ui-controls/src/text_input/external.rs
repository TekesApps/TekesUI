//! Controlled values reuse the existing editor and caller-owned DraftRevision.
//! No persistence, queue, retry or Session interpretation lives here.
use super::{Snapshot, TextInput};
use crate::DraftRevision;
use std::{
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};
use unicode_segmentation::UnicodeSegmentation;
static NEXT_EDITOR: AtomicU64 = AtomicU64::new(1);
pub(super) fn next_editor() -> u64 {
    NEXT_EDITOR
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("editor identity exhausted")
}
/// Immutable, process-local expected baseline, tied to this editor instance.
/// Capture when an external operation starts, not when its late result arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditBaseline {
    editor: u64,
    epoch: u64,
    value: Snapshot,
    marked: Option<Range<usize>>,
    multiline: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalSelection {
    /// Map both directed endpoints through the longest retained grapheme prefix
    /// and suffix. Endpoints inside the replacement collapse to its new end.
    Preserve,
    /// Canonical text UTF-16 range, normalized to whole grapheme boundaries.
    Set(Range<usize>),
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalUndo {
    /// Retain both existing history stacks, without adding an external step.
    Preserve,
    /// One ordinary undo transaction; undo may recover the pre-clear draft.
    Transaction,
    /// New-document/reset semantics; previous undo/redo cannot cross this boundary.
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalValueConflict {
    ForeignEditor,
    StaleBaseline,
    InvalidIdentity,
    Revision,
    DocumentResetRequired,
    Composition,
    Selection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalValueResult {
    Applied {
        changed: bool,
        document_changed: bool,
    },
    Unchanged,
    Conflict(ExternalValueConflict),
}
impl TextInput {
    /// Also checks the legacy public fields, so an untracked field change cannot
    /// silently overwrite the current value. Mutator epochs additionally fence
    /// edit→undo ABA. Direct public-field ABA is outside the controlled contract.
    pub fn external_baseline(&self) -> TextEditBaseline {
        TextEditBaseline {
            editor: self.editor_id,
            epoch: self.edit_epoch,
            value: self.snapshot(),
            marked: self.marked.clone(),
            multiline: self.multiline,
        }
    }
    pub fn external_value(&self) -> Option<&DraftRevision> {
        self.external.as_ref()
    }
    pub(crate) fn controlled_binding(&self) -> (u64, u64) {
        (self.editor_id, self.binding_epoch)
    }
    /// Atomic external synchronization. Repeating an acknowledged key/revision/
    /// payload is Unchanged even after local edits or while marked. Otherwise a
    /// stale baseline, old revision or composition is rejected without mutation.
    /// There is no internal deferred queue and no implicit preedit commit/cancel.
    /// `payload` is plain text normalized exactly like native TextInput.insert;
    /// explicit selection coordinates refer to that canonical text.
    pub fn apply_external_value(
        &mut self,
        expected: &TextEditBaseline,
        value: &DraftRevision,
        selection: ExternalSelection,
        undo: ExternalUndo,
    ) -> ExternalValueResult {
        use ExternalValueConflict as C;
        use ExternalValueResult as R;
        if expected.editor != self.editor_id {
            return R::Conflict(C::ForeignEditor);
        }
        if value.identity.is_empty() || value.version == 0 {
            return R::Conflict(C::InvalidIdentity);
        }
        if let Some(current) = &self.external
            && current.identity == value.identity
        {
            if value.version < current.version {
                return R::Conflict(C::Revision);
            }
            if value.version == current.version {
                return if value.payload == current.payload {
                    R::Unchanged
                } else {
                    R::Conflict(C::Revision)
                };
            }
        }
        if self.marked.is_some() || self.composition.is_some() {
            return R::Conflict(C::Composition);
        }
        if *expected != self.external_baseline() {
            return R::Conflict(C::StaleBaseline);
        }
        let document_changed = self
            .external
            .as_ref()
            .is_some_and(|v| v.identity != value.identity);
        if document_changed && undo != ExternalUndo::Reset {
            return R::Conflict(C::DocumentResetRequired);
        }
        let text = self.normalize_value(&value.payload);
        let boundaries = Self::boundaries_for(&text);
        let len = *boundaries.last().unwrap();
        let floor = |i| {
            boundaries
                .iter()
                .copied()
                .take_while(|n| *n <= i)
                .last()
                .unwrap_or(0)
        };
        let (anchor, caret) = match selection {
            ExternalSelection::Reset => (0, 0),
            ExternalSelection::Set(range) => {
                if range.start > range.end || range.end > len {
                    return R::Conflict(C::Selection);
                }
                let anchor = floor(range.start);
                let caret = if range.is_empty() {
                    anchor
                } else {
                    *boundaries.iter().find(|n| **n >= range.end).unwrap()
                };
                (anchor, caret)
            }
            ExternalSelection::Preserve => {
                let old: Vec<_> = self.text.graphemes(true).collect();
                let new: Vec<_> = text.graphemes(true).collect();
                let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
                let suffix = old[prefix..]
                    .iter()
                    .rev()
                    .zip(new[prefix..].iter().rev())
                    .take_while(|(a, b)| a == b)
                    .count();
                let prefix_utf16 = old[..prefix]
                    .iter()
                    .map(|s| s.encode_utf16().count())
                    .sum::<usize>();
                let old_end = self.len()
                    - old[old.len() - suffix..]
                        .iter()
                        .map(|s| s.encode_utf16().count())
                        .sum::<usize>();
                let new_end = len
                    - new[new.len() - suffix..]
                        .iter()
                        .map(|s| s.encode_utf16().count())
                        .sum::<usize>();
                let map = |i: usize| {
                    let i = i.min(self.len());
                    floor(if i < prefix_utf16 {
                        i
                    } else if i >= old_end {
                        new_end.saturating_add(i - old_end)
                    } else {
                        new_end
                    })
                };
                (map(self.anchor), map(self.caret))
            }
        };
        let before = self.snapshot();
        let changed = before.text != text || before.anchor != anchor || before.caret != caret;
        let had_history = !self.undo.is_empty() || !self.redo.is_empty();
        // All checks have completed; mutation starts here.
        self.text = text;
        self.anchor = anchor;
        self.caret = caret;
        match undo {
            ExternalUndo::Preserve => {}
            ExternalUndo::Transaction if changed => self.remember(before),
            ExternalUndo::Transaction => {}
            ExternalUndo::Reset => {
                self.undo.clear();
                self.redo.clear();
            }
        }
        self.external = Some(value.clone());
        self.advance_edit_epoch();
        self.binding_epoch = self
            .binding_epoch
            .checked_add(1)
            .expect("editor binding epoch exhausted");
        R::Applied {
            changed: changed || undo == ExternalUndo::Reset && had_history,
            document_changed,
        }
    }
}
