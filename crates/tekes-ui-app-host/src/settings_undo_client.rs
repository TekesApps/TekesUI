use crate::toolbar_source::ComponentSource;
// Serial background coordinator. All pipe waits and disk IO stay off the UI thread.
use super::*;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
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
    tx: Sender<Job>,
    pub rx: Receiver<Completion>,
}
struct Worker {
    process: ProcessComponent,
    actions: Vec<ToolbarAction>,
    current: Option<Reply>,
}
impl Worker {
    fn spawn(window: u64) -> Result<Self> {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64;
        let process = ProcessComponent::spawn_in_window(
            ProcessComponentOptions::new(MODULE_ID, epoch),
            window,
            || {},
        )?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match process.status() {
                ProcessStatus::Ready { pid, .. } => {
                    eprintln!("SETTINGS_UNDO_READY pid={pid}");
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
impl Client {
    pub fn start(provider: FileProvider, namespace: String, window: u64, wake: WindowWake) -> Self {
        let (tx, jobs) = mpsc::channel();
        let (results, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut worker: Option<Worker> = None;
            let run = |job: Job, worker: &mut Option<Worker>| -> Completion {
                let restore = matches!(job, Job::Step { .. });
                let mut submission = match &job {
                    Job::Save { submission, .. } => Some(submission.clone()),
                    _ => None,
                };
                let result = (|| -> std::result::Result<Snapshot, CommitFailure> {
                    let failure = |e: anyhow::Error| CommitFailure {
                        message: format!("{e:#}"),
                        unknown: false,
                    };
                    if worker
                        .as_ref()
                        .is_some_and(|w| !matches!(w.process.status(), ProcessStatus::Ready { .. }))
                    {
                        *worker = None;
                    }
                    if restore && worker.is_none() {
                        return Err(failure(anyhow::anyhow!(
                            "Original history worker is unavailable; recover before retry"
                        )));
                    }
                    if worker.is_none() {
                        *worker = Some(Worker::spawn(window).map_err(failure)?);
                    }
                    let w = worker.as_mut().unwrap();
                    let state = if restore {
                        let state = w.current.clone().ok_or_else(|| {
                            failure(anyhow::anyhow!("No acknowledged history state"))
                        })?;
                        ensure_result(
                            provider.load().map_err(failure)? == state.current,
                            "configuration changed since toolbar registration",
                        )?;
                        state
                    } else {
                        w.open(&provider, &namespace).map_err(failure)?
                    };
                    match job {
                        Job::Recover => return Ok(state.current),
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
                            // Open reconciles journal against the visible Provider snapshot first.
                            ensure_result(
                                state.current == saved,
                                "history/config reconciliation mismatch",
                            )?;
                            return Ok(saved);
                        }
                        Job::Save {
                            submission: ref task,
                            query: false,
                        } => {
                            ensure_result(
                                state.current.revision == task.expected_revision,
                                "stale save base",
                            )?;
                            let after = Snapshot {
                                revision: task.expected_revision + 1,
                                last_commit_id: task.commit_id.clone(),
                                overrides: task.overrides.clone(),
                            };
                            w.rpc(Command::Prepare {
                                before: state.current,
                                after,
                            })
                            .map_err(failure)?;
                        }
                        Job::Step {
                            command,
                            ref source,
                        } => {
                            let id = source.invoke(&w.process, command).map_err(failure)?;
                            let prepared =
                                w.wait(id).map_err(failure)?.prepared.ok_or_else(|| {
                                    failure(anyhow::anyhow!("history produced no operation"))
                                })?;
                            submission = Some(
                                Submission::replacement(&state.current, &prepared)
                                    .map_err(failure)?,
                            );
                        }
                    }
                    let task = submission.as_ref().unwrap();
                    let snapshot = match provider.commit_checked(task) {
                        Ok(s) => s,
                        Err(error) => {
                            // Reopen resolves an abandoned prepare when the Provider write certainly failed.
                            if !error.unknown {
                                let _ = w.open(&provider, &namespace);
                            }
                            return Err(error);
                        }
                    };
                    w.rpc(Command::Confirm {
                        current: snapshot.clone(),
                    })
                    .map_err(|e| CommitFailure {
                        message: format!("Saved; history confirmation unresolved: {e:#}"),
                        unknown: true,
                    })?;
                    Ok(snapshot)
                })();
                let actions = worker
                    .as_ref()
                    .map(|w| w.actions.clone())
                    .unwrap_or_default();
                Completion {
                    submission,
                    restore,
                    result,
                    actions,
                    source: worker
                        .as_ref()
                        .and_then(|w| ComponentSource::current(&w.process)),
                }
            };
            let initial = run(Job::Recover, &mut worker);
            if results.send(initial).is_err() {
                return;
            }
            wake.wake();
            while let Ok(job) = jobs.recv() {
                let completion = run(job, &mut worker);
                if results.send(completion).is_err() {
                    break;
                }
                wake.wake();
            }
            if let Some(w) = worker {
                w.process.close();
            }
        });
        Self { tx, rx }
    }
    pub fn send(&self, job: Job) -> Result<()> {
        self.tx.send(job).context("history coordinator stopped")
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

/// Executed by the consuming test app, so the actual component executable/factory is tested.
pub fn verify() -> Result<()> {
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
