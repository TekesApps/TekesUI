//! Workbench Search, Field/Area and Combo transport. The original ContentHost owns every edit,
//! native key frame, focus/capture and action; this adapter supplies transport.
use super::super::{PageCtx, text::TextEngine};
use tekes_ui_controls::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, TextSurfaceHost,
    composition::*,
};
use tekes_ui_platform_macos::{Event, TextInputState};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::Palette;

pub(super) mod native_projection;
use native_projection::NativeProjection;

pub(super) const SEARCH: ComponentId = ComponentId(8801);
#[derive(Debug)]
pub(super) struct StoryTextHost {
    pub(super) host: ContentHost,
    id: ComponentId,
    frame: Option<Rect>,
    clip: Option<Rect>,
    publication: u64,
    fixture: bool,
    error: Option<String>,
    last_key: Option<TextKeyOutcome>,
    last_consumed: bool,
    pub(super) last_redraw: bool,
    projection: std::cell::RefCell<NativeProjection>,
}
impl StoryTextHost {
    pub(super) fn new() -> Self {
        let mut search = SearchFieldContent::new("Search by name", "");
        search.set_submit_policy(TextSubmitPolicy::Enter);
        let host =
            ContentHost::new(Content::new(SEARCH, search)).expect("fixed Search story structure");
        Self::from_host(host, SEARCH)
    }
    pub(super) fn text_field(value: &str, multiline: bool) -> Self {
        let id = ComponentId(if multiline { 27502 } else { 27501 });
        let mut field = TextFieldContent::new("Project name", value);
        field.show_metadata = false;
        field.viewport = TextViewport::Fill;
        field.surface = tekes_ui_controls::TextSurface::new(value, multiline, false);
        let host =
            ContentHost::new(Content::new(id, field)).expect("fixed Field/Area story structure");
        Self::from_host(host, id)
    }
    pub(super) fn combo_box() -> Self {
        let id = ComponentId(27601);
        let combo = ComboBoxContent::new(
            "Combo options",
            ["Automatic", "Compact", "Comfortable"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        );
        Self::from_host(
            ContentHost::new(Content::new(id, combo)).expect("fixed Combo story structure"),
            id,
        )
    }
    pub(super) fn is_combo(&self) -> bool {
        self.id == ComponentId(27601)
    }
    pub(super) fn combo(&self) -> &ComboBoxContent {
        self.host
            .get(self.id)
            .unwrap()
            .component::<ComboBoxContent>()
            .unwrap()
    }
    pub(super) fn combo_mut(&mut self) -> &mut ComboBoxContent {
        self.host
            .get_mut(self.id)
            .unwrap()
            .component_mut::<ComboBoxContent>()
            .unwrap()
    }
    pub(super) fn is_search(&self) -> bool {
        self.id == SEARCH
    }
    fn text_field_mut(&mut self) -> &mut TextFieldContent {
        self.host
            .get_mut(self.id)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
    }
    fn from_host(mut host: ContentHost, id: ComponentId) -> Self {
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self {
            host,
            id,
            frame: None,
            clip: None,
            publication: 0,
            fixture: false,
            error: None,
            last_key: None,
            last_consumed: false,
            last_redraw: false,
            projection: Default::default(),
        }
    }
    fn original_input(&self) -> &tekes_ui_controls::TextInput {
        &self
            .host
            .text_surface(self.id)
            .expect("retained original text owner")
            .input
    }
    fn field(&self) -> &SearchFieldContent {
        self.host
            .get(SEARCH)
            .unwrap()
            .component::<SearchFieldContent>()
            .unwrap()
    }
    fn field_mut(&mut self) -> &mut SearchFieldContent {
        self.host
            .get_mut(SEARCH)
            .unwrap()
            .component_mut::<SearchFieldContent>()
            .unwrap()
    }
    pub(super) fn query(&self) -> &str {
        &self.original_input().text
    }
    pub(super) fn focus_field(&mut self) {
        self.host.focus(Some(self.id), true);
    }
    pub(super) fn blur(&mut self) {
        self.host.cancel();
        self.host.focus(None, false);
    }
    pub(super) fn focused(&self) -> bool {
        self.host.focused() == Some(self.id)
    }
    pub(super) fn has_pointer_capture(&self) -> bool {
        self.host.pointer_capture().is_some()
    }
    pub(super) fn frame_at(&self, index: usize) -> Option<Rect> {
        self.host
            .text_surface(self.id)
            .filter(|_| index == 0)
            .map(|surface| surface.outer)
    }
    pub(super) fn visible_frame(&self) -> Option<Rect> {
        self.frame?
            .intersection(&self.clip?)
            .filter(|r| !r.is_empty())
    }
    pub(super) fn state(&self) -> Option<TextInputState> {
        if !self.focused() {
            return None;
        }
        let key =
            tekes_ui_platform_macos::content_accessibility::content_key(&self.host, self.id, None)?;
        tekes_ui_platform_macos::content_accessibility::content_text_layouts(&self.host)
            .remove(&key)
            .map(|layout| layout.state)
    }
    pub(super) fn set_window_active(&mut self, active: bool) {
        if self.host.is_window_active() != active {
            self.revoke_projection();
        }
        self.host.set_window_active(active);
    }
    pub(super) fn set_disabled(&mut self, disabled: bool) {
        if self.is_combo() {
            self.combo_mut().set_disabled(disabled);
        } else if self.is_search() {
            if self.field().input().disabled != disabled {
                self.field_mut().set_availability(false, disabled);
            }
        } else {
            self.text_field_mut().surface.input.disabled = disabled;
            if disabled {
                self.blur();
            }
        }
    }
    pub(super) fn set_error(&mut self, error: bool) {
        let message = error.then(|| "Validation: name is required (fixture)".into());
        if self.is_combo() {
            self.combo_mut().set_error(message);
        } else if self.is_search() {
            self.field_mut().set_error(message);
        } else {
            let field = self.text_field_mut();
            field.field.error = message;
            field.surface.error = error;
        }
    }
    pub(super) fn set_value(&mut self, value: &str) {
        if self.is_combo() {
            if let Err(error) = self.combo_mut().set_query(value) {
                self.error = Some(format!("combo query: {error:?}"));
            }
            return;
        }
        if self.query() == value {
            return;
        }
        let Some(version) = self.publication.checked_add(1) else {
            self.error = Some("search: external revision capacity exhausted".into());
            return;
        };
        let baseline = self.original_input().external_baseline();
        let revision = DraftRevision {
            identity: if self.is_search() {
                "workbench-search"
            } else {
                "workbench-field-area"
            }
            .into(),
            version,
            payload: value.into(),
        };
        let result = if self.is_search() {
            self.field_mut().apply_query(
                &baseline,
                &revision,
                ExternalSelection::Reset,
                ExternalUndo::Reset,
            )
        } else {
            self.text_field_mut().surface.apply_external_value(
                &baseline,
                &revision,
                ExternalSelection::Reset,
                ExternalUndo::Reset,
            )
        };
        match result {
            ExternalValueResult::Applied { .. } | ExternalValueResult::Unchanged => {
                self.publication = version
            }
            ExternalValueResult::Conflict(reason) => {
                self.error = Some(format!("search publication rejected: {reason:?}"))
            }
        }
    }
    // Fixture caching and motion remain on the original text backend/window clock.
    pub(super) fn use_fixture_cache(&mut self) {
        self.fixture = true;
    }
    pub(super) fn set_reduced_motion(&mut self, reduced: bool) -> bool {
        if self.is_search() || self.is_combo() {
            false
        } else {
            self.text_field_mut()
                .surface
                .scroll
                .set_reduced_motion(reduced)
        }
    }
    pub(super) fn needs_animation(&self) -> bool {
        self.host.animation_frame_requested()
            || (!self.is_search()
                && !self.is_combo()
                && self
                    .host
                    .text_surface(self.id)
                    .unwrap()
                    .scroll
                    .needs_animation())
    }
    pub(super) fn animate(&mut self, dt: f32) -> bool {
        if self.is_search() || self.is_combo() {
            self.needs_animation()
        } else {
            self.text_field_mut().surface.scroll.tick(dt)
        }
    }
    pub(super) fn refresh(&mut self, text: &mut TextEngine) {
        if let (Some(frame), Some(clip)) = (self.frame, self.clip) {
            if self.is_combo() {
                self.combo_mut().prepare_text_input(text, frame, clip);
            } else if self.is_search() {
                self.field_mut().prepare_text_input(text, frame, clip);
            } else {
                self.text_field_mut().prepare_text_input(text, frame, clip);
            }
        }
    }
    pub(super) fn measure_search_height(&mut self, text: &mut TextEngine) -> f32 {
        if self.is_combo() {
            let previous = text.replace_fixture_surface_cache(self.fixture);
            let height = self
                .combo_mut()
                .measure(text, Size::new(650., 1000.))
                .size
                .height;
            text.replace_fixture_surface_cache(previous);
            return height;
        }
        if !self.is_search() {
            let previous = text.replace_fixture_surface_cache(self.fixture);
            let surface = &self.text_field_mut().surface;
            let role = surface.role;
            let height = surface.measured_outer_height(
                650.,
                text.scale(),
                text.type_scale(),
                |source, width| text.shape_surface(source, role, width),
            );
            text.replace_fixture_surface_cache(previous);
            return height;
        }
        let previous = text.replace_fixture_surface_cache(self.fixture);
        let height = self
            .field_mut()
            .measure(text, Size::new(650., 1000.))
            .size
            .height;
        text.replace_fixture_surface_cache(previous);
        height
    }
    pub(super) fn build_search_clipped(
        &mut self,
        ctx: &mut PageCtx<'_>,
        frame: Rect,
        clip: Option<Rect>,
        scene: &mut Scene,
        brand: usize,
    ) {
        if self.is_search() {
            let locale = ctx.text.locale();
            let placeholder = locale
                .text("\u{6309}\u{540d}\u{79f0}\u{641c}\u{7d22}\u{2026}")
                .into_owned();
            self.field_mut().set_placeholder(Some(placeholder));
            // Existing Gallery story copy; accessible clear is owned by this Button.
            self.field_mut()
                .set_clear_label("Clear query")
                .expect("fixed nonempty story copy");
        }
        let clip = clip.unwrap_or(frame);
        self.frame = Some(frame);
        self.clip = Some(clip);
        let previous = ctx.text.replace_fixture_surface_cache(self.fixture);
        let result = self.host.render(
            scene,
            ctx.text,
            frame,
            clip,
            Palette::new(ctx.theme, brand).expect("registered Gallery brand"),
        );
        ctx.text.replace_fixture_surface_cache(previous);
        if let Err(e) = result {
            self.error = Some(format!("search layout: {e:?}"));
        }
        // Retain the existing empty/unfocused Gallery fixture copy, outside the editor value.
        if !self.is_search() && !self.is_combo() && self.query().is_empty() && !self.focused() {
            let surface = self.host.text_surface(self.id).unwrap();
            if let Some(visible) = surface.inner.intersection(&clip) {
                scene.push_clip(visible);
                ctx.text.proposal_label_in(
                    scene,
                    ctx.theme,
                    surface.inner,
                    "按名称搜索…",
                    tekes_ui_tokens::proposal::TextRole::UiBody,
                    tekes_ui_tokens::proposal::render_color("secondary-text", ctx.theme)
                        .expect("JSON text role"),
                );
                scene.pop_clip();
            }
        }
    }
    pub(super) fn command(
        &mut self,
        command: &str,
        clipboard: Option<&str>,
        text: &mut TextEngine,
    ) -> (bool, Option<String>) {
        if !matches!(command, "copy:" | "cut:" | "paste:") {
            let r = self.deliver(&ContentEvent::TextCommand(command.into()), text);
            return (r.changed, None);
        }
        let Some(snapshot) = self.host.claim_clipboard_input() else {
            return (false, None);
        };
        let selected =
            (!snapshot.selection.is_empty()).then(|| self.original_input().selected_text());
        if command == "copy:" {
            return (false, selected);
        }
        if snapshot.read_only {
            return (false, None);
        }
        if command == "cut:" {
            if selected.is_none() {
                return (false, None);
            }
            let r = self.deliver(&ContentEvent::TextCommand("deleteBackward:".into()), text);
            return (r.changed, selected);
        }
        let Some(value) = clipboard else {
            return (false, None);
        };
        let r = self.deliver(
            &ContentEvent::Commit {
                text: value.into(),
                replacement: None,
            },
            text,
        );
        (r.changed, None)
    }
    fn deliver(&mut self, input: &ContentEvent, text: &mut TextEngine) -> ContentResponse {
        let result = self.host.event_with_text(input, text);
        match result {
            Ok(r) => {
                self.last_consumed = r.consumed;
                if r.text_key.is_some() {
                    self.last_key = r.text_key;
                }
                r
            }
            Err(e) => {
                self.error = Some(format!("search input: {e:?}"));
                ContentResponse::default()
            }
        }
    }
    pub(super) fn handle(
        &mut self,
        event: &Event,
        _: Option<Point>,
        text: &mut TextEngine,
    ) -> Option<bool> {
        self.last_key = None;
        self.last_consumed = false;
        self.last_redraw = false;
        let focused = self.focused();
        let captured = self.host.pointer_capture().is_some();
        let input = match event {
            Event::Mouse(m) => {
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                let inside = self.visible_frame().is_some_and(|r| r.contains(p));
                let overlay = self.is_combo() && self.combo().overlay_open();
                if !inside
                    && !overlay
                    && !captured
                    && !matches!(
                        m.kind,
                        tekes_ui_platform_macos::MouseEventKind::Moved
                            | tekes_ui_platform_macos::MouseEventKind::Exited
                    )
                {
                    // Field/Area background clicks retain the original Gallery blur
                    // behavior, using the formal Host to release the editor focus.
                    if !self.is_search()
                        && focused
                        && matches!(
                            m.kind,
                            tekes_ui_platform_macos::MouseEventKind::Down {
                                button: tekes_ui_platform_macos::MouseButton::Left,
                                ..
                            }
                        )
                    {
                        self.blur();
                        self.last_redraw = true;
                        return Some(true);
                    }
                    return None;
                }
                crate::reading_pointer::translate(*m)
            }
            Event::Scroll(s) if !self.is_search() => {
                let wheel = tekes_ui_module_runtime::content_wheel_event(s)?;
                let result = self.host.wheel_event_with_text(&wheel, text);
                return match result {
                    Ok(response) => {
                        self.last_consumed = response.consumed;
                        self.last_redraw = response.changed;
                        Some(response.consumed)
                    }
                    Err(error) => {
                        self.error = Some(format!("field/area wheel: {error:?}"));
                        None
                    }
                };
            }
            Event::TextKey(k) => crate::reading_key::translate(k.clone()),
            Event::Key(k) if focused => ContentEvent::KeyInput(crate::reading_key::key(k.clone())),
            Event::TextCommit { text, replacement } if focused => ContentEvent::Commit {
                text: text.clone(),
                replacement: replacement.clone(),
            },
            Event::TextPreedit {
                text,
                selection,
                clauses,
                replacement,
            } if focused => ContentEvent::Preedit {
                text: text.clone(),
                selection: selection.clone(),
                clauses: clauses.clone(),
                replacement: replacement.clone(),
            },
            Event::TextUnmark if focused => ContentEvent::Unmark,
            Event::TextCommand(c) if focused => ContentEvent::TextCommand(c.clone()),
            Event::CaretBlink if focused => return Some(true),
            _ => return None,
        };
        let r = self.deliver(&input, text);
        // An original host with no local focus target lets its parent scope act.
        self.last_redraw = r.changed;
        Some(r.consumed)
    }
    #[cfg(test)]
    pub(super) fn scroll_metrics(&self) -> (f32, f32) {
        let surface = self.host.text_surface(self.id).unwrap();
        (surface.scroll.limit().y, surface.inner.size.height)
    }
    #[cfg(test)]
    pub(super) fn scroll_to(&mut self, offset: f32) {
        let surface = &mut self.text_field_mut().surface;
        surface
            .scroll
            .scroll_by(0., offset - surface.scroll.offset.y);
    }
    pub(super) fn drain_logs(&mut self) -> Vec<String> {
        let mut logs = Vec::new();
        if let Some(error) = self.error.take() {
            logs.push(error);
        }
        for record in self.host.drain_action_records() {
            if let Some(ContentActionPayload::ComboBoxSelection(selection)) = &record.payload {
                logs.push(format!("combo selection: {}", selection.index()));
            }
            if let Some(ContentActionPayload::Search(action)) = record.payload {
                match action.kind {
                    SearchCommandKind::Submit => {
                        logs.push(format!("search: submit {}", action.query))
                    }
                    SearchCommandKind::Clear => logs.push("filter: cleared".into()),
                }
            }
        }
        logs
    }
}
