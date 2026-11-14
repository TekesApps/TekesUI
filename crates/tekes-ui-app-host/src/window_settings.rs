use super::*;
use tekes_ui_controls::composition::{Component, ContentResponse, TextKeyDisposition};
use tekes_ui_settings::FormIntent;
/// The ticket and receiver have the same requesting-Host lifetime. The worker
/// never owns the ticket, so closing that Host releases the owner's load gate.
pub(super) struct SettingsLoad {
    pub(super) _ticket: std::rc::Rc<()>,
    pub(super) receiver: Receiver<Result<tekes_ui_settings::Snapshot, String>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsFocusScope {
    Toolbar(ToolbarTarget),
    Sidebar,
    Form,
}
fn settings_content_inset(
    bounds: Rect,
    window: Rect,
    measured: f32,
    row_y: f32,
    row_height: f32,
    controls_floor: f32,
    leading: f32,
) -> f32 {
    let native_row = bounds.min_y() == window.min_y();
    let intersects_controls =
        bounds.min_y() < controls_floor && bounds.min_x() < window.min_x() + leading;
    presentation_content_inset(
        measured,
        (row_y + row_height).max(controls_floor),
        (controls_floor - bounds.min_y()).max(0.),
        native_row,
        intersects_controls,
    )
}
pub(super) struct SettingsDisplayBinding {
    owner: std::rc::Weak<std::cell::RefCell<crate::settings::SettingsOwnerState>>,
    display: std::rc::Rc<std::cell::RefCell<crate::settings::SettingsDisplayState>>,
    wake: Option<std::rc::Rc<WindowWake>>,
    loading: Option<Receiver<Result<tekes_ui_settings::Snapshot, String>>>,
    applied: Option<(Theme, f32)>,
    pub(super) system_theme: Theme,
}
impl SettingsDisplayBinding {
    pub(super) fn new(owner: &crate::SettingsOwner, system_theme: Theme) -> Option<Self> {
        let display = owner.0.borrow().display.clone()?;
        Some(Self {
            display,
            owner: std::rc::Rc::downgrade(&owner.0),
            wake: None,
            loading: None,
            applied: None,
            system_theme,
        })
    }
    fn poll(&mut self, w: &Window) -> Result<()> {
        if self.wake.is_none() {
            let wake = std::rc::Rc::new(w.wake_proxy());
            let mut state = self.display.borrow_mut();
            state.wakes.retain(|wake| wake.strong_count() > 0);
            state.wakes.push(std::rc::Rc::downgrade(&wake));
            // Each newly attached window can finish its own startup read even
            // if another opener closes. Reads share the original provider.
            if state.preferences.is_none()
                && let Some(owner) = self.owner.upgrade()
            {
                let provider = owner.borrow().provider.clone();
                let (sender, receiver) = mpsc::channel();
                let read_wake = (*wake).clone();
                std::thread::spawn(move || {
                    let _ = sender.send(provider.load().map_err(|error| format!("{error:#}")));
                    read_wake.wake();
                });
                self.loading = Some(receiver);
            }
            self.wake = Some(wake);
        }
        if let Some(result) = self
            .loading
            .as_ref()
            .and_then(|receiver| match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Settings display load worker disconnected".into()))
                }
            })
        {
            self.loading = None;
            let snapshot = result.map_err(anyhow::Error::msg)?;
            crate::settings::SettingsDisplayState::publish(&self.display, &snapshot)?;
        }
        Ok(())
    }
    fn apply(&mut self, theme: &mut Theme, text: &mut crate::text::Text) -> bool {
        let Some(preferences) = self.display.borrow().preferences else {
            return false;
        };
        let Some((next_theme, percent)) = self.resolve(preferences) else {
            return false;
        };
        *theme = next_theme;
        text.set_body_size(
            tekes_ui_tokens::proposal::TextRole::UiBody.style().size * percent / 100.,
        );
        true
    }
    fn resolve(
        &mut self,
        preferences: crate::settings::DisplayPreferences,
    ) -> Option<(Theme, f32)> {
        let desired = (
            Theme {
                appearance: preferences
                    .appearance
                    .unwrap_or(self.system_theme.appearance),
                contrast: self.system_theme.contrast,
            },
            preferences.text_size_percent,
        );
        if self.applied == Some(desired) {
            return None;
        }
        self.applied = Some(desired);
        Some(desired)
    }
}
struct SettingsColumn {
    state: std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>,
    sidebar: bool,
}
impl crate::ContainerContent for SettingsColumn {
    fn with_host(&mut self, visit: &mut dyn FnMut(&mut ContentHost)) {
        let mut state = self.state.borrow_mut();
        visit(if self.sidebar {
            &mut state.sidebar.host
        } else {
            &mut state.form.host
        });
    }
}
fn settings_columns(
    surface: &std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>,
) -> Vec<crate::Column> {
    vec![
        crate::Column::from(crate::LocalContainer::from_source(
            "settings-leading",
            SettingsColumn {
                state: surface.clone(),
                sidebar: true,
            },
        )),
        crate::Column::from(crate::LocalContainer::from_source(
            "settings-center",
            SettingsColumn {
                state: surface.clone(),
                sidebar: false,
            },
        )),
    ]
}
impl Host {
    pub(super) fn sync_settings_display(&mut self, w: &Window) -> Result<()> {
        if let Some(binding) = &mut self.settings_display {
            binding.poll(w)?;
            if binding.apply(&mut self.theme, &mut self.text) {
                w.request_redraw();
            }
        }
        Ok(())
    }
    // Original form input boundary, kept separate from native Window access so
    // ownership regressions exercise the same mounted Host path as production.
    fn settings_form_input(
        &mut self,
        event: &ContentEvent,
    ) -> Result<(Vec<FormIntent>, ContentResponse)> {
        // Opt-in diagnosis of the observed native menu click. Do not log text
        // input or settings values; pointer delivery and overlay ownership are
        // sufficient to distinguish routing from menu activation.
        let trace_pointer = matches!(
            event,
            ContentEvent::Pointer(_) | ContentEvent::PointerDown(_) | ContentEvent::PointerUp(_)
        ) && std::env::var_os("TEKESUI_SETTINGS_INPUT_TRACE").is_some();
        if trace_pointer {
            eprintln!("SETTINGS_FORM_POINTER before event={event:?}");
        }
        let response = self.panels[1]
            .local
            .as_mut()
            .unwrap()
            .event_with_response(event, &mut self.text)?;
        let shared = self.settings.as_ref().unwrap().clone();
        let mut state = shared.borrow_mut();
        let settings = &mut *state;
        let intents = settings
            .form
            .collect(&settings.view_schema, &mut settings.draft);
        let form_focused = settings.form.host.focused().is_some();
        if trace_pointer {
            eprintln!(
                "SETTINGS_FORM_POINTER after focus={:?} overlay={:?} bubbled={}",
                settings.form.host.focused(),
                settings.form.host.input_overlay_owner(),
                settings.form.host.event_bubbled()
            );
        }
        if form_focused {
            settings.sidebar.host.focus(None, false);
        }
        drop(state);
        if form_focused {
            self.set_toolbar_focus(None, false);
            self.focused = 1;
        }
        Ok((intents, response))
    }
    /// Settings embeds the original keyboard scopes in window order. Other
    /// AppHosts retain their original toolbar traversal contract.
    pub(super) fn settings_tab_input(&mut self, key: &ContentKeyEvent) -> Result<Option<bool>> {
        let Some(ControlKey::Tab { reverse }) = key.key else {
            return Ok(None);
        };
        if !self.is_settings_window
            || !key.down
            || key.modifiers.command
            || key.modifiers.control
            || key.modifiers.alt
            || self.open_toolbar().is_some()
        {
            return Ok(None);
        }
        self.settings_scope_tab(reverse, &ContentEvent::KeyInput(key.clone()))?;
        Ok(Some(true))
    }
    fn settings_scope_tab(&mut self, reverse: bool, event: &ContentEvent) -> Result<()> {
        let shared = self.settings.as_ref().unwrap().clone();
        let completing = matches!(event, ContentEvent::TextKeyEnd { .. });
        if self
            .navigation
            .as_ref()
            .is_some_and(|nav| nav.busy || !nav.host.input_open)
            || shared.borrow().restoring()
        {
            return Ok(());
        }
        {
            let mut state = shared.borrow_mut();
            // Selection/navigation can replace a form Host. Configure the
            // current original hosts, rather than relying on an old mount.
            state.sidebar.host.set_focus_wrap(false);
            state.form.host.set_focus_wrap(false);
            if !completing && (state.sidebar.host.is_composing() || state.form.host.is_composing())
            {
                return Ok(());
            }
        }
        let mut scopes: Vec<_> = self
            .toolbar_focus_targets()
            .into_iter()
            .map(SettingsFocusScope::Toolbar)
            .collect();
        if self.leading_visible {
            scopes.push(SettingsFocusScope::Sidebar);
        }
        scopes.push(SettingsFocusScope::Form);
        let current = self
            .actual_toolbar_owner()
            .map(SettingsFocusScope::Toolbar)
            .or_else(|| {
                let state = shared.borrow();
                if self.leading_visible && state.sidebar.host.focused().is_some() {
                    Some(SettingsFocusScope::Sidebar)
                } else if state.form.host.focused().is_some() {
                    Some(SettingsFocusScope::Form)
                } else {
                    None
                }
            });
        if completing
            && !matches!(
                current,
                Some(SettingsFocusScope::Sidebar | SettingsFocusScope::Form)
            )
        {
            return Ok(());
        }
        if let Some(scope @ (SettingsFocusScope::Sidebar | SettingsFocusScope::Form)) = current {
            let response = if scope == SettingsFocusScope::Sidebar {
                self.panels[0]
                    .local
                    .as_mut()
                    .unwrap()
                    .event_with_response(event, &mut self.text)?
            } else {
                // Plain autosave-form Tab changes focus/edits, not Apply/Cancel.
                self.settings_form_input(event)?.1
            };
            let mut state = shared.borrow_mut();
            let bubbles = if scope == SettingsFocusScope::Sidebar {
                state.sidebar.host.event_bubbled()
            } else {
                state.form.host.event_bubbled()
            };
            state.edited();
            // Only the original editor can authorize continuation of a native
            // Tab. A duplicate finish has no acknowledgement; consumed IME or
            // invalid-number completions must retain their existing owner.
            if let ContentEvent::TextKeyEnd { id, .. } = event
                && !response.text_key.is_some_and(|outcome| {
                    outcome.id == *id && outcome.disposition == TextKeyDisposition::Unhandled
                })
            {
                return Ok(());
            }
            if !bubbles {
                return Ok(());
            }
            if scope == SettingsFocusScope::Form && !state.prepare_item_navigation() {
                return Ok(());
            }
        }
        let start =
            current.and_then(|scope| scopes.iter().position(|candidate| *candidate == scope));
        for step in 1..=scopes.len() {
            let index = match start {
                Some(index) if reverse => (index + scopes.len() - step) % scopes.len(),
                Some(index) => (index + step) % scopes.len(),
                None if reverse => scopes.len() - step,
                None => step - 1,
            };
            let target = scopes[index];
            let accepted = match target {
                SettingsFocusScope::Toolbar(target) => {
                    self.set_toolbar_focus(Some(target), true);
                    self.actual_toolbar_owner() == Some(target)
                }
                SettingsFocusScope::Sidebar => {
                    shared.borrow_mut().sidebar.host.enter_focus(reverse)
                }
                SettingsFocusScope::Form => shared.borrow_mut().form.host.enter_focus(reverse),
            };
            if !accepted {
                continue;
            }
            let mut state = shared.borrow_mut();
            if target != SettingsFocusScope::Sidebar {
                state.sidebar.host.focus(None, false);
            }
            if target != SettingsFocusScope::Form {
                state.form.host.focus(None, false);
            }
            drop(state);
            if !matches!(target, SettingsFocusScope::Toolbar(_)) {
                self.set_toolbar_focus(None, false);
                self.focused = usize::from(target == SettingsFocusScope::Form);
            }
            return Ok(());
        }
        Ok(())
    }
    pub(super) fn prepare_settings_item_navigation(&mut self) -> bool {
        let allowed = self
            .settings
            .as_ref()
            .is_none_or(|state| state.borrow_mut().prepare_item_navigation());
        if !allowed {
            // Reject before a request or cross-Host focus cancellation. No
            // history/input gate is changed, and the actual form keeps its IME.
            self.set_toolbar_focus(None, false);
        }
        allowed
    }
    fn sidebar_navigation_point(&self, point: Point) -> bool {
        self.settings.as_ref().is_some_and(|state| {
            let state = state.borrow();
            state.sidebar.host.semantics().iter().any(|node| {
                state
                    .sidebar
                    .host
                    .get(node.id)
                    .and_then(|n| {
                        n.component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                    })
                    .is_some_and(|list| {
                        list.semantic_parts()
                            .iter()
                            .any(|row| row.frame.contains(point))
                    })
            })
        })
    }
    pub(super) fn settings_navigation_focus_guard(
        &mut self,
        event: &Event,
        publication: Option<
            &tekes_ui_platform_macos::content_accessibility::WindowAccessibilityPublication,
        >,
    ) -> bool {
        let Some(state) = &self.settings else {
            return false;
        };
        if self
            .navigation
            .as_ref()
            .is_some_and(|nav| nav.busy || !nav.host.input_open)
            || state.borrow().restoring()
        {
            return false;
        }
        let navigation_toolbar = self
            .toolbar_container(ToolbarTarget::Navigation)
            .unwrap_or(self.toolbar_index);
        let mut back = false;
        let target = match event {
            Event::Mouse(mouse)
                if matches!(
                    mouse.kind,
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    }
                ) =>
            {
                let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
                back = self.leading_visible
                    && state
                        .borrow()
                        .sidebar
                        .host
                        .get(ComponentId(4))
                        .and_then(|node| node.visible())
                        .is_some_and(|visible| visible.contains(point));
                (self.panels[navigation_toolbar]
                    .mounted_toolbar
                    .host
                    .content_host()
                    .get(Self::toolbar_control(ToolbarTarget::Navigation))
                    .is_some_and(|node| node.frame().contains(point))
                    || self.panels[navigation_toolbar]
                        .mounted_toolbar
                        .host
                        .content_host()
                        .input_overlay_owner()
                        .is_some_and(|(id, _)| {
                            id == Self::toolbar_control(ToolbarTarget::Navigation)
                        }))
                    || (self.leading_visible && self.sidebar_navigation_point(point))
                    || back
            }
            Event::Key(key) if key.kind == KeyEventKind::Down => {
                let key = toolbar_key(key.clone()).key;
                let sidebar = state.borrow();
                let focused_list = sidebar
                    .sidebar
                    .host
                    .focused()
                    .and_then(|id| sidebar.sidebar.host.get(id))
                    .is_some_and(|node| {
                        node.component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                            .is_some()
                    });
                back = self.leading_visible
                    && sidebar.sidebar.host.focused() == Some(ComponentId(4))
                    && matches!(key, Some(ControlKey::Enter | ControlKey::Space));
                (focused_list
                    && matches!(
                        key,
                        Some(
                            ControlKey::Up
                                | ControlKey::Down
                                | ControlKey::Home
                                | ControlKey::End
                                | ControlKey::Enter
                        )
                    ))
                    || (self.panels[navigation_toolbar]
                        .mounted_toolbar
                        .host
                        .content_host()
                        .focused()
                        == Some(Self::toolbar_control(ToolbarTarget::Navigation))
                        && matches!(key, Some(ControlKey::Enter | ControlKey::Space)))
                    || back
            }
            Event::Accessibility {
                target,
                action:
                    tekes_ui_platform_macos::AccessibilityAction::Focus
                    | tekes_ui_platform_macos::AccessibilityAction::Press,
            } => {
                let owner = publication.and_then(|p| p.owner(*target));
                if owner.is_some_and(|owner| owner.0 >= Self::toolbar_accessibility_owner(0) && owner.0 < Self::toolbar_accessibility_owner(self.panels.len())) {
                    let opaque = self.toolbar_ax.iter().find(|(key, _, _)| key == target).map(|(_, index, target)| (*index, *target));
                    let index = opaque.map_or(self.toolbar_index, |(index, _)| index);
                    self.panels[index].mounted_toolbar.host.semantics().iter().any(|node| node.node.id == Self::toolbar_control(ToolbarTarget::Navigation)
                        && opaque.is_some_and(|(_, target)| node.target == target || node.parts.iter().any(|(part, _)| *part == target)))
                } else if self.leading_visible && owner == Some(tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(1)) {
                    let state = state.borrow();
                    back = state.sidebar.host.semantic_mount(ComponentId(4)) == Some(target.component)
                        && state.sidebar.host.get(ComponentId(4)).is_some();
                    back || state.sidebar.host.semantics().iter().any(|node| state.sidebar.host.semantic_mount(node.id) == Some(target.component)
                        && state.sidebar.host.get(node.id).is_some_and(|n| n.component::<tekes_ui_controls::composition::KeyedSelectableListContent>().is_some()))
                } else { false }
            }
            _ => false,
        };
        let ready = state.borrow().navigation_input_ready();
        // Successful preflight is pure: this same native publication is still
        // needed by the actual action. Rejected input may update its feedback.
        if !target || ready {
            return false;
        }
        if back {
            let allowed = state.borrow_mut().prepare_back_focus();
            if !allowed {
                self.set_toolbar_focus(None, false);
            }
            !allowed
        } else {
            !self.prepare_settings_item_navigation()
        }
    }
    pub(super) fn settings_sidebar_input(
        &mut self,
        event: &ContentEvent,
    ) -> Result<Option<crate::navigation::Entry>> {
        self.panels[0]
            .local
            .as_mut()
            .unwrap()
            .event(event, &mut self.text)?;
        self.settings
            .as_ref()
            .unwrap()
            .borrow_mut()
            .take_item_navigation()
    }
    pub(super) fn open_settings(&mut self, w: &Window) -> Result<()> {
        let Some(owner) = self.settings_owner.clone() else {
            return Ok(());
        };
        // Never retain a RefCell borrow across native presentation (which can
        // synchronously deliver focus events).
        let child = owner.0.borrow().child.clone();
        if let Some(child) = child.filter(|child| !child.is_closed()) {
            let mode = owner.0.borrow().options.display_mode;
            if mode == crate::SettingsDisplayMode::Exclusive {
                w.present_exclusive(&child);
            } else {
                child.show();
            }
            return Ok(());
        }
        let retired = owner.0.borrow_mut().child.take();
        drop(retired);
        let Some(ticket) = owner.begin_load() else {
            return Ok(());
        };
        let provider = owner.0.borrow().provider.clone();
        let (tx, rx) = mpsc::channel();
        let wake = w.wake_proxy();
        std::thread::spawn(move || {
            let _ = tx.send(provider.load().map_err(|e| format!("{e:#}")));
            wake.wake();
        });
        self.settings_loading = Some(SettingsLoad {
            _ticket: ticket,
            receiver: rx,
        });
        Ok(())
    }
    /// Complete the cold child assembly before native Window creation. Both
    /// display modes use this same original-container/service binding path.
    fn create_settings_host(
        &self,
        surface: std::rc::Rc<std::cell::RefCell<crate::settings::SettingsSurface>>,
    ) -> Result<Self> {
        let columns = settings_columns(&surface);
        let app = crate::AppWindow::from_columns(columns)?
            .theme(self.theme)
            .navigation(surface.borrow().navigation_options());
        let mut host = app.into_host(false);
        host.mount_initial_containers();
        host.is_settings_window = true;
        host.settings_display = self.settings_owner.as_ref().and_then(|owner| {
            SettingsDisplayBinding::new(
                owner,
                self.settings_display
                    .as_ref()
                    .map_or(self.theme, |binding| binding.system_theme),
            )
        });
        host.acceptance_shared = self.acceptance_shared.clone();
        if self.acceptance_shared.is_some() {
            host.settings_options = self.settings_options.clone();
        }
        host.leading_visible = true;
        host.trailing_visible = false;
        host.sidebar_motion = crate::sidebar_motion::SidebarMotion::new(true, false);
        host.outer.set_ratio(0.25);
        host.settings_back = surface.borrow().exclusive;
        surface.borrow_mut().toolbar_window =
            host.panels[host.toolbar_index].mounted_toolbar.window;
        host.settings = Some(surface);
        Ok(host)
    }
    pub(super) fn poll_settings_open(&mut self, w: &Window) -> Result<()> {
        let Some(result) =
            self.settings_loading
                .as_ref()
                .and_then(|load| match load.receiver.try_recv() {
                    Ok(value) => Some(value),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        Some(Err("Settings load worker disconnected".into()))
                    }
                })
        else {
            return Ok(());
        };
        // Keep the strong ticket until native launch and owner publication finish.
        let _loading = self.settings_loading.take();
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(e) => {
                w.set_title(&format!("Settings unavailable: {e}"));
                return Ok(());
            }
        };
        let owner = self.settings_owner.as_ref().unwrap().clone();
        let (options, provider) = {
            let state = owner.0.borrow();
            (state.options.clone(), state.provider.clone())
        };
        let mode = options.display_mode;
        let mut surface = crate::settings::SettingsSurface::new(options, provider, snapshot)?;
        owner.publish_display(&surface.draft.base)?;
        surface.display = owner.0.borrow().display.clone();
        surface.show();
        let surface = std::rc::Rc::new(std::cell::RefCell::new(surface));
        let host = self.create_settings_host(surface)?;
        let config = WindowConfig {
            title: "Settings".into(),
            unified_titlebar: true,
            size: if mode == crate::SettingsDisplayMode::Exclusive {
                w.size()
            } else {
                WindowSize {
                    width: 900.,
                    height: 640.,
                }
            },
            min_size: Some(WindowSize {
                width: 600.,
                height: 360.,
            }),
            ..Default::default()
        };
        let child = if mode == crate::SettingsDisplayMode::Exclusive {
            w.open_exclusive_window(config, host)
        } else {
            w.open_window(config, host)
        };
        owner.0.borrow_mut().child = Some(child.clone());
        // Only the original finite acceptance harness needs a parent-local
        // observation handle; the shared owner is the runtime authority.
        if self.acceptance_shared.is_some() {
            self.settings_child = Some(child);
        }
        Ok(())
    }
    /// Observe the enabled control on its actual container, authenticated by the
    /// original Settings service contribution rather than the last input route.
    fn acceptance_settings_undo_target(&self) -> Option<(usize, Rect)> {
        let original = self.settings.as_ref()?.borrow().toolbar_source.clone()?;
        self.panels.iter().enumerate().find_map(|(index, panel)| {
            if panel.mounted_toolbar.source(ComponentId(92300))
                != Some(&crate::window_toolbar::Source::Undo(original.clone()))
            {
                return None;
            }
            panel
                .mounted_toolbar
                .host
                .semantics()
                .into_iter()
                .find(|node| node.node.id == ComponentId(92301))
                .into_iter()
                .flat_map(|node| node.parts)
                .find(|(_, part)| part.semantics.name == "Undo" && !part.semantics.disabled)
                .map(|(_, part)| (index, part.frame))
        })
    }
    pub(super) fn drive_settings_acceptance(&mut self, w: &mut Window) -> Result<()> {
        let Some(shared) = self.acceptance_shared.clone() else {
            return Ok(());
        };
        let stage = shared.stage();
        if matches!(
            stage,
            window_acceptance::Stage::Done | window_acceptance::Stage::Main
        ) {
            return Ok(());
        }
        if shared
            .0
            .lock()
            .is_ok_and(|s| s.phase_started.elapsed() > std::time::Duration::from_secs(10))
        {
            if stage == window_acceptance::Stage::AwaitingShown {
                let nav = self.navigation.as_ref();
                let publication = w.content_accessibility_publication();
                let text_size_targets: Vec<String> = publication
                    .as_ref()
                    .map(|p| {
                        p.nodes()
                            .iter()
                            .filter(|n| n.label == "文字大小")
                            .map(|n| {
                                format!(
                                    "label={:?},role={:?},enabled={},owner={:?}",
                                    n.label,
                                    n.role,
                                    n.enabled,
                                    p.owner(n.key).map(|owner| owner.0)
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let text_field_count = publication.as_ref().map_or(0, |p| {
                    p.nodes()
                        .iter()
                        .filter(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::TextField)
                        .count()
                });
                let text_fields: Vec<String> = publication
                    .as_ref()
                    .map(|p| {
                        p.nodes()
                            .iter()
                            .filter(|n| {
                                n.role == tekes_ui_platform_macos::AccessibilityRole::TextField
                            })
                            .take(12)
                            .map(|n| {
                                format!(
                                    "label={:?},role={:?},enabled={},owner={:?}",
                                    n.label,
                                    n.role,
                                    n.enabled,
                                    p.owner(n.key).map(|owner| owner.0)
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                eprintln!(
                    "SETTINGS_ACCEPTANCE_WAIT_TIMEOUT phase=AwaitingShown nav_present={} nav_busy={} nav_input_open={} nav_error={:?} active={} key={} occluded={} publication_present={} publication_current={} publication_valid={} publication_nodes={} text_size_targets={:?} text_field_count={} text_fields_first12={:?}",
                    nav.is_some(),
                    nav.is_some_and(|n| n.busy),
                    nav.is_some_and(|n| n.host.input_open),
                    nav.and_then(|n| n.error.as_deref()),
                    w.is_active(),
                    w.is_key_window(),
                    w.is_occluded(),
                    publication.is_some(),
                    publication.as_ref().is_some_and(|p| p.is_current()),
                    publication.as_ref().is_some_and(|p| p.is_valid()),
                    publication.as_ref().map_or(0, |p| p.nodes().len()),
                    text_size_targets,
                    text_field_count,
                    text_fields
                );
            }
            if stage == window_acceptance::Stage::Saving {
                let state = self.settings.as_ref().unwrap().borrow();
                eprintln!(
                    "SETTINGS_ACCEPTANCE_WAIT_TIMEOUT phase=Saving input_route={} dirty={} blocked={} original_source={:?} undo_target={:?}",
                    self.toolbar_index,
                    state.draft.dirty(),
                    state.blocked(),
                    state.toolbar_source,
                    self.acceptance_settings_undo_target()
                );
            }
            shared.fail(format!("10s timeout in {stage:?}"));
            w.close();
            if let Ok(s) = shared.0.lock()
                && let Some(wake) = &s.parent_wake
            {
                let _ = wake.wake();
            }
            return Ok(());
        }
        if stage == window_acceptance::Stage::Failed {
            w.close();
            if let Ok(s) = shared.0.lock()
                && let Some(wake) = &s.parent_wake
            {
                let _ = wake.wake();
            }
            return Ok(());
        }
        if shared
            .0
            .lock()
            .is_ok_and(|s| s.started.elapsed() > std::time::Duration::from_secs(30))
        {
            shared.fail("30s total acceptance timeout");
            w.close();
            return Ok(());
        }
        if let Ok(mut result) = shared.0.lock() {
            result.child_wake = Some(w.wake_proxy());
        }
        let publication = w.content_accessibility_publication();
        match stage {
            window_acceptance::Stage::AwaitingShown => {
                let options = self
                    .settings_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let field = options
                    .schema
                    .settings
                    .iter()
                    .find(|f| f.label == "文字大小")
                    .ok_or_else(|| {
                        anyhow::anyhow!("acceptance requires demo text-size field `文字大小`")
                    })?;
                let nav_ready = self
                    .navigation
                    .as_ref()
                    .is_some_and(|nav| nav.host.input_open && !nav.busy && nav.error.is_none());
                if !w.is_active() || !w.is_key_window() || w.is_occluded() || !nav_ready {
                    w.request_redraw();
                    return Ok(());
                }
                let Some(publication) = publication else {
                    w.request_redraw();
                    return Ok(());
                };
                if !publication.is_current() || !publication.is_valid() {
                    w.request_redraw();
                    return Ok(());
                }
                let owner =
                    tekes_ui_platform_macos::content_accessibility::ContentAccessibilityOwner(2);
                let Some(node) = publication.nodes().iter().find(|n| {
                    n.label == field.label
                        && n.enabled
                        && n.role == tekes_ui_platform_macos::AccessibilityRole::TextField
                        && publication.owner(n.key) == Some(owner)
                }) else {
                    w.request_redraw();
                    return Ok(());
                };
                let original = self
                    .settings
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .draft
                    .display(field);
                let current: i64 = original.parse().map_err(|_| {
                    anyhow::anyhow!("text-size AX value is not integer: {original}")
                })?;
                let edited = if current + 10 <= 300 {
                    current + 10
                } else {
                    current - 10
                };
                {
                    let mut result = shared
                        .0
                        .lock()
                        .map_err(|e| anyhow::anyhow!("acceptance lock: {e}"))?;
                    result.child_window = Some(w.native_window_number());
                    result.mode = Some(options.display_mode);
                    result.field_path = field.path.clone();
                    result.original = Some(original);
                    result.edited = Some(edited.to_string());
                    result.stage = window_acceptance::Stage::Shown;
                    result.phase_started = std::time::Instant::now();
                }
                eprintln!(
                    "SETTINGS_ACCEPTANCE_SHOWN parent_window={} child_window={} active={} key={} occluded={} ax_nodes={} mode={:?} physical_input=false",
                    shared.0.lock().unwrap().parent_window,
                    w.native_window_number(),
                    w.is_active(),
                    w.is_key_window(),
                    w.is_occluded(),
                    publication.nodes().len(),
                    options.display_mode
                );
                self.event(
                    Event::Accessibility {
                        target: node.key,
                        action: tekes_ui_platform_macos::AccessibilityAction::Focus,
                    },
                    w,
                )?;
                w.request_redraw();
            }
            window_acceptance::Stage::Shown => {
                let Some(publication) = publication else {
                    return Ok(());
                };
                let node = publication
                    .nodes()
                    .iter()
                    .find(|n| {
                        n.label == "文字大小"
                            && n.enabled
                            && n.role == tekes_ui_platform_macos::AccessibilityRole::TextField
                    })
                    .ok_or_else(|| anyhow::anyhow!("Settings AX field disappeared after Focus"))?;
                self.event(
                    Event::Accessibility {
                        target: node.key,
                        action: tekes_ui_platform_macos::AccessibilityAction::Focus,
                    },
                    w,
                )?;
                self.event(Event::TextCommand("selectAll:".into()), w)?;
                shared.transition(window_acceptance::Stage::Editing);
                w.request_redraw();
            }
            window_acceptance::Stage::Editing => {
                let value = shared
                    .0
                    .lock()
                    .unwrap()
                    .edited
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("acceptance edited value missing"))?;
                self.event(
                    Event::TextCommit {
                        text: value,
                        replacement: None,
                    },
                    w,
                )?;
                self.event(Event::TextCommand("insertNewline:".into()), w)?;
                shared.transition(window_acceptance::Stage::Saving);
                w.request_redraw();
            }
            window_acceptance::Stage::Saving => {
                let options = self
                    .settings_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let (field_path, edited) = {
                    let s = shared.0.lock().unwrap();
                    (s.field_path.clone(), s.edited.clone().unwrap())
                };
                let source = match std::fs::read_to_string(&options.file) {
                    Ok(source) => source,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        w.request_redraw();
                        return Ok(());
                    }
                    Err(e) => return Err(anyhow::anyhow!("read settings file: {e}")),
                };
                let saved = window_acceptance::read_integer_override(&source, &field_path)
                    .map_err(anyhow::Error::msg)?;
                let clean = self
                    .settings
                    .as_ref()
                    .is_some_and(|s| !s.borrow().draft.dirty() && !s.borrow().blocked());
                if saved.as_deref() == Some(edited.as_str())
                    && clean
                    && let Some((presenter, undo)) = self.acceptance_settings_undo_target()
                {
                    eprintln!(
                        "SETTINGS_ACCEPTANCE_SAVED path={} value={} draft_clean=true undo_enabled=true undo_presenter={} input_route={} original_source={:?}",
                        options.file.display(),
                        edited,
                        presenter,
                        self.toolbar_index,
                        self.settings.as_ref().unwrap().borrow().toolbar_source
                    );
                    shared.transition(window_acceptance::Stage::Undoing);
                    let point = tekes_ui_platform_macos::Point {
                        x: undo.center().x as f64,
                        y: undo.center().y as f64,
                    };
                    for kind in [
                        MouseEventKind::Down {
                            button: MouseButton::Left,
                            click_count: 1,
                        },
                        MouseEventKind::Up {
                            button: MouseButton::Left,
                            click_count: 1,
                        },
                    ] {
                        self.event(
                            Event::Mouse(tekes_ui_platform_macos::MouseEvent {
                                kind,
                                position: point,
                                modifiers: tekes_ui_platform_macos::Modifiers::default(),
                            }),
                            w,
                        )?;
                    }
                    shared.transition(window_acceptance::Stage::Restoring);
                } else {
                    // Observe the current post-save toolbar projection on the
                    // next actual paint, without relying on unrelated animation.
                    w.request_redraw();
                }
            }
            window_acceptance::Stage::Undoing => {
                w.request_redraw();
            }
            window_acceptance::Stage::Restoring => {
                let options = self
                    .settings_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("acceptance settings options absent"))?;
                let s = shared.0.lock().unwrap();
                let original = s
                    .original
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("acceptance original value missing"))?;
                let source = match std::fs::read_to_string(&options.file) {
                    Ok(source) => source,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(e) => return Err(e.into()),
                };
                let field = options
                    .schema
                    .settings
                    .iter()
                    .find(|f| f.label == "文字大小")
                    .ok_or_else(|| anyhow::anyhow!("text-size field disappeared"))?;
                let saved = window_acceptance::read_integer_override(&source, &s.field_path)
                    .map_err(anyhow::Error::msg)?
                    .unwrap_or_else(|| field.display(&field.default));
                let (form_value, edit, clean) = {
                    let settings = self.settings.as_ref().unwrap().borrow();
                    let edit = settings.form.host.semantics().iter().find_map(|node| {
                        if node.semantics.name != field.label { return None; }
                        settings.form.host.get(node.id)
                            .and_then(|n| n.component::<tekes_ui_controls::composition::NumberFieldContent>())
                            .map(|number| number.edit_state())
                    }).ok_or_else(|| anyhow::anyhow!("actual Settings NumberField absent"))?;
                    (
                        settings.draft.display(field),
                        edit,
                        !settings.draft.dirty() && !settings.blocked(),
                    )
                };
                drop(s);
                if saved == original
                    && form_value == original
                    && edit.text == format!("{original}{}", field.suffix)
                    && !edit.dirty
                    && !edit.composing
                    && clean
                {
                    eprintln!(
                        "SETTINGS_ACCEPTANCE_UNDO_RESTORED path={} value={} form_value={} field_text={:?} draft_clean=true",
                        options.file.display(),
                        saved,
                        form_value,
                        edit.text
                    );
                    let mode = shared.0.lock().unwrap().mode.unwrap();
                    if mode == crate::SettingsDisplayMode::Exclusive {
                        let back = publication
                            .as_ref()
                            .and_then(|p| {
                                p.nodes()
                                    .iter()
                                    .find(|n| n.label == "Back" || n.label == "返回")
                            })
                            .ok_or_else(|| {
                                anyhow::anyhow!("published original Settings Back action absent")
                            })?;
                        shared.transition(window_acceptance::Stage::Closing);
                        self.event(
                            Event::Accessibility {
                                target: back.key,
                                action: tekes_ui_platform_macos::AccessibilityAction::Press,
                            },
                            w,
                        )?;
                    } else {
                        shared.transition(window_acceptance::Stage::Closing);
                        self.event(Event::CloseRequested, w)?;
                    }
                    if let Some(wake) = shared.0.lock().unwrap().parent_wake.clone() {
                        let _ = wake.wake();
                    }
                } else {
                    w.request_redraw();
                }
            }
            window_acceptance::Stage::Closing => {}
            _ => {}
        }
        Ok(())
    }

    pub(super) fn draw_settings(&mut self, w: &Window) -> Result<()> {
        let progress = self.sidebar_motion.sample(
            self.motion_epoch.elapsed(),
            self.reduced_motion,
            w.is_active(),
        )?;
        let transitioning = self.sidebar_motion.needs_frame();
        let shared_settings = self.settings.as_ref().unwrap().clone();
        let mut state = shared_settings.borrow_mut();
        let settings = &mut *state;
        settings.poll(w.wake_proxy())?;
        settings.sidebar.sync_search()?;
        self.sync_settings_display(w)?;
        if !settings.open {
            w.close();
            return Ok(());
        }
        let navigation = self.navigation.as_mut().unwrap();
        if self
            .navigation_ack
            .as_ref()
            .is_some_and(|ack| ack.try_recv().is_ok())
        {
            self.navigation_ack = None;
            self.navigation_order = None;
            navigation.published()?;
        }
        navigation.poll()?;
        if self.navigation_order.is_none()
            && let Some(publication) = &navigation.publication
        {
            if let Some(target) = publication.target() {
                ensure!(
                    target.route == "settings-item",
                    "route not installed in Settings"
                );
                let item = target
                    .params
                    .get("item_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing Settings item"))?;
                settings.select(item)?;
            }
            self.navigation_order = Some(vec![]);
        }
        self.text.scale = w.scale() as f32;
        let size = w.size();
        let whole = Rect::new(0., 0., size.width as f32, size.height as f32);
        let palette = Palette::from(self.theme);
        let mut scene = Scene::new();
        let mut toolbar_scene = Scene::new();
        scene.push_quad(Quad::new(whole, palette.color("surface-panel").unwrap()));
        self.outer.min_pane = 160.;
        // Arrange now; paint only after current toolbar hit regions are known.
        let layout = self.outer_view.arrange(&mut self.outer, whole, None);
        let frames = presented_frames(
            &[layout.leading.frame(), layout.trailing.frame()],
            whole,
            progress,
            w.scale() as f32,
        );
        self.settings_frames = frames.clone();
        let renderer = self.renderer.as_mut().unwrap();

        let measured = ToolbarControl::window().row_height(&mut self.text);
        let native_center = w.set_toolbar_row_height(f64::from(measured));
        let (native, leading) = w.titlebar_safe_area();
        let (top, row_y, paint) = window_toolbar_geometry(
            native as f32,
            measured,
            native_center.map(|y| y as f32),
            w.scale() as f32,
        );
        self.settings_top = top;
        let controls_floor = native_center.map_or(0., |center| 2. * center as f32);
        if self.settings_back {
            settings.sidebar.set_back_template_icon("tekes-back")?;
        }
        let breadcrumb = settings.breadcrumb();
        let _ = navigation;
        let _ = renderer;
        let pad = tekes_ui_tokens::Spacing::Loose.value();
        let form_content = content_geometry(
            frames[1],
            settings_content_inset(
                frames[1],
                whole,
                measured,
                row_y,
                paint,
                controls_floor,
                leading as f32,
            ),
        );
        let form_content = Rect::new(
            form_content.min_x() + pad,
            form_content.min_y() + pad,
            (form_content.size.width - 2. * pad).max(1.),
            (form_content.size.height - 2. * pad).max(1.),
        );
        let width = form_content.size.width.clamp(1., 620.);
        let form_frame = Rect::new(
            form_content.min_x() + (form_content.size.width - width) * 0.5,
            form_content.min_y(),
            width,
            form_content.size.height,
        );
        let _ = settings;
        drop(state);
        self.sync_toolbar(
            w,
            whole,
            &frames,
            window_toolbar_mount::ToolbarBand {
                y: row_y,
                height: paint,
                native_leading: leading as f32,
            },
            breadcrumb,
        )?;
        for panel in &mut self.panels {
            with_content_icons(
                self.renderer.as_mut().unwrap(),
                &mut panel.mounted_toolbar.icons,
                &mut self.text,
                |text| {
                    panel
                        .mounted_toolbar
                        .host
                        .render(&mut toolbar_scene, text, palette)
                        .map_err(|e| anyhow::anyhow!("settings container toolbar: {e:?}"))
                },
            )?;
        }
        self.panels[1].bounds = frames[1];
        let panel = &mut self.panels[1];
        with_content_icons(
            self.renderer.as_mut().unwrap(),
            &mut panel.content_icons,
            &mut self.text,
            |text| {
                panel
                    .local
                    .as_mut()
                    .unwrap()
                    .render(&mut scene, text, form_frame, form_frame, palette)
            },
        )?;
        if frames[0].size.width > 32. {
            let left = content_geometry(
                frames[0],
                settings_content_inset(
                    frames[0],
                    whole,
                    measured,
                    row_y,
                    paint,
                    controls_floor,
                    leading as f32,
                ),
            );
            let rect = Rect::new(
                left.min_x() + pad,
                left.min_y() + pad,
                (left.size.width - 2. * pad).max(1.),
                (left.size.height - 2. * pad).max(1.),
            );
            self.panels[0].bounds = frames[0];
            let panel = &mut self.panels[0];
            with_content_icons(
                self.renderer.as_mut().unwrap(),
                &mut panel.content_icons,
                &mut self.text,
                |text| {
                    panel
                        .local
                        .as_mut()
                        .unwrap()
                        .render(&mut scene, text, rect, rect, palette)
                },
            )?;
        }
        let row = Rect::new(whole.min_x(), row_y, whole.size.width, paint);
        let mut toolbar_controls = self
            .panels
            .iter()
            .flat_map(|p| p.mounted_toolbar.host.content_host().pointer_regions())
            .collect::<Vec<_>>();
        if leading > 0. {
            toolbar_controls.push(Rect::new(0., row_y, leading as f32, paint));
        }
        self.outer_view
            .set_toolbar_region(Some(row), &toolbar_controls, &mut self.text);
        if self.leading_visible && !transitioning {
            self.outer_view.paint_presented_divider(
                &mut scene,
                &self.outer,
                toolbar_divider_frame(layout.divider.frame(), row_y),
                palette,
                w.scale() as f32,
            );
        }
        // The shared ToolbarHost render includes help and all original overlays.
        // Composite that complete toolbar layer above ordinary content.
        for primitive in toolbar_scene.primitives() {
            scene.push(primitive.clone());
        }

        let renderer = self.renderer.as_mut().unwrap();
        renderer.resize(w.drawable_size(), w.scale() as f32);
        renderer.render(
            &scene,
            &mut self.text.atlas,
            palette.color("surface-panel").unwrap(),
        )?;
        if self.navigation_order.is_some() && self.navigation_ack.is_none() {
            let (tx, rx) = mpsc::sync_channel(1);
            self.completion
                .as_ref()
                .unwrap()
                .try_send(vec![tx])
                .map_err(|_| anyhow::anyhow!("Settings GPU completion queue full"))?;
            self.navigation_ack = Some(rx);
        }
        let input_open = self
            .navigation
            .as_ref()
            .is_some_and(|n| n.host.input_open && !n.busy)
            && !shared_settings.borrow().restoring();
        let active = w.is_active();
        w.set_content_accessibility_publication(
            self.content_accessibility_publication(active && input_open),
        );
        w.set_text_input_state(if active && input_open {
            crate::columns::text_state(
                self.panels[self.toolbar_index]
                    .mounted_toolbar
                    .host
                    .content_host(),
            )
            .or_else(|| {
                (!self.panels[self.toolbar_index].mounted_toolbar.modal()
                    && self.panels[self.toolbar_index]
                        .mounted_toolbar
                        .host
                        .content_host()
                        .focused()
                        .is_none())
                .then(|| self.settings.as_ref().unwrap().borrow().text_state())
                .flatten()
            })
        } else {
            None
        });
        let resize_regions = if self.leading_visible && !transitioning {
            self.outer_view.resize_regions(&self.outer)
        } else {
            vec![]
        };
        w.set_cursor_regions(
            resize_regions
                .into_iter()
                .map(|(r, _)| tekes_ui_platform_macos::CursorRegion {
                    origin: tekes_ui_platform_macos::Point {
                        x: r.min_x().into(),
                        y: r.min_y().into(),
                    },
                    size: WindowSize {
                        width: r.size.width.into(),
                        height: r.size.height.into(),
                    },
                    icon: tekes_ui_platform_macos::CursorIcon::ResizeHorizontal,
                })
                .collect(),
        );
        let animate = (self.acceptance_shared.as_ref().is_some_and(|s| {
            !matches!(
                s.stage(),
                window_acceptance::Stage::Done | window_acceptance::Stage::Failed
            )
        }) || self.sidebar_motion.needs_frame()
            || self.panels[self.toolbar_index]
                .mounted_toolbar
                .host
                .needs_render()
            || self.text.animation_requested()
            || self.settings.as_ref().unwrap().borrow().needs_frame())
            && active;
        if !w.set_animating(animate) && animate {
            w.request_redraw();
        }
        Ok(())
    }

    fn settings_shell_event(&mut self, event: &Event, w: &Window) -> Result<bool> {
        let Event::Mouse(mouse) = event else {
            return Ok(false);
        };
        if !w.is_active() {
            return Ok(true);
        }
        let p = Point::new(mouse.position.x as f32, mouse.position.y as f32);
        let handled = match mouse.kind {
            MouseEventKind::Down {
                button: MouseButton::Left,
                click_count,
            } => {
                if self.leading_visible
                    && !self.sidebar_motion.needs_frame()
                    && self.outer_view.pointer_down(p, &mut self.outer)
                {
                    self.drag_split = Some(0);
                    true
                } else if p.y < self.settings_top {
                    if click_count == 2 {
                        w.try_titlebar_double_click();
                    } else {
                        w.try_begin_titlebar_drag();
                    }
                    true
                } else {
                    false
                }
            }
            MouseEventKind::Up {
                button: MouseButton::Left,
                ..
            } if self.drag_split.take().is_some() => {
                self.outer_view.pointer_up(&mut self.outer);
                true
            }
            MouseEventKind::Dragged {
                button: MouseButton::Left,
            } if self.drag_split == Some(0) => {
                self.outer_view.pointer_move(p, &mut self.outer);
                true
            }
            _ => false,
        };
        if handled {
            w.request_redraw();
        }
        Ok(handled)
    }
    pub(super) fn settings_event(&mut self, event: &Event, w: &mut Window) -> Result<bool> {
        if self
            .settings
            .as_ref()
            .is_some_and(|s| s.borrow().restoring())
            && matches!(
                event,
                Event::Mouse(_)
                    | Event::Key(_)
                    | Event::Accessibility { .. }
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextCommand(_)
            )
        {
            return Ok(true);
        }
        if self.settings_shell_event(event, w)? {
            return Ok(true);
        }
        if self
            .navigation
            .as_ref()
            .is_some_and(|n| n.busy || !n.host.input_open)
            && matches!(
                event,
                Event::Mouse(_)
                    | Event::Key(_)
                    | Event::Accessibility { .. }
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextCommand(_)
            )
        {
            return Ok(true);
        }
        if let Event::TextKey(tekes_ui_platform_macos::TextKeyEvent::Finished { context, .. }) =
            event
            && let Some(ControlKey::Tab { reverse }) = toolbar_key(context.key.clone()).key
            && !context.key.modifiers.command
            && !context.key.modifiers.control
            && !context.key.modifiers.alt
            && self.open_toolbar().is_none()
        {
            let event = window_toolbar_mount::native_toolbar_event(event).unwrap();
            self.settings_scope_tab(reverse, &event)?;
            w.set_text_input_state(self.settings.as_ref().unwrap().borrow().text_state());
            w.request_redraw();
            return Ok(true);
        }
        if let Event::TextCommand(command) = event
            && matches!(command.as_str(), "insertTab:" | "insertBacktab:")
            && self.open_toolbar().is_none()
        {
            self.settings_scope_tab(
                command == "insertBacktab:",
                &ContentEvent::TextCommand(command.clone()),
            )?;
            w.set_text_input_state(self.settings.as_ref().unwrap().borrow().text_state());
            w.request_redraw();
            return Ok(true);
        }
        let cancel_search = matches!(event, Event::Key(key)
            if key.kind == KeyEventKind::Down && key.key_code == 53)
            || matches!(event, Event::TextCommand(command) if command == "cancelOperation:");
        if cancel_search
            && self
                .settings
                .as_ref()
                .is_some_and(|state| state.borrow().sidebar.host.focused() == Some(ComponentId(6)))
            && self
                .settings
                .as_ref()
                .unwrap()
                .borrow_mut()
                .sidebar
                .cancel_search()?
        {
            w.set_text_input_state(None);
            w.request_redraw();
            return Ok(true);
        }
        // Header search is an original local editor, using the same text/IME
        // and clipboard adapter as every ordinary Container, not the form draft.
        let sidebar_editor = self
            .settings
            .as_ref()
            .is_some_and(|s| s.borrow().sidebar.host.focused_text().is_some());
        if sidebar_editor
            && matches!(
                event,
                Event::Key(_)
                    | Event::TextKey(_)
                    | Event::TextCommit { .. }
                    | Event::TextPreedit { .. }
                    | Event::TextUnmark
                    | Event::TextCommand(_)
            )
        {
            self.focused = 0;
            if self.local_content_event(event, w)? {
                self.settings
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .sidebar
                    .sync_search()?;
                self.draw(w)?;
                return Ok(true);
            }
        }
        let shared_settings = self.settings.as_ref().unwrap().clone();
        let mut state = shared_settings.borrow_mut();
        let settings = &mut *state;
        let mut intents = vec![];
        let mut routed = None;
        let mut sidebar_routed = None;
        match event {
            Event::Mouse(m) if w.is_active() => {
                let p = Point::new(m.position.x as f32, m.position.y as f32);
                if matches!(m.kind, MouseEventKind::Down { .. }) {
                    let sidebar = self.leading_visible
                        && self.settings_frames.first().is_some_and(|r| r.contains(p));
                    self.settings_pointer = Some(sidebar);
                    if sidebar {
                        settings.form.host.focus(None, false);
                    } else {
                        settings.sidebar.host.focus(None, false);
                    }
                }
                if self.settings_pointer == Some(true) {
                    sidebar_routed = Some(toolbar_pointer(*m));
                } else {
                    routed = Some(toolbar_pointer(*m));
                }
                if matches!(m.kind, MouseEventKind::Up { .. }) {
                    self.settings_pointer = None;
                }
            }
            Event::Scroll(s) if w.is_active() => {
                let scroll = ContentEvent::Scroll {
                    position: Point::new(s.position.x as f32, s.position.y as f32),
                    delta: Point::new(-s.delta.x as f32, -s.delta.y as f32),
                    precise: s.precise,
                    begin: false,
                    end: false,
                    momentum: false,
                    time: self.motion_epoch.elapsed().as_secs_f64(),
                };
                if self.leading_visible
                    && self.settings_frames.first().is_some_and(|r| {
                        r.contains(Point::new(s.position.x as f32, s.position.y as f32))
                    })
                {
                    sidebar_routed = Some(scroll);
                } else {
                    routed = Some(scroll);
                }
            }
            Event::Key(key)
                if w.is_active()
                    && settings.sidebar.host.focused().is_some()
                    && !key.modifiers.command
                    && key.key_code != 53 =>
            {
                sidebar_routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
            }
            Event::Key(key) if w.is_active() => {
                let composing = settings
                    .form
                    .host
                    .focused_text()
                    .is_some_and(|s| s.input.marked.is_some());
                if key.kind == KeyEventKind::Down
                    && key.modifiers.command
                    && settings.form.host.focused_text().is_some()
                {
                    let command = match key
                        .characters_ignoring_modifiers
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "a" => Some("selectAll:"),
                        "z" => Some(if key.modifiers.shift {
                            "redo:"
                        } else {
                            "undo:"
                        }),
                        _ => None,
                    };
                    if key.characters_ignoring_modifiers == "z"
                        && settings.form.host.focused_text().is_none()
                    {
                        if let Some(source) = settings.toolbar_source.clone() {
                            settings.undo_action(
                                if key.modifiers.shift {
                                    crate::settings_undo::REDO
                                } else {
                                    crate::settings_undo::UNDO
                                },
                                &source,
                            )?;
                        }
                    } else if let Some(command) = command {
                        routed = Some(ContentEvent::TextCommand(command.into()));
                    } else if key.characters_ignoring_modifiers == "c"
                        || key.characters_ignoring_modifiers == "x"
                    {
                        if let Some(surface) = settings.form.host.focused_text()
                            && !surface.input.selection().is_empty()
                        {
                            w.set_clipboard_text(&surface.input.selected_text());
                            if key.characters_ignoring_modifiers == "x" {
                                routed = Some(ContentEvent::TextCommand("deleteBackward:".into()));
                            }
                        }
                    } else if key.characters_ignoring_modifiers == "v" {
                        if let Some(text) = w.clipboard_text() {
                            routed = Some(ContentEvent::Commit {
                                text,
                                replacement: None,
                            });
                        }
                    } else {
                        routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
                    }
                } else if key.kind == KeyEventKind::Down
                    && key.key_code == 53
                    && !composing
                    && !settings.form.has_open_menu()
                    && !settings.form.number_editing()
                {
                    intents.push(FormIntent::Cancel);
                } else {
                    routed = Some(ContentEvent::KeyInput(toolbar_key(key.clone())));
                }
            }
            Event::TextKey(marker) => {
                routed = window_toolbar_mount::native_toolbar_event(&Event::TextKey(marker.clone()))
            }
            Event::TextCommit { text, replacement } => {
                routed = Some(ContentEvent::Commit {
                    text: text.clone(),
                    replacement: replacement.clone(),
                })
            }
            Event::TextPreedit {
                text,
                selection,
                clauses,
                replacement,
            } => {
                routed = Some(ContentEvent::Preedit {
                    text: text.clone(),
                    selection: selection.clone(),
                    clauses: clauses.clone(),
                    replacement: replacement.clone(),
                })
            }
            Event::TextUnmark => routed = Some(ContentEvent::Unmark),
            Event::TextCommand(command)
                if command == "cancelOperation:"
                    && !settings.form.number_editing()
                    && !settings.form.has_open_menu()
                    && settings
                        .form
                        .host
                        .focused_text()
                        .is_none_or(|s| s.input.marked.is_none()) =>
            {
                intents.push(FormIntent::Cancel)
            }
            Event::TextCommand(command) => {
                if command == "copy:" || command == "cut:" {
                    if let Some(surface) = settings.form.host.focused_text()
                        && !surface.input.selection().is_empty()
                    {
                        w.set_clipboard_text(&surface.input.selected_text());
                        if command == "cut:" {
                            routed = Some(ContentEvent::TextCommand("deleteBackward:".into()));
                        }
                    }
                } else if command == "paste:" {
                    if let Some(text) = w.clipboard_text() {
                        routed = Some(ContentEvent::Commit {
                            text,
                            replacement: None,
                        });
                    }
                } else {
                    routed = Some(ContentEvent::TextCommand(command.clone()));
                }
            }
            Event::Accessibility { target, action } => {
                let Some(publication) = w.content_accessibility_publication() else {
                    return Ok(false);
                };
                let Some(owner) = publication.owner(*target) else {
                    return Ok(false);
                };
                if !settings.accessibility_input(
                    &publication,
                    owner,
                    *target,
                    action.clone(),
                    self.leading_visible,
                ) {
                    return Ok(true);
                }
                // Do not revoke any participating Host until publication.dispatch
                // has delivered successfully to its stamped original owner.
                if matches!(
                    action,
                    tekes_ui_platform_macos::AccessibilityAction::Focus
                        | tekes_ui_platform_macos::AccessibilityAction::Press
                ) {
                    self.set_toolbar_focus(None, false);
                    self.focused = if owner.0 == 1 { 0 } else { 1 };
                }
                if owner.0 == 1 {
                    if let Some(entry) = settings.take_item_navigation()? {
                        if self
                            .navigation
                            .as_ref()
                            .is_some_and(|n| n.host.input_open && !n.busy)
                        {
                            self.navigation.as_mut().unwrap().send(
                                crate::navigation::Command::Navigate(
                                    crate::navigation::Action::Push(entry),
                                ),
                            )?;
                        }
                    } else if settings.sidebar.take_back() {
                        settings.close_requested();
                    }
                } else {
                    intents.extend(
                        settings
                            .form
                            .collect(&settings.view_schema, &mut settings.draft),
                    );
                }
            }
            Event::Focus(false) => {
                settings.form.host.cancel();
                settings.flush(w.wake_proxy())?;
                w.set_content_accessibility_publication(None);
                w.set_text_input_state(None);
                return Ok(false);
            }
            Event::QuitRequested => {
                settings.close_requested();
                settings.intents(vec![], w.wake_proxy())?;
                w.request_redraw();
                return Ok(true);
            }
            Event::CloseRequested => {
                if settings.close_requested() {
                    w.close();
                } else {
                    settings.intents(vec![], w.wake_proxy())?;
                }
                w.request_redraw();
                return Ok(true);
            }
            Event::CaretBlink => {
                w.request_redraw();
                return Ok(true);
            }
            Event::Created
            | Event::Redraw
            | Event::Resized { .. }
            | Event::Wake
            | Event::AnimationFrame(_)
            | Event::Focus(true)
            | Event::SystemMotionChanged(_) => return Ok(false),
            // No input or toolbar AX action may leak behind the Settings surface.
            _ => return Ok(true),
        }
        if let Some(event) = sidebar_routed {
            let _ = settings;
            drop(state);
            let entry = self.settings_sidebar_input(&event)?;
            state = shared_settings.borrow_mut();
            if let Some(entry) = entry {
                self.navigation
                    .as_mut()
                    .unwrap()
                    .send(crate::navigation::Command::Navigate(
                        crate::navigation::Action::Push(entry),
                    ))?;
            }
        }
        if state.sidebar.take_back() {
            state.close_requested();
        }
        if let Some(event) = routed {
            drop(state);
            intents.extend(self.settings_form_input(&event)?.0);
            state = shared_settings.borrow_mut();
        }
        let settings = &mut *state;
        settings.edited();
        settings.intents(intents, w.wake_proxy())?;
        if !settings.open {
            w.close();
        }
        w.set_text_input_state(if settings.open {
            settings.text_state()
        } else {
            None
        });
        w.request_redraw();
        Ok(true)
    }
}

#[cfg(test)]
mod settings_geometry_tests {
    use super::*;

    #[test]
    fn settings_columns_start_after_visible_toolbar_and_native_control_floor() {
        let window = Rect::new(0., 0., 900., 640.);
        let (allocation, row_y, row_height) = window_toolbar_geometry(66., 42., Some(24.), 2.);
        let controls_floor = 48.;
        let sidebar = Rect::new(0., 0., 220., 640.);
        let form = Rect::new(220., 0., 680., 640.);

        assert_eq!(allocation, 66.);
        assert_eq!(
            settings_content_inset(
                sidebar,
                window,
                row_height,
                row_y,
                row_height,
                controls_floor,
                80.,
            ),
            controls_floor,
            "the sidebar clears native titlebar controls"
        );
        assert_eq!(
            settings_content_inset(
                form,
                window,
                row_height,
                row_y,
                row_height,
                controls_floor,
                80.,
            ),
            controls_floor,
            "both columns share the main AppHost control-safe boundary"
        );
    }
}

#[cfg(test)]
mod settings_theme_radio_width_tests {
    use super::*;
    use tekes_ui_controls::{TextSurfaceHost, composition::RadioGroupContent};

    #[test]
    fn actual_core_text_theme_words_fit_their_slots_in_a_620_point_form() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../tekes-ui-gallery/src/gallery/apphost.settings.toml"
        ))
        .unwrap()
        .for_item("general/appearance")
        .unwrap();
        let field = schema
            .settings
            .iter()
            .find(|field| field.presentation == tekes_ui_settings::FieldPresentation::Radio)
            .unwrap();

        for scale in [1.0, 1.5, 3.0] {
            let mut text = crate::text::Text::new();
            text.set_body_size(tekes_ui_tokens::proposal::TextRole::UiBody.style().size * scale);
            let mut form = tekes_ui_settings::SettingsForm::new(
                &schema,
                &tekes_ui_settings::Draft::new(tekes_ui_settings::Snapshot::default()),
            )
            .unwrap();
            let frame = Rect::new(0., 0., 620., 500.);
            form.host
                .render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();

            let radio = form
                .host
                .get(ComponentId(100))
                .unwrap()
                .component::<RadioGroupContent>()
                .unwrap();
            let layout = radio.view().layout();
            assert_eq!(layout.labels.len(), field.options.len());
            for (index, option) in field.options.iter().enumerate() {
                let natural = text
                    .shape_label(
                        &option.label,
                        tekes_ui_tokens::proposal::TextRole::UiBody,
                        frame.size.width,
                        Default::default(),
                    )
                    .unwrap()
                    .paragraph()
                    .lines
                    .iter()
                    .map(|line| line.width_trimmed as f32)
                    .fold(0., f32::max);
                assert!(
                    layout.labels[index].size.width + 1. / 64. >= natural,
                    "at {:.0}% type scale, option {:?} needs {natural:.2}pt but its label slot is {:.2}pt",
                    scale * 100.,
                    option.label,
                    layout.labels[index].size.width
                );
            }
        }
    }
}

#[cfg(test)]
mod pinned_header_tests {
    use super::*;
    use tekes_ui_controls::composition::{ListContent, TextContent, TextFieldContent};

    // Same original Text + prepared IconImage lease adapter used by the
    // AppHost text/toolbar tests. Production supplies SvgIconCache's GPU service
    // through with_content_icons; this headless fixture validates catalog names.
    pub(super) struct HeaderIcons;
    impl tekes_ui_controls::layout_view::IconRenderer for HeaderIcons {
        fn prepare_icon(
            &mut self,
            name: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> std::result::Result<
            tekes_ui_controls::composition::IconImage,
            tekes_ui_controls::composition::IconRenderError,
        > {
            assert!(tekes_ui_icons::Glyph::named(name).is_some());
            Ok(tekes_ui_controls::composition::IconImage::new(
                ImageId(99),
                std::rc::Rc::new(()),
            ))
        }
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
            panic!("original Settings icons must retain their prepared image lease");
        }
    }
    pub(super) fn render_header(
        sidebar: &mut tekes_ui_settings::SettingsSidebar,
        text: &mut crate::text::Text,
        frame: Rect,
    ) -> Scene {
        let mut scene = Scene::new();
        sidebar
            .render(
                &mut scene,
                &mut tekes_ui_controls::composition::WithIconRenderer::new(text, &mut HeaderIcons),
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        scene
    }
    #[test]
    fn header_search_expands_in_place_and_keeps_original_editor_with_scrolling_list() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../../examples/app-host-demo/settings.schema.toml"
        ))
        .unwrap();
        let mut sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema).unwrap();
        sidebar.set_back_icon(tekes_ui_scene::ImageId(777));
        let mut text = crate::text::Text::new();
        let frame = Rect::new(0., 0., 260., 180.);
        let draw = |s: &mut tekes_ui_settings::SettingsSidebar, t: &mut crate::text::Text| {
            render_header(s, t, frame);
        };
        draw(&mut sidebar, &mut text);
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        assert!(sidebar.host.get(ComponentId(6)).is_none());
        let chip = sidebar.host.get(ComponentId(10)).unwrap().frame();
        let scroll = sidebar.host.get(ComponentId(1)).unwrap().frame();
        assert_eq!(back.min_x(), scroll.min_x());
        assert_eq!(back.center().y, chip.center().y);
        sidebar.event(&ContentEvent::PointerDown(chip.center()));
        sidebar.event(&ContentEvent::PointerUp(chip.center()));
        render_header(&mut sidebar, &mut text, Rect::new(12., 16., 760., 180.));
        let wide_editor = sidebar.host.get(ComponentId(6)).unwrap().frame();
        assert_eq!(
            wide_editor,
            sidebar.host.get(ComponentId(5)).unwrap().frame()
        );
        assert!(sidebar.host.get(ComponentId(4)).is_none());
        draw(&mut sidebar, &mut text);
        let search = sidebar.host.get(ComponentId(6)).unwrap().frame();
        let scroll = sidebar.host.get(ComponentId(1)).unwrap().frame();
        assert_eq!(sidebar.host.focused(), Some(ComponentId(6)));
        assert!(sidebar.host.get(ComponentId(10)).is_none());
        assert!(sidebar.host.get(ComponentId(4)).is_none());
        assert_eq!(search, sidebar.host.get(ComponentId(5)).unwrap().frame());
        assert!(search.max_y() < scroll.min_y());
        assert!(
            sidebar
                .host
                .get(ComponentId(2))
                .unwrap()
                .component::<ListContent>()
                .is_some()
        );
        let list = sidebar
            .host
            .get(ComponentId(101))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap();
        let items = list.semantic_parts();
        assert!(!items.is_empty());
        assert_eq!(
            items[0].frame.size.height,
            tekes_ui_tokens::ControlSize::RowHeight.value()
        );
        if items.len() > 1 {
            assert_eq!(
                items[1].frame.min_y() - items[0].frame.max_y(),
                tekes_ui_tokens::Spacing::Micro.value()
            );
        }
        sidebar
            .host
            .event_with_text(&ContentEvent::PointerDown(search.center()), &mut text)
            .unwrap();
        sidebar
            .host
            .event_with_text(&ContentEvent::PointerUp(search.center()), &mut text)
            .unwrap();
        sidebar
            .host
            .event_with_text(
                &ContentEvent::Commit {
                    text: "文字大小".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        sidebar
            .host
            .event_with_text(
                &ContentEvent::Scroll {
                    position: scroll.center(),
                    delta: Point::new(0., 100.),
                    precise: true,
                    begin: true,
                    end: false,
                    momentum: false,
                    time: 0.,
                },
                &mut text,
            )
            .unwrap();
        draw(&mut sidebar, &mut text);
        assert_eq!(sidebar.host.get(ComponentId(6)).unwrap().frame(), search);
        assert_eq!(
            sidebar
                .host
                .get(ComponentId(6))
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text,
            "文字大小"
        );
        assert!(sidebar.cancel_search().unwrap());
        draw(&mut sidebar, &mut text);
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        assert!(
            sidebar
                .host
                .get(ComponentId(4))
                .unwrap()
                .component::<tekes_ui_controls::Button>()
                .unwrap()
                .label
                .is_empty()
        );
        sidebar.event(&ContentEvent::PointerDown(back.center()));
        sidebar.event(&ContentEvent::PointerUp(back.center()));
        assert!(sidebar.take_back());
        assert!(!sidebar.take_back());
    }

    #[test]
    fn search_matches_field_routes_and_escape_restores_rows_without_losing_selection() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../tekes-ui-settings/tests/fixtures/layout.schema.toml"
        ))
        .unwrap();
        let mut sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema).unwrap();
        sidebar.set_back_template_icon("tekes-back").unwrap();
        sidebar.select("reading/text");
        let mut text = crate::text::Text::new();
        let frame = Rect::new(0., 0., 260., 600.);
        let draw = |s: &mut tekes_ui_settings::SettingsSidebar, t: &mut crate::text::Text| {
            render_header(s, t, frame);
        };
        draw(&mut sidebar, &mut text);
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        let arrow = sidebar.host.get(ComponentId(8)).unwrap().frame();
        let scroll = sidebar.host.get(ComponentId(1)).unwrap().frame();
        assert_eq!(arrow.min_x(), scroll.min_x());
        assert_eq!(arrow.center().y, back.center().y);
        assert!(
            back.size.height
                >= tekes_ui_tokens::proposal::metric("size", "size-action-button").unwrap()
        );
        let chip = sidebar.host.get(ComponentId(10)).unwrap().frame().center();
        sidebar.event(&ContentEvent::PointerDown(chip));
        sidebar.event(&ContentEvent::PointerUp(chip));
        draw(&mut sidebar, &mut text);
        sidebar
            .host
            .event_with_text(
                &ContentEvent::Commit {
                    text: "行距倍数".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        draw(&mut sidebar, &mut text);
        let reading = sidebar
            .host
            .get(ComponentId(103))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap();
        assert_eq!(
            reading
                .semantic_parts()
                .iter()
                .map(|part| part.semantics.name.as_str())
                .collect::<Vec<_>>(),
            ["高级排版"]
        );
        assert_eq!(reading.selected_key(), None);
        let target = reading.semantic_parts()[0].frame.center();
        assert_eq!(
            sidebar.event(&ContentEvent::PointerDown(target)),
            Some("reading/advanced".into())
        );
        sidebar.event(&ContentEvent::PointerUp(target));
        // The original List emits selection on pointer down, and its visible
        // index must map to advanced rather than the unfiltered text route.
        sidebar.host.focus(Some(ComponentId(103)), true);
        assert_eq!(
            sidebar.event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true,
            }),
            Some("reading/advanced".into())
        );
        // Escape while the original editor owns focus restores keyboard focus.
        sidebar.host.focus(Some(ComponentId(6)), true);
        assert!(sidebar.cancel_search().unwrap());
        let collapsed = render_header(&mut sidebar, &mut text, frame);
        let chip = sidebar.host.get(ComponentId(10)).unwrap();
        let button = chip.component::<tekes_ui_controls::Button>().unwrap();
        assert!(
            button.button.state.focused && button.button.state.focus_visible,
            "search keeps logical and keyboard focus after Escape"
        );
        let back = sidebar.host.get(ComponentId(4)).unwrap().frame();
        let arrow = sidebar.host.get(ComponentId(8)).unwrap().frame();
        assert_eq!(chip.frame().center().y, back.center().y);
        assert_eq!(
            arrow.min_x(),
            sidebar.host.get(ComponentId(1)).unwrap().frame().min_x()
        );
        let focus_ink = Palette::from(Theme::LIGHT).color("border-focused").unwrap();
        assert!(
            !collapsed
                .primitives()
                .iter()
                .any(|primitive| matches!(primitive,
                tekes_ui_scene::Primitive::Quad(quad)
                    if quad.border_width > 0. && quad.border_color == focus_ink)),
            "the restored search icon must not paint a clipped focus border"
        );
        let reading = sidebar
            .host
            .get(ComponentId(103))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap();
        assert_eq!(
            reading
                .semantic_parts()
                .iter()
                .map(|part| part.semantics.name.as_str())
                .collect::<Vec<_>>(),
            ["文本与语言", "高级排版"]
        );
        assert_eq!(
            reading.selected_key(),
            Some(tekes_ui_controls::extended::ListItemKey(2))
        );
        assert!(sidebar.host.get(ComponentId(6)).is_none());
        assert_eq!(sidebar.host.focused(), Some(ComponentId(10)));
        assert!(!sidebar.take_back());
        let chip = sidebar.host.get(ComponentId(10)).unwrap().frame().center();
        sidebar.event(&ContentEvent::PointerDown(chip));
        sidebar.event(&ContentEvent::PointerUp(chip));
        draw(&mut sidebar, &mut text);
        sidebar
            .host
            .event_with_text(
                &ContentEvent::Preedit {
                    text: "没有这样的设置".into(),
                    selection: 0..0,
                    clauses: vec![],
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        assert!(
            !sidebar.cancel_search().unwrap(),
            "IME owns cancellation while marked"
        );
        sidebar.sync_search().unwrap();
        assert_eq!(
            sidebar
                .host
                .get(ComponentId(103))
                .unwrap()
                .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                .unwrap()
                .model()
                .display_indices()
                .len(),
            2
        );
        sidebar
            .host
            .event_with_text(&ContentEvent::Unmark, &mut text)
            .unwrap();
        draw(&mut sidebar, &mut text);
        assert_eq!(
            sidebar
                .host
                .get(ComponentId(103))
                .unwrap()
                .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                .unwrap()
                .model()
                .display_indices()
                .len(),
            0
        );
        assert!(sidebar.cancel_search().unwrap());
        draw(&mut sidebar, &mut text);
        assert_eq!(
            sidebar
                .host
                .get(ComponentId(103))
                .unwrap()
                .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                .unwrap()
                .model()
                .display_indices()
                .len(),
            2
        );
    }

    #[test]
    fn nonempty_search_blur_preserves_result_action_selection_and_focus() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../tekes-ui-settings/tests/fixtures/layout.schema.toml"
        ))
        .unwrap();
        let mut sidebar = tekes_ui_settings::SettingsSidebar::new_with_back(&schema).unwrap();
        sidebar.select("reading/text");
        let mut text = crate::text::Text::new();
        let frame = Rect::new(0., 0., 260., 600.);
        render_header(&mut sidebar, &mut text, frame);
        let chip = sidebar.host.get(ComponentId(10)).unwrap().frame().center();
        sidebar.event(&ContentEvent::PointerDown(chip));
        sidebar.event(&ContentEvent::PointerUp(chip));
        sidebar
            .host
            .event_with_text(
                &ContentEvent::Commit {
                    text: "行距倍数".into(),
                    replacement: None,
                },
                &mut text,
            )
            .unwrap();
        render_header(&mut sidebar, &mut text, frame);
        let result = sidebar
            .host
            .get(ComponentId(103))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap()
            .semantic_parts()[0]
            .frame
            .center();
        let route = sidebar.event(&ContentEvent::PointerDown(result)).unwrap();
        assert_eq!(route, "reading/advanced");
        sidebar.select(&route);
        render_header(&mut sidebar, &mut text, frame);
        sidebar.event(&ContentEvent::PointerUp(result));
        assert!(sidebar.host.get(ComponentId(6)).is_none());
        assert!(sidebar.host.get(ComponentId(4)).is_some());
        assert!(sidebar.host.get(ComponentId(10)).is_some());
        assert_eq!(sidebar.host.focused(), Some(ComponentId(103)));
        let list = sidebar
            .host
            .get(ComponentId(103))
            .unwrap()
            .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
            .unwrap();
        assert_eq!(list.semantic_parts().len(), 2);
        assert_eq!(
            list.selected_key(),
            Some(tekes_ui_controls::extended::ListItemKey(3))
        );
    }

    #[test]
    fn group_headers_match_gallery_caption_and_spacing_hierarchy() {
        let schema = tekes_ui_settings::Schema::parse(include_str!(
            "../../tekes-ui-settings/tests/fixtures/layout.schema.toml"
        ))
        .unwrap();
        let mut sidebar = tekes_ui_settings::SettingsSidebar::new(&schema).unwrap();
        sidebar
            .render(
                &mut Scene::new(),
                &mut crate::text::Text::new(),
                Rect::new(0., 0., 260., 500.),
                Theme::LIGHT.into(),
            )
            .unwrap();

        let first_heading = sidebar.host.get(ComponentId(100)).unwrap();
        let first_list = sidebar.host.get(ComponentId(101)).unwrap();
        let second_heading = sidebar.host.get(ComponentId(102)).unwrap();
        let second_list = sidebar.host.get(ComponentId(103)).unwrap();
        assert_eq!(
            first_heading.component::<TextContent>().unwrap().role,
            tekes_ui_tokens::proposal::TextRole::UiCaption
        );
        assert_eq!(
            second_heading.component::<TextContent>().unwrap().role,
            tekes_ui_tokens::proposal::TextRole::UiCaption
        );
        assert_eq!(
            first_list.frame().min_y() - first_heading.frame().max_y(),
            tekes_ui_tokens::Spacing::Regular.value()
        );
        assert_eq!(
            second_heading.frame().min_y() - first_list.frame().max_y(),
            tekes_ui_tokens::Spacing::Micro.value()
        );
        assert!(
            second_list
                .component::<tekes_ui_controls::composition::KeyedSelectableListContent>()
                .is_some()
        );
    }
}

#[cfg(test)]
mod navigation_input_tests {
    use super::*;
    use crate::settings::{SettingsDisplayMode, SettingsSurface, SettingsWindow};
    use crate::toolbar_source::ComponentSource;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use tekes_ui_controls::composition::KeyedSelectableListContent;
    use tekes_ui_controls::composition::{
        AccessibilityAction as ContentAction, ToolbarButtonDeclaration, ToolbarGroupOverflow,
        ToolbarPlacement, ToolbarRegion,
    };
    use tekes_ui_platform_macos::{
        AccessibilityAction, Modifiers, content_accessibility::ContentAccessibilityOwner,
    };
    use tekes_ui_settings::{FileProvider, Snapshot};
    fn fixture(marked: bool) -> (Host, std::path::PathBuf) {
        fixture_mode(marked, SettingsDisplayMode::Independent)
    }
    fn fixture_mode(marked: bool, mode: SettingsDisplayMode) -> (Host, std::path::PathBuf) {
        static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let path = std::env::temp_dir()
            .join(format!(
                "settings-nav-input-{}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let schema=include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml").replace("min = 10\nmax = 32","min = 10\nmax = 32\nstep = 1")
            .replace("[[groups.items]]\nid = \"advanced\"", "[[groups.items.fields]]\nid = \"extra-number\"\npath = [\"reading\", \"extra_number\"]\nlabel = \"Extra number\"\ntype = \"integer\"\ndefault = 14\nmin = 10\nmax = 32\nstep = 1\n\n[[groups.items]]\nid = \"advanced\"");
        let options = SettingsWindow::from_toml(&schema, path.clone())
            .unwrap()
            .display_mode(mode);
        let provider = FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
        let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        if !marked {
            assert!(surface.prepare_item_navigation());
            surface.select("reading/text").unwrap();
        }
        surface.show();
        let state = std::rc::Rc::new(std::cell::RefCell::new(surface));
        let mut host = crate::AppWindow::from_columns(settings_columns(&state))
            .unwrap()
            .into_host(false);
        host.settings = Some(state);
        host.is_settings_window = true;
        host.leading_visible = true;
        assert!(host.mount_columns());
        let source = ComponentSource {
            session: tekes_ui_module_runtime::component::Session {
                module: crate::navigation::MODULE_ID.into(),
                instance: 1,
                host_epoch: 1,
            },
            registration: 1,
            window: host.panels[host.toolbar_index].mounted_toolbar.window,
        };
        host.panels[host.toolbar_index]
            .mounted_toolbar
            .sync(
                Rect::new(0., 0., 940., 600.),
                vec![ToolbarRegion::new(
                    ComponentId(10),
                    Rect::new(0., 0., 280., 52.),
                )],
                vec![Declaration {
                    id: ComponentId(92100),
                    region: ComponentId(10),
                    placement: ToolbarPlacement::Navigation,
                    order: 0,
                    contents: Contents::Commands(
                        vec![
                            ToolbarButtonDeclaration {
                                id: crate::navigation::BACK,
                                icon: None,
                                text: "Back".into(),
                                hint: None,
                                enabled: true,
                            },
                            ToolbarButtonDeclaration {
                                id: crate::navigation::FORWARD,
                                icon: None,
                                text: "Forward".into(),
                                hint: None,
                                enabled: true,
                            },
                        ],
                        ToolbarGroupOverflow::Split,
                    ),
                    source: Source::Navigation(source),
                }],
            )
            .unwrap();
        render(&mut host);
        {
            let mut s = host.settings.as_ref().unwrap().borrow_mut();
            let id = ComponentId(if marked { 100 } else { 101 });
            assert!(s.form.host.focus(Some(id), true));
            if marked {
                s.form.host.event(&ContentEvent::Preedit {
                    text: "中文".into(),
                    selection: 0..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: None,
                });
            } else {
                s.form
                    .host
                    .event(&ContentEvent::TextCommand("selectAll:".into()));
                s.form.host.event(&ContentEvent::Commit {
                    text: "99".into(),
                    replacement: None,
                });
            }
        }
        render(&mut host);
        (host, path)
    }
    fn render(host: &mut Host) {
        for (i, frame) in [
            Rect::new(16., 60., 260., 500.),
            Rect::new(300., 60., 620., 500.),
        ]
        .into_iter()
        .enumerate()
        {
            host.panels[i].bounds = frame;
            host.panels[i]
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut tekes_ui_controls::composition::WithIconRenderer::new(
                        &mut host.text,
                        &mut super::pinned_header_tests::HeaderIcons,
                    ),
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        host.panels[host.toolbar_index]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
    }
    fn correct(host: &mut Host, marked: bool) {
        let mut s = host.settings.as_ref().unwrap().borrow_mut();
        if marked {
            s.form.host.event(&ContentEvent::Unmark);
        } else {
            s.form
                .host
                .event(&ContentEvent::TextCommand("selectAll:".into()));
            s.form.host.event(&ContentEvent::Commit {
                text: "18".into(),
                replacement: None,
            });
        }
        let state = &mut *s;
        state.form.collect(&state.view_schema, &mut state.draft);
        state.edited();
        drop(s);
        render(host);
    }
    fn row(host: &Host, marked: bool) -> (ComponentId, Point, String) {
        let label = if marked {
            "文本与语言"
        } else {
            "高级排版"
        };
        let state = host.settings.as_ref().unwrap().borrow();
        for node in state.sidebar.host.semantics() {
            if let Some(list) = state
                .sidebar
                .host
                .get(node.id)
                .and_then(|n| n.component::<KeyedSelectableListContent>())
                && let Some(part) = list
                    .semantic_parts()
                    .into_iter()
                    .find(|part| part.semantics.name == label)
            {
                let point = part.frame.center();
                return (node.id, point, label.into());
            }
        }
        panic!("original sidebar route missing");
    }
    fn key(code: u16) -> Event {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Down,
            key_code: code,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
            modifiers: Modifiers::NONE,
            is_repeat: false,
        })
    }
    fn assert_retained(
        host: &Host,
        selected: &str,
        text: &str,
        marked: Option<std::ops::Range<usize>>,
    ) {
        let state = host.settings.as_ref().unwrap().borrow();
        assert_eq!(state.selected, selected);
        assert_eq!(state.form.host.focused_text().unwrap().input.text, text);
        assert_eq!(state.form.host.focused_text().unwrap().input.marked, marked);
        assert!(state.form.has_number_draft() || state.form.host.is_composing());
    }
    #[test]
    fn actual_sidebar_pointer_keyboard_and_ax_refuse_then_accept_corrected_input() {
        for marked in [false, true] {
            for channel in ["pointer", "key", "ax"] {
                let (mut host, path) = fixture(marked);
                let (list, point, label) = row(&host, marked);
                if channel == "key" {
                    assert!(
                        host.settings
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .sidebar
                            .host
                            .focus(Some(list), true)
                    );
                    render(&mut host);
                }
                let publication = host.content_accessibility_publication(true).unwrap();
                let target = publication
                    .nodes()
                    .iter()
                    .find(|n| n.label == label)
                    .unwrap()
                    .key;
                let event = match channel {
                    "pointer" => Event::Mouse(MouseEvent {
                        kind: MouseEventKind::Down {
                            button: MouseButton::Left,
                            click_count: 1,
                        },
                        position: tekes_ui_platform_macos::Point {
                            x: point.x as f64,
                            y: point.y as f64,
                        },
                        modifiers: Modifiers::NONE,
                    }),
                    "key" => key(if marked { 36 } else { 125 }),
                    _ => Event::Accessibility {
                        target,
                        action: AccessibilityAction::Press,
                    },
                };
                let (selected, text, composition) = {
                    let s = host.settings.as_ref().unwrap().borrow();
                    (
                        s.selected.clone(),
                        s.form.host.focused_text().unwrap().input.text.clone(),
                        s.form.host.focused_text().unwrap().input.marked.clone(),
                    )
                };
                assert!(
                    host.settings_navigation_focus_guard(&event, Some(&publication)),
                    "{channel} must refuse before focus/unmark"
                );
                assert_retained(&host, &selected, &text, composition.clone());
                if channel == "ax" {
                    let mut state = host.settings.as_ref().unwrap().borrow_mut();
                    let focused = state.sidebar.host.focused();
                    assert!(
                        !state.accessibility_input(
                            &publication,
                            ContentAccessibilityOwner(1),
                            target,
                            AccessibilityAction::Press,
                            true
                        ),
                        "stale/refused publication cannot dispatch"
                    );
                    assert_eq!(
                        state.sidebar.host.focused(),
                        focused,
                        "refusal must not transfer keyboard ownership"
                    );
                }
                assert_retained(&host, &selected, &text, composition);
                assert!(
                    host.settings
                        .as_ref()
                        .unwrap()
                        .borrow_mut()
                        .take_item_navigation()
                        .unwrap()
                        .is_none()
                );
                correct(&mut host, marked);
                let publication = host.content_accessibility_publication(true).unwrap();
                assert!(publication.is_current());
                assert!(
                    !host.settings_navigation_focus_guard(&event, Some(&publication)),
                    "corrected {channel} must remain usable"
                );
                assert!(
                    publication.is_current(),
                    "allowed preflight must preserve the same multi-Host AX publication"
                );
                let entry=match &event {
                Event::Accessibility{target,action}=>{let mut state=host.settings.as_ref().unwrap().borrow_mut();assert!(state.accessibility_input(&publication,ContentAccessibilityOwner(1),*target,action.clone(),true));assert_eq!(state.sidebar.host.focused(),Some(list),"successful List Press keeps original keyboard owner");assert_eq!(state.form.host.focused(),None,"old form focus retires only after successful AX delivery");state.take_item_navigation().unwrap()},
                _=>host.settings_sidebar_input(&crate::window::window_toolbar_mount::native_toolbar_event(&event).unwrap()).unwrap(),
            }.unwrap_or_else(|| panic!("original sidebar must produce its real settings-item Entry: marked={marked} channel={channel}"));
                let expected = if marked {
                    "reading/text"
                } else {
                    "reading/advanced"
                };
                assert_eq!(entry.route, "settings-item");
                assert_eq!(entry.params["item_id"], expected);
                host.settings
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .select(expected)
                    .unwrap();
                assert_eq!(host.settings.as_ref().unwrap().borrow().selected, expected);
                assert!(
                    host.settings.as_ref().unwrap().borrow().draft.dirty(),
                    "corrected edits survive item publication"
                );
                assert!(!path.exists());
                drop(host);
                std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn navigation_preflight_uses_actual_toolbar_after_another_container_owned_input() {
        for marked in [false, true] {
            let (mut host, path) = fixture(marked);
            let publication = host.content_accessibility_publication(true).unwrap();
            let node = publication
                .nodes()
                .iter()
                .find(|node| node.label == "Back")
                .unwrap();
            let point = node.frame.origin;
            let event = Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down {
                    button: MouseButton::Left,
                    click_count: 1,
                },
                position: point,
                modifiers: Modifiers::NONE,
            });
            let (selected, text, composition) = {
                let state = host.settings.as_ref().unwrap().borrow();
                (
                    state.selected.clone(),
                    state.form.host.focused_text().unwrap().input.text.clone(),
                    state.form.host.focused_text().unwrap().input.marked.clone(),
                )
            };
            host.toolbar_index = 1;
            assert!(host.settings_navigation_focus_guard(&event, Some(&publication)));
            assert_eq!(
                host.toolbar_index, 1,
                "refusal precedes route/focus transfer"
            );
            assert_retained(&host, &selected, &text, composition);
            correct(&mut host, marked);
            assert!(!host.settings_navigation_focus_guard(&event, None));
            drop(host);
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }
    #[test]
    fn saving_driver_observes_the_original_enabled_undo_in_center_while_input_route_is_leading() {
        let (mut host, path) = fixture(false);
        let source = ComponentSource {
            session: tekes_ui_module_runtime::component::Session {
                module: crate::settings_undo::MODULE_ID.into(),
                instance: 40,
                host_epoch: 2,
            },
            registration: 5,
            window: host.panels[0].mounted_toolbar.window,
        };
        host.settings.as_ref().unwrap().borrow_mut().toolbar_source = Some(source.clone());
        let mut declaration = Declaration {
            id: ComponentId(92300),
            region: ComponentId(93000),
            placement: ToolbarPlacement::SecondaryAction,
            order: 0,
            contents: Contents::Commands(
                vec![ToolbarButtonDeclaration {
                    id: crate::settings_undo::UNDO,
                    icon: None,
                    text: "Undo".into(),
                    hint: None,
                    enabled: true,
                }],
                ToolbarGroupOverflow::Together,
            ),
            source: Source::Undo(source.clone()),
        };
        let mount = |host: &mut Host, declaration: Declaration| {
            host.panels[1]
                .mounted_toolbar
                .sync(
                    Rect::new(0., 0., 1000., 600.),
                    vec![ToolbarRegion::new(
                        ComponentId(93000),
                        Rect::new(300., 0., 620., 48.),
                    )],
                    vec![declaration],
                )
                .unwrap();
            host.panels[1]
                .mounted_toolbar
                .host
                .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
                .unwrap();
        };
        mount(&mut host, declaration.clone());
        host.toolbar_index = 0;
        let (presenter, frame) = host
            .acceptance_settings_undo_target()
            .expect("original center Undo is independent of the input-route index");
        assert_eq!(presenter, 1);
        assert!(frame.min_x() >= 300.);
        assert_eq!(host.toolbar_index, 0);
        let old = host.panels[1]
            .mounted_toolbar
            .host
            .semantics()
            .into_iter()
            .flat_map(|node| node.parts)
            .find(|(_, part)| part.semantics.name == "Undo")
            .unwrap()
            .0;
        if let Contents::Commands(buttons, _) = &mut declaration.contents {
            buttons[0].enabled = false;
        }
        mount(&mut host, declaration.clone());
        assert!(host.acceptance_settings_undo_target().is_none());
        assert!(
            !host.panels[1]
                .mounted_toolbar
                .host
                .accessibility_action(old, ContentAction::Press)
                .consumed
        );
        if let Contents::Commands(buttons, _) = &mut declaration.contents {
            buttons[0].enabled = true;
        }
        let mut replacement = source.clone();
        replacement.registration += 1;
        declaration.source = Source::Undo(replacement.clone());
        mount(&mut host, declaration);
        assert!(
            host.acceptance_settings_undo_target().is_none(),
            "another source cannot satisfy the original Settings locator"
        );
        host.settings.as_ref().unwrap().borrow_mut().toolbar_source = Some(replacement);
        assert_eq!(host.acceptance_settings_undo_target().unwrap().0, 1);
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn original_back_forward_producers_preserve_invalid_or_marked_then_allow_corrected_commands() {
        for marked in [false, true] {
            for name in ["Back", "Forward"] {
                let (mut host, path) = fixture(marked);
                let publication = host.content_accessibility_publication(true).unwrap();
                let key = publication
                    .nodes()
                    .iter()
                    .find(|n| n.label == name)
                    .unwrap()
                    .key;
                let event = Event::Accessibility {
                    target: key,
                    action: AccessibilityAction::Press,
                };
                let (selected, text, composition) = {
                    let s = host.settings.as_ref().unwrap().borrow();
                    (
                        s.selected.clone(),
                        s.form.host.focused_text().unwrap().input.text.clone(),
                        s.form.host.focused_text().unwrap().input.marked.clone(),
                    )
                };
                assert!(host.settings_navigation_focus_guard(&event, Some(&publication)));
                assert_retained(&host, &selected, &text, composition);
                assert!(
                    host.panels[host.toolbar_index]
                        .mounted_toolbar
                        .host
                        .drain_commands()
                        .is_empty(),
                    "refusal creates no transport request payload"
                );
                correct(&mut host, marked);
                let publication = host.content_accessibility_publication(true).unwrap();
                assert!(publication.is_current());
                assert!(!host.settings_navigation_focus_guard(&event, Some(&publication)));
                assert!(
                    publication.is_current(),
                    "allowed toolbar AX preflight must not revoke form geometry"
                );
                let target = host
                    .toolbar_ax
                    .iter()
                    .find(|(k, _, _)| *k == key)
                    .unwrap()
                    .2;
                assert!(
                    host.panels[host.toolbar_index]
                        .mounted_toolbar
                        .host
                        .accessibility_action(target, ContentAction::Press)
                        .consumed
                );
                let invocations = host.panels[host.toolbar_index]
                    .mounted_toolbar
                    .host
                    .drain_commands();
                assert_eq!(invocations.len(), 1);
                assert_eq!(
                    invocations[0].command,
                    if name == "Back" {
                        crate::navigation::BACK
                    } else {
                        crate::navigation::FORWARD
                    }
                );
                assert!(matches!(&invocations[0].source, Source::Navigation(_)));
                assert!(
                    host.prepare_settings_item_navigation(),
                    "same production pre-RPC admission accepts corrected input"
                );
                assert!(host.settings.as_ref().unwrap().borrow().draft.dirty());
                assert!(!path.exists());
                drop(host);
                std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn refused_navigation_keeps_other_committed_fields_scheduled_with_visible_feedback() {
        let (mut host, path) = fixture(false);
        {
            let mut state = host.settings.as_ref().unwrap().borrow_mut();
            assert!(state.form.host.focus(Some(ComponentId(102)), true));
            state
                .form
                .host
                .event(&ContentEvent::TextCommand("selectAll:".into()));
            state.form.host.event(&ContentEvent::Commit {
                text: "18".into(),
                replacement: None,
            });
            assert!(!state.draft.dirty());
        }
        assert!(!host.prepare_settings_item_navigation());
        {
            let state = host.settings.as_ref().unwrap().borrow();
            let field = state
                .schema
                .settings
                .iter()
                .find(|f| f.id.ends_with("/extra-number"))
                .unwrap();
            assert_eq!(state.draft.display(field), "18");
            assert!(state.draft.dirty());
            assert!(
                state.needs_frame(),
                "valid partial commits keep original autosave scheduling"
            );
            assert!(
                state
                    .form
                    .host
                    .get(ComponentId(2))
                    .unwrap()
                    .component::<tekes_ui_controls::composition::TextContent>()
                    .unwrap()
                    .text
                    .contains("修正数值")
            );
        }
        assert!(!path.exists());
        drop(host);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn original_back_rejects_before_focus_then_activates_original_close_barrier() {
        for marked in [false, true] {
            for channel in ["pointer", "key", "ax-focus", "ax-press"] {
                let (mut host, path) = fixture_mode(marked, SettingsDisplayMode::Exclusive);
                if channel == "key" {
                    assert!(
                        host.settings
                            .as_ref()
                            .unwrap()
                            .borrow_mut()
                            .sidebar
                            .host
                            .focus(Some(ComponentId(4)), true)
                    );
                    render(&mut host);
                }
                let (point, mount) = {
                    let s = host.settings.as_ref().unwrap().borrow();
                    (
                        s.sidebar.host.get(ComponentId(4)).unwrap().frame().center(),
                        s.sidebar.host.semantic_mount(ComponentId(4)).unwrap(),
                    )
                };
                let publication = host.content_accessibility_publication(true).unwrap();
                let target = publication
                    .nodes()
                    .iter()
                    .find(|n| n.key.component == mount && n.key.part.is_none())
                    .unwrap()
                    .key;
                let event = match channel {
                    "pointer" => Event::Mouse(MouseEvent {
                        kind: MouseEventKind::Down {
                            button: MouseButton::Left,
                            click_count: 1,
                        },
                        position: tekes_ui_platform_macos::Point {
                            x: point.x as f64,
                            y: point.y as f64,
                        },
                        modifiers: Modifiers::NONE,
                    }),
                    "key" => key(49),
                    _ => Event::Accessibility {
                        target,
                        action: if channel == "ax-focus" {
                            AccessibilityAction::Focus
                        } else {
                            AccessibilityAction::Press
                        },
                    },
                };
                let (selected, text, composition) = {
                    let s = host.settings.as_ref().unwrap().borrow();
                    (
                        s.selected.clone(),
                        s.form.host.focused_text().unwrap().input.text.clone(),
                        s.form.host.focused_text().unwrap().input.marked.clone(),
                    )
                };
                assert!(
                    host.settings_navigation_focus_guard(&event, Some(&publication)),
                    "Back {channel} must preserve invalid/marked input before focus"
                );
                assert_retained(&host, &selected, &text, composition);
                {
                    let mut s = host.settings.as_ref().unwrap().borrow_mut();
                    assert!(s.open);
                    assert!(!s.sidebar.take_back());
                }
                correct(&mut host, marked);
                let publication = host.content_accessibility_publication(true).unwrap();
                assert!(!host.settings_navigation_focus_guard(&event, Some(&publication)));
                assert!(
                    publication.is_current(),
                    "accepted Back focus preflight is read-only"
                );
                match &event {
                    Event::Accessibility { target, action } => {
                        {
                            let mut s = host.settings.as_ref().unwrap().borrow_mut();
                            assert!(s.accessibility_input(
                                &publication,
                                ContentAccessibilityOwner(1),
                                *target,
                                action.clone(),
                                true
                            ));
                            assert!(
                                s.take_item_navigation().unwrap().is_none(),
                                "Back AX uses the original action consumption boundary without an item request"
                            );
                            assert!(s.open);
                            if channel == "ax-focus" {
                                assert!(!s.sidebar.take_back(), "AX Focus does not activate Back");
                            }
                        }
                        if channel == "ax-focus" {
                            // A completed focus is a real input change; the next native
                            // Press uses the subsequent ordinary rendered publication.
                            render(&mut host);
                            let next = host.content_accessibility_publication(true).unwrap();
                            let press = Event::Accessibility {
                                target: *target,
                                action: AccessibilityAction::Press,
                            };
                            assert!(!host.settings_navigation_focus_guard(&press, Some(&next)));
                            assert!(next.is_current());
                            assert!(
                                host.settings
                                    .as_ref()
                                    .unwrap()
                                    .borrow_mut()
                                    .accessibility_input(
                                        &next,
                                        ContentAccessibilityOwner(1),
                                        *target,
                                        AccessibilityAction::Press,
                                        true
                                    )
                            );
                            assert!(
                                host.settings
                                    .as_ref()
                                    .unwrap()
                                    .borrow_mut()
                                    .take_item_navigation()
                                    .unwrap()
                                    .is_none()
                            );
                        }
                    }
                    _ => {
                        assert!(
                            host.settings_sidebar_input(
                                &crate::window::window_toolbar_mount::native_toolbar_event(&event)
                                    .unwrap()
                            )
                            .unwrap()
                            .is_none(),
                            "Back never creates an item request"
                        );
                        {
                            let mut s = host.settings.as_ref().unwrap().borrow_mut();
                            assert!(s.open);
                            assert!(!s.sidebar.take_back(), "down does not activate or close");
                        }
                        let up = if channel == "pointer" {
                            Event::Mouse(MouseEvent {
                                kind: MouseEventKind::Up {
                                    button: MouseButton::Left,
                                    click_count: 1,
                                },
                                position: tekes_ui_platform_macos::Point {
                                    x: point.x as f64,
                                    y: point.y as f64,
                                },
                                modifiers: Modifiers::NONE,
                            })
                        } else {
                            let Event::Key(mut k) = key(49) else {
                                unreachable!()
                            };
                            k.kind = KeyEventKind::Up;
                            Event::Key(k)
                        };
                        assert!(
                            host.settings_sidebar_input(
                                &crate::window::window_toolbar_mount::native_toolbar_event(&up)
                                    .unwrap()
                            )
                            .unwrap()
                            .is_none()
                        );
                    }
                }
                {
                    let mut s = host.settings.as_ref().unwrap().borrow_mut();
                    assert!(
                        s.open,
                        "activation leaves close policy to original consumer"
                    );
                    assert!(
                        s.sidebar.take_back(),
                        "original Back producer activates: marked={marked} channel={channel}"
                    );
                    assert!(
                        !s.close_requested(),
                        "corrected edits await actual persistence, not an early focus-close"
                    );
                    assert!(s.open);
                    assert!(s.draft.dirty());
                }
                assert!(!path.exists());
                drop(host);
                std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod cold_settings_host_tests {
    use super::*;
    use crate::{SettingsDisplayMode, SettingsWindow, settings::SettingsSurface};
    use std::{cell::RefCell, rc::Rc};
    use tekes_ui_settings::{FileProvider, Snapshot};
    #[test]
    fn production_cold_settings_assembly_binds_original_containers_once_in_both_display_modes() {
        for mode in [
            SettingsDisplayMode::Exclusive,
            SettingsDisplayMode::Independent,
        ] {
            let path = std::env::temp_dir()
                .join(format!(
                    "settings-cold-host-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
                .join("settings.toml");
            let options = SettingsWindow::from_toml(
                include_str!("../../tekes-ui-settings/tests/fixtures/layout.schema.toml"),
                path.clone(),
            )
            .unwrap()
            .display_mode(mode);
            let provider =
                FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
            let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
            surface.show();
            let surface = Rc::new(RefCell::new(surface));
            let original = {
                let mut state = surface.borrow_mut();
                [
                    &mut state.sidebar.host as *mut ContentHost as usize,
                    &mut state.form.host as *mut ContentHost as usize,
                ]
            };
            let parent =
                Host::new(vec!["parent".into()], false, None).with_theme(Theme::DARK_INCREASED);
            // This is the exact production helper called by poll_settings_open;
            // no test-side mount or native/GPU initialization masks cold state.
            let mut child = parent.create_settings_host(surface.clone()).unwrap();
            assert_eq!(
                child.palette(),
                parent.palette(),
                "Settings inherits the selected Theme"
            );
            assert!(child.is_settings_window);
            assert_eq!(child.settings_back, mode == SettingsDisplayMode::Exclusive);
            assert!(Rc::ptr_eq(child.settings.as_ref().unwrap(), &surface));
            assert_eq!(child.panels.len(), 2);
            assert!(child.column_definitions.is_none());
            assert!(child.renderer.is_none());
            assert!(child.navigation.is_none());
            assert!(child.navigation_options.is_some());
            let identities: Vec<_> = child
                .panels
                .iter()
                .map(|panel| (panel.instance, panel.mounted_toolbar.window))
                .collect();
            let binding = surface.borrow().toolbar_window;
            assert_eq!(binding, identities[0].1);
            child.mount_initial_containers();
            assert_eq!(
                child
                    .panels
                    .iter()
                    .map(|panel| (panel.instance, panel.mounted_toolbar.window))
                    .collect::<Vec<_>>(),
                identities
            );
            assert_eq!(surface.borrow().toolbar_window, binding);
            for (index, panel) in child.panels.iter_mut().enumerate() {
                assert!(panel.pid.is_none());
                assert!(panel.launch_tx.is_none());
                panel.local.as_mut().unwrap().with_host(&mut |host| {
                    assert_eq!(host as *mut ContentHost as usize, original[index])
                });
            }
            drop(child);
            drop(parent);
            drop(surface);
            if path.parent().unwrap().exists() {
                std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn cold_fixed_process_profile_mount_does_not_start_or_replace_its_original_instances() {
        let mut host = Host::new(vec!["first".into(), "second".into()], false, None);
        host.mount_initial_containers();
        let identities: Vec<_> = host
            .panels
            .iter()
            .map(|panel| (panel.instance, panel.mounted_toolbar.window))
            .collect();
        host.mount_initial_containers();
        assert_eq!(
            host.panels
                .iter()
                .map(|panel| (panel.instance, panel.mounted_toolbar.window))
                .collect::<Vec<_>>(),
            identities
        );
        assert!(host.panels.iter().all(|panel| panel.local.is_none()
            && panel.launch_tx.is_some()
            && panel.pid.is_none()));
    }
}

#[cfg(test)]
mod display_preferences_tests {
    use super::*;
    use crate::{SettingsOwner, SettingsWindow, settings::SettingsConfiguration};
    use std::rc::Rc;
    use tekes_ui_settings::{Draft, Snapshot, Submission};

    #[test]
    fn loaded_saved_and_undo_snapshots_update_original_hosts_and_independent_settings() {
        let path = std::env::temp_dir()
            .join(format!(
                "settings-display-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let owner = SettingsOwner::new(
            SettingsWindow::from_toml(
                include_str!("../../tekes-ui-gallery/src/gallery/apphost.settings.toml"),
                path.clone(),
            )
            .unwrap(),
        )
        .unwrap()
        .with_display_preferences(
            "general/appearance/appearance",
            "general/appearance/text-size",
        )
        .unwrap();
        let provider = owner.0.borrow().provider.clone();
        let schema = owner.0.borrow().options.schema.clone();
        let initial = provider.load().unwrap();
        let mut draft = Draft::new(initial.clone());
        draft.edit(&schema.settings[0], "dark".into());
        draft.edit(&schema.settings[1], "120".into());
        let saved = provider
            .commit(&draft.submit(&schema, "display-save".into()).unwrap())
            .unwrap();
        let editor = crate::LocalContainer::new(
            "editor",
            tekes_ui_controls::composition::Content::new(
                ComponentId(1),
                tekes_ui_controls::composition::TextFieldContent::new("Title", "Unsubmitted text"),
            ),
        )
        .unwrap();
        let mut first =
            Host::from_columns(vec![editor.into()], false, None).with_theme(Theme::LIGHT_INCREASED);
        first.mount_initial_containers();
        let mut input_before = None;
        first.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert!(host.focus(Some(ComponentId(1)), true));
                host.event(&ContentEvent::Preedit {
                    text: "中文".into(),
                    selection: 0..2,
                    clauses: std::iter::once(0..2).collect(),
                    replacement: None,
                });
                let input = &host.focused_text().unwrap().input;
                input_before = Some((input.text.clone(), input.marked.clone()));
            });
        let mut second = Host::new(vec![], false, None).with_theme(Theme::LIGHT);
        first
            .configure_settings(Some(SettingsConfiguration::Shared(owner.clone())))
            .unwrap();
        second
            .configure_settings(Some(SettingsConfiguration::Shared(owner.clone())))
            .unwrap();
        // Same admission called by the startup worker completion, before any
        // Settings window opens. These are actual on-disk effective values.
        owner.publish_display(&provider.load().unwrap()).unwrap();
        assert!(
            first
                .settings_display
                .as_mut()
                .unwrap()
                .apply(&mut first.theme, &mut first.text)
        );
        assert!(
            second
                .settings_display
                .as_mut()
                .unwrap()
                .apply(&mut second.theme, &mut second.text)
        );
        first.panels[0]
            .local
            .as_mut()
            .unwrap()
            .with_host(&mut |host| {
                assert_eq!(host.focused(), Some(ComponentId(1)));
                assert!(
                    host.is_composing(),
                    "style changes preserve original composition"
                );
                let input = &host.focused_text().unwrap().input;
                assert_eq!(
                    (input.text.clone(), input.marked.clone()),
                    input_before.clone().unwrap()
                );
            });
        assert_eq!(first.theme, Theme::DARK_INCREASED);
        assert_eq!(second.theme, Theme::DARK);
        assert!((first.text.type_scale() - 1.2).abs() < 1e-6);
        assert!((second.text.type_scale() - 1.2).abs() < 1e-6);
        owner.publish_display(&initial).unwrap(); // Real late-startup-read scenario.
        assert!(
            !first
                .settings_display
                .as_mut()
                .unwrap()
                .apply(&mut first.theme, &mut first.text)
        );

        let mut surface = crate::settings::SettingsSurface::new(
            owner.0.borrow().options.clone(),
            provider.clone(),
            saved.clone(),
        )
        .unwrap();
        surface.display = owner.0.borrow().display.clone();
        let shared = Rc::new(std::cell::RefCell::new(surface));
        let mut child = first.create_settings_host(shared.clone()).unwrap();
        assert!(child.settings_owner.is_none());
        child
            .settings_display
            .as_mut()
            .unwrap()
            .apply(&mut child.theme, &mut child.text);
        assert_eq!(child.theme, Theme::DARK_INCREASED);
        // Undo's provider receipt advances revision and restores effective defaults.
        let target = Snapshot {
            revision: saved.revision + 1,
            last_commit_id: "display-undo".into(),
            overrides: initial.overrides,
        };
        let undone = provider
            .commit(&Submission::replacement(&saved, &target).unwrap())
            .unwrap();
        crate::settings::SettingsDisplayState::publish(
            shared.borrow().display.as_ref().unwrap(),
            &undone,
        )
        .unwrap();
        first.settings_display.as_mut().unwrap().system_theme = Theme::LIGHT_INCREASED;
        second.settings_display.as_mut().unwrap().system_theme = Theme::DARK;
        child.settings_display.as_mut().unwrap().system_theme = Theme::DARK_INCREASED;
        first
            .settings_display
            .as_mut()
            .unwrap()
            .apply(&mut first.theme, &mut first.text);
        second
            .settings_display
            .as_mut()
            .unwrap()
            .apply(&mut second.theme, &mut second.text);
        child
            .settings_display
            .as_mut()
            .unwrap()
            .apply(&mut child.theme, &mut child.text);
        assert_eq!(first.theme, Theme::LIGHT_INCREASED);
        assert_eq!(second.theme, Theme::DARK);
        assert_eq!(child.theme, Theme::DARK_INCREASED);
        assert_eq!(first.text.type_scale(), 1.);
        assert_eq!(child.text.type_scale(), 1.);
        drop(first);
        drop(second);
        drop(owner);
        // Independent Settings retains display values, not its original owner.
        assert!(
            child
                .settings_display
                .as_ref()
                .unwrap()
                .owner
                .upgrade()
                .is_none()
        );
        let redo = Snapshot {
            revision: undone.revision + 1,
            last_commit_id: "display-redo".into(),
            overrides: saved.overrides,
        };
        let redone = provider
            .commit(&Submission::replacement(&undone, &redo).unwrap())
            .unwrap();
        crate::settings::SettingsDisplayState::publish(
            shared.borrow().display.as_ref().unwrap(),
            &redone,
        )
        .unwrap();
        assert!(
            child
                .settings_display
                .as_mut()
                .unwrap()
                .apply(&mut child.theme, &mut child.text)
        );
        assert!((child.text.type_scale() - 1.2).abs() < 1e-6);
        drop(child);
        drop(shared);
        drop(provider);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn display_binding_rejects_unrelated_fields_and_invalid_loaded_values() {
        let path = std::env::temp_dir()
            .join(format!(
                "settings-display-invalid-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(
            include_str!("../../tekes-ui-gallery/src/gallery/apphost.settings.toml"),
            path.clone(),
        )
        .unwrap();
        let field = options
            .schema
            .settings
            .iter()
            .find(|f| f.id == "general/appearance/appearance")
            .unwrap();
        assert_eq!(
            field.presentation,
            tekes_ui_settings::FieldPresentation::Radio
        );
        let form_schema = options.schema.for_item("general/appearance").unwrap();
        let form = tekes_ui_settings::SettingsForm::new(
            &form_schema,
            &tekes_ui_settings::Draft::new(Snapshot::default()),
        )
        .unwrap();
        assert!(
            form.host
                .get(ComponentId(100))
                .unwrap()
                .component::<tekes_ui_controls::composition::RadioGroupContent>()
                .is_some()
        );
        let owner = SettingsOwner::new(options).unwrap();
        let wrong_kind = owner
            .clone()
            .with_display_preferences(
                "general/appearance/text-size",
                "general/appearance/appearance",
            )
            .unwrap_err();
        assert!(
            wrong_kind
                .to_string()
                .contains("appearance preference requires system/light/dark enum values")
        );
        let short_id = owner
            .clone()
            .with_display_preferences("appearance", "text-size")
            .unwrap_err();
        assert!(
            short_id
                .to_string()
                .contains("unknown display preference appearance")
        );
        let owner = owner
            .with_display_preferences(
                "general/appearance/appearance",
                "general/appearance/text-size",
            )
            .unwrap();
        let mut invalid = Snapshot::default();
        let mut values = tekes_ui_settings::Value::Table(Default::default());
        values.as_table_mut().unwrap().insert(
            "appearance".into(),
            tekes_ui_settings::Value::String("unknown".into()),
        );
        invalid.overrides.insert("general".into(), values);
        assert!(owner.publish_display(&invalid).is_err());
        assert!(
            owner
                .0
                .borrow()
                .display
                .as_ref()
                .unwrap()
                .borrow()
                .preferences
                .is_none()
        );
        drop(owner);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[cfg(test)]
mod form_keyboard_owner_tests {
    use super::*;
    use crate::window_toolbar::{Contents, Declaration, Source};
    use crate::{SettingsDisplayMode, SettingsWindow, settings::SettingsSurface};
    use std::{cell::RefCell, rc::Rc};
    use tekes_ui_controls::composition::{
        RadioGroupContent, ToolbarButtonDeclaration, ToolbarGroupOverflow, ToolbarPlacement,
        ToolbarRegion,
    };
    use tekes_ui_platform_macos::Modifiers;
    use tekes_ui_settings::{FileProvider, Snapshot};

    fn key(code: u16, down: bool) -> ContentKeyEvent {
        toolbar_key(KeyEvent {
            kind: if down {
                KeyEventKind::Down
            } else {
                KeyEventKind::Up
            },
            key_code: code,
            modifiers: Modifiers::NONE,
            is_repeat: false,
            characters: if code == 48 { "\t" } else { " " }.into(),
            characters_ignoring_modifiers: if code == 48 { "\t" } else { " " }.into(),
        })
    }

    fn fixture() -> (Host, Rc<RefCell<SettingsSurface>>, std::path::PathBuf) {
        fixture_source(include_str!(
            "../../tekes-ui-gallery/src/gallery/apphost.settings.toml"
        ))
    }
    fn fixture_source(source: &str) -> (Host, Rc<RefCell<SettingsSurface>>, std::path::PathBuf) {
        static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let fixture_id = NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!(
                "settings-keyboard-owner-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                fixture_id
            ))
            .join("settings.toml");
        let options = SettingsWindow::from_toml(source, path.clone())
            .unwrap()
            .display_mode(SettingsDisplayMode::Independent);
        let provider = FileProvider::open(path.clone(), options.schema.namespace.clone()).unwrap();
        let mut surface = SettingsSurface::new(options, provider, Snapshot::default()).unwrap();
        surface.show();
        let surface = Rc::new(RefCell::new(surface));
        let parent = Host::new(vec![], false, None);
        let mut host = parent.create_settings_host(surface.clone()).unwrap();
        let frames = [
            Rect::new(0., 42., 225., 598.),
            Rect::new(225., 42., 675., 598.),
        ];
        for (panel, frame) in host.panels.iter_mut().zip(frames) {
            panel.bounds = frame;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        // Mount the real visibility contribution used by production Settings.
        // Focus is acquired by the original Tab producer, never assigned here.
        host.panels[0]
            .mounted_toolbar
            .sync(
                Rect::new(0., 0., 900., 640.),
                vec![ToolbarRegion::new(
                    ComponentId(91020),
                    Rect::new(0., 0., 225., 42.),
                )],
                vec![Declaration {
                    id: Host::toolbar_group(ToolbarTarget::Visibility(0)),
                    region: ComponentId(91020),
                    placement: ToolbarPlacement::Navigation,
                    order: 0,
                    contents: Contents::Commands(
                        vec![ToolbarButtonDeclaration {
                            id: 0x5601,
                            icon: None,
                            text: "Hide leading panel".into(),
                            hint: None,
                            enabled: true,
                        }],
                        ToolbarGroupOverflow::Together,
                    ),
                    source: Source::Visibility(0),
                }],
            )
            .unwrap();
        host.panels[0]
            .mounted_toolbar
            .host
            .render(&mut Scene::new(), &mut host.text, Theme::LIGHT.into())
            .unwrap();
        (host, surface, path)
    }

    fn render_original_panels(host: &mut Host) {
        for panel in &mut host.panels {
            let frame = panel.bounds;
            panel
                .local
                .as_mut()
                .unwrap()
                .render(
                    &mut Scene::new(),
                    &mut host.text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
    }

    #[test]
    fn search_blur_to_original_form_keeps_clicked_editor_and_draft() {
        let (mut host, surface, path) = fixture_source(include_str!(
            "../../tekes-ui-settings/tests/fixtures/layout.schema.toml"
        ));
        // Independent fixture supplies the real mounted form; install the
        // Exclusive header declaration used by this interaction.
        {
            let mut state = surface.borrow_mut();
            state.sidebar =
                tekes_ui_settings::SettingsSidebar::new_with_back(&state.schema).unwrap();
            let selected = state.selected.clone();
            state.sidebar.select(&selected);
        }
        let draw_header = |host: &mut Host| {
            pinned_header_tests::render_header(
                &mut surface.borrow_mut().sidebar,
                &mut host.text,
                host.panels[0].bounds,
            );
        };
        draw_header(&mut host);
        let field = surface
            .borrow()
            .form
            .host
            .get(ComponentId(100))
            .unwrap()
            .frame()
            .center();
        host.settings_form_input(&ContentEvent::PointerDown(field))
            .unwrap();
        host.settings_form_input(&ContentEvent::PointerUp(field))
            .unwrap();
        host.settings_form_input(&ContentEvent::TextCommand("selectAll:".into()))
            .unwrap();
        host.settings_form_input(&ContentEvent::Commit {
            text: "Retained draft".into(),
            replacement: None,
        })
        .unwrap();
        let chip = surface
            .borrow()
            .sidebar
            .host
            .get(ComponentId(10))
            .unwrap()
            .frame()
            .center();
        host.settings_sidebar_input(&ContentEvent::PointerDown(chip))
            .unwrap();
        host.settings_sidebar_input(&ContentEvent::PointerUp(chip))
            .unwrap();
        draw_header(&mut host);
        host.settings_sidebar_input(&ContentEvent::Commit {
            text: "行距倍数".into(),
            replacement: None,
        })
        .unwrap();
        surface.borrow_mut().sidebar.sync_search().unwrap();
        draw_header(&mut host);
        host.settings_form_input(&ContentEvent::PointerDown(field))
            .unwrap();
        surface.borrow_mut().sidebar.sync_search().unwrap();
        draw_header(&mut host);
        host.settings_form_input(&ContentEvent::PointerUp(field))
            .unwrap();
        let state = surface.borrow();
        assert_eq!(state.selected, "workspace/general");
        assert_eq!(state.form.host.focused(), Some(ComponentId(100)));
        assert_eq!(
            state.form.host.focused_text().unwrap().input.text,
            "Retained draft"
        );
        assert_eq!(state.sidebar.host.focused(), None);
        assert!(state.sidebar.host.get(ComponentId(6)).is_none());
        assert!(state.sidebar.host.get(ComponentId(4)).is_some());
        assert!(state.sidebar.host.get(ComponentId(10)).is_some());
        drop(state);
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn original_settings_keyboard_route_traverses_scopes_and_space_selects_form_radio() {
        let (mut host, surface, path) = fixture();
        // This same key entry point is called by the actual native toolbar
        // route; no test injects Tab directly into the form helper.
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert_eq!(
            host.actual_toolbar_owner(),
            Some(ToolbarTarget::Visibility(0))
        );
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert!(surface.borrow().sidebar.host.focused().is_some());
        assert!(surface.borrow().form.host.focused().is_none());
        assert!(host.actual_toolbar_owner().is_none());
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(100)));
        assert!(surface.borrow().sidebar.host.focused().is_none());
        assert!(host.actual_toolbar_owner().is_none());
        assert!(host.toolbar_focus_lease.is_none());
        let mut modified = key(48, true);
        modified.modifiers.command = true;
        assert!(!host.toolbar_key_input(modified).unwrap());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(100)));
        for down in [true, false] {
            assert!(!host.toolbar_key_input(key(49, down)).unwrap());
            host.settings_form_input(&ContentEvent::KeyInput(key(49, down)))
                .unwrap();
        }
        assert!(!surface.borrow().form.has_open_menu());
        assert!(host.leading_visible);
        render_original_panels(&mut host);
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        assert!(host.actual_toolbar_owner().is_none());
        assert!(surface.borrow().sidebar.host.focused().is_none());
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn original_settings_keyboard_route_reverses_scopes_and_wraps_the_window() {
        let (mut host, surface, path) = fixture();
        for _ in 0..3 {
            assert!(host.toolbar_key_input(key(48, true)).unwrap());
            // Native input requests a redraw; the next native key is routed
            // against geometry from the resulting original panel render.
            render_original_panels(&mut host);
        }
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(100)));
        let mut reverse = key(48, true);
        reverse.key = Some(ControlKey::Tab { reverse: true });
        reverse.modifiers.shift = true;
        assert!(host.toolbar_key_input(reverse.clone()).unwrap());
        render_original_panels(&mut host);
        assert!(surface.borrow().sidebar.host.focused().is_some());
        assert!(surface.borrow().form.host.focused().is_none());
        assert!(host.toolbar_key_input(reverse.clone()).unwrap());
        render_original_panels(&mut host);
        assert_eq!(
            host.actual_toolbar_owner(),
            Some(ToolbarTarget::Visibility(0))
        );
        assert!(host.toolbar_key_input(reverse).unwrap());
        render_original_panels(&mut host);
        assert!(surface.borrow().form.host.focused().is_some());
        assert!(host.actual_toolbar_owner().is_none());
        assert!(host.leading_visible);
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn native_text_key_tab_leaves_last_valid_settings_number() {
        use tekes_ui_platform_macos::{TextKeyContext, TextKeyDisposition, TextKeyEvent};

        let (mut host, surface, path) = fixture();
        for _ in 0..4 {
            host.toolbar_key_input(key(48, true)).unwrap();
            render_original_panels(&mut host);
        }
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        let context = TextKeyContext {
            id: 1,
            key: KeyEvent {
                kind: KeyEventKind::Down,
                key_code: 48,
                modifiers: Modifiers::NONE,
                is_repeat: false,
                characters: "\t".into(),
                characters_ignoring_modifiers: "\t".into(),
            },
            composing_at_start: false,
        };
        // Mirror settings_event's native adapter and routing boundaries. The
        // command arrives inside the text-key span, before completion commits
        // the original editor's deferred Tab.
        let started = window_toolbar_mount::native_toolbar_event(&Event::TextKey(
            TextKeyEvent::Started(context.clone()),
        ))
        .unwrap();
        host.settings_form_input(&started).unwrap();
        host.settings_scope_tab(false, &ContentEvent::TextCommand("insertTab:".into()))
            .unwrap();
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        assert!(host.actual_toolbar_owner().is_none());
        let finished =
            window_toolbar_mount::native_toolbar_event(&Event::TextKey(TextKeyEvent::Finished {
                context,
                input_context_handled: Some(true),
                disposition: TextKeyDisposition::CommandDispatched,
                text_callbacks: 0,
                command_callbacks: 1,
            }))
            .unwrap();
        host.settings_scope_tab(false, &finished).unwrap();
        assert_eq!(
            host.actual_toolbar_owner(),
            Some(ToolbarTarget::Visibility(0)),
            "completed Tab at the last valid field must advance to the next window scope"
        );
        assert!(surface.borrow().form.host.focused().is_none());
        host.settings_scope_tab(false, &finished).unwrap();
        assert_eq!(
            host.actual_toolbar_owner(),
            Some(ToolbarTarget::Visibility(0)),
            "duplicate completion must not advance another scope"
        );
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn native_text_key_tab_retains_invalid_composing_and_input_consumed_number() {
        for case in ["invalid", "composing", "input-consumed"] {
            let (mut host, surface, path) = fixture();
            for _ in 0..4 {
                host.toolbar_key_input(key(48, true)).unwrap();
                render_original_panels(&mut host);
            }
            assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
            if case == "invalid" {
                host.settings_form_input(&ContentEvent::TextCommand("selectAll:".into()))
                    .unwrap();
                host.settings_form_input(&ContentEvent::Commit {
                    text: "999".into(),
                    replacement: None,
                })
                .unwrap();
            } else if case == "composing" {
                host.settings_form_input(&ContentEvent::Preedit {
                    text: "1".into(),
                    selection: 0..1,
                    clauses: std::iter::once(0..1).collect(),
                    replacement: None,
                })
                .unwrap();
            }
            let (text, marked) = {
                let state = surface.borrow();
                let input = &state.form.host.focused_text().unwrap().input;
                (input.text.clone(), input.marked.clone())
            };
            host.settings_form_input(&ContentEvent::TextKeyStart {
                id: 1,
                key: key(48, true),
                composing: case == "composing",
            })
            .unwrap();
            // InputContextConsumed means the input context supplied no callbacks.
            if case != "input-consumed" {
                host.settings_scope_tab(false, &ContentEvent::TextCommand("insertTab:".into()))
                    .unwrap();
            }
            let finished = ContentEvent::TextKeyEnd {
                id: 1,
                input_context_consumed: case == "input-consumed",
            };
            for _ in 0..2 {
                host.settings_scope_tab(false, &finished).unwrap();
                let state = surface.borrow();
                assert_eq!(state.form.host.focused(), Some(ComponentId(101)), "{case}");
                let input = &state.form.host.focused_text().unwrap().input;
                assert_eq!(input.text, text, "{case}");
                assert_eq!(input.marked, marked, "{case}");
                assert!(state.sidebar.host.focused().is_none(), "{case}");
                assert!(host.actual_toolbar_owner().is_none(), "{case}");
            }
            drop(host);
            drop(surface);
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn native_text_key_backtab_moves_number_to_radio_once() {
        let (mut host, surface, path) = fixture();
        for _ in 0..4 {
            host.toolbar_key_input(key(48, true)).unwrap();
            render_original_panels(&mut host);
        }
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        let mut reverse = key(48, true);
        reverse.key = Some(ControlKey::Tab { reverse: true });
        reverse.modifiers.shift = true;
        host.settings_form_input(&ContentEvent::TextKeyStart {
            id: 1,
            key: reverse,
            composing: false,
        })
        .unwrap();
        host.settings_scope_tab(true, &ContentEvent::TextCommand("insertBacktab:".into()))
            .unwrap();
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        let finished = ContentEvent::TextKeyEnd {
            id: 1,
            input_context_consumed: false,
        };
        for _ in 0..2 {
            host.settings_scope_tab(true, &finished).unwrap();
            assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(100)));
            assert!(surface.borrow().sidebar.host.focused().is_none());
            assert!(host.actual_toolbar_owner().is_none());
        }
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn original_settings_scope_tab_keeps_invalid_number_and_composition_in_their_owner() {
        let (mut host, surface, path) = fixture();
        for _ in 0..4 {
            assert!(host.toolbar_key_input(key(48, true)).unwrap());
        }
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        host.settings_form_input(&ContentEvent::TextCommand("selectAll:".into()))
            .unwrap();
        host.settings_form_input(&ContentEvent::Commit {
            text: "999".into(),
            replacement: None,
        })
        .unwrap();
        assert!(surface.borrow().form.has_number_draft());
        let invalid = surface
            .borrow()
            .form
            .host
            .focused_text()
            .unwrap()
            .input
            .text
            .clone();
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        assert!(host.actual_toolbar_owner().is_none());
        assert_eq!(
            surface
                .borrow()
                .form
                .host
                .focused_text()
                .unwrap()
                .input
                .text,
            invalid
        );
        // Correct/discard through the original editor, not a focus reset.
        host.settings_form_input(&ContentEvent::Key {
            key: ControlKey::Escape,
            down: true,
        })
        .unwrap();
        host.settings_form_input(&ContentEvent::Preedit {
            text: "1".into(),
            selection: 0..1,
            clauses: std::iter::once(0..1).collect(),
            replacement: None,
        })
        .unwrap();
        assert!(surface.borrow().form.host.is_composing());
        let marked = surface
            .borrow()
            .form
            .host
            .focused_text()
            .unwrap()
            .input
            .marked
            .clone();
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        assert_eq!(
            surface
                .borrow()
                .form
                .host
                .focused_text()
                .unwrap()
                .input
                .marked,
            marked
        );
        assert!(surface.borrow().sidebar.host.focused().is_none());
        assert!(host.actual_toolbar_owner().is_none());
        assert!(host.leading_visible);
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn original_settings_form_tab_retires_toolbar_owner_before_space_selects_theme_radio() {
        let (mut host, surface, path) = fixture();
        host.toolbar_key_input(key(48, true)).unwrap();
        assert_eq!(
            host.actual_toolbar_owner(),
            Some(ToolbarTarget::Visibility(0))
        );
        assert!(host.leading_visible);
        // This is the original mounted form-input boundary when a Tab reaches
        // form navigation; the outer toolbar consumed decision has its own test.
        host.settings_form_input(&ContentEvent::KeyInput(key(48, true)))
            .unwrap();
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(100)));
        assert!(
            host.panels.iter().all(|panel| panel
                .mounted_toolbar
                .host
                .content_host()
                .focused()
                .is_none()),
            "form Tab must retire every toolbar keyboard owner"
        );
        assert!(host.toolbar_focus.is_none());
        assert!(host.toolbar_focus_lease.is_none());
        for down in [true, false] {
            let consumed = host.toolbar_key_input(key(49, down)).unwrap();
            assert!(
                !consumed,
                "an unfocused toolbar must let Space reach the form"
            );
            host.settings_form_input(&ContentEvent::KeyInput(key(49, down)))
                .unwrap();
        }
        assert_eq!(
            surface
                .borrow()
                .form
                .host
                .get(ComponentId(100))
                .unwrap()
                .component::<RadioGroupContent>()
                .unwrap()
                .model()
                .selected,
            Some(0),
            "Space stays with the original selected Theme radio option"
        );
        assert!(surface.borrow().form.host.input_overlay_owner().is_none());
        assert!(
            host.leading_visible,
            "Space must not invoke the old toolbar visibility command"
        );
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn actual_general_radio_raw_pointer_selection_updates_draft_without_remount() {
        let (mut host, surface, path) = fixture();
        render_original_panels(&mut host);
        let id = ComponentId(100);
        let (mount, point, selected, radio_layout, radio_frame, row_frame) = {
            let state = surface.borrow();
            let radio = state
                .form
                .host
                .get(id)
                .unwrap()
                .component::<RadioGroupContent>()
                .unwrap();
            (
                state.form.host.semantic_mount(id),
                radio.view().layout().rows[2].frame().center(),
                radio.model().selected,
                radio.view().layout().clone(),
                state.form.host.get(id).unwrap().frame(),
                state
                    .form
                    .host
                    .get(ComponentId(1_u64 << 32))
                    .unwrap()
                    .frame(),
            )
        };
        assert_eq!(radio_layout.rows.len(), 3);
        assert!(
            radio_layout
                .rows
                .iter()
                .all(|row| row.frame().center().y == radio_layout.rows[0].frame().center().y)
        );
        assert!(
            radio_layout
                .rows
                .windows(2)
                .all(|rows| { rows[0].frame().min_x() < rows[1].frame().min_x() })
        );
        assert_eq!(
            radio_frame.max_x(),
            row_frame.max_x()
                - tekes_ui_tokens::proposal::metric("spacing", "space-grouped-form-row-inline")
                    .unwrap()
        );
        assert_eq!(selected, Some(0));
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            host.settings_form_input(&ContentEvent::Pointer(PointerEvent {
                position: point,
                phase,
                button: Some(PointerButton::Secondary),
                modifiers: PointerModifiers::default(),
                click_count: 1,
            }))
            .unwrap();
        }
        assert_eq!(
            surface
                .borrow()
                .draft
                .display(&surface.borrow().view_schema.settings[0]),
            "system",
            "secondary clicks must not select a radio option"
        );
        // Collection mutably visits the original field nodes. Republish layout
        // as the native render loop does before the next independent gesture.
        render_original_panels(&mut host);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            host.settings_form_input(&ContentEvent::Pointer(PointerEvent {
                position: point,
                phase,
                button: Some(PointerButton::Primary),
                modifiers: PointerModifiers::default(),
                click_count: 1,
            }))
            .unwrap();
            if phase == PointerPhase::Down {
                assert!(
                    !surface.borrow().form.host.event_bubbled(),
                    "native Primary PointerDown on an exposed radio row is handled"
                );
            }
        }
        {
            let state = surface.borrow();
            assert_eq!(state.draft.display(&state.view_schema.settings[0]), "dark");
        }
        assert_eq!(
            surface
                .borrow()
                .form
                .host
                .get(id)
                .unwrap()
                .component::<RadioGroupContent>()
                .unwrap()
                .model()
                .selected,
            Some(2)
        );
        assert!(
            surface.borrow().form.host.input_overlay_owner().is_none(),
            "radio choice never creates a popup"
        );
        {
            let mut state = surface.borrow_mut();
            let schema = state.view_schema.clone();
            state.draft.edit(&schema.settings[0], "light".into());
            let draft = state.draft.clone();
            state.form.replace_values(&schema, &draft);
        }
        assert_eq!(surface.borrow().form.host.semantic_mount(id), mount);
        assert_eq!(
            surface
                .borrow()
                .form
                .host
                .get(id)
                .unwrap()
                .component::<RadioGroupContent>()
                .unwrap()
                .model()
                .selected,
            Some(1)
        );
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn default_enum_menu_tab_stays_in_the_form_scope() {
        let source = include_str!("../../tekes-ui-gallery/src/gallery/apphost.settings.toml")
            .replace("presentation = \"radio\"\n", "");
        let (mut host, surface, path) = fixture_source(&source);
        for _ in 0..3 {
            assert!(host.toolbar_key_input(key(48, true)).unwrap());
            render_original_panels(&mut host);
        }
        for down in [true, false] {
            assert!(!host.toolbar_key_input(key(49, down)).unwrap());
            host.settings_form_input(&ContentEvent::KeyInput(key(49, down)))
                .unwrap();
        }
        assert!(surface.borrow().form.has_open_menu());
        render_original_panels(&mut host);
        assert!(host.toolbar_key_input(key(48, true)).unwrap());
        // The original Menu closes and advances once within the form. The
        // same Tab must not also escape to sidebar/toolbar scope navigation.
        assert!(!surface.borrow().form.has_open_menu());
        assert_eq!(surface.borrow().form.host.focused(), Some(ComponentId(101)));
        assert!(host.actual_toolbar_owner().is_none());
        assert!(surface.borrow().sidebar.host.focused().is_none());
        drop(host);
        drop(surface);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
