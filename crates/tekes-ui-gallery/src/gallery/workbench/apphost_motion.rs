//! The original Motion laboratory, retained inside AppHost's single ContentHost.
use super::super::{
    locale::Locale,
    motion_preview::{MotionPreview, Parameter},
    pages::motion::{self, Action, Layout, MotionText},
};
use super::Entry;
use anyhow::Result;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
    time::Duration,
};
use tekes_ui_app_host::ContainerRenderContext;
use tekes_ui_controls::{
    ButtonTextRenderer, ControlMotionContext, composition::*, layout::MeasuredChild,
};
use tekes_ui_platform_macos as native;
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::Paragraph;
use tekes_ui_tokens::{Rgba, proposal};

const ROOT: ComponentId = ComponentId(97000);
pub(crate) const SPECIMEN: ComponentId = ComponentId(97020);

type ParagraphKey = (String, String, Option<u32>);
#[derive(Debug, Default)]
struct ParagraphCache {
    type_scale: Option<u32>,
    paragraphs: HashMap<ParagraphKey, Rc<Paragraph>>,
    order: VecDeque<ParagraphKey>,
}
impl ParagraphCache {
    fn insert(&mut self, key: ParagraphKey, paragraph: Rc<Paragraph>) -> Rc<Paragraph> {
        // Match the original Gallery cache bound: repeated window resizing and
        // diagnostic labels must not retain an unlimited set of shaped widths.
        if self.paragraphs.len() >= 4096
            && let Some(old) = self.order.pop_front()
        {
            self.paragraphs.remove(&old);
        }
        self.order.push_back(key.clone());
        self.paragraphs.insert(key, paragraph.clone());
        paragraph
    }
}
struct HostText<'a> {
    text: &'a mut dyn ButtonTextRenderer,
    cache: &'a mut ParagraphCache,
    clip: Rect,
    scale: f32,
}
impl<'a> HostText<'a> {
    fn new(
        text: &'a mut dyn ButtonTextRenderer,
        cache: &'a mut ParagraphCache,
        clip: Rect,
    ) -> Self {
        let scale = text
            .text_surface_host()
            .expect("AppHost text service")
            .type_scale();
        if cache.type_scale != Some(scale.to_bits()) {
            cache.paragraphs.clear();
            cache.order.clear();
            cache.type_scale = Some(scale.to_bits());
        }
        Self {
            text,
            cache,
            clip,
            scale,
        }
    }
    fn paragraph(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
        localized: bool,
    ) -> Rc<Paragraph> {
        let value = if localized {
            Locale::default().text(value).into_owned()
        } else {
            value.to_owned()
        };
        let key = (value.clone(), role.id().to_owned(), width.map(f32::to_bits));
        if let Some(paragraph) = self.cache.paragraphs.get(&key) {
            return paragraph.clone();
        }
        let paragraph = self
            .text
            .text_surface_host()
            .expect("AppHost text service")
            .shape_surface(&value, role, width);
        self.cache.insert(key, paragraph)
    }
}
impl MotionText for HostText<'_> {
    fn renderer(&mut self) -> &mut dyn ButtonTextRenderer {
        self
    }
    fn type_scale(&self) -> f32 {
        self.scale
    }
    fn scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn proposal_label(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        self.paragraph(value, role, width, true)
    }
    fn proposal_document(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        self.paragraph(value, role, width, false)
    }
    fn digit(&mut self, value: &str, size: f32) -> Rc<Paragraph> {
        let key = (value.to_owned(), format!("system:{}", size.to_bits()), None);
        if let Some(paragraph) = self.cache.paragraphs.get(&key) {
            return paragraph.clone();
        }
        let paragraph = self
            .text
            .text_surface_host()
            .expect("AppHost text service")
            .shape_system_text(value, size, None)
            .expect("AppHost final-size system text");
        self.cache.insert(key, paragraph)
    }
    fn push(&mut self, scene: &mut Scene, p: &Paragraph, origin: Point, ink: Rgba) {
        self.text
            .text_surface_host()
            .expect("AppHost text service")
            .push_visible(scene, p, origin, ink, self.clip);
    }
}

// ButtonModel still owns measurement, hit layout, focus and animation. This
// bridge retains the page's localized paragraph cache behind that same API.
impl ButtonTextRenderer for HostText<'_> {
    fn backing_scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.text.resolved_font_size(role)
    }
    fn motion_context(&self) -> Option<ControlMotionContext> {
        self.text.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.text.request_animation_frame();
    }
    fn animation_frames_allowed(&self) -> bool {
        self.text.animation_frames_allowed()
    }
    fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.button_paragraph(value, role, width);
        Size::new(p.width as f32, p.height as f32)
    }
    fn first_line_height(&mut self, value: &str, role: proposal::TextRole, width: f32) -> f32 {
        let p = self.button_paragraph(value, role, width);
        p.lines
            .first()
            .map_or(p.default_line_height as f32, |line| line.height as f32)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        value: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: tekes_ui_controls::ButtonTextAlign,
        ink: Rgba,
    ) {
        let p = self.button_paragraph(value, role, rect.size.width);
        let x = if align == tekes_ui_controls::ButtonTextAlign::Center {
            rect.min_x() + (rect.size.width - p.width as f32) * 0.5
        } else {
            rect.min_x()
        };
        self.push(
            scene,
            &p,
            Point::new(
                x.round(),
                (rect.min_y() + (rect.size.height - p.height as f32) * 0.5).round(),
            ),
            ink,
        );
    }
}
impl HostText<'_> {
    fn button_paragraph(
        &mut self,
        value: &str,
        role: proposal::TextRole,
        width: f32,
    ) -> Rc<Paragraph> {
        let natural = self.proposal_label(value, role, None);
        if natural.width as f32 <= width.max(0.) + 1. / 64. {
            natural
        } else {
            self.proposal_label(value, role, Some(width.max(0.)))
        }
    }
}

#[derive(Debug)]
struct State {
    preview: MotionPreview,
    cache: ParagraphCache,
    layout: Option<Layout>,
    frame: Rect,
    clip: Rect,
    active: bool,
    runnable: bool,
    captured: bool,
    pending_reveal: bool,
    clock: Option<ControlMotionContext>,
    last_time: Option<Duration>,
    symbol_renderer: Option<u64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            preview: MotionPreview::default(),
            cache: ParagraphCache::default(),
            layout: None,
            frame: Rect::default(),
            clip: Rect::default(),
            active: true,
            runnable: false,
            captured: false,
            pending_reveal: false,
            clock: None,
            last_time: None,
            symbol_renderer: None,
        }
    }
}
impl State {
    fn cancel(&mut self) {
        // Press has a real down state, including cancellation outside the viewport.
        if self.preview.is_recipe()
            && self.preview.recipe.kind == tekes_ui_motion::recipes::RecipeKind::Press
            && self.preview.pressed.is_some()
        {
            self.preview.retarget(0.15);
        }
        self.preview.deactivate();
        self.captured = false;
        self.pending_reveal = false;
    }
    fn stop(&mut self) {
        self.cancel();
        self.last_time = None;
        self.clock = None;
    }
    fn begin_frame(&mut self, clock: Option<ControlMotionContext>) {
        if let Some(clock) = clock {
            self.preview.set_system_reduced_motion(clock.reduced_motion);
        }
        self.clock = clock;
        self.advance();
    }
    fn advance(&mut self) {
        if !self.active || !self.runnable {
            self.last_time = None;
            return;
        }
        if let Some(clock) = self.clock.take() {
            let dt = self
                .last_time
                .map_or(0., |old| clock.now.saturating_sub(old).as_secs_f32());
            self.last_time = Some(clock.now);
            self.preview.animate(dt);
        }
    }
    fn input(&mut self, event: native::Event) -> ContentResponse {
        if !self.active || !self.runnable {
            return ContentResponse::default();
        }
        // The legacy Press action's activation arm is intentionally empty: its
        // demonstration is a held pointer gesture. Keyboard activation uses
        // that same down/up reducer, preserving the actual recipe transition.
        if let native::Event::Key(key) = &event
            && matches!(key.key_code, 36 | 49)
            && self.preview.is_recipe()
            && self.preview.recipe.kind == tekes_ui_motion::recipes::RecipeKind::Press
            && self.preview.focused_button == Some(Layout::action_id(Action::PressDemo))
        {
            if key.is_repeat {
                return ContentResponse::consumed(false);
            }
            let Some(frame) = motion::focused_button_frame(&self.preview) else {
                return ContentResponse::default();
            };
            let down = key.kind == native::KeyEventKind::Down;
            let kind = if down {
                native::MouseEventKind::Down {
                    button: native::MouseButton::Left,
                    click_count: 1,
                }
            } else {
                native::MouseEventKind::Up {
                    button: native::MouseButton::Left,
                    click_count: 1,
                }
            };
            let response = self.input(mouse_event(kind, frame.center()));
            if key.key_code == 36 && down {
                self.input(mouse_event(
                    native::MouseEventKind::Up {
                        button: native::MouseButton::Left,
                        click_count: 1,
                    },
                    frame.center(),
                ));
            }
            return response;
        }
        let handled =
            motion::handle_input_clipped(&mut self.preview, &event, self.frame, self.clip);
        if handled {
            ContentResponse::consumed(true)
        } else {
            ContentResponse::default()
        }
    }
    fn tab(&mut self, reverse: bool) -> ContentResponse {
        let result = self.input(key_event(48, true, reverse));
        self.pending_reveal |= result.consumed;
        result
    }
    fn part(&self, action: Action) -> usize {
        (self.preview.catalog as usize + 1) * 0x10000 + Layout::action_id(action) as usize
    }
    fn slider_part(&self, index: usize) -> usize {
        (self.preview.catalog as usize + 1) * 0x10000 + 0x6000 + index
    }
}
fn key_event(code: u16, down: bool, reverse: bool) -> native::Event {
    native::Event::Key(native::KeyEvent {
        kind: if down {
            native::KeyEventKind::Down
        } else {
            native::KeyEventKind::Up
        },
        key_code: code,
        characters: String::new(),
        characters_ignoring_modifiers: String::new(),
        modifiers: native::Modifiers {
            shift: reverse,
            ..native::Modifiers::NONE
        },
        is_repeat: false,
    })
}
fn mouse_event(kind: native::MouseEventKind, point: Point) -> native::Event {
    native::Event::Mouse(native::MouseEvent {
        kind,
        position: native::Point {
            x: point.x as f64,
            y: point.y as f64,
        },
        modifiers: native::Modifiers::NONE,
    })
}
#[derive(Debug)]
struct MotionContent(Rc<RefCell<State>>);
impl Component for MotionContent {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let mut state = self.0.borrow_mut();
        let State { preview, cache, .. } = &mut *state;
        let mut text = HostText::new(text, cache, Rect::default());
        MeasuredChild::new(
            Layout::for_state(Rect::new(0., 0., available.width, 0.), preview, &mut text)
                .content
                .size,
        )
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        let mut state = self.0.borrow_mut();
        state.frame = frame;
        state.clip = clip;
        let State {
            preview,
            cache,
            layout,
            ..
        } = &mut *state;
        let mut text = HostText::new(text, cache, clip);
        *layout = Some(Layout::for_state(frame, preview, &mut text));
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let mut state = self.0.borrow_mut();
        state.advance();
        scene.push_clip(clip);
        let State {
            preview,
            cache,
            layout,
            ..
        } = &mut *state;
        let mut bridge = HostText::new(text, cache, clip);
        *layout = Some(Layout::for_state(frame, preview, &mut bridge));
        motion::build_content(
            theme.palette.theme,
            &mut bridge,
            frame,
            clip,
            scene,
            preview,
        );
        scene.pop_clip();
        if state.active
            && state.runnable
            && state.preview.needs_animation()
            && text.animation_frames_allowed()
        {
            text.request_animation_frame();
        }
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut state = self.0.borrow_mut();
        if !focused {
            state.cancel();
            return;
        }
        let focused_id = state.preview.focused_button;
        for (id, model) in &mut state.preview.button_models {
            model.state.focused = Some(*id) == focused_id;
            model.state.focus_visible = model.state.focused && visible;
        }
    }
    fn set_focus_from_traversal(&mut self, reverse: bool) {
        let mut state = self.0.borrow_mut();
        motion::focus_edge(&mut state.preview, reverse);
        state.pending_reveal = true;
    }
    fn focus_boundary(&mut self, reverse: bool) -> ContentResponse {
        self.0.borrow_mut().tab(reverse)
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if pointer.button.is_some_and(|b| b != PointerButton::Primary) {
            return ContentResponse::default();
        }
        let mut state = self.0.borrow_mut();
        let kind = match pointer.phase {
            PointerPhase::Down => native::MouseEventKind::Down {
                button: native::MouseButton::Left,
                click_count: pointer.click_count,
            },
            PointerPhase::Up => native::MouseEventKind::Up {
                button: native::MouseButton::Left,
                click_count: pointer.click_count,
            },
            PointerPhase::Drag => native::MouseEventKind::Dragged {
                button: native::MouseButton::Left,
            },
            PointerPhase::Move if state.captured => native::MouseEventKind::Dragged {
                button: native::MouseButton::Left,
            },
            PointerPhase::Move => native::MouseEventKind::Moved,
            PointerPhase::Enter => native::MouseEventKind::Entered,
            PointerPhase::Exit => native::MouseEventKind::Exited,
        };
        let mut response = state.input(mouse_event(kind, pointer.position));
        if pointer.phase == PointerPhase::Down && response.consumed {
            state.captured = state.preview.dragging.is_some()
                || state.preview.pressed.is_some()
                || state.preview.recipe_dragging;
            response.capture_pointer = state.captured;
        }
        if pointer.phase == PointerPhase::Up {
            state.captured = false;
        }
        response
    }
    fn key_event(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        // Host calls focus_boundary once after an unhandled Tab; avoid double stepping.
        if key.native_code == 48 {
            return ContentResponse::default();
        }
        self.0
            .borrow_mut()
            .input(native::Event::Key(native::KeyEvent {
                kind: if key.down {
                    native::KeyEventKind::Down
                } else {
                    native::KeyEventKind::Up
                },
                key_code: key.native_code,
                characters: key.characters.clone(),
                characters_ignoring_modifiers: key.characters_ignoring_modifiers.clone(),
                modifiers: native::Modifiers {
                    shift: key.modifiers.shift,
                    control: key.modifiers.control,
                    alt: key.modifiers.alt,
                    command: key.modifiers.command,
                    caps_lock: key.modifiers.caps_lock,
                    function: key.modifiers.function,
                },
                is_repeat: key.repeat,
            }))
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        match event {
            ContentEvent::Cancel => self.cancel_pointer(),
            ContentEvent::Key {
                key: tekes_ui_controls::ControlKey::Tab { .. },
                ..
            } => ContentResponse::default(),
            ContentEvent::Key { key, down } => {
                let code = match key {
                    tekes_ui_controls::ControlKey::Enter => 36,
                    tekes_ui_controls::ControlKey::Space => 49,
                    _ => return ContentResponse::default(),
                };
                self.0.borrow_mut().input(key_event(code, *down, false))
            }
            ContentEvent::PointerDown(p)
            | ContentEvent::PointerMove(p)
            | ContentEvent::PointerUp(p) => {
                let phase = match event {
                    ContentEvent::PointerDown(_) => PointerPhase::Down,
                    ContentEvent::PointerUp(_) => PointerPhase::Up,
                    _ => PointerPhase::Move,
                };
                self.pointer_event(&PointerEvent {
                    phase,
                    position: *p,
                    button: Some(PointerButton::Primary),
                    modifiers: InputModifiers::default(),
                    click_count: 1,
                })
            }
            _ => ContentResponse::default(),
        }
    }
    fn pointer_capture_current(&self) -> bool {
        let s = self.0.borrow();
        s.active
            && s.runnable
            && s.captured
            && (s.preview.dragging.is_some()
                || s.preview.pressed.is_some()
                || s.preview.recipe_dragging)
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        self.0.borrow_mut().cancel();
        ContentResponse::consumed(true)
    }
    fn window_active_changed(&mut self, active: bool) {
        let mut s = self.0.borrow_mut();
        s.active = active;
        if !active {
            s.stop();
        }
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        let mut s = self.0.borrow_mut();
        // Initial pre-layout withdrawal must not discard this frame's
        // prepared clock. Only leaving a runnable presentation retires it.
        if s.runnable && !runnable {
            s.stop();
        }
        s.runnable = runnable;
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        let mut s = self.0.borrow_mut();
        s.runnable = false;
        s.stop();
        if event == Lifecycle::Mounted {
            s.active = true;
        }
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Motion laboratory")
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let s = self.0.borrow();
        Some(vec![
            InspectionProperty::new("component", "Motion"),
            InspectionProperty::new("catalog", format!("{:?}", s.preview.catalog)),
            InspectionProperty::new(
                "preset",
                s.preview
                    .presets
                    .get(s.preview.selected)
                    .map_or("Ticking Number", |p| p.label.as_str()),
            ),
            InspectionProperty::new(
                "paused",
                (s.preview.is_recipe() && s.preview.recipe.is_paused()).to_string(),
            ),
            InspectionProperty::new("reduced_motion", s.preview.reduced_motion.to_string()),
            InspectionProperty::new("animating", s.preview.needs_animation().to_string()),
        ])
    }

    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let s = self.0.borrow();
        let Some(layout) = s.layout else {
            return vec![];
        };
        let mut parts: Vec<_> = layout
            .state_actions(&s.preview)
            .into_iter()
            .map(|(action, frame)| {
                let mut semantics = ContentSemantics::new(
                    ContentRole::Button,
                    Locale::default().text(motion::action_label(action, &s.preview)),
                );
                semantics.selected = s
                    .preview
                    .button_models
                    .get(&Layout::action_id(action))
                    .is_some_and(|b| b.state.selected);
                semantics.disabled = s
                    .preview
                    .button_models
                    .get(&Layout::action_id(action))
                    .is_some_and(|b| b.state.disabled);
                SemanticPart {
                    index: s.part(action),
                    frame,
                    focused: s.preview.focused_button == Some(Layout::action_id(action)),
                    semantics,
                }
            })
            .collect();
        for (i, parameter) in Parameter::ALL.into_iter().enumerate() {
            let frame = layout.slider(parameter);
            if frame.is_empty() {
                continue;
            }
            let mut semantics =
                ContentSemantics::new(ContentRole::Slider, s.preview.parameter_label(parameter));
            semantics.value = Some(s.preview.parameter_value(parameter).to_string());
            semantics.disabled = !s.preview.parameter_enabled(parameter);
            parts.push(SemanticPart {
                index: s.slider_part(i),
                frame,
                focused: false,
                semantics,
            });
        }
        parts
    }
    fn semantic_part_value_range(&self, part: usize) -> Option<std::ops::RangeInclusive<f64>> {
        let s = self.0.borrow();
        Parameter::ALL
            .into_iter()
            .enumerate()
            .find(|(i, _)| s.slider_part(*i) == part)
            .map(|(_, p)| {
                let (lo, hi) = s.preview.parameter_range(p);
                lo..=hi
            })
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        let mut s = self.0.borrow_mut();
        if !s.active || !s.runnable {
            return ContentResponse::default();
        }
        let (Some(part), Some(layout)) = (part, s.layout) else {
            return ContentResponse::default();
        };
        if let Some((button, _)) = layout
            .state_actions(&s.preview)
            .into_iter()
            .find(|(button, _)| s.part(*button) == part)
        {
            match action {
                AccessibilityAction::Focus => {
                    s.preview.focused_button = Some(Layout::action_id(button));
                    s.pending_reveal = true;
                }
                AccessibilityAction::Press => {
                    if s.preview
                        .button_models
                        .get(&Layout::action_id(button))
                        .is_some_and(|b| b.state.disabled)
                    {
                        return ContentResponse::default();
                    }
                    if button == Action::PressDemo {
                        s.preview.retarget(0.85);
                        s.preview.retarget(0.15);
                    } else {
                        motion::apply_action(&mut s.preview, button);
                    }
                }
                _ => return ContentResponse::default(),
            }
            return ContentResponse::consumed(true);
        }
        let Some((_, parameter)) = Parameter::ALL.into_iter().enumerate().find(|(i, p)| {
            s.slider_part(*i) == part
                && !layout.slider(*p).is_empty()
                && s.preview.parameter_enabled(*p)
        }) else {
            return ContentResponse::default();
        };
        let (lo, hi) = s.preview.parameter_range(parameter);
        let current = s.preview.parameter_value(parameter);
        let value = match action {
            AccessibilityAction::Increment => current + (hi - lo) / 20.,
            AccessibilityAction::Decrement => current - (hi - lo) / 20.,
            AccessibilityAction::SetValue(value) => match value.parse::<f64>() {
                Ok(value) if value.is_finite() => value,
                _ => return ContentResponse::default(),
            },
            _ => return ContentResponse::default(),
        };
        s.preview.set_parameter(parameter, value.clamp(lo, hi));
        ContentResponse::consumed(true)
    }
}

pub(crate) struct AppHostMotion {
    pub(crate) host: ContentHost,
    state: Rc<RefCell<State>>,
}
impl AppHostMotion {
    pub(crate) fn new(_: &'static Entry) -> Result<Self> {
        let state = Rc::new(RefCell::new(State::default()));
        let mut host = ContentHost::new(Content::new(
            ROOT,
            ScrollContent::document(Content::new(SPECIMEN, MotionContent(state.clone()))),
        ))
        .map_err(|e| anyhow::anyhow!("Motion host: {e:?}"))?;
        host.set_focus_wrap(false);
        Ok(Self { host, state })
    }
    pub(crate) fn after_host_visit(&mut self) {
        let frame = {
            let mut s = self.state.borrow_mut();
            if std::mem::take(&mut s.pending_reveal) {
                motion::focused_button_frame(&s.preview)
            } else {
                None
            }
        };
        if let Some(frame) = frame {
            self.host
                .get_mut(ROOT)
                .expect("Motion scroll root")
                .component_mut::<ScrollContent>()
                .expect("Motion ScrollContent")
                .reveal(frame);
        }
    }
    pub(crate) fn prepare_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> Result<()> {
        let mut state = self.state.borrow_mut();
        // Geometric symbol transforms are copied into Scene, so sample the
        // shared clock before preparing them. The first runnable paint consumes
        // any deferred zero-delta sample; a sample is never integrated twice.
        state.begin_frame(context.motion);
        let identity = context.renderer.resource_identity();
        if state.symbol_renderer != Some(identity) {
            // Original SymbolAssets owns raw IDs in one Renderer namespace.
            // Its existing fresh-clone contract preserves the Motion model.
            state.preview.symbol_assets = Default::default();
            state.symbol_renderer = Some(identity);
        }
        state
            .preview
            .prepare_symbols(context.renderer, context.palette.theme, context.scale);
        Ok(())
    }
    pub(crate) fn finish_resources(&mut self, _: &mut ContainerRenderContext<'_>) -> Result<()> {
        Ok(())
    }
    pub(crate) fn select_catalog(
        &mut self,
        catalog: crate::gallery::motion_preview::PresetCatalog,
    ) -> bool {
        let changed = self.state.borrow_mut().preview.select_catalog(catalog);
        if changed {
            let _ = self.host.get_mut(SPECIMEN);
        }
        changed
    }
    pub(crate) fn set_locale(&mut self, locale: crate::gallery::locale::Locale) {
        let mut s = self.state.borrow_mut();
        if s.preview.locale != locale {
            s.preview.locale = locale;
            s.cache.paragraphs.clear();
            let _ = self.host.get_mut(SPECIMEN);
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn retarget(&mut self, target: f64) {
        self.state.borrow_mut().preview.retarget(target);
        let _ = self.host.get_mut(SPECIMEN);
    }
    #[cfg(test)]
    pub(crate) fn set_ticking_target(
        &mut self,
        target: u64,
    ) -> std::result::Result<bool, tekes_ui_controls::TickingNumberError> {
        let changed = self.state.borrow_mut().preview.ticking.set_target(target)?;
        if changed {
            let _ = self.host.get_mut(SPECIMEN);
        }
        Ok(changed)
    }
    #[cfg(test)]
    pub(crate) fn ticking_value(&self) -> u64 {
        self.state.borrow().preview.ticking.value()
    }
    #[cfg(test)]
    pub(crate) fn ticking_needs_animation(&self) -> bool {
        self.state.borrow().preview.ticking.needs_animation()
    }
    #[cfg(test)]
    pub(crate) fn measure_ticking_width(&self, text: &mut dyn ButtonTextRenderer) -> f32 {
        self.state
            .borrow()
            .preview
            .ticking
            .measure(text, tekes_ui_tokens::proposal::TextRole::UiHeading)
            .width
    }
    pub(crate) fn scroll_metrics(&self) -> (f32, f32) {
        self.host
            .get(ROOT)
            .and_then(|node| node.component::<ScrollContent>())
            .map_or((0., 0.), |scroll| {
                (
                    scroll.scroll.limit().y,
                    scroll.scroll.viewport_size().height,
                )
            })
    }
    pub(crate) fn motion_case_diagnostics(&self) -> serde_json::Value {
        let rect_json = |rect: Rect| {
            serde_json::json!({
                "x": rect.min_x(),
                "y": rect.min_y(),
                "width": rect.size.width,
                "height": rect.size.height,
            })
        };
        let s = self.state.borrow();
        let actions = s
            .layout
            .map(|layout| layout.state_actions(&s.preview))
            .unwrap_or_default();
        let buttons: Vec<_> = actions
            .iter()
            .filter_map(|(action, _)| {
                let id = Layout::action_id(*action);
                let button = s.preview.button_models.get(&id)?;
                let layout = button.layout()?;
                let visible = layout.visible.filter(|rect| !rect.is_empty());
                Some(serde_json::json!({
                    "id": format!("{id:016x}"),
                    "action": format!("{action:?}"),
                    "frame": rect_json(layout.frame),
                    "content": rect_json(layout.content),
                    "visible": visible.map(rect_json),
                    "label_size": {
                        "width": layout.label_size.width,
                        "height": layout.label_size.height,
                    },
                }))
            })
            .collect();
        let (limit, view_h) = self.scroll_metrics();
        let document_extent =
            (limit + view_h).max(s.layout.map_or(0., |layout| layout.content.size.height));
        let visible_button_count = buttons
            .iter()
            .filter(|button| !button["visible"].is_null())
            .count();
        serde_json::json!({
            "document_extent": document_extent,
            "scroll_range": limit,
            "rendered_button_count": buttons.len(),
            "visible_button_count": visible_button_count,
            "buttons": buttons,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{motion_preview::PresetCatalog, text::TextEngine};
    use tekes_ui_controls::ControlKey;
    use tekes_ui_motion::recipes::RecipeKind;
    use tekes_ui_tokens::{Palette, Theme};

    fn page() -> AppHostMotion {
        AppHostMotion::new(
            super::super::entries()
                .iter()
                .find(|e| e.renderer() == "motion")
                .unwrap(),
        )
        .unwrap()
    }
    fn draw(
        page: &mut AppHostMotion,
        text: &mut TextEngine,
        width: f32,
        height: f32,
        now: f64,
    ) -> Scene {
        text.begin_frame();
        text.set_motion_enabled(true);
        text.set_motion_context(Duration::from_secs_f64(now), false);
        page.state
            .borrow_mut()
            .begin_frame(Some(ControlMotionContext {
                now: Duration::from_secs_f64(now),
                reduced_motion: false,
            }));
        let frame = Rect::new(0., 0., width, height);
        let mut scene = Scene::new();
        page.host
            .render(&mut scene, text, frame, frame, Palette::from(Theme::LIGHT))
            .unwrap();
        page.after_host_visit();
        scene
    }
    fn tab(page: &mut AppHostMotion, reverse: bool) -> ContentResponse {
        let response = page.host.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse },
            down: true,
        });
        page.after_host_visit();
        response
    }
    fn pointer(page: &mut AppHostMotion, phase: PointerPhase, point: Point) -> ContentResponse {
        page.host.event(&ContentEvent::Pointer(PointerEvent {
            phase,
            position: point,
            button: Some(PointerButton::Primary),
            modifiers: InputModifiers::default(),
            click_count: 1,
        }))
    }
    fn recipe(page: &mut AppHostMotion, kind: RecipeKind) {
        let mut s = page.state.borrow_mut();
        s.preview.select_catalog(PresetCatalog::Recipes);
        let i = RecipeKind::ALL.iter().position(|k| *k == kind).unwrap();
        s.preview.select(i);
    }
    #[test]
    fn apphost_motion_tab_reveals_internal_buttons_once_and_exits_both_ends() {
        let mut page = page();
        let mut text = TextEngine::new();
        draw(&mut page, &mut text, 720., 240., 0.);
        let order = page.state.borrow().preview.focus_order.clone();
        for (i, id) in order.iter().enumerate() {
            assert!(tab(&mut page, false).consumed, "forward {i}");
            draw(&mut page, &mut text, 720., 240., 0.);
            let s = page.state.borrow();
            assert_eq!(s.preview.focused_button, Some(*id), "one step per Tab");
            let frame = motion::focused_button_frame(&s.preview).unwrap();
            assert!(
                frame.min_y() >= s.clip.min_y() - 1. && frame.max_y() <= s.clip.max_y() + 1.,
                "button {i}: {frame:?}, {:?}",
                s.clip
            );
        }
        assert!(
            !tab(&mut page, false).consumed,
            "last button exits outer Host"
        );
        // The outer Container owns the transfer after the unconsumed Tab.
        assert!(page.host.focus(None, false));
        draw(&mut page, &mut text, 720., 240., 0.);
        for id in order.iter().rev() {
            assert!(tab(&mut page, true).consumed);
            draw(&mut page, &mut text, 720., 240., 0.);
            assert_eq!(page.state.borrow().preview.focused_button, Some(*id));
        }
        assert!(!tab(&mut page, true).consumed);
    }
    #[test]
    fn apphost_motion_all_catalog_actions_and_accessibility_mutate_original_models() {
        let mut page = page();
        let mut text = TextEngine::new();
        for catalog in [
            PresetCatalog::Apple,
            PresetCatalog::DesignSystem,
            PresetCatalog::Recipes,
            PresetCatalog::Symbols,
            PresetCatalog::TickingNumber,
        ] {
            page.state.borrow_mut().preview.select_catalog(catalog);
            draw(&mut page, &mut text, 1104., 2400., 0.);
            let parts = page
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<MotionContent>()
                .unwrap()
                .semantic_parts();
            let s = page.state.borrow();
            let actions = s.layout.unwrap().state_actions(&s.preview);
            assert_eq!(
                actions
                    .iter()
                    .filter(|(a, _)| matches!(a, Action::Catalog(_)))
                    .count(),
                5
            );
            assert_eq!(
                actions
                    .iter()
                    .filter(|(a, _)| matches!(a, Action::Preset(_)))
                    .count(),
                s.preview.presets.len()
            );
            for (action, frame) in actions {
                let part = parts.iter().find(|p| p.index == s.part(action)).unwrap();
                assert_eq!(part.frame, frame);
            }
            let toggle = s.part(Action::ReduceMotion);
            let old = s.preview.manual_reduced_motion();
            drop(s);
            assert!(
                page.host
                    .accessibility_action(SPECIMEN, Some(toggle), AccessibilityAction::Press)
                    .consumed
            );
            assert_ne!(page.state.borrow().preview.manual_reduced_motion(), old);
            draw(&mut page, &mut text, 1104., 2400., 0.);
        }
        let digits = page.state.borrow().part(Action::TickDigits);
        assert!(
            page.host
                .accessibility_action(SPECIMEN, Some(digits), AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(page.state.borrow().preview.ticking.digits(), 6);
        page.state
            .borrow_mut()
            .preview
            .select_catalog(PresetCatalog::Apple);
        draw(&mut page, &mut text, 1104., 2400., 0.);
        assert!(
            !page
                .host
                .accessibility_action(SPECIMEN, Some(digits), AccessibilityAction::Press)
                .consumed,
            "old catalog part must not become a new action"
        );
        let part = page.state.borrow().slider_part(0);
        let before = page.state.borrow().preview.frequency;
        assert!(
            page.host
                .accessibility_action(SPECIMEN, Some(part), AccessibilityAction::Increment)
                .consumed
        );
        assert_ne!(page.state.borrow().preview.frequency, before);
        assert!(
            !page
                .host
                .accessibility_action(
                    SPECIMEN,
                    Some(part),
                    AccessibilityAction::SetValue("NaN".into())
                )
                .consumed
        );
    }
    #[test]
    fn apphost_motion_captured_moves_cross_clip_release_and_cancel() {
        let mut page = page();
        let mut text = TextEngine::new();
        draw(&mut page, &mut text, 1104., 2400., 0.);
        let slider = page
            .state
            .borrow()
            .layout
            .unwrap()
            .slider(Parameter::Frequency);
        assert!(pointer(&mut page, PointerPhase::Down, slider.center()).capture_pointer);
        let outside = Point::new(2000., -100.);
        assert!(pointer(&mut page, PointerPhase::Move, outside).consumed);
        assert_eq!(
            page.state.borrow().preview.frequency,
            page.state
                .borrow()
                .preview
                .parameter_range(Parameter::Frequency)
                .1
        );
        assert!(pointer(&mut page, PointerPhase::Up, outside).consumed);
        assert!(page.state.borrow().preview.dragging.is_none());
        assert!(
            !page
                .host
                .get(SPECIMEN)
                .unwrap()
                .component::<MotionContent>()
                .unwrap()
                .pointer_capture_current()
        );
        recipe(&mut page, RecipeKind::DragSnap);
        draw(&mut page, &mut text, 1104., 2400., 0.);
        let stage = page.state.borrow().layout.unwrap().stage;
        pointer(&mut page, PointerPhase::Down, stage.center());
        assert!(page.state.borrow().preview.recipe_dragging);
        pointer(&mut page, PointerPhase::Move, outside);
        page.host.event(&ContentEvent::Cancel);
        let s = page.state.borrow();
        assert!(!s.captured && !s.preview.recipe_dragging && s.preview.pressed.is_none());
    }
    #[test]
    fn apphost_motion_press_ax_and_pointer_release_use_real_recipe_transitions() {
        let mut page = page();
        let mut text = TextEngine::new();
        recipe(&mut page, RecipeKind::Press);
        draw(&mut page, &mut text, 1104., 2400., 0.);
        let button = page
            .state
            .borrow()
            .layout
            .unwrap()
            .button(Action::PressDemo);
        pointer(&mut page, PointerPhase::Down, button.center());
        assert!(page.state.borrow().preview.recipe.target());
        page.host.event(&ContentEvent::Cancel);
        assert!(!page.state.borrow().preview.recipe.target());
        draw(&mut page, &mut text, 1104., 2400., 0.);
        page.state.borrow_mut().preview.pause_recipe();
        assert!(page.state.borrow().preview.recipe.is_paused());
        let part = page.state.borrow().part(Action::PressDemo);
        assert!(
            page.host
                .accessibility_action(SPECIMEN, Some(part), AccessibilityAction::Press)
                .consumed
        );
        assert!(!page.state.borrow().preview.recipe.target());
        assert!(
            !page.state.borrow().preview.recipe.is_paused(),
            "AX must run the real recipe transition, not the old empty apply_action arm"
        );
        assert!(
            page.host
                .accessibility_action(SPECIMEN, Some(part), AccessibilityAction::Focus)
                .consumed
        );
        page.after_host_visit();
        draw(&mut page, &mut text, 1104., 2400., 0.);
        assert!(
            page.host
                .event(&ContentEvent::Key {
                    key: ControlKey::Space,
                    down: true
                })
                .consumed
        );
        assert!(
            page.state.borrow().preview.recipe.target(),
            "Space holds the real Press target"
        );
        assert!(
            page.host
                .event(&ContentEvent::Key {
                    key: ControlKey::Space,
                    down: false
                })
                .consumed
        );
        assert!(!page.state.borrow().preview.recipe.target());
        page.state.borrow_mut().preview.pause_recipe();
        assert!(
            page.host
                .event(&ContentEvent::Key {
                    key: ControlKey::Enter,
                    down: true
                })
                .consumed
        );
        assert!(!page.state.borrow().preview.recipe.is_paused());
        assert!(
            !page.state.borrow().preview.recipe.target(),
            "Enter performs a complete press/release"
        );
    }
    #[test]
    fn apphost_motion_hidden_clock_does_not_catch_up_and_reduction_wins() {
        let mut page = page();
        let mut text = TextEngine::new();
        page.state.borrow_mut().preview.replay();
        draw(&mut page, &mut text, 720., 240., 1.);
        page.state
            .borrow_mut()
            .begin_frame(Some(ControlMotionContext {
                now: Duration::from_secs_f64(1.05),
                reduced_motion: false,
            }));
        let before_paint = page.state.borrow().preview.now;
        assert!(
            before_paint > Duration::ZERO,
            "resource preparation must sample this frame before painting"
        );
        draw(&mut page, &mut text, 720., 240., 1.05);
        assert_eq!(
            page.state.borrow().preview.now,
            before_paint,
            "paint must not integrate the resource sample twice"
        );
        page.host.set_window_active(false);
        let before = page.state.borrow().preview.now;
        draw(&mut page, &mut text, 720., 240., 100.);
        assert_eq!(page.state.borrow().preview.now, before);
        page.host.set_window_active(true);
        draw(&mut page, &mut text, 720., 240., 200.);
        assert_eq!(page.state.borrow().preview.now, before);
        draw(&mut page, &mut text, 720., 240., 200.05);
        assert!(page.state.borrow().preview.now > before);
        page.state
            .borrow_mut()
            .begin_frame(Some(ControlMotionContext {
                now: Duration::from_secs(201),
                reduced_motion: true,
            }));
        assert!(page.state.borrow().preview.reduced_motion);
        assert!(!page.state.borrow().preview.needs_animation());
    }
    #[test]
    fn apphost_motion_text_keeps_full_typing_clusters_and_final_size_digit_cache() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let mut cache = ParagraphCache::default();
        let mut bridge = HostText::new(&mut text, &mut cache, Rect::new(0., 0., 720., 240.));
        let full = "office e\u{301} 👩‍💻 中文";
        let paragraph = bridge.proposal_document(full, proposal::TextRole::UiBody, Some(160.));
        assert_eq!(paragraph.utf16_len, full.encode_utf16().count());
        assert!(Rc::ptr_eq(
            &paragraph,
            &bridge.proposal_document(full, proposal::TextRole::UiBody, Some(160.))
        ));
        let large = bridge.digit("8", 64.);
        let small = bridge.digit("8", 24.);
        assert!((large.fonts[0].size - 64.).abs() < 1e-6);
        assert!((small.fonts[0].size - 24.).abs() < 1e-6);
        assert!(small.width < large.width && small.height < large.height);
        assert!(Rc::ptr_eq(&large, &bridge.digit("8", 64.)));
        assert!(
            paragraph
                .lines
                .iter()
                .flat_map(|l| &l.runs)
                .flat_map(|r| &r.glyphs)
                .any(|g| g.cluster_utf16.len() > 1),
            "combining and emoji geometry remains shaped as clusters"
        );
        let mut page = page();
        recipe(&mut page, RecipeKind::Typing);
        draw(&mut page, &mut text, 720., 2400., 0.);
        let full = {
            let s = page.state.borrow();
            crate::gallery::motion_preview::typing_fixture(
                s.preview.locale,
                s.preview.recipe.target(),
            )
            .concat()
        };
        let retained = page
            .state
            .borrow()
            .cache
            .paragraphs
            .iter()
            .find(|((source, _, _), _)| source == &full)
            .unwrap()
            .1
            .clone();
        for now in [0.1, 0.3, 0.7] {
            draw(&mut page, &mut text, 720., 2400., now);
            let s = page.state.borrow();
            let current = s
                .cache
                .paragraphs
                .iter()
                .find(|((source, _, _), _)| source == &full)
                .unwrap()
                .1;
            assert!(
                Rc::ptr_eq(&retained, current),
                "typing progress must not reshape prefixes"
            );
        }
    }
    #[test]
    #[ignore = "requires a real wgpu GPU adapter"]
    fn apphost_motion_symbols_first_frame_same_clock_and_renderer_replacement() {
        use tekes_ui_render::Renderer;
        use tekes_ui_scene::Primitive;
        fn render(
            page: &mut AppHostMotion,
            renderer: &mut Renderer,
            text: &mut TextEngine,
            now: f64,
        ) -> (Scene, Vec<u8>) {
            text.begin_frame();
            text.set_motion_context(Duration::from_secs_f64(now), false);
            text.set_motion_enabled(true);
            let frame = Rect::new(0., 0., 1104., 1600.);
            let palette = Palette::from(Theme::LIGHT);
            let mut context = ContainerRenderContext {
                renderer,
                frame,
                clip: frame,
                scale: 1.,
                motion: Some(ControlMotionContext {
                    now: Duration::from_secs_f64(now),
                    reduced_motion: false,
                }),
                palette,
            };
            page.prepare_resources(&mut context).unwrap();
            let before_paint = page.state.borrow().preview.recipe.now();
            let mut scene = Scene::new();
            page.host
                .render(&mut scene, text, frame, frame, palette)
                .unwrap();
            page.finish_resources(&mut context).unwrap();
            assert_eq!(
                page.state.borrow().preview.recipe.now(),
                before_paint,
                "symbols and page consume the same frame sample"
            );
            let pixels = context
                .renderer
                .render_to_rgba8(
                    &scene,
                    text.atlas_mut(),
                    (1104, 1600),
                    1.,
                    crate::gallery::Gallery::background(Theme::LIGHT),
                )
                .unwrap();
            (scene, pixels)
        }
        let images = |scene: &Scene| {
            scene
                .primitives()
                .iter()
                .filter_map(|p| {
                    if let Primitive::TransformedImage(image) = p {
                        Some(*image)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        let mut page = page();
        let mut text = TextEngine::new();
        let mut renderer = Renderer::new_headless().unwrap();
        {
            let mut s = page.state.borrow_mut();
            s.preview.select_catalog(PresetCatalog::Symbols);
            let index = s
                .preview
                .presets
                .iter()
                .position(|p| p.id == "symbol.Rotate")
                .unwrap();
            s.preview.select(index);
        }
        let (first, pixels) = render(&mut page, &mut renderer, &mut text, 1.);
        assert!(
            !images(&first).is_empty(),
            "first Symbols frame must contain original asset images"
        );
        assert!(renderer.image_count() > 0);
        let mut without_symbols = Scene::new();
        for p in first
            .primitives()
            .iter()
            .filter(|p| !matches!(p, Primitive::TransformedImage(_)))
        {
            without_symbols.push(p.clone());
        }
        let blank = renderer
            .render_to_rgba8(
                &without_symbols,
                text.atlas_mut(),
                (1104, 1600),
                1.,
                crate::gallery::Gallery::background(Theme::LIGHT),
            )
            .unwrap();
        assert_ne!(
            pixels, blank,
            "registered original symbols must contribute actual pixels"
        );
        page.state.borrow_mut().preview.replay();
        let (animated, _) = render(&mut page, &mut renderer, &mut text, 1.25);
        assert_ne!(
            images(&first),
            images(&animated),
            "current-frame geometric transforms must advance before scene construction"
        );
        {
            let mut s = page.state.borrow_mut();
            let index = s
                .preview
                .presets
                .iter()
                .position(|p| p.id == "symbol.DrawOn")
                .unwrap();
            s.preview.select(index);
        }
        let (raster, _) = render(&mut page, &mut renderer, &mut text, 1.4);
        assert!(!images(&raster).is_empty());
        assert!(
            page.state
                .borrow()
                .preview
                .symbol_assets
                .images
                .iter()
                .all(Option::is_some)
        );
        let before = {
            let s = page.state.borrow();
            (
                s.preview.catalog,
                s.preview.selected,
                s.preview.symbol_icon,
                s.preview.recipe.now(),
                s.preview.recipe.values(),
            )
        };
        let mut replacement = Renderer::new_headless().unwrap();
        let (replaced, _) = render(&mut page, &mut replacement, &mut text, 1.4);
        assert!(!images(&replaced).is_empty());
        assert!(
            replacement.image_count() > 0,
            "new Renderer must receive its own registrations"
        );
        let after = {
            let s = page.state.borrow();
            (
                s.preview.catalog,
                s.preview.selected,
                s.preview.symbol_icon,
                s.preview.recipe.now(),
                s.preview.recipe.values(),
            )
        };
        assert_eq!(
            before, after,
            "resource replacement preserves the original Motion model"
        );
    }
}
