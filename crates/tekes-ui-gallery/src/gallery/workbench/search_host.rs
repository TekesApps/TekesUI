//! Workbench Search, Field/Area and Combo transport. The original ContentHost owns every edit,
//! native key frame, focus/capture and action; this adapter supplies transport.
use tekes_ui_controls::{
    DraftRevision, ExternalSelection, ExternalUndo, ExternalValueResult, composition::*,
};
use tekes_ui_platform_macos::TextInputState;

pub(super) const SEARCH: ComponentId = ComponentId(8801);
#[derive(Debug)]
pub(in crate::gallery::workbench) struct StoryTextHost {
    pub(super) host: ContentHost,
    id: ComponentId,
    publication: u64,
    error: Option<String>,
}
impl StoryTextHost {
    pub(super) fn with_root(root: fn(Content) -> Content) -> Self {
        let mut search = SearchFieldContent::new("Search by name", "");
        search.set_submit_policy(TextSubmitPolicy::Enter);
        let host = ContentHost::new(root(Content::new(SEARCH, search)))
            .expect("fixed Search story structure");
        Self::from_host(host, SEARCH)
    }
    pub(super) fn text_field(value: &str, multiline: bool, root: fn(Content) -> Content) -> Self {
        let id = ComponentId(if multiline { 27502 } else { 27501 });
        let mut field = TextFieldContent::new("Project name", value);
        field.show_metadata = false;
        field.viewport = TextViewport::Fill;
        field.surface = tekes_ui_controls::TextSurface::new(value, multiline, false);
        let host = ContentHost::new(root(Content::new(id, field)))
            .expect("fixed Field/Area story structure");
        Self::from_host(host, id)
    }
    pub(super) fn combo_box(root: fn(Content) -> Content) -> Self {
        let id = ComponentId(27601);
        let combo = ComboBoxContent::new(
            "Combo options",
            ["Automatic", "Compact", "Comfortable"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        );
        Self::from_host(
            ContentHost::new(root(Content::new(id, combo))).expect("fixed Combo story structure"),
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
            publication: 0,
            error: None,
        }
    }
    fn original_input(&self) -> &tekes_ui_controls::TextInput {
        self.host
            .text_surface(self.id)
            .expect("retained original text owner")
            .input()
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
        self.original_input().text()
    }
    pub(super) fn blur(&mut self) {
        self.host.cancel();
        self.host.focus(None, false);
    }
    pub(super) fn focused(&self) -> bool {
        self.host.focused() == Some(self.id)
    }
    pub(super) fn state(&self) -> Option<TextInputState> {
        if !self.focused() {
            return None;
        }
        let key =
            tekes_ui_macos_bridge::content_accessibility::content_key(&self.host, self.id, None)?;
        tekes_ui_macos_bridge::content_accessibility::content_text_layouts(&self.host)
            .remove(&key)
            .map(|layout| layout.state)
    }
    pub(super) fn set_window_active(&mut self, active: bool) {
        self.host.set_window_active(active);
    }
    pub(super) fn set_disabled(&mut self, disabled: bool) {
        if self.is_combo() {
            self.combo_mut().set_disabled(disabled);
        } else if self.is_search() {
            if self.field().input().is_disabled() != disabled {
                self.field_mut().set_availability(false, disabled);
            }
        } else {
            self.text_field_mut()
                .surface
                .input_mut()
                .set_disabled(disabled);
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
            field.surface.set_error(error);
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
