use crate::toolbar_source::ComponentSource;
use super::*;
use std::time::{Duration, Instant};
use tekes_ui_module_runtime::{ProcessComponent, ProcessComponentOptions, ProcessStatus};
use tekes_ui_settings::{FileProvider, Submission};

struct Worker {
    process: ProcessComponent,
    actions: Vec<ToolbarAction>,
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
