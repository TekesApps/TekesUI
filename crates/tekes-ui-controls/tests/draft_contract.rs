//! External consumer contract; persistence callback is explicit and can fail.
use std::io;
use tekes_ui_controls::{DraftJournal, DraftRevision};
fn saved(_: &DraftJournal) -> io::Result<()> {
    Ok(())
}
#[test]
fn immutable_retry_and_late_duplicate_cross_scope_ack_preserve_newer_editor() {
    let mut journal = DraftJournal::new("account-A/document-A", "first").unwrap();
    let first = journal.begin_submission("request-1", saved).unwrap();
    journal.edit("new typing", saved).unwrap();
    let retry = journal.retry_pending(saved).unwrap();
    assert_eq!(retry.draft, first.draft);
    assert_eq!(retry.request_id, first.request_id);
    assert_eq!(retry.idempotency_key, first.idempotency_key);
    assert_eq!(retry.attempts, 2);
    assert!(
        !journal
            .acknowledge(
                "account-B/document-A",
                first.draft.version,
                &first.request_id,
                saved
            )
            .unwrap()
    );
    assert!(
        !journal
            .acknowledge(&first.draft.identity, 999, &first.request_id, saved)
            .unwrap()
    );
    assert!(
        journal
            .acknowledge(
                &first.draft.identity,
                first.draft.version,
                &first.request_id,
                saved
            )
            .unwrap()
    );
    assert_eq!(journal.current().payload, "new typing");
    assert!(!journal.is_acknowledged());
    let second = journal.begin_submission("request-2", saved).unwrap();
    assert_ne!(first.idempotency_key, second.idempotency_key);
    assert!(
        !journal
            .acknowledge(
                &first.draft.identity,
                first.draft.version,
                &first.request_id,
                saved
            )
            .unwrap()
    );
    assert_eq!(journal.pending(), [second]);
}
#[test]
fn save_failure_leaves_journal_and_request_handoff_unchanged_then_restore_retries_exact_snapshot() {
    let mut journal = DraftJournal::new("scope", "draft").unwrap();
    let before = journal.clone();
    assert!(
        journal
            .edit("unsaved", |_| Err(io::Error::other("disk full")))
            .is_err()
    );
    assert_eq!(journal, before);
    assert!(
        journal
            .begin_submission("request", |_| Err(io::Error::other("save failed")))
            .is_err()
    );
    assert!(journal.pending().is_empty());
    let first = journal.begin_submission("request", saved).unwrap();
    let restored = DraftJournal::restore(
        "scope",
        journal.current().clone(),
        journal.pending().to_vec(),
        journal.acknowledged(),
    )
    .unwrap();
    let mut restored = restored;
    assert!(
        restored
            .acknowledge("scope", 1, "request", |_| Err(io::Error::other(
                "ack save failed"
            )))
            .is_err()
    );
    assert_eq!(restored.pending(), std::slice::from_ref(&first));
    let retried = restored.retry_pending(saved).unwrap();
    assert_eq!(retried.draft, first.draft);
    assert_eq!(retried.idempotency_key, first.idempotency_key);
}
#[test]
fn restoration_rejects_corrupt_or_cross_identity_values_without_repair() {
    let current = DraftRevision {
        identity: "scope".into(),
        version: 1,
        payload: "body".into(),
    };
    assert!(DraftJournal::restore("other expected scope", current.clone(), vec![], None).is_err());
    let mut journal = DraftJournal::restore("scope", current.clone(), vec![], None).unwrap();
    let submission = journal.begin_submission("request", saved).unwrap();
    let mut bad = submission.clone();
    bad.draft.identity = "other".into();
    assert!(DraftJournal::restore("scope", current.clone(), vec![bad], None).is_err());
    let mut bad = submission.clone();
    bad.draft.payload = "different same revision".into();
    assert!(DraftJournal::restore("scope", current.clone(), vec![bad], None).is_err());
    let mut bad = submission.clone();
    bad.idempotency_key = "forged".into();
    assert!(DraftJournal::restore("scope", current.clone(), vec![bad], None).is_err());
    assert!(
        DraftJournal::restore(
            "scope",
            current.clone(),
            vec![submission.clone(), submission],
            None
        )
        .is_err()
    );
    assert!(DraftJournal::restore("scope", current, vec![], Some(2)).is_err());
}
