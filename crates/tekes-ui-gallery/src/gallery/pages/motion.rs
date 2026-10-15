//! Motion page: same JSON colors/type/spacing, Scene and TextEngine as Gallery.
//! No host, window, renderer, timers or extra UI toolkit is created here.
use super::PageCtx;
use crate::gallery::motion_preview::{MotionPreview, Parameter};
use tekes_ui_platform_macos::{Event, MouseButton, MouseEventKind};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{CornerRadius, Rgba, Spacing, Stroke, Theme, proposal};

pub const HEIGHT: f32 = 816.0;
const TARGETS: [f64; 3] = [0.15, 0.5, 0.85];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Preset(usize),
    Target(usize),
    Replay,
    Interrupt,
    ReduceMotion,
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub content: Rect,
    pub stage: Rect,
    pub graph: Rect,
    pub inspector: Rect,
}
impl Layout {
    pub fn new(content: Rect) -> Self {
        // Keep the same minimum document width as extent(). Narrow windows
        // reveal this fixed fixture through the existing horizontal viewport.
        let content = Rect::new(
            content.min_x(),
            content.min_y(),
            content.size.width.max(840.0),
            HEIGHT,
        );
        let gap = Spacing::Loose.value();
        let side = 280.0;
        let left = content.size.width - side - gap;
        Self {
            content,
            stage: Rect::new(content.min_x(), content.min_y() + 144.0, left, 240.0),
            graph: Rect::new(content.min_x(), content.min_y() + 420.0, left, 180.0),
            inspector: Rect::new(
                content.min_x() + left + gap,
                content.min_y() + 144.0,
                side,
                456.0,
            ),
        }
    }
    pub fn button(self, action: Action) -> Rect {
        let x = self.content.min_x();
        let y = self.content.min_y();
        let gap = Spacing::Regular.value();
        match action {
            Action::Preset(i) => {
                let width = (self.content.size.width - 4.0 * gap) / 5.0;
                Rect::new(
                    x + (i % 5) as f32 * (width + gap),
                    y + 32.0 + (i / 5) as f32 * 42.0,
                    width,
                    34.0,
                )
            }
            Action::Target(i) => Rect::new(x + i as f32 * 92.0, y + 626.0, 84.0, 34.0),
            Action::Replay => Rect::new(x + 284.0, y + 626.0, 84.0, 34.0),
            Action::Interrupt => Rect::new(x + 376.0, y + 626.0, 140.0, 34.0),
            Action::ReduceMotion => Rect::new(
                self.inspector.min_x() + 16.0,
                self.inspector.max_y() - 52.0,
                self.inspector.size.width - 32.0,
                34.0,
            ),
        }
    }
    pub fn slider(self, p: Parameter) -> Rect {
        let i = Parameter::ALL.iter().position(|a| *a == p).unwrap();
        Rect::new(
            self.inspector.min_x() + 20.0,
            self.inspector.min_y() + 72.0 + i as f32 * 82.0,
            self.inspector.size.width - 40.0,
            24.0,
        )
    }
    pub fn actions(self, count: usize) -> Vec<(Action, Rect)> {
        (0..count)
            .map(Action::Preset)
            .chain((0..3).map(Action::Target))
            .chain([Action::Replay, Action::Interrupt, Action::ReduceMotion])
            .map(|a| (a, self.button(a)))
            .collect()
    }
    pub fn track(self) -> Rect {
        self.stage.inset(32.0)
    }
}

pub fn extent(content: Rect) -> Size {
    Size::new(content.size.width.max(840.0), HEIGHT)
}

/// Pass `Viewport::content()` and raw window event positions together; they
/// share the translated coordinate space. Only call after the viewport's bounds
/// routing. A captured slider must continue receiving drag/up outside bounds.
pub fn handle_input(state: &mut MotionPreview, event: &Event, content: Rect) -> bool {
    let layout = Layout::new(content);
    let set_from_x = |state: &mut MotionPreview, p: Parameter, x: f32| {
        let r = layout.slider(p);
        let unit = ((x - r.min_x()) / r.size.width).clamp(0.0, 1.0);
        let (lo, hi) = p.range();
        state.set_parameter(p, lo + f64::from(unit) * (hi - lo))
    };
    match event {
        Event::Focus(false) => {
            let dirty =
                state.dragging.is_some() || state.hovered.is_some() || state.pressed.is_some();
            state.deactivate();
            dirty
        }
        Event::Mouse(mouse) => {
            let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
            let actions = layout.actions(state.presets.len());
            let hit = actions.iter().position(|(_, r)| r.contains(point));
            match mouse.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    ..
                } => {
                    for p in Parameter::ALL {
                        if layout.slider(p).contains(point) && state.parameter_enabled(p) {
                            state.dragging = Some(p);
                            set_from_x(state, p, point.x);
                            return true;
                        }
                    }
                    if let Some(index) = hit {
                        state.pressed = Some(index);
                        return true;
                    }
                    if layout.stage.contains(point) {
                        let track = layout.track();
                        state.retarget(f64::from((point.x - track.min_x()) / track.size.width));
                        return true;
                    }
                    false
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } => state
                    .dragging
                    .is_some_and(|p| set_from_x(state, p, point.x)),
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => {
                    if state.dragging.take().is_some() {
                        return true;
                    }
                    let pressed = state.pressed.take();
                    if let Some(index) = pressed.filter(|i| Some(*i) == hit) {
                        match actions[index].0 {
                            Action::Preset(i) => {
                                state.select(i);
                            }
                            Action::Target(i) => state.retarget(TARGETS[i]),
                            Action::Replay => state.replay(),
                            Action::Interrupt => state.demonstrate_interrupt(),
                            Action::ReduceMotion => {
                                state.set_reduced_motion(!state.manual_reduced_motion());
                            }
                        }
                    }
                    pressed.is_some()
                }
                MouseEventKind::Moved | MouseEventKind::Entered => {
                    let changed = state.hovered != hit;
                    state.hovered = hit;
                    changed
                }
                MouseEventKind::Exited => state.hovered.take().is_some(),
                _ => false,
            }
        }
        _ => false,
    }
}

fn color(theme: Theme, id: &str) -> Rgba {
    proposal::render_color(id, theme).expect("validated motion page color")
}

struct Painter<'a, 'b> {
    ctx: &'a mut PageCtx<'b>,
    scene: &'a mut Scene,
}
impl Painter<'_, '_> {
    fn quad(&mut self, r: Rect, fill: &str, radius: f32, border: bool) {
        let mut quad = Quad::new(r, color(self.ctx.theme, fill)).with_corner_radius(radius);
        if border {
            quad = quad.with_border(Stroke::Rule.value(), color(self.ctx.theme, "border"));
        }
        self.scene.push_quad(quad);
    }
    fn label(&mut self, r: Rect, text: &str, role: proposal::TextRole, ink: &str) {
        self.ctx.text.proposal_label_in(
            self.scene,
            self.ctx.theme,
            r,
            text,
            role,
            color(self.ctx.theme, ink),
        );
    }
    fn body(&mut self, r: Rect, text: &str) {
        self.label(r, text, proposal::TextRole::UiBody, "primary-text");
    }
    fn caption(&mut self, r: Rect, text: &str) {
        self.label(r, text, proposal::TextRole::UiCaption, "secondary-text");
    }
    // Per-digit cached paragraphs prevent 120Hz telemetry from generating an
    // unbounded set of label shapes or repeatedly invoking CoreText shaping.
    fn number(&mut self, r: Rect, value: f64) {
        self.digits(r, &format!("{value:.2}"), "primary-text");
    }
    fn digits(&mut self, r: Rect, text: &str, ink: &str) {
        let mut x = r.min_x();
        for c in text.chars() {
            let digit = c.to_string();
            let width = self
                .ctx
                .text
                .proposal_label(&digit, proposal::TextRole::UiBody, None)
                .width
                .ceil() as f32;
            self.label(
                Rect::new(x, r.min_y(), width, r.size.height),
                &digit,
                proposal::TextRole::UiBody,
                ink,
            );
            x += width;
        }
    }
    fn line(&mut self, a: Point, b: Point, ink: &str) {
        let count = ((b.x - a.x).abs().max((b.y - a.y).abs()) / 1.5)
            .ceil()
            .max(1.0) as usize;
        for i in 0..=count {
            let t = i as f32 / count as f32;
            self.quad(
                Rect::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, 1.5, 1.5),
                ink,
                0.75,
                false,
            );
        }
    }
}

pub fn build(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene, state: &MotionPreview) {
    let layout = Layout::new(content);
    let content = layout.content;
    let x = content.min_x();
    let y = content.min_y();
    let mut p = Painter { ctx, scene };
    let radius = CornerRadius::Panel.value();
    // Inspector background precedes its interactive Reduce Motion button.
    p.quad(layout.inspector, "surface-panel", radius, true);
    p.caption(
        Rect::new(x, y, content.size.width, 24.0),
        "MODEL / PRESET     选择设计系统预设，或拖动参数调整运动",
    );
    for (index, (action, rect)) in layout.actions(state.presets.len()).iter().enumerate() {
        let selected = match action {
            Action::Preset(i) => *i == state.selected,
            Action::ReduceMotion => state.reduced_motion,
            _ => false,
        };
        let fill = if state.pressed == Some(index) {
            "button-pressed"
        } else if selected {
            "row-selection"
        } else if state.hovered == Some(index) {
            "button-hover"
        } else {
            "surface-panel"
        };
        p.quad(*rect, fill, CornerRadius::Control.value(), true);
        let label = match action {
            Action::Preset(i) => state.presets[*i].label.as_str(),
            Action::Target(0) => "A · 0.15",
            Action::Target(1) => "B · 0.50",
            Action::Target(_) => "C · 0.85",
            Action::Replay => "重播",
            Action::Interrupt => "演示中途改目标",
            Action::ReduceMotion => {
                if state.reduced_motion {
                    "减少动态效果 · 开"
                } else {
                    "减少动态效果 · 关"
                }
            }
        };
        let role = proposal::TextRole::UiBody;
        let label_width = p.ctx.text.proposal_label(label, role, None).width as f32;
        p.body(
            Rect::new(
                rect.min_x() + (rect.size.width - label_width).max(0.0) * 0.5,
                rect.min_y(),
                label_width.min(rect.size.width),
                rect.size.height,
            ),
            label,
        );
    }

    p.quad(layout.stage, "surface-panel", radius, true);
    p.caption(
        Rect::new(
            layout.stage.min_x() + 24.0,
            layout.stage.min_y() + 16.0,
            layout.stage.size.width - 48.0,
            24.0,
        ),
        "LIVE PREVIEW     点击轨道或 A / B / C，运动中也可切换",
    );
    let track = layout.track();
    let middle = layout.stage.center().y;
    p.quad(
        Rect::new(track.min_x(), middle - 1.0, track.size.width, 2.0),
        "separator",
        1.0,
        false,
    );
    for mark in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let mx = track.min_x() + track.size.width * mark;
        p.quad(Rect::new(mx, middle - 5.0, 1.0, 10.0), "border", 0.0, false);
    }
    let sample = state.channel.current();
    let target_x = track.min_x() + state.channel.target() as f32 * track.size.width;
    p.quad(
        Rect::new(target_x - 19.0, middle - 19.0, 38.0, 38.0),
        "surface-panel",
        19.0,
        true,
    );
    p.caption(
        Rect::new(target_x - 20.0, middle + 30.0, 60.0, 22.0),
        "目标",
    );
    p.scene.push_clip(layout.stage);
    let ball_x = track.min_x() + sample.value as f32 * track.size.width;
    p.quad(
        Rect::new(ball_x - 15.0, middle - 15.0, 30.0, 30.0),
        "primary-text",
        15.0,
        false,
    );
    p.scene.pop_clip();
    let status = if state.reduced_motion {
        "Reduce Motion · 即时到达"
    } else if state.channel.is_active() {
        "运行中 · 可随时改目标"
    } else {
        "已停止 · 无动画帧请求"
    };
    p.caption(
        Rect::new(
            track.min_x(),
            layout.stage.max_y() - 40.0,
            track.size.width,
            24.0,
        ),
        status,
    );

    let row = layout.stage.max_y() + 8.0;
    for (i, (label, value)) in [
        ("位置", sample.value),
        ("速度 / s", sample.velocity),
        ("目标", state.channel.target()),
    ]
    .into_iter()
    .enumerate()
    {
        let mx = x + i as f32 * 180.0;
        p.caption(Rect::new(mx, row, 80.0, 24.0), label);
        p.number(Rect::new(mx + 82.0, row, 96.0, 24.0), value);
    }

    p.quad(layout.graph, "surface-panel", radius, true);
    p.caption(
        Rect::new(
            layout.graph.min_x() + 20.0,
            layout.graph.min_y() + 10.0,
            layout.graph.size.width - 40.0,
            24.0,
        ),
        "TRAJECTORY     实线：位置    虚线：目标    下方：速度",
    );
    let graph = Rect::new(
        layout.graph.min_x() + 20.0,
        layout.graph.min_y() + 42.0,
        layout.graph.size.width - 40.0,
        100.0,
    );
    let bottom = graph.max_y();
    let end = state.now.as_secs_f64().max(1.0);
    let start = (end - 4.0).max(0.0);
    let duration = (end - start).max(1.0);
    let to_x = |t: f64| graph.min_x() + ((t - start) / duration) as f32 * graph.size.width;
    p.quad(
        Rect::new(graph.min_x(), bottom, graph.size.width, 1.0),
        "separator",
        0.0,
        false,
    );
    p.scene.push_clip(graph);
    let points: Vec<_> = state.trace.iter().filter(|s| s.time >= start).collect();
    let stride = points.len().div_ceil(160).max(1);
    let mut previous: Option<(Point, Point)> = None;
    for (index, point) in points.iter().step_by(stride).enumerate() {
        let px = to_x(point.time);
        let pos = Point::new(px, bottom - 20.0 - point.value as f32 * 65.0);
        let velocity = Point::new(
            px,
            bottom - 8.0 - point.velocity.clamp(-6.0, 6.0) as f32 * 1.0,
        );
        if let Some((old_pos, old_velocity)) = previous {
            p.line(old_pos, pos, "primary-text");
            p.line(old_velocity, velocity, "secondary-text");
        }
        if index % 2 == 0 {
            p.quad(
                Rect::new(px, bottom - 20.0 - point.target as f32 * 65.0, 3.0, 1.5),
                "tertiary-text",
                0.0,
                false,
            );
        }
        previous = Some((pos, velocity));
    }
    p.scene.pop_clip();
    p.caption(
        Rect::new(graph.min_x(), layout.graph.max_y() - 30.0, 100.0, 22.0),
        "时间 / s",
    );
    p.number(
        Rect::new(
            graph.max_x() - 64.0,
            layout.graph.max_y() - 30.0,
            64.0,
            22.0,
        ),
        state.now.as_secs_f64(),
    );

    p.body(
        Rect::new(
            layout.inspector.min_x() + 20.0,
            layout.inspector.min_y() + 12.0,
            240.0,
            28.0,
        ),
        "PARAMETERS / 参数",
    );
    for parameter in Parameter::ALL {
        let r = layout.slider(parameter);
        let enabled = state.parameter_enabled(parameter);
        let ink = if enabled {
            "primary-text"
        } else {
            "disabled-text"
        };
        p.label(
            Rect::new(r.min_x(), r.min_y() - 28.0, 170.0, 24.0),
            parameter.label(),
            proposal::TextRole::UiCaption,
            ink,
        );
        p.digits(
            Rect::new(r.max_x() - 56.0, r.min_y() - 28.0, 56.0, 24.0),
            &format!("{:.2}", state.parameter_value(parameter)),
            ink,
        );
        p.quad(
            Rect::new(r.min_x(), r.center().y - 2.0, r.size.width, 4.0),
            "separator",
            2.0,
            false,
        );
        let (lo, hi) = parameter.range();
        let unit = ((state.parameter_value(parameter) - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
        if enabled {
            p.quad(
                Rect::new(r.min_x(), r.center().y - 2.0, r.size.width * unit, 4.0),
                ink,
                2.0,
                false,
            );
        }
        p.quad(
            Rect::new(
                r.min_x() + r.size.width * unit - 7.0,
                r.center().y - 7.0,
                14.0,
                14.0,
            ),
            if enabled {
                "primary-text"
            } else {
                "disabled-text"
            },
            7.0,
            false,
        );
    }

    let source = if state.custom {
        "当前为实验参数；重新选择预设可恢复 JSON 值"
    } else {
        "预设数值来自现有 JSON Design System"
    };
    p.caption(Rect::new(x, y + 690.0, content.size.width, 24.0), source);
    p.caption(
        Rect::new(x, y + 716.0, content.size.width, 24.0),
        &state.presets[state.selected].source,
    );
    p.caption(
        Rect::new(x, y + 742.0, content.size.width, 24.0),
        "模型参考 Apple 官方机制；参数来自项目设计系统，非 Apple 系统控件常量。",
    );
    if let Some(error) = &state.error {
        p.body(Rect::new(x, y + 768.0, content.size.width, 24.0), error);
    } else if let Some(event) = state.events.front() {
        // Generation uses cached digits; the event tail has a finite vocabulary.
        if let Some((number, label)) = event.split_once("  ") {
            p.digits(
                Rect::new(x, y + 768.0, 80.0, 24.0),
                number,
                "secondary-text",
            );
            p.caption(
                Rect::new(x + 80.0, y + 768.0, content.size.width - 80.0, 24.0),
                label,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_platform_macos::{Modifiers, MouseEvent, Point as NativePoint};
    fn mouse(kind: MouseEventKind, p: Point) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            position: NativePoint {
                x: f64::from(p.x),
                y: f64::from(p.y),
            },
            modifiers: Modifiers::NONE,
        })
    }
    #[test]
    fn every_action_uses_same_layout_for_drawing_and_hit_routing() {
        let content = Rect::new(312.0, 80.0, 1104.0, 790.0);
        let layout = Layout::new(content);
        let mut state = MotionPreview::default();
        for (action, rect) in layout.actions(state.presets.len()) {
            assert!(rect.min_x() >= content.min_x() && rect.max_x() <= content.max_x());
            let at = rect.center();
            assert!(handle_input(
                &mut state,
                &mouse(
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        click_count: 1
                    },
                    at
                ),
                content
            ));
            assert!(handle_input(
                &mut state,
                &mouse(
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        click_count: 1
                    },
                    at
                ),
                content
            ));
            if let Action::Preset(i) = action {
                assert_eq!(state.selected, i);
            }
        }
    }
    #[test]
    fn slider_capture_drag_and_focus_loss() {
        let content = Rect::new(312.0, 80.0, 1104.0, 790.0);
        let layout = Layout::new(content);
        let mut state = MotionPreview::default();
        let p = layout.slider(Parameter::Frequency).center();
        handle_input(
            &mut state,
            &mouse(
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                p,
            ),
            content,
        );
        assert_eq!(state.dragging, Some(Parameter::Frequency));
        handle_input(
            &mut state,
            &mouse(
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                },
                Point::new(3000.0, p.y),
            ),
            content,
        );
        assert_eq!(state.frequency, Parameter::Frequency.range().1);
        assert!(handle_input(&mut state, &Event::Focus(false), content));
        assert!(state.dragging.is_none());
    }

    #[test]
    fn manual_toggle_is_independent_of_system_reduction_in_the_real_hit_path() {
        let content = Rect::new(312.0, 80.0, 1104.0, HEIGHT);
        let mut state = MotionPreview::default();
        state.set_system_reduced_motion(true);
        let point = Layout::new(content).button(Action::ReduceMotion).center();
        for (kind, count) in [
            (
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                0,
            ),
            (
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                1,
            ),
        ] {
            handle_input(&mut state, &mouse(kind, point), content);
            assert_eq!(state.manual_reduced_motion(), count == 1);
            assert!(state.reduced_motion);
        }
        state.set_system_reduced_motion(false);
        assert!(state.reduced_motion);
        assert!(!state.needs_animation());
    }

    #[test]
    fn translated_and_narrow_document_layout_keeps_controls_and_footer_in_extent() {
        for width in [100.0, 640.0, 840.0, 1104.0] {
            let content = Rect::new(-40.0, -230.0, width, 200.0);
            let layout = Layout::new(content);
            assert_eq!(layout.content.size, extent(content));
            for (_, rect) in layout.actions(10) {
                assert!(rect.min_x() >= layout.content.min_x());
                assert!(rect.max_x() <= layout.content.max_x());
                assert!(rect.min_y() >= layout.content.min_y());
                assert!(rect.max_y() <= layout.content.max_y());
            }
            assert!(layout.content.min_y() + 768.0 + 24.0 < layout.content.max_y());
            assert!(layout.stage.size.width >= 544.0);
            for parameter in Parameter::ALL {
                assert!(layout.slider(parameter).size.width > 0.0);
            }
        }
    }

    #[test]
    fn rebuilds_cache_telemetry_and_keep_reduce_motion_button_above_background() {
        use crate::gallery::{static_text::StaticText, text::TextEngine};
        use tekes_ui_scene::Primitive;
        let content = Rect::new(24.0, 24.0, 1104.0, HEIGHT);
        let mut text = TextEngine::new();
        let mut fixture = StaticText::new(None);
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                text.set_scale(scale);
                let mut ctx = PageCtx {
                    theme,
                    text: &mut text,
                    static_text: &mut fixture,
                };
                let mut scene = Scene::new();
                let mut state = MotionPreview::default();
                // Prime the finite telemetry alphabet and all lifecycle labels.
                Painter {
                    ctx: &mut ctx,
                    scene: &mut scene,
                }
                .digits(content, "0123456789.-#", "primary-text");
                build(&mut ctx, content, &mut scene, &state);
                state.replay();
                build(&mut ctx, content, &mut scene, &state);
                state.animate(2.0);
                build(&mut ctx, content, &mut scene, &state);
                let shapes = ctx.text.shape_count();
                state.replay();
                for _ in 0..240 {
                    state.animate(1.0 / 120.0);
                    ctx.text.begin_frame();
                    scene.clear();
                    build(&mut ctx, content, &mut scene, &state);
                    assert_eq!(
                        ctx.text.shape_count(),
                        shapes,
                        "telemetry must reuse cached shapes"
                    );
                }
                state.set_reduced_motion(true);
                ctx.text.begin_frame();
                scene.clear();
                build(&mut ctx, content, &mut scene, &state);
                let center = Layout::new(content).button(Action::ReduceMotion).center();
                let top = scene
                    .primitives()
                    .iter()
                    .rev()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect.contains(center) => Some(q),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(top.fill, color(theme, "row-selection"));
                assert!(scene.primitives().iter().all(|p| match p {
                    Primitive::Quad(q) =>
                        q.rect.size.width.is_finite() && q.rect.size.height.is_finite(),
                    _ => true,
                }));
            }
        }
    }
}
