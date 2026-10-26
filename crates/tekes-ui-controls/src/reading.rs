//! Opt-in reading policy over the existing TextSurface and ScrollArea.
use crate::{ReflowAnchor, TextInput, TextSurface};
use std::{ops::Range, rc::Rc};
use tekes_ui_scene::{Point, Rect};
use tekes_ui_text::Paragraph;
static NEXT_ATTACHMENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Finite tickets owned by the original reading allocator, never mount tickets.
#[derive(Debug)]
pub(crate) struct AttachmentRange { next:u64, end:u64 }
impl AttachmentRange {
    pub(crate) fn empty()->Self{Self{next:0,end:0}}
    pub(crate) fn take(&mut self)->u64 {assert!(self.next<self.end);let n=self.next;self.next+=1;n}
}
pub(crate) fn reserve_attachments(count:usize)->Result<AttachmentRange,ReadingUpdateError> {
    let n=u64::try_from(count).map_err(|_|ReadingUpdateError::InvalidDocument)?;
    if n==0 {return Ok(AttachmentRange{next:0,end:0});}
    let start=NEXT_ATTACHMENT.fetch_update(std::sync::atomic::Ordering::Relaxed,std::sync::atomic::Ordering::Relaxed,|v|v.checked_add(n)).map_err(|_|ReadingUpdateError::InvalidDocument)?;
    Ok(AttachmentRange{next:start,end:start+n})
}
/// A token for one attachment and revision, returned by the controller. Retain
/// this token with an async request, not a freshly read version at callback time.
/// Reattaching A after B creates a new attachment, even for the same key/revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingDocumentVersion {
    key: String,
    revision: u64,
    attachment: u64,
}
impl ReadingDocumentVersion {
    pub(crate) fn from_reserved(key:String,revision:u64,attachment:u64)->Self { Self{key,revision,attachment} }

    pub(crate) fn attach(key: String, revision: u64) -> Result<Self, ReadingUpdateError> {
        if key.trim().is_empty() {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        let attachment = NEXT_ATTACHMENT
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |id| id.checked_add(1),
            )
            .map_err(|_| ReadingUpdateError::InvalidDocument)?;
        Ok(Self {
            key,
            revision,
            attachment,
        })
    }
    pub(crate) fn advanced(
        &self,
        expected: &Self,
        revision: u64,
    ) -> Result<Self, ReadingUpdateError> {
        self.validate_current(expected)?;
        if revision <= self.revision {
            return Err(ReadingUpdateError::StaleRevision);
        }
        let mut version = self.clone();
        version.revision = revision;
        Ok(version)
    }
    pub(crate) fn validate_current(&self, expected: &Self) -> Result<(), ReadingUpdateError> {
        if self.key != expected.key || self.attachment != expected.attachment {
            return Err(ReadingUpdateError::WrongDocument);
        }
        if self != expected {
            return Err(ReadingUpdateError::StaleRevision);
        }
        Ok(())
    }
    /// Read-only attachment generation; this cannot construct or reattach a token.
    pub fn attachment(&self) -> u64 { self.attachment }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// An unchanged, grapheme-aligned UTF-16 interval in the old and new text.
/// Supply all retained intervals relevant to reading/selection. Deleted content
/// has no mapping. Intervals are ordered and non-overlapping in both documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRangeMapping {
    pub old: Range<usize>,
    pub new: Range<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingAnchorUpdate {
    /// Logical position mapped; the next layout computes pixel restoration.
    Mapped,
    FollowingLatest,
    /// Anchor was deleted, or no paragraph existed. Next layout starts at 0.
    FallbackStart,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingSelectionUpdate {
    Preserved,
    Invalidated,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingUpdate {
    pub version: ReadingDocumentVersion,
    pub anchor: ReadingAnchorUpdate,
    pub selection: ReadingSelectionUpdate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingUpdateError {
    EditableSurface,
    CompositionActive,
    InvalidDocument,
    WrongDocument,
    StaleRevision,
    UnversionedChange,
    InvalidMapping,
}

/// Retains explicit follow/history intent and a UTF-16 reading anchor through
/// tail append, width changes and text scaling. It is not enabled on ordinary
/// text editors. The default retains history
/// mode; a new TextSurface starts at the beginning. Only an explicit
/// `follow_latest` action opts into following.
///
/// The host still owns content, focus, event routing and its Return to latest
/// Button. Timers and arriving content never change this controller's intent.
/// Legacy unversioned replacement keeps ordinary offset clamping. For explicit
/// read-only document updates, set_document/update_document validate identity,
/// revision and retained UTF-16 intervals before restoring the reading anchor.
#[derive(Debug, Default)]
pub struct ReadingController {
    following_latest: bool,
    version: Option<ReadingDocumentVersion>,
    document_text: String,
    pending_anchor: Option<ReflowAnchor>,
}
impl ReadingController {
    /// Explicitly attach/reset caller-owned read-only history. This is a new
    /// document, not a per-frame controlled editor refresh (#18). It resets
    /// selection/undo/scroll but preserves flags, role and explicit follow intent.
    pub fn set_document(
        &mut self,
        surface: &mut TextSurface,
        key: impl Into<String>,
        revision: u64,
        text: &str,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        Self::check_surface(surface)?;
        let version = ReadingDocumentVersion::attach(key.into(), revision)?;
        let disabled = surface.input.disabled;
        surface.input = TextInput::new(text, surface.input.multiline);
        surface.input.read_only = true;
        surface.input.disabled = disabled;
        surface.clauses.clear();
        surface.cancel_interaction();
        surface.reflow_anchor = None;
        surface.paragraph = None;
        surface.shaped.clear();
        let offset = surface.scroll.offset;
        surface.scroll.scroll_by(-offset.x, -offset.y);
        self.pending_anchor = None;
        self.document_text = text.into();
        self.version = Some(version.clone());
        Ok(version)
    }

    pub fn document_version(&self) -> Option<&ReadingDocumentVersion> {
        self.version.as_ref()
    }

    fn check_surface(surface: &TextSurface) -> Result<(), ReadingUpdateError> {
        if surface.input.marked.is_some() {
            return Err(ReadingUpdateError::CompositionActive);
        }
        if !surface.input.read_only {
            return Err(ReadingUpdateError::EditableSurface);
        }
        Ok(())
    }

    /// Atomically replace a versioned read-only history document. No mutation
    /// occurs on Err. Selection and logical reading position use verified text
    /// intervals, never caller-computed pixel compensation. Consecutive updates
    /// before layout compose through the pending logical anchor.
    ///
    /// The same editor object survives; focus is host-owned and untouched.
    /// History arrivals are not undoable edits: dormant editor undo is cleared.
    /// Editable/IME surfaces use the separate controlled editing contract.
    pub fn update_document(
        &mut self,
        surface: &mut TextSurface,
        expected: &ReadingDocumentVersion,
        revision: u64,
        text: &str,
        mappings: &[TextRangeMapping],
    ) -> Result<ReadingUpdate, ReadingUpdateError> {
        Self::check_surface(surface)?;
        let current = self
            .version
            .as_ref()
            .ok_or(ReadingUpdateError::WrongDocument)?;
        let version = current.advanced(expected, revision)?;
        if surface.input.text != self.document_text {
            return Err(ReadingUpdateError::UnversionedChange);
        }
        let old = &surface.input.text;
        let old_boundaries = surface.input.boundaries();
        let new_boundaries = TextInput::boundaries_for(text);
        let mut ends = (0, 0);
        for mapping in mappings {
            if mapping.old.start < ends.0
                || mapping.new.start < ends.1
                || mapping.old.start >= mapping.old.end
                || mapping.new.start >= mapping.new.end
                || !old_boundaries.contains(&mapping.old.start)
                || !old_boundaries.contains(&mapping.old.end)
                || !new_boundaries.contains(&mapping.new.start)
                || !new_boundaries.contains(&mapping.new.end)
                || slice(old, mapping.old.clone()) != slice(text, mapping.new.clone())
            {
                return Err(ReadingUpdateError::InvalidMapping);
            }
            ends = (mapping.old.end, mapping.new.end);
        }
        let map = |index: usize| -> Option<usize> {
            // Right affinity at an inserted gap; final endpoint remains before
            // a tail append. Nonempty selection text is checked below as well.
            let mapping = mappings
                .iter()
                .find(|m| m.old.contains(&index))
                .or_else(|| mappings.iter().rev().find(|m| m.old.end == index))?;
            let mapped = mapping
                .new
                .start
                .checked_add(index.checked_sub(mapping.old.start)?)?;
            new_boundaries.contains(&mapped).then_some(mapped)
        };
        let old_anchor = self
            .pending_anchor
            .filter(|a| (a.applied_y - surface.scroll.offset.y).abs() < 0.001)
            .or_else(|| {
                (surface.input.text == surface.shaped)
                    .then(|| Self::capture_anchor(surface))
                    .flatten()
            });
        let mut mapped_anchor = old_anchor.and_then(|mut a| {
            a.utf16 = map(a.utf16)?;
            Some(a)
        });
        let anchor = if self.following_latest {
            mapped_anchor = None;
            ReadingAnchorUpdate::FollowingLatest
        } else if mapped_anchor.is_some() {
            ReadingAnchorUpdate::Mapped
        } else {
            mapped_anchor = Some(ReflowAnchor {
                utf16: 0,
                within_line: 0.,
                applied_y: surface.scroll.offset.y,
                bottom: false,
            });
            ReadingAnchorUpdate::FallbackStart
        };
        let selected = surface.input.selection();
        let mapped_selection = if selected.is_empty() {
            map(surface.input.caret).map(|c| (c, c))
        } else {
            // Endpoints alone cannot prove selected content retained identity:
            // an unmapped replacement may coincidentally have identical bytes.
            // Require continuous coverage in BOTH documents, with right/left
            // affinity at selection start/end respectively.
            map(selected.start).and_then(|start| {
                mappings
                    .iter()
                    .filter(|m| m.old.start < selected.end && m.old.end > selected.start)
                    .try_fold((selected.start, start), |(cursor, next), m| {
                        let from = m.old.start.max(selected.start);
                        let end = m.old.end.min(selected.end);
                        (from == cursor && m.new.start + from - m.old.start == next)
                            .then_some((end, next + end - from))
                    })
                    .filter(|(end, next)| *end == selected.end && new_boundaries.contains(next))
                    .map(|(_, end)| {
                        if surface.input.anchor <= surface.input.caret {
                            (start, end)
                        } else {
                            (end, start)
                        }
                    })
            })
        };
        let selection = if mapped_selection.is_some() {
            ReadingSelectionUpdate::Preserved
        } else {
            ReadingSelectionUpdate::Invalidated
        };
        let (a, c) = mapped_selection.unwrap_or_else(|| {
            let c = map(surface.input.caret).unwrap_or(0);
            (c, c)
        });
        surface.input.text = text.into();
        surface.input.anchor = a;
        surface.input.caret = c;
        surface.input.clear_reading_history();
        // Cancel stale drag geometry but never blur/focus a component or editor.
        surface.cancel_interaction();
        surface.reflow_anchor = None;
        self.pending_anchor = mapped_anchor;
        self.document_text = text.into();
        self.version = Some(version.clone());
        Ok(ReadingUpdate {
            version,
            anchor,
            selection,
        })
    }

    fn capture_anchor(surface: &TextSurface) -> Option<ReflowAnchor> {
        surface
            .reflow_anchor
            .filter(|a| !a.bottom && (a.applied_y - surface.scroll.offset.y).abs() < 0.001)
            .or_else(|| {
                let offset = surface.scroll.offset.y;
                let line = surface
                    .paragraph
                    .as_ref()?
                    .lines
                    .iter()
                    .rev()
                    .find(|l| l.top as f32 <= offset)?;
                Some(ReflowAnchor {
                    utf16: line.utf16.start,
                    within_line: offset - line.top as f32,
                    applied_y: offset,
                    bottom: false,
                })
            })
    }

    pub fn is_following_latest(&self) -> bool {
        self.following_latest
    }

    /// Explicitly stop following before a user scroll, selection, scrollbar
    /// interaction or programmatic jump to earlier content. Does not cancel an
    /// active native scroll gesture or change the editor's focus/selection.
    pub fn stop_following(&mut self, surface: &mut TextSurface) {
        self.following_latest = false;
        self.pending_anchor = None;
        surface.reflow_anchor = None;
    }

    /// Observe an event already routed to this surface, before calling the
    /// existing wheel API. Like TextSurface::wheel_at, positive delta.y means
    /// native upward input. Horizontal Shift-wheel does not change follow intent.
    pub fn observe_scroll(&mut self, surface: &mut TextSurface, delta: Point, shift: bool) {
        if delta.y > 0.0 && !shift {
            self.stop_following(surface);
        }
    }

    /// Explicit Return to latest action. Reuses the surface's cancellation and
    /// scrolling paths without touching text, IME, selection or host focus.
    /// The next layout follows any content that arrived since the last frame.
    pub fn follow_latest(&mut self, surface: &mut TextSurface) {
        self.following_latest = true;
        self.pending_anchor = None;
        surface.reflow_anchor = None;
        surface.cancel_interaction();
        let target = surface.scroll.limit().y;
        surface
            .scroll
            .scroll_by(0.0, target - surface.scroll.offset.y);
    }

    /// Same layout/shaping contract as TextSurface, with the existing reading
    /// anchor protection applied around it. The paragraph is captured before
    /// shaping so append and reflow in the same frame retain the logical line.
    /// Preserve the first UTF-16 anchor across repeated narrow/wide layouts.
    pub fn layout_with(
        &mut self,
        surface: &mut TextSurface,
        frame: Rect,
        ancestor_clip: Option<Rect>,
        scale: f32,
        type_scale: f32,
        shape: impl FnMut(&str, Option<f32>) -> Rc<Paragraph>,
    ) {
        let pending = self
            .pending_anchor
            .take()
            .filter(|a| (a.applied_y - surface.scroll.offset.y).abs() < 0.001);
        let anchor = pending.or_else(|| {
            (!self.following_latest && surface.input.text.starts_with(&surface.shaped))
                .then(|| Self::capture_anchor(surface))
                .flatten()
        });
        surface.layout_with(frame, ancestor_clip, scale, type_scale, shape);
        if self.following_latest {
            let target = surface.scroll.limit().y;
            surface
                .scroll
                .scroll_by(0.0, target - surface.scroll.offset.y);
        } else if let Some(mut anchor) = anchor {
            let target = surface.caret_doc(anchor.utf16).min_y() + anchor.within_line;
            surface
                .scroll
                .scroll_by(0.0, target - surface.scroll.offset.y);
            anchor.applied_y = surface.scroll.offset.y;
            surface.reflow_anchor = Some(anchor);
        }
    }
}

fn slice(text: &str, range: Range<usize>) -> Option<&str> {
    let byte = |index| {
        let mut utf16 = 0;
        for (byte, c) in text.char_indices() {
            if utf16 == index {
                return Some(byte);
            }
            utf16 += c.len_utf16();
        }
        (utf16 == index).then_some(text.len())
    };
    text.get(byte(range.start)?..byte(range.end)?)
}

#[cfg(test)]
pub(crate) fn test_attachment_counter(value:Option<u64>)->u64 {
    use std::sync::atomic::Ordering;
    match value{Some(v)=>NEXT_ATTACHMENT.swap(v,Ordering::Relaxed),None=>NEXT_ATTACHMENT.load(Ordering::Relaxed)}
}
