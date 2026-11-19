//! Single-writer local provider. Run load/commit/query on a worker, never the UI thread.
use crate::{
    Result, Schema, SettingsError, Snapshot, Submission,
    model::{from_toml_table, to_toml_table},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
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
impl From<CommitFailure> for SettingsError {
    fn from(failure: CommitFailure) -> Self {
        Self::RevisionConflict(failure.message)
    }
}
fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut os = OsString::from(path.as_os_str());
    os.push(suffix);
    PathBuf::from(os)
}

impl FileProvider {
    pub fn open(path: PathBuf, namespace: String) -> Result<Self> {
        let parent = parent_dir(&path);
        if parent != Path::new(".") {
            fs::create_dir_all(parent)?;
        }
        let writer = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(sidecar_path(&path, ".lock"))?;
        writer.try_lock().map_err(|_| {
            SettingsError::ProviderLocked("Settings provider already has a writer".into())
        })?;
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
    fn load_unlocked(&self) -> Result<Snapshot> {
        let source = match fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Snapshot::default()),
            Err(e) => return Err(e.into()),
        };
        let mut table: toml::Table = toml::from_str(&source).map_err(|err| {
            SettingsError::Format(format!("invalid Settings file; original preserved: {err}"))
        })?;
        let metadata: Metadata = table
            .remove("_tekes")
            .ok_or_else(|| SettingsError::Format("missing Settings metadata".into()))?
            .try_into()?;
        if metadata.namespace != self.namespace {
            return Err(SettingsError::InvalidSchema(
                "Settings namespace mismatch".into(),
            ));
        }
        Ok(Snapshot {
            revision: metadata.revision,
            last_commit_id: metadata.last_commit_id,
            overrides: from_toml_table(table)?,
        })
    }
    pub fn load(&self) -> Result<Snapshot> {
        let _gate = self
            .gate
            .lock()
            .map_err(|_| SettingsError::ProviderLocked("provider poisoned".into()))?;
        self.load_unlocked()
    }
    pub fn load_validated(&self, schema: &Schema) -> Result<Snapshot> {
        let snapshot = self.load()?;
        schema.validate_snapshot(&snapshot)?;
        Ok(snapshot)
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
            if submission.commit_id.trim().is_empty() {
                return Err(SettingsError::DraftState(
                    "commit ID must not be empty".into(),
                ));
            }
            let current = self.load_unlocked()?;
            // Identify duplicates before comparing expected revision.
            if current.last_commit_id == submission.commit_id {
                if current.overrides != submission.overrides {
                    return Err(SettingsError::RevisionConflict(
                        "commit ID reused with different input".into(),
                    ));
                }
                if current.revision != submission.expected_revision + 1 {
                    return Err(SettingsError::RevisionConflict(
                        "commit ID reused with different revision".into(),
                    ));
                }
                replaced = true;
                File::open(parent_dir(&self.path))?.sync_all()?;
                return Ok(current);
            }
            if current.revision != submission.expected_revision {
                return Err(SettingsError::RevisionConflict(
                    "Settings revision conflict; reopen after resolving edits".into(),
                ));
            }
            let next = Snapshot {
                revision: current
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| SettingsError::RevisionConflict("revision exhausted".into()))?,
                last_commit_id: submission.commit_id.clone(),
                overrides: submission.overrides.clone(),
            };
            let mut table = to_toml_table(&next.overrides)?;
            let metadata = Metadata {
                namespace: self.namespace.clone(),
                revision: next.revision,
                last_commit_id: next.last_commit_id.clone(),
            };
            table.insert("_tekes".into(), toml::Value::try_from(metadata)?);
            let source = toml::to_string_pretty(&table)?;
            let temp = sidecar_path(&self.path, ".pending");
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
            File::open(parent_dir(&self.path))?
                .sync_all()
                .map_err(|err| {
                    SettingsError::Storage(std::io::Error::new(
                        err.kind(),
                        format!("replacement durability unknown; query original commit: {err}"),
                    ))
                })?;
            Ok(next)
        })();
        result.map_err(|e| CommitFailure {
            message: e.to_string(),
            unknown: replaced,
        })
    }
    /// Last unretired commit is sufficient for this serial, local-only prototype.
    /// This is not the full Host-crash journal for remote Provider/action recovery.
    pub fn query(&self, submission: &Submission) -> Result<Option<Snapshot>> {
        if submission.commit_id.trim().is_empty() {
            return Err(SettingsError::DraftState(
                "commit ID must not be empty".into(),
            ));
        }
        let _gate = self
            .gate
            .lock()
            .map_err(|_| SettingsError::ProviderLocked("provider poisoned".into()))?;
        let current = self.load_unlocked()?;
        if current.last_commit_id == submission.commit_id
            && current.overrides == submission.overrides
        {
            File::open(parent_dir(&self.path))?.sync_all()?;
            Ok(Some(current))
        } else {
            Ok(None)
        }
    }
}
