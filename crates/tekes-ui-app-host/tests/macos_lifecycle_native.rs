#[cfg(target_os = "macos")]
mod macos {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use tekes_ui_app_host::{
        AppWindow, ContainerContent, LocalContainer, OthersOptions, PresentationOptions,
        SettingsDisplayMode, SettingsWindow,
    };
    use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, TextFieldContent};
    use tekes_ui_platform_macos::selftest::{Action, run_script};

    const SETTINGS_SCHEMA: &str = r#"
schema_version = 1
namespace = "tekes-lifecycle-native"
provider = "lifecycle.settings"

[[pages]]
id = "general"
label = "General"

[[groups]]
id = "workspace"
page = "general"
label = "Workspace"

[[groups.items]]
id = "general"
label = "Identity"

[[groups.items.fields]]
id = "font-size"
path = ["workspace", "font_size"]
label = "Font Size"
type = "integer"
default = 14
min = 10
max = 32
step = 1
"#;

    struct FocusNoteContent {
        host: ContentHost,
        drop_count: Rc<Cell<usize>>,
        final_text: Rc<RefCell<Option<String>>>,
    }

    impl ContainerContent for FocusNoteContent {
        fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
            visit(&mut self.host);
        }
    }

    impl Drop for FocusNoteContent {
        fn drop(&mut self) {
            self.drop_count.set(self.drop_count.get() + 1);
            let value = self
                .host
                .get(ComponentId(1))
                .and_then(|node| node.component::<TextFieldContent>())
                .map(|field| field.text().to_owned());
            *self.final_text.borrow_mut() = value;
        }
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        if tekes_ui_app_host::run_component_worker_from_args(&args)
            .expect("Settings component worker dispatch")
        {
            return;
        }
        let temp_dir = std::env::temp_dir().join(format!(
            "tekes-apphost-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let settings_path = temp_dir.join("settings.toml");
        let settings = SettingsWindow::from_toml(SETTINGS_SCHEMA, settings_path)
            .expect("valid settings schema")
            .display_mode(SettingsDisplayMode::Independent);

        let drop_count = Rc::new(Cell::new(0));
        let final_text = Rc::new(RefCell::new(None));
        let focus_host = ContentHost::new(Content::new(
            ComponentId(1),
            TextFieldContent::new("Focus Note", "initial"),
        ))
        .expect("focus host");
        let focus = LocalContainer::from_source(
            "focus",
            FocusNoteContent {
                host: focus_host,
                drop_count: drop_count.clone(),
                final_text: final_text.clone(),
            },
        )
        .title("Focus");

        let tab_one = LocalContainer::new(
            "tab_one",
            Content::new(ComponentId(1), TextFieldContent::new("Tab One", "alpha")),
        )
        .expect("tab one")
        .title("Tab One");
        let tab_two = LocalContainer::new(
            "tab_two",
            Content::new(ComponentId(1), TextFieldContent::new("Tab Two", "beta")),
        )
        .expect("tab two")
        .title("Tab Two");

        let presentation = PresentationOptions::new("focus")
            .others(OthersOptions::new(vec!["tab_one".into(), "tab_two".into()]));
        let app = AppWindow::from_containers(
            vec![focus.into(), tab_one.into(), tab_two.into()],
            presentation,
        )
        .expect("valid AppWindow")
        .app_name("TekesUI Lifecycle App")
        .window_title("TekesUI Lifecycle Main")
        .settings(settings);

        run_script(vec![
            (
                700,
                Action::ExpectLifecycle {
                    label: "apphost_initial",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Tab"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                900,
                Action::ClickPublished {
                    label: "Focus Note",
                    secondary: false,
                },
            ),
            (1050, Action::Text(" + kept".into())),
            (
                1200,
                Action::KeyEquivalent {
                    chars: "w",
                    key_code: 13,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (
                1400,
                Action::ExpectLifecycle {
                    label: "apphost_after_close_tab_one",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Tab"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (1500, Action::Close),
            (
                1700,
                Action::ExpectLifecycle {
                    label: "apphost_after_red_close",
                    primary_visible: Some(false),
                    primary_dormant: Some(true),
                    visible_windows: Some(0),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(false),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                1800,
                Action::KeyEquivalent {
                    chars: "n",
                    key_code: 45,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (
                2050,
                Action::ExpectLifecycle {
                    label: "apphost_after_cmd_n_tab_preserved",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Tab"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                2150,
                Action::KeyEquivalent {
                    chars: "w",
                    key_code: 13,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (
                2350,
                Action::ExpectLifecycle {
                    label: "apphost_after_close_tab_two",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                2450,
                Action::KeyEquivalent {
                    chars: "w",
                    key_code: 13,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (2650, Action::RedrawMark("apphost_dormant")),
            (
                2950,
                Action::ExpectLifecycle {
                    label: "apphost_while_dormant",
                    primary_visible: Some(false),
                    primary_dormant: Some(true),
                    visible_windows: Some(0),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(false),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: Some("apphost_dormant"),
                },
            ),
            (3020, Action::Reopen),
            (
                3280,
                Action::ExpectLifecycle {
                    label: "apphost_after_dock_reopen",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (3380, Action::MenuCommand(0x534554)),
            (
                4900,
                Action::ExpectLifecycle {
                    label: "apphost_settings_open",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(2),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                5000,
                Action::KeyEquivalent {
                    chars: "w",
                    key_code: 13,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (
                5250,
                Action::ExpectLifecycle {
                    label: "apphost_after_settings_cmd_w",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (5350, Action::MenuCommand(0x534554)),
            (
                6850,
                Action::ClickPublished {
                    label: "Font Size",
                    secondary: false,
                },
            ),
            (7000, Action::TextCommand("selectAll:")),
            (7100, Action::Preedit("99".into())),
            (
                7250,
                Action::ExpectText {
                    text: "99".into(),
                    marked: true,
                },
            ),
            (7350, Action::Quit),
            (
                7600,
                Action::ExpectLifecycle {
                    label: "apphost_quit_blocked_by_settings",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(2),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (7700, Action::Text("14".into())),
            (7850, Action::TextCommand("insertNewline:")),
            (
                8000,
                Action::KeyEquivalent {
                    chars: "w",
                    key_code: 13,
                    control: false,
                    alt: false,
                    shift: false,
                    command: true,
                },
            ),
            (
                8250,
                Action::ExpectLifecycle {
                    label: "apphost_after_clean_settings_close",
                    primary_visible: Some(true),
                    primary_dormant: Some(false),
                    visible_windows: Some(1),
                    close_menu_title: Some("Close Window"),
                    close_menu_enabled: Some(true),
                    new_window_enabled: Some(true),
                    redraws_delta_zero_since: None,
                },
            ),
            (
                8350,
                Action::ClickPublished {
                    label: "Focus Note",
                    secondary: false,
                },
            ),
            (8500, Action::Text(" + after_quit_blocked".into())),
            (8700, Action::Quit),
        ]);

        app.run().expect("AppWindow::run finishes cleanly on Quit");

        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        assert_eq!(
            drop_count.get(),
            1,
            "LocalContainer must drop once when quitting"
        );
        let retained = final_text
            .borrow()
            .clone()
            .expect("Focus Note text snapshot on drop");
        assert!(
            retained.contains("kept") && retained.contains("after_quit_blocked"),
            "Focus Note edit must survive dormant/reopen cycles and blocked Quit guard: {retained:?}"
        );
        println!("APPHOST_LIFECYCLE_NATIVE_OK retained_text={retained:?}");
    }
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}
