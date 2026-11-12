//! Host assembly and async persistence only. Shared schema/draft/form live in Settings.
use crate::settings_undo::client::{Client, Job};
use anyhow::Result;
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
use tekes_ui_module_runtime::component::ToolbarAction;
use tekes_ui_platform_macos::{TextInputState, WindowWake};
use tekes_ui_settings::{
    Draft, Field, FieldType, FileProvider, FormIntent, Schema, SettingsForm, SettingsSidebar,
    Snapshot, Submission,
};
use tekes_ui_tokens::Appearance;

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

/// Explicit application-owned Settings service. Clone this owner into each
/// AppWindow to share one configuration writer, history service and native child.
/// Separate owners remain independent and obey the provider's single-writer lock.
#[derive(Clone, Debug)]
pub struct SettingsOwner(pub(crate) Rc<RefCell<SettingsOwnerState>>);
#[derive(Debug)]
pub(crate) struct SettingsOwnerState {
    pub options: SettingsWindow,
    pub provider: FileProvider,
    pub child: Option<tekes_ui_platform_macos::WindowHandle>,
    loading: Weak<()>,
    pub display: Option<Rc<RefCell<SettingsDisplayState>>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayPreferences {
    /// None means follow this window's real system appearance.
    pub appearance: Option<Appearance>,
    pub text_size_percent: f32,
}
/// Display values/subscribers have no reference to the owner or native child.
/// Independent Settings can keep this state after its final opener closes.
#[derive(Debug)]
pub(crate) struct SettingsDisplayState {
    schema: Schema,
    fields: (Field, Field),
    pub preferences: Option<DisplayPreferences>,
    revision: Option<u64>,
    pub wakes: Vec<Weak<WindowWake>>,
}
impl SettingsDisplayState {
    pub(crate) fn publish(display: &Rc<RefCell<Self>>, snapshot: &Snapshot) -> Result<()> {
        let wakes = {
            let mut state = display.borrow_mut();
            if state
                .revision
                .is_some_and(|revision| snapshot.revision <= revision)
            {
                return Ok(());
            }
            state.schema.validate_snapshot(snapshot)?;
            let draft = Draft::new(snapshot.clone());
            let appearance = match draft.display(&state.fields.0).as_str() {
                "system" => None,
                "light" => Some(Appearance::Light),
                "dark" => Some(Appearance::Dark),
                value => anyhow::bail!("unsupported appearance preference {value}"),
            };
            let percent: f32 = draft.display(&state.fields.1).parse()?;
            anyhow::ensure!(
                percent.is_finite() && percent > 0.,
                "text size percentage must be positive"
            );
            state.preferences = Some(DisplayPreferences {
                appearance,
                text_size_percent: percent,
            });
            state.revision = Some(snapshot.revision);
            state.wakes.retain(|wake| wake.strong_count() > 0);
            state
                .wakes
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        for wake in wakes {
            wake.wake();
        }
        Ok(())
    }
}
impl SettingsOwner {
    pub fn new(options: SettingsWindow) -> Result<Self> {
        let provider = FileProvider::open(options.file.clone(), options.schema.namespace.clone())?;
        Ok(Self(Rc::new(RefCell::new(SettingsOwnerState {
            options,
            provider,
            child: None,
            loading: Weak::new(),
            display: None,
        }))))
    }
    /// Bind consumer-defined enum (system/light/dark) and positive integer
    /// percentage fields to AppHost and Settings display. Only acknowledged
    /// snapshots change display; pending/invalid drafts keep their current style.
    /// Configure this binding before attaching the owner to AppWindows.
    pub fn with_display_preferences(self, appearance: &str, text_size: &str) -> Result<Self> {
        {
            let mut state = self.0.borrow_mut();
            let find = |id: &str| {
                state
                    .options
                    .schema
                    .settings
                    .iter()
                    .find(|field| field.id == id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("unknown display preference {id}"))
            };
            let appearance = find(appearance)?;
            let text_size = find(text_size)?;
            anyhow::ensure!(
                appearance.kind == FieldType::Enum
                    && appearance
                        .options
                        .iter()
                        .all(|option| matches!(option.value.as_str(), "system" | "light" | "dark")),
                "appearance preference requires system/light/dark enum values"
            );
            anyhow::ensure!(
                text_size.kind == FieldType::Integer && text_size.min.is_some_and(|min| min > 0.),
                "text size preference requires a positive integer percentage"
            );
            state.display = Some(Rc::new(RefCell::new(SettingsDisplayState {
                schema: state.options.schema.clone(),
                fields: (appearance, text_size),
                preferences: None,
                revision: None,
                wakes: vec![],
            })));
        }
        Ok(self)
    }
    /// Publish actual loaded/acknowledged snapshots; never reread disk after a save.
    pub(crate) fn publish_display(&self, snapshot: &Snapshot) -> Result<()> {
        let display = self.0.borrow().display.clone();
        if let Some(display) = display {
            SettingsDisplayState::publish(&display, snapshot)?;
        }
        Ok(())
    }
    /// The requesting Host alone retains the ticket. If it closes before a
    /// background read finishes, another window can request a fresh read.
    pub(crate) fn begin_load(&self) -> Option<Rc<()>> {
        let mut state = self.0.borrow_mut();
        if state.loading.upgrade().is_some() {
            return None;
        }
        let ticket = Rc::new(());
        state.loading = Rc::downgrade(&ticket);
        Some(ticket)
    }
}
pub(crate) enum SettingsConfiguration {
    Window(SettingsWindow),
    Shared(SettingsOwner),
}
pub(crate) struct SettingsSurface {
    pub form: SettingsForm,
    pub schema: Schema,
    pub view_schema: Schema,
    pub sidebar: SettingsSidebar,
    pub exclusive: bool,
    pub selected: String,
    pub draft: Draft,
    pub open: bool,
    provider: FileProvider,
    client: Option<Client>,
    ready: bool,
    pending: bool,
    restoring: bool,
    pub actions: Vec<ToolbarAction>,
    pub toolbar_source: Option<crate::toolbar_source::ComponentSource>,
    pub toolbar_window: u64,
    due: Option<Instant>,
    failed_generation: Option<u64>,
    close_after_save: bool,
    unresolved: Option<Submission>,
    message: String,
    serial: u64,
    last_edit_generation: u64,
    pub display: Option<Rc<RefCell<SettingsDisplayState>>>,
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
        let mut sidebar = if options.display_mode == SettingsDisplayMode::Exclusive {
            SettingsSidebar::new_with_back(&options.schema)?
        } else {
            SettingsSidebar::new(&options.schema)?
        };
        sidebar.select(&selected);
        let exclusive = options.display_mode == SettingsDisplayMode::Exclusive;
        Ok(Self {
            exclusive,
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
            toolbar_source: None,
            toolbar_window: 1,
            due: None,
            failed_generation: None,
            close_after_save: false,
            unresolved: None,
            message: String::new(),
            serial: 0,
            last_edit_generation: 0,
            display: None,
        })
    }
    pub fn navigation_input_ready(&self) -> bool {
        !self.composition_active() && self.form.can_flush_numbers()
    }
    pub fn accessibility_input(
        &mut self,
        publication: &tekes_ui_platform_macos::content_accessibility::WindowAccessibilityPublication,
        owner: tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner,
        target: tekes_ui_platform_macos::AccessibilityKey,
        action: tekes_ui_platform_macos::AccessibilityAction,
        sidebar_visible: bool,
    ) -> bool {
        use tekes_ui_platform_macos::AccessibilityAction;
        let response = if owner.0 == 1 && sidebar_visible {
            publication.dispatch(owner, &mut self.sidebar.host, target, action.clone())
        } else if owner.0 == 2 || owner.0 == u64::MAX {
            publication.dispatch(owner, &mut self.form.host, target, action.clone())
        } else {
            return false;
        };
        if response.consumed
            && matches!(
                action,
                AccessibilityAction::Focus | AccessibilityAction::Press
            )
        {
            // The current original publication must authorize delivery first.
            // Its accepted owner keeps the focus semantics of that exact action.
            if owner.0 == 1 {
                self.form.host.focus(None, false);
                if matches!(action, AccessibilityAction::Press)
                    && let Some(id) = self.sidebar.host.semantics().iter().find_map(|node| {
                        (self.sidebar.host.semantic_mount(node.id) == Some(target.component)
                            && self.sidebar.host.get(node.id).is_some_and(|n| {
                                n.component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                                    .is_some()
                            }))
                        .then_some(node.id)
                    })
                {
                    // Retain the original Settings List Press keyboard intent,
                    // after delivery, without overriding AX Focus visibility.
                    self.sidebar.host.focus(Some(id), false);
                }
            } else {
                self.sidebar.host.focus(None, false);
            }
        }
        response.consumed
    }
    pub fn prepare_item_navigation(&mut self) -> bool {
        self.prepare_navigation_input(false)
    }
    pub fn prepare_back_focus(&mut self) -> bool {
        self.prepare_navigation_input(true)
    }
    fn prepare_navigation_input(&mut self, back_focus: bool) -> bool {
        if self.composition_active() {
            self.message = if back_focus {
                "请先完成输入法输入，再关闭设置。"
            } else {
                "请先完成输入法输入，再切换设置。"
            }
            .into();
            self.refresh();
            return false;
        }
        if !self.form.flush_numbers(&self.view_schema, &mut self.draft) {
            // Other valid fields may already have committed. Preserve and
            // schedule their real Draft changes before restoring this feedback.
            self.edited();
            self.message = if back_focus {
                "请修正数值，或按 Escape 取消当前输入。"
            } else {
                "请修正数值，或按 Escape 取消当前输入，再切换设置。"
            }
            .into();
            self.refresh();
            return false;
        }
        self.edited();
        if self.failed_generation.is_none() && self.unresolved.is_none() {
            self.message.clear();
        }
        self.refresh();
        true
    }
    pub fn take_item_navigation(&mut self) -> Result<Option<crate::navigation::Entry>> {
        let Some(item) = self.sidebar.take_selection() else {
            return Ok(None);
        };
        if !self.prepare_item_navigation() {
            self.sidebar.select(&self.selected);
            return Ok(None);
        }
        self.entry(&item).map(Some)
    }
    pub fn select(&mut self, item: &str) -> Result<()> {
        // Only publish an admitted request. Navigation busy freezes editing
        // between request preparation and this original publication callback.
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
    pub fn clean_for_close(&self) -> bool {
        !self.blocked()
            && !self.draft.dirty()
            && !self.form.has_number_draft()
            && !self.composition_active()
    }
    pub fn close_requested(&mut self) -> bool {
        if self.composition_active() {
            self.close_after_save = false;
            self.message = "请先完成输入法输入，再关闭设置。".into();
            self.refresh();
            return false;
        }
        if !self.form.flush_numbers(&self.view_schema, &mut self.draft) {
            self.close_after_save = false;
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
        self.form.host.is_composing()
    }
    pub fn undo_action(
        &mut self,
        command: u64,
        source: &crate::toolbar_source::ComponentSource,
    ) -> Result<()> {
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
            self.client.as_ref().unwrap().send(Job::Step {
                command,
                source: source.clone(),
            })?;
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
                self.toolbar_window,
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
            self.toolbar_source = completion.source;
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
                    if self.close_after_save && self.clean_for_close() {
                        self.open = false;
                    }
                    if let Some(display) = &self.display {
                        SettingsDisplayState::publish(display, &self.draft.base)?;
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
        crate::columns::text_state(&self.sidebar.host)
            .or_else(|| crate::columns::text_state(&self.form.host))
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
