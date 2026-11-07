//! Window container assembly. Local views retain their original ContentHost;
//! process containers retain the compiled module registry and isolated transport.
use anyhow::Result;
use tekes_ui_controls::{
    ButtonTextRenderer,
    composition::{Content, ContentActionRecord, ContentEvent, ContentHost},
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;

/// Borrow a current original host for one operation. Shared application state
/// can expose its own host without cloning controls or leaking RefCell guards.
pub trait ContainerContent {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost));
}
impl ContainerContent for ContentHost {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        visit(self);
    }
}
type CommandHandler = Box<dyn FnMut(&mut ContentHost, u64) -> Result<()>>;
type ActionHandler = Box<dyn FnMut(&mut ContentHost, Vec<ContentActionRecord>)>;
pub struct LocalContainer {
    id: String,
    source: Box<dyn ContainerContent>,
    action: Option<ActionHandler>,
    command: Option<CommandHandler>,
    pub(crate) toolbar: Vec<tekes_ui_app_protocol::ToolbarItem>,
    pub(crate) frame: Rect,
    pub(crate) options: ContainerOptions,
}
impl LocalContainer {
    pub fn new(id: impl Into<String>, content: Content) -> Result<Self> {
        let mut host =
            ContentHost::new(content).map_err(|e| anyhow::anyhow!("Container: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self::from_source(id, host))
    }
    pub fn from_source(id: impl Into<String>, source: impl ContainerContent + 'static) -> Self {
        Self {
            id: id.into(),
            source: Box::new(source),
            action: None,
            command: None,
            toolbar: vec![],
            frame: Rect::default(),
            options: ContainerOptions::default(),
        }
    }
    pub fn options(mut self, options: ContainerOptions) -> Self {
        self.options = options;
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = Some(title.into());
        self
    }
    pub fn list_height(mut self, height: f32) -> Self {
        self.options.list_height = Some(height);
        self
    }
    pub fn retain_button(mut self, command: u64, target: impl Into<String>) -> Self {
        self.options.retained_buttons.push(RetainedButton {
            command,
            target: target.into(),
        });
        self
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn on_action(
        mut self,
        handler: impl FnMut(&mut ContentHost, Vec<ContentActionRecord>) + 'static,
    ) -> Self {
        self.action = Some(Box::new(handler));
        self
    }
    /// Declare shared toolbar actions; the original ToolbarControl owns input.
    pub fn toolbar(mut self, items: Vec<tekes_ui_app_protocol::ToolbarItem>) -> Self {
        self.toolbar = items;
        self
    }
    pub fn on_command(
        mut self,
        handler: impl FnMut(&mut ContentHost, u64) -> Result<()> + 'static,
    ) -> Self {
        self.command = Some(Box::new(handler));
        self
    }
    pub(crate) fn command(&mut self, id: u64) -> Result<()> {
        let callback = self
            .command
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("column {} has no command binding", self.id))?;
        let mut result = Ok(());
        self.source.with_host(&mut |h| {
            result = callback(h, id);
        });
        result
    }
    pub(crate) fn dispatch_actions(&mut self) {
        let callback = &mut self.action;
        if let Some(callback) = callback {
            self.source.with_host(&mut |h| {
                let records = h.drain_action_records();
                callback(h, records);
            });
        }
    }
    pub(crate) fn cancel(&mut self) {
        self.source.with_host(&mut |h| {
            h.cancel();
            h.focus(None, false);
        });
    }
    pub fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        self.source.with_host(visit);
    }
    pub(crate) fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        palette: Palette,
    ) -> Result<()> {
        if self.frame != frame {
            self.source.with_host(&mut |h| {
                h.cancel_pointer_capture();
            });
        }
        self.frame = frame;
        let mut result = Ok(());
        self.source.with_host(&mut |h| {
            result = h
                .render(scene, text, frame, clip, palette)
                .map_err(|e| anyhow::anyhow!("Container render: {e:?}"))
        });
        result
    }
    pub(crate) fn event(
        &mut self,
        event: &ContentEvent,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<()> {
        let mut result = Ok(());
        let callback = &mut self.action;
        self.source.with_host(&mut |h| {
            result = h
                .event_with_text(event, text)
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("Container input: {e:?}"));
            if let Some(callback) = callback {
                let records = h.drain_action_records();
                callback(h, records);
            }
        });
        result
    }
}
/// Presentation metadata belongs to the container, independently of its backing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContainerOptions {
    pub title: Option<String>,
    /// Opt in to List with this content allocation (including its Toolbar).
    pub list_height: Option<f32>,
    /// Explicit destinations; there is no automatic first-visible fallback.
    pub retained_buttons: Vec<RetainedButton>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedButton {
    pub command: u64,
    pub target: String,
}
/// The factory remains compiled into the consumer; no runtime module discovery.
pub struct ProcessContainer {
    pub(crate) definition: crate::ModuleDefinition,
    pub(crate) options: ContainerOptions,
}
impl ProcessContainer {
    pub fn new(definition: crate::ModuleDefinition) -> Self {
        Self {
            definition,
            options: ContainerOptions::default(),
        }
    }
    pub fn options(mut self, options: ContainerOptions) -> Self {
        self.options = options;
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = Some(title.into());
        self
    }
    pub fn list_height(mut self, height: f32) -> Self {
        self.options.list_height = Some(height);
        self
    }
    pub fn retain_button(mut self, command: u64, target: impl Into<String>) -> Self {
        self.options.retained_buttons.push(RetainedButton {
            command,
            target: target.into(),
        });
        self
    }
}
/// A stable window container backed by local Content or a fixed process factory.
pub enum Container {
    Local(LocalContainer),
    Process(ProcessContainer),
}
impl Container {
    pub fn id(&self) -> &str {
        match self {
            Self::Local(c) => c.id(),
            Self::Process(p) => p.definition.id,
        }
    }
    pub fn container_options(&self) -> &ContainerOptions {
        match self {
            Self::Local(c) => &c.options,
            Self::Process(p) => &p.options,
        }
    }
}
impl From<LocalContainer> for Container {
    fn from(value: LocalContainer) -> Self {
        Self::Local(value)
    }
}
impl From<ProcessContainer> for Container {
    fn from(value: ProcessContainer) -> Self {
        Self::Process(value)
    }
}
/// Legacy horizontal assembly descriptor. Converts into the same runtime;
/// new applications use Container and AppWindow::from_containers.
pub enum Column {
    Container(LocalContainer),
    ProcessContainer(ProcessContainer),
}
impl Column {
    pub fn id(&self) -> &str {
        match self {
            Self::Container(c) => c.id(),
            Self::ProcessContainer(p) => p.definition.id,
        }
    }
    pub fn options(&self) -> &ContainerOptions {
        match self {
            Self::Container(c) => &c.options,
            Self::ProcessContainer(p) => &p.options,
        }
    }
}
impl From<LocalContainer> for Column {
    fn from(value: LocalContainer) -> Self {
        Self::Container(value)
    }
}
impl From<ProcessContainer> for Column {
    fn from(value: ProcessContainer) -> Self {
        Self::ProcessContainer(value)
    }
}
impl From<Container> for Column {
    fn from(value: Container) -> Self {
        match value {
            Container::Local(c) => Self::Container(c),
            Container::Process(p) => Self::ProcessContainer(p),
        }
    }
}
impl From<Column> for Container {
    fn from(value: Column) -> Self {
        match value {
            Column::Container(c) => Self::Local(c),
            Column::ProcessContainer(p) => Self::Process(p),
        }
    }
}
pub(crate) fn validate(columns: &[Column]) -> Result<()> {
    anyhow::ensure!(
        (1..=3).contains(&columns.len()),
        "legacy horizontal AppWindow requires one to three columns"
    );
    validate_ids(columns.iter().map(|c| (c.id(), c.options())))
}
pub(crate) fn validate_containers(containers: &[Container]) -> Result<()> {
    validate_ids(containers.iter().map(|c| (c.id(), c.container_options())))
}
pub(crate) fn validate_ids<'a>(
    containers: impl IntoIterator<Item = (&'a str, &'a ContainerOptions)>,
) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    let mut destinations = Vec::new();
    for (id, options) in containers {
        anyhow::ensure!(
            !id.is_empty()
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid container ID"
        );
        anyhow::ensure!(ids.insert(id), "duplicate container ID: {id}");
        anyhow::ensure!(
            options.list_height.is_none_or(|h| h.is_finite() && h > 0.),
            "invalid List height for {id}"
        );
        let mut commands = std::collections::HashSet::new();
        for button in &options.retained_buttons {
            anyhow::ensure!(
                commands.insert(button.command),
                "duplicate retained command {} for {id}",
                button.command
            );
            destinations.push(button.target.as_str());
        }
    }
    anyhow::ensure!(!ids.is_empty(), "AppWindow requires at least one container");
    for target in destinations {
        anyhow::ensure!(
            ids.contains(target),
            "unknown retained-button target: {target}"
        );
    }
    Ok(())
}

pub(crate) fn text_state(host: &ContentHost) -> Option<tekes_ui_platform_macos::TextInputState> {
    use tekes_ui_platform_macos::{Point, Size, TextInputState};
    let surface = host.focused_text()?;
    let caret = surface.caret();
    let len = surface.input.len();
    Some(TextInputState {
        caret_index: surface.input.caret,
        text: surface.input.text.clone(),
        selection: surface.input.selection(),
        marked: surface.input.marked.clone(),
        read_only: surface.input.read_only,
        caret_origin: Point {
            x: f64::from(caret.min_x()),
            y: f64::from(caret.min_y()),
        },
        caret_size: Size {
            width: f64::from(caret.size.width),
            height: f64::from(caret.size.height),
        },
        carets: (0..=len)
            .map(|i| {
                let r = surface.caret_doc(i);
                let o = surface.origin();
                (
                    i,
                    Point {
                        x: f64::from(o.x + r.min_x()),
                        y: f64::from(o.y + r.min_y()),
                    },
                    Size {
                        width: f64::from(r.size.width),
                        height: f64::from(r.size.height),
                    },
                )
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_controls::composition::{ComponentId, TextFieldContent};
    use tekes_ui_tokens::Theme;
    fn local(id: &str) -> LocalContainer {
        LocalContainer::new(
            id,
            Content::new(ComponentId(1), TextFieldContent::new("Notes", "hello")),
        )
        .unwrap()
    }
    #[test]
    fn fixed_column_assembly_validates_count_ids_and_process_registry() {
        assert!(crate::AppWindow::from_columns(vec![]).is_err());
        assert!(
            crate::AppWindow::from_columns(vec![local("same").into(), local("same").into()])
                .is_err()
        );
        assert!(crate::AppWindow::from_columns(vec![local("bad/id").into()]).is_err());
        assert!(
            crate::AppWindow::from_columns(
                (0..4).map(|i| local(&format!("c{i}")).into()).collect()
            )
            .is_err()
        );
        let local_app = crate::AppWindow::from_columns(vec![local("one").into()]).unwrap();
        assert!(local_app.profile.modules.is_empty());
        // This definition is only inspected; an isolated factory is never called in Host.
        let process = crate::ModuleDefinition {
            id: "remote",
            create: || panic!("factory must run in worker"),
        };
        let app = crate::AppWindow::from_columns(vec![
            local("left").into(),
            ProcessContainer::new(process).into(),
            local("right").into(),
        ])
        .unwrap();
        assert_eq!(
            app.profile.modules.iter().map(|m| m.id).collect::<Vec<_>>(),
            ["remote"]
        );
    }
    #[test]
    fn container_assembly_has_no_legacy_three_column_limit_and_keeps_registry() {
        let containers = (0..6)
            .map(|i| Container::from(local(&format!("c{i}"))))
            .collect();
        let app = crate::AppWindow::from_containers(
            containers,
            crate::PresentationOptions::new("c0").others(crate::OthersOptions::new(
                (1..6).map(|i| format!("c{i}")).collect(),
            )),
        )
        .unwrap();
        assert_eq!(app.columns.as_ref().unwrap().len(), 6);
        assert!(app.profile.modules.is_empty());
    }
    #[test]
    fn retained_button_routes_require_known_targets_and_unique_owner_commands() {
        let invalid: Vec<Container> = vec![local("a").retain_button(42, "missing").into()];
        assert!(validate_containers(&invalid).is_err());
        let duplicate: Vec<Container> = vec![
            local("a")
                .retain_button(42, "b")
                .retain_button(42, "a")
                .into(),
            local("b").into(),
        ];
        assert!(validate_containers(&duplicate).is_err());
        let valid: Vec<Container> =
            vec![local("a").retain_button(42, "b").into(), local("b").into()];
        validate_containers(&valid).unwrap();
        assert_eq!(valid[0].container_options().retained_buttons[0].target, "b");
    }
    #[test]
    fn local_original_text_host_accepts_input_and_resize_cancel_without_ipc() {
        let mut c = local("notes");
        let mut text = crate::text::Text::new();
        let frame = Rect::new(20., 60., 240., 160.);
        c.render(
            &mut Scene::new(),
            &mut text,
            frame,
            frame,
            Theme::LIGHT.into(),
        )
        .unwrap();
        c.with_host(&mut |h| {
            h.focus(Some(ComponentId(1)), false);
        });
        c.event(&ContentEvent::TextCommand("selectAll:".into()), &mut text)
            .unwrap();
        c.event(
            &ContentEvent::Commit {
                text: "本地 mixed".into(),
                replacement: None,
            },
            &mut text,
        )
        .unwrap();
        c.with_host(&mut |h| assert_eq!(h.focused_text().unwrap().input.text, "本地 mixed"));
        let next = Rect::new(40., 60., 180., 160.);
        c.render(
            &mut Scene::new(),
            &mut text,
            next,
            next,
            Theme::LIGHT.into(),
        )
        .unwrap();
        c.with_host(&mut |h| {
            assert_eq!(h.get(ComponentId(1)).unwrap().frame(), next);
            assert!(
                tekes_ui_platform_macos::content_accessibility::content_nodes(h)
                    .iter()
                    .any(|n| n.label == "Notes")
            );
        });
        c.cancel();
        c.with_host(&mut |h| assert!(h.focused().is_none()));
    }
    #[test]
    fn local_toolbar_command_runs_once_and_rejects_missing_binding() {
        use std::{cell::Cell, rc::Rc};
        let count = Rc::new(Cell::new(0));
        let record = count.clone();
        let mut c = local("commands").on_command(move |_, id| {
            assert_eq!(id, 42);
            record.set(record.get() + 1);
            Ok(())
        });
        c.command(42).unwrap();
        assert_eq!(count.get(), 1);
        assert!(local("unbound").command(42).is_err());
    }
}
