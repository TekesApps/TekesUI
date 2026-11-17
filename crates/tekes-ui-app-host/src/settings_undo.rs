//! Headless saved-configuration history. The Provider remains the sole config writer.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
use tekes_ui_module_runtime::{ComponentDefinition, ComponentEmitter, ProcessModule, component::*};
use tekes_ui_settings::Snapshot;

pub const MODULE_ID: &str = "settings-undo";
pub const UNDO: u64 = 0x5501;
pub const REDO: u64 = 0x5502;
pub const MAX_JOURNAL_ENTRIES: usize = 128;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    Open {
        file: PathBuf,
        namespace: String,
        current: Snapshot,
    },
    Prepare {
        before: Snapshot,
        after: Snapshot,
    },
    Confirm {
        current: Snapshot,
    },
    Inspect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub current: Snapshot,
    pub prepared: Option<Snapshot>,
    pub can_undo: bool,
    pub can_redo: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Change {
    before: Snapshot,
    after: Snapshot,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Kind {
    Save,
    Undo,
    Redo,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pending {
    kind: Kind,
    change: Change,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    namespace: String,
    current: Snapshot,
    entries: Vec<Change>,
    cursor: usize,
    pending: Option<Pending>,
}
struct Store {
    file: PathBuf,
    _lock: File,
    journal: Journal,
}
impl Store {
    fn open(file: PathBuf, namespace: String, current: Snapshot) -> Result<Self> {
        let parent = parent_dir(&file);
        if parent != Path::new(".") {
            fs::create_dir_all(parent)?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(file.with_extension("undo.lock"))?;
        lock.try_lock()
            .context("Settings history already has a writer")?;
        let (existed, journal) = match fs::read(&file) {
            Ok(bytes) => (
                true,
                serde_json::from_slice::<Journal>(&bytes).context("invalid history; preserved")?,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
                false,
                Journal {
                    version: 1,
                    namespace: namespace.clone(),
                    current: current.clone(),
                    entries: vec![],
                    cursor: 0,
                    pending: None,
                },
            ),
            Err(e) => return Err(e.into()),
        };
        ensure!(
            journal.version == 1
                && journal.namespace == namespace
                && journal.cursor <= journal.entries.len(),
            "history identity/cursor mismatch"
        );
        for pair in journal.entries.windows(2) {
            ensure!(
                pair[0].after.overrides == pair[1].before.overrides,
                "discontinuous history"
            );
        }
        if let Some(entry) = journal.entries.get(journal.cursor.wrapping_sub(1)) {
            ensure!(
                entry.after.overrides == journal.current.overrides,
                "history cursor differs from current config"
            );
        } else if let Some(entry) = journal.entries.first() {
            ensure!(
                entry.before.overrides == journal.current.overrides,
                "history origin differs from current config"
            );
        }
        let mut store = Self {
            file,
            _lock: lock,
            journal,
        };
        if let Some(pending) = &store.journal.pending {
            ensure!(
                pending.change.before == store.journal.current,
                "invalid pending base"
            );
            if pending.change.after == current {
                store.finish(current)?;
            } else {
                ensure!(
                    pending.change.before == current,
                    "unrelated config during recovery"
                );
                let mut next = store.journal.clone();
                next.pending = None;
                store.persist(next)?;
            }
        } else {
            ensure!(
                store.journal.current == current,
                "config changed outside history; refusing stale undo"
            );
            if !existed {
                store.persist(store.journal.clone())?;
            }
        }
        Ok(store)
    }
    // Publish in-memory state only after the rename and directory durability receipt.
    fn persist(&mut self, next: Journal) -> Result<()> {
        let temp = self.file.with_extension("undo.pending");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(&serde_json::to_vec_pretty(&next)?)?;
        file.sync_all()?;
        fs::rename(temp, &self.file)?;
        File::open(parent_dir(&self.file))?.sync_all()?;
        self.journal = next;
        Ok(())
    }
    fn prepare(&mut self, kind: Kind, before: Snapshot, after: Snapshot) -> Result<()> {
        ensure!(
            self.journal.pending.is_none() && self.journal.current == before,
            "history busy or stale base"
        );
        ensure!(
            after.revision
                == before
                    .revision
                    .checked_add(1)
                    .context("revision exhausted")?
                && !after.last_commit_id.is_empty()
                && after.last_commit_id != before.last_commit_id,
            "invalid operation identity"
        );
        let mut next = self.journal.clone();
        next.pending = Some(Pending {
            kind,
            change: Change { before, after },
        });
        self.persist(next)
    }
    fn finish(&mut self, current: Snapshot) -> Result<()> {
        let mut next = self.journal.clone();
        let Some(pending) = next.pending.take() else {
            ensure!(current == next.current, "mismatched duplicate receipt");
            return self.persist(next);
        };
        ensure!(current == pending.change.after, "mismatched save receipt");
        apply_change_to_journal(&mut next, pending.kind, pending.change, current)?;
        self.persist(next)
    }
    fn step(&mut self, command: u64) -> Result<()> {
        let j = &self.journal;
        ensure!(j.pending.is_none(), "history busy");
        let (kind, after) = resolve_step_snapshot(j, command)?;
        self.prepare(kind, j.current.clone(), after)
    }
    fn reply(&self) -> Reply {
        let j = &self.journal;
        Reply {
            current: j.current.clone(),
            prepared: j.pending.as_ref().map(|p| p.change.after.clone()),
            can_undo: j.pending.is_none() && j.cursor > 0,
            can_redo: j.pending.is_none() && j.cursor < j.entries.len(),
        }
    }
}
fn push_bounded_entry(next: &mut Journal, change: Change) {
    next.entries.truncate(next.cursor);
    next.entries.push(change);
    next.cursor += 1;
    if next.entries.len() > MAX_JOURNAL_ENTRIES {
        let excess = next.entries.len() - MAX_JOURNAL_ENTRIES;
        next.entries.drain(..excess);
        next.cursor = next.cursor.saturating_sub(excess);
    }
}
fn apply_change_to_journal(
    next: &mut Journal,
    kind: Kind,
    change: Change,
    current: Snapshot,
) -> Result<()> {
    match kind {
        Kind::Save => {
            if change.before.overrides != current.overrides {
                push_bounded_entry(next, change);
            }
        }
        Kind::Undo => {
            next.cursor = next.cursor.checked_sub(1).context("invalid undo cursor")?;
        }
        Kind::Redo => {
            ensure!(next.cursor < next.entries.len(), "invalid redo cursor");
            next.cursor += 1;
        }
    }
    next.current = current;
    Ok(())
}
fn resolve_step_snapshot(j: &Journal, command: u64) -> Result<(Kind, Snapshot)> {
    let (kind, target) = match command {
        UNDO if j.cursor > 0 => (Kind::Undo, j.entries[j.cursor - 1].before.overrides.clone()),
        REDO if j.cursor < j.entries.len() => {
            (Kind::Redo, j.entries[j.cursor].after.overrides.clone())
        }
        _ => bail!("no available undo/redo"),
    };
    let before = &j.current;
    let after = Snapshot {
        revision: before
            .revision
            .checked_add(1)
            .context("revision exhausted")?,
        last_commit_id: format!(
            "undo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ),
        overrides: target,
    };
    Ok((kind, after))
}
pub(crate) fn build_toolbar_actions(reply: Option<&Reply>, enabled: bool) -> Vec<ToolbarAction> {
    [
        (
            UNDO,
            "tekes-undo",
            "Undo",
            reply.is_some_and(|r| r.can_undo),
        ),
        (
            REDO,
            "tekes-redo",
            "Redo",
            reply.is_some_and(|r| r.can_redo),
        ),
    ]
    .into_iter()
    .map(|(id, icon, name, action_enabled)| ToolbarAction {
        command: tekes_ui_app_protocol::ToolbarItem {
            id,
            title: String::new(),
            enabled: action_enabled && enabled,
            geometry_dependent: false,
        },
        icon: Some(icon.into()),
        accessible_name: Some(name.into()),
        tooltip: Some(name.into()),
    })
    .collect()
}
#[cfg(target_os = "macos")]
pub(crate) struct InProcessUndo {
    store: Option<Store>,
    session: Session,
    window: u64,
    registration: u64,
}
#[cfg(target_os = "macos")]
impl InProcessUndo {
    pub fn new(session: Session, window: u64) -> Self {
        Self {
            store: None,
            session,
            window,
            registration: 0,
        }
    }
    pub fn open_or_reconcile(
        &mut self,
        file: PathBuf,
        namespace: String,
        current: Snapshot,
    ) -> Result<Reply> {
        if let Some(store) = &self.store
            && store.file == file
            && store.journal.namespace == namespace
            && store.journal.pending.is_none()
            && store.journal.current == current
        {
            if self.registration == 0 {
                self.registration = 1;
            }
            return Ok(store.reply());
        }
        self.store = None;
        let store = Store::open(file, namespace, current)?;
        let reply = store.reply();
        self.store = Some(store);
        self.registration = self.registration.max(1);
        Ok(reply)
    }
    pub fn invalidate(&mut self) {
        self.store = None;
    }
    pub fn current(&self) -> Result<Reply> {
        Ok(self.store.as_ref().context("history not opened")?.reply())
    }
    pub fn prepare_step(
        &self,
        source: &crate::toolbar_source::ComponentSource,
        command: u64,
        current: &Snapshot,
    ) -> Result<(Kind, Snapshot)> {
        ensure!(
            self.window != 0 && self.registration != 0,
            "history toolbar unavailable"
        );
        source.validate(&self.session, self.registration, self.window)?;
        let store = self.store.as_ref().context("history not opened")?;
        ensure!(
            store.journal.pending.is_none() && store.journal.current == *current,
            "stale undo baseline"
        );
        resolve_step_snapshot(&store.journal, command)
    }
    pub fn commit_atomic(
        &mut self,
        kind: Kind,
        before: Snapshot,
        confirmed: Snapshot,
    ) -> Result<Reply> {
        let store = self.store.as_mut().context("history not opened")?;
        ensure!(
            store.journal.pending.is_none() && store.journal.current == before,
            "history busy or stale base"
        );
        let mut next = store.journal.clone();
        apply_change_to_journal(
            &mut next,
            kind,
            Change {
                before,
                after: confirmed.clone(),
            },
            confirmed,
        )?;
        store.persist(next)?;
        self.registration += 1;
        Ok(store.reply())
    }
    pub fn toolbar_state(
        &self,
        enabled: bool,
    ) -> (
        Vec<ToolbarAction>,
        Option<crate::toolbar_source::ComponentSource>,
    ) {
        let reply = self.store.as_ref().map(Store::reply);
        let actions = build_toolbar_actions(reply.as_ref(), enabled && self.window != 0);
        let source =
            (self.store.is_some() && self.window != 0 && self.registration != 0).then(|| {
                crate::toolbar_source::ComponentSource {
                    session: self.session.clone(),
                    registration: self.registration,
                    window: self.window,
                }
            });
        (actions, source)
    }
}
#[derive(Default)]
pub struct UndoModule {
    store: Option<Store>,
    revision: u64,
    failed: bool,
}
impl ProcessModule for UndoModule {
    fn handle(&mut self, request: RequestBody, e: &ComponentEmitter) -> Result<serde_json::Value> {
        let result = (|| -> Result<Reply> {
            match request {
                RequestBody::Message { topic, payload } if topic == MODULE_ID => {
                    match serde_json::from_value(payload)? {
                        Command::Open {
                            file,
                            namespace,
                            current,
                        } => {
                            self.store = None;
                            self.failed = true;
                            self.store = Some(Store::open(file, namespace, current)?);
                            self.failed = false;
                        }
                        command => {
                            ensure!(!self.failed, "reopen history after failed transaction");
                            let s = self.store.as_mut().context("history not opened")?;
                            match command {
                                Command::Prepare { before, after } => {
                                    s.prepare(Kind::Save, before, after)?
                                }
                                Command::Confirm { current } => s.finish(current)?,
                                Command::Inspect => {}
                                Command::Open { .. } => unreachable!(),
                            }
                        }
                    }
                }
                RequestBody::Invoke { command, .. } => {
                    ensure!(!self.failed, "history unavailable");
                    self.store
                        .as_mut()
                        .context("history not opened")?
                        .step(command)?;
                }
                _ => bail!("unsupported history request"),
            }
            Ok(self.store.as_ref().context("history not opened")?.reply())
        })();
        if result.is_err() {
            self.failed = true;
        }
        let reply = self.store.as_ref().map(Store::reply);
        self.revision += 1;
        let items = build_toolbar_actions(reply.as_ref(), !self.failed);
        e.emit(ComponentEvent::Toolbar {
            revision: self.revision,
            scope: ToolbarScope::Window {
                window: e.window_identity().context("window binding required")?,
            },
            items,
        })?;
        Ok(serde_json::to_value(result?)?)
    }
}
pub fn definition() -> ComponentDefinition {
    ComponentDefinition {
        id: MODULE_ID,
        capabilities: Capabilities::HEADLESS,
        create: || Box::<UndoModule>::default(),
    }
}
#[cfg(target_os = "macos")]
#[path = "settings_undo_client.rs"]
pub(crate) mod client;
#[cfg(target_os = "macos")]
pub use client::verify;

#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "tekes-undo-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("undo.json")
    }
    fn next(base: &Snapshot, id: &str, value: i64) -> Snapshot {
        let mut s = base.clone();
        s.revision += 1;
        s.last_commit_id = id.into();
        s.overrides.insert("value".into(), value.into());
        s
    }
    #[test]
    fn saved_history_recovery_and_branching() {
        let path = path();
        let base = Snapshot::default();
        let a = next(&base, "a", 1);
        let b = next(&a, "b", 2);
        let mut s = Store::open(path.clone(), "test".into(), base.clone()).unwrap();
        s.prepare(Kind::Save, base.clone(), a.clone()).unwrap();
        assert_eq!(s.journal.cursor, 0);
        assert!(!s.reply().can_undo);
        // Config write did not happen: restart cancels the prepared history.
        drop(s);
        let mut s = Store::open(path.clone(), "test".into(), base.clone()).unwrap();
        assert!(s.journal.pending.is_none());
        assert_eq!(s.journal.cursor, 0);
        s.prepare(Kind::Save, base, a.clone()).unwrap();
        // Config durable but confirmation lost: restart finishes exactly once.
        drop(s);
        let mut s = Store::open(path.clone(), "test".into(), a.clone()).unwrap();
        s.finish(a.clone()).unwrap();
        assert_eq!(s.journal.cursor, 1);
        s.prepare(Kind::Save, a.clone(), b.clone()).unwrap();
        s.finish(b).unwrap();
        s.step(UNDO).unwrap();
        let undo = s.reply().prepared.unwrap();
        assert_eq!(s.journal.cursor, 2);
        assert_eq!(undo.overrides, a.overrides);
        s.finish(undo.clone()).unwrap();
        assert!(s.reply().can_redo);
        let c = next(&undo, "c", 3);
        s.prepare(Kind::Save, undo, c.clone()).unwrap();
        s.finish(c.clone()).unwrap();
        assert!(!s.reply().can_redo);
        drop(s);
        let s = Store::open(path.clone(), "test".into(), c).unwrap();
        assert_eq!(s.journal.cursor, 2);
        drop(s);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn unrelated_config_and_mismatched_receipts_fail_closed() {
        let path = path();
        let base = Snapshot::default();
        let a = next(&base, "a", 1);
        let mut s = Store::open(path.clone(), "test".into(), base.clone()).unwrap();
        s.prepare(Kind::Save, base.clone(), a.clone()).unwrap();
        assert!(s.finish(next(&base, "other", 2)).is_err());
        assert_eq!(s.journal.cursor, 0);
        drop(s);
        assert!(Store::open(path.clone(), "test".into(), next(&base, "external", 9)).is_err());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[cfg(test)]
mod lossless_snapshot_tests {
    use super::*;
    #[test]
    fn unknown_toml_datetime_and_nested_values_survive_history_transport() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../../examples/app-host-demo/settings.schema.toml"
        ))
        .unwrap();
        let file =
            std::env::temp_dir().join(format!("tekes-snapshot-wire-{}.toml", std::process::id()));
        std::fs::write(&file,format!("future = 1979-05-27T07:32:00Z\n[nested]\nvalues = [1, 2]\n[_tekes]\nnamespace = {:?}\nrevision = 1\nlast_commit_id = 'unknown-keys'\n",schema.namespace)).unwrap();
        let provider =
            tekes_ui_settings::FileProvider::open(file.clone(), schema.namespace).unwrap();
        let snapshot = provider.load().unwrap();
        let roundtrip: Snapshot =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(roundtrip, snapshot);
        drop(provider);
        std::fs::remove_file(&file).unwrap();
        let mut lock = file.as_os_str().to_os_string();
        lock.push(".lock");
        std::fs::remove_file(std::path::PathBuf::from(lock)).unwrap();
    }

    #[test]
    fn parent_dir_falls_back_to_current_dir_for_bare_relative_paths() {
        assert_eq!(parent_dir(Path::new("history.json")), Path::new("."));
        assert_eq!(
            parent_dir(Path::new("nested/history.json")),
            Path::new("nested")
        );
    }

    #[test]
    fn journal_caps_entries_at_capacity_preserving_cursor_continuity() {
        let path = std::env::temp_dir()
            .join(format!(
                "tekes-undo-cap-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("undo.json");
        let base = Snapshot::default();
        let mut s = Store::open(path.clone(), "cap".into(), base.clone()).unwrap();
        let mut cur = base;
        let total = MAX_JOURNAL_ENTRIES + 8;
        for i in 1..=total {
            let mut next = cur.clone();
            next.revision += 1;
            next.last_commit_id = format!("c-{i}");
            next.overrides.insert("value".into(), (i as i64).into());
            s.prepare(Kind::Save, cur, next.clone()).unwrap();
            s.finish(next.clone()).unwrap();
            cur = next;
        }
        assert_eq!(s.journal.entries.len(), MAX_JOURNAL_ENTRIES);
        assert_eq!(s.journal.cursor, MAX_JOURNAL_ENTRIES);
        assert_eq!(
            s.journal.entries[0]
                .after
                .overrides
                .get("value")
                .and_then(|v| v.as_integer()),
            Some(9)
        );
        drop(s);
        let reopened = Store::open(path.clone(), "cap".into(), cur).unwrap();
        assert_eq!(reopened.journal.entries.len(), MAX_JOURNAL_ENTRIES);
        drop(reopened);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
