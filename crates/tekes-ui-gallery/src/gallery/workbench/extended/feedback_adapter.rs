//! Gallery adapters for the production feedback and overlay views.
//!
//! The story keeps its deterministic fixture models; these runtimes own their
//! component geometry, clipping, transient pointer state and presentation.
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    extended::{Banner, EmptyState, Popover, Progress, StatusBadge, Toast, Tooltip},
    feedback_view::{
        BannerView, EmptyStateView, OverlayEvent, OverlayLayout, ProgressView, StatusBadgeView,
        SurfaceView, ToastView, TooltipEvent, TooltipView, ViewLayout,
    },
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{ColorRole, Palette, proposal};

use super::super::{Entry, PageCtx, StoryState, action, line, quiet_action};
use super::collections_adapter::InputResult;
use super::{ExtendedStory, controls};

#[derive(Debug, Default)]
pub(super) struct FeedbackViews {
    pub tooltip: TooltipView,
    pub popover: tekes_ui_controls::feedback_view::PopoverView,
    pub toast: ToastView,
    pub banner: BannerView,
    pub status_badge: StatusBadgeView,
    pub progress: ProgressView,
    pub empty_state: EmptyStateView,
    pub surface: SurfaceView,
}

pub(super) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "tooltip" | "popover" | "toast" | "banner" | "status-badge" | "progress" | "empty-state"
    )
}

fn draw_status_badge_label(
    text: &mut super::super::super::text::TextEngine,
    scene: &mut Scene,
    theme: tekes_ui_tokens::Theme,
    frame: Rect,
    label: &str,
    palette: Palette,
) {
    text.proposal_label_in(
        scene,
        theme,
        frame.inset(5.0),
        label,
        proposal::TextRole::UiCaption,
        StatusBadgeView::label_color(palette),
    );
}

/// Draws one feedback story from the existing fixtures. Layout and inner
/// button hit targets come from the production presentation views.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    ext: &mut ExtendedStory,
    entry: &Entry,
    state: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    body: Rect,
    clip: Rect,
    palette: Palette,
) -> bool {
    let renderer = entry.renderer();
    if !supports(renderer) {
        return false;
    }
    let clip = body
        .intersection(&clip)
        .unwrap_or(Rect::new(body.min_x(), body.min_y(), 0.0, 0.0));
    state.hits.retain(|(id, _)| !component_id(renderer, *id));
    state.button_hits.retain(|id| !component_id(renderer, *id));
    let x = body.min_x();
    let y = body.min_y();
    let width = body.size.width.max(0.0);
    let focus_visible = state.focus.is_some_and(|id| {
        state
            .buttons
            .get(&id)
            .is_some_and(|button| button.state.focus_visible)
    });
    match renderer {
        "tooltip" => {
            let anchor = Rect::new(x, y + 16.0, 164.0, 36.0);
            action(
                state,
                ctx,
                scene,
                20,
                anchor,
                "Hover or focus me",
                (false, false),
            );
            ext.feedback_views
                .tooltip_set_focus(state.focus == Some(20), focus_visible);
            ext.feedback_views.render_tooltip(
                scene,
                ctx.text,
                &ext.tooltip,
                anchor,
                Size::new(276.0, 48.0),
                body,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            line(
                ctx,
                scene,
                Rect::new(x, y + 122.0, width, 24.0),
                "Tooltip appears on hover or focus-visible. Escape dismisses it.",
            );
        }
        "popover" => {
            let anchor = Rect::new(x, y + 10.0, 150.0, 36.0);
            action(
                state,
                ctx,
                scene,
                20,
                anchor,
                "More options",
                (ext.popover.open, false),
            );
            ext.feedback_views
                .popover_focus_dismiss(state.focus == Some(21), focus_visible);
            if ext.popover.open {
                let layout = ext.feedback_views.render_popover_with_content(
                    scene,
                    ctx.text,
                    &ext.popover,
                    anchor,
                    Size::new(width.min(232.0), 32.0),
                    body,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                    |_, _| {},
                );
                if let Some(visible) = layout.visible {
                    scene.push_clip(visible);
                    ctx.text.proposal_label_in(
                        scene,
                        ctx.theme,
                        layout.content,
                        "Popover content",
                        proposal::TextRole::UiBody,
                        ColorRole::PrimaryText.resolve(ctx.theme),
                    );
                    scene.pop_clip();
                }
                if let Some(hit) = ext.feedback_views.popover_focus_target() {
                    state.hits.push((21, hit));
                }
            }
            line(
                ctx,
                scene,
                Rect::new(x, y + 170.0, width, 24.0),
                "Escape or outside click dismisses and restores the invoker focus.",
            );
        }
        "toast" => {
            if ext.toast.visible {
                let toast_width = width.min(520.0);
                let toast_size = ToastView::measure(
                    ctx.text,
                    &ext.toast.message,
                    toast_width,
                    proposal::TextRole::UiBody,
                );
                let frame = Rect::new(x, y + 12.0, toast_width, toast_size.height);
                ext.feedback_views.render_toast(
                    scene,
                    ctx.text,
                    &ext.toast,
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                if let Some(hit) = ext.feedback_views.toast_focus_target() {
                    state.hits.push((20, hit));
                }
                let timer = format!("{} ms remaining", ext.toast.remaining_ms());
                let toast_bottom = frame.max_y();
                let (action_frame, timer_frame, note_frame) = toast_fixture_layout(
                    ctx,
                    x,
                    y,
                    width,
                    toast_bottom,
                    &timer,
                    "Time advances only when the fixture timer control is activated.",
                    state.density,
                );
                quiet_action(
                    state,
                    ctx,
                    scene,
                    21,
                    action_frame,
                    "Advance fixture timer",
                    (false, false),
                );
                draw_measured_caption(ctx, scene, timer_frame, &timer, 28.0);
                draw_measured_caption(
                    ctx,
                    scene,
                    note_frame,
                    "Time advances only when the fixture timer control is activated.",
                    24.0,
                );
            } else {
                line(
                    ctx,
                    scene,
                    Rect::new(x, y + 20.0, width, 28.0),
                    "Toast dismissed.",
                );
                quiet_action(
                    state,
                    ctx,
                    scene,
                    21,
                    Rect::new(x, y + 54.0, 112.0, 34.0),
                    "Show again",
                    (false, false),
                );
                line(
                    ctx,
                    scene,
                    Rect::new(x, y + 126.0, width, 24.0),
                    "Time advances only when the fixture timer control is activated.",
                );
            }
        }
        "banner" => {
            if ext.banner.visible {
                let frame = Rect::new(x, y + 12.0, width.min(640.0), 74.0);
                ext.feedback_views.render_banner(
                    scene,
                    ctx.text,
                    &ext.banner,
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                if let Some(hit) = ext.feedback_views.banner_focus_target() {
                    state.hits.push((20, hit));
                }
            } else {
                line(
                    ctx,
                    scene,
                    Rect::new(x, y + 20.0, width, 28.0),
                    "Banner dismissed. Reset restores the initial fixture.",
                );
            }
        }
        "status-badge" => {
            for (index, status) in [
                StatusBadge::Info,
                StatusBadge::Running,
                StatusBadge::Succeeded,
                StatusBadge::Failed,
            ]
            .into_iter()
            .enumerate()
            {
                let frame = Rect::new(x + index as f32 * 116.0, y + 20.0, 106.0, 28.0);
                ext.feedback_views
                    .render_status_badge(scene, status, frame, Some(clip), palette);
                draw_status_badge_label(
                    ctx.text,
                    scene,
                    ctx.theme,
                    frame,
                    ["Info", "Running", "Succeeded", "Failed"][index],
                    palette,
                );
            }
        }
        "progress" => {
            let frame = Rect::new(x, y + 26.0, width.min(500.0), 12.0);
            ext.feedback_views
                .render_progress(scene, ext.progress, frame, Some(clip), palette);
            let percentage = format!("{:.0}%", ext.progress.value() * 100.0);
            let (button_frame, percentage_frame) =
                progress_fixture_layout(ctx, x, y, width, &percentage, state.density);
            action(
                state,
                ctx,
                scene,
                20,
                button_frame,
                "Advance 10%",
                (false, ext.progress.value() >= 1.0),
            );
            let natural = measured_body_size(ctx, &percentage, percentage_frame.size.width);
            if natural.height <= 34.0 {
                ctx.text.proposal_label_in(
                    scene,
                    ctx.theme,
                    Rect::new(
                        percentage_frame.min_x(),
                        percentage_frame.min_y(),
                        percentage_frame.size.width,
                        34.0,
                    ),
                    &percentage,
                    proposal::TextRole::UiBody,
                    ColorRole::PrimaryText.resolve(ctx.theme),
                );
            } else {
                wrapped_body(ctx, scene, percentage_frame, &percentage);
            }
        }
        "empty-state" => {
            let model = if ext.empty_state_created {
                EmptyState {
                    title: "Documents".into(),
                    detail: "One fixture document was created.".into(),
                    action: None,
                }
            } else {
                EmptyState {
                    title: "No documents yet".into(),
                    detail: "Create a document to begin your workspace.".into(),
                    action: Some("Create document".into()),
                }
            };
            let frame = Rect::new(x, y + 12.0, width.min(520.0), 164.0);
            let backing_scale = ctx.text.scale();
            let panel = ext.feedback_views.surface.render_at_scale(
                scene,
                frame,
                Some(clip),
                palette,
                true,
                backing_scale,
            );
            ext.feedback_views.empty_state_set_disabled(ext.disabled);
            ext.feedback_views
                .empty_state_focus_action(state.focus == Some(20), focus_visible);
            ext.feedback_views.render_empty_state(
                scene,
                ctx.text,
                &model,
                panel.content,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            if let Some(hit) = ext.feedback_views.empty_state_focus_target() {
                state.hits.push((20, hit));
            }
        }
        _ => return false,
    }
    true
}

fn measured_action_size(
    ctx: &mut PageCtx<'_>,
    label: &str,
    max_width: f32,
    baseline_width: f32,
    baseline_height: f32,
    density: usize,
) -> Size {
    let measured = ctx.text.measure_button(
        label,
        max_width.max(0.0),
        tekes_ui_controls::ButtonSize::Body,
        proposal::TextRole::UiBody,
    );
    // The shared Gallery action helper applies the density reduction to the
    // supplied frame. Compensate here so the measured text slot stays intact.
    let density_compensation = if density == 1 { 4.0 } else { 0.0 };
    Size::new(
        measured.width.max(baseline_width).min(max_width.max(0.0)),
        measured.height.max(baseline_height) + density_compensation,
    )
}

fn measured_caption_size(ctx: &mut PageCtx<'_>, text: &str, width: f32) -> Size {
    ctx.text
        .measure(text, proposal::TextRole::UiCaption, width.max(0.0))
}

fn measured_body_size(ctx: &mut PageCtx<'_>, text: &str, width: f32) -> Size {
    ctx.text
        .measure(text, proposal::TextRole::UiBody, width.max(0.0))
}

fn wrapped_body(ctx: &mut PageCtx<'_>, scene: &mut Scene, rect: Rect, text: &str) {
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        rect,
        text,
        proposal::TextRole::UiBody,
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
}

fn draw_measured_caption(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    measured: Rect,
    text: &str,
    legacy_height: f32,
) {
    let size = measured_caption_size(ctx, text, measured.size.width);
    if size.height <= legacy_height {
        line(
            ctx,
            scene,
            Rect::new(
                measured.min_x(),
                measured.min_y(),
                measured.size.width,
                legacy_height,
            ),
            text,
        );
    } else {
        ctx.text.proposal_wrapped_in(
            scene,
            ctx.theme,
            Rect::new(
                measured.min_x(),
                measured.min_y(),
                measured.size.width,
                size.height,
            ),
            text,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(ctx.theme),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn toast_fixture_layout(
    ctx: &mut PageCtx<'_>,
    x: f32,
    y: f32,
    width: f32,
    toast_bottom: f32,
    timer: &str,
    note: &str,
    density: usize,
) -> (Rect, Rect, Rect) {
    let action_size =
        measured_action_size(ctx, "Advance fixture timer", width, 156.0, 34.0, density);
    let timer_natural = measured_caption_size(ctx, timer, width);
    let controls_y = (y + 84.0).max(toast_bottom + 8.0);
    let action = Rect::new(x, controls_y, action_size.width, action_size.height);
    let inline = width >= action_size.width + 10.0 + timer_natural.width;
    let timer_x = if inline { action.max_x() + 10.0 } else { x };
    let timer_y = if inline {
        controls_y
    } else {
        action.max_y() + 8.0
    };
    let timer_rect_width = (x + width - timer_x).max(0.0);
    let timer_size = measured_caption_size(ctx, timer, timer_rect_width);
    let timer_rect = Rect::new(timer_x, timer_y, timer_rect_width, timer_size.height);
    let row_bottom = action.max_y().max(timer_rect.max_y());
    let note_y = (y + 126.0).max(row_bottom + 8.0);
    let note_size = measured_caption_size(ctx, note, width);
    (
        action,
        timer_rect,
        Rect::new(x, note_y, width, note_size.height),
    )
}

fn progress_fixture_layout(
    ctx: &mut PageCtx<'_>,
    x: f32,
    y: f32,
    width: f32,
    percentage: &str,
    density: usize,
) -> (Rect, Rect) {
    let action_size = measured_action_size(ctx, "Advance 10%", width, 118.0, 34.0, density);
    let natural_percentage = measured_body_size(ctx, percentage, width);
    let action = Rect::new(x, y + 54.0, action_size.width, action_size.height);
    let inline = width >= action_size.width + 14.0 + natural_percentage.width;
    let percentage_x = if inline { action.max_x() + 14.0 } else { x };
    let percentage_y = if inline {
        action.min_y()
    } else {
        action.max_y() + 8.0
    };
    let percentage_width = (x + width - percentage_x).max(0.0);
    let percentage_size = measured_body_size(ctx, percentage, percentage_width);
    let percentage_height = percentage_size.height.max(if inline { 34.0 } else { 0.0 });
    (
        action,
        Rect::new(
            percentage_x,
            percentage_y,
            percentage_width,
            percentage_height,
        ),
    )
}

fn component_id(renderer: &str, id: usize) -> bool {
    match renderer {
        "tooltip" | "popover" | "banner" | "empty-state" => (20..=21).contains(&id),
        "toast" => (20..=21).contains(&id),
        "progress" => id == 20,
        _ => false,
    }
}

pub(super) fn accepts_key(renderer: &str, focus: Option<usize>, key: ControlKey) -> bool {
    match renderer {
        // Escape is also valid while the tooltip is only hovered; TooltipView
        // decides whether that current transient state can consume it.
        "tooltip" => key == ControlKey::Escape,
        "popover" => focus.is_some_and(|id| (20..=21).contains(&id)) || key == ControlKey::Escape,
        "toast" => focus == Some(20),
        "banner" | "empty-state" => focus == Some(20),
        _ => false,
    }
}

fn overlay_result(
    event: OverlayEvent,
    ext: &mut ExtendedStory,
    log: &mut impl FnMut(String),
) -> InputResult {
    match event {
        OverlayEvent::None => InputResult {
            consumed: false,
            changed: false,
        },
        OverlayEvent::Dismissed { restore_focus } => {
            ext.focused = restore_focus;
            log("popover: dismissed; focus restored".into());
            InputResult {
                consumed: true,
                changed: true,
            }
        }
    }
}

pub(super) fn activate(
    ext: &mut ExtendedStory,
    renderer: &str,
    id: usize,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    let changed = match (renderer, id) {
        // Tooltip visibility is owned by TooltipView's hover/focus state.
        // Activating the anchor has no persistent model effect.
        ("tooltip", 20) => false,
        ("popover", 20) => {
            if ext.popover.open {
                let event = ext.feedback_views.popover_close(&mut ext.popover);
                let _ = overlay_result(event, ext, log);
            } else {
                ext.feedback_views.popover_show(&mut ext.popover, 20);
                log("popover: open".into());
            }
            true
        }
        ("toast", 21) => {
            if ext.toast.visible {
                let expired = ext.feedback_views.toast_advance(&mut ext.toast, 1000);
                log(if expired {
                    "toast: fixture timer elapsed".into()
                } else {
                    "toast timer: advanced 1 s".into()
                });
            } else {
                ext.toast
                    .show("Changes saved", controls::NoticeTone::Success, 5000);
                log("toast: shown (5 s fixture)".into());
            }
            true
        }
        ("progress", 20) => {
            let next = (ext.progress.value() + 0.1).min(1.0);
            let changed = ext.progress.set(next);
            if changed {
                log(format!("progress: {:.0}%", next * 100.0));
            }
            changed
        }
        _ => return None,
    };
    Some(InputResult {
        consumed: true,
        changed,
    })
}

pub(super) fn key(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    key: ControlKey,
    down: bool,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    if !supports(renderer) || !accepts_key(renderer, focus, key) {
        return None;
    }
    let result = match renderer {
        "tooltip" => {
            if !down {
                return Some(InputResult {
                    consumed: true,
                    changed: false,
                });
            }
            let changed = ext.feedback_views.tooltip_key(key) == TooltipEvent::Dismissed;
            if changed {
                log("tooltip: dismissed".into());
            }
            InputResult {
                // TooltipView handles Escape without changing the fixture model.
                consumed: changed,
                changed,
            }
        }
        "popover" if ext.popover.open => overlay_result(
            ext.feedback_views.popover_key(&mut ext.popover, key, down),
            ext,
            log,
        ),
        "toast" if focus == Some(20) => {
            let changed = ext.feedback_views.toast_key(&mut ext.toast, key, down);
            if changed {
                log("toast: dismissed".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        "banner" => {
            let changed = ext.feedback_views.banner_key(&mut ext.banner, key, down);
            if changed {
                log("banner: dismissed".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        "empty-state" => {
            let changed = ext.feedback_views.empty_state_key(key, down);
            if changed {
                ext.empty_state_created = true;
                log("empty-state action: created Documents fixture item".into());
            }
            InputResult {
                consumed: changed,
                changed,
            }
        }
        _ => return None,
    };
    Some(result)
}

pub(super) fn pointer_move(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
) -> Option<InputResult> {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_pointer_move(point),
        "popover" => ext.feedback_views.popover_pointer_move(point),
        "toast" => ext.feedback_views.toast_pointer_move(&mut ext.toast, point),
        "banner" => ext.feedback_views.banner_pointer_move(point),
        "empty-state" => ext.feedback_views.empty_state_pointer_move(point),
        _ => return None,
    }
    Some(InputResult {
        consumed: true,
        changed: false,
    })
}

pub(super) fn pointer_down(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
) -> Option<InputResult> {
    let consumed = match renderer {
        "popover" if ext.popover.open => ext
            .feedback_views
            .popover_pointer_down(&mut ext.popover, point),
        "toast" => ext.feedback_views.toast_pointer_down(point),
        "banner" => ext.feedback_views.banner_pointer_down(point),
        "empty-state" => ext.feedback_views.empty_state_pointer_down(point),
        "tooltip" | "progress" | "status-badge" => return None,
        _ => return None,
    };
    Some(InputResult {
        consumed,
        changed: false,
    })
}

pub(super) fn pointer_up(
    ext: &mut ExtendedStory,
    renderer: &str,
    point: Point,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    match renderer {
        "popover" if ext.popover.open => Some(overlay_result(
            ext.feedback_views
                .popover_pointer_up(&mut ext.popover, point),
            ext,
            log,
        )),
        "toast" => {
            let changed = ext.feedback_views.toast_pointer_up(&mut ext.toast, point);
            if changed {
                log("toast: dismissed".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "banner" => {
            let changed = ext.feedback_views.banner_pointer_up(&mut ext.banner, point);
            if changed {
                log("banner: dismissed".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "empty-state" => {
            let changed = ext.feedback_views.empty_state_pointer_up(point);
            if changed {
                ext.empty_state_created = true;
                log("empty-state action: created Documents fixture item".into());
            }
            Some(InputResult {
                consumed: changed,
                changed,
            })
        }
        "tooltip" | "progress" | "status-badge" => None,
        _ => None,
    }
}

pub(super) fn focus_changed(
    ext: &mut ExtendedStory,
    renderer: &str,
    focus: Option<usize>,
    focus_visible: bool,
) {
    match renderer {
        "tooltip" => ext
            .feedback_views
            .tooltip_set_focus(focus == Some(20), focus_visible),
        "popover" => ext
            .feedback_views
            .popover_focus_dismiss(focus == Some(21), focus_visible),
        "toast" => ext
            .feedback_views
            .toast_focus_dismiss(focus == Some(20), focus_visible),
        "banner" => ext
            .feedback_views
            .banner_focus_dismiss(focus == Some(20), focus_visible),
        "empty-state" => ext
            .feedback_views
            .empty_state_focus_action(focus == Some(20), focus_visible),
        _ => {}
    }
}

pub(super) fn blur(ext: &mut ExtendedStory, renderer: &str) {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_blur(),
        "popover" => ext.feedback_views.popover_cancel(),
        "toast" => ext.feedback_views.toast_cancel(),
        "banner" => ext.feedback_views.banner_cancel(),
        "empty-state" => ext.feedback_views.empty_state_focus_action(false, false),
        _ => {}
    }
}

pub(super) fn window_active(
    ext: &mut ExtendedStory,
    renderer: &str,
    active: bool,
    log: &mut impl FnMut(String),
) -> Option<InputResult> {
    match renderer {
        "tooltip" => ext.feedback_views.tooltip_set_window_active(active),
        "popover" => {
            let event = ext
                .feedback_views
                .popover_set_window_active(active, &mut ext.popover);
            let result = overlay_result(event, ext, log);
            return Some(result);
        }
        "toast" => ext.feedback_views.toast_set_window_active(active),
        "banner" => ext.feedback_views.banner_set_window_active(active),
        "empty-state" => ext.feedback_views.empty_state_set_window_active(active),
        "status-badge" | "progress" => {}
        _ => return None,
    }
    Some(InputResult {
        consumed: true,
        changed: false,
    })
}

impl FeedbackViews {
    #[allow(clippy::too_many_arguments)]
    pub fn render_tooltip(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &Tooltip,
        anchor: Rect,
        size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> OverlayLayout {
        self.tooltip.render(
            scene, text, model, anchor, size, viewport, clip, palette, role,
        )
    }

    pub fn tooltip_pointer_move(&mut self, point: Point) {
        self.tooltip.pointer_move(point);
    }

    pub fn tooltip_set_focus(&mut self, focused: bool, focus_visible: bool) {
        self.tooltip.set_focus(focused, focus_visible);
    }

    pub fn tooltip_key(&mut self, key: ControlKey) -> TooltipEvent {
        self.tooltip.key(key)
    }

    pub fn tooltip_blur(&mut self) {
        self.tooltip.blur();
    }

    pub fn tooltip_set_window_active(&mut self, active: bool) {
        self.tooltip.set_window_active(active);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_popover_with_content(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &Popover,
        anchor: Rect,
        content_size: Size,
        viewport: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
        draw_content: impl FnOnce(&mut Scene, Rect),
    ) -> OverlayLayout {
        let size = tekes_ui_controls::feedback_view::PopoverView::measure(text, content_size, role);
        self.popover.render_with_content(
            scene,
            text,
            model,
            anchor,
            size,
            viewport,
            clip,
            palette,
            role,
            draw_content,
        )
    }

    pub fn popover_show(&mut self, model: &mut Popover, invoker: usize) {
        self.popover.show(model, invoker);
    }

    pub fn popover_close(&mut self, model: &mut Popover) -> OverlayEvent {
        self.popover.close(model)
    }

    pub fn popover_set_window_active(&mut self, active: bool, model: &mut Popover) -> OverlayEvent {
        self.popover.set_window_active(active, model)
    }

    pub fn popover_cancel(&mut self) {
        self.popover.cancel();
    }

    pub fn popover_pointer_down(&mut self, model: &mut Popover, point: Point) -> bool {
        self.popover.pointer_down(model, point)
    }

    pub fn popover_pointer_move(&mut self, point: Point) {
        self.popover.pointer_move(point);
    }

    pub fn popover_pointer_up(&mut self, model: &mut Popover, point: Point) -> OverlayEvent {
        self.popover.pointer_up(model, point)
    }

    pub fn popover_key(
        &mut self,
        model: &mut Popover,
        key: ControlKey,
        down: bool,
    ) -> OverlayEvent {
        self.popover.key_event(model, key, down)
    }

    pub fn popover_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.popover.focus_dismiss(focused, focus_visible);
    }

    pub fn popover_focus_target(&self) -> Option<Rect> {
        self.popover.focus_target()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_toast(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &Toast,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.toast
            .render(scene, text, model, frame, clip, palette, role)
    }

    pub fn toast_pointer_move(&mut self, model: &mut Toast, point: Point) {
        self.toast.pointer_move(model, point);
    }

    pub fn toast_pointer_down(&mut self, point: Point) -> bool {
        self.toast.pointer_down(point)
    }

    pub fn toast_pointer_up(&mut self, model: &mut Toast, point: Point) -> bool {
        self.toast.pointer_up(model, point)
    }

    pub fn toast_key(&mut self, model: &mut Toast, key: ControlKey, down: bool) -> bool {
        self.toast.key(model, key, down)
    }

    pub fn toast_advance(&mut self, model: &mut Toast, elapsed_ms: u32) -> bool {
        self.toast.advance(model, elapsed_ms)
    }

    pub fn toast_cancel(&mut self) {
        self.toast.cancel();
    }

    pub fn toast_set_window_active(&mut self, active: bool) {
        self.toast.set_window_active(active);
    }

    pub fn toast_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.toast.focus_dismiss(focused, focus_visible);
    }

    pub fn toast_focus_target(&self) -> Option<Rect> {
        self.toast.focus_target()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_banner(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &Banner,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.banner
            .render(scene, text, model, frame, clip, palette, role)
    }

    pub fn banner_pointer_move(&mut self, point: Point) {
        self.banner.pointer_move(point);
    }

    pub fn banner_pointer_down(&mut self, point: Point) -> bool {
        self.banner.pointer_down(point)
    }

    pub fn banner_pointer_up(&mut self, model: &mut Banner, point: Point) -> bool {
        self.banner.pointer_up(model, point)
    }

    pub fn banner_key(&mut self, model: &mut Banner, key: ControlKey, down: bool) -> bool {
        self.banner.key(model, key, down)
    }

    pub fn banner_cancel(&mut self) {
        self.banner.cancel();
    }

    pub fn banner_set_window_active(&mut self, active: bool) {
        self.banner.set_window_active(active);
    }

    pub fn banner_focus_dismiss(&mut self, focused: bool, focus_visible: bool) {
        self.banner.focus_dismiss(focused, focus_visible);
    }

    pub fn banner_focus_target(&self) -> Option<Rect> {
        self.banner.focus_target()
    }

    pub fn render_status_badge(
        &mut self,
        scene: &mut Scene,
        model: StatusBadge,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        self.status_badge.render(scene, model, frame, clip, palette)
    }

    pub fn render_progress(
        &mut self,
        scene: &mut Scene,
        model: Progress,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ViewLayout {
        self.progress.render(scene, model, frame, clip, palette)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_empty_state(
        &mut self,
        scene: &mut Scene,
        text: &mut impl tekes_ui_controls::ButtonTextRenderer,
        model: &EmptyState,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        role: proposal::TextRole,
    ) -> ViewLayout {
        self.empty_state
            .render(scene, text, model, frame, clip, palette, role)
    }

    pub fn empty_state_pointer_down(&mut self, point: Point) -> bool {
        self.empty_state.pointer_down(point)
    }

    pub fn empty_state_pointer_move(&mut self, point: Point) {
        self.empty_state.pointer_move(point);
    }

    pub fn empty_state_set_disabled(&mut self, disabled: bool) {
        self.empty_state.set_disabled(disabled);
    }

    pub fn empty_state_set_window_active(&mut self, active: bool) {
        self.empty_state.set_window_active(active);
    }

    pub fn empty_state_focus_action(&mut self, focused: bool, focus_visible: bool) {
        self.empty_state.focus_action(focused, focus_visible);
    }

    pub fn empty_state_focus_target(&self) -> Option<Rect> {
        self.empty_state.focus_target()
    }

    pub fn empty_state_pointer_up(&mut self, point: Point) -> bool {
        self.empty_state.pointer_up(point)
    }

    pub fn empty_state_key(&mut self, key: ControlKey, down: bool) -> bool {
        self.empty_state.key(key, down)
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::gallery::{static_text::StaticText, text::TextEngine};

    fn context<'a>(text: &'a mut TextEngine, static_text: &'a mut StaticText) -> PageCtx<'a> {
        PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text,
            static_text,
        }
    }

    fn luminance(color: tekes_ui_tokens::Rgba) -> f32 {
        let linear = |channel: f32| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    fn contrast_ratio(a: tekes_ui_tokens::Rgba, b: tekes_ui_tokens::Rgba) -> f32 {
        let (lighter, darker) = if luminance(a) >= luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn status_badge_glyph_ink_contrasts_with_all_status_fills_across_themes_and_brands() {
        use tekes_ui_controls::extended::StatusBadge;
        use tekes_ui_scene::Primitive;
        use tekes_ui_tokens::{Palette, Theme};

        let cases = [
            (StatusBadge::Info, "Info", "status-info"),
            (StatusBadge::Running, "Running", "status-running"),
            (StatusBadge::Succeeded, "Succeeded", "status-succeeded"),
            (StatusBadge::Failed, "Failed", "status-failed"),
        ];
        let frame = Rect::new(0., 0., 112., 30.);
        for theme in Theme::ALL {
            for brand in 0..=2 {
                let palette = Palette { theme, brand };
                for (status, label, fill_role) in cases {
                    let mut scene = Scene::new();
                    let mut badge = StatusBadgeView::default();
                    let mut text = TextEngine::new();
                    badge.render(&mut scene, status, frame, None, palette);
                    draw_status_badge_label(&mut text, &mut scene, theme, frame, label, palette);

                    let fill = scene
                        .primitives()
                        .iter()
                        .find_map(|primitive| match primitive {
                            Primitive::Quad(quad) if quad.rect == frame => Some(quad.fill),
                            _ => None,
                        })
                        .expect("production status badge fill quad");
                    let glyph_ink = scene
                        .primitives()
                        .iter()
                        .find_map(|primitive| match primitive {
                            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run.color),
                            _ => None,
                        })
                        .expect("actual shaped badge-label glyph run");
                    assert_eq!(glyph_ink, StatusBadgeView::label_color(palette));
                    assert_eq!(fill, palette.color(fill_role).unwrap());
                    assert!(
                        contrast_ratio(glyph_ink, fill) >= 4.5,
                        "{theme:?} brand={brand} {status:?}: {:?} on {:?} = {}",
                        glyph_ink,
                        fill,
                        contrast_ratio(glyph_ink, fill)
                    );
                }
            }
        }
    }

    #[test]
    fn toast_and_progress_fixture_controls_follow_measured_large_text() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut static_text = StaticText::new(None);
        let mut ctx = context(&mut text, &mut static_text);
        let toast_message = "A toast message wrapping in a narrow preview viewport.";
        let toast_size =
            ToastView::measure(ctx.text, toast_message, 180.0, proposal::TextRole::UiBody);
        assert!(toast_size.height > 62.0);
        let toast_model = Toast::new(toast_message, controls::NoticeTone::Info, 5000);
        let toast_frame = Rect::new(0.0, 0.0, 180.0, toast_size.height);
        let mut toast_view = ToastView::default();
        let mut scene = Scene::new();
        let toast_layout = toast_view.render(
            &mut scene,
            ctx.text,
            &toast_model,
            toast_frame,
            Some(toast_frame),
            Palette::from(ctx.theme),
            proposal::TextRole::UiBody,
        );
        let dismiss_target = toast_view.focus_target().expect("visible dismiss button");
        assert_eq!(
            toast_layout.content.size.height,
            ctx.text
                .measure(
                    toast_message,
                    proposal::TextRole::UiBody,
                    toast_layout.content.size.width,
                )
                .height
        );
        assert!(dismiss_target.max_y() <= toast_frame.max_y());
        assert!(toast_layout.content.max_y() <= toast_frame.max_y());
        let toast_bottom = 12.0 + toast_size.height;
        let (action, timer, note) = toast_fixture_layout(
            &mut ctx,
            0.0,
            0.0,
            180.0,
            toast_bottom,
            "12000 ms remaining",
            "Time advances only when the fixture timer control is activated.",
            0,
        );
        assert!(action.min_y() >= toast_bottom + 8.0);
        assert!(timer.min_y() >= action.max_y() + 8.0);
        assert!(note.min_y() >= timer.max_y() + 8.0);
        for rect in [action, timer, note] {
            assert!(rect.min_x() >= 0.0 && rect.max_x() <= 180.0, "{rect:?}");
            assert!(rect.size.height > 0.0, "{rect:?}");
        }

        let (button, percentage) = progress_fixture_layout(&mut ctx, 0.0, 0.0, 180.0, "70%", 0);
        assert!(percentage.min_y() >= button.max_y() + 8.0);
        assert!(percentage.max_x() <= 180.0);
        assert!(
            percentage.size.height
                >= ctx
                    .text
                    .measure("70%", proposal::TextRole::UiBody, percentage.size.width)
                    .height
        );
    }

    #[test]
    fn default_size_fixture_rows_keep_their_existing_positions() {
        let mut text = TextEngine::new();
        let mut static_text = StaticText::new(None);
        let mut ctx = context(&mut text, &mut static_text);
        let toast_size =
            ToastView::measure(ctx.text, "Changes saved", 520.0, proposal::TextRole::UiBody);
        assert_eq!(toast_size.height, 62.0);
        let (action, timer, note) = toast_fixture_layout(
            &mut ctx,
            40.0,
            0.0,
            600.0,
            74.0,
            "5000 ms remaining",
            "Time advances only when the fixture timer control is activated.",
            0,
        );
        assert_eq!(action, Rect::new(40.0, 84.0, 156.0, 34.0));
        assert_eq!(timer.min_x(), 206.0);
        assert_eq!(timer.min_y(), 84.0);
        assert_eq!(note.min_y(), 126.0);
        let (button, percentage) = progress_fixture_layout(&mut ctx, 40.0, 0.0, 600.0, "70%", 0);
        assert_eq!(button, Rect::new(40.0, 54.0, 118.0, 34.0));
        assert_eq!(percentage.min_x(), 172.0);
        assert_eq!(percentage.min_y(), 54.0);
        assert_eq!(percentage.size.height, 34.0);
    }
}
