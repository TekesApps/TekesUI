//! Shared native AppHost. Consumers supply fixed native module factories.
#[macro_use]
mod diagnostics;
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
mod local_toolbar;
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
    ContainerRenderContext, ContainerToolbarHost, ContainerToolbarPosition, LocalContainer,
    ProcessContainer, RetainedButton,
};
#[cfg(target_os = "macos")]
pub use local_toolbar::{
    LocalToolbarAction, LocalToolbarButton, LocalToolbarGroup, LocalToolbarGroupOverflow,
    LocalToolbarItem, LocalToolbarMenu, LocalToolbarTone,
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
#[cfg(target_os = "macos")]
pub use tekes_ui_icons::Glyph;
pub use tekes_ui_module_runtime::{ComponentDefinition, ModuleDefinition, ProcessComponentOptions};

/// Fixed build-time assembly, not a dynamic plugin registry.
#[derive(Debug)]
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

fn validate_components(
    profile: &AppProfile,
    components: &[ComponentDefinition],
) -> anyhow::Result<()> {
    let mut ids = std::collections::HashSet::new();
    for component in components {
        anyhow::ensure!(
            !component.id.is_empty()
                && component
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid component ID"
        );
        anyhow::ensure!(
            component.capabilities.view,
            "V2 process component requires view capability: {}",
            component.id
        );
        anyhow::ensure!(
            !profile.modules.iter().any(|m| m.id == component.id),
            "component ID conflicts with V1 module ID: {}",
            component.id
        );
        ids.insert(component.id);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub struct HostedWindow(window::Host);

#[cfg(target_os = "macos")]
impl std::fmt::Debug for HostedWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostedWindow").finish_non_exhaustive()
    }
}

#[cfg(target_os = "macos")]
impl tekes_ui_platform_macos::WindowHandler for HostedWindow {
    fn allows_app_quit(&self) -> bool {
        self.0.allows_app_quit()
    }

    fn close_menu_title(&self) -> Option<&'static str> {
        self.0.close_menu_title()
    }

    fn handle_close_shortcut(&mut self, window: &mut tekes_ui_platform_macos::Window) -> bool {
        self.0.handle_close_shortcut(window)
    }

    fn handle(
        &mut self,
        event: tekes_ui_platform_macos::Event,
        window: &mut tekes_ui_platform_macos::Window,
    ) {
        self.0.handle(event, window);
    }
}

#[cfg(target_os = "macos")]
impl HostedWindow {
    pub fn theme(&self) -> tekes_ui_tokens::Theme {
        self.0.theme
    }

    pub fn set_theme(&mut self, theme: tekes_ui_tokens::Theme) {
        self.0.theme = theme;
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.0.set_reduced_motion(reduced);
    }

    pub fn set_motion_elapsed(&mut self, elapsed: std::time::Duration) {
        self.0.set_headless_motion_elapsed(elapsed);
    }

    pub fn set_type_scale(&mut self, type_scale: f32) {
        if type_scale.is_finite() && type_scale > 0. {
            let baseline = tekes_ui_tokens::proposal::TextRole::UiBody.style().size;
            self.0.text.set_body_size(baseline * type_scale);
            self.0.text.type_scale = type_scale;
        }
    }

    pub fn renderer_mut(&mut self) -> Option<&mut tekes_ui_render::Renderer> {
        self.0.renderer.as_mut()
    }

    pub fn glyph_atlas_mut(&mut self) -> &mut tekes_ui_scene::GlyphAtlas {
        &mut self.0.text.atlas
    }

    pub fn text_host_mut(&mut self) -> &mut tekes_ui_macos_bridge::coretext_host::CoreTextHost {
        &mut self.0.text.0
    }

    pub fn build_headless_scene(
        &mut self,
        renderer: &mut tekes_ui_render::Renderer,
        whole: tekes_ui_scene::Rect,
        scale: f32,
        active: bool,
    ) -> anyhow::Result<(tekes_ui_scene::Scene, tekes_ui_tokens::Rgba)> {
        self.0.build_headless_scene(renderer, whole, scale, active)
    }

    pub fn render_headless_rgba8(
        &mut self,
        renderer: &mut tekes_ui_render::Renderer,
        size_pt: (u32, u32),
        scale: u32,
    ) -> anyhow::Result<Vec<u8>> {
        self.0.render_headless_rgba8(renderer, size_pt, scale)
    }
}

/// Fixed local/process column assembly over the shared native window shell.
#[cfg(target_os = "macos")]
pub struct AppWindow {
    theme: tekes_ui_tokens::Theme,
    palette_source: Option<std::rc::Rc<std::cell::Cell<tekes_ui_tokens::Palette>>>,
    profile: AppProfile,
    components: Vec<ComponentDefinition>,
    columns: Option<Vec<Column>>,
    navigation: Option<NavigationOptions>,
    settings: Option<settings::SettingsConfiguration>,
    presentation: Option<PresentationOptions>,
    container_factories: Vec<ContainerFactory>,
    extension_icons: Vec<tekes_ui_icons::Glyph>,
    app_name: Option<String>,
    window_title: Option<String>,
    quit_after_last_window_closed: Option<bool>,
}
#[cfg(target_os = "macos")]
impl std::fmt::Debug for AppWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWindow")
            .field("theme", &self.theme)
            .field("profile", &self.profile)
            .field("components", &self.components)
            .finish_non_exhaustive()
    }
}
/// A consumer-owned window declaration opened by the current native event loop.
/// Requesting another window does not create a nested application or input owner.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub struct WindowOpenRequest {
    pub app: AppWindow,
    pub config: tekes_ui_platform_macos::WindowConfig,
}
#[cfg(target_os = "macos")]
impl AppWindow {
    pub fn new(profile: AppProfile) -> Self {
        Self {
            theme: tekes_ui_tokens::Theme::LIGHT,
            palette_source: None,
            profile,
            components: Vec::new(),
            columns: None,
            navigation: None,
            settings: None,
            presentation: None,
            container_factories: Vec::new(),
            extension_icons: Vec::new(),
            app_name: None,
            window_title: None,
            quit_after_last_window_closed: None,
        }
    }
    pub fn from_columns(columns: Vec<Column>) -> anyhow::Result<Self> {
        Self::from_columns_with_icons(columns, Vec::new())
    }
    /// Assemble fixed columns with host-registered extension icons validated upfront.
    pub fn from_columns_with_icons(
        columns: Vec<Column>,
        extension_icons: Vec<tekes_ui_icons::Glyph>,
    ) -> anyhow::Result<Self> {
        columns::validate_extension_icons(&extension_icons)?;
        columns::validate_with_icons(&columns, &extension_icons)?;
        let mut modules = Vec::new();
        let mut components = Vec::new();
        for column in &columns {
            if let Column::ProcessContainer(process) = column {
                match &process.backing {
                    columns::ProcessBacking::V1(definition) => modules.push(*definition),
                    columns::ProcessBacking::V2 { definition, .. } => components.push(*definition),
                }
            }
        }
        let profile = AppProfile { modules };
        validate_components(&profile, &components)?;
        Ok(Self {
            theme: tekes_ui_tokens::Theme::LIGHT,
            palette_source: None,
            profile,
            components,
            columns: Some(columns),
            navigation: None,
            settings: None,
            presentation: None,
            container_factories: Vec::new(),
            extension_icons,
            app_name: None,
            window_title: None,
            quit_after_last_window_closed: None,
        })
    }
    /// Assemble the optional NavBar and focus/others presentation from stable
    /// container IDs. All backing objects are moved into the original Host.
    pub fn from_containers(
        containers: Vec<Container>,
        presentation: PresentationOptions,
    ) -> anyhow::Result<Self> {
        Self::from_containers_with_icons(containers, presentation, Vec::new())
    }
    /// Assemble presentation containers with host-registered extension icons validated upfront.
    pub fn from_containers_with_icons(
        containers: Vec<Container>,
        presentation: PresentationOptions,
        extension_icons: Vec<tekes_ui_icons::Glyph>,
    ) -> anyhow::Result<Self> {
        columns::validate_extension_icons(&extension_icons)?;
        PresentationState::new_with_icons(&containers, presentation.clone(), &extension_icons)?;
        let columns: Vec<Column> = containers.into_iter().map(Column::from).collect();
        let mut modules = Vec::new();
        let mut components = Vec::new();
        for column in &columns {
            if let Column::ProcessContainer(process) = column {
                match &process.backing {
                    columns::ProcessBacking::V1(definition) => modules.push(*definition),
                    columns::ProcessBacking::V2 { definition, .. } => components.push(*definition),
                }
            }
        }
        let profile = AppProfile { modules };
        validate_components(&profile, &components)?;
        Ok(Self {
            theme: tekes_ui_tokens::Theme::LIGHT,
            palette_source: None,
            profile,
            components,
            columns: Some(columns),
            navigation: None,
            settings: None,
            presentation: Some(presentation),
            container_factories: Vec::new(),
            extension_icons,
            app_name: None,
            window_title: None,
            quit_after_last_window_closed: None,
        })
    }
    /// Register validated host extension glyphs before `.container_factories(...)` or opening.
    pub fn extension_icons(mut self, icons: Vec<tekes_ui_icons::Glyph>) -> anyhow::Result<Self> {
        let mut merged = self.extension_icons.clone();
        merged.extend(icons);
        columns::validate_extension_icons(&merged)?;
        self.extension_icons = merged;
        Ok(self)
    }
    pub(crate) fn into_host(self, acceptance: bool) -> window::Host {
        let theme = self.theme;
        let window_title = self.window_title.or_else(|| self.app_name.clone());
        let host = match self.columns {
            Some(columns) if self.presentation.is_some() => {
                window::Host::from_containers_with_icons(
                    columns.into_iter().map(Container::from).collect(),
                    acceptance,
                    self.navigation,
                    self.presentation,
                    self.extension_icons.clone(),
                )
            }
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
            .with_palette_source(self.palette_source)
            .with_window_title(window_title)
            .with_extension_icons(self.extension_icons)
            .with_container_factories(self.container_factories)
    }
    /// Register finite, consumer-defined creatable types before opening a window.
    /// Process definitions use the executable's existing fixed worker dispatch.
    pub fn container_factories(mut self, factories: Vec<ContainerFactory>) -> anyhow::Result<Self> {
        container_factory::validate_with_icons(&factories, &self.extension_icons)?;
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
                !self.profile.modules.iter().any(|m| m.id == definition.id)
                    && !self.components.iter().any(|c| c.id == definition.id),
                "worker factory registered more than once: {}",
                definition.id
            );
            self.profile.modules.push(definition);
        }
        for definition in factories
            .iter()
            .filter_map(ContainerFactory::component_definition)
        {
            anyhow::ensure!(
                !self.profile.modules.iter().any(|m| m.id == definition.id)
                    && !self.components.iter().any(|c| c.id == definition.id),
                "component factory registered more than once: {}",
                definition.id
            );
            self.components.push(definition);
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
    /// Read compiled color choices from this UI-thread source on each paint.
    /// Window appearance/contrast (including Settings) remains controlled by
    /// `theme`; it is projected onto the source palette. Changing this source
    /// during a normal UI action takes effect on that action's next redraw.
    pub fn palette_source(
        mut self,
        source: std::rc::Rc<std::cell::Cell<tekes_ui_tokens::Palette>>,
    ) -> Self {
        self.palette_source = Some(source);
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
    /// Configure the application name shown in the macOS menu bar
    /// (`About / Hide / Quit <AppName>`). When `window_title` is not set
    /// separately, this also serves as the initial main window title.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }
    /// Configure the initial title for the main application window.
    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }
    /// Configure whether closing the last window terminates the process
    /// instead of keeping the macOS app resident in the Dock and menu bar.
    /// Defaults to `false` for normal runs and `true` for `--window-acceptance`.
    pub fn quit_after_last_window_closed(mut self, quit: bool) -> Self {
        self.quit_after_last_window_closed = Some(quit);
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
            &self.extension_icons,
        )?;
        validate_components(&self.profile, &self.components)?;
        let settings = self.settings.take();
        self.into_host(false).open(parent, config, settings)
    }

    pub fn run(self) -> anyhow::Result<()> {
        let args: Vec<String> = std::env::args().collect();
        run_columns(self, &args, RunMode::Consumer)
    }

    /// Launch this window under the finite `window-acceptance` test driver.
    #[cfg(feature = "window-acceptance")]
    pub fn run_window_acceptance(self) -> anyhow::Result<()> {
        let args: Vec<String> = std::env::args().collect();
        run_columns(self, &args, RunMode::WindowAcceptance)
    }

    /// Run with explicit startup arguments, including the executable at argv[0].
    /// Embedding consumers remove only their own already-consumed arguments;
    /// all remaining worker and fixed-profile argument validation still applies.
    pub fn run_with_args(self, args: &[String]) -> anyhow::Result<()> {
        run_columns(self, args, RunMode::ValidateArgs)
    }

    /// Validate this window assembly and construct its [`HostedWindow`] handler
    /// and [`tekes_ui_platform_macos::MenuBar`] without starting the native event loop.
    pub fn into_window_handler(
        mut self,
    ) -> anyhow::Result<(tekes_ui_platform_macos::MenuBar, HostedWindow)> {
        validate_assembly(
            &self.profile,
            self.columns.as_deref(),
            self.navigation.is_some(),
            self.presentation.as_ref(),
            &self.extension_icons,
        )?;
        validate_components(&self.profile, &self.components)?;
        let settings = self.settings.take();
        let mut host = self.into_host(false);
        host.mount_initial_containers();
        host.configure_settings(settings)?;
        let menus = host.menu_bar();
        menus.validate()?;
        Ok((menus, HostedWindow(host)))
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunMode {
    Consumer,
    ValidateArgs,
    #[cfg(feature = "window-acceptance")]
    WindowAcceptance,
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
    let mut app = AppWindow::new(profile);
    app.navigation = navigation;
    app.settings = settings;
    run_columns(
        app,
        &std::env::args().collect::<Vec<_>>(),
        RunMode::Consumer,
    )
}
#[cfg(target_os = "macos")]
fn validate_assembly(
    profile: &AppProfile,
    columns: Option<&[Column]>,
    navigation: bool,
    presentation: Option<&PresentationOptions>,
    extension_icons: &[tekes_ui_icons::Glyph],
) -> anyhow::Result<()> {
    columns::validate_extension_icons(extension_icons)?;
    if let Some(columns) = columns {
        if let Some(presentation) = presentation {
            PresentationState::from_columns_with_icons(
                columns,
                presentation.clone(),
                extension_icons,
            )?;
        } else {
            columns::validate_with_icons(columns, extension_icons)?;
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
fn run_columns(app: AppWindow, args: &[String], mode: RunMode) -> anyhow::Result<()> {
    validate_assembly(
        &app.profile,
        app.columns.as_deref(),
        app.navigation.is_some(),
        app.presentation.as_ref(),
        &app.extension_icons,
    )?;
    validate_components(&app.profile, &app.components)?;
    let mut all_components = vec![navigation::definition(), settings_undo::definition()];
    for component in &app.components {
        if !all_components
            .iter()
            .any(|existing| existing.id == component.id)
        {
            all_components.push(*component);
        }
    }
    if tekes_ui_module_runtime::run_component_worker_from_args(&all_components, args)? {
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--worker") {
        anyhow::ensure!(args.len() == 4, "invalid worker arguments");
        let definition = app
            .profile
            .modules
            .iter()
            .find(|m| m.id == args[2])
            .ok_or_else(|| anyhow::anyhow!("worker module not in fixed profile"))?;
        return tekes_ui_module_runtime::run((definition.create)(), &args[3]);
    }
    let acceptance = resolve_acceptance(mode, args)?;
    window::run(app, acceptance)
}
#[cfg(target_os = "macos")]
fn resolve_acceptance(mode: RunMode, args: &[String]) -> anyhow::Result<bool> {
    match mode {
        RunMode::Consumer => Ok(false),
        RunMode::ValidateArgs => window_acceptance_from_args(args),
        #[cfg(feature = "window-acceptance")]
        RunMode::WindowAcceptance => Ok(true),
    }
}
#[cfg(target_os = "macos")]
fn window_acceptance_from_args(args: &[String]) -> anyhow::Result<bool> {
    match args.get(1).map(String::as_str) {
        None => Ok(false),
        #[cfg(feature = "window-acceptance")]
        Some("--window-acceptance") if args.len() == 2 => Ok(true),
        _ => anyhow::bail!("fixed-profile app accepts no runtime module discovery or selection"),
    }
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
    fn hosted_run_argument_gate_requires_consumed_consumer_flags() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        // Reproduces the native Gallery launch failure at the same run gate.
        assert!(window_acceptance_from_args(&args(&["gallery", "--apphost"])).is_err());
        assert!(!window_acceptance_from_args(&args(&["gallery"])).unwrap());
        #[cfg(feature = "window-acceptance")]
        assert!(window_acceptance_from_args(&args(&["gallery", "--window-acceptance"])).unwrap());
        #[cfg(not(feature = "window-acceptance"))]
        assert!(window_acceptance_from_args(&args(&["gallery", "--window-acceptance"])).is_err());
        // Consumer AppWindow::run does not reject arbitrary host CLI flags or enable acceptance.
        assert!(
            !resolve_acceptance(
                RunMode::Consumer,
                &args(&[
                    "app",
                    "--open",
                    "doc.tekes",
                    "-psn_0_1234",
                    "--window-acceptance"
                ]),
            )
            .unwrap()
        );
        for tail in [
            vec!["--modules", "unknown"],
            vec!["unknown"],
            vec!["--window-acceptance", "extra"],
            vec!["--worker", "unregistered", "socket"],
            vec!["--worker", "unregistered"],
        ] {
            let mut remaining = vec!["gallery".to_owned()];
            remaining.extend(tail.into_iter().map(str::to_owned));
            assert!(window_acceptance_from_args(&remaining).is_err());
            let local = LocalContainer::new(
                "focus",
                Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
            )
            .unwrap();
            let app =
                AppWindow::from_containers(vec![local.into()], PresentationOptions::new("focus"))
                    .unwrap();
            // Exercise the public startup path; each invalid argument must fail
            // before any worker connection or native window is created.
            assert!(app.run_with_args(&remaining).is_err());
        }
    }
    #[test]
    fn hosted_open_uses_run_assembly_preconditions_before_native_creation() {
        let empty = AppProfile { modules: vec![] };
        assert!(validate_assembly(&empty, None, false, None, &[]).is_err());
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
                app.presentation.as_ref(),
                &app.extension_icons,
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
                app.presentation.as_ref(),
                &app.extension_icons,
            )
            .is_ok()
        );
        let presentation = app.presentation.unwrap().navigation_toolbar("focus");
        assert!(
            validate_assembly(
                &app.profile,
                app.columns.as_deref(),
                true,
                Some(&presentation),
                &app.extension_icons,
            )
            .is_ok()
        );
    }

    #[test]
    fn app_window_identity_and_quit_policy_builders_and_fallback() {
        let focus = LocalContainer::new(
            "focus",
            Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
        )
        .unwrap();
        let default_app =
            AppWindow::from_containers(vec![focus.into()], PresentationOptions::new("focus"))
                .unwrap();
        assert_eq!(default_app.app_name, None);
        assert_eq!(default_app.window_title, None);
        assert_eq!(default_app.quit_after_last_window_closed, None);

        let fallback_focus = LocalContainer::new(
            "focus",
            Content::new(ComponentId(2), TextFieldContent::new("Field", "")),
        )
        .unwrap();
        let app_named = AppWindow::from_containers(
            vec![fallback_focus.into()],
            PresentationOptions::new("focus"),
        )
        .unwrap()
        .app_name("Studio")
        .quit_after_last_window_closed(false);
        assert_eq!(app_named.app_name.as_deref(), Some("Studio"));
        assert_eq!(app_named.window_title, None);
        assert_eq!(app_named.quit_after_last_window_closed, Some(false));
        let host = app_named.into_host(false);
        assert_eq!(host.window_title(), Some("Studio"));

        let custom_app = AppWindow::new(AppProfile { modules: vec![] })
            .app_name("Studio")
            .window_title("Workspace · Alpha")
            .quit_after_last_window_closed(true);
        assert_eq!(custom_app.app_name.as_deref(), Some("Studio"));
        assert_eq!(
            custom_app.window_title.as_deref(),
            Some("Workspace · Alpha")
        );
        assert_eq!(custom_app.quit_after_last_window_closed, Some(true));
        let host = custom_app.into_host(false);
        assert_eq!(host.window_title(), Some("Workspace · Alpha"));
    }

    #[test]
    fn host_extension_icons_flow_through_validation_factories_toolbars_and_rendering() {
        const EXT: Glyph = Glyph {
            name: "host-custom-tool",
            svg: r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 12h14" stroke="currentColor" stroke-width="1.5"/></svg>"#,
        };
        let invalid_name = Glyph {
            name: " ",
            svg: EXT.svg,
        };
        let invalid_svg = Glyph {
            name: "host-bad-svg",
            svg: "<svg>",
        };
        assert!(
            AppWindow::from_columns_with_icons(
                vec![
                    LocalContainer::new(
                        "main",
                        Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
                    )
                    .unwrap()
                    .into()
                ],
                vec![invalid_name],
            )
            .is_err()
        );
        assert!(
            AppWindow::from_columns_with_icons(
                vec![
                    LocalContainer::new(
                        "main",
                        Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
                    )
                    .unwrap()
                    .into()
                ],
                vec![EXT, EXT],
            )
            .is_err()
        );
        assert!(
            AppWindow::from_columns_with_icons(
                vec![
                    LocalContainer::new(
                        "main",
                        Content::new(ComponentId(1), TextFieldContent::new("Field", "")),
                    )
                    .unwrap()
                    .into()
                ],
                vec![invalid_svg],
            )
            .is_err()
        );

        let make_focus = |icon_name: &str| {
            LocalContainer::new(
                "focus",
                Content::new(ComponentId(1), TextFieldContent::new("Focus", "")),
            )
            .unwrap()
            .title("Focus")
            .switcher_icon(icon_name)
            .toolbar_items(vec![
                LocalToolbarItem::Button(
                    LocalToolbarButton::new(1, "Tool")
                        .icon(icon_name)
                        .tooltip("Custom tool"),
                ),
                LocalToolbarItem::Menu(
                    LocalToolbarMenu::new(2, "Menu", vec![])
                        .icon(icon_name)
                        .tooltip("Custom menu"),
                ),
                LocalToolbarItem::Group(LocalToolbarGroup::new(
                    3,
                    vec![LocalToolbarItem::Button(
                        LocalToolbarButton::new(4, "Grouped").icon(icon_name),
                    )],
                )),
            ])
        };
        let make_side = || {
            LocalContainer::new(
                "side",
                Content::new(ComponentId(2), TextFieldContent::new("Side", "")),
            )
            .unwrap()
            .title("Side")
            .switcher_icon("host-custom-tool")
        };

        // Without registering the extension icon, existing errors are preserved at each seam.
        let err = AppWindow::from_containers(
            vec![make_focus("host-custom-tool").into()],
            PresentationOptions::new("focus"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown switcher icon"), "{err}");

        let err = PresentationState::new_with_icons(
            &[make_focus("tekes-new").into(), make_side().into()],
            PresentationOptions::new("focus").others(
                OthersOptions::new(vec!["side".into()]).groups(vec![OthersGroup::new(
                    "g",
                    "Group",
                    "unknown-group-icon",
                    vec!["side".into()],
                )]),
            ),
            &[EXT],
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unknown Others group icon"),
            "{err}"
        );

        let err = AppWindow::from_containers(
            vec![make_focus("tekes-new").into()],
            PresentationOptions::new("focus").others(OthersOptions::new(vec![])),
        )
        .unwrap()
        .container_factories(vec![
            ContainerFactory::local(
                "custom-fact",
                "Custom Factory",
                ContainerMultiplicity::Multiple,
                |id| {
                    LocalContainer::new(
                        id,
                        Content::new(ComponentId(10), TextFieldContent::new("Dyn", "")),
                    )
                },
            )
            .switcher_icon("host-custom-tool"),
        ])
        .unwrap_err();
        assert!(
            err.to_string().contains("unknown container factory icon"),
            "{err}"
        );

        // Registering the extension icon unlocks containers, OthersGroup, factories, toolbars,
        // switcher tiles, and real SVG rasterization in headless scene rendering.
        let side = make_side();
        let presentation = PresentationOptions::new("focus").others(
            OthersOptions::new(vec!["side".into()]).groups(vec![OthersGroup::new(
                "custom-group",
                "Group",
                "host-custom-tool",
                vec!["side".into()],
            )]),
        );
        let app = AppWindow::from_containers_with_icons(
            vec![make_focus("host-custom-tool").into(), side.into()],
            presentation,
            vec![EXT],
        )
        .unwrap()
        .container_factories(vec![
            ContainerFactory::local(
                "custom-fact",
                "Custom Factory",
                ContainerMultiplicity::Multiple,
                |id| {
                    LocalContainer::new(
                        id,
                        Content::new(ComponentId(10), TextFieldContent::new("Dyn", "")),
                    )
                },
            )
            .switcher_icon("host-custom-tool"),
        ])
        .unwrap();

        let (_menus, mut hosted) = app.into_window_handler().unwrap();
        let created = hosted.0.create_container("custom-fact").unwrap();
        assert!(created.starts_with("custom-fact-instance-"));

        // Empty Others + factory also exercises window_switcher creation grid tiles with EXT icons.
        let grid_app = AppWindow::from_containers_with_icons(
            vec![make_focus("host-custom-tool").into()],
            PresentationOptions::new("focus").others(OthersOptions::new(vec![])),
            vec![EXT],
        )
        .unwrap()
        .container_factories(vec![
            ContainerFactory::local(
                "grid-fact",
                "Grid Factory",
                ContainerMultiplicity::Multiple,
                |id| {
                    LocalContainer::new(
                        id,
                        Content::new(ComponentId(11), TextFieldContent::new("Grid", "")),
                    )
                },
            )
            .switcher_icon("host-custom-tool"),
        ])
        .unwrap();
        let (_menus2, mut grid_hosted) = grid_app.into_window_handler().unwrap();

        if let Ok(mut renderer) = tekes_ui_render::Renderer::new_headless() {
            let whole = tekes_ui_scene::Rect::new(0., 0., 960., 640.);
            let (scene, _) = hosted
                .build_headless_scene(&mut renderer, whole, 2.0, true)
                .unwrap();
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, tekes_ui_scene::Primitive::Image(_))),
                "toolbar and presentation icons must rasterize and emit image quads"
            );
            let rgba = hosted
                .render_headless_rgba8(&mut renderer, (960, 640), 2)
                .unwrap();
            assert_eq!(rgba.len(), 960 * 640 * 4 * 4);

            let (grid_scene, _) = grid_hosted
                .build_headless_scene(&mut renderer, whole, 2.0, true)
                .unwrap();
            assert!(
                grid_scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, tekes_ui_scene::Primitive::Image(_))),
                "creation grid tiles must rasterize extension icons"
            );
        }

        // Unregistered toolbar template/menu icon names still fail with existing error messages.
        let mut mounted = crate::window_toolbar::MountedToolbar::new();
        mounted.icons.set_extensions(vec![EXT]);
        let bad_menu = crate::window_toolbar::Declaration {
            id: ComponentId(70001),
            region: ComponentId(91020),
            placement: tekes_ui_controls::composition::ToolbarPlacement::PrimaryAction,
            order: 0,
            contents: crate::window_toolbar::Contents::Menu(
                crate::window_toolbar::MenuDeclaration {
                    id: 99,
                    title: "Bad".into(),
                    accessible_name: None,
                    icon: Some("missing-extension".into()),
                    icon_position: tekes_ui_controls::ButtonImagePosition::Leading,
                    icon_slot: true,
                    tooltip: None,
                    enabled: true,
                    selected: false,
                    tone: tekes_ui_controls::ButtonTone::Default,
                    entries: vec![],
                },
            ),
            source: crate::window_toolbar::Source::Title,
        };
        let err = mounted
            .sync(
                tekes_ui_scene::Rect::new(0., 0., 400., 48.),
                vec![tekes_ui_controls::composition::ToolbarRegion::new(
                    ComponentId(91020),
                    tekes_ui_scene::Rect::new(0., 0., 400., 48.),
                )],
                vec![bad_menu],
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("unknown toolkit menu icon"),
            "{err}"
        );
    }

    #[test]
    fn existing_side_chat_factory_and_legacy_workflow_icon_aliases_validate_and_render() {
        let focus = LocalContainer::new(
            "focus",
            Content::new(ComponentId(1), TextFieldContent::new("Workspace", "")),
        )
        .unwrap()
        .title("Workspace")
        .switcher_icon("tekes-access-workspace")
        .toolbar_items(vec![
            LocalToolbarItem::Button(
                LocalToolbarButton::new(1, "Complete").icon("tekes-goal-completed"),
            ),
            LocalToolbarItem::Menu(
                LocalToolbarMenu::new(2, "Access", vec![]).icon("tekes-access-workspace"),
            ),
        ]);
        let app = AppWindow::from_containers(
            vec![focus.into()],
            PresentationOptions::new("focus").others(OthersOptions::new(vec![])),
        )
        .unwrap()
        .container_factories(vec![
            ContainerFactory::local(
                "side-chat",
                "SideChat",
                ContainerMultiplicity::Single,
                |id| {
                    LocalContainer::new(
                        id,
                        Content::new(ComponentId(2), TextFieldContent::new("SideChat", "")),
                    )
                },
            )
            .switcher_icon("tekes-side-chat"),
        ])
        .unwrap();
        let (_menus, mut hosted) = app.into_window_handler().unwrap();
        let created_id = hosted.0.create_container("side-chat").unwrap();
        assert!(created_id.starts_with("side-chat-instance-"));
        if let Ok(mut renderer) = tekes_ui_render::Renderer::new_headless() {
            let (scene, _) = hosted
                .build_headless_scene(
                    &mut renderer,
                    tekes_ui_scene::Rect::new(0., 0., 960., 640.),
                    2.0,
                    true,
                )
                .unwrap();
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p, tekes_ui_scene::Primitive::Image(_)))
            );
        }
    }
}
