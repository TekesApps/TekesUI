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
    Plan(HostRevealError),
}
impl From<HostRevealError> for ContentRevealError {
    fn from(error: HostRevealError) -> Self {
        match error {
            HostRevealError::InvalidOptions(error) => Self::InvalidOptions(error),
            HostRevealError::UnlaidOut => Self::UnlaidOut,
            HostRevealError::NoFeasibleRegion => Self::Clipped,
            error => Self::Plan(error),
        }
    }
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
pub(super) struct Step {
    pub(super) id: ComponentId,
    pub(super) mount: u64,
    pub(super) rect: Rect,
    pub(super) target: Point,
    pub(super) viewport: Size,
    pub(super) document: Size,
    pub(super) generation: u64,
}
#[derive(Debug)]
pub(super) struct ItemReveal {
    result: ContentRevealResult,
    owner_mount: u64,
    item_mount: u64,
    steps: Vec<Step>,
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
    fn reveal_cancel_routes(
        &self,
        steps: &[Step],
    ) -> Vec<Option<content_reveal_plan::ContentRoute>> {
        steps
            .iter()
            .map(|step| {
                let route =
                    content_reveal_plan::ContentRoute::capture(&self.root, step.id, usize::MAX)
                        .ok()?;
                route
                    .node(&self.root, route.depth())
                    .filter(|node| node.mount == step.mount)?;
                Some(route)
            })
            .collect()
    }
    fn cancel_item_reveal_steps_at(
        &mut self,
        steps: &[Step],
        routes: &[Option<content_reveal_plan::ContentRoute>],
    ) {
        for (step, route) in steps.iter().zip(routes) {
            let Some(route) = route else {
                continue;
            };
            let needed = route
                .node(&self.root, route.depth())
                .filter(|node| node.mount == step.mount)
                .and_then(|node| node.component::<ScrollContent>())
                .is_some_and(|scroll| {
                    scroll.scroll.reveal_identity() == step.generation
                        || scroll
                            .reading
                            .as_ref()
                            .is_some_and(|reading| reading.explicit_reveal == Some(step.generation))
                });
            if !needed {
                continue;
            }
            if let Some(node) = route
                .node_mut(&mut self.root, route.depth())
                .filter(|node| node.mount == step.mount)
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
    fn cancel_item_reveal_steps(&mut self, steps: &[Step]) {
        let routes = self.reveal_cancel_routes(steps);
        self.cancel_item_reveal_steps_at(steps, &routes);
    }
    /// Reveal a declared stable item without changing focus, selection, undo or
    /// marked text. Use the existing current text backend, as with event_with_text;
    /// the library owns real layout, ancestor transforms, clipping and scrolling.
    /// Render once first. No callbacks run against a stale token or replaced item.
    ///
    /// Planning reads original arrangement and translation constraints only.
    /// Unsupported layout and stale inputs reject before any mutation. Request
    /// normal rendering after acceptance before publishing native geometry.
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
        options.animated &= text.motion_context().is_some();
        let plan = self
            .plan_host_reveal(item, options, &[], HostRevealBudget::default())
            .map_err(ContentRevealError::from)?;
        let steps = plan.item_steps();
        let mut navigation = ItemReveal {
            result: ContentRevealResult {
                owner,
                version: expected.clone(),
                item,
                status: ContentRevealStatus::Animating,
                changed: false,
                clamped: false,
            },
            owner_mount: binding.0,
            item_mount: binding.1,
            steps,
        };
        let old_routes = self
            .item_reveal
            .as_ref()
            .map(|navigation| self.reveal_cancel_routes(&navigation.steps))
            .unwrap_or_default();
        let outcome = self
            .commit_host_reveal(plan)
            .map_err(ContentRevealError::from)?;
        // No fallible callback or live layout after the shared commit gate.
        if let Some(old) = self.item_reveal.take() {
            self.cancel_item_reveal_steps_at(&old.steps, &old_routes);
        }
        for step in &mut navigation.steps {
            step.generation = self
                .root
                .find(step.id)
                .unwrap()
                .component::<ScrollContent>()
                .unwrap()
                .scroll
                .reveal_identity();
        }
        navigation.result.changed = outcome.changed;
        // A tall target may intentionally be only partly visible after Start
        // alignment. The original reading API reports actual limit/shape clamps,
        // while the Host result retains its separate partial-visibility signal.
        navigation.result.clamped = outcome.limit_clamped || outcome.shape_clamped;
        if !outcome.animated {
            navigation.result.status = ContentRevealStatus::Completed;
        }
        let result = navigation.result.clone();
        self.item_reveal = Some(navigation);
        self.dirty = true;
        text.request_animation_frame();
        Ok(result)
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
