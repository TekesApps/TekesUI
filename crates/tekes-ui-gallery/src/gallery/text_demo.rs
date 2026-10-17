//! Live Text and TextEditor surfaces share shaping, selection, scrolling and IME data.
use super::{PageCtx, text::TextEngine};
use tekes_ui_controls::TextInput;
use tekes_ui_platform_macos::{
    Event, KeyEventKind, MouseButton, MouseEventKind, ScrollPhase, TextInputState,
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{Spacing, Stroke, Theme, proposal};

fn ink(id: &str, theme: Theme) -> tekes_ui_tokens::Rgba {
    proposal::render_color(id, theme).expect("JSON text role")
}
fn metric(group: &str, id: &str) -> f32 {
    proposal::metric(group, id).expect("JSON metric")
}
#[derive(Debug)]
struct Surface {
    view: tekes_ui_controls::TextSurface,
    prepared: Option<tekes_ui_text::coretext::PreparedText>,
    prepared_role: Option<proposal::TextRole>,
    fixture: bool,
}
impl std::ops::Deref for Surface {
    type Target = tekes_ui_controls::TextSurface;
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}
impl std::ops::DerefMut for Surface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
    }
}
impl Surface {
    fn new(text: &str, multiline: bool, read_only: bool) -> Self {
        Self {
            view: tekes_ui_controls::TextSurface::new(text, multiline, read_only),
            prepared: None,
            prepared_role: None,
            fixture: false,
        }
    }

    fn layout(&mut self, text: &mut TextEngine, rect: Rect) {
        self.layout_clipped(text, rect, None);
    }
    fn layout_clipped(&mut self, text: &mut TextEngine, rect: Rect, ancestor_clip: Option<Rect>) {
        let role = self.view.role;
        let fixture = self.fixture;
        let prepared = &mut self.prepared;
        let prepared_role = &mut self.prepared_role;
        let text_changed = self.view.shaped != self.view.input.text;
        let scale_changed = self.view.type_scale != text.type_scale();
        let role_changed = *prepared_role != Some(role);
        if text_changed || scale_changed || role_changed {
            *prepared = None;
        }
        self.view.layout_with(
            rect,
            ancestor_clip,
            text.scale(),
            text.type_scale(),
            |source, width| {
                if fixture {
                    text.proposal_document(source, role, width)
                } else {
                    if prepared.is_none() {
                        *prepared = Some(text.prepare_proposal(source, role));
                    }
                    std::rc::Rc::new(text.reflow_proposal(prepared.as_ref().unwrap(), role, width))
                }
            },
        );
        *prepared_role = Some(role);
    }

    fn layout_preserving_caret_clipped(
        &mut self,
        text: &mut TextEngine,
        rect: Rect,
        ancestor_clip: Option<Rect>,
        focused: bool,
    ) {
        let caret = self.caret();
        let keep_visible = focused
            && !self.input.read_only
            && self.input.selection().is_empty()
            && self.paragraph.is_some()
            && self.inner.size != rect.inset(Spacing::Regular.value()).size
            && caret.min_x() >= self.inner.min_x()
            && caret.max_x() <= self.inner.max_x()
            && caret.min_y() >= self.inner.min_y()
            && caret.max_y() <= self.inner.max_y();
        self.layout_clipped(text, rect, ancestor_clip);
        if keep_visible {
            self.reveal();
        }
    }

    fn draw(
        &self,
        text: &mut TextEngine,
        scene: &mut Scene,
        theme: Theme,
        focused: bool,
        caret_on: bool,
    ) {
        self.view.draw(text, scene, theme, focused, caret_on);
    }
}
#[derive(Debug)]
pub struct TextDemo {
    surfaces: Vec<Surface>,
    focus: Option<usize>,
    window_active: bool,
    drag: Option<usize>,
    scroll_target: Option<usize>,
    caret_on: bool,
    reveal_pending: bool,
}
impl Default for TextDemo {
    fn default() -> Self {
        let document = include_str!("../../../../fixtures/text/gallery-reading-en.txt").trim_end();
        Self {
            surfaces: vec![
                Surface::new(document, true, true),
                Surface::new("Example project name", false, false),
                Surface::new(document, true, false),
            ],
            focus: None,
            window_active: true,
            drag: None,
            scroll_target: None,
            caret_on: true,
            reveal_pending: false,
        }
    }
}
impl TextDemo {
    /// Return the un-clipped outer frame for a component-owned surface.
    /// Hosts use this for focus reveal; visible fragments are not a substitute
    /// for the control's full geometry.
    pub fn frame_at(&self, index: usize) -> Option<Rect> {
        self.surfaces.get(index).map(|surface| surface.outer)
    }

    pub fn set_locale(&mut self, locale: super::locale::Locale) {
        let zh = include_str!("../../../../fixtures/text/gallery-reading.txt").trim_end();
        let en = include_str!("../../../../fixtures/text/gallery-reading-en.txt").trim_end();
        for surface in &mut self.surfaces {
            if surface.input.marked.is_some() {
                continue;
            }
            let current = surface.input.text.as_str();
            let next = if current == zh || current == en {
                Some(if locale == super::locale::Locale::En {
                    en
                } else {
                    zh
                })
            } else if current == "示例项目名称" || current == "Example project name" {
                Some(if locale == super::locale::Locale::En {
                    "Example project name"
                } else {
                    "示例项目名称"
                })
            } else {
                None
            };
            if let Some(next) = next.filter(|next| *next != current) {
                let read_only = surface.input.read_only;
                surface.input = TextInput::new(next, surface.input.multiline);
                surface.input.read_only = read_only;
                surface.cancel_interaction();
                let offset = surface.scroll.offset;
                surface.scroll.scroll_by(-offset.x, -offset.y);
                surface.reflow_anchor = None;
            }
        }
    }
    pub fn text_only() -> Self {
        let mut demo = Self::default();
        demo.surfaces.truncate(1);
        demo
    }
    pub fn editors_only() -> Self {
        let mut demo = Self::default();
        demo.surfaces.remove(0);
        demo
    }
    pub fn scroll_only() -> Self {
        let mut demo = Self::default();
        let mut surface = demo.surfaces.pop().unwrap();
        surface.input.read_only = true;
        // This reading fixture ends at its last row, without an extra blank
        // editable line that would look like asymmetric bottom padding.
        surface.input.text = surface.input.text.trim_end_matches('\n').to_owned();
        demo.surfaces = vec![surface];
        demo
    }
    fn sections(
        &self,
        content: Rect,
        type_scale: f32,
        heading_heights: &[f32],
    ) -> Vec<(&'static str, Rect, Rect)> {
        let gap = Spacing::Loose.value();
        let type_scale = if type_scale.is_finite() {
            type_scale.max(1.0)
        } else {
            1.0
        };
        let label_h = proposal::TextRole::UiHeading.style().line_height * type_scale;
        let line_h = proposal::TextRole::ComposerBody.style().line_height * type_scale;
        let mut y = content.min_y();
        let bottom = content.max_y();
        let mut out = Vec::new();
        for (i, surface) in self.surfaces.iter().enumerate() {
            let name = if surface.input.read_only {
                "Text · 选择与复制"
            } else if surface.input.multiline {
                "TextEditor · 多行"
            } else {
                "TextEditor · 单行"
            };
            let measured_label_h = heading_heights
                .get(i)
                .copied()
                .unwrap_or(label_h)
                .max(label_h)
                .min((bottom - y).max(0.0));
            let label = Rect::new(content.min_x(), y, content.size.width, measured_label_h);
            y += label.size.height;
            y += Spacing::Regular.value().min((bottom - y).max(0.0));
            let desired_height = if self.surfaces.len() <= 2
                && i + 1 == self.surfaces.len()
                && surface.input.multiline
            {
                (bottom - y).max(0.0)
            } else if surface.input.read_only {
                line_h * 8.0
            } else if surface.input.multiline {
                line_h * 20.0
            } else {
                surface.minimum_outer_height(type_scale)
            };
            let height = desired_height.min((bottom - y).max(0.0));
            let panel = Rect::new(content.min_x(), y, content.size.width, height);
            out.push((name, label, panel));
            y += height;
            y += gap.min((bottom - y).max(0.0));
        }
        out
    }
    pub fn build(&mut self, ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
        self.build_clipped(ctx, content, None, scene);
    }
    pub fn build_clipped(
        &mut self,
        ctx: &mut PageCtx<'_>,
        content: Rect,
        ancestor_clip: Option<Rect>,
        scene: &mut Scene,
    ) {
        let clip = ancestor_clip.map_or((!content.is_empty()).then_some(content), |clip| {
            content.intersection(&clip)
        });
        if let Some(clip) = clip {
            scene.push_clip(clip);
        }
        let heading_heights: Vec<f32> = self
            .surfaces
            .iter()
            .map(|surface| {
                let name = if surface.input.read_only {
                    "Text · 选择与复制"
                } else if surface.input.multiline {
                    "TextEditor · 多行"
                } else {
                    "TextEditor · 单行"
                };
                let paragraph = ctx
                    .text
                    .proposal_label(name, proposal::TextRole::UiHeading, None);
                paragraph.height as f32
            })
            .collect();
        for (i, (name, label, panel)) in self
            .sections(content, ctx.text.type_scale(), &heading_heights)
            .into_iter()
            .enumerate()
        {
            if clip.is_some() {
                ctx.text.proposal_label_in(
                    scene,
                    ctx.theme,
                    label,
                    name,
                    proposal::TextRole::UiHeading,
                    ink("primary-text", ctx.theme),
                );
            }
            self.surfaces[i].layout_preserving_caret_clipped(
                ctx.text,
                panel,
                ancestor_clip,
                self.window_active && self.focus == Some(i),
            );
            self.surfaces[i].draw(
                ctx.text,
                scene,
                ctx.theme,
                self.window_active && self.focus == Some(i),
                self.caret_on,
            );
        }
        if clip.is_some() {
            scene.pop_clip();
        }
    }
    pub fn scroll_metrics(&self) -> (f32, f32) {
        let s = self.surfaces.last().unwrap();
        (s.scroll.limit().y, s.inner.size.height)
    }
    pub fn scroll_to(&mut self, offset: f32) {
        let s = self.surfaces.last_mut().unwrap();
        let current = s.scroll.offset.y;
        s.scroll.scroll_by(0.0, offset - current);
    }
    /// A single-line search field using the same selection, clipboard and IME engine.
    pub fn search() -> Self {
        Self {
            surfaces: vec![Surface::new("", false, false)],
            focus: None,
            window_active: true,
            drag: None,
            scroll_target: None,
            caret_on: true,
            reveal_pending: false,
        }
    }
    /// Single adapter over the production TextInput/ScrollArea models.
    pub fn field(value: &str, multiline: bool, read_only: bool) -> Self {
        let mut field = Self::search();
        field.surfaces[0] = Surface::new(value, multiline, read_only);
        field
    }
    pub fn use_fixture_cache(&mut self) {
        for surface in &mut self.surfaces {
            surface.fixture = true;
        }
    }
    pub fn set_disabled(&mut self, disabled: bool) {
        for s in &mut self.surfaces {
            s.input.disabled = disabled;
        }
        if disabled {
            self.blur();
        }
    }
    pub fn set_error(&mut self, error: bool) {
        for s in &mut self.surfaces {
            s.error = error;
        }
    }
    pub fn focus_field(&mut self) {
        if self.surfaces[0].input.disabled {
            return;
        }
        self.focus = Some(0);
        self.caret_on = true;
    }
    pub fn set_value(&mut self, value: &str) {
        self.surfaces[0].input =
            tekes_ui_controls::TextInput::new(value, self.surfaces[0].input.multiline);
    }
    pub fn search_fixture() -> Self {
        let mut field = Self::search();
        field.surfaces[0].fixture = true;
        field
    }
    pub fn query(&self) -> &str {
        &self.surfaces[0].input.text
    }
    pub fn build_search(&mut self, ctx: &mut PageCtx<'_>, rect: Rect, scene: &mut Scene) {
        self.build_search_clipped(ctx, rect, None, scene);
    }
    pub fn build_search_clipped(
        &mut self,
        ctx: &mut PageCtx<'_>,
        rect: Rect,
        ancestor_clip: Option<Rect>,
        scene: &mut Scene,
    ) {
        self.surfaces[0].layout_preserving_caret_clipped(
            ctx.text,
            rect,
            ancestor_clip,
            self.window_active && self.focus == Some(0),
        );
        self.surfaces[0].draw(
            ctx.text,
            scene,
            ctx.theme,
            self.window_active && self.focus == Some(0),
            self.caret_on,
        );
        if self.query().is_empty() && self.focus.is_none() {
            let Some(clip) = ancestor_clip.map_or(
                (!self.surfaces[0].inner.is_empty()).then_some(self.surfaces[0].inner),
                |clip| self.surfaces[0].inner.intersection(&clip),
            ) else {
                return;
            };
            scene.push_clip(clip);
            ctx.text.proposal_label_in(
                scene,
                ctx.theme,
                self.surfaces[0].inner,
                "按名称搜索…",
                proposal::TextRole::UiBody,
                ink("secondary-text", ctx.theme),
            );
            scene.pop_clip();
        }
    }
    pub fn contains_visible(&self, point: Point) -> bool {
        self.surfaces
            .iter()
            .any(|surface| surface.contains_visible(point))
    }
    pub fn visible_frame(&self) -> Option<Rect> {
        (self.surfaces.len() == 1)
            .then(|| self.surfaces[0].visible_frame())
            .flatten()
    }
    pub(crate) fn visible_frame_at(&self, index: usize) -> Option<Rect> {
        self.surfaces.get(index)?.visible_frame()
    }
    pub fn measure_search_height(&mut self, text: &mut TextEngine) -> f32 {
        let Some(surface) = self.surfaces.first_mut() else {
            return 0.0;
        };
        if surface.input.multiline {
            // Multiline parents own the available viewport. This is a minimum
            // line frame only; preserve the existing width-specific paragraph.
            return surface.minimum_outer_height(text.type_scale());
        }
        let role = surface.role;
        let source = surface.input.text.clone();
        let scale = text.type_scale();
        let role_changed = surface.prepared_role != Some(role);
        if surface.type_scale != scale
            || surface.shaped != source
            || surface.paragraph.is_none()
            || surface.width.is_some()
            || role_changed
        {
            if surface.type_scale != scale || surface.shaped != source || role_changed {
                surface.prepared = None;
            }
            let paragraph = if surface.fixture {
                text.proposal_document(&source, role, None)
            } else {
                if surface.prepared.is_none() {
                    surface.prepared = Some(text.prepare_proposal(&source, role));
                }
                std::rc::Rc::new(text.reflow_proposal(
                    surface.prepared.as_ref().unwrap(),
                    role,
                    None,
                ))
            };
            surface.paragraph = Some(paragraph);
            surface.shaped = source.clone();
            surface.width = None;
            surface.type_scale = scale;
            surface.prepared_role = Some(role);
        }
        let paragraph_height = surface.paragraph.as_ref().map_or(0.0, |p| p.height as f32);
        (paragraph_height + 2.0 * proposal::SCROLL_SINGLE_LINE_PADDING_Y)
            .max(metric("size", "size-button-md"))
    }
    pub fn state(&self) -> Option<TextInputState> {
        if !self.window_active {
            return None;
        }
        let s = self.surfaces.get(self.focus?)?;
        let c = s.caret();
        Some(TextInputState {
            caret_index: s.input.caret,
            text: s.input.text.clone(),
            selection: s.input.selection(),
            marked: s.input.marked.clone(),
            caret_origin: tekes_ui_platform_macos::Point {
                x: f64::from(c.min_x()),
                y: f64::from(c.min_y()),
            },
            caret_size: tekes_ui_platform_macos::Size {
                width: f64::from(c.size.width),
                height: f64::from(c.size.height),
            },
            read_only: s.input.read_only,
            carets: s
                .paragraph
                .as_ref()
                .map(|p| {
                    let origin = s.origin();
                    p.lines
                        .iter()
                        .filter(|line| {
                            origin.y + (line.top + line.height) as f32 >= s.inner.min_y()
                                && origin.y + line.top as f32 <= s.inner.max_y()
                        })
                        .flat_map(|line| {
                            line.carets.iter().map(move |(i, x)| {
                                (
                                    *i,
                                    tekes_ui_platform_macos::Point {
                                        x: f64::from(origin.x) + x,
                                        y: f64::from(origin.y) + line.top,
                                    },
                                    tekes_ui_platform_macos::Size {
                                        width: f64::from(Stroke::Rule.value()),
                                        height: line.height,
                                    },
                                )
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        self.caret_on = true;
        if !active {
            for surface in &mut self.surfaces {
                surface.input.unmark();
                surface.cancel_interaction();
            }
            self.drag = None;
            self.scroll_target = None;
        }
    }
    pub fn blur(&mut self) {
        for s in &mut self.surfaces {
            s.input.unmark();
            s.cancel_interaction();
        }
        self.focus = None;
        self.drag = None;
        self.scroll_target = None;
    }
    pub fn end_scroll_gesture(&mut self) {
        for s in &mut self.surfaces {
            s.scroll.end_gesture();
        }
        self.scroll_target = None;
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        let mut dirty = false;
        for surface in &mut self.surfaces {
            dirty |= surface.scroll.set_reduced_motion(reduced);
        }
        dirty
    }
    pub fn needs_animation(&self) -> bool {
        self.surfaces.iter().any(|s| s.scroll.needs_animation())
    }
    pub fn animate(&mut self, dt: f32) -> bool {
        let mut changed = false;
        for surface in &mut self.surfaces {
            changed |= surface.scroll.tick(dt);
        }
        changed
    }
    pub fn clipboard_selection(&self) -> Option<String> {
        self.focus.map(|i| self.surfaces[i].input.selected_text())
    }
    pub fn paste(&mut self, text: &str) -> bool {
        self.reveal_pending = true;
        let Some(i) = self.focus else {
            return false;
        };
        let changed = self.surfaces[i].input.insert(text, None);
        self.caret_on = true;
        changed
    }
    pub fn cut(&mut self) -> bool {
        self.paste("")
    }
    pub fn handle(&mut self, event: &Event, point: Option<Point>) -> Option<bool> {
        self.reveal_pending = !matches!(
            event,
            Event::CaretBlink
                | Event::Scroll(_)
                | Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                    kind: MouseEventKind::Moved | MouseEventKind::Up { .. },
                    ..
                })
        );
        match event {
            Event::CaretBlink => {
                if self.focus.is_some() {
                    self.caret_on = !self.caret_on;
                    Some(true)
                } else {
                    Some(false)
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count,
                } => {
                    let point = point?;
                    let index = self
                        .surfaces
                        .iter()
                        .position(|s| !s.input.disabled && s.contains_visible(point));
                    let before = self.focus;
                    self.focus = index;
                    self.drag = index;
                    self.caret_on = true;
                    if let Some(i) = index {
                        let s = &mut self.surfaces[i];
                        return s.pointer_down(
                            point,
                            m.modifiers.shift,
                            click_count,
                            metric("size", "size-row-height"),
                        );
                    }
                    Some(before.is_some())
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } => {
                    let i = self.drag?;
                    let p = point?;
                    let s = &mut self.surfaces[i];
                    Some(s.pointer_drag(p, metric("size", "size-row-height")))
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => {
                    self.drag = None;
                    for s in &mut self.surfaces {
                        s.pointer_up();
                    }
                    Some(false)
                }
                _ => None,
            },
            Event::Scroll(e) => {
                let p = point?;
                let hit = self.surfaces.iter().position(|s| s.contains_visible(p));
                if e.phase == ScrollPhase::Began {
                    self.scroll_target = hit;
                }
                let phased = e.precise
                    && (e.phase != ScrollPhase::None || e.momentum_phase != ScrollPhase::None);
                let i = if phased {
                    self.scroll_target.or(hit)?
                } else {
                    hit?
                };
                if phased && self.scroll_target.is_none() {
                    self.scroll_target = Some(i);
                }
                let captured = phased && self.scroll_target == Some(i);
                let s = &mut self.surfaces[i];
                let delta = Point::new(e.delta.x as f32, e.delta.y as f32);
                let begin = e.phase == ScrollPhase::Began;
                let end = matches!(e.phase, ScrollPhase::Ended | ScrollPhase::Cancelled)
                    || matches!(
                        e.momentum_phase,
                        ScrollPhase::Ended | ScrollPhase::Cancelled
                    );
                let momentum = e.momentum_phase != ScrollPhase::None;
                if captured {
                    s.wheel_captured_at(
                        p,
                        delta,
                        e.precise,
                        begin,
                        end,
                        momentum,
                        e.timestamp,
                        e.modifiers.shift,
                    )
                } else {
                    s.wheel_at(
                        p,
                        delta,
                        e.precise,
                        begin,
                        end,
                        momentum,
                        e.timestamp,
                        e.modifiers.shift,
                    )
                }
            }
            Event::TextCommit { text, replacement } => {
                let i = self.focus?;
                self.caret_on = true;
                self.reveal_pending = true;
                Some(self.surfaces[i].input.insert(text, replacement.clone()))
            }
            Event::TextPreedit {
                text,
                clauses,
                selection,
                replacement,
            } => {
                let i = self.focus?;
                self.caret_on = true;
                self.surfaces[i].clauses = clauses.clone();
                Some(
                    self.surfaces[i]
                        .input
                        .preedit(text, selection.clone(), replacement.clone()),
                )
            }
            Event::TextUnmark => {
                let i = self.focus?;
                self.surfaces[i].input.unmark();
                Some(true)
            }
            Event::TextCommand(command) => self.command(command),
            Event::Key(k)
                if k.kind == KeyEventKind::Down && self.focus.is_some() && k.modifiers.command =>
            {
                let command = match k.key_code {
                    0 => "selectAll:",
                    6 => {
                        if k.modifiers.shift {
                            "redo:"
                        } else {
                            "undo:"
                        }
                    }
                    123 => {
                        if k.modifiers.shift {
                            "moveToBeginningOfLineAndModifySelection:"
                        } else {
                            "moveToBeginningOfLine:"
                        }
                    }
                    124 => {
                        if k.modifiers.shift {
                            "moveToEndOfLineAndModifySelection:"
                        } else {
                            "moveToEndOfLine:"
                        }
                    }
                    125 => "moveToEndOfDocument:",
                    126 => "moveToBeginningOfDocument:",
                    _ => return Some(false),
                };
                self.command(command)
            }
            _ => None,
        }
    }
    pub fn command(&mut self, command: &str) -> Option<bool> {
        if matches!(command, "insertTab:" | "insertBacktab:") {
            if let Some(i) = self.focus {
                self.surfaces[i].input.unmark();
            }
            let candidates: Vec<usize> = self
                .surfaces
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.input.read_only)
                .map(|(i, _)| i)
                .collect();
            if candidates.is_empty() {
                self.focus = None;
                return Some(true);
            }
            if candidates.len() == 1 {
                self.focus = if self.focus == Some(candidates[0]) {
                    None
                } else {
                    Some(candidates[0])
                };
            } else {
                let next = self
                    .focus
                    .and_then(|current| candidates.iter().position(|i| *i == current))
                    .map(|i| {
                        if command == "insertBacktab:" {
                            (i + candidates.len() - 1) % candidates.len()
                        } else {
                            (i + 1) % candidates.len()
                        }
                    })
                    .unwrap_or(if command == "insertBacktab:" {
                        candidates.len() - 1
                    } else {
                        0
                    });
                self.focus = Some(candidates[next]);
            }
            self.caret_on = true;
            self.reveal_pending = true;
            return Some(true);
        }
        let i = self.focus?;
        self.caret_on = true;
        self.reveal_pending = true;
        let s = &mut self.surfaces[i];
        let extend = command.contains("ModifySelection");
        match command {
            "selectAll:" => s.input.select_all(),
            "undo:" => {
                s.input.undo();
            }
            "redo:" => {
                s.input.redo();
            }
            "deleteBackward:" => {
                s.input.delete(true, false);
            }
            "deleteForward:" => {
                s.input.delete(false, false);
            }
            "deleteWordBackward:" => {
                s.input.delete(true, true);
            }
            "deleteWordForward:" => {
                s.input.delete(false, true);
            }
            "moveLeft:"
            | "moveLeftAndModifySelection:"
            | "moveBackward:"
            | "moveBackwardAndModifySelection:" => s.input.move_by(-1, extend, false),
            "moveRight:"
            | "moveRightAndModifySelection:"
            | "moveForward:"
            | "moveForwardAndModifySelection:" => s.input.move_by(1, extend, false),
            "moveWordLeft:"
            | "moveWordLeftAndModifySelection:"
            | "moveWordBackward:"
            | "moveWordBackwardAndModifySelection:" => s.input.move_by(-1, extend, true),
            "moveWordRight:"
            | "moveWordRightAndModifySelection:"
            | "moveWordForward:"
            | "moveWordForwardAndModifySelection:" => s.input.move_by(1, extend, true),
            "moveToBeginningOfDocument:" | "moveToBeginningOfDocumentAndModifySelection:" => {
                s.input.select(0, extend)
            }
            "moveToEndOfDocument:" | "moveToEndOfDocumentAndModifySelection:" => {
                let end = s.input.len();
                s.input.select(end, extend)
            }
            "moveToBeginningOfLine:"
            | "moveToBeginningOfLineAndModifySelection:"
            | "moveToEndOfLine:"
            | "moveToEndOfLineAndModifySelection:" => {
                let begin = command.contains("Beginning");
                if let Some(p) = &s.paragraph {
                    let line = p
                        .lines
                        .iter()
                        .find(|l| s.input.caret >= l.utf16.start && s.input.caret < l.utf16.end)
                        .unwrap_or_else(|| p.lines.last().unwrap());
                    let target = if begin {
                        line.utf16.start
                    } else if s
                        .input
                        .text
                        .get(line.bytes.clone())
                        .is_some_and(|s| s.ends_with('\n'))
                    {
                        line.utf16.end.saturating_sub(1)
                    } else {
                        line.utf16.end
                    };
                    s.input.select(target, extend);
                }
            }
            "moveUp:"
            | "moveUpAndModifySelection:"
            | "moveDown:"
            | "moveDownAndModifySelection:" => {
                let c = s.caret();
                let step = if command.contains("Up") { -1.0 } else { 1.0 };
                let p = Point::new(c.min_x(), c.center().y + step * c.size.height);
                let hit = s.hit_drag(p);
                s.input.select(hit, extend);
            }
            "pageDown:" | "pageUp:" => {
                self.reveal_pending = false;
                let sign = if command == "pageUp:" { -1.0 } else { 1.0 };
                let amount = s.inner.size.height * 0.9 * sign;
                s.scroll.scroll_by(0.0, amount);
                return Some(true);
            }
            "insertNewline:" | "insertLineBreak:" => {
                if s.input.multiline {
                    s.input.insert("\n", None);
                }
            }
            "cancelOperation:" => {
                s.input.cancel_composition();
            }
            _ => return Some(false),
        }
        Some(true)
    }
    pub fn refresh(&mut self, text: &mut TextEngine) {
        for s in &mut self.surfaces {
            if s.outer.size.width > 0.0 {
                let rect = s.outer;
                s.layout(text, rect);
            }
        }
        if self.reveal_pending {
            if let Some(i) = self.focus {
                self.surfaces[i].reveal();
            }
            self.reveal_pending = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::static_text::StaticText;
    #[test]
    fn language_switch_changes_only_pristine_samples_and_preserves_documents() {
        use crate::gallery::locale::Locale;
        let mut demo = TextDemo::default();
        assert!(
            demo.surfaces[0]
                .input
                .text
                .starts_with("A day by the window")
        );
        let read_only = demo.surfaces[0].input.read_only;
        demo.set_locale(Locale::ZhCn);
        assert_eq!(
            demo.surfaces[0].input.text,
            include_str!("../../../../fixtures/text/gallery-reading.txt").trim_end()
        );
        assert_eq!(demo.surfaces[0].input.read_only, read_only);
        demo.set_locale(Locale::En);
        demo.surfaces[0]
            .input
            .text
            .push_str(" My own words. 我的文字。");
        let edited = demo.surfaces[0].input.text.clone();
        demo.surfaces[1].input.marked = Some(0..1);
        let composing = demo.surfaces[1].input.text.clone();
        demo.set_locale(Locale::ZhCn);
        assert_eq!(demo.surfaces[0].input.text, edited);
        assert_eq!(demo.surfaces[1].input.text, composing);
        assert_eq!(demo.surfaces[1].input.marked, Some(0..1));
        assert!(demo.surfaces[2].input.text.contains("窗"));
    }

    #[test]
    fn reflow_keeps_a_visible_insertion_caret_visible_without_revealing_a_scrolled_away_caret() {
        let mut text = TextEngine::new();
        let mut s = Surface::new(
            include_str!("../../../../fixtures/text/gallery-reading.txt"),
            true,
            false,
        );
        s.layout(&mut text, Rect::new(0.0, 0.0, 640.0, 260.0));
        s.input.select(500, false);
        s.reveal();
        assert!(s.caret().max_y() <= s.inner.max_y());
        s.layout_preserving_caret_clipped(&mut text, Rect::new(0.0, 0.0, 320.0, 260.0), None, true);
        assert!(s.caret().min_y() >= s.inner.min_y());
        assert!(s.caret().max_y() <= s.inner.max_y());
        s.input.select(0, false);
        s.scroll.scroll_by(0.0, 800.0);
        s.layout_preserving_caret_clipped(&mut text, Rect::new(0.0, 0.0, 400.0, 260.0), None, true);
        assert!(s.scroll.offset.y > 0.0);
        assert!(s.caret().max_y() < s.inner.min_y());
    }

    #[test]
    fn reflow_keeps_reading_anchor_without_accumulating_wrap_drift() {
        let mut text = TextEngine::new();
        for read_only in [false, true] {
            let mut s = Surface::new(
                include_str!("../../../../fixtures/text/gallery-reading.txt"),
                true,
                read_only,
            );
            s.layout(&mut text, Rect::new(0.0, 0.0, 640.0, 260.0));
            s.scroll.scroll_by(0.0, 713.25);
            s.input.caret = 120;
            s.input.anchor = 12;
            let initial = s.scroll.offset.y;
            let mut logical_anchor = None;
            for w in (320..640).rev().chain(320..=640) {
                s.layout(&mut text, Rect::new(0.0, 0.0, w as f32, 260.0));
                let anchor = s.reflow_anchor.unwrap();
                assert_eq!(*logical_anchor.get_or_insert(anchor.utf16), anchor.utf16);
                let expected = s.caret_doc(anchor.utf16).min_y() + anchor.within_line;
                assert!((s.scroll.offset.y - expected).abs() < 0.001);
                assert_eq!((s.input.anchor, s.input.caret), (12, 120));
                assert!(!s.paragraph.as_ref().unwrap().lines.is_empty());
                assert_eq!(s.scroll.limit().x, 0.0);
                assert_eq!(
                    s.scroll.scroll_policy(),
                    (
                        tekes_ui_controls::ScrollPolicy::Disabled,
                        tekes_ui_controls::ScrollPolicy::Auto
                    )
                );
            }
            assert!((s.scroll.offset.y - initial).abs() < 0.001);
            // A new user scroll must replace the old reading anchor.
            let old = logical_anchor.unwrap();
            s.scroll.scroll_by(0.0, 220.0);
            s.layout(&mut text, Rect::new(0.0, 0.0, 639.0, 260.0));
            assert_ne!(s.reflow_anchor.unwrap().utf16, old);
        }
    }

    #[test]
    fn reflow_pins_document_end_and_ignores_subpixel_width_noise() {
        let mut text = TextEngine::new();
        for scale in [1.0, 2.0] {
            text.set_scale(scale);
            let mut s = Surface::new(
                include_str!("../../../../fixtures/text/gallery-reading.txt"),
                true,
                false,
            );
            s.layout(&mut text, Rect::new(0.0, 0.0, 640.0, 260.0));
            let count = text.shape_count();
            let paragraph = s.paragraph.as_ref().unwrap().clone();
            s.layout(&mut text, Rect::new(0.0, 0.0, 640.0 + 0.2 / scale, 260.0));
            assert_eq!(count, text.shape_count());
            assert!(std::rc::Rc::ptr_eq(
                &paragraph,
                s.paragraph.as_ref().unwrap()
            ));
            s.scroll.scroll_by(0.0, f32::MAX);
            for (w, h) in [(320.0, 300.0), (700.0, 200.0), (640.0, 260.0)] {
                s.layout(&mut text, Rect::new(0.0, 0.0, w, h));
                assert!((s.scroll.offset.y - s.scroll.limit().y).abs() < 0.001);
            }
            s.input.text = "A中👨‍👩‍👧‍👦e\u{301}\n新的内容".into();
            s.layout(&mut text, Rect::new(0.0, 0.0, 400.0, 260.0));
            assert!(s.reflow_anchor.is_none());
            assert_eq!(s.scroll.offset.y, 0.0);
        }
    }

    #[test]
    fn type_scale_reflow_preserves_middle_anchor_and_document_bottom() {
        let mut text = TextEngine::new();
        let document = include_str!("../../../../fixtures/text/gallery-reading.txt");
        for bottom in [false, true] {
            let mut surface = Surface::new(document, true, true);
            let frame = Rect::new(10.0, 10.0, 320.0, 180.0);
            surface.layout(&mut text, frame);
            if bottom {
                surface.scroll.scroll_by(0.0, f32::MAX);
            } else {
                surface.scroll.scroll_by(0.0, 713.25);
            }
            surface.layout(&mut text, frame);
            let initial_offset = surface.scroll.offset.y;
            let initial_utf16 = surface
                .paragraph
                .as_ref()
                .unwrap()
                .lines
                .iter()
                .rev()
                .find(|line| line.top as f32 <= initial_offset)
                .unwrap()
                .utf16
                .start;

            text.set_type_scale(1.5);
            surface.layout(&mut text, frame);
            let scaled_anchor = surface.reflow_anchor.unwrap();
            assert_eq!(scaled_anchor.utf16, initial_utf16);
            if bottom {
                assert!((surface.scroll.offset.y - surface.scroll.limit().y).abs() < 0.001);
            }

            text.set_type_scale(1.0);
            surface.layout(&mut text, frame);
            let restored_anchor = surface.reflow_anchor.unwrap();
            assert_eq!(restored_anchor.utf16, initial_utf16);
            if bottom {
                assert!((surface.scroll.offset.y - surface.scroll.limit().y).abs() < 0.001);
            } else {
                assert!((surface.scroll.offset.y - initial_offset).abs() < 0.001);
            }
        }
    }

    #[test]
    fn text_role_change_invalidates_both_gallery_preparation_and_surface_paragraph() {
        let mut text = TextEngine::new();
        let mut surface = Surface::new("Role-specific text", false, true);
        let frame = Rect::new(0.0, 0.0, 320.0, 40.0);
        surface.layout(&mut text, frame);
        let shape_count = text.shape_count();
        let old_paragraph = surface.paragraph.as_ref().unwrap().clone();

        surface.view.set_role(proposal::TextRole::UiCaption);
        surface.layout(&mut text, frame);
        assert!(text.shape_count() > shape_count);
        assert!(!std::rc::Rc::ptr_eq(
            &old_paragraph,
            surface.paragraph.as_ref().unwrap()
        ));
        let after_role_shape = text.shape_count();
        surface.layout(&mut text, frame);
        assert_eq!(text.shape_count(), after_role_shape);
    }

    #[test]
    fn selection_capture_survives_layout_when_only_a_visible_sliver_remains() {
        let mut text = TextEngine::new();
        let mut demo = TextDemo::field("A long enough read-only label", false, true);
        let frame = Rect::new(20.0, 20.0, 120.0, 40.0);
        let clip = Rect::new(90.0, 20.0, 25.0, 40.0);
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(&mut ctx, frame, Some(clip), &mut Scene::new());
        let start = Point::new(94.0, 40.0);
        let down = Event::Mouse(tekes_ui_platform_macos::MouseEvent {
            kind: MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
            position: tekes_ui_platform_macos::Point {
                x: f64::from(start.x),
                y: f64::from(start.y),
            },
            modifiers: Default::default(),
        });
        assert_eq!(demo.handle(&down, Some(start)), Some(true));
        assert!(!demo.surfaces[0].contains_visible(frame.center()));

        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(&mut ctx, frame, Some(clip), &mut Scene::new());
        assert_eq!(
            demo.handle(
                &Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                    kind: MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    },
                    position: tekes_ui_platform_macos::Point { x: 101.0, y: 40.0 },
                    modifiers: Default::default(),
                }),
                Some(Point::new(101.0, 40.0)),
            ),
            Some(true)
        );
        assert!(!demo.surfaces[0].input.selection().is_empty());
    }

    fn layout(demo: &mut TextDemo, text: &mut TextEngine) -> Scene {
        text.begin_frame();
        let mut st = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text,
            static_text: &mut st,
        };
        let mut scene = Scene::new();
        demo.build(&mut ctx, super::super::layout::PAGE_CONTENT, &mut scene);
        scene
    }
    #[test]
    fn live_editor_and_scroll_panels_fit_remaining_viewport_and_padding_is_symmetric() {
        let mut text = TextEngine::new();
        for (width, height) in [(400.0, 280.0), (1104.0, 700.0), (1600.0, 1100.0)] {
            let content = Rect::new(312.0, 104.0, width, height);
            for mut demo in [TextDemo::editors_only(), TextDemo::scroll_only()] {
                let sections = demo.sections(content, 1.0, &[]);
                let panel = sections.last().unwrap().2;
                assert_eq!(panel.max_y(), content.max_y());
                for (_, _, rect) in sections {
                    assert!(rect.max_y() <= content.max_y());
                }
                let surface = demo.surfaces.last_mut().unwrap();
                surface.layout(&mut text, panel);
                let padding_top = surface.inner.min_y() - surface.outer.min_y();
                let padding_bottom = surface.outer.max_y() - surface.inner.max_y();
                assert_eq!(padding_top, padding_bottom);
                let vertical_track = surface.track(true).inset(-Spacing::Micro.value());
                let horizontal_track = surface.track(false).inset(-Spacing::Micro.value());
                assert!(surface.inner.max_x() < vertical_track.min_x());
                assert!(surface.inner.max_y() < horizontal_track.min_y());
                surface.scroll.scroll_by(0.0, f32::MAX);
                let paragraph = surface.paragraph.as_ref().unwrap();
                if paragraph.height as f32 > surface.inner.size.height {
                    assert!(
                        (surface.origin().y + paragraph.height as f32 - surface.inner.max_y())
                            .abs()
                            < 0.001
                    );
                } else {
                    assert_eq!(surface.origin().y, surface.inner.min_y());
                }
            }
        }
        assert!(
            !TextDemo::scroll_only().surfaces[0]
                .input
                .text
                .ends_with('\n')
        );
    }

    #[test]
    fn editor_layout_is_cached_scroll_does_not_reshape_and_all_text_stays_in_scissors() {
        let mut demo = TextDemo::default();
        let mut text = TextEngine::new();
        let a = layout(&mut demo, &mut text);
        let count = text.shape_count();
        assert_eq!(a, layout(&mut demo, &mut text));
        assert_eq!(count, text.shape_count());
        let (limit, _) = demo.scroll_metrics();
        assert!(limit > 500.0);
        demo.scroll_to(limit);
        let b = layout(&mut demo, &mut text);
        assert_ne!(a, b);
        assert_eq!(count, text.shape_count());
        demo.scroll_to(0.0);
        assert_eq!(a, layout(&mut demo, &mut text));
    }
    #[test]
    fn caret_hit_testing_uses_shaped_grapheme_positions_and_newline_stays_on_its_line() {
        let mut demo = TextDemo::default();
        let mut text = TextEngine::new();
        demo.surfaces[2].input.text = "A中👨‍👩‍👧‍👦e\u{301}\n第二行".into();
        layout(&mut demo, &mut text);
        let s = &demo.surfaces[2];
        let first = &s.paragraph.as_ref().unwrap().lines[0];
        for &(index, x) in first.carets.iter().filter(|(i, _)| *i < first.utf16.end) {
            let origin = s.origin();
            let p = Point::new(origin.x + x as f32, origin.y + first.height as f32 * 0.5);
            assert_eq!(s.hit(p), Some(index), "index {index}");
        }
        let p = Point::new(
            s.inner.max_x() - 1.0,
            s.inner.min_y() + first.height as f32 * 0.5,
        );
        assert_eq!(s.hit(p), Some(first.utf16.end - 1));
    }
    #[test]
    fn app_deactivation_preserves_selection_and_restores_native_input_focus() {
        let mut demo = TextDemo::editors_only();
        let mut text = TextEngine::new();
        layout(&mut demo, &mut text);
        demo.focus = Some(0);
        demo.surfaces[0].input.select_all();
        let before = demo.state().unwrap();
        demo.set_window_active(false);
        assert!(demo.state().is_none());
        assert_eq!(demo.focus, Some(0));
        demo.set_window_active(true);
        let after = demo.state().unwrap();
        assert_eq!(before.text, after.text);
        assert_eq!(before.selection, after.selection);
        assert_eq!(before.caret_index, after.caret_index);
        demo.blur();
        assert!(demo.state().is_none());
    }
    #[test]
    fn native_edit_events_selection_copy_and_scroll_position_are_consistent() {
        let mut d = TextDemo::default();
        let mut text = TextEngine::new();
        layout(&mut d, &mut text);
        d.focus = Some(1);
        d.command("selectAll:");
        d.handle(
            &Event::TextCommit {
                text: "原始".into(),
                replacement: None,
            },
            None,
        );
        d.refresh(&mut text);
        d.handle(
            &Event::TextPreedit {
                clauses: Vec::new(),
                text: "qing".into(),
                selection: 4..4,
                replacement: None,
            },
            None,
        );
        d.refresh(&mut text);
        assert!(d.state().unwrap().marked.is_some());
        d.handle(
            &Event::TextCommit {
                text: "清晰".into(),
                replacement: None,
            },
            None,
        );
        d.refresh(&mut text);
        assert_eq!(d.state().unwrap().text, "原始清晰");
        d.command("undo:");
        d.refresh(&mut text);
        assert_eq!(d.state().unwrap().text, "原始");
        d.command("selectAll:");
        assert_eq!(d.clipboard_selection().as_deref(), Some("原始"));
        d.focus = Some(0);
        let original = d.surfaces[0].input.text.clone();
        assert!(!d.paste("不能写入"));
        assert_eq!(d.surfaces[0].input.text, original);
        d.focus = Some(2);
        d.command("moveToEndOfDocument:");
        d.refresh(&mut text);
        let s = &d.surfaces[2];
        assert!(s.scroll.offset.y > 0.0);
        let caret = s.caret();
        assert!(caret.min_y() >= s.inner.min_y() && caret.max_y() <= s.inner.max_y() + 0.01);
        let offset = s.scroll.offset;
        d.handle(&Event::CaretBlink, None);
        d.refresh(&mut text);
        assert_eq!(d.surfaces[2].scroll.offset, offset);
    }

    #[test]
    fn text_surface_minimum_height_and_gallery_sections_follow_type_scale() {
        let mut text = TextEngine::new();
        let mut demo = TextDemo::field("Text 中文 👨‍👩‍👧‍👦 e\u{301}", false, false);
        let mut measured = Vec::new();
        for scale in [1.0, 1.25, 1.5] {
            text.set_type_scale(scale);
            let height = demo.measure_search_height(&mut text);
            let paragraph_height = demo.surfaces[0].paragraph.as_ref().unwrap().height as f32;
            assert!(
                paragraph_height <= height - 2.0 * proposal::SCROLL_SINGLE_LINE_PADDING_Y + 0.001,
                "real mixed-script line height must fit its frame at {scale}x"
            );
            measured.push(height);
        }
        assert!(measured[1] > measured[0] && measured[2] > measured[1]);

        let content = Rect::new(24.0, 32.0, 480.0, 400.0);
        let default = TextDemo::editors_only().sections(content, 1.0, &[]);
        let large = TextDemo::editors_only().sections(content, 1.5, &[]);
        assert!(large[0].1.size.height > default[0].1.size.height);
        assert!(large[0].2.size.height > default[0].2.size.height);
    }

    #[test]
    fn ancestor_clip_zero_height_empty_and_tall_single_line_share_hit_geometry() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut demo = TextDemo::field("A中👨‍👩‍👧‍👦e\u{301}", false, false);
        let frame = Rect::new(100.0, 80.0, 240.0, demo.measure_search_height(&mut text));
        let clip = Rect::new(120.0, 70.0, 200.0, 100.0);
        demo.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            frame,
            Some(clip),
            &mut Scene::new(),
        );
        let surface = &demo.surfaces[0];
        assert_eq!(
            surface.visible_frame(),
            Some(Rect::new(120.0, 80.0, 200.0, frame.size.height))
        );
        assert!(
            surface
                .hit(Point::new(121.0, surface.inner.center().y))
                .is_some()
        );
        assert_eq!(
            surface.hit(Point::new(110.0, surface.inner.center().y)),
            None
        );
        let lower = Point::new(surface.inner.center().x, surface.inner.max_y() - 1.0);
        assert!(
            surface.hit(lower).is_some(),
            "large single-line frames remain selectable below line center"
        );
        let right = Point::new(clip.max_x() - 0.1, surface.inner.center().y);
        assert_eq!(
            surface.hit(right),
            Some(surface.input.len()),
            "blank trailing field space maps to the final caret"
        );

        let mut empty = TextDemo::field("", false, false);
        empty.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            Rect::new(0.0, 0.0, 100.0, 0.0),
            None,
            &mut Scene::new(),
        );
        assert!(empty.visible_frame().is_none());
        assert!(!empty.contains_visible(Point::new(1.0, 0.0)));
        assert!(empty.surfaces[0].hit(Point::new(1.0, 0.0)).is_none());
    }

    #[test]
    fn current_scroll_transform_drives_caret_and_hits_without_a_rebuild() {
        let mut text = TextEngine::new();
        let mut demo = TextDemo::scroll_only();
        let mut st = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut st,
        };
        demo.build_clipped(
            &mut ctx,
            Rect::new(20.0, 30.0, 280.0, 180.0),
            None,
            &mut Scene::new(),
        );
        let surface = &mut demo.surfaces[0];
        surface.input.select(220, false);
        let before = surface.caret();
        surface.scroll.scroll_by(0.0, 55.0);
        let after = surface.caret();
        assert!((before.min_y() - after.min_y() - 55.0).abs() < 0.001);
        let point = after.center();
        assert_eq!(surface.hit(point), Some(220));
    }

    #[test]
    fn captured_precise_scroll_continues_outside_surface_until_end_or_blur() {
        use tekes_ui_platform_macos::{ScrollEvent, ScrollPhase, Vec2};

        fn event(phase: ScrollPhase, momentum_phase: ScrollPhase, y: f64) -> Event {
            Event::Scroll(ScrollEvent {
                timestamp: 1.0,
                delta: Vec2 { x: 0.0, y },
                precise: true,
                phase,
                momentum_phase,
                inverted: false,
                position: tekes_ui_platform_macos::Point { x: 40.0, y: 40.0 },
                modifiers: Default::default(),
            })
        }

        let mut text = TextEngine::new();
        let mut demo = TextDemo::field(
            include_str!("../../../../fixtures/text/gallery-reading.txt"),
            true,
            false,
        );
        let frame = Rect::new(20.0, 20.0, 260.0, 80.0);
        demo.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            frame,
            None,
            &mut Scene::new(),
        );
        let inside = Point::new(40.0, 40.0);
        let outside = Point::new(420.0, 420.0);

        assert_eq!(
            demo.handle(
                &event(ScrollPhase::Began, ScrollPhase::None, -22.0),
                Some(inside)
            ),
            Some(true)
        );
        assert_eq!(demo.scroll_target, Some(0));
        let moved = demo.surfaces[0].scroll.offset;

        assert_eq!(
            demo.handle(
                &event(ScrollPhase::Changed, ScrollPhase::None, -26.0),
                Some(outside)
            ),
            Some(true)
        );
        assert!(demo.surfaces[0].scroll.offset.y > moved.y);
        assert_eq!(demo.scroll_target, Some(0));

        assert_eq!(
            demo.handle(
                &event(ScrollPhase::None, ScrollPhase::Changed, -18.0),
                Some(outside)
            ),
            Some(true)
        );
        assert!(
            demo.handle(
                &event(ScrollPhase::None, ScrollPhase::Ended, 0.0),
                Some(outside)
            )
            .is_some()
        );
        demo.end_scroll_gesture();
        assert_eq!(demo.scroll_target, None);

        assert_eq!(
            demo.handle(
                &event(ScrollPhase::Changed, ScrollPhase::None, -22.0),
                Some(outside)
            ),
            None
        );
        assert_eq!(
            demo.scroll_target, None,
            "an outside point cannot start a capture"
        );

        assert_eq!(
            demo.handle(
                &event(ScrollPhase::Began, ScrollPhase::None, -22.0),
                Some(inside)
            ),
            Some(true)
        );
        demo.blur();
        assert_eq!(
            demo.scroll_target, None,
            "blur cancels retained gesture routing"
        );
    }

    #[test]
    fn text_surface_pointer_capture_clip_read_only_and_disabled_are_consistent() {
        let mut text = TextEngine::new();
        let mut read_only = TextDemo::field("Selectable text", false, true);
        let rect = Rect::new(20.0, 20.0, 240.0, 40.0);
        read_only.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            rect,
            Some(Rect::new(20.0, 20.0, 120.0, 40.0)),
            &mut Scene::new(),
        );
        let start = Point::new(30.0, 35.0);
        assert_eq!(
            read_only.surfaces[0].pointer_down(start, false, 1, metric("size", "size-row-height")),
            Some(true)
        );
        assert!(
            read_only.surfaces[0]
                .pointer_drag(Point::new(400.0, 35.0), metric("size", "size-row-height"))
        );
        assert!(!read_only.surfaces[0].input.selection().is_empty());
        read_only.surfaces[0].cancel_interaction();

        let mut disabled = TextDemo::field("Disabled", false, false);
        disabled.set_disabled(true);
        disabled.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            rect,
            None,
            &mut Scene::new(),
        );
        assert_eq!(
            disabled.surfaces[0].pointer_down(start, false, 1, metric("size", "size-row-height")),
            None
        );
        assert!(
            !disabled.surfaces[0]
                .pointer_drag(Point::new(80.0, 35.0), metric("size", "size-row-height"))
        );
    }

    #[test]
    fn partially_clipped_marked_line_keeps_its_original_typographic_baseline() {
        let mut text = TextEngine::new();
        let mut demo = TextDemo::field("first line\nmarked final line", true, false);
        let rect = Rect::new(10.0, 10.0, 180.0, 46.0);
        demo.build_search_clipped(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut StaticText::new(None),
            },
            rect,
            None,
            &mut Scene::new(),
        );
        let surface = &mut demo.surfaces[0];
        let (top, line_height, range) = {
            let last = surface.paragraph.as_ref().unwrap().lines.last().unwrap();
            (
                last.top as f32,
                last.height as f32,
                last.utf16.start..last.utf16.end,
            )
        };
        let inner_height = surface.inner.size.height;
        surface
            .scroll
            .scroll_by(0.0, top - inner_height + line_height * 0.5);
        surface.input.marked = Some(range.clone());
        let rects = surface.selection_rects(range);
        assert!(!rects.is_empty());
        let line = rects[0];
        assert_eq!(line.size.height, line_height);
        assert!(
            line.max_y() > surface.inner.max_y(),
            "scene clip trims pixels without moving the IME underline baseline"
        );
    }

    #[test]
    fn hidden_and_zero_sized_previews_clear_the_old_pointer_target_then_restore_it() {
        let mut text = TextEngine::new();
        let mut demo = TextDemo::search();
        let frame = Rect::new(48.0, 72.0, 240.0, 40.0);
        let point = Point::new(80.0, 90.0);
        let down = Event::Mouse(tekes_ui_platform_macos::MouseEvent {
            kind: MouseEventKind::Down {
                button: MouseButton::Left,
                click_count: 1,
            },
            position: tekes_ui_platform_macos::Point {
                x: f64::from(point.x),
                y: f64::from(point.y),
            },
            modifiers: Default::default(),
        });
        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(&mut ctx, frame, None, &mut Scene::new());
        assert_eq!(demo.handle(&down, Some(point)), Some(true));
        assert_eq!(demo.focus, Some(0));
        demo.blur();

        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(
            &mut ctx,
            frame,
            Some(Rect::new(0.0, 0.0, 0.0, 0.0)),
            &mut Scene::new(),
        );
        assert_eq!(demo.visible_frame(), None);
        assert_eq!(demo.handle(&down, Some(point)), Some(false));
        assert_eq!(demo.focus, None);

        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(
            &mut ctx,
            Rect::new(frame.origin.x, frame.origin.y, frame.size.width, 0.0),
            None,
            &mut Scene::new(),
        );
        assert_eq!(demo.visible_frame(), None);
        assert_eq!(demo.handle(&down, Some(point)), Some(false));
        assert_eq!(demo.focus, None);

        let mut static_text = StaticText::new(None);
        let mut ctx = PageCtx {
            theme: Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        demo.build_search_clipped(&mut ctx, frame, None, &mut Scene::new());
        assert_eq!(demo.handle(&down, Some(point)), Some(true));
        assert_eq!(demo.focus, Some(0));
    }

    #[test]
    fn section_geometry_is_clamped_to_short_and_empty_viewports() {
        let demo = TextDemo::default();
        for height in [0.0, 1.0, 28.0, 60.0] {
            let content = Rect::new(12.0, 28.0, 300.0, height);
            let sections = demo.sections(content, 1.5, &[24.0, 24.0, 24.0]);
            assert_eq!(sections.len(), 3);
            for (_, label, panel) in sections {
                assert!(label.min_y() >= content.min_y() && label.max_y() <= content.max_y());
                assert!(panel.min_y() >= content.min_y() && panel.max_y() <= content.max_y());
            }
        }
    }

    #[test]
    fn intrinsic_height_queries_do_not_reflow_multiline_paragraphs() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.25);
        let mut demo = TextDemo::editors_only();
        let content = Rect::new(20.0, 30.0, 480.0, 320.0);
        let mut static_text = StaticText::new(None);
        {
            let mut ctx = PageCtx {
                theme: Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            demo.build_clipped(&mut ctx, content, None, &mut Scene::new());
        }
        let count = text.shape_count();
        let paragraph = demo.surfaces[0].paragraph.as_ref().unwrap().clone();
        let width = demo.surfaces[0].width;
        let height = demo.measure_search_height(&mut text);
        assert!(height > 0.0);
        assert_eq!(text.shape_count(), count);
        assert_eq!(demo.surfaces[0].width, width);
        assert!(std::rc::Rc::ptr_eq(
            &paragraph,
            demo.surfaces[0].paragraph.as_ref().unwrap()
        ));
    }
}

#[cfg(test)]
mod pixel_review {
    use super::*;
    #[test]
    fn reflow_renders_every_width_and_returns_to_identical_pixels() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let output = std::env::var_os("TEKES_REFLOW_EVIDENCE").map(std::path::PathBuf::from);
        if let Some(path) = &output {
            std::fs::create_dir_all(path).unwrap();
        }
        let mut timings = Vec::new();
        for scale in [1, 2] {
            for read_only in [false, true] {
                let mut text = TextEngine::new();
                text.set_scale(scale as f32);
                let mut view = Surface::new(
                    include_str!("../../../../fixtures/text/gallery-reading.txt"),
                    true,
                    read_only,
                );
                view.layout(&mut text, Rect::new(20.0, 20.0, 640.0, 260.0));
                view.scroll.scroll_by(0.0, 713.25);
                let mut first = None;
                for (frame, width) in [
                    640, 600, 560, 520, 480, 440, 400, 360, 320, 360, 400, 440, 480, 520, 560, 600,
                    640,
                ]
                .into_iter()
                .enumerate()
                {
                    text.begin_frame();
                    let start = std::time::Instant::now();
                    view.layout(&mut text, Rect::new(20.0, 20.0, width as f32, 260.0));
                    timings.push(start.elapsed().as_secs_f64() * 1000.0);
                    let mut scene = Scene::new();
                    view.draw(&mut text, &mut scene, Theme::LIGHT, false, true);
                    let size = (680 * scale, 300 * scale);
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            size,
                            scale as f32,
                            ink("document-canvas", Theme::LIGHT),
                        )
                        .unwrap();
                    // A visible paper must never become a blank frame during reflow.
                    let dark = rgba
                        .chunks_exact(4)
                        .enumerate()
                        .filter(|(i, p)| {
                            let x = *i as u32 % size.0;
                            let y = *i as u32 / size.0;
                            x > 40 * scale
                                && x < (width - 20) * scale
                                && y > 40 * scale
                                && y < 260 * scale
                                && p[0] < 150
                                && p[1] < 150
                                && p[2] < 150
                        })
                        .count();
                    assert!(dark > 100, "missing text at frame {frame}, scale {scale}");
                    if frame == 0 {
                        first = Some(rgba.clone());
                    }
                    if frame == 16 {
                        assert_eq!(first.as_ref().unwrap(), &rgba);
                    }
                    if let Some(dir) = &output {
                        let file = std::fs::File::create(dir.join(format!(
                            "{}-{frame:02}-{width}@{scale}x.png",
                            if read_only { "text" } else { "editor" }
                        )))
                        .unwrap();
                        let mut encoder = png::Encoder::new(file, size.0, size.1);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&rgba)
                            .unwrap();
                    }
                }
            }
        }
        timings.sort_by(f64::total_cmp);
        let report = serde_json::json!({"scope":"synchronous text layout only; excludes GPU and native presentation", "samples":timings.len(),"p50_ms":timings[timings.len()/2],"p95_ms":timings[timings.len()*95/100],"p99_ms":timings[timings.len()*99/100],"max_ms":timings.last()});
        eprintln!("reflow timing: {report}");
        if let Some(dir) = output {
            std::fs::write(
                dir.join("layout-timing.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn selected_text_and_caret_share_the_same_measured_line_boxes_at_both_scales() {
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("Metal adapter required for layout review");
        let output = std::env::var_os("TEKES_TEXT_LAYOUT_EVIDENCE").map(std::path::PathBuf::from);
        if let Some(path) = &output {
            std::fs::create_dir_all(path).unwrap();
        }
        for theme in Theme::ALL {
            for scale in [1, 2] {
                for multiline in [false, true] {
                    let mut text = TextEngine::new();
                    text.set_scale(scale as f32);
                    text.begin_frame();
                    let value = if multiline {
                        "第一行 ABC e\u{301}\n第二行 👨‍👩‍👧‍👦，中英文混排。"
                    } else {
                        "Text 中文 e\u{301} 👨‍👩‍👧‍👦"
                    };
                    let mut view = Surface::new(value, multiline, false);
                    let h = if multiline {
                        160.0
                    } else {
                        metric("size", "size-button-md")
                    };
                    view.layout(&mut text, Rect::new(20.0, 20.0, 600.0, h));
                    view.input.select_all();
                    let boxes = view.selection_rects(view.input.selection());
                    assert!(!boxes.is_empty());
                    if !multiline {
                        let line = &view.paragraph.as_ref().unwrap().lines[0];
                        let caret = view.caret();
                        assert!(
                            (caret.center().y - view.outer.center().y).abs() <= 0.5,
                            "single-line caret must be vertically centered"
                        );
                        assert_eq!(boxes[0].min_y(), caret.min_y());
                        assert_eq!(boxes[0].size.height, caret.size.height);
                        assert!(
                            (boxes[0].max_x() - caret.min_x()).abs() < 0.01,
                            "selection/caret horizontal edges use CoreText advances"
                        );
                        assert!(
                            line.height <= f64::from(view.inner.size.height),
                            "font fallback must fit the line viewport"
                        );
                    }
                    let mut scene = Scene::new();
                    view.draw(&mut text, &mut scene, theme, true, true);
                    let size = (640 * scale, (h as u32 + 40) * scale);
                    let rgba = renderer
                        .render_to_rgba8(
                            &scene,
                            text.atlas_mut(),
                            size,
                            scale as f32,
                            ink("document-canvas", theme),
                        )
                        .unwrap();
                    if let Some(dir) = &output {
                        let path = dir.join(format!(
                            "{}-{}@{}x.png",
                            if multiline {
                                "multiline-selected"
                            } else {
                                "single-selected"
                            },
                            super::super::theme::theme_slug(theme),
                            scale
                        ));
                        let file = std::fs::File::create(path).unwrap();
                        let mut encoder = png::Encoder::new(file, size.0, size.1);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&rgba)
                            .unwrap();
                    }
                    // Review the insertion caret separately: selection normally hides it.
                    let caret_index = view.input.caret;
                    view.input.select(caret_index, false);
                    let mut caret_scene = Scene::new();
                    view.draw(&mut text, &mut caret_scene, theme, true, true);
                    let rgba = renderer
                        .render_to_rgba8(
                            &caret_scene,
                            text.atlas_mut(),
                            size,
                            scale as f32,
                            ink("document-canvas", theme),
                        )
                        .unwrap();
                    if let Some(dir) = &output {
                        let path = dir.join(format!(
                            "{}-caret-{}@{}x.png",
                            if multiline { "multiline" } else { "single" },
                            super::super::theme::theme_slug(theme),
                            scale
                        ));
                        let mut encoder =
                            png::Encoder::new(std::fs::File::create(path).unwrap(), size.0, size.1);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder
                            .write_header()
                            .unwrap()
                            .write_image_data(&rgba)
                            .unwrap();
                    }
                }
            }
        }
    }
}

pub fn build_editors_fixture(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
    let mut demo = TextDemo::editors_only();
    for s in &mut demo.surfaces {
        s.fixture = true;
    }
    demo.build(ctx, content, scene);
}
pub fn build_text_fixture(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
    let mut demo = TextDemo::text_only();
    demo.surfaces[0].fixture = true;
    demo.build(ctx, content, scene);
}
pub fn build_scroll_fixture(ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
    let mut demo = TextDemo::scroll_only();
    demo.surfaces[0].fixture = true;
    demo.build(ctx, content, scene);
}
