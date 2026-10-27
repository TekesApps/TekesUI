//! Single-writer local provider. Run load/commit/query on a worker, never the UI thread.
use crate::{Snapshot, Submission};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub struct FileProvider {
    path: PathBuf,
    namespace: String,
    _writer: Arc<File>,
    gate: Arc<Mutex<()>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    namespace: String,
    revision: u64,
    last_commit_id: String,
}
#[derive(Debug)]
pub struct CommitFailure {
    pub message: String,
    pub unknown: bool,
}
impl std::fmt::Display for CommitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CommitFailure {}
impl FileProvider {
    pub fn open(path: PathBuf, namespace: String) -> Result<Self> {
        let parent = path.parent().context("Settings file needs parent")?;
        fs::create_dir_all(parent)?;
        let writer = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))?;
        writer
            .try_lock()
            .context("Settings provider already has a writer")?;
        Ok(Self {
            path,
            namespace,
            _writer: Arc::new(writer),
            gate: Arc::new(Mutex::new(())),
        })
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    pub fn load(&self) -> Result<Snapshot> {
        let source = match fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Snapshot::default()),
            Err(e) => return Err(e.into()),
        };
        let mut table: toml::Table =
            toml::from_str(&source).context("invalid Settings file; original preserved")?;
        let metadata: Metadata = table
            .remove("_tekes")
            .context("missing Settings metadata")?
            .try_into()?;
        ensure!(
            metadata.namespace == self.namespace,
            "Settings namespace mismatch"
        );
        Ok(Snapshot {
            revision: metadata.revision,
            last_commit_id: metadata.last_commit_id,
            overrides: table,
        })
    }
    pub fn commit(&self, submission: &Submission) -> Result<Snapshot> {
        self.commit_checked(submission).map_err(Into::into)
    }
    pub fn commit_checked(
        &self,
        submission: &Submission,
    ) -> std::result::Result<Snapshot, CommitFailure> {
        let _gate = self.gate.lock().map_err(|_| CommitFailure {
            message: "provider poisoned".into(),
            unknown: true,
        })?;
        let mut replaced = false;
        let result = (|| -> Result<Snapshot> {
            let current = self.load()?;
            // Identify duplicates before comparing expected revision.
            if current.last_commit_id == submission.commit_id {
                ensure!(
                    current.overrides == submission.overrides,
                    "commit ID reused with different input"
                );
                ensure!(
                    current.revision == submission.expected_revision + 1,
                    "commit ID reused with different revision"
                );
                replaced = true;
                File::open(self.path.parent().unwrap())?.sync_all()?;
                return Ok(current);
            }
            ensure!(
                current.revision == submission.expected_revision,
                "Settings revision conflict; reopen after resolving edits"
            );
            let next = Snapshot {
                revision: current
                    .revision
                    .checked_add(1)
                    .context("revision exhausted")?,
                last_commit_id: submission.commit_id.clone(),
                overrides: submission.overrides.clone(),
            };
            let mut table = next.overrides.clone();
            let metadata = Metadata {
                namespace: self.namespace.clone(),
                revision: next.revision,
                last_commit_id: next.last_commit_id.clone(),
            };
            table.insert("_tekes".into(), toml::Value::try_from(metadata)?);
            let source = toml::to_string_pretty(&table)?;
            let temp = self.path.with_extension("pending");
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
            file.write_all(source.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            replaced = true;
            File::open(self.path.parent().unwrap())?
                .sync_all()
                .context("replacement durability unknown; query original commit")?;
            Ok(next)
        })();
        result.map_err(|e| CommitFailure {
            message: format!("{e:#}"),
            unknown: replaced,
        })
    }
    /// Last unretired commit is sufficient for this serial, local-only prototype.
    /// This is not the full Host-crash journal for remote Provider/action recovery.
    pub fn query(&self, submission: &Submission) -> Result<Option<Snapshot>> {
        let _gate = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("provider poisoned"))?;
        let current = self.load()?;
        if current.last_commit_id == submission.commit_id
            && current.overrides == submission.overrides
        {
            File::open(self.path.parent().unwrap())?.sync_all()?;
            Ok(Some(current))
        } else {
            Ok(None)
        }
    }
}
