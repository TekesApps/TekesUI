//! Shared native AppHost. Consumers supply fixed native module factories.
#[cfg(target_os = "macos")]
pub mod accessibility;
#[cfg(target_os = "macos")]
pub mod gpu;
#[cfg(target_os = "macos")]
pub mod module_process;
pub mod navigation;
#[cfg(target_os = "macos")]
mod navigation_window;
#[cfg(target_os = "macos")]
mod settings;
pub mod settings_undo;
#[cfg(target_os = "macos")]
mod sidebar_motion;
#[cfg(target_os = "macos")]
pub use settings::{SettingsDisplayMode, SettingsOptions, SettingsWindow};
#[cfg(target_os = "macos")]
mod text;
#[cfg(target_os = "macos")]
mod window;
#[cfg(target_os = "macos")]
mod window_toolbar;
#[cfg(target_os = "macos")]
pub use navigation_window::NavigationOptions;
pub use tekes_ui_module_runtime::ModuleDefinition;

/// Fixed build-time assembly, not a dynamic plugin registry.
pub struct AppProfile {
    pub modules: Vec<ModuleDefinition>,
}

fn validate(profile: &AppProfile) -> anyhow::Result<()> {
    anyhow::ensure!(
        (1..=3).contains(&profile.modules.len()),
        "expected one, two or three fixed modules"
    );
    let mut ids = std::collections::HashSet::new();
    for m in &profile.modules {
        anyhow::ensure!(
            !m.id.is_empty()
                && m.id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid module ID"
        );
        anyhow::ensure!(ids.insert(m.id), "duplicate module ID: {}", m.id);
    }
    Ok(())
}

/// Shared AppWindow assembly used by Main consumers. SettingsWindow uses the
/// same native window shell, splitter, toolbar, sidebar motion and navigation.
#[cfg(target_os = "macos")]
pub struct AppWindow {
    profile: AppProfile,
    navigation: Option<NavigationOptions>,
    settings: Option<SettingsWindow>,
}
#[cfg(target_os = "macos")]
impl AppWindow {
    pub fn new(profile: AppProfile) -> Self {
        Self {
            profile,
            navigation: None,
            settings: None,
        }
    }
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = Some(navigation);
        self
    }
    pub fn settings(mut self, settings: SettingsWindow) -> Self {
        self.settings = Some(settings);
        self
    }
    pub fn run(self) -> anyhow::Result<()> {
        run_internal(self.profile, self.navigation, self.settings)
    }
}

/// Same executable: parent is Host, --worker selects one fixed native factory.
/// Factories are never invoked by the Host process. Any diagnostic
/// entry point belongs to the consuming app.
#[cfg(target_os = "macos")]
pub fn run(profile: AppProfile) -> anyhow::Result<()> {
    run_internal(profile, None, None)
}
#[cfg(target_os = "macos")]
pub fn run_with_navigation(
    profile: AppProfile,
    navigation: NavigationOptions,
) -> anyhow::Result<()> {
    run_internal(profile, Some(navigation), None)
}
#[cfg(target_os = "macos")]
pub fn run_with_settings(
    profile: AppProfile,
    navigation: Option<NavigationOptions>,
    settings: SettingsOptions,
) -> anyhow::Result<()> {
    run_internal(profile, navigation, Some(settings))
}
#[cfg(target_os = "macos")]
fn run_internal(
    profile: AppProfile,
    navigation: Option<NavigationOptions>,
    settings: Option<SettingsOptions>,
) -> anyhow::Result<()> {
    validate(&profile)?;
    let args: Vec<String> = std::env::args().collect();
    if tekes_ui_module_runtime::run_component_worker_from_args(
        &[
            crate::navigation::definition(),
            crate::settings_undo::definition(),
        ],
        &args,
    )? {
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--worker") {
        anyhow::ensure!(args.len() == 4, "invalid worker arguments");
        let definition = profile
            .modules
            .iter()
            .find(|m| m.id == args[2])
            .ok_or_else(|| anyhow::anyhow!("worker module not in fixed profile"))?;
        return tekes_ui_module_runtime::run((definition.create)(), &args[3]);
    }
    let ids = profile
        .modules
        .iter()
        .map(|m| m.id.to_owned())
        .collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        None => window::run(ids, false, navigation, settings),
        Some("--window-acceptance") if args.len() == 2 => {
            window::run(ids, true, navigation, settings)
        }
        _ => anyhow::bail!("fixed-profile app accepts no runtime module discovery or selection"),
    }
}
#[cfg(not(target_os = "macos"))]
pub fn run(profile: AppProfile) -> anyhow::Result<()> {
    validate(&profile)?;
    anyhow::bail!("AppHost prototype currently requires macOS")
}
