//! Consumer owns fixed assembly; all Host/process/render code is shared.
#[cfg(target_os = "macos")]
mod settings_migration;
#[cfg(target_os = "macos")]
mod verify;
fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    if std::env::args().nth(1).as_deref() == Some("--verify-settings-undo") {
        return tekes_ui_app_host::settings_undo::verify();
    }
    let mut modules = tekes_ui_panel_module_demo::definitions();
    if !cfg!(feature = "three-panels") {
        modules.truncate(if cfg!(feature = "two-panels") { 2 } else { 1 });
    }
    #[cfg(target_os = "macos")]
    if std::env::args().nth(1).as_deref() == Some("--verify") {
        anyhow::ensure!(
            std::env::args().len() == 2,
            "invalid verification arguments"
        );
        return verify::run(&modules.iter().map(|m| m.id.to_owned()).collect::<Vec<_>>());
    }
    #[cfg(target_os = "macos")]
    let mut columns: Vec<tekes_ui_app_host::Column> = modules
        .into_iter()
        .map(|module| tekes_ui_app_host::ProcessContainer::new(module).into())
        .collect();
    #[cfg(target_os = "macos")]
    if cfg!(feature = "three-panels") {
        let entries = (0..3)
            .map(|i| {
                let document_id = ["a", "b", "c"][i];
                tekes_ui_app_host::navigation::Entry {
                    id: format!("文档 {}", ["A", "B", "C"][i]),
                    route: "demo-document".into(),
                    params: serde_json::json!({"document_id":document_id}),
                }
            })
            .collect();
        // Keep the older panel-layout demonstration history untouched.
        let directory = std::path::PathBuf::from(std::env::var("HOME")?)
            .join("Library/Application Support/TekesUI AppHost Prototype/navigation-documents");
        if cfg!(feature = "mixed-columns") {
            use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};
            columns[2] = tekes_ui_app_host::Container::new(
                "local-notes",
                Content::new(
                    ComponentId(1),
                    TextFieldContent::new(
                        "本地备注",
                        "普通 Container：在这里输入文字，验证本地输入。",
                    ),
                ),
            )?
            .into();
        }
        return tekes_ui_app_host::AppWindow::from_columns(columns)?
            .navigation(tekes_ui_app_host::NavigationOptions {
                directory,
                history_id: "three-panel-documents".into(),
                initial_entries: entries,
                view_command: Some(|entry| {
                    match entry.params.get("document_id").and_then(|v| v.as_str()) {
                        Some("a") => Ok(tekes_ui_panel_module_demo::DOCUMENT_A),
                        Some("b") => Ok(tekes_ui_panel_module_demo::DOCUMENT_B),
                        Some("c") => Ok(tekes_ui_panel_module_demo::DOCUMENT_C),
                        _ => anyhow::bail!("unknown demo document"),
                    }
                }),
            })
            .settings(settings()?)
            .run();
    }
    #[cfg(target_os = "macos")]
    {
        tekes_ui_app_host::AppWindow::from_columns(columns)?
            .settings(settings()?)
            .run()
    }
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!("AppHost sample currently requires macOS")
    }
}

#[cfg(target_os = "macos")]
fn settings() -> anyhow::Result<tekes_ui_app_host::SettingsOptions> {
    let path = std::path::PathBuf::from(std::env::var("HOME")?).join(
        "Library/Application Support/TekesUI AppHost Prototype/settings/general-percent/settings.toml",
    );
    settings_migration::migrate(&path)?;
    Ok(
        tekes_ui_app_host::SettingsWindow::from_toml(
            include_str!("../settings.schema.toml"),
            path,
        )?
        .display_mode(if cfg!(feature = "independent-settings") {
            tekes_ui_app_host::SettingsDisplayMode::Independent
        } else {
            tekes_ui_app_host::SettingsDisplayMode::Exclusive
        }),
    )
}
