//! Shared native AppHost. Consumers supply fixed native module factories.
#[cfg(target_os = "macos")]
pub mod gpu;
#[cfg(target_os = "macos")]
pub mod module_process;
pub mod navigation;
#[cfg(target_os = "macos")]
mod navigation_window;
#[cfg(target_os = "macos")]
mod sidebar_motion;
#[cfg(target_os = "macos")]
mod text;
#[cfg(target_os = "macos")]
mod window;
#[cfg(target_os = "macos")]
mod window_toolbar;
#[cfg(target_os = "macos")]
mod toolbar_source;
#[cfg(target_os = "macos")]
mod columns;
#[cfg(target_os = "macos")]
pub use columns::{Column, Container, ContainerContent, ProcessContainer};
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

/// Fixed local/process column assembly over the shared native window shell.
#[cfg(target_os = "macos")]
pub struct AppWindow {
    profile: AppProfile,
    columns: Option<Vec<Column>>,
    navigation: Option<NavigationOptions>,
}
#[cfg(target_os = "macos")]
impl AppWindow {
    pub fn new(profile: AppProfile) -> Self {
        Self {
            profile,
            columns: None,
            navigation: None,
        }
    }
    pub fn from_columns(columns: Vec<Column>) -> anyhow::Result<Self> {
        columns::validate(&columns)?;
        let modules = columns
            .iter()
            .filter_map(|c| match c {
                Column::ProcessContainer(p) => Some(p.definition),
                _ => None,
            })
            .collect();
        Ok(Self {
            profile: AppProfile { modules },
            columns: Some(columns),
            navigation: None,
        })
    }
    #[cfg(test)]
    pub(crate) fn into_host(self, acceptance: bool) -> window::Host {
        match self.columns {
            Some(columns) => window::Host::from_columns(columns, acceptance, self.navigation),
            None => window::Host::new(
                self.profile
                    .modules
                    .iter()
                    .map(|m| m.id.to_owned())
                    .collect(),
                acceptance,
                self.navigation,
            ),
        }
    }
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = Some(navigation);
        self
    }

    pub fn run(self) -> anyhow::Result<()> {
        run_columns(self.profile, self.columns, self.navigation)
    }
}

/// Same executable: parent is Host, --worker selects one fixed native factory.
/// Factories are never invoked by the Host process. Any diagnostic
/// entry point belongs to the consuming app.
#[cfg(target_os = "macos")]
pub fn run(profile: AppProfile) -> anyhow::Result<()> {
    run_internal(profile, None)
}
#[cfg(target_os = "macos")]
pub fn run_with_navigation(
    profile: AppProfile,
    navigation: NavigationOptions,
) -> anyhow::Result<()> {
    run_internal(profile, Some(navigation))
}
#[cfg(target_os = "macos")]
fn run_internal(profile: AppProfile, navigation: Option<NavigationOptions>) -> anyhow::Result<()> {
    run_columns(profile, None, navigation)
}
#[cfg(target_os = "macos")]
fn run_columns(profile: AppProfile, columns: Option<Vec<Column>>, navigation: Option<NavigationOptions>) -> anyhow::Result<()> {
    if let Some(columns) = &columns { columns::validate(columns)?; } else { validate(&profile)?; }
    let args: Vec<String> = std::env::args().collect();
    if tekes_ui_module_runtime::run_component_worker_from_args(
        &[crate::navigation::definition()],
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
        None => window::run(ids, columns, false, navigation),
        Some("--window-acceptance") if args.len() == 2 => window::run(ids, columns, true, navigation),
        _ => anyhow::bail!("fixed-profile app accepts no runtime module discovery or selection"),
    }
}
#[cfg(not(target_os = "macos"))]
pub fn run(profile: AppProfile) -> anyhow::Result<()> {
    validate(&profile)?;
    anyhow::bail!("AppHost prototype currently requires macOS")
}
