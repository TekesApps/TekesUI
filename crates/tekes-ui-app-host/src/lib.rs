//! Shared native AppHost. Consumers supply fixed native module factories.
#[cfg(target_os = "macos")]
pub mod gpu;
#[cfg(target_os = "macos")]
pub mod module_process;
pub mod navigation;
#[cfg(target_os = "macos")]
mod settings;
pub mod settings_undo;
#[cfg(target_os = "macos")]
pub use settings::{SettingsDisplayMode, SettingsOptions, SettingsOwner, SettingsWindow};
#[cfg(target_os = "macos")]
mod columns;
#[cfg(target_os = "macos")]
mod container_factory;
#[cfg(target_os = "macos")]
pub use container_factory::{ContainerFactory, ContainerMultiplicity};
#[cfg(target_os = "macos")]
mod navigation_window;
#[cfg(target_os = "macos")]
mod presentation;
#[cfg(target_os = "macos")]
mod sidebar_motion;
#[cfg(target_os = "macos")]
mod text;
#[cfg(target_os = "macos")]
mod toolbar_source;
#[cfg(target_os = "macos")]
mod window;
#[cfg(target_os = "macos")]
mod window_toolbar;
#[cfg(target_os = "macos")]
pub use columns::{
    Column, Container, ContainerContent, ContainerOptions, ContainerPresentation,
    ContainerToolbarHost, ContainerToolbarPosition, LocalContainer, ProcessContainer,
    RetainedButton,
};
#[cfg(target_os = "macos")]
pub use presentation::{
    OthersGroup, OthersOptions, OthersTab, PresentationLayout, PresentationOptions,
    PresentationState,
};
/// Source migration helpers for the former local-only Container name.
#[cfg(target_os = "macos")]
pub mod legacy {
    pub use crate::columns::{Column, LocalContainer as Container};
}
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
    theme: tekes_ui_tokens::Theme,
    profile: AppProfile,
    columns: Option<Vec<Column>>,
    navigation: Option<NavigationOptions>,
    settings: Option<settings::SettingsConfiguration>,
    presentation: Option<PresentationOptions>,
    container_factories: Vec<ContainerFactory>,
}
#[cfg(target_os = "macos")]
impl AppWindow {
    pub fn new(profile: AppProfile) -> Self {
        Self {
            theme: tekes_ui_tokens::Theme::LIGHT,
            profile,
            columns: None,
            navigation: None,
            settings: None,
            presentation: None,
            container_factories: Vec::new(),
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
            theme: tekes_ui_tokens::Theme::LIGHT,
            profile: AppProfile { modules },
            columns: Some(columns),
            navigation: None,
            settings: None,
            presentation: None,
            container_factories: Vec::new(),
        })
    }
    /// Assemble the optional NavBar and focus/others presentation from stable
    /// container IDs. All backing objects are moved into the original Host.
    pub fn from_containers(
        containers: Vec<Container>,
        presentation: PresentationOptions,
    ) -> anyhow::Result<Self> {
        PresentationState::new(&containers, presentation.clone())?;
        let columns: Vec<Column> = containers.into_iter().map(Column::from).collect();
        let modules = columns
            .iter()
            .filter_map(|c| match c {
                Column::ProcessContainer(p) => Some(p.definition),
                _ => None,
            })
            .collect();
        Ok(Self {
            theme: tekes_ui_tokens::Theme::LIGHT,
            profile: AppProfile { modules },
            columns: Some(columns),
            navigation: None,
            settings: None,
            presentation: Some(presentation),
            container_factories: Vec::new(),
        })
    }
    pub(crate) fn into_host(self, acceptance: bool) -> window::Host {
        let theme = self.theme;
        let host = match self.columns {
            Some(columns) if self.presentation.is_some() => window::Host::from_containers(
                columns.into_iter().map(Container::from).collect(),
                acceptance,
                self.navigation,
                self.presentation,
            ),
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
        };
        host.with_theme(theme)
            .with_container_factories(self.container_factories)
    }
    /// Register finite, consumer-defined creatable types before opening a window.
    /// Process definitions use the executable's existing fixed worker dispatch.
    pub fn container_factories(mut self, factories: Vec<ContainerFactory>) -> anyhow::Result<Self> {
        container_factory::validate(&factories)?;
        anyhow::ensure!(
            self.presentation
                .as_ref()
                .is_some_and(|p| p.others.is_some()),
            "container factories require Others"
        );
        let others = self.presentation.as_ref().unwrap().others.as_ref().unwrap();
        anyhow::ensure!(
            !factories
                .iter()
                .any(|factory| others.groups.iter().any(|group| group.id == factory.id)),
            "factory ID conflicts with an existing Others group"
        );
        for definition in factories.iter().filter_map(ContainerFactory::definition) {
            anyhow::ensure!(
                !self.profile.modules.iter().any(|m| m.id == definition.id),
                "worker factory registered more than once: {}",
                definition.id
            );
            self.profile.modules.push(definition);
        }
        self.container_factories = factories;
        Ok(self)
    }
    /// Initial appearance and contrast for this window and its local content.
    /// Defaults to LIGHT; does not follow an embedding window's later Theme.
    pub fn theme(mut self, theme: tekes_ui_tokens::Theme) -> Self {
        self.theme = theme;
        self
    }
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = Some(navigation);
        self
    }

    pub fn settings(mut self, settings: SettingsWindow) -> Self {
        self.settings = Some(settings::SettingsConfiguration::Window(settings));
        self
    }
    /// Reuse one explicitly owned Settings window and persistence service across
    /// AppWindows. Closing an opener does not close Independent Settings.
    pub fn shared_settings(mut self, owner: SettingsOwner) -> Self {
        self.settings = Some(settings::SettingsConfiguration::Shared(owner));
        self
    }
    /// Open a native window in an existing application event loop.
    /// Process containers require the embedding executable's usual worker dispatch.
    pub fn open(
        mut self,
        parent: &tekes_ui_platform_macos::Window,
        config: tekes_ui_platform_macos::WindowConfig,
    ) -> anyhow::Result<tekes_ui_platform_macos::WindowHandle> {
        validate_assembly(
            &self.profile,
            self.columns.as_deref(),
            self.navigation.is_some(),
            self.presentation.as_ref(),
        )?;
        let settings = self.settings.take();
        self.into_host(false).open(parent, config, settings)
    }

    pub fn run(self) -> anyhow::Result<()> {
        run_columns(
            self.profile,
            self.columns,
            self.navigation,
            self.settings,
            self.presentation,
            self.theme,
            self.container_factories,
        )
    }
}

/// Dispatch the fixed headless AppHost services before an embedding app creates
/// any windows. Returns true only when this process was a component worker.
#[cfg(target_os = "macos")]
pub fn run_component_worker_from_args(args: &[String]) -> anyhow::Result<bool> {
    tekes_ui_module_runtime::run_component_worker_from_args(
        &[navigation::definition(), settings_undo::definition()],
        args,
    )
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
    run_internal(
        profile,
        navigation,
        Some(settings::SettingsConfiguration::Window(settings)),
    )
}
#[cfg(target_os = "macos")]
fn run_internal(
    profile: AppProfile,
    navigation: Option<NavigationOptions>,
    settings: Option<settings::SettingsConfiguration>,
) -> anyhow::Result<()> {
    run_columns(
        profile,
        None,
        navigation,
        settings,
        None,
        tekes_ui_tokens::Theme::LIGHT,
        Vec::new(),
    )
}
#[cfg(target_os = "macos")]
fn validate_assembly(
    profile: &AppProfile,
    columns: Option<&[Column]>,
    navigation: bool,
    presentation: Option<&PresentationOptions>,
) -> anyhow::Result<()> {
    if let Some(columns) = columns {
        if let Some(presentation) = presentation {
            PresentationState::from_columns(columns, presentation.clone())?;
        } else {
            columns::validate(columns)?;
        }
    } else {
        validate(profile)?;
    }
    if navigation && let Some(presentation) = presentation {
        anyhow::ensure!(
            presentation.navigation_toolbar.is_some(),
            "headless navigation requires an explicit container Toolbar target"
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_columns(
    profile: AppProfile,
    columns: Option<Vec<Column>>,
    navigation: Option<NavigationOptions>,
    settings: Option<settings::SettingsConfiguration>,
    presentation: Option<PresentationOptions>,
    theme: tekes_ui_tokens::Theme,
    factories: Vec<ContainerFactory>,
) -> anyhow::Result<()> {
    validate_assembly(
        &profile,
        columns.as_deref(),
        navigation.is_some(),
        presentation.as_ref(),
    )?;
    let args: Vec<String> = std::env::args().collect();
    if run_component_worker_from_args(&args)? {
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
    let acceptance = match args.get(1).map(String::as_str) {
        None => false,
        Some("--window-acceptance") if args.len() == 2 => true,
        _ => anyhow::bail!("fixed-profile app accepts no runtime module discovery or selection"),
    };
    window::run(
        AppWindow {
            profile,
            columns,
            navigation,
            settings,
            presentation,
            theme,
            container_factories: factories,
        },
        acceptance,
    )
}
#[cfg(not(target_os = "macos"))]
pub fn run(profile: AppProfile) -> anyhow::Result<()> {
    validate(&profile)?;
    anyhow::bail!("AppHost prototype currently requires macOS")
}

#[cfg(all(test, target_os = "macos"))]
mod hosted_window_tests {
    use super::*;
    use tekes_ui_controls::composition::{ComponentId, Content, TextFieldContent};
    #[test]
    fn hosted_open_uses_run_assembly_preconditions_before_native_creation() {
        let empty = AppProfile { modules: vec![] };
        assert!(validate_assembly(&empty, None, false, None).is_err());
        let local = LocalContainer::new(
            "focus",
            Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
        )
        .unwrap();
        let app = AppWindow::from_containers(vec![local.into()], PresentationOptions::new("focus"))
            .unwrap();
        assert!(
            validate_assembly(
                &app.profile,
                app.columns.as_deref(),
                true,
                app.presentation.as_ref()
            )
            .unwrap_err()
            .to_string()
            .contains("explicit container Toolbar")
        );
        assert!(
            validate_assembly(
                &app.profile,
                app.columns.as_deref(),
                false,
                app.presentation.as_ref()
            )
            .is_ok()
        );
        let presentation = app.presentation.unwrap().navigation_toolbar("focus");
        assert!(
            validate_assembly(
                &app.profile,
                app.columns.as_deref(),
                true,
                Some(&presentation)
            )
            .is_ok()
        );
    }
}
