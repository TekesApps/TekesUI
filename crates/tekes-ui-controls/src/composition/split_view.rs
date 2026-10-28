//! Finite owned panes over the original SplitView model, View and Host owners.
use super::prepared::{self, TreePlan};
use super::*;
use crate::{
    extended::{Pane, SplitView},
    layout_view::{SplitViewLayout, SplitViewView},
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum SplitViewError {
    Composition(CompositionError),
    Effects(PreparedEffectsError),
    InvalidConfig,
    InvalidAllocation,
    Structure,
    Budget,
    ResourceCapacity,
    InvalidTarget,
    StaleExpected,
    Marked,
    UnsupportedScope,
    UnsupportedRestoreReveal,
}
impl From<PreparedEffectsError> for SplitViewError {
    fn from(e: PreparedEffectsError) -> Self {
        Self::Effects(e)
    }
}
/// Every refusal owns the COMPLETE original request; formatting never inspects it.
pub struct Rejected<I> {
    pub error: SplitViewError,
    pub input: Box<I>,
}
impl<I> std::fmt::Debug for Rejected<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rejected")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitViewConfig {
    pub axis: Axis,
    pub ratio: f32,
    pub divider: f32,
    pub min_pane: f32,
    pub collapsed: Option<Pane>,
}
impl Default for SplitViewConfig {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            ratio: 0.5,
            divider: 1.0,
            min_pane: 0.0,
            collapsed: None,
        }
    }
}
impl SplitViewConfig {
    fn valid(self) -> bool {
        self.ratio.is_finite()
            && (0.0..=1.0).contains(&self.ratio)
            && [self.divider, self.min_pane]
                .iter()
                .all(|n| n.is_finite() && (0.0..=1e8).contains(n))
    }
    fn model(self) -> SplitView {
        let mut model = SplitView::new(self.ratio);
        model.divider = self.divider;
        model.min_pane = self.min_pane;
        model.collapsed = self.collapsed;
        model
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SplitViewSlotIds {
    pub leading: ComponentId,
    pub trailing: ComponentId,
    pub divider: ComponentId,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitViewExpected {
    pub config: SplitViewConfig,
    pub mount: u64,
    pub revision: u64,
    pub leading_mount: u64,
    pub trailing_mount: u64,
}
#[derive(Debug)]
pub struct SplitViewConstruction {
    pub config: SplitViewConfig,
    pub slots: SplitViewSlotIds,
    pub leading: Content,
    pub trailing: Content,
    pub budget: usize,
}
#[derive(Debug)]
pub struct SplitViewReplacement {
    pub owner: ComponentId,
    pub expected: SplitViewExpected,
    pub pane: Pane,
    pub replacement: Content,
    pub budget: usize,
}
#[derive(Debug)]
struct PaneSlot<const LEADING: bool> {
    owner: Rc<()>,
    children: Vec<Content>,
    id: ComponentId,
    mount: u64,
    frame: Rect,
    clip: Rect,
    arranged: bool,
}
impl<const LEADING: bool> PaneSlot<LEADING> {
    fn intact(&self) -> bool {
        self.children.len() == 1
            && self.children[0].id == self.id
            && self.children[0].mount == self.mount
    }
}
impl<const LEADING: bool> Component for PaneSlot<LEADING> {
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged = false;
    }

    // These passive path owners have no local action reducer or action callbacks.
    // Their original mutable child access/focus hooks satisfy main #98's route contract.
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.arranged && self.intact()
    }

    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, h))
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.intact() {
            Ok(())
        } else {
            Err(CompositionError::InvalidSlotCount)
        }
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn participates_in_focus_traversal(&self) -> bool {
        self.intact() && !self.frame.is_empty()
    }
    fn semantic_subtree_hidden(&self) -> bool {
        !self.intact() || self.frame.is_empty()
    }
    fn semantic_hidden(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged = false;
        self.frame = frame;
        self.clip = clip;
        if self.intact() {
            let child = &mut self.children[0];
            child.measure(text, frame.size);
            child.arrange(text, frame, clip);
            self.arranged = valid_allocation(frame) && valid_allocation(clip);
        }
    }
}
#[derive(Debug)]
struct OriginalDivider {
    owner: Rc<()>,
    model: SplitView,
    view: SplitViewView,
    axis: Axis,
    focused: bool,
}
#[derive(Clone, Debug, PartialEq)]
struct PreparedSplitterAction {
    expected_ratio: f32,
    expected_model_ratio: f32,
    minimum: f64,
    maximum: f64,
    next_ratio: f32,
}
impl action_preparation::OriginalActionDecision for PreparedSplitterAction {}
impl OriginalDivider {
    fn semantic_ratio_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        let bounds = self.view.layout().frame.frame();
        if self.model.is_constrained(bounds, self.axis) {
            return None;
        }
        let available = match self.axis {
            Axis::Horizontal => bounds.size.width,
            Axis::Vertical => bounds.size.height,
        }
        .max(0.0);
        let usable = (available - self.model.divider.max(0.0).min(available)).max(0.0);
        if usable <= 0.0 {
            return None;
        }
        let minimum = if usable >= 2.0 * self.model.min_pane && usable > 0.0 {
            (self.model.min_pane / usable).clamp(0.0, 0.5)
        } else {
            0.5
        };
        // Publish the same decimal values used by `f32::to_string()` below.
        // Parsing the exact f32 bit pattern as f64 would make the wire value
        // "0.1" fall just outside a lower bound such as f32(0.1).
        let maximum = 1.0 - minimum;
        let wire_number = |value: f32| value.to_string().parse::<f64>().ok();
        Some(wire_number(minimum)?..=wire_number(maximum)?)
    }
    fn decide_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedSplitterAction> {
        let action_preparation::OwnedActionRequest::Accessibility(None, action) = request else {
            return None;
        };
        if self.semantic_hidden() {
            return None;
        }
        let range = self.semantic_ratio_range()?;
        let bounds = self.view.layout().frame.frame();
        let expected_model_ratio = self.model.ratio();
        let expected_ratio = self.model.ratio_in(bounds, self.axis);
        let next_ratio = match action {
            AccessibilityAction::SetValue(value) => {
                let value = value.parse::<f64>().ok()?;
                if !value.is_finite() || !range.contains(&value) {
                    return None;
                }
                value as f32
            }
            AccessibilityAction::Increment => {
                (expected_ratio + 0.05).clamp(*range.start() as f32, *range.end() as f32)
            }
            AccessibilityAction::Decrement => {
                (expected_ratio - 0.05).clamp(*range.start() as f32, *range.end() as f32)
            }
            _ => return None,
        };
        Some(PreparedSplitterAction {
            expected_ratio,
            expected_model_ratio,
            minimum: *range.start(),
            maximum: *range.end(),
            next_ratio,
        })
    }
    fn apply_action(&mut self, action: PreparedSplitterAction) {
        let _ = self.model.set_ratio(action.next_ratio);
    }
}
impl Component for OriginalDivider {
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(
            self,
            request,
            Self::decide_action,
            Self::apply_action,
            |decision| {
                (
                    ContentResponse::consumed(decision.next_ratio != decision.expected_model_ratio),
                    PreparedActionFocus::None,
                )
            },
        )
    }
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        Ok(PreparedEffects::local(self, h))
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .paint(s, &self.model, theme.palette, t.backing_scale());
    }
    fn interactive(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        !self.view.layout().frame.frame().is_empty()
    }
    fn set_focus(&mut self, f: bool, _: bool) {
        self.focused = f;
    }
    fn window_active_changed(&mut self, active: bool) {
        self.view.set_window_active(active, &mut self.model);
    }
    fn semantic_hidden(&self) -> bool {
        self.model
            .is_constrained(self.view.layout().frame.frame(), self.axis)
            || self
                .view
                .layout()
                .divider
                .visible_frame()
                .is_none_or(|r| r.is_empty())
    }
    fn semantic_frame(&self, _: Rect) -> Rect {
        self.view.layout().divider.frame()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Splitter, "Split view divider");
        semantics.value = Some(
            self.model
                .ratio_in(self.view.layout().frame.frame(), self.axis)
                .to_string(),
        );
        semantics
    }
    fn value_range(&self) -> Option<std::ops::RangeInclusive<f64>> {
        self.semantic_ratio_range()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let request = action_preparation::OwnedActionRequest::Accessibility(part, action.clone());
        let Some(decision) = Self::decide_action(self, &request) else {
            return ContentResponse::default();
        };
        let response =
            ContentResponse::consumed(decision.next_ratio != decision.expected_model_ratio);
        Self::apply_action(self, decision);
        response
    }
    fn pointer_event(&mut self, p: &PointerEvent) -> ContentResponse {
        if !p.is_primary() {
            return ContentResponse::default();
        }
        if p.phase == PointerPhase::Down && p.click_count >= 2 {
            return ContentResponse::consumed(self.view.double_click(p.position, &mut self.model));
        }
        self.event(&p.routing_event())
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        match e {
            ContentEvent::PointerDown(p) => {
                let accepted = self.view.pointer_down(*p, &mut self.model);
                ContentResponse {
                    consumed: accepted,
                    changed: accepted,
                    capture_pointer: accepted,
                    ..Default::default()
                }
            }
            ContentEvent::PointerMove(p) => {
                ContentResponse::consumed(self.view.pointer_move(*p, &mut self.model))
            }
            ContentEvent::PointerUp(_) => {
                ContentResponse::consumed(self.view.pointer_up(&mut self.model))
            }
            ContentEvent::Cancel => ContentResponse::consumed(self.view.cancel(&mut self.model)),
            ContentEvent::Key {
                key: ControlKey::Tab { .. },
                ..
            } => ContentResponse::default(),
            ContentEvent::Key { key, down: true } => {
                let changed = self.view.key(*key, &mut self.model);
                ContentResponse {
                    consumed: changed,
                    changed,
                    ..Default::default()
                }
            }
            _ => ContentResponse::default(),
        }
    }
}
#[derive(Debug)]
pub struct SplitViewContent {
    children: Vec<Content>,
    owner: Rc<()>,
    ids: SplitViewSlotIds,
    revision: u64,
    allocation: Option<(Rect, Rect)>,
    arranged_config: Option<SplitViewConfig>,
    native_owner: Option<ComponentId>,
    native_capture: Option<ComponentId>,
    retired_capture: bool,
}
fn bounded(c: &Content, budget: usize) -> Result<usize, SplitViewError> {
    let n = prepared::count(c).ok_or(SplitViewError::Budget)?;
    if n.checked_mul(n)
        .and_then(|v| v.checked_mul(256))
        .is_none_or(|n| n > budget)
    {
        Err(SplitViewError::Budget)
    } else {
        Ok(n)
    }
}
impl SplitViewContent {
    pub fn try_new(input: SplitViewConstruction) -> Result<Self, Rejected<SplitViewConstruction>> {
        let check = || -> Result<(), SplitViewError> {
            if !input.config.valid() {
                return Err(SplitViewError::InvalidConfig);
            }
            let n = prepared::count(&input.leading)
                .and_then(|n| n.checked_add(prepared::count(&input.trailing)?))
                .and_then(|n| n.checked_add(3))
                .ok_or(SplitViewError::Budget)?;
            let data = prepared::data_work(&input.leading, 16)?
                .checked_add(prepared::data_work(&input.trailing, 16)?)
                .ok_or(SplitViewError::Budget)?;
            if n.checked_mul(n)
                .and_then(|v| v.checked_mul(512))
                .and_then(|v| v.checked_add(data))
                .is_none_or(|v| v > input.budget)
            {
                return Err(SplitViewError::Budget);
            }
            let mut ids = BTreeSet::new();
            for id in [
                input.slots.leading,
                input.slots.trailing,
                input.slots.divider,
            ] {
                if !ids.insert(id) {
                    return Err(SplitViewError::Composition(CompositionError::DuplicateId(
                        id,
                    )));
                }
            }
            input
                .leading
                .ids(&mut ids)
                .map_err(SplitViewError::Composition)?;
            input
                .trailing
                .ids(&mut ids)
                .map_err(SplitViewError::Composition)?;
            fn admit(c: &Content) -> Result<(), SplitViewError> {
                c.prepare_node(&[PreparedHook::retained(PreparedHookKind::ChildrenAccess)])?;
                for child in c.component.children() {
                    admit(child)?;
                }
                Ok(())
            }
            admit(&input.leading)?;
            admit(&input.trailing)?;
            Ok(())
        };
        if let Err(error) = check() {
            return Err(Rejected {
                error,
                input: Box::new(input),
            });
        }
        let Some(mut range) = actions::reserve_mounts(3) else {
            return Err(Rejected {
                error: SplitViewError::ResourceCapacity,
                input: Box::new(input),
            });
        };
        let SplitViewConstruction {
            config,
            slots,
            leading,
            trailing,
            ..
        } = input;
        let owner = Rc::new(());
        let lead = PaneSlot::<true> {
            id: leading.id,
            mount: leading.mount,
            owner: owner.clone(),
            children: vec![leading],
            frame: Rect::default(),
            clip: Rect::default(),
            arranged: false,
        };
        let trail = PaneSlot::<false> {
            id: trailing.id,
            mount: trailing.mount,
            owner: owner.clone(),
            children: vec![trailing],
            frame: Rect::default(),
            clip: Rect::default(),
            arranged: false,
        };
        let divider = OriginalDivider {
            owner: owner.clone(),
            model: config.model(),
            view: SplitViewView::new(config.axis),
            axis: config.axis,
            focused: false,
        };
        Ok(Self {
            children: vec![
                Content::new_with_mount(slots.leading, lead, range.take()),
                Content::new_with_mount(slots.trailing, trail, range.take()),
                Content::new_with_mount(slots.divider, divider, range.take()).preserving_focus(),
            ],
            owner,
            ids: slots,
            revision: 0,
            allocation: None,
            arranged_config: None,
            native_owner: None,
            native_capture: None,
            retired_capture: false,
        })
    }
    fn intact(&self) -> bool {
        if self.children.len() != 3 {
            return false;
        }
        let Some(a) = self.children[0].component::<PaneSlot<true>>() else {
            return false;
        };
        let Some(b) = self.children[1].component::<PaneSlot<false>>() else {
            return false;
        };
        let Some(d) = self.children[2].component::<OriginalDivider>() else {
            return false;
        };
        self.children[0].id == self.ids.leading
            && self.children[1].id == self.ids.trailing
            && self.children[2].id == self.ids.divider
            && Rc::ptr_eq(&self.owner, &a.owner)
            && Rc::ptr_eq(&self.owner, &b.owner)
            && Rc::ptr_eq(&self.owner, &d.owner)
            && a.intact()
            && b.intact()
    }
    pub fn layout(&self) -> Option<SplitViewLayout> {
        self.intact().then(|| {
            self.children[2]
                .component::<OriginalDivider>()
                .unwrap()
                .view
                .layout()
        })
    }
    pub fn config(&self) -> Option<SplitViewConfig> {
        if !self.intact() {
            return None;
        }
        let d = self.children[2].component::<OriginalDivider>()?;
        Some(SplitViewConfig {
            axis: d.axis,
            ratio: d.model.ratio(),
            divider: d.model.divider,
            min_pane: d.model.min_pane,
            collapsed: d.model.collapsed,
        })
    }
    fn pane(&self, side: Pane) -> &Content {
        match side {
            Pane::Leading => {
                &self.children[0]
                    .component::<PaneSlot<true>>()
                    .unwrap()
                    .children[0]
            }
            Pane::Trailing => {
                &self.children[1]
                    .component::<PaneSlot<false>>()
                    .unwrap()
                    .children[0]
            }
        }
    }
    fn pane_vec(&mut self, side: Pane) -> &mut Vec<Content> {
        match side {
            Pane::Leading => {
                &mut self.children[0]
                    .component_mut::<PaneSlot<true>>()
                    .unwrap()
                    .children
            }
            Pane::Trailing => {
                &mut self.children[1]
                    .component_mut::<PaneSlot<false>>()
                    .unwrap()
                    .children
            }
        }
    }
}
impl Component for SplitViewContent {
    fn lifecycle(&mut self, _: Lifecycle) {
        self.arranged_config = None;
    }

    // These passive path owners have no local action reducer or action callbacks.
    // Their original mutable child access/focus hooks satisfy main #98's route contract.
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::ignored_pointer(request)
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.arranged_config.is_some() && self.arranged_config == self.config()
    }

    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        if !self.intact() {
            return Err(PreparedEffectsError::Unavailable);
        }
        Ok(PreparedEffects::local(self, h))
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if self.intact() {
            Ok(())
        } else {
            Err(CompositionError::InvalidSlotCount)
        }
    }
    fn active(&self) -> bool {
        self.intact()
    }
    fn semantic_subtree_hidden(&self) -> bool {
        self.layout()
            .is_none_or(|layout| layout.frame.visible_frame().is_none_or(|r| r.is_empty()))
    }
    fn fixed_slots(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged_config = None;
        if !self.intact() {
            return;
        }
        let focus_side = self.native_owner.and_then(|id| {
            if self.pane(Pane::Leading).find(id).is_some() {
                Some(Pane::Leading)
            } else if self.pane(Pane::Trailing).find(id).is_some() {
                Some(Pane::Trailing)
            } else {
                None
            }
        });
        let old_layout = self.layout();
        let capture = self.native_capture;
        let capture_side = capture.and_then(|id| {
            if self.pane(Pane::Leading).find(id).is_some() {
                Some(Pane::Leading)
            } else if self.pane(Pane::Trailing).find(id).is_some() {
                Some(Pane::Trailing)
            } else {
                None
            }
        });
        self.retired_capture = self.allocation.is_some_and(|v| v != (frame, clip))
            && capture == Some(self.ids.divider);
        let d = self.children[2].component_mut::<OriginalDivider>().unwrap();
        if self.allocation.is_some_and(|v| v != (frame, clip)) {
            d.view.cancel(&mut d.model);
        }
        if d.model.is_constrained(frame, d.axis)
            && let Some(side) = focus_side
        {
            d.model.choose_visible(side);
        }
        let layout = d.view.arrange(&mut d.model, frame, Some(clip));
        self.allocation = Some((frame, clip));
        if let Some(old) = old_layout {
            self.retired_capture |= match capture_side {
                Some(Pane::Leading) => old.leading != layout.leading,
                Some(Pane::Trailing) => old.trailing != layout.trailing,
                None => false,
            };
        }
        self.children[0].arrange(
            text,
            layout.leading.frame(),
            layout.leading.visible_frame().unwrap_or_default(),
        );
        self.children[1].arrange(
            text,
            layout.trailing.frame(),
            layout.trailing.visible_frame().unwrap_or_default(),
        );
        let hit = layout.divider_hit.unwrap_or_default();
        self.children[2].arrange(
            text,
            if hit.is_empty() { frame } else { hit },
            if hit.is_empty() {
                Rect::default()
            } else {
                clip
            },
        );
        self.arranged_config = if valid_allocation(frame) && valid_allocation(clip) {
            self.config()
        } else {
            None
        };
    }
}
// SplitView's original parent layout clips each Slot before arranging the Slot
// itself. This is library-owned owner geometry, not a custom descendant clip.
// Preserve that published owner allocation and read its full pane boundary.
pub(super) fn reveal_pane_boundary(node: &Content) -> Option<Rect> {
    if let Some(slot) = node.component::<PaneSlot<true>>() {
        return (slot.arranged && slot.intact()).then_some(slot.frame);
    }
    let slot = node.component::<PaneSlot<false>>()?;
    (slot.arranged && slot.intact()).then_some(slot.frame)
}
pub(super) fn refresh_local(c: &mut Content) {
    if let Some(s) = c.component_mut::<SplitViewContent>()
        && s.children.len() == 3
    {
        if let Some(p) = s.children[0].component_mut::<PaneSlot<true>>()
            && p.children.len() == 1
        {
            p.id = p.children[0].id;
            p.mount = p.children[0].mount;
        }
        if let Some(p) = s.children[1].component_mut::<PaneSlot<false>>()
            && p.children.len() == 1
        {
            p.id = p.children[0].id;
            p.mount = p.children[0].mount;
        }
    }
}
pub(super) fn refresh_split_baselines(c: &mut Content) {
    if let Some(ch) = c.component.children_mut() {
        for c in ch {
            refresh_split_baselines(c);
        }
    }
    refresh_local(c);
}
pub(super) fn valid_allocation(r: Rect) -> bool {
    [
        r.origin.x,
        r.origin.y,
        r.size.width,
        r.size.height,
        r.max_x(),
        r.max_y(),
    ]
    .iter()
    .all(|n| n.is_finite())
        && (0.0..=1e8).contains(&r.size.width)
        && (0.0..=1e8).contains(&r.size.height)
}

pub(super) struct PaneReplacementPlan {
    owner: Vec<usize>,
    old: Vec<usize>,
    effects: TreePlan,
    new: TreePlan,
    pre: usize,
    end: usize,
    modal: Option<Vec<usize>>,
    cancel_end: usize,
    restore: Option<(ComponentId, content_reveal_plan::HostRevealPlan)>,
    old_focus: bool,
    old_capture: bool,
}

impl ContentHost {
    pub fn split_view_expected(&self, id: ComponentId) -> Option<SplitViewExpected> {
        let node = self.root.find(id)?;
        let s = node.component::<SplitViewContent>()?;
        if !s.intact() {
            return None;
        }
        Some(SplitViewExpected {
            config: s.config()?,
            mount: node.mount,
            revision: s.revision,
            leading_mount: s.pane(Pane::Leading).mount,
            trailing_mount: s.pane(Pane::Trailing).mount,
        })
    }
    /// Synchronous replacement. All rejection paths retain the complete request.
    pub fn replace_split_view_pane(
        &mut self,
        input: SplitViewReplacement,
    ) -> Result<Content, Rejected<SplitViewReplacement>> {
        let plan = match self.prepare_split_view_replacement(&input) {
            Ok(p) => p,
            Err(error) => {
                return Err(Rejected {
                    error,
                    input: Box::new(input),
                });
            }
        };
        self.commit_split_view_replacement(input, plan)
    }
    pub(super) fn prepare_split_view_replacement(
        &self,
        input: &SplitViewReplacement,
    ) -> Result<PaneReplacementPlan, SplitViewError> {
        let host = self;
        if host.reading_transaction {
            return Err(SplitViewError::Composition(
                CompositionError::ReadingTransactionInProgress,
            ));
        }
        let total = prepared::count(&host.root)
            .and_then(|n| n.checked_add(prepared::count(&input.replacement)?))
            .ok_or(SplitViewError::Budget)?;
        let data = prepared::data_work(&host.root, 32)?
            .checked_add(prepared::data_work(&input.replacement, 32)?)
            .ok_or(SplitViewError::Budget)?;
        if total
            .checked_mul(total)
            .and_then(|n| n.checked_mul(512))
            .and_then(|n| n.checked_add(data))
            .is_none_or(|n| n > input.budget)
        {
            return Err(SplitViewError::Budget);
        }
        let owner =
            prepared::path_to(&host.root, input.owner).ok_or(SplitViewError::InvalidTarget)?;
        let node = prepared::at(&host.root, &owner).unwrap();
        let s = node
            .component::<SplitViewContent>()
            .ok_or(SplitViewError::InvalidTarget)?;
        if !s.intact() {
            return Err(SplitViewError::Structure);
        }
        if host.split_view_expected(input.owner) != Some(input.expected) {
            return Err(SplitViewError::StaleExpected);
        }
        if s.revision == u64::MAX {
            return Err(SplitViewError::ResourceCapacity);
        }
        let oldpane = s.pane(input.pane);
        if oldpane.find_composing() {
            return Err(SplitViewError::Marked);
        }
        let mut old = owner.clone();
        old.push(if input.pane == Pane::Leading { 0 } else { 1 });
        old.push(0);
        host.root
            .ids(&mut BTreeSet::new())
            .map_err(SplitViewError::Composition)?;
        let mut ids = BTreeSet::new();
        fn outside(
            c: &Content,
            path: &[usize],
            skip: &[usize],
            ids: &mut BTreeSet<ComponentId>,
        ) -> Result<(), SplitViewError> {
            if path == skip {
                return Ok(());
            }
            c.component
                .validate()
                .map_err(SplitViewError::Composition)?;
            if !ids.insert(c.id) {
                return Err(SplitViewError::Composition(CompositionError::DuplicateId(
                    c.id,
                )));
            }
            for (i, child) in c.component.children().iter().enumerate() {
                let mut p = path.to_vec();
                p.push(i);
                outside(child, &p, skip, ids)?;
            }
            Ok(())
        }
        outside(&host.root, &[], &old, &mut ids)?;
        input
            .replacement
            .ids(&mut ids)
            .map_err(SplitViewError::Composition)?;
        // An ancestor reading document whose ordered items are being replaced
        // requires its original typed document update, not a silent attachment edit.
        for depth in 0..owner.len() {
            if let Some(reader) = prepared::at(&host.root, &owner[..depth])
                .and_then(|c| c.component::<ScrollContent>())
                && reader
                    .reading_items()
                    .iter()
                    .any(|id| oldpane.find(*id).is_some())
            {
                return Err(SplitViewError::UnsupportedScope);
            }
        }
        let old_focus = host.focused.is_some_and(|id| oldpane.find(id).is_some());
        let old_capture = host.captured.is_some_and(|id| oldpane.find(id).is_some());
        let modal = host.modal.filter(|(id, _)| oldpane.find(*id).is_some());
        if !host.replacement_keeps_overlay_scope(oldpane, Some(&input.replacement)) {
            return Err(SplitViewError::UnsupportedScope);
        }
        let restore = if let Some((_, Some((id, mount)))) = modal {
            let target = host.root.find_active(id).filter(|c| {
                c.mount == mount && c.component.focusable() && !c.component.semantics().disabled
            });
            if target.is_some() {
                // A still-valid return owner in the retiring pane needs its own
                // restore-then-retire sequence; do not silently discard that intent.
                if oldpane.find(id).is_some() {
                    return Err(SplitViewError::UnsupportedRestoreReveal);
                }
                Some((
                    id,
                    host.plan_focus_reveal_after_modal_close(id)
                        .map_err(|_| SplitViewError::UnsupportedRestoreReveal)?,
                ))
            } else {
                None
            }
        } else {
            None
        };
        let focus_steps =
            u64::from(modal.is_some() && host.focused.is_some()) + u64::from(restore.is_some());
        if host.focus_epoch.checked_add(focus_steps).is_none() {
            return Err(SplitViewError::ResourceCapacity);
        }
        let mut effects = TreePlan::default();
        for depth in 0..old.len() {
            effects.add(
                &host.root,
                &old[..depth],
                PreparedHook::retained(PreparedHookKind::ChildrenAccess),
            )?;
        }
        let modal_path = if let Some((id, _)) = modal {
            effects.cancel(&host.root, &[])?;
            if let Some(focused) = host.focused {
                let p =
                    prepared::path_to(&host.root, focused).ok_or(SplitViewError::InvalidTarget)?;
                effects.add(
                    &host.root,
                    &p,
                    PreparedHook::retained(PreparedHookKind::Focus {
                        focused: false,
                        visible: false,
                    }),
                )?;
            }
            Some(prepared::path_to(&host.root, id).unwrap())
        } else {
            if old_capture {
                let p = prepared::path_to(&host.root, host.captured.unwrap()).unwrap();
                effects.add(
                    &host.root,
                    &p,
                    PreparedHook::retained(PreparedHookKind::CancelPointer),
                )?;
            }
            None
        };
        // Replacing either pane retires a captured original divider gesture.
        if host.captured == Some(s.ids.divider) {
            let mut p = owner.clone();
            p.push(2);
            effects.add(
                &host.root,
                &p,
                PreparedHook::retained(PreparedHookKind::CancelPointer),
            )?;
        }
        let cancel_end = effects.len();
        if let Some((id, _)) = &restore {
            let p = prepared::path_to(&host.root, *id).ok_or(SplitViewError::InvalidTarget)?;
            effects.add(
                &host.root,
                &p,
                PreparedHook::retained(PreparedHookKind::Focus {
                    focused: true,
                    visible: true,
                }),
            )?;
        }
        let pre = effects.len();
        effects.subtree(&host.root, &old, Lifecycle::Unmounted)?;
        let end = effects.len();
        fn withdrawals(
            plan: &mut TreePlan,
            root: &Content,
            c: &Content,
            path: &[usize],
            skip: &[usize],
            parent: bool,
        ) -> Result<(), PreparedEffectsError> {
            if path == skip {
                return Ok(());
            }
            let active = parent && c.active();
            if !active || c.visible().is_none() {
                plan.add(
                    root,
                    path,
                    PreparedHook::retained(PreparedHookKind::PresentationRunnable(false)),
                )?;
            }
            for (i, child) in c.component.children().iter().enumerate() {
                let mut p = path.to_vec();
                p.push(i);
                withdrawals(plan, root, child, &p, skip, active)?;
            }
            Ok(())
        }
        withdrawals(&mut effects, &host.root, &host.root, &[], &old, true)?;
        effects.prepare(&host.root)?;
        let mut new = TreePlan::default();
        new.subtree(&input.replacement, &[], Lifecycle::Mounted)?;
        let slot = prepared::at(&host.root, &old[..old.len() - 1]).unwrap();
        new.activity(
            &input.replacement,
            &[],
            slot.activity_parent && slot.active() && !slot.component.semantics().disabled,
        )?;
        // Newly mounted input starts with empty allocation; withdraw until layout.
        fn withdraw_new(
            plan: &mut TreePlan,
            root: &Content,
            c: &Content,
            p: &[usize],
        ) -> Result<(), PreparedEffectsError> {
            plan.add(
                root,
                p,
                PreparedHook::retained(PreparedHookKind::PresentationRunnable(false)),
            )?;
            for (i, ch) in c.component.children().iter().enumerate() {
                let mut next = p.to_vec();
                next.push(i);
                withdraw_new(plan, root, ch, &next)?;
            }
            Ok(())
        }
        withdraw_new(&mut new, &input.replacement, &input.replacement, &[])?;
        new.prepare(&input.replacement)?;
        Ok(PaneReplacementPlan {
            owner,
            old,
            effects,
            new,
            pre,
            end,
            modal: modal_path,
            cancel_end,
            restore,
            old_focus,
            old_capture,
        })
    }
    /// Synchronous final boundary: ownership remains borrowed until every reservation/recheck succeeds.
    pub(super) fn commit_split_view_replacement(
        &mut self,
        input: SplitViewReplacement,
        mut plan: PaneReplacementPlan,
    ) -> Result<Content, Rejected<SplitViewReplacement>> {
        let mut input = input;
        if let Err(error) = plan
            .effects
            .mutable_boundaries(&mut self.root)
            .and_then(|_| plan.new.mutable_boundaries(&mut input.replacement))
        {
            return Err(Rejected {
                error: error.into(),
                input: Box::new(input),
            });
        }
        let mut reading = match if plan.effects.readers() + plan.new.readers() == 0 {
            Ok(crate::reading::AttachmentRange::empty())
        } else {
            crate::reading::reserve_attachments(plan.effects.readers() + plan.new.readers())
        } {
            Ok(r) => r,
            Err(_) => {
                return Err(Rejected {
                    error: SplitViewError::ResourceCapacity,
                    input: Box::new(input),
                });
            }
        };
        let Some(mut mounts) = actions::reserve_mounts(plan.effects.mounts + plan.new.mounts)
        else {
            return Err(Rejected {
                error: SplitViewError::ResourceCapacity,
                input: Box::new(input),
            });
        };
        if self.split_view_expected(input.owner) != Some(input.expected) {
            return Err(Rejected {
                error: SplitViewError::StaleExpected,
                input: Box::new(input),
            });
        }
        if let Err(e) = plan
            .effects
            .validate(&self.root)
            .and_then(|_| plan.new.validate(&input.replacement))
        {
            return Err(Rejected {
                error: e.into(),
                input: Box::new(input),
            });
        }
        if !prepared::at(&self.root, &plan.old).is_some_and(|retired| {
            self.replacement_keeps_overlay_scope(retired, Some(&input.replacement))
        }) {
            return Err(Rejected {
                error: SplitViewError::UnsupportedScope,
                input: Box::new(input),
            });
        }
        let restore_ready = if let Some((id, reveal)) = plan.restore.take() {
            if self
                .validate_host_reveal_in_scope(&reveal, &self.root)
                .is_err()
            {
                return Err(Rejected {
                    error: SplitViewError::UnsupportedRestoreReveal,
                    input: Box::new(input),
                });
            }
            match self.prepare_host_reveal_commit(reveal, 1) {
                Ok(ready) => Some((id, ready)),
                Err(_) => {
                    return Err(Rejected {
                        error: SplitViewError::ResourceCapacity,
                        input: Box::new(input),
                    });
                }
            }
        } else {
            None
        };
        plan.effects.bind(&mut reading);
        plan.new.bind(&mut reading);
        // Original Host publication authority expires only after all refusals.
        let _ = self.reveal_clock.geometry_lease.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| Some(n.saturating_add(1)),
        );
        self.reveal_clock.pending = true;
        // All ordinary refusal ends above. No callbacks, IDs or authorization were published.
        self.reading_transaction = true;
        plan.effects
            .commit_range(&mut self.root, &mut mounts, 0, plan.cancel_end, &[]);
        if let Some(path) = &plan.modal {
            self.reveal_clock.invalidate_input();
            self.text_keys.clear();
            self.help.clear();
            self.modal_focus_pending = false;
            self.captured = None;
            if self.focused.is_some() {
                self.focus_epoch += 1;
            }
            self.focused = None;
            prepared::at_mut(&mut self.root, path)
                .unwrap()
                .component_mut::<DialogContent>()
                .expect("original dialog owner")
                .model
                .close();
            self.modal = None;
        }
        if let Some((id, ready)) = restore_ready {
            self.commit_prepared_host_reveal(ready);
            self.focus_epoch += 1;
            self.focused = Some(id);
            plan.effects
                .commit_range(&mut self.root, &mut mounts, plan.cancel_end, plan.pre, &[]);
            self.help_focus(Some(id), true);
        }
        if plan.old_focus && plan.modal.is_none() {
            self.focused = None;
        }
        if plan.old_capture
            || self.captured
                == Some(
                    prepared::at(&self.root, &plan.owner)
                        .unwrap()
                        .component::<SplitViewContent>()
                        .unwrap()
                        .ids
                        .divider,
                )
        {
            self.captured = None;
            self.help.clear();
        }
        let owner = prepared::at_mut(&mut self.root, &plan.owner)
            .unwrap()
            .component_mut::<SplitViewContent>()
            .unwrap();
        let mut old = owner.pane_vec(input.pane).remove(0);
        plan.effects
            .commit_range(&mut old, &mut mounts, plan.pre, plan.end, &plan.old);
        refresh_split_baselines(&mut old);
        if self
            .presented_dialog
            .is_some_and(|id| old.find(id).is_some())
        {
            self.presented_dialog = None;
        }
        let mut replacement = input.replacement;
        plan.new.commit(&mut replacement, &mut mounts);
        refresh_split_baselines(&mut replacement);
        let owner = prepared::at_mut(&mut self.root, &plan.owner)
            .unwrap()
            .component_mut::<SplitViewContent>()
            .unwrap();
        owner.pane_vec(input.pane).push(replacement);
        owner.revision += 1;
        owner.arranged_config = None;
        match input.pane {
            Pane::Leading => {
                owner.children[0]
                    .component_mut::<PaneSlot<true>>()
                    .unwrap()
                    .arranged = false
            }
            Pane::Trailing => {
                owner.children[1]
                    .component_mut::<PaneSlot<false>>()
                    .unwrap()
                    .arranged = false
            }
        }
        refresh_local(prepared::at_mut(&mut self.root, &plan.owner).unwrap());
        plan.effects.commit_range(
            &mut self.root,
            &mut mounts,
            plan.end,
            plan.effects.len(),
            &[],
        );
        plan.effects.assert_consumed();
        plan.new.assert_consumed();
        self.reading_transaction = false;
        self.dirty = true;
        debug_assert!(mounts.exhausted());
        Ok(old)
    }
    /// Invalid RAW allocation refuses before any render/layout/text/scene effects.
    pub fn render_checked(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<(), SplitViewError> {
        if !valid_allocation(frame) || !valid_allocation(clip) {
            return Err(SplitViewError::InvalidAllocation);
        }
        self.root
            .ids(&mut BTreeSet::new())
            .map_err(SplitViewError::Composition)?;
        self.preflight_empty_state_structures()
            .map_err(SplitViewError::Composition)?;
        self.preflight_notice_structures()
            .map_err(SplitViewError::Composition)?;
        self.render(scene, text, frame, clip, palette)
            .map_err(SplitViewError::Composition)
    }
}
impl Content {
    fn find_composing(&self) -> bool {
        self.component.composing()
            || self
                .component
                .children()
                .iter()
                .any(Content::find_composing)
    }
}

impl ContentHost {
    /// Zero-cost Keep exits before hooks and both resource allocators.
    pub fn update_split_view(
        &mut self,
        id: ComponentId,
        expected: SplitViewExpected,
        config: SplitViewConfig,
        budget: usize,
    ) -> Result<bool, SplitViewError> {
        if self.reading_transaction {
            return Err(SplitViewError::Composition(
                CompositionError::ReadingTransactionInProgress,
            ));
        }
        if !config.valid() {
            return Err(SplitViewError::InvalidConfig);
        }
        let path = prepared::path_to(&self.root, id).ok_or(SplitViewError::InvalidTarget)?;
        let node = prepared::at(&self.root, &path).unwrap();
        let split = node
            .component::<SplitViewContent>()
            .ok_or(SplitViewError::InvalidTarget)?;
        if !split.intact() {
            return Err(SplitViewError::Structure);
        }
        if self.split_view_expected(id) != Some(expected) {
            return Err(SplitViewError::StaleExpected);
        }
        bounded(&self.root, budget)?;
        self.root
            .ids(&mut BTreeSet::new())
            .map_err(SplitViewError::Composition)?;
        if split.config() == Some(config) {
            return Ok(false);
        }
        if split.revision == u64::MAX {
            return Err(SplitViewError::ResourceCapacity);
        }
        let projected = config.model().layout(node.frame, config.axis);
        let hidden = [projected.0.is_empty(), projected.2.is_empty()];
        for (i, side) in [Pane::Leading, Pane::Trailing].into_iter().enumerate() {
            if hidden[i] && split.pane(side).find_composing() {
                return Err(SplitViewError::Marked);
            }
        }

        let old_layout = split.layout().expect("intact layout");
        for (side, bounds) in [(Pane::Leading, projected.0), (Pane::Trailing, projected.2)] {
            let old_bounds = if side == Pane::Leading {
                old_layout.leading.frame()
            } else {
                old_layout.trailing.frame()
            };
            if old_bounds != bounds && !self.replacement_keeps_overlay_scope(split.pane(side), None)
            {
                return Err(SplitViewError::UnsupportedScope);
            }
        }
        let divider_id = split.ids.divider;
        let mut effects = TreePlan::default();
        for depth in 0..=path.len() {
            effects.add(
                &self.root,
                &path[..depth],
                PreparedHook::retained(PreparedHookKind::ChildrenAccess),
            )?;
        }
        let mut divider_path = path.clone();
        divider_path.push(2);
        effects.add(
            &self.root,
            &divider_path,
            PreparedHook::retained(PreparedHookKind::CancelPointer),
        )?;
        let cancel_capture = if let Some(captured) = self.captured {
            if captured == divider_id {
                true
            } else {
                let old = split.layout().expect("intact layout");
                (split.pane(Pane::Leading).find(captured).is_some()
                    && old.leading.frame() != projected.0)
                    || (split.pane(Pane::Trailing).find(captured).is_some()
                        && old.trailing.frame() != projected.2)
            }
        } else {
            false
        };
        if cancel_capture
            && let Some(captured) = self.captured
            && captured != divider_id
        {
            let p = prepared::path_to(&self.root, captured).unwrap();
            effects.add(
                &self.root,
                &p,
                PreparedHook::retained(PreparedHookKind::CancelPointer),
            )?;
        }
        fn withdraw(
            p: &mut TreePlan,
            root: &Content,
            c: &Content,
            path: &[usize],
        ) -> Result<(), PreparedEffectsError> {
            p.add(
                root,
                path,
                PreparedHook::retained(PreparedHookKind::PresentationRunnable(false)),
            )?;
            for (i, ch) in c.component.children().iter().enumerate() {
                let mut next = path.to_vec();
                next.push(i);
                withdraw(p, root, ch, &next)?;
            }
            Ok(())
        }
        for (i, is_hidden) in hidden.into_iter().enumerate() {
            if is_hidden {
                let mut p = path.clone();
                p.push(i);
                withdraw(
                    &mut effects,
                    &self.root,
                    prepared::at(&self.root, &p).unwrap(),
                    &p,
                )?;
            }
        }
        effects.prepare(&self.root)?;
        effects.mutable_boundaries(&mut self.root)?;
        // There are no mount/lifecycle/attachment demands for configuration.
        let mut empty_reading = crate::reading::AttachmentRange::empty();
        let mut empty_mounts = actions::MountRange::empty();
        effects.validate(&self.root)?;
        effects.bind(&mut empty_reading);
        // No Keep/refusal path reaches this infallible original lease revoke.
        let _ = self.reveal_clock.geometry_lease.fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| Some(n.saturating_add(1)),
        );
        self.reveal_clock.pending = true;
        self.reading_transaction = true;
        effects.commit(&mut self.root, &mut empty_mounts);
        if cancel_capture {
            self.captured = None;
            self.help.clear();
        }
        let node = prepared::at_mut(&mut self.root, &path).unwrap();
        let split = node.component_mut::<SplitViewContent>().unwrap();
        let d = split.children[2]
            .component_mut::<OriginalDivider>()
            .unwrap();
        if config.axis != d.axis {
            d.view = SplitViewView::new(config.axis);
            d.view.set_window_active(self.window_active, &mut d.model);
            d.axis = config.axis;
        }
        d.model.set_ratio(config.ratio);
        d.model.divider = config.divider;
        d.model.min_pane = config.min_pane;
        d.model.collapsed = config.collapsed;
        split.revision += 1;
        split.allocation = None;
        split.arranged_config = None;
        split.children[0]
            .component_mut::<PaneSlot<true>>()
            .unwrap()
            .arranged = false;
        split.children[1]
            .component_mut::<PaneSlot<false>>()
            .unwrap()
            .arranged = false;
        let mut retired = BTreeSet::new();
        for (i, is_hidden) in hidden.into_iter().enumerate() {
            if is_hidden {
                let slot = &mut split.children[i];
                if i == 0 {
                    slot.component_mut::<PaneSlot<true>>().unwrap().frame = Rect::default();
                } else {
                    slot.component_mut::<PaneSlot<false>>().unwrap().frame = Rect::default();
                }
                fn clear(c: &mut Content, ids: &mut BTreeSet<ComponentId>) {
                    ids.insert(c.id);
                    c.clip = Rect::default();
                    c.frame = Rect::default();
                    if let Some(children) = c.component.children_mut() {
                        for child in children {
                            clear(child, ids);
                        }
                    }
                }
                clear(slot, &mut retired);
            }
        }
        if let Some(actions) = &mut self.actions {
            actions.retain(|a| !retired.contains(&a.action.target));
        }
        self.reading_transaction = false;
        self.dirty = true;
        Ok(true)
    }
    pub(super) fn prepare_split_layout(&mut self, _frame: Rect, _clip: Rect) {
        fn collect(
            c: &Content,
            path: &mut Vec<usize>,
            out: &mut Vec<(Vec<usize>, ComponentId, u64)>,
        ) {
            if c.component::<SplitViewContent>().is_some() {
                out.push((path.clone(), c.id, c.mount));
            }
            for (i, child) in c.component.children().iter().enumerate() {
                path.push(i);
                collect(child, path, out);
                path.pop();
            }
        }
        let mut paths = vec![];
        collect(&self.root, &mut vec![], &mut paths);
        for (path, id, mount) in paths {
            if let Some(c) = prepared::at_mut(&mut self.root, &path)
                && c.id == id
                && c.mount == mount
                && let Some(s) = c.component_mut::<SplitViewContent>()
                && s.intact()
            {
                s.native_owner = self.focused;
                s.native_capture = self.captured;
                s.retired_capture = false;
            }
        }
    }
    pub(super) fn finish_split_layout(&mut self) {
        fn retired(c: &Content) -> bool {
            c.component::<SplitViewContent>()
                .is_some_and(|s| s.retired_capture)
                || c.component.children().iter().any(retired)
        }
        if retired(&self.root)
            && let Some(path) = self
                .captured
                .and_then(|id| prepared::path_to(&self.root, id))
        {
            if let Some(owner) = prepared::at_mut(&mut self.root, &path) {
                owner.component.cancel_pointer();
            }
            self.captured = None;
            self.help.clear();
        }
    }
    pub(super) fn split_structures_intact(&self) -> bool {
        fn check(c: &Content) -> bool {
            c.component::<SplitViewContent>()
                .is_none_or(SplitViewContent::intact)
                && c.component.children().iter().all(check)
        }
        check(&self.root)
    }
    pub(super) fn split_text_geometry_hidden(&self, id: ComponentId) -> bool {
        fn walk(c: &Content, id: ComponentId, hidden: bool) -> Option<bool> {
            let pane = c.component::<PaneSlot<true>>().is_some()
                || c.component::<PaneSlot<false>>().is_some();
            let hidden = hidden || (pane && c.visible().is_none());
            if c.id == id {
                return Some(hidden);
            }
            c.component
                .children()
                .iter()
                .find_map(|c| walk(c, id, hidden))
        }
        walk(&self.root, id, false).unwrap_or(false)
    }
    pub(super) fn split_ax_guard(&self, id: ComponentId) -> bool {
        self.is_composing()
            && self
                .root
                .find(id)
                .is_some_and(|c| c.component::<OriginalDivider>().is_some())
    }
    pub(super) fn split_input_guard(&mut self, event: &ContentEvent) -> bool {
        if matches!(event, ContentEvent::Preedit { .. })
            && let Some(id) = self.captured
            && self
                .root
                .find(id)
                .is_some_and(|c| c.component::<OriginalDivider>().is_some())
        {
            if let Some(p) = prepared::path_to(&self.root, id)
                && let Some(c) = prepared::at_mut(&mut self.root, &p)
            {
                c.component.cancel_pointer();
            }
            self.captured = None;
        }
        let routing = event.routing_event();
        if let ContentEvent::PointerDown(position) = routing.as_ref() {
            fn hit(c: &Content, p: Point) -> Option<ComponentId> {
                if !c.active() {
                    return None;
                }
                if c.component::<OriginalDivider>().is_some_and(|d| {
                    d.view
                        .resize_regions(&d.model)
                        .iter()
                        .any(|(r, _)| r.contains(p))
                }) {
                    return Some(c.id);
                }
                c.component.children().iter().rev().find_map(|c| hit(c, p))
            }
            if event.is_primary_pointer()
                && let Some(id) = hit(self.scope(), *position)
            {
                let double = matches!(event,ContentEvent::Pointer(p) if p.click_count>=2);
                if self.is_composing() || (!double && self.capture_epoch == u64::MAX) {
                    return true;
                }
                if double && self.captured == Some(id) {
                    self.captured = None;
                }
            }
        }
        false
    }
}
