//! A caller-owned file adapter over the public journal. No Gallery imports.
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_controls::{DraftJournal, DraftRevision, DraftSubmission};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "tekesui-public-draft-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn bad() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid consumer draft")
}
fn encode(r: &DraftRevision) -> Value {
    json!({"identity":r.identity,"version":r.version,"payload":r.payload})
}
fn decode(v: &Value) -> io::Result<DraftRevision> {
    Ok(DraftRevision {
        identity: v["identity"].as_str().ok_or_else(bad)?.into(),
        version: v["version"].as_u64().ok_or_else(bad)?,
        payload: v["payload"].as_str().ok_or_else(bad)?.into(),
    })
}
fn save(path: &Path, j: &DraftJournal) -> io::Result<()> {
    let bytes=serde_json::to_vec(&json!({"format":"consumer-draft/1","current":encode(j.current()),
        "pending":j.pending().iter().map(|p|json!({"draft":encode(&p.draft),"request_id":p.request_id,"key":p.idempotency_key,"attempts":p.attempts})).collect::<Vec<_>>(),"ack":j.acknowledged()})).map_err(io::Error::other)?;
    let tmp = path.with_extension(format!("pending-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        File::open(path.parent().unwrap())?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
fn restore(path: &Path, scope: &str) -> io::Result<DraftJournal> {
    let v: Value = serde_json::from_slice(&fs::read(path)?).map_err(|_| bad())?;
    if v["format"] != "consumer-draft/1" {
        return Err(bad());
    }
    let current = decode(&v["current"])?;
    let pending = v["pending"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|p| {
            Ok(DraftSubmission {
                draft: decode(&p["draft"])?,
                request_id: p["request_id"].as_str().ok_or_else(bad)?.into(),
                idempotency_key: p["key"].as_str().ok_or_else(bad)?.into(),
                attempts: p["attempts"].as_u64().ok_or_else(bad)?,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let ack = if v["ack"].is_null() {
        None
    } else {
        Some(v["ack"].as_u64().ok_or_else(bad)?)
    };
    DraftJournal::restore(scope, current, pending, ack)
}
#[test]
fn consumer_file_recovers_frozen_request_and_rejects_other_scope_without_overwriting() {
    let dir = Directory::new();
    let path = dir.0.join("draft.json");
    let mut journal = DraftJournal::new("consumer-A/note-1", "committed first").unwrap();
    let first = journal
        .begin_submission("request-1", |j| save(&path, j))
        .unwrap();
    journal.edit("newer input", |j| save(&path, j)).unwrap();
    drop(journal);
    let before = fs::read(&path).unwrap();
    assert!(restore(&path, "consumer-B/note-1").is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut journal = restore(&path, "consumer-A/note-1").unwrap();
    let retry = journal.retry_pending(|j| save(&path, j)).unwrap();
    assert_eq!(retry.draft, first.draft);
    assert_eq!(retry.idempotency_key, first.idempotency_key);
    assert_eq!(retry.attempts, 2);
    assert!(
        journal
            .acknowledge(
                &first.draft.identity,
                first.draft.version,
                &first.request_id,
                |j| save(&path, j)
            )
            .unwrap()
    );
    let journal = restore(&path, "consumer-A/note-1").unwrap();
    assert_eq!(journal.current().payload, "newer input");
    assert!(!journal.is_acknowledged());
}
#[test]
fn consumer_storage_failure_does_not_publish_a_request_or_replace_committed_file() {
    let dir = Directory::new();
    let path = dir.0.join("draft.json");
    let missing = dir.0.join("missing/draft.json");
    let mut journal = DraftJournal::new("scope", "saved").unwrap();
    save(&path, &journal).unwrap();
    let bytes = fs::read(&path).unwrap();
    let prior = journal.clone();
    assert!(
        journal
            .edit("unsaved editor", |j| save(&missing, j))
            .is_err()
    );
    assert_eq!(journal, prior);
    assert!(
        journal
            .begin_submission("not-sent", |j| save(&missing, j))
            .is_err()
    );
    assert!(journal.pending().is_empty());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(restore(&path, "scope").unwrap(), prior);
}
