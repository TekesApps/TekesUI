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
    composition::{ComponentId, Content, ContentEvent, ContentHost, TextContent, ToolbarPlacement},
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
    /// Consumer route adapter: select the center module's document using its
    /// existing command channel. None keeps navigation independent of View data.
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
    pub toolbar: ContentHost,
    pub title_toolbar: ContentHost,
    pub external_toolbar: ContentHost,
    pub rect: Rect,
    pub busy: bool,
    pub publication: Option<Publication>,
    options: NavigationOptions,
    opened: bool,
    pending: HashMap<u64, Command>,
    actions: Vec<ToolbarAction>,
    images: HashMap<(String, u32), ImageId>,
    external_actions: Vec<ToolbarAction>,
    delegated_actions: VecDeque<u64>,
    seed: VecDeque<Entry>,
    transaction: Option<Transaction>,
    pub error: Option<String>,
    trace_tx: std::sync::mpsc::SyncSender<String>,
}
fn navigation_toolbar() -> Content {
    crate::window_toolbar::command_group(ToolbarPlacement::Navigation)
}
fn breadcrumb_toolbar() -> Content {
    crate::window_toolbar::title_group()
}
impl WindowNavigation {
    /// Presentation supplied by the active content consumer, not another history entry.
    pub fn set_breadcrumb(&mut self, title: &str) {
        let label = self
            .title_toolbar
            .get_mut(ComponentId(2))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap();
        if label.text != title {
            label.text = title.into();
        }
    }

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
        let mut external_toolbar = ContentHost::new(crate::window_toolbar::command_group(
            ToolbarPlacement::SecondaryAction,
        ))
        .map_err(|e| anyhow::anyhow!("external toolbar: {e:?}"))?;
        external_toolbar.set_action_events(true);
        let mut toolbar = ContentHost::new(navigation_toolbar())
            .map_err(|e| anyhow::anyhow!("toolbar: {e:?}"))?;
        toolbar.set_action_events(true);
        Ok(Self {
            process,
            host,
            toolbar,
            external_toolbar,
            title_toolbar: ContentHost::new(breadcrumb_toolbar())
                .map_err(|e| anyhow::anyhow!("title toolbar: {e:?}"))?,
            rect: Rect::default(),
            busy: true,
            publication: None,
            options,
            opened: false,
            pending: HashMap::new(),
            actions: vec![],
            images: HashMap::new(),
            external_actions: vec![],
            delegated_actions: VecDeque::new(),
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
    pub fn event(&mut self, event: &ContentEvent) -> Result<bool> {
        self.toolbar.event(event);
        let bubbles = self.toolbar.event_bubbled();
        self.drain_action()?;
        Ok(bubbles)
    }
    /// AppWindow assembles registered commands; each component keeps its own state/IPC.
    pub fn set_external_actions(&mut self, actions: Vec<ToolbarAction>) {
        self.external_actions = actions;
    }
    pub fn take_external_action(&mut self) -> Option<u64> {
        self.delegated_actions.pop_front()
    }
    pub fn external_event(&mut self, event: &ContentEvent) -> Result<bool> {
        self.external_toolbar.event(event);
        let bubbles = self.external_toolbar.event_bubbled();
        self.drain_external_action();
        Ok(bubbles)
    }
    pub fn drain_external_action(&mut self) {
        let action = self
            .external_toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap()
            .take_action();
        self.external_toolbar.drain_actions();
        if let Some(action) = action
            && !self.busy
            && self.host.input_open
            && self.error.is_none()
            && self
                .external_actions
                .iter()
                .any(|a| a.command.id == action && a.command.enabled)
        {
            self.delegated_actions.push_back(action);
        }
    }
    pub fn drain_action(&mut self) -> Result<()> {
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
        Ok(())
    }
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        scale: f32,
        viewport: Rect,
        text: &mut Text,
    ) -> Result<tekes_ui_scene::Size> {
        let mut actions = vec![];
        for a in &self.actions {
            let name = a
                .icon
                .as_ref()
                .context("navigation action requires an icon")?;
            let image = crate::window_toolbar::icon_image(
                &mut self.images,
                renderer,
                name,
                scale * text.type_scale(),
            )?;
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
        let more = crate::window_toolbar::icon_image(
            &mut self.images,
            renderer,
            "tekes-more",
            scale * text.type_scale(),
        )?;
        let toolbar = self
            .toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        toolbar.set_viewport(viewport);
        toolbar.set_overflow_icon(Some(more));
        toolbar.set_icon_actions(actions);
        for action in &self.actions {
            toolbar.set_action_help(action.command.id, action.accessible_name.clone());
        }
        let height = toolbar.measure(text).height;
        let size = self
            .toolbar
            .get_mut(ComponentId(10))
            .unwrap()
            .measure(text, viewport.size)
            .size;
        Ok(tekes_ui_scene::Size::new(
            size.width,
            size.height.max(height),
        ))
    }
    pub fn prepare_external(
        &mut self,
        renderer: &mut Renderer,
        scale: f32,
        viewport: Rect,
        text: &mut Text,
    ) -> Result<tekes_ui_scene::Size> {
        let mut actions = vec![];
        for a in &self.external_actions {
            let image = crate::window_toolbar::icon_image(
                &mut self.images,
                renderer,
                a.icon
                    .as_ref()
                    .context("external action requires an icon")?,
                scale * text.type_scale(),
            )?;
            actions.push(ToolbarIconAction {
                id: a.command.id,
                image,
                label: a
                    .accessible_name
                    .clone()
                    .context("external accessible name")?,
                enabled: a.command.enabled
                    && !self.busy
                    && self.host.input_open
                    && self.error.is_none(),
            });
        }
        let more = crate::window_toolbar::icon_image(
            &mut self.images,
            renderer,
            "tekes-more",
            scale * text.type_scale(),
        )?;
        let toolbar = self
            .external_toolbar
            .get_mut(ComponentId(1))
            .unwrap()
            .component_mut::<ToolbarControl>()
            .unwrap();
        toolbar.set_viewport(viewport);
        toolbar.set_overflow_icon(Some(more));
        toolbar.set_icon_actions(actions);
        for a in &self.external_actions {
            toolbar.set_action_help(a.command.id, a.accessible_name.clone());
        }
        Ok(self
            .external_toolbar
            .get_mut(ComponentId(10))
            .unwrap()
            .measure(text, viewport.size)
            .size)
    }
    pub fn render_external(
        &mut self,
        scene: &mut Scene,
        text: &mut Text,
        rect: Rect,
        viewport: Rect,
    ) -> Result<()> {
        self.external_toolbar
            .render(scene, text, rect, viewport, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("external toolbar: {e:?}"))
    }
    pub fn prepare_title(&mut self, text: &mut Text, viewport: Rect) -> tekes_ui_scene::Size {
        self.title_toolbar
            .get_mut(ComponentId(10))
            .unwrap()
            .measure(text, viewport.size)
            .size
    }
    pub fn render_title(
        &mut self,
        scene: &mut Scene,
        text: &mut Text,
        rect: Rect,
        viewport: Rect,
    ) -> Result<()> {
        self.title_toolbar
            .render(scene, text, rect, viewport, Theme::LIGHT.into())
            .map_err(|e| anyhow::anyhow!("title toolbar: {e:?}"))
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
        if !self.external_actions.is_empty() {
            self.external_toolbar
                .get_mut(ComponentId(1))
                .unwrap()
                .paint_overlays(scene, text, Palette::from(Theme::LIGHT).into());
        }
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
    #[test]
    fn navbar_commands_and_center_title_are_separate_standard_navigation_contributions() {
        let commands = ContentHost::new(navigation_toolbar()).unwrap();
        let title = ContentHost::new(breadcrumb_toolbar()).unwrap();
        for host in [&commands, &title] {
            assert_eq!(
                host.get(ComponentId(11))
                    .unwrap()
                    .component::<ToolbarGroup>()
                    .unwrap()
                    .placement,
                ToolbarPlacement::Navigation
            );
        }
        assert!(
            commands
                .get(ComponentId(1))
                .unwrap()
                .component::<ToolbarControl>()
                .is_some()
        );
        assert!(commands.get(ComponentId(2)).is_none());
        assert!(
            title
                .get(ComponentId(2))
                .unwrap()
                .component::<TextContent>()
                .is_some()
        );
        assert!(title.get(ComponentId(1)).is_none());
    }

    use super::*;
    use tekes_ui_controls::composition::ToolbarGroup;
    #[test]
    fn editing_commands_keep_independent_group_and_original_action_owner() {
        use tekes_ui_controls::composition::{AccessibilityAction, Component};
        let mut navigation = ContentHost::new(navigation_toolbar()).unwrap();
        let mut editing = ContentHost::new(crate::window_toolbar::command_group(
            ToolbarPlacement::SecondaryAction,
        ))
        .unwrap();
        assert_eq!(
            editing
                .get(ComponentId(11))
                .unwrap()
                .component::<ToolbarGroup>()
                .unwrap()
                .placement,
            ToolbarPlacement::SecondaryAction
        );
        let mut text = Text::new();
        let viewport = Rect::new(0., 0., 1100., 640.);
        for (host, id, x) in [
            (&mut navigation, 1, 100.),
            (&mut editing, crate::settings_undo::UNDO, 900.),
        ] {
            let control = host
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap();
            control.set_icon_actions(vec![ToolbarIconAction {
                id,
                image: ImageId(1),
                label: format!("action {id}"),
                enabled: true,
            }]);
            host.render(
                &mut Scene::new(),
                &mut text,
                Rect::new(x, 0., 120., 36.),
                viewport,
                Theme::LIGHT.into(),
            )
            .unwrap();
        }
        let part = editing
            .get(ComponentId(1))
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap()
            .semantic_parts()[0]
            .index;
        assert!(
            editing
                .accessibility_action(ComponentId(1), Some(part), AccessibilityAction::Press)
                .consumed
        );
        assert_eq!(
            editing
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap()
                .take_action(),
            Some(crate::settings_undo::UNDO)
        );
        assert_eq!(
            navigation
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap()
                .take_action(),
            None
        );
        editing.focus(Some(ComponentId(1)), true);
        assert_eq!(navigation.focused(), None);
    }

    #[test]
    fn constrained_registered_commands_keep_shared_overflow_open_across_prepare() {
        use tekes_ui_controls::composition::{AccessibilityAction, Component};
        let mut host = ContentHost::new(navigation_toolbar()).unwrap();
        let mut text = Text::new();
        let viewport = Rect::new(0., 0., 1100., 640.);
        let actions: Vec<_> = (0..4)
            .map(|i| ToolbarIconAction {
                id: i + 1,
                image: ImageId((i + 1) as u32),
                label: format!("command {i}"),
                enabled: i == 3,
            })
            .collect();
        let row = ToolbarControl::window().row_height(&mut text);
        let frame = Rect::new(128., 0., 147., row);
        let prepare = |host: &mut ContentHost, text: &mut Text| {
            let bar = host
                .get_mut(ComponentId(1))
                .unwrap()
                .component_mut::<ToolbarControl>()
                .unwrap();
            bar.set_viewport(viewport);
            bar.set_icon_actions(actions.clone());
            bar.set_overflow_icon(Some(ImageId(99)));
            host.get_mut(ComponentId(10))
                .unwrap()
                .measure(text, viewport.size);
        };
        prepare(&mut host, &mut text);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ToolbarControl>()
                .unwrap()
                .semantic_parts()
                .iter()
                .any(|p| p.index == 0)
        );
        assert!(
            host.accessibility_action(ComponentId(1), Some(0), AccessibilityAction::Press)
                .consumed
        );
        assert!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ToolbarControl>()
                .unwrap()
                .is_open()
        );
        prepare(&mut host, &mut text);
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        assert!(
            host.get(ComponentId(1))
                .unwrap()
                .component::<ToolbarControl>()
                .unwrap()
                .is_open()
        );
    }

    #[test]
    fn document_navigation_has_no_layout_parameters() {
        let e = Entry {
            id: "Document A".into(),
            route: "demo-document".into(),
            params: json!({"document_id":"a"}),
        };
        assert_eq!(document_label(Some(&e)).unwrap(), "Document A");
        let mut old = e;
        old.route.clear();
        assert!(document_label(Some(&old)).is_err());
    }
}
