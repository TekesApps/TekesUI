use super::super::text_demo::TextDemo;
use super::*;
use search_input::StoryInput;
use select_story::SelectStoryHost;
use std::collections::VecDeque;
use std::ops::Add;
use std::time::Duration;
use tekes_ui_controls::{
    CheckState, Checkbox, Field, NumberField,
    feedback_view::{DialogEvent, DialogView, MenuEvent, MenuView, NumberFieldView, ToolbarView},
    layout_view::TabsView,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Invalid,
    Pending,
    Failed,
    Saved,
}
#[derive(Debug)]
pub struct StoryState {
    pub(super) input: StoryInput,
    pub(super) detail: TextDemo,
    pub checkbox: Checkbox,
    pub(super) tabs_view: TabsView,
    pub(super) menu_view: MenuView,
    pub(super) select_story: SelectStoryHost,
    pub(super) dialog_view: DialogView,
    pub number: NumberField,
    pub(super) field: Field,
    pub(super) toolbar_view: ToolbarView,
    pub(super) number_view: NumberFieldView,
    pub phase: Phase,
    pub(super) attempts: usize,
    pub(super) scene3d_reset: bool,
    pub(super) draft_fixture: Option<durable_draft::DurableDraft>,
    pub(super) draft_error: Option<String>,
    pub(super) draft_attempt: Option<(u64, u64)>,
    pub(super) focus: Option<usize>,
    pub section: usize,
    pub(super) disabled: bool,
    pub(super) loading: bool,
    pub(super) error: bool,
    pub(super) select_empty: bool,
    pub(super) log: VecDeque<String>,
    pub(super) document_bottom: f32,
    pub(super) selected: usize,
    pub(super) master: Option<master_host::MasterHost>,
    pub(super) window_active: bool,
    pub(super) extended: extended::ExtendedStory,
}
impl Default for StoryState {
    fn default() -> Self {
        Self {
            input: TextDemo::field("Studio", false, false).into(),
            detail: TextDemo::field(
                include_str!("../../../../../fixtures/text/gallery-reading-en.txt"),
                true,
                true,
            ),
            checkbox: Checkbox::default(),
            tabs_view: TabsView::new(3),
            menu_view: MenuView::new(3),
            select_story: SelectStoryHost::new(),
            dialog_view: DialogView::default(),
            number: NumberField::new(8., 1., 32., 1.).unwrap(),
            field: Field {
                label: "Workspace name".into(),
                help: "Use a clear name for your workspace.".into(),
                error: None,
                required: true,
            },
            toolbar_view: ToolbarView::new(3, 4.),
            number_view: NumberFieldView::default(),
            phase: Phase::Ready,
            attempts: 0,
            scene3d_reset: false,
            draft_fixture: None,
            draft_error: None,
            draft_attempt: None,
            focus: None,
            section: 1,
            disabled: false,
            loading: false,
            error: false,
            select_empty: false,
            log: VecDeque::new(),
            document_bottom: 0.,
            selected: 0,
            master: None,
            window_active: true,
            extended: extended::ExtendedStory::default(),
        }
    }
}
impl StoryState {
    pub(super) fn set_window_active(&mut self, active: bool, _now: Duration) {
        self.window_active = active;
        self.input.set_window_active(active);
        if let Some(master) = &mut self.master {
            master.host.set_window_active(active);
        }
        self.detail.set_window_active(active);
        self.tabs_view.set_window_active(active);
        let menu_event = self.menu_view.set_window_active(active);
        self.menu_event(menu_event, "menu", false);
        self.select_story.set_window_active(active);
        self.dialog_view.set_window_active(active);
        self.extended.set_window_active(active);
        if !active {
            // A dismissed transient must not require an inactive redraw to
            // discard retained exit paint. The persistent Dialog below keeps
            // its logical scope and is intentionally not treated as a Menu.
            self.menu_view.clear_presentation();
            self.toolbar_view.focus(None, false);
            self.number_view.blur();
            self.dialog_view.deactivate();
            self.extended.cancel_pointer();
            if let Some(master) = &mut self.master {
                master.host.cancel();
            }
            self.tabs_view.cancel();
        }
    }
    pub(super) fn for_entry(e: &Entry) -> Self {
        Self::for_entry_with_root(e, std::convert::identity)
    }
    pub(super) fn for_entry_with_root(
        e: &Entry,
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) -> Self {
        let mut s = Self::default();
        s.extended = extended::ExtendedStory::default();
        s.master = (e.renderer() == "master").then(master_host::MasterHost::default);
        s.input = match e.renderer() {
            "field" => StoryInput::field("Studio", false, root),
            "area" => StoryInput::field(
                "A quiet room, an open window.\nWrite a complete paragraph here. 中日文と English.",
                true,
                root,
            ),
            "search" => StoryInput::search_with_root(root),
            "combo-box" => StoryInput::combo_box(root),
            "filter" | "command-palette" => TextDemo::field("", false, false).into(),
            _ => s.input,
        };
        if e.renderer() == "select" {
            s.select_story = SelectStoryHost::with_root(root);
        }
        if e.renderer() == "form" {
            s.draft_fixture = Some(durable_draft::DurableDraft::memory());
        }
        // Unit tests inject temporary storage explicitly; never inherit a
        // developer's runtime fixture file through the process environment.
        if !cfg!(test)
            && e.renderer() == "form"
            && let Some(path) = std::env::var_os("TEKESUI_GALLERY_DRAFT_FIXTURE")
        {
            s.open_draft_fixture(path.into());
        }
        s
    }
    pub(super) fn open_draft_fixture(&mut self, path: std::path::PathBuf) {
        match durable_draft::DurableDraft::open(path) {
            Ok(store) => {
                self.input.set_value(&store.draft().name);
                self.checkbox.checked = if store.draft().notifications {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                self.phase = if store.has_pending() {
                    Phase::Failed
                } else if store.is_acknowledged() {
                    Phase::Saved
                } else {
                    Phase::Ready
                };
                self.draft_fixture = Some(store);
                self.draft_error = None;
                self.draft_attempt = None;
                self.record("draft fixture: restored from explicit storage; no network");
            }
            Err(error) => self.draft_failed(error),
        }
    }
    pub(super) fn draft_failed(&mut self, error: std::io::Error) {
        self.draft_error = Some(format!(
            "Draft storage failed ({:?}); input retained. Retry save.",
            error.kind()
        ));
        self.phase = Phase::Failed;
        self.record("draft: storage failed; not acknowledged");
    }
    pub(super) fn persist_draft(&mut self) -> bool {
        if self.input.state().is_some_and(|s| s.marked.is_some()) {
            return true;
        }
        // An explicitly requested file adapter must not silently fall back to
        // the default in-memory journal after configuration/storage failure.
        if self.draft_error.is_some()
            && self.draft_fixture.as_ref().is_some_and(|s| !s.is_durable())
        {
            return false;
        }
        let name = self.input.query().to_owned();
        let notifications = self.checkbox.checked == CheckState::Checked;
        let Some(store) = self.draft_fixture.as_mut() else {
            return self.draft_error.is_none();
        };
        match store.update(&name, notifications) {
            Ok(changed) => {
                if changed {
                    self.draft_error = None;
                }
                if changed && self.phase != Phase::Pending {
                    self.phase = Phase::Ready;
                }
                true
            }
            Err(error) => {
                self.draft_failed(error);
                false
            }
        }
    }
    pub(super) fn interrupt_draft(&mut self) {
        if !self.persist_draft() {
            return;
        }
        let Some(store) = self.draft_fixture.as_ref() else {
            return;
        };
        match store.reopen() {
            Ok(restored) => {
                self.input.set_value(&restored.draft().name);
                self.checkbox.checked = if restored.draft().notifications {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                self.phase = if restored.has_pending() {
                    Phase::Failed
                } else if restored.is_acknowledged() {
                    Phase::Saved
                } else {
                    Phase::Ready
                };
                self.draft_fixture = Some(restored);
                self.draft_attempt = None;
                self.record("interruption: discarded session; recovered durable draft/outbox");
            }
            Err(error) => self.draft_failed(error),
        }
    }
    pub(super) fn input_changed(&mut self, renderer: &str) {
        if renderer == "command-palette" {
            self.extended.command.combo.set_query(self.input.query());
        }
        self.record("input: value changed");
    }
    pub(super) fn record(&mut self, event: impl Into<String>) {
        if self.log.len() == 6 {
            self.log.pop_front();
        }
        self.log.push_back(event.into());
    }
    pub(super) fn menu_event(&mut self, event: MenuEvent, renderer: &str, select: bool) {
        if select {
            return;
        }
        match event {
            MenuEvent::Activated {
                index,
                restore_focus,
            } => {
                self.focus = restore_focus;
                self.record(format!("menu action: {index}"));
                let _ = renderer;
            }
            MenuEvent::Dismissed { restore_focus } => {
                self.focus = restore_focus;
            }
            MenuEvent::None => {}
        }
    }

    pub(super) fn dialog_title(renderer: &str) -> &'static str {
        if renderer == "danger" {
            "Delete this workspace? This fixture does not delete files."
        } else {
            "Confirm changes?"
        }
    }
    pub(super) fn dialog_event(
        &mut self,
        event: DialogEvent,
        renderer: &str,
        composition_guarded: bool,
    ) {
        match event {
            DialogEvent::Confirmed { restore_focus } => {
                self.focus = restore_focus;
                if renderer == "danger" {
                    if composition_guarded {
                        self.record("submit: deferred while text composition is active");
                    } else {
                        self.submit();
                    }
                }
                self.record("dialog: confirm; focus restored");
            }
            DialogEvent::Cancelled { restore_focus } => {
                self.focus = restore_focus;
                self.record("dialog: cancel; focus restored");
            }
            DialogEvent::Blocked | DialogEvent::None => {}
        }
    }
    pub(super) fn submit(&mut self) {
        if self.phase == Phase::Pending || self.input.state().is_some_and(|s| s.marked.is_some()) {
            return;
        }
        if self.input.query().trim().is_empty() {
            self.phase = Phase::Invalid;
            self.record("validation: name is required");
        } else {
            if !self.persist_draft() {
                return;
            }
            if let Some(store) = self.draft_fixture.as_mut() {
                if store.is_acknowledged() {
                    self.phase = Phase::Saved;
                    return;
                }
                match store.begin_attempt() {
                    Ok(attempt) => {
                        self.draft_attempt = Some(attempt);
                        self.draft_error = None;
                    }
                    Err(error) => {
                        self.draft_failed(error);
                        return;
                    }
                }
            }
            self.phase = Phase::Pending;
            self.attempts += 1;
            self.record("submit: pending (Resolve fixture to complete)");
        }
    }
    pub(super) fn resolve(&mut self) {
        if self.phase != Phase::Pending {
            return;
        }
        if let Some((revision, attempt)) = self.draft_attempt.take() {
            if attempt == 1 {
                self.phase = Phase::Failed;
                self.record("delivery: simulated failure; durable revision remains retryable");
            } else if let Some(store) = self.draft_fixture.as_mut() {
                match store.acknowledge(revision) {
                    Ok(true) => {
                        self.draft_error = None;
                        self.phase = if store.is_acknowledged() {
                            Phase::Saved
                        } else {
                            Phase::Ready
                        };
                        self.record(format!(
                            "delivery: acknowledged revision {revision}; newer edits retained"
                        ));
                    }
                    Ok(false) => {
                        self.phase = Phase::Failed;
                        self.record("delivery: stale acknowledgement ignored");
                    }
                    Err(error) => self.draft_failed(error),
                }
            }
            return;
        }
        self.phase = if self.attempts == 1 {
            Phase::Failed
        } else {
            Phase::Saved
        };
        self.record(if self.phase == Phase::Failed {
            "result: simulated failure; retry is available"
        } else {
            "result: saved"
        });
    }
    pub(super) fn activate(&mut self, id: usize, renderer: &str) {
        self.activate_with_root(id, renderer, std::convert::identity);
    }
    pub(super) fn activate_with_root(
        &mut self,
        id: usize,
        renderer: &str,
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) {
        // Story tabs belong to the workbench host for every renderer.
        // Dispatch them before the component-specific adapter.
        if (1..=6).contains(&id) {
            self.section = id - 1;
            self.document_bottom = 0.;
            self.input.blur();
            return;
        }
        if renderer == "tokens" && (400..=401).contains(&id) {
            self.selected = id - 400;
            return;
        }
        if renderer == "toolbar" && (20..=22).contains(&id) {
            self.record(format!(
                "toolbar: {}",
                ["create", "rename", "remove"][id - 20]
            ));
            return;
        }
        if (10000..10080).contains(&id) {
            self.selected = id - 10000;
            self.detail = TextDemo::field(
                &format!(
                    "Document {} — reading notes\n\n{}",
                    self.selected + 1,
                    include_str!("../../../../../fixtures/text/gallery-reading-en.txt")
                ),
                true,
                true,
            );
            self.record(format!("selection: {}", self.selected));
            return;
        }
        if extended::supports(renderer) {
            if id == 0 {
                let section = self.section;
                let active = self.window_active;
                *self = Self::for_entry_renderer_with_root(renderer, root);
                self.set_window_active(active, Duration::ZERO);
                self.section = section;
                self.record("reset: initial fixture");
                return;
            }
            let mut events = Vec::new();
            let changed = self
                .extended
                .activate(renderer, id, &mut |event| events.push(event));
            for event in events {
                self.record(event);
            }
            if changed && renderer == "command-palette" && id == 20 {
                self.input.set_value("");
            }
            if changed && renderer == "command-palette" {
                let (selected, items) = (
                    self.extended.command.combo.selected,
                    &self.extended.command.combo.items,
                );
                if let Some(index) = selected
                    && let Some(value) = items.get(index)
                {
                    self.input.set_value(value);
                }
            }
            return;
        }
        match id {
            0 => {
                let section = self.section;
                let active = self.window_active;
                *self = Self::for_entry_renderer_with_root(renderer, root);
                self.set_window_active(active, Duration::ZERO);
                self.section = section;
                self.scene3d_reset = renderer == "scene3d";
                self.record("reset: initial fixture");
            }
            10 => {
                self.disabled = !self.disabled;
                self.record(format!("disabled: {}", self.disabled));
            }
            11 => {
                self.loading = !self.loading;
                self.record(format!("loading: {}", self.loading));
            }
            12 => {
                self.error = !self.error;
            }
            14 => {
                self.input.set_value("A long workspace name with mixed scripts: 项目計画 and Japanese 日本語. Narrow containers should clip or wrap naturally.");
                self.record("fixture: long mixed text");
            }
            15 => {
                self.input.set_value("");
                self.record("fixture: empty text");
            }
            13 => {
                if renderer == "select" {
                    self.select_empty = !self.select_empty;
                } else {
                    self.checkbox.checked = CheckState::Mixed;
                }
            }
            20 => match renderer {
                "checkbox" => {
                    self.checkbox.toggle();
                    self.record(format!("change: {:?}", self.checkbox.checked));
                }
                "menu" => {
                    self.menu_view.show(20);
                    self.record("menu: open");
                }
                "dialog" | "danger" => {
                    self.dialog_view.show(20);
                    self.record("dialog: open");
                }
                "select" => {
                    // Use the retained trigger path: opening from the model
                    // alone resets active focus to zero instead of the current value.
                    self.select_story.focus(false);
                    self.select_story.open();
                }
                "form" | "async" | "settings" => self.submit(),
                _ => self.record("activate: primary action"),
            },
            21 => self.resolve(),
            22 => {
                self.input.set_value("");
                self.record("filter: cleared");
            }
            23 => {
                self.number.increment(false);
                self.record(format!("value: {}", self.number.value()));
            }
            24 => {
                self.number.increment(true);
                self.record(format!("value: {}", self.number.value()));
            }
            25 => {
                self.checkbox.toggle();
                self.record(format!("preference: {:?}", self.checkbox.checked));
            }
            26 if renderer == "form" => self.interrupt_draft(),
            30..=32 if self.tabs_view.select(id - 30) => {
                self.record(format!("tab: {}", id - 30));
            }
            30..=32 => {}
            40..=42 => {
                self.selected = id - 40;
                self.detail = TextDemo::field(
                    [
                        "First document. A calm morning in the studio.\n\n",
                        "Second document. The afternoon light changes.\n\n",
                        "Third document. Evening brings a different rhythm.\n\n",
                    ][self.selected]
                        .to_owned()
                        .add(include_str!(
                            "../../../../../fixtures/text/gallery-reading-en.txt"
                        ))
                        .as_str(),
                    true,
                    true,
                );
                self.record(format!("selection: {}", self.selected));
            }
            50..=52 => {
                self.menu_event(
                    MenuEvent::Activated {
                        index: id - 50,
                        restore_focus: Some(20),
                    },
                    renderer,
                    false,
                );
            }
            60..=62 => {
                self.menu_event(
                    MenuEvent::Activated {
                        index: id - 60,
                        restore_focus: Some(20),
                    },
                    renderer,
                    true,
                );
            }
            70 => self.dialog_event(
                DialogEvent::Cancelled {
                    restore_focus: Some(20),
                },
                renderer,
                false,
            ),
            71 => self.dialog_event(
                DialogEvent::Confirmed {
                    restore_focus: Some(20),
                },
                renderer,
                false,
            ),
            _ => {}
        }
    }
    #[cfg(test)]
    pub(super) fn for_entry_renderer(renderer: &str) -> Self {
        Self::for_entry_renderer_with_root(renderer, std::convert::identity)
    }
    pub(super) fn for_entry_renderer_with_root(
        renderer: &str,
        root: fn(
            tekes_ui_controls::composition::Content,
        ) -> tekes_ui_controls::composition::Content,
    ) -> Self {
        if let Some(e) = entries().iter().find(|e| e.renderer() == renderer) {
            Self::for_entry_with_root(e, root)
        } else {
            Self::default()
        }
    }
}
