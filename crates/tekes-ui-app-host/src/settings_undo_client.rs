//! Serial in-process undo coordinator. All file locks and disk IO stay off the UI thread.
use super::*;
use crate::toolbar_source::ComponentSource;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tekes_ui_module_runtime::{ProcessComponent, ProcessComponentOptions, ProcessStatus};
use tekes_ui_platform_macos::WindowWake;
use tekes_ui_settings::{CommitFailure, FileProvider, Submission};

pub enum Job {
    Save {
        submission: Submission,
        query: bool,
    },
    Step {
        command: u64,
        source: ComponentSource,
    },
    Recover,
}
pub struct Completion {
    pub submission: Option<Submission>,
    pub restore: bool,
    pub result: std::result::Result<Snapshot, CommitFailure>,
    pub actions: Vec<ToolbarAction>,
    pub source: Option<ComponentSource>,
}
pub struct Client {
    tx: Option<Sender<Job>>,
    pub rx: Receiver<Completion>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Client {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Client {
    pub fn start(provider: FileProvider, namespace: String, window: u64, wake: WindowWake) -> Self {
        Self::start_with_wake(provider, namespace, window, move || {
            wake.wake();
        })
    }
    pub(crate) fn start_with_wake(
        provider: FileProvider,
        namespace: String,
        window: u64,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let (tx, jobs) = mpsc::channel();
        let (results, rx) = mpsc::channel();
        let undo_file = provider.path().with_file_name("undo.json");
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        let session = Session {
            module: MODULE_ID.into(),
            instance: 1,
            host_epoch: epoch,
        };
        let worker = std::thread::Builder::new()
            .name("apphost-settings-undo".into())
            .spawn(move || {
                let mut undo = InProcessUndo::new(session, window);
                let run = |job: Job, undo: &mut InProcessUndo| -> Completion {
                    let restore = matches!(job, Job::Step { .. });
                    let mut submission = match &job {
                        Job::Save { submission, .. } => Some(submission.clone()),
                        _ => None,
                    };
                    let mut ready = true;
                    let result = (|| -> std::result::Result<Snapshot, CommitFailure> {
                        let failure = |e: anyhow::Error| CommitFailure {
                            message: format!("{e:#}"),
                            unknown: false,
                        };
                        let state = if restore {
                            let state = undo.current().map_err(failure)?;
                            ensure_result(
                                provider.load().map_err(failure)? == state.current,
                                "configuration changed since toolbar registration",
                            )?;
                            state
                        } else {
                            let current = provider.load().map_err(failure)?;
                            undo.open_or_reconcile(
                                undo_file.clone(),
                                namespace.clone(),
                                current,
                            )
                            .map_err(failure)?
                        };
                        match job {
                            Job::Recover => Ok(state.current),
                            Job::Save {
                                submission: ref task,
                                query: true,
                            } => {
                                let saved = provider
                                    .query(task)
                                    .map_err(|e| CommitFailure {
                                        message: format!("{e:#}"),
                                        unknown: true,
                                    })?
                                    .ok_or_else(|| CommitFailure {
                                        message:
                                            "Original save is not confirmed; configuration preserved"
                                                .into(),
                                        unknown: true,
                                    })?;
                                if state.current != saved {
                                    ensure_result(
                                        state.current.revision == task.expected_revision
                                            && saved.revision == task.expected_revision + 1
                                            && saved.last_commit_id == task.commit_id,
                                        "history/config reconciliation mismatch",
                                    )?;
                                    undo.commit_atomic(
                                        Kind::Save,
                                        state.current.clone(),
                                        saved.clone(),
                                    )
                                    .map_err(|e| CommitFailure {
                                        message: format!(
                                            "Saved; history confirmation unresolved: {e:#}"
                                        ),
                                        unknown: true,
                                    })?;
                                }
                                Ok(saved)
                            }
                            Job::Save {
                                submission: ref task,
                                query: false,
                            } => {
                                ensure_result(
                                    state.current.revision == task.expected_revision,
                                    "stale save base",
                                )?;
                                let snapshot = match provider.commit_checked(task) {
                                    Ok(s) => s,
                                    Err(error) => {
                                        if error.unknown {
                                            undo.invalidate();
                                        }
                                        return Err(error);
                                    }
                                };
                                undo.commit_atomic(Kind::Save, state.current, snapshot.clone())
                                    .map_err(|e| CommitFailure {
                                        message: format!(
                                            "Saved; history confirmation unresolved: {e:#}"
                                        ),
                                        unknown: true,
                                    })?;
                                Ok(snapshot)
                            }
                            Job::Step {
                                command,
                                ref source,
                            } => {
                                let (kind, prepared) = undo
                                    .prepare_step(source, command, &state.current)
                                    .map_err(failure)?;
                                let task = Submission::replacement(&state.current, &prepared)
                                    .map_err(failure)?;
                                submission = Some(task.clone());
                                let snapshot = match provider.commit_checked(&task) {
                                    Ok(s) => s,
                                    Err(error) => {
                                        if error.unknown {
                                            undo.invalidate();
                                        }
                                        return Err(error);
                                    }
                                };
                                undo.commit_atomic(kind, state.current, snapshot.clone())
                                    .map_err(|e| CommitFailure {
                                        message: format!(
                                            "Saved; history confirmation unresolved: {e:#}"
                                        ),
                                        unknown: true,
                                    })?;
                                Ok(snapshot)
                            }
                        }
                    })();
                    if let Err(err) = &result
                        && !err.unknown
                        && submission.is_none()
                    {
                        ready = false;
                    }
                    let (actions, source) = undo.toolbar_state(ready);
                    Completion {
                        submission,
                        restore,
                        result,
                        actions,
                        source,
                    }
                };
                let initial = run(Job::Recover, &mut undo);
                if results.send(initial).is_err() {
                    return;
                }
                wake();
                while let Ok(job) = jobs.recv() {
                    let completion = run(job, &mut undo);
                    if results.send(completion).is_err() {
                        break;
                    }
                    wake();
                }
            })
            .ok();
        Self {
            tx: Some(tx),
            rx,
            worker,
        }
    }
    pub fn send(&self, job: Job) -> Result<()> {
        self.tx
            .as_ref()
            .context("history coordinator stopped")?
            .send(job)
            .context("history coordinator stopped")
    }
}
fn ensure_result(ok: bool, message: &str) -> std::result::Result<(), CommitFailure> {
    if ok {
        Ok(())
    } else {
        Err(CommitFailure {
            message: message.into(),
            unknown: false,
        })
    }
}

struct Worker {
    process: ProcessComponent,
    actions: Vec<ToolbarAction>,
    current: Option<Reply>,
}
impl Worker {
    fn spawn(window: u64) -> Result<Self> {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        let process = ProcessComponent::spawn_in_window(
            ProcessComponentOptions::new(MODULE_ID, epoch),
            window,
            || {},
        )?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match process.status() {
                ProcessStatus::Ready { pid, .. } => {
                    host_diag!("SETTINGS_UNDO_READY pid={pid}");
                    break;
                }
                ProcessStatus::Starting if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                state => bail!("history worker startup: {state:?}"),
            }
        }
        Ok(Self {
            process,
            actions: vec![],
            current: None,
        })
    }
    fn rpc(&mut self, command: Command) -> Result<Reply> {
        let id = self.process.request(RequestBody::Message {
            topic: MODULE_ID.into(),
            payload: serde_json::to_value(command)?,
        })?;
        self.wait(id)
    }
    fn wait(&mut self, id: u64) -> Result<Reply> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            for packet in self.process.drain_events() {
                match packet.output {
                    Output::Event(ComponentEvent::Toolbar { items, .. }) => self.actions = items,
                    Output::Response { request, result } => {
                        ensure!(request == id, "unexpected history response");
                        let reply: Reply =
                            serde_json::from_value(result.map_err(anyhow::Error::msg)?)?;
                        self.current = Some(reply.clone());
                        return Ok(reply);
                    }
                    Output::Fatal { message } => bail!("{message}"),
                    _ => {}
                }
            }
            ensure!(Instant::now() < deadline, "history response timeout");
            ensure!(
                matches!(self.process.status(), ProcessStatus::Ready { .. }),
                "history worker disconnected"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn open(&mut self, provider: &FileProvider, namespace: &str) -> Result<Reply> {
        self.rpc(Command::Open {
            file: provider.path().with_file_name("undo.json"),
            namespace: namespace.into(),
            current: provider.load()?,
        })
    }
}

/// Executed by the consuming test app, so both in-process Client and component factory are tested.
pub fn verify() -> Result<()> {
    verify_in_process_client()?;
    let directory = std::env::temp_dir().join(format!(
        "tekes-settings-undo-ipc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let provider = FileProvider::open(directory.join("settings.toml"), "acceptance".into())?;
    let mut w = Worker::spawn(1)?;
    let state = w.open(&provider, "acceptance")?;
    ensure!(!state.can_undo, "unexpected history");
    let mut first = state.current.clone();
    first.revision += 1;
    first.last_commit_id = "save-1".into();
    first.overrides.insert("value".into(), 1.into());
    w.rpc(Command::Prepare {
        before: state.current.clone(),
        after: first.clone(),
    })?;
    // A known failed/no config write leaves no history after recovery.
    ensure!(
        !w.open(&provider, "acceptance")?.can_undo,
        "unwritten prepare advanced cursor"
    );
    w.rpc(Command::Prepare {
        before: state.current.clone(),
        after: first.clone(),
    })?;
    let task = Submission::replacement(&state.current, &first)?;
    provider.commit(&task)?;
    // Deliberately lose the confirmation and restart the actual child.
    w.process.close();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(w.process.status(), ProcessStatus::Closed) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    ensure!(
        matches!(w.process.status(), ProcessStatus::Closed),
        "worker did not stop"
    );
    let mut w = Worker::spawn(1)?;
    ensure!(
        w.open(&provider, "acceptance")?.can_undo,
        "saved prepare was not recovered"
    );
    ensure!(
        provider.query(&task)? == Some(first.clone()),
        "original save receipt unavailable"
    );
    let origin = ComponentSource::current(&w.process).context("missing undo source")?;
    let mut stale = origin.clone();
    stale.registration = stale.registration.wrapping_sub(1);
    ensure!(
        stale.invoke(&w.process, UNDO).is_err(),
        "stale registration accepted"
    );
    let mut wrong = origin.clone();
    wrong.window += 1;
    ensure!(
        wrong.invoke(&w.process, UNDO).is_err(),
        "wrong window accepted"
    );
    let mut dead = origin.clone();
    dead.session.host_epoch += 1;
    ensure!(
        dead.invoke(&w.process, UNDO).is_err(),
        "wrong session accepted"
    );
    let id = origin.invoke(&w.process, UNDO)?;
    let prepared = w.wait(id)?;
    ensure!(!prepared.can_redo, "cursor advanced before undo saved");
    let undo = prepared.prepared.context("missing undo target")?;
    ensure!(
        undo.overrides.is_empty(),
        "undo target differs from baseline"
    );
    let saved = provider.commit(&Submission::replacement(&first, &undo)?)?;
    let confirmed = w.rpc(Command::Confirm {
        current: saved.clone(),
    })?;
    ensure!(
        confirmed.can_redo && !confirmed.can_undo,
        "undo state incorrect"
    );
    ensure!(
        origin.invoke(&w.process, REDO).is_err(),
        "old registration survived confirmation"
    );
    let id = ComponentSource::current(&w.process)
        .context("missing redo source")?
        .invoke(&w.process, REDO)?;
    let redo = w.wait(id)?.prepared.context("missing redo target")?;
    let saved = provider.commit(&Submission::replacement(&saved, &redo)?)?;
    let confirmed = w.rpc(Command::Confirm { current: saved })?;
    ensure!(
        confirmed.can_undo && !confirmed.can_redo,
        "redo state incorrect"
    );
    let status = w.process.status();
    ensure!(
        w.actions.len() == 2 && w.actions.iter().all(|a| a.tooltip.is_some()),
        "toolbar registration missing"
    );
    w.process.close();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(w.process.status(), ProcessStatus::Closed) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(w);
    drop(provider);
    fs::remove_dir_all(directory)?;
    println!(
        "SETTINGS_UNDO_IPC_PASS {status:?}: prepare abort, lost confirmation restart, saved undo/redo, toolbar registration"
    );
    Ok(())
}

fn verify_in_process_client() -> Result<()> {
    let directory = std::env::temp_dir().join(format!(
        "tekes-settings-undo-inproc-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let provider = FileProvider::open(directory.join("settings.toml"), "inproc".into())?;
    let client = Client::start_with_wake(provider.clone(), "inproc".into(), 1, || {});
    let init = client
        .rx
        .recv_timeout(Duration::from_secs(5))
        .context("in-process initial recover timeout")?;
    let base = init.result.map_err(|e| anyhow::anyhow!(e.message))?;
    ensure!(!directory.join("undo.pending").exists());
    let mut after = base.clone();
    after.revision += 1;
    after.last_commit_id = "inproc-save-1".into();
    after.overrides.insert("value".into(), 42.into());
    client.send(Job::Save {
        submission: Submission::replacement(&base, &after)?,
        query: false,
    })?;
    let saved_completion = client
        .rx
        .recv_timeout(Duration::from_secs(5))
        .context("in-process save timeout")?;
    let stale_source = init.source.clone().context("missing initial source")?;
    let current_source = saved_completion
        .source
        .clone()
        .context("missing save source")?;
    ensure!(saved_completion.result.is_ok());
    ensure!(!directory.join("undo.pending").exists());
    // Stale source registration from before the save must be rejected.
    client.send(Job::Step {
        command: UNDO,
        source: stale_source,
    })?;
    let stale_undo = client
        .rx
        .recv_timeout(Duration::from_secs(5))
        .context("stale undo response timeout")?;
    ensure!(
        stale_undo.result.is_err(),
        "stale in-process source accepted"
    );
    // Valid undo -> redo roundtrip.
    client.send(Job::Step {
        command: UNDO,
        source: current_source,
    })?;
    let undone = client
        .rx
        .recv_timeout(Duration::from_secs(5))
        .context("undo timeout")?;
    ensure!(undone.result.is_ok());
    let redo_source = undone.source.clone().context("missing redo source")?;
    client.send(Job::Step {
        command: REDO,
        source: redo_source,
    })?;
    let redone = client
        .rx
        .recv_timeout(Duration::from_secs(5))
        .context("redo timeout")?;
    ensure!(redone.result.is_ok());
    drop(client);
    drop(provider);
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_process_client_handles_save_undo_redo_and_rejects_stale_source() {
        verify_in_process_client().unwrap();
    }
}
