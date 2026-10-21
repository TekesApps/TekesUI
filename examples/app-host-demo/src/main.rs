//! Consumer owns fixed assembly; all Host/process/render code is shared.
#[cfg(target_os = "macos")]
mod verify;
fn main() -> anyhow::Result<()> {
    let mut modules = tekes_ui_panel_module_demo::definitions();
    if !cfg!(feature = "three-panels") {
        modules.truncate(1);
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
        return tekes_ui_app_host::run_with_navigation(
            tekes_ui_app_host::AppProfile { modules },
            tekes_ui_app_host::NavigationOptions {
                directory,
                history_id: "three-panel-documents".into(),
                initial_entries: entries,
            },
        );
    }
    tekes_ui_app_host::run(tekes_ui_app_host::AppProfile { modules })
}
