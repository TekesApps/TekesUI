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
    /// Optional consumer request into the existing headless navigation service.
    /// Called only while its input gate is open; publications must not push again.
    fn take_navigation_request(&mut self) -> Option<crate::navigation::Entry> {
        None
    }
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
    pub fn toolbar_host(mut self, host: ContainerToolbarHost) -> Self {
        self.options.toolbar_host = host;
        self
    }
    pub fn switcher_icon(mut self, icon: impl Into<String>) -> Self {
        self.options.switcher_icon = Some(icon.into());
        self
    }
    pub fn presentation(mut self, mode: ContainerPresentation) -> Self {
        self.options.list_height = match mode {
            ContainerPresentation::Tab => None,
            ContainerPresentation::List { height } => Some(height),
        };
        self
    }
    /// Compatibility shorthand for selecting List mode with this height.
    pub fn list_height(mut self, height: f32) -> Self {
        self.options.list_height = Some(height);
        self
    }
    /// Override the window's default outer content padding for this container.
    pub fn content_padding(mut self, padding: f32) -> Self {
        self.options.content_padding = Some(padding);
        self
    }
    /// Route this explicitly declared toolbar command through the window's
    /// current focus/ordinary-Others exchange instead of business dispatch.
    pub fn focus_switch(mut self, command: u64) -> Self {
        self.options.focus_switch_command = Some(command);
        self
    }
    pub fn secondary_commands(mut self, commands: Vec<u64>) -> Self {
        self.options.secondary_commands = commands;
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
    pub(crate) fn take_navigation_request(&mut self) -> Option<crate::navigation::Entry> {
        self.source.take_navigation_request()
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
    pub(crate) fn window_event(
        &mut self,
        event: &tekes_ui_platform_macos::Event,
        clipboard: &dyn tekes_ui_module_runtime::ContentClipboard,
        text: &mut dyn ButtonTextRenderer,
    ) -> Result<bool> {
        let mut result = Ok(false);
        let callback = &mut self.action;
        self.source.with_host(&mut |host| {
            result = tekes_ui_module_runtime::dispatch_content_window_event_with_text(
                host, event, clipboard, text,
            )
            .map(|response| response.is_some())
            .map_err(|e| anyhow::anyhow!("Container input: {e:?}"));
            if let Some(callback) = callback {
                let records = host.drain_action_records();
                callback(host, records);
            }
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
/// Where one container's business toolbar is presented. Window is the default;
/// applications relying on an internal bar explicitly choose Container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContainerToolbarHost {
    #[default]
    Window,
    Container {
        position: ContainerToolbarPosition,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContainerToolbarPosition {
    #[default]
    Top,
    Bottom,
    Leading,
    Trailing,
}
/// One Others presentation per container, independent of toolbar mounting.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ContainerPresentation {
    #[default]
    Tab,
    List {
        height: f32,
    },
}
/// Presentation metadata belongs to the container, independently of its backing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContainerOptions {
    pub title: Option<String>,
    /// Canonical icon for this single-instance destination.
    pub switcher_icon: Option<String>,
    pub toolbar_host: ContainerToolbarHost,
    pub focus_switch_command: Option<u64>,
    /// Existing toolbar commands presented only in this owner's More menu.
    pub secondary_commands: Vec<u64>,
    /// Opt in to List with this content allocation (including its Toolbar).
    /// Canonical mode storage: None is Tab; Some(height) is List only.
    pub list_height: Option<f32>,
    /// None retains the existing default; zero lets local Content own padding.
    pub content_padding: Option<f32>,
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
    pub(crate) instance_id: String,
    pub(crate) options: ContainerOptions,
}
impl ProcessContainer {
    /// Stable container identity; the compiled worker factory remains unchanged.
    pub fn instance_id(mut self, id: impl Into<String>) -> Self {
        self.instance_id = id.into();
        self
    }
    pub fn new(definition: crate::ModuleDefinition) -> Self {
        Self {
            instance_id: definition.id.into(),
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
    pub fn toolbar_host(mut self, host: ContainerToolbarHost) -> Self {
        self.options.toolbar_host = host;
        self
    }
    pub fn switcher_icon(mut self, icon: impl Into<String>) -> Self {
        self.options.switcher_icon = Some(icon.into());
        self
    }
    pub fn presentation(mut self, mode: ContainerPresentation) -> Self {
        self.options.list_height = match mode {
            ContainerPresentation::Tab => None,
            ContainerPresentation::List { height } => Some(height),
        };
        self
    }
    /// Compatibility shorthand for selecting List mode with this height.
    pub fn list_height(mut self, height: f32) -> Self {
        self.options.list_height = Some(height);
        self
    }
    /// Route this explicitly declared toolbar command through the window's
    /// current focus/ordinary-Others exchange instead of business dispatch.
    pub fn focus_switch(mut self, command: u64) -> Self {
        self.options.focus_switch_command = Some(command);
        self
    }
    pub fn secondary_commands(mut self, commands: Vec<u64>) -> Self {
        self.options.secondary_commands = commands;
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
            Self::Process(p) => &p.instance_id,
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
            Self::ProcessContainer(p) => &p.instance_id,
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
        anyhow::ensure!(
            options
                .content_padding
                .is_none_or(|p| p.is_finite() && p >= 0.),
            "invalid content padding for {id}"
        );
        anyhow::ensure!(
            options
                .switcher_icon
                .as_deref()
                .is_none_or(|icon| tekes_ui_icons::Glyph::named(icon).is_some()),
            "unknown switcher icon for {id}"
        );
        let mut secondary = std::collections::HashSet::new();
        for command in &options.secondary_commands {
            anyhow::ensure!(
                secondary.insert(command),
                "duplicate secondary command {command} for {id}"
            );
        }
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
    tekes_ui_module_runtime::content_text_input_state(host)
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

#[cfg(test)]
mod text_state_tests {
    use super::*;
    use tekes_ui_controls::composition::*;
    use tekes_ui_scene::Size;
    use tekes_ui_tokens::Theme;

    const FIELD: ComponentId = ComponentId(27301);

    fn field() -> Content {
        Content::new(FIELD, TextFieldContent::new("Editor", "A😀B"))
    }

    fn render(host: &mut ContentHost, frame: Rect, clip: Rect) {
        host.render(
            &mut Scene::new(),
            &mut crate::text::Text::new(),
            frame,
            clip,
            Theme::LIGHT.into(),
        )
        .unwrap();
    }

    #[test]
    fn local_shaped_boundaries_and_editing_state_reach_platform_unchanged() {
        let mut host = ContentHost::new(field()).unwrap();
        let frame = Rect::new(0., 0., 300., 80.);
        render(&mut host, frame, frame);
        host.focus(Some(FIELD), true);
        let input = &mut host
            .get_mut(FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        input.select(1, false);
        input.select(3, true);
        input.marked = Some(1..3);
        let state = text_state(&host).unwrap();
        assert_eq!(state.text, "A😀B");
        assert_eq!(state.caret_index, 3);
        assert_eq!(state.selection, 1..3);
        assert_eq!(state.marked, Some(1..3));
        assert_eq!(
            state.carets.iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 3, 4]
        );
    }

    #[test]
    fn local_clip_withdraws_old_geometry_without_losing_text() {
        let mut host = ContentHost::new(field()).unwrap();
        let frame = Rect::new(0., 0., 300., 80.);
        render(&mut host, frame, frame);
        host.focus(Some(FIELD), true);
        let initial = text_state(&host).unwrap();
        assert!(!initial.carets.is_empty());
        let clip = Rect::new(0., initial.caret_origin.y as f32 + 2., 300., 3.);
        render(&mut host, frame, clip);
        let partial = text_state(&host).unwrap();
        assert!(!partial.carets.is_empty());
        assert_eq!(partial.caret_size.height, 3.);
        assert!(
            partial
                .carets
                .iter()
                .all(|(_, p, s)| p.y >= f64::from(clip.min_y())
                    && p.y + s.height <= f64::from(clip.max_y()))
        );
        render(&mut host, frame, Rect::new(400., 400., 10., 10.));
        let hidden = text_state(&host).unwrap();
        assert_eq!(hidden.text, "A😀B");
        assert_eq!(hidden.caret_size.height, 0.);
        assert!(hidden.carets.is_empty());
    }

    #[test]
    fn disabled_and_inactive_owner_cannot_publish_retained_geometry() {
        let mut host = ContentHost::new(field()).unwrap();
        let frame = Rect::new(0., 0., 300., 80.);
        render(&mut host, frame, frame);
        host.focus(Some(FIELD), true);
        assert!(!text_state(&host).unwrap().carets.is_empty());
        host.get_mut(FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .disabled = true;
        assert!(text_state(&host).is_none());
        host.get_mut(FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .disabled = false;
        host.set_window_active(false);
        assert!(text_state(&host).is_none());
    }

    #[test]
    fn zero_split_pane_revokes_retained_geometry() {
        let split = SplitViewContent::try_new(SplitViewConstruction {
            config: SplitViewConfig::default(),
            slots: SplitViewSlotIds {
                leading: ComponentId(27302),
                trailing: ComponentId(27303),
                divider: ComponentId(27304),
            },
            leading: field(),
            trailing: Content::new(ComponentId(27305), Button::new("Other")),
            budget: 64 * 1024 * 1024,
        })
        .unwrap();
        let mut host = ContentHost::try_new_prepared(
            Content::new(ComponentId(27300), split),
            64 * 1024 * 1024,
        )
        .unwrap();
        let frame = Rect::new(0., 0., 600., 200.);
        render(&mut host, frame, frame);
        host.focus(Some(FIELD), true);
        assert!(!text_state(&host).unwrap().carets.is_empty());
        render(&mut host, Rect::default(), Rect::default());
        let state = text_state(&host).unwrap();
        assert_eq!(state.text, "A😀B");
        assert_eq!(state.caret_size.height, 0.);
        assert!(state.carets.is_empty());
    }

    #[derive(Debug)]
    struct SnapshotOnly;
    impl Component for SnapshotOnly {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            _: Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(Size::new(100., 60.))
        }
        fn focusable(&self) -> bool {
            true
        }
        fn text_input_snapshot(&self) -> Option<ContentTextInput> {
            Some(ContentTextInput {
                binding: vec![1],
                text: "A😀B".into(),
                caret_index: 3,
                selection: 1..3,
                marked: Some(1..3),
                read_only: true,
                caret: Some(Rect::new(10., 5., 1., 20.)),
                clip: Some(Rect::new(0., 10., 100., 10.)),
                carets: vec![(3, Rect::new(10., 5., 1., 20.))],
            })
        }
    }

    #[test]
    fn delegated_snapshot_without_text_surface_reaches_platform() {
        let mut host = ContentHost::new(Content::new(FIELD, SnapshotOnly)).unwrap();
        host.focus(Some(FIELD), true);
        assert!(host.focused_text().is_none());
        let state = text_state(&host).expect("snapshot-only owner must reach native queries");
        assert_eq!(state.text, "A😀B");
        assert_eq!(state.caret_index, 3);
        assert_eq!(state.selection, 1..3);
        assert_eq!(state.marked, Some(1..3));
        assert!(state.read_only);
        assert_eq!(state.caret_origin.y, 10.);
        assert_eq!(state.caret_size.height, 10.);
        assert_eq!(state.carets.len(), 1);
        assert_eq!(state.carets[0].0, 3);
    }
}
