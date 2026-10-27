//! Nonblocking window adapter. The module alone owns history and files.
use crate::{navigation::*, toolbar_source::ComponentSource};
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    time::{SystemTime, UNIX_EPOCH},
};
use tekes_ui_module_runtime::{
    ProcessComponent, ProcessComponentOptions, ProcessStatus, component::*,
};
use tekes_ui_platform_macos::WindowWake;

pub struct NavigationOptions {
    pub directory: std::path::PathBuf,
    pub history_id: String,
    /// Demo routes, navigated normally only when the history is empty.
    pub initial_entries: Vec<Entry>,
    /// Consumer route adapter for the center column's existing command channel.
    /// None keeps navigation independent of View data.
    pub view_command: Option<fn(&Entry) -> Result<u64>>,
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
pub struct WindowNavigation {
    pub process: ProcessComponent,
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
    trace_tx: std::sync::mpsc::SyncSender<String>,
}
impl WindowNavigation {
    pub fn new(options: NavigationOptions, wake: WindowWake, window: u64) -> Result<Self> {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        let process = ProcessComponent::spawn_in_window(
            ProcessComponentOptions::new(MODULE_ID, epoch),
            window,
            move || {
                wake.wake();
            },
        )?;
        let host = NavigationHost::new(process.session().clone());
        let (trace_tx, trace_rx) = std::sync::mpsc::sync_channel::<String>(256);
        let trace_path = options.directory.join("window-events.jsonl");
        std::thread::spawn(move || {
            use std::io::Write;
            if let Some(parent) = trace_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(trace_path)
            {
                while let Ok(line) = trace_rx.recv() {
                    let _ = writeln!(file, "{line}");
                    let _ = file.flush();
                }
            }
        });
        Ok(Self {
            process,
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
            trace_tx,
        })
    }
    pub fn trace(&self, value: serde_json::Value) {
        let _ = self.trace_tx.try_send(value.to_string());
    }
    pub fn send(&mut self, command: Command) -> Result<()> {
        self.trace(json!({"event":"send","command":command}));
        let id = self.process.request(RequestBody::Message {
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
        if !self.opened && matches!(self.process.status(), ProcessStatus::Ready { .. }) {
            self.opened = true;
            self.send(Command::Open {
                directory: self.options.directory.clone(),
                history_id: self.options.history_id.clone(),
                window: self.window,
            })?;
        }
        for p in self.process.drain_events() {
            match p.output {
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
        if let ProcessStatus::Failed { message } = self.process.status() {
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
                eprintln!(
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
            self.process.session().host_epoch,
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
        let id = source.invoke(&self.process, command)?;
        self.pending.insert(id, Command::Inspect);
        self.busy = true;
        self.trace(json!({"event":"toolbar-invocation","command":command,"session":source.session,"registration":source.registration,"window":source.window}));
        Ok(())
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
}
