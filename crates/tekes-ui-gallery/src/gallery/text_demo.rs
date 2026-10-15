//! Live Text and TextEditor surfaces share shaping, selection, scrolling and IME data.
use super::{PageCtx, text::TextEngine};
use tekes_ui_controls::{ScrollArea, TextInput};
use tekes_ui_platform_macos::{
    Event, KeyEventKind, MouseButton, MouseEventKind, ScrollPhase, TextInputState,
};
use tekes_ui_scene::{Point, Quad, Rect, Scene, Size};
use tekes_ui_text::Paragraph;
use tekes_ui_tokens::{Spacing, Stroke, Theme, proposal};

fn ink(id: &str, theme: Theme) -> tekes_ui_tokens::Rgba {
    proposal::render_color(id, theme).expect("JSON text role")
}
fn metric(group: &str, id: &str) -> f32 {
    proposal::metric(group, id).expect("JSON metric")
}
#[derive(Debug)]
struct Surface {
    input: TextInput,
    scroll: ScrollArea,
    paragraph: Option<std::rc::Rc<Paragraph>>,
    shaped: String,
    width: Option<f32>,
    outer: Rect,
    inner: Rect,
    role: proposal::TextRole,
    fixture: bool,
    clauses: Vec<std::ops::Range<usize>>,
}
impl Surface {
    fn new(text: &str, multiline: bool, read_only: bool) -> Self {
        let mut input = TextInput::new(text, multiline);
        input.read_only = read_only;
        Self {
            input,
            scroll: ScrollArea::default(),
            paragraph: None,
            shaped: String::new(),
            width: None,
            outer: Rect::default(),
            inner: Rect::default(),
            fixture: false,
            clauses: Vec::new(),
            role: if multiline {
                proposal::TextRole::ComposerBody
            } else {
                proposal::TextRole::UiBody
            },
        }
    }
    fn layout(&mut self, text: &mut TextEngine, rect: Rect) {
        self.outer = rect;
        self.inner = rect.inset(Spacing::Regular.value());
        let width = self
            .input
            .multiline
            .then_some(self.inner.size.width.max(1.0));
        if self.paragraph.is_none() || self.shaped != self.input.text || self.width != width {
            self.paragraph = Some(if self.fixture {
                text.proposal_label(&self.input.text, self.role, width)
            } else {
                std::rc::Rc::new(text.shape_proposal(&self.input.text, self.role, width))
            });
            self.shaped.clone_from(&self.input.text);
            self.width = width;
        }
        let p = self.paragraph.as_ref().unwrap();
        self.scroll.layout(
            self.inner.size,
            Size::new(
                (p.width as f32 + Stroke::Rule.value()).max(self.inner.size.width),
                p.height as f32,
            ),
        );
    }
    fn caret_doc(&self, index: usize) -> Rect {
        self.paragraph.as_ref().map_or(Rect::default(), |p| {
            p.caret_rect(index, Stroke::Rule.value())
        })
    }
    fn origin(&self) -> Point {
        let mut origin = self.paragraph.as_ref().map_or(self.inner.origin, |p| {
            p.origin_in(self.inner, !self.input.multiline)
        });
        let offset = self.scroll.visual_offset();
        origin.x -= offset.x;
        origin.y -= offset.y;
        origin
    }
    fn caret(&self) -> Rect {
        let c = self.caret_doc(self.input.caret);
        let o = self.origin();
        c.offset(o.x, o.y)
    }
    fn hit(&self, point: Point) -> usize {
        let origin = self.origin();
        self.paragraph.as_ref().map_or(0, |p| {
            p.hit_test(Point::new(point.x - origin.x, point.y - origin.y))
        })
    }
    fn selection_rects(&self, range: std::ops::Range<usize>) -> Vec<Rect> {
        let origin = self.origin();
        self.paragraph.as_ref().map_or_else(Vec::new, |p| {
            p.selection_rects(range, self.inner.offset(-origin.x, -origin.y))
                .into_iter()
                .map(|r| r.offset(origin.x, origin.y))
                .collect()
        })
    }
    fn draw(
        &self,
        text: &mut TextEngine,
        scene: &mut Scene,
        theme: Theme,
        focused: bool,
        caret_on: bool,
    ) {
        let radius = metric("radius", "radius-input");
        scene.push_quad(
            Quad::new(
                self.outer,
                ink(
                    if self.input.read_only {
                        "surface-1"
                    } else {
                        "transcript-input"
                    },
                    theme,
                ),
            )
            .with_corner_radius(radius)
            .with_border(
                Stroke::Rule.value(),
                ink(if focused { "border-focused" } else { "border" }, theme),
            ),
        );
        scene.push_clip(self.inner);
        let selection = self.input.selection();
        for rect in self.selection_rects(selection.clone()) {
            scene.push_quad(Quad::new(rect, ink("text-selection", theme)));
        }
        if let Some(p) = &self.paragraph {
            text.push_visible(
                scene,
                p,
                self.origin(),
                ink("primary-text", theme),
                self.inner,
            );
            if !selection.is_empty() {
                for rect in self.selection_rects(selection) {
                    scene.push_clip(rect);
                    text.push_visible(scene, p, self.origin(), ink("selected-text", theme), rect);
                    scene.pop_clip();
                }
            }
        }
        if let Some(mark) = &self.input.marked {
            let ranges = if self.clauses.is_empty() {
                vec![mark.clone()]
            } else {
                self.clauses
                    .iter()
                    .map(|r| {
                        mark.start.saturating_add(r.start).min(mark.end)
                            ..mark.start.saturating_add(r.end).min(mark.end)
                    })
                    .collect()
            };
            for r in ranges
                .into_iter()
                .flat_map(|range| self.selection_rects(range))
            {
                scene.push_quad(Quad::new(
                    Rect::new(
                        r.min_x(),
                        r.max_y() - Stroke::Rule.value(),
                        r.size.width,
                        Stroke::Rule.value(),
                    ),
                    ink("primary-text", theme),
                ));
            }
        }
        if focused && !self.input.read_only && caret_on && self.input.selection().is_empty() {
            scene.push_quad(Quad::new(self.caret(), ink("primary-text", theme)));
        }
        scene.pop_clip();
        for vertical in [true, false] {
            let limit = self.scroll.limit();
            if (if vertical { limit.y } else { limit.x }) > 0.0 {
                let track = self.track(vertical);
                let q = self
                    .scroll
                    .thumb(vertical, track, metric("size", "size-row-height"));
                scene.push_quad(
                    Quad::new(q, ink("secondary-text", theme))
                        .with_corner_radius(Spacing::Micro.value()),
                );
            }
        }
    }
    fn track(&self, vertical: bool) -> Rect {
        let i = self.inner;
        let t = Spacing::Tight.value();
        if vertical {
            Rect::new(
                self.outer.max_x() - t - Spacing::Micro.value(),
                i.min_y(),
                t,
                i.size.height,
            )
        } else {
            Rect::new(
                i.min_x(),
                self.outer.max_y() - t - Spacing::Micro.value(),
                i.size.width,
                t,
            )
        }
    }
    fn reveal(&mut self) {
        let c = self.caret_doc(self.input.caret);
        self.scroll.reveal(c);
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
        let readonly="清晰的文字，平滑的阅读。\nText · CJK / Latin / emoji 👨‍👩‍👧‍👦 / e\u{301}\n支持选择、复制与独立滚动。\n".repeat(8);
        let mut document =
            String::from("在这里编辑多行文字。\n可以输入中文、选择、复制粘贴，并撤销重做。\n\n");
        for i in 1..=100 {
            document.push_str(&format!(
                "第 {i} 行 · 文字随纸张平滑滚动，光标和选区保持准确。\n"
            ));
        }
        Self {
            surfaces: vec![
                Surface::new(&readonly, true, true),
                Surface::new("示例项目名称", false, false),
                Surface::new(&document, true, false),
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
        demo.surfaces = vec![surface];
        demo
    }
    fn sections(&self, content: Rect) -> Vec<(&'static str, Rect, Rect)> {
        let gap = Spacing::Loose.value();
        let label_h = proposal::TextRole::UiHeading.style().line_height;
        let line_h = proposal::TextRole::ComposerBody.style().line_height;
        let mut y = content.min_y();
        let mut out = Vec::new();
        for (i, surface) in self.surfaces.iter().enumerate() {
            let name = if surface.input.read_only {
                "Text · 选择与复制"
            } else if surface.input.multiline {
                "TextEditor · 多行"
            } else {
                "TextEditor · 单行"
            };
            let label = Rect::new(content.min_x(), y, content.size.width, label_h);
            y += label_h + Spacing::Regular.value();
            let height = if self.surfaces.len() <= 2
                && i + 1 == self.surfaces.len()
                && surface.input.multiline
            {
                (content.max_y() - y).max(line_h)
            } else if surface.input.read_only {
                line_h * 8.0
            } else if surface.input.multiline {
                line_h * 20.0
            } else {
                metric("size", "size-button-md")
            };
            let panel = Rect::new(content.min_x(), y, content.size.width, height);
            out.push((name, label, panel));
            y += height + gap;
        }
        out
    }
    pub fn build(&mut self, ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
        for (i, (name, label, panel)) in self.sections(content).into_iter().enumerate() {
            ctx.text.proposal_label_in(
                scene,
                ctx.theme,
                label,
                name,
                proposal::TextRole::UiHeading,
                ink("primary-text", ctx.theme),
            );
            self.surfaces[i].layout(ctx.text, panel);
            self.surfaces[i].draw(
                ctx.text,
                scene,
                ctx.theme,
                self.window_active && self.focus == Some(i),
                self.caret_on,
            );
        }
    }
    pub fn editor_target(index: usize) -> Point {
        Self::editors_only().sections(super::layout::PAGE_CONTENT)[index]
            .2
            .inset(Spacing::Regular.value())
            .origin
    }
    pub fn scroll_metrics(&self) -> (f32, f32) {
        let s = self.surfaces.last().unwrap();
        (s.scroll.limit().y, s.inner.size.height)
    }
    pub fn scroll_to(&mut self, offset: f32) {
        let s = self.surfaces.last_mut().unwrap();
        s.scroll.scroll_by(0.0, offset - s.scroll.offset.y);
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
    pub fn search_fixture() -> Self {
        let mut field = Self::search();
        field.surfaces[0].fixture = true;
        field
    }
    pub fn query(&self) -> &str {
        &self.surfaces[0].input.text
    }
    pub fn build_search(&mut self, ctx: &mut PageCtx<'_>, rect: Rect, scene: &mut Scene) {
        self.surfaces[0].layout(ctx.text, rect);
        self.surfaces[0].draw(
            ctx.text,
            scene,
            ctx.theme,
            self.window_active && self.focus == Some(0),
            self.caret_on,
        );
        if self.query().is_empty() && self.focus.is_none() {
            ctx.text.proposal_label_in(
                scene,
                ctx.theme,
                rect.inset(Spacing::Regular.value()),
                "按名称搜索…",
                proposal::TextRole::UiBody,
                ink("secondary-text", ctx.theme),
            );
        }
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
                surface.scroll.cancel();
            }
            self.drag = None;
            self.scroll_target = None;
        }
    }
    pub fn blur(&mut self) {
        for s in &mut self.surfaces {
            s.input.unmark();
            s.scroll.cancel();
        }
        self.focus = None;
        self.drag = None;
        self.scroll_target = None;
    }
    pub fn contains_surface(&self, p: Point) -> bool {
        self.surfaces.iter().any(|s| s.outer.contains(p))
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
                    let index = self.surfaces.iter().position(|s| s.outer.contains(point));
                    let before = self.focus;
                    self.focus = index;
                    self.drag = index;
                    self.caret_on = true;
                    if let Some(i) = index {
                        let s = &mut self.surfaces[i];
                        s.input.unmark();
                        for v in [true, false] {
                            if s.scroll.pointer_down(
                                point,
                                v,
                                s.track(v).inset(-Spacing::Micro.value()),
                                metric("size", "size-row-height"),
                            ) {
                                return Some(true);
                            }
                        }
                        let index = s.hit(point);
                        s.input.select(index, m.modifiers.shift);
                        if click_count >= 3 {
                            s.input.select_all();
                        } else if click_count == 2 {
                            s.input.move_by(-1, false, true);
                            s.input.move_by(1, true, true);
                        }
                        return Some(true);
                    }
                    Some(before.is_some())
                }
                MouseEventKind::Dragged {
                    button: MouseButton::Left,
                } => {
                    let i = self.drag?;
                    let p = point?;
                    let s = &mut self.surfaces[i];
                    if let Some(v) = s.scroll.dragging_axis() {
                        let track = s.track(v).inset(-Spacing::Micro.value());
                        return Some(s.scroll.pointer_drag(
                            p,
                            track,
                            metric("size", "size-row-height"),
                        ));
                    }
                    let hit = s.hit(p);
                    s.input.select(hit, true);
                    s.reveal();
                    Some(true)
                }
                MouseEventKind::Up {
                    button: MouseButton::Left,
                    ..
                } => {
                    self.drag = None;
                    for s in &mut self.surfaces {
                        s.scroll.pointer_up();
                    }
                    Some(false)
                }
                _ => None,
            },
            Event::Scroll(e) => {
                let p = point?;
                let hit = self.surfaces.iter().position(|s| s.outer.contains(p));
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
                let s = &mut self.surfaces[i];
                let step = if e.precise {
                    1.0
                } else {
                    metric("size", "size-row-height")
                };
                let x = -(e.delta.x as f32) * step;
                let y = -(e.delta.y as f32) * step;
                let (x, y) = if e.modifiers.shift {
                    (x + y, 0.0)
                } else {
                    (x, y)
                };
                let before = s.scroll.offset;
                let changed = s.scroll.wheel(
                    x,
                    y,
                    e.precise,
                    e.phase == ScrollPhase::Began,
                    matches!(e.phase, ScrollPhase::Ended | ScrollPhase::Cancelled)
                        || matches!(
                            e.momentum_phase,
                            ScrollPhase::Ended | ScrollPhase::Cancelled
                        ),
                    e.momentum_phase != ScrollPhase::None,
                );
                // At a non-elastic wheel boundary allow the enclosing page to continue.
                if !changed && before == s.scroll.offset && !e.precise {
                    None
                } else {
                    Some(changed)
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
                s.input.select(s.input.len(), extend)
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
                    s.input.select(
                        if begin {
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
                        },
                        extend,
                    );
                }
            }
            "moveUp:"
            | "moveUpAndModifySelection:"
            | "moveDown:"
            | "moveDownAndModifySelection:" => {
                let c = s.caret();
                let step = if command.contains("Up") { -1.0 } else { 1.0 };
                let p = Point::new(c.min_x(), c.center().y + step * c.size.height);
                let hit = s.hit(p);
                s.input.select(hit, extend);
            }
            "pageDown:" | "pageUp:" => {
                self.reveal_pending = false;
                let sign = if command == "pageUp:" { -1.0 } else { 1.0 };
                s.scroll.scroll_by(0.0, s.inner.size.height * 0.9 * sign);
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
            assert_eq!(s.hit(p), index, "index {index}");
        }
        let p = Point::new(
            s.inner.max_x() - 1.0,
            s.inner.min_y() + first.height as f32 * 0.5,
        );
        assert_eq!(s.hit(p), first.utf16.end - 1);
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
}

#[cfg(test)]
mod pixel_review {
    use super::*;
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
                    view.input.select(view.input.caret, false);
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
