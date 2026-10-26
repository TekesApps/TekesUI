//! Per-host routing/timing for help on the original Content owner. TooltipView
//! retains presentation, dismissal and placement; no action/editor is wrapped.
use super::*;
use crate::{
    extended::Tooltip,
    feedback_view::{OverlayLayout, TooltipView},
};
use std::time::Duration;

impl Content {
    /// Declare help on this original semantic/action owner, without another row
    /// or AX node. Empty text clears the declaration. Unmount clears it too;
    /// a retained object must explicitly rebind after removal/reinsertion.
    pub fn with_help(mut self, text: impl Into<String>) -> Self {
        self.set_help(Some(text.into()));
        self
    }
    /// Update or revoke this mounted instance's help. Equal text is a no-op;
    /// changes invalidate any previous tooltip request without touching input.
    pub fn set_help(&mut self, text: Option<String>) {
        let text = text.filter(|s| !s.trim().is_empty());
        if self.help != text {
            self.help = text;
            self.help_revision = self
                .help_revision
                .checked_add(1)
                .expect("help revision exhausted");
        }
    }
    pub fn help_text(&self) -> Option<&str> {
        self.help
            .as_deref()
            .or_else(|| self.component.help().map(|(text, _)| text))
    }
    pub(super) fn help_semantics(&self) -> ContentSemantics {
        let mut s = self.component.semantics();
        if let Some(help) = self.help_text() {
            s.description = Some(match s.description {
                Some(existing) if !existing.is_empty() && existing != help => {
                    format!("{existing}\n{help}")
                }
                _ => help.to_owned(),
            });
        }
        s
    }
    fn help_frame(&self) -> Option<Rect> {
        self.component
            .semantic_frame(self.frame)
            .intersection(&self.clip)
    }
    fn help_details(&self, part: Option<usize>) -> Option<(&str, (u64, u64), Rect)> {
        if let Some(part) = part {
            let (text, revision) = self.component.semantic_part_help(part)?;
            let frame = self
                .component
                .semantic_parts()
                .into_iter()
                .find(|p| p.index == part)?
                .frame
                .intersection(&self.clip)?;
            if text.trim().is_empty() || frame.is_empty() {
                return None;
            }
            Some((text, (0, revision), frame))
        } else {
            Some((
                self.help_text()?,
                (
                    self.help_revision,
                    if self.help.is_some() {
                        0
                    } else {
                        self.component.help().map_or(0, |(_, revision)| revision)
                    },
                ),
                self.help_frame()?,
            ))
        }
    }
    fn help_at(&self, p: Point) -> Option<&Content> {
        if !self.component.active() {
            return None;
        }
        if !self.component.semantics().role.is_action_owner() {
            for c in self.component.children().iter().rev() {
                if let Some(hit) = c.help_at(p) {
                    return Some(hit);
                }
            }
        }
        (self.visible().is_some_and(|f| f.contains(p))
            && (self.help_text().is_some()
                || self.component.semantic_parts().iter().any(|part| {
                    part.frame.contains(p)
                        && self.component.semantic_part_help(part.index).is_some()
                })
                || self.component.interactive()
                || self.component.semantics().disabled))
            .then_some(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Request {
    id: ComponentId,
    mount: u64,
    part: Option<usize>,
    revision: (u64,u64),
    keyboard: bool,
    anchor: Option<Rect>,
}
impl Request {
    fn same_owner(self, other: Self) -> bool {
        self.id == other.id
            && self.mount == other.mount
            && self.part == other.part
            && self.revision == other.revision
            && self.keyboard == other.keyboard
    }
}
#[derive(Debug)]
pub(super) struct HelpState {
    request: Option<Request>,
    since: Option<Duration>,
    last_now: Option<Duration>,
    ready: bool,
    blocked: bool,
    view: TooltipView,
    label: label::LabelState,
}
impl Default for HelpState {
    fn default() -> Self {
        Self {
            request: None,
            since: None,
            last_now: None,
            ready: false,
            blocked: false,
            view: Default::default(),
            label: label::LabelState::new(tekes_ui_text::LabelOptions::default()),
        }
    }
}
impl HelpState {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn suppress(&mut self) {
        self.blocked = true;
        self.ready = false;
        self.since = None;
        self.view.set_window_active(false);
    }
    fn arm(&mut self, request: Option<Request>) {
        if self
            .request
            .zip(request)
            .is_some_and(|(a, b)| a.same_owner(b))
        {
            return;
        }
        self.clear();
        self.request = request;
    }
    pub(super) fn needs_frame(&self) -> bool {
        self.request.is_some() && self.since.is_some() && !self.ready && !self.blocked
    }
}
impl ContentHost {
    /// Hosts with separate content/toolbar paint passes can defer only help,
    /// then paint it once after ordinary content. The original owner/state and
    /// clock remain unchanged; menus/capture/modal barriers still apply.
    pub fn set_help_overlay_deferred(&mut self, deferred: bool) {
        if self.help_overlay_deferred != deferred {
            self.help_overlay_deferred = deferred;
            self.dirty = true;
        }
    }
    /// Paint/tick deferred help using the caller's real window viewport. Call
    /// after rendering this ContentHost, without replaying its content or input.
    /// This is a no-op unless deferred mode is enabled and layout is current.
    pub fn paint_help_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        viewport: Rect,
        palette: Palette,
    ) {
        if self.help_overlay_deferred && self.window_active && !self.reading_transaction
            && !self.reveal_clock.pending && self.last_layout.is_some()
        {
            self.render_help(scene, text, viewport, viewport, palette);
        }
    }
    fn help_request(
        &self,
        id: ComponentId,
        keyboard: bool,
        point: Option<Point>,
    ) -> Option<Request> {
        let n = self.scope().find_active(id)?;
        let part = n
            .component
            .semantic_parts()
            .into_iter()
            .rev()
            .find(|part| {
                (if keyboard {
                    part.focused
                } else {
                    point.is_some_and(|p| part.frame.contains(p))
                }) && n.component.semantic_part_help(part.index).is_some()
            })
            .map(|p| p.index);
        self.help_request_for(id, part, keyboard)
    }
    fn help_request_for(
        &self,
        id: ComponentId,
        part: Option<usize>,
        keyboard: bool,
    ) -> Option<Request> {
        let n = self.scope().find_active(id)?;
        let (_, revision, frame) = n.help_details(part)?;
        Some(Request {
            id,
            mount: n.mount,
            part,
            revision,
            keyboard,
            anchor: if keyboard { None } else { Some(frame) },
        })
    }
    fn help_available(&self) -> bool {
        self.captured.is_none()
            && self.scope().overlay_id().is_none()
            && (self.presented_dialog.is_none() || self.modal.is_some())
    }
    pub(super) fn help_focus(&mut self, id: Option<ComponentId>, visible: bool) {
        if visible && self.help_available() {
            let r = id.and_then(|id| self.help_request(id, true, None));
            self.help.arm(r);
        } else {
            self.help.suppress();
        }
    }
    fn help_owner(&self) -> Option<&Content> {
        if !self.help_available() {
            return None;
        }
        let r = self.help.request?;
        let n = self.scope().find_active(r.id)?;
        let (_, revision, _) = n.help_details(r.part)?;
        (n.mount == r.mount
            && revision == r.revision
            && (!r.keyboard
                || (self.focused == Some(r.id)
                    && r.part.is_none_or(|key| {
                        n.component
                            .semantic_parts()
                            .iter()
                            .any(|p| p.index == key && p.focused)
                    }))))
        .then_some(n)
    }
    /// Currently painted help from the same original owner; full source text,
    /// not a new semantic/action node. Revocation, mount/scope/clip changes,
    /// capture and overlay priority invalidate this view immediately.
    pub fn help_tooltip(&self) -> Option<(ComponentId, &str, OverlayLayout)> {
        if self.help.blocked || !self.help.ready {
            return None;
        }
        let n = self.help_owner()?;
        let (text, _, frame) = n.help_details(self.help.request?.part)?;
        if self.help.request?.anchor != Some(frame) {
            return None;
        }
        let layout = self.help.view.layout();
        layout.visible?;
        Some((n.id, text, layout))
    }
    pub(super) fn help_event(&mut self, event: &ContentEvent) -> bool {
        let routing = event.routing_event();
        match routing.as_ref() {
            ContentEvent::PointerMove(p) => {
                if event.is_primary_pointer()
                    && let Some((_, _, layout)) = self.help_tooltip()
                {
                    self.help.view.pointer_move(*p);
                    if layout.hit_test(*p) {
                        return false;
                    }
                }
                let r = if event.is_primary_pointer() && self.help_available() {
                    self.scope()
                        .help_at(*p)
                        .filter(|n| {
                            n.help_frame().is_some_and(|frame| frame.contains(*p))
                                || n.component.semantic_parts().iter().any(|part| {
                                    n.help_details(Some(part.index))
                                        .is_some_and(|(_, _, frame)| frame.contains(*p))
                                })
                        })
                        .and_then(|n| self.help_request(n.id, false, Some(*p)))
                } else {
                    None
                };
                self.help.arm(r);
            }
            ContentEvent::Scroll { .. } => self.help.clear(),
            ContentEvent::PointerDown(_) | ContentEvent::PointerUp(_) => self.help.suppress(),
            ContentEvent::Key { key, down: true } => {
                let visible = self.help_tooltip().is_some();
                let composing = self
                    .focused_text()
                    .is_some_and(|s| s.input.marked.is_some())
                    || self
                        .focused
                        .and_then(|id| self.root.find(id))
                        .is_some_and(|n| n.component.composing());
                let dismissed = if *key == ControlKey::Escape {
                    self.help.view.key(*key) == crate::feedback_view::TooltipEvent::Dismissed
                } else {
                    false
                };
                self.help.suppress();
                return *key == ControlKey::Escape && visible && dismissed && !composing;
            }
            ContentEvent::KeyInput(k) if k.down => self.help.suppress(),
            ContentEvent::Commit { .. }
            | ContentEvent::Preedit { .. }
            | ContentEvent::TextCommand(_) => self.help.suppress(),
            _ => {}
        }
        false
    }
    pub(super) fn render_help(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        clip: Rect,
        palette: Palette,
    ) {
        let Some(r) = self.help.request else {
            return;
        };
        // A new declaration starts a new dwell; rendering alone cannot rearm
        // cancelled owners or changed geometry.
        let n = self.scope().find_active(r.id);
        let Some(n) = n.filter(|n| n.mount == r.mount && n.help_details(r.part).is_some()) else {
            self.help.clear();
            return;
        };
        if n.help_details(r.part).unwrap().1 != r.revision {
            // Updating explanation is not a new hover/focus request. Keep an
            // explicit click/Escape/geometry dismissal across metadata changes.
            let blocked = self.help.blocked;
            let new = self.help_request_for(r.id, r.part, r.keyboard);
            self.help.arm(new);
            if blocked {
                self.help.suppress();
            }
        }
        let Some(n) = self.help_owner() else {
            self.help.suppress();
            return;
        };
        let (help, _, anchor) = n.help_details(r.part).unwrap();
        let model = Tooltip {
            visible: true,
            text: help.to_owned(),
        };
        if self
            .help
            .request
            .is_some_and(|r| r.anchor.is_some_and(|old| old != anchor))
        {
            self.help.suppress();
            return;
        }
        self.help.request.as_mut().unwrap().anchor = Some(anchor);
        if self.help.blocked {
            return;
        }
        let Some(context) = text.motion_context() else {
            self.help.clear();
            return;
        };
        let Some(delay) = proposal::tooltip_hover_delay() else {
            return;
        };
        if self.help.last_now.is_some_and(|t| context.now < t) {
            self.help.since = None;
            self.help.ready = false;
            self.help.view = Default::default();
        }
        self.help.last_now = Some(context.now);
        let start = *self.help.since.get_or_insert(context.now);
        self.help.ready = context.now.saturating_sub(start).as_secs_f32() >= delay;
        if !self.help.ready {
            text.request_animation_frame();
            return;
        }
        // The caller's clip is the overlay viewport, not the anchor's layout
        // row. A toolbar may be one row inside a whole-window clip.
        let viewport = clip;
        if viewport.is_empty() {
            self.help.suppress();
            return;
        }
        let role = proposal::TextRole::UiCaption;
        self.help.label.reset();
        let mut painted = false;
        let content = TooltipView::content_size(viewport);
        let shaped = text.text_surface_host().and_then(|host| {
            host.shape_label(
                &model.text,
                role,
                content.width,
                tekes_ui_text::LabelOptions::default(),
            )
        });
        if let Some(layout) = shaped {
            // Use actual shaped row ends, including fallback fonts and host
            // minimum line boxes; a nominal M metric can crop the first line.
            let lines = TooltipView::fitting_lines(
                viewport,
                layout
                    .paragraph()
                    .lines
                    .iter()
                    .map(|line| line.top + line.height),
            );
            if lines == 0 {
                self.help.suppress();
                return;
            }
            self.help.label = label::LabelState::new(tekes_ui_text::LabelOptions {
                max_lines: std::num::NonZeroUsize::new(lines),
                overflow: tekes_ui_text::LabelOverflow::TailEllipsis,
            });
            let mut renderer = label::LabelRenderer::new(text, &mut self.help.label, viewport);
            let size = TooltipView::fitted_size(&mut renderer, &model, viewport, role);
            if !renderer.failed() {
                self.help.view.render(
                    scene,
                    &mut renderer,
                    &model,
                    anchor,
                    size,
                    viewport,
                    Some(viewport),
                    palette,
                    role,
                );
                painted = true;
            }
        }
        if !painted {
            // Existing label-only renderers stay compatible. Without readonly
            // overflow support, show only text that actually fits; retain full
            // AX help rather than centering/clipping an oversized paragraph.
            let measured = text.measure(&model.text, role, content.width);
            if !measured.height.is_finite()
                || measured.height > content.height
                || content.width <= 0.
                || content.height <= 0.
            {
                self.help.suppress();
                return;
            }
            let size = TooltipView::fitted_size(text, &model, viewport, role);
            let mut renderer = text;
            self.help.view.render(
                scene,
                &mut renderer,
                &model,
                anchor,
                size,
                viewport,
                Some(viewport),
                palette,
                role,
            );
        }
    }
}
