//! Opt-in structural reading over the existing Content tree and ScrollArea.
use super::*;
use crate::{ReadingDocumentVersion, ReadingUpdateError};
use std::collections::BTreeMap;

impl Content {
    /// Observe a stopped original child route without delivering its wheel a
    /// second time. This runs synchronously on each original ancestor: there
    /// is no retained callback that can cross mount/document generations.
    pub(super) fn observe_nested_reading(
        &mut self,
        event: &ContentEvent,
        source: ComponentId,
        response: &ContentResponse,
    ) -> bool {
        let ContentEvent::Scroll {
            position,
            delta,
            begin,
            momentum,
            ..
        } = event
        else {
            return false;
        };
        if !response.consumed
            || !delta.y.is_finite()
            || delta.y == 0.
            || !self.active()
            || self.component.semantics().disabled
        {
            return false;
        }
        let original_reader = self.find_active(source).is_some_and(|owner| {
            !owner.component.semantics().disabled
                && owner.visible().is_some_and(|r| r.contains(*position))
                && owner.curve_contains(*position)
                && owner.component.reading_scroll_source()
        });
        if !original_reader {
            return false;
        }
        self.component_mut::<ScrollContent>()
            .and_then(|scroll| scroll.reading.as_mut())
            // A fresh downward gesture also retires an old momentum fence;
            // only upward intent pauses following in this original method.
            // Original user-input and momentum fences apply. This changes
            // reading intent only; never parent ScrollArea geometry/offset.
            .is_some_and(|reading| reading.observe_scroll(*delta, *begin, *momentum))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentReadingAnchorUpdate {
    Preserved(ComponentId),
    /// First surviving successor, then predecessor, then first mounted item.
    /// None means no declared item has nonempty active layout.
    Fallback(Option<ComponentId>),
    FollowingLatest,
}

/// Result of the last completed layout, bound to the accepted document token.
/// Item-local positions use logical points. This does not pretend that an
/// arbitrary custom component exposes word/UTF-16 anchors; TextSurface's
/// semantic text-document transactions remain ReadingController's contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentReadingResult {
    pub version: ReadingDocumentVersion,
    pub anchor: ContentReadingAnchorUpdate,
    pub clamped: bool,
    pub user_input_won: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Anchor {
    item: ComponentId,
    within: f32,
    screen_y: f32,
    fallback: Vec<ComponentId>,
}
#[derive(Clone, Debug)]
struct SavedDocument {
    revision: u64,
    following: bool,
    anchor: Option<Anchor>,
    offset: Point,
}

#[derive(Clone, Debug)]
pub(super) struct ContentReadingState {
    version: ReadingDocumentVersion,
    pub(super) items: Vec<ComponentId>,
    pub(super) item_mounts: BTreeMap<ComponentId, u64>,
    following: bool,
    pub(super) explicit_reveal: Option<u64>,
    memories: BTreeMap<String, SavedDocument>,
    frames: Vec<(ComponentId, Rect)>,
    viewport: Rect,
    visual_offset: Point,
    pending: Option<Anchor>,
    restore_offset: Option<Point>,
    user_input: bool,
    momentum_fenced: bool,
    result: Option<ContentReadingResult>,
}

// Existing anchor clamp tolerance also bounds round-trip restoration noise.
const ANCHOR_EPSILON: f32 = 0.001;

fn valid_items(items: &[ComponentId]) -> bool {
    items.iter().copied().collect::<BTreeSet<_>>().len() == items.len()
}
fn frames(children: &[Content], items: &[ComponentId]) -> Vec<(ComponentId, Rect)> {
    items
        .iter()
        .filter_map(|id| {
            let node = children.iter().find_map(|c| c.find_active(*id))?;
            (node.frame.size.height > 0.0 && node.frame.size.width > 0.0)
                .then_some((*id, node.frame))
        })
        .collect()
}

impl ContentReadingState {
    pub(super) fn capture(&self, scroll: &ScrollArea) -> Option<Anchor> {
        if self.frames.is_empty() {
            return self.pending.clone();
        }
        // Stored frames precede a new layout. Include input received since
        // those frames were arranged, so an arriving update cannot undo it.
        let top = self.viewport.min_y() + scroll.visual_offset().y - self.visual_offset.y;
        let chosen = self
            .frames
            .iter()
            .enumerate()
            .filter(|(_, (_, f))| f.max_y() > top && f.min_y() < top + self.viewport.size.height)
            .min_by(|(_, (_, a)), (_, (_, b))| a.min_y().total_cmp(&b.min_y()))
            .or_else(|| self.frames.iter().enumerate().next_back())?;
        let (index, (item, frame)) = chosen;
        let within = (top - frame.min_y()).clamp(0.0, frame.size.height);
        let fallback = self.frames[index + 1..]
            .iter()
            .chain(self.frames[..index].iter().rev())
            .map(|(id, _)| *id)
            .collect();
        Some(Anchor {
            item: *item,
            within,
            screen_y: frame.min_y() + within - top,
            fallback,
        })
    }
    pub(super) fn prepare(&mut self, scroll: &mut ScrollArea) {
        if let Some(offset) = self.restore_offset.take() {
            scroll.scroll_by(offset.x - scroll.offset.x, offset.y - scroll.offset.y);
        }
        if self.following {
            scroll.scroll_by(0.0, scroll.limit().y - scroll.offset.y);
        }
    }
    pub(super) fn restore(
        &mut self,
        scroll: &mut ScrollArea,
        children: &[Content],
        viewport: Rect,
        anchor: Option<Anchor>,
    ) -> bool {
        if viewport.size.height <= 0.0 || viewport.size.width <= 0.0 {
            self.pending = anchor;
            self.result = Some(ContentReadingResult {
                version: self.version.clone(),
                anchor: ContentReadingAnchorUpdate::Fallback(None),
                clamped: false,
                user_input_won: self.user_input,
            });
            return false;
        }
        let items = frames(children, &self.items);
        let mut clamped = false;
        let (target, status) = if self.explicit_reveal == Some(scroll.reveal_identity()) {
            // Explicit navigation already owns this offset; do not compensate
            // its animated nested-child movement as if it were content reflow.
            let anchor = items
                .iter()
                .find(|(_, f)| f.max_y() > viewport.min_y() && f.min_y() < viewport.max_y());
            (
                scroll.offset.y,
                anchor.map_or(ContentReadingAnchorUpdate::Fallback(None), |(id, _)| {
                    ContentReadingAnchorUpdate::Preserved(*id)
                }),
            )
        } else if self.following {
            (
                scroll.limit().y,
                ContentReadingAnchorUpdate::FollowingLatest,
            )
        } else if let Some(anchor) = anchor {
            let original = items.iter().find(|(id, _)| *id == anchor.item);
            let candidate = original
                .or_else(|| {
                    anchor
                        .fallback
                        .iter()
                        .find_map(|id| items.iter().find(|(key, _)| id == key))
                })
                .or_else(|| items.first());
            if let Some((id, frame)) = candidate {
                let local = if original.is_some() {
                    anchor.within.min(frame.size.height)
                } else {
                    0.0
                };
                clamped = original.is_some() && (local - anchor.within).abs() > ANCHOR_EPSILON;
                // Resolve displacement before adding the retained offset.
                // f32 intermediate sums can otherwise move an unchanged
                // fractional user offset by one ULP after navigation stops.
                let displacement = f64::from(frame.min_y()) + f64::from(local)
                    - f64::from(viewport.min_y())
                    - f64::from(anchor.screen_y);
                let target = (f64::from(scroll.offset.y) + displacement) as f32;
                (
                    target,
                    if original.is_some() {
                        ContentReadingAnchorUpdate::Preserved(*id)
                    } else {
                        ContentReadingAnchorUpdate::Fallback(Some(*id))
                    },
                )
            } else {
                (0.0, ContentReadingAnchorUpdate::Fallback(None))
            }
        } else {
            (
                if self.user_input {
                    scroll.offset.y
                } else {
                    0.0
                },
                ContentReadingAnchorUpdate::Fallback(items.first().map(|(id, _)| *id)),
            )
        };
        let applied = target.clamp(0.0, scroll.limit().y);
        clamped |= (target - applied).abs() > ANCHOR_EPSILON;
        let delta = applied - scroll.offset.y;
        // Keep a fractional manual offset bit-identical on unchanged reflow;
        // below the existing anchor tolerance there is no clamped movement.
        let changed = delta.abs() > ANCHOR_EPSILON && scroll.scroll_by(0.0, delta);
        self.result = Some(ContentReadingResult {
            version: self.version.clone(),
            anchor: status,
            clamped,
            user_input_won: self.user_input,
        });
        changed
    }
    pub(super) fn record(&mut self, scroll: &ScrollArea, children: &[Content], viewport: Rect) {
        if viewport.size.height <= 0.0 || viewport.size.width <= 0.0 {
            self.frames.clear();
            return;
        }
        self.frames = frames(children, &self.items);
        for (id, _) in &self.frames {
            if let Some(node) = children.iter().find_map(|c| c.find_active(*id)) {
                self.item_mounts.entry(*id).or_insert(node.mount);
            }
        }
        self.viewport = viewport;
        self.visual_offset = scroll.visual_offset();
        self.pending = None;
        self.user_input = false;
    }
    pub(super) fn observe_scroll(&mut self, delta: Point, begin: bool, momentum: bool) -> bool {
        if momentum && self.momentum_fenced {
            return false;
        }
        if !momentum && (begin || delta != Point::default()) {
            self.momentum_fenced = false;
        }
        if delta.y != 0.0 {
            if delta.y < 0.0 {
                self.following = false;
            }
            self.user_input = true;
            // A document switch's saved position must not override new input.
            self.pending = None;
            self.restore_offset = None;
        }
        true
    }
    pub(super) fn fence_momentum(&mut self) {
        self.momentum_fenced = true;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReaderUnmountWrites {
    pub(super) key: String,
    pub(super) revision: u64,
    pub(super) original: ReadingDocumentVersion,
    pending: Option<Anchor>,
    offset: Point,
}
pub(super) fn reader_preparation_work(state: &ContentReadingState, calls: usize) -> Option<usize> {
    let fallback = state.pending.as_ref().map_or(0, |a| a.fallback.len());
    let per = state
        .frames
        .len()
        .checked_add(fallback)?
        .checked_mul(std::mem::size_of::<ComponentId>())?
        .checked_add(state.version.key().len().checked_mul(4)?)?
        .checked_add(512)?;
    per.checked_mul(calls)?.checked_mul(4)
}
pub(super) fn prepare_reader_writes(
    state: &ContentReadingState,
    scroll: &ScrollArea,
) -> ReaderUnmountWrites {
    ReaderUnmountWrites {
        key: state.version.key().into(),
        revision: state.version.revision(),
        original: state.version.clone(),
        pending: state.capture(scroll),
        offset: scroll.offset,
    }
}
pub(super) fn apply_reader_writes(
    state: &mut ContentReadingState,
    writes: ReaderUnmountWrites,
    version: ReadingDocumentVersion,
) {
    state.pending = writes.pending;
    state.restore_offset = Some(writes.offset);
    state.frames.clear();
    state.item_mounts.clear();
    state.explicit_reveal = None;
    state.momentum_fenced = true;
    state.version = version;
    state.result = None;
}
impl ScrollContent {
    /// Initialize an unmounted original reader, including one retained by a public
    /// compliant wrapper. Mounted document changes use the existing Host transaction.
    /// Allocation here is caller preparation, before a checked Content request.
    pub fn initialize_reading_document(
        &mut self,
        key: impl Into<String>,
        revision: u64,
        items: Vec<ComponentId>,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        if self.reading_mounted {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        self.attach_reading(key.into(), revision, items)
    }

    fn attach_reading(
        &mut self,
        key: String,
        revision: u64,
        items: Vec<ComponentId>,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        if !valid_items(&items) {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        if let Some(state) = &self.reading {
            let known = if state.version.key() == key {
                Some(state.version.revision())
            } else {
                state.memories.get(&key).map(|m| m.revision)
            };
            if known.is_some_and(|known| revision < known) {
                return Err(ReadingUpdateError::StaleRevision);
            }
        }
        let version = ReadingDocumentVersion::attach(key, revision)?;
        let mut memories = BTreeMap::new();
        if let Some(state) = &mut self.reading {
            let saved = SavedDocument {
                revision: state.version.revision(),
                following: state.following,
                anchor: state.capture(&self.scroll),
                offset: state.restore_offset.unwrap_or(self.scroll.offset),
            };
            memories = std::mem::take(&mut state.memories);
            memories.insert(state.version.key().into(), saved);
        }
        let saved = memories.remove(version.key());
        let following = saved.as_ref().is_some_and(|m| m.following);
        let pending = saved.as_ref().and_then(|m| m.anchor.clone());
        let restore_offset = Some(saved.map_or(Point::default(), |m| m.offset));
        // Switching documents ends the old document's input ownership, not the
        // sibling composer's focus, selection, text or IME transaction.
        self.scroll.cancel();
        self.scroll
            .scroll_by(-self.scroll.offset.x, -self.scroll.offset.y);
        self.reading = Some(ContentReadingState {
            version: version.clone(),
            items,
            item_mounts: BTreeMap::new(),
            following,
            explicit_reveal: None,
            memories,
            frames: Vec::new(),
            viewport: Rect::default(),
            visual_offset: Point::default(),
            pending,
            restore_offset,
            user_input: false,
            momentum_fenced: true,
            result: None,
        });
        Ok(version)
    }
    fn advance_reading(
        &mut self,
        expected: &ReadingDocumentVersion,
        revision: u64,
        items: Vec<ComponentId>,
    ) -> Result<ReadingDocumentVersion, ReadingUpdateError> {
        if !valid_items(&items) {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        let state = self
            .reading
            .as_mut()
            .ok_or(ReadingUpdateError::WrongDocument)?;
        let version = state.version.advanced(expected, revision)?;
        state.version = version.clone();
        state.items = items;
        state.item_mounts.clear();
        state.explicit_reveal = None;
        state.result = None;
        Ok(version)
    }
    /// Declared stable item IDs from the original current document.
    pub fn reading_items(&self) -> &[ComponentId] {
        self.reading
            .as_ref()
            .map_or(&[], |state| state.items.as_slice())
    }
    pub fn reading_document_version(&self) -> Option<&ReadingDocumentVersion> {
        self.reading.as_ref().map(|s| &s.version)
    }
    pub fn reading_result(&self) -> Option<&ContentReadingResult> {
        self.reading.as_ref().and_then(|s| s.result.as_ref())
    }
    pub fn is_following_latest(&self) -> bool {
        self.reading.as_ref().is_some_and(|s| s.following)
    }
    pub fn stop_following(&mut self) {
        if let Some(state) = &mut self.reading {
            state.following = false;
            state.pending = None;
            state.restore_offset = None;
        }
    }
    /// Explicit user action only. Arrivals and layout never opt into following.
    /// Returns false on an ordinary, non-reading viewport.
    pub fn follow_latest(&mut self) -> bool {
        let Some(state) = &mut self.reading else {
            return false;
        };
        let changed = !state.following;
        state.following = true;
        state.explicit_reveal = None;
        state.momentum_fenced = true;
        state.pending = None;
        state.restore_offset = None;
        self.scroll.cancel();
        self.scroll
            .scroll_by(0.0, self.scroll.limit().y - self.scroll.offset.y)
            || changed
    }
    pub(super) fn reading_unmounted(&mut self) {
        if let Some(state) = &mut self.reading {
            let writes = prepare_reader_writes(state, &self.scroll);
            match ReadingDocumentVersion::attach(writes.key.clone(), writes.revision) {
                Ok(version) => apply_reader_writes(state, writes, version),
                Err(_) => self.reading = None,
            }
        }
    }
}

impl ContentHost {
    /// Attach a document and its caller-owned ordered stable item keys. The
    /// callback uses existing insert/remove/reorder/get_mut APIs; the library
    /// owns geometry compensation. No callback runs for an invalid attachment.
    /// A->B->A gets a fresh token and restores each document's reading intent.
    pub fn set_scroll_reading_document<R>(
        &mut self,
        owner: ComponentId,
        key: impl Into<String>,
        revision: u64,
        items: Vec<ComponentId>,
        update: impl FnOnce(&mut Self) -> R,
    ) -> Result<(ReadingDocumentVersion, R), ReadingUpdateError> {
        if self.reading_transaction {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        let scroll = self
            .root
            .find_mut(owner)
            .and_then(|c| c.component_mut::<ScrollContent>())
            .ok_or(ReadingUpdateError::InvalidDocument)?;
        let version = scroll.attach_reading(key.into(), revision, items)?;
        Ok((version, self.run_reading_transaction(update)))
    }
    /// Keep the request's expected token. Identity/revision is validated before
    /// the synchronous callback can mutate any tree/editor state. The next
    /// ordinary render measures and restores inside layout before painting.
    /// This is not rollback for arbitrary application callbacks/panics; do not
    /// replace the reading owner within its own callback. Missing/unmounted
    /// declared items produce the documented deterministic layout fallback.
    pub fn update_scroll_reading_document<R>(
        &mut self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
        revision: u64,
        items: Vec<ComponentId>,
        update: impl FnOnce(&mut Self) -> R,
    ) -> Result<(ReadingDocumentVersion, R), ReadingUpdateError> {
        if self.reading_transaction {
            return Err(ReadingUpdateError::InvalidDocument);
        }
        let scroll = self
            .root
            .find_mut(owner)
            .and_then(|c| c.component_mut::<ScrollContent>())
            .ok_or(ReadingUpdateError::InvalidDocument)?;
        let version = scroll.advance_reading(expected, revision, items)?;
        Ok((version, self.run_reading_transaction(update)))
    }
    fn run_reading_transaction<R>(&mut self, update: impl FnOnce(&mut Self) -> R) -> R {
        self.reading_transaction = true;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| update(self)));
        self.reading_transaction = false;
        self.dirty = true;
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}
