//! Independent public API fixtures for navigation composition adapters.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};

pub const ROOT: ComponentId = ComponentId(9_800);
pub const BREADCRUMB: ComponentId = ComponentId(9_801);
pub const TABS: ComponentId = ComponentId(9_802);
pub const FIRST_PANEL: ComponentId = ComponentId(9_820);
pub const COMPOSER: ComponentId = ComponentId(9_822);
pub const SECOND_PANEL: ComponentId = ComponentId(9_830);
pub const COMMAND: ComponentId = ComponentId(9_850);
pub const COMMAND_TRIGGER: ComponentId = ComponentId(9_851);
pub const COMMAND_QUERY: ComponentId = ComponentId(9_852);
pub const BACKGROUND_ACTION: ComponentId = ComponentId(9_853);

pub fn breadcrumb(items: &[&str]) -> Result<ContentHost, CompositionError> {
    let path = Content::new(
        BREADCRUMB,
        BreadcrumbContent::new(tekes_ui_controls::extended::Breadcrumb {
            items: items.iter().map(|item| (*item).to_owned()).collect(),
        }),
    );
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.0,
            },
            children: vec![path],
            flexible_child: None,
        },
    ))?;
    host.set_action_events(true);
    Ok(host)
}

pub fn tabs() -> Result<ContentHost, CompositionError> {
    let label_one = Content::new(
        ComponentId(9_810),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 2.0,
            },
            children: vec![
                Content::new(ComponentId(9_811), TextContent::new("Overview")),
                Content::new(ComponentId(9_812), TextContent::new("Welcome")),
            ],
            flexible_child: None,
        },
    );
    let label_two = Content::new(ComponentId(9_813), TextContent::new("Settings"));
    let first_panel = Content::new(
        FIRST_PANEL,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.0,
            },
            children: vec![Content::new(
                COMPOSER,
                TextFieldContent::new("Workspace name", "Current workspace"),
            )],
            flexible_child: None,
        },
    );
    let second_panel = Content::new(SECOND_PANEL, TextContent::new("Settings panel content"));
    let tabs = Content::new(
        TABS,
        TabsContent::new(
            vec![
                TabItem::new(label_one, first_panel),
                TabItem::new(label_two, second_panel),
            ],
            0,
        ),
    );
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.0,
            },
            children: vec![tabs],
            flexible_child: None,
        },
    ))?;
    host.set_action_events(true);
    Ok(host)
}

pub fn command_palette() -> Result<ContentHost, CompositionError> {
    let model = tekes_ui_controls::extended::CommandPalette {
        combo: tekes_ui_controls::extended::ComboBox {
            items: vec![
                "Open File".into(),
                "Open Folder".into(),
                "Preferences".into(),
            ],
            ..Default::default()
        },
    };
    let command = Content::new(
        COMMAND,
        CommandPaletteContent::new(
            Content::new(COMMAND_TRIGGER, TextContent::new("Commands")),
            Content::new(COMMAND_QUERY, TextFieldContent::new("Filter commands", "")),
            model,
        ),
    );
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.0,
            },
            children: vec![
                command,
                Content::new(BACKGROUND_ACTION, Button::new("Background action")),
            ],
            flexible_child: None,
        },
    ))?;
    host.set_action_events(true);
    Ok(host)
}
