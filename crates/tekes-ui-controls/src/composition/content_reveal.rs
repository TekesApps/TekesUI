//! Stable reading-item navigation through the original Content/ScrollContent tree.
use super::*;
use crate::{ReadingDocumentVersion, ReadingUpdateError, ScrollRevealError, ScrollRevealOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentRevealError {
    Reading(ReadingUpdateError),
    UnavailableOwner,
    UndeclaredItem,
    MissingItem,
    TargetReplaced,
    UnlaidOut,
    Clipped,
    InvalidOptions(ScrollRevealError),
    Layout(CompositionError),
    ReadingTransactionInProgress,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentRevealStatus {
    Animating,
    Completed,
    Interrupted,
    Rejected(ContentRevealError),
}
/// The most recent accepted navigation, bound to its original document token.
/// A later explicit command supersedes it; this is not a business request queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentRevealResult {
    pub owner: ComponentId,
    pub version: ReadingDocumentVersion,
    pub item: ComponentId,
    pub status: ContentRevealStatus,
    pub changed: bool,
    pub clamped: bool,
}
#[derive(Clone, Debug)]
struct Step {
    id: ComponentId,
    mount: u64,
    rect: Rect,
    target: Point,
    viewport: Size,
    document: Size,
    generation: u64,
    options: ScrollRevealOptions,
    start_clip_usable: bool,
}
#[derive(Debug)]
pub(super) struct ItemReveal {
    result: ContentRevealResult,
    owner_mount: u64,
    item_mount: u64,
    steps: Vec<Step>,
}
/// Only library-owned scroll/reading metadata is saved for clipping preflight.
/// Children, editors, actions and application components are never cloned.
struct ScrollSnapshot {
    id: ComponentId,
    mount: u64,
    scroll: ScrollArea,
    reading: Option<content_reading::ContentReadingState>,
    viewport: Rect,
    last_frame: Option<std::time::Duration>,
}
fn path(node: &Content, item: ComponentId, out: &mut Vec<ComponentId>) -> bool {
    if !node.component.active() {
        return false;
    }
    out.push(node.id);
    if node.id == item || node.component.children().iter().any(|c| path(c, item, out)) {
        return true;
    }
    out.pop();
    false
}
fn same_offset(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= 0.01 && (a.y - b.y).abs() <= 0.01
}
impl ContentHost {
    fn validate_reveal_item(
        &self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
        item: ComponentId,
    ) -> Result<(u64, u64), ContentRevealError> {
        if self.reading_transaction {
            return Err(ContentRevealError::ReadingTransactionInProgress);
        }
        let owner_node = self
            .scope()
            .find_active(owner)
            .ok_or(ContentRevealError::UnavailableOwner)?;
        let scroll = owner_node
            .component::<ScrollContent>()
            .ok_or(ContentRevealError::UnavailableOwner)?;
        let version = scroll
            .reading_document_version()
            .ok_or(ContentRevealError::Reading(
                ReadingUpdateError::WrongDocument,
            ))?;
        version
            .validate_current(expected)
            .map_err(ContentRevealError::Reading)?;
        let reading = scroll
            .reading
            .as_ref()
            .expect("version requires attachment");
        if !reading.items.contains(&item) {
            return Err(ContentRevealError::UndeclaredItem);
        }
        let target = owner_node
            .find_active(item)
            .ok_or(ContentRevealError::MissingItem)?;
        if self.last_layout.is_none()
            || scroll.reading_result().is_none()
            || target.frame.size.width <= 0.
            || target.frame.size.height <= 0.
        {
            return Err(ContentRevealError::UnlaidOut);
        }
        let original = reading
            .item_mounts
            .get(&item)
            .ok_or(ContentRevealError::UnlaidOut)?;
        if *original != target.mount {
            return Err(ContentRevealError::TargetReplaced);
        }
        Ok((owner_node.mount, target.mount))
    }
    fn reveal_steps(
        &self,
        item: ComponentId,
        options: ScrollRevealOptions,
    ) -> Result<Vec<Step>, ContentRevealError> {
        let mut ids = Vec::new();
        if !path(self.scope(), item, &mut ids) {
            return Err(ContentRevealError::MissingItem);
        }
        let mut rect = self
            .root
            .find(item)
            .ok_or(ContentRevealError::MissingItem)?
            .frame;
        let mut steps = Vec::new();
        for id in ids.into_iter().rev().skip(1) {
            let node = self.root.find(id).expect("path node");
            let Some(scroll) = node.component::<ScrollContent>() else {
                continue;
            };
            let geometry = scroll.scroll.geometry(scroll.viewport, None);
            let origin = geometry.content_origin();
            let local = rect.offset(-origin.x, -origin.y);
            let mut step_options = options;
            if let crate::ScrollRevealPlacement::Start(axis) = options.placement
                && let Some(visible) = scroll
                    .scroll
                    .geometry(scroll.viewport, Some(node.clip))
                    .visible_viewport_rect()
            {
                // Only the selected axis uses the actual visible viewport.
                // Entirely offscreen ancestors first use their own viewport;
                // same-tree endpoint preflight refines once parents reveal it.
                match axis {
                    Axis::Vertical => {
                        step_options.margins.top +=
                            (visible.min_y() - scroll.viewport.min_y()).max(0.);
                        step_options.margins.bottom +=
                            (scroll.viewport.max_y() - visible.max_y()).max(0.);
                    }
                    Axis::Horizontal => {
                        step_options.margins.left +=
                            (visible.min_x() - scroll.viewport.min_x()).max(0.);
                        step_options.margins.right +=
                            (scroll.viewport.max_x() - visible.max_x()).max(0.);
                    }
                }
            }
            let mut start_clip_usable = true;
            let (target, _) = match scroll.scroll.reveal_target(local, step_options) {
                Err(ScrollRevealError::InvalidMargins)
                    if matches!(options.placement, crate::ScrollRevealPlacement::Start(_)) =>
                {
                    // A currently tiny clipped slice may grow when its parent
                    // moves. Validate original margins against the real viewport,
                    // provisionally reveal it, then require usable END geometry.
                    step_options = options;
                    start_clip_usable = false;
                    scroll.scroll.reveal_target(local, options)
                }
                result => result,
            }
            .map_err(ContentRevealError::InvalidOptions)?;
            let old = scroll.scroll.visual_offset();
            rect = rect
                .offset(old.x - target.x, old.y - target.y)
                .intersection(&scroll.viewport)
                .ok_or(ContentRevealError::Clipped)?;
            steps.push(Step {
                id,
                mount: node.mount,
                rect: local,
                target,
                viewport: scroll.scroll.viewport_size(),
                document: scroll.scroll.document_size(),
                generation: 0,
                options: step_options,
                start_clip_usable,
            });
        }
        Ok(steps)
    }
    fn scroll_snapshots(&self, steps: &[Step]) -> Vec<ScrollSnapshot> {
        steps
            .iter()
            .map(|step| {
                let node = self.root.find(step.id).expect("planned scroll");
                let scroll = node
                    .component::<ScrollContent>()
                    .expect("planned ScrollContent");
                ScrollSnapshot {
                    id: step.id,
                    mount: node.mount,
                    scroll: scroll.scroll.clone(),
                    reading: scroll.reading.clone(),
                    viewport: scroll.viewport,
                    last_frame: scroll.last_frame,
                }
            })
            .collect()
    }
    fn restore_scroll_snapshots(&mut self, snapshots: Vec<ScrollSnapshot>) {
        for snapshot in snapshots {
            let Some(node) = self
                .root
                .find_mut(snapshot.id)
                .filter(|n| n.mount == snapshot.mount)
            else {
                continue;
            };
            if let Some(scroll) = node.component_mut::<ScrollContent>() {
                scroll.scroll = snapshot.scroll;
                scroll.reading = snapshot.reading;
                scroll.viewport = snapshot.viewport;
                scroll.last_frame = snapshot.last_frame;
            }
        }
    }
    fn apply_reveal_steps(
        &mut self,
        steps: &mut [Step],
        options: ScrollRevealOptions,
    ) -> Result<(bool, bool), ContentRevealError> {
        let (mut changed, mut clamped) = (false, false);
        for step in steps {
            let node = self
                .root
                .find_mut(step.id)
                .filter(|n| n.mount == step.mount)
                .ok_or(ContentRevealError::TargetReplaced)?;
            let scroll = node
                .component_mut::<ScrollContent>()
                .ok_or(ContentRevealError::TargetReplaced)?;
            let result = scroll
                .scroll
                .reveal_with(
                    step.rect,
                    ScrollRevealOptions {
                        animated: options.animated,
                        ..step.options
                    },
                )
                .map_err(ContentRevealError::InvalidOptions)?;
            step.generation = scroll.scroll.reveal_identity();
            changed |= result.changed;
            clamped |= result.clamped;
            scroll.stop_following();
            if let Some(reading) = &mut scroll.reading {
                reading.explicit_reveal = Some(step.generation);
                reading.fence_momentum();
            }
        }
        Ok((changed, clamped))
    }
    fn cancel_item_reveal_steps(&mut self, steps: &[Step]) {
        for step in steps {
            if let Some(node) = self
                .root
                .find_mut(step.id)
                .filter(|n| n.mount == step.mount)
                && let Some(scroll) = node.component_mut::<ScrollContent>()
            {
                scroll.scroll.cancel_reveal(step.generation);
                if let Some(reading) = &mut scroll.reading
                    && reading.explicit_reveal == Some(step.generation)
                {
                    reading.explicit_reveal = None;
                }
            }
        }
    }
    /// Reveal a declared stable item without changing focus, selection, undo or
    /// marked text. Use the existing current text backend, as with event_with_text;
    /// the library owns real layout, ancestor transforms, clipping and scrolling.
    /// Render once first. No callbacks run against a stale token or replaced item.
    ///
    /// Clipping preflight arranges the SAME tree at the proposed scroll positions,
    /// saves/restores only library scroll/reading metadata, then applies a valid
    /// command. Layout must be deterministic for unchanged inputs; arbitrary
    /// application mutations during layout are retained, as for ordinary render.
    /// Layout/resource errors require a successful render before trying again.
    /// Request a normal render before publishing native geometry or scene pixels.
    pub fn reveal_scroll_reading_item(
        &mut self,
        owner: ComponentId,
        expected: &ReadingDocumentVersion,
        item: ComponentId,
        mut options: ScrollRevealOptions,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<ContentRevealResult, ContentRevealError> {
        let binding = self.validate_reveal_item(owner, expected, item)?;
        self.root
            .ids(&mut BTreeSet::new())
            .map_err(ContentRevealError::Layout)?;
        let (frame, clip) = self.last_layout.ok_or(ContentRevealError::UnlaidOut)?;
        self.layout_frame(text, frame, clip)
            .map_err(ContentRevealError::Layout)?;
        if self.validate_reveal_item(owner, expected, item)? != binding {
            return Err(ContentRevealError::TargetReplaced);
        }
        // Without a host motion clock, complete immediately rather than retain
        // a navigation that this consumer cannot advance.
        options.animated &= text.motion_context().is_some();
        let mut steps = self.reveal_steps(item, options)?;
        let snapshots = self.scroll_snapshots(&steps);
        let trial = (|| {
            let start_alignment =
                matches!(options.placement, crate::ScrollRevealPlacement::Start(_));
            let limit = if start_alignment {
                steps.len().saturating_add(2)
            } else {
                1
            };
            for pass in 0..limit {
                self.apply_reveal_steps(
                    &mut steps,
                    ScrollRevealOptions {
                        animated: false,
                        ..options
                    },
                )?;
                self.layout_frame(text, frame, clip)
                    .map_err(ContentRevealError::Layout)?;
                if self.validate_reveal_item(owner, expected, item)? != binding {
                    return Err(ContentRevealError::TargetReplaced);
                }
                if start_alignment {
                    let refined = self.reveal_steps(item, options)?;
                    if refined.len() != steps.len()
                        || refined
                            .iter()
                            .zip(&steps)
                            .any(|(a, b)| a.id != b.id || a.mount != b.mount)
                    {
                        return Err(ContentRevealError::TargetReplaced);
                    }
                    let settled = refined
                        .iter()
                        .zip(&steps)
                        .all(|(a, b)| same_offset(a.target, b.target));
                    steps = refined;
                    if !settled {
                        // A parent moving into view can change a nested actual
                        // clip. Refine on the SAME tree, with a finite bound;
                        // arbitrary oscillating clipping is not a valid landing.
                        if pass + 1 == limit {
                            return Err(ContentRevealError::Clipped);
                        }
                        continue;
                    }
                    if steps.iter().any(|step| !step.start_clip_usable) {
                        return Err(ContentRevealError::Clipped);
                    }
                }
                return if self.root.find(item).and_then(Content::visible).is_some() {
                    Ok(())
                } else {
                    Err(ContentRevealError::Clipped)
                };
            }
            Err(ContentRevealError::Clipped)
        })();
        self.restore_scroll_snapshots(snapshots);
        let restored = self
            .layout_frame(text, frame, clip)
            .map_err(ContentRevealError::Layout);
        restored?;
        trial?;
        if self.validate_reveal_item(owner, expected, item)? != binding {
            return Err(ContentRevealError::TargetReplaced);
        }
        if let Some(old) = self.item_reveal.take() {
            self.cancel_item_reveal_steps(&old.steps);
        }
        let (changed, clamped) = self.apply_reveal_steps(&mut steps, options)?;
        self.layout_frame(text, frame, clip)
            .map_err(ContentRevealError::Layout)?;
        let result = ContentRevealResult {
            owner,
            version: expected.clone(),
            item,
            status: ContentRevealStatus::Animating,
            changed,
            clamped,
        };
        self.item_reveal = Some(ItemReveal {
            result: result.clone(),
            owner_mount: binding.0,
            item_mount: binding.1,
            steps,
        });
        self.observe_item_reveal();
        self.dirty = true;
        text.request_animation_frame();
        Ok(self
            .item_reveal
            .as_ref()
            .expect("accepted navigation")
            .result
            .clone())
    }
    /// Latest accepted command only; a new command supersedes its predecessor.
    /// Invalid current document/mount identity is reflected even before redraw.
    pub fn scroll_reading_reveal_result(&self) -> Option<ContentRevealResult> {
        let navigation = self.item_reveal.as_ref()?;
        let mut result = navigation.result.clone();
        if matches!(result.status, ContentRevealStatus::Rejected(_)) {
            return Some(result);
        }
        match self.validate_reveal_item(result.owner, &result.version, result.item) {
            Err(error) => result.status = ContentRevealStatus::Rejected(error),
            Ok(binding) if binding != (navigation.owner_mount, navigation.item_mount) => {
                result.status = ContentRevealStatus::Rejected(ContentRevealError::TargetReplaced)
            }
            _ => {}
        }
        Some(result)
    }
    pub(super) fn interrupt_item_reveal_layout(&mut self) {
        if let Some(mut nav) = self.item_reveal.take() {
            if nav.result.status == ContentRevealStatus::Animating {
                self.cancel_item_reveal_steps(&nav.steps);
                nav.result.status = ContentRevealStatus::Interrupted;
            }
            self.item_reveal = Some(nav);
        }
    }
    pub(super) fn fail_item_reveal(&mut self, error: ContentRevealError) {
        if let Some(mut nav) = self.item_reveal.take() {
            self.cancel_item_reveal_steps(&nav.steps);
            nav.result.status = ContentRevealStatus::Rejected(error);
            self.item_reveal = Some(nav);
        }
    }
    fn item_reveal_geometry_matches(&self, nav: &ItemReveal) -> bool {
        let mut current_path = Vec::new();
        if !path(self.scope(), nav.result.item, &mut current_path) {
            return false;
        }
        let ancestors = current_path.into_iter().rev().skip(1).filter(|id| {
            self.root
                .find(*id)
                .is_some_and(|n| n.component::<ScrollContent>().is_some())
        });
        if !ancestors.eq(nav.steps.iter().map(|step| step.id)) {
            return false;
        }
        let Some(target) = self.root.find(nav.result.item) else {
            return false;
        };
        let mut rect = target.frame;
        let close = |a: f32, b: f32| {
            (a - b).abs() <= 0.02_f32.max(4. * f32::EPSILON * a.abs().max(b.abs()))
        };
        for step in &nav.steps {
            let Some(node) = self.root.find(step.id).filter(|n| n.mount == step.mount) else {
                return false;
            };
            let Some(scroll) = node.component::<ScrollContent>() else {
                return false;
            };
            let geometry = scroll.scroll.geometry(scroll.viewport, None);
            let origin = geometry.content_origin();
            let local = rect.offset(-origin.x, -origin.y);
            if ![
                (local.origin.x, step.rect.origin.x),
                (local.origin.y, step.rect.origin.y),
                (local.size.width, step.rect.size.width),
                (local.size.height, step.rect.size.height),
            ]
            .into_iter()
            .all(|(a, b)| close(a, b))
            {
                return false;
            }
            // Compare each ancestor in its own document coordinates, projecting
            // inner viewports to their planned END position. Current animation
            // progress must not look like a document layout change.
            let visual = scroll.scroll.visual_offset();
            let Some(projected) = rect
                .offset(visual.x - step.target.x, visual.y - step.target.y)
                .intersection(&scroll.viewport)
            else {
                return false;
            };
            rect = projected;
        }
        true
    }
    pub(super) fn observe_item_reveal(&mut self) {
        let Some(mut nav) = self.item_reveal.take() else {
            return;
        };
        if matches!(nav.result.status, ContentRevealStatus::Rejected(_)) {
            self.item_reveal = Some(nav);
            return;
        }
        let validation =
            self.validate_reveal_item(nav.result.owner, &nav.result.version, nav.result.item);
        let error = match validation {
            Err(error) => Some(error),
            Ok(binding) if binding != (nav.owner_mount, nav.item_mount) => {
                Some(ContentRevealError::TargetReplaced)
            }
            _ => None,
        };
        if let Some(error) = error {
            self.cancel_item_reveal_steps(&nav.steps);
            nav.result.status = ContentRevealStatus::Rejected(error);
        } else if nav.result.status == ContentRevealStatus::Animating {
            let mut moving = false;
            let mut interrupted = !self.item_reveal_geometry_matches(&nav);
            for step in &nav.steps {
                let scroll = self
                    .root
                    .find(step.id)
                    .filter(|n| n.mount == step.mount)
                    .and_then(|n| n.component::<ScrollContent>());
                if let Some(scroll) = scroll {
                    let owned = scroll.scroll.reveal_identity() == step.generation;
                    let active = owned && scroll.scroll.reveal_in_flight();
                    moving |= active;
                    interrupted |= !owned
                        || scroll.scroll.viewport_size() != step.viewport
                        || scroll.scroll.document_size() != step.document
                        || (!active && !same_offset(scroll.scroll.offset, step.target));
                } else {
                    interrupted = true;
                }
            }
            if interrupted {
                self.cancel_item_reveal_steps(&nav.steps);
                nav.result.status = ContentRevealStatus::Interrupted;
            } else if !moving {
                self.cancel_item_reveal_steps(&nav.steps);
                nav.result.status = if self
                    .root
                    .find(nav.result.item)
                    .and_then(Content::visible)
                    .is_some()
                {
                    ContentRevealStatus::Completed
                } else {
                    ContentRevealStatus::Rejected(ContentRevealError::Clipped)
                };
            }
        }
        self.item_reveal = Some(nav);
    }
}
