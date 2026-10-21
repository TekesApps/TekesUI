use serde::{Deserialize, Serialize};
use serde_json::Value;
use tekes_ui_module_runtime::component::Session;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub route: String,
    pub params: Value,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub version: u32,
    pub history_id: String,
    pub revision: u64,
    pub last_commit_id: Option<String>,
    pub cursor: Option<usize>,
    pub entries: Vec<Entry>,
}
impl History {
    pub fn current(&self) -> Option<&Entry> {
        self.cursor.and_then(|i| self.entries.get(i))
    }
    pub(super) fn validate(&self, namespace: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.version == 1 && self.history_id == namespace,
            "unsupported history format/scope"
        );
        anyhow::ensure!(
            if self.entries.is_empty() {
                self.cursor.is_none()
            } else {
                self.cursor.is_some_and(|i| i < self.entries.len())
            },
            "invalid cursor"
        );
        let mut ids = std::collections::HashSet::new();
        for e in &self.entries {
            anyhow::ensure!(
                !e.id.is_empty()
                    && !e.route.is_empty()
                    && e.params.is_object()
                    && ids.insert(&e.id),
                "invalid/duplicate entry"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "entry")]
pub enum Action {
    Push(Entry),
    Back,
    Forward,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub navigation_id: String,
    pub session: Session,
    pub base_revision: u64,
    pub history_id: String,
    pub action: Action,
    pub target: Option<Entry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applied {
    pub transaction: Transaction,
    pub context: String,
    pub target: Option<Entry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub recovery_id: String,
    pub session: Session,
    pub history: History,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryApplied {
    pub offer: Offer,
    pub context: String,
    pub target: Option<Entry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", content = "data")]
pub enum Command {
    Open {
        directory: std::path::PathBuf,
        history_id: String,
        window: u64,
    },
    Recover,
    RecoveryApplied(RecoveryApplied),
    Navigate(Action),
    Applied(Applied),
    Failed(Transaction),
    Reconcile(Applied),
    ReconcileFailed(Transaction),
    CommitStatus(Transaction),
    Inspect,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum Reply {
    RestoreOffer(Offer),
    RecoveryReady(RecoveryApplied),
    Execute(Transaction),
    NeedReceipt(Transaction),
    HistoryCommitted(History),
    Aborted,
    State {
        history: History,
        pending: Option<Transaction>,
        ready: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pending {
    pub transaction: Transaction,
    pub applied: Option<Applied>,
    pub candidate: History,
}
