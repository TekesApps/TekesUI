//! Navigation consumer of TekesUI ProcessComponent. No View or renderer is created.
mod host;
mod model;
pub use host::NavigationHost;
mod store;
use anyhow::{Context, Result, bail, ensure};
use model::Pending;
pub use model::*;
use serde_json::Value;
pub use store::Fault;
use store::Store;
use tekes_ui_module_runtime::{ComponentEmitter, ProcessModule, component::*};

pub const MODULE_ID: &str = "navigation";
pub const BACK: u64 = 0x4e01;
pub const FORWARD: u64 = 0x4e02;
pub const MAX_HISTORY_ENTRIES: usize = 256;
/// Faults are only enabled by the dedicated acceptance executable's factory.
#[derive(Default)]
pub struct NavigationModule {
    store: Option<Store>,
    history: Option<History>,
    pending: Option<Pending>,
    recovery: Option<Offer>,
    recovery_ready: Option<RecoveryApplied>,
    ready: bool,
    window: u64,
    registration: u64,
    toolbar_state: Option<(bool, bool)>,
    sequence: u64,
    fault: Fault,
    in_process: bool,
}
impl NavigationModule {
    pub fn new_in_process() -> Self {
        Self {
            in_process: true,
            ..Self::default()
        }
    }
    pub fn acceptance(fault: Fault) -> Self {
        Self {
            fault,
            ..Self::default()
        }
    }
    pub fn inject_acceptance_fault(&mut self, fault: Fault) {
        self.fault = fault;
    }
    fn unique(&mut self, prefix: &str, session: &Session) -> String {
        self.sequence += 1;
        format!(
            "{prefix}-{}-{}-{}-{}",
            session.host_epoch,
            session.instance,
            std::process::id(),
            self.sequence
        )
    }
    fn h(&self) -> Result<&History> {
        self.history.as_ref().context("navigation not opened")
    }
    fn s(&self) -> Result<&Store> {
        self.store.as_ref().context("navigation not opened")
    }
    fn next_toolbar(&mut self) -> Result<Option<(u64, u64, Vec<ToolbarAction>)>> {
        if self.window == 0 {
            return Ok(None);
        }
        let enabled = self.ready && self.pending.is_none();
        let h = self.h()?;
        let projection = (
            enabled && h.cursor.is_some_and(|i| i > 0),
            enabled && h.cursor.is_some_and(|i| i + 1 < h.entries.len()),
        );
        if self.toolbar_state == Some(projection) {
            return Ok(None);
        }
        self.toolbar_state = Some(projection);
        self.registration += 1;
        let items = [
            (BACK, "tekes-back", "Back", projection.0),
            (FORWARD, "tekes-forward", "Forward", projection.1),
        ]
        .into_iter()
        .map(|(id, icon, name, enabled)| ToolbarAction {
            command: tekes_ui_app_protocol::ToolbarItem {
                id,
                title: String::new(),
                enabled,
                geometry_dependent: false,
            },
            icon: Some(icon.into()),
            accessible_name: Some(name.into()),
            tooltip: Some(name.into()),
        })
        .collect();
        Ok(Some((self.registration, self.window, items)))
    }
    fn toolbar(&mut self, e: &ComponentEmitter) -> Result<()> {
        if let Some((revision, window, items)) = self.next_toolbar()? {
            e.emit(ComponentEvent::Toolbar {
                revision,
                scope: ToolbarScope::Window { window },
                items,
            })?;
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<Reply> {
        let p = self.pending.as_ref().context("no pending transaction")?;
        let candidate = p.candidate.clone();
        self.s()?.commit(p, self.fault)?;
        self.history = Some(candidate.clone());
        // Cleanup is after the commit point: a cleanup error cannot turn a
        // durable history commit into an execution/persistence failure.
        if self.in_process && self.fault == Fault::None {
            if !self.s()?.root.join("nav.pending.json").exists()
                || self.s()?.clear_pending().is_ok()
            {
                self.pending = None;
            }
        } else if self.fault != Fault::BeforeCleanup && self.s()?.clear_pending().is_ok() {
            self.pending = None;
        }
        Ok(Reply::HistoryCommitted(candidate))
    }
    fn recover(&mut self, session: &Session) -> Result<Reply> {
        self.ready = false;
        self.recovery_ready = None;
        self.recovery = None;
        // Synchronous worker coordinator: a prior disk write/callback cannot run concurrently.
        let disk = self.s()?.history(&self.h()?.history_id)?;
        if let Some(p) = self.pending.clone() {
            let tx = &p.transaction;
            p.candidate.validate(&disk.history_id)?;
            ensure!(
                tx.history_id == disk.history_id
                    && p.candidate.current() == tx.target.as_ref()
                    && p.candidate.revision
                        == tx
                            .base_revision
                            .checked_add(1)
                            .context("revision exhausted")?
                    && p.candidate.last_commit_id.as_ref() == Some(&tx.navigation_id),
                "invalid pending identity"
            );
            if disk == p.candidate {
                self.s()?.confirm_visible()?;
                self.history = Some(disk);
                self.s()?.clear_pending()?;
                self.pending = None;
            } else {
                ensure!(disk.revision == tx.base_revision, "unrelated disk revision");
                ensure!(
                    candidate(&disk, &tx.action, &tx.navigation_id)? == p.candidate,
                    "pending candidate differs from its action/base"
                );
                self.history = Some(disk);
                if p.applied.is_some() {
                    self.finish()?;
                } else {
                    return Ok(Reply::NeedReceipt(tx.clone()));
                }
            }
        } else {
            self.history = Some(disk);
        }
        let offer = Offer {
            recovery_id: self.unique("restore", session),
            session: session.clone(),
            history: self.h()?.clone(),
        };
        self.recovery = Some(offer.clone());
        Ok(Reply::RestoreOffer(offer))
    }
    fn apply(&mut self, applied: Applied) -> Result<Reply> {
        let expected = self.pending.as_ref().context("no pending transaction")?;
        ensure!(
            expected.transaction == applied.transaction
                && applied.target == applied.transaction.target
                && !applied.context.is_empty(),
            "invalid execution receipt"
        );
        let p = Pending {
            transaction: applied.transaction.clone(),
            applied: Some(applied),
            candidate: expected.candidate.clone(),
        };
        self.pending = Some(p.clone());
        let result = (|| {
            if !(self.in_process && self.fault == Fault::None) {
                self.s()?.save_pending(&p)?;
            }
            self.finish()
        })();
        if result.is_err() {
            self.ready = false;
        }
        self.recovery = None;
        self.recovery_ready = None;
        result
    }
    fn command(&mut self, c: Command, session: &Session) -> Result<Reply> {
        match c {
            Command::Open {
                directory,
                history_id,
                window,
            } => {
                ensure!(
                    self.store.is_none() && !history_id.is_empty() && window != 0,
                    "invalid/duplicate open"
                );
                let store = Store::open(&directory)?;
                let h = store.history(&history_id)?;
                let pending = store.pending()?;
                self.store = Some(store);
                self.history = Some(h);
                self.pending = pending;
                self.window = window;
                self.recover(session)
            }
            Command::Recover => self.recover(session),
            Command::RecoveryApplied(applied) => {
                ensure!(
                    self.recovery.as_ref() == Some(&applied.offer)
                        && applied.offer.session == *session
                        && applied.offer.history == *self.h()?
                        && self.pending.is_none(),
                    "stale recovery session or baseline"
                );
                ensure!(
                    applied.target.as_ref() == applied.offer.history.current()
                        && !applied.context.is_empty(),
                    "restore did not publish offered target"
                );
                if let Some(old) = &self.recovery_ready {
                    ensure!(old == &applied, "different recovery context");
                }
                self.recovery_ready = Some(applied.clone());
                self.ready = true;
                Ok(Reply::RecoveryReady(applied))
            }
            Command::Navigate(action) => {
                ensure!(
                    self.ready && self.pending.is_none(),
                    "recovering or transaction busy"
                );
                let id = self.unique("navigation", session);
                let h = candidate(self.h()?, &action, &id)?;
                let tx = Transaction {
                    navigation_id: id,
                    session: session.clone(),
                    base_revision: self.h()?.revision,
                    history_id: h.history_id.clone(),
                    action,
                    target: h.current().cloned(),
                };
                let pending = Pending {
                    transaction: tx.clone(),
                    applied: None,
                    candidate: h,
                };
                // If journal durability fails, conservatively freeze until explicit recovery.
                self.pending = Some(pending.clone());
                if !(self.in_process && self.fault == Fault::None)
                    && let Err(error) = self.s()?.save_pending(&pending)
                {
                    self.ready = false;
                    return Err(error);
                }
                Ok(Reply::Execute(tx))
            }
            Command::Applied(applied) => {
                ensure!(
                    applied.transaction.session == *session && self.ready,
                    "old instance or recovering; use receipt reconciliation"
                );
                if self.pending.is_none() {
                    self.command(Command::CommitStatus(applied.transaction), session)
                } else {
                    self.apply(applied)
                }
            }
            Command::Reconcile(applied) => {
                ensure!(
                    !self.ready && self.recovery.is_none(),
                    "receipt reconciliation requires recovering handshake"
                );
                self.apply(applied)
            }
            Command::ReconcileFailed(tx) => {
                ensure!(
                    !self.ready && self.recovery.is_none(),
                    "failure reconciliation requires recovery handshake"
                );
                ensure!(
                    self.pending
                        .as_ref()
                        .is_some_and(|p| p.transaction == tx && p.applied.is_none()),
                    "unknown/already applied transaction"
                );
                if !(self.in_process && self.fault == Fault::None)
                    || self.s()?.root.join("nav.pending.json").exists()
                {
                    self.s()?.clear_pending()?;
                }
                self.pending = None;
                Ok(Reply::Aborted)
            }
            Command::Failed(tx) => {
                ensure!(
                    tx.session == *session
                        && self.ready
                        && self
                            .pending
                            .as_ref()
                            .is_some_and(|p| p.transaction == tx && p.applied.is_none()),
                    "unknown/already applied transaction"
                );
                if !(self.in_process && self.fault == Fault::None)
                    || self.s()?.root.join("nav.pending.json").exists()
                {
                    self.s()?.clear_pending()?;
                }
                self.pending = None;
                Ok(Reply::Aborted)
            }
            Command::CommitStatus(tx) => {
                let h = self.h()?;
                ensure!(
                    h.history_id == tx.history_id
                        && h.revision
                            == tx
                                .base_revision
                                .checked_add(1)
                                .context("revision exhausted")?
                        && h.current() == tx.target.as_ref()
                        && h.last_commit_id.as_ref() == Some(&tx.navigation_id),
                    "commit not confirmed at current baseline"
                );
                ensure!(
                    self.pending.is_none(),
                    "persistence/reconciliation still pending"
                );
                self.s()?.confirm_visible()?;
                Ok(Reply::HistoryCommitted(h.clone()))
            }
            Command::Inspect => Ok(Reply::State {
                history: self.h()?.clone(),
                pending: self.pending.as_ref().map(|p| p.transaction.clone()),
                ready: self.ready,
            }),
        }
    }
}
impl NavigationModule {
    fn parse_request(&self, request: RequestBody, in_process: bool) -> Result<Command> {
        match request {
            RequestBody::Message { topic, payload } if topic == "navigation" => {
                Ok(serde_json::from_value(payload)?)
            }
            RequestBody::Invoke {
                command,
                registration,
                window,
            } => {
                if in_process {
                    ensure!(
                        window != 0 && window == self.window && registration == self.registration,
                        "stale navigation toolbar registration"
                    );
                }
                match command {
                    BACK => Ok(Command::Navigate(Action::Back)),
                    FORWARD => Ok(Command::Navigate(Action::Forward)),
                    _ => bail!("unknown navigation action"),
                }
            }
            _ => bail!("unsupported navigation request"),
        }
    }
    pub(crate) fn handle_in_process(
        &mut self,
        request: RequestBody,
        session: &Session,
    ) -> InProcessOutcome {
        let result = (|| -> Result<Reply> {
            let cmd = self.parse_request(request, true)?;
            self.command(cmd, session)
        })();
        let toolbar = self.next_toolbar().ok().flatten();
        (result, toolbar)
    }
}

pub(crate) type InProcessToolbarUpdate = (u64, u64, Vec<ToolbarAction>);
pub(crate) type InProcessOutcome = (Result<Reply>, Option<InProcessToolbarUpdate>);
impl ProcessModule for NavigationModule {
    fn handle(&mut self, request: RequestBody, e: &ComponentEmitter) -> Result<Value> {
        let command = self.parse_request(request, false)?;
        let result = self.command(command, e.session());
        self.toolbar(e)?;
        result.and_then(|r| Ok(serde_json::to_value(r)?))
    }
}
fn candidate(old: &History, action: &Action, id: &str) -> Result<History> {
    let mut h = old.clone();
    match action {
        Action::Push(entry) => {
            h.entries.truncate(h.cursor.map_or(0, |i| i + 1));
            h.entries.push(entry.clone());
            if h.entries.len() > MAX_HISTORY_ENTRIES {
                let excess = h.entries.len() - MAX_HISTORY_ENTRIES;
                h.entries.drain(..excess);
            }
            h.cursor = Some(h.entries.len() - 1);
        }
        Action::Back => {
            h.cursor = Some(h.cursor.filter(|i| *i > 0).context("at beginning")? - 1);
        }
        Action::Forward => {
            let i = h.cursor.context("empty history")?;
            ensure!(i + 1 < h.entries.len(), "at end");
            h.cursor = Some(i + 1);
        }
    }
    h.revision = h.revision.checked_add(1).context("revision exhausted")?;
    h.last_commit_id = Some(id.into());
    h.validate(&h.history_id)?;
    Ok(h)
}
/// Fixed-catalogue factory: only child worker entry constructs navigation state.
pub fn create() -> Box<dyn ProcessModule> {
    Box::<NavigationModule>::default()
}
pub fn definition() -> tekes_ui_module_runtime::ComponentDefinition {
    tekes_ui_module_runtime::ComponentDefinition {
        id: MODULE_ID,
        capabilities: Capabilities::HEADLESS,
        create,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn in_process_navigation_caps_history_and_commits_single_write() {
        let dir = std::env::temp_dir().join(format!(
            "tekes-nav-inproc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let session = Session {
            module: MODULE_ID.into(),
            instance: 1,
            host_epoch: 7,
        };
        let mut module = NavigationModule::new_in_process();
        let (reply, _) = module.handle_in_process(
            RequestBody::Message {
                topic: "navigation".into(),
                payload: serde_json::to_value(Command::Open {
                    directory: dir.clone(),
                    history_id: "test-nav".into(),
                    window: 1,
                })
                .unwrap(),
            },
            &session,
        );
        let offer = match reply.unwrap() {
            Reply::RestoreOffer(offer) => offer,
            other => panic!("unexpected reply: {other:?}"),
        };
        let mut host = NavigationHost::new(session.clone());
        let applied = host
            .restore(offer, |_| Ok("window:7:empty".into()))
            .unwrap();
        let (reply, _) = module.handle_in_process(
            RequestBody::Message {
                topic: "navigation".into(),
                payload: serde_json::to_value(Command::RecoveryApplied(applied)).unwrap(),
            },
            &session,
        );
        let ready = match reply.unwrap() {
            Reply::RecoveryReady(r) => r,
            other => panic!("unexpected reply: {other:?}"),
        };
        host.ready(&ready).unwrap();

        let total = MAX_HISTORY_ENTRIES + 10;
        let mut last_history = None;
        for i in 0..total {
            let entry = Entry {
                id: format!("entry-{i}"),
                route: "doc".into(),
                params: json!({ "index": i }),
            };
            let (reply, _) = module.handle_in_process(
                RequestBody::Message {
                    topic: "navigation".into(),
                    payload: serde_json::to_value(Command::Navigate(Action::Push(entry))).unwrap(),
                },
                &session,
            );
            let tx = match reply.unwrap() {
                Reply::Execute(tx) => tx,
                other => panic!("unexpected reply: {other:?}"),
            };
            assert!(
                !dir.join("nav.pending.json").exists(),
                "in-process Navigate must not write nav.pending.json"
            );
            let receipt = host
                .execute(tx.clone(), |_| Ok("window:7:doc".into()))
                .unwrap();
            let (reply, _) = module.handle_in_process(
                RequestBody::Message {
                    topic: "navigation".into(),
                    payload: serde_json::to_value(Command::Applied(receipt)).unwrap(),
                },
                &session,
            );
            let history = match reply.unwrap() {
                Reply::HistoryCommitted(h) => h,
                other => panic!("unexpected reply: {other:?}"),
            };
            assert!(
                !dir.join("nav.pending.json").exists(),
                "in-process commit must not create nav.pending.json"
            );
            host.committed(&tx, &history).unwrap();
            last_history = Some(history);
        }
        let history = last_history.unwrap();
        assert_eq!(history.entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(history.cursor, Some(MAX_HISTORY_ENTRIES - 1));
        assert_eq!(history.entries[0].id, "entry-10");
        assert_eq!(
            history.entries[MAX_HISTORY_ENTRIES - 1].id,
            format!("entry-{}", total - 1)
        );
        drop(module);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
