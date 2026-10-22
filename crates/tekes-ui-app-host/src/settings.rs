//! Host assembly and async persistence only. Shared schema/draft/form live in Settings.
use crate::settings_undo::client::{Client, Job};
use anyhow::Result;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tekes_ui_module_runtime::component::ToolbarAction;
use tekes_ui_platform_macos::{Point, Size, TextInputState, WindowWake};
use tekes_ui_settings::{
    Draft, FileProvider, FormIntent, Schema, SettingsForm, SettingsSidebar, Snapshot, Submission,
};

#[path = "settings_back.rs"]
pub(super) mod settings_back;

/// Consumer supplies a fixed TOML definition and its own storage location.
#[derive(Clone, Debug)]
pub struct SettingsWindow {
    pub schema: Schema,
    pub file: PathBuf,
    pub display_mode: SettingsDisplayMode,
}
/// Settings presentation belongs to the window shell, not to the TOML schema.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsDisplayMode {
    #[default]
    Exclusive,
    Independent,
}
/// Compatibility name for the initial prototype configuration.
pub type SettingsOptions = SettingsWindow;
impl SettingsWindow {
    pub fn from_toml(source: &str, file: PathBuf) -> Result<Self> {
        Ok(Self {
            schema: Schema::parse(source)?,
            file,
            display_mode: SettingsDisplayMode::default(),
        })
    }
    pub fn display_mode(mut self, mode: SettingsDisplayMode) -> Self {
        self.display_mode = mode;
        self
    }
}
pub(crate) struct SettingsSurface {
    pub form: SettingsForm,
    pub schema: Schema,
    pub view_schema: Schema,
    pub sidebar: SettingsSidebar,
    pub back_button: Option<tekes_ui_controls::composition::ContentHost>,
    pub selected: String,
    pub draft: Draft,
    pub open: bool,
    provider: FileProvider,
    client: Option<Client>,
    ready: bool,
    pending: bool,
    restoring: bool,
    pub actions: Vec<ToolbarAction>,
    due: Option<Instant>,
    failed_generation: Option<u64>,
    close_after_save: bool,
    unresolved: Option<Submission>,
    message: String,
    serial: u64,
    last_edit_generation: u64,
}
impl SettingsSurface {
    pub fn new(
        options: SettingsOptions,
        provider: FileProvider,
        snapshot: Snapshot,
    ) -> Result<Self> {
        options.schema.validate_snapshot(&snapshot)?;
        let draft = Draft::new(snapshot);
        let selected = options
            .schema
            .items()
            .next()
            .map(|(id, _)| id)
            .ok_or_else(|| {
                anyhow::anyhow!("Settings requires Group -> Item -> Field declarations")
            })?;
        let view_schema = options.schema.for_item(&selected)?;
        let form = SettingsForm::new_autosave(&view_schema, &draft)?;
        let mut sidebar = SettingsSidebar::new(&options.schema)?;
        sidebar.select(&selected);
        let back_button = if options.display_mode == SettingsDisplayMode::Exclusive {
            Some(settings_back::button()?)
        } else {
            None
        };
        Ok(Self {
            back_button,
            form,
            schema: options.schema,
            view_schema,
            sidebar,
            selected,
            draft,
            provider,
            open: false,
            client: None,
            ready: false,
            pending: false,
            restoring: false,
            actions: vec![],
            due: None,
            failed_generation: None,
            close_after_save: false,
            unresolved: None,
            message: String::new(),
            serial: 0,
            last_edit_generation: 0,
        })
    }
    pub fn select(&mut self, item: &str) -> Result<()> {
        self.form.flush_numbers(&self.view_schema, &mut self.draft);
        if item != self.selected {
            self.view_schema = self.schema.for_item(item)?;
            self.form = SettingsForm::new_autosave(&self.view_schema, &self.draft)?;
            self.selected = item.into();
        }
        self.sidebar.select(item);
        self.refresh();
        Ok(())
    }
    pub fn breadcrumb(&self) -> String {
        let group_id = self.selected.split('/').next().unwrap_or("");
        let mut labels = vec![];
        let mut group = self.schema.groups.iter().find(|g| g.id == group_id);
        while let Some(current) = group {
            labels.push(current.label.clone());
            group = current
                .parent
                .as_ref()
                .and_then(|id| self.schema.groups.iter().find(|g| &g.id == id));
        }
        labels.reverse();
        if let Some((_, item)) = self.schema.items().find(|(id, _)| id == &self.selected) {
            labels.push(item.label.clone());
        }
        labels.join(" › ")
    }
    pub fn entry(&self, item: &str) -> Result<crate::navigation::Entry> {
        let (_, definition) = self
            .schema
            .items()
            .find(|(id, _)| id == item)
            .ok_or_else(|| anyhow::anyhow!("unknown Settings item"))?;
        static NEXT_VISIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let visit = NEXT_VISIT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        Ok(crate::navigation::Entry {
            id: format!("settings-{}-{time}-{visit}", std::process::id()),
            route: "settings-item".into(),
            params: serde_json::json!({"item_id":item,"title":definition.label}),
        })
    }
    pub fn navigation_options(&self) -> crate::NavigationOptions {
        crate::NavigationOptions {
            directory: self.provider.path().parent().unwrap().join("navigation"),
            history_id: format!("settings:{}", self.schema.namespace),
            initial_entries: vec![self.entry(&self.selected).expect("validated item")],
            view_command: None,
        }
    }
    pub fn show(&mut self) {
        self.open = true;
        self.refresh();
    }
    pub fn blocked(&self) -> bool {
        self.pending || self.unresolved.is_some()
    }
    pub fn restoring(&self) -> bool {
        self.pending && self.restoring
    }
    pub fn needs_frame(&self) -> bool {
        self.due.is_some() && !self.pending
    }
    pub fn close_requested(&mut self) -> bool {
        if !self.form.flush_numbers(&self.view_schema, &mut self.draft) {
            self.message = "请修正数值，或按 Escape 取消当前输入。".into();
            self.refresh();
            return false;
        }
        self.close_after_save = true;
        self.failed_generation = None;
        if self.blocked() || self.draft.dirty() {
            self.due = Some(Instant::now());
            false
        } else {
            self.open = false;
            true
        }
    }
    fn composition_active(&self) -> bool {
        self.form
            .host
            .focused_text()
            .is_some_and(|s| s.input.marked.is_some())
    }
    pub fn undo_action(&mut self, command: u64) -> Result<()> {
        // Toolbar focus can blur/commit a number before the command is drained.
        self.form
            .collect_number_changes(&self.view_schema, &mut self.draft);
        self.edited();
        if self.ready
            && !self.blocked()
            && !self.draft.dirty()
            && !self.composition_active()
            && !self.form.has_number_draft()
            && self
                .actions
                .iter()
                .any(|a| a.command.id == command && a.command.enabled)
        {
            // Retired TextField edit versions belong to the completed save, not
            // to later edits made while a restore receipt is being reconciled.
            self.begin_restore();
            self.client.as_ref().unwrap().send(Job::Step(command))?;
            self.pending = true;
            self.restoring = true;
            self.form.host.cancel();
            self.form.host.focus(None, false);
            self.message.clear();
        }
        Ok(())
    }
    fn begin_restore(&mut self) {
        debug_assert!(!self.draft.dirty());
        self.draft = Draft::new(self.draft.base.clone());
        self.last_edit_generation = self.draft.generation;
    }
    pub fn toolbar_actions(&self) -> Vec<ToolbarAction> {
        let enabled = self.ready
            && !self.blocked()
            && !self.draft.dirty()
            && !self.composition_active()
            && !self.form.has_number_draft();
        self.actions
            .iter()
            .cloned()
            .map(|mut a| {
                a.command.enabled &= enabled;
                a
            })
            .collect()
    }
    pub fn intents(&mut self, intents: Vec<FormIntent>, wake: WindowWake) -> Result<()> {
        for intent in intents {
            match intent {
                FormIntent::Apply if !self.pending => {
                    self.failed_generation = None;
                    self.due = Some(Instant::now());
                    if !self.ready
                        && self.unresolved.is_none()
                        && let Some(client) = &self.client
                    {
                        client.send(Job::Recover)?;
                        self.pending = true;
                    }
                }
                FormIntent::Cancel => {
                    self.close_requested();
                }
                // Autosave has no Reset or draft-discard command.
                _ => {}
            }
        }
        self.tick(wake)?;
        self.refresh();
        Ok(())
    }
    fn tick(&mut self, wake: WindowWake) -> Result<()> {
        if self.client.is_none() {
            self.client = Some(Client::start(
                self.provider.clone(),
                self.schema.namespace.clone(),
                wake,
            ));
        }
        if !self.ready || self.pending || self.composition_active() {
            return Ok(());
        }
        if self.due.is_none_or(|t| t > Instant::now()) {
            return Ok(());
        }
        self.due = None;
        if self.failed_generation == Some(self.draft.generation) {
            return Ok(());
        }
        if !self.draft.dirty() && self.unresolved.is_none() {
            return Ok(());
        }
        let query = self.unresolved.is_some();
        let submission = if let Some(task) = &self.unresolved {
            task.clone()
        } else {
            self.serial += 1;
            let id = format!(
                "settings-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos(),
                self.serial
            );
            match self.draft.submit(&self.schema, id) {
                Ok(task) => task,
                Err(error) => {
                    self.message = error.to_string();
                    self.failed_generation = Some(self.draft.generation);
                    self.close_after_save = false;
                    return Ok(());
                }
            }
        };
        self.client
            .as_ref()
            .unwrap()
            .send(Job::Save { submission, query })?;
        self.pending = true;
        self.message.clear();
        Ok(())
    }
    pub fn poll(&mut self, wake: WindowWake) -> Result<()> {
        while let Some(completion) = self.client.as_ref().and_then(|c| c.rx.try_recv().ok()) {
            self.pending = false;
            self.actions = completion.actions;
            match completion.result {
                Ok(snapshot) => {
                    self.ready = true;
                    if let Some(submission) = completion.submission {
                        self.draft.accept(&self.schema, &submission, snapshot)?;
                        if completion.restore || self.restoring {
                            self.form.replace_values(&self.view_schema, &self.draft);
                        }
                    } else {
                        anyhow::ensure!(
                            snapshot == self.draft.base,
                            "history recovery changed the Settings base"
                        );
                    }
                    self.restoring = false;
                    self.unresolved = None;
                    self.failed_generation = None;
                    self.message.clear();
                    if self.draft.dirty() {
                        self.due = Some(Instant::now() + Duration::from_millis(300));
                    }
                    if self.close_after_save && !self.draft.dirty() {
                        self.open = false;
                    }
                }
                Err(error) => {
                    self.message = format!("{} — Retry", error.message);
                    self.failed_generation = Some(self.draft.generation);
                    self.due = None;
                    self.close_after_save = false;
                    if error.unknown {
                        self.unresolved = completion.submission;
                    } else {
                        self.restoring = false;
                        if completion.submission.is_none() {
                            self.ready = false;
                        }
                    }
                }
            }
        }
        self.tick(wake)?;
        self.refresh();
        Ok(())
    }
    pub fn flush(&mut self, wake: WindowWake) -> Result<()> {
        self.form.flush_numbers(&self.view_schema, &mut self.draft);
        if self.draft.dirty() && self.failed_generation.is_none() {
            self.due = Some(Instant::now());
        }
        self.tick(wake)
    }
    pub fn edited(&mut self) {
        if self.draft.generation != self.last_edit_generation {
            self.last_edit_generation = self.draft.generation;
            self.failed_generation = None;
            if self.unresolved.is_none() {
                self.message.clear();
                self.due = Some(Instant::now() + Duration::from_millis(300));
            }
        }
        self.refresh();
    }
    fn refresh(&mut self) {
        self.form.save_error(
            (!self.message.is_empty()).then_some(self.message.as_str()),
            self.pending,
        );
    }
    pub fn text_state(&self) -> Option<TextInputState> {
        let surface = self.form.host.focused_text()?;
        let caret = surface.caret();
        let len = surface.input.len();
        Some(TextInputState {
            caret_index: surface.input.caret,
            text: surface.input.text.clone(),
            selection: surface.input.selection(),
            marked: surface.input.marked.clone(),
            read_only: surface.input.read_only,
            caret_origin: Point {
                x: f64::from(caret.min_x()),
                y: f64::from(caret.min_y()),
            },
            caret_size: Size {
                width: f64::from(caret.size.width),
                height: f64::from(caret.size.height),
            },
            carets: (0..=len)
                .map(|i| {
                    let r = surface.caret_doc(i);
                    let o = surface.origin();
                    (
                        i,
                        Point {
                            x: f64::from(o.x + r.min_x()),
                            y: f64::from(o.y + r.min_y()),
                        },
                        Size {
                            width: f64::from(r.size.width),
                            height: f64::from(r.size.height),
                        },
                    )
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod autosave_restore_tests {
    use super::*;
    #[test]
    fn undo_does_not_resurrect_retired_typing_but_keeps_later_edits() {
        let path = std::env::temp_dir()
            .join(format!("settings-restore-{}", std::process::id()))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("../../../examples/app-host-demo/settings.schema.toml"),
            path.clone(),
        )
        .unwrap();
        let provider = FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
        let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        let field = surface.schema.settings[1].clone();
        surface.draft.edit(&field, "110".into());
        let save = surface
            .draft
            .submit(&surface.schema, "save".into())
            .unwrap();
        let saved = surface.provider.commit(&save).unwrap();
        surface
            .draft
            .accept(&surface.schema, &save, saved.clone())
            .unwrap();
        surface.begin_restore();
        let target = Snapshot {
            revision: saved.revision + 1,
            last_commit_id: "undo".into(),
            overrides: Default::default(),
        };
        let restore = Submission::replacement(&saved, &target).unwrap();
        let mut newer = surface.draft.clone();
        surface
            .draft
            .accept(&surface.schema, &restore, target.clone())
            .unwrap();
        assert!(!surface.draft.dirty());
        assert_eq!(surface.draft.display(&field), "100");
        newer.edit(&field, "120".into());
        newer.accept(&surface.schema, &restore, target).unwrap();
        assert_eq!(newer.display(&field), "120");
        assert!(newer.dirty());
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
