//! Public column assembly. Local views retain original ContentHost ownership;
//! process columns retain the compiled module registry and isolated transport.
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
pub struct Container {
    id: String,
    source: Box<dyn ContainerContent>,
    action: Option<ActionHandler>,
    command: Option<CommandHandler>,
    pub(crate) toolbar: Vec<tekes_ui_app_protocol::ToolbarItem>,
    pub(crate) frame: Rect,
}
impl Container {
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
        }
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
                h.event(&ContentEvent::Cancel);
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
/// The factory remains compiled into the consumer; no runtime module discovery.
pub struct ProcessContainer {
    pub(crate) definition: crate::ModuleDefinition,
}
impl ProcessContainer {
    pub fn new(definition: crate::ModuleDefinition) -> Self {
        Self { definition }
    }
}
pub enum Column {
    Container(Container),
    ProcessContainer(ProcessContainer),
}
impl Column {
    pub fn id(&self) -> &str {
        match self {
            Self::Container(c) => c.id(),
            Self::ProcessContainer(p) => p.definition.id,
        }
    }
}
impl From<Container> for Column {
    fn from(value: Container) -> Self {
        Self::Container(value)
    }
}
impl From<ProcessContainer> for Column {
    fn from(value: ProcessContainer) -> Self {
        Self::ProcessContainer(value)
    }
}
pub(crate) fn validate(columns: &[Column]) -> Result<()> {
    anyhow::ensure!(
        (1..=3).contains(&columns.len()),
        "AppWindow requires one to three columns"
    );
    let mut ids = std::collections::HashSet::new();
    for c in columns {
        anyhow::ensure!(
            !c.id().is_empty()
                && c.id()
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid column ID"
        );
        anyhow::ensure!(ids.insert(c.id()), "duplicate column ID: {}", c.id());
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
    fn local(id: &str) -> Container {
        Container::new(
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
                crate::accessibility::content_nodes(h, true)
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
