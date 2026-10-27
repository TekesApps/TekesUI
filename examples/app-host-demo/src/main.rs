//! Consumer owns fixed assembly; all Host/process/render code is shared.
#[cfg(target_os = "macos")]
mod verify;
#[cfg(target_os = "macos")]
mod settings_migration;
fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    if std::env::args().nth(1).as_deref() == Some("--verify-settings-undo") {
        return tekes_ui_app_host::settings_undo::verify();
    }
    let mut modules = tekes_ui_panel_module_demo::definitions();
    if !cfg!(feature = "three-panels") {
        modules.truncate(1);
    }
    #[cfg(target_os = "macos")]
    if matches!(std::env::args().nth(1).as_deref(), Some("--tekesui-component" | "--worker")) {
        // The original fixed registry validates worker argv before constructing
        // any consumer Settings/Provider or performing HOME migration.
        return tekes_ui_app_host::run(tekes_ui_app_host::AppProfile { modules });
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
        return tekes_ui_app_host::run_with_settings(
            tekes_ui_app_host::AppProfile { modules },
            Some(tekes_ui_app_host::NavigationOptions {
                directory,
                history_id: "three-panel-documents".into(),
                initial_entries: entries,
            }),
            settings()?,
        );
    }
    #[cfg(target_os = "macos")]
    { tekes_ui_app_host::run_with_settings(tekes_ui_app_host::AppProfile { modules }, None, settings()?) }
    #[cfg(not(target_os = "macos"))]
    { tekes_ui_app_host::run(tekes_ui_app_host::AppProfile { modules }) }
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
