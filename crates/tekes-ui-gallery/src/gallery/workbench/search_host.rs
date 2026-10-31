//! Actual Workbench Search route. The original ContentHost owns every edit,
//! native key frame, focus/capture and action; this adapter supplies transport.
use super::super::{PageCtx, text::TextEngine};
use tekes_ui_controls::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, composition::*,
};
use tekes_ui_platform_macos::{Event, TextInputState};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::Palette;

mod native_projection;
use native_projection::NativeProjection;

pub(super) const SEARCH: ComponentId = ComponentId(8801);
#[derive(Debug)]
pub(super) struct SearchHost {
    pub(super) host: ContentHost,
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
impl SearchHost {
    pub(super) fn new() -> Self {
        let mut search = SearchFieldContent::new("Search by name", "");
        search.set_submit_policy(TextSubmitPolicy::Enter);
        let host =
            ContentHost::new(Content::new(SEARCH, search)).expect("fixed Search story structure");
        Self::from_host(host)
    }
    fn from_host(mut host: ContentHost) -> Self {
        host.set_action_events(true);
        host.set_focus_wrap(false);
        Self {
            host,
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
            .text_surface(SEARCH)
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
        self.host.focus(Some(SEARCH), true);
    }
    pub(super) fn blur(&mut self) {
        self.host.cancel();
        self.host.focus(None, false);
    }
    pub(super) fn focused(&self) -> bool {
        self.host.focused() == Some(SEARCH)
    }
    pub(super) fn has_pointer_capture(&self) -> bool {
        self.host.pointer_capture().is_some()
    }
    pub(super) fn frame_at(&self, index: usize) -> Option<Rect> {
        self.host
            .text_surface(SEARCH)
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
            tekes_ui_platform_macos::content_accessibility::content_key(&self.host, SEARCH, None)?;
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
        if self.field().input().disabled != disabled {
            self.field_mut().set_availability(false, disabled);
        }
    }
    pub(super) fn set_error(&mut self, error: bool) {
        self.field_mut()
            .set_error(error.then(|| "Validation: name is required (fixture)".into()));
    }
    pub(super) fn set_value(&mut self, value: &str) {
        if self.query() == value {
            return;
        }
        let Some(version) = self.publication.checked_add(1) else {
            self.error = Some("search: external revision capacity exhausted".into());
            return;
        };
        let baseline = self.field().input().external_baseline();
        let revision = DraftRevision {
            identity: "workbench-search".into(),
            version,
            payload: value.into(),
        };
        match self.field_mut().apply_query(
            &baseline,
            &revision,
            ExternalSelection::Reset,
            ExternalUndo::Reset,
        ) {
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
    pub(super) fn set_reduced_motion(&mut self, _reduced: bool) -> bool {
        false
    }
    pub(super) fn needs_animation(&self) -> bool {
        self.host.animation_frame_requested()
    }
    pub(super) fn animate(&mut self, _: f32) -> bool {
        self.needs_animation()
    }
    pub(super) fn refresh(&mut self, text: &mut TextEngine) {
        if let (Some(frame), Some(clip)) = (self.frame, self.clip) {
            self.field_mut().prepare_text_input(text, frame, clip);
        }
    }
    pub(super) fn measure_search_height(&mut self, text: &mut TextEngine) -> f32 {
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
        let locale = ctx.text.locale();
        let placeholder = locale
            .text("\u{6309}\u{540d}\u{79f0}\u{641c}\u{7d22}\u{2026}")
            .into_owned();
        self.field_mut().set_placeholder(Some(placeholder));
        // Existing Gallery story copy; accessible clear is owned by this Button.
        self.field_mut()
            .set_clear_label("Clear query")
            .expect("fixed nonempty story copy");
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
                if !inside
                    && !captured
                    && !matches!(
                        m.kind,
                        tekes_ui_platform_macos::MouseEventKind::Moved
                            | tekes_ui_platform_macos::MouseEventKind::Exited
                    )
                {
                    return None;
                }
                crate::reading_pointer::translate(*m)
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
    pub(super) fn drain_logs(&mut self) -> Vec<String> {
        let mut logs = Vec::new();
        if let Some(error) = self.error.take() {
            logs.push(error);
        }
        for record in self.host.drain_action_records() {
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
