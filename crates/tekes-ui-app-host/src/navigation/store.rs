//! Single-writer navigation storage, including replacement/durability uncertainty.
use super::model::*;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fault {
    #[default]
    None,
    BeforeRename,
    AfterRename,
    BeforeCleanup,
}

pub(super) struct Store {
    pub root: PathBuf,
    _lock: File,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        fs::create_dir_all(root)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("nav.lock"))?;
        lock.try_lock()
            .context("history namespace already has a writer")?;
        Ok(Self {
            root: root.to_owned(),
            _lock: lock,
        })
    }
    fn sync_dir(&self) -> Result<()> {
        File::open(&self.root)?
            .sync_all()
            .context("directory durability unknown")
    }
    fn replace(&self, name: &str, data: &impl Serialize) -> Result<()> {
        let tmp = self.root.join(format!("{name}.tmp"));
        let mut file = File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut file, data)?;
        file.sync_all()?;
        fs::rename(tmp, self.root.join(name))?;
        self.sync_dir()
    }
    pub fn history(&self, id: &str) -> Result<History> {
        let path = self.root.join("nav.json");
        let h = if path.exists() {
            serde_json::from_reader(File::open(path)?)?
        } else {
            History {
                version: 1,
                history_id: id.into(),
                revision: 0,
                last_commit_id: None,
                cursor: None,
                entries: vec![],
            }
        };
        h.validate(id)?;
        Ok(h)
    }
    pub fn pending(&self) -> Result<Option<Pending>> {
        let path = self.root.join("nav.pending.json");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_reader(File::open(path)?)?))
    }
    pub fn save_pending(&self, p: &Pending) -> Result<()> {
        self.replace("nav.pending.json", p)
    }
    pub fn clear_pending(&self) -> Result<()> {
        match fs::remove_file(self.root.join("nav.pending.json")) {
            Ok(()) => self.sync_dir(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.sync_dir(),
            Err(e) => Err(e.into()),
        }
    }
    pub fn commit(&self, p: &Pending, fault: Fault) -> Result<()> {
        let applied = p.applied.as_ref().context("execution receipt required")?;
        ensure!(
            applied.transaction == p.transaction
                && applied.target == p.transaction.target
                && !applied.context.is_empty(),
            "invalid execution receipt"
        );
        let h = &p.candidate;
        let tmp = self.root.join("nav.json.tmp");
        let mut f = File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut f, h)?;
        f.sync_all()?;
        ensure!(
            fault != Fault::BeforeRename,
            "injected pre-replacement failure"
        );
        fs::rename(tmp, self.root.join("nav.json"))?;
        ensure!(
            fault != Fault::AfterRename,
            "replacement-durability-unknown (injected directory sync failure)"
        );
        self.sync_dir()
    }
    pub fn confirm_visible(&self) -> Result<()> {
        File::open(self.root.join("nav.json"))?.sync_all()?;
        self.sync_dir()
    }
}
