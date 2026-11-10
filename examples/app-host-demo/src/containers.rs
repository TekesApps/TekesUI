//! Consumer-only assembly for #229. Runtime, Toolbar, menus and transport are
//! provided by tekes-ui-app-host; these callbacks own only demo business state.
use anyhow::{Result, bail};
use std::{cell::Cell, rc::Rc};
use tekes_ui_app_host::{
    AppWindow, Container, LocalContainer, NavigationOptions, OthersOptions, PresentationOptions,
    PresentationState, ProcessContainer,
};
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};

const RETAIN_FIRST: u64 = 101;
const RETAIN_SECOND: u64 = 102;

#[derive(Clone, Copy, Debug)]
enum Layout {
    NavOthers,
    Others,
    Nav,
    Focus,
}
impl Layout {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "nav-others" => Ok(Self::NavOthers),
            "others" => Ok(Self::Others),
            "nav" => Ok(Self::Nav),
            "focus" => Ok(Self::Focus),
            _ => bail!("TEKESUI_APPHOST_LAYOUT must be nav-others, others, nav or focus"),
        }
    }
    fn nav(self) -> bool {
        matches!(self, Self::NavOthers | Self::Nav)
    }
    fn others(self) -> bool {
        matches!(self, Self::NavOthers | Self::Others)
    }
}
#[derive(Clone, Copy, Debug)]
enum Backing {
    Mixed,
    Local,
    Process,
}
impl Backing {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "mixed" => Ok(Self::Mixed),
            "local" => Ok(Self::Local),
            "process" => Ok(Self::Process),
            _ => bail!("TEKESUI_APPHOST_BACKING must be mixed, local or process"),
        }
    }
}

fn toolbar_item(id: u64, title: &str) -> ToolbarItem {
    ToolbarItem {
        id,
        title: title.into(),
        enabled: true,
        geometry_dependent: false,
    }
}
fn local(id: &str, toolbar: bool) -> Result<LocalContainer> {
    let owner = id.to_owned();
    let actions_owner = owner.clone();
    let count = Rc::new(Cell::new(0_u64));
    let command_count = count.clone();
    let mut container = LocalContainer::new(
        id,
        Content::new(
            ComponentId(1),
            TextFieldContent::new(
                format!("Input {id}"),
                &format!("{id}: type here; moving this container keeps this text."),
            ),
        ),
    )?
    .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top })
    .on_command(move |host, command| {
        command_count.set(command_count.get() + 1);
        let text = host
            .get(ComponentId(1))
            .and_then(|content| content.component::<TextFieldContent>())
            .map(|field| field.surface.input.text.clone());
        eprintln!(
            "DEMO_COMMAND owner={owner} command={command} count={} text={text:?}",
            command_count.get()
        );
        Ok(())
    })
    .on_action(move |host, records| {
        for record in records {
            let text = host
                .get(ComponentId(1))
                .and_then(|content| content.component::<TextFieldContent>())
                .map(|field| field.surface.input.text.clone());
            eprintln!(
                "DEMO_ACTION owner={actions_owner} action={:?} payload={:?} text={text:?}",
                record.action, record.payload
            );
        }
    });
    if toolbar {
        container = container.toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title(format!("{id} · own Toolbar")).toolbar(vec![
            toolbar_item(RETAIN_FIRST, "Owner command A"),
            toolbar_item(RETAIN_SECOND, "Owner command B"),
            toolbar_item(103, "Owner only"),
        ]);
    }
    Ok(container)
}
fn assembly(layout: Layout, backing: Backing) -> Result<(Vec<Container>, PresentationOptions)> {
    let definitions = tekes_ui_panel_module_demo::definitions();
    if matches!(backing, Backing::Process) {
        let focus = definitions[0].id;
        let nav = definitions[2].id;
        let mut containers = vec![
            ProcessContainer::new(definitions[0])
                .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title("Process focus")
                .list_height(260.)
                .into(),
        ];
        let mut options = PresentationOptions::new(focus);
        if layout.nav() {
            containers.push(
                ProcessContainer::new(definitions[2])
                    .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title("Process NavBar")
                    .into(),
            );
            options = options.nav_bar(nav);
        }
        if layout.others() {
            let mut ids = vec![definitions[1].id.to_owned()];
            containers.push(
                ProcessContainer::new(definitions[1])
                    .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title("Process second")
                    .list_height(260.)
                    .retain_button(1, focus)
                    .into(),
            );
            if !layout.nav() {
                containers.push(
                    ProcessContainer::new(definitions[2])
                        .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title("Process third")
                        .list_height(260.)
                        .into(),
                );
                ids.push(definitions[2].id.to_owned());
            }
            options = options
                .others(OthersOptions::new(ids))
                .navigation_targets(vec![definitions[1].id.into()]);
        } else {
            options = options.navigation_targets(vec![focus.into()]);
        }
        options = options.navigation_toolbar(focus);
        return Ok((containers, options));
    }
    let remote = if matches!(backing, Backing::Mixed) {
        definitions[0].id
    } else {
        "preview"
    };
    let mut containers: Vec<Container> = vec![local("notes", true)?.list_height(260.).into()];
    let mut options = PresentationOptions::new("notes").navigation_toolbar("notes");
    if layout.nav() {
        containers.push(local("navbar", false)?.into());
        options = options.nav_bar("navbar");
    }
    if layout.others() {
        containers.push(
            local("tools", true)?
                .list_height(230.)
                .retain_button(RETAIN_FIRST, "notes")
                .retain_button(RETAIN_SECOND, remote)
                .into(),
        );
        // No title or Toolbar: this allocation must start directly at Content.
        containers.push(local("plain", false)?.into());
        containers.push(if matches!(backing, Backing::Mixed) {
            ProcessContainer::new(definitions[0])
                .toolbar_host(tekes_ui_app_host::ContainerToolbarHost::Container { position: tekes_ui_app_host::ContainerToolbarPosition::Top }).title("Remote · own Toolbar")
                .list_height(280.)
                .into()
        } else {
            local("preview", true)?.list_height(280.).into()
        });
        // The history service explicitly addresses one visible and one hidden
        // target; changing Tabs/focus must not silently retarget its commands.
        options = options
            .others(OthersOptions::new(vec![
                "tools".into(),
                "plain".into(),
                remote.into(),
            ]))
            .navigation_targets(vec!["notes".into(), remote.into()]);
    } else {
        options = options.navigation_targets(vec!["notes".into()]);
    }
    if matches!(layout, Layout::NavOthers)
        && matches!(backing, Backing::Mixed)
        && std::env::var("TEKESUI_PRESENTATION_WINDOW_ACCEPTANCE").as_deref() == Ok("1")
        && std::env::args().nth(1).as_deref() == Some("--window-acceptance")
    {
        // Keep the navigation controls visible while BOTH original business
        // recipients are hidden. Normal demo assembly retains its notes target.
        options = options.navigation_toolbar("navbar");
        eprintln!(
            "DEMO_ACCEPTANCE_FIXTURE navigation_toolbar=navbar navigation_targets=notes,demo-first normal_demo_unchanged=true"
        );
    }
    Ok((containers, options))
}
fn navigation() -> Result<NavigationOptions> {
    Ok(NavigationOptions {
        directory: super::data_root()?.join("container-navigation"),
        history_id: "container-documents".into(),
        initial_entries: (0..3)
            .map(|index| tekes_ui_app_host::navigation::Entry {
                id: format!("Container document {}", index + 1),
                route: "demo-document".into(),
                params: serde_json::json!({"document": index}),
            })
            .collect(),
        view_command: Some(|entry| {
            match entry
                .params
                .get("document")
                .and_then(serde_json::Value::as_u64)
            {
                Some(0) => Ok(tekes_ui_panel_module_demo::DOCUMENT_A),
                Some(1) => Ok(tekes_ui_panel_module_demo::DOCUMENT_B),
                Some(2) => Ok(tekes_ui_panel_module_demo::DOCUMENT_C),
                _ => bail!("unknown container document"),
            }
        }),
    })
}
pub fn run(verify_only: bool) -> Result<()> {
    let layout = Layout::parse(
        &std::env::var("TEKESUI_APPHOST_LAYOUT").unwrap_or_else(|_| "nav-others".into()),
    )?;
    let backing = Backing::parse(
        &std::env::var("TEKESUI_APPHOST_BACKING").unwrap_or_else(|_| "mixed".into()),
    )?;
    let (containers, options) = assembly(layout, backing)?;
    let state = PresentationState::new(&containers, options.clone())?;
    eprintln!(
        "DEMO_ASSEMBLY layout={layout:?} backing={backing:?} ids={:?} focus={} nav={:?} others={:?} navigation_targets={:?}",
        containers.iter().map(Container::id).collect::<Vec<_>>(),
        state.focus_id(),
        state.nav_bar_id(),
        state.others(),
        state.navigation_targets()
    );
    if verify_only {
        eprintln!(
            "DEMO_ASSEMBLY_VERIFIED frames={:?}",
            state.frames(tekes_ui_scene::Rect::new(0., 0., 1000., 720.))
        );
        return Ok(());
    }
    AppWindow::from_containers(containers, options)?
        .navigation(navigation()?)
        .settings(super::settings()?)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_consumer_assembles_each_layout_and_backing_without_starting_workers() {
        for layout in [
            Layout::NavOthers,
            Layout::Others,
            Layout::Nav,
            Layout::Focus,
        ] {
            for backing in [Backing::Mixed, Backing::Local, Backing::Process] {
                let (containers, options) = assembly(layout, backing).unwrap();
                let state = PresentationState::new(&containers, options.clone()).unwrap();
                assert_eq!(state.nav_bar_id().is_some(), layout.nav());
                assert_eq!(state.others().is_some(), layout.others());
                AppWindow::from_containers(containers, options).unwrap();
            }
        }
    }
}
