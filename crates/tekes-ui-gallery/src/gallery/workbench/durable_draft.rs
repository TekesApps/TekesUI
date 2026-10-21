//! Consumer-owned, single-writer fixture storage. One atomic snapshot contains
//! the current form draft, immutable delivery revisions, and acknowledgements.
//! No control owns persistence and no network or user directory is discovered.
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use tekes_ui_controls::{DraftJournal, DraftRevision, DraftSubmission};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Draft {
    pub revision: u64,
    pub name: String,
    pub notifications: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    pub(super) struct TemporaryDirectory(pub PathBuf);
    impl TemporaryDirectory {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "tekesui-draft-fixture-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn durable_retry_and_revision_specific_ack_survive_reopen() {
        let directory = TemporaryDirectory::new();
        let path = directory.0.join("form.json");
        let mut store = DurableDraft::open(path.clone()).unwrap();
        store.update("项目 Alpha", true).unwrap();
        let (first, attempt) = store.begin_attempt().unwrap();
        assert_eq!(attempt, 1);
        drop(store);
        let mut store = DurableDraft::open(path.clone()).unwrap();
        assert_eq!(store.draft().name, "项目 Alpha");
        assert_eq!(store.begin_attempt().unwrap(), (first, 2));
        store.update("Newer edit", false).unwrap();
        assert!(store.acknowledge(first).unwrap());
        assert!(!store.is_acknowledged());
        assert_eq!(store.draft().name, "Newer edit");
        let (newer, _) = store.begin_attempt().unwrap();
        assert!(!store.acknowledge(first).unwrap());
        assert!(store.has_pending());
        drop(store);
        let mut store = DurableDraft::open(path).unwrap();
        assert_eq!(store.begin_attempt().unwrap(), (newer, 2));
        assert!(store.acknowledge(newer).unwrap());
        assert!(store.reopen().unwrap().is_acknowledged());
    }

    #[test]
    fn interrupted_pre_rename_write_and_storage_failure_keep_last_committed_state() {
        let directory = TemporaryDirectory::new();
        let path = directory.0.join("form.json");
        let mut store = DurableDraft::open(path.clone()).unwrap();
        store.update("Committed", true).unwrap();
        let pending = directory.0.join("form.json.pending-interrupted");
        fs::write(&pending, b"partial interrupted write").unwrap();
        assert_eq!(store.reopen().unwrap().draft().name, "Committed");
        let original_path = store.path.clone();
        store.path = Some(directory.0.join("missing-parent/form.json"));
        assert!(store.update("Uncommitted", false).is_err());
        assert_eq!(store.draft().name, "Committed");
        store.path = original_path;
        assert_eq!(store.reopen().unwrap().draft().name, "Committed");
        assert_eq!(fs::read(pending).unwrap(), b"partial interrupted write");
    }

    #[test]
    fn legacy_metadata_free_outbox_restores_without_writing_and_preserves_unsent_revision() {
        let directory = TemporaryDirectory::new();
        let path = directory.0.join("legacy.json");
        let bytes=serde_json::to_vec(&json!({"format":"tekesui-gallery-form-draft/1",
            "draft":{"revision":3,"name":"latest","notifications":true},
            "outbox":[{"draft":{"revision":2,"name":"first","notifications":false},"attempts":1},
                {"draft":{"revision":3,"name":"latest","notifications":true},"attempts":0}],"acknowledged":null})).unwrap();
        fs::write(&path, &bytes).unwrap();
        let mut store = DurableDraft::open(path.clone()).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "opening never rewrites legacy data"
        );
        assert!(store.acknowledge(2).unwrap());
        assert!(!store.acknowledge(3).unwrap());
        assert_eq!(store.begin_attempt().unwrap(), (3, 1));
        assert!(store.acknowledge(3).unwrap());
        assert!(store.reopen().unwrap().is_acknowledged());
    }

    #[test]
    fn corrupt_or_unrelated_files_are_rejected_without_overwriting() {
        let directory = TemporaryDirectory::new();
        let path = directory.0.join("unrelated.json");
        fs::write(&path, b"{\"name\":\"unrelated\"}").unwrap();
        assert!(DurableDraft::open(path.clone()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{\"name\":\"unrelated\"}");
        assert!(DurableDraft::open("relative.json".into()).is_err());
    }

    #[test]
    fn inconsistent_revisions_cannot_restore_as_acknowledged_or_discard_a_draft() {
        let directory = TemporaryDirectory::new();
        let path = directory.0.join("form.json");
        let mut store = DurableDraft::open(path.clone()).unwrap();
        store.update("Latest draft", true).unwrap();
        store.begin_attempt().unwrap();
        let valid: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut corruptions = Vec::new();
        let mut future_delivery = valid.clone();
        future_delivery["outbox"][0]["draft"]["revision"] = json!(99);
        corruptions.push(future_delivery);
        let mut duplicate_delivery = valid.clone();
        duplicate_delivery["outbox"]
            .as_array_mut()
            .unwrap()
            .push(valid["outbox"][0].clone());
        corruptions.push(duplicate_delivery);
        let mut mismatched_payload = valid.clone();
        mismatched_payload["outbox"][0]["draft"]["name"] =
            json!("Different payload with same revision");
        corruptions.push(mismatched_payload);
        let mut false_acknowledgement = valid.clone();
        false_acknowledgement["acknowledged"] = json!(2);
        corruptions.push(false_acknowledgement);
        let mut malformed_revision = valid;
        malformed_revision["draft"]["revision"] = json!("2");
        corruptions.push(malformed_revision);
        for corrupted in corruptions {
            let bytes = serde_json::to_vec(&corrupted).unwrap();
            fs::write(&path, &bytes).unwrap();
            assert_eq!(
                DurableDraft::open(path.clone()).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
            assert_eq!(
                fs::read(&path).unwrap(),
                bytes,
                "loading must preserve rejected evidence"
            );
        }
    }
}
// Storage is a caller adapter. All revision/retry/ack policy lives in the
// same public journal used by independent consumers.
const IDENTITY: &str = "tekesui.gallery.form";
#[derive(Debug)]
pub(super) struct DurableDraft {
    path: Option<PathBuf>,
    state: DraftJournal,
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid gallery draft fixture")
}
fn decode_draft(v: &Value) -> io::Result<Draft> {
    Ok(Draft {
        revision: v["revision"]
            .as_u64()
            .filter(|r| *r > 0)
            .ok_or_else(invalid)?,
        name: v["name"].as_str().ok_or_else(invalid)?.into(),
        notifications: v["notifications"].as_bool().ok_or_else(invalid)?,
    })
}
fn payload(d: &Draft) -> String {
    json!({"name":d.name,"notifications":d.notifications}).to_string()
}
fn revision(d: &Draft) -> DraftRevision {
    DraftRevision {
        identity: IDENTITY.into(),
        version: d.revision,
        payload: payload(d),
    }
}
fn from_revision(d: &DraftRevision) -> io::Result<Draft> {
    let v: Value = serde_json::from_str(&d.payload).map_err(|_| invalid())?;
    Ok(Draft {
        revision: d.version,
        name: v["name"].as_str().ok_or_else(invalid)?.into(),
        notifications: v["notifications"].as_bool().ok_or_else(invalid)?,
    })
}
fn encode(d: &DraftRevision) -> io::Result<Value> {
    let d = from_revision(d)?;
    Ok(json!({"revision":d.revision,"name":d.name,"notifications":d.notifications}))
}
impl DurableDraft {
    /// Deterministic in-memory consumer adapter; never claims disk durability.
    pub fn memory() -> Self {
        Self {
            path: None,
            state: DraftJournal::new(
                IDENTITY,
                payload(&Draft {
                    revision: 1,
                    name: "Studio".into(),
                    notifications: false,
                }),
            )
            .unwrap(),
        }
    }
    pub fn is_durable(&self) -> bool {
        self.path.is_some()
    }
    /// Explicit single-writer absolute file, with legacy format-1 reads retained.
    /// Missing legacy metadata is imported in memory, never rewritten on open.
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if !path.is_absolute() || !path.parent().is_some_and(Path::is_dir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "draft fixture needs an absolute file in an existing directory",
            ));
        }
        let state = match fs::read(&path) {
            Ok(bytes) => {
                let v: Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
                if v["format"] != "tekesui-gallery-form-draft/1"
                    || v.get("identity")
                        .is_some_and(|identity| identity.as_str() != Some(IDENTITY))
                {
                    return Err(invalid());
                }
                let current = revision(&decode_draft(&v["draft"])?);
                let pending = v["outbox"]
                    .as_array()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(|item| {
                        let draft = revision(&decode_draft(&item["draft"])?);
                        let request_id = match item.get("request_id") {
                            Some(v) => v.as_str().ok_or_else(invalid)?.to_owned(),
                            None => format!("{IDENTITY}:revision-{}", draft.version),
                        };
                        let idempotency_key = match item.get("idempotency_key") {
                            Some(v) => v.as_str().ok_or_else(invalid)?.to_owned(),
                            None => DraftJournal::idempotency_key(&draft),
                        };
                        Ok(DraftSubmission {
                            draft,
                            request_id,
                            idempotency_key,
                            attempts: item["attempts"].as_u64().ok_or_else(invalid)?,
                        })
                    })
                    .collect::<io::Result<Vec<_>>>()?;
                let acknowledged = if v["acknowledged"].is_null() {
                    None
                } else {
                    Some(v["acknowledged"].as_u64().ok_or_else(invalid)?)
                };
                DraftJournal::restore(IDENTITY, current, pending, acknowledged)?
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Self::memory().state,
            Err(e) => return Err(e),
        };
        if !path.exists() {
            persist(Some(&path), &state)?;
        }
        Ok(Self {
            path: Some(path),
            state,
        })
    }
    pub fn draft(&self) -> Draft {
        from_revision(self.state.current()).expect("validated fixture payload")
    }
    pub fn has_pending(&self) -> bool {
        !self.state.pending().is_empty()
    }
    pub fn is_acknowledged(&self) -> bool {
        self.state.is_acknowledged()
    }
    pub fn update(&mut self, name: &str, notifications: bool) -> io::Result<bool> {
        let next = payload(&Draft {
            revision: self.state.current().version,
            name: name.into(),
            notifications,
        });
        let path = self.path.as_deref();
        self.state.edit(next, |j| persist(path, j))
    }
    pub fn begin_attempt(&mut self) -> io::Result<(u64, u64)> {
        let path = self.path.as_deref();
        let submission = if self.has_pending() {
            self.state.retry_pending(|j| persist(path, j))?
        } else {
            let request = format!("{IDENTITY}:revision-{}", self.state.current().version);
            self.state.begin_submission(request, |j| persist(path, j))?
        };
        Ok((submission.draft.version, submission.attempts))
    }
    pub fn acknowledge(&mut self, version: u64) -> io::Result<bool> {
        let Some(first) = self.state.pending().first() else {
            return Ok(false);
        };
        let request = first.request_id.clone();
        let path = self.path.as_deref();
        self.state
            .acknowledge(IDENTITY, version, &request, |j| persist(path, j))
    }
    pub fn reopen(&self) -> io::Result<Self> {
        match &self.path {
            Some(path) => Self::open(path.clone()),
            None => Ok(Self {
                path: None,
                state: self.state.clone(),
            }),
        }
    }
}
fn persist(path: Option<&Path>, state: &DraftJournal) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let outbox = state
        .pending()
        .iter()
        .map(|s| {
            Ok(json!({"draft":encode(&s.draft)?,
        "attempts":s.attempts,"request_id":s.request_id,"idempotency_key":s.idempotency_key}))
        })
        .collect::<io::Result<Vec<Value>>>()?;
    let bytes = serde_json::to_vec(
        &json!({"format":"tekesui-gallery-form-draft/1","identity":state.current().identity,
        "draft":encode(state.current())?,"outbox":outbox,"acknowledged":state.acknowledged()}),
    )
    .map_err(io::Error::other)?;
    static NEXT_WRITE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(format!(
        ".pending-{}-{}",
        std::process::id(),
        NEXT_WRITE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let temporary = PathBuf::from(temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result: io::Result<()> = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        File::open(path.parent().expect("validated parent"))?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
