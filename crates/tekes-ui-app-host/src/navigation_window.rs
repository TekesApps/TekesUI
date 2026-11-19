//! Nonblocking in-process window adapter. The bounded navigation state and store run on a dedicated coordinator thread without spawning a child OS process.
use crate::{navigation::*, toolbar_source::ComponentSource};
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tekes_ui_module_runtime::{ProcessStatus, component::*};
use tekes_ui_platform_macos::WindowWake;

const MAX_TRACE_BYTES: u64 = 1024 * 1024;

pub struct NavigationOptions {
    pub directory: std::path::PathBuf,
    pub history_id: String,
    /// Demo routes, navigated normally only when the history is empty.
    pub initial_entries: Vec<Entry>,
    /// Consumer route adapter for the center column's existing command channel.
    /// None keeps navigation independent of View data.
    pub view_command: Option<fn(&Entry) -> Result<u64>>,
}
impl std::fmt::Debug for NavigationOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationOptions")
            .field("directory", &self.directory)
            .field("history_id", &self.history_id)
            .field("initial_entries", &self.initial_entries)
            .finish_non_exhaustive()
    }
}
#[derive(Clone)]
pub enum Publication {
    Restore(Offer),
    Execute(Transaction),
}
impl Publication {
    pub(super) fn target(&self) -> Option<&Entry> {
        match self {
            Self::Restore(o) => o.history.current(),
            Self::Execute(t) => t.target.as_ref(),
        }
    }
}
struct SharedNavigationState {
    events: VecDeque<Output>,
    toolbar_source: Option<ComponentSource>,
    status: ProcessStatus,
}
pub struct WindowNavigation {
    session: Session,
    request_tx: Option<Sender<(u64, RequestBody)>>,
    worker: Option<std::thread::JoinHandle<()>>,
    shared: Arc<Mutex<SharedNavigationState>>,
    next_request: u64,
    pub host: NavigationHost,
    pub busy: bool,
    pub publication: Option<Publication>,
    options: NavigationOptions,
    opened: bool,
    pending: HashMap<u64, Command>,
    pub actions: Vec<ToolbarAction>,
    window: u64,
    seed: VecDeque<Entry>,
    transaction: Option<Transaction>,
    pub error: Option<String>,
    trace_tx: Option<std::sync::mpsc::SyncSender<String>>,
    trace_worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for WindowNavigation {
    fn drop(&mut self) {
        self.request_tx.take();
        self.trace_tx.take();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.trace_worker.take() {
            let _ = handle.join();
        }
    }
}
impl WindowNavigation {
    pub fn new(options: NavigationOptions, wake: WindowWake, window: u64) -> Result<Self> {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        let session = Session {
            module: MODULE_ID.into(),
            instance: 1,
            host_epoch: epoch,
        };
        let status = ProcessStatus::Ready {
            pid: std::process::id(),
            capabilities: Capabilities::HEADLESS,
        };
        let shared = Arc::new(Mutex::new(SharedNavigationState {
            events: VecDeque::new(),
            toolbar_source: None,
            status,
        }));
        let (request_tx, request_rx) = mpsc::channel::<(u64, RequestBody)>();
        let actor_shared = shared.clone();
        let actor_session = session.clone();
        let actor_wake = wake.clone();
        let worker = std::thread::Builder::new()
            .name("apphost-navigation".into())
            .spawn(move || {
                let mut module = NavigationModule::new_in_process();
                while let Ok((id, request)) = request_rx.recv() {
                    let (reply, toolbar) = module.handle_in_process(request, &actor_session);
                    let mut guard = actor_shared.lock().unwrap();
                    if let Some((revision, win, items)) = toolbar {
                        guard.toolbar_source = Some(ComponentSource {
                            session: actor_session.clone(),
                            registration: revision,
                            window: win,
                        });
                        guard
                            .events
                            .push_back(Output::Event(ComponentEvent::Toolbar {
                                revision,
                                scope: ToolbarScope::Window { window: win },
                                items,
                            }));
                    }
                    let result = reply
                        .and_then(|value| Ok(serde_json::to_value(value)?))
                        .map_err(|error| error.to_string());
                    guard.events.push_back(Output::Response {
                        request: id,
                        result,
                    });
                    drop(guard);
                    actor_wake.wake();
                }
                actor_shared.lock().unwrap().status = ProcessStatus::Closed;
            })?;
        wake.wake();
        let host = NavigationHost::new(session.clone());
        let (trace_tx, trace_rx) = std::sync::mpsc::sync_channel::<String>(256);
        let trace_path = options.directory.join("window-events.jsonl");
        let trace_worker = std::thread::Builder::new()
            .name("apphost-nav-trace".into())
            .spawn(move || {
                if let Some(parent) = trace_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&trace_path)
                    .ok();
                let mut bytes_written = file
                    .as_ref()
                    .and_then(|f| f.metadata().ok())
                    .map_or(0, |m| m.len());
                while let Ok(line) = trace_rx.recv() {
                    append_bounded_trace_line(
                        &trace_path,
                        &mut file,
                        &mut bytes_written,
                        &line,
                        MAX_TRACE_BYTES,
                    );
                }
            })?;
        Ok(Self {
            session,
            request_tx: Some(request_tx),
            worker: Some(worker),
            shared,
            next_request: 0,
            host,
            window,
            busy: true,
            publication: None,
            options,
            opened: false,
            pending: HashMap::new(),
            actions: vec![],
            seed: VecDeque::new(),
            transaction: None,
            error: None,
            trace_tx: Some(trace_tx),
            trace_worker: Some(trace_worker),
        })
    }
    pub fn status(&self) -> ProcessStatus {
        self.shared.lock().unwrap().status.clone()
    }
    pub(crate) fn toolbar_source(&self) -> Option<ComponentSource> {
        self.shared.lock().unwrap().toolbar_source.clone()
    }
    pub fn trace(&self, value: serde_json::Value) {
        if let Some(tx) = &self.trace_tx {
            let _ = tx.try_send(value.to_string());
        }
    }
    fn enqueue_request(&mut self, body: RequestBody) -> Result<u64> {
        self.next_request = self
            .next_request
            .checked_add(1)
            .context("navigation request ID exhausted")?;
        let id = self.next_request;
        self.request_tx
            .as_ref()
            .context("navigation coordinator stopped")?
            .send((id, body))
            .context("navigation coordinator stopped")?;
        Ok(id)
    }
    pub fn send(&mut self, command: Command) -> Result<()> {
        self.trace(json!({"event":"send","command":command}));
        let id = self.enqueue_request(RequestBody::Message {
            topic: "navigation".into(),
            payload: serde_json::to_value(&command)?,
        })?;
        self.pending.insert(id, command);
        self.busy = true;
        Ok(())
    }
    pub fn poll(&mut self) -> Result<()> {
        if self.error.is_some() {
            return Ok(());
        }
        if !self.opened && matches!(self.status(), ProcessStatus::Ready { .. }) {
            self.opened = true;
            self.send(Command::Open {
                directory: self.options.directory.clone(),
                history_id: self.options.history_id.clone(),
                window: self.window,
            })?;
        }
        let events: Vec<_> = self.shared.lock().unwrap().events.drain(..).collect();
        for output in events {
            match output {
                Output::Event(ComponentEvent::Toolbar {
                    scope: ToolbarScope::Window { window },
                    items,
                    ..
                }) if window == self.window => self.actions = items,
                Output::Response { request, result } => {
                    self.pending
                        .remove(&request)
                        .context("unmatched navigation response")?;
                    let reply: Reply = serde_json::from_value(result.map_err(anyhow::Error::msg)?)?;
                    self.reply(reply)?;
                }
                Output::Fatal { message } => bail!("{message}"),
                _ => {}
            }
        }
        if let ProcessStatus::Failed { message } = self.status() {
            bail!("navigation worker: {message}");
        }
        if !self.busy
            && let Some(entry) = self.seed.pop_front()
        {
            self.send(Command::Navigate(Action::Push(entry)))?;
        }
        Ok(())
    }
    fn reply(&mut self, reply: Reply) -> Result<()> {
        self.trace(json!({"event":"reply","reply":reply}));
        match reply {
            Reply::RestoreOffer(offer) => {
                self.host.input_open = false;
                if offer.history.entries.is_empty() {
                    self.seed = self.options.initial_entries.clone().into();
                }
                self.publication = Some(Publication::Restore(offer));
            }
            Reply::Execute(tx) => self.publication = Some(Publication::Execute(tx)),
            Reply::RecoveryReady(applied) => {
                self.host.ready(&applied)?;
                self.busy = false;
            }
            Reply::HistoryCommitted(history) => {
                let tx = self
                    .transaction
                    .take()
                    .context("commit without pending execution")?;
                self.host.committed(&tx, &history)?;
                self.busy = false;
                host_diag!(
                    "NAV_COMMITTED revision={} entry={:?} order={:?}",
                    history.revision,
                    history.current().map(|e| &e.id),
                    history.current().map(|e| &e.params)
                );
            }
            Reply::NeedReceipt(tx) => {
                let receipt = self.host.receipt(&tx)?;
                self.transaction = Some(tx);
                self.send(Command::Reconcile(receipt))?;
            }
            Reply::Aborted => {
                self.busy = false;
            }
            Reply::State { .. } => {}
        }
        Ok(())
    }
    pub fn publication_label(&self) -> Result<Option<String>> {
        let Some(publication) = &self.publication else {
            return Ok(None);
        };
        document_label(publication.target()).map(Some)
    }

    pub fn publication_view_command(&self) -> Result<Option<u64>> {
        self.publication
            .as_ref()
            .and_then(|p| p.target())
            .zip(self.options.view_command)
            .map(|(target, command)| command(target))
            .transpose()
    }

    pub fn title(&self) -> Option<&str> {
        self.publication
            .as_ref()
            .and_then(|p| p.target())
            .or(self.host.target.as_ref())
            .map(|e| {
                e.params
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(e.id.as_str())
            })
    }
    pub fn published(&mut self) -> Result<()> {
        let publication = self.publication.take().context("no pending publication")?;
        let context = format!(
            "window:{}:{}",
            self.session.host_epoch,
            document_label(publication.target())?
        );
        match publication {
            Publication::Restore(offer) => {
                let applied = self.host.restore(offer, |_| Ok(context))?;
                self.send(Command::RecoveryApplied(applied))?;
            }
            Publication::Execute(tx) => {
                let receipt = self.host.execute(tx.clone(), |_| Ok(context))?;
                self.transaction = Some(tx);
                self.send(Command::Applied(receipt))?;
            }
        }
        Ok(())
    }
    pub fn invoke(&mut self, source: &ComponentSource, command: u64) -> Result<()> {
        if self.busy || !self.host.input_open || self.error.is_some() {
            return Ok(());
        }
        ensure!(source.session == self.session, "stale navigation session");
        let id = self.enqueue_request(RequestBody::Invoke {
            command,
            registration: source.registration,
            window: source.window,
        })?;
        self.pending.insert(id, Command::Inspect);
        self.busy = true;
        self.trace(json!({"event":"toolbar-invocation","command":command,"session":source.session,"registration":source.registration,"window":source.window}));
        Ok(())
    }
}
fn append_bounded_trace_line(
    path: &std::path::Path,
    file: &mut Option<std::fs::File>,
    bytes_written: &mut u64,
    line: &str,
    max_bytes: u64,
) {
    use std::io::Write;
    let line_len = line.len() as u64 + 1;
    if *bytes_written > 0 && bytes_written.saturating_add(line_len) > max_bytes {
        if let Some(mut old) = file.take() {
            let _ = old.flush();
        }
        let rotated = std::path::PathBuf::from(format!("{}.1", path.display()));
        let _ = std::fs::rename(path, rotated);
        *file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .ok();
        *bytes_written = 0;
    }
    if let Some(active) = file.as_mut()
        && writeln!(active, "{line}").is_ok()
    {
        let _ = active.flush();
        *bytes_written = bytes_written.saturating_add(line_len);
    }
}
fn document_label(target: Option<&Entry>) -> Result<String> {
    let Some(entry) = target else {
        return Ok("empty".into());
    };
    ensure!(
        !entry.route.trim().is_empty() && !entry.id.trim().is_empty(),
        "invalid route identity"
    );
    Ok(entry
        .params
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&entry.id)
        .to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_navigation_has_no_layout_parameters() {
        let entry = Entry {
            id: "Document A".into(),
            route: "demo-document".into(),
            params: json!({"document_id":"a"}),
        };
        assert_eq!(document_label(Some(&entry)).unwrap(), "Document A");
    }
    #[test]
    fn trace_log_rotates_past_byte_cap() {
        let dir = std::env::temp_dir().join(format!(
            "tekes-nav-trace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("window-events.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        let mut bytes_written = 0;
        for i in 0..8 {
            append_bounded_trace_line(
                &path,
                &mut file,
                &mut bytes_written,
                &format!("entry-{i:02}"),
                24,
            );
        }
        drop(file);
        let active = std::fs::read_to_string(&path).unwrap();
        let rotated = std::fs::read_to_string(dir.join("window-events.jsonl.1")).unwrap();
        assert!(active.len() <= 24);
        assert!(rotated.len() <= 24);
        assert!(active.contains("entry-07"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
