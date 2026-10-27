//! Finite Content adapter for the original single-series `BarChart` owner.
use super::*;
use crate::chart::{BarChart, ChartDataUpdateError, ChartDatum};
use crate::collections_view::{ChartOwnerFocusSpace, ChartView, CollectionEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartContentError {
    EmptyName,
    NonFiniteValue { index: usize },
}
impl From<ChartDataUpdateError> for ChartContentError {
    fn from(value: ChartDataUpdateError) -> Self {
        Self::NonFiniteValue { index: value.index }
    }
}

#[derive(Debug)]
pub struct ChartContent {
    name: String,
    model: BarChart,
    view: ChartView,
    preferred_height: f32,
}
impl ChartContent {
    pub fn try_new(
        name: impl Into<String>,
        data: Vec<ChartDatum>,
    ) -> Result<Self, ChartContentError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ChartContentError::EmptyName);
        }
        if let Some((index, _)) = data.iter().enumerate().find(|(_, d)| !d.value.is_finite()) {
            return Err(ChartContentError::NonFiniteValue { index });
        }
        Ok(Self {
            name,
            model: BarChart::new(data),
            view: ChartView::new(),
            preferred_height: 240.0,
        })
    }
    pub fn preferred_height(mut self, height: f32) -> Self {
        if height.is_finite() && height >= 0.0 {
            self.preferred_height = height;
        }
        self
    }
    pub fn update_data(&mut self, data: Vec<ChartDatum>) -> Result<(), ChartContentError> {
        self.model.try_update_data(data)?;
        self.view.invalidate_owner_layout();
        Ok(())
    }
    pub fn data(&self) -> &[ChartDatum] {
        self.model.data()
    }
    pub fn selected_datum(&self) -> Option<&ChartDatum> {
        self.model
            .selected()
            .and_then(|index| self.model.data().get(index))
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.model.selected()
    }
    fn hit_test(&self, point: Point) -> Option<usize> {
        let visible = self.view.layout().visible?;
        if !visible.contains(point) {
            return None;
        }
        self.view
            .layout()
            .geometry
            .bars
            .iter()
            .find(|bar| {
                let hit = bar.rect.inset(-6.0);
                hit.intersection(&visible)
                    .is_some_and(|r| r.contains(point))
            })
            .map(|bar| bar.index)
    }
    fn event_response(event: CollectionEvent, capture: bool) -> ContentResponse {
        ContentResponse {
            consumed: event != CollectionEvent::Ignored,
            changed: matches!(
                event,
                CollectionEvent::Changed | CollectionEvent::Activated(_)
            ),
            capture_pointer: capture && event != CollectionEvent::Ignored,
            ..Default::default()
        }
    }
    fn decide_action(
        &self,
        request: &action_preparation::OwnedActionRequest,
    ) -> Option<PreparedChartAction> {
        if !self.view.window_active() {
            return None;
        }
        let action = match request {
            action_preparation::OwnedActionRequest::Pointer(event) => {
                let routed = event.routing_event();
                let ContentEvent::PointerDown(point) = routed.as_ref() else {
                    return None;
                };
                let index = self.hit_test(*point)?;
                Some(index)
            }
            _ => None,
        }?;
        let event = if self.model.selectable(action) {
            CollectionEvent::Changed
        } else {
            CollectionEvent::Handled
        };
        Some(PreparedChartAction {
            index: action,
            selected: self.model.selected(),
            data: self
                .model
                .data()
                .iter()
                .map(|d| (d.label.clone(), d.value.to_bits()))
                .collect(),
            response: Self::event_response(event, true),
            witness: match self.view.owner_required_focus_space(&self.model) {
                ChartOwnerFocusSpace::Visible { witness } => Some(witness),
                _ => None,
            },
        })
    }
    fn apply_action(&mut self, action: PreparedChartAction) {
        let _ = self.model.select(action.index);
    }
}

#[derive(Debug, PartialEq)]
struct PreparedChartAction {
    index: usize,
    selected: Option<usize>,
    data: Vec<(String, u64)>,
    response: ContentResponse,
    witness: Option<Rect>,
}
impl action_preparation::OriginalActionDecision for PreparedChartAction {
    fn focus_rect(&self) -> Option<Rect> {
        self.witness
    }
}

impl Component for ChartContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        // ChartContent is a local leaf owner: paint is called inside the
        // original Scene shape/rect clip scope, pointer targeting requires both
        // its visible chart bounds and the Host's exact curve geometry, and AX
        // visibility is projected through the same frozen shape chain. Its
        // target RequiredFocus witness remains a separate contract.
        CurveSubtreeCapability::Local
    }
    fn action_transaction_compatibility(&self) -> ActionTransactionCompatibility {
        ActionTransactionCompatibility::StableHooks
    }
    fn prepare_action(&self, request: ComponentActionRequest<'_>) -> ActionPreparation {
        action_preparation::prepare_original(
            self,
            request,
            Self::decide_action,
            Self::apply_action,
            |action| (action.response, PreparedActionFocus::Owner),
        )
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let width = if available.width.is_finite() {
            available.width.max(0.0)
        } else {
            0.0
        };
        let cap = if available.height.is_finite() {
            available.height.max(0.0)
        } else {
            self.preferred_height
        };
        let height = self.preferred_height.min(cap);
        let _ = self.view.measure_owner(text, &self.model, width, height);
        MeasuredChild::new(Size::new(width, height))
    }
    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.measure(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let _ = self
            .view
            .arrange_owner(text, &mut self.model, frame, Some(clip));
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        theme: ContentTheme,
    ) {
        let _ = self.view.paint_owner_on_surface(
            scene,
            text,
            &mut self.model,
            theme.palette,
            theme.surface(),
        );
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::PointerMove(point) => ContentResponse {
                changed: self.view.pointer_move(*point, &mut self.model),
                ..Default::default()
            },
            ContentEvent::Key { key, down } => {
                Self::event_response(self.view.key(*key, *down, &mut self.model), false)
            }
            ContentEvent::Cancel => {
                self.view.clear_hover(&mut self.model);
                ContentResponse::default()
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_event_in_region(&mut self, event: &ContentEvent, inside: bool) -> ContentResponse {
        if let ContentEvent::Pointer(pointer) = event {
            if !pointer.is_primary() {
                return self.pointer_event(pointer);
            }
            if let ContentEvent::PointerMove(point) = pointer.routing_event() {
                return ContentResponse {
                    changed: self
                        .view
                        .pointer_move_in_region(point, &mut self.model, inside),
                    ..Default::default()
                };
            }
            return self.pointer_event(pointer);
        }
        if let ContentEvent::PointerMove(point) = event {
            return ContentResponse {
                changed: self
                    .view
                    .pointer_move_in_region(*point, &mut self.model, inside),
                ..Default::default()
            };
        }
        self.event(event)
    }
    fn interactive(&self) -> bool {
        true
    }
    fn pointer_regions(&self, _frame: Rect, _clip: Rect) -> Vec<Rect> {
        self.view.layout().visible.into_iter().collect()
    }
    fn focusable(&self) -> bool {
        self.model.selected().is_some()
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        if focused {
            // ContentHost has already prepared, revalidated and consumed the
            // required witness/reveal plan before applying this owner effect.
            self.view.focus(visible, &mut self.model);
        } else {
            self.view.blur(&mut self.model);
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.view.set_window_active(active, &mut self.model);
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.view.blur(&mut self.model);
            self.model.set_hovered(None);
            self.view.invalidate_owner_layout();
        }
    }
    fn requires_focus_reveal(&self) -> bool {
        self.focusable()
    }
    fn required_focus_space(&self) -> RequiredFocusSpace {
        match self.view.owner_required_focus_space(&self.model) {
            ChartOwnerFocusSpace::Visible { witness } => {
                self.view.owner_focus_evidence(&self.model).map_or(
                    RequiredFocusSpace::Unmeasured,
                    |token| RequiredFocusSpace::Sufficient {
                        witness,
                        evidence: RequiredFocusEvidence::from_owner(token),
                    },
                )
            }
            ChartOwnerFocusSpace::NotRequired => RequiredFocusSpace::NotRequired,
            ChartOwnerFocusSpace::Unmeasured => RequiredFocusSpace::Unmeasured,
            ChartOwnerFocusSpace::Insufficient(_)
            | ChartOwnerFocusSpace::NoVisibleWitness
            | ChartOwnerFocusSpace::Excluded
            | ChartOwnerFocusSpace::Inactive => RequiredFocusSpace::Insufficient,
        }
    }
    fn required_focus_result(&self) -> Option<RequiredFocusResult> {
        Some(match self.view.owner_focus_space(&self.model) {
            ChartOwnerFocusSpace::NotRequired => RequiredFocusResult::NotRequired,
            ChartOwnerFocusSpace::Visible { witness } => RequiredFocusResult::Visible { witness },
            ChartOwnerFocusSpace::NoVisibleWitness | ChartOwnerFocusSpace::Excluded => {
                match self.view.owner_required_focus_space(&self.model) {
                    ChartOwnerFocusSpace::Visible { witness } => {
                        RequiredFocusResult::Clipped { witness }
                    }
                    _ => RequiredFocusResult::Insufficient,
                }
            }
            ChartOwnerFocusSpace::Unmeasured => RequiredFocusResult::Unmeasured,
            ChartOwnerFocusSpace::Inactive | ChartOwnerFocusSpace::Insufficient(_) => {
                RequiredFocusResult::Insufficient
            }
        })
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics = ContentSemantics::new(ContentRole::Group, &self.name);
        semantics.value = self
            .model
            .selected()
            .and_then(|index| self.model.data().get(index))
            .map(|datum| {
                if datum.label.trim().is_empty() {
                    datum.value.to_string()
                } else {
                    format!("{}: {}", datum.label, datum.value)
                }
            });
        semantics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrollArea;
    use crate::composition::{ComponentId, Content, ContentHost, ScrollContent};
    use tekes_ui_scene::Scene;
    use tekes_ui_tokens::{Rgba, Theme, proposal};

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, value: &str, _: proposal::TextRole, width: f32) -> Size {
            Size::new((value.chars().count() as f32 * 7.0).min(width), 18.0)
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
    fn chart() -> ChartContent {
        ChartContent::try_new(
            "Quarterly",
            vec![
                ChartDatum::new("", 4.0),
                ChartDatum::new("Q1", -2.0),
                ChartDatum::new("Q1", 3.0),
            ],
        )
        .unwrap()
    }
    fn render(host: &mut ContentHost, frame: Rect) {
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }

    #[test]
    fn static_parent_allocates_finite_chart_and_focuses_visible_original_owner() {
        let id = ComponentId(9201);
        let mut host = ContentHost::new(Content::new(id, chart())).unwrap();
        let frame = Rect::new(0.0, 0.0, 320.0, 280.0);
        render(&mut host, frame);
        assert!(matches!(
            host.required_focus_result(id),
            Some(RequiredFocusResult::Visible { .. })
        ));
        assert!(host.focus(Some(id), true));
        assert_eq!(host.focused(), Some(id));
    }

    #[test]
    fn curved_region_rejection_clears_chart_hover_through_original_view() {
        let mut chart = chart();
        let frame = Rect::new(0.0, 0.0, 320.0, 280.0);
        chart
            .view
            .arrange_owner(&mut Text, &mut chart.model, frame, Some(frame));
        let bar = chart.view.layout().geometry.bars[1].rect;
        let point = tekes_ui_scene::Point::new(
            bar.min_x() + bar.size.width * 0.5,
            bar.min_y() + bar.size.height * 0.5,
        );
        assert!(
            chart
                .view
                .pointer_move_in_region(point, &mut chart.model, true)
        );
        assert_eq!(chart.model.hovered(), Some(1));
        let secondary_move = ContentEvent::Pointer(PointerEvent {
            position: point,
            phase: PointerPhase::Move,
            button: Some(PointerButton::Secondary),
            modifiers: PointerModifiers::default(),
            click_count: 0,
        });
        assert_eq!(
            chart.pointer_event_in_region(&secondary_move, false),
            ContentResponse::default()
        );
        assert_eq!(
            chart.model.hovered(),
            Some(1),
            "non-primary metadata never reaches the region reducer"
        );
        let primary_move = ContentEvent::Pointer(PointerEvent {
            position: point,
            phase: PointerPhase::Move,
            button: None,
            modifiers: PointerModifiers::default(),
            click_count: 0,
        });
        assert!(chart.pointer_event_in_region(&primary_move, false).changed);
        assert_eq!(chart.model.hovered(), None);
    }

    #[test]
    fn panel_curve_does_not_replace_chart_target_focus_evidence() {
        let id = ComponentId(9209);
        let panel_id = ComponentId(9208);
        let mut host = ContentHost::new(Content::new(
            panel_id,
            PanelContent::new(Content::new(id, chart())).unwrap(),
        ))
        .unwrap();
        let frame = Rect::new(0.0, 0.0, 320.0, 280.0);
        assert!(
            !host.focus(Some(id), true),
            "unmeasured Chart target remains rejected"
        );
        render(&mut host, frame);
        assert!(matches!(
            host.required_focus_result(id),
            Some(RequiredFocusResult::Visible { .. })
        ));
        assert!(host.focus(Some(id), true));

        let mut tiny = ContentHost::new(Content::new(
            panel_id,
            PanelContent::new(Content::new(id, chart())).unwrap(),
        ))
        .unwrap();
        render(&mut tiny, Rect::new(0.0, 0.0, 4.0, 4.0));
        assert_eq!(
            tiny.required_focus_result(id),
            Some(RequiredFocusResult::Insufficient)
        );
        assert!(
            !tiny.focus(Some(id), true),
            "Panel cannot downgrade Chart Insufficient to Legacy"
        );
    }

    #[test]
    fn scroll_parent_uses_finite_measurement_and_reveals_original_owner_witness() {
        let id = ComponentId(9202);
        let scroll = ScrollContent::new(ScrollArea::default(), Content::new(id, chart()));
        let mut host = ContentHost::new(Content::new(ComponentId(9200), scroll)).unwrap();
        let frame = Rect::new(0.0, 0.0, 320.0, 96.0);
        render(&mut host, frame);
        host.get_mut(ComponentId(9200))
            .unwrap()
            .component_mut::<ScrollContent>()
            .unwrap()
            .scroll
            .offset = tekes_ui_scene::Point::new(0.0, 100.0);
        // Keep the chart body in view while clipping both vertical owner edges;
        // the top and bottom bands are outside the viewport until original scroll reveal.
        let interior_clip = Rect::new(40.0, 0.0, 240.0, 96.0);
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            interior_clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(matches!(
            host.required_focus_result(id),
            Some(RequiredFocusResult::Clipped { .. })
        ));
        let offset_before_focus = host
            .get(ComponentId(9200))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y;
        assert!(host.focus(Some(id), true));
        host.render(
            &mut Scene::new(),
            &mut Text,
            frame,
            interior_clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let offset_after_focus = host
            .get(ComponentId(9200))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y;
        assert!(offset_after_focus > offset_before_focus);
        assert!(matches!(
            host.required_focus_result(id),
            Some(RequiredFocusResult::Visible { .. })
        ));
        assert_eq!(host.focused(), Some(id));
    }

    #[test]
    fn pointer_down_uses_original_prepared_focus_then_selects_once() {
        let id = ComponentId(9208);
        let mut host = ContentHost::new(Content::new(id, chart())).unwrap();
        let frame = Rect::new(0.0, 0.0, 320.0, 280.0);
        render(&mut host, frame);
        let bar = host
            .get(id)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .view
            .layout()
            .geometry
            .bars[1]
            .rect;
        let point = tekes_ui_scene::Point::new(
            bar.min_x() + bar.size.width * 0.5,
            bar.min_y() + bar.size.height * 0.5,
        );
        let response = host.event(&ContentEvent::PointerDown(point));
        assert!(response.consumed);
        assert!(response.changed);
        assert_eq!(host.focused(), Some(id));
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ChartContent>()
                .unwrap()
                .selected_index(),
            Some(1)
        );
        assert!(host.pointer_capture().is_some());
    }

    #[test]
    fn tiny_static_parent_refuses_focus_without_fabricating_space() {
        let id = ComponentId(9203);
        let mut host = ContentHost::new(Content::new(id, chart())).unwrap();
        let frame = Rect::new(0.0, 0.0, 4.0, 4.0);
        render(&mut host, frame);
        assert_eq!(
            host.required_focus_result(id),
            Some(RequiredFocusResult::Insufficient)
        );
        assert!(!host.focus(Some(id), true));
        assert_eq!(host.focused(), None);
    }

    #[test]
    fn paint_owner_without_arrange_refuses_safely_before_mutating_or_painting() {
        let mut chart = chart();
        chart.view.focus(true, &mut chart.model);
        let was_focused = chart.model.is_focused();
        let selected = chart.model.selected();
        let mut scene = Scene::new();
        let theme: ContentTheme = tekes_ui_tokens::Palette::from(Theme::LIGHT).into();
        assert_eq!(
            chart
                .view
                .paint_owner(&mut scene, &mut Text, &mut chart.model, theme.palette),
            ChartOwnerFocusSpace::Unmeasured
        );
        assert_eq!(scene.len(), 0);
        assert_eq!(chart.model.is_focused(), was_focused);
        assert_eq!(chart.model.selected(), selected);
    }

    #[test]
    fn paint_owner_rejects_data_mismatch_before_using_old_geometry() {
        let mut chart = chart();
        let frame = Rect::new(0.0, 0.0, 320.0, 280.0);
        chart
            .view
            .arrange_owner(&mut Text, &mut chart.model, frame, Some(frame));
        chart.view.focus(true, &mut chart.model);
        let was_focused = chart.model.is_focused();
        let selected = chart.model.selected();
        chart
            .model
            .try_update_data(vec![ChartDatum::new("replacement", 7.0)])
            .unwrap();
        let mut scene = Scene::new();
        let theme: ContentTheme = tekes_ui_tokens::Palette::from(Theme::LIGHT).into();
        assert_eq!(
            chart
                .view
                .paint_owner(&mut scene, &mut Text, &mut chart.model, theme.palette),
            ChartOwnerFocusSpace::Unmeasured
        );
        assert_eq!(scene.len(), 0);
        assert_eq!(chart.model.is_focused(), was_focused);
        assert_eq!(chart.model.selected(), selected);
        assert_eq!(chart.model.data()[0].label, "replacement");
    }

    #[derive(Debug)]
    struct WitnessProbe {
        evidence: RequiredFocusEvidence,
        frame: Rect,
        focus_calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        invalid: bool,
        legacy_reveal: bool,
    }
    impl Component for WitnessProbe {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(Size::new(
                available.width.min(120.0),
                available.height.min(80.0),
            ))
        }
        fn arrange(&mut self, _: &mut dyn ButtonTextRenderer, frame: Rect, _: Rect) {
            self.frame = frame;
        }
        fn focusable(&self) -> bool {
            true
        }
        fn set_focus(&mut self, focused: bool, _: bool) {
            if focused {
                self.focus_calls
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        fn requires_focus_reveal(&self) -> bool {
            self.legacy_reveal
        }
        fn required_focus_space(&self) -> RequiredFocusSpace {
            let witness = if self.invalid {
                Rect::new(self.frame.max_x() + 1.0, self.frame.min_y(), 10.0, 10.0)
            } else {
                Rect::new(
                    self.frame.min_x() + 8.0,
                    self.frame.min_y() + 8.0,
                    12.0,
                    12.0,
                )
            };
            RequiredFocusSpace::Sufficient {
                witness,
                evidence: self.evidence.clone(),
            }
        }
    }
    fn probe(
        invalid: bool,
        legacy_reveal: bool,
    ) -> (WitnessProbe, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        (
            WitnessProbe {
                evidence: RequiredFocusEvidence::new(),
                frame: Rect::default(),
                focus_calls: calls.clone(),
                invalid,
                legacy_reveal,
            },
            calls,
        )
    }

    #[test]
    fn invalid_required_witness_rejects_before_focus_effect() {
        let id = ComponentId(9204);
        let (probe, calls) = probe(true, false);
        let mut host = ContentHost::new(Content::new(id, probe)).unwrap();
        let frame = Rect::new(0.0, 0.0, 180.0, 100.0);
        render(&mut host, frame);
        assert!(!host.focus(Some(id), true));
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(host.focused(), None);
    }

    #[test]
    fn required_witness_plan_is_bound_to_original_owner_evidence() {
        let id = ComponentId(9205);
        let (probe, _) = probe(false, false);
        let mut host = ContentHost::new(Content::new(id, probe)).unwrap();
        let frame = Rect::new(0.0, 0.0, 180.0, 100.0);
        render(&mut host, frame);
        let plan = host.plan_focus_reveal(id).unwrap();
        host.get_mut(id)
            .unwrap()
            .component_mut::<WitnessProbe>()
            .unwrap()
            .evidence = RequiredFocusEvidence::new();
        assert_eq!(
            host.validate_host_reveal(&plan),
            Err(HostRevealError::StalePlan)
        );
        assert_eq!(host.focused(), None);
    }

    #[test]
    fn legacy_reveal_boolean_without_query_stays_unsupported_and_plain_button_stays_logical() {
        let id = ComponentId(9206);
        let (probe, calls) = probe(false, true);
        // Restore the default Legacy query so this exercises the compatibility guard.
        let legacy = LegacyRevealProbe { probe };
        let mut host = ContentHost::new(Content::new(id, legacy)).unwrap();
        let frame = Rect::new(0.0, 0.0, 180.0, 100.0);
        render(&mut host, frame);
        assert!(!host.focus(Some(id), true));
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        let button_id = ComponentId(9207);
        let mut ordinary = ContentHost::new(Content::new(button_id, Button::new("OK"))).unwrap();
        let button_frame = Rect::new(0.0, 0.0, 120.0, 40.0);
        render(&mut ordinary, button_frame);
        assert!(ordinary.focus(Some(button_id), true));
    }

    #[derive(Debug)]
    struct LegacyRevealProbe {
        probe: WitnessProbe,
    }
    impl Component for LegacyRevealProbe {
        fn measure(&mut self, t: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
            self.probe.measure(t, a)
        }
        fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
            self.probe.arrange(t, f, c)
        }
        fn focusable(&self) -> bool {
            true
        }
        fn set_focus(&mut self, f: bool, v: bool) {
            self.probe.set_focus(f, v)
        }
        fn requires_focus_reveal(&self) -> bool {
            true
        }
    }

    #[test]
    fn invalid_updates_preserve_original_model_layout_and_required_evidence() {
        assert!(matches!(
            ChartContent::try_new("  ", vec![]),
            Err(ChartContentError::EmptyName)
        ));
        let mut chart = chart();
        let frame = Rect::new(0., 0., 320., 280.);
        chart
            .view
            .arrange_owner(&mut Text, &mut chart.model, frame, Some(frame));
        chart.view.focus(true, &mut chart.model);
        chart.model.select(1);
        chart.model.set_hovered(Some(2));
        let model = chart.model.clone();
        let layout = chart.view.layout().clone();
        let evidence = chart.required_focus_space();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                chart.update_data(vec![
                    ChartDatum::new("legal", 9.),
                    ChartDatum::new("bad", value)
                ]),
                Err(ChartContentError::NonFiniteValue { index: 1 })
            );
            assert_eq!(chart.model, model);
            assert_eq!(chart.view.layout(), &layout);
            assert_eq!(chart.required_focus_space(), evidence);
        }
        chart
            .update_data(vec![
                ChartDatum::new("", -0.),
                ChartDatum::new("", f64::MAX),
            ])
            .unwrap();
        assert_eq!(chart.selected_index(), Some(1));
        assert_eq!(chart.model.hovered(), None);
        assert_eq!(chart.required_focus_space(), RequiredFocusSpace::Unmeasured);
        chart.update_data(Vec::new()).unwrap();
        assert_eq!(chart.selected_index(), None);
    }

    #[test]
    fn late_pointer_up_cannot_select_or_activate_updated_data() {
        let id = ComponentId(9210);
        let mut host = ContentHost::new(Content::new(id, chart())).unwrap();
        let frame = Rect::new(0., 0., 320., 280.);
        render(&mut host, frame);
        let bar = host
            .get(id)
            .unwrap()
            .component::<ChartContent>()
            .unwrap()
            .view
            .layout()
            .geometry
            .bars[1]
            .rect;
        let point = Point::new(
            bar.min_x() + bar.size.width * 0.5,
            bar.min_y() + bar.size.height * 0.5,
        );
        assert!(host.event(&ContentEvent::PointerDown(point)).consumed);
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ChartContent>()
                .unwrap()
                .selected_index(),
            Some(1)
        );
        host.get_mut(id)
            .unwrap()
            .component_mut::<ChartContent>()
            .unwrap()
            .update_data(vec![ChartDatum::new("replacement", -4.)])
            .unwrap();
        render(&mut host, frame);
        let response = host.event(&ContentEvent::PointerUp(point));
        assert!(!response.changed && !response.activated);
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ChartContent>()
                .unwrap()
                .selected_index(),
            Some(0)
        );
        assert_eq!(
            host.get(id)
                .unwrap()
                .component::<ChartContent>()
                .unwrap()
                .data()[0]
                .label,
            "replacement"
        );
    }

    #[test]
    fn cancel_and_window_reactivation_do_not_restore_the_original_views_retired_hover() {
        for deactivate in [false, true] {
            let mut chart = chart();
            let frame = Rect::new(0., 0., 320., 280.);
            chart
                .view
                .arrange_owner(&mut Text, &mut chart.model, frame, Some(frame));
            chart.view.focus(true, &mut chart.model);
            let bar = chart.view.layout().geometry.bars[1].rect;
            let point = Point::new(
                bar.min_x() + bar.size.width * 0.5,
                bar.min_y() + bar.size.height * 0.5,
            );
            assert!(chart.view.pointer_move(point, &mut chart.model));
            assert_eq!(chart.model.hovered(), Some(1));
            let selected = chart.selected_index();
            let data = chart.data().to_vec();
            if deactivate {
                chart.window_active_changed(false);
                assert_eq!(chart.model.hovered(), None);
                chart.window_active_changed(true);
                assert!(!chart.model.is_focused());
            } else {
                chart.event(&ContentEvent::Cancel);
                assert!(chart.model.is_focused());
            }
            assert_eq!(chart.model.hovered(), None);
            // No fresh pointer input: painting must not copy a retired View
            // hover back into the same original BarChart model.
            chart.view.paint_owner(
                &mut Scene::new(),
                &mut Text,
                &mut chart.model,
                Theme::LIGHT.into(),
            );
            assert_eq!(chart.model.hovered(), None);
            assert_eq!(chart.selected_index(), selected);
            assert_eq!(chart.data(), data.as_slice());
        }
    }
}
