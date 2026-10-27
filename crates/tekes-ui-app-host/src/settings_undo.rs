//! Headless saved-configuration history. The Provider remains the sole config writer.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};
use tekes_ui_module_runtime::{ComponentDefinition, ComponentEmitter, ProcessModule, component::*};
use tekes_ui_settings::Snapshot;

pub const MODULE_ID: &str = "settings-undo";
pub const UNDO: u64 = 0x5501;
pub const REDO: u64 = 0x5502;
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
enum Kind {
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
        let parent = file.parent().context("history needs a directory")?;
        fs::create_dir_all(parent)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(file.with_extension("undo.lock"))?;
        lock.try_lock()
            .context("Settings history already has a writer")?;
        let journal = match fs::read(&file) {
            Ok(bytes) => {
                serde_json::from_slice::<Journal>(&bytes).context("invalid history; preserved")?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Journal {
                version: 1,
                namespace: namespace.clone(),
                current: current.clone(),
                entries: vec![],
                cursor: 0,
                pending: None,
            },
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
            store.persist(store.journal.clone())?;
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
        File::open(self.file.parent().unwrap())?.sync_all()?;
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
        match pending.kind {
            Kind::Save => {
                if pending.change.before.overrides != current.overrides {
                    next.entries.truncate(next.cursor);
                    next.entries.push(pending.change);
                    next.cursor += 1;
                }
            }
            Kind::Undo => {
                next.cursor = next.cursor.checked_sub(1).context("invalid undo cursor")?
            }
            Kind::Redo => {
                ensure!(next.cursor < next.entries.len(), "invalid redo cursor");
                next.cursor += 1;
            }
        }
        next.current = current;
        self.persist(next)
    }
    fn step(&mut self, command: u64) -> Result<()> {
        let j = &self.journal;
        ensure!(j.pending.is_none(), "history busy");
        let (kind, target) = match command {
            UNDO if j.cursor > 0 => (Kind::Undo, j.entries[j.cursor - 1].before.overrides.clone()),
            REDO if j.cursor < j.entries.len() => {
                (Kind::Redo, j.entries[j.cursor].after.overrides.clone())
            }
            _ => bail!("no available undo/redo"),
        };
        let before = j.current.clone();
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
        self.prepare(kind, before, after)
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
        let items = [
            (
                UNDO,
                "tekes-undo",
                "Undo",
                reply.as_ref().is_some_and(|r| r.can_undo),
            ),
            (
                REDO,
                "tekes-redo",
                "Redo",
                reply.as_ref().is_some_and(|r| r.can_redo),
            ),
        ]
        .into_iter()
        .map(|(id, icon, name, enabled)| ToolbarAction {
            command: tekes_ui_app_protocol::ToolbarItem {
                id,
                title: String::new(),
                enabled: enabled && !self.failed,
                geometry_dependent: false,
            },
            icon: Some(icon.into()),
            accessible_name: Some(name.into()),
            tooltip: Some(name.into()),
        })
        .collect();
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
        std::fs::remove_file(file.with_extension("lock")).unwrap();
    }
}
