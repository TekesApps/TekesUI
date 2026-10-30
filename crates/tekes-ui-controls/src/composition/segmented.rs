//! Fixed-label composition over the original Segmented model and retained View.
use super::*;
use crate::{extended::SegmentedControl, layout_view::SegmentedControlView};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentedContentError {
    LabelCount,
    InvalidSelection,
    InvalidIndex,
}
/// A delivered selection; authorization is tied to this original exposure,
/// independently of subsequent controlled selection updates.
#[derive(Clone, Debug)]
pub struct SegmentIntent {
    origin: Arc<()>,
    index: usize,
    token: usize,
}
impl PartialEq for SegmentIntent {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.origin, &other.origin)
            && self.index == other.index
            && self.token == other.token
    }
}
impl Eq for SegmentIntent {}
impl SegmentIntent {
    pub fn index(&self) -> usize {
        self.index
    }
}
#[derive(Debug)]
pub struct SegmentedControlContent {
    name: String,
    model: SegmentedControl,
    view: SegmentedControlView,
    labels: Vec<String>,
    compact: bool,
    disabled: bool,
    mounted: bool,
    active: bool,
    runnable: bool,
    // Fixed slots map checked exposure tickets to the original model indices.
    tokens: Vec<Option<usize>>,
    next_token: usize,
    pending: bool,
    origin: Arc<()>,
    last_intent: Option<SegmentIntent>,
    focus_receipt: bool,
}
#[derive(Debug, PartialEq)]
struct SegmentedActionDecision {
    origin: action_preparation::OwnerIdentity,
    model: SegmentedControl,
    state: crate::layout_view::SegmentedActionSnapshot,
    tokens: Vec<Option<usize>>,
    available: bool,
    last_intent: Option<SegmentIntent>,
    focus_receipt: bool,
    view: Option<crate::layout_view::SegmentedViewDecision>,
    response: ContentResponse,
    semantic: bool,
    intent: Option<SegmentIntent>,
}
impl action_preparation::OriginalActionDecision for SegmentedActionDecision {}
impl SegmentedControlContent {
    fn decide_action(&self, request: &action_preparation::OwnedActionRequest) -> Option<SegmentedActionDecision> {
        use action_preparation::OwnedActionRequest;
        let mut response = ContentResponse::default();
        let mut semantic = false;
        let mut intent = None;
        let mut view = None;
        match request {
            OwnedActionRequest::Pointer(ContentEvent::PointerDown(point)) => {
                if self.available() {
                    let target = self.view.layout().items.iter().position(|l| l.hit_test(*point));
                    if let Some(index) = target.filter(|&i| self.geometry_available(i) && self.tokens[i].is_some()) {
                        view = self.view.prepare_pointer_down(*point, self.view.focused() != Some(index));
                        response.consumed = view.is_some(); response.changed = response.consumed;
                        response.capture_pointer = response.consumed;
                    }
                }
            }
            OwnedActionRequest::Accessibility(part, action) => {
                if !matches!(action, AccessibilityAction::Focus | AccessibilityAction::Press) { return None; }
                if self.available()
                    && let Some(index) = part.and_then(|token| self.tokens.iter().position(|t| *t == Some(token)))
                    && self.geometry_available(index)
                {
                    view = match action {
                        AccessibilityAction::Focus => self.view.prepare_focus(index, true, self.view.focused() != Some(index)),
                        AccessibilityAction::Press => Some(self.view.prepare_select(index, &self.model)),
                        _ => unreachable!(),
                    };
                    response.consumed = view.is_some();
                    response.changed = match action {
                        AccessibilityAction::Focus => view.is_some() && self.view.focused() != Some(index),
                        _ => view.as_ref().is_some_and(|d| d.changed_selection()),
                    };
                    response.activated = matches!(action, AccessibilityAction::Press) && response.changed;
                    semantic = response.consumed;
                    if response.activated {
                        intent = Some(SegmentIntent { origin: self.origin.clone(), index,
                            token: self.tokens[index].expect("preflighted original part") });
                    }
                }
            }
            _ => return None,
        }
        Some(SegmentedActionDecision { origin: action_preparation::OwnerIdentity(self.origin.clone()),
            model: self.model.clone(), state: self.view.action_snapshot(), tokens: self.tokens.clone(),
            available: self.available(), last_intent: self.last_intent.clone(), focus_receipt: self.focus_receipt,
            view, response, semantic, intent })
    }
    fn apply_action(&mut self, decision: SegmentedActionDecision) {
        self.begin_delivery();
        if let Some(view) = decision.view { self.view.apply_decision(view, &mut self.model); }
        self.last_intent = decision.intent;
        self.focus_receipt = decision.semantic;
    }
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self {
        Self::from_model(name, SegmentedControl::new(labels.len()), labels)
            .expect("matching fixed labels")
    }
    pub fn from_model(
        name: impl Into<String>,
        model: SegmentedControl,
        labels: Vec<String>,
    ) -> Result<Self, SegmentedContentError> {
        if labels.len() != model.len() {
            return Err(SegmentedContentError::LabelCount);
        }
        if !model.is_empty() && model.selected >= model.len() {
            return Err(SegmentedContentError::InvalidSelection);
        }
        let count = model.len();
        Ok(Self {
            name: name.into(),
            model,
            view: SegmentedControlView::new(count),
            labels,
            compact: false,
            disabled: false,
            mounted: false,
            active: true,
            runnable: false,
            tokens: vec![None; count],
            next_token: 0,
            pending: false,
            origin: Arc::new(()),
            last_intent: None,
            focus_receipt: false,
        })
    }
    pub fn model(&self) -> &SegmentedControl {
        &self.model
    }
    pub fn view(&self) -> &SegmentedControlView {
        &self.view
    }
    pub fn set_selected(&mut self, index: usize) -> Result<(), SegmentedContentError> {
        // Reject every invalid target before gesture, delivery or identity mutation.
        if index >= self.model.len() || !self.model.is_enabled(index) {
            return Err(SegmentedContentError::InvalidSelection);
        }
        if self.model.selected == index {
            return Ok(());
        }
        self.view.cancel();
        self.model.select(index);
        Ok(())
    }
    pub fn set_item_enabled(
        &mut self,
        index: usize,
        enabled: bool,
    ) -> Result<(), SegmentedContentError> {
        if index >= self.model.len() {
            return Err(SegmentedContentError::InvalidIndex);
        }
        if self.model.is_enabled(index) == enabled {
            return Ok(());
        }
        self.view.cancel();
        self.tokens[index] = None;
        self.model.set_enabled(index, enabled);
        if self
            .view
            .focused()
            .is_some_and(|i| !self.model.is_enabled(i))
        {
            self.view.blur();
        }
        // Re-exposure requires the next successful original arrangement.
        Ok(())
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        self.view.set_disabled(disabled);
        if disabled {
            self.revoke();
        }
    }
    pub fn set_compact(&mut self, compact: bool) {
        self.compact = compact;
    }
    /// Current semantic exposure for a fixed logical slot. Host adapters use
    /// this read-only query instead of treating a business/navigation index as
    /// an authorization token. Hidden, disabled and pending slots return None.
    pub fn part_token(&self, index: usize) -> Option<usize> {
        if !self.available() || !self.geometry_available(index) {
            return None;
        }
        self.tokens.get(index).copied().flatten()
    }
    pub fn intent_is_current(&self, intent: &SegmentIntent) -> bool {
        self.available()
            && Arc::ptr_eq(&self.origin, &intent.origin)
            && self.tokens.get(intent.index) == Some(&Some(intent.token))
            && self.geometry_available(intent.index)
    }
    fn available(&self) -> bool {
        self.mounted && self.active && self.runnable && !self.disabled && !self.pending
    }
    fn geometry_available(&self, index: usize) -> bool {
        self.model.is_enabled(index)
            && self
                .view
                .layout()
                .items
                .get(index)
                .is_some_and(|l| l.visible.is_some_and(|r| !r.is_empty()))
    }
    fn revoke(&mut self) {
        self.tokens.fill(None);
        self.view.cancel();
        self.view.blur();
        self.focus_receipt = false;
    }
    fn publish(&mut self) {
        if !self.mounted || !self.active || !self.runnable || self.disabled {
            self.revoke();
            return;
        }
        // Withdraw old authority immediately, before trying to reserve the complete
        // new plan. No partial publication and no rollback/reuse/wrap of tickets.
        for i in 0..self.tokens.len() {
            if !self.geometry_available(i) {
                self.tokens[i] = None;
            }
        }
        let mut next = self.next_token;
        let mut plan = self.tokens.clone();
        for (i, slot) in plan.iter_mut().enumerate() {
            if self.geometry_available(i) && slot.is_none() {
                let Some(ticket) = next.checked_add(1) else {
                    self.pending = true;
                    self.revoke();
                    return;
                };
                next = ticket;
                *slot = Some(ticket);
            }
        }
        self.tokens = plan;
        self.next_token = next;
        self.pending = false;
    }
    fn begin_delivery(&mut self) {
        self.last_intent = None;
        self.focus_receipt = false;
    }
    fn delivered(&mut self, changed: bool) {
        if changed && let Some(token) = self.tokens.get(self.model.selected).copied().flatten() {
            self.last_intent = Some(SegmentIntent {
                origin: self.origin.clone(),
                index: self.model.selected,
                token,
            });
        }
    }
}
impl Component for SegmentedControlContent {
    // Hooks are owner-local: no child callback, tree mutation, queue insertion or
    // fallible work. set_focus only chooses original focus/cancels original arms;
    // a prepared decision establishes the original final state after those hooks.
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(self, request, Self::decide_action, Self::apply_action,
            |d| (d.response, if d.response.consumed { PreparedActionFocus::Owner } else { PreparedActionFocus::None }))
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        MeasuredChild::new(SegmentedControlView::measure(
            &mut TextAdapter(text, None, true),
            &labels,
            available,
            proposal::TextRole::UiBody,
            self.compact,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.view.arrange(
            &mut TextAdapter(text, None, true),
            &self.model,
            &labels,
            frame,
            Some(clip),
            proposal::TextRole::UiBody,
            self.compact,
        );
        // Retire per-part clipping even when the root remains runnable.
        let lost = self
            .tokens
            .iter()
            .enumerate()
            .any(|(i, t)| t.is_some() && !self.geometry_available(i));
        if lost {
            self.view.cancel();
        }
        self.publish();
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if !self.mounted || !self.active || !self.runnable {
            return;
        }
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Surface");
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.view.paint_arranged(
            scene,
            &mut TextAdapter(text, None, true),
            &self.model,
            &labels,
            Some(clip),
            theme.palette,
            proposal::TextRole::UiBody,
            self.compact,
        );
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.begin_delivery();
        self.revoke();
        self.mounted = event == Lifecycle::Mounted;
        self.runnable = false;
    }
    fn window_active_changed(&mut self, active: bool) {
        self.active = active;
        self.view.set_window_active(active);
        if !active {
            self.revoke();
        }
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.runnable = runnable;
        if runnable {
            self.publish();
        } else {
            self.revoke();
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::SegmentedGroup, &self.name);
        s.disabled = !self.available() || !(0..self.model.len()).any(|i| self.model.is_enabled(i));
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        if !self.available() {
            return vec![];
        }
        self.tokens
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let token = (*t)?;
                let frame = self.view.layout().items.get(i)?.visible?;
                let mut s = ContentSemantics::new(ContentRole::Segment, &self.labels[i]);
                s.selected = self.model.selected == i;
                Some(SemanticPart {
                    index: token,
                    frame,
                    focused: self.view.focused() == Some(i),
                    semantics: s,
                })
            })
            .collect()
    }
    fn focusable(&self) -> bool {
        self.available() && self.tokens.iter().any(Option::is_some)
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if !focused {
            self.view.blur();
            return;
        }
        if !self.available() {
            return;
        }
        if let Some(i) = self
            .view
            .focused()
            .filter(|&i| self.geometry_available(i))
            .or_else(|| {
                self.geometry_available(self.model.selected)
                    .then_some(self.model.selected)
            })
            .or_else(|| (0..self.model.len()).find(|&i| self.geometry_available(i)))
        {
            self.view.focus_with_visibility(i, visible);
        }
    }
    fn accessibility_focus_requested(&self) -> bool {
        self.focus_receipt
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let request = action_preparation::OwnedActionRequest::Accessibility(part, action.clone());
        let Some(decision) = self.decide_action(&request) else {
            self.begin_delivery(); return ContentResponse::default();
        };
        let response = decision.response;
        self.apply_action(decision);
        response
    }

    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if matches!(event, ContentEvent::PointerDown(_)) {
            let decision = self.decide_action(&action_preparation::OwnedActionRequest::Pointer(event.clone()))
                .expect("original pointer decision");
            let response = decision.response; self.apply_action(decision); return response;
        }
        self.begin_delivery();
        if matches!(event, ContentEvent::Cancel) {
            return ContentResponse {
                changed: self.view.cancel(),
                ..Default::default()
            };
        }
        if !self.available() {
            return ContentResponse::default();
        }
        let mut r = ContentResponse::default();
        match event {
            ContentEvent::PointerMove(p) => {
                self.view.pointer_move(*p);
                r.changed = true;
            }
            ContentEvent::PointerUp(p) => {
                let selected = self.view.pointer_up(*p, &mut self.model);
                r.consumed = selected.is_some();
                r.changed = selected.is_some();
                self.delivered(r.changed);
            }
            ContentEvent::Key { key, down } => {
                let e = self.view.key_event(*key, *down, &mut self.model);
                r.consumed = e.consumed;
                r.changed = e.changed;
                self.delivered(e.changed);
            }
            _ => {}
        }
        r.activated = self.last_intent.is_some();
        r
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        if !self.available() {
            return vec![];
        }
        self.view
            .layout()
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, l)| self.model.is_enabled(i).then_some(l.visible).flatten())
            .collect()
    }
    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.last_intent
            .clone()
            .map(ContentActionPayload::SegmentSelection)
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        match payload {
            ContentActionPayload::SegmentSelection(i) => self.intent_is_current(i),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, s: &str, _: proposal::TextRole, w: f32) -> Size {
            Size::new((s.len() as f32 * 7.).min(w), 18.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    fn ready() -> SegmentedControlContent {
        let mut c = SegmentedControlContent::new("s", vec!["a".into(), "a".into(), "".into()]);
        c.lifecycle(Lifecycle::Mounted);
        c.arrange(
            &mut Text,
            Rect::new(0., 0., 300., 40.),
            Rect::new(0., 0., 300., 40.),
        );
        c.presentation_runnable_changed(true);
        c
    }
    #[test]
    fn exposure_capacity_preflights_all_before_any_publication() {
        let mut c = ready();
        let old = c.semantic_parts();
        c.presentation_runnable_changed(false);
        c.next_token = usize::MAX - 2;
        c.presentation_runnable_changed(true);
        assert_eq!(c.next_token, usize::MAX - 2);
        assert!(c.pending);
        assert!(c.tokens.iter().all(Option::is_none));
        assert!(c.semantic_parts().is_empty());
        for p in old {
            assert!(
                !c.accessibility_action(Some(p.index), &AccessibilityAction::Press)
                    .consumed
            );
        }
        c.next_token = usize::MAX - 3;
        c.presentation_runnable_changed(true);
        assert_eq!(c.next_token, usize::MAX);
        assert!(!c.pending);
        assert_eq!(c.semantic_parts().len(), 3);
        c.presentation_runnable_changed(false);
        c.presentation_runnable_changed(true);
        assert!(c.pending);
        assert!(c.tokens.iter().all(Option::is_none));
        assert_eq!(c.next_token, usize::MAX);
    }
    #[test]
    fn intent_origin_and_reexposure_cannot_be_forged_by_same_labels() {
        let mut c = ready();
        let other = ready();
        let p = c.semantic_parts()[1].index;
        assert!(
            c.accessibility_action(Some(p), &AccessibilityAction::Press)
                .activated
        );
        let i = c.last_intent.clone().unwrap();
        assert!(c.intent_is_current(&i));
        assert!(!other.intent_is_current(&i));
        c.set_selected(0).unwrap();
        assert!(c.intent_is_current(&i));
        c.event(&ContentEvent::Cancel);
        assert!(c.intent_is_current(&i));
        c.lifecycle(Lifecycle::Unmounted);
        c.lifecycle(Lifecycle::Mounted);
        c.arrange(
            &mut Text,
            Rect::new(0., 0., 300., 40.),
            Rect::new(0., 0., 300., 40.),
        );
        c.presentation_runnable_changed(true);
        assert!(!c.intent_is_current(&i));
        assert!(
            !c.accessibility_action(Some(p), &AccessibilityAction::Press)
                .consumed
        );
    }
    #[test]
    fn prepared_segmented_decision_is_pure_and_stale_state_refuses() {
        for change in 0..6 {
            let mut c = ready();
            let token = c.part_token(1).unwrap();
            let before = format!("{c:?}");
            let request = action_preparation::OwnedActionRequest::Accessibility(Some(token), AccessibilityAction::Press);
            let prepared = c.decide_action(&request).expect("original decision");
            assert_eq!(format!("{c:?}"), before, "preparation is read only");
            assert_eq!(c.decide_action(&request).as_ref(), Some(&prepared));
            match change {
                0 => c.set_selected(2).unwrap(),
                1 => { c.view.focus(2); c.view.key_event(ControlKey::Space, true, &mut c.model); }
                2 => c.arrange(&mut Text, Rect::new(0., 0., 250., 40.), Rect::new(0., 0., 250., 40.)),
                3 => c.window_active_changed(false),
                4 => {
                    c.set_item_enabled(1, false).unwrap(); c.set_item_enabled(1, true).unwrap();
                    c.arrange(&mut Text, Rect::new(0., 0., 300., 40.), Rect::new(0., 0., 300., 40.));
                }
                _ => { let p = c.view.layout().items[0].frame.center(); c.event(&ContentEvent::PointerDown(p)); }
            }
            let mutated = format!("{c:?}");
            assert_ne!(c.decide_action(&request).as_ref(), Some(&prepared), "stale change {change}");
            assert_eq!(format!("{c:?}"), mutated, "validation is read only");
        }
    }
    #[test]
    fn prepared_segmented_consuming_apply_survives_original_focus_hooks() {
        for armed in [false, true] {
            for action in 0..3 {
                let mut direct = ready(); let mut prepared_owner = ready();
                for c in [&mut direct, &mut prepared_owner] {
                    c.view.focus(2);
                    if armed { c.view.key_event(ControlKey::Space, true, &mut c.model); }
                }
                let token = prepared_owner.part_token(1).unwrap();
                let point = prepared_owner.view.layout().items[1].frame.center();
                let down = ContentEvent::PointerDown(point);
                let request = match action {
                    0 => action_preparation::OwnedActionRequest::Pointer(down.clone()),
                    1 => action_preparation::OwnedActionRequest::Accessibility(Some(token), AccessibilityAction::Focus),
                    _ => action_preparation::OwnedActionRequest::Accessibility(Some(token), AccessibilityAction::Press),
                };
                let prepared = prepared_owner.decide_action(&request).expect("original decision");
                assert_eq!(prepared_owner.decide_action(&request).as_ref(), Some(&prepared));
                let expected = match action {
                    0 => direct.event(&down),
                    1 => direct.accessibility_action(Some(direct.part_token(1).unwrap()), &AccessibilityAction::Focus),
                    _ => direct.accessibility_action(Some(direct.part_token(1).unwrap()), &AccessibilityAction::Press),
                };
                assert_eq!(prepared.response, expected);
                // Same validated focus hook used by Host before consuming apply.
                prepared_owner.set_focus(true, action != 0);
                prepared_owner.apply_action(prepared);
                assert_eq!(prepared_owner.model(), direct.model());
                assert_eq!(format!("{:?}", prepared_owner.view()), format!("{:?}", direct.view()));
                assert_eq!(prepared_owner.last_intent.as_ref().map(SegmentIntent::index), direct.last_intent.as_ref().map(SegmentIntent::index));
                assert_eq!(prepared_owner.tokens, direct.tokens);
                assert_eq!(prepared_owner.event(&ContentEvent::Key { key: ControlKey::Space, down: false }), direct.event(&ContentEvent::Key { key: ControlKey::Space, down: false }));
                prepared_owner.set_focus(false, false);
                assert_eq!(prepared_owner.view().focused(), None);
                assert!(!prepared_owner.event(&ContentEvent::PointerUp(point)).activated);
            }
        }
    }
    #[test]
    fn prepared_segmented_host_exhaustion_refuses_before_owner_or_queue_effect() {
        const GROUP: ComponentId = ComponentId(110901);
        const OLD: ComponentId = ComponentId(110902);
        for action in 0..3 {
            let mut scroll = ScrollContent::new(crate::ScrollArea::default(), Content::new(GROUP,
                SegmentedControlContent::new("Segments", vec!["A".into(), "B".into(), "C".into()])));
            scroll.viewport_height = Some(80.);
            let mut h = ContentHost::new(Content::new(ComponentId(110900), StackContent {
                model: crate::extended::Stack { axis: crate::extended::Axis::Vertical, gap: 0. },
                flexible_child: None, children: vec![Content::new(OLD, Button::new("Old")),
                    Content::new(ComponentId(110903), scroll)],
            })).unwrap();
            let frame = Rect::new(0., 0., 300., 200.);
            h.render(&mut Scene::new(), &mut Text, frame, frame, tekes_ui_tokens::Theme::LIGHT.into()).unwrap();
            assert!(h.focus(Some(OLD), true)); h.set_action_events(true);
            let c = h.get(GROUP).unwrap().component::<SegmentedControlContent>().unwrap();
            assert_eq!(c.semantic_parts().len(), 3);
            let token = c.part_token(1).unwrap(); let point = c.semantic_parts()[1].frame.center();
            let before = format!("{c:?}");
            h.focus_epoch = u64::MAX;
            let response = match action {
                0 => h.event(&ContentEvent::PointerDown(point)),
                1 => h.accessibility_action(GROUP, Some(token), AccessibilityAction::Focus),
                _ => h.accessibility_action(GROUP, Some(token), AccessibilityAction::Press),
            };
            assert!(!response.consumed && !response.activated && !response.capture_pointer);
            assert_eq!(h.focused(), Some(OLD)); assert_eq!(h.focus_epoch, u64::MAX);
            assert!(h.captured.is_none()); assert!(h.drain_action_records().is_empty());
            assert_eq!(format!("{:?}", h.get(GROUP).unwrap().component::<SegmentedControlContent>().unwrap()), before);
        }
    }

}
