//! Standalone native settings #207 selftest harness moved out of Gallery's production binary.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use tekes_ui_app_host::{AppWindow, ContainerContent, LocalContainer};
    use tekes_ui_controls::composition::{
        ComponentId, Content, ContentHost, ListContent, TextContent, TextFieldContent,
    };
    use tekes_ui_platform_macos::selftest::{self, Action};
    use tekes_ui_platform_macos::{App, AppConfig, Modifiers, Size, WindowConfig, WindowPlacement};

    struct SettingsHost(ContentHost);
    impl ContainerContent for SettingsHost {
        fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
            visit(&mut self.0);
        }
    }

    let content = Content::new(
        ComponentId(10),
        ListContent::scrollable(
            "Settings #207",
            ComponentId(11),
            vec![
                Content::new(ComponentId(12), TextContent::new("Workspace Settings")),
                Content::new(
                    ComponentId(13),
                    TextFieldContent::new("Workspace title", "Studio"),
                ),
            ],
        ),
    );
    let host = ContentHost::new(content).map_err(|e| anyhow::anyhow!("settings host: {e:?}"))?;
    let app = AppWindow::from_columns(vec![
        LocalContainer::from_source("settings-207", SettingsHost(host))
            .title("Settings #207")
            .into(),
    ])?;
    let (menu_bar, handler) = app.into_window_handler()?;

    let out = std::env::var("TEKES_SELFTEST_OUT").unwrap_or_else(|_| ".".into());
    let mut t = 1400_u64;
    let mut steps = Vec::new();
    let mut add = |action: Action| {
        steps.push((t, action));
        t += 180;
    };
    add(Action::Resize { w: 1440., h: 960. });
    add(Action::Capture {
        path: format!("{out}/settings-207-desktop.png"),
    });
    add(Action::Resize { w: 720., h: 512. });
    add(Action::Log("207-selector-forward".into()));
    add(Action::TextCommand("insertTab:"));
    add(Action::Log("207-local-forward".into()));
    add(Action::KeyCharacters {
        key_code: 48,
        chars: "\t",
    });
    add(Action::KeyCharactersModified {
        key_code: 48,
        chars: "\u{19}",
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    });
    add(Action::Capture {
        path: format!("{out}/settings-207-mobile.png"),
    });
    add(Action::Close);

    selftest::run_script(steps);
    App::new(AppConfig {
        name: Some("TekesUI Settings 207".into()),
        quit_after_last_window_closed: true,
    })
    .run(
        WindowConfig {
            title: "TekesUI Settings 207".into(),
            unified_titlebar: true,
            size: Size {
                width: 1200.,
                height: 800.,
            },
            min_size: Some(Size {
                width: 720.,
                height: 480.,
            }),
            placement: WindowPlacement::Auto {
                need_120hz: false,
                size_check: false,
            },
            frame_autosave_name: None,
        },
        menu_bar,
        handler,
    );
    Ok(())
}
