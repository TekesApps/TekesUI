//! Value-owned draft/acknowledgement protocol. Persistence and transport belong
//! to the caller; controls never open files, discover user data or send requests.
use std::io;

/// Caller-owned scope identity plus a monotonic, value-owned editor revision.
/// Payload encoding is opaque to the library (plain text, JSON, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftRevision {
    pub identity: String,
    pub version: u64,
    pub payload: String,
}
/// Immutable submission value returned only after the caller saved the journal.
/// Retries preserve snapshot, request identity and revision-bound idempotency key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftSubmission {
    pub draft: DraftRevision,
    pub request_id: String,
    pub idempotency_key: String,
    /// Zero is permitted for restored queued deliveries not handed to transport yet.
    pub attempts: u64,
}
/// Single-writer, serial-delivery draft state. Full-journal save callbacks must
/// be durable/atomic according to the host's storage contract. An Err leaves
/// the in-memory journal unchanged; the actual editor can retain its unsaved text.
///
/// Transport starts after begin_submission/retry_pending succeeds. Confirmations
/// match scope, version and request. Acknowledgement never clears editor payload;
/// hosts may clear a visible editor only after checking its still-current revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftJournal {
    current: DraftRevision,
    pending: Vec<DraftSubmission>,
    acknowledged: Option<u64>,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn key(draft: &DraftRevision) -> String {
    // Length-prefix prevents delimiter collisions across arbitrary caller scopes.
    format!(
        "tekesui-draft/1:{}:{}:{}",
        draft.identity.len(),
        draft.identity,
        draft.version
    )
}
impl DraftJournal {
    pub fn new(identity: impl Into<String>, payload: impl Into<String>) -> io::Result<Self> {
        let current = DraftRevision {
            identity: identity.into(),
            version: 1,
            payload: payload.into(),
        };
        let expected = current.identity.clone();
        Self::restore(&expected, current, vec![], None)
    }
    /// Restore only after the host selected the intended identity and decoded its
    /// own storage format. Invalid/cross-scope/future/duplicate state is rejected.
    /// The input is not rewritten or repaired silently.
    pub fn restore(
        expected_identity: &str,
        current: DraftRevision,
        pending: Vec<DraftSubmission>,
        acknowledged: Option<u64>,
    ) -> io::Result<Self> {
        if current.identity.is_empty()
            || current.identity != expected_identity
            || current.version == 0
            || acknowledged.is_some_and(|v| v == 0 || v > current.version)
        {
            return Err(invalid("invalid draft identity/version/acknowledgement"));
        }
        let mut previous = acknowledged.unwrap_or(0);
        let mut requests = std::collections::BTreeSet::new();
        for submission in &pending {
            if submission.draft.identity != current.identity
                || submission.draft.version <= previous
                || submission.draft.version > current.version
                || submission.request_id.is_empty()
                || !requests.insert(&submission.request_id)
                || submission.idempotency_key != key(&submission.draft)
                || (submission.draft.version == current.version && submission.draft != current)
            {
                return Err(invalid("invalid immutable draft submission"));
            }
            previous = submission.draft.version;
        }
        Ok(Self {
            current,
            pending,
            acknowledged,
        })
    }
    pub fn current(&self) -> &DraftRevision {
        &self.current
    }
    pub fn pending(&self) -> &[DraftSubmission] {
        &self.pending
    }
    pub fn acknowledged(&self) -> Option<u64> {
        self.acknowledged
    }
    pub fn is_acknowledged(&self) -> bool {
        self.acknowledged == Some(self.current.version) && self.pending.is_empty()
    }
    /// Generate the stable key for an explicitly scoped revision. Storage
    /// adapters use the same definition when importing a legacy local snapshot.
    pub fn idempotency_key(draft: &DraftRevision) -> String {
        key(draft)
    }
    pub fn edit(
        &mut self,
        payload: impl Into<String>,
        save: impl FnOnce(&Self) -> io::Result<()>,
    ) -> io::Result<bool> {
        let payload = payload.into();
        if payload == self.current.payload {
            return Ok(false);
        }
        let mut next = self.clone();
        next.current.version = next
            .current
            .version
            .checked_add(1)
            .ok_or_else(|| invalid("draft version exhausted"))?;
        next.current.payload = payload;
        save(&next)?;
        *self = next;
        Ok(true)
    }
    /// Freeze the current payload and save before handing it to transport.
    /// A serial pending delivery must be retried/acknowledged first; newer editor
    /// revisions remain editable but are never substituted into that request.
    pub fn begin_submission(
        &mut self,
        request_id: impl Into<String>,
        save: impl FnOnce(&Self) -> io::Result<()>,
    ) -> io::Result<DraftSubmission> {
        let request_id = request_id.into();
        if request_id.is_empty() || self.is_acknowledged() {
            return Err(invalid(
                "missing request identity or already acknowledged revision",
            ));
        }
        if !self.pending.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "retry or acknowledge pending submission first",
            ));
        }
        let submission = DraftSubmission {
            draft: self.current.clone(),
            request_id,
            idempotency_key: key(&self.current),
            attempts: 1,
        };
        let mut next = self.clone();
        next.pending.push(submission.clone());
        save(&next)?;
        *self = next;
        Ok(submission)
    }
    pub fn retry_pending(
        &mut self,
        save: impl FnOnce(&Self) -> io::Result<()>,
    ) -> io::Result<DraftSubmission> {
        let mut next = self.clone();
        let first = next
            .pending
            .first_mut()
            .ok_or_else(|| invalid("no pending submission"))?;
        first.attempts = first
            .attempts
            .checked_add(1)
            .ok_or_else(|| invalid("submission attempts exhausted"))?;
        let result = first.clone();
        save(&next)?;
        *self = next;
        Ok(result)
    }
    /// Delayed, duplicate, cross-scope and future confirmations cannot clear a
    /// newer editor or acknowledge another pending request. Payload stays intact.
    pub fn acknowledge(
        &mut self,
        identity: &str,
        version: u64,
        request_id: &str,
        save: impl FnOnce(&Self) -> io::Result<()>,
    ) -> io::Result<bool> {
        if identity != self.current.identity
            || self.pending.first().is_none_or(|p| {
                p.draft.version != version || p.request_id != request_id || p.attempts == 0
            })
        {
            return Ok(false);
        }
        let mut next = self.clone();
        next.pending.remove(0);
        next.acknowledged = Some(version);
        save(&next)?;
        *self = next;
        Ok(true)
    }
}
