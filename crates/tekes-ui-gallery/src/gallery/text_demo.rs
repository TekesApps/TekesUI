//! Live Text and TextEditor surfaces share shaping, selection, scrolling and IME data.
use super::{PageCtx, text::TextEngine};
use tekes_ui_controls::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlKey, ReadingController,
    TextInput,
};
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
    reading: Option<ReadingController>,
    appended: usize,
    stream_remaining: u8,
    stream_elapsed: f32,
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
            reading: None,
            appended: 0,
            stream_remaining: 0,
            stream_elapsed: 0.,
        }
    }

    #[cfg(test)]
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
        let scale = text.scale();
        let type_scale = text.type_scale();
        let shape = |source: &str, width| {
            if fixture {
                text.proposal_document(source, role, width)
            } else {
                if prepared.is_none() {
                    *prepared = Some(text.prepare_proposal(source, role));
                }
                std::rc::Rc::new(text.reflow_proposal(prepared.as_ref().unwrap(), role, width))
            }
        };
        if let Some(reading) = &mut self.reading {
            reading.layout_with(
                &mut self.view,
                rect,
                ancestor_clip,
                scale,
                type_scale,
                shape,
            );
        } else {
            self.view
                .layout_with(rect, ancestor_clip, scale, type_scale, shape);
        }
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
    reading_buttons: [Button; 2],
    reading_button_focus: Option<usize>,
    motion_now: std::time::Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusStop {
    Surface(usize),
    ReadingButton(usize),
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
            reading_buttons: std::array::from_fn(|_| Button::default()),
            reading_button_focus: None,
            motion_now: std::time::Duration::ZERO,
        }
    }
}
impl TextDemo {
    fn focus_stops(&self) -> Vec<FocusStop> {
        let mut stops: Vec<_> = self
            .surfaces
            .iter()
            .enumerate()
            // Read-only surfaces still own a selection and copy interaction;
            // only disabled surfaces are removed from the keyboard sequence.
            .filter(|(_, surface)| !surface.input.disabled)
            .map(|(index, _)| FocusStop::Surface(index))
            .collect();
        if self
            .surfaces
            .first()
            .is_some_and(|surface| surface.reading.is_some())
        {
            stops.extend((0..self.reading_buttons.len()).map(FocusStop::ReadingButton));
        }
        stops
    }

    fn current_focus_stop(&self) -> Option<FocusStop> {
        self.reading_button_focus
            .map(FocusStop::ReadingButton)
            .or_else(|| self.focus.map(FocusStop::Surface))
    }

    fn focus_stop_frame(&self, stop: FocusStop) -> Option<Rect> {
        let frame = match stop {
            FocusStop::Surface(index) => self.surfaces.get(index)?.outer,
            FocusStop::ReadingButton(index) => self.reading_buttons.get(index)?.layout()?.frame,
        };
        (!frame.is_empty()).then_some(frame)
    }

    fn set_focus_stop(&mut self, stop: FocusStop) -> Option<Rect> {
        match stop {
            FocusStop::Surface(index) => {
                let surface = self.surfaces.get(index)?;
                if surface.input.disabled {
                    return None;
                }
                self.reading_button_focus = None;
                for button in &mut self.reading_buttons {
                    button.blur();
                }
                self.focus = Some(index);
                self.caret_on = true;
                self.focus_stop_frame(stop)
            }
            FocusStop::ReadingButton(index) => {
                let frame = self.reading_buttons.get(index)?.layout()?.frame;
                self.focus = None;
                self.reading_button_focus = Some(index);
                for (button_index, button) in self.reading_buttons.iter_mut().enumerate() {
                    button.state.focused = button_index == index;
                    button.state.focus_visible = button_index == index;
                }
                (!frame.is_empty()).then_some(frame)
            }
        }
    }

    /// Move keyboard focus to the first or last actual stop owned by this text demo.
    /// The returned frame is the complete, unclipped frame from the last layout.
    pub(super) fn focus_edge(&mut self, reverse: bool) -> Option<Rect> {
        let stop = if reverse {
            self.focus_stops().last().copied()
        } else {
            self.focus_stops().first().copied()
        }?;
        self.set_focus_stop(stop)
    }

    /// Complete frame for the currently focused editable surface or reading action.
    pub(super) fn focused_frame(&self) -> Option<Rect> {
        self.focus_stop_frame(self.current_focus_stop()?)
    }

    /// Clear keyboard focus while leaving the component's value and selection intact.
    pub(super) fn clear_focus(&mut self) {
        self.blur();
    }

    /// Whether the next Tab in this direction crosses the Gallery's component boundary.
    pub(super) fn tab_would_leave(&self, reverse: bool) -> bool {
        let stops = self.focus_stops();
        let Some(current) = self.current_focus_stop() else {
            return true;
        };
        let Some(index) = stops.iter().position(|stop| *stop == current) else {
            return true;
        };
        if reverse {
            index == 0
        } else {
            index + 1 == stops.len()
        }
    }

    fn advance_focus(&mut self, reverse: bool) -> Option<Rect> {
        let stops = self.focus_stops();
        let current = stops
            .iter()
            .position(|stop| Some(*stop) == self.current_focus_stop())?;
        let next = if reverse {
            current.checked_sub(1)?
        } else {
            (current + 1 < stops.len()).then_some(current + 1)?
        };
        self.set_focus_stop(stops[next])
    }

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
        demo.surfaces[0].reading = Some(ReadingController::default());
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
        surface.reading = Some(ReadingController::default());
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
        if let Some(motion) = ctx.text.motion_context() {
            self.motion_now = motion.now;
        }
        let content_clip = ancestor_clip.map_or((!content.is_empty()).then_some(content), |clip| {
            content.intersection(&clip)
        });
        // The surface frame is laid out inside `content`, while its focus ring
        // may extend into the surrounding viewport gutter. Keep this outer
        // paint clip at the true ancestor boundary; child hit/paint frames are
        // still constrained by their own geometry and `content_clip`.
        let paint_clip = ancestor_clip.or(content_clip);
        if let Some(paint_clip) = paint_clip {
            scene.push_clip(paint_clip);
        }
        let content = if self.surfaces[0].reading.is_some() {
            let gap = Spacing::Regular.value();
            let labels = ["Stream 4 updates", "Return to latest"];
            let widths = [content.size.width * 0.5 - gap * 0.5; 2];
            let sizes: Vec<_> = labels
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    Button::measure(
                        ctx.text,
                        label,
                        widths[i],
                        ButtonSize::Sm,
                        proposal::TextRole::UiBody,
                    )
                })
                .collect();
            let height = sizes.iter().map(|s| s.height).fold(0.0_f32, f32::max);
            let y = (content.max_y() - height).max(content.min_y());
            for (i, label) in labels.iter().enumerate() {
                if content_clip.is_none() {
                    self.reading_buttons[i].clear_layout();
                    continue;
                }
                self.reading_buttons[i].render(
                    scene,
                    ctx.text,
                    label,
                    Rect::new(
                        content.min_x() + i as f32 * (widths[i] + gap),
                        y,
                        widths[i].max(0.0),
                        height,
                    ),
                    paint_clip,
                    ctx.theme,
                    tekes_ui_controls::paint::ButtonVariant::Quiet,
                    ButtonSize::Sm,
                    proposal::TextRole::UiBody,
                    ButtonTextAlign::Center,
                );
            }
            Rect::new(
                content.min_x(),
                content.min_y(),
                content.size.width,
                (y - gap - content.min_y()).max(0.0),
            )
        } else {
            content
        };
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
        let mut sections = self.sections(content, ctx.text.type_scale(), &heading_heights);
        if self.surfaces.len() == 1 && self.surfaces[0].reading.is_some() {
            let line_height = ctx
                .text
                .proposal_label("Mg", self.surfaces[0].role, None)
                .height as f32;
            let minimum = line_height + 2.0 * proposal::SCROLL_CONTENT_PADDING;
            if sections[0].2.size.height < minimum {
                // The page already names this reading fixture. In a short
                // window, give its redundant heading space to the document.
                sections[0].1 = Rect::default();
                sections[0].2 = content;
            }
        }
        for (i, (name, label, panel)) in sections.into_iter().enumerate() {
            if content_clip.is_some() && !label.is_empty() {
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
        if paint_clip.is_some() {
            scene.pop_clip();
        }
    }
    fn begin_reading_stream(&mut self) {
        self.append_reading_paragraph();
        let surface = &mut self.surfaces[0];
        if surface.reading.is_some() {
            surface.stream_remaining = 3;
            surface.stream_elapsed = 0.;
        }
    }

    fn append_reading_paragraph(&mut self) {
        let surface = &mut self.surfaces[0];
        if surface.reading.is_none() {
            return;
        }
        surface.appended += 1;
        let appended = surface.appended;
        surface.input.text.push_str(&format!("\n\nUpdate {}. This paragraph is appended on request. While reading earlier lines, your place stays anchored; choose Return to latest to follow new text.", appended));
    }

    fn return_to_latest(&mut self) {
        let surface = &mut self.surfaces[0];
        if let Some(reading) = &mut surface.reading {
            reading.follow_latest(&mut surface.view);
        }
    }

    fn stop_following(&mut self) {
        let surface = &mut self.surfaces[0];
        if let Some(reading) = &mut surface.reading {
            reading.stop_following(&mut surface.view);
        }
    }

    pub fn scroll_metrics(&self) -> (f32, f32) {
        let s = self.surfaces.last().unwrap();
        (s.scroll.limit().y, s.inner.size.height)
    }
    pub fn scroll_to(&mut self, offset: f32) {
        self.stop_following();
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
            reading_buttons: std::array::from_fn(|_| Button::default()),
            reading_button_focus: None,
            motion_now: std::time::Duration::ZERO,
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
    /// State specimens use the production editor transactions, including UTF-16
    /// selection and preedit cancellation/commit behavior.
    pub(super) fn prepare_input_state_fixture(&mut self, state: &str) {
        let surface = &mut self.surfaces[0];
        match state {
            "selection" => {
                surface.input.select_range(0..6);
            }
            "composing" => {
                surface.input.select_range(0..6);
                surface.input.preedit("输入", 2..2, None);
                surface.clauses = vec![0..1, 1..2];
            }
            _ => {}
        }
    }
    /// State pages use the same visible frame and retained selection capture
    /// as live fields; no synthetic Button state is passed into input paint.
    pub(super) fn prepare_input_pointer_fixture(
        &mut self,
        text: &mut TextEngine,
        rect: Rect,
        clip: Option<Rect>,
        state: &str,
    ) {
        if !matches!(state, "hover" | "pressed") {
            return;
        }
        let surface = &mut self.surfaces[0];
        surface.layout_clipped(text, rect, clip);
        // Page captures may lay out a specimen below the viewport. Capture is
        // established on its full frame first, then the real clip is restored.
        if state == "pressed" {
            surface.layout_clipped(text, rect, None);
            let point = surface.inner.center();
            surface.pointer_down(point, false, 1, metric("size", "size-row-height"));
            surface.layout_clipped(text, rect, clip);
        }
        let pointer = surface.pointer_state(surface.visible_frame().map(|frame| frame.center()));
        debug_assert_eq!(pointer.pressed, state == "pressed");
        if state == "hover" {
            debug_assert_eq!(pointer.hovered, surface.visible_frame().is_some());
        }
        if state == "pressed" {
            self.focus_field();
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
    pub(super) fn accessibility_nodes(
        &self,
        owner: u64,
        label: &str,
    ) -> Vec<tekes_ui_platform_macos::AccessibilityNode> {
        use tekes_ui_platform_macos::{AccessibilityKey as K, AccessibilityRole as R};
        self.surfaces
            .iter()
            .enumerate()
            .filter_map(|(index, s)| {
                let frame = s.visible_frame()?;
                let mut n = super::accessibility::node(
                    K {
                        component: owner,
                        part: Some(index),
                    },
                    R::TextField,
                    label,
                    frame,
                );
                if s.input.multiline {
                    n.role = R::TextArea;
                }
                n.value = Some(s.input.text.clone());
                n.enabled = !s.input.disabled;
                n.invalid = s.error;
                n.focused = self.window_active && self.focus == Some(index);
                n.read_only = s.input.read_only;
                n.text_selection = Some(s.input.selection());
                Some(n)
            })
            .collect()
    }
    pub(super) fn accessibility_action(
        &mut self,
        index: usize,
        action: &tekes_ui_platform_macos::AccessibilityAction,
    ) -> bool {
        use tekes_ui_platform_macos::AccessibilityAction as A;
        let Some(s) = self.surfaces.get_mut(index) else {
            return false;
        };
        if s.visible_frame().is_none() || s.input.disabled || s.input.marked.is_some() {
            return false;
        }
        let changed = match action {
            A::Focus => {
                let changed = self.focus != Some(index);
                self.focus = Some(index);
                self.caret_on = true;
                return changed;
            }
            A::SetSelection(range) => {
                let before = s.input.selection();
                s.input.select_range(range.clone()) && before != s.input.selection()
            }
            A::SetValue(value) if !s.input.read_only => {
                let len = s.input.len();
                s.input.insert(value, Some(0..len))
            }
            A::ReplaceSelectedText(value) if !s.input.read_only => s.input.insert(value, None),
            _ => false,
        };
        if changed {
            s.clauses.clear();
            s.reveal();
            self.caret_on = true;
        }
        changed
    }
    pub fn state(&self) -> Option<TextInputState> {
        if !self.window_active {
            return None;
        }
        self.surface_state(self.focus?)
    }
    fn surface_state(&self, index: usize) -> Option<TextInputState> {
        let s = self.surfaces.get(index)?;
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
    pub fn set_window_active_with_time(&mut self, active: bool, now: std::time::Duration) {
        self.motion_now = now;
        self.set_window_active(active);
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        for button in &mut self.reading_buttons {
            button.set_window_active(active, self.motion_now);
        }
        self.caret_on = true;
        if !active {
            for button in &mut self.reading_buttons {
                button.blur();
            }
            self.reading_button_focus = None;
            for surface in &mut self.surfaces {
                surface.input.unmark();
                surface.cancel_interaction();
            }
            self.drag = None;
            self.scroll_target = None;
        }
    }
    pub fn blur(&mut self) {
        self.reading_button_focus = None;
        for button in &mut self.reading_buttons {
            button.cancel();
            button.state.focused = false;
            button.state.focus_visible = false;
        }
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
    pub fn set_reduced_motion_with_time(
        &mut self,
        reduced: bool,
        now: std::time::Duration,
    ) -> bool {
        self.motion_now = now;
        self.set_reduced_motion(reduced)
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        let mut dirty = false;
        for surface in &mut self.surfaces {
            dirty |= surface.scroll.set_reduced_motion(reduced);
        }
        for button in &mut self.reading_buttons {
            dirty |= button.set_reduced_motion(reduced, self.motion_now);
        }
        dirty
    }
    pub fn needs_animation(&self) -> bool {
        self.surfaces.iter().any(|s| s.scroll.needs_animation())
            || (self.window_active && self.surfaces[0].stream_remaining > 0)
            || self.reading_buttons.iter().any(Button::needs_animation)
    }
    pub fn animate(&mut self, dt: f32) -> bool {
        let mut changed = self.reading_buttons.iter().any(Button::needs_animation);
        for surface in &mut self.surfaces {
            changed |= surface.scroll.tick(dt);
        }
        // Bounded deterministic consumer fixture, not a component timer or
        // performance sample. Only an explicit stream action schedules updates.
        let mut chunks = 0;
        if self.window_active && dt.is_finite() && dt > 0. {
            let surface = &mut self.surfaces[0];
            surface.stream_elapsed += dt;
            while surface.stream_remaining > 0 && surface.stream_elapsed >= 0.75 {
                surface.stream_elapsed -= 0.75;
                surface.stream_remaining -= 1;
                chunks += 1;
            }
        }
        for _ in 0..chunks {
            self.append_reading_paragraph();
        }
        changed || chunks > 0
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
    fn handle_reading_actions(&mut self, event: &Event, point: Option<Point>) -> Option<bool> {
        self.surfaces[0].reading.as_ref()?;
        if !self.window_active {
            return None;
        }
        let mut activated = None;
        let handled = match event {
            Event::Mouse(mouse) => {
                let p = point.unwrap_or(Point::new(f32::NAN, f32::NAN));
                match mouse.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered | MouseEventKind::Exited => {
                        let mut changed = false;
                        for button in &mut self.reading_buttons {
                            let before = button.state.hovered;
                            button.pointer_move_at(p);
                            changed |= before != button.state.hovered;
                        }
                        return changed.then_some(true);
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let index = self
                            .reading_buttons
                            .iter()
                            .position(|b| b.layout().is_some_and(|l| l.hit_test(p)))?;
                        self.focus = None;
                        self.reading_button_focus = Some(index);
                        for (i, button) in self.reading_buttons.iter_mut().enumerate() {
                            button.state.focused = i == index;
                            button.state.focus_visible = false;
                        }
                        self.reading_buttons[index].pointer_down_at(p)
                    }
                    MouseEventKind::Dragged {
                        button: MouseButton::Left,
                    } if self.reading_buttons.iter().any(|b| b.state.pressed) => {
                        for button in &mut self.reading_buttons {
                            button.pointer_move_at(p);
                        }
                        true
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } if self.reading_buttons.iter().any(|b| b.state.pressed) => {
                        for (i, button) in self.reading_buttons.iter_mut().enumerate() {
                            if button.pointer_up_at(p) {
                                activated = Some(i);
                            }
                        }
                        true
                    }
                    _ => return None,
                }
            }
            Event::TextCommand(command)
                if self.reading_button_focus.is_some()
                    && matches!(command.as_str(), "insertNewline:" | "insertLineBreak:") =>
            {
                let index = self.reading_button_focus.unwrap();
                if self.reading_buttons[index].key(ControlKey::Enter, true) {
                    activated = Some(index);
                }
                true
            }
            Event::Key(key) if self.reading_button_focus.is_some() => {
                let control = match key.key_code {
                    36 | 76 => ControlKey::Enter,
                    49 => ControlKey::Space,
                    _ => return None,
                };
                let index = self.reading_button_focus.unwrap();
                if self.reading_buttons[index].key(control, key.kind == KeyEventKind::Down) {
                    activated = Some(index);
                }
                true
            }
            _ => return None,
        };
        match activated {
            Some(0) => self.begin_reading_stream(),
            Some(1) => self.return_to_latest(),
            _ => {}
        }
        Some(handled)
    }

    pub fn handle(&mut self, event: &Event, point: Option<Point>) -> Option<bool> {
        if let Some(changed) = self.handle_reading_actions(event, point) {
            self.reveal_pending = false;
            return Some(changed);
        }
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
                    if self.surfaces[0].contains_visible(point) {
                        self.stop_following();
                    }
                    let index = self
                        .surfaces
                        .iter()
                        .position(|s| !s.input.disabled && s.contains_visible(point));
                    // The text body owns focus after this click. An old
                    // reading action must not keep handling Space or Return.
                    self.reading_button_focus = None;
                    for button in &mut self.reading_buttons {
                        button.blur();
                    }
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
                // Native positive delta.y expresses upward reading intent. Stop
                // following before applying the event, even at the top edge.
                if i == 0 {
                    let surface = &mut self.surfaces[i];
                    if let Some(reading) = &mut surface.reading {
                        reading.observe_scroll(
                            &mut surface.view,
                            Point::new(e.delta.x as f32, e.delta.y as f32),
                            e.modifiers.shift,
                        );
                    }
                }
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
        if matches!(
            command,
            "moveUp:"
                | "moveUpAndModifySelection:"
                | "moveToBeginningOfDocument:"
                | "moveToBeginningOfDocumentAndModifySelection:"
                | "pageUp:"
                | "scrollPageUp:"
        ) {
            self.stop_following();
        }
        if self.surfaces[0].reading.is_some() && matches!(command, "insertTab:" | "insertBacktab:")
        {
            let backwards = command == "insertBacktab:";
            if self.tab_would_leave(backwards) {
                return None;
            }
            self.advance_focus(backwards);
            return Some(true);
        }
        if matches!(command, "insertTab:" | "insertBacktab:") {
            let backwards = command == "insertBacktab:";
            if self.tab_would_leave(backwards) {
                return None;
            }
            if let Some(index) = self.focus {
                self.surfaces[index].input.unmark();
            }
            self.advance_focus(backwards);
            self.caret_on = true;
            self.reveal_pending = true;
            return Some(true);
        }
        let i = self.focus?;
        self.caret_on = true;
        self.reveal_pending = true;
        let s = &mut self.surfaces[i];
        match command {
            "pageDown:" | "pageUp:" | "scrollPageDown:" | "scrollPageUp:" => {
                self.reveal_pending = false;
                let sign = if command.contains("Up") { -1. } else { 1. };
                let amount = s.inner.size.height * 0.9 * sign;
                s.scroll.scroll_by(0., amount);
            }
            _ => {
                if s.view.command(command).is_none() {
                    return Some(false);
                }
            }
        }
        Some(true)
    }
    pub fn refresh(&mut self, text: &mut TextEngine) {
        for s in &mut self.surfaces {
            if s.outer.size.width > 0.0 {
                let rect = s.outer;
                let clip = s.view.ancestor_clip();
                s.layout_clipped(text, rect, clip);
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
    #[test]
    fn scheduled_reading_updates_follow_explicit_intent_and_native_return_action() {
        let mut demo = TextDemo::scroll_only();
        let mut text = TextEngine::new();
        reading_layout(&mut demo, &mut text, 400.);
        demo.focus_edge(true);
        assert_eq!(
            demo.handle(&Event::TextCommand("insertNewline:".into()), None),
            Some(true)
        );
        assert!(
            demo.surfaces[0]
                .reading
                .as_ref()
                .unwrap()
                .is_following_latest()
        );
        demo.begin_reading_stream();
        assert!(demo.needs_animation());
        assert!(demo.animate(0.8));
        reading_layout(&mut demo, &mut text, 400.);
        assert_eq!(demo.surfaces[0].appended, 2);
        assert_eq!(
            demo.surfaces[0].scroll.offset.y,
            demo.surfaces[0].scroll.limit().y
        );
        demo.scroll_to(713.25);
        reading_layout(&mut demo, &mut text, 400.);
        let anchor = demo.surfaces[0].reflow_anchor.unwrap();
        demo.animate(1.6);
        reading_layout(&mut demo, &mut text, 280.);
        assert_eq!(demo.surfaces[0].appended, 4);
        assert_eq!(demo.surfaces[0].stream_remaining, 0);
        assert_eq!(demo.surfaces[0].reflow_anchor.unwrap().utf16, anchor.utf16);
        assert!(
            !demo.surfaces[0]
                .reading
                .as_ref()
                .unwrap()
                .is_following_latest()
        );
        assert_eq!(demo.reading_button_focus, Some(1));
        demo.handle(&Event::TextCommand("insertLineBreak:".into()), None);
        reading_layout(&mut demo, &mut text, 400.);
        assert_eq!(demo.focus, None);
        assert_eq!(demo.reading_button_focus, Some(1));
        assert_eq!(
            demo.surfaces[0].scroll.offset.y,
            demo.surfaces[0].scroll.limit().y
        );
    }
    #[test]
    fn input_pointer_specimens_use_real_capture_and_keep_hover_unfocused() {
        for multiline in [false, true] {
            let mut text = TextEngine::new();
            let frame = Rect::new(20., 20., 260., 100.);
            let mut hover = TextDemo::field("Sample text", multiline, false);
            hover.prepare_input_pointer_fixture(&mut text, frame, None, "hover");
            assert!(
                hover.surfaces[0]
                    .pointer_state(Some(frame.center()))
                    .hovered
            );
            assert!(!hover.surfaces[0].pointer_state(None).pressed);
            assert!(hover.focus.is_none());
            let mut pressed = TextDemo::field("Sample text", multiline, false);
            pressed.prepare_input_pointer_fixture(&mut text, frame, None, "pressed");
            assert!(pressed.surfaces[0].pointer_state(None).pressed);
            assert_eq!(pressed.focus, Some(0));
            pressed.blur();
            assert!(!pressed.surfaces[0].pointer_state(None).pressed);
        }
    }
    #[test]
    fn input_family_state_specimens_use_editor_transactions_and_separate_disabled_from_read_only() {
        for multiline in [false, true] {
            let mut selection = super::TextDemo::field("Sample text", multiline, false);
            selection.prepare_input_state_fixture("selection");
            assert_eq!(selection.surfaces[0].input.selected_text(), "Sample");
            let mut composing = super::TextDemo::field("Sample text", multiline, false);
            composing.prepare_input_state_fixture("composing");
            assert_eq!(composing.surfaces[0].input.text, "输入 text");
            assert_eq!(composing.surfaces[0].input.marked, Some(0..2));
            assert_eq!(composing.surfaces[0].clauses, [0..1, 1..2]);
            let mut read_only = super::TextDemo::field("Sample text", multiline, true);
            read_only.focus_field();
            assert!(!read_only.surfaces[0].input.insert("x", None));
            assert!(read_only.surfaces[0].input.select_range(0..6));
            assert!(read_only.focus.is_some());
            let mut disabled = super::TextDemo::field("Sample text", multiline, false);
            disabled.set_disabled(true);
            disabled.focus_field();
            assert!(!disabled.surfaces[0].input.read_only);
            assert!(disabled.focus.is_none());
            assert!(!disabled.surfaces[0].input.insert("x", None));
        }
    }
    use super::*;
    use crate::gallery::static_text::StaticText;
    fn reading_layout(demo: &mut TextDemo, text: &mut TextEngine, width: f32) {
        demo.build(
            &mut PageCtx {
                theme: Theme::LIGHT,
                text,
                static_text: &mut StaticText::new(None),
            },
            Rect::new(20.0, 20.0, width, 300.0),
            &mut Scene::new(),
        );
    }

    #[test]
    fn reading_append_and_reflow_preserve_logical_anchor_and_selection() {
        for mut demo in [TextDemo::text_only(), TextDemo::scroll_only()] {
            let mut text = TextEngine::new();
            reading_layout(&mut demo, &mut text, 640.0);
            demo.scroll_to(713.25);
            demo.surfaces[0].input.anchor = 12;
            demo.surfaces[0].input.caret = 120;
            reading_layout(&mut demo, &mut text, 640.0);
            let initial = demo.surfaces[0].reflow_anchor.unwrap();
            demo.append_reading_paragraph();
            reading_layout(&mut demo, &mut text, 320.0);
            let surface = &demo.surfaces[0];
            let after = surface.reflow_anchor.unwrap();
            assert_eq!(after.utf16, initial.utf16);
            assert_eq!(after.within_line, initial.within_line);
            let expected = surface.caret_doc(initial.utf16).min_y() + initial.within_line;
            assert!((surface.scroll.offset.y - expected).abs() < 0.001);
            assert_eq!((surface.input.anchor, surface.input.caret), (12, 120));
            reading_layout(&mut demo, &mut text, 640.0);
            assert!((demo.surfaces[0].scroll.offset.y - initial.applied_y).abs() < 0.001);
        }
    }

    #[test]
    fn reading_only_explicit_latest_repins_and_upward_intent_cancels_before_append() {
        use tekes_ui_platform_macos::{ScrollEvent, Vec2};
        for reduced in [false, true] {
            let mut demo = TextDemo::scroll_only();
            let mut text = TextEngine::new();
            demo.set_reduced_motion(reduced);
            reading_layout(&mut demo, &mut text, 400.0);
            demo.return_to_latest();
            demo.append_reading_paragraph();
            reading_layout(&mut demo, &mut text, 320.0);
            assert_eq!(
                demo.surfaces[0].scroll.offset.y,
                demo.surfaces[0].scroll.limit().y
            );
            let point = demo.surfaces[0].inner.center();
            let event = Event::Scroll(ScrollEvent {
                timestamp: 1.0,
                delta: Vec2 { x: 0.0, y: 80.0 },
                precise: true,
                phase: ScrollPhase::Began,
                momentum_phase: ScrollPhase::None,
                inverted: false,
                position: tekes_ui_platform_macos::Point {
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                },
                modifiers: Default::default(),
            });
            assert!(demo.handle(&event, Some(point)).is_some());
            assert!(
                !demo.surfaces[0]
                    .reading
                    .as_ref()
                    .unwrap()
                    .is_following_latest()
            );
            let history_offset = demo.surfaces[0].scroll.offset.y;
            // Append lands before another rendered frame; do not steal this gesture.
            demo.append_reading_paragraph();
            reading_layout(&mut demo, &mut text, 320.0);
            assert!((demo.surfaces[0].scroll.offset.y - history_offset).abs() < 0.001);
            // Even manually returning exactly to the bottom does not re-enable follow.
            demo.scroll_to(f32::MAX);
            let manual_bottom = demo.surfaces[0].scroll.offset.y;
            demo.append_reading_paragraph();
            reading_layout(&mut demo, &mut text, 320.0);
            assert!((demo.surfaces[0].scroll.offset.y - manual_bottom).abs() < 0.001);
            assert!(demo.surfaces[0].scroll.offset.y < demo.surfaces[0].scroll.limit().y);
            demo.return_to_latest();
            reading_layout(&mut demo, &mut text, 500.0);
            assert_eq!(
                demo.surfaces[0].scroll.offset.y,
                demo.surfaces[0].scroll.limit().y
            );
            assert!(!demo.surfaces[0].scroll.needs_animation());
        }
    }

    #[test]
    fn reading_fixture_uses_real_buttons_release_inside_and_keyboard_actions() {
        let mut demo = TextDemo::text_only();
        let mut text = TextEngine::new();
        reading_layout(&mut demo, &mut text, 400.0);
        let initial = demo.query().to_owned();
        let append = demo.reading_buttons[0].layout().unwrap().frame.center();
        let latest = demo.reading_buttons[1].layout().unwrap().frame.center();
        let pointer = |kind, p: Point| {
            Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                kind,
                position: tekes_ui_platform_macos::Point {
                    x: f64::from(p.x),
                    y: f64::from(p.y),
                },
                modifiers: Default::default(),
            })
        };
        let down = MouseEventKind::Down {
            button: MouseButton::Left,
            click_count: 1,
        };
        let up = MouseEventKind::Up {
            button: MouseButton::Left,
            click_count: 1,
        };
        demo.handle(&pointer(down, append), Some(append));
        demo.handle(&pointer(up, latest), Some(latest));
        assert_eq!(demo.query(), initial);
        demo.handle(&pointer(down, append), Some(append));
        demo.handle(&pointer(up, append), Some(append));
        assert!(demo.query().contains("Update 1."));
        demo.command("insertTab:");
        assert_eq!(demo.reading_button_focus, Some(1));
        let enter = Event::Key(tekes_ui_platform_macos::KeyEvent {
            kind: KeyEventKind::Down,
            key_code: 36,
            modifiers: Default::default(),
            is_repeat: false,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        });
        assert_eq!(demo.handle(&enter, None), Some(true));
        demo.refresh(&mut text);
        assert_eq!(
            demo.surfaces[0].scroll.offset.y,
            demo.surfaces[0].scroll.limit().y
        );
        assert!(
            demo.surfaces[0]
                .reading
                .as_ref()
                .unwrap()
                .is_following_latest()
        );
        demo.handle(&pointer(down, append), Some(append));
        demo.set_window_active(false);
        demo.set_window_active(true);
        demo.handle(&pointer(up, append), Some(append));
        assert!(!demo.query().contains("Update 2."));
    }

    #[test]
    fn gallery_focus_edges_use_real_frames_and_text_tab_bubbles_without_wrapping() {
        let mut text = TextEngine::new();
        let mut editor = TextDemo::editors_only();
        reading_layout(&mut editor, &mut text, 420.0);

        assert!(
            editor.tab_would_leave(false),
            "unfocused leaf yields to Gallery"
        );
        let first = editor
            .focus_edge(false)
            .expect("first laid out editor frame");
        assert_eq!(editor.focused_frame(), Some(first));
        assert_eq!(editor.focus, Some(0));
        assert!(!editor.tab_would_leave(false));
        assert_eq!(editor.command("insertTab:"), Some(true));
        assert_eq!(editor.focus, Some(1));
        let last = editor.focused_frame().expect("last editor frame");
        assert_eq!(last, editor.frame_at(1).unwrap());
        assert!(editor.tab_would_leave(false));
        assert_eq!(editor.command("insertTab:"), None, "forward edge bubbles");
        assert_eq!(
            editor.focus,
            Some(1),
            "edge bubbling preserves current focus"
        );

        assert!(!editor.tab_would_leave(true));
        assert_eq!(editor.command("insertBacktab:"), Some(true));
        assert_eq!(editor.focus, Some(0));
        assert!(editor.tab_would_leave(true));
        assert_eq!(
            editor.command("insertBacktab:"),
            None,
            "reverse edge bubbles"
        );
        assert_eq!(editor.focus, Some(0));

        editor.clear_focus();
        assert_eq!(editor.focused_frame(), None);
        assert_eq!(editor.focus, None);
    }

    #[test]
    fn focused_text_surface_ring_uses_viewport_gutter_without_expanding_frame() {
        let content = Rect::new(100.0, 80.0, 500.0, 500.0);
        let viewport = Rect::new(84.0, 64.0, 532.0, 532.0);
        fn assert_real_ring(
            mut demo: TextDemo,
            text: &mut TextEngine,
            content: Rect,
            viewport: Rect,
            advance: bool,
        ) {
            use tekes_ui_scene::Primitive;

            demo.build_clipped(
                &mut PageCtx {
                    theme: Theme::LIGHT,
                    text,
                    static_text: &mut StaticText::new(None),
                },
                content,
                Some(viewport),
                &mut Scene::new(),
            );
            demo.focus_edge(false).expect("first TextSurface frame");
            if advance {
                assert_eq!(demo.command("insertTab:"), Some(true));
            }
            let surface_index = demo.focus.expect("TextSurface keyboard focus");
            let outer = demo.focused_frame().expect("focused surface frame");
            assert!(content.contains(outer.center()));

            let mut scene = Scene::new();
            demo.build_clipped(
                &mut PageCtx {
                    theme: Theme::LIGHT,
                    text,
                    static_text: &mut StaticText::new(None),
                },
                content,
                Some(viewport),
                &mut scene,
            );
            assert_eq!(
                scene.primitives().first(),
                Some(&Primitive::PushClip(viewport)),
                "the parent viewport bounds paint while the layout frame remains within content"
            );
            let focused_border = proposal::render_color("border-focused", Theme::LIGHT).unwrap();
            let ring = scene
                .primitives()
                .iter()
                .find_map(|primitive| match primitive {
                    Primitive::Quad(quad) if quad.border_color == focused_border => Some(quad.rect),
                    _ => None,
                })
                .expect("real focused TextSurface ring primitive");
            assert!(ring.min_x() < outer.min_x());
            assert!(ring.max_x() > outer.max_x());
            assert!(ring.min_y() < outer.min_y());
            assert!(ring.max_y() > outer.max_y());
            assert!(viewport.contains(Point::new(ring.min_x(), ring.min_y())));
            assert!(viewport.contains(Point::new(ring.max_x(), ring.max_y())));
            assert_eq!(
                demo.focused_frame(),
                Some(outer),
                "focus paint does not resize layout"
            );
            assert_eq!(demo.visible_frame_at(surface_index), Some(outer));
        }

        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        assert_real_ring(TextDemo::editors_only(), &mut text, content, viewport, true);
        assert_real_ring(TextDemo::text_only(), &mut text, content, viewport, false);
        assert_real_ring(TextDemo::scroll_only(), &mut text, content, viewport, false);
    }

    #[test]
    fn reading_actions_are_real_leaf_stops_for_text_and_scroll_pages() {
        let mut text = TextEngine::new();
        for mut demo in [TextDemo::text_only(), TextDemo::scroll_only()] {
            reading_layout(&mut demo, &mut text, 440.0);
            assert!(demo.tab_would_leave(false));
            let body = demo.focus_edge(false).expect("selectable text frame");
            assert_eq!(Some(body), demo.frame_at(0));
            assert_eq!(demo.focus, Some(0));
            assert!(demo.state().unwrap().read_only);
            assert_eq!(demo.command("selectAll:"), Some(true));
            assert_eq!(demo.clipboard_selection().as_deref(), Some(demo.query()));
            assert!(!demo.tab_would_leave(false));
            assert_eq!(demo.command("insertTab:"), Some(true));
            assert_eq!(demo.reading_button_focus, Some(0));
            assert!(!demo.tab_would_leave(false));
            assert_eq!(demo.command("insertTab:"), Some(true));
            assert_eq!(demo.reading_button_focus, Some(1));
            let last = demo.focused_frame().expect("second real button frame");
            assert_eq!(
                Some(last),
                demo.reading_buttons[1].layout().map(|l| l.frame)
            );
            assert!(demo.tab_would_leave(false));
            assert_eq!(demo.command("insertTab:"), None);
            assert_eq!(demo.reading_button_focus, Some(1));
            assert!(demo.tab_would_leave(false));

            assert!(!demo.tab_would_leave(true));
            assert_eq!(demo.command("insertBacktab:"), Some(true));
            assert_eq!(demo.reading_button_focus, Some(0));
            assert!(!demo.tab_would_leave(true));
            assert_eq!(demo.command("insertBacktab:"), Some(true));
            assert_eq!(demo.focus, Some(0));
            assert_eq!(demo.reading_button_focus, None);
            assert!(demo.tab_would_leave(true));
            assert_eq!(demo.command("insertBacktab:"), None);
            assert_eq!(demo.focus, Some(0));
            demo.clear_focus();
            assert_eq!(demo.focused_frame(), None);
        }
    }

    #[test]
    fn reading_body_click_clears_latest_action_keyboard_focus() {
        let mut demo = TextDemo::scroll_only();
        let mut text = TextEngine::new();
        reading_layout(&mut demo, &mut text, 500.);
        let latest = demo.reading_buttons[1].layout().unwrap().frame.center();
        let pointer = |kind, p: Point| {
            Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                kind,
                position: tekes_ui_platform_macos::Point {
                    x: f64::from(p.x),
                    y: f64::from(p.y),
                },
                modifiers: Default::default(),
            })
        };
        let down = MouseEventKind::Down {
            button: MouseButton::Left,
            click_count: 1,
        };
        let up = MouseEventKind::Up {
            button: MouseButton::Left,
            click_count: 1,
        };
        demo.handle(&pointer(down, latest), Some(latest));
        demo.handle(&pointer(up, latest), Some(latest));
        assert_eq!(demo.reading_button_focus, Some(1));
        demo.refresh(&mut text);
        demo.scroll_to(80.);
        let body = demo.surfaces[0].inner.center();
        demo.handle(&pointer(down, body), Some(body));
        demo.handle(&pointer(up, body), Some(body));
        assert_eq!(demo.focus, Some(0));
        assert_eq!(demo.reading_button_focus, None);
        assert!(
            demo.reading_buttons
                .iter()
                .all(|b| !b.state.focused && !b.state.focus_visible)
        );
        let history_offset = demo.surfaces[0].scroll.offset.y;
        for code in [49, 36] {
            for kind in [KeyEventKind::Down, KeyEventKind::Up] {
                demo.handle(
                    &Event::Key(tekes_ui_platform_macos::KeyEvent {
                        kind,
                        key_code: code,
                        modifiers: Default::default(),
                        is_repeat: false,
                        characters: String::new(),
                        characters_ignoring_modifiers: String::new(),
                    }),
                    None,
                );
            }
        }
        demo.append_reading_paragraph();
        demo.refresh(&mut text);
        assert!(
            !demo.surfaces[0]
                .reading
                .as_ref()
                .unwrap()
                .is_following_latest()
        );
        assert!((demo.surfaces[0].scroll.offset.y - history_offset).abs() < 0.01);
    }

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
            Rect::new(20.0, 30.0, 280.0, 220.0),
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
        assert!(
            surface.inner.contains(point),
            "caret {after:?}, inner {:?}",
            surface.inner
        );
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
    fn reading_controller_stream_history_and_explicit_latest_render_on_gpu() {
        let mut renderer = tekes_ui_render::Renderer::new_headless().expect("actual GPU required");
        let output = std::env::var_os("TEKESUI_READING_OUT").map(std::path::PathBuf::from);
        if let Some(path) = &output {
            std::fs::create_dir_all(path).unwrap();
        }
        let mut manifest = Vec::new();
        for theme in Theme::ALL {
            for scale in [1, 2] {
                for (width, type_scale) in [(720_u32, 1.), (480, 1.5)] {
                    for kind in ["text", "scroll-area"] {
                        let mut text = TextEngine::new();
                        text.set_scale(scale as f32);
                        let mut demo = if kind == "text" {
                            TextDemo::text_only()
                        } else {
                            TextDemo::scroll_only()
                        };
                        demo.set_reduced_motion(true);
                        demo.surfaces[0].input.select_range(12..120);
                        let selected = demo.surfaces[0].input.selection();
                        let mut history = None;
                        for stage in ["initial", "following", "history", "append-reflow", "latest"]
                        {
                            match stage {
                                "following" => {
                                    demo.return_to_latest();
                                    demo.begin_reading_stream();
                                    demo.animate(0.8);
                                }
                                "history" => demo.scroll_to(713.25),
                                "append-reflow" => {
                                    demo.animate(1.6);
                                }
                                "latest" => {
                                    demo.focus_edge(true);
                                    demo.handle(&Event::TextCommand("insertNewline:".into()), None);
                                }
                                _ => {}
                            }
                            let reflow = stage == "append-reflow";
                            text.set_type_scale(if reflow {
                                type_scale * 1.25
                            } else {
                                type_scale
                            });
                            text.begin_frame();
                            let mut static_text =
                                crate::gallery::static_text::StaticText::new(None);
                            let mut ctx = PageCtx {
                                theme,
                                text: &mut text,
                                static_text: &mut static_text,
                            };
                            let mut scene = Scene::new();
                            let frame = Rect::new(
                                20.,
                                20.,
                                width as f32 * if reflow { 0.65 } else { 1. },
                                360.,
                            );
                            // Match the Gallery host: content has a gutter inside
                            // the ancestor viewport, including focus/AA paint.
                            demo.build_clipped(
                                &mut ctx,
                                frame,
                                Some(Rect::new(0., 0., (width + 40) as f32, 400.)),
                                &mut scene,
                            );
                            let surface = &demo.surfaces[0];
                            let following = surface.reading.as_ref().unwrap().is_following_latest();
                            if matches!(stage, "following" | "latest") {
                                assert!(following);
                                assert_eq!(surface.scroll.offset.y, surface.scroll.limit().y);
                            }
                            if stage == "history" {
                                history = surface.reflow_anchor;
                            }
                            if stage == "append-reflow" {
                                assert!(!following);
                                let before = history.unwrap();
                                let after = surface.reflow_anchor.unwrap();
                                assert_eq!(after.utf16, before.utf16);
                                assert_eq!(after.within_line, before.within_line);
                                assert_eq!(surface.appended, 4);
                            }
                            assert_eq!(surface.input.selection(), selected);
                            let size = ((width + 40) * scale, 400 * scale);
                            let rgba = renderer
                                .render_to_rgba8(
                                    &scene,
                                    text.atlas_mut(),
                                    size,
                                    scale as f32,
                                    ink("document-canvas", theme),
                                )
                                .unwrap();
                            if stage == "latest" {
                                let button = demo.reading_buttons[1].layout().unwrap().frame;
                                let x = ((button.max_x() + 3.) * scale as f32) as usize;
                                let y = (button.center().y * scale as f32) as usize;
                                let pixel = (y * size.0 as usize + x) * 4;
                                assert_ne!(
                                    &rgba[pixel..pixel + 4],
                                    &rgba[..4],
                                    "reading action focus ring must extend into the host gutter"
                                );
                            }
                            let name = format!(
                                "{kind}-{}-{width}-{stage}@{scale}x.png",
                                crate::gallery::theme::theme_slug(theme)
                            );
                            if let Some(path) = &output {
                                let mut encoder = png::Encoder::new(
                                    std::fs::File::create(path.join(&name)).unwrap(),
                                    size.0,
                                    size.1,
                                );
                                encoder.set_color(png::ColorType::Rgba);
                                encoder.set_depth(png::BitDepth::Eight);
                                encoder
                                    .write_header()
                                    .unwrap()
                                    .write_image_data(&rgba)
                                    .unwrap();
                            }
                            manifest.push(serde_json::json!({"file":name,"kind":kind,"stage":stage,"theme":format!("{theme:?}"),"backing_scale":scale,"width":width,"frame_width":frame.size.width,"type_scale":text.type_scale(),"following":following,"offset":surface.scroll.offset.y,"range":surface.scroll.limit().y,"anchor":surface.reflow_anchor.map(|a|a.utf16),"selection_preserved":true,"actual_gpu":true,"native_acceptance":false}));
                        }
                    }
                }
            }
        }
        assert_eq!(manifest.len(), 160);
        if let Some(path) = output {
            std::fs::write(
                path.join("manifest.json"),
                serde_json::to_vec_pretty(&manifest).unwrap(),
            )
            .unwrap();
        }
    }

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
                    // A host may retain focus during a state update. Disabled
                    // and read-only surfaces must render identical pixels for
                    // both blink phases, without clearing their draft.
                    for mode in ["disabled", "read-only"] {
                        view.input.disabled = mode == "disabled";
                        view.input.read_only = mode == "read-only";
                        let mut images = Vec::new();
                        for caret_on in [false, true] {
                            let mut scene = Scene::new();
                            view.draw(&mut text, &mut scene, theme, true, caret_on);
                            images.push(
                                renderer
                                    .render_to_rgba8(
                                        &scene,
                                        text.atlas_mut(),
                                        size,
                                        scale as f32,
                                        ink("document-canvas", theme),
                                    )
                                    .unwrap(),
                            );
                        }
                        assert_eq!(
                            images[0], images[1],
                            "{mode} must suppress the insertion caret"
                        );
                        assert_eq!(view.input.text, value);
                        if let Some(dir) = &output {
                            let path = dir.join(format!(
                                "{}-{mode}-{}@{}x.png",
                                if multiline { "multiline" } else { "single" },
                                super::super::theme::theme_slug(theme),
                                scale
                            ));
                            let mut encoder = png::Encoder::new(
                                std::fs::File::create(path).unwrap(),
                                size.0,
                                size.1,
                            );
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            encoder
                                .write_header()
                                .unwrap()
                                .write_image_data(&images[0])
                                .unwrap();
                        }
                    }
                    view.input.disabled = false;
                    view.input.read_only = false;
                    for mode in ["hover", "pressed"] {
                        let point = view.inner.center();
                        if mode == "pressed" {
                            view.pointer_down(point, false, 1, 24.);
                        }
                        let state = view.pointer_state(Some(point));
                        assert!(state.hovered);
                        assert_eq!(state.pressed, mode == "pressed");
                        let mut active = Scene::new();
                        view.draw(&mut text, &mut active, theme, mode == "pressed", true);
                        let active = renderer
                            .render_to_rgba8(
                                &active,
                                text.atlas_mut(),
                                size,
                                scale as f32,
                                ink("document-canvas", theme),
                            )
                            .unwrap();
                        let selection = view.input.selection();
                        view.pointer_up();
                        assert!(!view.pointer_state(None).pressed);
                        let mut released = Scene::new();
                        view.draw(&mut text, &mut released, theme, mode == "pressed", true);
                        let released = renderer
                            .render_to_rgba8(
                                &released,
                                text.atlas_mut(),
                                size,
                                scale as f32,
                                ink("document-canvas", theme),
                            )
                            .unwrap();
                        assert_eq!(active, released, "hover/pressed preserve input paint");
                        assert_eq!(view.input.selection(), selection);
                        assert_eq!(view.input.text, value);
                        if let Some(dir) = &output {
                            let path = dir.join(format!(
                                "{}-{mode}-{}@{}x.png",
                                if multiline { "multiline" } else { "single" },
                                super::super::theme::theme_slug(theme),
                                scale
                            ));
                            let mut encoder = png::Encoder::new(
                                std::fs::File::create(path).unwrap(),
                                size.0,
                                size.1,
                            );
                            encoder.set_color(png::ColorType::Rgba);
                            encoder.set_depth(png::BitDepth::Eight);
                            encoder
                                .write_header()
                                .unwrap()
                                .write_image_data(&active)
                                .unwrap();
                        }
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

#[cfg(test)]
mod native_semantics_tests {
    use super::*;
    #[test]
    fn readonly_text_keeps_public_native_selection_but_rejects_both_edit_forms() {
        use tekes_ui_platform_macos::{AccessibilityAction as A, AccessibilityRole as R};
        let mut demo = TextDemo::field("中文 😀 e\u{301}", true, true);
        let mut text = TextEngine::new();
        let frame = Rect::new(30., 40., 400., 150.);
        demo.surfaces[0].layout_clipped(&mut text, frame, Some(frame));
        let n = demo.accessibility_nodes(901, "Reading").remove(0);
        assert_eq!(n.role, R::TextArea);
        assert!(n.read_only);
        assert!(demo.accessibility_action(0, &A::SetSelection(3..5)));
        let before = format!("{:?}", demo.surfaces[0].input);
        assert!(!demo.accessibility_action(0, &A::SetValue("replace".into())));
        assert!(!demo.accessibility_action(0, &A::ReplaceSelectedText("replace".into())));
        assert_eq!(format!("{:?}", demo.surfaces[0].input), before);
        assert_eq!(
            demo.accessibility_nodes(901, "Reading")[0].text_selection,
            Some(3..5)
        );
    }
}
