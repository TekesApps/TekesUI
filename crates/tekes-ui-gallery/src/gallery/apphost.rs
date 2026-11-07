//! A small local-only consumer of the public AppHost window API.
use super::{Page, workbench};
use tekes_ui_app_host::{AppWindow, LocalContainer, OthersOptions, PresentationOptions};
use tekes_ui_app_protocol::ToolbarItem;
use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};
use tekes_ui_platform_macos::{Size as WindowSize, Window, WindowConfig, WindowHandle};

pub fn is_page(page: Page) -> bool {
    matches!(page, Page::Workbench(index) if workbench::entries()[index].slug() == "apphost")
}
fn container(id: &str, title: &str) -> anyhow::Result<LocalContainer> {
    let owner = id.to_owned();
    Ok(LocalContainer::new(
        id,
        Content::new(
            ComponentId(1),
            TextFieldContent::new(
                title,
                "Type here; presentation changes retain this content.",
            ),
        ),
    )?
    .title(title)
    .list_height(240.)
    .toolbar(vec![ToolbarItem {
        id: 1,
        title: "Owner action".into(),
        enabled: true,
        geometry_dependent: false,
    }])
    .on_command(move |_, command| {
        eprintln!("GALLERY_APPHOST_COMMAND owner={owner} command={command}");
        Ok(())
    }))
}
pub fn open(parent: &Window) -> anyhow::Result<WindowHandle> {
    let app = AppWindow::from_containers(
        vec![
            container("gallery-nav", "NavBar")?.into(),
            container("gallery-focus", "Focus")?.into(),
            container("gallery-detail", "Details")?
                .retain_button(1, "gallery-focus")
                .into(),
            container("gallery-list", "List item")?.into(),
        ],
        PresentationOptions::new("gallery-focus")
            .nav_bar("gallery-nav")
            .others(OthersOptions::new(vec![
                "gallery-detail".into(),
                "gallery-list".into(),
            ])),
    )?;
    app.open(
        parent,
        WindowConfig {
            title: "TekesUI AppHost".into(),
            unified_titlebar: true,
            size: WindowSize {
                width: 1100.,
                height: 700.,
            },
            min_size: Some(WindowSize {
                width: 600.,
                height: 320.,
            }),
            ..Default::default()
        },
    )
}
