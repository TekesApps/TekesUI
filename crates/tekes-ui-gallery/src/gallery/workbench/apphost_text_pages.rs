//! Original TextDemo documents mounted as real native text leaves in one ContentHost.
use super::super::{
    locale::Locale,
    text_demo::{TextDemo, section_layout},
};
use std::time::Duration;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Spacing, proposal};

pub(crate) const SPECIMEN: ComponentId = ComponentId(99000);
pub(crate) const STREAM: ComponentId = ComponentId(99020);
pub(crate) const LATEST: ComponentId = ComponentId(99021);
pub(crate) const FIRST_SURFACE: ComponentId = ComponentId(99001);

pub(crate) fn supports(renderer: &str) -> bool {
    matches!(renderer, "text" | "scroll" | "internal-text")
}

pub(crate) fn content(renderer: &str, locale: Locale) -> Option<Content> {
    let mut demo = match renderer {
        "text" => TextDemo::text_only(),
        "scroll" => TextDemo::scroll_only(),
        "internal-text" => TextDemo::editors_only(),
        _ => return None,
    };
    demo.set_locale(locale);
    let fields = demo.into_content_fields();
    let count = fields.len();
    let reading = fields[0].reading.is_some();
    let mut children: Vec<_> = fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| Content::new(ComponentId(FIRST_SURFACE.0 + i as u64), field))
        .collect();
    if reading {
        children.push(Content::new(STREAM, Button::new("Stream 4 updates")));
        children.push(Content::new(LATEST, Button::new("Return to latest")));
    }
    Some(Content::new(
        SPECIMEN,
        TextPage {
            children,
            count,
            reading,
            locale,
            headings: Vec::new(),
            appended: 0,
            remaining: 0,
            elapsed: 0.,
            last_time: None,
            active: true,
            runnable: false,
            arranged: false,
        },
    ))
}

/// Called by the existing Gallery action drain, after the real leaf Button activates.
pub(crate) fn action(host: &mut ContentHost, id: ComponentId) -> bool {
    if id != STREAM && id != LATEST {
        return false;
    }
    let Some(page) = host
        .get_mut(SPECIMEN)
        .and_then(|node| node.component_mut::<TextPage>())
    else {
        return false;
    };
    if id == STREAM {
        page.append();
        page.remaining = 3;
        page.elapsed = 0.;
        page.last_time = None;
    } else {
        let field = page.field_mut(0);
        if let Some(reading) = &mut field.reading {
            reading.follow_latest(&mut field.surface);
        }
    }
    true
}

#[derive(Debug)]
struct TextPage {
    children: Vec<Content>,
    count: usize,
    reading: bool,
    locale: Locale,
    headings: Vec<(&'static str, Rect)>,
    appended: usize,
    remaining: u8,
    elapsed: f32,
    last_time: Option<Duration>,
    active: bool,
    runnable: bool,
    arranged: bool,
}
impl TextPage {
    fn field_mut(&mut self, index: usize) -> &mut TextFieldContent {
        self.children[index]
            .component_mut::<TextFieldContent>()
            .expect("original TextDemo surface")
    }
    fn append(&mut self) {
        self.appended += 1;
        let update = format!(
            "\n\nUpdate {}. This paragraph is appended on request. While reading earlier lines, your place stays anchored; choose Return to latest to follow new text.",
            self.appended
        );
        self.field_mut(0).surface.append_text(&update);
    }
    fn advance(&mut self, text: &dyn ButtonTextRenderer, visible: bool) {
        let Some(clock) = text.motion_context() else {
            return;
        };
        if !self.active || !visible {
            self.last_time = None;
            return;
        }
        let previous = self.last_time.replace(clock.now);
        if let Some(previous) = previous {
            self.elapsed += clock.now.saturating_sub(previous).as_secs_f32();
            while self.remaining > 0 && self.elapsed >= 0.75 {
                self.elapsed -= 0.75;
                self.remaining -= 1;
                self.append();
            }
        }
    }
}
impl Component for TextPage {
    fn has_stable_hooks(&self) -> bool {
        true
    }
    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }
    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        // The original reading and editor pages fill the receiving live viewport.
        MeasuredChild::new(available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged = true;
        self.advance(
            text,
            frame.intersection(&clip).is_some_and(|r| !r.is_empty()),
        );
        let gap = Spacing::Regular.value();
        let body = if self.reading {
            let width = (frame.size.width * 0.5 - gap * 0.5).max(0.);
            let size = Size::new(width, frame.size.height);
            let height = self.children[self.count..]
                .iter_mut()
                .map(|child| child.measure(text, size).size.height)
                .fold(0.0_f32, f32::max);
            let y = (frame.max_y() - height).max(frame.min_y());
            for i in 0..2 {
                self.children[self.count + i].arrange(
                    text,
                    Rect::new(frame.min_x() + i as f32 * (width + gap), y, width, height),
                    clip,
                );
            }
            Rect::new(
                frame.min_x(),
                frame.min_y(),
                frame.size.width,
                (y - gap - frame.min_y()).max(0.),
            )
        } else {
            frame
        };
        let surfaces: Vec<_> = self.children[..self.count]
            .iter()
            .map(|node| {
                &node
                    .component::<TextFieldContent>()
                    .expect("original text field")
                    .surface
            })
            .collect();
        let heights: Vec<_> = self.children[..self.count]
            .iter()
            .map(|node| {
                let field = node.component::<TextFieldContent>().unwrap();
                text.measure(
                    &self.locale.text(&field.field.label),
                    proposal::TextRole::UiHeading,
                    f32::MAX,
                )
                .height
            })
            .collect();
        let scale = text
            .text_surface_host()
            .expect("AppHost text service")
            .type_scale();
        let mut sections = section_layout(&surfaces, body, scale, &heights);
        if self.reading {
            let role = surfaces[0].role();
            let height = text.measure("Mg", role, f32::MAX).height;
            if sections[0].2.size.height < height + 2. * proposal::SCROLL_CONTENT_PADDING {
                sections[0].1 = Rect::default();
                sections[0].2 = body;
            }
        }
        self.headings.clear();
        for (index, (name, label, panel)) in sections.into_iter().enumerate() {
            self.headings.push((name, label));
            self.children[index].arrange(text, panel, clip);
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        scene.push_clip(clip);
        let ink = proposal::render_color("primary-text", theme.palette.theme).expect("text token");
        for (label, rect) in &self.headings {
            if !rect.is_empty() {
                text.draw(
                    scene,
                    &self.locale.text(label),
                    proposal::TextRole::UiHeading,
                    *rect,
                    ButtonTextAlign::Leading,
                    ink,
                );
            }
        }
        scene.pop_clip();
        if self.active && self.runnable && self.remaining > 0 && text.animation_frames_allowed() {
            text.request_animation_frame();
        }
    }
    fn window_active_changed(&mut self, active: bool) {
        self.active = active;
        self.last_time = None;
    }
    fn presentation_runnable_changed(&mut self, runnable: bool) {
        self.runnable = runnable;
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.last_time = None;
        self.arranged = false;
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Original Text and ScrollArea fixtures")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_tokens::Theme;

    fn render(host: &mut ContentHost, text: &mut TextEngine, width: f32) {
        let frame = Rect::new(0., 0., width, 480.);
        host.render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
            .unwrap();
    }

    #[test]
    fn migrated_original_documents_keep_readonly_and_editable_native_contracts() {
        for renderer in ["text", "scroll", "internal-text"] {
            let mut host = ContentHost::new(content(renderer, Locale::En).unwrap()).unwrap();
            let mut text = TextEngine::new();
            render(&mut host, &mut text, 640.);
            assert!(host.focus(Some(FIRST_SURFACE), true));
            let before = host.text_surface(FIRST_SURFACE).unwrap().text().to_owned();
            let readonly = renderer != "internal-text";
            assert_eq!(
                host.text_surface(FIRST_SURFACE)
                    .unwrap()
                    .input()
                    .is_read_only(),
                readonly
            );
            host.event_with_text(
                &ContentEvent::Commit {
                    text: "replacement".into(),
                    replacement: Some(0..1),
                },
                &mut text,
            )
            .unwrap();
            let after = host.text_surface(FIRST_SURFACE).unwrap().text();
            if readonly {
                assert_eq!(after, &before);
            } else {
                assert_ne!(after, &before);
            }
            let binding = host
                .semantic_text_layout_snapshot(FIRST_SURFACE, None)
                .unwrap()
                .input
                .binding;
            render(&mut host, &mut text, 320.);
            assert_eq!(
                host.semantic_text_layout_snapshot(FIRST_SURFACE, None)
                    .unwrap()
                    .input
                    .binding,
                binding
            );
        }
    }

    #[test]
    fn reading_actions_append_original_fixture_without_replacing_native_owner() {
        let mut host = ContentHost::new(content("scroll", Locale::En).unwrap()).unwrap();
        let mut text = TextEngine::new();
        render(&mut host, &mut text, 640.);
        let before = host.text_surface(FIRST_SURFACE).unwrap().text().to_owned();
        let binding = host
            .semantic_text_layout_snapshot(FIRST_SURFACE, None)
            .unwrap()
            .input
            .binding;
        assert!(action(&mut host, STREAM));
        assert!(
            host.text_surface(FIRST_SURFACE)
                .unwrap()
                .text()
                .starts_with(&before)
        );
        assert!(
            host.text_surface(FIRST_SURFACE)
                .unwrap()
                .text()
                .ends_with("follow new text.")
        );
        render(&mut host, &mut text, 320.);
        assert!(action(&mut host, LATEST));
        render(&mut host, &mut text, 320.);
        let field = host.text_surface(FIRST_SURFACE).unwrap();
        assert!((field.scroll().offset.y - field.scroll().limit().y).abs() < 1.);
        assert_eq!(
            host.semantic_text_layout_snapshot(FIRST_SURFACE, None)
                .unwrap()
                .input
                .binding,
            binding
        );
    }
}

// Calibration keeps the original measurement-only offscreen engine. Its font keys
// never enter the window Scene: visible paragraphs are shaped by the borrowed host.
use super::super::{
    pages::static_text as calibration,
    static_text::{self, ShapedFixture, StaticText},
    text::TextEngine,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tekes_ui_app_host::ContainerRenderContext;
use tekes_ui_controls::{ButtonModel, ButtonSize, ControlKey};
use tekes_ui_scene::Point;
use tekes_ui_tokens::{Palette, Rgba, TextRole};

pub(crate) const CALIBRATION: ComponentId = ComponentId(99300);

struct CalibrationHostText<'a> {
    text: &'a mut dyn ButtonTextRenderer,
    fixtures: &'a mut HashMap<(usize, u32), Rc<ShapedFixture>>,
    locale: Locale,
    clip: Rect,
}
impl calibration::CalibrationText for CalibrationHostText<'_> {
    fn locale(&self) -> Locale {
        self.locale
    }
    fn scale(&self) -> f32 {
        self.text.backing_scale()
    }
    fn label(
        &mut self,
        value: &str,
        role: TextRole,
        width: Option<f32>,
    ) -> Rc<tekes_ui_text::Paragraph> {
        let value = self.locale.text(value);
        let host = self.text.text_surface_host().expect("AppHost text service");
        let style = role.style();
        host.shape_system_text_with_weight(
            &value,
            style.size * host.type_scale(),
            style.weight,
            width,
        )
        .expect("AppHost weighted system shaping")
    }
    fn shaped_fixture(&mut self, state: &mut StaticText) -> Rc<ShapedFixture> {
        let key = (state.fixture(), state.width());
        if let Some(shaped) = self.fixtures.get(&key) {
            return shaped.clone();
        }
        let paragraph = self
            .text
            .text_surface_host()
            .expect("AppHost text service")
            .shape_system_text(
                static_text::FIXTURES[key.0].text,
                static_text::FIXTURE_FONT_SIZE,
                Some(key.1 as f32),
            )
            .expect("AppHost system shaping");
        let shaped = Rc::new(static_text::crop_paragraph(&paragraph));
        self.fixtures.insert(key, shaped.clone());
        shaped
    }
    fn measure_button(
        &mut self,
        value: &str,
        width: f32,
        size: ButtonSize,
        role: proposal::TextRole,
    ) -> Size {
        ButtonModel::measure(&mut self.text, &self.locale.text(value), width, size, role)
    }
    fn render_button(
        &mut self,
        scene: &mut Scene,
        model: &mut ButtonModel,
        label: &str,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
        variant: tekes_ui_controls::ButtonVariant,
        size: ButtonSize,
        role: proposal::TextRole,
        align: ButtonTextAlign,
    ) {
        model.render(
            scene,
            &mut self.text,
            &self.locale.text(label),
            frame,
            clip,
            palette,
            variant,
            size,
            role,
            align,
        );
    }
    fn push(
        &mut self,
        scene: &mut Scene,
        paragraph: &tekes_ui_text::Paragraph,
        origin: Point,
        ink: Rgba,
    ) {
        self.text
            .text_surface_host()
            .expect("AppHost text service")
            .push_visible(scene, paragraph, origin, ink, self.clip);
    }
}
#[derive(Debug)]
struct CalibrationState {
    original: StaticText,
    measurement: TextEngine,
    visible: HashMap<(usize, u32), Rc<ShapedFixture>>,
    renderer: Option<u64>,
    locale: Locale,
    captured: bool,
}
/// Resource half of the calibration page. The coordinator's original
/// ContainerContent forwards prepare_resources here before its one host render.
#[derive(Clone, Debug)]
pub(crate) struct CalibrationPage(Rc<RefCell<CalibrationState>>);
impl CalibrationPage {
    pub(crate) fn new(locale: Locale) -> Self {
        Self(Rc::new(RefCell::new(CalibrationState {
            original: StaticText::from_env(),
            measurement: TextEngine::new(),
            visible: HashMap::new(),
            renderer: None,
            locale,
            captured: false,
        })))
    }
    pub(crate) fn content(&self) -> Content {
        Content::new(CALIBRATION, CalibrationContent(self.0.clone()))
    }
    pub(crate) fn prepare_resources(
        &mut self,
        context: &mut ContainerRenderContext<'_>,
    ) -> anyhow::Result<()> {
        let mut state = self.0.borrow_mut();
        let identity = context.renderer.resource_identity();
        if state.renderer != Some(identity) {
            state.original.forget_renderer_resources();
            state.visible.clear();
            state.renderer = Some(identity);
        }
        let CalibrationState {
            original,
            measurement,
            ..
        } = &mut *state;
        original.prepare(
            measurement,
            context.renderer,
            context.palette.theme,
            context.scale,
        );
        Ok(())
    }
}
#[derive(Debug)]
struct CalibrationContent(Rc<RefCell<CalibrationState>>);
impl Component for CalibrationContent {
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        true
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let mut state = self.0.borrow_mut();
        let CalibrationState {
            original,
            visible,
            locale,
            ..
        } = &mut *state;
        let mut bridge = CalibrationHostText {
            text,
            fixtures: visible,
            locale: *locale,
            clip: Rect::default(),
        };
        let height = calibration::document_extent(available.width, &mut bridge, original);
        MeasuredChild::new(Size::new(available.width, height))
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
        let CalibrationState {
            original,
            visible,
            locale,
            ..
        } = &mut *state;
        let mut bridge = CalibrationHostText {
            text,
            fixtures: visible,
            locale: *locale,
            clip,
        };
        scene.push_clip(clip);
        calibration::build_content(
            theme.palette.theme,
            &mut bridge,
            original,
            frame,
            clip,
            scene,
        );
        scene.pop_clip();
    }
    fn set_focus(&mut self, focused: bool, visible: bool) {
        let mut state = self.0.borrow_mut();
        if !focused {
            state.original.blur();
            state.captured = false;
        } else if let Some(index) = state.original.focus_index() {
            state.original.set_focus(Some(index), visible);
        }
    }
    fn set_focus_from_traversal(&mut self, reverse: bool) {
        let mut state = self.0.borrow_mut();
        let count = state.original.focus_order().len();
        state.original.set_focus(
            if count == 0 {
                None
            } else {
                Some(if reverse { count - 1 } else { 0 })
            },
            true,
        );
    }
    fn focus_boundary(&mut self, reverse: bool) -> ContentResponse {
        let mut state = self.0.borrow_mut();
        let count = state.original.focus_order().len();
        let edge = state.original.focus_index().is_some_and(|index| {
            if reverse {
                index == 0
            } else {
                index + 1 >= count
            }
        });
        if edge {
            return ContentResponse::default();
        }
        ContentResponse::consumed(state.original.handle_key(ControlKey::Tab { reverse }, true))
    }
    fn key_event(&mut self, event: &ContentKeyEvent) -> ContentResponse {
        if event.repeat {
            return ContentResponse::default();
        }
        event.key.map_or(ContentResponse::default(), |key| {
            if matches!(key, ControlKey::Tab { .. }) {
                return ContentResponse::default();
            }
            let handled = self.0.borrow_mut().original.handle_key(key, event.down);
            if handled {
                ContentResponse::consumed(true)
            } else {
                ContentResponse::default()
            }
        })
    }
    fn pointer_event(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if pointer.button.is_some_and(|b| b != PointerButton::Primary) {
            return ContentResponse::default();
        }
        let mut state = self.0.borrow_mut();
        let changed = match pointer.phase {
            PointerPhase::Down => {
                let hit = state
                    .original
                    .fixture_buttons()
                    .iter()
                    .chain(state.original.width_buttons())
                    .any(|button| {
                        button.layout().is_some_and(|layout| {
                            layout
                                .visible
                                .is_some_and(|rect| rect.contains(pointer.position))
                        })
                    });
                if !hit {
                    return ContentResponse::default();
                }
                state.captured = true;
                state.original.pointer_down(pointer.position)
            }
            PointerPhase::Up => {
                state.captured = false;
                state.original.pointer_up(pointer.position)
            }
            PointerPhase::Exit => state.original.pointer_move(Point::new(f32::NAN, f32::NAN)),
            _ => state.original.pointer_move(pointer.position),
        };
        ContentResponse {
            capture_pointer: state.captured,
            ..ContentResponse::consumed(changed)
        }
    }
    fn pointer_capture_current(&self) -> bool {
        self.0.borrow().captured
    }
    fn cancel_pointer(&mut self) -> ContentResponse {
        let mut state = self.0.borrow_mut();
        state.captured = false;
        state.original.cancel_pointer();
        ContentResponse::consumed(true)
    }
    fn window_active_changed(&mut self, active: bool) {
        if !active {
            let mut state = self.0.borrow_mut();
            state.captured = false;
            state.original.blur();
        }
    }
    fn lifecycle(&mut self, _: Lifecycle) {
        self.window_active_changed(false);
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Static text GPU calibration")
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        let state = self.0.borrow();
        state
            .original
            .fixture_buttons()
            .iter()
            .enumerate()
            .filter_map(|(i, model)| {
                model.layout().map(|layout| SemanticPart {
                    index: i,
                    focused: model.state.focused,
                    frame: layout.frame,
                    semantics: ContentSemantics::new(
                        ContentRole::Button,
                        static_text::FIXTURES[i].name,
                    ),
                })
            })
            .chain(
                state
                    .original
                    .width_buttons()
                    .iter()
                    .zip(state.original.widths())
                    .enumerate()
                    .filter_map(|(i, (model, width))| {
                        model.layout().map(|layout| SemanticPart {
                            index: static_text::FIXTURES.len() + i,
                            focused: model.state.focused,
                            frame: layout.frame,
                            semantics: ContentSemantics::new(
                                ContentRole::Button,
                                width.to_string(),
                            ),
                        })
                    }),
            )
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if !matches!(action, AccessibilityAction::Press) {
            return ContentResponse::default();
        }
        let Some(part) = part else {
            return ContentResponse::default();
        };
        let mut state = self.0.borrow_mut();
        let changed = if part < static_text::FIXTURES.len() {
            state.original.select_fixture(part)
        } else {
            let Some(width) = state
                .original
                .widths()
                .get(part - static_text::FIXTURES.len())
                .copied()
            else {
                return ContentResponse::default();
            };
            state.original.select_width(width)
        };
        ContentResponse::consumed(changed)
    }
}

#[cfg(test)]
mod calibration_tests {
    use super::calibration::CalibrationText;
    use super::*;
    #[test]
    fn calibration_visible_fixture_keeps_fixed_metrics_in_a_different_font_registry() {
        let mut original = StaticText::new(None);
        let mixed = static_text::FIXTURES
            .iter()
            .position(|fixture| fixture.name == "mixed-cjk-latin")
            .unwrap();
        original.select_fixture(mixed);
        original.select_width(320);
        let mut measurement = TextEngine::new();
        let expected = original.shaped(&mut measurement);
        let mut window = TextEngine::new();
        // Reachable collision scenario: the window has already shaped unrelated
        // typography before this calibration page mounts.
        window.shape(
            "Earlier window text",
            &tekes_ui_text::TextStyle::named("Menlo", 23.).with_weight(700),
            &tekes_ui_text::ShapeOptions::default(),
        );
        window.set_type_scale(1.5);
        let mut fixtures = HashMap::new();
        let mut bridge = CalibrationHostText {
            text: &mut window,
            fixtures: &mut fixtures,
            locale: Locale::En,
            clip: Rect::new(0., 0., 320., 170.),
        };
        let visible = bridge.shaped_fixture(&mut original);
        assert_eq!(
            visible
                .paragraph
                .lines
                .iter()
                .map(|line| line.utf16.clone())
                .collect::<Vec<_>>(),
            expected
                .paragraph
                .lines
                .iter()
                .map(|line| line.utf16.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(visible.paragraph.height, expected.paragraph.height);
        assert_eq!(visible.crop_pt, expected.crop_pt);
        assert_eq!(visible.shown_lines, expected.shown_lines);
        assert_eq!(visible.canvas_h, expected.canvas_h);
        // FontKey numeric identity is intentionally not an inter-registry contract.
    }
}

#[cfg(test)]
mod calibration_gpu_tests {
    use super::*;
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Theme;

    /// Resource-switch regression, not an independent text-raster acceptance oracle.
    /// Run serially in the coordinator's GPU window with --ignored --nocapture.
    #[test]
    #[ignore = "requires coordinator-owned finite GPU window; optional independent TEKESUI_REFERENCE_DIR"]
    fn calibration_shared_renderer_restores_window_text_and_scopes_reference_resources() {
        let frame = Rect::new(0., 0., 800., 900.);
        let palette: Palette = Theme::LIGHT.into();
        let clear = Rgba::new(1., 1., 1., 1.);
        let mut renderer = Renderer::new_headless().expect("actual GPU required");
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        // This is the real receiving ContentHost/atlas, not the measurement engine.
        let mut window = ContentHost::new(Content::new(
            ComponentId(99990),
            TextContent::new("Window text before calibration · 中文 e\u{301} 👨‍👩‍👧‍👦"),
        ))
        .unwrap();
        let mut window_scene = Scene::new();
        text.begin_frame();
        window
            .render(&mut window_scene, &mut text, frame, frame, palette)
            .unwrap();
        let before = renderer
            .render_to_rgba8(&window_scene, text.atlas_mut(), (800, 900), 1., clear)
            .unwrap();
        assert!(
            before.chunks_exact(4).any(|p| p[..3] != [255, 255, 255]),
            "window text must contribute pixels"
        );

        let mut page = CalibrationPage::new(Locale::En);
        let mut host = ContentHost::new(page.content()).unwrap();
        page.prepare_resources(&mut ContainerRenderContext {
            renderer: &mut renderer,
            frame,
            clip: frame,
            scale: 1.,
            motion: None,
            palette,
        })
        .unwrap();
        let computations = page.0.borrow().original.computations();
        let receipt = page
            .0
            .borrow()
            .original
            .result(Theme::LIGHT, 1.)
            .unwrap()
            .clone();

        // Force the actual original measurement atlas upload even when no independent
        // reference directory exists. These pixels are never saved as a reference,
        // compared to themselves, or used as proof of calibration accuracy.
        {
            let mut state = page.0.borrow_mut();
            let CalibrationState {
                original,
                measurement,
                ..
            } = &mut *state;
            let shaped = original.shaped(measurement);
            let pixels = measurement
                .render_paragraph_offscreen(
                    &mut renderer,
                    &shaped.paragraph,
                    (original.width(), shaped.canvas_h),
                    1.,
                    receipt.colors.fg(),
                    receipt.colors.bg(),
                )
                .unwrap();
            assert_eq!(
                pixels.len(),
                original.width() as usize * shaped.canvas_h as usize * 4
            );
        }
        let mut calibration_scene = Scene::new();
        text.begin_frame();
        host.render(&mut calibration_scene, &mut text, frame, frame, palette)
            .unwrap();
        let calibration_pixels = renderer
            .render_to_rgba8(&calibration_scene, text.atlas_mut(), (800, 900), 1., clear)
            .unwrap();
        let registered = renderer.image_count();
        match &receipt.status {
            static_text::ReferenceStatus::Loaded => {
                let reference = receipt
                    .reference_image
                    .expect("loaded reference must have a registered image");
                let heat = receipt
                    .heat_image
                    .expect("loaded reference must have a registered heatmap");
                assert!(receipt.numbers.is_some(), "actual full-canvas diff receipt");
                for id in [reference, heat] {
                    assert!(
                        calibration_scene
                            .primitives()
                            .iter()
                            .any(|p| matches!(p, Primitive::Image(image) if image.image == id)),
                        "resource must be in this same frame"
                    );
                    let mut without = Scene::new();
                    for primitive in calibration_scene
                        .primitives()
                        .iter()
                        .filter(|p| !matches!(p, Primitive::Image(image) if image.image == id))
                    {
                        without.push(primitive.clone());
                    }
                    let omitted = renderer
                        .render_to_rgba8(&without, text.atlas_mut(), (800, 900), 1., clear)
                        .unwrap();
                    assert_ne!(
                        calibration_pixels, omitted,
                        "actual reference/heatmap must contribute visible pixels"
                    );
                }
                eprintln!(
                    "INDEPENDENT_REFERENCE_OBSERVED: {:?}; numeric diff receipt is reported, not automatically accepted",
                    receipt.numbers
                );
            }
            static_text::ReferenceStatus::RenderFailed { reason } => {
                panic!("actual calibration GPU readback failed: {reason}")
            }
            status => eprintln!(
                "INDEPENDENT_REFERENCE_NOT_VERIFIED: {status:?}; atlas-switch regression still exercises real offscreen GPU rendering"
            ),
        }
        page.prepare_resources(&mut ContainerRenderContext {
            renderer: &mut renderer,
            frame,
            clip: frame,
            scale: 1.,
            motion: None,
            palette,
        })
        .unwrap();
        assert_eq!(
            page.0.borrow().original.computations(),
            computations,
            "same renderer/key must use its original receipt"
        );
        assert_eq!(
            renderer.image_count(),
            registered,
            "repeat prepare must not register duplicate images"
        );

        let mut restored_scene = Scene::new();
        text.begin_frame();
        window
            .render(&mut restored_scene, &mut text, frame, frame, palette)
            .unwrap();
        let restored = renderer
            .render_to_rgba8(&restored_scene, text.atlas_mut(), (800, 900), 1., clear)
            .unwrap();
        assert_eq!(
            before, restored,
            "measurement atlas must not corrupt the receiving window atlas"
        );

        let old_identity = renderer.resource_identity();
        let mut replacement = Renderer::new_headless().expect("replacement GPU renderer required");
        let sentinel = replacement
            .register_image(&[255, 0, 255, 255], 1, 1)
            .unwrap();
        assert_ne!(replacement.resource_identity(), old_identity);
        page.prepare_resources(&mut ContainerRenderContext {
            renderer: &mut replacement,
            frame,
            clip: frame,
            scale: 1.,
            motion: None,
            palette,
        })
        .unwrap();
        let state = page.0.borrow();
        assert_eq!(state.renderer, Some(replacement.resource_identity()));
        assert_eq!(
            state.original.computations(),
            computations + 1,
            "old renderer receipt must be discarded"
        );
        let current = state.original.result(Theme::LIGHT, 1.).unwrap();
        assert_eq!(
            current.key, receipt.key,
            "resource replacement preserves original fixture selection"
        );
        if current.status == static_text::ReferenceStatus::Loaded {
            assert_ne!(current.reference_image, Some(sentinel));
            assert_ne!(current.heat_image, Some(sentinel));
            assert_eq!(
                replacement.image_count(),
                3,
                "replacement owns sentinel plus its two new images"
            );
        } else {
            assert_eq!(replacement.image_count(), 1);
        }
        assert!(
            replacement.unregister_image(sentinel),
            "foreign old image IDs must never unregister a receiving renderer's resource"
        );
    }
}
