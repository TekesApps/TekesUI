//! Caller-owned outcome composition. Progress is measurement, never proof of success.
//! Reuses existing passive controls; this is a consumer fixture, not a Kit protocol.
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::*,
    extended::{Progress, StatusBadge},
    feedback_view::{ProgressView, SkeletonView, StatusBadgeView},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};
pub const STATUS: ComponentId = ComponentId(9821);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ready,
    Loading,
    Failed,
    Succeeded,
    Unverified,
    Cancelled,
}
impl Outcome {
    pub const ALL: [Self; 6] = [
        Self::Ready,
        Self::Loading,
        Self::Failed,
        Self::Succeeded,
        Self::Unverified,
        Self::Cancelled,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "就绪 · Ready",
            Self::Loading => "处理中 · Loading",
            Self::Failed => "失败 · Failed",
            Self::Succeeded => "已验证成功 · Succeeded",
            Self::Unverified => "尚未验证 · Unverified",
            Self::Cancelled => "已取消 · Cancelled",
        }
    }
    pub fn badge(self) -> StatusBadge {
        match self {
            Self::Ready | Self::Unverified | Self::Cancelled => StatusBadge::Info,
            Self::Loading => StatusBadge::Running,
            Self::Failed => StatusBadge::Failed,
            Self::Succeeded => StatusBadge::Succeeded,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct State {
    outcome: Outcome,
    value: Option<f32>,
}
impl State {
    fn progress_label(self) -> String {
        self.value.map_or_else(
            || "进度未知 · Progress unknown".into(),
            |v| format!("进度 · Progress {:.0}%", v * 100.),
        )
    }
}
#[derive(Debug)]
struct Feedback {
    state: Rc<Cell<State>>,
    badge: StatusBadgeView,
    progress: ProgressView,
    skeleton: SkeletonView,
}
impl Component for Feedback {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let state = self.state.get();
        let gap = Spacing::Regular.value();
        let badge = text.measure(
            state.outcome.label(),
            proposal::TextRole::UiBody,
            (available.width - gap * 2.).max(0.),
        );
        let label = text.measure(
            &state.progress_label(),
            proposal::TextRole::UiBody,
            available.width,
        );
        MeasuredChild::new(Size::new(
            available.width,
            badge.height
                + gap * 2.
                + label.height
                + gap * 2.
                + 12.
                + if state.outcome == Outcome::Loading && state.value.is_none() {
                    gap + 12.
                } else {
                    0.
                },
        ))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let state = self.state.get();
        let gap = Spacing::Regular.value();
        let width = frame.size.width;
        let measured = text.measure(
            state.outcome.label(),
            proposal::TextRole::UiBody,
            (width - gap * 2.).max(0.),
        );
        let badge = Rect::new(
            frame.min_x(),
            frame.min_y(),
            (measured.width + gap * 2.).min(width),
            measured.height + gap * 2.,
        );
        let layout = self.badge.render(
            scene,
            state.outcome.badge(),
            badge,
            Some(clip),
            theme.palette,
        );
        if let Some(visible) = layout.visible {
            scene.push_clip(visible);
            text.draw(
                scene,
                state.outcome.label(),
                proposal::TextRole::UiBody,
                Rect::new(
                    badge.min_x() + gap,
                    badge.min_y() + gap,
                    (badge.size.width - gap * 2.).max(0.),
                    measured.height,
                ),
                ButtonTextAlign::Center,
                StatusBadgeView::label_color(theme.palette),
            );
            scene.pop_clip();
        }
        let label = state.progress_label();
        let label_size = text.measure(&label, proposal::TextRole::UiBody, width);
        let label_frame = Rect::new(frame.min_x(), badge.max_y() + gap, width, label_size.height);
        if let Some(visible) = label_frame.intersection(&clip).filter(|r| !r.is_empty()) {
            scene.push_clip(visible);
            text.draw(
                scene,
                &label,
                proposal::TextRole::UiBody,
                label_frame,
                ButtonTextAlign::Leading,
                theme.palette.color("primary-text").unwrap(),
            );
            scene.pop_clip();
        }
        let bar = Rect::new(frame.min_x(), label_frame.max_y() + gap, width, 12.);
        let mut model = Progress::new(state.value.unwrap_or(0.));
        model.indeterminate = state.value.is_none();
        self.progress
            .render(scene, model, bar, Some(clip), theme.palette);
        if state.outcome == Outcome::Loading && state.value.is_none() {
            // A separate caller-owned loading placeholder; its presence is not a progress value.
            self.skeleton.render(
                scene,
                Rect::new(bar.min_x(), bar.max_y() + gap, width * 0.65, 12.),
                Some(clip),
                theme.palette,
            );
        }
    }
    fn semantics(&self) -> ContentSemantics {
        let state = self.state.get();
        let mut s = ContentSemantics::new(ContentRole::Group, "结果与进度 · Outcome and progress");
        s.value = Some(format!(
            "{}；{}",
            state.outcome.label(),
            state.progress_label()
        ));
        s.invalid = state.outcome == Outcome::Failed;
        s.description = Some("进度与验证结果独立；未知进度不显示百分比".into());
        s
    }
}
#[derive(Debug)]
pub struct Scenario {
    pub host: ContentHost,
    state: Rc<Cell<State>>,
}
impl Scenario {
    pub fn new() -> Result<Self, CompositionError> {
        let state = Rc::new(Cell::new(State {
            outcome: Outcome::Unverified,
            value: None,
        }));
        let host = ContentHost::new(Content::new(
            STATUS,
            Feedback {
                state: state.clone(),
                badge: Default::default(),
                progress: Default::default(),
                skeleton: Default::default(),
            },
        ))?;
        Ok(Self { host, state })
    }
    /// Caller publishes measured progress and independently determined outcome.
    /// Invalid measurements are rejected atomically; no result is derived from a percentage.
    pub fn set(&mut self, outcome: Outcome, value: Option<f32>) -> bool {
        if value.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v)) {
            return false;
        }
        let next = State { outcome, value };
        if self.state.get() == next {
            return false;
        }
        self.state.set(next);
        self.host.get_mut(STATUS);
        true
    }
    pub fn outcome(&self) -> Outcome {
        self.state.get().outcome
    }
    pub fn progress(&self) -> Option<f32> {
        self.state.get().value
    }
}
