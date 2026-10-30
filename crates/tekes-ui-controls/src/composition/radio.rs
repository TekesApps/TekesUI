//! Fixed-label Radio Content over the original model and roving View.
use super::*;
use crate::{
    RadioGroupView, SelectionEvent,
    extended::{RadioGroup, RadioSelectionError},
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadioContentError {
    LabelCount,
    Selection(RadioSelectionError),
}
impl From<RadioSelectionError> for RadioContentError {
    fn from(error: RadioSelectionError) -> Self {
        Self::Selection(error)
    }
}
/// Immutable delivery value plus private authority for the original queue.
#[derive(Clone, Debug)]
pub struct RadioSelectionIntent {
    origin: Arc<()>,
    index: usize,
    exposure: usize,
}
impl RadioSelectionIntent {
    pub fn index(&self) -> usize {
        self.index
    }
}
impl PartialEq for RadioSelectionIntent {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.origin, &other.origin)
            && self.index == other.index
            && self.exposure == other.exposure
    }
}
impl Eq for RadioSelectionIntent {}

#[derive(Debug)]
pub struct RadioGroupContent {
    name: String,
    labels: Vec<String>,
    model: RadioGroup,
    view: RadioGroupView,
    origin: Arc<()>,
    // Tokens authorize exposure only. Selection/navigation remain in model/View.
    exposures: Vec<Option<usize>>,
    next: usize,
    exhausted: bool,
    mounted: bool,
    runnable: bool,
    window_active: bool,
    last_intent: Option<RadioSelectionIntent>,
    focus_receipt: bool,
}
#[derive(Debug, PartialEq)]
struct PreparedRadioAction {
    identity: action_preparation::OwnerIdentity,
    exposures: Vec<Option<usize>>,
    selected: Option<usize>,
    enabled: Vec<bool>,
    view: Option<crate::selection::RadioViewDecision>,
    selection: Option<crate::extended::RadioSelectDecision>,
    response: ContentResponse,
    focus: PreparedActionFocus,
    rect: Option<Rect>,
    receipt: bool,
}
impl action_preparation::OriginalActionDecision for PreparedRadioAction {
    fn focus_rect(&self) -> Option<Rect> { self.rect }
}
impl RadioGroupContent {
    fn decide_action(&self, request: &action_preparation::OwnedActionRequest) -> Option<PreparedRadioAction> {
        use action_preparation::OwnedActionRequest::*;
        let mut d = PreparedRadioAction {
            identity: action_preparation::OwnerIdentity(self.origin.clone()),
            exposures: self.exposures.clone(), selected: self.model.selected,
            enabled: (0..self.model.len()).map(|i| self.model.is_enabled(i)).collect(),
            view: None, selection: None, response: ContentResponse::default(),
            focus: PreparedActionFocus::None, rect: None, receipt: false,
        };
        if !self.available() { return Some(d); }
        match request {
            Pointer(ContentEvent::PointerDown(p)) => {
                let Some((i, row)) = self.view.layout().rows.iter().enumerate()
                    .find(|(i,r)| self.exposures[*i].is_some() && self.exposable(*i) && r.contains(*p)) else { return Some(d); };
                let v = self.view.prepare_content_down(*p, &self.model);
                // Original pointer-down is handled even for a disabled row.
                d.response = ContentResponse {
                    consumed: v.event != SelectionEvent::Ignored,
                    changed: v.event != SelectionEvent::Ignored,
                    ..Default::default()
                };
                d.response.capture_pointer = d.response.consumed && self.eligible(i);
                d.focus = PreparedActionFocus::Owner;
                d.rect = row.visible_frame();
                d.view = Some(v);
            }
            Accessibility(part, action) => {
                let Some(i) = part.and_then(|t| self.resolve(t)) else { return Some(d); };
                match action {
                    AccessibilityAction::Focus => {
                        d.view = Some(self.view.prepare_content_focus(&self.model, Some(i), true));
                        d.response = ContentResponse::consumed(self.view.focused() != Some(i));
                    }
                    AccessibilityAction::Press => {
                        let selection = self.model.prepare_select(i);
                        d.response = ContentResponse { consumed: true, changed: true,
                            activated: selection.changed(), ..Default::default() };
                        d.selection = Some(selection);
                        d.view = Some(self.view.prepare_content_press(&self.model, i));
                    }
                    _ => return Some(d),
                }
                d.focus = PreparedActionFocus::Owner;
                d.rect = self.view.layout().rows[i].visible_frame();
                d.receipt = true;
            }
            _ => return None,
        }
        Some(d)
    }
    fn apply_action(&mut self, decision: PreparedRadioAction) {
        self.last_intent = None;
        self.focus_receipt = decision.receipt;
        if let Some(selection) = decision.selection {
            let changed = self.model.apply_select(selection);
            self.response(if changed { SelectionEvent::Changed } else { SelectionEvent::Handled });
        }
        if let Some(view) = decision.view { self.view.apply_content_action(view); }
    }
    pub fn new(name: impl Into<String>, labels: Vec<String>) -> Self {
        Self::from_original(
            name,
            RadioGroup::new(labels.len()),
            RadioGroupView::new(),
            labels,
        )
        .expect("new original model matches fixed labels")
    }
    pub fn from_original(
        name: impl Into<String>,
        model: RadioGroup,
        mut view: RadioGroupView,
        labels: Vec<String>,
    ) -> Result<Self, RadioContentError> {
        if model.len() != labels.len() {
            return Err(RadioContentError::LabelCount);
        }
        model.validate_selected(model.selected)?;
        view.clear_content_layout();
        let exposures = vec![None; labels.len()];
        Ok(Self {
            name: name.into(),
            labels,
            model,
            view,
            origin: Arc::new(()),
            exposures,
            next: 0,
            exhausted: false,
            mounted: false,
            runnable: false,
            window_active: true,
            last_intent: None,
            focus_receipt: false,
        })
    }
    pub fn model(&self) -> &RadioGroup {
        &self.model
    }
    pub fn view(&self) -> &RadioGroupView {
        &self.view
    }
    /// Current published semantic part token for an original fixed logical slot.
    /// Read-only: no reservation, recovery or input is performed. Hidden/pending
    /// slots have no token; a visible disabled token still cannot authorize AX.
    pub fn part_token(&self, index: usize) -> Option<usize> {
        self.exposable(index).then(|| self.exposures.get(index).copied().flatten()).flatten()
    }
    pub fn set_selected(&mut self, selected: Option<usize>) -> Result<(), RadioContentError> {
        self.model.validate_selected(selected)?;
        if self.model.selected == selected {
            return Ok(());
        }
        self.view.cancel();
        self.model.set_selected(selected)?;
        Ok(())
    }
    pub fn set_item_enabled(
        &mut self,
        index: usize,
        enabled: bool,
    ) -> Result<(), RadioContentError> {
        if index >= self.model.len() {
            return Err(RadioSelectionError::InvalidIndex.into());
        }
        if self.model.is_enabled(index) == enabled {
            return Ok(());
        }
        self.view.cancel();
        self.exposures[index] = None;
        self.model.set_enabled(index, enabled);
        if self
            .view
            .focused()
            .is_some_and(|i| !self.model.is_enabled(i))
        {
            self.view.blur();
        }
        // Recovery is pending until the next actual layout publication.
        Ok(())
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        if self.view.is_disabled() == disabled {
            return;
        }
        self.view.cancel();
        self.revoke();
        self.view.set_disabled(disabled);
        if disabled {
            self.runnable = false;
        }
    }
    pub fn set_error(&mut self, error: bool) {
        self.view.set_error(error);
    }
    fn available(&self) -> bool {
        self.mounted
            && self.runnable
            && self.window_active
            && !self.view.is_disabled()
            && !self.exhausted
    }
    fn revoke(&mut self) {
        self.exposures.fill(None);
        self.focus_receipt = false;
        self.view.cancel();
    }
    fn exposable(&self, i: usize) -> bool {
        self.available()
            && self
                .view
                .layout()
                .rows
                .get(i)
                .is_some_and(|row| row.visible_frame().is_some())
    }
    fn eligible(&self, i: usize) -> bool {
        self.exposable(i) && self.model.is_enabled(i)
    }
    fn publish(&mut self) {
        // Withdraw first. Prepare the complete checked plan before assigning any
        // new token. Failure is sticky, leaves no old/partial authority or wrap.
        for i in 0..self.exposures.len() {
            if !self.exposable(i) {
                self.exposures[i] = None;
            }
        }
        if !self.available() {
            return;
        }
        let needed = (0..self.exposures.len())
            .filter(|&i| self.exposable(i) && self.exposures[i].is_none())
            .count();
        let Some(end) = self.next.checked_add(needed) else {
            self.exhausted = true;
            self.revoke();
            return;
        };
        let mut next = self.next;
        let mut plan = self.exposures.clone();
        for (i, slot) in plan.iter_mut().enumerate() {
            if self.exposable(i) && slot.is_none() {
                *slot = Some(next);
                next += 1;
            }
        }
        self.exposures = plan;
        self.next = end;
    }
    fn resolve(&self, token: usize) -> Option<usize> {
        self.exposures
            .iter()
            .position(|&e| e == Some(token))
            .filter(|&i| self.eligible(i))
    }
    pub fn intent_is_current(&self, intent: &RadioSelectionIntent) -> bool {
        Arc::ptr_eq(&self.origin, &intent.origin)
            && self.eligible(intent.index)
            && self.exposures.get(intent.index) == Some(&Some(intent.exposure))
    }
    fn response(&mut self, event: SelectionEvent) -> ContentResponse {
        if event == SelectionEvent::Changed {
            self.last_intent = self.model.selected.and_then(|index| {
                self.exposures[index].map(|exposure| RadioSelectionIntent {
                    origin: self.origin.clone(),
                    index,
                    exposure,
                })
            });
        }
        ContentResponse {
            consumed: event != SelectionEvent::Ignored,
            changed: event != SelectionEvent::Ignored,
            activated: self.last_intent.is_some(),
            ..Default::default()
        }
    }
}
impl Component for RadioGroupContent {
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(self, request, Self::decide_action, Self::apply_action,
            |d| (d.response, d.focus))
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let labels: Vec<_> = self.labels.iter().map(String::as_str).collect();
        MeasuredChild::new(RadioGroupView::measure_content(
            &mut TextAdapter(text),
            &self.model,
            available.width,
            &labels,
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let labels: Vec<_> = self.labels.iter().map(String::as_str).collect();
        self.view.arrange_content(
            &mut TextAdapter(text),
            &self.model,
            frame,
            Some(clip),
            &labels,
        );
        // Part disappearance must be recorded even when the root remains runnable.
        for i in 0..self.exposures.len() {
            if self
                .view
                .layout()
                .rows
                .get(i)
                .is_none_or(|row| row.visible_frame().is_none())
            {
                self.exposures[i] = None;
            }
        }
        // The original Host's successful layout notification publishes exposure.
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.view
            .set_surface(theme.surface())
            .expect("validated inherited Surface");
        let labels: Vec<_> = self.labels.iter().map(String::as_str).collect();
        self.view.paint_content(
            scene,
            &mut TextAdapter(text),
            &self.model,
            Some(clip),
            theme.palette,
            &labels,
        );
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        self.last_intent = None;
        self.revoke();
        self.mounted = event == Lifecycle::Mounted;
        self.view
            .set_window_active(self.mounted && self.window_active);
        self.runnable = false;
        self.view.clear_content_layout();
    }
    fn window_active_changed(&mut self, active: bool) {
        if self.window_active == active {
            return;
        }
        self.window_active = active;
        if !active {
            self.runnable = false;
            self.revoke();
        }
        self.view.set_window_active(active);
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.runnable = runnable;
        if !runnable {
            self.revoke();
            self.view.blur();
        } else {
            self.publish();
        }
    }
    fn interactive(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        self.available()
            && self
                .exposures
                .iter()
                .enumerate()
                .any(|(i, token)| token.is_some() && self.model.is_enabled(i))
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if !focused {
            self.view.blur();
        } else if self.focusable() {
            self.view.focus(&self.model, self.view.focused(), visible);
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let mut s = ContentSemantics::new(ContentRole::RadioGroup, &self.name);
        s.disabled = !self.available();
        s
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        self.exposures
            .iter()
            .enumerate()
            .filter_map(|(i, token)| {
                let token = (*token)?;
                if !self.exposable(i) {
                    return None;
                }
                let frame = self.view.layout().rows[i].visible_frame()?;
                let mut semantics =
                    ContentSemantics::new(ContentRole::RadioButton, &self.labels[i]);
                semantics.selected = self.model.selected == Some(i);
                semantics.disabled = !self.model.is_enabled(i);
                Some(SemanticPart {
                    index: token,
                    frame,
                    focused: self.view.focused() == Some(i),
                    semantics,
                })
            })
            .collect()
    }
    fn pointer_regions(&self, _: Rect, _: Rect) -> Vec<Rect> {
        if !self.available() {
            return vec![];
        }
        self.view
            .layout()
            .rows
            .iter()
            .filter_map(|row| row.visible_frame())
            .collect()
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
        let Some(decision) = self.decide_action(&request) else { return ContentResponse::default(); };
        let response = decision.response;
        self.apply_action(decision);
        response
    }
    fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
        self.last_intent
            .clone()
            .map(ContentActionPayload::RadioSelection)
    }
    fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
        match payload {
            ContentActionPayload::RadioSelection(i) => self.intent_is_current(i),
            _ => false,
        }
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        self.last_intent = None;
        self.focus_receipt = false;
        if matches!(event, ContentEvent::Cancel) {
            return ContentResponse {
                changed: self.view.cancel(),
                ..Default::default()
            };
        }
        if !self.available() {
            return ContentResponse::default();
        }
        let event_result = match event {
            ContentEvent::PointerMove(p) => {
                return ContentResponse {
                    changed: self.view.pointer_move(*p),
                    ..Default::default()
                };
            }
            ContentEvent::PointerDown(_) => {
                let request = action_preparation::OwnedActionRequest::Pointer(event.clone());
                let decision = self.decide_action(&request).expect("pointer decision");
                let response = decision.response;
                self.apply_action(decision);
                return response;
            }
            ContentEvent::PointerUp(p) => self.view.pointer_up(*p, &mut self.model),
            ContentEvent::Key { key, down } => {
                if !self
                    .view
                    .focused()
                    .is_some_and(|i| self.exposures[i].is_some() && self.eligible(i))
                {
                    return ContentResponse::default();
                }
                self.view.key(*key, *down, &mut self.model)
            }
            _ => SelectionEvent::Ignored,
        };
        let mut result = self.response(event_result);
        result.capture_pointer = matches!(event, ContentEvent::PointerDown(p) if self.view.layout().rows.iter().enumerate().any(|(i,r)| self.eligible(i) && r.contains(*p)))
            && result.consumed;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(30., 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: crate::ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    #[test]
    fn checked_part_capacity_rejects_complete_plan_without_partial_tokens_or_wrap() {
        let mut c = RadioGroupContent::new("Choices", vec!["A".into(), "B".into(), "C".into()]);
        c.lifecycle(Lifecycle::Mounted);
        let f = Rect::new(0., 0., 300., 200.);
        c.arrange(&mut Text, f, f);
        c.next = usize::MAX - 1;
        c.presentation_runnable_changed(true);
        assert!(c.exhausted);
        assert_eq!(c.next, usize::MAX - 1);
        assert_eq!(c.exposures, vec![None, None, None]);
        assert!(c.semantic_parts().is_empty());
        c.presentation_runnable_changed(false);
        c.presentation_runnable_changed(true);
        assert!(c.semantic_parts().is_empty());
    }
    #[test]
    fn capacity_failure_also_revokes_previously_retained_authority() {
        let mut c = RadioGroupContent::new("Choices", vec!["A".into(), "B".into(), "C".into()]);
        c.lifecycle(Lifecycle::Mounted);
        let f = Rect::new(0., 0., 300., 200.);
        c.arrange(&mut Text, f, f);
        c.presentation_runnable_changed(true);
        let old = c.semantic_parts()[0].index;
        assert!(
            c.accessibility_action(
                Some(c.semantic_parts()[1].index),
                &AccessibilityAction::Press
            )
            .activated
        );
        let intent = c.last_intent.clone().unwrap();
        c.exposures[2] = None;
        c.next = usize::MAX;
        c.publish();
        assert!(c.exposures.iter().all(Option::is_none));
        assert!(!c.intent_is_current(&intent));
        assert!(
            !c.accessibility_action(Some(old), &AccessibilityAction::Focus)
                .consumed
        );
    }
    fn ready() -> RadioGroupContent {
        let mut c = RadioGroupContent::new("Choices", vec!["A".into(), "B".into()]);
        c.lifecycle(Lifecycle::Mounted);
        let f = Rect::new(0.,0.,300.,200.);
        c.arrange(&mut Text, f, f);
        c.presentation_runnable_changed(true);
        c
    }
    fn prepared_press(c: &RadioGroupContent) -> (action_preparation::OwnedActionRequest, PreparedRadioAction) {
        let request = action_preparation::OwnedActionRequest::Accessibility(
            c.part_token(1), AccessibilityAction::Press,
        );
        let decision = c.decide_action(&request).expect("original press decision");
        (request, decision)
    }
    #[test]
    fn prepared_final_validation_rejects_controlled_scope_geometry_and_foreign_changes() {
        for change in 0..6 {
            let mut c = ready(); let (request, p) = prepared_press(&c);
            assert_eq!(c.decide_action(&request).as_ref(), Some(&p));
            match change {
                0 => c.set_selected(None).unwrap(),
                1 => c.set_item_enabled(1,false).unwrap(),
                2 => c.window_active_changed(false),
                3 => c.presentation_runnable_changed(false),
                4 => c.arrange(&mut Text,Rect::new(0.,0.,300.,200.),Rect::new(600.,600.,1.,1.)),
                _ => { c.lifecycle(Lifecycle::Unmounted); c.lifecycle(Lifecycle::Mounted);
                    c.arrange(&mut Text,Rect::new(0.,0.,300.,200.),Rect::new(0.,0.,300.,200.));
                    c.presentation_runnable_changed(true); }
            }
            assert_ne!(c.decide_action(&request).as_ref(), Some(&p), "change {change}");
            assert!(c.last_intent.is_none());
        }
        let c = ready(); let (request, p) = prepared_press(&c); let foreign = ready();
        assert_ne!(foreign.decide_action(&request).as_ref(), Some(&p));
    }
    #[test]
    fn consumed_original_pointer_decision_survives_host_focus_hook_without_selecting() {
        let mut c = ready(); let point = c.semantic_parts()[1].frame.center();
        let event=ContentEvent::PointerDown(point);
        let request = action_preparation::OwnedActionRequest::Pointer(event);
        let p = c.decide_action(&request).expect("original pointer decision");
        assert_eq!(c.decide_action(&request).as_ref(), Some(&p));
        c.set_focus(true,false); // actual Host ordering: validate, focus, consume
        c.apply_action(p);
        assert_eq!(c.view.focused(),Some(1)); assert_eq!(c.model.selected,Some(0));
        assert!(c.last_intent.is_none());
        assert!(c.event(&ContentEvent::PointerUp(point)).activated);
        assert_eq!(c.last_intent.as_ref().unwrap().index(),1);
        assert!(!c.event(&ContentEvent::PointerUp(point)).activated);
    }
    #[test]
    fn consumed_original_ax_decisions_survive_host_focus_hook() {
        for press in [false,true] {
            let mut c=ready(); let action=if press {AccessibilityAction::Press}else{AccessibilityAction::Focus};
            let request = action_preparation::OwnedActionRequest::Accessibility(c.part_token(1), action);
            let p = c.decide_action(&request).expect("original AX decision");
            assert_eq!(c.decide_action(&request).as_ref(), Some(&p)); c.set_focus(true,true); c.apply_action(p);
            assert_eq!(c.view.focused(),Some(1));
            assert_eq!(c.model.selected,Some(if press {1}else{0}));
            assert_eq!(c.last_intent.is_some(),press); assert!(c.focus_receipt);
        }
    }

    #[test]
    fn disabled_pointer_decision_preserves_original_host_focus_hook_result() {
        fn disabled_ready() -> RadioGroupContent {
            let mut c = ready(); c.set_item_enabled(1, false).unwrap();
            let f = Rect::new(0.,0.,300.,200.); c.arrange(&mut Text,f,f);
            c.presentation_runnable_changed(true); c
        }
        let mut ordinary = disabled_ready();
        let point = ordinary.semantic_parts()[1].frame.center();
        let result = ordinary.event(&ContentEvent::PointerDown(point));
        assert!(result.consumed && !result.capture_pointer);
        ordinary.set_focus(true,false); // original ordinary Host order
        assert_eq!(ordinary.view.focused(),Some(0));
        let mut prepared = disabled_ready();
        let request=action_preparation::OwnedActionRequest::Pointer(ContentEvent::PointerDown(point));
        let decision=prepared.decide_action(&request).unwrap();
        assert_eq!(decision.focus,PreparedActionFocus::Owner);
        prepared.set_focus(true,false); // actual strict Host order
        prepared.apply_action(decision);
        assert_eq!(prepared.view.focused(),ordinary.view.focused());
        assert_eq!(prepared.model.selected,ordinary.model.selected);
        assert!(prepared.last_intent.is_none());
    }

}
