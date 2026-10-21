//! Nonblocking window adapter. The module alone owns history and files.
use crate::{navigation::*, text::Text};
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    time::{SystemTime, UNIX_EPOCH},
};
use tekes_ui_controls::{
    ToolbarControl, ToolbarIconAction,
    composition::{ComponentId, Content, ContentEvent, ContentHost},
};
use tekes_ui_module_runtime::{
    ProcessComponent, ProcessComponentOptions, ProcessStatus, component::*,
};
use tekes_ui_platform_macos::WindowWake;
use tekes_ui_render::Renderer;
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

pub struct NavigationOptions {
    pub directory: std::path::PathBuf,
    pub history_id: String,
    /// Demo routes, navigated normally only when the history is empty.
    pub initial_entries: Vec<Entry>,
}
#[derive(Clone)]
pub enum Publication {
    Restore(Offer),
    Execute(Transaction),
}
impl Publication {
    fn target(&self) -> Option<&Entry> {
        match self {
            Self::Restore(o) => o.history.current(),
            Self::Execute(t) => t.target.as_ref(),
        }
    }
}
pub struct WindowNavigation {
    pub process: ProcessComponent,
    pub host: NavigationHost,
    pub toolbar: ContentHost,
    pub rect: Rect,
    pub busy: bool,
    pub publication: Option<Publication>,
    options: NavigationOptions,
    opened: bool,
    pending: HashMap<u64, Command>,
    actions: Vec<ToolbarAction>,
    images: HashMap<(String, u32), ImageId>,
    seed: VecDeque<Entry>,
    transaction: Option<Transaction>,
    pub error: Option<String>,
    trace_tx: std::sync::mpsc::SyncSender<String>,
}
impl WindowNavigation {
    pub fn new(options: NavigationOptions, wake: WindowWake) -> Result<Self> {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        let process =
            ProcessComponent::spawn(ProcessComponentOptions::new(MODULE_ID, epoch), move || {
                wake.wake();
            })?;
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
        let mut toolbar = ContentHost::new(Content::new(ComponentId(1), ToolbarControl::window()))
            .map_err(|e| anyhow::anyhow!("toolbar: {e:?}"))?;
        toolbar.set_action_events(true);
        Ok(Self {
            process,
            host,
            toolbar,
            rect: Rect::default(),
            busy: true,
            publication: None,
            options,
            opened: false,
            pending: HashMap::new(),
            actions: vec![],
            images: HashMap::new(),
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
                window: 1,
            })?;
        }
        for p in self.process.drain_events() {
            match p.output {
                Output::Event(ComponentEvent::Toolbar {
                    scope: ToolbarScope::Window { window: 1 },
                    items,
                    ..
                }) => self.actions = items,
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
    pub fn event(&mut self, event: &ContentEvent) -> Result<bool> {
        self.toolbar.event(event);
        let bubbles = self.toolbar.event_bubbled();
        let action = self
            .toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap()
            .take_action();
        self.toolbar.drain_actions();
        if action.is_some() {
            self.trace(json!({"event":"toolbar-action","action":action,"busy":self.busy,"input_open":self.host.input_open}));
        }
        if let Some(action) = action
            && !self.busy
            && self.host.input_open
        {
            let id = self.process.invoke(action)?;
            // The transport correlates the original registration/window/command.
            self.pending.insert(id, Command::Inspect);
            self.busy = true;
        }
        Ok(bubbles)
    }
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        scale: f32,
        viewport: Rect,
        text: &mut Text,
    ) -> Result<f32> {
        let mut actions = vec![];
        for a in &self.actions {
            let name = a
                .icon
                .as_ref()
                .context("navigation action requires an icon")?;
            let image = crate::window_toolbar::icon_image(&mut self.images, renderer, name, scale)?;
            actions.push(ToolbarIconAction {
                id: a.command.id,
                image,
                label: a
                    .accessible_name
                    .clone()
                    .context("navigation accessible name")?,
                enabled: a.command.enabled
                    && !self.busy
                    && self.host.input_open
                    && self.error.is_none(),
            });
        }
        let toolbar = self
            .toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        toolbar.set_viewport(viewport);
        toolbar.set_icon_actions(actions);
        Ok(toolbar.measure(text).height)
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut Text,
        rect: Rect,
        viewport: Rect,
    ) -> Result<()> {
        self.rect = rect;
        self.toolbar
            .render(scene, text, rect, viewport, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("navigation toolbar: {e:?}"))?;
        Ok(())
    }
    pub fn overlay(&mut self, scene: &mut Scene, text: &mut Text) {
        self.toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .paint_overlays(scene, text, Palette::from(Theme::LIGHT).into());
    }
}
fn document_label(target: Option<&Entry>) -> Result<String> {
    let Some(entry) = target else {
        return Ok("empty".into());
    };
    ensure!(entry.route == "demo-document", "route not installed");
    ensure!(
        entry
            .params
            .get("document_id")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty()),
        "missing document ID"
    );
    Ok(entry.id.clone())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_navigation_has_no_layout_parameters() {
        let e = Entry {
            id: "Document A".into(),
            route: "demo-document".into(),
            params: json!({"document_id":"a"}),
        };
        assert_eq!(document_label(Some(&e)).unwrap(), "Document A");
        let mut old = e;
        old.route = "panel-layout".into();
        assert!(document_label(Some(&old)).is_err());
    }
}
